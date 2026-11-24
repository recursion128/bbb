//! Player-attributed vibrations and saved anger reaching the warden's combat.

use super::horse_interaction::record;
use super::*;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_DIGGING_ID, VANILLA_ENTITY_POSE_EMERGING_ID,
    VANILLA_ENTITY_POSE_ROARING_ID, VANILLA_ENTITY_POSE_SNIFFING_ID,
    VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_POSE_DATA_ID, VANILLA_WARDEN_ANGER_LEVEL_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID;
use bbb_protocol::packets::{EntityDataValueKind, SoundEventHolder};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const ALICE: ConnectionId = ConnectionId(1);
const PLAYER: Vec3d = Vec3d {
    x: 8.5,
    y: 65.0,
    z: 8.5,
};
const WARDEN: Vec3d = Vec3d {
    x: 3.5,
    y: 65.0,
    z: 8.5,
};

fn config() -> ServerWorldConfig {
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    config.difficulty = Difficulty::Normal;
    config.keep_alive_interval_ticks = 10_000;
    config
}

fn prepare(sender: &WorldCommandSender, receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    world.tick_rate_mut().set_frozen(true);
    join_loaded(sender, receiver, world, 1, "alice", 1);
    for x in 0..16 {
        for z in 0..16 {
            world.set_block(
                DIM,
                BlockPos { x, y: 64, z },
                block_state_id("minecraft:stone"),
                SET_BLOCK_UPDATE_ALL,
            );
        }
    }
    for (id, command) in [
        (100, "gamemode survival alice"),
        (101, "gamerule spawn_mobs false"),
    ] {
        let result = console_command(sender, receiver, world, id, command);
        assert!(console_command_answer(&result).0.is_some(), "{command}");
    }
    world.set_player_position_for_test(ALICE, PLAYER);
}

fn fixture() -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config());
    prepare(&sender, &receiver, &mut world);
    let (warden, uuid) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
        WARDEN,
        crate::entity::EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_WARDEN_ID).unwrap(),
        ),
    );
    accepted_command(&sender, &receiver, &mut world,
        &format!("data merge entity {uuid} {{Brain:{{memories:{{\"minecraft:dig_cooldown\":{{value:{{}},ttl:1200L}}}}}}}}"));
    (sender, receiver, world, warden, uuid)
}

fn roaring(tick: &TickResult, warden: i32) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
            if data.id == warden && data.values.iter().any(|value|
                value.data_id == VANILLA_ENTITY_POSE_DATA_ID
                && value.value == EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_ROARING_ID))
        )
    })
}

fn roar_sound(tick: &TickResult, warden: i32) -> bool {
    tick.events.iter().any(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SoundEntity(sound),
            ..
        } if sound.entity_id == warden => {
            matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(registry_id)
                    == Some("minecraft:entity.warden.roar"))
        }
        _ => false,
    })
}

fn fight(
    world: &mut ServerWorld,
    receiver: &WorldCommandReceiver,
    warden: i32,
    mut saw_roar: bool,
    mut heard_roar: bool,
) {
    saw_roar |= world.stored_entity_for_test(DIM, warden).unwrap().pose_id()
        == VANILLA_ENTITY_POSE_ROARING_ID;
    let before = world.player_health_for_test(ALICE);
    let mut approached = false;
    for _ in 0..400 {
        let tick = world.tick(receiver.drain());
        let entity = world.stored_entity_for_test(DIM, warden).unwrap();
        saw_roar |= roaring(&tick, warden) || entity.pose_id() == VANILLA_ENTITY_POSE_ROARING_ID;
        heard_roar |= roar_sound(&tick, warden);
        let position = entity.position();
        approached |= position.x > WARDEN.x + 1.0;
        if world.player_health_for_test(ALICE) < before {
            assert!(tick.events.iter().any(|event| {
                let (sound, volume, pitch) = match event {
                    WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. } =>
                        (&sound.sound, sound.volume, sound.pitch),
                    WorldEvent::Send { packet: ClientboundPlayPacket::SoundEntity(sound), .. } =>
                        (&sound.sound, sound.volume, sound.pitch),
                    _ => return false,
                };
                matches!(sound, SoundEventHolder::Reference { registry_id }
                    if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(*registry_id)
                        == Some("minecraft:entity.warden.attack_impact"))
                    && volume == 10.0 && (0.0..2.0).contains(&pitch)
            }), "the melee hit publishes its impact sound at volume 10 and adult pitch");
            assert!(
                saw_roar && heard_roar,
                "pose={saw_roar}, sound={heard_roar}"
            );
            assert!(
                approached,
                "the warden walks into melee range: {position:?}"
            );
            return;
        }
    }
    panic!(
        "the angry warden never reached and hurt the player; pose={saw_roar}, sound={heard_roar}"
    );
}

