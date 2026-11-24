use super::fixture::*;

#[test]
fn banner_item_places_a_wall_banner_or_a_rotated_standing_banner() {
    let item = item_id("minecraft:white_banner");
    let west = relative(CLICKED, Direction::West);
    let mut replacement = context(CLICKED);
    replacement.replacing_clicked = true;

    let wall = state_for_placement_with_view(
        item,
        replacement,
        view(0.0, 0.0),
        vanilla_tags(),
        &mut |at| {
            if at == west {
                stone()
            } else {
                air()
            }
        },
        &mut |_, _| panic!("banner collision is empty"),
    );
    assert_eq!(
        wall,
        Some(state_of(
            "minecraft:white_wall_banner",
            &[("facing", "east")]
        ))
    );

    assert_eq!(
        state_for_placement_with_view(
            item,
            context(CLICKED),
            view(0.0, 90.0),
            vanilla_tags(),
            &mut |at| {
                if at == west || at == below(CLICKED) {
                    stone()
                } else {
                    air()
                }
            },
            &mut |_, _| panic!("banner collision is empty"),
        ),
        Some(state_of("minecraft:white_banner", &[("rotation", "12")]))
    );

    assert_eq!(
        can_replace_for_placement(item, air(), replacement),
        Some(true)
    );
    assert_eq!(
        can_replace_for_placement(item, stone(), replacement),
        Some(false)
    );
}

#[test]
fn standing_and_wall_torches_follow_view_and_support() {
    let item = item_id("minecraft:torch");
    let north = relative(CLICKED, Direction::North);
    let mut replacement = context(CLICKED);
    replacement.replacing_clicked = true;
    let wall = state_for_placement_with_view(
        item,
        replacement,
        view(0.0, 0.0),
        vanilla_tags(),
        &mut |at| {
            if at == north || at == below(CLICKED) {
                stone()
            } else {
                air()
            }
        },
        &mut |_, _| Some(true),
    );
    assert_eq!(
        wall,
        Some(state_of("minecraft:wall_torch", &[("facing", "south")]))
    );

    replacement.replacing_clicked = false;
    let standing = state_for_placement_with_view(
        item,
        replacement,
        view(0.0, 0.0),
        vanilla_tags(),
        &mut |at| {
            if at == north || at == below(CLICKED) {
                stone()
            } else {
                air()
            }
        },
        &mut |_, _| Some(true),
    );
    assert_eq!(standing, Some(default_state_of("minecraft:torch")));

    replacement.replacing_clicked = true;
    assert_eq!(
        state_for_placement_with_view(
            item,
            replacement,
            view(0.0, 0.0),
            vanilla_tags(),
            &mut |_| air(),
            &mut |_, _| Some(true),
        ),
        None,
        "neither standing nor wall torch is supported"
    );

    let redstone = state_for_placement_with_view(
        item_id("minecraft:redstone_torch"),
        replacement,
        view(0.0, 0.0),
        vanilla_tags(),
        &mut |at| if at == north { stone() } else { air() },
        &mut |_, _| Some(true),
    );
    assert_eq!(
        redstone,
        Some(state_of(
            "minecraft:redstone_wall_torch",
            &[("facing", "south"), ("lit", "true")]
        ))
    );
}

#[test]
fn standing_and_wall_coral_fans_waterlog_in_full_water() {
    let item = item_id("minecraft:tube_coral_fan");
    let north = relative(CLICKED, Direction::North);
    let mut replacement = context(CLICKED);
    replacement.replacing_clicked = true;
    for (water_state, waterlogged) in [(water(8), "true"), (water(7), "false")] {
        assert_eq!(
            state_for_placement_with_view(
                item,
                replacement,
                view(0.0, 0.0),
                vanilla_tags(),
                &mut |at| {
                    if at == CLICKED {
                        water_state
                    } else if at == north {
                        stone()
                    } else {
                        air()
                    }
                },
                &mut |_, _| Some(true),
            ),
            Some(state_of(
                "minecraft:tube_coral_wall_fan",
                &[("facing", "south"), ("waterlogged", waterlogged)]
            ))
        );
    }

    replacement.replacing_clicked = false;
    assert_eq!(
        state_for_placement_with_view(
            item,
            replacement,
            view(0.0, 0.0),
            vanilla_tags(),
            &mut |at| {
                if at == CLICKED {
                    water(8)
                } else if at == north || at == below(CLICKED) {
                    stone()
                } else {
                    air()
                }
            },
            &mut |_, _| Some(true),
        ),
        Some(state_of(
            "minecraft:tube_coral_fan",
            &[("waterlogged", "true")]
        ))
    );
}

#[test]
fn standing_and_wall_heads_set_power_rotation_and_wall_facing() {
    let item = item_id("minecraft:player_head");
    let mut replacement = context(CLICKED);
    replacement.replacing_clicked = true;
    let standing = state_for_placement_with_view(
        item,
        replacement,
        view(90.0, 135.0),
        vanilla_tags(),
        &mut |at| {
            if at == below(CLICKED) {
                default_state_of("minecraft:redstone_block")
            } else {
                air()
            }
        },
        &mut |_, _| Some(true),
    );
    assert_eq!(
        standing,
        Some(state_of(
            "minecraft:player_head",
            &[("powered", "true"), ("rotation", "6")]
        ))
    );

    let north = relative(CLICKED, Direction::North);
    let wall = state_for_placement_with_view(
        item,
        replacement,
        view(0.0, 0.0),
        vanilla_tags(),
        &mut |at| {
            if at == below(CLICKED) {
                default_state_of("minecraft:redstone_block")
            } else if at == north {
                stone()
            } else {
                air()
            }
        },
        &mut |_, _| Some(true),
    );
    assert_eq!(
        wall,
        Some(state_of(
            "minecraft:player_wall_head",
            &[("facing", "south"), ("powered", "true")]
        ))
    );
    assert_eq!(
        state_for_placement_with_view(
            item,
            replacement,
            view(90.0, 135.0),
            vanilla_tags(),
            &mut |at| {
                if at == below(CLICKED) {
                    default_state_of("minecraft:redstone_block")
                } else {
                    air()
                }
            },
            &mut |_, _| Some(false),
        ),
        None,
        "a nonempty head collision shape obeys the final obstruction gate"
    );
}
