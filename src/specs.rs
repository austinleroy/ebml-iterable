//!
//! Provides the EBML specification types.
//!
//! Typically won't be used unless you are implementing a custom specification that uses EBML.  You can enable the `"derive-spec"` feature to obtain a macro to make implementation easier.
//!

#[cfg(feature = "derive-spec")]
pub use ebml_iterable_specification_derive::ebml_specification;
#[cfg(feature = "derive-spec")]
pub use ebml_iterable_specification_derive::easy_ebml;

pub use ebml_iterable_specification::EbmlSpecification as EbmlSpecification;
pub use ebml_iterable_specification::EbmlTag as EbmlTag;
pub use ebml_iterable_specification::TagDataType as TagDataType;
pub use ebml_iterable_specification::Master as Master;
