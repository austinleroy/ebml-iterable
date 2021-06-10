//! This crate provides an iterator and a serializer for [EBML][EBML] files.  Its primary goal is to provide typed iteration and serialization as lightly and quickly as possible.
//! 
//! [EBML][EBML] stands for Extensible Binary Meta-Language and is somewhat of a
//! binary version of XML. It's used for container formats like [WebM][webm] or
//! [MKV][mkv].
//! 
//! # Important - Specifications
//! The iterator contained in this crate is spec-agnostic and requires a specification implementing the [`specs::EbmlSpecification`] trait to read files.  Typically, you would only use this crate to implement a custom specification - most often you would prefer a crate providing an existing specification, like [webm-iterable][webm-iterable].
//! 
//! Implementing custom specifications can be made less painful by enabling the `"derive-spec"` feature flag in this crate and using the `EbmlSpecification` macro.
//! 
//! # Features
//! 
//! There is currently only one optional feature in this crate, but that may change over time as needs arise.
//! 
//! * **derive-spec** -
//!     When enabled, this provides a macro to simplify implementations of the [`EbmlSpecification`][`specs::EbmlSpecification`] trait.  This introduces dependencies on [`syn`](https://crates.io/crates/syn), [`quote`](https://crates.io/crates/quote), and [`proc-macro2`](https://crates.io/crates/proc-macro2), so expect compile times to increase a little.
//! 
//! # Known Limitations
//! This library was not built to work with an "Unknown Data Size" as defined in [RFC8794][rfc8794]. As such, it likely will not support streaming applications and will only work on complete datasets.
//! 
//! [EBML]: http://ebml.sourceforge.net/
//! [webm]: https://www.webmproject.org/
//! [mkv]: http://www.matroska.org/technical/specs/index.html
//! [rfc8794]: https://datatracker.ietf.org/doc/rfc8794/
//! [webm-iterable]: https://crates.io/crates/webm_iterable
//! 

mod errors;
mod tag_iterator;
mod tag_writer;
pub mod tools;
pub mod specs;
pub mod tags;

pub use self::tag_iterator::TagIterator;
pub use self::tag_writer::TagWriter;
