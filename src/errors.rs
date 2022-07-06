use std::fmt;
use std::error::Error;

pub mod tool {
    use super::fmt;
    use super::Error;

    use std::string::FromUtf8Error;

    #[derive(Debug)]
    pub enum ToolError {
        ReadVintOverflow,
        WriteVintOverflow(u64),
        ReadU64Overflow(Vec<u8>),
        ReadI64Overflow(Vec<u8>),
        ReadF64Mismatch(Vec<u8>),
        FromUtf8Error(Vec<u8>, FromUtf8Error)
    }

    impl fmt::Display for ToolError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ToolError::ReadVintOverflow => write!(f, "Unrepresentable Vint size encountered."),
                ToolError::WriteVintOverflow(val) => write!(f, "Value too large to be written as a vint: {}", val),
                ToolError::ReadU64Overflow(arr) => write!(f, "Could not read unsigned int from array: {:?}", arr),
                ToolError::ReadI64Overflow(arr) => write!(f, "Could not read int from array: {:?}", arr),
                ToolError::ReadF64Mismatch(arr) => write!(f, "Could not read float from array: {:?}", arr),
                ToolError::FromUtf8Error(arr, _source) => write!(f, "Could not read utf8 data: {:?}", arr),
            }
        }
    }

    impl Error for ToolError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                ToolError::FromUtf8Error(_arr, source) => Some(source),
                _ => None,
            }
        }
    }
}

pub mod tag_iterator {
    use super::fmt;
    use super::Error;
    use super::tool::ToolError;
    use std::io;

    ///
    /// Errors that can occur when reading ebml data.
    ///
    #[derive(Debug)]
    pub enum TagIteratorError {

        ///
        /// An error indicating that the file being read is not valid ebml.
        ///
        /// This error typically occurs if the file ends unexpectedly or has an unreadable tag id.
        ///
        CorruptedFileData(String),

        ///
        /// An error indicating that tag data appears to be corrupted.
        ///
        /// This error typically occurs if tag data cannot be read as its expected data type (e.g. trying to read `[32,42,8]` as float data, since floats require either 4 or 8 bytes).
        ///
        CorruptedTagData {

            ///
            /// The id of the corrupted tag.
            ///
            tag_id: u64,

            ///
            /// An error describing why the data is corrupted.
            ///
            problem: ToolError,
        },

        ///
        /// An error that wraps an IO error when reading from the underlying source.
        ///
        ReadError {

            ///
            /// The [`io::Error`] that caused this problem.
            ///
            source: io::Error,
        },
    }
    
    impl fmt::Display for TagIteratorError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                TagIteratorError::CorruptedFileData(message) => write!(f, "Encountered corrupted data.  Message: {}", message),
                TagIteratorError::CorruptedTagData {
                    tag_id,
                    problem,
                } => write!(f, "Error reading data for tag id (0x{:x?}). {}", tag_id, problem),
                TagIteratorError::ReadError { source: _ } => write!(f, "Error reading from source."),
            }
        }
    }
    
    impl Error for TagIteratorError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                TagIteratorError::CorruptedFileData(_) => None,
                TagIteratorError::CorruptedTagData { tag_id: _, problem } => problem.source(),
                TagIteratorError::ReadError { source } => Some(source),
            }
        }
    }
}

pub mod tag_writer {
    use super::fmt;
    use super::Error;
    use std::io;

    ///
    /// Errors that can occur when writing ebml data.
    ///
    #[derive(Debug)]
    pub enum TagWriterError {

        ///
        /// An error with the size of a tag.
        ///
        /// Can occur if the tag size overflows the max value representable by a vint (`2^57 - 1`, or `144,115,188,075,855,871`).
        /// 
        /// This can also occur if a non-[`Master`][`crate::specs::TagDataType::Master`] tag is sent to be written with an unknown size.
        ///
        TagSizeError(String),

        ///
        /// An error indicating a tag was closed unexpectedly.
        ///
        /// Can occur if a [`Master::End`][`crate::specs::Master::End`] variant is passed to the [`TagWriter`][`crate::TagWriter`] but the id doesn't match the currently open tag.
        ///
        UnexpectedClosingTag {

            ///
            /// The id of the tag being closed.
            ///
            tag_id: u64,

            ///
            /// The id of the currently open tag.
            ///
            expected_id: Option<u64>,
        },

        ///
        /// An error that wraps an IO error when writing to the underlying destination.
        ///
        WriteError {
            source: io::Error,
        },
    }

    impl fmt::Display for TagWriterError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                TagWriterError::TagSizeError(message) => write!(f, "Problem writing data tag size. {}", message),
                TagWriterError::UnexpectedClosingTag { tag_id, expected_id } => match expected_id {
                    Some(expected) => write!(f, "Unexpected closing tag 0x'{:x?}'. Expected 0x'{:x?}'", tag_id, expected),
                    None => write!(f, "Unexpected closing tag 0x'{:x?}'", tag_id),
                },
                TagWriterError::WriteError { source: _ } => write!(f, "Error writing to destination."),
            }
        }
    }
    
    impl Error for TagWriterError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                TagWriterError::TagSizeError(_) => None,
                TagWriterError::UnexpectedClosingTag { tag_id: _, expected_id: _ } => None,
                TagWriterError::WriteError { source } => Some(source),
            }
        }
    }
}