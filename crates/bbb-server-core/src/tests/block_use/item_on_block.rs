//! Items that change the clicked block -- tool conversions, honeycomb, water
//! bottles, shears, flint and steel and bone meal -- and the block legs that
//! clean items in a water cauldron, put a candle on a cake and tune a note
//! block.

use super::*;

use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::{custom_stat, StatKey};

mod lodestone_compass;

fn item_id(name: &str) -> i32 {
    bbb_protocol::item_types::vanilla_item_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("missing built-in item {name}"))
        .id
}

/// A survival session holding `count` of `name` in the selected slot.
fn survival_world_holding(
    name: &str,
    count: i32,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    hold(&mut world, name, count);
    (sender, receiver, world)
}

fn default_state(name: &str) -> i32 {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("missing built-in block {name}"));
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id)
        .expect("every block has a default state")
}

fn hold(world: &mut ServerWorld, name: &str, count: i32) {
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id(name)),
            count,
            ..ItemStackSummary::empty()
        },
    );
}

fn held(world: &ServerWorld) -> ItemStackSummary {
    world.publication().players[0].inventory.slots[0].clone()
}

fn place(world: &mut ServerWorld, pos: BlockPos, state: i32) {
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        state,
        SET_BLOCK_UPDATE_ALL
    ));
}

fn stat(world: &mut ServerWorld, stat: StatKey) -> i32 {
    world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap()
        .player_patches[0]
        .stats
        .value(stat)
}

fn items_used(world: &mut ServerWorld, name: &str) -> i32 {
    stat(
        world,
        StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id(name)),
    )
}

fn dropped_items(world: &mut ServerWorld) -> Vec<String> {
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    saved_entities(&request)
        .iter()
        .filter_map(|entity| match field(entity, "Item") {
            NbtSummaryValue::Compound(stack) => match field(stack, "id") {
                NbtSummaryValue::String(id) => Some(id.try_to_string().unwrap()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn an_axe_strips_a_log_keeping_its_axis_and_wears_by_one() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:iron_axe", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place(
        &mut world,
        at,
        placement_state("minecraft:oak_log", &[("axis", "x")]),
    );

    click_face(&sender, &receiver, &mut world, at, Direction::Up, 1);

    assert_eq!(
        live_block(&mut world, at),
        Some(placement_state(
            "minecraft:stripped_oak_log",
            &[("axis", "x")]
        ))
    );
    assert_eq!(held(&world).component_patch.damage, Some(1));
    assert_eq!(items_used(&mut world, "minecraft:iron_axe"), 1);
}

#[test]
fn an_axe_scrapes_copper_and_takes_wax_off() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:stone_axe", 1);
    let exposed = BlockPos { x: 0, y: 64, z: 1 };
    let waxed = BlockPos { x: 1, y: 64, z: 1 };
    place(
        &mut world,
        exposed,
        default_state("minecraft:exposed_copper"),
    );
    place(
        &mut world,
        waxed,
        default_state("minecraft:waxed_cut_copper"),
    );

    click_face(&sender, &receiver, &mut world, exposed, Direction::Up, 1);
    click_face(&sender, &receiver, &mut world, waxed, Direction::Up, 2);

    assert_eq!(
        live_block(&mut world, exposed),
        Some(default_state("minecraft:copper_block"))
    );
    assert_eq!(
        live_block(&mut world, waxed),
        Some(default_state("minecraft:cut_copper"))
    );
    assert_eq!(held(&world).component_patch.damage, Some(2));
}

#[test]
fn honeycomb_waxes_copper_and_is_consumed() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:honeycomb", 3);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place(&mut world, at, default_state("minecraft:copper_block"));

    click_face(&sender, &receiver, &mut world, at, Direction::Up, 1);

    assert_eq!(
        live_block(&mut world, at),
        Some(default_state("minecraft:waxed_copper_block"))
    );
    assert_eq!(held(&world).count, 2);
}

/// `HoneycombItem.useOn` (`HoneycombItem.java:156-160`) repeats the wax
/// particles and `BLOCK_CHANGE` on the other half of a double chest.
#[test]
fn honeycomb_on_a_double_copper_chest_waxes_both_halves_with_particles() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:honeycomb", 3);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let chest = |kind: &str| {
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:copper_chest",
            &BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("type".to_string(), kind.to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]),
        )
        .unwrap()
    };
    let left = BlockPos { x: 0, y: 64, z: 1 };
    // A north-facing left half reaches clockwise, east, for its partner.
    let right = BlockPos { x: 1, y: 64, z: 1 };
    place(&mut world, left, chest("left"));
    place(&mut world, right, chest("right"));
    // Sneaking skips the chest's own menu.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(bbb_protocol::packets::PlayerInput {
            shift: true,
            ..Default::default()
        }),
    );

    let tick = click_face(&sender, &receiver, &mut world, left, Direction::Up, 1);

    for pos in [left, right] {
        assert_eq!(
            live_block(&mut world, pos)
                .and_then(bbb_protocol::block_state_types::vanilla_block_state_block_type_id),
            bbb_protocol::block_types::vanilla_block_type_for_resource_id(
                "minecraft:waxed_copper_chest"
            )
            .map(|block| block.id)
        );
        assert!(tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::LevelEvent(level_event),
            } if level_event.event_type == 3003 && level_event.pos == pos)));
    }
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(events.iter().any(|event| event.game_event
        == crate::level_event::VanillaGameEvent::BLOCK_CHANGE
        && event.pos == right));
}

