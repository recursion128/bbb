use super::*;
#[test]
fn phantom_targetless_runtime_production_tick_and_no_ai_are_exact() {
    let mut store = EntityStore::new(0x5048_414E, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PHANTOM_ID, position())
        .unwrap();
    store.get_mut(id).unwrap().living_mut().unwrap().y_body_rot = 27.0;
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut store, level, true))
    };
    let entity = store.get(id).unwrap();
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    assert!(data.phantom_retained().unwrap().anchor().is_some());
    assert_ne!(entity.delta_movement(), Vec3d::default());
    assert_eq!(entity.living().unwrap().y_head_rot(), entity.y_rot());
    assert_eq!(entity.living().unwrap().y_body_rot, entity.y_rot());

    let mut no_ai = EntityStore::new(0x5048_414F, EntityIdCounter::new());
    let id = no_ai
        .spawn_general_living(VANILLA_ENTITY_TYPE_PHANTOM_ID, position())
        .unwrap();
    let EntityData::GeneralLiving(data) = &mut no_ai.get_mut(id).unwrap().data else {
        unreachable!()
    };
    data.set_no_ai(true);
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut no_ai, level, true))
    };
    let EntityData::GeneralLiving(data) = no_ai.get(id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(data.phantom_retained().unwrap().anchor(), None);
}

#[test]
fn strider_environment_runtime_cold_warm_and_no_ai() {
    let mut store = EntityStore::new(0x57_12_01, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_STRIDER_ID, position())
        .unwrap();
    let mut entity = store.detach(id).unwrap();
    let mut cold = NativeLevel::new();

    cold.with_level(|level| crate::entity::strider::tick_environment(&mut entity, level));
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!("Strider uses GeneralLiving")
    };
    assert!(data.strider_state().unwrap().suffocating());
    let modifier = entity
        .living()
        .unwrap()
        .attributes()
        .instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap()
        .modifier("minecraft:suffocating")
        .unwrap();
    assert_eq!(modifier.amount, f64::from(-0.34_f32));
    assert_eq!(
        modifier.operation,
        crate::entity::AttributeOperation::AddMultipliedBase
    );
    assert!(entity
        .take_synched_dirty()
        .contains(&bbb_protocol::entity_synched_data::VANILLA_STRIDER_SUFFOCATING_DATA_ID));

    cold.with_level(|level| crate::entity::strider::tick_environment(&mut entity, level));
    assert!(!entity
        .take_synched_dirty()
        .contains(&bbb_protocol::entity_synched_data::VANILLA_STRIDER_SUFFOCATING_DATA_ID));

    let mut warm = {
        let mut native = NativeLevel::new();
        native.set(entity.block_position(), state_id("minecraft:lava"));
        native
    };
    warm.with_level(|level| crate::entity::strider::tick_environment(&mut entity, level));
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    assert!(!data.strider_state().unwrap().suffocating());
    assert!(entity
        .living()
        .unwrap()
        .attributes()
        .instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap()
        .modifier("minecraft:suffocating")
        .is_none());

    cold.with_level(|level| crate::entity::strider::tick_environment(&mut entity, level));
    let mut legacy_warm = {
        let mut native = NativeLevel::new();
        native.set(entity.on_pos_legacy(), state_id("minecraft:lava"));
        native
    };
    assert_ne!(entity.block_position(), entity.on_pos_legacy());
    legacy_warm.with_level(|level| crate::entity::strider::tick_environment(&mut entity, level));
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    assert!(!data.strider_state().unwrap().suffocating());

    cold.with_level(|level| crate::entity::strider::tick_environment(&mut entity, level));
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.set_no_ai(true);
    warm.with_level(|level| crate::entity::strider::tick_environment(&mut entity, level));
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    assert!(data.strider_state().unwrap().suffocating());
    assert!(entity
        .living()
        .unwrap()
        .attributes()
        .instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap()
        .modifier("minecraft:suffocating")
        .is_some());
}

