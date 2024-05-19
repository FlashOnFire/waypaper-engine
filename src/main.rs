use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;

use smithay_client_toolkit::output::OutputInfo;
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;

use crate::project::WEProject;
use crate::scene_package::ScenePackage;
use crate::wallpaper::Wallpaper;
use crate::wl_renderer::{SimpleLayer, WLState};

mod wallpaper;
mod project;
mod scene_package;
mod wl_renderer;
mod mpv;
mod egl;
mod scene_renderer;
mod scene;
mod tex_file;
mod file_reading_utils;

const WP_DIR: &str = "/home/flashonfire/.steam/steam/steamapps/workshop/content/431960/";

fn main() -> Result<(), Box<dyn Error>> {
    tex_file::TexFile::new(Path::new("/home/flashonfire/RustroverProjects/waypaper-engine/scene/materials/wallhaven-543465.tex")).unwrap();
    
    
    /*let mut state = WLState::new();
    
    let path = Path::new(WP_DIR).join("1195491399");

    let mut wallpaper = Wallpaper::new(&state, &path).unwrap();
    let filename = path.file_name().unwrap();

    match wallpaper {
        Wallpaper::Video { ref mut project, .. } => add_id(project, filename)?,
        Wallpaper::Scene { ref mut project, .. } => add_id(project, filename)?,
        Wallpaper::Web { ref mut project } => add_id(project, filename)?,
        Wallpaper::Preset { ref mut project } => add_id(project, filename)?,
    }
    
    let outputs = state.get_outputs();
    outputs.print_outputs();
    let output = outputs.iter().find(|output| output.1.name.as_ref().unwrap() == "DP-3").unwrap();

    if let Wallpaper::Video { ref project, .. } = wallpaper {
        let path = Path::new(WP_DIR).join(project.workshop_id.unwrap().to_string()).join(project.file.as_ref().unwrap());

        if path.exists() {
            println!("Found video file ! (Path : {:?})", path);

            set_wallpaper(&mut state, output, wallpaper);

            state.loop_fn();
        }
    }*/

    Ok(())
}

fn set_wallpaper(wl_state: &mut WLState, output: (&WlOutput, &OutputInfo), wallpaper: Wallpaper) {
    let output_name = output.1.name.as_ref().unwrap();

    let layer: &mut SimpleLayer = if !wl_state.layers.contains_key(&output_name.clone()) {
        wl_state.setup_layer(output)
    } else {
        wl_state.layers.get_mut(output_name).unwrap()
    };

    layer.wallpaper = Some(wallpaper);
}

fn add_id(proj: &mut WEProject, filename: &OsStr) -> Result<(), Box<dyn Error>> {
    if proj.workshop_id.is_none() {
        proj.workshop_id = Some(u64::from_str(filename.to_str().unwrap())?);
    }

    Ok(())
}
