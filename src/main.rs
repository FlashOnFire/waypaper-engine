mod wallpaper;
mod project;
mod scene;
mod wl_renderer;
mod list_outputs;
mod mpv;
mod egl;

use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;
use smithay_client_toolkit::reexports::client::Connection;
use crate::egl::EGLState;
use crate::list_outputs::ListOutputs;
use crate::project::WEProject;
use crate::scene::ScenePackage;
use crate::wallpaper::Wallpaper;
use crate::wl_renderer::WLState;

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

            let scene = ScenePackage::from(&path)?;
            scene.save_to_disk(&Path::new("./scene").to_path_buf())?;
        }
    }

    let conn = Rc::new(Connection::connect_to_env().unwrap());

    let mut list_outputs = ListOutputs::new(&conn);
    let outputs = list_outputs.get_outputs();
    outputs.print_outputs();
    let output = outputs.iter().find(|output| output.1.name.as_ref().unwrap() == "DP-3").unwrap();

    /*let wp = wallpapers.iter()
        .filter(|w|
            matches!(w, Wallpaper::Video { .. })
        ).nth(7)
        .unwrap();*/

    let wp = wallpapers.iter().find(|wp| match wp {
        Wallpaper::Video {project} => {
            project.workshop_id.unwrap() == 3212120834
        }
        _ => false
    }).unwrap();

    if let Wallpaper::Video { project } = wp {
        println!("{:?}", wp);

        let path = Path::new(WP_DIR).join(project.workshop_id.unwrap().to_string()).join(project.file.as_ref().unwrap());

        if path.exists() {
            println!("Found video file ! (Path : {:?})", path);

            let egl_state = Rc::new(EGLState::new(&conn));

            let mut state = WLState::new(conn, output, path, egl_state);
            state.loop_fn();
        }
    }
    
    Ok(())
}
