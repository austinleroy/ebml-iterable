use std::io::Read;
use std::collections::{HashSet, VecDeque};

use crate::tag_iterator_util::EBMLSize::{Known, Unknown};
use crate::tag_iterator_util::{DEFAULT_BUFFER_LEN, EBMLSize, ProcessingTag};

use super::tools;
use super::specs::{EbmlSpecification, EbmlTag, Master, TagDataType};
use super::errors::tag_iterator::TagIteratorError;
use super::errors::tool::ToolError;

///
/// Provides an iterator over EBML files (read from a source implementing the [`std::io::Read`] trait). Can be configured to read specific "Master" tags as complete objects rather than just emitting when they start and end.
///
/// This is a generic struct that requires a specification implementing [`EbmlSpecification`] and [`EbmlTag`]. No specifications are included in this crate - you will need to either use another crate providing a spec (such as the Matroska spec implemented in the [webm-iterable](https://crates.io/crates/webm_iterable) or write your own spec if you want to iterate over a custom EBML file. The iterator outputs `TSpec` variants representing the type of tag (defined by the specification) and the accompanying tag data. "Master" tags (defined by the specification) usually will be read as `Start` and `End` variants, but the iterator can be configured to buffer Master tags into a `Full` variant using the `tags_to_buffer` parameter.
///
/// Note: The [`Self::with_capacity()`] method can be used to construct a `TagIterator` with a specified default buffer size.  This is only useful as a microoptimization to memory management if you know the maximum tag size of the file you're reading.
///
/// ## Example
///
/// ```no_run
/// use std::fs::File;
/// use ebml_iterable::TagIterator;
/// #
/// # use ebml_iterable::specs::{EbmlSpecification, TagDataType};
/// # use ebml_iterable_specification::empty_spec::EmptySpec;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let file = File::open("my_ebml_file.ebml")?;
/// let mut my_iterator: TagIterator<_, EmptySpec> = TagIterator::new(file, &[]);
/// for tag in my_iterator {
///   println!("{:?}", tag?);
/// }
/// # Ok(())
/// # }
/// ```
///
/// ## Errors
///
/// The `Item` type for the associated [`Iterator`] implementation is a [`Result<TSpec, TagIteratorError>`], meaning each `next()` call has the potential to fail.  This is because the source data is not parsed all at once - it is incrementally parsed as the iterator progresses.  If the iterator runs into an error (such as corrupted data or an unexpected end-of-file), it needs to be propagated to the logic trying to read the tags.  The different possible error states are enumerated in [`TagIteratorError`].
///
/// ## Panics
///
/// The iterator can panic if `<TSpec>` is an internally inconsistent specification (i.e. it claims that a specific tag id has a specific data type but fails to produce a tag variant using data of that type).  This won't happen if the specification being used was created using the [`#[ebml_specification]`](https://docs.rs/ebml-iterable-specification-derive/latest/ebml_iterable_specification_derive/attr.ebml_specification.html) attribute macro.
///

pub struct TagIterator<R: Read, TSpec>
    where
    TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone
{
    source: R,
    tag_ids_to_buffer: HashSet<u64>,

    buffer: Box<[u8]>,
    buffer_offset: Option<usize>,
    buffered_byte_length: usize,
    internal_buffer_position: usize,
    tag_stack: Vec<ProcessingTag<TSpec>>,
    emission_queue: VecDeque<Result<(TSpec, usize), TagIteratorError>>,
    last_emitted_tag_offset: usize,
}

