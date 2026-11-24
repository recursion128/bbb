//! Per-block-state mesher inputs, resolved once per resource generation and
//! shared read-only by every mesh job.

use std::{borrow::Cow, ops::Deref, sync::Arc};

use bbb_render_types::TerrainBlockRenderContext;

use super::{
    TerrainFluid, TerrainLight, TerrainMaterialClass, TerrainRenderShape, TerrainSkipRendering,
    TerrainTextureAtlas, TerrainTint, TerrainTransparency,
};
use crate::scene::terrain_textures::{
    BlockRenderPosition, SectionTintAccess, TerrainResolvedState, TerrainResourceSource,
    TerrainTextureState,
};

pub(crate) const MESH_ROUTE_OPAQUE: u8 = 1 << 0;
pub(crate) const MESH_ROUTE_CUTOUT: u8 = 1 << 1;
pub(crate) const MESH_ROUTE_TRANSLUCENT: u8 = 1 << 2;
pub(crate) const MESH_ROUTE_FLUID_OVERLAY: u8 = 1 << 3;
const MESH_ROUTE_SURFACE: u8 = MESH_ROUTE_OPAQUE | MESH_ROUTE_CUTOUT | MESH_ROUTE_TRANSLUCENT;

/// `ModelBlockRenderer.tesselateBlock`: smooth lighting needs the model's ambient occlusion
/// (with the option on) and a state that emits no light.
pub(crate) fn smooth_lit(model_ambient_occlusion: bool, state_light_emission: u8) -> bool {
    model_ambient_occlusion && state_light_emission == 0
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TerrainTintSource {
    White,
    Fixed(TerrainTint),
    Grass,
    Foliage,
    DryFoliage,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TerrainResolvedTintKey {
    pub(crate) source: TerrainTintSource,
    pub(crate) wants_blend: bool,
    pub(crate) water_fluid: bool,
}

/// Everything the mesher reads about one block state.
#[derive(Debug, Clone, Copy)]
pub struct TerrainBlockState {
    pub(crate) model_offset_limits: [f32; 2],
    pub(crate) material: TerrainMaterialClass,
    /// Exact loaded-empty-section input. Invisible non-air blocks are false.
    pub(crate) is_air: bool,
    pub(crate) has_no_mesh: bool,
    /// Vanilla `BlockState.isSolidRender()`, the `VisGraph` and fluid backward-up-face input.
    pub(crate) solid_render: bool,
    /// Vanilla `BlockState.isSolid()`, used by `FluidRenderer.getHeight`.
    pub(crate) legacy_solid: bool,
    /// Vanilla `BlockState.blocksMotion()`, used by `FlowingFluid.getFlow`.
    pub(crate) blocks_motion: bool,
    /// `BlockState.isCollisionShapeFullBlock`, which makes any planar model face on that block
    /// light like a face on the block boundary (`BlockModelLighter.prepareQuadShape`).
    pub(crate) collision_full_block: bool,
    /// Vanilla `getFaceOcclusionShape` palette indices in Down/Up/North/South/West/East order.
    pub(crate) face_occlusion_shape_indices: [u8; 6],
    /// `BlockState.isViewBlocking()` and `getLightDampening()`, paired by `BlockModelLighter`.
    pub(crate) view_blocking: bool,
    pub(crate) light_dampening: u8,
    /// `BlockState.getShadeBrightness()` encoded on 0..=255.
    pub(crate) shade_brightness: u8,
    pub(crate) fluid: Option<TerrainFluid>,
    /// Height of a fluid-material state's box shape.
    pub(crate) fluid_shape_height: Option<f32>,
    pub(crate) fluid_texture_indices: [u32; 6],
    pub(crate) fluid_overlay_texture_index: Option<u32>,
    pub(crate) fluid_overlay_neighbor: bool,
    /// Fixed-state inputs to Vanilla `LevelRenderer.getLightCoords`.
    pub(crate) state_light_emission: u8,
    pub(crate) emissive_rendering: bool,
    pub(crate) skip_rendering: TerrainSkipRendering,
    pub(crate) variants: TerrainStateVariants,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum TerrainStateVariants {
    Fixed {
        model_index: u32,
        fluid_tint: [TerrainTint; 6],
    },
    BiomeTinted {
        model_index: u32,
        tint_key: TerrainResolvedTintKey,
    },
    Weighted {
        model_start: u32,
        choices_index: u32,
        tint_key: TerrainResolvedTintKey,
    },
    Dynamic {
        dynamic_index: u32,
    },
}

/// One model variant of a block state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TerrainStateModel {
    pub(crate) texture_indices: [u32; 6],
    pub(crate) face_tints: [TerrainTint; 6],
    /// Faces that take the cell's biome tint instead of `face_tints`.
    pub(crate) face_tinted: [bool; 6],
    pub(crate) face_transparency: [TerrainTransparency; 6],
    pub(crate) render_shape: TerrainRenderShape,
    /// Per element of `render_shape`, the bits of the faces that take the
    /// cell's biome tint instead of the element tint; empty when none do.
    pub(crate) shape_tinted: Arc<[u8]>,
    pub(crate) ambient_occlusion: bool,
    pub(crate) route: u8,
}

impl TerrainStateModel {
    pub(crate) fn new(
        material: TerrainMaterialClass,
        fluid_overlay: bool,
        texture_indices: [u32; 6],
        face_tints: [TerrainTint; 6],
        face_transparency: [TerrainTransparency; 6],
        render_shape: TerrainRenderShape,
        ambient_occlusion: bool,
    ) -> Self {
        let mut route = match material {
            TerrainMaterialClass::Empty => 0,
            TerrainMaterialClass::Opaque => MESH_ROUTE_OPAQUE,
            TerrainMaterialClass::Cutout => MESH_ROUTE_CUTOUT,
            TerrainMaterialClass::Fluid | TerrainMaterialClass::Translucent => {
                MESH_ROUTE_TRANSLUCENT
            }
        };
        if has_texture_layer_overrides(face_transparency, &render_shape) {
            route |= MESH_ROUTE_SURFACE;
        }
        if fluid_overlay {
            route |= MESH_ROUTE_FLUID_OVERLAY;
        }
        Self {
            texture_indices,
            face_tints,
            face_transparency,
            render_shape,
            face_tinted: [false; 6],
            shape_tinted: Arc::from([]),
            ambient_occlusion,
            route,
        }
    }

    pub(crate) fn empty() -> Self {
        Self::new(
            TerrainMaterialClass::Empty,
            false,
            [0; 6],
            [TerrainTint::WHITE; 6],
            [TerrainTransparency::OPAQUE; 6],
            TerrainRenderShape::Cube,
            true,
        )
    }

    pub(crate) fn needs_cell_tint(&self) -> bool {
        self.face_tinted.contains(&true) || !self.shape_tinted.is_empty()
    }
}

/// Model, tint and fluid tint of one cell.
pub(crate) struct TerrainCellRender<'a> {
    pub(crate) model: Cow<'a, TerrainStateModel>,
    pub(crate) tint: TerrainTint,
    pub(crate) fluid_tint: [TerrainTint; 6],
}

impl TerrainCellRender<'_> {
    pub(crate) fn face_tint(&self, face: usize) -> TerrainTint {
        if self.model.face_tinted[face] {
            self.tint
        } else {
            self.model.face_tints[face]
        }
    }

    /// The tint of face `face` of shape element `element`, whose own tint is `tint`.
    pub(crate) fn shape_tint(&self, element: usize, face: usize, tint: TerrainTint) -> TerrainTint {
        if self
            .model
            .shape_tinted
            .get(element)
            .is_some_and(|faces| faces & (1 << face) != 0)
        {
            self.tint
        } else {
            tint
        }
    }
}

