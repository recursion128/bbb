//! Autonomous goat jumps, rams, horn drops and saved cooldowns on real world ticks.

use super::*;
use crate::entity::ai::{BrainPacked, MemoryModuleType, MemoryValue};
use crate::tests::entities::{console_command, restore_entity_writes};
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use std::sync::atomic::{AtomicU64, Ordering};

static COMMAND_ID: AtomicU64 = AtomicU64::new(100);

fn config() -> ServerWorldConfig {
    ServerWorldConfig {
        game_type: GameType::Creative,
        difficulty: Difficulty::Normal,
        keep_alive_interval_ticks: 10_000,
        ..datapack_config()
    }
}

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    assert!(text.chars().count() <= 256);
    let tick = console_command(
        sender,
        receiver,
        world,
        COMMAND_ID.fetch_add(1, Ordering::Relaxed),
        text,
    );
    let outcome = admin_outcome(&tick);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
    tick
}

fn prepare(sender: &WorldCommandSender, receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    world.tick_rate_mut().set_frozen(true);
    join_loaded(sender, receiver, world, 1, "alice", 1);
    install_floor(world);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 12.5,
        },
    );
    command(sender, receiver, world, "gamerule spawn_mobs false");
    crate::tests::drain_chunk_deliveries(sender, receiver, world, 1);
}

fn summon(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> (i32, Uuid) {
    let before = world.stored_entity_ids_for_test(DIMENSION);
    command(sender, receiver, world, text);
    let id = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .find(|id| !before.contains(id))
        .unwrap();
    (
        id,
        world.stored_entity_for_test(DIMENSION, id).unwrap().uuid(),
    )
}

fn fixture(
    baby: bool,
    screaming: bool,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
    i32,
    Uuid,
) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config());
    prepare(&sender, &receiver, &mut world);
    let (goat, uuid) = summon(&sender, &receiver, &mut world, &format!(
        "summon minecraft:goat 3.5 65 8.5 {{Age:{},IsScreamingGoat:{}b,PersistenceRequired:1b,Brain:{{memories:{{\"minecraft:ram_cooldown_ticks\":{{value:1}}}}}}}}", if baby { -24000 } else { 0 }, i32::from(screaming)));
    let (victim, victim_uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:cow 10.5 65 8.5 {NoAI:1b,NoGravity:1b,PersistenceRequired:1b}",
    );
    settle_and_enable_ram(&sender, &receiver, &mut world, goat, uuid);
    (sender, receiver, world, goat, uuid, victim, victim_uuid)
}

fn record(world: &ServerWorld, id: i32) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(fields) = world
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap()
    else {
        panic!("entity record is a compound");
    };
    fields
}

fn health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .health()
}

fn cooldown_from(fields: &[NbtSummaryEntry]) -> Option<i32> {
    brain_cooldown_from(fields, MemoryModuleType::RamCooldownTicks)
}

fn brain_cooldown_from(fields: &[NbtSummaryEntry], kind: MemoryModuleType) -> Option<i32> {
    BrainPacked::decode(field(fields, "Brain"))
        .unwrap()
        .memories
        .into_iter()
        .find_map(|memory| match (memory.memory_type, memory.value) {
            (memory_type, MemoryValue::Int(value)) if memory_type == kind => Some(value),
            _ => None,
        })
}

fn event(events: &[WorldEvent], goat: i32, value: i8) -> bool {
    events.iter().any(|event| {
        matches!(
            event.sent_packet(),
            Some(ClientboundPlayPacket::EntityEvent(update))
                if update.entity_id == goat && update.event_id == value
        )
    })
}

fn prepare_sound(events: &[WorldEvent], goat: i32) -> bool {
    events.iter().any(|event| {
        let sound = match event.sent_packet() {
            Some(ClientboundPlayPacket::Sound(sound)) => &sound.sound,
            Some(ClientboundPlayPacket::SoundEntity(sound)) if sound.entity_id == goat => {
                &sound.sound
            }
            _ => return false,
        };
        matches!(sound, SoundEventHolder::Reference { registry_id }
            if vanilla_sound_event_resource_id(*registry_id).is_some_and(|name| name.contains("entity.goat.") && name.ends_with("prepare_ram")))
    })
}

