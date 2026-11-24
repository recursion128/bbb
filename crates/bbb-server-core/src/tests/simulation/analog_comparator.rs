//! What a live comparator reads out of the analog owners beside it.

use super::*;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID;

/// The world a comparator reads in: the input block one cell east of where the
/// comparator will stand, written first so its block entity can be filled
/// before anything reads it.
fn analog_input_comparator_world(
    comparator: BlockPos,
    input: BlockPos,
    input_state: i32,
) -> ServerWorld {
    analog_input_comparator_world_with_rules(comparator, input, input_state, &[])
}

fn analog_input_comparator_world_with_rules(
    comparator: BlockPos,
    input: BlockPos,
    input_state: i32,
    rules: &[(&str, bbb_protocol::game_rules::GameRuleValue)],
) -> ServerWorld {
    let mut candidate = restore_candidate(10, None, Vec::new());
    for (key, value) in rules {
        candidate.game_rules.insert((*key).to_string(), *value);
    }
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let _ = world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            y: comparator.y - 1,
            ..comparator
        },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        input,
        input_state,
        SET_BLOCK_UPDATE_ALL,
    ));
    world
}

/// The comparator, then a block change beside it: `DiodeBlock.onPlace`
/// only tells the cell in front, so the first read
/// is scheduled by `neighborChanged` the way any later one is.
fn place_analog_comparator(world: &mut ServerWorld, comparator: BlockPos) {
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        comparator,
        comparator_state("east"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            y: comparator.y + 1,
            ..comparator
        },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    run_comparator_delay(world);
}

fn default_analog_state(name: &str) -> i32 {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("{name} is a built-in Java 26.1 block"));
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id)
        .expect("a registered block has a default state")
}

fn stone_stack_of(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count,
        component_patch: Default::default(),
    }
}

/// The same stack as [`stone_stack_of`] in the form a container minecart keeps.
fn stone_minecart_stack(count: i32) -> bbb_server_types::item_stack::RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::from_summary(stone_stack_of(count))
}

fn analog_state_of(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties: BTreeMap<String, String> = properties
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        name,
        &properties,
    )
    .unwrap_or_else(|| panic!("no built-in Java 26.1 state for {name} {properties:?}"))
}

fn awake_creaking_heart() -> i32 {
    analog_state_of(
        "minecraft:creaking_heart",
        &[
            ("axis", "y"),
            ("creaking_heart_state", "awake"),
            ("natural", "false"),
        ],
    )
}

/// `CreakingHeartBlockEntity.setCreakingInfo(UUID)`
/// which on this server only a save
/// or a command reaches.
fn bind_creaking_heart(world: &mut ServerWorld, pos: BlockPos, uuid: Uuid) {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .and_then(crate::block_entity::BlockEntity::creaking_heart_mut)
        .expect("the heart has its block entity")
        .set_protector_for_test(uuid);
}

fn chest_state(facing: &str, kind: &str) -> i32 {
    analog_state_of(
        "minecraft:chest",
        &[("facing", facing), ("type", kind), ("waterlogged", "false")],
    )
}

