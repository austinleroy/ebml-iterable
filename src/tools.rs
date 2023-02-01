//! 
//! Contains a number of tools that are useful when working with EBML encoded files.
//! 

use std::convert::TryInto;

use super::errors::tool::ToolError;

///
/// Trait to enable easy serialization to a vint.
/// 
/// This is only available for types that can be cast as `u64`.
/// 
pub trait Vint: Into<u64> + Copy {
    ///
    /// Returns a representation of the current value as a vint array.
    /// 
    /// # Errors
    ///
    /// This can return an error if the value is too large to be representable as a vint.
    /// 
    fn as_vint(&self) -> Result<Vec<u8>, ToolError> {
        let val: u64 = (*self).into();
        check_size_u64(val)?;
        let mut length = 1;
        while length <= 8 {
            if val < (1 << (7 * length)) {
                break;
            }
            length += 1;
        }

        Ok(as_vint_no_check_u64(val, length))
    }

    ///
    /// Returns a representation of the current value as a vint array with a specified length.
    /// 
    /// # Errors
    ///
    /// This can return an error if the value is too large to be representable as a vint.
    /// 
    fn as_vint_with_length(&self, length: usize) -> Result<Vec<u8>, ToolError> {
        let val: u64 = (*self).into();
        check_size_u64(val)?;
        Ok(as_vint_no_check_u64(val, length))
    }
}

impl Vint for u64 { }
impl Vint for u32 { }
impl Vint for u16 { }
impl Vint for u8 { }

fn check_size_u64(val: u64) -> Result<(), ToolError> {
    if val > (1 << 56) - 1 {
        Err(ToolError::WriteVintOverflow(val))
    } else {
        Ok(())
    }
}

fn as_vint_no_check_u64(val: u64, length: usize) -> Vec<u8> {
    let bytes: [u8; 8] = val.to_be_bytes();
    let mut result: Vec<u8> = Vec::from(&bytes[(8-length)..]);
    result[0] |= 1 << (8 - length);
    result
}

/// 
/// Reads a vint from the beginning of the input array slice.
/// 
/// This method returns an option with the `None` variant used to indicate there was not enough data in the buffer to completely read a vint.
/// 
/// The returned tuple contains the value of the vint (`u64`) and the length of the vint (`usize`).  The length will be less than or equal to the length of the input slice.
/// 
/// # Errors
///
/// This method can return a `ToolError` if the input array cannot be read as a vint.
/// 
pub fn read_vint(buffer: &[u8]) -> Result<Option<(u64, usize)>, ToolError> {
    if buffer.is_empty() {
        return Ok(None);
    }

    let length: usize = buffer[0].leading_zeros() as usize + 1;
    if length > 8 {
        return Err(ToolError::ReadVintOverflow)
    }

    if length > buffer.len() {
        // Not enough data in the buffer to read out the vint value
        return Ok(None);
    }

    let mut value = buffer[0] as u64;
    value -= 1 << (8 - length);

    for item in buffer.iter().take(length).skip(1) {
        value <<= 8;
        value += *item as u64;
    }

    Ok(Some((value, length)))
}

///
/// Reads a `u64` value from any length array slice.
/// 
/// Rather than forcing the input to be a `[u8; 8]` like standard library methods, this can interpret a `u64` from a slice of any length < 8.  Bytes are assumed to be least significant when reading the value - i.e. an array of `[4, 0]` would return a value of `1024`.  
///
/// # Errors
///
/// This method will return an error if the input slice has a length > 8.
/// 
/// ## Example
/// 
/// ```
/// # use ebml_iterable::tools::arr_to_u64;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let result = arr_to_u64(&[16,0])?;
/// assert_eq!(result, 4096);
/// # Ok(())
/// # }
/// ```
/// 
pub fn arr_to_u64(arr: &[u8]) -> Result<u64, ToolError> {
    if arr.len() > 8 {
        return Err(ToolError::ReadU64Overflow(Vec::from(arr)));
    }

    let mut val = 0u64;
    for byte in arr {
        val *= 256;
        val += *byte as u64;
    }
    Ok(val)
}

