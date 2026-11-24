//! The feeding entry of the breeding chain on the real packet path: the love
//! timer and the growth branch `Animal.mobInteract` answers a player's held
//! item with (`Animal.java:139-163`).

use super::*;

mod remainders;

/// One integer field of a stored entity's own saved record, which is the shape
/// `Animal.addAdditionalSaveData` writes (`Animal.java:97-101`).
pub(super) fn animal_record_int(world: &ServerWorld, entity_id: i32, name: &str) -> i32 {
    let bbb_protocol::NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("the entity has a record")
    else {
        panic!("an entity record is a compound")
    };
    match entries.iter().find(|entry| entry.name == name) {
        Some(bbb_protocol::NbtSummaryEntry {
            value: bbb_protocol::NbtSummaryValue::Int(value),
            ..
        }) => *value,
        other => panic!("{name} is an int field: {other:?}"),
    }
}

/// Rewrites one integer field of a stored entity's saved record, which is the
/// load half of the same pair.
fn set_animal_record_int(world: &mut ServerWorld, entity_id: i32, name: &str, value: i32) {
    let bbb_protocol::NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("the entity has a record")
    else {
        panic!("an entity record is a compound")
    };
    entries
        .iter_mut()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("an animal record carries {name}"))
        .value = bbb_protocol::NbtSummaryValue::Int(value);
    world.load_stored_entity_record_for_test(
        Java26BuiltinDimension::Overworld,
        entity_id,
        &entries,
    );
}

/// The `LoveCause` field of a stored animal's saved record as a UUID.
fn animal_record_love_cause(world: &ServerWorld, entity_id: i32) -> Option<Uuid> {
    let bbb_protocol::NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("the entity has a record")
    else {
        panic!("an entity record is a compound")
    };
    match entries.iter().find(|entry| entry.name == "LoveCause") {
        Some(bbb_protocol::NbtSummaryEntry {
            value: bbb_protocol::NbtSummaryValue::IntArray(parts),
            ..
        }) => {
            let parts: Vec<i32> = parts.clone();
            assert_eq!(parts.len(), 4, "a UUID is four ints");
            Some(Uuid::from_u128(
                (u128::from(parts[0] as u32) << 96)
                    | (u128::from(parts[1] as u32) << 64)
                    | (u128::from(parts[2] as u32) << 32)
                    | u128::from(parts[3] as u32),
            ))
        }
        None => None,
        other => panic!("LoveCause is a UUID array: {other:?}"),
    }
}

pub(super) fn interact_entity_packet(entity_id: i32) -> PlayServerbound {
    PlayServerbound::Interact(InteractEntity {
        entity_id,
        hand: InteractionHand::MainHand,
        location: Vec3d::default(),
        using_secondary_action: false,
    })
}

pub(super) fn stack_of(resource_id: &str, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)
                .unwrap_or_else(|| panic!("{resource_id} is a built-in item"))
                .id,
        ),
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

#[test]
fn a_golden_dandelion_locks_water_babies_without_an_animal_food_entry() {
    for name in [
        "minecraft:dolphin",
        "minecraft:squid",
        "minecraft:glow_squid",
        "minecraft:nautilus",
    ] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = GameType::Survival;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "age_locker", 1);
        let position = world.publication().players[0].position;
        let kind = bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(name)
            .unwrap()
            .id;
        let mut data = GeneralLivingData::new(kind).unwrap();
        data.set_no_ai(true);
        data.ageable_retained_mut().unwrap().set_age(-2_400);
        let id = world
            .spawn_stored_entity_for_test(
                Java26BuiltinDimension::Overworld,
                kind,
                Vec3d {
                    x: position.x + 1.0,
                    ..position
                },
                EntityData::GeneralLiving(data),
            )
            .0;
        world.set_inventory_slot_for_test(
            ConnectionId(1),
            0,
            stack_of("minecraft:golden_dandelion", 2),
        );

        let locked = play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );

        assert_eq!(
            locked.snapshot.players[0].inventory.slots[0].count, 1,
            "{name}"
        );
        let bbb_protocol::NbtSummaryValue::Compound(entries) = world
            .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
        else {
            panic!("entity save is a compound");
        };
        for field in ["AgeLocked", "PersistenceRequired"] {
            assert!(
                entries.iter().any(|entry| entry.name == field
                    && entry.value == bbb_protocol::NbtSummaryValue::Byte(1)),
                "{name}: {field}"
            );
        }
        assert_eq!(animal_record_int(&world, id, "Age"), -24_000, "{name}");
        assert!(locked.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
                if data.id == id && data.values.iter().any(|value|
                    value.data_id == bbb_protocol::entity_synched_data::VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID
                        && value.value == bbb_protocol::packets::EntityDataValueKind::Boolean(true))
        )), "{name}: age lock reaches the player");
    }
}