/// `BeehiveBlock.useItemOn` (`BeehiveBlock.java:166`, `:182-184`): shears lose
/// durability, and both harvesting items count as used.
#[test]
fn harvesting_a_full_bee_nest_wears_the_shears_and_counts_the_items_used() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:shears", 1);
    let nest = BlockPos { x: 0, y: 64, z: 1 };
    place(&mut world, nest, bee_nest_state(5));
    click_face(&sender, &receiver, &mut world, nest, Direction::Up, 1);
    assert_eq!(held(&world).component_patch.damage, Some(1));
    assert_eq!(items_used(&mut world, "minecraft:shears"), 1);

    hold(&mut world, "minecraft:glass_bottle", 2);
    place(&mut world, nest, bee_nest_state(5));
    click_face(&sender, &receiver, &mut world, nest, Direction::Up, 2);
    assert_eq!(items_used(&mut world, "minecraft:glass_bottle"), 1);
}

/// `ComposterBlock.useItemOn` (`ComposterBlock.java:262`) counts every
/// compostable put in below level 7.
#[test]
fn composting_counts_the_item_used() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:cake", 2);
    let composter = BlockPos { x: 0, y: 64, z: 1 };
    place(&mut world, composter, composter_state(0));
    click_face(&sender, &receiver, &mut world, composter, Direction::Up, 1);
    click_face(&sender, &receiver, &mut world, composter, Direction::Up, 2);
    assert_eq!(items_used(&mut world, "minecraft:cake"), 2);
}

#[test]
fn a_hoe_tills_dirt_only_with_air_above_and_rooted_dirt_drops_roots() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:wooden_hoe", 1);
    let dirt = BlockPos { x: 0, y: 64, z: 1 };
    let covered = BlockPos { x: 1, y: 64, z: 1 };
    let rooted = BlockPos { x: -1, y: 64, z: 1 };
    place(&mut world, dirt, default_state("minecraft:dirt"));
    place(&mut world, covered, default_state("minecraft:dirt"));
    place(
        &mut world,
        BlockPos { x: 1, y: 65, z: 1 },
        default_state("minecraft:stone"),
    );
    place(&mut world, rooted, default_state("minecraft:rooted_dirt"));

    click_face(&sender, &receiver, &mut world, dirt, Direction::Up, 1);
    click_face(&sender, &receiver, &mut world, covered, Direction::North, 2);
    click_face(&sender, &receiver, &mut world, rooted, Direction::North, 3);

    assert_eq!(
        live_block(&mut world, dirt),
        Some(default_state("minecraft:farmland"))
    );
    assert_eq!(
        live_block(&mut world, covered),
        Some(default_state("minecraft:dirt"))
    );
    assert_eq!(
        live_block(&mut world, rooted),
        Some(default_state("minecraft:dirt"))
    );
    assert_eq!(dropped_items(&mut world), ["minecraft:hanging_roots"]);
    assert_eq!(held(&world).component_patch.damage, Some(2));
}

#[test]
fn a_shovel_flattens_grass_into_a_path() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:iron_shovel", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place(&mut world, at, default_state("minecraft:grass_block"));

    click_face(&sender, &receiver, &mut world, at, Direction::Up, 1);

    assert_eq!(
        live_block(&mut world, at),
        Some(default_state("minecraft:dirt_path"))
    );
    assert_eq!(held(&world).component_patch.damage, Some(1));
}

