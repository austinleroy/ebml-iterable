use std::io::Write;
use std::convert::{TryInto, TryFrom};

use super::tag_iterator_util::EBMLSize::{self, Known, Unknown};

use super::tools::Vint;
use super::specs::{EbmlSpecification, EbmlTag, TagDataType, Master};

use super::errors::tag_writer::TagWriterError;

///
/// Provides a tool to write EBML files based on Tags.  Writes to a destination that implements [`std::io::Write`].
///
/// Unlike the [`TagIterator`][`super::TagIterator`], this does not require a specification to write data. This writer provides the [`write_raw()`](#method.write_raw) method which can be used to write data that is outside of any specification.  The regular [`write()`](#method.write) method can be used to write any `TSpec` objects regardless of whether they came from a [`TagIterator`][`super::TagIterator`] or not.
///

pub struct TagWriter<W: Write>
{
    dest: W,
    open_tags: Vec<(u64, EBMLSize)>,
    working_buffer: Vec<u8>,
}

impl<W: Write> TagWriter<W>
{
    /// 
    /// Returns a new [`TagWriter`] instance.
    ///
    /// The `dest` parameter can be anything that implements [`std::io::Write`].
    ///
    pub fn new(dest: W) -> Self {
        TagWriter {
            dest,
            open_tags: Vec::new(),
            working_buffer: Vec::new(),
        }
    }

    fn start_tag(&mut self, id: u64) {
        self.open_tags.push((id, Known(self.working_buffer.len())));
    }

    fn end_tag(&mut self, id: u64) -> Result<(), TagWriterError> {
        match self.open_tags.pop() {
            Some(open_tag) => {
                if open_tag.0 == id {
                    if let Known(start) = open_tag.1 {
                        let size: u64 = self.working_buffer.len()
                            .checked_sub(start).expect("overflow subtracting tag size from working buffer length")
                            .try_into().expect("couldn't convert usize to u64");
    
                        let size_vint = size.as_vint()
                            .map_err(|e| TagWriterError::TagSizeError(e.to_string()))?;
    
                        self.working_buffer.splice(start..start, open_tag.0.to_be_bytes().iter().skip_while(|&v| *v == 0u8).chain(size_vint.iter()).copied());
                    }
                    Ok(())
                } else {
                    Err(TagWriterError::UnexpectedClosingTag { tag_id: id, expected_id: Some(open_tag.0) })
                }
            },
            None => Err(TagWriterError::UnexpectedClosingTag { tag_id: id, expected_id: None })
        }
    }

    fn private_flush(&mut self) -> Result<(), TagWriterError> {
        self.dest.write_all(self.working_buffer.drain(..).as_slice()).map_err(|source| TagWriterError::WriteError { source })?;
        self.dest.flush().map_err(|source| TagWriterError::WriteError { source })
    }

    fn write_unsigned_int_tag(&mut self, id: u64, data: &u64) -> Result<(), TagWriterError> {
        self.working_buffer.extend(id.to_be_bytes().iter().skip_while(|&v| *v == 0u8));
        let data = *data;
        u8::try_from(data).map(|n| {
                self.working_buffer.push(0x81); // vint representation of "1"
                self.working_buffer.extend_from_slice(&n.to_be_bytes());
            })
            .or_else(|_| u16::try_from(data).map(|n| { 
                self.working_buffer.push(0x82); // vint representation of "2"
                self.working_buffer.extend_from_slice(&n.to_be_bytes());
            }))
            .or_else(|_| u32::try_from(data).map(|n| { 
                self.working_buffer.push(0x84); // vint representation of "4"
                self.working_buffer.extend_from_slice(&n.to_be_bytes());
            }))
            .unwrap_or_else(|_| { 
                self.working_buffer.push(0x88); // vint representation of "8"
                self.working_buffer.extend_from_slice(&data.to_be_bytes());
            });
        Ok(())
    }