#[test]
fn a_cookie_poisons_an_invulnerable_parrot_and_otherwise_kills_it() {
    for (game_type, invulnerable, dies, remaining) in [
        (GameType::Survival, false, true, 1),
        (GameType::Survival, true, false, 1),
        (GameType::Creative, true, true, 2),
    ] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = game_type;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let position = world.publication().players[0].position;
        let parrot_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PARROT_ID;
        let dimension = Java26BuiltinDimension::Overworld;
        let parrot = world
            .spawn_stored_entity_for_test(
                dimension,
                parrot_type,
                Vec3d {
                    x: position.x + 1.0,
                    ..position
                },
                EntityData::GeneralLiving(GeneralLivingData::new(parrot_type).unwrap()),
            )
            .0;
        world
            .stored_entity_mut_for_test(dimension, parrot)
            .unwrap()
            .set_invulnerable(invulnerable);
        world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:cookie", 2));
        let fed = play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(parrot),
        );
        assert_eq!(fed.snapshot.players[0].inventory.slots[0].count, remaining);
        let parrot = world.stored_entity_for_test(dimension, parrot).unwrap();
        assert_eq!(!parrot.is_alive_with_health(), dies);
        if !dies {
            let poison = parrot
                .living()
                .unwrap()
                .active_effect("minecraft:poison")
                .unwrap();
            assert_eq!(poison.amplifier(), 0);
            assert!((898..=900).contains(&poison.duration()));
        }
    }
}

/// The `(byte)18` heart events one tick sent, by entity id.
pub(super) fn heart_events(tick: &TickResult) -> Vec<i32> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityEvent(packet),
                ..
            } if packet.event_id == 18 => Some(packet.entity_id),
            _ => None,
        })
        .collect()
}

/// How many experience orbs one tick told the session about.
pub(super) fn experience_orb_spawns(tick: &TickResult) -> usize {
    tick.events
        .iter()
        .filter(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::AddEntity(add),
                    ..
                } if add.entity_type_id
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID
            )
        })
        .count()
}

/// The `FROG_EAT` sounds one tick sent.
fn frog_eat_sounds(tick: &TickResult) -> Vec<(i32, SoundSource, f32, f32)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SoundEntity(sound),
                ..
            } if matches!(&sound.sound, SoundEventHolder::Reference { registry_id }
                if vanilla_sound_event_resource_id(*registry_id)
                    == Some("minecraft:entity.frog.eat")) =>
            {
                Some((sound.entity_id, sound.source, sound.volume, sound.pitch))
            }
            _ => None,
        })
        .collect()
}

/// The long-jump cooldown `FrogAi.initMemories` writes at spawn
/// (`FrogAi.java:63-65`), without which `FrogAi.updateActivity` (`:191`) never
/// leaves LONG_JUMP for the IDLE activity `AnimalMakeLove` sits in.
fn set_long_jump_cooldown(world: &mut ServerWorld, entity_id: i32) {
    let entity = world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("the frog is live");
    assert!(crate::entity::set_spawner_brain_memory(
        entity,
        crate::entity::ai::PackedMemory {
            memory_type: crate::entity::ai::MemoryModuleType::LongJumpCooldownTicks,
            value: crate::entity::ai::MemoryValue::Int(100),
            time_to_live: None,
        },
    ));
}

/// One frog of the breeding fixture, standing next to the session.
fn a_frog_beside_a_session(world: &mut ServerWorld, player_position: Vec3d, dx: f64) -> i32 {
    let frog_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FROG_ID;
    let (frog_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        frog_type,
        Vec3d {
            x: player_position.x + dx,
            y: player_position.y,
            z: player_position.z,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(frog_type).unwrap(),
        ),
    );
    set_long_jump_cooldown(world, frog_id);
    frog_id
}

