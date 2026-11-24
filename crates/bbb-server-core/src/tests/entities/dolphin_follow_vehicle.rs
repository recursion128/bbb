use super::feeding::interact_entity_packet;
use super::horse_interaction::{hold, horse_world};
use super::nautilus_interaction::tame_with_buckets;
use super::saddled_mounts::mount;
use super::*;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_DOLPHIN_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
};
use bbb_protocol::packets::MoveVehicleCommand;

#[test]
fn dolphin_follows_a_players_moving_boat_and_swims_ahead() {
    follows(VANILLA_ENTITY_TYPE_OAK_BOAT_ID, 0.0, 1.0, 0.0);
}

#[test]
fn dolphin_follows_players_riding_either_nautilus_and_swims_ahead() {
    for kind in [
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ] {
        for (dx, yaw) in [(1.0, -90.0), (-1.0, 90.0)] {
            follows(kind, dx, 0.0, yaw);
        }
    }
}

fn follows(kind: i32, dx: f64, dz: f64, yaw: f32) {
    let (sender, receiver, mut world) = horse_world();
    let boat = kind == VANILLA_ENTITY_TYPE_OAK_BOAT_ID;
    let side = if boat { 16 } else { 32 };
    let (vehicle_x, vehicle_z) = if boat { (8.5, 3.5) } else { (16.5, 16.5) };
    let dimension = Java26BuiltinDimension::Overworld;
    let water = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &BTreeMap::from([("level".to_owned(), "0".to_owned())]),
    )
    .unwrap();
    let stone = block_state_id("minecraft:stone");
    for x in 0..side {
        for z in 0..side {
            assert!(world.set_block(
                dimension,
                BlockPos { x, y: 64, z },
                stone,
                SET_BLOCK_UPDATE_ALL
            ));
            for y in 65..=69 {
                assert!(world.set_block(
                    dimension,
                    BlockPos { x, y, z },
                    water,
                    SET_BLOCK_UPDATE_ALL
                ));
            }
        }
    }
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: vehicle_x,
            y: 70.0,
            z: vehicle_z,
        },
    );
    let vehicle = world
        .spawn_stored_entity_for_test(
            dimension,
            kind,
            Vec3d {
                x: vehicle_x,
                y: if boat { 69.6 } else { 68.5 },
                z: vehicle_z,
            },
            if boat {
                EntityData::Boat(crate::entity::BoatData::new())
            } else {
                EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap())
            },
        )
        .0;
    if !boat {
        tame_with_buckets(&sender, &receiver, &mut world, vehicle);
        hold(&mut world, "minecraft:saddle", 1);
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(vehicle),
        );
    }
    mount(&sender, &receiver, &mut world, vehicle);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: None,
            rotation: Some(PlayerRotation {
                y_rot: yaw,
                x_rot: 0.0,
            }),
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    let vehicle_position = world
        .stored_entity_for_test(dimension, vehicle)
        .unwrap()
        .position();
    let start = Vec3d {
        x: vehicle_position.x - 3.0,
        y: 68.5,
        z: vehicle_position.z,
    };
    let dolphin = world
        .spawn_stored_entity_for_test(
            dimension,
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            start,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_DOLPHIN_ID).unwrap(),
            ),
        )
        .0;
    let mut led_the_vehicle = false;
    for _ in 0..120 {
        let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
        let target = Vec3d {
            x: baseline.x + dx * 0.05,
            z: baseline.z + dz * 0.05,
            ..baseline
        };
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::MoveVehicle(MoveVehicleCommand {
                position: target,
                y_rot: yaw,
                x_rot: 0.0,
                on_ground: false,
            }),
        );
        assert_eq!(
            world
                .stored_entity_for_test(dimension, vehicle)
                .unwrap()
                .position(),
            target
        );
        let position = world
            .stored_entity_for_test(dimension, dolphin)
            .unwrap()
            .position();
        let player = world.publication().players[0].position;
        led_the_vehicle |= (position.x - player.x) * dx + (position.z - player.z) * dz > 1.0;
        if led_the_vehicle && (position.x - start.x) * dx + (position.z - start.z) * dz > 3.0 {
            break;
        }
    }
    let position = world
        .stored_entity_for_test(dimension, dolphin)
        .unwrap()
        .position();
    assert!(
        (position.x - start.x) * dx + (position.z - start.z) * dz > 3.0,
        "dolphin follows the moving vehicle: {position:?}"
    );
    assert!(
        led_the_vehicle,
        "dolphin swims ahead of its rider: {position:?}"
    );
}