///
/// Reads an `i64` value from any length array slice.
/// 
/// Rather than forcing the input to be a `[u8; 8]` like standard library methods, this can interpret an `i64` from a slice of any length < 8.  Bytes are assumed to be least significant when reading the value - i.e. an array of `[4, 0]` would return a value of `1024`.  
///
/// # Errors
///
/// This method will return an error if the input slice has a length > 8.
/// 
/// ## Example
/// 
/// ```
/// # use ebml_iterable::tools::arr_to_i64;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let result = arr_to_i64(&[4,0])?;
/// assert_eq!(result, 1024);
/// # Ok(())
/// # }
/// ```
///
pub fn arr_to_i64(arr: &[u8]) -> Result<i64, ToolError> {
    if arr.len() > 8 {
        return Err(ToolError::ReadI64Overflow(Vec::from(arr)));
    }

    if arr[0] > 127 {
        if arr.len() == 8 {
            Ok(i64::from_be_bytes(arr.try_into().expect("[u8;8] should be convertible to i64")))
        } else {
            Ok(-((1 << (arr.len() * 8)) - (arr_to_u64(arr).expect("arr_to_u64 shouldn't error if length is <= 8") as i64)))
        }
    } else {
        Ok(arr_to_u64(arr).expect("arr_to_u64 shouldn't error if length is <= 8") as i64)
    }
}

///
/// Reads an `f64` value from an array slice of length 4 or 8.
/// 
/// This method wraps `f32` and `f64` conversions from big endian byte arrays and casts the result as an `f64`.  
///
/// # Errors
///
/// This method will throw an error if the input slice length is not 4 or 8.
/// 
pub fn arr_to_f64(arr: &[u8]) -> Result<f64, ToolError> {
    if arr.len() == 4 {
        Ok(f32::from_be_bytes(arr.try_into().expect("arr should be [u8;4]")) as f64)
    } else if arr.len() == 8 {
        Ok(f64::from_be_bytes(arr.try_into().expect("arr should be [u8;8]")))
    } else {
        Err(ToolError::ReadF64Mismatch(Vec::from(arr)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_vint_sixteen() {
        let buffer = [144];
        let result = read_vint(&buffer).unwrap().expect("Reading vint failed");

        assert_eq!(16, result.0);
        assert_eq!(1, result.1);
    }

    #[test]
    fn write_vint_sixteen() {
        let result = 16u64.as_vint().expect("Writing vint failed");
        assert_eq!(vec![144u8], result);
    }

    #[test]
    fn read_vint_one_twenty_seven() {
        let buffer = [255u8];
        let result = read_vint(&buffer).unwrap().expect("Reading vint failed");

        assert_eq!(127, result.0);
        assert_eq!(1, result.1);
    }

    #[test]
    fn write_vint_one_twenty_seven() {
        let result = 127u64.as_vint().expect("Writing vint failed");
        assert_eq!(vec![255u8], result);
    }

    #[test]
    fn read_vint_two_hundred() {
        let buffer = [64, 200];
        let result = read_vint(&buffer).unwrap().expect("Reading vint failed");

        assert_eq!(200, result.0);
        assert_eq!(2, result.1);
    }

    #[test]
    fn write_vint_two_hundred() {
        let result = 200u64.as_vint().expect("Writing vint failed");
        assert_eq!(vec![64u8, 200u8], result);
    }

    #[test]
    fn read_vint_for_ebml_tag() {
        let buffer = [0x1a, 0x45, 0xdf, 0xa3];
        let result = read_vint(&buffer).unwrap().expect("Reading vint failed");

        assert_eq!(0x0a45dfa3, result.0);
        assert_eq!(4, result.1);
    }

    #[test]
    fn read_vint_very_long() {
        let buffer = [1, 0, 0, 0, 0, 0, 0, 1];
        let result = read_vint(&buffer).unwrap().expect("Reading vint failed");

        assert_eq!(1, result.0);
        assert_eq!(8, result.1);
    }

    #[test]
    fn write_vint_very_long() {
        let result = 1u64.as_vint_with_length(8).expect("Writing vint failed");
        assert_eq!(vec![1, 0, 0, 0, 0, 0, 0, 1], result);
    }

    #[test]
    fn read_vint_overflow() {
        let buffer = [1, 0, 0, 0];
        let result = read_vint(&buffer).expect("Reading vint failed");

        assert_eq!(true, result.is_none());
    }

    #[test]
    #[should_panic]
    fn too_big_for_vint() {
        (1u64 << 56).as_vint().expect("Writing vint failed");
    }

    #[test]
    fn read_u64_values() {
        let mut buffer = vec![];
        let mut expected = 0;
        for _ in 0..8 {
            buffer.push(0x25);
            expected = (expected << 8) + 0x25;

            let result = arr_to_u64(&buffer).unwrap();
            assert_eq!(expected, result);
        }
    }

    #[test]
    fn read_i64_values() {
        let mut buffer = vec![];
        let mut expected = 0;
        for _ in 0..8 {
            buffer.push(0x0a);
            expected = (expected << 8) + 0x0a;

            let result = arr_to_i64(&buffer).unwrap();
            assert_eq!(expected, result);

            let neg_result = arr_to_i64(&(buffer.iter().map(|b| !b).collect::<Vec<u8>>())).unwrap() + 1;
            assert_eq!(-expected, neg_result);
        }
    }
}