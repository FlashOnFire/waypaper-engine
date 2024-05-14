mod wallpaper;
mod project;
mod scene;

use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use crate::project::WEProject;
use crate::scene::Scene;
use crate::wallpaper::Wallpaper;

const WP_DIR: &str = "/home/flashonfire/.steam/steam/steamapps/workshop/content/431960/";

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    let mut wallpapers = vec![];

    for entry in fs::read_dir(WP_DIR)?.flatten().enumerate() {
        println!("{0} : {1:?}", entry.0, entry.1.path());

        let mut wp = Wallpaper::from(&entry.1)?;

        let add_id = |proj: &mut WEProject, filename: &OsString| -> Result<(), Box<dyn Error>> {
            if proj.workshop_id.is_none() {
                proj.workshop_id = Some(u64::from_str(filename.to_str().unwrap())?);
            }

            Ok(())
        };

        let filename = entry.1.file_name();

        match wp {
            Wallpaper::Video { ref mut project } => add_id(project, &filename)?,
            Wallpaper::Scene { ref mut project } => add_id(project, &filename)?,
            Wallpaper::Web { ref mut project } => add_id(project, &filename)?,
            Wallpaper::Preset { ref mut project } => add_id(project, &filename)?,
        }

        wallpapers.push(wp);
    };

    let wp = wallpapers.iter()
        .find(|w|
            matches!(w, Wallpaper::Scene { .. })
        ).unwrap();

    if let Wallpaper::Scene { project } = wp {
        println!("{:?}", wp);

        let path = Path::new(WP_DIR).join(project.workshop_id.unwrap().to_string()).join("scene.pkg");

        if path.exists() {
            println!("Found scene.pkg file ! (Path : {:?})", path);

            let scene = Scene::from(&path);
        }
    }


    Ok(())
}