/// `AbstractFurnaceBlock`, `BrewingStandBlock`, `CrafterBlock`, `CommandBlock`
/// and `SculkSensorBlock` all read a block entity this server keeps, so a live
/// comparator in front of each answers rather than recording a refusal.
#[test]
fn a_live_comparator_reads_the_block_entities_the_analog_owners_keep() {
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let input = BlockPos { x: 1, y: 64, z: 0 };

    // `AbstractFurnaceBlock.getAnalogOutputSignal` (`:67-69`): three full slots
    // are 1.0 of the container.
    let mut furnace = analog_input_comparator_world(
        comparator,
        input,
        default_analog_state("minecraft:blast_furnace"),
    );
    for slot in 0..3 {
        furnace
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, input)
            .and_then(BlockEntity::furnace_mut)
            .expect("the write created the furnace's block entity")
            .items_mut()[slot] = crate::tests::native_stack(stone_stack_of(64));
    }
    place_analog_comparator(&mut furnace, comparator);
    assert_eq!(live_comparator_output(&mut furnace, comparator), 15);

    // `BrewingStandBlock.getAnalogOutputSignal`
    // is the same container body over its five slots, so three full bottle
    // slots read as 1 + floor(3 / 5 * 14).
    let mut stand = analog_input_comparator_world(
        comparator,
        input,
        default_analog_state("minecraft:brewing_stand"),
    );
    for slot in 0..3 {
        stand
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, input)
            .and_then(BlockEntity::brewing_stand_mut)
            .expect("the write created the brewing stand's block entity")
            .items_mut()[slot] = crate::tests::native_stack(stone_stack_of(64));
    }
    place_analog_comparator(&mut stand, comparator);
    assert_eq!(live_comparator_output(&mut stand, comparator), 9);

    // `CrafterBlock.getAnalogOutputSignal` (`:68-70`) counts one taken slot and
    // one disabled empty one.
    let mut crafter =
        analog_input_comparator_world(comparator, input, default_analog_state("minecraft:crafter"));
    {
        let object = crafter
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, input)
            .and_then(BlockEntity::crafter_mut)
            .expect("the write created the crafter's block entity");
        object.items_mut()[0] = crate::tests::native_stack(stone_stack_of(1));
        assert!(object.set_slot_disabled(4, true));
    }
    place_analog_comparator(&mut crafter, comparator);
    assert_eq!(live_comparator_output(&mut crafter, comparator), 2);

    // `CommandBlock.getAnalogOutputSignal` (`:146-149`) is the success count.
    let mut command = analog_input_comparator_world(
        comparator,
        input,
        default_analog_state("minecraft:repeating_command_block"),
    );
    command
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, input)
        .and_then(BlockEntity::command_block_mut)
        .expect("the write created the command block's block entity")
        .set_success_count(7);
    place_analog_comparator(&mut command, comparator);
    assert_eq!(live_comparator_output(&mut command, comparator), 7);

    // `SculkSensorBlock.getAnalogOutputSignal` (`:269-275`) reads the saved
    // frequency only while the phase is ACTIVE.
    for (phase, expected) in [("active", 9), ("inactive", 0)] {
        let state = analog_state_of(
            "minecraft:sculk_sensor",
            &[
                ("sculk_sensor_phase", phase),
                ("power", "0"),
                ("waterlogged", "false"),
            ],
        );
        let mut sensor = analog_input_comparator_world(comparator, input, state);
        sensor
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, input)
            .and_then(BlockEntity::sculk_sensor_mut)
            .expect("the write created the sensor's block entity")
            .set_last_vibration_frequency(9);
        place_analog_comparator(&mut sensor, comparator);
        assert_eq!(
            live_comparator_output(&mut sensor, comparator),
            expected,
            "{phase}"
        );
    }
}

/// `ChestBlock.getAnalogOutputSignal` folds both
/// halves into one 54-slot container, so 27 full stacks in one half read as
/// half a container rather than a full one.
#[test]
fn a_live_comparator_reads_both_halves_of_a_double_chest() {
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let near = BlockPos { x: 1, y: 64, z: 0 };
    let far = BlockPos { x: 2, y: 64, z: 0 };

    fn fill(world: &mut ServerWorld, at: BlockPos) {
        let items = vec![stone_stack_of(64); 27];
        assert!(world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
            .and_then(BlockEntity::chest_mut)
            .expect("the write created the chest's block entity")
            .set_items(items.into_iter().map(crate::tests::native_stack).collect()));
    }

    let mut single = analog_input_comparator_world(comparator, near, chest_state("south", "right"));
    fill(&mut single, near);
    place_analog_comparator(&mut single, comparator);
    assert_eq!(live_comparator_output(&mut single, comparator), 15);

    let mut double = analog_input_comparator_world(comparator, near, chest_state("south", "right"));
    fill(&mut double, near);
    assert!(double.set_block(
        Java26BuiltinDimension::Overworld,
        far,
        chest_state("south", "left"),
        SET_BLOCK_UPDATE_ALL,
    ));
    place_analog_comparator(&mut double, comparator);
    assert_eq!(live_comparator_output(&mut double, comparator), 8);
}

/// `JukeboxBlock.getAnalogOutputSignal` is only
/// reachable now that `getSignal` answers: `DiodeBlock.getInputSignal`
/// asks that first, and a refusal there stopped the
/// comparator before it ever read the record.
#[test]
fn a_comparator_reads_the_song_of_the_record_the_jukebox_beside_it_holds() {
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let jukebox = BlockPos { x: 1, y: 64, z: 0 };

    let mut empty = analog_input_comparator_world(comparator, jukebox, jukebox_state(false));
    place_analog_comparator(&mut empty, comparator);
    assert_eq!(live_comparator_output(&mut empty, comparator), 0);

    // `register(context, CAT, MUSIC_DISC_CAT, 185, 2)`.
    let mut loaded = analog_input_comparator_world(comparator, jukebox, jukebox_state(false));
    assert!(loaded.set_the_jukebox_item(
        Java26BuiltinDimension::Overworld,
        jukebox,
        jukebox_state(true),
        crate::tests::native_stack(music_disc()),
    ));
    place_analog_comparator(&mut loaded, comparator);
    assert_eq!(live_comparator_output(&mut loaded, comparator), 2);
}