#[test]
fn a_water_bottle_turns_dirt_into_mud_and_leaves_a_glass_bottle() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:stone", 1);
    let water = bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:water")
        .expect("a built-in potion")
        .id;
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:potion")),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
                ],
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(water),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }.into(),
        },
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place(&mut world, at, default_state("minecraft:dirt"));

    let mut events = click_face(&sender, &receiver, &mut world, at, Direction::Up, 1).events;
    events.extend(world.tick(receiver.drain()).events);

    assert_eq!(
        live_block(&mut world, at),
        Some(default_state("minecraft:mud"))
    );
    // `PotionItem.java:48-60`.
    let splashes = events
        .iter()
        .filter(|event| {
            matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::LevelParticles(particles), .. }
                if particles.particle.particle_type_id
                    == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_SPLASH_ID
                    && particles.position.y == 65.0)
        })
        .count();
    assert_eq!(splashes, 5);
    assert_eq!(
        held(&world).item_id,
        Some(item_id("minecraft:glass_bottle"))
    );
}

#[test]
fn shears_stop_a_growing_vine_tip() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:shears", 1);
    let support = BlockPos { x: 0, y: 66, z: 1 };
    let tip = BlockPos { x: 0, y: 65, z: 1 };
    place(&mut world, support, default_state("minecraft:netherrack"));
    place(
        &mut world,
        tip,
        placement_state("minecraft:weeping_vines", &[("age", "3")]),
    );

    click_face(&sender, &receiver, &mut world, tip, Direction::North, 1);

    assert_eq!(
        live_block(&mut world, tip),
        Some(placement_state("minecraft:weeping_vines", &[("age", "25")]))
    );
    assert_eq!(held(&world).component_patch.damage, Some(1));
}

#[test]
fn flint_and_steel_sets_fire_and_lights_a_nether_portal() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:flint_and_steel", 1);
    let obsidian = default_state("minecraft:obsidian");
    // A 2x3 portal opening in the x axis at z = 2.
    for x in -1..=2 {
        place(&mut world, BlockPos { x, y: 64, z: 2 }, obsidian);
        place(&mut world, BlockPos { x, y: 68, z: 2 }, obsidian);
    }
    for y in 65..=67 {
        place(&mut world, BlockPos { x: -1, y, z: 2 }, obsidian);
        place(&mut world, BlockPos { x: 2, y, z: 2 }, obsidian);
    }
    let stone = BlockPos { x: 1, y: 64, z: 0 };
    place(&mut world, stone, default_state("minecraft:stone"));

    click_face(&sender, &receiver, &mut world, stone, Direction::Up, 1);
    click_face(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 0, y: 64, z: 2 },
        Direction::Up,
        2,
    );

    assert_eq!(
        live_block(&mut world, BlockPos { x: 1, y: 65, z: 0 }).and_then(block_type_name),
        Some("minecraft:fire")
    );
    for (x, y) in [(0, 65), (1, 65), (0, 67), (1, 67)] {
        assert_eq!(
            live_block(&mut world, BlockPos { x, y, z: 2 }).and_then(block_type_name),
            Some("minecraft:nether_portal"),
            "({x}, {y})"
        );
    }
    assert_eq!(held(&world).component_patch.damage, Some(2));
}

/// Lighting and shovel dousing are item rules (`FlintAndSteelItem.java:31-57`,
/// `FireChargeItem.java:34-48`, `ShovelItem.java:51-69`), so they still run
/// when sneaking skips the block's own use (`ServerPlayerGameMode.java:386-395`).
#[test]
fn sneaking_igniters_light_candles_and_a_shovel_douses_a_campfire() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:flint_and_steel", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(bbb_protocol::packets::PlayerInput {
            shift: true,
            ..Default::default()
        }),
    );
    let lit = |state: i32| bbb_server_types::block_queries::bool_property(state, "lit");
    let candle = BlockPos { x: 0, y: 64, z: 1 };
    place(&mut world, candle, default_state("minecraft:candle"));
    click_face(&sender, &receiver, &mut world, candle, Direction::Up, 1);
    assert_eq!(live_block(&mut world, candle).and_then(lit), Some(true));
    assert_eq!(held(&world).component_patch.damage, Some(1));

    hold(&mut world, "minecraft:fire_charge", 2);
    let cake = BlockPos { x: 1, y: 64, z: 1 };
    place(&mut world, cake, default_state("minecraft:candle_cake"));
    click_face(&sender, &receiver, &mut world, cake, Direction::Up, 2);
    assert_eq!(live_block(&mut world, cake).and_then(lit), Some(true));
    assert_eq!(held(&world).count, 1);

    hold(&mut world, "minecraft:iron_shovel", 1);
    let campfire = BlockPos { x: 2, y: 64, z: 1 };
    place(&mut world, campfire, default_state("minecraft:campfire"));
    assert_eq!(live_block(&mut world, campfire).and_then(lit), Some(true));
    click_face(&sender, &receiver, &mut world, campfire, Direction::Up, 3);
    assert_eq!(live_block(&mut world, campfire).and_then(lit), Some(false));
    assert_eq!(held(&world).component_patch.damage, Some(1));
}

