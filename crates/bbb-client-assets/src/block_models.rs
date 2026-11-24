use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

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

pub use blockstates::BlockStateModelChoices;
use blockstates::{RawBlockstate, RawBlockstateSelection, RawModelVariant};
pub(crate) use raw_model::RawBlockModel;
use raw_model::{RawBlockElement, RawBlockElementRotation};
use resolver::{
    effective_model_element_count, resolve_texture_alias, ResolvedBlockModel,
    ResolvedTextureReference,
};
pub(crate) use resolver::{normalize_cuboid_model_id, resolve_cuboid_model};
use rotation::{apply_variant_rotation, rotate_model_shape};
use shape::{
    classify_model_shape, combine_model_shapes, shape_has_force_translucent, simplify_model_shape,
};
pub use types::{
    BlockFaceTextures, BlockModelBox, BlockModelCross, BlockModelDisplayContext,
    BlockModelDisplayTransform, BlockModelDisplayTransforms, BlockModelFace, BlockModelGuiLight,
    BlockModelQuad, BlockModelShape, BlockRenderModel,
};

const MAX_BLOCKSTATE_DOCUMENTS: u64 = 65_536;
const MAX_BLOCKSTATE_SELECTION_ENTRIES: u64 = 262_144;
const MAX_CUBOID_MODEL_DOCUMENTS: u64 = 65_536;
const MAX_CUBOID_MODEL_ELEMENTS: u64 = 262_144;
const MAX_CUBOID_MODEL_FACES: u64 = 1_048_576;
const MAX_CUBOID_MODEL_TEXTURE_SLOTS: u64 = 262_144;
const MAX_ELEMENTS_PER_CUBOID_MODEL: usize = 1_024;
const MAX_FACES_PER_CUBOID_MODEL: usize = 6_144;
const MAX_COMBINED_ELEMENTS_PER_BLOCKSTATE: usize = 2_048;
const BLOCKSTATE_DOCUMENT_LOGICAL_BYTES: u64 = 512;
const BLOCKSTATE_SELECTION_LOGICAL_BYTES: u64 = 256;
const CUBOID_MODEL_DOCUMENT_LOGICAL_BYTES: u64 = 512;
const CUBOID_MODEL_ELEMENT_LOGICAL_BYTES: u64 = 256;
const CUBOID_MODEL_FACE_LOGICAL_BYTES: u64 = 128;
const CUBOID_MODEL_TEXTURE_SLOT_LOGICAL_BYTES: u64 = 128;
const ROTATED_VARIANT_LOGICAL_BYTES: u64 = 256;
const ROTATED_ELEMENT_LOGICAL_BYTES: u64 = 128;

#[cfg(not(target_arch = "wasm32"))]
type CacheLock<T> = std::sync::RwLock<T>;

#[cfg(target_arch = "wasm32")]
type CacheLock<T> = bbb_render_types::wasm_sync::SpinRwLock<T>;

type RotatedVariantKey = (String, i32, i32, i32, bool);

#[derive(Debug)]
struct CachedRotatedVariant {
    face_textures: BlockFaceTextures,
    particle_texture: String,
    shape: BlockModelShape,
    use_ambient_occlusion: bool,
    gui_light: BlockModelGuiLight,
    display_transforms: BlockModelDisplayTransforms,
    has_force_translucent: bool,
}

#[derive(Debug, Default)]
struct BlockModelCache {
    resolved_models: CacheLock<HashMap<String, Option<Arc<ResolvedBlockModel>>>>,
    rotated_variants: CacheLock<HashMap<RotatedVariantKey, Option<Arc<CachedRotatedVariant>>>>,
}

impl BlockModelCache {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            resolved_models: CacheLock::new(HashMap::new()),
            rotated_variants: CacheLock::new(HashMap::new()),
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_cache<T>(lock: &CacheLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(target_arch = "wasm32")]
fn read_cache<T>(lock: &CacheLock<T>) -> bbb_render_types::wasm_sync::SpinReadGuard<'_, T> {
    lock.read()
}

