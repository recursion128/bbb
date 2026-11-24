use crate::world::simulation_level::LevelCtx;
use std::fmt;

use crate::random::GameRandom;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::packets::{BlockPos, DebugGoalInfo, DebugPathInfo, Vec3d};
use bbb_server_types::geometry::Aabb;

use crate::entity::bee::{BeeCustomStep, BeeRuntime};
use crate::entity::blaze::{BlazeAiStepResult, BlazeRuntime};
use crate::entity::level::NearbyPlayer;
use crate::entity::living::LivingEntityState;
use crate::entity::piglin::PiglinRuntime;
use crate::entity::witch::{
    RangedAttackAction as WitchRangedAttackAction, SelfPotionInput, SelfPotionStep, WitchRuntime,
};
use crate::entity::wither::WitherRuntime;
use crate::entity::Entity;

use super::{Brain, Goal, GoalControl, MemoryModuleType, MemoryValue};
use bat::{BatAiStepResult, BatRuntime};
use body_rotation::{normalize_retained_rotations, BodyRotationControl};
use door::{BreakDoorState, DoorContext, DoorEffect, VindicatorDoors, ZombieDoors};
use ghast::GhastRuntime;
use goal_selector::GoalSelector;
use jump_control::JumpControl;
use patrol::PatrolStepResult;
use phantom::{PhantomAiStepResult, PhantomRuntime};
use rabbit::RabbitRuntime;
use random_look::LookControl;
use run_around_like_crazy::RunAroundLikeCrazyOutcome;
use sensing::Sensing;
use sheep::GrazingAction;
use slime::{SlimeJumpSound, SlimeRuntime};
use spec::{mob_spec, GoalSpec, MobGoal, MobSpec};
use squid::SquidRuntime;
use vex::{VexRuntime, VexWantedPosition};

use navigation::malus::PathfindingMalus;
use navigation::{
    GroundNavigation, GroundPath, MobNavigation, MobPathingSnapshot, MoveControl, MoveControlKind,
};

/// `Villager`'s navigation minimum.
pub(crate) const VILLAGER_REQUIRED_PATH_LENGTH: f32 = 48.0;

pub(crate) fn has_owned_navigation(entity_type_id: i32) -> bool {
    mob_spec(entity_type_id).is_some_and(|spec| spec.navigation.is_some())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::entity::ai::mob) struct LookTarget {
    x: f64,
    y: f64,
    z: f64,
}

// `Goal.reducedTickDelay`.
pub(crate) const fn reduced_tick_delay(ticks: i32) -> i32 {
    (ticks + 1) / 2
}

