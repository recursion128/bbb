use super::fixture::*;

#[test]
fn snow_layer_replacement_and_stacking() {
    let snow_item = item_id("minecraft:snow");
    let stone_item = item_id("minecraft:stone");
    let snow = |layers| state_of("minecraft:snow", &[("layers", layers)]);
    let mut clicked = context(CLICKED);
    clicked.replacing_clicked = true;

    clicked.clicked_face = Direction::Up;
    assert_eq!(
        can_replace_for_placement(snow_item, snow("7"), clicked),
        Some(true)
    );
    clicked.clicked_face = Direction::North;
    assert_eq!(
        can_replace_for_placement(snow_item, snow("7"), clicked),
        Some(false)
    );
    clicked.replacing_clicked = false;
    assert_eq!(
        can_replace_for_placement(snow_item, snow("7"), clicked),
        Some(true)
    );
    assert_eq!(
        can_replace_for_placement(snow_item, snow("8"), clicked),
        Some(false)
    );
    assert_eq!(
        can_replace_for_placement(stone_item, snow("1"), clicked),
        Some(true)
    );
    assert_eq!(
        can_replace_for_placement(stone_item, snow("2"), clicked),
        Some(false)
    );

    let place_on = |target| {
        state_for_placement(snow_item, context(CLICKED), vanilla_tags(), &mut |at| {
            if at == CLICKED {
                target
            } else if at == below(CLICKED) {
                stone()
            } else {
                air()
            }
        })
    };
    for (target, expected) in [
        (snow("1"), snow("2")),
        (snow("8"), snow("8")),
        (stone(), snow("1")),
    ] {
        assert_eq!(place_on(target), Some(expected));
    }

    assert_eq!(
        can_replace_for_placement(snow_item, default_state_of("minecraft:candle"), clicked),
        Some(false)
    );
}

#[test]
fn candle_placement_stacks_only_matching_non_secondary_items() {
    let item = item_id("minecraft:candle");
    let state = |candles| {
        state_of(
            "minecraft:candle",
            &[
                ("candles", candles),
                ("lit", "false"),
                ("waterlogged", "false"),
            ],
        )
    };
    let mut clicked = context(CLICKED);
    clicked.replacing_clicked = true;
    for (candles, next) in [("1", "2"), ("2", "3"), ("3", "4")] {
        assert_eq!(
            can_replace_for_placement(item, state(candles), clicked),
            Some(true),
        );
        assert_eq!(
            state_for_placement(item, clicked, vanilla_tags(), &mut |at| {
                if at == CLICKED {
                    state(candles)
                } else {
                    stone()
                }
            }),
            Some(state(next)),
        );
    }

    assert_eq!(
        can_replace_for_placement(item, state("4"), clicked),
        Some(false),
    );
    clicked.secondary_use_active = true;
    assert_eq!(
        can_replace_for_placement(item, state("1"), clicked),
        Some(false),
    );
    clicked.secondary_use_active = false;
    assert_eq!(
        can_replace_for_placement(item_id("minecraft:white_candle"), state("1"), clicked),
        Some(false),
    );
}

#[test]
fn flower_bed_placement_stacks_or_orients() {
    let dirt = default_state_of("minecraft:dirt");
    for name in ["minecraft:pink_petals"] {
        let item = item_id(name);
        let state =
            |amount, facing| state_of(name, &[("facing", facing), ("flower_amount", amount)]);
        let mut clicked = context(CLICKED);
        clicked.replacing_clicked = true;

        for (amount, next) in [("1", "2"), ("2", "3"), ("3", "4")] {
            assert_eq!(
                can_replace_for_placement(item, state(amount, "east"), clicked),
                Some(true),
            );
            assert_eq!(
                state_for_placement(item, clicked, vanilla_tags(), &mut |at| {
                    if at == CLICKED {
                        state(amount, "east")
                    } else {
                        dirt
                    }
                }),
                Some(state(next, "east")),
            );
        }

        assert_eq!(
            can_replace_for_placement(item, state("4", "east"), clicked),
            Some(false),
        );
        clicked.secondary_use_active = true;
        assert_eq!(
            can_replace_for_placement(item, state("1", "east"), clicked),
            Some(false),
        );

        let mut fresh = context(CLICKED);
        fresh.horizontal_direction = Direction::East;
        assert_eq!(
            state_for_placement(item, fresh, vanilla_tags(), &mut |at| {
                if at == CLICKED {
                    air()
                } else {
                    dirt
                }
            }),
            Some(state("1", "west")),
        );
    }

    let mut clicked = context(CLICKED);
    clicked.replacing_clicked = true;
    assert_eq!(
        can_replace_for_placement(
            item_id("minecraft:wildflowers"),
            state_of(
                "minecraft:pink_petals",
                &[("facing", "north"), ("flower_amount", "1")],
            ),
            clicked,
        ),
        Some(false),
    );
}

