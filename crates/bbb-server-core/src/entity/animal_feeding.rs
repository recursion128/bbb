//! `Animal.mobInteract`'s feeding branch (`Animal.java:139-163`), the one
//! production entry of the breeding chain: `Mob.interact` reaches it after
//! `Mob.checkAndHandleImportantInteractions` (`Mob.java:1121-1145`) and
//! `Entity.interact` (`Entity.java:2168-2242`) both PASSed (`Mob.java:1105-1114`).
//!
//! The food predicate is each subtype's `isFood` and the sound each subtype's
//! `playEatingSound`; [`FOOD_OWNERS`] carries both, plus the `canFallInLove`
//! and `usePlayerItem` terms a subtype adds, for the types whose `mobInteract`
//! leaves `Animal.mobInteract` reachable. [`age_lock`] owns the shared
//! `AgeableMob.mobInteract` tail, including ageable types without food entries.
//!
//! `Frog.isBaby` is overridden to a constant false with a no-op `setBaby`
//! (`Frog.java:250-256`), so `AgeableMob.canAgeUp` (`:210-212`) never answers
//! true for a frog: the growth branch is unreachable for it, and a frog whose
//! saved `Age` is negative takes neither branch (`:143` asks for an `age` of
//! exactly 0). The growth particle is the client's own
//! (`AgeableMob.aiStep`'s `forcedAgeTimer` arm, `:190-197`).

use crate::random::GameRandom;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMADILLO_ID, VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_BEE_ID,
    VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COW_ID,
    VANILLA_ENTITY_TYPE_FOX_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_GOAT_ID,
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
    VANILLA_ENTITY_TYPE_MOOSHROOM_ID, VANILLA_ENTITY_TYPE_OCELOT_ID, VANILLA_ENTITY_TYPE_PIG_ID,
    VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_RABBIT_ID, VANILLA_ENTITY_TYPE_SHEEP_ID,
    VANILLA_ENTITY_TYPE_SNIFFER_ID, VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TURTLE_ID,
    VANILLA_ENTITY_TYPE_WOLF_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_CLOSED_EYEBLOSSOM_ID, VANILLA_ITEM_OPEN_EYEBLOSSOM_ID,
    VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID, VANILLA_ITEM_WATER_BUCKET_ID,
    VANILLA_ITEM_WITHER_ROSE_ID,
};
use bbb_protocol::packets::{ItemStackSummary, SoundSource, Vec3d};
use uuid::Uuid;

use crate::entity::level::EntityRegistryTagView;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

mod age_lock;
mod ocelot;
mod tadpole;

/// The registry the food tags live in.
const ITEM_REGISTRY: &str = "minecraft:item";

/// `SoundEvents.FROG_EAT`, the sound `Frog.playEatingSound` plays
/// (`Frog.java:235-237`).
const FROG_EAT_SOUND: &str = "minecraft:entity.frog.eat";

/// `SoundEvents.FOX_EAT`, the sound `Fox.playEatingSound` plays
/// (`Fox.java:372-374`).
const FOX_EAT_SOUND: &str = "minecraft:entity.fox.eat";

/// `SoundEvents.GOAT_EAT` (`SoundEvents.java:730`) and
/// `SoundEvents.GOAT_SCREAMING_EAT` (`:739`).
const GOAT_EAT_SOUND: &str = "minecraft:entity.goat.eat";
const GOAT_SCREAMING_EAT_SOUND: &str = "minecraft:entity.goat.screaming.eat";

/// `SoundEvents.SNIFFER_EAT` (`SoundEvents.java:1527`).
const SNIFFER_EAT_SOUND: &str = "minecraft:entity.sniffer.eat";

/// `SoundEvents.PIG_EAT_BABY` (`SoundEvents.java:1250`), the `eat_sound` every
/// `pig_sound_variant`'s `baby_sounds` carries (`SoundEvents.java:1951-1953`).
const PIG_BABY_EAT_SOUND: &str = "minecraft:entity.baby_pig.eat";

