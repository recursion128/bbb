use super::fixture::*;

#[test]
fn tripwire_placement_connects_to_wires_and_facing_hooks() {
    let item = item_id("minecraft:string");
    let wire = default_state_of("minecraft:tripwire");
    let east_hook = state_of(
        "minecraft:tripwire_hook",
        &[
            ("attached", "false"),
            ("facing", "west"),
            ("powered", "false"),
        ],
    );
    let wrong_south_hook = state_of(
        "minecraft:tripwire_hook",
        &[
            ("attached", "false"),
            ("facing", "south"),
            ("powered", "false"),
        ],
    );
    let expected = state_of(
        "minecraft:tripwire",
        &[
            ("attached", "false"),
            ("disarmed", "false"),
            ("east", "true"),
            ("north", "true"),
            ("powered", "false"),
            ("south", "false"),
            ("west", "false"),
        ],
    );
    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == relative(CLICKED, Direction::North) {
                wire
            } else if at == relative(CLICKED, Direction::East) {
                east_hook
            } else if at == relative(CLICKED, Direction::South) {
                wrong_south_hook
            } else {
                stone()
            }
        }),
        Some(expected)
    );
}

#[test]
fn huge_mushroom_placement_connects_only_to_itself() {
    let name = "minecraft:brown_mushroom_block";
    let other = default_state_of("minecraft:red_mushroom_block");
    let own = default_state_of(name);
    let placed = state_for_placement(item_id(name), context(CLICKED), vanilla_tags(), &mut |at| {
        if at == below(CLICKED) || at == relative(CLICKED, Direction::East) {
            own
        } else if at == above(CLICKED) {
            other
        } else {
            air()
        }
    });
    assert_eq!(
        placed,
        Some(state_of(
            name,
            &[
                ("down", "false"),
                ("east", "false"),
                ("north", "true"),
                ("south", "true"),
                ("up", "true"),
                ("west", "true"),
            ]
        ))
    );
}

#[test]
fn chest_placement_auto_connects_clockwise_before_counterclockwise() {
    let item = item_id("minecraft:chest");
    let east = relative(CLICKED, Direction::East);
    let west = relative(CLICKED, Direction::West);
    let partner = chest_state("minecraft:chest", "north", "single", false);

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == CLICKED {
                water(0)
            } else if at == east || at == west {
                partner
            } else {
                air()
            }
        }),
        Some(chest_state("minecraft:chest", "north", "left", true)),
    );

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == CLICKED {
                water(8)
            } else if at == east {
                stone()
            } else if at == west {
                partner
            } else {
                air()
            }
        }),
        Some(chest_state("minecraft:chest", "north", "right", false)),
    );

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == east {
                chest_state("minecraft:trapped_chest", "north", "single", false)
            } else {
                air()
            }
        }),
        Some(chest_state("minecraft:chest", "north", "single", false)),
    );
}

#[test]
fn chest_placement_secondary_use_only_checks_the_clicked_side() {
    let item = item_id("minecraft:trapped_chest");
    let east = relative(CLICKED, Direction::East);
    let partner = chest_state("minecraft:trapped_chest", "north", "single", false);
    let mut placement = context(CLICKED);
    placement.horizontal_direction = Direction::East;
    placement.clicked_face = Direction::West;
    placement.secondary_use_active = true;

    assert_eq!(
        state_for_placement(item, placement, vanilla_tags(), &mut |at| {
            if at == east {
                partner
            } else {
                air()
            }
        }),
        Some(chest_state(
            "minecraft:trapped_chest",
            "north",
            "left",
            false,
        )),
    );

    placement.clicked_face = Direction::Up;
    assert_eq!(
        state_for_placement(item, placement, vanilla_tags(), &mut |at| {
            if at == east {
                partner
            } else {
                air()
            }
        }),
        Some(chest_state(
            "minecraft:trapped_chest",
            "west",
            "single",
            false
        )),
    );
}

