use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawBlockModel {
    #[serde(default)]
    pub(super) parent: Option<String>,
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
    },
}

impl RawTextureReference {
    pub(super) fn texture_id(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            Self::Object { sprite } => sprite.as_deref(),
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
    pub(super) rotation: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) faces: BTreeMap<String, RawBlockModelFace>,
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
