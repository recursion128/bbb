use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_cauldron_shell_shape() {
    for block_name in [
        "minecraft:cauldron",
        "minecraft:water_cauldron",
        "minecraft:lava_cauldron",
        "minecraft:powder_snow_cauldron",
    ] {
        assert_eq!(
            outline_shape_for_block(Some(block_name), &cauldron_properties(3)),
            Some(expected_cauldron_shape())
        );
    }
}

#[test]
fn cauldron_outline_clip_hits_outer_wall() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:cauldron"), &BTreeMap::new()),
    };

    assert_eq!(
        target.clip(
            [-1.0, 0.5, 0.5],
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

#[test]
fn cauldron_outline_clip_hits_bottom_through_hollow_center() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:water_cauldron"), &cauldron_properties(2)),
    };

    assert_eq!(
        target.clip(
            [0.5, 2.0, 0.5],
            [0.0, -1.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.75,
            face: ProtocolDirection::Up,
            inside: false,
        })
    );
}

fn expected_cauldron_shape() -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(vec![
        BlockOutlineBox::from_pixels([0.0, 0.0, 0.0], [2.0, 3.0, 4.0]),
        BlockOutlineBox::from_pixels([0.0, 0.0, 12.0], [2.0, 3.0, 16.0]),
        BlockOutlineBox::from_pixels([14.0, 0.0, 0.0], [16.0, 3.0, 4.0]),
        BlockOutlineBox::from_pixels([14.0, 0.0, 12.0], [16.0, 3.0, 16.0]),
        BlockOutlineBox::from_pixels([2.0, 0.0, 0.0], [4.0, 3.0, 2.0]),
        BlockOutlineBox::from_pixels([2.0, 0.0, 14.0], [4.0, 3.0, 16.0]),
        BlockOutlineBox::from_pixels([12.0, 0.0, 0.0], [14.0, 3.0, 2.0]),
        BlockOutlineBox::from_pixels([12.0, 0.0, 14.0], [14.0, 3.0, 16.0]),
        BlockOutlineBox::from_pixels([0.0, 3.0, 0.0], [16.0, 4.0, 16.0]),
        BlockOutlineBox::from_pixels([0.0, 4.0, 0.0], [2.0, 16.0, 16.0]),
        BlockOutlineBox::from_pixels([14.0, 4.0, 0.0], [16.0, 16.0, 16.0]),
        BlockOutlineBox::from_pixels([2.0, 4.0, 0.0], [14.0, 16.0, 2.0]),
        BlockOutlineBox::from_pixels([2.0, 4.0, 14.0], [14.0, 16.0, 16.0]),
    ])
}

fn cauldron_properties(level: u8) -> BTreeMap<String, String> {
    BTreeMap::from([("level".to_string(), level.to_string())])
}
