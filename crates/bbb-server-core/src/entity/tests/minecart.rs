use super::*;
use crate::connection::ConnectionId;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

use crate::fluid::with_property;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
    VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID, VANILLA_FURNACE_MINECART_FUEL_DATA_ID,
    VANILLA_VEHICLE_DAMAGE_DATA_ID, VANILLA_VEHICLE_HURT_DIR_DATA_ID,
    VANILLA_VEHICLE_HURT_TIME_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID, VANILLA_ENTITY_TYPE_MINECART_ID,
};
use bbb_server_types::geometry::Vec3Ops;

use crate::level_event::VanillaGameEvent;

use crate::entity::minecart::{self, DAMAGE_TO_DESTROY, DEFAULT_DISPLAY_OFFSET, DEFAULT_GRAVITY};

fn native_cart(data: EntityData) -> (NativeLevel, i32) {
    let mut level = NativeLevel::new();
    let kind = vanilla_entity_type_id_for_resource_id(data.entity_resource_id()).unwrap();
    let (id, _) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        kind,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        data,
    );
    (level, id)
}

fn minecart_at(position: Vec3d) -> (EntityStore, i32) {
    let mut store = EntityStore::new(0x4d_49_4e_45, EntityIdCounter::new());
    let id = crate::entity::spawn_minecart(&mut store, position, 37.5).expect("a fresh minecart");
    (store, id)
}

fn data(store: &EntityStore, id: i32) -> crate::entity::MinecartData {
    match store.get(id).expect("entity is live").data() {
        EntityData::Minecart(data) => *data,
        other => panic!("expected a minecart, found {other:?}"),
    }
}

fn furnace_minecart_at(position: Vec3d) -> (EntityStore, i32) {
    let mut store = EntityStore::new(0x46_55_52_4e, EntityIdCounter::new());
    let id = crate::entity::spawn_furnace_minecart(&mut store, position, 22.5)
        .expect("a fresh furnace minecart");
    (store, id)
}

fn furnace_data(store: &EntityStore, id: i32) -> crate::entity::FurnaceMinecartData {
    match store.get(id).expect("entity is live").data() {
        EntityData::FurnaceMinecart(data) => *data,
        other => panic!("expected a furnace minecart, found {other:?}"),
    }
}

#[test]
fn minecart_constructs_with_catalog_dimensions_and_synched_defaults() {
    let (store, id) = minecart_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let entity = store.get(id).expect("entity is live");
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_MINECART_ID);
    assert_eq!(entity.data().entity_resource_id(), "minecraft:minecart");
    assert_eq!([entity.bb_width(), entity.bb_height()], [0.98, 0.7]);
    assert_eq!(entity.y_rot(), 37.5);
    assert_eq!(entity.gravity(), DEFAULT_GRAVITY);
    assert!(entity.is_pickable());
    let data = data(&store, id);
    assert_eq!(data.hurt_time(), 0);
    assert_eq!(data.hurt_dir(), 1);
    assert_eq!(data.damage(), 0.0);
    assert_eq!(data.custom_display_block(), None);
    assert_eq!(data.display_offset(), DEFAULT_DISPLAY_OFFSET);
}

