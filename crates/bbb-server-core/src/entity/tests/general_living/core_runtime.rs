use super::*;
use crate::entity::tests::{queued, queued_sounds};
use crate::entity::LastHurtByMob;

mod cold_sleep;
mod death_loot;
mod external_die;
mod sound_sources;

fn hostile_queued(
    resource_id: &str,
    position: Vec3d,
    volume: f32,
    pitch: f32,
) -> crate::entity::tests::QueuedSound {
    let mut sound = queued(resource_id, position, volume, pitch);
    sound.source = SoundSource::Hostile;
    sound
}

#[test]
fn baby_happy_ghast_breathes_underwater_outside_the_entity_type_tag() {
    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    entity.set_air_supply(-7);
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    *data.ageable_retained_mut().unwrap() = AgeableMobRetainedState::from_saved(-20, 0, true);
    let mut level = NativeLevel::new();
    for y in 64..=69 {
        level.set(BlockPos { x: 0, y, z: 0 }, state_id("minecraft:water"));
    }
    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:fluid",
        "minecraft:water",
        &["minecraft:water"],
    )]));

    level.with_level(|level| tick_entities(&mut store, level, true));

    assert_eq!(store.get(id).unwrap().air_supply(), -3);
}

#[test]
fn fire_resistance_blocks_damage_tagged_is_fire() {
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    store
        .get_mut(id)
        .unwrap()
        .living_mut()
        .unwrap()
        .add_active_effect(
            crate::entity::ActiveMobEffect::new("minecraft:fire_resistance", 200, 0).unwrap(),
        );
    let facts = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:generic")
        .unwrap();
    let source = StoredEntityDamageSource {
        damage_type: facts,
        direct_entity_id: None,
        causing_entity: None,
        attacker_views: Default::default(),
    };
    let health = store.get(id).unwrap().living().unwrap().health();
    let mut level = NativeLevel::new();

    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:damage_type",
        "minecraft:is_fire",
        &["minecraft:generic"],
    )]));
    let mut entity = store.detach(id).unwrap();
    let fire = level.with_level(|level| {
        crate::entity::general_living::hurt_server_with_runtime_in_level(
            &mut entity,
            &mut store,
            level,
            source,
            3.0,
            StoredEntityDamageRules::default(),
        )
    });
    assert!(!fire.outcome.applied);
    assert_eq!(entity.living().unwrap().health(), health);
    store.attach(entity);

    level.install_tags(registry_tag_snapshot(&[]));
    let mut entity = store.detach(id).unwrap();
    let ordinary = level.with_level(|level| {
        crate::entity::general_living::hurt_server_with_runtime_in_level(
            &mut entity,
            &mut store,
            level,
            source,
            3.0,
            StoredEntityDamageRules::default(),
        )
    });
    assert!(ordinary.outcome.applied);
    assert!(entity.living().unwrap().health() < health);
    store.attach(entity);
}

#[test]
fn submerged_iron_golem_keeps_its_air() {
    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
            position(),
        )
        .unwrap();
    let mut level = NativeLevel::new();
    for y in 64..=69 {
        level.set(BlockPos { x: 0, y, z: 0 }, state_id("minecraft:water"));
    }
    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:fluid",
        "minecraft:water",
        &["minecraft:water"],
    )]));
    for _ in 0..5 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert_eq!(store.get(id).unwrap().air_supply(), 300);
}

#[test]
fn airborne_chickens_fall_slowly() {
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CHICKEN_ID, position())
        .unwrap();
    store.get_mut(id).unwrap().on_ground = false;
    let mut level = NativeLevel::new();
    for _ in 0..10 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    let fall = store.get(id).unwrap().delta_movement().y;
    assert!((-0.15..0.0).contains(&fall), "{fall}");
}

#[test]
fn base_tick_applies_on_fire_damage_and_advances_owned_living_state() {
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    store.drain_lifecycle_events();
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_air_supply(296);
        entity.set_remaining_fire_ticks(20);
        let living = entity.living_mut().unwrap();
        living.hurt_time = 2;
        living.set_item_slot(
            EquipmentSlot::Feet,
            crate::entity::item::plain_stack("minecraft:iron_boots", 1),
        );
    }
    let health = store.get(id).unwrap().living().unwrap().health();
    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut store, level, true));
    let entity = store.get(id).unwrap();
    assert_eq!(entity.air_supply(), 300);
    assert_eq!(entity.remaining_fire_ticks(), 19);
    assert!(entity.living().unwrap().health() < health);
    assert!(entity.hurt_marked());
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
    assert!(store.drain_lifecycle_events().iter().any(|event| matches!(
        event,
        EntityLifecycleEvent::Broadcast {
            entity_id,
            packet: crate::connection::ClientboundPlayPacket::SetEquipment(packet),
        } if *entity_id == id && packet.entity_id == id
    )));
}

#[test]
fn water_animal_air_runtime_dry_out_damage() {
    for entity_type_id in [
        VANILLA_ENTITY_TYPE_COD_ID,
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
    ] {
        let mut store = EntityStore::new(0xA960, EntityIdCounter::new());
        let id = store
            .spawn_general_living(entity_type_id, position())
            .unwrap();
        store.drain_lifecycle_events();
        let entity = store.get_mut(id).unwrap();
        entity.set_air_supply(-19);
        if entity_type_id == VANILLA_ENTITY_TYPE_PUFFERFISH_ID {
            let EntityData::GeneralLiving(data) = &mut entity.data else {
                unreachable!("fish uses general living")
            };
            data.set_no_ai(true);
        }
        let health = store.get(id).unwrap().living().unwrap().health();
        let mut level = NativeLevel::new();

        level.with_level(|level| tick_entities(&mut store, level, true));

        let entity = store.get(id).unwrap();
        assert_eq!(entity.air_supply(), 0);
        assert_eq!(entity.living().unwrap().health(), health - 2.0);
        assert!(!entity.was_touching_water());
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
}

/// `LivingEntity.decreaseAirSupply` (`LivingEntity.java:565-575`) keeps the
/// air with chance `1 - 1 / (OXYGEN_BONUS + 1)`.
#[test]
fn oxygen_bonus_slows_a_submerged_mobs_air_loss() {
    let run = |oxygen_bonus: f64| {
        let mut native = NativeLevel::new();
        native.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:water"));
        native.set(BlockPos { x: 0, y: 65, z: 0 }, state_id("minecraft:water"));
        let mut store = EntityStore::new(0xA1_2F, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                VANILLA_ENTITY_TYPE_COW_ID,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
            )
            .unwrap();
        let entity = store.get_mut(id).unwrap();
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            data.set_no_ai(true);
        }
        entity
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_OXYGEN_BONUS_ID)
            .unwrap()
            .set_base_value(oxygen_bonus);
        for _ in 0..20 {
            store.get_mut(id).unwrap().set_pos(Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            });
            native.with_level(|level| {
                crate::entity::tick_one_entity(&mut store, level, id);
            });
        }
        store.get(id).unwrap().air_supply()
    };
    assert_eq!(run(0.0), 280);
    assert!(run(1.0e9) >= 299);
}

