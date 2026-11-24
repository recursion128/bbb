use std::collections::BTreeMap;

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_with_property,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;

use super::super::{shapes::outline_shape_for_state, BlockOutlineShape};

/// The block's default state with `properties` applied; `None` for an
/// unknown block or property value.
pub(super) fn state_id(name: &str, properties: &BTreeMap<String, String>) -> Option<i32> {
    let block = vanilla_block_type_for_resource_id(name)?;
    properties.iter().try_fold(
        vanilla_block_default_state_id(block.id)?,
        |state, (property, value)| vanilla_block_state_with_property(state, property, value),
    )
}

pub(super) fn outline_shape_for_block(
    name: Option<&str>,
    properties: &BTreeMap<String, String>,
) -> Option<BlockOutlineShape> {
    outline_shape_for_state(state_id(name?, properties)?, None)
}

pub(super) fn snow_properties(layers: u8) -> BTreeMap<String, String> {
    BTreeMap::from([("layers".to_string(), layers.to_string())])
}

pub(super) fn pressure_plate_properties(powered: bool) -> BTreeMap<String, String> {
    BTreeMap::from([("powered".to_string(), powered.to_string())])
}

pub(super) fn wall_sign_properties(facing: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ])
}

pub(super) fn stair_properties(facing: &str, half: &str, shape: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("half".to_string(), half.to_string()),
        ("shape".to_string(), shape.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ])
}

pub(super) fn trapdoor_properties(
    facing: &str,
    half: &str,
    open: bool,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("half".to_string(), half.to_string()),
        ("open".to_string(), open.to_string()),
        ("powered".to_string(), "false".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ])
}

pub(super) fn door_properties(
    facing: &str,
    hinge: &str,
    open: bool,
    half: &str,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("half".to_string(), half.to_string()),
        ("hinge".to_string(), hinge.to_string()),
        ("open".to_string(), open.to_string()),
        ("powered".to_string(), "false".to_string()),
    ])
}

pub(super) fn ladder_properties(facing: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ])
}

pub(super) fn wall_torch_properties(facing: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("facing".to_string(), facing.to_string())])
}

pub(super) fn button_properties(
    facing: &str,
    face: &str,
    powered: bool,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("face".to_string(), face.to_string()),
        ("powered".to_string(), powered.to_string()),
    ])
}

pub(super) fn lever_properties(
    facing: &str,
    face: &str,
    powered: bool,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("face".to_string(), face.to_string()),
        ("powered".to_string(), powered.to_string()),
    ])
}

pub(super) fn fence_properties<const N: usize>(connected: [&str; N]) -> BTreeMap<String, String> {
    let mut properties = BTreeMap::from([
        ("north".to_string(), "false".to_string()),
        ("east".to_string(), "false".to_string()),
        ("south".to_string(), "false".to_string()),
        ("west".to_string(), "false".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    for direction in connected {
        properties.insert(direction.to_string(), "true".to_string());
    }
    properties
}

pub(super) fn fence_gate_properties(
    facing: &str,
    in_wall: bool,
    open: bool,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("in_wall".to_string(), in_wall.to_string()),
        ("open".to_string(), open.to_string()),
        ("powered".to_string(), "false".to_string()),
    ])
}

pub(super) fn pane_properties<const N: usize>(connected: [&str; N]) -> BTreeMap<String, String> {
    let mut properties = BTreeMap::from([
        ("north".to_string(), "false".to_string()),
        ("east".to_string(), "false".to_string()),
        ("south".to_string(), "false".to_string()),
        ("west".to_string(), "false".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    for direction in connected {
        properties.insert(direction.to_string(), "true".to_string());
    }
    properties
}

pub(super) fn wall_properties<const N: usize>(
    up: bool,
    sides: [(&str, &str); N],
) -> BTreeMap<String, String> {
    let mut properties = BTreeMap::from([
        ("up".to_string(), up.to_string()),
        ("north".to_string(), "none".to_string()),
        ("east".to_string(), "none".to_string()),
        ("south".to_string(), "none".to_string()),
        ("west".to_string(), "none".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    for (direction, side) in sides {
        properties.insert(direction.to_string(), side.to_string());
    }
    properties
}

pub(super) fn pale_moss_properties(
    bottom: bool,
    sides: [(&str, &str); 4],
) -> BTreeMap<String, String> {
    let mut properties = BTreeMap::from([("bottom".to_string(), bottom.to_string())]);
    for (name, value) in sides {
        properties.insert(name.to_string(), value.to_string());
    }
    properties
}
