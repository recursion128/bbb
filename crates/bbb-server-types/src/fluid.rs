//! Fluid values and generated block-state projections.

use crate::chunk_ticks::FluidTickType;
use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_id_by_name_and_properties,
    vanilla_block_state_support_facts,
};
use bbb_protocol::block_types::{
    vanilla_block_type, vanilla_block_type_for_resource_id, VanillaBucketPickupOverride,
    VanillaLiquidBlockContainerOverride, VANILLA_BLOCK_BAMBOO_SAPLING_ID, VANILLA_BLOCK_COBWEB_ID,
    VANILLA_BLOCK_LAVA_ID, VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::registry_data_types::VanillaFluidIdentity;
use bbb_protocol::PersistentRegistryTagMembers;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

/// `FluidState.AMOUNT_FULL` (`FluidState.java:27`).
pub const AMOUNT_FULL: i32 = 8;

/// One of the two flowing-fluid families Vanilla 26.1 registers
/// (`Fluids.java`).
///
/// A family *is* the `isSame` equivalence class: `WaterFluid.isSame`
/// (`WaterFluid.java`) answers true for `WATER` and `FLOWING_WATER`
/// and nothing else, and `LavaFluid.isSame` (`LavaFluid.java`) does
/// the same for lava. Every place `FlowingFluid` compares `this` against a
/// neighbour it goes through `isSame`, so carrying the family instead of the
/// four singletons loses nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FluidFamily {
    Water,
    Lava,
}

impl FluidFamily {
    /// `FlowingFluid.getSource` (`WaterFluid.java`, `LavaFluid.java`).
    pub const fn source_identity(self) -> VanillaFluidIdentity {
        match self {
            Self::Water => VanillaFluidIdentity::Water,
            Self::Lava => VanillaFluidIdentity::Lava,
        }
    }

    pub const fn to_facts_kind(self) -> bbb_facts::fluid::FluidKind {
        match self {
            Self::Water => bbb_facts::fluid::FluidKind::Water,
            Self::Lava => bbb_facts::fluid::FluidKind::Lava,
        }
    }

    /// `FlowingFluid.getFlowing` (`WaterFluid.java`, `LavaFluid.java`).
    pub const fn flowing_identity(self) -> VanillaFluidIdentity {
        match self {
            Self::Water => VanillaFluidIdentity::FlowingWater,
            Self::Lava => VanillaFluidIdentity::FlowingLava,
        }
    }

    /// The `LiquidBlock` this family's states live in
    /// (`WaterFluid.createLegacyBlock`, `WaterFluid.java`;
    /// `LavaFluid.java`).
    pub const fn liquid_block_type_id(self) -> i32 {
        match self {
            Self::Water => VANILLA_BLOCK_WATER_ID,
            Self::Lava => VANILLA_BLOCK_LAVA_ID,
        }
    }

    /// The family a built-in fluid singleton belongs to; `None` for
    /// `Fluids.EMPTY`.
    pub const fn of_identity(identity: VanillaFluidIdentity) -> Option<Self> {
        match identity {
            VanillaFluidIdentity::Water | VanillaFluidIdentity::FlowingWater => Some(Self::Water),
            VanillaFluidIdentity::Lava | VanillaFluidIdentity::FlowingLava => Some(Self::Lava),
            VanillaFluidIdentity::Empty => None,
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Water => 0,
            Self::Lava => 1,
        }
    }
}

/// A `FluidState` (`FluidState.java`).
///
/// Vanilla interns one instance per (fluid singleton, `LEVEL`, `FALLING`)
/// triple, and `FlowingFluid.tick` compares the new state against the old one
/// by reference (`FlowingFluid.java`). The derived `PartialEq` here is
/// exactly that comparison, because the enum has one value per interned
/// instance.
///
/// `FALLING` is on every flowing-fluid state, source included
/// (`FlowingFluid.createFluidStateDefinition`, `FlowingFluid.java`);
/// only the `Flowing` subclasses add `LEVEL` (`WaterFluid.java`,
/// `LavaFluid.java`), which is why the source variant has no amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidState {
    /// `Fluids.EMPTY.defaultFluidState()` (`EmptyFluid.java`).
    Empty,
    /// `FlowingFluid.getSource(falling)` (`FlowingFluid.java`);
    /// `getAmount` is the constant 8 (`WaterFluid.java`).
    Source { family: FluidFamily, falling: bool },
    /// `FlowingFluid.getFlowing(amount, falling)`
    /// (`FlowingFluid.java`). `amount` is `LEVEL_FLOWING`, which is
    /// `IntegerProperty.create("level", 1, 8)`
    /// (`BlockStateProperties.java:113`).
    Flowing {
        family: FluidFamily,
        amount: u8,
        falling: bool,
    },
}

