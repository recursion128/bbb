//! Saved hive occupant residence, release and honey delivery.

use crate::block_behavior::beehive::is_beehive;
use crate::entity::save::create_beehive_occupant_with_context;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID;
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_server_types::directions::relative;
use bbb_server_types::geometry::Vec3Ops;

use super::{LevelId, ServerWorld, WorldEvent};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BeehiveRelease {
    Emergency,
    Harvest,
    Timed,
}

fn facing(state: i32) -> Option<Direction> {
    match vanilla_block_state_identity(state)?
        .properties
        .get("facing")?
        .as_str()
    {
        "north" => Some(Direction::North),
        "south" => Some(Direction::South),
        "west" => Some(Direction::West),
        "east" => Some(Direction::East),
        _ => None,
    }
}

fn exit_position(
    pos: BlockPos,
    facing: Direction,
    blocked: bool,
    width: f32,
    height: f32,
) -> Vec3d {
    let delta = if blocked {
        0.0
    } else {
        0.55 + f64::from(width / 2.0)
    };
    let (dx, dz) = match facing {
        Direction::North => (0.0, -1.0),
        Direction::South => (0.0, 1.0),
        Direction::West => (-1.0, 0.0),
        Direction::East => (1.0, 0.0),
        Direction::Down | Direction::Up => unreachable!("hive facing is horizontal"),
    };
    Vec3d {
        x: f64::from(pos.x) + 0.5 + delta * dx,
        y: f64::from(pos.y) + 0.5 - f64::from(height / 2.0),
        z: f64::from(pos.z) + 0.5 + delta * dz,
    }
}

