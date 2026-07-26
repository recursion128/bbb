//! Fixed Vanilla 26.1 block-state registry wire identities.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use serde::Deserialize;

mod fluid_replaceability_26_1;
mod placement_defaults_26_1;
mod support_facts_26_1;

use fluid_replaceability_26_1::VANILLA_BLOCK_STATE_FLUID_REPLACEABLE_RANGES;
use placement_defaults_26_1::{
    VANILLA_BLOCK_DEFAULT_STATE_IDS, VANILLA_BLOCK_NOTE_INSTRUMENT_INDICES,
    VANILLA_BLOCK_NOTE_INSTRUMENT_PALETTE, VANILLA_ITEM_DEFAULT_BLOCK_STATE_PROPERTIES,
};
use support_facts_26_1::{
    VANILLA_BLOCK_STATE_LIQUID_BITS, VANILLA_BLOCK_STATE_REDSTONE_CONDUCTOR_BITS,
    VANILLA_BLOCK_STATE_SOLID_RENDER_BITS, VANILLA_BLOCK_STATE_SUPPORT_FACT_INDICES,
    VANILLA_BLOCK_STATE_SUPPORT_FACT_PALETTE,
};

/// The checked-in Vanilla 26.1 global block-state registry report.
///
/// Protocol decode needs only the contiguous id range. World and pack
/// consumers reuse the same report for names, properties, and model coverage
/// instead of embedding a second registry snapshot.
pub const VANILLA_BLOCK_STATES_26_1_JSON: &str = include_str!("../data/block_states_26_1.json");

/// `Block.BLOCK_STATE_REGISTRY.size()` for Vanilla 26.1.
pub const VANILLA_BLOCK_STATE_COUNT: i32 = 29_873;

static VANILLA_BLOCK_PROPERTIES: OnceLock<BTreeMap<String, BTreeSet<String>>> = OnceLock::new();

pub fn vanilla_block_state_id(id: i32) -> Option<i32> {
    (0..VANILLA_BLOCK_STATE_COUNT).contains(&id).then_some(id)
}

/// Returns the built-in block's Vanilla 26.1 default global state id.
pub fn vanilla_block_default_state_id(block_type_id: i32) -> Option<i32> {
    let block_type_id = usize::try_from(block_type_id).ok()?;
    VANILLA_BLOCK_DEFAULT_STATE_IDS.get(block_type_id).copied()
}

/// Returns the built-in block's Vanilla 26.1 note-block instrument.
pub fn vanilla_block_note_instrument(block_type_id: i32) -> Option<&'static str> {
    let block_type_id = usize::try_from(block_type_id).ok()?;
    let palette_index = usize::from(*VANILLA_BLOCK_NOTE_INSTRUMENT_INDICES.get(block_type_id)?);
    VANILLA_BLOCK_NOTE_INSTRUMENT_PALETTE
        .get(palette_index)
        .copied()
}

/// Returns an item's prototype `BLOCK_STATE` properties when Vanilla supplies
/// that component by default.
pub fn vanilla_item_default_block_state_properties(
    item_type_id: i32,
) -> Option<&'static [(&'static str, &'static str)]> {
    VANILLA_ITEM_DEFAULT_BLOCK_STATE_PROPERTIES
        .binary_search_by_key(&item_type_id, |(id, _)| *id)
        .ok()
        .map(|index| VANILLA_ITEM_DEFAULT_BLOCK_STATE_PROPERTIES[index].1)
}

/// Returns Vanilla's fluid replacement predicate for a built-in block state.
pub fn vanilla_block_state_fluid_replaceable(id: i32) -> Option<bool> {
    let id = vanilla_block_state_id(id)?;
    Some(
        VANILLA_BLOCK_STATE_FLUID_REPLACEABLE_RANGES
            .binary_search_by(|(start, end)| {
                if id < *start {
                    std::cmp::Ordering::Greater
                } else if id > *end {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .is_ok(),
    )
}

/// A direction used by Vanilla's cached per-face block-state support facts.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaBlockSupportDirection {
    Down = 0,
    Up = 1,
    North = 2,
    South = 3,
    West = 4,
    East = 5,
}

impl VanillaBlockSupportDirection {
    const fn index(self) -> u32 {
        self as u32
    }
}

/// One of Vanilla's three block-face support predicates.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaBlockSupportType {
    Full = 0,
    Center = 1,
    Rigid = 2,
}

impl VanillaBlockSupportType {
    const fn index(self) -> u32 {
        self as u32
    }
}

/// Runtime-generated support, collision, solid-render, redstone-conductor,
/// and legacy liquid facts for one built-in Vanilla 26.1 block state.
///
/// Support and collision-face results are exact state-cache facts when
/// `dynamic_shape` is false. Dynamic shapes require a live world query instead
/// of using the report's `EmptyBlockGetter` result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaBlockStateSupportFacts {
    facts: u32,
    solid_render: bool,
    redstone_conductor: bool,
    liquid: bool,
}

