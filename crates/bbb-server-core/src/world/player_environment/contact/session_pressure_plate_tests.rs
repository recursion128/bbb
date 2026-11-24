use super::*;
use crate::connection::ConnectionId;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use crate::mailbox::{WorldCommandReceiver, WorldCommandSender};
use crate::world::WorldCommand;
use crate::world::WorldEvent;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::packets::{MovePlayer, PlayServerbound, Vec3d};
use bbb_server_types::chunk_ticks::BlockTickType;
use bbb_server_types::save::Java26BuiltinDimension;

const PLAYER: ConnectionId = ConnectionId(1);
const SECOND: ConnectionId = ConnectionId(2);
const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const PLATE: BlockPos = BlockPos { x: 8, y: 65, z: 8 };

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id).unwrap()
}

fn position(x: f64) -> Vec3d {
    Vec3d {
        x,
        y: 65.0625,
        z: 8.5,
    }
}

struct Fixture {
    world: ServerWorld,
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    released: i32,
}

impl Fixture {
    fn new(plate: &str, second: bool) -> Self {
        Self::with_config(plate, second, crate::tests::datapack_config())
    }

    fn with_config(plate: &str, second: bool, config: crate::world::ServerWorldConfig) -> Self {
        let (sender, receiver) = crate::tests::mailbox();
        let mut world = ServerWorld::new(config).unwrap();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        if second {
            crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
            world.set_player_position_for_test(SECOND, position(8.875));
            let player = world.levels.player_mut(SECOND).unwrap();
            player.block_contact.old_position = player.position();
            player.block_contact.pending.clear();
        }
        world.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (0, 0));
        for x in 6..=10 {
            for y in 64..=67 {
                let pos = BlockPos { x, y, z: 8 };
                let state = state(if y == 64 {
                    "minecraft:stone"
                } else {
                    "minecraft:air"
                });
                if world.block_state(DIMENSION, pos) != Some(state) {
                    assert!(world.set_block(DIMENSION, pos, state, SET_BLOCK_UPDATE_ALL));
                }
            }
        }
        world.set_player_position_for_test(PLAYER, position(7.5));
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.block_contact.old_position = player.position();
        player.block_contact.pending.clear();
        let released = state(plate);
        assert!(world.set_block(DIMENSION, PLATE, released, SET_BLOCK_UPDATE_ALL));
        world.level_mut(DIMENSION).event_sink.drain_game_events();
        Self {
            world,
            sender,
            receiver,
            released,
        }
    }

    fn move_player(&mut self, id: ConnectionId, x: f64) {
        self.sender
            .try_send(WorldCommand::Play {
                connection_id: id,
                packet: PlayServerbound::MovePlayer(MovePlayer {
                    position: Some(position(x)),
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                }),
            })
            .unwrap();
        let tick = self.world.tick(self.receiver.drain());
        assert!(!tick
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })));
        assert_eq!(self.world.levels[id].position(), position(x));
    }

    fn current(&mut self) -> i32 {
        self.world.block_state(DIMENSION, PLATE).unwrap()
    }

    fn release_after(&mut self, ticks: usize) {
        for _ in 0..ticks {
            self.world.tick(self.receiver.drain());
        }
        assert_eq!(self.current(), self.released);
        assert!(self
            .world
            .level(DIMENSION)
            .event_sink
            .recent_game_events()
            .any(
                |event| event.game_event == VanillaGameEvent::BLOCK_DEACTIVATE
                    && event.pos == PLATE
                    && event.source_entity_id.is_none()
            ));
    }
}

#[test]
fn session_pressure_plate_movement_activates_binary_plate_and_scheduled_tick_releases() {
    let mut fixture = Fixture::new("minecraft:stone_pressure_plate", false);
    fixture
        .world
        .set_player_position_for_test(PLAYER, position(8.5));
    fixture.world.levels[PLAYER].game_type = GameType::Spectator;
    assert_eq!(
        fixture
            .world
            .player_block_trigger_inside(PLAYER, PLATE, fixture.released),
        Some(())
    );
    assert_eq!(fixture.current(), fixture.released);
    fixture.world.levels[PLAYER].game_type = GameType::Survival;
    fixture
        .world
        .set_player_position_for_test(PLAYER, position(7.5));
    assert_eq!(fixture.current(), fixture.released);
    fixture.move_player(PLAYER, 8.5);
    assert_eq!(
        fixture.current(),
        with_property(fixture.released, "powered", "true").unwrap()
    );
    assert!(fixture.world.level(DIMENSION).chunks.has_scheduled_tick(
        PLATE,
        BlockTickType::of_block_state(fixture.released).unwrap(),
    ));
    assert!(fixture
        .world
        .level(DIMENSION)
        .event_sink
        .recent_game_events()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_ACTIVATE
            && event.pos == PLATE
            && event.source_entity_id == Some(fixture.world.levels[PLAYER].entity_id())));
    fixture.move_player(PLAYER, 6.5);
    fixture.release_after(20);
}

#[test]
fn session_pressure_plate_weighted_first_contact_counts_actual_sessions_and_releases() {
    let mut fixture = Fixture::new("minecraft:light_weighted_pressure_plate", true);
    assert_eq!(fixture.current(), fixture.released);
    fixture.move_player(PLAYER, 8.125);
    assert_eq!(
        fixture.current(),
        with_property(fixture.released, "power", "2").unwrap()
    );
    assert!(fixture
        .world
        .level(DIMENSION)
        .event_sink
        .recent_game_events()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_ACTIVATE
            && event.pos == PLATE
            && event.source_entity_id == Some(fixture.world.levels[PLAYER].entity_id())));
    fixture.move_player(PLAYER, 6.5);
    fixture.move_player(SECOND, 9.5);
    fixture.release_after(10);
}

#[test]
fn a_player_standing_on_a_big_dripleaf_starts_its_tilt() {
    let mut fixture = Fixture::new("minecraft:big_dripleaf", false);
    let on_leaf = Vec3d {
        x: 8.5,
        y: 65.9375,
        z: 8.5,
    };
    fixture
        .sender
        .try_send(WorldCommand::Play {
            connection_id: PLAYER,
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: Some(on_leaf),
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            }),
        })
        .unwrap();
    fixture.world.tick(fixture.receiver.drain());
    assert_eq!(fixture.world.levels[PLAYER].position(), on_leaf);
    assert_eq!(
        fixture.current(),
        with_property(fixture.released, "tilt", "unstable").unwrap()
    );
}