#[test]
fn pointed_dripstone_placement_uses_view_fallback_thickness_and_water() {
    let item = item_id("minecraft:pointed_dripstone");
    let below_pos = below(CLICKED);
    let above_pos = above(CLICKED);
    let stone = stone();

    assert_eq!(
        state_for_placement_with_view(
            item,
            context(CLICKED),
            view(0.0, 30.0),
            vanilla_tags(),
            &mut |at| {
                if at == below_pos {
                    stone
                } else if at == CLICKED {
                    water(0)
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        ),
        Some(pointed_dripstone_state("up", "tip", true)),
    );

    assert_eq!(
        state_for_placement_with_view(
            item,
            context(CLICKED),
            view(-20.0, 30.0),
            vanilla_tags(),
            &mut |at| {
                if at == above_pos {
                    stone
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        ),
        Some(pointed_dripstone_state("down", "tip", false)),
    );

    assert_eq!(
        state_for_placement_with_view(
            item,
            context(CLICKED),
            view(0.0, 0.0),
            vanilla_tags(),
            &mut |at| {
                if at == above_pos {
                    stone
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        ),
        Some(pointed_dripstone_state("down", "tip", false)),
    );

    for (secondary_use_active, thickness) in [(false, "tip_merge"), (true, "tip")] {
        let mut placement = context(CLICKED);
        placement.secondary_use_active = secondary_use_active;
        assert_eq!(
            state_for_placement_with_view(
                item,
                placement,
                view(0.0, 0.0),
                vanilla_tags(),
                &mut |at| {
                    if at == below_pos {
                        stone
                    } else if at == above_pos {
                        pointed_dripstone_state("down", "tip", false)
                    } else {
                        water(8)
                    }
                },
                &mut |_, _| Some(true),
            ),
            Some(pointed_dripstone_state("up", thickness, false)),
        );
    }

    assert_eq!(
        state_for_placement_with_view(
            item,
            context(CLICKED),
            view(0.0, 0.0),
            vanilla_tags(),
            &mut |_| air(),
            &mut |_, _| Some(true),
        ),
        None,
    );
}

#[test]
fn sea_pickle_placement_stacks_or_sets_source_water() {
    let item = item_id("minecraft:sea_pickle");
    let stone = stone();
    let mut clicked = context(CLICKED);
    clicked.replacing_clicked = true;

    for (pickles, next) in [("1", "2"), ("2", "3"), ("3", "4")] {
        assert_eq!(
            can_replace_for_placement(item, sea_pickle_state(pickles, true), clicked),
            Some(true),
        );
        assert_eq!(
            state_for_placement(item, clicked, vanilla_tags(), &mut |at| {
                if at == CLICKED {
                    sea_pickle_state(pickles, true)
                } else {
                    stone
                }
            }),
            Some(sea_pickle_state(next, true)),
        );
    }

    assert_eq!(
        can_replace_for_placement(item, sea_pickle_state("4", true), clicked),
        Some(false),
    );
    clicked.secondary_use_active = true;
    assert_eq!(
        can_replace_for_placement(item, sea_pickle_state("1", true), clicked),
        Some(false),
    );

    for (replaced, waterlogged) in [(water(0), true), (water(1), false)] {
        assert_eq!(
            state_for_placement_with_view(
                item,
                context(CLICKED),
                view(20.0, -35.0),
                vanilla_tags(),
                &mut |at| {
                    if at == CLICKED {
                        replaced
                    } else {
                        stone
                    }
                },
                &mut |_, _| Some(true),
            ),
            Some(sea_pickle_state("1", waterlogged)),
        );
    }
}

#[test]
fn turtle_egg_placement_stacks_matching_non_secondary_items() {
    let item = item_id("minecraft:turtle_egg");
    let mut clicked = context(CLICKED);
    clicked.replacing_clicked = true;

    for (eggs, next) in [("1", "2"), ("2", "3"), ("3", "4")] {
        assert_eq!(
            can_replace_for_placement(item, turtle_egg_state(eggs, "2"), clicked),
            Some(true),
        );
        assert_eq!(
            state_for_placement(item, clicked, vanilla_tags(), &mut |_| turtle_egg_state(
                eggs, "2"
            )),
            Some(turtle_egg_state(next, "2")),
        );
    }

    assert_eq!(
        can_replace_for_placement(item, turtle_egg_state("4", "1"), clicked),
        Some(false),
    );
    clicked.secondary_use_active = true;
    assert_eq!(
        can_replace_for_placement(item, turtle_egg_state("1", "1"), clicked),
        Some(false),
    );

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |_| air()),
        Some(turtle_egg_state("1", "0")),
    );
}