impl VanillaBlockStateSupportFacts {
    pub const fn legacy_solid(self) -> bool {
        self.facts & 1 != 0
    }

    pub const fn dynamic_shape(self) -> bool {
        self.facts & (1 << 1) != 0
    }

    pub const fn solid_render(self) -> bool {
        self.solid_render
    }

    pub const fn redstone_conductor(self) -> bool {
        self.redstone_conductor
    }

    pub const fn liquid(self) -> bool {
        self.liquid
    }

    pub const fn face_sturdy(
        self,
        direction: VanillaBlockSupportDirection,
        support_type: VanillaBlockSupportType,
    ) -> bool {
        let bit = 2 + direction.index() * 3 + support_type.index();
        self.facts & (1 << bit) != 0
    }

    pub const fn collision_face_nonempty(self, direction: VanillaBlockSupportDirection) -> bool {
        self.facts & (1 << (20 + direction.index())) != 0
    }

    pub const fn collision_face_full(self, direction: VanillaBlockSupportDirection) -> bool {
        self.facts & (1 << (26 + direction.index())) != 0
    }
}

/// Returns Vanilla's runtime-generated block support facts for a built-in state.
pub fn vanilla_block_state_support_facts(id: i32) -> Option<VanillaBlockStateSupportFacts> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    let palette_index = usize::from(*VANILLA_BLOCK_STATE_SUPPORT_FACT_INDICES.get(id)?);
    Some(VanillaBlockStateSupportFacts {
        facts: *VANILLA_BLOCK_STATE_SUPPORT_FACT_PALETTE.get(palette_index)?,
        solid_render: VANILLA_BLOCK_STATE_SOLID_RENDER_BITS
            .get(id / 64)
            .is_some_and(|word| word & (1_u64 << (id % 64)) != 0),
        redstone_conductor: VANILLA_BLOCK_STATE_REDSTONE_CONDUCTOR_BITS
            .get(id / 64)
            .is_some_and(|word| word & (1_u64 << (id % 64)) != 0),
        liquid: VANILLA_BLOCK_STATE_LIQUID_BITS
            .get(id / 64)
            .is_some_and(|word| word & (1_u64 << (id % 64)) != 0),
    })
}

/// Whether the built-in block's 26.1 `StateDefinition` contains `property`.
///
/// The checked-in global-state report is the protocol-owned source for both
/// wire ids and property names. Consumers must still resolve `block` through
/// the generated block-type catalog when they need to distinguish an unknown
/// block from a known block without properties.
pub fn vanilla_block_has_property(block: &str, property: &str) -> bool {
    vanilla_block_properties()
        .get(block)
        .is_some_and(|properties| properties.contains(property))
}

fn vanilla_block_properties() -> &'static BTreeMap<String, BTreeSet<String>> {
    VANILLA_BLOCK_PROPERTIES.get_or_init(|| {
        let report: BlockStateReport = serde_json::from_str(VANILLA_BLOCK_STATES_26_1_JSON)
            .expect("embedded vanilla 26.1 block state registry is valid JSON");
        assert_eq!(
            report.version,
            crate::MC_VERSION,
            "embedded block state registry version must match protocol target"
        );
        let mut blocks = BTreeMap::<String, BTreeSet<String>>::new();
        for (expected_id, state) in report.states.into_iter().enumerate() {
            assert_eq!(
                state.id, expected_id as i32,
                "embedded block state registry must be contiguous"
            );
            let properties = blocks.entry(state.name).or_default();
            properties.extend(state.properties.into_keys());
        }
        blocks
    })
}

#[derive(Debug, Deserialize)]
struct BlockStateReport {
    version: String,
    states: Vec<BlockStateEntry>,
}

#[derive(Debug, Deserialize)]
struct BlockStateEntry {
    id: i32,
    name: String,
    #[serde(default)]
    properties: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_state_registry_report_is_contiguous_and_matches_wire_range() {
        let report: BlockStateReport =
            serde_json::from_str(VANILLA_BLOCK_STATES_26_1_JSON).unwrap();
        assert_eq!(report.version, crate::MC_VERSION);
        assert_eq!(report.states.len(), VANILLA_BLOCK_STATE_COUNT as usize);
        for (id, state) in report.states.iter().enumerate() {
            assert_eq!(state.id, id as i32);
        }
        assert_eq!(vanilla_block_state_id(-1), None);
        assert_eq!(vanilla_block_state_id(0), Some(0));
        assert_eq!(
            vanilla_block_state_id(VANILLA_BLOCK_STATE_COUNT - 1),
            Some(VANILLA_BLOCK_STATE_COUNT - 1)
        );
        assert_eq!(vanilla_block_state_id(VANILLA_BLOCK_STATE_COUNT), None);
    }

