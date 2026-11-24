//! Session adapter for synchronous block landing callbacks.
#[cfg(test)]
use super::super::stored_block_state;
use super::super::{ServerWorld, WorldEvent};
use crate::block_behavior::ground_contact::{self, FallOnBlock};
use crate::connection::ClientboundPlayPacket;
use crate::connection::ConnectionId;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_FALL_DAMAGE_MULTIPLIER_ID, VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID,
};
use bbb_protocol::packets::{
    BlockPos, EntityEvent, LevelParticles, ParticlePayload, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_server_types::stats::custom_stat;

impl ServerWorld {
    // ServerPlayer.java:1277-1286: this precedes fluid refresh and reads the old distance.
    pub(super) fn emit_player_extra_fall_particles(
        &mut self,
        id: ConnectionId,
        state: i32,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self
            .levels
            .player_mut(id)
            .expect("the landing player is online");
        if !player.cold.spawn_extra_particles_on_fall
            || player.block_contact.fall_distance.partial_cmp(&0.0)
                != Some(std::cmp::Ordering::Greater)
        {
            return Some(());
        }
        let count = (50.0 * player.block_contact.fall_distance).clamp(0.0, 200.0) as i32;
        player.cold.spawn_extra_particles_on_fall = false;
        let mut encoder = bbb_protocol::codec::Encoder::default();
        encoder.write_var_i32(state);
        let packet = LevelParticles {
            override_limiter: false,
            always_show: false,
            position: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 1.0,
                z: f64::from(pos.z) + 0.5,
            },
            offset: Vec3d {
                x: f64::from(0.3_f32),
                y: f64::from(0.3_f32),
                z: f64::from(0.3_f32),
            },
            max_speed: 0.15,
            count,
            particle: ParticlePayload {
                particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID,
                raw_options: encoder.into_inner(),
            },
        };
        self.publish_player_particles(id, packet, events);
        Some(())
    }

    pub(super) fn emit_player_fall_particles(
        &self,
        id: ConnectionId,
        state: i32,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self
            .levels
            .player(id)
            .expect("the landing player is online");
        if player.block_contact.fall_distance.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
        {
            return Some(());
        }
        let safe = player
            .attributes
            .value(VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID)?;
        if let Some(packet) = crate::entity::living::body_fall_particles(
            player.position(),
            pos,
            state,
            player.block_contact.fall_distance,
            safe,
        ) {
            self.publish_player_particles(id, packet, events);
        }
        Some(())
    }

    // ServerLevel.java:1356-1369: immediate sends include the emitting player.
    pub(in crate::world) fn publish_player_particles(
        &self,
        id: ConnectionId,
        packet: LevelParticles,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = &self.levels[id].dimension;
        let mut recipients = Vec::new();
        for (&listener_id, listener) in self.levels.player_entries() {
            if &listener.dimension == dimension
                && crate::level_event::particle_reaches(
                    packet.position.x,
                    packet.position.y,
                    packet.position.z,
                    listener.position().x,
                    listener.position().y,
                    listener.position().z,
                    false,
                )
            {
                recipients.push(listener_id);
            }
        }
        crate::world::player_packets::broadcast_to(
            recipients,
            ClientboundPlayPacket::LevelParticles(packet),
            events,
        );
    }
}