#[derive(Debug, Clone)]
pub(crate) struct MobBodyState {
    pub(in crate::entity::ai::mob) position: Vec3d,
    pub(in crate::entity::ai::mob) creaking_home_pos: Option<BlockPos>,
    pub(in crate::entity::ai::mob) bounding_box: Aabb,
    pub(in crate::entity::ai::mob) eye_y: f64,
    pub(in crate::entity::ai::mob) x_rot: f32,
    pub(in crate::entity::ai::mob) y_rot: f32,
    pub(in crate::entity::ai::mob) y_body_rot: f32,
    pub(in crate::entity::ai::mob) y_head_rot: f32,
    pub(crate) delta_movement: Vec3d,
    pub(in crate::entity::ai::mob) on_ground: bool,
    pub(in crate::entity::ai::mob) in_liquid: bool,
    pub(in crate::entity::ai::mob) in_water: bool,
    pub(in crate::entity::ai::mob) in_lava: bool,
    pub(in crate::entity::ai::mob) on_fire: bool,
    pub(in crate::entity::ai::mob) eye_in_water: bool,
    pub(in crate::entity::ai::mob) water_fluid_height: f64,
    pub(in crate::entity::ai::mob) is_passenger: bool,
    pub(in crate::entity::ai::mob) width: f32,
    pub(in crate::entity::ai::mob) movement_speed: f64,
    pub(in crate::entity::ai::mob) flying_speed: f64,
    pub(crate) follow_range: f64,
    pub(in crate::entity::ai::mob) step_height: f64,
    pub(in crate::entity::ai::mob) tempt_range: f64,
    pub(in crate::entity::ai::mob) health: f32,
    pub(in crate::entity::ai::mob) max_health: f32,
    pub(in crate::entity::ai::mob) immobile: bool,
    pub(in crate::entity::ai::mob) no_action_time: i32,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MobControlState {
    pub(crate) xxa: f32,
    pub(crate) yya: f32,
    pub(crate) zza: f32,
    pub(crate) speed: f32,
    pub(crate) jumping: bool,
    pub(in crate::entity::ai::mob) jump_requested: bool,
    pub(in crate::entity::ai::mob) look_target: Option<LookTarget>,
    pub(in crate::entity::ai::mob) look_limits: Option<(f32, f32)>,
    pub(in crate::entity::ai::mob) strafe_request: Option<(f32, f32)>,
    pub(in crate::entity::ai::mob) move_control_request: Option<navigation::NavigationMove>,
    pub(in crate::entity::ai::mob) no_gravity_request: Option<bool>,
    pub(in crate::entity::ai::mob) teleport: Option<Vec3d>,
    pub(in crate::entity::ai::mob) swimming_impulse: Vec3d,
    pub(in crate::entity::ai::mob) flight_y_rot: f32,
    pub(in crate::entity::ai::mob) body_x_rot_override: Option<f32>,
    pub(in crate::entity::ai::mob) body_y_rot_override: Option<f32>,
}

#[derive(Debug, Clone)]
pub(crate) struct MobNavState {
    pub(in crate::entity::ai::mob) malus: PathfindingMalus,
    pub(in crate::entity::ai::mob) inherits_malus: bool,
    pub(in crate::entity::ai::mob) own_malus_writes: Vec<(crate::entity::GroundPathType, f32)>,
    pub(in crate::entity::ai::mob) navigation_done: bool,
    pub(in crate::entity::ai::mob) last_navigation_failed: bool,
    pub(in crate::entity::ai::mob) navigation_target: Option<BlockPos>,
    pub(in crate::entity::ai::mob) navigation_request: Option<NavigationRequest>,
    pub(in crate::entity::ai::mob) navigation_speed_request: Option<f64>,
    pub(in crate::entity::ai::mob) avoid_sun_request: Option<bool>,
    pub(in crate::entity::ai::mob) stroll_allowed: bool,
    pub(in crate::entity::ai::mob) door: Option<DoorContext>,
    pub(in crate::entity::ai::mob) owner_teleport: Option<BlockPos>,
    pub(in crate::entity::ai::mob) patrol_cooldown_until: i64,
    pub(in crate::entity::ai::mob) patrol_request: Option<(BlockPos, bool)>,
    pub(in crate::entity::ai::mob) patrol_result: Option<PatrolStepResult>,
    pub(in crate::entity::ai::mob) goat_ram_path: Option<goat::RamPathProbe>,
    pub(in crate::entity::ai::mob) goat_long_jump_path: Option<goat::long_jump::LongJumpPathProbe>,
}

#[derive(Debug, Clone)]
pub(crate) struct MobSpeciesState {
    pub(in crate::entity::ai::mob) drowned_swim: drowned::SwimIntent,
    pub(in crate::entity::ai::mob) horse_stand_requested: bool,
    pub(in crate::entity::ai::mob) run_around_outcome: Option<RunAroundLikeCrazyOutcome>,
    pub(in crate::entity::ai::mob) enderman_teleports: Vec<enderman::teleport::Request>,
    pub(in crate::entity::ai::mob) squid_movement_vector: Option<Vec3d>,
    pub(in crate::entity::ai::mob) vex_random_move_requested: bool,
    pub(in crate::entity::ai::mob) vex_wanted_position: Option<VexWantedPosition>,
    pub(in crate::entity::ai::mob) creeper_swell_dir: Option<i32>,
    pub(in crate::entity::ai::mob) wolf_interested: Option<bool>,
    pub(in crate::entity::ai::mob) cat_lying: Option<bool>,
    pub(in crate::entity::ai::mob) cat_relaxing: Option<bool>,
    pub(in crate::entity::ai::mob) cat_sitting: Option<bool>,
    pub(in crate::entity::ai::mob) fox_sleep: Option<bool>,
    pub(in crate::entity::ai::mob) fox_reset_pitch: bool,
    pub(in crate::entity::ai::mob) fox_can_look: bool,
    pub(in crate::entity::ai::mob) caravan_change: Option<crate::entity::llama::CaravanChange>,
    pub(in crate::entity::ai::mob) raider_banner: Option<raider_banner::RaiderBannerState>,
    pub(in crate::entity::ai::mob) piglin_idle_allowed: bool,
    pub(in crate::entity::ai::mob) piglin_ride_target: Option<target::TargetRef>,
    pub(in crate::entity::ai::mob) ghast_movement_allowed: bool,
    pub(in crate::entity::ai::mob) ghast_careful: bool,
    pub(in crate::entity::ai::mob) ghast_wanted_position: Option<Vec3d>,
    pub(crate) slime_in_fluid: bool,
    pub(crate) slime_wanted_movement: Option<f64>,
    pub(crate) slime_direction: Option<(f32, bool)>,
    pub(crate) sheep_action: Option<GrazingAction>,
    pub(crate) blaze_charged: bool,
    pub(crate) blaze_action: Option<crate::entity::blaze::BlazeAttackAction>,
    pub(in crate::entity::ai::mob) player_attack_allowed: bool,
    pub(crate) witch_action: Option<WitchRangedAttackAction>,
    pub(crate) breed_goal_running: bool,
}

/// Deferred actions that require `MobAiRuntime` to be returned to `EntityStore`
/// (e.g. damage/thorns or target runtime queries) after `finish_server_ai_step`.
#[derive(Debug, Clone)]
pub(crate) enum MobPostStepAction {
    MeleeAttack {
        target_id: i32,
    },
    RangedAttack {
        target: ranged::RangedTarget,
        power: f32,
    },
    GuardianAttack {
        target_id: i32,
    },
    SpearStab {
        strikes: Vec<(i32, crate::entity::spear::KineticStrike)>,
    },
    GhastShoot {
        origin: Vec3d,
        direction: Vec3d,
    },
    ShulkerShoot {
        target: ranged::RangedTarget,
        attach_axis: Option<u8>,
    },
    CastSpell(ranged::SpellCast),
    GoatRamAttack {
        target_id: i32,
        knockback: f64,
        direction: Vec3d,
    },
    WardenSonicFire {
        target_id: i32,
    },
    WardenSniffEnd,
    AllayDeliverItem,
    SpawnChildFromBreeding {
        partner_id: i32,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct MobAiContext {
    pub(crate) entity_id: i32,
    pub(crate) tick_count: i32,
    pub(in crate::entity::ai::mob) difficulty: Option<Difficulty>,
    pub(in crate::entity::ai::mob) random: GameRandom,
    pub(crate) body: MobBodyState,
    pub(crate) controls: MobControlState,
    pub(crate) nav: MobNavState,
    pub(crate) species: MobSpeciesState,
    pub(crate) target: target::TargetPhase,
    pub(crate) ranged: ranged::RangedState,
    pub(in crate::entity::ai::mob) post_step: Vec<MobPostStepAction>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::entity::ai::mob) enum NavigationRequest {
    MoveToEntity {
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    },
    MoveToPosition {
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    },
    MoveToPositionAfterStop {
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    },
    /// A `createPath` the asking goal's `canUse` depends on: no path withdraws
    /// the admission and keeps the current one.
    TryMoveToPosition {
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    },
    /// A read-only `createPath` admission check for `canUse`, leaving the live
    /// path untouched until the goal's own `start()` installs it.
    RequirePath {
        target: BlockPos,
        reach_range: i32,
    },
    ThroughVillage {
        target: BlockPos,
        speed_modifier: f64,
        can_break_doors: bool,
    },
    Stop,
}

impl MobAiContext {
    #[cfg(test)]
    fn from_living(
        entity_id: i32,
        tick_count: i32,
        position: Vec3d,
        eye_y: f64,
        x_rot: f32,
        living: &LivingEntityState,
        random: GameRandom,
    ) -> Self {
        Self::from_living_body(
            entity_id,
            tick_count,
            position,
            Aabb::of_entity(position, 0.0, 0.0),
            eye_y,
            x_rot,
            living.y_body_rot,
            false,
            false,
            false,
            living,
            random,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_living_body(
        entity_id: i32,
        tick_count: i32,
        position: Vec3d,
        bounding_box: Aabb,
        eye_y: f64,
        x_rot: f32,
        y_rot: f32,
        on_ground: bool,
        in_liquid: bool,
        is_passenger: bool,
        living: &LivingEntityState,
        random: GameRandom,
    ) -> Self {
        Self {
            entity_id,
            tick_count,
            difficulty: None,
            random,
            body: MobBodyState {
                position,
                creaking_home_pos: None,
                bounding_box,
                eye_y,
                x_rot,
                y_rot,
                y_body_rot: living.y_body_rot,
                y_head_rot: living.y_head_rot,
                delta_movement: Vec3d::default(),
                on_ground,
                in_liquid,
                in_water: false,
                in_lava: false,
                on_fire: false,
                eye_in_water: false,
                water_fluid_height: 0.0,
                is_passenger,
                width: (bounding_box.max_x - bounding_box.min_x) as f32,
                movement_speed: living.attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                ),
                flying_speed: living.attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FLYING_SPEED_ID,
                ),
                follow_range: living.attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID,
                ),
                step_height: living.attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_STEP_HEIGHT_ID,
                ),
                tempt_range: living.attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_TEMPT_RANGE_ID,
                ),
                health: living.health(),
                max_health: living.max_health(),
                immobile: living.is_immobile(),
                no_action_time: living.no_action_time,
            },
            controls: MobControlState {
                xxa: living.xxa,
                yya: living.yya,
                zza: living.zza,
                speed: living.speed(),
                jumping: living.jumping,
                ..MobControlState::default()
            },
            nav: MobNavState {
                malus: PathfindingMalus::default(),
                inherits_malus: false,
                own_malus_writes: Vec::new(),
                navigation_done: true,
                last_navigation_failed: false,
                navigation_target: None,
                navigation_request: None,
                navigation_speed_request: None,
                avoid_sun_request: None,
                stroll_allowed: true,
                door: None,
                owner_teleport: None,
                patrol_cooldown_until: -1,
                patrol_request: None,
                patrol_result: None,
                goat_ram_path: None,
                goat_long_jump_path: None,
            },
            species: MobSpeciesState {
                drowned_swim: drowned::SwimIntent::default(),
                horse_stand_requested: false,
                run_around_outcome: None,
                enderman_teleports: Vec::new(),
                squid_movement_vector: None,
                vex_random_move_requested: false,
                vex_wanted_position: None,
                creeper_swell_dir: None,
                wolf_interested: None,
                cat_lying: None,
                cat_relaxing: None,
                cat_sitting: None,
                fox_sleep: None,
                fox_reset_pitch: true,
                fox_can_look: true,
                caravan_change: None,
                raider_banner: None,
                piglin_idle_allowed: false,
                piglin_ride_target: None,
                ghast_movement_allowed: false,
                ghast_careful: false,
                ghast_wanted_position: None,
                slime_in_fluid: false,
                slime_wanted_movement: None,
                slime_direction: None,
                sheep_action: None,
                blaze_charged: false,
                blaze_action: None,
                player_attack_allowed: true,
                witch_action: None,
                breed_goal_running: false,
            },
            target: target::TargetPhase::default(),
            ranged: ranged::RangedState::default(),
            post_step: Vec::new(),
        }
    }

