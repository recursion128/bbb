use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_composter_level_shapes() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:composter"), &composter_properties(0)),
        Some(composter_shape(2.0))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:composter"), &composter_properties(1)),
        Some(composter_shape(3.0))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:composter"), &composter_properties(7)),
        Some(composter_shape(15.0))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:composter"), &composter_properties(8)),
        Some(composter_shape(15.0))
    );
}

#[test]
fn outline_shape_rejects_invalid_composter_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:composter"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:composter"), &composter_properties(9)),
        None
    );
}

#[test]
fn composter_outline_clip_hits_bottom_through_center_hole() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:composter"), &composter_properties(0)),
    };

    assert_eq!(
        target.clip(
            [0.5, 2.0, 0.5],
            [0.0, -1.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 15.0 / 8.0,
            face: ProtocolDirection::Up,
            inside: false,
        })
    );
}

#[test]
fn composter_outline_clip_hits_outer_wall() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:composter"), &composter_properties(3)),
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

fn composter_shape(hole_y: f64) -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(vec![
        BlockOutlineBox::from_pixels([0.0, 0.0, 0.0], [16.0, hole_y, 16.0]),
        BlockOutlineBox::from_pixels([0.0, hole_y, 0.0], [16.0, 16.0, 2.0]),
        BlockOutlineBox::from_pixels([0.0, hole_y, 14.0], [16.0, 16.0, 16.0]),
        BlockOutlineBox::from_pixels([0.0, hole_y, 2.0], [2.0, 16.0, 14.0]),
        BlockOutlineBox::from_pixels([14.0, hole_y, 2.0], [16.0, 16.0, 14.0]),
    ])
}

fn composter_properties(level: u8) -> BTreeMap<String, String> {
    BTreeMap::from([("level".to_string(), level.to_string())])
}