/// A neighbor cell as the mesher samples it: state facts plus stored light.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CellRef<'a> {
    pub(crate) state: &'a TerrainBlockState,
    pub(crate) light: TerrainLight,
}

impl Deref for CellRef<'_> {
    type Target = TerrainBlockState;

    fn deref(&self) -> &TerrainBlockState {
        self.state
    }
}

impl CellRef<'_> {
    pub(crate) const fn empty() -> Self {
        Self {
            state: &TerrainBlockState::EMPTY,
            light: TerrainLight::FULL_BRIGHT,
        }
    }
}

impl TerrainBlockState {
    pub(crate) const EMPTY: Self = Self {
        model_offset_limits: [0.0; 2],
        material: TerrainMaterialClass::Empty,
        is_air: true,
        has_no_mesh: true,
        solid_render: false,
        legacy_solid: false,
        blocks_motion: false,
        collision_full_block: false,
        face_occlusion_shape_indices: [0; 6],
        view_blocking: false,
        light_dampening: 0,
        shade_brightness: 255,
        fluid: None,
        fluid_shape_height: None,
        fluid_texture_indices: [0; 6],
        fluid_overlay_texture_index: None,
        fluid_overlay_neighbor: false,
        state_light_emission: 0,
        emissive_rendering: false,
        skip_rendering: TerrainSkipRendering::NONE,
        variants: TerrainStateVariants::Fixed {
            model_index: 0,
            fluid_tint: [TerrainTint::WHITE; 6],
        },
    };