    #[test]
    fn runtime_generated_default_states_and_item_components_match_vanilla() {
        assert_eq!(
            VANILLA_BLOCK_DEFAULT_STATE_IDS.len(),
            crate::block_types::VANILLA_BLOCK_TYPES.len()
        );
        assert_eq!(
            VANILLA_BLOCK_NOTE_INSTRUMENT_INDICES.len(),
            crate::block_types::VANILLA_BLOCK_TYPES.len()
        );
        assert_eq!(VANILLA_BLOCK_NOTE_INSTRUMENT_PALETTE.len(), 27);
        assert!(VANILLA_BLOCK_NOTE_INSTRUMENT_INDICES
            .iter()
            .all(|index| usize::from(*index) < VANILLA_BLOCK_NOTE_INSTRUMENT_PALETTE.len()));
        assert_eq!(VANILLA_ITEM_DEFAULT_BLOCK_STATE_PROPERTIES.len(), 12);

        let report: BlockStateReport =
            serde_json::from_str(VANILLA_BLOCK_STATES_26_1_JSON).unwrap();
        let default_state = |block_type_id| {
            &report.states[vanilla_block_default_state_id(block_type_id).unwrap() as usize]
        };

        let grass = default_state(crate::block_types::VANILLA_BLOCK_GRASS_BLOCK_ID);
        assert_eq!(grass.name, "minecraft:grass_block");
        assert_eq!(grass.properties.get("snowy").unwrap(), "false");

        let oak_log = default_state(crate::block_types::VANILLA_BLOCK_OAK_LOG_ID);
        assert_eq!(oak_log.name, "minecraft:oak_log");
        assert_eq!(oak_log.properties.get("axis").unwrap(), "y");

        assert_eq!(
            vanilla_item_default_block_state_properties(crate::item_types::VANILLA_ITEM_LIGHT_ID),
            Some(&[("level", "15")][..])
        );
        assert_eq!(
            vanilla_item_default_block_state_properties(
                crate::item_types::VANILLA_ITEM_TEST_BLOCK_ID
            ),
            Some(&[("mode", "start")][..])
        );
        assert_eq!(
            vanilla_item_default_block_state_properties(crate::item_types::VANILLA_ITEM_BEEHIVE_ID),
            Some(&[("honey_level", "0")][..])
        );
        assert_eq!(
            vanilla_item_default_block_state_properties(
                crate::item_types::VANILLA_ITEM_COPPER_GOLEM_STATUE_ID
            ),
            Some(&[("copper_golem_pose", "standing")][..])
        );
        assert_eq!(
            vanilla_item_default_block_state_properties(crate::item_types::VANILLA_ITEM_STONE_ID),
            None
        );
        assert_eq!(
            vanilla_block_note_instrument(crate::block_types::VANILLA_BLOCK_STONE_ID),
            Some("basedrum")
        );
        assert_eq!(
            vanilla_block_note_instrument(crate::block_types::VANILLA_BLOCK_OAK_PLANKS_ID),
            Some("bass")
        );
        assert_eq!(
            vanilla_block_note_instrument(crate::block_types::VANILLA_BLOCK_SKELETON_SKULL_ID),
            Some("skeleton")
        );
        assert_eq!(vanilla_block_default_state_id(-1), None);
        assert_eq!(vanilla_block_note_instrument(-1), None);
        assert_eq!(
            vanilla_block_note_instrument(crate::block_types::VANILLA_BLOCK_TYPES.len() as i32),
            None
        );
        assert_eq!(
            vanilla_block_default_state_id(crate::block_types::VANILLA_BLOCK_TYPES.len() as i32),
            None
        );
    }

    #[test]
    fn fluid_replaceability_uses_the_runtime_generated_state_ranges() {
        let report: BlockStateReport =
            serde_json::from_str(VANILLA_BLOCK_STATES_26_1_JSON).unwrap();
        let state_id = |name: &str| {
            report
                .states
                .iter()
                .find(|state| state.name == name)
                .unwrap()
                .id
        };

        assert!(VANILLA_BLOCK_STATE_FLUID_REPLACEABLE_RANGES
            .windows(2)
            .all(|ranges| ranges[0].1 + 1 < ranges[1].0));
        assert_eq!(
            vanilla_block_state_fluid_replaceable(state_id("minecraft:air")),
            Some(true)
        );
        assert_eq!(
            vanilla_block_state_fluid_replaceable(state_id("minecraft:stone")),
            Some(false)
        );
        assert_eq!(
            vanilla_block_state_fluid_replaceable(state_id("minecraft:short_grass")),
            Some(true)
        );
        assert_eq!(
            vanilla_block_state_fluid_replaceable(state_id("minecraft:oak_sign")),
            Some(false)
        );
        assert_eq!(
            vanilla_block_state_fluid_replaceable(state_id("minecraft:end_portal")),
            Some(false)
        );
        assert_eq!(vanilla_block_state_fluid_replaceable(-1), None);
        assert_eq!(
            vanilla_block_state_fluid_replaceable(VANILLA_BLOCK_STATE_COUNT),
            None
        );
    }

