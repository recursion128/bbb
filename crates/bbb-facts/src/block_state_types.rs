//! Fixed Vanilla 26.1 block-state registry wire identities.

use std::{collections::BTreeMap, sync::OnceLock};

use serde::{Deserialize, Serialize};

mod collision_shapes_26_1;
mod destroy_facts_26_1;
mod fluid_replaceability_26_1;
#[cfg(feature = "server-facts")]
mod ignited_by_lava_26_1;
#[cfg(feature = "server-facts")]
mod mirrors_26_1;
mod outline_shapes_26_1;
#[cfg(feature = "server-facts")]
mod path_facts_26_1;
mod placement_defaults_26_1;
mod property_facts_26_1;
mod property_strides;
mod property_values;
#[cfg(feature = "server-facts")]
mod random_tick_facts_26_1;
#[cfg_attr(
    not(all(feature = "client-facts", feature = "server-facts")),
    allow(dead_code)
)]
mod render_facts_26_1;
#[cfg(feature = "server-facts")]
mod rotations_26_1;
#[cfg_attr(not(feature = "server-facts"), allow(dead_code))]
mod support_facts_26_1;

pub use property_strides::{
    vanilla_block_state_cycle_property, vanilla_block_state_properties,
    vanilla_block_state_property, vanilla_block_state_property_matches_utf16,
    vanilla_block_state_with_property,
};
pub use property_values::{
    vanilla_block_property_value_rank, vanilla_block_property_value_rank_utf16,
};

use fluid_replaceability_26_1::VANILLA_BLOCK_STATE_FLUID_REPLACEABLE_RANGES;
#[cfg(feature = "server-facts")]
use ignited_by_lava_26_1::VANILLA_BLOCK_STATE_IGNITED_BY_LAVA_RANGES;
use placement_defaults_26_1::{
    VANILLA_BLOCK_DEFAULT_STATE_IDS, VANILLA_BLOCK_NOTE_INSTRUMENT_INDICES,
    VANILLA_BLOCK_NOTE_INSTRUMENT_PALETTE, VANILLA_ITEM_DEFAULT_BLOCK_STATE_PROPERTIES,
};
#[cfg(feature = "server-facts")]
use random_tick_facts_26_1::{
    VANILLA_BLOCK_STATE_FLUID_RANDOMLY_TICKING_RANGES, VANILLA_BLOCK_STATE_RANDOMLY_TICKING_RANGES,
};
use render_facts_26_1::VANILLA_FACE_OCCLUSION_MASKS;

/// `Block.BLOCK_STATE_REGISTRY.size()` for Vanilla 26.1.
pub const VANILLA_BLOCK_STATE_COUNT: i32 = 29_873;

/// Offset limits extracted from `BlockState.getOffset(BlockPos.ZERO)`.
#[cfg(feature = "client-facts")]
pub fn vanilla_block_state_model_offset_limits(id: i32) -> Option<[f32; 2]> {
    let id = u16::try_from(vanilla_block_state_id(id)?).ok()?;
    render_facts_26_1::model_offset_limits(id)
}

/// `BlockBehaviour.Properties.offsetType`: XZ/XYZ displacement, independent of Y.
pub fn vanilla_block_model_offset(x: i32, z: i32, limits: [f32; 2]) -> [f64; 3] {
    let seed = i64::from(x.wrapping_mul(3_129_871)) ^ i64::from(z).wrapping_mul(116_129_781);
    let seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11))
        >> 16;
    let horizontal = |bits: i64| {
        ((f64::from(bits as f32 / 15.0) - 0.5) * 0.5)
            .clamp(-f64::from(limits[0]), f64::from(limits[0]))
    };
    [
        horizontal(seed & 15),
        (f64::from(((seed >> 4) & 15) as f32 / 15.0) - 1.0) * f64::from(limits[1]),
        horizontal((seed >> 8) & 15),
    ]
}

pub fn vanilla_block_state_id(id: i32) -> Option<i32> {
    (0..VANILLA_BLOCK_STATE_COUNT).contains(&id).then_some(id)
}

/// Applies Vanilla's exact horizontal `BlockState.rotate` behavior.
///
/// `clockwise_quarter_turns` is reduced modulo four. The table is evaluated
/// from every concrete 26.1 block owner rather than inferred from properties.
#[cfg(feature = "server-facts")]
pub fn vanilla_block_state_rotate_y(id: i32, clockwise_quarter_turns: u8) -> Option<i32> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    match clockwise_quarter_turns % 4 {
        0 => i32::try_from(id).ok(),
        turns => rotations_26_1::rotation_target(id, usize::from(turns - 1)).map(i32::from),
    }
}

#[cfg(feature = "server-facts")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaBlockStateMirror {
    None,
    LeftRight,
    FrontBack,
}

/// Applies Vanilla's exact horizontal `BlockState.mirror` behavior.
#[cfg(feature = "server-facts")]
pub fn vanilla_block_state_mirror(id: i32, mirror: VanillaBlockStateMirror) -> Option<i32> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    match mirror {
        VanillaBlockStateMirror::None => i32::try_from(id).ok(),
        VanillaBlockStateMirror::LeftRight => mirrors_26_1::mirror_target(id, 0).map(i32::from),
        VanillaBlockStateMirror::FrontBack => mirrors_26_1::mirror_target(id, 1).map(i32::from),
    }
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
    bytes: &'static [u8],
}

impl VanillaBlockCollisionShape {
    pub fn boxes(self) -> impl ExactSizeIterator<Item = VanillaBlockCollisionBox> {
        self.bytes.as_chunks::<48>().0.iter().map(|chunk| {
            let coord = |start: usize| {
                f64::from_bits(u64::from_le_bytes(
                    chunk[start..start + 8].try_into().unwrap(),
                ))
            };
            VanillaBlockCollisionBox {
                min_x: coord(0),
                min_y: coord(8),
                min_z: coord(16),
                max_x: coord(24),
                max_y: coord(32),
                max_z: coord(40),
            }
        })
    }
}

/// Returns the exact cached empty-context collision shape for a non-dynamic
/// built-in Vanilla 26.1 block state. Dynamic states return `None` because
/// their result requires a live level, position, block entity, or collision
/// context.
pub fn vanilla_block_state_static_collision_shape(id: i32) -> Option<VanillaBlockCollisionShape> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    Some(VanillaBlockCollisionShape {
        bytes: collision_shapes_26_1::shape_box_bytes(id)?,
    })
}

