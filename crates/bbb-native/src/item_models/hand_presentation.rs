use std::sync::Arc;

use bbb_item_model::NativeItemRuntime;
use bbb_protocol::packets::{InteractionHand, ItemStackSummary};
use bbb_world::WorldStore;

/// Simulation-owned equivalent of Vanilla 26.1 `ItemInHandRenderer`'s two
/// visible stacks and previous/current hand heights. Publications clone only
/// the two optional `Arc`s and four floats; canonical inventory stays in world.
#[derive(Debug, Clone, Default)]
pub(crate) struct FirstPersonHandPresentation {
    main: HandPresentationSlot,
    off: HandPresentationSlot,
}

#[derive(Debug, Clone, Default)]
struct HandPresentationSlot {
    visible_stack: Option<Arc<ItemStackSummary>>,
    previous_height: f32,
    current_height: f32,
}

impl FirstPersonHandPresentation {
    #[cfg(test)]
    pub(crate) fn fully_raised_for_test(world: &WorldStore) -> Self {
        let raised = |hand| HandPresentationSlot {
            visible_stack: world.local_item_in_hand(hand).cloned().map(Arc::new),
            previous_height: 1.0,
            current_height: 1.0,
        };
        Self {
            main: raised(InteractionHand::MainHand),
            off: raised(InteractionHand::OffHand),
        }
    }

    /// Advances the renderer-input owner once per raw client tick, matching
    /// `ItemInHandRenderer.tick`. Catch-up ticks observe the latest canonical
    /// inventory snapshot while retaining the same bounded two-stack state.
    pub(crate) fn advance_for_ticks(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        hands_busy: bool,
        ticks: u32,
    ) {
        if world.local_player_id().is_none() {
            return;
        }
        for tick_index in 0..ticks {
            let ticks_ago = ticks.saturating_sub(tick_index.saturating_add(1));
            let main_target = world
                .local_player_item_swap_scale_ticks_ago(ticks_ago, 1.0)
                .powi(3);
            self.main.advance(
                world.local_item_in_hand(InteractionHand::MainHand),
                item_runtime,
                hands_busy,
                main_target,
            );
            self.off.advance(
                world.local_item_in_hand(InteractionHand::OffHand),
                item_runtime,
                hands_busy,
                1.0,
            );
        }
    }

    /// Vanilla `ItemInHandRenderer.itemUsed`: immediately lower the selected
    /// current hand; the next renderer tick records it as the interpolation
    /// origin and starts raising it again.
    pub(crate) fn item_used(&mut self, hand: InteractionHand) {
        self.slot_mut(hand).current_height = 0.0;
    }

    pub(crate) fn visible_stack(&self, hand: InteractionHand) -> Option<&ItemStackSummary> {
        self.slot(hand).visible_stack.as_deref()
    }

    /// `ItemInHandRenderer.renderArmWithItem` receives
    /// `swapAnimationScale(visible) * (1 - lerp(oldHeight, height))`.
    pub(crate) fn interpolated_inverse_height(
        &self,
        hand: InteractionHand,
        item_runtime: Option<&NativeItemRuntime>,
        partial_tick: f32,
    ) -> f32 {
        let slot = self.slot(hand);
        let height =
            slot.previous_height + (slot.current_height - slot.previous_height) * partial_tick;
        let scale = slot.visible_stack.as_deref().map_or(1.0, |stack| {
            item_runtime.map_or(1.0, |runtime| runtime.swap_animation_scale_for_stack(stack))
        });
        scale * (1.0 - height)
    }

    fn slot(&self, hand: InteractionHand) -> &HandPresentationSlot {
        match hand {
            InteractionHand::MainHand => &self.main,
            InteractionHand::OffHand => &self.off,
        }
    }

    fn slot_mut(&mut self, hand: InteractionHand) -> &mut HandPresentationSlot {
        match hand {
            InteractionHand::MainHand => &mut self.main,
            InteractionHand::OffHand => &mut self.off,
        }
    }
}

impl HandPresentationSlot {
    fn advance(
        &mut self,
        expected: Option<&ItemStackSummary>,
        item_runtime: Option<&NativeItemRuntime>,
        hands_busy: bool,
        visible_target_height: f32,
    ) {
        self.previous_height = self.current_height;
        if should_instantly_replace(self.visible_stack.as_deref(), expected, item_runtime) {
            self.replace_visible_if_changed(expected);
        }

        if hands_busy {
            self.current_height = (self.current_height - 0.4).clamp(0.0, 1.0);
        } else {
            let target_height = if self.visible_stack.as_deref() == expected {
                visible_target_height
            } else {
                0.0
            };
            self.current_height += (target_height - self.current_height).clamp(-0.4, 0.4);
        }

        if self.current_height < 0.1 {
            self.replace_visible_if_changed(expected);
        }
    }

    fn replace_visible_if_changed(&mut self, expected: Option<&ItemStackSummary>) {
        if self.visible_stack.as_deref() == expected {
            return;
        }
        self.visible_stack = expected.cloned().map(Arc::new);
    }
}