fn block_type_name(state: i32) -> Option<&'static str> {
    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
        .and_then(bbb_protocol::block_types::vanilla_block_type)
        .map(|block| block.resource_id)
}

#[test]
fn bone_meal_ages_a_crop_and_is_consumed_only_on_a_target() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:bone_meal", 5);
    let farmland = BlockPos { x: 0, y: 64, z: 1 };
    let crop = BlockPos { x: 0, y: 65, z: 1 };
    let stone = BlockPos { x: 1, y: 64, z: 1 };
    place(&mut world, farmland, default_state("minecraft:farmland"));
    place(&mut world, crop, default_state("minecraft:wheat"));
    place(&mut world, stone, default_state("minecraft:stone"));

    click_face(&sender, &receiver, &mut world, stone, Direction::North, 1);
    assert_eq!(held(&world).count, 5);
    click_face(&sender, &receiver, &mut world, crop, Direction::North, 2);

    let age = live_block(&mut world, crop)
        .and_then(crate::block_behavior::crop::age)
        .expect("the wheat is still there");
    assert!((2..=5).contains(&age), "{age}");
    assert_eq!(held(&world).count, 4);
    assert_eq!(items_used(&mut world, "minecraft:bone_meal"), 1);
}

#[test]
fn bone_meal_grows_a_dripleaf_head_and_clicked_stem_until_blocked() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:bone_meal", 3);
    let base = BlockPos { x: 0, y: 65, z: 1 };
    let second = BlockPos { y: 66, ..base };
    let third = BlockPos { y: 67, ..base };
    place(
        &mut world,
        BlockPos { y: 64, ..base },
        default_state("minecraft:clay"),
    );
    place(
        &mut world,
        base,
        placement_state(
            "minecraft:big_dripleaf",
            &[
                ("facing", "east"),
                ("waterlogged", "true"),
                ("tilt", "full"),
            ],
        ),
    );
    place(
        &mut world,
        second,
        placement_state("minecraft:water", &[("level", "1")]),
    );

    let grown = click_face(&sender, &receiver, &mut world, base, Direction::North, 1);
    assert_eq!(
        live_block(&mut world, base),
        Some(placement_state(
            "minecraft:big_dripleaf_stem",
            &[("facing", "east"), ("waterlogged", "true")]
        ))
    );
    assert_eq!(
        live_block(&mut world, second),
        Some(placement_state(
            "minecraft:big_dripleaf",
            &[
                ("facing", "east"),
                ("waterlogged", "false"),
                ("tilt", "none")
            ]
        ))
    );
    assert!(plant_growth_event(&grown, base));
    assert_eq!(held(&world).count, 2);

    place(
        &mut world,
        base,
        placement_state(
            "minecraft:big_dripleaf_stem",
            &[("facing", "west"), ("waterlogged", "true")],
        ),
    );
    click_face(&sender, &receiver, &mut world, base, Direction::North, 2);
    assert_eq!(
        live_block(&mut world, second),
        Some(placement_state(
            "minecraft:big_dripleaf_stem",
            &[("facing", "west"), ("waterlogged", "false")]
        ))
    );
    let head = placement_state(
        "minecraft:big_dripleaf",
        &[
            ("facing", "west"),
            ("waterlogged", "false"),
            ("tilt", "none"),
        ],
    );
    assert_eq!(live_block(&mut world, third), Some(head));
    assert_eq!(held(&world).count, 1);

    place(
        &mut world,
        BlockPos { y: 68, ..base },
        default_state("minecraft:stone"),
    );
    let rejected = click_face(&sender, &receiver, &mut world, base, Direction::North, 3);
    assert_eq!(held(&world).count, 1);
    assert_eq!(live_block(&mut world, third), Some(head));
    assert!(!plant_growth_event(&rejected, base));
}

