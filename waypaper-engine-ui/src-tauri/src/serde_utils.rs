use std::collections::HashMap;
use std::str::FromStr;

use serde::{Deserialize, Deserializer};
use serde::de::{Error, Unexpected};

use crate::project::WallpaperType;

pub fn from_map_str_color<'de, D>(deserializer: D) -> Result<(f64, f64, f64), D::Error>
where
    D: Deserializer<'de>,
{
    let map: HashMap<String, String> = Deserialize::deserialize(deserializer)?;
    let s = map.get("value").ok_or(Error::missing_field("value"))?;

    let parts = s
        .split(' ')
        .map(f64::from_str)
        .map(|f| f.map_err(Error::custom))
        .collect::<Result<Vec<_>, _>>()?;

    if parts.len() == 3 {
        Ok((parts[0], parts[1], parts[2]))
    } else {
        Err(Error::invalid_length(parts.len(), &"3 floats"))
    }
}

pub fn as_str_opt<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_this_or_that::as_string(deserializer) {
        Ok(s) => Ok(Some(s)),
        Err(_) => Ok(None),
    }
}

pub fn as_u64_opt<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_this_or_that::as_u64(deserializer) {
        Ok(u) => Ok(Some(u)),
        Err(_) => Ok(None),
    }
}

pub fn as_wp_type<'de, D>(deserializer: D) -> Result<WallpaperType, D::Error>
where
    D: Deserializer<'de>,
{
    let string: Result<String, _> = Deserialize::deserialize(deserializer);
    match string {
        Ok(str) => match str.to_lowercase().as_str() {
            "video" => Ok(WallpaperType::Video),
            "scene" => Ok(WallpaperType::Scene),
            "web" => Ok(WallpaperType::Web),
            _ => Err(Error::invalid_value(
                Unexpected::Str(&str),
                &"Either video, scene, web or preset",
            )),
        },
        Err(a) => Err(a),
    }
}