impl ServerWorld {
    /// `BeehiveBlockEntity.serverTick` (`BeehiveBlockEntity.java:284-292`).
    pub(super) fn tick_beehive(&mut self, dimension: &LevelId, pos: BlockPos) {
        let Some(hive) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(crate::block_entity::BlockEntity::beehive_mut)
        else {
            return;
        };
        if hive.is_empty() {
            return;
        }
        let ready = hive.server_tick();
        if ready {
            let state = self.level_by_id(dimension).block_state(pos);
            self.release_beehive_occupants(*dimension, pos, state, BeehiveRelease::Timed);
        }
        let level = self.level_mut_by_id(dimension);
        if level
            .chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::beehive)
            .is_some_and(|hive| !hive.is_empty())
            && level.random.next_double() < 0.005
        {
            level
                .event_sink
                .play_block_sound(pos, "minecraft:block.beehive.work");
        }
    }

    /// `BeehiveBlock.getDrops` (`:309-323`) enters this only for its five
    /// direct `THIS_ENTITY` classes, before the base loot table is drawn.
    pub(super) fn release_beehive_explosion_occupants(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        state: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        self.release_beehive_emergency_occupants(dimension, pos, state, None, false, events);
    }

    pub(super) fn set_beehive_player_target(
        &mut self,
        dimension: impl Into<LevelId>,
        bee: i32,
        target: crate::entity::ai::mob::target::TargetRef,
    ) {
        let difficulty = self.admin.settings().difficulty;
        let game_time = self.game_time();
        self.with_level_and_entities(dimension, |level, entities| {
            // The first hive occupant can precede the world's first entity tick.
            entities.set_difficulty(difficulty);
            entities.set_game_time(game_time);
            crate::entity::ai::mob::target::sense::set_mob_target(
                entities,
                level,
                bee,
                Some(target),
            );
        });
    }

    pub(super) fn release_beehive_emergency_occupants(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        state: i32,
        player: Option<crate::connection::ConnectionId>,
        sedated: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        self.release_beehive_emergency_occupants_from(
            dimension, pos, state, player, sedated, None, events,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn release_beehive_emergency_occupants_from(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        state: i32,
        player: Option<crate::connection::ConnectionId>,
        sedated: bool,
        detached: Option<&mut crate::block_entity::beehive::BeehiveBlockEntity>,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let released = self.release_beehive_occupants_from(
            dimension,
            pos,
            state,
            BeehiveRelease::Emergency,
            detached,
        );
        self.flush_level_events(&dimension, events);
        self.apply_level_entity_lifecycle_by_key(&dimension, events);
        for entity_id in released {
            if let Some(player) = player.and_then(|id| self.levels.player(id)) {
                let bee = self
                    .level_by_id(&dimension)
                    .entities
                    .get(entity_id)
                    .expect("published occupant");
                // `BeehiveBlockEntity.emptyAllLivingFromHive` (`BeehiveBlockEntity.java:125-136`).
                if bee.entity_type_id() == VANILLA_ENTITY_TYPE_BEE_ID
                    && bee.position().distance_to_sqr(player.position()) <= 16.0
                {
                    if sedated {
                        if let Some(bee) =
                            self.level_mut_by_id(&dimension).entities.get_mut(entity_id)
                        {
                            crate::entity::bee::set_stay_out_of_hive_countdown(bee, 400);
                        }
                        continue;
                    }
                    let target = crate::entity::ai::mob::target::TargetRef {
                        entity_id: player.entity_id(),
                        uuid: player.uuid(),
                    };
                    self.set_beehive_player_target(dimension, entity_id, target);
                }
            }
        }
    }

    pub(super) fn release_beehive_occupants(
        &mut self,
        dimension: LevelId,
        pos: BlockPos,
        state: i32,
        reason: BeehiveRelease,
    ) -> Vec<i32> {
        self.release_beehive_occupants_from(dimension, pos, state, reason, None)
    }

    fn release_beehive_occupants_from(
        &mut self,
        dimension: LevelId,
        pos: BlockPos,
        state: i32,
        reason: BeehiveRelease,
        detached: Option<&mut crate::block_entity::beehive::BeehiveBlockEntity>,
    ) -> Vec<i32> {
        let mut released = Vec::new();
        let emergency = reason == BeehiveRelease::Emergency;
        // `BeehiveBlockEntity.releaseOccupant` (`BeehiveBlockEntity.java:201-211`).
        if !emergency && self.bees_stay_in_hive(&dimension, pos) != Some(false) {
            return released;
        }
        if !is_beehive(state) {
            return released;
        }
        let (is_empty, flower) = {
            let Some(hive) = detached.as_deref().or_else(|| {
                self.level_by_id(&dimension)
                    .chunks
                    .block_entities()
                    .get(pos)
                    .and_then(crate::block_entity::BlockEntity::beehive)
            }) else {
                return released;
            };
            (
                hive.is_empty(),
                match hive.saved_flower_pos() {
                    Ok(flower) => flower,
                    Err(error) => {
                        tracing::warn!("beehive release: {error}");
                        None
                    }
                },
            )
        };
        if is_empty {
            return released;
        }
        let Some(facing) = facing(state) else {
            tracing::warn!("beehive release: beehive release lacks a facing state");
            return released;
        };
        let front = relative(pos, facing);
        let front_state = {
            let chunks = &self.level_by_id(&dimension).chunks;
            if !super::level_blocks::is_in_world_bounds(chunks, front) {
                Some(chunks.air_state_id())
            } else {
                chunks
                    .get_block_state(front.x, front.y, front.z)
                    .or_else(|| chunks.generated_block_state(front.x, front.y, front.z))
            }
        };
        let Some(front_state) = front_state else {
            tracing::warn!("beehive release: beehive exit block state is unavailable");
            return released;
        };
        let front_blocked = super::movement_collision::block_state_collision_shape_checked(
            &self.level_by_id(&dimension).chunks,
            front_state,
            front,
            None,
        )
        .is_none_or(|shape| !shape.is_empty());
        if front_blocked && !emergency {
            return released;
        }
        let bee_count = if let Some(hive) = detached.as_deref() {
            hive.bees().len()
        } else {
            self.level_by_id(&dimension)
                .chunks
                .block_entities()
                .get(pos)
                .and_then(crate::block_entity::BlockEntity::beehive)
                .expect("hive was present")
                .bees()
                .len()
        };
        if bee_count == 0 {
            return released;
        }
        let tags = self
            .level_by_id(&dimension)
            .chunks
            .registry_tags_shared()
            .map_or(
                crate::entity::EntityRegistryTagView::Builtin26_1,
                crate::entity::EntityRegistryTagView::Frozen,
            );

        let input = self.entity_nbt_read_snapshot();
        let mut released_flags = vec![false; bee_count];
        let mut any_released = false;
        for (index, released_flag) in released_flags.iter_mut().enumerate() {
            let read = if let Some(hive) = detached.as_deref() {
                hive.release_entry_at(index)
            } else {
                self.level_by_id(&dimension)
                    .chunks
                    .block_entities()
                    .get(pos)
                    .and_then(crate::block_entity::BlockEntity::beehive)
                    .and_then(|hive| hive.release_entry_at(index))
            };
            let Some(read) = read else {
                break;
            };
            let occupant = match read {
                Ok(occupant) => occupant,
                Err(error) => {
                    tracing::warn!("beehive release: occupant {index}: {error}");
                    continue;
                }
            };
            if reason == BeehiveRelease::Timed
                && occupant.ticks_in_hive <= occupant.min_ticks_in_hive
            {
                continue;
            }
            let is_inhabitor = tags.contains(
                "minecraft:entity_type",
                "minecraft:beehive_inhabitors",
                &occupant.entity_resource_id,
            );
            let staged = {
                let level = self.level_mut_by_id(&dimension);
                create_beehive_occupant_with_context(
                    &mut level.entities,
                    &occupant,
                    pos,
                    flower,
                    &level.random,
                    is_inhabitor,
                    input.context(),
                )
            };
            let mut staged = match staged {
                Ok(Some(staged)) => staged,
                Ok(None) => {
                    tracing::warn!(
                        "beehive release: occupant {index}: {} is not a beehive inhabitant",
                        occupant.entity_resource_id
                    );
                    continue;
                }
                Err(error) => {
                    tracing::warn!("beehive release: occupant {index}: {error}");
                    continue;
                }
            };
            if staged.entity_type_id() == VANILLA_ENTITY_TYPE_BEE_ID {
                if reason == BeehiveRelease::Timed
                    && crate::entity::bee::drop_off_nectar(&mut staged)
                {
                    self.deliver_beehive_honey(dimension, pos);
                }
                let bee = &staged;
                let position =
                    exit_position(pos, facing, front_blocked, bee.bb_width(), bee.bb_height());
                staged.set_pos(position);
            }
            let entity_id = staged.id();
            let current_state = Some(self.level_by_id(&dimension).block_state(pos));
            self.level_mut_by_id(&dimension)
                .event_sink
                .play_block_sound(pos, "minecraft:block.beehive.exit");
            self.level_mut_by_id(&dimension)
                .event_sink
                .game_event_with_source(
                    VanillaGameEvent::BLOCK_CHANGE,
                    pos,
                    Some(entity_id),
                    current_state,
                );
            if let Some(team) = staged.take_loaded_team() {
                self.scoreboard
                    .add_player_to_team(&staged.uuid().to_string(), &team);
            }
            self.level_mut_by_id(&dimension).entities.add_fresh(staged);
            released.push(entity_id);
            *released_flag = true;
            any_released = true;
        }
        if any_released {
            if let Some(hive) = detached {
                hive.retain_unreleased(&released_flags);
            } else {
                let removed = self
                    .level_mut_by_id(&dimension)
                    .chunks
                    .block_entities_mut()
                    .get_mut(pos)
                    .and_then(crate::block_entity::BlockEntity::beehive_mut)
                    .is_some_and(|hive| hive.retain_unreleased(&released_flags));
                if removed {
                    self.level_mut_by_id(&dimension)
                        .chunks
                        .block_entities_mut()
                        .set_changed(pos);
                } else {
                    tracing::warn!("beehive release: published entities but hive record changed");
                }
            }
        }
        released
    }

    /// `BeehiveBlockEntity.releaseOccupant` (`BeehiveBlockEntity.java:221-232`).
    fn deliver_beehive_honey(&mut self, dimension: LevelId, pos: BlockPos) {
        self.with_level(dimension, |level| {
            let state = level.block_state(pos);
            let honey = crate::block_behavior::beehive::honey_level(state);
            let maximum = crate::block_behavior::beehive::MAX_HONEY_LEVELS;
            if honey >= maximum || !level.block_state_in_tag(state, "minecraft:beehives") {
                return;
            }
            let increase = if level.random.next_int(100) == 0 {
                2
            } else {
                1
            };
            if let Some(state) = crate::fluid::with_property(
                state,
                "honey_level",
                &(honey + increase).min(maximum).to_string(),
            ) {
                level.set_block(
                    pos,
                    state,
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                    crate::neighbor_updater::UPDATE_LIMIT,
                );
            }
        });
    }
}
