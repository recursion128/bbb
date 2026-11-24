//! `LivingEntity.makeSound` of each mob's hurt and death sound
//! (LivingEntity.java:1245-1256,1402-1410). The vanilla client never plays
//! these for other entities itself, so the server broadcasts them.

use bbb_protocol::entity_types::*;
use bbb_protocol::packets::SoundEventHolder;

use super::{living, Entity, EntityData, EntityStore};
use crate::entity::sound_variants::{registered_sound, sound_holder};

/// `wolf_armor_absorbs` is `Wolf.canArmorAbsorb(source)` (Wolf.java:284-286).
pub(super) fn play_hurt_or_death(
    entity: &Entity,
    store: &mut EntityStore,
    wolf_armor_absorbs: bool,
) {
    if entity.is_silent() {
        return;
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let death = entity
        .living()
        .is_some_and(|living| living.is_dead_or_dying());
    let baby = data.is_baby();
    let type_id = entity.entity_type_id();
    let Some(sound) = sound(entity, store, type_id, death, baby, wolf_armor_absorbs) else {
        return;
    };
    let volume = match type_id {
        VANILLA_ENTITY_TYPE_COW_ID
        | VANILLA_ENTITY_TYPE_MOOSHROOM_ID
        | VANILLA_ENTITY_TYPE_WOLF_ID
        | VANILLA_ENTITY_TYPE_ALLAY_ID
        | VANILLA_ENTITY_TYPE_BEE_ID
        | VANILLA_ENTITY_TYPE_SQUID_ID
        | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => 0.4,
        VANILLA_ENTITY_TYPE_GHAST_ID | VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => 5.0,
        VANILLA_ENTITY_TYPE_WARDEN_ID => 4.0,
        VANILLA_ENTITY_TYPE_BAT_ID => 0.1,
        // HappyGhast.java:232-234.
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID if !baby => 4.0,
        // Slime.java:301-303.
        VANILLA_ENTITY_TYPE_SLIME_ID | VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => {
            0.4 * slime_size(entity) as f32
        }
        // AbstractHorse.java:397-399.
        VANILLA_ENTITY_TYPE_HORSE_ID
        | VANILLA_ENTITY_TYPE_DONKEY_ID
        | VANILLA_ENTITY_TYPE_MULE_ID
        | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
        | VANILLA_ENTITY_TYPE_LLAMA_ID
        | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
        | VANILLA_ENTITY_TYPE_CAMEL_ID
        | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID => 0.8,
        _ => 1.0,
    };
    let random = presentation_random(entity, store);
    // `getVoicePitch` (LivingEntity.java:2284-2288) and its overrides:
    // Parrot.java:371-377, HappyGhast.java:201-203, ZombieVillager.java:308-312,
    // Bat.java:66-68.
    let voice = |base: f32| (random.next_float() - random.next_float()) * 0.2 + base;
    let pitch = match type_id {
        VANILLA_ENTITY_TYPE_PARROT_ID => voice(1.0),
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => 1.0,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID if baby => voice(2.0),
        VANILLA_ENTITY_TYPE_BAT_ID => voice(if baby { 1.5 } else { 1.0 }) * 0.95,
        _ => voice(if baby { 1.5 } else { 1.0 }),
    };
    emit(store, &random, sound, entity, volume, pitch);
}

/// Vanilla draws these from the entity's own random and the level's
/// `threadSafeRandom`; this world shares one gameplay stream, so the hurt
/// presentation draws from a throwaway one instead of shifting it.
fn presentation_random(entity: &Entity, store: &EntityStore) -> crate::random::GameRandom {
    let (high, low) = entity.uuid().as_u64_pair();
    crate::random::GameRandom::with_seed((high ^ low) as i64 ^ store.game_time())
}

fn emit(
    store: &mut EntityStore,
    random: &crate::random::GameRandom,
    sound: SoundEventHolder,
    entity: &Entity,
    volume: f32,
    pitch: f32,
) {
    store.emit_entity_sound(bbb_protocol::packets::SoundEvent {
        sound,
        position: entity.position(),
        source: living::living_sound_source(entity),
        volume,
        pitch,
        seed: random.next_long(),
    });
}

/// `IronGolem.hurtServer` (IronGolem.java:206-213): a crackiness step plays
/// the crack sound after the hurt itself.
pub(super) fn iron_golem_crackiness(entity: &Entity) -> Option<u8> {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_IRON_GOLEM_ID {
        return None;
    }
    let living = entity.living()?;
    // Crackiness.GOLEM (Crackiness.java:6,18-26).
    let fraction = living.health() / living.max_health();
    Some(if fraction < 0.25 {
        3
    } else if fraction < 0.5 {
        2
    } else if fraction < 0.75 {
        1
    } else {
        0
    })
}

pub(super) fn play_iron_golem_crack(entity: &Entity, store: &mut EntityStore, before: Option<u8>) {
    if before.is_none() || before == iron_golem_crackiness(entity) || entity.is_silent() {
        return;
    }
    if let Some(sound) = registered_sound("minecraft:entity.iron_golem.damage") {
        let random = presentation_random(entity, store);
        emit(store, &random, sound, entity, 1.0, 1.0);
    }
}

fn slime_size(entity: &Entity) -> i32 {
    match entity.data() {
        EntityData::GeneralLiving(data) => data.slime_retained().map_or(1, |slime| slime.size()),
        _ => 1,
    }
}

fn sound(
    entity: &Entity,
    store: &EntityStore,
    type_id: i32,
    death: bool,
    baby: bool,
    wolf_armor_absorbs: bool,
) -> Option<SoundEventHolder> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let pick = |hurt: &'static str, die: &'static str| if death { die } else { hurt };
    let id = match type_id {
        VANILLA_ENTITY_TYPE_COW_ID
        | VANILLA_ENTITY_TYPE_CHICKEN_ID
        | VANILLA_ENTITY_TYPE_PIG_ID
        | VANILLA_ENTITY_TYPE_CAT_ID
        | VANILLA_ENTITY_TYPE_WOLF_ID => {
            if type_id == VANILLA_ENTITY_TYPE_WOLF_ID && wolf_armor_absorbs && !death {
                "minecraft:item.wolf_armor.damage"
            } else if let Some(sound) = variant_sound(entity, store, type_id, death, baby) {
                return Some(sound);
            } else {
                match type_id {
                    VANILLA_ENTITY_TYPE_COW_ID => {
                        pick("minecraft:entity.cow.hurt", "minecraft:entity.cow.death")
                    }
                    VANILLA_ENTITY_TYPE_CHICKEN_ID => pick(
                        "minecraft:entity.chicken.hurt",
                        "minecraft:entity.chicken.death",
                    ),
                    VANILLA_ENTITY_TYPE_PIG_ID => {
                        pick("minecraft:entity.pig.hurt", "minecraft:entity.pig.death")
                    }
                    VANILLA_ENTITY_TYPE_CAT_ID => {
                        pick("minecraft:entity.cat.hurt", "minecraft:entity.cat.death")
                    }
                    _ => pick("minecraft:entity.wolf.hurt", "minecraft:entity.wolf.death"),
                }
            }
        }
        // AbstractCow.java:84-86: the mooshroom keeps the classic cow set.
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID => {
            pick("minecraft:entity.cow.hurt", "minecraft:entity.cow.death")
        }
        VANILLA_ENTITY_TYPE_ARMADILLO_ID
            if !death && crate::entity::armadillo::is_scared(entity) =>
        {
            "minecraft:entity.armadillo.hurt_reduced"
        }
        VANILLA_ENTITY_TYPE_GOAT_ID
            if data.goat_retained().is_some_and(|goat| goat.is_screaming()) =>
        {
            pick(
                "minecraft:entity.goat.screaming.hurt",
                "minecraft:entity.goat.screaming.death",
            )
        }
        VANILLA_ENTITY_TYPE_SHULKER_ID
            if !death
                && data
                    .shulker_state()
                    .is_some_and(|shulker| shulker.is_closed()) =>
        {
            "minecraft:entity.shulker.hurt_closed"
        }
        VANILLA_ENTITY_TYPE_SLIME_ID if slime_size(entity) <= 1 => pick(
            "minecraft:entity.slime.hurt_small",
            "minecraft:entity.slime.death_small",
        ),
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID if slime_size(entity) <= 1 => pick(
            "minecraft:entity.magma_cube.hurt_small",
            "minecraft:entity.magma_cube.death_small",
        ),
        VANILLA_ENTITY_TYPE_DROWNED_ID if entity.is_in_water() => pick(
            "minecraft:entity.drowned.hurt_water",
            "minecraft:entity.drowned.death_water",
        ),
        VANILLA_ENTITY_TYPE_GUARDIAN_ID if !entity.is_in_water() => pick(
            "minecraft:entity.guardian.hurt_land",
            "minecraft:entity.guardian.death_land",
        ),
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID if !entity.is_in_water() => pick(
            "minecraft:entity.elder_guardian.hurt_land",
            "minecraft:entity.elder_guardian.death_land",
        ),
        VANILLA_ENTITY_TYPE_NAUTILUS_ID => match (baby, entity.is_under_water()) {
            (true, true) => pick(
                "minecraft:entity.baby_nautilus.hurt",
                "minecraft:entity.baby_nautilus.death",
            ),
            (true, false) => pick(
                "minecraft:entity.baby_nautilus.hurt_land",
                "minecraft:entity.baby_nautilus.death_land",
            ),
            (false, true) => pick(
                "minecraft:entity.nautilus.hurt",
                "minecraft:entity.nautilus.death",
            ),
            (false, false) => pick(
                "minecraft:entity.nautilus.hurt_land",
                "minecraft:entity.nautilus.death_land",
            ),
        },
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID if !entity.is_under_water() => pick(
            "minecraft:entity.zombie_nautilus.hurt_land",
            "minecraft:entity.zombie_nautilus.death_land",
        ),
        VANILLA_ENTITY_TYPE_HORSE_ID if baby => pick(
            "minecraft:entity.baby_horse.hurt",
            "minecraft:entity.baby_horse.death",
        ),
        VANILLA_ENTITY_TYPE_TURTLE_ID if baby => pick(
            "minecraft:entity.turtle.hurt_baby",
            "minecraft:entity.turtle.death_baby",
        ),
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID if baby => pick(
            "minecraft:entity.ghastling.hurt",
            "minecraft:entity.ghastling.death",
        ),
        // CopperGolemOxidationLevels.java:9-38.
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => match crate::entity::copper_golem::weather_state(
            data,
        ) {
            bbb_protocol::entity_synched_data::VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID => {
                pick(
                    "minecraft:entity.copper_golem_weathered.hurt",
                    "minecraft:entity.copper_golem_weathered.death",
                )
            }
            bbb_protocol::entity_synched_data::VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID => {
                pick(
                    "minecraft:entity.copper_golem_oxidized.hurt",
                    "minecraft:entity.copper_golem_oxidized.death",
                )
            }
            _ => pick(
                "minecraft:entity.copper_golem.hurt",
                "minecraft:entity.copper_golem.death",
            ),
        },
        _ => {
            if let Some((hurt, die)) = fixed_sounds(type_id) {
                pick(hurt, die)
            } else {
                let content_registry = store.plugins().content_registry();
                let spec = content_registry.entity_by_id(type_id)?;
                let sound_id = if death {
                    spec.death_sound.as_deref()?
                } else {
                    spec.hurt_sound.as_deref()?
                };
                return crate::entity::sound_variants::registered_sound_with_registry(
                    sound_id,
                    content_registry,
                );
            }
        }
    };
    registered_sound(id)
}