    fn apply(&self, living: &mut LivingEntityState) {
        living.xxa = self.controls.xxa;
        living.yya = self.controls.yya;
        living.zza = self.controls.zza;
        living.set_speed(self.controls.speed);
        living.jumping = self.controls.jumping;
        living.y_body_rot = self.body.y_body_rot;
        living.y_head_rot = self.body.y_head_rot;
    }

    pub(crate) fn next_float(&mut self) -> f32 {
        self.random.next_float()
    }

    fn next_double(&mut self) -> f64 {
        self.random.next_double()
    }

    fn next_int(&mut self, bound: i32) -> i32 {
        self.random.next_int(bound)
    }

    const fn is_immobile(&self) -> bool {
        self.body.immobile
    }

    fn set_look_at(&mut self, x: f64, y: f64, z: f64) {
        self.controls.look_target = Some(LookTarget { x, y, z });
        self.controls.look_limits = None;
    }

    pub(crate) fn clear_look_target(&mut self) {
        self.controls.look_target = None;
        self.controls.look_limits = None;
    }

    fn set_look_at_with_limits(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        y_max_rot_speed: f32,
        x_max_rot_angle: f32,
    ) {
        self.controls.look_target = Some(LookTarget { x, y, z });
        self.controls.look_limits = Some((y_max_rot_speed, x_max_rot_angle));
    }

    pub(crate) fn request_jump(&mut self) {
        self.controls.jump_requested = true;
    }

    pub(crate) fn set_move_wanted(&mut self, x: f64, y: f64, z: f64, speed_modifier: f64) {
        self.controls.move_control_request = Some(navigation::NavigationMove {
            wanted: Vec3d { x, y, z },
            speed_modifier,
        });
    }

    fn request_horse_stand(&mut self) {
        self.species.horse_stand_requested = true;
    }

    pub(super) fn set_body_x_rot(&mut self, x_rot: f32) {
        self.controls.body_x_rot_override = Some(x_rot);
    }

    pub(super) fn set_body_y_rot(&mut self, y_rot: f32) {
        self.controls.body_y_rot_override = Some(y_rot);
    }

    pub(crate) const fn position(&self) -> Vec3d {
        self.body.position
    }

    pub(crate) const fn eye_y(&self) -> f64 {
        self.body.eye_y
    }

    pub(crate) fn triangle(&mut self, mean: f64, spread: f64) -> f64 {
        mean + spread * (self.next_double() - self.next_double())
    }

    pub(crate) fn set_look_at_player_with_limits(
        &mut self,
        target: NearbyPlayer,
        y_max_rot_speed: f32,
        x_max_rot_angle: f32,
    ) {
        self.set_look_at_with_limits(
            target.position.x,
            target.position.y + target.eye_height,
            target.position.z,
            y_max_rot_speed,
            x_max_rot_angle,
        );
    }

    fn navigate_to(&mut self, target: Vec3d, speed_modifier: f64) {
        self.nav.navigation_request = Some(NavigationRequest::MoveToEntity {
            target: navigation::containing_block(target),
            reach_range: 1,
            speed_modifier,
        });
    }

    fn navigate_to_position(&mut self, wanted: Vec3d, speed_modifier: f64) {
        let target = navigation::containing_block(wanted);
        self.nav.navigation_request = Some(match self.nav.navigation_request {
            Some(NavigationRequest::Stop) => NavigationRequest::MoveToPositionAfterStop {
                target,
                reach_range: 1,
                speed_modifier,
            },
            _ => NavigationRequest::MoveToPosition {
                target,
                reach_range: 1,
                speed_modifier,
            },
        });
    }

    pub(crate) fn stop_navigation(&mut self) {
        self.nav.navigation_request = Some(NavigationRequest::Stop);
    }

    fn try_navigate_to_position(&mut self, wanted: Vec3d, reach_range: i32, speed_modifier: f64) {
        self.nav.navigation_request = Some(NavigationRequest::TryMoveToPosition {
            target: navigation::containing_block(wanted),
            reach_range,
            speed_modifier,
        });
    }

    fn require_path_to(&mut self, wanted: Vec3d, reach_range: i32) {
        self.nav.navigation_request = Some(NavigationRequest::RequirePath {
            target: navigation::containing_block(wanted),
            reach_range,
        });
    }

    fn set_navigation_speed(&mut self, speed_modifier: f64) {
        self.nav.navigation_speed_request = Some(speed_modifier);
    }

    fn set_avoid_sun(&mut self, avoid_sun: bool) {
        self.nav.avoid_sun_request = Some(avoid_sun);
    }

    fn set_own_pathfinding_malus(&mut self, path_type: crate::entity::GroundPathType, malus: f32) {
        self.nav.own_malus_writes.push((path_type, malus));
    }

    fn snap_to(&mut self, position: Vec3d) {
        let delta = Vec3d {
            x: position.x - self.body.position.x,
            y: position.y - self.body.position.y,
            z: position.z - self.body.position.z,
        };
        self.body.bounding_box = self.body.bounding_box.moved(delta);
        self.body.eye_y += delta.y;
        self.body.position = position;
        self.controls.teleport = Some(position);
    }

    pub(crate) const fn navigation_done(&self) -> bool {
        self.nav.navigation_done
    }
}

