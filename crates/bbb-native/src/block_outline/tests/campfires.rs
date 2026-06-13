use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_campfire_shape() {
    for block_name in ["minecraft:campfire", "minecraft:soul_campfire"] {
        assert_eq!(
            outline_shape_for_block(Some(block_name), &campfire_properties("north", true)),
            Some(campfire_shape())
        );
    }
}

#[test]
fn outline_shape_ignores_campfire_state_properties() {
    assert_eq!(
        outline_shape_for_block(
            Some("minecraft:campfire"),
            &campfire_properties("north", true)
        ),
        outline_shape_for_block(
            Some("minecraft:campfire"),
            &campfire_properties("east", false)
        )
    );
}

#[test]
fn campfire_outline_clip_uses_seven_pixel_height() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(
            Some("minecraft:soul_campfire"),
            &campfire_properties("south", true),
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
            distance: 25.0 / 16.0,
            face: ProtocolDirection::Up,
            inside: false,
        })
    );
}

fn campfire_shape() -> BlockOutlineShape {
    BlockOutlineShape::single(BlockOutlineBox::centered_column(16.0, 16.0, 0.0, 7.0))
}

fn campfire_properties(facing: &str, lit: bool) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("lit".to_string(), lit.to_string()),
        ("signal_fire".to_string(), "false".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ])
}
