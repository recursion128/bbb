//! Per-type `playStepSound` and `getSwimSound` overrides.

use bbb_protocol::block_sound_facts_26_1::vanilla_block_state_sound_type_26_1;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_LAVA_ID, VANILLA_BLOCK_SNOW_ID, VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_ARMADILLO_ID, VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID,
    VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COD_ID,
    VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_COW_ID,
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID, VANILLA_ENTITY_TYPE_DONKEY_ID,
    VANILLA_ENTITY_TYPE_DROWNED_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_GOAT_ID,
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
    VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_HUSK_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_MOOSHROOM_ID, VANILLA_ENTITY_TYPE_MULE_ID,
    VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_PARCHED_ID,
    VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
    VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
    VANILLA_ENTITY_TYPE_RAVAGER_ID, VANILLA_ENTITY_TYPE_SALMON_ID, VANILLA_ENTITY_TYPE_SHEEP_ID,
    VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, VANILLA_ENTITY_TYPE_SKELETON_ID,
    VANILLA_ENTITY_TYPE_SNIFFER_ID, VANILLA_ENTITY_TYPE_SPIDER_ID, VANILLA_ENTITY_TYPE_STRAY_ID,
    VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
    VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID, VANILLA_ENTITY_TYPE_TURTLE_ID,
    VANILLA_ENTITY_TYPE_WARDEN_ID, VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
    VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::packets::{BlockPos, SoundEventHolder};
use bbb_protocol::sound_event_types::VANILLA_SOUND_EVENT_RESOURCE_IDS;

use crate::entity::living::living_sound_source;
use crate::entity::sound_variants::{registered_sound, sound_holder};
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;

const fn const_str_eq(lhs: &str, rhs: &str) -> bool {
    let lhs = lhs.as_bytes();
    let rhs = rhs.as_bytes();
    if lhs.len() != rhs.len() {
        return false;
    }
    let mut index = 0;
    while index < lhs.len() {
        if lhs[index] != rhs[index] {
            return false;
        }
        index += 1;
    }
    true
}

const fn const_sound(resource_id: &str) -> SoundEventHolder {
    let mut index = 0;
    while index < VANILLA_SOUND_EVENT_RESOURCE_IDS.len() {
        if const_str_eq(VANILLA_SOUND_EVENT_RESOURCE_IDS[index], resource_id) {
            return SoundEventHolder::Reference {
                registry_id: index as i32,
            };
        }
        index += 1;
    }
    panic!("unknown vanilla sound event resource_id")
}

const SOUND_CAMEL_STEP: SoundEventHolder = const_sound("minecraft:entity.camel.step");
const SOUND_CAMEL_STEP_SAND: SoundEventHolder = const_sound("minecraft:entity.camel.step_sand");
const SOUND_CAMEL_HUSK_STEP: SoundEventHolder = const_sound("minecraft:entity.camel_husk.step");
const SOUND_CAMEL_HUSK_STEP_SAND: SoundEventHolder =
    const_sound("minecraft:entity.camel_husk.step_sand");
const SOUND_COPPER_GOLEM_STEP: SoundEventHolder = const_sound("minecraft:entity.copper_golem.step");
const SOUND_COPPER_GOLEM_WEATHERED_STEP: SoundEventHolder =
    const_sound("minecraft:entity.copper_golem_weathered.step");
const SOUND_COPPER_GOLEM_OXIDIZED_STEP: SoundEventHolder =
    const_sound("minecraft:entity.copper_golem_oxidized.step");
