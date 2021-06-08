use std::io::{Read, Cursor};
use std::convert::TryInto;
use std::collections::HashSet;
use std::hash::Hash;

use super::tools;
use super::specs::{EbmlSpecification, TagDataType as SpecificationTagDataType};
use super::tags::{TagPosition, TagData};
use super::errors::tag_iterator::TagIteratorError; 
use super::errors::specs::SpecMismatchError;

struct ProcessingTag<TSpec> 
    where TSpec: EbmlSpecification<TSpec>
{
    spec_tag_type: TSpec,
    id: u64,
    size: usize,
    start: usize,
}

const DEFAULT_BUFFER_LEN: usize = 1024 * 64;

///
/// Provides an iterator over EBML files (read from a source implementing the [`std::io::Read`] trait). Can be configured to read specific "Master" tags as complete objects rather than just emitting when they start and end.
/// 
/// This is a generic struct that requires a specification implementing [`EbmlSpecification`]. No specifications are included in this crate - you will need to either use another crate providing a spec (such as the Matroska spec implemented in the [webm-iterable](https://crates.io/crates/webm_iterable) or write your own spec if you want to iterate over a custom EBML file. The iterator outputs `SpecTag<TSpec>` objects containing data on the type of tag (defined by the specification) and the tag data. The tag data is stored in a [`TagPosition`] enum member.  "Master" tags (defined by the specification) usually will be read as `StartTag` and `EndTag` variants, while all other tags will have their complete data within the `FullTag` variant.  The iterator can be configured to buffer Master tags into a `FullTag` variant using the `tags_to_buffer` parameter.
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
/// # #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
/// # enum FooSpec { Bar }
/// #
/// # impl EbmlSpecification<FooSpec> for FooSpec {
/// #   fn get_tag(id: u64) -> Option<(FooSpec, TagDataType)> { Some((FooSpec::Bar, TagDataType::UnsignedInt)) }
/// #   fn get_tag_id(item: &FooSpec) -> u64 { 0 }
/// # }
/// 
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let file = File::open("my_ebml_file.ebml")?;
/// let mut my_iterator: TagIterator<_, FooSpec> = TagIterator::new(file, &[]);
/// for tag in my_iterator {
///   println!("{:?}", tag?.tag);
/// }
/// # Ok(())
/// # }
/// ```
/// 

pub struct TagIterator<R: Read, TSpec> 
    where 
    TSpec: EbmlSpecification<TSpec> + Eq + Hash + Copy
{
    source: R,
    tags_to_buffer: HashSet<TSpec>,
    buffer_all: bool,

    buffer: Box<[u8]>,
    buffer_offset: Option<usize>,
    buffered_byte_length: usize,
    internal_buffer_position: usize,
    reached_eof: bool,
    tag_stack: Vec<ProcessingTag<TSpec>>,
}

