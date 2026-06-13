use std::collections::BTreeMap;

pub(super) fn slab_properties(slab_type: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("type".to_string(), slab_type.to_string())])
}

pub(super) fn snow_properties(layers: u8) -> BTreeMap<String, String> {
    BTreeMap::from([("layers".to_string(), layers.to_string())])
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

pub(super) fn redstone_torch_properties(lit: bool) -> BTreeMap<String, String> {
    BTreeMap::from([("lit".to_string(), lit.to_string())])
}

pub(super) fn redstone_wall_torch_properties(facing: &str, lit: bool) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("lit".to_string(), lit.to_string()),
    ])
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
