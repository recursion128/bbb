//! World -> renderer projection for bell block-entity models.
//!
//! Vanilla renders bells through the `BlockEntityRenderDispatcher` +
//! `BellRenderer` pair: per bell block entity, a `BellRenderState` carrying
//! `ticks + partialTicks`, `shaking ? clickDirection : null`, and the light
//! coords sampled at the block position. bbb has no separate BER dispatch;
//! bell instances ride the existing single entity-model submission stream as
//! `EntityModelKind::Bell`, like the chest and the sign. The bell body
//! renders identically for all four `attachment` variants (the support frame
//! is part of the `bell_*` block models the terrain path draws), so the
//! projection carries no attachment data.

use bbb_renderer::{BellShakeDirection, EntityModelInstance};
use bbb_world::{
    BellModelSourceState, BellShakeDirectionKind as WorldBellShakeDirectionKind, TerrainLight,
    WorldStore,
};

/// Like chests/signs, bell instances are projected from block states, not
/// the entity list, so they carry a sentinel id no server entity can use.
const BELL_BLOCK_MODEL_ENTITY_ID: i32 = -1;

/// Projects every bell block in the loaded chunks into a bell model
/// instance: position at the block min corner, the vanilla
/// `extractRenderState` shake fields (`ticks + partialTicks`, direction only
/// while shaking), and the block-position light.
pub(crate) fn bell_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    partial_tick: f32,
) -> Vec<EntityModelInstance> {
    world
        .bell_model_source_states(partial_tick)
        .into_iter()
        .map(|source| bell_model_instance(&source, world))
        .collect()
}

fn bell_model_instance(source: &BellModelSourceState, world: &WorldStore) -> EntityModelInstance {
    let mut instance = EntityModelInstance::bell(
        BELL_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
    )
    .with_bell_ticks(source.ticks)
    .with_bell_shake_direction(source.shake_direction.map(bell_shake_direction));
    if let Some(light) = world.sample_block_light(source.pos) {
        instance = instance.with_light_coords(bell_light_coords(light));
    }
    instance
}

fn bell_shake_direction(direction: WorldBellShakeDirectionKind) -> BellShakeDirection {
    match direction {
        WorldBellShakeDirectionKind::Down => BellShakeDirection::Down,
        WorldBellShakeDirectionKind::Up => BellShakeDirection::Up,
        WorldBellShakeDirectionKind::North => BellShakeDirection::North,
        WorldBellShakeDirectionKind::South => BellShakeDirection::South,
        WorldBellShakeDirectionKind::East => BellShakeDirection::East,
        WorldBellShakeDirectionKind::West => BellShakeDirection::West,
    }
}

/// Vanilla `LightCoordsUtil.pack(block, sky)` (`block << 4 | sky << 20`) over
/// the raw stored light sample — the bell render state's `lightCoords`.
fn bell_light_coords(light: TerrainLight) -> u32 {
    u32::from(light.block.min(15)) << 4 | u32::from(light.sky.min(15)) << 20
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos};
    use bbb_renderer::EntityModelKind;
    use bbb_world::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension,
    };
    use std::collections::BTreeMap;

    const VANILLA_AIR_BLOCK_STATE_ID: i32 = 0;

    fn world_with_air_chunk() -> WorldStore {
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
                    VANILLA_AIR_BLOCK_STATE_ID,
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

    fn set_bell(world: &mut WorldStore, pos: BlockPos, attachment: &str) {
        let properties: BTreeMap<String, String> = [
            ("attachment".to_string(), attachment.to_string()),
            ("facing".to_string(), "north".to_string()),
            ("powered".to_string(), "false".to_string()),
        ]
        .into_iter()
        .collect();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties("minecraft:bell", &properties)
            .unwrap_or_else(|| panic!("no registered state for minecraft:bell {properties:?}"));
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_state_id: state_id,
            })
        );
    }

    fn ring_bell(world: &mut WorldStore, pos: BlockPos, direction: u8) {
        world.apply_block_event(ProtocolBlockEvent {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            b0: 1,
            b1: direction,
            block_id: 0,
        });
    }

    #[test]
    fn projects_shaking_and_resting_bells_with_partial_ticks_and_light() {
        let mut world = world_with_air_chunk();
        let ringing_pos = BlockPos { x: 3, y: 4, z: 5 };
        let resting_pos = BlockPos { x: 6, y: 4, z: 5 };
        set_bell(&mut world, ringing_pos, "floor");
        set_bell(&mut world, resting_pos, "ceiling");
        // Direction 3D data value 4 = WEST.
        ring_bell(&mut world, ringing_pos, 4);
        world.advance_bell_shake_ticks(10);

        let instances = bell_model_instances_from_world_at_partial_tick(&world, 0.25);
        assert_eq!(instances.len(), 2);
        let ringing = &instances[0];
        let resting = &instances[1];
        assert_eq!(ringing.kind, EntityModelKind::Bell);
        assert_eq!(ringing.entity_id, BELL_BLOCK_MODEL_ENTITY_ID);
        assert_eq!(ringing.position, [3.0, 4.0, 5.0]);
        // BellRenderState.ticks = ticks + partialTicks.
        assert!((ringing.render_state.bell_ticks - 10.25).abs() < 1e-6);
        assert_eq!(
            ringing.render_state.bell_shake_direction,
            Some(BellShakeDirection::West)
        );
        // BellRenderer.submit applies no facing/attachment yaw.
        assert_eq!(ringing.render_state.body_rot, 0.0);
        // Empty light data: sky falls back to 15, block to 0 -> pack(0, 15).
        assert_eq!(ringing.render_state.light_coords, 15 << 20);
        // The ceiling bell renders the same body, resting at partial ticks
        // with no shake direction.
        assert_eq!(resting.kind, EntityModelKind::Bell);
        assert!((resting.render_state.bell_ticks - 0.25).abs() < 1e-6);
        assert_eq!(resting.render_state.bell_shake_direction, None);
    }

    #[test]
    fn shake_ends_after_the_vanilla_duration() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_bell(&mut world, pos, "single_wall");
        ring_bell(&mut world, pos, 2);
        world.advance_bell_shake_ticks(50);
        let instances = bell_model_instances_from_world_at_partial_tick(&world, 0.0);
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].render_state.bell_ticks, 0.0);
        assert_eq!(instances[0].render_state.bell_shake_direction, None);
    }

    #[test]
    fn packs_bell_light_coords_like_vanilla() {
        assert_eq!(
            bell_light_coords(TerrainLight { sky: 15, block: 0 }),
            15 << 20
        );
        assert_eq!(
            bell_light_coords(TerrainLight { sky: 7, block: 9 }),
            9 << 4 | 7 << 20
        );
    }
}
