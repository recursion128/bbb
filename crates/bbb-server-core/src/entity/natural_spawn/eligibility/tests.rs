use crate::entity::GeneralLivingData;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COD_ID,
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_OCELOT_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID,
    VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};
use uuid::Uuid;

use super::*;
use crate::entity::natural_spawn_facts_26_1::VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1;
use crate::entity::{EntityIdCounter, EntityStore};

fn fresh_entity(entity_type_id: i32) -> Entity {
    let mut store = EntityStore::new(0x45_4c_49_47, EntityIdCounter::new());
    let id = store
        .spawn_general_living(entity_type_id, Vec3d::default())
        .expect("test type is GeneralLiving");
    store.get(id).unwrap().clone()
}

#[test]
fn natural_spawn_default_mob_and_animal_answers_are_retained() {
    let zombie = fresh_entity(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    assert_eq!(
        natural_spawn_pre_finalize_mob_facts(&zombie, 16_384.0),
        Some(NaturalSpawnPreFinalizeMobFacts {
            remove_when_far_away: true,
            turtle_going_home: false,
            strider_is_in_lava: false,
        })
    );

    let cow = fresh_entity(VANILLA_ENTITY_TYPE_COW_ID);
    assert!(
        !natural_spawn_pre_finalize_mob_facts(&cow, 16_384.0)
            .unwrap()
            .remove_when_far_away
    );
}

#[test]
fn natural_spawn_fresh_fish_cat_ocelot_and_chicken_match_stateful_overrides() {
    let mut cod = fresh_entity(VANILLA_ENTITY_TYPE_COD_ID);
    assert!(
        natural_spawn_pre_finalize_mob_facts(&cod, 16_384.0)
            .unwrap()
            .remove_when_far_away
    );
    let EntityData::GeneralLiving(data) = &mut cod.data else {
        unreachable!()
    };
    assert!(data.set_from_bucket(true));
    assert!(
        !natural_spawn_pre_finalize_mob_facts(&cod, 16_384.0)
            .unwrap()
            .remove_when_far_away
    );

    let mut cat = fresh_entity(VANILLA_ENTITY_TYPE_CAT_ID);
    assert!(
        !natural_spawn_pre_finalize_mob_facts(&cat, 16_384.0)
            .unwrap()
            .remove_when_far_away
    );
    cat.tick_count = 2401;
    assert!(
        natural_spawn_pre_finalize_mob_facts(&cat, 16_384.0)
            .unwrap()
            .remove_when_far_away
    );

    for entity_type_id in [
        VANILLA_ENTITY_TYPE_OCELOT_ID,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ] {
        let entity = fresh_entity(entity_type_id);
        assert!(
            !natural_spawn_pre_finalize_mob_facts(&entity, 16_384.0)
                .unwrap()
                .remove_when_far_away
        );
    }
}

#[test]
fn natural_spawn_raider_uses_patrolling_distance() {
    let pillager = fresh_entity(VANILLA_ENTITY_TYPE_PILLAGER_ID);
    assert!(
        natural_spawn_pre_finalize_mob_facts(&pillager, 16_384.0)
            .unwrap()
            .remove_when_far_away
    );
    let mut patrol = fresh_entity(VANILLA_ENTITY_TYPE_PILLAGER_ID);
    let EntityData::GeneralLiving(data) = &mut patrol.data else {
        unreachable!()
    };
    *data.raider_retained_mut().unwrap() = crate::entity::raider::read_save_data(
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        &[entry("Patrolling", NbtSummaryValue::Byte(1))],
    );
    assert!(
        !natural_spawn_pre_finalize_mob_facts(&patrol, 16_384.0)
            .unwrap()
            .remove_when_far_away
    );
    assert!(
        natural_spawn_pre_finalize_mob_facts(&patrol, 16_384.01)
            .unwrap()
            .remove_when_far_away
    );
    let EntityData::GeneralLiving(data) = &mut patrol.data else {
        unreachable!()
    };
    data.set_raider_raid(crate::entity::raider::RaiderRaid::Unresolved(7));
    assert!(natural_spawn_pre_finalize_mob_facts(&patrol, 16_384.01).is_none());
    assert_eq!(patrol.remove_when_far_away(16_384.01), Some(true));

    // `Raider.java:234-235`: a current raid keeps the raider at any distance.
    let mut bound = fresh_entity(VANILLA_ENTITY_TYPE_PILLAGER_ID);
    let EntityData::GeneralLiving(data) = &mut bound.data else {
        unreachable!()
    };
    data.set_raider_raid(crate::entity::raider::RaiderRaid::Current(7));
    assert!(
        !natural_spawn_pre_finalize_mob_facts(&bound, 16_384.01)
            .unwrap()
            .remove_when_far_away
    );
}

#[test]
fn natural_spawn_turtle_and_strider_probe_facts_come_from_retained_state() {
    let turtle = fresh_entity(VANILLA_ENTITY_TYPE_TURTLE_ID);
    let facts = natural_spawn_pre_finalize_mob_facts(&turtle, 16_384.0).unwrap();
    assert!(!facts.turtle_going_home);
    assert!(!facts.strider_is_in_lava);

    let strider = fresh_entity(VANILLA_ENTITY_TYPE_STRIDER_ID);
    let facts = natural_spawn_pre_finalize_mob_facts(&strider, 16_384.0).unwrap();
    assert!(!facts.turtle_going_home);
    assert!(!facts.strider_is_in_lava);
}

#[test]
fn natural_spawn_remove_when_far_away_covers_every_owner_type() {
    for facts in VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1
        .iter()
        .filter(|facts| facts.is_mob)
    {
        let data = GeneralLivingData::new(facts.entity_type_id)
            .unwrap_or_else(|| panic!("{} lacks GeneralLiving state", facts.resource_id));
        let entity = Entity::new(
            1,
            Uuid::nil(),
            facts.entity_type_id,
            Vec3d::default(),
            EntityData::GeneralLiving(data),
        );
        let observed = natural_spawn_pre_finalize_mob_facts(&entity, 16_384.0)
            .unwrap_or_else(|| panic!("{} is not covered", facts.resource_id));
        let expected = match facts.resource_id {
            "minecraft:camel_husk"
            | "minecraft:zombie_horse"
            | "minecraft:ender_dragon"
            | "minecraft:hoglin"
            | "minecraft:nautilus"
            | "minecraft:zombie_nautilus" => Some(true),
            "minecraft:shulker" | "minecraft:tadpole" => Some(false),
            _ => None,
        };
        if let Some(expected) = expected {
            assert_eq!(
                observed.remove_when_far_away, expected,
                "{}",
                facts.resource_id
            );
        }
    }
}
