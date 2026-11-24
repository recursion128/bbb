use super::*;

impl WorldStore {
    pub fn set_default_item_crafting_remainders(&mut self, remainders: BTreeMap<i32, i32>) {
        self.items.set_default_item_crafting_remainders(remainders)
    }

    pub fn set_furnace_fuel_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_furnace_fuel_item_ids(item_ids)
    }

    pub fn set_furnace_fuel_profile(
        &mut self,
        item_ids: BTreeSet<i32>,
        rules: Vec<crate::FurnaceFuelRule>,
    ) {
        self.items.set_furnace_fuel_profile(item_ids, rules)
    }

    pub fn set_brewing_potion_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_brewing_potion_item_ids(item_ids)
    }

    pub fn set_brewing_ingredient_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_brewing_ingredient_item_ids(item_ids)
    }

    pub fn set_enchantment_lapis_lazuli_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_enchantment_lapis_lazuli_item_ids(item_ids)
    }

    pub fn set_cartography_additional_items(
        &mut self,
        items: BTreeMap<i32, crate::CartographyAdditionalItemKind>,
    ) {
        self.items.set_cartography_additional_items(items)
    }

    pub fn cartography_additional_item_kind(
        &self,
        item_id: i32,
    ) -> Option<crate::CartographyAdditionalItemKind> {
        self.items
            .cartography_additional_items
            .get(&item_id)
            .copied()
    }

    pub fn set_freeze_immune_wearable_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_freeze_immune_wearable_item_ids(item_ids)
    }

    pub fn set_powder_snow_walkable_foot_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.items.set_powder_snow_walkable_foot_item_ids(item_ids)
    }

    pub fn item_stack_equipment(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> Option<(
        EquipmentSlot,
        Option<crate::entities::EntityEquipmentAssetId>,
    )> {
        self.items.item_stack_equipment(stack)
    }
}
