use std::collections::BTreeMap;

use serde::Deserialize;

use super::{BlockModelDisplayContext, BlockModelDisplayTransform, BlockModelGuiLight};

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RawBlockModel {
    #[serde(default)]
    pub(crate) parent: Option<String>,
    #[serde(default)]
    pub(super) ambientocclusion: Option<bool>,
    #[serde(default)]
    pub(super) gui_light: Option<BlockModelGuiLight>,
    #[serde(default)]
    pub(super) display: Option<RawBlockModelDisplayTransforms>,
    #[serde(default)]
    pub(super) textures: BTreeMap<String, RawTextureReference>,
    #[serde(default)]
    pub(super) elements: Vec<RawBlockElement>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawBlockModelDisplayTransforms {
    #[serde(default, rename = "thirdperson_lefthand")]
    third_person_left_hand: Option<RawBlockModelDisplayTransform>,
    #[serde(default, rename = "thirdperson_righthand")]
    third_person_right_hand: Option<RawBlockModelDisplayTransform>,
    #[serde(default, rename = "firstperson_lefthand")]
    first_person_left_hand: Option<RawBlockModelDisplayTransform>,
    #[serde(default, rename = "firstperson_righthand")]
    first_person_right_hand: Option<RawBlockModelDisplayTransform>,
    #[serde(default)]
    head: Option<RawBlockModelDisplayTransform>,
    #[serde(default)]
    gui: Option<RawBlockModelDisplayTransform>,
    #[serde(default)]
    ground: Option<RawBlockModelDisplayTransform>,
    #[serde(default)]
    fixed: Option<RawBlockModelDisplayTransform>,
    #[serde(default)]
    on_shelf: Option<RawBlockModelDisplayTransform>,
}

impl RawBlockModelDisplayTransforms {
    pub(super) fn transform(
        &self,
        context: BlockModelDisplayContext,
    ) -> Option<BlockModelDisplayTransform> {
        match context {
            BlockModelDisplayContext::ThirdPersonLeftHand => self
                .third_person_left_hand
                .as_ref()
                .or(self.third_person_right_hand.as_ref()),
            BlockModelDisplayContext::ThirdPersonRightHand => self.third_person_right_hand.as_ref(),
            BlockModelDisplayContext::FirstPersonLeftHand => self
                .first_person_left_hand
                .as_ref()
                .or(self.first_person_right_hand.as_ref()),
            BlockModelDisplayContext::FirstPersonRightHand => self.first_person_right_hand.as_ref(),
            BlockModelDisplayContext::Head => self.head.as_ref(),
            BlockModelDisplayContext::Gui => self.gui.as_ref(),
            BlockModelDisplayContext::Ground => self.ground.as_ref(),
            BlockModelDisplayContext::Fixed => self.fixed.as_ref(),
            BlockModelDisplayContext::OnShelf => self.on_shelf.as_ref(),
        }
        .map(|transform| (*transform).to_model_transform())
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct RawBlockModelDisplayTransform {
    #[serde(default)]
    rotation: Option<[f32; 3]>,
    #[serde(default)]
    translation: Option<[f32; 3]>,
    #[serde(default)]
    scale: Option<[f32; 3]>,
}

impl RawBlockModelDisplayTransform {
    fn to_model_transform(self) -> BlockModelDisplayTransform {
        let rotation = self.rotation.unwrap_or([0.0; 3]);
        let translation = self
            .translation
            .unwrap_or([0.0; 3])
            .map(|value| (value * 0.0625).clamp(-5.0, 5.0));
        let scale = self
            .scale
            .unwrap_or([1.0; 3])
            .map(|value| value.clamp(-4.0, 4.0));
        BlockModelDisplayTransform {
            rotation,
            translation,
            scale,
        }
    }
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
