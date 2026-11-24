use crate::block_entity::piston::{
    piston_movement_area, piston_movement_to_clear, PistonMovingSnapshot, PistonTickResult,
};
use crate::world::simulation_level::LevelCtx;
use crate::world::LevelId;
use bbb_protocol::block_state_types::VanillaPistonPushReaction;
use bbb_server_types::geometry::{static_block_collision_boxes, Aabb};

use super::*;

impl ServerWorld {
    pub(super) fn tick_moving_piston(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(snapshot) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::piston_snapshot)
        else {
            return;
        };
        // `:320-334`: the entity half runs only while the move is still in
        // progress, and against the progress this tick is about to commit.
        if snapshot.progress < 1.0 {
            let new_progress = (snapshot.progress + 0.5).min(1.0);
            if !self.move_piston_entities(dimension, pos, &snapshot, new_progress, events) {
                return;
            }
        }
        let game_time = self.game_time();
        let result = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(|entity| entity.tick_piston_without_entities(game_time));
        let Some(PistonTickResult::Finalize {
            state,
            push_direction,
        }) = result
        else {
            return;
        };
        // `PistonMovingBlockEntity.java:314-316`: remove BE and guard on MOVING_PISTON.
        self.level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .remove(pos);
        if self
            .block_state_by_key(dimension, pos)
            .and_then(vanilla_block_state_block_type_id)
            != Some(bbb_protocol::block_types::VANILLA_BLOCK_MOVING_PISTON_ID)
        {
            return;
        }
        let new_state = self.with_level(dimension, |level| {
            update_from_neighbour_shapes(level, state, pos)
        });
        if bbb_server_types::fluid::block_state_is_air(new_state) {
            let _ = self.set_block(dimension, pos, state, 340);
            self.with_level(dimension, |level| {
                level.destroy_block(pos, true);
            });
            return;
        }
        let new_state =
            crate::fluid::with_property(new_state, "waterlogged", "false").unwrap_or(new_state);
        let _ = self.set_block(dimension, pos, new_state, 67);
        if let Some(block) = vanilla_block_state_block_type_id(new_state) {
            self.with_level(dimension, |level| {
                let orientation =
                    crate::block_behavior::redstone_wire::experimental::initial_orientation(
                        level,
                        Some(push_direction),
                        None,
                    );
                level.neighbor_changed_with_orientation(pos, block, orientation);
            });
        }
    }

    fn move_piston_entities(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        snapshot: &PistonMovingSnapshot,
        new_progress: f32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let movement = snapshot.movement_direction();
        let delta_progress = f64::from(new_progress) - f64::from(snapshot.progress);
        let shape_state = snapshot.collision_related_block_state(snapshot.progress);
        // `getCollisionShape(level, pos)` uses the empty context. Of the
        // dynamic-shape blocks only powder snow can be moved, and its
        // empty-context shape is empty (`PowderSnowBlock.java:116-131`).
        let shape = static_block_collision_boxes(shape_state).unwrap_or_default();
        let query = shape.first().map(|first| {
            let bounds = shape
                .iter()
                .fold(*first, |bounds, part| union(bounds, *part));
            let aabb = snapshot.move_by_position_and_progress(pos, bounds);
            union(piston_movement_area(aabb, movement, delta_progress), aabb)
        });
        let mut players = self.nearby_players_by_key(dimension);
        let mut leftover = PendingEntitySpawns::default();
        let mut cursor = PistonBodyCursor {
            references: Vec::new(),
            index: 0,
            phase: PistonBodyPhase::Collided,
            pending: None,
        };
        let block_interaction_policy = self.block_interaction_policy(dimension);
        let result = (|| {
            if let Some(bounds) = query {
                cursor.references = self.capture_piston_bodies(dimension, pos, bounds, false)?;
            }
            loop {
                let outcome = self.with_stored_entity_level(
                    dimension,
                    &block_interaction_policy,
                    None,
                    &players,
                    &mut leftover,
                    |entities, level, _| {
                        cursor.advance(entities, level, pos, snapshot, &shape, delta_progress)
                    },
                )?;
                match outcome {
                    PistonBodyOutcome::Paused => {
                        let pending = cursor.pending.as_mut()?;
                        let (id, uuid) = (pending.entity_id(), pending.uuid());
                        if self
                            .level_by_id(dimension)
                            .entities
                            .get(id)
                            .is_none_or(|body| body.uuid() != uuid)
                        {
                            continue;
                        }
                        self.flush_level_events(dimension, events);
                        self.apply_level_entity_lifecycle_by_key(dimension, events);
                        let succeeded = self.apply_projectile_hurt(
                            *dimension,
                            pending.hurt().clone(),
                            None,
                            events,
                        )?;
                        pending.complete_hurt(succeeded);
                        self.run_entity_commands(dimension, events)?;
                        players = self.nearby_players_by_key(dimension);
                    }
                    PistonBodyOutcome::Session(reference) => {
                        self.flush_level_events(dimension, events);
                        self.apply_level_entity_lifecycle_by_key(dimension, events);
                        cursor.advance_session(
                            self,
                            reference,
                            pos,
                            (snapshot, &shape, delta_progress),
                            events,
                        )?;
                        players = self.nearby_players_by_key(dimension);
                    }
                    PistonBodyOutcome::StuckQuery => {
                        cursor.references = match piston_sticky_bounds(pos, snapshot)? {
                            Some(bounds) => {
                                self.capture_piston_bodies(dimension, pos, bounds, true)?
                            }
                            None => Vec::new(),
                        };
                        cursor.index = 0;
                        cursor.phase = PistonBodyPhase::Stuck;
                    }
                    PistonBodyOutcome::Done => break,
                }
            }
            Some(())
        })();
        let applied = self.apply_pending_spawns_checked(dimension, leftover);
        result.is_some() && applied.is_some()
    }

    fn capture_piston_bodies(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        bounds: Aabb,
        sticky: bool,
    ) -> Option<Vec<PistonBodyReference>> {
        use crate::entity::store::EntityBodyReadView;
        let ids = EntityBodyReadView::new(&self.level_by_id(dimension).entities, None, None)
            .ids_in_box_sections_with_players(bounds);
        let mut references = Vec::new();
        for id in ids {
            if let Some(connection) = self.level_by_id(dimension).entities.player_connection(id) {
                let player = self.levels.player(connection)?;
                if player.entity_id() != id || &player.dimension != dimension {
                    return None;
                }
                if player.avatar_removed
                    || player.game_type == GameType::Spectator
                    || !bounds.intersects(player.bounding_box().ok()?)
                {
                    continue;
                }
                if sticky
                    && (!player.on_ground()
                        || !(player.block_contact.supporting == Some(pos)
                            || center_in_sticky_bounds(player.position(), bounds)))
                {
                    continue;
                }
                references.push(PistonBodyReference::Session(connection));
            } else {
                let body = self.level_by_id(dimension).entities.get(id)?;
                if body.removal_reason().is_some() || !bounds.intersects(body.bounding_box()) {
                    continue;
                }
                if sticky
                    && (crate::entity::piston_push_reaction(body)
                        != VanillaPistonPushReaction::Normal
                        || !body.on_ground()
                        || !(body.is_supported_by(pos)
                            || center_in_sticky_bounds(body.position(), bounds)))
                {
                    continue;
                }
                references.push(PistonBodyReference::Stored(id));
            }
        }
        Some(references)
    }

    fn move_player_by_piston(
        &mut self,
        reference: ConnectionId,
        delta: Vec3d,
        no_clip: Direction,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        use crate::world::movement_collision::set_piston_no_clip;
        let id = reference;
        let game_time = self.game_time();
        let player = self.levels.player_mut(id)?;
        set_piston_no_clip(Some(no_clip));
        let delta = if player.game_type == GameType::Spectator {
            delta
        } else {
            crate::entity::limit_body_piston_movement(
                &mut player.block_contact.piston_deltas,
                &mut player.block_contact.piston_deltas_game_time,
                delta,
                game_time,
            )
        };
        let moved =
            self.move_player_by_external_force(id, delta, crate::entity::MoverType::Piston, events);
        set_piston_no_clip(None);
        moved
    }
}

