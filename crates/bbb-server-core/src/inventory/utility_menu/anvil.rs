//! `AnvilMenu`: repair, enchantment merge and rename.

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_ENCHANTED_BOOK_ID;
use bbb_server_types::item_stack::rules::has_component;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::item_edits::{
    can_store_enchantments, crafting_enchantments, damage_value, hover_name, increased_repair_cost,
    is_damageable, is_valid_repair_item, max_damage, repair_cost, set_level,
    with_crafting_enchantments, with_custom_name, with_damage, with_repair_cost,
};
use super::{PlayerInventoryMenu, UtilityMenuEvent};
use crate::enchantment_facts::enchantments_compatible;

const INPUT: usize = 0;
const ADDITION: usize = 1;
const RESULT: usize = 2;
/// The `cost` data slot.
const COST: usize = 0;
/// `AnvilMenu.MAX_NAME_LENGTH` (`AnvilMenu.java:30`).
const MAX_NAME_LENGTH: usize = 50;
/// The price at which a survival player is refused (`AnvilMenu.java:250-252`).
const TOO_EXPENSIVE: i32 = 40;

#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct AnvilState {
    repair_item_count_cost: i32,
    item_name: Option<String>,
    only_renaming: bool,
}

impl PlayerInventoryMenu {
    /// `AnvilMenu.createResult` (`AnvilMenu.java:118-274`).
    pub(super) fn anvil_create_result(&mut self) -> Result<()> {
        let (result, cost) = self.anvil_result()?;
        self.utility_mut().data[COST] = cost;
        self.container[RESULT] = result;
        Ok(())
    }

    fn anvil_result(&mut self) -> Result<(RuntimeItemStack, i32)> {
        let input = self.container[INPUT].clone();
        let addition = self.container[ADDITION].clone();
        self.utility_mut().anvil.only_renaming = false;
        let Some(facts) = self.utility().facts.clone() else {
            return Ok((RuntimeItemStack::empty(), 0));
        };
        if input.is_empty() || !can_store_enchantments(&input)? {
            return Ok((RuntimeItemStack::empty(), 0));
        }
        let resolver = &facts.resolver;
        let infinite = self.utility().player.infinite_materials;
        let mut result = input.clone();
        let mut enchantments = crafting_enchantments(&result);
        let tax = i64::from(repair_cost(&input)?) + i64::from(repair_cost(&addition)?);
        let mut price: i32 = 0;
        let mut naming_cost = 0;
        self.utility_mut().anvil.repair_item_count_cost = 0;
        if !addition.is_empty() {
            let using_book = has_component(
                addition.summary(),
                VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
            )?;
            if is_damageable(&result)?
                && is_valid_repair_item(&input, &addition, &self.crafting.tags, resolver)?
            {
                let mut repair = damage_value(&result)?.min(max_damage(&result)? / 4);
                if repair <= 0 {
                    return Ok((RuntimeItemStack::empty(), 0));
                }
                let mut count = 0;
                while repair > 0 && count < addition.count() {
                    result = with_damage(&result, damage_value(&result)? - repair, resolver)?;
                    price += 1;
                    repair = damage_value(&result)?.min(max_damage(&result)? / 4);
                    count += 1;
                }
                self.utility_mut().anvil.repair_item_count_cost = count;
            } else {
                if !using_book
                    && (result.item_id() != addition.item_id() || !is_damageable(&result)?)
                {
                    return Ok((RuntimeItemStack::empty(), 0));
                }
                if is_damageable(&result)? && !using_book {
                    let remaining1 = max_damage(&input)? - damage_value(&input)?;
                    let remaining2 = max_damage(&addition)? - damage_value(&addition)?;
                    let additional = remaining2 + max_damage(&result)? * 12 / 100;
                    let result_damage = (max_damage(&result)? - (remaining1 + additional)).max(0);
                    if result_damage < damage_value(&result)? {
                        result = with_damage(&result, result_damage, resolver)?;
                        price += 2;
                    }
                }
                let mut any_compatible = false;
                let mut any_incompatible = false;
                for (holder, level) in crafting_enchantments(&addition) {
                    let current = enchantments
                        .iter()
                        .find(|(id, _)| *id == holder)
                        .map_or(0, |(_, level)| *level);
                    let mut level = if current == level {
                        level + 1
                    } else {
                        level.max(current)
                    };
                    let Some(definition) = facts.enchantments.get(holder) else {
                        any_incompatible = true;
                        continue;
                    };
                    let mut compatible = input
                        .item_id()
                        .is_some_and(|item| definition.supported_items.contains(&item));
                    if infinite || input.item_id() == Some(VANILLA_ITEM_ENCHANTED_BOOK_ID) {
                        compatible = true;
                    }
                    for &(other, _) in &enchantments {
                        if other != holder
                            && !enchantments_compatible(&facts.enchantments, holder, other)
                        {
                            compatible = false;
                            price += 1;
                        }
                    }
                    if !compatible {
                        any_incompatible = true;
                        continue;
                    }
                    any_compatible = true;
                    level = level.min(definition.max_level);
                    set_level(&mut enchantments, holder, level);
                    let mut fee = definition.anvil_cost;
                    if using_book {
                        fee = (fee / 2).max(1);
                    }
                    price += fee * level;
                    if input.count() > 1 {
                        price = TOO_EXPENSIVE;
                    }
                }
                if any_incompatible && !any_compatible {
                    return Ok((RuntimeItemStack::empty(), 0));
                }
            }
        }
        let item_name = self.utility().anvil.item_name.clone();
        match item_name.as_deref().filter(|name| !is_blank(name)) {
            Some(name) => {
                // `AnvilMenu.java:225`: compare against resolved hover name.
                if hover_name(&input, resolver)? != name {
                    naming_cost = 1;
                    price += naming_cost;
                    result = with_custom_name(&result, Some(name), resolver)?;
                }
            }
            None => {
                if has_component(input.summary(), VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID)? {
                    naming_cost = 1;
                    price += naming_cost;
                    result = with_custom_name(&result, None, resolver)?;
                }
            }
        }
        let mut cost = if price <= 0 {
            0
        } else {
            (tax + i64::from(price)).clamp(0, i64::from(i32::MAX)) as i32
        };
        if price <= 0 {
            result = RuntimeItemStack::empty();
        }
        if naming_cost == price && naming_cost > 0 {
            if cost >= TOO_EXPENSIVE {
                cost = TOO_EXPENSIVE - 1;
            }
            self.utility_mut().anvil.only_renaming = true;
        }
        if cost >= TOO_EXPENSIVE && !infinite {
            result = RuntimeItemStack::empty();
        }
        if !result.is_empty() {
            let mut base = repair_cost(&result)?.max(repair_cost(&addition)?);
            if naming_cost != price || naming_cost == 0 {
                base = increased_repair_cost(base);
            }
            result = with_repair_cost(&result, base, resolver)?;
            result = with_crafting_enchantments(&result, &enchantments, resolver)?;
        }
        Ok((result, cost))
    }

