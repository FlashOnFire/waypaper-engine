use std::collections::HashMap;
use std::str::FromStr;
use serde::{Deserialize, Deserializer};
use serde::de::{Error, Unexpected};
use serde_derive::Deserialize;
use serde_derive::Serialize;
use serde_this_or_that::{as_bool, as_f64, as_i64};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WEProject {
    #[serde(default)]
    #[serde(rename = "type")]
    #[serde(flatten)]
    #[serde(deserialize_with = "as_wp_type")]
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

    #[serde(deserialize_with = "as_u64_opt")]
    #[serde(default)]
    #[serde(rename = "lowercase")]
    pub workshop_id: Option<u64>,

    #[serde(default)]
    #[serde(rename = "lowercase")]
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
    pub order: i32,
    pub text: String,
    pub index: Option<i32>,

    #[serde(flatten)]
    pub value: PropertyValue,
    condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PropertyValue {
    #[serde(deserialize_with = "from_str_color")]
    Color {
        r: f32,
        g: f32,
        b: f32,
    },
    Slider {
        min: f32,
        max: f32,
        precision: Option<f32>,
        step: Option<f32>,

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
        value: bool
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
    #[serde(deserialize_with = "as_str_opt")]
    value: Option<String>,
    label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WallpaperType {
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
    #[serde(deserialize_with = "from_str_color")]
    Color {
        r: f32,
        g: f32,
        b: f32,
    },
    Bool {
        value: bool,
    },
    Integer {
        value: i32,
    }
}

fn from_str_color<'de, D>(deserializer: D) -> Result<(f32, f32, f32), D::Error>
    where D: Deserializer<'de>,
{
    let map: HashMap<String, String> = Deserialize::deserialize(deserializer)?;
    let s = map.get("value").ok_or(D::Error::missing_field("value"))?;

    let parts = s.split(' ')
        .map(f32::from_str)
        .map(|f|
            f.map_err(
                D::Error::custom
            ))
        .collect::<Result<Vec<_>, _>>()?;

    if parts.len() == 3 {
        Ok((
            parts[0],
            parts[1],
            parts[1]
        ))
    } else {
        Err(D::Error::invalid_length(parts.len(), &"3 floats"))
    }
}

fn as_str_opt<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where D: Deserializer<'de>,
{
    match serde_this_or_that::as_string(deserializer) {
        Ok(s) => { Ok(Some(s)) }
        Err(_) => { Ok(None) }
    }
}

fn as_u64_opt<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where D: Deserializer<'de>,
{
    match serde_this_or_that::as_u64(deserializer) {
        Ok(u) => { Ok(Some(u)) }
        Err(_) => { Ok(None) }
    }
}

fn as_wp_type<'de, D>(deserializer: D) -> Result<WallpaperType, D::Error>
    where D: Deserializer<'de>,
{
    let string: Result<String, _> = Deserialize::deserialize(deserializer);
    Ok(match string {
        Ok(str) => {
            match str.to_lowercase().as_str() {
                "video" => WallpaperType::Video,
                "scene" => WallpaperType::Scene,
                "web" => WallpaperType::Web,
                _ => return Err(D::Error::invalid_value(Unexpected::Str(&str), &"Either video, scene, web or preset"))
            }
        }
        Err(_) => { WallpaperType::Preset }
    })
}
