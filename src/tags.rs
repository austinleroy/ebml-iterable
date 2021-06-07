///
/// `EbmlTag` is an enumeration containing three different classifications of tags that this library understands:
///

#[derive(PartialEq, Debug, Clone)]
pub enum EbmlTag {

    ///
    /// A marker for the beginning of a "master" tag as defined in EBML.  Master tags are simply containers for other tags.  The u64 value is the "id" of the tag.
    ///
    StartTag(u64),

    ///
    /// A marker for the end of a "master" tag.  The u64 value is the "id" of the tag.
    ///
    EndTag(u64),

    ///
    /// A complete tag that includes both the id and full data of the tag.  See [`DataTag`] for more detail.
    ///
    FullTag(DataTag),
}


impl EbmlTag {
    pub fn start_tag(&self) -> Option<u64> {
        match &self {
            EbmlTag::StartTag(id) => Some(*id),
            _ => None
        }
    }

    pub fn end_tag(&self) -> Option<u64> {
        match &self {
            EbmlTag::EndTag(id) => Some(*id),
            _ => None
        }
    }

    pub fn full_tag(self) -> Option<DataTag> {
        match self {
            EbmlTag::FullTag(data) => Some(data),
            _ => None
        }
    }
}

///
/// Holds a tag id along with the tag data.
///
/// A DataTag is a simple struct containing a tag id and the tag "data_type".  It is important to note that the type of data contained in the tag directly corresponds to the tag id as defined in whichever specification is in use.  Take care when creating this struct - specifying the wrong data type for a tag can result in corrupted output.
///
#[derive(PartialEq, Debug, Clone)]
pub struct DataTag {
    pub id: u64,
    pub data_type: DataTagType,
}

///
/// Contains the content of a tag.
///
/// This struct contains tag content data - i.e. the "meat" of the tag.  
///
/// Note: This library made a concious decision to not parse "Date" elements from EBML due to lack of built-in support for dates in Rust. Specification implementations should treat Date elements as Binary so that consumers have the option of parsing the unaltered data using their library of choice, if needed.
/// 

#[derive(PartialEq, Debug, Clone)]
pub enum DataTagType {

    ///
    /// A complete master tag containing any number of child tags.
    ///
    Master(Vec<DataTag>),

    ///
    /// An unsigned integer.
    ///
    UnsignedInt(u64),

    ///
    /// A signed integer.
    ///
    Integer(i64),

    ///
    /// A Unicode text string.  Note that the [EBML spec][https://datatracker.ietf.org/doc/rfc8794/] includes a separate element type for ASCII.  Given that ASCII is a subset of Utf8, this library currently parses and encodes both types using the same Utf8 element.
    ///
    Utf8(String),

    ///
    /// Binary data, otherwise uninterpreted.
    ///
    Binary(Vec<u8>),

    ///
    /// IEEE-754 floating point number.
    ///
    Float(f64),
}

impl DataTagType {
    pub fn master(self) -> Option<Vec<DataTag>> {
        match self {
            DataTagType::Master(children) => Some(children),
            _ => None
        }
    }

    pub fn unsigned_int(&self) -> Option<u64> {
        match &self {
            DataTagType::UnsignedInt(val) => Some(*val),
            _ => None
        }
    }

    pub fn integer(&self) -> Option<i64> {
        match &self {
            DataTagType::Integer(val) => Some(*val),
            _ => None
        }
    }

    pub fn utf8(self) -> Option<String> {
        match self {
            DataTagType::Utf8(val) => Some(val),
            _ => None
        }
    }

    pub fn binary(self) -> Option<Vec<u8>> {
        match self {
            DataTagType::Binary(val) => Some(val),
            _ => None
        }
    }

    pub fn float(&self) -> Option<f64> {
        match &self {
            DataTagType::Float(val) => Some(*val),
            _ => None
        }
    }
}
