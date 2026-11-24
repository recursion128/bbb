use super::target::TargetView;
use super::{Goal, GroundNavigation, MobAiContext, MobPathingSnapshot};
use crate::entity::raider::{self, RaiderRaid};
use crate::entity::{Entity, EntityData};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::{mth_cos, mth_sin};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PatrolStepResult {
    pub(crate) target: Option<BlockPos>,
    pub(crate) patrolling: bool,
    pub(crate) companion_target: Option<(Vec<i32>, BlockPos)>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LongDistancePatrolGoal;

fn is_companion(entity: &Entity, body_id: i32) -> bool {
    entity.id() != body_id
        && matches!(entity.data(), EntityData::GeneralLiving(data) if data.raider_retained().is_some_and(|state| state.raid() == RaiderRaid::None))
}

impl Goal for LongDistancePatrolGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        matches!(
            raider::patrol_spawn_state(targets.body_entity()),
            Some((Some(_), _, true))
        ) && matches!(
            targets
                .store()
                .controlling_mob_passenger(targets.body_entity(), targets.level()),
            Some(None)
        ) && context.valid_target(targets).is_none()
            && targets.game_time() >= context.nav.patrol_cooldown_until
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if !context.nav.navigation_done {
            return;
        }
        let body = targets.body_entity();
        let Some((Some(target), leader, patrolling)) = raider::patrol_spawn_state(body) else {
            return;
        };
        let bounds = body.bounding_box().inflate(16.0, 16.0, 16.0);
        if patrolling
            && targets
                .store()
                .first_entity_in_box(bounds, |entity| is_companion(entity, body.id()))
                .is_none()
        {
            context.nav.patrol_result = Some(PatrolStepResult {
                target: Some(target),
                patrolling: false,
                companion_target: None,
            });
            return;
        }
        let position = context.body.position;
        let distance_squared = (position.x - (f64::from(target.x) + 0.5)).powi(2)
            + (position.y - (f64::from(target.y) + 0.5)).powi(2)
            + (position.z - (f64::from(target.z) + 0.5)).powi(2);
        if leader && distance_squared < 100.0 {
            let x = (position.x.floor() as i32).wrapping_add(-500 + context.next_int(1000));
            let z = (position.z.floor() as i32).wrapping_add(-500 + context.next_int(1000));
            context.nav.patrol_result = Some(PatrolStepResult {
                target: Some(BlockPos {
                    x,
                    y: position.y.floor() as i32,
                    z,
                }),
                patrolling: true,
                companion_target: None,
            });
        } else {
            context.nav.patrol_request = Some((path_target(position, target), leader));
        }
    }
}

fn path_target(position: Vec3d, target: BlockPos) -> BlockPos {
    let target = Vec3d {
        x: f64::from(target.x) + 0.5,
        y: f64::from(target.y),
        z: f64::from(target.z) + 0.5,
    };
    let x = position.x - target.x;
    let y = position.y - target.y;
    let z = position.z - target.z;
    let cos = f64::from(mth_cos(90.0));
    let sin = f64::from(mth_sin(90.0));
    let dx = (x * cos + z * sin) * 0.4 + target.x - position.x;
    let dy = y * 0.4 + target.y - position.y;
    let dz = (z * cos - x * sin) * 0.4 + target.z - position.z;
    let length = (dx * dx + dy * dy + dz * dz).sqrt();
    let (dx, dy, dz) = if length < f64::from(1.0e-5_f32) {
        (0.0, 0.0, 0.0)
    } else {
        (dx / length, dy / length, dz / length)
    };
    BlockPos {
        x: (dx * 10.0 + position.x).floor() as i32,
        y: (dy * 10.0 + position.y).floor() as i32,
        z: (dz * 10.0 + position.z).floor() as i32,
    }
}

pub(super) fn resolve_navigation(
    context: &mut MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    navigation: &mut GroundNavigation,
    mob: Option<MobPathingSnapshot>,
) -> Option<()> {
    if let Some((mut target, leader)) = context.nav.patrol_request.take() {
        let level = targets.level();
        target.y = level.motion_blocking_no_leaves_height(target.x, target.z)?;
        let mob = mob.unwrap_or_else(|| MobPathingSnapshot::from_context(context));
        if navigation.move_to(level, mob, target, 1, if leader { 0.595 } else { 0.7 }) {
            if leader {
                let body = targets.body_entity();
                let bounds = body.bounding_box().inflate(16.0, 16.0, 16.0);
                let mut companions = Vec::new();
                targets.store().for_each_box_section_id(bounds, |id| {
                    if targets.store().get(id).is_some_and(|entity| {
                        is_companion(entity, body.id()) && entity.bounding_box().intersects(bounds)
                    }) {
                        companions.push(id);
                    }
                });
                let (source_target, _, patrolling) = raider::patrol_spawn_state(body).unwrap();
                context.nav.patrol_result = Some(PatrolStepResult {
                    target: source_target,
                    patrolling,
                    companion_target: Some((companions, target)),
                });
            }
        } else {
            let x =
                (context.body.position.x.floor() as i32).wrapping_add(-8 + context.next_int(16));
            let z =
                (context.body.position.z.floor() as i32).wrapping_add(-8 + context.next_int(16));
            let target = BlockPos {
                x,
                y: level.motion_blocking_no_leaves_height(x, z)?,
                z,
            };
            navigation.move_to(level, mob, target, 1, 0.7);
            context.nav.patrol_cooldown_until = targets.game_time().wrapping_add(200);
        }
    }
    context.nav.navigation_done = navigation.path().is_none_or(super::GroundPath::is_done);
    Some(())
}