#[test]
fn player_vibrations_build_anger_then_roar_and_reach_a_melee_hit() {
    let (sender, receiver, mut world, warden, uuid) = fixture();
    let speed = world
        .stored_entity_for_test(DIM, warden)
        .unwrap()
        .living()
        .unwrap()
        .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:movement_speed base set 0"),
    );
    prevent_new_sniffs(&sender, &receiver, &mut world, uuid);
    world.tick_rate_mut().set_frozen(false);
    world.tick(receiver.drain());
    let boat = world
        .spawn_stored_entity_for_test(
            DIM,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            PLAYER,
            crate::entity::EntityData::Boat(crate::entity::BoatData::new()),
        )
        .0;
    for _ in 0..3 {
        world.record_game_event_for_test(
            DIM,
            VanillaGameEvent::BLOCK_PLACE,
            BlockPos { x: 8, y: 65, z: 8 },
            Some(boat),
            None,
        );
        for _ in 0..60 {
            let tick = world.tick(receiver.drain());
            assert!(
                !roaring(&tick, warden),
                "a boat cannot become a combat target"
            );
            assert_eq!(world.mob_target_for_test(DIM, warden), None);
            assert_eq!(world.player_health_for_test(ALICE), 20.0);
        }
    }
    assert!(world.remove_stored_entity_for_test(
        DIM,
        boat,
        crate::entity::RemovalReason::Discarded
    ));
    let player = world.player_entity_id_for_test(ALICE);
    world.record_game_event_for_test(
        DIM,
        VanillaGameEvent::BLOCK_PLACE,
        BlockPos { x: 8, y: 65, z: 8 },
        Some(player),
        None,
    );
    world.tick(receiver.drain());
    let result = console_command(
        &sender,
        &receiver,
        &mut world,
        103,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    assert_eq!(console_command_answer(&result).0, Some(1));
    for _ in 0..20 {
        world.tick(receiver.drain());
    }
    let result = console_command(
        &sender,
        &receiver,
        &mut world,
        104,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    assert_eq!(console_command_answer(&result).0, Some(1));
    for _ in 0..60 {
        world.tick(receiver.drain());
    }
    let mut saw_roar = false;
    let mut heard_roar = false;
    let mut published_angry = false;
    world.tick_rate_mut().set_frozen(true);
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:scale base set 2"),
    );
    let (passenger, _) = mount_small_stand(&sender, &receiver, &mut world, uuid);
    world.tick_rate_mut().set_frozen(false);
    for vibration in 0..3 {
        world.record_game_event_for_test(
            DIM,
            VanillaGameEvent::BLOCK_PLACE,
            BlockPos { x: 8, y: 65, z: 8 },
            Some(player),
            None,
        );
        for _ in 0..60 {
            let tick = world.tick(receiver.drain());
            saw_roar |= roaring(&tick, warden);
            heard_roar |= roar_sound(&tick, warden);
            if world.stored_entity_for_test(DIM, warden).unwrap().pose_id()
                == VANILLA_ENTITY_POSE_ROARING_ID
            {
                assert_normal_pose_passenger(&world, warden, passenger, 2.0);
            }
            published_angry |= tick.events.iter().any(|event| {
                matches!(event,
                    WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
                    if data.id == warden && data.values.iter().any(|value|
                        value.data_id == VANILLA_WARDEN_ANGER_LEVEL_DATA_ID
                        && matches!(value.value, EntityDataValueKind::Int(anger) if anger >= 80))
                )
            });
            if vibration < 2 {
                assert_eq!(world.player_health_for_test(ALICE), 20.0);
                assert!(!saw_roar, "sub-threshold vibrations do not trigger a roar");
                assert_eq!(world.mob_target_for_test(DIM, warden), None);
            }
        }
    }
    assert!(saw_roar, "player anger publishes the roaring metadata");
    assert!(published_angry, "player anger publishes the angry metadata");
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:scale base set 1"),
    );
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:movement_speed base set {speed}"),
    );
    fight(&mut world, &receiver, warden, saw_roar, heard_roar);
}

