use std::sync::OnceLock;

use bbb_protocol::entity_synched_data::{
    VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID, VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
};
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::{ItemStackSummary, Vec3d};
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use uuid::Uuid;

use super::*;
use crate::entity::leash::{self, EntityInteractionLoot, LeashInteraction};
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent};

const PLAYER_UUID: Uuid = Uuid::from_u128(0xfeed);

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

/// The same seam the world's `Player.interactOn` path takes
/// (`Player.java:834`), which is where the taming arms have to live to be
/// reachable at all.
fn interact(
    store: &mut EntityStore,
    target_id: i32,
    hand_stack: &mut ItemStackSummary,
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
            infinite_materials: false,
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
            registry_tags: &EntityRegistryTagView::Builtin26_1,
            level_random: None,
            growth: None,
            spawn_egg: None,
        },
    )
}

fn tamable(store: &EntityStore, id: i32) -> crate::entity::general_living::TamableRetainedState {
    let EntityData::GeneralLiving(data) = store.get(id).expect("the animal").data() else {
        panic!("a general living animal")
    };
    data.tamable_retained().expect("a tamable animal").clone()
}

/// What one interaction queued: the `(entity id, event id)` taming particles
/// and the `(sound, volume, pitch)` of every sound it played.
#[derive(Default)]
struct TamingEvents {
    broadcasts: Vec<(i32, i8)>,
    sounds: Vec<(String, f32, f32)>,
}

fn taming_events(store: &mut EntityStore) -> TamingEvents {
    let mut collected = TamingEvents::default();
    for event in store.drain_lifecycle_events() {
        match event {
            EntityLifecycleEvent::Broadcast {
                entity_id,
                packet: crate::connection::ClientboundPlayPacket::EntityEvent(packet),
            } => collected.broadcasts.push((entity_id, packet.event_id)),
            EntityLifecycleEvent::Sound {
                sound:
                    bbb_protocol::packets::SoundEvent {
                        sound: bbb_protocol::packets::SoundEventHolder::Reference { registry_id },
                        volume,
                        pitch,
                        ..
                    },
            } => collected.sounds.push((
                bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(registry_id)
                    .expect("a built-in sound")
                    .to_string(),
                volume,
                pitch,
            )),
            _ => {}
        }
    }
    collected
}

fn broadcast_events(store: &mut EntityStore) -> Vec<(i32, i8)> {
    taming_events(store).broadcasts
}

/// `finalizeSpawn`'s pick for `minecraft:cat_sound_variant` as the save field
/// carries it, which is the read `Cat.getSoundSet` takes (`Cat.java:133-135`).
fn set_cat_sound_variant(store: &mut EntityStore, id: i32, resource_id: &str) {
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).expect("the cat").data else {
        panic!("a general living cat")
    };
    data.apply_saved_variant_fields(&[bbb_protocol::NbtSummaryEntry {
        name: "sound_variant".into(),
        value: bbb_protocol::NbtSummaryValue::string(resource_id),
    }]);
}

fn synched_row(
    store: &EntityStore,
    id: i32,
    data_id: u8,
) -> Option<bbb_protocol::packets::EntityDataValue> {
    crate::entity::tracking::non_default_synched_values(store.get(id).expect("the animal"))
        .into_iter()
        .find(|row| row.data_id == data_id)
}

