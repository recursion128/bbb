use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_CRAFTING_TABLE_ID;
use bbb_protocol::packets::OpenScreen;
use bbb_server_types::stats::custom_stat;

use crate::connection::ClientboundPlayPacket;
use crate::inventory::{ContainerAnchor, MenuLayout};
use crate::world::{chest_menu_title, send, ServerWorld, WorldEvent};

use super::BlockUseContext;

pub(in crate::world) fn is_crafting_table(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id) == Some(VANILLA_BLOCK_CRAFTING_TABLE_ID)
}

/// `CraftingTableBlock.useWithoutItem` (`CraftingTableBlock.java:32-41`).
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    if open_menu(world, context, events) {
        world.levels[context.connection_id].award_stat_once(
            &mut world.scoreboard,
            custom_stat("minecraft:interact_with_crafting_table"),
        );
    }
    true
}

/// `CraftingTableBlock.getMenuProvider` (`CraftingTableBlock.java:44-48`).
pub(super) fn open_menu(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let resolver = world.item_stack_resolver();
    let connection_id = context.connection_id;
    let Some(container_id) = world.begin_container_open(connection_id, events) else {
        return false;
    };
    let prepared = world.levels[connection_id].inventory.prepare_reserved_menu(
        &resolver,
        Some(&mut world.maps),
        container_id,
        MenuLayout::CraftingTable,
        ContainerAnchor {
            positions: vec![context.update.hit.pos],
        },
        Vec::new(),
    );
    let Ok(Some(opened)) = prepared else {
        return false;
    };
    let Some(title) = chest_menu_title("container.crafting") else {
        return false;
    };
    events.push(send(
        connection_id,
        ClientboundPlayPacket::OpenScreen(OpenScreen {
            container_id: opened.container_id,
            menu_type_id: opened.menu_type_id,
            title,
        }),
    ));
    world.initialize_player_menu(connection_id, events);
    true
}