fn center_in_sticky_bounds(position: Vec3d, bounds: Aabb) -> bool {
    position.x >= bounds.min_x
        && position.x <= bounds.max_x
        && position.z >= bounds.min_z
        && position.z <= bounds.max_z
}

#[derive(Clone, Copy)]
enum PistonBodyReference {
    Stored(i32),
    Session(ConnectionId),
}

enum PistonBodyPhase {
    Collided,
    Retraction,
    Stuck,
}

enum PistonBodyOutcome {
    Paused,
    StuckQuery,
    Session(ConnectionId),
    Done,
}

struct PistonBodyCursor {
    references: Vec<PistonBodyReference>,
    index: usize,
    phase: PistonBodyPhase,
    pending: Option<crate::entity::PendingPistonMove>,
}

impl PistonBodyCursor {
    fn advance_session(
        &mut self,
        world: &mut ServerWorld,
        reference: ConnectionId,
        pos: BlockPos,
        (snapshot, shape, delta_progress): (&PistonMovingSnapshot, &[Aabb], f64),
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = world.levels.player(reference)?;
        let bounds = player.bounding_box().ok()?;
        let movement = snapshot.movement_direction();
        let step = step_of(movement);
        let delta = match self.phase {
            PistonBodyPhase::Collided => {
                if snapshot.causes_bounce() {
                    self.index += 1;
                    return Some(());
                }
                let Some(delta) =
                    piston_push_delta(snapshot, pos, shape, movement, delta_progress, bounds)
                else {
                    self.index += 1;
                    return Some(());
                };
                self.phase = PistonBodyPhase::Retraction;
                Vec3d {
                    x: delta * step.0,
                    y: delta * step.1,
                    z: delta * step.2,
                }
            }
            PistonBodyPhase::Retraction => {
                self.index += 1;
                self.phase = PistonBodyPhase::Collided;
                if snapshot.extending || !snapshot.is_source {
                    return Some(());
                }
                let Some(delta) = piston_base_delta(bounds, pos, movement, delta_progress) else {
                    return Some(());
                };
                delta
            }
            PistonBodyPhase::Stuck => {
                self.index += 1;
                Vec3d {
                    x: delta_progress * step.0,
                    y: delta_progress * step.1,
                    z: delta_progress * step.2,
                }
            }
        };
        world.move_player_by_piston(reference, delta, movement, events)
    }