#[test]
fn spawner_minecart_ignores_a_nearer_dead_player_and_uses_a_farther_live_player() {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    for (connection, name) in [(1, "alice"), (2, "bob")] {
        crate::tests::join_loaded(
            &sender,
            &receiver,
            &mut level.world,
            connection,
            name,
            u128::from(connection),
        );
    }
    level
        .world
        .set_player_position_for_test(ConnectionId(1), Vec3d { x: 8.5, ..position });
    level
        .world
        .set_player_position_for_test(ConnectionId(2), Vec3d { x: 1.5, ..position });
    level.world.set_player_dead_for_test(ConnectionId(2));
    let ids = EntityIdCounter::new();
    let highest = level
        .world
        .publication()
        .players
        .iter()
        .map(|player| player.entity_id)
        .max()
        .unwrap();
    for _ in 0..highest {
        ids.next_id();
    }
    let mut store = EntityStore::new(0x53_50_41_57, ids);
    let id = crate::entity::spawn_spawner_minecart(&mut store, position, 0.0).unwrap();
    crate::entity::spawner_minecart_data_mut(store.get_mut(id).unwrap())
        .unwrap()
        .set_spawn_delay(20);
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let EntityData::SpawnerMinecart(data) = store.get(id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(data.spawn_delay(), 19);
}

#[test]
fn off_rail_tick_applies_gravity_clamps_motion_and_decays_airborne_velocity() {
    let mut level = NativeLevel::new();

    let (mut store, id) = minecart_at(Vec3d {
        x: 0.5,
        y: 70.0,
        z: 0.5,
    });
    store
        .get_mut(id)
        .expect("entity is live")
        .set_delta_movement(Vec3d {
            x: 0.8,
            y: 0.0,
            z: -0.6,
        });
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let entity = store.get(id).expect("entity is live");
    assert_eq!(entity.position().x, 0.9);
    assert!((entity.position().z - 0.1).abs() < 1.0e-12);
    assert_eq!(entity.position().y, 70.0 - DEFAULT_GRAVITY);
    assert_eq!(entity.delta_movement().x, 0.4 * 0.95);
    assert_eq!(entity.delta_movement().z, -0.4 * 0.95);
    assert_eq!(entity.delta_movement().y, -DEFAULT_GRAVITY * 0.95);
    assert!(!data(&store, id).on_rails());
}

#[test]
fn legacy_minecart_move_scans_its_current_box_after_the_portal_cut() {
    let (mut store, id) = minecart_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let mut entity = store.detach(id).unwrap();
    entity.portal_cooldown = 1;
    entity.set_delta_movement(Vec3d {
        x: 0.8,
        y: 0.0,
        z: 0.0,
    });
    let entry = pos(0, 64, 0);
    let mut level = NativeLevel::new();
    level
        .world
        .set_block(DIMENSION, entry, state_id("minecraft:nether_portal"), 18);

    assert!(level.with_level(|ctx| minecart::tick(&mut entity, &mut store, ctx)));

    assert_eq!(entity.portal_cooldown(), 0);
    assert_eq!(entity.portal_process().unwrap().entry_position(), entry);
    assert_eq!(entity.position().x, 0.9);
}

#[test]
fn north_south_rail_aligns_motion_and_applies_natural_slowdown() {
    let rail = state_id("minecraft:rail");
    let mut level = NativeLevel::new();
    level.world.set_block(DIMENSION, pos(0, 64, 0), rail, 18);
    let (mut store, id) = minecart_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    store
        .get_mut(id)
        .expect("entity is live")
        .set_delta_movement(Vec3d {
            x: 0.1,
            y: 0.0,
            z: 0.2,
        });
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let entity = store.get(id).expect("entity is live");
    assert_eq!(entity.position().x, 0.5);
    assert!(entity.position().z > 0.5);
    assert_eq!(entity.delta_movement().x, 0.0);
    assert!(entity.delta_movement().z > 0.0);
    assert!(data(&store, id).on_rails());
}

#[test]
fn registry_tag_reload_requires_rails_membership() {
    let rail = state_id("minecraft:rail");
    let mut level = NativeLevel::new();
    level.world.set_block(DIMENSION, pos(0, 64, 0), rail, 18);
    let (mut store, id) = minecart_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.2,
        y: 0.0,
        z: 0.2,
    });

    let before = store.get(id).unwrap().clone();
    level.install_tags(registry_tag_snapshot(&[
        ("minecraft:block", "minecraft:rails", &[]),
        ("minecraft:fluid", "minecraft:water", &[]),
    ]));
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(!data(&store, id).on_rails());
    assert_ne!(store.get(id).unwrap().position(), before.position());
}

#[test]
fn registry_tagged_non_rail_never_enters_base_rail_dispatch() {
    let stone = state_id("minecraft:stone");
    let mut level = NativeLevel::new();
    level.world.set_block(DIMENSION, pos(0, 64, 0), stone, 18);
    level.install_tags(registry_tag_snapshot(&[
        ("minecraft:block", "minecraft:rails", &["minecraft:stone"]),
        ("minecraft:fluid", "minecraft:water", &[]),
    ]));
    let (mut store, id) = minecart_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    assert!(!data(&store, id).on_rails());
}

