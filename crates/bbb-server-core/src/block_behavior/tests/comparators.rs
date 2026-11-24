use bbb_protocol::packets::BlockPos;

use crate::block_behavior::comparator::calculate_output;
use crate::block_behavior::{string_property, tick_block, use_without_item};
use crate::block_entity::hopper::HopperBlockEntity;
use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
use bbb_protocol::packets::{Direction, ItemStackSummary};
use bbb_server_types::block_queries::bool_property;
use bbb_server_types::directions::relative;

use super::{interaction_player, state_of, stone, NativeWorld};

const COMPARATOR: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const INPUT: BlockPos = BlockPos { x: 0, y: 64, z: 1 };
const FLOOR: BlockPos = BlockPos { x: 0, y: 63, z: 0 };

fn comparator_state(facing: &str, powered: bool, mode: &str) -> i32 {
    state_of(
        "minecraft:comparator",
        &[
            ("facing", facing),
            ("powered", if powered { "true" } else { "false" }),
            ("mode", mode),
        ],
    )
}

fn redstone_power(power: i32) -> i32 {
    state_of(
        "minecraft:redstone_wire",
        &[
            ("east", "none"),
            ("north", "none"),
            ("south", "none"),
            ("west", "none"),
            ("power", &power.to_string()),
        ],
    )
}

fn cake(bites: i32) -> i32 {
    state_of("minecraft:cake", &[("bites", &bites.to_string())])
}

fn world_with_comparator(mode: &str) -> NativeWorld {
    let mut world = NativeWorld::ticking(FLOOR);
    world.set(FLOOR, stone());
    world.set(COMPARATOR, comparator_state("south", false, mode));
    world
}

#[test]
fn a_compare_comparator_copies_front_wire_power_into_its_output() {
    let mut world = world_with_comparator("compare");
    load_saved_wire(&mut world, INPUT, 10);
    world.neighbor_changed(
        COMPARATOR,
        bbb_server_types::block_queries::block_type_of(redstone_power(10)).unwrap(),
    );
    world.run_tick();
    world.run_tick();
    assert_eq!(
        bool_property(world.state(COMPARATOR), "powered"),
        Some(true)
    );
    assert_eq!(world.comparator_output(COMPARATOR), 10);
}

#[test]
fn a_subtract_comparator_zeros_when_the_side_is_stronger() {
    let mut world = world_with_comparator("subtract");
    load_saved_wire(&mut world, INPUT, 4);
    load_saved_wire(&mut world, BlockPos { x: 1, y: 64, z: 0 }, 8);
    world.neighbor_changed(
        COMPARATOR,
        bbb_server_types::block_queries::block_type_of(redstone_power(4)).unwrap(),
    );
    world.run_tick();
    world.run_tick();
    assert_eq!(
        bool_property(world.state(COMPARATOR), "powered"),
        Some(false)
    );
    assert_eq!(world.comparator_output(COMPARATOR), 0);
}

#[test]
fn side_equality_distinguishes_compare_from_subtract() {
    for (mode, expected_output, expected_powered) in [("compare", 8, true), ("subtract", 0, false)]
    {
        let mut world = world_with_comparator(mode);
        load_saved_wire(&mut world, INPUT, 8);
        load_saved_wire(&mut world, relative(COMPARATOR, Direction::West), 8);
        world.neighbor_changed(COMPARATOR, 0);
        world.run_tick();
        world.run_tick();
        assert_eq!(
            world.comparator_output(COMPARATOR),
            expected_output,
            "{mode}"
        );
        assert_eq!(
            bool_property(world.state(COMPARATOR), "powered"),
            Some(expected_powered),
            "{mode}"
        );
    }
}

#[test]
fn a_direct_cake_input_sets_the_comparator_output() {
    let mut direct = world_with_comparator("compare");
    direct.set(INPUT, cake(4));
    direct.neighbor_changed(COMPARATOR, 0);
    direct.run_tick();
    direct.run_tick();
    assert_eq!(direct.comparator_output(COMPARATOR), 6);
}

