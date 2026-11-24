//! `LootTable.getRandomItems` under any `LootContextParamSets` entry.
//!
//! [`bbb_server_datapack::loot`] owns the *definitions*: every loot table,
//! pool, entry, condition, function and number provider is decoded once into
//! typed values. This module owns the *runtime*: it walks those values the way
//! vanilla walks them and answers which item stacks a context draws.
//!
//! A block break fills these parameters:
//!
//! | parameter          | value under `dropResources`                       |
//! |--------------------|---------------------------------------------------|
//! | `BLOCK_STATE`      | the broken state                                  |
//! | `ORIGIN`           | the block's centre                                |
//! | `TOOL`             | [`BlockDropParams::tool`]                         |
//! | `THIS_ENTITY`      | [`BlockDropParams::breaker`]                      |
//! | `BLOCK_ENTITY`     | [`BlockDropParams::block_entity`]                 |
//! | `EXPLOSION_RADIUS` | [`BlockDropParams::explosion_radius`]             |
//! | luck               | `0.0` (`LootParams.Builder.luck` is never set)    |
//! | dynamic drops      | [`crate::block_entity::BlockEntity::dynamic_drop`] |
//!
//! # Fail closed
//!
//! A node this server cannot evaluate ends in an `Err`, never in a silent
//! empty drop: an empty list is the one answer that would look like agreement
//! with the official server while being wrong. [`check_table`] walks a whole
//! table for such nodes — and against a parameter set — so a branch that is
//! never drawn still cannot hide one.

use self::native_contents::ExpansionOutput;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::OnceLock;

use crate::random::GameRandom;
use bbb_facts::environment_attribute_types::{
    VanillaEnvironmentAttribute, VanillaEnvironmentScalarValue,
};
use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_item_default_block_state_properties,
    VanillaBlockStateIdentity,
};
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::data_component_types::{
    vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
};
use bbb_protocol::item_types::{
    vanilla_item_type, vanilla_item_type_for_resource_id, VANILLA_ITEM_AIR_ID, VANILLA_ITEM_TYPES,
};
use bbb_protocol::packets::{BlockPos, ItemStackSummary};
use bbb_protocol::{CommandNbtPath, NbtSummaryValue, PersistentRegistryTagMembers};

/// Loot functions retain item data through a temporary zero count
/// (`SetItemCountFunction.java:42-45`); live inventory stacks normalize empties.
pub type LootStack = ItemStackSummary;
use crate::inventory::max_stack_size;
use bbb_server_types::item_stack::RuntimeItemStack;

use bbb_server_datapack::loot::{
    BonusFormula, IntRange, LootCondition, LootEntry, LootEntryKind, LootFunction,
    LootFunctionKind, LootPool, LootRegistries, LootTable, NestedLootTable, NumberProvider,
    ScoreTarget,
};

/// `LootParams.getLuck` (`LootParams.java:42-44`): `Block.getDrops` never calls
/// `withLuck`, so every `getWeight`/`bonus_rolls` term that scales by luck is
/// scaled by zero.
const DROP_RESOURCES_LUCK: f32 = 0.0;

/// `ItemStack.EMPTY`'s registry id for the one comparison that needs it:
/// `ItemStack.typeHolder` answers `minecraft:air` for an empty stack
/// (`ItemStack.java:350-352`).
const EMPTY_TOOL_RESOURCE_ID: &str = "minecraft:air";

/// Loaded block states read by loot location predicates.
pub type LootBlockStateAt<'a> = dyn Fn(BlockPos) -> Option<i32> + 'a;

#[derive(Clone, Copy)]
pub enum LootBlockEntityRead {
    Nbt(BlockPos),
    Components(BlockPos),
}

pub enum LootBlockEntityData {
    Nbt(NbtSummaryValue),
    Components(crate::block_entity::CollectedComponents),
}

/// Only the block-entity data the predicate requests, at a loaded position.
pub type LootBlockEntityDataAt<'a> = dyn Fn(LootBlockEntityRead) -> std::result::Result<Option<LootBlockEntityData>, LootRuntimeReason>
    + 'a;

/// The biome id at a loaded position, read by a location predicate's `biomes`.
pub type LootBiomeAt<'a> = dyn Fn(BlockPos) -> Option<String> + 'a;

/// `LevelReader.canSeeSky` at a position; `None` is an unloaded position.
pub type LootSkyAt<'a> = dyn Fn(BlockPos) -> Option<bool> + 'a;

/// Max local raw brightness at a loaded position; absent facts stay unknown.
pub type LootLightAt<'a> = dyn Fn(BlockPos) -> Option<i32> + 'a;

/// Structure piece membership at a loaded position; absent facts stay unknown.
pub type LootStructuresAt<'a> =
    dyn Fn(BlockPos, &bbb_server_datapack::loot::HolderSet) -> Option<bool> + 'a;

mod entity_view;
pub use entity_view::{LootEntity, LootEntityFacts, StoredLootEntity};

/// One entity read vanilla can answer without reading a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootEntityAnswer<T> {
    /// The gate the term opens with fails — a null vehicle, or the
    /// `instanceof` a sub-predicate names — so the term is false and no value
    /// is read.
    NotApplicable,
    Value(T),
}

///
/// A plain fact bundle rather than a decision, the way [`LootTool`] is: three
/// of the nine terms are gated on `entity instanceof LivingEntity`
/// (`:57-70`), so the gate is a read of its own and the composition is
/// the loot crate's.
///
/// Every flag is `None` for a view that cannot read it, which makes the term
/// that spells it a refusal instead of a false; the default is therefore a view
/// that reads no flag at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LootEntityFlags {
    /// `Entity.onGround()` (`:37`).
    pub on_ground: Option<bool>,
    /// `Entity.isOnFire()` (`:41`).
    pub on_fire: Option<bool>,
    /// `Entity.isCrouching()` (`:45`), the `is_sneaking` term.
    pub crouching: Option<bool>,
    /// `Entity.isSprinting()` (`:49`).
    pub sprinting: Option<bool>,
    /// `Entity.isSwimming()` (`:53`).
    pub swimming: Option<bool>,
    /// `Entity.isInWater()` (`:65`).
    pub in_water: Option<bool>,
    /// `LivingEntity.isFallFlying()`, read by `is_flying` (`:58-59`) and
    /// `is_fall_flying` (`:68`).
    pub fall_flying: Option<bool>,
    /// `Player.getAbilities().flying`, the second disjunct of `is_flying`
    /// (`:59`).
    pub ability_flying: Option<bool>,
    /// `LivingEntity.isBaby()` (`:70`).
    pub baby: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootEntityPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// One entity's movement as the `movement` term reads it: the per-tick
/// known movement and the fall distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootEntityMovement {
    pub known_movement: [f64; 3],
    pub fall_distance: f64,
}

/// The `ServerPlayer` reads `PlayerPredicate` makes for its level, food and
/// game-mode terms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootPlayerFacts {
    pub experience_level: i32,
    pub food_level: i32,
    pub saturation_level: f32,
    pub game_mode: &'static str,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LootRaiderFlags {
    pub has_raid: Option<bool>,
    pub is_captain: Option<bool>,
}

/// `LootContextParams.TOOL`: the stack a `Block.getDrops` overload puts in the
/// parameter set.
///
///
/// The enchantment map is keyed by *resource id* rather than by the registry id
/// a `bbb_protocol::packets::ItemStackSummary` carries, because
/// `minecraft:enchantment` is a datapack registry whose ids are handed out in
/// the order `RegistryDataLoader` lists its directory. Only the loaded
/// generation can turn one into the other, so that resolution belongs to the
/// caller; `crate::entity::block_drops::loot_tool_for_stack` is the one that does it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LootTool {
    /// The whole stack, which `copy_components` reads.
    pub stack: RuntimeItemStack,
    pub item_id: Option<i32>,
    pub count: i32,
    pub enchantments: BTreeMap<String, i32>,
}

impl LootTool {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            stack: RuntimeItemStack::empty(),
            item_id: None,
            count: 0,
            enchantments: BTreeMap::new(),
        }
    }

    fn is_empty(&self) -> bool {
        !matches!(self.item_id, Some(id) if id != VANILLA_ITEM_AIR_ID) || self.count <= 0
    }

    fn item_resource_id_with_registry<'a>(
        &self,
        content_registry: &'a bbb_protocol::content_registry::ContentRegistry,
    ) -> std::result::Result<&'a str, LootRuntimeReason> {
        if self.is_empty() {
            return Ok(EMPTY_TOOL_RESOURCE_ID);
        }
        let item_id = self.item_id.expect("a non-empty stack carries an item id");
        if let Some(entry) = vanilla_item_type(item_id) {
            return Ok(entry.resource_id);
        }
        content_registry
            .item_by_id(item_id)
            .map(bbb_protocol::content_registry::ModItemSpec::resource_id)
            .ok_or_else(|| LootRuntimeReason::UnknownItem(item_id.to_string()))
    }

    fn count(&self) -> i32 {
        if self.is_empty() {
            0
        } else {
            self.count
        }
    }

    #[must_use]
    pub fn enchantment_level(&self, resource_id: &str) -> i32 {
        if self.is_empty() {
            return 0;
        }
        self.enchantments.get(resource_id).copied().unwrap_or(0)
    }
}

