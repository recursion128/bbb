use super::tests::block;
use super::*;
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::{ServerWorldConfig, WorldCommand};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{ClientCommandAction, PlayServerbound, SoundEventHolder, Vec3d};
use bbb_server_persistence::RegionCompression;
use bbb_server_persistence::{Java26PlayerData, PlayerDataSource};

const PLAYER: ConnectionId = ConnectionId(1);
const OBSERVER: ConnectionId = ConnectionId(2);

fn fixture() -> (
    ServerWorld,
    crate::mailbox::WorldCommandSender,
    crate::mailbox::WorldCommandReceiver,
) {
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let position = Vec3d {
        x: 0.5,
        y: 80.0,
        z: 0.5,
    };
    for id in [PLAYER, OBSERVER] {
        let player = world.levels.player_mut(id).unwrap();
        player.set_position(if id == OBSERVER {
            Vec3d {
                x: position.x + 2.0,
                ..position
            }
        } else {
            position
        });
        player.block_contact.old_position = player.position();
    }
    (world, sender, receiver)
}

fn sound_count(events: &[WorldEvent], name: &str) -> usize {
    let sound = SoundEventHolder::Reference {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(name)
            .unwrap(),
    };
    events
        .iter()
        .filter(|event| {
            matches!(
                event.packet_for(OBSERVER),
                Some(ClientboundPlayPacket::Sound(packet)) if packet.sound == sound
            )
        })
        .count()
}

#[test]
fn player_environment_silent_fall_keeps_player_sound_and_damage() {
    for silent in [false, true] {
        let (mut world, _, _) = fixture();
        block(
            &mut world,
            BlockPos { x: 0, y: 79, z: 0 },
            "minecraft:stone",
        );
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.set_silent(silent);
        player.block_contact.fall_distance = 7.0;
        let mut events = Vec::new();
        world.check_player_fall_state(PLAYER, Vec3d::default(), true, &mut events);
        assert_eq!(world.levels[PLAYER].damage.health, 16.0);
        assert_eq!(
            sound_count(&events, "minecraft:entity.player.small_fall"),
            1
        );
        assert_eq!(
            sound_count(&events, "minecraft:block.stone.fall"),
            usize::from(!silent)
        );
    }
}

#[test]
fn player_environment_silent_lava_keeps_hurt_and_ignition_without_pitch_draw() {
    for silent in [true, false] {
        let (mut world, _, _) = fixture();
        block(&mut world, BlockPos { x: 0, y: 80, z: 0 }, "minecraft:lava");
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.set_silent(silent);
        let mut events = Vec::new();
        world.tick_player_block_contacts(&mut events);
        let player = world.levels.player_at(PLAYER);
        assert_eq!(player.damage.health, 16.0);
        assert_eq!(player.remaining_fire_ticks, 300);
        assert_eq!(
            sound_count(&events, "minecraft:entity.generic.burn"),
            usize::from(!silent)
        );
    }
}

#[test]
fn player_environment_silent_honey_and_extinguish_remain_audible() {
    let (mut world, _, _) = fixture();
    let pos = BlockPos { x: 0, y: 80, z: 0 };
    block(&mut world, pos, "minecraft:honey_block");
    world.levels[PLAYER].set_silent(true);
    // The slide sound is a `nextInt(5) == 0` roll; 200 slides all miss with
    // odds of about 4e-20.
    let mut events = Vec::new();
    for _ in 0..200 {
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.set_position(Vec3d {
            x: -0.25,
            y: 80.2,
            z: 0.5,
        });
        player.block_contact.old_position = player.position();
        player.set_on_ground(false);
        player.set_delta_movement(Vec3d {
            x: 0.0,
            y: -0.3,
            z: 0.0,
        });
        events.clear();
        world.tick_player_block_contacts(&mut events);
        if sound_count(&events, "minecraft:block.honey_block.slide") > 0 {
            break;
        }
    }
    assert_eq!(sound_count(&events, "minecraft:block.honey_block.slide"), 1);

    block(&mut world, pos, "minecraft:water");
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.set_position(Vec3d {
        x: 0.5,
        y: 80.0,
        z: 0.5,
    });
    player.block_contact.old_position = player.position();
    player.remaining_fire_ticks = 20;
    events.clear();
    world.tick_player_block_contacts(&mut events);
    assert!(world.levels[PLAYER].remaining_fire_ticks <= 0);
    assert_eq!(
        sound_count(&events, "minecraft:entity.generic.extinguish_fire"),
        1
    );
}

