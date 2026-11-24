use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_path_facts;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_types::geometry::{
    collect_block_collisions_with_moving_pistons, collide_with_boxes, Aabb, Vec3Ops,
};

use super::target::TargetView;
use crate::entity::inside_blocks::{collided_along_vector, for_each_block_intersected};
use crate::entity::projectile_hit::mth_atan2;
use crate::entity::{Entity, EntityData};

use super::{Goal, MobAiContext, MobAiRuntime, MobAiStepKindResult};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(super) struct GhastRuntime {
    wanted_position: Option<Vec3d>,
    float_duration: i32,
}

impl GhastRuntime {
    pub(super) const fn new() -> Self {
        Self {
            wanted_position: None,
            float_duration: 0,
        }
    }

    pub(super) fn prepare_step(&self, context: &mut MobAiContext, entity: &Entity) {
        let EntityData::GeneralLiving(data) = entity.data() else {
            unreachable!("ghast controller has general living state");
        };
        let happy = data.happy_ghast_state();
        if happy.is_some() && data.is_baby() {
            return;
        }
        context.species.ghast_movement_allowed =
            happy.is_none_or(|state| state.still_timeout <= 0 && !state.stays_still);
        context.species.ghast_careful = happy.is_some();
        context.species.ghast_wanted_position = self.wanted_position;
        context.controls.flight_y_rot = entity.y_rot();
    }

    pub(super) fn set_wanted_position(&mut self, wanted_position: Option<Vec3d>) {
        self.wanted_position = wanted_position;
    }

    pub(super) fn move_control_tick(
        &mut self,
        context: &mut MobAiContext,
        level: &LevelCtx<'_, '_>,
    ) -> MobAiStepKindResult {
        let mut movement = context.body.delta_movement;
        if !context.species.ghast_movement_allowed {
            if context.species.ghast_careful {
                context.species.ghast_wanted_position = None;
                movement = Vec3d::default();
            }
        } else if let Some(wanted) = context.species.ghast_wanted_position {
            let old_duration = self.float_duration;
            self.float_duration = self.float_duration.wrapping_sub(1);
            if old_duration <= 0 {
                self.float_duration = self
                    .float_duration
                    .wrapping_add(context.next_int(5))
                    .wrapping_add(2);
                let travel = Vec3d {
                    x: wanted.x - context.body.position.x,
                    y: wanted.y - context.body.position.y,
                    z: wanted.z - context.body.position.z,
                };
                if can_reach(
                    level,
                    context.body.bounding_box,
                    travel,
                    context.species.ghast_careful,
                    context.body.in_water,
                    context.body.in_lava,
                ) {
                    let acceleration = travel
                        .normalize()
                        .scale(context.body.flying_speed * 5.0 / 3.0);
                    movement = movement.plus(acceleration);
                } else {
                    context.species.ghast_wanted_position = None;
                }
            }
        }
        MobAiStepKindResult::Motion {
            movement,
            y_rot: context.controls.flight_y_rot,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GhastRandomFloatAroundGoal;

impl Goal for GhastRandomFloatAroundGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        if !context.species.ghast_movement_allowed {
            return false;
        }
        context.species.ghast_wanted_position.is_none_or(|wanted| {
            let distance = context.body.position.distance_to_sqr(wanted);
            distance < 1.0 || distance > 3600.0
        })
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        false
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        context.species.ghast_wanted_position = suitable_fly_to(context, targets);
    }
}

fn random_position(context: &mut MobAiContext) -> Vec3d {
    Vec3d {
        x: context.body.position.x + f64::from((context.next_float() * 2.0 - 1.0) * 16.0),
        y: context.body.position.y + f64::from((context.next_float() * 2.0 - 1.0) * 16.0),
        z: context.body.position.z + f64::from((context.next_float() * 2.0 - 1.0) * 16.0),
    }
}

fn suitable_fly_to(context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) -> Option<Vec3d> {
    let distance_to_blocks = if context.species.ghast_careful { 16 } else { 0 };
    let mut last = None;
    for _ in 0..64 {
        let candidate = random_position(context);
        last = targets
            .body_entity()
            .mob_home()
            .is_none_or(|(home, radius)| {
                let center = Vec3d {
                    x: f64::from(home.x) + 0.5,
                    y: f64::from(home.y) + 0.5,
                    z: f64::from(home.z) + 0.5,
                };
                center.distance_to_sqr(candidate) < f64::from(radius).powi(2)
            })
            .then_some(candidate);
        if let Some(candidate) = last {
            if good_target(targets.level(), candidate, distance_to_blocks) {
                return Some(candidate);
            }
        }
    }
    let mut candidate = last.unwrap_or_else(|| random_position(context));
    let pos = super::navigation::containing_block(candidate);
    let height = targets.level().motion_blocking_height(pos.x, pos.z)?;
    if height < pos.y && height > targets.level().min_y() {
        candidate.y = context.body.position.y - (context.body.position.y - candidate.y).abs();
    }
    Some(candidate)
}

fn is_air(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    let state = level.block_state_checked(pos)?;
    if let Some(facts) = vanilla_block_state_path_facts(state) {
        Some(facts.is_air())
    } else {
        level.content_registry().block_by_state_id(state)?;
        Some(false)
    }
}

fn good_target(level: &LevelCtx<'_, '_>, candidate: Vec3d, distance_to_blocks: i32) -> bool {
    if distance_to_blocks <= 0 {
        return true;
    }
    let pos = super::navigation::containing_block(candidate);
    if is_air(level, pos) != Some(true) {
        return false;
    }
    for (dx, dy, dz) in [
        (1, 0, 0),
        (-1, 0, 0),
        (0, 1, 0),
        (0, -1, 0),
        (0, 0, 1),
        (0, 0, -1),
    ] {
        for distance in 1..distance_to_blocks {
            if is_air(
                level,
                BlockPos {
                    x: pos.x + dx * distance,
                    y: pos.y + dy * distance,
                    z: pos.z + dz * distance,
                },
            ) == Some(false)
            {
                return true;
            }
        }
    }
    false
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GhastLookGoal;

impl Goal for GhastLookGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        true
    }