    #[inline]
    pub(crate) fn fixed_model_has_no_mesh(&self) -> bool {
        self.has_no_mesh
    }
}

pub(crate) struct TerrainBlockTableParts {
    pub(crate) states: Box<[TerrainBlockState]>,
    pub(crate) unknown: TerrainBlockState,
    pub(crate) models: Box<[TerrainStateModel]>,
    pub(crate) side_table: TerrainResolvedState,
}

/// Lock-free flat block render table resolved eagerly per resource generation.
pub struct TerrainBlockRenderTable {
    states: Box<[TerrainBlockState]>,
    unknown: TerrainBlockState,
    models: Box<[TerrainStateModel]>,
    source: TerrainTableSource,
}

enum TerrainTableSource {
    Fixture(TerrainTextureAtlas),
    Resources {
        source: Box<TerrainResourceSource>,
        side_table: Box<TerrainResolvedState>,
    },
}

impl std::fmt::Debug for TerrainBlockRenderTable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TerrainBlockRenderTable")
            .field("states", &self.states.len())
            .field("models", &self.models.len())
            .finish_non_exhaustive()
    }
}

impl TerrainBlockRenderTable {
    pub fn from_resources(source: TerrainResourceSource) -> Self {
        let parts = source.build_table();
        Self {
            states: parts.states,
            unknown: parts.unknown,
            models: parts.models,
            source: TerrainTableSource::Resources {
                source: Box::new(source),
                side_table: Box::new(parts.side_table),
            },
        }
    }

    /// A table whose block state ids index `states`; ids past the end are air.
    pub(crate) fn from_states(
        states: Vec<(TerrainBlockState, TerrainStateModel)>,
        atlas: TerrainTextureAtlas,
    ) -> Self {
        let mut models = vec![TerrainStateModel::empty()];
        let mut flat_states = Vec::with_capacity(states.len());
        for (mut state, model) in states {
            state.has_no_mesh = model.route == 0;
            let model_index = if models.last() == Some(&model) {
                (models.len() - 1) as u32
            } else {
                let index = models.len() as u32;
                models.push(model);
                index
            };
            if let TerrainStateVariants::Fixed {
                model_index: slot, ..
            } = &mut state.variants
            {
                *slot = model_index;
            }
            flat_states.push(state);
        }
        Self {
            states: flat_states.into_boxed_slice(),
            unknown: TerrainBlockState::EMPTY,
            models: models.into_boxed_slice(),
            source: TerrainTableSource::Fixture(atlas),
        }
    }

    /// Whether the table was resolved from the current textures and tag context.
    pub fn is_current(
        &self,
        textures: &TerrainTextureState,
        context: &TerrainBlockRenderContext,
    ) -> bool {
        match &self.source {
            TerrainTableSource::Fixture(_) => true,
            TerrainTableSource::Resources { source, .. } => source.is_current(textures, context),
        }
    }

