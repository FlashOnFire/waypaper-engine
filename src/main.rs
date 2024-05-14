mod wallpaper;
mod project;

use std::error::Error;
use std::fs;
use crate::wallpaper::Wallpaper;

const WP_DIR: &str = "/home/flashonfire/.steam/steam/steamapps/workshop/content/431960/";

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    let mut wallpapers = vec![];

    for entry in (fs::read_dir(WP_DIR)?).flatten().enumerate() {
        println!("{0} : {1:?}", entry.0, entry.1.path());
        wallpapers.push(Wallpaper::from(entry.1)?)
    };

    Ok(())
}
