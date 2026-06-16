use std::collections::BTreeMap;

use serde::Deserialize;

use super::BlockModelGuiLight;

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawBlockModel {
    #[serde(default)]
    pub(super) parent: Option<String>,
    #[serde(default)]
    pub(super) ambientocclusion: Option<bool>,
    #[serde(default)]
    pub(super) gui_light: Option<BlockModelGuiLight>,
    #[serde(default)]
    pub(super) textures: BTreeMap<String, RawTextureReference>,
    #[serde(default)]
    pub(super) elements: Vec<RawBlockElement>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(super) enum RawTextureReference {
    String(String),
    Object {
        #[serde(default)]
        sprite: Option<String>,
        #[serde(default)]
        force_translucent: bool,
    },
}

impl RawTextureReference {
    pub(super) fn texture_id(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            Self::Object { sprite, .. } => sprite.as_deref(),
        }
    }

    pub(super) fn force_translucent(&self) -> bool {
        match self {
            Self::String(_) => false,
            Self::Object {
                force_translucent, ..
            } => *force_translucent,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawBlockElement {
    #[serde(default)]
    pub(super) from: Option<[f32; 3]>,
    #[serde(default)]
    pub(super) to: Option<[f32; 3]>,
    #[serde(default)]
    pub(super) rotation: Option<RawBlockElementRotation>,
    #[serde(default)]
    pub(super) shade: Option<bool>,
    #[serde(default)]
    pub(super) light_emission: Option<i32>,
    #[serde(default)]
    pub(super) faces: BTreeMap<String, RawBlockModelFace>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawBlockElementRotation {
    pub(super) origin: [f32; 3],
    #[serde(default)]
    pub(super) axis: Option<String>,
    #[serde(default)]
    pub(super) angle: Option<f32>,
    #[serde(default)]
    pub(super) x: Option<f32>,
    #[serde(default)]
    pub(super) y: Option<f32>,
    #[serde(default)]
    pub(super) z: Option<f32>,
    #[serde(default)]
    pub(super) rescale: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawBlockModelFace {
    pub(super) texture: String,
    #[serde(default)]
    pub(super) uv: Option<[f32; 4]>,
    #[serde(default)]
    pub(super) rotation: Option<i32>,
    #[serde(default)]
    pub(super) cullface: Option<String>,
    #[serde(default)]
    pub(super) tintindex: Option<i32>,
}
