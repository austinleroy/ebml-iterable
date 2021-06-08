#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum TagDataType {
    Master,
    UnsignedInt,
    Integer,
    Utf8,
    Binary,
    Float,
}

///
/// This trait should be implemented to define a specification so that EBML can be parsed correctly.  Typically implemented on an Enum of tag variants.
/// 
/// Any specification using EBML can take advantage of this library to parse or write binary data.  As stated in the docs, [TagWriter][`super::TagWriter`] needs nothing special, but [TagIterator][`super::TagIterator`] requires a struct implementing this trait.  Custom specification implementations can refer to [webm-iterable](https://crates.io/crates/webm_iterable) as an example.
///

pub trait EbmlSpecification<T: EbmlSpecification<T>> {
///
/// Pulls the "type" of tag and the tag data type from the spec based on the tag id.
///
/// This function *must* return `None` if the input id is not a `TagSpecificationDataType` in the specification.  Implementors can reference [webm-iterable](https://crates.io/crates/webm_iterable) for an example.
///
    fn get_tag(id: u64) -> Option<(T, TagDataType)>;

///
/// Gets the id of a specific tag "type".
///
/// Implementors can reference [webm-iterable](https://crates.io/crates/webm_iterable) for an example.
///
    fn get_tag_id(item: &T) -> u64;

///
/// Gets the type of data that is stored in a specific tag "type".
///
/// The default implementation simply calls [`get_tag_id`] followed by [`get_tag`] and maps the result as the return value.  This function *must* return `None` if the input id is not a `TagSpecificationDataType` in the specification.
///
/// [`get_tag_id`]: #method.get_tag_id
/// [`get_tag`]: #method.get_tag
    fn get_tag_data_type(item: &T) -> Option<TagDataType> {
        <T>::get_tag(<T>::get_tag_id(item)).map(|tag: (T, TagDataType)| tag.1)
    }
}
