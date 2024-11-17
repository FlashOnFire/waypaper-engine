#![feature(let_chains)]

use std::error::Error;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use crate::app_state::AppState;

mod app_state;
mod egl;
mod file_reading_utils;
mod scene;
mod scene_package;
mod tex_file;
mod mpv_video_wp_renderer;
mod wallpaper;
mod wallpaper_renderer;
mod wl_renderer;

mod gl_utils;
mod video_rs_wp_renderer;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let mut app = AppState::new(waypaper_engine_shared::get_wpe_dir());
    app.run()
}
