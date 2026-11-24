use crate::random::GameRandom;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
};
use bbb_protocol::packets::Vec3d;
use uuid::Uuid;

use super::*;
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent};
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::{tick_entities, GeneralLivingData};

const FLOOR_Y: i32 = 64;

fn position(x: f64) -> Vec3d {
    Vec3d {
        x,
        y: f64::from(FLOOR_Y),
        z: 0.5,
    }
}

fn frog(store: &mut EntityStore, uuid: u128, at: Vec3d, in_love: i32) -> i32 {
    let id = store
        .restore(
            Uuid::from_u128(uuid),
            VANILLA_ENTITY_TYPE_FROG_ID,
            at,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_FROG_ID).expect("the frog is living"),
            ),
        )
        .expect("the frog restores");
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).expect("stored").data else {
        unreachable!("a frog is general living")
    };
    *data.animal_retained_mut().expect("a frog is an animal") =
        crate::entity::general_living::AnimalRetainedState::from_saved(in_love, None);
    assert!(crate::entity::set_spawner_brain_memory(
        store.get_mut(id).expect("stored"),
        crate::entity::ai::PackedMemory {
            memory_type: MemoryModuleType::LongJumpCooldownTicks,
            value: MemoryValue::Int(100),
            time_to_live: None,
        },
    ));
    id
}

fn animal(store: &EntityStore, id: i32) -> crate::entity::general_living::AnimalRetainedState {
    let EntityData::GeneralLiving(data) = store.get(id).expect("stored").data() else {
        unreachable!("a frog is general living")
    };
    data.animal_retained().expect("a frog is an animal")
}

fn age(store: &EntityStore, id: i32) -> i32 {
    let EntityData::GeneralLiving(data) = store.get(id).expect("stored").data() else {
        unreachable!("a frog is general living")
    };
    data.ageable_retained().expect("a frog is ageable").age()
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

fn orb_values(store: &EntityStore) -> Vec<i32> {
    store
        .ids()
        .into_iter()
        .filter_map(|id| match store.get(id).expect("stored").data() {
            EntityData::ExperienceOrb(orb) => Some(orb.value()),
            _ => None,
        })
        .collect()
}

fn pregnant(store: &EntityStore, id: i32) -> bool {
    store.live_brain(id).is_some_and(|live| {
        live.check_memory(
            MemoryModuleType::IsPregnant,
            crate::entity::ai::MemoryStatus::Registered,
        )
    })
}

#[test]
fn a_mating_pays_the_pair_and_marks_the_breeding_frog_pregnant() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let body_id = frog(&mut store, 1, position(0.5), 600);
    let partner_id = frog(&mut store, 2, position(1.5), 600);
    let random = GameRandom::with_seed(0);
    let mut native = NativeLevel::new();
    let mut body = store.detach(body_id).expect("the body detaches");

    assert!(native.with_level(|level| spawn_child_from_breeding(
        &mut body, &mut store, level, partner_id, &random,
    )));
    store.attach(body);

    assert_eq!(age(&store, body_id), PARENT_AGE_AFTER_BREEDING);
    assert_eq!(age(&store, partner_id), PARENT_AGE_AFTER_BREEDING);
    assert!(!animal(&store, body_id).is_in_love());
    assert!(!animal(&store, partner_id).is_in_love());
    assert_eq!(broadcasts(&mut store), vec![(body_id, 18)]);
    assert!(matches!(orb_values(&store)[..], [1..=7]));
    assert!(pregnant(&store, body_id));
}

#[test]
fn a_mating_pays_no_experience_when_mob_drops_is_off() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let body_id = frog(&mut store, 3, position(0.5), 600);
    let partner_id = frog(&mut store, 4, position(1.5), 600);
    let random = GameRandom::with_seed(0);
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    sender
        .try_send(crate::world::WorldCommand::Admin {
            envelope: crate::admin::AdminEnvelope {
                operation_id: 1,
                request_id: "rule".into(),
                principal: "fixture".into(),
                scope: crate::admin::AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 0,
            },
            request: crate::admin::AdminRequest::ExecuteCommand {
                command: "gamerule minecraft:mob_drops false".into(),
            },
        })
        .unwrap();
    let result = native.world.tick(receiver.drain());
    assert!(result.events.iter().any(
        |event| matches!(event, crate::world::WorldEvent::Admin(outcome) if outcome.result.is_ok())
    ));
    let mut body = store.detach(body_id).expect("the body detaches");

    assert!(native.with_level(|level| spawn_child_from_breeding(
        &mut body, &mut store, level, partner_id, &random,
    )));
    store.attach(body);

    assert!(orb_values(&store).is_empty());
    assert_eq!(age(&store, body_id), PARENT_AGE_AFTER_BREEDING);
}

