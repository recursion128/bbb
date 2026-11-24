use super::fixture::*;

#[test]
fn bamboo_placement_refuses_fluids_and_takes_age_from_its_column() {
    let item = item_id("minecraft:bamboo");
    let bamboo_age = |age| {
        state_of(
            "minecraft:bamboo",
            &[("age", age), ("leaves", "none"), ("stage", "0")],
        )
    };

    for replaced in [water(0), water(1), default_state_of("minecraft:lava")] {
        assert_eq!(
            state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
                if at == CLICKED {
                    replaced
                } else {
                    air()
                }
            }),
            None
        );
    }

    for (support, expected_age) in [
        (default_state_of("minecraft:bamboo_sapling"), "0"),
        (bamboo_age("0"), "0"),
        (bamboo_age("1"), "1"),
    ] {
        assert_eq!(
            state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
                if at == below(CLICKED) {
                    support
                } else {
                    air()
                }
            }),
            Some(bamboo_age(expected_age))
        );
    }

    let dirt = default_state_of("minecraft:dirt");
    for (above_state, expected) in [
        (bamboo_age("0"), bamboo_age("0")),
        (bamboo_age("1"), bamboo_age("1")),
        (stone(), default_state_of("minecraft:bamboo_sapling")),
    ] {
        assert_eq!(
            state_for_placement_with_view(
                item,
                context(CLICKED),
                placement_view(0.0, 0.0),
                vanilla_tags(),
                &mut |at| {
                    if at == below(CLICKED) {
                        dirt
                    } else if at == above(CLICKED) {
                        above_state
                    } else {
                        air()
                    }
                },
                &mut |_, _| Some(true),
            ),
            Some(expected)
        );
    }
}

#[test]
fn concrete_powder_placement_solidifies_beside_water() {
    let item = item_id("minecraft:white_concrete_powder");
    let powder = default_state_of("minecraft:white_concrete_powder");
    let concrete = default_state_of("minecraft:white_concrete");
    let up = relative(CLICKED, Direction::Up);
    let north = relative(CLICKED, Direction::North);

    for replaced in [water(0), water(1)] {
        let state = state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == CLICKED {
                replaced
            } else {
                air()
            }
        });
        assert_eq!(state, Some(concrete));
    }

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |_| air()),
        Some(powder)
    );

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == north {
                water(0)
            } else {
                air()
            }
        }),
        Some(concrete)
    );

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == below(CLICKED) {
                water(0)
            } else {
                air()
            }
        }),
        Some(powder),
        "DOWN never moves the mutable cursor when the current cell is dry"
    );

    let sturdy_water = state_of(
        "minecraft:stone_slab",
        &[("type", "bottom"), ("waterlogged", "true")],
    );
    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == up {
                sturdy_water
            } else {
                air()
            }
        }),
        Some(powder),
        "water behind a sturdy DOWN face does not solidify powder"
    );
}

#[test]
fn sculk_sensor_family_sets_water_and_calibrated_facing() {
    let sensor = item_id("minecraft:sculk_sensor");
    assert_eq!(
        state_for_placement(sensor, context(CLICKED), vanilla_tags(), &mut |_| water(0)),
        Some(state_of(
            "minecraft:sculk_sensor",
            &[
                ("power", "0"),
                ("sculk_sensor_phase", "inactive"),
                ("waterlogged", "true"),
            ]
        ))
    );

    let calibrated = item_id("minecraft:calibrated_sculk_sensor");
    {
        let direction = Direction::West;
        let mut placement = context(CLICKED);
        placement.horizontal_direction = direction;
        let expected = state_of(
            "minecraft:calibrated_sculk_sensor",
            &[
                ("facing", direction_serialized_name(direction)),
                ("power", "0"),
                ("sculk_sensor_phase", "inactive"),
                ("waterlogged", "false"),
            ],
        );
        assert_eq!(
            state_for_placement(calibrated, placement, vanilla_tags(), &mut |_| water(1)),
            Some(expected),
            "{direction:?}"
        );
    }
}

#[test]
fn mangrove_propagule_placement_sets_age_and_water_and_needs_support() {
    let item = item_id("minecraft:mangrove_propagule");
    let clay = default_state_of("minecraft:clay");
    let expected = state_of(
        "minecraft:mangrove_propagule",
        &[
            ("age", "4"),
            ("hanging", "false"),
            ("stage", "0"),
            ("waterlogged", "true"),
        ],
    );
    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == CLICKED {
                water(0)
            } else {
                clay
            }
        }),
        Some(expected)
    );

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == CLICKED {
                water(1)
            } else {
                clay
            }
        }),
        Some(state_of(
            "minecraft:mangrove_propagule",
            &[
                ("age", "4"),
                ("hanging", "false"),
                ("stage", "0"),
                ("waterlogged", "false"),
            ]
        )),
        "flowing water is not the Fluids.WATER source identity"
    );

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |_| air()),
        None,
        "unsupported below state refuses placement"
    );
}