#[test]
fn strider_environment_runtime_float_surface_and_submerged() {
    let mut surface_store = EntityStore::new(0x57_12_02, EntityIdCounter::new());
    let surface_id = surface_store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            Vec3d {
                x: 0.5,
                y: 64.6,
                z: 0.5,
            },
        )
        .unwrap();
    let mut surface = surface_store.detach(surface_id).unwrap();
    surface.set_delta_movement(Vec3d {
        x: 0.4,
        y: -0.2,
        z: -0.6,
    });
    let mut surface_level = {
        let mut native = NativeLevel::new();
        native.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:lava"));
        native
    };
    surface_level.with_level(|level| crate::entity::strider::float_strider(&mut surface, level));
    assert!(surface.on_ground());
    assert_eq!(
        surface.delta_movement(),
        Vec3d {
            x: 0.4,
            y: -0.2,
            z: -0.6,
        }
    );

    let mut submerged_store = EntityStore::new(0x57_12_03, EntityIdCounter::new());
    let submerged_id = submerged_store
        .spawn_general_living(VANILLA_ENTITY_TYPE_STRIDER_ID, position())
        .unwrap();
    let mut submerged = submerged_store.detach(submerged_id).unwrap();
    submerged.set_delta_movement(Vec3d {
        x: 0.4,
        y: -0.2,
        z: -0.6,
    });
    let mut submerged_level = {
        let mut native = NativeLevel::new();
        native.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:lava"));
        native.set(BlockPos { x: 0, y: 65, z: 0 }, state_id("minecraft:lava"));
        native
    };
    submerged_level
        .with_level(|level| crate::entity::strider::float_strider(&mut submerged, level));
    assert!(!submerged.on_ground());
    assert_eq!(
        submerged.delta_movement(),
        Vec3d {
            x: 0.2,
            y: -0.05,
            z: -0.3,
        }
    );
}

#[test]
fn ghast_autonomous_runtime_restore_save_and_tracking() {
    let uuid = Uuid::from_u128(0x6A57_0001);
    let mut store = EntityStore::new(0x6A57_0002, EntityIdCounter::new());
    let id = read_entity_chunk(
        &mut store,
        &chunk(ghast_record(uuid, Some(NbtSummaryValue::Byte(-7)))),
    )
    .unwrap()[0];
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!("Ghast uses GeneralLiving")
    };
    assert_eq!(
        data.ghast_retained()
            .map(GhastRetainedState::explosion_power),
        Some(-7)
    );
    assert!(data
        .opaque_fields()
        .iter()
        .all(|field| field.name != "ExplosionPower"));

    let entity = store.get_mut(id).unwrap();
    assert!(crate::entity::ghast::set_charging(entity, true));
    assert!(entity
        .take_synched_dirty()
        .contains(&VANILLA_GHAST_CHARGING_DATA_ID));
    assert!(!crate::entity::ghast::set_charging(entity, true));
    assert!(entity.take_synched_dirty().is_empty());
    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    assert_eq!(
        saved
            .entities()
            .next()
            .unwrap()
            .raw()
            .iter()
            .filter(|field| field.name == "ExplosionPower")
            .map(|field| &field.value)
            .collect::<Vec<_>>(),
        vec![&NbtSummaryValue::Byte(-7)]
    );

    let mut defaults = EntityStore::new(0x6A57_0003, EntityIdCounter::new());
    let id = read_entity_chunk(&mut defaults, &chunk(ghast_record(uuid, None))).unwrap()[0];
    let EntityData::GeneralLiving(data) = defaults.get(id).unwrap().data() else {
        unreachable!("Ghast uses GeneralLiving")
    };
    assert_eq!(
        data.ghast_retained()
            .map(GhastRetainedState::explosion_power),
        Some(1)
    );
    assert!(write_entity_chunk(&defaults, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .iter()
        .any(|field| field.name == "ExplosionPower" && field.value == NbtSummaryValue::Byte(1)));
}

#[test]
fn phantom_targetless_runtime_restore_save() {
    let uuid = Uuid::from_u128(0x5048_414E_0001);
    let mut store = EntityStore::new(0x5048_414E_0002, EntityIdCounter::new());
    let id = read_entity_chunk(
        &mut store,
        &chunk(phantom_record(
            uuid,
            Some(NbtSummaryValue::Int(80)),
            Some(NbtSummaryValue::IntArray(vec![3, 90, -4])),
        )),
    )
    .unwrap()[0];
    let entity = store.get(id).unwrap();
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    assert_eq!(data.phantom_retained().unwrap().size(), 64);
    assert_eq!(
        data.phantom_retained().unwrap().anchor(),
        Some(BlockPos { x: 3, y: 90, z: -4 })
    );
    assert!(data
        .opaque_fields()
        .iter()
        .all(|entry| !matches!(entry.name.to_jdk_string().as_str(), "anchor_pos" | "size")));
    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    let raw = saved.entities().next().unwrap().raw();
    assert_eq!(
        raw.iter()
            .filter(|entry| entry.name == "size")
            .map(|entry| &entry.value)
            .collect::<Vec<_>>(),
        vec![&NbtSummaryValue::Int(64)]
    );
    assert_eq!(
        raw.iter()
            .filter(|entry| entry.name == "anchor_pos")
            .count(),
        1
    );
}

