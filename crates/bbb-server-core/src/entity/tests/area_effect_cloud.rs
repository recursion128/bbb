use super::native_level::{NativeLevel, DIMENSION};
use super::*;

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID;
use bbb_protocol::entity_types::{
    vanilla_entity_type_client_tracking_range, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_WITHER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::nbt_summary::{entry, NbtSummaryDecodeLimits};
use bbb_protocol::packets::{
    DataComponentPatchSummary, EntityDataValueKind, ItemStackSummary, PotionContentsSummary,
    SetEntityData,
};
use bbb_protocol::NbtSummaryEntry;
use bbb_registry_codec::persistent_item_stack_projection::potion_contents;
use bbb_registry_codec::RegistryElementValueRef;

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::area_effect_cloud::{
    self, AreaEffectCloudData, DATA_RADIUS_ID, DEFAULT_DURATION_ON_USE, DEFAULT_RADIUS,
    DEFAULT_RADIUS_ON_USE, DEFAULT_RADIUS_PER_TICK, DEFAULT_REAPPLICATION_DELAY, DEFAULT_WAIT_TIME,
    HEIGHT, INFINITE_DURATION,
};
use crate::entity::tracking::{
    non_default_synched_values, EntityTracker, ServerEntity, TrackingViewer,
};
use crate::entity::{
    spawn_area_effect_cloud, StoredDamageCausingEntity, StoredEntityDamageRules,
    StoredEntityDamageSource, StoredEntityHurtOutcome,
};

fn position() -> Vec3d {
    Vec3d {
        x: 1.5,
        y: 64.0,
        z: 2.5,
    }
}

fn cloud_data(store: &EntityStore, id: i32) -> AreaEffectCloudData {
    let EntityData::AreaEffectCloud(data) = store.get(id).expect("the cloud is live").data() else {
        panic!("expected an area effect cloud")
    };
    data.clone()
}

fn cloud_at(position: Vec3d, data: AreaEffectCloudData) -> (EntityStore, i32) {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = spawn_area_effect_cloud(&mut store, position, data).expect("a fresh UUID");
    (store, id)
}

fn damage_type(resource_id: &str) -> &'static bbb_server_datapack::damage::DamageTypeFacts {
    bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is a built-in damage type"))
}

fn field<'a>(record: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    record
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn water_potion() -> PotionContentsSummary {
    potion_contents::read(
        RegistryElementValueRef::Nbt(&NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "potion".into(),
            value: NbtSummaryValue::string("minecraft:water"),
        }])),
        NbtSummaryDecodeLimits::SAVED_FILE.max_depth,
    )
    .unwrap()
}

fn custom_effect_potion(resource_id: &str, amplifier: i8, duration: i32) -> PotionContentsSummary {
    potion_contents::read(
        RegistryElementValueRef::Nbt(&NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "custom_effects".into(),
            value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::string(resource_id),
                },
                NbtSummaryEntry {
                    name: "amplifier".into(),
                    value: NbtSummaryValue::Byte(amplifier),
                },
                NbtSummaryEntry {
                    name: "duration".into(),
                    value: NbtSummaryValue::Int(duration),
                },
            ])]),
        }])),
        NbtSummaryDecodeLimits::SAVED_FILE.max_depth,
    )
    .unwrap()
}

#[test]
fn a_spawned_cloud_carries_constructor_defaults_and_the_catalog_size() {
    let (store, id) = cloud_at(position(), AreaEffectCloudData::new());
    let entity = store.get(id).unwrap();
    assert_eq!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID
    );
    assert_eq!(entity.position(), position());
    assert_eq!(
        entity.bounding_box(),
        Aabb::of_entity(position(), DEFAULT_RADIUS * 2.0, HEIGHT)
    );
    assert_eq!(entity.gravity(), area_effect_cloud::DEFAULT_GRAVITY);
    let data = cloud_data(&store, id);
    assert_eq!(data.radius(), DEFAULT_RADIUS);
    assert!(!data.is_waiting());
    assert_eq!(data.duration(), INFINITE_DURATION);
    assert_eq!(data.wait_time(), DEFAULT_WAIT_TIME);
    assert_eq!(data.reapplication_delay(), DEFAULT_REAPPLICATION_DELAY);
    assert_eq!(data.duration_on_use(), DEFAULT_DURATION_ON_USE);
    assert_eq!(data.radius_on_use(), DEFAULT_RADIUS_ON_USE);
    assert_eq!(data.radius_per_tick(), DEFAULT_RADIUS_PER_TICK);
    assert_eq!(data.potion_duration_scale(), 1.0);
    assert!(data.owner().is_none());
    assert!(data.potion_contents().is_none());
    assert!(data.custom_particle().is_none());
    assert_eq!(data.age(), 0);
    const _: () = assert!(!area_effect_cloud::IS_PICKABLE);

    let rows = area_effect_cloud::synched_values(entity, &data);
    let entity_rows = crate::entity::living::entity_synched_values(entity).len();
    assert!(rows[entity_rows..].iter().all(|row| row.is_default));
    assert!(non_default_synched_values(entity).is_empty());
}

