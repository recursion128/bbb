use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_hopper_down_shape() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:hopper"), &hopper_properties("down", true)),
        Some(hopper_shape(BlockOutlineBox::from_pixels(
            [6.0, 0.0, 6.0],
            [10.0, 6.0, 10.0],
        )))
    );
}

#[test]
fn outline_shape_uses_vanilla_hopper_horizontal_spouts() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:hopper"), &hopper_properties("north", true)),
        Some(hopper_shape(BlockOutlineBox::from_pixels(
            [6.0, 4.0, 0.0],
            [10.0, 8.0, 8.0],
        )))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:hopper"), &hopper_properties("east", true)),
        Some(hopper_shape(BlockOutlineBox::from_pixels(
            [8.0, 4.0, 6.0],
            [16.0, 8.0, 10.0],
        )))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:hopper"), &hopper_properties("south", true)),
        Some(hopper_shape(BlockOutlineBox::from_pixels(
            [6.0, 4.0, 8.0],
            [10.0, 8.0, 16.0],
        )))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:hopper"), &hopper_properties("west", true)),
        Some(hopper_shape(BlockOutlineBox::from_pixels(
            [0.0, 4.0, 6.0],
            [8.0, 8.0, 10.0],
        )))
    );
}

#[test]
fn outline_shape_ignores_hopper_enabled_property() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:hopper"), &hopper_properties("north", true)),
        outline_shape_for_block(Some("minecraft:hopper"), &hopper_properties("north", false))
    );
}

#[test]
fn outline_shape_rejects_invalid_hopper_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:hopper"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:hopper"), &hopper_properties("up", true)),
        None
    );
}

#[test]
fn hopper_outline_clip_hits_horizontal_spout_bottom() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:hopper"),
            &hopper_properties("east", true),
        ),
    };

    assert_eq!(
        target.clip(
            [14.0 / 16.0, -1.0, 0.5],
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

#[test]
fn hopper_outline_clip_hits_inner_bottom_through_top_hole() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:hopper"),
            &hopper_properties("down", true),
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
            distance: 21.0 / 16.0,
            face: ProtocolDirection::Up,
            inside: false,
        })
    );
}

fn hopper_shape(spout: BlockOutlineBox) -> BlockOutlineShape {
    let mut boxes = spoutless_hopper_boxes();
    boxes.push(spout);
    BlockOutlineShape::from_boxes(boxes)
}

fn spoutless_hopper_boxes() -> Vec<BlockOutlineBox> {
    vec![
        BlockOutlineBox::from_pixels([0.0, 10.0, 0.0], [16.0, 11.0, 16.0]),
        BlockOutlineBox::from_pixels([0.0, 11.0, 0.0], [16.0, 16.0, 2.0]),
        BlockOutlineBox::from_pixels([0.0, 11.0, 14.0], [16.0, 16.0, 16.0]),
        BlockOutlineBox::from_pixels([0.0, 11.0, 2.0], [2.0, 16.0, 14.0]),
        BlockOutlineBox::from_pixels([14.0, 11.0, 2.0], [16.0, 16.0, 14.0]),
        BlockOutlineBox::centered_column(8.0, 8.0, 4.0, 10.0),
    ]
}

fn hopper_properties(facing: &str, enabled: bool) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("enabled".to_string(), enabled.to_string()),
    ])
}