pub(crate) struct MobAiStepResult {
    pub(crate) x_rot: f32,
    pub(crate) y_rot: f32,
    pub(crate) no_gravity: Option<bool>,
    pub(crate) swimming_impulse: Vec3d,
    pub(crate) patrol: Option<PatrolStepResult>,
    pub(crate) horse_stand_requested: bool,
    pub(crate) run_around_outcome: Option<RunAroundLikeCrazyOutcome>,
    pub(crate) creeper_swell_dir: Option<i32>,
    pub(crate) wolf_interested: Option<bool>,
    pub(crate) kind: MobAiStepKindResult,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum MobAiStepKindResult {
    None,
    Bat(BatAiStepResult),
    Motion {
        movement: Vec3d,
        y_rot: f32,
    },
    RabbitJump(f32),
    Slime {
        y_rot: f32,
        jump_sound: Option<SlimeJumpSound>,
    },
    Blaze(BlazeAiStepResult),
    Witch(WitchRangedAttackAction),
    Graze(GrazingAction),
}

#[derive(Clone)]
pub(crate) struct MobAiRuntime {
    target_selector: GoalSelector,
    goal_selector: GoalSelector,
    random: GameRandom,
    ambient_sound_time: i32,
    sensing: Sensing,
    look_control: Option<LookControl>,
    jump_control: Option<JumpControl>,
    body_rotation_control: Option<BodyRotationControl>,
    kind: MobKind,
    target_state: target::TargetRuntime,
    navigation: MobNavigation,
    brain: Option<Brain>,
    /// Set when the entity's carried pack changed under the live brain, which
    /// `/data merge` does; the next tick rebuilds from the new pack.
    brain_stale: bool,
    villager_raid_activity: Option<crate::entity::ai::Activity>,
    villager_hero_gift: villager::hero::HeroGiftState,
    tongue_target: Option<i32>,
    brain_look_request: Option<Vec3d>,
    brain_jump_request: bool,
    pathfinding_malus: PathfindingMalus,
    move_control: MoveControl,
    raider_banner: Option<raider_banner::RaiderBannerState>,
    patrol_cooldown_until: i64,
    ranged: ranged::RangedState,
    doors: Option<interact_with_door::DoorRuntime>,
    combat_activity: CombatActivity,
    last_navigation_failed: bool,
}

/// The non-core activity the hoglin, zoglin and piglin brute brains last chose.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum CombatActivity {
    #[default]
    Idle,
    Avoid,
    Fight,
}

/// The state only one kind of mob carries.
#[derive(Clone)]
enum MobKind {
    Other,
    Bat(BatRuntime),
    Squid(SquidRuntime),
    Vex(VexRuntime),
    Bee(BeeRuntime),
    Ghast(GhastRuntime),
    Phantom(PhantomRuntime),
    Wither(WitherRuntime),
    Rabbit(RabbitRuntime),
    Slime(SlimeRuntime),
    Blaze(BlazeRuntime),
    Witch(WitchRuntime),
    Piglin(PiglinRuntime),
    CopperGolem(crate::entity::copper_golem::transport::TransportRuntime),
    Zombie(ZombieDoors),
    Vindicator(VindicatorDoors),
}

macro_rules! kind_accessors {
    ($($($get:ident)? / $($get_mut:ident)?: $variant:ident($runtime:ty);)*) => {
        impl MobKind {
            $(
                $(
                    fn $get(&self) -> Option<&$runtime> {
                        match self {
                            Self::$variant(runtime) => Some(runtime),
                            _ => None,
                        }
                    }
                )?

                $(
                    fn $get_mut(&mut self) -> Option<&mut $runtime> {
                        match self {
                            Self::$variant(runtime) => Some(runtime),
                            _ => None,
                        }
                    }
                )?
            )*
        }
    };
}

kind_accessors! {
    squid / squid_mut: Squid(SquidRuntime);
    vex / : Vex(VexRuntime);
    / bee_mut: Bee(BeeRuntime);
    ghast / ghast_mut: Ghast(GhastRuntime);
    phantom / phantom_mut: Phantom(PhantomRuntime);
    / wither_mut: Wither(WitherRuntime);
    rabbit / rabbit_mut: Rabbit(RabbitRuntime);
    / slime_mut: Slime(SlimeRuntime);
    / blaze_mut: Blaze(BlazeRuntime);
    witch / witch_mut: Witch(WitchRuntime);
    piglin / piglin_mut: Piglin(PiglinRuntime);
    doors / doors_mut: Zombie(ZombieDoors);
    / vindicator_doors_mut: Vindicator(VindicatorDoors);
    copper_golem / copper_golem_mut: CopperGolem(crate::entity::copper_golem::transport::TransportRuntime);
}

impl MobKind {
    fn new(entity_type_id: i32, random: &GameRandom) -> Self {
        use bbb_protocol::entity_types::*;
        if crate::entity::zombie::is_zombie_hierarchy(entity_type_id) {
            return Self::Zombie(ZombieDoors {
                can_break: false,
                state: BreakDoorState::default(),
                navigation_owned: entity_type_id != VANILLA_ENTITY_TYPE_DROWNED_ID,
            });
        }
        match entity_type_id {
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => Self::CopperGolem(Default::default()),
            VANILLA_ENTITY_TYPE_VINDICATOR_ID => Self::Vindicator(Default::default()),
            VANILLA_ENTITY_TYPE_PIGLIN_ID => Self::Piglin(PiglinRuntime::new()),
            VANILLA_ENTITY_TYPE_SLIME_ID | VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => {
                Self::Slime(SlimeRuntime::new())
            }
            VANILLA_ENTITY_TYPE_BLAZE_ID => Self::Blaze(BlazeRuntime::new()),
            VANILLA_ENTITY_TYPE_WITCH_ID => Self::Witch(WitchRuntime::new()),
            VANILLA_ENTITY_TYPE_VEX_ID => Self::Vex(VexRuntime::new()),
            VANILLA_ENTITY_TYPE_GHAST_ID | VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => {
                Self::Ghast(GhastRuntime::new())
            }
            VANILLA_ENTITY_TYPE_PHANTOM_ID => Self::Phantom(PhantomRuntime::new()),
            VANILLA_ENTITY_TYPE_BAT_ID => Self::Bat(BatRuntime::new()),
            VANILLA_ENTITY_TYPE_SQUID_ID | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => {
                Self::Squid(SquidRuntime::new(random))
            }
            VANILLA_ENTITY_TYPE_BEE_ID => Self::Bee(BeeRuntime::new(random)),
            VANILLA_ENTITY_TYPE_WITHER_ID => Self::Wither(WitherRuntime::new()),
            VANILLA_ENTITY_TYPE_RABBIT_ID => Self::Rabbit(RabbitRuntime::new()),
            _ => Self::Other,
        }
    }
}

pub(crate) struct MobNavigationDelegate<'a> {
    pub(crate) runtime: &'a mut MobAiRuntime,
    pub(crate) mob: MobPathingSnapshot,
    pub(crate) factory_owned: bool,
}

impl fmt::Debug for MobAiRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MobAiRuntime")
            .field("target_goals", &self.target_selector.len())
            .field("goals", &self.goal_selector.len())
            .field("ambient_sound_time", &self.ambient_sound_time)
            .field("sensing", &self.sensing)
            .field("base_look_control", &self.look_control.is_some())
            .field("base_jump_control", &self.jump_control.is_some())
            .field("base_body_rotation", &self.body_rotation_control.is_some())
            .field("ground_navigation", &self.navigation.path().is_some())
            .field("brain", &self.brain.is_some())
            .finish()
    }
}

impl MobAiRuntime {
    pub(crate) fn copper_golem_opened_chest(&self) -> Option<BlockPos> {
        self.kind.copper_golem()?.opened_chest()
    }