#[test]
fn save_restores_defaults_and_a_nonempty_duration() {
    let (store, id) = cloud_at(position(), AreaEffectCloudData::new());
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:area_effect_cloud"
    );
    let record = document.entities().next().unwrap().raw();
    assert_eq!(field(record, "Age"), Some(&NbtSummaryValue::Int(0)));
    assert_eq!(
        field(record, "Duration"),
        Some(&NbtSummaryValue::Int(INFINITE_DURATION))
    );
    assert_eq!(
        field(record, "WaitTime"),
        Some(&NbtSummaryValue::Int(DEFAULT_WAIT_TIME))
    );
    assert_eq!(
        field(record, "ReapplicationDelay"),
        Some(&NbtSummaryValue::Int(DEFAULT_REAPPLICATION_DELAY))
    );
    assert_eq!(
        field(record, "Radius"),
        Some(&NbtSummaryValue::Float(DEFAULT_RADIUS.to_bits()))
    );
    assert!(field(record, "potion_contents").is_none());
    assert!(field(record, "potion_duration_scale").is_none());
    assert!(field(record, "Owner").is_none());
    assert!(field(record, "custom_particle").is_none());

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get(ids[0]).unwrap();
    assert_eq!(after.uuid(), store.get(id).unwrap().uuid());
    assert_eq!(cloud_data(&reloaded, ids[0]).duration(), INFINITE_DURATION);

    let owner = uuid::Uuid::from_u128(0x1111_2222_3333_4444_5555_6666_7777_8888);
    let mut hidden = NbtSummaryValue::Compound(vec![
        entry("duration", NbtSummaryValue::Int(200)),
        entry("amplifier", NbtSummaryValue::Int(1)),
    ]);
    for _ in 1..65 {
        hidden = NbtSummaryValue::Compound(vec![
            entry("duration", NbtSummaryValue::Int(200)),
            entry("amplifier", NbtSummaryValue::Int(1)),
            entry("hidden_effect", hidden),
        ]);
    }
    let contents_nbt = NbtSummaryValue::Compound(vec![
        entry("potion", NbtSummaryValue::string("minecraft:water")),
        entry("custom_color", NbtSummaryValue::Int(1193046)),
        entry("custom_name", NbtSummaryValue::string("mist")),
        entry(
            "custom_effects",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::string("minecraft:wither")),
                entry("duration", NbtSummaryValue::Int(100)),
                entry("hidden_effect", hidden),
            ])]),
        ),
    ]);
    let contents = potion_contents::read(
        RegistryElementValueRef::Nbt(&contents_nbt),
        NbtSummaryDecodeLimits::SAVED_FILE.max_depth,
    )
    .unwrap();
    assert_eq!(
        std::iter::successors(
            contents.custom_effects[0].hidden_effect.as_deref(),
            |effect| { effect.hidden_effect.as_deref() }
        )
        .count(),
        65
    );
    assert!(
        bbb_registry_codec::persistent_item_stack_projection::read_item_component(
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
            &contents_nbt,
            crate::world::registry_runtime::item_stack_resolver_for_test(),
        )
        .is_err()
    );
    assert!(
        bbb_registry_codec::persistent_item_stack_projection::item_stack_summary_to_persistent_nbt(
            &ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_LINGERING_POTION_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                    potion_contents: contents.clone(),
                    ..Default::default()
                }
                .into(),
            },
        )
        .is_err()
    );
    let (store, id) = cloud_at(
        position(),
        AreaEffectCloudData::new()
            .with_duration(40)
            .with_wait_time(10)
            .with_radius(5.0)
            .with_radius_per_tick(-0.25)
            .with_owner(Some(owner))
            .with_potion_contents(Some(contents.clone()))
            .with_potion_duration_scale(0.5),
    );
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    let saved_limits = NbtSummaryDecodeLimits::SAVED_FILE;
    let bytes = bbb_protocol::nbt_summary::encode_named_nbt_summary_with_limits(
        "",
        &document.raw().to_summary(),
        1 << 20,
        saved_limits.max_depth,
        saved_limits.max_compound_entries,
    )
    .expect("the saved-file depth budget permits the cloud's hidden effects");
    let root = bbb_protocol::decode_file_saved_nbt_summary(&bytes)
        .unwrap()
        .unwrap();
    let document = bbb_server_persistence::Java26EntityChunkData::parse(
        bbb_server_persistence::NamedCompound::try_from_summary("", root).unwrap(),
        0,
        0,
    )
    .unwrap();
    let record = document.entities().next().unwrap().raw();
    assert_eq!(
        store
            .get(id)
            .unwrap()
            .loot_component(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID)
            .unwrap()
            .as_ref(),
        field(record, "potion_contents")
    );
    assert_eq!(field(record, "Duration"), Some(&NbtSummaryValue::Int(40)));
    assert_eq!(field(record, "WaitTime"), Some(&NbtSummaryValue::Int(10)));
    assert_eq!(
        field(record, "Radius"),
        Some(&NbtSummaryValue::Float(5.0_f32.to_bits()))
    );
    assert_eq!(
        field(record, "potion_duration_scale"),
        Some(&NbtSummaryValue::Float(0.5_f32.to_bits()))
    );
    assert!(field(record, "Owner").is_some());

    let mut reloaded = EntityStore::new(1, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let restored = cloud_data(&reloaded, ids[0]);
    assert_eq!(restored.duration(), 40);
    assert_eq!(restored.wait_time(), 10);
    assert_eq!(restored.radius(), 5.0);
    assert_eq!(restored.radius_per_tick(), -0.25);
    assert_eq!(restored.owner(), Some(owner));
    assert_eq!(restored.potion_contents(), Some(&contents));
    assert_eq!(restored.potion_duration_scale(), 0.5);
    assert_eq!(
        reloaded.get(ids[0]).unwrap().bounding_box(),
        Aabb::of_entity(position(), 10.0, HEIGHT)
    );

    assert_eq!(
        area_effect_cloud::read_save_data(&[]).duration(),
        INFINITE_DURATION
    );
    assert_eq!(
        area_effect_cloud::read_save_data(&[]).wait_time(),
        DEFAULT_WAIT_TIME
    );
    assert_eq!(
        area_effect_cloud::read_save_data(&[]).radius(),
        DEFAULT_RADIUS
    );
}

