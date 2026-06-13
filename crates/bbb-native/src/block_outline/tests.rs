use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_renderer::{SelectionBox, SelectionOutline};
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::*;

#[test]
fn outline_shape_uses_vanilla_slab_type_property() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_slab"), &slab_properties("bottom")),
        Some(BlockOutlineShape::single(BlockOutlineBox::BOTTOM_SLAB))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:smooth_stone_slab"), &slab_properties("top")),
        Some(BlockOutlineShape::single(BlockOutlineBox::TOP_SLAB))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:petrified_oak_slab"),
            &slab_properties("double"),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_slab"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_slab"), &slab_properties("unexpected")),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_snow_layers_property() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:snow"), &snow_properties(1)),
        Some(BlockOutlineShape::single(BlockOutlineBox {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 0.125, 1.0],
        }))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:snow"), &snow_properties(8)),
        Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:snow"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:snow"), &snow_properties(9)),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_stair_straight_boxes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_stairs"),
            &stair_properties("north", "bottom", "straight"),
        ),
        Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::BOTTOM_SLAB,
            BlockOutlineBox::STAIR_NORTH_HALF,
        ]))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_stairs"),
            &stair_properties("east", "bottom", "straight"),
        ),
        Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::BOTTOM_SLAB,
            BlockOutlineBox {
                min: [0.5, 0.5, 0.0],
                max: [1.0, 1.0, 1.0],
            },
        ]))
    );
}

#[test]
fn outline_shape_uses_vanilla_top_stair_boxes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_stairs"),
            &stair_properties("north", "top", "straight"),
        ),
        Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::TOP_SLAB,
            BlockOutlineBox {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 0.5, 0.5],
            },
        ]))
    );
}

#[test]
fn outline_shape_uses_vanilla_stair_corner_boxes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_stairs"),
            &stair_properties("north", "bottom", "outer_right"),
        ),
        Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::BOTTOM_SLAB,
            BlockOutlineBox {
                min: [0.5, 0.5, 0.0],
                max: [1.0, 1.0, 0.5],
            },
        ]))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_stairs"),
            &stair_properties("north", "bottom", "inner_left"),
        ),
        Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::BOTTOM_SLAB,
            BlockOutlineBox {
                min: [0.0, 0.5, 0.0],
                max: [0.5, 1.0, 1.0],
            },
            BlockOutlineBox {
                min: [0.5, 0.5, 0.0],
                max: [1.0, 1.0, 0.5],
            },
        ]))
    );
}

#[test]
fn outline_shape_rejects_invalid_stair_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_stairs"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_stairs"),
            &stair_properties("north", "middle", "straight"),
        ),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_stairs"),
            &stair_properties("north", "bottom", "sideways"),
        ),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_closed_trapdoor_boxes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_trapdoor"),
            &trapdoor_properties("north", "bottom", false),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::TRAPDOOR_BOTTOM))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_trapdoor"),
            &trapdoor_properties("south", "top", false),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::TRAPDOOR_TOP))
    );
}

#[test]
fn outline_shape_uses_vanilla_open_trapdoor_boxes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_trapdoor"),
            &trapdoor_properties("north", "bottom", true),
        ),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::TRAPDOOR_NORTH_OPEN,
        ))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_trapdoor"),
            &trapdoor_properties("east", "top", true),
        ),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::TRAPDOOR_EAST_OPEN
        ))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_trapdoor"),
            &trapdoor_properties("south", "bottom", true),
        ),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::TRAPDOOR_SOUTH_OPEN,
        ))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_trapdoor"),
            &trapdoor_properties("west", "top", true),
        ),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::TRAPDOOR_WEST_OPEN
        ))
    );
}

#[test]
fn outline_shape_rejects_invalid_trapdoor_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_trapdoor"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_trapdoor"),
            &trapdoor_properties("north", "middle", false),
        ),
        None
    );
    let mut properties = trapdoor_properties("north", "bottom", true);
    properties.insert("open".to_string(), "sometimes".to_string());
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_trapdoor"), &properties),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_disconnected_fence_post() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_fence"), &fence_properties([])),
        Some(BlockOutlineShape::single(BlockOutlineBox::FENCE_POST))
    );
}