#[test]
fn bone_meal_turns_either_small_dripleaf_half_into_a_column() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:bone_meal", 3);
    for (index, half) in ["lower", "upper"].into_iter().enumerate() {
        let lower = BlockPos {
            x: index as i32,
            y: 65,
            z: 1,
        };
        let upper = BlockPos { y: 66, ..lower };
        place(
            &mut world,
            BlockPos { y: 64, ..lower },
            default_state("minecraft:clay"),
        );
        place(
            &mut world,
            lower,
            placement_state(
                "minecraft:small_dripleaf",
                &[
                    ("half", "lower"),
                    ("facing", "south"),
                    ("waterlogged", "false"),
                ],
            ),
        );
        place(
            &mut world,
            upper,
            placement_state(
                "minecraft:small_dripleaf",
                &[
                    ("half", "upper"),
                    ("facing", "south"),
                    ("waterlogged", "false"),
                ],
            ),
        );
        place(
            &mut world,
            BlockPos { y: 67, ..lower },
            default_state("minecraft:stone"),
        );
        let clicked = if half == "lower" { lower } else { upper };
        let grown = click_face(
            &sender,
            &receiver,
            &mut world,
            clicked,
            Direction::North,
            index as i32 + 1,
        );
        assert_eq!(
            live_block(&mut world, lower),
            Some(placement_state(
                "minecraft:big_dripleaf_stem",
                &[("facing", "south"), ("waterlogged", "false")]
            ))
        );
        assert_eq!(
            live_block(&mut world, upper),
            Some(placement_state(
                "minecraft:big_dripleaf",
                &[
                    ("facing", "south"),
                    ("waterlogged", "false"),
                    ("tilt", "none")
                ]
            ))
        );
        assert!(plant_growth_event(&grown, clicked));
        assert_eq!(held(&world).count, 2 - index as i32);
    }
}

#[test]
fn bone_meal_spreads_glow_lichen_once_and_rejects_all_faces() {
    use bbb_server_worldgen::multiface::{has_face, DIRECTIONS};

    let (sender, receiver, mut world) = survival_world_holding("minecraft:bone_meal", 3);
    let pos = BlockPos { x: 0, y: 65, z: 1 };
    for x in -1..=1 {
        for z in 0..=2 {
            let floor = BlockPos { x, y: 64, z };
            if live_block(&mut world, floor) != Some(default_state("minecraft:stone")) {
                place(&mut world, floor, default_state("minecraft:stone"));
            }
        }
    }
    let mut lichen =
        crate::fluid::with_property(default_state("minecraft:glow_lichen"), "down", "true")
            .unwrap();
    place(&mut world, pos, lichen);
    let grown = click_face(&sender, &receiver, &mut world, pos, Direction::Up, 1);
    let mut attachments = 0;
    for x in -1..=1 {
        for y in 64..=66 {
            for z in 0..=2 {
                if let Some(state) = live_block(&mut world, BlockPos { x, y, z }) {
                    if block_type_name(state) == Some("minecraft:glow_lichen") {
                        attachments += DIRECTIONS
                            .into_iter()
                            .filter(|&face| has_face(state, face))
                            .count();
                    }
                }
            }
        }
    }
    assert_eq!(attachments, 2);
    assert_eq!(held(&world).count, 2);
    assert!(plant_growth_event(&grown, pos));

    for face in DIRECTIONS {
        let support = bbb_server_types::directions::relative(pos, face);
        if live_block(&mut world, support) != Some(default_state("minecraft:stone")) {
            place(&mut world, support, default_state("minecraft:stone"));
        }
        lichen = crate::fluid::with_property(
            lichen,
            bbb_server_worldgen::multiface::face_property(face),
            "true",
        )
        .unwrap();
    }
    place(&mut world, pos, lichen);
    let unchanged = click_face(&sender, &receiver, &mut world, pos, Direction::Up, 2);
    assert_eq!(held(&world).count, 2);
    assert_eq!(live_block(&mut world, pos), Some(lichen));
    assert!(!plant_growth_event(&unchanged, pos));
}

