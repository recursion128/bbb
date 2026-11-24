use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use crate::{
    terrain::{
        TerrainCross, TerrainFace, TerrainFluidKind, TerrainQuad, TerrainRenderShape,
        TerrainTextureAtlas, TerrainTint, TerrainTransparency, TerrainUvRect,
    },
    ParticleSpriteUv, ParticleUvRect,
};
use anyhow::Result;
use bbb_client_assets::{
    AtlasLayout, AtlasPacker, BiomeColorCatalog, BiomeColorProfile, BiomeTemperatureModifier,
    BlockFaceTextures, BlockModelCatalog, BlockModelFace, BlockModelShape,
    ContentRegistryAssetProjection, GrassColorModifier, PackRoots, SpriteImage, TerrainColorMaps,
};
use bbb_protocol::content_registry::ContentRegistry;
pub use bbb_protocol::packets::BlockPos as BlockRenderPosition;

use crate::scene::biome_tint::{
    apply_grass_color_modifier, biome_colormap_climate, is_foliage_tinted_block,
    terrain_tint_from_rgb, BiomeBlend, BiomeTintChannel, BiomeTintResolver,
};

mod state_render;
pub use state_render::TerrainResourceSource;
pub(crate) use state_render::{SectionTintAccess, TerrainResolvedState};
use state_render::{TerrainModelRender, TerrainTintRule, TerrainTintSource};

// Vanilla 26.1's `Stitcher` resolves the consumed blocks atlas to 2048x2048
// for the mip-4 vanilla pack (confirmed by the official client's atlas log).
// Keeping the same width is observable in `terrain.fsh`: RGSS derives its LOD
// from the atlas dimensions, so the previous 4096x1024 shelf changed filtering
// even though every sprite and mip payload was otherwise identical.
const VANILLA_26_1_BLOCK_ATLAS_WIDTH: u32 = 2048;

const MAX_BLOCK_TRANSLUCENT_MATERIAL_CACHE: usize = 32_768;

type CachedQuadsByName = RefCell<HashMap<String, Option<Arc<[crate::ItemModelQuad]>>>>;
type CachedQuadsByProperties =
    RefCell<HashMap<(String, BTreeMap<String, String>), Option<Arc<[crate::ItemModelQuad]>>>>;
type ItemFrameBorderQuadCache = RefCell<HashMap<(bool, bool), Arc<[crate::ItemModelQuad]>>>;
type BlockTranslucentMaterialCache =
    RefCell<hashbrown::HashMap<(String, BTreeMap<String, String>), bool>>;

#[derive(Hash)]
struct BlockMaterialCacheKey<'a>(&'a str, &'a BTreeMap<String, String>);

impl hashbrown::Equivalent<(String, BTreeMap<String, String>)> for BlockMaterialCacheKey<'_> {
    fn equivalent(&self, key: &(String, BTreeMap<String, String>)) -> bool {
        key.0 == self.0 && &key.1 == self.1
    }
}

#[derive(Debug, Clone)]
pub struct TerrainTextureState {
    /// Changes whenever anything a block render table bakes in changes.
    generation: u64,
    core: Arc<TerrainTextureCore>,
    colormaps: Option<TerrainColorMaps>,
    biome_color_source: Option<BiomeColorCatalog>,
    biome_registry_order: Option<Vec<String>>,
    biome_colors: Option<BiomeColorCatalog>,
    biome_blend_radius: u32,
    /// Vanilla `Options.mipmapLevels`, retained with the generation so a
    /// server-pack reload rebuilds the atlas with the same startup choice.
    max_mipmap_levels: u32,
    /// Vanilla `Stitcher` pads sprites by `1 << mipLevel << clamp(bit - 1, 0,
    /// 4)`; the packer multiplies this base by `1 << mipLevel`, so the base is
    /// `1 << clamp(bit - 1, 0, 4)` under `ANISOTROPIC` filtering and 1
    /// otherwise (`SpriteLoader` passes bit 0 for the other modes).
    atlas_base_padding: u32,
    block_item_default_quads: CachedQuadsByName,
    block_item_property_quads: CachedQuadsByProperties,
    block_display_default_quads: CachedQuadsByName,
    block_display_property_quads: CachedQuadsByProperties,
    item_frame_border_quads: ItemFrameBorderQuadCache,
    block_translucent_material_cache: BlockTranslucentMaterialCache,
}

/// The immutable atlas and block-model inputs of terrain model baking, shared
/// with the mesh workers through the block render table.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainTextureCore {
    pub(super) atlas: TerrainTextureAtlas,
    indices: HashMap<String, u32>,
    block_models: Option<BlockModelCatalog>,
    content_registry: Arc<ContentRegistry>,
    content_projection: Option<Arc<ContentRegistryAssetProjection>>,
    transparencies: Vec<TerrainTransparency>,
    sprite_alphas: HashMap<String, SpriteAlpha>,
    fallback_index: u32,
    cutout_leaves: bool,
    /// Vanilla `Options.ao` (smooth lighting): `ModelBlockRenderer` is built
    /// with the option and gates every model's ambient occlusion on it.
    smooth_lighting: bool,
}

impl std::ops::Deref for TerrainTextureState {
    type Target = TerrainTextureCore;

    fn deref(&self) -> &TerrainTextureCore {
        &self.core
    }
}

#[derive(Debug, Clone)]
pub struct TerrainParticleTintCatalog {
    colormaps: Option<TerrainColorMaps>,
    biome_colors: Option<BiomeColorCatalog>,
    content_registry: Arc<ContentRegistry>,
    content_projection: Option<Arc<ContentRegistryAssetProjection>>,
}

