//! Each mob type's `registerGoals` written as data, the goals it builds, and
//! the Java class chain its `instanceof` tests read.

use super::allay::AllayCollectGoal;
use super::avoid::{AvoidEntityGoal, AvoidGate, AvoidSpec};
use super::beg::BegGoal;
use super::breed::BreedGoal;
use super::breeze::BreezeCombatGoal;
use super::camel_idle::CamelIdlePoseGoal;
use super::cat::CatRelaxOnOwnerGoal;
use super::creeper::CreeperSwellGoal;
use super::dolphin::{
    BreathAirGoal, JumpGoal, PlayWithItemsGoal, SwimToTreasureGoal, SwimWithPlayerGoal,
    TryFindWaterGoal,
};
use super::door::BreakDoorGoal;
use super::drowned::{DrownedGoToWaterGoal, DrownedSwimUpGoal};
use super::enderman::blocks::{EndermanLeaveBlockGoal, EndermanTakeBlockGoal};
use super::enderman::look_for_player::LookForPlayerGoal;
use super::enderman_freeze::EndermanFreezeWhenLookedAt;
use super::float_goal::FloatGoal;
use super::follow_mob::FollowMobGoal;
use super::follow_owner::FollowOwnerGoal;
use super::follow_parent::{FollowMode, FollowParentGoal};
use super::follow_player_vehicle::FollowPlayerVehicleGoal;
use super::fox_hunt::{FoxFaceplantGoal, FoxPounceGoal, FoxStalkGoal};
use super::fox_items::FoxSearchItemsGoal;
use super::fox_perch::FoxPerchGoal;
use super::fox_sleep::FoxSleepGoal;
use super::ghast::{GhastLookGoal, GhastRandomFloatAroundGoal};
use super::goat::{long_jump::GoatLongJumpGoal, GoatRamGoal};
use super::guardian::GuardianAttackGoal;
use super::hoglin::HoglinIdleGoal;
use super::iron_golem::MoveTowardsTargetGoal;
use super::llama_caravan::LlamaCaravanGoal;
use super::look_at::{LookAtPlayerGoal, LookAtSpec, LookAtTargetKind};
use super::melee::{LeapAtTargetGoal, MeleeAttackGoal, MeleeKind, OcelotAttackGoal};
use super::move_through_village::MoveThroughVillageGoal;
use super::move_to_block::{MoveToBlockGoal, MoveToBlockKind};
use super::nautilus::NautilusGoal;
use super::navigation::{GroundTuning, MoveControlKind, NavigationFactory};
use super::offer_flower::OfferFlowerGoal;
use super::panda_lie::PandaLieOnBackGoal;
use super::panda_roll::PandaRollGoal;
use super::panda_sit::PandaSitGoal;
use super::panda_sneeze::PandaSneezeGoal;
use super::panic::{PanicGoal, PanicKind};
use super::parrot::LandOnOwnersShoulderGoal;
use super::patrol::LongDistancePatrolGoal;
use super::piglin::PiglinIdleGoal;
use super::powder_snow::ClimbPowderSnowGoal;
use super::raider_banner::RaiderBannerGoal;
use super::raider_celebration::RaiderCelebrationGoal;
use super::raider_hold_ground::RaiderHoldGroundGoal;
use super::raider_travel::RaiderTravelGoal;
use super::raider_village::RaiderVillageGoal;
use super::random_look::{LookControlKind, RandomLookAroundGoal};
use super::random_stand::RandomStandGoal;
use super::ranged::{
    BowAttackGoal, BowSpec, CrossbowAttackGoal, GhastShootFireballGoal, RangedAttackGoal,
    RangedKind, RangedSpec, ShulkerAttackGoal, Spell, SpellGoal, SpellcasterCastingGoal,
};
use super::run_around_like_crazy::RunAroundLikeCrazyGoal;
use super::sheep::SheepEatBlockGoal;
use super::shulker::ShulkerPeekGoal;
use super::silverfish::{SilverfishMergeGoal, SilverfishWakeFriendsGoal};
use super::sit_when_ordered::SitWhenOrderedToGoal;
use super::skeleton_horse::SkeletonHorseTrapGoal;
use super::slime::{
    SlimeAttackGoal, SlimeFloatGoal, SlimeKeepOnJumpingGoal, SlimeRandomDirectionGoal,
};
use super::spear_use::SpearUseGoal;
use super::squid::{SquidFleeGoal, SquidRandomMovementGoal};
use super::stroll::{RandomStrollGoal, StrollPosition};
use super::sun::{FleeSunGoal, RestrictSunGoal};
use super::target::anger::ResetUniversalAngerTargetGoal;
use super::target::bee::{BeeTargetGoal, BeeTargetKind};
use super::target::goals::{
    HurtByTargetGoal, HurtByVariant, NearestAttackableTargetGoal, NearestVariant,
};
use super::target::owner::{
    DefendVillageTargetGoal, OwnerMemory, OwnerTargetGoal, TraderLlamaDefendGoal,
    VexCopyOwnerTargetGoal,
};
use super::target::selector::TargetSelector;
use super::target::TargetView;
use super::tempt::{TemptGoal, TemptItems, TemptScare, TemptSpec};
use super::trade_with_player::TradeWithPlayerGoal;
use super::turtle::TurtleTravelGoal;
use super::vex::{VexChargeAttackGoal, VexRandomMovementGoal};
use super::wandering_trader::{TraderUseItemGoal, WanderToPositionGoal};
use super::warden::WardenCombatGoal;
use super::warden_awareness::WardenAwarenessGoal;
use super::warden_lifecycle::WardenLifecycleGoal;
use super::MobAiContext;
use crate::entity::ai::{Goal, GoalControls};
use crate::entity::axolotl::PlayDeadGoal;
use crate::entity::blaze::BlazeAttackGoal;
use crate::entity::wandering_trader::TraderDrinkKind;

mod table;
#[cfg(test)]
mod tests;
pub(crate) use table::{mob_spec, GHASTLING, KILLER_BUNNY};

#[derive(Debug)]
pub(crate) struct MobSpec {
    /// The runtime class's simple name; [`superclass`] walks its chain.
    pub(crate) class: &'static str,
    /// What `createNavigation` builds, `None` for one this runtime does not.
    pub(crate) navigation: Option<NavigationFactory>,
    pub(crate) ground_tuning: GroundTuning,
    pub(crate) move_control: MoveControlKind,
    pub(crate) look_control: Option<LookControlKind>,
    pub(crate) jump_control: bool,
    pub(crate) max_head_x_rot: Option<f32>,
    pub(crate) goals: &'static [(i32, GoalSpec)],
    pub(crate) targets: &'static [(i32, GoalSpec)],
}

