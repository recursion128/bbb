use super::*;

const VANILLA_BUNDLE_USE_DURATION_TICKS: u32 = 200;
const VANILLA_BUNDLE_REPEAT_DELAY_TICKS: u32 = 10;

impl WorldStore {
    /// Advances the local `LivingEntity.updateUsingItem` clock and applies the
    /// logical-client work owned by item `onUseTick` implementations.
    ///
    /// Bundle removal is the first such owner. Each returned drop must be
    /// expanded by the native boundary into remove-one sound, main-hand swing,
    /// and suppressed drop-contents sound in that order. Client-side
    /// `Player.drop` creates no local item entity.
    pub fn advance_local_using_item_ticks_with_actions(
        &mut self,
        ticks: u32,
    ) -> LocalUsingItemTickActions {
        let mut actions = LocalUsingItemTickActions::default();
        if ticks == 0 {
            return actions;
        }
        let using_bundle = self
            .local_using_item_item_id()
            .and_then(vanilla_item_type)
            .and_then(|item_type| item_type.use_override)
            == Some(VanillaItemUseOverride::BundleItem);
        if !self.local_player().interaction.using_item || !using_bundle {
            self.advance_local_using_item_ticks(ticks);
            return actions;
        }

        for _ in 0..ticks {
            let using_ticks = self.local_player().interaction.using_item_ticks;
            if bundle_drops_content_at_using_tick(using_ticks) {
                let hand = self
                    .local_player()
                    .interaction
                    .using_item_hand
                    .unwrap_or(InteractionHand::MainHand);
                let selected_hotbar_slot = self.local_player().selected_hotbar_slot;
                if self
                    .inventory
                    .remove_one_local_bundle_item(selected_hotbar_slot, hand)
                {
                    actions.bundle_drops = actions.bundle_drops.saturating_add(1);
                }
            }
            self.advance_local_using_item_ticks(1);
        }
        actions
    }
}

fn bundle_drops_content_at_using_tick(using_ticks: u32) -> bool {
    using_ticks == 0
        || (using_ticks > VANILLA_BUNDLE_REPEAT_DELAY_TICKS
            && (i64::from(VANILLA_BUNDLE_USE_DURATION_TICKS) - i64::from(using_ticks))
                .rem_euclid(2)
                == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        item_types::{
            VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_ARROW_ID, VANILLA_ITEM_BUNDLE_ID,
            VANILLA_ITEM_RED_BUNDLE_ID, VANILLA_ITEM_STONE_ID,
        },
        packets::{
            DataComponentPatchSummary, ItemStackSummary, ItemStackTemplateSummary,
            SetPlayerInventory,
        },
    };

    fn template(item_id: i32) -> ItemStackTemplateSummary {
        ItemStackTemplateSummary {
            item_id,
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        }
    }

    fn bundle_stack(bundle_item_id: i32, item_ids: &[i32]) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(bundle_item_id),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
                bundle_contents_items: item_ids.iter().copied().map(template).collect(),
                bundle_contents_item_count: Some(item_ids.len()),
                ..DataComponentPatchSummary::default()
            },
        }
    }

    #[test]
    fn bundle_use_starts_with_client_swing_and_tick_drops_selected_then_first_item() {
        let mut world = WorldStore::new();
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: bundle_stack(
                VANILLA_ITEM_RED_BUNDLE_ID,
                &[
                    VANILLA_ITEM_STONE_ID,
                    VANILLA_ITEM_APPLE_ID,
                    VANILLA_ITEM_ARROW_ID,
                ],
            ),
        }));
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::bundle_use()
            ))
        );
        assert!(world.apply_local_select_bundle_item(0, 1));

        world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        let actions = world.advance_local_using_item_ticks_with_actions(13);
        assert_eq!(actions, LocalUsingItemTickActions { bundle_drops: 2 });
        assert_eq!(world.local_player().interaction.using_item_ticks, 13);
        assert!(world.local_player().interaction.using_item);
        assert_eq!(
            world.inventory().hotbar_item_states()[0].local_selected_bundle_item_index(),
            None
        );
        let bundle = world
            .local_item_in_hand(InteractionHand::MainHand)
            .expect("held bundle");
        assert_eq!(bundle.component_patch.bundle_contents_item_count, Some(1));
        assert_eq!(
            bundle
                .component_patch
                .bundle_contents_items
                .iter()
                .map(|item| item.item_id)
                .collect::<Vec<_>>(),
            vec![VANILLA_ITEM_ARROW_ID]
        );

        assert_eq!(
            world.advance_local_using_item_ticks_with_actions(187),
            LocalUsingItemTickActions { bundle_drops: 1 }
        );
        assert_eq!(world.local_player().interaction.using_item_ticks, 200);
        assert!(world.local_player().interaction.using_item);
        let empty_bundle = world
            .local_item_in_hand(InteractionHand::MainHand)
            .expect("held empty bundle");
        assert_eq!(
            empty_bundle.component_patch.bundle_contents_item_count,
            None
        );
        assert!(!empty_bundle
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID));
        assert!(world
            .take_pending_local_interaction_sound_actions()
            .is_empty());
    }

    #[test]
    fn empty_removed_and_malformed_bundle_contents_do_not_emit_tick_actions() {
        let mut malformed = bundle_stack(VANILLA_ITEM_BUNDLE_ID, &[VANILLA_ITEM_STONE_ID]);
        malformed.component_patch.bundle_contents_item_count = Some(2);
        let mut removed = bundle_stack(VANILLA_ITEM_BUNDLE_ID, &[VANILLA_ITEM_STONE_ID]);
        removed
            .component_patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);

        for stack in [
            bundle_stack(VANILLA_ITEM_BUNDLE_ID, &[]),
            malformed,
            removed,
        ] {
            let mut world = WorldStore::new();
            assert!(world.apply_set_player_inventory(SetPlayerInventory {
                slot: 0,
                item: stack,
            }));
            world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
            assert_eq!(
                world.advance_local_using_item_ticks_with_actions(1),
                LocalUsingItemTickActions::default()
            );
            assert!(world
                .take_pending_local_interaction_sound_actions()
                .is_empty());
        }
    }
}
