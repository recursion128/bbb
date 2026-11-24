use super::fixture::*;

#[test]
fn lantern_placement_hangs_from_supported_ceiling_and_waterlogs_in_source_water() {
    let item = item_id("minecraft:lantern");
    let mut ceiling_context = context(CLICKED);
    ceiling_context.clicked_face = Direction::Down;

    let hanging = state_for_placement_with_view(
        item,
        ceiling_context,
        placement_view(-90.0, 0.0),
        vanilla_tags(),
        &mut |at| {
            if at == CLICKED {
                water(0)
            } else if at == above(CLICKED) {
                stone()
            } else {
                air()
            }
        },
        &mut |_, _| Some(true),
    );
    assert_eq!(
        hanging,
        Some(state_of(
            "minecraft:lantern",
            &[("hanging", "true"), ("waterlogged", "true")]
        ))
    );

    let fallback = state_for_placement_with_view(
        item,
        ceiling_context,
        placement_view(-90.0, 0.0),
        vanilla_tags(),
        &mut |at| {
            if at == CLICKED {
                water(1)
            } else if at == below(CLICKED) {
                stone()
            } else {
                air()
            }
        },
        &mut |_, _| Some(true),
    );
    assert_eq!(
        fallback,
        Some(state_of(
            "minecraft:lantern",
            &[("hanging", "false"), ("waterlogged", "false")]
        ))
    );
}