    pub(crate) fn take_copper_golem_transport(
        &mut self,
    ) -> Option<crate::entity::copper_golem::transport::TransportRuntime> {
        self.kind.copper_golem_mut().map(std::mem::take)
    }

    pub(crate) fn return_copper_golem_transport(
        &mut self,
        state: crate::entity::copper_golem::transport::TransportRuntime,
    ) {
        if let Some(runtime) = self.kind.copper_golem_mut() {
            *runtime = state;
        }
    }

    /// Records `now` and returns the previous activity.
    pub(crate) fn swap_combat_activity(&mut self, now: CombatActivity) -> CombatActivity {
        std::mem::replace(&mut self.combat_activity, now)
    }

    pub(crate) fn set_transport_look(&mut self, position: Vec3d) {
        self.brain_look_request = Some(position);
    }

    pub(crate) fn piglin(&self) -> Option<&PiglinRuntime> {
        self.kind.piglin()
    }

    pub(crate) fn piglin_mut(&mut self) -> Option<&mut PiglinRuntime> {
        self.kind.piglin_mut()
    }

    pub(crate) fn pathfinding_malus(&self) -> PathfindingMalus {
        self.pathfinding_malus
    }

    pub(crate) fn has_brain(&self) -> bool {
        self.brain.is_some()
    }

    pub(crate) fn nautilus_brain_pack(
        &self,
        base: &super::BrainPacked,
        tick_count: i32,
    ) -> Option<super::BrainPacked> {
        self.goal_selector.goals().find_map(|goal| match goal {
            MobGoal::Nautilus(goal) => goal.brain_pack(base, tick_count),
            _ => None,
        })
    }

    pub(crate) fn nautilus_is_angry(&self, tick_count: i32) -> Option<bool> {
        self.goal_selector.goals().find_map(|goal| match goal {
            MobGoal::Nautilus(goal) => goal.is_angry(tick_count),
            _ => None,
        })
    }

    /// The memory's value, `None` for a memory this brain never registered.
    pub(crate) fn brain_memory(&self, memory_type: MemoryModuleType) -> Option<&MemoryValue> {
        let brain = self.brain.as_ref()?;
        if !brain.check_memory(memory_type, crate::entity::ai::MemoryStatus::Registered) {
            return None;
        }
        brain.memory(memory_type)
    }

    pub(crate) fn set_brain_memory(
        &mut self,
        memory_type: MemoryModuleType,
        value: MemoryValue,
    ) -> bool {
        let Some(brain) = self.brain.as_mut() else {
            return false;
        };
        brain.set_memory(memory_type, Some(value));
        true
    }

    pub(crate) fn set_brain_memory_with_expiry(
        &mut self,
        memory_type: MemoryModuleType,
        value: MemoryValue,
        time_to_live: i64,
    ) {
        if let Some(brain) = self.brain.as_mut() {
            brain.set_memory_with_expiry(memory_type, value, time_to_live);
        }
    }

    pub(crate) fn erase_brain_memory(&mut self, memory_type: MemoryModuleType) {
        if let Some(brain) = self.brain.as_mut() {
            brain.erase_memory(memory_type);
        }
    }

    /// A runtime drawing from `random`, the level's stream.
    #[cfg(test)]
    pub(crate) fn for_entity(entity_type_id: i32, random: GameRandom) -> Box<Self> {
        Self::for_entity_with_plugins(
            entity_type_id,
            random,
            crate::plugin::ServerPluginRegistry::vanilla(),
        )
    }

    pub(crate) fn for_entity_with_plugins(
        entity_type_id: i32,
        random: GameRandom,
        plugins: &crate::plugin::ServerPluginRegistry,
    ) -> Box<Self> {
        let mut runtime = Box::new(Self::with_random(random));
        runtime.pathfinding_malus = PathfindingMalus::for_entity_type(entity_type_id);
        let follow_range_override = plugins.mob_entry(entity_type_id).map(|entry| {
            entry
                .spec
                .default_attributes
                .iter()
                .find_map(|&(id, val)| {
                    (id == bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)
                        .then_some(val)
                })
                .unwrap_or(16.0)
        });
        runtime.install_navigation_with_follow_range(entity_type_id, false, follow_range_override);
        runtime.look_control = LookControl::for_entity(entity_type_id);
        runtime.jump_control = JumpControl::for_entity(entity_type_id);
        runtime.body_rotation_control = BodyRotationControl::for_entity(entity_type_id);
        runtime.kind = MobKind::new(entity_type_id, &runtime.random);
        runtime.raider_banner = crate::entity::raider::is_raider(entity_type_id)
            .then(raider_banner::RaiderBannerState::default);
        runtime.doors = interact_with_door::opens_doors(entity_type_id)
            .then(interact_with_door::DoorRuntime::default);
        if let Some(entry) = plugins.mob_entry(entity_type_id) {
            runtime.add_plugin_behavior_goals(
                &entry.behavior_spec,
                entity_type_id,
                follow_range_override.unwrap_or(16.0),
            );
        } else if let Some(spec) = mob_spec(entity_type_id) {
            runtime.add_spec_goals(spec, entity_type_id);
        }
        runtime
    }

    fn install_navigation(&mut self, entity_type_id: i32, baby: bool) {
        self.install_navigation_with_follow_range(entity_type_id, baby, None);
    }

    fn install_navigation_with_follow_range(
        &mut self,
        entity_type_id: i32,
        baby: bool,
        follow_range_override: Option<f64>,
    ) {
        let factory = navigation::navigation_factory(entity_type_id, baby);
        let base_follow_range = follow_range_override.unwrap_or_else(|| {
            bbb_protocol::entity_types::vanilla_entity_type(entity_type_id)
                .and_then(|facts| {
                    crate::entity::attributes::AttributeSupplier::for_entity_type(facts.resource_id)
                })
                .and_then(|supplier| {
                    supplier.base_value(
                        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID,
                    )
                })
                .unwrap_or(0.0)
        });
        self.navigation = MobNavigation::from_factory(
            factory.unwrap_or(navigation::NavigationFactory::Ground),
            entity_type_id,
            base_follow_range,
        );
        if interact_with_door::opens_doors(entity_type_id) {
            self.navigation.set_can_open_doors(true);
        }
        self.move_control = MoveControl::default();
        let move_kind = match factory {
            Some(navigation::NavigationFactory::Flying(spec)) => {
                MoveControlKind::Flying(spec.move_control)
            }
            _ => mob_spec(entity_type_id).map_or_else(
                || {
                    if crate::entity::fish::uses_fish_move_control(entity_type_id) {
                        MoveControlKind::Fish
                    } else {
                        MoveControlKind::Ground
                    }
                },
                |spec| spec.move_control,
            ),
        };
        self.move_control.set_kind(move_kind);
    }

    fn uses_ghast_move_control(&self) -> bool {
        self.kind.ghast().is_some() && !matches!(self.navigation, MobNavigation::Flying(_))
    }