#[test]
fn outline_shape_uses_vanilla_fence_connection_boxes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:nether_brick_fence"),
            &fence_properties(["north", "east"]),
        ),
        Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::FENCE_POST,
            BlockOutlineBox::FENCE_NORTH_ARM,
            BlockOutlineBox::FENCE_EAST_ARM,
        ]))
    );
}

#[test]
fn outline_shape_rejects_invalid_fence_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_fence"), &BTreeMap::new()),
        None
    );

    let mut properties = fence_properties(["west"]);
    properties.insert("west".to_string(), "sometimes".to_string());
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_fence"), &properties),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_disconnected_wall_post() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:cobblestone_wall"),
            &wall_properties(true, [])
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::WALL_POST))
    );
}

#[test]
fn outline_shape_uses_vanilla_wall_side_boxes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:mossy_cobblestone_wall"),
            &wall_properties(true, [("north", "low"), ("east", "tall")]),
        ),
        Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::WALL_POST,
            BlockOutlineBox::WALL_NORTH_LOW,
            BlockOutlineBox::WALL_EAST_TALL,
        ]))
    );
}

#[test]
fn outline_shape_uses_vanilla_wall_without_post() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:cobblestone_wall"),
            &wall_properties(false, [("south", "low"), ("west", "tall")]),
        ),
        Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::WALL_SOUTH_LOW,
            BlockOutlineBox::WALL_WEST_TALL,
        ]))
    );
}

#[test]
fn outline_shape_rejects_invalid_wall_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:cobblestone_wall"), &BTreeMap::new()),
        None
    );

    let mut properties = wall_properties(true, [("north", "low")]);
    properties.insert("north".to_string(), "medium".to_string());
    assert_eq!(
        outline_shape_for_block(Some("minecraft:cobblestone_wall"), &properties),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_flat_carpet_shape() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:white_carpet"), &BTreeMap::new()),
        Some(BlockOutlineShape::single(BlockOutlineBox::CARPET))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:moss_carpet"), &BTreeMap::new()),
        Some(BlockOutlineShape::single(BlockOutlineBox::CARPET))
    );
}

#[test]
fn outline_shape_uses_vanilla_pale_moss_carpet_boxes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:pale_moss_carpet"),
            &pale_moss_properties(
                true,
                [
                    ("north", "low"),
                    ("east", "tall"),
                    ("south", "none"),
                    ("west", "none"),
                ],
            ),
        ),
        Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::CARPET,
            BlockOutlineBox::PALE_MOSS_NORTH_LOW,
            BlockOutlineBox::PALE_MOSS_EAST_TALL,
        ]))
    );
}

#[test]
fn outline_shape_uses_vanilla_pale_moss_empty_shape_fallback() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:pale_moss_carpet"),
            &pale_moss_properties(
                false,
                [
                    ("north", "none"),
                    ("east", "none"),
                    ("south", "none"),
                    ("west", "none"),
                ],
            ),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
    );
}

#[test]
fn outline_shape_rejects_invalid_pale_moss_properties() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:pale_moss_carpet"),
            &pale_moss_properties(
                true,
                [
                    ("north", "low"),
                    ("east", "unexpected"),
                    ("south", "none"),
                    ("west", "none"),
                ],
            ),
        ),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:pale_moss_carpet"), &BTreeMap::new()),
        None
    );
}

#[test]
fn selection_outline_uses_slab_bounds() {
    assert_eq!(
        selection_outline_for_box(
            BlockPos { x: -2, y: 63, z: 4 },
            BlockOutlineBox::BOTTOM_SLAB,
        ),
        SelectionOutline::from_box([-2.0, 63.0, 4.0], [-1.0, 63.5, 5.0])
    );
    assert_eq!(
        selection_outline_for_box(BlockPos { x: -2, y: 63, z: 4 }, BlockOutlineBox::TOP_SLAB,),
        SelectionOutline::from_box([-2.0, 63.5, 4.0], [-1.0, 64.0, 5.0])
    );
}

#[test]
fn selection_outline_uses_snow_layer_bounds() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:snow"), &snow_properties(3))
            .unwrap()
            .selection_outline(BlockPos { x: -2, y: 63, z: 4 }),
        SelectionOutline::from_box([-2.0, 63.0, 4.0], [-1.0, 63.375, 5.0])
    );
}

