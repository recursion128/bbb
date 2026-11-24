use super::*;

fn landing_body() -> (EntityStore, NativeLevel, Box<Entity>) {
    let (mut store, id) = walking_store(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let mut entity = store.detach(id).unwrap();
    entity.fall_distance = 5.0;
    entity.set_delta_movement(HALF_BLOCK);
    store.drain_lifecycle_events();
    (store, gold_plate(), entity)
}

fn sound_id(name: &str) -> i32 {
    bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(name).unwrap()
}

#[test]
fn living_air_travel_landing_takes_fall_damage_and_plays_its_fall_sound() {
    let (mut store, mut level, mut entity) = landing_body();
    level
        .with_level(|ctx| {
            crate::entity::living::travel_in_air(&mut entity, &mut store, ctx, Vec3d::default())
        })
        .unwrap();
    assert_eq!(entity.living().unwrap().health(), 18.0);
    assert_eq!(entity.fall_distance(), 0.0);
    let sounds = queued_sounds(&mut store);
    {
        let name = "minecraft:entity.hostile.small_fall";
        assert!(sounds
            .iter()
            .any(|sound| sound.registry_id == sound_id(name)));
    }
    {
        let game_event = VanillaGameEvent::ENTITY_DAMAGE;
        assert!(game_events(&level)
            .iter()
            .any(|event| event.game_event == game_event));
    }
}

#[test]
fn an_airborne_move_preserves_health_and_fall_distance() {
    let (mut store, mut level, mut entity) = landing_body();
    let up = Vec3d {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    assert_eq!(
        level.with_level(|ctx| crate::entity::movement::move_living_entity(
            &mut entity,
            &mut store,
            ctx,
            crate::entity::MoverType::Self_,
            up
        )),
        Ok(())
    );
    assert_eq!(entity.position().y, FEET.y + 1.0);
    assert_eq!(entity.fall_distance(), 5.0);
    assert_eq!(entity.living().unwrap().health(), 20.0);
}

#[test]
fn armor_stand_fall_removal_skips_block_velocity_response() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = crate::entity::spawn_armor_stand(&mut store, FEET).unwrap();
    let mut entity = store.detach(id).unwrap();
    entity.fall_distance = 5.0;
    entity.set_delta_movement(HALF_BLOCK);
    let mut level = gold_plate();
    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:damage_type",
        "minecraft:always_kills_armor_stands",
        &["minecraft:fall"],
    )]));
    store.drain_lifecycle_events();
    assert_eq!(
        level.with_level(|ctx| crate::entity::movement::move_living_entity(
            &mut entity,
            &mut store,
            ctx,
            crate::entity::MoverType::Self_,
            HALF_BLOCK
        )),
        Ok(())
    );
    assert_eq!(entity.removal_reason(), Some(RemovalReason::Killed));
    assert_eq!(entity.delta_movement(), HALF_BLOCK);
    assert_eq!(entity.fall_distance(), 0.0);
    assert!(game_events(&level)
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::ENTITY_DIE));
    assert!(!game_events(&level)
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::STEP));
}

fn fall_provider_body(stand: bool) -> (EntityStore, NativeLevel, Box<Entity>) {
    if !stand {
        return landing_body();
    }
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = crate::entity::spawn_armor_stand(&mut store, FEET).unwrap();
    store.drain_lifecycle_events();
    let entity = store.detach(id).unwrap();
    (store, gold_plate(), entity)
}

fn provider_fall(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut NativeLevel,
    distance: f64,
    modifier: f32,
    damage_type: &'static str,
) -> Option<bool> {
    level.with_level(|ctx| {
        crate::entity::living::cause_fall_damage(
            entity,
            store,
            ctx,
            distance,
            modifier,
            damage_type,
        )
    })
}

#[test]
fn living_fall_provider_uses_modifier_and_actual_damage_source() {
    for damage_type in ["minecraft:fall", "minecraft:stalagmite"] {
        for (modifier, damage) in [(0.0, 0.0), (2.0, 10.0)] {
            let (mut store, mut level, mut entity) = landing_body();
            provider_fall(
                &mut entity,
                &mut store,
                &mut level,
                8.0,
                modifier,
                damage_type,
            );
            assert_eq!(entity.living().unwrap().health(), 20.0 - damage);
            assert_eq!(
                entity.living().unwrap().last_damage_type.as_deref(),
                (damage > 0.0).then_some(damage_type)
            );
            if damage == 0.0 {
                assert!(game_events(&level).is_empty());
            } else {
                assert!(game_events(&level)
                    .iter()
                    .any(|event| event.game_event == VanillaGameEvent::ENTITY_DAMAGE));
            }
        }
    }
}

