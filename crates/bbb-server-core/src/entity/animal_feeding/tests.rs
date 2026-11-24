use std::sync::OnceLock;

use crate::random::GameRandom;
use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_COW_ID,
    VANILLA_ENTITY_TYPE_FOX_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_GOAT_ID,
    VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
    VANILLA_ENTITY_TYPE_TURTLE_ID,
};
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::{ItemStackSummary, SoundEventHolder, SoundSource, Vec3d};
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use uuid::Uuid;

use super::*;
use crate::entity::leash::{self, EntityInteractionLoot, LeashInteraction};
use crate::entity::level::EntityRegistryTagView;
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent};

/// The interacting session's UUID, which `Animal.setInLove` records as the love
/// cause (`Animal.java:174-176`).
const PLAYER_UUID: Uuid = Uuid::from_u128(0xfeed);

/// The embedded vanilla datapack, which the dispatch's sheep arm needs before
/// it can answer `Pass` for a non-sheep target.
fn datapacks() -> &'static DatapackStartup {
    static STARTUP: OnceLock<DatapackStartup> = OnceLock::new();
    STARTUP.get_or_init(|| {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .expect("the embedded vanilla datapack loads")
    })
}

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn stack(resource_id: &str, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(
            vanilla_item_type_for_resource_id(resource_id)
                .unwrap_or_else(|| panic!("{resource_id} is a built-in item"))
                .id,
        ),
        count,
        component_patch: Default::default(),
    }
}

/// One stored `Animal` of `entity_type_id` at [`position`].
fn animal(store: &mut EntityStore, entity_type_id: i32) -> i32 {
    store
        .spawn_general_living(entity_type_id, position())
        .expect("a stored animal")
}

fn set_age(store: &mut EntityStore, id: i32, age: i32) {
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).expect("the animal").data else {
        panic!("a general living animal")
    };
    assert!(data
        .ageable_retained_mut()
        .expect("an ageable animal")
        .set_age(age));
}

fn ageable(store: &EntityStore, id: i32) -> crate::entity::general_living::AgeableMobRetainedState {
    let EntityData::GeneralLiving(data) = store.get(id).expect("the animal").data() else {
        panic!("a general living animal")
    };
    data.ageable_retained().expect("an ageable animal")
}

fn animal_state(
    store: &EntityStore,
    id: i32,
) -> crate::entity::general_living::AnimalRetainedState {
    let EntityData::GeneralLiving(data) = store.get(id).expect("the animal").data() else {
        panic!("a general living animal")
    };
    data.animal_retained().expect("an animal")
}

fn set_in_love_time(store: &mut EntityStore, id: i32, in_love: i32) {
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).expect("the animal").data else {
        panic!("a general living animal")
    };
    data.animal_retained_mut()
        .expect("an animal")
        .set_in_love(in_love, Some(Uuid::from_u128(9)));
}

/// The leash seam's own dispatch, which is the entry the world's
/// `Player.interactOn` path takes (`Player.java:834`).
fn interact(
    store: &mut EntityStore,
    target_id: i32,
    hand_stack: &mut ItemStackSummary,
    tags: &EntityRegistryTagView,
) -> LeashInteraction {
    interact_full(store, target_id, hand_stack, tags, false, None)
}

fn interact_as(
    store: &mut EntityStore,
    target_id: i32,
    hand_stack: &mut ItemStackSummary,
    tags: &EntityRegistryTagView,
    infinite_materials: bool,
) -> LeashInteraction {
    interact_full(store, target_id, hand_stack, tags, infinite_materials, None)
}

/// The same seam with `Level.random` lent in, which the goat's and the
/// sniffer's `playEatingSound` draw their pitch from.
fn interact_with_level_random(
    store: &mut EntityStore,
    target_id: i32,
    hand_stack: &mut ItemStackSummary,
    level_random: &GameRandom,
) -> LeashInteraction {
    interact_full(
        store,
        target_id,
        hand_stack,
        &EntityRegistryTagView::Builtin26_1,
        false,
        Some(level_random),
    )
}

fn interact_full(
    store: &mut EntityStore,
    target_id: i32,
    hand_stack: &mut ItemStackSummary,
    tags: &EntityRegistryTagView,
    infinite_materials: bool,
    level_random: Option<&GameRandom>,
) -> LeashInteraction {
    store.set_item_publication(crate::entity::ItemPublication::builtin().clone());
    leash::interact_leash_with_loot(
        store,
        target_id,
        &mut leash::LeashInteractingPlayer {
            mob_item_used: false,
            hand_stack,
            entity_id: 1,
            uuid: PLAYER_UUID,
            position: position(),
            secondary_use_active: false,
            infinite_materials,
        },
        EntityInteractionLoot {
            dimension: None,
            interacting_entity: crate::loot_runtime::LootEntity::Absent,
            weather: crate::loot_runtime::LootWeather {
                raining: false,
                thundering: false,
            },
            tables: crate::entity::block_drops::BlockDropTables {
                tables: Some(&datapacks().loot),
                recipes: None,
                damage_type_tags: None,
            },
            level: None,
            block_entity_data: None,
            item_environment: None,
            enchantment_registry: &[],
            registry_tags: tags,
            level_random,
            growth: None,
            spawn_egg: None,
        },
    )
}

