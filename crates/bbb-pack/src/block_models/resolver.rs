use std::collections::{BTreeMap, HashSet};

use super::{
    classify_model_shape, BlockFaceTextures, BlockModelCatalog, BlockModelFace, BlockModelShape,
    RawBlockElement, RawBlockModel,
};

#[derive(Debug, Clone, Default)]
pub(super) struct ResolvedBlockModel {
    textures: BTreeMap<String, ResolvedTextureReference>,
    faces: [Option<ResolvedModelFace>; 6],
    elements: Vec<RawBlockElement>,
    ambient_occlusion: Option<bool>,
    pub(super) shape: BlockModelShape,
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
    pub(super) fn use_ambient_occlusion(&self) -> bool {
        self.ambient_occlusion.unwrap_or(true)
    }

    pub(super) fn face_textures(&self) -> Option<BlockFaceTextures> {
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

impl BlockModelCatalog {
    pub(super) fn resolve_model(&self, model_id: &str) -> Option<ResolvedBlockModel> {
        resolve_model_inner(&self.models, model_id, &mut HashSet::new())
    }
}

fn resolve_model_inner(
    models: &std::collections::HashMap<String, RawBlockModel>,
    model_id: &str,
    seen: &mut HashSet<String>,
) -> Option<ResolvedBlockModel> {
    let model_id = normalize_model_id(model_id);
    if !seen.insert(model_id.clone()) {
        return None;
    }
    let raw = models.get(&model_id)?;
    let mut resolved = raw
        .parent
        .as_deref()
        .and_then(|parent| resolve_model_inner(models, parent, seen))
        .unwrap_or_default();

    let mut textures_changed = false;
    for (key, value) in &raw.textures {
        if let Some(id) = value.texture_id() {
            resolved.textures.insert(
                key.clone(),
                ResolvedTextureReference {
                    id: id.to_string(),
                    force_translucent: value.force_translucent(),
                },
            );
            textures_changed = true;
        }
    }
    if let Some(ambient_occlusion) = raw.ambientocclusion {
        resolved.ambient_occlusion = Some(ambient_occlusion);
    }

    if !raw.elements.is_empty() {
        resolved.elements = raw.elements.clone();
        resolved.faces = std::array::from_fn(|_| None);
        resolved.shape = classify_model_shape(&resolved.elements, &resolved.textures);
        for element in &raw.elements {
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
    } else if textures_changed && !resolved.elements.is_empty() {
        resolved.shape = classify_model_shape(&resolved.elements, &resolved.textures);
    }

    seen.remove(&model_id);
    Some(resolved)
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
        let Some(alias) = current.strip_prefix('#') else {
            return Some(ResolvedTextureReference {
                id: normalize_texture_id(current),
                force_translucent,
            });
        };
        if !seen.insert(alias.to_string()) {
            return None;
        }
        let texture = textures.get(alias)?;
        force_translucent |= texture.force_translucent;
        current = &texture.id;
    }
}

fn normalize_model_id(id: &str) -> String {
    if id.contains(':') {
        id.to_string()
    } else if id.contains('/') {
        format!("minecraft:{id}")
    } else {
        format!("minecraft:block/{id}")
    }
}

fn normalize_texture_id(id: &str) -> String {
    if id.contains(':') {
        id.to_string()
    } else if id.contains('/') {
        format!("minecraft:{id}")
    } else {
        format!("minecraft:block/{id}")
    }
}
