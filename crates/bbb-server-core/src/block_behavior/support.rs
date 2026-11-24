//! Shared block behavior facts and live block removal.

use bbb_server_types::block_queries::{
    block_type_of, empty_context_shape, static_collision_shape_is_full_block, EmptyContextShape,
};
use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;

use crate::random::GameRandom;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_cycle_property,
    vanilla_block_state_id_by_name_and_properties, vanilla_block_state_property,
    vanilla_block_state_static_collision_shape, vanilla_block_state_support_facts,
};
use bbb_protocol::block_types::{
    vanilla_block_type, vanilla_block_type_for_resource_id, VanillaCanSurviveOverride,
    VanillaGetStateForPlacementOverride, VanillaMayPlaceOnOverride, VanillaRandomTickOverride,
    VanillaTickOverride, VANILLA_BLOCK_FIRE_ID, VANILLA_BLOCK_SOUL_FIRE_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::block_getter::BlockTags;
use crate::fluid::HORIZONTAL;
use bbb_protocol::block_state_types::VanillaBlockSupportType;
use bbb_server_types::block_queries::{can_support_center_at_with_membership, face_sturdy};

const UNSTABLE_BOTTOM_CENTER_TAG: &str = "minecraft:unstable_bottom_center";

/// `Block.canSupportCenter` (`Block.java:343-348`) against the loaded tags.
pub(crate) fn can_support_center_tagged(
    tags: &dyn BlockTags,
    block_state_id: i32,
    direction: Direction,
) -> Option<bool> {
    if direction == Direction::Down && tags.block_in_tag(block_state_id, UNSTABLE_BOTTOM_CENTER_TAG)
    {
        return Some(false);
    }
    face_sturdy(block_state_id, direction, VanillaBlockSupportType::Center)
}

/// Positioned [`can_support_center_tagged`].
pub(crate) fn can_support_center_at_tagged(
    tags: &dyn BlockTags,
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
) -> bool {
    can_support_center_at_with_membership(
        block_state_id,
        pos,
        direction,
        direction == Direction::Down
            && tags.block_in_tag(block_state_id, UNSTABLE_BOTTOM_CENTER_TAG),
    )
}

/// `BlockState.is(Block)`.
pub fn state_is_block(block_state_id: i32, block_type_id: i32) -> bool {
    block_type_of(block_state_id) == Some(block_type_id)
}

/// `Block.defaultBlockState()`.
///
/// # Panics
///
/// Panics for a block id the generated 26.1 registry does not know; every
/// caller here passes a `bbb_protocol::block_types` constant.
pub fn default_block_state_id(block_type_id: i32) -> i32 {
    vanilla_block_default_state_id(block_type_id)
        .expect("generated block registry has a default state for every built-in block")
}

/// The state of `block_type_id` carrying one integer property value, resolved
/// through the generated registry so no id is written down.
pub fn block_state_with_int(block_type_id: i32, property: &str, value: i32) -> Option<i32> {
    let name = vanilla_block_type(block_type_id)?.resource_id;
    let mut properties = BTreeMap::new();
    properties.insert(property.to_string(), value.to_string());
    vanilla_block_state_id_by_name_and_properties(name, &properties)
}

pub const fn is_base_fire_block_type(block_type_id: i32) -> bool {
    matches!(
        block_type_id,
        VANILLA_BLOCK_FIRE_ID | VANILLA_BLOCK_SOUL_FIRE_ID
    )
}

pub fn base_fire_block_type_ids() -> &'static HashSet<i32> {
    static IDS: OnceLock<HashSet<i32>> = OnceLock::new();
    IDS.get_or_init(|| HashSet::from([VANILLA_BLOCK_FIRE_ID, VANILLA_BLOCK_SOUL_FIRE_ID]))
}

pub fn state_liquid(block_state_id: i32) -> bool {
    vanilla_block_state_support_facts(block_state_id).is_some_and(|facts| facts.liquid())
}

