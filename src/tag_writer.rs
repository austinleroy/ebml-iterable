use std::io::Write;
use std::convert::{TryInto, TryFrom};

use super::tools::Vint;
use super::tags::{DataTag, DataTagType, EbmlTag};

use super::errors::tag_writer::TagWriterError;

///
/// Provides a tool to write EBML files based on Tags.  Writes to a destination that implements [`std::io::Write`].
///
/// Unlike the [TagIterator][`super::TagIterator`], this does not require a specification to write data. The reason for this is that tags passed into this writer *must* provide the tag id, and these tags by necessity have their data in a format that can be encoded to binary. Because a specification is really only useful for providing context for tags based on the tag id, there is little value in using a specification during writing (other than ensuring that tag data matches the format described by the specification, which is not currently implemented.)  The `TagWriter` can  write any `EbmlTag` objects regardless of whether they came from a `TagIterator` or not.
///
/// ## Example
/// 
/// ```no_run
/// use std::fs::File;
/// use ebml_iterable::TagWriter;
/// use ebml_iterable::tags::{EbmlTag, DataTag, DataTagType};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut file = File::create("my_ebml_file.ebml")?;
/// let mut my_writer = TagWriter::new(&mut file);
/// my_writer.write(EbmlTag::FullTag(DataTag { 
///   id: 0x1a45dfa3, 
///   data_type: DataTagType::Master(Vec::new()) 
/// }))?;
/// # Ok(())
/// # }
/// ```
///

pub struct TagWriter<'a> {
    dest: &'a mut dyn Write,
    open_tags: Vec<(u64, usize)>,
    working_buffer: Vec<u8>,
}

impl<'a> TagWriter<'a> {
    pub fn new(dest: &'a mut dyn Write) -> Self {
        TagWriter {
            dest,
            open_tags: Vec::new(),
            working_buffer: Vec::new(),
        }
    }

    fn start_tag(&mut self, id: u64) {
        self.open_tags.push((id, self.working_buffer.len()));
    }

    fn end_tag(&mut self, id: u64) -> Result<(), TagWriterError> {
        match self.open_tags.pop() {
            Some(open_tag) => {
                if open_tag.0 == id {
                    self.finalize_tag(open_tag.0, (self.working_buffer.len() - open_tag.1).try_into().unwrap())?;
                    Ok(())
                } else {
                    Err(TagWriterError::UnexpectedClosingTag { tag_id: id, expected_id: Some(open_tag.0) })
                }
            },
            None => Err(TagWriterError::UnexpectedClosingTag { tag_id: id, expected_id: None })
        }
    }

    fn write_full_tag(&mut self, tag: DataTag) -> Result<(), TagWriterError> {
        let mut size: u64 = 0;
        match tag.data_type {
            DataTagType::Master(children) => {
                self.write(EbmlTag::StartTag(tag.id))?;
                for child in children {
                    self.write(EbmlTag::FullTag(child))?;
                }
                self.write(EbmlTag::EndTag(tag.id))?;
                return Ok(());
            },
            DataTagType::UnsignedInt(val) => 
                u8::try_from(val).map(|n| { self.working_buffer.extend_from_slice(&n.to_be_bytes()); size = 1; })
                    .or_else(|_| u16::try_from(val).map(|n| { self.working_buffer.extend_from_slice(&n.to_be_bytes()); size = 2; }))
                    .or_else(|_| u32::try_from(val).map(|n| { self.working_buffer.extend_from_slice(&n.to_be_bytes()); size = 4; }))
                    .unwrap_or_else(|_| { self.working_buffer.extend_from_slice(&val.to_be_bytes()); size = 8; })
            ,
            DataTagType::Integer(val) => 
                i8::try_from(val).map(|n| { self.working_buffer.extend_from_slice(&n.to_be_bytes()); size = 1; })
                    .or_else(|_| i16::try_from(val).map(|n| { self.working_buffer.extend_from_slice(&n.to_be_bytes()); size = 2; }))
                    .or_else(|_| i32::try_from(val).map(|n| { self.working_buffer.extend_from_slice(&n.to_be_bytes()); size = 4; }))
                    .unwrap_or_else(|_| { self.working_buffer.extend_from_slice(&val.to_be_bytes()); size = 8; })
            ,
            DataTagType::Utf8(val) => { 
                let slice = val.as_bytes();
                self.working_buffer.extend_from_slice(slice);
                size = slice.len().try_into().unwrap();
            },
            DataTagType::Binary(val) => { self.working_buffer.extend_from_slice(&val); size = val.len().try_into().unwrap(); },
            DataTagType::Float(val) => { self.working_buffer.extend_from_slice(&val.to_be_bytes()); size = 8; },
        };

        let tag_id = tag.id;
        self.finalize_tag(tag_id, size)?;
        Ok(())
    }

    fn finalize_tag(&mut self, id: u64, size: u64) -> Result<(), TagWriterError> {
        let size_vint = size.as_vint()
            .map_err(|e| TagWriterError::TagSizeError(e.to_string()))?;

        let index: usize = self.working_buffer.len().checked_sub(size.try_into().unwrap()).unwrap();
        self.working_buffer.splice(index..index, id.to_be_bytes().iter().skip_while(|&v| *v == 0u8).chain(size_vint.iter()).copied());

        if self.open_tags.is_empty() {
            self.dest.write_all(&self.working_buffer.drain(..).as_slice()).map_err(|source| TagWriterError::WriteError { source })?;
            self.dest.flush().map_err(|source| TagWriterError::WriteError { source })?;
        }

        Ok(())
    }

    pub fn write(&mut self, tag: EbmlTag) -> Result<(), TagWriterError> {
        match tag {
            EbmlTag::StartTag(id) => self.start_tag(id),
            EbmlTag::EndTag(id) => self.end_tag(id)?,
            EbmlTag::FullTag(tag) => self.write_full_tag(tag)?,
        }

        Ok(())
    }

    //TODO: panic on drop if there is an open tag that hasn't been written.  Or maybe flush stream of any open tags?
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::super::tools::Vint;
    use super::super::tags::{EbmlTag, DataTag, DataTagType};
    use super::TagWriter;

    #[test]
    fn write_ebml_tag() {
        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);
        writer.write(EbmlTag::FullTag(DataTag { id: 0x1a45dfa3, data_type: DataTagType::Master(Vec::new()) })).expect("Error writing tag");

        let zero_size = 0u64.as_vint().expect("Error converting [0] to vint")[0];
        assert_eq!(vec![0x1a, 0x45, 0xdf, 0xa3, zero_size], dest.get_ref().to_vec());
    }
}