#[test]
fn interaction_cycles_mode_with_exact_sound_parameters() {
    let mut forbidden = world_with_comparator("compare");
    let state = forbidden.state(COMPARATOR);
    assert!(!forbidden.with_level(|level| use_without_item(
        level,
        COMPARATOR,
        state,
        interaction_player(false, Direction::North)
    )));
    assert!(forbidden.sounds().is_empty());

    let mut world = world_with_comparator("compare");
    let state = world.state(COMPARATOR);
    assert!(world.with_level(|level| use_without_item(
        level,
        COMPARATOR,
        state,
        interaction_player(true, Direction::North)
    )));
    assert_eq!(
        string_property(world.state(COMPARATOR), "mode").as_deref(),
        Some("subtract")
    );
    assert_eq!(
        world
            .sounds()
            .into_iter()
            .map(|sound| (
                sound.pos,
                sound.resource_id,
                sound.volume,
                sound.pitch,
                sound.except_entity_id
            ))
            .collect::<Vec<_>>(),
        [(
            COMPARATOR,
            "minecraft:block.comparator.click".to_string(),
            0.3,
            0.55,
            Some(interaction_player(true, Direction::North).entity_id)
        )]
    );

    let state = world.state(COMPARATOR);
    assert!(world.with_level(|level| use_without_item(
        level,
        COMPARATOR,
        state,
        interaction_player(true, Direction::North)
    )));
    assert_eq!(world.sounds()[0].pitch, 0.5);
}

fn hopper() -> i32 {
    state_of(
        "minecraft:hopper",
        &[("facing", "down"), ("enabled", "true")],
    )
}

fn hopper_slots_with_stone(count: i32) -> (Vec<ItemStackSummary>, i32) {
    let mut hopper = HopperBlockEntity::default();
    hopper.items_mut()[0] = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count,
        component_patch: Default::default(),
    }
    .into();
    let analog = HopperBlockEntity::redstone_signal(&hopper).unwrap();
    (
        hopper
            .items()
            .iter()
            .map(|item| item.summary().clone())
            .collect(),
        analog,
    )
}

#[test]
fn hopper_analog_feeds_compare_and_subtract_through_the_tick_dispatcher() {
    let (items, analog) = hopper_slots_with_stone(64);
    assert!(analog > 0);

    let mut compare = world_with_comparator("compare");
    compare.set(INPUT, hopper());
    compare.with_level(|level| {
        level
            .set_container_items(INPUT, items.clone().into_iter().map(Into::into).collect())
            .unwrap()
    });
    assert_eq!(
        compare.with_level(|level| level.analog_output_signal(INPUT, Direction::North)),
        analog
    );
    let state = compare.state(COMPARATOR);
    let output = compare.with_level(|level| calculate_output(level, COMPARATOR, state));
    assert_eq!(output, analog);
    compare.with_level(|level| tick_block(level, COMPARATOR, state));
    assert_eq!(
        compare.with_level(|level| level
            .comparator_block_entity_output_signal(COMPARATOR)
            .unwrap()),
        output
    );
    assert_eq!(
        bool_property(compare.state(COMPARATOR), "powered"),
        Some(true)
    );

    let mut subtract = world_with_comparator("subtract");
    subtract.set(INPUT, hopper());
    subtract.with_level(|level| {
        level
            .set_container_items(INPUT, items.into_iter().map(Into::into).collect())
            .unwrap()
    });
    load_saved_wire(&mut subtract, BlockPos { x: 1, y: 64, z: 0 }, 1);
    let state = subtract.state(COMPARATOR);
    let output = subtract.with_level(|level| calculate_output(level, COMPARATOR, state));
    subtract.with_level(|level| tick_block(level, COMPARATOR, state));
    assert_eq!(
        subtract.with_level(|level| level
            .comparator_block_entity_output_signal(COMPARATOR)
            .unwrap()),
        output
    );
    assert!(output < analog);
}

