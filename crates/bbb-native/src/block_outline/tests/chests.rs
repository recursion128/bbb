use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_single_chest_shape() {
    for block_name in [
        "minecraft:chest",
        "minecraft:trapped_chest",
        "minecraft:ender_chest",
    ] {
        let properties = if block_name == "minecraft:ender_chest" {
            BTreeMap::new()
        } else {
            chest_properties("north", "single")
        };
        assert_eq!(
            outline_shape_for_block(Some(block_name), &properties),
            Some(BlockOutlineShape::single(BlockOutlineBox::CHEST_SINGLE))
        );
    }
}

#[test]
fn outline_shape_uses_vanilla_double_chest_connected_direction() {
    for (facing, chest_type, expected) in [
        ("north", "left", BlockOutlineBox::CHEST_CONNECTED_EAST),
        ("north", "right", BlockOutlineBox::CHEST_CONNECTED_WEST),
        ("west", "left", BlockOutlineBox::CHEST_CONNECTED_NORTH),
        ("east", "right", BlockOutlineBox::CHEST_CONNECTED_NORTH),
        ("south", "left", BlockOutlineBox::CHEST_CONNECTED_WEST),
        ("south", "right", BlockOutlineBox::CHEST_CONNECTED_EAST),
    ] {
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:chest"),
                &chest_properties(facing, chest_type)
            ),
            Some(BlockOutlineShape::single(expected))
        );
    }
}

#[test]
fn outline_shape_rejects_invalid_chest_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:chest"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:chest"),
            &chest_properties("north", "unexpected")
        ),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:trapped_chest"),
            &BTreeMap::from([("type".to_string(), "left".to_string())]),
        ),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:chest"), &chest_properties("up", "left")),
        None
    );
}

#[test]
fn single_chest_outline_clip_uses_vanilla_height() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:chest"),
            &chest_properties("north", "single"),
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
fn double_chest_outline_clip_uses_connected_half_shape() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:chest"),
            &chest_properties("north", "left"),
        ),
    };

    assert_eq!(
        target.clip(
            [2.0, 0.5, 0.5],
            [-1.0, 0.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.0,
            face: ProtocolDirection::East,
            inside: false,
        })
    );
}

fn chest_properties(facing: &str, chest_type: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("type".to_string(), chest_type.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ])
}
