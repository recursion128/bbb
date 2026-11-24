use super::*;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::ServerWorldConfig;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_STANDING_ID, VANILLA_ENTITY_POSE_SWIMMING_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;

const PLAYER: ConnectionId = ConnectionId(1);
const BLOCK: BlockPos = BlockPos { x: 0, y: 79, z: 0 };

fn state(resource: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(resource).unwrap().id)
        .unwrap()
}

fn world() -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.levels[PLAYER].set_position(Vec3d {
        x: 0.5,
        y: 79.9375,
        z: 0.5,
    });
    world.update_player_query_section(PLAYER);
    world
}

#[test]
fn block_push_lifts_intersecting_stored_body_updates_old_position_and_refiles_section() {
    let mut world = world();
    let dimension = world.levels[PLAYER].dimension;
    world.levels[PLAYER].game_type = GameType::Spectator;
    let start = Vec3d {
        x: 0.5,
        y: 79.9375,
        z: 0.5,
    };
    let entities = &mut world.level_mut_by_id(&dimension).entities;
    let lifted = entities
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, start)
        .unwrap();
    let outside_pos = Vec3d { x: 2.5, ..start };
    let outside = entities
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, outside_pos)
        .unwrap();
    let mut events = Vec::new();
    world
        .push_live_entities_up(
            &dimension,
            state("minecraft:farmland"),
            state("minecraft:dirt"),
            BLOCK,
            &mut events,
        )
        .unwrap();
    let entities = &world.level_by_id(&dimension).entities;
    let entity = entities.get(lifted).unwrap();
    assert_eq!(entity.position(), Vec3d { y: 80.0, ..start });
    assert_eq!(entity.old_position(), entity.position());
    assert_eq!(entity.bounding_box().min_y, 80.0);
    assert_eq!(entities.get(outside).unwrap().position(), outside_pos);
    assert!(entities
        .ids_in_box_sections(Aabb {
            min_x: 0.0,
            min_y: 80.0,
            min_z: 0.0,
            max_x: 1.0,
            max_y: 80.1,
            max_z: 1.0
        })
        .contains(&lifted));
    assert!(events.is_empty());
}

#[test]
fn block_push_scaled_swimming_player_receives_relative_teleport() {
    let mut world = world();
    let dimension = world.levels[PLAYER].dimension;
    let player = world.levels.player_mut(PLAYER).unwrap();
    {
        let mut position = player.position();
        position.x = 1.45;
        player.set_position(position);
    }
    player.boarding_cooldown = 7;
    player.set_delta_movement(Vec3d {
        x: 0.25,
        y: -0.1,
        z: 0.5,
    });
    player.pose_id = VANILLA_ENTITY_POSE_SWIMMING_ID;
    player
        .attributes
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap()
        .set_base_value(2.0);
    let mut events = Vec::new();
    world
        .push_live_entities_up(
            &dimension,
            state("minecraft:farmland"),
            state("minecraft:dirt"),
            BLOCK,
            &mut events,
        )
        .unwrap();
    assert_eq!(world.levels[PLAYER].position().y, 80.0);
    assert_eq!(world.levels[PLAYER].position().x, 1.45);
    assert_eq!(world.levels[PLAYER].boarding_cooldown, 7);
    assert_eq!(
        world.levels[PLAYER].delta_movement(),
        Vec3d {
            x: 0.25,
            y: -0.1,
            z: 0.5
        }
    );
    let update = events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } if *connection_id == PLAYER => Some(update),
            _ => None,
        })
        .expect("the scaled body overlaps the added shape");
    assert_eq!(
        update.position,
        Vec3d {
            x: 0.0,
            y: 0.0625,
            z: 0.0
        }
    );
    assert_eq!(
        update.relatives_mask,
        PLAYER_RELATIVE_X
            | PLAYER_RELATIVE_Y
            | PLAYER_RELATIVE_Z
            | PLAYER_RELATIVE_DELTA_X
            | PLAYER_RELATIVE_DELTA_Y
            | PLAYER_RELATIVE_DELTA_Z
            | PLAYER_RELATIVE_Y_ROT
            | PLAYER_RELATIVE_X_ROT
            | PLAYER_RELATIVE_ROTATE_DELTA
    );
}

