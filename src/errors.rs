use std::fmt;
use std::error::Error;

pub mod tool {
    use super::fmt;
    use super::Error;

    #[derive(Debug)]
    pub enum ToolError {
        ReadVintOverflow,
        WriteVintOverflow(u64),
        ReadU64Overflow(Vec<u8>),
        ReadI64Overflow(Vec<u8>),
        ReadF64Mismatch(Vec<u8>),
    }

    impl fmt::Display for ToolError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ToolError::ReadVintOverflow => write!(f, "Unrepresentable Vint size encountered."),
                ToolError::WriteVintOverflow(val) => write!(f, "Value too large to be written as a vint: {}", val),
                ToolError::ReadU64Overflow(arr) => write!(f, "Could not read unsigned int from array: {:?}", arr),
                ToolError::ReadI64Overflow(arr) => write!(f, "Could not read int from array: {:?}", arr),
                ToolError::ReadF64Mismatch(arr) => write!(f, "Could not read float from array: {:?}", arr),
            }
        }
    }

    impl Error for ToolError {}
}

pub mod specs {
    use super::fmt;
    use super::Error;
    use std::string;

    #[derive(Debug)]
    pub enum SpecMismatchError {
        UintParseError(String),
        IntParseError(String),
        Utf8ParseError {
            source: string::FromUtf8Error,
        },
        FloatParseError(String),
    }

    impl fmt::Display for SpecMismatchError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                SpecMismatchError::UintParseError(err) => write!(f, "Error parsing data as Unsigned Int: {}", err),
                SpecMismatchError::IntParseError(err) => write!(f, "Error parsing data as Integer: {}", err),
                SpecMismatchError::Utf8ParseError { source: _ } => write!(f, "Error parsing data as Utf8.  See `source()` for details."),
                SpecMismatchError::FloatParseError(err) => write!(f, "Error parsing data as Float: {}", err),
            }
        }
    }

    impl Error for SpecMismatchError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                SpecMismatchError::Utf8ParseError { source } => Some(source),
                _ => None,
            }
        }
    }
}

pub mod tag_iterator {
    use super::fmt;
    use super::Error;
    use super::specs::SpecMismatchError;
    use std::io;

    #[derive(Debug)]
    pub enum TagIteratorError {
        CorruptedData(String),
        SpecMismatch {
            tag_id: u64,
            problem: SpecMismatchError,
        },
        UnknownTag {
            id: u64,
            data: Vec<u8>,
        },
        ReadError {
            source: io::Error,
        },
    }
    
    impl fmt::Display for TagIteratorError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                TagIteratorError::CorruptedData(message) => write!(f, "Encountered corrupted data.  Message: {}", message),
                TagIteratorError::SpecMismatch {
                    tag_id,
                    problem,
                } => write!(f, "Source data does not seem to match tag specification for tag id ({}). {}", tag_id, problem),
                TagIteratorError::ReadError { source: _ } => write!(f, "Error reading from source."),
                TagIteratorError::UnknownTag { id, data: _ } => write!(f, "Unknown tag id: {}", id),
            }
        }
    }
    
    impl Error for TagIteratorError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                TagIteratorError::CorruptedData(_) => None,
                TagIteratorError::UnknownTag{ id: _, data: _ } => None,
                TagIteratorError::SpecMismatch { tag_id: _, problem } => problem.source(),
                TagIteratorError::ReadError { source } => Some(source),
            }
        }
    }
}

pub mod tag_writer {
    use super::fmt;
    use super::Error;
    use std::io;

    #[derive(Debug)]
    pub enum TagWriterError {
        TagSizeError(String),
        UnexpectedClosingTag {
            tag_id: u64,
            expected_id: Option<u64>,
        },
        WriteError {
            source: io::Error,
        },
    }

    impl fmt::Display for TagWriterError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                TagWriterError::TagSizeError(message) => write!(f, "Problem writing data tag size. {}", message),
                TagWriterError::UnexpectedClosingTag { tag_id, expected_id } => match expected_id {
                    Some(expected) => write!(f, "Unexpected closing tag '{}'. Expected '{}'", tag_id, expected),
                    None => write!(f, "Unexpected closing tag '{}'", tag_id),
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