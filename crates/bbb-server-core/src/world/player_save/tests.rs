use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::GameProfile;
use bbb_server_persistence::RegionCompression;

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::world::{JoinContext, ServerWorld, WorldEvent};

fn join(world: &mut ServerWorld, connection_id: ConnectionId) -> Vec<WorldEvent> {
    let mut events = Vec::new();
    world.join(
        connection_id,
        GameProfile {
            uuid: uuid::Uuid::from_u128(7),
            name: "alice".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut events,
    );
    events
}

/// `ServerPlayer.java:400,2038-2042,2157`: the mode a session left crosses a
/// rejoin and reaches the login's previous game type.
#[test]
fn a_rejoin_sends_and_saves_the_previous_game_mode() {
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    join(&mut world, ConnectionId(1));
    let first = world.levels[ConnectionId(1)].game_type;
    assert!(world.set_player_game_mode(ConnectionId(1), GameType::Spectator, &mut Vec::new()));
    world.remove_player(ConnectionId(1), &mut Vec::new());

    let events = join(&mut world, ConnectionId(2));
    let login = events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Login(login),
                ..
            } => Some(login),
            _ => None,
        })
        .expect("the rejoin sends a login");
    assert_eq!(login.common_spawn_info.game_type, 3);
    assert_eq!(
        i32::from(login.common_spawn_info.previous_game_type),
        first.id()
    );
    let patch = world
        .save_request(0, 0, RegionCompression::Zlib)
        .unwrap()
        .player_patches
        .remove(0);
    assert_eq!(patch.previous_game_mode, Some(first));
}
