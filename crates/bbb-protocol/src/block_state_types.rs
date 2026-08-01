//! Fixed Vanilla 26.1 block-state registry wire identities.

use std::{collections::BTreeMap, sync::OnceLock};

use serde::Deserialize;

mod collision_shapes_26_1;
mod fluid_replaceability_26_1;
mod placement_defaults_26_1;
mod render_facts_26_1;
mod support_facts_26_1;

use collision_shapes_26_1::{
    VANILLA_BLOCK_COLLISION_BOX_PALETTE, VANILLA_BLOCK_COLLISION_SHAPE_RANGES,
    VANILLA_BLOCK_STATE_COLLISION_SHAPE_INDICES,
};
use fluid_replaceability_26_1::VANILLA_BLOCK_STATE_FLUID_REPLACEABLE_RANGES;
use placement_defaults_26_1::{
    VANILLA_BLOCK_DEFAULT_STATE_IDS, VANILLA_BLOCK_NOTE_INSTRUMENT_INDICES,
    VANILLA_BLOCK_NOTE_INSTRUMENT_PALETTE, VANILLA_ITEM_DEFAULT_BLOCK_STATE_PROPERTIES,
};
use render_facts_26_1::{
    VANILLA_BLOCK_STATE_FACE_OCCLUSION_SHAPE_INDICES, VANILLA_BLOCK_STATE_RENDER_BEHAVIOR_BITS,
    VANILLA_BLOCK_STATE_RENDER_BLOCK_IDS,
};
use support_facts_26_1::{
    VANILLA_BLOCK_STATE_CAN_OCCLUDE_BITS, VANILLA_BLOCK_STATE_LIGHT_DAMPENING,
    VANILLA_BLOCK_STATE_LIQUID_BITS, VANILLA_BLOCK_STATE_REDSTONE_CONDUCTOR_BITS,
    VANILLA_BLOCK_STATE_SHADE_BRIGHTNESS, VANILLA_BLOCK_STATE_SOLID_RENDER_BITS,
    VANILLA_BLOCK_STATE_SUFFOCATING_BITS, VANILLA_BLOCK_STATE_SUPPORT_FACT_INDICES,
    VANILLA_BLOCK_STATE_SUPPORT_FACT_PALETTE, VANILLA_BLOCK_STATE_VIEW_BLOCKING_BITS,
};

/// The checked-in Vanilla 26.1 global block-state registry report.
///
/// Protocol decode needs only the contiguous id range. World and pack
/// consumers reuse the same report for names, properties, and model coverage
/// instead of embedding a second registry snapshot.
pub const VANILLA_BLOCK_STATES_26_1_JSON: &str = include_str!("../data/block_states_26_1.json");

/// `Block.BLOCK_STATE_REGISTRY.size()` for Vanilla 26.1.
pub const VANILLA_BLOCK_STATE_COUNT: i32 = 29_873;

type VanillaBlockPropertyCatalog = BTreeMap<String, BTreeMap<String, Vec<String>>>;

static VANILLA_BLOCK_PROPERTIES: OnceLock<VanillaBlockPropertyCatalog> = OnceLock::new();

pub fn vanilla_block_state_id(id: i32) -> Option<i32> {
    (0..VANILLA_BLOCK_STATE_COUNT).contains(&id).then_some(id)
}