/// `Wolf.mobInteract`'s bone arm (`Wolf.java:491-495`) over `tryToTame`
/// (`:500-510`) on the real interaction seam: one bone leaves the hand per
/// click, each click raises exactly one taming particle event, and the first
/// event 7 is the click that wrote the owner.
///
/// The roll is `this.random.nextInt(3) == 0` off the wolf's own random, so the
/// click that takes is fixed by the store seed rather than chosen here; the
/// loop asserts the shape of every click and the state either side of the one
/// that took.
#[test]
fn a_wolf_fed_bones_is_tamed_by_the_one_in_three_roll() {
    let mut store = EntityStore::new(0x7A_01, EntityIdCounter::new());
    let wolf = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WOLF_ID, position())
        .expect("a stored wolf");
    let mut bones = stack("minecraft:bone", 32);
    let mut tamed_on = None;
    for click in 0..32 {
        let before = bones.count;
        let outcome = interact(&mut store, wolf, &mut bones);
        let events = broadcast_events(&mut store);
        assert_eq!(bones.count, before - 1, "click {click} spent one bone");
        match outcome {
            LeashInteraction::Tamed { server_swing, .. } => {
                assert!(server_swing, "`Wolf.java:494` is SUCCESS_SERVER");
                assert_eq!(events, vec![(wolf, 7)]);
                tamed_on = Some(click);
                break;
            }
            LeashInteraction::TamingFailed { server_swing, .. } => {
                assert!(server_swing);
                assert_eq!(events, vec![(wolf, 6)]);
                assert!(
                    !tamable(&store, wolf).is_tame(),
                    "a refused roll writes nothing"
                );
                assert!(store.drain_tamed_animals().is_empty());
            }
            other => panic!("click {click} answered {other:?}"),
        }
    }
    let tamed_on = tamed_on.expect("the roll takes inside 32 clicks");

    let state = tamable(&store, wolf);
    assert!(state.is_tame());
    assert_eq!(state.owner(), Some(PLAYER_UUID));
    // `setOrderedToSit(true)` (`Wolf.java:505`).
    assert!(state.ordered_to_sit());
    // `Wolf.applyTamingSideEffects` (`Wolf.java:425-432`).
    let living = store
        .get(wolf)
        .expect("the wolf")
        .living()
        .expect("a living wolf");
    assert_eq!(living.max_health(), 40.0);
    assert_eq!(living.health(), 40.0);
    // `CriteriaTriggers.TAME_ANIMAL.trigger` (`TamableAnimal.java:171-173`).
    assert_eq!(
        store.drain_tamed_animals(),
        vec![crate::entity::PendingTamedAnimal {
            player_uuid: PLAYER_UUID,
            entity_id: wolf,
            entity_type_id: VANILLA_ENTITY_TYPE_WOLF_ID,
        }]
    );
    // Both rows a client reads the tame state off (`TamableAnimal.java:40`,
    // `:46`), published only because the taming marked them dirty.
    assert!(synched_row(&store, wolf, VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID).is_some());
    assert!(synched_row(&store, wolf, VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID).is_some());

    // `if (this.isTame())` (`Wolf.java:447`) now takes the other branch: a bone
    // is neither food nor a collar dye, so the click falls past
    // `super.mobInteract` onto the sit toggle (`:473-479`) and stands the wolf
    // back up.
    let before = bones.count;
    assert_eq!(
        interact(&mut store, wolf, &mut bones),
        LeashInteraction::SitToggled
    );
    assert_eq!(bones.count, before, "a tame wolf's bone is not eaten");
    assert!(!tamable(&store, wolf).ordered_to_sit());
    assert!(broadcast_events(&mut store).is_empty());
    assert!(tamed_on < 32);
}