/// The `Block.getDrops` inputs, for either overload.
pub struct BlockDropParams<'a> {
    /// `LootContextParams.BLOCK_STATE`: the global block-state id being broken.
    pub block_state_id: i32,
    /// The position whose centre becomes `LootContextParams.ORIGIN`.
    pub pos: BlockPos,
    /// The actual level resource key; absent facts fail dimension predicates.
    pub dimension: Option<&'a str>,
    /// The loaded blocks a `location_check` reads.
    pub level: Option<&'a LootBlockStateAt<'a>>,
    pub block_entity_data: Option<&'a LootBlockEntityDataAt<'a>>,
    /// `LootContextParams.TOOL`.
    pub tool: &'a LootTool,
    /// `LootContextParams.THIS_ENTITY`.
    pub breaker: LootEntity<'a>,
    /// `LootContextParams.EXPLOSION_RADIUS`.
    pub explosion_radius: Option<f32>,
    /// `LootContextParams.BLOCK_ENTITY`: `state.hasBlockEntity() ?
    /// level.getBlockEntity(pos) : null` for every caller but a player's break,
    /// which hands over the object it read before the removal.
    pub block_entity: Option<LootBlockEntity<'a>>,
    /// Where a component-writing function applies its components. `None`
    /// refuses the ones that have something to write.
    pub item_environment: Option<&'a dyn LootItemEnvironment>,
}

/// Why a loot table could not be evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootRuntimeError {
    /// The loot table being evaluated, e.g. `minecraft:blocks/stone`.
    pub table_id: String,
    /// A JSON-path-like location inside it, e.g. `pools[0].entries[1]`.
    pub path: String,
    pub reason: LootRuntimeReason,
}

/// The kinds of gap [`block_drops`] refuses to paper over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LootRuntimeReason {
    UnsupportedEntry(String),
    UnsupportedCondition(String),
    UnsupportedFunction(String),
    UnsupportedNumberProvider(String),
    /// A field whose value needs a mechanism this crate does not have.
    UnsupportedField {
        owner: &'static str,
        detail: String,
    },
    /// A field the vanilla codec requires, missing or of the wrong shape.
    MalformedField {
        owner: &'static str,
        field: &'static str,
    },
    UnknownItem(String),
    UnknownBlockState(i32),
    ParameterNotProvided(LootContextParam),
    ParameterNotAllowed(LootContextParam),
    /// A caller left out a parameter the set requires (the same check).
    MissingRequiredParameter(LootContextParam),
    UnfollowableReference {
        reference: String,
        problem: ReferenceProblem,
    },
    /// A `location_check` was reached without a level to read.
    NoLevelView,
    /// A generated catalog lookup failed.
    Catalog(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceProblem {
    NoResolver,
    /// `ValidationContext.RecursiveReferenceProblem`: the key is one of the
    /// tables this walk is already inside.
    Recursive,
    /// `ValidationContext.MissingReferenceProblem`: the registry has no
    /// element for the key.
    Missing,
}

impl fmt::Display for LootRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}: {}", self.table_id, self.path, self.reason)
    }
}

impl fmt::Display for LootRuntimeReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LootRuntimeReason::UnsupportedEntry(id) => write!(f, "unsupported entry type {id}"),
            LootRuntimeReason::UnsupportedCondition(id) => {
                write!(f, "unsupported condition type {id}")
            }
            LootRuntimeReason::UnsupportedFunction(id) => {
                write!(f, "unsupported function type {id}")
            }
            LootRuntimeReason::UnsupportedNumberProvider(id) => {
                write!(f, "unsupported number provider type {id}")
            }
            LootRuntimeReason::UnsupportedField { owner, detail } => {
                write!(f, "{owner} needs a mechanism this crate lacks: {detail}")
            }
            LootRuntimeReason::MalformedField { owner, field } => {
                write!(f, "{owner} has no usable `{field}`")
            }
            LootRuntimeReason::UnknownItem(id) => write!(f, "unknown item {id}"),
            LootRuntimeReason::UnknownBlockState(id) => write!(f, "unknown block state id {id}"),
            LootRuntimeReason::ParameterNotProvided(param) => write!(
                f,
                "{} is not carried by this parameter set",
                param.resource_id()
            ),
            LootRuntimeReason::ParameterNotAllowed(param) => write!(
                f,
                "{} is not allowed in this parameter set",
                param.resource_id()
            ),
            LootRuntimeReason::MissingRequiredParameter(param) => {
                write!(f, "this parameter set requires {}", param.resource_id())
            }
            LootRuntimeReason::UnfollowableReference { reference, problem } => match problem {
                ReferenceProblem::NoResolver => {
                    write!(
                        f,
                        "{reference} was referenced with no registry to resolve it in"
                    )
                }
                ReferenceProblem::Recursive => write!(f, "{reference} is recursively called"),
                ReferenceProblem::Missing => write!(f, "missing element {reference}"),
            },
            LootRuntimeReason::NoLevelView => {
                write!(f, "a location_check needs a level and none was supplied")
            }
            LootRuntimeReason::Catalog(message) => write!(f, "catalog lookup failed: {message}"),
        }
    }
}

impl std::error::Error for LootRuntimeError {}

type Result<T> = std::result::Result<T, LootRuntimeError>;

/// The loot table key a block state's block declares.
///
#[cfg(test)]
pub fn block_loot_table_id(block_state_id: i32) -> Option<String> {
    block_loot_table_id_with_registry(
        block_state_id,
        bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
    )
}

pub fn block_loot_table_id_with_registry(
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<String> {
    if let Some(block_type_id) = vanilla_block_state_block_type_id(block_state_id) {
        if !vanilla_block_type(block_type_id)?.has_loot_table {
            return None;
        }
        let effective = wall_variant_standing_block(block_type_id).unwrap_or(block_type_id);
        let resource_id = vanilla_block_type(effective)?.resource_id;
        let (namespace, path) = resource_id.split_once(':')?;
        return Some(format!("{namespace}:blocks/{path}"));
    }
    let (custom, _) = content_registry.block_by_state_id(block_state_id)?;
    let (namespace, path) = custom.resource_id().split_once(':')?;
    Some(format!("{namespace}:blocks/{path}"))
}

/// Wall block id → the standing block whose loot table it borrows.
///
/// `Blocks.wallVariant` is the only caller of `overrideLootTable` in 26.1, and
/// every block it builds is placed by a `StandingAndWallBlockItem`-shaped item
/// that names both halves, so the generated item catalog already carries the
/// pairing and no second table is written here.
fn wall_variant_standing_block(block_type_id: i32) -> Option<i32> {
    static WALL_VARIANTS: OnceLock<BTreeMap<i32, i32>> = OnceLock::new();
    WALL_VARIANTS
        .get_or_init(|| {
            VANILLA_ITEM_TYPES
                .iter()
                .filter_map(|entry| {
                    let behavior = entry.block_item_behavior?;
                    Some((behavior.wall_block_type_id?, behavior.block_type_id))
                })
                .collect()
        })
        .get(&block_type_id)
        .copied()
}

pub fn block_loot_table_with_registry<'a>(
    tables: &'a LootRegistries,
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<(String, &'a LootTable)> {
    let table_id = block_loot_table_id_with_registry(block_state_id, content_registry)?;
    let table = tables.table(&table_id)?;
    Some((table_id, table))
}

pub fn block_drops(
    tables: &LootRegistries,
    params: &BlockDropParams<'_>,
    random: &GameRandom,
) -> Result<Vec<LootStack>> {
    block_drops_with_text(tables, params, random, None, None)
}

pub(crate) fn block_drops_with_text(
    tables: &LootRegistries,
    params: &BlockDropParams<'_>,
    random: &GameRandom,
    text_resolver: Option<&dyn LootTextResolver>,
    vehicle_slots: Option<&params::LootVehicleSlots<'_>>,
) -> Result<Vec<LootStack>> {
    let mut inputs = block_loot_params(params, true);
    inputs.text_resolver = text_resolver;
    inputs.vehicle_slots = vehicle_slots;
    let context = inputs.create().map_err(|reason| LootRuntimeError {
        table_id: String::new(),
        path: "params".to_string(),
        reason,
    })?;
    let content_registry = params.item_environment.map_or_else(
        || bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
        |env| env.item_resolver().content_registry(),
    );
    let Some((table_id, table)) =
        block_loot_table_with_registry(tables, params.block_state_id, content_registry)
    else {
        return Ok(Vec::new());
    };
    let mut eval = Eval {
        table_id: &table_id,
        tables,
        context,
        visited: Vec::new(),
        visited_functions: Vec::new(),
        visited_conditions: Vec::new(),
        random,
    };
    eval.get_random_items(table)
}

fn block_loot_params<'a>(params: &'a BlockDropParams<'a>, dynamic_drops: bool) -> LootParams<'a> {
    LootParams {
        item_environment: params.item_environment,
        block_entity: params.block_entity,
        dynamic_drops: params
            .block_entity
            .filter(|_| dynamic_drops)
            .map(|block_entity| block_entity.block_entity),
        origin: Some(LootOrigin::at_center_of(params.pos)),
        block_state_id: Some(params.block_state_id),
        tool: Some(params.tool),
        explosion_radius: params.explosion_radius,
        entities: LootEntities {
            this_entity: params.breaker,
            ..LootEntities::default()
        },
        dimension: params.dimension,
        level: params.level,
        block_entity_data: params.block_entity_data,
        luck: DROP_RESOURCES_LUCK,
        ..LootParams::new(param_sets::BLOCK)
    }
}

pub fn loot_table_drops(
    tables: &LootRegistries,
    table_id: &str,
    params: &LootParams<'_>,
    random: &GameRandom,
) -> Result<Vec<LootStack>> {
    draw_table(
        tables,
        table_id,
        loot_table_by_id(tables, table_id),
        params,
        random,
    )
}

pub fn loot_table_drops_from_table(
    tables: &LootRegistries,
    table_id: &str,
    table: &LootTable,
    params: &LootParams<'_>,
    random: &GameRandom,
) -> Result<Vec<LootStack>> {
    draw_table(tables, table_id, Some(table), params, random)
}

fn draw_table(
    tables: &LootRegistries,
    table_id: &str,
    table: Option<&LootTable>,
    params: &LootParams<'_>,
    random: &GameRandom,
) -> Result<Vec<LootStack>> {
    let context = params.create().map_err(|reason| LootRuntimeError {
        table_id: table_id.to_string(),
        path: "params".to_string(),
        reason,
    })?;
    let Some(table) = table else {
        return Ok(Vec::new());
    };
    let mut eval = Eval {
        table_id,
        tables,
        context,
        visited: Vec::new(),
        visited_functions: Vec::new(),
        visited_conditions: Vec::new(),
        random,
    };
    eval.get_random_items(table)
}

pub fn loot_table_by_id<'a>(tables: &'a LootRegistries, table_id: &str) -> Option<&'a LootTable> {
    tables.table(table_id)
}