#[test]
fn official_saved_suspect_survives_a_restart_and_drives_combat() {
    let (sender, receiver, mut world, warden, uuid) = fixture();
    let result = console_command(
        &sender,
        &receiver,
        &mut world,
        102,
        &format!(
            "data merge entity {uuid} {{anger:{{suspects:[{{uuid:[I;0,0,0,1],anger:100}}]}},Brain:{{memories:{{\"minecraft:vibration_cooldown\":{{value:{{}},ttl:300L}},\"minecraft:recent_projectile\":{{value:{{}}}}}}}}}}"
        ),
    );
    assert_eq!(console_command_answer(&result).0, Some(1));
    for (name, value) in [
        ("minecraft:sonic_boom_cooldown", "{value:{},ttl:300L}"),
        ("minecraft:sonic_boom_sound_delay", "{value:{},ttl:350L}"),
        ("minecraft:sonic_boom_sound_cooldown", "{value:{}}"),
        ("minecraft:is_sniffing", "{value:{},ttl:300L}"),
        ("minecraft:sniff_cooldown", "{value:{}}"),
    ] {
        accepted_command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{Brain:{{memories:{{\"{name}\":{value}}}}}}}"),
        );
    }
    assert_saved_suspect(&record(&world, warden));
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    assert_saved_suspect(&saved);
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let warden = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    assert_saved_suspect(&record(&restored, warden));
    prepare(&sender, &receiver, &mut restored);
    restored.tick_rate_mut().set_frozen(false);
    fight(&mut restored, &receiver, warden, false, false);
    let saved = record(&restored, warden);
    let memories = saved_memories(&saved);
    if let Some(cooldown) = memories
        .iter()
        .find(|entry| entry.name == "minecraft:vibration_cooldown")
    {
        let NbtSummaryValue::Compound(fields) = &cooldown.value else {
            panic!("vibration cooldown is a memory");
        };
        assert!(
            matches!(field(fields, "ttl"), NbtSummaryValue::Long(ttl) if (1..300).contains(ttl)),
            "the saved cooldown advances during combat"
        );
    }
    let NbtSummaryValue::Compound(recent) = field(memories, "minecraft:recent_projectile") else {
        panic!("recent projectile is a memory");
    };
    assert_eq!(
        field(recent, "value"),
        &NbtSummaryValue::Compound(Vec::new())
    );
    assert!(recent.iter().all(|entry| entry.name != "ttl"));
    for (name, initial) in [
        ("minecraft:sonic_boom_cooldown", 300),
        ("minecraft:sonic_boom_sound_delay", 350),
        ("minecraft:is_sniffing", 300),
    ] {
        if let Some(memory) = memories.iter().find(|entry| entry.name == name) {
            let NbtSummaryValue::Compound(fields) = &memory.value else {
                panic!("sonic memory compound")
            };
            assert!(
                matches!(field(fields, "ttl"), NbtSummaryValue::Long(ttl) if (1..initial).contains(ttl)),
                "sonic cooldown advances during combat"
            );
        }
    }
    assert_unit_memory(memories, "minecraft:sonic_boom_sound_cooldown", None);
    assert_unit_memory(memories, "minecraft:sniff_cooldown", None);
}

fn assert_saved_suspect(record: &[NbtSummaryEntry]) {
    let NbtSummaryValue::Compound(anger) = field(record, "anger") else {
        panic!("anger is a compound");
    };
    let NbtSummaryValue::List(suspects) = field(anger, "suspects") else {
        panic!("suspects is a list");
    };
    let [NbtSummaryValue::Compound(suspect)] = suspects.as_slice() else {
        panic!("one saved suspect: {suspects:?}");
    };
    assert_eq!(field(suspect, "uuid"), &nbt_uuid(Uuid::from_u128(1)));
    assert_eq!(field(suspect, "anger"), &NbtSummaryValue::Int(100));
    let memories = saved_memories(record);
    let NbtSummaryValue::Compound(cooldown) = field(memories, "minecraft:vibration_cooldown")
    else {
        panic!("vibration cooldown is a memory");
    };
    assert_eq!(
        field(cooldown, "value"),
        &NbtSummaryValue::Compound(Vec::new())
    );
    assert_eq!(field(cooldown, "ttl"), &NbtSummaryValue::Long(300));
    let NbtSummaryValue::Compound(recent) = field(memories, "minecraft:recent_projectile") else {
        panic!("recent projectile is a memory");
    };
    assert_eq!(
        field(recent, "value"),
        &NbtSummaryValue::Compound(Vec::new())
    );
    assert!(recent.iter().all(|entry| entry.name != "ttl"));
    assert_unit_memory(memories, "minecraft:sonic_boom_cooldown", Some(300));
    assert_unit_memory(memories, "minecraft:sonic_boom_sound_delay", Some(350));
    assert_unit_memory(memories, "minecraft:sonic_boom_sound_cooldown", None);
    assert_unit_memory(memories, "minecraft:is_sniffing", Some(300));
    assert_unit_memory(memories, "minecraft:sniff_cooldown", None);
}

fn assert_unit_memory(memories: &[NbtSummaryEntry], name: &str, ttl: Option<i64>) {
    let NbtSummaryValue::Compound(memory) = field(memories, name) else {
        panic!("unit memory compound")
    };
    assert_eq!(
        field(memory, "value"),
        &NbtSummaryValue::Compound(Vec::new())
    );
    match ttl {
        Some(ttl) => assert_eq!(field(memory, "ttl"), &NbtSummaryValue::Long(ttl)),
        None => assert!(memory.iter().all(|entry| entry.name != "ttl")),
    }
}

fn saved_memories(record: &[NbtSummaryEntry]) -> &[NbtSummaryEntry] {
    let NbtSummaryValue::Compound(brain) = field(record, "Brain") else {
        panic!("Brain is a compound");
    };
    let NbtSummaryValue::Compound(memories) = field(brain, "memories") else {
        panic!("memories is a compound");
    };
    memories
}

fn accepted_command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    command: &str,
) -> TickResult {
    let tick = console_command(sender, receiver, world, 200, command);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{command}: {:?}",
        console_command_answer(&tick)
    );
    tick
}

fn anger_at_player(world: &ServerWorld, warden: i32) -> i32 {
    let saved = record(world, warden);
    let NbtSummaryValue::Compound(anger) = field(&saved, "anger") else {
        panic!("anger compound")
    };
    let NbtSummaryValue::List(suspects) = field(anger, "suspects") else {
        panic!("suspects list")
    };
    suspects
        .iter()
        .find_map(|suspect| {
            let NbtSummaryValue::Compound(fields) = suspect else {
                return None;
            };
            if field(fields, "uuid") != &nbt_uuid(Uuid::from_u128(1)) {
                return None;
            }
            match field(fields, "anger") {
                NbtSummaryValue::Int(anger) => Some(*anger),
                _ => None,
            }
        })
        .unwrap_or(0)
}

#[test]
fn a_direct_player_hit_starts_pursuit_and_melee_without_roaring() {
    let (sender, receiver, mut world, warden, uuid) = fixture();
    world.tick_rate_mut().set_frozen(false);
    world.tick(receiver.drain());
    let hit = accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {uuid} 1 minecraft:mob_attack by alice"),
    );
    assert!(!roaring(&hit, warden) && !roar_sound(&hit, warden));
    assert!(anger_at_player(&world, warden) >= 99);
    let before = world.player_health_for_test(ALICE);
    let mut approached = false;
    for _ in 0..200 {
        let tick = world.tick(receiver.drain());
        let entity = world.stored_entity_for_test(DIM, warden).unwrap();
        assert_ne!(entity.pose_id(), VANILLA_ENTITY_POSE_ROARING_ID);
        assert!(!roaring(&tick, warden) && !roar_sound(&tick, warden));
        approached |= entity.position().x > WARDEN.x + 1.0;
        if world.player_health_for_test(ALICE) < before {
            assert!(
                approached,
                "the hurt warden pursues the attacker before its melee hit"
            );
            return;
        }
    }
    panic!("the directly attacked warden never reached its attacker");
}

#[test]
fn repeated_real_contact_builds_anger_and_selects_the_player() {
    let (_sender, receiver, mut world, warden, _) = fixture();
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..100 {
        let at = world
            .stored_entity_for_test(DIM, warden)
            .unwrap()
            .position();
        world.set_player_position_for_test(
            ALICE,
            Vec3d {
                x: at.x + 0.5,
                ..at
            },
        );
        world.tick(receiver.drain());
        if world.mob_target_for_test(DIM, warden) == Some(Uuid::from_u128(1)) {
            assert!(anger_at_player(&world, warden) >= 80);
            let saved = record(&world, warden);
            let NbtSummaryValue::Compound(touch) =
                field(saved_memories(&saved), "minecraft:touch_cooldown")
            else {
                panic!("contact writes the touch cooldown memory");
            };
            assert!(
                matches!(field(touch, "ttl"), NbtSummaryValue::Long(ttl) if (1..=20).contains(ttl))
            );
            return;
        }
    }
    panic!("real player contact never made the warden angry");
}

fn darkness_updates(tick: &TickResult, connection: ConnectionId) -> bool {
    let effect = bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(
        "minecraft:darkness",
    )
    .unwrap()
    .id;
    tick.events.iter().any(|event| match event {
        WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::UpdateMobEffect(update),
        } if *connection_id == connection && update.effect_id == effect => {
            assert_eq!(update.duration_ticks, 260);
            assert_eq!(update.amplifier, 0);
            assert!(!update.flags.visible);
            true
        }
        _ => false,
    })
}

