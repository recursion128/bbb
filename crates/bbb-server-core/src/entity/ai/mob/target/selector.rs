//! The `TargetingConditions.Selector` lambdas the target registrations pass.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_FOX_ID,
    VANILLA_ENTITY_TYPE_RABBIT_ID, VANILLA_ENTITY_TYPE_SHEEP_ID, VANILLA_ENTITY_TYPE_WITCH_ID,
    VANILLA_ENTITY_TYPE_ZOGLIN_ID, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};

use super::anger::NeutralAnger;
use super::{Target, TargetView};
use crate::entity::ai::mob::spec;

/// `Enemy`'s implementors.
pub(crate) const ENEMY_IMPLEMENTORS: [&str; 7] = [
    "Monster",
    "Ghast",
    "Slime",
    "Hoglin",
    "Shulker",
    "Phantom",
    "EnderDragon",
];

/// One selector a target registration hands its conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetSelector {
    /// A registration with no selector.
    None,
    NotZombifiedPiglin,
    TurtleBabyOnLand,
    EnemyNotCreeper,
    Enemy,
    BodyNotBaby,
    WolfPrey,
    WitherLivingEntity,
    WithinFourBlocksHeight,
    GuardianAttack,
    TargetNotBaby,
    HealableRaider,
    DrownedOkTarget,
    AngryAt,
    ChickenOrRabbit,
    SchoolingFish,
    FoxUntrustedAttacker,
    Attackable,
    ZoglinAttackable,
    UntamedWolf,
    AxolotlPrey,
}

impl TargetSelector {
    pub(crate) fn test<'w>(
        self,
        world: &TargetView<'_, '_, 'w>,
        anger: Option<&NeutralAnger>,
        target: &Target<'_, '_, 'w>,
    ) -> bool {
        let body = &world.body();
        match self {
            Self::None => true,
            Self::NotZombifiedPiglin => {
                target.entity_type_id() != VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
            }
            Self::TurtleBabyOnLand => target.baby() && !target.in_water(),
            Self::EnemyNotCreeper => {
                is_enemy(target.entity_type_id())
                    && target.entity_type_id() != VANILLA_ENTITY_TYPE_CREEPER_ID
            }
            Self::Enemy => is_enemy(target.entity_type_id()),
            Self::BodyNotBaby => !body.baby(),
            Self::WolfPrey => matches!(
                target.entity_type_id(),
                VANILLA_ENTITY_TYPE_SHEEP_ID
                    | VANILLA_ENTITY_TYPE_RABBIT_ID
                    | VANILLA_ENTITY_TYPE_FOX_ID
            ),
            Self::WitherLivingEntity => {
                !target.wither_friend()
                    && target.entity_type_id() != VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
            }
            Self::WithinFourBlocksHeight => (target.position().y - body.position().y).abs() <= 4.0,
            Self::GuardianAttack => {
                (target.is_player()
                    || spec::classes(target.entity_type_id())
                        .any(|class| matches!(class, "Squid" | "Axolotl")))
                    && target.distance_to_facts_sqr(body) > 9.0
            }
            Self::TargetNotBaby => !target.baby(),
            Self::HealableRaider => {
                world.has_active_raid() && target.entity_type_id() != VANILLA_ENTITY_TYPE_WITCH_ID
            }
            Self::DrownedOkTarget => world.bright_outside() == Some(false) || target.in_water(),
            Self::AngryAt => anger.is_some_and(|anger| anger.is_angry_at(world, target)),
            Self::ChickenOrRabbit => spec::classes(target.entity_type_id())
                .any(|class| matches!(class, "Chicken" | "Rabbit")),
            Self::SchoolingFish => is_instance_of(target, "AbstractSchoolingFish"),
            Self::FoxUntrustedAttacker => {
                target.last_hurt_mob().is_some()
                    && target.last_hurt_mob_timestamp() < target.tick_count().wrapping_add(600)
                    && !world.trusts(target.uuid())
            }
            Self::Attackable => target.entity_type_id() != VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            Self::ZoglinAttackable => !matches!(
                target.entity_type_id(),
                VANILLA_ENTITY_TYPE_CREEPER_ID | VANILLA_ENTITY_TYPE_ZOGLIN_ID
            ),
            Self::UntamedWolf => !target.tame(),
            Self::AxolotlPrey => {
                let resource_id =
                    bbb_protocol::entity_types::vanilla_entity_type(target.entity_type_id())
                        .map_or("", |kind| kind.resource_id);
                target.in_water()
                    && target.distance_to_facts_sqr(body) <= 64.0
                    && (world.level().registry_tag_membership(
                        "minecraft:entity_type",
                        "minecraft:axolotl_always_hostiles",
                        resource_id,
                    ) || (!crate::entity::axolotl::hunting_on_cooldown(world.body_entity())
                        && world.level().registry_tag_membership(
                            "minecraft:entity_type",
                            "minecraft:axolotl_hunt_targets",
                            resource_id,
                        )))
            }
        }
    }
}

/// `instanceof Enemy` over [`ENEMY_IMPLEMENTORS`].
pub(crate) fn is_enemy(entity_type_id: i32) -> bool {
    let mut classes = spec::classes(entity_type_id);
    classes.clone().any(|class| class == ENEMY_IMPLEMENTORS[0])
        || classes.any(|class| ENEMY_IMPLEMENTORS[1..].contains(&class))
}

/// `Class.isInstance` for the simple class name a registration passes, as in
/// `Player.class` or `AbstractVillager.class`.
pub(crate) fn is_instance_of(target: &Target<'_, '_, '_>, simple_class: &str) -> bool {
    match simple_class {
        "LivingEntity" => true,
        "Player" | "ServerPlayer" => target.is_player(),
        "ArmorStand" => target.entity_type_id() == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        _ => spec::is_instance_of(target.entity_type_id(), simple_class),
    }
}

/// `other.getClass() == ignoreClass` for a simple class name.
pub(crate) fn is_exactly(entity_type_id: i32, simple_class: &str) -> bool {
    spec::runtime_class(entity_type_id) == Some(simple_class)
}