/// `Cat.mobInteract`'s cat-food arm (`Cat.java:427-436`) over `tryToTame`
/// (`:474-481`): the same roll, `SUCCESS` instead of `SUCCESS_SERVER`
/// (`:435`), plus the two writes the cat arm adds -- `setPersistenceRequired`
/// (`:431`) and `playEatingSound` (`:432`).
#[test]
fn a_cat_fed_cod_is_tamed_and_stops_despawning() {
    let mut store = EntityStore::new(0x7A_02, EntityIdCounter::new());
    let cat = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CAT_ID, position())
        .expect("a stored cat");
    assert!(!store.get(cat).expect("the cat").is_persistence_required());
    let mut cod = stack("minecraft:cod", 32);
    let mut tamed = false;
    for click in 0..32 {
        let before = cod.count;
        let outcome = interact(&mut store, cat, &mut cod);
        assert_eq!(cod.count, before - 1, "click {click} spent one cod");
        // `Cat.playEatingSound` (`Cat.java:284-286`) runs whether or not the
        // roll took, and its `setPersistenceRequired` with it.
        assert!(store.get(cat).expect("the cat").is_persistence_required());
        let events = taming_events(&mut store);
        // `this.playSound(this.getSoundSet().eatSound(), 1.0F, 1.0F)` (`:285`)
        // for the `classic` sound variant a constructed cat carries.
        assert_eq!(
            events.sounds,
            vec![("minecraft:entity.cat.eat".to_owned(), 1.0, 1.0)]
        );
        match outcome {
            LeashInteraction::Tamed { server_swing, .. } => {
                assert!(
                    !server_swing,
                    "`Cat.java:435` is SUCCESS, not SUCCESS_SERVER"
                );
                assert_eq!(events.broadcasts, vec![(cat, 7)]);
                tamed = true;
                break;
            }
            LeashInteraction::TamingFailed { server_swing, .. } => {
                assert!(!server_swing);
                assert_eq!(events.broadcasts, vec![(cat, 6)]);
            }
            other => panic!("click {click} answered {other:?}"),
        }
    }
    assert!(tamed, "the roll takes inside 32 clicks");

    let state = tamable(&store, cat);
    assert!(state.is_tame());
    assert_eq!(state.owner(), Some(PLAYER_UUID));
    assert!(state.ordered_to_sit());
    // `Cat.removeWhenFarAway` (`Cat.java:452-454`) is the one consequence of a
    // tame cat this runtime already reads, at `general_living/despawn.rs`'s
    // `"minecraft:cat"` arm: its `!data.is_tame()` half is now false, so the
    // distance despawn can no longer take this cat.
    let EntityData::GeneralLiving(data) = store.get(cat).expect("the cat").data() else {
        panic!("a general living cat")
    };
    assert!(data.is_tame());
}

/// `Cat.getSoundSet().eatSound()` is the cat's own sound variant's
/// (`Cat.java:133-135`, `CatSoundVariants.java:36-37`), so a `royal` cat eats
/// with the royal event.
#[test]
fn a_cats_eat_sound_follows_its_sound_variant() {
    let mut store = EntityStore::new(0x7A_03, EntityIdCounter::new());
    let royal = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CAT_ID, position())
        .expect("a stored cat");
    set_cat_sound_variant(&mut store, royal, "minecraft:royal");
    let mut cod = stack("minecraft:cod", 4);

    let outcome = interact(&mut store, royal, &mut cod);

    assert!(matches!(
        outcome,
        LeashInteraction::Tamed { .. } | LeashInteraction::TamingFailed { .. }
    ));
    assert_eq!(cod.count, 3);
    assert_eq!(
        taming_events(&mut store).sounds,
        vec![("minecraft:entity.cat_royal.eat".to_owned(), 1.0, 1.0)]
    );
}

/// `TamableAnimal.addAdditionalSaveData` / `readAdditionalSaveData`
/// (`TamableAnimal.java:66-83`): the two fields the format already declares
/// carry a freshly tamed animal across a save, and `readAdditionalSaveData`
/// rebuilds the tame flag from the `Owner` reference it read (`:68-73`).
#[test]
fn a_freshly_tamed_wolf_survives_the_save_round_trip() {
    let mut store = EntityStore::new(0x7A_04, EntityIdCounter::new());
    let wolf = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WOLF_ID, position())
        .expect("a stored wolf");
    let mut bones = stack("minecraft:bone", 64);
    let mut tamed = false;
    for _ in 0..64 {
        if matches!(
            interact(&mut store, wolf, &mut bones),
            LeashInteraction::Tamed { .. }
        ) {
            tamed = true;
            break;
        }
    }
    assert!(tamed);
    let _ = store.drain_lifecycle_events();
    let _ = store.drain_tamed_animals();

    let body = store.get(wolf).expect("the wolf").clone();
    let record =
        crate::entity::save::entity_transfer_record(&store, &body).expect("the wolf saves");
    let mut restored_store = EntityStore::new(0x7A_05, EntityIdCounter::new());
    let restored = crate::entity::save::load_entity_transfer_record(
        &mut restored_store,
        record.as_ref(),
        None,
    )
    .expect("the record reads back");

    let EntityData::GeneralLiving(data) = restored.data() else {
        panic!("a general living wolf")
    };
    let state = data.tamable_retained().expect("a tamable wolf");
    assert!(state.is_tame());
    assert_eq!(state.owner(), Some(PLAYER_UUID));
    assert!(state.ordered_to_sit());
}

