use std::fs;
use std::path::PathBuf;

pub struct Scene {}

impl Scene {
    pub fn from(file: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Unpacking...");
        let data: Vec<u8> = fs::read(file)?;

        println!("{}", data.len());

        read_str(&data, 0);

        Ok(Scene {})
    }
}

fn read_str(data: &[u8], position: usize) -> String {
    let iter = data
        .iter()
        .skip(position);

    let first_4_bytes = iter
        .clone()
        .take(4)
        .copied()
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();

    let size = u32::from_le_bytes(first_4_bytes);
    println!("{}", size);

    let bytes: Vec<u8> = iter
        .clone()
        .skip(4)
        .take(size as usize)
        .copied()
        .collect();

    let string = String::from_utf8(bytes).unwrap();

    println!("{:?}", string);
    string
}
