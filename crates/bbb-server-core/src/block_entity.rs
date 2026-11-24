//! Block entities: the per-position objects a block state declares, their
//! type binding, and the create/remove seam a block write reaches. Storage and
//! the ticker list live in [`crate::chunk::block_entities`].
//!
//! * `BlockEntity` — the object itself, its position, its cached block state,
//!   its `DataComponentMap`, and the load/save pair.
//! * `BlockEntityType` — the registry and `isValid`, answered by the generated
//!   [`bbb_protocol::block_entity_types`] catalog and the `block_entity_type_id`
//!   column of [`bbb_protocol::block_types`].
//! * `EntityBlock` / `BaseEntityBlock` — `newBlockEntity` and `getTicker`.
//!
//! # What a runtime block entity is, and what it is not
//!
//! A [`BlockEntity`] exists here only for the types this crate implements —
//! today the rows of the `block_entity_payloads!` table below. Every other
//! `EntityBlock` still answers `hasBlockEntity()` true, still has its saved
//! record validated by `bbb_server_persistence`, and still gets its record
//! removed when its block is broken; what it does not get is a live object,
//! which is exactly the shape vanilla's `@Nullable BlockEntity
//! newBlockEntity(...)` already allows. A piston pair and a command block
//! each land by adding one `block_entity/<type>.rs` and one row of the
//! `block_entity_payloads!` table below, which is the only place a block
//! entity type is bound: one row carries that type's `newBlockEntity`,
//! `getTicker`, `saveAdditional` / `loadAdditional`, `getUpdateTag`,
//! `getUpdatePacket`, `preRemoveSideEffects` and `triggerEvent` answers at once,
//! so the rest of this module (ownership, ticking, saving, the update packet)
//! stays type-agnostic.
//!
//! The two seams the next slices need, named here so they are not re-invented:
//!
//! * containers hang off [`BlockEntity::pre_remove_side_effects`]
//!   (`BlockEntity.java:233-237`, `Containers.dropContents`) and off the menu
//!   provider `BaseEntityBlock.getMenuProvider` (`BaseEntityBlock.java:30-33`)
//!   reads out of the live object;
//! * `getRedstoneSignalFromBlockEntity` and the comparator hang off
//!   [`BlockEntitiesMut::set_changed`], which is where vanilla's
//!   `BlockEntity.setChanged` runs `updateNeighbourForOutputSignal`
//!   (`BlockEntity.java:198-203`);
//! * `triggerEvent` hangs off [`BlockEntity::trigger_event`]
//!   (`BlockEntity.java:239-241`), which `BaseEntityBlock.triggerEvent`
//!   (`BaseEntityBlock.java:23-27`) forwards every 26.1 override to.

use crate::block_getter::BlockTags;
use crate::registry_tags::RegistryTagSnapshot;
pub(crate) use bbb_protocol::nbt_summary::{codec_uuid, entry, uuid_int_array};

