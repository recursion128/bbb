use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID;

use crate::level_event::VanillaGameEvent;

// ---- armor stand ----------------------------------------------------------

fn native_stand() -> (NativeLevel, i32) {
    let mut level = NativeLevel::new();
    let (id, _) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    (level, id)
}

fn damage_type(resource_id: &str) -> &'static bbb_server_datapack::damage::DamageTypeFacts {
    bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is a built-in damage type"))
}

fn armor_stand_over_stone(y: f64) -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let level = NativeLevel::new();
    let id = crate::entity::spawn_armor_stand(
        &mut store,
        Vec3d {
            x: 0.5,
            y: y + 63.0,
            z: 0.5,
        },
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    (store, level, id)
}

#[test]
fn a_spawned_armor_stand_carries_the_constructor_state() {
    // `LivingEntity.java:272-282`, `ArmorStand.java:123-132`.
    let (store, _, id) = armor_stand_over_stone(5.0);
    let entity = store.get(id).unwrap();
    let living = entity.living().unwrap();
    assert_eq!(living.health(), 20.0);
    assert_eq!(entity.y_head_rot(), entity.y_rot());
    assert_eq!(
        entity.bounding_box().max_y - entity.bounding_box().min_y,
        f64::from(1.975_f32)
    );
    let non_default = crate::entity::tracking::non_default_synched_values(entity);
    // Only health (`20.0F` against the `1.0F` default, `LivingEntity.java:314`)
    // differs from the defined defaults on a fresh stand.
    assert_eq!(non_default.len(), 1);
    assert_eq!(
        non_default[0].data_id,
        bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_HEALTH_DATA_ID
    );
}

#[test]
fn a_placed_armor_stand_publishes_its_final_rotation() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let position = Vec3d {
        x: 2.5,
        y: 70.0,
        z: -3.5,
    };
    let id = crate::entity::armor_stand::spawn_placed(&mut store, position, 135.0)
        .expect("a fresh UUID");
    let events = store.drain_lifecycle_events();
    let [EntityLifecycleEvent::Added { entity, .. }] = events.as_slice() else {
        panic!("a placed stand publishes one add")
    };
    assert_eq!(entity.id(), id);
    assert_eq!(entity.position(), position);
    assert_eq!(entity.y_rot(), 135.0);
    assert_eq!(entity.x_rot(), 0.0);
    assert_eq!(entity.living().unwrap().y_body_rot, 0.0);
}

#[test]
fn an_armor_stand_falls_under_living_gravity_and_stops_on_the_ground() {
    let (mut store, mut level, id) = armor_stand_over_stone(5.0);
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let entity = store.get(id).unwrap();
    assert_eq!(entity.position().y, 68.0);
    assert_eq!(
        entity.delta_movement().y,
        (0.0 - 0.08) * f64::from(0.98_f32)
    );
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let entity = store.get(id).unwrap();
    assert_eq!(
        entity.position().y,
        68.0 + (0.0 - 0.08) * f64::from(0.98_f32)
    );
    for _ in 0..60 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    let entity = store.get(id).unwrap();
    assert!(entity.on_ground());
    assert_eq!(entity.position().y, 64.0);
    // `Entity.move` zeroes the blocked axis (`Entity.java:783-788`) and
    // `travelInAir` re-applies gravity after it, so a grounded living entity
    // keeps `-0.08 * 0.98F` as its stored vertical velocity.
    assert_eq!(
        entity.delta_movement().y,
        (0.0 - 0.08) * f64::from(0.98_f32)
    );
    assert_eq!(entity.fall_distance(), 0.0);
    assert!(entity.is_alive());
    // `calculateFallDamage` over four blocks is one point, so the fall sounds
    // play (`LivingEntity.java:1771-1772`); `hurtServer` refuses fall damage.
    // `ArmorStand` overrides only `getFallSounds` (`ArmorStand.java:587-589`),
    // so the block fall sound is the stone one fifth of a block below the
    // feet at `y = 1.0`.
    let sounds: Vec<_> = store
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Sound { sound } => Some(sound),
            _ => None,
        })
        .collect();
    assert_eq!(sounds.len(), 2);
    let sound_id = |resource_id: &str| {
        bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id).unwrap()
    };
    assert_eq!(
        (
            sounds[0].sound.clone(),
            sounds[0].volume,
            sounds[0].pitch,
            sounds[1].sound.clone(),
            sounds[1].volume,
            sounds[1].pitch,
            sounds[1].position,
        ),
        (
            bbb_protocol::packets::SoundEventHolder::Reference {
                registry_id: sound_id(crate::entity::armor_stand::ARMOR_STAND_FALL_SOUND),
            },
            1.0,
            1.0,
            bbb_protocol::packets::SoundEventHolder::Reference {
                registry_id: sound_id("minecraft:block.stone.fall"),
            },
            0.5,
            0.75,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
    );
    assert_eq!(store.get(id).unwrap().living().unwrap().health(), 20.0);
}