/// What the store queued for one interaction, split by the `Level.playSound`
/// overload each call took.
#[derive(Default)]
struct FeedingEvents {
    /// `(entity id, event id)`.
    broadcasts: Vec<(i32, i8)>,
    /// `Level.playSound(null, x, y, z, ...)` (`Level.java:435-445`), as
    /// `(sound, source, position, volume, pitch)`.
    sounds: Vec<(String, SoundSource, Vec3d, f32, f32)>,
    /// `Level.playSound(null, Entity, ...)` (`Level.java:461-465`), as
    /// `(sound, source, entity id, volume, pitch)`.
    entity_sounds: Vec<(String, SoundSource, i32, f32, f32)>,
}

fn sound_resource_id(registry_id: i32) -> String {
    bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(registry_id)
        .expect("a built-in sound")
        .to_string()
}

fn events(store: &mut EntityStore) -> FeedingEvents {
    let mut collected = FeedingEvents::default();
    for event in store.drain_lifecycle_events() {
        match event {
            EntityLifecycleEvent::Broadcast {
                entity_id,
                packet: crate::connection::ClientboundPlayPacket::EntityEvent(packet),
            } => collected.broadcasts.push((entity_id, packet.event_id)),
            EntityLifecycleEvent::Sound {
                sound:
                    bbb_protocol::packets::SoundEvent {
                        sound: SoundEventHolder::Reference { registry_id },
                        source,
                        position,
                        volume,
                        pitch,
                        ..
                    },
            } => collected.sounds.push((
                sound_resource_id(registry_id),
                source,
                position,
                volume,
                pitch,
            )),
            EntityLifecycleEvent::SoundFromEntity {
                sound:
                    bbb_protocol::packets::SoundEntityEvent {
                        sound: SoundEventHolder::Reference { registry_id },
                        source,
                        entity_id,
                        volume,
                        pitch,
                        ..
                    },
                ..
            } => collected.entity_sounds.push((
                sound_resource_id(registry_id),
                source,
                entity_id,
                volume,
                pitch,
            )),
            _ => {}
        }
    }
    collected
}

#[test]
fn an_adult_frog_fed_its_food_falls_in_love_and_eats_it() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let frog = animal(&mut store, VANILLA_ENTITY_TYPE_FROG_ID);
    let mut hand = stack("minecraft:slime_ball", 2);

    let outcome = interact(
        &mut store,
        frog,
        &mut hand,
        &EntityRegistryTagView::Builtin26_1,
    );

    assert_eq!(outcome, LeashInteraction::FellInLove);
    // `this.inLove = 600` and the `ServerPlayer` love cause (`Animal.java:173-176`).
    assert_eq!(animal_state(&store, frog).in_love(), 600);
    assert_eq!(animal_state(&store, frog).love_cause(), Some(PLAYER_UUID));
    // `this.usePlayerItem` (`:144`) over `ItemStack.consume(1, player)`.
    assert_eq!(hand.count, 1);
    assert_eq!(hand.item_id, stack("minecraft:slime_ball", 1).item_id);
    let events = events(&mut store);
    // `this.level().broadcastEntityEvent(this, (byte)18)` (`:178`) with
    // `Frog.playEatingSound`'s `FROG_EAT` (`Frog.java:235-236`).
    assert_eq!(events.broadcasts, vec![(frog, 18)]);
    assert!(events.sounds.is_empty());
    assert_eq!(
        events.entity_sounds,
        vec![(
            "minecraft:entity.frog.eat".to_string(),
            SoundSource::Neutral,
            frog,
            2.0,
            1.0
        )]
    );
}

#[test]
fn feeding_the_last_item_leaves_an_empty_hand() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let frog = animal(&mut store, VANILLA_ENTITY_TYPE_FROG_ID);
    let mut hand = stack("minecraft:slime_ball", 1);

    assert_eq!(
        interact(
            &mut store,
            frog,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::FellInLove
    );

    assert_eq!(hand, ItemStackSummary::empty());
}

#[test]
fn a_creative_player_feeds_without_spending_the_item() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let frog = animal(&mut store, VANILLA_ENTITY_TYPE_FROG_ID);
    let mut hand = stack("minecraft:slime_ball", 2);

    let outcome = interact_as(
        &mut store,
        frog,
        &mut hand,
        &EntityRegistryTagView::Builtin26_1,
        true,
    );

    // `ItemStack.consume` skips the shrink for `hasInfiniteMaterials`
    // (`ItemStack.java:1082-1086`); `Player.interactOn` (`:836-838`) is what
    // restores a shrink that ran anyway.
    assert_eq!(outcome, LeashInteraction::FellInLove);
    assert_eq!(hand.count, 2);
}

#[test]
fn a_non_food_item_leaves_an_animal_alone() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let frog = animal(&mut store, VANILLA_ENTITY_TYPE_FROG_ID);
    let mut hand = stack("minecraft:stone", 1);

    assert_eq!(
        interact(
            &mut store,
            frog,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::Pass
    );

    assert!(!animal_state(&store, frog).is_in_love());
    assert_eq!(hand.count, 1);
    let events = events(&mut store);
    assert!(events.broadcasts.is_empty() && events.sounds.is_empty());
}

