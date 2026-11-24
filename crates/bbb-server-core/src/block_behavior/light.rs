//! `LightBlock` identity and `LEVEL` (`LightBlock.java:31-63`).

use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseWithoutItemOverride};

use super::{block_type_of, cycle_property};

/// `LightBlock.LEVEL` (`LightBlock.java:34`).
const LEVEL: &str = "level";

/// `LightBlock.useWithoutItem` is unique to that class.
#[must_use]
pub fn is_light(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::LightBlock)
}

/// `state.cycle(LEVEL)` (`LightBlock.java:58`).
#[must_use]
pub fn cycled_level(block_state_id: i32) -> Option<i32> {
    cycle_property(block_state_id, LEVEL)
}

#[cfg(test)]
mod tests {
    use crate::fluid::with_property;
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::vanilla_block_type_for_resource_id;

    use super::*;
    use bbb_server_types::block_queries::{bool_property, int_property};

    fn default_light() -> i32 {
        let block = vanilla_block_type_for_resource_id("minecraft:light").expect("light");
        vanilla_block_default_state_id(block.id).expect("default light")
    }

    #[test]
    fn default_light_matches_the_generated_override() {
        assert!(is_light(default_light()));
        assert_eq!(int_property(default_light(), LEVEL), Some(15));
    }

    #[test]
    fn cycle_wraps_fifteen_to_zero_and_keeps_waterlogged() {
        let start = with_property(default_light(), "waterlogged", "true").expect("waterlogged");
        let cycled = cycled_level(start).expect("LEVEL cycles");
        assert_eq!(int_property(cycled, LEVEL), Some(0));
        assert_eq!(bool_property(cycled, "waterlogged"), Some(true));
    }
}
