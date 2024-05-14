use std::collections::HashMap;
use std::{fs, io};
use std::fs::create_dir_all;
use std::path::PathBuf;

#[derive(Debug)]
pub struct FileEntry {
    pub(crate) name: String,
    offset: u32,
    size: u32,
}

pub struct FileContent {
    pub(crate) name: String,
    data: Vec<u8>,
}

impl FileContent {
    pub fn as_str(&self) -> String {
        String::from_utf8_lossy(&self.data).to_string()
    }

    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn save_to_disk(&self, path: &PathBuf) -> io::Result<()> {
        fs::write(path, self.bytes())
    }
}

pub struct ScenePackage {
    pub contents: HashMap<String, FileContent>,
}

impl ScenePackage {
    pub fn from(file: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let data: Vec<u8> = fs::read(file)?;
        let mut position: usize = 0;

        println!("Data Length : {}", data.len());
        let file_count = read_header(&data, &mut position);

        let files = read_files(&data, &mut position, file_count);

        let mut contents: HashMap<String, FileContent> = HashMap::new();
        for entry in &files {
            println!("\t{} - {} - {}", entry.name, entry.offset, entry.size);
            contents.insert(entry.name.clone(), read_file(&data, position, entry));
        }

        Ok(ScenePackage {
            contents,
        })
    }

    pub fn save_to_disk(&self, dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>>  {
        if !dir.try_exists()? {
            create_dir_all(dir)?;
        }
        
        assert!(dir.is_dir());
        
        for c in self.contents.values() {
            let path = &dir.join(c.name.clone());
            
            if let Some(parent_dir) = path.parent() {
                create_dir_all(parent_dir)?;
            }
            
            c.save_to_disk(path)?;
        }
        
        Ok(())
    }
}

fn read_str(data: &[u8], position: &mut usize) -> String {
    let size = read_u32(data, position);

    read_sized_str(data, position, size)
}

fn read_sized_str(data: &[u8], position: &mut usize, size: u32) -> String {
    let bytes: Vec<u8> = data.iter()
        .skip(*position)
        .clone()
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
    let version = read_str(data, position);
    assert_eq!(version, "PKGV0001");

    let file_count = read_u32(data, position);
    println!("PKGV0001 - File count : {file_count}");

    file_count
}

fn read_files(data: &[u8], position: &mut usize, file_count: u32) -> Vec<FileEntry> {
    let mut files = vec![];

    for _ in 0..file_count {
        files.push(FileEntry {
            name: read_str(data, position),
            offset: read_u32(data, position),
            size: read_u32(data, position),
        })
    }

    files
}

pub fn read_file(data: &[u8], header_offset: usize, file: &FileEntry) -> FileContent {
    let content: Vec<_> = data.iter().skip(header_offset + file.offset as usize).take(file.size as usize).copied().collect();

    FileContent {
        name: file.name.clone(),
        data: content,
    }
}