#[test]
fn wait_time_sets_the_waiting_flag_until_it_elapses() {
    let (mut store, id) = cloud_at(position(), AreaEffectCloudData::new());
    store.drain_lifecycle_events();
    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(cloud_data(&store, id).is_waiting());

    for _ in 0..19 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert!(!cloud_data(&store, id).is_waiting());
    assert!(store.get(id).unwrap().is_alive());
}

#[test]
fn a_finite_duration_discards_after_wait_plus_duration_ticks() {
    let (mut store, id) = cloud_at(
        position(),
        AreaEffectCloudData::new()
            .with_duration(5)
            .with_wait_time(20),
    );
    store.drain_lifecycle_events();
    let mut level = NativeLevel::new();
    for _ in 0..24 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert!(store.get(id).unwrap().is_alive());
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(store.get(id).is_none());
    assert!(store.drain_lifecycle_events().iter().any(|event| matches!(event, EntityLifecycleEvent::Removed { entity, .. } if entity.removal_reason() == Some(RemovalReason::Discarded))));
}

#[test]
fn radius_per_tick_shrinks_below_half_and_discards() {
    let (mut store, id) = cloud_at(
        position(),
        AreaEffectCloudData::new()
            .with_wait_time(0)
            .with_radius(0.6)
            .with_radius_per_tick(-0.2),
    );
    store.drain_lifecycle_events();
    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(store.get(id).is_none());
}