    fn write_signed_int_tag(&mut self, id: u64, data: &i64) -> Result<(), TagWriterError> {
        self.working_buffer.extend(id.to_be_bytes().iter().skip_while(|&v| *v == 0u8));
        let data = *data;
        i8::try_from(data).map(|n| { 
                self.working_buffer.push(0x81); // vint representation of "1"
                self.working_buffer.extend_from_slice(&n.to_be_bytes());
            })
            .or_else(|_| i16::try_from(data).map(|n| { 
                self.working_buffer.push(0x82); // vint representation of "2"
                self.working_buffer.extend_from_slice(&n.to_be_bytes());
            }))
            .or_else(|_| i32::try_from(data).map(|n| { 
                self.working_buffer.push(0x84); // vint representation of "4"
                self.working_buffer.extend_from_slice(&n.to_be_bytes());
            }))
            .unwrap_or_else(|_| { 
                self.working_buffer.push(0x88); // vint representation of "8"
                self.working_buffer.extend_from_slice(&data.to_be_bytes());
            });
        Ok(())
    }

    fn write_utf8_tag(&mut self, id: u64, data: &str) -> Result<(), TagWriterError> {
        self.working_buffer.extend(id.to_be_bytes().iter().skip_while(|&v| *v == 0u8));

        let slice: &[u8] = data.as_bytes();
        let size: u64 = slice.len().try_into().expect("couldn't convert usize to u64");
        let size_vint = size.as_vint().map_err(|e| TagWriterError::TagSizeError(e.to_string()))?;
        self.working_buffer.extend_from_slice(&size_vint);

        self.working_buffer.extend_from_slice(slice);
        Ok(())
    }

    fn write_binary_tag(&mut self, id: u64, data: &[u8]) -> Result<(), TagWriterError> {
        self.working_buffer.extend(id.to_be_bytes().iter().skip_while(|&v| *v == 0u8));

        let size: u64 = data.len().try_into().expect("couldn't convert usize to u64");
        let size_vint = size.as_vint().map_err(|e| TagWriterError::TagSizeError(e.to_string()))?;
        self.working_buffer.extend_from_slice(&size_vint);

        self.working_buffer.extend_from_slice(data);
        Ok(())
    }

    fn write_float_tag(&mut self, id: u64, data: &f64) -> Result<(), TagWriterError> {
        self.working_buffer.extend(id.to_be_bytes().iter().skip_while(|&v| *v == 0u8));
        self.working_buffer.push(0x88); // vint representation of "8"
        self.working_buffer.extend_from_slice(&data.to_be_bytes());
        Ok(())
    }

    ///
    /// Write a tag to this instance's destination.
    ///
    /// This method writes a tag from any specification.  There are no restrictions on the type of specification being written - it simply needs to implement the [`EbmlSpecification`] and [`EbmlTag`] traits.
    ///
    /// ## Errors
    /// 
    /// This method can error if there is a problem writing the input tag.  The different possible error states are enumerated in [`TagWriterError`].
    ///
    /// ## Panics
    ///
    /// This method can panic if `<TSpec>` is an internally inconsistent specification (i.e. it claims that a specific tag variant is a specific data type but it is not).  This won't happen if the specification being used was created using the [`#[ebml_specification]`](https://docs.rs/ebml-iterable-specification-derive/latest/ebml_iterable_specification_derive/attr.ebml_specification.html) attribute macro.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use ebml_iterable::TagWriter;
    /// use ebml_iterable::specs::Master;
    /// # use ebml_iterable_specification::empty_spec::EmptySpec;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut file = File::create("my_ebml_file.ebml")?;
    /// let mut my_writer = TagWriter::new(&mut file);
    /// my_writer.write(&EmptySpec::with_children(
    ///   0x1a45dfa3, 
    ///   vec![EmptySpec::with_data(0x18538067, &[0x01])])
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    pub fn write<TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone>(&mut self, tag: &TSpec) -> Result<(), TagWriterError> {
        let tag_id = tag.get_id();
        match TSpec::get_tag_data_type(tag_id) {
            TagDataType::UnsignedInt => {
                let val = tag.as_unsigned_int().unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was unsigned int, but could not get tag!", tag_id));
                self.write_unsigned_int_tag(tag_id, val)?
            },
            TagDataType::Integer => {
                let val = tag.as_signed_int().unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was integer, but could not get tag!", tag_id));
                self.write_signed_int_tag(tag_id, val)?
            },
            TagDataType::Utf8 => {
                let val = tag.as_utf8().unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was utf8, but could not get tag!", tag_id));
                self.write_utf8_tag(tag_id, val)?
            },
            TagDataType::Binary => {
                let val = tag.as_binary().unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was binary, but could not get tag!", tag_id));
                self.write_binary_tag(tag_id, val)?
            },
            TagDataType::Float => {
                let val = tag.as_float().unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was float, but could not get tag!", tag_id));
                self.write_float_tag(tag_id, val)?
            },
            TagDataType::Master => {
                let position = tag.as_master().unwrap_or_else(|| panic!("Bad specification implementation: Tag id {} type was master, but could not get tag!", tag_id));

                match position {
                    Master::Start => self.start_tag(tag_id),
                    Master::End => self.end_tag(tag_id)?,
                    Master::Full(children) => {
                        self.start_tag(tag_id);
                        for child in children {
                            self.write(child)?;
                        }
                        self.end_tag(tag_id)?;
                    }
                }
            }
        }

        if !self.open_tags.iter().any(|t| matches!(t.1, Known(_))) {
            self.private_flush()
        } else {
            Ok(())
        }
    }

