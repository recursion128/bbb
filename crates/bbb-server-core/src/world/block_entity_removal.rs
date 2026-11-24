//! The `preRemoveSideEffects` overrides that reach past the item queue:
//! `SculkShriekerBlockEntity.tryRespond`
//! (`SculkShriekerBlockEntity.java:140-148`) and
//! `CreakingHeartBlockEntity.removeProtector(null)`
//! (`CreakingHeartBlockEntity.java:311-321`).

use crate::entity::{EntityData, GeneralLivingData, PendingEntitySound, RemovalReason};
use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::codec::Encoder;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_WARDEN_ID,
};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{
    LevelParticles, ParticlePayload, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_CRUMBLE_ID;
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_server_types::geometry::{static_block_collision_box_iter, Aabb};
use uuid::Uuid;

use super::natural_spawn::box_contains_liquid;
use super::simulation_level::pending_spawns::PendingBlockEntityRemovalEffect;
use super::*;

/// The `SpawnUtil.Strategy` arms this server reaches (`SpawnUtil.java:79-105`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SpawnStrategy {
    /// `ON_TOP_OF_COLLIDER` (`:95-96`), the shrieker's warden.
    OnTopOfCollider,
    /// `ON_TOP_OF_COLLIDER_NO_LEAVES` (`:97-105`), the creaking heart's
    /// protector.
    OnTopOfColliderNoLeaves,
    /// `LEGACY_IRON_GOLEM` (`:79-95`), the villagers' golem.
    LegacyIronGolem,
}

#[derive(Clone, Copy)]
pub(super) struct MobSpawnRequest {
    pub(super) entity_type_id: i32,
    pub(super) start: BlockPos,
    pub(super) attempts: i32,
    pub(super) range_xz: i32,
    pub(super) range_y: i32,
    pub(super) strategy: SpawnStrategy,
    pub(super) check_collisions: bool,
    pub(super) reason: crate::entity::SpawnReason,
}

/// `SculkShriekerBlockEntity.WARNING_SOUND_RADIUS` (`:43`).
const WARNING_SOUND_RADIUS: i32 = 10;
/// `SculkShriekerBlockEntity.WARDEN_SPAWN_ATTEMPTS` (`:44`).
const WARDEN_SPAWN_ATTEMPTS: i32 = 20;
/// `SculkShriekerBlockEntity.WARDEN_SPAWN_RANGE_XZ` (`:45`).
const WARDEN_SPAWN_RANGE_XZ: i32 = 5;
/// `SculkShriekerBlockEntity.WARDEN_SPAWN_RANGE_Y` (`:46`).
const WARDEN_SPAWN_RANGE_Y: i32 = 6;
/// `SculkShriekerBlockEntity.DARKNESS_RADIUS` (`:47`).
const DARKNESS_RADIUS: f64 = 40.0;
/// `Creaking.tearDown`'s `BlockParticleOption(BLOCK_CRUMBLE, ...)` spread
/// (`Creaking.java:293-303`, `:304-315`).
const TEAR_DOWN_PARTICLES: i32 = 100;
const TEAR_DOWN_HEART_PARTICLES: i32 = 10;

impl ServerWorld {
    /// Runs the effects the removal path queued, in the order they were
    /// raised. Vanilla runs them inside `LevelChunk.setBlockState`; this
    /// server reaches them at the tick's own drain instead, which is the
    /// earliest point that holds the entity store and the session list
    /// together.
    pub(super) fn drain_block_entity_removal_effects(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let mut mine = Vec::new();
        let mut rest = Vec::new();
        for (key, effect) in std::mem::take(&mut self.pending_removal_effects) {
            if &key == dimension {
                mine.push(effect);
            } else {
                rest.push((key, effect));
            }
        }
        self.pending_removal_effects = rest;
        for effect in mine {
            match effect {
                PendingBlockEntityRemovalEffect::SculkShrieker {
                    position,
                    warning_level,
                    can_summon,
                } => self.sculk_shrieker_try_respond(
                    dimension,
                    position,
                    warning_level,
                    can_summon,
                    events,
                ),
                PendingBlockEntityRemovalEffect::CreakingProtector { uuid } => {
                    self.tear_down_creaking_protector(dimension, uuid);
                }
                PendingBlockEntityRemovalEffect::FurnaceExperience {
                    position,
                    recipes_used,
                } => {
                    self.pop_removed_furnace_experience(*dimension, position, &recipes_used);
                }
            }
        }
    }

