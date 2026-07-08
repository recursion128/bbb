//! World -> renderer projection for conduit block-entity models.
//!
//! Vanilla `ConduitRenderer.submit` emits different model parts with different
//! root transforms: inactive shell, or active cage + two wind shells + eye.
//! bbb keeps one renderer submission stream by expanding each conduit render
//! source into those part-specific `EntityModelInstance`s.

use bbb_renderer::{CameraPose, ConduitModelPart, EntityModelInstance};
use bbb_world::{ConduitModelSourceState, TerrainLight, WorldStore};

/// Block-entity model sentinel id; no vanilla network entity id is negative.
const CONDUIT_BLOCK_MODEL_ENTITY_ID: i32 = -1;

pub(crate) fn conduit_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    partial_tick: f32,
) -> Vec<EntityModelInstance> {
    world
        .conduit_model_source_states(partial_tick)
        .into_iter()
        .flat_map(|source| conduit_model_instances(&source, world, camera_pose))
        .collect()
}

fn conduit_model_instances(
    source: &ConduitModelSourceState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Vec<EntityModelInstance> {
    let light = world
        .sample_block_light(source.pos)
        .map(conduit_light_coords);
    if !source.is_active {
        return vec![conduit_part_instance(
            source,
            ConduitModelPart::Shell,
            light,
        )];
    }

    let mut instances = vec![
        conduit_part_instance(source, ConduitModelPart::Cage, light),
        conduit_part_instance(
            source,
            ConduitModelPart::OuterWind {
                phase: source.animation_phase,
            },
            light,
        ),
        conduit_part_instance(
            source,
            ConduitModelPart::InnerWind {
                vertical: source.animation_phase == 1,
            },
            light,
        ),
        conduit_part_instance(
            source,
            ConduitModelPart::Eye {
                open: source.is_hunting,
            },
            light,
        ),
    ];
    if let Some(camera_pose) = camera_pose {
        let eye = instances.last_mut().expect("eye instance is present");
        eye.render_state.body_rot = camera_pose.y_rot;
        eye.render_state.head_pitch = camera_pose.x_rot;
    }
    instances
}

fn conduit_part_instance(
    source: &ConduitModelSourceState,
    part: ConduitModelPart,
    light: Option<u32>,
) -> EntityModelInstance {
    let mut instance = EntityModelInstance::conduit(
        CONDUIT_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        part,
    )
    .with_conduit_anim_time(source.anim_time)
    .with_conduit_active_rotation(source.active_rotation_radians);
    if let Some(light) = light {
        instance = instance.with_light_coords(light);
    }
    instance
}

/// Vanilla `LightCoordsUtil.pack(block, sky)`.
fn conduit_light_coords(light: TerrainLight) -> u32 {
    u32::from(light.block.min(15)) << 4 | u32::from(light.sky.min(15)) << 20
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate, PlayTime};
    use bbb_renderer::EntityModelKind;
    use bbb_world::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
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
            light: bbb_world::LightData::default(),
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
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    fn offset_pos(pos: BlockPos, x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos {
            x: pos.x + x,
            y: pos.y + y,
            z: pos.z + z,
        }
    }

    fn activate_conduit(world: &mut WorldStore, pos: BlockPos) {
        for ox in -1..=1 {
            for oy in -1..=1 {
                for oz in -1..=1 {
                    if ox != 0 || oy != 0 || oz != 0 {
                        set_block(
                            world,
                            offset_pos(pos, ox, oy, oz),
                            "minecraft:water",
                            &[("level", "0")],
                        );
                    }
                }
            }
        }
        for ox in -2_i32..=2 {
            for oy in -2_i32..=2 {
                for oz in -2_i32..=2 {
                    let ax = ox.abs();
                    let ay = oy.abs();
                    let az = oz.abs();
                    let slot = (ax > 1 || ay > 1 || az > 1)
                        && ((ox == 0 && (ay == 2 || az == 2))
                            || (oy == 0 && (ax == 2 || az == 2))
                            || (oz == 0 && (ax == 2 || ay == 2)));
                    if slot {
                        set_block(
                            world,
                            offset_pos(pos, ox, oy, oz),
                            "minecraft:prismarine",
                            &[],
                        );
                    }
                }
            }
        }
        set_block(world, pos, "minecraft:conduit", &[("waterlogged", "true")]);
        world.apply_world_time(PlayTime {
            game_time: 40,
            clock_updates: Vec::new(),
        });
        world.advance_conduit_ticks(1);
    }

    #[test]
    fn inactive_conduit_projects_only_the_shell_part() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 4, y: 4, z: 4 };
        set_block(
            &mut world,
            pos,
            "minecraft:conduit",
            &[("waterlogged", "true")],
        );

        let instances = conduit_model_instances_from_world_at_partial_tick(&world, None, 0.5);

        assert_eq!(instances.len(), 1);
        assert_eq!(
            instances[0].kind,
            EntityModelKind::Conduit {
                part: ConduitModelPart::Shell,
            }
        );
        assert_eq!(instances[0].position, [4.0, 4.0, 4.0]);
        assert_eq!(instances[0].render_state.light_coords, 15 << 20);
    }

    #[test]
    fn active_conduit_projects_cage_winds_and_camera_facing_eye() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 5, y: 5, z: 5 };
        activate_conduit(&mut world, pos);
        let camera = CameraPose {
            position: [0.0, 2.0, 0.0],
            y_rot: 35.0,
            x_rot: -12.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };

        let instances =
            conduit_model_instances_from_world_at_partial_tick(&world, Some(camera), 0.5);

        assert_eq!(instances.len(), 4);
        assert_eq!(
            instances[0].kind,
            EntityModelKind::Conduit {
                part: ConduitModelPart::Cage,
            }
        );
        assert_eq!(
            instances[1].kind,
            EntityModelKind::Conduit {
                part: ConduitModelPart::OuterWind { phase: 0 },
            }
        );
        assert_eq!(
            instances[2].kind,
            EntityModelKind::Conduit {
                part: ConduitModelPart::InnerWind { vertical: false },
            }
        );
        assert_eq!(
            instances[3].kind,
            EntityModelKind::Conduit {
                part: ConduitModelPart::Eye { open: true },
            }
        );
        assert!((instances[0].render_state.conduit_anim_time - 1.5).abs() < 1.0e-6);
        assert!(
            (instances[0].render_state.conduit_active_rotation - (-1.5 * 0.0375)).abs() < 1.0e-6
        );
        assert_eq!(instances[3].render_state.body_rot, 35.0);
        assert_eq!(instances[3].render_state.head_pitch, -12.0);
    }
}
