use std::collections::HashMap;
use cgmath::{Vector2, Vector3};
use serde::Deserialize;
use serde_json::Value;

use waypaper_engine_shared::serde_utils::{as_vec2f32, as_vec3f32, from_str_color};
use serde_this_or_that::as_bool;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub camera: Camera,
    pub general: General,
    pub objects: Vec<Object>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Camera {
    #[serde(deserialize_with = "as_vec3f32")]
    pub center: Vector3<f32>,
    #[serde(deserialize_with = "as_vec3f32")]
    pub eye: Vector3<f32>,
    #[serde(deserialize_with = "as_vec3f32")]
    pub up: Vector3<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct General {
    #[serde(deserialize_with = "from_str_color")]
    pub ambientcolor: (f64, f64, f64),
    pub bloom: bool,
    pub bloomstrength: f64,
    pub bloomthreshold: i64,
    pub camerafade: bool,
    pub cameraparallax: bool,
    pub cameraparallaxamount: f64,
    pub cameraparallaxdelay: f64,
    pub cameraparallaxmouseinfluence: f64,
    pub camerapreview: bool,
    pub camerashake: bool,
    pub camerashakeamplitude: f64,
    pub camerashakeroughness: f64,
    pub camerashakespeed: f64,
    #[serde(deserialize_with = "from_str_color")]
    pub clearcolor: (f64, f64, f64),
    pub clearenabled: Value, // todo
    pub orthogonalprojection: OrthogonalProjection,
    #[serde(deserialize_with = "from_str_color")]
    pub skylightcolor: (f64, f64, f64),
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrthogonalProjection {
    pub height: i64,
    pub width: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct Object {
    #[serde(deserialize_with = "as_vec3f32")]
    angles: Vector3<f32>,
    #[serde(deserialize_with = "as_vec3f32")]
    origin: Vector3<f32>,
    #[serde(deserialize_with = "as_vec3f32")]
    scale: Vector3<f32>,

    name: String,

    #[serde(alias = "parallaxDepth", deserialize_with = "as_vec2f32")]
    parallax_depth: Vector2<f32>,

    id: u32,

    #[serde(flatten)]
    value: ObjectValue,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum ObjectValue {
    Image {
        #[serde(alias = "colorBlendMode")]
        color_blend_mode: i32,
        #[serde(alias = "copybackground", deserialize_with = "as_bool")]
        copy_background: bool,
        image: String,
        #[serde(deserialize_with = "as_bool")]
        visible: bool,
        #[serde(deserialize_with = "as_vec2f32")]
        size: Vector2<f32>,
    },
    Sound {
        sound: Vec<String>,
        volume: f32,

        #[serde(alias = "muteineditor", deserialize_with = "as_bool")]
        mute_in_editor: bool,
        #[serde(alias = "playbackmode")]
        playback_mode: String,
    },
    Particle {
        image: Option<String>,
        model: Option<String>,
        particle: String,
        #[serde(alias = "instanceoverride")]
        instance_override: HashMap<String, Value>
    }
}
