use std::error::Error;

use crate::app_state::AppState;

mod app_state;
mod egl;
mod file_reading_utils;
mod mpv;
mod project;
mod scene;
mod scene_package;
mod tex_file;
mod wallpaper;
mod wl_renderer;

const WP_DIR: &str = "/home/flashonfire/.steam/steam/steamapps/workshop/content/431960/";

fn main() -> Result<(), Box<dyn Error>> {
    let mut app = AppState::new();
    app.run()
}