#[test]
fn invulnerable_living_and_armor_stands_preserve_health_but_play_fall_sounds() {
    for stand in [false, true] {
        for invulnerable in [false, true] {
            let (mut store, mut level, mut entity) = fall_provider_body(stand);
            entity.set_invulnerable(invulnerable);
            let health = entity.living().unwrap().health();
            provider_fall(
                &mut entity,
                &mut store,
                &mut level,
                8.0,
                1.0,
                "minecraft:fall",
            );
            if stand || invulnerable {
                assert_eq!(entity.living().unwrap().health(), health);
            }
            assert!(!queued_sounds(&mut store).is_empty());
        }
        let (mut store, mut level, mut entity) = fall_provider_body(stand);
        let health = entity.living().unwrap().health();
        provider_fall(
            &mut entity,
            &mut store,
            &mut level,
            8.0,
            0.0,
            "minecraft:stalagmite",
        );

        assert_eq!(entity.living().unwrap().health(), health);
        assert!(game_events(&level).is_empty());
    }
}

#[test]
fn armor_stand_fall_provider_selects_stalagmite_for_live_hurt_tags() {
    for damage_type in ["minecraft:fall", "minecraft:stalagmite"] {
        let (mut store, mut level, mut entity) = fall_provider_body(true);
        level.install_tags(registry_tag_snapshot(&[(
            "minecraft:damage_type",
            "minecraft:always_kills_armor_stands",
            &["minecraft:stalagmite"],
        )]));
        provider_fall(&mut entity, &mut store, &mut level, 8.0, 1.0, damage_type);
        assert_eq!(
            entity.removal_reason(),
            (damage_type == "minecraft:stalagmite").then_some(RemovalReason::Killed)
        );
        assert_eq!(
            game_events(&level)
                .iter()
                .any(|event| event.game_event == VanillaGameEvent::ENTITY_DIE),
            damage_type == "minecraft:stalagmite"
        );
    }
}

fn with_fall_passenger(mut store: EntityStore, entity: Box<Entity>) -> (EntityStore, Box<Entity>) {
    let id = entity.id();
    store.attach(entity);
    let passenger = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, FEET)
        .unwrap();
    assert!(store.link_existing_passenger_forced(id, passenger));
    store.drain_lifecycle_events();
    let entity = store.detach(id).unwrap();
    (store, entity)
}

#[test]
fn living_and_armor_stand_fall_provider_propagate_to_stored_passengers() {
    for stand in [false, true] {
        let (store, mut level, entity) = fall_provider_body(stand);
        let (mut store, mut entity) = with_fall_passenger(store, entity);
        let passenger = store.passengers(entity.id())[0];
        let initial_health = entity.living().unwrap().health();
        provider_fall(
            &mut entity,
            &mut store,
            &mut level,
            8.0,
            0.0,
            "minecraft:fall",
        );
        assert_eq!(entity.living().unwrap().health(), initial_health);
        assert_eq!(
            store.get(passenger).unwrap().living().unwrap().health(),
            20.0
        );

        provider_fall(
            &mut entity,
            &mut store,
            &mut level,
            8.0,
            1.0,
            "minecraft:fall",
        );
        assert_eq!(
            store.get(passenger).unwrap().living().unwrap().health(),
            15.0
        );
    }
}

#[test]
fn living_fall_provider_immune_creeper_swells_without_hurting_its_passengers() {
    let (mut store, id) = walking_store(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREEPER_ID);
    let entity = store.detach(id).unwrap();
    let (mut store, mut entity) = with_fall_passenger(store, entity);
    let passenger = store.passengers(entity.id())[0];
    let mut level = gold_plate();
    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:entity_type",
        "minecraft:fall_damage_immune",
        &["minecraft:creeper"],
    )]));
    provider_fall(
        &mut entity,
        &mut store,
        &mut level,
        5.0,
        1.0,
        "minecraft:fall",
    );
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    assert_eq!(data.creeper_retained().unwrap().swell(), 7);
    assert_eq!(entity.living().unwrap().health(), 20.0);
    assert_eq!(
        store.get(passenger).unwrap().living().unwrap().health(),
        20.0
    );

    assert!(game_events(&level).is_empty());
}