#[test]
fn ghast_autonomous_runtime_floats_and_plays_ambient_unless_silent() {
    let uuid = Uuid::from_u128(0x6A57_0010);
    let mut audible = EntityStore::new(0x6A57_0011, EntityIdCounter::new());
    let audible_id = read_entity_chunk(
        &mut audible,
        &chunk(ghast_record(uuid, Some(NbtSummaryValue::Byte(1)))),
    )
    .unwrap()[0];
    audible.drain_lifecycle_events();
    audible
        .mob_ai_mut(audible_id)
        .unwrap()
        .set_ambient_sound_time(1000);

    let mut silent = EntityStore::new(0x6A57_0012, EntityIdCounter::new());
    let silent_id = read_entity_chunk(
        &mut silent,
        &chunk(ghast_record(uuid, Some(NbtSummaryValue::Byte(1)))),
    )
    .unwrap()[0];
    silent.drain_lifecycle_events();
    silent.get_mut(silent_id).unwrap().set_silent(true);
    silent.get_mut(silent_id).unwrap().take_synched_dirty();
    silent
        .mob_ai_mut(silent_id)
        .unwrap()
        .set_ambient_sound_time(1000);

    let origin = Vec3d {
        x: 8.5,
        y: 70.0,
        z: 8.5,
    };
    audible.get_mut(audible_id).unwrap().set_pos(origin);
    silent.get_mut(silent_id).unwrap().set_pos(origin);
    let mut native = NativeLevel::new();
    native.with_level(|level| tick_entities(&mut audible, level, true));
    native.with_level(|level| tick_entities(&mut silent, level, true));
    for _ in 0..100 {
        if audible.get(audible_id).unwrap().position() != origin {
            break;
        }
        native.with_level(|level| tick_entities(&mut audible, level, true));
    }
    assert_ne!(audible.get(audible_id).unwrap().position(), origin);
    let sound = audible
        .drain_lifecycle_events()
        .into_iter()
        .find_map(|event| match event {
            EntityLifecycleEvent::Sound { sound } => Some(sound),
            _ => None,
        })
        .unwrap();
    assert_eq!(sound.source, SoundSource::Hostile);
    assert_eq!(sound.volume, 5.0);
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.ghast.ambient",
            )
            .unwrap(),
        }
    );
    assert!(silent
        .drain_lifecycle_events()
        .iter()
        .all(|event| !matches!(event, EntityLifecycleEvent::Sound { .. })));
}

#[test]
fn ghast_no_ai_keeps_its_position_in_air_and_fluids() {
    for (case, mut level) in [
        (0_u128, {
            let mut native = NativeLevel::new();
            native.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:water"));
            native
        }),
        (1, {
            let mut native = NativeLevel::new();
            native.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:lava"));
            native
        }),
        (2, NativeLevel::new()),
    ] {
        let mut store = EntityStore::new(0x6A57_0020 + case as i64, EntityIdCounter::new());
        let id = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_GHAST_ID, position())
            .unwrap();
        let entity = store.get_mut(id).unwrap();
        entity.set_delta_movement(Vec3d {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!("Ghast uses GeneralLiving")
        };
        data.set_no_ai(true);
        level.with_level(|level| tick_entities(&mut store, level, true));
        let entity = store.get(id).unwrap();
        assert_eq!(entity.position(), position());
        assert_eq!(entity.delta_movement().x, 1.0);
    }
}

