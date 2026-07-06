//! World -> renderer projection for chest block-entity models.
//!
//! Vanilla renders chests through the `BlockEntityRenderDispatcher` +
//! `ChestRenderer` pair: per chest block entity, a `ChestRenderState` carrying
//! the block state's `FACING`/`TYPE`, the `Sheets.chooseSprite` material, the
//! combined lid openness, and the light coords sampled at the block position
//! (with the double-chest `BrightnessCombiner` max). bbb has no separate BER
//! dispatch; chest instances ride the existing single entity-model submission
//! stream (`RendererFrame.entity_model_instances`) as
//! `EntityModelKind::Chest`, so the renderer keeps one dispatch authority for
//! all textured model submissions.

use bbb_renderer::{ChestModelHalf, ChestModelTexture, EntityModelInstance};
use bbb_world::{
    ChestModelFacing as WorldChestModelFacing, ChestModelHalf as WorldChestModelHalf,
    ChestModelKind as WorldChestModelKind, ChestModelSourceState, TerrainLight, WorldStore,
};

/// Chest instances are projected from block states, not the entity list, so
/// they carry a sentinel id no server entity can use (vanilla network entity
/// ids are non-negative). The id only feeds per-entity animation seeds
/// (`instance.entity_id`), which the chest model does not read.
const CHEST_BLOCK_MODEL_ENTITY_ID: i32 = -1;

/// Projects every chest-family block in the loaded chunks into a chest model
/// instance: position at the block min corner, `-facing.toYRot()` yaw
/// (`ChestRenderer.createModelTransformation`), the raw combined openness for
/// the model's lid easing, and the block-position light with the vanilla
/// double-chest `BrightnessCombiner` per-component max.
pub(crate) fn chest_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    partial_tick: f32,
) -> Vec<EntityModelInstance> {
    world
        .chest_model_source_states(partial_tick)
        .into_iter()
        .map(|source| chest_model_instance(&source, world))
        .collect()
}

fn chest_model_instance(source: &ChestModelSourceState, world: &WorldStore) -> EntityModelInstance {
    let mut instance = EntityModelInstance::chest(
        CHEST_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        chest_facing_y_rot(source.facing),
        chest_model_texture(source.kind),
        chest_model_half(source.half),
    )
    .with_chest_openness(source.openness);
    let mut light = world.sample_block_light(source.pos);
    if let Some(partner_pos) = source.partner_pos {
        // Vanilla `BrightnessCombiner`: a joined double chest renders both
        // halves with `LightCoordsUtil.max(pack(first), pack(second))`, i.e.
        // the per-component max of the two block-position samples.
        if let (Some(own), Some(partner)) = (light, world.sample_block_light(partner_pos)) {
            light = Some(TerrainLight {
                sky: own.sky.max(partner.sky),
                block: own.block.max(partner.block),
            });
        }
    }
    if let Some(light) = light {
        instance = instance.with_light_coords(chest_light_coords(light));
    }
    instance
}

/// Vanilla `Direction.toYRot()` (SOUTH 0°, WEST 90°, NORTH 180°, EAST 270°),
/// negated per `ChestRenderer.createModelTransformation`'s
/// `Axis.YP.rotationDegrees(-facing.toYRot())`.
fn chest_facing_y_rot(facing: WorldChestModelFacing) -> f32 {
    match facing {
        WorldChestModelFacing::South => 0.0,
        WorldChestModelFacing::West => -90.0,
        WorldChestModelFacing::North => -180.0,
        WorldChestModelFacing::East => -270.0,
    }
}

fn chest_model_texture(kind: WorldChestModelKind) -> ChestModelTexture {
    match kind {
        WorldChestModelKind::Normal => ChestModelTexture::Normal,
        WorldChestModelKind::Trapped => ChestModelTexture::Trapped,
        WorldChestModelKind::Ender => ChestModelTexture::Ender,
        WorldChestModelKind::Copper => ChestModelTexture::Copper,
        WorldChestModelKind::CopperExposed => ChestModelTexture::CopperExposed,
        WorldChestModelKind::CopperWeathered => ChestModelTexture::CopperWeathered,
        WorldChestModelKind::CopperOxidized => ChestModelTexture::CopperOxidized,
    }
}

fn chest_model_half(half: WorldChestModelHalf) -> ChestModelHalf {
    match half {
        WorldChestModelHalf::Single => ChestModelHalf::Single,
        WorldChestModelHalf::Left => ChestModelHalf::Left,
        WorldChestModelHalf::Right => ChestModelHalf::Right,
    }
}