/// Returns the built-in block's Vanilla 26.1 default global state id.
pub fn vanilla_block_default_state_id(block_type_id: i32) -> Option<i32> {
    let block_type_id = usize::try_from(block_type_id).ok()?;
    VANILLA_BLOCK_DEFAULT_STATE_IDS.get(block_type_id).copied()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaBlockCollisionBox {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct VanillaBlockCollisionShape {
    boxes: &'static [[u64; 6]],
}

impl VanillaBlockCollisionShape {
    pub fn boxes(self) -> impl ExactSizeIterator<Item = VanillaBlockCollisionBox> {
        self.boxes.iter().map(|bits| VanillaBlockCollisionBox {
            min_x: f64::from_bits(bits[0]),
            min_y: f64::from_bits(bits[1]),
            min_z: f64::from_bits(bits[2]),
            max_x: f64::from_bits(bits[3]),
            max_y: f64::from_bits(bits[4]),
            max_z: f64::from_bits(bits[5]),
        })
    }
}

/// Returns the exact cached empty-context collision shape for a non-dynamic
/// built-in Vanilla 26.1 block state. Dynamic states return `None` because
/// their result requires a live level, position, block entity, or collision
/// context.
pub fn vanilla_block_state_static_collision_shape(id: i32) -> Option<VanillaBlockCollisionShape> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    let palette_index = *VANILLA_BLOCK_STATE_COLLISION_SHAPE_INDICES.get(id)?;
    if palette_index == u16::MAX {
        return None;
    }
    let (start, count) = *VANILLA_BLOCK_COLLISION_SHAPE_RANGES.get(usize::from(palette_index))?;
    let start = usize::try_from(start).ok()?;
    let end = start.checked_add(usize::from(count))?;
    Some(VanillaBlockCollisionShape {
        boxes: VANILLA_BLOCK_COLLISION_BOX_PALETTE.get(start..end)?,
    })
}

/// Renderer-visible Vanilla state which is independent from the baked model
/// and chunk render layer.
///
/// The checked-in facts come from the live 26.1 block registry. In particular,
/// block identity and class/tag relations are not reconstructed from resource
/// names, and face occlusion is not reconstructed from baked model cuboids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaBlockStateRenderFacts {
    block_type_id: u16,
    behavior_bits: u16,
    face_occlusion_shape_indices: [u8; 6],
}

impl VanillaBlockStateRenderFacts {
    pub const fn block_type_id(self) -> i32 {
        self.block_type_id as i32
    }

    /// Stable non-zero identity for `neighborState.is(this)` comparisons.
    pub const fn block_identity(self) -> u64 {
        self.block_type_id as u64 + 1
    }

    /// Vanilla `FluidRenderer`'s `HalfTransparentBlock || LeavesBlock`
    /// neighbor relation for the water-overlay sprite.
    pub const fn fluid_water_overlay_neighbor(self) -> bool {
        self.behavior_bits & 1 != 0
    }

    pub const fn leaves_block(self) -> bool {
        self.behavior_bits & (1 << 1) != 0
    }

    pub const fn same_block_culls_all_faces(self) -> bool {
        self.behavior_bits & (1 << 2) != 0
    }

    pub const fn same_block_culls_vertical_faces(self) -> bool {
        self.behavior_bits & (1 << 3) != 0
    }

    pub const fn iron_bars_block(self) -> bool {
        self.behavior_bits & (1 << 4) != 0
    }

    pub const fn liquid_block(self) -> bool {
        self.behavior_bits & (1 << 6) != 0
    }

    pub const fn north(self) -> bool {
        self.behavior_bits & (1 << 7) != 0
    }

    pub const fn south(self) -> bool {
        self.behavior_bits & (1 << 8) != 0
    }

    pub const fn west(self) -> bool {
        self.behavior_bits & (1 << 9) != 0
    }

    pub const fn east(self) -> bool {
        self.behavior_bits & (1 << 10) != 0
    }

    pub const fn connection_property_mask(self) -> u8 {
        ((self.behavior_bits >> 11) & 0x0f) as u8
    }

    pub const fn face_occlusion_shape_index(self, direction: VanillaBlockSupportDirection) -> u8 {
        self.face_occlusion_shape_indices[direction.index() as usize]
    }

    pub const fn face_occlusion_shape_indices(self) -> [u8; 6] {
        self.face_occlusion_shape_indices
    }
}

/// Returns exact Vanilla renderer mechanism facts for one built-in 26.1 state.
pub fn vanilla_block_state_render_facts(id: i32) -> Option<VanillaBlockStateRenderFacts> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    Some(VanillaBlockStateRenderFacts {
        block_type_id: *VANILLA_BLOCK_STATE_RENDER_BLOCK_IDS.get(id)?,
        behavior_bits: *VANILLA_BLOCK_STATE_RENDER_BEHAVIOR_BITS.get(id)?,
        face_occlusion_shape_indices: *VANILLA_BLOCK_STATE_FACE_OCCLUSION_SHAPE_INDICES.get(id)?,
    })
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