impl MobSpec {
    const fn new(class: &'static str, navigation: Option<NavigationFactory>) -> Self {
        Self {
            class,
            navigation,
            ground_tuning: GroundTuning::DEFAULT,
            move_control: MoveControlKind::Ground,
            look_control: Some(LookControlKind::Standard),
            jump_control: true,
            max_head_x_rot: Some(40.0),
            goals: &[],
            targets: &[],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LookAtGoalSpec {
    Standard(LookAtSpec),
    Trading,
    TraderInteract,
    IdleAttention {
        players_only: bool,
        max_distance: f64,
        approach_speed: f64,
    },
}

/// One `addGoal` a mob's `registerGoals` makes, with its constructor
/// arguments. Only goals this runtime implements are listed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum GoalSpec {
    FoxSearchItems,
    FoxPerch,
    FoxSleep,
    FoxStalk,
    FoxPounce,
    FoxFaceplant,
    PandaLieOnBack,
    PandaRoll,
    PandaSit,
    PandaSneeze,
    SkeletonHorseTrap,
    SilverfishWakeFriends,
    SilverfishMerge,
    Float,
    ClimbPowderSnow,
    SlimeFloat,
    SlimeAttack,
    SlimeRandomDirection,
    SlimeKeepOnJumping,
    EatBlock,
    Swell,
    BlazeAttack,
    BreezeCombat,
    WardenCombat,
    AllayCollect,
    WardenAwareness,
    WardenLifecycle,
    BowAttack(BowSpec),
    CrossbowAttack {
        speed: f64,
        radius: f32,
    },
    RangedAttack(RangedSpec),
    GhastShootFireball,
    ShulkerAttack,
    ShulkerPeek,
    GuardianAttack,
    OfferFlower,
    MoveTowardsTarget,
    Nautilus,
    GoatRam,
    GoatLongJump,
    BreathAir,
    TryFindWater {
        move_look: bool,
    },
    AxolotlPlayDead,
    DolphinSwimWithPlayer,
    DolphinSwimToTreasure,
    DolphinPlayWithItems,
    DolphinJump,
    FollowPlayerVehicle,
    SpellcasterCasting,
    Spell(Spell),
    VexRandomMove,
    VexChargeAttack,
    /// `SpearUseGoal(mob, 1.0, 1.0, 10.0F, 2.0F)` (`Zombie.java`, `ZombifiedPiglin.java`).
    SpearUse,
    GhastRandomFloat,
    GhastLook,
    SquidRandomMovement,
    SquidFlee,
    RestrictSun,
    FleeSun {
        speed: f64,
        fox: bool,
    },
    AvoidEntity(AvoidSpec),
    LongDistancePatrol,
    RandomLookAround {
        camel: bool,
    },
    RandomStand,
    TradeWithPlayer,
    TraderTravel,
    LlamaCaravan,
    RaiderBanner,
    RaiderTravel,
    RaiderHoldGround,
    RaiderVillage,
    RaiderCelebration,
    MoveThroughVillage,
    TurtleTravel,
    TraderUseItem(TraderDrinkKind),
    EndermanFreeze,
    EndermanLeaveBlock,
    EndermanTakeBlock,
    BreakDoor,
    OcelotAttack,
    MoveToBlock(MoveToBlockKind),
    DrownedGoToWater,
    DrownedSwimUp,
    LookAt(LookAtGoalSpec),
    CamelIdlePose,
    HoglinIdle,
    PiglinIdle,
    Beg,
    MeleeAttack {
        speed: f64,
        follow_unseen: bool,
        kind: MeleeKind,
    },
    LeapAtTarget {
        yd: f32,
    },
    Tempt {
        speed: f64,
        items: TemptItems,
        scare: TemptScare,
    },
    FollowParent {
        speed: f64,
        mode: FollowMode,
    },
    Panic {
        speed: f64,
        kind: PanicKind,
    },
    Breed {
        speed: f64,
    },
    RunAroundLikeCrazy {
        speed: f64,
    },
    FollowOwner {
        speed: f64,
        start_distance: f32,
        stop_distance: f32,
    },
    SitWhenOrderedTo,
    CatRelaxOnOwner,
    LandOnOwnersShoulder,
    FollowMob {
        speed: f64,
        stop_distance: f32,
        area_size: f32,
    },
    Stroll {
        speed: f64,
        position: StrollPosition,
        interval: i32,
    },
    Nearest(NearestSpec),
    HurtBy(HurtBySpec),
    ResetUniversalAnger {
        alert_others: bool,
    },
    OwnerTarget(OwnerMemory),
    TraderLlamaDefend,
    VexCopyOwner,
    DefendVillage,
    EndermanLookForPlayer,
    BeeTarget(BeeTargetKind),
}

/// `NearestAttackableTargetGoal` and its subclasses' constructor arguments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NearestSpec {
    pub(crate) variant: NearestVariant,
    /// The simple name of the searched class.
    pub(crate) target_class: &'static str,
    /// `randomInterval` before `reducedTickDelay`.
    pub(crate) interval: i32,
    pub(crate) must_see: bool,
    pub(crate) selector: TargetSelector,
    pub(crate) unseen_memory_ticks: i32,
}

/// `HurtByTargetGoal(mob, ignoreDamage...)` and its `setAlertOthers(...)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HurtBySpec {
    pub(crate) variant: HurtByVariant,
    pub(crate) ignore_damage: &'static [&'static str],
    /// The classes `setAlertOthers` exempts, `None` when it is not called.
    pub(crate) alert_others: Option<&'static [&'static str]>,
}

pub(crate) const fn nearest(target_class: &'static str, must_see: bool) -> GoalSpec {
    GoalSpec::Nearest(NearestSpec {
        variant: NearestVariant::Plain,
        target_class,
        interval: 10,
        must_see,
        selector: TargetSelector::None,
        unseen_memory_ticks: 60,
    })
}

const fn panic(speed: f64) -> GoalSpec {
    GoalSpec::Panic {
        speed,
        kind: PanicKind::Plain,
    }
}

const fn avoid(class: &'static str, max_dist: f32, walk_speed: f64, sprint_speed: f64) -> GoalSpec {
    GoalSpec::AvoidEntity(AvoidSpec {
        class,
        max_dist,
        walk_speed,
        sprint_speed,
        gate: AvoidGate::None,
    })
}

