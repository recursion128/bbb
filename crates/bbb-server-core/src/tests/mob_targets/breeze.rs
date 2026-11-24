use super::melee::{fixture, ALICE, PLAYER_POSITION};
use super::*;
use crate::tests::entities::{console_command, console_command_answer, restore_entity_writes};
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_INHALING_ID, VANILLA_ENTITY_POSE_LONG_JUMPING_ID,
    VANILLA_ENTITY_POSE_SHOOTING_ID, VANILLA_ENTITY_POSE_SLIDING_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BREEZE_ID, VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID,
};
use bbb_protocol::packets::EntityDataValueKind;

const BREEZE_SCALE: f32 = 2.0;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) {
    let tick = console_command(sender, receiver, world, 200, text);
    assert!(console_command_answer(&tick).0.is_some(), "{text}");
}

fn mount_small_stand(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    breeze_uuid: Uuid,
) -> (i32, Uuid) {
    command(
        sender,
        receiver,
        world,
        "summon minecraft:armor_stand 3.5 66 8.5 {Small:1b,NoGravity:1b,Invulnerable:1b}",
    );
    let passenger = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIMENSION, *id)
                .unwrap()
                .entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
        })
        .unwrap();
    let uuid = world
        .stored_entity_for_test(DIMENSION, passenger)
        .unwrap()
        .uuid();
    command(
        sender,
        receiver,
        world,
        &format!("ride {uuid} mount {breeze_uuid}"),
    );
    (passenger, uuid)
}

fn assert_normal_pose_passenger(world: &ServerWorld, breeze: i32, passenger: i32) {
    assert_eq!(
        world.stored_passengers_for_test(DIMENSION, breeze),
        &[passenger]
    );
    let vehicle = world.stored_entity_for_test(DIMENSION, breeze).unwrap();
    let origin = vehicle.position();
    let rider = world
        .stored_entity_for_test(DIMENSION, passenger)
        .unwrap()
        .position();
    assert!((rider.x - origin.x).abs() < 1e-6);
    let height =
        bbb_protocol::entity_types::vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_BREEZE_ID)
            .unwrap()[1];
    assert!((rider.y - origin.y - f64::from(height * BREEZE_SCALE)).abs() < 1e-6);
    assert!((rider.z - origin.z).abs() < 1e-6);
}

#[test]
fn a_breeze_slides_jumps_and_shoots_owned_wind_charges_that_push_the_player() {
    for distance in [2.0, 6.0] {
        let mut observed_poses = Vec::new();
        let mut mounted_poses = Vec::new();
        let mut pushed = false;
        let mut jumped = false;
        let mut moved = false;
        let mut owned_shot = false;
        let expected_poses = if distance == 2.0 {
            &[
                VANILLA_ENTITY_POSE_SLIDING_ID,
                VANILLA_ENTITY_POSE_SHOOTING_ID,
            ][..]
        } else {
            &[
                VANILLA_ENTITY_POSE_INHALING_ID,
                VANILLA_ENTITY_POSE_LONG_JUMPING_ID,
                VANILLA_ENTITY_POSE_SHOOTING_ID,
            ][..]
        };
        let (sender, receiver, mut world) = fixture();
        world.tick_rate_mut().set_frozen(true);
        let origin = Vec3d {
            x: PLAYER_POSITION.x - distance,
            ..PLAYER_POSITION
        };
        let (breeze, owner) = spawn(&mut world, VANILLA_ENTITY_TYPE_BREEZE_ID, origin);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("attribute {owner} minecraft:scale base set {BREEZE_SCALE}"),
        );
        let (passenger, passenger_uuid) = mount_small_stand(&sender, &receiver, &mut world, owner);
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..600 {
            let tick = world.tick(Vec::new());
            if let Some(body) = world.stored_entity_for_test(DIMENSION, breeze) {
                jumped |= body.position().y > origin.y + 1.0;
                moved |= (body.position().x - origin.x).abs() > 1.0
                    || (body.position().z - origin.z).abs() > 1.0;
                if expected_poses.contains(&body.pose_id()) {
                    assert_normal_pose_passenger(&world, breeze, passenger);
                    if !mounted_poses.contains(&body.pose_id()) {
                        mounted_poses.push(body.pose_id());
                    }
                }
            }
            for event in &tick.events {
                match event {
                    WorldEvent::Send {
                        connection_id: ALICE,
                        packet: ClientboundPlayPacket::SetEntityData(update),
                    } if update.id == breeze => {
                        for value in &update.values {
                            if let EntityDataValueKind::Pose(pose) = value.value {
                                if !observed_poses.contains(&pose) {
                                    observed_poses.push(pose);
                                }
                            }
                        }
                    }
                    WorldEvent::Send {
                        connection_id: ALICE,
                        packet: ClientboundPlayPacket::AddEntity(add),
                    } if add.entity_type_id == VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID => {
                        let shot = world
                            .stored_entity_for_test(DIMENSION, add.id)
                            .expect("a real wind charge spawns");
                        let EntityData::BreezeWindCharge(data) = shot.data() else {
                            panic!("breeze projectile data")
                        };
                        assert_eq!(data.projectile().owner(), Some(owner));
                        owned_shot = true;
                    }
                    WorldEvent::Send {
                        connection_id: ALICE,
                        packet: ClientboundPlayPacket::Explode(explosion),
                    } => {
                        pushed |= explosion.player_knockback.is_some_and(|movement| {
                            movement.x != 0.0 || movement.y != 0.0 || movement.z != 0.0
                        });
                    }
                    _ => {}
                }
            }
            if owned_shot
                && pushed
                && moved
                && (distance == 2.0 || jumped)
                && expected_poses
                    .iter()
                    .all(|pose| observed_poses.contains(pose))
                && expected_poses
                    .iter()
                    .all(|pose| mounted_poses.contains(pose))
            {
                break;
            }
        }
        assert_eq!(target_of(&world, breeze), Some(Uuid::from_u128(1)));
        assert!(owned_shot && pushed && moved && (distance == 2.0 || jumped),
            "distance={distance}, shot={owned_shot}, pushed={pushed}, jumped={jumped}, moved={moved}");
        for pose in expected_poses {
            assert!(
                observed_poses.contains(pose),
                "distance={distance}, pose {pose} in {observed_poses:?}"
            );
            assert!(
                mounted_poses.contains(pose),
                "distance={distance}, mounted pose {pose} in {mounted_poses:?}"
            );
        }
        if distance == 6.0 {
            let request = world
                .save_request(1_786_000_014_002, 216, RegionCompression::Zlib)
                .unwrap();
            let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
            restore_entity_writes(&mut candidate, &request.entity_writes);
            let mut config = datapack_config();
            config.game_type = GameType::Survival;
            config.difficulty = Difficulty::Normal;
            let mut restored = spawn_pinned_restore(config, &candidate);
            let breeze = restored
                .live_entity_id_for_uuid_for_test(DIMENSION, owner)
                .unwrap();
            let passenger = restored
                .live_entity_id_for_uuid_for_test(DIMENSION, passenger_uuid)
                .unwrap();
            assert_eq!(
                restored.stored_passengers_for_test(DIMENSION, breeze),
                &[passenger]
            );
            restored.tick_rate_mut().set_frozen(true);
            join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
            install_floor(&mut restored);
            restored.set_player_position_for_test(ALICE, PLAYER_POSITION);
            command(
                &sender,
                &receiver,
                &mut restored,
                "gamerule spawn_mobs false",
            );
            restored.tick_rate_mut().set_frozen(false);
            let mut resumed_pose = None;
            for _ in 0..600 {
                restored.tick(Vec::new());
                let body = restored.stored_entity_for_test(DIMENSION, breeze).unwrap();
                if expected_poses.contains(&body.pose_id()) {
                    assert_normal_pose_passenger(&restored, breeze, passenger);
                    resumed_pose = Some(body.pose_id());
                    break;
                }
            }
            assert!(
                resumed_pose.is_some(),
                "the restored mounted breeze resumes combat"
            );
            assert_eq!(target_of(&restored, breeze), Some(Uuid::from_u128(1)));
        }
    }
}

