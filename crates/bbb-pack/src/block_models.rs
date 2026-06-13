use std::collections::{BTreeMap, HashMap};

use anyhow::{Context, Result};

use crate::PackRoots;

mod blockstates;
mod raw_model;
mod resolver;
mod rotation;
mod shape;
mod types;

use blockstates::RawBlockstate;
use raw_model::{RawBlockElement, RawBlockModel};
use resolver::resolve_texture_alias;
use rotation::{apply_variant_rotation, rotate_model_shape};
use shape::{classify_model_shape, combine_model_shapes};
pub use types::{
    BlockFaceTextures, BlockModelBox, BlockModelFace, BlockModelShape, BlockRenderModel,
};

#[derive(Debug, Clone)]
pub struct BlockModelCatalog {
    blockstates: HashMap<String, RawBlockstate>,
    models: HashMap<String, RawBlockModel>,
}

impl BlockModelCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        let mut blockstates = HashMap::new();
        let blockstates_dir = roots.blockstates_dir();
        for entry in std::fs::read_dir(&blockstates_dir)
            .with_context(|| format!("read blockstate directory {}", blockstates_dir.display()))?
        {
            let entry = entry.with_context(|| {
                format!("read blockstate entry in {}", blockstates_dir.display())
            })?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let id = format!("minecraft:{stem}");
            let bytes = std::fs::read(&path)
                .with_context(|| format!("read blockstate {}", path.display()))?;
            let blockstate = serde_json::from_slice(&bytes)
                .with_context(|| format!("parse blockstate {}", path.display()))?;
            blockstates.insert(id, blockstate);
        }

        let mut models = HashMap::new();
        let models_dir = roots.block_models_dir();
        for entry in std::fs::read_dir(&models_dir)
            .with_context(|| format!("read block model directory {}", models_dir.display()))?
        {
            let entry = entry
                .with_context(|| format!("read block model entry in {}", models_dir.display()))?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let id = format!("minecraft:block/{stem}");
            let bytes = std::fs::read(&path)
                .with_context(|| format!("read block model {}", path.display()))?;
            let model = serde_json::from_slice(&bytes)
                .with_context(|| format!("parse block model {}", path.display()))?;
            models.insert(id, model);
        }

        Ok(Self {
            blockstates,
            models,
        })
    }

    pub fn len(&self) -> usize {
        self.blockstates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blockstates.is_empty()
    }

    pub fn block_face_textures(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<BlockFaceTextures> {
        self.block_render_model(block_name, properties)
            .map(|model| model.face_textures)
    }

    pub fn block_render_model(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<BlockRenderModel> {
        let blockstate = self.blockstates.get(&normalize_block_id(block_name))?;
        let variants = blockstate.select_variants(properties)?;
        let mut face_textures = None;
        let mut shapes = Vec::with_capacity(variants.len());
        for variant in variants {
            let model = self.resolve_model(&variant.model)?;
            let local = model.face_textures()?;
            face_textures
                .get_or_insert_with(|| apply_variant_rotation(local, variant.x, variant.y));
            shapes.push(rotate_model_shape(model.shape, variant.x, variant.y));
        }

        Some(BlockRenderModel {
            face_textures: face_textures?,
            shape: combine_model_shapes(shapes),
        })
    }
}

fn normalize_block_id(id: &str) -> String {
    if id.contains(':') {
        id.to_string()
    } else {
        format!("minecraft:{id}")
    }
}

#[cfg(test)]
mod tests;
