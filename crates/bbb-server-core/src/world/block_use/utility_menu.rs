//! The blocks that open a utility menu: anvils, the beacon, the cartography,
//! enchanting and smithing tables, the grindstone, the loom and the
//! stonecutter.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_ANVIL_ID, VANILLA_BLOCK_BEACON_ID, VANILLA_BLOCK_CARTOGRAPHY_TABLE_ID,
    VANILLA_BLOCK_CHIPPED_ANVIL_ID, VANILLA_BLOCK_DAMAGED_ANVIL_ID,
    VANILLA_BLOCK_ENCHANTING_TABLE_ID, VANILLA_BLOCK_GRINDSTONE_ID, VANILLA_BLOCK_LOOM_ID,
    VANILLA_BLOCK_SMITHING_TABLE_ID, VANILLA_BLOCK_STONECUTTER_ID,
};
use bbb_protocol::packets::OpenScreen;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::custom_stat;

use crate::connection::ClientboundPlayPacket;
use crate::inventory::{ContainerAnchor, MenuLayout, UtilityMenuKind};
use crate::world::{chest_menu_title, send, ServerWorld, WorldEvent};

use super::BlockUseContext;

/// One family: its menu, the default title and the `useWithoutItem` statistic.
struct UtilityBlock {
    kind: UtilityMenuKind,
    title: &'static str,
    stat: Option<&'static str>,
    /// The beacon and enchanting table open over their block entity, whose
    /// custom name is the title (`BeaconBlock.java:47-56`,
    /// `EnchantingTableBlock.java:103-112`).
    named_block_entity: bool,
}

/// `*Block.useWithoutItem` and `getMenuProvider` of each family
/// (`AnvilBlock.java:58-74`, `GrindstoneBlock.java:74-90`,
/// `LoomBlock.java:34-50`, `SmithingTableBlock.java:32-48`,
/// `StonecutterBlock.java:49-65`, `CartographyTableBlock.java:33-49`).
fn utility_block(block_state_id: i32) -> Option<UtilityBlock> {
    let block = |kind, title, stat, named_block_entity| {
        Some(UtilityBlock {
            kind,
            title,
            stat,
            named_block_entity,
        })
    };
    match vanilla_block_state_block_type_id(block_state_id)? {
        VANILLA_BLOCK_ANVIL_ID
        | VANILLA_BLOCK_CHIPPED_ANVIL_ID
        | VANILLA_BLOCK_DAMAGED_ANVIL_ID => block(
            UtilityMenuKind::Anvil,
            "container.repair",
            Some("minecraft:interact_with_anvil"),
            false,
        ),
        VANILLA_BLOCK_BEACON_ID => block(
            UtilityMenuKind::Beacon,
            "container.beacon",
            Some("minecraft:interact_with_beacon"),
            true,
        ),
        VANILLA_BLOCK_CARTOGRAPHY_TABLE_ID => block(
            UtilityMenuKind::CartographyTable,
            "container.cartography_table",
            Some("minecraft:interact_with_cartography_table"),
            false,
        ),
        VANILLA_BLOCK_ENCHANTING_TABLE_ID => block(
            UtilityMenuKind::Enchantment,
            "container.enchant",
            None,
            true,
        ),
        VANILLA_BLOCK_GRINDSTONE_ID => block(
            UtilityMenuKind::Grindstone,
            "container.grindstone_title",
            Some("minecraft:interact_with_grindstone"),
            false,
        ),
        VANILLA_BLOCK_LOOM_ID => block(
            UtilityMenuKind::Loom,
            "container.loom",
            Some("minecraft:interact_with_loom"),
            false,
        ),
        VANILLA_BLOCK_SMITHING_TABLE_ID => block(
            UtilityMenuKind::Smithing,
            "container.upgrade",
            Some("minecraft:interact_with_smithing_table"),
            false,
        ),
        VANILLA_BLOCK_STONECUTTER_ID => block(
            UtilityMenuKind::Stonecutter,
            "container.stonecutter",
            Some("minecraft:interact_with_stonecutter"),
            false,
        ),
        _ => None,
    }
}

pub(in crate::world) fn is_utility_menu_block(block_state_id: i32) -> bool {
    utility_block(block_state_id).is_some()
}

/// Whether `block_state_id` still opens `kind`, the block half of each
/// menu's `stillValid`.
pub(in crate::world) fn opens_utility_menu(block_state_id: i32, kind: UtilityMenuKind) -> bool {
    utility_block(block_state_id).is_some_and(|block| block.kind == kind)
}

pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let Some(block) = utility_block(context.block_state_id) else {
        return true;
    };
    // `BeaconBlockEntity.createMenu` (`BeaconBlockEntity.java:318-325`) refuses a
    // player without the lock key; `BeaconBlock.useWithoutItem` still awards the stat.
    let refused = block.kind == UtilityMenuKind::Beacon
        && world
            .get_block_entity(context.dimension, context.update.hit.pos)
            .cloned()
            .is_some_and(|beacon| {
                !world.container_menu_may_open(
                    context.connection_id,
                    context.dimension,
                    &[beacon],
                    block.title,
                    events,
                )
            });
    if refused || open_menu(world, context, events) {
        let stat = block.stat;
        if let Some(stat) = stat {
            world.levels[context.connection_id]
                .award_stat_once(&mut world.scoreboard, custom_stat(stat));
        }
    }
    true
}

pub(super) fn open_menu(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let resolver = world.item_stack_resolver();
    let Some(block) = utility_block(context.block_state_id) else {
        return false;
    };
    let pos = context.update.hit.pos;
    let title = if block.named_block_entity {
        let Some(entity) = world.get_block_entity(context.dimension, pos) else {
            return false;
        };
        entity
            .nameable_custom_name()
            .flatten()
            .and_then(|name| {
                bbb_protocol::RichTextComponent::try_from_component_nbt(name.clone()).ok()
            })
            .or_else(|| chest_menu_title(block.title))
    } else {
        chest_menu_title(block.title)
    };
    let Some(title) = title else {
        return false;
    };
    let connection_id = context.connection_id;
    let Some(container_id) = world.begin_container_open(connection_id, events) else {
        return false;
    };
    let facts = world.utility_menu_facts();
    let inventory = &mut world.levels[connection_id].inventory;
    let prepared = inventory.prepare_reserved_menu(
        &resolver,
        None,
        container_id,
        MenuLayout::Utility(block.kind),
        ContainerAnchor {
            positions: vec![pos],
        },
        vec![RuntimeItemStack::empty(); block.kind.container_size()],
    );
    let Ok(Some(opened)) = prepared else {
        return false;
    };
    inventory.attach_utility_facts(facts);
    let seed = world.levels[connection_id].experience.enchantment_seed();
    world.levels[connection_id]
        .inventory
        .set_enchantment_seed(seed);
    world.refresh_utility_menu(connection_id);
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
