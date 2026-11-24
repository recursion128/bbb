use super::*;
#[test]
fn duplicate_uuid_records_refuse_while_supported_nonliving_restores() {
    let uuid = Uuid::from_u128(0x5678);
    let mut store = EntityStore::new(5, EntityIdCounter::new());

    let mut nonliving = base_entity("minecraft:ominous_item_spawner", Uuid::from_u128(0x6789));
    nonliving.push(entry("spawn_item_after_ticks", NbtSummaryValue::Long(20)));
    let nonliving_id = read_entity_chunk(&mut store, &chunk(nonliving)).unwrap()[0];
    let EntityData::OminousItemSpawner(data) = store.get(nonliving_id).unwrap().data() else {
        unreachable!("restored Ominous Item Spawner keeps its specialized runtime")
    };
    assert!(data.item().is_empty());
    assert_eq!(data.spawn_item_after_ticks(), 20);

    let valid = chunk(zombie_record(uuid));
    read_entity_chunk(&mut store, &valid).unwrap();
    let ids = store.ids();
    assert!(read_entity_chunk(&mut store, &valid).unwrap().is_empty());
    assert_eq!(store.ids(), ids);
}

#[test]
fn tamable_despawn_state_save_restore_and_defaults() {
    let owner = Uuid::from_u128(0xFA00);
    for (index, (resource_id, entity_type_id)) in [
        ("minecraft:cat", VANILLA_ENTITY_TYPE_CAT_ID),
        ("minecraft:wolf", VANILLA_ENTITY_TYPE_WOLF_ID),
    ]
    .into_iter()
    .enumerate()
    {
        let fresh = GeneralLivingData::new(entity_type_id).unwrap();
        let fresh_tamable = fresh.tamable_retained().unwrap();
        assert_eq!(fresh_tamable.flags(), 0);
        assert_eq!(fresh_tamable.owner(), None);
        assert!(!fresh_tamable.ordered_to_sit());

        let mut store = EntityStore::new(0xFA + index as i64, EntityIdCounter::new());
        let id = read_entity_chunk(
            &mut store,
            &chunk(tamable_record(
                resource_id,
                Uuid::from_u128(0xFA10 + index as u128),
                Some(owner),
                Some(true),
            )),
        )
        .unwrap()[0];
        let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
            unreachable!("tamable uses general living")
        };
        let tamable = data.tamable_retained().unwrap();
        assert_eq!(tamable.flags(), 5);
        assert_eq!(tamable.owner(), Some(owner));
        assert!(tamable.ordered_to_sit());
        assert!(data
            .opaque_fields()
            .iter()
            .all(|field| !matches!(field.name.to_jdk_string().as_str(), "Owner" | "Sitting")));

        let saved = write_entity_chunk(&store, (0, 0)).unwrap();
        let fields = saved.entities().next().unwrap().raw();
        assert_eq!(
            fields
                .iter()
                .find(|field| field.name == "Owner")
                .map(|field| &field.value),
            Some(&saved_uuid(owner))
        );
        assert!(matches!(
            fields
                .iter()
                .find(|field| field.name == "Sitting")
                .map(|field| &field.value),
            Some(NbtSummaryValue::Byte(1))
        ));
    }
    assert!(GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID)
        .unwrap()
        .tamable_retained()
        .is_none());
}

#[test]
fn tamable_despawn_sparse_restore() {
    let mut sparse = EntityStore::new(0xFB, EntityIdCounter::new());
    let id = read_entity_chunk(
        &mut sparse,
        &chunk(tamable_record(
            "minecraft:cat",
            Uuid::from_u128(0xFB01),
            None,
            None,
        )),
    )
    .unwrap()[0];
    let EntityData::GeneralLiving(data) = sparse.get(id).unwrap().data() else {
        unreachable!("cat uses general living")
    };
    let tamable = data.tamable_retained().unwrap();
    assert_eq!((tamable.flags(), tamable.owner()), (0, None));
    assert!(!tamable.ordered_to_sit());
    let saved = write_entity_chunk(&sparse, (0, 0)).unwrap();
    assert!(saved
        .entities()
        .next()
        .unwrap()
        .raw()
        .iter()
        .all(|field| field.name != "Owner"));
    assert!(matches!(
        saved
            .entities()
            .next()
            .unwrap()
            .raw()
            .iter()
            .find(|field| field.name == "Sitting")
            .map(|field| &field.value),
        Some(NbtSummaryValue::Byte(0))
    ));
}