#[test]
fn bat_autonomous_runtime_restore_save_defaults() {
    let uuid = Uuid::from_u128(0xBA70);
    let mut store = EntityStore::new(0xBA70, EntityIdCounter::new());
    let ids = read_entity_chunk(
        &mut store,
        &chunk(bat_record(uuid, Some(NbtSummaryValue::Byte(0x55)))),
    )
    .unwrap();
    let EntityData::GeneralLiving(data) = store.get(ids[0]).unwrap().data() else {
        unreachable!("Bat uses GeneralLiving")
    };
    assert_eq!(data.bat_retained().map(BatRetainedState::flags), Some(0x55));
    assert!(data
        .opaque_fields()
        .iter()
        .all(|entry| entry.name != "BatFlags"));
    let output = write_entity_chunk(&store, (0, 0)).unwrap();
    assert_eq!(
        output
            .entities()
            .next()
            .unwrap()
            .raw()
            .iter()
            .filter(|entry| entry.name == "BatFlags")
            .map(|entry| &entry.value)
            .collect::<Vec<_>>(),
        vec![&NbtSummaryValue::Byte(0x55)]
    );

    let mut defaults = EntityStore::new(0xBA71, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut defaults, &chunk(bat_record(uuid, None))).unwrap();
    let EntityData::GeneralLiving(data) = defaults.get(ids[0]).unwrap().data() else {
        unreachable!("Bat uses GeneralLiving")
    };
    assert_eq!(data.bat_retained().map(BatRetainedState::flags), Some(0));
    assert!(write_entity_chunk(&defaults, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .iter()
        .any(|entry| entry.name == "BatFlags" && entry.value == NbtSummaryValue::Byte(0)));
}

#[test]
fn bat_autonomous_runtime_ambient_wake_and_no_ai_rest() {
    let mut ambient = EntityStore::new(0xBA73, EntityIdCounter::new());
    let ambient_id = ambient
        .spawn_general_living(VANILLA_ENTITY_TYPE_BAT_ID, position())
        .unwrap();
    ambient.drain_lifecycle_events();
    ambient
        .mob_ai_mut(ambient_id)
        .unwrap()
        .set_ambient_sound_time(1000);
    let entity = ambient.get_mut(ambient_id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!("Bat uses GeneralLiving")
    };
    data.bat_retained_mut().unwrap().set_resting(false);
    data.set_no_ai(true);
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut ambient, level, true))
    };
    let sound = ambient
        .drain_lifecycle_events()
        .into_iter()
        .find_map(|event| match event {
            EntityLifecycleEvent::Sound { sound } => Some(sound),
            _ => None,
        })
        .expect("flying Bat always resolves an ambient sound after a timer hit");
    assert_eq!((sound.source, sound.volume), (SoundSource::Neutral, 0.1));
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.bat.ambient",
            )
            .unwrap(),
        }
    );

    let mut waking = EntityStore::new(0xBA74, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    let waking_id = waking
        .spawn_general_living(VANILLA_ENTITY_TYPE_BAT_ID, position())
        .unwrap();
    waking.drain_lifecycle_events();
    let above = bbb_protocol::packets::BlockPos { x: 0, y: 65, z: 0 };
    let mut level = {
        let mut native = NativeLevel::new();
        native.set(above, state_id("minecraft:stone"));
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
    level.with_level(|level| tick_entities(&mut waking, level, true));
    let EntityData::GeneralLiving(data) = waking.get(waking_id).unwrap().data() else {
        unreachable!("Bat uses GeneralLiving")
    };
    assert_eq!(
        data.bat_retained().map(BatRetainedState::is_resting),
        Some(false)
    );
    assert!(level
        .world
        .drain_level_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.event_type == 1025
            && event.pos == BlockPos { x: 0, y: 64, z: 0 }
            && event.data == 0));
    assert!(waking.get(waking_id).unwrap().synched_data_is_dirty());

    let mut no_ai = EntityStore::new(0xBA75, EntityIdCounter::new());
    let no_ai_id = no_ai
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_BAT_ID,
            Vec3d {
                y: 64.4,
                ..position()
            },
        )
        .unwrap();
    let entity = no_ai.get_mut(no_ai_id).unwrap();
    entity.set_delta_movement(Vec3d {
        x: 0.2,
        y: 0.3,
        z: -0.1,
    });
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!("Bat uses GeneralLiving")
    };
    data.set_no_ai(true);
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut no_ai, level, true))
    };
    let entity = no_ai.get(no_ai_id).unwrap();
    assert_eq!(entity.delta_movement(), Vec3d::default());
    assert_eq!(
        entity.position().y,
        entity.position().y.floor() + 1.0 - f64::from(entity.bb_height())
    );
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!("Bat uses GeneralLiving")
    };
    assert!(data.bat_retained().unwrap().is_resting());

    let mut flying = EntityStore::new(0xBA76, EntityIdCounter::new());
    let start = Vec3d {
        x: 0.25,
        z: 0.25,
        ..position()
    };
    let flying_id = flying
        .spawn_general_living(VANILLA_ENTITY_TYPE_BAT_ID, start)
        .unwrap();
    let EntityData::GeneralLiving(data) = &mut flying.get_mut(flying_id).unwrap().data else {
        unreachable!("Bat uses GeneralLiving")
    };
    data.bat_retained_mut().unwrap().set_resting(false);
    let mut native = NativeLevel::new();
    native.with_level(|level| tick_entities(&mut flying, level, true));
    let entity = flying.get(flying_id).unwrap();
    assert_ne!(entity.position(), start);
    let movement = entity.delta_movement();
    assert!(movement.x * movement.x + movement.z * movement.z > 0.0);
    assert_ne!(entity.y_rot(), 0.0);
}

