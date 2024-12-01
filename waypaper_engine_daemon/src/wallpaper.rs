use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;


use waypaper_engine_shared::project::{WallpaperType, WEProject};

use crate::scene_package::ScenePackage;

pub enum Wallpaper {
    Video {
        project: WEProject,
        base_dir_path: PathBuf,
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
        path: PathBuf,
    ) -> Result<Wallpaper, Box<dyn Error>> {
        let project = WEProject::new(
            &path.join("project.json"),
            u64::from_str(path.file_name().unwrap().to_str().unwrap()).unwrap(),
        );

        Ok(match project.wallpaper_type {
            WallpaperType::Video => {
                tracing::debug!("{}", project.file.as_ref().unwrap());

                Wallpaper::Video {
                    base_dir_path: path,
                    project,
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
    
    pub fn wp_type(&self) -> WallpaperType {
        match self {
            Wallpaper::Video { .. } => WallpaperType::Video,
            Wallpaper::Scene { .. } => WallpaperType::Scene,
            Wallpaper::Web { .. } => WallpaperType::Web,
            Wallpaper::Preset { .. } => WallpaperType::Preset,
        }
    }
}