#[test]
fn tamable_despawn_cat_and_nautilus_apply_distinct_retention_rules() {
    let player = Vec3d {
        x: 200.5,
        y: 64.0,
        z: 0.5,
    };
    let owner = Uuid::from_u128(0xFC00);
    let mut store = EntityStore::new(0xFD, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    let untamed_cat = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CAT_ID, position())
        .unwrap();
    store.get_mut(untamed_cat).unwrap().tick_count = 2400;
    let tame_cat = read_entity_chunk(
        &mut store,
        &chunk(tamable_record(
            "minecraft:cat",
            Uuid::from_u128(0xFC01),
            Some(owner),
            Some(false),
        )),
    )
    .unwrap()[0];
    let mut tame_nautilus = Vec::new();
    let mut untamed_nautilus = Vec::new();
    for (index, (resource_id, entity_type_id)) in [
        ("minecraft:nautilus", VANILLA_ENTITY_TYPE_NAUTILUS_ID),
        (
            "minecraft:zombie_nautilus",
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        tame_nautilus.push(
            read_entity_chunk(
                &mut store,
                &chunk(tamable_record(
                    resource_id,
                    Uuid::from_u128(0xFC10 + index as u128),
                    Some(owner),
                    Some(false),
                )),
            )
            .unwrap()[0],
        );
        untamed_nautilus.push(
            store
                .spawn_general_living(entity_type_id, position())
                .unwrap(),
        );
    }
    let parrot = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PARROT_ID, position())
        .unwrap();
    let wolf = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WOLF_ID, position())
        .unwrap();
    for id in store.ids() {
        store
            .get_mut(id)
            .unwrap()
            .living_mut()
            .unwrap()
            .no_action_time = 700;
    }

    {
        let mut native = {
            let mut native = NativeLevel::new();
            let (sender, receiver) = crate::tests::mailbox();
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut native.world,
                1,
                "player",
                0xC0DE_C0DE,
            );
            native
                .world
                .set_player_position_for_test(crate::connection::ConnectionId(1), player);
            native.world.set_player_mode_and_abilities_for_test(
                crate::connection::ConnectionId(1),
                bbb_protocol::game_types::GameType::Survival,
                bbb_protocol::packets::PlayerAbilities::for_game_type(
                    bbb_protocol::game_types::GameType::Survival,
                ),
            );
            native
        };
        native.with_level(|level| tick_entities(&mut store, level, true))
    };
    assert!(store.get(untamed_cat).is_some());

    assert!(store.get(tame_cat).is_some());
    for id in &tame_nautilus {
        assert!(store.get(*id).is_some());
    }
    for id in &untamed_nautilus {
        assert!(store.get(*id).is_none());
    }
    assert!(store.get(parrot).is_some());
    assert!(store.get(wolf).is_some());

    store.get_mut(untamed_cat).unwrap().tick_count = 2401;
    {
        let mut native = {
            let mut native = NativeLevel::new();
            let (sender, receiver) = crate::tests::mailbox();
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut native.world,
                1,
                "player",
                0xC0DE_C0DE,
            );
            native
                .world
                .set_player_position_for_test(crate::connection::ConnectionId(1), player);
            native.world.set_player_mode_and_abilities_for_test(
                crate::connection::ConnectionId(1),
                bbb_protocol::game_types::GameType::Survival,
                bbb_protocol::packets::PlayerAbilities::for_game_type(
                    bbb_protocol::game_types::GameType::Survival,
                ),
            );
            native
        };
        native.with_level(|level| tick_entities(&mut store, level, true))
    };
    assert!(store.get(untamed_cat).is_none());
}