const SOUND_IRON_GOLEM_STEP: SoundEventHolder = const_sound("minecraft:entity.iron_golem.step");
const SOUND_STRIDER_STEP_LAVA: SoundEventHolder = const_sound("minecraft:entity.strider.step_lava");
const SOUND_STRIDER_STEP: SoundEventHolder = const_sound("minecraft:entity.strider.step");
const SOUND_WARDEN_STEP: SoundEventHolder = const_sound("minecraft:entity.warden.step");
const SOUND_ARMADILLO_STEP: SoundEventHolder = const_sound("minecraft:entity.armadillo.step");
const SOUND_COW_STEP: SoundEventHolder = const_sound("minecraft:entity.cow.step");
const SOUND_LLAMA_STEP: SoundEventHolder = const_sound("minecraft:entity.llama.step");
const SOUND_FROG_STEP: SoundEventHolder = const_sound("minecraft:entity.frog.step");
const SOUND_GOAT_STEP: SoundEventHolder = const_sound("minecraft:entity.goat.step");
const SOUND_PANDA_STEP: SoundEventHolder = const_sound("minecraft:entity.panda.step");
const SOUND_PARROT_STEP: SoundEventHolder = const_sound("minecraft:entity.parrot.step");
const SOUND_POLAR_BEAR_STEP: SoundEventHolder = const_sound("minecraft:entity.polar_bear.step");
const SOUND_SHEEP_STEP: SoundEventHolder = const_sound("minecraft:entity.sheep.step");
const SOUND_SNIFFER_STEP: SoundEventHolder = const_sound("minecraft:entity.sniffer.step");
const SOUND_TURTLE_SHAMBLE_BABY: SoundEventHolder =
    const_sound("minecraft:entity.turtle.shamble_baby");
const SOUND_TURTLE_SHAMBLE: SoundEventHolder = const_sound("minecraft:entity.turtle.shamble");
const SOUND_RAVAGER_STEP: SoundEventHolder = const_sound("minecraft:entity.ravager.step");
const SOUND_ZOGLIN_STEP: SoundEventHolder = const_sound("minecraft:entity.zoglin.step");
const SOUND_CREAKING_STEP: SoundEventHolder = const_sound("minecraft:entity.creaking.step");
const SOUND_HOGLIN_STEP: SoundEventHolder = const_sound("minecraft:entity.hoglin.step");
const SOUND_PIGLIN_STEP: SoundEventHolder = const_sound("minecraft:entity.piglin.step");
const SOUND_PIGLIN_BRUTE_STEP: SoundEventHolder = const_sound("minecraft:entity.piglin_brute.step");
const SOUND_SKELETON_STEP: SoundEventHolder = const_sound("minecraft:entity.skeleton.step");
const SOUND_STRAY_STEP: SoundEventHolder = const_sound("minecraft:entity.stray.step");
const SOUND_WITHER_SKELETON_STEP: SoundEventHolder =
    const_sound("minecraft:entity.wither_skeleton.step");
const SOUND_BOGGED_STEP: SoundEventHolder = const_sound("minecraft:entity.bogged.step");
const SOUND_PARCHED_STEP: SoundEventHolder = const_sound("minecraft:entity.parched.step");
const SOUND_SPIDER_STEP: SoundEventHolder = const_sound("minecraft:entity.spider.step");
const SOUND_ZOMBIE_STEP: SoundEventHolder = const_sound("minecraft:entity.zombie.step");
const SOUND_HUSK_STEP: SoundEventHolder = const_sound("minecraft:entity.husk.step");
const SOUND_DROWNED_STEP: SoundEventHolder = const_sound("minecraft:entity.drowned.step");
const SOUND_ZOMBIE_VILLAGER_STEP: SoundEventHolder =
    const_sound("minecraft:entity.zombie_villager.step");
const SOUND_HORSE_GALLOP: SoundEventHolder = const_sound("minecraft:entity.horse.gallop");
const SOUND_BABY_HORSE_BREATHE: SoundEventHolder =
    const_sound("minecraft:entity.baby_horse.breathe");
const SOUND_HORSE_BREATHE: SoundEventHolder = const_sound("minecraft:entity.horse.breathe");
const SOUND_HORSE_STEP_WOOD: SoundEventHolder = const_sound("minecraft:entity.horse.step_wood");
const SOUND_BABY_HORSE_STEP: SoundEventHolder = const_sound("minecraft:entity.baby_horse.step");
const SOUND_HORSE_STEP: SoundEventHolder = const_sound("minecraft:entity.horse.step");
const SOUND_PARROT_FLY: SoundEventHolder = const_sound("minecraft:entity.parrot.fly");
const SOUND_AMETHYST_CHIME: SoundEventHolder = const_sound("minecraft:block.amethyst_block.chime");
const SOUND_AXOLOTL_SWIM: SoundEventHolder = const_sound("minecraft:entity.axolotl.swim");
const SOUND_DOLPHIN_SWIM: SoundEventHolder = const_sound("minecraft:entity.dolphin.swim");
const SOUND_FISH_SWIM: SoundEventHolder = const_sound("minecraft:entity.fish.swim");
const SOUND_BABY_NAUTILUS_SWIM: SoundEventHolder =
    const_sound("minecraft:entity.baby_nautilus.swim");