impl FluidState {
    /// `FlowingFluid.getSource(boolean)` (`FlowingFluid.java`).
    pub const fn source(family: FluidFamily, falling: bool) -> Self {
        Self::Source { family, falling }
    }

    /// `FlowingFluid.getFlowing(int, boolean)` (`FlowingFluid.java`).
    ///
    /// # Panics
    ///
    /// Panics when `amount` is outside `LEVEL_FLOWING`'s `1..=8`
    /// (`BlockStateProperties.java:113`); vanilla's `setValue` throws in the
    /// same situation.
    pub fn flowing(family: FluidFamily, amount: i32, falling: bool) -> Self {
        assert!(
            (1..=AMOUNT_FULL).contains(&amount),
            "LEVEL_FLOWING is IntegerProperty.create(\"level\", 1, 8); got {amount}"
        );
        Self::Flowing {
            family,
            amount: amount as u8,
            falling,
        }
    }

    /// `Fluid.isEmpty` (`EmptyFluid.java`; false everywhere else).
    pub const fn is_empty(self) -> bool {
        matches!(self, Self::Empty)
    }

    /// `FluidState.isSource` (`FluidState.java`) resolved through the
    /// two `isSource` overrides (`WaterFluid.java`, `:155-158`).
    pub const fn is_source(self) -> bool {
        matches!(self, Self::Source { .. })
    }

    /// `FluidState.getAmount` (`FluidState.java`).
    pub const fn amount(self) -> i32 {
        match self {
            Self::Empty => 0,
            Self::Source { .. } => AMOUNT_FULL,
            Self::Flowing { amount, .. } => amount as i32,
        }
    }

    /// The `FALLING` property (`FlowingFluid.java`); `Fluids.EMPTY` has no
    /// such property and vanilla never reads it off an empty state.
    pub const fn falling(self) -> bool {
        match self {
            Self::Empty => false,
            Self::Source { falling, .. } | Self::Flowing { falling, .. } => falling,
        }
    }

    /// `FluidState.isFull` (`FluidState.java`).
    pub const fn is_full(self) -> bool {
        self.amount() == AMOUNT_FULL
    }

    /// The family, or `None` for `Fluids.EMPTY`.
    pub const fn family(self) -> Option<FluidFamily> {
        match self {
            Self::Empty => None,
            Self::Source { family, .. } | Self::Flowing { family, .. } => Some(family),
        }
    }

    /// `FlowingFluid.getOwnHeight` (`FlowingFluid.java`), with
    /// `EmptyFluid`'s constant zero (`EmptyFluid.java`).
    pub fn own_height(self) -> f32 {
        bbb_facts::fluid::fluid_own_height_f32(if self.is_empty() {
            0
        } else {
            self.amount() as u8
        })
    }

    pub fn to_facts_sample(self) -> Option<bbb_facts::fluid::FluidSample> {
        Some(bbb_facts::fluid::FluidSample {
            kind: self.family()?.to_facts_kind(),
            amount: self.amount() as u8,
            falling: self.falling(),
            is_source: self.is_source(),
        })
    }

    /// Which of the five `Fluids` singletons this state belongs to
    /// (`FluidState.getType`, `FluidState.java`).
    ///
    /// The distinction between the source and the flowing singleton of one
    /// family is observable twice: `ServerLevel.tickFluid` only ticks a
    /// position whose current fluid `is` the scheduled type
    /// (`ServerLevel.java`), and `SimpleWaterloggedBlock.canPlaceLiquid`
    /// accepts `Fluids.WATER` only (`SimpleWaterloggedBlock.java`).
    pub const fn identity(self) -> VanillaFluidIdentity {
        match self {
            Self::Empty => VanillaFluidIdentity::Empty,
            Self::Source { family, .. } => family.source_identity(),
            Self::Flowing { family, .. } => family.flowing_identity(),
        }
    }