#[test]
fn a_marker_or_no_gravity_armor_stand_does_not_travel() {
    // `ArmorStand.hasPhysics` (`ArmorStand.java:113-115`).
    let (mut store, mut level, id) = armor_stand_over_stone(5.0);
    crate::entity::armor_stand::set_marker(store.get_mut(id).unwrap(), true);
    for _ in 0..5 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    assert_eq!(store.get(id).unwrap().position().y, 68.0);
    assert_eq!(store.get(id).unwrap().bounding_box().max_y, 68.0);
}

#[test]
fn a_player_hit_wobbles_first_and_breaks_within_five_ticks() {
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
    let player = level.world.publication().players[0].position;
    let position = Vec3d {
        x: player.x + 2.0,
        ..player
    };
    let expected_pos = BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    };
    let (id, _) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        position,
        EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    level
        .world
        .apply_level_entity_lifecycle_for_test(DIMENSION, &mut Vec::new());
    for _ in 0..6 {
        level.world.tick(Vec::new());
    }
    let uuid = level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .uuid();
    let first = crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        &format!("damage {uuid} 1 minecraft:player_attack by alice"),
    );
    assert!(level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .is_alive());
    assert!(first.events.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::EntityEvent(event), .. }
            | crate::world::WorldEvent::Broadcast { packet: crate::connection::ClientboundPlayPacket::EntityEvent(event), .. }
            if event.entity_id == id && event.event_id == 32
    )));
    let second = crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        2,
        &format!("damage {uuid} 1 minecraft:player_attack by alice"),
    );
    assert!(second.events.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::Sound(sound), .. }
            | crate::world::WorldEvent::Broadcast { packet: crate::connection::ClientboundPlayPacket::Sound(sound), .. }
            if sound.sound == bbb_protocol::packets::SoundEventHolder::Reference {
                registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id("minecraft:entity.armor_stand.break").unwrap()
            }
    )));
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(level
        .world
        .stored_items_for_test(DIMENSION)
        .iter()
        .any(|item| item.2 == "minecraft:armor_stand" && item.3 == 1));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                && event.recorded.source_entity_id == Some(id)
                && event.recorded.pos == expected_pos
        ));
}

#[test]
fn a_creative_hit_breaks_at_once_without_a_drop() {
    let (mut level, id) = native_stand();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "alice", 1);
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "gamemode creative alice",
    );
    let uuid = level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .uuid();
    let broken = crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        2,
        &format!("damage {uuid} 1 minecraft:player_attack by alice"),
    );
    assert!(broken.events.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::Sound(sound), .. }
            | crate::world::WorldEvent::Broadcast { packet: crate::connection::ClientboundPlayPacket::Sound(sound), .. }
            if sound.sound == bbb_protocol::packets::SoundEventHolder::Reference {
                registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id("minecraft:entity.armor_stand.break").unwrap()
            }
    )));
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                && event.recorded.source_entity_id == Some(id)
        ));
}

#[test]
fn a_fall_or_an_invisible_stand_refuses_damage_and_a_kill_bypasses_everything() {
    let (mut level, id) = native_stand();
    let refused = level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:fall", 3.0, None)
        .unwrap();
    assert!(!refused.applied && !refused.succeeded);
    crate::entity::armor_stand::set_invisible(
        level
            .world
            .stored_entity_mut_for_test(DIMENSION, id)
            .unwrap(),
        true,
    );
    let refused = level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:player_attack", 1.0, None)
        .unwrap();
    assert!(!refused.applied);
    let killed = level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:generic_kill", 1.0, None)
        .unwrap();
    assert!(killed.applied && !killed.succeeded);
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
}

