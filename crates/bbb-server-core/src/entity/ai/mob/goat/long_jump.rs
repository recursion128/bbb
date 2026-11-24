//! Goat long jumps using live landing support, navigation and collision queries.

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_HONEY_BLOCK_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_LONG_JUMPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID;
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};
use bbb_server_types::geometry::{
    long_jump_transition_clear, long_jump_velocity, no_block_collision_with_moving_pistons, Aabb,
    Vec3Ops,
};

use crate::entity::ai::MemoryModuleType;
use crate::entity::{general_living, living, EntityData};

use super::super::navigation::{
    containing_block, random_pos_stable_destination, walk_path_type, GroundNavigation,
    MobPathingSnapshot,
};
use super::super::target::TargetView;
use super::super::{Goal, MobAiContext};

#[derive(Debug, Clone, Copy)]
struct JumpCandidate {
    target: BlockPos,
    movement: Vec3d,
    weight: i32,
}

#[derive(Debug, Clone)]
pub(crate) struct LongJumpPathProbe {
    candidates: Vec<JumpCandidate>,
    selected: Option<JumpCandidate>,
    pending: bool,
}

pub(crate) fn resolve_path(
    context: &mut MobAiContext,
    targets: &mut TargetView<'_, '_, '_>,
    navigation: &mut GroundNavigation,
    mob: Option<MobPathingSnapshot>,
) -> Option<bool> {
    if !context.nav.goat_long_jump_path.as_ref()?.pending {
        return None;
    }
    let mob = mob.unwrap_or_else(|| MobPathingSnapshot::from_context(context));
    let probe = context.nav.goat_long_jump_path.as_mut()?;
    probe.pending = false;
    let level = targets.level();
    // Bound A* reachability probes per tick to Vanilla's FIND_JUMP_TRIES budget (20).
    let mut remaining_tries = 20_u32;
    while remaining_tries > 0 && !probe.candidates.is_empty() {
        remaining_tries -= 1;
        let mut weight = context.random.next_int(
            probe
                .candidates
                .iter()
                .map(|candidate| candidate.weight)
                .sum(),
        );
        let index = probe
            .candidates
            .iter()
            .position(|candidate| {
                weight -= candidate.weight;
                weight < 0
            })
            .expect("positive jump candidate weights");
        let candidate = probe.candidates.swap_remove(index);
        // LongJumpToRandomPos: jump only where walking cannot reach.
        if !navigation
            .create_path_to(level, mob, candidate.target, 0, Some(8.0))
            .is_some_and(|path| path.reached())
        {
            probe.selected = Some(candidate);
            break;
        }
    }
    let admitted = probe.selected.is_some();
    if !admitted {
        cooldown(targets, true);
    }
    Some(admitted)
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Preparing {
        origin: Vec3d,
        jump: JumpCandidate,
        until: i64,
    },
    Airborne {
        launched_at: i64,
    },
}

#[derive(Debug, Default, Clone)]
pub(crate) struct GoatLongJumpGoal {
    phase: Option<Phase>,
    next_scan: i64,
}