/// Block interaction drops, without the ORIGIN that only destruction provides.
pub fn named_table_drops(
    tables: &LootRegistries,
    table_id: &str,
    params: &BlockDropParams<'_>,
    random: &GameRandom,
) -> Result<Vec<LootStack>> {
    let context = LootParams {
        item_environment: params.item_environment,
        block_entity: params.block_entity,
        block_state_id: Some(params.block_state_id),
        tool: Some(params.tool),
        entities: LootEntities {
            interacting_entity: params.breaker,
            ..Default::default()
        },
        dimension: params.dimension,
        level: params.level,
        block_entity_data: params.block_entity_data,
        ..LootParams::new(param_sets::BLOCK_INTERACT)
    };
    loot_table_drops(tables, table_id, &context, random)
}

/// Walk a whole table and report the first node this module could not evaluate
/// under `param_set`, whatever the draws would have been.
#[cfg(test)]
pub fn check_table(
    table_id: &str,
    table: &LootTable,
    param_set: &'static LootContextParamSet,
) -> Result<()> {
    check_table_in(None, table_id, table, param_set)
}

pub fn check_modifier_codec(function: &LootFunction, tables: &LootRegistries) -> Result<()> {
    Checker {
        table_id: "item_modifier",
        param_set: param_sets::ALL_PARAMS,
        tables: Some(tables),
        visited: Vec::new(),
    }
    .function(function, "modifier")
}

#[cfg(test)]
pub fn check_table_in(
    tables: Option<&LootRegistries>,
    table_id: &str,
    table: &LootTable,
    param_set: &'static LootContextParamSet,
) -> Result<()> {
    Checker {
        table_id,
        param_set,
        tables,
        visited: vec![table_id.to_string()],
    }
    .table(table, "")
}

#[cfg(test)]
pub fn check_block_table(table_id: &str, table: &LootTable) -> Result<()> {
    check_table(table_id, table, param_sets::BLOCK)
}

/// `owner` names the list the way `table_id` names a table in an error.
pub fn matches_conditions(
    tables: &LootRegistries,
    owner: &str,
    conditions: &[LootCondition],
    params: &LootParams<'_>,
    random: &GameRandom,
) -> Result<bool> {
    Eval::new(owner, tables, params, random)?.all_of(conditions, "")
}

pub fn check_conditions(
    owner: &str,
    conditions: &[LootCondition],
    param_set: &'static LootContextParamSet,
) -> Result<()> {
    Checker {
        table_id: owner,
        param_set,
        tables: None,
        visited: Vec::new(),
    }
    .conditions(conditions, "")
}

struct Checker<'a> {
    table_id: &'a str,
    param_set: &'static LootContextParamSet,
    /// The registry a `minecraft:loot_table` entry's reference is followed in.
    /// `None` walks this table alone.
    tables: Option<&'a LootRegistries>,
    visited: Vec<String>,
}