impl Default for TerrainParticleTintCatalog {
    fn default() -> Self {
        Self {
            colormaps: None,
            biome_colors: None,
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            content_projection: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TerrainTintContext<'a> {
    colormaps: Option<&'a TerrainColorMaps>,
    biome_colors: Option<&'a BiomeColorCatalog>,
}

fn next_texture_generation() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl Default for TerrainTextureState {
    fn default() -> Self {
        Self::from_core(
            TerrainTextureCore {
                atlas: TerrainTextureAtlas::unit(),
                indices: HashMap::new(),
                block_models: None,
                content_registry: Arc::clone(ContentRegistry::vanilla()),
                content_projection: None,
                transparencies: vec![TerrainTransparency::OPAQUE],
                sprite_alphas: HashMap::new(),
                fallback_index: 0,
                cutout_leaves: true,
                smooth_lighting: true,
            },
            None,
            None,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
struct SpriteAlpha {
    width: u32,
    height: u32,
    transparency: TerrainTransparency,
    /// Two bits per texel (`bit 0 = has_transparent`, `bit 1 = has_translucent`),
    /// 32 texels per `u64`. Empty when every texel is opaque.
    words: Box<[u64]>,
}

const SPRITE_TRANSPARENT_BITS: u64 = 0x5555_5555_5555_5555;
const SPRITE_TRANSLUCENT_BITS: u64 = 0xAAAA_AAAA_AAAA_AAAA;

impl SpriteAlpha {
    fn from_image(image: &SpriteImage) -> Self {
        let texels = (image.width as usize).saturating_mul(image.height as usize);
        let mut words = vec![0_u64; texels.div_ceil(32)];
        if image.animation_frames_rgba.is_empty() {
            accumulate_sprite_alpha_words(&mut words, &image.rgba);
        } else {
            for rgba in &image.animation_frames_rgba {
                accumulate_sprite_alpha_words(&mut words, rgba);
            }
        }
        let any_non_opaque = words.iter().any(|&word| word != 0);
        Self {
            width: image.width,
            height: image.height,
            transparency: terrain_transparency(image.transparency),
            words: if any_non_opaque {
                words.into_boxed_slice()
            } else {
                Box::new([])
            },
        }
    }

    fn is_uniform_opaque(&self) -> bool {
        self.words.is_empty() && self.transparency == TerrainTransparency::OPAQUE
    }

    fn model_uv_transparency(&self, uv: [u8; 4]) -> TerrainTransparency {
        if uv == [0, 0, 16, 16] {
            return self.transparency;
        }
        let min_u = u32::from(uv[0].min(uv[2]));
        let min_v = u32::from(uv[1].min(uv[3]));
        let max_u = u32::from(uv[0].max(uv[2]));
        let max_v = u32::from(uv[1].max(uv[3]));
        let x0 = ((min_u * self.width) / 16).min(self.width);
        let y0 = ((min_v * self.height) / 16).min(self.height);
        let x1 = max_u
            .saturating_mul(self.width)
            .div_ceil(16)
            .min(self.width);
        let y1 = max_v
            .saturating_mul(self.height)
            .div_ceil(16)
            .min(self.height);
        if x0 >= x1 || y0 >= y1 || self.words.is_empty() {
            return TerrainTransparency::OPAQUE;
        }

        let mut bits = 0_u64;
        for y in y0..y1 {
            let row_base = (y * self.width) as usize;
            let start = row_base + x0 as usize;
            let end = row_base + x1 as usize;
            let first_word = start >> 5;
            let last_word = (end - 1) >> 5;
            let start_mask = !0_u64 << ((start & 31) << 1);
            let end_rem = end & 31;
            let end_mask = if end_rem == 0 {
                !0_u64
            } else {
                (1_u64 << (end_rem << 1)) - 1
            };
            if first_word == last_word {
                bits |= self.words[first_word] & start_mask & end_mask;
            } else {
                bits |= self.words[first_word] & start_mask;
                for &word in &self.words[first_word + 1..last_word] {
                    bits |= word;
                }
                bits |= self.words[last_word] & end_mask;
            }
            if (bits & SPRITE_TRANSPARENT_BITS) != 0 && (bits & SPRITE_TRANSLUCENT_BITS) != 0 {
                break;
            }
        }
        TerrainTransparency {
            has_transparent: (bits & SPRITE_TRANSPARENT_BITS) != 0,
            has_translucent: (bits & SPRITE_TRANSLUCENT_BITS) != 0,
        }
    }
}

fn accumulate_sprite_alpha_words(words: &mut [u64], rgba: &[u8]) {
    for (texel, pixel) in rgba.as_chunks::<4>().0.iter().enumerate() {
        let bits = match pixel[3] {
            255 => continue,
            0 => 0b01_u64,
            _ => 0b10_u64,
        };
        if let Some(word) = words.get_mut(texel >> 5) {
            *word |= bits << ((texel & 31) << 1);
        }
    }
}

impl TerrainTextureState {
    #[doc(hidden)]
    pub fn from_layout(
        layout: &AtlasLayout,
        block_models: Option<BlockModelCatalog>,
        colormaps: Option<TerrainColorMaps>,
        biome_colors: Option<BiomeColorCatalog>,
    ) -> Self {
        Self::from_layout_and_images(layout, &[], block_models, colormaps, biome_colors)
    }

    #[doc(hidden)]
    pub fn insert_block_item_quads_for_test(
        &self,
        block_name: &str,
        properties: BTreeMap<String, String>,
        quads: Arc<[crate::ItemModelQuad]>,
    ) {
        if properties.is_empty() {
            self.block_item_default_quads
                .borrow_mut()
                .insert(block_name.to_string(), Some(quads));
        } else {
            self.block_item_property_quads
                .borrow_mut()
                .insert((block_name.to_string(), properties), Some(quads));
        }
    }

    fn from_layout_and_images(
        layout: &AtlasLayout,
        images: &[SpriteImage],
        block_models: Option<BlockModelCatalog>,
        colormaps: Option<TerrainColorMaps>,
        biome_colors: Option<BiomeColorCatalog>,
    ) -> Self {
        let mut indices = HashMap::new();
        let mut rects = Vec::with_capacity(layout.sprites.len());
        let mut transparencies = Vec::with_capacity(layout.sprites.len());
        for (index, sprite) in layout.sprites.iter().enumerate() {
            indices.insert(sprite.id.clone(), index as u32);
            rects.push(terrain_uv_rect(layout, sprite));
            transparencies.push(terrain_transparency(sprite.transparency));
        }
        let sprite_alphas = images
            .iter()
            .filter_map(|image| {
                let alpha = SpriteAlpha::from_image(image);
                (!alpha.is_uniform_opaque() || !indices.contains_key(&image.id))
                    .then(|| (image.id.clone(), alpha))
            })
            .collect();
        let fallback_index = indices.get("minecraft:block/stone").copied().unwrap_or(0);
        Self::from_core(
            TerrainTextureCore {
                atlas: TerrainTextureAtlas {
                    rects,
                    fallback_index,
                },
                indices,
                block_models,
                content_registry: Arc::clone(ContentRegistry::vanilla()),
                content_projection: None,
                transparencies,
                sprite_alphas,
                fallback_index,
                cutout_leaves: true,
                smooth_lighting: true,
            },
            colormaps,
            biome_colors,
        )
    }

    fn from_core(
        core: TerrainTextureCore,
        colormaps: Option<TerrainColorMaps>,
        biome_colors: Option<BiomeColorCatalog>,
    ) -> Self {
        Self {
            generation: next_texture_generation(),
            core: Arc::new(core),
            colormaps,
            biome_color_source: biome_colors.clone(),
            biome_registry_order: None,
            biome_colors,
            biome_blend_radius: 2,
            max_mipmap_levels: 4,
            atlas_base_padding: 1,
            block_item_default_quads: RefCell::new(HashMap::new()),
            block_item_property_quads: RefCell::new(HashMap::new()),
            block_display_default_quads: RefCell::new(HashMap::new()),
            block_display_property_quads: RefCell::new(HashMap::new()),
            item_frame_border_quads: RefCell::new(HashMap::new()),
            block_translucent_material_cache: RefCell::new(hashbrown::HashMap::new()),
        }
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub fn with_cutout_leaves(mut self, cutout_leaves: bool) -> Self {
        Arc::make_mut(&mut self.core).cutout_leaves = cutout_leaves;
        self.generation = next_texture_generation();
        self
    }

    pub fn with_smooth_lighting(mut self, smooth_lighting: bool) -> Self {
        Arc::make_mut(&mut self.core).smooth_lighting = smooth_lighting;
        self.generation = next_texture_generation();
        self
    }

    pub fn smooth_lighting(&self) -> bool {
        self.smooth_lighting
    }

    pub fn with_biome_blend_radius(mut self, biome_blend_radius: u32) -> Self {
        self.biome_blend_radius = biome_blend_radius.min(7);
        self.generation = next_texture_generation();
        self
    }

    pub fn biome_blend_radius(&self) -> u32 {
        self.biome_blend_radius
    }

    pub fn with_max_mipmap_levels(mut self, max_mipmap_levels: u32) -> Self {
        self.max_mipmap_levels = max_mipmap_levels.min(4);
        self
    }

    pub fn max_mipmap_levels(&self) -> u32 {
        self.max_mipmap_levels
    }

    pub fn with_atlas_base_padding(mut self, atlas_base_padding: u32) -> Self {
        self.atlas_base_padding = atlas_base_padding;
        self
    }

    pub fn atlas_base_padding(&self) -> u32 {
        self.atlas_base_padding
    }

    pub fn cutout_leaves(&self) -> bool {
        self.cutout_leaves
    }

    pub fn with_content_registry(
        mut self,
        content_registry: Arc<ContentRegistry>,
        content_projection: Option<Arc<ContentRegistryAssetProjection>>,
    ) -> Self {
        self.set_content_registry(content_registry, content_projection);
        self
    }

    /// Synchronizes modded block definitions and pack asset projections onto this terrain texture
    /// state, bumping the generation and clearing derived block-quad caches when inputs change.
    pub fn set_content_registry(
        &mut self,
        content_registry: Arc<ContentRegistry>,
        content_projection: Option<Arc<ContentRegistryAssetProjection>>,
    ) -> bool {
        let registry_unchanged = Arc::ptr_eq(&self.core.content_registry, &content_registry)
            || self.core.content_registry.blocks() == content_registry.blocks();
        let projection_unchanged = match (&self.core.content_projection, &content_projection) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a.blocks == b.blocks,
            _ => false,
        };
        if registry_unchanged && projection_unchanged {
            return false;
        }
        let core = Arc::make_mut(&mut self.core);
        core.content_registry = content_registry;
        core.content_projection = content_projection;
        self.generation = next_texture_generation();
        self.block_item_default_quads.borrow_mut().clear();
        self.block_item_property_quads.borrow_mut().clear();
        self.block_display_default_quads.borrow_mut().clear();
        self.block_display_property_quads.borrow_mut().clear();
        self.block_translucent_material_cache.borrow_mut().clear();
        true
    }

    pub fn content_registry(&self) -> &Arc<ContentRegistry> {
        &self.core.content_registry
    }

    pub fn particle_sprite_id_for_block_state(&self, block_state_id: i32) -> Option<String> {
        self.particle_sprite_id_for_block_state_with_registry(
            block_state_id,
            &self.core.content_registry,
        )
    }

    pub fn particle_sprite_id_for_block_state_with_registry(
        &self,
        block_state_id: i32,
        content_registry: &ContentRegistry,
    ) -> Option<String> {
        if let Some(block_state) =
            bbb_protocol::block_state_types::vanilla_block_state_identity(block_state_id)
        {
            let model_particle = self
                .block_models
                .as_ref()
                .and_then(|models| {
                    models.block_render_model(&block_state.name, &block_state.properties)
                })
                .and_then(|model| model.particle_texture);
            let sprite_id =
                model_particle.unwrap_or_else(|| block_fallback_texture_id(&block_state.name));
            return if self.indices.contains_key(&sprite_id) {
                Some(sprite_id)
            } else {
                Some(block_fallback_texture_id(&block_state.name))
            };
        }
        let (spec, _) = content_registry.block_by_state_id(block_state_id)?;
        let block_name = spec.resource_id();
        let empty_props = BTreeMap::new();
        let model_particle = self
            .block_models
            .as_ref()
            .and_then(|models| models.block_render_model(block_name, &empty_props))
            .and_then(|model| model.particle_texture)
            .or_else(|| {
                self.content_projection
                    .as_ref()
                    .and_then(|projection| projection.block(block_name))
                    .and_then(|projected| projected.render_model.particle_texture.clone())
            });
        let sprite_id = model_particle.unwrap_or_else(|| block_fallback_texture_id(block_name));
        if self.indices.contains_key(&sprite_id) {
            Some(sprite_id)
        } else {
            let fallback = block_fallback_texture_id(block_name);
            if self.indices.contains_key(&fallback) {
                Some(fallback)
            } else if self
                .indices
                .contains_key(bbb_client_assets::MISSING_TEXTURE_ID)
            {
                Some(bbb_client_assets::MISSING_TEXTURE_ID.to_string())
            } else {
                Some(fallback)
            }
        }
    }

    /// Terrain-atlas index of [`Self::particle_sprite_id_for_block_state`];
    /// `u32::MAX` when the fallback texture is not in the atlas either.
    pub fn particle_sprite_index_for_block_state(&self, block_state_id: i32) -> Option<u32> {
        let sprite_id = self.particle_sprite_id_for_block_state(block_state_id)?;
        Some(self.indices.get(&sprite_id).copied().unwrap_or(u32::MAX))
    }

    pub fn particle_sprite_index_for_block_state_with_registry(
        &self,
        block_state_id: i32,
        content_registry: &ContentRegistry,
    ) -> Option<u32> {
        let sprite_id = self
            .particle_sprite_id_for_block_state_with_registry(block_state_id, content_registry)?;
        Some(self.indices.get(&sprite_id).copied().unwrap_or(u32::MAX))
    }

    /// Retains the world's resolved biome colors when the reloaded resources bake identical meshes.
    pub fn preserve_mesh_inputs_from(&mut self, previous: &mut Self) -> bool {
        if self.core != previous.core
            || self.colormaps != previous.colormaps
            || self.biome_color_source != previous.biome_color_source
            || self.biome_blend_radius != previous.biome_blend_radius
        {
            return false;
        }
        self.biome_registry_order = previous.biome_registry_order.take();
        self.biome_colors = previous.biome_colors.take();
        true
    }

    pub fn particle_tint_catalog(&self) -> TerrainParticleTintCatalog {
        TerrainParticleTintCatalog {
            colormaps: self.colormaps.clone(),
            biome_colors: self.biome_colors.clone(),
            content_registry: Arc::clone(&self.core.content_registry),
            content_projection: self.core.content_projection.clone(),
        }
    }

    pub fn synchronize_biome_color_profiles<'a, I>(
        &mut self,
        resource_ids: Option<I>,
        mut projected_profile: impl FnMut(i32, &str) -> Option<BiomeColorProfile>,
    ) -> bool
    where
        I: ExactSizeIterator<Item = &'a str> + Clone,
    {
        let order_is_unchanged = match resource_ids.clone() {
            None => self.biome_registry_order.is_none(),
            Some(ids) => self.biome_registry_order.as_ref().is_some_and(|current| {
                current.len() == ids.len() && current.iter().map(String::as_str).eq(ids)
            }),
        };
        if order_is_unchanged {
            return false;
        }
        let next_order = resource_ids.map(|ids| ids.map(str::to_string).collect::<Vec<_>>());
        let next = self
            .biome_color_source
            .as_ref()
            .map(|source| match next_order.as_deref() {
                None => source.clone(),
                Some(resource_ids) => {
                    BiomeColorCatalog::new(resource_ids.iter().enumerate().filter_map(
                        |(holder_id, resource_id)| match projected_profile(
                            holder_id as i32,
                            resource_id.as_str(),
                        ) {
                            Some(profile) => Some(profile),
                            None => {
                                let mut profile = source.profile_by_name(resource_id)?.clone();
                                profile.id = holder_id as i32;
                                Some(profile)
                            }
                        },
                    ))
                }
            });
        self.biome_registry_order = next_order;
        if self.biome_colors == next {
            return false;
        }
        self.biome_colors = next;
        self.generation = next_texture_generation();
        true
    }

    pub fn terrain_particle_tint_color_for_block_state(
        &self,
        block_state_id: i32,
    ) -> Option<[f32; 4]> {
        self.terrain_particle_tint_color_for_block_state_at(block_state_id, None, None)
    }

    pub fn terrain_particle_tint_color_for_block_state_at(
        &self,
        block_state_id: i32,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> Option<[f32; 4]> {
        if let Some(block_state) =
            bbb_protocol::block_state_types::vanilla_block_state_identity(block_state_id)
        {
            return self
                .block_colors_layer0_terrain_particle_tint(
                    &block_state.name,
                    &block_state.properties,
                    biome_id,
                    position,
                )
                .map(terrain_particle_rgb_06);
        }
        self.mod_block_fallback_particle_tint(block_state_id)
            .map(terrain_particle_rgb_06)
    }

    pub fn falling_dust_block_tint_color_for_block_state(
        &self,
        block_state_id: i32,
    ) -> Option<[f32; 4]> {
        self.falling_dust_block_tint_color_for_block_state_at(block_state_id, None, None)
    }

    pub fn falling_dust_block_tint_color_for_block_state_at(
        &self,
        block_state_id: i32,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> Option<[f32; 4]> {
        if let Some(block_state) =
            bbb_protocol::block_state_types::vanilla_block_state_identity(block_state_id)
        {
            return self
                .block_colors_layer0_terrain_particle_tint(
                    &block_state.name,
                    &block_state.properties,
                    biome_id,
                    position,
                )
                .map(terrain_tint_particle_rgb);
        }
        self.mod_block_fallback_particle_tint(block_state_id)
            .map(terrain_tint_particle_rgb)
    }

    fn mod_block_fallback_particle_tint(&self, block_state_id: i32) -> Option<TerrainTint> {
        let (spec, _) = self
            .core
            .content_registry
            .block_by_state_id(block_state_id)?;
        self.core.mod_block_fallback_tint(spec.resource_id())
    }

    pub fn destroy_stage_uv_rect(&self, stage: u8) -> Option<TerrainUvRect> {
        let texture_id = format!("minecraft:block/destroy_stage_{}", stage.min(9));
        let index = *self.indices.get(&texture_id)?;
        self.atlas.rects.get(index as usize).copied()
    }

    /// Resolves one blocks-atlas sprite like `TextureAtlas.getSprite`: an
    /// undeclared id uses the atlas `missingno` sprite when present, then the
    /// terrain owner's established fallback as a final malformed-atlas guard.
    pub fn sprite_uv_rect(&self, texture_id: &str) -> TerrainUvRect {
        let index = self
            .indices
            .get(texture_id)
            .or_else(|| self.indices.get("minecraft:missingno"))
            .copied()
            .unwrap_or(self.fallback_index);
        self.atlas
            .rects
            .get(index as usize)
            .copied()
            .or_else(|| self.atlas.rects.get(self.fallback_index as usize).copied())
            .unwrap_or(TerrainUvRect::UNIT)
    }

    pub fn biome_has_precipitation(&self, biome_id: Option<i32>) -> Option<bool> {
        Some(self.biome_profile(biome_id)?.has_precipitation)
    }

    pub fn biome_temperature(&self, biome_id: Option<i32>) -> Option<f32> {
        Some(self.biome_profile(biome_id)?.temperature)
    }

    pub fn biome_temperature_modifier(
        &self,
        biome_id: Option<i32>,
    ) -> Option<BiomeTemperatureModifier> {
        Some(self.biome_profile(biome_id)?.temperature_modifier)
    }

    pub fn with_biome_colors_for_tests(biome_colors: BiomeColorCatalog) -> Self {
        Self {
            biome_color_source: Some(biome_colors.clone()),
            biome_colors: Some(biome_colors),
            generation: next_texture_generation(),
            ..Self::default()
        }
    }

    /// The terrain render shape for a single block state at `position`, projected from the same
    /// `block_render_data` source the chunk mesher uses (position feeds the model-variant seed, so
    /// the crumbling decal follows the exact rotated model the chunk drew). Biome tint inputs are
    /// omitted because they never affect geometry.
    pub fn block_render_shape(
        &self,
        block_name: Option<&str>,
        properties: &BTreeMap<String, String>,
        material: bbb_render_types::TerrainMaterialClass,
        position: BlockRenderPosition,
    ) -> TerrainRenderShape {
        let (_, _, _, render_shape, _) =
            self.block_render_data(block_name, properties, material, None, Some(position), None);
        render_shape
    }

    /// Mirrors Vanilla `BlockStateModel.hasMaterialFlag(BakedQuad.FLAG_TRANSLUCENT)` using the
    /// current immutable block-model catalog and the same atlas-alpha projection as terrain mesh
    /// baking. Results are cached per `(block_name, properties)` so crosshair targeting stays O(1).
    pub fn block_model_has_translucent_material(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
        _material: bbb_render_types::TerrainMaterialClass,
    ) -> bool {
        if let Some(&cached) = self
            .block_translucent_material_cache
            .borrow()
            .get(&BlockMaterialCacheKey(block_name, properties))
        {
            return cached;
        }
        let has_translucent = self.block_models.as_ref().is_some_and(|models| {
            // Fast path: check cached `force_translucent` flags before cloning combined geometry.
            if models.block_material_force_translucent(block_name, properties) == Some(true) {
                return true;
            }
            models
                .block_material_models(block_name, properties)
                .is_some_and(|models| {
                    models.into_iter().any(|model| {
                        let fallback_transparency =
                            self.face_texture_transparencies(&model.face_textures);
                        let shape = self.terrain_render_shape(
                            model.shape,
                            [self.fallback_index; 6],
                            [TerrainTint::WHITE; 6],
                            fallback_transparency,
                            &|_| TerrainTint::WHITE,
                        );
                        render_shape_has_translucent_material(&shape, fallback_transparency)
                    })
                })
        });
        let mut cache = self.block_translucent_material_cache.borrow_mut();
        if cache.len() < MAX_BLOCK_TRANSLUCENT_MATERIAL_CACHE {
            cache.insert(
                (block_name.to_string(), properties.clone()),
                has_translucent,
            );
        }
        has_translucent
    }

    pub(super) fn block_render_data(
        &self,
        block_name: Option<&str>,
        properties: &BTreeMap<String, String>,
        material: bbb_render_types::TerrainMaterialClass,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
        blend: Option<&BiomeBlend>,
    ) -> (
        [u32; 6],
        [TerrainTint; 6],
        [TerrainTransparency; 6],
        TerrainRenderShape,
        bool,
    ) {
        let mut state = self.state_render(block_name, properties, material, None, false);
        let combination = state.combination(position);
        let rule = state.tint;
        let model = self.state_model(&mut state, combination);
        let (tint, shape) = model.tinted(self.resolve_tint(rule, biome_id, position, blend));
        (
            model.texture_indices,
            tint,
            model.face_transparency,
            shape,
            model.ambient_occlusion,
        )
    }

    /// One model variant of `block_name` chosen by `seed`, as the terrain mesher would see it.
    fn seeded_model_render(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
        seed: Option<i64>,
        force_opaque: bool,
    ) -> (TerrainTintRule, TerrainModelRender) {
        let mut state = self.state_render(
            Some(block_name),
            properties,
            bbb_render_types::TerrainMaterialClass::Opaque,
            None,
            force_opaque,
        );
        let combination = state.combination_for_seed(seed);
        let rule = state.tint;
        (rule, self.state_model(&mut state, combination).clone())
    }

    /// Bakes the **item form** of a block — the block's model rendered as a held / dropped / framed /
    /// inventory item — into item-model quads in vanilla `0..=16` model space, sampling the blocks atlas.
    /// Returns `None` when no real block model exists for `block_name`, so a non-block item (apple, stick)
    /// is left to the flat-item path; a `Cross` foliage block bakes to an empty `Vec` (it renders as a
    /// flat item, not a 3D cross), which the caller treats the same as "not a 3D item". Biome tints use
    /// the default (no-position) climate, matching vanilla's fixed inventory grass/foliage color.
    pub fn block_item_quads(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<Arc<[crate::ItemModelQuad]>> {
        if properties.is_empty() {
            if let Some(cached) = self.block_item_default_quads.borrow().get(block_name) {
                return cached.clone();
            }
            let quads = self.build_block_item_quads(block_name, properties);
            self.block_item_default_quads
                .borrow_mut()
                .insert(block_name.to_string(), quads.clone());
            return quads;
        }

        let key = (block_name.to_string(), properties.clone());
        if let Some(cached) = self.block_item_property_quads.borrow().get(&key) {
            return cached.clone();
        }
        let quads = self.build_block_item_quads(block_name, properties);
        self.block_item_property_quads
            .borrow_mut()
            .insert(key, quads.clone());
        quads
    }

    /// Bakes the block-state model used by `BlockDisplayRenderer`. Unlike an
    /// inventory block item, Vanilla resolves weighted variants with the fixed
    /// `BlockModelResolver.MODEL_SEED` (`42`) and applies no item display
    /// transform.
    pub fn block_display_quads(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<Arc<[crate::ItemModelQuad]>> {
        if properties.is_empty() {
            if let Some(cached) = self.block_display_default_quads.borrow().get(block_name) {
                return cached.clone();
            }
            let quads = self.build_block_display_quads(block_name, properties);
            self.block_display_default_quads
                .borrow_mut()
                .insert(block_name.to_string(), quads.clone());
            return quads;
        }

        let key = (block_name.to_string(), properties.clone());
        if let Some(cached) = self.block_display_property_quads.borrow().get(&key) {
            return cached.clone();
        }
        let quads = self.build_block_display_quads(block_name, properties);
        self.block_display_property_quads
            .borrow_mut()
            .insert(key, quads.clone());
        quads
    }

    /// Bakes the world form consumed by vanilla `MovingBlockRenderState`.
    /// Weighted model selection uses the falling block's immutable start
    /// position, while biome tint samples the current moving-block position;
    /// cardinal lighting remains a separate per-dimension input.
    pub fn moving_block_quads(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
        random_seed_position: BlockRenderPosition,
        block_position: BlockRenderPosition,
        biome_id: Option<i32>,
        cardinal_lighting: crate::terrain::TerrainCardinalLighting,
    ) -> Option<Arc<[crate::ItemModelQuad]>> {
        let (rule, model) = self.seeded_model_render(
            block_name,
            properties,
            Some(block_model_seed(random_seed_position)),
            true,
        );
        if !model.has_model {
            return None;
        }
        let (tint, shape) =
            model.tinted(self.resolve_tint(rule, biome_id, Some(block_position), None));
        Some(Arc::from(crate::terrain::bake_moving_block_quads(
            &shape,
            model.texture_indices,
            tint,
            model.face_transparency,
            cardinal_lighting,
            &self.atlas,
        )))
    }

    fn build_block_display_quads(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<Arc<[crate::ItemModelQuad]>> {
        let (rule, model) = self.seeded_model_render(block_name, properties, Some(42), false);
        if !model.has_model {
            return None;
        }
        let (tint, shape) = model.tinted(self.resolve_tint(rule, None, None, None));
        Some(Arc::from(crate::terrain::bake_block_item_quads(
            &shape,
            model.texture_indices,
            tint,
            &self.atlas,
        )))
    }

    fn build_block_item_quads(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<Arc<[crate::ItemModelQuad]>> {
        let (rule, model) = self.seeded_model_render(block_name, properties, None, false);
        if !model.has_model {
            return None;
        }
        let (tint, shape) = model.tinted(self.resolve_tint(rule, None, None, None));
        Some(Arc::from(crate::terrain::bake_block_item_quads(
            &shape,
            model.texture_indices,
            tint,
            &self.atlas,
        )))
    }

    /// The 3D item-frame border (vanilla `block/item_frame` / `block/glow_item_frame`, or the
    /// `*_map` variants): four `birch_planks` wood bars plus a `back` panel showing the `item_frame` /
    /// `glow_item_frame` texture, baked over the blocks atlas into item-model quads in `0..=16` model
    /// space while preserving the vanilla templates' fractional `15.5` / `15.001` depths.
    pub fn item_frame_border_quads(&self, glow: bool, map: bool) -> Arc<[crate::ItemModelQuad]> {
        if let Some(quads) = self.item_frame_border_quads.borrow().get(&(glow, map)) {
            return Arc::clone(quads);
        }
        let quads: Arc<[crate::ItemModelQuad]> =
            Arc::from(self.build_item_frame_border_quads(glow, map));
        self.item_frame_border_quads
            .borrow_mut()
            .insert((glow, map), Arc::clone(&quads));
        quads
    }

    fn build_item_frame_border_quads(&self, glow: bool, map: bool) -> Vec<crate::ItemModelQuad> {
        let wood = self.texture_index("minecraft:block/birch_planks");
        let back = self.texture_index(if glow {
            "minecraft:block/glow_item_frame"
        } else {
            "minecraft:block/item_frame"
        });
        if map {
            return self.item_frame_map_border_quads(wood, back);
        }
        self.item_frame_normal_border_quads(wood, back)
    }

    fn item_frame_normal_border_quads(&self, wood: u32, back: u32) -> Vec<crate::ItemModelQuad> {
        let quads = [
            // Back panel: north + south faces, the `back` texture.
            frame_border_box_quads(
                [3.0, 3.0, 15.5],
                [13.0, 13.0, 16.0],
                [false, false, true, true, false, false],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [3, 3, 13, 13],
                    [3, 3, 13, 13],
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                ],
                [back; 6],
            ),
            // Bottom wood bar.
            frame_border_box_quads(
                [2.0, 2.0, 15.0],
                [14.0, 3.0, 16.0],
                [true; 6],
                [
                    [2, 0, 14, 1],
                    [2, 15, 14, 16],
                    [2, 13, 14, 14],
                    [2, 13, 14, 14],
                    [15, 13, 16, 14],
                    [0, 13, 1, 14],
                ],
                [wood; 6],
            ),
            // Top wood bar.
            frame_border_box_quads(
                [2.0, 13.0, 15.0],
                [14.0, 14.0, 16.0],
                [true; 6],
                [
                    [2, 0, 14, 1],
                    [2, 15, 14, 16],
                    [2, 2, 14, 3],
                    [2, 2, 14, 3],
                    [15, 2, 16, 3],
                    [0, 2, 1, 3],
                ],
                [wood; 6],
            ),
            // Left wood bar (no up/down faces).
            frame_border_box_quads(
                [2.0, 3.0, 15.0],
                [3.0, 13.0, 16.0],
                [false, false, true, true, true, true],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [13, 3, 14, 13],
                    [2, 3, 3, 13],
                    [15, 3, 16, 13],
                    [0, 3, 1, 13],
                ],
                [wood; 6],
            ),
            // Right wood bar (no up/down faces).
            frame_border_box_quads(
                [13.0, 3.0, 15.0],
                [14.0, 13.0, 16.0],
                [false, false, true, true, true, true],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [2, 3, 3, 13],
                    [13, 3, 14, 13],
                    [15, 3, 16, 13],
                    [0, 3, 1, 13],
                ],
                [wood; 6],
            ),
        ]
        .into_iter()
        .flatten()
        .collect();
        crate::terrain::bake_block_item_quads(
            &TerrainRenderShape::Quads(quads),
            [self.fallback_index; 6],
            [TerrainTint::WHITE; 6],
            &self.atlas,
        )
    }

    fn item_frame_map_border_quads(&self, wood: u32, back: u32) -> Vec<crate::ItemModelQuad> {
        let quads = [
            frame_border_box_quads(
                [1.0, 1.0, 15.001],
                [15.0, 15.0, 16.0],
                [false, false, true, true, false, false],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [1, 1, 15, 15],
                    [1, 1, 15, 15],
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                ],
                [back; 6],
            ),
            frame_border_box_quads(
                [0.0, 0.0, 15.001],
                [16.0, 1.0, 16.0],
                [true; 6],
                [
                    [0, 0, 16, 1],
                    [0, 15, 16, 16],
                    [0, 15, 16, 16],
                    [0, 15, 16, 16],
                    [15, 15, 16, 16],
                    [0, 15, 1, 16],
                ],
                [wood; 6],
            ),
            frame_border_box_quads(
                [0.0, 15.0, 15.001],
                [16.0, 16.0, 16.0],
                [true; 6],
                [
                    [0, 0, 16, 1],
                    [0, 15, 16, 16],
                    [0, 0, 16, 1],
                    [0, 0, 16, 1],
                    [15, 0, 16, 1],
                    [0, 0, 1, 1],
                ],
                [wood; 6],
            ),
            frame_border_box_quads(
                [0.0, 1.0, 15.001],
                [1.0, 15.0, 16.0],
                [false, false, true, true, true, true],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [15, 1, 16, 15],
                    [0, 1, 1, 15],
                    [15, 1, 16, 15],
                    [0, 1, 1, 15],
                ],
                [wood; 6],
            ),
            frame_border_box_quads(
                [15.0, 1.0, 15.001],
                [16.0, 15.0, 16.0],
                [false, false, true, true, true, true],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [0, 1, 1, 15],
                    [15, 1, 16, 15],
                    [15, 1, 16, 15],
                    [0, 1, 1, 15],
                ],
                [wood; 6],
            ),
        ]
        .into_iter()
        .flatten()
        .collect();
        crate::terrain::bake_block_item_quads(
            &TerrainRenderShape::Quads(quads),
            [self.fallback_index; 6],
            [TerrainTint::WHITE; 6],
            &self.atlas,
        )
    }

    #[cfg(test)]
    fn block_tint(
        &self,
        block_name: &str,
        material: bbb_render_types::TerrainMaterialClass,
        tint_index: Option<i32>,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> TerrainTint {
        self.block_tint_blended(block_name, material, tint_index, biome_id, position, None)
    }

    #[cfg(test)]
    fn block_tint_blended(
        &self,
        block_name: &str,
        _material: bbb_render_types::TerrainMaterialClass,
        tint_index: Option<i32>,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
        blend: Option<&BiomeBlend>,
    ) -> TerrainTint {
        let rule = TerrainTintRule::for_block(block_name);
        if rule.tints(tint_index) {
            self.resolve_tint(rule, biome_id, position, blend)
        } else {
            TerrainTint::WHITE
        }
    }

    fn block_colors_layer0_terrain_particle_tint(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> Option<TerrainTint> {
        self.tint_context()
            .block_colors_layer0_terrain_particle_tint(
                block_name, properties, biome_id, position, None,
            )
    }

    fn biome_profile(&self, biome_id: Option<i32>) -> Option<&BiomeColorProfile> {
        self.biome_colors.as_ref()?.profile(biome_id?)
    }

    fn tint_context(&self) -> TerrainTintContext<'_> {
        TerrainTintContext {
            colormaps: self.colormaps.as_ref(),
            biome_colors: self.biome_colors.as_ref(),
        }
    }
}

fn block_name_is_leaves(block_name: &str) -> bool {
    bbb_protocol::block_types::vanilla_block_type_for_resource_id(block_name)
        .and_then(|block| bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id))
        .and_then(bbb_protocol::block_state_types::vanilla_block_state_render_facts)
        .is_some_and(|facts| facts.leaves_block())
}

impl TerrainTextureCore {
    pub fn forces_opaque_leaves(&self, block_name: &str) -> bool {
        !self.cutout_leaves && block_name_is_leaves(block_name)
    }

    pub(super) fn mod_block_fallback_tint(&self, block_name: &str) -> Option<TerrainTint> {
        let projected = self.content_projection.as_ref()?.block(block_name)?;
        if projected.resolved_from_pack {
            return None;
        }
        let has_pack_texture = projected.texture_ids.iter().any(|texture_id| {
            texture_id != bbb_client_assets::MISSING_TEXTURE_ID
                && self.indices.contains_key(texture_id)
        });
        if has_pack_texture {
            return None;
        }
        let rgb = projected.fallback_tint.rgb;
        Some(TerrainTint::from_rgb_u8(
            as_vanilla_argb_channel(rgb[0]),
            as_vanilla_argb_channel(rgb[1]),
            as_vanilla_argb_channel(rgb[2]),
        ))
    }

    fn texture_index(&self, texture_id: &str) -> u32 {
        self.indices
            .get(texture_id)
            .copied()
            .unwrap_or(self.fallback_index)
    }

    fn texture_transparency(&self, texture_id: &str) -> TerrainTransparency {
        self.texture_index_transparency(self.texture_index(texture_id))
    }

    fn texture_uv_transparency(&self, texture_id: &str, uv: [u8; 4]) -> TerrainTransparency {
        self.sprite_alphas
            .get(texture_id)
            .map(|alpha| alpha.model_uv_transparency(uv))
            .unwrap_or_else(|| self.texture_transparency(texture_id))
    }

    fn texture_index_transparency(&self, texture_index: u32) -> TerrainTransparency {
        self.transparencies
            .get(texture_index as usize)
            .copied()
            .or_else(|| {
                self.transparencies
                    .get(self.fallback_index as usize)
                    .copied()
            })
            .unwrap_or(TerrainTransparency::OPAQUE)
    }

    fn fluid_texture_indices(&self, block_name: &str) -> Option<[u32; 6]> {
        let (still, flowing) = match block_name {
            "minecraft:water" => ("minecraft:block/water_still", "minecraft:block/water_flow"),
            "minecraft:lava" => ("minecraft:block/lava_still", "minecraft:block/lava_flow"),
            _ => return None,
        };
        let still = self.texture_index(still);
        let flowing = self.texture_index(flowing);
        Some([still, still, flowing, flowing, flowing, flowing])
    }

    fn fluid_kind_texture_indices(&self, kind: TerrainFluidKind) -> [u32; 6] {
        let block_name = match kind {
            TerrainFluidKind::Water => "minecraft:water",
            TerrainFluidKind::Lava => "minecraft:lava",
        };
        self.fluid_texture_indices(block_name)
            .unwrap_or([self.fallback_index; 6])
    }

    pub(super) fn fluid_overlay_texture_index(&self, kind: TerrainFluidKind) -> Option<u32> {
        match kind {
            TerrainFluidKind::Water => Some(self.texture_index("minecraft:block/water_overlay")),
            TerrainFluidKind::Lava => None,
        }
    }

    fn face_texture_indices(&self, face_textures: &BlockFaceTextures) -> [u32; 6] {
        std::array::from_fn(|index| self.texture_index(&face_textures.textures[index]))
    }

    fn face_texture_transparencies(
        &self,
        face_textures: &BlockFaceTextures,
    ) -> [TerrainTransparency; 6] {
        std::array::from_fn(|index| {
            self.texture_transparency(&face_textures.textures[index])
                .or(force_translucent(face_textures.force_translucent[index]))
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn terrain_render_shape_for_block(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
        material: bbb_render_types::TerrainMaterialClass,
        model_shape: BlockModelShape,
        fallback_texture_indices: [u32; 6],
        fallback_tint: [TerrainTint; 6],
        fallback_transparency: [TerrainTransparency; 6],
        tint: &dyn Fn(Option<i32>) -> TerrainTint,
    ) -> TerrainRenderShape {
        if matches!(material, bbb_render_types::TerrainMaterialClass::Fluid) {
            if let Some(shape) = fluid_render_shape(block_name, properties) {
                return shape;
            }
        }
        self.terrain_render_shape(
            model_shape,
            fallback_texture_indices,
            fallback_tint,
            fallback_transparency,
            tint,
        )
    }

    fn terrain_render_shape(
        &self,
        shape: BlockModelShape,
        fallback_texture_indices: [u32; 6],
        fallback_tint: [TerrainTint; 6],
        fallback_transparency: [TerrainTransparency; 6],
        tint: &dyn Fn(Option<i32>) -> TerrainTint,
    ) -> TerrainRenderShape {
        match shape {
            BlockModelShape::Cross {
                shade,
                light_emission,
            } => TerrainRenderShape::Cross {
                shade,
                light_emission,
            },
            BlockModelShape::Crosses(model_crosses) => TerrainRenderShape::Crosses(
                model_crosses
                    .into_iter()
                    .map(|model_cross| TerrainCross {
                        texture_indices: self
                            .model_cross_texture_indices(&model_cross, fallback_texture_indices),
                        tint: std::array::from_fn(|index| {
                            if model_cross.face_textures[index].is_some() {
                                tint(model_cross.face_tint_indices[index])
                            } else {
                                fallback_tint[index]
                            }
                        }),
                        face_transparency: self
                            .model_cross_face_transparencies(&model_cross, fallback_transparency),
                        shade: model_cross.shade,
                        light_emission: model_cross.light_emission,
                    })
                    .collect(),
            ),
            BlockModelShape::Box(model_box) => TerrainRenderShape::Box {
                from: model_box.from,
                to: model_box.to,
                face_present: model_box.face_present,
                face_uvs: model_box.face_uvs,
                face_uv_rotations: model_box.face_uv_rotations,
                face_shade: model_box.face_shade,
                face_light_emission: model_box.face_light_emission,
                face_cull: model_box_cull_faces(model_box.face_cull),
                face_transparency: self
                    .model_box_face_transparencies(&model_box, fallback_transparency),
            },
            BlockModelShape::Boxes(model_boxes) => TerrainRenderShape::Boxes(
                model_boxes
                    .into_iter()
                    .map(|model_box| crate::terrain::TerrainBox {
                        from: model_box.from,
                        to: model_box.to,
                        face_present: model_box.face_present,
                        face_uvs: model_box.face_uvs,
                        face_uv_rotations: model_box.face_uv_rotations,
                        face_shade: model_box.face_shade,
                        face_light_emission: model_box.face_light_emission,
                        face_cull: model_box_cull_faces(model_box.face_cull),
                        texture_indices: self
                            .model_box_texture_indices(&model_box, fallback_texture_indices),
                        tint: std::array::from_fn(|index| {
                            if model_box.face_present[index] {
                                tint(model_box.face_tint_indices[index])
                            } else {
                                fallback_tint[index]
                            }
                        }),
                        face_transparency: self
                            .model_box_face_transparencies(&model_box, fallback_transparency),
                    })
                    .collect(),
            ),
            BlockModelShape::Quads(model_quads) => TerrainRenderShape::Quads(
                model_quads
                    .into_iter()
                    .map(|model_quad| TerrainQuad {
                        corners: model_quad.corners,
                        normal: model_quad.normal,
                        uvs: model_quad.uvs,
                        cull: model_quad.cull.map(model_face_to_terrain_face),
                        texture_index: model_quad
                            .texture
                            .as_deref()
                            .map(|texture| self.texture_index(texture))
                            .unwrap_or(fallback_texture_indices[model_quad.face.index()]),
                        tint: if model_quad.texture.is_some() {
                            tint(model_quad.tint_index)
                        } else {
                            fallback_tint[model_quad.face.index()]
                        },
                        transparency: self
                            .model_quad_transparency(&model_quad, fallback_transparency)
                            .or(force_translucent(model_quad.force_translucent)),
                        shade: model_quad.shade,
                        light_emission: model_quad.light_emission,
                    })
                    .collect(),
            ),
            BlockModelShape::Cube => TerrainRenderShape::Cube,
        }
    }

    fn model_box_texture_indices(
        &self,
        model_box: &bbb_client_assets::BlockModelBox,
        fallback: [u32; 6],
    ) -> [u32; 6] {
        std::array::from_fn(|index| {
            model_box.face_textures[index]
                .as_deref()
                .map(|texture| self.texture_index(texture))
                .unwrap_or(fallback[index])
        })
    }

    fn model_cross_texture_indices(
        &self,
        model_cross: &bbb_client_assets::BlockModelCross,
        fallback: [u32; 6],
    ) -> [u32; 6] {
        std::array::from_fn(|index| {
            model_cross.face_textures[index]
                .as_deref()
                .map(|texture| self.texture_index(texture))
                .unwrap_or(fallback[index])
        })
    }

    fn model_box_face_transparencies(
        &self,
        model_box: &bbb_client_assets::BlockModelBox,
        fallback: [TerrainTransparency; 6],
    ) -> [TerrainTransparency; 6] {
        std::array::from_fn(|index| {
            model_box.face_textures[index]
                .as_deref()
                .map(|texture| self.texture_uv_transparency(texture, model_box.face_uvs[index]))
                .unwrap_or(fallback[index])
                .or(force_translucent(model_box.face_force_translucent[index]))
        })
    }

    fn model_cross_face_transparencies(
        &self,
        model_cross: &bbb_client_assets::BlockModelCross,
        fallback: [TerrainTransparency; 6],
    ) -> [TerrainTransparency; 6] {
        std::array::from_fn(|index| {
            model_cross.face_textures[index]
                .as_deref()
                .map(|texture| self.texture_transparency(texture))
                .unwrap_or(fallback[index])
                .or(force_translucent(model_cross.face_force_translucent[index]))
        })
    }

    fn model_quad_transparency(
        &self,
        model_quad: &bbb_client_assets::BlockModelQuad,
        fallback: [TerrainTransparency; 6],
    ) -> TerrainTransparency {
        model_quad
            .texture
            .as_deref()
            .map(|texture| {
                quad_uv_crop(model_quad.uvs)
                    .map(|uv| self.texture_uv_transparency(texture, uv))
                    .unwrap_or_else(|| self.texture_transparency(texture))
            })
            .unwrap_or(fallback[model_quad.face.index()])
    }
}

impl TerrainParticleTintCatalog {
    pub fn client_leaf_tint_color(
        &self,
        block_name: &str,
        biome_id: Option<i32>,
        blend: Option<&BiomeBlend>,
    ) -> [f32; 4] {
        let tint = match block_name {
            "minecraft:spruce_leaves" => TerrainTint::from_rgb_u8(0x61, 0x99, 0x61),
            "minecraft:birch_leaves" => TerrainTint::from_rgb_u8(0x80, 0xa7, 0x55),
            name if is_foliage_tinted_block(name) => {
                self.tint_context().foliage_tint(biome_id, blend)
            }
            _ => TerrainTint::WHITE,
        };
        terrain_tint_particle_rgb(tint)
    }

    pub fn terrain_particle_tint_color_for_block_state(
        &self,
        block_state_id: i32,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
        blend: Option<&BiomeBlend>,
    ) -> Option<[f32; 4]> {
        if let Some(block_state) =
            bbb_protocol::block_state_types::vanilla_block_state_identity(block_state_id)
        {
            return self
                .tint_context()
                .block_colors_layer0_terrain_particle_tint(
                    &block_state.name,
                    &block_state.properties,
                    biome_id,
                    position,
                    blend,
                )
                .map(terrain_particle_rgb_06);
        }
        self.mod_block_fallback_tint(block_state_id)
            .map(terrain_particle_rgb_06)
    }

    pub fn block_state_biome_blend_center(
        &self,
        block_state_id: i32,
        position: BlockRenderPosition,
    ) -> Option<BlockRenderPosition> {
        terrain_particle_biome_blend_center(block_state_id, position)
    }

    pub fn falling_dust_block_tint_color_for_block_state(
        &self,
        block_state_id: i32,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
        blend: Option<&BiomeBlend>,
    ) -> Option<[f32; 4]> {
        if let Some(block_state) =
            bbb_protocol::block_state_types::vanilla_block_state_identity(block_state_id)
        {
            return self
                .tint_context()
                .block_colors_layer0_terrain_particle_tint(
                    &block_state.name,
                    &block_state.properties,
                    biome_id,
                    position,
                    blend,
                )
                .map(terrain_tint_particle_rgb);
        }
        self.mod_block_fallback_tint(block_state_id)
            .map(terrain_tint_particle_rgb)
    }

    fn mod_block_fallback_tint(&self, block_state_id: i32) -> Option<TerrainTint> {
        let (spec, _) = self.content_registry.block_by_state_id(block_state_id)?;
        let projected = self
            .content_projection
            .as_ref()?
            .block(spec.resource_id())?;
        if projected.resolved_from_pack {
            return None;
        }
        let has_pack_texture = projected
            .texture_ids
            .iter()
            .any(|texture_id| texture_id != bbb_client_assets::MISSING_TEXTURE_ID);
        if has_pack_texture {
            return None;
        }
        let rgb = projected.fallback_tint.rgb;
        Some(TerrainTint::from_rgb_u8(
            as_vanilla_argb_channel(rgb[0]),
            as_vanilla_argb_channel(rgb[1]),
            as_vanilla_argb_channel(rgb[2]),
        ))
    }

    fn tint_context(&self) -> TerrainTintContext<'_> {
        TerrainTintContext {
            colormaps: self.colormaps.as_ref(),
            biome_colors: self.biome_colors.as_ref(),
        }
    }
}

impl TerrainTintContext<'_> {
    fn resolve_tint(
        &self,
        source: TerrainTintSource,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
        blend: Option<&BiomeBlend>,
    ) -> TerrainTint {
        match source {
            TerrainTintSource::White => TerrainTint::WHITE,
            TerrainTintSource::Fixed(tint) => tint,
            TerrainTintSource::Grass => self.grass_tint(biome_id, position, blend),
            TerrainTintSource::Foliage => self.foliage_tint(biome_id, blend),
            TerrainTintSource::DryFoliage => self.dry_foliage_tint(biome_id, blend),
            TerrainTintSource::Water => self.water_tint(biome_id, blend),
        }
    }

    pub(super) fn tint_channel_for_source(source: TerrainTintSource) -> Option<BiomeTintChannel> {
        match source {
            TerrainTintSource::White | TerrainTintSource::Fixed(_) => None,
            TerrainTintSource::Grass => Some(BiomeTintChannel::Grass),
            TerrainTintSource::Foliage => Some(BiomeTintChannel::Foliage),
            TerrainTintSource::DryFoliage => Some(BiomeTintChannel::DryFoliage),
            TerrainTintSource::Water => Some(BiomeTintChannel::Water),
        }
    }
}

impl BiomeTintResolver for TerrainTintContext<'_> {
    fn is_quart_constant(&self, channel: BiomeTintChannel, biome_id: Option<i32>) -> bool {
        !matches!(channel, BiomeTintChannel::Grass)
            || !matches!(
                self.biome_profile(biome_id)
                    .map(|profile| profile.grass_color_modifier),
                Some(GrassColorModifier::Swamp)
            )
    }

    fn resolve_rgb(
        &self,
        channel: BiomeTintChannel,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> [u8; 3] {
        match channel {
            BiomeTintChannel::Grass => self.resolve_grass_rgb(biome_id, position),
            BiomeTintChannel::Foliage => self.resolve_foliage_rgb(biome_id),
            BiomeTintChannel::DryFoliage => self.resolve_dry_foliage_rgb(biome_id),
            BiomeTintChannel::Water => self.resolve_water_rgb(biome_id),
        }
    }
}

impl TerrainTintContext<'_> {
    fn block_colors_layer0_terrain_particle_tint(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
        blend: Option<&BiomeBlend>,
    ) -> Option<TerrainTint> {
        // Vanilla `BlockTintSources.grassBlock().colorAsTerrainParticle` returns -1, and
        // `bubble_column` shares `waterParticles()` with `water`.
        if block_name == "minecraft:grass_block" {
            return Some(TerrainTint::WHITE);
        }
        if block_name == "minecraft:bubble_column" {
            return Some(self.water_tint(biome_id, blend));
        }
        let rule = TerrainTintRule::for_block_state(block_name, properties);
        if rule.source == TerrainTintSource::White {
            return None;
        }
        if !rule.tints(Some(0)) {
            return Some(TerrainTint::WHITE);
        }
        Some(self.resolve_tint(rule.source, biome_id, position, blend))
    }

    fn grass_tint(
        &self,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
        blend: Option<&BiomeBlend>,
    ) -> TerrainTint {
        terrain_tint_from_rgb(
            self.blend_rgb(biome_id, position, blend, |ctx, biome_id, pos| {
                ctx.resolve_grass_rgb(biome_id, pos)
            }),
        )
    }

    fn resolve_grass_rgb(
        &self,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> [u8; 3] {
        let profile = self.biome_profile(biome_id);
        let base = profile.and_then(|profile| profile.grass_color).or_else(|| {
            self.colormaps.map(|colormaps| {
                let (temperature, downfall) = biome_colormap_climate(profile);
                colormaps
                    .grass
                    .sample_temperature_downfall(temperature, downfall)
            })
        });
        let Some(base) = base else {
            return [0x91, 0xbd, 0x59];
        };
        apply_grass_color_modifier(
            profile.map_or(GrassColorModifier::None, |profile| {
                profile.grass_color_modifier
            }),
            base,
            position,
        )
    }

    fn foliage_tint(&self, biome_id: Option<i32>, blend: Option<&BiomeBlend>) -> TerrainTint {
        terrain_tint_from_rgb(
            self.blend_rgb(biome_id, None, blend, |ctx, biome_id, _pos| {
                ctx.resolve_foliage_rgb(biome_id)
            }),
        )
    }

    fn resolve_foliage_rgb(&self, biome_id: Option<i32>) -> [u8; 3] {
        let profile = self.biome_profile(biome_id);
        profile
            .and_then(|profile| profile.foliage_color)
            .or_else(|| {
                self.colormaps.map(|colormaps| {
                    let (temperature, downfall) = biome_colormap_climate(profile);
                    colormaps
                        .foliage
                        .sample_temperature_downfall(temperature, downfall)
                })
            })
            .unwrap_or([0x48, 0xb5, 0x18])
    }

    fn dry_foliage_tint(&self, biome_id: Option<i32>, blend: Option<&BiomeBlend>) -> TerrainTint {
        terrain_tint_from_rgb(
            self.blend_rgb(biome_id, None, blend, |ctx, biome_id, _pos| {
                ctx.resolve_dry_foliage_rgb(biome_id)
            }),
        )
    }

    fn resolve_dry_foliage_rgb(&self, biome_id: Option<i32>) -> [u8; 3] {
        let profile = self.biome_profile(biome_id);
        profile
            .and_then(|profile| profile.dry_foliage_color)
            .or_else(|| {
                self.colormaps
                    .and_then(|colormaps| colormaps.dry_foliage.as_ref())
                    .map(|colormap| {
                        let (temperature, downfall) = biome_colormap_climate(profile);
                        colormap.sample_temperature_downfall(temperature, downfall)
                    })
            })
            .unwrap_or([0x5c, 0x3c, 0x32])
    }

    fn water_tint(&self, biome_id: Option<i32>, blend: Option<&BiomeBlend>) -> TerrainTint {
        terrain_tint_from_rgb(
            self.blend_rgb(biome_id, None, blend, |ctx, biome_id, _pos| {
                ctx.resolve_water_rgb(biome_id)
            }),
        )
    }

    fn resolve_water_rgb(&self, biome_id: Option<i32>) -> [u8; 3] {
        self.biome_profile(biome_id)
            .and_then(|profile| profile.water_color)
            .unwrap_or([0x3f, 0x76, 0xe4])
    }

    /// Averages a biome `ColorResolver` over the blend window, mirroring vanilla
    /// `ClientLevel.calculateBlockTint`: each available column is resolved to an
    /// RGB colour (the swamp grass modifier runs per sample, using that column's
    /// position), then the R/G/B channels are summed and integer-divided by the
    /// sample count. With a fully-loaded window the count is the vanilla
    /// `(2r+1)^2`; unloaded columns are dropped so a render-distance edge
    /// averages only the columns that actually exist. Without a sampling
    /// owner, or when every window column is unavailable, it degrades to the
    /// single-biome resolve, i.e. `biomeBlendRadius == 0`.
    fn blend_rgb(
        &self,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
        blend: Option<&BiomeBlend>,
        resolve: impl Fn(&Self, Option<i32>, Option<BlockRenderPosition>) -> [u8; 3],
    ) -> [u8; 3] {
        let Some(blend) = blend else {
            return resolve(self, biome_id, position);
        };
        let mut sum = [0u32; 3];
        let mut count = 0u32;
        for (sample_biome, sample_position) in blend.samples() {
            let Some(sample_biome) = sample_biome else {
                continue;
            };
            let rgb = resolve(self, Some(sample_biome), Some(sample_position));
            sum[0] += u32::from(rgb[0]);
            sum[1] += u32::from(rgb[1]);
            sum[2] += u32::from(rgb[2]);
            count += 1;
        }
        if count == 0 {
            return resolve(self, biome_id, position);
        }
        [
            (sum[0] / count) as u8,
            (sum[1] / count) as u8,
            (sum[2] / count) as u8,
        ]
    }

    fn biome_profile(&self, biome_id: Option<i32>) -> Option<&BiomeColorProfile> {
        self.biome_colors?.profile(biome_id?)
    }
}

pub fn terrain_particle_biome_blend_center(
    block_state_id: i32,
    mut position: BlockRenderPosition,
) -> Option<BlockRenderPosition> {
    let block_state =
        bbb_protocol::block_state_types::vanilla_block_state_identity(block_state_id)?;
    let uses_biome_particle_tint = if block_state.name == "minecraft:grass_block" {
        false
    } else if block_state.name == "minecraft:bubble_column" {
        true
    } else {
        let rule = TerrainTintRule::for_block_state(&block_state.name, &block_state.properties);
        rule.tints(Some(0)) && TerrainTintContext::tint_channel_for_source(rule.source).is_some()
    };
    if !uses_biome_particle_tint {
        return None;
    }
    if matches!(
        block_state.name.as_str(),
        "minecraft:large_fern" | "minecraft:tall_grass"
    ) && block_state.properties.get("half").map(String::as_str) == Some("upper")
    {
        position.y = position.y.saturating_sub(1);
    }
    Some(position)
}

fn terrain_particle_rgb_06(tint: TerrainTint) -> [f32; 4] {
    [
        tint.r as f32 / 255.0 * 0.6,
        tint.g as f32 / 255.0 * 0.6,
        tint.b as f32 / 255.0 * 0.6,
        1.0,
    ]
}

fn terrain_tint_particle_rgb(tint: TerrainTint) -> [f32; 4] {
    [
        tint.r as f32 / 255.0,
        tint.g as f32 / 255.0,
        tint.b as f32 / 255.0,
        1.0,
    ]
}

fn redstone_wire_tint(power: i32) -> TerrainTint {
    let power = power.clamp(0, 15) as f32 / 15.0;
    let red = power * 0.6 + if power > 0.0 { 0.4 } else { 0.3 };
    let green = (power * power * 0.7 - 0.5).clamp(0.0, 1.0);
    let blue = (power * power * 0.6 - 0.7).clamp(0.0, 1.0);
    TerrainTint::from_rgb_u8(
        as_vanilla_argb_channel(red),
        as_vanilla_argb_channel(green),
        as_vanilla_argb_channel(blue),
    )
}

fn stem_tint(age: i32) -> TerrainTint {
    let age = age.clamp(0, 7);
    TerrainTint::from_rgb_u8(
        (age * 32).clamp(0, 255) as u8,
        (255 - age * 8).clamp(0, 255) as u8,
        (age * 4).clamp(0, 255) as u8,
    )
}

fn as_vanilla_argb_channel(value: f32) -> u8 {
    (value * 255.0).floor().clamp(0.0, 255.0) as u8
}

pub fn load_terrain_textures(
    renderer: &mut crate::Renderer,
    roots: Option<&PackRoots>,
    cutout_leaves: bool,
    smooth_lighting: bool,
    biome_blend_radius: u32,
    max_mipmap_levels: u32,
    atlas_base_padding: u32,
) -> TerrainTextureState {
    let Some(roots) = roots else {
        tracing::warn!("falling back to default terrain texture atlas without pack roots");
        renderer.set_terrain_particle_sprite_uvs(Vec::new());
        return TerrainTextureState::default()
            .with_cutout_leaves(cutout_leaves)
            .with_smooth_lighting(smooth_lighting)
            .with_biome_blend_radius(biome_blend_radius)
            .with_max_mipmap_levels(max_mipmap_levels)
            .with_atlas_base_padding(atlas_base_padding);
    };
    match try_load_terrain_textures(
        renderer,
        roots,
        cutout_leaves,
        smooth_lighting,
        biome_blend_radius,
        max_mipmap_levels,
        atlas_base_padding,
    ) {
        Ok(textures) => textures,
        Err(err) => {
            tracing::warn!(?err, "falling back to default terrain texture atlas");
            renderer.set_terrain_particle_sprite_uvs(Vec::new());
            TerrainTextureState::default()
                .with_cutout_leaves(cutout_leaves)
                .with_smooth_lighting(smooth_lighting)
                .with_biome_blend_radius(biome_blend_radius)
                .with_max_mipmap_levels(max_mipmap_levels)
                .with_atlas_base_padding(atlas_base_padding)
        }
    }
}

pub fn try_load_terrain_textures(
    renderer: &mut crate::Renderer,
    roots: &PackRoots,
    cutout_leaves: bool,
    smooth_lighting: bool,
    biome_blend_radius: u32,
    max_mipmap_levels: u32,
    atlas_base_padding: u32,
) -> Result<TerrainTextureState> {
    let images = roots.load_block_texture_images_shared()?;
    let block_models = roots.load_block_model_catalog()?;
    let colormaps = match roots.load_terrain_colormaps() {
        Ok(colormaps) => Some(colormaps),
        Err(err) => {
            tracing::warn!(?err, "falling back to constant terrain tint colors");
            None
        }
    };
    let biome_colors = match roots.load_biome_color_catalog() {
        Ok(biome_colors) => Some(biome_colors),
        Err(err) => {
            tracing::warn!(?err, "falling back to default terrain biome tint");
            None
        }
    };
    let stack = roots.resource_stack();
    let max_dim = renderer.max_texture_dimension_2d().min(16_384);
    let packer = AtlasPacker::new_bounded(
        VANILLA_26_1_BLOCK_ATLAS_WIDTH.min(max_dim),
        max_dim,
        atlas_base_padding,
    )?;
    let mut atlas =
        packer.stitch_mips_with_max_level_and_stack(&images, max_mipmap_levels.min(4), &stack)?;
    let mip_level = atlas.mip_level();
    {
        let mip_rgba = atlas.rgba_slices();
        renderer.upload_terrain_texture_atlas_mips(
            atlas.layout.width,
            atlas.layout.height,
            &mip_rgba,
        )?;
    }
    atlas.release_rgba();
    let animation_plan = atlas
        .layout
        .prepare_gpu_animation_plan(&images, mip_level)?;
    renderer.upload_terrain_texture_animation(animation_plan)?;
    renderer.set_terrain_particle_sprite_uvs(terrain_particle_sprite_uvs(&atlas.layout));
    let animated_sprites = images
        .iter()
        .filter(|image| image.animation.is_some())
        .count();
    tracing::info!(
        width = atlas.layout.width,
        height = atlas.layout.height,
        mip_level,
        sprites = atlas.layout.sprites.len(),
        animated_sprites,
        blockstates = block_models.len(),
        colormaps = colormaps.is_some(),
        biome_colors = biome_colors.as_ref().map_or(0, |colors| colors.len()),
        "loaded terrain texture atlas"
    );
    let textures = TerrainTextureState::from_layout_and_images(
        &atlas.layout,
        &images,
        Some(block_models),
        colormaps,
        biome_colors,
    )
    .with_cutout_leaves(cutout_leaves)
    .with_smooth_lighting(smooth_lighting)
    .with_biome_blend_radius(biome_blend_radius)
    .with_max_mipmap_levels(max_mipmap_levels);
    Ok(textures)
}

fn terrain_uv_rect(layout: &AtlasLayout, sprite: &bbb_client_assets::AtlasSprite) -> TerrainUvRect {
    let width = layout.width as f32;
    let height = layout.height as f32;
    let x0 = sprite.content.x as f32;
    let y0 = sprite.content.y as f32;
    let x1 = (sprite.content.x + sprite.content.width) as f32;
    let y1 = (sprite.content.y + sprite.content.height) as f32;
    TerrainUvRect {
        // Vanilla `TextureAtlasSprite` exposes the exact content edges. The
        // mip-aligned replicated gutter prevents atlas bleed; shrinking every
        // 16x16 sprite by one texel changes glass borders and leaf cutouts.
        min: [x0 / width, y0 / height],
        max: [x1 / width, y1 / height],
    }
}

fn terrain_particle_sprite_uvs(layout: &AtlasLayout) -> Vec<ParticleSpriteUv> {
    layout
        .sprites
        .iter()
        .map(|sprite| {
            let uv = terrain_uv_rect(layout, sprite);
            ParticleSpriteUv {
                uv: ParticleUvRect {
                    min: uv.min,
                    max: uv.max,
                },
                has_translucent: sprite.transparency.has_translucent,
            }
        })
        .collect()
}

fn terrain_transparency(
    transparency: bbb_client_assets::SpriteTransparency,
) -> TerrainTransparency {
    TerrainTransparency {
        has_transparent: transparency.has_transparent,
        has_translucent: transparency.has_translucent,
    }
}

fn force_translucent(force: bool) -> TerrainTransparency {
    if force {
        TerrainTransparency::TRANSLUCENT
    } else {
        TerrainTransparency::OPAQUE
    }
}

fn render_shape_has_translucent_material(
    shape: &TerrainRenderShape,
    fallback: [TerrainTransparency; 6],
) -> bool {
    let translucent = |value: &TerrainTransparency| value.has_translucent;
    match shape {
        TerrainRenderShape::Cube | TerrainRenderShape::Cross { .. } => {
            fallback.iter().any(translucent)
        }
        TerrainRenderShape::Crosses(crosses) => crosses
            .iter()
            .flat_map(|cross| cross.face_transparency.iter())
            .any(translucent),
        TerrainRenderShape::Box {
            face_present,
            face_transparency,
            ..
        } => face_present
            .iter()
            .zip(face_transparency)
            .any(|(present, transparency)| *present && transparency.has_translucent),
        TerrainRenderShape::Boxes(boxes) => boxes.iter().any(|model_box| {
            model_box
                .face_present
                .iter()
                .zip(model_box.face_transparency)
                .any(|(present, transparency)| *present && transparency.has_translucent)
        }),
        TerrainRenderShape::Quads(quads) => {
            quads.iter().any(|quad| quad.transparency.has_translucent)
        }
    }
}

fn quad_uv_crop(uvs: [[f32; 2]; 4]) -> Option<[u8; 4]> {
    let min_u = uvs
        .iter()
        .map(|uv| uv[0])
        .try_fold(f32::INFINITY, finite_min)?;
    let min_v = uvs
        .iter()
        .map(|uv| uv[1])
        .try_fold(f32::INFINITY, finite_min)?;
    let max_u = uvs
        .iter()
        .map(|uv| uv[0])
        .try_fold(f32::NEG_INFINITY, finite_max)?;
    let max_v = uvs
        .iter()
        .map(|uv| uv[1])
        .try_fold(f32::NEG_INFINITY, finite_max)?;
    Some([
        quantize_model_uv(min_u)?,
        quantize_model_uv(min_v)?,
        quantize_model_uv(max_u)?,
        quantize_model_uv(max_v)?,
    ])
}

fn finite_min(current: f32, value: f32) -> Option<f32> {
    value.is_finite().then_some(current.min(value))
}

fn finite_max(current: f32, value: f32) -> Option<f32> {
    value.is_finite().then_some(current.max(value))
}

fn quantize_model_uv(value: f32) -> Option<u8> {
    let scaled = value * 16.0;
    let rounded = scaled.round();
    ((0.0..=16.0).contains(&rounded) && (scaled - rounded).abs() < 0.001).then_some(rounded as u8)
}

fn block_model_seed(position: BlockRenderPosition) -> i64 {
    let seed = i64::from(position.x).wrapping_mul(3_129_871)
        ^ i64::from(position.z).wrapping_mul(116_129_781)
        ^ i64::from(position.y);
    seed.wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11))
        >> 16
}

fn block_fallback_texture_id(block_name: &str) -> String {
    let (namespace, path) = block_name
        .split_once(':')
        .unwrap_or(("minecraft", block_name));
    format!("{namespace}:block/{path}")
}

fn fluid_render_shape(
    block_name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<TerrainRenderShape> {
    if !matches!(block_name, "minecraft:water" | "minecraft:lava") {
        return None;
    }

    let level = properties
        .get("level")
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(0);
    Some(fluid_box_shape(fluid_height_units(level)))
}

fn fluid_height_units(level: u8) -> u8 {
    let amount = match level {
        0 => 8,
        1..=7 => 8 - level,
        _ => 8,
    };
    ((amount as u16 * 16 + 4) / 9).clamp(1, 16) as u8
}

fn fluid_box_shape(height: u8) -> TerrainRenderShape {
    let height = height.clamp(1, 16);
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    let side_v0 = 16 - height;
    face_uvs[2] = [0, side_v0, 16, 16];
    face_uvs[3] = [0, side_v0, 16, 16];
    face_uvs[4] = [0, side_v0, 16, 16];
    face_uvs[5] = [0, side_v0, 16, 16];
    TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, height, 16],
        face_present: [true; 6],
        face_uvs,
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull: all_terrain_face_cull(),
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    }
}

fn model_box_cull_faces(face_cull: [Option<BlockModelFace>; 6]) -> [Option<TerrainFace>; 6] {
    face_cull.map(|face| face.map(model_face_to_terrain_face))
}

fn model_face_to_terrain_face(face: BlockModelFace) -> TerrainFace {
    match face {
        BlockModelFace::Down => TerrainFace::Down,
        BlockModelFace::Up => TerrainFace::Up,
        BlockModelFace::North => TerrainFace::North,
        BlockModelFace::South => TerrainFace::South,
        BlockModelFace::West => TerrainFace::West,
        BlockModelFace::East => TerrainFace::East,
    }
}

fn all_terrain_face_cull() -> [Option<TerrainFace>; 6] {
    TerrainFace::ALL.map(Some)
}

/// One element of the item-frame border model: opaque, shaded, unculled quads whose present faces map
/// their `0..=16` UV crop into the named atlas texture. This path keeps the vanilla model templates'
/// fractional Z depths without broadening the integer `TerrainBox` representation used by normal blocks.
fn frame_border_box_quads(
    from: [f32; 3],
    to: [f32; 3],
    face_present: [bool; 6],
    face_uvs: [[u8; 4]; 6],
    texture_indices: [u32; 6],
) -> Vec<TerrainQuad> {
    TerrainFace::ALL
        .into_iter()
        .filter_map(|face| {
            let index = terrain_face_index(face);
            face_present[index].then_some(TerrainQuad {
                corners: frame_border_face_corners(face, from, to),
                normal: terrain_face_normal(face),
                uvs: frame_border_face_uvs(face_uvs[index]),
                cull: Some(face),
                texture_index: texture_indices[index],
                tint: TerrainTint::WHITE,
                transparency: TerrainTransparency::OPAQUE,
                shade: true,
                light_emission: 0,
            })
        })
        .collect()
}

fn frame_border_face_corners(face: TerrainFace, min: [f32; 3], max: [f32; 3]) -> [[f32; 3]; 4] {
    match face {
        TerrainFace::Down => [
            [min[0], min[1], max[2]],
            [max[0], min[1], max[2]],
            [max[0], min[1], min[2]],
            [min[0], min[1], min[2]],
        ],
        TerrainFace::Up => [
            [min[0], max[1], min[2]],
            [max[0], max[1], min[2]],
            [max[0], max[1], max[2]],
            [min[0], max[1], max[2]],
        ],
        TerrainFace::North => [
            [max[0], min[1], min[2]],
            [max[0], max[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], min[1], min[2]],
        ],
        TerrainFace::South => [
            [min[0], min[1], max[2]],
            [min[0], max[1], max[2]],
            [max[0], max[1], max[2]],
            [max[0], min[1], max[2]],
        ],
        TerrainFace::West => [
            [min[0], min[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], max[1], max[2]],
            [min[0], min[1], max[2]],
        ],
        TerrainFace::East => [
            [max[0], min[1], max[2]],
            [max[0], max[1], max[2]],
            [max[0], max[1], min[2]],
            [max[0], min[1], min[2]],
        ],
    }
}

fn frame_border_face_uvs(uv: [u8; 4]) -> [[f32; 2]; 4] {
    [
        [uv[0] as f32 / 16.0, uv[1] as f32 / 16.0],
        [uv[2] as f32 / 16.0, uv[1] as f32 / 16.0],
        [uv[2] as f32 / 16.0, uv[3] as f32 / 16.0],
        [uv[0] as f32 / 16.0, uv[3] as f32 / 16.0],
    ]
}

fn terrain_face_normal(face: TerrainFace) -> [f32; 3] {
    match face {
        TerrainFace::Down => [0.0, -1.0, 0.0],
        TerrainFace::Up => [0.0, 1.0, 0.0],
        TerrainFace::North => [0.0, 0.0, -1.0],
        TerrainFace::South => [0.0, 0.0, 1.0],
        TerrainFace::West => [-1.0, 0.0, 0.0],
        TerrainFace::East => [1.0, 0.0, 0.0],
    }
}

fn terrain_face_index(face: TerrainFace) -> usize {
    match face {
        TerrainFace::Down => 0,
        TerrainFace::Up => 1,
        TerrainFace::North => 2,
        TerrainFace::South => 3,
        TerrainFace::West => 4,
        TerrainFace::East => 5,
    }
}

#[cfg(test)]
mod tests;
