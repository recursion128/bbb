//! Goat ram preparation and charge navigation.

use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_LONG_JUMPING_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_GOAT_ID};
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::ai::MemoryModuleType;
use crate::entity::EntityData;
use crate::world::simulation_level::LevelCtx;

use super::navigation::{
    containing_block, random_pos_stable_destination, walk_path_type, GroundNavigation,
    MobPathingSnapshot,
};
use super::target::{conditions::TargetingConditions, Target, TargetRef, TargetView};
use super::{Goal, MobAiContext, MobPostStepAction};

pub(super) mod long_jump;

#[derive(Debug, Clone)]
pub(super) struct RamPathProbe {
    candidates: [Option<BlockPos>; 4],
    selected: Option<BlockPos>,
    pending: bool,
}

pub(super) fn resolve_path(
    context: &mut MobAiContext,
    targets: &mut TargetView<'_, '_, '_>,
    navigation: &mut GroundNavigation,
    mob: Option<MobPathingSnapshot>,
) -> Option<bool> {
    if !context.nav.goat_ram_path.as_ref()?.pending {
        return None;
    }
    let mob = mob.unwrap_or_else(|| MobPathingSnapshot::from_context(context));
    let probe = context.nav.goat_ram_path.as_mut()?;
    probe.pending = false;
    let level = targets.level();
    probe.selected = probe.candidates.iter().flatten().copied().find(|pos| {
        navigation
            .create_path_to(level, mob, *pos, 0, None)
            .is_some_and(|path| path.reached())
    });
    let admitted = probe.selected.is_some();
    if !admitted {
        finish_effect(targets, true, false, false);
    }
    Some(admitted)
}

#[derive(Debug, Clone, Copy)]
enum RamPhase {
    Prepare {
        target: TargetRef,
        target_pos: BlockPos,
        start: Option<BlockPos>,
        reached_at: Option<i64>,
        until: i64,
    },
    Charge {
        wanted: Vec3d,
        direction: Vec3d,
        until: i64,
    },
}

#[derive(Debug, Default, Clone)]
pub(crate) struct GoatRamGoal {
    phase: Option<RamPhase>,
    next_scan: i64,
}

impl GoatRamGoal {
    fn finish(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
        now: i64,
        failed: bool,
        snap_horn: bool,
        impact: bool,
    ) {
        self.phase = None;
        self.next_scan = now.saturating_add(20);
        context.nav.goat_ram_path = None;
        context.stop_navigation();
        finish_effect(targets, failed, snap_horn, impact);
    }

    fn consume_probe(&mut self, context: &mut MobAiContext) -> bool {
        let Some(probe) = context.nav.goat_ram_path.take() else {
            return false;
        };
        if let Some(RamPhase::Prepare { start, .. }) = &mut self.phase {
            *start = probe.selected;
        }
        if probe.selected.is_none() {
            // The resolver already queued the failed cooldown and head reset.
            self.phase = None;
            return false;
        }
        true
    }
}