impl ServerWorld {
    pub(super) fn player_block_fall_on(
        &mut self,
        id: ConnectionId,
        state: i32,
        pos: BlockPos,
        distance: f64,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let body = self.player_ground_body(id)?;
        let player = self
            .levels
            .player(id)
            .expect("the landing player is online");
        let (dimension, entity_id) = (player.dimension, player.entity_id());
        let random = &self.random;
        let landing = ground_contact::fall_on_with_registry(
            state,
            distance,
            body,
            random,
            self.registries.content_registry(),
        )?;
        match landing.block {
            FallOnBlock::None => {}
            FallOnBlock::TrampleFarmland => {
                let dirt = bbb_protocol::block_state_types::vanilla_block_default_state_id(
                    bbb_protocol::block_types::VANILLA_BLOCK_DIRT_ID,
                )?;
                self.push_live_entities_up(&dimension, state, dirt, pos, events)?;
                self.with_level(dimension, |level| {
                    crate::block_behavior::farmland::turn_to_dirt_with_source(
                        level,
                        pos,
                        Some(entity_id),
                    );
                });
            }
            FallOnBlock::DecreaseTurtleEggs => self.with_level(dimension, |level| {
                crate::block_behavior::turtle_egg::decrease_eggs(level, pos, state)
            })?,
            FallOnBlock::FallSound { big } => {
                let sound = if big {
                    "minecraft:entity.player.big_fall"
                } else {
                    "minecraft:entity.player.small_fall"
                };
                self.play_player_landing_sound(id, sound, 1.0, 1.0, events);
            }
            FallOnBlock::HoneySlide => {
                self.play_player_landing_sound(
                    id,
                    "minecraft:block.honey_block.slide",
                    1.0,
                    1.0,
                    events,
                );
                self.send_to_player_tracking_and_self(
                    id,
                    entity_id,
                    ClientboundPlayPacket::EntityEvent(EntityEvent {
                        entity_id,
                        event_id: 54,
                    }),
                    events,
                );
            }
        }
        if let Some(damage) = landing.damage {
            let hurt = self.cause_player_fall_damage(
                id,
                damage.distance,
                damage.modifier,
                damage.damage_type,
                events,
            )?;
            if let Some((sound, volume, pitch)) = landing.damaged_sound.filter(|_| hurt) {
                self.play_player_landing_sound(id, sound, volume, pitch, events);
            }
        }
        Some(())
    }

    // Player.java:1428-1437: mayfly gates even statistics; hurt admission comes later.
    pub(in crate::world) fn cause_player_fall_damage(
        &mut self,
        id: ConnectionId,
        distance: f64,
        modifier: f32,
        damage_type: &'static str,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        if self.levels[id].abilities.can_fly {
            return Some(false);
        }
        let player = self
            .levels
            .player_mut(id)
            .expect("the landing player is online");
        if distance >= 2.0 {
            player.award_stat(
                &mut self.scoreboard,
                custom_stat("minecraft:fall_one_cm"),
                ((distance * 100.0 + 0.5).floor() as i64) as i32,
            );
        }
        let distance = player.adjusted_impulse_fall_distance(distance);
        let immune = self.player_registry_tags(id)?.contains(
            "minecraft:entity_type",
            "minecraft:fall_damage_immune",
            "minecraft:player",
        );
        if immune {
            return Some(false);
        }
        let player = self
            .levels
            .player_mut(id)
            .expect("the landing player is online");
        let damage = crate::entity::living::calculate_body_fall_damage(
            distance,
            modifier,
            player
                .attributes
                .value(VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID)?,
            player
                .attributes
                .value(VANILLA_ATTRIBUTE_FALL_DAMAGE_MULTIPLIER_ID)?,
            immune,
        );
        if damage <= 0 {
            return Some(false);
        }
        self.levels[id].reset_fall_impulse();
        self.play_player_landing_sound(
            id,
            crate::entity::living::fall_damage_sound(
                damage,
                "minecraft:entity.player.small_fall",
                "minecraft:entity.player.big_fall",
            ),
            1.0,
            1.0,
            events,
        );
        if !self.levels[id].silent {
            let player = self
                .levels
                .player(id)
                .expect("the landing player is online");
            let level = self.level_by_id(&player.dimension);
            let mut known = true;
            let sound = crate::entity::living::body_block_fall_sound(
                player.position(),
                SoundSource::Players,
                |pos| {
                    self.player_contact_state(id, pos).unwrap_or_else(|| {
                        known = false;
                        level.chunks.air_state_id()
                    })
                },
            );
            if !known {
                return None;
            }
            if let Some(sound) = sound {
                self.play_player_landing_sound(
                    id,
                    &sound.resource_id,
                    sound.volume,
                    sound.pitch,
                    events,
                );
            }
        }
        self.hurt_connected_player_checked(
            id,
            damage_type,
            damage as f32,
            None,
            None,
            None,
            events,
        )?;
        // LivingEntity.java:1770-1777: a refused hurt still reports positive calculated damage.
        Some(true)
    }

    pub(super) fn play_player_landing_sound(
        &mut self,
        id: ConnectionId,
        resource_id: &str,
        volume: f32,
        pitch: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(id) else {
            return;
        };
        let position = player.position();
        let except = Some(id);
        let Some(registry_id) =
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id)
        else {
            return;
        };
        // Player.java:397-403: player sounds exclude self and have no Entity Silent gate.
        self.play_player_sound(
            id,
            except,
            SoundEventHolder::Reference { registry_id },
            SoundSource::Players,
            position,
            volume,
            pitch,
            events,
        );
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod particle_tests;

#[cfg(test)]
mod velocity_tests;
