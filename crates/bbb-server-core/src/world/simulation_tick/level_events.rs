//! The `Level.levelEvent`/playSeededSound/sendParticles broadcasts a
//! mechanism queued on its level slice, in the order it raised them.

use super::*;
use crate::world::LevelId;

/// `GameRules.GLOBAL_SOUND_EVENTS` (`gamerules/GameRules.java:44`).
const RULE_GLOBAL_SOUND_EVENTS: &str = "minecraft:global_sound_events";

impl ServerWorld {
    /// Every `Level.levelEvent` a mechanism raised since the last flush,
    /// broadcast in the order it was raised.
    ///
    /// **Where this differs from vanilla.** `ServerLevel.levelEvent`
    /// (`ServerLevel.java:1095-1107`) sends its packet inside the body that
    /// raised it. A mechanism here holds a [`LevelCtx`], which cannot see the
    /// sessions, so the event is queued on the level and delivered at the end
    /// of the phase that raised it. The recipients and the payload are
    /// vanilla's; what can differ is the packet's position relative to other
    /// packets produced later in the same phase.
    pub(crate) fn flush_level_events(&mut self, dimension: &LevelId, events: &mut Vec<WorldEvent>) {
        // A game-event delivery ran at the end of a level frame, which may not
        // have held the tick's own output list; its packets leave here.
        events.append(&mut self.deferred_game_event_events);
        if !self
            .level_by_id(dimension)
            .event_sink
            .has_pending_level_events()
        {
            return;
        }
        let pending: Vec<PendingLevelBroadcast> = self
            .level_mut_by_id(dimension)
            .event_sink
            .drain_level_broadcasts()
            .collect();
        let sounds: Vec<PendingBlockSound> = self
            .level_mut_by_id(dimension)
            .event_sink
            .drain_block_sounds()
            .collect();
        for event in pending {
            match event {
                PendingLevelBroadcast::BeehiveEmergencyRelease { pos, state } => {
                    self.release_beehive_occupants(
                        *dimension,
                        pos,
                        state,
                        crate::world::beehive_release::BeehiveRelease::Emergency,
                    );
                }
                PendingLevelBroadcast::EntityAnimation { entity_id, hand } => {
                    // LivingEntity.java:1996-2005; ClientboundAnimatePacket.java:13-15.
                    let action = match hand {
                        bbb_protocol::packets::InteractionHand::MainHand => 0,
                        bbb_protocol::packets::InteractionHand::OffHand => 3,
                    };
                    for connection_id in self
                        .level_by_id(dimension)
                        .entity_tracker
                        .viewers_of(entity_id)
                    {
                        if self
                            .levels
                            .player(connection_id)
                            .is_some_and(|player| &player.dimension == dimension)
                        {
                            events.push(send(
                                connection_id,
                                ClientboundPlayPacket::EntityAnimation(EntityAnimation {
                                    id: entity_id,
                                    action,
                                }),
                            ));
                        }
                    }
                }
                PendingLevelBroadcast::LevelEvent(event) => {
                    self.broadcast_level_event_by_key(dimension, None, event, events)
                }
                PendingLevelBroadcast::GlobalLevelEvent(event) => {
                    self.broadcast_global_level_event(dimension, event, events)
                }
                PendingLevelBroadcast::BlockDestruction {
                    entity_id,
                    pos,
                    stage,
                } => self.destroy_block_progress(dimension, entity_id, pos, stage, events),
            }
        }
        for sound in sounds {
            self.play_queued_block_sound(dimension, sound, events);
        }
        let trail_particles: Vec<PendingTrailParticle> = self
            .level_mut_by_id(dimension)
            .event_sink
            .drain_trail_particles()
            .collect();
        for particle in trail_particles {
            self.broadcast_trail_particle(dimension, particle, events);
        }
    }

