use super::*;
use bbb_protocol::entity_types::{
    vanilla_entity_type, VanillaEntityMobCategory, VANILLA_ENTITY_TYPES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpawnDefaultFamily {
    BaseNonLiving,
    ProjectileVehicleDisplay,
    LivingPassive,
    HostileBoss,
    Player,
}

fn spawn_default_family(entity_type_id: i32) -> SpawnDefaultFamily {
    if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
        return SpawnDefaultFamily::Player;
    }
    let metadata = &VANILLA_ENTITY_TYPE_METADATA_26_1[entity_type_id as usize];
    if metadata.is_living {
        return if vanilla_entity_type(entity_type_id).unwrap().mob_category
            == VanillaEntityMobCategory::Monster
        {
            SpawnDefaultFamily::HostileBoss
        } else {
            SpawnDefaultFamily::LivingPassive
        };
    }
    let runtime_class = metadata.runtime_class.unwrap();
    if metadata.is_projectile
        || metadata.is_boat
        || metadata.is_minecart
        || runtime_class.contains(".projectile.")
        || runtime_class.contains(".Display")
        || runtime_class.contains(".decoration.")
    {
        SpawnDefaultFamily::ProjectileVehicleDisplay
    } else {
        SpawnDefaultFamily::BaseNonLiving
    }
}

fn model_source(store: &WorldStore, entity_id: i32) -> EntityModelSourceState {
    store
        .entity_model_sources_at_partial_tick(0.0)
        .into_iter()
        .find(|source| source.entity_id == entity_id)
        .unwrap()
}

fn add_entity_with_uuid(
    entity_id: i32,
    entity_type_id: i32,
    uuid: uuid::Uuid,
) -> ProtocolAddEntity {
    ProtocolAddEntity {
        uuid,
        ..protocol_add_entity_with_type(entity_id, entity_type_id)
    }
}

#[test]
fn generated_spawn_family_partition_covers_the_fixed_catalog_once() {
    let mut counts = [0_usize; 5];
    for entry in VANILLA_ENTITY_TYPES {
        assert_eq!(
            VANILLA_ENTITY_TYPE_METADATA_26_1[entry.id as usize].resource_id,
            entry.resource_id
        );
        counts[spawn_default_family(entry.id) as usize] += 1;
    }
    assert_eq!(counts, [10, 55, 47, 44, 1]);
}

#[test]
fn base_non_living_spawn_defaults_stay_sparse_and_type_owned() {
    let mut store = WorldStore::new();
    let mut spawned = 0;
    for entry in VANILLA_ENTITY_TYPES {
        if spawn_default_family(entry.id) != SpawnDefaultFamily::BaseNonLiving {
            continue;
        }
        let entity_id = 1_000 + entry.id;
        let uuid = uuid::Uuid::from_u128(0x1000_0000_0000_0000 + entry.id as u128);
        store.apply_add_entity(add_entity_with_uuid(entity_id, entry.id, uuid));
        let entity = store.probe_entity(entity_id).unwrap();
        assert_eq!(entity.entity_type_id, entry.id, "{}", entry.resource_id);
        assert!(entity.data_values.is_empty(), "{}", entry.resource_id);
        assert!(entity.attributes.is_empty(), "{}", entry.resource_id);
        assert!(entity.equipment.is_empty(), "{}", entry.resource_id);
        assert!(entity.passengers.is_empty(), "{}", entry.resource_id);
        assert!(entity.mob_effects.is_empty(), "{}", entry.resource_id);
        assert_eq!(
            entity.client_animations.age_ticks, 0,
            "{}",
            entry.resource_id
        );
        assert_eq!(
            entity.client_animations.item_bob_offset.is_some(),
            entry.id == VANILLA_ENTITY_TYPE_ITEM_ID,
            "{}",
            entry.resource_id
        );
        assert_eq!(
            entity.client_animations.end_crystal_time_offset.is_some(),
            entry.id == VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
            "{}",
            entry.resource_id
        );
        spawned += 1;
    }
    assert_eq!(spawned, 10);
}

