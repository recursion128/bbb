//! Test fixtures: chunks described cell by cell with explicit render data,
//! interned into a block render table the production mesher reads.

use super::{
    block_table::{
        fluid_shape_height, smooth_lit, TerrainBlockState, TerrainStateModel, TerrainStateVariants,
    },
    build_terrain_section_meshes, TerrainBlockRenderTable, TerrainCardinalLighting,
    TerrainChunkSnapshot, TerrainFluid, TerrainLight, TerrainMaterialClass, TerrainRenderShape,
    TerrainSectionMeshLayers, TerrainSkipRendering, TerrainTextureAtlas, TerrainTint,
    TerrainTransparency,
};

/// One fixture cell: the render inputs of its block state plus its light.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainCell {
    pub material: TerrainMaterialClass,
    pub is_air: bool,
    pub solid_render: bool,
    pub legacy_solid: bool,
    pub blocks_motion: bool,
    pub collision_full_block: bool,
    pub face_occlusion_shape_indices: [u8; 6],
    pub view_blocking: bool,
    pub light_dampening: u8,
    pub shade_brightness: u8,
    pub fluid: Option<TerrainFluid>,
    pub fluid_texture_indices: [u32; 6],
    pub fluid_overlay_texture_index: Option<u32>,
    pub fluid_overlay_neighbor: bool,
    pub fluid_tint: [TerrainTint; 6],
    pub texture_indices: [u32; 6],
    pub render_shape: TerrainRenderShape,
    pub ambient_occlusion: bool,
    pub state_light_emission: u8,
    pub emissive_rendering: bool,
    pub light: TerrainLight,
    pub tint: [TerrainTint; 6],
    pub face_transparency: [TerrainTransparency; 6],
    pub skip_rendering: TerrainSkipRendering,
}

impl TerrainCell {
    pub const EMPTY: Self = Self {
        material: TerrainMaterialClass::Empty,
        is_air: true,
        solid_render: false,
        legacy_solid: false,
        blocks_motion: false,
        collision_full_block: false,
        face_occlusion_shape_indices: [0; 6],
        view_blocking: false,
        light_dampening: 0,
        shade_brightness: 255,
        fluid: None,
        fluid_texture_indices: [0; 6],
        fluid_overlay_texture_index: None,
        fluid_overlay_neighbor: false,
        fluid_tint: [TerrainTint::WHITE; 6],
        texture_indices: [0; 6],
        render_shape: TerrainRenderShape::Cube,
        ambient_occlusion: true,
        state_light_emission: 0,
        emissive_rendering: false,
        light: TerrainLight::FULL_BRIGHT,
        tint: [TerrainTint::WHITE; 6],
        face_transparency: [TerrainTransparency::OPAQUE; 6],
        skip_rendering: TerrainSkipRendering::NONE,
    };

    pub fn with_texture(material: TerrainMaterialClass, texture_index: u32) -> Self {
        Self::with_shape(material, texture_index, TerrainRenderShape::Cube)
    }

    pub fn with_shape(
        material: TerrainMaterialClass,
        texture_index: u32,
        render_shape: TerrainRenderShape,
    ) -> Self {
        Self {
            material,
            is_air: matches!(material, TerrainMaterialClass::Empty),
            solid_render: matches!(material, TerrainMaterialClass::Opaque),
            legacy_solid: matches!(material, TerrainMaterialClass::Opaque),
            blocks_motion: matches!(material, TerrainMaterialClass::Opaque),
            collision_full_block: matches!(material, TerrainMaterialClass::Opaque),
            face_occlusion_shape_indices: if matches!(material, TerrainMaterialClass::Opaque) {
                [1; 6]
            } else {
                [0; 6]
            },
            fluid_texture_indices: [texture_index; 6],
            texture_indices: [texture_index; 6],
            render_shape,
            ..Self::EMPTY
        }
    }

    pub fn with_light(mut self, light: TerrainLight) -> Self {
        self.light = light;
        self
    }

    pub fn with_state_light(mut self, light_emission: u8, emissive_rendering: bool) -> Self {
        self.state_light_emission = light_emission.min(15);
        self.emissive_rendering = emissive_rendering;
        self
    }

    pub fn with_fluid(mut self, fluid: TerrainFluid) -> Self {
        self.fluid = Some(fluid);
        self.fluid_texture_indices = self.texture_indices;
        self.fluid_tint = self.tint;
        self
    }

    pub fn with_fluid_render_data(
        mut self,
        fluid: TerrainFluid,
        texture_indices: [u32; 6],
        tint: [TerrainTint; 6],
    ) -> Self {
        self.fluid = Some(fluid);
        self.fluid_texture_indices = texture_indices;
        self.fluid_tint = tint;
        self
    }

    pub fn with_fluid_overlay_texture(mut self, texture_index: u32) -> Self {
        self.fluid_overlay_texture_index = Some(texture_index);
        self
    }

    pub fn with_fluid_overlay_neighbor(mut self) -> Self {
        self.fluid_overlay_neighbor = true;
        self
    }

    pub fn with_face_occlusion_shape_indices(mut self, indices: [u8; 6]) -> Self {
        self.face_occlusion_shape_indices = indices;
        self
    }

    pub fn with_ambient_light_facts(
        mut self,
        view_blocking: bool,
        light_dampening: u8,
        shade_brightness: u8,
    ) -> Self {
        self.view_blocking = view_blocking;
        self.light_dampening = light_dampening;
        self.shade_brightness = shade_brightness;
        self
    }

    pub fn with_ambient_occlusion(mut self, ambient_occlusion: bool) -> Self {
        self.ambient_occlusion = ambient_occlusion;
        self
    }