fn an_axolotl_beside_a_session(world: &mut ServerWorld) -> i32 {
    let player_position = world.publication().players[0].position;
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            kind,
            Vec3d {
                x: player_position.x + 1.0,
                ..player_position
            },
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(kind).unwrap(),
            ),
        )
        .0
}

fn axolotl_feed_packet(entity_id: i32, hand: InteractionHand) -> PlayServerbound {
    PlayServerbound::Interact(InteractEntity {
        entity_id,
        hand,
        location: Vec3d::default(),
        using_secondary_action: false,
    })
}

fn stacked_fish_bucket(count: i32) -> ItemStackSummary {
    let mut fish = stack_of("minecraft:tropical_fish_bucket", count);
    fish.component_patch.max_stack_size = Some(16);
    fish.component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID);
    bbb_registry_codec::item_stack_summary_to_persistent_nbt(&fish)
        .expect("the patched bucket is a valid saved item stack");
    fish
}

#[test]
fn axolotl_filled_result_replaces_the_last_survival_bucket_and_gives_the_creative_bucket() {
    for creative in [false, true] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = if creative {
            GameType::Creative
        } else {
            GameType::Survival
        };
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "bucket_feeder", 0xfeed);
        let axolotl = an_axolotl_beside_a_session(&mut world);
        world.set_inventory_slot_for_test(
            ConnectionId(1),
            0,
            stack_of("minecraft:tropical_fish_bucket", 1),
        );
        world.set_inventory_slot_for_test(ConnectionId(1), 5, stack_of("minecraft:stone", 3));
        let before_events = world.game_events(Java26BuiltinDimension::Overworld).1;

        let fed = play(
            &sender,
            &receiver,
            &mut world,
            1,
            axolotl_feed_packet(axolotl, InteractionHand::MainHand),
        );

        assert_eq!(animal_record_int(&world, axolotl, "InLove"), 599);
        let slots = &fed.snapshot.players[0].inventory.slots;
        assert_eq!(
            slots[0],
            if creative {
                stack_of("minecraft:tropical_fish_bucket", 1)
            } else {
                stack_of("minecraft:water_bucket", 1)
            }
        );
        if creative {
            assert_eq!(slots[1], stack_of("minecraft:water_bucket", 1));
        } else {
            assert_eq!(slots[1], ItemStackSummary::empty());
        }
        assert_eq!(slots[5], stack_of("minecraft:stone", 3));
        assert_eq!(heart_events(&fed), vec![axolotl]);
        assert_eq!(
            world.game_events(Java26BuiltinDimension::Overworld).1,
            before_events + 1
        );
        assert!(world
            .game_events(Java26BuiltinDimension::Overworld)
            .0
            .iter()
            .any(|event| {
                event.game_event == VanillaGameEvent::ENTITY_INTERACT
                    && event.source_entity_id
                        == Some(world.player_entity_id_for_test(ConnectionId(1)))
            }));
        assert!(fed.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::EntityAnimation(animation), .. }
                if animation.id == world.player_entity_id_for_test(ConnectionId(1)) && animation.action == 0
        )));
    }
}

#[test]
fn creative_axolotl_feed_checks_offhand_and_does_not_drop_on_full_inventory() {
    for (existing_offhand_bucket, full_storage) in [(true, false), (false, true)] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = GameType::Creative;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "creative_feeder", 0xfeed);
        let axolotl = an_axolotl_beside_a_session(&mut world);
        if full_storage {
            for slot in 0..36 {
                world.set_inventory_slot_for_test(
                    ConnectionId(1),
                    slot,
                    stack_of("minecraft:stone", 64),
                );
            }
        }
        world.set_inventory_slot_for_test(
            ConnectionId(1),
            0,
            stack_of("minecraft:tropical_fish_bucket", 1),
        );
        if existing_offhand_bucket {
            world.set_inventory_slot_for_test(
                ConnectionId(1),
                40,
                stack_of("minecraft:water_bucket", 1),
            );
        }
        let fed = play(
            &sender,
            &receiver,
            &mut world,
            1,
            axolotl_feed_packet(axolotl, InteractionHand::MainHand),
        );
        assert_eq!(
            fed.snapshot.players[0].inventory.slots[0],
            stack_of("minecraft:tropical_fish_bucket", 1)
        );
        assert_eq!(
            fed.snapshot.players[0].inventory.slots[40],
            if existing_offhand_bucket {
                stack_of("minecraft:water_bucket", 1)
            } else {
                ItemStackSummary::empty()
            }
        );
        if existing_offhand_bucket {
            assert_eq!(
                fed.snapshot.players[0].inventory.slots[1],
                ItemStackSummary::empty()
            );
        }
        assert!(!fed.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::AddEntity(add), .. }
                if add.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID
        )));
    }
}

