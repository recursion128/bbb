//! Client-side Trial Spawner display-entity state.
//!
//! Vanilla retains one lazily loaded display entity in
//! `TrialSpawnerStateData`, advances only its two spin scalars, and sends the
//! entity through the ordinary entity renderer. This module keeps a bounded
//! persistent projection of the NBT fields the native renderer can consume;
//! it deliberately does not become a general entity-NBT loader.

use bbb_protocol::{
    entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
    },
    entity_types::{vanilla_entity_type_for_resource_id, vanilla_entity_type_is_serializable},
    packets::{
        AddEntity as ProtocolAddEntity, EntityDataValue, EntityDataValueKind, EquipmentSlotUpdate,
        SetEntityData, SetEquipment, Vec3d,
    },
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    chunks::{
        TrialSpawnerBlockEntityData, TrialSpawnerEntityNbtProjection,
        TrialSpawnerEquipmentNbtProjection,
    },
    entities::{
        vanilla_entity_metadata_baby_accessor_for_entity_type,
        vanilla_entity_metadata_entity_accessors,
        vanilla_entity_metadata_living_entity_accessors_for_entity_type,
        vanilla_entity_metadata_mob_accessors_for_entity_type,
    },
    BlockPos, TrialSpawnerClientState, WorldStore,
};

