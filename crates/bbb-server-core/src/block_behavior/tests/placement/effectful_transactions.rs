use super::fixture::*;

#[test]
fn the_redstone_lamp_item_reads_the_signal_at_the_cell_it_lands_in() {
    // `RedstoneLampBlock.getStateForPlacement` (`RedstoneLampBlock.java:31-33`):
    // a redstone block below the clicked cell lands the lamp already lit.
    let (state, _) = place(
        "minecraft:redstone_lamp",
        default_state_of("minecraft:stone"),
    );
    assert_eq!(
        state,
        Some(state_of("minecraft:redstone_lamp", &[("lit", "false")]))
    );

    let (state, _) = place(
        "minecraft:redstone_lamp",
        default_state_of("minecraft:redstone_block"),
    );
    assert_eq!(
        state,
        Some(state_of("minecraft:redstone_lamp", &[("lit", "true")]))
    );
}

#[test]
fn shelf_placement_sets_facing_water_and_signal() {
    let item = item_id("minecraft:oak_shelf");
    let mut placement = context(CLICKED);
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
            "minecraft:oak_shelf",
            &[
                ("facing", "east"),
                ("powered", "false"),
                ("side_chain", "unconnected"),
                ("waterlogged", "true"),
            ]
        ))
    );

    let powered = state_for_placement(item, placement, vanilla_tags(), &mut |at| {
        if at == below(CLICKED) {
            default_state_of("minecraft:redstone_block")
        } else {
            air()
        }
    });
    assert_eq!(
        powered,
        Some(state_of(
            "minecraft:oak_shelf",
            &[
                ("facing", "east"),
                ("powered", "true"),
                ("side_chain", "unconnected"),
                ("waterlogged", "false"),
            ]
        ))
    );
}

#[test]
fn campfire_placement_sets_water_smoke_lighting_and_facing() {
    let hay = default_state_of("minecraft:hay_block");
    let name = "minecraft:campfire";
    for (replaced, below_state, waterlogged, signal_fire, lit) in [
        (water(0), hay, "true", "true", "false"),
        (water(8), hay, "false", "true", "true"),
        (air(), stone(), "false", "false", "true"),
    ] {
        assert_eq!(
            state_for_placement(item_id(name), context(CLICKED), vanilla_tags(), &mut |at| {
                if at == CLICKED {
                    replaced
                } else {
                    below_state
                }
            }),
            Some(state_of(
                name,
                &[
                    ("facing", "south"),
                    ("lit", lit),
                    ("signal_fire", signal_fire),
                    ("waterlogged", waterlogged),
                ]
            )),
            "replaced {replaced} below {below_state}"
        );
    }
}

#[test]
fn comparator_placement_faces_away_from_the_player_and_needs_support() {
    let item = item_id("minecraft:comparator");
    for direction in [Direction::North, Direction::East] {
        let mut placement = context(CLICKED);
        placement.horizontal_direction = direction;
        assert_eq!(
            state_for_placement(item, placement, vanilla_tags(), &mut |_| stone()),
            Some(state_of(
                "minecraft:comparator",
                &[
                    (
                        "facing",
                        super::super::super::direction_serialized_name(opposite(direction)),
                    ),
                    ("mode", "compare"),
                    ("powered", "false"),
                ],
            ))
        );
    }

    assert_eq!(
        state_for_placement(item, context(CLICKED), vanilla_tags(), &mut |_| air()),
        None
    );
}

#[test]
fn repeater_placement_faces_away_and_locks_against_a_powered_side_diode() {
    let item = item_id("minecraft:repeater");
    let mut placement = context(CLICKED);
    placement.horizontal_direction = Direction::West;
    assert_eq!(
        state_for_placement(item, placement, vanilla_tags(), &mut |at| {
            if at == below(CLICKED) {
                stone()
            } else {
                air()
            }
        }),
        Some(state_of(
            "minecraft:repeater",
            &[
                ("delay", "1"),
                ("facing", "east"),
                ("locked", "false"),
                ("powered", "false"),
            ],
        ))
    );

    placement.horizontal_direction = Direction::South;
    let east = relative(CLICKED, Direction::East);
    let powered_repeater = state_of(
        "minecraft:repeater",
        &[
            ("delay", "1"),
            ("facing", "east"),
            ("locked", "false"),
            ("powered", "true"),
        ],
    );
    assert_eq!(
        state_for_placement(item, placement, vanilla_tags(), &mut |at| {
            if at == east {
                powered_repeater
            } else if at == below(CLICKED) {
                stone()
            } else {
                air()
            }
        }),
        Some(state_of(
            "minecraft:repeater",
            &[
                ("delay", "1"),
                ("facing", "north"),
                ("locked", "true"),
                ("powered", "false"),
            ],
        ))
    );
}

#[test]
fn command_block_and_pitcher_crop_replacement_rules() {
    let command = item_id("minecraft:command_block");
    assert_eq!(
        can_replace_for_placement(command, air(), context(CLICKED)),
        Some(true)
    );
    assert_eq!(
        can_replace_for_placement(
            command,
            default_state_of("minecraft:command_block"),
            context(CLICKED)
        ),
        Some(false)
    );
    let crop = default_state_of("minecraft:pitcher_crop");
    for item in ["minecraft:pitcher_pod", "minecraft:stone"] {
        assert_eq!(
            can_replace_for_placement(item_id(item), crop, context(CLICKED)),
            Some(false)
        );
    }
}