    /// The scheduled-tick type this state's singleton keys, for
    /// [`crate::chunk_ticks::ChunkTicks`].
    pub const fn tick_type(self) -> FluidTickType {
        FluidTickType::of_fluid(self.identity())
    }

    /// `Fluid.isSame` folded onto the family (`WaterFluid.java`).
    pub fn is_same_family(self, family: FluidFamily) -> bool {
        self.family() == Some(family)
    }

    /// `FlowingFluid.getLegacyLevel` (`FlowingFluid.java`): the
    /// `LiquidBlock.LEVEL` value that carries this state on the block side.
    pub const fn legacy_level(self) -> i32 {
        if self.is_source() {
            0
        } else {
            let amount = if self.amount() < AMOUNT_FULL {
                self.amount()
            } else {
                AMOUNT_FULL
            };
            AMOUNT_FULL - amount + if self.falling() { 8 } else { 0 }
        }
    }

    /// `FluidState.createLegacyBlock` (`FluidState.java`) resolved
    /// through the two overrides (`WaterFluid.java`,
    /// `LavaFluid.java`) and `EmptyFluid`'s air
    /// (`EmptyFluid.java`).
    pub fn create_legacy_block(self) -> i32 {
        match self.family() {
            None => air_block_state_id(),
            Some(family) => liquid_block_state_id(family, self.legacy_level()),
        }
    }
}

pub fn air_block_state_id() -> i32 {
    static ID: OnceLock<i32> = OnceLock::new();
    *ID.get_or_init(|| {
        default_block_state_id("minecraft:air").expect("Vanilla 26.1 registers minecraft:air")
    })
}

/// The single global state id of a property-less block, resolved by name so
/// that no id is hand-written here.
fn default_block_state_id(resource_id: &str) -> Option<i32> {
    vanilla_block_state_id_by_name_and_properties(resource_id, &BTreeMap::new())
}

/// `LiquidBlock`'s state for one `LEVEL` value (`LiquidBlock.LEVEL` is
/// `IntegerProperty.create("level", 0, 15)`, `BlockStateProperties.java:116`).
///
/// The 32 ids are looked up once from the generated block-state registry
/// rather than written down.
fn liquid_block_state_id(family: FluidFamily, legacy_level: i32) -> i32 {
    static IDS: OnceLock<[[i32; 16]; 2]> = OnceLock::new();
    let table = IDS.get_or_init(|| {
        let mut table = [[0_i32; 16]; 2];
        for family in [FluidFamily::Water, FluidFamily::Lava] {
            let name = vanilla_block_type(family.liquid_block_type_id())
                .expect("generated block catalog holds the two LiquidBlock ids")
                .resource_id;
            for level in 0..16_i32 {
                let mut properties = BTreeMap::new();
                properties.insert("level".to_string(), level.to_string());
                table[family.index()][level as usize] =
                    vanilla_block_state_id_by_name_and_properties(name, &properties)
                        .expect("every LiquidBlock LEVEL value has a generated state");
            }
        }
        table
    });
    table[family.index()][usize::try_from(legacy_level).expect("legacy level is never negative")]
}

/// `BlockState.getFluidState()` (`BlockBehaviour.java`) for a built-in
/// 26.1 global state id.
///
/// The cached value comes from `Block.getFluidState(state)`
/// (`BlockBehaviour.java`), which exactly four shapes override:
///
/// * `LiquidBlock.getFluidState` (`LiquidBlock.java`): the
///   `stateCache` built in the constructor (`LiquidBlock.java`), indexed
///   by `min(LEVEL, 8)`.
/// * the waterlogged pattern, `state.getValue(WATERLOGGED) ?
///   Fluids.WATER.getSource(false) : super` — 30-odd identical overrides, e.g.
///   `SlabBlock.java`, `TrapDoorBlock.java`.
/// * the always-flooded plants, which return `Fluids.WATER.getSource(false)`
///   unconditionally: `KelpBlock.java`, `KelpPlantBlock.java`,
///   `SeagrassBlock.java`, `TallSeagrassBlock.java`.
/// * `BubbleColumnBlock.getFluidState` (`BubbleColumnBlock.java`), same
///   unconditional water source.
///
/// All four are reconstructed from generated facts: the block-state name and
/// property map, and `BlockTypeCatalogEntry`'s
/// `liquid_block_container_override` / `bucket_pickup_override` class tags.
pub fn block_state_fluid_state(block_state_id: i32) -> FluidState {
    static STATES: OnceLock<Vec<FluidState>> = OnceLock::new();
    let Ok(index) = usize::try_from(block_state_id) else {
        return FluidState::Empty;
    };
    STATES
        .get_or_init(build_fluid_state_index)
        .get(index)
        .copied()
        .unwrap_or(FluidState::Empty)
}

