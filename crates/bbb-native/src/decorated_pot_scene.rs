//! World -> renderer projection for decorated pot block-entity models.
//!
//! Vanilla renders decorated pots through the `BlockEntityRenderDispatcher` +
//! `DecoratedPotRenderer` pair: per pot block entity, a
//! `DecoratedPotRenderState` carrying the block state's `HORIZONTAL_FACING`,
//! the four-face `PotDecorations` sherds, the wobble style/progress, and the
//! light coords sampled at the block position. bbb has no separate BER
//! dispatch; pot instances ride the existing single entity-model submission
//! stream as `EntityModelKind::DecoratedPot`, like the chest and the bell.
//! The sherd item -> pattern mapping comes from the generated renderer catalog,
//! extracted from `DecoratedPotPatterns.getPatternFromItem`'s source table.

use bbb_renderer::{DecoratedPotPattern, DecoratedPotWobble, EntityModelInstance};
use bbb_world::{
    DecoratedPotFacing as WorldDecoratedPotFacing, DecoratedPotModelSourceState,
    DecoratedPotWobbleStyleKind as WorldDecoratedPotWobbleStyleKind, WorldStore,
};

/// Like chests/bells, pot instances are projected from block states, not the
/// entity list, so they carry a sentinel id no server entity can use.
const DECORATED_POT_BLOCK_MODEL_ENTITY_ID: i32 = -1;

/// Projects every decorated pot block in the loaded chunks into a pot model
/// instance: position at the block min corner, `180 - facing.toYRot()` yaw
/// (`DecoratedPotRenderer.createModelTransformation`), the four sherd
/// patterns, the wobble style/progress, and the block-position light.
pub(crate) fn decorated_pot_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    partial_tick: f32,
) -> Vec<EntityModelInstance> {
    world
        .decorated_pot_model_source_states(partial_tick)
        .into_iter()
        .map(|source| decorated_pot_model_instance(&source, world))
        .collect()
}

fn decorated_pot_model_instance(
    source: &DecoratedPotModelSourceState,
    world: &WorldStore,
) -> EntityModelInstance {
    let mut instance = EntityModelInstance::decorated_pot(
        DECORATED_POT_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        decorated_pot_facing_y_rot(source.facing),
        source
            .back
            .as_deref()
            .and_then(DecoratedPotPattern::for_sherd_item_resource_id),
        source
            .left
            .as_deref()
            .and_then(DecoratedPotPattern::for_sherd_item_resource_id),
        source
            .right
            .as_deref()
            .and_then(DecoratedPotPattern::for_sherd_item_resource_id),
        source
            .front
            .as_deref()
            .and_then(DecoratedPotPattern::for_sherd_item_resource_id),
    );
    if let Some(wobble) = source.wobble {
        instance = instance.with_decorated_pot_wobble(Some(DecoratedPotWobble {
            positive: wobble.style == WorldDecoratedPotWobbleStyleKind::Positive,
            progress: wobble.progress,
        }));
    }
    if let Some(light) = world.sample_render_light(source.pos) {
        instance = instance.with_light_coords(light.packed());
    }
    instance
}