    /// `SculkShriekerBlockEntity.tryRespond` (`:140-148`).
    pub(super) fn sculk_shrieker_try_respond(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
        warning_level: i32,
        can_summon: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        // `canRespond` (`:127-131`).
        if !can_summon
            || self.admin.settings().difficulty == Difficulty::Peaceful
            || !self.game_rules.spawn_wardens
            || warning_level <= 0
        {
            return;
        }
        if !self.try_summon_warden(dimension, position, warning_level) {
            self.play_warden_reply_sound(dimension, position, warning_level, events);
        }
        self.apply_darkness_around(
            dimension,
            Vec3d {
                x: f64::from(position.x) + 0.5,
                y: f64::from(position.y) + 0.5,
                z: f64::from(position.z) + 0.5,
            },
            DARKNESS_RADIUS,
            None,
            events,
        );
    }

    /// `SculkShriekerBlockEntity.trySummonWarden` (`:167-175`) over
    /// `SpawnUtil.trySpawnMob` (`SpawnUtil.java:18-49`) with
    /// `Strategy.ON_TOP_OF_COLLIDER` (`:95-96`).
    fn try_summon_warden(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
        warning_level: i32,
    ) -> bool {
        // `:167-169`: only a fully warned shrieker summons.
        if warning_level < 4 {
            return false;
        }
        self.try_spawn_mob(
            dimension,
            MobSpawnRequest {
                entity_type_id: VANILLA_ENTITY_TYPE_WARDEN_ID,
                start: position,
                attempts: WARDEN_SPAWN_ATTEMPTS,
                range_xz: WARDEN_SPAWN_RANGE_XZ,
                range_y: WARDEN_SPAWN_RANGE_Y,
                strategy: SpawnStrategy::OnTopOfCollider,
                check_collisions: false,
                reason: crate::entity::SpawnReason::Triggered,
            },
        )
        .is_some()
    }

    /// `SculkShriekerBlockEntity.playWardenReplySound` (`:150-159`).
    fn play_warden_reply_sound(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
        warning_level: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        // `SOUND_BY_LEVEL` (`:48-53`).
        let Some(resource_id) = [
            "minecraft:entity.warden.nearby_close",
            "minecraft:entity.warden.nearby_closer",
            "minecraft:entity.warden.nearby_closest",
            "minecraft:entity.warden.listening_angry",
        ]
        .get(warning_level as usize - 1)
        .copied() else {
            return;
        };
        let Some(registry_id) = vanilla_sound_event_registry_id(resource_id) else {
            return;
        };
        let random = &self.random;
        let x = position.x
            + random_between_inclusive(random, -WARNING_SOUND_RADIUS, WARNING_SOUND_RADIUS);
        let y = position.y
            + random_between_inclusive(random, -WARNING_SOUND_RADIUS, WARNING_SOUND_RADIUS);
        let z = position.z
            + random_between_inclusive(random, -WARNING_SOUND_RADIUS, WARNING_SOUND_RADIUS);
        let packet = SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            source: SoundSource::Hostile,
            position: Vec3d {
                x: f64::from(x),
                y: f64::from(y),
                z: f64::from(z),
            },
            volume: 5.0,
            pitch: 1.0,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(dimension, &packet, None, events);
    }