fn jukebox_state(has_record: bool) -> i32 {
    analog_state_of(
        "minecraft:jukebox",
        &[("has_record", if has_record { "true" } else { "false" })],
    )
}

fn music_disc() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_MUSIC_DISC_CAT_ID),
        count: 1,
        component_patch: Default::default(),
    }
}

/// `JukeboxBlock.getSignal` on a real level: the
/// record itself powers nothing, and it is `onSongChanged`'s neighbour update
/// that carries the song player's state out
/// to the lamp.
#[test]
fn a_jukebox_lights_a_lamp_while_its_song_player_runs_in_a_real_world() {
    let candidate = restore_candidate(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let jukebox = BlockPos { x: 1, y: 64, z: 1 };
    let lamp = BlockPos { x: 2, y: 64, z: 1 };
    for floor in [
        BlockPos { x: 1, y: 63, z: 1 },
        BlockPos { x: 2, y: 63, z: 1 },
    ] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            floor,
            block_state_id("minecraft:dirt"),
            SET_BLOCK_UPDATE_ALL,
        ));
    }
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        lamp,
        redstone_lamp_state(false),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        jukebox,
        jukebox_state(false),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(false))
    );

    assert!(world.set_the_jukebox_item(
        Java26BuiltinDimension::Overworld,
        jukebox,
        jukebox_state(true),
        crate::tests::native_stack(music_disc()),
    ));
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(true))
    );

    // Popping the record stops the player, and the lamp takes its own four
    // ticks to notice.
    assert!(world.set_the_jukebox_item(
        Java26BuiltinDimension::Overworld,
        jukebox,
        jukebox_state(false),
        crate::tests::native_stack(ItemStackSummary::empty()),
    ));
    for _ in 1..redstone_lamp::TICK_DELAY {
        world.tick(Vec::new());
    }
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(true))
    );
    world.tick(Vec::new());
    assert_eq!(
        live_block(&mut world, lamp),
        Some(redstone_lamp_state(false))
    );
}

/// `CreakingHeartBlock.getAnalogOutputSignal`
/// reads its object for every state but
/// `UPROOTED`, and the object's number is the distance to the creaking it is
/// bound to. The heart the world
/// stands here is bound the only way this server can bind one: through the
/// UUID a save or a command wrote, because `spawnProtector` has
/// no owner.
#[test]
fn a_live_comparator_follows_the_creaking_a_heart_is_bound_to() {
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let heart = BlockPos { x: 1, y: 64, z: 0 };
    let mut world = analog_input_comparator_world(comparator, heart, awake_creaking_heart());
    // An AWAKE heart only exists inside `EnvironmentAttributes.CREAKING_ACTIVE`
    // outside it the countdown releases the protector.
    world.set_game_time_for_test(13_000);
    world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(13_000),
        &mut Vec::new(),
    );
    let (creaking, uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
        Vec3d {
            x: 1.5,
            y: 72.0,
            z: 0.5,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_CREAKING_ID).unwrap(),
        ),
    );
    bind_creaking_heart(&mut world, heart, uuid);
    world.tick(Vec::new());
    place_analog_comparator(&mut world, comparator);

    // Eight blocks up the roaming radius is 15 - floor(8 / 32 * 15).
    assert_eq!(live_comparator_output(&mut world, comparator), 12);

    // Nothing else tells the comparator the creaking moved: the heart's own
    // tick notices the new number and arms it, and the comparator's two-tick
    // delay runs after that.
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, creaking)
        .expect("the creaking is still filed")
        .set_pos(Vec3d {
            x: 1.5,
            y: 68.0,
            z: 0.5,
        });
    world.tick(Vec::new());
    run_comparator_delay(&mut world);
    assert_eq!(live_comparator_output(&mut world, comparator), 14);

    // A creaking that leaves keeps the binding for the grace period and then
    // loses it, which is the refusal below rather than a zero.
    world.remove_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        creaking,
        crate::entity::RemovalReason::Killed,
    );
    for _ in 0..32 {
        world.tick(Vec::new());
    }
    assert_eq!(live_comparator_output(&mut world, comparator), 0);
}

