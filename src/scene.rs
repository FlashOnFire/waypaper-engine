use std::fs;
use std::path::PathBuf;

pub struct Scene {}

impl Scene {
    pub fn from(file: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Unpacking...");
        let data: Vec<u8> = fs::read(file)?;

        println!("Data Length : {}", data.len());

        read_header(&data, 0);

        Ok(Scene {})
    }
}

fn read_str(data: &[u8], position: usize) -> (String, usize) {
    let iter = data
        .iter()
        .skip(position);

    let (size, position) = read_u32(data, position);

    let bytes: Vec<u8> = iter
        .clone()
        .skip(4)
        .take(size as usize)
        .copied()
        .collect();
    
    let string = String::from_utf8(bytes).unwrap();
    (string, position + size as usize)
}

fn read_u32(data: &[u8], position: usize) -> (u32, usize) {
    let first_4_bytes = data
        .iter()
        .skip(position)
        .clone()
        .take(4)
        .copied()
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();

    let size = u32::from_le_bytes(first_4_bytes);

    (size, position+4)
}

fn read_header(data: &[u8], position: usize) -> (u32, usize) {
    let (version, position) = read_str(data, position);
    assert_eq!(version, "PKGV0001");

    let (file_count, position) = read_u32(data, position);
    println!("PKGV0001 - File count : {file_count}");

    (file_count, position)
}