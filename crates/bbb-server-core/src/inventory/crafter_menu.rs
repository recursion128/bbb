//! `CrafterMenu` input admission and its independent recipe preview.

use crate::world::maps::ServerMapStore;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::{MenuLayout, PlayerInventoryMenu};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CrafterMenuState {
    pub(super) disabled: [bool; 9],
    pub(super) preview: RuntimeItemStack,
}

impl Default for CrafterMenuState {
    fn default() -> Self {
        Self {
            disabled: [false; 9],
            preview: RuntimeItemStack::empty(),
        }
    }
}

impl PlayerInventoryMenu {
    pub fn is_crafter_menu(&self) -> bool {
        self.menu.layout == MenuLayout::Crafter
    }

    pub fn refresh_crafter_state(&mut self, items: Vec<RuntimeItemStack>, disabled: [bool; 9]) {
        if !self.is_crafter_menu() || items.len() != 9 {
            return;
        }
        self.container = items;
        self.menu.crafter.as_mut().expect("crafter layout").disabled = disabled;
    }

    // `CrafterMenu.slotChanged`: its listener runs before remote publication,
    // including inventory changes and the initial listeners-only scan.
    pub(super) fn rematch_crafter_preview(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
    ) -> Result<()> {
        if !self.is_crafter_menu() {
            return Ok(());
        }
        let preview = self.crafting_preview(resolver, maps, self.container_items())?;
        self.menu.crafter.as_mut().expect("crafter layout").preview = preview;
        Ok(())
    }