#[test]
fn powered_rail_accelerates_and_unpowered_rail_halts_a_slow_cart() {
    let default = state_id("minecraft:powered_rail");
    let powered = with_property(default, "powered", "true").expect("powered rail state");
    for (state, starts_moving) in [(powered, true), (default, false)] {
        let mut level = NativeLevel::new();
        if starts_moving {
            level.set(pos(0, 63, 0), state_id("minecraft:redstone_block"));
        }
        level.world.set_block(DIMENSION, pos(0, 64, 0), state, 18);
        level
            .world
            .set_block(DIMENSION, pos(0, 64, -1), state_id("minecraft:stone"), 18);
        let (mut store, id) = minecart_at(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        assert_eq!(
            store
                .get(id)
                .expect("entity is live")
                .delta_movement()
                .horizontal_distance_sqr()
                > 0.0,
            starts_moving
        );
    }
}

#[test]
fn metadata_defaults_and_custom_display_values_track_exact_ids() {
    let stone = state_id("minecraft:stone");
    let (mut store, id) = minecart_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    assert!(crate::entity::tracking::non_default_synched_values(
        store.get(id).expect("entity is live")
    )
    .is_empty());
    let entity = store.get_mut(id).expect("entity is live");
    minecart::set_custom_display_block(entity, Some(stone));
    minecart::set_display_offset(entity, 9);
    let dirty = entity.take_synched_dirty();
    assert_eq!(
        dirty,
        vec![
            VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
            VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID,
        ]
    );
    let values = crate::entity::tracking::non_default_synched_values(entity);
    assert_eq!(values.len(), 2);
    assert!(matches!(
        values[0],
        bbb_protocol::packets::EntityDataValue {
            data_id: VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
            value: bbb_protocol::packets::EntityDataValueKind::OptionalBlockState(Some(value)),
        } if value == stone
    ));
    assert_eq!(
        (values[1].data_id, values[1].serializer_id),
        (
            VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID
        )
    );
}

#[test]
fn custom_display_state_and_runtime_identity_round_trip_through_entity_regions() {
    let stone = state_id("minecraft:stone");
    let (mut store, id) = minecart_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let entity = store.get_mut(id).expect("entity is live");
    minecart::set_custom_display_block(entity, Some(stone));
    minecart::set_display_offset(entity, 11);
    let document = write_entity_chunk(&store, (0, 0)).expect("minecart save is canonical");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:minecart"
    );
    let mut restored = EntityStore::new(1, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut restored, &document).expect("minecart save restores");
    let data = data(&restored, ids[0]);
    assert_eq!(data.custom_display_block(), Some(stone));
    assert_eq!(data.display_offset(), 11);
}

#[test]
fn spawner_passenger_tree_saves_as_one_root_and_restores_vehicle_links() {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let mut store = EntityStore::new(0x53_50_41_57, EntityIdCounter::new());
    let vehicle_id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
            position,
        )
        .expect("pig vehicle constructs");
    let passenger_id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID,
            position,
        )
        .expect("chicken passenger constructs");
    assert!(store.link_existing_passenger(vehicle_id, passenger_id));

    let document = write_entity_chunk(&store, (0, 0)).expect("passenger tree is canonical");
    assert_eq!(document.entities().len(), 1);
    assert_eq!(document.entities().next().unwrap().id(), "minecraft:pig");
    assert_eq!(document.entities().next().unwrap().passengers().len(), 1);
    assert_eq!(
        document
            .entities()
            .next()
            .unwrap()
            .passengers()
            .next()
            .unwrap()
            .id(),
        "minecraft:chicken"
    );

    let mut restored = EntityStore::new(0x54_52_45_45, EntityIdCounter::new());
    let restored_roots = read_entity_chunk(&mut restored, &document).expect("tree restores");
    assert_eq!(restored_roots.len(), 1);
    let restored_vehicle_id = restored_roots[0];
    let restored_passengers = restored.passengers(restored_vehicle_id);
    assert_eq!(restored_passengers.len(), 1);
    assert_eq!(
        restored
            .get(restored_passengers[0])
            .unwrap()
            .entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID
    );
    assert_eq!(
        restored.vehicle(restored_passengers[0]),
        Some(restored_vehicle_id)
    );
}