use bbb_protocol::block_entity_types::{
    vanilla_block_entity_resource_id_for_type_id, vanilla_block_entity_type_id_for_resource_id,
    VANILLA_BLOCK_ENTITY_TYPE_BANNER_ID, VANILLA_BLOCK_ENTITY_TYPE_BARREL_ID,
    VANILLA_BLOCK_ENTITY_TYPE_BEACON_ID, VANILLA_BLOCK_ENTITY_TYPE_BED_ID,
    VANILLA_BLOCK_ENTITY_TYPE_BEEHIVE_ID, VANILLA_BLOCK_ENTITY_TYPE_BELL_ID,
    VANILLA_BLOCK_ENTITY_TYPE_BLAST_FURNACE_ID, VANILLA_BLOCK_ENTITY_TYPE_BREWING_STAND_ID,
    VANILLA_BLOCK_ENTITY_TYPE_BRUSHABLE_BLOCK_ID,
    VANILLA_BLOCK_ENTITY_TYPE_CALIBRATED_SCULK_SENSOR_ID, VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID,
    VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID, VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID,
    VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID,
    VANILLA_BLOCK_ENTITY_TYPE_CONDUIT_ID, VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID,
    VANILLA_BLOCK_ENTITY_TYPE_CRAFTER_ID, VANILLA_BLOCK_ENTITY_TYPE_CREAKING_HEART_ID,
    VANILLA_BLOCK_ENTITY_TYPE_DAYLIGHT_DETECTOR_ID, VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
    VANILLA_BLOCK_ENTITY_TYPE_DISPENSER_ID, VANILLA_BLOCK_ENTITY_TYPE_DROPPER_ID,
    VANILLA_BLOCK_ENTITY_TYPE_ENCHANTING_TABLE_ID, VANILLA_BLOCK_ENTITY_TYPE_ENDER_CHEST_ID,
    VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID, VANILLA_BLOCK_ENTITY_TYPE_END_PORTAL_ID,
    VANILLA_BLOCK_ENTITY_TYPE_FURNACE_ID, VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID,
    VANILLA_BLOCK_ENTITY_TYPE_HOPPER_ID, VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
    VANILLA_BLOCK_ENTITY_TYPE_JUKEBOX_ID, VANILLA_BLOCK_ENTITY_TYPE_LECTERN_ID,
    VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID, VANILLA_BLOCK_ENTITY_TYPE_PISTON_ID,
    VANILLA_BLOCK_ENTITY_TYPE_SCULK_CATALYST_ID, VANILLA_BLOCK_ENTITY_TYPE_SCULK_SENSOR_ID,
    VANILLA_BLOCK_ENTITY_TYPE_SCULK_SHRIEKER_ID, VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID,
    VANILLA_BLOCK_ENTITY_TYPE_SHULKER_BOX_ID, VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
    VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID, VANILLA_BLOCK_ENTITY_TYPE_SMOKER_ID,
    VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
    VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_TRAPPED_CHEST_ID,
    VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID, VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::packets::{BlockEntityData, BlockPos, LevelChunkBlockEntity};
use bbb_protocol::{encode_nbt_summary_network_quota, NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::PersistentItemStackHolderResolver;

use crate::chunk::block_entities::{BlockEntities, BlockEntitiesMut};
use components::{ComponentMapBuilder, ComponentMapReader};
use game_event::{DeliveryMode, GameEventListenerEntry, GameEventListenerOwner};
use item_projection::BlockEntityItemSave;

pub mod banner;
pub mod barrel;
pub mod base_spawner;
pub mod beacon;
pub mod bed;
pub mod beehive;
pub mod bell;
pub mod brewing_stand;
pub mod brushable;
pub mod campfire;
pub mod chest;
pub mod chiseled_bookshelf;
pub mod command_block;
mod command_data;
pub mod comparator;
mod components;
pub mod conduit;
mod container_access;
mod container_fields;
pub mod copper_golem_statue;
pub mod crafter;
pub mod creaking_heart;
pub mod daylight_detector;
pub mod decorated_pot;
pub mod dispenser;
pub mod enchanting_table;
pub mod end_gateway;
pub mod end_portal;
pub mod ender_chest;
pub mod furnace;
pub mod game_event;
pub mod hopper;
mod item_projection;
pub mod jigsaw;
pub mod jukebox;
pub mod lectern;
mod loot_view;
mod mutation;
pub mod piston;
pub mod sculk_catalyst;
pub mod sculk_sensor;
pub mod sculk_shrieker;
pub mod shelf;
pub mod shulker_box;
pub mod sign;
pub mod skull;
pub mod spawner;
pub mod structure_block;
pub mod test_block;
pub mod trial_spawner;
mod value_input;
pub mod vault;
pub mod vibration;

pub use command_data::BlockEntityCommandDataError;
pub use components::CollectedComponents;
pub use container_fields::{ContainerLoot, ContainerLootTable};
pub use item_projection::{BlockEntityItemLoad, BlockEntityItemProjection};

/// The key every per-position map in this crate uses; [`BlockPos`] carries no
/// `Ord`, and the changed-block overlay already keys on the triple.
pub type BlockEntityKey = (i32, i32, i32);

/// A chunk column's coordinates.
type ChunkKey = (i32, i32);

/// `SectionPos.blockToSectionCoord`, an arithmetic shift rather than a
/// division, so a negative coordinate rounds towards the floor.
fn chunk_of(pos: BlockPos) -> ChunkKey {
    (pos.x >> 4, pos.z >> 4)
}

use bbb_server_types::block_queries::{block_entity_type_for_state, block_state_has_block_entity};

/// `BlockEntityType.isValid(state)` (`BlockEntityType.java:299-301`):
/// the type's `validBlocks` set holds the state's block.
#[must_use]
pub fn block_entity_type_is_valid(type_id: i32, block_state_id: i32) -> bool {
    block_entity_type_for_state(block_state_id) == Some(type_id)
}

/// `BlockState.shouldChangedStateKeepBlockEntity(oldState)`
/// (`BlockBehaviour.java:851-853`), which asks the *new* state's block.
///
/// `BlockBehaviour.shouldChangedStateKeepBlockEntity` is false
/// (`BlockBehaviour.java:258-260`) and exactly two 26.1 blocks override it:
/// `CopperChestBlock` answers `oldState.is(BlockTags.COPPER_CHESTS)`
/// (`CopperChestBlock.java:142-144`) and `CopperGolemStatueBlock` answers
/// `oldState.is(BlockTags.COPPER_GOLEM_STATUES)`
/// (`CopperGolemStatueBlock.java:136-138`) — the two families whose oxidation
/// and waxing steps replace the block while keeping the container behind it.
/// Both are answered off the generated 26.1 tag snapshot rather than off a
/// list written here.
#[must_use]
pub fn should_changed_state_keep_block_entity(
    tags: &dyn BlockTags,
    old_state: i32,
    new_state: i32,
) -> bool {
    // The new block's class is fixed by registration; the old state is
    // matched against the loaded tag.
    ["minecraft:copper_chests", "minecraft:copper_golem_statues"]
        .into_iter()
        .any(|tag| {
            RegistryTagSnapshot::builtin().block_state_in(tag, new_state)
                && tags.block_in_tag(old_state, tag)
        })
}

/// `EntityBlock.newBlockEntity(worldPosition, blockState)`
/// (`EntityBlock.java:14`) over the types this crate carries.
///
/// `None` covers both of vanilla's shapes at once: a block that is not an
/// `EntityBlock` at all, and an `EntityBlock` whose `newBlockEntity` answers
/// null. `LevelChunk.setBlockState` (`LevelChunk.java:338-342`) and
/// `LevelChunk.createBlockEntity` (`:358-361`) both branch on exactly that
/// nullability, so a type with no owner here is handled by the code vanilla
/// already has rather than by an extra gate.
#[must_use]
pub fn new_block_entity(pos: BlockPos, block_state_id: i32) -> Option<BlockEntity> {
    let type_id = block_entity_type_for_state(block_state_id)?;
    let payload = payload_for_block_entity_type(type_id)?;
    let mut entity = BlockEntity::new(type_id, pos, block_state_id, payload);
    if let Some(crafter) = entity.crafter_mut() {
        crafter.set_triggered(
            bbb_server_types::block_queries::bool_property(block_state_id, "triggered")
                .unwrap_or(false),
        );
    }
    if let Some(command) = entity.command_block_mut() {
        command.set_automatic(
            crate::block_behavior::command_block::mode(block_state_id)
                == bbb_protocol::packets::CommandBlockMode::Sequence,
        );
    }
    if let Some(structure) = entity.structure_block_mut() {
        let mode = match crate::block_behavior::string_property(block_state_id, "mode").as_deref() {
            Some("save") => bbb_protocol::packets::StructureBlockMode::Save,
            Some("corner") => bbb_protocol::packets::StructureBlockMode::Corner,
            Some("data") => bbb_protocol::packets::StructureBlockMode::Data,
            _ => bbb_protocol::packets::StructureBlockMode::Load,
        };
        structure.set_mode(mode);
    }
    Some(entity)
}

/// `yes` / `no` as the table below spells a per-type boolean.
macro_rules! block_entity_yes_no {
    (no) => {
        false
    };
    (yes) => {
        true
    };
}

/// The table's `listener:` column, i.e. the `EntityBlock.getListener`
/// (`EntityBlock.java:20-22`) answer of a type's block.
macro_rules! block_entity_listener {
    (sensor, $pos:expr, $state:expr) => {
        Some(GameEventListenerEntry {
            pos: $pos,
            radius: sculk_sensor::listener_radius($state),
            delivery_mode: DeliveryMode::Unspecified,
            owner: GameEventListenerOwner::BlockEntity,
        })
    };
    (no, $pos:expr, $state:expr) => {
        None
    };
    ({ radius: $radius:expr, delivery: by_distance }, $pos:expr, $state:expr) => {
        Some(GameEventListenerEntry {
            pos: $pos,
            radius: $radius,
            delivery_mode: DeliveryMode::ByDistance,
            owner: GameEventListenerOwner::BlockEntity,
        })
    };
    ({ radius: $radius:expr, delivery: unspecified }, $pos:expr, $state:expr) => {
        Some(GameEventListenerEntry {
            pos: $pos,
            radius: $radius,
            delivery_mode: DeliveryMode::Unspecified,
            owner: GameEventListenerOwner::BlockEntity,
        })
    };
}

/// The table's `new:` column, which defaults to the payload's `Default`.
macro_rules! block_entity_payload_new {
    ($module:ident, $object:ident) => {
        $module::$object::default()
    };
    ($module:ident, $object:ident, $new:expr) => {
        $new
    };
}

/// The table's `saves:` column on the save half: `no` is the empty base body.
/// The `items:` column adds the payload's own item stacks, which only the rows
/// that declare it encode through the caller's publication.
macro_rules! block_entity_save_additional {
    (no, $payload:ident, $save:ident) => {{
        let _ = ($payload, $save);
        Ok(Vec::new())
    }};
    (yes, $payload:ident, $save:ident) => {
        Ok($payload.save_additional())
    };
    (yes, $payload:ident, $save:ident, yes) => {
        $payload.save_additional($save)
    };
}

macro_rules! block_entity_load_additional {
    (jigsaw, yes, $payload:ident, $record:ident, $load:ident, $state:expr) => {{
        $payload.load_additional_for_state($record, $state);
        Ok(())
    }};
    ($module:ident, no, $payload:ident, $record:ident, $load:ident, $state:expr) => {{
        let _ = ($payload, $record, $load);
        Ok(())
    }};
    ($module:ident, yes, $payload:ident, $record:ident, $load:ident, $state:expr) => {{
        $payload.load_additional($record);
        Ok(())
    }};
    ($module:ident, yes, $payload:ident, $record:ident, $load:ident, $state:expr, yes) => {
        $payload.load_additional($record, $load)
    };
}

/// The table's `update_tag:` column: `saveCustomOnly` or the inherited empty
/// compound.
///
/// The wire carriers take the resolver-free projection on purpose: an update
/// tag is the byte for byte document this server already sent for a record, and
/// the publication a session opened against is not part of it. They also take
/// the level-free accumulator: the tag is the whole of what they answer.
macro_rules! block_entity_update_tag {
    (empty, $payload:ident, $state:expr, $save:expr) => {{
        let _ = ($payload, $state, $save);
        Ok(Vec::new())
    }};
    (empty, $payload:ident, $state:expr, $save:expr, yes) => {{
        let _ = ($payload, $state, $save);
        Ok(Vec::new())
    }};
    (additional, $payload:ident, $state:expr, $save:expr) => {{
        let _ = ($state, $save);
        Ok($payload.save_additional())
    }};
    (additional, $payload:ident, $state:expr, $save:expr, yes) => {{
        let _ = $state;
        $payload.save_additional($save)
    }};
    (items, $payload:ident, $state:expr, $save:expr) => {{
        let _ = ($state, $save);
        Ok($payload.save_items())
    }};
    (items, $payload:ident, $state:expr, $save:expr, yes) => {{
        let _ = $state;
        $payload.save_items($save)
    }};
    // The one update tag that reads the block state beside the payload
    // (`TrialSpawnerBlockEntity.getUpdateTag`, `:51-53`).
    (items_with_state, $payload:ident, $state:expr, $save:expr) => {
        Ok($payload.save_items_with_state($state))
    };
    (items_with_state, $payload:ident, $state:expr, $save:expr, yes) => {
        Ok($payload.save_items_with_state($state))
    };
    (custom, $payload:ident, $state:expr, $save:expr) => {{
        let _ = ($state, $save);
        Ok($payload.save_update_tag())
    }};
    (custom, $payload:ident, $state:expr, $save:expr, yes) => {{
        let _ = $state;
        $payload.save_update_tag($save)
    }};
}

/// What `BlockEntity.preRemoveSideEffects(pos, state)`
/// (`BlockEntity.java:233-237`) and its overrides hand to the level before the
/// record goes, in the shape the level needs to build the entities vanilla
/// `addFreshEntity`s.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockEntityRemovalDrops {
    /// The base body for a payload that is not a `Container`.
    None,
    /// `Containers.dropContents(level, pos, container)` (`Containers.java:11-19`):
    /// one entry per container slot in slot order, empty slots included,
    /// because `dropItemStack` draws a position for every slot
    /// (`Containers.java:25-30`).
    ContainerSlots(Vec<bbb_server_types::item_stack::RuntimeItemStack>),
    /// `LecternBlockEntity.preRemoveSideEffects` (`LecternBlockEntity.java:226-236`):
    /// the book, offset from the block's lower corner by
    /// `0.5 + 0.25 * stepX`, `1.0`, `0.5 + 0.25 * stepZ` for the lectern's
    /// facing.
    LecternBook {
        stack: bbb_server_types::item_stack::RuntimeItemStack,
        offset: [f64; 3],
    },
    /// `JukeboxBlockEntity.popOutTheItem` (`JukeboxBlockEntity.java:48-62`),
    /// which `preRemoveSideEffects` (`:153-155`) runs, then `setRemoved`
    /// (`:126-130`), which also runs when the side effects are skipped.
    Jukebox {
        state: i32,
        record: Option<bbb_server_types::item_stack::RuntimeItemStack>,
        stops_song: bool,
    },
    /// `SculkShriekerBlockEntity.preRemoveSideEffects`
    /// (`SculkShriekerBlockEntity.java:134-138`) over `tryRespond`
    /// (`:140-148`): a shrieking shrieker answers as it goes.
    SculkShriekerResponse {
        warning_level: i32,
        can_summon: bool,
    },
    /// `CreakingHeartBlockEntity.preRemoveSideEffects`
    /// (`CreakingHeartBlockEntity.java:307-309`) over `removeProtector(null)`
    /// (`:311-321`): the bound protector is torn down.
    CreakingHeartProtector { uuid: uuid::Uuid },
    /// `AbstractFurnaceBlockEntity.preRemoveSideEffects`
    /// (`AbstractFurnaceBlockEntity.java:376-381`): the inherited container
    /// drop plus `getRecipesToAwardAndPopExperience(level, atCenterOf(pos))`
    /// (`:345-356`), whose recipe list has no taker on this path.
    Furnace {
        slots: Vec<bbb_server_types::item_stack::RuntimeItemStack>,
        recipes_used: Vec<(String, i32)>,
    },
}

/// The table's `drops:` column: the container body for a `Container`, the two
/// class overrides that are not one, and the empty base body for everything
/// else.
macro_rules! block_entity_removal_drops {
    (no, $module:ident, $payload:ident, $old_state:ident) => {{
        let _ = ($payload, $old_state);
        BlockEntityRemovalDrops::None
    }};
    (yes, $module:ident, $payload:ident, $old_state:ident) => {{
        let _ = $old_state;
        BlockEntityRemovalDrops::ContainerSlots($module::drop_contents($payload))
    }};
    (lectern, $module:ident, $payload:ident, $old_state:ident) => {
        $module::removal_drops($payload, $old_state)
    };
    (jukebox, $module:ident, $payload:ident, $old_state:ident) => {
        $module::removal_drops($payload, $old_state)
    };
    (shrieker, $module:ident, $payload:ident, $old_state:ident) => {{
        $module::removal_response($payload, $old_state)
    }};
    (creaking_heart, $module:ident, $payload:ident, $old_state:ident) => {{
        let _ = $old_state;
        $module::removal_response($payload)
    }};
    (furnace, $module:ident, $payload:ident, $old_state:ident) => {{
        let _ = $old_state;
        BlockEntityRemovalDrops::Furnace {
            slots: $module::drop_contents($payload),
            recipes_used: $payload
                .recipes_used()
                .iter()
                .map(|(id, count)| (id.clone(), *count))
                .collect(),
        }
    }};
}

/// The table's `components:` column: `no` inherits the empty
/// `BlockEntity.collectImplicitComponents` (`BlockEntity.java:302-303`).
macro_rules! block_entity_collect_components {
    (no, $payload:ident, $components:ident) => {{
        let _ = ($payload, $components);
    }};
    (yes, $payload:ident, $components:ident) => {
        $payload.collect_implicit_components($components)
    };
}

macro_rules! block_entity_apply_components {
    (no, $payload:ident, $components:ident) => {{
        let _ = ($payload, $components);
        Ok(())
    }};
    (yes, $payload:ident, $components:ident) => {
        $payload.apply_implicit_components($components)
    };
}

/// The table's `trigger_event:` column: `no` inherits
/// `BlockEntity.triggerEvent`'s false (`BlockEntity.java:239-241`). Named
/// tokens call the payload's override.
macro_rules! block_entity_trigger_event {
    (no, $payload:ident, $a:ident, $b:ident) => {{
        let _ = ($payload, $a, $b);
        false
    }};
    (chest, $payload:ident, $a:ident, $b:ident) => {{
        let _ = $b;
        $payload.trigger_event($a)
    }};
    (ender_chest, $payload:ident, $a:ident, $b:ident) => {
        $payload.trigger_event($a, $b)
    };
    (bell, $payload:ident, $a:ident, $b:ident) => {
        $payload.trigger_event($a, $b)
    };
    (shulker_box, $payload:ident, $a:ident, $b:ident) => {
        $payload.trigger_event($a, $b)
    };
    (spawner, $payload:ident, $a:ident, $b:ident) => {
        $payload.trigger_event($a, $b)
    };
    (end_gateway, $payload:ident, $a:ident, $b:ident) => {
        $payload.trigger_event($a, $b)
    };
    (decorated_pot, $payload:ident, $a:ident, $b:ident) => {{
        let _ = $payload;
        $a == decorated_pot::EVENT_POT_WOBBLES
            && ($b == decorated_pot::WobbleStyle::Positive as i32
                || $b == decorated_pot::WobbleStyle::Negative as i32)
    }};
}

/// Declares the only binding from a block entity type to a payload.
///
/// Vanilla spreads one type's answers over its `EntityBlock` and its
/// `BlockEntity` subclass: `newBlockEntity`, `getTicker`, the
/// `saveAdditional` / `loadAdditional` pair, `getUpdateTag`,
/// `getUpdatePacket` and `preRemoveSideEffects`. Each of those used to be its
/// own hand-written `match` over [`BlockEntityPayload`], plus a pair of
/// accessors that spelled out every other variant to answer `None`, so one
/// new type meant editing two dozen places that could each drift apart. The
/// table states all seven answers per type instead, one row, and expands to the
/// enum, [`payload_for_block_entity_type`], each of the dispatchers and
/// the accessor pairs.
///
/// Every column but `new:` and `accessors:` is mandatory, which is what
/// replaces the exhaustiveness the deleted `match`es used to force: a row that
/// leaves a column out does not compile, so a new type cannot silently inherit
/// an answer. The generated dispatchers stay exhaustive over the enum for the
/// same reason.
///
/// The columns:
///
/// * `types:` — the [`bbb_protocol::block_entity_types`] ids whose
///   `newBlockEntity` builds this payload, and the only list of them.
/// * `new:` — the construction expression; the payload's `Default` when the
///   column is left out.
/// * `ticker:` — `EntityBlock.getTicker` hands the server a ticker.
/// * `saves:` — the payload owns a `saveAdditional` / `loadAdditional` pair
///   rather than inheriting the empty base bodies.
/// * `items:` — the payload's own fields hold item stacks, so the pair above
///   takes the record's [`BlockEntityItemProjection`] and resolves its named
///   registry holders. Left out for a payload whose fields hold no stack; a
///   `saves: no` row that declares it does not compile.
/// * `update_tag:` — `getUpdateTag` is `saveCustomOnly` (`additional`) or the
///   inherited empty compound (`empty`).
/// * `update_packet:` — `getUpdatePacket` overrides the base null.
/// * `drops:` — `preRemoveSideEffects` runs the module's `drop_contents`.
/// * `components:` — `collectImplicitComponents` is overridden, and the
///   payload's own `collect_implicit_components` answers it.
/// * `trigger_event:` — `BlockEntity.triggerEvent`; `no` inherits false.
/// * `listener:` — `EntityBlock.getListener`; `no` inherits the interface's
///   null, otherwise the listener radius and delivery mode the object
///   registers with its section.
/// * `accessors:` — the borrow pair and the noun their doc names it by; left
///   out for a payload nothing outside this module reaches yet.
macro_rules! block_entity_payloads {
    ($(
        $(#[$variant_meta:meta])*
        $variant:ident($module:ident::$object:ident) {
            types: [$($type_id:ident),+ $(,)?],
            $(new: $new:expr,)?
            ticker: $ticker:tt,
            saves: $saves:tt,
            $(items: $items:tt,)?
            update_tag: $update_tag:tt,
            update_packet: $update_packet:tt,
            drops: $drops:tt,
            components: $components:tt,
            trigger_event: $trigger_event:tt,
            listener: $listener:tt,
            $(accessors: {
                $get:ident,
                $(#[$get_mut_meta:meta])*
                $get_mut:ident,
                $noun:literal $(,)?
            },)?
        }
    ),* $(,)?) => {
        /// The per-type half of a [`BlockEntity`]: `loadAdditional` /
        /// `saveAdditional` (`BlockEntity.java:95-96`, `:107-108`) and whatever
        /// state sits between them.
        #[derive(Debug, Clone, PartialEq)]
        pub enum BlockEntityPayload {
            $(
                $(#[$variant_meta])*
                $variant($module::$object),
            )*
        }

        /// The payload half of `EntityBlock.newBlockEntity`
        /// (`EntityBlock.java:14`), which is what makes a type "owned" here.
        fn payload_for_block_entity_type(type_id: i32) -> Option<BlockEntityPayload> {
            match type_id {
                $(
                    $($type_id)|+ => Some(BlockEntityPayload::$variant(
                        block_entity_payload_new!($module, $object $(, $new)?),
                    )),
                )*
                _ => None,
            }
        }

        /// `BlockState.getTicker(level, type)` (`BlockBehaviour.java:855-857`)
        /// reaching `EntityBlock.getTicker` (`EntityBlock.java:16-18`), whose
        /// base answer is null, over the table's `ticker:` column — each
        /// through `BaseEntityBlock.createTickerHelper`
        /// (`BaseEntityBlock.java:35-39`), which is the identity for the type
        /// the block actually carries.
        pub(crate) fn has_ticker(block_entity: &BlockEntity) -> bool {
            match &block_entity.payload {
                $(BlockEntityPayload::$variant(_) => block_entity_yes_no!($ticker),)*
            }
        }

        /// Whether `BlockEntity.getUpdatePacket` (`BlockEntity.java:213-215`)
        /// is overridden at all, which is what decides between a packet and
        /// vanilla's null.
        fn payload_sends_update_packet(payload: &BlockEntityPayload) -> bool {
            match payload {
                $(BlockEntityPayload::$variant(_) => block_entity_yes_no!($update_packet),)*
            }
        }

        impl BlockEntity {
            /// `EntityBlock.getListener(level, blockEntity)`
            /// (`EntityBlock.java:20-22`) over the table's `listener:` column:
            /// the listener this object registers with its section, or `None`
            /// for a type whose `EntityBlock` leaves the interface default.
            #[must_use]
            pub fn game_event_listener_entry(&self) -> Option<GameEventListenerEntry> {
                let pos = self.block_pos();
                match &self.payload {
                    $(
                        BlockEntityPayload::$variant(_) => {
                            block_entity_listener!($listener, pos, self.block_state())
                        }
                    )*
                }
            }
        }

        impl BlockEntity {
            /// `BlockEntity.preRemoveSideEffects(pos, state)`
            /// (`BlockEntity.java:233-237`): the base body is
            /// `Containers.dropContents` for a `Container`, and the three 26.1
            /// overrides that are not one answer through the table's `drops:`
            /// column.
            ///
            /// `old_state` is the state the object is being removed *from*,
            /// which is what the overrides read (`LecternBlockEntity.java:227`).
            #[must_use]
            pub fn pre_remove_side_effects(&self, old_state: i32) -> BlockEntityRemovalDrops {
                match &self.payload {
                    $(
                        BlockEntityPayload::$variant(payload) => {
                            block_entity_removal_drops!($drops, $module, payload, old_state)
                        }
                    )*
                }
            }

            /// `BlockEntity.collectImplicitComponents` over the table's
            /// `components:` column.
            fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
                match &self.payload {
                    $(
                        BlockEntityPayload::$variant(payload) => {
                            block_entity_collect_components!($components, payload, components)
                        }
                    )*
                }
            }

            fn apply_implicit_components(
                &mut self,
                components: &mut ComponentMapReader<'_>,
            ) -> bbb_server_types::Result<()> {
                match &mut self.payload {
                    $(
                        BlockEntityPayload::$variant(payload) => {
                            block_entity_apply_components!($components, payload, components)
                        }
                    )*
                }
            }

            /// `BlockEntity.saveAdditional` (`BlockEntity.java:107-108`) for
            /// the owned types; the base body is empty.
            fn save_additional(
                &self,
                save: &mut BlockEntityItemSave<'_>,
            ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
                match &self.payload {
                    $(
                        BlockEntityPayload::$variant(payload) => {
                            block_entity_save_additional!($saves, payload, save $(, $items)?)
                        }
                    )*
                }
            }

            /// `BlockEntity.loadAdditional` (`BlockEntity.java:95-96`).
            fn load_additional(
                &mut self,
                record: &[NbtSummaryEntry],
                load: &mut BlockEntityItemLoad<'_>,
            ) -> bbb_server_types::Result<()> {
                match &mut self.payload {
                    $(
                        BlockEntityPayload::$variant(payload) => {
                            block_entity_load_additional!(
                                $module, $saves, payload, record, load, self.block_state $(, $items)?
                            )
                        }
                    )*
                }
            }

            /// `BlockEntity.getUpdateTag(registries)`
            /// (`BlockEntity.java:217-219`) over the owned types: the empty
            /// compound unless the type overrides it.
            ///
            /// This is what both wire carriers take: `BlockEntityInfo.create`
            /// (`ClientboundLevelChunkPacketData.java:153-158`) for the chunk
            /// packet and `ClientboundBlockEntityDataPacket.create`
            /// (`ClientboundBlockEntityDataPacket.java:37-39`) for the
            /// single-position update.
            pub fn update_tag(&self) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
                let mut save = BlockEntityItemSave::new(BlockEntityItemProjection::without_resolver());
                self.update_tag_projected(&mut save)
            }

            pub fn update_tag_with_resolver(&self, resolver: &dyn PersistentItemStackHolderResolver)
                -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
                let mut save = BlockEntityItemSave::new(BlockEntityItemProjection::with_resolver(resolver));
                self.update_tag_projected(&mut save)
            }

            fn update_tag_projected(&self, save: &mut BlockEntityItemSave<'_>)
                -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
                match &self.payload {
                    $(
                        BlockEntityPayload::$variant(payload) => {
                            block_entity_update_tag!(
                                $update_tag, payload, self.block_state, save $(, $items)?
                            )
                        }
                    )*
                }
            }

            /// `BlockEntity.triggerEvent(b0, b1)` (`BlockEntity.java:239-241`).
            #[must_use]
            pub fn trigger_event(&mut self, param_a: i32, param_b: i32) -> bool {
                match &mut self.payload {
                    $(
                        BlockEntityPayload::$variant(payload) => {
                            block_entity_trigger_event!($trigger_event, payload, param_a, param_b)
                        }
                    )*
                }
            }

            $($(
                #[doc = concat!("The ", $noun, " half of this object, when it is one.")]
                #[must_use]
                pub fn $get(&self) -> Option<&$module::$object> {
                    match &self.payload {
                        BlockEntityPayload::$variant(payload) => Some(payload),
                        _ => None,
                    }
                }

                #[doc = concat!("The ", $noun, " half of this object, when it is one.")]
                $(#[$get_mut_meta])*
                pub fn $get_mut(&mut self) -> Option<&mut $module::$object> {
                    match &mut self.payload {
                        BlockEntityPayload::$variant(payload) => Some(payload),
                        _ => None,
                    }
                }
            )?)*
        }
    };
}

block_entity_payloads! {
    /// `SignBlockEntity` (`SignBlockEntity.java:36-280`), shared by
    /// `HangingSignBlockEntity` (`HangingSignBlockEntity.java:8-29`), which
    /// adds only render metrics and a sound event.
    Sign(sign::SignBlockEntity) {
        // `SignBlock.newBlockEntity` (`SignBlock.java:85-87`),
        // `CeilingHangingSignBlock.newBlockEntity`
        // (`CeilingHangingSignBlock.java:174-176`) and
        // `WallHangingSignBlock.newBlockEntity`
        // (`WallHangingSignBlock.java:173-175`).
        types: [
            VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
            VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID,
        ],
        // All four sign blocks answer `SignBlockEntity::tick` —
        // `SignBlock.getTicker` (`SignBlock.java:190-192`),
        // `CeilingHangingSignBlock.getTicker`
        // (`CeilingHangingSignBlock.java:179-181`) and
        // `WallHangingSignBlock.getTicker`
        // (`WallHangingSignBlock.java:183-185`).
        ticker: yes,
        saves: yes,
        // `SignBlockEntity.getUpdateTag` (`SignBlockEntity.java:229-231`)
        // answers `saveCustomOnly`, i.e. `saveAdditional` with neither the
        // component map nor the position metadata.
        update_tag: additional,
        // `SignBlockEntity.getUpdatePacket` (`SignBlockEntity.java:224-226`).
        update_packet: yes,
        // `SignBlockEntity` is not a `Container`, so the body is empty.
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
        accessors: { sign, sign_mut, "sign" },
    },
    /// `ChestBlockEntity` (`ChestBlockEntity.java:28-199`).
    Chest(chest::ChestBlockEntity) {
        // `ChestBlock.newBlockEntity` (`ChestBlock.java:322-324`), which
        // `BlockEntityType.CHEST` (`BlockEntityType.java:23-35`) registers for
        // the plain chest and the whole copper-chest family at once, and
        // `TrappedChestBlock.newBlockEntity` (`TrappedChestBlock.java:32-34`),
        // which builds a `TrappedChestBlockEntity`
        // (`TrappedChestBlockEntity.java:12-28`) that still extends
        // `ChestBlockEntity`. The extra override — `signalOpenCount`'s
        // neighbour fan-out (`:18-27`) — lives on the world's opener-count
        // path rather than on a second payload.
        types: [
            VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
            VANILLA_BLOCK_ENTITY_TYPE_TRAPPED_CHEST_ID,
        ],
        // `ChestBlock.getTicker` (`ChestBlock.java:327-329`) answers a ticker
        // only on the client; the server has none, and reaches
        // `ChestBlockEntity.recheckOpen` (`:189-193`) through the scheduled
        // block tick `ChestBlock.tick` (`ChestBlock.java:384-389`) instead.
        ticker: no,
        saves: yes,
        // `ChestBlockEntity.items` (`ChestBlockEntity.java:31`), 27 stacks
        // through `ContainerHelper.saveAllItems` / `loadAllItems`.
        items: yes,
        // `ChestBlockEntity` declares no `getUpdateTag`, so it inherits
        // `BlockEntity.getUpdateTag`'s empty compound
        // (`BlockEntity.java:217-219`) — a chest's contents never ride a
        // chunk packet.
        update_tag: empty,
        // `ChestBlockEntity` declares no `getUpdatePacket`, so it inherits
        // `BlockEntity.getUpdatePacket`'s null (`BlockEntity.java:213-215`)
        // and `ChunkHolder.broadcastBlockEntity` (`ChunkHolder.java:226-233`)
        // sends nothing for it.
        update_packet: no,
        drops: yes,
        // `ChestBlockEntity.triggerEvent` (`ChestBlockEntity.java:120-127`).
        components: yes,
        trigger_event: chest,
        listener: no,
        accessors: { chest, chest_mut, "chest" },
    },
    /// `ComparatorBlockEntity` (`ComparatorBlockEntity.java:8-34`).
    Comparator(comparator::ComparatorBlockEntity) {
        // `ComparatorBlock.newBlockEntity` (`ComparatorBlock.java:192-194`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID],
        // `ComparatorBlock` declares no ticker; output updates are scheduled
        // block ticks (`ComparatorBlock.java:145-155`, `:180-182`).
        ticker: no,
        saves: yes,
        update_tag: empty,
        update_packet: no,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
        accessors: { comparator, comparator_mut, "comparator" },
    },
    /// `HopperBlockEntity` (`HopperBlockEntity.java:31-460`).
    Hopper(hopper::HopperBlockEntity) {
        // `HopperBlock.newBlockEntity` (`HopperBlock.java:89-91`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_HOPPER_ID],
        // `HopperBlock.getTicker` (`HopperBlock.java:94-96`) is
        // `HopperBlockEntity::pushItemsTick` on the server.
        ticker: yes,
        saves: yes,
        items: yes,
        update_tag: empty,
        update_packet: no,
        drops: yes,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: { hopper, hopper_mut, "hopper" },
    },
    /// `BarrelBlockEntity` (`BarrelBlockEntity.java:25-141`).
    Barrel(barrel::BarrelBlockEntity) {
        // `BarrelBlock.newBlockEntity` (`BarrelBlock.java:69-71`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_BARREL_ID],
        // `BarrelBlock.getTicker` is null; `recheckOpen` is the scheduled
        // block tick `BarrelBlock.tick` (`BarrelBlock.java:61-66`).
        ticker: no,
        saves: yes,
        items: yes,
        update_tag: empty,
        update_packet: no,
        drops: yes,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: { barrel, barrel_mut, "barrel" },
    },
    /// `DispenserBlockEntity` (`DispenserBlockEntity.java:16-109`), shared by
    /// `DropperBlockEntity`.
    Dispenser(dispenser::DispenserBlockEntity) {
        // `DispenserBlock.newBlockEntity` (`DispenserBlock.java:143-145`) and
        // `DropperBlock.newBlockEntity` (`DropperBlock.java:43-45`). The
        // dropper only replaces the default name.
        types: [
            VANILLA_BLOCK_ENTITY_TYPE_DISPENSER_ID,
            VANILLA_BLOCK_ENTITY_TYPE_DROPPER_ID,
        ],
        ticker: no,
        saves: yes,
        items: yes,
        update_tag: empty,
        update_packet: no,
        drops: yes,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: { dispenser, dispenser_mut, "dispenser or dropper" },
    },
    /// `ShulkerBoxBlockEntity` (`ShulkerBoxBlockEntity.java:35-270`).
    ShulkerBox(shulker_box::ShulkerBoxBlockEntity) {
        // `ShulkerBoxBlock.newBlockEntity` (`ShulkerBoxBlock.java:64-66`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_SHULKER_BOX_ID],
        // `ShulkerBoxBlock.getTicker` (`ShulkerBoxBlock.java:69-71`) is
        // `ShulkerBoxBlockEntity::tick`.
        ticker: yes,
        saves: yes,
        items: yes,
        update_tag: empty,
        update_packet: no,
        // A shulker box keeps its contents in the item it drops as, so
        // `preRemoveSideEffects` hands nothing back here.
        drops: no,
        components: yes,
        trigger_event: shulker_box,
        listener: no,
        accessors: { shulker_box, shulker_box_mut, "shulker-box" },
    },
    /// `LecternBlockEntity` (`LecternBlockEntity.java:33-257`).
    Lectern(lectern::LecternBlockEntity) {
        // `LecternBlock.newBlockEntity` (`LecternBlock.java:122-124`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_LECTERN_ID],
        // `LecternBlock` declares no ticker; page pulses are scheduled
        // block ticks (`LecternBlock.java:155-158`, `:172-174`).
        ticker: no,
        saves: yes,
        items: yes,
        update_tag: empty,
        update_packet: no,
        // `LecternBlockEntity.preRemoveSideEffects` (`:226-236`) drops the book
        // at the lectern's facing rather than through `Containers`.
        drops: lectern,
        components: no,
        trigger_event: no,
        listener: no,
        accessors: { lectern, lectern_mut, "lectern" },
    },
    /// `ChiseledBookShelfBlockEntity` (`ChiseledBookShelfBlockEntity.java:24-138`).
    ChiseledBookshelf(chiseled_bookshelf::ChiseledBookShelfBlockEntity) {
        // `ChiseledBookShelfBlock.newBlockEntity` (`ChiseledBookShelfBlock.java:154-156`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID],
        ticker: no,
        saves: yes,
        items: yes,
        update_tag: empty,
        update_packet: no,
        drops: yes,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: {
            chiseled_bookshelf,
            chiseled_bookshelf_mut,
            "chiseled-bookshelf",
        },
    },
    /// `DecoratedPotBlockEntity` (`DecoratedPotBlockEntity.java:27-187`).
    DecoratedPot(decorated_pot::DecoratedPotBlockEntity) {
        // `DecoratedPotBlock.newBlockEntity` (`DecoratedPotBlock.java:171-173`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID],
        ticker: no,
        saves: yes,
        // `DecoratedPotBlockEntity.item` (`DecoratedPotBlockEntity.java:29`).
        items: yes,
        // `DecoratedPotBlockEntity.getUpdateTag` (`:71-73`) is
        // `saveCustomOnly`.
        update_tag: additional,
        // `DecoratedPotBlockEntity.getUpdatePacket` (`:66-68`).
        update_packet: yes,
        drops: yes,
        components: yes,
        trigger_event: decorated_pot,
        listener: no,
        accessors: { decorated_pot, decorated_pot_mut, "decorated-pot" },
    },
    /// `ShelfBlockEntity` (`ShelfBlockEntity.java:31-138`).
    Shelf(shelf::ShelfBlockEntity) {
        // `ShelfBlock.newBlockEntity` (`ShelfBlock.java:92-94`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID],
        ticker: no,
        saves: yes,
        items: yes,
        // `ShelfBlockEntity.getUpdateTag` (`:62-69`).
        update_tag: additional,
        // `ShelfBlockEntity.getUpdatePacket` (`:57-59`).
        update_packet: yes,
        drops: yes,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: { shelf, shelf_mut, "shelf" },
    },
    /// `BellBlockEntity` (`BellBlockEntity.java:21-169`).
    Bell(bell::BellBlockEntity) {
        // `BellBlock.newBlockEntity` (`BellBlock.java:281-283`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_BELL_ID],
        // `BellBlock.getTicker` (`BellBlock.java:286-288`) is
        // `BellBlockEntity::serverTick` on the server.
        ticker: yes,
        // `BellBlockEntity` declares neither `saveAdditional` nor
        // `loadAdditional`, so both inherit the empty base bodies.
        saves: no,
        update_tag: empty,
        update_packet: no,
        drops: no,
        components: no,
        trigger_event: bell,
        listener: no,
        accessors: { bell, bell_mut, "bell" },
    },
    /// `CopperGolemStatueBlockEntity` (`CopperGolemStatueBlockEntity.java:16-54`).
    CopperGolemStatue(copper_golem_statue::CopperGolemStatueBlockEntity) {
        // `CopperGolemStatueBlock.newBlockEntity` (`CopperGolemStatueBlock.java:131-133`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID],
        // `CopperGolemStatueBlock` declares no ticker.
        ticker: no,
        saves: no,
        update_tag: empty,
        // `CopperGolemStatueBlockEntity.getUpdatePacket` (`:45-47`).
        update_packet: yes,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
    },
    /// `BeehiveBlockEntity` (`BeehiveBlockEntity.java:51-433`).
    Beehive(beehive::BeehiveBlockEntity) {
        // `BeehiveBlock.newBlockEntity` (`BeehiveBlock.java:279-281`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_BEEHIVE_ID],
        ticker: yes,
        saves: yes,
        update_tag: empty,
        // `BeehiveBlockEntity` inherits `BlockEntity.getUpdatePacket`'s null.
        update_packet: no,
        drops: no,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: { beehive, beehive_mut, "beehive" },
    },
    /// `SkullBlockEntity` (`SkullBlockEntity.java:21-104`).
    Skull(skull::SkullBlockEntity) {
        // `AbstractSkullBlock.newBlockEntity` (`AbstractSkullBlock.java:35-36`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID],
        // Server ticker is client-only animation (`AbstractSkullBlock.java:39-51`).
        ticker: no,
        saves: yes,
        // `SkullBlockEntity.getUpdateTag` (`SkullBlockEntity.java:77-79`)
        // is `saveCustomOnly`.
        update_tag: additional,
        // `SkullBlockEntity.getUpdatePacket` (`SkullBlockEntity.java:72-74`).
        update_packet: yes,
        drops: no,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: {
            skull,
            #[cfg(test)]
            skull_mut,
            "skull",
        },
    },
    /// `JukeboxBlockEntity` (`JukeboxBlockEntity.java:22-169`).
    Jukebox(jukebox::JukeboxBlockEntity) {
        // `JukeboxBlock.newBlockEntity` (`JukeboxBlock.java:90-92`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_JUKEBOX_ID],
        // `JukeboxBlock.getTicker` (`JukeboxBlock.java:120-122`) files one only
        // for `HAS_RECORD`, which this per-payload column cannot say. Filing it
        // always is the same behaviour: the body it runs does nothing without a
        // song, and only a record can start one.
        ticker: yes,
        saves: yes,
        // `JukeboxBlockEntity.item` (`JukeboxBlockEntity.java:25`).
        items: yes,
        update_tag: empty,
        update_packet: no,
        // `JukeboxBlockEntity.preRemoveSideEffects` (`:153-155`) pops the record
        // through `popOutTheItem` rather than through `Containers`.
        drops: jukebox,
        components: no,
        trigger_event: no,
        listener: no,
        accessors: { jukebox, jukebox_mut, "jukebox" },
    },
    /// `CampfireBlockEntity` (`CampfireBlockEntity.java:41-222`).
    Campfire(campfire::CampfireBlockEntity) {
        // `CampfireBlock.newBlockEntity` (`CampfireBlock.java:303-305`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID],
        // `CampfireBlock.getTicker` (`CampfireBlock.java:308-321`) files a
        // ticker for a lit campfire (`cookTick`) and for an unlit one
        // (`cooldownTick`) alike; the state picks the body, not the filing.
        ticker: yes,
        saves: yes,
        items: yes,
        // `CampfireBlockEntity.getUpdateTag` (`:159-164`) is items-only.
        update_tag: items,
        // `CampfireBlockEntity.getUpdatePacket` (`:154-156`).
        update_packet: yes,
        drops: yes,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: { campfire, campfire_mut, "campfire" },
    },
    /// `BrushableBlockEntity` (`BrushableBlockEntity.java:38-247`).
    Brushable(brushable::BrushableBlockEntity) {
        // `BrushableBlock.newBlockEntity` (`BrushableBlock.java:115-117`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_BRUSHABLE_BLOCK_ID],
        ticker: no,
        saves: yes,
        // `BrushableBlockEntity.item` (`BrushableBlockEntity.java:43`).
        items: yes,
        // `BrushableBlockEntity.getUpdateTag` (`:190-198`): writes `hit_direction`
        // and non-empty `item` only, never `LootTable`/`LootTableSeed`.
        update_tag: custom,
        // `BrushableBlockEntity.getUpdatePacket` (`:201-203`).
        update_packet: yes,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
        accessors: {
            brushable,
            brushable_mut,
            "brushable-block",
        },
    },
    /// `AbstractFurnaceBlockEntity` (`AbstractFurnaceBlockEntity.java:41-382`).
    Furnace(furnace::AbstractFurnaceBlockEntity) {
        // `FurnaceBlock.newBlockEntity` (`FurnaceBlock.java:35-37`),
        // `SmokerBlock.newBlockEntity` (`SmokerBlock.java:34-36`) and
        // `BlastFurnaceBlock.newBlockEntity` (`BlastFurnaceBlock.java:35-37`).
        types: [
            VANILLA_BLOCK_ENTITY_TYPE_FURNACE_ID,
            VANILLA_BLOCK_ENTITY_TYPE_SMOKER_ID,
            VANILLA_BLOCK_ENTITY_TYPE_BLAST_FURNACE_ID,
        ],
        // `AbstractFurnaceBlock.createFurnaceTicker` (`AbstractFurnaceBlock.java:86-94`)
        // is `serverTick`, which the world drives with the live recipe map and
        // fuel table.
        ticker: yes,
        saves: yes,
        items: yes,
        update_tag: empty,
        update_packet: no,
        drops: furnace,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: { furnace, furnace_mut, "furnace, smoker, or blast furnace" },
    },
    /// `BrewingStandBlockEntity` (`BrewingStandBlockEntity.java:28-252`).
    BrewingStand(brewing_stand::BrewingStandBlockEntity) {
        // `BrewingStandBlock.newBlockEntity` (`BrewingStandBlock.java:50-52`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_BREWING_STAND_ID],
        // `BrewingStandBlock.getTicker` (`BrewingStandBlock.java:55-57`) is
        // `serverTick`, which the world drives with the vanilla mix table.
        ticker: yes,
        saves: yes,
        items: yes,
        update_tag: empty,
        update_packet: no,
        drops: yes,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: { brewing_stand, brewing_stand_mut, "brewing-stand" },
    },
    /// `BannerBlockEntity` (`BannerBlockEntity.java:21-72`).
    Banner(banner::BannerBlockEntity) {
        // `AbstractBannerBlock.newBlockEntity` (`AbstractBannerBlock.java:30-32`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_BANNER_ID],
        ticker: no,
        saves: yes,
        update_tag: additional,
        // `BannerBlockEntity.getUpdatePacket` (`:65-67`).
        update_packet: yes,
        drops: no,
        components: yes,
        trigger_event: no,
        listener: no,
    },
    /// `BedBlockEntity` (`BedBlockEntity.java:9-28`).
    Bed(bed::BedBlockEntity) {
        // `BedBlock.newBlockEntity` (`BedBlock.java:291-293`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_BED_ID],
        ticker: no,
        saves: no,
        update_tag: empty,
        // `BedBlockEntity.getUpdatePacket` (`:21-23`).
        update_packet: yes,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
    },
    /// `DaylightDetectorBlockEntity` (`DaylightDetectorBlockEntity.java:6-10`).
    DaylightDetector(daylight_detector::DaylightDetectorBlockEntity) {
        // `DaylightDetectorBlock.newBlockEntity` (`DaylightDetectorBlock.java:101-103`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_DAYLIGHT_DETECTOR_ID],
        // `DaylightDetectorBlock.tickEntity` (`:112-116`) updates POWER on the
        // block; the BE itself has no tick state.
        ticker: yes,
        saves: no,
        update_tag: empty,
        update_packet: no,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
    },
    /// `EnchantingTableBlockEntity` (`EnchantingTableBlockEntity.java:19-48`).
    EnchantingTable(enchanting_table::EnchantingTableBlockEntity) {
        // `EnchantingTableBlock.newBlockEntity` (`EnchantingTableBlock.java:82-84`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_ENCHANTING_TABLE_ID],
        // `bookAnimationTick` is client-only (`EnchantingTableBlock.java:87-89`).
        ticker: no,
        saves: yes,
        update_tag: empty,
        update_packet: no,
        drops: no,
        components: yes,
        trigger_event: no,
        listener: no,
    },
    /// `TheEndPortalBlockEntity` (`TheEndPortalBlockEntity.java:7-18`).
    EndPortal(end_portal::TheEndPortalBlockEntity) {
        // `EndPortalBlock.newBlockEntity` (`EndPortalBlock.java:46-48`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_END_PORTAL_ID],
        ticker: no,
        saves: no,
        update_tag: empty,
        update_packet: no,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
    },
    /// `SculkSensorBlockEntity` (`SculkSensorBlockEntity.java:18-140`), shared
    /// by `CalibratedSculkSensorBlockEntity`
    /// (`CalibratedSculkSensorBlockEntity.java:14-47`), which adds no save
    /// fields.
    SculkSensor(sculk_sensor::SculkSensorBlockEntity) {
        // `SculkSensorBlock.newBlockEntity` (`SculkSensorBlock.java:151-153`)
        // and `CalibratedSculkSensorBlock.newBlockEntity`
        // (`CalibratedSculkSensorBlock.java:36-38`).
        types: [
            VANILLA_BLOCK_ENTITY_TYPE_SCULK_SENSOR_ID,
            VANILLA_BLOCK_ENTITY_TYPE_CALIBRATED_SCULK_SENSOR_ID,
        ],
        // `SculkSensorBlock.getTicker` (`SculkSensorBlock.java:156-163`) and
        // `CalibratedSculkSensorBlock.getTicker`
        // (`CalibratedSculkSensorBlock.java:41-48`) run
        // `VibrationSystem.Ticker.tick` on the server.
        ticker: yes,
        saves: yes,
        update_tag: empty,
        update_packet: no,
        drops: no,
        components: no,
        trigger_event: no,
        listener: sensor,
        accessors: {
            sculk_sensor,
            sculk_sensor_mut,
            "sculk-sensor",
        },
    },
    /// `SculkCatalystBlockEntity` (`SculkCatalystBlockEntity.java:29-124`).
    SculkCatalyst(sculk_catalyst::SculkCatalystBlockEntity) {
        // `SculkCatalystBlock.newBlockEntity` (`SculkCatalystBlock.java:50-52`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_SCULK_CATALYST_ID],
        // `SculkCatalystBlock.getTicker` (`SculkCatalystBlock.java:55-57`) is
        // `serverTick`, which walks the spreader's cursors (`:37-39`).
        ticker: yes,
        saves: yes,
        update_tag: empty,
        update_packet: no,
        drops: no,
        components: no,
        trigger_event: no,
        // `SculkCatalystBlockEntity.getListener` (`:53-55`) is the one
        // `BY_DISTANCE` listener of the three families.
        listener: { radius: 8, delivery: by_distance },
        accessors: { sculk_catalyst, sculk_catalyst_mut, "sculk-catalyst" },
    },
    /// `SculkShriekerBlockEntity` (`SculkShriekerBlockEntity.java:41-225`).
    SculkShrieker(sculk_shrieker::SculkShriekerBlockEntity) {
        // `SculkShriekerBlock.newBlockEntity` (`SculkShriekerBlock.java:94-96`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_SCULK_SHRIEKER_ID],
        // `SculkShriekerBlock.getTicker` (`SculkShriekerBlock.java:135-143`)
        // runs `VibrationSystem.Ticker.tick` on the server.
        ticker: yes,
        saves: yes,
        update_tag: empty,
        update_packet: no,
        // `SculkShriekerBlockEntity.preRemoveSideEffects` (`:133-138`).
        drops: shrieker,
        components: no,
        trigger_event: no,
        // `SculkShriekerBlockEntity.getListener` (`:171-173`) is the shrieker's
        // own `VibrationSystem.Listener`, radius eight (`:179-185`).
        listener: { radius: 8, delivery: unspecified },
        accessors: { sculk_shrieker, sculk_shrieker_mut, "sculk-shrieker" },
    },
    /// `CommandBlockEntity` (`CommandBlockEntity.java:22-186`).
    CommandBlock(command_block::CommandBlockEntity) {
        // `CommandBlock.newBlockEntity` (`CommandBlock.java:55-59`);
        // `BlockEntityType.COMMAND_BLOCK` (`BlockEntityType.java:169-170`)
        // registers impulse, chain, and repeating.
        types: [VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID],
        // Execution is the command surface, not a block-entity ticker.
        ticker: no,
        saves: yes,
        update_tag: empty,
        update_packet: no,
        drops: no,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: {
            command_block,
            command_block_mut,
            "command-block",
        },
    },
    /// `StructureBlockEntity` (`StructureBlockEntity.java:39-553`).
    StructureBlock(structure_block::StructureBlockEntity) {
        // `StructureBlock.newBlockEntity` (`StructureBlock.java:38-40`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID],
        ticker: no,
        saves: yes,
        // `StructureBlockEntity.getUpdateTag` (`:144-146`) is `saveCustomOnly`.
        update_tag: additional,
        // `StructureBlockEntity.getUpdatePacket` (`:139-141`).
        update_packet: yes,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
        accessors: { structure_block, structure_block_mut, "structure_block" },
    },
    /// `JigsawBlockEntity` (`JigsawBlockEntity.java:25-166`).
    Jigsaw(jigsaw::JigsawBlockEntity) {
        // `JigsawBlock.newBlockEntity` (`JigsawBlock.java:64-66`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID],
        ticker: no,
        saves: yes,
        // `JigsawBlockEntity.getUpdateTag` (`:135-137`) is `saveCustomOnly`.
        update_tag: additional,
        // `JigsawBlockEntity.getUpdatePacket` (`:130-132`).
        update_packet: yes,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
        accessors: { jigsaw, jigsaw_mut, "jigsaw" },
    },
    /// `TestBlockEntity` (`TestBlockEntity.java:18-127`).
    TestBlock(test_block::TestBlockEntity) {
        // `TestBlock.newBlockEntity` (`TestBlock.java:38-40`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID],
        ticker: no,
        saves: yes,
        // `TestBlockEntity.getUpdateTag` (`:61-63`) is `saveCustomOnly`.
        update_tag: additional,
        // `TestBlockEntity.getUpdatePacket` (`:56-58`).
        update_packet: yes,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
    },
    /// `TestInstanceBlockEntity` (`TestInstanceBlockEntity.java:55-503`).
    TestInstanceBlock(test_block::TestInstanceBlockEntity) {
        // `TestInstanceBlock.newBlockEntity` (`TestInstanceBlock.java:23-25`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID],
        ticker: no,
        saves: yes,
        // `TestInstanceBlockEntity.getUpdateTag` (`:156-158`) is `saveCustomOnly`.
        update_tag: additional,
        // `TestInstanceBlockEntity.getUpdatePacket` (`:151-153`).
        update_packet: yes,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
    },
    /// `SpawnerBlockEntity` (`SpawnerBlockEntity.java:20-86`).
    Spawner(spawner::SpawnerBlockEntity) {
        // `SpawnerBlock.newBlockEntity` (`SpawnerBlock.java:30-32`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID],
        // `SpawnerBlock.getTicker` (`SpawnerBlock.java:35-37`) is `serverTick`,
        // which `simulation_tick` runs through the shared `BaseSpawner`.
        ticker: yes,
        saves: yes,
        // `SpawnerBlockEntity.getUpdateTag` (`:66-69`) is `saveCustomOnly`
        // minus `SpawnPotentials`.
        update_tag: items,
        // `SpawnerBlockEntity.getUpdatePacket` (`:61-63`).
        update_packet: yes,
        drops: no,
        // `SpawnerBlockEntity.triggerEvent` (`:71-74`) is `onEventTriggered`.
        components: no,
        trigger_event: spawner,
        listener: no,
        accessors: { spawner, spawner_mut, "spawner" },
    },
    /// `TrialSpawnerBlockEntity` (`TrialSpawnerBlockEntity.java:22-93`).
    TrialSpawner(trial_spawner::TrialSpawnerBlockEntity) {
        // `TrialSpawnerBlock.newBlockEntity` (`TrialSpawnerBlock.java:41-43`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID],
        // `TrialSpawnerBlock.getTicker` (`:46-59`) is `tickServer`, which the
        // world runs because the ticker's body needs the level.
        ticker: yes,
        saves: yes,
        // `TrialSpawnerBlockEntity.getUpdateTag` (`:55-57`) is the state-data
        // subset, not `saveAdditional`, and it reads the block state.
        update_tag: items_with_state,
        // `TrialSpawnerBlockEntity.getUpdatePacket` (`:50-52`).
        update_packet: yes,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
        accessors: { trial_spawner, trial_spawner_mut, "trial_spawner" },
    },
    /// `VaultBlockEntity` (`VaultBlockEntity.java:43-100`).
    Vault(vault::VaultBlockEntity) {
        // `VaultBlock.newBlockEntity` (`VaultBlock.java:70-72`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID],
        // `VaultBlock.getTicker` (`VaultBlock.java:80-94`) is `Server.tick`.
        ticker: yes,
        saves: yes,
        items: yes,
        // `VaultBlockEntity.getUpdateTag` (`:59-63`) is `shared_data` only.
        update_tag: items,
        // `VaultBlockEntity.getUpdatePacket` (`:54-56`).
        update_packet: yes,
        // The displayed item is not `Containers.dropContents`.
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
        accessors: { vault, vault_mut, "vault" },
    },
    /// `CreakingHeartBlockEntity` (`CreakingHeartBlockEntity.java:42-356`).
    CreakingHeart(creaking_heart::CreakingHeartBlockEntity) {
        // `CreakingHeartBlock.newBlockEntity` (`CreakingHeartBlock.java:53-55`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_CREAKING_HEART_ID],
        // `CreakingHeartBlock.getTicker` (`:58-66`) is `serverTick` for every
        // state but `UPROOTED`, which the caller gates on.
        ticker: yes,
        saves: yes,
        // `CreakingHeartBlockEntity.getUpdateTag` (`:220-222`) is `saveCustomOnly`.
        update_tag: additional,
        // `CreakingHeartBlockEntity.getUpdatePacket` (`:215-217`).
        update_packet: yes,
        // `CreakingHeartBlockEntity.preRemoveSideEffects` (`:306-309`).
        drops: creaking_heart,
        components: no,
        trigger_event: no,
        listener: no,
        accessors: { creaking_heart, creaking_heart_mut, "creaking-heart" },
    },
    /// `EnderChestBlockEntity` (`EnderChestBlockEntity.java:13-102`).
    EnderChest(ender_chest::EnderChestBlockEntity) {
        // `EnderChestBlock.newBlockEntity` (`EnderChestBlock.java:103-104`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_ENDER_CHEST_ID],
        // `EnderChestBlock.getTicker` (`:107-109`) is client `lidAnimateTick`.
        ticker: no,
        saves: no,
        update_tag: empty,
        update_packet: no,
        drops: no,
        components: no,
        trigger_event: ender_chest,
        listener: no,
        accessors: { ender_chest, ender_chest_mut, "ender-chest" },
    },
    /// `PistonMovingBlockEntity` (`PistonMovingBlockEntity.java:32-404`).
    Piston(piston::PistonMovingBlockEntity) {
        // `MovingPistonBlock.newMovingBlockEntity` (`MovingPistonBlock.java:57-66`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_PISTON_ID],
        // `MovingPistonBlock.getTicker` (`MovingPistonBlock.java:69-70`).
        ticker: yes,
        saves: yes,
        items: yes,
        // `PistonMovingBlockEntity.getUpdateTag` (`:69-72`) is `saveCustomOnly`.
        update_tag: additional,
        update_packet: no,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
    },
    /// `BeaconBlockEntity` (`BeaconBlockEntity.java:51-364`).
    Beacon(beacon::BeaconBlockEntity) {
        // `BeaconBlock.newBlockEntity` (`BeaconBlock.java:37-38`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_BEACON_ID],
        // `BeaconBlock.getTicker` (`:41-44`) is `tick`.
        ticker: yes,
        saves: yes,
        // `BeaconBlockEntity.getUpdateTag` (`:274-277`) is `saveCustomOnly`.
        update_tag: additional,
        // `BeaconBlockEntity.getUpdatePacket` (`:270-272`).
        update_packet: yes,
        drops: no,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: { beacon, beacon_mut, "beacon" },
    },
    /// `ConduitBlockEntity` (`ConduitBlockEntity.java:35-266`).
    Conduit(conduit::ConduitBlockEntity) {
        // `ConduitBlock.newBlockEntity` (`ConduitBlock.java:50-51`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_CONDUIT_ID],
        // `ConduitBlock.getTicker` (`:54-57`) is `serverTick`.
        ticker: yes,
        saves: yes,
        // `ConduitBlockEntity.getUpdateTag` (`:71-74`) is `saveCustomOnly`.
        update_tag: additional,
        // `ConduitBlockEntity.getUpdatePacket` (`:67-69`).
        update_packet: yes,
        drops: no,
        components: no,
        trigger_event: no,
        listener: no,
        accessors: { conduit, conduit_mut, "conduit" },
    },
    /// `CrafterBlockEntity` (`CrafterBlockEntity.java:25-267`).
    Crafter(crafter::CrafterBlockEntity) {
        // `CrafterBlock.newBlockEntity` (`CrafterBlock.java:106-109`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_CRAFTER_ID],
        // `CrafterBlock.getTicker` (`:95-97`) is `serverTick`.
        ticker: yes,
        saves: yes,
        items: yes,
        update_tag: empty,
        update_packet: no,
        drops: yes,
        components: yes,
        trigger_event: no,
        listener: no,
        accessors: {
            crafter,
            crafter_mut,
            "crafter",
        },
    },
    /// `TheEndGatewayBlockEntity` (`TheEndGatewayBlockEntity.java:29-277`).
    EndGateway(end_gateway::TheEndGatewayBlockEntity) {
        // `EndGatewayBlock.newBlockEntity` (`EndGatewayBlock.java:40-41`).
        types: [VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID],
        // `EndGatewayBlock.getTicker` (`:44-48`) is `portalTick`.
        ticker: yes,
        saves: yes,
        // `TheEndGatewayBlockEntity.getUpdateTag` (`:107-110`) is `saveCustomOnly`.
        update_tag: additional,
        // `TheEndGatewayBlockEntity.getUpdatePacket` (`:103-105`).
        update_packet: yes,
        drops: no,
        components: no,
        trigger_event: end_gateway,
        listener: no,
        accessors: { end_gateway, end_gateway_mut, "end_gateway" },
    },
}

/// `BlockEntity` (`BlockEntity.java:41-56`).
#[derive(Debug, Clone, PartialEq)]
pub struct BlockEntity {
    /// `BlockEntity.type`, as the generated registry id.
    type_id: i32,
    /// `BlockEntity.worldPosition` (`:46`).
    world_position: BlockPos,
    /// `BlockEntity.blockState` (`:48`), the state the object was last
    /// validated against.
    block_state: i32,
    /// `BlockEntity.components` (`:49`), which
    /// `BlockEntity.saveWithoutMetadata` always writes (`:136-139`) and the
    /// save schema always requires.
    components: Vec<NbtSummaryEntry>,
    payload: BlockEntityPayload,
}

impl BlockEntity {
    /// `BlockEntity(type, worldPosition, blockState)`
    /// (`BlockEntity.java:51-56`), whose `validateBlockState` (`:58-62`) is
    /// this constructor's own precondition: every caller here derives the type
    /// from the state.
    fn new(
        type_id: i32,
        world_position: BlockPos,
        block_state: i32,
        payload: BlockEntityPayload,
    ) -> Self {
        debug_assert!(
            block_entity_type_is_valid(type_id, block_state),
            "invalid block entity state"
        );
        Self {
            type_id,
            world_position,
            block_state,
            components: Vec::new(),
            payload,
        }
    }

    #[must_use]
    pub const fn type_id(&self) -> i32 {
        self.type_id
    }

    /// Whether `other` stands for this block entity: the same type at the
    /// same position.
    pub fn same_position_and_type(&self, other: &Self) -> bool {
        self.type_id == other.type_id && self.world_position == other.world_position
    }

    pub fn save_custom_only(&self) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut save = BlockEntityItemSave::new(BlockEntityItemProjection::without_resolver());
        self.save_without_metadata(&mut save)
    }

    pub fn save_custom_only_with_resolver(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut save = BlockEntityItemSave::new(BlockEntityItemProjection::with_resolver(resolver));
        self.save_without_metadata(&mut save)
    }

    /// `BlockEntity.getBlockPos` (`BlockEntity.java:205-207`).
    #[must_use]
    pub const fn block_pos(&self) -> BlockPos {
        self.world_position
    }

    /// `BlockEntity.getBlockState` (`BlockEntity.java:209-211`).
    #[must_use]
    pub const fn block_state(&self) -> i32 {
        self.block_state
    }

    /// `BlockEntity.isValidBlockState` (`BlockEntity.java:64-66`).
    #[must_use]
    pub fn is_valid_block_state(&self, block_state_id: i32) -> bool {
        block_entity_type_is_valid(self.type_id, block_state_id)
    }

    /// `BlockEntity.setBlockState` (`BlockEntity.java:267-271`), which
    /// re-validates before it caches.
    pub(crate) fn set_block_state(&mut self, block_state_id: i32) {
        debug_assert!(self.is_valid_block_state(block_state_id));
        self.block_state = block_state_id;
    }

    #[must_use]
    pub const fn payload(&self) -> &BlockEntityPayload {
        &self.payload
    }

    /// `BlockEntity.saveWithoutMetadata(output)`
    /// (`BlockEntity.java:136-139`): the type's own fields, then the
    /// component map, which is stored whether or not it is empty.
    fn save_without_metadata(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut entries = self.save_additional(save)?;
        entries.push(entry(
            "components",
            NbtSummaryValue::Compound(self.components.clone()),
        ));
        Ok(entries)
    }

    /// `BlockEntity.saveWithFullMetadata(output)`
    /// (`BlockEntity.java:118-121`): the body above plus `saveMetadata`'s id
    /// and position (`:161-166`).
    ///
    /// `CompoundTag` is a hash map, so the order the file ends up in carries
    /// no meaning; this writes vanilla's insertion order because that is the
    /// order the two methods run in.
    pub fn save_with_full_metadata(&self) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut save = BlockEntityItemSave::new(BlockEntityItemProjection::without_resolver());
        self.save_with_full_metadata_projected(&mut save)
    }

    /// [`Self::save_with_full_metadata`] through the registry publication the
    /// caller holds, so a stack whose components name a dynamic-registry holder
    /// keeps them.
    pub fn save_with_full_metadata_with_resolver(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut save = BlockEntityItemSave::new(BlockEntityItemProjection::with_resolver(resolver));
        self.save_with_full_metadata_projected(&mut save)
    }

    fn save_with_full_metadata_projected(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut entries = self.save_without_metadata(save)?;
        let resource_id = vanilla_block_entity_resource_id_for_type_id(self.type_id)
            .expect("a live block entity carries a registered type");
        entries.push(entry("id", NbtSummaryValue::string(resource_id)));
        entries.push(entry("x", NbtSummaryValue::Int(self.world_position.x)));
        entries.push(entry("y", NbtSummaryValue::Int(self.world_position.y)));
        entries.push(entry("z", NbtSummaryValue::Int(self.world_position.z)));
        Ok(entries)
    }

    /// `BlockEntity.loadStatic(pos, state, tag, registries)`
    /// (`BlockEntity.java:168-190`): the record's `id` picks the type, the
    /// type builds the object against the state it is standing on, and
    /// `loadWithComponents` (`:98-101`) fills it.
    ///
    /// `None` where vanilla logs and answers null: an `id` outside the
    /// registry, and a type whose `create` throws — which here is a type
    /// whose state does not accept it, or one this crate does not model.
    ///
    /// This is the resolver-free form; a caller that holds the level's own
    /// publication uses [`Self::load_static_with_resolver`].
    pub fn load_static(
        pos: BlockPos,
        block_state_id: i32,
        record: &[NbtSummaryEntry],
    ) -> bbb_server_types::Result<Option<Self>> {
        Self::load_static_projected(
            pos,
            block_state_id,
            record,
            BlockEntityItemProjection::without_resolver(),
        )
    }

    /// [`Self::load_static`] through the registry publication the caller
    /// holds, so a saved stack carrying a named holder projects.
    pub fn load_static_with_resolver(
        pos: BlockPos,
        block_state_id: i32,
        record: &[NbtSummaryEntry],
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> bbb_server_types::Result<Option<Self>> {
        Self::load_static_projected(
            pos,
            block_state_id,
            record,
            BlockEntityItemProjection::with_resolver(resolver),
        )
    }

    fn load_static_projected(
        pos: BlockPos,
        block_state_id: i32,
        record: &[NbtSummaryEntry],
        projection: BlockEntityItemProjection<'_>,
    ) -> bbb_server_types::Result<Option<Self>> {
        let type_id = match unique_field(record, "id") {
            Some(NbtSummaryValue::String(id)) => id
                .with_utf8(vanilla_block_entity_type_id_for_resource_id)
                .flatten(),
            _ => None,
        };
        let Some(type_id) = type_id else {
            return Ok(None);
        };
        if !block_entity_type_is_valid(type_id, block_state_id) {
            return Ok(None);
        }
        let Some(mut block_entity) = new_block_entity(pos, block_state_id) else {
            return Ok(None);
        };
        if block_entity.type_id != type_id {
            return Ok(None);
        }
        // `BlockEntity.loadWithComponents` (`BlockEntity.java:98-101`): the type's own fields
        // first, the validated component map second.
        let mut load = BlockEntityItemLoad::new(projection);
        block_entity.load_additional(record, &mut load)?;
        block_entity.components = match unique_field(record, "components") {
            Some(NbtSummaryValue::Compound(entries)) => {
                sanitize_block_entity_components(entries, projection)
            }
            _ => Vec::new(),
        };
        Ok(Some(block_entity))
    }
}

/// Validates saved block-entity `"components"` entries (`BlockEntity.java:98-101`),
/// dropping unknown keys, `!` removals, and malformed payloads that would break
/// loot `copy_components` or container drop/pick-block projection.
pub(super) fn sanitize_block_entity_components(
    entries: &[NbtSummaryEntry],
    projection: BlockEntityItemProjection<'_>,
) -> Vec<NbtSummaryEntry> {
    let mut sanitized = Vec::with_capacity(entries.len());
    for entry in entries {
        let Ok(name) = entry.name.try_to_string() else {
            continue;
        };
        if name.starts_with('!') {
            continue;
        }
        let Ok(normalized) = bbb_protocol::normalize_resource_location(name) else {
            continue;
        };
        if bbb_protocol::data_component_types::vanilla_data_component_type_for_resource_id(
            &normalized,
        )
        .is_none()
        {
            continue;
        }
        if projection.has_resolver()
            && bbb_registry_codec::read_component_patch(
                &NbtSummaryValue::Compound(vec![entry.clone()]),
                projection.resolver(),
            )
            .is_err()
        {
            continue;
        }
        sanitized.push(entry.clone());
    }
    sanitized
}

#[cfg(test)]
pub(crate) use crate::inventory::test_item_resolver;

#[cfg(test)]
fn load_test_fixture(
    pos: BlockPos,
    state: i32,
    record: &[NbtSummaryEntry],
) -> bbb_server_types::Result<Option<BlockEntity>> {
    BlockEntity::load_static_with_resolver(pos, state, record, test_item_resolver())
}

#[cfg(test)]
impl BlockEntityItemLoad<'static> {
    pub(super) fn for_test() -> Self {
        Self::new(BlockEntityItemProjection::with_resolver(
            test_item_resolver(),
        ))
    }
}

