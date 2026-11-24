use super::*;
#[test]
fn ageable_mob_runtime_restore_save_defaults_and_nondefault_state_are_canonical() {
    let mut store = EntityStore::new(0xA60, EntityIdCounter::new());
    let id = read_entity_chunk(
        &mut store,
        &chunk(ageable_mob_record(Uuid::from_u128(0xA601), -17, 23, true)),
    )
    .unwrap()[0];
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!("axolotl uses general living")
    };
    assert_eq!(
        data.ageable_retained(),
        Some(AgeableMobRetainedState::from_saved(-17, 23, true))
    );
    assert!(data.opaque_fields().iter().all(|entry| !matches!(
        entry.name.to_jdk_string().as_str(),
        "Age" | "ForcedAge" | "AgeLocked"
    )));
    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    let raw = saved.entities().next().unwrap().raw();
    for (name, expected) in [
        ("Age", NbtSummaryValue::Int(-17)),
        ("ForcedAge", NbtSummaryValue::Int(23)),
        ("AgeLocked", NbtSummaryValue::Byte(1)),
    ] {
        assert_eq!(
            raw.iter()
                .filter(|entry| entry.name == name)
                .map(|entry| &entry.value)
                .collect::<Vec<_>>(),
            vec![&expected]
        );
    }

    let mut sparse_record = ageable_mob_record(Uuid::from_u128(0xA602), 0, 0, false);
    sparse_record.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "Age" | "ForcedAge" | "AgeLocked"
        )
    });
    let mut sparse = EntityStore::new(0xA61, EntityIdCounter::new());
    let sparse_id = read_entity_chunk(&mut sparse, &chunk(sparse_record)).unwrap()[0];
    let EntityData::GeneralLiving(data) = sparse.get(sparse_id).unwrap().data() else {
        unreachable!("axolotl uses general living")
    };
    assert_eq!(
        data.ageable_retained(),
        Some(AgeableMobRetainedState::from_saved(0, 0, false))
    );
    let saved = write_entity_chunk(&sparse, (0, 0)).unwrap();
    for name in ["Age", "ForcedAge", "AgeLocked"] {
        assert_eq!(
            saved
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
fn ageable_mob_runtime_ticks_after_ai_under_no_ai_and_honors_horse_overrides() {
    let mut store = EntityStore::new(0xA70, EntityIdCounter::new());
    let cases = [
        ("minecraft:cow", -1, 31, false, true),
        ("minecraft:cow", -1, 0, true, false),
        ("minecraft:cow", 1, 0, true, false),
        ("minecraft:skeleton_horse", -1, 0, false, false),
        ("minecraft:zombie_horse", 1, 0, false, false),
    ];
    let mut ids = Vec::new();
    for (resource_id, age, forced_age, age_locked, no_ai) in cases {
        let id = store
            .spawn_general_living(entity_type_id(resource_id), position())
            .unwrap();
        let entity = store.get_mut(id).unwrap();
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!("ageable test type uses general living")
        };
        *data.ageable_retained_mut().unwrap() =
            AgeableMobRetainedState::from_saved(age, forced_age, age_locked);
        data.set_no_ai(no_ai);
        entity.take_synched_dirty();
        ids.push(id);
    }

    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut store, level, true))
    };

    let expected = [
        (0, 31, false, true),
        (-1, 0, true, false),
        (0, 0, true, false),
        (-1, 0, false, false),
        (0, 0, false, false),
    ];
    for (id, (age, forced_age, age_locked, baby_dirty)) in ids.into_iter().zip(expected) {
        let entity = store.get_mut(id).unwrap();
        let EntityData::GeneralLiving(data) = entity.data() else {
            unreachable!("ageable test type uses general living")
        };
        assert_eq!(
            data.ageable_retained(),
            Some(AgeableMobRetainedState::from_saved(
                age, forced_age, age_locked
            ))
        );
        assert_eq!(
            entity
                .take_synched_dirty()
                .contains(&VANILLA_AGEABLE_MOB_BABY_DATA_ID),
            baby_dirty
        );
    }
}