fn build_fluid_state_index() -> Vec<FluidState> {
    let mut index = vec![
        FluidState::Empty;
        bbb_protocol::block_state_types::VANILLA_BLOCK_STATE_COUNT as usize
    ];
    for family in [FluidFamily::Water, FluidFamily::Lava] {
        for level in 0..16_i32 {
            index[liquid_block_state_id(family, level) as usize] =
                liquid_block_cached_state(family, level);
        }
    }
    // The waterlogged pattern plus the five unconditionally flooded blocks.
    // Both are enumerated off the block registry, never off a name guess.
    for &block_type_id in unconditionally_flooded_block_type_ids() {
        insert_all_states_of_block(&mut index, block_type_id, water_source());
    }
    for block_type_id in 0.. {
        let Some(entry) = vanilla_block_type(block_type_id) else {
            break;
        };
        if entry.liquid_block_container_override.is_some() {
            insert_waterlogged_states(&mut index, entry.id);
        }
    }
    index
}

fn water_source() -> FluidState {
    FluidState::source(FluidFamily::Water, false)
}

/// `LiquidBlock.stateCache` (`LiquidBlock.java`) read at
/// `min(level, 8)` (`LiquidBlock.java`).
fn liquid_block_cached_state(family: FluidFamily, legacy_level: i32) -> FluidState {
    match legacy_level.min(8) {
        0 => FluidState::source(family, false),
        8 => FluidState::flowing(family, AMOUNT_FULL, true),
        level => FluidState::flowing(family, AMOUNT_FULL - level, false),
    }
}

/// The blocks whose `getFluidState` is an unconditional
/// `Fluids.WATER.getSource(false)`.
///
/// Kelp, kelp plant, seagrass and tall seagrass are named by
/// `BlockTypeCatalogEntry::liquid_block_container_override`; bubble column is
/// named by `bucket_pickup_override`. Both are generated class facts, so the
/// set is derived, not written down.
fn unconditionally_flooded_block_type_ids() -> &'static [i32] {
    static IDS: OnceLock<Vec<i32>> = OnceLock::new();
    IDS.get_or_init(|| {
        let mut ids = Vec::new();
        for block_type_id in 0.. {
            let Some(entry) = vanilla_block_type(block_type_id) else {
                break;
            };
            let flooded_plant = matches!(
                entry.liquid_block_container_override,
                Some(
                    VanillaLiquidBlockContainerOverride::KelpBlock
                        | VanillaLiquidBlockContainerOverride::KelpPlantBlock
                        | VanillaLiquidBlockContainerOverride::SeagrassBlock
                        | VanillaLiquidBlockContainerOverride::TallSeagrassBlock
                )
            );
            let bubble_column = matches!(
                entry.bucket_pickup_override,
                Some(VanillaBucketPickupOverride::BubbleColumnBlock)
            );
            if flooded_plant || bubble_column {
                ids.push(entry.id);
            }
        }
        ids
    })
}

fn insert_all_states_of_block(index: &mut [FluidState], block_type_id: i32, state: FluidState) {
    for &id in block_state_ids_of(block_type_id) {
        index[id as usize] = state;
    }
}

fn insert_waterlogged_states(index: &mut [FluidState], block_type_id: i32) {
    for &id in block_state_ids_of(block_type_id) {
        if bbb_protocol::block_state_types::vanilla_block_state_property(id, "waterlogged")
            == Some("true")
        {
            index[id as usize] = water_source();
        }
    }
}

/// Every global state id belonging to one block, from the generated registry.
fn block_state_ids_of(block_type_id: i32) -> &'static [i32] {
    static BY_BLOCK: OnceLock<HashMap<i32, Vec<i32>>> = OnceLock::new();
    static EMPTY: &[i32] = &[];
    BY_BLOCK
        .get_or_init(|| {
            let mut by_block: HashMap<i32, Vec<i32>> = HashMap::new();
            for id in 0.. {
                let Some(owner) = vanilla_block_state_block_type_id(id) else {
                    break;
                };
                by_block.entry(owner).or_default().push(id);
            }
            by_block
        })
        .get(&block_type_id)
        .map_or(EMPTY, Vec::as_slice)
}