#[test]
fn a_warden_darkens_nearby_non_allied_survival_and_adventure_players() {
    let (sender, receiver, mut world, _warden, uuid) = fixture();
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:movement_speed base set 0"),
    );
    prevent_new_sniffs(&sender, &receiver, &mut world, uuid);
    for (connection, name) in [(2, "bob"), (3, "carol"), (4, "dave")] {
        join_loaded(
            &sender,
            &receiver,
            &mut world,
            connection,
            name,
            u128::from(connection),
        );
    }
    for command in [
        "gamemode adventure bob",
        "gamemode survival carol",
        "gamemode survival dave",
        "team add wardens",
        "team join wardens dave",
        &format!("team join wardens {uuid}"),
    ] {
        accepted_command(&sender, &receiver, &mut world, command);
    }
    for (connection, x) in [(2, 10.5), (3, WARDEN.x + 20.0), (4, 12.5)] {
        world.set_player_position_for_test(ConnectionId(connection), Vec3d { x, ..PLAYER });
    }
    for x in 16..=24 {
        world.set_block(
            DIM,
            BlockPos { x, y: 64, z: 8 },
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL,
        );
    }
    world.tick_rate_mut().set_frozen(false);
    let mut received = [false; 2];
    for _ in 0..130 {
        let tick = world.tick(receiver.drain());
        for (index, connection) in [ALICE, ConnectionId(2)].into_iter().enumerate() {
            received[index] |= darkness_updates(&tick, connection);
        }
    }
    assert!(received.into_iter().all(|received| received));
    for connection in [ALICE, ConnectionId(2)] {
        let effect = world
            .player_active_effect_for_test(connection, "minecraft:darkness")
            .unwrap();
        assert_eq!(effect.amplifier(), 0);
        assert!(!effect.show_particles());
    }
    for connection in [ConnectionId(3), ConnectionId(4)] {
        assert!(world
            .player_active_effect_for_test(connection, "minecraft:darkness")
            .is_none());
    }
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    for player in ["alice", "bob"] {
        accepted_command(
            &sender,
            &receiver,
            &mut world,
            &format!("effect clear {player} minecraft:darkness"),
        );
    }
    for _ in 0..140 {
        world.tick(receiver.drain());
    }
    for connection in [ALICE, ConnectionId(2)] {
        assert!(
            world
                .player_active_effect_for_test(connection, "minecraft:darkness")
                .is_none(),
            "NoAI stops the aura"
        );
    }
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        "effect give alice minecraft:darkness infinite 0 true",
    );
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        "effect give bob minecraft:darkness 20 0 true",
    );
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    for _ in 0..130 {
        let tick = world.tick(receiver.drain());
        assert!(
            !darkness_updates(&tick, ALICE),
            "permanent darkness is preserved"
        );
        assert!(
            !darkness_updates(&tick, ConnectionId(2)),
            "long finite darkness is not refreshed early"
        );
    }
    assert_eq!(
        world
            .player_active_effect_for_test(ALICE, "minecraft:darkness")
            .unwrap()
            .duration(),
        -1
    );
    let mut refreshed = false;
    for _ in 0..240 {
        let tick = world.tick(receiver.drain());
        refreshed |= darkness_updates(&tick, ConnectionId(2));
        assert!(!darkness_updates(&tick, ALICE));
    }
    assert!(
        refreshed,
        "finite darkness is refreshed after it becomes short"
    );
}

fn sonic_sound(tick: &TickResult, name: &str) -> bool {
    tick.events.iter().any(|event| {
        let (sound, volume, pitch) = match event {
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. } => (&sound.sound, sound.volume, sound.pitch),
            WorldEvent::Send { packet: ClientboundPlayPacket::SoundEntity(sound), .. } => (&sound.sound, sound.volume, sound.pitch),
            _ => return false,
        };
        if !matches!(sound, SoundEventHolder::Reference { registry_id }
            if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(*registry_id) == Some(name)) {
            return false;
        }
        assert_eq!((volume, pitch), (3.0, 1.0));
        true
    })
}

