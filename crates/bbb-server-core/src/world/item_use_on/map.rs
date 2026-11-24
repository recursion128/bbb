//! Filled-map banner interaction.

use super::ItemUseOnContext;
use crate::world::{ServerWorld, WorldEvent};

pub(super) fn use_on(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    // MapItem.java:325-337: missing map data still consumes a banner click.
    world.with_level(context.dimension, |level| {
        let pos = context.pos();
        if !level.block_state_in_tag(level.block_state(pos), "minecraft:banners") {
            return false;
        }
        let Some(map_id) = context.stack.component_patch.map_id else {
            return true;
        };
        level
            .loot_level_facts
            .maps
            .as_ref()
            .expect("world level context has canonical maps")
            .borrow_mut()
            .toggle_banner(map_id, level, pos)
            .unwrap_or(true)
    })
}