    #[test]
    fn support_facts_use_the_runtime_generated_state_palette() {
        let report: BlockStateReport =
            serde_json::from_str(VANILLA_BLOCK_STATES_26_1_JSON).unwrap();
        let state_id = |name: &str| {
            report
                .states
                .iter()
                .find(|state| state.name == name)
                .unwrap()
                .id
        };

        assert_eq!(
            VANILLA_BLOCK_STATE_SUPPORT_FACT_INDICES.len(),
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        assert!(VANILLA_BLOCK_STATE_SUPPORT_FACT_INDICES
            .iter()
            .all(|index| usize::from(*index) < VANILLA_BLOCK_STATE_SUPPORT_FACT_PALETTE.len()));
        assert_eq!(
            VANILLA_BLOCK_STATE_SOLID_RENDER_BITS.len(),
            (VANILLA_BLOCK_STATE_COUNT as usize).div_ceil(64)
        );
        assert_eq!(
            VANILLA_BLOCK_STATE_REDSTONE_CONDUCTOR_BITS.len(),
            (VANILLA_BLOCK_STATE_COUNT as usize).div_ceil(64)
        );
        assert_eq!(
            VANILLA_BLOCK_STATE_LIQUID_BITS.len(),
            (VANILLA_BLOCK_STATE_COUNT as usize).div_ceil(64)
        );

        let air = vanilla_block_state_support_facts(state_id("minecraft:air")).unwrap();
        assert!(!air.legacy_solid());
        assert!(!air.dynamic_shape());
        assert!(!air.solid_render());
        assert!(!air.redstone_conductor());
        assert!(!air.liquid());
        assert!(!air.face_sturdy(
            VanillaBlockSupportDirection::Up,
            VanillaBlockSupportType::Full
        ));
        assert!(!air.collision_face_nonempty(VanillaBlockSupportDirection::Up));
        assert!(!air.collision_face_full(VanillaBlockSupportDirection::Up));

        let stone = vanilla_block_state_support_facts(state_id("minecraft:stone")).unwrap();
        assert!(stone.legacy_solid());
        assert!(stone.solid_render());
        assert!(stone.redstone_conductor());
        assert!(!stone.liquid());
        assert!(
            !vanilla_block_state_support_facts(state_id("minecraft:glass"))
                .unwrap()
                .solid_render()
        );
        assert!(
            !vanilla_block_state_support_facts(state_id("minecraft:glass"))
                .unwrap()
                .redstone_conductor()
        );
        assert!(
            vanilla_block_state_support_facts(state_id("minecraft:soul_sand"))
                .unwrap()
                .redstone_conductor()
        );
        assert!(
            vanilla_block_state_support_facts(state_id("minecraft:water"))
                .unwrap()
                .liquid()
        );
        assert!(
            vanilla_block_state_support_facts(state_id("minecraft:bubble_column"))
                .unwrap()
                .liquid()
        );
        for direction in [
            VanillaBlockSupportDirection::Down,
            VanillaBlockSupportDirection::Up,
            VanillaBlockSupportDirection::North,
            VanillaBlockSupportDirection::South,
            VanillaBlockSupportDirection::West,
            VanillaBlockSupportDirection::East,
        ] {
            assert!(stone.face_sturdy(direction, VanillaBlockSupportType::Full));
            assert!(stone.face_sturdy(direction, VanillaBlockSupportType::Center));
            assert!(stone.face_sturdy(direction, VanillaBlockSupportType::Rigid));
            assert!(stone.collision_face_nonempty(direction));
            assert!(stone.collision_face_full(direction));
        }

        assert!(
            vanilla_block_state_support_facts(state_id("minecraft:moving_piston"))
                .unwrap()
                .dynamic_shape()
        );

        assert_eq!(vanilla_block_state_support_facts(-1), None);
        assert_eq!(
            vanilla_block_state_support_facts(VANILLA_BLOCK_STATE_COUNT),
            None
        );
    }

    #[test]
    fn block_property_catalog_is_derived_from_the_embedded_state_report() {
        assert!(vanilla_block_has_property("minecraft:oak_log", "axis"));
        assert!(vanilla_block_has_property("minecraft:lever", "powered"));
        assert!(!vanilla_block_has_property("minecraft:stone", "axis"));
        assert!(!vanilla_block_has_property("minecraft:missing", "axis"));
        assert_eq!(
            vanilla_block_properties().len(),
            crate::block_types::VANILLA_BLOCK_TYPES.len()
        );
    }
}