#[test]
fn player_environment_silent_live_save_disconnect_and_respawn() {
    let (mut world, sender, receiver) = fixture();
    world.levels[PLAYER].set_silent(true);
    let uuid = world.levels[PLAYER].profile.uuid;
    let save = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    let patch = save
        .player_patches
        .iter()
        .find(|patch| patch.uuid == uuid)
        .unwrap();
    assert!(patch.created.silent);
    let document =
        bbb_server_persistence::created_player_document(patch, save.item_stack_resolver.as_dyn())
            .unwrap();
    let saved = Java26PlayerData::parse(document, uuid).unwrap();
    assert!(saved.silent());
    let projection = world
        .player_save_without_id(world.levels.player_at(PLAYER), &world.item_stack_resolver())
        .expect("the fixture operation succeeds");
    assert!(projection.iter().any(
        |entry| entry.name == "Silent" && entry.value == bbb_protocol::NbtSummaryValue::Byte(1)
    ));

    sender
        .try_send(WorldCommand::Disconnect {
            connection_id: PLAYER,
        })
        .unwrap();
    world.tick(receiver.drain());
    crate::tests::join_loaded(&sender, &receiver, &mut world, 3, "alice", 1);
    let rejoined = ConnectionId(3);
    assert!(world.levels[rejoined].silent);
    assert!(world.hurt_connected_player(
        rejoined,
        "minecraft:generic_kill",
        40.0,
        None,
        None,
        None,
        &mut Vec::new()
    ));
    sender
        .try_send(WorldCommand::Play {
            connection_id: rejoined,
            packet: PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
        })
        .unwrap();
    world.tick(receiver.drain());
    assert!(!world.levels[rejoined].silent);
    assert!(!world.levels[rejoined].silent_dirty);
    let projection = world
        .player_save_without_id(
            world.levels.player_at(rejoined),
            &world.item_stack_resolver(),
        )
        .expect("the fixture operation succeeds");
    assert!(!projection.iter().any(|entry| entry.name == "Silent"));

    let root =
        std::env::temp_dir().join(format!("bbb-player-silent-restore-{}", std::process::id()));
    let storage = crate::tests::created_fixture_save(&root, "silent-restore");
    let mut candidate = storage.load_candidate().unwrap();
    candidate
        .players
        .insert(uuid, (PlayerDataSource::Primary, saved));
    let mut restored = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert!(restored.cold.restored_players[&uuid].silent);
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
    assert!(restored.levels[PLAYER].silent);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn player_environment_silent_world_tracking_changes_once_and_teleport_preserves() {
    use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SILENT_DATA_ID;
    use bbb_protocol::packets::EntityDataValueKind;
    use bbb_server_types::save::Java26BuiltinDimension;
    let (mut world, _, _) = fixture();
    world.reconcile_player_tracking_for_test(PLAYER);
    world.reconcile_player_tracking_for_test(OBSERVER);
    let entity_id = world.levels[PLAYER].entity_id();
    for silent in [true, false, true] {
        world.levels[PLAYER].set_silent(silent);
        let tick = world.tick(Vec::new());
        let rows = |events: &[WorldEvent]| {
            events
                .iter()
                .flat_map(WorldEvent::deliveries)
                .filter_map(|(connection_id, packet)| match packet {
                    ClientboundPlayPacket::SetEntityData(data) if data.id == entity_id => data
                        .values
                        .iter()
                        .find(|row| row.data_id == VANILLA_ENTITY_SILENT_DATA_ID)
                        .map(|row| (connection_id, row.value.clone())),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let observed = rows(&tick.events);
        assert_eq!(observed.len(), 2);
        for recipient in [PLAYER, OBSERVER] {
            assert!(observed.contains(&(recipient, EntityDataValueKind::Boolean(silent))));
        }
        world.levels[PLAYER].set_silent(silent);
        assert!(rows(&world.tick(Vec::new()).events).is_empty());
    }
    assert!(world.teleport_player(
        PLAYER,
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 0.5,
                y: 80.0,
                z: 0.5
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut Vec::new()
    ));
    assert!(world.levels[PLAYER].silent);
}
