use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_render_facts;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PARROT_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::{collect_block_collisions_with_moving_pistons, Aabb, Vec3Ops};

use crate::entity::GroundPathType;

use super::navigation::walk_path_type;
use super::target::{Target, TargetView};
use super::{reduced_tick_delay, Goal, MobAiContext};

// `TamableAnimal.shouldTryTeleportToOwner` (`TamableAnimal.java:248-251`).
const TELEPORT_DISTANCE_SQR: f64 = 144.0;

fn movable_owner<'a, 'l, 'w>(targets: &'a TargetView<'_, 'l, 'w>) -> Option<Target<'a, 'l, 'w>> {
    let body = targets.body();
    if body.ordered_to_sit()
        || targets.store().vehicle(body.entity_id()).is_some()
        || crate::entity::leash::leash_data(targets.body_entity()).is_some()
    {
        return None;
    }
    targets
        .by_uuid(targets.body_entity().ownable_owner().flatten()?)
        .filter(|owner| owner.is_local() && !owner.spectator())
}

pub(super) fn request_teleport_to_owner(
    context: &mut MobAiContext,
    targets: &TargetView<'_, '_, '_>,
) {
    if let Some(owner) = movable_owner(targets) {
        if context.body.position.distance_to_sqr(owner.position()) >= TELEPORT_DISTANCE_SQR {
            context.nav.owner_teleport =
                Some(super::navigation::containing_block(owner.position()));
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FollowOwnerGoal {
    speed_modifier: f64,
    start_distance: f32,
    stop_distance: f32,
    time_to_recalc_path: i32,
    old_water_cost: f32,
}

impl FollowOwnerGoal {
    pub(super) const fn new(speed_modifier: f64, start_distance: f32, stop_distance: f32) -> Self {
        Self {
            speed_modifier,
            start_distance,
            stop_distance,
            time_to_recalc_path: 0,
            old_water_cost: 0.0,
        }
    }
}

impl Goal for FollowOwnerGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(owner) = movable_owner(targets) else {
            return false;
        };
        let start = self.start_distance * self.start_distance;
        context.body.position.distance_to_sqr(owner.position()) >= f64::from(start)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if context.nav.navigation_done {
            return false;
        }
        let Some(owner) = movable_owner(targets) else {
            return false;
        };
        let stop = self.stop_distance * self.stop_distance;
        context.body.position.distance_to_sqr(owner.position()) > f64::from(stop)
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.time_to_recalc_path = 0;
        self.old_water_cost = context.nav.malus.get(GroundPathType::Water);
        context.set_own_pathfinding_malus(GroundPathType::Water, 0.0);
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
        context.set_own_pathfinding_malus(GroundPathType::Water, self.old_water_cost);
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(owner) = movable_owner(targets) else {
            return;
        };
        let position = owner.position();
        let far_owner = context.body.position.distance_to_sqr(position) >= TELEPORT_DISTANCE_SQR;
        if !far_owner {
            let crate::entity::EntityData::GeneralLiving(data) = targets.body_entity().data()
            else {
                unreachable!("following an owner requires a tamable mob");
            };
            context.set_look_at_with_limits(
                position.x,
                owner.eye_y(),
                position.z,
                10.0,
                data.tamable_max_head_x_rot() as f32,
            );
        }
        self.time_to_recalc_path -= 1;
        if self.time_to_recalc_path > 0 {
            return;
        }
        self.time_to_recalc_path = reduced_tick_delay(10);
        if far_owner {
            context.nav.owner_teleport = Some(super::navigation::containing_block(position));
        } else {
            context.navigate_to(position, self.speed_modifier);
        }
    }
}

/// Runs a pending `tryToTeleportToOwner` against the live level; `None` refuses
/// the tick because the level could not answer a read.
pub(super) fn resolve_request(
    context: &mut MobAiContext,
    targets: &TargetView<'_, '_, '_>,
) -> Option<()> {
    let Some(target) = context.nav.owner_teleport.take() else {
        return Some(());
    };
    let can_fly_to_owner = targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_PARROT_ID;
    let mob_block = super::navigation::containing_block(context.body.position);
    let bounding_box = context.body.bounding_box;
    let destination = teleport_to_around_block_pos(&context.random, target, |pos| {
        can_teleport_to(
            targets.level(),
            targets.store(),
            bounding_box,
            mob_block,
            can_fly_to_owner,
            pos,
        )
    })?;
    if let Some(pos) = destination {
        let position = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y),
            z: f64::from(pos.z) + 0.5,
        };
        context.snap_to(position);
        context.stop_navigation();
    }
    Some(())
}

fn teleport_to_around_block_pos(
    random: &GameRandom,
    target: BlockPos,
    mut can_teleport_to: impl FnMut(BlockPos) -> Option<bool>,
) -> Option<Option<BlockPos>> {
    for _ in 0..10 {
        let xd = random.next_int(7) - 3;
        let zd = random.next_int(7) - 3;
        if xd.abs() >= 2 || zd.abs() >= 2 {
            let yd = random.next_int(3) - 1;
            let pos = BlockPos {
                x: target.x + xd,
                y: target.y + yd,
                z: target.z + zd,
            };
            if can_teleport_to(pos)? {
                return Some(Some(pos));
            }
        }
    }
    Some(None)
}

fn can_teleport_to(
    level: &LevelCtx<'_, '_>,
    entities: &crate::entity::EntityStore,
    bounding_box: Aabb,
    mob_block: BlockPos,
    can_fly_to_owner: bool,
    pos: BlockPos,
) -> Option<bool> {
    if walk_path_type(level, pos)? != GroundPathType::Walkable {
        return Some(false);
    }
    if !can_fly_to_owner {
        let below = BlockPos {
            y: pos.y - 1,
            ..pos
        };
        let below_state = level.block_state(below);
        let is_leaves = if let Some(facts) = vanilla_block_state_render_facts(below_state) {
            facts.leaves_block()
        } else {
            level.content_registry().block_by_state_id(below_state)?;
            false
        };
        if is_leaves {
            return Some(false);
        }
    }
    let moved = bounding_box.moved(Vec3d {
        x: f64::from(pos.x - mob_block.x),
        y: f64::from(pos.y - mob_block.y),
        z: f64::from(pos.z - mob_block.z),
    });
    let block_collision = collect_block_collisions_with_moving_pistons(
        moved,
        |pos| level.is_moving_piston_at(pos).unwrap_or(true),
        |pos| level.block_collision_boxes(pos),
    )
    .into_iter()
    .any(|(_, shape)| shape.intersects(moved));
    let search = moved.inflate(1.0e-7, 1.0e-7, 1.0e-7);
    let entity_collision = entities
        .first_entity_in_box(search, |other| other.can_be_collided_with())
        .is_some();
    Some(!block_collision && !entity_collision)
}
