//! Idle destinations and movement restrictions.

use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::{
    vanilla_block_state_path_facts, vanilla_block_state_render_facts,
};
use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::entity::level::block_state_tag_membership;

use super::random_pos::{
    air_and_water_random_pos, default_random_pos, hover_random_pos, land_random_pos, RandomPosMob,
};
use super::{reduced_tick_delay, Goal, MobAiContext};

// `RandomStrollGoal.DEFAULT_INTERVAL`.
pub(super) const DEFAULT_INTERVAL: i32 = 120;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum StrollPosition {
    Default,
    Land,
    /// `RandomStroll.stroll(speed, maxXz, maxY)`.
    LandNear {
        xz: i32,
        y: i32,
    },
    Home {
        move_only: bool,
    },
    SavedHome,
    AroundSavedHome,
    VillageReturn,
    VillagePatrol,
    FoxVillage,
    TowardsTarget {
        target: Vec3d,
    },
    Swimming,
    Axolotl,
    WaterAvoiding {
        probability: f32,
    },
    Flying,
    AirAndWater,
    ParrotWander {
        probability: f32,
    },
    Turtle,
}

pub(crate) fn view_vector(x_rot_o: f32, y_head_rot_o: f32) -> Vec3d {
    use bbb_server_types::geometry::{mth_cos, mth_sin};
    let to_radians = (std::f64::consts::PI / 180.0) as f32;
    let real_x_rot = x_rot_o * to_radians;
    let real_y_rot = -y_head_rot_o * to_radians;
    let y_cos = mth_cos(f64::from(real_y_rot));
    let y_sin = mth_sin(f64::from(real_y_rot));
    let x_cos = mth_cos(f64::from(real_x_rot));
    let x_sin = mth_sin(f64::from(real_x_rot));
    Vec3d {
        x: f64::from(y_sin * x_cos),
        y: f64::from(-x_sin),
        z: f64::from(y_cos * x_cos),
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RandomStrollGoal {
    speed_modifier: f64,
    position: StrollPosition,
    interval: i32,
    next_stroll: i64,
    wanted: Vec3d,
}

impl RandomStrollGoal {
    pub(super) const fn new(speed_modifier: f64, position: StrollPosition, interval: i32) -> Self {
        Self {
            speed_modifier,
            position,
            interval,
            next_stroll: 0,
            wanted: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        }
    }

    fn choose_position(
        &mut self,
        context: &MobAiContext,
        targets: &super::target::TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(wanted) = get_position(
            targets,
            &context.random,
            RandomPosMob::of(targets.body_entity(), context.nav.malus),
            self.position,
        )
        .flatten() else {
            return false;
        };
        self.wanted = wanted;
        true
    }
}

fn fox_can_visit_village(
    context: &MobAiContext,
    targets: &super::target::TargetView<'_, '_, '_>,
) -> bool {
    context.target.target.is_none()
        && matches!(targets.body_entity().data(), crate::entity::EntityData::GeneralLiving(data)
            if data.fox_retained().is_some_and(|fox|
                !fox.is_sleeping() && !fox.is_sitting() && !fox.is_defending()))
}

impl Goal for RandomStrollGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        if !context.nav.stroll_allowed
            || !crate::entity::sniffer::allows_idle_stroll(targets.body_entity())
            || crate::entity::camel::refuses_to_move(targets.body_entity(), targets.game_time())
        {
            return false;
        }
        if self.position == StrollPosition::Axolotl && context.target.target.is_some() {
            return false;
        }
        if self.position == StrollPosition::Turtle && !super::turtle::may_stroll(targets) {
            return false;
        }
        if self.position == StrollPosition::VillageReturn {
            let pos = super::navigation::containing_block(context.body.position);
            if targets
                .level()
                .chunks()
                .sections_to_village([pos.x >> 4, pos.y >> 4, pos.z >> 4])
                .is_none_or(|distance| distance <= 1)
            {
                return false;
            }
        }
        let Some(controlling) = targets
            .store()
            .controlling_mob_passenger(targets.body_entity(), targets.level())
        else {
            return false;
        };
        if matches!(
            self.position,
            StrollPosition::SavedHome | StrollPosition::AroundSavedHome
        ) {
            if controlling.is_some()
                || context.target.target.is_some()
                || targets.game_time() < self.next_stroll
            {
                return false;
            }
            let Some(memory) = crate::entity::spawner_brain_memory(
                targets.body_entity(),
                crate::entity::ai::MemoryModuleType::Home,
            ) else {
                return false;
            };
            let crate::entity::ai::MemoryValue::GlobalPos(home) = &memory.value else {
                return false;
            };
            if home.dimension != targets.level().dimension_key().as_str() {
                return false;
            }
            let wanted = Vec3d {
                x: f64::from(home.pos.x) + 0.5,
                y: f64::from(home.pos.y),
                z: f64::from(home.pos.z) + 0.5,
            };
            let distance = (wanted.x - context.body.position.x).powi(2)
                + (wanted.y + 0.5 - context.body.position.y).powi(2)
                + (wanted.z - context.body.position.z).powi(2);
            let returning = self.position == StrollPosition::SavedHome;
            if distance >= if returning { 100.0 * 100.0 } else { 25.0 }
                || (returning && distance <= 4.0)
            {
                return false;
            }
            self.next_stroll = targets.game_time() + i64::from(self.interval);
            if returning {
                self.wanted = wanted;
                return true;
            }
            return self.choose_position(context, targets);
        }
        if matches!(self.position, StrollPosition::Home { .. }) {
            let Some((center, radius)) = targets.body_entity().mob_home() else {
                return false;
            };
            let pos = super::navigation::containing_block(context.body.position);
            let dx = f64::from(pos.x) - f64::from(center.x);
            let dy = f64::from(pos.y) - f64::from(center.y);
            let dz = f64::from(pos.z) - f64::from(center.z);
            if dx * dx + dy * dy + dz * dz < f64::from(radius) * f64::from(radius) {
                return false;
            }
        } else {
            if controlling.is_some() {
                return false;
            }
            if self.position == StrollPosition::Swimming
                && (!targets.body_entity().is_in_water()
                    || !crate::entity::fish::can_random_swim(
                        targets.body_entity(),
                        targets.store(),
                    ))
            {
                return false;
            }
            if matches!(
                self.position,
                StrollPosition::Land
                    | StrollPosition::LandNear { .. }
                    | StrollPosition::AirAndWater
                    | StrollPosition::Axolotl
            ) {
                // RandomStroll: only select a stroll without a walk target.
                if !context.nav.navigation_done {
                    return false;
                }
                if matches!(
                    self.position,
                    StrollPosition::Land | StrollPosition::LandNear { .. }
                ) {
                    if targets.game_time() < self.next_stroll {
                        return false;
                    }
                    self.next_stroll = targets.game_time() + i64::from(self.interval);
                }
            } else {
                if context.body.no_action_time >= 100
                    && !matches!(
                        self.position,
                        StrollPosition::VillageReturn
                            | StrollPosition::VillagePatrol
                            | StrollPosition::FoxVillage
                    )
                {
                    return false;
                }
                if context.next_int(reduced_tick_delay(self.interval)) != 0 {
                    return false;
                }
            }
        }
        if self.position == StrollPosition::FoxVillage {
            let pos = targets.body_entity().block_position();
            if !fox_can_visit_village(context, targets)
                || targets.bright_outside() != Some(false)
                || targets
                    .level()
                    .chunks()
                    .sections_to_village([pos.x >> 4, pos.y >> 4, pos.z >> 4])
                    .is_none_or(|distance| distance > 6)
            {
                return false;
            }
        }
        self.choose_position(context, targets)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        if !context.nav.stroll_allowed
            || !crate::entity::sniffer::allows_idle_stroll(targets.body_entity())
            || crate::entity::camel::refuses_to_move(targets.body_entity(), targets.game_time())
        {
            return false;
        }
        if self.position == StrollPosition::FoxVillage && !fox_can_visit_village(context, targets) {
            return false;
        }
        !context.nav.navigation_done
            && (matches!(self.position, StrollPosition::Home { .. })
                || targets
                    .store()
                    .controlling_mob_passenger(targets.body_entity(), targets.level())
                    .is_some_and(|controlling| controlling.is_none()))
            && (!matches!(
                self.position,
                StrollPosition::SavedHome
                    | StrollPosition::AroundSavedHome
                    | StrollPosition::Axolotl
            ) || context.target.target.is_none())
    }

    fn start(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        let wanted = self.wanted;
        super::breeze::idle_slide(context, targets, true);
        if self.position == StrollPosition::FoxVillage {
            context.species.fox_sleep = Some(false);
        }
        if self.position == StrollPosition::SavedHome {
            context.try_navigate_to_position(wanted, 2, self.speed_modifier);
        } else if self.position == StrollPosition::Axolotl {
            let speed = f64::from(if targets.body_entity().is_in_water() {
                0.5_f32
            } else {
                0.15_f32
            });
            context.try_navigate_to_position(wanted, 0, speed);
        } else if matches!(
            self.position,
            StrollPosition::Land | StrollPosition::LandNear { .. } | StrollPosition::AirAndWater
        ) {
            context.try_navigate_to_position(wanted, 0, self.speed_modifier);
        } else {
            context.navigate_to_position(wanted, self.speed_modifier);
        }
    }

    fn stop(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        context.stop_navigation();
        super::breeze::idle_slide(context, targets, false);
    }
}

