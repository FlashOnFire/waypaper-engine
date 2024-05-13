use std::collections::HashMap;
use std::str::FromStr;
use serde::{Deserialize, Deserializer};
use serde::de::Error;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use serde_this_or_that::{as_bool, as_f64};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub approved: Option<bool>,
    pub contentrating: Option<String>,
    pub description: Option<String>,
    pub file: String,
    pub general: General,
    pub preview: String,
    pub tags: Option<Vec<String>>,
    pub title: String,

    #[serde(rename = "type")]
    #[serde(flatten)]
    pub wallpaper_type: WallpaperType,

    pub visibility: Option<String>,

    #[serde(default)]
    pub official: bool,
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
        options: Vec<ComboOption>
    },
    Bool {
        #[serde(deserialize_with = "as_bool")]
        value: bool
    },
    #[serde(alias = "textinput")]
    TextInput {
        value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboOption {
    value: String,
    label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WallpaperType {
    Video,
    #[serde(alias = "Scene")]
    Scene,
    Web,
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
