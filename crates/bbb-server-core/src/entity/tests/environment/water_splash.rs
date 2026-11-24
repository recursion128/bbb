use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::level_event::VanillaGameEvent;

fn base(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
    let mut body = store.detach(id).unwrap();
    assert!(level.with_level(|ctx| crate::entity::base_tick(&mut body, store, ctx)));
    store.attach(body);
}

#[test]
fn water_entry_splashes_once_and_a_silent_body_only_emits_the_game_event() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:cow", FEET);
    store.drain_lifecycle_events();
    let mut wet = sloping_pool("minecraft:water");
    let mut dry = NativeLevel::new();

    base(&mut store, &mut wet, id);
    assert!(store.get(id).unwrap().was_touching_water());
    assert!(store.drain_lifecycle_events().is_empty());
    assert!(splash_events(&wet).is_empty());
    base(&mut store, &mut dry, id);
    let body = store.get_mut(id).unwrap();
    body.set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.5,
        z: 0.0,
    });
    body.set_fall_distance(8.0);
    let mut body = store.detach(id).unwrap();
    assert_eq!(
        wet.with_level(|ctx| crate::entity::movement::move_living_entity(
            &mut body,
            &mut store,
            ctx,
            crate::entity::MoverType::Self_,
            Vec3d {
                y: -0.1,
                ..Vec3d::default()
            }
        )),
        Ok(())
    );
    store.attach(body);
    assert_eq!(store.get(id).unwrap().fall_distance(), 0.0);
    let outputs = store.drain_lifecycle_events();
    let sound = outputs
        .iter()
        .find_map(|event| {
            if let EntityLifecycleEvent::Sound { sound } = event {
                Some(sound)
            } else {
                None
            }
        })
        .expect("splash sound");
    assert_eq!(sound.volume, 0.1);
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.generic.splash"
            )
            .unwrap(),
        }
    );
    assert_eq!(splash_events(&wet).len(), 1);
    assert_eq!(splash_events(&wet)[0].game_event, VanillaGameEvent::SPLASH);
    assert_eq!(splash_events(&wet)[0].source_entity_id, Some(id));
    base(&mut store, &mut wet, id);
    assert!(store.drain_lifecycle_events().is_empty());
    base(&mut store, &mut dry, id);
    store.get_mut(id).unwrap().set_silent(true);
    base(&mut store, &mut wet, id);
    assert!(store.drain_lifecycle_events().is_empty());
    assert_eq!(splash_events(&wet).len(), 2);
}

#[test]
fn water_splash_reads_actual_controlling_passenger_before_fluid_currents() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let boat =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, FEET, 0.0).unwrap();
    let rider = living_at(&mut store, "minecraft:cow", FEET);
    assert!(store.link_existing_passenger(boat, rider,));
    let mut dry = NativeLevel::new();

    base(&mut store, &mut dry, boat);
    store
        .get_mut(boat)
        .unwrap()
        .set_delta_movement(Vec3d::default());
    store.get_mut(rider).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.8,
        z: 0.0,
    });
    store.drain_lifecycle_events();
    let mut wet = sloping_pool("minecraft:water");
    base(&mut store, &mut wet, boat);
    let outputs = store.drain_lifecycle_events();
    let [EntityLifecycleEvent::Sound { sound }] = outputs.as_slice() else {
        panic!("one boat entry splash: {outputs:?}");
    };
    assert_eq!(sound.volume, 0.8_f32 * 0.9_f32);
    assert_eq!(splash_events(&wet)[0].source_entity_id, Some(boat));
    assert_eq!(
        store.get(boat).unwrap().delta_movement(),
        Vec3d {
            x: WATER_CURRENT_SCALE,
            ..Vec3d::default()
        }
    );
}

#[test]
fn water_splash_aquatic_and_zombie_tick_preserve_the_entry_edge() {
    for kind in ["minecraft:dolphin", "minecraft:zombie"] {
        let mut store = EntityStore::new(7, EntityIdCounter::new());
        let id = living_at(&mut store, kind, FEET);
        let body = store.get_mut(id).unwrap();
        body.set_no_gravity(true);
        let EntityData::GeneralLiving(data) = &mut body.data else {
            unreachable!()
        };
        data.set_no_ai(true);
        NativeLevel::new().with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));
        assert!(!store.get(id).unwrap().is_first_tick());
        store.drain_lifecycle_events();
        let mut wet = sloping_pool("minecraft:water");
        wet.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));
        assert!(store.get(id).unwrap().is_in_water());
        assert_eq!(
            splash_events(&wet)
                .iter()
                .filter(|event| event.game_event == VanillaGameEvent::SPLASH)
                .count(),
            1,
            "{kind}"
        );
        assert!(
            store
                .drain_lifecycle_events()
                .iter()
                .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })),
            "{kind}"
        );
    }
}

#[test]
fn boat_shielded_living_air_uses_the_refreshed_tracker() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;
    for (kind, expected_air) in [("minecraft:cod", 19), ("minecraft:rabbit", 24)] {
        let mut store = EntityStore::new(7, EntityIdCounter::new());
        let boat =
            crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, FEET, 0.0)
                .unwrap();
        let rider = living_at(&mut store, kind, FEET);
        assert!(store.link_existing_passenger(boat, rider,));
        let mut body = store.detach(rider).unwrap();
        if kind == "minecraft:rabbit" {
            assert!(crate::entity::general_living::set_spawner_age(
                &mut body, -100
            ));
        }
        assert!(body.bounding_box().max_y <= store.get(boat).unwrap().bounding_box().max_y);
        body.set_air_supply(20);
        body.set_no_gravity(true);
        let EntityData::GeneralLiving(data) = &mut body.data else {
            unreachable!()
        };
        data.set_no_ai(true);
        assert!(sloping_pool("minecraft:water")
            .with_level(|ctx| crate::entity::general_living::tick(&mut body, &mut store, ctx,)));
        assert_eq!(body.air_supply(), expected_air, "{kind}");
        store.attach(body);
    }
}

#[test]
fn experience_orb_water_entry_keeps_the_empty_splash_override() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = store.spawn(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        FEET,
        EntityData::ExperienceOrb(crate::entity::ExperienceOrbData::from_saved(5, 9, 3, 1)),
    );
    store.get_mut(id).unwrap().set_no_gravity(true);
    NativeLevel::new().with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));
    store.drain_lifecycle_events();
    let mut wet = sloping_pool("minecraft:water");
    wet.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));
    assert!(store.get(id).unwrap().was_touching_water());
    assert!(store.drain_lifecycle_events().is_empty());
    assert!(splash_events(&wet).is_empty());
}

fn splash_events(native: &NativeLevel) -> Vec<crate::level_event::RecordedGameEvent> {
    native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .into_iter()
        .map(|event| event.recorded)
        .filter(|event| event.game_event == VanillaGameEvent::SPLASH)
        .collect()
}
