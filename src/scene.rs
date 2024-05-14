use std::fs;
use std::path::PathBuf;

struct FileEntry {
    name: String,
    offset: u32,
    size: u32,
}

pub struct Scene {}

impl Scene {
    pub fn from(file: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Unpacking...");
        let data: Vec<u8> = fs::read(file)?;
        let mut position: usize = 0;

        println!("Data Length : {}", data.len());
        let file_count = read_header(&data, &mut position);

        Ok(Scene {})
    }
}

fn read_str(data: &[u8], position: &mut usize) -> String {
    let iter = data
        .iter()
        .skip(*position);

    let size = read_u32(data, position);

    let bytes: Vec<u8> = iter
        .clone()
        .skip(4)
        .take(size as usize)
        .copied()
        .collect();
    *position += size as usize;

    String::from_utf8(bytes).unwrap()
}

fn read_u32(data: &[u8], position: &mut usize) -> u32 {
    let first_4_bytes = data
        .iter()
        .skip(*position)
        .clone()
        .take(4)
        .copied()
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();
    *position += 4;

    u32::from_le_bytes(first_4_bytes)
}

fn read_header(data: &[u8], position: &mut usize) -> u32 {
    let version= read_str(data, position);
    assert_eq!(version, "PKGV0001");

    let file_count = read_u32(data, position);
    println!("PKGV0001 - File count : {file_count}");

    file_count
}