#[test]
fn bone_meal_grows_moss_carpet_topper_only_when_incomplete() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:bone_meal", 3);
    let base = BlockPos { x: 0, y: 65, z: 1 };
    let topper = BlockPos { y: 66, ..base };
    for pos in [
        BlockPos { y: 64, ..base },
        BlockPos { z: 0, ..base },
        BlockPos { z: 0, ..topper },
    ] {
        place(&mut world, pos, default_state("minecraft:stone"));
    }
    place(
        &mut world,
        base,
        placement_state(
            "minecraft:pale_moss_carpet",
            &[
                ("bottom", "true"),
                ("north", "low"),
                ("east", "none"),
                ("south", "none"),
                ("west", "none"),
            ],
        ),
    );

    let grown = click_face(&sender, &receiver, &mut world, base, Direction::Up, 1);
    let state = live_block(&mut world, topper).expect("a topper grew");
    assert_eq!(
        bbb_server_types::block_queries::bool_property(state, "bottom"),
        Some(false)
    );
    assert_eq!(
        crate::block_behavior::string_property(state, "north").as_deref(),
        Some("low")
    );
    assert_eq!(
        live_block(&mut world, base)
            .and_then(|state| crate::block_behavior::string_property(state, "north"))
            .as_deref(),
        Some("tall")
    );
    assert_eq!(held(&world).count, 2);
    assert!(plant_growth_event(&grown, base));

    let unchanged = click_face(&sender, &receiver, &mut world, base, Direction::Up, 2);
    assert_eq!(held(&world).count, 2);
    assert!(!plant_growth_event(&unchanged, base));
}

#[test]
fn bone_meal_grows_underwater_and_rejects_shallow_water() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:bone_meal", 3);
    let floor = BlockPos { x: 0, y: 64, z: 1 };
    let water = BlockPos { y: 65, ..floor };
    place(&mut world, floor, default_state("minecraft:stone"));
    place(
        &mut world,
        water,
        placement_state("minecraft:water", &[("level", "1")]),
    );
    let rejected = click_face(&sender, &receiver, &mut world, floor, Direction::Up, 1);
    assert_eq!(held(&world).count, 3);
    assert!(!plant_growth_event(&rejected, water));

    // Falling water has a full fluid height even though it is not a source.
    place(
        &mut world,
        water,
        placement_state("minecraft:water", &[("level", "8")]),
    );
    let grown = click_face(&sender, &receiver, &mut world, floor, Direction::Up, 2);
    assert_eq!(
        live_block(&mut world, water).and_then(block_type_name),
        Some("minecraft:seagrass")
    );
    assert_eq!(held(&world).count, 2);
    assert!(plant_growth_event(&grown, water));
}

fn plant_growth_event(tick: &TickResult, pos: BlockPos) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::LevelEvent(update), .. }
                if update.event_type == 1505 && update.pos == pos && update.data == 15
        )
    })
}

#[test]
fn bone_meal_uses_the_candidate_biome_and_live_wall_coral_tag() {
    use crate::registry_tags::RegistryTagSnapshot;
    let (sender, receiver, mut world) = survival_world_holding("minecraft:bone_meal", 2);
    let support = BlockPos { x: 0, y: 65, z: 1 };
    let water = BlockPos { x: 1, ..support };
    place(&mut world, support, default_state("minecraft:stone"));
    place(&mut world, water, default_state("minecraft:water"));
    for x in -1..=1 {
        for y in 15..=17 {
            for z in -1..=1 {
                assert!(world.set_noise_biome_for_test(
                    Java26BuiltinDimension::Overworld,
                    (x, y, z),
                    "minecraft:plains"
                ));
            }
        }
    }
    world.install_level_registry_tags(Arc::new(RegistryTagSnapshot::new([
        (
            "minecraft:worldgen/biome".into(),
            "minecraft:produces_corals_from_bonemeal".into(),
            vec!["minecraft:desert".into()],
        ),
        (
            "minecraft:block".into(),
            "minecraft:wall_corals".into(),
            vec!["minecraft:tube_coral_wall_fan".into()],
        ),
    ])));
    click_face(&sender, &receiver, &mut world, support, Direction::East, 1);
    assert_eq!(
        live_block(&mut world, water).and_then(block_type_name),
        Some("minecraft:water")
    );
    assert_eq!(held(&world).count, 1);

    world.install_level_registry_tags(Arc::new(RegistryTagSnapshot::new([
        (
            "minecraft:worldgen/biome".into(),
            "minecraft:produces_corals_from_bonemeal".into(),
            vec!["minecraft:plains".into()],
        ),
        (
            "minecraft:block".into(),
            "minecraft:wall_corals".into(),
            vec!["minecraft:tube_coral_wall_fan".into()],
        ),
    ])));
    let tick = click_face(&sender, &receiver, &mut world, support, Direction::East, 2);
    assert_eq!(
        live_block(&mut world, water),
        Some(placement_state(
            "minecraft:tube_coral_wall_fan",
            &[("facing", "east"), ("waterlogged", "true")]
        ))
    );
    assert_eq!(held(&world).count, 0);
    assert!(plant_growth_event(&tick, water));
}