const TRIAL_SPAWNER_DISPLAY_ENTITY_ID: i32 = i32::MIN;
const VANILLA_SPAWNER_DISPLAY_BASE_SCALE: f32 = 0.53125;
const VANILLA_SPAWNER_SPIN_SCALE: f32 = 10.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrialSpawnerBlockRenderState {
    pub pos: BlockPos,
    pub state: TrialSpawnerClientState,
    pub next_mob_spawns_at: i64,
    pub spin: f64,
    pub o_spin: f64,
    display_entity: Option<TrialSpawnerDisplayEntityProjection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TrialSpawnerDisplayEntityProjection {
    entity_type_id: i32,
    rotation: [f32; 2],
    glowing: bool,
    ticks_frozen: i32,
    fall_flying: bool,
    baby: bool,
    left_handed: bool,
    equipment: Vec<EquipmentSlotUpdate>,
}

/// Per-frame shared-entity-renderer input for one Trial Spawner. The tiny
/// world owns exactly one synthetic entity while sharing the current
/// registry/item publications with the canonical world.
#[derive(Debug, Clone)]
pub struct TrialSpawnerDisplayEntitySourceState {
    pub pos: BlockPos,
    pub entity_id: i32,
    pub spin_degrees: f32,
    pub scale: f32,
    pub entity_world: WorldStore,
}

#[derive(Debug, Clone)]
struct TrialSpawnerBlockSource {
    pos: BlockPos,
    state: TrialSpawnerClientState,
    data: TrialSpawnerBlockEntityData,
}

impl WorldStore {
    /// Advances `TrialSpawner.tickClient` and lazily materializes the bounded
    /// display projection. Once created, the projection remains cached across
    /// later update tags exactly like Vanilla's `displayEntity` field.
    pub fn advance_trial_spawner_block_ticks(&mut self, ticks: u32) {
        let sources = self.trial_spawner_block_sources();
        let mut states = std::mem::take(&mut self.trial_spawner_blocks);
        states.retain(|state| sources.iter().any(|source| source.pos == state.pos));

        for source in &sources {
            let index = states
                .iter()
                .position(|state| state.pos == source.pos)
                .unwrap_or_else(|| {
                    states.push(TrialSpawnerBlockRenderState {
                        pos: source.pos,
                        state: source.state,
                        next_mob_spawns_at: source.data.next_mob_spawns_at,
                        spin: 0.0,
                        o_spin: 0.0,
                        display_entity: None,
                    });
                    states.len() - 1
                });
            let state = &mut states[index];
            state.state = source.state;
            state.next_mob_spawns_at = source.data.next_mob_spawns_at;
            if state.display_entity.is_none() && source.state.has_spinning_mob() {
                state.display_entity = source
                    .data
                    .spawn_entity
                    .as_ref()
                    .and_then(|entity| self.trial_spawner_display_projection(entity));
            }
        }

        let game_time = self.world_time.as_ref().map_or(0, |time| time.game_time);
        for state in &mut states {
            let Some(speed) = state.state.spinning_mob_speed() else {
                continue;
            };
            for elapsed in 0..ticks {
                let tick_time =
                    game_time - i64::from(ticks.saturating_sub(elapsed).saturating_sub(1));
                let spawn_delay = (state.next_mob_spawns_at - tick_time).max(0) as f64;
                state.o_spin = state.spin;
                state.spin = (state.spin + speed / (spawn_delay + 200.0)).rem_euclid(360.0);
            }
        }

        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        self.trial_spawner_blocks = states;
    }

    pub fn trial_spawner_block_render_states(&self) -> &[TrialSpawnerBlockRenderState] {
        &self.trial_spawner_blocks
    }

    pub(crate) fn prune_trial_spawner_blocks_in_chunk(&mut self, chunk: crate::ChunkPos) {
        self.trial_spawner_blocks.retain(|state| {
            state.pos.x.div_euclid(16) != chunk.x || state.pos.z.div_euclid(16) != chunk.z
        });
    }

    pub fn trial_spawner_display_entity_source_states(
        &self,
        partial_tick: f32,
    ) -> Vec<TrialSpawnerDisplayEntitySourceState> {
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        self.trial_spawner_blocks
            .iter()
            .filter(|state| state.state.has_spinning_mob())
            .filter_map(|state| {
                let projection = state.display_entity.as_ref()?;
                let entity_world = self.trial_spawner_entity_world(state.pos, projection)?;
                let bounds =
                    entity_world.probe_entity_pick_bounds(TRIAL_SPAWNER_DISPLAY_ENTITY_ID)?;
                let width = (bounds.max[0] - bounds.min[0]) as f32;
                let height = (bounds.max[1] - bounds.min[1]) as f32;
                let max_length = width.max(height);
                let scale = if max_length > 1.0 {
                    VANILLA_SPAWNER_DISPLAY_BASE_SCALE / max_length
                } else {
                    VANILLA_SPAWNER_DISPLAY_BASE_SCALE
                };
                Some(TrialSpawnerDisplayEntitySourceState {
                    pos: state.pos,
                    entity_id: TRIAL_SPAWNER_DISPLAY_ENTITY_ID,
                    spin_degrees: lerp_f64(state.o_spin, state.spin, partial_tick) as f32
                        * VANILLA_SPAWNER_SPIN_SCALE,
                    scale,
                    entity_world,
                })
            })
            .collect()
    }

    fn trial_spawner_display_projection(
        &self,
        value: &TrialSpawnerEntityNbtProjection,
    ) -> Option<TrialSpawnerDisplayEntityProjection> {
        let entity_id = value.entity_id.as_deref()?;
        let entity_type = vanilla_entity_type_for_resource_id(entity_id)?;
        if !vanilla_entity_type_is_serializable(entity_type.id) {
            return None;
        }
        let rotation = value.rotation_bits.map(f32::from_bits);
        if !rotation.iter().all(|value| value.is_finite()) {
            return None;
        }
        let is_living =
            vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type.id)
                .is_some();
        Some(TrialSpawnerDisplayEntityProjection {
            entity_type_id: entity_type.id,
            rotation,
            glowing: value.glowing,
            ticks_frozen: value.ticks_frozen,
            fall_flying: is_living && value.fall_flying,
            baby: value.baby,
            left_handed: value.left_handed,
            equipment: if is_living {
                self.trial_spawner_equipment(&value.equipment)
            } else {
                Vec::new()
            },
        })
    }

    fn trial_spawner_equipment(
        &self,
        entries: &[TrialSpawnerEquipmentNbtProjection],
    ) -> Vec<EquipmentSlotUpdate> {
        let mut equipment = Vec::with_capacity(entries.len().min(8));
        for entry in entries {
            let Ok(item) = self.persistent_item_stack_from_nbt(&entry.item) else {
                return Vec::new();
            };
            if item.count > 0 && item.item_id.is_some() {
                equipment.push(EquipmentSlotUpdate {
                    slot: entry.slot,
                    item,
                });
            }
        }
        equipment.sort_by_key(|entry| entry.slot.ordinal());
        equipment
    }

    fn trial_spawner_entity_world(
        &self,
        pos: BlockPos,
        projection: &TrialSpawnerDisplayEntityProjection,
    ) -> Option<WorldStore> {
        let mut world = WorldStore::with_dimension(self.dimension);
        world.level = self.level.clone();
        world.gameplay = self.gameplay.clone();
        world.world_time = self.world_time.clone();
        world.weather = self.weather;
        world.registries = self.registries.clone();
        world.features = self.features.clone();
        world.selected_known_pack_data = self.selected_known_pack_data.clone();
        world.items = self.items.clone();
        world.apply_add_entity(ProtocolAddEntity {
            id: TRIAL_SPAWNER_DISPLAY_ENTITY_ID,
            uuid: Uuid::nil(),
            entity_type_id: projection.entity_type_id,
            position: Vec3d {
                x: f64::from(pos.x),
                y: f64::from(pos.y),
                z: f64::from(pos.z),
            },
            delta_movement: Vec3d::default(),
            x_rot: projection.rotation[1],
            y_rot: projection.rotation[0],
            y_head_rot: projection.rotation[0],
            data: 0,
        });

        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let mut shared_flags = entity_accessors.shared_flags_default;
        if projection.glowing {
            shared_flags |= entity_accessors.glowing_mask;
        }
        if projection.fall_flying {
            shared_flags |= entity_accessors.fall_flying_mask;
        }
        let mut values = vec![
            EntityDataValue {
                data_id: entity_accessors.shared_flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(shared_flags),
            },
            EntityDataValue {
                data_id: entity_accessors.ticks_frozen_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(projection.ticks_frozen),
            },
        ];
        if let Some((data_id, _)) =
            vanilla_entity_metadata_baby_accessor_for_entity_type(projection.entity_type_id)
        {
            values.push(EntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(projection.baby),
            });
        }
        if let Some(accessors) =
            vanilla_entity_metadata_mob_accessors_for_entity_type(projection.entity_type_id)
        {
            values.push(EntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(
                    accessors.flags_default
                        | if projection.left_handed {
                            accessors.left_handed_mask
                        } else {
                            0
                        },
                ),
            });
        }
        if !world.apply_set_entity_data(SetEntityData {
            id: TRIAL_SPAWNER_DISPLAY_ENTITY_ID,
            values,
        }) {
            return None;
        }
        if !projection.equipment.is_empty()
            && !world.apply_set_equipment(SetEquipment {
                entity_id: TRIAL_SPAWNER_DISPLAY_ENTITY_ID,
                slots: projection.equipment.clone(),
            })
        {
            return None;
        }
        Some(world)
    }

    fn trial_spawner_block_sources(&self) -> Vec<TrialSpawnerBlockSource> {
        let mut sources = Vec::new();
        for chunk in &self.chunks {
            for record in &chunk.block_entities {
                let Some(data) = record.trial_spawner.as_ref() else {
                    continue;
                };
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + i32::from(record.local_x),
                    y: i32::from(record.y),
                    z: chunk.pos.z * 16 + i32::from(record.local_z),
                };
                let Some(block) = self.probe_block(pos) else {
                    continue;
                };
                if block.block_name.as_deref() != Some("minecraft:trial_spawner") {
                    continue;
                }
                let Some(state) = block
                    .block_properties
                    .get("trial_spawner_state")
                    .and_then(|state| TrialSpawnerClientState::from_block_property(state))
                else {
                    continue;
                };
                sources.push(TrialSpawnerBlockSource {
                    pos,
                    state,
                    data: data.clone(),
                });
            }
        }
        sources.sort_by_key(|source| (source.pos.y, source.pos.z, source.pos.x));
        sources
    }
}