#[cfg(test)]
impl BlockEntityItemSave<'static> {
    /// The write half of [`BlockEntityItemLoad::for_test`].
    pub(super) fn for_test() -> Self {
        Self::new(BlockEntityItemProjection::with_resolver(
            test_item_resolver(),
        ))
    }
}

pub(crate) fn unique_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &str,
) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

/// `Block.UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS` (`Block.java:99`).
pub const SET_BLOCK_UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS: i32 = 256;

/// `LevelChunk.setBlockState`'s removal arm (`LevelChunk.java:304-319`), which
/// runs `preRemoveSideEffects` before `removeBlockEntity`, and therefore before
/// `affectNeighborsAfterRemoval` and before `onPlace`.
///
/// The position is marked changed whether or not a runtime object stood there:
/// the *record* is what the save has to lose, and this world holds records for
/// block entity types it does not model yet. Leaving one behind would put a
/// chest's contents at a position that is now air, which is worse than losing
/// them — vanilla's own reader warns and drops such a record
/// (`LevelChunk.promotePendingBlockEntity`, `LevelChunk.java:598-619`).
///
/// The returned plan is what `preRemoveSideEffects` handed back; the caller
/// turns it into entities, because that needs the level's random source and its
/// entity store.
#[must_use]
pub fn remove_block_entity_on_block_change(
    mut entities: BlockEntitiesMut<'_>,
    pos: BlockPos,
    old_state: i32,
    new_state: i32,
    flags: i32,
    tags: &dyn BlockTags,
) -> BlockEntityRemovalDrops {
    if !block_change_removes_block_entity(tags, old_state, new_state) {
        return BlockEntityRemovalDrops::None;
    }
    // `LevelChunk.java:307-313`: flag 256 skips the side effects, and they run
    // against the object the level still holds at that position.
    let drops = if flags & SET_BLOCK_UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS == 0 {
        entities
            .get(pos)
            .map_or(BlockEntityRemovalDrops::None, |block_entity| {
                block_entity.pre_remove_side_effects(old_state)
            })
    } else if entities.get(pos).and_then(BlockEntity::jukebox).is_some() {
        BlockEntityRemovalDrops::Jukebox {
            state: old_state,
            record: None,
            stops_song: false,
        }
    } else {
        BlockEntityRemovalDrops::None
    };
    entities.remove(pos);
    entities.set_changed(pos);
    drops
}