impl<R: Read, TSpec> TagIterator<R, TSpec>
    where 
    TSpec: EbmlSpecification<TSpec> + Eq + Hash + Copy
{

    /// 
    /// Returns a new `TagIterator<TSpec>` instance.
    ///
    /// The `source` parameter must implement [`std::io::Read`].  The second argument, `tags_to_buffer`, specifies which "Master" tags should be read as [`TagPosition::FullTag`]s rather than as [`TagPosition::StartTag`] and [`TagPosition::EndTag`]s.  Refer to the documentation on [`TagIterator`] for more explanation of how to use the returned instance.
    ///
    pub fn new(source: R, tags_to_buffer: &[TSpec]) -> Self {
        TagIterator::with_capacity(source, tags_to_buffer, DEFAULT_BUFFER_LEN)
    }
    
    ///
    /// Returns a new `TagIterator<TSpec>` instance with the specified internal buffer capacity.
    ///
    /// This initializes the `TagIterator` with a specific byte capacity.  The iterator will still reallocate if necessary. (Reallocation occurs if the iterator comes across a tag that should be output as a [`TagPosition::FullTag`] and its size in bytes is greater than the iterator's current buffer capacity.)
    ///
    pub fn with_capacity(source: R, tags_to_buffer: &[TSpec], capacity: usize) -> Self {
        let buffer = vec![0;capacity];

        TagIterator {
            source,
            tags_to_buffer: tags_to_buffer.iter().copied().collect(),
            buffer_all: false,
            buffer: buffer.into_boxed_slice(),
            buffered_byte_length: 0,
            buffer_offset: None,
            internal_buffer_position: 0,
            reached_eof: false,
            tag_stack: Vec::new(),
        }
    }

    fn current_offset(&self) -> usize {
        self.buffer_offset.unwrap_or(0) + self.internal_buffer_position
    }

    fn private_read(&mut self, internal_buffer_start: usize) -> Result<(), TagIteratorError> {        
        let bytes_read = self.source.read(&mut self.buffer[internal_buffer_start..]).map_err(|source| TagIteratorError::ReadError { source })?;
        if bytes_read == 0 {
            self.reached_eof = true;
        }
        self.buffered_byte_length += bytes_read;
        Ok(())
    }

    fn ensure_capacity(&mut self, required_capacity: usize) {
        if required_capacity > self.buffer.len() {
            let mut new_buffer = Vec::from(&self.buffer[..]);
            new_buffer.resize(required_capacity, 0);
            self.buffer = new_buffer.into_boxed_slice();
        }
    }

    fn ensure_data_read(&mut self, length: usize) -> Result<(), TagIteratorError> {
        if self.buffer_offset.is_none() {
            self.private_read(0)?;
            self.buffer_offset = Some(0);
            self.internal_buffer_position = 0;
        } else if self.internal_buffer_position + length > self.buffered_byte_length {
            self.buffer.copy_within(self.internal_buffer_position..self.buffered_byte_length, 0);
            self.buffered_byte_length -= self.internal_buffer_position;
            self.buffer_offset = Some(self.buffer_offset.unwrap() + self.internal_buffer_position);
            self.internal_buffer_position = 0;
            self.private_read(self.buffered_byte_length)?;
        }

        Ok(())
    }

    fn read_tag_id(&mut self) -> Result<u64, TagIteratorError> {
        self.ensure_data_read(8)?;
        match tools::read_vint(&self.buffer[self.internal_buffer_position..]).map_err(|e| TagIteratorError::CorruptedData(e.to_string()))? {
            Some((value, length)) => {
                self.internal_buffer_position += length;
                Ok(value + (1 << (7 * length)))
            },
            None => Err(TagIteratorError::CorruptedData(String::from("Expected tag id, but reached end of source."))),
        }
    }

    fn read_tag_size(&mut self) -> Result<usize, TagIteratorError> {
        self.ensure_data_read(8)?;
        match tools::read_vint(&self.buffer[self.internal_buffer_position..]).map_err(|e| TagIteratorError::CorruptedData(e.to_string()))? {
            Some((value, length)) => {
                self.internal_buffer_position += length;
                Ok(value.try_into().unwrap())
            },
            None => Err(TagIteratorError::CorruptedData(String::from("Expected tag size, but reached end of source."))),
        }
    }

    fn read_tag_data(&mut self, size: usize) -> Result<&[u8], TagIteratorError> {
        self.ensure_capacity(size);        
        self.ensure_data_read(size)?;

        self.internal_buffer_position += size;
        Ok(&self.buffer[(self.internal_buffer_position-size)..self.internal_buffer_position])
    }

    fn read_tag(&mut self) -> Result<SpecTag<TSpec>, TagIteratorError> {
        let tag_id = self.read_tag_id()?;
        let size: usize = self.read_tag_size()?;

        let spec_tag = <TSpec>::get_tag(tag_id);

        if spec_tag.is_none() {
            return Ok(SpecTag { 
                spec_tag: None, 
                tag: TagPosition::FullTag(tag_id, TagData::Binary(self.read_tag_data(size)?.to_vec()))
            });
        }

        let spec_tag = spec_tag.unwrap();

        let is_master = matches!(spec_tag.1, SpecificationTagDataType::Master);
        if is_master && !self.buffer_all && !self.tags_to_buffer.contains(&spec_tag.0) {
            self.tag_stack.push(ProcessingTag {
                spec_tag_type: spec_tag.0,
                id: tag_id,
                size,
                start: self.current_offset(),
            });

            Ok(SpecTag { spec_tag: Some(spec_tag.0), tag: TagPosition::StartTag(tag_id) })
        } else {
            let raw_data = self.read_tag_data(size)?;
            let tag_data = match spec_tag.1 {
                SpecificationTagDataType::Master => {
                    let mut src = Cursor::new(raw_data);
                    let mut sub_iterator: TagIterator<_, TSpec> = TagIterator::new(&mut src, &[]);
                    sub_iterator.buffer_all = true;

                    let children: Result<Vec<(u64, TagData)>, TagIteratorError> = sub_iterator.map(|c| {
                        match c?.tag {
                            TagPosition::FullTag(id, data) => Ok((id, data)),
                            _ => panic!("Everything should be buffered here"),
                        }
                    }).collect();

                    TagData::Master(children?)
                },
                SpecificationTagDataType::UnsignedInt => TagData::UnsignedInt(tools::arr_to_u64(raw_data)
                    .map_err(|e| TagIteratorError::SpecMismatch { tag_id, problem: SpecMismatchError::UintParseError(e.to_string()) })
                ?),
                SpecificationTagDataType::Integer => TagData::Integer(tools::arr_to_i64(raw_data)
                    .map_err(|e| TagIteratorError::SpecMismatch { tag_id, problem: SpecMismatchError::IntParseError(e.to_string()) })
                ?),
                SpecificationTagDataType::Utf8 => TagData::Utf8(String::from_utf8(raw_data.to_vec())
                    .map_err(|e| TagIteratorError::SpecMismatch { tag_id, problem: SpecMismatchError::Utf8ParseError { source: e } })
                ?),
                SpecificationTagDataType::Binary => TagData::Binary(raw_data.to_vec()),
                SpecificationTagDataType::Float => TagData::Float(tools::arr_to_f64(raw_data)
                    .map_err(|e| TagIteratorError::SpecMismatch { tag_id, problem: SpecMismatchError::FloatParseError(e.to_string()) })
                ?),
            };

            Ok(SpecTag { 
                spec_tag: Some(spec_tag.0), 
                tag: TagPosition::FullTag(tag_id, tag_data)
            })
        }
    }
}

