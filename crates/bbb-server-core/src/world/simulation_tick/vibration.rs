//! Live world reads and effects for game-event listeners.

use super::*;
use crate::level_event::VanillaGameEvent;
use crate::world::LevelId;

use crate::block_entity::vibration::{
    GameEventTag, VibrationEntity, VibrationSourceFacts, DAMPENS_VIBRATIONS_TAG,
};
use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::codec::Encoder;
use bbb_protocol::packets::{chunks::encode_block_pos, LevelParticles};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_VIBRATION_ID;
use bbb_protocol::PersistentRegistryTagMembers;
use bbb_server_types::block_queries::block_type_of;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use uuid::Uuid;

pub(super) fn schedule_block_tick_at(
    world: &mut ServerWorld,
    dimension: &LevelId,
    pos: BlockPos,
    delay: i32,
) {
    let Some(state) = world
        .level_by_id(dimension)
        .chunks
        .get_block_state(pos.x, pos.y, pos.z)
    else {
        return;
    };
    let Some(tick_type) = BlockTickType::of_block_state(state) else {
        return;
    };
    world.schedule_block_tick(dimension, pos, tick_type, delay, TickPriority::Normal);
}

/// One level as the vibration path reads it, plus the two side effects a
/// listener body raises (`setChanged` and the travelling particle).
pub(crate) struct WorldVibrationReads<'a> {
    pub(super) world: &'a mut ServerWorld,
    pub(super) dimension: &'a LevelId,
    pub(super) listener_pos: BlockPos,
    pub(super) events: &'a mut Vec<WorldEvent>,
}

impl WorldVibrationReads<'_> {
    pub(crate) fn trigger_avoid_vibration(&mut self, entity_id: i32) {
        let Some(connection) = self.world.connection_by_entity_id(entity_id) else {
            return;
        };
        if self.world.levels[connection].dimension == *self.dimension {
            self.world
                .fire_player_trigger(connection, "minecraft:avoid_vibration", self.events);
        }
    }

    pub(crate) fn block_state(&self, pos: BlockPos) -> Option<i32> {
        self.world
            .level_by_id(self.dimension)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z)
    }

    pub(crate) fn set_block_state(&mut self, pos: BlockPos, block_state_id: i32, flags: i32) {
        self.world
            .set_block(self.dimension, pos, block_state_id, flags);
    }

    pub(crate) fn schedule_block_tick(&mut self, pos: BlockPos, delay: i32) {
        schedule_block_tick_at(self.world, self.dimension, pos, delay);
    }

    pub(crate) fn next_random_float(&mut self) -> f32 {
        self.world
            .level_mut_by_id(self.dimension)
            .random
            .next_float()
    }

    fn item_dampens_vibrations(&self, item_type_id: i32) -> bool {
        let Some(item) = bbb_protocol::item_types::vanilla_item_type(item_type_id) else {
            return false;
        };
        let builtin = || {
            matches!(
                vanilla_persistent_registry_tag_members_26_1(
                    "minecraft:item",
                    DAMPENS_VIBRATIONS_TAG,
                ),
                PersistentRegistryTagMembers::Present(members)
                    if members.iter().any(|member| member == item.resource_id)
            )
        };
        self.world
            .level_by_id(self.dimension)
            .chunks
            .registry_tags_shared()
            .map_or_else(builtin, |tags| {
                tags.contains("minecraft:item", DAMPENS_VIBRATIONS_TAG, item.resource_id)
            })
    }
}

