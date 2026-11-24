use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::entity::tracking::ServerEntity;
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_FALL_FLYING_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
    VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
};
use bbb_protocol::packets::EntityDataValueKind;

#[test]
fn desired_pose_swimming_precedes_gliding_and_reaches_tracking_once() {
    let (sender, receiver) =
        world_command_mailbox(crate::mailbox::WorldMailboxLimits::default()).unwrap();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let dimension = Java26BuiltinDimension::Overworld.into();
    world
        .level_mut_by_id(&dimension)
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    let player = world.levels.player_mut(ConnectionId(1)).unwrap();
    player.set_position(bbb_protocol::packets::Vec3d {
        x: 8.5,
        y: 100.0,
        z: 8.5,
    });
    player.abilities.flying = false;
    let before = player.drain_tracking_frame(false);
    assert_eq!(before.pose_id, VANILLA_ENTITY_POSE_STANDING_ID);
    let mut tracker = ServerEntity::for_player(&before);
    for (swimming, gliding, crouching, flying, expected) in [
        (true, true, true, false, VANILLA_ENTITY_POSE_SWIMMING_ID),
        (false, true, true, false, VANILLA_ENTITY_POSE_FALL_FLYING_ID),
        (false, false, true, false, VANILLA_ENTITY_POSE_CROUCHING_ID),
        (false, false, true, true, VANILLA_ENTITY_POSE_STANDING_ID),
    ] {
        let player = world.levels.player_mut(ConnectionId(1)).unwrap();
        player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SWIMMING, swimming);
        player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, gliding);
        player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, crouching);
        player.abilities.flying = flying;
        world.tick_player_poses();
        let frame = world.levels[ConnectionId(1)].drain_tracking_frame(false);
        assert_eq!(frame.pose_id, expected);
        assert!(frame.pose_dirty);
        let packets = tracker.send_player_changes(frame);
        assert_eq!(
            packets
                .ordered
                .iter()
                .filter(|send| matches!(&send.packet,
            ClientboundPlayPacket::SetEntityData(data) if data.values.iter().any(|row|
                row.value == EntityDataValueKind::Pose(expected))))
                .count(),
            1
        );
        world.tick_player_poses();
        let stable = world.levels[ConnectionId(1)].drain_tracking_frame(false);
        assert_eq!(stable.pose_id, expected);
        assert!(!stable.pose_dirty);
        assert!(!tracker.send_player_changes(stable).ordered.iter().any(
            |send| matches!(&send.packet,
            ClientboundPlayPacket::SetEntityData(data) if data.values.iter().any(|row|
                matches!(row.value, EntityDataValueKind::Pose(_))))
        ));
    }
}
