//! Per-block-state terrain render data: everything that depends only on the state and the
//! loaded resources is resolved once; cells only add the weighted variant pick and biome tint.

use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

use bbb_client_assets::{BlockModelShape, BlockStateModelChoices};

use super::{
    block_fallback_texture_id, block_model_seed, BlockRenderPosition, TerrainParticleTintCatalog,
    TerrainTextureCore, TerrainTextureState,
};
use crate::{
    scene::biome_tint::{
        block_wants_biome_blend, build_biome_blend, is_dry_foliage_tinted_block,
        is_foliage_tinted_block, is_grass_tinted_block, BiomeBlend, SectionBiomeTintCache,
    },
    terrain::{
        block_table::{
            fluid_shape_height, smooth_lit, TerrainBlockState, TerrainBlockTableParts,
            TerrainResolvedTintKey, TerrainStateModel, TerrainStateVariants,
        },
        TerrainCross, TerrainFluid, TerrainFluidKind, TerrainMaterialClass, TerrainRenderShape,
        TerrainSkipRendering, TerrainTextureAtlas, TerrainTint, TerrainTransparency,
    },
};

pub(crate) struct SectionTintAccess<'a> {
    pub(crate) cache: &'a mut SectionBiomeTintCache,
    pub(crate) uniform_qy: [bool; 4],
    pub(crate) sample_quart_biome: &'a dyn Fn(i32, usize, i32) -> Option<i32>,
    pub(crate) fallback_biome_at: &'a dyn Fn(BlockRenderPosition) -> Option<i32>,
}

pub(super) use crate::terrain::block_table::TerrainTintSource;

/// Vanilla `BlockColors` routing for one block state: which colour a tinted face receives,
/// which tintindex layer takes it, and whether untinted faces are tinted too (water tints every
/// quad).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TerrainTintRule {
    pub(super) source: TerrainTintSource,
    tints_untinted_faces: bool,
    tint_layer: i32,
}

impl TerrainTintRule {
    pub(super) fn for_block_state(block_name: &str, properties: &BTreeMap<String, String>) -> Self {
        let source = match block_name {
            "minecraft:water" => {
                return Self {
                    source: TerrainTintSource::Water,
                    tints_untinted_faces: true,
                    tint_layer: 0,
                };
            }
            "minecraft:water_cauldron" => TerrainTintSource::Water,
            "minecraft:pink_petals" | "minecraft:wildflowers" => {
                return Self {
                    source: TerrainTintSource::Grass,
                    tints_untinted_faces: false,
                    tint_layer: 1,
                };
            }
            "minecraft:spruce_leaves" => {
                TerrainTintSource::Fixed(TerrainTint::from_rgb_u8(0x61, 0x99, 0x61))
            }
            "minecraft:birch_leaves" => {
                TerrainTintSource::Fixed(TerrainTint::from_rgb_u8(0x80, 0xa7, 0x55))
            }
            "minecraft:redstone_wire" => TerrainTintSource::Fixed(super::redstone_wire_tint(
                properties
                    .get("power")
                    .and_then(|value| value.parse::<i32>().ok())
                    .unwrap_or(0),
            )),
            "minecraft:attached_melon_stem" | "minecraft:attached_pumpkin_stem" => {
                TerrainTintSource::Fixed(TerrainTint::from_rgb_u8(0xe0, 0xc7, 0x1c))
            }
            "minecraft:melon_stem" | "minecraft:pumpkin_stem" => {
                TerrainTintSource::Fixed(super::stem_tint(
                    properties
                        .get("age")
                        .and_then(|value| value.parse::<i32>().ok())
                        .unwrap_or(0),
                ))
            }
            "minecraft:lily_pad" => {
                TerrainTintSource::Fixed(TerrainTint::from_rgb_u8(0x20, 0x80, 0x30))
            }
            name if is_dry_foliage_tinted_block(name) => TerrainTintSource::DryFoliage,
            name if is_foliage_tinted_block(name) => TerrainTintSource::Foliage,
            name if is_grass_tinted_block(name) => TerrainTintSource::Grass,
            _ => TerrainTintSource::White,
        };
        Self {
            source,
            tints_untinted_faces: false,
            tint_layer: 0,
        }
    }

    #[cfg(test)]
    pub(super) fn for_block(block_name: &str) -> Self {
        Self::for_block_state(block_name, &BTreeMap::new())
    }

    const NONE: Self = Self {
        source: TerrainTintSource::White,
        tints_untinted_faces: false,
        tint_layer: 0,
    };

    pub(super) fn tints(self, tint_index: Option<i32>) -> bool {
        self.tints_untinted_faces || tint_index == Some(self.tint_layer)
    }

    fn constant(self) -> Option<TerrainTint> {
        match self.source {
            TerrainTintSource::White => Some(TerrainTint::WHITE),
            TerrainTintSource::Fixed(tint) => Some(tint),
            _ => None,
        }
    }
}

/// One selected model variant of a block state with textures, transparencies and shape resolved.
/// When the tint source is a biome colour, `face_tinted` and `shape_tinted` mark
/// the faces that take it; a constant tint is already applied.
#[derive(Debug, Clone)]
pub(super) struct TerrainModelRender {
    pub(super) has_model: bool,
    pub(super) texture_indices: [u32; 6],
    face_tinted: [bool; 6],
    pub(super) face_transparency: [TerrainTransparency; 6],
    shape: TerrainRenderShape,
    /// Per shape element, the bits of its faces that take the biome tint.
    shape_tinted: Arc<[u8]>,
    pub(super) ambient_occlusion: bool,
}