const SOUND_NAUTILUS_SWIM: SoundEventHolder = const_sound("minecraft:entity.nautilus.swim");
const SOUND_ZOMBIE_NAUTILUS_SWIM: SoundEventHolder =
    const_sound("minecraft:entity.zombie_nautilus.swim");
const SOUND_TURTLE_SWIM: SoundEventHolder = const_sound("minecraft:entity.turtle.swim");
const SOUND_DROWNED_SWIM: SoundEventHolder = const_sound("minecraft:entity.drowned.swim");
const SOUND_HOSTILE_SWIM: SoundEventHolder = const_sound("minecraft:entity.hostile.swim");
const SOUND_PLAYER_SWIM: SoundEventHolder = const_sound("minecraft:entity.player.swim");
const SOUND_SKELETON_HORSE_GALLOP_WATER: SoundEventHolder =
    const_sound("minecraft:entity.skeleton_horse.gallop_water");
const SOUND_SKELETON_HORSE_SWIM: SoundEventHolder =
    const_sound("minecraft:entity.skeleton_horse.swim");
const SOUND_SKELETON_HORSE_STEP_WATER: SoundEventHolder =
    const_sound("minecraft:entity.skeleton_horse.step_water");
const SOUND_GENERIC_SWIM: SoundEventHolder = const_sound("minecraft:entity.generic.swim");

fn baby(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby())
}

fn play(
    entity: &Entity,
    store: &mut EntityStore,
    sound: Option<SoundEventHolder>,
    volume: f32,
    pitch: f32,
) {
    if let Some(sound) = sound {
        crate::entity::sound_variants::play(
            store,
            sound,
            entity.position(),
            living_sound_source(entity),
            volume,
            pitch,
        );
    }
}

