use crate::{BlockPos, TerrainFluidKind, TerrainFluidState, WorldStore};

/// Vanilla `Entity.updateFluidStateAndDoFluidPushing` deflates the bounding box by this
/// amount (`box.deflate(0.001)`) before testing fluid overlap.
pub(crate) const FLUID_INTERACTION_BOX_DEFLATE: f64 = 0.001;

/// Vanilla `FlowingFluid.getHeight(level, pos)`: a fluid column reaches full height
/// (`1.0`) when the same fluid is directly above, otherwise it is the level-derived own
/// height. Shared by the local-player fluid contact scan and the entity in-water probe so
/// both stay byte-for-byte aligned with the vanilla surface height.
pub(crate) fn fluid_height_at(world: &WorldStore, pos: BlockPos, fluid: TerrainFluidState) -> f64 {
    let same_fluid_above = pos.y.checked_add(1).is_some_and(|above_y| {
        world
            .probe_block(BlockPos {
                x: pos.x,
                y: above_y,
                z: pos.z,
            })
            .and_then(|block| block.fluid)
            .is_some_and(|above| above.kind == fluid.kind)
    });
    if same_fluid_above {
        1.0
    } else {
        fluid.own_height()
    }
}

/// Vanilla `Entity.isInWater()` (`Entity.wasTouchingWater`, refreshed each tick by
/// `updateFluidStateAndDoFluidPushing(FluidTags.WATER, ...)`): the entity's bounding box,
/// deflated by [`FLUID_INTERACTION_BOX_DEFLATE`], is scanned block by block and the entity
/// is "in water" when any water column's surface (`y + getHeight`) reaches the deflated
/// box bottom. `min`/`max` are the world-space AABB (`position + EntityDimensions`).
///
/// This is a per-frame projection from the world fluid state rather than the entity's own
/// simulated `wasTouchingWater` (the client does not run entity physics), but the overlap
/// test itself is the vanilla algorithm.
pub(crate) fn world_aabb_in_water(world: &WorldStore, min: [f64; 3], max: [f64; 3]) -> bool {
    let deflated_min = [
        min[0] + FLUID_INTERACTION_BOX_DEFLATE,
        min[1] + FLUID_INTERACTION_BOX_DEFLATE,
        min[2] + FLUID_INTERACTION_BOX_DEFLATE,
    ];
    let deflated_max = [
        max[0] - FLUID_INTERACTION_BOX_DEFLATE,
        max[1] - FLUID_INTERACTION_BOX_DEFLATE,
        max[2] - FLUID_INTERACTION_BOX_DEFLATE,
    ];

    let min_x = deflated_min[0].floor() as i32;
    let max_x = deflated_max[0].ceil() as i32 - 1;
    let min_y = deflated_min[1].floor() as i32;
    let max_y = deflated_max[1].ceil() as i32 - 1;
    let min_z = deflated_min[2].floor() as i32;
    let max_z = deflated_max[2].ceil() as i32 - 1;

    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                let pos = BlockPos { x, y, z };
                let Some(fluid) = world.probe_block(pos).and_then(|block| block.fluid) else {
                    continue;
                };
                if fluid.kind != TerrainFluidKind::Water {
                    continue;
                }
                let fluid_top = f64::from(y) + fluid_height_at(world, pos, fluid);
                if fluid_top >= deflated_min[1] {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };

    const AIR_BLOCK_STATE_ID: i32 = 0;
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

    fn empty_world() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    AIR_BLOCK_STATE_ID,
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
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
