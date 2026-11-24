use super::*;

impl WorldStore {
    pub(super) fn predict_local_thrown_projectile_item_use(
        &mut self,
        hand: InteractionHand,
    ) -> LocalItemUsePrediction {
        let stack = self.local_item_in_hand(hand).unwrap();
        let cooldown = if self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID)
        {
            let effective = self.effective_item_stack(stack);
            let Some(duration) = effective.component_patch.use_cooldown_ticks() else {
                return LocalItemUsePrediction::MissingComponentValue(
                    VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
                );
            };
            if duration <= 0 {
                return LocalItemUsePrediction::MissingComponentValue(
                    VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
                );
            }
            let Some(group) = self.local_item_stack_cooldown_group(stack) else {
                return LocalItemUsePrediction::UnknownItemType(stack.item_id);
            };
            Some((group, duration))
        } else {
            None
        };

        let item_id = stack.item_id;
        if !self.consume_local_item_in_hand(hand, 1) {
            return LocalItemUsePrediction::UnknownItemType(item_id);
        }
        if let Some((group, duration)) = cooldown {
            self.add_local_item_cooldown(group, duration);
        }

        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        item_types::{
            VANILLA_ITEM_EGG_ID, VANILLA_ITEM_ENDER_PEARL_ID, VANILLA_ITEM_EXPERIENCE_BOTTLE_ID,
            VANILLA_ITEM_LINGERING_POTION_ID, VANILLA_ITEM_SNOWBALL_ID,
            VANILLA_ITEM_SPLASH_POTION_ID, VANILLA_ITEM_WIND_CHARGE_ID,
        },
        packets::{DataComponentPatchSummary, PlayerAbilities, SetPlayerInventory},
    };
    use std::collections::BTreeMap;

    const PROJECTILE_ITEMS: [i32; 7] = [
        VANILLA_ITEM_SNOWBALL_ID,
        VANILLA_ITEM_EGG_ID,
        VANILLA_ITEM_ENDER_PEARL_ID,
        VANILLA_ITEM_EXPERIENCE_BOTTLE_ID,
        VANILLA_ITEM_SPLASH_POTION_ID,
        VANILLA_ITEM_LINGERING_POTION_ID,
        VANILLA_ITEM_WIND_CHARGE_ID,
    ];

    fn stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary::default().into(),
        }
    }

    fn install_default_cooldowns(world: &mut WorldStore) {
        world.set_default_item_component_codec_values(
            [
                (VANILLA_ITEM_ENDER_PEARL_ID, 1.0),
                (VANILLA_ITEM_WIND_CHARGE_ID, 0.5),
            ]
            .into_iter()
            .map(|(item_id, seconds)| {
                (
                    item_id,
                    BTreeMap::from([(
                        VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
                        serde_json::json!({"seconds": seconds}),
                    )]),
                )
            })
            .collect(),
        );
        world.rebuild_item_component_defaults().unwrap();
    }

    fn set_main_hand(world: &mut WorldStore, item: ProtocolItemStackSummary) {
        assert!(world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item }));
    }

    #[test]
    fn projectile_use_family_consumes_once_and_applies_only_effective_cooldowns() {
        for item_id in PROJECTILE_ITEMS {
            let mut world = WorldStore::new();
            install_default_cooldowns(&mut world);
            set_main_hand(&mut world, stack(item_id, 2));

            assert_eq!(
                world.predict_local_item_use(InteractionHand::MainHand),
                LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_item_interaction()
                )),
                "item id {item_id}"
            );
            assert_eq!(
                world
                    .local_item_in_hand(InteractionHand::MainHand)
                    .unwrap()
                    .count,
                1,
                "item id {item_id}"
            );
            let (cooldown_group, duration) = match item_id {
                VANILLA_ITEM_ENDER_PEARL_ID => (Some("minecraft:ender_pearl"), Some(20)),
                VANILLA_ITEM_WIND_CHARGE_ID => (Some("minecraft:wind_charge"), Some(10)),
                _ => (None, None),
            };
            assert_eq!(
                cooldown_group
                    .and_then(|group| world.cooldown(group))
                    .map(|cooldown| cooldown.duration),
                duration,
                "item id {item_id}"
            );
            if cooldown_group.is_none() {
                assert!(world.cooldowns().is_empty(), "item id {item_id}");
            }
        }
    }

    #[test]
    fn projectile_use_honors_infinite_materials_and_explicit_cooldown_group() {
        let mut world = WorldStore::new();
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let mut snowball = stack(VANILLA_ITEM_SNOWBALL_ID, 2);
        snowball.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID];
        snowball.component_patch.use_cooldown_seconds_bits = Some(0.35_f32.to_bits());
        snowball.component_patch.use_cooldown_group = Some("example:shared_throw".to_string());
        set_main_hand(&mut world, snowball);

        assert!(matches!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            2
        );
        assert_eq!(
            world
                .cooldown("example:shared_throw")
                .map(|cooldown| cooldown.duration),
            Some(7)
        );
    }

    #[test]
    fn projectile_use_respects_cooldown_removal_active_gate_and_missing_value() {
        let mut removed = WorldStore::new();
        install_default_cooldowns(&mut removed);
        let mut pearl = stack(VANILLA_ITEM_ENDER_PEARL_ID, 2);
        pearl.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID];
        set_main_hand(&mut removed, pearl);
        assert!(matches!(
            removed.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        assert!(removed.cooldowns().is_empty());

        let mut active = WorldStore::new();
        install_default_cooldowns(&mut active);
        set_main_hand(&mut active, stack(VANILLA_ITEM_ENDER_PEARL_ID, 2));
        active.apply_cooldown(bbb_protocol::packets::Cooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 5,
        });
        assert_eq!(
            active.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert_eq!(
            active
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            2
        );

        let mut malformed = WorldStore::new();
        let mut snowball = stack(VANILLA_ITEM_SNOWBALL_ID, 2);
        snowball.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID];
        set_main_hand(&mut malformed, snowball);
        assert_eq!(
            malformed.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::MissingComponentValue(
                VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID
            )
        );
        assert_eq!(
            malformed
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            2
        );
    }
}
