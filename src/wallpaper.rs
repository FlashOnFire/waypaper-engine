use std::error::Error;
use std::fs::{DirEntry, File};
use crate::project::{Project, WallpaperType};

pub enum Wallpaper {
    Video { project: Project },
    Scene { project: Project },
    Web { project: Project },
}

impl Wallpaper {
    pub fn from(entry: DirEntry) -> Result<Wallpaper, Box<dyn Error>> {
        let aa =File::open(entry.path().join("project.json"))?;

        let project: Project = serde_json::from_reader(aa)?;

        Ok(match project.wallpaper_type {
            WallpaperType::Video => Wallpaper::Video { project },
            WallpaperType::Scene => Wallpaper::Scene { project },
            WallpaperType::Web => Wallpaper::Web { project },
        })

    }
}