/// `LevelChunk.java:304`: `blockChanged` is `!oldState.is(newBlock)`, an
/// identity test on the block rather than on the state, so a property-only
/// write — waterlogging a sign, rotating a shelf — keeps the object.
fn block_change_removes_block_entity(tags: &dyn BlockTags, old_state: i32, new_state: i32) -> bool {
    vanilla_block_state_block_type_id(old_state) != vanilla_block_state_block_type_id(new_state)
        && block_state_has_block_entity(old_state)
        && !should_changed_state_keep_block_entity(tags, old_state, new_state)
}

/// Whether [`remove_block_entity_on_block_change`]'s `preRemoveSideEffects`
/// would first unpack a loot table into the container it empties
/// (`LevelChunk.java:307-313`, [`BlockEntity::removal_unpacks_loot`]).
#[must_use]
pub fn block_change_unpacks_container_loot(
    entities: BlockEntities<'_>,
    pos: BlockPos,
    old_state: i32,
    new_state: i32,
    flags: i32,
    tags: &dyn BlockTags,
) -> bool {
    flags & SET_BLOCK_UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS == 0
        && block_change_removes_block_entity(tags, old_state, new_state)
        && entities
            .get(pos)
            .is_some_and(|block_entity| block_entity.removal_unpacks_loot(old_state))
}