fn should_instantly_replace(
    visible: Option<&ItemStackSummary>,
    expected: Option<&ItemStackSummary>,
    item_runtime: Option<&NativeItemRuntime>,
) -> bool {
    match (visible, expected) {
        (None, None) => true,
        (Some(visible), Some(expected)) => {
            item_runtime.map_or(visible == expected, |runtime| {
                runtime.stacks_match_ignoring_swap_animation(visible, expected)
            }) || !item_runtime.map_or(true, |runtime| {
                runtime.hand_animation_on_swap_for_stack(expected)
            })
        }
        (_, Some(expected)) => !item_runtime.map_or(true, |runtime| {
            runtime.hand_animation_on_swap_for_stack(expected)
        }),
        (Some(_), None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID;

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    #[test]
    fn first_person_hand_presentation_lowers_replaces_and_raises_visible_stack() {
        let runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
        let apple_id = runtime.item_protocol_id("minecraft:apple").unwrap();
        let apple = stack(apple_id, 1);
        let mut slot = HandPresentationSlot::default();

        slot.advance(Some(&apple), Some(&runtime), false, 1.0);
        assert_eq!(slot.visible_stack.as_deref(), Some(&apple));
        assert_eq!(slot.current_height, 0.0);
        slot.advance(Some(&apple), Some(&runtime), false, 1.0);
        assert_eq!(slot.current_height, 0.4);
        slot.advance(Some(&apple), Some(&runtime), false, 1.0);
        slot.advance(Some(&apple), Some(&runtime), false, 1.0);
        assert_eq!(slot.current_height, 1.0);

        let two_apples = stack(apple_id, 2);
        slot.advance(Some(&two_apples), Some(&runtime), false, 1.0);
        assert_eq!(slot.visible_stack.as_deref(), Some(&apple));
        assert_eq!(slot.current_height, 0.6);
        slot.advance(Some(&two_apples), Some(&runtime), false, 1.0);
        slot.advance(Some(&two_apples), Some(&runtime), false, 1.0);
        assert_eq!(slot.visible_stack.as_deref(), Some(&two_apples));
        assert_eq!(slot.current_height, 0.0);
    }

    #[test]
    fn first_person_hand_presentation_ignores_damage_change_and_interpolates_item_used() {
        let runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
        let sword_id = runtime.item_protocol_id("minecraft:diamond_sword").unwrap();
        let damaged = |damage| ItemStackSummary {
            item_id: Some(sword_id),
            count: 1,
            component_patch: bbb_protocol::packets::DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
                damage: Some(damage),
                ..Default::default()
            },
        };
        let original = damaged(1);
        let updated = damaged(7);
        let mut presentation = FirstPersonHandPresentation::default();
        presentation.main.visible_stack = Some(Arc::new(original));
        presentation.main.previous_height = 1.0;
        presentation.main.current_height = 1.0;

        presentation
            .main
            .advance(Some(&updated), Some(&runtime), false, 1.0);
        assert_eq!(presentation.main.visible_stack.as_deref(), Some(&updated));
        assert_eq!(presentation.main.current_height, 1.0);

        presentation.item_used(InteractionHand::MainHand);
        assert_eq!(
            presentation.interpolated_inverse_height(
                InteractionHand::MainHand,
                Some(&runtime),
                0.25,
            ),
            0.25
        );
        presentation
            .main
            .advance(Some(&updated), Some(&runtime), false, 1.0);
        assert_eq!(presentation.main.previous_height, 0.0);
        assert_eq!(presentation.main.current_height, 0.4);
    }

    #[test]
    fn first_person_hand_presentation_hands_busy_lowers_both_hands() {
        let mut presentation = FirstPersonHandPresentation::default();
        presentation.main.current_height = 1.0;
        presentation.off.current_height = 0.3;

        presentation.main.advance(None, None, true, 1.0);
        presentation.off.advance(None, None, true, 1.0);

        assert_eq!(presentation.main.current_height, 0.6);
        assert_eq!(presentation.off.current_height, 0.0);
    }

    #[test]
    fn first_person_hand_presentation_applies_client_item_swap_properties() {
        let runtime = NativeItemRuntime::empty_for_test()
            .with_vanilla_item_registry_for_test()
            .with_client_item_swap_properties_for_test("minecraft:apple", false, 0.5);
        let apple_id = runtime.item_protocol_id("minecraft:apple").unwrap();
        let one_apple = stack(apple_id, 1);
        let two_apples = stack(apple_id, 2);
        let mut presentation = FirstPersonHandPresentation::default();
        presentation.main.visible_stack = Some(Arc::new(one_apple));
        presentation.main.previous_height = 1.0;
        presentation.main.current_height = 1.0;

        presentation
            .main
            .advance(Some(&two_apples), Some(&runtime), false, 1.0);
        assert_eq!(
            presentation.main.visible_stack.as_deref(),
            Some(&two_apples),
            "hand_animation_on_swap=false replaces without lowering"
        );
        assert_eq!(presentation.main.current_height, 1.0);

        presentation.item_used(InteractionHand::MainHand);
        assert_eq!(
            presentation.interpolated_inverse_height(
                InteractionHand::MainHand,
                Some(&runtime),
                1.0,
            ),
            0.5,
            "swap_animation_scale scales the interpolated inverse height"
        );
    }
}