#[test]
fn conditional_animal_despawn_state_save_restore_and_defaults() {
    for (index, (resource_id, entity_type_id, field)) in [
        (
            "minecraft:chicken",
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            "IsChickenJockey",
        ),
        (
            "minecraft:ocelot",
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            "Trusting",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let saved_value = |data: &GeneralLivingData| {
            if entity_type_id == VANILLA_ENTITY_TYPE_CHICKEN_ID {
                data.chicken_jockey()
            } else {
                data.ocelot_trusting()
            }
        };
        let fresh = GeneralLivingData::new(entity_type_id).unwrap();
        assert_eq!(saved_value(&fresh), Some(false));

        let mut store = EntityStore::new(0xA10 + index as i64, EntityIdCounter::new());
        let id = read_entity_chunk(
            &mut store,
            &chunk(conditional_animal_record(
                resource_id,
                Uuid::from_u128(0xA100 + index as u128),
                Some(true),
            )),
        )
        .unwrap()[0];
        let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
            unreachable!("conditional animal uses general living")
        };
        assert_eq!(saved_value(data), Some(true));
        assert!(data.opaque_fields().iter().all(|entry| entry.name != field));

        let saved = write_entity_chunk(&store, (0, 0)).unwrap();
        assert!(matches!(
            saved
                .entities()
                .next()
                .unwrap()
                .raw()
                .iter()
                .find(|entry| entry.name == field)
                .map(|entry| &entry.value),
            Some(NbtSummaryValue::Byte(1))
        ));
    }
}

#[test]
fn conditional_animal_despawn_sparse_restore() {
    for (index, resource_id) in ["minecraft:chicken", "minecraft:ocelot"]
        .into_iter()
        .enumerate()
    {
        let mut sparse = EntityStore::new(0xA20 + index as i64, EntityIdCounter::new());
        let id = read_entity_chunk(
            &mut sparse,
            &chunk(conditional_animal_record(
                resource_id,
                Uuid::from_u128(0xA200 + index as u128),
                None,
            )),
        )
        .unwrap()[0];
        let EntityData::GeneralLiving(data) = sparse.get(id).unwrap().data() else {
            unreachable!("conditional animal uses general living")
        };
        let (value, field) = if resource_id == "minecraft:chicken" {
            (data.chicken_jockey(), "IsChickenJockey")
        } else {
            (data.ocelot_trusting(), "Trusting")
        };
        assert_eq!(value, Some(false));
        let saved = write_entity_chunk(&sparse, (0, 0)).unwrap();
        assert!(matches!(
            saved
                .entities()
                .next()
                .unwrap()
                .raw()
                .iter()
                .find(|entry| entry.name == field)
                .map(|entry| &entry.value),
            Some(NbtSummaryValue::Byte(0))
        ));
    }
}

#[test]
fn conditional_animal_despawn_applies_jockey_trusting_and_age_edges_without_reset() {
    let mut store = EntityStore::new(0xA30, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    let jockey = read_entity_chunk(
        &mut store,
        &chunk(conditional_animal_record(
            "minecraft:chicken",
            Uuid::from_u128(0xA301),
            Some(true),
        )),
    )
    .unwrap()[0];
    let chicken = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CHICKEN_ID, position())
        .unwrap();
    let trusting = read_entity_chunk(
        &mut store,
        &chunk(conditional_animal_record(
            "minecraft:ocelot",
            Uuid::from_u128(0xA302),
            Some(true),
        )),
    )
    .unwrap()[0];
    let young_ocelot = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_OCELOT_ID, position())
        .unwrap();
    let old_ocelot = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_OCELOT_ID, position())
        .unwrap();
    store.get_mut(young_ocelot).unwrap().tick_count = 2400;
    store.get_mut(old_ocelot).unwrap().tick_count = 2401;
    for id in store.ids() {
        store
            .get_mut(id)
            .unwrap()
            .living_mut()
            .unwrap()
            .no_action_time = 700;
    }

    {
        let mut native = {
            let mut native = NativeLevel::new();
            let (sender, receiver) = crate::tests::mailbox();
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut native.world,
                1,
                "player",
                0xC0DE_C0DE,
            );
            native.world.set_player_position_for_test(
                crate::connection::ConnectionId(1),
                Vec3d {
                    x: 200.5,
                    y: 64.0,
                    z: 0.5,
                },
            );
            native.world.set_player_mode_and_abilities_for_test(
                crate::connection::ConnectionId(1),
                bbb_protocol::game_types::GameType::Survival,
                bbb_protocol::packets::PlayerAbilities::for_game_type(
                    bbb_protocol::game_types::GameType::Survival,
                ),
            );
            native
        };
        native.with_level(|level| tick_entities(&mut store, level, true))
    };

    assert!(store.get(jockey).is_none());
    assert!(store.get(old_ocelot).is_none());
    for id in [chicken, trusting, young_ocelot] {
        assert!(store.get(id).is_some());
    }
}