#[test]
fn animal_love_runtime_restore_save_defaults() {
    let love_cause = Uuid::from_u128(0xA710);
    let mut store = EntityStore::new(0xA71, EntityIdCounter::new());
    let id = read_entity_chunk(
        &mut store,
        &chunk(animal_record(
            Uuid::from_u128(0xA711),
            0,
            37,
            Some(love_cause),
        )),
    )
    .unwrap()[0];
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!("axolotl uses general living")
    };
    assert_eq!(
        data.animal_retained(),
        Some(AnimalRetainedState::from_saved(37, Some(love_cause)))
    );
    assert!(data
        .opaque_fields()
        .iter()
        .all(|entry| !matches!(entry.name.to_jdk_string().as_str(), "InLove" | "LoveCause")));
    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    let raw = saved.entities().next().unwrap().raw();
    assert_eq!(
        raw.iter()
            .filter(|entry| entry.name == "InLove")
            .map(|entry| &entry.value)
            .collect::<Vec<_>>(),
        vec![&NbtSummaryValue::Int(37)]
    );
    assert_eq!(
        raw.iter()
            .find(|entry| entry.name == "LoveCause")
            .map(|entry| &entry.value),
        Some(&saved_uuid(love_cause))
    );

    let mut sparse_record = animal_record(Uuid::from_u128(0xA712), 0, 0, None);
    sparse_record.retain(|entry| entry.name != "InLove");
    let mut sparse = EntityStore::new(0xA72, EntityIdCounter::new());
    let sparse_id = read_entity_chunk(&mut sparse, &chunk(sparse_record)).unwrap()[0];
    let EntityData::GeneralLiving(data) = sparse.get(sparse_id).unwrap().data() else {
        unreachable!("axolotl uses general living")
    };
    assert_eq!(
        data.animal_retained(),
        Some(AnimalRetainedState::from_saved(0, None))
    );
    assert!(write_entity_chunk(&sparse, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .iter()
        .any(|entry| entry.name == "InLove" && entry.value == NbtSummaryValue::Int(0)));
}

#[test]
fn animal_love_runtime_decays_love_and_clears_it_for_non_adults() {
    let mut store = EntityStore::new(0xA74, EntityIdCounter::new());
    let mut cases = Vec::new();
    cases.extend([
        ("minecraft:bee", false, -1, 20),
        ("minecraft:cow", false, -1, 0),
        ("minecraft:cow", true, -1, 20),
        ("minecraft:cow", false, 0, 20),
    ]);

    let mut ids = Vec::new();
    for (resource_id, no_ai, age, expected_love) in cases {
        let id = store
            .spawn_general_living(entity_type_id(resource_id), position())
            .unwrap();
        let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data else {
            unreachable!("Animal uses general living")
        };
        *data.ageable_retained_mut().unwrap() = AgeableMobRetainedState::from_saved(age, 0, false);
        *data.animal_retained_mut().unwrap() = AnimalRetainedState::from_saved(21, None);
        data.set_no_ai(no_ai);
        ids.push((id, expected_love));
    }

    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut store, level, true))
    };

    for (id, expected_love) in ids {
        let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
            unreachable!("Animal uses general living")
        };
        assert_eq!(data.ageable_retained().unwrap().age(), 0);
        assert_eq!(data.animal_retained().unwrap().in_love(), expected_love);
    }
}