#[cfg(not(target_arch = "wasm32"))]
fn write_cache<T>(lock: &CacheLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(target_arch = "wasm32")]
fn write_cache<T>(lock: &CacheLock<T>) -> bbb_render_types::wasm_sync::SpinWriteGuard<'_, T> {
    lock.write()
}

#[derive(Debug, Clone)]
pub struct BlockModelCatalog {
    blockstates: HashMap<String, Vec<RawBlockstate>>,
    models: Arc<HashMap<String, RawBlockModel>>,
    cache: Arc<BlockModelCache>,
}

impl PartialEq for BlockModelCatalog {
    fn eq(&self, other: &Self) -> bool {
        self.blockstates == other.blockstates && self.models == other.models
    }
}

impl BlockModelCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        let stack = roots.resource_stack();
        let models = shared_cuboid_models(roots, &stack)?;
        Self::load_with_models(&stack, models)
    }

    pub fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let models = Arc::new(load_cuboid_model_resources(stack)?);
        Self::load_with_models(stack, models)
    }

    pub(crate) fn load_with_models(
        stack: &PackResourceStack,
        models: Arc<HashMap<String, RawBlockModel>>,
    ) -> Result<Self> {
        let listed = stack.list_resource_stacks("blockstates", ".json")?;
        let parsed = if listed.len() >= 32 && crate::can_use_rayon() {
            use rayon::prelude::*;
            let failed = std::sync::atomic::AtomicBool::new(false);
            listed
                .into_par_iter()
                .map(|(location, resources)| {
                    if failed.load(std::sync::atomic::Ordering::Relaxed) {
                        return Ok(None);
                    }
                    let id = blockstate_id_from_resource(&location)?;
                    let mut blockstate_stack = Vec::with_capacity(resources.len());
                    for resource in resources {
                        let blockstate: RawBlockstate = match stack
                            .read_json_resource(&resource, "blockstate")
                            .with_context(|| {
                                format!("parse blockstate {}", resource.path.display())
                            }) {
                            Ok(blockstate) => blockstate,
                            Err(err) => {
                                failed.store(true, std::sync::atomic::Ordering::Relaxed);
                                return Err(err);
                            }
                        };
                        blockstate_stack.push(blockstate);
                    }
                    Ok(Some((id, blockstate_stack)))
                })
                .collect::<Vec<_>>()
        } else {
            listed
                .into_iter()
                .map(|(location, resources)| {
                    let id = blockstate_id_from_resource(&location)?;
                    let mut blockstate_stack = Vec::with_capacity(resources.len());
                    for resource in resources {
                        let blockstate: RawBlockstate = stack
                            .read_json_resource(&resource, "blockstate")
                            .with_context(|| {
                                format!("parse blockstate {}", resource.path.display())
                            })?;
                        blockstate_stack.push(blockstate);
                    }
                    Ok(Some((id, blockstate_stack)))
                })
                .collect::<Vec<_>>()
        };

        let mut blockstates = HashMap::with_capacity(parsed.len());
        let mut blockstate_documents = 0_u64;
        let mut blockstate_selections = 0_u64;
        let mut unique_rotated_variants: HashSet<RotatedVariantKey> = HashSet::new();
        let mut model_element_counts: HashMap<String, usize> = HashMap::new();
        let mut derived_geometry_bytes = 0_u64;
        for entry in parsed {
            let Some((id, blockstate_stack)) = entry? else {
                continue;
            };
            for blockstate in &blockstate_stack {
                if let Err(err) = blockstate.validate() {
                    return Err(
                        stack.reject_resource_data(format!("invalid blockstate {id}: {err:#}"))
                    );
                }
                blockstate_documents = blockstate_documents
                    .checked_add(1)
                    .context("blockstate document count overflow")?;
                blockstate_selections = blockstate_selections
                    .checked_add(
                        u64::try_from(
                            blockstate
                                .selection_entry_count()
                                .context("blockstate selection count overflow")?,
                        )
                        .context("blockstate selection count overflow")?,
                    )
                    .context("blockstate selection count overflow")?;
                stack.admit_resource_collection_count(
                    blockstate_documents,
                    MAX_BLOCKSTATE_DOCUMENTS,
                    "blockstate documents",
                )?;
                stack.admit_resource_collection_count(
                    blockstate_selections,
                    MAX_BLOCKSTATE_SELECTION_ENTRIES,
                    "blockstate selection entries",
                )?;

                let mut multipart_elements = 0usize;
                for selector in blockstate.multipart_selector_slices() {
                    let selector_max = selector
                        .iter()
                        .map(|variant| {
                            cached_effective_element_count(
                                &models,
                                &mut model_element_counts,
                                &variant.model,
                            )
                        })
                        .max()
                        .unwrap_or(0);
                    multipart_elements =
                        multipart_elements
                            .checked_add(selector_max)
                            .ok_or_else(|| {
                                stack.reject_resource_data(format!(
                                    "blockstate {id} combined multipart element count overflow"
                                ))
                            })?;
                    if multipart_elements > MAX_COMBINED_ELEMENTS_PER_BLOCKSTATE {
                        return Err(stack.reject_resource_data(format!(
                            "blockstate {id} multipart selectors combine {multipart_elements} elements, exceeding the {MAX_COMBINED_ELEMENTS_PER_BLOCKSTATE} element limit"
                        )));
                    }
                }

                for variant in blockstate.all_referenced_variants() {
                    let Some(key) = rotated_variant_key(variant) else {
                        continue;
                    };
                    if unique_rotated_variants.insert(key.clone()) {
                        let element_count = cached_effective_element_count(
                            &models,
                            &mut model_element_counts,
                            &key.0,
                        );
                        let variant_bytes = u64::try_from(element_count)
                            .ok()
                            .and_then(|count| count.checked_mul(ROTATED_ELEMENT_LOGICAL_BYTES))
                            .and_then(|bytes| bytes.checked_add(ROTATED_VARIANT_LOGICAL_BYTES))
                            .ok_or_else(|| {
                                stack.reject_resource_data(
                                    "blockstate derived geometry byte count overflow",
                                )
                            })?;
                        derived_geometry_bytes = derived_geometry_bytes
                            .checked_add(variant_bytes)
                            .ok_or_else(|| {
                                stack.reject_resource_data(
                                    "blockstate derived geometry byte count overflow",
                                )
                            })?;
                    }
                }
            }
            blockstates.insert(id, blockstate_stack);
        }
        let blockstate_logical_bytes = blockstate_documents
            .checked_mul(BLOCKSTATE_DOCUMENT_LOGICAL_BYTES)
            .and_then(|bytes| {
                blockstate_selections
                    .checked_mul(BLOCKSTATE_SELECTION_LOGICAL_BYTES)
                    .and_then(|selections| bytes.checked_add(selections))
            })
            .and_then(|bytes| bytes.checked_add(derived_geometry_bytes))
            .context("blockstate derived logical byte count overflow")?;
        stack.reserve_derived_resource_bytes(
            blockstate_logical_bytes,
            "retained blockstate catalog",
        )?;

        Ok(Self {
            blockstates,
            models,
            cache: BlockModelCache::new(),
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
        let choices = self.block_state_model_choices(block_name, properties)?;
        self.block_state_model_for_seed(&choices, seed)
    }

    pub fn block_state_model_choices(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<BlockStateModelChoices> {
        let blockstate_stack = self.blockstates.get(&normalize_block_id(block_name))?;
        blockstate_stack
            .iter()
            .filter_map(|blockstate| blockstate.model_choices(properties))
            .next_back()
    }

    /// Builds the model selected by `combination` (from `BlockStateModelChoices::combination`).
    pub fn block_state_model(
        &self,
        choices: &BlockStateModelChoices,
        combination: usize,
    ) -> Option<BlockRenderModel> {
        if choices.is_empty() {
            return Some(BlockRenderModel::empty());
        }
        self.build_block_render_model(choices.models(combination))
    }

    /// Builds the model selected directly from `seed`, avoiding mixed-radix combination overflow
    /// for multipart blockstates with many weighted parts.
    pub fn block_state_model_for_seed(
        &self,
        choices: &BlockStateModelChoices,
        seed: Option<i64>,
    ) -> Option<BlockRenderModel> {
        if choices.is_empty() {
            return Some(BlockRenderModel::empty());
        }
        self.build_block_render_model(choices.models_for_seed(seed))
    }

    /// Returns whether any baked material-flag candidate for this block state marks
    /// `force_translucent`, without cloning combined geometry.
    pub fn block_material_force_translucent(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<bool> {
        let blockstate_stack = self.blockstates.get(&normalize_block_id(block_name))?;
        let mut selected = None;
        for blockstate in blockstate_stack {
            if let Some(selection) = blockstate.material_flag_variants(properties) {
                selected = Some(selection);
            }
        }
        match selected? {
            RawBlockstateSelection::Empty => Some(false),
            RawBlockstateSelection::Variants(variants) => {
                let mut any_translucent = false;
                for variant in &variants {
                    let cached = self.cached_rotated_variant(variant)?;
                    any_translucent |= cached.has_force_translucent;
                }
                Some(any_translucent)
            }
        }
    }

    /// Returns the complete baked-model candidate set whose material flags determine whether
    /// Vanilla draws a selected block outline before or after translucent terrain.
    ///
    /// This is not a geometry-selection API: weighted variants and multipart selectors have
    /// intentionally broader flag aggregation rules than the per-position model resolver.
    pub fn block_material_models(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<Vec<BlockRenderModel>> {
        let blockstate_stack = self.blockstates.get(&normalize_block_id(block_name))?;
        let mut selected = None;
        for blockstate in blockstate_stack {
            if let Some(selection) = blockstate.material_flag_variants(properties) {
                selected = Some(selection);
            }
        }
        match selected? {
            RawBlockstateSelection::Empty => Some(Vec::new()),
            RawBlockstateSelection::Variants(variants) => variants
                .into_iter()
                .map(|variant| self.build_block_render_model(std::iter::once(&variant)))
                .collect(),
        }
    }

    pub(crate) fn resolve_direct_block_model(&self, model_id: &str) -> Option<BlockRenderModel> {
        let model = self.resolve_model(model_id)?;
        let face_textures = model.face_textures()?;
        let particle_texture = model
            .texture_slots()
            .get("particle")
            .map(|(texture_id, _)| texture_id.clone())
            .or_else(|| Some(face_textures.get(BlockModelFace::Down).to_string()));
        Some(BlockRenderModel {
            face_textures,
            particle_texture,
            shape: model.shape(),
            use_ambient_occlusion: model.use_ambient_occlusion(),
            gui_light: model.gui_light(),
            display_transforms: model.display_transforms(),
        })
    }

    fn resolve_model(&self, model_id: &str) -> Option<Arc<ResolvedBlockModel>> {
        let normalized = normalize_cuboid_model_id(model_id)?;
        if let Some(entry) = read_cache(&self.cache.resolved_models).get(&normalized) {
            return entry.clone();
        }
        let resolved = resolve_cuboid_model(&self.models, &normalized).map(Arc::new);
        let mut guard = write_cache(&self.cache.resolved_models);
        guard
            .entry(normalized)
            .or_insert_with(|| resolved.clone())
            .clone()
    }

    fn cached_rotated_variant(
        &self,
        variant: &RawModelVariant,
    ) -> Option<Arc<CachedRotatedVariant>> {
        let key = rotated_variant_key(variant)?;
        if let Some(entry) = read_cache(&self.cache.rotated_variants).get(&key) {
            return entry.clone();
        }
        let computed = self.compute_rotated_variant(variant).map(Arc::new);
        let mut guard = write_cache(&self.cache.rotated_variants);
        guard.entry(key).or_insert_with(|| computed.clone()).clone()
    }

    fn compute_rotated_variant(&self, variant: &RawModelVariant) -> Option<CachedRotatedVariant> {
        let model = self.resolve_model(&variant.model)?;
        let local = model.face_textures()?;
        let particle_texture = model
            .texture_slots()
            .get("particle")
            .map(|(texture_id, _force_translucent)| texture_id.clone())
            .unwrap_or_else(|| local.get(BlockModelFace::Down).to_string());
        // Blockstate rotations turn clockwise (`OctahedralGroup.BLOCK_ROT_Y_90 =
        // ROT_90_Y_NEG`); the quarter turns in `rotation` are counter-clockwise.
        let (x, y, z) = (-variant.x, -variant.y, -variant.z);
        let face_textures = apply_variant_rotation(local, x, y, z);
        let shape = rotate_model_shape(model.shape.clone(), x, y, z, variant.uvlock);
        let has_force_translucent = shape_has_force_translucent(&shape, &face_textures);
        Some(CachedRotatedVariant {
            face_textures,
            particle_texture,
            shape,
            use_ambient_occlusion: model.use_ambient_occlusion(),
            gui_light: model.gui_light(),
            display_transforms: model.display_transforms(),
            has_force_translucent,
        })
    }

    fn build_block_render_model<'a>(
        &self,
        variants: impl Iterator<Item = &'a RawModelVariant>,
    ) -> Option<BlockRenderModel> {
        let mut face_textures = None;
        let mut particle_texture = None;
        let mut use_ambient_occlusion = None;
        let mut gui_light = None;
        let mut display_transforms = None;
        let mut shapes = Vec::new();
        for variant in variants {
            let cached = self.cached_rotated_variant(variant)?;
            use_ambient_occlusion.get_or_insert(cached.use_ambient_occlusion);
            gui_light.get_or_insert(cached.gui_light);
            display_transforms.get_or_insert_with(|| cached.display_transforms.clone());
            particle_texture.get_or_insert_with(|| cached.particle_texture.clone());
            face_textures.get_or_insert_with(|| cached.face_textures.clone());
            shapes.push(cached.shape.clone());
        }

        let face_textures = face_textures?;
        let shape = simplify_model_shape(combine_model_shapes(shapes), &face_textures);
        Some(BlockRenderModel {
            face_textures,
            particle_texture,
            shape,
            use_ambient_occlusion: use_ambient_occlusion.unwrap_or(true),
            gui_light: gui_light.unwrap_or_default(),
            display_transforms: display_transforms.unwrap_or_default(),
        })
    }
}

fn rotated_variant_key(variant: &RawModelVariant) -> Option<RotatedVariantKey> {
    Some((
        normalize_cuboid_model_id(&variant.model)?,
        variant.x.rem_euclid(360),
        variant.y.rem_euclid(360),
        variant.z.rem_euclid(360),
        variant.uvlock,
    ))
}

fn cached_effective_element_count(
    models: &HashMap<String, RawBlockModel>,
    cache: &mut HashMap<String, usize>,
    model_id: &str,
) -> usize {
    let Some(normalized) = normalize_cuboid_model_id(model_id) else {
        return 0;
    };
    if let Some(&count) = cache.get(&normalized) {
        return count;
    }
    let count = effective_model_element_count(models, &normalized);
    cache.insert(normalized, count);
    count
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

pub(crate) fn shared_cuboid_models(
    roots: &PackRoots,
    stack: &PackResourceStack,
) -> Result<Arc<HashMap<String, RawBlockModel>>> {
    match roots.client_resource_generation() {
        Some(generation) => generation.cuboid_models(),
        None => load_cuboid_model_resources(stack).map(Arc::new),
    }
}

pub(crate) fn load_cuboid_model_resources(
    stack: &PackResourceStack,
) -> Result<HashMap<String, RawBlockModel>> {
    let label = "block model";
    let resources = stack.list_resources("models", ".json")?;
    let parsed = if resources.len() >= 32 && crate::can_use_rayon() {
        use rayon::prelude::*;
        let failed = std::sync::atomic::AtomicBool::new(false);
        resources
            .into_par_iter()
            .map(|resource| {
                if failed.load(std::sync::atomic::Ordering::Relaxed) {
                    return Ok(None);
                }
                let id = cuboid_model_id_from_resource(&resource.location)?;
                let model: RawBlockModel = match stack
                    .read_json_resource(&resource, label)
                    .with_context(|| format!("parse {label} {}", resource.path.display()))
                {
                    Ok(model) => model,
                    Err(err) => {
                        failed.store(true, std::sync::atomic::Ordering::Relaxed);
                        return Err(err);
                    }
                };
                Ok(Some((id, model)))
            })
            .collect::<Vec<_>>()
    } else {
        resources
            .into_iter()
            .map(|resource| {
                let id = cuboid_model_id_from_resource(&resource.location)?;
                let model: RawBlockModel = stack
                    .read_json_resource(&resource, label)
                    .with_context(|| format!("parse {label} {}", resource.path.display()))?;
                Ok(Some((id, model)))
            })
            .collect::<Vec<_>>()
    };

    let mut models = HashMap::with_capacity(parsed.len());
    let mut documents = 0_u64;
    let mut elements = 0_u64;
    let mut faces = 0_u64;
    let mut texture_slots = 0_u64;
    for entry in parsed {
        let Some((id, model)) = entry? else {
            continue;
        };
        let model_elements = model.element_count();
        if model_elements > MAX_ELEMENTS_PER_CUBOID_MODEL {
            return Err(stack.reject_resource_data(format!(
                "cuboid model {id} has {model_elements} elements, exceeding the {MAX_ELEMENTS_PER_CUBOID_MODEL} element limit"
            )));
        }
        let model_faces = model.face_count().context("model face overflow")?;
        if model_faces > MAX_FACES_PER_CUBOID_MODEL {
            return Err(stack.reject_resource_data(format!(
                "cuboid model {id} has {model_faces} faces, exceeding the {MAX_FACES_PER_CUBOID_MODEL} face limit"
            )));
        }
        documents = documents.checked_add(1).context("model count overflow")?;
        elements = elements
            .checked_add(u64::try_from(model_elements).context("model element overflow")?)
            .context("model element count overflow")?;
        faces = faces
            .checked_add(u64::try_from(model_faces).context("model face overflow")?)
            .context("model face count overflow")?;
        texture_slots = texture_slots
            .checked_add(
                u64::try_from(model.texture_slot_count()).context("model texture slot overflow")?,
            )
            .context("model texture slot count overflow")?;
        stack.admit_resource_collection_count(
            documents,
            MAX_CUBOID_MODEL_DOCUMENTS,
            &format!("{label} documents"),
        )?;
        stack.admit_resource_collection_count(
            elements,
            MAX_CUBOID_MODEL_ELEMENTS,
            &format!("{label} elements"),
        )?;
        stack.admit_resource_collection_count(
            faces,
            MAX_CUBOID_MODEL_FACES,
            &format!("{label} faces"),
        )?;
        stack.admit_resource_collection_count(
            texture_slots,
            MAX_CUBOID_MODEL_TEXTURE_SLOTS,
            &format!("{label} texture slots"),
        )?;
        models.insert(id, model);
    }
    let logical_bytes = documents
        .checked_mul(CUBOID_MODEL_DOCUMENT_LOGICAL_BYTES)
        .and_then(|bytes| {
            elements
                .checked_mul(CUBOID_MODEL_ELEMENT_LOGICAL_BYTES)
                .and_then(|elements| bytes.checked_add(elements))
        })
        .and_then(|bytes| {
            faces
                .checked_mul(CUBOID_MODEL_FACE_LOGICAL_BYTES)
                .and_then(|faces| bytes.checked_add(faces))
        })
        .and_then(|bytes| {
            texture_slots
                .checked_mul(CUBOID_MODEL_TEXTURE_SLOT_LOGICAL_BYTES)
                .and_then(|slots| bytes.checked_add(slots))
        })
        .context("model derived logical byte count overflow")?;
    stack.reserve_derived_resource_bytes(logical_bytes, &format!("retained {label} catalog"))?;
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
