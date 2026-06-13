use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_bed_foot_shape() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:red_bed"), &bed_properties("north", "foot")),
        Some(bed_shape_with_south_legs())
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:blue_bed"), &bed_properties("east", "foot")),
        Some(bed_shape_with_west_legs())
    );
}

#[test]
fn outline_shape_uses_vanilla_bed_head_shape() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:red_bed"), &bed_properties("north", "head")),
        Some(bed_shape_with_north_legs())
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:yellow_bed"),
            &bed_properties("east", "head")
        ),
        Some(bed_shape_with_east_legs())
    );
}

#[test]
fn outline_shape_rejects_invalid_bed_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:red_bed"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:red_bed"),
            &bed_properties("north", "middle")
        ),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:red_bed"), &bed_properties("up", "foot")),
        None
    );
}

#[test]
fn bed_outline_clip_uses_vanilla_platform_height() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:red_bed"),
            &bed_properties("north", "foot"),
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
            distance: 1.4375,
            face: ProtocolDirection::Up,
            inside: false,
        })
    );
}

fn bed_shape_with_north_legs() -> BlockOutlineShape {
    bed_shape([
        BlockOutlineBox::from_pixels([0.0, 0.0, 0.0], [3.0, 3.0, 3.0]),
        BlockOutlineBox::from_pixels([13.0, 0.0, 0.0], [16.0, 3.0, 3.0]),
    ])
}

fn bed_shape_with_east_legs() -> BlockOutlineShape {
    bed_shape([
        BlockOutlineBox::from_pixels([13.0, 0.0, 0.0], [16.0, 3.0, 3.0]),
        BlockOutlineBox::from_pixels([13.0, 0.0, 13.0], [16.0, 3.0, 16.0]),
    ])
}

fn bed_shape_with_south_legs() -> BlockOutlineShape {
    bed_shape([
        BlockOutlineBox::from_pixels([13.0, 0.0, 13.0], [16.0, 3.0, 16.0]),
        BlockOutlineBox::from_pixels([0.0, 0.0, 13.0], [3.0, 3.0, 16.0]),
    ])
}

fn bed_shape_with_west_legs() -> BlockOutlineShape {
    bed_shape([
        BlockOutlineBox::from_pixels([0.0, 0.0, 13.0], [3.0, 3.0, 16.0]),
        BlockOutlineBox::from_pixels([0.0, 0.0, 0.0], [3.0, 3.0, 3.0]),
    ])
}

fn bed_shape(legs: [BlockOutlineBox; 2]) -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(vec![BlockOutlineBox::BED_PLATFORM, legs[0], legs[1]])
}

fn bed_properties(facing: &str, part: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("part".to_string(), part.to_string()),
        ("occupied".to_string(), "false".to_string()),
    ])
}