#[test]
fn axolotls_and_nautiluses_dry_out_on_land() {
    for entity_type_id in [
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ] {
        let mut store = EntityStore::new(0xA962, EntityIdCounter::new());
        let id = store
            .spawn_general_living(entity_type_id, position())
            .unwrap();
        store.drain_lifecycle_events();
        store.get_mut(id).unwrap().set_air_supply(-18);
        let health = store.get(id).unwrap().living().unwrap().health();
        let mut level = NativeLevel::new();

        level.with_level(|level| tick_entities(&mut store, level, true));
        let entity = store.get(id).unwrap();
        assert_eq!(entity.air_supply(), -19);
        assert_eq!(entity.living().unwrap().health(), health);

        level.with_level(|level| tick_entities(&mut store, level, true));
        let entity = store.get(id).unwrap();
        assert_eq!(entity.air_supply(), 0);
        assert_eq!(entity.living().unwrap().health(), health - 2.0);
    }
}

#[test]
fn water_animal_air_runtime_water_and_dead_restore_air() {
    let mut wet = EntityStore::new(0xA961, EntityIdCounter::new());
    let wet_id = wet
        .spawn_general_living(VANILLA_ENTITY_TYPE_COD_ID, position())
        .unwrap();
    wet.get_mut(wet_id).unwrap().set_air_supply(-19);
    let wet_health = wet.get(wet_id).unwrap().living().unwrap().health();
    let mut wet_level = {
        let mut native = NativeLevel::new();
        native.set(
            bbb_protocol::packets::BlockPos { x: 0, y: 64, z: 0 },
            state_id("minecraft:water"),
        );
        native
    };
    wet_level.with_level(|level| tick_entities(&mut wet, level, true));
    let entity = wet.get(wet_id).unwrap();
    assert_eq!(entity.air_supply(), 300);
    assert_eq!(entity.living().unwrap().health(), wet_health);
    assert!(entity.was_touching_water());
    assert!(wet_level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .is_empty());

    let mut dead = EntityStore::new(0xA962, EntityIdCounter::new());
    let dead_id = dead
        .spawn_general_living(VANILLA_ENTITY_TYPE_SALMON_ID, position())
        .unwrap();
    dead.get_mut(dead_id).unwrap().set_air_supply(17);
    dead.get_mut(dead_id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(0.0);
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut dead, level, true))
    };
    assert_eq!(dead.get(dead_id).unwrap().air_supply(), 300);
}

#[test]
fn abstract_fish_runtime_dry_flop_jumps_and_plays_its_flop_sound() {
    let mut store = EntityStore::new(0xA980, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COD_ID, position())
        .unwrap();
    store.drain_lifecycle_events();
    let entity = store.get_mut(id).unwrap();
    entity.on_ground = true;
    entity.vertical_collision = true;

    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut store, level, true))
    };

    let entity = store.get(id).unwrap();
    assert!(entity.position().y > position().y + 0.39);
    assert!(!entity.on_ground());
    let sound = store
        .drain_lifecycle_events()
        .into_iter()
        .find_map(|event| match event {
            EntityLifecycleEvent::Sound { sound } => Some(sound),
            _ => None,
        })
        .expect("dry AbstractFish queues its concrete flop sound");
    assert_eq!((sound.source, sound.volume), (SoundSource::Neutral, 1.0));
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.cod.flop",
            )
            .unwrap(),
        }
    );
}

#[test]
fn abstract_fish_runtime_water_travel() {
    let mut wet = EntityStore::new(0xA987, EntityIdCounter::new());
    let wet_id = wet
        .spawn_general_living(VANILLA_ENTITY_TYPE_SALMON_ID, position())
        .unwrap();
    wet.drain_lifecycle_events();
    let old_delta = Vec3d {
        x: 0.1,
        y: 0.02,
        z: -0.2,
    };
    let entity = wet.get_mut(wet_id).unwrap();
    entity.set_rot(0.0, 0.0);
    entity.set_delta_movement(old_delta);
    let living = entity.living_mut().unwrap();
    living.xxa = 0.5;
    let moved = Vec3d {
        x: old_delta.x + f64::from(0.5_f32 * 0.98_f32) * f64::from(0.01_f32),
        y: old_delta.y + 0.005,
        ..old_delta
    };
    let mut wet_level = {
        let mut native = NativeLevel::new();
        native.set(
            bbb_protocol::packets::BlockPos { x: 0, y: 64, z: 0 },
            state_id("minecraft:water"),
        );
        native
    };

    wet_level.with_level(|level| tick_entities(&mut wet, level, true));

    let entity = wet.get(wet_id).unwrap();
    assert!((entity.position().x - (position().x + moved.x)).abs() < 1.0E-7);
    assert_eq!(entity.position().y, position().y + moved.y);
    assert_eq!(entity.position().z, position().z + moved.z);
    assert!((entity.delta_movement().x - moved.x * 0.9).abs() < 1.0E-7);
    assert!((entity.delta_movement().y - (moved.y * 0.9 - 0.005)).abs() < 1.0E-7);
    assert_eq!(entity.delta_movement().z, moved.z * 0.9);
    assert!(entity.was_touching_water());
}

#[test]
fn ageable_water_runtime_restore_save() {
    let mut store = EntityStore::new(0xA964, EntityIdCounter::new());
    let dolphin_id = read_entity_chunk(
        &mut store,
        &chunk(ageable_water_record(
            "minecraft:dolphin",
            Uuid::from_u128(0xA964),
            true,
            37,
            0,
        )),
    )
    .unwrap()[0];
    let glow_id = read_entity_chunk(
        &mut store,
        &chunk(ageable_water_record(
            "minecraft:glow_squid",
            Uuid::from_u128(0xA965),
            false,
            0,
            19,
        )),
    )
    .unwrap()[0];
    let dolphin = store.get(dolphin_id).unwrap();
    let EntityData::GeneralLiving(dolphin_data) = dolphin.data() else {
        unreachable!("Dolphin uses GeneralLiving")
    };
    assert_eq!(
        dolphin_data
            .ageable_water_retained()
            .and_then(AgeableWaterCreatureRetainedState::dolphin_fields),
        Some((true, 37))
    );
    assert!(dolphin_data
        .opaque_fields()
        .iter()
        .all(|field| { !matches!(field.name.to_jdk_string().as_str(), "GotFish" | "Moistness") }));
    let EntityData::GeneralLiving(glow_data) = store.get(glow_id).unwrap().data() else {
        unreachable!("GlowSquid uses GeneralLiving")
    };
    assert_eq!(
        glow_data
            .ageable_water_retained()
            .and_then(AgeableWaterCreatureRetainedState::glow_squid_dark_ticks),
        Some(19)
    );

    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    for (resource_id, expected) in [
        (
            "minecraft:dolphin",
            vec![
                ("GotFish", NbtSummaryValue::Byte(1)),
                ("Moistness", NbtSummaryValue::Int(37)),
            ],
        ),
        (
            "minecraft:glow_squid",
            vec![("DarkTicksRemaining", NbtSummaryValue::Int(19))],
        ),
    ] {
        let raw = saved
            .entities()
            .find(|record| record.id() == resource_id)
            .unwrap()
            .raw();
        for (name, value) in expected {
            assert_eq!(
                raw.iter()
                    .filter(|field| field.name == name)
                    .map(|field| &field.value)
                    .collect::<Vec<_>>(),
                vec![&value]
            );
        }
    }

    let mut sparse =
        ageable_water_record("minecraft:dolphin", Uuid::from_u128(0xA966), false, 2400, 0);
    sparse.retain(|field| !matches!(field.name.to_jdk_string().as_str(), "GotFish" | "Moistness"));
    let mut sparse_store = EntityStore::new(0xA966, EntityIdCounter::new());
    let sparse_id = read_entity_chunk(&mut sparse_store, &chunk(sparse)).unwrap()[0];
    let EntityData::GeneralLiving(data) = sparse_store.get(sparse_id).unwrap().data() else {
        unreachable!("Dolphin uses GeneralLiving")
    };
    assert_eq!(
        data.ageable_water_retained()
            .and_then(AgeableWaterCreatureRetainedState::dolphin_fields),
        Some((false, 2400))
    );
}

