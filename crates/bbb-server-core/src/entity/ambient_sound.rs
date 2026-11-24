//! `Mob.playAmbientSound` (Mob.java:269-298): the periodic clock in
//! `baseTick` and the one-off playback after fresh spawn finalization.

use super::{living, Entity, EntityData, EntityStore};
use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_facts::entity_ambient_sounds::{
    vanilla_mob_spawn_ambient_sound, VanillaMobSpawnAmbientSound,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_ARMADILLO_ID, VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_BREEZE_ID,
    VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COW_ID,
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    VANILLA_ENTITY_TYPE_DROWNED_ID, VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID,
    VANILLA_ENTITY_TYPE_ENDERMAN_ID, VANILLA_ENTITY_TYPE_FOX_ID, VANILLA_ENTITY_TYPE_GHAST_ID,
    VANILLA_ENTITY_TYPE_GOAT_ID, VANILLA_ENTITY_TYPE_GUARDIAN_ID,
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
    VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_OCELOT_ID,
    VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_PARROT_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_PIG_ID,
    VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_SHULKER_ID,
    VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
    VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, VANILLA_ENTITY_TYPE_WARDEN_ID,
    VANILLA_ENTITY_TYPE_WITCH_ID, VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{SoundEvent, SoundEventHolder};

impl EntityStore {
    // EntityType.java:1308-1313: spawning plays immediately without resetting the ambient clock.
    pub(crate) fn play_spawn_ambient(
        &mut self,
        entity_id: i32,
        level: Option<&LevelCtx<'_, '_>>,
    ) -> bool {
        let random = self.random().clone();
        self.with_entity_mut(entity_id, |entity, store| {
            play(entity, store, level, &random)
        })
        .unwrap_or(false)
    }
}

/// `Mob.baseTick` (Mob.java:282-285) for every mob without its own clock.
/// The roll is hashed from the mob and its tick count, and the presentation
/// draws from a throwaway random, so ambient sounds leave the shared gameplay
/// stream alone.
pub(crate) fn tick(entity: &Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    if has_own_clock(entity.entity_type_id()) || !entity.is_alive_with_health() {
        return;
    }
    let (high, low) = entity.uuid().as_u64_pair();
    let seed = high ^ low.rotate_left(32) ^ entity.tick_count() as u64;
    let Some(runtime) = store.mob_ai_mut(entity.id()) else {
        return;
    };
    let roll = (mix(seed) % 1000) as i32;
    let time = runtime.ambient_sound_time();
    runtime.set_ambient_sound_time(time.wrapping_add(1));
    if roll >= time {
        return;
    }
    reset_clock(entity, store);
    if entity.is_silent() {
        return;
    }
    let random = crate::random::GameRandom::with_seed(mix(seed ^ 0x5bd1_e995) as i64);
    play(entity, store, Some(level), &random);
}

/// `Mob.resetAmbientSoundTime` (Mob.java:296-298), which `Mob.playHurtSound`
/// also calls (`:290-294`).
pub(crate) fn reset_clock(entity: &Entity, store: &mut EntityStore) {
    let interval = ambient_sound_interval(entity, store);
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.set_ambient_sound_time(-interval);
    }
}

fn mix(value: u64) -> u64 {
    let product = u128::from(value.wrapping_add(0xa076_1d64_78bd_642f))
        * u128::from(value ^ 0xe703_7ed1_a0b4_28db);
    ((product >> 64) as u64) ^ (product as u64)
}

/// Types whose own tick already runs this clock.
fn has_own_clock(type_id: i32) -> bool {
    matches!(
        type_id,
        VANILLA_ENTITY_TYPE_BAT_ID
            | VANILLA_ENTITY_TYPE_GHAST_ID
            | VANILLA_ENTITY_TYPE_BLAZE_ID
            | VANILLA_ENTITY_TYPE_WITCH_ID
            | VANILLA_ENTITY_TYPE_PARROT_ID
            | VANILLA_ENTITY_TYPE_VILLAGER_ID
            | VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
    ) || super::general_living::is_abstract_horse_entity_type(type_id)
}