#[test]
fn mossy_carpet_placement_copies_sides_from_above_and_needs_support() {
    let item = item_id("minecraft:pale_moss_carpet");
    let north = relative(CLICKED, Direction::North);
    let up = above(CLICKED);
    let down = below(CLICKED);
    let topper = state_of(
        "minecraft:pale_moss_carpet",
        &[
            ("bottom", "false"),
            ("east", "none"),
            ("north", "low"),
            ("south", "none"),
            ("west", "none"),
        ],
    );
    let expected = state_of(
        "minecraft:pale_moss_carpet",
        &[
            ("bottom", "true"),
            ("east", "none"),
            ("north", "tall"),
            ("south", "none"),
            ("west", "none"),
        ],
    );
    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == north || at == down {
                stone()
            } else if at == up {
                topper
            } else {
                air()
            }
        }),
        Some(expected),
    );

    assert_eq!(
        state_for_placement_with_view(
            item,
            context(CLICKED),
            view(-38.0, 117.0),
            vanilla_tags(),
            &mut |at| {
                if at == down {
                    stone()
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        ),
        Some(state_of(
            "minecraft:pale_moss_carpet",
            &[
                ("bottom", "true"),
                ("east", "none"),
                ("north", "none"),
                ("south", "none"),
                ("west", "none"),
            ],
        )),
    );
}

#[test]
fn seagrass_placement_requires_full_water_and_support() {
    let item = item_id("minecraft:seagrass");
    let dirt = default_state_of("minecraft:dirt");
    for full_water in [water(0), water(8)] {
        assert_eq!(
            state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
                if at == CLICKED {
                    full_water
                } else {
                    dirt
                }
            }),
            Some(default_state_of("minecraft:seagrass"))
        );
    }

    for rejected_fluid in [water(7), air(), default_state_of("minecraft:lava")] {
        assert_eq!(
            state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |_| {
                rejected_fluid
            }),
            None
        );
    }

    for support in [default_state_of("minecraft:magma_block"), air()] {
        assert_eq!(
            state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
                if at == CLICKED {
                    water(0)
                } else {
                    support
                }
            }),
            None
        );
    }
}

#[test]
fn big_dripleaf_placement_copies_the_column_or_faces_the_player() {
    let item = item_id("minecraft:big_dripleaf");
    let dirt = default_state_of("minecraft:dirt");
    let expected = state_of(
        "minecraft:big_dripleaf",
        &[
            ("facing", "north"),
            ("tilt", "none"),
            ("waterlogged", "true"),
        ],
    );
    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == below(CLICKED) {
                dirt
            } else {
                water(0)
            }
        }),
        Some(expected)
    );

    let stem = state_of(
        "minecraft:big_dripleaf_stem",
        &[("facing", "east"), ("waterlogged", "false")],
    );
    let mut on_column = context(CLICKED);
    on_column.horizontal_direction = Direction::Up;
    assert_eq!(
        state_for_placement(item, on_column, vanilla_tags(), &mut |at| {
            if at == below(CLICKED) {
                stem
            } else {
                water(1)
            }
        }),
        Some(state_of(
            "minecraft:big_dripleaf",
            &[
                ("facing", "east"),
                ("tilt", "none"),
                ("waterlogged", "false"),
            ]
        )),
        "an existing column supplies facing before the context fallback"
    );
}

#[test]
fn farmland_placement_keeps_supported_ground_or_falls_back_to_dirt() {
    let item = item_id("minecraft:farmland");
    let farmland = state_of("minecraft:farmland", &[("moisture", "0")]);
    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |_| air()),
        Some(farmland)
    );

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |_| stone()),
        Some(default_state_of("minecraft:dirt"))
    );
}

#[test]
fn leaves_placement_sets_persistence_water_and_nearest_distance() {
    let item = item_id("minecraft:oak_leaves");
    let beside_log = state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
        if at == CLICKED {
            water(0)
        } else if at == below(CLICKED) {
            oak_log()
        } else {
            air()
        }
    });
    assert_eq!(beside_log, Some(oak_leaves(1, true, true)));

    let flowing = state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
        if at == CLICKED {
            water(1)
        } else {
            air()
        }
    });
    assert_eq!(flowing, Some(oak_leaves(7, true, false)));

    let two_from_leaf = state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
        if at == below(CLICKED) {
            oak_leaves(4, false, false)
        } else if at == above(CLICKED) {
            oak_leaves(1, false, false)
        } else {
            air()
        }
    });
    assert_eq!(two_from_leaf, Some(oak_leaves(2, true, false)));
}