    ///
    /// Write a tag with an unknown size to this instance's destination.
    /// 
    /// This method allows you to start a tag that doesn't have a known size.  Useful for streaming, or when the data is expected to be too large to fit into memory.  This method can *only* be used on Master type tags.
    /// 
    /// ## Errors
    /// 
    /// This method will return an error if the input tag is not a Master type tag, as those are the only types allowed to be of unknown size.
    /// 
    pub fn write_unknown_size<TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone>(&mut self, tag: &TSpec) -> Result<(), TagWriterError> {
        let tag_id = tag.get_id();
        let tag_type = TSpec::get_tag_data_type(tag_id);
        match tag_type {
            TagDataType::Master => {},
            _ => {
                return Err(TagWriterError::TagSizeError(format!("Cannot write an unknown size for tag of type {:?}", tag_type)))
            }
        };
        self.working_buffer.extend(tag_id.to_be_bytes().iter().skip_while(|&v| *v == 0u8));
        self.working_buffer.extend_from_slice(&(u64::MAX >> 7).to_be_bytes());
        self.open_tags.push((tag_id, Unknown));
        Ok(())
    }

    ///
    /// Write raw tag data to this instance's destination.
    ///
    /// This method allows writing any tag id with any arbitrary data without using a specification.  Specifications should generally provide an `Unknown` variant to handle arbitrary unknown data which can be written through the regular [`write()`](#method.write) method, so use of this method is typically discouraged.
    ///
    /// ## Errors
    /// 
    /// This method can error if there is a problem writing the input tag.  The different possible error states are enumerated in [`TagWriterError`].
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use ebml_iterable::TagWriter;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut file = File::create("my_ebml_file.ebml")?;
    /// let mut my_writer = TagWriter::new(&mut file);
    /// my_writer.write_raw(0x1a45dfa3, &[0x18, 0x53, 0x80, 0x67, 0x81, 0x01])?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    pub fn write_raw(&mut self, tag_id: u64, data: &[u8]) -> Result<(), TagWriterError> {
        self.write_binary_tag(tag_id, data)?;
        
        if !self.open_tags.iter().any(|t| matches!(t.1, Known(_))) {
            self.private_flush()
        } else {
            Ok(())
        }        
    }

    ///
    /// Attempts to flush all unwritten tags to the underlying destination.
    /// 
    /// This method can be used to finalize any open [`Master`] type tags that have not been ended.  The writer makes an attempt to close every open tag and write all bytes to the instance's destination.
    /// 
    /// ## Errors
    /// 
    /// This method can error if there is a problem writing to the destination.
    /// 
    pub fn flush(&mut self) -> Result<(), TagWriterError> {
        while let Some(id) = self.open_tags.last().map(|t| t.0) {
            self.end_tag(id)?;
        }
        self.private_flush()
    }

    //TODO: panic on drop if there is an open tag that hasn't been written.  Or maybe flush stream of any open tags?
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::super::tools::Vint;
    use super::TagWriter;

    #[test]
    fn write_ebml_tag() {
        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);
        writer.write_raw(0x1a45dfa3, &[]).expect("Error writing tag");

        let zero_size = 0u64.as_vint().expect("Error converting [0] to vint")[0];
        assert_eq!(vec![0x1a, 0x45, 0xdf, 0xa3, zero_size], dest.get_ref().to_vec());
    }
}