    pub(crate) fn atlas(&self) -> &TerrainTextureAtlas {
        match &self.source {
            TerrainTableSource::Fixture(atlas) => atlas,
            TerrainTableSource::Resources { source, .. } => source.atlas(),
        }
    }

    #[inline]
    pub(crate) fn state(&self, block_state_id: u32) -> &TerrainBlockState {
        self.states
            .get(block_state_id as usize)
            .unwrap_or(&self.unknown)
    }

    #[inline]
    pub(crate) fn cell_ref(&self, block_state_id: u32, light: TerrainLight) -> CellRef<'_> {
        CellRef {
            state: self.state(block_state_id),
            light,
        }
    }

    pub(crate) fn model<'a>(
        &'a self,
        state: &'a TerrainBlockState,
        position: BlockRenderPosition,
    ) -> Cow<'a, TerrainStateModel> {
        match state.variants {
            TerrainStateVariants::Fixed { model_index, .. }
            | TerrainStateVariants::BiomeTinted { model_index, .. } => {
                Cow::Borrowed(&self.models[model_index as usize])
            }
            TerrainStateVariants::Weighted {
                model_start,
                choices_index,
                ..
            } => match &self.source {
                TerrainTableSource::Resources { side_table, .. } => {
                    let offset = side_table.combination(choices_index, position);
                    let model_index = side_table.variant_model_index(model_start, offset);
                    Cow::Borrowed(&self.models[model_index as usize])
                }
                TerrainTableSource::Fixture(_) => {
                    unreachable!("fixture tables hold fixed models")
                }
            },
            TerrainStateVariants::Dynamic { dynamic_index } => match &self.source {
                TerrainTableSource::Resources { source, side_table } => {
                    Cow::Owned(source.dynamic_model(state, side_table, dynamic_index, position))
                }
                TerrainTableSource::Fixture(_) => {
                    unreachable!("fixture tables hold fixed models")
                }
            },
        }
    }

    pub(crate) fn biome_blend_radius(&self) -> u32 {
        match &self.source {
            TerrainTableSource::Fixture(_) => 0,
            TerrainTableSource::Resources { source, .. } => source.biome_blend_radius(),
        }
    }

    /// The cell tint and fluid tint of a cell whose model is `model`.
    pub(crate) fn cell_render<'a>(
        &self,
        state: &TerrainBlockState,
        model: Cow<'a, TerrainStateModel>,
        biome_id: Option<i32>,
        position: BlockRenderPosition,
        biome_at: &dyn Fn(BlockRenderPosition) -> Option<i32>,
    ) -> TerrainCellRender<'a> {
        self.cell_render_with(state, model, |source, tint_key, model| {
            source.tints_for_key(tint_key, model, biome_id, position, biome_at)
        })
    }

    /// The cell tint and fluid tint resolved against the active section's 2D biome prefix cache.
    pub(crate) fn cell_render_in_section<'a>(
        &self,
        state: &TerrainBlockState,
        model: Cow<'a, TerrainStateModel>,
        biome_id: Option<i32>,
        position: BlockRenderPosition,
        access: &mut SectionTintAccess<'_>,
    ) -> TerrainCellRender<'a> {
        self.cell_render_with(state, model, |source, tint_key, model| {
            source.tints_for_key_in_section(tint_key, model, biome_id, position, access)
        })
    }

    fn cell_render_with<'a>(
        &self,
        state: &TerrainBlockState,
        model: Cow<'a, TerrainStateModel>,
        resolve_tints: impl FnOnce(
            &TerrainResourceSource,
            TerrainResolvedTintKey,
            &TerrainStateModel,
        ) -> (TerrainTint, TerrainTint),
    ) -> TerrainCellRender<'a> {
        match state.variants {
            TerrainStateVariants::Fixed { fluid_tint, .. } => TerrainCellRender {
                model,
                tint: TerrainTint::WHITE,
                fluid_tint,
            },
            TerrainStateVariants::BiomeTinted { tint_key, .. }
            | TerrainStateVariants::Weighted { tint_key, .. } => match &self.source {
                TerrainTableSource::Resources { source, .. } => {
                    let (tint, fluid_tint) = resolve_tints(source, tint_key, &model);
                    TerrainCellRender {
                        model,
                        tint,
                        fluid_tint: [fluid_tint; 6],
                    }
                }
                TerrainTableSource::Fixture(_) => {
                    unreachable!("fixture tables hold fixed models")
                }
            },
            TerrainStateVariants::Dynamic { dynamic_index } => match &self.source {
                TerrainTableSource::Resources { source, side_table } => {
                    let tint_key = side_table.dynamic_tint_key(dynamic_index);
                    let (tint, fluid_tint) = resolve_tints(source, tint_key, &model);
                    TerrainCellRender {
                        model,
                        tint,
                        fluid_tint: [fluid_tint; 6],
                    }
                }
                TerrainTableSource::Fixture(_) => {
                    unreachable!("fixture tables hold fixed models")
                }
            },
        }
    }

    /// Samples top-face texture UV center and biome/fluid tint for compact far-terrain LOD quads.
    pub fn sample_lod_surface(
        &self,
        block_state_id: u32,
        biome_id: Option<i32>,
        position: BlockRenderPosition,
    ) -> Option<LodSurfaceSample> {
        let state = self.state(block_state_id);
        if state.is_air || state.has_no_mesh {
            return None;
        }
        let model = self.model(state, position);
        if matches!(
            model.render_shape,
            TerrainRenderShape::Cross { .. } | TerrainRenderShape::Crosses(_)
        ) {
            return None;
        }
        let tex_idx = if state.fluid.is_some() {
            state.fluid_texture_indices[1]
        } else {
            model.texture_indices[1]
        };
        let rect = self.atlas().rect(tex_idx);
        let uv_center = [
            (rect.min[0] + rect.max[0]) * 0.5,
            (rect.min[1] + rect.max[1]) * 0.5,
        ];
        let cell = self.cell_render(state, model, biome_id, position, &|_| biome_id);
        let tint = if state.fluid.is_some() {
            cell.fluid_tint[1]
        } else {
            cell.face_tint(1)
        };
        Some(LodSurfaceSample {
            uv_center,
            uv_min: rect.min,
            uv_max: rect.max,
            tint_rgba: [tint.r, tint.g, tint.b, 255],
            light_emission: state.state_light_emission,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LodSurfaceSample {
    pub uv_center: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub tint_rgba: [u8; 4],
    pub light_emission: u8,
}

pub(crate) fn fluid_shape_height(shape: &TerrainRenderShape) -> Option<f32> {
    match shape {
        TerrainRenderShape::Box { to, .. } => Some((to[1] as f32 / 16.0).clamp(0.0, 1.0)),
        _ => None,
    }
}

fn has_texture_layer_overrides(
    face_transparency: [TerrainTransparency; 6],
    shape: &TerrainRenderShape,
) -> bool {
    let overrides = |transparency: &TerrainTransparency| {
        transparency.has_transparent || transparency.has_translucent
    };
    face_transparency.iter().any(overrides)
        || match shape {
            TerrainRenderShape::Box {
                face_transparency, ..
            } => face_transparency.iter().any(overrides),
            TerrainRenderShape::Boxes(model_boxes) => model_boxes
                .iter()
                .any(|model_box| model_box.face_transparency.iter().any(overrides)),
            TerrainRenderShape::Crosses(model_crosses) => model_crosses
                .iter()
                .any(|model_cross| model_cross.face_transparency.iter().any(overrides)),
            TerrainRenderShape::Quads(model_quads) => model_quads
                .iter()
                .any(|model_quad| overrides(&model_quad.transparency)),
            TerrainRenderShape::Cube | TerrainRenderShape::Cross { .. } => false,
        }
}
