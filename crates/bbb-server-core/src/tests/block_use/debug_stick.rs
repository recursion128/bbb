//! Game-master clicks in an operator's creative hand: `DebugStickItem`
//! (`DebugStickItem.java:29-104`) and the light block's server swing.

use super::*;

use crate::connection::ConnectionId;
use bbb_protocol::packets::{ItemStackSummary, PlayerAction, PlayerActionKind};

const AT: BlockPos = BlockPos { x: 0, y: 64, z: 1 };

fn stairs(facing: &str, half: &str) -> i32 {
    let properties = [
        ("facing", facing),
        ("half", half),
        ("shape", "straight"),
        ("waterlogged", "false"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_string(), value.to_string()))
    .collect();
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:oak_stairs",
        &properties,
    )
    .expect("a built-in stairs state")
}

fn overlay_messages(result: &TickResult) -> Vec<String> {
    result
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SystemChat(chat),
            } if chat.overlay => Some(format!("{:?}", chat.content)),
            _ => None,
        })
        .collect()
}

fn left_click(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: AT,
            direction: Direction::Up,
            sequence,
        }),
    )
}

/// Right click cycles the first property, left click selects the next one
/// into `debug_stick_state` without breaking the block, and the next right
/// click cycles that selection.
#[test]
fn an_operators_debug_stick_cycles_and_selects_block_properties() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.admin_lists = operator_config(&[("alice", 1)]).admin_lists;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Creative,
        PlayerAbilities::for_game_type(GameType::Creative),
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DEBUG_STICK_ID),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        AT,
        stairs("north", "bottom"),
        SET_BLOCK_UPDATE_ALL,
    ));

    let cycled = click_use(&sender, &receiver, &mut world, AT, 0.5, 1);

    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, AT),
        Some(stairs("south", "bottom"))
    );
    let messages = overlay_messages(&cycled);
    assert!(
        messages.len() == 1 && messages[0].contains("item.minecraft.debug_stick.update"),
        "{messages:?}"
    );

    left_click(&sender, &receiver, &mut world, 2);
    let selected = left_click(&sender, &receiver, &mut world, 3);

    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, AT),
        Some(stairs("south", "bottom"))
    );
    let messages = overlay_messages(&selected);
    assert!(
        messages.len() == 1 && messages[0].contains("item.minecraft.debug_stick.select"),
        "{messages:?}"
    );
    let stairs_block = bbb_protocol::block_state_types::vanilla_block_state_block_type_id(stairs(
        "north", "bottom",
    ))
    .unwrap();
    assert_eq!(
        world
            .player_inventory_slot_for_test(ConnectionId(1), 0)
            .component_patch
            .debug_stick_properties,
        vec![bbb_protocol::packets::DebugStickPropertySummary {
            block_registry_id: stairs_block,
            property: "half".to_string(),
        }]
    );

    click_use(&sender, &receiver, &mut world, AT, 0.5, 4);

    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, AT),
        Some(stairs("south", "top"))
    );
}
