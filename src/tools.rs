use std::convert::TryInto;

use super::errors::tool::ToolError;

pub trait Vint {
    fn as_vint(&self) -> Result<Vec<u8>, ToolError>;
    fn as_vint_with_length(&self, length: usize) -> Result<Vec<u8>, ToolError>;
}

fn check_size_u64(val: &u64) -> Result<(), ToolError> {
    if *val > (1 << 56) - 2 {
        Err(ToolError::WriteVintOverflow(*val))
    } else {
        Ok(())
    }
}

fn as_vint_no_check_u64(val: &u64, length: usize) -> Vec<u8> {
    let bytes: [u8; 8] = val.to_be_bytes();
    let mut result: Vec<u8> = Vec::from(&bytes[(8-length)..]);
    result[0] |= 1 << (8 - length);
    result
}

impl Vint for u64 {
    fn as_vint(&self) -> Result<Vec<u8>, ToolError> { 
        check_size_u64(&self)?;
        let mut length = 1;
        while length <= 8 {
            if *self < (1 << ((7 * length) - 1)) {
                break;
            }
            length += 1;
        }

        Ok(as_vint_no_check_u64(&self, length))
    }

    fn as_vint_with_length(&self, length: usize) -> Result<Vec<u8>, ToolError> {
        check_size_u64(&self)?;
        Ok(as_vint_no_check_u64(&self, length))
    }
}

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

pub fn arr_to_i64(arr: &[u8]) -> Result<i64, ToolError> {
    if arr.len() > 8 {
        return Err(ToolError::ReadI64Overflow(Vec::from(arr)));
    }

    if arr[0] > 127 {
        if arr.len() == 8 {
            Ok(i64::from_be_bytes(arr.try_into().unwrap()))
        } else {
            Ok(-((1 << (arr.len() * 8)) - (arr_to_u64(arr).unwrap() as i64)))
        }
    } else {
        Ok(arr_to_u64(arr).unwrap() as i64)
    }
}

pub fn arr_to_f64(arr: &[u8]) -> Result<f64, ToolError> {
    if arr.len() == 4 {
        Ok(f32::from_be_bytes(arr.try_into().unwrap()) as f64)
    } else if arr.len() == 8 {
        Ok(f64::from_be_bytes(arr.try_into().unwrap()))
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