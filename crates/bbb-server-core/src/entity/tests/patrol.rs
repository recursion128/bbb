use super::native_level::NativeLevel;
use super::*;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_SLIME_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};

#[test]
fn patrol_uses_the_first_mob_rider_and_non_controlling_tag() {
    for (rider_type, controlling) in [
        (VANILLA_ENTITY_TYPE_ZOMBIE_ID, true),
        (VANILLA_ENTITY_TYPE_SLIME_ID, false),
    ] {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        let vehicle = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_PILLAGER_ID, Vec3d::default())
            .unwrap();
        let rider = store
            .spawn_general_living(rider_type, Vec3d::default())
            .unwrap();
        assert!(store.link_existing_passenger_forced(vehicle, rider));
        let mut level = NativeLevel::new();
        let passenger = level
            .with_level(|level| store.controlling_mob_passenger(store.get(vehicle).unwrap(), level))
            .unwrap();
        assert_eq!(passenger.is_some(), controlling);
    }
}

fn saved_field(store: &mut EntityStore, id: i32, name: &str, value: NbtSummaryValue) {
    let NbtSummaryValue::Compound(mut entries) = store.command_data(id).unwrap() else {
        panic!("entity compound");
    };
    entries.retain(|entry| entry.name != name);
    entries.push(bbb_protocol::NbtSummaryEntry {
        name: name.into(),
        value,
    });
    store.load_command_data(id, &entries).unwrap();
}

#[test]
fn patrol_input_distinguishes_controlled_vehicle_navigation_from_ordinary_riding() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let vehicle = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
        .unwrap();
    let rider = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PILLAGER_ID, Vec3d::default())
        .unwrap();
    assert!(store.link_existing_passenger_forced(vehicle, rider));
    let mut level = NativeLevel::new();
    let owner = |store: &EntityStore, level: &crate::world::simulation_level::LevelCtx<'_, '_>| {
        store.mob_control_owner(store.get(rider).unwrap(), level)
    };
    assert_eq!(
        level.with_level(|level| owner(&store, level)),
        Some(vehicle)
    );
    saved_field(&mut store, vehicle, "NoAI", NbtSummaryValue::Byte(1));
    assert_eq!(level.with_level(|level| owner(&store, level)), Some(rider));
    saved_field(&mut store, vehicle, "NoAI", NbtSummaryValue::Byte(0));
    for (membership, owned) in [(true, Some(rider)), (false, Some(vehicle))] {
        let members = if membership {
            &["minecraft:pillager"][..]
        } else {
            &[][..]
        };
        level.install_tags(registry_tag_snapshot(&[(
            "minecraft:entity_type",
            "minecraft:non_controlling_rider",
            members,
        )]));
        assert_eq!(level.with_level(|level| owner(&store, level)), owned);
    }
}