#[test]
fn sonic_boom_crosses_stone_and_hurts_an_armoured_shield_user() {
    let (sender, receiver, mut world, warden, uuid) = fixture();
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        "gamerule natural_health_regeneration false",
    );
    for x in 2..=4 {
        for z in 7..=9 {
            for y in 65..=69 {
                if x == 3 && z == 8 && y < 69 {
                    continue;
                }
                world.set_block(
                    DIM,
                    BlockPos { x, y, z },
                    block_state_id("minecraft:stone"),
                    SET_BLOCK_UPDATE_ALL,
                );
            }
        }
    }
    for (slot, item) in [
        ("armor.head", "diamond_helmet"),
        ("armor.chest", "diamond_chestplate"),
        ("armor.legs", "diamond_leggings"),
        ("armor.feet", "diamond_boots"),
        ("weapon.offhand", "shield"),
    ] {
        accepted_command(
            &sender,
            &receiver,
            &mut world,
            &format!("item replace entity alice {slot} with minecraft:{item}"),
        );
    }
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItem(bbb_protocol::packets::UseItem {
            hand: bbb_protocol::packets::InteractionHand::OffHand,
            sequence: 1,
            y_rot: 90.0,
            x_rot: 0.0,
        }),
    );
    assert!(world.player_use_state_for_test(ALICE).0);
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {uuid} 1 minecraft:mob_attack by alice"),
    );
    world.tick_rate_mut().set_frozen(false);
    let mut charged = false;
    let mut charge_sound = false;
    let mut boom_sound = false;
    let mut beam = false;
    let player = world.player_entity_id_for_test(ALICE);
    let sonic = world.damage_type_registry_id_for_test("minecraft:sonic_boom");
    let motion = |tick: &TickResult| {
        tick.events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ALICE,
                    packet: ClientboundPlayPacket::SetEntityMotion(update),
                } if update.id == player && update.delta_movement.x > 2.0 => {
                    Some(update.delta_movement)
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    for _ in 0..600 {
        let tick = world.tick(receiver.drain());
        charged |= tick.events.iter().any(|event| {
            matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(event), .. }
                if event.entity_id == warden && event.event_id == 62)
        });
        charge_sound |= sonic_sound(&tick, "minecraft:entity.warden.sonic_charge");
        boom_sound |= sonic_sound(&tick, "minecraft:entity.warden.sonic_boom");
        if let Some(first) = tick.events.iter().find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::LevelParticles(particles),
                ..
            } if particles.particle.particle_type_id
                == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_SONIC_BOOM_ID =>
            {
                Some(particles.position)
            }
            _ => None,
        }) {
            let origin = world
                .stored_entity_for_test(DIM, warden)
                .unwrap()
                .position();
            assert!((origin.y + 1.5..origin.y + 1.7).contains(&first.y)
                && (origin.x..origin.x + 2.0).contains(&first.x)
                && (first.z - origin.z).abs() < 0.01,
                "the first sonic particle travels from the chest towards the player's eye: {first:?}");
            beam = true;
        }
        if world.player_health_for_test(ALICE) < 20.0 {
            assert_eq!(
                world.player_health_for_test(ALICE),
                10.0,
                "sonic damage bypasses diamond armour and the raised shield on Normal"
            );
            assert!(charged && charge_sound && boom_sound && beam);
            assert!(tick.events.iter().any(|event| matches!(event,
                WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::DamageEvent(damage) }
                    if damage.entity_id == player && damage.source_type_id == sonic)));
            assert!(world.player_use_state_for_test(ALICE).0);
            let mut impulses = motion(&tick);
            impulses.extend(motion(&world.tick(receiver.drain())));
            assert!(
                impulses
                    .iter()
                    .any(|impulse| impulse.y.abs() < 0.5 && impulse.z.abs() < 0.1),
                "the hit sends the sonic impulse: {impulses:?}"
            );
            assert!(
                world
                    .stored_entity_for_test(DIM, warden)
                    .unwrap()
                    .position()
                    .x
                    < 4.0,
                "stone keeps the warden out of melee range"
            );
            world.tick_rate_mut().set_frozen(true);
            world.set_player_position_for_test(ALICE, PLAYER);
            accepted_command(&sender, &receiver, &mut world,
                &format!("data remove entity {uuid} Brain.memories.\"minecraft:sonic_boom_sound_cooldown\""));
            accepted_command(&sender, &receiver, &mut world,
                &format!("data merge entity {uuid} {{Brain:{{memories:{{\"minecraft:sonic_boom_sound_cooldown\":{{value:{{}}}}}}}}}}"));
            world.tick_rate_mut().set_frozen(false);
            let mut recharged = false;
            for _ in 0..240 {
                let tick = world.tick(receiver.drain());
                recharged |= tick.events.iter().any(|event| {
                    matches!(event,
                    WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(event), .. }
                        if event.entity_id == warden && event.event_id == 62)
                });
                assert_eq!(
                    world.player_health_for_test(ALICE),
                    10.0,
                    "permanent sound cooldown suppresses the next sonic hit"
                );
            }
            assert!(
                recharged,
                "the warden charges again with the saved permanent memory"
            );
            let saved = record(&world, warden);
            assert_unit_memory(
                saved_memories(&saved),
                "minecraft:sonic_boom_sound_cooldown",
                None,
            );
            return;
        }
    }
    panic!("the warden never fired its sonic boom through the wall");
}

fn prevent_new_sniffs(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    uuid: Uuid,
) {
    accepted_command(sender, receiver, world,
        &format!("data merge entity {uuid} {{Brain:{{memories:{{\"minecraft:sniff_cooldown\":{{value:{{}}}}}}}}}}"));
}

#[test]
fn an_unattributed_vibration_makes_the_warden_investigate_its_position() {
    let (sender, receiver, mut world, warden, uuid) = fixture();
    prevent_new_sniffs(&sender, &receiver, &mut world, uuid);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 12.5,
            z: 12.5,
            ..PLAYER
        },
    );
    world.record_game_event_for_test(
        DIM,
        VanillaGameEvent::BLOCK_PLACE,
        BlockPos { x: 10, y: 65, z: 8 },
        None,
        None,
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..200 {
        world.tick(receiver.drain());
        let position = world
            .stored_entity_for_test(DIM, warden)
            .unwrap()
            .position();
        if (position.x - 10.5).hypot(position.z - 8.5) < 3.0 {
            assert!(
                position.x > WARDEN.x + 3.0,
                "the warden walks towards the heard disturbance"
            );
            assert_eq!(anger_at_player(&world, warden), 0);
            assert_eq!(world.mob_target_for_test(DIM, warden), None);
            return;
        }
    }
    panic!("the warden never investigated the unattributed vibration");
}

fn sniff_sound(tick: &TickResult) -> bool {
    tick.events.iter().any(|event| {
        let (sound, volume, pitch) = match event {
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. } => (&sound.sound, sound.volume, sound.pitch),
            WorldEvent::Send { packet: ClientboundPlayPacket::SoundEntity(sound), .. } => (&sound.sound, sound.volume, sound.pitch),
            _ => return false,
        };
        if !matches!(sound, SoundEventHolder::Reference { registry_id }
            if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(*registry_id) == Some("minecraft:entity.warden.sniff")) {
            return false;
        }
        assert_eq!((volume, pitch), (5.0, 1.0));
        true
    })
}