pub(crate) fn level_weighs_positions(level: &LevelCtx<'_, '_>) -> bool {
    level.dimension_ambient_light().is_some()
}

pub(super) fn get_position(
    targets: &super::target::TargetView<'_, '_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    request: StrollPosition,
) -> Option<Option<Vec3d>> {
    let level = targets.level();
    let probability = match request {
        StrollPosition::Default | StrollPosition::Turtle => {
            return default_random_pos(level, random, mob, 10, 7)
        }
        StrollPosition::Land => {
            // RandomStroll: land targets are also selected while in water.
            return land_random_pos(level, random, mob, 10, 7);
        }
        StrollPosition::LandNear { xz, y } => {
            return land_random_pos(level, random, mob, xz, y);
        }
        StrollPosition::SavedHome => return Some(None),
        StrollPosition::AroundSavedHome => return land_random_pos(level, random, mob, 8, 6),
        StrollPosition::VillageReturn => {
            let pos = super::navigation::containing_block(mob.position);
            let Some(center) = level.chunks().closest_village_section(pos) else {
                return Some(None);
            };
            return super::random_pos::default_random_pos_towards(
                level,
                random,
                mob,
                10,
                7,
                Vec3d {
                    x: f64::from(center.x) + 0.5,
                    y: f64::from(center.y),
                    z: f64::from(center.z) + 0.5,
                },
                f64::from(std::f32::consts::FRAC_PI_2),
            );
        }
        StrollPosition::VillagePatrol => {
            return super::iron_golem::patrol_position(targets, random, mob);
        }
        StrollPosition::FoxVillage => {
            return super::random_pos::land_random_pos_scored(level, random, mob, 15, 7, |pos| {
                level
                    .chunks()
                    .sections_to_village([pos.x >> 4, pos.y >> 4, pos.z >> 4])
                    .map(|distance| -f64::from(distance))
            });
        }
        StrollPosition::TowardsTarget { target } => {
            return super::random_pos::default_random_pos_towards(
                level,
                random,
                mob,
                16,
                7,
                target,
                f64::from(std::f32::consts::FRAC_PI_2),
            );
        }
        StrollPosition::Home { .. } => {
            let Some((center, _)) = mob.home else {
                return Some(None);
            };
            return super::random_pos::default_random_pos_towards(
                level,
                random,
                mob,
                16,
                7,
                Vec3d {
                    x: f64::from(center.x) + 0.5,
                    y: f64::from(center.y),
                    z: f64::from(center.z) + 0.5,
                },
                f64::from(std::f32::consts::FRAC_PI_2),
            );
        }
        StrollPosition::Swimming => {
            return super::random_pos::random_swimmable_pos(level, random, mob, 10, 7);
        }
        StrollPosition::Axolotl => {
            // AxolotlAi; RandomStroll.
            return if targets.body_entity().is_in_water() {
                super::random_pos::swim_target_pos(level, random, mob)
            } else {
                land_random_pos(level, random, mob, 10, 7)
            };
        }
        StrollPosition::WaterAvoiding { probability } => probability,
        StrollPosition::Flying => {
            return flying_position(level, random, mob, look_direction(targets.body_entity()))
        }
        StrollPosition::AirAndWater => {
            let direction = look_direction(targets.body_entity());
            return air_and_water_random_pos(
                level,
                random,
                mob,
                10,
                7,
                -2,
                direction.x,
                direction.z,
                f64::from(std::f64::consts::FRAC_PI_2 as f32),
            );
        }
        StrollPosition::ParrotWander { probability } => {
            return parrot_wander_position(targets, random, mob, probability);
        }
    };
    if targets.body_entity().is_in_water() {
        return match land_random_pos(level, random, mob, 15, 7)? {
            Some(pos) => Some(Some(pos)),
            None => default_random_pos(level, random, mob, 10, 7),
        };
    }
    if random.next_float() >= probability {
        land_random_pos(level, random, mob, 10, 7)
    } else {
        default_random_pos(level, random, mob, 10, 7)
    }
}