#[test]
fn hurt_server_is_false_and_does_not_mark() {
    let (mut store, id) = cloud_at(position(), AreaEffectCloudData::new());
    store.drain_lifecycle_events();
    let mut sink = LevelEventSink::default();
    for resource_id in [
        "minecraft:player_attack",
        "minecraft:out_of_world",
        "minecraft:generic_kill",
    ] {
        let outcome = store
            .hurt_server(
                id,
                StoredEntityDamageSource {
                    damage_type: damage_type(resource_id),
                    direct_entity_id: Some(77),
                    causing_entity: Some(crate::entity::StoredDamageCausingEntity::Player {
                        uuid: uuid::Uuid::nil(),
                        entity_id: 77,
                        instabuild: true,
                    }),
                    attacker_views: Default::default(),
                },
                1.0e9,
                Default::default(),
                crate::world::registry_runtime::item_stack_resolver_for_test(),
                &mut sink,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            outcome,
            StoredEntityHurtOutcome {
                applied: false,
                succeeded: false
            },
            "{resource_id}"
        );
    }
    let entity = store.get_mut(id).unwrap();
    assert!(entity.is_alive());
    assert!(!entity.take_hurt_marked());
    assert!(store.drain_lifecycle_events().is_empty());
}

#[test]
fn tracker_emits_add_and_the_synched_radius() {
    let (mut store, id) = cloud_at(position(), AreaEffectCloudData::new().with_radius(5.0));
    assert_eq!(
        vanilla_entity_type_client_tracking_range(VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID),
        Some(10)
    );
    let viewer = TrackingViewer {
        connection_id: ConnectionId(1),
        position: position(),
        view_distance: 32,
        uuid: uuid::Uuid::from_u128(1),
        entity_id: -1,
    };
    let mut tracker = EntityTracker::new();
    let mut paired = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[viewer],
        |_, _, _| true,
        &mut paired,
    );
    assert_eq!(tracker.viewers_of(id), vec![ConnectionId(1)]);
    let paired = crate::entity::tracking::recipient_packets(&paired)
        .map(|(recipient, packet)| (*recipient, packet))
        .collect::<Vec<_>>();
    assert!(matches!(
        paired.as_slice(),
        [
            (ConnectionId(1), ClientboundPlayPacket::BundleDelimiter),
            (ConnectionId(1), ClientboundPlayPacket::AddEntity(add)),
            (ConnectionId(1), ClientboundPlayPacket::SetEntityData(SetEntityData { id: sent_id, values })),
            (ConnectionId(1), ClientboundPlayPacket::BundleDelimiter),
        ] if add.id == id
            && add.entity_type_id == VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID
            && *sent_id == id
            && values.iter().any(|value|
                value.data_id == DATA_RADIUS_ID
                    && value.serializer_id == VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID
                    && value.value == EntityDataValueKind::Float(5.0))
    ));

    let entity = store.get_mut(id).unwrap();
    let mut server_entity = ServerEntity::new(entity, i32::MAX, false);
    area_effect_cloud::set_radius(entity, 4.0);
    let packets = server_entity.send_changes(entity);
    let Some(ClientboundPlayPacket::SetEntityData(SetEntityData {
        id: sent_id,
        values,
    })) = packets
        .iter()
        .find(|packet| matches!(packet, ClientboundPlayPacket::SetEntityData(_)))
    else {
        panic!("dirty radius publishes SetEntityData: {packets:?}")
    };
    assert_eq!(*sent_id, id);
    assert!(values.iter().any(|value| {
        value.data_id == DATA_RADIUS_ID && value.value == EntityDataValueKind::Float(4.0)
    }));
}

#[test]
fn nonempty_potion_contents_still_discard_on_duration_without_applying() {
    let (mut store, id) = cloud_at(
        position(),
        AreaEffectCloudData::new()
            .with_duration(1)
            .with_wait_time(0)
            .with_potion_contents(Some(water_potion())),
    );
    store.drain_lifecycle_events();
    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(store.get(id).is_none());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .is_empty());
}

