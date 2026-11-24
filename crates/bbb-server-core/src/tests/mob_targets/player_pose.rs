use super::*;
use crate::entity::LightningBoltData;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID;

fn command(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    id: u64,
    text: &str,
) -> TickResult {
    let tick = console_command(sender, receiver, world, id, text);
    assert!(
        console_command_answer(&tick)
            .0
            .is_some_and(|result| result > 0),
        "{text}: {:?}",
        console_command_answer(&tick)
    );
    tick
}

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, Vec3d) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    teleport(
        &mut world,
        &sender,
        &receiver,
        100,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    );
    for _ in 0..80 {
        world.tick(Vec::new());
    }
    let position = world.publication().players[0].position;
    (sender, receiver, world, position)
}

fn teleport(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    id: u64,
    position: Vec3d,
) {
    let tick = command(
        world,
        sender,
        receiver,
        id,
        &format!("tp alice {} {} {}", position.x, position.y, position.z),
    );
    let teleport_id = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } => Some(update.id),
            _ => None,
        })
        .expect("actual command teleport");
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
}

fn plate(powered: bool) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:oak_pressure_plate",
        &BTreeMap::from([("powered".into(), powered.to_string())]),
    )
    .unwrap()
}

#[test]
fn player_pose_scaled_edge_occupancy_keeps_a_real_plate_pressed_and_spectator_does_not() {
    for (scale, spectator, expected) in
        [(0.25, false, false), (1.0, false, true), (1.0, true, false)]
    {
        let (sender, receiver, mut world, position) = fixture();
        let pos = BlockPos {
            x: position.x.floor() as i32,
            y: position.y.floor() as i32,
            z: position.z.floor() as i32,
        };
        world.set_block(
            DIMENSION,
            BlockPos {
                y: pos.y - 1,
                ..pos
            },
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL,
        );
        command(
            &mut world,
            &sender,
            &receiver,
            1,
            &format!("attribute alice minecraft:scale base set {scale}"),
        );
        if spectator {
            command(
                &mut world,
                &sender,
                &receiver,
                2,
                "gamemode spectator alice",
            );
        }
        teleport(
            &mut world,
            &sender,
            &receiver,
            3,
            Vec3d {
                x: f64::from(pos.x) + 1.5,
                y: f64::from(pos.y),
                z: f64::from(pos.z) + 0.5,
            },
        );
        assert!(world.set_block(DIMENSION, pos, plate(false), SET_BLOCK_UPDATE_ALL));
        let (_, stand_uuid) = world.spawn_stored_entity_for_test(
            DIMENSION,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y),
                z: f64::from(pos.z) + 0.5,
            },
            EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
        );
        world.tick(Vec::new());
        assert_eq!(live_block(&mut world, pos), Some(plate(true)));
        command(
            &mut world,
            &sender,
            &receiver,
            4,
            &format!("tp {stand_uuid} {} {} {}", pos.x + 4, pos.y, pos.z),
        );
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    x: f64::from(pos.x) + 1.1,
                    y: f64::from(pos.y),
                    z: f64::from(pos.z) + 0.5,
                }),
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            }),
        );
        for _ in 0..25 {
            world.tick(Vec::new());
        }
        let projected = world.nearby_players_for_test(DIMENSION)[0];
        assert_eq!(projected.dimensions[0], 0.6_f32 * scale as f32);
        assert_eq!(projected.spectator, spectator);
        assert_eq!(
            live_block(&mut world, pos),
            Some(plate(expected)),
            "scale={scale}, spectator={spectator}, projected={projected:?}"
        );
    }
}

#[test]
fn player_pose_crouching_changes_real_lightning_vertical_search() {
    for (crouching, dead, expected_hit) in [
        (true, false, false),
        (false, false, true),
        (false, true, false),
    ] {
        let (sender, receiver, mut world, position) = fixture();
        command(
            &mut world,
            &sender,
            &receiver,
            1,
            "attribute alice minecraft:scale base set 2",
        );
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerInput(PlayerInput {
                shift: crouching,
                ..PlayerInput::default()
            }),
        );
        world.tick(Vec::new());
        if dead {
            command(&mut world, &sender, &receiver, 2, "kill alice");
        }
        let projected = world.nearby_players_for_test(DIMENSION)[0];
        assert_eq!(projected.dead_or_dying, dead);
        assert_eq!(
            world.player_pose_id_for_test(ConnectionId(1)),
            if crouching {
                VANILLA_ENTITY_POSE_CROUCHING_ID
            } else {
                VANILLA_ENTITY_POSE_STANDING_ID
            },
            "projected: {projected:?}"
        );
        assert_eq!(
            projected.dimensions,
            [1.2, if crouching { 3.0 } else { 3.6 }]
        );
        assert_eq!(
            projected.eye_height,
            f64::from(if crouching { 1.27_f32 } else { 1.62_f32 } * 2.0)
        );
        let before = world.player_health_for_test(ConnectionId(1));
        let (bolt, _) = world.spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
            Vec3d {
                y: position.y + 6.2,
                ..position
            },
            EntityData::LightningBolt(LightningBoltData::new(1, 1)),
        );
        world.tick(Vec::new());
        let EntityData::LightningBolt(data) = world
            .stored_entity_for_test(DIMENSION, bolt)
            .unwrap()
            .data()
        else {
            panic!("actual lightning entity");
        };
        assert_eq!(
            data.hit_entities().contains(&projected.entity_id),
            expected_hit
        );
        assert_eq!(
            world.player_health_for_test(ConnectionId(1)) < before,
            expected_hit
        );
    }
}
