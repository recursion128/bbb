//! Source-derived entity-event dispatch and semantic consumer ownership.
//!
//! The generated schema owns Java declaration ids and `super` reachability. This module is the
//! single handwritten boundary that names what bbb consumes after that dispatch has resolved; the
//! reverse test makes every generated direct/delegated declaration id choose a semantic consumer.

use bbb_protocol::packets::EquipmentSlot;

use super::schema::vanilla_entity_metadata_handle_entity_event_owner_for_event;
#[cfg(test)]
use super::schema::VANILLA_ENTITY_EVENT_HANDLERS_26_1;

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

fn listener_special_event_kind(event_id: i8) -> Option<VanillaEntityEventKind> {
    match event_id {
        21 => Some(VanillaEntityEventKind::GuardianAttackSound),
        35 => Some(VanillaEntityEventKind::TotemUse),
        63 => Some(VanillaEntityEventKind::SnifferDiggingSound),
        _ => None,
    }
}

fn declaration_event_kind(owner: &str, event_id: u8) -> Option<VanillaEntityEventKind> {
    use VanillaEntityEventKind::*;

    Some(match (owner, event_id) {
        ("net.minecraft.world.entity.Entity", 53) => HoneySlide,
        ("net.minecraft.world.entity.LivingEntity", 2) => LivingKineticHit,
        ("net.minecraft.world.entity.LivingEntity", 3) => LivingDeath,
        ("net.minecraft.world.entity.LivingEntity", 46) => LivingPortal,
        ("net.minecraft.world.entity.LivingEntity", 47) => LivingItemBreak(EquipmentSlot::MainHand),
        ("net.minecraft.world.entity.LivingEntity", 48) => LivingItemBreak(EquipmentSlot::OffHand),
        ("net.minecraft.world.entity.LivingEntity", 49) => LivingItemBreak(EquipmentSlot::Head),
        ("net.minecraft.world.entity.LivingEntity", 50) => LivingItemBreak(EquipmentSlot::Chest),
        ("net.minecraft.world.entity.LivingEntity", 51) => LivingItemBreak(EquipmentSlot::Legs),
        ("net.minecraft.world.entity.LivingEntity", 52) => LivingItemBreak(EquipmentSlot::Feet),
        ("net.minecraft.world.entity.LivingEntity", 54) => HoneyJump,
        ("net.minecraft.world.entity.LivingEntity", 55) => LivingSwapHandItems,
        ("net.minecraft.world.entity.LivingEntity", 60) => LivingPoof,
        ("net.minecraft.world.entity.LivingEntity", 65) => LivingItemBreak(EquipmentSlot::Body),
        ("net.minecraft.world.entity.LivingEntity", 67) => LivingDrown,
        ("net.minecraft.world.entity.LivingEntity", 68) => LivingItemBreak(EquipmentSlot::Saddle),
        ("net.minecraft.world.entity.Mob", 20) => MobSpawn,
        ("net.minecraft.world.entity.TamableAnimal", 7)
        | ("net.minecraft.world.entity.animal.equine.AbstractHorse", 7) => {
            Taming { succeeded: true }
        }
        ("net.minecraft.world.entity.TamableAnimal", 6)
        | ("net.minecraft.world.entity.animal.equine.AbstractHorse", 6) => {
            Taming { succeeded: false }
        }
        ("net.minecraft.world.entity.animal.Animal", 18) => AnimalLove,
        ("net.minecraft.world.entity.animal.allay.Allay", 18) => AllayDuplication,
        ("net.minecraft.world.entity.animal.armadillo.Armadillo", 64) => ArmadilloPeek,
        ("net.minecraft.world.entity.animal.dolphin.Dolphin", 38) => DolphinHappy,
        ("net.minecraft.world.entity.animal.feline.Ocelot", 41) => {
            OcelotTrusting { succeeded: true }
        }
        ("net.minecraft.world.entity.animal.feline.Ocelot", 40) => {
            OcelotTrusting { succeeded: false }
        }
        ("net.minecraft.world.entity.animal.fox.Fox", 45) => FoxEat,
        ("net.minecraft.world.entity.animal.goat.Goat", 58) => GoatLowerHead,
        ("net.minecraft.world.entity.animal.goat.Goat", 59) => GoatRaiseHead,
        ("net.minecraft.world.entity.animal.golem.IronGolem", 4) => IronGolemAttack,
        ("net.minecraft.world.entity.animal.golem.IronGolem", 11) => IronGolemOfferFlower,
        ("net.minecraft.world.entity.animal.golem.IronGolem", 34) => IronGolemStopOfferFlower,
        ("net.minecraft.world.entity.animal.rabbit.Rabbit", 1) => RabbitJump,
        ("net.minecraft.world.entity.animal.sheep.Sheep", 10) => SheepEatGrass,
        ("net.minecraft.world.entity.animal.squid.Squid", 19) => SquidResetMovement,
        ("net.minecraft.world.entity.animal.wolf.Wolf", 8) => WolfStartShaking,
        ("net.minecraft.world.entity.animal.wolf.Wolf", 56) => WolfCancelShaking,
        ("net.minecraft.world.entity.decoration.ArmorStand", 32) => ArmorStandHit,
        ("net.minecraft.world.entity.monster.Ravager", 4) => RavagerAttack,
        ("net.minecraft.world.entity.monster.Ravager", 39) => RavagerStun,
        ("net.minecraft.world.entity.monster.Ravager", 69) => RavagerRoar,
        ("net.minecraft.world.entity.monster.Witch", 15) => WitchMagic,
        ("net.minecraft.world.entity.monster.Zoglin", 4) => ZoglinAttack,
        ("net.minecraft.world.entity.monster.creaking.Creaking", 66) => CreakingInvulnerable,
        ("net.minecraft.world.entity.monster.creaking.Creaking", 4) => CreakingAttack,
        ("net.minecraft.world.entity.monster.hoglin.Hoglin", 4) => HoglinAttack,
        ("net.minecraft.world.entity.monster.warden.Warden", 4) => WardenAttack,
        ("net.minecraft.world.entity.monster.warden.Warden", 61) => WardenTendril,
        ("net.minecraft.world.entity.monster.warden.Warden", 62) => WardenSonicBoom,
        ("net.minecraft.world.entity.monster.zombie.ZombieVillager", 16) => ZombieVillagerCure,
        ("net.minecraft.world.entity.npc.villager.Villager", 12) => VillagerHeart,
        ("net.minecraft.world.entity.npc.villager.Villager", 13) => VillagerAngry,
        ("net.minecraft.world.entity.npc.villager.Villager", 14) => VillagerHappy,
        ("net.minecraft.world.entity.npc.villager.Villager", 42) => VillagerSplash,
        ("net.minecraft.world.entity.player.Player", 9) => PlayerFinishUsingItem,
        ("net.minecraft.world.entity.player.Player", 23) => PlayerDisableReducedDebug,
        ("net.minecraft.world.entity.player.Player", 22) => PlayerEnableReducedDebug,
        ("net.minecraft.world.entity.projectile.EvokerFangs", 4) => EvokerFangsAttack,
        ("net.minecraft.world.entity.projectile.FireworkRocketEntity", 17) => FireworkExplosion,
        ("net.minecraft.world.entity.projectile.FishingHook", 31) => FishingHookPull,
        ("net.minecraft.world.entity.projectile.arrow.Arrow", 0) => ArrowEffectClear,
        ("net.minecraft.world.entity.projectile.throwableitemprojectile.Snowball", 3) => {
            SnowballHit
        }
        ("net.minecraft.world.entity.projectile.throwableitemprojectile.ThrownEgg", 3) => {
            ThrownEggHit
        }
        ("net.minecraft.world.entity.vehicle.minecart.MinecartSpawner", 1) => {
            MinecartSpawnerDelayReset
        }
        ("net.minecraft.world.entity.vehicle.minecart.MinecartTNT", 10) => MinecartTntPrime,
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
    const A: EventConsumerDisposition = EventConsumerDisposition {
        canonical: false,
        animation: true,
        particle: false,
        sound: false,
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
        | WardenSonicBoom => A,
        ArmadilloPeek | IronGolemAttack | ArmorStandHit | RavagerAttack | ZoglinAttack
        | CreakingInvulnerable | CreakingAttack | HoglinAttack | EvokerFangsAttack => AS,
        RabbitJump | RavagerRoar => AP,
        ArrowEffectClear => CP,
        TotemUse => CPS,
        GuardianAttackSound | SnifferDiggingSound => S,
        LivingSwapHandItems
        | SquidResetMovement
        | PlayerFinishUsingItem
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
                        "missing bbb consumer for {} event {}",
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
}