fn ram(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    goat: i32,
    wall: bool,
) -> Vec<WorldEvent> {
    world.tick_rate_mut().set_frozen(false);
    let mut events = Vec::new();
    let mut placed_wall = false;
    for _ in 0..500 {
        let tick = world.tick(receiver.drain());
        let finished = event(&tick.events, goat, 59);
        let prepared = prepare_sound(&tick.events, goat);
        events.extend(tick.events);
        if wall && !placed_wall && prepared {
            world.tick_rate_mut().set_frozen(true);
            command(
                sender,
                receiver,
                world,
                "fill 7 65 0 7 66 15 minecraft:stone",
            );
            world.tick_rate_mut().set_frozen(false);
            placed_wall = true;
        }
        if finished {
            world.tick_rate_mut().set_frozen(true);
            assert!(
                event(&events, goat, 58),
                "the autonomous goat lowers its head before charging"
            );
            if wall {
                assert!(
                    placed_wall,
                    "the wall is placed after the charge's audible preparation"
                );
            }
            return events;
        }
    }
    panic!(
        "the goat never finished an autonomous ram: {:?}",
        world
            .stored_entity_for_test(DIMENSION, goat)
            .unwrap()
            .position()
    );
}

fn pushed(events: &[WorldEvent], victim: i32) -> bool {
    events.iter().any(|event| matches!(
        event.sent_packet(),
        Some(ClientboundPlayPacket::SetEntityMotion(update))
            if update.id == victim && update.delta_movement.x.hypot(update.delta_movement.z) > 0.2
    ))
}

fn settle_and_enable_ram(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    goat: i32,
    uuid: Uuid,
) {
    // A freshly summoned body needs a real collision before ground pathfinding.
    command(sender, receiver, world, &format!(
        "data merge entity {uuid} {{NoAI:0b,Brain:{{memories:{{\"minecraft:ram_cooldown_ticks\":{{value:100}}}}}}}}"));
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..20 {
        if world
            .stored_entity_for_test(DIMENSION, goat)
            .unwrap()
            .on_ground()
        {
            break;
        }
        world.tick(receiver.drain());
    }
    world.tick_rate_mut().set_frozen(true);
    assert!(world
        .stored_entity_for_test(DIMENSION, goat)
        .unwrap()
        .on_ground());
    assert!(cooldown_from(&record(world, goat)).unwrap() > 0);
    command(
        sender,
        receiver,
        world,
        &format!("data merge entity {uuid} {{Brain:{{memories:{{\"minecraft:ram_cooldown_ticks\":{{value:1}}}}}}}}"),
    );
}

fn reset_ram(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    uuid: Uuid,
) {
    command(sender, receiver, world, &format!(
        "data merge entity {uuid} {{NoAI:1b,Motion:[0d,0d,0d],Brain:{{memories:{{\"minecraft:ram_cooldown_ticks\":{{value:1}}}}}}}}"));
    command(sender, receiver, world, &format!("tp {uuid} 3.5 65 8.5"));
    let goat = world
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .unwrap();
    settle_and_enable_ram(sender, receiver, world, goat, uuid);
}

fn horn_ids(world: &ServerWorld) -> Vec<i32> {
    world.stored_entity_ids_for_test(DIMENSION).into_iter().filter(|id| {
        matches!(world.stored_entity_for_test(DIMENSION, *id).unwrap().data(), EntityData::Item(item)
            if item.item().item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_GOAT_HORN_ID))
    }).collect()
}

fn horn_flags(world: &ServerWorld, id: i32) -> [bool; 2] {
    let fields = record(world, id);
    let saved = ["HasLeftHorn", "HasRightHorn"]
        .map(|name| field(&fields, name) == &NbtSummaryValue::Byte(1));
    let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
    let values = crate::entity::tracking::synched_values(body);
    for (data_id, expected) in [
        bbb_protocol::entity_synched_data::VANILLA_GOAT_LEFT_HORN_DATA_ID,
        bbb_protocol::entity_synched_data::VANILLA_GOAT_RIGHT_HORN_DATA_ID,
    ]
    .into_iter()
    .zip(saved)
    {
        assert!(values.iter().any(|row| row.value.data_id == data_id
            && row.value.value == bbb_protocol::packets::EntityDataValueKind::Boolean(expected)));
    }
    saved
}