    fn add_spec_goals(&mut self, spec: &MobSpec, entity_type_id: i32) {
        self.goal_selector.reserve(spec.goals.len());
        self.target_selector.reserve(spec.targets.len());
        for &(priority, goal) in spec.goals {
            let Some(built) = MobGoal::new(goal, entity_type_id) else {
                continue;
            };
            if matches!(goal, GoalSpec::Float) {
                self.navigation.set_can_float(true);
            }
            self.goal_selector.add(priority, goal, built);
        }
        for &(priority, goal) in spec.targets {
            if let Some(built) = MobGoal::new(goal, entity_type_id) {
                self.target_selector.add(priority, goal, built);
            }
        }
    }

    fn add_plugin_behavior_goals(
        &mut self,
        behavior_spec: &crate::plugin::MobBehaviorSpec,
        entity_type_id: i32,
        follow_range: f64,
    ) {
        self.goal_selector.reserve(behavior_spec.goals.len());
        self.target_selector.reserve(behavior_spec.targets.len());
        for &(priority, goal) in &behavior_spec.goals {
            let spec_goal = spec::from_plugin_goal(goal);
            let Some(built) =
                MobGoal::new_with_follow_range(spec_goal, entity_type_id, Some(follow_range))
            else {
                continue;
            };
            if matches!(spec_goal, GoalSpec::Float) {
                self.navigation.set_can_float(true);
            }
            self.goal_selector.add(priority, spec_goal, built);
        }
        for &(priority, goal) in &behavior_spec.targets {
            let spec_goal = spec::from_plugin_goal(goal);
            if let Some(built) =
                MobGoal::new_with_follow_range(spec_goal, entity_type_id, Some(follow_range))
            {
                self.target_selector.add(priority, spec_goal, built);
            }
        }
    }

    fn has_goal(&self, matches: impl Fn(&MobGoal) -> bool) -> bool {
        self.goal_selector.goals().any(matches)
    }

    fn with_random(random: GameRandom) -> Self {
        Self {
            target_selector: GoalSelector::default(),
            goal_selector: GoalSelector::default(),
            random,
            ambient_sound_time: 0,
            sensing: Sensing::default(),
            look_control: None,
            jump_control: None,
            body_rotation_control: None,
            kind: MobKind::Other,
            target_state: target::TargetRuntime::default(),
            navigation: MobNavigation::default(),
            brain: None,
            brain_stale: false,
            villager_raid_activity: None,
            villager_hero_gift: villager::hero::HeroGiftState::default(),
            tongue_target: None,
            brain_look_request: None,
            brain_jump_request: false,
            pathfinding_malus: PathfindingMalus::default(),
            move_control: MoveControl::default(),
            raider_banner: None,
            patrol_cooldown_until: -1,
            ranged: ranged::RangedState::default(),
            doors: None,
            combat_activity: CombatActivity::Idle,
            last_navigation_failed: false,
        }
    }

    pub(crate) fn is_panicking(&self) -> bool {
        self.brain
            .as_ref()
            .is_some_and(|brain| brain.has_memory_value(MemoryModuleType::IsPanicking))
            || self.panic_goal_running()
    }

    fn panic_goal_running(&self) -> bool {
        self.goal_selector
            .is_running(|goal| matches!(goal, MobGoal::Panic(_)))
    }

    pub(crate) fn breed_goal_running(&self) -> bool {
        self.goal_selector
            .is_running(|goal| matches!(goal, MobGoal::Breed(_)))
    }

    pub(crate) fn tempt_goal_running(&self) -> bool {
        self.goal_selector
            .is_running(|goal| matches!(goal, MobGoal::Tempt(_)))
    }

    /// `false` is a brain this runtime never built, which is the same refusal
    /// [`Self::ensure_brain`] answers for a type without a provider.
    pub(crate) fn set_live_brain_memory(
        &mut self,
        memory_type: MemoryModuleType,
        value: Option<MemoryValue>,
    ) -> bool {
        let Some(brain) = self.brain.as_mut() else {
            return false;
        };
        brain.set_memory(memory_type, value);
        true
    }

    pub(crate) fn prepare_witch_targeting(&mut self, allowed: bool) {
        if let Some(witch) = self.kind.witch_mut() {
            witch.attack_players = allowed;
        }
    }