#[test]
fn spawner_only_id_load_constructs_every_owned_general_living_type() {
    let mut failed = Vec::new();
    for facts in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
        if crate::entity::GeneralLivingData::new(facts.id).is_none() {
            continue;
        }
        let entity_nbt = vec![bbb_protocol::NbtSummaryEntry {
            name: "id".into(),
            value: bbb_protocol::NbtSummaryValue::string(facts.resource_id),
        }];
        let request = crate::entity::SpawnRequest {
            entity_type_id: facts.id,
            position: Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            spawn_data: crate::entity::SpawnerSpawnData::new(entity_nbt, None, None),
            spawner_position: BlockPos { x: 0, y: 64, z: 0 },
            spawn_range: 4,
            max_nearby_entities: 6,
        };
        let plan = crate::entity::prepare_spawn_request(&request).expect("only-id request decodes");
        let mut store = EntityStore::new(0x53_50_4e_42_54, EntityIdCounter::new());
        match crate::entity::load_fresh_entity_tree(&mut store, &plan) {
            Some(root) => store.discard_fresh(root),
            None => failed.push(facts.resource_id),
        }
    }
    assert!(
        failed.is_empty(),
        "only-id spawner load failed for {failed:?}"
    );
}

#[test]
fn destroying_a_minecart_emits_damage_and_die_events_and_drops_its_item() {
    let (mut level, id) = native_cart(EntityData::Minecart(crate::entity::MinecartData::new()));
    let target_uuid = level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .uuid();
    let (cause, cause_uuid) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::Minecart(crate::entity::MinecartData::new()),
    );
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        &format!(
            "damage {target_uuid} {} minecraft:generic by {cause_uuid}",
            DAMAGE_TO_DESTROY / 10.0 + 0.1
        ),
    );
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DAMAGE
                && event.recorded.source_entity_id == Some(cause)
                && event.recorded.pos == pos(0, 64, 0)
        ));

    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(level
        .world
        .stored_items_for_test(DIMENSION)
        .iter()
        .any(|item| item.2 == "minecraft:minecart" && item.3 == 1));
    let events = level.world.positioned_game_events_for_test(DIMENSION);
    for game_event in [
        VanillaGameEvent::ENTITY_DAMAGE,
        VanillaGameEvent::ENTITY_DIE,
    ] {
        assert!(events
            .iter()
            .any(|event| event.recorded.game_event == game_event
                && event.recorded.pos == pos(0, 64, 0)));
    }
    assert!(events.iter().any(
        |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
            && event.recorded.source_entity_id == Some(id)
    ));
}

#[test]
fn ordinary_removed_invulnerable_and_creative_damage_keep_vehicle_semantics() {
    let (mut level, id) = native_cart(EntityData::Minecart(crate::entity::MinecartData::new()));
    let outcome = level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:generic", 1.0, None)
        .unwrap();
    assert!(outcome.applied && outcome.succeeded);
    let entity = level.world.stored_entity_for_test(DIMENSION, id).unwrap();
    let EntityData::Minecart(data) = entity.data() else {
        unreachable!()
    };
    assert_eq!(
        (data.hurt_time(), data.hurt_dir(), data.damage()),
        (10, -1, 10.0)
    );
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_invulnerable(true);
    let refused = level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:generic", 1.0, None)
        .unwrap();
    assert!(!refused.applied && !refused.succeeded);
    let (mut creative, creative_id) =
        native_cart(EntityData::Minecart(crate::entity::MinecartData::new()));
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut creative.world, 1, "alice", 1);
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut creative.world,
        1,
        "gamemode creative alice",
    );
    let uuid = creative
        .world
        .stored_entity_for_test(DIMENSION, creative_id)
        .unwrap()
        .uuid();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut creative.world,
        2,
        &format!("damage {uuid} 1 minecraft:generic by alice"),
    );
    assert!(creative
        .world
        .stored_entity_for_test(DIMENSION, creative_id)
        .is_none());
    assert!(creative.world.stored_items_for_test(DIMENSION).is_empty());
    let (mut removed_store, removed_id) = minecart_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let mut removed = removed_store.detach(removed_id).unwrap();
    removed.set_removed(RemovalReason::Killed);
    let effects = minecart::hurt_server(
        &mut removed,
        1.0,
        crate::entity::MinecartHurtSource {
            creative_player: false,
            bypasses_invulnerability: false,
            is_fire: false,
            fire_immune: false,
            is_fall: false,
            fall_damage_immune: false,
            causing_entity_id: None,
        },
        true,
    );
    assert!(!effects.applied && effects.succeeded);
    assert!(!effects.emits_entity_damage);
}

#[test]
fn entity_drops_false_removes_the_minecart_without_adding_an_item() {
    let (mut level, id) = native_cart(EntityData::Minecart(crate::entity::MinecartData::new()));
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "gamerule minecraft:entity_drops false",
    );
    level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:generic", 5.0, None)
        .unwrap();
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
}