#[test]
fn enderman_carried_despawn_state_save_restore() {
    let fresh = GeneralLivingData::new(VANILLA_ENTITY_TYPE_ENDERMAN_ID).unwrap();
    assert_eq!(
        fresh.enderman_retained().unwrap().carried_block_state_id(),
        None
    );

    let mut store = EntityStore::new(0xE10, EntityIdCounter::new());
    let id = read_entity_chunk(
        &mut store,
        &chunk(enderman_record(
            Uuid::from_u128(0xE101),
            Some(saved_block_state("minecraft:stone")),
        )),
    )
    .unwrap()[0];
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!("enderman uses general living")
    };
    assert_eq!(
        data.enderman_retained().unwrap().carried_block_state_id(),
        Some(state_id("minecraft:stone"))
    );
    assert!(data
        .opaque_fields()
        .iter()
        .all(|entry| entry.name != "carriedBlockState"));
    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    assert_eq!(
        saved
            .entities()
            .next()
            .unwrap()
            .raw()
            .iter()
            .find(|entry| entry.name == "carriedBlockState")
            .map(|entry| &entry.value),
        Some(&saved_block_state("minecraft:stone"))
    );
}

#[test]
fn enderman_carried_despawn_air_and_absent_states_carry_nothing() {
    for (index, carried) in [None, Some(saved_block_state("minecraft:air"))]
        .into_iter()
        .enumerate()
    {
        let mut store = EntityStore::new(0xE20 + index as i64, EntityIdCounter::new());
        let id = read_entity_chunk(
            &mut store,
            &chunk(enderman_record(
                Uuid::from_u128(0xE200 + index as u128),
                carried,
            )),
        )
        .unwrap()[0];
        let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
            unreachable!("enderman uses general living")
        };
        assert_eq!(
            data.enderman_retained().unwrap().carried_block_state_id(),
            None
        );
        let saved = write_entity_chunk(&store, (0, 0)).unwrap();
        assert!(saved
            .entities()
            .next()
            .unwrap()
            .raw()
            .iter()
            .all(|entry| entry.name != "carriedBlockState"));
    }
}

#[test]
fn enderman_carried_despawn_resets_carrier_and_removes_empty_base_mob() {
    let mut store = EntityStore::new(0xE40, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    let carried = read_entity_chunk(
        &mut store,
        &chunk(enderman_record(
            Uuid::from_u128(0xE401),
            Some(saved_block_state("minecraft:stone")),
        )),
    )
    .unwrap()[0];
    let empty = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ENDERMAN_ID, position())
        .unwrap();
    for id in [carried, empty] {
        store
            .get_mut(id)
            .unwrap()
            .living_mut()
            .unwrap()
            .no_action_time = 700;
    }

    {
        let mut native = {
            let mut native = NativeLevel::new();
            let (sender, receiver) = crate::tests::mailbox();
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut native.world,
                1,
                "player",
                0xC0DE_C0DE,
            );
            native.world.set_player_position_for_test(
                crate::connection::ConnectionId(1),
                Vec3d {
                    x: 200.5,
                    y: 64.0,
                    z: 0.5,
                },
            );
            native.world.set_player_mode_and_abilities_for_test(
                crate::connection::ConnectionId(1),
                bbb_protocol::game_types::GameType::Survival,
                bbb_protocol::packets::PlayerAbilities::for_game_type(
                    bbb_protocol::game_types::GameType::Survival,
                ),
            );
            native
        };
        native.with_level(|level| tick_entities(&mut store, level, true))
    };

    assert!(store.get(empty).is_none());
    assert!(store.get(carried).is_some());
}