const fn hurt_by(variant: HurtByVariant, ignore_damage: &'static [&'static str]) -> GoalSpec {
    GoalSpec::HurtBy(HurtBySpec {
        variant,
        ignore_damage,
        alert_others: None,
    })
}

// Chained constructor arguments and setters; each leaves other specs as is.
impl GoalSpec {
    pub(crate) const fn every(self, interval: i32) -> Self {
        match self {
            Self::Nearest(spec) => Self::Nearest(NearestSpec { interval, ..spec }),
            other => other,
        }
    }

    pub(crate) const fn selector(self, selector: TargetSelector) -> Self {
        match self {
            Self::Nearest(spec) => Self::Nearest(NearestSpec { selector, ..spec }),
            other => other,
        }
    }

    const fn scare(self, scare: TemptScare) -> Self {
        match self {
            Self::Tempt { speed, items, .. } => Self::Tempt {
                speed,
                items,
                scare,
            },
            other => other,
        }
    }

    const fn panic_kind(self, kind: PanicKind) -> Self {
        match self {
            Self::Panic { speed, .. } => Self::Panic { speed, kind },
            other => other,
        }
    }

    const fn gate(self, gate: AvoidGate) -> Self {
        match self {
            Self::AvoidEntity(spec) => Self::AvoidEntity(AvoidSpec { gate, ..spec }),
            other => other,
        }
    }

    const fn variant(self, variant: NearestVariant) -> Self {
        match self {
            Self::Nearest(spec) => Self::Nearest(NearestSpec { variant, ..spec }),
            other => other,
        }
    }

    const fn unseen_memory(self, unseen_memory_ticks: i32) -> Self {
        match self {
            Self::Nearest(spec) => Self::Nearest(NearestSpec {
                unseen_memory_ticks,
                ..spec
            }),
            other => other,
        }
    }

    const fn alert_others(self, ignored: &'static [&'static str]) -> Self {
        match self {
            Self::HurtBy(spec) => Self::HurtBy(HurtBySpec {
                alert_others: Some(ignored),
                ..spec
            }),
            other => other,
        }
    }
}

/// `AbstractSkeleton`'s bow goal and its melee fallback
/// (`AbstractSkeleton.java:51-56`): `(hard, other)` attack intervals.
const fn skeleton_bow(hard_interval: i32, interval: i32) -> GoalSpec {
    GoalSpec::BowAttack(BowSpec {
        speed: 1.0,
        hard_interval,
        interval,
        radius: 15.0,
        melee_fallback: true,
    })
}

const fn ranged(speed: f64, interval_min: i32, interval_max: i32, radius: f32) -> GoalSpec {
    GoalSpec::RangedAttack(RangedSpec {
        kind: RangedKind::Plain,
        speed,
        interval_min,
        interval_max,
        radius,
    })
}

const fn melee(speed: f64, follow_unseen: bool) -> GoalSpec {
    GoalSpec::MeleeAttack {
        speed,
        follow_unseen,
        kind: MeleeKind::Plain,
    }
}

const fn look_at(target_kind: LookAtTargetKind, distance: f64, probability: f32) -> GoalSpec {
    GoalSpec::LookAt(LookAtGoalSpec::Standard(LookAtSpec {
        target_kind,
        distance,
        probability,
        only_horizontal: false,
    }))
}

// `LookAtPlayerGoal(..., onlyHorizontal = true)` keeps the head level.
const fn look_at_horizontally(
    target_kind: LookAtTargetKind,
    distance: f64,
    probability: f32,
) -> GoalSpec {
    GoalSpec::LookAt(LookAtGoalSpec::Standard(LookAtSpec {
        target_kind,
        distance,
        probability,
        only_horizontal: true,
    }))
}

const fn idle_look(players_only: bool, max_distance: f64, approach_speed: f64) -> GoalSpec {
    GoalSpec::LookAt(LookAtGoalSpec::IdleAttention {
        players_only,
        max_distance,
        approach_speed,
    })
}

const fn follow_parent(speed: f64) -> GoalSpec {
    GoalSpec::FollowParent {
        speed,
        mode: FollowMode::Animal,
    }
}

const fn move_towards_restriction(speed: f64) -> GoalSpec {
    GoalSpec::Stroll {
        speed,
        position: StrollPosition::Home { move_only: true },
        interval: super::stroll::DEFAULT_INTERVAL,
    }
}

const fn flee_sun(speed: f64) -> GoalSpec {
    GoalSpec::FleeSun { speed, fox: false }
}

const RANDOM_LOOK: GoalSpec = GoalSpec::RandomLookAround { camel: false };
const CAMEL_RANDOM_LOOK: GoalSpec = GoalSpec::RandomLookAround { camel: true };
const TRY_FIND_WATER: GoalSpec = GoalSpec::TryFindWater { move_look: false };

const fn tempt(speed: f64, items: TemptItems) -> GoalSpec {
    GoalSpec::Tempt {
        speed,
        items,
        scare: TemptScare::Never,
    }
}

const fn stroll(speed: f64) -> GoalSpec {
    GoalSpec::Stroll {
        speed,
        position: StrollPosition::Default,
        interval: super::stroll::DEFAULT_INTERVAL,
    }
}

const fn water_avoiding_stroll(speed: f64, probability: f32) -> GoalSpec {
    GoalSpec::Stroll {
        speed,
        position: StrollPosition::WaterAvoiding { probability },
        interval: super::stroll::DEFAULT_INTERVAL,
    }
}

const fn stroll_flying(speed: f64) -> GoalSpec {
    GoalSpec::Stroll {
        speed,
        position: StrollPosition::Flying,
        interval: super::stroll::DEFAULT_INTERVAL,
    }
}

const fn parrot_wander(speed: f64) -> GoalSpec {
    GoalSpec::Stroll {
        speed,
        position: StrollPosition::ParrotWander { probability: 0.001 },
        interval: super::stroll::DEFAULT_INTERVAL,
    }
}

