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