/// `Animal.playEatingSound` (`Animal.java:165-166`) is empty; each override
/// this branch reaches names its own sound, source, volume and pitch stream.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EatingSound {
    /// The empty base body and every subtype that does not override it.
    Silent,
    /// A fixed-pitch sound using the override's Level overload.
    Fixed {
        resource_id: &'static str,
        source: SoundSource,
        volume: f32,
        pitch: f32,
        route: EatingSoundRoute,
    },
    /// `Pig.playEatingSound` (`Pig.java:147-149`): the sound variant's
    /// `eat_sound` through `LivingEntity.makeSound` (`LivingEntity.java:1406-1410`),
    /// whose volume is `getSoundVolume` (`:2280-2282`) and whose pitch is the
    /// two draws `getVoicePitch` takes on the pig's own `random` (`:2284-2288`).
    PigSoundSet,
    /// `Cat.playEatingSound` (`Cat.java:284-286`): the sound variant's
    /// `eat_sound` through `Entity.playSound` (`Entity.java:1404-1408`), at a
    /// fixed volume and pitch rather than a drawn one.
    CatSoundSet,
    /// A fixed sound through `LivingEntity.makeSound` (`LivingEntity.java:1406-1410`)
    /// with the inherited volume and voice pitch.
    Made(&'static str),
    /// `Strider.mobInteract` plays a guarded position sound after feeding
    /// (`Strider.java:408-420`), with a voice pitch that stays adult for babies.
    Strider,
    /// `level.playSound(null, this, ...)` (`Level.java:461-465`) with the pitch
    /// `Mth.randomBetween(level.getRandom(), 0.8F, 1.2F)` draws
    /// (`Mth.java:668-670`).
    FromEntity {
        resource_id: &'static str,
        /// `Goat.isScreamingGoat` picks this one instead (`Goat.java:198`).
        screaming_resource_id: Option<&'static str>,
        source: SoundSource,
        volume: f32,
    },
}

/// One subtype's `isFood` override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FoodPredicate {
    /// `itemStack.is(ItemTags.X)`.
    Tag(&'static str),
    /// `return false`.
    Never,
}

/// The `isFood`, `playEatingSound`, `canFallInLove`, `usePlayerItem` and
/// `canAgeUp` answers of one `Animal` subtype whose `mobInteract` reaches
/// `Animal.mobInteract`.
struct FoodOwner {
    entity_type_id: i32,
    food: FoodPredicate,
    sound: EatingSound,
    /// `Goat.mobInteract` (`Goat.java:220-225`) and `Sniffer.mobInteract`
    /// (`Sniffer.java:374-383`) play the eating sound a second time once
    /// `super.mobInteract` consumed the food.
    sound_after_super: bool,
    /// `false` only where `isBaby` is overridden to a constant false.
    can_age_up: bool,
    /// `Turtle.canFallInLove` adds `!this.hasEgg()` (`Turtle.java:204-206`).
    egg_blocks_love: bool,
    /// `Axolotl.usePlayerItem` hands back a water bucket instead of shrinking
    /// (`Axolotl.java:540-546`).
    bucket_swap: bool,
}

const fn owner(entity_type_id: i32, food: FoodPredicate, sound: EatingSound) -> FoodOwner {
    FoodOwner {
        entity_type_id,
        food,
        sound,
        sound_after_super: false,
        can_age_up: true,
        egg_blocks_love: false,
        bucket_swap: false,
    }
}

/// `ItemTags.COW_FOOD` (`AbstractCow.java:51-53`), which `MushroomCow` inherits:
/// its own `mobInteract` takes a bowl, shears or a brown variant's flower and
/// answers `super.mobInteract` for anything else (`MushroomCow.java:128`).
const COW_FOOD: FoodPredicate = FoodPredicate::Tag("minecraft:cow_food");