#[test]
fn an_animal_already_in_love_eats_nothing() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let frog = animal(&mut store, VANILLA_ENTITY_TYPE_FROG_ID);
    set_in_love_time(&mut store, frog, 600);
    let mut hand = stack("minecraft:slime_ball", 1);

    // `this.canFallInLove()` is false (`Animal.java:143`, `:168-170`), and a
    // frog's `age` is 0 rather than negative, so neither branch applies.
    assert_eq!(
        interact(
            &mut store,
            frog,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::Pass
    );

    assert_eq!(animal_state(&store, frog).in_love(), 600);
    assert_eq!(hand.count, 1);
    let events = events(&mut store);
    assert!(events.broadcasts.is_empty() && events.sounds.is_empty());
}

#[test]
fn a_baby_fed_its_food_grows_by_the_speed_up_seconds() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let cow = animal(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    set_age(&mut store, cow, -24_000);
    let mut hand = stack("minecraft:wheat", 1);

    let outcome = interact(
        &mut store,
        cow,
        &mut hand,
        &EntityRegistryTagView::Builtin26_1,
    );

    // `SUCCESS`: `this.ageUp(getSpeedUpSecondsWhenFeeding(-age), true)`
    // (`Animal.java:150-155`) with `-age` = 24000, whose
    // `(int)(ticksUntilAdult / 20 * 0.1F)` is 120 seconds (`AgeableMob.java:250-252`).
    assert_eq!(outcome, LeashInteraction::GrewUp);
    assert_eq!(ageable(&store, cow).age(), -24_000 + 120 * 20);
    // `this.forcedAge += delta` (`:134`), which the next feeding's own
    // `delta` and the reapply at `:140-142` read.
    assert_eq!(ageable(&store, cow).forced_age(), 2_400);
    assert_eq!(hand, ItemStackSummary::empty());
    // `Animal.playEatingSound` is empty and a cow does not override it
    // (`Animal.java:165-166`).
    let events = events(&mut store);
    assert!(events.broadcasts.is_empty() && events.sounds.is_empty());
    assert!(!animal_state(&store, cow).is_in_love());
}

#[test]
fn a_baby_happy_ghast_eats_a_snowball_and_grows() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let ghast = animal(
        &mut store,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    );
    set_age(&mut store, ghast, -24_000);
    let mut hand = stack("minecraft:snowball", 1);
    let outcome = interact(
        &mut store,
        ghast,
        &mut hand,
        &EntityRegistryTagView::Builtin26_1,
    );
    assert_eq!(outcome, LeashInteraction::GrewUp);
    assert_eq!(ageable(&store, ghast).age(), -24_000 + 120 * 20);
    assert_eq!(hand, ItemStackSummary::empty());
}

#[test]
fn a_frog_with_a_negative_age_takes_neither_branch() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let frog = animal(&mut store, VANILLA_ENTITY_TYPE_FROG_ID);
    set_age(&mut store, frog, -24_000);
    let mut hand = stack("minecraft:slime_ball", 1);

    // `Frog.isBaby` is a constant false (`Frog.java:250-252`), so `canAgeUp`
    // (`AgeableMob.java:210-212`) is false, and `:143` asks for an age of
    // exactly 0 for the love branch.
    assert_eq!(
        interact(
            &mut store,
            frog,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::Pass
    );

    assert_eq!(ageable(&store, frog).age(), -24_000);
    assert_eq!(hand.count, 1);
}

#[test]
fn the_food_predicate_reads_the_generation_it_was_handed() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let frog = animal(&mut store, VANILLA_ENTITY_TYPE_FROG_ID);
    // A generation that takes slime balls out of `#minecraft:frog_food`.
    let snapshot = RegistryTagSnapshot::new([(
        "minecraft:item".to_string(),
        "minecraft:frog_food".to_string(),
        Vec::new(),
    )]);
    let tags = EntityRegistryTagView::Frozen(std::sync::Arc::new(snapshot));
    assert!(!tags.contains(
        "minecraft:item",
        "minecraft:frog_food",
        "minecraft:slime_ball"
    ));
    let mut hand = stack("minecraft:slime_ball", 1);

    assert_eq!(
        interact(&mut store, frog, &mut hand, &tags),
        LeashInteraction::Pass
    );

    assert!(!animal_state(&store, frog).is_in_love());
    assert_eq!(hand.count, 1);
}

/// `Mth.randomBetween(random, 0.8F, 1.2F)` (`Mth.java:668-670`), the pitch a
/// goat's and a sniffer's `playEatingSound` draws.
fn eating_pitch(pitch: f32) -> bool {
    (0.8..=1.2).contains(&pitch)
}

/// `LivingEntity.getVoicePitch` (`LivingEntity.java:2284-2288`).
fn voice_pitch(pitch: f32, baby: bool) -> bool {
    let base = if baby { 1.5 } else { 1.0 };
    (base - 0.2..=base + 0.2).contains(&pitch)
}

fn set_cat_sound_variant(store: &mut EntityStore, id: i32) {
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).expect("the cat").data else {
        panic!("a general living cat")
    };
    data.apply_saved_variant_fields(&[bbb_protocol::NbtSummaryEntry {
        name: "sound_variant".into(),
        value: bbb_protocol::NbtSummaryValue::String("minecraft:classic".into()),
    }]);
}

