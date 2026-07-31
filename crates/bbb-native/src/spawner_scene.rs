//! World -> renderer projection for ordinary and Trial Spawner display entities.
//!
//! Vanilla `SpawnerRenderer` reuses the entity renderer for the cached
//! display entity, then wraps that submission in a spawner-local transform.
//! Trial Spawners additionally carry a tiny world containing the bounded
//! persistent entity projection, so the same entity/equipment/item paths can
//! consume custom `spawn_data.entity` state.

use bbb_renderer::{EntityModelInstance, EntityModelKind, SpawnerDisplayRenderState};
use bbb_world::{SpawnerDisplayEntitySourceState, TerrainLight, WorldStore};

use crate::{
    entity_scene::{
        entity_model_instance_from_world_entity_at_partial_tick,
        entity_model_instance_from_world_entity_with_variant_textures_at_partial_tick,
        entity_model_kind_for_world_entity_type_at_partial_tick,
    },
    entity_variant_textures::NativeEntityVariantTextureRuntime,
};
use bbb_item_model::NativeItemRuntime;

/// Block-entity model sentinel id; no vanilla network entity id is negative.
const SPAWNER_BLOCK_MODEL_ENTITY_ID: i32 = -1;

pub(crate) struct SpawnerDisplayEntitySubmission {
    pub instance: EntityModelInstance,
    /// Present only for Trial Spawners whose persistent equipment must be
    /// resolved by the shared held-item extraction path.
    pub entity_world: Option<WorldStore>,
}

#[cfg(test)]
pub(crate) fn spawner_display_entity_instances_from_world_at_partial_tick(
    world: &WorldStore,
    partial_tick: f32,
) -> Vec<EntityModelInstance> {
    spawner_display_entity_submissions_from_world_at_partial_tick(world, None, None, partial_tick)
        .into_iter()
        .map(|submission| submission.instance)
        .collect()
}

pub(crate) fn spawner_display_entity_submissions_from_world_at_partial_tick(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    partial_tick: f32,
) -> Vec<SpawnerDisplayEntitySubmission> {
    let mut submissions: Vec<_> = world
        .spawner_display_entity_source_states(partial_tick)
        .into_iter()
        .filter_map(|source| spawner_display_entity_instance(source, world, partial_tick))
        .map(|instance| SpawnerDisplayEntitySubmission {
            instance,
            entity_world: None,
        })
        .collect();
    submissions.extend(
        world
            .trial_spawner_display_entity_source_states(partial_tick)
            .into_iter()
            .filter_map(|source| {
                let mut instance = match variant_textures {
                    Some(textures) => {
                        entity_model_instance_from_world_entity_with_variant_textures_at_partial_tick(
                            &source.entity_world,
                            item_runtime,
                            textures,
                            source.entity_id,
                            partial_tick,
                        )
                    }
                    None => entity_model_instance_from_world_entity_at_partial_tick(
                        &source.entity_world,
                        item_runtime,
                        source.entity_id,
                        partial_tick,
                    ),
                }?;
                if matches!(instance.kind, EntityModelKind::NoRender) {
                    return None;
                }
                instance.position = [
                    source.pos.x as f32,
                    source.pos.y as f32,
                    source.pos.z as f32,
                ];
                instance = instance.with_spawner_display(Some(SpawnerDisplayRenderState {
                    spin_degrees: source.spin_degrees,
                    scale: source.scale,
                }));
                if let Some(light) = world.sample_block_light(source.pos) {
                    instance = instance.with_light_coords(spawner_light_coords(light));
                }
                Some(SpawnerDisplayEntitySubmission {
                    instance,
                    entity_world: Some(source.entity_world),
                })
            }),
    );
    submissions
}

fn spawner_display_entity_instance(
    source: SpawnerDisplayEntitySourceState,
    world: &WorldStore,
    partial_tick: f32,
) -> Option<EntityModelInstance> {
    let kind = entity_model_kind_for_world_entity_type_at_partial_tick(
        world,
        source.entity_type_id,
        partial_tick,
    );
    if matches!(kind, EntityModelKind::NoRender) {
        return None;
    }
    let mut instance = EntityModelInstance::new(
        SPAWNER_BLOCK_MODEL_ENTITY_ID,
        kind,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        0.0,
    )
    .with_age_in_ticks(partial_tick)
    .with_spawner_display(Some(SpawnerDisplayRenderState {
        spin_degrees: source.spin_degrees,
        scale: source.scale,
    }));
    if let Some(light) = world.sample_block_light(source.pos) {
        instance = instance.with_light_coords(spawner_light_coords(light));
    }
    Some(instance)
}

