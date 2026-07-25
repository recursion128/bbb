use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
};
use bbb_protocol::menu_types::{
    VANILLA_MENU_TYPE_ENCHANTMENT_ID, VANILLA_MENU_TYPE_LECTERN_ID, VANILLA_MENU_TYPE_LOOM_ID,
    VANILLA_MENU_TYPE_STONECUTTER_ID,
};
use bbb_protocol::packets::{
    BannerPatternLayerSummary as ProtocolBannerPatternLayerSummary,
    DataComponentPatchSummary as ProtocolDataComponentPatchSummary,
    ItemStackSummary as ProtocolItemStackSummary,
};

use super::*;

const LOOM_MAX_BANNER_PATTERNS: usize = 6;
const LOOM_SELECTED_PATTERN_DATA_ID: i16 = 0;
const STONECUTTER_SELECTED_RECIPE_DATA_ID: i16 = 0;
const ITEM_REGISTRY: &str = "minecraft:item";

impl WorldStore {
    /// Applies the client-side half of vanilla menu `clickMenuButton` before
    /// `ServerboundContainerButtonClickPacket` is sent.
    ///
    /// Lectern buttons are deliberately packet-only: `LecternScreen` calls
    /// `MultiPlayerGameMode.handleInventoryButtonClick` without invoking the
    /// local menu. Enchantment's client menu has `ContainerLevelAccess.NULL`,
    /// so only its eligibility guard runs locally. Loom and stonecutter mutate
    /// their client menu state and enqueue their immediate UI selection sound.
    pub fn apply_local_container_button_click(&mut self, button_id: i32) -> Option<i32> {
        let (container_id, menu_type_id) = self
            .inventory
            .open_container
            .as_ref()
            .map(|container| (container.container_id, container.menu_type_id))?;

        match menu_type_id? {
            VANILLA_MENU_TYPE_LECTERN_ID => {
                // `LecternScreen.sendButtonClick` sends directly. The local
                // page/data and book slot change only when the server replies.
            }
            VANILLA_MENU_TYPE_ENCHANTMENT_ID => {
                if !self.local_enchantment_button_is_eligible(button_id) {
                    return None;
                }
            }
            VANILLA_MENU_TYPE_LOOM_ID => {
                let result = self.local_loom_button_result(button_id)?;
                let container = self
                    .inventory
                    .open_container
                    .as_mut()
                    .filter(|container| container.container_id == container_id)?;
                set_container_data_value(
                    &mut container.data_values,
                    LOOM_SELECTED_PATTERN_DATA_ID,
                    i16::try_from(button_id).ok()?,
                );
                set_container_slot(
                    &mut container.slots,
                    ContainerSlot {
                        slot: LOOM_RESULT_SLOT,
                        item: result,
                        local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
                    },
                );
                self.queue_local_interaction_sound_action(
                    LocalInteractionSoundAction::LoomSelectPattern,
                );
            }
            VANILLA_MENU_TYPE_STONECUTTER_ID => {
                let selected = self
                    .open_container_data_value(STONECUTTER_SELECTED_RECIPE_DATA_ID)
                    .map(i32::from)
                    .unwrap_or(-1);
                if selected == button_id {
                    return None;
                }
                if usize::try_from(button_id).is_ok_and(|button| {
                    button < self.stonecutter_visible_recipe_count().unwrap_or(0)
                }) {
                    let container = self
                        .inventory
                        .open_container
                        .as_mut()
                        .filter(|container| container.container_id == container_id)?;
                    set_container_data_value(
                        &mut container.data_values,
                        STONECUTTER_SELECTED_RECIPE_DATA_ID,
                        i16::try_from(button_id).ok()?,
                    );
                    set_container_slot(
                        &mut container.slots,
                        ContainerSlot {
                            slot: STONECUTTER_RESULT_SLOT,
                            item: ProtocolItemStackSummary::empty(),
                            local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
                        },
                    );
                }
                // `StonecutterMenu.clickMenuButton` returns true for every
                // non-selected id, including an empty cell in the last row.
                self.queue_local_interaction_sound_action(
                    LocalInteractionSoundAction::StonecutterSelectRecipe,
                );
            }
            _ => return None,
        }

        Some(container_id)
    }

    /// Number of client stonecutter entries selected by the current input.
    /// Both direct holder lists and synchronized ingredient tags are resolved
    /// in current-session item-registry order.
    pub fn stonecutter_visible_recipe_count(&self) -> Option<usize> {
        let container = self.inventory.open_container.as_ref()?;
        if container.menu_type_id != Some(VANILLA_MENU_TYPE_STONECUTTER_ID) {
            return None;
        }
        let input_item_id = container
            .slots
            .iter()
            .find(|slot| slot.slot == STONECUTTER_INPUT_SLOT)
            .and_then(|slot| {
                (!item_stack_is_empty(&slot.item))
                    .then_some(slot.item.item_id)
                    .flatten()
            })?;
        let access = self.registries.frozen_access();
        Some(
            self.recipes
                .stonecutter_recipes
                .iter()
                .filter(|recipe| match recipe.input.tag.as_deref() {
                    Some(tag) => access
                        .holder_set_by_tag(ITEM_REGISTRY, tag)
                        .is_ok_and(|set| {
                            set.entries
                                .iter()
                                .any(|entry| entry.holder_id == input_item_id)
                        }),
                    None => recipe.input.item_ids.contains(&input_item_id),
                })
                .count(),
        )
    }

