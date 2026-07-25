use super::*;

impl WorldStore {
    /// Predicts the logical-client half of `FishingRodItem.use`.
    ///
    /// Both cast and retrieve return client-swing `SUCCESS`. Bobber
    /// spawn/removal, retrieval durability, enchantments, stats, and game
    /// events are server-owned. The local `playSound(null, ...)` call is
    /// suppressed but still consumes its pitch random and sound seed.
    pub(super) fn predict_local_fishing_rod_item_use(&self) -> LocalItemUsePrediction {
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::fishing_rod_use(
                self.local_player_fishing_bobber_id().is_some(),
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        entity_types::VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        item_types::VANILLA_ITEM_FISHING_ROD_ID,
        packets::{
            AddEntity, CommonPlayerSpawnInfo, ItemStackSummary, PlayLogin, SetPlayerInventory,
            Vec3d as ProtocolVec3d,
        },
    };
    use uuid::Uuid;

    fn login(world: &mut WorldStore, player_id: i32) {
        world.apply_login(&PlayLogin {
            player_id,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
    }

    fn add_fishing_bobber(world: &mut WorldStore, entity_id: i32, owner_id: i32) {
        world.apply_add_entity(AddEntity {
            id: entity_id,
            uuid: Uuid::from_u128(entity_id as u128),
            entity_type_id: VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: owner_id,
        });
    }

    #[test]
    fn fishing_rod_prediction_preserves_cast_and_retrieve_sound_identity() {
        let mut world = WorldStore::new();
        login(&mut world, 42);
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_FISHING_ROD_ID),
                count: 1,
                component_patch: Default::default(),
            },
        }));
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::fishing_rod_use(false)
            ))
        );

        add_fishing_bobber(&mut world, 88, 42);
        assert_eq!(world.local_player_fishing_bobber_id(), Some(88));
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::fishing_rod_use(true)
            ))
        );
    }
}