/// Returns the exact cached empty-context outline shape for a non-dynamic
/// built-in Vanilla 26.1 block state. Dynamic states return `None` because
/// their result requires a live level, position, block entity, or collision
/// context.
pub fn vanilla_block_state_static_outline_shape(id: i32) -> Option<VanillaBlockCollisionShape> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    Some(VanillaBlockCollisionShape {
        bytes: outline_shapes_26_1::shape_box_bytes(id)?,
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
    light_emission: u8,
    emissive_rendering: bool,
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

    /// Vanilla `BlockState.useShapeForLightOcclusion()` for this fixed state.
    pub const fn use_shape_for_light_occlusion(self) -> bool {
        self.behavior_bits & (1 << 5) != 0
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

    /// Vanilla `BlockState.getLightEmission()` for this fixed 26.1 state.
    pub const fn light_emission(self) -> u8 {
        self.light_emission
    }

    /// Vanilla `BlockState.emissiveRendering(level, pos)` for this fixed state.
    ///
    /// The only 26.1 built-in predicates are magma's unconditional value and
    /// the sculk-sensor phase predicate, so the runtime report preserves the
    /// exact state result without reconstructing it from block names.
    pub const fn emissive_rendering(self) -> bool {
        self.emissive_rendering
    }

    pub const fn face_occlusion_shape_index(self, direction: VanillaBlockSupportDirection) -> u8 {
        self.face_occlusion_shape_indices[direction.index() as usize]
    }

    pub const fn face_occlusion_shape_indices(self) -> [u8; 6] {
        self.face_occlusion_shape_indices
    }
}

/// `BlockState.getBlock()`: the block a built-in 26.1 global state id belongs
/// to.
pub fn vanilla_block_state_block_type_id(id: i32) -> Option<i32> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    render_facts_26_1::render_block_id(id).map(i32::from)
}

/// Returns exact Vanilla renderer mechanism facts for one built-in 26.1 state.
pub fn vanilla_block_state_render_facts(id: i32) -> Option<VanillaBlockStateRenderFacts> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    let (
        block_type_id,
        behavior_bits,
        light_emission,
        emissive_rendering,
        face_occlusion_shape_indices,
    ) = render_facts_26_1::render_state_record(id)?;
    Some(VanillaBlockStateRenderFacts {
        block_type_id,
        behavior_bits,
        light_emission,
        emissive_rendering,
        face_occlusion_shape_indices,
    })
}

/// The single generated 16x16 occupancy palette behind every built-in
/// `BlockState.getFaceOcclusionShape` index.
#[cfg(feature = "client-facts")]
pub fn vanilla_face_occlusion_masks() -> &'static [[u64; 4]] {
    VANILLA_FACE_OCCLUSION_MASKS
}

/// `LightEngine.getOcclusionShape(state, direction)` reduced to the shared
/// generated face-shape palette. Index zero is the empty shape.
pub fn vanilla_block_state_light_face_occlusion_index(
    id: i32,
    direction: VanillaBlockSupportDirection,
) -> Option<u8> {
    let render = vanilla_block_state_render_facts(id)?;
    let support = vanilla_block_state_support_facts(id)?;
    Some(
        if support.can_occlude() && render.use_shape_for_light_occlusion() {
            render.face_occlusion_shape_index(direction)
        } else {
            0
        },
    )
}

/// `Shapes.faceShapeOccludes`: whether the union of two generated face shapes
/// covers the complete block face.
pub fn vanilla_face_shape_union_occludes(first: u8, second: u8) -> Option<bool> {
    let first = VANILLA_FACE_OCCLUSION_MASKS.get(usize::from(first))?;
    let second = VANILLA_FACE_OCCLUSION_MASKS.get(usize::from(second))?;
    Some(
        first
            .iter()
            .zip(second)
            .all(|(first, second)| first | second == u64::MAX),
    )
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

/// Static predicates used by Vanilla's ground path classification
/// (`WalkNodeEvaluator.java:514-581`).
#[cfg(feature = "server-facts")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaBlockStatePathFacts {
    bits: u16,
}

#[cfg(feature = "server-facts")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaWaterPathRule {
    Static,
    StateFluidWater,
    LiquidSourceNotLava,
}

#[cfg(feature = "server-facts")]
impl VanillaBlockStatePathFacts {
    pub const fn is_air(self) -> bool {
        self.bits & 1 != 0
    }

    /// Exact `BlockState.isPathfindable(PathComputationType.LAND)`.
    pub const fn land_pathfindable(self) -> bool {
        self.bits & (1 << 1) != 0
    }

    /// Exact `BlockState.isPathfindable(PathComputationType.WATER)`.
    pub const fn water_pathfindable(self) -> bool {
        self.bits & (1 << 10) != 0
    }

    pub const fn water_path_rule(self) -> Option<VanillaWaterPathRule> {
        match (self.bits >> 11) & 3 {
            0 => Some(VanillaWaterPathRule::Static),
            1 => Some(VanillaWaterPathRule::StateFluidWater),
            2 => Some(VanillaWaterPathRule::LiquidSourceNotLava),
            _ => None,
        }
    }

    pub const fn is_door(self) -> bool {
        self.bits & (1 << 2) != 0
    }

    pub const fn door_can_open_by_hand(self) -> bool {
        self.bits & (1 << 3) != 0
    }

    pub const fn door_open(self) -> bool {
        self.bits & (1 << 4) != 0
    }

    pub const fn is_rail(self) -> bool {
        self.bits & (1 << 5) != 0
    }

    pub const fn is_leaves(self) -> bool {
        self.bits & (1 << 6) != 0
    }

    pub const fn is_fence_gate(self) -> bool {
        self.bits & (1 << 7) != 0
    }

    pub const fn fence_gate_open(self) -> bool {
        self.bits & (1 << 8) != 0
    }

    pub const fn is_lit_campfire(self) -> bool {
        self.bits & (1 << 9) != 0
    }
}

/// Returns exact Vanilla ground-path facts for a built-in 26.1 state id.
#[cfg(feature = "server-facts")]
pub fn vanilla_block_state_path_facts(id: i32) -> Option<VanillaBlockStatePathFacts> {
    let id = u16::try_from(vanilla_block_state_id(id)?).ok()?;
    Some(VanillaBlockStatePathFacts {
        bits: path_facts_26_1::path_fact_bits(id)?,
    })
}