    pub fn with_tint(mut self, tint: [TerrainTint; 6]) -> Self {
        self.tint = tint;
        self
    }

    pub fn with_face_transparency(mut self, face_transparency: [TerrainTransparency; 6]) -> Self {
        self.face_transparency = face_transparency;
        self
    }

    pub fn with_skip_rendering(mut self, skip_rendering: TerrainSkipRendering) -> Self {
        self.skip_rendering = skip_rendering;
        self
    }

    pub fn with_fluid_solid_facts(
        mut self,
        solid_render: bool,
        legacy_solid: bool,
        blocks_motion: bool,
    ) -> Self {
        self.solid_render = solid_render;
        self.legacy_solid = legacy_solid;
        self.blocks_motion = blocks_motion;
        self
    }

    #[cfg(test)]
    pub(crate) fn block_state(&self) -> TerrainBlockState {
        self.block_state_and_model().0
    }

    pub(crate) fn block_state_and_model(&self) -> (TerrainBlockState, TerrainStateModel) {
        let fluid_overlay =
            self.fluid.is_some() && !matches!(self.material, TerrainMaterialClass::Fluid);
        let model = TerrainStateModel::new(
            self.material,
            fluid_overlay,
            self.texture_indices,
            self.tint,
            self.face_transparency,
            self.render_shape.clone(),
            smooth_lit(self.ambient_occlusion, self.state_light_emission),
        );
        let has_no_mesh = model.route == 0;
        let state = TerrainBlockState {
            model_offset_limits: [0.0; 2],
            material: self.material,
            is_air: self.is_air,
            has_no_mesh,
            solid_render: self.solid_render,
            legacy_solid: self.legacy_solid,
            blocks_motion: self.blocks_motion,
            collision_full_block: self.collision_full_block,
            face_occlusion_shape_indices: self.face_occlusion_shape_indices,
            view_blocking: self.view_blocking,
            light_dampening: self.light_dampening,
            shade_brightness: self.shade_brightness,
            fluid: self.fluid,
            fluid_shape_height: fluid_shape_height(&self.render_shape),
            fluid_texture_indices: self.fluid_texture_indices,
            fluid_overlay_texture_index: self.fluid_overlay_texture_index,
            fluid_overlay_neighbor: self.fluid_overlay_neighbor,
            state_light_emission: self.state_light_emission,
            emissive_rendering: self.emissive_rendering,
            skip_rendering: self.skip_rendering,
            variants: TerrainStateVariants::Fixed {
                model_index: 0,
                fluid_tint: self.fluid_tint,
            },
        };
        (state, model)
    }
}

/// One fixture chunk in the snapshot's y, z, x cell order.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainChunkFixture {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub min_y: i32,
    pub height: usize,
    pub cells: Vec<TerrainCell>,
    pub cardinal_lighting: TerrainCardinalLighting,
}

impl TerrainChunkFixture {
    pub fn new(
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        height: usize,
        cells: Vec<TerrainCell>,
    ) -> Self {
        assert_eq!(cells.len(), 16 * height * 16);
        Self {
            chunk_x,
            chunk_z,
            min_y,
            height,
            cells,
            cardinal_lighting: TerrainCardinalLighting::Default,
        }
    }

    pub fn with_cardinal_lighting(mut self, cardinal_lighting: TerrainCardinalLighting) -> Self {
        self.cardinal_lighting = cardinal_lighting;
        self
    }
}

/// Fixture chunks interned into one table and their slim snapshots.
pub struct TerrainFixtureWorld {
    pub table: TerrainBlockRenderTable,
    pub snapshots: Vec<TerrainChunkSnapshot>,
}

impl TerrainFixtureWorld {
    pub fn new(chunks: &[TerrainChunkFixture], atlas: TerrainTextureAtlas) -> Self {
        let mut states: Vec<TerrainCell> = Vec::new();
        let mut intern = |cell: &TerrainCell| {
            let key = TerrainCell {
                light: TerrainLight::FULL_BRIGHT,
                ..cell.clone()
            };
            let index = states
                .iter()
                .rposition(|state| *state == key)
                .unwrap_or_else(|| {
                    states.push(key);
                    states.len() - 1
                });
            index as u32
        };
        let snapshots = chunks
            .iter()
            .map(|chunk| {
                TerrainChunkSnapshot::from_cells(
                    chunk.chunk_x,
                    chunk.chunk_z,
                    chunk.min_y,
                    chunk.height,
                    chunk.cells.iter().map(|cell| (intern(cell), cell.light)),
                )
                .with_cardinal_lighting(chunk.cardinal_lighting)
            })
            .collect();
        Self {
            table: TerrainBlockRenderTable::from_states(
                states
                    .iter()
                    .map(TerrainCell::block_state_and_model)
                    .collect(),
                atlas,
            ),
            snapshots,
        }
    }

    /// Meshes `section_ys` of chunk `target` with every other chunk as its neighborhood.
    pub fn build(
        &self,
        target: usize,
        camera_position: [f32; 3],
        section_ys: &[i32],
    ) -> TerrainSectionMeshLayers {
        build_terrain_section_meshes(
            &self.snapshots[target],
            self.snapshots
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != target)
                .map(|(_, snapshot)| snapshot),
            &self.table,
            camera_position,
            section_ys,
        )
    }

    /// Every section of chunk `target`.
    pub fn build_all(&self, target: usize, camera_position: [f32; 3]) -> TerrainSectionMeshLayers {
        let snapshot = &self.snapshots[target];
        let section_ys = (0..snapshot.height)
            .step_by(16)
            .map(|start_y| (snapshot.min_y + start_y as i32).div_euclid(16))
            .collect::<Vec<_>>();
        self.build(target, camera_position, &section_ys)
    }
}