#[test]
fn an_adult_goat_rams_a_living_target_and_restores_its_paused_cooldown() {
    let (sender, receiver, mut world, goat, uuid, victim, victim_uuid) = fixture(false, false);
    let before = health(&world, victim);
    let events = ram(&sender, &receiver, &mut world, goat, false);
    assert_eq!(health(&world, victim), before - 2.0);
    let kind = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|entry| entry.resource_id == "minecraft:mob_attack_no_aggro")
        .unwrap()
        .registry_id;
    assert!(events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::DamageEvent(hit), .. }
        if hit.entity_id == victim && hit.source_cause_id == goat && hit.source_type_id == kind)));
    assert!(
        pushed(&events, victim)
            || world
                .stored_entity_for_test(DIMENSION, victim)
                .unwrap()
                .delta_movement()
                .x
                > 0.2
    );
    let remaining = cooldown_from(&record(&world, goat)).unwrap();
    assert!((600..=6000).contains(&remaining));
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b,Motion:[0d,0d,0d]}}"),
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    world.tick_rate_mut().set_frozen(true);
    assert_eq!(cooldown_from(&record(&world, goat)), Some(remaining));
    let request = world
        .save_request(1_786_000_014_000, 214, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    assert_eq!(field(&saved, "NoAI"), &NbtSummaryValue::Byte(1));
    assert_eq!(cooldown_from(&saved), Some(remaining));
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let goat = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .unwrap();
    let victim = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, victim_uuid)
        .unwrap();
    assert_eq!(cooldown_from(&record(&restored, goat)), Some(remaining));
    assert_eq!(
        field(&record(&restored, goat), "NoAI"),
        &NbtSummaryValue::Byte(1)
    );
    prepare(&sender, &receiver, &mut restored);
    assert!(restored
        .player_tracking_viewers_for_test(DIMENSION, goat)
        .contains(&ConnectionId(1)));
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    restored.tick_rate_mut().set_frozen(false);
    for _ in 0..10 {
        restored.tick(receiver.drain());
    }
    restored.tick_rate_mut().set_frozen(true);
    assert!(cooldown_from(&record(&restored, goat)).unwrap() < remaining);
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {victim_uuid} {{Health:10f,Motion:[0d,0d,0d]}}"),
    );
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("tp {victim_uuid} 10.5 65 8.5"),
    );
    reset_ram(&sender, &receiver, &mut restored, uuid);
    let before = health(&restored, victim);
    ram(&sender, &receiver, &mut restored, goat, false);
    assert_eq!(health(&restored, victim), before - 2.0);
}

#[test]
fn regular_and_screaming_goats_drop_two_matching_instruments_when_a_charge_meets_a_wall() {
    for screaming in [false, true] {
        let (sender, receiver, mut world, goat, uuid, victim, _) = fixture(false, screaming);
        let startup = datapack_config().datapacks.unwrap();
        let resolver = FrozenDatapackRegistryLookup::from_startup(&startup);
        let mut instruments = Vec::new();
        for attempt in 0..3 {
            if attempt > 0 {
                reset_ram(&sender, &receiver, &mut world, uuid);
            }
            let before_flags = horn_flags(&world, goat);
            let mut events = ram(&sender, &receiver, &mut world, goat, true);
            assert_eq!(
                health(&world, victim),
                10.0,
                "the inserted wall stops the charge before its victim"
            );
            let drops = horn_ids(&world);
            assert_eq!(drops.len(), (attempt + 1).min(2));
            let flags = horn_flags(&world, goat);
            assert_eq!(
                flags.into_iter().filter(|present| *present).count(),
                2 - (attempt + 1).min(2)
            );
            let remaining = cooldown_from(&record(&world, goat)).unwrap();
            assert!(if screaming {
                (100..=300).contains(&remaining)
            } else {
                (600..=6000).contains(&remaining)
            });
            if attempt < 2 {
                let drop = *drops
                    .iter()
                    .find(|id| !instruments.iter().any(|(seen, _)| seen == *id))
                    .unwrap();
                let body = world.stored_entity_for_test(DIMENSION, drop).unwrap();
                let EntityData::Item(item) = body.data() else {
                    unreachable!()
                };
                assert_eq!(item.item().count(), 1);
                assert_eq!(item.pickup_delay(), 0);
                assert!(body.delta_movement().y > 0.0);
                let NbtSummaryValue::String(instrument) = item.item().component_nbt(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID, &resolver).unwrap().unwrap() else { panic!("a horn names its instrument"); };
                let instrument = instrument.try_to_string().unwrap();
                assert!(world.level_registry_tags_for_test(DIMENSION).contains(
                    "minecraft:instrument",
                    if screaming {
                        "minecraft:screaming_goat_horns"
                    } else {
                        "minecraft:regular_goat_horns"
                    },
                    &instrument
                ));
                instruments.push((drop, instrument));
                let lost_horn = [
                    bbb_protocol::entity_synched_data::VANILLA_GOAT_LEFT_HORN_DATA_ID,
                    bbb_protocol::entity_synched_data::VANILLA_GOAT_RIGHT_HORN_DATA_ID,
                ]
                .into_iter()
                .zip(before_flags.into_iter().zip(flags))
                .find_map(|(data_id, (before, after))| (before && !after).then_some(data_id))
                .unwrap();
                let published = |events: &[WorldEvent]| {
                    events.iter().any(|event| matches!(event,
                    WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(update), .. }
                    if update.id == goat && update.values.iter().any(|row|
                        row.data_id == lost_horn && row.value == bbb_protocol::packets::EntityDataValueKind::Boolean(false))))
                };
                world.tick_rate_mut().set_frozen(false);
                for _ in 0..20 {
                    if published(&events) {
                        break;
                    }
                    events.extend(world.tick(receiver.drain()).events);
                }
                world.tick_rate_mut().set_frozen(true);
                assert!(
                    published(&events),
                    "the broken horn is published to the client"
                );
            }
            command(
                &sender,
                &receiver,
                &mut world,
                "fill 7 65 0 7 66 15 minecraft:air",
            );
        }
        assert_eq!(
            instruments[0].1, instruments[1].1,
            "the same goat's two horns use the same instrument"
        );
    }
}