/// A tame body of `entity_type_id` owned by `owner`, which is what the tamed
/// side of both overrides opens on (`Wolf.java:447`, `Cat.java:398-399`).
fn a_tame_animal(store: &mut EntityStore, entity_type_id: i32, owner: Uuid) -> i32 {
    let id = store
        .spawn_general_living(entity_type_id, position())
        .expect("a stored animal");
    let mut entity = store.detach(id).expect("the animal detaches");
    assert!(set_tame_offspring(&mut entity, Some(owner)));
    store.attach(entity);
    store.drain_lifecycle_events();
    id
}

fn collar(store: &EntityStore, id: i32) -> i8 {
    crate::entity::collar::color_of(store.get(id).expect("the animal")).expect("a collared animal")
}

/// The same interaction seam with a different session behind it.
fn interact_as(
    store: &mut EntityStore,
    target_id: i32,
    hand_stack: &mut ItemStackSummary,
    uuid: Uuid,
) -> LeashInteraction {
    leash::interact_leash_with_loot(
        store,
        target_id,
        &mut leash::LeashInteractingPlayer {
            mob_item_used: false,
            hand_stack,
            entity_id: 1,
            uuid,
            position: position(),
            secondary_use_active: false,
            infinite_materials: false,
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
            registry_tags: &EntityRegistryTagView::Builtin26_1,
            level_random: None,
            growth: None,
            spawn_egg: None,
        },
    )
}

/// `Wolf.mobInteract`'s collar arm (`Wolf.java:485-490`), which sits behind
/// `!itemStack.is(ItemTags.WOLF_COLLAR_DYES) || !this.isOwnedBy(player)`
/// (`:453`): only the owner reaches it, and a dye the collar already carries
/// falls through it.
#[test]
fn only_a_tame_wolfs_owner_dyes_its_collar_and_only_with_a_new_colour() {
    let mut store = EntityStore::new(0x7A_11, EntityIdCounter::new());
    let wolf = a_tame_animal(&mut store, VANILLA_ENTITY_TYPE_WOLF_ID, PLAYER_UUID);
    // `DEFAULT_COLLAR_COLOR = DyeColor.RED` (`Wolf.java:102`).
    assert_eq!(collar(&store, wolf), 14);

    // A stranger's dye never leaves `:453`'s block, so nothing is written and
    // the dye is not spent.
    let mut stranger_dye = stack("minecraft:lime_dye", 1);
    assert_eq!(
        interact_as(&mut store, wolf, &mut stranger_dye, Uuid::from_u128(0xbeef)),
        LeashInteraction::Pass
    );
    assert_eq!(collar(&store, wolf), 14);
    assert_eq!(stranger_dye.count, 1);

    // `DyeColor.LIME` is 5.
    let mut dye = stack("minecraft:lime_dye", 2);
    assert_eq!(
        interact(&mut store, wolf, &mut dye),
        LeashInteraction::CollarDyed
    );
    assert_eq!(collar(&store, wolf), 5);
    assert_eq!(dye.count, 1, "`itemStack.consume(1, player)` (`:488`)");
    assert!(
        synched_row(
            &store,
            wolf,
            bbb_protocol::entity_synched_data::VANILLA_WOLF_COLLAR_COLOR_DATA_ID
        )
        .is_some(),
        "the collar row the client reads (`Wolf.java:89`)"
    );

    // `color != this.getCollarColor()` (`:486`) refuses the same dye again,
    // and `:453` kept the click out of the sit toggle behind `super`.
    assert_eq!(interact(&mut store, wolf, &mut dye), LeashInteraction::Pass);
    assert_eq!(collar(&store, wolf), 5);
    assert_eq!(dye.count, 1);
    assert!(!tamable(&store, wolf).ordered_to_sit());
}