#[test]
fn burning_takes_four_points_per_hit_and_breaks_at_half_a_heart() {
    let (mut level, id) = native_stand();
    for expected in [16.0, 12.0, 8.0, 4.0] {
        let outcome = level
            .world
            .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:on_fire", 1.0, None)
            .unwrap();
        assert!(outcome.applied && !outcome.succeeded);
        assert_eq!(
            level
                .world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .living()
                .unwrap()
                .health(),
            expected
        );
    }
    level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:on_fire", 1.0, None)
        .unwrap();
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                && event.recorded.source_entity_id == Some(id)
        ));
}

#[test]
fn armor_stand_flags_pose_and_equipment_survive_a_save() {
    let (mut store, _, id) = armor_stand_over_stone(1.0);
    {
        let entity = store.get_mut(id).unwrap();
        crate::entity::armor_stand::set_small(entity, true);
        crate::entity::armor_stand::set_show_arms(entity, true);
        crate::entity::armor_stand::set_no_base_plate(entity, true);
        crate::entity::armor_stand::set_disabled_slots(entity, 0x1f);
        crate::entity::armor_stand::set_head_pose(
            entity,
            crate::entity::Rotations::new(10.0, 20.0, 30.0),
        );
        entity.living_mut().unwrap().set_item_slot(
            bbb_protocol::equipment_slots::EquipmentSlot::Head,
            Some(stack_of("minecraft:iron_helmet", 1)),
        );
        entity.living_mut().unwrap().set_health(7.5);
    }
    let document = write_entity_chunk(&store, (0, 0)).unwrap();
    let record = document.entities().next().unwrap().raw();
    let names: Vec<&bbb_protocol::JavaString> = record.iter().map(|entry| &entry.name).collect();
    let position = |name: &str| names.iter().position(|n| *n == name).unwrap();
    assert!(position("Health") < position("attributes"));
    assert!(position("Brain") < position("equipment"));
    assert!(position("equipment") < position("Invisible"));
    assert!(position("NoBasePlate") < position("Pose"));
    assert!(!names.iter().any(|name| **name == "Marker"));
    let pose = record.iter().find(|entry| entry.name == "Pose").unwrap();
    assert_eq!(
        pose.value,
        NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
            name: "Head".into(),
            value: NbtSummaryValue::List(vec![
                NbtSummaryValue::Float(10.0_f32.to_bits()),
                NbtSummaryValue::Float(20.0_f32.to_bits()),
                NbtSummaryValue::Float(30.0_f32.to_bits()),
            ]),
        }])
    );

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).unwrap();
    let before = store.get(id).unwrap();
    let after = reloaded.get(ids[0]).unwrap();
    let (EntityData::ArmorStand(before_data), EntityData::ArmorStand(after_data)) =
        (before.data(), after.data())
    else {
        panic!("armor stands")
    };
    assert_eq!(after_data.client_flags(), before_data.client_flags());
    assert_eq!(after_data.disabled_slots(), before_data.disabled_slots());
    assert_eq!(after_data.pose(), before_data.pose());
    assert_eq!(
        after_data.living().equipment(),
        before_data.living().equipment()
    );
    // `Entity.load` (`Entity.java:2062-2063`) re-derives the head rotation
    // from the saved yaw, which the constructor also set it from.
    assert_eq!(after.y_rot(), before.y_rot());
    assert_eq!(after.y_head_rot(), before.y_head_rot());
    assert_eq!(
        after.bounding_box().max_y - after.bounding_box().min_y,
        f64::from(1.975_f32 * 0.5)
    );
    assert_eq!(after.living().unwrap().health(), 7.5);
}

#[test]
fn equipping_through_set_item_slot_plays_the_equip_sound_once_ticked() {
    let (mut store, mut level, id) = armor_stand_over_stone(1.0);
    let mut entity = store.detach(id).unwrap();
    crate::entity::armor_stand::set_item_slot(
        &mut entity,
        &mut store,
        bbb_protocol::equipment_slots::EquipmentSlot::Head,
        Some(stack_of("minecraft:iron_helmet", 1)),
    );
    store.attach(entity);
    assert!(store.drain_lifecycle_events().is_empty());
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    // `detectEquipmentUpdates` publishes the new slot.
    assert!(store.drain_lifecycle_events().iter().any(|event| matches!(
        event,
        EntityLifecycleEvent::Broadcast {
            packet: crate::connection::ClientboundPlayPacket::SetEquipment(update),
            ..
        } if update.slots.len() == 1
    )));
    let mut entity = store.detach(id).unwrap();
    crate::entity::armor_stand::set_item_slot(
        &mut entity,
        &mut store,
        bbb_protocol::equipment_slots::EquipmentSlot::Head,
        Some(stack_of("minecraft:golden_helmet", 1)),
    );
    store.attach(entity);
    let sounds: Vec<_> = store
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Sound { sound } => Some(sound),
            _ => None,
        })
        .collect();
    assert_eq!(sounds.len(), 1);
    assert_eq!(
        sounds[0].sound,
        bbb_protocol::packets::SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:item.armor.equip_gold"
            )
            .unwrap()
        }
    );
}