    /// `Ghast.faceMovementDirection`.
    fn tick(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        let (xd, zd) = match context.unchecked_target(targets) {
            None => (context.body.delta_movement.x, context.body.delta_movement.z),
            Some(target) => {
                let position = target.position();
                if position.distance_to_sqr(context.body.position) >= 4096.0 {
                    return;
                }
                (
                    position.x - context.body.position.x,
                    position.z - context.body.position.z,
                )
            }
        };
        context.controls.flight_y_rot =
            -(mth_atan2(xd, zd) as f32) * (180.0 / std::f32::consts::PI);
        context.body.y_body_rot = context.controls.flight_y_rot;
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

fn can_reach(
    level: &LevelCtx<'_, '_>,
    bounding_box: Aabb,
    travel: Vec3d,
    careful: bool,
    in_water: bool,
    in_lava: bool,
) -> bool {
    if careful {
        let destination = bounding_box.moved(travel);
        let area = destination.inflate(1.0, 1.0, 1.0);
        for x in area.min_x.floor() as i32..=area.max_x.floor() as i32 {
            for y in area.min_y.floor() as i32..=area.max_y.floor() as i32 {
                for z in area.min_z.floor() as i32..=area.max_z.floor() as i32 {
                    if !block_traversal_possible(level, BlockPos { x, y, z }, None, false, false) {
                        return false;
                    }
                }
            }
        }
        let start = Vec3d {
            y: bounding_box.min_y,
            ..bounding_box.center()
        };
        let mut reachable = true;
        for_each_block_intersected(start, start.plus(travel), destination, |pos, _| {
            reachable = bounding_box.intersects(Aabb::of_block(pos))
                || block_traversal_possible(
                    level,
                    pos,
                    Some((bounding_box, travel)),
                    in_water,
                    in_lava,
                );
            reachable
        });
        return reachable;
    }
    let boxes = collect_block_collisions_with_moving_pistons(
        bounding_box.expand_towards(travel),
        |pos| level.is_moving_piston_at(pos).unwrap_or(true),
        |pos| level.block_collision_boxes(pos),
    )
    .into_iter()
    .filter(|(pos, _)| !bounding_box.intersects(Aabb::of_block(*pos)))
    .map(|(_, shape)| shape)
    .collect::<Vec<_>>();
    collide_with_boxes(travel, bounding_box, &boxes) == travel
}

fn block_traversal_possible(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    sweep: Option<(Aabb, Vec3d)>,
    can_path_through_water: bool,
    can_path_through_lava: bool,
) -> bool {
    let Some(state) = level.block_state_checked(pos) else {
        return false;
    };
    if is_air(level, pos) == Some(true) {
        return true;
    }
    if level.block_state_in_tag(state, "minecraft:happy_ghast_avoids") {
        return false;
    }
    let fluid = block_state_fluid_state(state);
    if let Some(family) = fluid.family() {
        let intersects = if let Some((bounds, travel)) = sweep {
            let Some(height) =
                crate::entity::environment::fluid_height_at(level.chunks(), pos, fluid)
            else {
                return false;
            };
            collided_along_vector(
                bounds,
                travel,
                Aabb {
                    max_y: f64::from(pos.y) + f64::from(height),
                    ..Aabb::of_block(pos)
                },
            )
        } else {
            true
        };
        if intersects {
            return match family {
                FluidFamily::Water => can_path_through_water,
                FluidFamily::Lava => can_path_through_lava,
            };
        }
    }
    let Some(boxes) = level.block_collision_boxes_empty(pos) else {
        return false;
    };
    // Ghast.java:302: move local block shapes into world space before sweep check.
    let offset = Vec3d {
        x: f64::from(pos.x),
        y: f64::from(pos.y),
        z: f64::from(pos.z),
    };
    match sweep {
        Some((bounds, travel)) => !boxes
            .into_iter()
            .any(|shape| collided_along_vector(bounds, travel, shape.moved(offset))),
        None => boxes.is_empty(),
    }
}

impl MobAiRuntime {
    pub(crate) fn tick_ghast_ambient_sound(&mut self, alive: bool) -> Option<f32> {
        if !alive {
            return None;
        }
        let roll = self.random.next_int(1000);
        let threshold = self.ambient_sound_time;
        self.ambient_sound_time = self.ambient_sound_time.wrapping_add(1);
        if roll >= threshold {
            return None;
        }
        self.ambient_sound_time = -80;
        Some(self.voice_pitch(false))
    }
}
