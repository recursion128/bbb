//! Each mob type's `registerGoals`, in the order Vanilla runs its `addGoal`s.

use crate::entity::wandering_trader::TraderDrinkKind;
use bbb_protocol::entity_types::*;
use bbb_protocol::item_types::VANILLA_ITEM_CARROT_ON_A_STICK_ID;

use super::super::avoid::AvoidGate;
use super::super::follow_parent::FollowMode;
use super::super::look_at::LookAtTargetKind::*;
use super::super::melee::MeleeKind;
use super::super::move_to_block::MoveToBlockKind;
use super::super::navigation::{GroundTuning, MoveControlKind};
use super::super::navigation::{
    NavigationFactory, ALLAY_NAVIGATION, BEE_NAVIGATION, FLYING_NAVIGATION,
};
use super::super::panic::PanicKind;
use super::super::random_look::LookControlKind;
use super::super::ranged::{BowSpec, RangedKind, RangedSpec, Spell};
use super::super::target::bee::BeeTargetKind;
use super::super::target::goals::{HurtByVariant, NearestVariant};
use super::super::target::owner::OwnerMemory;
use super::super::target::selector::TargetSelector;
use super::super::tempt::TemptItems::*;
use super::super::tempt::TemptScare;
use super::GoalSpec::*;
use super::{
    avoid, flee_sun, follow_parent, hurt_by, idle_look, look_at, look_at_horizontally, melee,
    move_towards_restriction, nearest, panic, parrot_wander, ranged, skeleton_bow, stroll,
    stroll_flying, tempt, water_avoiding_stroll, LookAtGoalSpec, MobSpec, CAMEL_RANDOM_LOOK,
    RANDOM_LOOK, TRY_FIND_WATER,
};

const GROUND: Option<NavigationFactory> = Some(NavigationFactory::Ground);
const FISH_GOALS: &[(i32, super::GoalSpec)] = &[
    (0, panic(1.25)),
    (2, avoid("Player", 8.0, 1.6, 1.4)),
    (
        4,
        Stroll {
            speed: 1.0,
            position: super::StrollPosition::Swimming,
            interval: 40,
        },
    ),
];

const WATER_NO_BREACH: Option<NavigationFactory> = Some(NavigationFactory::Water {
    allow_breaching: false,
});

const fn fish(name: &'static str) -> MobSpec {
    MobSpec {
        move_control: MoveControlKind::Fish,
        goals: FISH_GOALS,
        ..MobSpec::new(name, WATER_NO_BREACH)
    }
}

const SKELETON_TARGETS: &[(i32, super::GoalSpec)] = &[
    (1, hurt_by(HurtByVariant::Plain, &[])),
    (2, nearest("Player", true)),
    (3, nearest("IronGolem", true)),
    (
        3,
        nearest("Turtle", true).selector(TargetSelector::TurtleBabyOnLand),
    ),
];

const WITHER_SKELETON_TARGETS: &[(i32, super::GoalSpec)] = &[
    (3, nearest("AbstractPiglin", true)),
    (1, hurt_by(HurtByVariant::Plain, &[])),
    (2, nearest("Player", true)),
    (3, nearest("IronGolem", true)),
    (
        3,
        nearest("Turtle", true).selector(TargetSelector::TurtleBabyOnLand),
    ),
];

const fn skeleton_goals(hard_interval: i32, interval: i32) -> [(i32, super::GoalSpec); 7] {
    [
        (2, RestrictSun),
        (3, flee_sun(1.0)),
        (3, avoid("Wolf", 6.0, 1.0, 1.2)),
        (4, skeleton_bow(hard_interval, interval)),
        (5, water_avoiding_stroll(1.0, 0.001)),
        (6, look_at(Player, 8.0, 0.02)),
        (6, RANDOM_LOOK),
    ]
}

const SKELETON_GOALS: &[(i32, super::GoalSpec)] = &skeleton_goals(20, 40);
const BOGGED_GOALS: &[(i32, super::GoalSpec)] = &skeleton_goals(50, 70);

const ZOMBIE_GOALS: &[(i32, super::GoalSpec)] = &[
    (4, MoveToBlock(MoveToBlockKind::ZombieAttackTurtleEgg)),
    (2, SpearUse),
    (
        3,
        MeleeAttack {
            speed: 1.0,
            follow_unseen: false,
            kind: MeleeKind::Zombie,
        },
    ),
    (8, look_at(Player, 8.0, 0.02)),
    (8, RANDOM_LOOK),
    (6, MoveThroughVillage),
    (7, water_avoiding_stroll(1.0, 0.001)),
];

const ZOMBIE_TARGETS: &[(i32, super::GoalSpec)] = &[
    (
        1,
        hurt_by(HurtByVariant::Plain, &[]).alert_others(&["ZombifiedPiglin"]),
    ),
    (2, nearest("Player", true)),
    (3, nearest("AbstractVillager", false)),
    (3, nearest("IronGolem", true)),
    (
        5,
        nearest("Turtle", true).selector(TargetSelector::TurtleBabyOnLand),
    ),
];

const SPIDER_GOALS: &[(i32, super::GoalSpec)] = &[
    (
        2,
        avoid("Armadillo", 6.0, 1.0, 1.2).gate(AvoidGate::SpiderFromArmadillo),
    ),
    (3, LeapAtTarget { yd: 0.4 }),
    (
        4,
        MeleeAttack {
            speed: 1.0,
            follow_unseen: true,
            kind: MeleeKind::Spider,
        },
    ),
    (1, Float),
    (5, water_avoiding_stroll(0.8, 0.001)),
    (6, look_at(Player, 8.0, 0.02)),
    (6, RANDOM_LOOK),
];

const SPIDER_TARGETS: &[(i32, super::GoalSpec)] = &[
    (1, hurt_by(HurtByVariant::Plain, &[])),
    (2, nearest("Player", true).variant(NearestVariant::Spider)),
    (
        3,
        nearest("IronGolem", true).variant(NearestVariant::Spider),
    ),
];

const COW_GOALS: &[(i32, super::GoalSpec)] = &[
    (0, Float),
    (1, panic(2.0)),
    (2, Breed { speed: 1.0 }),
    (3, tempt(1.25, Tag("minecraft:cow_food"))),
    (4, follow_parent(1.25)),
    (5, water_avoiding_stroll(1.0, 0.001)),
    (6, look_at(Player, 6.0, 0.02)),
    (7, RANDOM_LOOK),
];

const HORSE_GOALS: &[(i32, super::GoalSpec)] = &[
    (1, RunAroundLikeCrazy { speed: 1.2 }),
    (2, Breed { speed: 1.0 }),
    (4, follow_parent(1.0)),
    (6, water_avoiding_stroll(0.7, 0.001)),
    (7, look_at(Player, 6.0, 0.02)),
    (8, RANDOM_LOOK),
    (9, RandomStand),
    (0, Float),
    (1, panic(1.2).panic_kind(PanicKind::Mount)),
    (3, tempt(1.25, Tag("minecraft:horse_tempt_items"))),
];

const SQUID_GOALS: &[(i32, super::GoalSpec)] = &[(0, SquidRandomMovement), (1, SquidFlee)];

const SLIME_GOALS: &[(i32, super::GoalSpec)] = &[
    (1, SlimeFloat),
    (2, SlimeAttack),
    (3, SlimeRandomDirection),
    (5, SlimeKeepOnJumping),
];

const SLIME_TARGETS: &[(i32, super::GoalSpec)] = &[
    (
        1,
        nearest("Player", true).selector(TargetSelector::WithinFourBlocksHeight),
    ),
    (3, nearest("IronGolem", true)),
];

const GUARDIAN_TARGETS: &[(i32, super::GoalSpec)] = &[(
    1,
    nearest("LivingEntity", true).selector(TargetSelector::GuardianAttack),
)];