#[test]
fn bone_meal_grows_grass_flowers_from_each_candidate_biome() {
    use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};

    for without_grass_feature in [false, true] {
        let mut config = datapack_config();
        if without_grass_feature {
            Arc::make_mut(config.datapacks.as_mut().unwrap())
                .registries
                .worldgen
                .placed_features
                .remove("minecraft:grass_bonemeal");
        }
        let flowers = config
            .datapacks
            .as_ref()
            .unwrap()
            .tags
            .registry("minecraft:block")
            .unwrap()
            .tag("minecraft:small_flowers")
            .unwrap()
            .to_vec();
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        world.set_player_mode_and_abilities_for_test(
            ConnectionId(1),
            GameType::Survival,
            PlayerAbilities::for_game_type(GameType::Survival),
        );
        hold(&mut world, "minecraft:bone_meal", 8);
        let clicked = BlockPos { x: 0, y: 64, z: 1 };
        let zoom_seed = biome_zoom_seed(world.seed());
        let clicked_quart = fuzzy_biome_quart_position(zoom_seed, clicked.x, clicked.y, clicked.z);
        for x in -3..=3 {
            for y in 15..=17 {
                for z in -3..=3 {
                    let biome = if (x, y, z) == clicked_quart {
                        "minecraft:plains"
                    } else {
                        "minecraft:swamp"
                    };
                    assert!(world.set_noise_biome_for_test(
                        Java26BuiltinDimension::Overworld,
                        (x, y, z),
                        biome
                    ));
                }
            }
        }
        for x in -8..=8 {
            for z in -8..=8 {
                place(
                    &mut world,
                    BlockPos { x, y: 64, z },
                    default_state("minecraft:grass_block"),
                );
            }
        }
        for sequence in 1..=8 {
            for y in 65..=66 {
                let pos = BlockPos { y, ..clicked };
                if live_block(&mut world, pos) != Some(default_state("minecraft:air")) {
                    place(&mut world, pos, default_state("minecraft:air"));
                }
            }
            let tick = click_face(
                &sender,
                &receiver,
                &mut world,
                clicked,
                Direction::Up,
                sequence,
            );
            assert!(plant_growth_event(&tick, clicked));
        }
        assert_eq!(held(&world).count, 0);
        let mut plains_flowers = 0;
        let mut swamp_flowers = 0;
        for x in -8..=8 {
            for z in -8..=8 {
                let pos = BlockPos { x, y: 65, z };
                let block = live_block(&mut world, pos)
                    .and_then(block_type_name)
                    .unwrap();
                match block {
                    "minecraft:air" | "minecraft:short_grass" | "minecraft:tall_grass" => continue,
                    "minecraft:blue_orchid" => {
                        assert_ne!(
                            fuzzy_biome_quart_position(zoom_seed, x, 65, z),
                            clicked_quart
                        );
                        swamp_flowers += 1;
                    }
                    _ => {
                        assert!(
                            flowers.iter().any(|flower| flower == block),
                            "unexpected plant {block}"
                        );
                        assert_eq!(
                            fuzzy_biome_quart_position(zoom_seed, x, 65, z),
                            clicked_quart,
                            "{block} at {pos:?}"
                        );
                        plains_flowers += 1;
                    }
                }
            }
        }
        assert!(
            plains_flowers > 0,
            "plains flowers missing, grass feature absent: {without_grass_feature}"
        );
        assert!(
            swamp_flowers > 0,
            "candidate swamp flowers missing, grass feature absent: {without_grass_feature}"
        );
    }
}

#[test]
fn bone_meal_grows_a_sapling_into_a_tree() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:bone_meal", 64);
    let sapling = BlockPos { x: 0, y: 64, z: 3 };
    place(
        &mut world,
        BlockPos { x: 0, y: 63, z: 3 },
        default_state("minecraft:dirt"),
    );
    place(&mut world, sapling, default_state("minecraft:oak_sapling"));

    for sequence in 1..=40 {
        click_face(
            &sender,
            &receiver,
            &mut world,
            sapling,
            Direction::North,
            sequence,
        );
        if live_block(&mut world, sapling).and_then(block_type_name) == Some("minecraft:oak_log") {
            return;
        }
    }
    panic!("forty bone meals never grew the oak");
}

