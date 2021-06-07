pub enum SpecTagType {
    Master,
    UnsignedInt,
    Integer,
    Utf8,
    Binary,
    Float,
}

///
/// This trait should be implemented to define a specification so that EBML can be parsed correctly.
/// 
/// Any specification using EBML can take advantage of this library to parse or write binary data.  As stated in the docs, [TagWriter][`super::TagWriter`] needs nothing special, but [TagIterator][`super::TagIterator`] requires a struct implementing this trait.  Custom specification implementations can refer to [webm-iterable](https://crates.io/crates/webm_iterable) as an example.
///

pub trait TagSpec {
    type SpecType: Copy;

///
/// Pulls the "type" of tag from the spec based on the tag id.
///
/// Implementors can reference [webm-iterable](https://crates.io/crates/webm_iterable) for an example.
///
    fn get_tag(&self, id: u64) -> Self::SpecType;

///
/// Identifies the type of data that is stored in a specific tag "type".
///
/// Implementors can reference [webm-iterable](https://crates.io/crates/webm_iterable) for an example.
///
    fn get_tag_type(&self, tag: &Self::SpecType) -> SpecTagType;
}