/// The food predicates of the types whose `Animal.mobInteract` is the body the
/// click reaches, and the override that leaves it reachable otherwise:
/// `Sheep.mobInteract` shears and else `super` (`Sheep.java:140-151`),
/// `AbstractCow.mobInteract` milks a bucket and else `super`
/// (`AbstractCow.java:91-97`), `Strider.mobInteract` rides and else `super`
/// (`Strider.java:393-402`), `Hoglin.mobInteract` always `super`
/// (`Hoglin.java:204-211`), `MushroomCow.mobInteract` falls through for
/// anything but a bowl, shears or a brown variant's flower
/// (`MushroomCow.java:128`), `Pig.mobInteract` rides only when the held item is
/// not food (`Pig.java:157-174`), `Goat.mobInteract` milks a bucket first
/// (`Goat.java:211-226`), `Sniffer.mobInteract` is `super` plus a second sound
/// (`Sniffer.java:374-383`) and `Axolotl.mobInteract` only intercepts a water
/// bucket (`Axolotl.java:424-426` over `Bucketable.bucketMobPickup`,
/// `Bucketable.java:71-91`), while chicken, fox, frog, polar bear, rabbit and
/// turtle have no override at all.
///
/// `Cat.mobInteract` and `Wolf.mobInteract` are here too, because `taming.rs`
/// owns every arm they put in front of `super.mobInteract`
/// (`Cat.java:399-417`, `Wolf.java:448-490`) and hands the click on.
///
/// `Ocelot.mobInteract`'s trust arm runs first (`ocelot.rs`). Types the
/// branch deliberately does not answer for: panda, camel, horse and parrot
/// each have a `mobInteract` arm that consumes before the food branch with no
/// owner here.
static FOOD_OWNERS: &[FoodOwner] = &[
    // `Armadillo.java:306-313` with `Armadillo.playEatingSound` (`:345-347`).
    owner(
        VANILLA_ENTITY_TYPE_ARMADILLO_ID,
        FoodPredicate::Tag("minecraft:armadillo_food"),
        EatingSound::Made("minecraft:entity.armadillo.eat"),
    ),
    // `Axolotl.java:355-357` with `Axolotl.usePlayerItem` (`:540-546`).
    FoodOwner {
        bucket_swap: true,
        ..owner(
            VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            FoodPredicate::Tag("minecraft:axolotl_food"),
            EatingSound::Silent,
        )
    },
    // `Bee.isFood` (`Bee.java:584-586`); special flowers intercept below.
    owner(
        VANILLA_ENTITY_TYPE_BEE_ID,
        FoodPredicate::Tag("minecraft:bee_food"),
        EatingSound::Silent,
    ),
    // `Cat.java:447-449` with `Cat.playEatingSound` (`Cat.java:284-286`). The
    // arms `Cat.mobInteract` puts in front of `super` -- taming, the collar dye
    // and the healing feed -- are `taming.rs`'s.
    owner(
        VANILLA_ENTITY_TYPE_CAT_ID,
        FoodPredicate::Tag("minecraft:cat_food"),
        EatingSound::CatSoundSet,
    ),
    // `Wolf.java:538-540`; `Wolf` does not override `playEatingSound`.
    owner(
        VANILLA_ENTITY_TYPE_WOLF_ID,
        FoodPredicate::Tag("minecraft:wolf_food"),
        EatingSound::Silent,
    ),
    // `Chicken.java:189-191`.
    owner(
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        FoodPredicate::Tag("minecraft:chicken_food"),
        EatingSound::Silent,
    ),
    // `AbstractCow.java:51-53` for both cow types.
    owner(VANILLA_ENTITY_TYPE_COW_ID, COW_FOOD, EatingSound::Silent),
    owner(
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
        COW_FOOD,
        EatingSound::Silent,
    ),
    // `Fox.java:586-588` with `Fox.playEatingSound` (`:371-374`).
    owner(
        VANILLA_ENTITY_TYPE_FOX_ID,
        FoodPredicate::Tag("minecraft:fox_food"),
        EatingSound::Fixed {
            resource_id: FOX_EAT_SOUND,
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
            route: EatingSoundRoute::GuardedPosition,
        },
    ),
    // `Frog.java:333-335` with `Frog.playEatingSound` (`:234-237`) and the
    // `isBaby` override (`:249-256`).
    FoodOwner {
        can_age_up: false,
        ..owner(
            VANILLA_ENTITY_TYPE_FROG_ID,
            FoodPredicate::Tag("minecraft:frog_food"),
            EatingSound::Fixed {
                resource_id: FROG_EAT_SOUND,
                source: SoundSource::Neutral,
                volume: 2.0,
                pitch: 1.0,
                route: EatingSoundRoute::DirectEntity,
            },
        )
    },
    // `Goat.java:206-208` with `Goat.playEatingSound` (`:193-203`).
    FoodOwner {
        sound_after_super: true,
        ..owner(
            VANILLA_ENTITY_TYPE_GOAT_ID,
            FoodPredicate::Tag("minecraft:goat_food"),
            EatingSound::FromEntity {
                resource_id: GOAT_EAT_SOUND,
                screaming_resource_id: Some(GOAT_SCREAMING_EAT_SOUND),
                source: SoundSource::Neutral,
                volume: 1.0,
            },
        )
    },
    // `HappyGhast.java:257-259`; adults reach it via `super.mobInteract` (`:273-290`).
    owner(
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        FoodPredicate::Tag("minecraft:happy_ghast_food"),
        EatingSound::Silent,
    ),
    // `Hoglin.java:245-247`.
    owner(
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        FoodPredicate::Tag("minecraft:hoglin_food"),
        EatingSound::Silent,
    ),
    // `Ocelot.java:221-223`.
    owner(
        VANILLA_ENTITY_TYPE_OCELOT_ID,
        FoodPredicate::Tag("minecraft:ocelot_food"),
        EatingSound::Silent,
    ),
    // `Pig.java:239-241` with `Pig.playEatingSound` (`:147-149`).
    owner(
        VANILLA_ENTITY_TYPE_PIG_ID,
        FoodPredicate::Tag("minecraft:pig_food"),
        EatingSound::PigSoundSet,
    ),
    // `PolarBear.java:75-77`.
    owner(
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
        FoodPredicate::Never,
        EatingSound::Silent,
    ),
    // `Rabbit.java:359-361`.
    owner(
        VANILLA_ENTITY_TYPE_RABBIT_ID,
        FoodPredicate::Tag("minecraft:rabbit_food"),
        EatingSound::Silent,
    ),
    // `Sheep.java:80-82`.
    owner(
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        FoodPredicate::Tag("minecraft:sheep_food"),
        EatingSound::Silent,
    ),
    // `Sniffer.java:437-439` with `Sniffer.playEatingSound` (`:386-388`).
    FoodOwner {
        sound_after_super: true,
        ..owner(
            VANILLA_ENTITY_TYPE_SNIFFER_ID,
            FoodPredicate::Tag("minecraft:sniffer_food"),
            EatingSound::FromEntity {
                resource_id: SNIFFER_EAT_SOUND,
                screaming_resource_id: None,
                source: SoundSource::Neutral,
                volume: 1.0,
            },
        )
    },
    // `Strider.java:388-390` with the eating sound in `mobInteract` (`:408-420`).
    owner(
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        FoodPredicate::Tag("minecraft:strider_food"),
        EatingSound::Strider,
    ),
    // `Turtle.java:229-231` with `Turtle.canFallInLove` (`:204-206`).
    FoodOwner {
        egg_blocks_love: true,
        ..owner(
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            FoodPredicate::Tag("minecraft:turtle_food"),
            EatingSound::Silent,
        )
    },
];