impl GoalSpec {
    /// Controls each goal owns while running.
    pub(crate) const fn controls(self) -> GoalControls {
        const MOVE_LOOK: GoalControls = GoalControls::MOVE.union(GoalControls::LOOK);
        const MOVE_JUMP: GoalControls = GoalControls::MOVE.union(GoalControls::JUMP);
        match self {
            Self::Float | Self::ClimbPowderSnow => GoalControls::JUMP,
            Self::PandaSneeze
            | Self::PandaLieOnBack
            | Self::SilverfishWakeFriends
            | Self::SkeletonHorseTrap
            | Self::TryFindWater { move_look: false }
            | Self::FollowPlayerVehicle
            | Self::ShulkerPeek
            | Self::EndermanLeaveBlock
            | Self::EndermanTakeBlock
            | Self::CatRelaxOnOwner
            | Self::TraderUseItem(_)
            | Self::TurtleTravel
            | Self::LandOnOwnersShoulder => GoalControls::NONE,
            Self::SlimeFloat
            | Self::DolphinJump
            | Self::SitWhenOrderedTo
            | Self::SlimeKeepOnJumping
            | Self::TradeWithPlayer
            | Self::EndermanFreeze
            | Self::MoveToBlock(_)
            | Self::LeapAtTarget { .. } => MOVE_JUMP,
            Self::SlimeRandomDirection
            | Self::SlimeAttack
            | Self::GhastLook
            | Self::LookAt(LookAtGoalSpec::Standard(_) | LookAtGoalSpec::Trading)
            | Self::RandomLookAround { camel: true }
            | Self::Beg => GoalControls::LOOK,
            Self::EatBlock | Self::PandaRoll | Self::FoxSleep | Self::FoxFaceplant => {
                MOVE_LOOK.union(GoalControls::JUMP)
            }
            Self::Stroll {
                position: StrollPosition::Home { move_only: false },
                ..
            } => MOVE_LOOK,
            Self::FoxSearchItems
            | Self::PandaSit
            | Self::SilverfishMerge
            | Self::CamelIdlePose
            | Self::TraderTravel
            | Self::LlamaCaravan
            | Self::RaiderBanner
            | Self::RaiderTravel
            | Self::RaiderVillage
            | Self::RaiderCelebration
            | Self::MoveThroughVillage
            | Self::MoveTowardsTarget
            | Self::DrownedGoToWater
            | Self::DrownedSwimUp
            | Self::Swell
            | Self::VexRandomMove
            | Self::VexChargeAttack
            | Self::GhastRandomFloat
            | Self::LongDistancePatrol
            | Self::Panic { .. }
            | Self::FleeSun { .. }
            | Self::AvoidEntity(_)
            | Self::RunAroundLikeCrazy { .. }
            | Self::DolphinPlayWithItems
            | Self::Stroll { .. } => GoalControls::MOVE,
            Self::FoxPounce => MOVE_JUMP,
            Self::FoxPerch
            | Self::FoxStalk
            | Self::RaiderHoldGround
            | Self::BlazeAttack
            | Self::BreezeCombat
            | Self::WardenCombat
            | Self::AllayCollect
            | Self::WardenAwareness
            | Self::WardenLifecycle
            | Self::LookAt(LookAtGoalSpec::TraderInteract | LookAtGoalSpec::IdleAttention { .. })
            | Self::BowAttack(_)
            | Self::CrossbowAttack { .. }
            | Self::RangedAttack(_)
            | Self::ShulkerAttack
            | Self::Nautilus
            | Self::GoatRam
            | Self::GoatLongJump
            | Self::HoglinIdle
            | Self::PiglinIdle
            | Self::GuardianAttack
            | Self::OfferFlower
            | Self::BreathAir
            | Self::DolphinSwimWithPlayer
            | Self::DolphinSwimToTreasure
            | Self::SpellcasterCasting
            | Self::MeleeAttack { .. }
            | Self::SpearUse
            | Self::OcelotAttack
            | Self::RandomLookAround { camel: false }
            | Self::Tempt { .. }
            | Self::Breed { .. }
            | Self::FollowOwner { .. }
            | Self::FollowParent {
                mode:
                    FollowMode::HappyGhast
                    | FollowMode::Axolotl
                    | FollowMode::Goat
                    | FollowMode::Camel
                    | FollowMode::Hoglin,
                ..
            }
            | Self::TryFindWater { move_look: true }
            | Self::AxolotlPlayDead
            | Self::FollowMob { .. } => MOVE_LOOK,
            Self::SquidRandomMovement
            | Self::SquidFlee
            | Self::RestrictSun
            | Self::RandomStand
            | Self::BreakDoor
            | Self::FollowParent {
                mode: FollowMode::Animal,
                ..
            }
            | Self::ResetUniversalAnger { .. }
            | Self::GhastShootFireball
            | Self::Spell(_)
            | Self::VexCopyOwner => GoalControls::NONE,
            Self::Nearest(_)
            | Self::HurtBy(_)
            | Self::OwnerTarget(_)
            | Self::TraderLlamaDefend
            | Self::DefendVillage
            | Self::EndermanLookForPlayer
            | Self::BeeTarget(_) => GoalControls::TARGET,
        }
    }
}

macro_rules! mob_goals {
    ($($variant:ident($goal:ty),)*) => {
        /// A registered goal with its running state.
        #[derive(Clone)]
        pub(super) enum MobGoal {
            MoveToBlock(MoveToBlockGoal),
            $($variant($goal),)*
        }

        impl MobGoal {
            pub(super) const fn debug_name(&self) -> &'static str {
                match self {
                    Self::MoveToBlock(goal) => goal.kind().debug_name(),
                    $(Self::$variant(_) => stringify!($goal),)*
                }
            }
        }

        impl Goal for MobGoal {
            fn can_use(
                &mut self,
                context: &mut MobAiContext,
                targets: &mut TargetView<'_, '_, '_>,
            ) -> bool {
                match self {
                    Self::MoveToBlock(goal) => goal.can_use(context, targets),
                    $(Self::$variant(goal) => goal.can_use(context, targets),)*
                }
            }

            fn can_continue(
                &mut self,
                context: &mut MobAiContext,
                targets: &mut TargetView<'_, '_, '_>,
            ) -> bool {
                match self {
                    Self::MoveToBlock(goal) => goal.can_continue(context, targets),
                    $(Self::$variant(goal) => goal.can_continue(context, targets),)*
                }
            }

            fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
                match self {
                    Self::MoveToBlock(goal) => goal.start(context, targets),
                    $(Self::$variant(goal) => goal.start(context, targets),)*
                }
            }

            fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
                match self {
                    Self::MoveToBlock(goal) => goal.stop(context, targets),
                    $(Self::$variant(goal) => goal.stop(context, targets),)*
                }
            }

            fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
                match self {
                    Self::MoveToBlock(goal) => goal.tick(context, targets),
                    $(Self::$variant(goal) => goal.tick(context, targets),)*
                }
            }

            fn requires_update_every_tick(&self) -> bool {
                match self {
                    Self::MoveToBlock(goal) => goal.requires_update_every_tick(),
                    $(Self::$variant(goal) => goal.requires_update_every_tick(),)*
                }
            }

            fn is_interruptable(&self) -> bool {
                match self {
                    Self::MoveToBlock(goal) => goal.is_interruptable(),
                    $(Self::$variant(goal) => goal.is_interruptable(),)*
                }
            }
        }
    };
}

