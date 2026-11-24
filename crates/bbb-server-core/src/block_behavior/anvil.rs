use crate::fluid::with_property;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_ANVIL_ID, VANILLA_BLOCK_CHIPPED_ANVIL_ID, VANILLA_BLOCK_DAMAGED_ANVIL_ID,
};

use super::{block_type_of, string_property};

// AnvilBlock.java:105-110.
pub fn damage(state: i32) -> Option<Option<i32>> {
    let target = match block_type_of(state)? {
        VANILLA_BLOCK_ANVIL_ID => VANILLA_BLOCK_CHIPPED_ANVIL_ID,
        VANILLA_BLOCK_CHIPPED_ANVIL_ID => VANILLA_BLOCK_DAMAGED_ANVIL_ID,
        _ => return Some(None),
    };
    let facing = string_property(state, "facing")?;
    Some(Some(with_property(
        vanilla_block_default_state_id(target)?,
        "facing",
        &facing,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anvil_damage_preserves_facing_through_both_stages() {
        {
            let facing = "east";
            let state = |block| {
                with_property(
                    vanilla_block_default_state_id(block).unwrap(),
                    "facing",
                    facing,
                )
                .unwrap()
            };
            assert_eq!(
                damage(state(VANILLA_BLOCK_ANVIL_ID)),
                Some(Some(state(VANILLA_BLOCK_CHIPPED_ANVIL_ID)))
            );
            assert_eq!(
                damage(state(VANILLA_BLOCK_CHIPPED_ANVIL_ID)),
                Some(Some(state(VANILLA_BLOCK_DAMAGED_ANVIL_ID)))
            );
            assert_eq!(damage(state(VANILLA_BLOCK_DAMAGED_ANVIL_ID)), Some(None));
        }
    }
}
