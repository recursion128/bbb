use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::packets::{BlockPos, Direction};

use crate::block_behavior::chest::{trapped_direct_signal, trapped_signal};
use crate::redstone::{direct_signal, is_signal_source, signal, SignalContext, WireSignal};

use super::{state_of, stone, NativeWorld};

const CHEST: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const HOPPER: BlockPos = BlockPos { x: 1, y: 64, z: 0 };
const FLOOR: BlockPos = BlockPos { x: 0, y: 63, z: 0 };
const ORIGIN: BlockPos = BlockPos { x: 0, y: 63, z: 0 };

fn trapped_chest() -> i32 {
    state_of(
        "minecraft:trapped_chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    )
}

fn plain_chest() -> i32 {
    state_of(
        "minecraft:chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    )
}

fn hopper_state(enabled: bool) -> i32 {
    state_of(
        "minecraft:hopper",
        &[
            ("facing", "down"),
            ("enabled", if enabled { "true" } else { "false" }),
        ],
    )
}

fn enabled(state: i32) -> Option<String> {
    vanilla_block_state_identity(state)?
        .properties
        .get("enabled")
        .cloned()
}

#[test]
fn a_trapped_chest_is_a_signal_source_and_a_plain_chest_is_not() {
    assert!(is_signal_source(trapped_chest(), WireSignal::Emitting));
    assert!(!is_signal_source(plain_chest(), WireSignal::Emitting));
}

#[test]
fn trapped_chest_signal_clamps_the_opener_count() {
    assert_eq!(trapped_signal(0), 0);
    assert_eq!(trapped_signal(1), 1);
    assert_eq!(trapped_signal(15), 15);
    assert_eq!(trapped_signal(16), 15);
    assert_eq!(trapped_direct_signal(Direction::Up, 7), 7);
    assert_eq!(trapped_direct_signal(Direction::North, 7), 0);
    assert_eq!(trapped_direct_signal(Direction::Down, 16), 0);

    let openers = |_: BlockPos| 16;
    let sources = crate::redstone::TestSignalSources {
        chest_open_count_at: Some(&openers),
        ..Default::default()
    };
    let mut read = |_: BlockPos| trapped_chest();
    let mut context = SignalContext::emitting(&mut read).with_reader(&sources);
    assert_eq!(
        signal(trapped_chest(), ORIGIN, Direction::North, &mut context),
        15
    );
    assert_eq!(
        direct_signal(trapped_chest(), ORIGIN, Direction::Up, &mut context),
        15
    );
    assert_eq!(
        direct_signal(trapped_chest(), ORIGIN, Direction::South, &mut context),
        0
    );
}

#[test]
fn real_chest_menus_lock_the_hopper_only_for_a_trapped_chest() {
    use crate::world::{ConnectionId, WorldCommand};
    use bbb_protocol::packets::{
        BlockHitResult, ContainerCloseRequest, InteractionHand, PlayServerbound, UseItemOn, Vec3d,
    };
    for trapped in [false, true] {
        let mut world = NativeWorld::new();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut world.world, 1, "opener", 1);
        crate::tests::place_player(
            &sender,
            &receiver,
            &mut world.world,
            1,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: -2.0,
            },
            None,
        );
        world.set(FLOOR, stone());
        world.set(
            CHEST,
            if trapped {
                trapped_chest()
            } else {
                plain_chest()
            },
        );
        world.set(HOPPER, hopper_state(true));
        sender
            .try_send(WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: PlayServerbound::UseItemOn(UseItemOn {
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
            })
            .unwrap();
        let opened = world.world.tick(receiver.drain());
        assert!(opened.events.iter().any(|event| matches!(
            event,
            crate::world::WorldEvent::Send {
                packet: crate::connection::ClientboundPlayPacket::OpenScreen(_),
                ..
            }
        )));
        assert_eq!(
            world.with_level(|level| level.container_openers_count(CHEST)),
            Some(1)
        );
        assert_eq!(
            enabled(world.state(HOPPER)).as_deref(),
            Some(if trapped { "false" } else { "true" })
        );
        sender
            .try_send(WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: PlayServerbound::ContainerClose(ContainerCloseRequest { container_id: 1 }),
            })
            .unwrap();
        world.world.tick(receiver.drain());
        assert_eq!(
            world.with_level(|level| level.container_openers_count(CHEST)),
            Some(0)
        );
        assert_eq!(enabled(world.state(HOPPER)).as_deref(), Some("true"));
    }
}