#[test]
fn silent_animals_keep_feeding_state_without_position_sound() {
    for (entity_type, food) in [
        (VANILLA_ENTITY_TYPE_FOX_ID, "minecraft:sweet_berries"),
        (VANILLA_ENTITY_TYPE_CAT_ID, "minecraft:cod"),
        (VANILLA_ENTITY_TYPE_PIG_ID, "minecraft:carrot"),
        (VANILLA_ENTITY_TYPE_STRIDER_ID, "minecraft:warped_fungus"),
    ] {
        let mut audible = EntityStore::new(7, EntityIdCounter::new());
        let mut silent = EntityStore::new(7, EntityIdCounter::new());
        let audible_id = animal(&mut audible, entity_type);
        let silent_id = animal(&mut silent, entity_type);
        if entity_type == VANILLA_ENTITY_TYPE_CAT_ID {
            for (store, id) in [(&mut audible, audible_id), (&mut silent, silent_id)] {
                let mut target = store.detach(id).unwrap();
                assert!(crate::entity::taming::set_tame_offspring(
                    &mut target,
                    Some(PLAYER_UUID)
                ));
                store.attach(target);
                set_cat_sound_variant(store, id);
                store.drain_lifecycle_events();
            }
        }
        if entity_type == VANILLA_ENTITY_TYPE_PIG_ID {
            set_age(&mut audible, audible_id, -24_000);
            set_age(&mut silent, silent_id, -24_000);
        }
        silent.get_mut(silent_id).unwrap().set_silent(true);
        let mut audible_hand = stack(food, 2);
        let mut silent_hand = stack(food, 2);
        let audible_outcome = interact(
            &mut audible,
            audible_id,
            &mut audible_hand,
            &EntityRegistryTagView::Builtin26_1,
        );
        let silent_outcome = interact(
            &mut silent,
            silent_id,
            &mut silent_hand,
            &EntityRegistryTagView::Builtin26_1,
        );
        assert_eq!(silent_outcome, audible_outcome);
        assert!(matches!(
            audible_outcome,
            LeashInteraction::FellInLove | LeashInteraction::GrewUp
        ));
        assert_eq!(silent_hand, audible_hand);
        assert_eq!(
            ageable(&silent, silent_id).age(),
            ageable(&audible, audible_id).age()
        );
        assert_eq!(
            animal_state(&silent, silent_id).in_love(),
            animal_state(&audible, audible_id).in_love()
        );
        let audible_events = events(&mut audible);
        let silent_events = events(&mut silent);
        assert_eq!(audible_events.sounds.len(), 1);
        assert!(audible_events.entity_sounds.is_empty());
        assert!(silent_events.sounds.is_empty() && silent_events.entity_sounds.is_empty());
        assert_eq!(silent_events.broadcasts, audible_events.broadcasts);
    }
}

#[test]
fn silent_frog_goat_and_sniffer_keep_direct_entity_sounds() {
    for (entity_type, food, count) in [
        (VANILLA_ENTITY_TYPE_FROG_ID, "minecraft:slime_ball", 1),
        (VANILLA_ENTITY_TYPE_GOAT_ID, "minecraft:wheat", 2),
        (
            VANILLA_ENTITY_TYPE_SNIFFER_ID,
            "minecraft:torchflower_seeds",
            2,
        ),
    ] {
        let mut store = EntityStore::new(7, EntityIdCounter::new());
        let id = animal(&mut store, entity_type);
        store.get_mut(id).unwrap().set_silent(true);
        let mut hand = stack(food, 1);
        let level_random = GameRandom::with_seed(23);
        assert_eq!(
            interact_with_level_random(&mut store, id, &mut hand, &level_random),
            LeashInteraction::FellInLove
        );
        let events = events(&mut store);
        assert!(events.sounds.is_empty());
        assert_eq!(events.entity_sounds.len(), count);
        assert!(events.entity_sounds.iter().all(|sound| sound.2 == id));
        if entity_type == VANILLA_ENTITY_TYPE_FROG_ID {
            assert_eq!(
                events.entity_sounds[0],
                (FROG_EAT_SOUND.into(), SoundSource::Neutral, id, 2.0, 1.0)
            );
        } else {
            assert!(events
                .entity_sounds
                .iter()
                .all(|sound| eating_pitch(sound.4)));
        }
        assert_eq!(hand, ItemStackSummary::empty());
        assert_eq!(animal_state(&store, id).in_love(), 600);
    }
}

fn set_turtle_egg(store: &mut EntityStore, id: i32) {
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).expect("the turtle").data else {
        panic!("a general living turtle")
    };
    data.turtle_retained_mut()
        .expect("turtle state")
        .set_has_egg(true);
}

fn age_locked(store: &EntityStore, id: i32) -> bool {
    ageable(store, id).age_locked()
}

fn persistence_required(store: &EntityStore, id: i32) -> bool {
    let EntityData::GeneralLiving(data) = store.get(id).expect("the animal").data() else {
        panic!("a general living animal")
    };
    data.mob_retained()
        .expect("mob state")
        .persistence_required()
}

/// `AgeableMob.makeAgeLockedParticle`'s countdown (`AgeableMob.java:214-228`)
/// run from `aiStep` (`:207`) for `AGE_LOCK_COOLDOWN_TICKS` ticks (`:32`).
fn run_age_lock_cooldown(store: &mut EntityStore, id: i32) {
    for _ in 0..40 {
        let EntityData::GeneralLiving(data) = &mut store.get_mut(id).expect("the animal").data
        else {
            panic!("a general living animal")
        };
        data.ageable_retained_mut()
            .expect("an ageable animal")
            .tick_age_lock_particle_timer();
    }
}