fn stack_with_component(item: &str, component: &str, value: NbtSummaryValue) -> RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string(item),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                    name: component.into(),
                    value,
                }]),
            },
        ]),
        &crate::entity::test_items::BuiltinItems,
    )
    .expect("the test stack is writer-canonical")
}

#[test]
fn armor_stand_break_uses_the_store_enchantment_effect_set() {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use bbb_protocol::equipment_slots::EquipmentSlot;

    let (mut store, _, id) = armor_stand_over_stone(1.0);
    store.set_prevent_equipment_drop_enchantments(Arc::new(BTreeSet::from([
        "minecraft:sharpness".to_string(),
    ])));
    let sharp = stack_with_component(
        "minecraft:iron_sword",
        "minecraft:enchantments",
        NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
            name: "sharpness".into(),
            value: NbtSummaryValue::Int(1),
        }]),
    );
    {
        let living = store.get_mut(id).unwrap().living_mut().unwrap();
        living.set_item_slot(EquipmentSlot::MainHand, Some(sharp));
        living.set_item_slot(
            EquipmentSlot::Feet,
            Some(stack_of("minecraft:iron_boots", 1)),
        );
    }
    let mut entity = store.detach(id).unwrap();
    let effects = crate::entity::armor_stand::hurt_server(
        &mut entity,
        &mut store,
        crate::entity::StoredEntityDamageSource::without_attacker_views(
            damage_type("minecraft:explosion"),
            None,
            None,
        ),
        1.0,
        Default::default(),
        Default::default(),
    )
    .unwrap();
    assert!(effects.outcome.applied && !effects.outcome.succeeded);
    assert_eq!(entity.removal_reason(), Some(RemovalReason::Killed));
    let dropped = store
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Added { entity, .. } => match entity.data() {
                EntityData::Item(item) => item.item().item_resource_id().map(str::to_string),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(dropped, vec!["minecraft:iron_boots".to_string()]);
}

#[test]
fn swept_wither_rose_runs_after_armor_stand_travel_and_effect_tick() {
    let (mut store, mut level, id) = armor_stand_over_stone(1.0);
    store.set_difficulty(bbb_protocol::difficulties::Difficulty::Normal);
    let entity = store.get_mut(id).unwrap();
    entity.set_delta_movement(Vec3d {
        x: 2.0,
        y: 0.0,
        z: 0.0,
    });
    entity.take_synched_dirty();
    store.drain_lifecycle_events();
    level.set(
        BlockPos { x: 1, y: 64, z: 0 },
        state_id("minecraft:wither_rose"),
    );

    level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));

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
    // `ArmorStand.updateInvisibilityStatus` (`ArmorStand.java:445-447`) writes
    // no particle row.
    assert!(!entity.take_synched_dirty().contains(
        &bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID
    ));
}

#[test]
fn armor_stand_inside_damage_uses_its_damage_tag_decision() {
    let (mut store, mut level, id) = armor_stand_over_stone(1.0);
    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:damage_type",
        "minecraft:always_kills_armor_stands",
        &["minecraft:cactus"],
    )]));
    level.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:cactus"));

    level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));

    assert!(store.get(id).is_none());
}

#[test]
fn armor_stand_freezing_suffix_observes_the_post_travel_contact() {
    let (mut store, mut level, id) = armor_stand_over_stone(1.0);
    store.get_mut(id).unwrap().set_ticks_frozen(4);
    level.set(
        BlockPos { x: 0, y: 64, z: 0 },
        state_id("minecraft:powder_snow"),
    );

    level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));

    let entity = store.get(id).unwrap();
    assert!(entity.is_in_powder_snow());
    assert_eq!(entity.ticks_frozen(), 5);
}