#[test]
fn two_in_love_frogs_mate_once_on_the_real_tick() {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let body_id = frog(&mut store, 11, position(0.5), 600);
    let partner_id = frog(&mut store, 12, position(2.5), 600);
    let mut native = NativeLevel::new();
    for _ in 0..400 {
        native
            .world
            .set_game_time_for_test(native.world.game_time() + 1);
        native.with_level(|level| tick_entities(&mut store, level, true));
    }

    assert_eq!(orb_values(&store).len(), 1);
    assert!(age(&store, body_id) > 0);
    assert!(age(&store, partner_id) > 0);
    assert!(!animal(&store, body_id).is_in_love());
    assert!(!animal(&store, partner_id).is_in_love());
    assert!(pregnant(&store, body_id) || pregnant(&store, partner_id));
    assert_eq!(
        store
            .ids()
            .into_iter()
            .filter(|id| *id != body_id && *id != partner_id)
            .count(),
        1
    );
}

fn cow(store: &mut EntityStore, uuid: u128, at: Vec3d, in_love: i32) -> i32 {
    let id = store
        .restore(
            Uuid::from_u128(uuid),
            VANILLA_ENTITY_TYPE_COW_ID,
            at,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).expect("the cow is living"),
            ),
        )
        .expect("the cow restores");
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).expect("stored").data else {
        unreachable!("a cow is general living")
    };
    *data.animal_retained_mut().expect("a cow is an Animal") =
        crate::entity::general_living::AnimalRetainedState::from_saved(in_love, None);
    id
}

fn children(store: &EntityStore, parents: [i32; 2]) -> Vec<(i32, i32)> {
    store
        .ids()
        .into_iter()
        .filter(|id| !parents.contains(id))
        .filter(|id| {
            !matches!(
                store.get(*id).expect("stored").data(),
                EntityData::ExperienceOrb(_)
            )
        })
        .map(|id| (store.get(id).expect("stored").entity_type_id(), id))
        .collect()
}

#[test]
fn a_cow_mating_spawns_one_baby_cow() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let body_id = cow(&mut store, 1, position(0.5), 600);
    let partner_id = cow(&mut store, 2, position(1.5), 600);
    let random = GameRandom::with_seed(0);
    let mut native = NativeLevel::new();
    let mut body = store.detach(body_id).expect("the body detaches");

    assert!(native.with_level(|level| spawn_child_from_breeding(
        &mut body, &mut store, level, partner_id, &random,
    )));
    store.attach(body);

    assert_eq!(age(&store, body_id), PARENT_AGE_AFTER_BREEDING);
    assert_eq!(age(&store, partner_id), PARENT_AGE_AFTER_BREEDING);
    assert!(!animal(&store, body_id).is_in_love());
    assert!(!animal(&store, partner_id).is_in_love());
    assert_eq!(broadcasts(&mut store), vec![(body_id, 18)]);
    assert!(matches!(orb_values(&store)[..], [1..=7]));
    let child = children(&store, [body_id, partner_id]);
    assert_eq!(child.len(), 1);
    assert_eq!(child[0].0, VANILLA_ENTITY_TYPE_COW_ID);
    assert_eq!(age(&store, child[0].1), -24_000);
    assert_eq!(
        store.get(child[0].1).expect("stored").position(),
        position(0.5)
    );
}

#[test]
fn a_hoglin_mating_marks_its_child_persistence_required() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let body_id = store
        .restore(
            Uuid::from_u128(9),
            VANILLA_ENTITY_TYPE_HOGLIN_ID,
            position(0.5),
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_HOGLIN_ID)
                    .expect("the hoglin is living"),
            ),
        )
        .expect("the hoglin restores");
    let partner_id = store
        .restore(
            Uuid::from_u128(11),
            VANILLA_ENTITY_TYPE_HOGLIN_ID,
            position(1.5),
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_HOGLIN_ID)
                    .expect("the hoglin is living"),
            ),
        )
        .expect("the hoglin restores");
    let random = GameRandom::with_seed(0);
    let mut native = NativeLevel::new();
    let mut body = store.detach(body_id).expect("the body detaches");

    assert!(native.with_level(|level| spawn_child_from_breeding(
        &mut body, &mut store, level, partner_id, &random,
    )));
    store.attach(body);

    let child = children(&store, [body_id, partner_id]);
    assert_eq!(child.len(), 1);
    assert_eq!(child[0].0, VANILLA_ENTITY_TYPE_HOGLIN_ID);
    assert!(store
        .get(child[0].1)
        .expect("stored")
        .is_persistence_required());
}