impl<R: Read, TSpec> TagIterator<R, TSpec>
    where
    TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone
{

    ///
    /// Returns a new [`TagIterator<TSpec>`] instance.
    ///
    /// The `source` parameter must implement [`std::io::Read`].  The second argument, `tags_to_buffer`, specifies which "Master" tags should be read as [`Master::Full`]s rather than as [`Master::Start`] and [`Master::End`]s.  Refer to the documentation on [`TagIterator`] for more explanation of how to use the returned instance.
    ///
    pub fn new(source: R, tags_to_buffer: &[TSpec]) -> Self {
        TagIterator::with_capacity(source, tags_to_buffer, DEFAULT_BUFFER_LEN)
    }

    ///
    /// Returns a new [`TagIterator<TSpec>`] instance with the specified internal buffer capacity.
    ///
    /// This initializes the [`TagIterator`] with a specific byte capacity.  The iterator will still reallocate if necessary. (Reallocation occurs if the iterator comes across a tag that should be output as a [`Master::Full`] and its size in bytes is greater than the iterator's current buffer capacity.)
    ///
    pub fn with_capacity(source: R, tags_to_buffer: &[TSpec], capacity: usize) -> Self {
        let buffer = vec![0;capacity];

        TagIterator {
            source,
            tag_ids_to_buffer: tags_to_buffer.iter().map(|tag| tag.get_id()).collect(),
            buffer: buffer.into_boxed_slice(),
            buffered_byte_length: 0,
            buffer_offset: None,
            internal_buffer_position: 0,
            tag_stack: Vec::new(),
            emission_queue: VecDeque::new(),
            last_emitted_tag_offset: 0,
        }
    }

    ///
    /// Consumes self and returns the underlying read stream.
    /// 
    /// Note that any leftover tags in the internal emission queue are lost. Therefore, constructing a new TagIterator using the returned stream may lead to data loss.
    /// 
    pub fn into_inner(self) -> R {
        self.source
    }

    ///
    /// Gets a mutable reference to the underlying read stream.
    /// 
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.source
    }

    ///
    /// Gets a reference to the underlying read stream.
    /// 
    pub fn get_ref(&self) -> &R {
        &self.source
    }

    ///
    /// Returns the byte offset of the last emitted tag.
    /// 
    /// This function returns a byte index specifying the start of the last emitted tag in the context of the [`TagIterator`]'s source read stream.  This value is *not guaranteed to always increase as the file is read*.  Whenever the iterator emits a [`Master::End`] variant, [`Self::last_emitted_tag_offset()`] will reflect the start index of the "Master" tag, which will be before previous values that were obtainable when any children of the master were emitted.
    /// 
    pub fn last_emitted_tag_offset(&self) -> usize {
        self.last_emitted_tag_offset
    }

    fn current_offset(&self) -> usize {
        self.buffer_offset.unwrap_or(0) + self.internal_buffer_position
    }

    fn private_read(&mut self, internal_buffer_start: usize) -> Result<bool, TagIteratorError> {
        let bytes_read = self.source.read(&mut self.buffer[internal_buffer_start..]).map_err(|source| TagIteratorError::ReadError { source })?;
        if bytes_read == 0 {
            Ok(false)
        } else {
            self.buffered_byte_length += bytes_read;
            Ok(true)
        }
    }

    fn ensure_capacity(&mut self, required_capacity: usize) {
        if required_capacity > self.buffer.len() {
            let mut new_buffer = Vec::from(&self.buffer[..]);
            new_buffer.resize(required_capacity, 0);
            self.buffer = new_buffer.into_boxed_slice();
        }
    }

    fn ensure_data_read(&mut self, length: usize) -> Result<bool, TagIteratorError> {
        if self.internal_buffer_position + length <= self.buffered_byte_length {
            return Ok(true)
        }

        if self.buffer_offset.is_none() {
            if !self.private_read(0)? {
                return Ok(false);
            }
            self.buffer_offset = Some(0);
            self.internal_buffer_position = 0;
        } else {
            while self.internal_buffer_position + length > self.buffered_byte_length {
                self.buffer.copy_within(self.internal_buffer_position..self.buffered_byte_length, 0);
                self.buffered_byte_length -= self.internal_buffer_position;
                self.buffer_offset = Some(self.current_offset());
                self.internal_buffer_position = 0;
                if !self.private_read(self.buffered_byte_length)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn read_tag_id(&mut self) -> Result<Option<u64>, TagIteratorError> {
        self.ensure_data_read(8)?;
        match tools::read_vint(&self.buffer[self.internal_buffer_position..]).map_err(|e| TagIteratorError::CorruptedFileData(e.to_string()))? {
            Some((value, length)) => {
                if length > self.buffered_byte_length {
                    Ok(None)
                } else {
                    self.internal_buffer_position += length;
                    Ok(Some(value + (1 << (7 * length))))
                }
            },
            None => Ok(None)
        }
    }

    fn read_tag_size(&mut self) -> Result<Option<EBMLSize>, TagIteratorError> {
        self.ensure_data_read(8)?;
        match tools::read_vint(&self.buffer[self.internal_buffer_position..]).map_err(|e| TagIteratorError::CorruptedFileData(e.to_string()))? {
            Some((value, length)) => {
                if length > self.buffered_byte_length {
                    Ok(None)
                } else {
                    self.internal_buffer_position += length;
                    Ok(Some(EBMLSize::new(value, length)))
                }
            },
            None => Ok(None)
        }
    }

    fn read_tag_data(&mut self, size: usize) -> Result<&[u8], TagIteratorError> {
        self.ensure_capacity(size);
        if !self.ensure_data_read(size)? {
            return Err(TagIteratorError::UnexpectedEOF { tag_start: 0, tag_id: None, tag_size: None, partial_data: Some(self.buffer[self.internal_buffer_position..].to_vec()) });
        }

        self.internal_buffer_position += size;
        Ok(&self.buffer[(self.internal_buffer_position-size)..self.internal_buffer_position])
    }

    fn read_tag(&mut self) -> Result<ProcessingTag<TSpec>, TagIteratorError> {
        let tag_start = self.current_offset();

        let tag_id = self.read_tag_id().and_then(|res| {
            if let Some(res) = res {
                Ok(res)
            } else {
                Err(TagIteratorError::UnexpectedEOF { tag_start, tag_id: None, tag_size: None, partial_data: None })
            }
        })?;
        let size: EBMLSize = self.read_tag_size().and_then(|res| {
            if let Some(res) = res {
                Ok(res)
            } else {
                Err(TagIteratorError::UnexpectedEOF { tag_start, tag_id: Some(tag_id), tag_size: None, partial_data: None })
            }
        })?;

        let spec_tag_type = <TSpec>::get_tag_data_type(tag_id);
        let data_start = self.current_offset();

        let raw_data = if matches!(spec_tag_type, TagDataType::Master) {
            &[]
        } else if let Known(size) = size {
            self.read_tag_data(size).map_err(|err| {
                if let TagIteratorError::UnexpectedEOF{ tag_start: _, tag_id: _, tag_size: _, partial_data} = err {
                    TagIteratorError::UnexpectedEOF { tag_start, tag_id: Some(tag_id), tag_size: Some(size), partial_data }
                } else {
                    err
                }
            })?
        } else {
            return Err(TagIteratorError::CorruptedFileData("Unknown size for primitive tag not allowed".into()));
        };

        let tag = match spec_tag_type {
            TagDataType::Master => {
                TSpec::get_master_tag(tag_id, Master::Start).unwrap_or_else(|| panic!("Bad specification implementation: Tag id 0x{:x?} type was master, but could not get tag!", tag_id))
            },
            TagDataType::UnsignedInt => {
                let val = tools::arr_to_u64(raw_data).map_err(|e| TagIteratorError::CorruptedTagData{ tag_id, problem: e })?;
                TSpec::get_unsigned_int_tag(tag_id, val).unwrap_or_else(|| panic!("Bad specification implementation: Tag id 0x{:x?} type was unsigned int, but could not get tag!", tag_id))
            },
            TagDataType::Integer => {
                let val = tools::arr_to_i64(raw_data).map_err(|e| TagIteratorError::CorruptedTagData{ tag_id, problem: e })?;
                TSpec::get_signed_int_tag(tag_id, val).unwrap_or_else(|| panic!("Bad specification implementation: Tag id 0x{:x?} type was integer, but could not get tag!", tag_id))
            },
            TagDataType::Utf8 => {
                let val = String::from_utf8(raw_data.to_vec()).map_err(|e| TagIteratorError::CorruptedTagData{ tag_id, problem: ToolError::FromUtf8Error(raw_data.to_vec(), e) })?;
                TSpec::get_utf8_tag(tag_id, val).unwrap_or_else(|| panic!("Bad specification implementation: Tag id 0x{:x?} type was utf8, but could not get tag!", tag_id))
            },
            TagDataType::Binary => {
                TSpec::get_binary_tag(tag_id, raw_data).unwrap_or_else(|| TSpec::get_raw_tag(tag_id, raw_data))
            },
            TagDataType::Float => {
                let val = tools::arr_to_f64(raw_data).map_err(|e| TagIteratorError::CorruptedTagData{ tag_id, problem: e })?;
                TSpec::get_float_tag(tag_id, val).unwrap_or_else(|| panic!("Bad specification implementation: Tag id 0x{:x?} type was float, but could not get tag!", tag_id))
            },
        };

        Ok(ProcessingTag { tag, size, tag_start, data_start })
    }

    fn read_tag_checked(&mut self) -> Option<Result<ProcessingTag<TSpec>, TagIteratorError>> {
        if self.internal_buffer_position == self.buffered_byte_length {
            //If we've already consumed the entire internal buffer
            //ensure there is nothing else in the data source before returning `None`
            let read_result = self.ensure_data_read(1);
            match read_result {
                Err(err) => return Some(Err(err)),
                Ok(data_remaining) => {
                    if !data_remaining {
                        return None;
                    }
                 }
            }
        }

        if self.internal_buffer_position > self.buffered_byte_length {
            panic!("read position exceeded buffer length");
        }

        Some(self.read_tag())
    }

    fn read_next(&mut self) {
        //If we have reached the known end of any open master tags, queue that tag and all children to emit ends
        let ended_tag_index = self.tag_stack.iter().position(|tag| matches!(tag.size, Known(size) if self.current_offset() >= tag.data_start + size));
        if let Some(index) = ended_tag_index {
            self.emission_queue.extend(self.tag_stack.drain(index..).map(|t| Ok((t.tag, t.tag_start))).rev());
        }

        if let Some(next_read) = self.read_tag_checked() {
            if let Ok(next_tag) = &next_read {
                while matches!(self.tag_stack.last(), Some(open_tag) if open_tag.size == Unknown) {
                    // Unknown sized tags can be ended if we reach an element that is:
                    //  - A parent of the tag
                    //  - A direct sibling of the tag
                    //  - A Root element
                    let open_tag = self.tag_stack.last().unwrap();
                    let previous_tag_ended =
                        open_tag.is_parent(next_tag.tag.get_id()) || // parent
                        open_tag.is_sibling(&next_tag.tag) || // sibling
                        ( // Root element
                            std::mem::discriminant(&next_tag.tag) != std::mem::discriminant(&TSpec::get_raw_tag(next_tag.tag.get_id(), &[])) && 
                            matches!(next_tag.tag.get_parent_id(), None)
                        );
        
                    if previous_tag_ended {
                        let t = self.tag_stack.pop().unwrap();
                        self.emission_queue.push_back(Ok((t.tag, t.tag_start)));
                    } else {
                        break;
                    }
                }

                if let Some(Master::Start) = next_tag.tag.as_master() {
                    let tag_id = next_tag.tag.get_id();

                    self.tag_stack.push(ProcessingTag {
                        tag: TSpec::get_master_tag(tag_id, Master::End).unwrap(),
                        size: next_tag.size,
                        tag_start: next_tag.tag_start,
                        data_start: next_tag.data_start
                    });

                    if self.tag_ids_to_buffer.contains(&tag_id) {
                        self.buffer_master(tag_id);
                        return;
                    }
                }
            }

            self.emission_queue.push_back(next_read.map(|r| (r.tag, r.tag_start)));
        } else {
            while let Some(tag) = self.tag_stack.pop() {
                self.emission_queue.push_back(Ok((tag.tag, tag.tag_start)));
            }
        }
    }

    fn buffer_master(&mut self, tag_id: u64) {
        let tag_start = self.current_offset();
        let pre_queue_len = self.emission_queue.len();

        let mut position = pre_queue_len;
        'endTagSearch: loop {
            if position >= self.emission_queue.len() {
                self.read_next();
    
                if position >= self.emission_queue.len() {
                    self.emission_queue.push_back(Err(TagIteratorError::UnexpectedEOF{ tag_start, tag_id: Some(tag_id), tag_size: None, partial_data: None }));
                    return;
                }
            }

            while position < self.emission_queue.len() {
                if let Some(r) = self.emission_queue.get(position) {
                    match r {
                        Err(_) => break 'endTagSearch,
                        Ok(t) => {
                            if t.0.get_id() == tag_id && matches!(t.0.as_master(), Some(Master::End)) {
                                break 'endTagSearch;
                            }
                        }
                    }
                }
                position += 1;
            }
        }

        let mut children = self.emission_queue.split_off(pre_queue_len);
        let split_to = position - pre_queue_len;
        if children.get(split_to).unwrap().is_ok() {
            let remaining = children.split_off(split_to).into_iter().skip(1);
            let full_tag = Self::roll_up_children(tag_id, children.into_iter().map(|c| c.unwrap().0).collect());
            self.emission_queue.push_back(Ok((full_tag, tag_start)));
            self.emission_queue.extend(remaining);
        } else {
            self.emission_queue.extend(children.drain(split_to..).take(1));
        }
    }

    fn roll_up_children(tag_id: u64, children: Vec<TSpec>) -> TSpec {
        let mut rolled_children = Vec::new();

        let mut iter = children.into_iter();
        while let Some(child) = iter.next() {
            if let Some(Master::Start) = child.as_master() {
                let child_id = child.get_id();
                let subchildren = iter.by_ref().take_while(|c| !matches!(c.as_master(), Some(Master::End)) || c.get_id() != child_id).collect();
                rolled_children.push(Self::roll_up_children(child_id, subchildren));
            } else {
                rolled_children.push(child);
            }
        }

        TSpec::get_master_tag(tag_id, Master::Full(rolled_children)).unwrap_or_else(|| panic!("Bad specification implementation: Tag id 0x{:x?} type was master, but could not get tag!", tag_id))
    }
}

impl<R: Read, TSpec> Iterator for TagIterator<R, TSpec>
    where TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone
{
    type Item = Result<TSpec, TagIteratorError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.emission_queue.is_empty() {
            self.read_next();
        }
        let next_item = self.emission_queue.pop_front();
        if let Some(Ok(ref tuple)) = next_item {
            self.last_emitted_tag_offset = tuple.1;
        }
        next_item.map(|r| r.map(|t| t.0))
    }
}