#[test]
fn fire_damage_arms_short_fuse_and_publishes_prime_sound_and_packet() {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "alice", 1);
    let position = level.world.publication().players[0].position;
    let (id, _) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
        position,
        EntityData::TntMinecart(crate::entity::TntMinecartData::new()),
    );
    level
        .world
        .apply_level_entity_lifecycle_for_test(DIMENSION, &mut Vec::new());
    let uuid = level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .uuid();
    let tick = crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        &format!("damage {uuid} 1 minecraft:in_fire by alice"),
    );
    let EntityData::TntMinecart(data) = level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .data()
    else {
        unreachable!()
    };
    assert!((0..40).contains(&data.fuse()));
    assert!(tick.events.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::EntityEvent(event), .. }
            | crate::world::WorldEvent::Broadcast { packet: crate::connection::ClientboundPlayPacket::EntityEvent(event), .. }
            if event.entity_id == id && event.event_id == crate::entity::minecart_tnt::PRIME_EVENT)));
    let sound_id = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.tnt.primed",
    )
    .unwrap();
    assert!(tick.events.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::Sound(sound), .. }
            | crate::world::WorldEvent::Broadcast { packet: crate::connection::ClientboundPlayPacket::Sound(sound), .. }
            if sound.sound == bbb_protocol::packets::SoundEventHolder::Reference { registry_id: sound_id })));
}

#[test]
fn burning_arrow_damage_explodes_the_cart_with_the_real_projectile_cause() {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "alice", 1);
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        10,
        "gamemode survival alice",
    );
    let position = level.world.publication().players[0].position;
    let (cart, uuid) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
        position,
        EntityData::TntMinecart(crate::entity::TntMinecartData::new()),
    );
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let temporary = crate::entity::spawn_arrow(
        &mut store,
        Vec3d {
            y: position.y + 2.0,
            ..position
        },
        Vec3d {
            x: 3.0,
            y: 4.0,
            z: 0.0,
        },
        plain_stack("minecraft:arrow", 1).unwrap(),
    )
    .unwrap();
    let (arrow, arrow_uuid) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
        Vec3d {
            y: position.y + 2.0,
            ..position
        },
        store.get(temporary).unwrap().data().clone(),
    );
    let entity = level
        .world
        .stored_entity_mut_for_test(DIMENSION, arrow)
        .unwrap();
    entity.set_remaining_fire_ticks(20);
    entity.set_delta_movement(Vec3d {
        x: 3.0,
        y: 4.0,
        z: 0.0,
    });
    let tick = crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        &format!("damage {uuid} 1 minecraft:generic by {arrow_uuid} from {arrow_uuid}"),
    );
    assert!(level
        .world
        .stored_entity_for_test(DIMENSION, cart)
        .is_none());
    assert!(tick.events.iter().any(|event| matches!(
        event,
        crate::world::WorldEvent::Send {
            packet: crate::connection::ClientboundPlayPacket::Explode(_),
            ..
        } | crate::world::WorldEvent::Broadcast {
            packet: crate::connection::ClientboundPlayPacket::Explode(_),
            ..
        }
    )));
    assert!(tick.events.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::DamageEvent(event), .. }
            if event.source_cause_id == arrow && event.source_direct_id == cart)));
}

#[test]
fn vehicle_metadata_rows_keep_the_vanilla_serializer_contract() {
    let (store, id) = minecart_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let entity = store.get(id).expect("entity is live");
    let rows = minecart::synched_values(entity, &data(&store, id));
    let tail: Vec<_> = rows[rows.len() - 5..]
        .iter()
        .map(|row| (row.value.data_id, row.value.serializer_id, row.is_default))
        .collect();
    assert_eq!(
        tail,
        vec![
            (
                VANILLA_VEHICLE_HURT_TIME_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                true
            ),
            (
                VANILLA_VEHICLE_HURT_DIR_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                true
            ),
            (
                VANILLA_VEHICLE_DAMAGE_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                true
            ),
            (
                VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
                true
            ),
            (
                VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                true
            ),
        ]
    );
}