    pub(super) fn play_queued_block_sound(
        &mut self,
        dimension: &LevelId,
        sound: PendingBlockSound,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(registry_id) = vanilla_sound_event_registry_id(&sound.resource_id) else {
            return;
        };
        let packet = SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            source: SoundSource::Blocks,
            position: Vec3d {
                x: f64::from(sound.pos.x) + 0.5,
                y: f64::from(sound.pos.y) + 0.5,
                z: f64::from(sound.pos.z) + 0.5,
            },
            volume: sound.volume,
            pitch: sound.pitch,
            seed: self.random.next_long(),
        };
        let except = sound
            .except_entity_id
            .and_then(|entity_id| self.connection_by_entity_id(entity_id));
        self.broadcast_level_sound_by_key(dimension, &packet, except, events);
    }

    /// One `ServerLevel.levelEvent(source, type, pos, data)`
    /// (`ServerLevel.java:1095-1107`): `PlayerList.broadcast`
    /// (`PlayerList.java:597-617`) over
    /// [`LEVEL_EVENT_BROADCAST_RANGE`], with `globalEvent` false.
    ///
    /// `except` is the `@Nullable Entity source` narrowed to a player
    /// (`:1099`), which `broadcast` compares by identity before it measures
    /// (`PlayerList.java:608`). Only `Block.spawnDestroyParticles`
    /// (`Block.java:514-516`) passes one; every other producer here passes
    /// null through the three-argument overload (`LevelAccessor.java:84-86`).
    pub(in crate::world) fn broadcast_level_event_by_key(
        &self,
        dimension: &LevelId,
        except: Option<ConnectionId>,
        event: PendingLevelEvent,
        events: &mut Vec<WorldEvent>,
    ) {
        let packet = ClientboundPlayPacket::LevelEvent(bbb_protocol::packets::LevelEvent {
            event_type: event.event_type,
            pos: event.pos,
            data: event.data,
            // `ServerLevel.levelEvent` builds the packet with `false`
            // (`ServerLevel.java:1105`).
            global: false,
        });
        let mut recipients = Vec::new();
        for (&connection_id, listener) in self.levels.player_entries() {
            if except == Some(connection_id) || &listener.dimension != dimension {
                continue;
            }
            if level_event_reaches(
                event.pos,
                LEVEL_EVENT_BROADCAST_RANGE,
                listener.position().x,
                listener.position().y,
                listener.position().z,
            ) {
                recipients.push(connection_id);
            }
        }
        crate::world::player_packets::broadcast_to(recipients, packet, events);
    }

    /// `ServerLevel.globalLevelEvent` (`ServerLevel.java:1071-1092`).
    fn broadcast_global_level_event(
        &self,
        dimension: &LevelId,
        event: PendingLevelEvent,
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.game_rule_bool(RULE_GLOBAL_SOUND_EVENTS, true) {
            self.broadcast_level_event_by_key(dimension, None, event, events);
            return;
        }
        let center = [
            f64::from(event.pos.x) + 0.5,
            f64::from(event.pos.y) + 0.5,
            f64::from(event.pos.z) + 0.5,
        ];
        for (&connection_id, listener) in self.levels.player_entries() {
            let feet = [
                listener.position().x,
                listener.position().y,
                listener.position().z,
            ];
            let sound = if &listener.dimension == dimension {
                let offset = [0, 1, 2].map(|axis| center[axis] - feet[axis]);
                let distance_squared = offset.iter().map(|delta| delta * delta).sum::<f64>();
                if distance_squared < 32.0 * 32.0 {
                    center
                } else {
                    // `Vec3.normalize` (`Vec3.java:83-86`).
                    let length = distance_squared.sqrt();
                    [0, 1, 2].map(|axis| feet[axis] + offset[axis] / length * 32.0)
                }
            } else {
                feet
            };
            let [x, y, z] = sound.map(|coordinate| coordinate.floor() as i32);
            events.push(send(
                connection_id,
                ClientboundPlayPacket::LevelEvent(bbb_protocol::packets::LevelEvent {
                    event_type: event.event_type,
                    pos: BlockPos { x, y, z },
                    data: event.data,
                    global: true,
                }),
            ));
        }
    }

    /// `ServerLevel.sendParticles` (`ServerLevel.java:1321-1330`).
    pub(super) fn broadcast_trail_particle(
        &self,
        dimension: &LevelId,
        particle: PendingTrailParticle,
        events: &mut Vec<WorldEvent>,
    ) {
        let packet = ClientboundPlayPacket::LevelParticles(LevelParticles {
            override_limiter: false,
            always_show: false,
            position: Vec3d {
                x: particle.x,
                y: particle.y,
                z: particle.z,
            },
            offset: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max_speed: 0.0,
            // `EyeblossomBlock.Type.spawnTransformParticle` (`EyeblossomBlock.java:171`).
            count: 1,
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_TRAIL_ID,
                raw_options: encode_trail_particle_options(
                    particle.target_x,
                    particle.target_y,
                    particle.target_z,
                    particle.color,
                    particle.duration,
                ),
            },
        });
        let mut recipients = Vec::new();
        for (&connection_id, listener) in self.levels.player_entries() {
            if &listener.dimension != dimension {
                continue;
            }
            if particle_reaches(
                particle.x,
                particle.y,
                particle.z,
                listener.position().x,
                listener.position().y,
                listener.position().z,
                false,
            ) {
                recipients.push(connection_id);
            }
        }
        crate::world::player_packets::broadcast_to(recipients, packet, events);
    }

    /// `ServerLevel.playSeededSound` (`ServerLevel.java:1023-1045`) once the
    /// packet is built: `PlayerList.broadcast` (`PlayerList.java:597-617`) over
    /// `sound.value().getRange(volume)` (`:1041`).
    ///
    /// `except` is the `@Nullable Player` that call carries all the way down
    /// (`:597`), which `broadcast` compares by identity at `:608` before it
    /// measures the distance. Most callers pass `null` and every listener in
    /// range hears the sound; the ones that pass a player are saying that
    /// player already knows. The same line drops every listener standing in
    /// another level.
    pub(in crate::world) fn broadcast_level_sound_by_key(
        &self,
        dimension: &LevelId,
        sound: &SoundEvent,
        except: Option<ConnectionId>,
        events: &mut Vec<WorldEvent>,
    ) {
        let range = sound_broadcast_range(sound);
        let packet = ClientboundPlayPacket::Sound(sound.clone());
        // `PlayerList.broadcast`'s own recipient test is
        // `player != except && player.level().dimension() == dimension`
        // (`PlayerList.java:608`) before the range comparison.
        let mut recipients = Vec::new();
        for (&connection_id, listener) in self.levels.player_entries() {
            if except == Some(connection_id) || &listener.dimension != dimension {
                continue;
            }
            let dx = sound.position.x - listener.position().x;
            let dy = sound.position.y - listener.position().y;
            let dz = sound.position.z - listener.position().z;
            if dx * dx + dy * dy + dz * dz < range * range {
                recipients.push(connection_id);
            }
        }
        crate::world::player_packets::broadcast_to(recipients, packet, events);
    }

    #[cfg(test)]
    pub(crate) fn drain_block_sounds_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
    ) -> Vec<PendingBlockSound> {
        self.level_mut(dimension)
            .event_sink
            .drain_block_sounds()
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn drain_level_events_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
    ) -> Vec<PendingLevelEvent> {
        self.level_mut(dimension)
            .event_sink
            .drain_level_broadcasts()
            .map(|event| match event {
                PendingLevelBroadcast::LevelEvent(event) => event,
                PendingLevelBroadcast::BeehiveEmergencyRelease { .. }
                | PendingLevelBroadcast::BlockDestruction { .. }
                | PendingLevelBroadcast::GlobalLevelEvent(_)
                | PendingLevelBroadcast::EntityAnimation { .. } => {
                    panic!("unexpected entity broadcast in level-event fixture")
                }
            })
            .collect()
    }
}
