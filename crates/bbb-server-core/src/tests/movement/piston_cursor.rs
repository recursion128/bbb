use super::*;
use crate::level_event::VanillaGameEvent;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const PISTON: BlockPos = BlockPos { x: 9, y: 63, z: 8 };
const START: Vec3d = Vec3d {
    x: 8.65,
    y: 64.6,
    z: 8.5,
};

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn moving_state() -> i32 {
    crate::block_behavior::piston::moving_piston_state(Direction::East, false)
}

fn fixture() -> (ServerWorld, WorldCommandSender, WorldCommandReceiver) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    for x in 7..=10 {
        for y in 62..=68 {
            for z in 7..=10 {
                world.set_raw_block_state_for_test(
                    DIM,
                    BlockPos { x, y, z },
                    state("minecraft:air"),
                );
            }
        }
    }
    assert!(world.set_block(DIM, PISTON, moving_state(), 3));
    let block_entity = world.level_block_entity_for_test(DIM, PISTON).unwrap();
    assert!(block_entity.configure_piston(
        state("minecraft:oak_fence"),
        Direction::East,
        true,
        false
    ));
    block_entity.tick_piston_without_entities(0).unwrap();
    assert_eq!(block_entity.piston_snapshot().unwrap().progress, 0.5);
    world.set_player_position_for_test(ConnectionId(1), START);
    assert_eq!(
        world.player_movement_validation_for_test(ConnectionId(1)).0,
        START
    );
    (world, sender, receiver)
}

fn move_packet(target_y: f64, on_ground: bool) -> PlayServerbound {
    PlayServerbound::MovePlayer(MovePlayer {
        position: Some(Vec3d {
            y: target_y,
            ..START
        }),
        rotation: None,
        on_ground,
        horizontal_collision: false,
    })
}

fn correction(tick: &TickResult) -> Option<Vec3d> {
    tick.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::PlayerPosition(update),
        } => Some(update.position),
        _ => None,
    })
}

#[test]
fn piston_cursor_session_rejects_claim_inside_moving_fence_but_accepts_nonmoving_stale_be() {
    // ServerGamePacketListenerImpl.java:1121-1146; ServerGamePacketListenerImpl.java:1180-1183.
    for moving in [true, false] {
        let (mut world, sender, receiver) = fixture();
        if !moving {
            world.set_raw_block_state_for_test(DIM, PISTON, state("minecraft:oak_fence"));
        }
        assert_eq!(
            world
                .level_block_entity_for_test(DIM, PISTON)
                .unwrap()
                .piston_snapshot()
                .unwrap()
                .progress,
            0.5
        );
        let walked = play(&sender, &receiver, &mut world, 1, move_packet(64.1, false));
        let (position, awaiting, _, last_good, _, _) =
            world.player_movement_validation_for_test(ConnectionId(1));
        if moving {
            assert_eq!(correction(&walked), Some(START));
            assert_eq!(position, START);
            assert!(awaiting.is_some());
            assert_eq!(last_good, START);
        } else {
            let target = Vec3d { y: 64.1, ..START };
            assert_eq!(correction(&walked), None);
            assert_eq!(position, target);
            assert_eq!(last_good, target);
            assert_eq!(awaiting, None);
        }
    }
}

#[test]
fn piston_cursor_session_accepts_reported_fence_top_and_attributes_hit_ground_to_piston() {
    let (mut world, sender, receiver) = fixture();
    let before = world.game_events(DIM).0.len();
    let entity_id = entity_id_of(&world, 1);
    let walked = play(&sender, &receiver, &mut world, 1, move_packet(64.5, true));
    let target = Vec3d { y: 64.5, ..START };
    assert_eq!(correction(&walked), None);
    assert_eq!(
        world.player_movement_validation_for_test(ConnectionId(1)).0,
        target
    );
    // The claim lands (the hit below), but the block entity's tick then slides
    // the fence on to a full step east, out from under the session, so the
    // tick's own travel finds nothing there (`LivingEntity.java:3061-3065`).
    assert!(
        !world
            .publication()
            .players
            .iter()
            .find(|player| player.connection_id == ConnectionId(1))
            .unwrap()
            .on_ground
    );
    let events = world.game_events(DIM).0;
    let hits: Vec<_> = events
        .iter()
        .skip(before)
        .filter(|event| {
            event.game_event == VanillaGameEvent::HIT_GROUND
                && event.source_entity_id == Some(entity_id)
        })
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].source_entity_id, Some(entity_id));
    assert_eq!(hits[0].affected_state, Some(moving_state()));
    assert_eq!(
        world
            .positioned_game_events_for_test(DIM)
            .iter()
            .find(|event| event.recorded == *hits[0])
            .unwrap()
            .position,
        target
    );
}
