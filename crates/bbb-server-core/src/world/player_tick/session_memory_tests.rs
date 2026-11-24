use super::*;
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::LevelId;
use crate::world::ServerWorldConfig;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::game_types::GameType;

const PLAYER: ConnectionId = ConnectionId(1);

fn world() -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player = world.levels.player_mut(PLAYER).unwrap();
    {
        let mut position = player.position();
        position.y = 80.0;
        player.set_position(position);
    }
    {
        let mut position = player.position();
        position.x = player.position().x.floor() + 0.5;
        player.set_position(position);
    }
    {
        let mut position = player.position();
        position.z = player.position().z.floor() + 0.5;
        player.set_position(position);
    }
    world
}

fn remember(world: &mut ServerWorld, uuid: Uuid, age: i32) {
    let identity = world
        .levels
        .values()
        .find_map(|level| level.entities.entity_id_for_uuid(uuid).map(|id| (id, uuid)));
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.cold.last_hurt_mob = identity;
    player.cold.last_hurt_mob_timestamp = player.tick_count - age;
    player.cold.last_hurt_by_mob = Some(uuid);
    player.cold.last_hurt_by_mob_timestamp = player.tick_count - age;
}

fn cow(world: &mut ServerWorld, dimension: &LevelId) -> (i32, Uuid) {
    let entities = &mut world.level_mut_by_id(dimension).entities;
    let id = entities
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 0.5,
                y: 80.0,
                z: 0.5,
            },
        )
        .unwrap();
    (id, entities.get(id).unwrap().uuid())
}

#[test]
fn session_memory_tick_keeps_live_other_dimension_and_clears_dead_before_removal() {
    for other_dimension in [false, true] {
        let mut world = world();
        let dimension = if other_dimension {
            Java26BuiltinDimension::Nether.into()
        } else {
            world.levels[PLAYER].dimension
        };
        let (id, uuid) = cow(&mut world, &dimension);
        remember(&mut world, uuid, 3);
        let timestamp = world.levels[PLAYER].cold.last_hurt_by_mob_timestamp;
        world.tick_players(&mut Vec::new());
        let player = world.levels.player_at(PLAYER);
        assert!(player.body_tick_enabled);
        assert_eq!(player.cold.last_hurt_mob.map(|(_, uuid)| uuid), Some(uuid));
        assert_eq!(player.cold.last_hurt_by_mob, Some(uuid));
        assert_eq!(player.cold.last_hurt_by_mob_timestamp, timestamp);
        let body = world
            .level_mut_by_id(&dimension)
            .entities
            .get_mut(id)
            .unwrap();
        body.living_mut().unwrap().set_health(0.0);
        assert!(body.is_alive());
        world.tick_players(&mut Vec::new());
        let player = world.levels.player_at(PLAYER);
        assert_eq!(player.cold.last_hurt_mob, None);
        assert_eq!(player.cold.last_hurt_by_mob, None);
        assert_eq!(player.cold.last_hurt_by_mob_timestamp, player.tick_count);
    }
}

#[test]
fn session_memory_tick_preserves_unresolved_uuid_past_expiry_but_expires_resolved_live() {
    let mut world = world();
    let dimension = world.levels[PLAYER].dimension;
    let unresolved = Uuid::from_u128(0xfeed);
    remember(&mut world, unresolved, 101);
    let timestamp = world.levels[PLAYER].cold.last_hurt_by_mob_timestamp;
    world.tick_players(&mut Vec::new());
    let player = world.levels.player_at(PLAYER);
    assert!(player.body_tick_enabled);
    assert_eq!(player.cold.last_hurt_mob, None);
    assert_eq!(player.cold.last_hurt_by_mob, Some(unresolved));
    assert_eq!(player.cold.last_hurt_by_mob_timestamp, timestamp);
    let (_, uuid) = cow(&mut world, &dimension);
    remember(&mut world, uuid, 101);
    world.tick_players(&mut Vec::new());
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.cold.last_hurt_mob.map(|(_, uuid)| uuid), Some(uuid));
    assert_eq!(player.cold.last_hurt_by_mob, None);
    assert_eq!(player.cold.last_hurt_by_mob_timestamp, player.tick_count);
}