#[test]
fn ageable_water_runtime_tick_and_damage() {
    let mut squid = EntityStore::new(0xA980, EntityIdCounter::new());
    let squid_id = squid
        .spawn_general_living(VANILLA_ENTITY_TYPE_SQUID_ID, position())
        .unwrap();
    squid.get_mut(squid_id).unwrap().set_air_supply(-19);
    let health = squid.get(squid_id).unwrap().living().unwrap().health();
    let mut dry_level = NativeLevel::new();
    dry_level.with_level(|level| tick_entities(&mut squid, level, true));
    assert_eq!(squid.get(squid_id).unwrap().air_supply(), 0);
    assert_eq!(
        squid.get(squid_id).unwrap().living().unwrap().health(),
        health - 2.0
    );

    let mut glow = EntityStore::new(0xA981, EntityIdCounter::new());
    let glow_id = glow
        .spawn_general_living(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, position())
        .unwrap();
    let EntityData::GeneralLiving(data) = &mut glow.get_mut(glow_id).unwrap().data else {
        unreachable!("GlowSquid uses GeneralLiving")
    };
    *data.ageable_water_retained_mut().unwrap() =
        AgeableWaterCreatureRetainedState::from_saved_glow_squid(2);
    let mut wet_level = {
        let mut native = NativeLevel::new();
        native.set(
            bbb_protocol::packets::BlockPos { x: 0, y: 64, z: 0 },
            state_id("minecraft:water"),
        );
        native
    };
    wet_level.with_level(|level| tick_entities(&mut glow, level, true));
    let EntityData::GeneralLiving(data) = glow.get(glow_id).unwrap().data() else {
        unreachable!("GlowSquid uses GeneralLiving")
    };
    assert_eq!(glow.get(glow_id).unwrap().air_supply(), 300);
    assert_eq!(
        data.ageable_water_retained()
            .and_then(AgeableWaterCreatureRetainedState::glow_squid_dark_ticks),
        Some(1)
    );

    let mut detached = glow.detach(glow_id).unwrap();
    let generic = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:generic")
        .unwrap();
    let effects = hurt_server_with_runtime(
        &mut detached,
        &mut glow,
        StoredEntityDamageSource {
            damage_type: generic,
            direct_entity_id: None,
            causing_entity: None,
            attacker_views: Default::default(),
        },
        1.0,
        StoredEntityDamageRules::default(),
    );
    assert!(effects.outcome.succeeded);
    let EntityData::GeneralLiving(data) = detached.data() else {
        unreachable!("GlowSquid uses GeneralLiving")
    };
    assert_eq!(
        data.ageable_water_retained()
            .and_then(AgeableWaterCreatureRetainedState::glow_squid_dark_ticks),
        Some(100)
    );
    assert!(detached
        .take_synched_dirty()
        .contains(&VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID));
    glow.attach(detached);

    let mut dolphin = EntityStore::new(0xA982, EntityIdCounter::new());
    let dolphin_id = dolphin
        .spawn_general_living(VANILLA_ENTITY_TYPE_DOLPHIN_ID, position())
        .unwrap();
    let entity = dolphin.get_mut(dolphin_id).unwrap();
    entity.tick_count = 1;
    entity.on_ground = true;
    entity.set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.08,
        z: 0.0,
    });
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!("Dolphin uses GeneralLiving")
    };
    *data.ageable_water_retained_mut().unwrap() =
        AgeableWaterCreatureRetainedState::from_saved_dolphin(false, 1);
    let dolphin_health = entity.living().unwrap().health();
    let mut ground_level = NativeLevel::new();
    ground_level.with_level(|level| tick_entities(&mut dolphin, level, true));
    let entity = dolphin.get(dolphin_id).unwrap();
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!("Dolphin uses GeneralLiving")
    };
    assert_eq!(
        data.ageable_water_retained()
            .and_then(AgeableWaterCreatureRetainedState::dolphin_fields),
        Some((false, 0))
    );
    assert_eq!(entity.living().unwrap().health(), dolphin_health - 1.0);
    assert!(!entity.on_ground());
}

#[test]
fn ageable_water_runtime_dolphin_no_ai_retains_moistness_and_restores_air() {
    let mut store = EntityStore::new(0xA984, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_DOLPHIN_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    entity.set_air_supply(12);
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!("Dolphin uses GeneralLiving")
    };
    data.set_no_ai(true);
    *data.ageable_water_retained_mut().unwrap() =
        AgeableWaterCreatureRetainedState::from_saved_dolphin(false, 3);
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut store, level, true))
    };
    let entity = store.get(id).unwrap();
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!("Dolphin uses GeneralLiving")
    };
    assert_eq!(entity.air_supply(), 4800);
    assert_eq!(
        data.ageable_water_retained()
            .and_then(AgeableWaterCreatureRetainedState::dolphin_fields),
        Some((false, 3))
    );
}

#[test]
fn base_entity_observable_state_restore_save_and_tracking_are_exact() {
    let uuid = Uuid::from_u128(0xA970);
    let mut entries = zombie_record(uuid);
    entries.extend([
        entry("Silent", NbtSummaryValue::Byte(1)),
        entry("Glowing", NbtSummaryValue::Byte(1)),
        entry("TicksFrozen", NbtSummaryValue::Int(37)),
        entry("HasVisualFire", NbtSummaryValue::Byte(1)),
    ]);
    let mut store = EntityStore::new(0xA970, EntityIdCounter::new());
    let id = read_entity_chunk(&mut store, &chunk(entries)).unwrap()[0];
    let entity = store.get(id).unwrap();
    assert!(entity.is_silent());
    assert!(entity.has_glowing_tag());
    assert_eq!(entity.ticks_frozen(), 37);
    assert!(entity.has_visual_fire());
    assert!(entity.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_GLOWING));

    let rows = non_default_synched_values(entity);
    assert!(rows.iter().any(|row| {
        row.data_id == VANILLA_ENTITY_SILENT_DATA_ID
            && row.value == bbb_protocol::packets::EntityDataValueKind::Boolean(true)
    }));
    assert!(rows.iter().any(|row| {
        row.data_id == VANILLA_ENTITY_TICKS_FROZEN_DATA_ID
            && row.value == bbb_protocol::packets::EntityDataValueKind::Int(37)
    }));
    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    let raw = saved.entities().next().unwrap().raw();
    for name in ["Silent", "Glowing", "TicksFrozen", "HasVisualFire"] {
        assert_eq!(raw.iter().filter(|field| field.name == name).count(), 1);
    }

    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut store, level, true))
    };
    assert!(store
        .get(id)
        .unwrap()
        .get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE));

    store.get_mut(id).unwrap().set_ticks_frozen(-5);
    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    assert!(!saved
        .entities()
        .next()
        .unwrap()
        .raw()
        .iter()
        .any(|field| field.name == "TicksFrozen"));
}