/// Vanilla `LightCoordsUtil.pack(block, sky)`.
fn spawner_light_coords(light: TerrainLight) -> u32 {
    u32::from(light.block.min(15)) << 4 | u32::from(light.sky.min(15)) << 20
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
    use bbb_protocol::packets::{
        BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate, Vec3d as ProtocolVec3d,
    };
    use bbb_world::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, LocalPlayerPoseState,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
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

    fn set_spawner(world: &mut WorldStore, pos: BlockPos) {
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties("minecraft:spawner", &BTreeMap::new())
            .expect("spawner block state is registered");
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    fn apply_spawner_entity_data(world: &mut WorldStore, pos: BlockPos) {
        world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
                raw_nbt: spawner_nbt("minecraft:zombie"),
            })
            .expect("spawner block entity data decodes");
    }

    fn spawner_nbt(entity_id: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(10);
        write_tag_name(&mut out, 2, "Delay");
        out.extend_from_slice(&20_i16.to_be_bytes());
        write_tag_name(&mut out, 10, "SpawnData");
        write_tag_name(&mut out, 10, "entity");
        write_tag_name(&mut out, 8, "id");
        write_nbt_string_payload(&mut out, entity_id);
        out.push(0);
        out.push(0);
        out.push(0);
        out
    }

    fn write_tag_name(out: &mut Vec<u8>, tag: u8, name: &str) {
        out.push(tag);
        write_nbt_string_payload(out, name);
    }

    fn write_nbt_string_payload(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        let len = u16::try_from(bytes.len()).expect("test nbt string fits u16");
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(bytes);
    }

    #[test]
    fn projects_spawner_display_entity_as_wrapped_entity_model_instance() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_spawner(&mut world, pos);
        apply_spawner_entity_data(&mut world, pos);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 2.5,
                y: 3.5,
                z: 5.5,
            },
            ..LocalPlayerPoseState::default()
        });
        world.advance_spawner_block_ticks(1);

        let instances = spawner_display_entity_instances_from_world_at_partial_tick(&world, 0.5);

        assert_eq!(instances.len(), 1);
        let instance = &instances[0];
        assert_eq!(instance.entity_id, SPAWNER_BLOCK_MODEL_ENTITY_ID);
        assert_eq!(instance.position, [2.0, 3.0, 4.0]);
        assert_eq!(instance.kind, EntityModelKind::Zombie { baby: false });
        let source = &world.spawner_display_entity_source_states(0.5)[0];
        assert_eq!(source.entity_type_id, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        assert_eq!(
            instance.render_state.spawner_display,
            Some(SpawnerDisplayRenderState {
                spin_degrees: source.spin_degrees,
                scale: source.scale,
            })
        );
    }

    #[test]
    fn projects_trial_spawner_custom_entity_through_shared_entity_world() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_trial_spawner(&mut world, pos);
        world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID,
                raw_nbt: trial_spawner_nbt(),
            })
            .expect("trial spawner block entity data decodes");
        world.advance_trial_spawner_block_ticks(1);

        let submissions =
            spawner_display_entity_submissions_from_world_at_partial_tick(&world, None, None, 1.0);
        let submission = submissions
            .iter()
            .find(|submission| submission.entity_world.is_some())
            .expect("trial spawner submission");
        assert_eq!(
            submission.instance.kind,
            EntityModelKind::Zombie { baby: true }
        );
        assert_eq!(submission.instance.position, [2.0, 3.0, 4.0]);
        assert_eq!(
            submission.instance.render_state.spawner_display,
            Some(SpawnerDisplayRenderState {
                spin_degrees: 50.0,
                scale: 0.53125,
            })
        );
        let entity_world = submission.entity_world.as_ref().unwrap();
        assert!(entity_world
            .equipment_item(
                submission.instance.entity_id,
                bbb_protocol::packets::EquipmentSlot::MainHand,
            )
            .is_some());
    }

    fn set_trial_spawner(world: &mut WorldStore, pos: BlockPos) {
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties(
                "minecraft:trial_spawner",
                &BTreeMap::from([
                    ("ominous".to_string(), "false".to_string()),
                    ("trial_spawner_state".to_string(), "active".to_string()),
                ]),
            )
            .expect("active trial spawner block state is registered");
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    fn trial_spawner_nbt() -> Vec<u8> {
        let mut out = vec![10];
        write_tag_name(&mut out, 10, "spawn_data");
        write_tag_name(&mut out, 10, "entity");
        write_tag_name(&mut out, 8, "id");
        write_nbt_string_payload(&mut out, "minecraft:zombie");
        write_tag_name(&mut out, 1, "IsBaby");
        out.push(1);
        write_tag_name(&mut out, 10, "equipment");
        write_tag_name(&mut out, 10, "mainhand");
        write_tag_name(&mut out, 8, "id");
        write_nbt_string_payload(&mut out, "minecraft:stone");
        write_tag_name(&mut out, 3, "count");
        out.extend_from_slice(&1_i32.to_be_bytes());
        out.push(0);
        out.push(0);
        out.push(0);
        out.push(0);
        write_tag_name(&mut out, 4, "next_mob_spawns_at");
        out.extend_from_slice(&0_i64.to_be_bytes());
        out.push(0);
        out
    }
}
