//! `BedBlockEntity` (`BedBlockEntity.java:9-28`).
//!
//! Color is on the block. The object carries no extra NBT.

/// `BedBlockEntity` (`:9-10`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BedBlockEntity {}

#[cfg(test)]
mod tests {
    use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_BED_ID;
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_WHITE_BED_ID;
    use bbb_protocol::packets::BlockPos;

    use crate::block_entity::new_block_entity;

    #[test]
    fn a_white_bed_builds_a_bed_block_entity() {
        let state = vanilla_block_default_state_id(VANILLA_BLOCK_WHITE_BED_ID)
            .expect("minecraft:white_bed has a default state");
        let built = new_block_entity(BlockPos { x: 0, y: 0, z: 0 }, state)
            .expect("a white bed builds an object");
        assert_eq!(built.type_id(), VANILLA_BLOCK_ENTITY_TYPE_BED_ID);
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
            .is_some()
        );
    }
}