/// `getAmbientSoundInterval` and its overrides.
fn ambient_sound_interval(entity: &Entity, store: &EntityStore) -> i32 {
    use super::ai::mob::spec::is_instance_of;
    let type_id = entity.entity_type_id();
    match type_id {
        // Ocelot.java:144-146.
        VANILLA_ENTITY_TYPE_OCELOT_ID => 900,
        // Turtle.java:168-170.
        VANILLA_ENTITY_TYPE_TURTLE_ID => 200,
        // Guardian.java:153-155.
        VANILLA_ENTITY_TYPE_GUARDIAN_ID | VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID => 160,
        // HappyGhast.java:211-214.
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID if !store.passengers(entity.id()).is_empty() => 720,
        // AbstractHorse.java:402-404.
        _ if super::general_living::is_abstract_horse_entity_type(type_id) => 400,
        // Animal.java:122, AbstractGolem.java:31, WaterAnimal.java:30,
        // AgeableWaterCreature.java:28 (Cat.java:261 repeats Animal's).
        _ if [
            "Animal",
            "AbstractGolem",
            "WaterAnimal",
            "AgeableWaterCreature",
        ]
        .into_iter()
        .any(|class| is_instance_of(type_id, class)) =>
        {
            120
        }
        _ => 80,
    }
}

fn voice_pitch(random: &GameRandom, baby: bool) -> f32 {
    let base = if baby { 1.5 } else { 1.0 };
    (random.next_float() - random.next_float()) * 0.2 + base
}