#[test]
fn a_saved_sniff_finds_a_player_through_stone_before_a_nearer_mob() {
    let (sender, receiver, mut world, warden, uuid) = fixture();
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:movement_speed base set 0"),
    );
    let (_cow, cow_uuid) = world.spawn_stored_entity_for_test(
        DIM,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d { z: 11.5, ..WARDEN },
        crate::entity::EntityData::GeneralLiving(
            GeneralLivingData::new(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID).unwrap(),
        ),
    );
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {cow_uuid} {{NoAI:1b}}"),
    );
    for z in 0..16 {
        for y in 65..=69 {
            world.set_block(
                DIM,
                BlockPos { x: 5, y, z },
                block_state_id("minecraft:stone"),
                SET_BLOCK_UPDATE_ALL,
            );
        }
    }
    world.tick_rate_mut().set_frozen(false);
    let mut heard_sniff = false;
    let mut posed = false;
    for _ in 0..200 {
        let tick = world.tick(receiver.drain());
        heard_sniff |= sniff_sound(&tick);
        posed |= world.stored_entity_for_test(DIM, warden).unwrap().pose_id()
            == VANILLA_ENTITY_POSE_SNIFFING_ID;
        if heard_sniff && posed {
            break;
        }
    }
    assert!(
        heard_sniff && posed,
        "the warden publishes the sniff pose and sound"
    );
    world.tick_rate_mut().set_frozen(true);
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:scale base set 2"),
    );
    let (passenger, passenger_uuid) = mount_small_stand(&sender, &receiver, &mut world, uuid);
    world.tick_rate_mut().set_frozen(false);
    world.tick(receiver.drain());
    world.tick_rate_mut().set_frozen(true);
    assert_eq!(
        world.stored_entity_for_test(DIM, warden).unwrap().pose_id(),
        VANILLA_ENTITY_POSE_SNIFFING_ID
    );
    assert_normal_pose_passenger(&world, warden, passenger, 2.0);
    let before = record(&world, warden);
    let sniff = field(saved_memories(&before), "minecraft:is_sniffing").clone();
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let warden = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    let passenger = restored
        .live_entity_id_for_uuid_for_test(DIM, passenger_uuid)
        .unwrap();
    assert_normal_pose_passenger(&restored, warden, passenger, 2.0);
    assert_eq!(
        field(
            saved_memories(&record(&restored, warden)),
            "minecraft:is_sniffing"
        ),
        &sniff
    );
    prepare(&sender, &receiver, &mut restored);
    assert_normal_pose_passenger(&restored, warden, passenger, 2.0);
    for z in 0..16 {
        for y in 65..=69 {
            restored.set_block(
                DIM,
                BlockPos { x: 5, y, z },
                block_state_id("minecraft:stone"),
                SET_BLOCK_UPDATE_ALL,
            );
        }
    }
    restored.tick_rate_mut().set_frozen(false);
    let mut restored_sniff = false;
    for _ in 0..200 {
        restored.tick(receiver.drain());
        restored_sniff |= restored
            .stored_entity_for_test(DIM, warden)
            .unwrap()
            .pose_id()
            == VANILLA_ENTITY_POSE_SNIFFING_ID;
        assert_normal_pose_passenger(&restored, warden, passenger, 2.0);
        if anger_at_player(&restored, warden) > 0 {
            break;
        }
    }
    assert!(restored_sniff, "the saved sniff resumes after rejoining");
    assert!(
        anger_at_player(&restored, warden) >= 30,
        "the restored sniff finds the legal player through stone"
    );
    assert!(
        restored
            .live_entity_id_for_uuid_for_test(DIM, cow_uuid)
            .is_some(),
        "the nearer cow reloads"
    );
    let saved = record(&restored, warden);
    let NbtSummaryValue::Compound(anger) = field(&saved, "anger") else {
        panic!("anger compound")
    };
    let NbtSummaryValue::List(suspects) = field(anger, "suspects") else {
        panic!("suspects list")
    };
    assert!(
        suspects.iter().all(|suspect| match suspect {
            NbtSummaryValue::Compound(fields) => field(fields, "uuid") != &nbt_uuid(cow_uuid),
            _ => false,
        }),
        "the player is preferred to the closer cow"
    );
    restored.tick_rate_mut().set_frozen(true);
    accepted_command(&sender, &receiver, &mut restored,
        &format!("data modify entity {uuid} Brain.memories.\"minecraft:sniff_cooldown\" set value {{value:{{}}}}"));
    restored.tick_rate_mut().set_frozen(false);
    for _ in 0..300 {
        let tick = restored.tick(receiver.drain());
        assert!(!sniff_sound(&tick));
        assert_ne!(
            restored
                .stored_entity_for_test(DIM, warden)
                .unwrap()
                .pose_id(),
            VANILLA_ENTITY_POSE_SNIFFING_ID
        );
    }
    assert!(
        anger_at_player(&restored, warden) <= 35,
        "permanent cooldown prevents another sniff"
    );
    assert_unit_memory(
        saved_memories(&record(&restored, warden)),
        "minecraft:sniff_cooldown",
        None,
    );
}

fn lifecycle_sound(tick: &TickResult, name: &str) -> bool {
    tick.events.iter().any(|event| {
        let (sound, volume, pitch) = match event {
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. } => (&sound.sound, sound.volume, sound.pitch),
            WorldEvent::Send { packet: ClientboundPlayPacket::SoundEntity(sound), .. } => (&sound.sound, sound.volume, sound.pitch),
            _ => return false,
        };
        if !matches!(sound, SoundEventHolder::Reference { registry_id }
            if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(*registry_id) == Some(name)) { return false; }
        assert_eq!((volume, pitch), (5.0, 1.0));
        true
    })
}

fn mount_small_stand(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    warden_uuid: Uuid,
) -> (i32, Uuid) {
    accepted_command(
        sender,
        receiver,
        world,
        "summon minecraft:armor_stand 3.5 66 8.5 {Small:1b,NoGravity:1b,Invulnerable:1b}",
    );
    let passenger = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
        })
        .unwrap();
    let uuid = world.stored_entity_for_test(DIM, passenger).unwrap().uuid();
    accepted_command(
        sender,
        receiver,
        world,
        &format!("ride {uuid} mount {warden_uuid}"),
    );
    (passenger, uuid)
}

fn assert_normal_pose_passenger(world: &ServerWorld, warden: i32, passenger: i32, scale: f32) {
    assert_eq!(world.stored_passengers_for_test(DIM, warden), &[passenger]);
    let vehicle = world.stored_entity_for_test(DIM, warden).unwrap();
    assert!(matches!(
        vehicle.pose_id(),
        VANILLA_ENTITY_POSE_STANDING_ID
            | VANILLA_ENTITY_POSE_ROARING_ID
            | VANILLA_ENTITY_POSE_SNIFFING_ID
    ));
    let attachment = bbb_protocol::entity_types::vanilla_entity_type_passenger_attachments(
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    )
    .unwrap()[0];
    let origin = vehicle.position();
    let rider = world
        .stored_entity_for_test(DIM, passenger)
        .unwrap()
        .position();
    assert!((rider.x - origin.x).abs() < 1e-6);
    assert!((rider.y - origin.y - f64::from(attachment[1] * scale)).abs() < 1e-6);
    assert!((rider.z - origin.z).abs() < 1e-6);
}