#[test]
fn instant_damage_hits_once_inside_the_reapplication_window() {
    let mut store = EntityStore::new(8, EntityIdCounter::new());
    let target = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    let cloud = spawn_area_effect_cloud(
        &mut store,
        position(),
        AreaEffectCloudData::new()
            .with_wait_time(0)
            .with_duration(100)
            .with_radius_on_use(-0.25)
            .with_duration_on_use(-3)
            .with_potion_contents(Some(custom_effect_potion("minecraft:instant_damage", 1, 1))),
    )
    .unwrap();
    let before = store.get(target).unwrap().living().unwrap().health();
    let mut level = NativeLevel::new();
    for _ in 0..5 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert_eq!(
        store.get(target).unwrap().living().unwrap().health(),
        before - 6.0
    );
    assert_eq!(cloud_data(&store, cloud).radius(), 2.75);
    assert_eq!(cloud_data(&store, cloud).duration(), 97);

    for _ in 0..5 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert_eq!(
        store.get(target).unwrap().living().unwrap().health(),
        before - 6.0
    );
    assert_eq!(cloud_data(&store, cloud).radius(), 2.75);
    assert_eq!(cloud_data(&store, cloud).duration(), 97);
}

#[test]
fn wither_friends_are_immune_to_the_withers_indirect_magic() {
    let mut store = EntityStore::new(10, EntityIdCounter::new());
    let wither = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WITHER_ID, position())
        .unwrap();
    let causing = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    let source = StoredEntityDamageSource {
        damage_type: damage_type("minecraft:indirect_magic"),
        direct_entity_id: None,
        causing_entity: Some(StoredDamageCausingEntity::Mob { entity_id: causing }),
        attacker_views: Default::default(),
    };
    let mut level = NativeLevel::new();
    let wither_hurt_allowed =
        |level: &mut NativeLevel, store: &mut EntityStore, source: StoredEntityDamageSource<'_>| {
            let mut probe = store.get(wither).unwrap().clone();
            level
                .with_level(|level| {
                    crate::entity::general_living::hurt_server_with_runtime_in_level(
                        &mut probe,
                        store,
                        level,
                        source,
                        1.0,
                        StoredEntityDamageRules::default(),
                    )
                })
                .outcome
                .succeeded
        };
    level.install_tags(registry_tag_snapshot(&[]));
    assert!(wither_hurt_allowed(&mut level, &mut store, source));

    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:entity_type",
        "minecraft:wither_friends",
        &["minecraft:player", "minecraft:cow"],
    )]));
    let player_source = StoredEntityDamageSource {
        causing_entity: Some(StoredDamageCausingEntity::Player {
            uuid: uuid::Uuid::nil(),
            entity_id: 900,
            instabuild: false,
        }),
        ..source
    };
    let other_source = StoredEntityDamageSource {
        causing_entity: Some(StoredDamageCausingEntity::Other { entity_id: causing }),
        ..source
    };
    for source in [player_source, other_source] {
        assert!(!wither_hurt_allowed(&mut level, &mut store, source));
    }

    level.install_tags(registry_tag_snapshot(&[]));
    for source in [player_source, other_source] {
        assert!(wither_hurt_allowed(&mut level, &mut store, source));
    }
}

#[test]
fn wither_duration_is_scaled_before_the_stored_effect_is_added() {
    let mut store = EntityStore::new(9, EntityIdCounter::new());
    let target = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    spawn_area_effect_cloud(
        &mut store,
        position(),
        AreaEffectCloudData::new()
            .with_wait_time(0)
            .with_potion_duration_scale(0.5)
            .with_potion_contents(Some(custom_effect_potion("minecraft:wither", 1, 9))),
    )
    .unwrap();
    let mut level = NativeLevel::new();
    for _ in 0..5 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    let effect = store
        .get(target)
        .unwrap()
        .living()
        .unwrap()
        .active_effects()
        .next()
        .unwrap();
    assert_eq!(effect.resource_id(), "minecraft:wither");
    assert_eq!(effect.amplifier(), 1);
    assert_eq!(effect.duration(), 4);
}

