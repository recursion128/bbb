use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_candle_count_shapes() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:candle"),
            &candle_properties(1, false, false)
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::centered_column(
            2.0, 2.0, 0.0, 6.0,
        )))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:candle"),
            &candle_properties(2, false, false)
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::from_pixels(
            [5.0, 0.0, 6.0],
            [11.0, 6.0, 9.0],
        )))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:candle"),
            &candle_properties(3, false, false)
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::from_pixels(
            [5.0, 0.0, 6.0],
            [10.0, 6.0, 11.0],
        )))
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:candle"),
            &candle_properties(4, false, false)
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::from_pixels(
            [5.0, 0.0, 5.0],
            [11.0, 6.0, 10.0],
        )))
    );
}

#[test]
fn outline_shape_uses_vanilla_colored_candle_names() {
    for block_name in [
        "minecraft:white_candle",
        "minecraft:light_blue_candle",
        "minecraft:black_candle",
    ] {
        assert_eq!(
            outline_shape_for_block(Some(block_name), &candle_properties(1, true, true)),
            Some(BlockOutlineShape::single(BlockOutlineBox::centered_column(
                2.0, 2.0, 0.0, 6.0,
            )))
        );
    }
}

#[test]
fn outline_shape_ignores_candle_lit_and_waterlogged_properties() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:candle"),
            &candle_properties(3, false, false)
        ),
        outline_shape_for_block(Some("minecraft:candle"), &candle_properties(3, true, true))
    );
}

#[test]
fn outline_shape_rejects_invalid_candle_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:candle"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:candle"),
            &candle_properties(0, false, false)
        ),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:candle"),
            &candle_properties(5, false, false)
        ),
        None
    );
}

#[test]
fn candle_outline_clip_uses_single_candle_column_width() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:white_candle"),
            &candle_properties(1, false, false),
        ),
    };

    assert_eq!(
        target.clip(
            [-1.0, 0.25, 0.5],
            [1.0, 0.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 23.0 / 16.0,
            face: ProtocolDirection::West,
            inside: false,
        })
    );
}

fn candle_properties(candles: u8, lit: bool, waterlogged: bool) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("candles".to_string(), candles.to_string()),
        ("lit".to_string(), lit.to_string()),
        ("waterlogged".to_string(), waterlogged.to_string()),
    ])
}