    fn local_enchantment_button_is_eligible(&self, button_id: i32) -> bool {
        let Ok(button) = usize::try_from(button_id) else {
            return false;
        };
        if button >= 3 {
            return false;
        }
        let Some(container) =
            self.inventory.open_container.as_ref().filter(|container| {
                container.menu_type_id == Some(VANILLA_MENU_TYPE_ENCHANTMENT_ID)
            })
        else {
            return false;
        };
        let slot_item = |slot_num| {
            container
                .slots
                .iter()
                .find(|slot| slot.slot == slot_num)
                .map(|slot| &slot.item)
        };
        let item_is_present =
            slot_item(ENCHANTMENT_INPUT_SLOT).is_some_and(item_stack_is_non_empty);
        if !item_is_present {
            return false;
        }

        let infinite_materials = self
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild);
        let material_cost = button_id + 1;
        let currency_count = slot_item(ENCHANTMENT_LAPIS_SLOT)
            .filter(|stack| item_stack_is_non_empty(stack))
            .map(|stack| stack.count)
            .unwrap_or_default();
        if currency_count < material_cost && !infinite_materials {
            return false;
        }

        let displayed_cost = container
            .data_values
            .iter()
            .find_map(|value| (value.id == button_id as i16).then_some(i32::from(value.value)))
            .unwrap_or_default();
        let experience_level = self
            .local_player
            .experience
            .map(|experience| experience.level)
            .unwrap_or_default();
        displayed_cost > 0
            && (infinite_materials
                || (experience_level >= material_cost && experience_level >= displayed_cost))
    }

    fn local_loom_button_result(&self, button_id: i32) -> Option<ProtocolItemStackSummary> {
        let button = usize::try_from(button_id).ok()?;
        let selectable_patterns = self.loom_selectable_banner_pattern_ids()?;
        let pattern_registry_id = *selectable_patterns.get(button)?;
        let container = self
            .inventory
            .open_container
            .as_ref()
            .filter(|container| container.menu_type_id == Some(VANILLA_MENU_TYPE_LOOM_ID))?;
        let slot_item = |slot_num| {
            container
                .slots
                .iter()
                .find(|slot| slot.slot == slot_num)
                .map(|slot| &slot.item)
        };
        let banner = slot_item(LOOM_BANNER_SLOT)?;
        let current_layers = effective_banner_pattern_layers(banner);
        // `LoomScreen.displayPatterns` is false at the six-layer limit, so the
        // click never reaches `LoomMenu.clickMenuButton`.
        if current_layers.len() >= LOOM_MAX_BANNER_PATTERNS {
            return None;
        }
        let dye = slot_item(LOOM_DYE_SLOT)?;
        let effective_dye = self.effective_item_stack(dye);
        let dye_patch = &effective_dye.component_patch;
        let dye_color_id = component_is_present(dye_patch, VANILLA_DATA_COMPONENT_DYE_TYPE_ID)
            .then_some(dye_patch.dye_color_id)
            .flatten();

        let mut result = ProtocolItemStackSummary::empty();
        if let Some(dye_color_id) = dye_color_id {
            result = banner.clone();
            result.count = 1;
            result.component_patch.banner_pattern_layers = current_layers;
            result
                .component_patch
                .banner_pattern_layers
                .push(ProtocolBannerPatternLayerSummary {
                    registry_id: Some(pattern_registry_id),
                    asset_id: None,
                    translation_key: None,
                    color_id: dye_color_id,
                });
            set_component(
                &mut result.component_patch,
                VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            );
        }
        Some(result)
    }
}

fn effective_banner_pattern_layers(
    stack: &ProtocolItemStackSummary,
) -> Vec<ProtocolBannerPatternLayerSummary> {
    component_is_present(
        &stack.component_patch,
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
    )
    .then(|| stack.component_patch.banner_pattern_layers.clone())
    .unwrap_or_default()
}

fn component_is_present(patch: &ProtocolDataComponentPatchSummary, component_type_id: i32) -> bool {
    !patch.removed_type_ids.contains(&component_type_id)
        && patch.added_type_ids.contains(&component_type_id)
}

fn set_component(patch: &mut ProtocolDataComponentPatchSummary, component_type_id: i32) {
    patch
        .removed_type_ids
        .retain(|removed| *removed != component_type_id);
    if !patch.added_type_ids.contains(&component_type_id) {
        patch.added_type_ids.push(component_type_id);
    }
    patch.added = patch.added_type_ids.len();
}