#[test]
fn canonical_hurt_dispatch_keeps_cooldown_credit_and_death_state() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 0x55);
    native.world.set_player_mode_and_abilities_for_test(
        crate::connection::ConnectionId(1),
        bbb_protocol::game_types::GameType::Survival,
        bbb_protocol::packets::PlayerAbilities::for_game_type(
            bbb_protocol::game_types::GameType::Survival,
        ),
    );
    let player = Uuid::from_u128(0x55);
    let player_id = native
        .world
        .player_entity_id_for_test(crate::connection::ConnectionId(1));
    let ids = EntityIdCounter::new();
    while ids.next_id() < player_id {}
    let mut store = EntityStore::new(5, ids);
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let generic = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:generic")
        .unwrap();
    let source = StoredEntityDamageSource {
        damage_type: generic,
        direct_entity_id: Some(player_id),
        causing_entity: Some(StoredDamageCausingEntity::Player {
            uuid: player,
            entity_id: player_id,
            instabuild: false,
        }),
        attacker_views: Default::default(),
    };
    let before = store.get(id).unwrap().living().unwrap().health();
    let mut events = LevelEventSink::default();
    let outcome = store
        .hurt_server(
            id,
            source,
            3.0,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut events,
        )
        .unwrap()
        .unwrap();
    assert!(outcome.applied && outcome.succeeded);
    let entity = store.get_mut(id).unwrap();
    assert!(entity.living().unwrap().health() < before);
    assert_eq!(entity.living().unwrap().last_hurt, 3.0);
    assert_eq!(entity.living().unwrap().last_hurt_by_player, Some(player));
    assert_eq!(
        entity.living().unwrap().last_hurt_by_player_memory_time,
        100
    );
    assert_eq!(
        entity.living().unwrap().last_hurt_by_mob(),
        Some(LastHurtByMob::Uuid(player))
    );
    assert!(entity.take_hurt_marked());

    let before_refusal = store.get(id).unwrap().living().unwrap().health();
    let outcome = store
        .hurt_server(
            id,
            source,
            3.0,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut events,
        )
        .unwrap()
        .unwrap();
    assert!(!outcome.applied && !outcome.succeeded);
    assert_eq!(
        store.get(id).unwrap().living().unwrap().health(),
        before_refusal
    );

    let health = store.get(id).unwrap().living().unwrap().health();
    let outcome = store
        .hurt_server(
            id,
            source,
            5.0,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut events,
        )
        .unwrap()
        .unwrap();
    assert!(outcome.applied && outcome.succeeded);
    assert!(store.get(id).unwrap().living().unwrap().health() < health);
    assert!(!store.get(id).unwrap().hurt_marked());

    let generic_kill = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:generic_kill")
        .unwrap();
    let outcome = store
        .hurt_server(
            id,
            StoredEntityDamageSource {
                damage_type: generic_kill,
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            f32::INFINITY,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut events,
        )
        .unwrap()
        .unwrap();
    assert!(outcome.applied && outcome.succeeded);
    assert!(store.get(id).unwrap().living().unwrap().dead);
    assert!(store.get(id).unwrap().living().unwrap().is_dead_or_dying());

    let mut level = NativeLevel::new();
    for _ in 0..19 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert!(store.get(id).is_some());
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(store.get(id).is_none());
}

#[test]
fn the_damage_path_files_the_uuid_of_a_stored_attacker() {
    let (store, victim, attacker) = victim_hurt_by_stored_zombie("minecraft:generic");
    let attacker_uuid = store.get(attacker).unwrap().uuid();
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        Some(LastHurtByMob::Stored {
            entity_id: attacker,
            uuid: Some(attacker_uuid),
        })
    );
}

#[test]
fn a_reference_to_a_removed_attacker_still_saves_its_uuid() {
    let (mut store, victim, attacker) = victim_hurt_by_stored_zombie("minecraft:generic");
    let attacker_uuid = store.get(attacker).unwrap().uuid();
    let victim_uuid = store.get(victim).unwrap().uuid();
    store.get_mut(victim).unwrap().tick_count = 9;
    store
        .remove(attacker, crate::entity::RemovalReason::Killed)
        .expect("the attacker is stored");

    let saved = write_entity_chunk(&store, (0, 0)).expect("the survivor saves");
    let raw = record_of(&saved, victim_uuid);
    assert_eq!(
        raw.iter()
            .find(|entry| entry.name == "last_hurt_by_mob")
            .map(|entry| &entry.value),
        Some(&uuid_words(attacker_uuid)),
        "the store dropping the entity does not drop the UUID the reference holds"
    );
    assert_eq!(
        raw.iter()
            .find(|entry| entry.name == "ticks_since_last_hurt_by_mob")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::Int(9)),
        "the saved age survives after removing the attacker"
    );
}

#[test]
fn a_reloaded_uuid_reference_resolves_by_uuid_and_clears_on_its_death() {
    let (store, victim, attacker) = victim_hurt_by_stored_zombie("minecraft:generic");
    let attacker_uuid = store.get(attacker).unwrap().uuid();
    let victim_uuid = store.get(victim).unwrap().uuid();
    let saved = write_entity_chunk(&store, (0, 0)).expect("the pair saves");

    let mut reloaded = EntityStore::new(0x4E57, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &saved).expect("the records reload");
    let reloaded_victim = id_of_uuid(&reloaded, &ids, victim_uuid);
    let reloaded_attacker = id_of_uuid(&reloaded, &ids, attacker_uuid);
    set_no_ai(&mut reloaded, reloaded_victim);
    set_no_ai(&mut reloaded, reloaded_attacker);
    assert_eq!(
        reloaded
            .get(reloaded_victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        Some(LastHurtByMob::Uuid(attacker_uuid)),
        "the saved reference carries the attacker UUID"
    );

    let mut level = floor_level();
    level.with_level(|level| tick_entities(&mut reloaded, level, true));
    assert_eq!(
        reloaded
            .get(reloaded_victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        Some(LastHurtByMob::Uuid(attacker_uuid)),
        "a living target inside the window keeps the memory"
    );

    reloaded
        .get_mut(reloaded_attacker)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(0.0);
    level.with_level(|level| tick_entities(&mut reloaded, level, true));
    assert_eq!(
        reloaded
            .get(reloaded_victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        None,
        "the target's death is what clears it, long before the window"
    );
}

#[test]
fn a_dead_session_player_attacker_is_forgotten() {
    let mut store = EntityStore::new(0x5E55, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    let victim = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    set_no_ai(&mut store, victim);
    let player = Uuid::from_u128(0x5E55);
    store
        .get_mut(victim)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_last_hurt_by_mob(Some(LastHurtByMob::Uuid(player)), 0);

    let mut level = floor_level();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "player",
        player.as_u128(),
    );
    level
        .world
        .set_player_position_for_test(crate::connection::ConnectionId(1), position());
    level.world.set_player_mode_and_abilities_for_test(
        crate::connection::ConnectionId(1),
        bbb_protocol::game_types::GameType::Survival,
        bbb_protocol::packets::PlayerAbilities::for_game_type(
            bbb_protocol::game_types::GameType::Survival,
        ),
    );
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "damage player 40 minecraft:generic",
    );
    assert_eq!(
        level
            .world
            .player_health_for_test(crate::connection::ConnectionId(1)),
        0.0
    );
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        None,
        "a resolved but dead session clears the memory"
    );
}

fn id_of_uuid(store: &EntityStore, ids: &[i32], uuid: Uuid) -> i32 {
    *ids.iter()
        .find(|id| store.get(**id).unwrap().uuid() == uuid)
        .expect("the record reloads")
}

#[test]
fn a_dead_stored_attacker_is_forgotten_before_the_window() {
    let (mut store, victim, attacker) = victim_hurt_by_stored_zombie("minecraft:generic");
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        Some(stored_reference(&store, attacker))
    );
    store
        .get_mut(attacker)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(0.0);
    {
        let mut native = NativeLevel::new();
        native.with_level(|level| tick_entities(&mut store, level, true))
    };
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        None
    );
}