fn with_component(name: &str, type_id: i32, value: NbtSummaryValue) -> ItemStackSummary {
    RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(item_id(name)),
        count: 1,
        ..ItemStackSummary::empty()
    })
    .with_component_nbt(
        type_id,
        Some(&value),
        crate::world::registry_runtime::item_stack_resolver_for_test(),
    )
    .expect("a valid component value")
    .into_summary()
}

fn full_water_cauldron() -> i32 {
    placement_state("minecraft:water_cauldron", &[("level", "3")])
}

#[test]
fn a_water_cauldron_washes_dye_off_leather_armor() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:stone", 1);
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        with_component(
            "minecraft:leather_helmet",
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            NbtSummaryValue::Int(0x00ff00),
        ),
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place(&mut world, at, full_water_cauldron());

    click_face(&sender, &receiver, &mut world, at, Direction::Up, 1);

    assert_eq!(
        held(&world).item_id,
        Some(item_id("minecraft:leather_helmet"))
    );
    assert_eq!(held(&world).component_patch.dyed_color, None);
    assert_eq!(
        live_block(&mut world, at),
        Some(placement_state(
            "minecraft:water_cauldron",
            &[("level", "2")]
        ))
    );
    assert_eq!(stat(&mut world, custom_stat("minecraft:clean_armor")), 1);
}

#[test]
fn a_water_cauldron_washes_the_top_banner_layer_and_a_shulker_box_colour() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:stone", 1);
    let layer = |pattern: &str| {
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "pattern".into(),
                value: NbtSummaryValue::string(pattern),
            },
            NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string("red"),
            },
        ])
    };
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        with_component(
            "minecraft:white_banner",
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            NbtSummaryValue::List(vec![
                layer("minecraft:stripe_top"),
                layer("minecraft:border"),
            ]),
        ),
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place(&mut world, at, full_water_cauldron());

    click_face(&sender, &receiver, &mut world, at, Direction::Up, 1);

    let banner = held(&world);
    assert_eq!(banner.item_id, Some(item_id("minecraft:white_banner")));
    assert_eq!(banner.component_patch.banner_pattern_layers.len(), 1);
    assert_eq!(stat(&mut world, custom_stat("minecraft:clean_banner")), 1);

    hold(&mut world, "minecraft:red_shulker_box", 1);
    click_face(&sender, &receiver, &mut world, at, Direction::Up, 2);

    assert_eq!(held(&world).item_id, Some(item_id("minecraft:shulker_box")));
    assert_eq!(
        live_block(&mut world, at),
        Some(placement_state(
            "minecraft:water_cauldron",
            &[("level", "1")]
        ))
    );
    assert_eq!(
        stat(&mut world, custom_stat("minecraft:clean_shulker_box")),
        1
    );
}

#[test]
fn a_candle_goes_onto_an_uneaten_cake_only() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:red_candle", 2);
    let whole = BlockPos { x: 0, y: 64, z: 1 };
    let bitten = BlockPos { x: 1, y: 64, z: 1 };
    place(&mut world, whole, default_state("minecraft:cake"));
    place(
        &mut world,
        bitten,
        placement_state("minecraft:cake", &[("bites", "1")]),
    );

    click_face(&sender, &receiver, &mut world, whole, Direction::Up, 1);

    assert_eq!(
        live_block(&mut world, whole),
        Some(default_state("minecraft:red_candle_cake"))
    );
    assert_eq!(held(&world).count, 1);
    assert_eq!(items_used(&mut world, "minecraft:red_candle"), 1);

    click_face(&sender, &receiver, &mut world, bitten, Direction::Up, 2);
    assert_eq!(
        live_block(&mut world, bitten),
        Some(placement_state("minecraft:cake", &[("bites", "1")]))
    );
}

#[test]
fn tuning_a_note_block_counts_tune_noteblock() {
    let (sender, receiver, mut world) = survival_world_holding("minecraft:stone", 1);
    clear_hand(&sender, &receiver, &mut world);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, ItemStackSummary::empty());
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place(&mut world, at, note_block_state("harp", 0));

    click_face(&sender, &receiver, &mut world, at, Direction::Up, 1);

    let tuned = live_block(&mut world, at)
        .and_then(bbb_protocol::block_state_types::vanilla_block_state_identity)
        .and_then(|identity| identity.properties.get("note").cloned());
    assert_eq!(tuned.as_deref(), Some("1"));
    assert_eq!(stat(&mut world, custom_stat("minecraft:tune_noteblock")), 1);
}