/// `WaterAvoidingRandomFlyingGoal.getPosition`, which `BeeWanderGoal.findPos`
/// repeats with its own heading.
pub(super) fn flying_position(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    direction: Vec3d,
) -> Option<Option<Vec3d>> {
    let max_xz_radians = f64::from(std::f64::consts::FRAC_PI_2 as f32);
    let hover = hover_random_pos(
        level,
        random,
        mob,
        8,
        7,
        direction.x,
        direction.z,
        max_xz_radians,
        3,
        1,
    )?;
    if hover.is_some() {
        return Some(hover);
    }
    air_and_water_random_pos(
        level,
        random,
        mob,
        8,
        4,
        -2,
        direction.x,
        direction.z,
        max_xz_radians,
    )
}

fn look_direction(entity: &crate::entity::Entity) -> Vec3d {
    view_vector(
        entity.x_rot(),
        entity
            .living()
            .expect("strolling mob is living")
            .y_head_rot_o,
    )
}

fn parrot_wander_position(
    targets: &super::target::TargetView<'_, '_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    probability: f32,
) -> Option<Option<Vec3d>> {
    let level = targets.level();
    let mut pos = None;
    if targets.body_entity().is_in_water() {
        pos = land_random_pos(level, random, mob, 15, 15)?;
    }
    if random.next_float() >= probability {
        pos = parrot_tree_pos(level, mob.position)?;
    }
    match pos {
        Some(pos) => Some(Some(pos)),
        None => flying_position(level, random, mob, look_direction(targets.body_entity())),
    }
}