#[test]
fn item_and_end_crystal_constructor_random_state_resets_and_is_generation_isolated() {
    let item_id = 2_001;
    let crystal_id = 2_002;
    let target_id = 2_003;
    let item_uuid = uuid::Uuid::from_u128(0x1111_2222_3333_4444_5555_6666_7777_8888);
    let crystal_uuid = uuid::Uuid::from_u128(0x9999_aaaa_bbbb_cccc_dddd_eeee_ffff_0001);
    let mut store = WorldStore::new();
    store.apply_add_entity(add_entity_with_uuid(
        item_id,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        item_uuid,
    ));
    store.apply_add_entity(add_entity_with_uuid(
        crystal_id,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        crystal_uuid,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        target_id,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: item_id,
        values: vec![item_stack_entity_data(item_stack(42, 3))],
    }));

    let item = store.probe_entity(item_id).unwrap();
    let bob_offset = item.client_animations.item_bob_offset.unwrap();
    assert!((0.0..std::f32::consts::TAU).contains(&bob_offset));
    let crystal = store.probe_entity(crystal_id).unwrap();
    let crystal_offset = crystal.client_animations.end_crystal_time_offset.unwrap();
    assert!(crystal_offset < 100_000);
    assert_eq!(model_source(&store, crystal_id).age_ticks, crystal_offset);
    assert_eq!(store.item_entity_stacks()[0].bob_offset, bob_offset);
    assert_eq!(
        store
            .take_item_entity_pickup_particle_state(item_id, target_id)
            .unwrap()
            .item_bob_offset,
        bob_offset
    );

    let generation_a = store.publication_snapshot();
    store.advance_entity_client_animations(3);
    assert_eq!(
        model_source(&store, crystal_id).age_ticks,
        crystal_offset + 3
    );
    assert_eq!(
        model_source(&generation_a, crystal_id).age_ticks,
        crystal_offset
    );
    assert_eq!(generation_a.item_entity_stacks()[0].bob_offset, bob_offset);

    store.apply_add_entity(add_entity_with_uuid(
        item_id,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        item_uuid,
    ));
    store.apply_add_entity(add_entity_with_uuid(
        crystal_id,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        crystal_uuid,
    ));
    assert_eq!(
        store
            .probe_entity(item_id)
            .unwrap()
            .client_animations
            .item_bob_offset,
        Some(bob_offset)
    );
    assert_eq!(
        store
            .probe_entity(crystal_id)
            .unwrap()
            .client_animations
            .end_crystal_time_offset,
        Some(crystal_offset)
    );
    assert_eq!(model_source(&store, crystal_id).age_ticks, crystal_offset);

    let replacement_uuid = uuid::Uuid::from_u128(0xaaaa_bbbb_cccc_dddd_eeee_ffff_0000_1111);
    store.apply_add_entity(add_entity_with_uuid(
        item_id,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        replacement_uuid,
    ));
    assert_ne!(
        store
            .probe_entity(item_id)
            .unwrap()
            .client_animations
            .item_bob_offset,
        Some(bob_offset)
    );
    assert_eq!(generation_a.item_entity_stacks()[0].bob_offset, bob_offset);
}

#[test]
fn constructor_random_state_survives_entity_store_serialization() {
    let mut store = WorldStore::new();
    store.apply_add_entity(add_entity_with_uuid(
        3_001,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        uuid::Uuid::from_u128(0x1234),
    ));
    store.apply_add_entity(add_entity_with_uuid(
        3_002,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        uuid::Uuid::from_u128(0x5678),
    ));
    let encoded = serde_json::to_vec(&store).unwrap();
    let decoded: WorldStore = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded.probe_entity(3_001), store.probe_entity(3_001));
    assert_eq!(decoded.probe_entity(3_002), store.probe_entity(3_002));
}