/// The interacting session as the feeding branch reads it.
pub struct FeedingPlayer<'a> {
    /// `Player.getItemInHand(hand)`, which `usePlayerItem` shrinks.
    pub hand_stack: &'a mut ItemStackSummary,
    /// The `ServerPlayer` `Animal.setInLove` records as the love cause.
    pub uuid: Uuid,
    /// `Player.hasInfiniteMaterials()` (`Player.java:1293-1295`), which
    /// `ItemStack.consume` exempts from the shrink (`ItemStack.java:1082-1086`).
    pub infinite_materials: bool,
}

/// What `Animal.mobInteract` answered on the server. The client-only
/// `InteractionResult.CONSUME` (`Animal.java:157-159`) is unreachable here.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AnimalFeeding {
    /// `super.mobInteract` (`:162`) answered PASS: the item was not food, no
    /// branch applied, and the golden dandelion arm did not apply either.
    Pass,
    /// `SUCCESS_SERVER` (`:147`): an adult took the 600-tick love timer.
    FellInLove,
    /// `SUCCESS` (`:154`): a baby took the growth.
    GrewUp,
    /// A subtype arm before the food branch consumed the item: a bee's special
    /// flower or an ocelot's trust roll.
    Fed,
    /// The axolotl's filled-result offer, after the hand and age state changed.
    FedWithRemainder {
        server_swing: bool,
        item: ItemStackSummary,
    },
    /// `AgeableMob.mobInteract`'s `SUCCESS` (`AgeableMob.java:70-72`): a golden
    /// dandelion flipped `AgeLocked`.
    AgeLockToggled,
    /// A term of the branch cannot be answered: this level has no registry-tag
    /// generation, or the eating sound has no stream to draw its pitch from.
    /// The branch refuses rather than guess.
    Refused,
}

fn food_owner(entity_type_id: i32) -> Option<&'static FoodOwner> {
    FOOD_OWNERS
        .iter()
        .find(|owner| owner.entity_type_id == entity_type_id)
}

impl FoodPredicate {
    /// `Animal.isFood`'s answer, or `None` when the tag generation cannot
    /// answer it. An empty stack resolves to the air item, which no food tag
    /// names.
    fn matches(self, item_id: Option<i32>, tags: &EntityRegistryTagView) -> bool {
        let tag = match self {
            Self::Never => return false,
            Self::Tag(tag) => tag,
        };
        let Some(item) = item_id.and_then(bbb_protocol::item_types::vanilla_item_type) else {
            return false;
        };
        tags.contains(ITEM_REGISTRY, tag, item.resource_id)
    }
}