#[test]
fn furnace_minecart_constructs_fuels_and_tracks_lit_state_after_tick() {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let (mut store, id) = furnace_minecart_at(position);
    let entity = store.get(id).unwrap();
    assert_eq!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
    );
    assert_eq!(
        entity.data().entity_resource_id(),
        "minecraft:furnace_minecart"
    );
    assert_eq!(entity.y_rot(), 22.5);
    assert!(!furnace_data(&store, id).has_fuel());

    let entity = store.get_mut(id).unwrap();
    assert!(!crate::entity::add_furnace_minecart_fuel(
        entity,
        position,
        "minecraft:stick"
    ));
    assert!(crate::entity::add_furnace_minecart_fuel(
        entity,
        Vec3d {
            z: -0.5,
            ..position
        },
        "minecraft:coal"
    ));
    assert_eq!(furnace_data(&store, id).fuel(), 3_600);
    assert_eq!(furnace_data(&store, id).push().z, 1.0);

    NativeLevel::new().with_level(|ctx| tick_entities(&mut store, ctx, true));
    let data = furnace_data(&store, id);
    assert_eq!(data.fuel(), 3_599);
    assert!(data.has_fuel());
    let rows = minecart::furnace_synched_values(store.get(id).unwrap(), &data);
    let fuel = rows.last().unwrap();
    assert_eq!(fuel.value.data_id, VANILLA_FURNACE_MINECART_FUEL_DATA_ID);
    assert_eq!(
        fuel.value.serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
    );
    assert!(!fuel.is_default);
}

#[test]
fn registry_tag_reload_changes_furnace_minecart_fuel_without_caching() {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let (mut store, id) = furnace_minecart_at(position);
    let mut level = NativeLevel::new();

    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:item",
        "minecraft:furnace_minecart_fuel",
        &["minecraft:stick"],
    )]));
    assert!(level.with_level(
        |ctx| crate::entity::minecart::add_furnace_minecart_fuel_in_level(
            store.get_mut(id).unwrap(),
            ctx,
            position,
            "minecraft:stick",
        )
    ));
    assert_eq!(furnace_data(&store, id).fuel(), 3_600);

    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:item",
        "minecraft:furnace_minecart_fuel",
        &[],
    )]));
    assert!(!level.with_level(
        |ctx| crate::entity::minecart::add_furnace_minecart_fuel_in_level(
            store.get_mut(id).unwrap(),
            ctx,
            position,
            "minecraft:stick",
        )
    ));
    assert_eq!(furnace_data(&store, id).fuel(), 3_600);
}

#[test]
fn furnace_minecart_rail_tick_uses_push_and_subclass_speed_limit() {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let (mut store, id) = furnace_minecart_at(position);
    let entity = store.get_mut(id).unwrap();
    entity.set_delta_movement(Vec3d {
        x: 0.0,
        y: 0.0,
        z: 0.4,
    });
    assert!(crate::entity::add_furnace_minecart_fuel(
        entity,
        Vec3d {
            z: -0.5,
            ..position
        },
        "minecraft:charcoal"
    ));
    let mut level = NativeLevel::new();
    level
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:rail"), 18);
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let entity = store.get(id).unwrap();
    assert!((entity.position().z - 0.7).abs() < 1.0e-12);
    assert!(entity.delta_movement().z > 0.2);
}

#[test]
fn furnace_minecart_small_push_is_not_projected_below_vanilla_threshold() {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let (mut store, id) = furnace_minecart_at(position);
    let entity = store.get_mut(id).unwrap();
    entity.set_delta_movement(Vec3d {
        x: 0.0,
        y: 0.0,
        z: 0.2,
    });
    assert!(crate::entity::add_furnace_minecart_fuel(
        entity,
        Vec3d {
            x: position.x - 0.001,
            ..position
        },
        "minecraft:coal"
    ));
    let mut level = NativeLevel::new();
    level
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:rail"), 18);
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let entity = store.get(id).unwrap();
    assert!((furnace_data(&store, id).push().x - 0.001).abs() < 1.0e-12);
    assert!(entity.delta_movement().x > 0.0);
}

#[test]
fn furnace_minecart_fuel_cap_fails_closed_before_consumption() {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let (mut store, id) = furnace_minecart_at(position);
    let entity = store.get_mut(id).unwrap();
    for _ in 0..8 {
        assert!(crate::entity::add_furnace_minecart_fuel(
            entity,
            position,
            "minecraft:coal"
        ));
    }
    assert_eq!(
        match entity.data() {
            EntityData::FurnaceMinecart(data) => data.fuel(),
            other => panic!("expected furnace minecart, found {other:?}"),
        },
        28_800
    );
    assert!(!crate::entity::add_furnace_minecart_fuel(
        entity,
        position,
        "minecraft:charcoal"
    ));
}