#[test]
fn instant_damage_heals_undead_and_a_water_cloud_does_nothing() {
    let mut store = EntityStore::new(10, EntityIdCounter::new());
    let zombie = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    store
        .get_mut(zombie)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(5.0);
    spawn_area_effect_cloud(
        &mut store,
        position(),
        AreaEffectCloudData::new()
            .with_wait_time(0)
            .with_potion_contents(Some(custom_effect_potion("minecraft:instant_damage", 1, 1))),
    )
    .unwrap();
    let registry_position = Vec3d {
        x: 10.5,
        ..position()
    };
    let cow = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, registry_position)
        .unwrap();
    let cow_health = store.get(cow).unwrap().living().unwrap().health();
    spawn_area_effect_cloud(
        &mut store,
        registry_position,
        AreaEffectCloudData::new()
            .with_wait_time(0)
            .with_potion_contents(Some(water_potion())),
    )
    .unwrap();
    let mut level = NativeLevel::new();
    level.set(pos(0, 67, 0), state_id("minecraft:stone"));
    let (sender, receiver) = crate::tests::mailbox();
    sender
        .try_send(crate::world::WorldCommand::Admin {
            envelope: crate::admin::AdminEnvelope {
                operation_id: 1,
                request_id: "night".into(),
                principal: "fixture".into(),
                scope: crate::admin::AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 0,
            },
            request: crate::admin::AdminRequest::ExecuteCommand {
                command: "time set midnight".into(),
            },
        })
        .unwrap();
    level.world.tick(receiver.drain());
    for _ in 0..5 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert_eq!(
        store.get(zombie).unwrap().living().unwrap().health(),
        9.0,
        "damage={:?} fire={}",
        store
            .get(zombie)
            .unwrap()
            .living()
            .unwrap()
            .last_damage_type,
        store.get(zombie).unwrap().remaining_fire_ticks()
    );
    assert_eq!(
        store.get(cow).unwrap().living().unwrap().health(),
        cow_health
    );
}

#[test]
fn a_lethal_cloud_raises_entity_damage_and_entity_die() {
    let mut store = EntityStore::new(8, EntityIdCounter::new());
    let target = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    spawn_area_effect_cloud(
        &mut store,
        position(),
        AreaEffectCloudData::new()
            .with_wait_time(0)
            .with_duration(100)
            .with_potion_contents(Some(custom_effect_potion("minecraft:instant_damage", 2, 1))),
    )
    .unwrap();
    let _ = store.drain_lifecycle_events();
    let mut level = NativeLevel::new();

    for _ in 0..5 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }

    assert!(store
        .get(target)
        .unwrap()
        .living()
        .unwrap()
        .is_dead_or_dying());
    let block = store.get(target).unwrap().block_position();
    let raised = level.world.positioned_game_events_for_test(DIMENSION);
    for kind in [
        crate::level_event::VanillaGameEvent::ENTITY_DAMAGE,
        crate::level_event::VanillaGameEvent::ENTITY_DIE,
    ] {
        assert!(raised.iter().any(|event| event.recorded.game_event == kind
            && event.recorded.pos == block
            && event.recorded.source_entity_id == Some(target)));
    }
    assert_eq!(
        store
            .drain_lifecycle_events()
            .into_iter()
            .filter_map(|event| match event {
                EntityLifecycleEvent::Broadcast { entity_id, packet } => match packet {
                    ClientboundPlayPacket::EntityEvent(update)
                        if update.event_id == crate::entity::living::ENTITY_EVENT_DEATH =>
                        Some(entity_id),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![target]
    );
}

#[test]
fn a_cloud_syncs_its_custom_particle_or_its_potion_colour() {
    let particle = |data: AreaEffectCloudData| {
        let (store, id) = cloud_at(position(), data);
        non_default_synched_values(store.get(id).unwrap())
            .into_iter()
            .find(|value| value.data_id == area_effect_cloud::DATA_PARTICLE_ID)
            .map(|value| match value.value {
                EntityDataValueKind::Particle(particle) => particle,
                other => panic!("not a particle: {other:?}"),
            })
    };
    let breath = particle(AreaEffectCloudData::new().with_custom_particle(Some(
        crate::entity::hurting_projectile::dragon_fireball::dragon_breath_particle(),
    )))
    .expect("dragon breath is not the default particle");
    assert_eq!(
        breath.particle_type_id,
        bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_DRAGON_BREATH_ID
    );
    assert_eq!(breath.raw_options, 1.0_f32.to_be_bytes().to_vec());

    let poison = particle(
        AreaEffectCloudData::new().with_potion_contents(Some(custom_effect_potion(
            "minecraft:poison",
            0,
            100,
        ))),
    )
    .expect("a poison cloud is tinted");
    let colour =
        bbb_facts::mob_effect_types::vanilla_mob_effect_type_for_resource_id("minecraft:poison")
            .unwrap()
            .color as i32
            | 0xff00_0000_u32 as i32;
    assert_eq!(
        poison.particle_type_id,
        bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID
    );
    assert_eq!(poison.raw_options, colour.to_be_bytes().to_vec());
}