    /// `CreakingHeartBlockEntity.removeProtector(null)` (`:311-321`) over
    /// `Creaking.tearDown` (`Creaking.java:286-323`).
    ///
    /// Every packet the three statements produce is raised on the store's
    /// ordered journal, so the sound seed is drawn here and the packets leave at
    /// the journal's next drain rather than into `events` at the call.
    pub(super) fn tear_down_creaking_protector(&mut self, dimension: &LevelId, uuid: Uuid) {
        let Some((entity_id, bounds, position)) = self
            .level_by_id(dimension)
            .entities
            .entity_id_for_uuid(uuid)
            .and_then(|entity_id| {
                let protector = self.level_by_id(dimension).entities.get(entity_id)?;
                (protector.entity_type_id() == VANILLA_ENTITY_TYPE_CREAKING_ID
                    && protector.is_alive())
                .then(|| (entity_id, protector.bounding_box(), protector.position()))
            })
        else {
            return;
        };
        let center = bounds.center();
        let (x_spread, y_spread, z_spread) = (
            (bounds.max_x - bounds.min_x) * 0.3,
            (bounds.max_y - bounds.min_y) * 0.3,
            (bounds.max_z - bounds.min_z) * 0.3,
        );
        let wood = vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_PALE_OAK_WOOD_ID,
        );
        let heart = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:creaking_heart",
            &std::collections::BTreeMap::from([(
                "creaking_heart_state".to_string(),
                "awake".to_string(),
            )]),
        )
        .or_else(|| {
            vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_CREAKING_HEART_ID,
            )
        });
        for (state, count) in [
            (wood, TEAR_DOWN_PARTICLES),
            (heart, TEAR_DOWN_HEART_PARTICLES),
        ] {
            let Some(state) = state else {
                continue;
            };
            self.level_mut_by_id(dimension)
                .entities
                .send_particles(LevelParticles {
                    override_limiter: false,
                    always_show: false,
                    position: center,
                    offset: Vec3d {
                        x: x_spread,
                        y: y_spread,
                        z: z_spread,
                    },
                    max_speed: 0.0,
                    count,
                    particle: block_crumble_particle(state),
                });
        }
        // `this.makeSound(this.getDeathSound())` (`Creaking.java:319`). The
        // store's ordered journal puts the packet on the wire between the two
        // bursts `:293`/`:304` raised and the `remove` `:320` below, which is the
        // order those statements stand in.
        let seed = self.random.next_long();
        self.level_mut_by_id(dimension)
            .entities
            .play_entity_sound_with_seed(
                PendingEntitySound {
                    position,
                    resource_id: "minecraft:entity.creaking.death".to_owned(),
                    source: SoundSource::Hostile,
                    volume: 1.0,
                    pitch: 1.0,
                },
                seed,
            );
        self.level_mut_by_id(dimension)
            .entities
            .remove(entity_id, RemovalReason::Discarded);
    }

    /// `SpawnUtil.trySpawnMob` (`SpawnUtil.java:18-49`) for the warden arm:
    /// `EntityType.WARDEN`'s `SpawnPlacements` row is `NO_RESTRICTIONS` with
    /// `Mob.checkSpawnRules` answering true (`SpawnPlacements.java:177`,
    /// `Mob.java:783-785`), so only the position walk and
    /// `checkSpawnObstruction` (`Mob.java:787-789`) decide.
    pub(super) fn try_spawn_mob(
        &mut self,
        dimension: &LevelId,
        request: MobSpawnRequest,
    ) -> Option<i32> {
        let MobSpawnRequest {
            entity_type_id,
            start,
            attempts,
            range_xz,
            range_y,
            strategy,
            check_collisions,
            reason,
        } = request;
        let [width, height] =
            bbb_protocol::entity_types::vanilla_entity_type_dimensions(entity_type_id)?;
        let mut search;
        for _ in 0..attempts {
            {
                let random = &self.random;
                search = BlockPos {
                    x: start.x + random_between_inclusive(random, -range_xz, range_xz),
                    y: start.y + range_y,
                    z: start.z + random_between_inclusive(random, -range_xz, range_xz),
                };
            }
            if !self
                .level_by_id(dimension)
                .world_border
                .contains_block_at(search, self.game_time())
            {
                continue;
            }
            if !self.move_to_possible_spawn_position(dimension, &mut search, range_y, strategy) {
                continue;
            }
            let spawn_position = Vec3d {
                x: f64::from(search.x) + 0.5,
                y: f64::from(search.y),
                z: f64::from(search.z) + 0.5,
            };
            let bounds = Aabb::of_entity(spawn_position, width, height);
            // `SpawnUtil.java:36-38`: `checkCollisions` is
            // `level.noCollision(spawnAABB)`, the block half only.
            if check_collisions && !self.no_block_collision(dimension, bounds) {
                continue;
            }
            if !self.spawn_obstruction_clear(dimension, entity_type_id, search, bounds) {
                continue;
            }
            let data = GeneralLivingData::new(entity_type_id)?;
            let spawned = self.level_mut_by_id(dimension).entities.spawn(
                entity_type_id,
                spawn_position,
                EntityData::GeneralLiving(data),
            );
            // `:44-46`'s ambient sound needs the entity sound table this server
            // does not carry.
            self.finalize_added_mob(dimension, spawned, reason);
            return Some(spawned);
        }
        None
    }

    /// `Level.noCollision(AABB)` (`SpawnUtil.java:36-38`).
    fn no_block_collision(&self, dimension: &LevelId, bounds: Aabb) -> bool {
        super::movement_collision::block_collision_boxes(
            &self.level_by_id(dimension).chunks,
            bounds,
        )
        .is_empty()
    }

    /// `SpawnUtil.moveToPossibleSpawnPosition` (`SpawnUtil.java:51-69`) with
    /// `Strategy.ON_TOP_OF_COLLIDER` (`:95-96`): the cell above is empty and
    /// the cell itself carries a full top face.
    fn move_to_possible_spawn_position(
        &self,
        dimension: &LevelId,
        search: &mut BlockPos,
        range_y: i32,
        strategy: SpawnStrategy,
    ) -> bool {
        let level = self.level_by_id(dimension);
        let mut above_state = level.chunks.get_block_state(search.x, search.y, search.z);
        for _ in 0..=range_y * 2 {
            search.y -= 1;
            let Some(current_state) = level.chunks.get_block_state(search.x, search.y, search.z)
            else {
                return false;
            };
            let on_top = match strategy {
                SpawnStrategy::LegacyIronGolem => {
                    legacy_iron_golem_floor(above_state, current_state)
                }
                _ => {
                    above_is_empty(above_state)
                        && is_full_top_face(Some(current_state))
                        && (strategy == SpawnStrategy::OnTopOfCollider
                            || !level
                                .chunks
                                .block_tags()
                                .block_state_in("minecraft:leaves", current_state))
                }
            };
            if on_top {
                search.y += 1;
                return true;
            }
            above_state = Some(current_state);
        }
        false
    }

    /// `Mob.checkSpawnObstruction` (`Mob.java:787-789`), with the iron
    /// golem's own block rule in place of the liquid test.
    fn spawn_obstruction_clear(
        &self,
        dimension: &LevelId,
        entity_type_id: i32,
        pos: BlockPos,
        bounds: Aabb,
    ) -> bool {
        let level = self.level_by_id(dimension);
        let blocks_clear = if entity_type_id == VANILLA_ENTITY_TYPE_IRON_GOLEM_ID {
            super::spawner_minecarts::iron_golem_blocks_clear(&level.chunks, pos, entity_type_id)
        } else {
            !box_contains_liquid(&level.chunks, bounds)
        };
        if !blocks_clear {
            return false;
        }
        if level
            .entities
            .ids_in_box_sections(bounds)
            .into_iter()
            .any(|id| {
                level.entities.get(id).is_some_and(|entity| {
                    entity.is_alive()
                        && entity.can_be_collided_with()
                        && entity.bounding_box().intersects(bounds)
                })
            })
        {
            return false;
        }
        !self.levels.players().any(|player| {
            &player.dimension == dimension
                && player.game_type != GameType::Spectator
                && player
                    .bounding_box()
                    .is_ok_and(|box_| box_.intersects(bounds))
        })
    }
}

