use std::io::{BufRead, Cursor, Read};

pub fn read_u32(cursor: &mut Cursor<Vec<u8>>) -> u32 {
    let mut first_4_bytes: [u8; 4] = [0; 4];
    cursor.read_exact(&mut first_4_bytes).unwrap();

    u32::from_le_bytes(first_4_bytes)
}

pub fn read_null_terminated_str(cursor: &mut Cursor<Vec<u8>>) -> String {
    let mut bytes = vec![];

    cursor.read_until(0x00, &mut bytes).unwrap();
    bytes.pop(); // Remove the null terminator

    String::from_utf8(bytes).unwrap()
}

pub fn read_str(data: &mut Cursor<Vec<u8>>) -> String {
    let size = read_u32(data);
    read_sized_str(data, size)
}

fn read_sized_str(data: &mut Cursor<Vec<u8>>, size: u32) -> String {
    let mut bytes = vec![];
    data.take(size as u64).read_to_end(&mut bytes).unwrap();

    String::from_utf8(bytes).unwrap()
}