#[test]
fn the_last_hurt_by_mob_window_closes_after_a_hundred_ticks() {
    let (mut store, victim, _attacker) = victim_hurt_by_stored_zombie("minecraft:generic");
    let mut level = floor_level();
    for _ in 0..100 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert!(store
        .get(victim)
        .unwrap()
        .living()
        .unwrap()
        .last_hurt_by_mob()
        .is_some());
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        None
    );
}

#[test]
fn a_session_player_attacker_survives_the_store_lookup_and_the_window_clears_it() {
    let mut store = EntityStore::new(13, {
        let ids = EntityIdCounter::new();
        ids.next_id();
        ids
    });
    let victim = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    set_no_ai(&mut store, victim);
    let player = Uuid::from_u128(0x7E5);
    let generic = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:generic")
        .unwrap();
    let mut events = LevelEventSink::default();
    store
        .hurt_server(
            victim,
            StoredEntityDamageSource {
                damage_type: generic,
                direct_entity_id: Some(4),
                causing_entity: Some(StoredDamageCausingEntity::Player {
                    uuid: player,
                    entity_id: 4,
                    instabuild: false,
                }),
                attacker_views: Default::default(),
            },
            1.0,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut events,
        )
        .unwrap()
        .unwrap();
    let mut level = floor_level();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "player",
        player.as_u128(),
    );
    level
        .world
        .set_player_position_for_test(crate::connection::ConnectionId(1), position());
    for _ in 0..100 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        Some(LastHurtByMob::Uuid(player))
    );
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        None
    );
}

#[test]
fn the_anger_tags_keep_a_source_out_of_the_reference() {
    let (store, victim, _attacker) = victim_hurt_by_stored_zombie("minecraft:mob_attack_no_aggro");
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        None
    );

    let (store, victim, _attacker) = victim_hurt_by_stored_zombie("minecraft:wind_charge");
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        None,
        "a zombie is in #minecraft:no_anger_from_wind_charge"
    );

    let (store, victim, attacker) =
        victim_hurt_by_stored_zombie_of("minecraft:wind_charge", VANILLA_ENTITY_TYPE_COW_ID);
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        Some(stored_reference(&store, attacker)),
        "a cow is not in #minecraft:no_anger_from_wind_charge"
    );
}

fn set_no_ai(store: &mut EntityStore, id: i32) {
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data else {
        unreachable!("the fixture is a general living entity")
    };
    data.set_no_ai(true);
}

fn floor_level() -> NativeLevel {
    NativeLevel::new()
}

fn victim_hurt_by_stored_zombie(damage_type: &str) -> (EntityStore, i32, i32) {
    victim_hurt_by_stored_zombie_of(damage_type, VANILLA_ENTITY_TYPE_ZOMBIE_ID)
}

fn victim_hurt_by_stored_zombie_of(
    damage_type: &str,
    victim_type_id: i32,
) -> (EntityStore, i32, i32) {
    let mut store = EntityStore::new(12, EntityIdCounter::new());
    let attacker = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let victim = store
        .spawn_general_living(victim_type_id, position())
        .unwrap();
    set_no_ai(&mut store, attacker);
    set_no_ai(&mut store, victim);
    let facts = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == damage_type)
        .unwrap();
    let mut events = LevelEventSink::default();
    store
        .hurt_server(
            victim,
            StoredEntityDamageSource {
                damage_type: facts,
                direct_entity_id: Some(attacker),
                causing_entity: Some(StoredDamageCausingEntity::Mob {
                    entity_id: attacker,
                }),
                attacker_views: Default::default(),
            },
            1.0,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut events,
        )
        .unwrap()
        .unwrap();
    (store, victim, attacker)
}

#[test]
fn freezing_multiplier_and_mob_helmet_quarter() {
    let mut store = EntityStore::new(6, EntityIdCounter::new());
    let blaze = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_BLAZE_ID, position())
        .unwrap();
    let freeze = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:freeze")
        .unwrap();
    let health = store.get(blaze).unwrap().living().unwrap().health();
    let mut events = LevelEventSink::default();
    let outcome = store
        .hurt_server(
            blaze,
            StoredEntityDamageSource {
                damage_type: freeze,
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
    assert!(outcome.applied && outcome.succeeded);
    assert_eq!(
        store.get(blaze).unwrap().living().unwrap().health(),
        health - 5.0
    );

    let zombie = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    store
        .get_mut(zombie)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(
            EquipmentSlot::Head,
            crate::entity::item::plain_stack("minecraft:iron_helmet", 1),
        );
    let falling_anvil = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:falling_anvil")
        .unwrap();
    let before = store.get(zombie).unwrap().clone();
    let outcome = store
        .hurt_server(
            zombie,
            StoredEntityDamageSource {
                damage_type: falling_anvil,
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            8.0,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut events,
        )
        .unwrap()
        .unwrap();
    assert!(outcome.applied && outcome.succeeded);
    let after = store.get(zombie).unwrap();
    let taken = before.living().unwrap().health() - after.living().unwrap().health();
    assert!((taken - 6.0 * (1.0 - 0.4 / 25.0)).abs() < 1e-5, "{taken}");
    assert_eq!(
        after.living().unwrap().item_by_slot(EquipmentSlot::Head),
        before.living().unwrap().item_by_slot(EquipmentSlot::Head)
    );
}

#[test]
fn ai_step_applies_dead_band_input_and_immobile_state_before_air_travel() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.002,
        y: 0.002,
        z: 0.004,
    });
    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut store, level, true));
    let entity = store.get(id).unwrap();
    assert_eq!(entity.position().x, position().x);
    assert_eq!(entity.position().y, position().y);
    assert_eq!(entity.position().z, position().z + 0.004);
    assert_eq!(entity.delta_movement().x, 0.0);
    assert_eq!(
        entity.delta_movement().y,
        (0.0 - 0.08) * f64::from(0.98_f32)
    );
    assert_eq!(entity.delta_movement().z, 0.004 * f64::from(0.91_f32));

    {
        let living = store.get_mut(id).unwrap().living_mut().unwrap();
        living.xxa = 1.0;
        living.yya = 0.25;
        living.zza = -0.5;
    }
    level.with_level(|level| tick_entities(&mut store, level, true));
    let living = store.get(id).unwrap().living().unwrap();
    assert_eq!(living.xxa, 0.98);
    assert_eq!(living.yya, 0.25);
    assert_eq!(living.zza, 0.0);

    {
        let living = store.get_mut(id).unwrap().living_mut().unwrap();
        living.set_health(0.0);
        living.jumping = true;
        living.xxa = 1.0;
        living.zza = 1.0;
    }
    level.with_level(|level| tick_entities(&mut store, level, true));
    let living = store.get(id).unwrap().living().unwrap();
    assert!(!living.jumping);
    assert_eq!(living.xxa, 0.0);
    assert_eq!(living.zza, 0.0);
}

