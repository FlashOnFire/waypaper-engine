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

    pub fn save_wallpaper(&mut self, id: u64, screen: &str) -> Result<(), String> {
        let path = save_dir()?;
        self.wallpapers.insert(screen.to_owned(), id);

        match File::create(&path) {
            Ok(file) => match serde_json::to_writer_pretty(file, &self.wallpapers) {
                Ok(_) => Ok(()),
                Err(e) => Err(e.to_string()),
            },
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn load_wallpaper(&mut self, screen: &str) -> Option<u64> {
        if !self.wallpapers.contains_key(screen) {
            let file_path = save_dir().ok()?;
            let file = File::open(&file_path).ok()?;
            self.wallpapers = serde_json::from_reader(file).ok()?;
        }
        self.wallpapers.get(screen).copied()
    }
}

fn save_dir() -> Result<PathBuf, String> {
    let base_dir = if let Ok(config) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(config)
    } else if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return Err("Unable to find directory".to_string())
    };

    let parent = base_dir.join("waypaper_engine");
    if !parent.exists() && fs::create_dir_all(&parent).is_err() {
        return Err("Unable to create save directory".to_string());
    }

    let file_path = parent.join("wallpapers.conf");
    Ok(file_path)
}
