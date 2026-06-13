use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_flat_rail_shape() {
    for (block_name, shape) in [
        ("minecraft:rail", "north_south"),
        ("minecraft:rail", "north_east"),
        ("minecraft:powered_rail", "east_west"),
        ("minecraft:detector_rail", "north_south"),
        ("minecraft:activator_rail", "east_west"),
    ] {
        assert_eq!(
            outline_shape_for_block(Some(block_name), &rail_properties(shape)),
            Some(BlockOutlineShape::single(BlockOutlineBox::RAIL_FLAT))
        );
    }
}

#[test]
fn outline_shape_uses_vanilla_sloped_rail_shape() {
    for shape in [
        "ascending_north",
        "ascending_east",
        "ascending_south",
        "ascending_west",
    ] {
        assert_eq!(
            outline_shape_for_block(Some("minecraft:powered_rail"), &rail_properties(shape)),
            Some(BlockOutlineShape::single(BlockOutlineBox::RAIL_SLOPE))
        );
    }
}

#[test]
fn outline_shape_rejects_invalid_rail_shape_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:rail"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:rail"), &rail_properties("unexpected")),
        None
    );
}

#[test]
fn flat_rail_outline_clip_uses_two_pixel_height() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:rail"), &rail_properties("north_south")),
    };

    assert_eq!(
        target.clip(
            [0.5, 2.0, 0.5],
            [0.0, -1.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.875,
            face: ProtocolDirection::Up,
            inside: false,
        })
    );
}

#[test]
fn sloped_rail_outline_clip_uses_eight_pixel_height() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:activator_rail"),
            &rail_properties("ascending_east"),
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
            distance: 1.5,
            face: ProtocolDirection::Up,
            inside: false,
        })
    );
}

fn rail_properties(shape: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("shape".to_string(), shape.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ])
}
