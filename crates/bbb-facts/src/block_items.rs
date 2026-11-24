//! `Item.BY_BLOCK`: the item a block turns back into.
//!
//! Vanilla fills this map from `BlockItem`'s constructor as `Items` registers
//! each one, so it is the item catalog read backwards rather than a table of
//! its own. Deriving it here keeps the generated catalog the single source and
//! leaves nothing to drift.

use crate::item_types::VANILLA_ITEM_TYPES;

/// `Block.asItem()` (`BlockBehaviour.asItem` through `Item.BY_BLOCK`).
///
/// `BY_BLOCK` is a plain map and every `BlockItem` constructor puts into it, so
/// when several items name the same block the last registration is the one
/// that survives. The catalog is in registry order, which is registration
/// order, so the last match here is that same item.
///
/// `StandingAndWallBlockItem.registerBlocks` also maps its wall block to the
/// same item (`StandingAndWallBlockItem.java:48-51`).
pub fn vanilla_block_item(block_type_id: i32) -> Option<i32> {
    VANILLA_ITEM_TYPES
        .iter()
        .rfind(|entry| {
            entry.block_item_behavior.is_some_and(|behavior| {
                behavior.block_type_id == block_type_id
                    || behavior.wall_block_type_id == Some(block_type_id)
            })
        })
        .map(|entry| entry.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_types::{vanilla_block_type_for_resource_id, VANILLA_BLOCK_STONE_ID};
    use crate::item_types::{vanilla_item_type_for_resource_id, VANILLA_ITEM_STONE_ID};

    #[test]
    fn a_block_answers_the_item_that_places_it() {
        assert_eq!(
            vanilla_block_item(VANILLA_BLOCK_STONE_ID),
            Some(VANILLA_ITEM_STONE_ID)
        );
    }

    #[test]
    fn a_wall_variant_answers_the_paired_item() {
        let wall = vanilla_block_type_for_resource_id("minecraft:oak_wall_sign")
            .expect("the catalog carries the wall sign");
        let standing = vanilla_block_type_for_resource_id("minecraft:oak_sign")
            .expect("the catalog carries the standing sign");
        let sign = vanilla_item_type_for_resource_id("minecraft:oak_sign")
            .expect("the catalog carries the sign item");
        assert_eq!(vanilla_block_item(wall.id), Some(sign.id));
        assert_eq!(vanilla_block_item(standing.id), Some(sign.id));

        let player_wall = vanilla_block_type_for_resource_id("minecraft:player_wall_head")
            .expect("the catalog carries the player wall head");
        let player_head = vanilla_item_type_for_resource_id("minecraft:player_head")
            .expect("the catalog carries the player head item");
        assert_eq!(vanilla_block_item(player_wall.id), Some(player_head.id));
    }

    #[test]
    fn every_block_item_names_a_block_that_answers_some_item() {
        for entry in VANILLA_ITEM_TYPES {
            let Some(behavior) = entry.block_item_behavior else {
                continue;
            };
            assert!(
                vanilla_block_item(behavior.block_type_id).is_some(),
                "{} places a block no item answers for",
                entry.resource_id
            );
            if let Some(wall_block_type_id) = behavior.wall_block_type_id {
                assert_eq!(
                    vanilla_block_item(wall_block_type_id),
                    Some(entry.id),
                    "{} registers its paired wall block",
                    entry.resource_id
                );
            }
        }
    }
}
