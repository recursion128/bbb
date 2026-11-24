//! `DaylightDetectorBlockEntity` (`DaylightDetectorBlockEntity.java:6-10`).
//!
//! Empty. POWER updates are `DaylightDetectorBlock.tickEntity`, not BE state.

/// `DaylightDetectorBlockEntity` (`:6-9`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DaylightDetectorBlockEntity {}

#[cfg(test)]
mod tests {
    use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_DAYLIGHT_DETECTOR_ID;
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_DAYLIGHT_DETECTOR_ID;
    use bbb_protocol::packets::BlockPos;

    use crate::block_entity::new_block_entity;

    #[test]
    fn a_daylight_detector_builds_an_empty_block_entity() {
        let state = vanilla_block_default_state_id(VANILLA_BLOCK_DAYLIGHT_DETECTOR_ID)
            .expect("minecraft:daylight_detector has a default state");
        let built = new_block_entity(BlockPos { x: 0, y: 0, z: 0 }, state)
            .expect("a daylight detector builds an object");
        assert_eq!(
            built.type_id(),
            VANILLA_BLOCK_ENTITY_TYPE_DAYLIGHT_DETECTOR_ID
        );
        assert!(built
            .update_tag_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds")
            .is_empty());
        assert!(
            crate::block_entity::block_entity_update_packet_with_resolver(
                &built,
                crate::block_entity::test_item_resolver()
            )
            .expect("fixture codec succeeds")
            .is_none()
        );
    }
}