/// `ItemStack.consume` without `Mob.usePlayerItem` component side effects.
pub(crate) fn use_player_item_for_taming(player: &mut FeedingPlayer<'_>) {
    let _ = use_player_item(player, false);
}

/// Records the `Mob.usePlayerItem` branch for the world's native-stack remainder.
pub(super) fn use_mob_player_item(
    player: &mut FeedingPlayer<'_>,
    mob_item_used: &mut bool,
) -> Option<ItemStackSummary> {
    *mob_item_used = true;
    use_player_item(player, false)
}

pub(super) fn use_player_item(
    player: &mut FeedingPlayer<'_>,
    bucket_swap: bool,
) -> Option<ItemStackSummary> {
    if bucket_swap && player.hand_stack.item_id == Some(VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID) {
        return use_player_bucket_food(player);
    }
    if player.infinite_materials {
        return None;
    }
    player.hand_stack.count -= 1;
    if player.hand_stack.count <= 0 {
        *player.hand_stack = ItemStackSummary::empty();
    }
    None
}

pub(super) fn use_player_bucket_food(player: &mut FeedingPlayer<'_>) -> Option<ItemStackSummary> {
    let water_bucket = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_WATER_BUCKET_ID),
        count: 1,
        component_patch: Default::default(),
    };
    if player.infinite_materials {
        return Some(water_bucket);
    }
    player.hand_stack.count -= 1;
    if player.hand_stack.count <= 0 {
        *player.hand_stack = water_bucket;
        return None;
    }
    Some(water_bucket)
}

/// `AgeableMob.getSpeedUpSecondsWhenFeeding` (`AgeableMob.java:250-252`): the
/// integer division happens before the float multiply.
pub(crate) fn speed_up_seconds_when_feeding(ticks_until_adult: i32) -> i32 {
    ((ticks_until_adult / 20) as f32 * 0.1) as i32
}

/// One resolved `playEatingSound` call.
struct EatingSoundCall {
    resource_id: &'static str,
    source: SoundSource,
    volume: f32,
    pitch: EatingPitch,
    route: EatingSoundRoute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EatingSoundRoute {
    /// `Entity.playSound` checks Silent before calling the position overload.
    GuardedPosition,
    /// `Level.playSound(null, this, ...)` bypasses that guard.
    DirectEntity,
}

enum EatingPitch {
    Fixed(f32),
    /// `LivingEntity.getVoicePitch` (`LivingEntity.java:2284-2288`): two draws
    /// on the entity's own `random` around `base`.
    Voice {
        base: f32,
    },
    /// `Mth.randomBetween(level.getRandom(), min, max)` (`Mth.java:668-670`).
    LevelBetween {
        min: f32,
        max: f32,
    },
}

/// `Pig.getSoundSet().eatSound()` for an adult (`Pig.java:264-266`), whose
/// `eat_sound` is `entity.<sound event identifier>.eat`
/// (`SoundEvents.java:1943-1950`, `PigSoundVariants.java:37-56`).
fn pig_adult_eat_sound(sound_variant_resource_id: &str) -> Option<&'static str> {
    match sound_variant_resource_id {
        "minecraft:classic" => Some("minecraft:entity.pig.eat"),
        "minecraft:mini" => Some("minecraft:entity.pig_mini.eat"),
        "minecraft:big" => Some("minecraft:entity.pig_big.eat"),
        _ => None,
    }
}

fn is_baby(target: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = target.data() else {
        return false;
    };
    data.ageable_retained()
        .is_some_and(|ageable| ageable.is_baby())
}

fn is_screaming_goat(target: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = target.data() else {
        return false;
    };
    data.goat_retained().is_some_and(|goat| goat.is_screaming())
}