fn lerp_f64(from: f64, to: f64, partial_tick: f32) -> f64 {
    from + (to - from) * f64::from(partial_tick)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::{
        block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID,
        entity_types::{VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID},
        packets::{BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate, EquipmentSlot},
    };

    use super::*;
    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };

    #[test]
    fn active_trial_spawner_projects_custom_baby_rotation_flags_and_equipment() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_trial_spawner(&mut world, pos, "active");
        apply_trial_spawner_data(
            &mut world,
            pos,
            trial_spawner_nbt("minecraft:zombie", true, true),
        );

        world.advance_trial_spawner_block_ticks(1);

        let sources = world.trial_spawner_display_entity_source_states(1.0);
        assert_eq!(sources.len(), 1);
        let source = &sources[0];
        assert_eq!(source.pos, pos);
        assert_eq!(source.spin_degrees, 50.0);
        assert_eq!(source.scale, VANILLA_SPAWNER_DISPLAY_BASE_SCALE);
        let model = source
            .entity_world
            .entity_model_source_at_partial_tick(source.entity_id, 1.0)
            .unwrap();
        assert_eq!(model.entity_type_id, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        assert_eq!(model.y_rot, 90.0);
        assert_eq!(model.x_rot, 15.0);
        assert!(model.appears_glowing);
        assert!(model.is_fully_frozen);
        assert!(model.main_arm_left);
        assert!(model.age_scale < 1.0);
        let held = source
            .entity_world
            .equipment_item(source.entity_id, EquipmentSlot::MainHand)
            .expect("persistent main-hand stack");
        assert_eq!(held.count, 1);
    }

    #[test]
    fn display_entity_cache_survives_update_tags_and_state_visibility_changes() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_trial_spawner(&mut world, pos, "active");
        apply_trial_spawner_data(
            &mut world,
            pos,
            trial_spawner_nbt("minecraft:zombie", false, false),
        );
        world.advance_trial_spawner_block_ticks(1);

        apply_trial_spawner_data(
            &mut world,
            pos,
            trial_spawner_nbt("minecraft:skeleton", false, false),
        );
        world.advance_trial_spawner_block_ticks(1);
        assert_eq!(
            projected_type(&world),
            Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
            "TrialSpawnerStateData does not clear its cached displayEntity on apply"
        );

        set_trial_spawner(&mut world, pos, "cooldown");
        world.advance_trial_spawner_block_ticks(1);
        assert!(world
            .trial_spawner_display_entity_source_states(1.0)
            .is_empty());

        set_trial_spawner(&mut world, pos, "waiting_for_players");
        world.advance_trial_spawner_block_ticks(1);
        assert_eq!(projected_type(&world), Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID));
        assert_ne!(
            projected_type(&world),
            Some(VANILLA_ENTITY_TYPE_SKELETON_ID)
        );

        let replacement = world.probe_chunk(ChunkPos { x: 0, z: 0 }).unwrap().clone();
        world.insert_decoded_chunk(replacement);
        world.advance_trial_spawner_block_ticks(1);
        assert_eq!(
            projected_type(&world),
            Some(VANILLA_ENTITY_TYPE_SKELETON_ID),
            "a full chunk replacement constructs a fresh TrialSpawnerStateData cache"
        );
    }

    #[test]
    fn waiting_state_uses_its_distinct_vanilla_spin_speed() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_trial_spawner(&mut world, pos, "waiting_for_players");
        apply_trial_spawner_data(
            &mut world,
            pos,
            trial_spawner_nbt("minecraft:zombie", false, false),
        );

        world.advance_trial_spawner_block_ticks(1);

        let source = world
            .trial_spawner_display_entity_source_states(1.0)
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(source.spin_degrees, 10.0);
    }

    #[test]
    fn replacing_the_block_discards_the_cached_display_entity() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_trial_spawner(&mut world, pos, "active");
        apply_trial_spawner_data(
            &mut world,
            pos,
            trial_spawner_nbt("minecraft:zombie", false, false),
        );
        world.advance_trial_spawner_block_ticks(1);

        assert!(world.apply_block_update(BlockUpdate {
            pos: protocol_pos(pos),
            block_state_id: crate::vanilla_air_block_state_id(),
        }));
        assert!(world.trial_spawner_block_render_states().is_empty());

        set_trial_spawner(&mut world, pos, "active");
        apply_trial_spawner_data(
            &mut world,
            pos,
            trial_spawner_nbt("minecraft:skeleton", false, false),
        );
        world.advance_trial_spawner_block_ticks(1);
        assert_eq!(
            projected_type(&world),
            Some(VANILLA_ENTITY_TYPE_SKELETON_ID)
        );
    }

    fn projected_type(world: &WorldStore) -> Option<i32> {
        let source = world
            .trial_spawner_display_entity_source_states(1.0)
            .into_iter()
            .next()?;
        source
            .entity_world
            .entity_model_source_at_partial_tick(source.entity_id, 1.0)
            .map(|model| model.entity_type_id)
    }

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
                block_states: PalettedContainerData {
                    domain: PaletteDomain::BlockStates,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![crate::vanilla_air_block_state_id()],
                    packed_data: Vec::new(),
                    entry_count: 4096,
                },
                biomes: PalettedContainerData {
                    domain: PaletteDomain::Biomes,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![0],
                    packed_data: Vec::new(),
                    entry_count: 64,
                },
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn set_trial_spawner(world: &mut WorldStore, pos: BlockPos, state: &str) {
        let properties = BTreeMap::from([
            ("ominous".to_string(), "false".to_string()),
            ("trial_spawner_state".to_string(), state.to_string()),
        ]);
        let block_state_id = world
            .registries()
            .block_state_id_by_name_and_properties("minecraft:trial_spawner", &properties)
            .expect("trial spawner state");
        assert!(world.apply_block_update(BlockUpdate {
            pos: protocol_pos(pos),
            block_state_id,
        }));
    }

    fn apply_trial_spawner_data(world: &mut WorldStore, pos: BlockPos, raw_nbt: Vec<u8>) {
        world
            .apply_block_entity_data(BlockEntityData {
                pos: protocol_pos(pos),
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID,
                raw_nbt,
            })
            .expect("trial spawner update tag");
    }

    fn protocol_pos(pos: BlockPos) -> ProtocolBlockPos {
        ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        }
    }

    fn trial_spawner_nbt(entity_id: &str, custom_render_state: bool, equipment: bool) -> Vec<u8> {
        let mut out = vec![10];
        write_tag_name(&mut out, 10, "spawn_data");
        write_tag_name(&mut out, 10, "entity");
        write_tag_name(&mut out, 8, "id");
        write_string(&mut out, entity_id);
        if custom_render_state {
            write_tag_name(&mut out, 1, "IsBaby");
            out.push(1);
            write_tag_name(&mut out, 1, "Glowing");
            out.push(1);
            write_tag_name(&mut out, 3, "TicksFrozen");
            out.extend_from_slice(&140_i32.to_be_bytes());
            write_tag_name(&mut out, 1, "LeftHanded");
            out.push(1);
            write_tag_name(&mut out, 9, "Rotation");
            out.push(5);
            out.extend_from_slice(&2_i32.to_be_bytes());
            out.extend_from_slice(&90.0_f32.to_bits().to_be_bytes());
            out.extend_from_slice(&15.0_f32.to_bits().to_be_bytes());
        }
        if equipment {
            write_tag_name(&mut out, 10, "equipment");
            write_tag_name(&mut out, 10, "mainhand");
            write_tag_name(&mut out, 8, "id");
            write_string(&mut out, "minecraft:stone");
            write_tag_name(&mut out, 3, "count");
            out.extend_from_slice(&1_i32.to_be_bytes());
            out.push(0);
            out.push(0);
        }
        out.push(0);
        out.push(0);
        write_tag_name(&mut out, 4, "next_mob_spawns_at");
        out.extend_from_slice(&0_i64.to_be_bytes());
        out.push(0);
        out
    }

    fn write_tag_name(out: &mut Vec<u8>, tag: u8, name: &str) {
        out.push(tag);
        write_string(out, name);
    }

    fn write_string(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }
}