/// `LevelChunk.setBlockState`'s creation arm (`LevelChunk.java:330-347`),
/// which runs after `onPlace`.
///
/// Vanilla's `getBlockEntity(pos, CHECK)` (`:331`) would also promote a record
/// the chunk read but has not turned into an object yet
/// (`LevelChunk.java:368-378`). This world has no such queue: the restore
/// promotes every record it can the moment the save is read, and a record it
/// cannot promote has no type here to promote it into.
pub fn create_block_entity_on_block_change(
    mut entities: BlockEntitiesMut<'_>,
    pos: BlockPos,
    new_state: i32,
) {
    let Some(type_id) = block_entity_type_for_state(new_state) else {
        return;
    };
    // `:331`: the object a body borrowed is still the chunk's, so a write that
    // body makes to its own cell takes the `:343-345` arm, which
    // [`BlockEntitiesMut::put_back`] runs.
    if entities.read().loaned_type(pos) == Some(type_id) {
        return;
    }
    // `:332-336`: an object whose state no longer accepts it is logged and
    // dropped, and the arm below builds a fresh one.
    if entities
        .get(pos)
        .is_some_and(|block_entity| !block_entity.is_valid_block_state(new_state))
    {
        entities.remove(pos);
        entities.set_changed(pos);
    }
    if entities.get(pos).is_some() {
        // `:343-345`.
        entities.rebind_block_state(pos, new_state);
        return;
    }
    // `:338-342`.
    if let Some(block_entity) = new_block_entity(pos, new_state) {
        entities.add_and_register(block_entity);
        entities.set_changed(pos);
    }
}