    fn advance(
        &mut self,
        entities: &mut EntityStore,
        level: &mut LevelCtx<'_, '_>,
        pos: BlockPos,
        snapshot: &PistonMovingSnapshot,
        shape: &[Aabb],
        delta_progress: f64,
    ) -> Option<PistonBodyOutcome> {
        if let Some(pending) = self.pending.take() {
            if let std::ops::ControlFlow::Break(pending) = pending.resume(entities, level)? {
                self.pending = Some(pending);
                return Some(PistonBodyOutcome::Paused);
            }
        }
        let movement = snapshot.movement_direction();
        let step = step_of(movement);
        while let Some(reference) = self.references.get(self.index).copied() {
            let id = match reference {
                PistonBodyReference::Stored(id) => id,
                PistonBodyReference::Session(reference) => {
                    return Some(PistonBodyOutcome::Session(reference))
                }
            };
            let Some(pending) = entities.with_entity_mut(id, |body, entities| match self.phase {
                PistonBodyPhase::Collided => {
                    if crate::entity::piston_push_reaction(body)
                        == VanillaPistonPushReaction::Ignore
                    {
                        self.index += 1;
                        return Some(None);
                    }
                    let bounds = body.bounding_box();
                    if snapshot.causes_bounce() {
                        let motion = body.delta_movement();
                        body.set_delta_movement(Vec3d {
                            x: if step.0 != 0.0 { step.0 } else { motion.x },
                            y: if step.1 != 0.0 { step.1 } else { motion.y },
                            z: if step.2 != 0.0 { step.2 } else { motion.z },
                        });
                    }
                    let Some(delta) =
                        piston_push_delta(snapshot, pos, shape, movement, delta_progress, bounds)
                    else {
                        self.index += 1;
                        return Some(None);
                    };
                    self.phase = PistonBodyPhase::Retraction;
                    crate::entity::move_entity_by_piston_pausing(
                        body,
                        entities,
                        level,
                        Vec3d {
                            x: delta * step.0,
                            y: delta * step.1,
                            z: delta * step.2,
                        },
                        movement,
                    )
                    .ok()
                }
                PistonBodyPhase::Retraction => {
                    self.index += 1;
                    self.phase = PistonBodyPhase::Collided;
                    if !snapshot.extending && snapshot.is_source {
                        fix_entity_within_piston_base(
                            body,
                            entities,
                            level,
                            pos,
                            movement,
                            delta_progress,
                        )
                        .ok()
                    } else {
                        Some(None)
                    }
                }
                PistonBodyPhase::Stuck => {
                    self.index += 1;
                    crate::entity::move_entity_by_piston_pausing(
                        body,
                        entities,
                        level,
                        Vec3d {
                            x: delta_progress * step.0,
                            y: delta_progress * step.1,
                            z: delta_progress * step.2,
                        },
                        movement,
                    )
                    .ok()
                }
            }) else {
                // An entity removed earlier in this step is no longer moved.
                self.index += 1;
                if !matches!(self.phase, PistonBodyPhase::Stuck) {
                    self.phase = PistonBodyPhase::Collided;
                }
                continue;
            };
            if let Some(pending) = pending? {
                self.pending = Some(pending);
                return Some(PistonBodyOutcome::Paused);
            }
        }
        Some(match self.phase {
            PistonBodyPhase::Stuck => PistonBodyOutcome::Done,
            _ => PistonBodyOutcome::StuckQuery,
        })
    }
}