impl TerrainModelRender {
    /// Face tints and render shape for a cell whose tinted faces resolve to `tint`.
    pub(super) fn tinted(&self, tint: TerrainTint) -> ([TerrainTint; 6], TerrainRenderShape) {
        (self.face_tints(tint), self.shape_with_tint(tint))
    }

    fn face_tints(&self, tint: TerrainTint) -> [TerrainTint; 6] {
        self.face_tinted
            .map(|tinted| if tinted { tint } else { TerrainTint::WHITE })
    }

    fn shape_with_tint(&self, tint: TerrainTint) -> TerrainRenderShape {
        tint_shape(&self.shape, &self.shape_tinted, tint)
    }
}

/// Block-state facts shared by every cell of that state.
#[derive(Debug, Clone)]
pub(super) struct TerrainStateRender {
    block_name: Option<String>,
    properties: BTreeMap<String, String>,
    material: bbb_render_types::TerrainMaterialClass,
    force_opaque: bool,
    choices: Option<BlockStateModelChoices>,
    models: Vec<(usize, TerrainModelRender)>,
    pub(super) tint: TerrainTintRule,
    pub(super) wants_blend: bool,
    fluid_kind: Option<TerrainFluidKind>,
    #[cfg(test)]
    pub(super) fluid: Option<TerrainStateFluid>,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(super) struct TerrainStateFluid {
    pub(super) texture_indices: [u32; 6],
    pub(super) overlay_texture_index: Option<u32>,
}

impl TerrainStateRender {
    /// The weighted-variant combination Vanilla picks at `position` (the first one without).
    pub(super) fn combination(&self, position: Option<BlockRenderPosition>) -> usize {
        self.combination_for_seed(position.map(block_model_seed))
    }

    pub(super) fn combination_for_seed(&self, seed: Option<i64>) -> usize {
        match &self.choices {
            Some(choices) if choices.combination_count() > 1 => choices.combination(seed),
            _ => 0,
        }
    }
}

impl TerrainTextureCore {
    fn tint_rule_for_block_state(
        &self,
        block_name: Option<&str>,
        properties: &BTreeMap<String, String>,
    ) -> TerrainTintRule {
        let Some(block_name) = block_name else {
            return TerrainTintRule::NONE;
        };
        let rule = TerrainTintRule::for_block_state(block_name, properties);
        if rule.source == TerrainTintSource::White {
            if let Some(fallback_tint) = self.mod_block_fallback_tint(block_name) {
                return TerrainTintRule {
                    source: TerrainTintSource::Fixed(fallback_tint),
                    tints_untinted_faces: true,
                    tint_layer: 0,
                };
            }
        }
        rule
    }

    pub(super) fn state_render(
        &self,
        block_name: Option<&str>,
        properties: &BTreeMap<String, String>,
        material: bbb_render_types::TerrainMaterialClass,
        fluid: Option<TerrainFluidKind>,
        force_opaque: bool,
    ) -> TerrainStateRender {
        let has_water_fluid = matches!(fluid, Some(TerrainFluidKind::Water));
        TerrainStateRender {
            block_name: block_name.map(str::to_string),
            properties: properties.clone(),
            material,
            force_opaque: force_opaque
                && block_name.is_some_and(|block_name| self.forces_opaque_leaves(block_name)),
            choices: block_name.and_then(|block_name| {
                self.block_models
                    .as_ref()?
                    .block_state_model_choices(block_name, properties)
            }),
            models: Vec::new(),
            tint: self.tint_rule_for_block_state(block_name, properties),
            wants_blend: block_name.map_or(has_water_fluid, |block_name| {
                block_wants_biome_blend(block_name, has_water_fluid)
            }),
            fluid_kind: fluid,
            #[cfg(test)]
            fluid: fluid.map(|kind| TerrainStateFluid {
                texture_indices: self.fluid_kind_texture_indices(kind),
                overlay_texture_index: self.fluid_overlay_texture_index(kind),
            }),
        }
    }