/// Membership test for the coalesced ascending id ranges the generated
/// random-tick tables use.
#[cfg(feature = "server-facts")]
fn range_table_contains(ranges: &[(i32, i32)], id: i32) -> bool {
    ranges
        .binary_search_by(|(start, end)| {
            if id < *start {
                std::cmp::Ordering::Greater
            } else if id > *end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// `BlockState.isRandomlyTicking()` for a built-in 26.1 state
/// (`BlockBehaviour.java:863-865`).
///
/// This is a per-state fact, not a per-block one: `BlockStateBase.initCache`
/// caches `owner.isRandomlyTicking(asState())` (`BlockBehaviour.java:506`) and
/// subclasses answer per state — leaves by `persistent`, crops by age,
/// weathering copper by oxidation stage, `LiquidBlock` by its fluid
/// (`LiquidBlock.java:105-107`). The `randomTicks()` builder calls in
/// `Blocks.java` are therefore only the default, never the answer.
#[cfg(feature = "server-facts")]
pub fn vanilla_block_state_randomly_ticking(id: i32) -> Option<bool> {
    let id = vanilla_block_state_id(id)?;
    Some(range_table_contains(
        VANILLA_BLOCK_STATE_RANDOMLY_TICKING_RANGES,
        id,
    ))
}

/// `BlockState.getFluidState().isRandomlyTicking()` for a built-in 26.1 state
/// (`FluidState.java:87-89`).
///
/// `Fluid.isRandomlyTicking` is false by default and only `LavaFluid`
/// overrides it (`LavaFluid.java:221-223`), so this is true exactly for the
/// lava block states — including the flowing ones, and never for water or a
/// waterlogged block.
#[cfg(feature = "server-facts")]
pub fn vanilla_block_state_fluid_randomly_ticking(id: i32) -> Option<bool> {
    let id = vanilla_block_state_id(id)?;
    Some(range_table_contains(
        VANILLA_BLOCK_STATE_FLUID_RANDOMLY_TICKING_RANGES,
        id,
    ))
}

/// `BlockState.ignitedByLava()` for a built-in 26.1 state
/// (`BlockBehaviour.java:586-588`), the predicate `LavaFluid.isFlammable`
/// answers with (`LavaFluid.java:135-137`).
///
/// Not the same set as the `FireBlock.bootStrap` ignite odds
/// `FireBlock.canBurn` reads (`FireBlock.java:285-287`): a chest carries this
/// flag with no odds at all, and a coal block odds of five without the flag.
#[cfg(feature = "server-facts")]
pub fn vanilla_block_state_ignited_by_lava(id: i32) -> Option<bool> {
    let id = vanilla_block_state_id(id)?;
    Some(range_table_contains(
        VANILLA_BLOCK_STATE_IGNITED_BY_LAVA_RANGES,
        id,
    ))
}

/// `BlockState.getDestroySpeed(level, pos)` for a built-in 26.1 state
/// (`BlockBehaviour.java:639-641`), which is the `destroySpeed` field
/// `BlockStateBase` assigned once from `Properties.destroyTime`
/// (`BlockBehaviour.java:468`).
///
/// The two arguments vanilla takes cannot change the answer: 26.1 declares this
/// accessor exactly once and no block overrides it, which is why the checked-in
/// table has one entry per state and no dynamic marker. `-1.0` is the
/// unbreakable value `getDestroyProgress` short-circuits on
/// (`BlockBehaviour.java:357-359`), and it is a real palette entry rather than
/// an absence.
pub fn vanilla_block_state_destroy_speed(id: i32) -> Option<f32> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    Some(f32::from_bits(destroy_facts_26_1::destroy_speed_bits(id)?))
}

/// `BlockState.requiresCorrectToolForDrops()` for a built-in 26.1 state
/// (`BlockBehaviour.java:897-899`), the field assigned from
/// `Properties.requiresCorrectToolForDrops` (`BlockBehaviour.java:469`).
///
/// `Player.hasCorrectToolForDrops` (`Player.java:618-620`) reads it to decide
/// whether the held stack has to answer for the block at all, and
/// `BlockBehaviour.getDestroyProgress` (`:361`) turns that into the 30 or 100
/// divisor.
pub fn vanilla_block_state_requires_correct_tool_for_drops(id: i32) -> Option<bool> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    destroy_facts_26_1::requires_correct_tool(id)
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
    piston_facts: u8,
}

/// Vanilla `PushReaction` for one fixed built-in block state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaPistonPushReaction {
    Normal,
    Destroy,
    Block,
    Ignore,
    PushOnly,
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

    pub fn piston_push_reaction(self) -> VanillaPistonPushReaction {
        match self.piston_facts & 0x07 {
            0 => VanillaPistonPushReaction::Normal,
            1 => VanillaPistonPushReaction::Destroy,
            2 => VanillaPistonPushReaction::Block,
            3 => VanillaPistonPushReaction::Ignore,
            4 => VanillaPistonPushReaction::PushOnly,
            _ => unreachable!("checked generated piston reaction is outside Vanilla's enum"),
        }
    }

    /// Whether Vanilla reports `getDestroySpeed(level, pos) == -1.0F`.
    pub const fn piston_unbreakable(self) -> bool {
        self.piston_facts & (1 << 3) != 0
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
    let [support_index, flags, dampening_piston, shade_brightness, _] =
        support_facts_26_1::state_raw_record(id)?;
    Some(VanillaBlockStateSupportFacts {
        facts: support_facts_26_1::support_palette_entry(usize::from(support_index))?,
        can_occlude: flags & 1 != 0,
        solid_render: flags & (1 << 1) != 0,
        redstone_conductor: flags & (1 << 2) != 0,
        suffocating: flags & (1 << 3) != 0,
        view_blocking: flags & (1 << 4) != 0,
        light_dampening: dampening_piston & 0x0f,
        shade_brightness,
        liquid: flags & (1 << 5) != 0,
        piston_facts: dampening_piston >> 4,
    })
}

/// Returns the cached Vanilla map material id, including states with live block tint.
#[cfg(feature = "server-facts")]
pub fn vanilla_block_state_map_color_id(id: i32) -> Option<u8> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    support_facts_26_1::map_color_id(id)
}

/// Context-free Vanilla 26.1 facts selected by
/// `FallingDustParticle.Provider.createParticle` around its live block-tint branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaFallingDustStaticFacts {
    provider_accepts: bool,
    falling_block_dust_color_rgb: Option<u32>,
    map_color_rgb: Option<u32>,
}

impl VanillaFallingDustStaticFacts {
    pub const fn provider_accepts(self) -> bool {
        self.provider_accepts
    }

    pub const fn falling_block_dust_color_rgb(self) -> Option<u32> {
        self.falling_block_dust_color_rgb
    }

    pub const fn map_color_rgb(self) -> Option<u32> {
        self.map_color_rgb
    }
}

/// Returns the immutable falling-dust provider facts for one built-in state.
///
/// The optional falling-block color has priority over live block tint. For a
/// non-falling block, consumers query their generation-matched tint catalog
/// before using the optional `map_color_rgb` default.
pub fn vanilla_falling_dust_static_facts(id: i32) -> Option<VanillaFallingDustStaticFacts> {
    let id = usize::try_from(vanilla_block_state_id(id)?).ok()?;
    let state_id = u16::try_from(id).ok()?;
    let [_, flags, _, _, map_color_index] = support_facts_26_1::state_raw_record(id)?;
    Some(VanillaFallingDustStaticFacts {
        provider_accepts: flags & (1 << 6) != 0,
        falling_block_dust_color_rgb: support_facts_26_1::falling_block_dust_color_rgb(state_id),
        map_color_rgb: support_facts_26_1::static_map_color_rgb(map_color_index),
    })
}

/// Whether the built-in block's 26.1 `StateDefinition` contains `property`.
///
/// The checked-in global-state report is the protocol-owned source for both
/// wire ids and property names. Consumers must still resolve `block` through
/// the generated block-type catalog when they need to distinguish an unknown
/// block from a known block without properties.
pub fn vanilla_block_has_property(block: &str, property: &str) -> bool {
    property_strides::block_has_property(block, property)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaBlockPropertyKind {
    Boolean,
    Integer {
        min: i32,
        max: i32,
    },
    Enum {
        values: &'static [(&'static str, i32)],
    },
}

pub fn vanilla_block_property_kind(
    block: &str,
    property: &str,
) -> Option<&'static VanillaBlockPropertyKind> {
    property_strides::block_property_kind(block, property)
}

/// Returns the built-in block property's Vanilla 26.1 value order.
///
/// The checked-in global-state report enumerates every property value in its
/// `StateDefinition` order. Repeated values from the cartesian state product
/// are collapsed while preserving their first occurrence.
pub fn vanilla_block_property_values(
    block: &str,
    property: &str,
) -> Option<&'static [&'static str]> {
    property_strides::block_property_values(block, property)
}

/// Returns the exact number of properties encoded for a built-in block state.
///
/// Vanilla's persistent `BlockState.CODEC` writes every property for blocks
/// whose `StateDefinition` is not empty. Save-schema consumers use this count
/// after validating each name/value so a missing property cannot silently
/// select the reader's default state.
pub fn vanilla_block_property_count(block: &str) -> Option<usize> {
    property_strides::block_property_count(block)
}