#[test]
fn copper_chest_placement_projects_both_halves_to_the_least_oxidized_variant() {
    let east = relative(CLICKED, Direction::East);
    for (placing, partner, expected) in [
        (
            "minecraft:oxidized_copper_chest",
            "minecraft:exposed_copper_chest",
            "minecraft:exposed_copper_chest",
        ),
        (
            "minecraft:waxed_copper_chest",
            "minecraft:oxidized_copper_chest",
            "minecraft:copper_chest",
        ),
    ] {
        let partner_state = chest_state(partner, "north", "single", false);
        assert_eq!(
            state_for_placement(
                item_id(placing),
                context(CLICKED),
                vanilla_tags(),
                &mut |at| {
                    if at == east {
                        partner_state
                    } else {
                        air()
                    }
                }
            ),
            Some(chest_state(expected, "north", "left", false)),
            "{placing} beside {partner}",
        );
    }
}

#[test]
fn iron_bars_placement_connects_to_panes_walls_and_solid_faces() {
    let item = item_id("minecraft:iron_bars");
    let north = relative(CLICKED, Direction::North);
    let south = relative(CLICKED, Direction::South);
    let west = relative(CLICKED, Direction::West);
    let east = relative(CLICKED, Direction::East);
    let placed = state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
        if at == CLICKED {
            water(0)
        } else if at == north {
            default_state_of("minecraft:glass_pane")
        } else if at == south {
            default_state_of("minecraft:cobblestone_wall")
        } else if at == west {
            stone()
        } else if at == east {
            default_state_of("minecraft:barrier")
        } else {
            air()
        }
    });
    assert_eq!(
        placed,
        Some(state_of(
            "minecraft:iron_bars",
            &[
                ("east", "false"),
                ("north", "true"),
                ("south", "true"),
                ("waterlogged", "true"),
                ("west", "true"),
            ]
        ))
    );
}

#[test]
fn fence_placement_connects_to_same_family_fences_gates_and_solid_faces() {
    let oak_item = item_id("minecraft:oak_fence");
    let north = relative(CLICKED, Direction::North);
    let east = relative(CLICKED, Direction::East);
    let south = relative(CLICKED, Direction::South);
    let west = relative(CLICKED, Direction::West);
    let oak_gate = |facing| {
        with_property(
            default_state_of("minecraft:oak_fence_gate"),
            "facing",
            facing,
        )
        .expect("every fence gate state carries `facing`")
    };
    let placed = state_for_placement(oak_item, context(CLICKED), vanilla_tags(), &mut |at| {
        if at == CLICKED {
            water(0)
        } else if at == north {
            default_state_of("minecraft:oak_fence")
        } else if at == east {
            default_state_of("minecraft:nether_brick_fence")
        } else if at == south {
            oak_gate("east")
        } else if at == west {
            stone()
        } else {
            air()
        }
    });
    assert_eq!(
        placed,
        Some(state_of(
            "minecraft:oak_fence",
            &[
                ("east", "false"),
                ("north", "true"),
                ("south", "true"),
                ("waterlogged", "true"),
                ("west", "true"),
            ]
        ))
    );

    for neighbour in [default_state_of("minecraft:barrier"), oak_gate("north")] {
        let placed = state_for_placement(oak_item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == north {
                neighbour
            } else {
                air()
            }
        });
        assert_eq!(placed, Some(default_state_of("minecraft:oak_fence")));
    }

    let nether_item = item_id("minecraft:nether_brick_fence");
    let placed = state_for_placement(nether_item, context(CLICKED), vanilla_tags(), &mut |at| {
        if at == north {
            default_state_of("minecraft:oak_fence")
        } else if at == east {
            default_state_of("minecraft:nether_brick_fence")
        } else {
            air()
        }
    });
    assert_eq!(
        placed,
        with_property(
            default_state_of("minecraft:nether_brick_fence"),
            "east",
            "true"
        )
    );
}