    pub(super) fn state_model<'s>(
        &self,
        state: &'s mut TerrainStateRender,
        combination: usize,
    ) -> &'s TerrainModelRender {
        let index = match state
            .models
            .iter()
            .position(|(cached, _)| *cached == combination)
        {
            Some(index) => index,
            None => {
                let model = self.build_model_render(state, combination);
                state.models.push((combination, model));
                state.models.len() - 1
            }
        };
        &state.models[index].1
    }

    fn build_model_render(
        &self,
        state: &TerrainStateRender,
        combination: usize,
    ) -> TerrainModelRender {
        self.build_model_render_parts(
            state.block_name.as_deref(),
            &state.properties,
            state.material,
            state.force_opaque,
            state.choices.as_ref(),
            state.tint,
            combination,
            None,
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build_model_render_parts(
        &self,
        block_name: Option<&str>,
        properties: &BTreeMap<String, String>,
        material: bbb_render_types::TerrainMaterialClass,
        force_opaque: bool,
        choices: Option<&BlockStateModelChoices>,
        rule: TerrainTintRule,
        combination: usize,
        direct_seed: Option<i64>,
        bake_constant_tint: bool,
    ) -> TerrainModelRender {
        let Some(block_name) = block_name else {
            return TerrainModelRender {
                has_model: false,
                texture_indices: [self.fallback_index; 6],
                face_tinted: [false; 6],
                face_transparency: [TerrainTransparency::OPAQUE; 6],
                shape: TerrainRenderShape::Cube,
                shape_tinted: Arc::from([]),
                ambient_occlusion: true,
            };
        };
        let model = choices
            .and_then(|choices| {
                let models = self.block_models.as_ref()?;
                if let Some(seed) = direct_seed {
                    models.block_state_model_for_seed(choices, Some(seed))
                } else {
                    models.block_state_model(choices, combination)
                }
            })
            .or_else(|| {
                self.content_projection
                    .as_ref()
                    .and_then(|projection| projection.block(block_name))
                    .map(|projected| projected.render_model.clone())
            });
        let (has_model, texture_indices, face_tinted, mut face_transparency, shape, ao) =
            match model {
                Some(model) => {
                    let texture_indices = self
                        .fluid_texture_indices(block_name)
                        .unwrap_or_else(|| self.face_texture_indices(&model.face_textures));
                    let face_tinted = model.face_textures.tint_indices.map(|ti| rule.tints(ti));
                    let face_transparency = self.face_texture_transparencies(&model.face_textures);
                    (
                        true,
                        texture_indices,
                        face_tinted,
                        face_transparency,
                        model.shape,
                        model.use_ambient_occlusion,
                    )
                }
                None => {
                    let all = self.texture_index(&block_fallback_texture_id(block_name));
                    let texture_indices =
                        self.fluid_texture_indices(block_name).unwrap_or([all; 6]);
                    (
                        false,
                        texture_indices,
                        [rule.tints(Some(0)); 6],
                        [TerrainTransparency::OPAQUE; 6],
                        BlockModelShape::Cube,
                        true,
                    )
                }
            };
        let mut shape_tinted = shape_tint_mask(&shape, face_tinted, &|index| rule.tints(index));
        let mut shape = self.terrain_render_shape_for_block(
            block_name,
            properties,
            material,
            shape,
            texture_indices,
            [TerrainTint::WHITE; 6],
            face_transparency,
            &|_| TerrainTint::WHITE,
        );
        if shape_element_count(&shape) != shape_tinted.len() {
            // A fluid shape replaced the model shape.
            shape_tinted = Arc::from([]);
        }
        if force_opaque {
            face_transparency = [TerrainTransparency::OPAQUE; 6];
            shape.force_opaque();
        }
        let should_bake_constant = bake_constant_tint
            || matches!(rule.source, TerrainTintSource::White)
            || shape_tinted.is_empty();
        if should_bake_constant {
            if let Some(constant) = rule.constant() {
                shape = tint_shape(&shape, &shape_tinted, constant);
                shape_tinted = Arc::from([]);
            }
        }
        TerrainModelRender {
            has_model,
            texture_indices,
            face_tinted,
            face_transparency,
            shape,
            shape_tinted,
            ambient_occlusion: ao && self.smooth_lighting,
        }
    }
}

impl TerrainTextureState {
    pub(super) fn resolve_tint(
        &self,
        rule: TerrainTintRule,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
        blend: Option<&BiomeBlend>,
    ) -> TerrainTint {
        self.tint_context()
            .resolve_tint(rule.source, biome_id, position, blend)
    }
}

/// The resource-backed source of a block render table: the immutable model
/// inputs, the biome tint inputs and the session's tag facts.
pub struct TerrainResourceSource {
    texture_generation: u64,
    core: Arc<TerrainTextureCore>,
    tints: TerrainParticleTintCatalog,
    biome_blend_radius: u32,
    context: bbb_render_types::TerrainBlockRenderContext,
}

/// Dense immutable side-tables for weighted model choices and rare high-cardinality states.
#[derive(Debug)]
pub(crate) struct TerrainResolvedState {
    choices: Box<[BlockStateModelChoices]>,
    variant_model_indices: Box<[u32]>,
    dynamic: Box<[TerrainStateRender]>,
    dynamic_cache: Mutex<hashbrown::HashMap<(u32, u64), TerrainStateModel>>,
}

impl TerrainResolvedState {
    pub(crate) fn combination(&self, choices_index: u32, position: BlockRenderPosition) -> usize {
        self.choices[choices_index as usize].combination(Some(block_model_seed(position)))
    }

    #[inline]
    pub(crate) fn variant_model_index(&self, model_start: u32, offset: usize) -> u32 {
        self.variant_model_indices[model_start as usize + offset]
    }

    pub(crate) fn dynamic_tint_key(&self, dynamic_index: u32) -> TerrainResolvedTintKey {
        let render = &self.dynamic[dynamic_index as usize];
        TerrainResolvedTintKey {
            source: render.tint.source,
            wants_blend: render.wants_blend,
            water_fluid: matches!(render.fluid_kind, Some(TerrainFluidKind::Water)),
        }
    }
}

/// Weighted-variant combinations up to this bound are baked eagerly into the flat model table;
/// states exceeding it rebuild on demand with bounded caching.
const MAX_EAGER_COMBINATIONS: usize = 256;
/// Hard cap on retained geometry inside `TerrainBlockRenderTable::models` so adversarial packs
/// with many unique heavy blockstates spill cleanly into `Dynamic` instead of OOMing.
const MAX_EAGER_TERRAIN_TABLE_BYTES: usize = 64 * 1024 * 1024;
const MAX_DYNAMIC_MODEL_CACHE_ENTRIES: usize = 256;

struct ModelInternTable {
    models: Vec<TerrainStateModel>,
    buckets: hashbrown::HashMap<u64, Vec<u32>>,
    retained_bytes: usize,
    variant_model_indices: Vec<u32>,
}

impl ModelInternTable {
    fn new() -> Self {
        let empty = TerrainStateModel::empty();
        let hash = hash_state_model(&empty);
        let mut buckets = hashbrown::HashMap::new();
        buckets.insert(hash, vec![0]);
        Self {
            models: vec![empty],
            buckets,
            retained_bytes: 0,
            variant_model_indices: Vec::new(),
        }
    }

    fn intern(&mut self, model: TerrainStateModel) -> u32 {
        let hash = hash_state_model(&model);
        if let Some(bucket) = self.buckets.get(&hash) {
            for &index in bucket {
                if self.models[index as usize] == model {
                    return index;
                }
            }
        }
        let estimated = estimate_state_model_bytes(&model);
        let index = self.models.len() as u32;
        self.models.push(model);
        self.retained_bytes = self.retained_bytes.saturating_add(estimated);
        self.buckets.entry(hash).or_default().push(index);
        index
    }

    fn within_budget(&self) -> bool {
        self.retained_bytes < MAX_EAGER_TERRAIN_TABLE_BYTES
    }
}

impl TerrainResourceSource {
    pub fn new(
        textures: &TerrainTextureState,
        context: bbb_render_types::TerrainBlockRenderContext,
    ) -> Self {
        Self {
            texture_generation: textures.generation(),
            core: Arc::clone(&textures.core),
            tints: textures.particle_tint_catalog(),
            biome_blend_radius: textures.biome_blend_radius,
            context,
        }
    }

    /// Whether this source was built from `textures` and `context` as they are now.
    pub fn is_current(
        &self,
        textures: &TerrainTextureState,
        context: &bbb_render_types::TerrainBlockRenderContext,
    ) -> bool {
        self.texture_generation == textures.generation() && self.context == *context
    }

    pub(crate) fn atlas(&self) -> &TerrainTextureAtlas {
        &self.core.atlas
    }

    pub(crate) fn build_table(&self) -> TerrainBlockTableParts {
        let count = self.context.total_block_state_count();
        let mut states = Vec::with_capacity(count as usize);
        let mut intern = ModelInternTable::new();
        let mut choices_table = Vec::new();
        let mut dynamic_table = Vec::new();

        for block_state_id in 0..count {
            states.push(self.build_flat_state(
                block_state_id,
                &mut intern,
                &mut choices_table,
                &mut dynamic_table,
            ));
        }
        let unknown = self.build_flat_state(
            i32::MAX,
            &mut intern,
            &mut choices_table,
            &mut dynamic_table,
        );

        TerrainBlockTableParts {
            states: states.into_boxed_slice(),
            unknown,
            models: intern.models.into_boxed_slice(),
            side_table: TerrainResolvedState {
                choices: choices_table.into_boxed_slice(),
                variant_model_indices: intern.variant_model_indices.into_boxed_slice(),
                dynamic: dynamic_table.into_boxed_slice(),
                dynamic_cache: Mutex::new(hashbrown::HashMap::new()),
            },
        }
    }

    fn build_flat_state(
        &self,
        block_state_id: i32,
        intern: &mut ModelInternTable,
        choices_table: &mut Vec<BlockStateModelChoices>,
        dynamic_table: &mut Vec<TerrainStateRender>,
    ) -> TerrainBlockState {
        let state = self.context.block_state(block_state_id);
        if matches!(
            state.material,
            bbb_render_types::TerrainMaterialClass::Empty
        ) {
            return TerrainBlockState::EMPTY;
        }
        let fluid = state.fluid.map(renderer_fluid);
        let fluid_kind = fluid.map(|fluid| fluid.kind);
        let water_fluid = matches!(fluid_kind, Some(TerrainFluidKind::Water));
        let material = renderer_material(state.material);
        let fluid_overlay = fluid.is_some() && !matches!(material, TerrainMaterialClass::Fluid);
        let force_opaque = state
            .block_name
            .is_some_and(|block_name| self.core.forces_opaque_leaves(block_name));
        let tint_rule = self
            .core
            .tint_rule_for_block_state(state.block_name, state.block_properties);
        let wants_blend = state.block_name.map_or(water_fluid, |block_name| {
            block_wants_biome_blend(block_name, water_fluid)
        });
        let choices = state.block_name.and_then(|block_name| {
            self.core
                .block_models
                .as_ref()?
                .block_state_model_choices(block_name, state.block_properties)
        });
        let combinations = choices
            .as_ref()
            .map_or(1, |choices| choices.combination_count().max(1));

        let build_one = |combination: usize, choices_ref: Option<&BlockStateModelChoices>| {
            let raw = self.core.build_model_render_parts(
                state.block_name,
                state.block_properties,
                state.material,
                force_opaque,
                choices_ref,
                tint_rule,
                combination,
                None,
                false,
            );
            bake_state_model(
                material,
                fluid_overlay,
                state.state_light_emission,
                tint_rule,
                true,
                raw,
            )
        };

        let tint_key = TerrainResolvedTintKey {
            source: tint_rule.source,
            wants_blend,
            water_fluid,
        };
        let make_dynamic = |dynamic_table: &mut Vec<TerrainStateRender>,
                            choices: Option<BlockStateModelChoices>| {
            let dynamic_index = dynamic_table.len() as u32;
            dynamic_table.push(TerrainStateRender {
                block_name: state.block_name.map(str::to_string),
                properties: state.block_properties.clone(),
                material: state.material,
                force_opaque,
                choices,
                models: Vec::new(),
                tint: tint_rule,
                wants_blend,
                fluid_kind,
                #[cfg(test)]
                fluid: None,
            });
            (false, TerrainStateVariants::Dynamic { dynamic_index })
        };

        let first_model = build_one(0, choices.as_ref());
        let fluid_shape_height = matches!(material, TerrainMaterialClass::Fluid)
            .then(|| fluid_shape_height(&first_model.render_shape))
            .flatten();

        let (has_no_mesh, variants) = if !intern.within_budget() {
            make_dynamic(dynamic_table, choices)
        } else if combinations == 1 {
            let has_no_mesh = first_model.route == 0;
            let needs_biome_tint = first_model.needs_cell_tint() || water_fluid;
            let model_index = intern.intern(first_model);
            let variants = if needs_biome_tint {
                TerrainStateVariants::BiomeTinted {
                    model_index,
                    tint_key,
                }
            } else {
                TerrainStateVariants::Fixed {
                    model_index,
                    fluid_tint: [TerrainTint::WHITE; 6],
                }
            };
            (has_no_mesh, variants)
        } else if combinations <= MAX_EAGER_COMBINATIONS {
            let mut has_no_mesh = first_model.route == 0;
            let first_needs_tint = first_model.needs_cell_tint();
            let first_index = intern.intern(first_model);
            let mut indices = Vec::with_capacity(combinations);
            indices.push(first_index);
            let mut all_same = true;
            let mut overflowed_budget = !intern.within_budget();
            if !overflowed_budget {
                for combination in 1..combinations {
                    let next_model = build_one(combination, choices.as_ref());
                    has_no_mesh &= next_model.route == 0;
                    let idx = intern.intern(next_model);
                    all_same &= idx == first_index;
                    indices.push(idx);
                    if !intern.within_budget() {
                        overflowed_budget = true;
                        break;
                    }
                }
            }
            if overflowed_budget {
                make_dynamic(dynamic_table, choices)
            } else if all_same {
                let needs_biome_tint = first_needs_tint || water_fluid;
                let variants = if needs_biome_tint {
                    TerrainStateVariants::BiomeTinted {
                        model_index: first_index,
                        tint_key,
                    }
                } else {
                    TerrainStateVariants::Fixed {
                        model_index: first_index,
                        fluid_tint: [TerrainTint::WHITE; 6],
                    }
                };
                (has_no_mesh, variants)
            } else {
                let model_start = intern.variant_model_indices.len() as u32;
                intern.variant_model_indices.extend_from_slice(&indices);
                let choices_index = choices_table.len() as u32;
                choices_table.push(choices.expect("combinations > 1 requires choices"));
                (
                    has_no_mesh,
                    TerrainStateVariants::Weighted {
                        model_start,
                        choices_index,
                        tint_key,
                    },
                )
            }
        } else {
            make_dynamic(dynamic_table, choices)
        };

        let (fluid_texture_indices, fluid_overlay_texture_index) = match fluid_kind {
            Some(kind) => (
                self.core.fluid_kind_texture_indices(kind),
                self.core.fluid_overlay_texture_index(kind),
            ),
            None => ([0; 6], None),
        };

        TerrainBlockState {
            model_offset_limits:
                bbb_protocol::block_state_types::vanilla_block_state_model_offset_limits(
                    block_state_id,
                )
                .unwrap_or([0.0; 2]),
            material,
            is_air: false,
            has_no_mesh,
            solid_render: state.solid_render,
            legacy_solid: state.legacy_solid,
            blocks_motion: state.blocks_motion,
            collision_full_block: collision_full_block(
                block_state_id,
                self.context.content_registry(),
            ),
            face_occlusion_shape_indices: state.face_occlusion_shape_indices,
            view_blocking: state.view_blocking,
            light_dampening: state.light_dampening,
            shade_brightness: state.shade_brightness,
            fluid,
            fluid_shape_height,
            fluid_texture_indices,
            fluid_overlay_texture_index,
            fluid_overlay_neighbor: state.fluid_overlay_neighbor,
            state_light_emission: state.state_light_emission,
            emissive_rendering: state.emissive_rendering,
            skip_rendering: renderer_skip_rendering(state.skip_rendering, self.core.cutout_leaves),
            variants,
        }
    }

    pub(crate) fn dynamic_model(
        &self,
        state: &TerrainBlockState,
        side_table: &TerrainResolvedState,
        dynamic_index: u32,
        position: BlockRenderPosition,
    ) -> TerrainStateModel {
        let render = &side_table.dynamic[dynamic_index as usize];
        let seed = block_model_seed(position);
        let cache_key = match &render.choices {
            Some(choices) if choices.combination_count() < usize::MAX => {
                (dynamic_index, choices.combination(Some(seed)) as u64)
            }
            _ => (dynamic_index, seed as u64),
        };
        if let Ok(cache) = side_table.dynamic_cache.lock() {
            if let Some(cached) = cache.get(&cache_key) {
                return cached.clone();
            }
        }
        // Use `block_state_model_for_seed` directly so multipart states whose product of weighted
        // selectors saturates `usize` still evaluate accurately without combination-index overflow.
        let model = self.core.build_model_render_parts(
            render.block_name.as_deref(),
            &render.properties,
            render.material,
            render.force_opaque,
            render.choices.as_ref(),
            render.tint,
            0,
            Some(seed),
            true,
        );
        let baked = bake_state_model(
            state.material,
            state.fluid.is_some() && !matches!(state.material, TerrainMaterialClass::Fluid),
            state.state_light_emission,
            render.tint,
            false,
            model,
        );
        if let Ok(mut cache) = side_table.dynamic_cache.lock() {
            if cache.len() >= MAX_DYNAMIC_MODEL_CACHE_ENTRIES {
                cache.clear();
            }
            cache.insert(cache_key, baked.clone());
        }
        baked
    }

    /// The cell tint and the fluid tint at `position`.
    pub(crate) fn biome_blend_radius(&self) -> u32 {
        self.biome_blend_radius
    }

    pub(crate) fn tints_for_key(
        &self,
        tint_key: TerrainResolvedTintKey,
        model: &TerrainStateModel,
        biome_id: Option<i32>,
        position: BlockRenderPosition,
        biome_at: &dyn Fn(BlockRenderPosition) -> Option<i32>,
    ) -> (TerrainTint, TerrainTint) {
        if !model.needs_cell_tint() && !tint_key.water_fluid {
            return (TerrainTint::WHITE, TerrainTint::WHITE);
        }
        // Only biome-resolver blocks read the blend window; the stone/dirt majority skips it.
        let blend = tint_key
            .wants_blend
            .then(|| build_biome_blend(position, self.biome_blend_radius, biome_at));
        let context = self.tints.tint_context();
        resolve_tint_pair(model.needs_cell_tint(), tint_key, |source| {
            context.resolve_tint(source, biome_id, Some(position), blend.as_ref())
        })
    }

    pub(crate) fn tints_for_key_in_section(
        &self,
        tint_key: TerrainResolvedTintKey,
        model: &TerrainStateModel,
        biome_id: Option<i32>,
        position: BlockRenderPosition,
        access: &mut SectionTintAccess<'_>,
    ) -> (TerrainTint, TerrainTint) {
        let needs_cell = model.needs_cell_tint();
        if !needs_cell && !tint_key.water_fluid {
            return (TerrainTint::WHITE, TerrainTint::WHITE);
        }
        if !tint_key.wants_blend {
            let context = self.tints.tint_context();
            return resolve_tint_pair(needs_cell, tint_key, |source| {
                context.resolve_tint(source, biome_id, Some(position), None)
            });
        }
        let local_x = (position.x & 15) as usize;
        let qy = ((position.y & 15) >> 2) as usize;
        let local_z = (position.z & 15) as usize;
        if !access.uniform_qy[qy] {
            return self.tints_for_key(
                tint_key,
                model,
                biome_id,
                position,
                access.fallback_biome_at,
            );
        }
        let context = self.tints.tint_context();
        resolve_tint_pair(needs_cell, tint_key, |source| {
            match super::TerrainTintContext::tint_channel_for_source(source) {
                Some(channel) => access.cache.resolve_blended_tint(
                    [local_x, qy, local_z],
                    channel,
                    biome_id,
                    position,
                    access.sample_quart_biome,
                    &context,
                ),
                None => context.resolve_tint(source, biome_id, Some(position), None),
            }
        })
    }
}

fn resolve_tint_pair(
    needs_cell_tint: bool,
    tint_key: TerrainResolvedTintKey,
    mut resolve_source: impl FnMut(TerrainTintSource) -> TerrainTint,
) -> (TerrainTint, TerrainTint) {
    let tint = if needs_cell_tint {
        resolve_source(tint_key.source)
    } else {
        TerrainTint::WHITE
    };
    let fluid_tint = if tint_key.water_fluid {
        resolve_source(TerrainTintSource::Water)
    } else {
        TerrainTint::WHITE
    };
    (tint, fluid_tint)
}

fn bake_state_model(
    material: TerrainMaterialClass,
    fluid_overlay: bool,
    state_light_emission: u8,
    tint_rule: TerrainTintRule,
    preserve_fixed_tint: bool,
    model: TerrainModelRender,
) -> TerrainStateModel {
    let constant = if preserve_fixed_tint && !model.shape_tinted.is_empty() {
        None
    } else {
        tint_rule.constant()
    };
    let mut built = TerrainStateModel::new(
        material,
        fluid_overlay,
        model.texture_indices,
        model.face_tints(constant.unwrap_or(TerrainTint::WHITE)),
        model.face_transparency,
        model.shape,
        smooth_lit(model.ambient_occlusion, state_light_emission),
    );
    if constant.is_none() {
        built.face_tinted = model.face_tinted;
        built.shape_tinted = model.shape_tinted;
    }
    built
}

fn estimate_state_model_bytes(model: &TerrainStateModel) -> usize {
    let shape_bytes = match &model.render_shape {
        TerrainRenderShape::Cube
        | TerrainRenderShape::Cross { .. }
        | TerrainRenderShape::Box { .. } => 0,
        TerrainRenderShape::Crosses(crosses) => crosses
            .len()
            .saturating_mul(std::mem::size_of::<TerrainCross>()),
        TerrainRenderShape::Boxes(boxes) => boxes
            .len()
            .saturating_mul(std::mem::size_of::<crate::terrain::TerrainBox>()),
        TerrainRenderShape::Quads(quads) => quads
            .len()
            .saturating_mul(std::mem::size_of::<crate::terrain::TerrainQuad>()),
    };
    std::mem::size_of::<TerrainStateModel>()
        .saturating_add(shape_bytes)
        .saturating_add(model.shape_tinted.len())
}

fn hash_state_model(model: &TerrainStateModel) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    model.texture_indices.hash(&mut hasher);
    for tint in &model.face_tints {
        (tint.r, tint.g, tint.b).hash(&mut hasher);
    }
    model.face_tinted.hash(&mut hasher);
    for transparency in &model.face_transparency {
        (transparency.has_transparent, transparency.has_translucent).hash(&mut hasher);
    }
    model.shape_tinted.as_ref().hash(&mut hasher);
    model.ambient_occlusion.hash(&mut hasher);
    model.route.hash(&mut hasher);
    match &model.render_shape {
        TerrainRenderShape::Cube => 0u8.hash(&mut hasher),
        TerrainRenderShape::Cross {
            shade,
            light_emission,
        } => {
            1u8.hash(&mut hasher);
            shade.hash(&mut hasher);
            light_emission.hash(&mut hasher);
        }
        TerrainRenderShape::Crosses(crosses) => {
            2u8.hash(&mut hasher);
            crosses.len().hash(&mut hasher);
            for cross in crosses.iter() {
                cross.texture_indices.hash(&mut hasher);
                for tint in &cross.tint {
                    (tint.r, tint.g, tint.b).hash(&mut hasher);
                }
                for t in &cross.face_transparency {
                    (t.has_transparent, t.has_translucent).hash(&mut hasher);
                }
                cross.shade.hash(&mut hasher);
                cross.light_emission.hash(&mut hasher);
            }
        }
        TerrainRenderShape::Box {
            from,
            to,
            face_present,
            face_uvs,
            face_uv_rotations,
            face_shade,
            face_light_emission,
            face_cull,
            face_transparency,
        } => {
            3u8.hash(&mut hasher);
            from.hash(&mut hasher);
            to.hash(&mut hasher);
            face_present.hash(&mut hasher);
            face_uvs.hash(&mut hasher);
            face_uv_rotations.hash(&mut hasher);
            face_shade.hash(&mut hasher);
            face_light_emission.hash(&mut hasher);
            for cull in face_cull {
                cull.map(|f| f.index()).hash(&mut hasher);
            }
            for t in face_transparency {
                (t.has_transparent, t.has_translucent).hash(&mut hasher);
            }
        }
        TerrainRenderShape::Boxes(boxes) => {
            4u8.hash(&mut hasher);
            boxes.len().hash(&mut hasher);
            for b in boxes.iter() {
                b.from.hash(&mut hasher);
                b.to.hash(&mut hasher);
                b.face_present.hash(&mut hasher);
                b.face_uvs.hash(&mut hasher);
                b.face_uv_rotations.hash(&mut hasher);
                b.face_shade.hash(&mut hasher);
                b.face_light_emission.hash(&mut hasher);
                for cull in &b.face_cull {
                    cull.map(|f| f.index()).hash(&mut hasher);
                }
                b.texture_indices.hash(&mut hasher);
                for tint in &b.tint {
                    (tint.r, tint.g, tint.b).hash(&mut hasher);
                }
                for t in &b.face_transparency {
                    (t.has_transparent, t.has_translucent).hash(&mut hasher);
                }
            }
        }
        TerrainRenderShape::Quads(quads) => {
            5u8.hash(&mut hasher);
            quads.len().hash(&mut hasher);
            for q in quads.iter() {
                for c in &q.corners {
                    c.map(f32::to_bits).hash(&mut hasher);
                }
                q.normal.map(f32::to_bits).hash(&mut hasher);
                for uv in &q.uvs {
                    uv.map(f32::to_bits).hash(&mut hasher);
                }
                q.cull.map(|f| f.index()).hash(&mut hasher);
                q.texture_index.hash(&mut hasher);
                (q.tint.r, q.tint.g, q.tint.b).hash(&mut hasher);
                (
                    q.transparency.has_transparent,
                    q.transparency.has_translucent,
                )
                    .hash(&mut hasher);
                q.shade.hash(&mut hasher);
                q.light_emission.hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

fn renderer_material(material: bbb_render_types::TerrainMaterialClass) -> TerrainMaterialClass {
    match material {
        bbb_render_types::TerrainMaterialClass::Empty
        | bbb_render_types::TerrainMaterialClass::Invisible => TerrainMaterialClass::Empty,
        bbb_render_types::TerrainMaterialClass::Opaque => TerrainMaterialClass::Opaque,
        bbb_render_types::TerrainMaterialClass::Cutout => TerrainMaterialClass::Cutout,
        bbb_render_types::TerrainMaterialClass::Fluid => TerrainMaterialClass::Fluid,
        bbb_render_types::TerrainMaterialClass::Translucent => TerrainMaterialClass::Translucent,
    }
}

pub(crate) fn renderer_skip_rendering(
    skip_rendering: bbb_render_types::TerrainSkipRendering,
    cutout_leaves: bool,
) -> TerrainSkipRendering {
    TerrainSkipRendering {
        same_block_key: skip_rendering.same_block_key,
        same_block_culls_all_faces: skip_rendering.same_block_culls_all_faces,
        same_block_culls_vertical_faces: skip_rendering.same_block_culls_vertical_faces,
        culls_with_leaves: !cutout_leaves && skip_rendering.leaves_block,
        iron_bars_block: skip_rendering.iron_bars_block,
        bars_tag: skip_rendering.bars_tag,
        north: skip_rendering.north,
        south: skip_rendering.south,
        west: skip_rendering.west,
        east: skip_rendering.east,
        connection_property_mask: skip_rendering.connection_property_mask,
    }
}

pub(crate) fn renderer_fluid(fluid: bbb_render_types::TerrainFluidState) -> TerrainFluid {
    let kind = match fluid.kind {
        bbb_render_types::TerrainFluidKind::Water => TerrainFluidKind::Water,
        bbb_render_types::TerrainFluidKind::Lava => TerrainFluidKind::Lava,
    };
    TerrainFluid::new(kind, fluid.amount, fluid.falling)
}

/// Per element of the terrain shape `shape` bakes into, the bits of the faces
/// `tinted` marks, following how `terrain_render_shape` assigns element tints;
/// empty when no element face is tinted.
fn shape_tint_mask(
    shape: &BlockModelShape,
    fallback_tinted: [bool; 6],
    tinted: &dyn Fn(Option<i32>) -> bool,
) -> Arc<[u8]> {
    let bits =
        |tinted: [bool; 6]| (0..6).fold(0_u8, |bits, face| bits | (u8::from(tinted[face]) << face));
    let mask: Vec<u8> = match shape {
        BlockModelShape::Crosses(crosses) => crosses
            .iter()
            .map(|cross| {
                bits(std::array::from_fn(|face| {
                    if cross.face_textures[face].is_some() {
                        tinted(cross.face_tint_indices[face])
                    } else {
                        fallback_tinted[face]
                    }
                }))
            })
            .collect(),
        BlockModelShape::Boxes(boxes) => boxes
            .iter()
            .map(|model_box| {
                bits(std::array::from_fn(|face| {
                    if model_box.face_present[face] {
                        tinted(model_box.face_tint_indices[face])
                    } else {
                        fallback_tinted[face]
                    }
                }))
            })
            .collect(),
        BlockModelShape::Quads(quads) => quads
            .iter()
            .map(|quad| {
                u8::from(if quad.texture.is_some() {
                    tinted(quad.tint_index)
                } else {
                    fallback_tinted[quad.face.index()]
                })
            })
            .collect(),
        _ => Vec::new(),
    };
    if mask.iter().all(|bits| *bits == 0) {
        Arc::from([])
    } else {
        Arc::from(mask)
    }
}

fn shape_element_count(shape: &TerrainRenderShape) -> usize {
    match shape {
        TerrainRenderShape::Crosses(crosses) => crosses.len(),
        TerrainRenderShape::Boxes(boxes) => boxes.len(),
        TerrainRenderShape::Quads(quads) => quads.len(),
        TerrainRenderShape::Cube
        | TerrainRenderShape::Cross { .. }
        | TerrainRenderShape::Box { .. } => 0,
    }
}

/// `shape` with `tint` on the element faces `tinted` marks.
fn tint_shape(shape: &TerrainRenderShape, tinted: &[u8], tint: TerrainTint) -> TerrainRenderShape {
    if tinted.is_empty() {
        return shape.clone();
    }
    let faces = |element: usize, tints: [TerrainTint; 6]| {
        std::array::from_fn(|face| {
            if tinted[element] & (1 << face) != 0 {
                tint
            } else {
                tints[face]
            }
        })
    };
    match shape {
        TerrainRenderShape::Crosses(crosses) => TerrainRenderShape::Crosses(
            crosses
                .iter()
                .enumerate()
                .map(|(element, cross)| TerrainCross {
                    tint: faces(element, cross.tint),
                    ..*cross
                })
                .collect(),
        ),
        TerrainRenderShape::Boxes(boxes) => TerrainRenderShape::Boxes(
            boxes
                .iter()
                .enumerate()
                .map(|(element, model_box)| crate::terrain::TerrainBox {
                    tint: faces(element, model_box.tint),
                    ..*model_box
                })
                .collect(),
        ),
        TerrainRenderShape::Quads(quads) => TerrainRenderShape::Quads(
            quads
                .iter()
                .enumerate()
                .map(|(element, quad)| crate::terrain::TerrainQuad {
                    tint: if tinted[element] != 0 {
                        tint
                    } else {
                        quad.tint
                    },
                    ..*quad
                })
                .collect(),
        ),
        shape => shape.clone(),
    }
}

fn collision_full_block(
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> bool {
    if let Some(shape) =
        bbb_protocol::block_state_types::vanilla_block_state_static_collision_shape(block_state_id)
    {
        let mut boxes = shape.boxes();
        return boxes.len() == 1
            && boxes.next().is_some_and(|full| {
                [full.min_x, full.min_y, full.min_z] == [0.0; 3]
                    && [full.max_x, full.max_y, full.max_z] == [1.0; 3]
            });
    }
    content_registry
        .block_by_state_id(block_state_id)
        .is_some_and(|(spec, _)| spec.full_cube_collision)
}
