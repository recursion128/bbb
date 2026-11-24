use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::Vec3d;
use uuid::Uuid;

use super::*;
use crate::entity::leash::{self, LeashInteraction};
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent};

const PLAYER_UUID: Uuid = Uuid::from_u128(0xb0a7);

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

/// The same seam `Player.interactOn` takes (`Player.java:834`), which is where
/// `AbstractHorse.mobInteract` has to live to be reachable at all.
fn interact(
    store: &mut EntityStore,
    target_id: i32,
    hand_stack: &mut ItemStackSummary,
    secondary_use_active: bool,
) -> LeashInteraction {
    leash::interact_leash(
        store,
        target_id,
        &mut leash::LeashInteractingPlayer {
            mob_item_used: false,
            hand_stack,
            entity_id: 1,
            uuid: PLAYER_UUID,
            position: position(),
            secondary_use_active,
            infinite_materials: false,
        },
    )
}

fn horse_flags(store: &EntityStore, id: i32) -> i8 {
    let EntityData::GeneralLiving(data) = store.get(id).expect("the horse").data() else {
        panic!("a general living horse")
    };
    data.abstract_horse_retained().expect("an equine").flags()
}

fn broadcasts(store: &mut EntityStore) -> Vec<(i32, i8)> {
    store
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Broadcast {
                entity_id,
                packet: crate::connection::ClientboundPlayPacket::EntityEvent(packet),
            } => Some((entity_id, packet.event_id)),
            _ => None,
        })
        .collect()
}

/// `Horse.mobInteract` with an empty hand reaches `doPlayerRide`
/// (`Horse.java:157-170`, `AbstractHorse.java:666`), and its two body writes
/// run before the caller is told to seat the rider (`:495-496`).
#[test]
fn an_empty_hand_on_an_untamed_adult_horse_answers_the_ride() {
    let mut store = EntityStore::new(0x40_01, EntityIdCounter::new());
    let horse = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_HORSE_ID, position())
        .expect("a stored horse");
    store
        .get_mut(horse)
        .expect("the horse")
        .set_rot(41.0, -12.0);
    // `makeMad`'s rearing is the state `clearStanding` has to clear
    // (`AbstractHorse.java:687-690`, `:704`).
    let mut stick = stack("minecraft:stick", 1);
    assert_eq!(
        interact(&mut store, horse, &mut stick, false),
        LeashInteraction::MadeMad
    );

    let mut empty = ItemStackSummary::empty();
    assert_eq!(
        interact(&mut store, horse, &mut empty, false),
        LeashInteraction::Mounted {
            vehicle_entity_id: horse,
            rotation: Some((41.0, -12.0)),
        }
    );
    assert_eq!(horse_flags(&store, horse), 0);
}

/// `Horse.mobInteract`'s untamed non-food arm (`Horse.java:164-167`) over
/// `makeMad` (`AbstractHorse.java:702-707`): the horse rears and the angry
/// sound plays, and the click never reaches `doPlayerRide`.
#[test]
fn a_non_food_item_on_an_untamed_horse_makes_it_mad() {
    let mut store = EntityStore::new(0x40_03, EntityIdCounter::new());
    let horse = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_HORSE_ID, position())
        .expect("a stored horse");
    let _ = broadcasts(&mut store);

    let mut stick = stack("minecraft:stick", 1);
    assert_eq!(
        interact(&mut store, horse, &mut stick, false),
        LeashInteraction::MadeMad
    );
    assert_eq!(stick.count, 1, "the arm spends nothing");
    // `setStanding(20)`'s flag (`AbstractHorse.java:681-684`).
    assert_eq!(
        horse_flags(&store, horse),
        bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAG_STANDING
    );
    // `makeMad` is guarded by `!isStanding()` (`:703`), so a second click adds
    // nothing.
    assert_eq!(
        interact(&mut store, horse, &mut stick, false),
        LeashInteraction::MadeMad
    );
    assert!(broadcasts(&mut store).is_empty(), "no entity event is owed");
}