#[test]
fn bat_autonomous_runtime_damage_wakes_before_base_refusal_without_event() {
    let generic = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:generic")
        .unwrap();
    let mut store = EntityStore::new(0xBA76, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_BAT_ID, position())
        .unwrap();
    let mut entity = store.detach(id).unwrap();
    let effects = hurt_server_with_runtime(
        &mut entity,
        &mut store,
        StoredEntityDamageSource {
            damage_type: generic,
            direct_entity_id: None,
            causing_entity: None,
            attacker_views: Default::default(),
        },
        0.0,
        StoredEntityDamageRules::default(),
    );
    assert!(effects.outcome.succeeded);
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!("Bat uses GeneralLiving")
    };
    assert!(!data.bat_retained().unwrap().is_resting());
    assert!(entity
        .take_synched_dirty()
        .contains(&VANILLA_BAT_FLAGS_DATA_ID));

    let mut invulnerable = EntityStore::new(0xBA77, EntityIdCounter::new());
    let id = invulnerable
        .spawn_general_living(VANILLA_ENTITY_TYPE_BAT_ID, position())
        .unwrap();
    let mut entity = invulnerable.detach(id).unwrap();
    entity.set_invulnerable(true);
    let effects = hurt_server_with_runtime(
        &mut entity,
        &mut invulnerable,
        StoredEntityDamageSource {
            damage_type: generic,
            direct_entity_id: None,
            causing_entity: None,
            attacker_views: Default::default(),
        },
        1.0,
        StoredEntityDamageRules::default(),
    );
    assert!(!effects.outcome.succeeded);
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!("Bat uses GeneralLiving")
    };
    assert!(data.bat_retained().unwrap().is_resting());
}

#[test]
fn vex_autonomous_runtime_restore_save() {
    let uuid = Uuid::from_u128(0x0E_0001);
    let owner = Uuid::from_u128(0x0E_0002);
    let mut store = EntityStore::new(0x0E_0003, EntityIdCounter::new());
    let ids = read_entity_chunk(
        &mut store,
        &chunk(vex_record(
            uuid,
            Some(NbtSummaryValue::IntArray(vec![7, -3, 11])),
            Some(NbtSummaryValue::Int(39)),
            Some(saved_uuid(owner)),
        )),
    )
    .unwrap();
    let EntityData::GeneralLiving(data) = store.get(ids[0]).unwrap().data() else {
        unreachable!("Vex uses GeneralLiving")
    };
    let vex = data.vex_retained().unwrap();
    assert_eq!(vex.flags(), 0);
    assert_eq!(vex.owner(), Some(owner));
    assert_eq!(vex.bound_origin(), Some(BlockPos { x: 7, y: -3, z: 11 }));
    assert_eq!(vex.limited_life_ticks(), Some(39));
    assert!(data.opaque_fields().iter().all(|field| !matches!(
        field.name.to_jdk_string().as_str(),
        "bound_pos" | "life_ticks" | "owner"
    )));

    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    let raw = saved.entities().next().unwrap().raw();
    assert!(raw.iter().any(|field| {
        field.name == "bound_pos" && field.value == NbtSummaryValue::IntArray(vec![7, -3, 11])
    }));
    assert!(raw
        .iter()
        .any(|field| { field.name == "life_ticks" && field.value == NbtSummaryValue::Int(39) }));
    assert!(raw
        .iter()
        .any(|field| field.name == "owner" && field.value == saved_uuid(owner)));
    assert!(raw.iter().all(|field| field.name != "VexFlags"));
}