// ---- summoned -------------------------------------------------------------

/// `EntityType.create` runs the constructor and then loads the `{id}` tag
/// `/summon` built (`EntityType.java:1450-1454`), whose absent `Rotation`
/// zeroes the constructor's draw (`Entity.java:2052`, `:2059-2063`).
#[test]
fn a_summoned_armor_stand_takes_the_loaded_zero_rotation() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let position = Vec3d {
        x: 11.5,
        y: 64.0,
        z: 11.5,
    };
    let crate::entity::summon::SummonedEntity::Published(id) = store
        .spawn_summoned(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            position,
        )
        .expect("a fresh UUID")
    else {
        panic!("an armor stand is not a Mob, so createEntity runs no finalize")
    };
    let events = store.drain_lifecycle_events();
    let [EntityLifecycleEvent::Added { entity, .. }] = events.as_slice() else {
        panic!("a summoned stand publishes one add")
    };
    assert_eq!(entity.id(), id);
    assert_eq!(entity.position(), position);
    assert_eq!(entity.y_rot(), 0.0);
    assert_eq!(entity.x_rot(), 0.0);
    assert_eq!(entity.y_head_rot(), 0.0);
    assert_eq!(entity.living().unwrap().y_body_rot, 0.0);
    assert_eq!(entity.living().unwrap().health(), 20.0);
}

/// A summoned stand is the stand a save restores: it writes the same record,
/// and reading that record back rebuilds the same runtime.
#[test]
fn a_summoned_armor_stand_round_trips_through_a_save() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let position = Vec3d {
        x: 11.5,
        y: 64.0,
        z: 11.5,
    };
    let crate::entity::summon::SummonedEntity::Published(id) = store
        .spawn_summoned(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            position,
        )
        .expect("a fresh UUID")
    else {
        panic!("an armor stand is not a Mob, so createEntity runs no finalize")
    };
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    let record = document.entities().next().unwrap().raw();
    let field = |name: &str| {
        record
            .iter()
            .find(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("a summoned stand writes {name}"))
            .value
            .clone()
    };
    assert_eq!(
        field("id"),
        NbtSummaryValue::string("minecraft:armor_stand")
    );
    assert_eq!(
        field("Rotation"),
        NbtSummaryValue::List(vec![
            NbtSummaryValue::Float(0.0_f32.to_bits()),
            NbtSummaryValue::Float(0.0_f32.to_bits()),
        ])
    );
    // `ArmorStand.addAdditionalSaveData` (`ArmorStand.java:140-152`) omits only
    // `Marker` while it is false.
    for name in ["Invisible", "Small", "ShowArms", "NoBasePlate"] {
        assert_eq!(field(name), NbtSummaryValue::Byte(0), "{name}");
    }
    assert_eq!(field("DisabledSlots"), NbtSummaryValue::Int(0));
    assert_eq!(field("Pose"), NbtSummaryValue::Compound(Vec::new()));
    assert!(!record.iter().any(|entry| entry.name == "Marker"));

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the record restores");
    let before = store.get(id).expect("the summoned stand");
    let after = reloaded.get(ids[0]).expect("the restored stand");
    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.position(), before.position());
    assert_eq!(after.y_rot(), before.y_rot());
    assert_eq!(after.y_head_rot(), before.y_head_rot());
    let restored = write_entity_chunk(&reloaded, (0, 0)).expect("the save form is well formed");
    assert_eq!(restored.entities().next().unwrap().raw(), record);
}

#[test]
fn every_armor_stand_kill_arm_raises_entity_die_from_the_stand() {
    for damage in [
        "minecraft:generic_kill",
        "minecraft:explosion",
        "minecraft:on_fire",
    ] {
        let (mut level, id) = native_stand();
        if damage == "minecraft:on_fire" {
            crate::entity::living::set_health(
                level
                    .world
                    .stored_entity_mut_for_test(DIMENSION, id)
                    .unwrap(),
                4.0,
            );
        }
        level
            .world
            .hurt_stored_entity_by_command_for_test(DIMENSION, id, damage, 1.0, None)
            .unwrap();
        assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
        assert!(level
            .world
            .positioned_game_events_for_test(DIMENSION)
            .iter()
            .any(
                |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                    && event.recorded.source_entity_id == Some(id)
                    && event.recorded.pos == pos(0, 64, 0)
            ));
    }
}
