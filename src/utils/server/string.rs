use super::varint::{read_var_int, write_var_int};

pub fn read_string(data: &[u8], mut index: &mut usize) -> Result<String, std::io::Error> {
    let length = read_var_int(data, Some(&mut index)) as usize;

    if *index + length > data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Attempted to read beyond the buffer",
        ));
    }

    let str_bytes = &data[*index..*index + length];

    *index += length;

    let decoded_string = String::from_utf8(str_bytes.to_vec())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    Ok(decoded_string)
}

pub fn write_string(buffer: &mut Vec<u8>, string: &str) {
    let utf16_len = string
        .chars()
        .map(|c| if c > '\u{FFFF}' { 2 } else { 1 })
        .sum::<usize>();

    if utf16_len > 32767 {
        panic!("String is too long for the Minecraft protocol!");
    }
    
    write_var_int(buffer, &(utf16_len as i32));

    buffer.extend_from_slice(string.as_bytes());
}