///
/// A struct holding EBML tag data.  Emitted by [`TagIterator`].
///
/// This struct houses the specification tag type and tag data for items emitted by the `TagIterator`. The `spec_tag` is defined by the specification and represents the tag type (pulled from `TSpec` based on the tag id present in the data being read) and may be ignored if you prefer to work directly with the u64 tag ids.  Note that the `spec_tag` can be `None` if the id was not found in the defined specification. The `tag` contains the actual data enclosed in the tag along with the tag id.
/// 
pub struct SpecTag<TSpec> 
    where TSpec: EbmlSpecification<TSpec> + Eq + Hash + Copy
{
    pub spec_tag: Option<TSpec>,
    pub tag: TagPosition
}

impl<R: Read, TSpec> Iterator for TagIterator<R, TSpec>
    where TSpec: EbmlSpecification<TSpec> + Eq + Hash + Copy
{
    type Item = Result<SpecTag<TSpec>, TagIteratorError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(tag) = self.tag_stack.last() {
            if self.current_offset() >= tag.start + tag.size {
                let tag = self.tag_stack.pop().unwrap();
                return Some(Ok(SpecTag { spec_tag: Some(tag.spec_tag_type), tag: TagPosition::EndTag(tag.id) }));
            }
        }

        if self.internal_buffer_position >= self.buffered_byte_length {
            //If we've already consumed the entire internal buffer, ensure there is nothing else in the data
            //source before returning `None`
            let read_result = self.ensure_data_read(1);
            match read_result {
                Err(err) => return Some(Err(err)),
                Ok(()) => {
                    if self.reached_eof {
                        return None;
                    }
                }
            }
        }

        Some(self.read_tag())
    }
}
