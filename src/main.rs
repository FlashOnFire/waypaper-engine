mod wallpaper;
mod project;

use std::error::Error;
use std::fs;
use crate::wallpaper::Wallpaper;

const WP_DIR: &str = "/home/flashonfire/.steam/steam/steamapps/workshop/content/431960/";

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    let mut wallpapers = vec![];

    for entry in fs::read_dir(WP_DIR)? {
        if let Ok(entry) = entry {
            println!("{:?}", entry.path());
            wallpapers.push(Wallpaper::from(entry)?)
        }
    };

    Ok(())
}
