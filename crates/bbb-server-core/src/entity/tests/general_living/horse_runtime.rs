use super::*;
fn random_stand_horse(entity_type_id: i32, uuid: Uuid, baby: bool) -> (EntityStore, i32) {
    let mut data = GeneralLivingData::new(entity_type_id).unwrap();
    if baby {
        *data.ageable_retained_mut().unwrap() =
            AgeableMobRetainedState::from_saved(-24_000, 0, false);
    }
    let mut store = EntityStore::new(0xA95, EntityIdCounter::new());
    let id = store
        .restore(
            uuid,
            entity_type_id,
            position(),
            EntityData::GeneralLiving(data),
        )
        .unwrap();
    store.drain_lifecycle_events();
    (store, id)
}

#[test]
fn horse_random_stand_applies_state_and_exact_ambient_sound_family() {
    let cases = [
        (
            VANILLA_ENTITY_TYPE_HORSE_ID,
            false,
            false,
            "minecraft:entity.horse.ambient",
        ),
        (
            VANILLA_ENTITY_TYPE_HORSE_ID,
            true,
            false,
            "minecraft:entity.baby_horse.ambient",
        ),
        (
            VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
            false,
            true,
            "minecraft:entity.skeleton_horse.ambient_water",
        ),
    ];

    for (entity_type_id, baby, water_at_eye, expected_sound) in cases {
        let (mut store, id) =
            random_stand_horse(entity_type_id, Uuid::from_u128(0xA950_0000), baby);
        let entity = store.get_mut(id).unwrap();
        entity.set_no_gravity(true);
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            data.set_no_ai(true);
        }
        let mut level = NativeLevel::new();
        if water_at_eye {
            let entity = store.get(id).unwrap();
            let eye_y = entity.position().y
                + f64::from(
                    bbb_protocol::entity_types::vanilla_entity_type_eye_height(entity_type_id)
                        .unwrap(),
                );
            level.set(
                bbb_protocol::packets::BlockPos {
                    x: 0,
                    y: eye_y.floor() as i32,
                    z: 0,
                },
                state_id("minecraft:water"),
            );
        }

        level.with_level(|level| tick_entities(&mut store, level, true));
        assert_eq!(
            store
                .get(id)
                .unwrap()
                .is_eye_in_fluid(crate::entity::environment::FluidTag::Water),
            water_at_eye
        );
        store.drain_lifecycle_events();
        let mut entity = store.detach(id).unwrap();
        apply_abstract_horse_random_stand(&mut entity, &mut store);
        store.attach(entity);
        let sound = store
            .drain_lifecycle_events()
            .into_iter()
            .find_map(|event| match event {
                EntityLifecycleEvent::Sound { sound } => Some(sound),
                _ => None,
            })
            .expect("RandomStand start queues the ambient sound");
        let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
            unreachable!("horse uses general living")
        };
        let horse = data.abstract_horse_retained().unwrap();
        assert!(horse.is_standing());
        assert!(!horse.is_eating());
        assert_eq!(horse.flags() & VANILLA_ABSTRACT_HORSE_FLAG_STANDING, 32);
        assert_eq!(sound.source, SoundSource::Neutral);
        assert_eq!((sound.volume, sound.pitch), (1.0, 1.0));
        assert_eq!(
            sound.sound,
            SoundEventHolder::Reference {
                registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                    expected_sound
                )
                .unwrap(),
            }
        );
        assert!(store
            .get_mut(id)
            .unwrap()
            .take_synched_dirty()
            .contains(&VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID));
    }
}