impl Checker<'_> {
    fn err(&self, path: &str, reason: LootRuntimeReason) -> LootRuntimeError {
        LootRuntimeError {
            table_id: self.table_id.to_string(),
            path: path.to_string(),
            reason,
        }
    }

    fn at(&self, path: &str, result: std::result::Result<(), LootRuntimeReason>) -> Result<()> {
        result.map_err(|reason| self.err(path, reason))
    }

    /// `ValidationContext.validateContextUsage`.
    fn referenced(&self, path: &str, param: LootContextParam) -> Result<()> {
        if self.param_set.allows(param) {
            Ok(())
        } else {
            Err(self.err(path, LootRuntimeReason::ParameterNotProvided(param)))
        }
    }

    fn unfollowable(
        &self,
        path: &str,
        reference: &str,
        problem: ReferenceProblem,
    ) -> LootRuntimeError {
        self.err(
            path,
            LootRuntimeReason::UnfollowableReference {
                reference: reference.to_string(),
                problem,
            },
        )
    }

    fn table(&mut self, table: &LootTable, prefix: &str) -> Result<()> {
        self.functions(&table.functions, &format!("{prefix}functions"))?;
        for (index, pool) in table.pools.iter().enumerate() {
            let path = format!("{prefix}pools[{index}]");
            self.conditions(&pool.conditions, &format!("{path}.conditions"))?;
            self.functions(&pool.functions, &format!("{path}.functions"))?;
            self.number(&pool.rolls, &format!("{path}.rolls"))?;
            self.number(&pool.bonus_rolls, &format!("{path}.bonus_rolls"))?;
            for (entry_index, entry) in pool.entries.iter().enumerate() {
                self.entry(entry, &format!("{path}.entries[{entry_index}]"))?;
            }
        }
        Ok(())
    }

    fn entry(&mut self, entry: &LootEntry, path: &str) -> Result<()> {
        self.conditions(&entry.conditions, &format!("{path}.conditions"))?;
        self.functions(&entry.functions, &format!("{path}.functions"))?;
        match &entry.kind {
            LootEntryKind::LootTable(NestedLootTable::Key(reference)) => {
                let Some(tables) = self.tables else {
                    return Err(self.unfollowable(path, reference, ReferenceProblem::NoResolver));
                };
                if self.visited.iter().any(|seen| seen == reference) {
                    return Err(self.unfollowable(path, reference, ReferenceProblem::Recursive));
                }
                let Some(nested) = tables.table(reference) else {
                    return Err(self.unfollowable(path, reference, ReferenceProblem::Missing));
                };
                self.visited.push(reference.clone());
                let result = self.table(nested, &format!("{path}->{{{reference}}}."));
                self.visited.pop();
                result
            }
            LootEntryKind::LootTable(NestedLootTable::Inline(table)) => {
                self.table(table, &format!("{path}->inline."))
            }
            LootEntryKind::Slots(source) => self.slot_source(source, path),
            LootEntryKind::Alternatives(children)
            | LootEntryKind::Group(children)
            | LootEntryKind::Sequence(children) => {
                for (index, child) in children.iter().enumerate() {
                    self.entry(child, &format!("{path}.children[{index}]"))?;
                }
                Ok(())
            }
            LootEntryKind::Item(_)
            | LootEntryKind::Tag { .. }
            | LootEntryKind::Dynamic(_)
            | LootEntryKind::Empty => Ok(()),
        }
    }

    fn conditions(&mut self, conditions: &[LootCondition], path: &str) -> Result<()> {
        for (index, condition) in conditions.iter().enumerate() {
            self.condition(condition, &format!("{path}[{index}]"))?;
        }
        Ok(())
    }

    fn condition(&mut self, condition: &LootCondition, path: &str) -> Result<()> {
        use LootCondition as C;
        use LootContextParam as P;
        match condition {
            C::AllOf(terms) | C::AnyOf(terms) => self.conditions(terms, &format!("{path}.terms")),
            C::Inverted(term) => self.condition(term, &format!("{path}.term")),
            C::RandomChance(chance) => self.number(chance, &format!("{path}.chance")),
            C::LocationCheck { predicate, .. } => {
                self.referenced(path, P::Origin)?;
                match predicate {
                    Some(predicate) => self.at(path, check_location_predicate(predicate)),
                    None => Ok(()),
                }
            }
            C::MatchTool(predicate) => {
                self.referenced(path, P::Tool)?;
                match predicate {
                    Some(predicate) => self.at(path, check_item_predicate(predicate)),
                    None => Ok(()),
                }
            }
            C::TableBonus { .. } => self.referenced(path, P::Tool),
            C::SurvivesExplosion => self.referenced(path, P::ExplosionRadius),
            C::BlockStateProperty { .. } => self.referenced(path, P::BlockState),
            C::EntityProperties { entity, predicate } => {
                self.referenced(path, *entity)?;
                match predicate {
                    Some(predicate) => self.at(path, check_entity_predicate(predicate)),
                    None => Ok(()),
                }
            }
            C::EnchantmentActiveCheck(_) => self.referenced(path, P::EnchantmentActive),
            C::KilledByPlayer => self.referenced(path, P::LastDamagePlayer),
            C::RandomChanceWithEnchantedBonus { .. } => self.referenced(path, P::AttackingEntity),
            C::DamageSourceProperties(predicate) => {
                self.referenced(path, P::Origin)?;
                self.referenced(path, P::DamageSource)?;
                for entity in predicate
                    .iter()
                    .flat_map(|predicate| [&predicate.direct_entity, &predicate.source_entity])
                    .flatten()
                {
                    self.at(path, check_entity_predicate(entity))?;
                }
                Ok(())
            }
            C::ValueCheck { value, range } => {
                self.number(value, &format!("{path}.value"))?;
                self.range(range, &format!("{path}.range"))
            }
            C::TimeCheck { value, .. } => self.range(value, &format!("{path}.value")),
            C::EntityScores { entity, scores } => {
                self.referenced(path, *entity)?;
                for (objective, range) in scores {
                    self.range(range, &format!("{path}.scores.{objective}"))?;
                }
                Ok(())
            }
            C::EnvironmentAttributeCheck { attribute, value } => {
                self.at(
                    path,
                    environment_attribute_expected(attribute, value).map(|_| ()),
                )?;
                if attribute.positional {
                    self.referenced(path, P::Origin)
                } else {
                    Ok(())
                }
            }
            C::Reference(_) | C::WeatherCheck { .. } => Ok(()),
        }
    }

    fn functions(&mut self, functions: &[LootFunction], path: &str) -> Result<()> {
        for (index, function) in functions.iter().enumerate() {
            self.function(function, &format!("{path}[{index}]"))?;
        }
        Ok(())
    }

    fn function(&mut self, function: &LootFunction, path: &str) -> Result<()> {
        use LootContextParam as P;
        use LootFunctionKind as K;
        self.conditions(&function.conditions, &format!("{path}.conditions"))?;
        let number = |checker: &mut Self, provider, name: &str| {
            checker.number(provider, &format!("{path}.{name}"))
        };
        match &function.kind {
            K::Sequence(functions) => self.functions(functions, &format!("{path}.functions")),
            K::SetCount { count, .. } => number(self, count, "count"),
            K::SetDamage { damage, .. } => number(self, damage, "damage"),
            K::LimitCount(limit) => self.range(limit, &format!("{path}.limit")),
            K::EnchantedCountIncrease { count, .. } => {
                self.referenced(path, P::AttackingEntity)?;
                number(self, count, "count")
            }
            K::ApplyBonus { .. } => self.referenced(path, P::Tool),
            K::CopyState { .. } => self.referenced(path, P::BlockState),
            K::CopyComponents { source, .. } => self.referenced(path, *source),
            K::CopyName(param) | K::FillPlayerHead(param) => self.referenced(path, *param),
            K::CopyCustomData(copy) => match copy.source {
                LootNbtProvider::Context(param) => self.referenced(path, param),
                LootNbtProvider::Storage(_) => Ok(()),
            },
            K::ModifyContents { modifier, .. } => {
                self.function(modifier, &format!("{path}.modifier"))
            }
            K::SetContents { entries, .. } => {
                for (index, entry) in entries.iter().enumerate() {
                    self.entry(entry, &format!("{path}.entries[{index}]"))?;
                }
                Ok(())
            }
            K::SetStewEffect(effects) => {
                for (index, (_, duration)) in effects.iter().enumerate() {
                    number(self, duration, &format!("effects[{index}].duration"))?;
                }
                Ok(())
            }
            K::SetOminousBottleAmplifier(amplifier) => number(self, amplifier, "amplifier"),
            K::SetRandomDyes(dyes) => number(self, dyes, "number_of_dyes"),
            K::SetEnchantments { enchantments, .. } => {
                for (id, level) in enchantments {
                    number(self, level, &format!("enchantments.{id}"))?;
                }
                Ok(())
            }
            K::SetAttributes { modifiers, .. } => {
                for (index, modifier) in modifiers.iter().enumerate() {
                    number(
                        self,
                        &modifier.amount,
                        &format!("modifiers[{index}].amount"),
                    )?;
                }
                Ok(())
            }
            K::SetName { entity, .. } | K::SetLore { entity, .. } => match entity {
                Some(entity) => self.referenced(path, *entity),
                None => Ok(()),
            },
            K::ExplorationMap(_) => self.referenced(path, P::Origin),
            K::EnchantRandomly {
                include_additional_cost_component,
                ..
            } => {
                if *include_additional_cost_component {
                    self.referenced(path, P::AdditionalCostComponentAllowed)?;
                }
                Ok(())
            }
            K::EnchantWithLevels {
                levels,
                include_additional_cost_component,
                ..
            } => {
                number(self, levels, "levels")?;
                if *include_additional_cost_component {
                    self.referenced(path, P::AdditionalCostComponentAllowed)?;
                }
                Ok(())
            }
            K::Filtered {
                item_filter,
                on_pass,
                on_fail,
            } => {
                self.at(path, check_item_predicate(item_filter))?;
                for (field, nested) in [("on_pass", on_pass), ("on_fail", on_fail)] {
                    if let Some(nested) = nested {
                        self.function(nested, &format!("{path}.{field}"))?;
                    }
                }
                Ok(())
            }
            K::SetCustomModelData { floats, colors, .. } => {
                for (field, values) in [("floats", floats), ("colors", colors)] {
                    for (index, value) in values.iter().flat_map(|(_, values)| values).enumerate() {
                        number(self, value, &format!("{field}.values[{index}]"))?;
                    }
                }
                Ok(())
            }
            K::Discard
            | K::ExplosionDecay
            | K::FurnaceSmelt { .. }
            | K::Reference(_)
            | K::SetBannerPattern { .. }
            | K::SetBookCover { .. }
            | K::SetBookPages { .. }
            | K::SetComponents(_)
            | K::SetCustomData(_)
            | K::SetFireworkExplosion(_)
            | K::SetFireworks { .. }
            | K::SetInstrument(_)
            | K::SetItem(_)
            | K::SetLootTable { .. }
            | K::SetPotion(_)
            | K::SetRandomPotion(_)
            | K::ToggleTooltips(_) => Ok(()),
        }
    }

    fn range(&mut self, range: &IntRange, path: &str) -> Result<()> {
        for (name, bound) in [("min", &range.min), ("max", &range.max)] {
            if let Some(bound) = bound {
                self.number(bound, &format!("{path}.{name}"))?;
            }
        }
        Ok(())
    }

    fn number(&mut self, provider: &NumberProvider, path: &str) -> Result<()> {
        match provider {
            NumberProvider::Uniform { min: a, max: b }
            | NumberProvider::Binomial { n: a, p: b } => {
                self.number(a, path)?;
                self.number(b, path)
            }
            NumberProvider::Sum(terms) => {
                for term in terms {
                    self.number(term, path)?;
                }
                Ok(())
            }
            NumberProvider::Constant(_)
            | NumberProvider::EnchantmentLevel(_)
            | NumberProvider::Storage { .. } => Ok(()),
            NumberProvider::Score { target, .. } => match target {
                ScoreTarget::Fixed(_) => Ok(()),
                ScoreTarget::Context(param) => self.referenced(path, *param),
            },
            NumberProvider::EnvironmentAttribute(attribute) => {
                if attribute.positional {
                    self.referenced(path, LootContextParam::Origin)
                } else {
                    Ok(())
                }
            }
        }
    }
}

struct Eval<'a> {
    table_id: &'a str,
    tables: &'a LootRegistries,
    /// The bound parameter bundle every read below goes through.
    context: LootContext<'a>,
    /// Tables, item modifiers and predicates being evaluated, for the loop
    /// guard `LootContext.pushVisitedElement` keeps.
    visited: Vec<String>,
    visited_functions: Vec<String>,
    visited_conditions: Vec<String>,
    random: &'a GameRandom,
}

