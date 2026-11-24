//! `CopperBulbBlock` identity, POWERED/LIT and analog.

use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::packets::BlockPos;

use super::{state_of, stone, NativeWorld};

const BULB: BlockPos = BlockPos { x: 0, y: 71, z: 0 };
const SIDE: BlockPos = BlockPos { x: 1, y: 71, z: 0 };

fn bulb(name: &str, lit: bool, powered: bool) -> i32 {
    state_of(
        name,
        &[
            ("lit", if lit { "true" } else { "false" }),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
}

fn lit(state: i32) -> Option<String> {
    vanilla_block_state_identity(state)?
        .properties
        .get("lit")
        .cloned()
}

fn powered(state: i32) -> Option<String> {
    vanilla_block_state_identity(state)?
        .properties
        .get("powered")
        .cloned()
}

fn redstone_block() -> i32 {
    super::default_state("minecraft:redstone_block")
}

#[test]
fn a_rising_edge_cycles_lit_and_a_falling_edge_keeps_it() {
    let mut world = NativeWorld::new();
    world.set(BULB, bulb("minecraft:waxed_copper_bulb", false, false));
    world.set(SIDE, redstone_block());
    world.neighbor_changed(
        BULB,
        bbb_server_types::block_queries::block_type_of(redstone_block()).unwrap(),
    );
    assert_eq!(lit(world.state(BULB)).as_deref(), Some("true"));
    assert_eq!(powered(world.state(BULB)).as_deref(), Some("true"));
    assert_eq!(
        crate::block_behavior::copper_bulb::analog_output(world.state(BULB)),
        15
    );

    world.set(SIDE, stone());
    world.neighbor_changed(
        BULB,
        bbb_server_types::block_queries::block_type_of(stone()).unwrap(),
    );
    assert_eq!(lit(world.state(BULB)).as_deref(), Some("true"));
    assert_eq!(powered(world.state(BULB)).as_deref(), Some("false"));
    assert_eq!(
        crate::block_behavior::copper_bulb::analog_output(world.state(BULB)),
        15
    );

    world.set(SIDE, redstone_block());
    world.neighbor_changed(
        BULB,
        bbb_server_types::block_queries::block_type_of(redstone_block()).unwrap(),
    );
    assert_eq!(lit(world.state(BULB)).as_deref(), Some("false"));
    assert_eq!(powered(world.state(BULB)).as_deref(), Some("true"));
    assert_eq!(
        crate::block_behavior::copper_bulb::analog_output(world.state(BULB)),
        0
    );
}

/// what a comparator in front of the bulb reads.
#[test]
fn a_comparator_copies_a_lit_bulb_s_analog() {
    use bbb_server_types::block_queries::bool_property;
    const COMPARATOR: BlockPos = BlockPos { x: 0, y: 71, z: 0 };
    const INPUT: BlockPos = BlockPos { x: 0, y: 71, z: 1 };
    const FLOOR: BlockPos = BlockPos { x: 0, y: 70, z: 0 };
    let mut world = NativeWorld::ticking(COMPARATOR);
    world.set(FLOOR, stone());
    world.set(
        COMPARATOR,
        state_of(
            "minecraft:comparator",
            &[
                ("facing", "south"),
                ("powered", "false"),
                ("mode", "compare"),
            ],
        ),
    );
    world.set(INPUT, bulb("minecraft:copper_bulb", true, false));
    let input_type = bbb_server_types::block_queries::block_type_of(world.state(INPUT)).unwrap();
    world.neighbor_changed(COMPARATOR, input_type);
    world.run_tick();
    world.run_tick();
    assert_eq!(
        bool_property(world.state(COMPARATOR), "powered"),
        Some(true)
    );
    assert_eq!(world.comparator_output(COMPARATOR), 15);
}
