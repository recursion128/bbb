use super::*;

impl WorldStore {
    /// Predicts the logical-client half of `FireworkRocketItem.use`.
    ///
    /// Fall-flying use returns client-swing `SUCCESS`. Leash disconnection,
    /// launch sound, rocket spawn, stack consumption, and stats are all inside
    /// vanilla's `ServerLevel` branch. Grounded use passes to the other hand.
    pub(super) fn predict_local_firework_rocket_item_use(&self) -> LocalItemUsePrediction {
        let fall_flying = self
            .local_player_id()
            .and_then(|id| self.entities.is_fall_flying(id))
            .unwrap_or(false);
        LocalItemUsePrediction::Result(if fall_flying {
            LocalInteractionResult::Success(LocalInteractionSuccess::client_item_interaction())
        } else {
            LocalInteractionResult::Pass
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        item_types::VANILLA_ITEM_FIREWORK_ROCKET_ID,
        packets::{
            AddEntity, EntityDataValue, EntityDataValueKind, ItemStackSummary, SetEntityData,
            SetPlayerInventory, Vec3d,
        },
    };
    use uuid::Uuid;

    #[test]
    fn firework_rocket_use_reads_canonical_fall_flying_flag() {
        let mut world = WorldStore::new();
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_FIREWORK_ROCKET_ID),
                count: 2,
                component_patch: Default::default(),
            },
        }));
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
        );

        world.apply_add_entity(AddEntity {
            id: 7,
            uuid: Uuid::from_u128(7),
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: Vec3d::default(),
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world.local_player_id = Some(7);
        let accessors = crate::entities::vanilla_entity_metadata_entity_accessors().unwrap();
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 7,
            values: vec![EntityDataValue {
                data_id: accessors.shared_flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(
                    accessors.shared_flags_default | accessors.fall_flying_mask,
                ),
            }],
        }));

        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .map(|stack| stack.count),
            Some(2)
        );
        assert_eq!(world.entity_count(), 1);
    }
}
