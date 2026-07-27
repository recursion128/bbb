//! Vanilla creative-inventory catalog facts and canonical state.

mod generated_26_1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreativeInventoryTabRow {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreativeInventoryTabType {
    Category,
    Hotbar,
    Search,
    Inventory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreativeInventoryBackground {
    Items,
    ItemSearch,
    Inventory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreativeInventoryTabVisibility {
    ParentAndSearch,
    ParentOnly,
    SearchOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreativeInventoryDynamicEntry {
    FireworksAllDurations,
    Instruments {
        item_resource_id: &'static str,
        tag_resource_id: &'static str,
    },
    Potions {
        item_resource_id: &'static str,
    },
    SuspiciousStews,
    OminousBottles,
    EnchantmentBooksMaxLevel,
    EnchantmentBooksAllLevels,
    PresetPaintings {
        placeable: bool,
    },
    OminousBanner,
    TestBlockModes,
    LightLevels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreativeInventoryCatalogEntry {
    Item {
        resource_id: &'static str,
        visibility: CreativeInventoryTabVisibility,
    },
    Dynamic {
        kind: CreativeInventoryDynamicEntry,
        visibility: CreativeInventoryTabVisibility,
    },
}

impl CreativeInventoryCatalogEntry {
    pub const fn visibility(self) -> CreativeInventoryTabVisibility {
        match self {
            Self::Item { visibility, .. } | Self::Dynamic { visibility, .. } => visibility,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreativeInventoryTab {
    pub resource_id: &'static str,
    pub row: CreativeInventoryTabRow,
    pub column: u8,
    pub title_key: &'static str,
    pub icon_item_resource_id: &'static str,
    pub tab_type: CreativeInventoryTabType,
    pub background: CreativeInventoryBackground,
    pub aligned_right: bool,
    pub show_title: bool,
    pub can_scroll: bool,
    pub requires_permissions: bool,
    pub entries: &'static [CreativeInventoryCatalogEntry],
}

pub const fn vanilla_creative_inventory_tabs() -> &'static [CreativeInventoryTab] {
    generated_26_1::VANILLA_CREATIVE_INVENTORY_TABS
}