/// `Cat.mobInteract`'s collar arm (`Cat.java:400-410`), which adds
/// `setPersistenceRequired` (`:406`) to the wolf's two writes.
#[test]
fn a_tame_cats_owner_dyes_its_collar_and_stops_it_despawning() {
    let mut store = EntityStore::new(0x7A_12, EntityIdCounter::new());
    let cat = a_tame_animal(&mut store, VANILLA_ENTITY_TYPE_CAT_ID, PLAYER_UUID);
    assert_eq!(collar(&store, cat), 14);
    assert!(!store.get(cat).expect("the cat").is_persistence_required());

    // `DyeColor.YELLOW` is 4.
    let mut dye = stack("minecraft:yellow_dye", 1);
    assert_eq!(
        interact(&mut store, cat, &mut dye),
        LeashInteraction::CollarDyed
    );
    assert_eq!(collar(&store, cat), 4);
    assert_eq!(dye.count, 0);
    assert!(store.get(cat).expect("the cat").is_persistence_required());
    assert!(synched_row(
        &store,
        cat,
        bbb_protocol::entity_synched_data::VANILLA_CAT_COLLAR_COLOR_DATA_ID
    )
    .is_some());
}

/// The sit toggle behind `super.mobInteract` (`Cat.java:419-423`), which is
/// the arm an owner's empty hand reaches.
#[test]
fn a_tame_cats_owner_toggles_its_sit_order_and_a_stranger_does_not() {
    let mut store = EntityStore::new(0x7A_13, EntityIdCounter::new());
    let cat = a_tame_animal(&mut store, VANILLA_ENTITY_TYPE_CAT_ID, PLAYER_UUID);
    assert!(!tamable(&store, cat).ordered_to_sit());
    assert!(!tamable(&store, cat).is_in_sitting_pose());

    let mut empty = ItemStackSummary::empty();
    assert_eq!(
        interact(&mut store, cat, &mut empty),
        LeashInteraction::SitToggled
    );
    assert!(tamable(&store, cat).ordered_to_sit());
    assert!(tamable(&store, cat).is_in_sitting_pose());
    assert!(synched_row(&store, cat, VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID).is_some());

    // `!this.isOwnedBy(player)` (`:399`) keeps the whole block from a stranger.
    assert_eq!(
        interact_as(&mut store, cat, &mut empty, Uuid::from_u128(0xbeef)),
        LeashInteraction::Pass
    );
    assert!(tamable(&store, cat).ordered_to_sit());

    // The toggle reads its own field back (`:421`).
    assert_eq!(
        interact(&mut store, cat, &mut empty),
        LeashInteraction::SitToggled
    );
    assert!(!tamable(&store, cat).ordered_to_sit());
    assert!(!tamable(&store, cat).is_in_sitting_pose());
}

