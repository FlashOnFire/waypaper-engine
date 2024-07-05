use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;

use smithay_client_toolkit::reexports::client::Connection;

use waypaper_engine_shared::project::{WallpaperType, WEProject};

use crate::egl::EGLState;
use crate::mpv::MpvRenderer;
use crate::scene_package::ScenePackage;

pub enum Wallpaper {
    Video {
        project: WEProject,
        mpv_renderer: MpvRenderer,
    },
    Scene {
        project: WEProject,
        scene_package: ScenePackage,
    },
    Web {
        project: WEProject,
    },
    Preset {
        project: WEProject,
    },
}

impl Wallpaper {
    pub fn new(
        connection: Rc<Connection>,
        egl_state: &Rc<EGLState>,
        path: &Path,
    ) -> Result<Wallpaper, Box<dyn Error>> {
        let project_file = File::open(path.join("project.json"))?;
        let project: WEProject = serde_json::from_reader(project_file)?;

        Ok(match project.wallpaper_type {
            WallpaperType::Video => {
                tracing::debug!("{}", project.file.as_ref().unwrap());
                let mpv_renderer = MpvRenderer::new(
                    connection,
                    egl_state.egl.clone(),
                    path.join(project.file.as_ref().unwrap()),
                );

                Wallpaper::Video {
                    project,
                    mpv_renderer,
                }
            }
            WallpaperType::Scene => {
                let scene_pkg_path = path.join("scene.pkg");
                let scene_package = ScenePackage::new(&scene_pkg_path).unwrap();

                Wallpaper::Scene {
                    project,
                    scene_package,
                }
            }
            WallpaperType::Web => Wallpaper::Web { project },
            WallpaperType::Preset => Wallpaper::Preset { project },
        })
    }

    pub(crate) fn init_render(&mut self) {
        match self {
            Wallpaper::Video { mpv_renderer, .. } => mpv_renderer.init_rendering_context(),
            Wallpaper::Scene { .. } => todo!(),
            Wallpaper::Web { .. } => todo!(),
            Wallpaper::Preset { .. } => todo!(),
        }
    }

    pub(crate) fn clear_color(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    pub(crate) fn render(&mut self, width: u32, height: u32) {
        if let Wallpaper::Video {
            ref mut mpv_renderer,
            ..
        } = self
        {
            mpv_renderer.render(width, height)
        }
    }
}
