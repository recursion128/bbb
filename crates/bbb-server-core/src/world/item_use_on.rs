//! `Item.useOn` for the items that change the clicked block -- tools that
//! convert it, honeycomb, water bottles, shears, igniters, bone meal and the
//! powder snow bucket -- or place an entity against it.
//!
//! The leg runs after the block's own `useItemOn` declined and before block
//! placement (`ServerPlayerGameMode.useItemOn`, `:386-401`). A new item owner
//! is a `item_use_on/<owner>.rs` file plus its arm in [`handler`].

pub(in crate::world) mod block_transform;
mod bone_meal;
mod compass;
mod end_crystal;
mod ender_eye;
mod firework_rocket;
mod hanging_entity;
mod igniter;
mod map;
pub(in crate::world) mod minecart;
mod potion;
mod shears;
mod solid_bucket;

use bbb_protocol::item_types::{vanilla_item_type, VanillaItemUseOnOverride};
use bbb_protocol::packets::{BlockPos, InteractionHand, ItemStackSummary, UseItemOn};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;

use crate::connection::ConnectionId;
use crate::world::{LevelId, ServerWorld, WorldEvent};

/// What one `useOn` sees: the clicking session, the click and the stack as it
/// was before the use.
pub(super) struct ItemUseOnContext<'a> {
    pub(super) connection_id: ConnectionId,
    pub(super) dimension: LevelId,
    pub(super) update: UseItemOn,
    pub(super) stack: &'a ItemStackSummary,
}

impl ItemUseOnContext<'_> {
    fn pos(&self) -> BlockPos {
        self.update.hit.pos
    }

    fn hand(&self) -> InteractionHand {
        self.update.hand
    }
}

/// True when the item consumed the click (`SUCCESS` / `SUCCESS_SERVER`).
type ItemUseOnHandler = fn(&mut ServerWorld, &ItemUseOnContext<'_>, &mut Vec<WorldEvent>) -> bool;

fn handler(owner: VanillaItemUseOnOverride) -> Option<ItemUseOnHandler> {
    use VanillaItemUseOnOverride as Owner;
    Some(match owner {
        Owner::AxeItem => block_transform::axe,
        Owner::HoeItem => block_transform::hoe,
        Owner::ShovelItem => block_transform::shovel,
        Owner::HoneycombItem => block_transform::honeycomb,
        Owner::PotionItem => potion::use_on,
        Owner::ShearsItem => shears::use_on,
        Owner::FlintAndSteelItem => igniter::flint_and_steel,
        Owner::FireChargeItem => igniter::fire_charge,
        Owner::BoneMealItem => bone_meal::use_on,
        Owner::SolidBucketItem => solid_bucket::use_on,
        Owner::HangingEntityItem => hanging_entity::use_on,
        Owner::MinecartItem => minecart::use_on,
        Owner::EnderEyeItem => ender_eye::use_on,
        Owner::FireworkRocketItem => firework_rocket::use_on,
        Owner::EndCrystalItem => end_crystal::use_on,
        Owner::CompassItem => compass::use_on,
        Owner::MapItem => map::use_on,
        // Earlier legs of `apply_use_item_on`.
        Owner::ArmorStandItem
        | Owner::BlockItem
        | Owner::BrushItem
        | Owner::DebugStickItem
        | Owner::LeadItem
        | Owner::SpawnEggItem => return None,
        // `PlaceOnWaterBlockItem.useOn` answers `PASS` (`PlaceOnWaterBlockItem.java:18-20`); its
        // placement is the `use` the client sends next.
        Owner::PlaceOnWaterBlockItem => return None,
    })
}

/// `ItemStack.useOn` (`ItemStack.java:358-372`) for the owners above,
/// including its `ITEM_USED` award.
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    connection_id: ConnectionId,
    update: UseItemOn,
    stack: &ItemStackSummary,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    let Some(handler) = vanilla_item_type(item_id)
        .and_then(|item| item.use_on_override)
        .and_then(handler)
    else {
        return false;
    };
    let context = ItemUseOnContext {
        connection_id,
        dimension: world.player_level_id(connection_id),
        update,
        stack,
    };
    if !handler(world, &context, events) {
        return false;
    }
    world.levels[connection_id].award_stat_once(
        &mut world.scoreboard,
        StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
    );
    true
}

impl ServerWorld {
    /// `ItemStack.shrink(1)` on the used stack, which
    /// `ServerPlayerGameMode.useItemOn` (`:389-393`) undoes for infinite
    /// materials.
    fn shrink_used_stack(&mut self, context: &ItemUseOnContext<'_>) {
        let player = &mut self.levels[context.connection_id];
        let infinite = player.abilities.instabuild;
        player
            .inventory
            .consume_in_hand(context.hand(), 1, infinite);
    }

    fn player_entity_id(&self, context: &ItemUseOnContext<'_>) -> i32 {
        self.levels[context.connection_id].entity_id()
    }
}