#[test]
fn bucketable_despawn_state_defaults_and_tadpole_constant() {
    for (entity_type_id, default) in [
        (VANILLA_ENTITY_TYPE_COD_ID, false),
        (VANILLA_ENTITY_TYPE_TADPOLE_ID, true),
    ] {
        let mut data = GeneralLivingData::new(entity_type_id).unwrap();
        assert_eq!(data.is_from_bucket(), Some(default));
        assert!(data.set_from_bucket(!default));
        assert_eq!(
            data.is_from_bucket(),
            Some(if entity_type_id == VANILLA_ENTITY_TYPE_TADPOLE_ID {
                true
            } else {
                !default
            })
        );
    }
    let mut zombie = GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
    assert_eq!(zombie.is_from_bucket(), None);
    assert!(!zombie.set_from_bucket(true));
}

#[test]
fn bucketable_despawn_save_restore_keeps_exact_name_and_owned_flag() {
    let component = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string("Bucket friend")),
        entry("color", NbtSummaryValue::string("aqua")),
        entry("bold", NbtSummaryValue::Byte(1)),
    ]);
    for (index, resource_id) in ["minecraft:cod", "minecraft:tadpole"]
        .into_iter()
        .enumerate()
    {
        let mut entries = bucketable_record(
            resource_id,
            Uuid::from_u128(0xB000 + index as u128),
            Some(resource_id != "minecraft:tadpole"),
        );
        entries.push(entry("CustomName", component.clone()));
        entries.push(entry("CustomNameVisible", NbtSummaryValue::Byte(1)));
        let mut store = EntityStore::new(0xB0 + index as i64, EntityIdCounter::new());
        let id = read_entity_chunk(&mut store, &chunk(entries)).unwrap()[0];
        let entity = store.get(id).unwrap();
        assert_eq!(entity.custom_name().unwrap().component(), &component);
        assert!(entity.custom_name_visible());
        let EntityData::GeneralLiving(data) = entity.data() else {
            unreachable!("bucketable uses general living")
        };
        assert_eq!(data.is_from_bucket(), Some(true));

        let saved = write_entity_chunk(&store, (0, 0)).unwrap();
        let fields = saved.entities().next().unwrap().raw();
        assert_eq!(
            fields
                .iter()
                .filter(|field| field.name == "CustomName")
                .count(),
            1
        );
        assert_eq!(
            fields
                .iter()
                .find(|field| field.name == "CustomName")
                .map(|field| &field.value),
            Some(&component)
        );
        assert!(matches!(
            fields
                .iter()
                .find(|field| field.name == "CustomNameVisible")
                .map(|field| &field.value),
            Some(NbtSummaryValue::Byte(1))
        ));
        assert!(matches!(
            fields
                .iter()
                .find(|field| field.name == "FromBucket")
                .map(|field| &field.value),
            Some(NbtSummaryValue::Byte(1))
        ));
    }
}

#[test]
fn bucketable_despawn_missing_flag_uses_vanilla_default_before_canonical_save() {
    for (index, (resource_id, expected)) in [("minecraft:cod", false), ("minecraft:tadpole", true)]
        .into_iter()
        .enumerate()
    {
        let mut store = EntityStore::new(0xC0 + index as i64, EntityIdCounter::new());
        let id = read_entity_chunk(
            &mut store,
            &chunk(bucketable_record(
                resource_id,
                Uuid::from_u128(0xC000 + index as u128),
                None,
            )),
        )
        .unwrap()[0];
        let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
            unreachable!("bucketable uses general living")
        };
        assert_eq!(data.is_from_bucket(), Some(expected));
        let saved = write_entity_chunk(&store, (0, 0)).unwrap();
        assert!(matches!(
            saved.entities().next().unwrap()
                .raw()
                .iter()
                .find(|field| field.name == "FromBucket")
                .map(|field| &field.value),
            Some(NbtSummaryValue::Byte(value)) if *value == i8::from(expected)
        ));
    }
}

