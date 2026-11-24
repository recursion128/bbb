use std::collections::BTreeMap;

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;

use super::*;

fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect::<BTreeMap<_, _>>();
    vanilla_block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("Java 26.1 state {name}: {properties:?}"))
}

fn bamboo(age: &str, leaves: &str, stage: &str) -> i32 {
    state(
        "minecraft:bamboo",
        &[("age", age), ("leaves", leaves), ("stage", stage)],
    )
}

fn dripstone(direction: &str, thickness: &str, waterlogged: &str) -> i32 {
    state(
        "minecraft:pointed_dripstone",
        &[
            ("vertical_direction", direction),
            ("thickness", thickness),
            ("waterlogged", waterlogged),
        ],
    )
}

fn origin() -> BlockPos {
    BlockPos { x: 0, y: 0, z: 0 }
}

#[test]
fn positioned_shapes_native_seed_wrapping_and_float_bits() {
    // Java 26.1 Mth.getSeed / OffsetType.XZ, evaluated with the prepared JDK.
    let cases = [
        (0, 0, 0xbfd0000000000000, 0xbfd0000000000000),
        (1, 2, 0xbfbdddddc0000000, 0xbfc7777770000000),
        (-1, -2, 0xbfb5555540000000, 0x3fa9999a00000000),
        (686, 1, 0x3fa9999a00000000, 0xbfa9999980000000),
        (687, 1, 0x3fc7777780000000, 0xbfa9999980000000),
        (i32::MIN, i32::MAX, 0x3f91111200000000, 0xbfc7777770000000),
        (i32::MAX, i32::MIN, 0xbfd0000000000000, 0xbfc3333330000000),
        (100000, -100000, 0x3fbdddde00000000, 0xbfbdddddc0000000),
    ];
    for (x, z, x_bits, z_bits) in cases {
        let pos = BlockPos { x, y: i32::MAX, z };
        let offset = xz_offset(pos, 0.25);
        assert_eq!(offset.x.to_bits(), x_bits, "X at {pos:?}");
        assert_eq!(offset.y.to_bits(), 0);
        assert_eq!(offset.z.to_bits(), z_bits, "Z at {pos:?}");
        let clamped = xz_offset(pos, 0.125);
        assert_eq!(clamped.x, f64::from_bits(x_bits).clamp(-0.125, 0.125));
        assert_eq!(clamped.z, f64::from_bits(z_bits).clamp(-0.125, 0.125));
    }
}

#[test]
fn positioned_shapes_bamboo_collision_outline_and_visual() {
    for age in ["0", "1"] {
        for stage in ["0", "1"] {
            for leaves in ["none", "small", "large"] {
                let state = bamboo(age, leaves, stage);
                assert!(static_block_collision_boxes(state).is_none());
                assert!(static_block_outline_boxes(state).is_none());
                let collision = vec![Aabb {
                    min_x: 0.15625,
                    min_y: 0.0,
                    min_z: 0.15625,
                    max_x: 0.34375,
                    max_y: 1.0,
                    max_z: 0.34375,
                }];
                assert_eq!(collision_boxes(state, origin()), Some(collision.clone()));
                assert_eq!(visual_boxes(state, origin()), Some(collision));
                let (min, max) = if leaves == "large" {
                    (-0.0625, 0.5625)
                } else {
                    (0.0625, 0.4375)
                };
                assert_eq!(
                    outline_boxes(state, origin()),
                    Some(vec![Aabb {
                        min_x: min,
                        min_y: 0.0,
                        min_z: min,
                        max_x: max,
                        max_y: 1.0,
                        max_z: max,
                    }])
                );
            }
        }
    }
}