#[test]
fn selection_outline_uses_flat_carpet_bounds() {
    assert_eq!(
        selection_outline_for_box(BlockPos { x: -2, y: 63, z: 4 }, BlockOutlineBox::CARPET),
        SelectionOutline::from_box([-2.0, 63.0, 4.0], [-1.0, 63.0625, 5.0])
    );
}

#[test]
fn selection_outline_preserves_pale_moss_multi_box_shape() {
    let shape = outline_shape_for_block(
        Some("minecraft:pale_moss_carpet"),
        &pale_moss_properties(
            true,
            [
                ("north", "low"),
                ("east", "tall"),
                ("south", "none"),
                ("west", "none"),
            ],
        ),
    )
    .unwrap();

    assert_eq!(
        shape.selection_outline(BlockPos { x: -2, y: 63, z: 4 }),
        SelectionOutline::from_boxes([
            SelectionBox {
                min: [-2.0, 63.0, 4.0],
                max: [-1.0, 63.0625, 5.0],
            },
            SelectionBox {
                min: [-2.0, 63.0, 4.0],
                max: [-1.0, 63.625, 4.0625],
            },
            SelectionBox {
                min: [-1.0625, 63.0, 4.0],
                max: [-1.0, 64.0, 5.0],
            },
        ])
    );
}

#[test]
fn multi_box_outline_clip_uses_nearest_hit() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::PALE_MOSS_EAST_TALL,
            BlockOutlineBox::PALE_MOSS_NORTH_LOW,
        ])),
    };

    assert_eq!(
        target.clip(
            [0.5, 0.5, -1.0],
            [0.0, 0.0, 1.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.0,
            face: ProtocolDirection::North,
            inside: false,
        })
    );
}

#[test]
fn stair_outline_clip_hits_step_face_inside_block() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:oak_stairs"),
            &stair_properties("south", "bottom", "straight"),
        ),
    };

    assert_eq!(
        target.clip(
            [0.5, 0.62, -1.0],
            [0.0, 0.0, 1.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.5,
            face: ProtocolDirection::North,
            inside: false,
        })
    );
}

#[test]
fn trapdoor_outline_clip_uses_thin_closed_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:oak_trapdoor"),
            &trapdoor_properties("north", "top", false),
        ),
    };

    assert_eq!(
        target.clip(
            [0.5, 2.0, 0.5],
            [0.0, -1.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.0,
            face: ProtocolDirection::Up,
            inside: false,
        })
    );
}

#[test]
fn fence_outline_clip_hits_connected_arm_before_post() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:oak_fence"), &fence_properties(["west"])),
    };

    assert_eq!(
        target.clip(
            [-1.0, 0.5, 0.5],
            [1.0, 0.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.0,
            face: ProtocolDirection::West,
            inside: false,
        })
    );
}

#[test]
fn wall_outline_clip_hits_low_side_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:cobblestone_wall"),
            &wall_properties(true, [("north", "low")]),
        ),
    };

    assert_eq!(
        target.clip(
            [0.5, 0.5, -1.0],
            [0.0, 0.0, 1.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.0,
            face: ProtocolDirection::North,
            inside: false,
        })
    );
}

fn slab_properties(slab_type: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("type".to_string(), slab_type.to_string())])
}

fn snow_properties(layers: u8) -> BTreeMap<String, String> {
    BTreeMap::from([("layers".to_string(), layers.to_string())])
}

fn stair_properties(facing: &str, half: &str, shape: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("half".to_string(), half.to_string()),
        ("shape".to_string(), shape.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ])
}

fn trapdoor_properties(facing: &str, half: &str, open: bool) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("half".to_string(), half.to_string()),
        ("open".to_string(), open.to_string()),
        ("powered".to_string(), "false".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ])
}

fn fence_properties<const N: usize>(connected: [&str; N]) -> BTreeMap<String, String> {
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

fn wall_properties<const N: usize>(up: bool, sides: [(&str, &str); N]) -> BTreeMap<String, String> {
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

fn pale_moss_properties(bottom: bool, sides: [(&str, &str); 4]) -> BTreeMap<String, String> {
    let mut properties = BTreeMap::from([("bottom".to_string(), bottom.to_string())]);
    for (name, value) in sides {
        properties.insert(name.to_string(), value.to_string());
    }
    properties
}