mob_goals! {
    FoxSearchItems(FoxSearchItemsGoal),
    FoxPerch(FoxPerchGoal),
    FoxSleep(FoxSleepGoal),
    FoxStalk(FoxStalkGoal),
    FoxPounce(FoxPounceGoal),
    FoxFaceplant(FoxFaceplantGoal),
    PandaLieOnBack(PandaLieOnBackGoal),
    PandaRoll(PandaRollGoal),
    PandaSit(PandaSitGoal),
    PandaSneeze(PandaSneezeGoal),
    SkeletonHorseTrap(SkeletonHorseTrapGoal),
    SilverfishWakeFriends(SilverfishWakeFriendsGoal),
    SilverfishMerge(SilverfishMergeGoal),
    Float(FloatGoal),
    ClimbPowderSnow(ClimbPowderSnowGoal),
    SlimeFloat(SlimeFloatGoal),
    SlimeAttack(SlimeAttackGoal),
    SlimeRandomDirection(SlimeRandomDirectionGoal),
    SlimeKeepOnJumping(SlimeKeepOnJumpingGoal),
    EatBlock(SheepEatBlockGoal),
    Swell(CreeperSwellGoal),
    BlazeAttack(BlazeAttackGoal),
    BreezeCombat(BreezeCombatGoal),
    WardenCombat(WardenCombatGoal),
    AllayCollect(AllayCollectGoal),
    WardenAwareness(WardenAwarenessGoal),
    WardenLifecycle(WardenLifecycleGoal),
    BowAttack(BowAttackGoal),
    CrossbowAttack(CrossbowAttackGoal),
    RangedAttack(RangedAttackGoal),
    GhastShootFireball(GhastShootFireballGoal),
    ShulkerAttack(ShulkerAttackGoal),
    ShulkerPeek(ShulkerPeekGoal),
    GuardianAttack(GuardianAttackGoal),
    OfferFlower(OfferFlowerGoal),
    MoveTowardsTarget(MoveTowardsTargetGoal),
    Nautilus(NautilusGoal),
    GoatRam(GoatRamGoal),
    GoatLongJump(GoatLongJumpGoal),
    CamelIdlePose(CamelIdlePoseGoal),
    HoglinIdle(HoglinIdleGoal),
    PiglinIdle(PiglinIdleGoal),
    BreathAir(BreathAirGoal),
    TryFindWater(TryFindWaterGoal),
    AxolotlPlayDead(PlayDeadGoal),
    DolphinSwimWithPlayer(SwimWithPlayerGoal),
    DolphinSwimToTreasure(SwimToTreasureGoal),
    DolphinPlayWithItems(PlayWithItemsGoal),
    DolphinJump(JumpGoal),
    FollowPlayerVehicle(FollowPlayerVehicleGoal),
    SpellcasterCasting(SpellcasterCastingGoal),
    Spell(SpellGoal),
    VexRandomMove(VexRandomMovementGoal),
    VexChargeAttack(VexChargeAttackGoal),
    SpearUse(SpearUseGoal),
    GhastRandomFloat(GhastRandomFloatAroundGoal),
    GhastLook(GhastLookGoal),
    SquidRandomMovement(SquidRandomMovementGoal),
    SquidFlee(SquidFleeGoal),
    RestrictSun(RestrictSunGoal),
    FleeSun(FleeSunGoal),
    AvoidEntity(AvoidEntityGoal),
    LongDistancePatrol(LongDistancePatrolGoal),
    RandomLookAround(RandomLookAroundGoal),
    RandomStand(RandomStandGoal),
    TradeWithPlayer(TradeWithPlayerGoal),
    TraderTravel(WanderToPositionGoal),
    LlamaCaravan(LlamaCaravanGoal),
    RaiderBanner(RaiderBannerGoal),
    RaiderTravel(RaiderTravelGoal),
    RaiderHoldGround(RaiderHoldGroundGoal),
    RaiderVillage(RaiderVillageGoal),
    MoveThroughVillage(MoveThroughVillageGoal),
    TurtleTravel(TurtleTravelGoal),
    RaiderCelebration(RaiderCelebrationGoal),
    TraderUseItem(TraderUseItemGoal),
    EndermanFreeze(EndermanFreezeWhenLookedAt),
    EndermanLeaveBlock(EndermanLeaveBlockGoal),
    EndermanTakeBlock(EndermanTakeBlockGoal),
    BreakDoor(BreakDoorGoal),
    MeleeAttack(MeleeAttackGoal),
    LeapAtTarget(LeapAtTargetGoal),
    OcelotAttack(OcelotAttackGoal),
    DrownedGoToWater(DrownedGoToWaterGoal),
    DrownedSwimUp(DrownedSwimUpGoal),
    LookAt(LookAtPlayerGoal),
    Beg(BegGoal),
    Tempt(TemptGoal),
    FollowParent(FollowParentGoal),
    Panic(PanicGoal),
    Breed(BreedGoal),
    RunAroundLikeCrazy(RunAroundLikeCrazyGoal),
    FollowOwner(FollowOwnerGoal),
    SitWhenOrderedTo(SitWhenOrderedToGoal),
    CatRelaxOnOwner(CatRelaxOnOwnerGoal),
    LandOnOwnersShoulder(LandOnOwnersShoulderGoal),
    FollowMob(FollowMobGoal),
    Stroll(RandomStrollGoal),
    Nearest(NearestAttackableTargetGoal),
    HurtBy(HurtByTargetGoal),
    ResetUniversalAnger(ResetUniversalAngerTargetGoal),
    Owner(OwnerTargetGoal),
    TraderLlamaDefend(TraderLlamaDefendGoal),
    VexCopyOwner(VexCopyOwnerTargetGoal),
    DefendVillage(DefendVillageTargetGoal),
    EndermanLookForPlayer(LookForPlayerGoal),
    Bee(BeeTargetGoal),
}

