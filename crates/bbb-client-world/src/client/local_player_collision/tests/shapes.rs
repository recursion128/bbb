use super::*;

#[test]
fn leaves_collision_membership_rejects_suffix_lookalikes() {
    assert!(is_leaves_block_name("minecraft:oak_leaves"));
    assert!(is_leaves_block_name("minecraft:flowering_azalea_leaves"));
    assert!(!is_leaves_block_name("minecraft:custom_leaves"));
    assert!(!is_leaves_block_name("example:oak_leaves"));
}

#[test]
fn every_static_vanilla_state_projects_the_generated_collision_aabbs_exactly() {
    let world = empty_world();
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let template = world.block_view(pos).expect("air template probe");

    for state in crate::registries::BlockStateRegistry::vanilla_26_1().iter() {
        let Some(expected) = vanilla_block_state_static_collision_shape(state.id) else {
            continue;
        };
        let probe =
            block_probe_for_state(&world, &template, state.id).expect("vanilla state probe");
        let actual = block_collision_shape(&probe, pos)
            .map(|shape| {
                shape
                    .boxes()
                    .map(|shape_box| {
                        [
                            shape_box.min[0],
                            shape_box.min[1],
                            shape_box.min[2],
                            shape_box.max[0],
                            shape_box.max[1],
                            shape_box.max[2],
                        ]
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let expected = expected
            .boxes()
            .map(|shape_box| {
                [
                    shape_box.min_x,
                    shape_box.min_y,
                    shape_box.min_z,
                    shape_box.max_x,
                    shape_box.max_y,
                    shape_box.max_z,
                ]
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{} ({})", state.name, state.id);
        assert_eq!(
            probe.collision_shape_is_full_block(),
            expected == [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]],
            "{} ({})",
            state.name,
            state.id
        );
    }
}

#[test]
fn diode_and_light_collision_shapes_match_vanilla() {
    let mut world = empty_world();
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    for block_name in ["minecraft:repeater", "minecraft:comparator"] {
        let state_id = bbb_protocol::block_state_types::vanilla_block_default_state_id(
            vanilla_block_type_for_resource_id(block_name).unwrap().id,
        )
        .unwrap();
        set_block(&mut world, pos, state_id);
        let probe = world.block_view(pos).expect("diode probe");
        let shape_box = block_collision_shape(&probe, pos)
            .expect("diode collision shape")
            .boxes()
            .next()
            .expect("diode collision box");
        assert_eq!(
            shape_box,
            WorldCollisionBox::centered_column(16.0, 16.0, 0.0, 2.0),
            "{block_name}"
        );
    }

    set_block(
        &mut world,
        pos,
        vanilla_block_state_id(
            "minecraft:light",
            [("level", "15"), ("waterlogged", "false")],
        ),
    );
    let light = world.block_view(pos).expect("light probe");
    assert!(
        block_collision_shape(&light, pos).is_none(),
        "LightBlock's entity collision uses its empty-context outline shape"
    );
}

#[test]
fn shulker_box_collision_shape_tracks_current_lid_progress_in_all_directions() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let cases = [
        ("north", [0.0, 0.0, -0.25], [1.0, 1.0, 1.0]),
        ("east", [0.0, 0.0, 0.0], [1.25, 1.0, 1.0]),
        ("south", [0.0, 0.0, 0.0], [1.0, 1.0, 1.25]),
        ("west", [-0.25, 0.0, 0.0], [1.0, 1.0, 1.0]),
        ("up", [0.0, 0.0, 0.0], [1.0, 1.25, 1.0]),
        ("down", [0.0, -0.25, 0.0], [1.0, 1.0, 1.0]),
    ];

    for (facing, expected_min, expected_max) in cases {
        let mut world = empty_world();
        set_block(
            &mut world,
            pos,
            vanilla_block_state_id("minecraft:shulker_box", [("facing", facing)]),
        );
        let closed = world.block_view(pos).expect("closed shulker probe");
        assert_eq!(
            shulker_box_collision_shape(&world, &closed, pos)
                .unwrap()
                .boxes()
                .next(),
            Some(WorldCollisionBox::FULL)
        );

        world.apply_block_event(bbb_protocol::packets::BlockEvent {
            pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
            b0: 1,
            b1: 1,
            block_id: 0,
        });
        world.advance_shulker_box_lid_ticks(5);
        let opening = world.block_view(pos).expect("opening shulker probe");
        let shape_box = shulker_box_collision_shape(&world, &opening, pos)
            .unwrap()
            .boxes()
            .next()
            .unwrap();
        for (actual, expected) in [
            (shape_box.min[0], expected_min[0]),
            (shape_box.min[1], expected_min[1]),
            (shape_box.min[2], expected_min[2]),
            (shape_box.max[0], expected_max[0]),
            (shape_box.max[1], expected_max[1]),
            (shape_box.max[2], expected_max[2]),
        ] {
            assert_f64_near(actual, expected);
        }
    }
}

#[test]
fn pointed_dripstone_xz_offset_matches_vanilla_seed() {
    let (origin_x, origin_z) = pointed_dripstone_xz_offset(BlockPos { x: 0, y: 0, z: 0 });
    assert_f64_near(origin_x, -0.125);
    assert_f64_near(origin_z, -0.125);

    let (offset_x, offset_z) = pointed_dripstone_xz_offset(BlockPos { x: 1, y: 99, z: 0 });
    assert_f64_near(offset_x, 7.0 / 60.0);
    assert_f64_near(offset_z, -1.0 / 60.0);
}

#[test]
fn pointed_dripstone_tip_up_shape_uses_direction_height_and_offset() {
    let mut properties = BTreeMap::new();
    properties.insert("thickness".to_owned(), "tip".to_owned());
    properties.insert("vertical_direction".to_owned(), "up".to_owned());

    let shape = pointed_dripstone_collision_shape(&properties, BlockPos { x: 0, y: 7, z: 0 })
        .expect("pointed dripstone shape");
    let mut boxes = shape.boxes();
    let shape_box = boxes.next().expect("shape box");
    assert!(boxes.next().is_none());

    assert_f64_near(shape_box.min[0], 3.0 * PX);
    assert_f64_near(shape_box.min[1], 0.0);
    assert_f64_near(shape_box.min[2], 3.0 * PX);
    assert_f64_near(shape_box.max[0], 9.0 * PX);
    assert_f64_near(shape_box.max[1], 11.0 * PX);
    assert_f64_near(shape_box.max[2], 9.0 * PX);
}

#[test]
fn camera_visual_shape_overrides_match_vanilla_catalog() {
    let world = empty_world();
    let context = LocalPlayerCollisionContext::for_bounds(WorldCollisionBox {
        min: [0.2, 1.0, 0.2],
        max: [0.8, 2.8, 0.8],
    });
    let shape_for = |name: &str, properties: BTreeMap<String, String>| {
        let block_type = vanilla_block_type_for_resource_id(name).expect("vanilla block");
        let state_id = if properties.is_empty() {
            bbb_protocol::block_state_types::vanilla_block_default_state_id(block_type.id)
        } else {
            crate::vanilla_block_state_id(name, &properties)
        }
        .expect("vanilla block state");
        let block = BlockView::of_state(BlockPos { x: 0, y: 0, z: 0 }, state_id);
        block_visual_shape_with_context(&world, &block, block.pos, context)
    };

    for empty in [
        "minecraft:glass",
        "minecraft:red_stained_glass",
        "minecraft:tinted_glass",
        "minecraft:copper_grate",
        "minecraft:iron_bars",
        "minecraft:blue_stained_glass_pane",
        "minecraft:powder_snow",
    ] {
        assert!(shape_for(empty, BTreeMap::new()).is_none(), "{empty}");
    }

    for full in ["minecraft:soul_sand", "minecraft:mud"] {
        assert!(
            shape_for(full, BTreeMap::new()).is_some_and(BlockCollisionShape::is_full_block),
            "{full}"
        );
    }

    let snow = shape_for("minecraft:snow", string_props([("layers", "4")]))
        .expect("four-layer visual snow");
    assert_f64_near(snow.boxes().next().expect("snow box").max[1], 0.5);

    let fence = shape_for(
        "minecraft:oak_fence",
        string_props([
            ("east", "false"),
            ("north", "true"),
            ("south", "false"),
            ("waterlogged", "false"),
            ("west", "false"),
        ]),
    )
    .expect("fence visual shape");
    assert!(fence.boxes().all(|shape_box| shape_box.max[1] == 1.0));
}

#[test]
fn collision_shape_to_aabbs_coalesces_overlapping_union_cells_like_vanilla() {
    let shape = BlockCollisionShape::from_boxes([
        Some(WorldCollisionBox {
            min: [0.0, 0.0, 0.0],
            max: [0.75, 1.0, 1.0],
        }),
        Some(WorldCollisionBox {
            min: [0.25, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        }),
    ]);
    assert_eq!(shape.to_aabbs(), [WorldCollisionBox::FULL]);
}

#[test]
fn collision_shape_to_aabbs_preserves_non_rectangular_union_as_merged_boxes() {
    let shape = BlockCollisionShape::from_boxes([
        Some(WorldCollisionBox {
            min: [0.0, 0.0, 0.0],
            max: [0.5, 1.0, 1.0],
        }),
        Some(WorldCollisionBox {
            min: [0.5, 0.0, 0.5],
            max: [1.0, 1.0, 1.0],
        }),
    ]);
    assert_eq!(
        shape.to_aabbs(),
        [
            WorldCollisionBox {
                min: [0.0, 0.0, 0.0],
                max: [0.5, 1.0, 1.0]
            },
            WorldCollisionBox {
                min: [0.5, 0.0, 0.5],
                max: [1.0, 1.0, 1.0]
            },
        ]
    );
}

#[test]
fn debug_solid_faces_use_full_support_shapes_and_vanilla_overrides() {
    let mut world = empty_world();
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let cases = [
        (vanilla_block_state_id("minecraft:stone", []), [true; 6]),
        (
            vanilla_block_state_id(
                "minecraft:oak_slab",
                [("type", "bottom"), ("waterlogged", "false")],
            ),
            [false, false, false, false, true, false],
        ),
        (
            vanilla_block_state_id("minecraft:snow", [("layers", "1")]),
            [false, false, false, false, true, false],
        ),
        (
            vanilla_block_state_id("minecraft:snow", [("layers", "8")]),
            [true; 6],
        ),
        (vanilla_block_state_id("minecraft:soul_sand", []), [true; 6]),
        (
            vanilla_block_state_id(
                "minecraft:oak_leaves",
                [
                    ("distance", "7"),
                    ("persistent", "false"),
                    ("waterlogged", "false"),
                ],
            ),
            [false; 6],
        ),
        (
            vanilla_block_state_id(
                "minecraft:oak_fence_gate",
                [
                    ("facing", "north"),
                    ("in_wall", "false"),
                    ("open", "false"),
                    ("powered", "false"),
                ],
            ),
            [false; 6],
        ),
        (
            vanilla_block_state_id(
                "minecraft:scaffolding",
                [
                    ("bottom", "false"),
                    ("distance", "0"),
                    ("waterlogged", "false"),
                ],
            ),
            [false, false, false, false, false, true],
        ),
    ];

    for (block_state_id, expected) in cases {
        set_block(&mut world, pos, block_state_id);
        let probe = world.block_view(pos).expect("test block probe");
        assert_eq!(world.block_sturdy_faces(&probe), expected);
        for (direction, expected) in [
            ProtocolDirection::Down,
            ProtocolDirection::Up,
            ProtocolDirection::North,
            ProtocolDirection::South,
            ProtocolDirection::West,
            ProtocolDirection::East,
        ]
        .into_iter()
        .zip([
            expected[4],
            expected[5],
            expected[3],
            expected[1],
            expected[0],
            expected[2],
        ]) {
            assert_eq!(
                world.block_full_face_sturdy(&probe, direction),
                Some(expected),
                "{block_state_id} {direction:?}"
            );
        }
    }
}

#[test]
fn debug_solid_faces_track_open_shulker_support_direction() {
    let mut world = empty_world();
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let shulker = vanilla_block_state_id("minecraft:shulker_box", [("facing", "east")]);
    set_block(&mut world, pos, shulker);
    let probe = world.block_view(pos).expect("closed shulker probe");
    assert_eq!(world.block_sturdy_faces(&probe), [true; 6]);

    world.apply_block_event(bbb_protocol::packets::BlockEvent {
        pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
        b0: 1,
        b1: 1,
        block_id: 0,
    });
    let probe = world.block_view(pos).expect("opening shulker probe");
    assert_eq!(
        world.block_sturdy_faces(&probe),
        [true, false, false, false, false, false]
    );
    assert_eq!(
        world.block_full_face_sturdy(&probe, ProtocolDirection::West),
        Some(true)
    );
    assert_eq!(
        world.block_full_face_sturdy(&probe, ProtocolDirection::East),
        Some(false)
    );
}

#[test]
fn debug_scaffolding_outline_boxes_reuse_vanilla_voxel_union_merge() {
    let mut world = empty_world();
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let stable = vanilla_block_state_id(
        "minecraft:scaffolding",
        [
            ("bottom", "false"),
            ("distance", "0"),
            ("waterlogged", "false"),
        ],
    );
    set_block(&mut world, pos, stable);
    let probe = world.probe_block(pos).expect("stable scaffolding probe");
    let boxes = world
        .debug_scaffolding_outline_shape_boxes(&probe)
        .expect("stable scaffolding outline boxes");
    assert_eq!(boxes.len(), 7);
    assert_eq!(boxes[0], ([0.0, 0.0, 0.0], [0.125, 1.0, 0.125]));
    assert_eq!(boxes[4], ([0.0, 0.875, 0.125], [1.0, 1.0, 0.875]));

    let unstable = vanilla_block_state_id(
        "minecraft:scaffolding",
        [
            ("bottom", "true"),
            ("distance", "1"),
            ("waterlogged", "false"),
        ],
    );
    set_block(&mut world, pos, unstable);
    let probe = world.probe_block(pos).expect("bottom scaffolding probe");
    let boxes = world
        .debug_scaffolding_outline_shape_boxes(&probe)
        .expect("bottom scaffolding outline boxes");
    assert_eq!(boxes.len(), 8);
    assert_eq!(boxes[0], ([0.0, 0.0, 0.0], [1.0, 0.125, 1.0]));
}

#[test]
fn connected_fence_to_aabbs_removes_post_arm_overlap() {
    let properties = BTreeMap::from([
        ("north".to_owned(), "true".to_owned()),
        ("east".to_owned(), "true".to_owned()),
        ("south".to_owned(), "false".to_owned()),
        ("west".to_owned(), "false".to_owned()),
    ]);
    let boxes = fence_collision_shape(&properties)
        .expect("connected fence shape")
        .to_aabbs();
    assert_eq!(
        boxes,
        [
            WorldCollisionBox {
                min: [6.0 * PX, 0.0, 0.0],
                max: [10.0 * PX, 1.5, 10.0 * PX]
            },
            WorldCollisionBox {
                min: [10.0 * PX, 0.0, 6.0 * PX],
                max: [1.0, 1.5, 10.0 * PX]
            },
        ]
    );
}
