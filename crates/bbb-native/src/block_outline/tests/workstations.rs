use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_brewing_stand_shape() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:brewing_stand"), &brewing_stand_properties()),
        Some(BlockOutlineShape::from_boxes(vec![
            BlockOutlineBox::BREWING_STAND_ROD,
            BlockOutlineBox::BREWING_STAND_BASE,
        ]))
    );
}

#[test]
fn brewing_stand_outline_clip_hits_center_rod() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:brewing_stand"),
            &brewing_stand_properties(),
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

#[test]
fn brewing_stand_outline_clip_hits_base_width() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:brewing_stand"),
            &brewing_stand_properties(),
        ),
    };

    assert_eq!(
        target.clip(
            [-1.0, 0.0625, 0.5],
            [1.0, 0.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.0625,
            face: ProtocolDirection::West,
            inside: false,
        })
    );
}

#[test]
fn outline_shape_uses_vanilla_anvil_z_axis_shape() {
    for block_name in [
        "minecraft:anvil",
        "minecraft:chipped_anvil",
        "minecraft:damaged_anvil",
    ] {
        assert_eq!(
            outline_shape_for_block(Some(block_name), &anvil_properties("north")),
            Some(anvil_z_axis_shape())
        );
        assert_eq!(
            outline_shape_for_block(Some(block_name), &anvil_properties("south")),
            Some(anvil_z_axis_shape())
        );
    }
}

#[test]
fn outline_shape_uses_vanilla_anvil_x_axis_shape() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:anvil"), &anvil_properties("east")),
        Some(anvil_x_axis_shape())
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:chipped_anvil"), &anvil_properties("west")),
        Some(anvil_x_axis_shape())
    );
}

#[test]
fn outline_shape_rejects_invalid_anvil_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:anvil"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:anvil"), &anvil_properties("up")),
        None
    );
}

#[test]
fn anvil_outline_clip_hits_top_slab() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:anvil"), &anvil_properties("north")),
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
fn anvil_outline_clip_uses_axis_rotated_top_width() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:anvil"), &anvil_properties("east")),
    };

    assert_eq!(
        target.clip(
            [-1.0, 0.75, 0.5],
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

fn anvil_z_axis_shape() -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(vec![
        BlockOutlineBox::centered_column(12.0, 12.0, 0.0, 4.0),
        BlockOutlineBox::centered_column(8.0, 10.0, 4.0, 5.0),
        BlockOutlineBox::centered_column(4.0, 8.0, 5.0, 10.0),
        BlockOutlineBox::centered_column(10.0, 16.0, 10.0, 16.0),
    ])
}

fn anvil_x_axis_shape() -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(vec![
        BlockOutlineBox::centered_column(12.0, 12.0, 0.0, 4.0),
        BlockOutlineBox::centered_column(10.0, 8.0, 4.0, 5.0),
        BlockOutlineBox::centered_column(8.0, 4.0, 5.0, 10.0),
        BlockOutlineBox::centered_column(16.0, 10.0, 10.0, 16.0),
    ])
}

fn anvil_properties(facing: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("facing".to_string(), facing.to_string())])
}

fn brewing_stand_properties() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("has_bottle_0".to_string(), "false".to_string()),
        ("has_bottle_1".to_string(), "false".to_string()),
        ("has_bottle_2".to_string(), "false".to_string()),
    ])
}
