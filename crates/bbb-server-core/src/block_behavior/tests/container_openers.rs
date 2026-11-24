use super::{state_of, NativeWorld};
use crate::connection::ClientboundPlayPacket;
use crate::world::{ConnectionId, TickResult, WorldCommand, WorldEvent};
use bbb_protocol::packets::{
    BlockHitResult, BlockPos, ContainerCloseRequest, Direction, InteractionHand, PlayServerbound,
    UseItemOn, Vec3d,
};

const CHEST: BlockPos = BlockPos { x: 0, y: 64, z: 1 };

fn play(
    world: &mut NativeWorld,
    sender: &crate::mailbox::WorldCommandSender,
    receiver: &crate::mailbox::WorldCommandReceiver,
    id: u64,
    packet: PlayServerbound,
) -> TickResult {
    sender
        .try_send(WorldCommand::Play {
            connection_id: ConnectionId(id),
            packet,
        })
        .unwrap();
    world.world.tick(receiver.drain())
}

fn open(
    world: &mut NativeWorld,
    sender: &crate::mailbox::WorldCommandSender,
    receiver: &crate::mailbox::WorldCommandReceiver,
    id: u64,
) -> TickResult {
    play(
        world,
        sender,
        receiver,
        id,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: CHEST,
                direction: Direction::North,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.0,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    )
}

fn count(world: &mut NativeWorld) -> i32 {
    world.with_level(|level| level.container_openers_count(CHEST).unwrap())
}

fn state(name: &str) -> i32 {
    match name {
        "minecraft:barrel" => state_of(name, &[("facing", "north"), ("open", "false")]),
        "minecraft:ender_chest" => state_of(name, &[("facing", "north"), ("waterlogged", "false")]),
        _ => state_of(
            name,
            &[
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        ),
    }
}

#[test]
fn real_open_menus_keep_the_count_and_closing_the_last_menu_closes_the_lid() {
    for name in [
        "minecraft:chest",
        "minecraft:trapped_chest",
        "minecraft:barrel",
        "minecraft:ender_chest",
    ] {
        let mut world = NativeWorld::new();
        let (sender, receiver) = crate::tests::mailbox();
        for id in 1..=2 {
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut world.world,
                id,
                &format!("opener{id}"),
                u128::from(id),
            );
            crate::tests::place_player(
                &sender,
                &receiver,
                &mut world.world,
                id,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: -1.5,
                },
                None,
            );
        }
        world.set(CHEST, state(name));
        for id in 1..=2 {
            let opened = open(&mut world, &sender, &receiver, id);
            assert!(
                opened.events.iter().any(|event| matches!(
                    event,
                    WorldEvent::Send {
                        packet: ClientboundPlayPacket::OpenScreen(_),
                        ..
                    }
                )),
                "{name}"
            );
            assert_eq!(count(&mut world), i32::try_from(id).unwrap(), "{name}");
        }
        for _ in 0..6 {
            world.run_tick();
        }
        assert_eq!(count(&mut world), 2, "{name}");
        let close_sound =
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(match name {
                "minecraft:barrel" => crate::block_behavior::barrel::BARREL_CLOSE_SOUND,
                "minecraft:ender_chest" => {
                    crate::block_behavior::container_openers::ENDER_CHEST_CLOSE_SOUND
                }
                _ => crate::block_behavior::chest::CHEST_CLOSE_SOUND,
            })
            .unwrap();
        let partial = play(
            &mut world,
            &sender,
            &receiver,
            1,
            PlayServerbound::ContainerClose(ContainerCloseRequest { container_id: 1 }),
        );
        assert_eq!(count(&mut world), 1, "{name}");
        assert!(!partial.events.iter().any(|event| matches!(
            event.sent_packet(),
            Some(ClientboundPlayPacket::Sound(sound))
            if matches!(&sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id } if *registry_id == close_sound)
        )), "{name}");
        if name == "minecraft:barrel" {
            assert_eq!(
                bbb_server_types::block_queries::bool_property(world.state(CHEST), "open"),
                Some(true)
            );
        }
        let closed = play(
            &mut world,
            &sender,
            &receiver,
            2,
            PlayServerbound::ContainerClose(ContainerCloseRequest { container_id: 1 }),
        );
        assert_eq!(count(&mut world), 0, "{name}");
        assert!(
            closed.events.iter().any(|event| matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::Sound(sound))
                    if matches!(&sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id } if *registry_id == close_sound)
            )),
            "{name}"
        );
        if name != "minecraft:barrel" {
            assert!(closed.events.iter().any(|event| matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::BlockEvent(event))
                if event.pos == CHEST && i32::from(event.b0) == crate::block_behavior::chest::EVENT_SET_OPEN_COUNT && event.b1 == 0
            )), "{name}");
        }
        if name == "minecraft:barrel" {
            assert_eq!(
                bbb_server_types::block_queries::bool_property(world.state(CHEST), "open"),
                Some(false)
            );
        }

        let reopened = open(&mut world, &sender, &receiver, 2);
        assert!(
            reopened.events.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::OpenScreen(_),
                    ..
                }
            )),
            "{name}"
        );
        assert_eq!(count(&mut world), 1, "{name}");
        sender
            .try_send(WorldCommand::Disconnect {
                connection_id: ConnectionId(2),
            })
            .unwrap();
        // `Player.remove` closes the menu (`Player.java:1244-1250`), so the
        // disconnect itself runs `stopOpen`.
        let mut output = world.world.tick(receiver.drain()).events;
        assert_eq!(count(&mut world), 0, "{name}");
        let saved_state = world.state(CHEST);
        // Recheck a real disconnected opener, as an already scheduled block tick would.
        world.with_level(|level| {
            crate::block_behavior::container_openers::schedule_recheck(level, CHEST, saved_state)
        });
        for _ in 0..6 {
            output.extend(world.world.tick(receiver.drain()).events);
        }
        assert_eq!(count(&mut world), 0, "{name}");
        assert!(output.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
            if matches!(&sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id } if *registry_id == close_sound)
        )), "{name}");
        if name == "minecraft:barrel" {
            assert_eq!(
                bbb_server_types::block_queries::bool_property(world.state(CHEST), "open"),
                Some(false)
            );
        }
    }
}
