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

fn brewing_stand_properties() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("has_bottle_0".to_string(), "false".to_string()),
        ("has_bottle_1".to_string(), "false".to_string()),
        ("has_bottle_2".to_string(), "false".to_string()),
    ])
}
