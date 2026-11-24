use std::collections::{BTreeMap, HashMap, HashSet};

use super::{
    classify_model_shape, simplify_model_shape, BlockFaceTextures, BlockModelDisplayContext,
    BlockModelDisplayTransform, BlockModelDisplayTransforms, BlockModelFace, BlockModelGuiLight,
    BlockModelShape, RawBlockElement, RawBlockModel,
};
use crate::resources::ResourceLocation;

pub(crate) const MAX_MODEL_PARENT_DEPTH: usize = 256;

#[derive(Debug, Clone, Default)]
pub(crate) struct ResolvedBlockModel {
    textures: BTreeMap<String, ResolvedTextureReference>,
    faces: [Option<ResolvedModelFace>; 6],
    ambient_occlusion: Option<bool>,
    gui_light: Option<BlockModelGuiLight>,
    display_transforms: ResolvedBlockModelDisplayTransforms,
    inherits_builtin_generated: bool,
    /// Exact geometry before rotation; `shape()` adds the fast paths.
    pub(crate) shape: BlockModelShape,
}

#[derive(Debug, Clone, Default)]
struct ResolvedBlockModelDisplayTransforms {
    transforms: [Option<BlockModelDisplayTransform>; 9],
}

impl ResolvedBlockModelDisplayTransforms {
    fn set(&mut self, context: BlockModelDisplayContext, transform: BlockModelDisplayTransform) {
        self.transforms[context.index()] = Some(transform);
    }

    fn get(&self, context: BlockModelDisplayContext) -> BlockModelDisplayTransform {
        self.transforms[context.index()].unwrap_or_default()
    }

