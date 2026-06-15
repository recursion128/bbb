use std::collections::{BTreeMap, HashMap};

use anyhow::{Context, Result};

use crate::{
    resources::{PackResourceStack, ResourceLocation},
    PackRoots,
};

mod blockstates;
mod raw_model;
mod resolver;
mod rotation;
mod shape;
mod types;

use blockstates::{RawBlockstate, RawBlockstateSelection};
use raw_model::{RawBlockElement, RawBlockElementRotation, RawBlockModel};
use resolver::{resolve_texture_alias, ResolvedTextureReference};
use rotation::{apply_variant_rotation, rotate_model_shape};
use shape::{classify_model_shape, combine_model_shapes};
pub use types::{
    BlockFaceTextures, BlockModelBox, BlockModelCross, BlockModelFace, BlockModelQuad,
    BlockModelShape, BlockRenderModel,
};

#[derive(Debug, Clone)]
pub struct BlockModelCatalog {
    blockstates: HashMap<String, RawBlockstate>,
    models: HashMap<String, RawBlockModel>,
}

impl BlockModelCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        Self::load_resource_stack(&roots.resource_stack())
    }

    fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let mut blockstates = HashMap::new();
        for resource in stack.list_resources("blockstates", ".json")? {
            let id = blockstate_id_from_resource(&resource.location)?;
            let bytes = std::fs::read(&resource.path)
                .with_context(|| format!("read blockstate {}", resource.path.display()))?;
            let blockstate = serde_json::from_slice(&bytes)
                .with_context(|| format!("parse blockstate {}", resource.path.display()))?;
            blockstates.insert(id, blockstate);
        }

        let mut models = HashMap::new();
        for resource in stack.list_resources("models/block", ".json")? {
            let id = block_model_id_from_resource(&resource.location)?;
            let bytes = std::fs::read(&resource.path)
                .with_context(|| format!("read block model {}", resource.path.display()))?;
            let model = serde_json::from_slice(&bytes)
                .with_context(|| format!("parse block model {}", resource.path.display()))?;
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
        self.block_render_model_with_seed(block_name, properties, None)
    }

    pub fn block_render_model_with_seed(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
        seed: Option<i64>,
    ) -> Option<BlockRenderModel> {
        let blockstate = self.blockstates.get(&normalize_block_id(block_name))?;
        let variants = match blockstate.select_variants(properties, seed)? {
            RawBlockstateSelection::Variants(variants) => variants,
            RawBlockstateSelection::Empty => return Some(BlockRenderModel::empty()),
        };
        let mut face_textures = None;
        let mut use_ambient_occlusion = None;
        let mut shapes = Vec::with_capacity(variants.len());
        for variant in variants {
            let model = self.resolve_model(&variant.model)?;
            use_ambient_occlusion.get_or_insert_with(|| model.use_ambient_occlusion());
            let local = model.face_textures()?;
            face_textures
                .get_or_insert_with(|| apply_variant_rotation(local, variant.x, variant.y));
            shapes.push(rotate_model_shape(
                model.shape,
                variant.x,
                variant.y,
                variant.uvlock,
            ));
        }

        Some(BlockRenderModel {
            face_textures: face_textures?,
            shape: combine_model_shapes(shapes),
            use_ambient_occlusion: use_ambient_occlusion.unwrap_or(true),
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

fn blockstate_id_from_resource(location: &ResourceLocation) -> Result<String> {
    let path = location
        .path()
        .strip_prefix("blockstates/")
        .and_then(|path| path.strip_suffix(".json"))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "blockstate resource {} is outside blockstates",
                location.id()
            )
        })?;
    ResourceLocation::new(location.namespace().to_string(), path.to_string()).map(|id| id.id())
}

fn block_model_id_from_resource(location: &ResourceLocation) -> Result<String> {
    let path = location
        .path()
        .strip_prefix("models/")
        .and_then(|path| path.strip_suffix(".json"))
        .ok_or_else(|| {
            anyhow::anyhow!("block model resource {} is outside models", location.id())
        })?;
    ResourceLocation::new(location.namespace().to_string(), path.to_string()).map(|id| id.id())
}

#[cfg(test)]
mod tests;