#[test]
fn fence_gate_placement_opens_when_powered_and_lowers_between_walls() {
    let name = "minecraft:oak_fence_gate";
    assert_eq!(
        state_for_placement(item_id(name), context(CLICKED), vanilla_tags(), &mut |_| {
            air()
        }),
        Some(state_of(
            name,
            &[
                ("facing", "south"),
                ("in_wall", "false"),
                ("open", "false"),
                ("powered", "false"),
            ]
        ))
    );

    let mut placement = context(CLICKED);
    placement.horizontal_direction = Direction::West;
    assert_eq!(
        state_for_placement(item_id(name), placement, vanilla_tags(), &mut |at| {
            if at == below(CLICKED) {
                default_state_of("minecraft:redstone_block")
            } else if at == relative(CLICKED, Direction::North) {
                default_state_of("minecraft:cobblestone_wall")
            } else {
                air()
            }
        }),
        Some(state_of(
            name,
            &[
                ("facing", "west"),
                ("in_wall", "true"),
                ("open", "true"),
                ("powered", "true"),
            ]
        ))
    );
}

#[test]
fn wall_placement_sets_side_heights_and_post_from_neighbours() {
    let item = item_id("minecraft:cobblestone_wall");
    let north = relative(CLICKED, Direction::North);
    let east = relative(CLICKED, Direction::East);
    let south = relative(CLICKED, Direction::South);
    let west = relative(CLICKED, Direction::West);
    let top = above(CLICKED);
    let gate = with_property(
        default_state_of("minecraft:oak_fence_gate"),
        "facing",
        "east",
    )
    .expect("every fence gate state carries `facing`");
    let bottom_slab = state_of(
        "minecraft:stone_slab",
        &[("type", "bottom"), ("waterlogged", "false")],
    );
    let placed = state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
        if at == CLICKED {
            water(0)
        } else if at == north {
            default_state_of("minecraft:brick_wall")
        } else if at == east {
            default_state_of("minecraft:iron_bars")
        } else if at == south {
            gate
        } else if at == west {
            stone()
        } else if at == top {
            bottom_slab
        } else {
            air()
        }
    });
    assert_eq!(
        placed,
        Some(state_of(
            "minecraft:cobblestone_wall",
            &[
                ("east", "tall"),
                ("north", "tall"),
                ("south", "tall"),
                ("up", "false"),
                ("waterlogged", "true"),
                ("west", "tall"),
            ]
        ))
    );

    let placed = state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
        if at == north {
            default_state_of("minecraft:brick_wall")
        } else if at == east {
            default_state_of("minecraft:barrier")
        } else {
            air()
        }
    });
    assert_eq!(
        placed,
        Some(state_of(
            "minecraft:cobblestone_wall",
            &[
                ("east", "none"),
                ("north", "low"),
                ("south", "none"),
                ("up", "true"),
                ("waterlogged", "false"),
                ("west", "none"),
            ]
        ))
    );
}

#[test]
fn chorus_plant_placement_connects_and_survives_on_end_stone_or_a_side_plant() {
    let item = item_id("minecraft:chorus_plant");
    let down = relative(CLICKED, Direction::Down);
    let up = relative(CLICKED, Direction::Up);
    let north = relative(CLICKED, Direction::North);
    let north_below = relative(north, Direction::Down);
    let end_stone = default_state_of("minecraft:end_stone");
    let flower = default_state_of("minecraft:chorus_flower");
    let plant = default_state_of("minecraft:chorus_plant");

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == down {
                end_stone
            } else if at == up {
                flower
            } else {
                air()
            }
        }),
        Some(chorus_plant_state(&[Direction::Down, Direction::Up]))
    );

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == north {
                plant
            } else if at == north_below {
                end_stone
            } else {
                air()
            }
        }),
        Some(chorus_plant_state(&[Direction::North]))
    );
}

#[test]
fn creaking_heart_placement_takes_the_clicked_axis() {
    let item = item_id("minecraft:creaking_heart");
    for (clicked_face, axis) in [
        (Direction::East, "x"),
        (Direction::Up, "y"),
        (Direction::North, "z"),
    ] {
        let mut placement = context(CLICKED);
        placement.clicked_face = clicked_face;
        assert_eq!(
            state_for_placement(item, placement, vanilla_tags(), &mut |_| air()),
            Some(state_of(
                "minecraft:creaking_heart",
                &[
                    ("axis", axis),
                    ("creaking_heart_state", "uprooted"),
                    ("natural", "false"),
                ],
            ))
        );
    }
}
