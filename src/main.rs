use std::error::Error;

use crate::app_state::AppState;

mod wallpaper;
mod project;
mod scene_package;
mod wl_renderer;
mod mpv;
mod egl;
mod scene;
mod tex_file;
mod file_reading_utils;
mod app_state;

const WP_DIR: &str = "/home/flashonfire/.steam/steam/steamapps/workshop/content/431960/";

fn main() -> Result<(), Box<dyn Error>> {
    let mut app = AppState::new();
    app.run()
}