#[test]
fn a_screaming_baby_rams_for_one_damage_and_pushes_even_when_damage_is_resisted() {
    let (sender, receiver, mut world, goat, uuid, victim, victim_uuid) = fixture(true, true);
    let before = health(&world, victim);
    ram(&sender, &receiver, &mut world, goat, false);
    assert_eq!(health(&world, victim), before - 1.0);
    assert!((100..=300).contains(&cooldown_from(&record(&world, goat)).unwrap()));
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {victim_uuid} {{Health:10f,Motion:[0d,0d,0d]}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("tp {victim_uuid} 10.5 65 8.5"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("effect give {victim_uuid} minecraft:resistance 60 4"),
    );
    reset_ram(&sender, &receiver, &mut world, uuid);
    let events = ram(&sender, &receiver, &mut world, goat, false);
    assert_eq!(
        health(&world, victim),
        10.0,
        "resistance rejects the ram's damage"
    );
    assert!(
        pushed(&events, victim)
            || world
                .stored_entity_for_test(DIMENSION, victim)
                .unwrap()
                .delta_movement()
                .x
                > 0.2,
        "the ram still knocks back a target whose damage is rejected"
    );
    assert!(horn_ids(&world).is_empty());
}

#[test]
fn an_online_survival_player_is_rammed_and_can_block_the_damage_with_a_raised_shield() {
    use crate::tests::using_item::{stack, Fixture, OFFHAND_MENU_SLOT, OFFHAND_SLOT};

    let mut fixture = Fixture::with_config(GameType::Survival, config());
    fixture.world.tick_rate_mut().set_frozen(true);
    install_floor(&mut fixture.world);
    command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        "gamerule spawn_mobs false",
    );
    command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        "gamemode spectator bob",
    );
    let target = Vec3d {
        x: 8.5,
        y: 65.0,
        z: 3.5,
    };
    fixture
        .world
        .set_player_position_for_test(ConnectionId(1), target);
    let (goat, uuid) = summon(&fixture.sender, &fixture.receiver, &mut fixture.world,
        "summon minecraft:goat 8.5 65 10.5 {PersistenceRequired:1b,Brain:{memories:{\"minecraft:ram_cooldown_ticks\":{value:1}}}}");
    settle_and_enable_ram(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        goat,
        uuid,
    );
    // Four damage also exercises the shield's durability threshold.
    command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        &format!("attribute {uuid} minecraft:attack_damage base set 4"),
    );
    let before = fixture.world.player_health_for_test(ConnectionId(1));
    let mut hit = ram(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        goat,
        false,
    );
    assert_eq!(
        fixture.world.player_health_for_test(ConnectionId(1)),
        before - 4.0
    );
    fixture.world.tick_rate_mut().set_frozen(false);
    for _ in 0..4 {
        hit.extend(fixture.idle().events);
    }
    fixture.world.tick_rate_mut().set_frozen(true);
    assert!(hit.iter().any(|event| matches!(
        event.sent_packet(),
        Some(ClientboundPlayPacket::DamageEvent(hurt))
            if hurt.entity_id == fixture.alice && hurt.source_cause_id == goat
    )));
    assert!(
        pushed(&hit, fixture.alice),
        "the online victim receives ram motion packets"
    );
    command(&fixture.sender, &fixture.receiver, &mut fixture.world, &format!(
        "data merge entity {uuid} {{NoAI:1b,Motion:[0d,0d,0d],Brain:{{memories:{{\"minecraft:ram_cooldown_ticks\":{{value:1}}}}}}}}"));
    command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        &format!("tp {uuid} 8.5 65 10.5"),
    );
    fixture
        .world
        .set_player_position_for_test(ConnectionId(1), target);
    command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        "effect give alice minecraft:instant_health 1 1",
    );
    fixture.hold(OFFHAND_MENU_SLOT, stack("minecraft:shield", 1));
    fixture.world.tick_rate_mut().set_frozen(false);
    fixture.use_item(InteractionHand::OffHand);
    for _ in 0..6 {
        fixture.idle();
    }
    fixture.world.tick_rate_mut().set_frozen(true);
    assert!(
        fixture.use_state().0,
        "the shield is raised through a real UseItem packet"
    );
    assert_eq!(fixture.use_state().1, InteractionHand::OffHand);
    assert_eq!(fixture.world.player_health_for_test(ConnectionId(1)), 20.0);
    settle_and_enable_ram(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        goat,
        uuid,
    );
    command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        &format!("attribute {uuid} minecraft:attack_damage base set 4"),
    );
    let mut blocked = ram(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        goat,
        false,
    );
    fixture.world.tick_rate_mut().set_frozen(false);
    for _ in 0..4 {
        blocked.extend(fixture.idle().events);
    }
    fixture.world.tick_rate_mut().set_frozen(true);
    assert_eq!(
        fixture.world.player_health_for_test(ConnectionId(1)),
        20.0,
        "the shield faces the goat and blocks the charge's damage"
    );
    assert!(
        fixture
            .slot(OFFHAND_SLOT)
            .component_patch
            .damage
            .unwrap_or(0)
            > 0,
        "blocking the ram wears the actual shield"
    );
    assert!(blocked.iter().any(|event| match event.sent_packet() {
        Some(ClientboundPlayPacket::Sound(sound)) => matches!(sound.sound,
            SoundEventHolder::Reference { registry_id } if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:item.shield.block")),
        _ => false,
    }));
    assert!(
        pushed(&blocked, fixture.alice),
        "blocking the damage still leaves visible ram knockback"
    );
}