fn piston_sticky_bounds(pos: BlockPos, snapshot: &PistonMovingSnapshot) -> Option<Option<Aabb>> {
    if !snapshot.is_sticky_for_entities()
        || !matches!(
            snapshot.movement_direction(),
            Direction::North | Direction::South | Direction::West | Direction::East
        )
    {
        return Some(None);
    }
    let shape = static_block_collision_boxes(snapshot.moved_state)?;
    let sticky_top = shape
        .iter()
        .map(|part| part.max_y)
        .fold(f64::NEG_INFINITY, f64::max);
    Some(Some(snapshot.move_by_position_and_progress(
        pos,
        Aabb {
            min_x: 0.0,
            min_y: sticky_top,
            min_z: 0.0,
            max_x: 1.0,
            max_y: 1.500_001_000_000_000_1,
            max_z: 1.0,
        },
    )))
}

/// `PistonMovingBlockEntity.moveEntityByPiston`'s per-part displacement
/// (`:167-186`), or `None` when the entity is clear of the swept band.
fn piston_push_delta(
    snapshot: &PistonMovingSnapshot,
    pos: BlockPos,
    shape: &[Aabb],
    movement: Direction,
    delta_progress: f64,
    entity_box: Aabb,
) -> Option<f64> {
    let mut delta = 0.0_f64;
    for part in shape {
        let moving = piston_movement_area(
            snapshot.move_by_position_and_progress(pos, *part),
            movement,
            delta_progress,
        );
        if moving.intersects(entity_box) {
            delta = delta.max(piston_movement_to_clear(moving, movement, entity_box));
            if delta >= delta_progress {
                break;
            }
        }
    }
    (delta > 0.0).then_some(delta.min(delta_progress) + 0.01)
}

/// `PistonMovingBlockEntity.fixEntityWithinPistonBase` (`:274-286`).
fn fix_entity_within_piston_base(
    entity: &mut crate::entity::Entity,
    store: &mut crate::entity::EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    direction: Direction,
    delta_progress: f64,
) -> Result<Option<crate::entity::PendingPistonMove>, ()> {
    let Some(delta) = piston_base_delta(entity.bounding_box(), pos, direction, delta_progress)
    else {
        return Ok(None);
    };
    crate::entity::move_entity_by_piston_pausing(entity, store, level, delta, direction)
}

fn piston_base_delta(
    entity_box: Aabb,
    pos: BlockPos,
    direction: Direction,
    delta_progress: f64,
) -> Option<Vec3d> {
    let base = Aabb {
        min_x: f64::from(pos.x),
        min_y: f64::from(pos.y),
        min_z: f64::from(pos.z),
        max_x: f64::from(pos.x) + 1.0,
        max_y: f64::from(pos.y) + 1.0,
        max_z: f64::from(pos.z) + 1.0,
    };
    if !entity_box.intersects(base) {
        return None;
    }
    let opposite_direction = opposite(direction);
    let delta = piston_movement_to_clear(base, opposite_direction, entity_box) + 0.01;
    let delta_intersected =
        piston_movement_to_clear(base, opposite_direction, intersect(entity_box, base)) + 0.01;
    if (delta - delta_intersected).abs() >= 0.01 {
        return None;
    }
    let delta = delta.min(delta_progress) + 0.01;
    let step = step_of(opposite_direction);
    Some(Vec3d {
        x: delta * step.0,
        y: delta * step.1,
        z: delta * step.2,
    })
}