#[test]
fn animal_love_runtime_damage_resets_after_cooldown_except_wolf_armor() {
    let love_cause = Uuid::from_u128(0xA750);
    let mut store = EntityStore::new(0xA75, EntityIdCounter::new());
    let cow = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    let EntityData::GeneralLiving(data) = &mut store.get_mut(cow).unwrap().data else {
        unreachable!("cow uses general living")
    };
    *data.animal_retained_mut().unwrap() = AnimalRetainedState::from_saved(40, Some(love_cause));
    let generic = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:generic")
        .unwrap();
    let mut events = LevelEventSink::default();
    store
        .hurt_server(
            cow,
            StoredEntityDamageSource {
                damage_type: generic,
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            3.0,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut events,
        )
        .unwrap()
        .unwrap();
    let EntityData::GeneralLiving(data) = store.get(cow).unwrap().data() else {
        unreachable!("cow uses general living")
    };
    assert_eq!(
        data.animal_retained(),
        Some(AnimalRetainedState::from_saved(0, Some(love_cause)))
    );

    let EntityData::GeneralLiving(data) = &mut store.get_mut(cow).unwrap().data else {
        unreachable!("cow uses general living")
    };
    *data.animal_retained_mut().unwrap() = AnimalRetainedState::from_saved(30, Some(love_cause));
    let refused = store
        .hurt_server(
            cow,
            StoredEntityDamageSource {
                damage_type: generic,
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            3.0,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut events,
        )
        .unwrap()
        .unwrap();
    assert!(!refused.applied);
    let EntityData::GeneralLiving(data) = store.get(cow).unwrap().data() else {
        unreachable!("cow uses general living")
    };
    assert_eq!(data.animal_retained().unwrap().in_love(), 30);

    for (index, damage_id) in ["minecraft:generic", "minecraft:generic_kill"]
        .into_iter()
        .enumerate()
    {
        let wolf = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_WOLF_ID, position())
            .unwrap();
        let entity = store.get_mut(wolf).unwrap();
        entity.living_mut().unwrap().set_item_slot(
            EquipmentSlot::Body,
            crate::entity::item::plain_stack("minecraft:wolf_armor", 1),
        );
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!("wolf uses general living")
        };
        *data.animal_retained_mut().unwrap() =
            AnimalRetainedState::from_saved(50 + index as i32, None);
        let damage_type = builtin_damage_type_registry()
            .iter()
            .find(|facts| facts.resource_id == damage_id)
            .unwrap();
        store
            .hurt_server(
                wolf,
                StoredEntityDamageSource {
                    damage_type,
                    direct_entity_id: None,
                    causing_entity: None,
                    attacker_views: Default::default(),
                },
                1.0,
                StoredEntityDamageRules::default(),
                crate::world::registry_runtime::item_stack_resolver_for_test(),
                &mut events,
            )
            .unwrap()
            .unwrap();
        let EntityData::GeneralLiving(data) = store.get(wolf).unwrap().data() else {
            unreachable!("wolf uses general living")
        };
        assert_eq!(
            data.animal_retained().unwrap().in_love(),
            if damage_id == "minecraft:generic" {
                50
            } else {
                0
            }
        );
    }
}

#[test]
fn abstract_horse_runtime_restore_save_defaults() {
    let owner = Uuid::from_u128(0xA850);
    let mut store = EntityStore::new(0xA85, EntityIdCounter::new());
    let id = read_entity_chunk(
        &mut store,
        &chunk(abstract_horse_record(
            Uuid::from_u128(0xA851),
            Some(true),
            Some(true),
            Some(37),
            Some(false),
            Some(owner),
        )),
    )
    .unwrap()[0];
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!("zombie horse uses general living")
    };
    let horse = data.abstract_horse_retained().unwrap();
    assert_eq!(
        horse.flags(),
        VANILLA_ABSTRACT_HORSE_FLAG_EATING | VANILLA_ABSTRACT_HORSE_FLAG_BRED
    );
    assert_eq!((horse.temper(), horse.owner()), (37, Some(owner)));
    assert!(data.opaque_fields().iter().all(|field| !matches!(
        field.name.to_jdk_string().as_str(),
        "EatingHaystack" | "Bred" | "Temper" | "Tame" | "Owner"
    )));
    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    let raw = saved.entities().next().unwrap().raw();
    assert_eq!(
        raw.iter()
            .find(|field| field.name == "Owner")
            .map(|field| &field.value),
        Some(&saved_uuid(owner))
    );

    let mut sparse = EntityStore::new(0xA86, EntityIdCounter::new());
    let sparse_id = read_entity_chunk(
        &mut sparse,
        &chunk(abstract_horse_record(
            Uuid::from_u128(0xA852),
            None,
            None,
            None,
            None,
            None,
        )),
    )
    .unwrap()[0];
    let EntityData::GeneralLiving(data) = sparse.get(sparse_id).unwrap().data() else {
        unreachable!("zombie horse uses general living")
    };
    assert_eq!(
        data.abstract_horse_retained(),
        Some(&AbstractHorseRetainedState::from_saved(
            false, false, 0, false, None
        ))
    );
    let sparse_saved = write_entity_chunk(&sparse, (0, 0)).unwrap();
    for name in ["EatingHaystack", "Bred", "Temper", "Tame"] {
        assert_eq!(
            sparse_saved
                .entities()
                .next()
                .unwrap()
                .raw()
                .iter()
                .filter(|field| field.name == name)
                .count(),
            1
        );
    }
}