    /// `AnvilMenu.onTake` (`AnvilMenu.java:75-115`); the levels, the damage
    /// roll and the level event are the world's.
    pub(super) fn anvil_on_take(&mut self) -> Result<()> {
        let cost = self.utility().data[COST];
        let state = &self.utility().anvil;
        let (repair_count, only_renaming) = (state.repair_item_count_cost, state.only_renaming);
        if repair_count > 0 {
            let addition = &mut self.container[ADDITION];
            if !addition.is_empty() && addition.count() > repair_count {
                addition.shrink(repair_count);
            } else {
                *addition = RuntimeItemStack::empty();
            }
        } else if !only_renaming {
            self.container[ADDITION] = RuntimeItemStack::empty();
        }
        self.utility_mut().data[COST] = 0;
        self.container[INPUT] = RuntimeItemStack::empty();
        self.anvil_create_result()?;
        self.utility_mut()
            .events
            .push(UtilityMenuEvent::AnvilUsed { cost });
        Ok(())
    }

    /// `ServerGamePacketListenerImpl.handleRenameItem` (`:761-771`) into
    /// `AnvilMenu.setItemName` (`AnvilMenu.java:280-298`).
    pub fn rename_anvil_item(&mut self, name: &str) -> Result<bool> {
        if self.utility_menu_kind() != Some(super::UtilityMenuKind::Anvil) {
            return Ok(false);
        }
        // `AnvilMenu.validateName` (`AnvilMenu.java:300-303`) over
        // `StringUtil.filterText`.
        let filtered: String = name
            .chars()
            .filter(|&c| c != '\u{a7}' && c >= ' ' && c != '\u{7f}')
            .collect();
        if filtered.encode_utf16().count() > MAX_NAME_LENGTH
            || self.utility().anvil.item_name.as_deref() == Some(filtered.as_str())
        {
            return Ok(false);
        }
        self.utility_mut().anvil.item_name = Some(filtered);
        self.anvil_create_result()?;
        Ok(true)
    }
}

