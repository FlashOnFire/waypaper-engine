use std::error::Error;
use std::fs::File;
use std::path::Path;
use crate::mpv::MpvRenderer;
use crate::project::{WEProject, WallpaperType};
use crate::scene_package::ScenePackage;
use crate::wallpaper::Wallpaper::Video;
use crate::wl_renderer::WLState;

pub enum Wallpaper {
    Video { project: WEProject, mpv_renderer: MpvRenderer },
    Scene { project: WEProject, scene_package: ScenePackage },
    Web { project: WEProject },
    Preset { project: WEProject },
}

impl Wallpaper {
    pub fn new(state: &WLState, path: &Path) -> Result<Wallpaper, Box<dyn Error>> {
        let project_file = File::open(path.join("project.json"))?;
        let project: WEProject = serde_json::from_reader(project_file)?;

        Ok(match project.wallpaper_type {
            WallpaperType::Video => {
                println!("{}", project.file.as_ref().unwrap());
                let mpv_renderer = MpvRenderer::new(state.connection.clone(), state.egl_state.egl.clone(), path.join(project.file.as_ref().unwrap()));

                Video { project, mpv_renderer }
            }
            WallpaperType::Scene => Wallpaper::Scene { project, scene_package: ScenePackage::new(&path.join("scene.pkg")).unwrap() },
            WallpaperType::Web => Wallpaper::Web { project },
            WallpaperType::Preset => Wallpaper::Preset { project }
        })
    }

    pub(crate) fn clear_color(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    pub(crate) fn render(&mut self, width: u32, height: u32) {
        if let Video { ref mut mpv_renderer, .. } = self {
            mpv_renderer.render(width, height)
        }
    }
}