// //!
// //! Contains types that hold tag data.
// //! 
// //! These types are used by both the [`TagIterator`][`super::TagIterator`] and [`TagWriter`][`super::TagWriter`].
// //!

// ///
// /// `TagPosition` is an enumeration containing three different tag "positions" that this library works with:
// ///

// #[derive(PartialEq, Debug, Clone)]
// pub enum TagPosition {

//     ///
//     /// A marker for the beginning of a "master" tag as defined in EBML.  Master tags are simply containers for other tags.  The u64 value is the "id" of the tag.
//     ///
//     StartTag(u64),

//     ///
//     /// A marker for the end of a "master" tag.  The u64 value is the "id" of the tag.
//     ///
//     EndTag(u64),

//     ///
//     /// A complete tag that includes both the id and full data of the tag.  See [`TagData`] for more detail.
//     ///
//     FullTag(u64, TagData),
// }


// impl TagPosition {
//     pub fn start_tag(&self) -> Option<u64> {
//         match &self {
//             TagPosition::StartTag(id) => Some(*id),
//             _ => None
//         }
//     }

//     pub fn end_tag(&self) -> Option<u64> {
//         match &self {
//             TagPosition::EndTag(id) => Some(*id),
//             _ => None
//         }
//     }

//     pub fn full_tag(self) -> Option<(u64, TagData)> {
//         match self {
//             TagPosition::FullTag(id, data) => Some((id, data)),
//             _ => None
//         }
//     }
// }

// ///
// /// Contains the content of a tag.
// ///
// /// This struct contains tag content data - i.e. the "meat" of the tag.  
// ///
// /// Note: This library made a concious decision to not parse "Date" elements from EBML due to lack of built-in support for dates in Rust. Specification implementations should treat Date elements as Binary so that consumers have the option of parsing the unaltered data using their library of choice, if needed.
// /// 

// #[derive(PartialEq, Debug, Clone)]
// pub enum TagData {

//     ///
//     /// A complete master tag containing any number of child tags.
//     ///
//     Master(Vec<(u64, TagData)>),

//     ///
//     /// An unsigned integer.
//     ///
//     UnsignedInt(u64),

//     ///
//     /// A signed integer.
//     ///
//     Integer(i64),

//     ///
//     /// A Unicode text string.  Note that the [EBML spec][https://datatracker.ietf.org/doc/rfc8794/] includes a separate element type for ASCII.  Given that ASCII is a subset of Utf8, this library currently parses and encodes both types using the same Utf8 element.
//     ///
//     Utf8(String),

//     ///
//     /// Binary data, otherwise uninterpreted.
//     ///
//     Binary(Vec<u8>),

//     ///
//     /// IEEE-754 floating point number.
//     ///
//     Float(f64),
// }

// impl TagData {
//     pub fn master(self) -> Option<Vec<(u64, TagData)>> {
//         match self {
//             TagData::Master(children) => Some(children),
//             _ => None
//         }
//     }

//     pub fn unsigned_int(&self) -> Option<u64> {
//         match &self {
//             TagData::UnsignedInt(val) => Some(*val),
//             _ => None
//         }
//     }

//     pub fn integer(&self) -> Option<i64> {
//         match &self {
//             TagData::Integer(val) => Some(*val),
//             _ => None
//         }
//     }

//     pub fn utf8(self) -> Option<String> {
//         match self {
//             TagData::Utf8(val) => Some(val),
//             _ => None
//         }
//     }

//     pub fn binary(self) -> Option<Vec<u8>> {
//         match self {
//             TagData::Binary(val) => Some(val),
//             _ => None
//         }
//     }

//     pub fn float(&self) -> Option<f64> {
//         match &self {
//             TagData::Float(val) => Some(*val),
//             _ => None
//         }
//     }
// }