#[test]
fn furnace_minecart_save_round_trips_fuel_push_and_base_state() {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let (mut store, id) = furnace_minecart_at(position);
    let entity = store.get_mut(id).unwrap();
    assert!(crate::entity::add_furnace_minecart_fuel(
        entity,
        Vec3d {
            x: -1.5,
            ..position
        },
        "minecraft:coal"
    ));
    minecart::set_display_offset(entity, 9);
    let document = write_entity_chunk(&store, (0, 0)).unwrap();
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:furnace_minecart"
    );
    let mut restored = EntityStore::new(0x52_45_53_54, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut restored, &document).unwrap();
    let data = furnace_data(&restored, ids[0]);
    assert_eq!(data.fuel(), 3_600);
    assert_eq!(data.push().x, 2.0);
    assert_eq!(data.minecart().display_offset(), 9);
    assert!(!data.has_fuel());
}

#[test]
fn furnace_minecart_damage_drops_its_subclass_item() {
    let (mut store, id) = furnace_minecart_at(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    let mut entity = store.detach(id).unwrap();
    let effects = minecart::hurt_server(
        &mut entity,
        5.0,
        crate::entity::MinecartHurtSource {
            creative_player: false,
            bypasses_invulnerability: false,
            is_fire: false,
            fire_immune: false,
            is_fall: false,
            fall_damage_immune: false,
            causing_entity_id: None,
        },
        true,
    );
    assert_eq!(entity.removal_reason(), Some(RemovalReason::Killed));
    assert_eq!(
        effects.drops,
        vec![plain_stack("minecraft:furnace_minecart", 1).unwrap()]
    );
}

#[test]
fn a_saved_minecart_moving_into_water_splashes() {
    let (mut store, _) = minecart_at(Vec3d {
        x: 8.4,
        y: 64.0,
        z: 0.5,
    });
    let mut dry = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            dry.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    dry.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let document = write_entity_chunk(&store, (0, 0)).unwrap();
    let mut restored = EntityStore::new(1, EntityIdCounter::new());
    let id = read_entity_chunk(&mut restored, &document).unwrap()[0];
    restored.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.4,
        y: 0.0,
        z: 0.0,
    });
    restored.drain_lifecycle_events();
    let mut wet = dry;
    wet.world
        .set_block(DIMENSION, pos(9, 64, 0), state_id("minecraft:water"), 18);
    wet.with_level(|ctx| tick_entities(&mut restored, ctx, true));
    assert!(restored.get(id).unwrap().is_in_water());
    assert!(wet
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::SPLASH));
    assert!(restored
        .drain_lifecycle_events()
        .iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
    wet.with_level(|ctx| tick_entities(&mut restored, ctx, true));
}

#[test]
fn a_destroyed_named_minecart_drops_an_item_with_its_name() {
    let (mut level, id) = native_cart(EntityData::Minecart(crate::entity::MinecartData::new()));
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_custom_name(Some(bbb_protocol::RichTextComponent::plain("Cart")));
    level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:generic", 5.0, None)
        .unwrap();
    let items = level.world.stored_items_for_test(DIMENSION);
    let [(item, ..)] = items.as_slice() else {
        panic!("one drop, got {items:?}");
    };
    let EntityData::Item(item) = level
        .world
        .stored_entity_for_test(DIMENSION, *item)
        .unwrap()
        .data()
    else {
        unreachable!()
    };
    assert_eq!(
        item.item().summary().component_patch.custom_name.as_deref(),
        Some("Cart")
    );
}

fn rail_track(level: &mut NativeLevel) {
    let rail = state_id("minecraft:rail");
    for z in 0..16 {
        level.world.set_block(DIMENSION, pos(0, 64, z), rail, 18);
    }
}

fn along_track(z: f64) -> Vec3d {
    Vec3d { x: 0.5, y: 64.0, z }
}

fn idle_mob(store: &mut EntityStore, uuid: u128, kind: i32, z: f64) -> i32 {
    let mut data = crate::entity::GeneralLivingData::new(kind).unwrap();
    data.set_no_ai(true);
    store
        .restore(
            uuid::Uuid::from_u128(uuid),
            kind,
            along_track(z),
            EntityData::GeneralLiving(data),
        )
        .unwrap()
}