/// Runtime-generated support, collision, occlusion-face, replaceable,
/// can-occlude, solid-render, redstone-conductor, suffocating, AO-lighting,
/// and legacy liquid facts for one built-in Vanilla 26.1 block state.
///
/// Support and collision-face results are exact state-cache facts when
/// `dynamic_shape` is false. Dynamic shapes require a live world query instead
/// of using the report's `EmptyBlockGetter` result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaBlockStateSupportFacts {
    facts: u64,
    can_occlude: bool,
    solid_render: bool,
    redstone_conductor: bool,
    suffocating: bool,
    view_blocking: bool,
    light_dampening: u8,
    shade_brightness: u8,
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

    pub const fn can_occlude(self) -> bool {
        self.can_occlude
    }

    pub const fn redstone_conductor(self) -> bool {
        self.redstone_conductor
    }

    pub const fn suffocating(self) -> bool {
        self.suffocating
    }

    pub const fn view_blocking(self) -> bool {
        self.view_blocking
    }

    pub const fn light_dampening(self) -> u8 {
        self.light_dampening
    }

    /// Vanilla `BlockState.getShadeBrightness()` encoded on an exact 0..=255
    /// scalar. Vanilla 26.1's built-in states report values representable by
    /// this scale (`0.2` is 51 and `1.0` is 255).
    pub const fn shade_brightness(self) -> u8 {
        self.shade_brightness
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

    /// Whether `BlockState.getFaceOcclusionShape(direction)` is the full block
    /// face used by `Block.shouldRenderFace`'s first culling branch.
    pub const fn occlusion_face_full(self, direction: VanillaBlockSupportDirection) -> bool {
        self.facts & (1 << (32 + direction.index())) != 0
    }

    /// Vanilla's context-free `BlockState.canBeReplaced()` property.
    pub const fn replaceable(self) -> bool {
        self.facts & (1 << 38) != 0
    }
}

/// Returns Vanilla's runtime-generated block support facts for a built-in state.
pub fn vanilla_block_state_support_facts(id: i32) -> Option<VanillaBlockStateSupportFacts> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    let palette_index = usize::from(*VANILLA_BLOCK_STATE_SUPPORT_FACT_INDICES.get(id)?);
    Some(VanillaBlockStateSupportFacts {
        facts: *VANILLA_BLOCK_STATE_SUPPORT_FACT_PALETTE.get(palette_index)?,
        can_occlude: VANILLA_BLOCK_STATE_CAN_OCCLUDE_BITS
            .get(id / 64)
            .is_some_and(|word| word & (1_u64 << (id % 64)) != 0),
        solid_render: VANILLA_BLOCK_STATE_SOLID_RENDER_BITS
            .get(id / 64)
            .is_some_and(|word| word & (1_u64 << (id % 64)) != 0),
        redstone_conductor: VANILLA_BLOCK_STATE_REDSTONE_CONDUCTOR_BITS
            .get(id / 64)
            .is_some_and(|word| word & (1_u64 << (id % 64)) != 0),
        suffocating: VANILLA_BLOCK_STATE_SUFFOCATING_BITS
            .get(id / 64)
            .is_some_and(|word| word & (1_u64 << (id % 64)) != 0),
        view_blocking: VANILLA_BLOCK_STATE_VIEW_BLOCKING_BITS
            .get(id / 64)
            .is_some_and(|word| word & (1_u64 << (id % 64)) != 0),
        light_dampening: *VANILLA_BLOCK_STATE_LIGHT_DAMPENING.get(id)?,
        shade_brightness: *VANILLA_BLOCK_STATE_SHADE_BRIGHTNESS.get(id)?,
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
        .is_some_and(|properties| properties.contains_key(property))
}

/// Returns the built-in block property's Vanilla 26.1 value order.
///
/// The checked-in global-state report enumerates every property value in its
/// `StateDefinition` order. Repeated values from the cartesian state product
/// are collapsed while preserving their first occurrence.
pub fn vanilla_block_property_values(block: &str, property: &str) -> Option<&'static [String]> {
    vanilla_block_properties()
        .get(block)?
        .get(property)
        .map(Vec::as_slice)
}