    pub(crate) fn refresh_retained_navigation(
        &mut self,
        entity: &crate::entity::Entity,
    ) -> Option<BlockPos> {
        // `Rabbit.setVariant(EVIL)` adds the killer bunny's goals
        // leaving the variant keeps them.
        if self.kind.rabbit().is_some()
            && crate::entity::rabbit::is_evil(entity)
            && !self.has_goal(|goal| matches!(goal, MobGoal::MeleeAttack(_)))
        {
            self.add_spec_goals(&spec::KILLER_BUNNY, entity.entity_type_id());
        }
        let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
            return None;
        };
        if data.happy_ghast_state().is_some()
            && data.is_baby() != matches!(self.navigation, MobNavigation::Flying(_))
        {
            self.install_navigation(entity.entity_type_id(), data.is_baby());
            self.goal_selector = GoalSelector::default();
            let goals = if data.is_baby() {
                &spec::GHASTLING
            } else {
                mob_spec(entity.entity_type_id())?
            };
            self.add_spec_goals(goals, entity.entity_type_id());
            if let Some(ghast) = self.kind.ghast_mut() {
                ghast.set_wanted_position(None);
            }
        }
        let can_break_doors = data.zombie_retained()?.can_break_doors();
        let doors = self.kind.doors_mut()?;
        if doors.can_break == can_break_doors {
            return None;
        }
        doors.can_break = can_break_doors;
        let stop_effect = doors.state.stop_effect();
        self.navigation.set_can_open_doors(can_break_doors);
        if can_break_doors {
            // `Zombie.setCanBreakDoors`.
            self.goal_selector.add(
                1,
                GoalSpec::BreakDoor,
                MobGoal::BreakDoor(door::BreakDoorGoal),
            );
            return None;
        }
        if self
            .goal_selector
            .remove(|goal| matches!(goal, MobGoal::BreakDoor(_)))
            != Some(true)
        {
            return None;
        }
        match stop_effect {
            DoorEffect::Progress(pos, -1) => Some(pos),
            _ => None,
        }
    }

    pub(crate) fn navigate_to(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        let moved = if self.kind.copper_golem().is_some() {
            // `TransportItemsBetweenContainers` (`TransportItemsBetweenContainers.java:101`).
            self.navigation.set_can_path_to_targets_below_surface(true);
            self.navigation
                .create_path_to(level, mob, &self.random, target, reach_range, Some(48.0))
                .is_some_and(|path| {
                    self.navigation.start_created_path(
                        level,
                        path,
                        mob,
                        reach_range,
                        speed_modifier,
                    )
                })
        } else {
            self.navigation.move_to(
                level,
                mob,
                &self.random,
                target,
                reach_range,
                speed_modifier,
            )
        };
        if let Some(prepared) = self.navigation.take_amphibious_prepare() {
            prepared.apply(&mut self.pathfinding_malus);
        }
        moved
    }

    pub(crate) fn navigate_for_leash(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        speed_modifier: f64,
    ) -> bool {
        self.goal_selector
            .set_control_enabled(GoalControl::Move, true);
        self.navigate_to(level, mob, target, 0, speed_modifier)
    }

    pub(crate) fn can_follow_school(&self) -> bool {
        !self
            .goal_selector
            .is_running(|goal| matches!(goal, MobGoal::Panic(_) | MobGoal::AvoidEntity(_)))
    }

    pub(crate) fn ground_path(&self) -> Option<&crate::entity::GroundPath> {
        self.navigation.path()
    }

    pub(crate) fn ground_path_reaches(
        &self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        reach_range: i32,
        max_path_length: f32,
    ) -> Option<bool> {
        self.navigation
            .ground()?
            .create_path_to(level, mob, target, reach_range, Some(max_path_length))
            .map(|path| path.reached())
    }

    pub(crate) fn guardian_moving(&self) -> Option<bool> {
        self.move_control.guardian_moving(self.navigation.is_done())
    }

    pub(crate) fn stop_navigation(&mut self) {
        self.navigation.stop();
        self.navigation.set_can_path_to_targets_below_surface(false);
    }

    pub(crate) fn navigation_done(&self) -> bool {
        self.navigation.is_done()
    }

    pub(crate) fn goal_debug_info(&self) -> DebugGoalInfo {
        let mut info = self.goal_selector.debug_info();
        if let Some(phantom) = self.kind.phantom() {
            info.goals.extend(phantom.goal_debug_info().goals);
        }
        info
    }

    pub(crate) fn path_debug_info(&self) -> Option<DebugPathInfo> {
        self.navigation.debug_info()
    }

    pub(crate) fn set_path_debug_capture(&mut self, enabled: bool) {
        self.navigation.set_debug_capture(enabled);
    }

    pub(crate) fn should_recompute_ground_path(
        &self,
        positions: &[BlockPos],
        mob_position: Vec3d,
    ) -> bool {
        self.navigation.should_recompute(positions, mob_position)
    }

    pub(crate) fn recompute_ground_path(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
    ) {
        self.navigation.recompute(level, mob, &self.random);
        if let Some(prepared) = self.navigation.take_amphibious_prepare() {
            prepared.apply(&mut self.pathfinding_malus);
        }
    }

    pub(crate) fn witch_self_potion_step(
        &mut self,
        input: SelfPotionInput,
    ) -> Option<(bool, SelfPotionStep)> {
        self.kind
            .witch_mut()
            .map(|witch| witch.self_potion_step(&self.random, input))
    }

    pub(crate) fn witch_water_check_reached(&self) -> Option<bool> {
        self.kind.witch()?;
        Some(self.random.next_float() < 0.15)
    }

    /// `Mob.getTarget()` as the last step left it, for the readers outside
    /// the mob's own tick.
    pub(crate) fn valid_target<'a, 'l, 'w>(
        &'a self,
        body: &'a Entity,
        store: &'a crate::entity::EntityStore,
        level: &'a LevelCtx<'l, 'w>,
    ) -> Option<target::Target<'a, 'l, 'w>> {
        target::TargetView::valid_shared_target(
            body,
            store,
            level,
            &self.sensing,
            self.target_state.target?,
        )
    }
    pub(crate) fn water_target_position(
        &self,
        body: &Entity,
        store: &crate::entity::EntityStore,
        level: &LevelCtx<'_, '_>,
    ) -> Option<Vec3d> {
        let view = target::TargetView::new(body, store, level, &self.sensing);
        let target = view.facts_of(self.target_state.target?)?;
        target.in_water().then(|| target.position())
    }

    pub(crate) fn target_view<'a, 'l, 'w>(
        &'a self,
        body: &'a Entity,
        store: &'a crate::entity::EntityStore,
        level: &'a LevelCtx<'l, 'w>,
    ) -> target::TargetView<'a, 'l, 'w> {
        target::TargetView::new(body, store, level, &self.sensing)
    }

    pub(crate) const fn has_target(&self) -> bool {
        self.target_state.target.is_some()
    }

    /// `Mob.target` without `asValidTarget`, which another mob's
    /// `getTarget()` is validated from.
    pub(crate) const fn target_ref(&self) -> Option<target::TargetRef> {
        self.target_state.target
    }

    pub(crate) fn target_state(&self) -> &target::TargetRuntime {
        &self.target_state
    }

    pub(crate) fn target_state_mut(&mut self) -> &mut target::TargetRuntime {
        &mut self.target_state
    }

    /// The runtime's random and target half together, for a `setTarget` made
    /// on this mob from another mob's tick.
    pub(crate) fn target_state_and_random(&mut self) -> (&mut target::TargetRuntime, &GameRandom) {
        (&mut self.target_state, &mut self.random)
    }

    pub(crate) fn witch_throw_pitch(&mut self) -> f32 {
        0.8 + self.random.next_float() * 0.4
    }

    pub(crate) fn slime_after_super(
        &mut self,
        on_ground: bool,
        was_on_ground: bool,
        size: i32,
        magma_cube: bool,
    ) -> Option<SlimeJumpSound> {
        self.kind
            .slime_mut()?
            .after_super(&self.random, on_ground, was_on_ground, size, magma_cube)
    }

    pub(crate) fn rabbit_after_ai_step(&mut self) -> bool {
        self.kind
            .rabbit_mut()
            .is_some_and(RabbitRuntime::after_ai_step)
    }

    pub(crate) fn rabbit_jump_input(&self) -> Option<crate::entity::rabbit::RabbitJumpInput> {
        self.kind.rabbit()?;
        let movement = self.move_control.wanted_movement();
        Some(crate::entity::rabbit::RabbitJumpInput {
            speed_modifier: movement.speed_modifier,
            wanted_y: movement.wanted.y,
            next_node_y: self
                .navigation
                .path()
                .and_then(|path| path.nodes().get(path.next_node_index()))
                .map(|node| f64::from(node.pos.y)),
        })
    }

    pub(crate) fn squid_x_body_rot_o(&self) -> Option<f32> {
        self.kind.squid().map(SquidRuntime::x_body_rot_o)
    }

    pub(crate) fn next_random_int(&mut self, bound: i32) -> i32 {
        self.random.next_int(bound)
    }

    pub(crate) fn phantom_step(
        &mut self,
        body: &Entity,
        store: &crate::entity::EntityStore,
        level: &LevelCtx<'_, '_>,
    ) -> Option<PhantomAiStepResult> {
        let runtime = self.kind.phantom_mut()?;
        self.sensing.tick();
        let targets = target::TargetView::new(body, store, level, &self.sensing);
        Some(runtime.step(&self.random, &targets, &mut self.target_state.target))
    }

    pub(crate) fn slime_split_count(&mut self) -> usize {
        (2 + self.random.next_int(3)) as usize
    }

    pub(crate) fn slime_split_y_rot(&mut self) -> f32 {
        self.random.next_float() * 360.0
    }

    pub(crate) fn bee_custom_server_ai_step(
        &mut self,
        has_stung: bool,
        in_water: Option<bool>,
    ) -> Option<BeeCustomStep> {
        self.kind
            .bee_mut()
            .map(|bee| bee.custom_server_ai_step(&self.random, has_stung, in_water))
    }

    /// `BeePollinateGoal.stopPollinating` and its `stop`.
    pub(crate) fn bee_stop_pollinating(&mut self) {
        if self
            .kind
            .bee_mut()
            .is_some_and(BeeRuntime::stop_pollinating)
        {
            self.stop_navigation();
        }
    }

    pub(crate) fn bee_after_ai_step(&mut self) {
        if let Some(bee) = self.kind.bee_mut() {
            bee.after_ai_step();
        }
    }

    pub(crate) fn bee_after_tick(&mut self, rolling: bool) {
        if let Some(bee) = self.kind.bee_mut() {
            bee.after_tick(rolling);
        }
    }

    pub(crate) fn wither_mut(&mut self) -> Option<&mut WitherRuntime> {
        self.kind.wither_mut()
    }

    pub(crate) fn wither_after_ai_step(&mut self, y_body_rot: f32) {
        if let Some(wither) = self.kind.wither_mut() {
            wither.after_ai_step(y_body_rot);
        }
    }

    pub(crate) fn accept_wither_hurt(&mut self) {
        if let Some(wither) = self.kind.wither_mut() {
            wither.accepted_hurt();
        }
    }

    pub(crate) fn notify_silverfish_hurt(&mut self, now: i64) {
        self.goal_selector.notify_silverfish_hurt(now);
    }

    pub(crate) fn dolphin_dry_jump_draws(&mut self) -> (f32, f32, f32) {
        (
            self.random.next_float(),
            self.random.next_float(),
            self.random.next_float(),
        )
    }

    pub(crate) fn abstract_fish_flop_draws(&mut self) -> (f32, f32) {
        (self.random.next_float(), self.random.next_float())
    }

    pub(crate) fn tick_abstract_horse_ambient_sound(&mut self, alive: bool) -> bool {
        if !alive {
            return false;
        }
        let roll = self.random.next_int(1000);
        let threshold = self.ambient_sound_time;
        self.ambient_sound_time = self.ambient_sound_time.wrapping_add(1);
        if roll >= threshold {
            return false;
        }
        self.ambient_sound_time = -400;
        true
    }

    pub(crate) fn tick_mob_ambient_pitch(&mut self, alive: bool) -> Option<f32> {
        self.tick_mob_ambient_sound(alive)
            .then(|| self.voice_pitch(false))
    }

    pub(crate) fn voice_pitch(&mut self, is_baby: bool) -> f32 {
        let base = if is_baby { 1.5 } else { 1.0 };
        (self.random.next_float() - self.random.next_float()) * 0.2 + base
    }

    pub(crate) fn set_ambient_sound_time(&mut self, ambient_sound_time: i32) {
        self.ambient_sound_time = ambient_sound_time;
    }

    pub(crate) const fn ambient_sound_time(&self) -> i32 {
        self.ambient_sound_time
    }

    pub(crate) fn tick_mob_ambient_sound(&mut self, alive: bool) -> bool {
        if !alive {
            return false;
        }
        let roll = self.random.next_int(1000);
        let threshold = self.ambient_sound_time;
        self.ambient_sound_time = self.ambient_sound_time.wrapping_add(1);
        if roll >= threshold {
            return false;
        }
        self.ambient_sound_time = -80;
        true
    }

    pub(crate) fn merchant_trade_xp(&mut self) -> i32 {
        3 + self.random.next_int(4)
    }

    /// The base `LookControl`'s wanted position, `None` for a replaced one.
    pub(crate) fn look_wanted_position(&self) -> Option<Vec3d> {
        self.look_control.as_ref().map(LookControl::wanted_position)
    }

    pub(crate) fn tick_squid(
        &mut self,
        entity: &mut crate::entity::Entity,
        in_water: Option<bool>,
    ) -> Option<bool> {
        self.kind
            .squid_mut()
            .map(|squid| squid.ai_step(entity, &self.random, in_water))
    }

    #[cfg(test)]
    pub(crate) fn squid_runtime_mut(&mut self) -> Option<&mut SquidRuntime> {
        self.kind.squid_mut()
    }

    pub(crate) fn tick_body_rotation(
        &mut self,
        previous_position: Vec3d,
        position: Vec3d,
        y_rot: f32,
        living: &mut LivingEntityState,
    ) {
        if let Some(control) = &mut self.body_rotation_control {
            // GeneralLiving save refuses mob passengers.
            control.tick(
                previous_position,
                position,
                y_rot,
                &mut living.y_body_rot,
                &mut living.y_head_rot,
                false,
            );
        }
        normalize_retained_rotations(living);
    }

    pub(crate) fn after_living_tick(
        &mut self,
        tick_count: i32,
        no_controller: bool,
        not_in_boat: bool,
        ravager: bool,
    ) {
        if tick_count % 5 != 0 {
            return;
        }
        self.goal_selector
            .set_control_enabled(GoalControl::Move, no_controller);
        self.goal_selector
            .set_control_enabled(GoalControl::Jump, no_controller && not_in_boat);
        self.goal_selector
            .set_control_enabled(GoalControl::Look, no_controller);
        if ravager {
            self.goal_selector
                .set_control_enabled(GoalControl::Target, no_controller);
        }
    }
}