#[test]
fn an_adult_pig_fed_its_food_plays_its_sound_sets_eat_sound() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let pig = animal(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    let mut hand = stack("minecraft:carrot", 2);

    let outcome = interact(
        &mut store,
        pig,
        &mut hand,
        &EntityRegistryTagView::Builtin26_1,
    );

    assert_eq!(outcome, LeashInteraction::FellInLove);
    assert_eq!(hand.count, 1);
    let events = events(&mut store);
    // `Pig.playEatingSound` (`Pig.java:147-149`) over the `classic` sound
    // variant's adult `eat_sound` (`SoundEvents.java:1943-1950`), at the
    // target's coordinates through `Entity.playSound` (`Entity.java:1404-1408`).
    let [(sound, source, at, volume, pitch)] = &events.sounds[..] else {
        panic!("one eat sound: {:?}", events.sounds)
    };
    assert_eq!(
        (sound.as_str(), *source, *at, *volume),
        (
            "minecraft:entity.pig.eat",
            SoundSource::Neutral,
            position(),
            1.0
        )
    );
    assert!(voice_pitch(*pitch, false));
    assert!(events.entity_sounds.is_empty());
}

#[test]
fn a_baby_pig_fed_its_food_keeps_the_baby_eat_sound_and_voice() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let pig = animal(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    set_age(&mut store, pig, -24_000);
    let mut hand = stack("minecraft:potato", 1);

    let outcome = interact(
        &mut store,
        pig,
        &mut hand,
        &EntityRegistryTagView::Builtin26_1,
    );

    // `playEatingSound` runs after `ageUp` (`Animal.java:152-153`), and 120
    // seconds of growth leave the pig a baby, so `Pig.getSoundSet`
    // (`Pig.java:264-266`) still answers `baby_sounds`.
    assert_eq!(outcome, LeashInteraction::GrewUp);
    assert_eq!(ageable(&store, pig).age(), -24_000 + 120 * 20);
    let events = events(&mut store);
    let [(sound, _, _, _, pitch)] = &events.sounds[..] else {
        panic!("one eat sound: {:?}", events.sounds)
    };
    assert_eq!(sound, "minecraft:entity.baby_pig.eat");
    assert!(voice_pitch(*pitch, true));
}

#[test]
fn a_goat_fed_its_food_plays_the_eating_sound_twice_off_the_level_random() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let goat = animal(&mut store, VANILLA_ENTITY_TYPE_GOAT_ID);
    let level_random = GameRandom::with_seed(99);
    let mut hand = stack("minecraft:wheat", 1);

    let outcome = interact_with_level_random(&mut store, goat, &mut hand, &level_random);

    assert_eq!(outcome, LeashInteraction::FellInLove);
    assert_eq!(hand, ItemStackSummary::empty());
    let events = events(&mut store);
    // `Animal.mobInteract` plays it once (`Animal.java:146`) and
    // `Goat.mobInteract` again once `super` consumed (`Goat.java:220-225`),
    // each through `level.playSound(null, this, ...)` (`Goat.java:193-203`).
    assert!(events.sounds.is_empty());
    assert_eq!(events.entity_sounds.len(), 2);
    assert!(events
        .entity_sounds
        .iter()
        .all(|(sound, source, id, volume, pitch)| {
            sound == "minecraft:entity.goat.eat"
                && *source == SoundSource::Neutral
                && *id == goat
                && *volume == 1.0
                && eating_pitch(*pitch)
        }));
}

#[test]
fn a_screaming_goat_fed_its_food_plays_the_screaming_eat_sound() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let goat = animal(&mut store, VANILLA_ENTITY_TYPE_GOAT_ID);
    assert!(crate::entity::goat::apply_spawn_traits(
        store.get_mut(goat).expect("the goat"),
        true,
        crate::entity::goat::SpawnHornRemoval::None
    ));
    let level_random = GameRandom::with_seed(3);
    let mut hand = stack("minecraft:wheat", 1);

    assert_eq!(
        interact_with_level_random(&mut store, goat, &mut hand, &level_random),
        LeashInteraction::FellInLove
    );

    // `this.isScreamingGoat() ? GOAT_SCREAMING_EAT : GOAT_EAT` (`Goat.java:198`).
    let events = events(&mut store);
    assert_eq!(events.entity_sounds.len(), 2);
    assert!(events
        .entity_sounds
        .iter()
        .all(|sound| sound.0 == "minecraft:entity.goat.screaming.eat"));
}

#[test]
fn a_sniffer_fed_its_food_plays_the_eating_sound_twice() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let sniffer = animal(&mut store, VANILLA_ENTITY_TYPE_SNIFFER_ID);
    let level_random = GameRandom::with_seed(11);
    let mut hand = stack("minecraft:torchflower_seeds", 1);

    let outcome = interact_with_level_random(&mut store, sniffer, &mut hand, &level_random);

    // `Sniffer.mobInteract` adds the second call after `super` consumed
    // (`Sniffer.java:374-383`, `:385-388`).
    assert_eq!(outcome, LeashInteraction::FellInLove);
    let events = events(&mut store);
    assert_eq!(events.entity_sounds.len(), 2);
    assert!(events
        .entity_sounds
        .iter()
        .all(|(sound, source, id, volume, pitch)| {
            sound == "minecraft:entity.sniffer.eat"
                && *source == SoundSource::Neutral
                && *id == sniffer
                && *volume == 1.0
                && eating_pitch(*pitch)
        }));
}

