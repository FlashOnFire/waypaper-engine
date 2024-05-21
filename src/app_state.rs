use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;

use crate::{tex_file, WP_DIR};
use crate::project::WEProject;
use crate::wallpaper::Wallpaper;
use crate::wl_renderer::RenderingContext;

pub struct AppState {
    rendering_context: RenderingContext,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            rendering_context: RenderingContext::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        //tex_file::TexFile::new(Path::new("/home/flashonfire/RustroverProjects/waypaper-engine/tests/scene/materials/wallhaven-543465.tex")).unwrap();
        
        let path = Path::new(WP_DIR).join("1195491399");

        let mut wallpaper = Wallpaper::new(self.rendering_context.connection.clone(), &self.rendering_context.egl_state, &path).unwrap();
        let filename = path.file_name().unwrap();

        match wallpaper {
            Wallpaper::Video { ref mut project, .. } => add_id(project, filename)?,
            Wallpaper::Scene { ref mut project, .. } => add_id(project, filename)?,
            Wallpaper::Web { ref mut project } => add_id(project, filename)?,
            Wallpaper::Preset { ref mut project } => add_id(project, filename)?,
        }

        let outputs = self.rendering_context.get_outputs();
        outputs.print_outputs();
        let output = outputs.iter().find(|output| output.1.name.as_ref().unwrap() == "DP-3").unwrap();

        if let Wallpaper::Video { ref project, .. } = wallpaper {
            let path = Path::new(WP_DIR).join(project.workshop_id.unwrap().to_string()).join(project.file.as_ref().unwrap());

            if path.exists() {
                println!("Found video file ! (Path : {path:?})");

                self.rendering_context.set_wallpaper(output, wallpaper);

                self.rendering_context.loop_fn();
            }
        }

        Ok(())
    }
}

fn add_id(proj: &mut WEProject, filename: &OsStr) -> Result<(), Box<dyn Error>> {
    if proj.workshop_id.is_none() {
        proj.workshop_id = Some(u64::from_str(filename.to_str().unwrap())?);
    }

    Ok(())
}