fn vanilla_block_properties() -> &'static VanillaBlockPropertyCatalog {
    VANILLA_BLOCK_PROPERTIES.get_or_init(|| {
        let report: BlockStateReport = serde_json::from_str(VANILLA_BLOCK_STATES_26_1_JSON)
            .expect("embedded vanilla 26.1 block state registry is valid JSON");
        assert_eq!(
            report.version,
            crate::MC_VERSION,
            "embedded block state registry version must match protocol target"
        );
        let mut blocks = VanillaBlockPropertyCatalog::new();
        for (expected_id, state) in report.states.into_iter().enumerate() {
            assert_eq!(
                state.id, expected_id as i32,
                "embedded block state registry must be contiguous"
            );
            let properties = blocks.entry(state.name).or_default();
            for (property, value) in state.properties {
                let values = properties.entry(property).or_default();
                if !values.contains(&value) {
                    values.push(value);
                }
            }
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
    use std::{collections::BTreeSet, fs, path::Path};

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
    fn runtime_generated_static_collision_shapes_cover_every_non_dynamic_state() {
        assert_eq!(
            VANILLA_BLOCK_STATE_COLLISION_SHAPE_INDICES.len(),
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        assert_eq!(VANILLA_BLOCK_COLLISION_SHAPE_RANGES.len(), 318);
        assert_eq!(VANILLA_BLOCK_COLLISION_BOX_PALETTE.len(), 702);
        assert_eq!(
            VANILLA_BLOCK_STATE_COLLISION_SHAPE_INDICES
                .iter()
                .filter(|index| **index == u16::MAX)
                .count(),
            179
        );
        assert_eq!(
            VANILLA_BLOCK_COLLISION_SHAPE_RANGES
                .iter()
                .map(|(_, count)| *count)
                .max(),
            Some(15)
        );
        assert!(VANILLA_BLOCK_STATE_COLLISION_SHAPE_INDICES
            .iter()
            .all(|index| *index == u16::MAX
                || usize::from(*index) < VANILLA_BLOCK_COLLISION_SHAPE_RANGES.len()));
        assert!(VANILLA_BLOCK_COLLISION_SHAPE_RANGES
            .iter()
            .all(|(start, count)| {
                usize::try_from(*start)
                    .ok()
                    .and_then(|start| start.checked_add(usize::from(*count)))
                    .is_some_and(|end| end <= VANILLA_BLOCK_COLLISION_BOX_PALETTE.len())
            }));

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
        assert!(
            vanilla_block_state_static_collision_shape(state_id("minecraft:air"))
                .unwrap()
                .boxes()
                .next()
                .is_none()
        );
        assert_eq!(
            vanilla_block_state_static_collision_shape(state_id("minecraft:stone"))
                .unwrap()
                .boxes()
                .collect::<Vec<_>>(),
            vec![VanillaBlockCollisionBox {
                min_x: 0.0,
                min_y: 0.0,
                min_z: 0.0,
                max_x: 1.0,
                max_y: 1.0,
                max_z: 1.0,
            }]
        );

        let dynamic_names = report
            .states
            .iter()
            .filter(|state| {
                vanilla_block_state_support_facts(state.id)
                    .unwrap()
                    .dynamic_shape()
            })
            .map(|state| state.name.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            dynamic_names,
            BTreeSet::from([
                "minecraft:bamboo",
                "minecraft:black_shulker_box",
                "minecraft:blue_shulker_box",
                "minecraft:brown_shulker_box",
                "minecraft:cyan_shulker_box",
                "minecraft:gray_shulker_box",
                "minecraft:green_shulker_box",
                "minecraft:light_blue_shulker_box",
                "minecraft:light_gray_shulker_box",
                "minecraft:lime_shulker_box",
                "minecraft:magenta_shulker_box",
                "minecraft:moving_piston",
                "minecraft:orange_shulker_box",
                "minecraft:pink_shulker_box",
                "minecraft:pointed_dripstone",
                "minecraft:powder_snow",
                "minecraft:purple_shulker_box",
                "minecraft:red_shulker_box",
                "minecraft:scaffolding",
                "minecraft:shulker_box",
                "minecraft:white_shulker_box",
                "minecraft:yellow_shulker_box",
            ])
        );
        for state in &report.states {
            let static_shape = vanilla_block_state_static_collision_shape(state.id);
            assert_eq!(
                static_shape.is_none(),
                dynamic_names.contains(state.name.as_str()),
                "{} ({})",
                state.name,
                state.id
            );
        }
        assert!(vanilla_block_state_static_collision_shape(-1).is_none());
        assert!(vanilla_block_state_static_collision_shape(VANILLA_BLOCK_STATE_COUNT).is_none());
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
    fn render_facts_project_vanilla_state_classes_and_face_shapes() {
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
            VANILLA_BLOCK_STATE_RENDER_BLOCK_IDS.len(),
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        assert_eq!(
            VANILLA_BLOCK_STATE_RENDER_BEHAVIOR_BITS.len(),
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        assert_eq!(
            VANILLA_BLOCK_STATE_FACE_OCCLUSION_SHAPE_INDICES.len(),
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        assert!(VANILLA_BLOCK_STATE_FACE_OCCLUSION_SHAPE_INDICES
            .iter()
            .flatten()
            .all(|index| *index < 126));

        let air = vanilla_block_state_render_facts(state_id("minecraft:air")).unwrap();
        assert_eq!(air.face_occlusion_shape_indices(), [0; 6]);
        let stone = vanilla_block_state_render_facts(state_id("minecraft:stone")).unwrap();
        assert_eq!(stone.face_occlusion_shape_indices(), [1; 6]);

        let glass = vanilla_block_state_render_facts(state_id("minecraft:glass")).unwrap();
        assert!(glass.fluid_water_overlay_neighbor());
        assert!(glass.same_block_culls_all_faces());
        assert!(!glass.leaves_block());
        assert_eq!(glass.face_occlusion_shape_indices(), [0; 6]);
        assert_ne!(glass.block_identity(), stone.block_identity());
        let white_glass =
            vanilla_block_state_render_facts(state_id("minecraft:white_stained_glass")).unwrap();
        assert_ne!(glass.block_identity(), white_glass.block_identity());

        let leaves = vanilla_block_state_render_facts(state_id("minecraft:oak_leaves")).unwrap();
        assert!(leaves.fluid_water_overlay_neighbor());
        assert!(leaves.leaves_block());
        assert!(!leaves.same_block_culls_all_faces());
        let powder = vanilla_block_state_render_facts(state_id("minecraft:powder_snow")).unwrap();
        assert!(powder.same_block_culls_all_faces());
        let roots = vanilla_block_state_render_facts(state_id("minecraft:mangrove_roots")).unwrap();
        assert!(roots.same_block_culls_vertical_faces());

        let pane = vanilla_block_state_render_facts(state_id("minecraft:white_stained_glass_pane"))
            .unwrap();
        assert!(pane.iron_bars_block());
        let bars = vanilla_block_state_render_facts(state_id("minecraft:copper_bars")).unwrap();
        assert!(bars.iron_bars_block());
        assert_eq!(
            bars.block_type_id(),
            crate::block_types::VANILLA_BLOCK_COPPER_BARS_ID
        );
        let water = vanilla_block_state_render_facts(state_id("minecraft:water")).unwrap();
        assert!(water.liquid_block());

        assert!(vanilla_block_state_render_facts(-1).is_none());
        assert!(vanilla_block_state_render_facts(VANILLA_BLOCK_STATE_COUNT).is_none());
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
            VANILLA_BLOCK_STATE_CAN_OCCLUDE_BITS.len(),
            (VANILLA_BLOCK_STATE_COUNT as usize).div_ceil(64)
        );
        assert_eq!(
            VANILLA_BLOCK_STATE_SOLID_RENDER_BITS.len(),
            (VANILLA_BLOCK_STATE_COUNT as usize).div_ceil(64)
        );
        assert_eq!(
            VANILLA_BLOCK_STATE_REDSTONE_CONDUCTOR_BITS.len(),
            (VANILLA_BLOCK_STATE_COUNT as usize).div_ceil(64)
        );
        assert_eq!(
            VANILLA_BLOCK_STATE_SUFFOCATING_BITS.len(),
            (VANILLA_BLOCK_STATE_COUNT as usize).div_ceil(64)
        );
        assert_eq!(
            VANILLA_BLOCK_STATE_VIEW_BLOCKING_BITS.len(),
            (VANILLA_BLOCK_STATE_COUNT as usize).div_ceil(64)
        );
        assert_eq!(
            VANILLA_BLOCK_STATE_LIGHT_DAMPENING.len(),
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        assert_eq!(
            VANILLA_BLOCK_STATE_SHADE_BRIGHTNESS.len(),
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        assert_eq!(
            VANILLA_BLOCK_STATE_LIQUID_BITS.len(),
            (VANILLA_BLOCK_STATE_COUNT as usize).div_ceil(64)
        );

        let air = vanilla_block_state_support_facts(state_id("minecraft:air")).unwrap();
        assert!(!air.legacy_solid());
        assert!(!air.dynamic_shape());
        assert!(!air.can_occlude());
        assert!(!air.solid_render());
        assert!(!air.redstone_conductor());
        assert!(!air.suffocating());
        assert!(!air.view_blocking());
        assert_eq!(air.light_dampening(), 0);
        assert_eq!(air.shade_brightness(), 255);
        assert!(!air.liquid());
        assert!(!air.face_sturdy(
            VanillaBlockSupportDirection::Up,
            VanillaBlockSupportType::Full
        ));
        assert!(!air.collision_face_nonempty(VanillaBlockSupportDirection::Up));
        assert!(!air.collision_face_full(VanillaBlockSupportDirection::Up));
        assert!(!air.occlusion_face_full(VanillaBlockSupportDirection::Up));
        assert!(air.replaceable());

        let stone = vanilla_block_state_support_facts(state_id("minecraft:stone")).unwrap();
        assert!(stone.legacy_solid());
        assert!(stone.can_occlude());
        assert!(stone.solid_render());
        assert!(stone.redstone_conductor());
        assert!(stone.suffocating());
        assert!(stone.view_blocking());
        assert_eq!(stone.light_dampening(), 15);
        assert_eq!(stone.shade_brightness(), 51);
        assert!(!stone.liquid());
        assert!(stone.occlusion_face_full(VanillaBlockSupportDirection::Up));
        assert!(!stone.replaceable());
        assert!(
            vanilla_block_state_support_facts(state_id("minecraft:short_grass"))
                .unwrap()
                .replaceable()
        );
        assert!(
            !vanilla_block_state_support_facts(state_id("minecraft:glass"))
                .unwrap()
                .occlusion_face_full(VanillaBlockSupportDirection::Up)
        );
        assert!(
            !vanilla_block_state_support_facts(state_id("minecraft:end_gateway"))
                .unwrap()
                .occlusion_face_full(VanillaBlockSupportDirection::Up)
        );
        assert!(
            !vanilla_block_state_support_facts(state_id("minecraft:glass"))
                .unwrap()
                .can_occlude()
        );
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
        let glass = vanilla_block_state_support_facts(state_id("minecraft:glass")).unwrap();
        assert!(!glass.view_blocking());
        assert_eq!(glass.light_dampening(), 0);
        assert_eq!(glass.shade_brightness(), 255);
        let leaves = vanilla_block_state_support_facts(state_id("minecraft:oak_leaves")).unwrap();
        assert!(!leaves.view_blocking());
        assert_eq!(leaves.light_dampening(), 1);
        assert_eq!(leaves.shade_brightness(), 51);
        assert!(
            vanilla_block_state_support_facts(state_id("minecraft:soul_sand"))
                .unwrap()
                .redstone_conductor()
        );
        assert!(
            vanilla_block_state_support_facts(state_id("minecraft:farmland"))
                .unwrap()
                .suffocating()
        );
        assert!(
            !vanilla_block_state_support_facts(state_id("minecraft:mangrove_roots"))
                .unwrap()
                .suffocating()
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
            vanilla_block_property_values("minecraft:copper_golem_statue", "copper_golem_pose")
                .unwrap(),
            ["standing", "sitting", "running", "star"]
        );
        assert_eq!(
            vanilla_block_property_values(
                "minecraft:waxed_oxidized_copper_golem_statue",
                "copper_golem_pose"
            )
            .unwrap(),
            ["standing", "sitting", "running", "star"]
        );
        assert_eq!(
            vanilla_block_property_values("minecraft:stone", "axis"),
            None
        );
        assert_eq!(
            vanilla_block_property_values("minecraft:missing", "axis"),
            None
        );
        assert_eq!(
            vanilla_block_properties().len(),
            crate::block_types::VANILLA_BLOCK_TYPES.len()
        );
    }

    #[test]
    fn production_consumers_do_not_restore_copper_golem_pose_tables() {
        fn visit(directory: &Path, violations: &mut Vec<String>) {
            for entry in fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(&path, violations);
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) != Some("rs")
                    || path.to_string_lossy().contains("/tests/")
                    || path.file_name().and_then(|value| value.to_str()) == Some("tests.rs")
                {
                    continue;
                }
                let source = fs::read_to_string(&path).unwrap();
                let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
                let compact: String = production
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .collect();
                if compact.contains("VANILLA_COPPER_GOLEM_POSES")
                    || compact.contains("[\"standing\",\"sitting\",\"running\",\"star\"]")
                {
                    violations.push(path.display().to_string());
                }
            }
        }

        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let mut violations = Vec::new();
        visit(crates, &mut violations);
        assert!(
            violations.is_empty(),
            "parallel copper-golem pose tables: {violations:?}"
        );
    }
}
