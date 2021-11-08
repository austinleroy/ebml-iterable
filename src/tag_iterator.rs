use std::io::{Read, Cursor};
use std::convert::TryInto;
use std::collections::HashSet;

use super::tools;
use super::specs::{EbmlSpecification, EbmlTag, TagDataType, Master};
use super::errors::tag_iterator::TagIteratorError;
use super::errors::tool::ToolError;

struct ProcessingTag<TSpec> 
    where TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone
{
    end_tag: TSpec,
    size: usize,
    start: usize,
}

const DEFAULT_BUFFER_LEN: usize = 1024 * 64;

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
    buffer_all: bool,

    buffer: Box<[u8]>,
    buffer_offset: Option<usize>,
    buffered_byte_length: usize,
    internal_buffer_position: usize,
    tag_stack: Vec<ProcessingTag<TSpec>>,
}

impl<'a, R: Read, TSpec> TagIterator<R, TSpec>
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
            buffer_all: false,
            buffer: buffer.into_boxed_slice(),
            buffered_byte_length: 0,
            buffer_offset: None,
            internal_buffer_position: 0,
            tag_stack: Vec::new(),
        }
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
                self.buffer_offset = Some(self.buffer_offset.unwrap() + self.internal_buffer_position);
                self.internal_buffer_position = 0;
                if !self.private_read(self.buffered_byte_length)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn read_tag_id(&mut self) -> Result<u64, TagIteratorError> {
        self.ensure_data_read(8)?;
        match tools::read_vint(&self.buffer[self.internal_buffer_position..]).map_err(|e| TagIteratorError::CorruptedFileData(e.to_string()))? {
            Some((value, length)) => {
                self.internal_buffer_position += length;
                Ok(value + (1 << (7 * length)))
            },
            None => Err(TagIteratorError::CorruptedFileData(String::from("Expected tag id, but reached end of source."))),
        }
    }

    fn read_tag_size(&mut self) -> Result<usize, TagIteratorError> {
        self.ensure_data_read(8)?;
        match tools::read_vint(&self.buffer[self.internal_buffer_position..]).map_err(|e| TagIteratorError::CorruptedFileData(e.to_string()))? {
            Some((value, length)) => {
                self.internal_buffer_position += length;
                Ok(value.try_into().expect("u64 couldn't convert into usize"))
            },
            None => Err(TagIteratorError::CorruptedFileData(String::from("Expected tag size, but reached end of source."))),
        }
    }

    fn read_tag_data(&mut self, size: usize) -> Result<&[u8], TagIteratorError> {
        self.ensure_capacity(size);        
        if !self.ensure_data_read(size)? {
            return Err(TagIteratorError::CorruptedFileData(String::from("reached end of file but expecting more data")));
        }

        self.internal_buffer_position += size;
        Ok(&self.buffer[(self.internal_buffer_position-size)..self.internal_buffer_position])
    }

    fn read_tag(&mut self) -> Result<TSpec, TagIteratorError> {
        let tag_id = self.read_tag_id()?;
        let size: usize = self.read_tag_size()?;

        let spec_tag_type = <TSpec>::get_tag_data_type(tag_id);

        let is_master = matches!(spec_tag_type, TagDataType::Master);
        if is_master && !self.buffer_all && !self.tag_ids_to_buffer.contains(&tag_id) {
            self.tag_stack.push(ProcessingTag {
                end_tag: TSpec::get_master_tag(tag_id, Master::End).unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was master, but could not get tag!", tag_id)),
                size,
                start: self.current_offset(),
            });

            Ok(TSpec::get_master_tag(tag_id, Master::Start).unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was master, but could not get tag!", tag_id)))
        } else {
            let raw_data = self.read_tag_data(size)?;
            let tag_data = match spec_tag_type {
                TagDataType::Master => {
                    let mut src = Cursor::new(raw_data);
                    let mut sub_iterator: TagIterator<_, TSpec> = TagIterator::new(&mut src, &[]);
                    sub_iterator.buffer_all = true;
                    let children: Result<Vec<TSpec>, TagIteratorError> = sub_iterator.collect();

                    TSpec::get_master_tag(tag_id, Master::Full(children?)).unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was master, but could not get tag!", tag_id))
                },
                TagDataType::UnsignedInt => {
                    let val = tools::arr_to_u64(raw_data).map_err(|e| TagIteratorError::CorruptedTagData{ tag_id, problem: e })?;
                    TSpec::get_unsigned_int_tag(tag_id, val).unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was unsigned int, but could not get tag!", tag_id))
                },
                TagDataType::Integer => {
                    let val = tools::arr_to_i64(raw_data).map_err(|e| TagIteratorError::CorruptedTagData{ tag_id, problem: e })?;
                    TSpec::get_signed_int_tag(tag_id, val).unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was integer, but could not get tag!", tag_id))
                },
                TagDataType::Utf8 => {
                    let val = String::from_utf8(raw_data.to_vec()).map_err(|e| TagIteratorError::CorruptedTagData{ tag_id, problem: ToolError::FromUtf8Error(raw_data.to_vec(), e) })?;
                    TSpec::get_utf8_tag(tag_id, val).unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was utf8, but could not get tag!", tag_id))
                },
                TagDataType::Binary => {
                    TSpec::get_binary_tag(tag_id, raw_data).unwrap_or_else(|| TSpec::get_raw_tag(tag_id, raw_data))
                },
                TagDataType::Float => {
                    let val = tools::arr_to_f64(raw_data).map_err(|e| TagIteratorError::CorruptedTagData{ tag_id, problem: e })?;
                    TSpec::get_float_tag(tag_id, val).unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was float, but could not get tag!", tag_id))
                },
            };

            Ok(tag_data)
        }
    }
}

impl<R: Read, TSpec> Iterator for TagIterator<R, TSpec>
    where TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone
{
    type Item = Result<TSpec, TagIteratorError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(tag) = self.tag_stack.last() {
            if self.current_offset() >= tag.start + tag.size {
                let tag = self.tag_stack.pop().unwrap();
                return Some(Ok(tag.end_tag));
            }
        }

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
}