pub fn light_dampening(block_state_id: i32) -> i32 {
    light_dampening_with_registry(
        block_state_id,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn light_dampening_with_registry(
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> i32 {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        return i32::from(facts.light_dampening());
    }
    content_registry
        .block_by_state_id(block_state_id)
        .map_or(
            0,
            |(block, _)| if block.full_cube_collision { 15 } else { 0 },
        )
}

pub fn solid_render(block_state_id: i32) -> Option<bool> {
    solid_render_with_registry(
        block_state_id,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn solid_render_with_registry(
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<bool> {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        return Some(facts.solid_render());
    }
    content_registry
        .block_by_state_id(block_state_id)
        .map(|(block, _)| block.full_cube_collision)
}

/// The `can_survive_override` class tag the generated block catalog carries,
/// used to recognise a whole vanilla class without naming its members.
pub fn can_survive_override(block_type_id: i32) -> Option<VanillaCanSurviveOverride> {
    vanilla_block_type(block_type_id)?.can_survive_override
}

/// The `get_state_for_placement_override` class tag the generated block
/// catalog carries, read the same way [`can_survive_override`] is: as the name
/// of the vanilla class that declares the override, which for a method no
/// subclass touches is the whole class.
pub fn get_state_for_placement_override(
    block_type_id: i32,
) -> Option<VanillaGetStateForPlacementOverride> {
    vanilla_block_type(block_type_id)?.get_state_for_placement_override
}

/// The `may_place_on_override` class tag the generated block catalog carries.
///
/// Read exactly as [`can_survive_override`] is, with one difference:
/// `mayPlaceOn` is declared by `VegetationBlock` rather than by
/// `BlockBehaviour`, so `None` means "not a `VegetationBlock`" instead of
/// "takes the default body".
pub fn may_place_on_override(block_type_id: i32) -> Option<VanillaMayPlaceOnOverride> {
    vanilla_block_type(block_type_id)?.may_place_on_override
}

/// Live empty-context equivalent of `state.isCollisionShapeFullBlock(...)`.
pub fn collision_shape_full_block_at(block_state_id: i32, pos: BlockPos) -> bool {
    let Some(facts) = vanilla_block_state_support_facts(block_state_id) else {
        return false;
    };
    if !facts.dynamic_shape() {
        return vanilla_block_state_static_collision_shape(block_state_id)
            .is_some_and(static_collision_shape_is_full_block);
    }
    let collision = empty_context_shape(block_state_id, pos, facts);
    matches!(collision, EmptyContextShape::Full)
}

/// Zero-allocation `BlockState.getValue(EnumProperty)` serialized name lookup.
pub fn str_property(block_state_id: i32, property: &str) -> Option<&'static str> {
    vanilla_block_state_property(block_state_id, property)
}

/// `BlockState.getValue(EnumProperty)` as the value's serialized name, i.e.
/// what `StringRepresentable.getSerializedName` produced when the state was
/// written down.
pub fn string_property(block_state_id: i32, property: &str) -> Option<String> {
    str_property(block_state_id, property).map(str::to_owned)
}

/// The `may_place_on_support_tag` fact the generated block catalog carries:
/// the `TagKey<Block>` a block's *registration* handed the constructor whose
/// `mayPlaceOn` reads it.
pub fn may_place_on_support_tag(block_type_id: i32) -> Option<&'static str> {
    vanilla_block_type(block_type_id)?.may_place_on_support_tag
}

/// The `random_tick_override` class tag the generated block catalog carries:
/// the class whose `BlockBehaviour.randomTick` body this block reaches.
///
pub fn random_tick_override(block_type_id: i32) -> Option<VanillaRandomTickOverride> {
    vanilla_block_type(block_type_id)?.random_tick_override
}

/// The `tick_override` class tag the generated block catalog carries: the
/// class whose `BlockBehaviour.tick` body this block reaches. Read exactly as
/// [`random_tick_override`] is.
pub fn tick_override(block_type_id: i32) -> Option<VanillaTickOverride> {
    vanilla_block_type(block_type_id)?.tick_override
}

pub fn direction_property(block_state_id: i32, property: &str) -> Option<Direction> {
    match vanilla_block_state_property(block_state_id, property)? {
        "down" => Some(Direction::Down),
        "up" => Some(Direction::Up),
        "north" => Some(Direction::North),
        "south" => Some(Direction::South),
        "west" => Some(Direction::West),
        "east" => Some(Direction::East),
        _ => None,
    }
}

///
/// The order is observable through the draw: `nextInt(4)` picks an index, so
/// 0 is north and 3 is west.
pub fn random_horizontal_direction(random: &GameRandom) -> Direction {
    let index = random.next_int(i32::try_from(HORIZONTAL.len()).expect("four horizontal faces"));
    HORIZONTAL[usize::try_from(index).expect("nextInt answers a non-negative index")]
}

pub fn cycle_property(block_state_id: i32, property: &str) -> Option<i32> {
    vanilla_block_state_cycle_property(block_state_id, property)
}

/// A `ResourceKey<Block>` a registration handed its own block constructor, as
/// a `BuiltInRegistries.BLOCK` id.
///
/// The generated `constructor_block_ids` fact names the instance field the
/// constructor stored the key in, so `field` is the vanilla field name — the
/// same kind of reference to a vanilla symbol a method name is, and the only
/// part of the join that is not derived. `None` is "that class takes no such
/// argument", which is the fail-closed answer for a body that needs one.
pub fn constructor_block_type_id(block_type_id: i32, field: &str) -> Option<i32> {
    let entry = vanilla_block_type(block_type_id)?;
    let resource_id = entry
        .constructor_block_ids
        .iter()
        .find(|(name, _)| *name == field)
        .map(|(_, resource_id)| *resource_id)?;
    vanilla_block_type_for_resource_id(resource_id).map(|entry| entry.id)
}

/// A `TagKey<Block>` a registration handed its own block constructor, as the
/// tag resource id. Read exactly as [`constructor_block_type_id`] is.
pub fn constructor_block_tag(block_type_id: i32, field: &str) -> Option<&'static str> {
    vanilla_block_type(block_type_id)?
        .constructor_block_tags
        .iter()
        .find(|(name, _)| *name == field)
        .map(|(_, tag_id)| *tag_id)
}