/// `LevelChunk.getBlockEntityNbtForSaving(pos, registryAccess)`'s live-object
/// arm (`LevelChunk.java:452-458`): the full metadata plus `keepPacked` false,
/// which is what tells a reader to build the object rather than keep the
/// record packed (`SerializableChunkData.postLoadChunk`,
/// `SerializableChunkData.java:502-512`).
///
/// This is the resolver-free form; the save path uses
/// [`saved_record_with_resolver`] so a container's stacks keep the components
/// that name a dynamic-registry holder.
#[cfg(test)]
pub fn saved_record(block_entity: &BlockEntity) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
    let mut save = BlockEntityItemSave::new(BlockEntityItemProjection::without_resolver());
    saved_record_with_save(block_entity, &mut save)
}

/// [`saved_record`] through the registry publication the save was taken under,
/// the writer half of what [`BlockEntity::load_static_with_resolver`] reads back.
pub fn saved_record_with_resolver(
    block_entity: &BlockEntity,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
    let mut save = BlockEntityItemSave::new(BlockEntityItemProjection::with_resolver(resolver));
    saved_record_with_save(block_entity, &mut save)
}

fn saved_record_with_save(
    block_entity: &BlockEntity,
    save: &mut BlockEntityItemSave<'_>,
) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
    let mut entries = block_entity.save_with_full_metadata_projected(save)?;
    entries.push(entry("keepPacked", NbtSummaryValue::Byte(0)));
    Ok(entries)
}