const fn guardian_goals(stroll_interval: i32) -> [(i32, super::GoalSpec); 6] {
    [
        (4, GuardianAttack),
        (
            5,
            Stroll {
                speed: 1.0,
                position: super::StrollPosition::Home { move_only: false },
                interval: 0,
            },
        ),
        (
            7,
            Stroll {
                speed: 1.0,
                position: super::StrollPosition::Default,
                interval: stroll_interval,
            },
        ),
        (8, look_at(Player, 8.0, 0.02)),
        (8, look_at(Guardian, 12.0, 0.01)),
        (9, RANDOM_LOOK),
    ]
}

const GUARDIAN_GOALS: &[(i32, super::GoalSpec)] = &guardian_goals(80);
const ELDER_GUARDIAN_GOALS: &[(i32, super::GoalSpec)] = &guardian_goals(400);

const HOSTILE_ALERT_TARGETS: &[(i32, super::GoalSpec)] = &[
    (1, hurt_by(HurtByVariant::Plain, &[]).alert_others(&[])),
    (2, nearest("Player", true)),
];

/// The goals `Rabbit.setVariant(EVIL)` adds.
pub(crate) const KILLER_BUNNY: MobSpec = MobSpec {
    jump_control: false,
    goals: &[(4, melee(1.4, true))],
    targets: &[
        (1, hurt_by(HurtByVariant::Plain, &[]).alert_others(&[])),
        (2, nearest("Player", true)),
        (2, nearest("Wolf", true)),
    ],
    ..MobSpec::new("Rabbit", GROUND)
};

pub(crate) const GHASTLING: MobSpec = MobSpec {
    goals: &[
        (0, Float),
        (0, panic(2.0).panic_kind(PanicKind::HappyGhast)),
        (1, tempt(1.25, Tag("minecraft:happy_ghast_food"))),
        (
            2,
            FollowParent {
                speed: 1.1,
                mode: FollowMode::HappyGhast,
            },
        ),
        (
            3,
            Stroll {
                speed: 1.0,
                position: super::StrollPosition::AirAndWater,
                interval: 1,
            },
        ),
    ],
    ..MobSpec::new("HappyGhast", GROUND)
};

