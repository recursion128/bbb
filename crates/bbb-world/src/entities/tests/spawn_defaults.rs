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

#[test]
fn projectile_vehicle_display_defaults_attach_only_generated_family_owners() {
    let mut store = WorldStore::new();
    let mut spawned = 0;
    for entry in VANILLA_ENTITY_TYPES {
        if spawn_default_family(entry.id) != SpawnDefaultFamily::ProjectileVehicleDisplay {
            continue;
        }
        let entity_id = 4_000 + entry.id;
        store.apply_add_entity(add_entity_with_uuid(
            entity_id,
            entry.id,
            uuid::Uuid::from_u128(0x2000_0000_0000_0000 + entry.id as u128),
        ));
        let metadata = &VANILLA_ENTITY_TYPE_METADATA_26_1[entry.id as usize];
        let entity = store.probe_entity(entity_id).unwrap();
        assert_eq!(entity.entity_type_id, entry.id, "{}", entry.resource_id);
        assert!(entity.data_values.is_empty(), "{}", entry.resource_id);
        assert!(entity.attributes.is_empty(), "{}", entry.resource_id);
        assert_eq!(
            entity.hurting_projectile.is_some(),
            metadata.is_hurting_projectile,
            "{}",
            entry.resource_id
        );
        assert_eq!(
            entity.projectile_lifecycle.is_some(),
            metadata.is_projectile && entry.serializable,
            "{}",
            entry.resource_id
        );
        assert_eq!(
            store.entities.minecart_lerp(entity_id).is_some(),
            metadata.is_minecart,
            "{}",
            entry.resource_id
        );
        assert_eq!(
            entity.client_animations.age_ticks, 0,
            "{}",
            entry.resource_id
        );
        assert_eq!(entity.client_animations.item_bob_offset, None);
        assert_eq!(entity.client_animations.end_crystal_time_offset, None);
        spawned += 1;
    }
    assert_eq!(spawned, 55);
}

#[test]
fn projectile_vehicle_display_mutations_reset_without_touching_retained_generation() {
    let minecart_id = 5_001;
    let fireball_id = 5_002;
    let trident_id = 5_003;
    let display_id = 5_004;
    let mut store = WorldStore::new();
    store.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec![
            "minecraft:vanilla".to_string(),
            "minecraft:minecart_improvements".to_string(),
        ],
    });
    for (entity_id, entity_type_id) in [
        (minecart_id, VANILLA_ENTITY_TYPE_MINECART_ID),
        (fireball_id, VANILLA_ENTITY_TYPE_FIREBALL_ID),
        (trident_id, VANILLA_ENTITY_TYPE_TRIDENT_ID),
        (display_id, VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(entity_id, entity_type_id));
    }
    let generation_a = store.publication_snapshot();

    assert!(
        store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: minecart_id,
            lerp_steps: vec![minecart_step(
                2.0, 65.0, -3.0, 0.25, 0.0, -0.25, 90.0, 5.0, 1.0,
            )],
        })
    );
    assert!(
        store.apply_projectile_power(bbb_protocol::packets::ProjectilePower {
            entity_id: fireball_id,
            acceleration_power: 0.35,
        })
    );
    let display_duration = vanilla_entity_metadata_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID,
        "DATA_POS_ROT_INTERPOLATION_DURATION_ID",
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: display_id,
        values: vec![protocol_int_data(display_duration.data_id, 7)],
    }));
    store.advance_entity_client_animations(1);

    assert_eq!(
        store
            .probe_entity(minecart_id)
            .unwrap()
            .minecart_lerp_pending_steps
            .len(),
        0
    );
    assert_eq!(
        store
            .probe_entity(minecart_id)
            .unwrap()
            .minecart_lerp_steps
            .len(),
        1
    );
    assert_eq!(
        store
            .probe_entity(fireball_id)
            .unwrap()
            .hurting_projectile
            .unwrap()
            .acceleration_power,
        0.35
    );
    assert!(
        store
            .probe_entity(trident_id)
            .unwrap()
            .projectile_lifecycle
            .unwrap()
            .has_been_shot
    );
    assert_eq!(store.probe_entity(display_id).unwrap().data_values.len(), 1);

    assert!(generation_a
        .probe_entity(minecart_id)
        .unwrap()
        .minecart_lerp_pending_steps
        .is_empty());
    assert_eq!(
        generation_a
            .probe_entity(fireball_id)
            .unwrap()
            .hurting_projectile
            .unwrap()
            .acceleration_power,
        0.1
    );
    assert!(
        !generation_a
            .probe_entity(trident_id)
            .unwrap()
            .projectile_lifecycle
            .unwrap()
            .has_been_shot
    );
    assert!(generation_a
        .probe_entity(display_id)
        .unwrap()
        .data_values
        .is_empty());

    for (entity_id, entity_type_id) in [
        (minecart_id, VANILLA_ENTITY_TYPE_MINECART_ID),
        (fireball_id, VANILLA_ENTITY_TYPE_FIREBALL_ID),
        (trident_id, VANILLA_ENTITY_TYPE_TRIDENT_ID),
        (display_id, VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(entity_id, entity_type_id));
        let entity = store.probe_entity(entity_id).unwrap();
        assert_eq!(entity.client_animations.age_ticks, 0);
        assert!(entity.data_values.is_empty());
    }
    assert!(store
        .probe_entity(minecart_id)
        .unwrap()
        .minecart_lerp_pending_steps
        .is_empty());
    assert!(store
        .probe_entity(minecart_id)
        .unwrap()
        .minecart_lerp_steps
        .is_empty());
    assert_eq!(
        store
            .probe_entity(fireball_id)
            .unwrap()
            .hurting_projectile
            .unwrap()
            .acceleration_power,
        0.1
    );
    assert!(
        !store
            .probe_entity(trident_id)
            .unwrap()
            .projectile_lifecycle
            .unwrap()
            .has_been_shot
    );
}

