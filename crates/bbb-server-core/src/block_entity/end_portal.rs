//! `TheEndPortalBlockEntity` (`TheEndPortalBlockEntity.java:7-18`).
//!
//! Empty. `shouldRenderFace` is client.

/// `TheEndPortalBlockEntity` (`:7-14`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TheEndPortalBlockEntity {}

#[cfg(test)]
mod tests {
    use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_END_PORTAL_ID;
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_END_PORTAL_ID;
    use bbb_protocol::packets::BlockPos;

    use crate::block_entity::new_block_entity;

    #[test]
    fn an_end_portal_builds_an_empty_block_entity() {
        let state = vanilla_block_default_state_id(VANILLA_BLOCK_END_PORTAL_ID)
            .expect("minecraft:end_portal has a default state");
        let built = new_block_entity(BlockPos { x: 0, y: 0, z: 0 }, state)
            .expect("an end portal builds an object");
        assert_eq!(built.type_id(), VANILLA_BLOCK_ENTITY_TYPE_END_PORTAL_ID);
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