/// The zero an uprooted heart answers is its own state's
/// the zero a standing one without a
/// protector would answer is the unowned spawn's, so it refuses instead.
#[test]
fn the_creaking_heart_answers_its_own_zero_and_refuses_the_spawn_it_never_runs() {
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let heart = BlockPos { x: 1, y: 64, z: 0 };

    let mut uprooted = analog_input_comparator_world(
        comparator,
        heart,
        default_analog_state("minecraft:creaking_heart"),
    );
    place_analog_comparator(&mut uprooted, comparator);
    assert_eq!(live_comparator_output(&mut uprooted, comparator), 0);

    let mut unbound = analog_input_comparator_world(comparator, heart, awake_creaking_heart());
    place_analog_comparator(&mut unbound, comparator);
    assert_eq!(live_comparator_output(&mut unbound, comparator), 0);
    // The comparator is what asked, so the refusal names its cell and the
    // fact that could not be established.
}

/// `DetectorRailBlock.getAnalogOutputSignal`
/// reads the minecarts standing in the rail's own box, and a comparator armed
/// by a plain `setBlock` reads them: the level `Level.setBlock`'s neighbour leg
/// runs on carries the same entity projection a
/// scheduled tick builds.
/// A detector rail written over the cell the minecarts already stand in.
///
/// The order is the fixture's whole shape now: `DetectorRailBlock.onPlace`
/// runs `checkPressed`, so a `powered` rail
/// written over an empty cell releases itself on the way in and a
/// pressed one only survives the write with a minecart under it.
fn place_detector_rail(world: &mut ServerWorld, rail: BlockPos, state: i32) {
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        rail,
        state,
        SET_BLOCK_UPDATE_ALL,
    ));
}

#[test]
fn a_set_block_armed_comparator_reads_the_detector_rails_minecarts() {
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let input = BlockPos { x: 1, y: 64, z: 0 };
    let powered_rail = analog_state_of(
        "minecraft:detector_rail",
        &[
            ("shape", "north_south"),
            ("powered", "true"),
            ("waterlogged", "false"),
        ],
    );
    // Inside `getSearchBB`, which insets the
    // rail's cell by 0.2 on both horizontal axes.
    let on_the_rail = Vec3d {
        x: f64::from(input.x) + 0.5,
        y: f64::from(input.y),
        z: f64::from(input.z) + 0.5,
    };

    // A plain minecart holds the rail down without answering either half of
    // `:147-155`, which is the reachable zero -- answered rather than refused
    // now that the projection is there.
    let mut empty =
        analog_input_comparator_world(comparator, input, block_state_id("minecraft:stone"));
    empty.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
        on_the_rail,
        crate::entity::EntityData::Minecart(crate::entity::MinecartData::new()),
    );
    place_detector_rail(&mut empty, input, powered_rail);
    place_analog_comparator(&mut empty, comparator);
    assert_eq!(live_comparator_output(&mut empty, comparator), 0);

    // Nine full stacks of a twenty-seven slot chest minecart:
    // `Mth.lerpDiscrete(9 / 27, 0, 15)` is five.
    let nine_stacks = || {
        let mut container = crate::entity::MinecartContainerData::new_chest();
        for slot in 0..9 {
            container
                .set_item(slot, Some(stone_minecart_stack(64)))
                .expect("a chest minecart slot");
        }
        container
    };
    let container = nine_stacks();
    let mut chest =
        analog_input_comparator_world(comparator, input, block_state_id("minecraft:stone"));
    chest.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        on_the_rail,
        crate::entity::EntityData::ChestMinecart(container),
    );
    place_detector_rail(&mut chest, input, powered_rail);
    place_analog_comparator(&mut chest, comparator);
    assert_eq!(live_comparator_output(&mut chest, comparator), 5);

    // `:148-150` takes the command block's success count ahead of any
    // container standing in the same box: the chest beside it would answer
    // five, so three is the command block being read rather than a zero.
    let mut command_block_data = crate::entity::CommandBlockMinecartData::new();
    command_block_data.set_success_count(3);
    let mut command_block =
        analog_input_comparator_world(comparator, input, block_state_id("minecraft:stone"));
    command_block.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        on_the_rail,
        crate::entity::EntityData::ChestMinecart(nine_stacks()),
    );
    command_block.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        on_the_rail,
        crate::entity::EntityData::CommandBlockMinecart(command_block_data),
    );
    place_detector_rail(&mut command_block, input, powered_rail);
    place_analog_comparator(&mut command_block, comparator);
    assert_eq!(live_comparator_output(&mut command_block, comparator), 3);
}

