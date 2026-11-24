//! The tool and wax conversions: `AxeItem.useOn` (strip, scrape, wax off),
//! `HoeItem.useOn`, `ShovelItem.useOn` and `HoneycombItem.useOn`, each one
//! lookup into a generated source-to-target table.

use bbb_protocol::block_types::VANILLA_BLOCK_ROOTED_DIRT_ID;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::item_types::{
    VanillaBlockTransform, VANILLA_AXE_SCRAPEABLES, VANILLA_AXE_STRIPPABLES, VANILLA_AXE_WAX_OFF,
    VANILLA_HOE_TILLABLES, VANILLA_HONEYCOMB_WAXABLES, VANILLA_ITEM_HANGING_ROOTS_ID,
    VANILLA_SHOVEL_FLATTENABLES,
};
use bbb_protocol::packets::{Direction, InteractionHand, ItemStackSummary};
use bbb_server_types::block_queries::block_type_of;
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::block_state_is_air;

use super::ItemUseOnContext;
use crate::block_behavior::weathering::with_properties_of;
use crate::block_behavior::{campfire, chest, default_block_state_id};
use crate::inventory::has_component;
use crate::level_event::{PendingLevelEvent, VanillaGameEvent};
use crate::world::{ServerWorld, WorldEvent};

/// `Block.UPDATE_ALL_IMMEDIATE`, the flags every conversion writes with.
const UPDATE_ALL_IMMEDIATE: i32 = 11;

/// One generated conversion table and what applying it plays.
struct Conversion {
    table: &'static [VanillaBlockTransform],
    sound: Option<&'static str>,
    level_event: Option<i32>,
    /// `withPropertiesOf` (strip keeps `AXIS`, the copper chains keep every
    /// shared property); otherwise the target's default state.
    keep_properties: bool,
}

/// `AxeItem.evaluateNewBlockState` (`AxeItem.java:94-115`), in its order.
const AXE: &[Conversion] = &[
    Conversion {
        table: VANILLA_AXE_STRIPPABLES,
        sound: Some("minecraft:item.axe.strip"),
        level_event: None,
        keep_properties: true,
    },
    Conversion {
        table: VANILLA_AXE_SCRAPEABLES,
        sound: Some("minecraft:item.axe.scrape"),
        level_event: Some(3005),
        keep_properties: true,
    },
    Conversion {
        table: VANILLA_AXE_WAX_OFF,
        sound: Some("minecraft:item.axe.wax_off"),
        level_event: Some(3004),
        keep_properties: true,
    },
];

/// `HoneycombItem.useOn` (`HoneycombItem.java:141-164`).
const WAX_ON: Conversion = Conversion {
    table: VANILLA_HONEYCOMB_WAXABLES,
    sound: None,
    level_event: Some(3003),
    keep_properties: true,
};

/// `ShovelItem.useOn` (`ShovelItem.java:37-73`).
const FLATTEN: Conversion = Conversion {
    table: VANILLA_SHOVEL_FLATTENABLES,
    sound: Some("minecraft:item.shovel.flatten"),
    level_event: None,
    keep_properties: false,
};

const HOE_TILL: &str = "minecraft:item.hoe.till";

impl Conversion {
    fn converted(&self, state: i32) -> Option<i32> {
        let block = block_type_of(state)?;
        let entry = self
            .table
            .iter()
            .find(|entry| entry.source_block_type_id == block)?;
        if self.keep_properties {
            with_properties_of(entry.target_block_type_id, state)
        } else {
            Some(default_block_state_id(entry.target_block_type_id))
        }
    }
}

pub(super) fn axe(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    if blocking_item_use_intent(world, context) {
        return false;
    }
    let state = world.block_state_or_air(context.dimension, context.pos());
    let Some((conversion, new_state)) = AXE
        .iter()
        .find_map(|conversion| Some((conversion, conversion.converted(state)?)))
    else {
        return false;
    };
    // `AxeItem.java:73-75`: before the write, so the location sees the old block.
    world.trigger_item_used_on_location(
        context.connection_id,
        "minecraft:item_used_on_block",
        context.pos(),
        context.stack,
        events,
    );
    world.apply_block_conversion(
        context,
        conversion.sound,
        conversion.level_event,
        new_state,
        events,
    );
    world.hurt_and_break_in_hand(context.connection_id, context.hand(), 1, events);
    true
}

/// `HoneycombItem.getWaxed`.
pub(in crate::world) fn waxed_state(state: i32) -> Option<i32> {
    WAX_ON.converted(state)
}

pub(super) fn honeycomb(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let state = world.block_state_or_air(context.dimension, context.pos());
    let Some(new_state) = waxed_state(state) else {
        return false;
    };
    // `HoneycombItem.java:148-150`.
    world.trigger_item_used_on_location(
        context.connection_id,
        "minecraft:item_used_on_block",
        context.pos(),
        context.stack,
        events,
    );
    world.shrink_used_stack(context);
    world.apply_block_conversion(context, WAX_ON.sound, WAX_ON.level_event, new_state, events);
    true
}