fn assert_fixed_pose_passenger(world: &ServerWorld, warden: i32, passenger: i32) {
    assert_eq!(world.stored_passengers_for_test(DIM, warden), &[passenger]);
    let vehicle = world.stored_entity_for_test(DIM, warden).unwrap();
    assert!(matches!(
        vehicle.pose_id(),
        VANILLA_ENTITY_POSE_EMERGING_ID | VANILLA_ENTITY_POSE_DIGGING_ID
    ));
    assert!((f64::from(vehicle.bb_height()) - 1.0).abs() < 1e-6);
    assert!((f64::from(vehicle.eye_height()) - 0.85).abs() < 1e-6);
    let origin = vehicle.position();
    let rider = world
        .stored_entity_for_test(DIM, passenger)
        .unwrap()
        .position();
    assert!((rider.x - origin.x).abs() < 1e-6);
    assert!((rider.y - origin.y - 1.0).abs() < 1e-6);
    assert!((rider.z - origin.z).abs() < 1e-6);
}

#[test]
fn permanent_emerging_memory_reloads_with_fixed_geometry_and_damage_immunity() {
    let (sender, receiver, mut world, warden, uuid) = fixture();
    let bounds = world
        .stored_entity_for_test(DIM, warden)
        .unwrap()
        .bounding_box();
    let width = bounds.max_x - bounds.min_x;
    accepted_command(&sender, &receiver, &mut world, "gamemode creative alice");
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:scale base set 2"),
    );
    let (passenger, passenger_uuid) = mount_small_stand(&sender, &receiver, &mut world, uuid);
    accepted_command(&sender, &receiver, &mut world,
        &format!("data merge entity {uuid} {{Brain:{{memories:{{\"minecraft:is_emerging\":{{value:{{}}}},\"minecraft:dig_cooldown\":{{value:{{}},ttl:300L}}}}}}}}"));
    world.tick_rate_mut().set_frozen(false);
    let mut heard = false;
    for _ in 0..20 {
        let tick = world.tick(receiver.drain());
        heard |= lifecycle_sound(&tick, "minecraft:entity.warden.emerge");
        if world.stored_entity_for_test(DIM, warden).unwrap().pose_id()
            == VANILLA_ENTITY_POSE_EMERGING_ID
            && heard
        {
            break;
        }
    }
    assert!(heard, "the emerging warden publishes its emerge sound");
    for _ in 0..2 {
        world.tick(receiver.drain());
        assert_fixed_pose_passenger(&world, warden, passenger);
    }
    world.tick_rate_mut().set_frozen(true);
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    let saved = record(&world, warden);
    assert_unit_memory(saved_memories(&saved), "minecraft:is_emerging", None);
    let cooldown = field(saved_memories(&saved), "minecraft:dig_cooldown").clone();
    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    assert_unit_memory(saved_memories(&saved), "minecraft:is_emerging", None);
    assert_eq!(
        field(saved_memories(&saved), "minecraft:dig_cooldown"),
        &cooldown
    );
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let warden = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    let passenger = restored
        .live_entity_id_for_uuid_for_test(DIM, passenger_uuid)
        .unwrap();
    assert_eq!(
        restored.stored_passengers_for_test(DIM, warden),
        &[passenger]
    );
    assert_unit_memory(
        saved_memories(&record(&restored, warden)),
        "minecraft:is_emerging",
        None,
    );
    prepare(&sender, &receiver, &mut restored);
    accepted_command(&sender, &receiver, &mut restored, "gamemode creative alice");
    restored.tick_rate_mut().set_frozen(false);
    for _ in 0..180 {
        restored.tick(receiver.drain());
    }
    let saved = record(&restored, warden);
    assert_unit_memory(saved_memories(&saved), "minecraft:is_emerging", None);
    assert_eq!(
        field(saved_memories(&saved), "minecraft:dig_cooldown"),
        &cooldown,
        "NoAI leaves the cooldown unchanged"
    );
    accepted_command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    for _ in 0..180 {
        restored.tick(receiver.drain());
    }
    let entity = restored.stored_entity_for_test(DIM, warden).unwrap();
    assert_eq!(entity.pose_id(), VANILLA_ENTITY_POSE_EMERGING_ID);
    assert_fixed_pose_passenger(&restored, warden, passenger);
    let bounds = entity.bounding_box();
    assert!((bounds.max_y - bounds.min_y - 1.0).abs() < 1e-6);
    assert!(
        (bounds.max_x - bounds.min_x - width).abs() < 1e-6,
        "scale does not enlarge the fixed emerging shape"
    );
    assert!((f64::from(entity.eye_height()) - 0.85).abs() < 1e-6);
    let before = entity.living().unwrap().health();
    console_command(
        &sender,
        &receiver,
        &mut restored,
        210,
        &format!("damage {uuid} 5 minecraft:generic by alice"),
    );
    assert_eq!(
        restored
            .stored_entity_for_test(DIM, warden)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        before,
        "a creative attacker does not bypass emergence immunity"
    );
    accepted_command(
        &sender,
        &receiver,
        &mut restored,
        &format!("damage {uuid} 2 minecraft:generic_kill"),
    );
    assert_eq!(
        restored
            .stored_entity_for_test(DIM, warden)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        before - 2.0
    );
    assert_unit_memory(
        saved_memories(&record(&restored, warden)),
        "minecraft:is_emerging",
        None,
    );
}

