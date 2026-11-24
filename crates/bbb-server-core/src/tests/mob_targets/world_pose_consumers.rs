use super::*;
use crate::entity::BoatData;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_SNOWBALL_ID,
};
use bbb_protocol::item_types::{VANILLA_ITEM_SNOWBALL_ID, VANILLA_ITEM_STONE_ID};

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

fn scale_and_crouch(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    scale: f64,
    crouch: bool,
) {
    command(
        world,
        sender,
        receiver,
        1,
        &format!("attribute alice minecraft:scale base set {scale}"),
    );
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::PlayerInput(PlayerInput {
            shift: crouch,
            ..PlayerInput::default()
        }),
    );
    world.tick(Vec::new());
}
fn stack(item_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}
fn took(tick: &TickResult, item: i32) -> bool {
    tick.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::TakeItemEntity(take), .. } if take.item_id == item && take.amount == 1))
}

#[test]
fn world_pose_consumers_throw_from_scaled_standing_and_crouching_eyes() {
    for crouch in [false, true] {
        let (sender, receiver, mut world, _) = fixture();
        scale_and_crouch(&mut world, &sender, &receiver, 2.0, crouch);
        world.set_inventory_slot_for_test(ConnectionId(1), 0, stack(VANILLA_ITEM_SNOWBALL_ID));
        let at = world.publication().players[0].position;
        let tick = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::UseItem(UseItem {
                hand: InteractionHand::MainHand,
                sequence: 7,
                y_rot: 0.0,
                x_rot: 0.0,
            }),
        );
        let adds: Vec<_> = tick
            .events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::AddEntity(add),
                } if add.entity_type_id == VANILLA_ENTITY_TYPE_SNOWBALL_ID => Some(add),
                _ => None,
            })
            .collect();
        assert_eq!(adds.len(), 1, "snowball AddEntity count");
        let add = adds[0];
        assert_eq!(
            add.position,
            Vec3d {
                y: at.y + f64::from((if crouch { 1.27_f32 } else { 1.62_f32 }) * 2.0)
                    - f64::from(0.1_f32),
                ..at
            }
        );
        assert_eq!(add.data, entity_id_of(&world, 1));
        assert_eq!(
            world
                .player_inventory_slot_for_test(ConnectionId(1), 0)
                .count,
            0
        );
        let blur = 0.0172275 * 1.5 + 1e-9;
        assert!(add.delta_movement.x.abs() <= blur);
        assert!(add.delta_movement.y.abs() <= blur);
        assert!((add.delta_movement.z - 1.5).abs() <= blur);
    }
}

#[test]
fn world_pose_consumers_crouching_narrows_the_actual_unmounted_pickup_height() {
    for (crouch, expected) in [(true, false), (false, true)] {
        let (sender, receiver, mut world, _) = fixture();
        scale_and_crouch(&mut world, &sender, &receiver, 0.5, crouch);
        let at = world.publication().players[0].position;
        let item = world.spawn_ground_item_for_test(
            DIMENSION,
            Vec3d {
                y: at.y + 1.35,
                ..at
            },
            &stack(VANILLA_ITEM_STONE_ID),
        );
        let tick = world.tick(Vec::new());
        assert_eq!(took(&tick, item), expected, "crouch={crouch}");
        assert_eq!(
            world.stored_entity_for_test(DIMENSION, item).is_none(),
            expected
        );
        assert_eq!(
            world
                .player_inventory_slot_for_test(ConnectionId(1), 0)
                .count,
            i32::from(expected)
        );
    }
}

#[test]
fn world_pose_consumers_real_boat_ride_extends_pickup_width_without_vertical_inflation() {
    let (sender, receiver, mut world, at) = fixture();
    scale_and_crouch(&mut world, &sender, &receiver, 0.25, false);
    let (boat, _) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        at,
        EntityData::Boat(BoatData::new()),
    );
    let mounted = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: boat,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    let teleport_id = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .expect("real boat admission positions its rider");
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(boat)
    );
    let boat_position = world
        .stored_entity_for_test(DIMENSION, boat)
        .unwrap()
        .position();
    let player = world.nearby_players_for_test(DIMENSION)[0];
    let union_item = world.spawn_ground_item_for_test(
        DIMENSION,
        Vec3d {
            x: boat_position.x + 1.6,
            y: boat_position.y + 0.1,
            z: boat_position.z,
        },
        &stack(VANILLA_ITEM_STONE_ID),
    );
    let upper_y = world
        .stored_entity_for_test(DIMENSION, boat)
        .unwrap()
        .bounding_box()
        .max_y
        .max(player.bounding_box().max_y);
    let above_item = world.spawn_ground_item_for_test(
        DIMENSION,
        Vec3d {
            y: upper_y + 0.1,
            ..player.position
        },
        &stack(VANILLA_ITEM_SNOWBALL_ID),
    );
    let tick = world.tick(Vec::new());
    assert!(
        took(&tick, union_item),
        "vehicle union reaches beyond the tiny rider"
    );
    assert!(
        !took(&tick, above_item),
        "mounted pickup has no vertical inflation"
    );
    assert!(world
        .stored_entity_for_test(DIMENSION, union_item)
        .is_none());
    assert!(world
        .stored_entity_for_test(DIMENSION, above_item)
        .is_some());
    assert_eq!(
        world
            .player_inventory_slot_for_test(ConnectionId(1), 0)
            .item_id,
        Some(VANILLA_ITEM_STONE_ID)
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(ConnectionId(1), 0)
            .count,
        1
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(boat)
    );
}
