use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::{env, fs};

pub struct ProfileManager {
    wallpapers: HashMap<String, u64>,
}

impl ProfileManager {
    pub fn new() -> ProfileManager {
        ProfileManager {
            wallpapers: HashMap::new(),
        }
    }

    pub fn save_wallpaper(&mut self, id: u64, screen: &str) {
        let path = save_dir();
        self.wallpapers.insert(screen.to_owned(), id);
        let file = File::create(&path).expect("Unable to create save file");
        serde_json::to_writer_pretty(file, &self.wallpapers).expect("Unable to write save into file");
    }

    pub fn load_wallpaper(&mut self, screen: &str) -> Option<u64> {
        if !self.wallpapers.contains_key(screen) {
            let file_path = save_dir();
            let file = File::open(&file_path).ok()?;
            self.wallpapers = serde_json::from_reader(file).ok()?;
        }
        self.wallpapers.get(screen).copied()
    }
}

fn save_dir() -> PathBuf {
    let base_dir = if let Ok(config) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(config)
    } else {
        PathBuf::from(env::var("HOME").expect("Unable to find directory")).join(".config")
    };

    let parent = base_dir.join("waypaper_engine");
    if !parent.exists() {
        fs::create_dir_all(&parent).expect("Unable to create save directory");
    }

    parent.join("wallpapers.conf")
}