impl Goal for GoatLongJumpGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if targets.game_time() < self.next_scan
            || crate::entity::goat::cooldown_active(
                targets.body_entity(),
                MemoryModuleType::LongJumpCooldownTicks,
            )
            || !super::ram_allowed(context, targets)
            || !context.nav.navigation_done
            || targets.body_entity().pose_id() == VANILLA_ENTITY_POSE_LONG_JUMPING_ID
        {
            return false;
        }
        self.phase = None;
        if !context.body.on_ground
            || context.body.in_water
            || context.body.in_lava
            || vanilla_block_state_block_type_id(
                targets
                    .level()
                    .block_state(containing_block(context.body.position)),
            ) == Some(VANILLA_BLOCK_HONEY_BLOCK_ID)
        {
            cooldown(targets, true);
            return false;
        }
        let candidates = jump_candidates(context, targets);
        if candidates.is_empty() {
            cooldown(targets, true);
            return false;
        }
        context.nav.goat_long_jump_path = Some(LongJumpPathProbe {
            candidates,
            selected: None,
            pending: true,
        });
        true
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(jump) = context
            .nav
            .goat_long_jump_path
            .take()
            .and_then(|probe| probe.selected)
        else {
            return;
        };
        context.stop_navigation();
        self.phase = Some(Phase::Preparing {
            origin: context.body.position,
            jump,
            until: targets.game_time().saturating_add(40),
        });
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        match self.phase {
            Some(Phase::Preparing { origin, .. }) => {
                context.body.position == origin
                    && !context.body.in_water
                    && super::ram_allowed(context, targets)
            }
            Some(Phase::Airborne { .. }) => true,
            None => false,
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.finish(context, targets);
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let now = targets.game_time();
        match self.phase {
            Some(Phase::Preparing {
                origin,
                jump,
                until,
            }) => {
                if context.body.position != origin
                    || context.body.in_water
                    || !super::ram_allowed(context, targets)
                {
                    self.finish(context, targets);
                    return;
                }
                context.stop_navigation();
                let target = center(jump.target);
                context.set_look_at(target.x, target.y, target.z);
                if now < until {
                    return;
                }
                // LongJumpToRandomPos: boost scales the whole vector.
                let boost = targets
                    .body_entity()
                    .living()
                    .and_then(|body| body.active_effect("minecraft:jump_boost"))
                    .map_or(0.0, |effect| {
                        f64::from(0.1_f32 * (effect.amplifier() as f32 + 1.0))
                    });
                let length = jump.movement.length_sqr().sqrt();
                let movement = jump.movement.scale((length + boost) / length);
                context.set_body_y_rot(context.body.y_body_rot);
                targets.set_body_discard_friction(true);
                targets.set_pose(VANILLA_ENTITY_POSE_LONG_JUMPING_ID);
                targets.set_body_delta_movement(movement);
                let screaming = matches!(
                    targets.body_entity().data(),
                    EntityData::GeneralLiving(data)
                        if data.goat_retained().is_some_and(|goat| goat.is_screaming())
                );
                sound(
                    targets,
                    if screaming {
                        "minecraft:entity.goat.screaming.long_jump"
                    } else {
                        "minecraft:entity.goat.long_jump"
                    },
                    1.0,
                );
                self.phase = Some(Phase::Airborne { launched_at: now });
            }
            Some(Phase::Airborne { launched_at })
                if now > launched_at && (context.body.on_ground || now - launched_at >= 100) =>
            {
                self.finish(context, targets);
            }
            Some(Phase::Airborne { .. }) | None => {}
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

impl GoatLongJumpGoal {
    fn finish(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(phase) = self.phase.take() else {
            return;
        };
        self.next_scan = targets.game_time().saturating_add(1);
        context.stop_navigation();
        if let Phase::Airborne { launched_at } = phase {
            if context.body.on_ground && targets.game_time() > launched_at {
                let movement = targets.body_entity().delta_movement();
                targets.set_body_delta_movement(Vec3d {
                    x: movement.x * f64::from(0.1_f32),
                    z: movement.z * f64::from(0.1_f32),
                    ..movement
                });
                sound(targets, "minecraft:entity.goat.step", 2.0);
            }
            targets.set_body_discard_friction(false);
            targets.set_pose(VANILLA_ENTITY_POSE_STANDING_ID);
        }
        cooldown(targets, matches!(phase, Phase::Preparing { .. }));
    }
}

fn cooldown(targets: &mut TargetView<'_, '_, '_>, failed: bool) {
    if let Some((entity, _, level)) = targets.world_mut() {
        crate::entity::goat::reset_long_jump_cooldown(entity, level, failed);
    }
}

fn sound(targets: &mut TargetView<'_, '_, '_>, resource_id: &'static str, volume: f32) {
    if !targets.body_entity().is_silent() {
        targets.play_body_sound(resource_id, SoundSource::Neutral, volume, 1.0);
    }
}

fn center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

fn jump_candidates(context: &MobAiContext, targets: &TargetView<'_, '_, '_>) -> Vec<JumpCandidate> {
    let Some([width, height, _]) = general_living::dimensions::geometry(
        targets.body_entity().data(),
        VANILLA_ENTITY_TYPE_GOAT_ID,
        VANILLA_ENTITY_POSE_LONG_JUMPING_ID,
    ) else {
        return Vec::new();
    };
    let Some(body) = targets.body_entity().living() else {
        return Vec::new();
    };
    let max_speed = f64::from(
        (body.attribute_value(VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID) * f64::from(3.571_428_8_f32))
            as f32,
    );
    let gravity = living::gravity(targets.body_entity());
    let mut angles = [65, 70, 75, 80];
    for index in (1..angles.len()).rev() {
        angles.swap(index, context.random.next_int(index as i32 + 1) as usize);
    }
    let origin = containing_block(context.body.position);
    let mut candidates = Vec::new();
    for dz in -5..=5 {
        for dx in -5..=5 {
            if dx == 0 && dz == 0 {
                continue;
            }
            for dy in -5..=5 {
                let target = BlockPos {
                    x: origin.x + dx,
                    y: origin.y + dy,
                    z: origin.z + dz,
                };
                if random_pos_stable_destination(
                    targets.level(),
                    VANILLA_ENTITY_TYPE_GOAT_ID,
                    target,
                ) != Some(true)
                    || !walk_path_type(targets.level(), target)
                        .is_some_and(|path| context.nav.malus.get(path) == 0.0)
                {
                    continue;
                }
                let mut transition = |from, to| {
                    long_jump_transition_clear(
                        from,
                        to,
                        f64::from(width),
                        f64::from(height),
                        |bounds: Aabb| {
                            no_block_collision_with_moving_pistons(
                                bounds,
                                |pos| {
                                    targets.level().has_ticking_column_at(pos).then_some(())?;
                                    targets.level().is_moving_piston_at(pos)
                                },
                                |pos| {
                                    targets.level().has_ticking_column_at(pos).then_some(())?;
                                    targets.level().block_collision_boxes_for_entity(
                                        targets.body_entity(),
                                        pos,
                                    )
                                },
                            ) == Some(true)
                        },
                    )
                };
                if let Some(movement) = angles.into_iter().find_map(|angle| {
                    long_jump_velocity(
                        context.body.position,
                        center(target),
                        max_speed,
                        angle,
                        gravity,
                        Some(&mut transition),
                    )
                }) {
                    candidates.push(JumpCandidate {
                        target,
                        movement,
                        weight: dx * dx + dy * dy + dz * dz,
                    });
                }
            }
        }
    }
    candidates
}