fn jump_platforms(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) {
    for text in [
        "setblock 3 99 8 minecraft:stone",
        "fill 7 99 7 8 99 9 minecraft:stone",
    ] {
        command(sender, receiver, world, text);
    }
}

#[test]
fn a_goat_long_jumps_across_a_gap_and_restores_its_paused_cooldown() {
    use bbb_protocol::entity_poses::{
        VANILLA_ENTITY_POSE_LONG_JUMPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
    };
    use bbb_protocol::packets::EntityDataValueKind;

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config());
    prepare(&sender, &receiver, &mut world);
    jump_platforms(&sender, &receiver, &mut world);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 8.5,
            y: 100.0,
            z: 12.5,
        },
    );
    let (goat, uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:goat 3.5 100 8.5 {PersistenceRequired:1b,Brain:{memories:{\"minecraft:ram_cooldown_ticks\":{value:10000},\"minecraft:long_jump_cooling_down\":{value:100}}}}",
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..20 {
        if world
            .stored_entity_for_test(DIMENSION, goat)
            .unwrap()
            .on_ground()
        {
            break;
        }
        world.tick(receiver.drain());
    }
    world.tick_rate_mut().set_frozen(true);
    let body = world.stored_entity_for_test(DIMENSION, goat).unwrap();
    assert!(body.on_ground());
    let origin = body.position();
    let standing = body.bounding_box();
    let standing_width = standing.max_x - standing.min_x;
    let standing_height = standing.max_y - standing.min_y;
    assert!(world
        .player_tracking_viewers_for_test(DIMENSION, goat)
        .contains(&ConnectionId(1)));
    let (rider, rider_uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:armor_stand 3.5 102 8.5 {Small:1b,NoGravity:1b,Invulnerable:1b}",
    );
    let mut events = command(
        &sender,
        &receiver,
        &mut world,
        &format!("ride {rider_uuid} mount {uuid}"),
    )
    .events;
    assert_eq!(world.stored_passengers_for_test(DIMENSION, goat), &[rider]);
    let standing_attachment =
        bbb_protocol::entity_types::vanilla_entity_type_passenger_attachments(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID,
        )
        .unwrap()[0];
    command(
        &sender,
        &receiver,
        &mut world,
        &format!(
            "data merge entity {uuid} {{Brain:{{memories:{{\"minecraft:long_jump_cooling_down\":{{value:1}}}}}}}}"
        ),
    );

    let mut trace = Vec::new();
    let mut rose_and_travelled = false;
    let mut compact_jump = false;
    let mut landed = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..240 {
        events.extend(world.tick(receiver.drain()).events);
        let body = world.stored_entity_for_test(DIMENSION, goat).unwrap();
        let position = body.position();
        let motion = body.delta_movement();
        let bounds = body.bounding_box();
        assert_eq!(world.stored_passengers_for_test(DIMENSION, goat), &[rider]);
        let rider_position = world
            .stored_entity_for_test(DIMENSION, rider)
            .unwrap()
            .position();
        let expected_height = if body.pose_id() == VANILLA_ENTITY_POSE_LONG_JUMPING_ID {
            bounds.max_y - bounds.min_y
        } else {
            f64::from(standing_attachment[1])
        };
        for (offset, expected) in [
            rider_position.x - position.x,
            rider_position.y - position.y,
            rider_position.z - position.z,
        ]
        .into_iter()
        .zip([0.0, expected_height, 0.0])
        {
            assert!((offset - expected).abs() < 1.0e-5);
        }
        trace.push((position, motion, body.pose_id(), body.on_ground(), bounds));
        rose_and_travelled |=
            position.y > origin.y + 0.5 && position.x > origin.x + 0.5 && motion.y > 0.0;
        if body.pose_id() == VANILLA_ENTITY_POSE_LONG_JUMPING_ID {
            assert!(((bounds.max_x - bounds.min_x) / standing_width - 0.7).abs() < 0.001);
            assert!(((bounds.max_y - bounds.min_y) / standing_height - 0.7).abs() < 0.001);
            compact_jump = true;
        }
        if compact_jump && body.on_ground() && body.pose_id() == VANILLA_ENTITY_POSE_STANDING_ID {
            assert!(
                (7.0..9.0).contains(&position.x)
                    && (7.0..10.0).contains(&position.z)
                    && (position.y - 100.0).abs() < 0.001,
                "landing: {trace:?}"
            );
            assert!((bounds.max_x - bounds.min_x - standing_width).abs() < 0.001);
            assert!((bounds.max_y - bounds.min_y - standing_height).abs() < 0.001);
            landed = true;
            break;
        }
    }
    world.tick_rate_mut().set_frozen(true);
    assert!(
        rose_and_travelled && compact_jump && landed,
        "jump: {trace:?}"
    );
    let remaining = brain_cooldown_from(
        &record(&world, goat),
        MemoryModuleType::LongJumpCooldownTicks,
    )
    .unwrap();
    assert!((600..=1200).contains(&remaining));
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..8 {
        events.extend(world.tick(receiver.drain()).events);
        let body = world.stored_entity_for_test(DIMENSION, goat).unwrap();
        let position = body.position();
        assert!(
            body.on_ground()
                && (7.0..9.0).contains(&position.x)
                && (7.0..10.0).contains(&position.z)
                && (position.y - 100.0).abs() < 0.001,
            "the landed goat stays on the platform: {position:?}"
        );
    }
    world.tick_rate_mut().set_frozen(true);
    assert!(events.iter().any(|event| matches!(
        event.packet_for(ConnectionId(1)),
        Some(ClientboundPlayPacket::SetPassengers(packet))
            if packet.vehicle_id == goat && packet.passenger_ids == [rider]
    )));
    assert!(!events.iter().any(|event| matches!(
        event.packet_for(ConnectionId(1)),
        Some(ClientboundPlayPacket::SetPassengers(packet))
            if packet.vehicle_id == goat && packet.passenger_ids.is_empty()
    )));
    for pose in [
        VANILLA_ENTITY_POSE_LONG_JUMPING_ID,
        VANILLA_ENTITY_POSE_STANDING_ID,
    ] {
        assert!(
            events.iter().any(|event| matches!(
                event.packet_for(ConnectionId(1)),
                Some(ClientboundPlayPacket::SetEntityData(update))
                    if update.id == goat && update.values.iter().any(|row|
                        row.value == EntityDataValueKind::Pose(pose))
            )),
            "the viewer receives pose {pose}"
        );
    }
    for expected in [
        "minecraft:entity.goat.long_jump",
        "minecraft:entity.goat.step",
    ] {
        assert!(
            events.iter().any(|event| {
                let sound = match event.sent_packet() {
                    Some(ClientboundPlayPacket::Sound(sound)) => &sound.sound,
                    Some(ClientboundPlayPacket::SoundEntity(sound)) if sound.entity_id == goat => {
                        &sound.sound
                    }
                    _ => return false,
                };
                matches!(sound, SoundEventHolder::Reference { registry_id }
                if vanilla_sound_event_resource_id(*registry_id) == Some(expected))
            }),
            "the viewer hears {expected}"
        );
    }
    let remaining = brain_cooldown_from(
        &record(&world, goat),
        MemoryModuleType::LongJumpCooldownTicks,
    )
    .unwrap();
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    world.tick_rate_mut().set_frozen(true);
    assert_eq!(
        brain_cooldown_from(
            &record(&world, goat),
            MemoryModuleType::LongJumpCooldownTicks
        ),
        Some(remaining)
    );
    let request = world
        .save_request(1_786_000_014_001, 215, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    assert_eq!(field(&saved, "NoAI"), &NbtSummaryValue::Byte(1));
    let NbtSummaryValue::List(passengers) = field(&saved, "Passengers") else {
        panic!("the saved goat retains its passenger");
    };
    assert!(passengers.iter().any(|passenger| matches!(passenger,
        NbtSummaryValue::Compound(fields) if field(fields, "UUID") == &nbt_uuid(rider_uuid))));
    assert_eq!(
        brain_cooldown_from(&saved, MemoryModuleType::LongJumpCooldownTicks),
        Some(remaining)
    );
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let goat = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .unwrap();
    let rider = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, rider_uuid)
        .unwrap();
    assert_eq!(
        restored.stored_passengers_for_test(DIMENSION, goat),
        &[rider]
    );
    assert_eq!(
        field(&record(&restored, goat), "NoAI"),
        &NbtSummaryValue::Byte(1)
    );
    assert_eq!(
        brain_cooldown_from(
            &record(&restored, goat),
            MemoryModuleType::LongJumpCooldownTicks
        ),
        Some(remaining)
    );
    prepare(&sender, &receiver, &mut restored);
    jump_platforms(&sender, &receiver, &mut restored);
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    restored.tick_rate_mut().set_frozen(false);
    for _ in 0..10 {
        restored.tick(receiver.drain());
    }
    restored.tick_rate_mut().set_frozen(true);
    let resumed = brain_cooldown_from(
        &record(&restored, goat),
        MemoryModuleType::LongJumpCooldownTicks,
    )
    .unwrap();
    assert!(resumed > 0 && resumed < remaining);
    assert_eq!(
        restored.stored_passengers_for_test(DIMENSION, goat),
        &[rider]
    );
    let goat_position = restored
        .stored_entity_for_test(DIMENSION, goat)
        .unwrap()
        .position();
    let rider_position = restored
        .stored_entity_for_test(DIMENSION, rider)
        .unwrap()
        .position();
    for (offset, expected) in [
        rider_position.x - goat_position.x,
        rider_position.y - goat_position.y,
        rider_position.z - goat_position.z,
    ]
    .into_iter()
    .zip(standing_attachment.map(f64::from))
    {
        assert!((offset - expected).abs() < 1.0e-5);
    }
}
