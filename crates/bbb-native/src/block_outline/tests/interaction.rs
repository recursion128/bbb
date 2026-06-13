use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_renderer::{SelectionBox, SelectionOutline};
use bbb_world::{BlockPos, TerrainMaterialClass};
use std::collections::BTreeMap;

use super::super::*;
use super::support::*;

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
fn pressure_plate_outline_clip_uses_thin_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:oak_pressure_plate"),
            &pressure_plate_properties(false),
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
            distance: 1.9375,
            face: ProtocolDirection::Up,
            inside: false,
        })
    );
}

#[test]
fn flower_pot_outline_clip_uses_small_column_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:potted_oak_sapling"), &BTreeMap::new()),
    };

    assert_eq!(
        target.clip(
            [-1.0, 0.25, 0.5],
            [1.0, 0.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.3125,
            face: ProtocolDirection::West,
            inside: false,
        })
    );
}

#[test]
fn wall_sign_outline_clip_uses_thin_direction_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:oak_wall_sign"),
            &wall_sign_properties("south"),
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
fn door_outline_clip_uses_thin_direction_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:oak_door"),
            &door_properties("north", "left", false, "lower"),
        ),
    };

    assert_eq!(
        target.clip(
            [0.5, 0.5, 2.0],
            [0.0, 0.0, -1.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.0,
            face: ProtocolDirection::South,
            inside: false,
        })
    );
}

#[test]
fn ladder_outline_clip_uses_thin_direction_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:ladder"), &ladder_properties("south")),
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
fn floor_torch_outline_clip_uses_center_column_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:torch"), &BTreeMap::new()),
    };

    assert_eq!(
        target.clip(
            [-1.0, 0.5, 0.5],
            [1.0, 0.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.375,
            face: ProtocolDirection::West,
            inside: false,
        })
    );
}

#[test]
fn wall_torch_outline_clip_uses_direction_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:wall_torch"),
            &wall_torch_properties("south"),
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

#[test]
fn button_outline_clip_uses_pressed_wall_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:oak_button"),
            &button_properties("south", "wall", true),
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

#[test]
fn lever_outline_clip_uses_wall_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:lever"),
            &lever_properties("south", "wall", true),
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
fn fence_gate_outline_clip_uses_shape_even_when_open() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:oak_fence_gate"),
            &fence_gate_properties("north", false, true),
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
            distance: 1.375,
            face: ProtocolDirection::North,
            inside: false,
        })
    );
}

#[test]
fn pane_outline_clip_hits_connected_arm_before_post() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:glass_pane"), &pane_properties(["north"])),
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