#[test]
fn general_living_ground_jump_reaches_travel_with_honey_and_sprint_factors() {
    fn jump_on(floor: &str, sprinting: bool, boost: bool) -> (Vec3d, Vec3d, Vec3d, bool) {
        let mut store = EntityStore::new(71, EntityIdCounter::new());
        let data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
        let id = store
            .restore(
                Uuid::from_u128(71),
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
                EntityData::GeneralLiving(data),
            )
            .unwrap();
        let mut level = {
            let mut native = NativeLevel::new();
            for x in 0..16 {
                for z in 0..16 {
                    native.world.set_block(
                        DIMENSION,
                        BlockPos { x, y: 63, z },
                        state_id(floor),
                        crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
                    );
                }
            }
            native
        };
        for _ in 0..3 {
            level.with_level(|level| tick_entities(&mut store, level, true));
            if store.get(id).unwrap().on_ground() {
                break;
            }
        }
        assert!(store.get(id).unwrap().on_ground());
        let before = store.get(id).unwrap().position();
        let entity = store.get_mut(id).unwrap();
        entity.living_mut().unwrap().jumping = true;
        if boost {
            assert!(entity.living_mut().unwrap().add_active_effect(
                crate::entity::ActiveMobEffect::new("minecraft:jump_boost", 100, 1).unwrap()
            ));
        }
        entity.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING, sprinting);

        level.with_level(|level| {
            crate::entity::living::tick_jump(store.get_mut(id).unwrap(), level)
        });
        level.with_level(|level| tick_entities(&mut store, level, true));
        let entity = store.get(id).unwrap();
        (
            before,
            entity.position(),
            entity.delta_movement(),
            entity.needs_sync(),
        )
    }

    let (stone_before, stone_position, stone_movement, stone_sync) =
        jump_on("minecraft:stone", true, false);
    assert!((stone_position.y - stone_before.y - 0.42).abs() < 1.0e-6);
    assert!((stone_position.z - 0.7).abs() < 1.0e-6);
    assert!(stone_movement.y > 0.3);
    assert!(stone_sync);

    let (honey_before, honey_position, honey_movement, honey_sync) =
        jump_on("minecraft:honey_block", false, false);
    assert!((honey_position.y - honey_before.y - 0.21).abs() < 1.0e-6);
    assert_eq!(honey_position.z, 0.5);
    assert!(honey_movement.y > 0.1 && honey_movement.y < stone_movement.y);
    assert!(honey_sync);
    let (boost_before, boost_position, boost_movement, boost_sync) =
        jump_on("minecraft:honey_block", false, true);
    assert!((boost_position.y - boost_before.y - f64::from(0.21_f32 + 0.2_f32)).abs() < 1.0e-6);
    assert!(boost_movement.y > honey_movement.y);
    assert!(boost_sync);
}

#[test]
fn in_air_travel_lands_and_routes_fall_damage_through_general_hurt() {
    let mut store = EntityStore::new(8, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 0.5,
                y: 68.0,
                z: 0.5,
            },
        )
        .unwrap();
    store.drain_lifecycle_events();
    let mut level = {
        let mut native = NativeLevel::new();
        for x in 0..16 {
            for z in 0..16 {
                native.world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 63, z },
                    state_id("minecraft:stone"),
                    crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
                );
            }
        }
        native
    };
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(&sender, &receiver, &mut level.world, 1, "time set midnight");
    let health = store.get(id).unwrap().living().unwrap().health();
    for _ in 0..70 {
        level.with_level(|level| tick_entities(&mut store, level, true));
        if store.get(id).unwrap().on_ground() {
            break;
        }
    }
    let entity = store.get(id).unwrap();
    assert!(entity.on_ground());
    assert_eq!(entity.position().y, 64.0);
    assert_eq!(entity.fall_distance(), 0.0);
    assert_eq!(entity.living().unwrap().health(), health - 1.0);
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DAMAGE
                && event.recorded.source_entity_id == Some(id)
        ));
    let landing = entity.position();
    let sounds = queued_sounds(&mut store);
    assert!(sounds.contains(&hostile_queued(
        "minecraft:entity.hostile.small_fall",
        landing,
        1.0,
        1.0
    )));
    assert!(sounds.contains(&hostile_queued(
        "minecraft:block.stone.fall",
        landing,
        0.5,
        0.75
    )));
}

#[test]
fn fall_damage_reads_the_block_a_fifth_of_a_block_below_the_feet() {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let mut level = {
        let mut native = NativeLevel::new();
        for x in 0..16 {
            for z in 0..16 {
                native.world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 63, z },
                    state_id("minecraft:stone"),
                    crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
                );
            }
        }
        for x in 0..16 {
            for z in 0..16 {
                native.world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 64, z },
                    state_id("minecraft:gravel"),
                    crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
                );
            }
        }
        native
    };
    for (feet_y, expected) in [
        (64.2, "minecraft:block.stone.fall"),
        (64.5, "minecraft:block.gravel.fall"),
    ] {
        let id = store
            .spawn_general_living(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                Vec3d {
                    x: 0.5,
                    y: feet_y,
                    z: 0.5,
                },
            )
            .unwrap();
        let entity = store.detach(id).unwrap();
        store.drain_lifecycle_events();

        level.with_level(|level| {
            crate::entity::living::fall_damage_sounds(
                &entity,
                &mut store,
                level,
                crate::entity::living::GENERIC_SMALL_FALL_SOUND,
            )
        });

        let sounds = queued_sounds(&mut store);
        assert!(sounds.contains(&hostile_queued(
            "minecraft:entity.generic.small_fall",
            entity.position(),
            1.0,
            1.0,
        )));
        assert!(sounds.contains(&hostile_queued(expected, entity.position(), 0.5, 0.75)));
    }
}

