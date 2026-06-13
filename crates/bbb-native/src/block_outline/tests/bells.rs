use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_bell_floor_axis_shapes() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:bell"), &bell_properties("floor", "north")),
        Some(BlockOutlineShape::single(BlockOutlineBox::from_pixels(
            [0.0, 0.0, 4.0],
            [16.0, 16.0, 12.0],
        )))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:bell"), &bell_properties("floor", "east")),
        Some(BlockOutlineShape::single(BlockOutlineBox::from_pixels(
            [4.0, 0.0, 0.0],
            [12.0, 16.0, 16.0],
        )))
    );
}

#[test]
fn outline_shape_uses_vanilla_bell_ceiling_shape() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:bell"), &bell_properties("ceiling", "south")),
        Some(bell_ceiling_shape())
    );
}

#[test]
fn outline_shape_uses_vanilla_bell_single_wall_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:bell"),
            &bell_properties("single_wall", "north")
        ),
        Some(bell_single_wall_shape(HorizontalDirection::North))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:bell"),
            &bell_properties("single_wall", "east")
        ),
        Some(bell_single_wall_shape(HorizontalDirection::East))
    );
}

#[test]
fn outline_shape_uses_vanilla_bell_double_wall_axis_shapes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:bell"),
            &bell_properties("double_wall", "north")
        ),
        Some(bell_double_wall_z_shape())
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:bell"),
            &bell_properties("double_wall", "west")
        ),
        Some(bell_double_wall_x_shape())
    );
}

#[test]
fn outline_shape_rejects_invalid_bell_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:bell"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:bell"), &bell_properties("floor", "up")),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:bell"), &bell_properties("side", "north")),
        None
    );
}

#[test]
fn bell_floor_outline_clip_uses_axis_width() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:bell"),
            &bell_properties("floor", "north"),
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
            distance: 1.25,
            face: ProtocolDirection::North,
            inside: false,
        })
    );
}

#[test]
fn bell_single_wall_outline_clip_uses_wall_support_depth() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:bell"),
            &bell_properties("single_wall", "north"),
        ),
    };

    assert_eq!(
        target.clip(
            [0.5, 14.0 / 16.0, 2.0],
            [0.0, 0.0, -1.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 19.0 / 16.0,
            face: ProtocolDirection::South,
            inside: false,
        })
    );
}

fn bell_ceiling_shape() -> BlockOutlineShape {
    let mut boxes = bell_body_boxes();
    boxes.push(BlockOutlineBox::centered_column(2.0, 2.0, 13.0, 16.0));
    BlockOutlineShape::from_boxes(boxes)
}

fn bell_single_wall_shape(facing: HorizontalDirection) -> BlockOutlineShape {
    let mut boxes = bell_body_boxes();
    boxes.push(
        BlockOutlineBox::from_pixels([7.0, 13.0, 0.0], [9.0, 15.0, 13.0])
            .rotate_to_direction(facing),
    );
    BlockOutlineShape::from_boxes(boxes)
}

fn bell_double_wall_z_shape() -> BlockOutlineShape {
    let mut boxes = bell_body_boxes();
    boxes.push(BlockOutlineBox::from_pixels(
        [7.0, 13.0, 0.0],
        [9.0, 15.0, 16.0],
    ));
    BlockOutlineShape::from_boxes(boxes)
}

fn bell_double_wall_x_shape() -> BlockOutlineShape {
    let mut boxes = bell_body_boxes();
    boxes.push(BlockOutlineBox::from_pixels(
        [0.0, 13.0, 7.0],
        [16.0, 15.0, 9.0],
    ));
    BlockOutlineShape::from_boxes(boxes)
}

fn bell_body_boxes() -> Vec<BlockOutlineBox> {
    vec![
        BlockOutlineBox::centered_column(6.0, 6.0, 6.0, 13.0),
        BlockOutlineBox::centered_column(8.0, 8.0, 4.0, 6.0),
    ]
}

fn bell_properties(attachment: &str, facing: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("attachment".to_string(), attachment.to_string()),
        ("facing".to_string(), facing.to_string()),
        ("powered".to_string(), "false".to_string()),
    ])
}