    fn to_display_transforms(&self) -> BlockModelDisplayTransforms {
        BlockModelDisplayTransforms {
            third_person_left_hand: self.get(BlockModelDisplayContext::ThirdPersonLeftHand),
            third_person_right_hand: self.get(BlockModelDisplayContext::ThirdPersonRightHand),
            first_person_left_hand: self.get(BlockModelDisplayContext::FirstPersonLeftHand),
            first_person_right_hand: self.get(BlockModelDisplayContext::FirstPersonRightHand),
            head: self.get(BlockModelDisplayContext::Head),
            gui: self.get(BlockModelDisplayContext::Gui),
            ground: self.get(BlockModelDisplayContext::Ground),
            fixed: self.get(BlockModelDisplayContext::Fixed),
            on_shelf: self.get(BlockModelDisplayContext::OnShelf),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedTextureReference {
    pub(super) id: String,
    pub(super) force_translucent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedModelFace {
    texture: String,
    tint_index: Option<i32>,
}

impl ResolvedBlockModel {
    pub(crate) fn use_ambient_occlusion(&self) -> bool {
        self.ambient_occlusion.unwrap_or(true)
    }

    pub(crate) fn gui_light(&self) -> BlockModelGuiLight {
        self.gui_light.unwrap_or_default()
    }

    pub(crate) fn display_transforms(&self) -> BlockModelDisplayTransforms {
        self.display_transforms.to_display_transforms()
    }

    pub(crate) fn inherits_builtin_generated(&self) -> bool {
        self.inherits_builtin_generated
    }

    pub(crate) fn texture_slots(&self) -> BTreeMap<String, (String, bool)> {
        self.textures
            .iter()
            .filter_map(|(slot, texture)| {
                resolve_texture_reference(&self.textures, texture)
                    .map(|texture| (slot.clone(), (texture.id, texture.force_translucent)))
            })
            .collect()
    }

    /// The unrotated shape with the `Cube`/`Cross` fast paths applied.
    pub(crate) fn shape(&self) -> BlockModelShape {
        match self.face_textures() {
            Some(face_textures) => simplify_model_shape(self.shape.clone(), &face_textures),
            None => self.shape.clone(),
        }
    }

    pub(crate) fn face_textures(&self) -> Option<BlockFaceTextures> {
        let resolved_faces: [Option<ResolvedTextureReference>; 6] = std::array::from_fn(|index| {
            self.faces[index]
                .as_ref()
                .and_then(|face| resolve_texture_alias(&self.textures, &face.texture))
        });
        let fallback = self
            .textures
            .get("particle")
            .and_then(|texture| resolve_texture_reference(&self.textures, texture))
            .or_else(|| resolved_faces.iter().find_map(Clone::clone))?;
        Some(BlockFaceTextures {
            textures: std::array::from_fn(|index| {
                resolved_faces[index]
                    .as_ref()
                    .unwrap_or(&fallback)
                    .id
                    .clone()
            }),
            force_translucent: std::array::from_fn(|index| {
                resolved_faces[index]
                    .as_ref()
                    .unwrap_or(&fallback)
                    .force_translucent
            }),
            tint_indices: std::array::from_fn(|index| {
                self.faces[index].as_ref().and_then(|face| face.tint_index)
            }),
        })
    }
}

/// Resolves a cuboid model's parent chain iteratively in two passes (child -> root
/// collection with cycle/depth checks, then root -> child inheritance merge).
///
/// Vanilla `ModelDiscovery.java:92-105` rejects cyclic parent chains.
pub(crate) fn resolve_cuboid_model(
    models: &HashMap<String, RawBlockModel>,
    model_id: &str,
) -> Option<ResolvedBlockModel> {
    let mut current_id = normalize_cuboid_model_id(model_id)?;
    let mut chain: Vec<&RawBlockModel> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut reaches_builtin_generated = false;

    loop {
        if chain.len() >= MAX_MODEL_PARENT_DEPTH {
            return None;
        }
        if !seen.insert(current_id.clone()) {
            return None;
        }
        let Some(raw) = models.get(&current_id) else {
            if chain.is_empty() {
                return None;
            }
            break;
        };
        chain.push(raw);
        let Some(parent_id) = raw.parent.as_deref().and_then(normalize_cuboid_model_id) else {
            break;
        };
        if parent_id == "minecraft:builtin/generated" {
            reaches_builtin_generated = true;
            break;
        }
        current_id = parent_id;
    }

    let mut resolved = ResolvedBlockModel::default();
    let mut active_elements: Option<&[RawBlockElement]> = None;

    for raw in chain.into_iter().rev() {
        for (key, value) in &raw.textures {
            if let Some(id) = value.texture_id() {
                resolved.textures.insert(
                    key.clone(),
                    ResolvedTextureReference {
                        id: id.to_string(),
                        force_translucent: value.force_translucent(),
                    },
                );
            }
        }
        if let Some(ambient_occlusion) = raw.ambientocclusion {
            resolved.ambient_occlusion = Some(ambient_occlusion);
        }
        if let Some(gui_light) = raw.gui_light {
            resolved.gui_light = Some(gui_light);
        }
        if let Some(display) = &raw.display {
            for context in BlockModelDisplayContext::ALL {
                if let Some(transform) = display.transform(context) {
                    resolved.display_transforms.set(context, transform);
                }
            }
        }
        if !raw.elements.is_empty() {
            active_elements = Some(raw.elements.as_slice());
        }
    }

    if let Some(elements) = active_elements {
        resolved.shape = classify_model_shape(elements, &resolved.textures);
        for element in elements {
            for (face_name, face) in &element.faces {
                let Some(face_kind) = BlockModelFace::from_name(face_name) else {
                    continue;
                };
                let slot = &mut resolved.faces[face_kind.index()];
                if slot.is_none() {
                    *slot = Some(ResolvedModelFace {
                        texture: face.texture.clone(),
                        tint_index: face.tintindex,
                    });
                }
            }
        }
    }

    resolved.inherits_builtin_generated = reaches_builtin_generated
        && matches!(&resolved.shape, BlockModelShape::Boxes(boxes) if boxes.is_empty());
    Some(resolved)
}

/// Computes the effective element count of a model by walking its parent chain iteratively
/// without baking geometry. Returns 0 for missing, cyclic, or depth-exceeding chains.
pub(super) fn effective_model_element_count(
    models: &HashMap<String, RawBlockModel>,
    model_id: &str,
) -> usize {
    let Some(mut current_id) = normalize_cuboid_model_id(model_id) else {
        return 0;
    };
    let mut seen: HashSet<String> = HashSet::new();
    for _ in 0..MAX_MODEL_PARENT_DEPTH {
        if !seen.insert(current_id.clone()) {
            return 0;
        }
        let Some(raw) = models.get(&current_id) else {
            return 0;
        };
        if !raw.elements.is_empty() {
            return raw.elements.len();
        }
        let Some(parent_id) = raw.parent.as_deref().and_then(normalize_cuboid_model_id) else {
            return 0;
        };
        current_id = parent_id;
    }
    0
}

pub(super) fn resolve_texture_alias(
    textures: &BTreeMap<String, ResolvedTextureReference>,
    texture: &str,
) -> Option<ResolvedTextureReference> {
    resolve_texture_alias_with_force(textures, texture, false)
}

fn resolve_texture_reference(
    textures: &BTreeMap<String, ResolvedTextureReference>,
    texture: &ResolvedTextureReference,
) -> Option<ResolvedTextureReference> {
    resolve_texture_alias_with_force(textures, &texture.id, texture.force_translucent)
}

fn resolve_texture_alias_with_force(
    textures: &BTreeMap<String, ResolvedTextureReference>,
    texture: &str,
    mut force_translucent: bool,
) -> Option<ResolvedTextureReference> {
    let mut current = texture;
    let mut seen = HashSet::new();
    loop {
        let slot = current.strip_prefix('#').unwrap_or(current);
        if let Some(texture) = textures.get(slot) {
            if !seen.insert(slot.to_string()) {
                return None;
            }
            force_translucent |= texture.force_translucent;
            current = &texture.id;
            continue;
        }

        if current.starts_with('#') {
            return None;
        }

        return Some(ResolvedTextureReference {
            id: normalize_texture_id(current)?,
            force_translucent,
        });
    }
}

pub(crate) fn normalize_cuboid_model_id(id: &str) -> Option<String> {
    ResourceLocation::parse(id)
        .ok()
        .map(|location| location.id())
}

fn normalize_texture_id(id: &str) -> Option<String> {
    ResourceLocation::parse(id)
        .ok()
        .map(|location| location.id())
}
