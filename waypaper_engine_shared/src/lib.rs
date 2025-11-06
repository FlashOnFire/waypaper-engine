use std::{env, fs};
use std::error::Error;
use std::path::PathBuf;

pub mod ipc;
pub mod project;
pub mod serde_utils;

const WPE_DIR: &str = ".steam/steam/steamapps/workshop/content/431960/";

pub fn get_wpe_dir() -> PathBuf {
    let wpe_dir = PathBuf::from(std::env::var("HOME").expect("No HOME environment variable set ?"))
        .join(WPE_DIR);

    assert!(
        wpe_dir.exists() && wpe_dir.is_dir(),
        "Wallpaper Engine folder not found (tried path: {})",
        wpe_dir.to_string_lossy()
    );

    wpe_dir
}

pub fn save_dir() -> Result<PathBuf, Box<dyn Error>> {
    let base_dir = if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config)
    } else if let Ok(home) = env::var("HOME") {
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