fn variant_sound(
    entity: &Entity,
    store: &EntityStore,
    type_id: i32,
    death: bool,
    baby: bool,
) -> Option<SoundEventHolder> {
    let name = super::sound_variant_resource_id(entity)?;
    let resolver = store.item_resolver();
    macro_rules! adult_or_baby {
        ($variant:expr) => {{
            let set = if baby {
                &$variant.baby_sounds
            } else {
                &$variant.adult_sounds
            };
            if death {
                &set.death_sound
            } else {
                &set.hurt_sound
            }
        }};
    }
    let sound = match type_id {
        VANILLA_ENTITY_TYPE_COW_ID => {
            let variant = resolver.cow_sound_variant(name)?;
            if death {
                &variant.death_sound
            } else {
                &variant.hurt_sound
            }
        }
        VANILLA_ENTITY_TYPE_CHICKEN_ID => adult_or_baby!(resolver.chicken_sound_variant(name)?),
        VANILLA_ENTITY_TYPE_PIG_ID => adult_or_baby!(resolver.pig_sound_variant(name)?),
        VANILLA_ENTITY_TYPE_WOLF_ID => adult_or_baby!(resolver.wolf_sound_variant(name)?),
        VANILLA_ENTITY_TYPE_CAT_ID => {
            let holder = resolver.holder_id("minecraft:cat_sound_variant", name)?;
            adult_or_baby!(resolver.cat_sound_variant(holder)?)
        }
        _ => return None,
    };
    sound_holder(sound)
}

