use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

#[test]
fn living_entities_step_onto_beds_within_their_height_and_clearance() {
    for (step_height, ceiling, can_step) in
        [(0.6, false, true), (0.0, false, false), (0.6, true, false)]
    {
        let mut level = gold_plate();
        let bed =
            crate::fluid::with_property(state_id("minecraft:red_bed"), "facing", "south").unwrap();
        for (z, part) in [(0, "foot"), (1, "head")] {
            level.set(
                BlockPos { x: 1, y: 64, z },
                crate::fluid::with_property(bed, "part", part).unwrap(),
            );
        }
        if ceiling {
            for x in 0..=2 {
                level.set(BlockPos { x, y: 66, z: 0 }, state_id("minecraft:stone"));
            }
        }
        let (mut store, id) = walking_store(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        let mut entity = store.detach(id).unwrap();
        entity
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_STEP_HEIGHT_ID)
            .unwrap()
            .set_base_value(step_height);
        level
            .with_level(|ctx| {
                crate::entity::movement::move_living_entity(
                    &mut entity,
                    &mut store,
                    ctx,
                    crate::entity::MoverType::Self_,
                    Vec3d {
                        x: 0.8,
                        y: -0.01,
                        z: 0.0,
                    },
                )
            })
            .unwrap();
        if can_step {
            assert!((entity.position().x - 1.3).abs() < 1.0e-6);
            assert_eq!(entity.position().y, 64.5625);
        } else {
            assert!(entity.position().x < 1.0);
            assert_eq!(entity.position().y, 64.0);
        }
    }
}

fn landing(block: &str, distance: f64, kind: i32) -> (EntityStore, Box<Entity>, NativeLevel, i32) {
    let mut native = NativeLevel::new();
    let mut state = state_id(block);
    if block == "minecraft:pointed_dripstone" {
        state = crate::fluid::with_property(state, "vertical_direction", "up").unwrap();
        state = crate::fluid::with_property(state, "thickness", "tip").unwrap();
    }
    if block == "minecraft:red_bed" {
        state = crate::fluid::with_property(state, "facing", "south").unwrap();
        native.set(
            BlockPos { x: 0, y: 63, z: 1 },
            crate::fluid::with_property(state, "part", "head").unwrap(),
        );
    }
    if block == "minecraft:pointed_dripstone" || block == "minecraft:red_bed" {
        native.set(BlockPos { x: 0, y: 63, z: 0 }, state);
    } else {
        for x in 0..=3 {
            native.set(BlockPos { x, y: 63, z: 0 }, state);
        }
    }
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = if kind == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
        crate::entity::spawn_armor_stand(&mut store, FEET).unwrap()
    } else {
        store.spawn_general_living(kind, FEET).unwrap()
    };
    let mut entity = store.detach(id).unwrap();
    entity.fall_distance = distance;
    let top = native
        .with_level(|level| {
            level.block_collision_boxes_for_entity(&entity, BlockPos { x: 0, y: 63, z: 0 })
        })
        .unwrap()
        .iter()
        .map(|part| part.max_y)
        .reduce(f64::max)
        .expect("actual landing surface");
    entity.set_pos(Vec3d {
        y: 63.0 + top,
        ..FEET
    });
    entity.set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.01,
        z: 0.0,
    });
    store.drain_lifecycle_events();
    (store, entity, native, state)
}

fn land(entity: &mut Entity, store: &mut EntityStore, native: &mut NativeLevel) -> Result<(), ()> {
    let state = native
        .world
        .block_state(DIMENSION, BlockPos { x: 0, y: 63, z: 0 })
        .unwrap();
    let stationary = vanilla_block_state_identity(state).is_some_and(|state| {
        matches!(
            state.name.as_str(),
            "minecraft:pointed_dripstone" | "minecraft:red_bed"
        )
    });
    native.with_level(|level| {
        crate::entity::movement::move_living_entity(
            entity,
            store,
            level,
            crate::entity::MoverType::Self_,
            Vec3d {
                x: if stationary { 0.0 } else { 2.0 },
                y: -0.01,
                z: 0.0,
            },
        )
    })
}

fn sound(name: &str) -> i32 {
    bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(name).unwrap()
}