#[test]
fn stacked_axolotl_bucket_uses_offhand_and_drops_an_overflow_remainder() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "stacked_feeder", 0xfeed);
    let axolotl = an_axolotl_beside_a_session(&mut world);
    for slot in 0..36 {
        world.set_inventory_slot_for_test(ConnectionId(1), slot, stack_of("minecraft:stone", 64));
    }
    let mut fish = stacked_fish_bucket(2);
    world.set_inventory_slot_for_test(ConnectionId(1), 40, fish.clone());

    let fed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        axolotl_feed_packet(axolotl, InteractionHand::OffHand),
    );

    fish.count = 1;
    assert_eq!(fed.snapshot.players[0].inventory.slots[40], fish);
    assert_eq!(animal_record_int(&world, axolotl, "InLove"), 599);
    assert!(fed.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::EntityAnimation(animation), .. }
            if animation.id == world.player_entity_id_for_test(ConnectionId(1)) && animation.action == 3
    )));
    let drops: Vec<i32> = fed
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::AddEntity(add),
                ..
            } if add.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID => {
                Some(add.id)
            }
            _ => None,
        })
        .collect();
    assert_eq!(drops.len(), 1);
    let drop = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, drops[0])
        .expect("the bucket drop remains live");
    let crate::entity::EntityData::Item(item) = drop.data() else {
        panic!("the result is an item entity")
    };
    let dropped = item.item();
    assert_eq!(
        (dropped.item_resource_id(), dropped.count()),
        (Some("minecraft:water_bucket"), 1)
    );
}

#[test]
fn baby_axolotl_feeding_offers_a_bucket_without_a_server_swing() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "baby_feeder", 0xfeed);
    let axolotl = an_axolotl_beside_a_session(&mut world);
    set_animal_record_int(&mut world, axolotl, "Age", -24_000);
    let mut fish = stacked_fish_bucket(2);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, fish.clone());
    let before_events = world.game_events(Java26BuiltinDimension::Overworld).1;

    let fed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        axolotl_feed_packet(axolotl, InteractionHand::MainHand),
    );

    fish.count = 1;
    assert_eq!(fed.snapshot.players[0].inventory.slots[0], fish);
    assert_eq!(
        fed.snapshot.players[0].inventory.slots[1],
        stack_of("minecraft:water_bucket", 1)
    );
    assert_eq!(
        animal_record_int(&world, axolotl, "Age"),
        -24_000 + 120 * 20 + 1
    );
    assert_eq!(animal_record_int(&world, axolotl, "InLove"), 0);
    assert!(heart_events(&fed).is_empty());
    assert_eq!(
        world.game_events(Java26BuiltinDimension::Overworld).1,
        before_events + 1
    );
    assert!(!fed.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::EntityAnimation(animation), .. }
            if animation.id == world.player_entity_id_for_test(ConnectionId(1))
    )));
}