#[test]
fn living_passive_defaults_use_generated_attributes_and_fresh_body_yaw() {
    let mut store = WorldStore::new();
    let mut spawned = 0;
    for entry in VANILLA_ENTITY_TYPES {
        if spawn_default_family(entry.id) != SpawnDefaultFamily::LivingPassive {
            continue;
        }
        let entity_id = 6_000 + entry.id;
        store.apply_add_entity(add_entity_with_uuid(
            entity_id,
            entry.id,
            uuid::Uuid::from_u128(0x3000_0000_0000_0000 + entry.id as u128),
        ));
        let metadata = &VANILLA_ENTITY_TYPE_METADATA_26_1[entry.id as usize];
        let entity = store.probe_entity(entity_id).unwrap();
        assert!(metadata.is_living, "{}", entry.resource_id);
        assert!(
            !metadata.default_attributes.is_empty(),
            "{}",
            entry.resource_id
        );
        assert!(entity.attributes.is_empty(), "{}", entry.resource_id);
        assert!(entity.data_values.is_empty(), "{}", entry.resource_id);
        assert_eq!(
            entity.client_animations.spawn_body_y_rot,
            Some(30.0),
            "{}",
            entry.resource_id
        );
        assert_eq!(
            entity.client_animations.age_ticks, 0,
            "{}",
            entry.resource_id
        );
        assert_eq!(entity.client_animations.item_bob_offset, None);
        assert_eq!(entity.client_animations.end_crystal_time_offset, None);
        spawned += 1;
    }
    assert_eq!(spawned, 47);
}

#[test]
fn living_passive_tick_owners_reset_without_touching_retained_generation() {
    let chicken_id = 7_001;
    let squid_id = 7_002;
    let camel_id = 7_003;
    let horse_id = 7_004;
    let mut store = WorldStore::new();
    for (entity_id, entity_type_id) in [
        (chicken_id, VANILLA_ENTITY_TYPE_CHICKEN_ID),
        (squid_id, VANILLA_ENTITY_TYPE_SQUID_ID),
        (camel_id, VANILLA_ENTITY_TYPE_CAMEL_ID),
        (horse_id, VANILLA_ENTITY_TYPE_HORSE_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(entity_id, entity_type_id));
    }
    let generation_a = store.publication_snapshot();
    store.advance_entity_client_animations(1);

    assert!(store
        .probe_entity(chicken_id)
        .unwrap()
        .client_animations
        .chicken_flap
        .is_some());
    assert!(store
        .probe_entity(squid_id)
        .unwrap()
        .client_animations
        .squid
        .is_some());
    assert!(store
        .probe_entity(camel_id)
        .unwrap()
        .client_animations
        .camel_idle
        .is_some());
    assert!(store
        .probe_entity(horse_id)
        .unwrap()
        .client_animations
        .equine_tail
        .is_some());
    for entity_id in [chicken_id, squid_id, camel_id, horse_id] {
        assert_eq!(
            store
                .probe_entity(entity_id)
                .unwrap()
                .client_animations
                .age_ticks,
            1
        );
        assert_eq!(
            generation_a
                .probe_entity(entity_id)
                .unwrap()
                .client_animations
                .age_ticks,
            0
        );
    }
    assert!(generation_a
        .probe_entity(chicken_id)
        .unwrap()
        .client_animations
        .chicken_flap
        .is_none());
    assert!(generation_a
        .probe_entity(squid_id)
        .unwrap()
        .client_animations
        .squid
        .is_none());

    for (entity_id, entity_type_id) in [
        (chicken_id, VANILLA_ENTITY_TYPE_CHICKEN_ID),
        (squid_id, VANILLA_ENTITY_TYPE_SQUID_ID),
        (camel_id, VANILLA_ENTITY_TYPE_CAMEL_ID),
        (horse_id, VANILLA_ENTITY_TYPE_COW_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(entity_id, entity_type_id));
        let animations = store.probe_entity(entity_id).unwrap().client_animations;
        assert_eq!(animations.age_ticks, 0);
        assert_eq!(animations.spawn_body_y_rot, Some(30.0));
        assert!(animations.chicken_flap.is_none());
        assert!(animations.squid.is_none());
        assert!(animations.camel_idle.is_none());
        assert!(animations.equine_tail.is_none());
    }
}