/// `ClientboundBlockEntityDataPacket.create(blockEntity)`
/// (`ClientboundBlockEntityDataPacket.java:37-39`) as
/// `BlockEntity.getUpdatePacket` (`BlockEntity.java:213-215`) answers it.
///
/// `None` is vanilla's null: the base `getUpdatePacket` sends nothing, and
/// `ChunkHolder.broadcastBlockEntity` (`ChunkHolder.java:226-233`) skips a
/// block entity that answers it.
pub fn block_entity_update_packet_with_resolver(
    block_entity: &BlockEntity,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> bbb_server_types::Result<Option<BlockEntityData>> {
    if !payload_sends_update_packet(block_entity.payload()) {
        return Ok(None);
    }
    block_entity_packet(
        block_entity,
        block_entity.update_tag_with_resolver(resolver)?,
    )
    .map(Some)
}

pub fn block_entity_custom_only_packet_with_resolver(
    block_entity: &BlockEntity,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> bbb_server_types::Result<Option<BlockEntityData>> {
    block_entity_packet(
        block_entity,
        block_entity.save_custom_only_with_resolver(resolver)?,
    )
    .map(Some)
}

fn block_entity_packet(
    block_entity: &BlockEntity,
    tag: Vec<NbtSummaryEntry>,
) -> bbb_server_types::Result<BlockEntityData> {
    let raw_nbt =
        encode_nbt_summary_network_quota(&NbtSummaryValue::Compound(tag)).map_err(|error| {
            bbb_server_types::ServerWorldError::InvalidConfiguration(format!(
                "block-entity update NBT: {error}"
            ))
        })?;
    Ok(BlockEntityData {
        pos: block_entity.block_pos(),
        block_entity_type_id: block_entity.type_id(),
        raw_nbt,
    })
}

pub fn chunk_block_entity_with_resolver(
    block_entity: &BlockEntity,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> bbb_server_types::Result<Option<LevelChunkBlockEntity>> {
    chunk_block_entity_with_tag(
        block_entity,
        block_entity.update_tag_with_resolver(resolver)?,
    )
}

fn chunk_block_entity_with_tag(
    block_entity: &BlockEntity,
    tag: Vec<NbtSummaryEntry>,
) -> bbb_server_types::Result<Option<LevelChunkBlockEntity>> {
    let pos = block_entity.block_pos();
    let raw_nbt = if tag.is_empty() {
        vec![0]
    } else {
        encode_nbt_summary_network_quota(&NbtSummaryValue::Compound(tag)).map_err(|error| {
            bbb_server_types::ServerWorldError::InvalidConfiguration(format!(
                "chunk block-entity NBT: {error}"
            ))
        })?
    };
    let Ok(y) = i16::try_from(pos.y) else {
        return Ok(None);
    };
    Ok(Some(LevelChunkBlockEntity {
        packed_xz: (((pos.x & 15) << 4) | (pos.z & 15)) as u8,
        y,
        block_entity_type_id: block_entity.type_id(),
        raw_nbt,
    }))
}

#[cfg(test)]
mod tests;