/// The function lists one stack passes through on its way out, innermost
/// first.
struct Decoration<'a> {
    functions: &'a [LootFunction],
    path: &'a str,
    outer: Option<&'a Decoration<'a>>,
}

impl<'a> Eval<'a> {
    fn new(
        table_id: &'a str,
        tables: &'a LootRegistries,
        params: &LootParams<'a>,
        random: &'a GameRandom,
    ) -> Result<Self> {
        let context = params.create().map_err(|reason| LootRuntimeError {
            table_id: table_id.to_string(),
            path: "params".to_string(),
            reason,
        })?;
        Ok(Self {
            table_id,
            tables,
            context,
            visited: Vec::new(),
            visited_functions: Vec::new(),
            visited_conditions: Vec::new(),
            random,
        })
    }
}

impl Eval<'_> {
    fn err(&self, path: &str, reason: LootRuntimeReason) -> LootRuntimeError {
        LootRuntimeError {
            table_id: self.table_id.to_string(),
            path: path.to_string(),
            reason,
        }
    }

    fn at<T>(&self, path: &str, result: std::result::Result<T, LootRuntimeReason>) -> Result<T> {
        result.map_err(|reason| self.err(path, reason))
    }

    fn catalog(&self, path: &str, error: impl fmt::Display) -> LootRuntimeError {
        self.err(path, LootRuntimeReason::Catalog(error.to_string()))
    }

    fn entity_predicate_world(&self) -> EntityPredicateWorld<'_> {
        EntityPredicateWorld {
            origin: self.context.origin(),
            dimension: self.context.dimension(),
            level: self.context.level(),
            block_entity_data: self.context.block_entity_data(),
            biome: self.context.biome(),
            sky: self.context.sky(),
            light: self.context.light(),
            structures: self.context.structures(),
            item_environment: self.context.item_environment(),
            vehicle_slots: self.context.vehicle_slots(),
            bodies: self.context.entity_bodies(),
            item_resolver: self
                .context
                .item_environment()
                .map(|environment| environment.item_resolver()),
        }
    }

    fn get_random_items(&mut self, table: &LootTable) -> Result<Vec<LootStack>> {
        let mut out = Vec::new();
        let table_id = self.table_id.to_string();
        self.get_random_items_raw(Some(&table_id), table, "", None, &mut out)?;
        Ok(out)
    }

    /// `LootTable.getRandomItemsRaw`. A keyed table already being drawn draws
    /// nothing; an inline table has no key to guard.
    fn get_random_items_raw(
        &mut self,
        key: Option<&str>,
        table: &LootTable,
        prefix: &str,
        outer: Option<&Decoration<'_>>,
        out: &mut Vec<LootStack>,
    ) -> Result<()> {
        if let Some(key) = key {
            if self.visited.iter().any(|seen| seen == key) {
                return Ok(());
            }
            self.visited.push(key.to_string());
        }
        let mut result = Ok(());
        for (index, pool) in table.pools.iter().enumerate() {
            result = self.add_random_items(
                pool,
                &format!("{prefix}pools[{index}]"),
                &table.functions,
                outer,
                out,
            );
            if result.is_err() {
                break;
            }
        }
        if key.is_some() {
            self.visited.pop();
        }
        result
    }

    fn add_random_items(
        &mut self,
        pool: &LootPool,
        path: &str,
        table_functions: &[LootFunction],
        outer: Option<&Decoration<'_>>,
        out: &mut Vec<LootStack>,
    ) -> Result<()> {
        if !self.all_of(&pool.conditions, &format!("{path}.conditions"))? {
            return Ok(());
        }
        let rolls = self.number_int(&pool.rolls, &format!("{path}.rolls"))?;
        let bonus_rolls = self.number_float(&pool.bonus_rolls, &format!("{path}.bonus_rolls"))?;
        let count = rolls + mth_floor(bonus_rolls * self.context.luck());
        for _ in 0..count {
            self.add_random_item(pool, path, table_functions, outer, out)?;
        }
        Ok(())
    }

    fn add_random_item<'p>(
        &mut self,
        pool: &'p LootPool,
        path: &str,
        table_functions: &[LootFunction],
        outer: Option<&Decoration<'_>>,
        out: &mut Vec<LootStack>,
    ) -> Result<()> {
        let mut valid: Vec<(String, ValidEntry<'p>)> = Vec::new();
        let mut total_weight = 0_i32;
        for (index, entry) in pool.entries.iter().enumerate() {
            self.expand(
                entry,
                &format!("{path}.entries[{index}]"),
                &mut ExpansionOutput::Weighted {
                    valid: &mut valid,
                    total_weight: &mut total_weight,
                },
            )?;
        }
        if total_weight == 0 || valid.is_empty() {
            return Ok(());
        }
        let chosen = if valid.len() == 1 {
            0
        } else {
            let mut index = self.random.next_int(total_weight);
            let mut chosen = None;
            for (position, (_, entry)) in valid.iter().enumerate() {
                index -= self.entry_weight(entry.entry());
                if index < 0 {
                    chosen = Some(position);
                    break;
                }
            }
            match chosen {
                Some(position) => position,
                // Unreachable while the weights are the ones just summed.
                None => return Ok(()),
            }
        };
        let (entry_path, entry) = valid.swap_remove(chosen);
        self.create_item_stack(
            entry,
            entry_path,
            &pool.functions,
            table_functions,
            outer,
            out,
        )
    }

    /// `LootPoolEntryContainer.expand` for each entry type.
    fn expand<'p>(
        &mut self,
        entry: &'p LootEntry,
        path: &str,
        output: &mut ExpansionOutput<'_, 'p>,
    ) -> Result<bool> {
        if !self.all_of(&entry.conditions, &format!("{path}.conditions"))? {
            return Ok(false);
        }
        let children = |path: &str, index: usize| format!("{path}.children[{index}]");
        match &entry.kind {
            LootEntryKind::Alternatives(entries) => {
                for (index, child) in entries.iter().enumerate() {
                    if self.expand(child, &children(path, index), output)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            LootEntryKind::Group(entries) => {
                for (index, child) in entries.iter().enumerate() {
                    self.expand(child, &children(path, index), output)?;
                }
                Ok(true)
            }
            LootEntryKind::Sequence(entries) => {
                for (index, child) in entries.iter().enumerate() {
                    if !self.expand(child, &children(path, index), output)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            LootEntryKind::Tag { tag, expand: true } => {
                // The members are read before any weight is added, because a
                // tag the generation cannot resolve refuses the entry whatever
                // the draw would have been.
                let members = self.at(
                    path,
                    tag_members(
                        tag,
                        self.context
                            .item_environment()
                            .map(|environment| environment.item_resolver()),
                    ),
                )?;
                for item_id in members {
                    self.accept_expanded(ValidEntry::TagItem(entry, item_id), path, output)?;
                }
                Ok(true)
            }
            _ => {
                self.accept_expanded(ValidEntry::Entry(entry), path, output)?;
                Ok(true)
            }
        }
    }

    fn entry_weight(&self, entry: &LootEntry) -> i32 {
        mth_floor(entry.weight as f32 + entry.quality as f32 * self.context.luck()).max(0)
    }

    fn create_item_stack(
        &mut self,
        entry: ValidEntry<'_>,
        path: String,
        pool_functions: &[LootFunction],
        table_functions: &[LootFunction],
        outer: Option<&Decoration<'_>>,
        out: &mut Vec<LootStack>,
    ) -> Result<()> {
        let path = path.as_str();
        let entry_functions_path = format!("{path}.functions");
        let table_decoration = Decoration {
            functions: table_functions,
            path: "functions",
            outer,
        };
        let pool_decoration = Decoration {
            functions: pool_functions,
            path: "pool.functions",
            outer: Some(&table_decoration),
        };
        let decoration = Decoration {
            functions: &entry.entry().functions,
            path: &entry_functions_path,
            outer: Some(&pool_decoration),
        };
        let one = |item_id: i32| ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            ..ItemStackSummary::empty()
        };
        let entry = match entry {
            // An expanded tag member skips the entry's own functions.
            ValidEntry::TagItem(_, item_id) => {
                return self.emit(one(item_id), Some(&pool_decoration), path, out)
            }
            ValidEntry::Entry(entry) => entry,
        };
        match &entry.kind {
            LootEntryKind::Item(item_id) => self.emit(one(*item_id), Some(&decoration), path, out),
            LootEntryKind::Empty => Ok(()),
            LootEntryKind::Tag { tag, .. } => {
                for item_id in self.at(
                    path,
                    tag_members(
                        tag,
                        self.context
                            .item_environment()
                            .map(|environment| environment.item_resolver()),
                    ),
                )? {
                    self.emit(one(item_id), Some(&decoration), path, out)?;
                }
                Ok(())
            }
            LootEntryKind::LootTable(NestedLootTable::Key(reference)) => {
                let tables = self.tables;
                let Some(nested) = tables.table(reference) else {
                    return Ok(());
                };
                self.get_random_items_raw(
                    Some(reference),
                    nested,
                    &format!("{path}->{{{reference}}}."),
                    Some(&decoration),
                    out,
                )
            }
            LootEntryKind::LootTable(NestedLootTable::Inline(table)) => self.get_random_items_raw(
                None,
                table,
                &format!("{path}->inline."),
                Some(&decoration),
                out,
            ),
            LootEntryKind::Dynamic(name) => {
                let Some(stacks) = self
                    .context
                    .dynamic_drops()
                    .and_then(|block_entity| block_entity.dynamic_drop(name))
                else {
                    return Ok(());
                };
                let stacks = stacks
                    .map_err(|detail| self.err(path, unsupported_field("dynamic entry", detail)))?;
                for stack in stacks {
                    self.emit(stack, Some(&decoration), path, out)?;
                }
                Ok(())
            }
            LootEntryKind::Slots(source) => {
                for stack in self.slot_source_stacks(source, path)? {
                    if !stack.is_empty() {
                        self.emit(stack.into_summary(), Some(&decoration), path, out)?;
                    }
                }
                Ok(())
            }
            LootEntryKind::Alternatives(_)
            | LootEntryKind::Group(_)
            | LootEntryKind::Sequence(_) => unreachable!("composite entries only expand"),
        }
    }

    fn emit(
        &mut self,
        mut stack: LootStack,
        decoration: Option<&Decoration<'_>>,
        path: &str,
        out: &mut Vec<LootStack>,
    ) -> Result<()> {
        let mut node = decoration;
        while let Some(current) = node {
            for (index, function) in current.functions.iter().enumerate() {
                stack = self.apply_loot_function(
                    function,
                    stack,
                    &format!("{}[{index}]", current.path),
                )?;
            }
            node = current.outer;
        }
        self.split_stack(stack, path, out)
    }

    /// `LootTable.createStackSplitter`. `isItemEnabled` is constantly true
    /// here: no 26.1 item or block declares `requiredFeatures`.
    fn split_stack(&self, stack: LootStack, path: &str, out: &mut Vec<LootStack>) -> Result<()> {
        let limit = max_stack_size(&stack).map_err(|error| self.catalog(path, error))?;
        if stack_count(&stack) <= limit {
            out.push(stack);
            return Ok(());
        }
        let mut remaining = stack_count(&stack);
        while remaining > 0 {
            let taken = limit.min(remaining);
            let mut copy = stack.clone();
            copy.count = taken;
            remaining -= taken;
            out.push(copy);
        }
        Ok(())
    }

    fn apply_loot_function(
        &mut self,
        function: &LootFunction,
        mut stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        use LootFunctionKind as K;
        if let K::Sequence(functions) = &function.kind {
            for (index, function) in functions.iter().enumerate() {
                stack = self.apply_loot_function(
                    function,
                    stack,
                    &format!("{path}.functions[{index}]"),
                )?;
            }
            return Ok(stack);
        }
        if !self.all_of(&function.conditions, &format!("{path}.conditions"))? {
            return Ok(stack);
        }
        match &function.kind {
            K::Sequence(_) => unreachable!("handled above"),
            K::SetCustomData(tag) => self.apply_set_custom_data(tag, stack, path),
            K::SetDamage { damage, add } => self.apply_set_damage(damage, *add, stack, path),
            K::ToggleTooltips(toggles) => self.apply_native_tooltips(toggles, stack, path),
            K::SetCustomModelData {
                floats,
                flags,
                strings,
                colors,
            } => self.apply_native_model_data(floats, flags, strings, colors, stack, path),
            K::SetFireworks {
                flight_duration,
                explosions,
            } => self.apply_set_fireworks(*flight_duration, explosions.as_ref(), stack, path),
            K::SetFireworkExplosion(fields) => {
                self.apply_set_firework_explosion(fields, stack, path)
            }
            K::CopyName(source) => self.apply_copy_name(*source, stack, path),
            K::FillPlayerHead(entity) => self.apply_fill_player_head(*entity, stack, path),
            K::CopyCustomData(copy) => self.apply_native_nbt_copy(copy, stack, path),
            K::ModifyContents {
                component,
                modifier,
            } => self.apply_modify_contents(*component, modifier, stack, path),
            K::SetContents { component, entries } => {
                self.apply_set_contents(*component, entries, stack, path)
            }
            K::SetStewEffect(effects) => self.apply_native_stew(effects, stack, path),
            K::SetInstrument(options) => {
                self.apply_holder_component(INSTRUMENT_REGISTRY, Some(options), true, stack, path)
            }
            K::SetRandomPotion(options) => {
                self.apply_holder_component(POTION_REGISTRY, options.as_ref(), true, stack, path)
            }
            K::SetPotion(potion) => self.apply_holder_component(
                POTION_REGISTRY,
                Some(&bbb_server_datapack::loot::HolderSet::Ids(vec![
                    potion.clone()
                ])),
                false,
                stack,
                path,
            ),
            K::SetOminousBottleAmplifier(amplifier) => {
                self.apply_ominous_bottle_amplifier(amplifier, stack, path)
            }
            K::SetEnchantments { enchantments, add } => {
                self.apply_native_enchantments(enchantments, *add, stack, path)
            }
            K::SetAttributes { modifiers, replace } => {
                self.apply_native_attributes(modifiers, *replace, stack, path)
            }
            K::SetBannerPattern { patterns, append } => {
                self.apply_banner_pattern(patterns, *append, stack, path)
            }
            K::SetLootTable { name, seed } => self.apply_container_loot(name, *seed, stack, path),
            K::SetRandomDyes(dyes) => self.apply_random_dyes(dyes, stack, path),
            K::SetName {
                name,
                entity,
                component,
            } => self.apply_set_name(name.as_ref(), *entity, *component, stack, path),
            K::SetLore { lore, mode, entity } => {
                self.apply_set_lore(lore, *mode, *entity, stack, path)
            }
            K::SetBookCover { value, retain } => self.apply_book_cover(value, retain, stack, path),
            K::SetBookPages {
                component,
                value,
                mode,
            } => self.apply_book_pages(*component, value, *mode, stack, path),
            K::SetItem(item_id) => self.apply_set_item(*item_id, stack, path),
            K::Reference(name) => self.apply_function_reference(name, stack, path),
            K::ExplorationMap(options) => self.apply_exploration_map(options, stack, path),
            K::Discard => Ok(ItemStackSummary::empty()),
            K::EnchantRandomly {
                options,
                only_compatible,
                include_additional_cost_component,
            } => self.apply_enchant_randomly(
                options.as_ref(),
                *only_compatible,
                *include_additional_cost_component,
                stack,
                path,
            ),
            K::EnchantWithLevels {
                levels,
                options,
                include_additional_cost_component,
            } => self.apply_enchant_with_levels(
                levels,
                options.as_ref(),
                *include_additional_cost_component,
                stack,
                path,
            ),
            K::Filtered {
                item_filter,
                on_pass,
                on_fail,
            } => self.apply_filtered(item_filter, on_pass, on_fail, stack, path),
            K::FurnaceSmelt { use_input_count } => {
                self.apply_furnace_smelt(*use_input_count, stack, path)
            }
            K::SetComponents(components) => self.apply_set_components(components, stack, path),
            K::EnchantedCountIncrease {
                enchantment,
                count,
                limit,
            } => {
                let edited =
                    enchanted_count_increase(self, enchantment, count, *limit, stack, path)?;
                Ok(edited)
            }
            K::SetCount { count, add } => {
                let base = if *add { stack_count(&stack) } else { 0 };
                let count = base.wrapping_add(self.number_int(count, &format!("{path}.count"))?);
                stack.count = count;
                Ok(stack)
            }
            K::ExplosionDecay => {
                if let Some(radius) = self.context.explosion_radius() {
                    let probability = 1.0_f32 / radius;
                    let mut retained = 0;
                    for _ in 0..stack_count(&stack) {
                        if self.random.next_float() <= probability {
                            retained += 1;
                        }
                    }
                    stack.count = retained;
                }
                Ok(stack)
            }
            K::ApplyBonus {
                enchantment,
                formula,
            } => {
                let Some(tool) = self.context.tool() else {
                    return Ok(stack);
                };
                let count = stack_count(&stack);
                let level = tool.enchantment_level(enchantment);
                let updated = match *formula {
                    BonusFormula::OreDrops => {
                        if level > 0 {
                            let bonus = (self.random.next_int(level + 2) - 1).max(0);
                            count * (bonus + 1)
                        } else {
                            count
                        }
                    }
                    BonusFormula::UniformBonusCount { bonus_multiplier } => {
                        count + self.random.next_int(bonus_multiplier * level + 1)
                    }
                    BonusFormula::BinomialWithBonusCount {
                        extra_rounds,
                        probability,
                    } => {
                        let mut result = count;
                        for _ in 0..(level + extra_rounds) {
                            if self.random.next_float() < probability {
                                result += 1;
                            }
                        }
                        result
                    }
                };
                stack.count = updated;
                Ok(stack)
            }
            K::LimitCount(limit) => {
                let count = self.int_range_clamp(limit, stack_count(&stack), path)?;
                stack.count = count;
                Ok(stack)
            }
            K::CopyState { properties, .. } => {
                let Some(state) = self.context.block_state() else {
                    return Ok(stack);
                };
                let edited = copy_block_state(properties, stack, state);
                Ok(edited)
            }
            K::CopyComponents {
                source,
                include,
                exclude,
            } => self.copy_components(*source, include.as_deref(), exclude.as_deref(), stack, path),
        }
    }

    /// `IntRange.clamp`.
    fn int_range_clamp(&mut self, range: &IntRange, value: i32, path: &str) -> Result<i32> {
        let min = match &range.min {
            Some(min) => Some(self.number_int(min, &format!("{path}.limit.min"))?),
            None => None,
        };
        let max = match &range.max {
            Some(max) => Some(self.number_int(max, &format!("{path}.limit.max"))?),
            None => None,
        };
        Ok(match (min, max) {
            (None, None) => value,
            (None, Some(max)) => value.min(max),
            (Some(min), None) => value.max(min),
            (Some(min), Some(max)) => value.max(min).min(max),
        })
    }

    /// `IntRange.test`.
    fn int_range_test(&mut self, range: &IntRange, value: i32, path: &str) -> Result<bool> {
        if let Some(min) = &range.min {
            if value < self.number_int(min, &format!("{path}.min"))? {
                return Ok(false);
            }
        }
        if let Some(max) = &range.max {
            if value > self.number_int(max, &format!("{path}.max"))? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn all_of(&mut self, conditions: &[LootCondition], path: &str) -> Result<bool> {
        for (index, condition) in conditions.iter().enumerate() {
            if !self.condition(condition, &format!("{path}[{index}]"))? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn condition(&mut self, condition: &LootCondition, path: &str) -> Result<bool> {
        use LootCondition as C;
        match condition {
            C::AllOf(terms) => self.all_of(terms, &format!("{path}.terms")),
            C::AnyOf(terms) => {
                for (index, term) in terms.iter().enumerate() {
                    if self.condition(term, &format!("{path}.terms[{index}]"))? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            C::Inverted(term) => Ok(!self.condition(term, &format!("{path}.term"))?),
            C::KilledByPlayer => Ok(killed_by_player(&self.context)),
            C::EnchantmentActiveCheck(active) => {
                let current = self.context.enchantment_active().ok_or_else(|| {
                    self.err(
                        path,
                        LootRuntimeReason::ParameterNotProvided(
                            LootContextParam::EnchantmentActive,
                        ),
                    )
                })?;
                Ok(current == *active)
            }
            C::RandomChanceWithEnchantedBonus {
                unenchanted_chance,
                enchanted_chance,
                enchantment,
            } => random_chance_with_enchanted_bonus(
                self,
                *unenchanted_chance,
                enchanted_chance,
                enchantment,
                path,
            ),
            C::SurvivesExplosion => {
                let radius = self.context.explosion_radius();
                Ok(radius.is_none_or(|radius| self.random.next_float() <= 1.0_f32 / radius))
            }
            C::WeatherCheck {
                raining,
                thundering,
            } => {
                let weather = self.context.weather().ok_or_else(|| {
                    self.err(
                        path,
                        unsupported_field("weather_check", "no level weather was supplied"),
                    )
                })?;
                Ok(raining.is_none_or(|raining| raining == weather.raining)
                    && thundering.is_none_or(|thundering| thundering == weather.thundering))
            }
            C::BlockStateProperty { block, properties } => {
                let Some(state) = self.context.block_state() else {
                    return Ok(false);
                };
                if state.name != *block {
                    return Ok(false);
                }
                match properties {
                    None => Ok(true),
                    Some(properties) => self.at(path, matches_state_properties(properties, state)),
                }
            }
            C::MatchTool(predicate) => {
                let Some(tool) = self.context.tool() else {
                    return Ok(false);
                };
                match predicate {
                    None => Ok(true),
                    Some(predicate) => self.at(
                        path,
                        item_predicate_matches(
                            predicate,
                            tool,
                            self.context
                                .item_environment()
                                .map(|environment| environment.item_resolver())
                                .unwrap_or(&bbb_registry_codec::UnavailableHolderResolver),
                        ),
                    ),
                }
            }
            C::TableBonus {
                enchantment,
                chances,
            } => {
                // `tool != null ? getItemEnchantmentLevel(..) : 0`: an absent
                // `TOOL` is level 0 and still spends the draw below.
                let level = match self.context.tool() {
                    Some(tool) => tool.enchantment_level(enchantment),
                    None => 0,
                };
                let chance = chances[level.clamp(0, chances.len() as i32 - 1) as usize];
                Ok(self.random.next_float() < chance)
            }
            C::RandomChance(chance) => {
                let probability = self.number_float(chance, &format!("{path}.chance"))?;
                Ok(self.random.next_float() < probability)
            }
            C::DamageSourceProperties(predicate) => {
                damage_source::damage_source_properties(self, predicate.as_deref(), path)
            }
            C::EntityProperties { entity, predicate } => {
                let entity = self.context.entity(*entity);
                let Some(predicate) = predicate else {
                    return Ok(true);
                };
                if !entity.present() {
                    return Ok(false);
                }
                self.at(
                    path,
                    entity_predicate_matches(predicate, entity, self.entity_predicate_world()),
                )
            }
            C::LocationCheck { predicate, offset } => {
                let Some(origin) = self.context.origin() else {
                    return Ok(false);
                };
                match predicate {
                    None => Ok(true),
                    Some(predicate) => self.at(
                        path,
                        location_predicate_matches_in_world(
                            predicate,
                            [
                                origin.x + f64::from(offset[0]),
                                origin.y + f64::from(offset[1]),
                                origin.z + f64::from(offset[2]),
                            ],
                            self.context.dimension(),
                            self.context.level(),
                            self.context.block_entity_data(),
                            self.context.biome(),
                            self.context.sky(),
                            self.context.light(),
                            self.context.structures(),
                            self.context
                                .item_environment()
                                .map(|environment| environment.item_resolver()),
                            self.context.item_environment(),
                        ),
                    ),
                }
            }
            // `ConditionReference.test`: an unknown or recursive reference is false.
            C::Reference(name) => {
                let Some(referenced) = self.tables.predicate(name) else {
                    return Ok(false);
                };
                if self.visited_conditions.iter().any(|seen| seen == name) {
                    return Ok(false);
                }
                self.visited_conditions.push(name.clone());
                let result = self.condition(referenced, &format!("{path}->{{{name}}}"));
                self.visited_conditions.pop();
                result
            }
            C::ValueCheck { value, range } => {
                let value = self.number_int(value, &format!("{path}.value"))?;
                self.int_range_test(range, value, &format!("{path}.range"))
            }
            C::TimeCheck {
                clock,
                period,
                value,
            } => {
                let mut time = self.at(path, self.environment(path)?.clock_total_ticks(clock))?;
                if let Some(period) = period {
                    if *period == 0 {
                        return Err(self.catalog(path, "time check period is zero"));
                    }
                    time = time.wrapping_rem(*period);
                }
                self.int_range_test(value, time as i32, &format!("{path}.value"))
            }
            C::EntityScores { entity, scores } => self.entity_scores(*entity, scores, path),
            C::EnvironmentAttributeCheck { attribute, value } => {
                let expected = self.at(path, environment_attribute_expected(attribute, value))?;
                let actual = self.environment_attribute_value(attribute, path)?;
                self.at(path, environment_attribute_equal(expected, actual))
            }
        }
    }

    fn environment_attribute_value(
        &self,
        attribute: &'static VanillaEnvironmentAttribute,
        path: &str,
    ) -> Result<LootEnvironmentAttributeValue<'_>> {
        let origin = if attribute.positional {
            Some(self.context.origin().ok_or_else(|| {
                self.catalog(path, "environment attribute requires an actual origin")
            })?)
        } else {
            None
        };
        self.at(
            path,
            self.environment(path)?
                .environment_attribute(attribute, origin),
        )
    }

    fn storage_value(
        &self,
        storage: &str,
        nbt_path: &CommandNbtPath,
        path: &str,
    ) -> Result<Option<NbtSummaryValue>> {
        let stored = self.at(path, self.environment(path)?.nbt_storage(storage))?;
        let Ok(mut values) = nbt_path.get(&stored) else {
            return Ok(None);
        };
        Ok(if values.len() == 1 {
            values.pop()
        } else {
            None
        })
    }

    /// `NumberProvider.getInt`.
    fn number_int(&mut self, provider: &NumberProvider, path: &str) -> Result<i32> {
        match provider {
            NumberProvider::Storage {
                storage,
                path: nbt_path,
            } => Ok(self
                .storage_value(storage, nbt_path, path)?
                .as_ref()
                .and_then(bbb_protocol::nbt_summary::codec_int_value)
                .unwrap_or(0)),
            NumberProvider::Uniform { min, max } => {
                let min = self.number_int(min, &format!("{path}.min"))?;
                let max = self.number_int(max, &format!("{path}.max"))?;
                if min >= max {
                    return Ok(min);
                }
                Ok(self.random.next_int(max - min + 1) + min)
            }
            NumberProvider::Binomial { n, p } => {
                let n = self.number_int(n, &format!("{path}.n"))?;
                let p = self.number_float(p, &format!("{path}.p"))?;
                let mut result = 0;
                for _ in 0..n {
                    if self.random.next_float() < p {
                        result += 1;
                    }
                }
                Ok(result)
            }
            NumberProvider::Sum(_) => Ok(mth_floor(self.number_float(provider, path)?)),
            _ => Ok(java_round(self.number_float(provider, path)?)),
        }
    }

    /// `NumberProvider.getFloat`.
    fn number_float(&mut self, provider: &NumberProvider, path: &str) -> Result<f32> {
        match provider {
            NumberProvider::Constant(value) => Ok(*value),
            NumberProvider::Uniform { min, max } => {
                let min = self.number_float(min, &format!("{path}.min"))?;
                let max = self.number_float(max, &format!("{path}.max"))?;
                if min >= max {
                    return Ok(min);
                }
                Ok(self.random.next_float() * (max - min) + min)
            }
            NumberProvider::Binomial { .. } => Ok(self.number_int(provider, path)? as f32),
            NumberProvider::Sum(terms) => {
                let mut value = 0.0;
                for term in terms {
                    value += self.number_float(term, path)?;
                }
                Ok(value)
            }
            NumberProvider::EnchantmentLevel(amount) => {
                let level = self.context.enchantment_level().ok_or_else(|| {
                    self.err(
                        path,
                        LootRuntimeReason::ParameterNotProvided(LootContextParam::EnchantmentLevel),
                    )
                })?;
                Ok(amount.calculate(level))
            }
            NumberProvider::Storage {
                storage,
                path: nbt_path,
            } => Ok(self
                .storage_value(storage, nbt_path, path)?
                .as_ref()
                .and_then(bbb_protocol::nbt_summary::codec_float_value)
                .unwrap_or(0.0)),
            NumberProvider::Score {
                target,
                score,
                scale,
            } => self.score_value(target, score, *scale, path),
            NumberProvider::EnvironmentAttribute(attribute) => {
                match self.environment_attribute_value(attribute, path)? {
                    LootEnvironmentAttributeValue::Scalar(
                        VanillaEnvironmentScalarValue::FloatBits(bits),
                    ) => Ok(f32::from_bits(bits)),
                    LootEnvironmentAttributeValue::Integer(value) => Ok(value as f32),
                    _ => {
                        Err(self
                            .catalog(path, "numeric environment attribute returned a non-number"))
                    }
                }
            }
        }
    }
}

/// `CopyBlockState.run`: copies the listed properties the state has into the
/// stack's `block_state` component; a result equal to the item's prototype
/// leaves no patch entry.
fn copy_block_state(
    names: &[String],
    mut stack: ItemStackSummary,
    state: &VanillaBlockStateIdentity,
) -> ItemStackSummary {
    let mut properties = stack.component_patch.block_state_properties.clone();
    for name in names {
        if let Some(value) = state.properties.get(name) {
            properties.insert(name.as_str().into(), value.as_str().into());
        }
    }
    let prototype: Option<bbb_protocol::packets::data_components::BlockStateProperties> =
        stack.item_id.and_then(|item_id| {
            vanilla_item_default_block_state_properties(item_id).map(|pairs| {
                pairs
                    .iter()
                    .map(|(name, value)| ((*name).into(), (*value).into()))
                    .collect()
            })
        });
    let patch = &mut stack.component_patch;
    if prototype.as_ref() == Some(&properties) {
        patch
            .added_type_ids
            .retain(|type_id| *type_id != VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID);
        patch.block_state_properties.clear();
        return stack;
    }
    if !patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID)
    {
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID);
    }
    patch.block_state_properties = properties;
    stack
}

fn malformed(owner: &'static str, field: &'static str) -> LootRuntimeReason {
    LootRuntimeReason::MalformedField { owner, field }
}

fn unsupported_field(owner: &'static str, detail: impl Into<String>) -> LootRuntimeReason {
    LootRuntimeReason::UnsupportedField {
        owner,
        detail: detail.into(),
    }
}

#[derive(Clone, Copy)]
enum ValidEntry<'p> {
    Entry(&'p LootEntry),
    TagItem(&'p LootEntry, i32),
}

impl<'p> ValidEntry<'p> {
    fn entry(self) -> &'p LootEntry {
        match self {
            Self::Entry(entry) | Self::TagItem(entry, _) => entry,
        }
    }
}

fn tag_members(
    tag: &str,
    resolver: Option<&dyn bbb_registry_codec::PersistentItemStackHolderResolver>,
) -> std::result::Result<Vec<i32>, LootRuntimeReason> {
    let members = match resolver {
        Some(resolver) => resolver.tag_members("minecraft:item", tag),
        None => vanilla_persistent_registry_tag_members_26_1("minecraft:item", tag),
    };
    match members {
        PersistentRegistryTagMembers::Present(members) => Ok(members
            .iter()
            .filter_map(|member| vanilla_item_type_for_resource_id(member).map(|entry| entry.id))
            .collect()),
        // `getTagOrEmpty` is an empty holder set for a tag nothing declares.
        PersistentRegistryTagMembers::Absent => Ok(Vec::new()),
        PersistentRegistryTagMembers::Unavailable => Err(unsupported_field(
            "tag entry",
            format!("item tag #{tag} membership is unavailable"),
        )),
    }
}

fn stack_count(stack: &ItemStackSummary) -> i32 {
    match stack.item_id {
        Some(item_id) if item_id != VANILLA_ITEM_AIR_ID && stack.count > 0 => stack.count,
        _ => 0,
    }
}

/// `Mth.floor(float)`.
fn mth_floor(value: f32) -> i32 {
    value.floor() as i32
}

/// `Math.round(float)`: the closest int, ties toward positive infinity.
fn java_round(value: f32) -> i32 {
    if value.is_nan() {
        return 0;
    }
    let rounded = (f64::from(value) + 0.5).floor();
    if rounded >= f64::from(i32::MAX) {
        i32::MAX
    } else if rounded <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        rounded as i32
    }
}

mod attacker_terms;
mod component_copy;
mod container_fill;
mod damage_source;
mod entity_predicate;
mod evaluator;
mod exploration_map;
mod function_reference;
mod item_modifiers;
pub(crate) mod item_predicate;
mod location_predicate;
mod native_attributes;
mod native_books;
mod native_contents;
pub(crate) mod native_enchantments;
mod native_entity_functions;
mod native_environment;
mod native_fireworks;
mod native_holder_functions;
mod native_model_data;
mod native_nbt_copy;
mod native_simple_components;
mod native_stew;
mod native_text;
mod native_tooltips;
mod native_value_functions;
mod score;
mod slot_source;
mod state_properties;
use attacker_terms::{
    enchanted_count_increase, killed_by_player, random_chance_with_enchanted_bonus,
};
pub use bbb_server_datapack::loot::LootExplorationMap;
use bbb_server_datapack::loot::LootNbtProvider;
pub use container_fill::fill_container;
use entity_predicate::{check_entity_predicate, entity_predicate_matches, EntityPredicateWorld};
use evaluator::{
    apply_prepared_component_patch, catalog_reason, environment_attribute_equal,
    environment_attribute_expected,
};
pub(crate) use evaluator::{nbt_field, nbt_string};
pub use evaluator::{
    prepare_component_patch, LootEnvironmentAttributeValue, LootEvaluator, LootItemEnvironment,
    LootRandomEnchantment,
};
pub use item_predicate::matches_item_predicate;
use item_predicate::{check_item_predicate, item_predicate_matches};
pub use location_predicate::check_location_predicate;
#[cfg(test)]
pub use location_predicate::location_predicate_matches;
pub(crate) use location_predicate::location_predicate_matches_in_world;
pub use native_environment::RegistryItemEnvironment;
use native_holder_functions::{INSTRUMENT_REGISTRY, POTION_REGISTRY};
pub use state_properties::matches_state_properties;

pub mod params;

#[cfg(test)]
pub use params::LOOT_CONTEXT_PARAMS;
pub use params::{
    param_sets, LootBlockEntity, LootContext, LootContextParam, LootContextParamSet,
    LootDamageSource, LootDamageSourceFacts, LootEntities, LootOrigin, LootParams,
    LootTextResolver, LootWeather,
};

#[cfg(test)]
mod tests;