/// `BlockState.isAir()`, the `Properties.air()` flag. Exactly three blocks
/// carry it (`Blocks.java`, `:5232`, `:5233`) and they are exactly the
/// `#minecraft:air` block tag (`air`, `cave_air`, `void_air`), each single-state,
/// so the 3 global state IDs are cached once for an O(1) comparison with zero hashing.
///
/// This is the crate's one `isAir`; `bbb_server_core::entity` and
/// `bbb_server_core::persistence` both answer through it.
#[inline]
pub fn block_state_is_air(block_state_id: i32) -> bool {
    static AIR_STATE_IDS: OnceLock<[i32; 3]> = OnceLock::new();
    let ids = AIR_STATE_IDS.get_or_init(|| {
        let mut states = Vec::with_capacity(3);
        for block_type_id in block_type_ids_in_tag("minecraft:air") {
            states.extend_from_slice(block_state_ids_of(block_type_id));
        }
        states.sort_unstable();
        states.dedup();
        states
            .try_into()
            .expect("#minecraft:air must resolve to exactly 3 air block states")
    });
    block_state_id == ids[0] || block_state_id == ids[1] || block_state_id == ids[2]
}

pub fn block_type_ids_in_tag(tag_id: &str) -> HashSet<i32> {
    declared_block_type_ids_in_tag(tag_id).unwrap_or_default()
}

/// The same members as [`block_type_ids_in_tag`], keeping the one distinction it
/// throws away: `None` is "the generated catalog declares no such block tag",
/// while `Some(empty)` is a tag vanilla declares with no members.
///
/// Both exist in 26.1 -- `#minecraft:incorrect_for_diamond_tool` and
/// `#minecraft:incorrect_for_netherite_tool` are declared and empty -- so a
/// caller that has to fail on an unresolvable tag cannot use emptiness as the
/// signal.
pub fn declared_block_type_ids_in_tag(tag_id: &str) -> Option<HashSet<i32>> {
    let PersistentRegistryTagMembers::Present(members) =
        vanilla_persistent_registry_tag_members_26_1("minecraft:block", tag_id)
    else {
        return None;
    };
    Some(
        members
            .iter()
            .filter_map(|member| vanilla_block_type_for_resource_id(member))
            .map(|entry| entry.id)
            .collect(),
    )
}

/// `BlockBehaviour.BlockStateBase.isSolid` (`BlockBehaviour.java`):
/// the cached `legacySolid` (`:482-502`).
pub fn block_state_is_solid(block_state_id: i32) -> bool {
    vanilla_block_state_support_facts(block_state_id).is_some_and(|facts| facts.legacy_solid())
}

/// `ContentRegistry`-aware variant of [`block_state_is_solid`].
pub fn block_state_is_solid_with_registry(
    block_state_id: i32,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> bool {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        return facts.legacy_solid();
    }
    registry
        .block_by_state_id(block_state_id)
        .is_some_and(|(spec, _)| spec.full_cube_collision)
}

/// `BlockBehaviour.BlockStateBase.blocksMotion` (`BlockBehaviour.java`):
/// solid, minus the two blocks vanilla excludes by identity.
pub fn block_state_blocks_motion(block_state_id: i32) -> bool {
    let Some(block_type_id) = vanilla_block_state_block_type_id(block_state_id) else {
        return false;
    };
    if block_type_id == VANILLA_BLOCK_COBWEB_ID || block_type_id == VANILLA_BLOCK_BAMBOO_SAPLING_ID
    {
        return false;
    }
    block_state_is_solid(block_state_id)
}

/// `ContentRegistry`-aware variant of [`block_state_blocks_motion`].
pub fn block_state_blocks_motion_with_registry(
    block_state_id: i32,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> bool {
    if let Some(block_type_id) = vanilla_block_state_block_type_id(block_state_id) {
        if block_type_id == VANILLA_BLOCK_COBWEB_ID
            || block_type_id == VANILLA_BLOCK_BAMBOO_SAPLING_ID
        {
            return false;
        }
        return block_state_is_solid(block_state_id);
    }
    registry
        .block_by_state_id(block_state_id)
        .is_some_and(|(spec, _)| spec.full_cube_collision)
}

#[cfg(test)]
mod tests;