#[test]
fn a_player_feeding_a_baby_strider_hears_eating_only_when_food_is_accepted() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "strider_feeder", 0xfeed);
    let position = world.publication().players[0].position;
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID;
    let mut data = GeneralLivingData::new(kind).unwrap();
    data.set_no_ai(true);
    data.ageable_retained_mut().unwrap().set_age(-24_000);
    let (strider, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        kind,
        Vec3d {
            x: position.x + 1.0,
            ..position
        },
        EntityData::GeneralLiving(data),
    );
    let food = stack_of("minecraft:warped_fungus", 2);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, food.clone());

    let eating_sounds = |tick: &TickResult| {
        tick.events.iter().filter_map(|event| match event {
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                if matches!(&sound.sound, SoundEventHolder::Reference { registry_id }
                    if vanilla_sound_event_resource_id(*registry_id) == Some("minecraft:entity.strider.eat")) =>
                Some((sound.source, sound.volume, sound.pitch)),
            _ => None,
        }).collect::<Vec<_>>()
    };
    let fed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(strider),
    );

    assert_eq!(fed.snapshot.players[0].inventory.slots[0], food);
    assert!(animal_record_int(&world, strider, "Age") > -24_000);
    assert_eq!(animal_record_int(&world, strider, "ForcedAge"), 2_400);
    assert_eq!(animal_record_int(&world, strider, "InLove"), 0);
    let sounds = eating_sounds(&fed);
    let [(source, volume, pitch)] = sounds.as_slice() else {
        panic!("one strider eating sound: {sounds:?}")
    };
    assert_eq!((*source, *volume), (SoundSource::Neutral, 1.0));
    assert!(
        (0.8..=1.2).contains(pitch),
        "babies keep the adult eating pitch"
    );

    set_animal_record_int(&mut world, strider, "Age", 6_000);
    let refused = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(strider),
    );
    assert!(eating_sounds(&refused).is_empty());
    assert_eq!(refused.snapshot.players[0].inventory.slots[0], food);
    assert_eq!(animal_record_int(&world, strider, "InLove"), 0);
}

/// The player-feeding entry of the breeding chain on the real packet path
/// (`ServerGamePacketListenerImpl.handleInteract`, `:1813-1839`): a survival
/// player feeds two frogs `#minecraft:frog_food`, which puts both in love
/// through the state `animal_breeding.rs` reads, spends one slime ball per
/// feeding, and lets `AnimalMakeLove` mate the pair on a later tick.
/// `Armadillo.playEatingSound` (`Armadillo.java:345-347`).
#[test]
fn a_fed_armadillo_plays_its_eating_sound() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    join_loaded(
        &sender,
        &receiver,
        &mut world,
        1,
        "armadillo_feeder",
        0xfeed,
    );
    let position = world.publication().players[0].position;
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMADILLO_ID;
    let mut data = GeneralLivingData::new(kind).unwrap();
    data.set_no_ai(true);
    let (armadillo, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        kind,
        Vec3d {
            x: position.x + 1.0,
            ..position
        },
        EntityData::GeneralLiving(data),
    );
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:spider_eye", 1));

    let fed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(armadillo),
    );
    let sounds = fed.events.iter().filter_map(|event| match event {
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
            if matches!(&sound.sound, SoundEventHolder::Reference { registry_id }
                if vanilla_sound_event_resource_id(*registry_id) == Some("minecraft:entity.armadillo.eat")) =>
            Some((sound.source, sound.volume)),
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(sounds, vec![(SoundSource::Neutral, 1.0)]);
}

#[test]
fn a_player_feeding_two_frogs_puts_them_in_love_and_they_mate() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    let player_position = world.publication().players[0].position;
    let first = a_frog_beside_a_session(&mut world, player_position, 1.0);
    let second = a_frog_beside_a_session(&mut world, player_position, 2.5);
    // `#minecraft:frog_food` names exactly `minecraft:slime_ball`.
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:slime_ball", 2));

    let fed_first = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(first),
    );

    // `this.setInLove`'s timer and cause (`Animal.java:172-179`), which are the
    // two fields `readAdditionalSaveData` (`:104-108`) hands back on a reload.
    // The interaction runs in the packet phase of this tick and the frog's own
    // `aiStep` after it, which is where `Animal.aiStep` counts the timer down
    // (`:74-75`).
    assert_eq!(animal_record_int(&world, first, "InLove"), 600 - 1);
    assert_eq!(
        animal_record_love_cause(&world, first),
        Some(world.publication().players[0].uuid)
    );
    // `this.usePlayerItem(player, hand, itemStack)` (`:144`).
    assert_eq!(
        fed_first.snapshot.players[0].inventory.slots[0].count, 1,
        "one slime ball is spent"
    );
    // `this.level().broadcastEntityEvent(this, (byte)18)` (`:178`) and
    // `this.playEatingSound()` (`:146`), which is `FROG_EAT` at 2.0/1.0
    // (`Frog.java:235-236`).
    assert_eq!(heart_events(&fed_first), vec![first]);
    assert_eq!(
        frog_eat_sounds(&fed_first),
        vec![(first, SoundSource::Neutral, 2.0, 1.0)]
    );
    assert!(!fed_first.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
            if matches!(&sound.sound, SoundEventHolder::Reference { registry_id }
                if vanilla_sound_event_resource_id(*registry_id) == Some("minecraft:entity.frog.eat"))
    )));
    // `Mob.interact`'s `SUCCESS_SERVER` owes the server swing
    // (`Mob.java:1110-1111`, `:1831-1833`), which is the main-hand animation
    // (`LivingEntity.java:1999`).
    let player_entity_id = world.player_entity_id_for_test(ConnectionId(1));
    assert!(
        fed_first.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityAnimation(animation),
                ..
            } if animation.id == player_entity_id && animation.action == 0
        )),
        "the feeding swings the main hand"
    );

    let fed_second = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(second),
    );

    assert_eq!(animal_record_int(&world, second, "InLove"), 600 - 1);
    assert_eq!(
        fed_second.snapshot.players[0].inventory.slots[0].count, 0,
        "the second feeding spends the last slime ball"
    );

    // Both halves are in love, which is what `Animal.canMate` asks for
    // (`Animal.java:201-207`); `AnimalMakeLove` mates them on a later tick.
    let mut orbs = 0;
    let mut hearts = 0;
    for _ in 0..300 {
        let result = world.tick(receiver.drain());
        orbs += experience_orb_spawns(&result);
        hearts += heart_events(&result).len();
        if orbs > 0 {
            break;
        }
    }

    assert_eq!(orbs, 1, "the fed pair mated {orbs} times");
    assert_eq!(hearts, 1, "the mating broadcast {hearts} heart events");
    // `this.setAge(6000)` and `this.resetLove()` on both parents
    // (`Animal.java:224-227`).
    for frog in [first, second] {
        assert_eq!(animal_record_int(&world, frog, "InLove"), 0);
        assert!(animal_record_int(&world, frog, "Age") > 0);
    }
}

