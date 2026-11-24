//! Source-derived entity-event dispatch and semantic consumer ownership.
//!
//! The generated schema owns Java declaration ids and `super` reachability. This module is the
//! single handwritten boundary that names what bbb consumes after that dispatch has resolved; the
//! reverse test makes every generated direct/delegated declaration id choose a semantic consumer.

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::EquipmentSlot;

#[cfg(test)]
use super::schema::VANILLA_ENTITY_EVENT_HANDLERS_26_1;
use super::schema::{
    vanilla_entity_metadata_handle_entity_event_owner_for_event, VanillaEntityOwner,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VanillaEntityEventKind {
    HoneySlide,
    LivingKineticHit,
    LivingDeath,
    LivingPortal,
    LivingItemBreak(EquipmentSlot),
    HoneyJump,
    LivingSwapHandItems,
    LivingPoof,
    LivingDrown,
    MobSpawn,
    Taming { succeeded: bool },
    AnimalLove,
    AllayDuplication,
    ArmadilloPeek,
    DolphinHappy,
    OcelotTrusting { succeeded: bool },
    FoxEat,
    GoatLowerHead,
    GoatRaiseHead,
    IronGolemAttack,
    IronGolemOfferFlower,
    IronGolemStopOfferFlower,
    RabbitJump,
    SheepEatGrass,
    SquidResetMovement,
    WolfStartShaking,
    WolfCancelShaking,
    ArmorStandHit,
    RavagerAttack,
    RavagerStun,
    RavagerRoar,
    WitchMagic,
    ZoglinAttack,
    CreakingInvulnerable,
    CreakingAttack,
    HoglinAttack,
    WardenAttack,
    WardenTendril,
    WardenSonicBoom,
    ZombieVillagerCure,
    VillagerHeart,
    VillagerAngry,
    VillagerHappy,
    VillagerSplash,
    PlayerFinishUsingItem,
    PlayerDisableReducedDebug,
    PlayerEnableReducedDebug,
    EvokerFangsAttack,
    FireworkExplosion,
    FishingHookPull,
    ArrowEffectClear,
    SnowballHit,
    ThrownEggHit,
    MinecartSpawnerDelayReset,
    MinecartTntPrime,
    GuardianAttackSound,
    TotemUse,
    SnifferDiggingSound,
}

pub(crate) fn vanilla_entity_event_kind(
    entity_type_id: i32,
    event_id: i8,
) -> Option<VanillaEntityEventKind> {
    if let Some(kind) = listener_special_event_kind(event_id) {
        return Some(kind);
    }
    let event_id = u8::try_from(event_id).ok()?;
    let owner =
        vanilla_entity_metadata_handle_entity_event_owner_for_event(entity_type_id, event_id)?;
    declaration_event_kind(owner, event_id)
}

pub(crate) fn entity_event_kind_with_registry(
    entity_type_id: i32,
    event_id: i8,
    content_registry: &ContentRegistry,
) -> Option<VanillaEntityEventKind> {
    if let Some(kind) = vanilla_entity_event_kind(entity_type_id, event_id) {
        return Some(kind);
    }
    let spec = super::content_facts::mod_entity_spec(entity_type_id, content_registry)?;
    let event_id = u8::try_from(event_id).ok()?;
    if spec.is_mob {
        if let Some(kind) = declaration_event_kind(VanillaEntityOwner::Mob, event_id) {
            return Some(kind);
        }
    }
    if spec.is_living {
        if let Some(kind) = declaration_event_kind(VanillaEntityOwner::LivingEntity, event_id) {
            return Some(kind);
        }
    }
    declaration_event_kind(VanillaEntityOwner::Entity, event_id)
}

fn listener_special_event_kind(event_id: i8) -> Option<VanillaEntityEventKind> {
    match event_id {
        21 => Some(VanillaEntityEventKind::GuardianAttackSound),
        35 => Some(VanillaEntityEventKind::TotemUse),
        63 => Some(VanillaEntityEventKind::SnifferDiggingSound),
        _ => None,
    }
}

fn declaration_event_kind(
    owner: VanillaEntityOwner,
    event_id: u8,
) -> Option<VanillaEntityEventKind> {
    use VanillaEntityEventKind::*;
    use VanillaEntityOwner as O;

    Some(match (owner, event_id) {
        (O::Entity, 53) => HoneySlide,
        (O::LivingEntity, 2) => LivingKineticHit,
        (O::LivingEntity, 3) => LivingDeath,
        (O::LivingEntity, 46) => LivingPortal,
        (O::LivingEntity, 47) => LivingItemBreak(EquipmentSlot::MainHand),
        (O::LivingEntity, 48) => LivingItemBreak(EquipmentSlot::OffHand),
        (O::LivingEntity, 49) => LivingItemBreak(EquipmentSlot::Head),
        (O::LivingEntity, 50) => LivingItemBreak(EquipmentSlot::Chest),
        (O::LivingEntity, 51) => LivingItemBreak(EquipmentSlot::Legs),
        (O::LivingEntity, 52) => LivingItemBreak(EquipmentSlot::Feet),
        (O::LivingEntity, 54) => HoneyJump,
        (O::LivingEntity, 55) => LivingSwapHandItems,
        (O::LivingEntity, 60) => LivingPoof,
        (O::LivingEntity, 65) => LivingItemBreak(EquipmentSlot::Body),
        (O::LivingEntity, 67) => LivingDrown,
        (O::LivingEntity, 68) => LivingItemBreak(EquipmentSlot::Saddle),
        (O::Mob, 20) => MobSpawn,
        (O::TamableAnimal, 7) | (O::AbstractHorse, 7) => Taming { succeeded: true },
        (O::TamableAnimal, 6) | (O::AbstractHorse, 6) => Taming { succeeded: false },
        (O::Animal, 18) => AnimalLove,
        (O::Allay, 18) => AllayDuplication,
        (O::Armadillo, 64) => ArmadilloPeek,
        (O::Dolphin, 38) => DolphinHappy,
        (O::Ocelot, 41) => OcelotTrusting { succeeded: true },
        (O::Ocelot, 40) => OcelotTrusting { succeeded: false },
        (O::Fox, 45) => FoxEat,
        (O::Goat, 58) => GoatLowerHead,
        (O::Goat, 59) => GoatRaiseHead,
        (O::IronGolem, 4) => IronGolemAttack,
        (O::IronGolem, 11) => IronGolemOfferFlower,
        (O::IronGolem, 34) => IronGolemStopOfferFlower,
        (O::Rabbit, 1) => RabbitJump,
        (O::Sheep, 10) => SheepEatGrass,
        (O::Squid, 19) => SquidResetMovement,
        (O::Wolf, 8) => WolfStartShaking,
        (O::Wolf, 56) => WolfCancelShaking,
        (O::ArmorStand, 32) => ArmorStandHit,
        (O::Ravager, 4) => RavagerAttack,
        (O::Ravager, 39) => RavagerStun,
        (O::Ravager, 69) => RavagerRoar,
        (O::Witch, 15) => WitchMagic,
        (O::Zoglin, 4) => ZoglinAttack,
        (O::Creaking, 66) => CreakingInvulnerable,
        (O::Creaking, 4) => CreakingAttack,
        (O::Hoglin, 4) => HoglinAttack,
        (O::Warden, 4) => WardenAttack,
        (O::Warden, 61) => WardenTendril,
        (O::Warden, 62) => WardenSonicBoom,
        (O::ZombieVillager, 16) => ZombieVillagerCure,
        (O::Villager, 12) => VillagerHeart,
        (O::Villager, 13) => VillagerAngry,
        (O::Villager, 14) => VillagerHappy,
        (O::Villager, 42) => VillagerSplash,
        (O::Player, 9) => PlayerFinishUsingItem,
        (O::Player, 23) => PlayerDisableReducedDebug,
        (O::Player, 22) => PlayerEnableReducedDebug,
        (O::EvokerFangs, 4) => EvokerFangsAttack,
        (O::FireworkRocketEntity, 17) => FireworkExplosion,
        (O::FishingHook, 31) => FishingHookPull,
        (O::Arrow, 0) => ArrowEffectClear,
        (O::Snowball, 3) => SnowballHit,
        (O::ThrownEgg, 3) => ThrownEggHit,
        (O::MinecartSpawner, 1) => MinecartSpawnerDelayReset,
        (O::MinecartTNT, 10) => MinecartTntPrime,
        _ => return None,
    })
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EventConsumerDisposition {
    canonical: bool,
    animation: bool,
    particle: bool,
    sound: bool,
}

#[cfg(test)]
fn event_consumer_disposition(kind: VanillaEntityEventKind) -> EventConsumerDisposition {
    use VanillaEntityEventKind::*;

    const NONE: EventConsumerDisposition = EventConsumerDisposition {
        canonical: false,
        animation: false,
        particle: false,
        sound: false,
    };
    const C: EventConsumerDisposition = EventConsumerDisposition {
        canonical: true,
        ..NONE
    };
    const P: EventConsumerDisposition = EventConsumerDisposition {
        canonical: false,
        animation: false,
        particle: true,
        sound: false,
    };
    const S: EventConsumerDisposition = EventConsumerDisposition {
        canonical: false,
        animation: false,
        particle: false,
        sound: true,
    };
    const AS: EventConsumerDisposition = EventConsumerDisposition {
        animation: true,
        sound: true,
        ..NONE
    };
    const APS: EventConsumerDisposition = EventConsumerDisposition {
        animation: true,
        particle: true,
        sound: true,
        ..NONE
    };
    const AP: EventConsumerDisposition = EventConsumerDisposition {
        animation: true,
        particle: true,
        ..NONE
    };
    const PS: EventConsumerDisposition = EventConsumerDisposition {
        particle: true,
        sound: true,
        ..NONE
    };
    const CS: EventConsumerDisposition = EventConsumerDisposition { sound: true, ..C };
    const CP: EventConsumerDisposition = EventConsumerDisposition {
        particle: true,
        ..C
    };
    const CPS: EventConsumerDisposition = EventConsumerDisposition {
        particle: true,
        sound: true,
        ..C
    };

    match kind {
        HoneySlide
        | LivingPortal
        | HoneyJump
        | LivingPoof
        | LivingDrown
        | MobSpawn
        | Taming { .. }
        | AnimalLove
        | AllayDuplication
        | DolphinHappy
        | OcelotTrusting { .. }
        | FoxEat
        | WitchMagic
        | VillagerHeart
        | VillagerAngry
        | VillagerHappy
        | VillagerSplash
        | SnowballHit
        | ThrownEggHit => P,
        LivingItemBreak(_) | FireworkExplosion => PS,
        LivingDeath | ZombieVillagerCure => CS,
        LivingKineticHit
        | GoatLowerHead
        | GoatRaiseHead
        | IronGolemOfferFlower
        | IronGolemStopOfferFlower
        | SheepEatGrass
        | WolfStartShaking
        | WolfCancelShaking
        | RavagerStun
        | WardenAttack
        | WardenTendril
        | WardenSonicBoom => APS,
        ArmadilloPeek | IronGolemAttack | ArmorStandHit | RavagerAttack | ZoglinAttack
        | CreakingInvulnerable | CreakingAttack | HoglinAttack | EvokerFangsAttack => AS,
        RabbitJump | RavagerRoar => AP,
        ArrowEffectClear => CP,
        PlayerFinishUsingItem | TotemUse => CPS,
        GuardianAttackSound | SnifferDiggingSound => S,
        LivingSwapHandItems
        | SquidResetMovement
        | PlayerDisableReducedDebug
        | PlayerEnableReducedDebug
        | FishingHookPull
        | MinecartSpawnerDelayReset
        | MinecartTntPrime => C,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID;

    use super::super::schema::{
        VanillaEntityBlockEffectTickPolicy, VanillaEntityClientMovementPolicy,
        VANILLA_ENTITY_TYPE_METADATA_26_1,
    };
    use super::*;

    #[test]
    fn every_generated_entity_event_declaration_has_one_semantic_consumer() {
        let mut declarations = HashSet::new();
        for handler in VANILLA_ENTITY_EVENT_HANDLERS_26_1 {
            for event_id in handler
                .direct_event_ids
                .iter()
                .chain(handler.delegated_event_ids)
            {
                assert!(declarations.insert((handler.owner, *event_id)));
                let kind = declaration_event_kind(handler.owner, *event_id).unwrap_or_else(|| {
                    panic!(
                        "missing bbb consumer for {:?} event {}",
                        handler.owner, event_id
                    )
                });
                let disposition = event_consumer_disposition(kind);
                assert!(
                    disposition.canonical
                        || disposition.animation
                        || disposition.particle
                        || disposition.sound,
                    "semantic consumer {kind:?} has no disposition"
                );
            }
        }
    }

    #[test]
    fn listener_specials_are_owned_before_class_dispatch() {
        assert_eq!(
            listener_special_event_kind(21),
            Some(VanillaEntityEventKind::GuardianAttackSound)
        );
        assert_eq!(
            listener_special_event_kind(35),
            Some(VanillaEntityEventKind::TotemUse)
        );
        assert_eq!(
            listener_special_event_kind(63),
            Some(VanillaEntityEventKind::SnifferDiggingSound)
        );
        assert_eq!(listener_special_event_kind(3), None);
        for event_id in [21, 35, 63] {
            let kind = listener_special_event_kind(event_id).expect("listener special kind");
            let disposition = event_consumer_disposition(kind);
            assert!(
                disposition.canonical
                    || disposition.animation
                    || disposition.particle
                    || disposition.sound
            );
        }
    }

    #[test]
    fn firework_event_and_tick_ownership_stays_in_the_fixed_type_reverse_closure() {
        let event_kind = vanilla_entity_event_kind(VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID, 17)
            .expect("firework explosion event kind");
        assert_eq!(event_kind, VanillaEntityEventKind::FireworkExplosion);
        assert_eq!(
            event_consumer_disposition(event_kind),
            EventConsumerDisposition {
                canonical: false,
                animation: false,
                particle: true,
                sound: true,
            }
        );

        let entry = &VANILLA_ENTITY_TYPE_METADATA_26_1
            [usize::try_from(VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID).unwrap()];
        assert_eq!(entry.resource_id, "minecraft:firework_rocket");
        assert_eq!(
            entry.client_movement_policy,
            VanillaEntityClientMovementPolicy::FireworkRocket
        );
        assert_eq!(
            entry.block_effect_tick_policy,
            VanillaEntityBlockEffectTickPolicy::DetachedFirework
        );
    }
}