#[test]
fn rail_placement_sets_cardinal_shape_and_water_and_needs_support() {
    for name in ["minecraft:rail", "minecraft:powered_rail"] {
        let item = item_id(name);
        for (horizontal_direction, shape) in [
            (Direction::North, "north_south"),
            (Direction::South, "north_south"),
            (Direction::West, "east_west"),
            (Direction::East, "east_west"),
        ] {
            let mut placement = context(CLICKED);
            placement.horizontal_direction = horizontal_direction;
            let placed = state_for_placement(item, placement, vanilla_tags(), &mut |at| {
                if at == CLICKED {
                    water(0)
                } else {
                    stone()
                }
            });
            let expected = with_property(default_state_of(name), "shape", shape)
                .and_then(|state| with_property(state, "waterlogged", "true"));
            assert_eq!(placed, expected, "{name} {horizontal_direction:?}");
        }

        let placed = state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == CLICKED {
                water(8)
            } else {
                stone()
            }
        });
        let expected = with_property(default_state_of(name), "shape", "north_south")
            .and_then(|state| with_property(state, "waterlogged", "false"));
        assert_eq!(placed, expected, "{name} flowing amount-eight water");
    }

    assert_eq!(
        state_for_placement(
            item_id("minecraft:rail"),
            context(CLICKED),
            vanilla_tags(),
            &mut |_| air()
        ),
        None
    );
}

#[test]
fn amethyst_cluster_placement_sets_water_and_facing_and_needs_support() {
    for name in ["minecraft:amethyst_cluster"] {
        let item = item_id(name);
        for clicked_face in DIRECTION_VALUES {
            let mut placement = context(CLICKED);
            placement.clicked_face = clicked_face;
            let placed = state_for_placement(item, placement, vanilla_tags(), &mut |at| {
                if at == CLICKED {
                    water(0)
                } else {
                    stone()
                }
            });
            let expected =
                with_property(default_state_of(name), "waterlogged", "true").and_then(|state| {
                    with_property(state, "facing", direction_serialized_name(clicked_face))
                });
            assert_eq!(placed, expected, "{name} {clicked_face:?}");
        }

        let placed = state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == CLICKED {
                water(8)
            } else {
                stone()
            }
        });
        let expected = with_property(default_state_of(name), "waterlogged", "false")
            .and_then(|state| with_property(state, "facing", "up"));
        assert_eq!(placed, expected, "{name} flowing amount-eight water");
    }

    let item = item_id("minecraft:amethyst_cluster");
    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |_| air()),
        None
    );
}

#[test]
fn snowy_placement_follows_snow_above() {
    for name in ["minecraft:grass_block"] {
        for (above_state, snowy) in [
            (default_state_of("minecraft:snow_block"), "true"),
            (default_state_of("minecraft:snow"), "true"),
            (stone(), "false"),
        ] {
            assert_eq!(
                state_for_placement(item_id(name), context(CLICKED), vanilla_tags(), &mut |_| {
                    above_state
                }),
                Some(state_of(name, &[("snowy", snowy)])),
                "{name} above state {above_state}"
            );
        }
    }
}

#[test]
fn coral_plant_placement_waterlogs_in_full_water() {
    let names = ["minecraft:dead_tube_coral", "minecraft:tube_coral"];
    for name in names {
        assert_eq!(
            state_for_placement(item_id(name), context(CLICKED), vanilla_tags(), &mut |at| {
                if at == CLICKED {
                    water(0)
                } else {
                    stone()
                }
            }),
            Some(state_of(name, &[("waterlogged", "true")])),
            "{name}"
        );
    }

    for (replaced, waterlogged) in [(water(8), "true"), (water(7), "false"), (air(), "false")] {
        assert_eq!(
            state_for_placement(
                item_id("minecraft:tube_coral"),
                context(CLICKED),
                vanilla_tags(),
                &mut |at| {
                    if at == CLICKED {
                        replaced
                    } else {
                        stone()
                    }
                },
            ),
            Some(state_of(
                "minecraft:tube_coral",
                &[("waterlogged", waterlogged)]
            )),
            "replaced {replaced}"
        );
    }
}
