use super::*;
use bbb_protocol::item_types::{
    VANILLA_ITEM_BOW_ID, VANILLA_ITEM_SPYGLASS_ID, VANILLA_ITEM_TRIDENT_ID,
};

const VANILLA_ARROWS_ITEM_TAG: &str = "minecraft:arrows";
const VANILLA_BOW_MAX_DRAW_DURATION_TICKS: f32 = 20.0;
const VANILLA_BOW_MINIMUM_RELEASE_POWER: f32 = 0.1;

impl WorldStore {
    pub(super) fn predict_local_bow_item_use(&self) -> LocalItemUsePrediction {
        if !self
            .local_player()
            .abilities
            .is_some_and(|abilities| abilities.instabuild)
        {
            let Some(has_arrow) = self.local_player_has_item_in_tag(VANILLA_ARROWS_ITEM_TAG) else {
                return LocalItemUsePrediction::MissingRegistryTag(VANILLA_ARROWS_ITEM_TAG);
            };
            if !has_arrow {
                return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
            }
        }

        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(true),
        ))
    }

    pub fn release_local_using_item(&mut self) -> bool {
        if !self.local_player().interaction.using_item {
            return false;
        }
        let action = if self.local_using_item_item_id() == Some(VANILLA_ITEM_TRIDENT_ID) {
            self.release_local_trident_using_item()
        } else {
            self.local_using_item_release_sound_action()
        };
        if let Some(action) = action {
            self.queue_local_interaction_sound_action(action);
        }
        self.take_local_using_item()
    }

    pub(crate) fn finish_local_using_item(&mut self) -> bool {
        if !self.local_player().interaction.using_item {
            return false;
        }
        if self.local_using_item_item_id() == Some(VANILLA_ITEM_SPYGLASS_ID) {
            self.queue_local_interaction_sound_action(
                LocalInteractionSoundAction::SpyglassStopUsing,
            );
        }
        self.take_local_using_item()
    }

    fn local_using_item_release_sound_action(&self) -> Option<LocalInteractionSoundAction> {
        match self.local_using_item_item_id()? {
            VANILLA_ITEM_SPYGLASS_ID => Some(LocalInteractionSoundAction::SpyglassStopUsing),
            VANILLA_ITEM_BOW_ID => {
                let has_arrow = self
                    .local_player()
                    .abilities
                    .is_some_and(|abilities| abilities.instabuild)
                    || self.local_player_has_item_in_tag(VANILLA_ARROWS_ITEM_TAG) == Some(true);
                if !has_arrow {
                    return None;
                }
                let power =
                    bow_power_for_time(self.local_player().interaction.using_item_ticks as f32);
                (power >= VANILLA_BOW_MINIMUM_RELEASE_POWER).then_some(
                    LocalInteractionSoundAction::SuppressedBowRelease {
                        power_bits: power.to_bits(),
                    },
                )
            }
            _ => None,
        }
    }

    /// Vanilla `Player.isScoping`: the local player is actively using a spyglass.
    pub fn local_player_is_scoping(&self) -> bool {
        self.local_player().interaction.using_item
            && self.local_using_item_item_id() == Some(VANILLA_ITEM_SPYGLASS_ID)
    }

    pub fn local_player_camera_fov_modifier(&self, first_person: bool, effect_scale: f32) -> f32 {
        let mut modifier = self.local_player_abilities_fov_modifier();
        if self.local_player().interaction.using_item {
            if first_person && self.local_player_is_scoping() {
                return 0.1;
            }
            match self.local_using_item_item_id() {
                Some(VANILLA_ITEM_BOW_ID) => {
                    let scale = (self.local_player().interaction.using_item_ticks as f32
                        / VANILLA_BOW_MAX_DRAW_DURATION_TICKS)
                        .min(1.0);
                    modifier *= 1.0 - scale * scale * 0.15;
                }
                _ => {}
            }
        }
        let effect_scale = if effect_scale.is_finite() {
            effect_scale.clamp(0.0, 1.0)
        } else {
            1.0
        };
        1.0 + effect_scale * (modifier - 1.0)
    }
}