#[test]
fn camel_food_heals_and_grows_ten_seconds_without_taming_or_remainders() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID;
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let camel = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CAMEL_ID, position())
        .unwrap();
    let target = store.get_mut(camel).unwrap();
    crate::entity::living::set_health(target, 5.0);
    let EntityData::GeneralLiving(data) = &mut target.data else {
        unreachable!()
    };
    data.ageable_retained_mut().unwrap().set_age(-24000);
    let mut hand = stack("minecraft:cactus", 2);
    assert_eq!(
        interact(&mut store, camel, &mut hand, false),
        LeashInteraction::MountFed
    );
    assert_eq!(hand.count, 1);
    assert_eq!(store.get(camel).unwrap().living().unwrap().health(), 7.0);
    let EntityData::GeneralLiving(data) = store.get(camel).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(data.ageable_retained().unwrap().age(), -23800);
    assert!(!data.animal_retained().unwrap().is_in_love());
    assert_eq!(data.ageable_retained().unwrap().forced_age(), 0);
    assert!(store
        .drain_commands()
        .iter()
        .any(|command| matches!(command,
        crate::entity::EntityCommand::GameEvent { kind, source_entity_id: Some(id), .. }
            if *kind == crate::level_event::VanillaGameEvent::EAT && *id == camel)));

    let target = store.get_mut(camel).unwrap();
    let max_health = target.living().unwrap().max_health();
    crate::entity::living::set_health(target, max_health);
    let EntityData::GeneralLiving(data) = &mut target.data else {
        unreachable!()
    };
    data.ageable_retained_mut().unwrap().set_age(0);
    data.animal_retained_mut()
        .unwrap()
        .set_in_love(100, Some(PLAYER_UUID));
    assert_eq!(
        interact(&mut store, camel, &mut hand, false),
        LeashInteraction::Pass
    );
    assert_eq!(hand.count, 1);
    assert!(store.drain_commands().is_empty());
}

#[test]
fn feeding_opens_ordinary_equine_mouths_even_when_silent_but_not_llamas() {
    use bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH;
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_MULE_ID,
    };
    for entity_type in [
        VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_DONKEY_ID,
        VANILLA_ENTITY_TYPE_MULE_ID,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
    ] {
        let mut store = EntityStore::new(9, EntityIdCounter::new());
        let id = store.spawn_general_living(entity_type, position()).unwrap();
        let target = store.get_mut(id).unwrap();
        target.set_silent(true);
        let EntityData::GeneralLiving(data) = &mut target.data else {
            unreachable!()
        };
        data.abstract_horse_retained_mut().unwrap().set_tame(true);
        let mut hand = stack("minecraft:wheat", 2);
        assert_eq!(
            interact(&mut store, id, &mut hand, false),
            LeashInteraction::Pass
        );
        assert_eq!(
            horse_flags(&store, id) & VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH,
            0
        );
        assert!(store.drain_commands().is_empty());
        assert_eq!(hand.count, 2);

        crate::entity::living::set_health(store.get_mut(id).unwrap(), 5.0);
        assert_eq!(
            interact(&mut store, id, &mut hand, false),
            LeashInteraction::MountFed
        );
        assert_eq!(hand.count, 1);
        assert!(!store
            .drain_lifecycle_events()
            .iter()
            .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
        let ordinary = entity_type != VANILLA_ENTITY_TYPE_LLAMA_ID;
        assert_eq!(
            horse_flags(&store, id) & VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH != 0,
            ordinary
        );
        assert_eq!(
            store
                .drain_commands()
                .iter()
                .any(|command| matches!(command,
            crate::entity::EntityCommand::GameEvent { kind, source_entity_id: Some(source), .. }
                if *kind == crate::level_event::VanillaGameEvent::EAT && *source == id)),
            ordinary
        );
    }
}