/// `Mth.randomBetweenInclusive` (`Mth.java:664-666`).
fn random_between_inclusive(random: &crate::random::GameRandom, min: i32, max: i32) -> i32 {
    random.next_int(max - min + 1) + min
}

/// `SpawnUtil.Strategy.LEGACY_IRON_GOLEM` (`SpawnUtil.java:79-95`): a solid
/// floor that is none of the listed see-through or light blocks, under air or
/// a liquid block. `isSolid` is read as a full top face.
fn legacy_iron_golem_floor_excluded_table() -> &'static Vec<bool> {
    static EXCLUDED: std::sync::OnceLock<Vec<bool>> = std::sync::OnceLock::new();
    EXCLUDED.get_or_init(|| {
        let mut flags = vec![false; bbb_protocol::block_types::VANILLA_BLOCK_TYPES.len()];
        for entry in bbb_protocol::block_types::VANILLA_BLOCK_TYPES {
            let path = entry.resource_id.trim_start_matches("minecraft:");
            let is_excluded = matches!(
                entry.id,
                bbb_protocol::block_types::VANILLA_BLOCK_COBWEB_ID
                    | bbb_protocol::block_types::VANILLA_BLOCK_CACTUS_ID
                    | bbb_protocol::block_types::VANILLA_BLOCK_GLASS_PANE_ID
                    | bbb_protocol::block_types::VANILLA_BLOCK_CONDUIT_ID
                    | bbb_protocol::block_types::VANILLA_BLOCK_ICE_ID
                    | bbb_protocol::block_types::VANILLA_BLOCK_TNT_ID
                    | bbb_protocol::block_types::VANILLA_BLOCK_GLOWSTONE_ID
                    | bbb_protocol::block_types::VANILLA_BLOCK_BEACON_ID
                    | bbb_protocol::block_types::VANILLA_BLOCK_SEA_LANTERN_ID
                    | bbb_protocol::block_types::VANILLA_BLOCK_FROSTED_ICE_ID
                    | bbb_protocol::block_types::VANILLA_BLOCK_TINTED_GLASS_ID
                    | bbb_protocol::block_types::VANILLA_BLOCK_GLASS_ID
            ) || path.ends_with("_stained_glass")
                || path.ends_with("_stained_glass_pane");
            flags[entry.id as usize] = is_excluded;
        }
        flags
    })
}

