use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::{env, fs};

pub struct ProfileManager {}

impl ProfileManager {
    pub fn save_wallpaper(id: u64, screen: &str) -> Result<(), Box<dyn Error>> {
        let path = save_dir()?;

        let mut lines: HashMap<String, u64> = File::open(&path)
            .ok()
            .and_then(|file| serde_json::from_reader(BufReader::new(file)).ok())
            .unwrap_or_default();

        lines.insert(screen.to_owned(), id);

        let file = File::create(&path)?;
        serde_json::to_writer_pretty(file, &lines)?;

        Ok(())
    }

    pub fn load_wallpaper(screen: &String) -> Result<u64, ()> {
        let mut wallpapers: HashMap<String, u64> = HashMap::new();

        if let Ok(file_path) = save_dir()
            && let Ok(file) = File::open(&file_path)
        {
            wallpapers = serde_json::from_reader(file).unwrap_or_default();
        }

        if let Some(&id) = wallpapers.get(screen) {
            return Ok(id);
        }

        Err(())
    }
}

pub fn save_dir() -> Result<PathBuf, Box<dyn Error>> {
    let base_dir = if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Impossible de déterminer le répertoire de configuration",
        )));
    };

    let parent = base_dir.join("waypaper_engine");
    if !parent.exists() {
        fs::create_dir_all(&parent)?;
    }

    let file_path = parent.join("wallpapers.conf");
    Ok(file_path)
}
