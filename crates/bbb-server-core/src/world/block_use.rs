//! Using an item on a block, and what each block type answers: the
//! `useItemOn` / `useWithoutItem` arms of every block this world serves, the
//! placements they turn into, and the sounds they play.
//!
//! This file is the `ServerPlayerGameMode.useItemOn` skeleton and the table
//! hung off it. [`ServerWorld::use_clicked_block`] is the skeleton — the
//! spectator arm, the secondary-use refusal, the three-way
//! [`BlockUseResult`], the main-hand-only empty-hand leg and the two generic
//! [`crate::block_behavior`] paths it ends in — and it names no block at all.
//! Which block answers which of its three legs is the
//! [`block_use_dispatch!`] table, one row per block.
//!
//! Giving a new block a right-click behaviour is a new
//! `world/block_use/<block>.rs` holding its bodies and the adapters the table
//! calls, its `mod` line, and one new row naming that module, the predicate
//! that recognises the block -- an `is_*` from [`crate::block_behavior`], or
//! one the module writes itself when recognising the block needs more than a
//! class tag -- and which of the three legs it answers. No leg of the
//! skeleton is edited for it, and neither is any other block's file. That is
//! the point: the three `if`-chains this replaced were three edits in one
//! 2000-line file for every block, in three different orders, with nothing
//! holding them together.

use crate::world::LevelId;
mod armor_stand;
mod barrel;
mod bed;
mod beehive;
mod bell;
mod brewing_stand;
mod cake;
mod campfire;
pub(in crate::world) use brewing_stand::BrewingStandHopperFace;
mod cauldron;
pub(in crate::world) use cauldron::{is_water_potion, water_potion_stack};
mod cave_vines;
mod chest;
mod chiseled_bookshelf;
mod composter;
mod copper_golem_statue;
mod crafter;
pub(in crate::world) mod crafting_table;
mod debug_stick;
mod decorated_pot;
mod dispenser;
mod dragon_egg;
mod ender_chest;
mod fence;
mod flower_pot;
mod furnace;
mod game_master;
mod hopper;
pub(in crate::world) use furnace::{
    input_cooking_time as furnace_input_cooking_time_for_items, FurnaceHopperFace,
};
pub(in crate::world) mod item_components;
mod jukebox;
mod lead;
mod lectern;
mod light;
mod lightable;
mod moving_piston;
mod note_block;
pub(in crate::world) mod placement;
mod pumpkin;
mod redstone_ore;
mod respawn_anchor;
mod shelf;
mod shulker_box;
mod sign;
pub(in crate::world) mod spawn_egg;
mod sweet_berry_bush;
mod tnt;
pub(in crate::world) mod utility_menu;
mod vault;

use std::borrow::Cow;

use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{
    BlockPos, BlockUpdate, InteractionHand, ItemStackSummary, PlayerRotation, SetCreativeModeSlot,
    SignUpdate, SoundEvent, SoundEventHolder, SoundSource, UseItem, UseItemOn, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;
use uuid::Uuid;

use crate::block_behavior::barrel as barrel_block;
use crate::block_behavior::beehive as beehive_block;
use crate::block_behavior::bell as bell_block;
use crate::block_behavior::cake as cake_block;
use crate::block_behavior::campfire as campfire_block;
use crate::block_behavior::candle as candle_block;
use crate::block_behavior::chest as chest_block;
use crate::block_behavior::chiseled_bookshelf as chiseled_bookshelf_block;
use crate::block_behavior::composter as composter_block;
use crate::block_behavior::copper_golem_statue as copper_golem_statue_block;
use crate::block_behavior::decorated_pot as decorated_pot_block;
use crate::block_behavior::dispenser as dispenser_block;
use crate::block_behavior::footing as candle_cake_block;
use crate::block_behavior::hopper as hopper_block;
use crate::block_behavior::jukebox as jukebox_block;
use crate::block_behavior::lectern as lectern_block;
use crate::block_behavior::redstone_ore as redstone_ore_block;
use crate::block_behavior::respawn_anchor as respawn_anchor_block;
use crate::block_behavior::shelf as shelf_block;
use crate::block_behavior::sweet_berry_bush as sweet_berry_bush_block;
use crate::block_behavior::update_placement_context as block_update_placement_context;
use crate::block_behavior::use_item_on as block_use_item_on;
use crate::block_behavior::use_without_item as block_use_without_item;
use crate::block_behavior::{BlockInteractionPlayer, BlockPlacementContext, BlockUseResult};
use crate::block_entity::sign as sign_entity;
use crate::block_entity::BlockEntity;
#[cfg(test)]
use crate::chunk::ChunkStore;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use bbb_server_types::block_queries;
use bbb_server_types::directions::{horizontal_direction, relative};
use bbb_server_types::interaction::is_within_block_interaction_range;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::item_use::ItemUseContext;
use super::simulation_level::pending_spawns::PendingEntitySpawns;
#[cfg(test)]
use super::simulation_level::placement_raw_brightness;
use super::using_item::runtime_item_in_hand;
use super::{
    hit_location_is_finite, send, wrap_degrees, ServerWorld, WorldCloseReason, WorldEvent,
    BLOCK_INTERACTION_BUFFER, CREATIVE_SLOT_MAX, CREATIVE_SLOT_MIN,
};
use crate::connection::{ClientboundPlayPacket, ConnectionId};

/// One column of one [`block_use_dispatch!`] row: `none` when the block does
/// not own that leg at all, otherwise the name of the adapter function in the
/// row's own module.
macro_rules! block_use_handler {
    ($module:ident, none) => {
        None
    };
    ($module:ident, $handler:ident) => {
        Some($module::$handler)
    };
}

/// The one table that says which block answers which leg of
/// `ServerPlayerGameMode.useItemOn`, and the only place a block name appears
/// on this surface.
///
/// [`ServerWorld::use_clicked_block`] used to carry three hand-written
/// `if`-chains — one per leg — that each named a different subset of the same
/// blocks in a different order, so adding a block meant editing three lists
/// plus a 2000-line file of bodies, and the three could drift apart without
/// anything noticing. The table states the three answers per block instead,
/// one row, and expands to the row array and the three dispatchers the
/// skeleton calls.
///
/// A row is:
///
/// ```text
/// module: predicate {
///     owners: ["VanillaClassName", ...],
///     item_on: adapter | none,
///     without_item: adapter | none,
///     spectator_menu: adapter | none,
/// }
/// ```
///
/// * the row key is the module — `world/block_use/<module>.rs` — that holds
///   this block's bodies and the adapters the three leg columns name. The
///   `mod` line stays above rather than being expanded from here, because
///   rustfmt does not follow a `mod` that only exists after expansion and
///   would stop formatting the file it names.
/// * `predicate` is `fn(i32) -> bool` over the clicked block's state id.
///   Rows are tried in table order and the first match wins, so the
///   predicates have to stay mutually exclusive; every one of them below is
///   a distinct block family, derived from a generated class tag rather than
///   from a list written here.
/// * `owners:` names every vanilla class whose `useItemOn` / `useWithoutItem`
///   / `getMenuProvider` this row answers for. It is what
///   `BLOCK_USE_OWNERS` flattens, and the guard test holds it against the
///   generated block catalog.
/// * the three leg columns are mandatory, which is what replaces the
///   exhaustiveness the deleted chains never had: a row that leaves one out
///   does not compile, so a new block cannot silently inherit an answer. Each
///   names an adapter in the row's module — `use_item_on`, `use_without_item`
///   and `open_menu` — or `none`, and `none` means the block falls through to
///   the generic path the skeleton ends in.
macro_rules! block_use_dispatch {
    ($(
        $module:ident: $predicate:path {
            owners: [$($owner:literal),+ $(,)?],
            item_on: $item_on:ident,
            without_item: $without_item:ident,
            spectator_menu: $spectator_menu:ident $(,)?
        }
    ),* $(,)?) => {
        /// The table, in the order the dispatchers walk it.
        const BLOCK_USE_ROWS: &[BlockUseRow] = &[
            $(
                BlockUseRow {
                    #[cfg(test)]
                    owners: &[$($owner),+],
                    matches: $predicate,
                    item_on: block_use_handler!($module, $item_on),
                    without_item: block_use_handler!($module, $without_item),
                    spectator_menu: block_use_handler!($module, $spectator_menu),
                },
            )*
        ];

        /// Every vanilla class the table's `owners:` column declares, flattened
        /// in table order. The guard test holds this against the generated
        /// block catalog's interaction columns, which is what keeps the table
        /// pointed at [`bbb_protocol::block_types`] rather than at a second
        /// hand-written list.
        #[cfg(test)]
        const BLOCK_USE_OWNERS: &[&str] = &[$($($owner),+),*];

        /// The first row whose predicate claims this block state, in table
        /// order.
        fn block_use_row(block_state_id: i32) -> Option<&'static BlockUseRow> {
            BLOCK_USE_ROWS
                .iter()
                .find(|row| (row.matches)(block_state_id))
        }

        /// `BlockBehaviour.getMenuProvider` opened on the clicking spectator,
        /// on the same terms; `None` is vanilla's null provider.
        fn dispatch_spectator_menu(
            world: &mut ServerWorld,
            context: &BlockUseContext<'_>,
            events: &mut Vec<WorldEvent>,
        ) -> Option<bool> {
            let handler = block_use_row(context.block_state_id)?.spectator_menu?;
            Some(handler(world, context, events))
        }
    };
}