/// `DetectorRailBlock.onPlace`: the rail a
/// command writes over a standing minecart presses itself and reaches the
/// comparator beside it inside that one write, with no tick in between. The
/// tail was held back until every level that writes a block carried the
/// projection `checkPressed` and the analog read ask.
#[test]
fn a_placed_detector_rail_presses_against_the_minecart_already_standing_on_it() {
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let rail = BlockPos { x: 1, y: 64, z: 0 };
    let detector = |powered: &str| {
        analog_state_of(
            "minecraft:detector_rail",
            &[
                ("shape", "north_south"),
                ("powered", powered),
                ("waterlogged", "false"),
            ],
        )
    };
    let on_the_rail = Vec3d {
        x: f64::from(rail.x) + 0.5,
        y: f64::from(rail.y),
        z: f64::from(rail.z) + 0.5,
    };
    // The comparator's input cell holds a plain block until the rail replaces
    // it, so nothing has read a rail before the write under test.
    let mut world =
        analog_input_comparator_world(comparator, rail, block_state_id("minecraft:stone"));
    place_analog_comparator(&mut world, comparator);
    assert_eq!(live_comparator_output(&mut world, comparator), 0);

    // Nine full stacks of a twenty-seven slot chest minecart is five.
    let mut container = crate::entity::MinecartContainerData::new_chest();
    for slot in 0..9 {
        container
            .set_item(slot, Some(stone_minecart_stack(64)))
            .expect("a chest minecart slot");
    }
    world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        on_the_rail,
        crate::entity::EntityData::ChestMinecart(container),
    );

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        rail,
        detector("false"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, rail),
        Some(detector("true"))
    );
    run_comparator_delay(&mut world);
    assert_eq!(live_comparator_output(&mut world, comparator), 5);
}

/// `ShelfBlock.getAnalogOutputSignal`: one bit per
/// filled slot, and only towards the side behind the shelf.
#[test]
fn a_comparator_reads_a_shelf_slot_mask_only_from_behind_it() {
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let shelf = BlockPos { x: 1, y: 64, z: 0 };
    for (facing, expected) in [("east", 5), ("west", 0)] {
        let state = analog_state_of(
            "minecraft:oak_shelf",
            &[
                ("facing", facing),
                ("powered", "false"),
                ("side_chain", "unconnected"),
                ("waterlogged", "false"),
            ],
        );
        let mut world = analog_input_comparator_world(comparator, shelf, state);
        let slots = world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, shelf)
            .and_then(crate::block_entity::BlockEntity::shelf_mut)
            .expect("the shelf has its block entity")
            .items_mut();
        slots[0] = stone_minecart_stack(1);
        slots[2] = stone_minecart_stack(64);
        place_analog_comparator(&mut world, comparator);
        assert_eq!(
            live_comparator_output(&mut world, comparator),
            expected,
            "{facing}"
        );
    }
}

/// `JukeboxBlockEntity.onSongChanged` runs
/// `setChanged`, whose `updateNeighbourForOutputSignal` reaches a comparator
/// reading the jukebox through a solid block.
#[test]
fn a_record_inserted_into_a_jukebox_reaches_a_comparator_through_a_solid_block() {
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let conductor = BlockPos { x: 1, y: 64, z: 0 };
    let jukebox = BlockPos { x: 2, y: 64, z: 0 };
    let mut world =
        analog_input_comparator_world(comparator, conductor, block_state_id("minecraft:stone"));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        jukebox,
        jukebox_state(false),
        SET_BLOCK_UPDATE_ALL,
    ));
    place_analog_comparator(&mut world, comparator);
    assert_eq!(live_comparator_output(&mut world, comparator), 0);

    assert!(world.set_the_jukebox_item(
        Java26BuiltinDimension::Overworld,
        jukebox,
        jukebox_state(true),
        crate::tests::native_stack(music_disc()),
    ));
    run_comparator_delay(&mut world);
    assert_eq!(live_comparator_output(&mut world, comparator), 2);
}