/// Vanilla `DecoratedPotRenderer.createModelTransformation`:
/// `Axis.YP.rotationDegrees(180 - facing.toYRot())` about the block centre
/// (SOUTH 180°, WEST 90°, NORTH 0°, EAST -90°).
fn decorated_pot_facing_y_rot(facing: WorldDecoratedPotFacing) -> f32 {
    180.0 - facing.to_y_rot()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        BlockEntityData, BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos,
    };
    use bbb_renderer::EntityModelKind;
    use bbb_world::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension,
    };
    use std::collections::BTreeMap;

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
                    bbb_world::vanilla_air_block_state_id(),
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

    fn set_decorated_pot(world: &mut WorldStore, pos: BlockPos, facing: &str) {
        let properties: BTreeMap<String, String> = [
            ("cracked".to_string(), "false".to_string()),
            ("facing".to_string(), facing.to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ]
        .into_iter()
        .collect();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties("minecraft:decorated_pot", &properties)
            .unwrap_or_else(|| {
                panic!("no registered state for minecraft:decorated_pot {properties:?}")
            });
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

    fn pot_nbt(sherds: &[&str]) -> Vec<u8> {
        let mut payload = vec![10];
        payload.push(9);
        write_mutf8(&mut payload, "sherds");
        payload.push(8);
        payload.extend_from_slice(&(sherds.len() as i32).to_be_bytes());
        for sherd in sherds {
            write_mutf8(&mut payload, sherd);
        }
        payload.push(0);
        payload
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }

    fn apply_pot_sherds(world: &mut WorldStore, pos: BlockPos, nbt: Vec<u8>) {
        assert!(world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
                raw_nbt: nbt,
            })
            .unwrap());
    }

    fn wobble_pot(world: &mut WorldStore, pos: BlockPos, style: u8) {
        world.apply_block_event(ProtocolBlockEvent {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            b0: 1,
            b1: style,
            block_id: 0,
        });
    }

    #[test]
    fn projects_pot_instances_with_patterns_facing_wobble_and_light() {
        let mut world = world_with_air_chunk();
        let decorated_pos = BlockPos { x: 3, y: 4, z: 5 };
        let plain_pos = BlockPos { x: 6, y: 4, z: 5 };
        set_decorated_pot(&mut world, decorated_pos, "north");
        set_decorated_pot(&mut world, plain_pos, "east");
        apply_pot_sherds(
            &mut world,
            decorated_pos,
            pot_nbt(&[
                "minecraft:angler_pottery_sherd",
                "minecraft:brick",
                "minecraft:unknown_item",
                "minecraft:skull_pottery_sherd",
            ]),
        );
        // WobbleStyle.POSITIVE (ordinal 0, 7 ticks).
        wobble_pot(&mut world, decorated_pos, 0);
        world.advance_decorated_pot_wobble_ticks(3);

        let instances = decorated_pot_model_instances_from_world_at_partial_tick(&world, 0.5);
        assert_eq!(instances.len(), 2);
        let decorated = &instances[0];
        let plain = &instances[1];
        assert_eq!(
            decorated.kind,
            EntityModelKind::DecoratedPot {
                back: Some(DecoratedPotPattern::Angler),
                left: None,
                // An item with no registered pattern falls back to the plain
                // side, like vanilla's null-pattern branch.
                right: None,
                front: Some(DecoratedPotPattern::Skull),
            }
        );
        assert_eq!(decorated.entity_id, DECORATED_POT_BLOCK_MODEL_ENTITY_ID);
        assert_eq!(decorated.position, [3.0, 4.0, 5.0]);
        // NORTH: 180 - toYRot(180) = 0.
        assert_eq!(decorated.render_state.body_rot, 0.0);
        let wobble = decorated.render_state.decorated_pot_wobble.unwrap();
        assert!(wobble.positive);
        // (3 + 0.5) / POSITIVE duration 7.
        assert!((wobble.progress - 0.5).abs() < 1e-6);
        // Empty light data: sky falls back to 15, block to 0 -> pack(0, 15).
        assert_eq!(decorated.render_state.light_coords, 15 << 20);
        assert_eq!(
            plain.kind,
            EntityModelKind::DecoratedPot {
                back: None,
                left: None,
                right: None,
                front: None,
            }
        );
        // EAST: 180 - toYRot(270) = -90.
        assert_eq!(plain.render_state.body_rot, -90.0);
        assert_eq!(plain.render_state.decorated_pot_wobble, None);
    }

    #[test]
    fn facing_angles_match_vanilla_model_transformation() {
        // 180 - Direction.toYRot(): SOUTH 180, WEST 90, NORTH 0, EAST -90.
        assert_eq!(
            decorated_pot_facing_y_rot(WorldDecoratedPotFacing::South),
            180.0
        );
        assert_eq!(
            decorated_pot_facing_y_rot(WorldDecoratedPotFacing::West),
            90.0
        );
        assert_eq!(
            decorated_pot_facing_y_rot(WorldDecoratedPotFacing::North),
            0.0
        );
        assert_eq!(
            decorated_pot_facing_y_rot(WorldDecoratedPotFacing::East),
            -90.0
        );
    }

    #[test]
    fn sherd_item_pattern_lookup_uses_generated_vanilla_catalog() {
        assert_eq!(
            DecoratedPotPattern::for_sherd_item_resource_id("minecraft:angler_pottery_sherd"),
            Some(DecoratedPotPattern::Angler)
        );
        assert_eq!(
            DecoratedPotPattern::for_sherd_item_resource_id("minecraft:snort_pottery_sherd"),
            Some(DecoratedPotPattern::Snort)
        );
        assert_eq!(
            DecoratedPotPattern::for_sherd_item_resource_id("minecraft:brick"),
            None
        );
        assert_eq!(
            DecoratedPotPattern::for_sherd_item_resource_id("minecraft:diamond"),
            None
        );
    }
}