#[test]
fn vex_no_ai_keeps_position_and_ticks_limited_life() {
    let uuid = Uuid::from_u128(0x0E_0010);
    let mut store = EntityStore::new(0x0E_0011, EntityIdCounter::new());
    let ids = read_entity_chunk(
        &mut store,
        &chunk(vex_record(uuid, None, Some(NbtSummaryValue::Int(1)), None)),
    )
    .unwrap();
    let id = ids[0];
    store.drain_lifecycle_events();
    let health_before = store.get(id).unwrap().living().unwrap().health();
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_delta_movement(Vec3d {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!("Vex uses GeneralLiving")
        };
        data.set_no_ai(true);
    }
    let mut level = {
        let mut native = NativeLevel::new();
        native.set(BlockPos { x: 1, y: 64, z: 0 }, state_id("minecraft:stone"));
        native
    };
    level.with_level(|level| tick_entities(&mut store, level, true));

    let entity = store.get_mut(id).unwrap();
    assert_eq!(entity.position(), position());
    assert!(entity.no_gravity());
    assert_eq!(entity.living().unwrap().health(), health_before - 1.0);
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!("Vex uses GeneralLiving")
    };
    assert_eq!(
        data.vex_retained()
            .and_then(VexRetainedState::limited_life_ticks),
        Some(20)
    );
    assert!(entity
        .take_synched_dirty()
        .contains(&VANILLA_ENTITY_NO_GRAVITY_DATA_ID));
    assert_eq!(
        level.world.positioned_game_events_for_test(DIMENSION).len(),
        1
    );
    assert_eq!(
        level.world.positioned_game_events_for_test(DIMENSION)[0]
            .recorded
            .game_event,
        VanillaGameEvent::ENTITY_DAMAGE
    );
}

#[test]
fn creeper_runtime_restore_save() {
    let uuid = Uuid::from_u128(0xC0EE_0001);
    let mut store = EntityStore::new(0xC0EE_0002, EntityIdCounter::new());
    let ids =
        read_entity_chunk(&mut store, &chunk(creeper_record(uuid, true, 41, -7, true))).unwrap();
    let EntityData::GeneralLiving(data) = store.get(ids[0]).unwrap().data() else {
        unreachable!("Creeper uses GeneralLiving")
    };
    let creeper = data.creeper_retained().unwrap();
    assert_eq!(
        (
            creeper.swell_dir(),
            creeper.powered(),
            creeper.ignited(),
            creeper.old_swell(),
            creeper.swell(),
            creeper.max_swell(),
            creeper.explosion_radius(),
        ),
        (-1, true, true, 0, 0, 41, -7)
    );
    assert!(data
        .opaque_fields()
        .iter()
        .any(|field| { field.name == "FutureCreeperPayload" }));
    assert!(data.opaque_fields().iter().all(|field| !matches!(
        field.name.to_jdk_string().as_str(),
        "powered" | "Fuse" | "ExplosionRadius" | "ignited"
    )));

    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    let raw = saved.entities().next().unwrap().raw();
    for (name, value) in [
        ("powered", NbtSummaryValue::Byte(1)),
        ("Fuse", NbtSummaryValue::Short(41)),
        ("ExplosionRadius", NbtSummaryValue::Byte(-7)),
        ("ignited", NbtSummaryValue::Byte(1)),
    ] {
        assert_eq!(
            raw.iter()
                .filter(|field| field.name == name)
                .map(|field| &field.value)
                .collect::<Vec<_>>(),
            vec![&value]
        );
    }
}

#[test]
fn creeper_runtime_ignited_no_ai_primes_and_explodes_in_the_world() {
    let mut record = creeper_record(Uuid::from_u128(0xC0EE_0010), true, 2, 4, true);
    record.push(entry("NoAI", NbtSummaryValue::Byte(1)));
    let mut loaded = EntityStore::new(0xC0EE_0011, EntityIdCounter::new());
    let id = read_entity_chunk(&mut loaded, &chunk(record)).unwrap()[0];
    let data = loaded.get(id).unwrap().data().clone();
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "observer", 1);
    native.world.set_player_position_for_test(
        crate::connection::ConnectionId(1),
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 8.5,
        },
    );
    let (id, _) = native.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        position(),
        data,
    );
    let primed = native.world.tick(receiver.drain());
    assert!(native.world.stored_entity_for_test(DIMENSION, id).is_some());
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::PRIME_FUSE
                && event.recorded.source_entity_id == Some(id)
        ));
    assert!(primed.events.iter().any(|event| matches!(
        event,
        crate::world::WorldEvent::Send {
            packet: crate::connection::ClientboundPlayPacket::Sound(_),
            ..
        }
    )));
    let blasted = native.world.tick(receiver.drain());
    assert!(native.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(blasted.events.iter().any(|event|matches!(event,crate::world::WorldEvent::Send { packet:crate::connection::ClientboundPlayPacket::Explode(explosion),.. } if explosion.radius == 8.0)));
    assert_eq!(
        native
            .world
            .block_state(DIMENSION, BlockPos { x: 0, y: 63, z: 0 }),
        Some(state_id("minecraft:air"))
    );
}

