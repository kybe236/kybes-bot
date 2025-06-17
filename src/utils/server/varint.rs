use tokio::{
    io::{self, AsyncReadExt},
    net::TcpStream,
};

const SEGMENT_BITS: u32 = 0x7F;
const CONTINUE_BIT: u32 = 0x80;
const SEGMENT_BITS_U8: u8 = 0x7F;
const CONTINUE_BIT_U8: u8 = 0x80;

#[allow(unused)]
pub fn read_var_int_long(var_int: &[u8], offset: Option<&mut usize>) -> i64 {
    let mut value: i64 = 0;
    let mut position = 0;

    let mut binding = 0;
    let offset = offset.unwrap_or(&mut binding);

    while *offset < var_int.len() {
        let current_byte = var_int[*offset];
        value |= ((current_byte & SEGMENT_BITS_U8) as i64) << position;

        if (current_byte & CONTINUE_BIT_U8) == 0 {
            break;
        }

        position += 7;
        *offset += 1;

        if position >= 64 {
            panic!("var_int is too big");
        }
    }

    *offset += 1;
    value
}

pub fn read_var_int(var_int: &[u8], offset: Option<&mut usize>) -> i32 {
    let mut value: i32 = 0;
    let mut position = 0;

    let mut binding = 0;
    let offset = offset.unwrap_or(&mut binding);

    while *offset < var_int.len() {
        let current_byte = var_int[*offset];
        value |= ((current_byte & SEGMENT_BITS_U8) as i32) << position;

        if (current_byte & CONTINUE_BIT_U8) == 0 {
            break;
        }

        position += 7;
        *offset += 1;

        if position >= 32 {
            panic!("var_int is too big");
        }
    }

    *offset += 1;
    value
}

#[allow(unused)]
pub fn write_var_long(result: &mut Vec<u8>, value: i64) {
    let mut value = value as u64;

    loop {
        if (value & !(SEGMENT_BITS as u64)) == 0 {
            result.push(value as u8);
            return;
        }

        result.push(((value & SEGMENT_BITS as u64) | (CONTINUE_BIT as u64)) as u8);
        value >>= 7;
    }
}

pub fn write_var_int(result: &mut Vec<u8>, value: &i32) {
    let mut value = *value as u32;

    loop {
        if (value & !SEGMENT_BITS) == 0 {
            result.push(value as u8);
            return;
        }

        result.push(((value & SEGMENT_BITS) | (CONTINUE_BIT)) as u8);
        value >>= 7;
    }
}

pub async fn read_var_int_from_stream(stream: &mut TcpStream) -> io::Result<i32> {
    let mut num_read = 0;
    let mut value = 0u32;

    loop {
        let byte = stream.read_u8().await?;

        value |= (byte as u32 & 0x7F) << (7 * num_read);
        num_read += 1;

        if byte & 0x80 == 0 {
            break;
        }
    }

    Ok(value as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_varint(value: i32) -> Vec<u8> {
        let mut vec = Vec::new();
        write_var_int(&mut vec, &value);
        vec
    }

    fn encode_varlong(value: i64) -> Vec<u8> {
        let mut vec = Vec::new();
        write_var_long(&mut vec, value);
        vec
    }

    fn decode_varint(bytes: &[u8]) -> i32 {
        read_var_int(bytes, None)
    }

    fn decode_varlong(bytes: &[u8]) -> i64 {
        read_var_int_long(bytes, None)
    }

    #[test]
    fn test_varint_roundtrip() {
        let values = [
            0,
            1,
            2,
            127,
            128,
            255,
            25565,
            2097151,
            2147483647,
            -1,
            -2147483648,
        ];

        for &val in &values {
            let encoded = encode_varint(val);
            let decoded = decode_varint(&encoded);
            assert_eq!(val, decoded, "Failed on value {val}");
        }
    }

    #[test]
    fn test_varlong_roundtrip() {
        let values = [
            0,
            1,
            2,
            127,
            128,
            255,
            2147483647,
            9223372036854775807,
            -1,
            -2147483648,
            -9223372036854775808,
        ];

        for &val in &values {
            let encoded = encode_varlong(val);
            let decoded = decode_varlong(&encoded);
            assert_eq!(val, decoded, "Failed on value {val}");
        }
    }
}