impl Goal for GoatRamGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let now = targets.game_time();
        if now < self.next_scan
            || targets.body_entity().pose_id() == VANILLA_ENTITY_POSE_LONG_JUMPING_ID
            || crate::entity::goat::cooldown_active(
                targets.body_entity(),
                MemoryModuleType::RamCooldownTicks,
            )
            || !ram_allowed(context, targets)
        {
            return false;
        }
        self.next_scan = now.saturating_add(20);
        self.phase = None;
        let bounds = context.body.bounding_box.inflate(
            context.body.follow_range,
            context.body.follow_range,
            context.body.follow_range,
        );
        let Some(target) = targets
            .local_living()
            .filter(|target| {
                target.bounding_box().intersects(bounds) && ram_target(targets, target)
            })
            .min_by(|a, b| {
                context
                    .body
                    .position
                    .distance_to_sqr(a.position())
                    .total_cmp(&context.body.position.distance_to_sqr(b.position()))
            })
        else {
            return false;
        };
        let target_pos = target.block_position();
        let target_ref = target.target_ref();
        let Some(probe) = prepare_probe(context, targets.level(), target_pos) else {
            finish_effect(targets, true, false, false);
            return false;
        };
        context.nav.goat_ram_path = Some(probe);
        self.phase = Some(RamPhase::Prepare {
            target: target_ref,
            target_pos,
            start: None,
            reached_at: None,
            until: now.saturating_add(160),
        });
        true
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        if self.consume_probe(context) {
            if let Some(RamPhase::Prepare {
                start: Some(pos), ..
            }) = self.phase
            {
                context.try_navigate_to_position(bottom_center(pos), 0, 1.25);
            }
        }
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if context.nav.goat_ram_path.is_some() && !self.consume_probe(context) {
            return false;
        }
        if !ram_allowed(context, targets) {
            return false;
        }
        match self.phase {
            Some(RamPhase::Prepare {
                target,
                target_pos,
                until,
                ..
            }) => {
                if targets.game_time() >= until {
                    return false;
                }
                let Some((new_pos, target_ref)) = targets
                    .facts_of(target)
                    .filter(|target| target.alive() && !target.dead_or_dying())
                    .map(|target| (target.block_position(), target.target_ref()))
                else {
                    return false;
                };
                if new_pos != target_pos {
                    context.stop_navigation();
                    targets.body_entity_event(59);
                    let Some(probe) = prepare_probe(context, targets.level(), new_pos) else {
                        return false;
                    };
                    context.nav.goat_ram_path = Some(probe);
                    self.phase = Some(RamPhase::Prepare {
                        target: target_ref,
                        target_pos: new_pos,
                        start: None,
                        reached_at: None,
                        until,
                    });
                }
                true
            }
            Some(RamPhase::Charge { until, .. }) => targets.game_time() < until,
            None => false,
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if context.nav.goat_ram_path.is_some() && !self.consume_probe(context) {
            self.next_scan = targets.game_time().saturating_add(20);
            context.stop_navigation();
            return;
        }
        if let Some(phase) = self.phase {
            self.finish(
                context,
                targets,
                targets.game_time(),
                matches!(phase, RamPhase::Prepare { .. }),
                false,
                false,
            );
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let now = targets.game_time();
        if context.nav.goat_ram_path.is_some() && !self.consume_probe(context) {
            self.next_scan = now.saturating_add(20);
            return;
        }
        match self.phase {
            Some(RamPhase::Prepare {
                target,
                target_pos,
                start,
                reached_at,
                until,
            }) => {
                let Some((eye, target_ref)) = targets.facts_of(target).map(|target| {
                    (
                        Vec3d {
                            y: target.eye_y(),
                            ..target.position()
                        },
                        target.target_ref(),
                    )
                }) else {
                    return;
                };
                context.set_look_at(eye.x, eye.y, eye.z);
                let Some(start) = start else {
                    return;
                };
                if containing_block(context.body.position) != start {
                    context.try_navigate_to_position(bottom_center(start), 0, 1.25);
                    return;
                }
                if reached_at.is_none() {
                    targets.body_entity_event(58);
                }
                let reached_at = reached_at.unwrap_or(now);
                if now - reached_at < 20 {
                    self.phase = Some(RamPhase::Prepare {
                        target: target_ref,
                        target_pos,
                        start: Some(start),
                        reached_at: Some(reached_at),
                        until,
                    });
                    return;
                }
                // PrepareRamNearestTarget: run through the far edge.
                let wanted = Vec3d {
                    x: f64::from(target_pos.x)
                        + 0.5
                        + 0.5 * f64::from((target_pos.x - start.x).signum()),
                    y: f64::from(target_pos.y),
                    z: f64::from(target_pos.z)
                        + 0.5
                        + 0.5 * f64::from((target_pos.z - start.z).signum()),
                };
                let block = containing_block(context.body.position);
                let direction = Vec3d {
                    x: f64::from(block.x) - wanted.x,
                    y: 0.0,
                    z: f64::from(block.z) - wanted.z,
                }
                .normalize();
                let screaming = matches!(targets.body_entity().data(), EntityData::GeneralLiving(data) if data.goat_retained().is_some_and(|goat| goat.is_screaming()));
                let pitch = (context.next_float() - context.next_float()) * 0.2
                    + if targets.body().baby() { 1.5 } else { 1.0 };
                targets.play_body_sound(
                    if screaming {
                        "minecraft:entity.goat.screaming.prepare_ram"
                    } else {
                        "minecraft:entity.goat.prepare_ram"
                    },
                    SoundSource::Neutral,
                    1.0,
                    pitch,
                );
                self.phase = Some(RamPhase::Charge {
                    wanted,
                    direction,
                    until: now.saturating_add(200),
                });
                context.try_navigate_to_position(wanted, 0, 3.0);
            }
            Some(RamPhase::Charge {
                wanted, direction, ..
            }) => {
                let collision_id = targets
                    .first_living_in_box(context.body.bounding_box, |target| {
                        ram_target(targets, target)
                    })
                    .map(|target| target.entity_id());
                if let Some(target_id) = collision_id {
                    let living = targets
                        .body_entity()
                        .living()
                        .expect("goat has living state");
                    let effect_level = |id| {
                        living
                            .active_effect(id)
                            .map_or(0, |effect| effect.amplifier() + 1)
                    };
                    let speed = (context.controls.speed * 1.65).clamp(0.2, 3.0)
                        + 0.25
                            * (effect_level("minecraft:speed") - effect_level("minecraft:slowness"))
                                as f32;
                    let knockback =
                        f64::from(speed) * if targets.body().baby() { 1.0 } else { 2.5 };
                    context.post_step.push(MobPostStepAction::GoatRamAttack {
                        target_id,
                        knockback,
                        direction,
                    });
                    self.finish(context, targets, now, false, false, true);
                } else if crate::entity::goat::ram::has_horn_breaking_block(
                    targets.body_entity(),
                    targets.level(),
                    wanted,
                ) {
                    self.finish(context, targets, now, false, true, true);
                } else if context.nav.navigation_done
                    || context.body.position.distance_to_sqr(wanted) < 0.25 * 0.25
                {
                    self.finish(context, targets, now, false, false, false);
                }
            }
            None => {}
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

fn finish_effect(
    targets: &mut TargetView<'_, '_, '_>,
    failed: bool,
    snap_horn: bool,
    impact: bool,
) {
    if let Some((entity, store, level)) = targets.world_mut() {
        crate::entity::goat::ram::finish_ram(entity, store, level, failed, snap_horn, impact);
    }
}

fn bottom_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y),
        z: f64::from(pos.z) + 0.5,
    }
}

