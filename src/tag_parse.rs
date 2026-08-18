use crate::errors::tag_iterator::{CorruptedFileError, TagIteratorError};
use crate::errors::tool::ToolError;
use crate::specs::{EbmlSpecification, EbmlTag, TagDataType};
use crate::tag_iterator_util::EBMLSize;
use crate::tools;

/// A parsed EBML tag header.
///
/// This contains the parsed element identifier, the spec-inferred data type
/// (if available), the element data size as an [`EBMLSize`], and the total
/// header length in bytes (identifier + size descriptor).
///
/// The header is produced by [`read_header`]. The `data_type` field will be
/// `None` for unknown tag ids (when the specification does not include the
/// id). Consumers should treat that as an indicator that the element is a
/// raw/unknown element unless their validation policy allows unknown ids.
pub(crate) struct TagHeader {
    pub id: u64,
    pub data_type: Option<TagDataType>,
    pub size: EBMLSize,
    pub len: usize,
}

/// Attempts to parse an EBML tag identifier from the front of `input`.
///
/// Returns `Some((id, id_len))` when a complete identifier is available, where
/// `id` is the numeric element id and `id_len` is the number of bytes consumed
/// by the identifier. Returns `None` when `input` does not contain enough
/// bytes to form a complete identifier.
pub(crate) fn read_tag_id(input: &[u8]) -> Option<(u64, usize)> {
    let first = input.first().copied()?;
    let id_len = if first == 0 { 1 } else { 8 - first.ilog2() as usize };
    if input.len() < id_len {
        return None;
    }

    let id = input[..id_len]
        .iter()
        .fold(0u64, |value, byte| (value << 8) + u64::from(*byte));
    Some((id, id_len))
}

/// Attempts to read a full EBML tag header from the start of `input`.
///
/// This returns `Ok(None)` when there is not yet enough data in `input` to
/// parse a complete header (identifier + size). When a header is available it
/// returns `Ok(Some(TagHeader))`. Parsing errors that indicate corrupted file
/// data are returned as `Err(TagIteratorError::CorruptedFileData(...))`.
///
/// `position` is the logical byte offset of `input` within the source stream
/// and is used in produced error variants for better diagnostics.
pub(crate) fn read_header<TSpec>(input: &[u8], position: usize) -> Result<Option<TagHeader>, TagIteratorError>
where
    TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone,
{
    let Some((id, id_len)) = read_tag_id(input) else {
        return Ok(None);
    };

    let Some((size, size_len)) = tools::read_vint(&input[id_len..]).map_err(|_| {
        TagIteratorError::CorruptedFileData(CorruptedFileError::InvalidTagData { tag_id: id, position })
    })?
    else {
        return Ok(None);
    };
    let data_type = TSpec::get_tag_data_type(id);

    // Numeric types may be at most 8 bytes; reject larger sizes as corrupted.
    if matches!(
        data_type,
        Some(TagDataType::UnsignedInt | TagDataType::Integer | TagDataType::Float)
    ) && size > 8
    {
        return Err(TagIteratorError::CorruptedFileData(
            CorruptedFileError::InvalidTagData { tag_id: id, position },
        ));
    }

    Ok(Some(TagHeader {
        id,
        data_type,
        size: EBMLSize::new(size, size_len),
        len: id_len + size_len,
    }))
}

/// Converts raw element bytes into a typed tag variant as defined by `TSpec`.
///
/// `id` is the element id, `data_type` is the optional spec-provided data type
/// (as returned by [`TSpec::get_tag_data_type`]) and `raw_data` is the element
/// payload bytes (not including the header). If `data_type` is `None` the
/// method returns a `RawTag` produced by the specification implementation.
///
/// Errors are returned when the raw bytes cannot be interpreted as the
/// expected type (for example invalid UTF-8 for `Utf8` or wrong length for
/// numeric types). These are returned as [`TagIteratorError::CorruptedTagData`].
///
/// # Panics
///
/// This function will panic if the specification implementation claims a
/// particular `id` maps to a data type but fails to construct a tag variant
/// for that id (i.e. the spec is internally inconsistent). 
pub(crate) fn read_data_tag<TSpec>(
    id: u64,
    data_type: Option<TagDataType>,
    raw_data: &[u8],
) -> Result<TSpec, TagIteratorError>
where
    TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone,
{
    let tag = match data_type {
        Some(TagDataType::Master) => unreachable!(),
        Some(TagDataType::UnsignedInt) => {
            let value = if raw_data.is_empty() {
                0
            } else {
                tools::arr_to_u64(raw_data)
                    .map_err(|problem| TagIteratorError::CorruptedTagData { tag_id: id, problem })?
            };
            TSpec::get_unsigned_int_tag(id, value)
        }
        Some(TagDataType::Integer) => {
            let value = if raw_data.is_empty() {
                0
            } else {
                tools::arr_to_i64(raw_data)
                    .map_err(|problem| TagIteratorError::CorruptedTagData { tag_id: id, problem })?
            };
            TSpec::get_signed_int_tag(id, value)
        }
        Some(TagDataType::Utf8) => {
            let value = String::from_utf8(raw_data.to_vec()).map_err(|error| TagIteratorError::CorruptedTagData {
                tag_id: id,
                problem: ToolError::FromUtf8Error(raw_data.to_vec(), error),
            })?;
            TSpec::get_utf8_tag(id, value)
        }
        Some(TagDataType::Binary) => TSpec::get_binary_tag(id, raw_data),
        Some(TagDataType::Float) => {
            let value = if raw_data.is_empty() {
                0.0
            } else {
                tools::arr_to_f64(raw_data)
                    .map_err(|problem| TagIteratorError::CorruptedTagData { tag_id: id, problem })?
            };
            TSpec::get_float_tag(id, value)
        }
        None => return Ok(TSpec::get_raw_tag(id, raw_data)),
    }
    .unwrap_or_else(|| {
        panic!(
            "Bad specification implementation: Tag id 0x{:x?} had an incompatible data type!",
            id
        )
    });

    Ok(tag)
}
