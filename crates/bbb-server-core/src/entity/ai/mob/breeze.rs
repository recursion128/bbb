//! Breeze combat on the existing goal, navigation and projectile owners.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_HONEY_BLOCK_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_INHALING_ID, VANILLA_ENTITY_POSE_LONG_JUMPING_ID,
    VANILLA_ENTITY_POSE_SHOOTING_ID, VANILLA_ENTITY_POSE_SLIDING_ID,
    VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air, FluidFamily};
use bbb_server_types::geometry::{long_jump_velocity, Vec3Ops};

use super::float_goal::should_float;
use super::navigation::ground_path_type_from_state;
use super::random_pos::{default_random_pos_away, RandomPosMob};
use super::ranged::RangedTarget;
use super::sensing::body_line_of_sight;
use super::target::TargetView;
use super::{Goal, MobAiContext, MobPostStepAction};
use crate::entity::{living, GroundPathType};

#[derive(Debug, Clone, Copy, Default)]
enum Phase {
    #[default]
    Ready,
    Sliding {
        until: i32,
    },
    Inhaling {
        until: i32,
        landing: Vec3d,
    },
    Airborne {
        until: i32,
        leaving_water: bool,
    },
    ChargingShot {
        until: i32,
    },
    RecoveringShot {
        until: i32,
    },
}

#[derive(Debug, Clone, Default)]
pub(super) struct BreezeCombatGoal {
    phase: Phase,
    jump_ready_at: i32,
    shoot_ready_at: i32,
    shoot_requested: bool,
}