#[test]
fn an_idle_breeze_rises_in_water_and_lava_without_an_attack_target() {
    for fluid in [
        bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID,
        bbb_protocol::block_types::VANILLA_BLOCK_LAVA_ID,
    ] {
        let (sender, receiver, mut world) = fixture();
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            120,
            "gamemode creative alice",
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
        let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(fluid).unwrap();
        for x in 2..=6 {
            for z in 2..=6 {
                for y in 65..=68 {
                    assert!(world.set_block(
                        DIMENSION,
                        BlockPos { x, y, z },
                        state,
                        SET_BLOCK_UPDATE_ALL
                    ));
                }
            }
        }
        let (breeze, _) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_BREEZE_ID,
            Vec3d {
                x: 4.5,
                y: 65.0,
                z: 4.5,
            },
        );
        world
            .stored_entity_mut_for_test(DIMENSION, breeze)
            .unwrap()
            .set_invulnerable(true);
        tick_until(&mut world, 160, |world| {
            world
                .stored_entity_for_test(DIMENSION, breeze)
                .is_some_and(|body| body.position().y > 67.0)
        });
        assert_eq!(target_of(&world, breeze), None);
    }
}

#[test]
fn a_breeze_in_shallow_water_shoots_when_no_jump_landing_exists() {
    let (_sender, _receiver, mut world) = fixture();
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            y: 76.0,
            ..PLAYER_POSITION
        },
    );
    let water = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID,
    )
    .unwrap();
    assert!(world.set_block(
        DIMENSION,
        BlockPos { x: 4, y: 65, z: 8 },
        water,
        SET_BLOCK_UPDATE_ALL
    ));
    let (_, owner) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_BREEZE_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 8.5,
        },
    );
    for _ in 0..100 {
        let tick = world.tick(Vec::new());
        let shot = tick.events.iter().find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::AddEntity(add),
            } if add.entity_type_id == VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID => Some(add.id),
            _ => None,
        });
        if let Some(shot) = shot {
            let EntityData::BreezeWindCharge(data) = world
                .stored_entity_for_test(DIMENSION, shot)
                .unwrap()
                .data()
            else {
                panic!("breeze projectile data")
            };
            assert_eq!(data.projectile().owner(), Some(owner));
            return;
        }
    }
    panic!("a breeze with no reachable landing shoots from shallow water");
}