    pub(super) fn crafter_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        slot: i16,
    ) -> Result<RuntimeItemStack> {
        if !(0..45).contains(&slot) {
            return Ok(RuntimeItemStack::empty());
        }
        self.container_quick_move_stack(resolver, maps, 9, slot)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bbb_facts::container_inputs::ContainerInput;
    use bbb_protocol::{
        item_types::{VANILLA_ITEM_STICK_ID, VANILLA_ITEM_STONE_ID},
        packets::{BlockPos, ItemStackSummary},
    };
    use bbb_server_datapack::recipe::{ItemTagIndex, RecipeElement, RecipeSet};

    use super::*;
    use crate::inventory::{
        is_empty, ContainerAnchor, MenuBroadcastEvent, MenuBroadcastMode, MenuSlot, MenuSync,
    };

    fn stack(item: i32, count: i32) -> RuntimeItemStack {
        RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(item),
            count,
            component_patch: Default::default(),
        })
    }

    #[test]
    fn crafter_preview_uses_listener_order_and_independent_slot_permissions() {
        let source = bbb_registry_codec::decode_direct_registry_json(
            br#"{
                "type": "minecraft:crafting_shaped",
                "pattern": ["SSS", "SSS", "SSS"],
                "key": {"S": "minecraft:stone"},
                "result": {"id": "minecraft:stick", "count": 2}
            }"#,
        )
        .unwrap();
        let startup = crate::registry_sync::builtin_datapack_startup_shared();
        let lookup = bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup::from_startup(
            &startup,
        );
        let recipes = Arc::new(RecipeSet {
            recipes: vec![RecipeElement {
                id: "test:preview".into(),
                source_pack: "test".into(),
                recipe: bbb_server_datapack::recipe::parse_recipe_source_with_lookup(
                    source.root(),
                    &lookup,
                )
                .unwrap(),
            }],
            warnings: Vec::new(),
        });
        let mut menu = PlayerInventoryMenu::empty();
        menu.set_crafting_slot(
            crate::inventory::test_item_resolver(),
            None,
            0,
            stack(VANILLA_ITEM_STONE_ID, 7),
        )
        .unwrap();
        menu.prepare_menu(
            crate::inventory::test_item_resolver(),
            None,
            MenuLayout::Crafter,
            ContainerAnchor {
                positions: vec![BlockPos { x: 0, y: 0, z: 0 }],
            },
            vec![stack(VANILLA_ITEM_STONE_ID, 1); 9],
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(MenuLayout::Crafter.slot_count(), 46);
        assert_eq!(MenuLayout::Crafter.slot(9), Some(MenuSlot::Inventory(9)));
        assert_eq!(MenuLayout::Crafter.slot(44), Some(MenuSlot::Inventory(8)));
        assert_eq!(MenuLayout::Crafter.slot(45), Some(MenuSlot::CrafterPreview));
        assert!(menu.is_crafter_menu());
        menu.install_crafting_catalog(
            crate::inventory::test_item_resolver(),
            None,
            recipes.clone(),
            ItemTagIndex::default(),
        )
        .unwrap();
        let preview = stack(VANILLA_ITEM_STICK_ID, 2);
        assert_eq!(&*menu.slot_item(45), &preview);
        let mut inputs = menu.container_items().to_vec();
        inputs[0] = RuntimeItemStack::empty();
        menu.refresh_crafter_state(inputs, [false; 9]);
        let mut cursor = menu.begin_menu_broadcast(MenuBroadcastMode::ListenersOnly);
        while let Some(event) = menu
            .next_menu_broadcast(crate::inventory::test_item_resolver(), None, &mut cursor)
            .unwrap()
        {
            assert!(!matches!(event, MenuBroadcastEvent::Remote(_)));
        }
        assert!(menu.slot_item(45).is_empty());
        menu.refresh_crafter_state(vec![stack(VANILLA_ITEM_STONE_ID, 1); 9], [false; 9]);
        let mut cursor = menu.begin_menu_broadcast(MenuBroadcastMode::ListenersOnly);
        while menu
            .next_menu_broadcast(crate::inventory::test_item_resolver(), None, &mut cursor)
            .unwrap()
            .is_some()
        {}
        assert_eq!(&*menu.slot_item(45), &preview);
        menu.send_all_data_to_remote().unwrap();

        menu.clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            45,
            0,
            ContainerInput::Pickup,
            false,
        )
        .unwrap();
        menu.clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            45,
            0,
            ContainerInput::QuickMove,
            false,
        )
        .unwrap();
        assert!(menu.carried().is_empty());
        assert!(menu
            .slot_remove(&bbb_registry_codec::UnavailableHolderResolver, None, 45, 1)
            .unwrap()
            .is_empty());
        assert!(!menu.slot_allows_modification(45, true).unwrap());
        menu.clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            45,
            2,
            ContainerInput::Clone,
            true,
        )
        .unwrap();
        assert_eq!(menu.carried(), &stack(VANILLA_ITEM_STICK_ID, 64));
        assert_eq!(
            menu.container_items(),
            vec![stack(VANILLA_ITEM_STONE_ID, 1); 9]
        );
        assert_eq!(menu.crafting_slots()[0], stack(VANILLA_ITEM_STONE_ID, 7));
        assert_eq!(menu.crafted_count(VANILLA_ITEM_STICK_ID), 0);
        assert!(menu.recipe_book().known().next().is_none());
        menu.set_carried(RuntimeItemStack::empty());

        let mut items = menu.container_items().to_vec();
        items[0] = RuntimeItemStack::empty();
        menu.refresh_crafter_state(
            items,
            [true, false, false, false, false, false, false, false, false],
        );
        menu.set_menu_data(0, 0);
        menu.set_carried(stack(VANILLA_ITEM_STONE_ID, 1));
        menu.clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            0,
            ContainerInput::Pickup,
            false,
        )
        .unwrap();
        assert!(menu.container_items()[0].is_empty());
        assert_eq!(menu.carried(), &stack(VANILLA_ITEM_STONE_ID, 1));
        menu.set_carried(RuntimeItemStack::empty());
        menu.set_inventory_slot(0, stack(VANILLA_ITEM_STONE_ID, 1));
        menu.clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            36,
            0,
            ContainerInput::QuickMove,
            false,
        )
        .unwrap();
        assert!(menu.container_items()[0].is_empty());
        assert_eq!(menu.container_items()[1], stack(VANILLA_ITEM_STONE_ID, 2));
        let sync = menu
            .broadcast_changes(crate::inventory::test_item_resolver(), None)
            .unwrap();
        assert!(sync
            .iter()
            .any(|event| matches!(event, MenuSync::Slot { slot: 45, item, .. } if is_empty(item))));
        assert!(menu.slot_item(45).is_empty());

        let mut items = menu.container_items().to_vec();
        items[0] = stack(VANILLA_ITEM_STONE_ID, 1);
        menu.refresh_crafter_state(items, [false; 9]);
        let sync = menu
            .broadcast_changes(crate::inventory::test_item_resolver(), None)
            .unwrap();
        assert!(sync
            .iter()
            .any(|event| matches!(event, MenuSync::Slot { slot: 45, item, .. } if item == preview.summary())));
        menu.install_crafting_catalog(
            crate::inventory::test_item_resolver(),
            None,
            Arc::new(RecipeSet {
                recipes: Vec::new(),
                warnings: Vec::new(),
            }),
            ItemTagIndex::default(),
        )
        .unwrap();
        assert!(menu.slot_item(45).is_empty());
        menu.install_crafting_catalog(
            crate::inventory::test_item_resolver(),
            None,
            recipes,
            ItemTagIndex::default(),
        )
        .unwrap();
        assert_eq!(&*menu.slot_item(45), &preview);
        menu.clicked(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            0,
            ContainerInput::QuickMove,
            false,
        )
        .unwrap();
        assert_eq!(menu.slot(8).cloned(), Some(stack(VANILLA_ITEM_STONE_ID, 1)));
        assert!(menu.container_items()[0].is_empty());
    }
}