fn legacy_iron_golem_floor(above: Option<i32>, floor: i32) -> bool {
    let Some(floor_block_id) =
        bbb_protocol::block_state_types::vanilla_block_state_block_type_id(floor)
    else {
        return false;
    };
    let excluded = legacy_iron_golem_floor_excluded_table()
        .get(floor_block_id as usize)
        .copied()
        .unwrap_or(false)
        || bbb_server_types::block_queries::is_leaves(floor);
    let above_open = above.is_some_and(|above| {
        bbb_server_types::fluid::block_state_is_air(above)
            || bbb_protocol::block_state_types::vanilla_block_state_block_type_id(above)
                .is_some_and(|block_id| {
                    matches!(
                        block_id,
                        bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID
                            | bbb_protocol::block_types::VANILLA_BLOCK_LAVA_ID
                    )
                })
    });
    !excluded
        && above_open
        && (is_full_top_face(Some(floor))
            || floor_block_id == bbb_protocol::block_types::VANILLA_BLOCK_POWDER_SNOW_ID)
}

/// `Shapes.empty()`-equivalent test for the cell the walk just left.
fn above_is_empty(state: Option<i32>) -> bool {
    state.is_some_and(|state| {
        static_block_collision_box_iter(state).is_some_and(|boxes| boxes.len() == 0)
    })
}

/// `Block.isFaceFull(shape, Direction.UP)` (`Block.java:350-353`).
fn is_full_top_face(state: Option<i32>) -> bool {
    state.is_some_and(|state| {
        static_block_collision_box_iter(state).is_some_and(|mut boxes| {
            boxes.any(|box_| {
                box_.min_x <= 0.0
                    && box_.min_z <= 0.0
                    && box_.max_x >= 1.0
                    && box_.max_z >= 1.0
                    && (box_.max_y - 1.0).abs() < 1.0e-9
            })
        })
    })
}

/// `BlockParticleOption(ParticleTypes.BLOCK_CRUMBLE, state)`'s stream codec.
fn block_crumble_particle(state: i32) -> ParticlePayload {
    let mut encoder = Encoder::default();
    encoder.write_var_i32(state);
    ParticlePayload {
        particle_type_id: VANILLA_PARTICLE_TYPE_BLOCK_CRUMBLE_ID,
        raw_options: encoder.into_inner(),
    }
}

#[cfg(test)]
mod tests;
