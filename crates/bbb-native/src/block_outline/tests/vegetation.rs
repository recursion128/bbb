use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_world::{BlockPos, TerrainMaterialClass};

use super::super::*;

#[test]
fn outline_shape_uses_vanilla_sapling_shape() {
    for block_name in [
        "minecraft:oak_sapling",
        "minecraft:spruce_sapling",
        "minecraft:cherry_sapling",
        "minecraft:pale_oak_sapling",
    ] {
        assert_eq!(
            outline_shape_for_block(Some(block_name), &stage_properties(1)),
            Some(BlockOutlineShape::single(BlockOutlineBox::SAPLING))
        );
    }
}

#[test]
fn outline_shape_uses_vanilla_simple_vegetation_shapes() {
    for block_name in [
        "minecraft:short_grass",
        "minecraft:fern",
        "minecraft:dead_bush",
    ] {
        assert_eq!(
            outline_shape_for_block(Some(block_name), &BTreeMap::new()),
            Some(BlockOutlineShape::single(
                BlockOutlineBox::GROUND_VEGETATION
            ))
        );
    }

    assert_eq!(
        outline_shape_for_block(Some("minecraft:bush"), &BTreeMap::new()),
        Some(BlockOutlineShape::single(BlockOutlineBox::BUSH))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:short_dry_grass"), &BTreeMap::new()),
        Some(BlockOutlineShape::single(BlockOutlineBox::SHORT_DRY_GRASS))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:tall_dry_grass"), &BTreeMap::new()),
        Some(BlockOutlineShape::single(BlockOutlineBox::TALL_DRY_GRASS))
    );
}

#[test]
fn outline_shape_uses_vanilla_crop_age_heights() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:wheat"), &age_properties(0)),
        Some(BlockOutlineShape::single(BlockOutlineBox::centered_column(
            16.0, 16.0, 0.0, 2.0,
        )))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:wheat"), &age_properties(7)),
        Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:carrots"), &age_properties(7)),
        Some(BlockOutlineShape::single(BlockOutlineBox::centered_column(
            16.0, 16.0, 0.0, 9.0,
        )))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:potatoes"), &age_properties(3)),
        Some(BlockOutlineShape::single(BlockOutlineBox::centered_column(
            16.0, 16.0, 0.0, 5.0,
        )))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:beetroots"), &age_properties(3)),
        Some(BlockOutlineShape::single(BlockOutlineBox::centered_column(
            16.0, 16.0, 0.0, 8.0,
        )))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:nether_wart"), &age_properties(3)),
        Some(BlockOutlineShape::single(BlockOutlineBox::centered_column(
            16.0, 16.0, 0.0, 14.0,
        )))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:torchflower_crop"), &age_properties(1)),
        Some(BlockOutlineShape::single(BlockOutlineBox::centered_column(
            6.0, 6.0, 0.0, 10.0,
        )))
    );
}

#[test]
fn outline_shape_uses_vanilla_sweet_berry_bush_age_shapes() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:sweet_berry_bush"), &age_properties(0)),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::SWEET_BERRY_SAPLING,
        ))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:sweet_berry_bush"), &age_properties(2)),
        Some(BlockOutlineShape::single(
            BlockOutlineBox::SWEET_BERRY_GROWING,
        ))
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:sweet_berry_bush"), &age_properties(3)),
        Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
    );
}

#[test]
fn outline_shape_rejects_invalid_crop_age_properties() {
    assert_eq!(
        outline_shape_for_block(Some("minecraft:wheat"), &BTreeMap::new()),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:beetroots"), &age_properties(4)),
        None
    );
    assert_eq!(
        outline_shape_for_block(Some("minecraft:torchflower_crop"), &age_properties(2)),
        None
    );
}

#[test]
fn crop_outline_clip_uses_age_height() {
    let target = BlockOutlineTarget {
        material: TerrainMaterialClass::Opaque,
        outline: outline_shape_for_block(Some("minecraft:carrots"), &age_properties(3)),
    };

    assert_eq!(
        target.clip(
            [0.5, 2.0, 0.5],
            [0.0, -1.0, 0.0],
            4.5,
            BlockPos { x: 0, y: 0, z: 0 },
        ),
        Some(BlockOutlineHit {
            distance: 1.6875,
            face: ProtocolDirection::Up,
            inside: false,
        })
    );
}

fn age_properties(age: u8) -> BTreeMap<String, String> {
    BTreeMap::from([("age".to_string(), age.to_string())])
}

fn stage_properties(stage: u8) -> BTreeMap<String, String> {
    BTreeMap::from([("stage".to_string(), stage.to_string())])
}
