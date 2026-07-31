//! Canonical world projection for client-ticked block-entity particles/audio.
//!
//! Vanilla advances these effects from `ClientLevel.tickBlockEntities`, before
//! the separate block `animateTick` pass. Native owns the random cursor and
//! backend commands; this module owns the exact loaded block/entity facts and
//! existing ticker-state gates needed to reproduce the five effect families.

use bbb_protocol::block_entity_types::{
    VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID, VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID,
    VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
};
use serde::{Deserialize, Serialize};

use crate::{
    entities::vanilla_living_entity_type, BlockPos, VaultConnectionParticleState, WorldStore,
};

#[derive(Debug, Clone, PartialEq)]
pub enum BlockEntityClientEffectSource {
    Campfire(CampfireClientEffectSource),
    Spawner { pos: BlockPos },
    TrialSpawner(TrialSpawnerClientEffectSource),
    Conduit(ConduitClientEffectSource),
    Vault(VaultClientEffectSource),
}

impl BlockEntityClientEffectSource {
    fn sort_key(&self) -> (i32, i32, i32, u8) {
        let (pos, family) = match self {
            Self::Campfire(source) => (source.pos, 0),
            Self::Spawner { pos } => (*pos, 1),
            Self::TrialSpawner(source) => (source.pos, 2),
            Self::Conduit(source) => (source.pos, 3),
            Self::Vault(source) => (source.pos, 4),
        };
        (pos.y, pos.z, pos.x, family)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampfireClientEffectSource {
    pub pos: BlockPos,
    /// Vanilla `Direction.get2DDataValue`: south=0, west=1, north=2, east=3.
    pub facing_2d: u8,
    pub signal_fire: bool,
    pub occupied_slots: [bool; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrialSpawnerClientEffectSource {
    pub pos: BlockPos,
    pub state: TrialSpawnerClientState,
    pub ominous: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrialSpawnerClientState {
    Inactive,
    WaitingForPlayers,
    Active,
    WaitingForRewardEjection,
    EjectingReward,
    Cooldown,
}

impl TrialSpawnerClientState {
    pub(crate) fn from_block_property(state: &str) -> Option<Self> {
        match state {
            "inactive" => Some(Self::Inactive),
            "waiting_for_players" => Some(Self::WaitingForPlayers),
            "active" => Some(Self::Active),
            "waiting_for_reward_ejection" => Some(Self::WaitingForRewardEjection),
            "ejecting_reward" => Some(Self::EjectingReward),
            "cooldown" => Some(Self::Cooldown),
            _ => None,
        }
    }

    pub fn is_capable_of_spawning(self) -> bool {
        matches!(self, Self::WaitingForPlayers | Self::Active)
    }

    pub(crate) fn has_spinning_mob(self) -> bool {
        matches!(self, Self::WaitingForPlayers | Self::Active)
    }

    pub(crate) fn spinning_mob_speed(self) -> Option<f64> {
        match self {
            Self::WaitingForPlayers => Some(200.0),
            Self::Active => Some(1000.0),
            Self::Inactive
            | Self::WaitingForRewardEjection
            | Self::EjectingReward
            | Self::Cooldown => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConduitClientEffectSource {
    pub pos: BlockPos,
    pub tick_count: i32,
    pub effect_blocks: Vec<BlockPos>,
    pub destroy_target: Option<ConduitDestroyTargetState>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConduitDestroyTargetState {
    pub entity_id: i32,
    pub position: [f64; 3],
    pub eye_y: f64,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VaultClientEffectSource {
    pub pos: BlockPos,
    pub ominous: bool,
    pub has_display_item: bool,
    pub previous_spin: f32,
    pub current_spin: f32,
    pub connection_particles: Option<VaultConnectionParticleState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VaultBlockState {
    pub pos: BlockPos,
    pub previous_spin: f32,
    pub current_spin: f32,
}

impl VaultBlockState {
    fn new(pos: BlockPos) -> Self {
        Self {
            pos,
            previous_spin: 0.0,
            current_spin: 0.0,
        }
    }
}

impl WorldStore {
    /// Advances `VaultClientData.updateDisplayItemSpin` for every loaded vault.
    pub fn advance_vault_block_ticks(&mut self, ticks: u32) {
        let positions = self.vault_positions();
        let mut states = std::mem::take(&mut self.vault_blocks);
        states.retain(|state| positions.binary_search(&state.pos).is_ok());
        for pos in positions {
            if !states.iter().any(|state| state.pos == pos) {
                states.push(VaultBlockState::new(pos));
            }
        }
        states.sort_by_key(|state| state.pos);
        for state in &mut states {
            for _ in 0..ticks {
                state.previous_spin = state.current_spin;
                state.current_spin = wrap_degrees(state.current_spin + 10.0);
            }
        }
        self.vault_blocks = states;
    }

    pub fn vault_block_states(&self) -> &[VaultBlockState] {
        &self.vault_blocks
    }

    /// One typed source inventory for every loaded block-entity client ticker
    /// with local particle or audio effects in Vanilla 26.1.
    pub fn block_entity_client_effect_sources(&self) -> Vec<BlockEntityClientEffectSource> {
        let mut sources = Vec::new();
        self.append_record_backed_effect_sources(&mut sources);
        sources.extend(
            self.spawner_client_effect_positions()
                .into_iter()
                .map(|pos| BlockEntityClientEffectSource::Spawner { pos }),
        );
        sources.extend(self.conduit_blocks.iter().map(|state| {
            BlockEntityClientEffectSource::Conduit(ConduitClientEffectSource {
                pos: state.pos,
                tick_count: state.tick_count,
                effect_blocks: state.effect_blocks.clone(),
                destroy_target: self.conduit_destroy_target(state.pos),
            })
        }));
        sources.sort_by_key(BlockEntityClientEffectSource::sort_key);
        sources
    }

    fn append_record_backed_effect_sources(
        &self,
        sources: &mut Vec<BlockEntityClientEffectSource>,
    ) {
        for chunk in &self.chunks {
            for record in &chunk.block_entities {
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + i32::from(record.local_x),
                    y: i32::from(record.y),
                    z: chunk.pos.z * 16 + i32::from(record.local_z),
                };
                let Some(block) = self.probe_block(pos) else {
                    continue;
                };
                match record.type_id {
                    VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID
                        if matches!(
                            block.block_name.as_deref(),
                            Some("minecraft:campfire" | "minecraft:soul_campfire")
                        ) && block.block_properties.get("lit").map(String::as_str)
                            == Some("true") =>
                    {
                        let data = record.campfire.unwrap_or_default();
                        let Some(facing_2d) = block
                            .block_properties
                            .get("facing")
                            .and_then(|facing| campfire_facing_2d(facing))
                        else {
                            continue;
                        };
                        sources.push(BlockEntityClientEffectSource::Campfire(
                            CampfireClientEffectSource {
                                pos,
                                facing_2d,
                                signal_fire: block
                                    .block_properties
                                    .get("signal_fire")
                                    .map(String::as_str)
                                    == Some("true"),
                                occupied_slots: data.occupied_slots,
                            },
                        ));
                    }
                    VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID
                        if block.block_name.as_deref() == Some("minecraft:trial_spawner") =>
                    {
                        let Some(state) = block
                            .block_properties
                            .get("trial_spawner_state")
                            .and_then(|state| TrialSpawnerClientState::from_block_property(state))
                        else {
                            continue;
                        };
                        sources.push(BlockEntityClientEffectSource::TrialSpawner(
                            TrialSpawnerClientEffectSource {
                                pos,
                                state,
                                ominous: block.block_properties.get("ominous").map(String::as_str)
                                    == Some("true"),
                            },
                        ));
                    }
                    VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID
                        if block.block_name.as_deref() == Some("minecraft:vault") =>
                    {
                        let shared = record.vault_shared_data.as_ref();
                        let spin = self
                            .vault_blocks
                            .iter()
                            .find(|state| state.pos == pos)
                            .copied()
                            .unwrap_or_else(|| VaultBlockState::new(pos));
                        sources.push(BlockEntityClientEffectSource::Vault(
                            VaultClientEffectSource {
                                pos,
                                ominous: block.block_properties.get("ominous").map(String::as_str)
                                    == Some("true"),
                                has_display_item: shared
                                    .is_some_and(|shared| shared.has_display_item),
                                previous_spin: spin.previous_spin,
                                current_spin: spin.current_spin,
                                connection_particles: self.vault_connection_particle_state(pos),
                            },
                        ));
                    }
                    _ => {}
                }
            }
        }
    }

    fn vault_positions(&self) -> Vec<BlockPos> {
        let mut positions = Vec::new();
        for chunk in &self.chunks {
            for record in &chunk.block_entities {
                if record.type_id != VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID {
                    continue;
                }
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + i32::from(record.local_x),
                    y: i32::from(record.y),
                    z: chunk.pos.z * 16 + i32::from(record.local_z),
                };
                if self
                    .probe_block(pos)
                    .is_some_and(|block| block.block_name.as_deref() == Some("minecraft:vault"))
                {
                    positions.push(pos);
                }
            }
        }
        positions.sort();
        positions.dedup();
        positions
    }

    fn conduit_destroy_target(&self, pos: BlockPos) -> Option<ConduitDestroyTargetState> {
        let target_uuid = self.block_entity_record_at(pos)?.conduit_destroy_target?;
        let entity_id = self.entities.protocol_id_for_uuid(target_uuid)?;
        let identity = self.entities.identity(entity_id)?;
        if !vanilla_living_entity_type(identity.entity_type_id) {
            return None;
        }
        let pose = self.probe_entity_camera_pose(entity_id)?;
        let bounds = self.entities.pick_bounds(entity_id)?;
        Some(ConduitDestroyTargetState {
            entity_id,
            position: [pose.position.x, pose.position.y, pose.position.z],
            eye_y: pose.position.y + f64::from(pose.eye_height),
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
        })
    }
}

fn campfire_facing_2d(facing: &str) -> Option<u8> {
    match facing {
        "south" => Some(0),
        "west" => Some(1),
        "north" => Some(2),
        "east" => Some(3),
        _ => None,
    }
}

fn wrap_degrees(value: f32) -> f32 {
    let wrapped = value.rem_euclid(360.0);
    if wrapped >= 180.0 {
        wrapped - 360.0
    } else {
        wrapped
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::{
        block_entity_types::{
            VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID, VANILLA_BLOCK_ENTITY_TYPE_CONDUIT_ID,
            VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID, VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
        },
        entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        packets::{
            AddEntity, BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate,
            Vec3d as ProtocolVec3d,
        },
    };
    use uuid::Uuid;

    use super::*;
    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, LocalPlayerPoseState,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };

    #[test]
    fn block_entity_client_effect_sources_project_all_five_ticker_families() {
        let mut world = world_with_air_chunk();
        let campfire_pos = BlockPos { x: 1, y: 2, z: 1 };
        set_block(
            &mut world,
            campfire_pos,
            "minecraft:campfire",
            &[
                ("facing", "north"),
                ("lit", "true"),
                ("signal_fire", "true"),
                ("waterlogged", "false"),
            ],
        );
        apply_block_entity_data(
            &mut world,
            campfire_pos,
            VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID,
            campfire_nbt(),
        );

        let spawner_pos = BlockPos { x: 2, y: 3, z: 2 };
        set_block(&mut world, spawner_pos, "minecraft:spawner", &[]);
        apply_block_entity_data(
            &mut world,
            spawner_pos,
            VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
            spawner_nbt(),
        );
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 2.5,
                y: 3.5,
                z: 2.5,
            },
            ..LocalPlayerPoseState::default()
        });
        world.advance_spawner_block_ticks(1);

        let trial_pos = BlockPos { x: 3, y: 4, z: 3 };
        set_block(
            &mut world,
            trial_pos,
            "minecraft:trial_spawner",
            &[("ominous", "true"), ("trial_spawner_state", "active")],
        );

        let target_uuid = Uuid::from_u128(0x00112233445566778899aabbccddeeff);
        world.apply_add_entity(AddEntity {
            id: 17,
            uuid: target_uuid,
            entity_type_id: VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            position: ProtocolVec3d {
                x: 8.0,
                y: 5.0,
                z: 9.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        let conduit_pos = BlockPos { x: 4, y: 5, z: 4 };
        set_block(
            &mut world,
            conduit_pos,
            "minecraft:conduit",
            &[("waterlogged", "true")],
        );
        apply_block_entity_data(
            &mut world,
            conduit_pos,
            VANILLA_BLOCK_ENTITY_TYPE_CONDUIT_ID,
            conduit_nbt(target_uuid),
        );
        world.conduit_blocks.push(crate::ConduitBlockState {
            pos: conduit_pos,
            tick_count: 7,
            active_rotation: 0.0,
            is_active: true,
            is_hunting: false,
            effect_blocks: vec![BlockPos { x: 4, y: 7, z: 4 }],
        });

        let vault_pos = BlockPos { x: 5, y: 6, z: 5 };
        set_block(
            &mut world,
            vault_pos,
            "minecraft:vault",
            &[
                ("facing", "east"),
                ("ominous", "false"),
                ("vault_state", "active"),
            ],
        );
        apply_block_entity_data(
            &mut world,
            vault_pos,
            VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
            vault_nbt(),
        );
        world.advance_vault_block_ticks(19);

        let sources = world.block_entity_client_effect_sources();
        assert_eq!(sources.len(), 5);
        assert_eq!(
            sources[0],
            BlockEntityClientEffectSource::Campfire(CampfireClientEffectSource {
                pos: campfire_pos,
                facing_2d: 2,
                signal_fire: true,
                occupied_slots: [false, true, false, false],
            })
        );
        assert_eq!(
            sources[1],
            BlockEntityClientEffectSource::Spawner { pos: spawner_pos }
        );
        assert_eq!(
            sources[2],
            BlockEntityClientEffectSource::TrialSpawner(TrialSpawnerClientEffectSource {
                pos: trial_pos,
                state: TrialSpawnerClientState::Active,
                ominous: true,
            })
        );
        let BlockEntityClientEffectSource::Conduit(conduit) = &sources[3] else {
            panic!("fourth source should be conduit");
        };
        assert_eq!(conduit.pos, conduit_pos);
        assert_eq!(conduit.tick_count, 7);
        assert_eq!(conduit.effect_blocks, vec![BlockPos { x: 4, y: 7, z: 4 }]);
        let target = conduit.destroy_target.expect("loaded living target");
        assert_eq!(target.entity_id, 17);
        assert_eq!(target.position, [8.0, 5.0, 9.0]);
        assert!(target.eye_y > target.position[1]);
        assert!(target.width > 0.0);
        assert!(target.height > 0.0);

        let BlockEntityClientEffectSource::Vault(vault) = &sources[4] else {
            panic!("fifth source should be vault");
        };
        assert_eq!(vault.pos, vault_pos);
        assert!(vault.has_display_item);
        assert_eq!(vault.previous_spin, -180.0);
        assert_eq!(vault.current_spin, -170.0);
        assert_eq!(vault.connection_particles, None);
    }

    #[test]
    fn effect_source_gates_unlit_invalid_and_inactive_families() {
        let mut world = world_with_air_chunk();
        let campfire_pos = BlockPos { x: 1, y: 2, z: 1 };
        set_block(
            &mut world,
            campfire_pos,
            "minecraft:campfire",
            &[
                ("facing", "north"),
                ("lit", "false"),
                ("signal_fire", "false"),
                ("waterlogged", "false"),
            ],
        );
        let trial_pos = BlockPos { x: 2, y: 3, z: 2 };
        set_block(
            &mut world,
            trial_pos,
            "minecraft:trial_spawner",
            &[("ominous", "false"), ("trial_spawner_state", "inactive")],
        );

        let sources = world.block_entity_client_effect_sources();
        assert_eq!(
            sources,
            vec![BlockEntityClientEffectSource::TrialSpawner(
                TrialSpawnerClientEffectSource {
                    pos: trial_pos,
                    state: TrialSpawnerClientState::Inactive,
                    ominous: false,
                }
            )]
        );
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

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str, properties: &[(&str, &str)]) {
        let properties = properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("no state for {name} {properties:?}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: protocol_pos(pos),
            block_state_id: state_id,
        }));
    }

    fn apply_block_entity_data(
        world: &mut WorldStore,
        pos: BlockPos,
        block_entity_type_id: i32,
        raw_nbt: Vec<u8>,
    ) {
        world
            .apply_block_entity_data(BlockEntityData {
                pos: protocol_pos(pos),
                block_entity_type_id,
                raw_nbt,
            })
            .expect("block entity data should decode");
    }

    fn protocol_pos(pos: BlockPos) -> ProtocolBlockPos {
        ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        }
    }

    fn campfire_nbt() -> Vec<u8> {
        let mut out = vec![10];
        write_tag_name(&mut out, 9, "Items");
        out.push(10);
        out.extend_from_slice(&1_i32.to_be_bytes());
        write_tag_name(&mut out, 1, "Slot");
        out.push(1);
        write_tag_name(&mut out, 8, "id");
        write_string(&mut out, "minecraft:cod");
        write_tag_name(&mut out, 3, "count");
        out.extend_from_slice(&1_i32.to_be_bytes());
        out.push(0);
        out.push(0);
        out
    }

    fn spawner_nbt() -> Vec<u8> {
        let mut out = vec![10];
        write_tag_name(&mut out, 10, "SpawnData");
        write_tag_name(&mut out, 10, "entity");
        write_tag_name(&mut out, 8, "id");
        write_string(&mut out, "minecraft:zombie");
        out.push(0);
        out.push(0);
        out.push(0);
        out
    }

    fn conduit_nbt(target: Uuid) -> Vec<u8> {
        let mut out = vec![10];
        write_tag_name(&mut out, 11, "Target");
        out.extend_from_slice(&4_i32.to_be_bytes());
        let bits = target.as_u128();
        for part in [
            (bits >> 96) as u32,
            (bits >> 64) as u32,
            (bits >> 32) as u32,
            bits as u32,
        ] {
            out.extend_from_slice(&(part as i32).to_be_bytes());
        }
        out.push(0);
        out
    }

    fn vault_nbt() -> Vec<u8> {
        let mut out = vec![10];
        write_tag_name(&mut out, 10, "shared_data");
        write_tag_name(&mut out, 10, "display_item");
        write_tag_name(&mut out, 8, "id");
        write_string(&mut out, "minecraft:trial_key");
        write_tag_name(&mut out, 3, "count");
        out.extend_from_slice(&1_i32.to_be_bytes());
        out.push(0);
        out.push(0);
        out.push(0);
        out
    }

    fn write_tag_name(out: &mut Vec<u8>, tag_id: u8, name: &str) {
        out.push(tag_id);
        write_string(out, name);
    }

    fn write_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