#[test]
fn block_push_query_uses_pose_height_not_standing_height() {
    let mut world = world();
    let dimension = world.levels[PLAYER].dimension;
    let player = world.levels.player_mut(PLAYER).unwrap();
    {
        let mut position = player.position();
        position.y = 79.0;
        player.set_position(position);
    }
    player.pose_id = VANILLA_ENTITY_POSE_SWIMMING_ID;
    let above = BlockPos { y: 80, ..BLOCK };
    let mut events = Vec::new();
    world
        .push_live_entities_up(
            &dimension,
            state("minecraft:air"),
            state("minecraft:stone"),
            above,
            &mut events,
        )
        .unwrap();
    assert!(events.is_empty());
    world.levels[PLAYER].pose_id = VANILLA_ENTITY_POSE_STANDING_ID;
    world
        .push_live_entities_up(
            &dimension,
            state("minecraft:air"),
            state("minecraft:stone"),
            above,
            &mut events,
        )
        .unwrap();
    assert!(events.iter().any(|event| matches!(event, WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::PlayerPosition(_) } if *connection_id == PLAYER)));
}

#[test]
fn block_push_skips_empty_shape_and_spectators() {
    let mut world = world();
    let dimension = world.levels[PLAYER].dimension;
    let before = world.levels[PLAYER].position();
    let mut events = Vec::new();
    world
        .push_live_entities_up(
            &dimension,
            state("minecraft:stone"),
            state("minecraft:dirt"),
            BLOCK,
            &mut events,
        )
        .unwrap();
    world.levels[PLAYER].game_type = GameType::Spectator;
    world
        .push_live_entities_up(
            &dimension,
            state("minecraft:farmland"),
            state("minecraft:dirt"),
            BLOCK,
            &mut events,
        )
        .unwrap();
    assert_eq!(world.levels[PLAYER].position(), before);
    assert!(events.is_empty());
}

#[test]
fn block_push_snaps_the_live_player_passenger_without_allocating_a_teleport() {
    use crate::entity::ExternalPlayerCanRide;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;
    let mut world = world();
    let dimension = world.levels[PLAYER].dimension;
    let player_id = world.levels[PLAYER].entity_id();
    let start = world.levels[PLAYER].position();
    // The rider is outside the initial block query; the vehicle must still move it.
    {
        let mut position = world.levels[PLAYER].position();
        position.y = 82.0;
        world.levels[PLAYER].set_position(position);
    }
    world.levels[PLAYER].set_delta_movement(Vec3d {
        x: 0.3,
        y: -0.2,
        z: 0.1,
    });
    let velocity = world.levels[PLAYER].delta_movement();
    world.levels[PLAYER].set_y_rot(180.0);
    world.levels[PLAYER].awaiting_teleport = None;
    let entities = &mut world.level_mut_by_id(&dimension).entities;
    let boat =
        crate::entity::spawn_boat(entities, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, start, 30.0).unwrap();
    assert!(entities.attach_test_player(player_id, boat, ExternalPlayerCanRide::Allowed));
    world.update_player_query_section(PLAYER);
    let mut events = Vec::new();
    world
        .push_live_entities_up(
            &dimension,
            state("minecraft:farmland"),
            state("minecraft:dirt"),
            BLOCK,
            &mut events,
        )
        .unwrap();
    let entities = &world.level_by_id(&dimension).entities;
    assert_eq!(
        entities.get(boat).unwrap().position(),
        Vec3d { y: 80.0, ..start }
    );
    let expected = entities
        .external_player_vehicle_placement(player_id, 180.0)
        .unwrap();
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.position(), expected.position);
    assert_eq!(player.rotation().y_rot, expected.y_rot);
    assert_eq!(player.block_contact.old_position, player.position());
    assert_eq!(player.first_good, player.position());
    assert_eq!(player.last_good, player.position());
    assert_eq!(player.delta_movement(), velocity);
    assert_eq!(player.awaiting_teleport, None);
    assert!(world
        .level_by_id(&dimension)
        .entities
        .players_in_box_sections(Aabb::of_entity(player.position(), 0.6, 1.8))
        .contains(&PLAYER));
    assert!(!events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::PlayerPosition(_),
            ..
        }
    )));
}
