[EBML][EBML] stands for Extensible Binary Meta-Language and is somewhat of a
binary version of XML. It's used for container formats like [WebM][webm] or
[MKV][mkv].

> IMPORTANT: The iterator contained in this crate is spec-agnostic and requires a specification implementing the `EbmlSpecification` trait to read files.  Typically, you would only use this crate to implement a custom specification - most often you would prefer a crate providing an existing specification, like [webm-iterable][webm-iterable].

> KNOWN LIMITATION: This library was not built to work with an "Unknown Data Size" as defined in [RFC8794][rfc8794]. As such, it likely will not support streaming applications and will only work on complete datasets.

```Cargo.toml
[dependencies]
ebml-iterable = "0.1.0"
```

# Usage

The `TagIterator` struct implements Rust's standard [Iterator][rust-iterator] trait.
This struct can be created with the `new` function on any source that implements the standard [Read][rust-read] trait. The iterator outputs `SpecTag` objects reflecting the type of tag (based on the defined specification) and the tag data.

> Note: The `with_capacity` method can be used to construct a `TagIterator` with a specified default buffer size.  This is only useful as a microoptimization to memory management if you know the maximum tag size of the file you're reading.

The data in the `TagPosition` property can then be modified as desired (encryption, compression, etc.) and reencoded using the `TagWriter` struct. This struct can be created with the `new` function on any source that implements the standard [Write][rust-write] trait. Once created, this struct can encode EBML using the `write` method on any `TagPosition` objects regardless of whether they came from a `TagIterator`.  This will emit binary EBML to the underlying `Write` destination.

## TagPosition Enum

`TagPosition` is an enumeration of three different classifications of tags that this library understands:

  * `StartTag(u64)` is a marker for the beginning of a "master" tag as defined in EBML.  Master tags are simply containers for other tags.  The u64 value is the "id" of the tag.
  * `EndTag(u64)` is a marker for the end of a "master" tag.  The u64 value is the "id" of the tag.
  * `FullTag(id, TagData)` is a complete tag that includes both the id and full data of the tag.  The TagData value is described in more detail below.

## DataTag and DataTagType

```rs
pub enum TagData {
    Master(Vec<(u64, TagData)>),
    UnsignedInt(u64),
    Integer(i64),
    Utf8(String),
    Binary(Vec<u8>),
    Float(f64),
}
```

TagData is an enum containing data stored within a tag.  It is important to note that the type of data contained in the tag directly corresponds to the tag id as defined in whichever specification is in use.  Because EBML is binary, the correct specification is required to parse tag content.  

  * Master(Vec<(u64, TagData)>): A complete master tag containing any number of child tags.
  * UnsignedInt(u64): An unsigned integer.
  * Integer(i64): A signed integer.
  * Utf8(String): A Unicode text string.  Note that the [EBML spec][rfc8794] includes a separate element type for ASCII.  Given that ASCII is a subset of Utf8, this library currently parses and encodes both types using the same Utf8 element.
  * Binary(Vec<u8>): Binary data, otherwise uninterpreted.
  * Float(f64): IEEE-754 floating point number.

> Note: This library made a concious decision to not parse "Date" elements from EBML due to lack of built-in support for dates in Rust. Specification implementations should treat Date elements as Binary so that consumers have the option of parsing the unaltered data using their library of choice, if needed.

# Specification Implementation

Any specification based on EBML can use this library to parse or write binary data.  Writing needs nothing special, but parsing requires a struct implementing the `EbmlSpecification` trait.  This trait currently requires implementation of two methods - `get_tag` and `get_tag_id`.  These are used to convert between specific tag instances and ids.  Custom specification implementations can refer to [webm-iterable][webm-iterable] as an example.


# State of this project

Parsing and writing complete files should both work.  Streaming isn't supported yet, but may be an option in the future. If something is broken, please create [an issue][new-issue].

Any additional feature requests can also be submitted as [an issue][new-issue].

# Author

[Austin Blake](https://github.com/austinleroy)

[EBML]: http://ebml.sourceforge.net/
[webm]: https://www.webmproject.org/
[mkv]: http://www.matroska.org/technical/specs/index.html
[rfc8794]: https://datatracker.ietf.org/doc/rfc8794/
[rust-iterator]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
[rust-read]: https://doc.rust-lang.org/std/io/trait.Read.html
[rust-write]: https://doc.rust-lang.org/std/io/trait.Write.html
[new-issue]: https://github.com/austinleroy/ebml-iterable/issues
[webm-iterable]: https://github.com/austinleroy/webm-iterable