pub(super) fn shovel(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    if context.update.hit.direction == Direction::Down {
        return false;
    }
    let state = world.block_state_or_air(context.dimension, context.pos());
    let Some(new_state) = FLATTEN
        .converted(state)
        .filter(|_| air_above(world, context))
    else {
        // `:51-58`: a lit campfire is doused instead.
        if !campfire::is_campfire(state) {
            return false;
        }
        let source = Some(world.player_entity_id(context));
        let doused = world.with_level(context.dimension, |level| {
            campfire::shovel_dowse(level, context.pos(), state, source)
        });
        if doused {
            world.hurt_and_break_in_hand(context.connection_id, context.hand(), 1, events);
        }
        return doused;
    };
    world.apply_block_conversion(
        context,
        FLATTEN.sound,
        FLATTEN.level_event,
        new_state,
        events,
    );
    world.hurt_and_break_in_hand(context.connection_id, context.hand(), 1, events);
    true
}

/// `HoeItem.useOn` (`HoeItem.java:44-68`) over `TILLABLES` (`:24-36`).
pub(super) fn hoe(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let state = world.block_state_or_air(context.dimension, context.pos());
    let Some(tillable) = block_type_of(state).and_then(|block| {
        VANILLA_HOE_TILLABLES
            .iter()
            .find(|entry| entry.transform.source_block_type_id == block)
    }) else {
        return false;
    };
    // `onlyIfAirAbove` (`:85-87`).
    if tillable.requires_air_above
        && (context.update.hit.direction == Direction::Down || !air_above(world, context))
    {
        return false;
    }
    let new_state = default_block_state_id(tillable.transform.target_block_type_id);
    world.apply_block_conversion(context, Some(HOE_TILL), None, new_state, events);
    // `changeIntoStateAndDropItem` (`:77-83`).
    if tillable.transform.source_block_type_id == VANILLA_BLOCK_ROOTED_DIRT_ID {
        let roots = ItemStackSummary {
            item_id: Some(VANILLA_ITEM_HANGING_ROOTS_ID),
            count: 1,
            ..ItemStackSummary::empty()
        };
        let pos = relative(context.pos(), context.update.hit.direction);
        world.with_level(context.dimension, |level| level.pop_resource(pos, &roots));
    }
    world.hurt_and_break_in_hand(context.connection_id, context.hand(), 1, events);
    true
}

/// `AxeItem.playerHasBlockingItemUseIntent` (`AxeItem.java:87-92`).
fn blocking_item_use_intent(world: &ServerWorld, context: &ItemUseOnContext<'_>) -> bool {
    let player = &world.levels[context.connection_id];
    context.hand() == InteractionHand::MainHand
        && has_component(
            player.inventory.off_hand_stack().summary(),
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        )
        .unwrap_or(false)
        && !player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN)
}

fn air_above(world: &mut ServerWorld, context: &ItemUseOnContext<'_>) -> bool {
    block_state_is_air(
        world.block_state_or_air(context.dimension, relative(context.pos(), Direction::Up)),
    )
}

impl ServerWorld {
    /// The shared tail: the sound and level event the user's client already
    /// played for itself, the write and the `BLOCK_CHANGE` game event; a
    /// particle conversion of a double chest repeats both on the other half
    /// (`AxeItem.java:122-126`, `HoneycombItem.java:156-160`).
    fn apply_block_conversion(
        &mut self,
        context: &ItemUseOnContext<'_>,
        sound: Option<&str>,
        level_event: Option<i32>,
        new_state: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let pos = context.pos();
        let except = Some(context.connection_id);
        let old_state = self.block_state_or_air(context.dimension, pos);
        let neighbor = level_event
            .filter(|_| {
                matches!(
                    chest::block_type(old_state),
                    Some(chest::ChestHalf::First | chest::ChestHalf::Second)
                )
            })
            .and_then(|_| chest::connected_direction(old_state))
            .map(|direction| relative(pos, direction));
        if let Some(sound) = sound {
            self.play_block_sound_as(context.dimension, pos, sound, (1.0, 1.0), except, events);
        }
        let mut level_event_at = |world: &mut Self, pos| {
            if let Some(event_type) = level_event {
                world.broadcast_level_event_by_key(
                    &context.dimension,
                    except,
                    PendingLevelEvent {
                        event_type,
                        pos,
                        data: 0,
                    },
                    events,
                );
            }
        };
        level_event_at(self, pos);
        if let Some(neighbor) = neighbor {
            level_event_at(self, neighbor);
        }
        self.set_block(context.dimension, pos, new_state, UPDATE_ALL_IMMEDIATE);
        let source = self.player_entity_id(context);
        let neighbor = neighbor.map(|neighbor| {
            (
                neighbor,
                self.block_state_or_air(context.dimension, neighbor),
            )
        });
        let sink = &mut self.level_mut_by_id(&context.dimension).event_sink;
        sink.game_event_with_source(
            VanillaGameEvent::BLOCK_CHANGE,
            pos,
            Some(source),
            Some(new_state),
        );
        if let Some((neighbor, state)) = neighbor {
            sink.game_event_with_source(
                VanillaGameEvent::BLOCK_CHANGE,
                neighbor,
                Some(source),
                Some(state),
            );
        }
    }
}