/// `Entity.playStepSound` (`Entity.java:1375-1378`) and its overrides.
pub(super) fn play_step(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
) {
    if entity.is_silent() {
        return;
    }
    let baby = baby(entity);
    let sound = match entity.entity_type_id() {
        // Allay, Bee, AbstractFish, HappyGhast and AbstractNautilus step silently.
        VANILLA_ENTITY_TYPE_ALLAY_ID
        | VANILLA_ENTITY_TYPE_BEE_ID
        | VANILLA_ENTITY_TYPE_COD_ID
        | VANILLA_ENTITY_TYPE_SALMON_ID
        | VANILLA_ENTITY_TYPE_PUFFERFISH_ID
        | VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID
        | VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        | VANILLA_ENTITY_TYPE_NAUTILUS_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => return,
        // Chicken.java, AbstractCow.java:75-77, Pig.java, Wolf.java.
        VANILLA_ENTITY_TYPE_CHICKEN_ID
        | VANILLA_ENTITY_TYPE_COW_ID
        | VANILLA_ENTITY_TYPE_PIG_ID
        | VANILLA_ENTITY_TYPE_WOLF_ID => {
            let sound = variant_step_sound(entity, store, baby);
            return play(entity, store, sound, 0.15, 1.0);
        }
        VANILLA_ENTITY_TYPE_HORSE_ID
        | VANILLA_ENTITY_TYPE_DONKEY_ID
        | VANILLA_ENTITY_TYPE_MULE_ID
        | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => {
            return horse_step(entity, store, level, pos, state, baby)
        }
        // Camel.java, CamelHusk.java.
        id @ (VANILLA_ENTITY_TYPE_CAMEL_ID | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID) => {
            let sand = crate::entity::level::block_state_tag_membership(
                level,
                state,
                "minecraft:camel_sand_step_sound_blocks",
            );
            let (sound, volume) = match (id, sand) {
                (VANILLA_ENTITY_TYPE_CAMEL_ID, false) => (SOUND_CAMEL_STEP, 1.0),
                (VANILLA_ENTITY_TYPE_CAMEL_ID, true) => (SOUND_CAMEL_STEP_SAND, 1.0),
                (_, false) => (SOUND_CAMEL_HUSK_STEP, 0.4),
                (_, true) => (SOUND_CAMEL_HUSK_STEP_SAND, 0.4),
            };
            return play(entity, store, Some(sound), volume, 1.0);
        }
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => {
            return play(entity, store, Some(copper_golem_step(entity)), 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => {
            return play(entity, store, Some(SOUND_IRON_GOLEM_STEP), 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_STRIDER_ID => {
            let sound = if entity.is_in_lava() {
                SOUND_STRIDER_STEP_LAVA
            } else {
                SOUND_STRIDER_STEP
            };
            return play(entity, store, Some(sound), 1.0, 1.0);
        }
        VANILLA_ENTITY_TYPE_WARDEN_ID => {
            return play(entity, store, Some(SOUND_WARDEN_STEP), 10.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_ARMADILLO_ID => SOUND_ARMADILLO_STEP,
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID => SOUND_COW_STEP,
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => SOUND_LLAMA_STEP,
        VANILLA_ENTITY_TYPE_FROG_ID => SOUND_FROG_STEP,
        VANILLA_ENTITY_TYPE_GOAT_ID => SOUND_GOAT_STEP,
        VANILLA_ENTITY_TYPE_PANDA_ID => SOUND_PANDA_STEP,
        VANILLA_ENTITY_TYPE_PARROT_ID => SOUND_PARROT_STEP,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => SOUND_POLAR_BEAR_STEP,
        VANILLA_ENTITY_TYPE_SHEEP_ID => SOUND_SHEEP_STEP,
        VANILLA_ENTITY_TYPE_SNIFFER_ID => SOUND_SNIFFER_STEP,
        VANILLA_ENTITY_TYPE_TURTLE_ID if baby => SOUND_TURTLE_SHAMBLE_BABY,
        VANILLA_ENTITY_TYPE_TURTLE_ID => SOUND_TURTLE_SHAMBLE,
        VANILLA_ENTITY_TYPE_RAVAGER_ID => SOUND_RAVAGER_STEP,
        VANILLA_ENTITY_TYPE_ZOGLIN_ID => SOUND_ZOGLIN_STEP,
        VANILLA_ENTITY_TYPE_CREAKING_ID => SOUND_CREAKING_STEP,
        VANILLA_ENTITY_TYPE_HOGLIN_ID => SOUND_HOGLIN_STEP,
        VANILLA_ENTITY_TYPE_PIGLIN_ID => SOUND_PIGLIN_STEP,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => SOUND_PIGLIN_BRUTE_STEP,
        VANILLA_ENTITY_TYPE_SKELETON_ID => SOUND_SKELETON_STEP,
        VANILLA_ENTITY_TYPE_STRAY_ID => SOUND_STRAY_STEP,
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID => SOUND_WITHER_SKELETON_STEP,
        VANILLA_ENTITY_TYPE_BOGGED_ID => SOUND_BOGGED_STEP,
        VANILLA_ENTITY_TYPE_PARCHED_ID => SOUND_PARCHED_STEP,
        VANILLA_ENTITY_TYPE_SPIDER_ID | VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID => SOUND_SPIDER_STEP,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => {
            SOUND_ZOMBIE_STEP
        }
        VANILLA_ENTITY_TYPE_HUSK_ID => SOUND_HUSK_STEP,
        VANILLA_ENTITY_TYPE_DROWNED_ID => SOUND_DROWNED_STEP,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => SOUND_ZOMBIE_VILLAGER_STEP,
        _ => {
            let sound_type = vanilla_block_state_sound_type_26_1(state);
            return play(
                entity,
                store,
                registered_sound(sound_type.step_sound),
                sound_type.volume * 0.15,
                sound_type.pitch,
            );
        }
    };
    play(entity, store, Some(sound), 0.15, 1.0);
}

fn variant_step_sound(
    entity: &Entity,
    store: &EntityStore,
    baby: bool,
) -> Option<SoundEventHolder> {
    let name = crate::entity::general_living::sound_variant_resource_id(entity)?;
    let resolver = store.item_resolver();
    let sound = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_COW_ID => &resolver.cow_sound_variant(name)?.step_sound,
        VANILLA_ENTITY_TYPE_CHICKEN_ID => {
            let variant = resolver.chicken_sound_variant(name)?;
            if baby {
                &variant.baby_sounds.step_sound
            } else {
                &variant.adult_sounds.step_sound
            }
        }
        VANILLA_ENTITY_TYPE_PIG_ID => {
            let variant = resolver.pig_sound_variant(name)?;
            if baby {
                &variant.baby_sounds.step_sound
            } else {
                &variant.adult_sounds.step_sound
            }
        }
        _ => {
            let variant = resolver.wolf_sound_variant(name)?;
            if baby {
                &variant.baby_sounds.step_sound
            } else {
                &variant.adult_sounds.step_sound
            }
        }
    };
    sound_holder(sound)
}

// CopperGolemOxidationLevels.java.
fn copper_golem_step(entity: &Entity) -> SoundEventHolder {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return SOUND_COPPER_GOLEM_STEP;
    };
    match crate::entity::copper_golem::weather_state(data) {
        bbb_protocol::entity_synched_data::VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID => {
            SOUND_COPPER_GOLEM_WEATHERED_STEP
        }
        bbb_protocol::entity_synched_data::VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID => {
            SOUND_COPPER_GOLEM_OXIDIZED_STEP
        }
        _ => SOUND_COPPER_GOLEM_STEP,
    }
}

/// `AbstractHorse.playStepSound` (`AbstractHorse.java:342-363`) and
/// `Horse.playGallopSound` (`Horse.java:122-127`). The gallop cadence comes
/// from the walked distance instead of a stored step counter.
fn horse_step(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
    baby: bool,
) {
    if matches!(
        vanilla_block_state_block_type_id(state),
        Some(VANILLA_BLOCK_WATER_ID | VANILLA_BLOCK_LAVA_ID)
    ) {
        return;
    }
    let above = level.block_state(BlockPos {
        y: pos.y + 1,
        ..pos
    });
    let sound_type = vanilla_block_state_sound_type_26_1(
        if vanilla_block_state_block_type_id(above) == Some(VANILLA_BLOCK_SNOW_ID) {
            above
        } else {
            state
        },
    );
    let (volume, pitch) = (sound_type.volume * 0.15, sound_type.pitch);
    let can_gallop = !matches!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_DONKEY_ID | VANILLA_ENTITY_TYPE_MULE_ID
    );
    if can_gallop && !store.passengers(entity.id()).is_empty() {
        if (entity.move_dist as i32) % 3 != 0 {
            return;
        }
        play(entity, store, Some(SOUND_HORSE_GALLOP), volume, pitch);
        if entity.entity_type_id() == VANILLA_ENTITY_TYPE_HORSE_ID
            && store.random().next_int(10) == 0
        {
            let breathe = if baby {
                SOUND_BABY_HORSE_BREATHE
            } else {
                SOUND_HORSE_BREATHE
            };
            play(entity, store, Some(breathe), sound_type.volume * 0.6, pitch);
        }
        return;
    }
    let wood = matches!(
        sound_type.place_sound,
        "minecraft:block.wood.place"
            | "minecraft:block.nether_wood.place"
            | "minecraft:block.stem.place"
            | "minecraft:block.cherry_wood.place"
            | "minecraft:block.bamboo_wood.place"
    );
    let sound = if wood {
        SOUND_HORSE_STEP_WOOD
    } else if baby {
        SOUND_BABY_HORSE_STEP
    } else {
        SOUND_HORSE_STEP
    };
    play(entity, store, Some(sound), volume, pitch);
}

/// `Parrot.onFlap` (`Parrot.java:365-368`).
pub(super) fn play_flap(entity: &Entity, store: &mut EntityStore) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_PARROT_ID || entity.is_silent() {
        return;
    }
    // A hashed seed keeps every flap from drawing on the shared gameplay stream.
    store.emit_entity_sound(bbb_protocol::packets::SoundEvent {
        sound: SOUND_PARROT_FLY,
        position: entity.position(),
        source: living_sound_source(entity),
        volume: 0.15,
        pitch: 1.0,
        seed: entity.uuid().as_u64_pair().0 as i64 ^ i64::from(entity.fly_dist.to_bits()),
    });
}

/// `Entity.walkingStepSound`'s crystal chime (`Entity.java:1339-1344`,
/// `:1380-1391`).
pub(super) fn play_amethyst_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    state: i32,
) {
    let tick = entity.tick_count();
    if !entity.crystal_sound.ready(tick)
        || !crate::entity::level::block_state_tag_membership(
            level,
            state,
            "minecraft:crystal_sound_blocks",
        )
    {
        return;
    }
    let random = store.random().next_float();
    let (volume, pitch) = entity.crystal_sound.sound(tick, random);
    if !entity.is_silent() {
        play(entity, store, Some(SOUND_AMETHYST_CHIME), volume, pitch);
    }
}

/// `Entity.waterSwimSound`/`playSwimSound` (`Entity.java:1346-1356`,
/// `:1393-1395`) and the `getSwimSound`/`playSwimSound` overrides.
pub(super) fn play_swim(entity: &Entity, store: &mut EntityStore) {
    if entity.is_silent() {
        return;
    }
    let volume = super::body_swim_sound_volume(entity.delta_movement, false);
    let (sound, volume) = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => (SOUND_AXOLOTL_SWIM, volume),
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => (SOUND_DOLPHIN_SWIM, volume),
        VANILLA_ENTITY_TYPE_COD_ID
        | VANILLA_ENTITY_TYPE_SALMON_ID
        | VANILLA_ENTITY_TYPE_PUFFERFISH_ID
        | VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => (SOUND_FISH_SWIM, volume),
        VANILLA_ENTITY_TYPE_NAUTILUS_ID if baby(entity) => (SOUND_BABY_NAUTILUS_SWIM, volume),
        VANILLA_ENTITY_TYPE_NAUTILUS_ID => (SOUND_NAUTILUS_SWIM, volume),
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => (SOUND_ZOMBIE_NAUTILUS_SWIM, volume),
        // Turtle.java:178-180.
        VANILLA_ENTITY_TYPE_TURTLE_ID => (SOUND_TURTLE_SWIM, volume * 1.5),
        VANILLA_ENTITY_TYPE_DROWNED_ID => (SOUND_DROWNED_SWIM, volume),
        VANILLA_ENTITY_TYPE_HOGLIN_ID => (SOUND_HOSTILE_SWIM, volume),
        VANILLA_ENTITY_TYPE_PLAYER_ID => (SOUND_PLAYER_SWIM, volume),
        // SkeletonHorse.java:83-109. The ridden gallop cadence comes from the
        // walked distance instead of a stored counter.
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            if entity.on_ground() && !store.passengers(entity.id()).is_empty() =>
        {
            if (entity.move_dist as i32) % 3 == 0 {
                (SOUND_SKELETON_HORSE_GALLOP_WATER, 0.3)
            } else {
                (SOUND_SKELETON_HORSE_SWIM, 0.3)
            }
        }
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID if entity.on_ground() => {
            (SOUND_SKELETON_HORSE_STEP_WATER, 0.3)
        }
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => {
            (SOUND_SKELETON_HORSE_SWIM, (volume * 25.0).min(0.1))
        }
        _ if crate::entity::ai::mob::spec::is_instance_of(entity.entity_type_id(), "Monster") => {
            (SOUND_HOSTILE_SWIM, volume)
        }
        _ => (SOUND_GENERIC_SWIM, volume),
    };
    let random = store.random();
    let pitch = 1.0 + (random.next_float() - random.next_float()) * 0.4;
    play(entity, store, Some(sound), volume, pitch);
}