#[test]
fn an_axolotl_fed_a_tropical_fish_bucket_is_handed_a_water_bucket() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let axolotl = animal(&mut store, VANILLA_ENTITY_TYPE_AXOLOTL_ID);
    let mut hand = stack("minecraft:tropical_fish_bucket", 1);

    let outcome = interact(
        &mut store,
        axolotl,
        &mut hand,
        &EntityRegistryTagView::Builtin26_1,
    );

    // `Axolotl.usePlayerItem` (`Axolotl.java:540-546`) replaces an emptied
    // stack with the water bucket (`ItemUtils.java:27-32`).
    assert_eq!(outcome, LeashInteraction::FellInLove);
    assert_eq!(animal_state(&store, axolotl).in_love(), 600);
    assert_eq!(hand, stack("minecraft:water_bucket", 1));
}

#[test]
fn a_creative_axolotl_feeding_keeps_the_tropical_fish_bucket() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let axolotl = animal(&mut store, VANILLA_ENTITY_TYPE_AXOLOTL_ID);
    let mut hand = stack("minecraft:tropical_fish_bucket", 1);

    let outcome = interact_as(
        &mut store,
        axolotl,
        &mut hand,
        &EntityRegistryTagView::Builtin26_1,
        true,
    );

    // `createFilledResult`'s creative arm returns the held stack untouched
    // (`ItemUtils.java:19-25`).
    assert_eq!(
        outcome,
        LeashInteraction::FedWithRemainder {
            server_swing: true,
            item: stack("minecraft:water_bucket", 1),
        }
    );
    assert_eq!(hand, stack("minecraft:tropical_fish_bucket", 1));
}

#[test]
fn a_stacked_axolotl_bucket_offers_the_remainder_after_one_is_consumed() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let axolotl = animal(&mut store, VANILLA_ENTITY_TYPE_AXOLOTL_ID);
    let mut hand = stack("minecraft:tropical_fish_bucket", 2);
    hand.component_patch.max_stack_size = Some(16);
    hand.component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID);
    let mut expected_hand = hand.clone();
    expected_hand.count = 1;

    let outcome = interact(
        &mut store,
        axolotl,
        &mut hand,
        &EntityRegistryTagView::Builtin26_1,
    );

    assert_eq!(
        outcome,
        LeashInteraction::FedWithRemainder {
            server_swing: true,
            item: stack("minecraft:water_bucket", 1),
        }
    );
    assert_eq!(hand, expected_hand);
}

#[test]
fn a_turtle_carrying_an_egg_does_not_fall_in_love() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let turtle = animal(&mut store, VANILLA_ENTITY_TYPE_TURTLE_ID);
    set_turtle_egg(&mut store, turtle);
    let mut hand = stack("minecraft:seagrass", 1);

    // `Turtle.canFallInLove` adds `!this.hasEgg()` (`Turtle.java:204-206`), and
    // an adult turtle has no growth branch either (`Animal.java:150`).
    assert_eq!(
        interact(
            &mut store,
            turtle,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::Pass
    );

    assert!(!animal_state(&store, turtle).is_in_love());
    assert_eq!(hand.count, 1);
}

#[test]
fn a_turtle_without_an_egg_falls_in_love() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let turtle = animal(&mut store, VANILLA_ENTITY_TYPE_TURTLE_ID);
    let mut hand = stack("minecraft:seagrass", 1);

    assert_eq!(
        interact(
            &mut store,
            turtle,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::FellInLove
    );

    assert_eq!(animal_state(&store, turtle).in_love(), 600);
    assert_eq!(hand, ItemStackSummary::empty());
}

#[test]
fn a_golden_dandelion_locks_a_baby_animals_age() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let cow = animal(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    set_age(&mut store, cow, -2_400);
    let mut hand = stack("minecraft:golden_dandelion", 2);

    let outcome = interact(
        &mut store,
        cow,
        &mut hand,
        &EntityRegistryTagView::Builtin26_1,
    );

    // `AgeableMob.setAgeLockedData` (`AgeableMob.java:82-86`) and
    // `setAgeLocked` (`:88-102`).
    assert_eq!(outcome, LeashInteraction::AgeLockToggled);
    assert!(age_locked(&store, cow));
    assert_eq!(ageable(&store, cow).age(), -24_000);
    assert_eq!(ageable(&store, cow).age_lock_particle_timer(), 40);
    assert!(persistence_required(&store, cow));
    assert_eq!(hand.count, 1);
    let events = events(&mut store);
    // `mob.level().playSound(null, mob.blockPosition(), ...)` (`:98-101`) over
    // the block centre (`Level.java:389-392`).
    assert_eq!(
        events.sounds,
        vec![(
            "minecraft:item.golden_dandelion.use".to_string(),
            SoundSource::Players,
            Vec3d {
                x: 0.5,
                y: 64.5,
                z: 0.5
            },
            1.0,
            1.0
        )]
    );
}

#[test]
fn a_sniffer_age_lock_uses_its_own_baby_start_age() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let sniffer = animal(&mut store, VANILLA_ENTITY_TYPE_SNIFFER_ID);
    set_age(&mut store, sniffer, -100);
    let mut hand = stack("minecraft:golden_dandelion", 1);

    assert_eq!(
        interact(
            &mut store,
            sniffer,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::AgeLockToggled
    );

    // `Sniffer.getBabyStartAge` (`Sniffer.java:117-119`).
    assert_eq!(ageable(&store, sniffer).age(), -48_000);
    assert_eq!(hand, ItemStackSummary::empty());
}