impl WorldVibrationReads<'_> {
    pub(crate) fn signal(
        &mut self,
        pos: BlockPos,
        direction: bbb_protocol::packets::Direction,
    ) -> Option<i32> {
        self.world
            .vibration_back_signal(self.dimension, pos, direction)
    }

    pub(crate) fn game_time(&self) -> i64 {
        self.world.game_time()
    }

    pub(crate) fn block_state_in_tag(&self, block_state_id: i32, tag_id: &str) -> bool {
        let Some(block_id) = block_type_of(block_state_id) else {
            return false;
        };
        self.world
            .level_by_id(self.dimension)
            .chunks
            .registry_tags_shared()
            .map_or_else(
                || {
                    crate::block_entity::vibration::builtin_block_state_in_tag(
                        block_state_id,
                        tag_id,
                    )
                },
                |tags| tags.contains_block(tag_id, block_id),
            )
    }

    pub(crate) fn game_event_in_tag(&self, event: VanillaGameEvent, tag: GameEventTag) -> bool {
        self.world
            .level_by_id(self.dimension)
            .chunks
            .registry_tags_shared()
            .map_or_else(
                || tag.contains_event(event),
                |tags| {
                    tags.contains(
                        "minecraft:game_event",
                        tag.resource_id(),
                        event.resource_id(),
                    )
                },
            )
    }

    pub(crate) fn source_entity_facts(&self, entity_id: i32) -> Option<VibrationSourceFacts> {
        let player = self
            .world
            .levels
            .players()
            .find(|state| state.entity_id() == entity_id);
        if let Some(player) = player {
            return Some(VibrationSourceFacts {
                spectator: player.game_type == GameType::Spectator,
                // `ServerPlayer.isSteppingCarefully` is the shift bit of the
                // last declared input.
                stepping_carefully: player.last_client_input.shift,
                dampens_vibrations: false,
            });
        }
        let entity = self
            .world
            .level_by_id(self.dimension)
            .entities
            .get(entity_id)?;
        Some(VibrationSourceFacts {
            spectator: false,
            stepping_carefully: crate::entity::inside_blocks::step::is_stepping_carefully(entity),
            dampens_vibrations: entity
                .data()
                .dampens_vibrations(|item_type_id| self.item_dampens_vibrations(item_type_id)),
        })
    }

    pub(crate) fn is_online_player(&self, uuid: Uuid) -> bool {
        self.world
            .levels
            .players()
            .any(|state| state.profile.uuid == uuid)
    }

    pub(crate) fn entity_identity(&self, entity_id: i32) -> Option<VibrationEntity> {
        let player = self
            .world
            .levels
            .players()
            .find(|state| state.entity_id() == entity_id);
        if let Some(player) = player {
            return Some(VibrationEntity {
                uuid: player.profile.uuid,
                is_player: true,
                is_projectile: false,
                controlling_passenger_player: None,
                projectile_owner: None,
                item_owner: None,
            });
        }
        let level = self.world.level_by_id(self.dimension);
        let entity = level.entities.get(entity_id)?;
        let controlling_passenger_player = if level.entities.passengers(entity_id).is_empty() {
            None
        } else {
            let tags = level.chunks.registry_tags_shared().map_or(
                crate::entity::EntityRegistryTagView::Builtin26_1,
                crate::entity::EntityRegistryTagView::Frozen,
            );
            crate::entity::direct_controlling_player_with_registry_tags(
                entity,
                Some(&level.entities),
                &tags,
                |passenger_id| {
                    self.world
                        .levels
                        .players()
                        .find(|state| state.entity_id() == passenger_id)
                        .map(ServerWorld::nearby_player_facts)
                },
            )
            .flatten()
            .map(|controller| controller.uuid)
        };
        Some(VibrationEntity {
            uuid: entity.uuid(),
            is_player: false,
            is_projectile: entity.is_projectile_class(),
            controlling_passenger_player,
            projectile_owner: entity.data().projectile_owner(),
            item_owner: entity
                .data()
                .item_thrower()
                .filter(|&uuid| self.is_online_player(uuid)),
        })
    }

    pub(crate) fn entity_identity_by_uuid(&self, uuid: Uuid) -> Option<VibrationEntity> {
        if self
            .world
            .levels
            .players()
            .any(|state| state.profile.uuid == uuid)
        {
            return Some(VibrationEntity {
                uuid,
                is_player: true,
                is_projectile: false,
                controlling_passenger_player: None,
                projectile_owner: None,
                item_owner: None,
            });
        }
        let entity_id = self
            .world
            .level_by_id(self.dimension)
            .entities
            .entity_id_for_uuid(uuid)?;
        self.entity_identity(entity_id)
    }

    pub(crate) fn adjacent_chunks_ticking(&self, pos: BlockPos) -> bool {
        let level = self.world.level_by_id(self.dimension);
        for chunk_x in (pos.x >> 4) - 1..=(pos.x >> 4) + 1 {
            for chunk_z in (pos.z >> 4) - 1..=(pos.z >> 4) + 1 {
                if !level.simulation_levels.is_block_ticking((chunk_x, chunk_z))
                    || !level.chunks.is_loaded(chunk_x, chunk_z)
                {
                    return false;
                }
            }
        }
        true
    }

    pub(crate) fn mark_block_entity_changed(&mut self, pos: BlockPos) {
        self.world
            .level_mut_by_id(self.dimension)
            .chunks
            .block_entities_mut()
            .set_changed(pos);
    }

    pub(crate) fn send_vibration_particle(&mut self, origin: Vec3d, travel_time_in_ticks: i32) {
        // `ServerLevel.sendParticles(new VibrationParticleOption(...))`
        let packet = LevelParticles {
            override_limiter: false,
            always_show: false,
            position: origin,
            offset: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max_speed: 0.0,
            count: 1,
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_VIBRATION_ID,
                raw_options: vibration_particle_options(self.listener_pos, travel_time_in_ticks),
            },
        };
        let mut recipients = Vec::new();
        for (&connection_id, listener) in self.world.levels.player_entries() {
            if &listener.dimension != self.dimension {
                continue;
            }
            if particle_reaches(
                origin.x,
                origin.y,
                origin.z,
                listener.position().x,
                listener.position().y,
                listener.position().z,
                false,
            ) {
                recipients.push(connection_id);
            }
        }
        crate::world::player_packets::broadcast_to(
            recipients,
            ClientboundPlayPacket::LevelParticles(packet),
            self.events,
        );
    }
}

/// `VibrationParticleOption.STREAM_CODEC` (`VibrationParticleOption.java:23-29`)
/// over `BlockPositionSource.STREAM_CODEC` (`BlockPositionSource.java:16-18`)
/// and `BlockPos.STREAM_CODEC`.
fn vibration_particle_options(destination: BlockPos, arrival_in_ticks: i32) -> Vec<u8> {
    let mut encoder = Encoder::default();
    // `PositionSource.STREAM_CODEC`'s registry dispatch
    // (`PositionSource.java:15-16`): `PositionSourceType.BLOCK` is row zero.
    encoder.write_var_i32(0);
    encoder.write_i64(encode_block_pos(destination));
    encoder.write_var_i32(arrival_in_ticks);
    encoder.into_inner()
}