// Mob.java:273-275; LivingEntity.java:1406-1409,2280-2288.
fn play(
    entity: &Entity,
    store: &mut EntityStore,
    level: Option<&LevelCtx<'_, '_>>,
    random: &GameRandom,
) -> bool {
    let baby = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
    let mut selected_pitch = None;
    let (sound, volume) = match vanilla_mob_spawn_ambient_sound(entity.data().entity_resource_id())
    {
        Some(_)
            if matches!(
                entity.entity_type_id(),
                VANILLA_ENTITY_TYPE_PIGLIN_ID | VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
            ) =>
        {
            let Some(level) = level else {
                return false;
            };
            let Some(sound) = super::piglin::ambient_sound(entity, store, level) else {
                return true;
            };
            (super::sound_variants::registered_sound(sound), 1.0)
        }
        Some(VanillaMobSpawnAmbientSound::Dynamic)
            if entity.entity_type_id() == VANILLA_ENTITY_TYPE_FOX_ID =>
        {
            let Some(level) = level else {
                return false;
            };
            let Some((sound, volume)) = super::fox::ambient_sound(entity, level) else {
                return false;
            };
            (super::sound_variants::registered_sound(sound), volume)
        }
        Some(VanillaMobSpawnAmbientSound::Dynamic)
            if entity.entity_type_id() == VANILLA_ENTITY_TYPE_WARDEN_ID =>
        {
            let Some(level) = level else {
                return false;
            };
            let Some(sound) = super::warden::anger::ambient_sound(entity, store, level) else {
                return true;
            };
            (super::sound_variants::registered_sound(sound), 4.0)
        }
        Some(VanillaMobSpawnAmbientSound::Fixed { sound, volume }) => {
            (super::sound_variants::registered_sound(sound), volume)
        }
        Some(VanillaMobSpawnAmbientSound::Silent) => return true,
        // Breeze.java:183-187; Level.java:473-474: its ambient playback is client-local.
        Some(VanillaMobSpawnAmbientSound::Dynamic)
            if entity.entity_type_id() == VANILLA_ENTITY_TYPE_BREEZE_ID =>
        {
            return true;
        }
        Some(VanillaMobSpawnAmbientSound::Dynamic)
            if matches!(
                entity.entity_type_id(),
                VANILLA_ENTITY_TYPE_ALLAY_ID
                    | VANILLA_ENTITY_TYPE_ARMADILLO_ID
                    | VANILLA_ENTITY_TYPE_AXOLOTL_ID
                    | VANILLA_ENTITY_TYPE_BAT_ID
                    | VANILLA_ENTITY_TYPE_GOAT_ID
                    | VANILLA_ENTITY_TYPE_HORSE_ID
                    | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
                    | VANILLA_ENTITY_TYPE_SNIFFER_ID
                    | VANILLA_ENTITY_TYPE_NAUTILUS_ID
                    | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
                    | VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
                    | VANILLA_ENTITY_TYPE_STRIDER_ID
                    | VANILLA_ENTITY_TYPE_TURTLE_ID
                    | VANILLA_ENTITY_TYPE_SHULKER_ID
                    | VANILLA_ENTITY_TYPE_VILLAGER_ID
                    | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
                    | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
                    | VANILLA_ENTITY_TYPE_ZOGLIN_ID
                    | VANILLA_ENTITY_TYPE_HOGLIN_ID
                    | VANILLA_ENTITY_TYPE_ENDERMAN_ID
                    | VANILLA_ENTITY_TYPE_POLAR_BEAR_ID
                    | VANILLA_ENTITY_TYPE_GUARDIAN_ID
                    | VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID
                    | VANILLA_ENTITY_TYPE_DROWNED_ID
                    | VANILLA_ENTITY_TYPE_DOLPHIN_ID
                    | VANILLA_ENTITY_TYPE_PANDA_ID
                    | VANILLA_ENTITY_TYPE_CREAKING_ID
                    | VANILLA_ENTITY_TYPE_PARROT_ID
            ) =>
        {
            let EntityData::GeneralLiving(data) = entity.data() else {
                return false;
            };
            let sound = match entity.entity_type_id() {
                VANILLA_ENTITY_TYPE_PARROT_ID => {
                    let Some(difficulty) = store.difficulty() else {
                        return false;
                    };
                    super::parrot_sounds::ambient(random, difficulty)
                }
                // Creaking.java:423-425.
                VANILLA_ENTITY_TYPE_CREAKING_ID => {
                    let Some(state) = data.creaking_retained() else {
                        return false;
                    };
                    if state.is_active() {
                        return true;
                    }
                    "minecraft:entity.creaking.ambient"
                }
                VANILLA_ENTITY_TYPE_SNIFFER_ID => {
                    let Some(state) = data.sniffer_state() else {
                        return false;
                    };
                    let Some(sound) = state.ambient_sound() else {
                        return true;
                    };
                    sound
                }
                // Nautilus.java:64-70.
                VANILLA_ENTITY_TYPE_NAUTILUS_ID => match (baby, entity.is_under_water()) {
                    (true, true) => "minecraft:entity.baby_nautilus.ambient",
                    (true, false) => "minecraft:entity.baby_nautilus.ambient_land",
                    (false, true) => "minecraft:entity.nautilus.ambient",
                    (false, false) => "minecraft:entity.nautilus.ambient_land",
                },
                // ZombieNautilus.java:87-89.
                VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
                    if entity.is_under_water() {
                        "minecraft:entity.zombie_nautilus.ambient"
                    } else {
                        "minecraft:entity.zombie_nautilus.ambient_land"
                    }
                }
                // HappyGhast.java:201-203,217-219.
                VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => {
                    selected_pitch = Some(1.0);
                    if baby {
                        "minecraft:entity.ghastling.ambient"
                    } else {
                        "minecraft:entity.happy_ghast.ambient"
                    }
                }
                // Turtle.java:173-175.
                VANILLA_ENTITY_TYPE_TURTLE_ID => {
                    if baby || entity.is_in_water() || !entity.on_ground() {
                        return true;
                    }
                    "minecraft:entity.turtle.ambient_land"
                }
                // Axolotl.java:148-152,508-510.
                VANILLA_ENTITY_TYPE_AXOLOTL_ID => {
                    if super::axolotl::is_playing_dead(entity) {
                        return true;
                    }
                    if entity.is_in_water() {
                        "minecraft:entity.axolotl.idle_water"
                    } else {
                        "minecraft:entity.axolotl.idle_air"
                    }
                }
                VANILLA_ENTITY_TYPE_BAT_ID => {
                    let Some(bat) = data.bat_retained() else {
                        return false;
                    };
                    let Some(runtime) = store.mob_ai_mut(entity.id()) else {
                        return false;
                    };
                    let Some(pitch) = runtime.bat_ambient_pitch(bat.is_resting()) else {
                        return true;
                    };
                    selected_pitch = Some(pitch);
                    "minecraft:entity.bat.ambient"
                }
                // Goat.java:122-124,335-340.
                VANILLA_ENTITY_TYPE_GOAT_ID => {
                    let Some(state) = data.goat_retained() else {
                        return false;
                    };
                    if state.is_screaming() {
                        "minecraft:entity.goat.screaming.ambient"
                    } else {
                        "minecraft:entity.goat.ambient"
                    }
                }
                VANILLA_ENTITY_TYPE_HORSE_ID => {
                    super::general_living::horse_runtime::horse_ambient_sound(baby)
                }
                VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => {
                    super::general_living::horse_runtime::skeleton_horse_ambient_sound(entity)
                }
                VANILLA_ENTITY_TYPE_STRIDER_ID => {
                    let Some(runtime) = store.mob_ai(entity.id()) else {
                        return false;
                    };
                    let Some(sound) = super::strider::ambient_sound(runtime) else {
                        return true;
                    };
                    sound
                }
                // Shulker.java:116-125,436-437.
                VANILLA_ENTITY_TYPE_SHULKER_ID => {
                    let Some(state) = data.shulker_state() else {
                        return false;
                    };
                    if state.is_closed() {
                        return true;
                    }
                    "minecraft:entity.shulker.ambient"
                }
                VANILLA_ENTITY_TYPE_VILLAGER_ID => {
                    let Some(sound) = super::villager::ambient_sound(entity) else {
                        return true;
                    };
                    sound
                }
                // ZombieVillager.java:308-317.
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => {
                    if data.zombie_retained().is_none() {
                        return false;
                    }
                    if baby {
                        selected_pitch = Some(voice_pitch(random, true) + 0.5);
                    }
                    "minecraft:entity.zombie_villager.ambient"
                }
                // ZombifiedPiglin.java:209-211.
                VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => {
                    let Some(anger) = super::ai::mob::target::anger::read(entity) else {
                        return false;
                    };
                    if anger.is_angry(store.game_time()) {
                        "minecraft:entity.zombified_piglin.angry"
                    } else {
                        "minecraft:entity.zombified_piglin.ambient"
                    }
                }
                // Zoglin.java:272-277.
                VANILLA_ENTITY_TYPE_ZOGLIN_ID => {
                    let Some(runtime) = store.mob_ai(entity.id()) else {
                        return false;
                    };
                    if runtime.has_target() {
                        "minecraft:entity.zoglin.angry"
                    } else {
                        "minecraft:entity.zoglin.ambient"
                    }
                }
                VANILLA_ENTITY_TYPE_HOGLIN_ID => {
                    let (Some(state), Some(zombifies)) = (
                        data.zombification(),
                        level.and_then(|level| level.piglins_zombify(entity.position())),
                    ) else {
                        return false;
                    };
                    let converting = state.is_converting(data.is_no_ai(), zombifies);
                    let Some(sound) = super::hoglin::ambient_sound(entity, store, converting)
                    else {
                        return false;
                    };
                    sound
                }
                // Allay.java:181-183,196-198.
                VANILLA_ENTITY_TYPE_ALLAY_ID => {
                    let held = entity
                        .living()
                        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
                        .is_some_and(|stack| !stack.is_empty());
                    if held {
                        "minecraft:entity.allay.ambient_with_item"
                    } else {
                        "minecraft:entity.allay.ambient_without_item"
                    }
                }
                // Armadillo.java:97-99,340-342.
                VANILLA_ENTITY_TYPE_ARMADILLO_ID => {
                    if super::armadillo::is_scared(entity) {
                        return true;
                    }
                    "minecraft:entity.armadillo.ambient"
                }
                // EnderMan.java:305-307.
                VANILLA_ENTITY_TYPE_ENDERMAN_ID => {
                    let Some(state) = data.enderman_retained() else {
                        return false;
                    };
                    if state.creepy() {
                        "minecraft:entity.enderman.scream"
                    } else {
                        "minecraft:entity.enderman.ambient"
                    }
                }
                // PolarBear.java:151-153.
                VANILLA_ENTITY_TYPE_POLAR_BEAR_ID if baby => {
                    "minecraft:entity.polar_bear.ambient_baby"
                }
                VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => "minecraft:entity.polar_bear.ambient",
                // Guardian.java:158-160; ElderGuardian.java:44-46.
                VANILLA_ENTITY_TYPE_GUARDIAN_ID if entity.is_in_water() => {
                    "minecraft:entity.guardian.ambient"
                }
                VANILLA_ENTITY_TYPE_GUARDIAN_ID => "minecraft:entity.guardian.ambient_land",
                VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID if entity.is_in_water() => {
                    "minecraft:entity.elder_guardian.ambient"
                }
                VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID => {
                    "minecraft:entity.elder_guardian.ambient_land"
                }
                // Drowned.java:166-168; Dolphin.java:335-337.
                VANILLA_ENTITY_TYPE_DROWNED_ID if entity.is_in_water() => {
                    "minecraft:entity.drowned.ambient_water"
                }
                VANILLA_ENTITY_TYPE_DROWNED_ID => "minecraft:entity.drowned.ambient",
                VANILLA_ENTITY_TYPE_DOLPHIN_ID if entity.is_in_water() => {
                    "minecraft:entity.dolphin.ambient_water"
                }
                VANILLA_ENTITY_TYPE_DOLPHIN_ID => "minecraft:entity.dolphin.ambient",
                // Panda.java:661-667.
                VANILLA_ENTITY_TYPE_PANDA_ID => {
                    let Some(state) = data.panda_retained() else {
                        return false;
                    };
                    if state.is_aggressive() {
                        "minecraft:entity.panda.aggressive_ambient"
                    } else if state.is_worried() {
                        "minecraft:entity.panda.worried_ambient"
                    } else {
                        "minecraft:entity.panda.ambient"
                    }
                }
                _ => return false,
            };
            let volume = match entity.entity_type_id() {
                VANILLA_ENTITY_TYPE_BAT_ID => super::ai::mob::bat::AMBIENT_VOLUME,
                VANILLA_ENTITY_TYPE_ALLAY_ID => 0.4,
                // HappyGhast.java:232-234.
                VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID if !baby => 4.0,
                // AbstractHorse.java:397-404.
                VANILLA_ENTITY_TYPE_HORSE_ID | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => 0.8,
                _ => 1.0,
            };
            (super::sound_variants::registered_sound(sound), volume)
        }
        Some(VanillaMobSpawnAmbientSound::Dynamic)
            if entity.entity_type_id() == VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID =>
        {
            (
                super::sound_variants::registered_sound(super::wandering_trader::ambient_sound(
                    entity,
                )),
                1.0,
            )
        }
        Some(VanillaMobSpawnAmbientSound::Dynamic)
            if matches!(
                entity.entity_type_id(),
                VANILLA_ENTITY_TYPE_CAT_ID
                    | VANILLA_ENTITY_TYPE_COW_ID
                    | VANILLA_ENTITY_TYPE_CHICKEN_ID
                    | VANILLA_ENTITY_TYPE_PIG_ID
                    | VANILLA_ENTITY_TYPE_WOLF_ID
            ) =>
        {
            (
                animal_ambient(entity, store, baby, random),
                if matches!(
                    entity.entity_type_id(),
                    VANILLA_ENTITY_TYPE_COW_ID | VANILLA_ENTITY_TYPE_WOLF_ID
                ) {
                    // AbstractCow.java:80-82; Wolf.java:294-296.
                    0.4
                } else {
                    1.0
                },
            )
        }
        _ => {
            let content_registry = store.plugins().content_registry();
            let Some(sound_id) = content_registry
                .entity_by_id(entity.entity_type_id())
                .and_then(|spec| spec.ambient_sound.as_deref())
            else {
                return false;
            };
            (
                super::sound_variants::registered_sound_with_registry(sound_id, content_registry),
                1.0,
            )
        }
    };
    let Some(sound) = sound else {
        return false;
    };
    if !entity.is_silent() {
        let pitch = selected_pitch.unwrap_or_else(|| voice_pitch(random, baby));
        store.emit_entity_sound(SoundEvent {
            sound,
            position: entity.position(),
            source: living::living_sound_source(entity),
            volume,
            pitch,
            seed: random.next_long(),
        });
    }
    true
}

