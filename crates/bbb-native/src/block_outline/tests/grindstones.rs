use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_grindstone_wall_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:grindstone"),
            &grindstone_properties("wall", "north")
        ),
        Some(grindstone_wall_shape(HorizontalDirection::North))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:grindstone"),
            &grindstone_properties("wall", "east")
        ),
        Some(grindstone_wall_shape(HorizontalDirection::East))
    );
}

#[test]
fn outline_shape_uses_vanilla_grindstone_floor_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:grindstone"),
            &grindstone_properties("floor", "north")
        ),
        Some(grindstone_floor_shape(HorizontalDirection::North))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:grindstone"),
            &grindstone_properties("floor", "west")
        ),
        Some(grindstone_floor_shape(HorizontalDirection::West))
    );
}

#[test]
fn outline_shape_uses_vanilla_grindstone_ceiling_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:grindstone"),
            &grindstone_properties("ceiling", "north")
        ),
        Some(grindstone_ceiling_shape(HorizontalDirection::North))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:grindstone"),
            &grindstone_properties("ceiling", "south")
        ),
        Some(grindstone_ceiling_shape(HorizontalDirection::South))
    );
}

#[test]
fn outline_shape_rejects_invalid_grindstone_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:grindstone"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:grindstone"),
            &grindstone_properties("wall", "up")
        ),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:grindstone"),
            &grindstone_properties("side", "north")
        ),
        None
    );
}

#[test]
fn grindstone_wall_outline_clip_uses_wheel_depth() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:grindstone"),
            &grindstone_properties("wall", "north"),
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
            distance: 1.25,
            face: ProtocolDirection::South,
            inside: false,
        })
    );
}

#[test]
fn grindstone_floor_outline_clip_uses_rotated_wheel_bottom() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:grindstone"),
            &grindstone_properties("floor", "north"),
        ),
    };

    assert_eq!(
        target.clip(
            [0.5, -1.0, 0.5],
            [0.0, 1.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.25,
            face: ProtocolDirection::Down,
            inside: false,
        })
    );
}

fn grindstone_wall_shape(facing: HorizontalDirection) -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(
        grindstone_north_wall_boxes()
            .into_iter()
            .map(|outline| outline.rotate_to_direction(facing))
            .collect(),
    )
}

fn grindstone_floor_shape(facing: HorizontalDirection) -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(
        grindstone_north_wall_boxes()
            .into_iter()
            .map(rotate_grindstone_to_floor)
            .map(|outline| outline.rotate_to_direction(facing))
            .collect(),
    )
}

fn grindstone_ceiling_shape(facing: HorizontalDirection) -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(
        grindstone_north_wall_boxes()
            .into_iter()
            .map(rotate_grindstone_to_ceiling)
            .map(|outline| outline.rotate_to_direction(facing))
            .collect(),
    )
}

fn grindstone_north_wall_boxes() -> Vec<BlockOutlineBox> {
    vec![
        BlockOutlineBox::from_pixels([4.0, 2.0, 0.0], [12.0, 14.0, 12.0]),
        BlockOutlineBox::from_pixels([2.0, 6.0, 7.0], [4.0, 10.0, 16.0]),
        BlockOutlineBox::from_pixels([2.0, 5.0, 3.0], [4.0, 11.0, 9.0]),
        BlockOutlineBox::from_pixels([12.0, 6.0, 7.0], [14.0, 10.0, 16.0]),
        BlockOutlineBox::from_pixels([12.0, 5.0, 3.0], [14.0, 11.0, 9.0]),
    ]
}

fn rotate_grindstone_to_floor(outline: BlockOutlineBox) -> BlockOutlineBox {
    BlockOutlineBox {
        min: [outline.min[0], 1.0 - outline.max[2], outline.min[1]],
        max: [outline.max[0], 1.0 - outline.min[2], outline.max[1]],
    }
}

fn rotate_grindstone_to_ceiling(outline: BlockOutlineBox) -> BlockOutlineBox {
    BlockOutlineBox {
        min: [1.0 - outline.max[0], outline.min[2], outline.min[1]],
        max: [1.0 - outline.min[0], outline.max[2], outline.max[1]],
    }
}

fn grindstone_properties(face: &str, facing: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("face".to_string(), face.to_string()),
        ("facing".to_string(), facing.to_string()),
    ])
}