impl MobGoal {
    pub(super) fn new(spec: GoalSpec, entity_type_id: i32) -> Option<Self> {
        Self::new_with_follow_range(spec, entity_type_id, None)
    }

    pub(super) fn new_with_follow_range(
        spec: GoalSpec,
        entity_type_id: i32,
        follow_range_override: Option<f64>,
    ) -> Option<Self> {
        Some(match spec {
            GoalSpec::FoxSearchItems => Self::FoxSearchItems(FoxSearchItemsGoal),
            GoalSpec::FoxPerch => Self::FoxPerch(FoxPerchGoal::default()),
            GoalSpec::FoxSleep => Self::FoxSleep(FoxSleepGoal::default()),
            GoalSpec::FoxStalk => Self::FoxStalk(FoxStalkGoal),
            GoalSpec::FoxPounce => Self::FoxPounce(FoxPounceGoal),
            GoalSpec::FoxFaceplant => Self::FoxFaceplant(FoxFaceplantGoal::default()),
            GoalSpec::PandaLieOnBack => Self::PandaLieOnBack(PandaLieOnBackGoal::default()),
            GoalSpec::PandaRoll => Self::PandaRoll(PandaRollGoal),
            GoalSpec::PandaSit => Self::PandaSit(PandaSitGoal::default()),
            GoalSpec::PandaSneeze => Self::PandaSneeze(PandaSneezeGoal),
            GoalSpec::SkeletonHorseTrap => Self::SkeletonHorseTrap(SkeletonHorseTrapGoal),
            GoalSpec::SilverfishWakeFriends => {
                Self::SilverfishWakeFriends(SilverfishWakeFriendsGoal::default())
            }
            GoalSpec::SilverfishMerge => Self::SilverfishMerge(SilverfishMergeGoal::default()),
            GoalSpec::DrownedGoToWater => Self::DrownedGoToWater(DrownedGoToWaterGoal::default()),
            GoalSpec::DrownedSwimUp => Self::DrownedSwimUp(DrownedSwimUpGoal::default()),
            GoalSpec::Float => Self::Float(FloatGoal),
            GoalSpec::ClimbPowderSnow => Self::ClimbPowderSnow(ClimbPowderSnowGoal),
            GoalSpec::SlimeFloat => Self::SlimeFloat(SlimeFloatGoal),
            GoalSpec::SlimeAttack => Self::SlimeAttack(SlimeAttackGoal::default()),
            GoalSpec::SlimeRandomDirection => {
                Self::SlimeRandomDirection(SlimeRandomDirectionGoal::new())
            }
            GoalSpec::SlimeKeepOnJumping => Self::SlimeKeepOnJumping(SlimeKeepOnJumpingGoal),
            GoalSpec::EatBlock => Self::EatBlock(SheepEatBlockGoal::default()),
            GoalSpec::Swell => Self::Swell(CreeperSwellGoal::default()),
            GoalSpec::BlazeAttack => Self::BlazeAttack(BlazeAttackGoal::default()),
            GoalSpec::BreezeCombat => Self::BreezeCombat(BreezeCombatGoal::default()),
            GoalSpec::WardenCombat => Self::WardenCombat(WardenCombatGoal::default()),
            GoalSpec::AllayCollect => Self::AllayCollect(AllayCollectGoal::default()),
            GoalSpec::WardenAwareness => Self::WardenAwareness(WardenAwarenessGoal::default()),
            GoalSpec::WardenLifecycle => Self::WardenLifecycle(WardenLifecycleGoal::default()),
            GoalSpec::BowAttack(spec) => Self::BowAttack(BowAttackGoal::new(spec)),
            GoalSpec::CrossbowAttack { speed, radius } => {
                Self::CrossbowAttack(CrossbowAttackGoal::new(speed, radius))
            }
            GoalSpec::RangedAttack(spec) => Self::RangedAttack(RangedAttackGoal::new(spec)),
            GoalSpec::GhastShootFireball => {
                Self::GhastShootFireball(GhastShootFireballGoal::default())
            }
            GoalSpec::ShulkerAttack => Self::ShulkerAttack(ShulkerAttackGoal::default()),
            GoalSpec::ShulkerPeek => Self::ShulkerPeek(ShulkerPeekGoal::default()),
            GoalSpec::GuardianAttack => Self::GuardianAttack(GuardianAttackGoal::default()),
            GoalSpec::Nautilus => Self::Nautilus(NautilusGoal::default()),
            GoalSpec::GoatRam => Self::GoatRam(GoatRamGoal::default()),
            GoalSpec::GoatLongJump => Self::GoatLongJump(GoatLongJumpGoal::default()),
            GoalSpec::BreathAir => Self::BreathAir(BreathAirGoal),
            GoalSpec::TryFindWater { .. } => Self::TryFindWater(TryFindWaterGoal::default()),
            GoalSpec::AxolotlPlayDead => {
                Self::AxolotlPlayDead(crate::entity::axolotl::PlayDeadGoal)
            }
            GoalSpec::DolphinSwimWithPlayer => {
                Self::DolphinSwimWithPlayer(SwimWithPlayerGoal::default())
            }
            GoalSpec::DolphinSwimToTreasure => Self::DolphinSwimToTreasure(SwimToTreasureGoal),
            GoalSpec::DolphinPlayWithItems => {
                Self::DolphinPlayWithItems(PlayWithItemsGoal::default())
            }
            GoalSpec::DolphinJump => Self::DolphinJump(JumpGoal),
            GoalSpec::FollowPlayerVehicle => {
                Self::FollowPlayerVehicle(FollowPlayerVehicleGoal::default())
            }
            GoalSpec::SpellcasterCasting => Self::SpellcasterCasting(SpellcasterCastingGoal),
            GoalSpec::Spell(spell) => Self::Spell(SpellGoal::new(spell)),
            GoalSpec::VexRandomMove => Self::VexRandomMove(VexRandomMovementGoal),
            GoalSpec::VexChargeAttack => Self::VexChargeAttack(VexChargeAttackGoal::default()),
            GoalSpec::SpearUse => Self::SpearUse(SpearUseGoal::default()),
            GoalSpec::GhastRandomFloat => Self::GhastRandomFloat(GhastRandomFloatAroundGoal),
            GoalSpec::GhastLook => Self::GhastLook(GhastLookGoal),
            GoalSpec::SquidRandomMovement => {
                Self::SquidRandomMovement(SquidRandomMovementGoal::new())
            }
            GoalSpec::SquidFlee => Self::SquidFlee(SquidFleeGoal::default()),
            GoalSpec::RestrictSun => Self::RestrictSun(RestrictSunGoal),
            GoalSpec::FleeSun { speed, fox } => Self::FleeSun(FleeSunGoal::new(speed, fox)),
            GoalSpec::AvoidEntity(spec) => Self::AvoidEntity(AvoidEntityGoal::new(spec)),
            GoalSpec::LongDistancePatrol => Self::LongDistancePatrol(LongDistancePatrolGoal),
            GoalSpec::RandomLookAround { camel } => Self::RandomLookAround(if camel {
                RandomLookAroundGoal::camel()
            } else {
                RandomLookAroundGoal::new()
            }),
            GoalSpec::RandomStand => Self::RandomStand(RandomStandGoal::new()),
            GoalSpec::TradeWithPlayer => Self::TradeWithPlayer(TradeWithPlayerGoal),
            GoalSpec::TraderTravel => Self::TraderTravel(WanderToPositionGoal),
            GoalSpec::LlamaCaravan => Self::LlamaCaravan(LlamaCaravanGoal::new()),
            GoalSpec::RaiderBanner => Self::RaiderBanner(RaiderBannerGoal),
            GoalSpec::RaiderTravel => Self::RaiderTravel(RaiderTravelGoal::default()),
            GoalSpec::RaiderHoldGround => Self::RaiderHoldGround(RaiderHoldGroundGoal),
            GoalSpec::RaiderVillage => Self::RaiderVillage(RaiderVillageGoal::default()),
            GoalSpec::RaiderCelebration => Self::RaiderCelebration(RaiderCelebrationGoal),
            GoalSpec::TraderUseItem(kind) => Self::TraderUseItem(TraderUseItemGoal::new(kind)),
            GoalSpec::MoveThroughVillage => {
                Self::MoveThroughVillage(MoveThroughVillageGoal::default())
            }
            GoalSpec::TurtleTravel => Self::TurtleTravel(TurtleTravelGoal::default()),
            GoalSpec::EndermanFreeze => Self::EndermanFreeze(EndermanFreezeWhenLookedAt::default()),
            GoalSpec::EndermanLeaveBlock => Self::EndermanLeaveBlock(EndermanLeaveBlockGoal),
            GoalSpec::EndermanTakeBlock => Self::EndermanTakeBlock(EndermanTakeBlockGoal),
            GoalSpec::BreakDoor => Self::BreakDoor(BreakDoorGoal),
            GoalSpec::MeleeAttack {
                speed,
                follow_unseen,
                kind,
            } => Self::MeleeAttack(MeleeAttackGoal::new(speed, follow_unseen, kind)),
            GoalSpec::LeapAtTarget { yd } => Self::LeapAtTarget(LeapAtTargetGoal::new(yd)),
            GoalSpec::OcelotAttack => Self::OcelotAttack(OcelotAttackGoal::default()),
            GoalSpec::MoveToBlock(kind) => Self::MoveToBlock(MoveToBlockGoal::new(kind)),
            GoalSpec::LookAt(LookAtGoalSpec::Standard(spec)) => {
                Self::LookAt(LookAtPlayerGoal::new(spec))
            }
            GoalSpec::LookAt(LookAtGoalSpec::Trading) => Self::LookAt(LookAtPlayerGoal::trading()),
            GoalSpec::LookAt(LookAtGoalSpec::TraderInteract) => {
                Self::LookAt(LookAtPlayerGoal::new(LookAtSpec {
                    target_kind: LookAtTargetKind::Player,
                    distance: 3.0,
                    probability: 1.0,
                    only_horizontal: false,
                }))
            }
            GoalSpec::LookAt(LookAtGoalSpec::IdleAttention {
                players_only,
                max_distance,
                approach_speed,
            }) => Self::LookAt(LookAtPlayerGoal::idle_attention(
                players_only,
                max_distance,
                approach_speed,
            )),
            GoalSpec::CamelIdlePose => Self::CamelIdlePose(CamelIdlePoseGoal::default()),
            GoalSpec::HoglinIdle => Self::HoglinIdle(HoglinIdleGoal::default()),
            GoalSpec::PiglinIdle => Self::PiglinIdle(PiglinIdleGoal::default()),
            GoalSpec::OfferFlower => Self::OfferFlower(OfferFlowerGoal::default()),
            GoalSpec::MoveTowardsTarget => {
                Self::MoveTowardsTarget(MoveTowardsTargetGoal::default())
            }
            GoalSpec::Beg => Self::Beg(BegGoal::default()),
            GoalSpec::Tempt {
                speed,
                items,
                scare,
            } => Self::Tempt(TemptGoal::new(TemptSpec::new(
                items,
                scare,
                speed,
                entity_type_id,
            )?)),
            GoalSpec::FollowParent { speed, mode } => {
                Self::FollowParent(FollowParentGoal::new(speed, mode))
            }
            GoalSpec::Panic { speed, kind } => Self::Panic(PanicGoal::new(speed, kind)),
            GoalSpec::Breed { speed } => Self::Breed(BreedGoal::new(speed)),
            GoalSpec::RunAroundLikeCrazy { speed } => {
                Self::RunAroundLikeCrazy(RunAroundLikeCrazyGoal::new(speed))
            }
            GoalSpec::FollowOwner {
                speed,
                start_distance,
                stop_distance,
            } => Self::FollowOwner(FollowOwnerGoal::new(speed, start_distance, stop_distance)),
            GoalSpec::SitWhenOrderedTo => Self::SitWhenOrderedTo(SitWhenOrderedToGoal),
            GoalSpec::LandOnOwnersShoulder => {
                Self::LandOnOwnersShoulder(LandOnOwnersShoulderGoal::default())
            }
            GoalSpec::CatRelaxOnOwner => Self::CatRelaxOnOwner(CatRelaxOnOwnerGoal::default()),
            GoalSpec::FollowMob {
                speed,
                stop_distance,
                area_size,
            } => Self::FollowMob(FollowMobGoal::new(speed, stop_distance, area_size)),
            GoalSpec::Stroll {
                speed,
                position,
                interval,
            } => Self::Stroll(RandomStrollGoal::new(speed, position, interval)),
            GoalSpec::Nearest(spec) => Self::Nearest(match follow_range_override {
                Some(range) => {
                    NearestAttackableTargetGoal::from_spec_with_follow_range(spec, range)
                }
                None => NearestAttackableTargetGoal::from_spec(spec, entity_type_id)?,
            }),
            GoalSpec::HurtBy(spec) => Self::HurtBy(HurtByTargetGoal::from_spec(spec)),
            GoalSpec::ResetUniversalAnger { alert_others } => {
                Self::ResetUniversalAnger(ResetUniversalAngerTargetGoal::new(alert_others))
            }
            GoalSpec::OwnerTarget(memory) => Self::Owner(OwnerTargetGoal::new(memory)),
            GoalSpec::TraderLlamaDefend => Self::TraderLlamaDefend(TraderLlamaDefendGoal::new()),
            GoalSpec::VexCopyOwner => Self::VexCopyOwner(VexCopyOwnerTargetGoal::new()),
            GoalSpec::DefendVillage => Self::DefendVillage(DefendVillageTargetGoal::new()),
            GoalSpec::EndermanLookForPlayer => Self::EndermanLookForPlayer(LookForPlayerGoal),
            GoalSpec::BeeTarget(kind) => Self::Bee(BeeTargetGoal::new(kind)),
        })
    }
}