fn parrot_tree_pos(level: &LevelCtx<'_, '_>, position: Vec3d) -> Option<Option<Vec3d>> {
    let mob_pos = super::navigation::containing_block(position);
    let (min_x, max_x) = (
        (position.x - 3.0).floor() as i32,
        (position.x + 3.0).floor() as i32,
    );
    let (min_y, max_y) = (
        (position.y - 6.0).floor() as i32,
        (position.y + 6.0).floor() as i32,
    );
    let (min_z, max_z) = (
        (position.z - 3.0).floor() as i32,
        (position.z + 3.0).floor() as i32,
    );
    for z in min_z..=max_z {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let pos = BlockPos { x, y, z };
                if pos == mob_pos {
                    continue;
                }
                let below = BlockPos { y: y - 1, ..pos };
                let above = BlockPos { y: y + 1, ..pos };
                if !level.has_ticking_column_at(below) {
                    return None;
                }
                let below_state = level.block_state(below);
                let is_leaves = if let Some(facts) = vanilla_block_state_render_facts(below_state) {
                    facts.leaves_block()
                } else {
                    level.content_registry().block_by_state_id(below_state)?;
                    false
                };
                let can_sit_on =
                    is_leaves || block_state_tag_membership(level, below_state, "minecraft:logs");
                if can_sit_on && is_air(level, pos)? && is_air(level, above)? {
                    return Some(Some(Vec3d {
                        x: f64::from(x) + 0.5,
                        y: f64::from(y),
                        z: f64::from(z) + 0.5,
                    }));
                }
            }
        }
    }
    Some(None)
}

fn is_air(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    if !level.has_ticking_column_at(pos) {
        return None;
    }
    let state = level.block_state(pos);
    if let Some(facts) = vanilla_block_state_path_facts(state) {
        Some(facts.is_air())
    } else {
        level.content_registry().block_by_state_id(state)?;
        Some(false)
    }
}
