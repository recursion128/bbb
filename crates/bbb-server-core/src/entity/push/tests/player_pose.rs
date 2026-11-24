use super::*;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_STANDING_ID, VANILLA_ENTITY_POSE_SWIMMING_ID,
};

#[test]
fn world_push_uses_actual_session_pose_overlap_for_both_impulses() {
    for (x, y, swimming, scale, overlap) in [
        (5.0, 64.0, false, 1.0, true),
        (5.0, 64.0, false, 0.1, false),
        (4.6, 63.0, false, 1.0, true),
        (4.6, 63.0, true, 1.0, false),
        (4.6, 63.0, false, 0.5, false),
        (4.6, 63.0, true, 4.0, true),
    ] {
        let (mut native, sender, receiver) = session_level(x, GameType::Survival);
        command(
            &mut native,
            &sender,
            &receiver,
            "gamerule minecraft:max_entity_cramming 1",
        );
        native.world.set_player_attribute_base_for_test(
            ConnectionId(1),
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID,
            scale,
        );
        native
            .world
            .set_player_position_for_test(ConnectionId(1), Vec3d { x, y, z: 8.5 });
        if y < 64.0 {
            for bx in 2..=7 {
                for bz in 6..=10 {
                    for by in 63..=71 {
                        native.world.set_block(
                            DIMENSION,
                            BlockPos {
                                x: bx,
                                y: by,
                                z: bz,
                            },
                            state(if swimming {
                                "minecraft:water"
                            } else {
                                "minecraft:air"
                            }),
                            2 | 16,
                        );
                    }
                }
            }
        }
        if swimming {
            for flag in [
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
            ] {
                native
                    .world
                    .set_player_shared_flag_for_test(ConnectionId(1), flag, true);
            }
        }
        native.world.tick(receiver.drain());
        assert_eq!(
            native.world.player_pose_id_for_test(ConnectionId(1)),
            if swimming {
                VANILLA_ENTITY_POSE_SWIMMING_ID
            } else {
                VANILLA_ENTITY_POSE_STANDING_ID
            }
        );
        let body = spawn_world_mob(&mut native, 2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, position(4.5));
        native.world.tick(receiver.drain());
        if overlap {
            assert!(
                world_delta(&native, body).x < 0.0,
                "x={x} y={y} swimming={swimming} scale={scale}"
            );
            assert!(player_delta(&native).x > 0.0);
        } else {
            assert_eq!(world_delta(&native, body).x, 0.0);
            assert_eq!(player_delta(&native).x, 0.0);
        }
    }
}