fn chest(facing: &str, kind: &str) -> i32 {
    state_of(
        "minecraft:chest",
        &[("facing", facing), ("type", kind), ("waterlogged", "false")],
    )
}

fn stone_slots(filled: usize, size: usize) -> Vec<ItemStackSummary> {
    let mut items = vec![ItemStackSummary::empty(); size];
    items[..filled].fill(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count: 64,
        component_patch: Default::default(),
    });
    items
}

#[test]
fn a_double_chest_is_one_container_to_the_comparator() {
    const RIGHT: BlockPos = INPUT;
    let left = BlockPos {
        x: INPUT.x - 1,
        ..INPUT
    };

    let mut single = world_with_comparator("compare");
    single.set(RIGHT, chest("north", "single"));
    single.with_level(|level| {
        level
            .set_container_items(
                RIGHT,
                stone_slots(27, 27).into_iter().map(Into::into).collect(),
            )
            .unwrap()
    });
    assert_eq!(
        single.with_level(|level| level.analog_output_signal(RIGHT, Direction::North)),
        15
    );

    let mut double = world_with_comparator("compare");
    double.set(RIGHT, chest("north", "right"));
    double.set(left, chest("north", "left"));
    double.with_level(|level| {
        level
            .set_container_items(
                RIGHT,
                stone_slots(27, 27).into_iter().map(Into::into).collect(),
            )
            .unwrap()
    });
    double.with_level(|level| {
        level
            .set_container_items(
                left,
                stone_slots(0, 27).into_iter().map(Into::into).collect(),
            )
            .unwrap()
    });
    assert_eq!(
        double.with_level(|level| level.analog_output_signal(RIGHT, Direction::North)),
        8
    );
    assert_eq!(
        double.with_level(|level| level.analog_output_signal(left, Direction::North)),
        8,
        "either half answers for the whole compound container"
    );
}

#[test]
fn a_chest_blocked_from_above_reads_zero_rather_than_its_contents() {
    let above = BlockPos {
        y: INPUT.y + 1,
        ..INPUT
    };

    let mut open = world_with_comparator("compare");
    open.set(INPUT, chest("north", "single"));
    open.with_level(|level| {
        level
            .set_container_items(
                INPUT,
                stone_slots(27, 27).into_iter().map(Into::into).collect(),
            )
            .unwrap()
    });
    assert_eq!(
        open.with_level(|level| level.analog_output_signal(INPUT, Direction::North)),
        15
    );

    let mut blocked = world_with_comparator("compare");
    blocked.set(INPUT, chest("north", "single"));
    blocked.with_level(|level| {
        level
            .set_container_items(
                INPUT,
                stone_slots(27, 27).into_iter().map(Into::into).collect(),
            )
            .unwrap()
    });
    blocked.set(above, stone());
    assert_eq!(
        blocked.with_level(|level| level.analog_output_signal(INPUT, Direction::North)),
        0
    );
}

fn load_saved_wire(world: &mut NativeWorld, pos: BlockPos, power: i32) {
    use bbb_server_types::save::Java26BuiltinDimension;
    // Loaded wire power is a saved input, before the comparator reads it.
    world.world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        redstone_power(0),
        2 | 16,
    );
    world.world.set_raw_block_state_for_test(
        Java26BuiltinDimension::Overworld,
        pos,
        redstone_power(power),
    );
}

/// `ComparatorBlock.updateShape` (`ComparatorBlock.java:51-63`) asks the live
/// support shape, and a moving piston has none.
#[test]
fn a_comparator_pops_when_its_support_turns_into_a_moving_piston() {
    let mut world = world_with_comparator("compare");
    world.set(
        FLOOR,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_MOVING_PISTON_ID,
        )
        .unwrap(),
    );
    world.shape_update(COMPARATOR, Direction::Down);
    assert_eq!(world.state(COMPARATOR), super::air());
}