/// Identity of one Vanilla 26.1 global block state: registry id, resource
/// name, and canonical property map (empty for property-less states).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VanillaBlockStateIdentity {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

static VANILLA_BLOCK_STATE_IDENTITIES: OnceLock<Vec<VanillaBlockStateIdentity>> = OnceLock::new();

/// Looks up the resource name (`"minecraft:..."`) of a Vanilla 26.1 global
/// block-state id without initializing the owned identity table.
#[inline]
pub fn vanilla_block_state_name(id: i32) -> Option<&'static str> {
    let block_id = vanilla_block_state_block_type_id(id)?;
    crate::block_types::vanilla_block_type(block_id).map(|block| block.resource_id)
}

/// Looks up the identity of a Vanilla 26.1 global block-state id.
///
/// Returns `None` for out-of-range or absent ids.
#[inline]
pub fn vanilla_block_state_identity(id: i32) -> Option<&'static VanillaBlockStateIdentity> {
    vanilla_block_state_identities().get(usize::try_from(id).ok()?)
}

/// Resolves a Vanilla 26.1 block state by resource name and exact canonical
/// property map.
///
/// Returns `None` when no state matches exactly.
pub fn vanilla_block_state_id_by_name_and_properties(
    name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<i32> {
    property_strides::state_by_properties(name, properties)
}

/// Resolves the lowest global block-state id for `name` whose properties match
/// every `(property, value)` pair in `properties`.
pub fn vanilla_block_first_state_id_matching_properties(
    name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<i32> {
    property_strides::first_state_matching_properties(name, properties)
}

#[inline]
pub fn vanilla_block_state_identities() -> &'static [VanillaBlockStateIdentity] {
    VANILLA_BLOCK_STATE_IDENTITIES.get_or_init(|| {
        (0..VANILLA_BLOCK_STATE_COUNT)
            .map(|id| {
                let block = vanilla_block_state_block_type_id(id)
                    .and_then(crate::block_types::vanilla_block_type)
                    .expect("every built-in block state has an owning block");
                VanillaBlockStateIdentity {
                    id,
                    name: block.resource_id.to_owned(),
                    properties: vanilla_block_state_properties(id)
                        .map(|(name, value)| (name.to_owned(), value.to_owned()))
                        .collect(),
                }
            })
            .collect()
    })
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
pub(super) struct BlockStateReport {
    pub(super) version: String,
    pub(super) states: Vec<BlockStateEntry>,
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
pub(super) struct BlockStateEntry {
    pub(super) id: i32,
    pub(super) name: String,
    #[serde(default)]
    pub(super) properties: BTreeMap<String, String>,
}

#[cfg(test)]
pub(super) fn test_block_state_report() -> &'static BlockStateReport {
    static REPORT: OnceLock<BlockStateReport> = OnceLock::new();
    REPORT.get_or_init(|| {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/block_states_26_1.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        serde_json::from_str(&raw).expect("valid block_states_26_1.json fixture")
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn block_state_registry_report_is_contiguous_and_matches_wire_range() {
        let report = test_block_state_report();
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

    #[cfg(feature = "server-facts")]
    #[test]
    fn runtime_generated_rotations_are_closed_exact_quarter_turn_groups() {
        assert_eq!(
            rotations_26_1::STATE_COUNT,
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        for id in 0..VANILLA_BLOCK_STATE_COUNT {
            let clockwise_90 = vanilla_block_state_rotate_y(id, 1).unwrap();
            let clockwise_180 = vanilla_block_state_rotate_y(id, 2).unwrap();
            let counterclockwise_90 = vanilla_block_state_rotate_y(id, 3).unwrap();
            assert_eq!(
                vanilla_block_state_rotate_y(clockwise_90, 1),
                Some(clockwise_180),
                "state {id}"
            );
            assert_eq!(
                vanilla_block_state_rotate_y(clockwise_180, 1),
                Some(counterclockwise_90),
                "state {id}"
            );
            assert_eq!(
                vanilla_block_state_rotate_y(counterclockwise_90, 1),
                Some(id),
                "state {id}"
            );
            assert_eq!(vanilla_block_state_rotate_y(id, 4), Some(id));
            assert_eq!(vanilla_block_state_rotate_y(id, 5), Some(clockwise_90));
        }
        assert_eq!(vanilla_block_state_rotate_y(-1, 1), None);
        assert_eq!(
            vanilla_block_state_rotate_y(VANILLA_BLOCK_STATE_COUNT, 1),
            None
        );
    }

    #[cfg(feature = "server-facts")]
    #[test]
    fn runtime_generated_mirrors_are_exact_involutions() {
        assert_eq!(
            mirrors_26_1::STATE_COUNT,
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        for id in 0..VANILLA_BLOCK_STATE_COUNT {
            for mirror in [
                VanillaBlockStateMirror::LeftRight,
                VanillaBlockStateMirror::FrontBack,
            ] {
                let mirrored = vanilla_block_state_mirror(id, mirror).unwrap();
                assert_eq!(
                    vanilla_block_state_mirror(mirrored, mirror),
                    Some(id),
                    "state {id} with {mirror:?}"
                );
            }
            assert_eq!(
                vanilla_block_state_mirror(id, VanillaBlockStateMirror::None),
                Some(id)
            );
        }
        assert_eq!(
            vanilla_block_state_mirror(-1, VanillaBlockStateMirror::LeftRight),
            None
        );
        assert_eq!(
            vanilla_block_state_mirror(
                VANILLA_BLOCK_STATE_COUNT,
                VanillaBlockStateMirror::FrontBack
            ),
            None
        );
    }

    #[cfg(feature = "server-facts")]
    #[test]
    fn runtime_generated_mirrors_preserve_concrete_block_owner_semantics() {
        let state = |name: &str, properties: &[(&str, &str)]| {
            vanilla_block_state_id_by_name_and_properties(
                name,
                &properties
                    .iter()
                    .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
                    .collect(),
            )
            .unwrap()
        };
        let north_torch = state("minecraft:wall_torch", &[("facing", "north")]);
        let south_torch = state("minecraft:wall_torch", &[("facing", "south")]);
        assert_eq!(
            vanilla_block_state_mirror(north_torch, VanillaBlockStateMirror::LeftRight),
            Some(south_torch)
        );

        let north_fence = state(
            "minecraft:oak_fence",
            &[
                ("east", "false"),
                ("north", "true"),
                ("south", "false"),
                ("waterlogged", "false"),
                ("west", "false"),
            ],
        );
        let south_fence = state(
            "minecraft:oak_fence",
            &[
                ("east", "false"),
                ("north", "false"),
                ("south", "true"),
                ("waterlogged", "false"),
                ("west", "false"),
            ],
        );
        assert_eq!(
            vanilla_block_state_mirror(north_fence, VanillaBlockStateMirror::LeftRight),
            Some(south_fence)
        );

        let ascending_north = state(
            "minecraft:rail",
            &[("shape", "ascending_north"), ("waterlogged", "false")],
        );
        let ascending_south = state(
            "minecraft:rail",
            &[("shape", "ascending_south"), ("waterlogged", "false")],
        );
        assert_eq!(
            vanilla_block_state_mirror(ascending_north, VanillaBlockStateMirror::LeftRight),
            Some(ascending_south)
        );
    }

    #[cfg(feature = "server-facts")]
    #[test]
    fn runtime_generated_rotations_preserve_concrete_block_owner_semantics() {
        let state = |name: &str, properties: &[(&str, &str)]| {
            vanilla_block_state_id_by_name_and_properties(
                name,
                &properties
                    .iter()
                    .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
                    .collect(),
            )
            .unwrap()
        };
        let north_stairs = state(
            "minecraft:oak_stairs",
            &[
                ("facing", "north"),
                ("half", "bottom"),
                ("shape", "straight"),
                ("waterlogged", "false"),
            ],
        );
        let east_stairs = state(
            "minecraft:oak_stairs",
            &[
                ("facing", "east"),
                ("half", "bottom"),
                ("shape", "straight"),
                ("waterlogged", "false"),
            ],
        );
        assert_eq!(
            vanilla_block_state_rotate_y(north_stairs, 1),
            Some(east_stairs)
        );

        let down_north = state("minecraft:jigsaw", &[("orientation", "down_north")]);
        let down_east = state("minecraft:jigsaw", &[("orientation", "down_east")]);
        assert_eq!(vanilla_block_state_rotate_y(down_north, 1), Some(down_east));

        let north_south = state(
            "minecraft:rail",
            &[("shape", "north_south"), ("waterlogged", "false")],
        );
        let east_west = state(
            "minecraft:rail",
            &[("shape", "east_west"), ("waterlogged", "false")],
        );
        assert_eq!(
            vanilla_block_state_rotate_y(north_south, 1),
            Some(east_west)
        );
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

        let report = test_block_state_report();
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
            collision_shapes_26_1::STATE_COUNT,
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        assert_eq!(collision_shapes_26_1::RANGE_COUNT, 318);
        assert_eq!(collision_shapes_26_1::BOX_COUNT, 702);
        assert_eq!(
            (0..collision_shapes_26_1::STATE_COUNT)
                .filter(|&id| collision_shapes_26_1::shape_index(id) == Some(u16::MAX))
                .count(),
            179
        );
        assert_eq!(
            (0..collision_shapes_26_1::RANGE_COUNT)
                .filter_map(collision_shapes_26_1::shape_range)
                .map(|(_, count)| count)
                .max(),
            Some(15)
        );
        assert!((0..collision_shapes_26_1::STATE_COUNT).all(|id| {
            let index = collision_shapes_26_1::shape_index(id).unwrap();
            index == u16::MAX || usize::from(index) < collision_shapes_26_1::RANGE_COUNT
        }));
        assert!((0..collision_shapes_26_1::RANGE_COUNT).all(|idx| {
            let (start, count) = collision_shapes_26_1::shape_range(idx).unwrap();
            usize::try_from(start)
                .ok()
                .and_then(|start| start.checked_add(usize::from(count)))
                .is_some_and(|end| end <= collision_shapes_26_1::BOX_COUNT)
        }));

        let report = test_block_state_report();
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
    fn runtime_generated_static_outline_shapes_cover_every_non_dynamic_state() {
        assert_eq!(
            outline_shapes_26_1::STATE_COUNT,
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        assert_eq!(outline_shapes_26_1::RANGE_COUNT, 850);
        assert_eq!(outline_shapes_26_1::BOX_COUNT, 2377);
        assert_eq!(
            (0..outline_shapes_26_1::STATE_COUNT)
                .filter(|&id| outline_shapes_26_1::shape_index(id) == Some(u16::MAX))
                .count(),
            179
        );
        assert_eq!(
            (0..outline_shapes_26_1::RANGE_COUNT)
                .filter_map(outline_shapes_26_1::shape_range)
                .map(|(_, count)| count)
                .max(),
            Some(15)
        );
        assert!((0..outline_shapes_26_1::STATE_COUNT).all(|id| {
            let index = outline_shapes_26_1::shape_index(id).unwrap();
            index == u16::MAX || usize::from(index) < outline_shapes_26_1::RANGE_COUNT
        }));
        assert!((0..outline_shapes_26_1::RANGE_COUNT).all(|idx| {
            let (start, count) = outline_shapes_26_1::shape_range(idx).unwrap();
            usize::try_from(start)
                .ok()
                .and_then(|start| start.checked_add(usize::from(count)))
                .is_some_and(|end| end <= outline_shapes_26_1::BOX_COUNT)
        }));

        let report = test_block_state_report();
        let state_id = |name: &str| {
            report
                .states
                .iter()
                .find(|state| state.name == name)
                .unwrap()
                .id
        };
        assert!(
            vanilla_block_state_static_outline_shape(state_id("minecraft:air"))
                .unwrap()
                .boxes()
                .next()
                .is_none()
        );
        assert_eq!(
            vanilla_block_state_static_outline_shape(state_id("minecraft:stone"))
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
        // The outline shape is not the collision shape: a plant has no
        // collision box but is what the crosshair picks
        // (`CrossBlock.getShape`).
        assert!(
            vanilla_block_state_static_collision_shape(state_id("minecraft:short_grass"))
                .unwrap()
                .boxes()
                .next()
                .is_none()
        );
        assert!(
            vanilla_block_state_static_outline_shape(state_id("minecraft:short_grass"))
                .unwrap()
                .boxes()
                .next()
                .is_some()
        );

        for state in &report.states {
            let dynamic = vanilla_block_state_support_facts(state.id)
                .unwrap()
                .dynamic_shape();
            assert_eq!(
                vanilla_block_state_static_outline_shape(state.id).is_none(),
                dynamic,
                "{} ({})",
                state.name,
                state.id
            );
        }
        assert!(vanilla_block_state_static_outline_shape(-1).is_none());
        assert!(vanilla_block_state_static_outline_shape(VANILLA_BLOCK_STATE_COUNT).is_none());
    }

    #[cfg(feature = "server-facts")]
    #[test]
    fn block_state_path_facts_cover_the_registry_and_preserve_path_predicates() {
        const {
            assert!(path_facts_26_1::RUN_COUNT > 0);
        }
        assert_eq!(
            path_facts_26_1::run_at(path_facts_26_1::RUN_COUNT - 1).0,
            (VANILLA_BLOCK_STATE_COUNT - 1) as u16
        );
        assert!((1..path_facts_26_1::RUN_COUNT).all(|idx| {
            let prev = path_facts_26_1::run_at(idx - 1);
            let next = path_facts_26_1::run_at(idx);
            prev.0 < next.0 && prev.1 != next.1
        }));
        assert!((0..path_facts_26_1::RUN_COUNT).all(|idx| {
            let (_, bits) = path_facts_26_1::run_at(idx);
            bits & !((1 << 13) - 1) == 0
                && VanillaBlockStatePathFacts { bits }
                    .water_path_rule()
                    .is_some()
        }));
        assert_eq!(
            VanillaBlockStatePathFacts { bits: 3 << 11 }.water_path_rule(),
            None
        );
        assert!(
            (0..VANILLA_BLOCK_STATE_COUNT).all(|id| vanilla_block_state_path_facts(id).is_some())
        );
        assert_eq!(vanilla_block_state_path_facts(-1), None);
        assert_eq!(
            vanilla_block_state_path_facts(VANILLA_BLOCK_STATE_COUNT),
            None
        );

        let report = test_block_state_report();
        let state_id = |name: &str, properties: &[(&str, &str)]| {
            report
                .states
                .iter()
                .find(|state| {
                    state.name == name
                        && properties.iter().all(|(name, value)| {
                            state
                                .properties
                                .get(*name)
                                .is_some_and(|actual| actual == value)
                        })
                })
                .unwrap()
                .id
        };

        let air = vanilla_block_state_path_facts(state_id("minecraft:air", &[])).unwrap();
        assert!(air.is_air());
        assert!(air.land_pathfindable());

        let stone = vanilla_block_state_path_facts(state_id("minecraft:stone", &[])).unwrap();
        assert!(!stone.is_air());
        assert!(!stone.land_pathfindable());
        assert!(!stone.water_pathfindable());
        assert!(
            vanilla_block_state_path_facts(state_id("minecraft:water", &[]))
                .unwrap()
                .water_pathfindable()
        );
        assert_eq!(
            vanilla_block_state_path_facts(state_id("minecraft:water", &[]))
                .unwrap()
                .water_path_rule(),
            Some(VanillaWaterPathRule::LiquidSourceNotLava)
        );
        assert_eq!(
            vanilla_block_state_path_facts(state_id(
                "minecraft:oak_slab",
                &[("waterlogged", "true")]
            ))
            .unwrap()
            .water_path_rule(),
            Some(VanillaWaterPathRule::StateFluidWater)
        );
        assert_eq!(
            vanilla_block_state_path_facts(state_id(
                "minecraft:oak_trapdoor",
                &[("waterlogged", "true")]
            ))
            .unwrap()
            .water_path_rule(),
            Some(VanillaWaterPathRule::Static)
        );
        let lava = vanilla_block_state_path_facts(state_id("minecraft:lava", &[])).unwrap();
        assert!(!lava.land_pathfindable());
        assert!(!lava.water_pathfindable());
        assert!(
            !vanilla_block_state_path_facts(state_id("minecraft:sea_pickle", &[]))
                .unwrap()
                .water_pathfindable()
        );
        assert!(!vanilla_block_state_path_facts(state_id(
            "minecraft:campfire",
            &[("waterlogged", "true")]
        ))
        .unwrap()
        .water_pathfindable());
        assert!(vanilla_block_state_path_facts(state_id(
            "minecraft:oak_slab",
            &[("waterlogged", "true")]
        ))
        .unwrap()
        .water_pathfindable());

        let open_oak_door =
            vanilla_block_state_path_facts(state_id("minecraft:oak_door", &[("open", "true")]))
                .unwrap();
        assert!(open_oak_door.is_door());
        assert!(open_oak_door.door_can_open_by_hand());
        assert!(open_oak_door.door_open());
        assert!(open_oak_door.land_pathfindable());
        let closed_oak_door =
            vanilla_block_state_path_facts(state_id("minecraft:oak_door", &[("open", "false")]))
                .unwrap();
        assert!(closed_oak_door.door_can_open_by_hand());
        assert!(!closed_oak_door.door_open());

        let closed_iron_door =
            vanilla_block_state_path_facts(state_id("minecraft:iron_door", &[("open", "false")]))
                .unwrap();
        assert!(closed_iron_door.is_door());
        assert!(!closed_iron_door.door_can_open_by_hand());
        assert!(!closed_iron_door.door_open());
        let open_iron_door =
            vanilla_block_state_path_facts(state_id("minecraft:iron_door", &[("open", "true")]))
                .unwrap();
        assert!(!open_iron_door.door_can_open_by_hand());
        assert!(open_iron_door.door_open());

        assert!(
            vanilla_block_state_path_facts(state_id("minecraft:rail", &[]))
                .unwrap()
                .is_rail()
        );
        assert!(
            vanilla_block_state_path_facts(state_id("minecraft:oak_leaves", &[]))
                .unwrap()
                .is_leaves()
        );

        let open_gate = vanilla_block_state_path_facts(state_id(
            "minecraft:oak_fence_gate",
            &[("open", "true")],
        ))
        .unwrap();
        assert!(open_gate.is_fence_gate());
        assert!(open_gate.fence_gate_open());

        let lit_campfire =
            vanilla_block_state_path_facts(state_id("minecraft:campfire", &[("lit", "true")]))
                .unwrap();
        assert!(lit_campfire.is_lit_campfire());
        assert!(!vanilla_block_state_path_facts(state_id(
            "minecraft:campfire",
            &[("lit", "false")]
        ))
        .unwrap()
        .is_lit_campfire());
    }

    #[test]
    fn fluid_replaceability_uses_the_runtime_generated_state_ranges() {
        let report = test_block_state_report();
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
        let report = test_block_state_report();
        let state_id = |name: &str| {
            report
                .states
                .iter()
                .find(|state| state.name == name)
                .unwrap()
                .id
        };

        assert_eq!(
            render_facts_26_1::STATE_COUNT,
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        assert_eq!(
            render_facts_26_1::PALETTE_LEN,
            VANILLA_FACE_OCCLUSION_MASKS.len()
        );
        assert!((0..render_facts_26_1::STATE_COUNT).all(|id| {
            let (_, _, light, _, faces) = render_facts_26_1::render_state_record(id).unwrap();
            light <= 15
                && faces
                    .into_iter()
                    .all(|index| usize::from(index) < VANILLA_FACE_OCCLUSION_MASKS.len())
        }));

        let air = vanilla_block_state_render_facts(state_id("minecraft:air")).unwrap();
        assert_eq!(air.face_occlusion_shape_indices(), [0; 6]);
        assert!(!air.use_shape_for_light_occlusion());
        let stone = vanilla_block_state_render_facts(state_id("minecraft:stone")).unwrap();
        assert_eq!(stone.face_occlusion_shape_indices(), [1; 6]);
        assert!(!stone.use_shape_for_light_occlusion());

        let bottom_slab = vanilla_block_state_id_by_name_and_properties(
            "minecraft:oak_slab",
            &[
                ("type".to_string(), "bottom".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();
        assert!(vanilla_block_state_render_facts(bottom_slab)
            .unwrap()
            .use_shape_for_light_occlusion());
        #[cfg(feature = "server-facts")]
        {
            assert_eq!(
                vanilla_block_state_light_face_occlusion_index(
                    bottom_slab,
                    VanillaBlockSupportDirection::Down
                ),
                Some(1)
            );
            assert_eq!(
                vanilla_block_state_light_face_occlusion_index(
                    bottom_slab,
                    VanillaBlockSupportDirection::Up
                ),
                Some(0)
            );
            assert_eq!(
                vanilla_block_state_light_face_occlusion_index(
                    state_id("minecraft:stone"),
                    VanillaBlockSupportDirection::Up
                ),
                Some(0)
            );
            assert_eq!(vanilla_face_shape_union_occludes(1, 0), Some(true));
            assert_eq!(vanilla_face_shape_union_occludes(0, 0), Some(false));
            assert_eq!(vanilla_face_shape_union_occludes(u8::MAX, 0), None);
        }

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

        let magma = vanilla_block_state_render_facts(state_id("minecraft:magma_block")).unwrap();
        assert_eq!(magma.light_emission(), 3);
        assert!(magma.emissive_rendering());
        let conduit = vanilla_block_state_render_facts(state_id("minecraft:conduit")).unwrap();
        assert_eq!(conduit.light_emission(), 15);
        assert!(!conduit.emissive_rendering());
        let redstone_torch = |lit: &str| {
            report
                .states
                .iter()
                .find(|state| {
                    state.name == "minecraft:redstone_torch"
                        && state.properties.get("lit").map(String::as_str) == Some(lit)
                })
                .unwrap()
                .id
        };
        assert_eq!(
            vanilla_block_state_render_facts(redstone_torch("true"))
                .unwrap()
                .light_emission(),
            7
        );
        assert_eq!(
            vanilla_block_state_render_facts(redstone_torch("false"))
                .unwrap()
                .light_emission(),
            0
        );
        let sculk_sensor = |phase: &str| {
            report
                .states
                .iter()
                .find(|state| {
                    state.name == "minecraft:sculk_sensor"
                        && state
                            .properties
                            .get("sculk_sensor_phase")
                            .map(String::as_str)
                            == Some(phase)
                })
                .unwrap()
                .id
        };
        assert!(vanilla_block_state_render_facts(sculk_sensor("active"))
            .unwrap()
            .emissive_rendering());
        assert!(!vanilla_block_state_render_facts(sculk_sensor("inactive"))
            .unwrap()
            .emissive_rendering());

        assert!(vanilla_block_state_render_facts(-1).is_none());
        assert!(vanilla_block_state_render_facts(VANILLA_BLOCK_STATE_COUNT).is_none());
    }

    #[test]
    fn support_facts_use_the_runtime_generated_state_palette() {
        let report = test_block_state_report();
        let state_id = |name: &str| {
            report
                .states
                .iter()
                .find(|state| state.name == name)
                .unwrap()
                .id
        };

        assert_eq!(
            support_facts_26_1::STATE_COUNT,
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        let mut reaction_counts = [0usize; 5];
        let mut unbreakable_count = 0usize;
        for id in 0..support_facts_26_1::STATE_COUNT {
            let [support_index, _, dampening_piston, _, _] =
                support_facts_26_1::state_raw_record(id).unwrap();
            assert!(usize::from(support_index) < support_facts_26_1::SUPPORT_PALETTE_LEN);
            let facts = dampening_piston >> 4;
            assert!(facts & !0x0f == 0 && facts & 0x07 <= 4);
            reaction_counts[usize::from(facts & 0x07)] += 1;
            unbreakable_count += usize::from(facts & (1 << 3) != 0);
        }
        assert_eq!(reaction_counts, [22_869, 6_849, 91, 0, 64]);
        assert_eq!(unbreakable_count, 116);

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
        assert_eq!(
            stone.piston_push_reaction(),
            VanillaPistonPushReaction::Normal
        );
        assert!(!stone.piston_unbreakable());
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
        assert_eq!(
            leaves.piston_push_reaction(),
            VanillaPistonPushReaction::Destroy
        );
        assert_eq!(
            vanilla_block_state_support_facts(state_id("minecraft:sticky_piston"))
                .unwrap()
                .piston_push_reaction(),
            VanillaPistonPushReaction::Block
        );
        assert_eq!(
            vanilla_block_state_support_facts(state_id("minecraft:white_glazed_terracotta"))
                .unwrap()
                .piston_push_reaction(),
            VanillaPistonPushReaction::PushOnly
        );
        assert!(
            vanilla_block_state_support_facts(state_id("minecraft:bedrock"))
                .unwrap()
                .piston_unbreakable()
        );
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

    #[cfg(feature = "server-facts")]
    #[test]
    fn map_color_ids_include_tinted_block_states() {
        assert!(
            (0..VANILLA_BLOCK_STATE_COUNT).all(|id| vanilla_block_state_map_color_id(id).is_some())
        );
        let report = test_block_state_report();
        for (name, material_id) in [
            ("minecraft:air", 0),
            ("minecraft:grass_block", 1),
            ("minecraft:water", 12),
        ] {
            for state in report.states.iter().filter(|state| state.name == name) {
                assert_eq!(
                    vanilla_block_state_map_color_id(state.id),
                    Some(material_id)
                );
            }
        }
        assert_eq!(vanilla_block_state_map_color_id(-1), None);
        assert_eq!(
            vanilla_block_state_map_color_id(VANILLA_BLOCK_STATE_COUNT),
            None
        );
    }

    #[test]
    fn falling_dust_static_facts_use_the_runtime_generated_state_palette() {
        let report = test_block_state_report();
        let state_id = |name: &str, properties: &[(&str, &str)]| {
            report
                .states
                .iter()
                .find(|state| {
                    state.name == name
                        && properties.iter().all(|(property, value)| {
                            state
                                .properties
                                .get(*property)
                                .is_some_and(|actual| actual == value)
                        })
                })
                .unwrap()
                .id
        };

        assert_eq!(support_facts_26_1::STATIC_MAP_COLOR_PALETTE_LEN, 62);
        let mut accepts_count = 0usize;
        let mut static_map_count = 0usize;
        for id in 0..support_facts_26_1::STATE_COUNT {
            let [_, flags, _, _, map_color_index] =
                support_facts_26_1::state_raw_record(id).unwrap();
            accepts_count += usize::from(flags & (1 << 6) != 0);
            assert!(
                map_color_index == 0
                    || usize::from(map_color_index)
                        <= support_facts_26_1::STATIC_MAP_COLOR_PALETTE_LEN
            );
            static_map_count += usize::from(map_color_index != 0);
        }
        assert_eq!(accepts_count, 29_790);
        assert_eq!(static_map_count, 28_229);
        assert_eq!(support_facts_26_1::FALLING_DUST_COLOR_COUNT, 32);
        assert!(
            (1..support_facts_26_1::FALLING_DUST_COLOR_COUNT).all(|idx| {
                support_facts_26_1::falling_dust_entry(idx - 1).unwrap().0
                    < support_facts_26_1::falling_dust_entry(idx).unwrap().0
            })
        );
        assert!(
            (0..support_facts_26_1::FALLING_DUST_COLOR_COUNT).all(|idx| {
                let (state_id, color) = support_facts_26_1::falling_dust_entry(idx).unwrap();
                i32::from(state_id) < VANILLA_BLOCK_STATE_COUNT && color <= 0xFF_FFFF
            })
        );

        let air = vanilla_falling_dust_static_facts(state_id("minecraft:air", &[])).unwrap();
        assert!(air.provider_accepts());
        assert_eq!(air.falling_block_dust_color_rgb(), None);
        assert_eq!(air.map_color_rgb(), Some(0x00_0000));
        assert!(
            !vanilla_falling_dust_static_facts(state_id("minecraft:barrier", &[]))
                .unwrap()
                .provider_accepts()
        );

        for (name, dust_color, map_color) in [
            ("minecraft:sand", 0xDB_D3_A0, 0xF7_E9_A3),
            ("minecraft:red_sand", 0xA9_58_21, 0xD8_7F_33),
            ("minecraft:anvil", 0xA7_A7_A7, 0xA7_A7_A7),
            ("minecraft:white_concrete_powder", 0xFF_FF_FF, 0xFF_FF_FF),
            ("minecraft:dragon_egg", 0x00_00_00, 0x19_19_19),
        ] {
            let facts = vanilla_falling_dust_static_facts(state_id(name, &[])).unwrap();
            assert!(facts.provider_accepts(), "{name}");
            assert_eq!(
                facts.falling_block_dust_color_rgb(),
                Some(dust_color),
                "{name}"
            );
            assert_eq!(facts.map_color_rgb(), Some(map_color), "{name}");
        }

        let bed_foot =
            vanilla_falling_dust_static_facts(state_id("minecraft:white_bed", &[("part", "foot")]))
                .unwrap();
        let bed_head =
            vanilla_falling_dust_static_facts(state_id("minecraft:white_bed", &[("part", "head")]))
                .unwrap();
        assert_eq!(bed_foot.map_color_rgb(), Some(0xFF_FF_FF));
        assert_eq!(bed_head.map_color_rgb(), Some(0xC7_C7_C7));
        assert_eq!(
            vanilla_falling_dust_static_facts(state_id("minecraft:birch_leaves", &[]))
                .unwrap()
                .map_color_rgb(),
            None
        );
        assert_eq!(vanilla_falling_dust_static_facts(-1), None);
        assert_eq!(
            vanilla_falling_dust_static_facts(VANILLA_BLOCK_STATE_COUNT),
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
        assert_eq!(vanilla_block_property_count("minecraft:stone"), Some(0));
        assert_eq!(vanilla_block_property_count("minecraft:oak_log"), Some(1));
        assert_eq!(vanilla_block_property_count("minecraft:missing"), None);
    }

    #[test]
    fn block_state_identities_round_trip_names_and_properties() {
        let air_id =
            vanilla_block_default_state_id(crate::block_types::VANILLA_BLOCK_AIR_ID).unwrap();
        let air = vanilla_block_state_identity(air_id).unwrap();
        assert_eq!(air.id, air_id);
        assert_eq!(air.name, "minecraft:air");
        assert!(air.properties.is_empty());

        let stone_id =
            vanilla_block_default_state_id(crate::block_types::VANILLA_BLOCK_STONE_ID).unwrap();
        let stone = vanilla_block_state_identity(stone_id).unwrap();
        assert_eq!(stone.id, stone_id);
        assert_eq!(stone.name, "minecraft:stone");
        assert!(stone.properties.is_empty());

        let oak_stairs_properties = BTreeMap::from([
            ("facing".to_string(), "north".to_string()),
            ("half".to_string(), "bottom".to_string()),
            ("shape".to_string(), "straight".to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ]);
        let oak_stairs_id = vanilla_block_state_id_by_name_and_properties(
            "minecraft:oak_stairs",
            &oak_stairs_properties,
        )
        .unwrap();
        let oak_stairs = vanilla_block_state_identity(oak_stairs_id).unwrap();
        assert_eq!(oak_stairs.id, oak_stairs_id);
        assert_eq!(oak_stairs.name, "minecraft:oak_stairs");
        assert_eq!(oak_stairs.properties, oak_stairs_properties);

        // Every registry entry round-trips through both lookup directions.
        let report = test_block_state_report();
        for state in &report.states {
            let identity = vanilla_block_state_identity(state.id).unwrap();
            assert_eq!(
                vanilla_block_state_name(state.id),
                Some(state.name.as_str())
            );
            assert_eq!(identity.id, state.id);
            assert_eq!(identity.name, state.name);
            assert_eq!(identity.properties, state.properties);
            assert_eq!(
                vanilla_block_state_id_by_name_and_properties(&state.name, &state.properties),
                Some(state.id)
            );
        }
    }

    #[test]
    fn destroy_facts_carry_the_two_block_side_inputs_of_get_destroy_progress() {
        let report = test_block_state_report();
        let state_id = |name: &str| {
            report
                .states
                .iter()
                .find(|state| state.name == name)
                .unwrap()
                .id
        };

        assert_eq!(
            destroy_facts_26_1::STATE_COUNT,
            VANILLA_BLOCK_STATE_COUNT as usize
        );
        assert!((0..destroy_facts_26_1::STATE_COUNT).all(|id| {
            usize::from(destroy_facts_26_1::speed_index(id).unwrap())
                < destroy_facts_26_1::PALETTE_LEN
        }));
        assert!((0..VANILLA_BLOCK_STATE_COUNT)
            .all(|id| vanilla_block_state_requires_correct_tool_for_drops(id).is_some()));

        // `Blocks.java:54-57` stone, `:85` dirt, `:92-95` cobblestone,
        // `:275-283` bedrock, `:284-292` water, `:609-617` glass,
        // `:640-642` sandstone: the `strength(..)` first argument is
        // `Properties.destroyTime`, and `requiresCorrectToolForDrops()` is the
        // builder call that appears beside it.
        let speed = |name: &str| vanilla_block_state_destroy_speed(state_id(name)).unwrap();
        assert_eq!(speed("minecraft:stone"), 1.5);
        assert_eq!(speed("minecraft:dirt"), 0.5);
        assert_eq!(speed("minecraft:cobblestone"), 2.0);
        assert_eq!(speed("minecraft:bedrock"), -1.0);
        assert_eq!(speed("minecraft:glass"), 0.3);
        assert_eq!(speed("minecraft:sandstone"), 0.8);
        // `Blocks.java:284-292` is `strength(100.0F)`; every flowing level is
        // the same block and therefore carries the same field.
        assert_eq!(speed("minecraft:water"), 100.0);
        assert_eq!(speed("minecraft:air"), 0.0);

        let requires = |name: &str| {
            vanilla_block_state_requires_correct_tool_for_drops(state_id(name)).unwrap()
        };
        assert!(requires("minecraft:stone"));
        assert!(requires("minecraft:cobblestone"));
        assert!(requires("minecraft:sandstone"));
        assert!(!requires("minecraft:dirt"));
        assert!(!requires("minecraft:glass"));
        // `:275-283` declares no `requiresCorrectToolForDrops()`, so an
        // unbreakable block is not a correct-tool block either; the two facts
        // are independent.
        assert!(!requires("minecraft:bedrock"));

        assert_eq!(vanilla_block_state_destroy_speed(-1), None);
        assert_eq!(
            vanilla_block_state_destroy_speed(VANILLA_BLOCK_STATE_COUNT),
            None
        );
        assert_eq!(
            vanilla_block_state_requires_correct_tool_for_drops(VANILLA_BLOCK_STATE_COUNT),
            None
        );
    }

    #[test]
    fn block_state_identities_reject_unknown_or_inexact_states() {
        let empty = BTreeMap::new();
        assert_eq!(
            vanilla_block_state_id_by_name_and_properties("minecraft:missing", &empty),
            None
        );
        // Stone has no properties, so any property map does not match.
        let wrong_value = BTreeMap::from([("snowy".to_string(), "true".to_string())]);
        assert_eq!(
            vanilla_block_state_id_by_name_and_properties("minecraft:stone", &wrong_value),
            None
        );
        // An exact match requires every property, not a subset.
        let subset = BTreeMap::from([("facing".to_string(), "north".to_string())]);
        assert_eq!(
            vanilla_block_state_id_by_name_and_properties("minecraft:oak_stairs", &subset),
            None
        );
        assert_eq!(vanilla_block_state_name(-1), None);
        assert_eq!(vanilla_block_state_name(VANILLA_BLOCK_STATE_COUNT), None);
        assert_eq!(vanilla_block_state_identity(-1), None);
        assert_eq!(
            vanilla_block_state_identity(VANILLA_BLOCK_STATE_COUNT),
            None
        );
    }
}