/// The sound the override plays for this body right now, read after the feeding
/// wrote the age: `Pig.getSoundSet` asks `isBaby` (`Pig.java:264-266`) and
/// `playEatingSound` runs after `ageUp` (`Animal.java:152-153`).
fn eating_sound_call(target: &Entity, sound: EatingSound) -> Option<EatingSoundCall> {
    match sound {
        EatingSound::Silent => None,
        EatingSound::Fixed {
            resource_id,
            source,
            volume,
            pitch,
            route,
        } => Some(EatingSoundCall {
            resource_id,
            source,
            volume,
            pitch: EatingPitch::Fixed(pitch),
            route,
        }),
        EatingSound::PigSoundSet => {
            let baby = is_baby(target);
            let resource_id = if baby {
                PIG_BABY_EAT_SOUND
            } else {
                pig_adult_eat_sound(crate::entity::general_living::sound_variant_resource_id(
                    target,
                )?)?
            };
            Some(EatingSoundCall {
                resource_id,
                // `Entity.getSoundSource()` (`Entity.java:3548-3550`).
                source: SoundSource::Neutral,
                volume: 1.0,
                pitch: EatingPitch::Voice {
                    base: if baby { 1.5 } else { 1.0 },
                },
                route: EatingSoundRoute::GuardedPosition,
            })
        }
        EatingSound::CatSoundSet => None,
        EatingSound::Made(resource_id) => Some(EatingSoundCall {
            resource_id,
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: EatingPitch::Voice {
                base: if is_baby(target) { 1.5 } else { 1.0 },
            },
            route: EatingSoundRoute::GuardedPosition,
        }),
        EatingSound::Strider => Some(EatingSoundCall {
            resource_id: "minecraft:entity.strider.eat",
            // Inherited `Entity.getSoundSource` (`Entity.java:3548-3550`).
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: EatingPitch::Voice { base: 1.0 },
            route: EatingSoundRoute::GuardedPosition,
        }),
        EatingSound::FromEntity {
            resource_id,
            screaming_resource_id,
            source,
            volume,
        } => Some(EatingSoundCall {
            resource_id: screaming_resource_id
                .filter(|_| is_screaming_goat(target))
                .unwrap_or(resource_id),
            source,
            volume,
            pitch: EatingPitch::LevelBetween { min: 0.8, max: 1.2 },
            route: EatingSoundRoute::DirectEntity,
        }),
    }
}

/// Whether the facts the override's sound reads off the body are there. The
/// feeding commits nothing when they are not, so this runs before the consume;
/// the pitch stream itself is checked beside it.
fn eating_sound_facts_available(
    store: &EntityStore,
    target: &Entity,
    sound: EatingSound,
    level_random_present: bool,
) -> bool {
    match sound {
        EatingSound::Silent
        | EatingSound::Fixed { .. }
        | EatingSound::Made(_)
        | EatingSound::Strider => true,
        EatingSound::PigSoundSet => {
            crate::entity::general_living::sound_variant_resource_id(target)
                .is_some_and(|variant| pig_adult_eat_sound(variant).is_some())
        }
        EatingSound::CatSoundSet => {
            super::cat_sounds::resolve(store, target, super::cat_sounds::CatSound::Eat).is_some()
        }
        EatingSound::FromEntity { .. } => level_random_present,
    }
}

fn play_eating_sound(
    store: &mut EntityStore,
    target_id: i32,
    position: Vec3d,
    call: &EatingSoundCall,
    silent: bool,
    level_random: Option<&GameRandom>,
) {
    if silent && call.route == EatingSoundRoute::GuardedPosition {
        return;
    }
    let pitch = match call.pitch {
        EatingPitch::Fixed(pitch) => pitch,
        EatingPitch::Voice { base } => {
            let random = store.random();
            let first = random.next_float();
            let second = random.next_float();
            (first - second) * 0.2 + base
        }
        EatingPitch::LevelBetween { min, max } => {
            let Some(random) = level_random else {
                return;
            };
            random.next_float() * (max - min) + min
        }
    };
    if call.route == EatingSoundRoute::DirectEntity {
        store.play_sound_from_entity(target_id, call.resource_id, call.source, call.volume, pitch);
        return;
    }
    store.play_entity_sound(PendingEntitySound {
        position,
        resource_id: call.resource_id.to_owned(),
        source: call.source,
        volume: call.volume,
        pitch,
    });
}

/// `Animal.mobInteract` (`Animal.java:139-163`) for one stored target the world
/// already range-checked, with the love write left to
/// [`crate::entity::animal_breeding::set_in_love`] and the `super.mobInteract` tail to
/// [`age_lock::mob_interact`].
pub(crate) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
    level_random: Option<&GameRandom>,
    mob_item_used: &mut bool,
    growth: Option<&crate::entity::living::dimensions_growth::SizeGrowthCollisionView<'_>>,
) -> AnimalFeeding {
    let outcome = animal_mob_interact(
        store,
        target_id,
        player,
        tags,
        level_random,
        mob_item_used,
        growth,
    );
    if !matches!(outcome, AnimalFeeding::Pass | AnimalFeeding::Refused) {
        // `Hoglin.mobInteract` (`Hoglin.java:204-211`).
        if let Some(target) = store
            .get_mut(target_id)
            .filter(|target| target.entity_type_id() == VANILLA_ENTITY_TYPE_HOGLIN_ID)
        {
            crate::entity::conversion::set_persistence_required(target);
        }
    }
    outcome
}