/// The growth branch of `Animal.mobInteract` (`Animal.java:150-155`): a baby
/// fed its food grows by `getSpeedUpSecondsWhenFeeding` instead of falling in
/// love. A frog cannot take this branch at all -- `Frog.isBaby` is a constant
/// false (`Frog.java:250-252`) -- so the baby here is a cow and the food is
/// `#minecraft:cow_food`.
#[test]
fn a_player_feeding_a_baby_cow_grows_it_up_instead() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "feeder", 0xfeed);
    let player_position = world.publication().players[0].position;
    let cow_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let (cow, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        cow_type,
        Vec3d {
            z: player_position.z + 1.0,
            ..player_position
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(cow_type).unwrap(),
        ),
    );
    // The `Age` `AgeableMob.setBaby(true)` writes (`AgeableMob.java:246-248`).
    set_animal_record_int(&mut world, cow, "Age", -24_000);
    assert_eq!(animal_record_int(&world, cow, "Age"), -24_000);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:wheat", 2));

    let fed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(cow),
    );

    // `this.ageUp(getSpeedUpSecondsWhenFeeding(-age), true)` with `-age` =
    // 24000 is 120 seconds, and `ForcedAge` takes the same delta
    // (`AgeableMob.java:123-142`, `:250-252`). The cow's own `aiStep` runs
    // after the packet phase of the same tick and ages a baby by one
    // (`:198-204`).
    assert_eq!(
        animal_record_int(&world, cow, "Age"),
        -24_000 + 120 * 20 + 1
    );
    assert_eq!(animal_record_int(&world, cow, "ForcedAge"), 2_400);
    // The growth branch answers `SUCCESS`, so the item is still spent
    // (`Animal.java:151`) while the love timer stays empty (`:143`).
    assert_eq!(animal_record_int(&world, cow, "InLove"), 0);
    assert_eq!(
        fed.snapshot.players[0].inventory.slots[0].count, 1,
        "one wheat is spent"
    );
    assert!(heart_events(&fed).is_empty());
    // `Animal.playEatingSound` is empty and `AbstractCow` does not override it
    // (`Animal.java:165-166`).
    assert!(frog_eat_sounds(&fed).is_empty());
}