#[test]
fn bucketable_despawn_exact_persistence_and_name_removal_semantics() {
    let far = Vec3d {
        x: 200.5,
        y: 64.0,
        z: 0.5,
    };
    let mut store = EntityStore::new(0xE0, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    let spawn =
        |store: &mut EntityStore, uuid: u128, data: GeneralLivingData, name: Option<&str>| {
            store
                .restore_initialized(
                    Uuid::from_u128(uuid),
                    VANILLA_ENTITY_TYPE_COD_ID,
                    far,
                    EntityData::GeneralLiving(data),
                    |entity| {
                        entity.living_mut().unwrap().no_action_time = 700;
                        entity.set_custom_name(name.map(RichTextComponent::plain));
                    },
                )
                .unwrap()
        };
    let unnamed = spawn(
        &mut store,
        0xE001,
        GeneralLivingData::new(VANILLA_ENTITY_TYPE_COD_ID).unwrap(),
        None,
    );
    let named = spawn(
        &mut store,
        0xE002,
        GeneralLivingData::new(VANILLA_ENTITY_TYPE_COD_ID).unwrap(),
        Some("kept"),
    );
    let mut bucketed_data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_COD_ID).unwrap();
    assert!(bucketed_data.set_from_bucket(true));
    let bucketed = spawn(&mut store, 0xE003, bucketed_data, None);
    let named_zombie = store
        .restore_initialized(
            Uuid::from_u128(0xE004),
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            far,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
            ),
            |entity| {
                entity.set_custom_name(Some(RichTextComponent::plain("not persistent")));
            },
        )
        .unwrap();
    let tadpole = store
        .restore_initialized(
            Uuid::from_u128(0xE005),
            VANILLA_ENTITY_TYPE_TADPOLE_ID,
            far,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_TADPOLE_ID).unwrap(),
            ),
            |entity| entity.living_mut().unwrap().no_action_time = 700,
        )
        .unwrap();

    {
        let mut native = {
            let mut native = NativeLevel::new();
            let (sender, receiver) = crate::tests::mailbox();
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut native.world,
                1,
                "player",
                0xC0DE_C0DE,
            );
            native
                .world
                .set_player_position_for_test(crate::connection::ConnectionId(1), position());
            native.world.set_player_mode_and_abilities_for_test(
                crate::connection::ConnectionId(1),
                bbb_protocol::game_types::GameType::Survival,
                bbb_protocol::packets::PlayerAbilities::for_game_type(
                    bbb_protocol::game_types::GameType::Survival,
                ),
            );
            native
        };
        native.with_level(|level| tick_entities(&mut store, level, true))
    };
    assert!(store.get(unnamed).is_none());
    assert!(store.get(named_zombie).is_none());
    for id in [named, bucketed, tadpole] {
        assert!(store.get(id).is_some());
    }
}

#[test]
fn despawn_peaceful_removes_monsters_only() {
    let mut peaceful = EntityStore::new(31, EntityIdCounter::new());
    let zombie = peaceful
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let cow = peaceful
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    peaceful.set_difficulty(Difficulty::Peaceful);
    peaceful.drain_lifecycle_events();
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut peaceful, level, true))
    };
    assert!(peaceful.get(zombie).is_none());
    assert!(peaceful.get(cow).is_some());
    assert!(peaceful
        .drain_lifecycle_events()
        .iter()
        .any(|event| matches!(
            event,
            EntityLifecycleEvent::Removed { entity, .. }
                if entity.id() == zombie
        )));
}

