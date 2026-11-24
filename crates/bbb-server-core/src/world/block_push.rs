use super::{ServerWorld, TeleportTransition, WorldEvent};
use crate::connection::ConnectionId;
use crate::entity::store::EntityBodyReadView;
use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_static_collision_shape;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{
    BlockPos, Vec3d, PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_DELTA_Z,
    PLAYER_RELATIVE_ROTATE_DELTA, PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y,
    PLAYER_RELATIVE_Y_ROT, PLAYER_RELATIVE_Z,
};
use bbb_server_types::geometry::{collide_axis, Aabb, Axis};
use bbb_server_types::interaction::player_pose_bounding_box;

fn intersecting_box(left: Aabb, right: Aabb) -> Option<Aabb> {
    let intersection = Aabb {
        min_x: left.min_x.max(right.min_x),
        min_y: left.min_y.max(right.min_y),
        min_z: left.min_z.max(right.min_z),
        max_x: left.max_x.min(right.max_x),
        max_y: left.max_y.min(right.max_y),
        max_z: left.max_z.min(right.max_z),
    };
    (intersection.min_x < intersection.max_x
        && intersection.min_y < intersection.max_y
        && intersection.min_z < intersection.max_z)
        .then_some(intersection)
}

fn subtract_box(source: Aabb, removed: Aabb) -> Vec<Aabb> {
    let Some(cut) = intersecting_box(source, removed) else {
        return vec![source];
    };
    let mut pieces = Vec::with_capacity(6);
    let mut push = |piece: Aabb| {
        if piece.min_x < piece.max_x && piece.min_y < piece.max_y && piece.min_z < piece.max_z {
            pieces.push(piece);
        }
    };
    push(Aabb {
        max_x: cut.min_x,
        ..source
    });
    push(Aabb {
        min_x: cut.max_x,
        ..source
    });
    let middle_x = Aabb {
        min_x: cut.min_x,
        max_x: cut.max_x,
        ..source
    };
    push(Aabb {
        max_y: cut.min_y,
        ..middle_x
    });
    push(Aabb {
        min_y: cut.max_y,
        ..middle_x
    });
    let middle_xy = Aabb {
        min_y: cut.min_y,
        max_y: cut.max_y,
        ..middle_x
    };
    push(Aabb {
        max_z: cut.min_z,
        ..middle_xy
    });
    push(Aabb {
        min_z: cut.max_z,
        ..middle_xy
    });
    pieces
}

pub(crate) fn collision_only_second(
    old_state: i32,
    new_state: i32,
    pos: BlockPos,
) -> Option<Vec<Aabb>> {
    let old = vanilla_block_state_static_collision_shape(old_state)?
        .boxes()
        .map(|unit| Aabb {
            min_x: unit.min_x,
            min_y: unit.min_y,
            min_z: unit.min_z,
            max_x: unit.max_x,
            max_y: unit.max_y,
            max_z: unit.max_z,
        })
        .collect::<Vec<_>>();
    let mut difference = vanilla_block_state_static_collision_shape(new_state)?
        .boxes()
        .map(|unit| Aabb {
            min_x: unit.min_x,
            min_y: unit.min_y,
            min_z: unit.min_z,
            max_x: unit.max_x,
            max_y: unit.max_y,
            max_z: unit.max_z,
        })
        .collect::<Vec<_>>();
    for removed in old {
        difference = difference
            .into_iter()
            .flat_map(|source| subtract_box(source, removed))
            .collect();
    }
    let delta = Vec3d {
        x: f64::from(pos.x),
        y: f64::from(pos.y),
        z: f64::from(pos.z),
    };
    Some(
        difference
            .into_iter()
            .map(|shape| shape.moved(delta))
            .collect(),
    )
}

/// One entity `push_entities_up` lifts: a stored entity the level's store owns,
/// or a session player, which is not in that store.
#[derive(Debug, Clone, Copy)]
enum PushTarget {
    Stored(i32),
    Player(ConnectionId),
}

fn push_entities_offset_y(bounding_box: Aabb, shape: &[Aabb]) -> f64 {
    1.0 + collide_axis(
        Axis::Y,
        bounding_box.moved(Vec3d {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }),
        shape,
        -1.0,
    )
}

pub(in crate::world) fn block_push_shape(
    old_state: i32,
    new_state: i32,
    pos: BlockPos,
) -> Vec<Aabb> {
    collision_only_second(old_state, new_state, pos).unwrap_or_default()
}

impl ServerWorld {
    pub(in crate::world) fn push_live_entities_up(
        &mut self,
        dimension: &LevelId,
        old_state: i32,
        new_state: i32,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let shape = block_push_shape(old_state, new_state, pos);
        if shape.is_empty() {
            return Some(());
        }
        let mut access = WorldBlockPush {
            world: self,
            dimension,
            events,
        };
        access.push(&shape)
    }
}

struct WorldBlockPush<'a> {
    world: &'a mut ServerWorld,
    dimension: &'a LevelId,
    events: &'a mut Vec<WorldEvent>,
}

