//! `CreakingHeartBlockEntity.serverTick`'s countdown half
//! (`CreakingHeartBlockEntity.java:70-133`): the level's answers for the
//! ticker, the protector spawn and the three release conditions.

use super::*;
use crate::block_behavior::multiface;
use crate::block_behavior::support::state_is_block;
use crate::fluid::with_property;
use crate::level_event::VanillaGameEvent;
use crate::world::block_entity_removal::SpawnStrategy;
use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air};

/// `CreakingHeartBlockEntity.spawnProtector`'s `SpawnUtil.trySpawnMob`
/// arguments (`CreakingHeartBlockEntity.java:201-203`).
const CREAKING_SPAWN_ATTEMPTS: i32 = 5;
const CREAKING_SPAWN_RANGE_XZ: i32 = 16;
const CREAKING_SPAWN_RANGE_Y: i32 = 8;
/// `Level.broadcastEntityEvent(creaking, (byte)60)`
/// (`CreakingHeartBlockEntity.java:209`).
const CREAKING_SPAWN_ANIMATION_EVENT_ID: i8 = 60;

impl ServerWorld {
    /// `CreakingHeartBlockEntity.serverTick` (`CreakingHeartBlockEntity.java:70-133`):
    /// the analog half, the emitter countdown and the ticker that spawns or
    /// releases the protector. The state machine and the decisions live in the
    /// block crate; this pass supplies the level's answers and runs the
    /// effects.
    pub(super) fn tick_creaking_heart(&mut self, dimension: &LevelId, pos: BlockPos) {
        let Some(block_state_id) = self.block_state_by_key(dimension, pos) else {
            return;
        };
        if !ticks_block_entity(block_state_id) {
            return;
        }
        let Some((uuid, ticker_due)) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::creaking_heart)
            .map(|heart| (heart.protector_uuid(), heart.ticker_due()))
        else {
            return;
        };
        // `getCreakingProtector`'s `serverLevel.getEntity(uuid) instanceof
        // Creaking` (`CreakingHeartBlockEntity.java:182-193`), whose
        // `isRemoved` test (`:175`) the store answers as `isAlive`.
        let protector_entity_id = uuid.and_then(|uuid| {
            let entities = &self.level_by_id(dimension).entities;
            let entity = entities.get(entities.entity_id_for_uuid(uuid)?)?;
            (entity.entity_type_id() == VANILLA_ENTITY_TYPE_CREAKING_ID && entity.is_alive())
                .then(|| entity.id())
        });
        // `Creaking.playerIsStuckInYou` (`Creaking.java:357-375`) is only
        // evaluated inside `if (entity.ticker-- < 0)` (`CreakingHeartBlockEntity.java:98-123`).
        let protector = protector_entity_id.and_then(|entity_id| {
            let (eye_inside, position, bounding_box, persistence_required) = {
                let entities = &self.level_by_id(dimension).entities;
                let entity = entities.get(entity_id)?;
                (
                    ticker_due && self.creaking_player_eye_inside(dimension, entity),
                    entity.position(),
                    entity.bounding_box(),
                    // `Mob.isPersistenceRequired` (`CreakingHeartBlockEntity.java:120`).
                    entity.is_persistence_required(),
                )
            };
            let player_stuck = ticker_due
                && self
                    .level_mut_by_id(dimension)
                    .entities
                    .get_mut(entity_id)
                    .is_some_and(|entity| entity.creaking_note_player_eye_inside(eye_inside));
            Some(
                crate::block_entity::creaking_heart::CreakingHeartProtectorState {
                    position,
                    bounding_box,
                    persistence_required,
                    player_stuck,
                },
            )
        });
        // `CreakingHeartBlock.hasRequiredLogs` (`CreakingHeartBlock.java:112-123`),
        // `EnvironmentAttributes.CREAKING_ACTIVE` (`:142`), and the 32-block
        // player query are only read inside `if (entity.ticker-- < 0)`.
        let (has_required_logs, creaking_active, spawning_monsters, has_player_within_32) =
            if ticker_due {
                let (has_required_logs, creaking_active) = self.with_level(dimension, |level| {
                    (
                        crate::block_behavior::creaking_heart::has_required_logs(
                            level,
                            pos,
                            block_state_id,
                        ),
                        level.creaking_active(pos),
                    )
                });
                let spawning_monsters = self
                    .game_rules
                    .spawning_monsters(self.admin.settings().difficulty);
                // `getNearestPlayer(x, y, z, 32.0, false)` over
                // `EntitySelector.NO_SPECTATORS` (`EntityGetter.java:95-98`, `:74-87`).
                let has_player_within_32 = self.levels.players().any(|player| {
                    &player.dimension == dimension
                        && !player.avatar_removed
                        && player.game_type != GameType::Spectator
                        && {
                            let (dx, dy, dz) = (
                                player.position().x - f64::from(pos.x),
                                player.position().y - f64::from(pos.y),
                                player.position().z - f64::from(pos.z),
                            );
                            dx * dx + dy * dy + dz * dz < 32.0 * 32.0
                        }
                });
                (
                    has_required_logs,
                    creaking_active,
                    spawning_monsters,
                    has_player_within_32,
                )
            } else {
                (None, false, false, false)
            };
        let input = crate::block_entity::creaking_heart::CreakingHeartTickInput {
            has_required_logs,
            creaking_active,
            spawning_monsters,
            has_player_within_32,
            protector,
        };
        let ServerLevel {
            chunks,
            random: world_random,
            ..
        } = self.level_mut_by_id(dimension);
        let outcome = chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::creaking_heart_mut)
            .map(|heart| heart.server_tick(pos, block_state_id, input, world_random));
        let Some(outcome) = outcome else {
            return;
        };

        if outcome.set_changed {
            self.level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .set_changed(pos);
        }
        if outcome.notify_output_signal {
            self.with_level(dimension, |level| {
                level.update_neighbour_for_output_signal(pos, VANILLA_BLOCK_CREAKING_HEART_ID);
            });
        }
        if let Some(new_state) = outcome.new_state {
            self.set_block(dimension, pos, new_state, SET_BLOCK_UPDATE_ALL);
        }
        for towards_creaking in outcome.emit_particles {
            self.emit_creaking_trail_particles(dimension, pos, input.protector, towards_creaking);
        }
        if let Some((sound_pos, volume)) = outcome.hurt_sound {
            self.with_level(dimension, |level| {
                level.play_block_sound_with_volume_pitch(
                    sound_pos,
                    "minecraft:block.creaking_heart.hurt",
                    volume,
                    1.0,
                );
            });
        }
        if outcome.spawn_protector {
            self.spawn_creaking_protector(dimension, pos);
        }
        if outcome.remove_protector {
            if let Some(uuid) = uuid {
                self.tear_down_creaking_protector(dimension, uuid);
                self.clear_creaking_protector_binding(dimension, pos);
            }
        }
    }

    /// `CreakingHeartBlockEntity.spawnProtector` (`:199-217`).
    fn spawn_creaking_protector(&mut self, dimension: &LevelId, pos: BlockPos) {
        let Some(entity_id) = self.try_spawn_mob(
            dimension,
            crate::world::block_entity_removal::MobSpawnRequest {
                entity_type_id: VANILLA_ENTITY_TYPE_CREAKING_ID,
                start: pos,
                attempts: CREAKING_SPAWN_ATTEMPTS,
                range_xz: CREAKING_SPAWN_RANGE_XZ,
                range_y: CREAKING_SPAWN_RANGE_Y,
                strategy: SpawnStrategy::OnTopOfColliderNoLeaves,
                check_collisions: true,
                reason: crate::entity::SpawnReason::Spawner,
            },
        ) else {
            return;
        };
        // `:207-209`.
        let (uuid, creaking_pos) = self
            .level_by_id(dimension)
            .entities
            .get(entity_id)
            .map(|entity| (entity.uuid(), entity.position()))
            .unzip();
        if let Some(creaking_pos) = creaking_pos {
            self.level_mut_by_id(dimension)
                .event_sink
                .game_event_with_source_at(
                    crate::level_event::VanillaGameEvent::ENTITY_PLACE,
                    creaking_pos,
                    Some(entity_id),
                    None,
                );
        }
        self.level_mut_by_id(dimension)
            .entities
            .broadcast_to_tracking(
                entity_id,
                ClientboundPlayPacket::EntityEvent(bbb_protocol::packets::EntityEvent {
                    entity_id,
                    event_id: CREAKING_SPAWN_ANIMATION_EVENT_ID,
                }),
            );
        // `:211`: `spawnedCreaking.setTransient(pos)` binds the entity to
        // the heart before the heart stores its UUID.
        let _ = self
            .level_mut_by_id(dimension)
            .entities
            .get_mut(entity_id)
            .is_some_and(|entity| entity.set_creaking_transient(pos));
        // `:212` binds the heart to the fresh protector.
        if let Some(uuid) = uuid {
            let changed = self
                .level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(BlockEntity::creaking_heart_mut)
                .map(|heart| {
                    heart.bind_protector(uuid);
                    true
                });
            if changed.is_some() {
                self.level_mut_by_id(dimension)
                    .chunks
                    .block_entities_mut()
                    .set_changed(pos);
            }
            // `:212`: `creaking.makeSound` is a positional `Level.playSound`
            // at the entity, not `playSoundFromEntity`. `:201-203` already
            // sent the add packet and `:210` the spawn animation, both of
            // which the store's journal holds ahead of this call, so the
            // sound leaves after them the way vanilla's statement order
            // does.
            if let Some(position) = self
                .level_by_id(dimension)
                .entities
                .get(entity_id)
                .map(crate::entity::Entity::position)
            {
                let seed = self.random.next_long();
                self.level_mut_by_id(dimension)
                    .entities
                    .play_entity_sound_with_seed(
                        PendingEntitySound {
                            position,
                            resource_id: "minecraft:entity.creaking.spawn".to_owned(),
                            source: SoundSource::Hostile,
                            volume: 1.0,
                            pitch: 1.0,
                        },
                        seed,
                    );
            }
            self.with_level(dimension, |level| {
                level.play_block_sound_with_volume_pitch(
                    pos,
                    "minecraft:block.creaking_heart.spawn",
                    1.0,
                    1.0,
                );
            });
        }
    }

    /// `Creaking.tick`'s binding check (`Creaking.java:243-256`): a
    /// heart-bound creaking whose heart no longer names it as protector
    /// dies. Vanilla runs it at the head of that entity's tick; this runs
    /// once for the level before the tick loop.
    pub(super) fn enforce_creaking_heart_bindings(&mut self, dimension: &LevelId) {
        let mut bound = Vec::new();
        for entity in self.level_by_id(dimension).entities.bodies() {
            if let Some(home_pos) = entity.creaking_home_pos() {
                bound.push((entity.id(), entity.uuid(), home_pos));
            }
        }
        for (entity_id, uuid, home_pos) in bound {
            let protected = self
                .level_by_id(dimension)
                .chunks
                .block_entities()
                .get(home_pos)
                .and_then(BlockEntity::creaking_heart)
                .and_then(|heart| heart.protector_uuid())
                .is_some_and(|protector| protector == uuid);
            if protected {
                continue;
            }
            if let Some(entity) = self
                .level_mut_by_id(dimension)
                .entities
                .get_mut(entity_id)
                .and_then(crate::entity::Entity::living_mut)
            {
                entity.set_health(0.0);
            }
        }
    }

    /// `CreakingHeartBlockEntity.clearCreakingInfo` (`:152-155`) on the release
    /// arm, which keeps the heart itself.
    fn clear_creaking_protector_binding(&mut self, dimension: &LevelId, pos: BlockPos) {
        let cleared = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::creaking_heart_mut)
            .map(|heart| heart.clear_protector());
        if cleared.is_some() {
            self.level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .set_changed(pos);
        }
    }

    /// `CreakingHeartBlockEntity.emitParticles` (`:284-304`) for one particle.
    fn emit_creaking_trail_particles(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        protector: Option<crate::block_entity::creaking_heart::CreakingHeartProtectorState>,
        towards_creaking: bool,
    ) {
        let Some(protector) = protector else {
            return;
        };
        let bounds = protector.bounding_box;
        let random = &self.random;
        let mut source = Vec3d {
            x: bounds.min_x + random.next_double() * (bounds.max_x - bounds.min_x),
            y: bounds.min_y + random.next_double() * (bounds.max_y - bounds.min_y),
            z: bounds.min_z + random.next_double() * (bounds.max_z - bounds.min_z),
        };
        let mut destination = Vec3d {
            x: f64::from(pos.x) + random.next_double(),
            y: f64::from(pos.y) + random.next_double(),
            z: f64::from(pos.z) + random.next_double(),
        };
        if towards_creaking {
            std::mem::swap(&mut source, &mut destination);
        }
        let duration = random.next_int(40) + 10;
        // `:287-288`: the colour follows the direction.
        let color = if towards_creaking {
            16_545_810
        } else {
            6_250_335
        };
        self.level_mut_by_id(dimension).entities.send_particles(
            bbb_protocol::packets::LevelParticles {
                // `sendParticles(option, true, true, ...)` (`:302`).
                override_limiter: true,
                always_show: true,
                position: source,
                offset: Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max_speed: 0.0,
                count: 1,
                particle: bbb_protocol::packets::ParticlePayload {
                    particle_type_id: VANILLA_PARTICLE_TYPE_TRAIL_ID,
                    raw_options: encode_trail_particle_options(
                        destination.x,
                        destination.y,
                        destination.z,
                        color,
                        duration,
                    ),
                },
            },
        );
    }

    /// `Creaking.playerIsStuckInYou` (`Creaking.java:281-284`).
    /// `CreakingHeartBlockEntity.creakingHurt` (`:225-243`), which the
    /// protector's own damage path calls with the player that hurt it: the
    /// first hit inside the window starts the emitter, sprays twenty trail
    /// particles and, while the heart is AWAKE, spreads resin onto up to
    /// three pale oak logs.
    pub(super) fn creaking_heart_creaking_hurt(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        protector_center: Vec3d,
    ) {
        let started = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::creaking_heart_mut)
            .is_some_and(|heart| heart.creaking_hurt_start(protector_center));
        if !started {
            return;
        }
        self.level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .set_changed(pos);
        let protector = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::creaking_heart)
            .and_then(|heart| heart.protector_uuid())
            .and_then(|uuid| {
                let entities = &self.level_by_id(dimension).entities;
                let entity = entities.get(entities.entity_id_for_uuid(uuid)?)?;
                Some(
                    crate::block_entity::creaking_heart::CreakingHeartProtectorState {
                        position: entity.position(),
                        bounding_box: entity.bounding_box(),
                        persistence_required: entity.is_persistence_required(),
                        player_stuck: false,
                    },
                )
            });
        for _ in 0..20 {
            self.emit_creaking_trail_particles(dimension, pos, protector, false);
        }
        let awake = self
            .block_state_by_key(dimension, pos)
            .and_then(bbb_protocol::block_state_types::vanilla_block_state_identity)
            .and_then(|identity| identity.properties.get("creaking_heart_state").cloned())
            .is_some_and(|state| state == "awake");
        if !awake {
            return;
        }
        let clumps = self.random.next_int(2) + 2;
        for _ in 0..clumps {
            let Some(placed) = self.spread_creaking_resin(dimension, pos) else {
                continue;
            };
            // `this.level.playSound(null, blockPos, RESIN_PLACE, BLOCKS, 1.0F,
            // 1.0F)` (`:234`): the seed is drawn where the call sits, and the
            // packet waits on the store's ordered journal so the twenty
            // bursts `:228` raised stay ahead of every place sound.
            let seed = self.random.next_long();
            self.level_mut_by_id(dimension)
                .entities
                .play_entity_sound_with_seed(
                    PendingEntitySound {
                        position: Vec3d {
                            x: f64::from(placed.x) + 0.5,
                            y: f64::from(placed.y) + 0.5,
                            z: f64::from(placed.z) + 0.5,
                        },
                        resource_id: "minecraft:block.resin.place".to_owned(),
                        source: SoundSource::Blocks,
                        volume: 1.0,
                        pitch: 1.0,
                    },
                    seed,
                );
            self.level_mut_by_id(dimension).event_sink.game_event(
                VanillaGameEvent::BLOCK_PLACE,
                placed,
                None,
            );
        }
    }

    /// `Creaking.hurtServer` (`Creaking.java:168-174`): notify the bound heart
    /// when a player is responsible and emit the heart-bound sway sound.
    pub(in crate::world) fn notify_creaking_heart_hurt(
        &mut self,
        dimension: &LevelId,
        home_pos: BlockPos,
        protector_uuid: uuid::Uuid,
        entity_id: i32,
        center: Vec3d,
        responsible_player: bool,
    ) {
        let protected = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(home_pos)
            .and_then(BlockEntity::creaking_heart)
            .and_then(|heart| heart.protector_uuid())
            .is_some_and(|protector| protector == protector_uuid);
        if protected {
            if responsible_player {
                self.creaking_heart_creaking_hurt(dimension, home_pos, center);
            }
            let seed = self.random.next_long();
            let position = self
                .level_by_id(dimension)
                .entities
                .get(entity_id)
                .map_or(center, crate::entity::Entity::position);
            self.level_mut_by_id(dimension)
                .entities
                .play_entity_sound_with_seed(
                    PendingEntitySound {
                        position,
                        resource_id: "minecraft:entity.creaking.sway".to_owned(),
                        source: SoundSource::Hostile,
                        volume: 1.0,
                        pitch: 1.0,
                    },
                    seed,
                );
        }
    }

    /// `BlockState.is(BlockTags.PALE_OAK_LOGS)` for `spreadResin`
    /// (`CreakingHeartBlockEntity.java:255`, `:258`).
    fn is_pale_oak_log_state(&self, dimension: &LevelId, state: Option<i32>) -> bool {
        state.is_some_and(|state| {
            self.level_by_id(dimension)
                .chunks
                .block_tags()
                .block_state_in("minecraft:pale_oak_logs", state)
        })
    }

    /// `CreakingHeartBlockEntity.spreadResin` (`:245-278`) as one
    /// `BlockPos.breadthFirstTraversal` (`BlockPos.java:465-501`) of the
    /// heart through pale oak logs, two deep and sixty-four nodes wide, that
    /// attaches one resin clump to the first free log face it finds.
    fn spread_creaking_resin(&mut self, dimension: &LevelId, start: BlockPos) -> Option<BlockPos> {
        let resin = vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_RESIN_CLUMP_ID,
        )?;
        let waterlogged = with_property(resin, "waterlogged", "true")?;
        let mut queue: std::collections::VecDeque<(BlockPos, i32)> =
            std::collections::VecDeque::new();
        let mut visited = std::collections::BTreeSet::new();
        queue.push_back((start, 0));
        let mut count = 0;
        while let Some((pos, depth)) = queue.pop_front() {
            if !visited.insert((pos.x, pos.y, pos.z)) {
                continue;
            }
            let state = self.block_state_by_key(dimension, pos);
            if self.is_pale_oak_log_state(dimension, state) {
                for direction in self.shuffled_directions() {
                    let neighbour = relative(pos, direction);
                    let opposite = opposite(direction);
                    let Some(state) = self.block_state_by_key(dimension, neighbour) else {
                        continue;
                    };
                    // `:262-270`: air becomes a dry clump, a water source a
                    // waterlogged one, and only an existing clump is extended.
                    let candidate = if block_state_is_air(state) {
                        resin
                    } else if state_is_block(
                        state,
                        bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID,
                    ) && block_state_fluid_state(state).is_source()
                    {
                        waterlogged
                    } else {
                        state
                    };
                    if !state_is_block(
                        candidate,
                        bbb_protocol::block_types::VANILLA_BLOCK_RESIN_CLUMP_ID,
                    ) || multiface::has_face(candidate, opposite)
                    {
                        continue;
                    }
                    let Some(next) = multiface::with_face(candidate, opposite) else {
                        continue;
                    };
                    // `setBlock(..., 3)` (`:273`).
                    self.set_block(dimension, neighbour, next, 3);
                    return Some(neighbour);
                }
            }
            count += 1;
            if count >= 64 {
                break;
            }
            if depth < 2 {
                for direction in self.shuffled_directions() {
                    let neighbour = relative(pos, direction);
                    let state = self.block_state_by_key(dimension, neighbour);
                    if self.is_pale_oak_log_state(dimension, state) {
                        queue.push_back((neighbour, depth + 1));
                    }
                }
            }
        }
        None
    }

    /// `Util.shuffledCopy(Direction.values(), random)`
    /// (`Util.java:1014-1031`), which `spreadResin` asks for once per node.
    fn shuffled_directions(&mut self) -> Vec<Direction> {
        const DIRECTIONS: [Direction; 6] = [
            Direction::Down,
            Direction::Up,
            Direction::North,
            Direction::South,
            Direction::West,
            Direction::East,
        ];
        let random = &self.random;
        let mut shuffled = DIRECTIONS.to_vec();
        for index in (2..=shuffled.len()).rev() {
            let swap_to = random.next_int(index as i32) as usize;
            shuffled.swap(index - 1, swap_to);
        }
        shuffled
    }

    /// `Creaking.playerIsStuckInYou` (`Creaking.java:357-375`): the entity
    /// owns the counter, this answers whether a player's eye is inside its
    /// box this tick. `NEAREST_PLAYERS` only ever holds this level's
    /// players, so the search is level-local like the 32-block one above.
    pub(super) fn creaking_player_eye_inside(
        &self,
        dimension: &LevelId,
        protector: &crate::entity::Entity,
    ) -> bool {
        let bounds = protector.bounding_box();
        self.levels.players().any(|player| {
            &player.dimension == dimension
                && !player.avatar_removed
                && player.game_type != GameType::Spectator
                && player.eye_position().is_ok_and(|eye| {
                    eye.x >= bounds.min_x
                        && eye.x < bounds.max_x
                        && eye.y >= bounds.min_y
                        && eye.y < bounds.max_y
                        && eye.z >= bounds.min_z
                        && eye.z < bounds.max_z
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(name: &str) -> i32 {
        vanilla_block_default_state_id(
            bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
                .unwrap()
                .id,
        )
        .unwrap()
    }

    /// `spreadResin` (`CreakingHeartBlockEntity.java:247-282`) walks two logs
    /// deep through `#pale_oak_logs`, so a clump lands on the second log when
    /// every face of the first is taken.
    #[test]
    fn resin_spreads_onto_a_second_tagged_log() {
        let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
        let heart = BlockPos { x: 2, y: 70, z: 2 };
        let first = relative(heart, Direction::Up);
        let second = relative(first, Direction::Up);
        world.set_block(dimension, heart, state("minecraft:stone"), 3);
        world.set_block(dimension, first, state("minecraft:pale_oak_log"), 3);
        world.set_block(
            dimension,
            second,
            state("minecraft:stripped_pale_oak_wood"),
            3,
        );
        for direction in [
            Direction::North,
            Direction::South,
            Direction::West,
            Direction::East,
        ] {
            world.set_block(
                dimension,
                relative(first, direction),
                state("minecraft:stone"),
                3,
            );
        }

        let placed = world
            .spread_creaking_resin(&dimension, heart)
            .expect("a clump is placed");

        assert_ne!(placed, first);
        assert!([
            Direction::Up,
            Direction::North,
            Direction::South,
            Direction::West,
            Direction::East
        ]
        .iter()
        .any(|&direction| relative(second, direction) == placed));
        assert!(state_is_block(
            world.block_state_by_key(&dimension, placed).unwrap(),
            bbb_protocol::block_types::VANILLA_BLOCK_RESIN_CLUMP_ID
        ));
    }
}
