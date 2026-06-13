use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_chain_axis_shapes() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:iron_chain"), &chain_properties("x", false)),
        Some(BlockOutlineShape::single(BlockOutlineBox::from_pixels(
            [0.0, 6.5, 6.5],
            [16.0, 9.5, 9.5],
        )))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:iron_chain"), &chain_properties("y", false)),
        Some(BlockOutlineShape::single(BlockOutlineBox::from_pixels(
            [6.5, 0.0, 6.5],
            [9.5, 16.0, 9.5],
        )))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:iron_chain"), &chain_properties("z", false)),
        Some(BlockOutlineShape::single(BlockOutlineBox::from_pixels(
            [6.5, 6.5, 0.0],
            [9.5, 9.5, 16.0],
        )))
    );
}

#[test]
fn outline_shape_uses_vanilla_copper_chain_names() {
    for block_name in [
        "minecraft:copper_chain",
        "minecraft:exposed_copper_chain",
        "minecraft:weathered_copper_chain",
        "minecraft:oxidized_copper_chain",
        "minecraft:waxed_copper_chain",
        "minecraft:waxed_exposed_copper_chain",
        "minecraft:waxed_weathered_copper_chain",
        "minecraft:waxed_oxidized_copper_chain",
    ] {
        assert_eq!(
            outline_shape_for_block(Some(block_name), &chain_properties("y", true)),
            Some(BlockOutlineShape::single(BlockOutlineBox::from_pixels(
                [6.5, 0.0, 6.5],
                [9.5, 16.0, 9.5],
            )))
        );
    }
}

#[test]
fn outline_shape_ignores_chain_waterlogged_property() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:iron_chain"), &chain_properties("z", false)),
        outline_shape_for_block(Some("minecraft:iron_chain"), &chain_properties("z", true))
    );
}

#[test]
fn outline_shape_rejects_invalid_chain_properties_and_non_chain_names() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:iron_chain"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:iron_chain"),
            &chain_properties("north", false)
        ),
        None
    );
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:chain_command_block"),
            &chain_properties("y", false),
        ),
        Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
    );
}

#[test]
fn chain_outline_clip_uses_half_pixel_axis_bounds() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:iron_chain"),
            &chain_properties("y", false),
        ),
    };

    assert_eq!(
        target.clip(
            [-1.0, 0.5, 0.5],
            [1.0, 0.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 45.0 / 32.0,
            face: ProtocolDirection::West,
            inside: false,
        })
    );
}

fn chain_properties(axis: &str, waterlogged: bool) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("axis".to_string(), axis.to_string()),
        ("waterlogged".to_string(), waterlogged.to_string()),
    ])
}
