//! `static_block_visual_boxes` over the 26.1 states whose `getVisualShape`
//! override answers something other than the collision shape.

use std::collections::BTreeMap;

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::packets::BlockPos;

use super::{static_block_collision_boxes, static_block_outline_boxes, static_block_visual_boxes};
use crate::geometry::Aabb;

fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect::<BTreeMap<_, _>>();
    vanilla_block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("built-in Java 26.1 {name}"))
}

/// `CrossCollisionBlock`'s default state, shared by the bars, the panes and the
/// fences.
fn cross<'a>(extra: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut properties = vec![
        ("east", "false"),
        ("north", "false"),
        ("south", "false"),
        ("waterlogged", "false"),
        ("west", "false"),
    ];
    properties.extend_from_slice(extra);
    properties
}

/// `TransparentBlock.getVisualShape`, `IronBarsBlock.getVisualShape` and
/// `PowderSnowBlock.getVisualShape` answer `Shapes.empty()`
/// (`TransparentBlock.java:25-27`, `IronBarsBlock.java:81-83`,
/// `PowderSnowBlock.java:135-137`), so a `ClipContext.Block.VISUAL` ray passes
/// through them even though four of the families collide.
#[test]
fn the_empty_visual_owners_let_a_visual_ray_through_their_collision_shape() {
    let empty = Vec::new();
    for id in [
        state("minecraft:glass", &[]),
        state("minecraft:tinted_glass", &[]),
        state("minecraft:white_stained_glass", &[]),
        state("minecraft:copper_grate", &[("waterlogged", "false")]),
        state("minecraft:iron_bars", &cross(&[])),
        state("minecraft:glass_pane", &cross(&[])),
        state("minecraft:copper_bars", &cross(&[])),
        state("minecraft:powder_snow", &[]),
    ] {
        assert_eq!(static_block_visual_boxes(id), Some(empty.clone()));
    }
    for id in [
        state("minecraft:glass", &[]),
        state("minecraft:iron_bars", &cross(&[])),
        state("minecraft:glass_pane", &cross(&[])),
    ] {
        assert_ne!(
            static_block_collision_boxes(id),
            Some(Vec::new()),
            "a collider ray still stops on it"
        );
    }
}

/// `SnowLayerBlock.getVisualShape` and `FenceBlock.getVisualShape` answer
/// `getShape` (`SnowLayerBlock.java:62-64`, `FenceBlock.java:50-52`), which is
/// the outline palette, and a snow layer's outline is one layer taller than its
/// collision shape (`SnowLayerBlock.java:47-54`).
#[test]
fn the_outline_visual_owners_answer_the_outline_palette() {
    for id in [
        state("minecraft:snow", &[("layers", "1")]),
        state("minecraft:snow", &[("layers", "4")]),
        state("minecraft:snow", &[("layers", "8")]),
        state("minecraft:oak_fence", &cross(&[])),
        state("minecraft:nether_brick_fence", &cross(&[])),
    ] {
        assert_eq!(
            static_block_visual_boxes(id),
            static_block_outline_boxes(id),
            "getVisualShape defers to getShape"
        );
    }
    let four = state("minecraft:snow", &[("layers", "4")]);
    assert_ne!(
        static_block_collision_boxes(four),
        static_block_outline_boxes(four),
        "a four-layer snow block collides four layers high and is five layers tall"
    );
}

/// `MudBlock.getVisualShape` and `SoulSandBlock.getVisualShape` answer
/// `Shapes.block()` (`MudBlock.java:37-39`, `SoulSandBlock.java:37-39`), which
/// is taller than their own collision shape.
#[test]
fn the_full_block_visual_owners_answer_a_full_block() {
    let full_block = vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })];
    for id in [
        state("minecraft:mud", &[]),
        state("minecraft:soul_sand", &[]),
    ] {
        assert_eq!(static_block_visual_boxes(id), Some(full_block.clone()));
        assert_ne!(static_block_collision_boxes(id), Some(full_block.clone()));
    }
}

/// Every other state inherits `BlockBehaviour.getVisualShape`'s collision-shape
/// body (`BlockBehaviour.java:345-347`).
#[test]
fn every_other_state_answers_its_collision_shape() {
    for id in [
        state("minecraft:stone", &[]),
        state("minecraft:oak_planks", &[]),
        state("minecraft:dirt", &[]),
        state("minecraft:sand", &[]),
        state("minecraft:short_grass", &[]),
        state("minecraft:water", &[("level", "0")]),
    ] {
        assert_eq!(
            static_block_visual_boxes(id),
            static_block_collision_boxes(id)
        );
    }
}

mod moving_pistons;
mod whole_shapes;

#[test]
fn checked_block_collision_stops_before_later_unknown_and_keeps_clear_unknown() {
    let query = Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 });
    let hit = BlockPos { x: 0, y: 0, z: 0 };
    let unit = Aabb::of_block(hit);
    assert_eq!(
        super::no_block_collision_with_moving_pistons(
            query,
            |_| Some(false),
            |pos| {
                if pos == hit {
                    Some(vec![unit])
                } else if pos.z > 0 {
                    None
                } else {
                    Some(Vec::new())
                }
            },
        ),
        Some(false)
    );
    assert_eq!(
        super::no_block_collision_with_moving_pistons(query, |_| Some(false), |_| None),
        None
    );
    assert_eq!(
        super::no_block_collision_with_moving_pistons(query, |_| Some(false), |_| Some(Vec::new())),
        Some(true)
    );
}

mod minmax;