#[test]
fn positioned_shapes_dripstone_thickness_direction_and_water() {
    let shapes = [
        ("tip_merge", 0.1875, 0.5625, 0.0, 1.0),
        ("tip", 0.1875, 0.5625, 0.0, 0.6875),
        ("frustum", 0.125, 0.625, 0.0, 1.0),
        ("middle", 0.0625, 0.6875, 0.0, 1.0),
        ("base", 0.0, 0.75, 0.0, 1.0),
    ];
    for (thickness, min, max, mut min_y, mut max_y) in shapes {
        for direction in ["up", "down"] {
            if thickness == "tip" && direction == "down" {
                min_y = 0.3125;
                max_y = 1.0;
            }
            for waterlogged in ["false", "true"] {
                let state = dripstone(direction, thickness, waterlogged);
                assert!(static_block_collision_boxes(state).is_none());
                let expected = Some(vec![Aabb {
                    min_x: min,
                    min_y,
                    min_z: min,
                    max_x: max,
                    max_y,
                    max_z: max,
                }]);
                assert_eq!(collision_boxes(state, origin()), expected);
                assert_eq!(outline_boxes(state, origin()), expected);
                assert_eq!(visual_boxes(state, origin()), expected);
            }
        }
    }
}

#[test]
fn positioned_shapes_public_queries_apply_native_xz_offsets() {
    let pos = BlockPos {
        x: -1,
        y: 99,
        z: -2,
    };
    let offset_x = f64::from_bits(0xbfb5555540000000);
    let offset_z = f64::from_bits(0x3fa9999a00000000);
    for (boxes, min, max) in [
        (
            collision_boxes(bamboo("0", "large", "0"), pos),
            0.40625,
            0.59375,
        ),
        (
            outline_boxes(bamboo("0", "large", "0"), pos),
            0.1875,
            0.8125,
        ),
        (
            visual_boxes(bamboo("0", "large", "0"), pos),
            0.40625,
            0.59375,
        ),
        (
            collision_boxes(dripstone("up", "tip", "false"), pos),
            0.3125,
            0.6875,
        ),
    ] {
        let bounds = boxes.unwrap()[0];
        assert_eq!(bounds.min_x.to_bits(), (min + offset_x).to_bits());
        assert_eq!(bounds.max_x.to_bits(), (max + offset_x).to_bits());
        assert_eq!(bounds.min_z.to_bits(), (min + offset_z).to_bits());
        assert_eq!(bounds.max_z.to_bits(), (max + offset_z).to_bits());
    }
}

#[test]
fn positioned_shapes_y_independent_and_block_local() {
    let pos = BlockPos { x: -1, y: 0, z: -2 };
    for state in [bamboo("1", "large", "1"), dripstone("down", "tip", "true")] {
        for query in [collision_boxes, outline_boxes, visual_boxes] {
            let expected = query(state, pos).unwrap();
            for y in [i32::MIN, -1, 1, i32::MAX] {
                assert_eq!(query(state, BlockPos { y, ..pos }), Some(expected.clone()));
            }
            let bounds = expected[0];
            assert!(bounds.min_x > -0.25 && bounds.max_x < 1.25);
            assert!(bounds.min_z > -0.25 && bounds.max_z < 1.25);
            assert!(bounds.min_y >= 0.0 && bounds.max_y <= 1.0);
        }
    }
}

#[test]
fn positioned_shapes_existing_static_owners_preserved() {
    let states = [
        state("minecraft:air", &[]),
        state("minecraft:stone", &[]),
        state("minecraft:mud", &[]),
        state("minecraft:glass", &[]),
        state("minecraft:snow", &[("layers", "3")]),
    ];
    let pos = BlockPos {
        x: i32::MIN,
        y: i32::MAX,
        z: 12345,
    };
    for state in states {
        assert_eq!(
            collision_boxes(state, pos),
            static_block_collision_boxes(state)
        );
        assert_eq!(outline_boxes(state, pos), static_block_outline_boxes(state));
        assert_eq!(visual_boxes(state, pos), static_block_visual_boxes(state));
    }
}

#[test]
fn positioned_shapes_unknown_and_unowned_dynamic_refused() {
    let moving_piston = state(
        "minecraft:moving_piston",
        &[("facing", "north"), ("type", "normal")],
    );
    for state in [i32::MIN, -1, i32::MAX, moving_piston] {
        assert_eq!(collision_boxes(state, origin()), None);
        assert_eq!(outline_boxes(state, origin()), None);
        assert_eq!(visual_boxes(state, origin()), None);
    }
}