#[test]
fn despawn_immediate_distance_is_strict_and_uses_generated_category() {
    let mut store = EntityStore::new(33, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    let edge = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 128.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let beyond = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 128.5001,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let never = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 500.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    store.set_difficulty(Difficulty::Normal);
    let mut level = {
        let mut native = NativeLevel::new();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(
            &sender,
            &receiver,
            &mut native.world,
            1,
            "player",
            0xC0DE_C0DE,
        );
        native
            .world
            .set_player_position_for_test(crate::connection::ConnectionId(1), position());
        native.world.set_player_mode_and_abilities_for_test(
            crate::connection::ConnectionId(1),
            bbb_protocol::game_types::GameType::Survival,
            bbb_protocol::packets::PlayerAbilities::for_game_type(
                bbb_protocol::game_types::GameType::Survival,
            ),
        );
        native
    };
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(store.get(edge).is_some());
    assert!(store.get(beyond).is_none());
    assert!(store.get(never).is_some());
}

#[test]
fn despawn_persistence_and_leash() {
    let player = Vec3d {
        x: 200.5,
        y: 64.0,
        z: 0.5,
    };

    let mut persistent_entries = zombie_record(Uuid::from_u128(0x3001));
    persistent_entries
        .iter_mut()
        .find(|entry| entry.name == "PersistenceRequired")
        .unwrap()
        .value = NbtSummaryValue::Byte(1);
    let mut persistent = EntityStore::new(35, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    let persistent_id = read_entity_chunk(&mut persistent, &chunk(persistent_entries)).unwrap()[0];
    persistent
        .get_mut(persistent_id)
        .unwrap()
        .living_mut()
        .unwrap()
        .no_action_time = 700;
    {
        let mut native = {
            let mut native = NativeLevel::new();
            let (sender, receiver) = crate::tests::mailbox();
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut native.world,
                1,
                "player",
                0xC0DE_C0DE,
            );
            native
                .world
                .set_player_position_for_test(crate::connection::ConnectionId(1), player);
            native.world.set_player_mode_and_abilities_for_test(
                crate::connection::ConnectionId(1),
                bbb_protocol::game_types::GameType::Survival,
                bbb_protocol::packets::PlayerAbilities::for_game_type(
                    bbb_protocol::game_types::GameType::Survival,
                ),
            );
            native
        };
        native.with_level(|level| tick_entities(&mut persistent, level, true))
    };

    let mut leashed_entries = zombie_record(Uuid::from_u128(0x3002));
    leashed_entries.push(entry(
        "leash",
        NbtSummaryValue::Compound(vec![entry(
            "UUID",
            NbtSummaryValue::IntArray(vec![0, 0, 0, 7]),
        )]),
    ));
    let mut leashed = EntityStore::new(36, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    let leashed_id = read_entity_chunk(&mut leashed, &chunk(leashed_entries)).unwrap()[0];
    leashed
        .get_mut(leashed_id)
        .unwrap()
        .living_mut()
        .unwrap()
        .no_action_time = 700;
    {
        let mut native = {
            let mut native = NativeLevel::new();
            let (sender, receiver) = crate::tests::mailbox();
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut native.world,
                1,
                "player",
                0xC0DE_C0DE,
            );
            native
                .world
                .set_player_position_for_test(crate::connection::ConnectionId(1), player);
            native.world.set_player_mode_and_abilities_for_test(
                crate::connection::ConnectionId(1),
                bbb_protocol::game_types::GameType::Survival,
                bbb_protocol::packets::PlayerAbilities::for_game_type(
                    bbb_protocol::game_types::GameType::Survival,
                ),
            );
            native
        };
        native.with_level(|level| tick_entities(&mut leashed, level, true))
    };
    assert!(leashed.get(leashed_id).is_none());

    let mut resolved = EntityStore::new(38, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    let leashed_id = read_entity_chunk(
        &mut resolved,
        &chunk(zombie_record(Uuid::from_u128(0x3003))),
    )
    .unwrap()[0];
    let holder = resolved
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
            position(),
        )
        .unwrap();
    assert!(resolved.leash_entity_to(leashed_id, holder, false));
    resolved
        .get_mut(leashed_id)
        .unwrap()
        .living_mut()
        .unwrap()
        .no_action_time = 700;
    {
        let mut native = {
            let mut native = NativeLevel::new();
            let (sender, receiver) = crate::tests::mailbox();
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut native.world,
                1,
                "player",
                0xC0DE_C0DE,
            );
            native
                .world
                .set_player_position_for_test(crate::connection::ConnectionId(1), player);
            native.world.set_player_mode_and_abilities_for_test(
                crate::connection::ConnectionId(1),
                bbb_protocol::game_types::GameType::Survival,
                bbb_protocol::packets::PlayerAbilities::for_game_type(
                    bbb_protocol::game_types::GameType::Survival,
                ),
            );
            native
        };
        native.with_level(|level| tick_entities(&mut resolved, level, true))
    };
}

fn despawn_level(player: Vec3d) -> NativeLevel {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(
        &sender,
        &receiver,
        &mut native.world,
        1,
        "player",
        0xC0DE_C0DE,
    );
    native
        .world
        .set_player_position_for_test(crate::connection::ConnectionId(1), player);
    native
}

fn stationary_despawn_mob(store: &mut EntityStore, entity_type_id: i32, uuid: u128) -> i32 {
    let mut data = GeneralLivingData::new(entity_type_id).unwrap();
    data.set_no_ai(true);
    store
        .restore_initialized(
            Uuid::from_u128(uuid),
            entity_type_id,
            position(),
            EntityData::GeneralLiving(data),
            |entity| {
                entity.set_no_gravity(true);
            },
        )
        .unwrap()
}

