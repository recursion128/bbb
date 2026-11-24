use super::*;

const VANILLA_BUNDLE_USE_DURATION_TICKS: u32 = 200;
const VANILLA_BUNDLE_REPEAT_DELAY_TICKS: u32 = 10;

impl WorldStore {
    /// Advances the local `LivingEntity.updateUsingItem` clock and applies the
    /// logical-client work owned by item `onUseTick` implementations.
    ///
    /// Client-side `Player.drop` creates no local item entity. Each bundle
    /// drop requests a remove-one sound and a main-hand swing.
    pub fn advance_local_using_item_ticks_with_actions(
        &mut self,
        ticks: u32,
    ) -> LocalUsingItemTickActions {
        let mut actions = LocalUsingItemTickActions::default();
        if ticks == 0 {
            return actions;
        }
        let using_item_type = self.local_using_item_item_id().and_then(vanilla_item_type);
        let using_bundle = using_item_type.and_then(|item_type| item_type.use_override)
            == Some(VanillaItemUseOverride::BundleItem);
        let using_brush = using_item_type.and_then(|item_type| item_type.use_on_override)
            == Some(VanillaItemUseOnOverride::BrushItem);
        let hand = self.local_player().interaction.using_item_hand;
        let consumable = hand.and_then(|hand| {
            self.effective_item_consumable(self.local_item_in_hand(hand)?)
                .copied()
                .map(|component| (hand, component))
        });
        if !using_bundle && !using_brush && consumable.is_none() {
            self.advance_local_using_item_ticks(ticks);
            return actions;
        }

        for _ in 0..ticks {
            let using_ticks = self.local_player().interaction.using_item_ticks;
            if let Some((hand, component)) = consumable {
                let remaining = component.consume_ticks() - using_ticks.min(i32::MAX as u32) as i32;
                if component.emits_feedback(remaining) {
                    self.queue_local_interaction_sound_action(
                        LocalInteractionSoundAction::Consume { hand },
                    );
                    self.queue_local_interaction_particle_action(
                        LocalInteractionParticleAction::Consume { hand, count: 5 },
                    );
                }
            }
            if using_brush && (using_ticks + 1) % 10 == 5 {
                // Vanilla `BrushItem.onUseTick` (`BrushItem.java:62-79`): emits block brush
                // sound (excluded for acting player by server) and 7..12 dust particles.
                if let Some((pos, block_state_id, hit_loc, face, view)) =
                    self.clip_local_player_brush_block_hit()
                {
                    let block_name = self.block_view(pos).and_then(|block| block.block_name);
                    let sound_resource_id = match block_name {
                        Some("minecraft:suspicious_sand") => "minecraft:item.brush.brushing.sand",
                        Some("minecraft:suspicious_gravel") => {
                            "minecraft:item.brush.brushing.gravel"
                        }
                        _ => "minecraft:item.brush.brushing.generic",
                    };
                    self.queue_local_interaction_sound_action(
                        LocalInteractionSoundAction::BlockPositioned {
                            pos,
                            sound_resource_id,
                            volume_bits: 1.0_f32.to_bits(),
                            pitch_bits: 1.0_f32.to_bits(),
                        },
                    );
                    let using_main_hand =
                        hand.unwrap_or(InteractionHand::MainHand) == InteractionHand::MainHand;
                    let main_right = !self.local_player_main_arm_left().unwrap_or(false);
                    let flip = if using_main_hand == main_right {
                        1.0
                    } else {
                        -1.0
                    };
                    let (dx, dz) = match face {
                        ProtocolDirection::Down | ProtocolDirection::Up => (view[2], -view[0]),
                        ProtocolDirection::North => (1.0, -0.1),
                        ProtocolDirection::South => (-1.0, 0.1),
                        ProtocolDirection::West => (-0.1, -1.0),
                        ProtocolDirection::East => (0.1, 1.0),
                    };
                    let px = hit_loc[0]
                        - if face == ProtocolDirection::West {
                            1.0e-6
                        } else {
                            0.0
                        };
                    let py = hit_loc[1];
                    let pz = hit_loc[2]
                        - if face == ProtocolDirection::North {
                            1.0e-6
                        } else {
                            0.0
                        };
                    let count = 7
                        + (self
                            .client_audio
                            .sound_seed_random
                            .next_long()
                            .unsigned_abs()
                            % 5) as usize;
                    for _ in 0..count {
                        let r1 = ((self
                            .client_audio
                            .sound_seed_random
                            .next_long()
                            .unsigned_abs()
                            >> 11) as f64)
                            * (1.0 / ((1u64 << 53) as f64));
                        let r2 = ((self
                            .client_audio
                            .sound_seed_random
                            .next_long()
                            .unsigned_abs()
                            >> 11) as f64)
                            * (1.0 / ((1u64 << 53) as f64));
                        self.push_pending_client_particle_spawn(
                            crate::particles::ParticleSpawn::new(
                                crate::particles::ParticleKind::BLOCK,
                                [px, py, pz],
                                [dx * flip * 3.0 * r1, 0.0, dz * flip * 3.0 * r2],
                            )
                            .with_sprite(crate::particles::SpawnSprite::BlockState(block_state_id)),
                        );
                    }
                }
            }
            if using_bundle && bundle_drops_content_at_using_tick(using_ticks) {
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
            component_patch: DataComponentPatchSummary::default().into(),
        }
    }

    fn bundle_stack(bundle_item_id: i32, item_ids: &[i32]) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(bundle_item_id),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
                bundle_contents_items: item_ids.iter().copied().map(template).collect(),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
            world.inventory().hotbar_item_states()[0]
                .unwrap()
                .local_selected_bundle_item_index(),
            None
        );
        let bundle = world
            .local_item_in_hand(InteractionHand::MainHand)
            .expect("held bundle");
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
        assert!(!empty_bundle
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID));
        assert!(world
            .take_pending_local_interaction_sound_actions()
            .is_empty());
    }

    #[test]
    fn empty_and_removed_bundle_contents_do_not_emit_tick_actions() {
        let mut removed = bundle_stack(VANILLA_ITEM_BUNDLE_ID, &[VANILLA_ITEM_STONE_ID]);
        removed
            .component_patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);

        for stack in [bundle_stack(VANILLA_ITEM_BUNDLE_ID, &[]), removed] {
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
