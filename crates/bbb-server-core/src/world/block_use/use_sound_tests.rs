use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::world::{ServerWorld, WorldCommand, WorldEvent};
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::packets::{
    BlockHitResult, BlockPos, Direction, InteractionHand, PlayServerbound, SoundEventHolder,
    UseItemOn, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_server_types::save::Java26BuiltinDimension;

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id).unwrap()
}

fn sound_recipients(events: &[WorldEvent], resource_id: &str) -> Vec<ConnectionId> {
    let registry_id = vanilla_sound_event_registry_id(resource_id).unwrap();
    let mut recipients: Vec<_> = events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(connection_id, packet)| match packet {
            ClientboundPlayPacket::Sound(sound)
                if sound.sound == SoundEventHolder::Reference { registry_id } =>
            {
                Some(connection_id)
            }
            _ => None,
        })
        .collect();
    recipients.sort_by_key(|connection| connection.0);
    recipients
}

/// `Level.playSound(player, ...)` on a use path (TrapDoorBlock.java:119)
/// skips the acting player, who already predicted it; the lever pulls with a
/// null player on the server (LeverBlock.java:72), so everyone hears it.
#[test]
fn use_sounds_skip_the_acting_player_but_reach_others() {
    let (sender, receiver) = crate::tests::mailbox();
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    for connection in [1, 2] {
        world.set_player_position_for_test(
            ConnectionId(connection),
            Vec3d {
                x: 0.5,
                y: 101.0,
                z: 0.5,
            },
        );
    }
    let trapdoor = BlockPos { x: 2, y: 101, z: 0 };
    let lever = BlockPos {
        x: -2,
        y: 101,
        z: 0,
    };
    let overworld = Java26BuiltinDimension::Overworld;
    let all = crate::fluid::SET_BLOCK_UPDATE_ALL;
    assert!(world.set_block(overworld, trapdoor, state("minecraft:oak_trapdoor"), all));
    // The default lever hangs on the wall to its south.
    assert!(world.set_block(
        overworld,
        BlockPos { z: 1, ..lever },
        state("minecraft:stone"),
        all
    ));
    assert!(world.set_block(overworld, lever, state("minecraft:lever"), all));
    world.tick(receiver.drain());

    for (pos, sound, expected) in [
        (
            trapdoor,
            "minecraft:block.wooden_trapdoor.open",
            vec![ConnectionId(2)],
        ),
        (
            lever,
            "minecraft:block.lever.click",
            vec![ConnectionId(1), ConnectionId(2)],
        ),
    ] {
        sender
            .try_send(WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: PlayServerbound::UseItemOn(UseItemOn {
                    hand: InteractionHand::MainHand,
                    hit: BlockHitResult {
                        pos,
                        direction: Direction::Up,
                        cursor_x: 0.5,
                        cursor_y: 0.5,
                        cursor_z: 0.5,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence: 1,
                }),
            })
            .unwrap();
        let tick = world.tick(receiver.drain());
        assert_eq!(sound_recipients(&tick.events, sound), expected, "{sound}");
    }
}