/// The constant `getHurtSound` / `getDeathSound` of each remaining type;
/// `None` for the golems that inherit AbstractGolem's silence
/// (AbstractGolem.java:21-28) and for types handled elsewhere.
fn fixed_sounds(type_id: i32) -> Option<(&'static str, &'static str)> {
    macro_rules! s {
        ($name:literal) => {
            (
                concat!("minecraft:entity.", $name, ".hurt"),
                concat!("minecraft:entity.", $name, ".death"),
            )
        };
    }
    Some(match type_id {
        VANILLA_ENTITY_TYPE_ALLAY_ID => s!("allay"),
        VANILLA_ENTITY_TYPE_ARMADILLO_ID => s!("armadillo"),
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => s!("axolotl"),
        VANILLA_ENTITY_TYPE_BAT_ID => s!("bat"),
        VANILLA_ENTITY_TYPE_BEE_ID => s!("bee"),
        VANILLA_ENTITY_TYPE_BLAZE_ID => s!("blaze"),
        VANILLA_ENTITY_TYPE_BOGGED_ID => s!("bogged"),
        VANILLA_ENTITY_TYPE_BREEZE_ID => s!("breeze"),
        VANILLA_ENTITY_TYPE_CAMEL_ID => s!("camel"),
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID => s!("camel_husk"),
        VANILLA_ENTITY_TYPE_SPIDER_ID | VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID => s!("spider"),
        VANILLA_ENTITY_TYPE_COD_ID => s!("cod"),
        // Creaking.java:428-435: off its heart the hurt falls back to Monster's.
        VANILLA_ENTITY_TYPE_CREAKING_ID => (
            "minecraft:entity.hostile.hurt",
            "minecraft:entity.creaking.death",
        ),
        VANILLA_ENTITY_TYPE_CREEPER_ID => s!("creeper"),
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => s!("dolphin"),
        VANILLA_ENTITY_TYPE_DONKEY_ID => s!("donkey"),
        VANILLA_ENTITY_TYPE_DROWNED_ID => s!("drowned"),
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID => s!("elder_guardian"),
        VANILLA_ENTITY_TYPE_ENDERMAN_ID => s!("enderman"),
        VANILLA_ENTITY_TYPE_ENDERMITE_ID => s!("endermite"),
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => (
            "minecraft:entity.ender_dragon.hurt",
            "minecraft:entity.generic.death",
        ),
        VANILLA_ENTITY_TYPE_EVOKER_ID => s!("evoker"),
        VANILLA_ENTITY_TYPE_FOX_ID => s!("fox"),
        VANILLA_ENTITY_TYPE_FROG_ID => s!("frog"),
        VANILLA_ENTITY_TYPE_GHAST_ID => s!("ghast"),
        VANILLA_ENTITY_TYPE_GIANT_ID => s!("hostile"),
        VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => s!("glow_squid"),
        VANILLA_ENTITY_TYPE_GOAT_ID => s!("goat"),
        VANILLA_ENTITY_TYPE_GUARDIAN_ID => s!("guardian"),
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => s!("happy_ghast"),
        VANILLA_ENTITY_TYPE_HOGLIN_ID => s!("hoglin"),
        VANILLA_ENTITY_TYPE_HORSE_ID => s!("horse"),
        VANILLA_ENTITY_TYPE_HUSK_ID => s!("husk"),
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID => s!("illusioner"),
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => s!("iron_golem"),
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => s!("llama"),
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => s!("magma_cube"),
        VANILLA_ENTITY_TYPE_MANNEQUIN_ID => s!("generic"),
        VANILLA_ENTITY_TYPE_MULE_ID => s!("mule"),
        VANILLA_ENTITY_TYPE_OCELOT_ID => s!("ocelot"),
        VANILLA_ENTITY_TYPE_PANDA_ID => s!("panda"),
        VANILLA_ENTITY_TYPE_PARCHED_ID => s!("parched"),
        VANILLA_ENTITY_TYPE_PARROT_ID => s!("parrot"),
        VANILLA_ENTITY_TYPE_PHANTOM_ID => s!("phantom"),
        VANILLA_ENTITY_TYPE_PIGLIN_ID => s!("piglin"),
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => s!("piglin_brute"),
        VANILLA_ENTITY_TYPE_PILLAGER_ID => s!("pillager"),
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => s!("polar_bear"),
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID => s!("puffer_fish"),
        VANILLA_ENTITY_TYPE_RABBIT_ID => s!("rabbit"),
        VANILLA_ENTITY_TYPE_RAVAGER_ID => s!("ravager"),
        VANILLA_ENTITY_TYPE_SALMON_ID => s!("salmon"),
        VANILLA_ENTITY_TYPE_SHEEP_ID => s!("sheep"),
        VANILLA_ENTITY_TYPE_SHULKER_ID => s!("shulker"),
        VANILLA_ENTITY_TYPE_SILVERFISH_ID => s!("silverfish"),
        VANILLA_ENTITY_TYPE_SKELETON_ID => s!("skeleton"),
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => s!("skeleton_horse"),
        VANILLA_ENTITY_TYPE_SLIME_ID => s!("slime"),
        VANILLA_ENTITY_TYPE_SNIFFER_ID => s!("sniffer"),
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID => s!("snow_golem"),
        VANILLA_ENTITY_TYPE_SQUID_ID => s!("squid"),
        VANILLA_ENTITY_TYPE_STRAY_ID => s!("stray"),
        VANILLA_ENTITY_TYPE_STRIDER_ID => s!("strider"),
        VANILLA_ENTITY_TYPE_TADPOLE_ID => s!("tadpole"),
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => s!("tropical_fish"),
        VANILLA_ENTITY_TYPE_TURTLE_ID => s!("turtle"),
        VANILLA_ENTITY_TYPE_VEX_ID => s!("vex"),
        VANILLA_ENTITY_TYPE_VILLAGER_ID => s!("villager"),
        VANILLA_ENTITY_TYPE_VINDICATOR_ID => s!("vindicator"),
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID => s!("wandering_trader"),
        VANILLA_ENTITY_TYPE_WARDEN_ID => s!("warden"),
        VANILLA_ENTITY_TYPE_WITCH_ID => s!("witch"),
        VANILLA_ENTITY_TYPE_WITHER_ID => s!("wither"),
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID => s!("wither_skeleton"),
        VANILLA_ENTITY_TYPE_ZOGLIN_ID => s!("zoglin"),
        VANILLA_ENTITY_TYPE_ZOMBIE_ID => s!("zombie"),
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => s!("zombie_horse"),
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => s!("zombie_nautilus"),
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => s!("zombie_villager"),
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => s!("zombified_piglin"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fixed_sound_is_a_registered_sound_event() {
        for type_id in 0..bbb_protocol::entity_types::VANILLA_ENTITY_TYPES.len() as i32 {
            if let Some((hurt, death)) = fixed_sounds(type_id) {
                assert!(registered_sound(hurt).is_some(), "{hurt}");
                assert!(registered_sound(death).is_some(), "{death}");
            }
        }
        let source = include_str!("hurt_sounds.rs");
        for literal in source.split('"').filter(|part| {
            part.starts_with("minecraft:") && part.contains('.') && !part.ends_with('.')
        }) {
            assert!(registered_sound(literal).is_some(), "{literal}");
        }
    }
}
