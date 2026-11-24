use crate::connection::ConnectionId;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_DYING_ID,
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
    VANILLA_ENTITY_POSE_SWIMMING_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_types::VANILLA_ITEM_LEATHER_BOOTS_ID;
use bbb_server_types::save::Java26BuiltinDimension;

use crate::mailbox::world_command_mailbox;
use crate::tests::join_loaded;
use crate::world::{ServerWorld, ServerWorldConfig};

#[test]
fn nearby_player_projection_uses_live_pose_scale_and_collision_facts() {
    let (sender, receiver) =
        world_command_mailbox(crate::mailbox::WorldMailboxLimits::default()).unwrap();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let dimension = Java26BuiltinDimension::Overworld.into();
    let connection = ConnectionId(1);
    let player = world.levels.player_mut(connection).unwrap();
    player.block_contact.fall_distance = 3.25;
    player
        .attributes
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap()
        .set_base_value(2.0);
    let feet = player
        .inventory
        .equipment_slots()
        .into_iter()
        .find(|(slot, _)| *slot == EquipmentSlot::Feet)
        .unwrap()
        .1;
    *player.inventory.slot_mut(feet).unwrap() =
        crate::tests::native_stack(bbb_protocol::packets::ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_LEATHER_BOOTS_ID),
            count: 1,
            ..bbb_protocol::packets::ItemStackSummary::empty()
        });
    for (pose, expected) in [
        (VANILLA_ENTITY_POSE_STANDING_ID, 1.62_f32 * 2.0),
        (VANILLA_ENTITY_POSE_CROUCHING_ID, 1.27_f32 * 2.0),
        (VANILLA_ENTITY_POSE_SWIMMING_ID, 0.4_f32 * 2.0),
        (VANILLA_ENTITY_POSE_SLEEPING_ID, 0.2_f32),
        (VANILLA_ENTITY_POSE_DYING_ID, 1.62_f32),
    ] {
        let player = world.levels.player_mut(connection).unwrap();
        player.set_pose(pose);
        assert_eq!(player.eye_height().unwrap(), expected);
        assert_eq!(
            player.eye_position().unwrap().y,
            player.position().y + f64::from(expected)
        );
        let projected = world.nearby_players_by_key(&dimension);
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].eye_height, f64::from(expected));
        let facts = projected[0].collision.unwrap();
        assert_eq!(facts.fall_distance, 3.25);
        assert_eq!(facts.feet_item_id, Some(VANILLA_ITEM_LEATHER_BOOTS_ID));
    }
    *world.levels[connection].inventory.slot_mut(feet).unwrap() =
        bbb_server_types::item_stack::RuntimeItemStack::empty();
    assert_eq!(
        world.nearby_players_by_key(&dimension)[0]
            .collision
            .unwrap()
            .feet_item_id,
        None
    );
}

mod desired_pose;