/// `Turtle.ageBoundaryReached` (`Turtle.java:255-260`) on a baby turtle whose
/// growth seagrass sped up: the `gameplay/turtle_grow` scute drops when it
/// comes of age.
#[test]
fn a_fed_baby_turtle_drops_a_scute_when_it_grows_up() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "feeder", 0xfeed);
    let player_position = world.publication().players[0].position;
    let turtle_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID;
    let (turtle, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        turtle_type,
        Vec3d {
            z: player_position.z + 1.0,
            ..player_position
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(turtle_type).unwrap(),
        ),
    );
    set_animal_record_int(&mut world, turtle, "Age", -400);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:seagrass", 1));

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(turtle),
    );
    // `getSpeedUpSecondsWhenFeeding(400)` is two seconds (`AgeableMob.java:250-252`).
    assert_eq!(animal_record_int(&world, turtle, "Age"), -400 + 40 + 1);
    assert_eq!(
        world.publication().players[0].inventory.slots[0].count,
        0,
        "the seagrass is eaten"
    );
    for _ in 0..400 {
        if animal_record_int(&world, turtle, "Age") >= 0 {
            break;
        }
        world.tick(receiver.drain());
    }

    assert!(animal_record_int(&world, turtle, "Age") >= 0);
    let items: Vec<String> = world
        .stored_items_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .map(|(_, _, item, _, _, _)| item)
        .collect();
    assert_eq!(items, ["minecraft:turtle_scute"]);
}

#[test]
fn panda_packet_feeding_enters_love_and_grows_babies_with_server_swing() {
    for age in [0, -24_000] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = GameType::Survival;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "panda_feeder", 0xfeed);
        let position = world.publication().players[0].position;
        let dimension = Java26BuiltinDimension::Overworld;
        let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PANDA_ID;
        let mut data = GeneralLivingData::new(kind).unwrap();
        data.set_no_ai(true);
        data.ageable_retained_mut().unwrap().set_age(age);
        let (id, _) = world.spawn_stored_entity_for_test(
            dimension,
            kind,
            Vec3d {
                z: position.z + 1.0,
                ..position
            },
            EntityData::GeneralLiving(data),
        );
        world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:bamboo", 2));
        let fed = play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        assert_eq!(fed.snapshot.players[0].inventory.slots[0].count, 1);
        if age == 0 {
            assert!(animal_record_int(&world, id, "InLove") > 0);
            assert_eq!(
                animal_record_love_cause(&world, id),
                Some(world.publication().players[0].uuid)
            );
        } else {
            assert!((-21_600..=-21_599).contains(&animal_record_int(&world, id, "Age")));
            assert_eq!(animal_record_int(&world, id, "ForcedAge"), 2_400);
            assert_eq!(animal_record_int(&world, id, "InLove"), 0);
        }
        assert!(fed.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::EntityAnimation(animation), .. }
                if animation.id == world.player_entity_id_for_test(ConnectionId(1)) && animation.action == 0
        )));
        let bbb_protocol::NbtSummaryValue::Compound(record) = world
            .stored_entity_command_data_for_test(dimension, id)
            .unwrap()
        else {
            unreachable!()
        };
        let saved_age = animal_record_int(&world, id, "Age");
        world.load_stored_entity_record_for_test(dimension, id, &record);
        assert_eq!(animal_record_int(&world, id, "Age"), saved_age);
    }
}