fn animal_ambient(
    entity: &Entity,
    store: &EntityStore,
    baby: bool,
    random: &GameRandom,
) -> Option<SoundEventHolder> {
    // Cat.java:248-258.
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_CAT_ID {
        use super::cat_sounds::CatSound;
        let EntityData::GeneralLiving(data) = entity.data() else {
            return None;
        };
        let sound = if !data.is_tame() {
            CatSound::StrayAmbient
        } else if super::animal_breeding::is_in_love(entity) {
            CatSound::Purr
        } else if random.next_int(4) == 0 {
            CatSound::Purreow
        } else {
            CatSound::Ambient
        };
        return super::cat_sounds::resolve(store, entity, sound);
    }
    let name = super::general_living::sound_variant_resource_id(entity)?;
    let resolver = store.item_resolver();
    // Cow.java:99-101; Chicken.java:99-101; Pig.java:264-266.
    let sound = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_COW_ID => &resolver.cow_sound_variant(name)?.ambient_sound,
        VANILLA_ENTITY_TYPE_CHICKEN_ID => {
            let variant = resolver.chicken_sound_variant(name)?;
            let set = if baby {
                &variant.baby_sounds
            } else {
                &variant.adult_sounds
            };
            &set.ambient_sound
        }
        VANILLA_ENTITY_TYPE_PIG_ID => {
            let variant = resolver.pig_sound_variant(name)?;
            let set = if baby {
                &variant.baby_sounds
            } else {
                &variant.adult_sounds
            };
            &set.ambient_sound
        }
        // Wolf.java:273-281.
        VANILLA_ENTITY_TYPE_WOLF_ID => {
            let EntityData::GeneralLiving(data) = entity.data() else {
                return None;
            };
            let variant = resolver.wolf_sound_variant(name)?;
            let set = if baby {
                &variant.baby_sounds
            } else {
                &variant.adult_sounds
            };
            if super::ai::mob::target::anger::read(entity)?.is_angry(store.game_time()) {
                &set.growl_sound
            } else if random.next_int(3) == 0 {
                if data.is_tame() && entity.living()?.health() < 20.0 {
                    &set.whine_sound
                } else {
                    &set.pant_sound
                }
            } else {
                &set.ambient_sound
            }
        }
        _ => return None,
    };
    super::sound_variants::sound_holder(sound)
}