fn step_of(direction: Direction) -> (f64, f64, f64) {
    match direction {
        Direction::Down => (0.0, -1.0, 0.0),
        Direction::Up => (0.0, 1.0, 0.0),
        Direction::North => (0.0, 0.0, -1.0),
        Direction::South => (0.0, 0.0, 1.0),
        Direction::West => (-1.0, 0.0, 0.0),
        Direction::East => (1.0, 0.0, 0.0),
    }
}

fn union(left: Aabb, right: Aabb) -> Aabb {
    Aabb {
        min_x: left.min_x.min(right.min_x),
        min_y: left.min_y.min(right.min_y),
        min_z: left.min_z.min(right.min_z),
        max_x: left.max_x.max(right.max_x),
        max_y: left.max_y.max(right.max_y),
        max_z: left.max_z.max(right.max_z),
    }
}

fn intersect(left: Aabb, right: Aabb) -> Aabb {
    Aabb {
        min_x: left.min_x.max(right.min_x),
        min_y: left.min_y.max(right.min_y),
        min_z: left.min_z.max(right.min_z),
        max_x: left.max_x.min(right.max_x),
        max_y: left.max_y.min(right.max_y),
        max_z: left.max_z.min(right.max_z),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit() -> Aabb {
        Aabb {
            min_x: 0.0,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 1.0,
            max_y: 1.0,
            max_z: 1.0,
        }
    }

    #[test]
    fn an_entity_clear_of_the_band_is_not_pushed() {
        let mut snapshot = crate::block_entity::piston::PistonMovingSnapshot {
            moved_state: 0,
            facing: Direction::East,
            last_progress: 0.0,
            progress: 0.0,
            extending: true,
            is_source: true,
            last_ticked: 0,
        };
        snapshot.progress = 0.0;
        let clear = Aabb {
            min_x: 3.0,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 4.0,
            max_y: 1.0,
            max_z: 1.0,
        };
        assert_eq!(
            piston_push_delta(
                &snapshot,
                BlockPos { x: 0, y: 0, z: 0 },
                &[unit()],
                Direction::East,
                0.5,
                clear,
            ),
            None
        );
    }
    fn batch_state(name: &str) -> Option<i32> {
        let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)?;
        bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id)
    }

    fn batch_world() -> (ServerWorld, LevelId, BlockPos, [i32; 2]) {
        batch_world_with_config(crate::tests::datapack_config())
    }

    fn batch_world_with_config(
        config: crate::world::ServerWorldConfig,
    ) -> (ServerWorld, LevelId, BlockPos, [i32; 2]) {
        use crate::entity::{ArmorStandData, EntityData};
        use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID;

        let (sender, receiver) =
            crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
                .unwrap();
        let mut world = ServerWorld::new(config).unwrap();
        world.set_test_respawn_radius(0);
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
        let pos = BlockPos { x: 5, y: 64, z: 0 };
        let state = |name| batch_state(name).unwrap();
        assert!(world.set_block(dimension, pos, state("minecraft:moving_piston"), 2));
        assert!(world
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .unwrap()
            .configure_piston(state("minecraft:slime_block"), Direction::East, true, false));
        let ids = [64.0, 64.75].map(|y| {
            world
                .spawn_stored_entity_for_test(
                    Java26BuiltinDimension::Overworld,
                    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                    Vec3d { x: 5.25, y, z: 0.5 },
                    EntityData::ArmorStand(ArmorStandData::new().unwrap()),
                )
                .0
        });
        world.levels[ConnectionId(1)].set_position(Vec3d {
            x: 5.25,
            y: 64.0,
            z: 0.5,
        });
        assert_eq!(world.level_by_id(&dimension).entities.difficulty(), None);
        world
            .level_mut_by_id(&dimension)
            .entities
            .drain_lifecycle_events();
        (world, dimension, pos, ids)
    }

    #[path = "stored_contact_tests.rs"]
    mod stored_contact_tests;

    #[test]
    fn piston_player_applies_lava_contact_in_the_actual_push_call() {
        let mut config = crate::tests::datapack_config();
        config.game_type = GameType::Survival;
        let (mut world, dimension, pos, ids) = batch_world_with_config(config);
        for id in ids {
            world
                .level_mut_by_id(&dimension)
                .entities
                .remove(id, crate::entity::RemovalReason::UnloadedToChunk);
        }
        assert!(world
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .unwrap()
            .configure_piston(
                batch_state("minecraft:stone").unwrap(),
                Direction::East,
                true,
                false
            ));
        let previous = Vec3d {
            x: 5.25,
            y: 64.0,
            z: 0.5,
        };
        world.set_player_position_for_test(ConnectionId(1), previous);
        world
            .level_mut_by_id(&dimension)
            .chunks
            .set_block_state(6, 64, 0, batch_state("minecraft:lava").unwrap())
            .unwrap();
        world
            .level_mut_by_id(&dimension)
            .chunks
            .set_block_state(5, 63, 0, batch_state("minecraft:stone").unwrap())
            .unwrap();
        world.levels[ConnectionId(1)]
            .block_contact
            .movement_emission
            .move_dist = 1.0;
        let before = world.player_health_for_test(ConnectionId(1));
        let mut events = Vec::new();
        let count = world.game_events(Java26BuiltinDimension::Overworld).0.len();
        world.tick_moving_piston(&dimension, pos, &mut events);
        let player = world.levels.player_at(ConnectionId(1));
        assert!(player.position().x > previous.x);
        assert_eq!(player.position().y, previous.y);
        assert!(player.on_ground());
        assert!(player.block_contact.movement_emission.move_dist > 1.0);
        assert!(world
            .game_events(Java26BuiltinDimension::Overworld)
            .0
            .iter()
            .skip(count)
            .any(|event| event.game_event == crate::level_event::VanillaGameEvent::STEP));
        assert!(player.remaining_fire_ticks > 0);
        assert!(world.player_health_for_test(ConnectionId(1)) < before);
        assert_eq!(
            world
                .level_by_id(&dimension)
                .chunks
                .block_entities()
                .get(pos)
                .unwrap()
                .piston_snapshot()
                .unwrap()
                .progress,
            0.5
        );
        assert!(!events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::PlayerPosition(_)
            }
        )));
    }
    #[test]
    fn piston_moves_session_across_mixed_sections_without_teleporting() {
        use crate::entity::store::EntityBodyReadView;
        let (mut world, dimension, _, ids) = batch_world();
        let connection = ConnectionId(1);
        let start = Vec3d {
            x: 15.6,
            y: 64.0,
            z: 0.5,
        };
        world.set_player_position_for_test(connection, start);
        world.levels[connection].set_on_ground(true);
        let entity_id = world.levels[connection].entity_id();
        let bounds = Aabb {
            min_x: 0.0,
            min_y: 64.0,
            min_z: 0.0,
            max_x: 16.0,
            max_y: 66.0,
            max_z: 1.0,
        };
        let slab = crate::fluid::with_property(
            batch_state("minecraft:stone_slab").unwrap(),
            "type",
            "bottom",
        )
        .unwrap();
        world
            .level_mut_by_id(&dimension)
            .chunks
            .set_block_state(16, 64, 0, slab)
            .unwrap();
        let filed = world.levels[connection].cold.last_section_pos;
        let teleport = world.next_teleport_id;
        let mut events = Vec::new();
        assert!(world
            .move_player_by_piston(
                connection,
                Vec3d {
                    x: 0.4,
                    y: 0.0,
                    z: 0.0,
                },
                Direction::East,
                &mut events,
            )
            .is_some());
        assert_eq!(world.levels[connection].position().x, 16.0);
        assert_eq!(world.levels[connection].position().y, 64.5);
        assert_eq!(world.levels[connection].cold.last_section_pos, filed);
        assert_eq!(world.next_teleport_id, teleport);
        assert!(events.is_empty());
        let level = world.level_by_id(&dimension);
        assert!(level
            .entities
            .players_in_box_sections(Aabb::of_entity(
                world.levels[connection].position(),
                0.6,
                1.8
            ))
            .contains(&connection));
        assert_eq!(level.entities.ids_in_box_sections(bounds), ids);
        assert!(!EntityBodyReadView::new(&level.entities, None, None)
            .ids_in_box_sections_with_players(Aabb::of_entity(
                Vec3d {
                    x: 1.0,
                    y: 64.0,
                    z: 0.5
                },
                0.6,
                1.8
            ))
            .contains(&entity_id));
        assert_eq!(
            crate::world::movement_collision::current_piston_no_clip(),
            None
        );
    }
}