/// Vanilla `LightCoordsUtil.pack(block, sky)` (`block << 4 | sky << 20`) over
/// the raw stored light sample — the chest render state's `lightCoords` with
/// no renderer-specific block-light override.
fn chest_light_coords(light: TerrainLight) -> u32 {
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

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str, properties: &[(&str, &str)]) {
        let properties: BTreeMap<String, String> = properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name} {properties:?}"));
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

    fn send_chest_open_count(world: &mut WorldStore, pos: BlockPos, count: u8) {
        world.apply_block_event(ProtocolBlockEvent {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            b0: 1,
            b1: count,
            block_id: 0,
        });
    }

    #[test]
    fn projects_single_chest_instance_with_facing_openness_and_light() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_block(
            &mut world,
            pos,
            "minecraft:chest",
            &[
                ("facing", "east"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        );
        send_chest_open_count(&mut world, pos, 1);
        world.advance_chest_lid_ticks(2);

        let instances = chest_model_instances_from_world_at_partial_tick(&world, 0.5);
        assert_eq!(instances.len(), 1);
        let instance = &instances[0];
        assert_eq!(
            instance.kind,
            EntityModelKind::Chest {
                texture: ChestModelTexture::Normal,
                half: ChestModelHalf::Single,
            }
        );
        assert_eq!(instance.entity_id, CHEST_BLOCK_MODEL_ENTITY_ID);
        assert_eq!(instance.position, [3.0, 4.0, 5.0]);
        // EAST: toYRot = 270°, negated.
        assert_eq!(instance.render_state.body_rot, -270.0);
        // Two 0.1 lid ticks: openness 0.2, o_openness 0.1, lerped at 0.5 -> 0.15.
        let expected = 0.1 + (0.2 - 0.1) * 0.5;
        assert!((instance.render_state.chest_openness - expected).abs() < 1e-6);
        // Empty light data: sky falls back to 15, block to 0 -> pack(0, 15).
        assert_eq!(instance.render_state.light_coords, 15 << 20);
    }

    #[test]
    fn projects_double_chest_halves_with_shared_max_openness() {
        let mut world = world_with_air_chunk();
        // A double chest facing north: the LEFT half's partner sits toward
        // facing.getClockWise() = east (+x).
        let left_pos = BlockPos { x: 6, y: 4, z: 5 };
        let right_pos = BlockPos { x: 7, y: 4, z: 5 };
        set_block(
            &mut world,
            left_pos,
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "left"),
                ("waterlogged", "false"),
            ],
        );
        set_block(
            &mut world,
            right_pos,
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "right"),
                ("waterlogged", "false"),
            ],
        );
        // Only the right half's lid controller receives the open count; the
        // opennessCombiner max still animates both halves.
        send_chest_open_count(&mut world, right_pos, 1);
        world.advance_chest_lid_ticks(1);

        let instances = chest_model_instances_from_world_at_partial_tick(&world, 1.0);
        assert_eq!(instances.len(), 2);
        let left = instances
            .iter()
            .find(|instance| instance.position == [6.0, 4.0, 5.0])
            .unwrap();
        let right = instances
            .iter()
            .find(|instance| instance.position == [7.0, 4.0, 5.0])
            .unwrap();
        assert_eq!(
            left.kind,
            EntityModelKind::Chest {
                texture: ChestModelTexture::Normal,
                half: ChestModelHalf::Left,
            }
        );
        assert_eq!(
            right.kind,
            EntityModelKind::Chest {
                texture: ChestModelTexture::Normal,
                half: ChestModelHalf::Right,
            }
        );
        // NORTH: toYRot = 180°, negated.
        assert_eq!(left.render_state.body_rot, -180.0);
        assert!((left.render_state.chest_openness - 0.1).abs() < 1e-6);
        assert!((right.render_state.chest_openness - 0.1).abs() < 1e-6);
    }

    #[test]
    fn projects_trapped_and_ender_chest_textures() {
        let mut world = world_with_air_chunk();
        set_block(
            &mut world,
            BlockPos { x: 1, y: 2, z: 3 },
            "minecraft:trapped_chest",
            &[
                ("facing", "south"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        );
        set_block(
            &mut world,
            BlockPos { x: 2, y: 2, z: 3 },
            "minecraft:ender_chest",
            &[("facing", "west"), ("waterlogged", "false")],
        );

        let instances = chest_model_instances_from_world_at_partial_tick(&world, 0.0);
        assert_eq!(instances.len(), 2);
        assert_eq!(
            instances[0].kind,
            EntityModelKind::Chest {
                texture: ChestModelTexture::Trapped,
                half: ChestModelHalf::Single,
            }
        );
        assert_eq!(instances[0].render_state.body_rot, 0.0);
        assert_eq!(
            instances[1].kind,
            EntityModelKind::Chest {
                texture: ChestModelTexture::Ender,
                half: ChestModelHalf::Single,
            }
        );
        assert_eq!(instances[1].render_state.body_rot, -90.0);
    }

    #[test]
    fn packs_chest_light_coords_like_vanilla() {
        assert_eq!(
            chest_light_coords(TerrainLight { sky: 15, block: 0 }),
            15 << 20
        );
        assert_eq!(
            chest_light_coords(TerrainLight { sky: 7, block: 9 }),
            9 << 4 | 7 << 20
        );
        assert_eq!(
            chest_light_coords(TerrainLight {
                sky: 200,
                block: 200
            }),
            15 << 4 | 15 << 20
        );
    }
}