/// `Wolf.mobInteract`'s healing arm (`Wolf.java:448-451`) over
/// `TamableAnimal.feed` (`TamableAnimal.java:135-140`): it precedes
/// `super.mobInteract`, so a hurt tame wolf eats to heal and a whole one eats
/// to fall in love.
#[test]
fn a_hurt_tame_wolf_heals_on_its_food_and_a_whole_one_falls_in_love() {
    let mut store = EntityStore::new(0x7A_14, EntityIdCounter::new());
    let wolf = a_tame_animal(&mut store, VANILLA_ENTITY_TYPE_WOLF_ID, PLAYER_UUID);
    // `Wolf.applyTamingSideEffects` (`Wolf.java:425-432`) left it at 40 of 40.
    let entity = store.get_mut(wolf).expect("the wolf");
    crate::entity::living::set_health(entity, 10.0);

    // `#minecraft:wolf_food` names raw beef, whose `FOOD` component's
    // `nutrition` is 3, and the wolf's healing factor is 2.0F (`:449`).
    let mut beef = stack("minecraft:beef", 2);
    assert_eq!(
        interact(&mut store, wolf, &mut beef),
        LeashInteraction::AnimalFed
    );
    let living = store
        .get(wolf)
        .expect("the wolf")
        .living()
        .expect("a living wolf");
    assert_eq!(living.health(), 16.0);
    assert_eq!(beef.count, 1);
    assert!(!crate::entity::animal_breeding::is_in_love(
        store.get(wolf).expect("the wolf")
    ));

    // At full health `:448` is false and the same click reaches
    // `Animal.mobInteract` (`Animal.java:143-148`).
    let entity = store.get_mut(wolf).expect("the wolf");
    crate::entity::living::set_health(entity, 40.0);
    assert_eq!(
        interact(&mut store, wolf, &mut beef),
        LeashInteraction::FellInLove
    );
    assert_eq!(beef.count, 0);
    assert!(crate::entity::animal_breeding::is_in_love(
        store.get(wolf).expect("the wolf")
    ));
}

/// `!this.isAngry()` (`Wolf.java:491`) keeps a bone off an angry wolf, and an
/// owner's sit toggle drops the target (`Wolf.java:475-479`).
#[test]
fn an_angry_wolf_refuses_the_bone_and_a_sit_order_drops_the_target() {
    use crate::entity::ai::mob::target::anger::{self, NeutralAnger};
    use crate::entity::ai::mob::target::TargetRef;

    let mut store = EntityStore::new(0x7A16, EntityIdCounter::new());
    store.set_game_time(100);
    let wild = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WOLF_ID, position())
        .expect("a stored wolf");
    let mut entity = store.detach(wild).expect("the wolf detaches");
    anger::write(
        &mut entity,
        NeutralAnger {
            end_time: 101,
            target: None,
        },
    );
    store.attach(entity);
    let mut bones = stack("minecraft:bone", 4);
    assert_ne!(
        interact(&mut store, wild, &mut bones),
        LeashInteraction::Tamed {
            server_swing: true,
            remainder: None
        }
    );
    assert_eq!(bones.count, 4, "an angry wolf eats no bone");
    assert!(!tamable(&store, wild).is_tame());

    let wolf = a_tame_animal(&mut store, VANILLA_ENTITY_TYPE_WOLF_ID, PLAYER_UUID);
    let cat = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CAT_ID, position())
        .unwrap();
    let target = TargetRef {
        entity_id: cat,
        uuid: store.get(cat).unwrap().uuid(),
    };
    let state = store.mob_ai_mut(wolf).expect("a mob").target_state_mut();
    state.target = Some(target);
    assert!(store.mob_target(wolf).is_some());
    let mut empty = ItemStackSummary::empty();
    assert_eq!(
        interact(&mut store, wolf, &mut empty),
        LeashInteraction::SitToggled
    );
    assert!(tamable(&store, wolf).ordered_to_sit());
    assert_eq!(store.mob_target(wolf), None);
}