#[test]
fn living_fall_provider_keeps_goat_and_frog_calculation_overrides() {
    for (kind, damage) in [
        (bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID, 0.0),
        (bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FROG_ID, 5.0),
    ] {
        let (mut store, id) = walking_store(kind);
        let mut entity = store.detach(id).unwrap();
        let health = entity.living().unwrap().health();
        let mut level = gold_plate();
        provider_fall(
            &mut entity,
            &mut store,
            &mut level,
            13.0,
            1.0,
            "minecraft:fall",
        );
        assert_eq!(entity.living().unwrap().health(), health - damage);
    }
}

#[test]
fn fall_sounds_follow_runtime_inheritance_not_monster_category() {
    for (name, hostile) in [
        ("minecraft:zombie", true),
        ("minecraft:zoglin", true),
        ("minecraft:hoglin", false),
        ("minecraft:camel_husk", false),
        ("minecraft:slime", false),
        ("minecraft:cow", false),
    ] {
        let kind =
            bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(name).unwrap();
        let (mut store, id) = walking_store(kind);
        let entity = store.detach(id).unwrap();
        for big in [false, true] {
            let expected = match (hostile, big) {
                (true, true) => "minecraft:entity.hostile.big_fall",
                (true, false) => "minecraft:entity.hostile.small_fall",
                (false, true) => "minecraft:entity.generic.big_fall",
                (false, false) => "minecraft:entity.generic.small_fall",
            };
            assert_eq!(
                crate::entity::living::fall_sound_for(&entity, big),
                expected,
                "{name}"
            );
        }
    }
}

fn fall_subtype_body(name: &str) -> (EntityStore, NativeLevel, Box<Entity>) {
    let kind = bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(name).unwrap();
    let (mut store, id) = walking_store(kind);
    let entity = store.detach(id).unwrap();
    store.drain_lifecycle_events();
    (store, gold_plate(), entity)
}

fn standard_fall_attributes(entity: &mut Entity) {
    for (attribute, value) in [
        (
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID,
            3.0,
        ),
        (
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FALL_DAMAGE_MULTIPLIER_ID,
            1.0,
        ),
    ] {
        entity
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(attribute)
            .unwrap()
            .set_base_value(value);
    }
}

#[test]
fn horse_fall_prefix_threshold_baby_and_camel_husk_adult_override() {
    for name in ["minecraft:horse", "minecraft:camel", "minecraft:camel_husk"] {
        for baby in [false, true] {
            for distance in [1.0, 1.01] {
                let (mut store, mut level, mut entity) = fall_subtype_body(name);
                if baby {
                    let EntityData::GeneralLiving(data) = &mut entity.data else {
                        unreachable!()
                    };
                    data.ageable_retained_mut().unwrap().set_age(-100);
                }
                provider_fall(
                    &mut entity,
                    &mut store,
                    &mut level,
                    distance,
                    0.0,
                    "minecraft:fall",
                );
                let sounds = queued_sounds(&mut store);
                if distance == 1.0 {
                    assert!(sounds.is_empty());
                } else {
                    let sound = if baby && name != "minecraft:camel_husk" {
                        "minecraft:entity.baby_horse.land"
                    } else {
                        "minecraft:entity.horse.land"
                    };
                    assert!(sounds
                        .iter()
                        .any(|actual| actual == &(queued(sound, entity.position(), 0.4, 1.0))));
                }
                assert!(game_events(&level).is_empty());
            }
        }
    }
}

#[test]
fn llama_fall_five_and_six_use_attributes_but_only_six_hurts() {
    for name in ["minecraft:llama"] {
        for (distance, modifier, positive, damage) in [
            (5.0, 1.0, true, 0.0),
            (6.0, 1.0, true, 3.0),
            (6.0, 0.2, false, 0.0),
            (6.0, 2.0, true, 6.0),
        ] {
            let (mut store, mut level, mut entity) = fall_subtype_body(name);
            standard_fall_attributes(&mut entity);
            let health = entity.living().unwrap().health();
            provider_fall(
                &mut entity,
                &mut store,
                &mut level,
                distance,
                modifier,
                "minecraft:fall",
            );
            assert_eq!(entity.living().unwrap().health(), health - damage);
            let sounds = queued_sounds(&mut store);
            assert_eq!(
                sounds
                    .iter()
                    .any(|sound| sound.registry_id == sound_id("minecraft:block.metal.fall")),
                positive
            );
            assert!(sounds.iter().all(|sound| ![
                sound_id("minecraft:entity.horse.land"),
                sound_id("minecraft:entity.generic.small_fall"),
                sound_id("minecraft:entity.generic.big_fall")
            ]
            .contains(&sound.registry_id)));
            if positive {
                assert_eq!(
                    sounds
                        .iter()
                        .find(|s| s.registry_id == sound_id("minecraft:block.metal.fall"))
                        .unwrap()
                        .registry_id,
                    sound_id("minecraft:block.metal.fall")
                );
            }
            assert_eq!(
                game_events(&level)
                    .iter()
                    .any(|event| event.game_event == VanillaGameEvent::ENTITY_DAMAGE),
                damage > 0.0
            );
        }
    }
}