mod allay;
mod avoid;
pub(crate) mod bat;
mod beg;
mod body_rotation;
pub(crate) mod brain_runtime;
mod breed;
mod breeze;
mod camel_idle;
pub(crate) mod cat;
pub(crate) mod creaking;
pub(crate) mod creeper;
mod dolphin;
mod door;
mod float_goal;
mod follow_mob;
mod follow_owner;
mod follow_parent;
mod follow_player_vehicle;
pub(crate) mod frog;
pub(crate) mod ghast;
mod goal_selector;
mod goat;
mod guardian;
mod hoglin;
pub(crate) mod interact_with_door;
mod iron_golem;
mod jump_control;
mod llama_caravan;
mod look_at;
mod melee;
mod nautilus;
mod offer_flower;
pub(crate) mod parrot;
mod piglin;
mod shulker;
mod wandering_trader;
mod warden;
mod warden_awareness;
mod warden_lifecycle;
pub(crate) use melee::{do_hurt_target, melee_attack_box, MeleeReach};
mod drowned;
mod fox_berries;
mod fox_hunt;
mod fox_items;
mod fox_perch;
mod fox_sleep;
mod move_through_village;
mod move_to_block;
pub(crate) mod navigation;
mod panda_lie;
mod panda_roll;
mod panda_sit;
mod panda_sneeze;
pub(crate) mod panic;
pub(crate) mod patrol;
pub(crate) mod phantom;
mod powder_snow;
pub(crate) mod rabbit;
mod raider_banner;
mod raider_celebration;
mod raider_hold_ground;
mod raider_travel;
mod raider_village;
mod random_look;
pub(crate) mod random_pos;
mod random_stand;
pub(crate) mod ranged;
pub(crate) mod run_around_like_crazy;
mod sensing;
mod server_ai_step;
pub(crate) use server_ai_step::apply_post_step_actions;
pub(crate) mod sheep;
mod silverfish;
mod sit_when_ordered;
mod skeleton_horse;
pub(crate) mod slime;
mod spear_use;
pub(crate) mod spec;
pub(crate) mod squid;
pub(crate) mod stroll;
mod sun;
mod tadpole;
pub(crate) mod target;
mod tempt;
#[cfg(test)]
mod tests;
mod trade_with_player;
mod turtle;
pub(crate) mod vex;
pub(crate) mod villager;
mod zombie;

mod enderman;
mod enderman_freeze;