fn prepare_probe(
    context: &MobAiContext,
    level: &LevelCtx<'_, '_>,
    target: BlockPos,
) -> Option<RamPathProbe> {
    let walkable = |pos| {
        random_pos_stable_destination(level, VANILLA_ENTITY_TYPE_GOAT_ID, pos)?.then_some(())?;
        (context.nav.malus.get(walk_path_type(level, pos)?) == 0.0).then_some(())
    };
    walkable(target)?;
    let mut candidates = [None; 4];
    for (index, (dx, dz)) in [(0, -1), (0, 1), (-1, 0), (1, 0)].into_iter().enumerate() {
        let mut farthest = target;
        for distance in 1..=7 {
            let pos = BlockPos {
                x: target.x + dx * distance,
                z: target.z + dz * distance,
                ..target
            };
            if walkable(pos).is_none() {
                break;
            }
            farthest = pos;
        }
        if (farthest.x - target.x).abs() + (farthest.z - target.z).abs() >= 4 {
            candidates[index] = Some(farthest);
        }
    }
    let body = containing_block(context.body.position);
    let distance = |pos: Option<BlockPos>| {
        pos.map_or(f64::INFINITY, |pos| {
            let dx = f64::from(pos.x) - f64::from(body.x);
            let dy = f64::from(pos.y) - f64::from(body.y);
            let dz = f64::from(pos.z) - f64::from(body.z);
            dx * dx + dy * dy + dz * dz
        })
    };
    candidates.sort_by(|a, b| distance(*a).total_cmp(&distance(*b)));
    candidates[0]?;
    Some(RamPathProbe {
        candidates,
        selected: None,
        pending: true,
    })
}

fn ram_target<'w>(targets: &TargetView<'_, '_, 'w>, target: &Target<'_, '_, 'w>) -> bool {
    TargetingConditions::for_combat().test(targets, target, |target| {
        if target.entity_type_id() == VANILLA_ENTITY_TYPE_GOAT_ID
            || (target.entity_type_id() == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
                && !targets.level().mob_griefing())
        {
            return false;
        }
        let Some(border) = targets.level().world_border() else {
            return false;
        };
        let bounds = target.bounding_box();
        let now = targets.level().game_time();
        let epsilon = f64::from(1.0e-5_f32);
        bounds.min_x >= border.min_x_at(now)
            && bounds.max_x - epsilon < border.max_x_at(now)
            && bounds.min_z >= border.min_z_at(now)
            && bounds.max_z - epsilon < border.max_z_at(now)
    })
}

fn ram_allowed(context: &MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
    if context.species.breed_goal_running {
        return false;
    }
    let items = super::spec::mob_spec(VANILLA_ENTITY_TYPE_GOAT_ID)
        .expect("goat specification")
        .goals
        .iter()
        .find_map(|(_, goal)| match goal {
            super::spec::GoalSpec::Tempt { items, .. } => Some(*items),
            _ => None,
        })
        .expect("goats have a temptation goal");
    super::tempt::nearest_tempting_player(context, targets, items, true).is_none()
}