#[test]
fn missing_dig_cooldown_dismounts_then_discards_without_drops_and_water_exits() {
    let (sender, receiver, mut world, warden, uuid) = fixture();
    accepted_command(&sender, &receiver, &mut world, "gamemode spectator alice");
    let width = f64::from(
        world
            .stored_entity_for_test(DIM, warden)
            .unwrap()
            .bb_width(),
    );
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:scale base set 2"),
    );
    let (passenger, passenger_uuid) = mount_small_stand(&sender, &receiver, &mut world, uuid);
    let (boat, boat_uuid) = world.spawn_stored_entity_for_test(
        DIM,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        WARDEN,
        crate::entity::EntityData::Boat(crate::entity::BoatData::new()),
    );
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("ride {uuid} mount {boat_uuid}"),
    );
    assert_eq!(world.stored_passengers_for_test(DIM, boat), vec![warden]);
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("data remove entity {uuid} Brain.memories.\"minecraft:dig_cooldown\""),
    );
    world.tick_rate_mut().set_frozen(false);
    let mut dug = false;
    let mut heard = false;
    let mut removed = false;
    let mut mounted_dig_ticks = 0;
    for _ in 0..240 {
        let tick = world.tick(receiver.drain());
        heard |= lifecycle_sound(&tick, "minecraft:entity.warden.dig");
        removed |= tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::RemoveEntities(remove), .. } if remove.entity_ids.contains(&warden)));
        if let Some(entity) = world.stored_entity_for_test(DIM, warden) {
            dug |= entity.pose_id() == VANILLA_ENTITY_POSE_DIGGING_ID;
            if entity.pose_id() == VANILLA_ENTITY_POSE_DIGGING_ID {
                assert_fixed_pose_passenger(&world, warden, passenger);
                assert!((f64::from(entity.bb_width()) - width).abs() < 1e-6);
                mounted_dig_ticks += 1;
            }
        } else {
            break;
        }
    }
    assert!(
        dug && heard && removed,
        "dig={dug}, sound={heard}, removal={removed}"
    );
    assert!(world.live_entity_id_for_uuid_for_test(DIM, uuid).is_none());
    assert!(mounted_dig_ticks >= 2);
    assert!(world
        .live_entity_id_for_uuid_for_test(DIM, passenger_uuid)
        .is_some());
    assert!(world.stored_passengers_for_test(DIM, boat).is_empty());
    assert!(
        world.stored_entity_ids_for_test(DIM).into_iter().all(|id| {
            let kind = world
                .stored_entity_for_test(DIM, id)
                .unwrap()
                .entity_type_id();
            kind != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID
                && kind != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID
        }),
        "digging discards the warden without loot or experience"
    );
    world.tick_rate_mut().set_frozen(true);
    for y in 65..=68 {
        world.set_block(
            DIM,
            BlockPos { x: 8, y, z: 8 },
            bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID,
            )
            .unwrap(),
            SET_BLOCK_UPDATE_ALL,
        );
    }
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:warden 8.5 66 8.5",
    );
    let wet = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_WARDEN_ID
        })
        .unwrap();
    let wet_uuid = world.stored_entity_for_test(DIM, wet).unwrap().uuid();
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("data remove entity {wet_uuid} Brain.memories.\"minecraft:dig_cooldown\""),
    );
    world.tick_rate_mut().set_frozen(false);
    let mut agitated = false;
    for _ in 0..20 {
        let tick = world.tick(receiver.drain());
        agitated |= lifecycle_sound(&tick, "minecraft:entity.warden.agitated");
        if world
            .live_entity_id_for_uuid_for_test(DIM, wet_uuid)
            .is_none()
        {
            break;
        }
    }
    assert!(
        agitated
            && world
                .live_entity_id_for_uuid_for_test(DIM, wet_uuid)
                .is_none(),
        "a submerged warden leaves with its agitated sound"
    );
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config());
    prepare(&sender, &receiver, &mut world);
    let airborne = Vec3d { y: 72.0, ..PLAYER };
    let (air_boat, air_boat_uuid) = world.spawn_stored_entity_for_test(
        DIM,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        airborne,
        crate::entity::EntityData::Boat(crate::entity::BoatData::new()),
    );
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {air_boat_uuid} {{NoGravity:1b}}"),
    );
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:warden 8.5 72 8.5 {NoGravity:1b}",
    );
    let waiting = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_WARDEN_ID
        })
        .unwrap();
    let waiting_uuid = world.stored_entity_for_test(DIM, waiting).unwrap().uuid();
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("ride {waiting_uuid} mount {air_boat_uuid}"),
    );
    assert!(
        saved_memories(&record(&world, waiting))
            .iter()
            .all(|memory| memory.name != "minecraft:dig_cooldown"),
        "explicit summon NBT preserves an absent dig cooldown"
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..10 {
        world.tick(receiver.drain());
        if world.stored_passengers_for_test(DIM, air_boat).is_empty() {
            break;
        }
    }
    assert!(
        world.stored_passengers_for_test(DIM, air_boat).is_empty(),
        "the airborne warden dismounts before waiting to land"
    );
    assert_ne!(
        world
            .stored_entity_for_test(DIM, waiting)
            .unwrap()
            .pose_id(),
        VANILLA_ENTITY_POSE_DIGGING_ID
    );
    let before = world.player_health_for_test(ALICE);
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {waiting_uuid} 1 minecraft:mob_attack by alice"),
    );
    world.tick_rate_mut().set_frozen(true);
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("teleport {waiting_uuid} 3.5 65 8.5"),
    );
    accepted_command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {waiting_uuid} {{NoGravity:0b}}"),
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..200 {
        world.tick(receiver.drain());
        let body = world
            .stored_entity_for_test(DIM, waiting)
            .expect("a newly attacked warden fights instead of leaving");
        assert_ne!(
            body.pose_id(),
            VANILLA_ENTITY_POSE_DIGGING_ID,
            "the hit interrupts waiting for landing"
        );
        if world.player_health_for_test(ALICE) < before {
            return;
        }
    }
    let body = world.stored_entity_for_test(DIM, waiting).unwrap();
    panic!("the attacked airborne warden never resumed combat after landing: warden={:?}, pose={}, target={:?}, player={:?}, anger={}, path={:?}",
        body.position(), body.pose_id(), world.mob_target_for_test(DIM, waiting),
        world.player_movement_validation_for_test(ALICE).0, anger_at_player(&world, waiting),
        world.ground_path_for_test(DIM, waiting));
}

#[test]
fn a_persistent_warden_never_digs_away() {
    let (sender, receiver, mut world, warden, uuid) = fixture();
    accepted_command(&sender, &receiver, &mut world, "gamemode spectator alice");
    accepted_command(&sender, &receiver, &mut world,
        &format!("data merge entity {uuid} {{PersistenceRequired:1b,Brain:{{memories:{{\"minecraft:dig_cooldown\":{{value:{{}},ttl:5L}}}}}}}}"));
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..80 {
        world.tick(receiver.drain());
        let entity = world
            .stored_entity_for_test(DIM, warden)
            .expect("the warden stays");
        assert_ne!(entity.pose_id(), VANILLA_ENTITY_POSE_DIGGING_ID);
    }
}