/// `Parrot.mobInteract` (`Parrot.java:252-289`): seeds roll a one-in-ten
/// taming without a sit order, and the owner's empty hand on the ground
/// toggles the order.
#[test]
fn a_parrot_fed_seeds_is_tamed_and_its_owner_sits_it() {
    let mut store = EntityStore::new(0x7A_20, EntityIdCounter::new());
    let parrot = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PARROT_ID, position())
        .expect("a stored parrot");
    let mut seeds = stack("minecraft:wheat_seeds", 64);
    let mut fed = 0;
    while !tamable(&store, parrot).is_tame() {
        assert!(fed < 64, "a roll takes inside 64 seeds");
        fed += 1;
        let answer = interact(&mut store, parrot, &mut seeds);
        assert!(matches!(
            answer,
            LeashInteraction::Tamed { .. } | LeashInteraction::TamingFailed { .. }
        ));
        let events = taming_events(&mut store);
        assert!(events
            .sounds
            .iter()
            .any(|(sound, _, _)| sound == "minecraft:entity.parrot.eat"));
    }
    assert_eq!(seeds.count, 64 - fed);
    assert_eq!(tamable(&store, parrot).owner(), Some(PLAYER_UUID));
    assert!(!tamable(&store, parrot).ordered_to_sit());
    assert_eq!(store.drain_tamed_animals().len(), 1);

    store
        .get_mut(parrot)
        .expect("the parrot")
        .set_on_ground(true);
    let mut empty = ItemStackSummary::empty();
    assert_eq!(
        interact(&mut store, parrot, &mut empty),
        LeashInteraction::SitToggled
    );
    assert!(tamable(&store, parrot).is_in_sitting_pose());
}

fn body_armour(store: &EntityStore, id: i32) -> Option<ItemStackSummary> {
    store
        .get(id)
        .and_then(Entity::living)
        .and_then(|living| living.item_by_slot(bbb_protocol::packets::EquipmentSlot::Body))
        .map(|armour| armour.summary().clone())
}

/// `Wolf.java:454-480`: the owner equips wolf armour, mends it with a scute
/// while the wolf sits, and any other item still reaches the sit toggle.
#[test]
fn a_tame_wolfs_owner_equips_repairs_and_still_sits_an_armoured_wolf() {
    let mut store = EntityStore::new(0x7A_13, EntityIdCounter::new());
    let wolf = a_tame_animal(&mut store, VANILLA_ENTITY_TYPE_WOLF_ID, PLAYER_UUID);
    let mut armour = stack("minecraft:wolf_armor", 2);
    assert!(matches!(
        interact(&mut store, wolf, &mut armour),
        LeashInteraction::MountEquipped { entity_id, .. } if entity_id == wolf
    ));
    assert_eq!(armour.count, 1);
    let mut worn = body_armour(&store, wolf).expect("the wolf wears its armour");
    assert_eq!(worn.item_id, armour.item_id);
    assert_eq!(worn.count, 1);

    let mut stick = stack("minecraft:stick", 1);
    assert_eq!(
        interact(&mut store, wolf, &mut stick),
        LeashInteraction::SitToggled
    );
    assert!(tamable(&store, wolf).ordered_to_sit());

    crate::item_durability::repair_to(&mut worn, 20);
    let target = store.get_mut(wolf).unwrap();
    target.living_mut().unwrap().update_item_slot(
        bbb_protocol::packets::EquipmentSlot::Body,
        bbb_server_types::item_stack::RuntimeItemStack::from_summary(worn),
    );
    let EntityData::GeneralLiving(data) = &mut target.data else {
        unreachable!()
    };
    data.tamable_retained_mut()
        .unwrap()
        .set_in_sitting_pose(true);
    let mut scute = stack("minecraft:armadillo_scute", 3);
    assert_eq!(
        interact(&mut store, wolf, &mut scute),
        LeashInteraction::Repaired
    );
    assert_eq!(scute.count, 2);
    // 64 max damage: one scute mends `(int) (64 * 0.125F)` = 8.
    let mended = body_armour(&store, wolf).unwrap();
    assert_eq!(
        crate::item_durability::damage_and_max(&mended),
        Some((12, 64))
    );

    assert_eq!(
        interact(&mut store, wolf, &mut stick),
        LeashInteraction::SitToggled
    );
    assert!(!tamable(&store, wolf).ordered_to_sit());
}
