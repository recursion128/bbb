use super::*;
use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
    item_types::VANILLA_ITEM_FIREWORK_ROCKET_ID, packets::ItemStackSummary,
};

const VANILLA_ARROWS_ITEM_TAG: &str = "minecraft:arrows";

impl WorldStore {
    /// Predicts the logical-client half of `CrossbowItem.use`.
    ///
    /// Charging sounds, projectile loading, charged-component mutation,
    /// shooting, durability, and ammo consumption are all server-only in
    /// vanilla 26.1. The client chooses only the consume/fail result and
    /// whether an uncharged crossbow starts its using-item clock.
    pub(super) fn predict_local_crossbow_item_use(
        &self,
        stack: &ItemStackSummary,
    ) -> LocalItemUsePrediction {
        if local_crossbow_is_charged(stack) {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false),
            ));
        }

        match self.local_player_has_crossbow_projectile() {
            Some(true) => LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true),
            )),
            Some(false) => LocalItemUsePrediction::Result(LocalInteractionResult::Fail),
            None => LocalItemUsePrediction::MissingRegistryTag(VANILLA_ARROWS_ITEM_TAG),
        }
    }

    fn local_player_has_crossbow_projectile(&self) -> Option<bool> {
        let has_held_firework = [InteractionHand::OffHand, InteractionHand::MainHand]
            .into_iter()
            .filter_map(|hand| self.local_item_in_hand(hand))
            .filter_map(|item| item.item_id)
            .any(|item_id| item_id == VANILLA_ITEM_FIREWORK_ROCKET_ID);
        if has_held_firework
            || self
                .local_player()
                .abilities
                .is_some_and(|abilities| abilities.instabuild)
        {
            return Some(true);
        }
        self.local_player_has_item_in_tag(VANILLA_ARROWS_ITEM_TAG)
    }
}

fn local_crossbow_is_charged(stack: &ItemStackSummary) -> bool {
    !stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID)
        && stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID)
        && !stack.component_patch.charged_projectiles_items.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        item_types::{VANILLA_ITEM_ARROW_ID, VANILLA_ITEM_CROSSBOW_ID, VANILLA_ITEM_STONE_ID},
        packets::{
            DataComponentPatchSummary, ItemStackTemplateSummary, PlayerAbilities, RegistryTags,
            SetPlayerInventory, TagNetworkPayload, UpdateTags,
        },
    };

    fn stack(item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        }
    }

    fn charged_crossbow(projectile_item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_CROSSBOW_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID],
                charged_projectiles_items: vec![ItemStackTemplateSummary {
                    item_id: projectile_item_id,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            },
        }
    }

    fn install_arrows_tag(world: &mut WorldStore, entries: Vec<i32>) {
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_ARROWS_ITEM_TAG.to_string(),
                    entries,
                }],
            }],
        });
    }

    fn set_slot(world: &mut WorldStore, slot: i32, item: ItemStackSummary) {
        assert!(world.apply_set_player_inventory(SetPlayerInventory { slot, item }));
    }

    #[test]
    fn charged_crossbow_consumes_without_starting_use_or_mutating_client_components() {
        let mut world = WorldStore::new();
        let crossbow = charged_crossbow(VANILLA_ITEM_ARROW_ID);
        set_slot(&mut world, 0, crossbow.clone());

        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false)
            ))
        );
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&crossbow)
        );

        let mut removed = charged_crossbow(VANILLA_ITEM_ARROW_ID);
        removed
            .component_patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID);
        set_slot(&mut world, 0, removed);
        install_arrows_tag(&mut world, Vec::new());
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
        );
    }

    #[test]
    fn uncharged_crossbow_uses_only_held_fireworks_or_inventory_arrows() {
        let mut held_firework = WorldStore::new();
        set_slot(&mut held_firework, 0, stack(VANILLA_ITEM_CROSSBOW_ID));
        set_slot(
            &mut held_firework,
            VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            stack(VANILLA_ITEM_FIREWORK_ROCKET_ID),
        );
        assert_eq!(
            held_firework.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true)
            ))
        );

        let mut inventory_firework = WorldStore::new();
        set_slot(&mut inventory_firework, 0, stack(VANILLA_ITEM_CROSSBOW_ID));
        set_slot(
            &mut inventory_firework,
            9,
            stack(VANILLA_ITEM_FIREWORK_ROCKET_ID),
        );
        install_arrows_tag(&mut inventory_firework, Vec::new());
        assert_eq!(
            inventory_firework.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
        );

        let mut inventory_arrow = WorldStore::new();
        set_slot(&mut inventory_arrow, 0, stack(VANILLA_ITEM_CROSSBOW_ID));
        set_slot(&mut inventory_arrow, 9, stack(VANILLA_ITEM_ARROW_ID));
        install_arrows_tag(&mut inventory_arrow, vec![VANILLA_ITEM_ARROW_ID]);
        assert_eq!(
            inventory_arrow.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true)
            ))
        );

        let mut creative = WorldStore::new();
        set_slot(&mut creative, 0, stack(VANILLA_ITEM_CROSSBOW_ID));
        creative.apply_player_abilities(PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        assert_eq!(
            creative.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true)
            ))
        );
    }

    #[test]
    fn crossbow_missing_arrow_tag_is_explicit_and_client_tick_does_not_load() {
        let mut world = WorldStore::new();
        set_slot(&mut world, 0, stack(VANILLA_ITEM_CROSSBOW_ID));
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::MissingRegistryTag(VANILLA_ARROWS_ITEM_TAG)
        );

        set_slot(&mut world, 9, stack(VANILLA_ITEM_STONE_ID));
        install_arrows_tag(&mut world, vec![VANILLA_ITEM_ARROW_ID]);
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
        );
        set_slot(&mut world, 9, stack(VANILLA_ITEM_ARROW_ID));
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true)
            ))
        );
        world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        assert_eq!(
            world.advance_local_using_item_ticks_with_actions(30),
            LocalUsingItemTickActions::default()
        );
        assert_eq!(world.local_player().interaction.using_item_ticks, 30);
        assert!(world.local_player().interaction.using_item);
        assert!(!local_crossbow_is_charged(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .expect("held crossbow")
        ));
        assert!(world.release_local_using_item());
        assert!(world
            .take_pending_local_interaction_sound_actions()
            .is_empty());
    }
}
