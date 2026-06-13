use std::collections::BTreeMap;

use super::super::*;
use super::support::*;

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
fn outline_shape_uses_vanilla_pressure_plate_powered_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_pressure_plate"),
            &pressure_plate_properties(false),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::PRESSURE_PLATE))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:stone_pressure_plate"),
            &pressure_plate_properties(true),
        ),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::PRESSURE_PLATE_PRESSED,
        ))
    );
}

#[test]
fn outline_shape_uses_vanilla_weighted_pressure_plate_power_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:light_weighted_pressure_plate"),
            &weighted_pressure_plate_properties(0),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::PRESSURE_PLATE))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:heavy_weighted_pressure_plate"),
            &weighted_pressure_plate_properties(7),
        ),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::PRESSURE_PLATE_PRESSED,
        ))
    );
}

#[test]
fn outline_shape_rejects_invalid_pressure_plate_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_pressure_plate"), &BTreeMap::new()),
        None
    );

    let mut properties = pressure_plate_properties(false);
    properties.insert("powered".to_string(), "sometimes".to_string());
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_pressure_plate"), &properties),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:light_weighted_pressure_plate"),
            &weighted_pressure_plate_properties(16),
        ),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_standing_sign_shape() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_sign"), &standing_sign_properties(0)),
        Some(BlockOutlineShape::single(BlockOutlineBox::SIGN))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:crimson_sign"),
            &standing_sign_properties(12)
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::SIGN))
    );
}

#[test]
fn outline_shape_uses_vanilla_wall_sign_direction_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_wall_sign"),
            &wall_sign_properties("north"),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::WALL_SIGN_NORTH))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:warped_wall_sign"),
            &wall_sign_properties("east"),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::WALL_SIGN_EAST))
    );
}

#[test]
fn outline_shape_rejects_invalid_wall_sign_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_wall_sign"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_wall_sign"), &wall_sign_properties("up"),),
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
fn outline_shape_uses_vanilla_closed_door_direction_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_door"),
            &door_properties("north", "left", false, "lower"),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::DOOR_NORTH))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:iron_door"),
            &door_properties("east", "right", false, "upper"),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::DOOR_EAST))
    );
}

#[test]
fn outline_shape_uses_vanilla_open_door_hinge_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_door"),
            &door_properties("north", "left", true, "lower"),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::DOOR_EAST))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_door"),
            &door_properties("north", "right", true, "upper"),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::DOOR_WEST))
    );
}

#[test]
fn outline_shape_ignores_vanilla_door_half_for_selection_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_door"),
            &door_properties("south", "left", false, "lower"),
        ),
        outline_shape_for_block(
            Some("minecraft:oak_door"),
            &door_properties("south", "left", false, "upper"),
        )
    );
}

#[test]
fn outline_shape_rejects_invalid_door_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_door"), &BTreeMap::new()),
        None
    );

    let mut properties = door_properties("north", "left", true, "lower");
    properties.insert("hinge".to_string(), "middle".to_string());
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_door"), &properties),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_door"),
            &door_properties("up", "left", false, "lower"),
        ),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_ladder_direction_shape() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:ladder"), &ladder_properties("north")),
        Some(BlockOutlineShape::single(BlockOutlineBox::LADDER_NORTH))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:ladder"), &ladder_properties("east")),
        Some(BlockOutlineShape::single(BlockOutlineBox::LADDER_EAST))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:ladder"), &ladder_properties("south")),
        Some(BlockOutlineShape::single(BlockOutlineBox::LADDER_SOUTH))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:ladder"), &ladder_properties("west")),
        Some(BlockOutlineShape::single(BlockOutlineBox::LADDER_WEST))
    );
}

#[test]
fn outline_shape_rejects_invalid_ladder_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:ladder"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:ladder"), &ladder_properties("up")),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_floor_torch_shape() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:torch"), &BTreeMap::new()),
        Some(BlockOutlineShape::single(BlockOutlineBox::TORCH))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:soul_torch"), &BTreeMap::new()),
        Some(BlockOutlineShape::single(BlockOutlineBox::TORCH))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:copper_torch"), &BTreeMap::new()),
        Some(BlockOutlineShape::single(BlockOutlineBox::TORCH))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:redstone_torch"),
            &redstone_torch_properties(false),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::TORCH))
    );
}

#[test]
fn outline_shape_uses_vanilla_wall_torch_direction_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:wall_torch"),
            &wall_torch_properties("north")
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::WALL_TORCH_NORTH))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:soul_wall_torch"),
            &wall_torch_properties("east"),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::WALL_TORCH_EAST))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:copper_wall_torch"),
            &wall_torch_properties("south"),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::WALL_TORCH_SOUTH))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:redstone_wall_torch"),
            &redstone_wall_torch_properties("west", false),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::WALL_TORCH_WEST))
    );
}