fn assert_block_landing(block: &str, expected_health: f32) {
    let (mut store, mut entity, mut level, state) =
        landing(block, 8.0, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    assert_eq!(land(&mut entity, &mut store, &mut level), Ok(()), "{block}");
    assert!(entity.on_ground(), "{block} must really collide");
    assert_eq!(
        entity.living().unwrap().health(),
        expected_health,
        "{block}"
    );
    assert_eq!(entity.fall_distance(), 0.0, "{block}");
    let hit = level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .find(|event| event.recorded.game_event == VanillaGameEvent::HIT_GROUND)
        .copied()
        .expect(block);
    assert_eq!(hit.recorded.source_entity_id, Some(entity.id()));
    assert_eq!(hit.recorded.affected_state, Some(state));

    if block == "minecraft:powder_snow" {
        assert!(queued_sounds(&mut store)
            .iter()
            .any(|entry| entry.registry_id == sound("minecraft:entity.hostile.big_fall")));
    }
}

macro_rules! block_landings {
    ($($name:ident: $block:literal => $health:literal),* $(,)?) => {
        $(#[test]
        fn $name() { assert_block_landing($block, $health); })*
    };
}

block_landings! {
    default_landing: "minecraft:gold_block" => 15.0,
    bed_landing: "minecraft:red_bed" => 19.0,
    hay_landing: "minecraft:hay_block" => 19.0,
    honey_landing: "minecraft:honey_block" => 19.0,
    slime_landing: "minecraft:slime_block" => 20.0,
    powder_snow_landing: "minecraft:powder_snow" => 20.0,
    pointed_dripstone_landing: "minecraft:pointed_dripstone" => 5.0,
}

#[test]
fn zero_damage_landing_still_emits_hit_ground_and_step() {
    let (mut store, mut entity, mut level, state) =
        landing("minecraft:gold_block", 2.0, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    assert_eq!(land(&mut entity, &mut store, &mut level), Ok(()));
    assert_eq!(entity.living().unwrap().health(), 20.0);
    assert_eq!(entity.fall_distance(), 0.0);
    let events = level.world.positioned_game_events_for_test(DIMENSION);
    for kind in [VanillaGameEvent::HIT_GROUND, VanillaGameEvent::STEP] {
        assert!(events.iter().any(|event| event.recorded.game_event == kind
            && event.recorded.source_entity_id == Some(entity.id())
            && event.recorded.affected_state == Some(state)));
    }
    assert!(!queued_sounds(&mut store)
        .iter()
        .any(|entry| entry.registry_id == sound("minecraft:entity.hostile.small_fall")));
}

#[test]
fn honey_positive_calculation_keeps_suffix_when_hurt_is_refused() {
    let (mut store, mut entity, mut level, _) =
        landing("minecraft:honey_block", 8.0, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    entity.set_invulnerable(true);
    assert_eq!(land(&mut entity, &mut store, &mut level), Ok(()));
    assert_eq!(entity.living().unwrap().health(), 20.0);
    let events = store.drain_lifecycle_events();
    assert!(events.iter().any(|event| matches!(event, crate::entity::EntityLifecycleEvent::Broadcast { entity_id, packet: crate::connection::ClientboundPlayPacket::EntityEvent(packet) } if *entity_id==entity.id() && packet.event_id==54)));
    let sounds: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            crate::entity::EntityLifecycleEvent::Sound { sound } => match sound.sound {
                SoundEventHolder::Reference { registry_id } => Some(registry_id),
                _ => None,
            },
            _ => None,
        })
        .collect();
    for name in [
        "minecraft:block.honey_block.slide",
        "minecraft:entity.hostile.small_fall",
    ] {
        assert!(sounds.contains(&sound(name)));
    }
    assert!(sounds.contains(&sound("minecraft:block.honey_block.fall")));
    assert!(!level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::ENTITY_DAMAGE));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::HIT_GROUND));
}

#[test]
fn silent_armor_stand_honey_landing_keeps_entity_event_and_ground_event() {
    let (mut store, mut entity, mut level, _) = landing(
        "minecraft:honey_block",
        8.0,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    );
    entity.set_silent(true);
    assert_eq!(land(&mut entity, &mut store, &mut level), Ok(()));
    assert!(entity.removal_reason().is_none());
    assert_eq!(entity.fall_distance(), 0.0);
    let events = store.drain_lifecycle_events();
    assert!(!events
        .iter()
        .any(|event| matches!(event, crate::entity::EntityLifecycleEvent::Sound { .. })));
    assert!(events.iter().any(|event| matches!(event,crate::entity::EntityLifecycleEvent::Broadcast {packet: crate::connection::ClientboundPlayPacket::EntityEvent(packet),..} if packet.event_id==54)));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::HIT_GROUND));
}

#[test]
fn disabled_griefing_skips_unowned_mutation_and_completes_base_fall() {
    for block in ["minecraft:farmland", "minecraft:turtle_egg"] {
        let (mut store, mut entity, mut level, _) =
            landing(block, 8.0, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::console_command(
            &sender,
            &receiver,
            &mut level.world,
            1,
            "gamerule minecraft:mob_griefing false",
        );
        assert_eq!(land(&mut entity, &mut store, &mut level), Ok(()), "{block}");
        assert_eq!(entity.living().unwrap().health(), 15.0);
        assert_eq!(entity.fall_distance(), 0.0);
        assert!(level
            .world
            .positioned_game_events_for_test(DIMENSION)
            .iter()
            .any(|event| event.recorded.game_event == VanillaGameEvent::HIT_GROUND));
    }
}

#[test]
fn honey_zero_calculation_omits_extra_fall_sound_but_keeps_slide_event() {
    let (mut store, mut entity, mut level, _) =
        landing("minecraft:honey_block", 2.0, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    assert_eq!(land(&mut entity, &mut store, &mut level), Ok(()));
    assert_eq!(entity.living().unwrap().health(), 20.0);
    let events = store.drain_lifecycle_events();
    assert!(events.iter().any(
        |event| matches!(event, crate::entity::EntityLifecycleEvent::Broadcast {
        packet: crate::connection::ClientboundPlayPacket::EntityEvent(packet), ..
    } if packet.event_id == 54)
    ));
    let sounds: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            crate::entity::EntityLifecycleEvent::Sound { sound } => match sound.sound {
                SoundEventHolder::Reference { registry_id } => Some(registry_id),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert!(sounds.contains(&sound("minecraft:block.honey_block.slide")));
    assert!(!sounds.contains(&sound("minecraft:block.honey_block.fall")));
    assert!(!sounds.contains(&sound("minecraft:entity.hostile.small_fall")));
}

#[test]
fn slime_fall_suppression_reads_shift_flag_not_crouching_pose() {
    let (mut store, mut entity, mut level, _) =
        landing("minecraft:slime_block", 8.0, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    level.install_tags(crate::registry_tags::RegistryTagSnapshot::default());
    entity.set_shared_flag(
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
        true,
    );
    assert_eq!(land(&mut entity, &mut store, &mut level), Ok(()));
    assert_eq!(entity.living().unwrap().health(), 20.0);
    assert_eq!(entity.fall_distance(), 0.0);
}