impl WorldBlockPush<'_> {
    fn push(&mut self, shape: &[Aabb]) -> Option<()> {
        let bounds = shape.iter().copied().reduce(|left, right| Aabb {
            min_x: left.min_x.min(right.min_x),
            min_y: left.min_y.min(right.min_y),
            min_z: left.min_z.min(right.min_z),
            max_x: left.max_x.max(right.max_x),
            max_y: left.max_y.max(right.max_y),
            max_z: left.max_z.max(right.max_z),
        })?;
        for target in self.targets(bounds)? {
            let Some(bounding_box) = self.bounding_box(target) else {
                continue;
            };
            let delta_y = push_entities_offset_y(bounding_box, shape);
            self.teleport_relative(target, delta_y)?;
        }
        Some(())
    }

    fn targets(&mut self, bounds: Aabb) -> Option<Vec<PushTarget>> {
        let level = self.world.level_by_id(self.dimension);
        let bodies = EntityBodyReadView::new(&level.entities, None, None);
        let mut pushed = Vec::new();
        for id in bodies.ids_in_box_sections_with_players(bounds) {
            if let Some(reference) = level.entities.player_connection(id) {
                let player = self.world.levels.player(reference)?;
                if &player.dimension != self.dimension
                    || player.avatar_removed
                    || player.game_type == GameType::Spectator
                {
                    continue;
                }
                let Ok(bounding_box) = player_pose_bounding_box(
                    player.position(),
                    player.pose_id,
                    player.pose_scale(),
                ) else {
                    continue;
                };
                if bounding_box.intersects(bounds) {
                    pushed.push(PushTarget::Player(reference));
                }
            } else if let Some(entity) = bodies.get(id) {
                if entity.removal_reason().is_none() && entity.bounding_box().intersects(bounds) {
                    pushed.push(PushTarget::Stored(id));
                }
            }
        }
        Some(pushed)
    }

    fn bounding_box(&self, target: PushTarget) -> Option<Aabb> {
        match target {
            PushTarget::Stored(id) => {
                let entity = self.world.level_by_id(self.dimension).entities.get(id)?;
                Some(entity.bounding_box())
            }
            PushTarget::Player(id) => {
                let player = self.world.levels.player(id)?;
                Some(
                    player_pose_bounding_box(
                        player.position(),
                        player.pose_id,
                        player.pose_scale(),
                    )
                    .ok()?,
                )
            }
        }
    }

    fn teleport_relative(&mut self, target: PushTarget, delta_y: f64) -> Option<()> {
        match target {
            PushTarget::Stored(id) => {
                let entities = &self.world.level_by_id(self.dimension).entities;
                let entity = entities.get(id)?;
                let position = entity.position();
                let mut tree =
                    crate::entity::TeleportTree::capture_with_players(entity, entities, |id| {
                        let reference = self
                            .world
                            .level_by_id(self.dimension)
                            .entities
                            .player_connection(id)?;
                        let player = self.world.levels.player(reference)?;
                        if &player.dimension != self.dimension {
                            return None;
                        }
                        Some(crate::entity::ExternalPlayerTeleportBody {
                            uuid: player.profile.uuid,
                            y_rot: player.rotation().y_rot,
                            scale: player.pose_scale(),
                            pose_id: player.pose_id,
                        })
                    })?;
                tree.snap_to(
                    Vec3d {
                        y: position.y + delta_y,
                        ..position
                    },
                    self.world.game_time,
                )?;
                loop {
                    let entities = &mut self.world.level_mut_by_id(self.dimension).entities;
                    let step = entities
                        .with_entity_mut(id, |body, entities| tree.apply_next(body, entities))??;
                    let Some(step) = step else {
                        break;
                    };
                    let snap = match step {
                        crate::entity::TeleportApplication::Native {
                            entity_id,
                            waypoint_updated,
                        } => {
                            if waypoint_updated {
                                self.world.apply_waypoint_entity_lifecycle(
                                    self.dimension,
                                    &[crate::entity::store::EntityLifecycleEvent::WaypointUpdated { entity_id }],
                                    self.events,
                                );
                            }
                            continue;
                        }
                        crate::entity::TeleportApplication::Player(snap) => snap,
                    };
                    let reference = self
                        .world
                        .level_by_id(self.dimension)
                        .entities
                        .player_connection(snap.entity_id)?;
                    let player = self.world.levels.player_mut(reference)?;
                    let waypoint_updated = player.position() != snap.placement.position
                        && player.tick_count != 0
                        && !player.avatar_removed;
                    player.set_position(snap.placement.position);
                    player.block_contact.old_position = player.position();
                    player.set_y_rot(snap.placement.y_rot);
                    // ServerPlayer.java:1707-1710: snapTo resets movement validation.
                    player.first_good = player.position();
                    player.last_good = player.position();
                    self.world.update_player_query_section(reference);
                    if waypoint_updated {
                        self.world
                            .waypoint_update_player_source(reference, self.events);
                        self.world
                            .waypoint_update_player_receiver(reference, self.events);
                    }
                }
            }
            PushTarget::Player(reference) => {
                // The expanding block lifts the player on the same support.
                self.world.sync_teleport_position(
                    reference,
                    &TeleportTransition {
                        new_level: *self.dimension,
                        position: Vec3d {
                            x: 0.0,
                            y: delta_y,
                            z: 0.0,
                        },
                        y_rot: 0.0,
                        x_rot: 0.0,
                        relatives_mask: PLAYER_RELATIVE_X
                            | PLAYER_RELATIVE_Y
                            | PLAYER_RELATIVE_Z
                            | PLAYER_RELATIVE_DELTA_X
                            | PLAYER_RELATIVE_DELTA_Y
                            | PLAYER_RELATIVE_DELTA_Z
                            | PLAYER_RELATIVE_Y_ROT
                            | PLAYER_RELATIVE_X_ROT
                            | PLAYER_RELATIVE_ROTATE_DELTA,
                    },
                    self.events,
                );
            }
        }
        Some(())
    }
}

#[cfg(test)]
mod tests;