pub(crate) const fn from_plugin_goal(goal: crate::plugin::MobGoalSpec) -> GoalSpec {
    use crate::plugin::MobGoalSpec;
    match goal {
        MobGoalSpec::Float => GoalSpec::Float,
        MobGoalSpec::MeleeAttack {
            speed_modifier,
            following_target_even_if_not_seen,
        } => GoalSpec::MeleeAttack {
            speed: speed_modifier,
            follow_unseen: following_target_even_if_not_seen,
            kind: MeleeKind::Plain,
        },
        MobGoalSpec::WaterAvoidingRandomStroll { speed_modifier } => {
            water_avoiding_stroll(speed_modifier, 0.001)
        }
        MobGoalSpec::LookAtPlayer { look_distance } => {
            look_at(LookAtTargetKind::Player, look_distance as f64, 0.02)
        }
        MobGoalSpec::RandomLookAround => RANDOM_LOOK,
        MobGoalSpec::HurtByTarget { alert_others } => GoalSpec::HurtBy(HurtBySpec {
            variant: HurtByVariant::Plain,
            ignore_damage: &[],
            alert_others: if alert_others { Some(&[]) } else { None },
        }),
        MobGoalSpec::NearestAttackablePlayer { must_see } => nearest("Player", must_see),
    }
}