#[test]
fn abstract_horse_runtime_heals_grazes_and_stops_eating_movement() {
    let mut store = EntityStore::new(0xA91, EntityIdCounter::new());
    let heal = restore_test_horse(
        &mut store,
        Uuid::from_u128(0x2000_0000),
        VANILLA_ENTITY_TYPE_HORSE_ID,
        AbstractHorseRetainedState::from_saved(false, false, 0, false, None),
    );
    store
        .get_mut(heal)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(5.0);
    let grass = restore_test_horse(
        &mut store,
        Uuid::from_u128(0x3000_0000),
        VANILLA_ENTITY_TYPE_HORSE_ID,
        AbstractHorseRetainedState::from_saved(false, false, 0, false, None),
    );
    let existing = restore_test_horse(
        &mut store,
        Uuid::from_u128(0x4000_0000),
        VANILLA_ENTITY_TYPE_HORSE_ID,
        AbstractHorseRetainedState::from_saved(true, false, 0, false, None),
    );
    store.get_mut(existing).unwrap().set_pos(Vec3d {
        x: 2.5,
        ..position()
    });
    store.get_mut(existing).unwrap().living_mut().unwrap().xxa = 1.0;
    let llama = restore_test_horse(
        &mut store,
        Uuid::from_u128(0x5000_0000),
        VANILLA_ENTITY_TYPE_LLAMA_ID,
        AbstractHorseRetainedState::from_saved(true, false, 0, false, None),
    );
    store.get_mut(llama).unwrap().living_mut().unwrap().xxa = 1.0;
    let mut native = NativeLevel::new();
    native.set(
        BlockPos { x: 0, y: 63, z: 0 },
        state_id("minecraft:grass_block"),
    );
    native.with_level(|level| tick_entities(&mut store, level, true));
    for id in [existing, llama] {
        assert_eq!(store.get(id).unwrap().living().unwrap().xxa, 0.0);
    }
    let mut grazed = false;
    for _ in 0..5000 {
        native.with_level(|level| tick_entities(&mut store, level, true));
        let EntityData::GeneralLiving(data) = store.get(grass).unwrap().data() else {
            unreachable!()
        };
        grazed |= data.abstract_horse_retained().unwrap().is_eating();
        if grazed && store.get(heal).unwrap().living().unwrap().health() > 5.0 {
            break;
        }
    }
    assert!(
        grazed,
        "a supported horse starts eating the actual grass below it"
    );
    assert!(store.get(heal).unwrap().living().unwrap().health() > 5.0);
}

#[test]
fn abstract_horse_runtime_damage_rears_and_rearing_exclusions_hold() {
    let generic = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:generic")
        .unwrap();
    let mut store = EntityStore::new(0xA94, EntityIdCounter::new());
    let mut horses = Vec::new();
    for value in 0x6000_0000..0x6000_0020 {
        horses.push(restore_test_horse(
            &mut store,
            Uuid::from_u128(value),
            VANILLA_ENTITY_TYPE_HORSE_ID,
            AbstractHorseRetainedState::from_saved(true, false, 0, false, None),
        ));
    }
    let camel = restore_test_horse(
        &mut store,
        Uuid::from_u128(0x7000_0000),
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        AbstractHorseRetainedState::from_saved(true, false, 0, false, None),
    );
    let mut sink = LevelEventSink::default();
    for id in horses.iter().copied().chain([camel]) {
        let outcome = store
            .hurt_server(
                id,
                StoredEntityDamageSource {
                    damage_type: generic,
                    direct_entity_id: None,
                    causing_entity: None,
                    attacker_views: Default::default(),
                },
                1.0,
                StoredEntityDamageRules::default(),
                crate::world::registry_runtime::item_stack_resolver_for_test(),
                &mut sink,
            )
            .unwrap()
            .unwrap();
        assert!(outcome.succeeded);
    }
    let retained = |id| match store.get(id).unwrap().data() {
        EntityData::GeneralLiving(data) => data.abstract_horse_retained().unwrap(),
        _ => unreachable!(),
    };
    assert!(horses
        .iter()
        .any(|id| retained(*id).is_standing() && !retained(*id).is_eating()));
    assert!(!retained(camel).is_standing());
    let mut burning = EntityStore::new(0xA95, EntityIdCounter::new());
    let mut ids = Vec::new();
    for value in 0x8000_0000..0x8000_0020 {
        let id = restore_test_horse(
            &mut burning,
            Uuid::from_u128(value),
            VANILLA_ENTITY_TYPE_HORSE_ID,
            AbstractHorseRetainedState::from_saved(false, false, 0, false, None),
        );
        burning.get_mut(id).unwrap().set_remaining_fire_ticks(20);
        ids.push(id);
    }
    let mut native = NativeLevel::new();
    native.with_level(|level| tick_entities(&mut burning, level, true));
    assert!(ids.iter().any(|id| matches!(burning.get(*id).unwrap().data(),EntityData::GeneralLiving(data) if data.abstract_horse_retained().unwrap().is_standing())));
}