/// Everything `ServerPlayerGameMode.useItemOn` has in hand when it reaches a
/// block, which is what every [`block_use_dispatch!`] adapter is handed: the
/// clicking player's connection and level, the packet the click arrived as —
/// position, face, cursor and hand — the state that was read at the position
/// once, and the stack in the hand named by the packet.
pub(super) struct BlockUseContext<'a> {
    pub(super) connection_id: ConnectionId,
    pub(super) dimension: LevelId,
    pub(super) update: UseItemOn,
    pub(super) block_state_id: i32,
    pub(super) selected_item: Cow<'a, RuntimeItemStack>,
}

impl BlockUseContext<'_> {
    fn projected_item(&self) -> Option<&ItemStackSummary> {
        Some(self.selected_item.summary())
    }
}

type BlockUseItemOnHandler =
    fn(&mut ServerWorld, &BlockUseContext<'_>, &mut Vec<WorldEvent>) -> BlockUseResult;

type BlockUseWithoutItemHandler =
    fn(&mut ServerWorld, &BlockUseContext<'_>, &mut Vec<WorldEvent>) -> bool;

type BlockUseMenuHandler = fn(&mut ServerWorld, &BlockUseContext<'_>, &mut Vec<WorldEvent>) -> bool;

/// One row of the [`block_use_dispatch!`] table.
struct BlockUseRow {
    /// The vanilla classes this row answers for; see `BLOCK_USE_OWNERS`.
    #[cfg(test)]
    owners: &'static [&'static str],
    matches: fn(i32) -> bool,
    item_on: Option<BlockUseItemOnHandler>,
    without_item: Option<BlockUseWithoutItemHandler>,
    spectator_menu: Option<BlockUseMenuHandler>,
}

block_use_dispatch! {
    vault: vault::is_vault {
        owners: ["VaultBlock"],
        item_on: use_item_on,
        without_item: none,
        spectator_menu: none,
    },
    game_master: game_master::is_game_master_block {
        owners: ["CommandBlock", "StructureBlock", "JigsawBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: none,
    },
    // The four sign blocks: both legs read the `SignBlockEntity` at the
    // position, and `SignBlock` overrides no `MenuProvider`.
    sign: sign::is_sign {
        owners: ["SignBlock", "CeilingHangingSignBlock", "WallHangingSignBlock"],
        item_on: use_item_on,
        without_item: use_without_item,
        spectator_menu: none,
    },
    // `ChestBlock`, which the generated `menu_provider_override` tag spreads
    // over the trapped and copper chests that inherit its legs.
    chest: chest_block::is_chest_block {
        owners: ["ChestBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: open_menu,
    },
    ender_chest: crate::block_behavior::ender_chest::is_ender_chest {
        owners: ["EnderChestBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: none,
    },
    // `BarrelBlock`.
    barrel: barrel_block::is_barrel {
        owners: ["BarrelBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: open_menu,
    },
    // `DispenserBlock` and the `DropperBlock` that inherits its legs.
    dispenser: dispenser_block::is_dispenser_family {
        owners: ["DispenserBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: open_menu,
    },
    hopper: hopper_block::is_hopper {
        owners: ["HopperBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: open_menu,
    },
    crafter: crafter::is_crafter {
        owners: ["CrafterBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: open_menu,
    },
    crafting_table: crafting_table::is_crafting_table {
        owners: ["CraftingTableBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: open_menu,
    },
    // `ShulkerBoxBlock`, all seventeen colours.
    shulker_box: block_queries::is_shulker_box {
        owners: ["ShulkerBoxBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: open_menu,
    },
    // `LecternBlock`, the one block outside the containers that answers all
    // three legs.
    lectern: lectern_block::is_lectern {
        owners: ["LecternBlock"],
        item_on: use_item_on,
        without_item: use_without_item,
        spectator_menu: open_menu,
    },
    // `BrewingStandBlock`: the block entity's own menu and the spectator's
    // inherited `BaseEntityBlock.getMenuProvider`.
    brewing_stand: brewing_stand::is_brewing_stand {
        owners: ["BrewingStandBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: open_menu,
    },
    // The menus whose slots all belong to the menu.
    utility_menu: utility_menu::is_utility_menu_block {
        owners: [
            "AnvilBlock",
            "BeaconBlock",
            "CartographyTableBlock",
            "EnchantingTableBlock",
            "GrindstoneBlock",
            "LoomBlock",
            "SmithingTableBlock",
            "StonecutterBlock",
        ],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: open_menu,
    },
    furnace: furnace::is_furnace {
        owners: ["AbstractFurnaceBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: open_menu,
    },
    // `ChiseledBookShelfBlock`.
    chiseled_bookshelf: chiseled_bookshelf_block::is_chiseled_bookshelf {
        owners: ["ChiseledBookShelfBlock"],
        item_on: use_item_on,
        without_item: use_without_item,
        spectator_menu: none,
    },
    // `DecoratedPotBlock`.
    decorated_pot: decorated_pot_block::is_decorated_pot {
        owners: ["DecoratedPotBlock"],
        item_on: use_item_on,
        without_item: use_without_item,
        spectator_menu: none,
    },
    // `ShelfBlock`, whose whole interaction is the item leg.
    shelf: shelf_block::is_shelf {
        owners: ["ShelfBlock"],
        item_on: use_item_on,
        without_item: none,
        spectator_menu: none,
    },
    // `BellBlock`, whose whole interaction is the empty-hand leg.
    bell: bell_block::is_bell {
        owners: ["BellBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: none,
    },
    // `CopperGolemStatueBlock` and the weathering subclass that overrides its
    // item leg.
    copper_golem_statue: copper_golem_statue_block::is_copper_golem_statue {
        owners: ["CopperGolemStatueBlock", "WeatheringCopperGolemStatueBlock"],
        item_on: use_item_on,
        without_item: none,
        spectator_menu: none,
    },
    // `BeehiveBlock`, both the hive and the bee nest.
    beehive: beehive_block::is_beehive {
        owners: ["BeehiveBlock"],
        item_on: use_item_on,
        without_item: none,
        spectator_menu: none,
    },
    // `ComposterBlock`.
    composter: composter_block::is_composter {
        owners: ["ComposterBlock"],
        item_on: use_item_on,
        without_item: use_without_item,
        spectator_menu: none,
    },
    // `CakeBlock`, whose empty-hand leg is the bite.
    cake: cake_block::is_cake {
        owners: ["CakeBlock"],
        item_on: use_item_on,
        without_item: use_without_item,
        spectator_menu: none,
    },
    // `DragonEggBlock`, whose empty-hand leg teleports.
    dragon_egg: dragon_egg::is_dragon_egg {
        owners: ["DragonEggBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: none,
    },
    moving_piston: moving_piston::is_moving_piston {
        owners: ["MovingPistonBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: none,
    },
    // `NoteBlock`, whose empty-hand leg tunes and counts `tune_noteblock`.
    note_block: note_block::is_note_block {
        owners: ["NoteBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: none,
    },
    light: light::is_light {
        owners: ["LightBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: none,
    },
    // `FenceBlock`, whose empty-hand leg binds held leashables to a knot.
    fence: fence::is_fence {
        owners: ["FenceBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: none,
    },
    // `TntBlock`, whose item leg lights the block into a primed TNT entity.
    tnt: tnt::is_tnt {
        owners: ["TntBlock"],
        item_on: use_item_on,
        without_item: none,
        spectator_menu: none,
    },
    lightable: candle_block::is_candle {
        owners: ["CandleBlock"],
        item_on: candle_use_item_on,
        without_item: none,
        spectator_menu: none,
    },
    lightable: candle_cake_block::is_candle_cake {
        owners: ["CandleCakeBlock"],
        item_on: candle_cake_use_item_on,
        without_item: use_candle_cake_without_item,
        spectator_menu: none,
    },
    lightable: campfire_block::is_campfire {
        owners: ["CampfireBlock"],
        item_on: campfire_use_item_on,
        without_item: none,
        spectator_menu: none,
    },
    pumpkin: pumpkin::is_pumpkin {
        owners: ["PumpkinBlock"],
        item_on: use_item_on,
        without_item: none,
        spectator_menu: none,
    },
    flower_pot: flower_pot::is_flower_pot {
        owners: ["FlowerPotBlock"],
        item_on: use_item_on,
        without_item: use_without_item,
        spectator_menu: none,
    },
    redstone_ore: redstone_ore_block::is_redstone_ore {
        owners: ["RedStoneOreBlock"],
        item_on: use_item_on,
        without_item: none,
        spectator_menu: none,
    },
    bed: crate::block_behavior::bed::is_bed {
        owners: ["BedBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: none,
    },
    respawn_anchor: respawn_anchor_block::is_respawn_anchor {
        owners: ["RespawnAnchorBlock"],
        item_on: use_item_on,
        without_item: use_without_item,
        spectator_menu: none,
    },
    jukebox: jukebox_block::is_jukebox {
        owners: ["JukeboxBlock"],
        item_on: use_item_on,
        without_item: use_without_item,
        spectator_menu: none,
    },
    sweet_berry_bush: sweet_berry_bush_block::is_sweet_berry_bush {
        owners: ["SweetBerryBushBlock"],
        item_on: use_item_on,
        without_item: use_without_item,
        spectator_menu: none,
    },
    cave_vines: cave_vines::is_cave_vines {
        owners: ["CaveVinesBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: none,
    },
    cave_vines: cave_vines::is_cave_vines_plant {
        owners: ["CaveVinesPlantBlock"],
        item_on: none,
        without_item: use_without_item,
        spectator_menu: none,
    },
    cauldron: cauldron::is_cauldron {
        owners: ["AbstractCauldronBlock"],
        item_on: use_item_on,
        without_item: none,
        spectator_menu: none,
    },
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod use_sound_tests;

impl ServerWorld {
    #[cfg(test)]
    pub(crate) fn placement_raw_brightness_for_test(
        chunks: &ChunkStore,
        pos: BlockPos,
        sky_darkening: i32,
    ) -> i32 {
        placement_raw_brightness(chunks, pos, sky_darkening)
    }

    /// `ServerGamePacketListenerImpl.handleUseItem` (:1356-1377): the
    /// right-click with nothing in front of the player.
    ///
    pub(super) fn apply_use_item(
        &mut self,
        connection_id: ConnectionId,
        update: UseItem,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        if !player.client_loaded() {
            return None;
        }
        // `ackBlockChangesUpTo` (:2270-2276) raises on a negative sequence,
        // which is the same refusal `handleUseItemOn` makes of one.
        if update.sequence < 0 || !update.y_rot.is_finite() || !update.x_rot.is_finite() {
            return Some(WorldCloseReason::InvalidBlockInteraction);
        }
        self.acknowledge_block_change(connection_id, update.sequence);
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let held = runtime_item_in_hand(player, update.hand).clone();
        if held.is_empty() {
            return None;
        }
        // `Mth.wrapDegrees` and `absSnapRotationTo` (:1366-1370), which run
        // before the item does and whatever the item then answers.
        player.set_rotation(PlayerRotation {
            y_rot: wrap_degrees(update.y_rot),
            x_rot: wrap_degrees(update.x_rot),
        });
        let dimension = player.dimension;
        let item = held.summary().clone();
        let result = self.use_item_in_hand(
            ItemUseContext {
                connection_id,
                dimension,
                hand: update.hand,
                item,
            },
            events,
        );
        if let Err(error) = result {
            tracing::warn!("use item: {error}");
        }
        None
    }

    /// `ServerGamePacketListenerImpl.handleSignUpdate` (:2028-2031) and the
    /// `updateSignText` (:2033-2044) it hands the filtered lines to.
    ///
    pub(super) fn apply_sign_update(
        &mut self,
        connection_id: ConnectionId,
        update: SignUpdate,
        events: &mut Vec<WorldEvent>,
    ) {
        let lines: [String; sign_entity::SIGN_TEXT_LINES] =
            std::array::from_fn(|index| sign_entity::strip_formatting(&update.lines[index]));
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let uuid = player.profile.uuid;
        let text_filtering = player.cold.client_information.text_filtering_enabled;
        let dimension = player.dimension;
        // `:2037`: a position in a column this level does not hold is dropped
        // without an answer.
        if !self
            .level_by_id(&dimension)
            .chunks
            .is_loaded(update.pos.x >> 4, update.pos.z >> 4)
        {
            return;
        }
        let Some(sign) = self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(update.pos)
            .and_then(BlockEntity::sign_mut)
        else {
            // `:2038-2040`: not a sign, nothing happens.
            return;
        };
        if sign.update_sign_text(uuid, text_filtering, update.is_front_text, &lines)
            != sign_entity::SignTextUpdate::Updated
        {
            return;
        }
        self.level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .set_changed(update.pos);
        self.level_mut_by_id(&dimension)
            .changed_block_broadcasts
            .insert((update.pos.x, update.pos.y, update.pos.z));
        let _ = events;
    }

    ///
    /// The two block updates at the end are unconditional in vanilla too
    /// (`:1384-1385`): they are sent whatever the interaction answered, which is
    /// how a client that mispredicted a refused placement is put straight.
    /// [`ServerWorld::player_may_place_block`] owns the game-mode half of that
    /// refusal.
    pub(super) fn apply_use_item_on(
        &mut self,
        connection_id: ConnectionId,
        update: UseItemOn,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        if update.sequence < 0 || !hit_location_is_finite(update) {
            return Some(WorldCloseReason::InvalidBlockInteraction);
        }
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        // `handleUseItemOn` (:1330): nothing before the client has loaded.
        if !player.client_loaded() {
            return None;
        }
        let awaiting_teleport = player.awaiting_teleport.is_some();
        self.acknowledge_block_change(connection_id, update.sequence);
        // :1339-1342 and :1386-1390 drop an out-of-reach hit or a cursor
        // outside the clicked block without a reply.
        if !self.player_reaches_block(connection_id, update.hit.pos)
            || !hit_location_near_block(update)
        {
            return None;
        }
        let (min_y, max_y) = self.player_build_height(connection_id);
        // :1346-1349.
        if update.hit.pos.y > max_y {
            self.send_build_limit_message(connection_id, true, max_y, events);
            return None;
        }
        if update.hit.pos.y < min_y {
            self.send_build_limit_message(connection_id, false, min_y, events);
            return None;
        }
        let dimension = self.player_level_id(connection_id);
        let target = relative(update.hit.pos, update.hit.direction);
        if self.refuse_block_interaction(connection_id, update.hit.pos, events) || awaiting_teleport
        {
            // :1373-1375: a refusal other than spawn protection, including an
            // outstanding teleport, answers with the build limit.
            if !self.player_is_under_spawn_protection(connection_id, update.hit.pos) {
                self.send_build_limit_message(connection_id, true, max_y, events);
            }
            self.send_use_item_on_updates(connection_id, dimension, update.hit.pos, target, events);
            return None;
        }

        let held = runtime_item_in_hand(
            self.levels
                .player(connection_id)
                .expect("connection presence was checked"),
            update.hand,
        )
        .clone();
        self.use_item_on_server_swing = false;
        let used_block = self.use_clicked_block(connection_id, update, &held, events);
        if used_block || self.levels[connection_id].game_type == GameType::Spectator {
            if used_block {
                self.trigger_any_block_use(connection_id, update.hit.pos, update.hand, events);
            }
            self.use_item_on_server_swings(connection_id, update.hand, events);
            self.send_use_item_on_updates(connection_id, dimension, update.hit.pos, target, events);
            return None;
        }
        // ServerPlayerGameMode.java:386-387.
        if self.levels[connection_id]
            .cold
            .cooldowns
            .is_on_cooldown(held.summary())
            .unwrap_or(false)
        {
            self.use_item_on_server_swings(connection_id, update.hand, events);
            self.send_use_item_on_updates(connection_id, dimension, update.hit.pos, target, events);
            return None;
        }
        let selected_item = held.summary().clone();

        let armor_stand_target =
            if held.item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_ARMOR_STAND_ID) {
                armor_stand::clicked_pos(self, dimension, update)
            } else {
                target
            };
        let armor_stand_use = if !used_block
            && self.is_in_world_bounds_by_key(&dimension, armor_stand_target)
            && !update.hit.world_border_hit
            && !held.is_empty()
            && self.player_may_place_block(connection_id, &selected_item, update.hit.pos)
        {
            armor_stand::use_item_on(self, connection_id, update, &selected_item)
        } else {
            None
        };
        let armor_stand_item = armor_stand_use.is_some();
        let lead_item = !used_block
            && !armor_stand_item
            && !update.hit.world_border_hit
            && !held.is_empty()
            && self.player_may_place_block(connection_id, &selected_item, update.hit.pos)
            && lead::use_item_on(self, connection_id, update, &selected_item, events);
        let spawn_egg_item = !used_block
            && !armor_stand_item
            && !lead_item
            && !update.hit.world_border_hit
            && !held.is_empty()
            && self.player_may_place_block(connection_id, &selected_item, update.hit.pos)
            && spawn_egg::use_item_on(self, connection_id, update, &selected_item, events);
        let debug_stick_use = if !used_block
            && !armor_stand_item
            && !lead_item
            && !spawn_egg_item
            && !update.hit.world_border_hit
            && !held.is_empty()
            && self.player_may_place_block(connection_id, &selected_item, update.hit.pos)
        {
            debug_stick::use_item_on(self, connection_id, update, &selected_item, events)
        } else {
            None
        };
        let debug_stick_item = debug_stick_use.is_some();
        let brush_item = !used_block
            && !armor_stand_item
            && !lead_item
            && !spawn_egg_item
            && !debug_stick_item
            && !update.hit.world_border_hit
            && !held.is_empty()
            && self.player_may_place_block(connection_id, &selected_item, update.hit.pos)
            && self.use_brush_on(connection_id, update.hand, &selected_item);
        let item_on_block = !used_block
            && !armor_stand_item
            && !lead_item
            && !spawn_egg_item
            && !debug_stick_item
            && !brush_item
            && !update.hit.world_border_hit
            && !held.is_empty()
            && self.player_may_place_block(connection_id, &selected_item, update.hit.pos)
            && super::item_use_on::use_item_on(self, connection_id, update, &selected_item, events);
        let mut placed_block = false;
        if !used_block
            && !armor_stand_item
            && !lead_item
            && !spawn_egg_item
            && !debug_stick_item
            && !brush_item
            && !item_on_block
            && !update.hit.world_border_hit
            && !held.is_empty()
            && self.player_may_place_block(connection_id, &selected_item, update.hit.pos)
        {
            let context = match held.item_id().map(|item_type_id| {
                self.block_placement_context(connection_id, dimension, update, item_type_id)
            }) {
                Some(Ok(context)) => context,
                Some(Err(reason)) => return Some(reason),
                None => None,
            };
            if let Some(context) = context {
                if let Some(block_state_id) = self.placed_block_state_with_events(
                    connection_id,
                    dimension,
                    context,
                    held.item_id(),
                    Some(&selected_item),
                    events,
                ) {
                    placed_block = self.place_block_from_item(
                        connection_id,
                        dimension,
                        context.pos,
                        block_state_id,
                        update.hand,
                        &held,
                        None,
                        events,
                    );
                }
            }
            if !placed_block && was_block_placement_attempt(&held) {
                placed_block = self.consume_unplaced_block_item(
                    &super::item_use::ItemUseContext {
                        connection_id,
                        dimension,
                        hand: update.hand,
                        item: selected_item.clone(),
                    },
                    events,
                );
            }
            // `ItemStack.useOn` (`ItemStack.java:367-369`) for `BlockItem.place`'s `SUCCESS`.
            if let (true, Some(item_id)) = (placed_block, selected_item.item_id) {
                self.levels[connection_id].award_stat_once(
                    &mut self.scoreboard,
                    StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
                );
            }
        }

        // `:397-399`, which the item leg reaches whichever item answered it,
        // at the clicked position rather than the one a placement wrote.
        if armor_stand_use == Some(true) || lead_item || brush_item || item_on_block || placed_block
        {
            self.trigger_item_used_on_location(
                connection_id,
                "minecraft:item_used_on_block",
                update.hit.pos,
                &selected_item,
                events,
            );
        }

        // ServerGamePacketListenerImpl.java:1356-1358.
        if armor_stand_use == Some(true)
            || lead_item
            || spawn_egg_item
            || debug_stick_use == Some(true)
            || brush_item
            || item_on_block
            || placed_block
        {
            self.trigger_any_block_use(connection_id, update.hit.pos, update.hand, events);
        } else if was_block_placement_attempt(&held) {
            // :1361-1368: a failed placement against the build height.
            match update.hit.direction {
                bbb_protocol::packets::Direction::Up if update.hit.pos.y >= max_y => {
                    self.send_build_limit_message(connection_id, true, max_y, events);
                }
                bbb_protocol::packets::Direction::Down if update.hit.pos.y <= min_y => {
                    self.send_build_limit_message(connection_id, false, min_y, events);
                }
                _ => {}
            }
        }
        self.use_item_on_server_swings(connection_id, update.hand, events);
        self.send_use_item_on_updates(connection_id, dimension, update.hit.pos, target, events);
        None
    }

    /// `BlockItem.place` (`BlockItem.java:67-90`) once the placement state is
    /// chosen: the write, the placed block's item data, the trigger, sound and
    /// game event, and the consume. `place_sound` is a `getPlaceSound`
    /// override (`SolidBucketItem.java:37-39`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn place_block_from_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        hand: InteractionHand,
        held: &RuntimeItemStack,
        place_sound: Option<&str>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        if !self.set_block(dimension, pos, block_state_id, SET_BLOCK_UPDATE_ALL) {
            return false;
        }
        item_components::apply_block_state(self, dimension, pos, held);
        let custom_data_loaded =
            item_components::apply_custom_data(self, dimension, pos, Some(connection_id), held);
        if !custom_data_loaded && self.block_state(dimension, pos).is_some_and(sign::is_sign) {
            self.open_sign_text_edit(connection_id, dimension, pos, true, events);
        }
        item_components::apply_components(self, dimension, pos, held);
        item_components::set_placed_by(self, dimension, pos, connection_id, held, held.summary());
        self.trigger_item_used_on_location(
            connection_id,
            "minecraft:placed_block",
            pos,
            held.summary(),
            events,
        );
        if let Some(placed) = self.block_state(dimension, pos) {
            let sound =
                bbb_protocol::block_sound_facts_26_1::vanilla_block_place_sound_26_1(placed);
            self.play_block_sound_as(
                dimension,
                pos,
                place_sound.unwrap_or(sound.sound),
                (sound.volume, sound.pitch),
                Some(connection_id),
                events,
            );
        }
        let placed = self.block_state_or_air(dimension, pos);
        let player_entity_id = self.levels[connection_id].entity_id();
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::BLOCK_PLACE,
                pos,
                Some(player_entity_id),
                Some(placed),
            );
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let has_infinite_materials = player.abilities.instabuild;
        player
            .inventory
            .consume_in_hand(hand, 1, has_infinite_materials);
        true
    }

    /// Marks the `SUCCESS_SERVER` the current `useItemOn` answers.
    pub(super) fn mark_use_item_on_server_swing(&mut self) {
        self.use_item_on_server_swing = true;
    }

    fn use_item_on_server_swings(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        events: &mut Vec<WorldEvent>,
    ) {
        if std::mem::take(&mut self.use_item_on_server_swing) {
            self.player_swing(connection_id, hand, true, events);
            self.player_swing(connection_id, hand, true, events);
        }
    }

    fn send_use_item_on_updates(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        clicked: BlockPos,
        target: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        for pos in [clicked, target] {
            if let Some(block_state_id) = self.block_state(dimension, pos) {
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::BlockUpdate(BlockUpdate {
                        pos,
                        block_state_id,
                    }),
                ));
            }
        }
    }

    ///
    ///
    /// The spectator arm (`:358-365`) is the block's `MenuProvider` and
    /// nothing else: a spectator opens a container it clicks and answers
    /// `CONSUME`, and answers `PASS` for a block that has none.
    /// [`ServerWorld::player_may_place_block`] refuses the item leg either
    /// way, so `PASS` and `false` are the same answer here.
    ///
    /// Which block answers which of the three legs is not written here: this
    /// is the vanilla skeleton, and every branch of it that names a block is
    /// one lookup into the [`block_use_dispatch!`] table.
    pub(super) fn use_clicked_block(
        &mut self,
        connection_id: ConnectionId,
        update: UseItemOn,
        selected_item: &RuntimeItemStack,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        if player.game_type == GameType::Spectator {
            let dimension = self.player_level_id(connection_id);
            let Some(block_state_id) = self.block_state(dimension, update.hit.pos) else {
                return false;
            };
            let context = BlockUseContext {
                connection_id,
                dimension,
                update,
                block_state_id,
                selected_item: Cow::Borrowed(selected_item),
            };
            // A block with no `spectator_menu:` owner has no `MenuProvider`,
            // which is vanilla's null and this method's `false`.
            return dispatch_spectator_menu(self, &context, events).unwrap_or(false);
        }
        let have_something_in_our_hands = !player.inventory.selected_stack().is_empty()
            || !player.inventory.off_hand_stack().is_empty();
        if player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN)
            && have_something_in_our_hands
        {
            return false;
        }
        let dimension = self.player_level_id(connection_id);
        let Some(block_state_id) = self.block_state(dimension, update.hit.pos) else {
            return false;
        };
        let context = BlockUseContext {
            connection_id,
            dimension,
            update,
            block_state_id,
            selected_item: Cow::Borrowed(selected_item),
        };
        let row = block_use_row(block_state_id);
        let use_item_on = row
            .and_then(|row| row.item_on)
            .map(|handler| handler(self, &context, events))
            .unwrap_or_else(|| {
                block_use_item_on(
                    block_state_id,
                    selected_item.item_id(),
                    update.hit.direction,
                )
            });
        match use_item_on {
            BlockUseResult::Consumed => {
                // `:372-374`, over the copy the caller took before this leg.
                let listening = self.levels[connection_id]
                    .cold
                    .advancements
                    .trigger_listeners("minecraft:item_used_on_block")
                    .next()
                    .is_some();
                if listening {
                    self.trigger_item_used_on_location(
                        connection_id,
                        "minecraft:item_used_on_block",
                        update.hit.pos,
                        selected_item.summary(),
                        events,
                    );
                }
                return true;
            }
            BlockUseResult::Pass => return false,
            BlockUseResult::TryWithEmptyHand => {}
        }
        // `:377-383`: the empty-hand leg is main-hand only, whatever the
        // off hand is holding.
        if update.hand != InteractionHand::MainHand {
            return false;
        }
        // The same fall-through over the `without_item:` column.
        let consumed = row
            .and_then(|row| row.without_item)
            .map(|handler| handler(self, &context, events))
            .unwrap_or_else(|| {
                self.block_interaction_player(connection_id)
                    .is_some_and(|player| {
                        self.use_block_without_item(
                            dimension,
                            update.hit.pos,
                            block_state_id,
                            player,
                        )
                    })
            });
        // ServerPlayerGameMode.java:377-383.
        if consumed {
            self.trigger_default_block_use(connection_id, update.hit.pos, events);
        }
        consumed
    }

    /// The `Player` `ServerPlayerGameMode.useItemOn` hands the block (`:378`).
    pub(super) fn block_interaction_player(
        &self,
        connection_id: ConnectionId,
    ) -> Option<BlockInteractionPlayer> {
        let player = self.levels.player(connection_id)?;
        Some(BlockInteractionPlayer {
            entity_id: player.entity_id(),
            may_build: player.may_build(),
            horizontal_direction: horizontal_direction(player.rotation().y_rot)?,
        })
    }

    /// The `SignBlockEntity` at a position, when one is there.
    pub(super) fn sign_at(
        &self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
    ) -> Option<&sign_entity::SignBlockEntity> {
        let dimension = dimension.into();
        self.level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::sign)
    }

    /// Whether the block entity at a position is a `HangingSignBlockEntity`
    /// rather than a plain `SignBlockEntity`, which is the only thing the two
    /// registry types are told apart by here.
    pub(super) fn block_entity_type_is_hanging_sign(
        &self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
    ) -> bool {
        let dimension = dimension.into();
        self.level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .is_some_and(|block_entity| {
                block_entity.type_id() == VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID
            })
    }

    pub(super) fn play_block_sound(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        sound_resource_id: &str,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        self.play_block_sound_as(dimension, pos, sound_resource_id, (1.0, 1.0), None, events);
    }

    ///
    /// A resource id the sound catalog does not hold is played as nothing
    /// rather than as a different sound.
    pub(super) fn play_block_sound_as(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        sound_resource_id: &str,
        (volume, pitch): (f32, f32),
        except: Option<ConnectionId>,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let Some(registry_id) = vanilla_sound_event_registry_id(sound_resource_id) else {
            return;
        };
        let sound = SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            source: SoundSource::Blocks,
            position: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            volume,
            pitch,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(&dimension, &sound, except, events);
    }

    pub(super) fn use_block_without_item(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        player: BlockInteractionPlayer,
    ) -> bool {
        let dimension = dimension.into();
        let players = self.nearby_players_by_key(&dimension);
        let mut leftover = PendingEntitySpawns::default();
        let consumed = self.with_level_ctx(&dimension, &players, &mut leftover, |level, rest| {
            let mut level = level.with_structure_entities(rest.entities, None);
            block_use_without_item(&mut level, pos, block_state_id, player)
        });
        // The frame the block use ran on is over.
        self.post_pending_game_events(&dimension);
        self.apply_pending_spawns(&dimension, leftover);
        consumed
    }

    pub(crate) fn block_placement_context(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        update: UseItemOn,
        item_type_id: i32,
    ) -> Result<Option<BlockPlacementContext>, WorldCloseReason> {
        let dimension = dimension.into();
        let (horizontal_direction, secondary_use_active) = {
            let player = self
                .levels
                .player(connection_id)
                .expect("connection presence was checked");
            let horizontal_direction = horizontal_direction(player.rotation().y_rot)
                .ok_or(WorldCloseReason::InvalidBlockInteraction)?;
            (
                horizontal_direction,
                player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
            )
        };
        let absolute_hit_x = f64::from(update.hit.pos.x) + f64::from(update.hit.cursor_x);
        let absolute_hit_y = f64::from(update.hit.pos.y) + f64::from(update.hit.cursor_y);
        let absolute_hit_z = f64::from(update.hit.pos.z) + f64::from(update.hit.cursor_z);
        let clicked_context = BlockPlacementContext {
            pos: update.hit.pos,
            clicked_face: update.hit.direction,
            click_x: absolute_hit_x - f64::from(update.hit.pos.x),
            click_y: absolute_hit_y - f64::from(update.hit.pos.y),
            click_z: absolute_hit_z - f64::from(update.hit.pos.z),
            replacing_clicked: true,
            horizontal_direction,
            secondary_use_active,
            inside: update.hit.inside,
        };
        let clicked_state = self.block_state_or_air(dimension, update.hit.pos);
        let content_registry = std::sync::Arc::clone(self.registries.content_registry_arc());
        let Some(replacing_clicked) =
            crate::block_behavior::can_replace_for_placement_with_registry(
                item_type_id,
                clicked_state,
                clicked_context,
                &content_registry,
            )
        else {
            return Ok(None);
        };
        let pos = if replacing_clicked {
            update.hit.pos
        } else {
            relative(update.hit.pos, update.hit.direction)
        };
        let context = BlockPlacementContext {
            pos,
            clicked_face: update.hit.direction,
            click_x: absolute_hit_x - f64::from(pos.x),
            click_y: absolute_hit_y - f64::from(pos.y),
            click_z: absolute_hit_z - f64::from(pos.z),
            replacing_clicked,
            horizontal_direction,
            secondary_use_active,
            inside: update.hit.inside,
        };
        // `Level.setBlock` refuses outside the world, so the placement fails.
        if !self.is_in_world_bounds_by_key(&dimension, pos) {
            return Ok(None);
        }
        if replacing_clicked
            || crate::block_behavior::can_replace_for_placement_with_registry(
                item_type_id,
                self.block_state_or_air(dimension, pos),
                context,
                &content_registry,
            ) == Some(true)
        {
            Ok(block_update_placement_context(
                item_type_id,
                context,
                &mut |at| {
                    (
                        self.is_in_world_bounds_by_key(&dimension, at),
                        self.block_state_or_air(dimension, at),
                    )
                },
            ))
        } else {
            Ok(None)
        }
    }

    #[cfg(test)]
    pub(crate) fn placed_block_state(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        context: BlockPlacementContext,
        item_type_id: Option<i32>,
    ) -> Option<i32> {
        let dimension = dimension.into();
        self.placed_block_state_with_events(
            connection_id,
            dimension,
            context,
            item_type_id,
            None,
            &mut Vec::new(),
        )
    }

    #[cfg(test)]
    pub(crate) fn placed_block_state_with_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        context: BlockPlacementContext,
        held_item: &ItemStackSummary,
    ) -> Option<i32> {
        let dimension = dimension.into();
        self.placed_block_state_with_events(
            connection_id,
            dimension,
            context,
            held_item.item_id,
            Some(held_item),
            &mut Vec::new(),
        )
    }

    /// `Level.getBlockState` with the filler every mechanism already sees.
    ///
    pub(super) fn block_state_or_air(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
    ) -> i32 {
        let dimension = dimension.into();
        self.block_state(dimension, pos)
            .unwrap_or_else(|| self.level_by_id(&dimension).chunks.air_state_id())
    }

    /// `ServerGamePacketListenerImpl.handleSetCreativeModeSlot` (:2001-2025).
    ///
    /// The write is answered with silence, and deliberately so: vanilla writes
    /// the slot *and* the menu's remote mirror of it in the same breath
    /// (:2013-2015), so the `broadcastChanges` that follows finds nothing to
    /// report. The client already knows what it asked for.
    fn is_item_enabled(&self, item: &RuntimeItemStack) -> bool {
        let Some(item_id) = item.item_id() else {
            return true;
        };
        let vanilla_enabled = self.cold.config.datapacks.as_ref().is_none_or(|startup| {
            startup
                .enabled_features
                .iter()
                .any(|feature| feature == "minecraft:vanilla")
        });
        if bbb_protocol::item_types::vanilla_item_type(item_id).is_some() {
            return vanilla_enabled;
        }
        self.cold
            .config
            .plugins
            .content_registry()
            .item_by_id(item_id)
            .is_some()
    }

    pub(super) fn apply_creative_slot(
        &mut self,
        connection_id: ConnectionId,
        update: SetCreativeModeSlot,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        if !player.abilities.instabuild {
            return;
        }
        let item = RuntimeItemStack::from_summary(update.item);
        // `ServerGamePacketListenerImpl.handleSetCreativeModeSlot` (`:2006`):
        // ignore items disabled under the world's enabled feature flags.
        if !self.is_item_enabled(&item) {
            return;
        }
        let slot = update.slot_num;
        let valid_slot = (CREATIVE_SLOT_MIN..=CREATIVE_SLOT_MAX).contains(&slot);
        let valid_data = if item.is_empty() {
            true
        } else {
            let max_ok = match item.max_stack_size() {
                Ok(maximum) => item.count() <= maximum,
                Err(error) => {
                    tracing::warn!("creative incoming item maximum: {error}");
                    return;
                }
            };
            if !max_ok {
                false
            } else {
                // `ItemStack.validatedStreamCodec` (`ItemStack.java:197-213`): reject any creative
                // stack whose components or registry holders fail persistence or outbound wire encoding.
                let resolver = self.item_stack_resolver();
                let persistence_ok =
                    bbb_registry_codec::item_stack_summary_to_persistent_nbt_with_resolver(
                        item.summary(),
                        &resolver,
                    )
                    .is_ok();
                let wire_ok = persistence_ok
                    && bbb_protocol::packets::encode_play_clientbound_container_set_slot_with_content_registry(
                        &bbb_protocol::packets::ContainerSetSlot {
                            container_id: 0,
                            state_id: 0,
                            slot: 0,
                            item: item.summary().clone(),
                        },
                        self.cold.config.plugins.content_registry(),
                    )
                    .is_ok();
                if !wire_ok {
                    tracing::warn!("creative incoming item failed persistence/wire validation");
                }
                wire_ok
            }
        };
        if valid_slot && valid_data {
            self.write_inventory_menu_slot(connection_id, slot, item, events);
            return;
        }
        // `:2016-2022`: a negative slot drops the stack instead of writing it,
        // and only while the connection's own `TickThrottler(20, 1480)`
        // (`:244`) is under its threshold.
        if slot >= 0 || !valid_data {
            return;
        }
        let throttled = {
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("connection presence was checked");
            if player.cold.drop_spam_throttler.is_under_threshold() {
                player.cold.drop_spam_throttler.increment();
                false
            } else {
                true
            }
        };
        if throttled {
            return;
        }
        // `player.drop(itemStack, true)` (`:2019`).
        self.drop_player_stack(connection_id, &item, true);
    }

    fn write_inventory_menu_slot(
        &mut self,
        connection_id: ConnectionId,
        slot: i16,
        item: RuntimeItemStack,
        events: &mut Vec<WorldEvent>,
    ) {
        let resolver = self.item_stack_resolver();
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        let inventory = &mut player.inventory;
        let parked = inventory.begin_inventory_menu();
        let prepared = (|| {
            let grid_write = inventory.is_crafting_grid_slot(slot);
            if !inventory.set_by_player(&resolver, Some(&mut self.maps), slot, item.clone())? {
                return Ok(None);
            }
            let published = if grid_write {
                Some(inventory.publish_crafting_result()?)
            } else {
                None
            };
            inventory.set_remote_slot(slot, &item);
            Ok::<_, bbb_server_types::ServerWorldError>(Some(published))
        })();
        self.play_equip_changes(connection_id, events);
        match prepared {
            Ok(Some(published)) => {
                if let Some(published) = published {
                    Self::send_menu_sync(connection_id, [published], events);
                }
                self.broadcast_player_menu(
                    connection_id,
                    crate::inventory::MenuBroadcastMode::Changes,
                    events,
                );
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!("creative inventory slot: {error}");
            }
        }
        if let Some(player) = self.levels.player_mut(connection_id) {
            player.inventory.end_inventory_menu(parked);
        }
    }

    /// [`Self::write_inventory_menu_slot`] without the ability gate in front of
    /// it, for the fixtures that stage a stack on a session whose game mode
    /// refuses the creative write.
    #[cfg(test)]
    pub(crate) fn stock_menu_slot_for_test(
        &mut self,
        connection_id: ConnectionId,
        slot: i16,
        item: ItemStackSummary,
    ) {
        let item = RuntimeItemStack::from_summary(item);
        self.write_inventory_menu_slot(connection_id, slot, item, &mut Vec::new());
    }

    /// The two game-mode gates a block placement passes on its way through
    /// `ServerPlayerGameMode.useItemOn` (`:349-406`), for the one item this
    /// world can place.
    ///
    /// The first is the spectator arm (`:358-365`), which returns before the
    /// item is consulted at all: the clicked block's `MenuProvider` is the only
    /// thing a spectator reaches, and otherwise the answer is
    /// `InteractionResult.PASS`. [`ServerWorld::use_clicked_block`] runs that
    /// arm, so what is left here is the refusal of the item leg that follows
    /// it.
    ///
    ///
    /// Creative and survival both pass; what separates them is the consume at
    /// the call site, not this gate.
    pub(super) fn player_may_place_block(
        &self,
        connection_id: ConnectionId,
        stack: &ItemStackSummary,
        pos: BlockPos,
    ) -> bool {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        if player.game_type == GameType::Spectator {
            return false;
        }
        player.may_build() || self.adventure_mode_allows(&player.dimension, pos, stack, false)
    }

    /// `ServerPlayer.isWithinBlockInteractionRange(pos, 1.0)`, the reach gate of
    /// `handleUseItemOn` (:1339) and `handleBlockBreakAction` (:153).
    pub(super) fn player_reaches_block(&self, connection_id: ConnectionId, pos: BlockPos) -> bool {
        let Some(player) = self.levels.player(connection_id) else {
            return false;
        };
        let Ok(eye) = player.eye_position() else {
            return false;
        };
        is_within_block_interaction_range(
            eye,
            pos,
            player.block_interaction_range(),
            BLOCK_INTERACTION_BUFFER,
        )
    }

    /// `Level.getMinY()` and `getMaxY()` of the player's level.
    pub(super) fn player_build_height(&self, connection_id: ConnectionId) -> (i32, i32) {
        let chunks = &self
            .level_by_id(&self.player_level_id(connection_id))
            .chunks;
        let min_y = chunks.min_y();
        let height = i32::try_from(chunks.section_count() * 16).unwrap_or(i32::MAX);
        (min_y, min_y.saturating_add(height) - 1)
    }

    /// `ServerPlayer.sendBuildLimitMessage` (ServerPlayer.java:1791-1793).
    pub(super) fn send_build_limit_message(
        &self,
        connection_id: ConnectionId,
        too_high: bool,
        limit: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        self.send_system_message(
            connection_id,
            bbb_protocol::packets::SystemChat::from_component(
                crate::commands::build_limit_message(too_high, limit),
                true,
            ),
            events,
        );
    }

    pub(super) fn sign_editor_is_too_far_away_by_key(
        &self,
        dimension: &LevelId,
        editor: Uuid,
        pos: BlockPos,
    ) -> bool {
        let Some(player) = self
            .levels
            .players()
            .find(|player| player.profile.uuid == editor && &player.dimension == dimension)
        else {
            // `:262`: `getPlayerByUUID` answering null is "too far".
            return true;
        };
        let Ok(eye) = player.eye_position() else {
            return true;
        };
        !is_within_block_interaction_range(
            eye,
            pos,
            player.block_interaction_range(),
            sign_entity::SIGN_EDIT_RANGE_BUFFER,
        )
    }
}

/// The cursor check of `handleUseItemOn` (:1342-1344): the hit lies within
/// about a block of the clicked block's centre.
fn hit_location_near_block(update: UseItemOn) -> bool {
    const LIMIT: f32 = 1.000_000_1;
    [
        update.hit.cursor_x,
        update.hit.cursor_y,
        update.hit.cursor_z,
    ]
    .into_iter()
    .all(|offset| (offset - 0.5).abs() < LIMIT)
}

/// `wasBlockPlacementAttempt` (:1317-1325); the cooldown leg is answered
/// before the caller reaches it.
fn was_block_placement_attempt(held: &RuntimeItemStack) -> bool {
    if held.is_empty() {
        return false;
    }
    held.item_id()
        .and_then(bbb_protocol::item_types::vanilla_item_type)
        .is_some_and(|item| {
            item.block_item_behavior.is_some()
                || item.bucket_behavior.is_some_and(|bucket| {
                    bucket.fluid != bbb_protocol::item_types::VanillaBucketFluid::Empty
                })
        })
}