/// The mob type's runtime class's simple name.
pub(crate) fn runtime_class(entity_type_id: i32) -> Option<&'static str> {
    if entity_type_id >= bbb_protocol::entity_types::VANILLA_ENTITY_TYPES.len() as i32 {
        return Some("Monster");
    }
    mob_spec(entity_type_id).map(|spec| spec.class)
}

pub(crate) fn classes(entity_type_id: i32) -> impl Iterator<Item = &'static str> + Clone {
    let mut first = Some(runtime_class(entity_type_id));
    let mut current = None;
    // `successors` would resolve a parent even when the current class matches.
    std::iter::from_fn(move || {
        current = first.take().unwrap_or_else(|| current.and_then(superclass));
        current
    })
}

/// `instanceof` over the mob type's class chain, by simple class name.
pub(crate) fn is_instance_of(entity_type_id: i32, class: &str) -> bool {
    classes(entity_type_id).any(|candidate| candidate == class)
}

/// The direct superclass of each mob class up to `Mob`.
fn superclass(class: &str) -> Option<&'static str> {
    Some(match class {
        "Donkey" | "Llama" | "Mule" => "AbstractChestedHorse",
        "Cow" | "MushroomCow" => "AbstractCow",
        "AbstractSchoolingFish" | "Pufferfish" | "Tadpole" => "AbstractFish",
        "CopperGolem" | "IronGolem" | "Shulker" | "SnowGolem" => "AbstractGolem",
        "AbstractChestedHorse" | "Camel" | "Horse" | "SkeletonHorse" | "ZombieHorse" => {
            "AbstractHorse"
        }
        "Pillager" | "SpellcasterIllager" | "Vindicator" => "AbstractIllager",
        "Nautilus" | "ZombieNautilus" => "AbstractNautilus",
        "Piglin" | "PiglinBrute" => "AbstractPiglin",
        "Cod" | "Salmon" | "TropicalFish" => "AbstractSchoolingFish",
        "Bogged" | "Parched" | "Skeleton" | "Stray" | "WitherSkeleton" => "AbstractSkeleton",
        "Villager" | "WanderingTrader" => "AbstractVillager",
        "AbstractVillager" | "AgeableWaterCreature" | "Animal" => "AgeableMob",
        "Dolphin" | "Squid" => "AgeableWaterCreature",
        "Bat" => "AmbientCreature",
        "AbstractCow" | "AbstractHorse" | "Armadillo" | "Axolotl" | "Bee" | "Chicken" | "Fox"
        | "Frog" | "Goat" | "HappyGhast" | "Hoglin" | "Ocelot" | "Panda" | "Pig" | "PolarBear"
        | "Rabbit" | "Sheep" | "Sniffer" | "Strider" | "TamableAnimal" | "Turtle" => "Animal",
        "CamelHusk" => "Camel",
        "ElderGuardian" => "Guardian",
        "TraderLlama" => "Llama",
        "AmbientCreature" | "EnderDragon" | "Ghast" | "PathfinderMob" | "Phantom" | "Slime" => {
            "Mob"
        }
        "AbstractPiglin" | "AbstractSkeleton" | "Blaze" | "Breeze" | "Creaking" | "Creeper"
        | "EnderMan" | "Endermite" | "Giant" | "Guardian" | "PatrollingMonster" | "Silverfish"
        | "Spider" | "Vex" | "Warden" | "WitherBoss" | "Zoglin" | "Zombie" => "Monster",
        "AbstractGolem" | "AgeableMob" | "Allay" | "Monster" | "WaterAnimal" => "PathfinderMob",
        "Raider" => "PatrollingMonster",
        "AbstractIllager" | "Ravager" | "Witch" => "Raider",
        "Parrot" => "ShoulderRidingEntity",
        "MagmaCube" => "Slime",
        "Evoker" | "Illusioner" => "SpellcasterIllager",
        "CaveSpider" => "Spider",
        "GlowSquid" => "Squid",
        "AbstractNautilus" | "Cat" | "ShoulderRidingEntity" | "Wolf" => "TamableAnimal",
        "AbstractFish" => "WaterAnimal",
        "Drowned" | "Husk" | "ZombieVillager" | "ZombifiedPiglin" => "Zombie",
        _ => return None,
    })
}