#[test]
fn creeper_runtime_fall_swell_uses_java_truncation_and_fuse_cap() {
    let mut state = CreeperRetainedState::from_saved(false, 30, 3, false);
    state.apply_fall_distance(4.9);
    assert_eq!(state.swell(), 7);
    state.apply_fall_distance(100.0);
    assert_eq!(state.swell(), 25);
}

#[test]
fn mob_ai_runtime_restores_and_saves_owned_mob_flags() {
    let uuid = Uuid::from_u128(0x1235);
    let mut entries = zombie_record(uuid);
    entries
        .iter_mut()
        .find(|entry| entry.name == "LeftHanded")
        .unwrap()
        .value = NbtSummaryValue::Byte(1);
    entries.push(entry("NoAI", NbtSummaryValue::Byte(1)));
    let mut store = EntityStore::new(9, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut store, &chunk(entries)).unwrap();
    let EntityData::GeneralLiving(data) = store.get(ids[0]).unwrap().data() else {
        panic!("zombie restores through the general living owner")
    };
    assert_eq!(
        data.mob_flags(),
        Some(VANILLA_MOB_FLAG_LEFT_HANDED | VANILLA_MOB_FLAG_NO_AI)
    );

    let output = write_entity_chunk(&store, (0, 0)).unwrap();
    let raw = output.entities().next().unwrap().raw();
    for name in ["LeftHanded", "NoAI"] {
        assert_eq!(raw.iter().filter(|entry| entry.name == name).count(), 1);
        assert_eq!(
            raw.iter()
                .find(|entry| entry.name == name)
                .map(|entry| &entry.value),
            Some(&NbtSummaryValue::Byte(1))
        );
    }
}

#[test]
fn mob_retained_state_defaults_cover_mobs_but_not_mannequin() {
    let zombie = GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
    let mob = zombie.mob_retained().expect("zombie owns Mob state");
    assert!(!mob.can_pick_up_loot());
    assert!(!mob.persistence_required());
    for slot in VANILLA_EQUIPMENT_SLOTS {
        assert_eq!(mob.drop_chance(slot.slot), 0.085);
    }

    let mannequin = GeneralLivingData::new(VANILLA_ENTITY_TYPE_MANNEQUIN_ID).unwrap();
    assert!(!mannequin.is_mob());
    assert!(mannequin.mob_retained().is_none());

    let sparse = crate::entity::save::read_mob_retained_state(&[]);
    assert!(!sparse.can_pick_up_loot());
    assert!(!sparse.persistence_required());
    assert!(sparse.has_default_drop_chances());
}

#[test]
fn nbt_creation_reads_mob_defaults_and_preserves_bare_constructors() {
    for (resource_id, bare_flags, loaded_flags) in [
        ("minecraft:fox", (true, false), (false, false)),
        ("minecraft:elder_guardian", (false, true), (false, false)),
        ("minecraft:piglin", (true, false), (true, false)),
        ("minecraft:pillager", (false, false), (true, false)),
    ] {
        let mut store = EntityStore::new(27, EntityIdCounter::new());
        let root = crate::entity::create_fresh_entity(&mut store, resource_id, position()).unwrap();
        let EntityData::GeneralLiving(data) = root.data() else {
            unreachable!()
        };
        let mob = data.mob_retained().unwrap();
        assert_eq!(
            (mob.can_pick_up_loot(), mob.persistence_required()),
            bare_flags,
            "{resource_id}"
        );
        store.discard_fresh(root);
        for configured in [false, true] {
            let mut entries = vec![NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string(resource_id),
            }];
            if configured {
                entries.push(NbtSummaryEntry {
                    name: "NoAI".into(),
                    value: NbtSummaryValue::Byte(1),
                });
            }
            let request = crate::entity::SpawnRequest {
                entity_type_id: bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(
                    resource_id,
                )
                .unwrap(),
                position: position(),
                spawn_data: crate::entity::SpawnerSpawnData::new(entries, None, None),
                spawner_position: BlockPos { x: 0, y: 64, z: 0 },
                spawn_range: 4,
                max_nearby_entities: 6,
            };
            let plan = crate::entity::prepare_spawn_request(&request).unwrap();
            let root = crate::entity::load_fresh_entity_tree(&mut store, &plan).unwrap();
            let EntityData::GeneralLiving(data) = root.data() else {
                unreachable!()
            };
            let mob = data.mob_retained().unwrap();
            assert_eq!(
                (mob.can_pick_up_loot(), mob.persistence_required()),
                loaded_flags,
                "{resource_id}, configured={configured}"
            );
            assert_eq!(data.is_no_ai(), configured);
            store.discard_fresh(root);
        }
    }
}

