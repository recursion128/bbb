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
pub(crate) use raw_model::RawBlockModel;
use raw_model::{RawBlockElement, RawBlockElementRotation};
pub(crate) use resolver::{normalize_cuboid_model_id, resolve_cuboid_model};
use resolver::{resolve_texture_alias, ResolvedTextureReference};
use rotation::{apply_variant_rotation, rotate_model_shape};
use shape::{classify_model_shape, combine_model_shapes};
pub use types::{
    BlockFaceTextures, BlockModelBox, BlockModelCross, BlockModelDisplayContext,
    BlockModelDisplayTransform, BlockModelDisplayTransforms, BlockModelFace, BlockModelGuiLight,
    BlockModelQuad, BlockModelShape, BlockRenderModel,
};

#[derive(Debug, Clone)]
pub struct BlockModelCatalog {
    blockstates: HashMap<String, Vec<RawBlockstate>>,
    models: HashMap<String, RawBlockModel>,
}

impl BlockModelCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        Self::load_resource_stack(&roots.resource_stack())
    }

    fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let mut blockstates = HashMap::new();
        for (location, resources) in stack.list_resource_stacks("blockstates", ".json")? {
            let id = blockstate_id_from_resource(&location)?;
            let mut blockstate_stack = Vec::with_capacity(resources.len());
            for resource in resources {
                let bytes = std::fs::read(&resource.path)
                    .with_context(|| format!("read blockstate {}", resource.path.display()))?;
                let blockstate = serde_json::from_slice(&bytes)
                    .with_context(|| format!("parse blockstate {}", resource.path.display()))?;
                blockstate_stack.push(blockstate);
            }
            blockstates.insert(id, blockstate_stack);
        }

        let models = load_cuboid_model_resources(stack, "models", "block model")?;

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
        let blockstate_stack = self.blockstates.get(&normalize_block_id(block_name))?;
        let mut selected = None;
        for blockstate in blockstate_stack {
            if let Some(selection) = blockstate.select_variants(properties, seed) {
                selected = Some(selection);
            }
        }
        let variants = match selected? {
            RawBlockstateSelection::Variants(variants) => variants,
            RawBlockstateSelection::Empty => return Some(BlockRenderModel::empty()),
        };
        let mut face_textures = None;
        let mut use_ambient_occlusion = None;
        let mut gui_light = None;
        let mut display_transforms = None;
        let mut shapes = Vec::with_capacity(variants.len());
        for variant in variants {
            let model = self.resolve_model(&variant.model)?;
            use_ambient_occlusion.get_or_insert_with(|| model.use_ambient_occlusion());
            gui_light.get_or_insert_with(|| model.gui_light());
            display_transforms.get_or_insert_with(|| model.display_transforms());
            let local = model.face_textures()?;
            face_textures.get_or_insert_with(|| {
                apply_variant_rotation(local, variant.x, variant.y, variant.z)
            });
            shapes.push(rotate_model_shape(
                model.shape,
                variant.x,
                variant.y,
                variant.z,
                variant.uvlock,
            ));
        }

        Some(BlockRenderModel {
            face_textures: face_textures?,
            shape: combine_model_shapes(shapes),
            use_ambient_occlusion: use_ambient_occlusion.unwrap_or(true),
            gui_light: gui_light.unwrap_or_default(),
            display_transforms: display_transforms.unwrap_or_default(),
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

pub(crate) fn load_cuboid_model_resources(
    stack: &PackResourceStack,
    path_prefix: &str,
    label: &str,
) -> Result<HashMap<String, RawBlockModel>> {
    let mut models = HashMap::new();
    for resource in stack.list_resources(path_prefix, ".json")? {
        let id = cuboid_model_id_from_resource(&resource.location)?;
        let bytes = std::fs::read(&resource.path)
            .with_context(|| format!("read {label} {}", resource.path.display()))?;
        let model = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse {label} {}", resource.path.display()))?;
        models.insert(id, model);
    }
    Ok(models)
}

fn cuboid_model_id_from_resource(location: &ResourceLocation) -> Result<String> {
    let path = location
        .path()
        .strip_prefix("models/")
        .and_then(|path| path.strip_suffix(".json"))
        .ok_or_else(|| anyhow::anyhow!("model resource {} is outside models", location.id()))?;
    ResourceLocation::new(location.namespace().to_string(), path.to_string()).map(|id| id.id())
}

#[cfg(test)]
mod tests;