impl Goal for BreezeCombatGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        context.valid_target(targets).is_some()
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(target) = context.valid_target(targets) else {
            return false;
        };
        if context.body.position.distance_to_sqr(target.position())
            > context.body.follow_range.powi(2)
        {
            context.set_target(None, targets);
            return false;
        }
        true
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.phase = Phase::Ready;
        self.shoot_requested = false;
        context.stop_navigation();
        pose(targets, VANILLA_ENTITY_POSE_STANDING_ID);
        targets.set_body_discard_friction(false);
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some((target_id, position, eye_y, ranged_target)) =
            context.valid_target(targets).map(|target| {
                (
                    target.entity_id(),
                    target.position(),
                    target.eye_y(),
                    RangedTarget::of(&target),
                )
            })
        else {
            return;
        };
        context.set_look_at_with_limits(position.x, eye_y, position.z, 90.0, 90.0);
        let now = context.tick_count;
        match self.phase {
            Phase::Ready => self.choose_action(context, targets, target_id, position),
            Phase::Sliding { until } => {
                if context.navigation_done() || now >= until {
                    context.stop_navigation();
                    pose(targets, VANILLA_ENTITY_POSE_STANDING_ID);
                    self.phase = Phase::Ready;
                    self.shoot_requested = true;
                }
            }
            Phase::Inhaling { until, landing } => {
                context.stop_navigation();
                context.set_look_at_with_limits(landing.x, landing.y, landing.z, 90.0, 90.0);
                if now >= until {
                    if let Some(movement) = jump_velocity(context, targets, landing) {
                        pose(targets, VANILLA_ENTITY_POSE_LONG_JUMPING_ID);
                        targets.set_body_discard_friction(true);
                        context.set_body_y_rot(context.body.y_body_rot);
                        targets.set_body_delta_movement(movement);
                        sound(targets, "minecraft:entity.breeze.jump", 1.0);
                        self.phase = Phase::Airborne {
                            until: now + 200,
                            leaving_water: context.body.in_water,
                        };
                    } else {
                        pose(targets, VANILLA_ENTITY_POSE_STANDING_ID);
                        self.phase = Phase::Ready;
                        self.jump_ready_at = now + 10;
                    }
                }
            }
            Phase::Airborne {
                until,
                mut leaving_water,
            } => {
                leaving_water &= context.body.in_water;
                let landed = context.body.on_ground || (context.body.in_water && !leaving_water);
                if landed || now >= until {
                    pose(targets, VANILLA_ENTITY_POSE_STANDING_ID);
                    targets.set_body_discard_friction(false);
                    if landed {
                        sound(targets, "minecraft:entity.breeze.land", 1.0);
                    }
                    self.phase = Phase::Ready;
                    self.jump_ready_at = now
                        + if targets
                            .body_entity()
                            .living()
                            .is_some_and(|body| body.last_damage_memory_time > 0)
                        {
                            2
                        } else {
                            10
                        };
                    self.shoot_requested = landed;
                } else {
                    self.phase = Phase::Airborne {
                        until,
                        leaving_water,
                    };
                }
            }
            Phase::ChargingShot { until } => {
                context.stop_navigation();
                if now >= until {
                    context.post_step.push(MobPostStepAction::RangedAttack {
                        target: ranged_target,
                        power: 1.0,
                    });
                    self.phase = Phase::RecoveringShot { until: now + 4 };
                }
            }
            Phase::RecoveringShot { until } => {
                if now >= until {
                    pose(targets, VANILLA_ENTITY_POSE_STANDING_ID);
                    self.phase = Phase::Ready;
                    self.shoot_requested = false;
                    self.shoot_ready_at = now + 10;
                }
            }
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

impl BreezeCombatGoal {
    // Shoot; ShootWhenStuck.
    fn choose_action(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
        target_id: i32,
        enemy: Vec3d,
    ) {
        let now = context.tick_count;
        if targets.body_entity().pose_id() != VANILLA_ENTITY_POSE_STANDING_ID {
            return;
        }
        let swimming = should_float(context, targets.body_entity());
        let immobilized = context.body.is_passenger
            || targets
                .body_entity()
                .living()
                .is_some_and(|body| body.active_effect("minecraft:levitation").is_some());
        // LongJump.
        let can_jump = now >= self.jump_ready_at
            && !immobilized
            && !swimming
            && (context.body.on_ground || context.body.in_water)
            && context.body.position.distance_to_sqr(enemy) > 16.0
            && can_jump_from(targets);
        let stuck = immobilized || (context.body.in_water && !can_jump);
        self.shoot_requested |= stuck;
        if self.shoot_requested {
            if now >= self.shoot_ready_at && context.body.position.distance_to_sqr(enemy) < 256.0 {
                context.stop_navigation();
                pose(targets, VANILLA_ENTITY_POSE_SHOOTING_ID);
                sound(targets, "minecraft:entity.breeze.inhale", 1.0);
                self.phase = Phase::ChargingShot { until: now + 15 };
                return;
            }
            if stuck || now < self.shoot_ready_at {
                return;
            }
            self.shoot_requested = false;
        }
        // LongJump.
        if can_jump {
            let wanted = around_target(context, targets, target_id, enemy);
            if let Some(landing) = landing_surface(context, targets, wanted) {
                context.stop_navigation();
                pose(targets, VANILLA_ENTITY_POSE_INHALING_ID);
                targets.play_body_sound(
                    "minecraft:entity.breeze.charge",
                    SoundSource::Hostile,
                    1.0,
                    1.0,
                );
                self.phase = Phase::Inhaling {
                    until: now + 10,
                    landing,
                };
                return;
            }
            self.shoot_requested = context.body.in_water;
        }
        if context.body.on_ground && !context.body.in_water && now >= self.jump_ready_at {
            let position = context.body.position;
            let away = inner_circle(context, enemy)
                .then(|| {
                    default_random_pos_away(
                        targets.level(),
                        &context.random,
                        RandomPosMob::of(targets.body_entity(), context.nav.malus),
                        5,
                        5,
                        enemy,
                    )
                    .flatten()
                    .filter(|wanted| {
                        wanted.distance_to_sqr(enemy) > position.distance_to_sqr(enemy)
                            && line_of_sight(context, targets, *wanted)
                    })
                })
                .flatten();
            let wanted = away.unwrap_or_else(|| {
                if context.random.next_boolean() {
                    around_target(context, targets, target_id, enemy)
                } else {
                    let direction = enemy.plus(position.scale(-1.0));
                    enemy.plus(
                        direction
                            .normalize()
                            .scale(-(4.0 + context.random.next_double() * 4.0)),
                    )
                }
            });
            context.try_navigate_to_position(wanted, 1, 0.6);
            pose(targets, VANILLA_ENTITY_POSE_SLIDING_ID);
            sound(targets, "minecraft:entity.breeze.slide", 1.0);
            self.phase = Phase::Sliding {
                until: now + context.random.next_int_between_inclusive(20, 40),
            };
        }
    }
}

fn inner_circle(context: &MobAiContext, target: Vec3d) -> bool {
    let dx = target.x - (context.body.position.x.floor() + 0.5);
    let dy = target.y - (context.body.position.y.floor() + 0.5);
    let dz = target.z - (context.body.position.z.floor() + 0.5);
    dx * dx + dz * dz < 16.0 && dy.abs() < 10.0
}

/// `BreezeAi.SlideToTargetSink`: an idle walk slides.
pub(super) fn idle_slide(
    _context: &mut MobAiContext,
    targets: &mut TargetView<'_, '_, '_>,
    start: bool,
) {
    if targets.body_entity().entity_type_id()
        != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BREEZE_ID
    {
        return;
    }
    if start {
        sound(targets, "minecraft:entity.breeze.slide", 1.0);
        pose(targets, VANILLA_ENTITY_POSE_SLIDING_ID);
    } else {
        pose(targets, VANILLA_ENTITY_POSE_STANDING_ID);
    }
}

fn pose(targets: &mut TargetView<'_, '_, '_>, pose_id: i32) {
    targets.set_pose(pose_id);
}

fn sound(targets: &mut TargetView<'_, '_, '_>, resource_id: &'static str, volume: f32) {
    if !targets.body_entity().is_silent() {
        targets.play_body_sound(resource_id, SoundSource::Hostile, volume, 1.0);
    }
}

fn around_target(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    target_id: i32,
    target_pos: Vec3d,
) -> Vec3d {
    let head_yaw = targets
        .store()
        .get(target_id)
        .and_then(|body| body.living())
        .map(|body| body.y_head_rot)
        .or_else(|| {
            targets
                .level()
                .players()
                .iter()
                .find(|body| body.entity_id == target_id)
                .map(|body| body.y_rot)
        })
        .unwrap_or(context.body.y_rot);
    let yaw = (f64::from(head_yaw) + 180.0 + context.random.next_gaussian() * 45.0).to_radians();
    let radius = 4.0 + f64::from(context.random.next_float()) * 4.0;
    target_pos.plus(Vec3d {
        x: -yaw.sin() * radius,
        y: 0.0,
        z: yaw.cos() * radius,
    })
}

fn can_jump_from(targets: &TargetView<'_, '_, '_>) -> bool {
    let pos = targets.body_entity().block_position();
    let level = targets.level();
    if vanilla_block_state_block_type_id(level.block_state(pos))
        == Some(VANILLA_BLOCK_HONEY_BLOCK_ID)
    {
        return false;
    }
    (1..=4).all(|dy| {
        let state = level.block_state(BlockPos {
            y: pos.y + dy,
            ..pos
        });
        block_state_is_air(state)
            || block_state_fluid_state(state).family() == Some(FluidFamily::Water)
    })
}

fn landing_surface(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    wanted: Vec3d,
) -> Option<Vec3d> {
    for dy in [-10.0, 10.0] {
        let hit = bbb_server_types::geometry::clip::clip_with(
            wanted,
            wanted.plus(Vec3d {
                x: 0.0,
                y: dy,
                z: 0.0,
            }),
            |pos| {
                targets.level().block_state_checked(pos)?;
                Some((
                    targets
                        .level()
                        .block_collision_boxes_for_entity(targets.body_entity(), pos)?,
                    None,
                ))
            },
        )?;
        let Some(hit) = hit else {
            continue;
        };
        let surface = BlockPos {
            x: hit.location.x.floor() as i32,
            y: hit.location.y.floor() as i32 + 1,
            z: hit.location.z.floor() as i32,
        };
        let path_type = ground_path_type_from_state(targets.level(), hit.pos)?;
        if matches!(
            path_type,
            GroundPathType::Fire
                | GroundPathType::Lava
                | GroundPathType::Damaging
                | GroundPathType::PowderSnow
                | GroundPathType::DamageCautious
        ) {
            continue;
        }
        let landing = Vec3d {
            x: f64::from(surface.x) + 0.5,
            y: f64::from(surface.y),
            z: f64::from(surface.z) + 0.5,
        };
        if line_of_sight(
            context,
            targets,
            landing.plus(Vec3d {
                x: 0.0,
                y: 0.5,
                z: 0.0,
            }),
        ) || line_of_sight(
            context,
            targets,
            landing.plus(Vec3d {
                x: 0.0,
                y: 4.5,
                z: 0.0,
            }),
        ) {
            return Some(landing);
        }
    }
    None
}

fn line_of_sight(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    position: Vec3d,
) -> bool {
    context.body.position.distance_to_sqr(position) <= context.body.follow_range.max(50.0).powi(2)
        && body_line_of_sight(
            targets.level(),
            targets.body_entity(),
            context.body.position,
            position,
        ) == Some(true)
}

fn jump_velocity(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    landing: Vec3d,
) -> Option<Vec3d> {
    let max_speed = f64::from(0.058_333_334_f32 * context.body.follow_range as f32);
    let mut angles = [40, 55, 60, 75, 80];
    angles.rotate_left(context.random.next_int(5) as usize);
    angles.into_iter().find_map(|angle| {
        let mut velocity = long_jump_velocity(
            context.body.position,
            landing,
            max_speed,
            angle,
            living::gravity(targets.body_entity()),
            None,
        )?;
        if let Some(effect) = targets
            .body_entity()
            .living()
            .and_then(|body| body.active_effect("minecraft:jump_boost"))
        {
            velocity.y +=
                velocity.normalize().y * f64::from(0.1_f32 * (effect.amplifier() as f32 + 1.0));
        }
        Some(velocity)
    })
}