fn played(store: &mut EntityStore, resource_id: &str) -> bool {
    let id = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id)
        .expect("a registered sound");
    store.drain_lifecycle_events().iter().any(|event| {
        matches!(event, EntityLifecycleEvent::Sound { sound }
            if sound.sound == bbb_protocol::packets::SoundEventHolder::Reference { registry_id: id })
    })
}

/// `Mule.playChestEquipsSound` (`Mule.java:49-51`).
#[test]
fn a_chest_on_a_tamed_mule_plays_the_mule_chest_sound() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MULE_ID;
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let mule = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_MULE_ID, position())
        .unwrap();
    let EntityData::GeneralLiving(data) = &mut store.get_mut(mule).unwrap().data else {
        unreachable!()
    };
    data.abstract_horse_retained_mut().unwrap().set_tame(true);
    let _ = store.drain_lifecycle_events();
    let mut hand = stack("minecraft:chest", 1);
    assert!(matches!(
        interact(&mut store, mule, &mut hand, false),
        LeashInteraction::MountEquipped { .. }
    ));
    assert!(played(&mut store, "minecraft:entity.mule.chest"));
}

/// `ZombieHorse.mobInteract` feeds its food (`ZombieHorse.java:148-155`) and
/// `getEatingSound` is the zombie horse's (`:110-112`).
#[test]
fn red_mushroom_feeds_a_hurt_zombie_horse_with_its_eat_sound() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID;
    let mut store = EntityStore::new(13, EntityIdCounter::new());
    let horse = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, position())
        .unwrap();
    crate::entity::living::set_health(store.get_mut(horse).unwrap(), 5.0);
    let _ = store.drain_lifecycle_events();
    let mut hand = stack("minecraft:red_mushroom", 2);
    assert_eq!(
        interact(&mut store, horse, &mut hand, false),
        LeashInteraction::MountFed
    );
    assert_eq!(hand.count, 1);
    assert!(played(&mut store, "minecraft:entity.zombie_horse.eat"));
}

/// `SkeletonHorse.mobInteract` passes until tamed and then answers
/// `AbstractHorse.mobInteract` (`SkeletonHorse.java:173-175`,
/// `AbstractHorse.java:643-668`).
#[test]
fn a_tamed_skeleton_horse_opens_equips_and_mounts() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID;
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let horse = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, position())
        .unwrap();
    let mut empty = ItemStackSummary::empty();
    assert_eq!(
        interact(&mut store, horse, &mut empty, false),
        LeashInteraction::Pass
    );
    let EntityData::GeneralLiving(data) = &mut store.get_mut(horse).unwrap().data else {
        unreachable!()
    };
    data.abstract_horse_retained_mut().unwrap().set_tame(true);
    assert_eq!(
        interact(&mut store, horse, &mut empty, true),
        LeashInteraction::OpenMountInventory {
            vehicle_entity_id: horse
        }
    );
    let mut saddle = stack("minecraft:saddle", 1);
    assert!(matches!(
        interact(&mut store, horse, &mut saddle, false),
        LeashInteraction::MountEquipped { .. }
    ));
    assert!(saddle.item_id.is_none() || saddle.count == 0);
    assert!(matches!(
        interact(&mut store, horse, &mut empty, false),
        LeashInteraction::Mounted { .. }
    ));
}

#[test]
fn a_trader_llama_led_by_its_wandering_trader_cannot_be_ridden() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
    };
    let mut store = EntityStore::new(0x40_02, EntityIdCounter::new());
    let llama = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, position())
        .unwrap();
    let trader = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, position())
        .unwrap();
    let mut body = store.detach(llama).unwrap();
    assert!(leash::set_leashed_to(&mut body, &mut store, trader, true));
    store.attach(body);
    let mut empty = ItemStackSummary::empty();
    assert!(!matches!(
        interact(&mut store, llama, &mut empty, false),
        LeashInteraction::Mounted { .. }
    ));

    let free = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, position())
        .unwrap();
    assert!(matches!(
        interact(&mut store, free, &mut empty, false),
        LeashInteraction::Mounted { .. }
    ));
}
