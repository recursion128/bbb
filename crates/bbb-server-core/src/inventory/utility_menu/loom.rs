//! `LoomMenu`: one more banner pattern layer per take.

use std::borrow::Cow;

use crate::world::maps::ServerMapStore;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
};
use bbb_protocol::packets::{BannerPatternLayerSummary, DataComponentPatchSummary};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_datapack::recipe::ingredient::is_banner_item;
use bbb_server_types::item_stack::rules::has_component;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::{PlayerInventoryMenu, UtilityMenuEvent};

const BANNER: usize = 0;
const DYE: usize = 1;
const PATTERN: usize = 2;
const RESULT: usize = 3;
/// The `selectedBannerPatternIndex` data slot.
const SELECTED: usize = 0;
const BANNER_PATTERN_REGISTRY: &str = "minecraft:banner_pattern";
/// `BannerPatternLayers` past which the loom refuses (`LoomMenu.java:178`).
const MAX_PATTERNS: usize = 6;

impl PlayerInventoryMenu {
    /// The three input slots' `mayPlace` (`LoomMenu.java:64-81`).
    pub(super) fn loom_may_place(&self, slot: i16, item: &RuntimeItemStack) -> bool {
        match slot {
            0 => item.item_id().is_some_and(is_banner_item),
            1 => self.loom_dye_item(item),
            _ => self.loom_pattern_item(item),
        }
    }

    /// `LoomMenu.isDyeItem` (`LoomMenu.java:115-117`).
    fn loom_dye_item(&self, item: &RuntimeItemStack) -> bool {
        self.item_tag_contains("minecraft:loom_dyes", item)
            && has_component(item.summary(), VANILLA_DATA_COMPONENT_DYE_TYPE_ID).unwrap_or(false)
    }

    /// `LoomMenu.isPatternItem` (`LoomMenu.java:111-113`).
    fn loom_pattern_item(&self, item: &RuntimeItemStack) -> bool {
        self.item_tag_contains("minecraft:loom_patterns", item)
            && has_component(
                item.summary(),
                VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
            )
            .unwrap_or(false)
    }

    /// `LoomMenu.getSelectablePatterns` (`LoomMenu.java:135-142`) as banner
    /// pattern holder ids.
    fn loom_selectable_patterns(&self) -> Result<Vec<i32>> {
        let Some(facts) = self.facts() else {
            return Ok(Vec::new());
        };
        let tag_members = |tag: &str| -> Vec<i32> {
            facts
                .registry_tags
                .members(BANNER_PATTERN_REGISTRY, tag.trim_start_matches('#'))
                .iter()
                .filter_map(|id| {
                    PersistentItemStackHolderResolver::holder_id(
                        &facts.resolver,
                        BANNER_PATTERN_REGISTRY,
                        id,
                    )
                })
                .collect()
        };
        let pattern = &self.container[PATTERN];
        if pattern.is_empty() {
            return Ok(tag_members("minecraft:no_item_required"));
        }
        let Some(provides) = pattern.effective_component(
            VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
            &facts.resolver,
        )?
        else {
            return Ok(Vec::new());
        };
        if let Some(tag) = provides.provides_banner_pattern_tag.as_deref() {
            return Ok(tag_members(tag));
        }
        Ok(match provides {
            Cow::Owned(provides) => provides.provides_banner_pattern_registry_ids,
            Cow::Borrowed(provides) => provides.provides_banner_pattern_registry_ids.clone(),
        })
    }

    /// `LoomMenu.slotsChanged` (`LoomMenu.java:149-196`).
    pub(super) fn loom_slots_changed(&mut self) -> Result<()> {
        if self.container[BANNER].is_empty() || self.container[DYE].is_empty() {
            self.container[RESULT] = RuntimeItemStack::empty();
            self.utility_mut().loom_patterns.clear();
            self.utility_mut().data[SELECTED] = -1;
            return Ok(());
        }
        let selected = self.utility().data[SELECTED];
        let previous = std::mem::take(&mut self.utility_mut().loom_patterns);
        let valid = usize::try_from(selected).is_ok_and(|index| index < previous.len());
        let patterns = self.loom_selectable_patterns()?;
        let display = if patterns.len() == 1 {
            self.utility_mut().data[SELECTED] = 0;
            Some(patterns[0])
        } else if !valid {
            self.utility_mut().data[SELECTED] = -1;
            None
        } else {
            let value = previous[selected as usize];
            match patterns.iter().position(|pattern| *pattern == value) {
                Some(index) => {
                    self.utility_mut().data[SELECTED] = index as i32;
                    Some(value)
                }
                None => {
                    self.utility_mut().data[SELECTED] = -1;
                    None
                }
            }
        };
        self.utility_mut().loom_patterns = patterns;
        match display {
            Some(pattern) => {
                if self.loom_layers()?.len() >= MAX_PATTERNS {
                    self.utility_mut().data[SELECTED] = -1;
                    self.container[RESULT] = RuntimeItemStack::empty();
                } else {
                    self.loom_setup_result(pattern)?;
                }
            }
            None => self.container[RESULT] = RuntimeItemStack::empty(),
        }
        Ok(())
    }

