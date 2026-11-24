use crate::{BlockPos, TerrainFluidKind, TerrainFluidState, WorldStore};

use crate::entities::EntityBoatStatus;

pub(crate) const fn to_facts_fluid_kind(kind: TerrainFluidKind) -> bbb_facts::fluid::FluidKind {
    match kind {
        TerrainFluidKind::Water => bbb_facts::fluid::FluidKind::Water,
        TerrainFluidKind::Lava => bbb_facts::fluid::FluidKind::Lava,
    }
}

pub(crate) const fn to_facts_fluid_sample(
    fluid: TerrainFluidState,
) -> bbb_facts::fluid::FluidSample {
    bbb_facts::fluid::FluidSample {
        kind: to_facts_fluid_kind(fluid.kind),
        amount: fluid.amount,
        falling: fluid.falling,
        is_source: fluid.amount == 8 && !fluid.falling,
    }
}

/// Vanilla `FlowingFluid.getHeight(level, pos)`: a fluid column reaches full height
/// (`1.0`) when the same fluid is directly above, otherwise it is the level-derived own
/// height. Shared by the local-player fluid contact scan and the entity in-water probe so
/// both stay byte-for-byte aligned with the vanilla surface height.
pub(crate) fn fluid_height_at(world: &WorldStore, pos: BlockPos, fluid: TerrainFluidState) -> f64 {
    let same_fluid_above = pos.y.checked_add(1).is_some_and(|above_y| {
        world
            .block_view(BlockPos {
                x: pos.x,
                y: above_y,
                z: pos.z,
            })
            .and_then(|block| block.fluid)
            .is_some_and(|above| above.kind == fluid.kind)
    });
    bbb_facts::fluid::fluid_height_f64(Some(to_facts_fluid_sample(fluid)), same_fluid_above)
}

impl WorldStore {
    /// Samples a loaded block's fluid state together with vanilla
    /// `FluidState.getHeight(level, pos)`. Keeping the neighbour-column check
    /// in `bbb-client-world` gives movement, effects, and renderer projections one
    /// authoritative `FlowingFluid.getHeight` implementation.
    pub fn fluid_state_and_height_at(&self, pos: BlockPos) -> Option<(TerrainFluidState, f64)> {
        let fluid = self.block_view(pos)?.fluid?;
        Some((fluid, fluid_height_at(self, pos, fluid)))
    }
}