#[test]
fn multiface_placement_adds_the_first_attachable_vacant_face() {
    for name in ["minecraft:glow_lichen", "minecraft:sculk_vein"] {
        let item = item_id(name);
        let placed = state_for_placement_with_view(
            item,
            context(CLICKED),
            placement_view(0.0, 0.0),
            vanilla_tags(),
            &mut |at| {
                if at == below(CLICKED) {
                    stone()
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        );
        assert_eq!(
            placed,
            Some(multiface_state(name, &[Direction::Down], false)),
            "{name}"
        );
    }

    for (fluid, waterlogged) in [(water(0), true), (water(1), false)] {
        let placed = state_for_placement_with_view(
            item_id("minecraft:glow_lichen"),
            context(CLICKED),
            placement_view(0.0, 0.0),
            vanilla_tags(),
            &mut |at| if at == CLICKED { fluid } else { stone() },
            &mut |_, _| Some(true),
        );
        assert_eq!(
            placed,
            Some(multiface_state(
                "minecraft:glow_lichen",
                &[Direction::Down],
                waterlogged
            ))
        );
    }

    let old = multiface_state("minecraft:glow_lichen", &[Direction::Down], false);
    let expanded = state_for_placement_with_view(
        item_id("minecraft:glow_lichen"),
        context(CLICKED),
        placement_view(0.0, 0.0),
        vanilla_tags(),
        &mut |at| {
            if at == CLICKED || at == relative(CLICKED, Direction::South) {
                if at == CLICKED {
                    old
                } else {
                    stone()
                }
            } else {
                air()
            }
        },
        &mut |_, _| Some(true),
    );
    assert_eq!(
        expanded,
        Some(multiface_state(
            "minecraft:glow_lichen",
            &[Direction::Down, Direction::South],
            false,
        ))
    );
}

#[test]
fn multiface_replaceability_requires_a_vacant_face_for_the_same_item() {
    let item = item_id("minecraft:glow_lichen");
    let vacant = default_state_of("minecraft:glow_lichen");
    let full = multiface_state(
        "minecraft:glow_lichen",
        &[
            Direction::Down,
            Direction::Up,
            Direction::North,
            Direction::South,
            Direction::West,
            Direction::East,
        ],
        false,
    );
    assert_eq!(
        can_replace_for_placement(item, vacant, context(CLICKED)),
        Some(true)
    );
    assert_eq!(
        can_replace_for_placement(item, full, context(CLICKED)),
        Some(false)
    );
    assert_eq!(
        can_replace_for_placement(item_id("minecraft:stone"), full, context(CLICKED)),
        Some(true)
    );
}

#[test]
fn vine_placement_uses_nearest_vacant_face_and_above_fallback() {
    let item = item_id("minecraft:vine");

    let old = vine_state(&[Direction::South]);
    let west = relative(CLICKED, Direction::West);
    let expanded = state_for_placement_with_view(
        item,
        context(CLICKED),
        placement_view(0.0, 0.0),
        vanilla_tags(),
        &mut |at| {
            if at == CLICKED {
                old
            } else if at == west {
                stone()
            } else {
                air()
            }
        },
        &mut |_, _| Some(true),
    );
    assert_eq!(
        expanded,
        Some(vine_state(&[Direction::South, Direction::West]))
    );

    assert_eq!(
        state_for_placement_with_view(
            item,
            context(CLICKED),
            placement_view(0.0, 0.0),
            vanilla_tags(),
            &mut |at| if at == CLICKED { old } else { air() },
            &mut |_, _| Some(true),
        ),
        Some(old),
        "an existing vine with no new supported face is preserved"
    );
}

#[test]
fn cocoa_placement_uses_nearest_supported_horizontal_face() {
    let item = item_id("minecraft:cocoa_beans");

    let west = relative(CLICKED, Direction::West);
    let jungle_log = default_state_of("minecraft:jungle_log");
    let state = state_for_placement_with_view(
        item,
        context(CLICKED),
        placement_view(0.0, 0.0),
        vanilla_tags(),
        &mut |at| {
            if at == west {
                jungle_log
            } else {
                stone()
            }
        },
        &mut |_, _| Some(true),
    );
    assert_eq!(
        state,
        Some(state_of(
            "minecraft:cocoa",
            &[("age", "0"), ("facing", "west")]
        ))
    );

    let mut reordered = context(CLICKED);
    reordered.clicked_face = Direction::East;
    assert_eq!(
        state_for_placement_with_view(
            item,
            reordered,
            placement_view(0.0, 0.0),
            vanilla_tags(),
            &mut |_| jungle_log,
            &mut |_, _| Some(true),
        ),
        Some(state_of(
            "minecraft:cocoa",
            &[("age", "0"), ("facing", "west")]
        ))
    );
}

#[test]
fn vine_replaceability_counts_all_five_supported_faces() {
    let vine_item = item_id("minecraft:vine");
    let stone_item = item_id("minecraft:stone");
    let vacant = vine_state(&[
        Direction::Up,
        Direction::North,
        Direction::East,
        Direction::South,
    ]);
    let full = vine_state(&[
        Direction::Up,
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ]);
    for item in [vine_item, stone_item] {
        assert_eq!(
            can_replace_for_placement(item, vacant, context(CLICKED)),
            Some(true)
        );
        assert_eq!(
            can_replace_for_placement(item, full, context(CLICKED)),
            Some(false)
        );
    }
}

#[test]
fn dispenser_faces_away_from_the_nearest_view_direction() {
    let cases = [
        (placement_view(-90.0, 0.0), Direction::Down),
        (placement_view(90.0, 0.0), Direction::Up),
        (placement_view(0.0, 0.0), Direction::North),
        (placement_view(0.0, 180.0), Direction::South),
        (placement_view(0.0, -90.0), Direction::West),
        (placement_view(0.0, 90.0), Direction::East),
    ];
    for name in ["minecraft:dispenser"] {
        let item = item_id(name);
        for (view, facing) in cases {
            let placed = state_for_placement_with_view(
                item,
                context(CLICKED),
                view,
                vanilla_tags(),
                &mut |_| air(),
                &mut |_, _| Some(true),
            );
            assert_eq!(
                placed,
                Some(state_of(
                    name,
                    &[
                        ("facing", direction_serialized_name(facing)),
                        ("triggered", "false"),
                    ],
                )),
                "{name} at {view:?}"
            );
        }
    }
}

#[test]
fn crafter_maps_the_nearest_view_to_front_and_top_orientation() {
    let item = item_id("minecraft:crafter");
    let cases = [
        (view(-90.0, 0.0), Direction::East, "down_west"),
        (view(90.0, 0.0), Direction::North, "up_north"),
        (view(0.0, -90.0), Direction::North, "west_up"),
        (view(0.0, 180.0), Direction::North, "south_up"),
    ];
    for (view, horizontal, orientation) in cases {
        let mut placement = context(CLICKED);
        placement.horizontal_direction = horizontal;
        assert_eq!(
            state_for_placement_with_view(
                item,
                placement,
                view,
                vanilla_tags(),
                &mut |_| air(),
                &mut |_, _| Some(true),
            ),
            Some(state_of(
                "minecraft:crafter",
                &[
                    ("crafting", "false"),
                    ("orientation", orientation),
                    ("triggered", "false"),
                ],
            )),
            "view {view:?}, horizontal {horizontal:?}",
        );
    }

    assert_eq!(
        state_for_placement_with_view(
            item,
            context(CLICKED),
            view(0.0, 0.0),
            vanilla_tags(),
            &mut |at| {
                if at == below(CLICKED) {
                    default_state_of("minecraft:redstone_block")
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        ),
        Some(state_of(
            "minecraft:crafter",
            &[
                ("crafting", "false"),
                ("orientation", "north_up"),
                ("triggered", "true"),
            ],
        )),
    );
}

#[test]
fn observer_faces_the_nearest_view_direction() {
    let item = item_id("minecraft:observer");
    let mut placement = context(CLICKED);
    placement.clicked_face = Direction::East;
    for (view, facing) in [
        (placement_view(-90.0, 0.0), Direction::Up),
        (placement_view(90.0, 0.0), Direction::Down),
        (placement_view(0.0, 0.0), Direction::South),
        (placement_view(0.0, 180.0), Direction::North),
        (placement_view(0.0, -90.0), Direction::East),
        (placement_view(0.0, 90.0), Direction::West),
    ] {
        assert_eq!(
            state_for_placement_with_view(
                item,
                placement,
                view,
                vanilla_tags(),
                &mut |_| air(),
                &mut |_, _| Some(true),
            ),
            Some(state_of(
                "minecraft:observer",
                &[
                    ("facing", direction_serialized_name(facing)),
                    ("powered", "false"),
                ],
            )),
            "view {view:?}"
        );
    }
}

#[test]
fn beehive_family_faces_away_from_horizontal_placement_direction() {
    for name in ["minecraft:beehive"] {
        let item = item_id(name);
        for (horizontal, facing) in [
            (Direction::North, Direction::South),
            (Direction::East, Direction::West),
            (Direction::South, Direction::North),
            (Direction::West, Direction::East),
        ] {
            let mut placement = context(CLICKED);
            placement.horizontal_direction = horizontal;
            let expected = Some(state_of(
                name,
                &[
                    ("facing", direction_serialized_name(facing)),
                    ("honey_level", "0"),
                ],
            ));
            assert_eq!(
                state_for_placement_with_view(
                    item,
                    placement,
                    placement_view(0.0, 0.0),
                    vanilla_tags(),
                    &mut |_| air(),
                    &mut |_, _| Some(true),
                ),
                expected,
                "view-aware {name} toward {horizontal:?}"
            );
        }
    }
}

#[test]
fn ladder_placement_attaches_to_the_nearest_supported_wall() {
    let item = item_id("minecraft:ladder");

    let mut anti_self = context(CLICKED);
    anti_self.clicked_face = Direction::South;
    let original_clicked = relative(CLICKED, Direction::North);
    assert_eq!(
        state_for_placement_with_view(
            item,
            anti_self,
            placement_view(0.0, 0.0),
            vanilla_tags(),
            &mut |at| {
                if at == original_clicked {
                    state_of(
                        "minecraft:ladder",
                        &[("facing", "south"), ("waterlogged", "false")],
                    )
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        ),
        None
    );

    let mut replacement = context(CLICKED);
    replacement.replacing_clicked = true;
    let south = relative(CLICKED, Direction::South);
    let west = relative(CLICKED, Direction::West);
    let placed = state_for_placement_with_view(
        item,
        replacement,
        placement_view(0.0, 0.0),
        vanilla_tags(),
        &mut |at| {
            if at == CLICKED {
                water(0)
            } else if at == west {
                stone()
            } else {
                air()
            }
        },
        &mut |_, _| Some(true),
    );
    assert_eq!(
        placed,
        Some(state_of(
            "minecraft:ladder",
            &[("facing", "east"), ("waterlogged", "true")]
        ))
    );

    assert_eq!(
        state_for_placement_with_view(
            item,
            replacement,
            placement_view(0.0, 0.0),
            vanilla_tags(),
            &mut |at| {
                if at == CLICKED {
                    water(1)
                } else if at == south {
                    stone()
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        ),
        Some(state_of(
            "minecraft:ladder",
            &[("facing", "north"), ("waterlogged", "false")]
        ))
    );
}

#[test]
fn tripwire_hook_placement_uses_nearest_supported_wall() {
    let item = item_id("minecraft:tripwire_hook");

    let west = relative(CLICKED, Direction::West);
    assert_eq!(
        state_for_placement_with_view(
            item,
            context(CLICKED),
            placement_view(0.0, 0.0),
            vanilla_tags(),
            &mut |at| {
                if at == west {
                    stone()
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        ),
        Some(state_of(
            "minecraft:tripwire_hook",
            &[
                ("attached", "false"),
                ("facing", "east"),
                ("powered", "false"),
            ]
        ))
    );
}

#[test]
fn hopper_placement_faces_away_from_the_clicked_face() {
    let item = item_id("minecraft:hopper");
    for (clicked_face, facing) in [
        (Direction::Down, Direction::Down),
        (Direction::Up, Direction::Down),
        (Direction::North, Direction::South),
        (Direction::South, Direction::North),
        (Direction::West, Direction::East),
        (Direction::East, Direction::West),
    ] {
        let mut placement = context(CLICKED);
        placement.clicked_face = clicked_face;
        let expected = Some(state_of(
            "minecraft:hopper",
            &[
                ("enabled", "true"),
                ("facing", direction_serialized_name(facing)),
            ],
        ));
        assert_eq!(
            state_for_placement_with_view(
                item,
                placement,
                placement_view(0.0, 0.0),
                vanilla_tags(),
                &mut |_| air(),
                &mut |_, _| Some(true),
            ),
            expected,
            "view-aware path for {clicked_face:?}"
        );
    }
}

#[test]
fn bell_placement_picks_floor_ceiling_or_wall_attachment() {
    let item = item_id("minecraft:bell");

    for (clicked_face, horizontal, support, attachment, facing) in [
        (
            Direction::Up,
            Direction::South,
            below(CLICKED),
            "floor",
            "south",
        ),
        (
            Direction::Down,
            Direction::East,
            above(CLICKED),
            "ceiling",
            "east",
        ),
    ] {
        let mut placement = context(CLICKED);
        placement.clicked_face = clicked_face;
        placement.horizontal_direction = horizontal;
        let expected = Some(state_of(
            "minecraft:bell",
            &[
                ("attachment", attachment),
                ("facing", facing),
                ("powered", "false"),
            ],
        ));
        assert_eq!(
            state_for_placement(item, placement, vanilla_tags(), &mut |at| {
                if at == support {
                    stone()
                } else {
                    air()
                }
            }),
            expected
        );
    }

    let west = relative(CLICKED, Direction::West);
    let east = relative(CLICKED, Direction::East);
    let mut wall = context(CLICKED);
    wall.clicked_face = Direction::East;
    assert_eq!(
        state_for_placement(item, wall, vanilla_tags(), &mut |at| {
            if matches!(at, value if value == west || value == east) {
                stone()
            } else {
                air()
            }
        }),
        Some(state_of(
            "minecraft:bell",
            &[
                ("attachment", "double_wall"),
                ("facing", "west"),
                ("powered", "false"),
            ]
        ))
    );

    assert_eq!(
        state_for_placement(item, wall, vanilla_tags(), &mut |at| {
            if at == below(CLICKED) {
                stone()
            } else {
                air()
            }
        }),
        Some(state_of(
            "minecraft:bell",
            &[
                ("attachment", "floor"),
                ("facing", "west"),
                ("powered", "false"),
            ]
        ))
    );

    assert_eq!(
        state_for_placement(item, wall, vanilla_tags(), &mut |at| {
            if at == above(CLICKED) {
                stone()
            } else {
                air()
            }
        }),
        Some(state_of(
            "minecraft:bell",
            &[
                ("attachment", "ceiling"),
                ("facing", "west"),
                ("powered", "false"),
            ]
        ))
    );
}

#[test]
fn face_attached_family_placement_uses_nearest_supported_face() {
    let supported = ["minecraft:lever", "minecraft:stone_button"];
    for name in supported {
        let item = item_id(name);

        assert_eq!(
            state_for_placement_with_view(
                item,
                context(CLICKED),
                placement_view(0.0, 0.0),
                vanilla_tags(),
                &mut |at| {
                    if at == below(CLICKED) {
                        stone()
                    } else {
                        air()
                    }
                },
                &mut |_, _| Some(true),
            ),
            Some(state_of(
                name,
                &[("face", "floor"), ("facing", "south"), ("powered", "false"),]
            )),
            "view-aware {name}"
        );
    }

    let lever = item_id("minecraft:lever");
    let south = relative(CLICKED, Direction::South);
    let mut wall = context(CLICKED);
    wall.clicked_face = Direction::East;
    assert_eq!(
        state_for_placement_with_view(
            lever,
            wall,
            placement_view(0.0, 0.0),
            vanilla_tags(),
            &mut |at| {
                if at == south {
                    stone()
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        ),
        Some(state_of(
            "minecraft:lever",
            &[("face", "wall"), ("facing", "north"), ("powered", "false"),]
        ))
    );

    let mut ceiling = context(CLICKED);
    ceiling.clicked_face = Direction::Down;
    ceiling.horizontal_direction = Direction::East;
    assert_eq!(
        state_for_placement_with_view(
            lever,
            ceiling,
            placement_view(0.0, 0.0),
            vanilla_tags(),
            &mut |at| {
                if at == above(CLICKED) {
                    stone()
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        ),
        Some(state_of(
            "minecraft:lever",
            &[
                ("face", "ceiling"),
                ("facing", "east"),
                ("powered", "false"),
            ]
        ))
    );
}

#[test]
fn end_rod_placement_faces_the_clicked_face_and_flips_against_matching_rods() {
    let item = item_id("minecraft:end_rod");
    for clicked_face in DIRECTION_VALUES {
        let mut placement = context(CLICKED);
        placement.clicked_face = clicked_face;
        let expected = state_of(
            "minecraft:end_rod",
            &[("facing", direction_serialized_name(clicked_face))],
        );
        assert_eq!(
            state_for_placement(item, placement, vanilla_tags(), &mut |_| stone()),
            Some(expected),
            "known non-rod behind {clicked_face:?}"
        );

        let matching_rod = state_of(
            "minecraft:end_rod",
            &[("facing", direction_serialized_name(clicked_face))],
        );
        let flipped = state_of(
            "minecraft:end_rod",
            &[("facing", direction_serialized_name(opposite(clicked_face)))],
        );
        assert_eq!(
            state_for_placement(item, placement, vanilla_tags(), &mut |_| matching_rod),
            Some(flipped),
            "matching rod behind {clicked_face:?}"
        );
    }
}

#[test]
fn trapdoor_placement_sets_orientation_signal_and_water() {
    let item = item_id("minecraft:oak_trapdoor");
    let mut side = context(CLICKED);
    side.clicked_face = Direction::West;
    side.horizontal_direction = Direction::Up;
    side.click_y = 0.5;
    assert_eq!(
        state_for_placement(item, side, vanilla_tags(), &mut |at| {
            if at == CLICKED {
                water(0)
            } else {
                air()
            }
        }),
        Some(state_of(
            "minecraft:oak_trapdoor",
            &[
                ("facing", "west"),
                ("half", "bottom"),
                ("open", "false"),
                ("powered", "false"),
                ("waterlogged", "true"),
            ]
        )),
        "the horizontal-face branch does not consult player direction"
    );

    side.click_y = 0.500_001;
    let state = state_for_placement(item, side, vanilla_tags(), &mut |at| {
        if at == below(CLICKED) {
            default_state_of("minecraft:redstone_block")
        } else {
            water(1)
        }
    });
    assert_eq!(
        state,
        Some(state_of(
            "minecraft:oak_trapdoor",
            &[
                ("facing", "west"),
                ("half", "top"),
                ("open", "true"),
                ("powered", "true"),
                ("waterlogged", "false"),
            ]
        )),
        "signal mutates OPEN then POWERED and flowing water stays false"
    );

    for (clicked_face, half) in [
        (Direction::Up, "bottom"),
        (Direction::Down, "top"),
        (Direction::East, "top"),
    ] {
        let mut placement = context(CLICKED);
        placement.clicked_face = clicked_face;
        placement.replacing_clicked = clicked_face == Direction::East;
        placement.horizontal_direction = Direction::West;
        let state = state_for_placement(item, placement, vanilla_tags(), &mut |at| {
            if at == CLICKED {
                water(0)
            } else {
                air()
            }
        });
        assert_eq!(
            state,
            Some(state_of(
                "minecraft:oak_trapdoor",
                &[
                    ("facing", "east"),
                    ("half", half),
                    ("open", "false"),
                    ("powered", "false"),
                    ("waterlogged", "true"),
                ]
            )),
            "{clicked_face:?}"
        );
    }
}

#[test]
fn anvil_placement_faces_clockwise_from_the_player() {
    let name = "minecraft:anvil";
    for (horizontal_direction, facing) in [
        (Direction::North, "east"),
        (Direction::East, "south"),
        (Direction::South, "west"),
        (Direction::West, "north"),
    ] {
        let mut placement = context(CLICKED);
        placement.horizontal_direction = horizontal_direction;
        assert_eq!(
            state_for_placement(item_id(name), placement, vanilla_tags(), &mut |_| air()),
            Some(state_of(name, &[("facing", facing)])),
            "{horizontal_direction:?}"
        );
    }
}

/// The view vector `Direction.orderedByNearest` orders by, as
/// `Entity.calculateViewVector` (`Entity.java:1872-1880`) builds it — computed
/// here with the standard library rather than with the `Mth` table
/// [`ordered_by_nearest`] itself reads, so this is a second opinion and not a
/// second copy.
fn view_vector(x_rot: f32, y_rot: f32) -> [f64; 3] {
    let pitch = f64::from(x_rot).to_radians();
    let yaw = -f64::from(y_rot).to_radians();
    [
        yaw.sin() * pitch.cos(),
        -pitch.sin(),
        yaw.cos() * pitch.cos(),
    ]
}

fn step(direction: Direction) -> [f64; 3] {
    match direction {
        Direction::Down => [0.0, -1.0, 0.0],
        Direction::Up => [0.0, 1.0, 0.0],
        Direction::North => [0.0, 0.0, -1.0],
        Direction::South => [0.0, 0.0, 1.0],
        Direction::West => [-1.0, 0.0, 0.0],
        Direction::East => [1.0, 0.0, 0.0],
    }
}

/// `Direction.orderedByNearest` (`Direction.java:86-115`) is a branchy
/// optimisation of one rule: the six faces in decreasing order of how far the
/// view vector points along each. This checks it against that rule, sorted
/// independently, on angles far enough from a tie that the two ways of
/// computing a sine cannot disagree.
///
/// Every consumer in this module takes either `[0]` or the whole array, so the
/// order *is* the behaviour — a wall torch, a lantern and a shelf each attach
/// to the first face in it that will hold them.
#[test]
fn ordered_by_nearest_is_the_six_faces_sorted_by_the_view_vector() {
    for x_rot in [-77.0_f32, -41.0, -13.0, 7.0, 29.0, 63.0] {
        for y_rot in [-143.0_f32, -71.0, -17.0, 23.0, 59.0, 118.0, 167.0] {
            let ordered =
                ordered_by_nearest(view(x_rot, y_rot)).expect("a finite view answers an order");
            let vector = view_vector(x_rot, y_rot);
            let mut expected = [
                Direction::Down,
                Direction::Up,
                Direction::North,
                Direction::South,
                Direction::West,
                Direction::East,
            ];
            expected.sort_by(|left, right| {
                let along = |direction| {
                    let normal = step(direction);
                    vector[0] * normal[0] + vector[1] * normal[1] + vector[2] * normal[2]
                };
                along(*right)
                    .partial_cmp(&along(*left))
                    .expect("finite dot products")
            });
            assert_eq!(ordered, expected, "x_rot {x_rot} y_rot {y_rot}");
            // `makeDirectionArray` (`Direction.java:117-119`) pairs each face
            // with its opposite across the middle.
            for index in 0..6 {
                assert_eq!(ordered[index], opposite(ordered[5 - index]));
            }
        }
    }
}

/// `BlockPlaceContext.getNearestLookingDirections`
/// (`BlockPlaceContext.java:71-89`): replacing the clicked block leaves the
/// view order alone, and placing *against* a face moves that face's opposite to
/// the front while everything ahead of it shifts back one — the `System.arraycopy`
/// at `:85-86`, which is a rotate and not a swap.
#[test]
fn a_placement_against_a_face_moves_that_face_to_the_front_and_shifts_the_rest() {
    let looking = view(-30.0, 40.0);
    let base = ordered_by_nearest(looking).expect("a finite view answers an order");

    let mut replacing = context(CLICKED);
    replacing.replacing_clicked = true;
    replacing.clicked_face = Direction::Down;
    assert_eq!(nearest_looking_directions(replacing, looking), Some(base));

    for clicked_face in [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ] {
        let mut against = context(CLICKED);
        against.replacing_clicked = false;
        against.clicked_face = clicked_face;
        let moved =
            nearest_looking_directions(against, looking).expect("a finite view answers an order");

        let preferred = opposite(clicked_face);
        assert_eq!(moved[0], preferred, "clicked {clicked_face:?}");
        let mut rest: Vec<Direction> = base.into_iter().filter(|d| *d != preferred).collect();
        rest.insert(0, preferred);
        assert_eq!(moved.to_vec(), rest, "clicked {clicked_face:?}");
    }
}