#[test]
fn horse_fall_defaults_hurt_refusal_and_death_play_the_block_fall_sound() {
    for invulnerable in [false, true] {
        let (mut store, mut level, mut entity) = fall_subtype_body("minecraft:horse");
        entity.set_invulnerable(invulnerable);
        let health = entity.living().unwrap().health();
        provider_fall(
            &mut entity,
            &mut store,
            &mut level,
            12.0,
            1.0,
            "minecraft:fall",
        );
        assert_eq!(
            entity.living().unwrap().health(),
            health - if invulnerable { 0.0 } else { 3.0 }
        );
        let sounds = queued_sounds(&mut store);
        assert_eq!(
            sounds
                .iter()
                .find(|s| s.registry_id == sound_id("minecraft:entity.horse.land"))
                .unwrap(),
            &queued("minecraft:entity.horse.land", entity.position(), 0.4, 1.0)
        );
        assert_eq!(
            sounds
                .iter()
                .find(|s| s.registry_id == sound_id("minecraft:block.metal.fall"))
                .unwrap(),
            &queued("minecraft:block.metal.fall", entity.position(), 0.5, 1.125)
        );
        assert!(sounds
            .iter()
            .all(|sound| sound.registry_id != sound_id("minecraft:entity.generic.small_fall")));
    }
    for name in ["minecraft:horse", "minecraft:llama"] {
        let (mut store, mut level, mut entity) = fall_subtype_body(name);
        entity.living_mut().unwrap().set_health(1.0);
        provider_fall(
            &mut entity,
            &mut store,
            &mut level,
            12.0,
            1.0,
            "minecraft:fall",
        );
        assert!(entity.living().unwrap().is_dead_or_dying());
        assert!(game_events(&level)
            .iter()
            .any(|event| event.game_event == VanillaGameEvent::ENTITY_DIE));
    }
}

#[test]
fn breeze_fall_prefix_obeys_live_immunity_and_sound_source() {
    for distance in [3.0, 3.01, 8.0] {
        let (mut store, mut level, mut entity) = fall_subtype_body("minecraft:breeze");
        provider_fall(
            &mut entity,
            &mut store,
            &mut level,
            distance,
            1.0,
            "minecraft:fall",
        );
        let sounds = queued_sounds(&mut store);
        assert_eq!(
            sounds
                .iter()
                .any(|s| s.registry_id == sound_id("minecraft:entity.breeze.land")),
            distance > 3.0
        );
        if let Some(sound) = sounds.first() {
            assert_eq!(
                (sound.registry_id, sound.source, sound.volume, sound.pitch),
                (
                    sound_id("minecraft:entity.breeze.land"),
                    SoundSource::Hostile,
                    1.0,
                    1.0
                )
            );
        }
    }
    let (mut store, mut level, mut entity) = fall_subtype_body("minecraft:breeze");
    standard_fall_attributes(&mut entity);
    level.install_tags(registry_tag_snapshot(&[]));
    let health = entity.living().unwrap().health();
    provider_fall(
        &mut entity,
        &mut store,
        &mut level,
        5.0,
        1.0,
        "minecraft:fall",
    );
    assert_eq!(entity.living().unwrap().health(), health - 2.0);
    let sounds = queued_sounds(&mut store);
    for name in [
        "minecraft:entity.breeze.land",
        "minecraft:entity.hostile.small_fall",
        "minecraft:block.metal.fall",
    ] {
        assert!(sounds.iter().any(
            |sound| sound.registry_id == sound_id(name) && sound.source == SoundSource::Hostile
        ));
    }
}

mod impulse;
mod passengers;
