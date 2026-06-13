use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_cake_bite_shapes() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:cake"), &cake_properties(0)),
        Some(cake_shape_for_bites(0))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:cake"), &cake_properties(3)),
        Some(cake_shape_for_bites(3))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:cake"), &cake_properties(6)),
        Some(cake_shape_for_bites(6))
    );
}

#[test]
fn outline_shape_rejects_invalid_cake_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:cake"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:cake"), &cake_properties(7)),
        None
    );
}

#[test]
fn cake_outline_clip_uses_bite_shortened_west_edge() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:cake"), &cake_properties(3)),
    };

    assert_eq!(
        target.clip(
            [-1.0, 0.25, 0.5],
            [1.0, 0.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.4375,
            face: ProtocolDirection::West,
            inside: false,
        })
    );
}

#[test]
fn outline_shape_uses_vanilla_candle_cake_shape() {
    for block_name in [
        "minecraft:candle_cake",
        "minecraft:white_candle_cake",
        "minecraft:black_candle_cake",
    ] {
        assert_eq!(
            outline_shape_for_block(Some(block_name), &candle_cake_properties(false)),
            Some(candle_cake_shape())
        );
    }
}

#[test]
fn candle_cake_outline_clip_hits_candle_above_cake() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:white_candle_cake"),
            &candle_cake_properties(true),
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
            distance: 1.125,
            face: ProtocolDirection::Up,
            inside: false,
        })
    );
}

fn cake_shape_for_bites(bites: u8) -> BlockOutlineShape {
    BlockOutlineShape::single(BlockOutlineBox::from_pixels(
        [1.0 + f64::from(bites) * 2.0, 0.0, 1.0],
        [15.0, 8.0, 15.0],
    ))
}

fn candle_cake_shape() -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(vec![
        BlockOutlineBox::centered_column(2.0, 2.0, 8.0, 14.0),
        BlockOutlineBox::centered_column(14.0, 14.0, 0.0, 8.0),
    ])
}

fn cake_properties(bites: u8) -> BTreeMap<String, String> {
    BTreeMap::from([("bites".to_string(), bites.to_string())])
}

fn candle_cake_properties(lit: bool) -> BTreeMap<String, String> {
    BTreeMap::from([("lit".to_string(), lit.to_string())])
}