#[test]
fn abstract_horse_ambient_sound_family_and_pitch_range() {
    let cases = [
        (
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            false,
            false,
            "minecraft:entity.camel.ambient",
        ),
        (
            VANILLA_ENTITY_TYPE_HORSE_ID,
            true,
            false,
            "minecraft:entity.baby_horse.ambient",
        ),
        (
            VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
            false,
            true,
            "minecraft:entity.skeleton_horse.ambient_water",
        ),
        (
            VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
            false,
            false,
            "minecraft:entity.llama.ambient",
        ),
    ];

    for (case, (entity_type_id, baby, water_at_eye, expected_sound)) in
        cases.into_iter().enumerate()
    {
        let uuid = Uuid::from_u128(0xA952_0000 + case as u128);
        let (mut store, id) = random_stand_horse(entity_type_id, uuid, baby);
        let entity = store.get_mut(id).unwrap();
        entity.set_no_gravity(true);
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            data.set_no_ai(true);
        }
        let mut level = NativeLevel::new();
        if water_at_eye {
            let entity = store.get(id).unwrap();
            let eye_y = entity.position().y
                + f64::from(
                    bbb_protocol::entity_types::vanilla_entity_type_eye_height(entity_type_id)
                        .unwrap(),
                );
            level.set(
                bbb_protocol::packets::BlockPos {
                    x: 0,
                    y: eye_y.floor() as i32,
                    z: 0,
                },
                state_id("minecraft:water"),
            );
        }

        let expected = SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                expected_sound,
            )
            .unwrap(),
        };
        let mut heard = None;
        for _ in 0..1000 {
            level.with_level(|level| tick_entities(&mut store, level, true));
            heard = store
                .drain_lifecycle_events()
                .into_iter()
                .find_map(|event| match event {
                    EntityLifecycleEvent::Sound { sound } if sound.sound == expected => Some(sound),
                    _ => None,
                });
            if heard.is_some() {
                break;
            }
        }
        assert_eq!(
            store
                .get(id)
                .unwrap()
                .is_eye_in_fluid(crate::entity::environment::FluidTag::Water),
            water_at_eye
        );
        let sound = heard.expect("ordinary horse ticks emit the matching ambient sound");
        let base = if baby { 1.5 } else { 1.0 };
        assert_eq!(sound.source, SoundSource::Neutral);
        assert_eq!(sound.volume, 0.8);
        assert!((sound.pitch - base).abs() <= 0.2, "{}", sound.pitch);
        assert_eq!(
            sound.sound,
            SoundEventHolder::Reference {
                registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                    expected_sound
                )
                .unwrap(),
            }
        );
    }
}

#[test]
fn base_entity_observable_state_silent_suppresses_sound() {
    let uuid = Uuid::from_u128(0xA981);
    let (mut store, id) = random_stand_horse(VANILLA_ENTITY_TYPE_HORSE_ID, uuid, false);
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_silent(true);
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!("horse uses general living")
        };
        data.set_no_ai(true);
    }
    store.mob_ai_mut(id).unwrap().set_ambient_sound_time(1000);

    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut store, level, true))
    };

    assert!(!store
        .drain_lifecycle_events()
        .iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
}

#[test]
fn horse_random_stand_no_ai_and_immobile_paths_emit_nothing() {
    let uuid = Uuid::from_u128(0xA951_0000);

    let mut no_ai_data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_HORSE_ID).unwrap();
    no_ai_data.set_no_ai(true);
    let mut no_ai = EntityStore::new(0xA96, EntityIdCounter::new());
    no_ai
        .restore(
            uuid,
            VANILLA_ENTITY_TYPE_HORSE_ID,
            position(),
            EntityData::GeneralLiving(no_ai_data),
        )
        .unwrap();
    no_ai.drain_lifecycle_events();
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut no_ai, level, true))
    };
    assert!(no_ai.drain_lifecycle_events().is_empty());

    let (mut immobile, immobile_id) = random_stand_horse(VANILLA_ENTITY_TYPE_HORSE_ID, uuid, false);
    let EntityData::GeneralLiving(data) = &mut immobile.get_mut(immobile_id).unwrap().data else {
        unreachable!("horse uses general living")
    };
    *data.abstract_horse_retained_mut().unwrap() =
        AbstractHorseRetainedState::from_saved(true, false, 0, false, None);
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut immobile, level, true))
    };
    assert!(immobile.drain_lifecycle_events().is_empty());
}
