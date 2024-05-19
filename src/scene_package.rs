use std::{fs, io};
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::io::{Cursor, Read, Seek};
use std::path::{Path, PathBuf};

use crate::file_reading_utils::{read_str, read_u32};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub(crate) name: String,
    offset: u32,
    size: u32,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct ScenePackage {
    pub contents: HashMap<String, FileContent>,
}

impl ScenePackage {
    pub fn new(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Unpacking Scene Package !");

        let mut data: Cursor<Vec<u8>> = Cursor::new(fs::read(path)?);
        println!("Data Length : {}", data.get_ref().len());
        
        let file_count = read_header(&mut data);

        let files = read_files(&mut data, file_count);

        let mut contents: HashMap<String, FileContent> = HashMap::new();
        
        let header_offset = data.position();
        for entry in &files {
            println!("\t{} - {} - {}", entry.name, entry.offset, entry.size);
            contents.insert(entry.name.clone(), read_file(&mut data, header_offset, entry));
        }

        Ok(ScenePackage {
            contents,
        })
    }

    pub fn save_to_disk(&self, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

fn read_header(data: &mut Cursor<Vec<u8>>) -> u32 {
    let version = read_str(data);
    assert_eq!(version, "PKGV0001");

    let file_count = read_u32(data);
    println!("{version} - File count : {file_count}");

    file_count
}

fn read_files(data: &mut Cursor<Vec<u8>>, file_count: u32) -> Vec<FileEntry> {
    let mut files = vec![];

    for _ in 0..file_count {
        files.push(FileEntry {
            name: read_str(data),
            offset: read_u32(data),
            size: read_u32(data),
        })
    }

    files
}

pub fn read_file(data: &mut Cursor<Vec<u8>>, header_offset: u64, file: &FileEntry) -> FileContent {
    data.rewind().unwrap();
    data.set_position(header_offset + file.offset as u64);
    
    let mut content = vec![];
    data.take(file.size as u64).read_to_end(&mut content).unwrap();

    FileContent {
        name: file.name.clone(),
        data: content,
    }
}