#[test]
fn raider_and_zombie_villager_distance_exceptions_yield_to_peaceful() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID;
    let mut store = EntityStore::new(0xDE5, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    store.set_difficulty(Difficulty::Normal);
    let mut ordinary = Vec::new();
    let mut retained = Vec::new();
    for kind in VANILLA_ENTITY_TYPES
        .iter()
        .filter(|kind| crate::entity::raider::is_raider(kind.id))
    {
        let id = stationary_despawn_mob(&mut store, kind.id, 0xDE500 + kind.id as u128);
        ordinary.push(id);
        let id = stationary_despawn_mob(&mut store, kind.id, 0xDE600 + kind.id as u128);
        assert!(crate::entity::raider::join_raid(
            store.get_mut(id).unwrap(),
            7,
            1
        ));
        retained.push(id);
    }
    ordinary.push(stationary_despawn_mob(
        &mut store,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
        0xDE700,
    ));
    for (uuid, xp, conversion_time) in [(0xDE701, 1, -1), (0xDE702, 0, 3600)] {
        let mut record = base_entity("minecraft:zombie_villager", Uuid::from_u128(uuid));
        record.extend([
            entry("Xp", NbtSummaryValue::Int(xp)),
            entry("ConversionTime", NbtSummaryValue::Int(conversion_time)),
            entry("NoAI", NbtSummaryValue::Byte(1)),
            entry("NoGravity", NbtSummaryValue::Byte(1)),
        ]);
        retained.push(read_entity_chunk(&mut store, &chunk(record)).unwrap()[0]);
    }
    let vehicle = stationary_despawn_mob(&mut store, VANILLA_ENTITY_TYPE_COW_ID, 0xDE703);
    let rider = stationary_despawn_mob(
        &mut store,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID,
        0xDE704,
    );
    assert!(store.link_existing_passenger_forced(vehicle, rider));
    retained.push(rider);
    let mut level = despawn_level(Vec3d {
        x: 200.5,
        ..position()
    });
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(ordinary.len(), 7);
    for id in ordinary {
        assert!(
            store.get(id).is_none(),
            "an unbound raider or unexperienced zombie villager despawns far away"
        );
    }
    for id in &retained {
        assert!(
            store.get(*id).is_some(),
            "raid, passenger, conversion or XP keeps entity {id}"
        );
    }
    assert_eq!(store.vehicle(rider), Some(vehicle));
    store.set_difficulty(Difficulty::Peaceful);
    level.with_level(|level| tick_entities(&mut store, level, true));
    for id in retained {
        assert!(
            store.get(id).is_none(),
            "peaceful removes retained hostile entity {id}"
        );
    }
    assert!(store.get(vehicle).is_some());
}

#[test]
fn a_patrol_outlasts_idle_distance_despawn_but_not_the_hard_distance() {
    let mut store = EntityStore::new(0xDE8, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    store.set_difficulty(Difficulty::Normal);
    let ordinary = stationary_despawn_mob(
        &mut store,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID,
        0xDE801,
    );
    let mut patrol_record = base_entity("minecraft:pillager", Uuid::from_u128(0xDE802));
    patrol_record.extend([
        entry("Patrolling", NbtSummaryValue::Byte(1)),
        entry("NoAI", NbtSummaryValue::Byte(1)),
        entry("NoGravity", NbtSummaryValue::Byte(1)),
    ]);
    let patrol = read_entity_chunk(&mut store, &chunk(patrol_record)).unwrap()[0];
    for id in [ordinary, patrol] {
        store
            .get_mut(id)
            .unwrap()
            .living_mut()
            .unwrap()
            .no_action_time = 700;
    }
    let mut level = despawn_level(Vec3d {
        x: 100.5,
        ..position()
    });
    for _ in 0..10000 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert!(
        store.get(ordinary).is_none(),
        "an idle non-patrol eventually despawns beyond 32 blocks"
    );
    assert!(
        store.get(patrol).is_some(),
        "the patrol remains inside its 128-block boundary"
    );
    level.world.set_player_position_for_test(
        crate::connection::ConnectionId(1),
        Vec3d {
            x: 200.5,
            ..position()
        },
    );
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(store.get(patrol).is_none());
}