#[test]
fn every_dye_colour_has_exactly_one_carrier_item() {
    for entry in bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_VALUES {
        let items = super::dye_mix::dye_items_of(entry.id);
        assert_eq!(
            items.len(),
            1,
            "{} has {items:?} carrying `minecraft:dye`",
            entry.serialized_name
        );
        let item = bbb_protocol::item_types::vanilla_item_type(items[0])
            .expect("a built-in Java 26.1 item");
        assert_eq!(
            item.resource_id,
            format!("minecraft:{}_dye", entry.serialized_name)
        );
    }
}

fn pair(
    store: &mut EntityStore,
    entity_type_id: i32,
    fields: &[(&str, bbb_protocol::NbtSummaryValue)],
) -> [i32; 2] {
    [1, 2].map(|n| {
        let id = store
            .restore(
                Uuid::from_u128(n),
                entity_type_id,
                position(n as f64 - 0.5),
                EntityData::GeneralLiving(GeneralLivingData::new(entity_type_id).unwrap()),
            )
            .unwrap();
        let bbb_protocol::NbtSummaryValue::Compound(mut entries) = store.command_data(id).unwrap()
        else {
            unreachable!()
        };
        for (name, value) in fields {
            entries.retain(|entry| entry.name != *name);
            entries.push(bbb_protocol::NbtSummaryEntry {
                name: (*name).into(),
                value: value.clone(),
            });
        }
        store.load_command_data(id, &entries).unwrap();
        id
    })
}

/// Breeds one pair per seed and answers each child's `field`.
fn litter(
    entity_type_id: i32,
    fields: &[(&str, bbb_protocol::NbtSummaryValue)],
    field: &str,
    seeds: std::ops::Range<i64>,
    mut each: impl FnMut(&Entity, Option<bbb_protocol::NbtSummaryValue>),
) {
    let mut native = NativeLevel::new();
    for seed in seeds {
        let mut store = EntityStore::new(7, EntityIdCounter::new());
        let [body_id, partner_id] = pair(&mut store, entity_type_id, fields);
        let random = GameRandom::with_seed(seed);
        let mut body = store.detach(body_id).unwrap();
        assert!(native.with_level(|level| spawn_child_from_breeding(
            &mut body, &mut store, level, partner_id, &random,
        )));
        store.attach(body);
        let [(_, child)] = children(&store, [body_id, partner_id])[..] else {
            panic!("one child")
        };
        let bbb_protocol::NbtSummaryValue::Compound(record) = store.command_data(child).unwrap()
        else {
            unreachable!()
        };
        let value = record
            .into_iter()
            .find(|entry| entry.name == field)
            .map(|entry| entry.value);
        each(store.get(child).unwrap(), value);
    }
}

#[test]
fn bred_axolotls_are_persistent_and_rarely_blue() {
    use bbb_protocol::NbtSummaryValue::Int;
    let mut blue = 0;
    litter(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        &[("Variant", Int(1))],
        "Variant",
        0..6_000,
        |child, variant| {
            assert!(child.is_persistence_required());
            match variant {
                Some(Int(1)) => {}
                Some(Int(4)) => blue += 1,
                other => panic!("unexpected axolotl variant {other:?}"),
            }
        },
    );
    assert!((1..=20).contains(&blue), "blue children: {blue}");
}

#[test]
fn rabbit_children_sometimes_take_the_biome_variant() {
    use bbb_protocol::NbtSummaryValue::Int;
    let (mut parental, mut biome) = (0, 0);
    litter(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RABBIT_ID,
        &[("RabbitType", Int(99))],
        "RabbitType",
        0..400,
        |_, variant| match variant {
            Some(Int(99)) => parental += 1,
            Some(Int(0 | 2 | 5)) => biome += 1,
            other => panic!("unexpected rabbit variant {other:?}"),
        },
    );
    assert!(
        parental > 300 && biome > 5,
        "{parental} parental, {biome} biome"
    );
}

#[test]
fn screaming_goats_breed_screaming_kids() {
    use bbb_protocol::NbtSummaryValue::Byte;
    litter(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID,
        &[("IsScreamingGoat", Byte(1))],
        "IsScreamingGoat",
        0..20,
        |_, screaming| assert_eq!(screaming, Some(Byte(1))),
    );
}
