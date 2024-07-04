use std::error::Error;
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use crate::app_state::AppState;

mod app_state;
mod egl;
mod file_reading_utils;
mod mpv;
mod scene;
mod scene_package;
mod tex_file;
mod wallpaper;
mod wl_renderer;

const WPE_DIR: &str = ".steam/steam/steamapps/workshop/content/431960/";

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let wpe_dir = PathBuf::from(std::env::var("HOME").expect("No HOME environment variable set ?")).join(WPE_DIR);
    assert!(wpe_dir.exists() && wpe_dir.is_dir(), "Wallpaper Engine folder not found (tried path: {})", wpe_dir.to_string_lossy());
    
    let mut app = AppState::new(wpe_dir);
    app.run()
}
