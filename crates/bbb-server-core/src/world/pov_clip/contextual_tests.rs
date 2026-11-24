use super::*;
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::ServerWorldConfig;
use bbb_protocol::packets::ItemStackSummary;
use bbb_server_types::save::Java26BuiltinDimension;

#[test]
fn scaffolding_pov_uses_actual_main_hand_and_body_context_while_empty_visual_stays_stable() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let pos = BlockPos { x: 8, y: 70, z: 8 };
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_SCAFFOLDING_ID,
    )
    .unwrap();
    world
        .level_mut(Java26BuiltinDimension::Overworld)
        .chunks
        .set_block_state(pos.x, pos.y, pos.z, state)
        .unwrap();
    let mut player = world.levels.remove_player(ConnectionId(1)).unwrap();
    let chunks = &world.level(Java26BuiltinDimension::Overworld).chunks;
    let ray = |y| (Vec3d { x: 7.5, y, z: 8.5 }, Vec3d { x: 9.5, y, z: 8.5 });
    let (from, to) = ray(70.5);
    assert!(clip_player(
        chunks,
        &player,
        from,
        to,
        ClipFluid::None,
        ClipShape::Outline
    )
    .is_none());
    *player.inventory.selected_item_mut() = crate::tests::native_stack(ItemStackSummary {
        count: 1,
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SCAFFOLDING_ID),
        ..ItemStackSummary::empty()
    });
    assert_eq!(
        clip_player(
            chunks,
            &player,
            from,
            to,
            ClipFluid::None,
            ClipShape::Outline
        )
        .unwrap()
        .pos,
        pos
    );
    let (from, to) = ray(70.95);
    {
        let mut position = player.position();
        position.y = 71.0;
        player.set_position(position);
    }
    assert!(clip_player(
        chunks,
        &player,
        from,
        to,
        ClipFluid::None,
        ClipShape::Collider
    )
    .is_some());
    player.set_shared_flag(
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
        true,
    );
    assert!(clip_player(
        chunks,
        &player,
        from,
        to,
        ClipFluid::None,
        ClipShape::Collider
    )
    .is_none());
    player.set_shared_flag(
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
        false,
    );
    {
        let mut position = player.position();
        position.y = 70.5;
        player.set_position(position);
    }
    assert!(clip_player(
        chunks,
        &player,
        from,
        to,
        ClipFluid::None,
        ClipShape::Collider
    )
    .is_none());
    assert_eq!(
        clip_exact(chunks, from, to, ClipFluid::None, ClipShape::Visual)
            .unwrap()
            .unwrap()
            .pos,
        pos
    );
    *player.inventory.selected_item_mut() = crate::tests::native_stack(ItemStackSummary::empty());
    let hit = clip_player(
        chunks,
        &player,
        Vec3d {
            x: 7.0,
            y: 70.5,
            z: 8.5,
        },
        Vec3d {
            x: 9.0,
            y: 71.1,
            z: 8.5,
        },
        ClipFluid::None,
        ClipShape::Outline,
    )
    .unwrap();
    assert_eq!(hit.direction, bbb_protocol::packets::Direction::West);
    assert_eq!(hit.location.y, 70.875);
    assert!((hit.location.x - 8.25).abs() < 1.0e-12);
}