fn bow_power_for_time(time_held: f32) -> f32 {
    let power = time_held / VANILLA_BOW_MAX_DRAW_DURATION_TICKS;
    ((power * power + power * 2.0) / 3.0).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        item_types::VANILLA_ITEM_ARROW_ID,
        packets::{
            ItemStackSummary, PlayerAbilities, RegistryTags, SetPlayerInventory, TagNetworkPayload,
            UpdateTags,
        },
    };

    fn stack(item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        }
    }

    fn set_inventory_item(world: &mut WorldStore, slot: i32, item_id: i32) {
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot,
            item: stack(item_id),
        }));
    }

    fn install_arrow_tag(world: &mut WorldStore) {
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_ARROWS_ITEM_TAG.to_string(),
                    entries: vec![VANILLA_ITEM_ARROW_ID],
                }],
            }],
        });
    }

    #[test]
    fn bow_use_requires_live_arrow_tag_membership_or_infinite_materials() {
        let mut missing_tag = WorldStore::new();
        set_inventory_item(&mut missing_tag, 0, VANILLA_ITEM_BOW_ID);
        assert_eq!(
            missing_tag.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::MissingRegistryTag(VANILLA_ARROWS_ITEM_TAG)
        );

        let mut survival = WorldStore::new();
        install_arrow_tag(&mut survival);
        set_inventory_item(&mut survival, 0, VANILLA_ITEM_BOW_ID);
        assert_eq!(
            survival.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
        );
        set_inventory_item(&mut survival, 9, VANILLA_ITEM_ARROW_ID);
        assert_eq!(
            survival.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true)
            ))
        );

        let mut infinite = WorldStore::new();
        infinite.apply_player_abilities(PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        set_inventory_item(&mut infinite, 0, VANILLA_ITEM_BOW_ID);
        assert!(matches!(
            infinite.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
    }

    #[test]
    fn bow_release_uses_three_tick_threshold_without_client_inventory_mutation() {
        let mut world = WorldStore::new();
        install_arrow_tag(&mut world);
        set_inventory_item(&mut world, 0, VANILLA_ITEM_BOW_ID);
        set_inventory_item(&mut world, 9, VANILLA_ITEM_ARROW_ID);

        world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        world.advance_local_using_item_ticks(2);
        assert!(world.release_local_using_item());
        assert!(world
            .take_pending_local_interaction_sound_actions()
            .is_empty());

        world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        world.advance_local_using_item_ticks(3);
        assert!(world.release_local_using_item());
        assert_eq!(
            world.take_pending_local_interaction_sound_actions(),
            vec![LocalInteractionSoundAction::SuppressedBowRelease {
                power_bits: bow_power_for_time(3.0).to_bits(),
            }]
        );
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&stack(VANILLA_ITEM_BOW_ID))
        );
        assert_eq!(
            world
                .inventory()
                .player_slots
                .iter()
                .find(|slot| slot.slot == 9)
                .map(|slot| &slot.item),
            Some(&stack(VANILLA_ITEM_ARROW_ID))
        );
    }

    #[test]
    fn local_player_is_scoping_drives_spyglass_fov_release_and_natural_finish() {
        let mut world = WorldStore::new();
        set_inventory_item(&mut world, 0, VANILLA_ITEM_SPYGLASS_ID);
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::spyglass_use()
            ))
        );

        world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        assert!(world.local_player_is_scoping());
        assert!((world.local_player_camera_fov_modifier(true, 1.0) - 0.1).abs() < 1.0e-6);
        assert!((world.local_player_camera_fov_modifier(false, 1.0) - 1.0).abs() < 1.0e-6);
        assert!(world.release_local_using_item());
        assert!(!world.local_player_is_scoping());
        assert_eq!(
            world.take_pending_local_interaction_sound_actions(),
            vec![LocalInteractionSoundAction::SpyglassStopUsing]
        );

        set_inventory_item(&mut world, 0, VANILLA_ITEM_BOW_ID);
        world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        assert!(!world.local_player_is_scoping());
        world.advance_local_using_item_ticks(10);
        assert!((world.local_player_camera_fov_modifier(true, 1.0) - 0.9625).abs() < 1.0e-6);
        assert!((world.local_player_camera_fov_modifier(true, 0.5) - 0.98125).abs() < 1.0e-6);
        world.take_local_using_item();

        set_inventory_item(&mut world, 0, VANILLA_ITEM_SPYGLASS_ID);
        world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        world.advance_local_using_item_ticks(1_200);
        assert!(world.local_player().interaction.using_item);
        world.finish_local_using_item();
        assert!(!world.local_player().interaction.using_item);
        assert_eq!(
            world.take_pending_local_interaction_sound_actions(),
            vec![LocalInteractionSoundAction::SpyglassStopUsing]
        );
    }
}