#[test]
fn fall_damage_over_air_plays_the_damage_sound_only() {
    let mut store = EntityStore::new(12, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let entity = store.detach(id).unwrap();
    store.drain_lifecycle_events();
    {
        let mut native = NativeLevel::new();
        native.set(BlockPos { x: 0, y: 63, z: 0 }, state_id("minecraft:air"));
        native.with_level(|level| {
            crate::entity::living::fall_damage_sounds(
                &entity,
                &mut store,
                level,
                crate::entity::living::GENERIC_BIG_FALL_SOUND,
            )
        })
    };
    assert_eq!(
        queued_sounds(&mut store),
        vec![hostile_queued(
            "minecraft:entity.generic.big_fall",
            entity.position(),
            1.0,
            1.0
        )]
    );
}

#[test]
fn a_silent_entity_plays_neither_fall_sound() {
    let mut store = EntityStore::new(13, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    store.get_mut(id).unwrap().set_silent(true);
    let entity = store.detach(id).unwrap();
    let mut level = NativeLevel::new();
    store.drain_lifecycle_events();

    level.with_level(|level| {
        crate::entity::living::fall_damage_sounds(
            &entity,
            &mut store,
            level,
            crate::entity::living::GENERIC_SMALL_FALL_SOUND,
        )
    });

    assert!(store.drain_lifecycle_events().is_empty());
}

#[test]
fn validated_zombie_restore_save_and_reload_retains_opaque_fields() {
    let uuid = Uuid::from_u128(0x1234);
    let input = chunk(zombie_record(uuid));
    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut store, &input).expect("general zombie restores");
    let entity = store.get(ids[0]).unwrap();
    assert_eq!(entity.uuid(), uuid);
    assert_eq!(entity.living().unwrap().health(), 17.0);

    let output = write_entity_chunk(&store, (0, 0)).expect("restored zombie saves");
    let raw = output.entities().next().unwrap().raw();
    for name in [
        "Health",
        "attributes",
        "Brain",
        "CanPickUpLoot",
        "IsBaby",
        "FutureZombieAlpha",
        "FutureZombieOmega",
    ] {
        assert_eq!(
            raw.iter().filter(|entry| entry.name == name).count(),
            1,
            "{name}"
        );
    }
    assert_eq!(
        raw.iter()
            .find(|entry| entry.name == "HurtByTimestamp")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::Int(0)),
        "the reader restamped `lastHurtByMobTimestamp` from the absent \
         `ticks_since_last_hurt_by_mob` , so the \
         rewrite carries the restamped value"
    );

    let mut reloaded = EntityStore::new(4, EntityIdCounter::new());
    let reloaded_ids = read_entity_chunk(&mut reloaded, &output).expect("BBB output reloads");
    assert_eq!(
        reloaded.get(reloaded_ids[0]).unwrap().data(),
        store.get(ids[0]).unwrap().data()
    );
}

#[test]
fn a_saved_last_hurt_by_mob_reference_and_its_window_round_trip() {
    let mut store = EntityStore::new(0x5A7E, EntityIdCounter::new());
    let killer = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let victim = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let killer_uuid = store.get(killer).unwrap().uuid();
    let victim_uuid = store.get(victim).unwrap().uuid();
    let player = Uuid::from_u128(0x91A7E);
    let reference = stored_reference(&store, killer);
    let victim_entity = store.get_mut(victim).unwrap();
    victim_entity.tick_count = 7;
    victim_entity
        .living_mut()
        .unwrap()
        .set_last_hurt_by_mob(Some(reference), 2);

    let stored_reference = write_entity_chunk(&store, (0, 0)).expect("the live mobs save");
    let raw = record_of(&stored_reference, victim_uuid);
    assert_eq!(
        raw.iter()
            .find(|entry| entry.name == "last_hurt_by_mob")
            .map(|entry| &entry.value),
        Some(&uuid_words(killer_uuid)),
        "a stored reference writes the entity's own UUID"
    );
    assert_eq!(
        raw.iter()
            .find(|entry| entry.name == "ticks_since_last_hurt_by_mob")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::Int(5)),
        "the saved age survives after removing the attacker"
    );

    store
        .get_mut(victim)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_last_hurt_by_mob(Some(LastHurtByMob::Uuid(player)), 7);
    let player_reference = write_entity_chunk(&store, (0, 0)).expect("the live mobs save");
    let raw = record_of(&player_reference, victim_uuid);
    assert_eq!(
        raw.iter()
            .find(|entry| entry.name == "last_hurt_by_mob")
            .map(|entry| &entry.value),
        Some(&uuid_words(player))
    );
    assert_eq!(
        raw.iter()
            .find(|entry| entry.name == "ticks_since_last_hurt_by_mob")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::Int(0))
    );

    let mut reloaded = EntityStore::new(0x5A7E, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &stored_reference).expect("the record reloads");
    let restored_victim = ids
        .into_iter()
        .find(|id| reloaded.get(*id).unwrap().uuid() == victim_uuid)
        .expect("the victim reloads");
    let living = reloaded.get(restored_victim).unwrap().living().unwrap();
    assert_eq!(
        living.last_hurt_by_mob(),
        Some(LastHurtByMob::Uuid(killer_uuid))
    );
    assert_eq!(living.last_hurt_by_mob_timestamp, 5);
    resolve_saved_last_hurt_by_mob_references(&mut reloaded);
    let restored_killer = reloaded
        .entity_id_for_uuid(killer_uuid)
        .expect("the killer reloads into the same chunk");
    let living = reloaded.get(restored_victim).unwrap().living().unwrap();
    assert_eq!(
        living.last_hurt_by_mob(),
        Some(LastHurtByMob::Stored {
            entity_id: restored_killer,
            uuid: Some(killer_uuid),
        }),
        "`EntityReference.getEntity` resolves the UUID through the level's lookup"
    );
    assert_eq!(
        living.last_hurt_by_mob_timestamp, 5,
        "resolving the reference restamps nothing"
    );

    let mut reloaded = EntityStore::new(0x5A7E, EntityIdCounter::new());
    read_entity_chunk(&mut reloaded, &player_reference).expect("the record reloads");
    resolve_saved_last_hurt_by_mob_references(&mut reloaded);
    let restored_victim = reloaded
        .ids()
        .into_iter()
        .find(|id| reloaded.get(*id).unwrap().uuid() == victim_uuid)
        .expect("the victim reloads");
    assert_eq!(
        reloaded
            .get(restored_victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        Some(LastHurtByMob::Uuid(player))
    );
}

#[test]
fn a_fresh_living_record_omits_the_last_hurt_by_mob_pair() {
    let mut store = EntityStore::new(0xF2E5, EntityIdCounter::new());
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let saved = write_entity_chunk(&store, (0, 0)).expect("the fresh mob saves");
    let raw = saved.entities().next().unwrap().raw();
    assert!(raw.iter().all(|entry| entry.name != "last_hurt_by_mob"));
    assert!(raw
        .iter()
        .all(|entry| entry.name != "ticks_since_last_hurt_by_mob"));
    assert_eq!(
        raw.iter()
            .find(|entry| entry.name == "HurtByTimestamp")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::Int(0)),
        "`HurtByTimestamp`  is written whether or not a reference is held"
    );
}