fn animal_mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
    level_random: Option<&GameRandom>,
    mob_item_used: &mut bool,
    growth: Option<&crate::entity::living::dimensions_growth::SizeGrowthCollisionView<'_>>,
) -> AnimalFeeding {
    let Some(target) = store.get(target_id) else {
        return AnimalFeeding::Pass;
    };
    if crate::entity::armadillo::is_scared(target) {
        return AnimalFeeding::Refused;
    }
    let EntityData::GeneralLiving(_) = target.data() else {
        return AnimalFeeding::Pass;
    };
    if target.entity_type_id() == VANILLA_ENTITY_TYPE_OCELOT_ID {
        if let Some(fed) = ocelot::mob_interact(store, target_id, player, tags, mob_item_used) {
            return fed;
        }
    }
    let Some(target) = store.get(target_id) else {
        return AnimalFeeding::Pass;
    };
    let EntityData::GeneralLiving(data) = target.data() else {
        return AnimalFeeding::Pass;
    };
    if data.tadpole_retained().is_some() {
        return tadpole::mob_interact(store, target_id, player, tags)
            .unwrap_or(AnimalFeeding::Pass);
    }
    let Some(owner) = food_owner(target.entity_type_id()) else {
        return super_mob_interact(store, target_id, player, tags);
    };
    let is_food = owner.food.matches(player.hand_stack.item_id, tags);
    if !is_food {
        return super_mob_interact(store, target_id, player, tags);
    }
    // `Bee.mobInteract` intercepts flower effects before `Animal.mobInteract`.
    if target.entity_type_id() == VANILLA_ENTITY_TYPE_BEE_ID {
        let effect = match player.hand_stack.item_id {
            // `WitherRoseBlock.java:79-80`, `EyeblossomBlock.java:114-115`.
            Some(VANILLA_ITEM_WITHER_ROSE_ID) => Some(("minecraft:wither", 40)),
            Some(VANILLA_ITEM_OPEN_EYEBLOSSOM_ID | VANILLA_ITEM_CLOSED_EYEBLOSSOM_ID) => {
                Some(("minecraft:poison", 25))
            }
            _ => None,
        };
        if let Some((resource_id, duration)) = effect {
            let Some(mut target) = store.detach(target_id) else {
                return AnimalFeeding::Pass;
            };
            let _ = use_mob_player_item(player, mob_item_used);
            use crate::entity::living::mob_effects as effects;
            if effects::can_be_affected_with_registry_tag_view(tags, "minecraft:bee", resource_id) {
                let effect = crate::entity::living::ActiveMobEffect::new(resource_id, duration, 0)
                    .expect("a built-in flower effect");
                if let Some(published) = target
                    .living_mut()
                    .and_then(|living| living.add_active_effect_published(effect))
                {
                    effects::send_effect_update(store, target_id, &published);
                }
            }
            store.attach(target);
            return AnimalFeeding::Fed;
        }
    }
    let Some(ageable) = data.ageable_retained() else {
        return AnimalFeeding::Pass;
    };
    // `int age = this.getAge()` (`:142`).
    let age = ageable.age();
    // `player instanceof ServerPlayer && age == 0 && this.canFallInLove()`
    // (`:143`).
    let falls_in_love = age == 0
        && data.animal_retained().is_some()
        && crate::entity::animal_breeding::can_fall_in_love(target)
        && !(owner.egg_blocks_love && has_egg(target));
    // `if (this.canAgeUp())` (`:150`), which `Frog.isBaby` makes unreachable
    // for a frog (`Frog.java:250-252`).
    let grows_up = owner.can_age_up && ageable.can_age_up();
    let sound_facts_available =
        eating_sound_facts_available(store, target, owner.sound, level_random.is_some());
    if !falls_in_love && !grows_up {
        return super_mob_interact(store, target_id, player, tags);
    }
    if !sound_facts_available {
        return AnimalFeeding::Refused;
    }
    let Some(mut target) = store.detach(target_id) else {
        return AnimalFeeding::Pass;
    };
    let paid = if falls_in_love {
        // `this.setInLove(serverPlayer)` (`:145`).
        crate::entity::animal_breeding::set_in_love(&mut target, store, Some(player.uuid))
    } else {
        // `this.ageUp(getSpeedUpSecondsWhenFeeding(-age), true)` (`:152`), whose
        // forced bookkeeping is what a second feeding reads back.
        set_age_up(&mut target, age, growth)
    };
    if !paid {
        store.attach(target);
        return AnimalFeeding::Refused;
    }
    // `this.usePlayerItem(player, hand, itemStack)` (`:144`, `:151`).
    let bucket_swap = owner.bucket_swap
        && player.hand_stack.item_id == Some(VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID);
    let remainder = if bucket_swap {
        use_player_bucket_food(player)
    } else {
        use_mob_player_item(player, mob_item_used)
    };
    let position = target.position();
    store.attach(target);
    // `this.playEatingSound()` (`:146`, `:153`), and the second call a goat's
    // and a sniffer's own `mobInteract` adds once `super` consumed
    // (`Goat.java:220-225`, `Sniffer.java:374-383`).
    let calls = usize::from(owner.sound_after_super) + 1;
    for _ in 0..calls {
        if owner.sound == EatingSound::CatSoundSet {
            let Some(sound) = store
                .get(target_id)
                .filter(|target| !target.is_silent())
                .and_then(|target| {
                    super::cat_sounds::resolve(store, target, super::cat_sounds::CatSound::Eat)
                })
            else {
                break;
            };
            super::sound_variants::play(store, sound, position, SoundSource::Neutral, 1.0, 1.0);
            continue;
        }
        let Some((call, silent)) = store.get(target_id).and_then(|target| {
            eating_sound_call(target, owner.sound).map(|call| (call, target.is_silent()))
        }) else {
            break;
        };
        play_eating_sound(store, target_id, position, &call, silent, level_random);
    }
    if let Some(item) = remainder {
        AnimalFeeding::FedWithRemainder {
            server_swing: falls_in_love,
            item,
        }
    } else if falls_in_love {
        AnimalFeeding::FellInLove
    } else {
        AnimalFeeding::GrewUp
    }
}