/// `StringUtil.isBlank`.
fn is_blank(name: &str) -> bool {
    name.chars().all(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::super::item_edits::transmute;
    use super::*;
    use crate::inventory::{ContainerAnchor, MenuLayout, UtilityMenuKind};
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    };
    use bbb_protocol::item_types::{
        VANILLA_ITEM_BOOK_ID, VANILLA_ITEM_DIAMOND_SWORD_ID, VANILLA_ITEM_ENCHANTED_BOOK_ID,
        VANILLA_ITEM_WRITTEN_BOOK_ID,
    };
    use bbb_protocol::packets::{
        BlockPos, DataComponentPatchSummary, ItemStackSummary, WrittenBookContentSummary,
    };

    fn opened_anvil(input: RuntimeItemStack) -> PlayerInventoryMenu {
        let mut menu = PlayerInventoryMenu::empty();
        menu.open_menu(
            crate::inventory::test_item_resolver(),
            None,
            MenuLayout::Utility(UtilityMenuKind::Anvil),
            ContainerAnchor {
                positions: vec![BlockPos { x: 0, y: 64, z: 0 }],
            },
            vec![input, RuntimeItemStack::empty(), RuntimeItemStack::empty()],
            false,
        )
        .unwrap()
        .unwrap();
        let world = crate::world::ServerWorld::new(crate::tests::datapack_config()).unwrap();
        menu.attach_utility_facts(std::sync::Arc::new(crate::inventory::UtilityMenuFacts {
            resolver: world.item_stack_resolver(),
            enchantments: std::sync::Arc::new(Vec::new()),
            registry_tags: std::sync::Arc::new(
                crate::registry_tags::RegistryTagSnapshot::builtin().clone(),
            ),
        }));
        menu
    }

    #[test]
    fn anvil_rename_compares_against_resolved_hover_name() {
        let sword = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(VANILLA_ITEM_DIAMOND_SWORD_ID),
            count: 1,
            component_patch: Default::default(),
        });
        let mut menu = opened_anvil(sword);
        menu.rename_anvil_item("Diamond Sword").unwrap();
        assert!(
            menu.slot_item(2).is_empty(),
            "renaming an item to its default translated hover name must not charge or produce a result"
        );
        assert_eq!(menu.utility_menu_data(), Some(&[0][..]));

        menu.rename_anvil_item("Excalibur").unwrap();
        assert_eq!(
            menu.slot_item(2).item_id(),
            Some(VANILLA_ITEM_DIAMOND_SWORD_ID)
        );
        assert_eq!(menu.utility_menu_data(), Some(&[1][..]));

        let mut book_patch = DataComponentPatchSummary::default();
        book_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID);
        book_patch.written_book = Some(WrittenBookContentSummary {
            title: "My Journal".into(),
            title_filter: None,
            author: "alice".into(),
            generation: 0,
            pages: Vec::new(),
            page_filters: Vec::new(),
            resolved: true,
        });
        let book = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(VANILLA_ITEM_WRITTEN_BOOK_ID),
            count: 1,
            component_patch: book_patch.into(),
        });
        let mut book_menu = opened_anvil(book);
        book_menu.rename_anvil_item("My Journal").unwrap();
        assert!(
            book_menu.slot_item(2).is_empty(),
            "renaming a written book to its own title must be a no-op"
        );
    }

    #[test]
    fn transmute_sanitizes_prototype_patch_entries() {
        let resolver = crate::inventory::test_item_resolver();
        // Patch max_stack_size=1 onto a Book (prototype is 64) and transmute to
        // EnchantedBook (prototype is 1): the redundant added patch entry must drop.
        let mut patch = DataComponentPatchSummary::default();
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID);
        patch.max_stack_size = Some(1);
        patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID);
        let book = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(VANILLA_ITEM_BOOK_ID),
            count: 1,
            component_patch: patch.into(),
        });
        let transmuted = transmute(&book, VANILLA_ITEM_ENCHANTED_BOOK_ID, resolver).unwrap();
        assert_eq!(transmuted.item_id(), Some(VANILLA_ITEM_ENCHANTED_BOOK_ID));
        assert!(
            !transmuted
                .summary()
                .component_patch
                .added_type_ids
                .contains(&VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID),
            "prototype-equal max_stack_size patch must be stripped on transmute"
        );
        // Removing ENCHANTMENT_GLINT_OVERRIDE stays on EnchantedBook because it is
        // in EnchantedBook's prototype, then drops when transmuted back to Book.
        let back = transmute(&transmuted, VANILLA_ITEM_BOOK_ID, resolver).unwrap();
        assert!(
            !back
                .summary()
                .component_patch
                .removed_type_ids
                .contains(&VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID),
            "removal of a component absent from target prototype must be stripped"
        );
    }
}
