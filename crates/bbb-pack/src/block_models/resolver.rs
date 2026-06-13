use std::collections::{BTreeMap, HashSet};

use super::{
    classify_model_shape, BlockFaceTextures, BlockModelCatalog, BlockModelFace, BlockModelShape,
    RawBlockModel,
};

#[derive(Debug, Clone, Default)]
pub(super) struct ResolvedBlockModel {
    textures: BTreeMap<String, String>,
    faces: [Option<ResolvedModelFace>; 6],
    pub(super) shape: BlockModelShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedModelFace {
    texture: String,
    tint_index: Option<i32>,
}

impl ResolvedBlockModel {
    pub(super) fn face_textures(&self) -> Option<BlockFaceTextures> {
        let resolved_faces: [Option<String>; 6] = std::array::from_fn(|index| {
            self.faces[index]
                .as_ref()
                .and_then(|face| resolve_texture_alias(&self.textures, &face.texture))
        });
        let fallback = self
            .textures
            .get("particle")
            .and_then(|texture| resolve_texture_alias(&self.textures, texture))
            .or_else(|| resolved_faces.iter().find_map(Clone::clone))?;
        Some(BlockFaceTextures {
            textures: std::array::from_fn(|index| {
                resolved_faces[index]
                    .clone()
                    .unwrap_or_else(|| fallback.clone())
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

    for (key, value) in &raw.textures {
        if let Some(value) = value.texture_id() {
            resolved.textures.insert(key.clone(), value.to_string());
        }
    }

    if !raw.elements.is_empty() {
        resolved.faces = std::array::from_fn(|_| None);
        resolved.shape = classify_model_shape(&raw.elements, &resolved.textures);
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
    }

    seen.remove(&model_id);
    Some(resolved)
}

pub(super) fn resolve_texture_alias(
    textures: &BTreeMap<String, String>,
    texture: &str,
) -> Option<String> {
    let mut current = texture;
    let mut seen = HashSet::new();
    loop {
        let Some(alias) = current.strip_prefix('#') else {
            return Some(normalize_texture_id(current));
        };
        if !seen.insert(alias.to_string()) {
            return None;
        }
        current = textures.get(alias)?;
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
