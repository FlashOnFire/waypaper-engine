use std::collections::HashMap;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::path::PathBuf;
use waypaper_engine_shared::save_dir;

pub struct ProfileManager {
}

impl ProfileManager {
    pub fn save_wallpaper(id: u64, screen: &str) -> Result<(), Box<dyn Error>> {
        let path: PathBuf = save_dir()?;

        let mut lines: HashMap<String, u64> = if path.exists() {
            let file = File::open(&path)?;
            serde_json::from_reader(BufReader::new(file)).unwrap_or_default()
        } else {
            HashMap::new()
        };

        lines.insert(screen.to_string(), id);

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;

        serde_json::to_writer_pretty(file, &lines)?;

        Ok(())
    }

    pub fn load_wallpaper(screen: &String) -> Result<u64, ()> {
        let mut wallpapers: HashMap<String, u64> = HashMap::new();

        if let Ok(file_path) = save_dir() && let Ok(file) = File::open(&file_path) {
            wallpapers = serde_json::from_reader(file).unwrap_or_default();
        }

        if let Some(&id) = wallpapers.get(screen) {
            return Ok(id)
        }

        Err(())
    }
}