    fn loom_layers(&self) -> Result<Vec<BannerPatternLayerSummary>> {
        let Some(facts) = self.facts() else {
            return Ok(Vec::new());
        };
        Ok(self.container[BANNER]
            .effective_component(
                VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
                &facts.resolver,
            )?
            .map(|patch| match patch {
                Cow::Owned(patch) => patch.banner_pattern_layers,
                Cow::Borrowed(patch) => patch.banner_pattern_layers.clone(),
            })
            .unwrap_or_default())
    }

    /// `LoomMenu.setupResultSlot` (`LoomMenu.java:269-288`).
    fn loom_setup_result(&mut self, pattern: i32) -> Result<()> {
        let Some(facts) = self.utility().facts.clone() else {
            return Ok(());
        };
        let banner = &self.container[BANNER];
        let dye = self.container[DYE]
            .effective_component(VANILLA_DATA_COMPONENT_DYE_TYPE_ID, &facts.resolver)?
            .and_then(|patch| patch.dye_color_id);
        let result = match dye {
            Some(color_id) if !banner.is_empty() => {
                let mut layers = self.loom_layers()?;
                layers.push(BannerPatternLayerSummary {
                    registry_id: Some(pattern),
                    asset_id: None,
                    translation_key: None,
                    color_id,
                });
                let patch = DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID],
                    banner_pattern_layers: layers,
                    ..Default::default()
                };
                banner
                    .with_count(1)
                    .with_component_patch(&patch, &facts.resolver)?
            }
            _ => RuntimeItemStack::empty(),
        };
        self.container[RESULT] = result;
        Ok(())
    }

    /// `LoomMenu.clickMenuButton` (`LoomMenu.java:125-133`).
    pub(super) fn loom_button(&mut self, button_id: i32) -> Result<bool> {
        let Some(&pattern) = usize::try_from(button_id)
            .ok()
            .and_then(|index| self.utility().loom_patterns.get(index))
        else {
            return Ok(false);
        };
        self.utility_mut().data[SELECTED] = button_id;
        self.loom_setup_result(pattern)?;
        Ok(true)
    }

    /// The result slot's `onTake` (`LoomMenu.java:89-104`).
    pub(super) fn loom_on_take(&mut self) -> Result<()> {
        for slot in [BANNER, DYE] {
            self.container[slot].shrink(1);
            self.loom_slots_changed()?;
        }
        if self.container[BANNER].is_empty() || self.container[DYE].is_empty() {
            self.utility_mut().data[SELECTED] = -1;
        }
        self.utility_mut().events.push(UtilityMenuEvent::LoomUsed);
        Ok(())
    }

    /// `LoomMenu.quickMoveStack` (`LoomMenu.java:211-261`).
    pub(super) fn loom_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
    ) -> Result<RuntimeItemStack> {
        let clicked = self.slot_item(slot).into_owned();
        let mut stack = clicked.clone();
        let moved = if slot == RESULT as i16 {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 4, 40, true)?
        } else if !(0..3).contains(&slot) {
            if stack.item_id().is_some_and(is_banner_item) {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 0, 1, false)?
            } else if self.loom_dye_item(&stack) {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 1, 2, false)?
            } else if self.loom_pattern_item(&stack) {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 2, 3, false)?
            } else if (4..31).contains(&slot) {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 31, 40, false)?
            } else if (31..40).contains(&slot) {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 4, 31, false)?
            } else {
                true
            }
        } else {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 4, 40, false)?
        };
        if !moved {
            return Ok(RuntimeItemStack::empty());
        }
        self.finish_utility_move(resolver, maps, slot, &clicked, stack)
    }
}
