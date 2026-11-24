use bbb_protocol::packets::BlockPos;

use super::{state_of, stone, NativeWorld};

const LECTERN: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const FLOOR: BlockPos = BlockPos { x: 0, y: 63, z: 0 };

fn lectern(has_book: bool, powered: bool) -> i32 {
    state_of(
        "minecraft:lectern",
        &[
            ("facing", "north"),
            ("has_book", if has_book { "true" } else { "false" }),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
}

fn powered(state: i32) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_identity(state)?
        .properties
        .get("powered")
        .cloned()
}

#[test]
fn a_powered_lectern_answers_fifteen() {
    use crate::block_behavior::lectern::signal;
    assert_eq!(signal(lectern(true, false)), 0);
    assert_eq!(signal(lectern(true, true)), 15);
}

#[test]
fn a_page_change_pulses_powered_for_two_ticks() {
    let mut world = NativeWorld::ticking(FLOOR);
    world.set(FLOOR, stone());
    world.set(LECTERN, lectern(true, false));
    world.with_level(|level| {
        crate::block_behavior::lectern::signal_page_change(level, LECTERN, lectern(true, false))
    });
    assert_eq!(powered(world.state(LECTERN)).as_deref(), Some("true"));
    world.run_tick();
    assert_eq!(powered(world.state(LECTERN)).as_deref(), Some("true"));
    world.run_tick();
    assert_eq!(powered(world.state(LECTERN)).as_deref(), Some("false"));
}