#[test]
fn outline_shape_rejects_invalid_wall_torch_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:wall_torch"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:wall_torch"), &wall_torch_properties("up")),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_wall_button_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_button"),
            &button_properties("north", "wall", false),
        ),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::BUTTON_WALL_NORTH
        ))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:stone_button"),
            &button_properties("east", "wall", true),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox {
            min: [0.0, 6.0 / 16.0, 5.0 / 16.0],
            max: [1.0 / 16.0, 10.0 / 16.0, 11.0 / 16.0],
        }))
    );
}

#[test]
fn outline_shape_uses_vanilla_floor_and_ceiling_button_shapes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:birch_button"),
            &button_properties("north", "floor", false),
        ),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::BUTTON_FLOOR_NORTH
        ))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:polished_blackstone_button"),
            &button_properties("east", "floor", true),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox {
            min: [6.0 / 16.0, 15.0 / 16.0, 5.0 / 16.0],
            max: [10.0 / 16.0, 1.0, 11.0 / 16.0],
        }))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:spruce_button"),
            &button_properties("west", "ceiling", false),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox {
            min: [6.0 / 16.0, 0.0, 5.0 / 16.0],
            max: [10.0 / 16.0, 2.0 / 16.0, 11.0 / 16.0],
        }))
    );
}

#[test]
fn outline_shape_rejects_invalid_button_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_button"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_button"),
            &button_properties("up", "wall", false),
        ),
        None
    );

    let mut properties = button_properties("north", "side", false);
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_button"), &properties),
        None
    );
    properties.insert("face".to_string(), "wall".to_string());
    properties.insert("powered".to_string(), "sometimes".to_string());
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_button"), &properties),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_lever_attach_face_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:lever"),
            &lever_properties("north", "wall", false)
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::LEVER_WALL_NORTH))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:lever"),
            &lever_properties("east", "wall", true)
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox {
            min: [0.0, 4.0 / 16.0, 5.0 / 16.0],
            max: [6.0 / 16.0, 12.0 / 16.0, 11.0 / 16.0],
        }))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:lever"),
            &lever_properties("south", "floor", true)
        ),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::LEVER_FLOOR_NORTH
        ))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:lever"),
            &lever_properties("west", "ceiling", false),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox {
            min: [4.0 / 16.0, 0.0, 5.0 / 16.0],
            max: [12.0 / 16.0, 6.0 / 16.0, 11.0 / 16.0],
        }))
    );
}

#[test]
fn outline_shape_rejects_invalid_lever_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:lever"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:lever"),
            &lever_properties("up", "wall", false)
        ),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:lever"),
            &lever_properties("north", "side", false)
        ),
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
fn outline_shape_uses_vanilla_fence_gate_axis_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_fence_gate"),
            &fence_gate_properties("north", false, false),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::FENCE_GATE_Z))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:birch_fence_gate"),
            &fence_gate_properties("east", false, false),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::FENCE_GATE_X))
    );
}

#[test]
fn outline_shape_uses_vanilla_fence_gate_in_wall_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:acacia_fence_gate"),
            &fence_gate_properties("south", true, false),
        ),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::FENCE_GATE_Z_IN_WALL
        ))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:dark_oak_fence_gate"),
            &fence_gate_properties("west", true, false),
        ),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::FENCE_GATE_X_IN_WALL
        ))
    );
}

#[test]
fn outline_shape_ignores_vanilla_fence_gate_open_for_selection_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_fence_gate"),
            &fence_gate_properties("north", false, true),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::FENCE_GATE_Z))
    );
}

#[test]
fn outline_shape_rejects_invalid_fence_gate_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_fence_gate"), &BTreeMap::new()),
        None
    );

    let mut properties = fence_gate_properties("north", false, false);
    properties.insert("in_wall".to_string(), "sometimes".to_string());
    assert_eq!(
        outline_shape_for_block(Some("minecraft:oak_fence_gate"), &properties),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:oak_fence_gate"),
            &fence_gate_properties("up", false, false),
        ),
        None
    );
}

#[test]
fn outline_shape_uses_vanilla_disconnected_pane_post() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:glass_pane"), &pane_properties([])),
        Some(BlockOutlineShape::single(BlockOutlineBox::PANE_POST))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:iron_bars"), &pane_properties([])),
        Some(BlockOutlineShape::single(BlockOutlineBox::PANE_POST))
    );
}

#[test]
fn outline_shape_uses_vanilla_pane_connection_boxes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:white_stained_glass_pane"),
            &pane_properties(["south", "west"]),
        ),
        Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::PANE_POST,
            BlockOutlineBox::PANE_SOUTH_ARM,
            BlockOutlineBox::PANE_WEST_ARM,
        ]))
    );
}

#[test]
fn outline_shape_rejects_invalid_pane_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:glass_pane"), &BTreeMap::new()),
        None
    );

    let mut properties = pane_properties(["north"]);
    properties.insert("north".to_string(), "sometimes".to_string());
    assert_eq!(
        outline_shape_for_block(Some("minecraft:iron_bars"), &properties),
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