#[test]
fn panda_packet_feeding_swaps_held_stack_and_finishes_sitting_meal() {
    use bbb_protocol::packets::EquipmentSlot;
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "panda_feeder", 0xfeed);
    let position = world.publication().players[0].position;
    let dimension = Java26BuiltinDimension::Overworld;
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PANDA_ID;
    let mut data = GeneralLivingData::new(kind).unwrap();
    data.set_no_ai(true);
    data.ageable_retained_mut().unwrap().set_age(6_000);
    let mut old = stack_of("minecraft:stick", 3);
    old.component_patch.custom_data = Some(bbb_protocol::NbtSummaryValue::Compound(vec![
        bbb_protocol::NbtSummaryEntry {
            name: "marker".into(),
            value: bbb_protocol::NbtSummaryValue::Int(7),
        },
    ]));
    let old = bbb_server_types::item_stack::RuntimeItemStack::from_summary(old);
    data.living_mut()
        .set_item_slot(EquipmentSlot::MainHand, Some(old.clone()));
    let (id, _) = world.spawn_stored_entity_for_test(
        dimension,
        kind,
        Vec3d {
            z: position.z + 1.0,
            ..position
        },
        EntityData::GeneralLiving(data),
    );
    let mut bamboo = stack_of("minecraft:bamboo", 2);
    bamboo.component_patch.custom_name = Some("Lunch".into());
    world.set_inventory_slot_for_test(ConnectionId(1), 0, bamboo.clone());
    let fed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    bamboo.count = 1;
    assert_eq!(fed.snapshot.players[0].inventory.slots[0], bamboo);
    let hand = |world: &ServerWorld| {
        world
            .stored_entity_for_test(dimension, id)
            .unwrap()
            .living()
            .unwrap()
            .equipment()
            .get(EquipmentSlot::MainHand)
            .cloned()
    };
    assert_eq!(
        hand(&world).unwrap().summary(),
        &stack_of("minecraft:bamboo", 1)
    );
    let drops: Vec<_> = fed
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::AddEntity(add),
                ..
            } if add.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID => {
                Some(add.id)
            }
            _ => None,
        })
        .collect();
    assert_eq!(drops.len(), 1);
    let EntityData::Item(drop) = world
        .stored_entity_for_test(dimension, drops[0])
        .unwrap()
        .data()
    else {
        unreachable!()
    };
    assert_eq!(drop.item(), &old);
    let bbb_protocol::NbtSummaryValue::Compound(record) = world
        .stored_entity_command_data_for_test(dimension, id)
        .unwrap()
    else {
        unreachable!()
    };
    world.load_stored_entity_record_for_test(dimension, id, &record);
    assert_eq!(
        hand(&world).unwrap().summary(),
        &stack_of("minecraft:bamboo", 1)
    );
    let mut heard_eating = false;
    let mut ate = false;
    let mut stopped_eating = false;
    let mut stood_up = false;
    for _ in 0..20_000 {
        let tick = world.tick(receiver.drain());
        for event in &tick.events {
            if let WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::KeepAlive { id },
            } = event
            {
                enqueue(
                    &sender,
                    WorldCommand::Play {
                        connection_id: *connection_id,
                        packet: PlayServerbound::KeepAlive { id: *id },
                    },
                )
                .unwrap();
            }
        }
        ate |= world.game_events(dimension).0.iter().any(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::EAT
                && event.source_entity_id == Some(id)
        });
        heard_eating |= tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                if matches!(&sound.sound, SoundEventHolder::Reference { registry_id }
                    if vanilla_sound_event_resource_id(*registry_id) == Some("minecraft:entity.panda.eat"))
        ));
        if hand(&world).is_none() {
            for event in &tick.events {
                if let WorldEvent::Send {
                    packet: ClientboundPlayPacket::SetEntityData(data),
                    ..
                } = event
                {
                    if data.id == id {
                        stopped_eating |= data.values.iter().any(|row|
                            row.data_id == bbb_protocol::entity_synched_data::VANILLA_PANDA_EAT_COUNTER_DATA_ID
                                && row.value == bbb_protocol::packets::EntityDataValueKind::Int(0));
                        stood_up |= data.values.iter().any(|row| {
                            row.data_id
                                == bbb_protocol::entity_synched_data::VANILLA_PANDA_FLAGS_DATA_ID
                                && row.value == bbb_protocol::packets::EntityDataValueKind::Byte(0)
                        });
                    }
                }
            }
        }
        if hand(&world).is_none() && stopped_eating && stood_up && ate {
            break;
        }
    }
    assert!(heard_eating);
    assert!(
        hand(&world).is_none(),
        "the panda eventually eats its bamboo"
    );
    assert!(stopped_eating);
    assert!(stood_up);
    assert!(ate);
}