/// Vanilla `Entity.isInWater()` (`Entity.wasTouchingWater`, refreshed each tick by
/// `updateFluidStateAndDoFluidPushing(FluidTags.WATER, ...)`): the entity's bounding box,
/// deflated by [`bbb_facts::fluid::FLUID_BOX_MARGIN`], is scanned block by block and the entity
/// is "in water" when any water column's surface (`y + getHeight`) reaches the deflated
/// box bottom. `min`/`max` are the world-space AABB (`position + EntityDimensions`).
///
/// This is a per-frame projection from the world fluid state rather than the entity's own
/// simulated `wasTouchingWater` (the client does not run entity physics), but the overlap
/// test itself is the vanilla algorithm.
pub(crate) fn world_aabb_in_water(world: &WorldStore, min: [f64; 3], max: [f64; 3]) -> bool {
    if world.chunks.is_empty() || !min.iter().chain(&max).all(|coord| coord.is_finite()) {
        return false;
    }
    let margin = bbb_facts::fluid::FLUID_BOX_MARGIN;
    let dmin = [min[0] + margin, min[1] + margin, min[2] + margin];
    let dmax = [max[0] - margin, max[1] - margin, max[2] - margin];
    if dmin[0] >= dmax[0] || dmin[1] >= dmax[1] || dmin[2] >= dmax[2] {
        return false;
    }
    let min_x = dmin[0].floor() as i32;
    let max_x = (dmax[0].ceil() as i32).saturating_sub(1);
    let min_y = dmin[1].floor() as i32;
    let max_y = (dmax[1].ceil() as i32).saturating_sub(1);
    let min_z = dmin[2].floor() as i32;
    let max_z = (dmax[2].ceil() as i32).saturating_sub(1);

    let dim = world.dimension();
    let min_y = min_y.max(dim.min_y);
    let max_y = max_y.min(dim.min_y.saturating_add(dim.height).saturating_sub(1));
    if min_x > max_x || min_y > max_y || min_z > max_z {
        return false;
    }

    let min_cx = min_x.div_euclid(16);
    let max_cx = max_x.div_euclid(16);
    let min_cz = min_z.div_euclid(16);
    let max_cz = max_z.div_euclid(16);
    let chunk_span_x = (i64::from(max_cx) - i64::from(min_cx)).saturating_add(1);
    let chunk_span_z = (i64::from(max_cz) - i64::from(min_cz)).saturating_add(1);
    let rect_chunks = chunk_span_x.saturating_mul(chunk_span_z);
    let min_sy = min_y.div_euclid(16);
    let max_sy = max_y.div_euclid(16);
    let min_section_y = dim.min_section_y();

    let column_in_water = |column: &crate::ChunkColumn| -> bool {
        let cx = column.pos.x;
        let cz = column.pos.z;
        let x_start = min_x.max(cx.saturating_mul(16));
        let x_end = max_x.min(cx.saturating_mul(16).saturating_add(15));
        let z_start = min_z.max(cz.saturating_mul(16));
        let z_end = max_z.min(cz.saturating_mul(16).saturating_add(15));
        for sy in min_sy..=max_sy {
            let Some(section) = column.sections.get((sy - min_section_y) as usize) else {
                continue;
            };
            if section.fluid_count == 0 {
                continue;
            }
            let y_start = min_y.max(sy.saturating_mul(16));
            let y_end = max_y.min(sy.saturating_mul(16).saturating_add(15));
            for y in y_start..=y_end {
                for z in z_start..=z_end {
                    for x in x_start..=x_end {
                        let pos = BlockPos { x, y, z };
                        let Some(fluid) = world.block_view(pos).and_then(|block| block.fluid)
                        else {
                            continue;
                        };
                        if fluid.kind == TerrainFluidKind::Water
                            && f64::from(y) + fluid_height_at(world, pos, fluid) >= dmin[1]
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    };

    if rect_chunks > world.chunks.len() as i64 {
        for column in world.chunks.iter() {
            if column.pos.x >= min_cx
                && column.pos.x <= max_cx
                && column.pos.z >= min_cz
                && column.pos.z <= max_cz
                && column_in_water(column)
            {
                return true;
            }
        }
        false
    } else {
        for cz in min_cz..=max_cz {
            for cx in min_cx..=max_cx {
                if let Some(column) = world.probe_chunk(crate::ChunkPos { x: cx, z: cz }) {
                    if column_in_water(column) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Vanilla `AbstractBoat.isUnderwater()`: scan only the thin slice at the boat AABB top
/// (`aabb.maxY .. aabb.maxY + 0.001`) and report underwater when the water surface is
/// strictly above that top. The source-vs-flowing distinction controls boat physics status
/// in vanilla, but the render state only needs the boolean `isUnderWater` gate.
pub(crate) fn world_aabb_boat_underwater(world: &WorldStore, min: [f64; 3], max: [f64; 3]) -> bool {
    bbb_facts::fluid::boat_is_underwater((min, max), |[x, y, z]| {
        let pos = BlockPos { x, y, z };
        let fluid = world.block_view(pos).and_then(|block| block.fluid)?;
        (fluid.kind == TerrainFluidKind::Water).then(|| {
            (
                fluid_height_at(world, pos, fluid) as f32,
                fluid.amount == 8 && !fluid.falling,
            )
        })
    })
    .is_some()
}

/// Exact water portion of Vanilla `AbstractBoat.getStatus()`. The caller owns
/// the following land-friction/air decision because it requires collision
/// shapes rather than fluid state.
pub(crate) fn boat_water_status(
    world: &WorldStore,
    min: [f64; 3],
    max: [f64; 3],
) -> (Option<EntityBoatStatus>, f64) {
    let (status, water_level) = bbb_facts::fluid::boat_water_status((min, max), |[x, y, z]| {
        let pos = BlockPos { x, y, z };
        let fluid = world.block_view(pos).and_then(|block| block.fluid)?;
        (fluid.kind == TerrainFluidKind::Water).then(|| {
            (
                fluid_height_at(world, pos, fluid) as f32,
                fluid.amount == 8 && !fluid.falling,
            )
        })
    });
    let status = status.map(|status| match status {
        bbb_facts::fluid::BoatWaterStatus::InWater => EntityBoatStatus::InWater,
        bbb_facts::fluid::BoatWaterStatus::UnderWater => EntityBoatStatus::UnderWater,
        bbb_facts::fluid::BoatWaterStatus::UnderFlowingWater => EntityBoatStatus::UnderFlowingWater,
    });
    (status, water_level)
}

pub(crate) fn boat_water_level_above(
    world: &WorldStore,
    min: [f64; 3],
    max: [f64; 3],
    last_yd: f64,
) -> f64 {
    f64::from(bbb_facts::fluid::boat_water_level_above(
        (min, max),
        last_yd,
        |[x, y, z]| {
            let pos = BlockPos { x, y, z };
            let fluid = world.block_view(pos).and_then(|block| block.fluid)?;
            (fluid.kind == TerrainFluidKind::Water)
                .then(|| fluid_height_at(world, pos, fluid) as f32)
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };

    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
    const FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID: i32 = 89;

    #[test]
    fn world_aabb_in_water_true_when_submerged_in_source_water() {
        // A cod-sized box (0.5 × 0.3 × 0.5) centred in a source-water column: the column's
        // surface is well above the box bottom, so the entity is in water.
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        set_block(
            &mut world,
            BlockPos { x: 0, y: 1, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        assert!(world_aabb_in_water(
            &world,
            [0.25, 0.5, 0.25],
            [0.75, 0.8, 0.75]
        ));
    }

    #[test]
    fn fluid_state_and_height_at_uses_same_fluid_above_and_level_height() {
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        set_block(
            &mut world,
            BlockPos { x: 0, y: 1, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        set_block(
            &mut world,
            BlockPos { x: 1, y: 0, z: 0 },
            FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID,
        );

        assert_eq!(
            world.fluid_state_and_height_at(BlockPos { x: 0, y: 0, z: 0 }),
            Some((
                TerrainFluidState::new(TerrainFluidKind::Water, 8, false),
                1.0,
            ))
        );
        assert_eq!(
            world.fluid_state_and_height_at(BlockPos { x: 1, y: 0, z: 0 }),
            Some((
                TerrainFluidState::new(TerrainFluidKind::Water, 5, false),
                5.0 / 9.0,
            ))
        );
        assert_eq!(
            world.fluid_state_and_height_at(BlockPos { x: 2, y: 0, z: 0 }),
            None
        );
    }

    #[test]
    fn world_aabb_in_water_false_in_air() {
        let world = empty_world();
        assert!(!world_aabb_in_water(
            &world,
            [0.25, 0.5, 0.25],
            [0.75, 0.8, 0.75]
        ));
    }

    #[test]
    fn world_aabb_in_water_false_when_box_sits_above_low_fluid_surface() {
        // A level-3 flowing column reaches 5/9 ≈ 0.556; a box whose bottom is at y = 0.7 is
        // above that surface, so vanilla `wasTouchingWater` stays false.
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID,
        );
        assert!(!world_aabb_in_water(
            &world,
            [0.25, 0.7, 0.25],
            [0.75, 1.0, 0.75]
        ));
    }

    #[test]
    fn world_aabb_in_water_true_when_box_bottom_dips_into_fluid_surface() {
        // The same level-3 column (surface 5/9 ≈ 0.556) with the box bottom lowered to
        // y = 0.5 now overlaps, so the entity is in water.
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID,
        );
        assert!(world_aabb_in_water(
            &world,
            [0.25, 0.5, 0.25],
            [0.75, 0.9, 0.75]
        ));
    }

    #[test]
    fn world_aabb_boat_underwater_true_when_top_is_below_water_surface() {
        // `AbstractBoat.isUnderwater` compares `aabb.maxY + 0.001` with the water surface
        // in the top slice of the boat AABB. A source block whose top is y=2 covers a boat
        // whose top is just under that surface.
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 1, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        assert!(world_aabb_boat_underwater(
            &world,
            [0.25, 1.0, 0.25],
            [0.75, 1.8, 0.75]
        ));
    }

    #[test]
    fn world_aabb_boat_underwater_false_when_only_bottom_touches_water() {
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        assert!(!world_aabb_boat_underwater(
            &world,
            [0.25, 0.5, 0.25],
            [0.75, 1.1, 0.75]
        ));
    }

    #[test]
    fn world_aabb_boat_underwater_false_in_air() {
        let world = empty_world();
        assert!(!world_aabb_boat_underwater(
            &world,
            [0.25, 1.0, 0.25],
            [0.75, 1.8, 0.75]
        ));
    }

    #[test]
    fn boat_water_status_distinguishes_source_flowing_surface_and_air() {
        let mut source = empty_world();
        set_block(
            &mut source,
            BlockPos { x: 0, y: 1, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        assert_eq!(
            boat_water_status(&source, [0.25, 1.0, 0.25], [0.75, 1.8, 0.75]),
            (Some(EntityBoatStatus::UnderWater), 1.8)
        );

        let mut flowing = empty_world();
        set_block(
            &mut flowing,
            BlockPos { x: 0, y: 1, z: 0 },
            FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID,
        );
        assert_eq!(
            boat_water_status(&flowing, [0.25, 0.5, 0.25], [0.75, 1.4, 0.75]),
            (Some(EntityBoatStatus::UnderFlowingWater), 1.4)
        );

        let mut surface = empty_world();
        set_block(
            &mut surface,
            BlockPos { x: 0, y: 0, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        assert_eq!(
            boat_water_status(&surface, [0.25, 0.8, 0.25], [0.75, 1.6, 0.75]),
            (
                Some(EntityBoatStatus::InWater),
                f64::from(8.0_f32 / 9.0_f32)
            )
        );

        assert_eq!(
            boat_water_status(&empty_world(), [0.25, 1.0, 0.25], [0.75, 1.8, 0.75]),
            (None, -f64::MAX)
        );
    }

    #[test]
    fn boat_water_level_above_stops_at_first_non_full_water_layer() {
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 1, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        set_block(
            &mut world,
            BlockPos { x: 0, y: 2, z: 0 },
            FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID,
        );

        let level = boat_water_level_above(&world, [0.25, 0.5, 0.25], [0.75, 1.8, 0.75], -1.0);
        assert_eq!(level, f64::from(2.0_f32 + 5.0_f32 / 9.0_f32));
    }

    fn empty_world() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![std::sync::Arc::new(ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    crate::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })],
            block_entities: std::sync::Arc::default(),
            light: LightData::default(),
        });
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_state_id,
            })
        );
    }
}