#[test]
fn mob_retained_state_restore_defaults_absent_vanilla_fields_before_publication() {
    let mut entries = zombie_record(Uuid::from_u128(0x1237));
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "CanPickUpLoot" | "PersistenceRequired" | "LeftHanded"
        )
    });
    let mut store = EntityStore::new(12, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut store, &chunk(entries)).unwrap();
    let EntityData::GeneralLiving(data) = store.get(ids[0]).unwrap().data() else {
        panic!("zombie restores through the general living owner")
    };
    let mob = data.mob_retained().unwrap();
    assert!(!mob.can_pick_up_loot());
    assert!(!mob.persistence_required());
    assert_eq!(mob.flags(), 0);

    let output = write_entity_chunk(&store, (0, 0)).unwrap();
    for name in ["CanPickUpLoot", "PersistenceRequired", "LeftHanded"] {
        assert_eq!(
            output
                .entities()
                .next()
                .unwrap()
                .raw()
                .iter()
                .filter(|entry| entry.name == name)
                .count(),
            1
        );
    }
}

#[test]
fn mob_retained_state_round_trips_canonical_nondefault_drop_chances() {
    let mut entries = zombie_record(Uuid::from_u128(0x1236));
    entries
        .iter_mut()
        .find(|entry| entry.name == "CanPickUpLoot")
        .unwrap()
        .value = NbtSummaryValue::Byte(1);
    entries
        .iter_mut()
        .find(|entry| entry.name == "PersistenceRequired")
        .unwrap()
        .value = NbtSummaryValue::Byte(1);
    entries.push(entry(
        "drop_chances",
        NbtSummaryValue::Compound(vec![
            entry("saddle", NbtSummaryValue::Float(0.25_f32.to_bits())),
            entry("mainhand", NbtSummaryValue::Float(2.0_f32.to_bits())),
            entry("head", NbtSummaryValue::Float(0.0_f32.to_bits())),
        ]),
    ));

    let mut store = EntityStore::new(10, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut store, &chunk(entries)).unwrap();
    let EntityData::GeneralLiving(data) = store.get(ids[0]).unwrap().data() else {
        panic!("zombie restores through the general living owner")
    };
    let mob = data.mob_retained().unwrap();
    assert!(mob.can_pick_up_loot());
    assert!(mob.persistence_required());
    assert_eq!(mob.drop_chance(EquipmentSlot::MainHand), 2.0);
    assert_eq!(mob.drop_chance(EquipmentSlot::Head), 0.0);
    assert_eq!(mob.drop_chance(EquipmentSlot::Saddle), 0.25);
    assert_eq!(mob.drop_chance(EquipmentSlot::OffHand), 0.085);
    assert!(data.opaque_fields().iter().all(|entry| !matches!(
        entry.name.to_jdk_string().as_str(),
        "CanPickUpLoot" | "PersistenceRequired" | "drop_chances"
    )));

    let output = write_entity_chunk(&store, (0, 0)).unwrap();
    let raw = output.entities().next().unwrap().raw();
    for name in ["CanPickUpLoot", "PersistenceRequired", "drop_chances"] {
        assert_eq!(raw.iter().filter(|entry| entry.name == name).count(), 1);
    }
    let drop_chances = raw
        .iter()
        .find(|entry| entry.name == "drop_chances")
        .map(|entry| &entry.value);
    assert_eq!(
        drop_chances,
        Some(&NbtSummaryValue::Compound(vec![
            entry("mainhand", NbtSummaryValue::Float(2.0_f32.to_bits())),
            entry("head", NbtSummaryValue::Float(0.0_f32.to_bits())),
            entry("saddle", NbtSummaryValue::Float(0.25_f32.to_bits())),
        ]))
    );

    let mut reloaded = EntityStore::new(11, EntityIdCounter::new());
    let reloaded_ids = read_entity_chunk(&mut reloaded, &output).unwrap();
    assert_eq!(
        reloaded.get(reloaded_ids[0]).unwrap().data(),
        store.get(ids[0]).unwrap().data()
    );
}