#[test]
fn a_second_golden_dandelion_inside_the_cooldown_does_nothing() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let cow = animal(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    set_age(&mut store, cow, -2_400);
    let mut hand = stack("minecraft:golden_dandelion", 2);
    assert_eq!(
        interact(
            &mut store,
            cow,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::AgeLockToggled
    );
    let _ = events(&mut store);

    // `cooldown == 0` (`AgeableMob.java:78-80`).
    assert_eq!(
        interact(
            &mut store,
            cow,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::Pass
    );

    assert!(age_locked(&store, cow));
    assert_eq!(hand.count, 1);
    let events = events(&mut store);
    assert!(events.sounds.is_empty());
}

#[test]
fn a_golden_dandelion_after_the_cooldown_unlocks_the_age() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let cow = animal(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    set_age(&mut store, cow, -2_400);
    let mut hand = stack("minecraft:golden_dandelion", 2);
    assert_eq!(
        interact(
            &mut store,
            cow,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::AgeLockToggled
    );
    let _ = events(&mut store);
    run_age_lock_cooldown(&mut store, cow);

    assert_eq!(
        interact(
            &mut store,
            cow,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::AgeLockToggled
    );

    // The flip is `!this.isAgeLocked()` (`AgeableMob.java:83`), and the
    // unlocked arm plays `GOLDEN_DANDELION_UNUSE` (`:98-101`).
    assert!(!age_locked(&store, cow));
    assert_eq!(hand, ItemStackSummary::empty());
    let events = events(&mut store);
    assert_eq!(events.sounds.len(), 1);
    assert_eq!(events.sounds[0].0, "minecraft:item.golden_dandelion.unuse");
}

#[test]
fn an_adult_animal_takes_no_golden_dandelion() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let cow = animal(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    let mut hand = stack("minecraft:golden_dandelion", 1);

    // `isBaby` (`AgeableMob.java:78-80`).
    assert_eq!(
        interact(
            &mut store,
            cow,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::Pass
    );

    assert!(!age_locked(&store, cow));
    assert_eq!(hand.count, 1);
}

#[test]
fn bee_special_flowers_apply_their_effect_without_love_or_growth() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID;
    for (flower, effect, duration, age) in [
        ("minecraft:wither_rose", "minecraft:wither", 40, 0),
        ("minecraft:open_eyeblossom", "minecraft:poison", 25, -24000),
        (
            "minecraft:closed_eyeblossom",
            "minecraft:poison",
            25,
            -24000,
        ),
    ] {
        let mut store = EntityStore::new(7, EntityIdCounter::new());
        let bee = animal(&mut store, VANILLA_ENTITY_TYPE_BEE_ID);
        if age != 0 {
            set_age(&mut store, bee, age);
        }
        // The closed flower is food only when the effective datapack adds it.
        let tags =
            EntityRegistryTagView::Frozen(std::sync::Arc::new(RegistryTagSnapshot::new([(
                "minecraft:item".to_owned(),
                "minecraft:bee_food".to_owned(),
                vec![flower.to_owned()],
            )])));
        let mut hand = stack(flower, 2);
        let mut player = leash::LeashInteractingPlayer {
            mob_item_used: false,
            hand_stack: &mut hand,
            entity_id: 100,
            uuid: PLAYER_UUID,
            position: position(),
            secondary_use_active: false,
            infinite_materials: false,
        };
        let result = leash::interact_leash_with_loot(
            &mut store,
            bee,
            &mut player,
            EntityInteractionLoot {
                dimension: None,
                interacting_entity: crate::loot_runtime::LootEntity::Absent,
                weather: crate::loot_runtime::LootWeather {
                    raining: false,
                    thundering: false,
                },
                tables: crate::entity::block_drops::BlockDropTables {
                    tables: Some(&datapacks().loot),
                    recipes: None,
                    damage_type_tags: None,
                },
                level: None,
                block_entity_data: None,
                item_environment: None,
                enchantment_registry: &[],
                registry_tags: &tags,
                level_random: None,
                growth: None,
                spawn_egg: None,
            },
        );
        assert_eq!(result, LeashInteraction::AnimalFed);
        assert!(player.mob_item_used);
        assert_eq!(hand.count, 1);
        assert_eq!(ageable(&store, bee).age(), age);
        assert!(!animal_state(&store, bee).is_in_love());
        let applied = store
            .get(bee)
            .unwrap()
            .living()
            .unwrap()
            .active_effect(effect)
            .unwrap();
        assert_eq!(applied.duration(), duration);
    }
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let bee = animal(&mut store, VANILLA_ENTITY_TYPE_BEE_ID);
    let mut hand = stack("minecraft:closed_eyeblossom", 1);
    assert_eq!(
        interact(
            &mut store,
            bee,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::Pass
    );
    assert_eq!(hand.count, 1);
    assert!(store
        .get(bee)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:poison")
        .is_none());
}

fn tadpole_state(store: &EntityStore, id: i32) -> crate::entity::tadpole::TadpoleRetainedState {
    let EntityData::GeneralLiving(data) = store.get(id).expect("the tadpole").data() else {
        panic!("a general living tadpole")
    };
    data.tadpole_retained().expect("tadpole state")
}

#[test]
fn a_slime_ball_ages_a_tadpole_and_a_golden_dandelion_locks_it() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let tadpole = animal(
        &mut store,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TADPOLE_ID,
    );
    let mut hand = stack("minecraft:slime_ball", 2);
    let tags = EntityRegistryTagView::Builtin26_1;

    assert_eq!(
        interact(&mut store, tadpole, &mut hand, &tags),
        LeashInteraction::GrewUp
    );
    // `getSpeedUpSecondsWhenFeeding(24000)` is 120 seconds (`Tadpole.java:212-216`).
    assert_eq!(tadpole_state(&store, tadpole).age(), 2_400);
    assert_eq!(hand.count, 1);

    let mut dandelion = stack("minecraft:golden_dandelion", 1);
    assert_eq!(
        interact(&mut store, tadpole, &mut dandelion, &tags),
        LeashInteraction::AgeLockToggled
    );
    let state = tadpole_state(&store, tadpole);
    assert!(state.age_locked());
    assert_eq!(state.age(), 0);
    assert!(persistence_required(&store, tadpole));

    // A locked tadpole refuses food.
    assert_eq!(
        interact(&mut store, tadpole, &mut hand, &tags),
        LeashInteraction::Pass
    );
    assert_eq!(hand.count, 1);
}

/// An ocelot whose `OcelotTemptGoal` follows a player holding cod.
fn tempted_ocelot() -> (EntityStore, i32) {
    use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
    let mut native = NativeLevel::new();
    native
        .world
        .load_entity_chunk_for_test(DIMENSION, (0, 0))
        .unwrap();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "player1", 1);
    let player = crate::connection::ConnectionId(1);
    native.world.set_player_position_for_test(
        player,
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 8.5,
        },
    );
    native
        .world
        .stock_menu_slot_for_test(player, 36, stack("minecraft:cod", 1));
    let ids = EntityIdCounter::new();
    for _ in 0..native.world.player_entity_id_for_test(player) {
        ids.next_id();
    }
    let mut store = EntityStore::new(7, ids);
    let ocelot = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OCELOT_ID,
            Vec3d {
                x: 4.5,
                y: 64.0,
                z: 8.5,
            },
        )
        .unwrap();
    for _ in 0..20 {
        native.with_level(|level| crate::entity::tick_one_entity(&mut store, level, ocelot));
    }
    (store, ocelot)
}

#[test]
fn fed_fish_makes_an_ocelot_trust_and_then_breed() {
    let tags = EntityRegistryTagView::Builtin26_1;
    // Without a running tempt goal the fish only feeds `Animal.mobInteract`.
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let untempted = animal(
        &mut store,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OCELOT_ID,
    );
    let mut hand = stack("minecraft:cod", 64);
    assert_eq!(
        interact(&mut store, untempted, &mut hand, &tags),
        LeashInteraction::FellInLove
    );

    let (mut store, ocelot) = tempted_ocelot();
    let trusting = |store: &EntityStore| {
        let EntityData::GeneralLiving(data) = store.get(ocelot).unwrap().data() else {
            unreachable!()
        };
        data.ocelot_trusting().unwrap()
    };
    let mut hand = stack("minecraft:cod", 64);
    let mut feedings = 0;
    while !trusting(&store) {
        assert_eq!(
            interact(&mut store, ocelot, &mut hand, &tags),
            LeashInteraction::AnimalFed
        );
        feedings += 1;
        assert!(feedings < 64, "an ocelot trusts within a stack of fish");
    }
    assert_eq!(hand.count, 64 - feedings);
    let bbb_protocol::NbtSummaryValue::Compound(record) = store.command_data(ocelot).unwrap()
    else {
        panic!("an entity record is a compound")
    };
    assert!(record
        .iter()
        .any(|entry| entry.name == "Trusting"
            && entry.value == bbb_protocol::NbtSummaryValue::Byte(1)));

    // `Ocelot.mobInteract` falls through to `Animal.mobInteract` once trusting.
    assert_eq!(
        interact(&mut store, ocelot, &mut hand, &tags),
        LeashInteraction::FellInLove
    );
    assert_eq!(animal_state(&store, ocelot).in_love(), 600);
}

/// `HappyGhast.canFallInLove` is false (`HappyGhast.java:247-249`), so an adult
/// takes neither branch and keeps the snowball.
#[test]
fn an_adult_happy_ghast_does_not_fall_in_love() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let ghast = animal(
        &mut store,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    );
    let _ = events(&mut store);
    let mut hand = stack("minecraft:snowball", 2);
    assert_eq!(
        interact(
            &mut store,
            ghast,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::Pass
    );
    assert_eq!(hand.count, 2);
    assert_eq!(animal_state(&store, ghast).in_love(), 0);
    assert!(events(&mut store).broadcasts.is_empty());
}

/// `Hoglin.mobInteract` sets persistence once `super` consumed (`Hoglin.java:204-211`).
#[test]
fn a_fed_hoglin_becomes_persistent() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let hoglin = animal(
        &mut store,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HOGLIN_ID,
    );
    let persistent = |store: &EntityStore| match store.get(hoglin).unwrap().data() {
        EntityData::GeneralLiving(data) => data.mob.as_ref().unwrap().persistence_required(),
        _ => unreachable!(),
    };
    assert!(!persistent(&store));
    let mut hand = stack("minecraft:crimson_fungus", 2);
    assert_eq!(
        interact(
            &mut store,
            hoglin,
            &mut hand,
            &EntityRegistryTagView::Builtin26_1
        ),
        LeashInteraction::FellInLove
    );
    assert!(persistent(&store));
}
