use super::*;
use crate::connection::ConnectionId;
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::mailbox::{WorldCommandReceiver, WorldCommandSender};
use bbb_protocol::packets::{BlockPos, ItemStackSummary};

fn joined() -> (NativeLevel, WorldCommandSender, WorldCommandReceiver) {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "viewer", 1);
    native.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    (native, sender, receiver)
}

#[test]
fn gaze_reverse_ray_uses_collider_or_visual() {
    let (mut native, sender, receiver) = joined();
    let p = native.world.nearby_players_for_test(DIMENSION)[0];
    let body = Vec3d {
        z: 8.5,
        ..p.position
    };
    let heights = [p.position.y + p.eye_height];
    assert_eq!(
        native.with_level(|level| is_looking_at_me(level, body, &heights, &p, 0.025, true, false)),
        Some(true)
    );
    native.set(BlockPos { x: 0, y: 65, z: 4 }, state("minecraft:glass"));
    assert_eq!(
        native.with_level(|level| is_looking_at_me(level, body, &heights, &p, 0.025, true, false)),
        Some(false)
    );
    assert_eq!(
        native.with_level(|level| is_looking_at_me(level, body, &heights, &p, 0.025, true, true)),
        Some(true)
    );
    native.set(BlockPos { x: 0, y: 65, z: 4 }, state("minecraft:air"));
    sender
        .try_send(crate::world::WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: bbb_protocol::packets::PlayServerbound::MovePlayer(
                bbb_protocol::packets::MovePlayer {
                    position: None,
                    rotation: Some(bbb_protocol::packets::PlayerRotation {
                        y_rot: 180.0,
                        x_rot: 0.0,
                    }),
                    on_ground: true,
                    horizontal_collision: false,
                },
            ),
        })
        .unwrap();
    native.world.tick(receiver.drain());
    let p = native.world.nearby_players_for_test(DIMENSION)[0];
    assert_eq!(
        native.with_level(|level| is_looking_at_me(level, body, &heights, &p, 0.025, true, false)),
        Some(false)
    );
}

#[test]
fn gaze_distance_adjusted_zero_and_range_follow_native_normalization() {
    let (mut native, _, _) = joined();
    let p = native.world.nearby_players_for_test(DIMENSION)[0];
    let heights = [p.position.y + p.eye_height];
    native.with_level(|level| {
        assert_eq!(
            is_looking_at_me(level, p.position, &heights, &p, 0.025, true, false),
            Some(true)
        );
        assert_eq!(
            is_looking_at_me(level, p.position, &heights, &p, 0.5, false, true),
            Some(false)
        );
        assert_eq!(
            is_looking_at_me(
                level,
                Vec3d {
                    z: 129.0,
                    ..p.position
                },
                &heights,
                &p,
                0.025,
                true,
                false
            ),
            Some(false)
        );
    });
}

#[test]
fn gaze_player_powder_snow_blocks_only_the_collider_ray() {
    let (mut native, _, receiver) = joined();
    native.set(BlockPos { x: 0, y: 65, z: 0 }, state("minecraft:stone"));
    native.world.tick(receiver.drain());
    assert_eq!(
        native.world.player_pose_id_for_test(ConnectionId(1)),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID
    );
    let p = native.world.nearby_players_for_test(DIMENSION)[0];
    let body = Vec3d {
        z: 8.5,
        ..p.position
    };
    let heights = [p.position.y + p.eye_height];
    native.set(
        BlockPos { x: 0, y: 64, z: 4 },
        state("minecraft:powder_snow"),
    );
    assert_eq!(
        native.with_level(|level| is_looking_at_me(level, body, &heights, &p, 0.025, true, false)),
        Some(true)
    );
    native
        .world
        .set_player_ground_for_test(ConnectionId(1), false, 3.0);
    let p = native.world.nearby_players_for_test(DIMENSION)[0];
    assert_eq!(
        native.with_level(|level| is_looking_at_me(level, body, &heights, &p, 0.025, true, false)),
        Some(false)
    );
    assert_eq!(
        native.with_level(|level| is_looking_at_me(level, body, &heights, &p, 0.025, true, true)),
        Some(true)
    );
}

#[test]
fn disguise_tag_decides_whether_the_actual_head_item_disguises() {
    let (mut native, _, _) = joined();
    let p = native.world.nearby_players_for_test(DIMENSION)[0];
    assert!(native.with_level(|level| not_wearing_disguise(level, &p)));
    let inventory = native.world.player_inventory_for_test(ConnectionId(1));
    let head = inventory
        .equipment_slots()
        .into_iter()
        .find(|(slot, _)| *slot == bbb_protocol::equipment_slots::EquipmentSlot::Head)
        .unwrap()
        .1;
    *inventory.slot_mut(head).unwrap() = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_CARVED_PUMPKIN_ID),
        count: 1,
        component_patch: Default::default(),
    });
    let p = native.world.nearby_players_for_test(DIMENSION)[0];
    for (members, disguised) in [
        (vec!["minecraft:carved_pumpkin".to_owned()], true),
        (Vec::new(), false),
    ] {
        native.install_tags(crate::registry_tags::RegistryTagSnapshot::new([(
            "minecraft:item".to_owned(),
            "minecraft:gaze_disguise_equipment".to_owned(),
            members,
        )]));
        assert_eq!(
            native.with_level(|level| not_wearing_disguise(level, &p)),
            !disguised
        );
    }
}
