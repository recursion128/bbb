use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID,
    VANILLA_ENTITY_TYPE_TADPOLE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::packets::Vec3d;

use super::*;
use crate::entity::EntityIdCounter;

fn position() -> Vec3d {
    Vec3d {
        x: 3.75,
        y: 64.0,
        z: -2.25,
    }
}

#[test]
fn existing_facts_distinguish_base_persistence_and_custom_passenger_state() {
    let mut store = EntityStore::new(0x43_41_50, EntityIdCounter::new());
    let pig = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, position())
        .unwrap();
    let zombie = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let EntityData::GeneralLiving(pig_data) = &mut store.get_mut(pig).unwrap().data else {
        panic!("pig uses general living state");
    };
    pig_data
        .mob
        .as_mut()
        .unwrap()
        .set_persistence_required(true);

    let pig_facts = natural_spawn_existing_facts(&store, store.get(pig).unwrap()).unwrap();
    assert!(pig_facts.is_mob);
    assert!(pig_facts.persistence_required);
    assert!(!pig_facts.requires_custom_persistence);

    assert!(store.link_existing_passenger_forced(pig, zombie));
    let zombie_facts = natural_spawn_existing_facts(&store, store.get(zombie).unwrap()).unwrap();
    assert!(!zombie_facts.persistence_required);
    assert!(zombie_facts.requires_custom_persistence);
}

#[test]
fn existing_facts_include_bucket_custom_persistence_and_floor_position() {
    let mut store = EntityStore::new(0x42_55_43_4b, EntityIdCounter::new());
    let cod = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COD_ID, position())
        .unwrap();
    let EntityData::GeneralLiving(data) = &mut store.get_mut(cod).unwrap().data else {
        panic!("cod uses general living state");
    };
    assert!(data.set_from_bucket(true));

    let facts = natural_spawn_existing_facts(&store, store.get(cod).unwrap()).unwrap();
    assert_eq!(facts.position, BlockPos { x: 3, y: 64, z: -3 });
    assert!(facts.requires_custom_persistence);

    let tadpole = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_TADPOLE_ID, position())
        .unwrap();
    let tadpole_facts = natural_spawn_existing_facts(&store, store.get(tadpole).unwrap()).unwrap();
    assert!(tadpole_facts.requires_custom_persistence);
}

#[test]
fn a_raider_answers_custom_persistence_from_its_resolved_raid() {
    let mut store = EntityStore::new(0x52_41_49, EntityIdCounter::new());
    let pillager = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PILLAGER_ID, position())
        .unwrap();
    // A raider with no raid at all is an ordinary despawn candidate.
    assert!(
        !natural_spawn_existing_facts(&store, store.get(pillager).unwrap())
            .unwrap()
            .requires_custom_persistence
    );

    // `Raider.java:239-241`: the resolved current raid is what keeps it.
    set_raid(
        &mut store,
        pillager,
        crate::entity::raider::RaiderRaid::Current(4),
    );
    assert!(
        natural_spawn_existing_facts(&store, store.get(pillager).unwrap())
            .unwrap()
            .requires_custom_persistence
    );
}

fn set_raid(store: &mut EntityStore, id: i32, raid: crate::entity::raider::RaiderRaid) {
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data else {
        panic!("a raider uses general living state");
    };
    data.set_raider_raid(raid);
}