fn launched_cart(store: &mut EntityStore, speed: f64) -> i32 {
    let id = crate::entity::spawn_minecart(store, along_track(0.5), 0.0).unwrap();
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: 0.0,
        z: speed,
    });
    id
}

fn ridden_cart(store: &mut EntityStore, speed: f64) -> i32 {
    let cart = launched_cart(store, speed);
    let rider = idle_mob(
        store,
        0x52_49_44_45,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID,
        0.5,
    );
    assert!(store.link_existing_passenger(cart, rider));
    cart
}

#[test]
fn a_ridden_cart_coasts_farther_than_an_empty_one() {
    const TICKS: i32 = 40;
    // OldMinecartBehavior.java:207-210 and :411-414: a ridden cart moves
    // 0.75 of its speed and keeps 0.997 of it, an empty one all and 0.96.
    let expected = |scale: f64, keep: f64| scale * 0.3 * (1.0 - keep.powi(TICKS)) / (1.0 - keep);
    for (ridden, distance) in [(true, expected(0.75, 0.997)), (false, expected(1.0, 0.96))] {
        let mut level = NativeLevel::new();
        rail_track(&mut level);
        let mut store = EntityStore::new(0x43_4f_41_53, EntityIdCounter::new());
        let cart = if ridden {
            ridden_cart(&mut store, 0.3)
        } else {
            launched_cart(&mut store, 0.3)
        };
        for _ in 0..TICKS {
            level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        }
        let travelled = store.get(cart).unwrap().position().z - 0.5;
        assert!(
            (travelled - distance).abs() < 0.02,
            "ridden={ridden}: {travelled} vs {distance}"
        );
    }
}

#[test]
fn a_ridden_cart_stops_at_a_cow_on_the_rail() {
    let mut level = NativeLevel::new();
    rail_track(&mut level);
    let mut store = EntityStore::new(0x43_4f_57_53, EntityIdCounter::new());
    let cart = ridden_cart(&mut store, 0.4);
    let cow = idle_mob(
        &mut store,
        0x43_4f_57,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        5.5,
    );
    for _ in 0..40 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        let cart_box = store.get(cart).unwrap().bounding_box();
        let cow_box = store.get(cow).unwrap().bounding_box();
        assert!(
            cart_box.max_z <= cow_box.min_z + 1.0e-6,
            "{cart_box:?} {cow_box:?}"
        );
    }
}

#[test]
fn two_carts_on_one_rail_never_overlap() {
    let mut level = NativeLevel::new();
    rail_track(&mut level);
    let mut store = EntityStore::new(0x54_57_4f_43, EntityIdCounter::new());
    let moving = ridden_cart(&mut store, 0.4);
    let parked = crate::entity::spawn_minecart(&mut store, along_track(6.5), 0.0).unwrap();
    for _ in 0..40 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        let behind = store.get(moving).unwrap().bounding_box();
        let ahead = store.get(parked).unwrap().bounding_box();
        assert!(behind.max_z <= ahead.min_z + 1.0e-6, "{behind:?} {ahead:?}");
    }
}

/// `CactusBlock.entityInside` hurts any entity; a cart under its own tick
/// banks the damage until `VehicleEntity.hurtServer` destroys it.
#[test]
fn a_minecart_inside_cactus_breaks_under_its_own_tick() {
    let (mut level, id) = native_cart(EntityData::Minecart(crate::entity::MinecartData::new()));
    level.set(
        pos(0, 63, 0),
        crate::entity::tests::native_level::state("minecraft:sand"),
    );
    level.set(
        pos(0, 64, 0),
        crate::entity::tests::native_level::state("minecraft:cactus"),
    );
    for _ in 0..20 {
        level.world.tick(Vec::new());
        if level.world.stored_entity_for_test(DIMENSION, id).is_none() {
            break;
        }
    }
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(level
        .world
        .stored_items_for_test(DIMENSION)
        .iter()
        .any(|item| item.2 == "minecraft:minecart"));
}

/// `LavaFluid.entityInside` -> `Entity.lavaHurt`: 4 damage banks 40 per tick.
#[test]
fn a_minecart_in_lava_breaks_under_its_own_tick() {
    let (mut level, id) = native_cart(EntityData::Minecart(crate::entity::MinecartData::new()));
    level.set(
        pos(0, 64, 0),
        crate::entity::tests::native_level::state("minecraft:lava"),
    );
    for _ in 0..3 {
        level.world.tick(Vec::new());
    }
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
}
