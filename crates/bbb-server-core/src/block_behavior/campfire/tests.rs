use super::*;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_CAMPFIRE_ID, VANILLA_BLOCK_SOUL_CAMPFIRE_ID};

#[test]
fn campfire_inside_uses_lit_and_living_gates() {
    for (block, damage) in [
        (VANILLA_BLOCK_CAMPFIRE_ID, 1.0),
        (VANILLA_BLOCK_SOUL_CAMPFIRE_ID, 2.0),
    ] {
        for lit in [false, true] {
            for living in [false, true] {
                {
                    let state = vanilla_block_default_state_id(block).unwrap();
                    let state =
                        with_property(state, "lit", if lit { "true" } else { "false" }).unwrap();
                    assert_eq!(
                        inside_damage(state, living),
                        Some((lit && living).then_some(damage))
                    );
                }
            }
        }
    }
}