/// `return super.mobInteract(player, hand)` (`Animal.java:162`).
pub(super) fn super_mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
) -> AnimalFeeding {
    age_lock::mob_interact(store, target_id, player, tags)
}

fn has_egg(target: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = target.data() else {
        return false;
    };
    data.turtle_retained()
        .is_some_and(crate::entity::turtle::TurtleRetainedState::has_egg)
}

/// `Animal.mobInteract`'s growth write, whose baby-flag flip owes the
/// `DATA_BABY_ID` row and the box rebuilt from it (`AgeableMob.setAge`,
/// `AgeableMob.java:149-156`).
fn set_age_up(
    entity: &mut Entity,
    age: i32,
    growth: Option<&crate::entity::living::dimensions_growth::SizeGrowthCollisionView<'_>>,
) -> bool {
    age_up(
        entity,
        speed_up_seconds_when_feeding(age.wrapping_neg()),
        true,
        growth,
    )
}

/// `AgeableMob.ageUp(seconds, forced)` (`AgeableMob.java:123-142`).
pub(crate) fn age_up(
    entity: &mut Entity,
    seconds: i32,
    forced: bool,
    growth: Option<&crate::entity::living::dimensions_growth::SizeGrowthCollisionView<'_>>,
) -> bool {
    let previous_ageable = match entity.data() {
        EntityData::GeneralLiving(data) => match data.ageable_retained() {
            Some(state) => state,
            None => return false,
        },
        _ => return false,
    };
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => match data.ageable_retained_mut() {
            Some(state) => state.age_up(seconds, forced),
            None => return false,
        },
        _ => return false,
    };
    if !changed {
        return true;
    }
    let previous_dims = [entity.bb_width(), entity.bb_height()];
    let previous_eye = entity.eye_height();
    crate::entity::living::refresh_dimensions(entity);
    let resolved = match growth {
        Some(view) => crate::entity::living::dimensions_growth::resolve_with_collision_view(
            entity,
            previous_dims,
            view,
        )
        .is_some(),
        None => {
            !crate::entity::living::dimensions_growth::requires_collision(entity, previous_dims)
        }
    };
    if resolved {
        entity.mark_synched_dirty(
            bbb_protocol::entity_synched_data::VANILLA_AGEABLE_MOB_BABY_DATA_ID,
        );
        crate::entity::goat::age_boundary_reached(entity);
        true
    } else {
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            if let Some(state) = data.ageable_retained_mut() {
                *state = previous_ageable;
            }
        }
        entity.set_dimensions_and_eye(previous_dims[0], previous_dims[1], previous_eye);
        false
    }
}

#[cfg(test)]
mod tests;
