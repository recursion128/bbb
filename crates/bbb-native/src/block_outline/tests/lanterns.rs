use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_lantern_standing_shape() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:lantern"), &lantern_properties(false, false)),
        Some(lantern_shape(false))
    );
}

#[test]
fn outline_shape_uses_vanilla_lantern_hanging_shape() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:soul_lantern"),
            &lantern_properties(true, false),
        ),
        Some(lantern_shape(true))
    );
}

#[test]
fn outline_shape_uses_vanilla_copper_lantern_names() {
    for block_name in [
        "minecraft:copper_lantern",
        "minecraft:exposed_copper_lantern",
        "minecraft:weathered_copper_lantern",
        "minecraft:oxidized_copper_lantern",
        "minecraft:waxed_copper_lantern",
        "minecraft:waxed_exposed_copper_lantern",
        "minecraft:waxed_weathered_copper_lantern",
        "minecraft:waxed_oxidized_copper_lantern",
    ] {
        assert_eq!(
            outline_shape_for_block(Some(block_name), &lantern_properties(false, true)),
            Some(lantern_shape(false))
        );
    }
}

#[test]
fn outline_shape_rejects_invalid_lantern_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:lantern"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:lantern"),
            &lantern_properties_value("maybe")
        ),
        None
    );
}

#[test]
fn lantern_outline_clip_uses_narrow_body_width() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:lantern"),
            &lantern_properties(false, false),
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
            distance: 21.0 / 16.0,
            face: ProtocolDirection::West,
            inside: false,
        })
    );
}

#[test]
fn hanging_lantern_outline_clip_uses_one_pixel_vertical_offset() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:soul_lantern"),
            &lantern_properties(true, false),
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
            distance: 17.0 / 16.0,
            face: ProtocolDirection::Down,
            inside: false,
        })
    );
}

fn lantern_shape(hanging: bool) -> BlockOutlineShape {
    let offset = if hanging { 1.0 } else { 0.0 };
    BlockOutlineShape::from_boxes(vec![
        BlockOutlineBox::centered_column(4.0, 4.0, 7.0 + offset, 9.0 + offset),
        BlockOutlineBox::centered_column(6.0, 6.0, offset, 7.0 + offset),
    ])
}

fn lantern_properties(hanging: bool, waterlogged: bool) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("hanging".to_string(), hanging.to_string()),
        ("waterlogged".to_string(), waterlogged.to_string()),
    ])
}

fn lantern_properties_value(hanging: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("hanging".to_string(), hanging.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ])
}