#[test]
fn a_reloaded_last_hurt_by_mob_reference_no_lookup_places_is_kept_past_the_window() {
    let victim_uuid = Uuid::from_u128(0x5EED);
    let player = Uuid::from_u128(0x9A7E);
    let mut entries = zombie_record(victim_uuid);
    entries.push(entry("last_hurt_by_mob", uuid_words(player)));
    entries.push(entry(
        "ticks_since_last_hurt_by_mob",
        NbtSummaryValue::Int(0),
    ));
    let document = chunk(entries);
    let mut store = EntityStore::new(0x5EED, EntityIdCounter::new());
    let victim = read_entity_chunk(&mut store, &document).expect("the record reloads")[0];
    set_no_ai(&mut store, victim);
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        Some(LastHurtByMob::Uuid(player))
    );
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob_timestamp,
        0
    );
    let mut level = floor_level();
    for _ in 0..100 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        Some(LastHurtByMob::Uuid(player)),
        "the hundredth tick is still inside the reloaded window"
    );
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(
        store
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .last_hurt_by_mob(),
        Some(LastHurtByMob::Uuid(player)),
        "a reference nothing resolves keeps the memory its own save still carries"
    );
    let saved = write_entity_chunk(&store, (0, 0)).expect("the held mob saves");
    let raw = record_of(&saved, victim_uuid);
    assert!(
        raw.iter().any(|entry| entry.name == "last_hurt_by_mob"),
        "the held UUID is still written "
    );
}

fn stored_reference(store: &EntityStore, entity_id: i32) -> LastHurtByMob {
    LastHurtByMob::Stored {
        entity_id,
        uuid: Some(
            store
                .get(entity_id)
                .expect("the fixture entity is stored")
                .uuid(),
        ),
    }
}

fn record_of(saved: &Java26EntityChunkData, uuid: Uuid) -> &[NbtSummaryEntry] {
    saved
        .entities()
        .find(|record| record.uuid() == uuid)
        .map(|record| record.raw())
        .expect("the record names the entity")
}

fn uuid_words(uuid: Uuid) -> NbtSummaryValue {
    let bits = uuid.as_u128();
    NbtSummaryValue::IntArray(vec![
        (bits >> 96) as u32 as i32,
        (bits >> 64) as u32 as i32,
        (bits >> 32) as u32 as i32,
        bits as u32 as i32,
    ])
}

#[test]
fn swept_wither_rose_runs_after_general_living_travel_and_effect_tick() {
    let mut store = EntityStore::new(0x0057_17E2, EntityIdCounter::new());
    store.set_difficulty(Difficulty::Normal);
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    entity.set_delta_movement(Vec3d {
        x: 2.0,
        y: 0.0,
        z: 0.0,
    });
    entity.take_synched_dirty();
    store.drain_lifecycle_events();
    let mut level = {
        let mut native = NativeLevel::new();
        native.set(
            BlockPos { x: 1, y: 64, z: 0 },
            state_id("minecraft:wither_rose"),
        );
        native
    };

    level.with_level(|level| tick_entities(&mut store, level, true));

    let entity = store.get_mut(id).unwrap();
    assert!(entity.position().x > 1.5);
    assert_eq!(
        entity
            .living()
            .unwrap()
            .active_effect("minecraft:wither")
            .unwrap()
            .duration(),
        40
    );
    assert!(entity.take_synched_dirty().contains(
        &bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID,
    ));
}

#[test]
fn general_living_inside_damage_uses_the_environment_hurt_owner() {
    let mut store = EntityStore::new(0xCA_C7_05, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    let health = store.get(id).unwrap().living().unwrap().health();
    let mut level = {
        let mut native = NativeLevel::new();
        native.set(BlockPos { x: 0, y: 63, z: 0 }, state_id("minecraft:sand"));
        native.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:cactus"));
        native
    };

    level.with_level(|level| tick_entities(&mut store, level, true));

    assert_eq!(
        store.get(id).unwrap().living().unwrap().health(),
        health - 1.0
    );
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
fn swept_water_extinguish_emits_the_living_sound() {
    let mut store = EntityStore::new(0x00E5_7109, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    store.get_mut(id).unwrap().set_remaining_fire_ticks(20);
    store.drain_lifecycle_events();
    let mut level = {
        let mut native = NativeLevel::new();
        native.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:water"));
        native
    };

    level.with_level(|level| tick_entities(&mut store, level, true));

    assert_eq!(store.get(id).unwrap().remaining_fire_ticks(), 0);
    let extinguish = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.generic.extinguish_fire",
    )
    .unwrap();
    assert!(store.drain_lifecycle_events().iter().any(|event| {
        matches!(event, EntityLifecycleEvent::Sound { sound }
        if sound.sound == bbb_protocol::packets::SoundEventHolder::Reference {
            registry_id: extinguish,
        })
    }));
}

#[test]
fn swept_lava_damage_emits_burn_sound_only_after_success() {
    let mut store = EntityStore::new(0x1A_AA, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    store.drain_lifecycle_events();
    let health = store.get(id).unwrap().living().unwrap().health();
    let mut level = {
        let mut native = NativeLevel::new();
        native.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:lava"));
        native
    };

    level.with_level(|level| tick_entities(&mut store, level, true));

    assert_eq!(
        store.get(id).unwrap().living().unwrap().health(),
        health - 4.0
    );
    let burn = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.generic.burn",
    )
    .unwrap();
    assert!(store.drain_lifecycle_events().iter().any(|event| {
        matches!(event, EntityLifecycleEvent::Sound { sound }
        if sound.sound == bbb_protocol::packets::SoundEventHolder::Reference {
            registry_id: burn,
        })
    }));

    let mut store = EntityStore::new(0x1A_AB, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    store.get_mut(id).unwrap().set_invulnerable(true);
    store.drain_lifecycle_events();
    let health = store.get(id).unwrap().living().unwrap().health();
    let mut level = {
        let mut native = NativeLevel::new();
        native.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:lava"));
        native
    };

    level.with_level(|level| tick_entities(&mut store, level, true));

    assert_eq!(store.get(id).unwrap().living().unwrap().health(), health);
    assert!(!store.drain_lifecycle_events().iter().any(|event| {
        matches!(event, EntityLifecycleEvent::Sound { sound }
        if sound.sound == bbb_protocol::packets::SoundEventHolder::Reference {
            registry_id: burn,
        })
    }));
}

#[test]
fn general_living_freezing_suffix_observes_the_post_travel_contact() {
    let mut store = EntityStore::new(0xF2_EE_23, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .unwrap();
    store.get_mut(id).unwrap().set_ticks_frozen(4);
    let mut level = {
        let mut native = NativeLevel::new();
        native.set(
            BlockPos { x: 0, y: 64, z: 0 },
            state_id("minecraft:powder_snow"),
        );
        native
    };

    level.with_level(|level| tick_entities(&mut store, level, true));

    let entity = store.get(id).unwrap();
    assert!(entity.is_in_powder_snow());
    assert_eq!(entity.ticks_frozen(), 5);
}

#[test]
fn a_monster_in_the_light_idles_two_ticks_faster_than_an_animal() {
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let creeper = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CREEPER_ID, position())
        .unwrap();
    let cow = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 8.5,
                ..position()
            },
        )
        .unwrap();
    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut store, level, true));
    let idle = |id| store.get(id).unwrap().living().unwrap().no_action_time;
    assert_eq!(idle(creeper) - idle(cow), 2);
}
