use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_this_or_that::{as_bool, as_f64, as_i64, as_opt_string, as_opt_u64};

use crate::serde_utils::as_wp_type;
use crate::serde_utils::from_map_str_color;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WEProject {
    #[serde(rename = "type")]
    #[serde(deserialize_with = "as_wp_type")]
    #[serde(default)]
    pub wallpaper_type: WallpaperType,

    #[serde(flatten)]
    #[serde(default)]
    pub preset: Option<Preset>,

    pub approved: Option<bool>,

    #[serde(rename = "lowercase")]
    pub content_rating: Option<String>,
    pub description: Option<String>,

    #[serde(default)]
    pub file: Option<String>,

    #[serde(default)]
    pub general: Option<General>,
    pub preview: String,
    pub tags: Option<Vec<String>>,
    pub title: String,

    pub visibility: Option<String>,

    #[serde(default)]
    pub official: bool,

    #[serde(deserialize_with = "as_opt_u64")]
    #[serde(default)]
    #[serde(rename = "workshopid")]
    pub workshop_id: Option<u64>,

    #[serde(default)]
    #[serde(rename = "workshopurl")]
    pub workshop_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct General {
    pub properties: HashMap<String, Property>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    #[serde(default)]
    pub order: i64,
    pub text: String,
    pub index: Option<i64>,

    #[serde(flatten)]
    pub value: PropertyValue,
    condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PropertyValue {
    #[serde(deserialize_with = "from_map_str_color")]
    Color {
        r: f64,
        g: f64,
        b: f64,
    },
    Slider {
        min: f64,
        max: f64,
        precision: Option<f64>,
        step: Option<f64>,

        #[serde(deserialize_with = "as_f64")]
        value: f64,
    },
    Combo {
        options: Vec<ComboOption>,

        #[serde(deserialize_with = "as_i64")]
        value: i64,
    },
    Bool {
        #[serde(deserialize_with = "as_bool")]
        value: bool,
    },
    #[serde(alias = "textinput")]
    TextInput {
        value: String,
    },
    Text {},
    File {
        #[serde(default)]
        value: Option<String>,
    },
    Directory {
        #[serde(flatten)]
        mode: DirMode,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboOption {
    #[serde(deserialize_with = "as_opt_string")]
    value: Option<String>,
    label: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WallpaperType {
    #[default]
    Preset,
    #[serde(alias = "Video")]
    Video,
    #[serde(alias = "Scene")]
    Scene,
    Web,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum DirMode {
    OnDemand,
    FetchAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    preset: HashMap<String, Option<PresetValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PresetValue {
    #[serde(deserialize_with = "from_map_str_color")]
    Color {
        r: f64,
        g: f64,
        b: f64,
    },
    Bool {
        value: bool,
    },
    Integer {
        value: i64,
    },
}

impl WEProject {
    pub fn new(path: &Path, id: u64) -> Self {
        let project_file = File::open(path).unwrap();
        let mut proj: WEProject = serde_json::from_reader(project_file).unwrap();

        if proj.workshop_id.is_none() {
            proj.workshop_id = Some(id);
        }

        proj
    }
}