pub(crate) fn mob_spec(entity_type_id: i32) -> Option<&'static MobSpec> {
    Some(match entity_type_id {
        VANILLA_ENTITY_TYPE_ALLAY_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (0, Float),
                        (1, panic(2.5)),
                        (2, AllayCollect),
                        (3, idle_look(false, 6.0, 1.0)),
                        (4, stroll_flying(1.0)),
                    ],
                    ..MobSpec::new("Allay", Some(NavigationFactory::Flying(ALLAY_NAVIGATION)))
                }
            }
        }
        // Armadillo, axolotl, goat, hoglin and sniffer breed through a brain in
        // Vanilla; a `BreedGoal` row gives the same mating.
        VANILLA_ENTITY_TYPE_ARMADILLO_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::FLOAT,
                    goals: &[
                        (0, Float),
                        (1, panic(2.0).panic_kind(PanicKind::AnimalEnvironmental)),
                        (2, Breed { speed: 1.0 }),
                        (3, tempt(1.25, Tag("minecraft:armadillo_food"))),
                        (
                            4,
                            FollowParent {
                                speed: 1.25,
                                mode: FollowMode::Goat,
                            },
                        ),
                        (5, idle_look(true, 6.0, 1.0)),
                        (
                            6,
                            Stroll {
                                speed: 1.0,
                                position: super::StrollPosition::Land,
                                interval: 20,
                            },
                        ),
                        (7, CAMEL_RANDOM_LOOK),
                    ],
                    ..MobSpec::new("Armadillo", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => {
            const {
                &MobSpec {
                    move_control: MoveControlKind::AxolotlSwimming,
                    look_control: Some(LookControlKind::SmoothSwimming(20.0)),
                    max_head_x_rot: Some(1.0),
                    // AxolotlAi.
                    goals: &[
                        (-1, AxolotlPlayDead),
                        (0, Breed { speed: 0.2 }),
                        (
                            1,
                            MeleeAttack {
                                speed: 0.6,
                                follow_unseen: false,
                                kind: MeleeKind::Axolotl,
                            },
                        ),
                        (2, tempt(0.15, Tag("minecraft:axolotl_food"))),
                        (
                            3,
                            FollowParent {
                                speed: 0.15,
                                mode: FollowMode::Axolotl,
                            },
                        ),
                        (4, super::GoalSpec::TryFindWater { move_look: true }),
                        (5, idle_look(true, 6.0, 0.15)),
                        (
                            6,
                            Stroll {
                                speed: 0.15,
                                position: super::StrollPosition::Axolotl,
                                interval: 1,
                            },
                        ),
                    ],
                    targets: &[(
                        0,
                        super::GoalSpec::Nearest(super::NearestSpec {
                            variant: NearestVariant::Axolotl,
                            target_class: "LivingEntity",
                            interval: 20,
                            must_see: true,
                            selector: TargetSelector::AxolotlPrey,
                            unseen_memory_ticks: 0,
                        }),
                    )],
                    ..MobSpec::new("Axolotl", Some(NavigationFactory::Amphibious))
                }
            }
        }
        VANILLA_ENTITY_TYPE_BAT_ID => const { &MobSpec::new("Bat", GROUND) },
        VANILLA_ENTITY_TYPE_BEE_ID => {
            const {
                &MobSpec {
                    look_control: None,
                    goals: &[
                        (
                            0,
                            MeleeAttack {
                                speed: 1.4,
                                follow_unseen: true,
                                kind: MeleeKind::Bee,
                            },
                        ),
                        (2, Breed { speed: 1.0 }),
                        (3, tempt(1.25, Tag("minecraft:bee_food"))),
                        (5, follow_parent(1.25)),
                        (9, Float),
                    ],
                    targets: &[
                        (1, BeeTarget(BeeTargetKind::HurtBy)),
                        (2, BeeTarget(BeeTargetKind::Nearest)),
                        (3, ResetUniversalAnger { alert_others: true }),
                    ],
                    ..MobSpec::new("Bee", Some(NavigationFactory::Flying(BEE_NAVIGATION)))
                }
            }
        }
        VANILLA_ENTITY_TYPE_BLAZE_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (4, BlazeAttack),
                        (5, move_towards_restriction(1.0)),
                        (7, water_avoiding_stroll(1.0, 0.0)),
                        (8, look_at(Player, 8.0, 0.02)),
                        (8, RANDOM_LOOK),
                    ],
                    targets: HOSTILE_ALERT_TARGETS,
                    ..MobSpec::new("Blaze", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_BOGGED_ID => {
            const {
                &MobSpec {
                    goals: BOGGED_GOALS,
                    targets: SKELETON_TARGETS,
                    ..MobSpec::new("Bogged", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_BREEZE_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (0, Float),
                        (1, BreezeCombat),
                        (
                            5,
                            Stroll {
                                speed: 0.6,
                                position: super::StrollPosition::Land,
                                // RunOne of a stroll (weight 2) and DoNothing(20, 100) (weight 1).
                                interval: 20,
                            },
                        ),
                    ],
                    targets: &[
                        (1, hurt_by(HurtByVariant::Breeze, &[])),
                        (2, nearest("LivingEntity", true)),
                    ],
                    ..MobSpec::new("Breeze", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_CAMEL_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::CAMEL,
                    goals: &[
                        (0, Float),
                        (1, panic(4.0).panic_kind(PanicKind::Camel)),
                        (2, Breed { speed: 1.0 }),
                        (3, tempt(2.5, Tag("minecraft:camel_food"))),
                        (4, CamelIdlePose),
                        (
                            6,
                            FollowParent {
                                speed: 2.5,
                                mode: FollowMode::Camel,
                            },
                        ),
                        (7, idle_look(true, 6.0, 2.0)),
                        (
                            8,
                            Stroll {
                                speed: 2.0,
                                position: super::StrollPosition::Land,
                                interval: 20,
                            },
                        ),
                        (9, CAMEL_RANDOM_LOOK),
                    ],
                    ..MobSpec::new("Camel", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::CAMEL,
                    goals: &[
                        (0, Float),
                        (1, panic(4.0).panic_kind(PanicKind::Camel)),
                        (3, tempt(2.5, Tag("minecraft:camel_husk_food"))),
                        (4, CamelIdlePose),
                        (7, idle_look(true, 6.0, 2.0)),
                        (
                            8,
                            Stroll {
                                speed: 2.0,
                                position: super::StrollPosition::Land,
                                interval: 20,
                            },
                        ),
                        (9, CAMEL_RANDOM_LOOK),
                    ],
                    ..MobSpec::new("CamelHusk", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_CAT_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (8, LeapAtTarget { yd: 0.3 }),
                        (9, OcelotAttack),
                        (1, Float),
                        (2, SitWhenOrderedTo),
                        (3, CatRelaxOnOwner),
                        (
                            4,
                            tempt(0.6, Tag("minecraft:cat_food")).scare(TemptScare::Cat),
                        ),
                        (5, MoveToBlock(MoveToBlockKind::CatLieOnBed)),
                        (7, MoveToBlock(MoveToBlockKind::CatSitOnBlock)),
                        (
                            6,
                            FollowOwner {
                                speed: 1.0,
                                start_distance: 10.0,
                                stop_distance: 5.0,
                            },
                        ),
                        (1, panic(1.5).panic_kind(PanicKind::TamablePlain)),
                        (4, avoid("Player", 16.0, 0.8, 1.33).gate(AvoidGate::Untamed)),
                        (10, Breed { speed: 0.8 }),
                        (11, water_avoiding_stroll(0.8, 1.0000001e-5)),
                        (12, look_at(Player, 10.0, 0.02)),
                    ],
                    targets: &[
                        (
                            1,
                            nearest("Rabbit", false).variant(NearestVariant::NonTameRandom),
                        ),
                        (
                            1,
                            nearest("Turtle", false)
                                .selector(TargetSelector::TurtleBabyOnLand)
                                .variant(NearestVariant::NonTameRandom),
                        ),
                    ],
                    ..MobSpec::new("Cat", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID => {
            const {
                &MobSpec {
                    goals: SPIDER_GOALS,
                    targets: SPIDER_TARGETS,
                    ..MobSpec::new("CaveSpider", Some(NavigationFactory::WallClimber))
                }
            }
        }
        VANILLA_ENTITY_TYPE_CHICKEN_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (0, Float),
                        (1, panic(1.4)),
                        (2, Breed { speed: 1.0 }),
                        (3, tempt(1.0, Tag("minecraft:chicken_food"))),
                        (4, follow_parent(1.1)),
                        (5, water_avoiding_stroll(1.0, 0.001)),
                        (6, look_at(Player, 6.0, 0.02)),
                        (7, RANDOM_LOOK),
                    ],
                    ..MobSpec::new("Chicken", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_COD_ID => const { &fish("Cod") },
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::required(48.0),
                    // CopperGolemAi.
                    goals: &[
                        (0, panic(1.5).panic_kind(PanicKind::Animal)),
                        (1, look_at(Player, 6.0, 0.02)),
                        // A RunOne of this stroll and DoNothing(30, 60), each weight 1,
                        // only while the transport cooldown counts down.
                        (
                            2,
                            Stroll {
                                speed: 1.0,
                                position: super::StrollPosition::LandNear { xz: 2, y: 2 },
                                interval: 22,
                            },
                        ),
                    ],
                    ..MobSpec::new("CopperGolem", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_COW_ID => {
            const {
                &MobSpec {
                    goals: COW_GOALS,
                    ..MobSpec::new("Cow", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_CREAKING_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::FLOAT,
                    look_control: None,
                    ..MobSpec::new("Creaking", None)
                }
            }
        }
        VANILLA_ENTITY_TYPE_CREEPER_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::CREEPER,
                    goals: &[
                        (4, melee(1.0, false)),
                        (1, Float),
                        (2, Swell),
                        (3, avoid("Ocelot", 6.0, 1.0, 1.2)),
                        (3, avoid("Cat", 6.0, 1.0, 1.2)),
                        (5, water_avoiding_stroll(0.8, 0.001)),
                        (6, look_at(Player, 8.0, 0.02)),
                        (6, RANDOM_LOOK),
                    ],
                    targets: &[
                        (1, nearest("Player", true)),
                        (2, hurt_by(HurtByVariant::Plain, &[])),
                    ],
                    ..MobSpec::new("Creeper", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => {
            const {
                &MobSpec {
                    move_control: MoveControlKind::SmoothSwimming,
                    look_control: Some(LookControlKind::SmoothSwimming(10.0)),
                    max_head_x_rot: Some(1.0),
                    goals: &[
                        (0, BreathAir),
                        (0, TRY_FIND_WATER),
                        (1, DolphinSwimToTreasure),
                        (2, DolphinSwimWithPlayer),
                        (4, RANDOM_LOOK),
                        (5, look_at(Player, 6.0, 0.02)),
                        (6, melee(1.2, true)),
                        (8, DolphinPlayWithItems),
                        (9, avoid("Guardian", 8.0, 1.0, 1.0)),
                        (
                            4,
                            Stroll {
                                speed: 1.0,
                                position: super::StrollPosition::Swimming,
                                interval: 10,
                            },
                        ),
                        (5, DolphinJump),
                        (8, FollowPlayerVehicle),
                    ],
                    targets: &[(
                        1,
                        hurt_by(HurtByVariant::Plain, &["Guardian"]).alert_others(&[]),
                    )],
                    ..MobSpec::new(
                        "Dolphin",
                        Some(NavigationFactory::Water {
                            allow_breaching: true,
                        }),
                    )
                }
            }
        }
        VANILLA_ENTITY_TYPE_DONKEY_ID => {
            const {
                &MobSpec {
                    goals: HORSE_GOALS,
                    ..MobSpec::new("Donkey", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_DROWNED_ID => {
            const {
                &MobSpec {
                    move_control: MoveControlKind::Drowned,
                    goals: &[
                        (1, DrownedGoToWater),
                        // Drowned: speed 1, interval 40, radius 10.
                        (
                            2,
                            RangedAttack(RangedSpec {
                                kind: RangedKind::DrownedTrident,
                                speed: 1.0,
                                interval_min: 40,
                                interval_max: 40,
                                radius: 10.0,
                            }),
                        ),
                        (5, MoveToBlock(MoveToBlockKind::DrownedGoToBeach)),
                        (6, DrownedSwimUp),
                        (7, stroll(1.0)),
                        (4, MoveToBlock(MoveToBlockKind::ZombieAttackTurtleEgg)),
                        (
                            2,
                            MeleeAttack {
                                speed: 1.0,
                                follow_unseen: false,
                                kind: MeleeKind::Drowned,
                            },
                        ),
                        (8, look_at(Player, 8.0, 0.02)),
                        (8, RANDOM_LOOK),
                    ],
                    targets: &[
                        (
                            1,
                            hurt_by(HurtByVariant::Plain, &["Drowned"])
                                .alert_others(&["ZombifiedPiglin"]),
                        ),
                        (
                            2,
                            nearest("Player", true).selector(TargetSelector::DrownedOkTarget),
                        ),
                        (3, nearest("AbstractVillager", false)),
                        (3, nearest("IronGolem", true)),
                        (3, nearest("Axolotl", true)),
                        (
                            5,
                            nearest("Turtle", true).selector(TargetSelector::TurtleBabyOnLand),
                        ),
                    ],
                    ..MobSpec::new("Drowned", Some(NavigationFactory::Amphibious))
                }
            }
        }
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID => {
            const {
                &MobSpec {
                    move_control: MoveControlKind::Guardian,
                    max_head_x_rot: Some(180.0),
                    goals: ELDER_GUARDIAN_GOALS,
                    targets: GUARDIAN_TARGETS,
                    ..MobSpec::new("ElderGuardian", WATER_NO_BREACH)
                }
            }
        }
        VANILLA_ENTITY_TYPE_ENDERMAN_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (2, melee(1.0, false)),
                        (0, Float),
                        (1, EndermanFreeze),
                        (7, water_avoiding_stroll(1.0, 0.0)),
                        (8, look_at(Player, 8.0, 0.02)),
                        (8, RANDOM_LOOK),
                        (10, EndermanLeaveBlock),
                        (11, EndermanTakeBlock),
                    ],
                    targets: &[
                        (1, EndermanLookForPlayer),
                        (2, hurt_by(HurtByVariant::Plain, &[])),
                        (3, nearest("Endermite", true)),
                        (
                            4,
                            ResetUniversalAnger {
                                alert_others: false,
                            },
                        ),
                    ],
                    ..MobSpec::new("EnderMan", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_ENDERMITE_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (2, melee(1.0, false)),
                        (1, Float),
                        (1, ClimbPowderSnow),
                        (3, water_avoiding_stroll(1.0, 0.001)),
                        (7, look_at(Player, 8.0, 0.02)),
                        (8, RANDOM_LOOK),
                    ],
                    targets: HOSTILE_ALERT_TARGETS,
                    ..MobSpec::new("Endermite", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => const { &MobSpec::new("EnderDragon", GROUND) },
        VANILLA_ENTITY_TYPE_EVOKER_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, RaiderBanner),
                        (3, RaiderTravel),
                        (4, RaiderVillage),
                        (5, RaiderCelebration),
                        (4, LongDistancePatrol),
                        (0, Float),
                        (2, avoid("Player", 8.0, 0.6, 1.0)),
                        (3, avoid("Creaking", 8.0, 0.6, 1.0)),
                        (1, SpellcasterCasting),
                        (4, Spell(Spell::SummonVex)),
                        (5, Spell(Spell::Fangs)),
                        (6, Spell(Spell::Wololo)),
                        (8, stroll(0.6)),
                        (9, look_at(Player, 3.0, 1.0)),
                        (10, look_at(AnyMob, 8.0, 0.02)),
                    ],
                    targets: &[
                        (
                            1,
                            hurt_by(HurtByVariant::Plain, &["Raider"]).alert_others(&[]),
                        ),
                        (2, nearest("Player", true).unseen_memory(300)),
                        (3, nearest("AbstractVillager", false).unseen_memory(300)),
                        (3, nearest("IronGolem", false)),
                    ],
                    ..MobSpec::new("Evoker", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_FOX_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::required(32.0),
                    goals: &[
                        (0, Float),
                        (0, ClimbPowderSnow),
                        (1, FoxFaceplant),
                        (5, FoxStalk),
                        (6, FoxPounce),
                        (2, panic(2.2).panic_kind(PanicKind::Fox)),
                        (
                            4,
                            avoid("Player", 16.0, 1.6, 1.4).gate(AvoidGate::FoxFromPlayer),
                        ),
                        (4, avoid("Wolf", 8.0, 1.6, 1.4).gate(AvoidGate::FoxFromWolf)),
                        (
                            4,
                            avoid("PolarBear", 8.0, 1.6, 1.4).gate(AvoidGate::FoxNotDefending),
                        ),
                        (
                            7,
                            MeleeAttack {
                                speed: 1.2,
                                follow_unseen: true,
                                kind: MeleeKind::Fox,
                            },
                        ),
                        (
                            6,
                            super::GoalSpec::FleeSun {
                                speed: 1.25,
                                fox: true,
                            },
                        ),
                        (7, FoxSleep),
                        (3, Breed { speed: 1.0 }),
                        (10, LeapAtTarget { yd: 0.4 }),
                        (11, water_avoiding_stroll(1.0, 0.001)),
                        (8, follow_parent(1.25)),
                        (11, FoxSearchItems),
                        (
                            9,
                            Stroll {
                                speed: 1.0,
                                position: super::StrollPosition::FoxVillage,
                                interval: 200,
                            },
                        ),
                        (10, MoveToBlock(MoveToBlockKind::FoxEatBerries)),
                        (12, look_at(Player, 24.0, 0.02)),
                        (13, FoxPerch),
                    ],
                    targets: &[(
                        3,
                        nearest("LivingEntity", false)
                            .selector(TargetSelector::FoxUntrustedAttacker)
                            .variant(NearestVariant::FoxDefendTrusted),
                    )],
                    ..MobSpec::new("Fox", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_FROG_ID => {
            const {
                &MobSpec {
                    move_control: MoveControlKind::SmoothSwimming,
                    look_control: None,
                    ..MobSpec::new("Frog", Some(NavigationFactory::Frog))
                }
            }
        }
        VANILLA_ENTITY_TYPE_GHAST_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (5, GhastRandomFloat),
                        (7, GhastLook),
                        (7, GhastShootFireball),
                    ],
                    targets: &[(
                        1,
                        nearest("Player", true).selector(TargetSelector::WithinFourBlocksHeight),
                    )],
                    ..MobSpec::new("Ghast", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (3, Float),
                        (4, tempt(1.0, HappyGhast)),
                        (5, GhastRandomFloat),
                    ],
                    ..MobSpec::new("HappyGhast", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_GIANT_ID => const { &MobSpec::new("Giant", GROUND) },
        VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => {
            const {
                &MobSpec {
                    goals: SQUID_GOALS,
                    ..MobSpec::new("GlowSquid", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_GOAT_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::FLOAT,
                    goals: &[
                        (0, Float),
                        (1, panic(2.0).panic_kind(PanicKind::Animal)),
                        (2, GoatRam),
                        (3, Breed { speed: 1.0 }),
                        (4, tempt(1.25, Tag("minecraft:goat_food"))),
                        (5, GoatLongJump),
                        (
                            6,
                            FollowParent {
                                speed: 1.25,
                                mode: FollowMode::Goat,
                            },
                        ),
                        (7, idle_look(true, 6.0, 1.0)),
                        (
                            8,
                            Stroll {
                                speed: 1.0,
                                position: super::StrollPosition::Land,
                                interval: 20,
                            },
                        ),
                    ],
                    ..MobSpec::new("Goat", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_GUARDIAN_ID => {
            const {
                &MobSpec {
                    move_control: MoveControlKind::Guardian,
                    max_head_x_rot: Some(180.0),
                    goals: GUARDIAN_GOALS,
                    targets: GUARDIAN_TARGETS,
                    ..MobSpec::new("Guardian", WATER_NO_BREACH)
                }
            }
        }
        VANILLA_ENTITY_TYPE_HOGLIN_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (2, Breed { speed: 0.6 }),
                        (3, melee(1.0, true)),
                        (
                            6,
                            FollowParent {
                                speed: 0.6,
                                mode: FollowMode::Hoglin,
                            },
                        ),
                        (7, HoglinIdle),
                    ],
                    targets: &[(1, nearest("Player", true))],
                    ..MobSpec::new("Hoglin", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_HORSE_ID => {
            const {
                &MobSpec {
                    goals: HORSE_GOALS,
                    ..MobSpec::new("Horse", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_HUSK_ID => {
            const {
                &MobSpec {
                    goals: ZOMBIE_GOALS,
                    targets: ZOMBIE_TARGETS,
                    ..MobSpec::new("Husk", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, RaiderBanner),
                        (3, RaiderTravel),
                        (4, RaiderVillage),
                        (5, RaiderCelebration),
                        (4, LongDistancePatrol),
                        (0, Float),
                        (1, SpellcasterCasting),
                        (3, avoid("Creaking", 8.0, 1.0, 1.2)),
                        (4, Spell(Spell::Disappear)),
                        (5, Spell(Spell::Blindness)),
                        (
                            6,
                            BowAttack(BowSpec {
                                speed: 0.5,
                                hard_interval: 20,
                                interval: 20,
                                radius: 15.0,
                                melee_fallback: false,
                            }),
                        ),
                        (8, stroll(0.6)),
                        (9, look_at(Player, 3.0, 1.0)),
                        (10, look_at(AnyMob, 8.0, 0.02)),
                    ],
                    targets: &[
                        (
                            1,
                            hurt_by(HurtByVariant::Plain, &["Raider"]).alert_others(&[]),
                        ),
                        (2, nearest("Player", true).unseen_memory(300)),
                        (3, nearest("AbstractVillager", false).unseen_memory(300)),
                        (3, nearest("IronGolem", false).unseen_memory(300)),
                    ],
                    ..MobSpec::new("Illusioner", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, melee(1.0, true)),
                        (2, MoveTowardsTarget),
                        (
                            2,
                            Stroll {
                                speed: 0.6,
                                position: super::StrollPosition::VillageReturn,
                                interval: 10,
                            },
                        ),
                        (
                            4,
                            Stroll {
                                speed: 0.6,
                                position: super::StrollPosition::VillagePatrol,
                                interval: 240,
                            },
                        ),
                        (5, OfferFlower),
                        (7, look_at(Player, 6.0, 0.02)),
                        (8, RANDOM_LOOK),
                    ],
                    targets: &[
                        (1, DefendVillage),
                        (2, hurt_by(HurtByVariant::Plain, &[])),
                        (3, nearest("Player", true).selector(TargetSelector::AngryAt)),
                        (
                            3,
                            nearest("Mob", false)
                                .every(5)
                                .selector(TargetSelector::EnemyNotCreeper),
                        ),
                        (
                            4,
                            ResetUniversalAnger {
                                alert_others: false,
                            },
                        ),
                    ],
                    ..MobSpec::new("IronGolem", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_LLAMA_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::required(40.0),
                    goals: &[
                        (0, Float),
                        (1, RunAroundLikeCrazy { speed: 1.2 }),
                        (2, LlamaCaravan),
                        (3, ranged(1.25, 40, 40, 20.0)),
                        (3, panic(1.2)),
                        (4, Breed { speed: 1.0 }),
                        (5, tempt(1.25, Tag("minecraft:llama_tempt_items"))),
                        (6, follow_parent(1.0)),
                        (7, water_avoiding_stroll(0.7, 0.001)),
                        (8, look_at(Player, 6.0, 0.02)),
                        (9, RANDOM_LOOK),
                    ],
                    targets: &[
                        (1, hurt_by(HurtByVariant::Llama, &[])),
                        (
                            2,
                            nearest("Wolf", false)
                                .every(16)
                                .selector(TargetSelector::UntamedWolf)
                                .variant(NearestVariant::LlamaAttackWolf),
                        ),
                    ],
                    ..MobSpec::new("Llama", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => {
            const {
                &MobSpec {
                    max_head_x_rot: Some(0.0),
                    goals: SLIME_GOALS,
                    targets: SLIME_TARGETS,
                    ..MobSpec::new("MagmaCube", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID => {
            const {
                &MobSpec {
                    goals: COW_GOALS,
                    ..MobSpec::new("MushroomCow", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_MULE_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, RunAroundLikeCrazy { speed: 1.2 }),
                        (4, follow_parent(1.0)),
                        (6, water_avoiding_stroll(0.7, 0.001)),
                        (7, look_at(Player, 6.0, 0.02)),
                        (8, RANDOM_LOOK),
                        (9, RandomStand),
                        (0, Float),
                        (1, panic(1.2).panic_kind(PanicKind::Mount)),
                        (3, tempt(1.25, Tag("minecraft:horse_tempt_items"))),
                    ],
                    ..MobSpec::new("Mule", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_NAUTILUS_ID => {
            const {
                &MobSpec {
                    move_control: MoveControlKind::NautilusSwimming,
                    look_control: None,
                    goals: &[
                        (0, panic(1.6)),
                        (1, Breed { speed: 0.4 }),
                        (2, Nautilus),
                        (
                            4,
                            Stroll {
                                speed: 1.0,
                                position: super::StrollPosition::Swimming,
                                interval: super::super::stroll::DEFAULT_INTERVAL,
                            },
                        ),
                    ],
                    ..MobSpec::new(
                        "Nautilus",
                        Some(NavigationFactory::Water {
                            allow_breaching: false,
                        }),
                    )
                }
            }
        }
        VANILLA_ENTITY_TYPE_OCELOT_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (7, LeapAtTarget { yd: 0.3 }),
                        (8, OcelotAttack),
                        (1, Float),
                        (
                            3,
                            tempt(0.6, Tag("minecraft:ocelot_food")).scare(TemptScare::Ocelot),
                        ),
                        (
                            4,
                            avoid("Player", 16.0, 0.8, 1.33).gate(AvoidGate::Untrusting),
                        ),
                        (9, Breed { speed: 0.8 }),
                        (10, water_avoiding_stroll(0.8, 1.0000001e-5)),
                        (11, look_at(Player, 10.0, 0.02)),
                    ],
                    targets: &[
                        (1, nearest("Chicken", false)),
                        (
                            1,
                            nearest("Turtle", false).selector(TargetSelector::TurtleBabyOnLand),
                        ),
                    ],
                    ..MobSpec::new("Ocelot", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_PANDA_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (0, Float),
                        (2, panic(2.0).panic_kind(PanicKind::Panda)),
                        (2, Breed { speed: 1.0 }),
                        (
                            3,
                            MeleeAttack {
                                speed: 1.2,
                                follow_unseen: true,
                                kind: MeleeKind::Panda,
                            },
                        ),
                        (4, tempt(1.0, Tag("minecraft:panda_food"))),
                        (
                            6,
                            avoid("Player", 8.0, 2.0, 2.0).gate(AvoidGate::PandaWorried),
                        ),
                        (
                            6,
                            avoid("Monster", 4.0, 2.0, 2.0).gate(AvoidGate::PandaWorried),
                        ),
                        (7, PandaSit),
                        (8, PandaLieOnBack),
                        (8, PandaSneeze),
                        (9, look_at(Player, 6.0, 0.02)),
                        (10, RANDOM_LOOK),
                        (12, PandaRoll),
                        (13, follow_parent(1.25)),
                        (14, water_avoiding_stroll(1.0, 0.001)),
                    ],
                    targets: &[(1, hurt_by(HurtByVariant::Panda, &[]).alert_others(&[]))],
                    ..MobSpec::new("Panda", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_PARCHED_ID => {
            const {
                &MobSpec {
                    goals: BOGGED_GOALS,
                    targets: SKELETON_TARGETS,
                    ..MobSpec::new("Parched", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_PARROT_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (0, panic(1.25).panic_kind(PanicKind::TamablePlain)),
                        (0, Float),
                        (1, look_at(Player, 8.0, 0.02)),
                        (2, SitWhenOrderedTo),
                        (
                            2,
                            FollowOwner {
                                speed: 1.0,
                                start_distance: 5.0,
                                stop_distance: 1.0,
                            },
                        ),
                        (2, parrot_wander(1.0)),
                        (3, LandOnOwnersShoulder),
                        (
                            3,
                            FollowMob {
                                speed: 1.0,
                                stop_distance: 3.0,
                                area_size: 7.0,
                            },
                        ),
                    ],
                    ..MobSpec::new("Parrot", Some(NavigationFactory::Flying(FLYING_NAVIGATION)))
                }
            }
        }
        VANILLA_ENTITY_TYPE_PHANTOM_ID => {
            const {
                &MobSpec {
                    look_control: None,
                    ..MobSpec::new("Phantom", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_PIG_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (0, Float),
                        (1, panic(1.25)),
                        (3, Breed { speed: 1.0 }),
                        (4, tempt(1.2, Item(VANILLA_ITEM_CARROT_ON_A_STICK_ID))),
                        (4, tempt(1.2, Tag("minecraft:pig_food"))),
                        (5, follow_parent(1.1)),
                        (6, water_avoiding_stroll(1.0, 0.001)),
                        (7, look_at(Player, 6.0, 0.02)),
                        (8, RANDOM_LOOK),
                    ],
                    ..MobSpec::new("Pig", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_PIGLIN_ID => {
            const {
                &MobSpec {
                    goals: &[
                        // PiglinAi: a golden spear holder approaches, stabs and retreats.
                        (1, SpearUse),
                        (
                            1,
                            CrossbowAttack {
                                speed: 1.0,
                                radius: 8.0,
                            },
                        ),
                        (2, melee(1.0, true)),
                        (7, PiglinIdle),
                    ],
                    ..MobSpec::new("Piglin", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => {
            const {
                &MobSpec {
                    goals: &[(1, melee(1.0, true)), (7, PiglinIdle)],
                    ..MobSpec::new("PiglinBrute", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_PILLAGER_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, RaiderBanner),
                        (3, RaiderTravel),
                        (4, RaiderVillage),
                        (5, RaiderCelebration),
                        (4, LongDistancePatrol),
                        (0, Float),
                        (1, avoid("Creaking", 8.0, 1.0, 1.2)),
                        (2, RaiderHoldGround),
                        (
                            3,
                            CrossbowAttack {
                                speed: 1.0,
                                radius: 8.0,
                            },
                        ),
                        (8, stroll(0.6)),
                        (9, look_at(Player, 15.0, 1.0)),
                        (10, look_at(AnyMob, 15.0, 0.02)),
                    ],
                    targets: &[
                        (
                            1,
                            hurt_by(HurtByVariant::Plain, &["Raider"]).alert_others(&[]),
                        ),
                        (2, nearest("Player", true)),
                        (3, nearest("AbstractVillager", false)),
                        (3, nearest("IronGolem", true)),
                    ],
                    ..MobSpec::new("Pillager", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (0, Float),
                        (
                            1,
                            MeleeAttack {
                                speed: 1.25,
                                follow_unseen: true,
                                kind: MeleeKind::PolarBear,
                            },
                        ),
                        (1, panic(2.0).panic_kind(PanicKind::PolarBear)),
                        (4, follow_parent(1.25)),
                        (5, stroll(1.0)),
                        (6, look_at(Player, 6.0, 0.02)),
                        (7, RANDOM_LOOK),
                    ],
                    targets: &[
                        (1, hurt_by(HurtByVariant::PolarBear, &[])),
                        (
                            2,
                            nearest("Player", true)
                                .every(20)
                                .variant(NearestVariant::PolarBearAttackPlayers),
                        ),
                        (3, nearest("Player", true).selector(TargetSelector::AngryAt)),
                        (
                            4,
                            nearest("Fox", true).selector(TargetSelector::BodyNotBaby),
                        ),
                        (
                            5,
                            ResetUniversalAnger {
                                alert_others: false,
                            },
                        ),
                    ],
                    ..MobSpec::new("PolarBear", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID => const { &fish("Pufferfish") },
        VANILLA_ENTITY_TYPE_RABBIT_ID => {
            const {
                &MobSpec {
                    jump_control: false,
                    goals: &[
                        (1, Float),
                        (1, ClimbPowderSnow),
                        (1, panic(2.2)),
                        (2, Breed { speed: 0.8 }),
                        (3, tempt(1.0, Tag("minecraft:rabbit_food"))),
                        (
                            4,
                            avoid("Player", 8.0, 2.2, 2.2).gate(AvoidGate::NotKillerBunny),
                        ),
                        (
                            4,
                            avoid("Wolf", 10.0, 2.2, 2.2).gate(AvoidGate::NotKillerBunny),
                        ),
                        (
                            4,
                            avoid("Monster", 4.0, 2.2, 2.2).gate(AvoidGate::NotKillerBunny),
                        ),
                        (5, MoveToBlock(MoveToBlockKind::RabbitRaidGarden)),
                        (6, water_avoiding_stroll(0.6, 0.001)),
                        (11, look_at(Player, 10.0, 0.02)),
                    ],
                    ..MobSpec::new("Rabbit", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_RAVAGER_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, RaiderBanner),
                        (3, RaiderTravel),
                        (4, RaiderVillage),
                        (5, RaiderCelebration),
                        (4, melee(1.0, true)),
                        (4, LongDistancePatrol),
                        (0, Float),
                        (5, water_avoiding_stroll(0.4, 0.001)),
                        (6, look_at(Player, 6.0, 0.02)),
                        (10, look_at(AnyMob, 8.0, 0.02)),
                    ],
                    targets: &[
                        (
                            2,
                            hurt_by(HurtByVariant::Plain, &["Raider"]).alert_others(&[]),
                        ),
                        (3, nearest("Player", true)),
                        (
                            4,
                            nearest("AbstractVillager", true)
                                .selector(TargetSelector::TargetNotBaby),
                        ),
                        (4, nearest("IronGolem", true)),
                    ],
                    ..MobSpec::new("Ravager", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_SALMON_ID => const { &fish("Salmon") },
        VANILLA_ENTITY_TYPE_SHEEP_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (0, Float),
                        (1, panic(1.25)),
                        (2, Breed { speed: 1.0 }),
                        (3, tempt(1.1, Tag("minecraft:sheep_food"))),
                        (4, follow_parent(1.1)),
                        (5, EatBlock),
                        (6, water_avoiding_stroll(1.0, 0.001)),
                        (7, look_at(Player, 6.0, 0.02)),
                        (8, RANDOM_LOOK),
                    ],
                    ..MobSpec::new("Sheep", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_SHULKER_ID => {
            const {
                &MobSpec {
                    look_control: Some(LookControlKind::ShulkerDown),
                    max_head_x_rot: Some(180.0),
                    goals: &[
                        (1, look_at_horizontally(Player, 8.0, 0.02)),
                        (4, ShulkerAttack),
                        (7, ShulkerPeek),
                        (8, RANDOM_LOOK),
                    ],
                    targets: &[
                        (
                            1,
                            hurt_by(HurtByVariant::Plain, &["Shulker"]).alert_others(&[]),
                        ),
                        (
                            2,
                            nearest("Player", true).variant(NearestVariant::ShulkerNearest),
                        ),
                        (
                            3,
                            nearest("LivingEntity", true)
                                .selector(TargetSelector::Enemy)
                                .variant(NearestVariant::ShulkerDefense),
                        ),
                    ],
                    ..MobSpec::new("Shulker", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_SILVERFISH_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, Float),
                        (1, ClimbPowderSnow),
                        (3, SilverfishWakeFriends),
                        (4, melee(1.0, false)),
                        (5, SilverfishMerge),
                    ],
                    targets: HOSTILE_ALERT_TARGETS,
                    ..MobSpec::new("Silverfish", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_SKELETON_ID => {
            const {
                &MobSpec {
                    goals: SKELETON_GOALS,
                    targets: SKELETON_TARGETS,
                    ..MobSpec::new("Skeleton", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, SkeletonHorseTrap),
                        (1, RunAroundLikeCrazy { speed: 1.2 }),
                        (4, follow_parent(1.0)),
                        (6, water_avoiding_stroll(0.7, 0.001)),
                        (7, look_at(Player, 6.0, 0.02)),
                        (8, RANDOM_LOOK),
                        (9, RandomStand),
                    ],
                    ..MobSpec::new("SkeletonHorse", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_SLIME_ID => {
            const {
                &MobSpec {
                    max_head_x_rot: Some(0.0),
                    goals: SLIME_GOALS,
                    targets: SLIME_TARGETS,
                    ..MobSpec::new("Slime", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_SNIFFER_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::SNIFFER,
                    goals: &[
                        (0, Float),
                        (1, panic(2.0).panic_kind(PanicKind::Animal)),
                        (2, Breed { speed: 1.0 }),
                        (3, tempt(1.25, Tag("minecraft:sniffer_food"))),
                        (6, water_avoiding_stroll(1.0, 1.0000001e-5)),
                        (7, look_at(Player, 6.0, 0.02)),
                        (8, RANDOM_LOOK),
                    ],
                    ..MobSpec::new("Sniffer", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, ranged(1.25, 20, 20, 10.0)),
                        (2, water_avoiding_stroll(1.0, 1.0000001e-5)),
                        (3, look_at(Player, 6.0, 0.02)),
                        (4, RANDOM_LOOK),
                    ],
                    targets: &[(1, nearest("Mob", true).selector(TargetSelector::Enemy))],
                    ..MobSpec::new("SnowGolem", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_SPIDER_ID => {
            const {
                &MobSpec {
                    goals: SPIDER_GOALS,
                    targets: SPIDER_TARGETS,
                    ..MobSpec::new("Spider", Some(NavigationFactory::WallClimber))
                }
            }
        }
        VANILLA_ENTITY_TYPE_SQUID_ID => {
            const {
                &MobSpec {
                    goals: SQUID_GOALS,
                    ..MobSpec::new("Squid", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_STRAY_ID => {
            const {
                &MobSpec {
                    goals: SKELETON_GOALS,
                    targets: SKELETON_TARGETS,
                    ..MobSpec::new("Stray", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_STRIDER_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, panic(1.65)),
                        (2, Breed { speed: 1.0 }),
                        (3, tempt(1.4, Tag("minecraft:strider_tempt_items"))),
                        (4, MoveToBlock(MoveToBlockKind::StriderGoToLava)),
                        (5, follow_parent(1.0)),
                        (
                            7,
                            Stroll {
                                speed: 1.0,
                                position: super::StrollPosition::Default,
                                interval: 60,
                            },
                        ),
                        (8, look_at(Player, 8.0, 0.02)),
                        (8, RANDOM_LOOK),
                        (9, look_at(Strider, 8.0, 0.02)),
                    ],
                    ..MobSpec::new("Strider", Some(NavigationFactory::Strider))
                }
            }
        }
        VANILLA_ENTITY_TYPE_TADPOLE_ID => {
            const {
                &MobSpec {
                    move_control: MoveControlKind::SmoothSwimming,
                    look_control: None,
                    ..MobSpec::new("Tadpole", WATER_NO_BREACH)
                }
            }
        }
        VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::required(40.0),
                    goals: &[
                        (0, Float),
                        (1, RunAroundLikeCrazy { speed: 1.2 }),
                        (2, LlamaCaravan),
                        (3, ranged(1.25, 40, 40, 20.0)),
                        (3, panic(1.2)),
                        (4, Breed { speed: 1.0 }),
                        (5, tempt(1.25, Tag("minecraft:llama_tempt_items"))),
                        (6, follow_parent(1.0)),
                        (7, water_avoiding_stroll(0.7, 0.001)),
                        (8, look_at(Player, 6.0, 0.02)),
                        (9, RANDOM_LOOK),
                        (1, panic(2.0)),
                    ],
                    targets: &[
                        (1, hurt_by(HurtByVariant::Llama, &[])),
                        (
                            2,
                            nearest("Wolf", false)
                                .every(16)
                                .selector(TargetSelector::UntamedWolf)
                                .variant(NearestVariant::LlamaAttackWolf),
                        ),
                        (1, TraderLlamaDefend),
                        (
                            2,
                            nearest("Zombie", true).selector(TargetSelector::NotZombifiedPiglin),
                        ),
                        (2, nearest("AbstractIllager", true)),
                    ],
                    ..MobSpec::new("TraderLlama", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => const { &fish("TropicalFish") },
        VANILLA_ENTITY_TYPE_TURTLE_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (0, panic(1.2).panic_kind(PanicKind::Turtle)),
                        (1, Breed { speed: 1.0 }),
                        (2, tempt(1.1, Tag("minecraft:turtle_food"))),
                        (3, MoveToBlock(MoveToBlockKind::TurtleGoToWater)),
                        (7, TurtleTravel),
                        (8, look_at(Player, 8.0, 0.02)),
                        (
                            9,
                            Stroll {
                                speed: 1.0,
                                position: super::StrollPosition::Turtle,
                                interval: 100,
                            },
                        ),
                    ],
                    ..MobSpec::new("Turtle", Some(NavigationFactory::Amphibious))
                }
            }
        }
        VANILLA_ENTITY_TYPE_VEX_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (0, Float),
                        (4, VexChargeAttack),
                        (8, VexRandomMove),
                        (9, look_at(Player, 3.0, 1.0)),
                        (10, look_at(AnyMob, 8.0, 0.02)),
                    ],
                    targets: &[
                        (
                            1,
                            hurt_by(HurtByVariant::Plain, &["Raider"]).alert_others(&[]),
                        ),
                        (2, VexCopyOwner),
                        (3, nearest("Player", true)),
                    ],
                    ..MobSpec::new("Vex", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_VILLAGER_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::VILLAGER,
                    ..MobSpec::new("Villager", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_VINDICATOR_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, RaiderBanner),
                        (3, RaiderTravel),
                        (4, RaiderVillage),
                        (5, RaiderCelebration),
                        (5, melee(1.0, false)),
                        (4, LongDistancePatrol),
                        (0, Float),
                        (1, avoid("Creaking", 8.0, 1.0, 1.2)),
                        (4, RaiderHoldGround),
                        (8, stroll(0.6)),
                        (9, look_at(Player, 3.0, 1.0)),
                        (10, look_at(AnyMob, 8.0, 0.02)),
                    ],
                    targets: &[
                        (
                            1,
                            hurt_by(HurtByVariant::Plain, &["Raider"]).alert_others(&[]),
                        ),
                        (2, nearest("Player", true)),
                        (3, nearest("AbstractVillager", true)),
                        (3, nearest("IronGolem", true)),
                        (
                            4,
                            nearest("LivingEntity", true)
                                .every(0)
                                .selector(TargetSelector::Attackable)
                                .variant(NearestVariant::VindicatorJohnny),
                        ),
                    ],
                    ..MobSpec::new("Vindicator", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (0, Float),
                        (0, TraderUseItem(TraderDrinkKind::Invisibility)),
                        (0, TraderUseItem(TraderDrinkKind::Milk)),
                        (1, TradeWithPlayer),
                        (1, avoid("Zombie", 8.0, 0.5, 0.5)),
                        (1, avoid("Evoker", 12.0, 0.5, 0.5)),
                        (1, avoid("Vindicator", 8.0, 0.5, 0.5)),
                        (1, avoid("Vex", 8.0, 0.5, 0.5)),
                        (1, avoid("Pillager", 15.0, 0.5, 0.5)),
                        (1, avoid("Illusioner", 12.0, 0.5, 0.5)),
                        (1, avoid("Zoglin", 10.0, 0.5, 0.5)),
                        (1, panic(0.5)),
                        (1, LookAt(LookAtGoalSpec::Trading)),
                        (2, TraderTravel),
                        (4, move_towards_restriction(0.35)),
                        (8, water_avoiding_stroll(0.35, 0.001)),
                        (9, LookAt(LookAtGoalSpec::TraderInteract)),
                        (10, look_at(AnyMob, 8.0, 0.02)),
                    ],
                    ..MobSpec::new("WanderingTrader", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_WARDEN_ID => {
            const {
                &MobSpec {
                    ground_tuning: GroundTuning::FLOAT,
                    goals: &[
                        (0, Float),
                        (1, WardenLifecycle),
                        (2, WardenCombat),
                        (3, WardenAwareness),
                        (4, stroll(0.5)),
                    ],
                    ..MobSpec::new("Warden", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_WITCH_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, RaiderBanner),
                        (3, RaiderTravel),
                        (4, RaiderVillage),
                        (5, RaiderCelebration),
                        (4, LongDistancePatrol),
                        (1, Float),
                        (
                            2,
                            RangedAttack(RangedSpec {
                                kind: RangedKind::Witch,
                                speed: 1.0,
                                interval_min: 60,
                                interval_max: 60,
                                radius: 10.0,
                            }),
                        ),
                        (2, water_avoiding_stroll(1.0, 0.001)),
                        (3, look_at(Player, 8.0, 0.02)),
                        (3, RANDOM_LOOK),
                    ],
                    targets: &[
                        (1, hurt_by(HurtByVariant::Plain, &["Raider"])),
                        (
                            2,
                            nearest("Raider", true)
                                .every(500)
                                .selector(TargetSelector::HealableRaider)
                                .variant(NearestVariant::HealableRaider),
                        ),
                        (3, nearest("Player", true).variant(NearestVariant::Witch)),
                    ],
                    ..MobSpec::new("Witch", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_WITHER_ID => {
            const {
                &MobSpec {
                    // WitherDoNothingGoal is the skipped goal phase while invulnerable.
                    goals: &[
                        (2, ranged(1.0, 40, 40, 20.0)),
                        (5, stroll_flying(1.0)),
                        (6, look_at(Player, 8.0, 0.02)),
                        (7, RANDOM_LOOK),
                    ],
                    targets: &[
                        (1, hurt_by(HurtByVariant::Plain, &[])),
                        (
                            2,
                            nearest("LivingEntity", false)
                                .every(0)
                                .selector(TargetSelector::WitherLivingEntity),
                        ),
                    ],
                    ..MobSpec::new(
                        "WitherBoss",
                        Some(NavigationFactory::Flying(FLYING_NAVIGATION)),
                    )
                }
            }
        }
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID => {
            const {
                &MobSpec {
                    goals: SKELETON_GOALS,
                    targets: WITHER_SKELETON_TARGETS,
                    ..MobSpec::new("WitherSkeleton", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_WOLF_ID => {
            const {
                &MobSpec {
                    max_head_x_rot: None,
                    goals: &[
                        (4, LeapAtTarget { yd: 0.4 }),
                        (5, melee(1.0, true)),
                        (1, Float),
                        (2, SitWhenOrderedTo),
                        (
                            6,
                            FollowOwner {
                                speed: 1.0,
                                start_distance: 10.0,
                                stop_distance: 2.0,
                            },
                        ),
                        (1, panic(1.5).panic_kind(PanicKind::TamableEnvironmental)),
                        (
                            3,
                            avoid("Llama", 24.0, 1.5, 1.5).gate(AvoidGate::WolfFromLlama),
                        ),
                        (7, Breed { speed: 1.0 }),
                        (8, water_avoiding_stroll(1.0, 0.001)),
                        (9, Beg),
                        (10, look_at(Player, 8.0, 0.02)),
                        (10, RANDOM_LOOK),
                    ],
                    targets: &[
                        (1, OwnerTarget(OwnerMemory::HurtBy)),
                        (2, OwnerTarget(OwnerMemory::Hurt)),
                        (3, hurt_by(HurtByVariant::Plain, &[]).alert_others(&[])),
                        (4, nearest("Player", true).selector(TargetSelector::AngryAt)),
                        (
                            5,
                            nearest("Animal", false)
                                .selector(TargetSelector::WolfPrey)
                                .variant(NearestVariant::NonTameRandom),
                        ),
                        (
                            6,
                            nearest("Turtle", false)
                                .selector(TargetSelector::TurtleBabyOnLand)
                                .variant(NearestVariant::NonTameRandom),
                        ),
                        (7, nearest("AbstractSkeleton", false)),
                        (8, ResetUniversalAnger { alert_others: true }),
                    ],
                    ..MobSpec::new("Wolf", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_ZOGLIN_ID => {
            const {
                &MobSpec {
                    // Zoglin (the hoglin's idle stroll and look).
                    goals: &[(1, melee(1.0, true)), (7, HoglinIdle)],
                    targets: &[(
                        1,
                        nearest("LivingEntity", true).selector(TargetSelector::ZoglinAttackable),
                    )],
                    ..MobSpec::new("Zoglin", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_ID => {
            const {
                &MobSpec {
                    goals: ZOMBIE_GOALS,
                    targets: ZOMBIE_TARGETS,
                    ..MobSpec::new("Zombie", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (1, RunAroundLikeCrazy { speed: 1.2 }),
                        (4, follow_parent(1.0)),
                        (6, water_avoiding_stroll(0.7, 0.001)),
                        (7, look_at(Player, 6.0, 0.02)),
                        (8, RANDOM_LOOK),
                        (9, RandomStand),
                        (0, Float),
                        (3, tempt(1.25, Tag("minecraft:zombie_horse_food"))),
                    ],
                    ..MobSpec::new("ZombieHorse", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
            const {
                &MobSpec {
                    move_control: MoveControlKind::NautilusSwimming,
                    look_control: None,
                    goals: &[
                        (2, Nautilus),
                        (
                            4,
                            Stroll {
                                speed: 1.0,
                                position: super::StrollPosition::Swimming,
                                interval: super::super::stroll::DEFAULT_INTERVAL,
                            },
                        ),
                    ],
                    ..MobSpec::new(
                        "ZombieNautilus",
                        Some(NavigationFactory::Water {
                            allow_breaching: false,
                        }),
                    )
                }
            }
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => {
            const {
                &MobSpec {
                    goals: ZOMBIE_GOALS,
                    targets: ZOMBIE_TARGETS,
                    ..MobSpec::new("ZombieVillager", GROUND)
                }
            }
        }
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => {
            const {
                &MobSpec {
                    goals: &[
                        (4, MoveToBlock(MoveToBlockKind::ZombieAttackTurtleEgg)),
                        (1, SpearUse),
                        (
                            2,
                            MeleeAttack {
                                speed: 1.0,
                                follow_unseen: false,
                                kind: MeleeKind::Zombie,
                            },
                        ),
                        (8, look_at(Player, 8.0, 0.02)),
                        (8, RANDOM_LOOK),
                        (7, water_avoiding_stroll(1.0, 0.001)),
                    ],
                    targets: &[
                        (1, hurt_by(HurtByVariant::Plain, &[]).alert_others(&[])),
                        (2, nearest("Player", true).selector(TargetSelector::AngryAt)),
                        (3, ResetUniversalAnger { alert_others: true }),
                    ],
                    ..MobSpec::new("ZombifiedPiglin", GROUND)
                }
            }
        }

        _ => return None,
    })
}
