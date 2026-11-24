//! `AbstractCraftingMenu` grids -- the player's 2x2 and a crafting table's
//! 3x3 -- with `slotsChanged` / `ResultSlot.onTake`, and the live RecipeBook.

use crate::world::maps::ServerMapStore;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use bbb_server_datapack::recipe::{
    CraftingInput, CraftingInputPositioned, ItemTagIndex, RecipeSet,
};
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::{
    DroppedMenuStack, MenuLayout, MenuSlot, MenuSync, PlayerInventoryMenu, MENU_RESULT_SLOT,
};

/// `InventoryMenu` 2x2 craft grid (`InventoryMenu.java:16-17`).
const PLAYER_CRAFT_WIDTH: i32 = 2;
/// `CraftingMenu.CRAFTING_GRID_WIDTH` (`CraftingMenu.java:20-21`).
const TABLE_CRAFT_WIDTH: i32 = 3;
/// `CraftingMenu.INV_SLOT_START` .. `USE_ROW_SLOT_END` (`CraftingMenu.java:26-29`).
const TABLE_INV_SLOT_START: i16 = 10;
const TABLE_USE_ROW_SLOT_START: i16 = 37;
const TABLE_USE_ROW_SLOT_END: i16 = 46;

/// A crafting table menu's own `craftSlots` and `resultSlots`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CraftingTableState {
    grid: Vec<RuntimeItemStack>,
    result: RuntimeItemStack,
    recipe_used: Option<String>,
}

impl Default for CraftingTableState {
    fn default() -> Self {
        Self {
            grid: vec![RuntimeItemStack::empty(); 9],
            result: RuntimeItemStack::empty(),
            recipe_used: None,
        }
    }
}

/// Live `ServerRecipeBook` known / highlight sets (`ServerRecipeBook.java:32-34`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecipeBook {
    known: BTreeSet<String>,
    to_be_displayed: BTreeSet<String>,
}

impl RecipeBook {
    pub fn contains(&self, id: &str) -> bool {
        self.known.contains(id)
    }

    pub fn known(&self) -> impl Iterator<Item = &str> {
        self.known.iter().map(String::as_str)
    }

    /// `ServerRecipeBook.highlight` (`ServerRecipeBook.java:33-34`), the half
    /// `Packed` writes as `toBeDisplayed`.
    pub fn to_be_displayed(&self) -> impl Iterator<Item = &str> {
        self.to_be_displayed.iter().map(String::as_str)
    }

    /// `ServerRecipeBook.addRecipes` (`ServerRecipeBook.java:61-70`).
    pub fn award(&mut self, id: impl Into<String>) -> bool {
        let id = id.into();
        if !self.known.insert(id.clone()) {
            return false;
        }
        self.to_be_displayed.insert(id);
        true
    }

    /// `ServerRecipeBook.remove` (`ServerRecipeBook.java:48-51`).
    pub fn remove(&mut self, id: &str) -> bool {
        let removed = self.known.remove(id);
        self.to_be_displayed.remove(id);
        removed
    }

    /// `ServerRecipeBook.removeHighlight` (`ServerRecipeBook.java:53-55`).
    pub fn remove_highlight(&mut self, id: &str) -> bool {
        self.to_be_displayed.remove(id)
    }

    pub fn retain_known<F>(&mut self, mut keep: F)
    where
        F: FnMut(&str) -> bool,
    {
        self.known.retain(|id| keep(id));
        self.to_be_displayed.retain(|id| self.known.contains(id));
    }
}

#[derive(Debug, Clone)]
struct CraftingPublication(Arc<DatapackStartup>);

impl PartialEq for CraftingPublication {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CraftedRecipe {
    pub recipe_id: String,
    pub ingredients: Vec<RuntimeItemStack>,
    pub unlock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct SmithingCatalog {
    pub(super) inputs: [BTreeSet<i32>; 3],
    pub(super) recipe_indices: Vec<usize>,
}

impl SmithingCatalog {
    fn build(recipes: &RecipeSet, tags: &ItemTagIndex) -> Self {
        use bbb_server_datapack::recipe::ingredient::IngredientTerm;
        use bbb_server_datapack::recipe::{Ingredient, RecipeKind};
        let mut catalog = Self::default();
        let insert = |set: &mut BTreeSet<i32>, ingredient: Option<&Ingredient>| {
            let Some(ingredient) = ingredient else {
                return;
            };
            for term in &ingredient.terms {
                match term {
                    IngredientTerm::Item(item) => {
                        set.insert(*item);
                    }
                    IngredientTerm::Tag(tag) => {
                        set.extend(tags.members(tag));
                    }
                }
            }
        };
        for (index, element) in recipes.recipes.iter().enumerate() {
            match &element.recipe.kind {
                RecipeKind::SmithingTransform(recipe) => {
                    catalog.recipe_indices.push(index);
                    insert(&mut catalog.inputs[0], recipe.template.as_ref());
                    insert(&mut catalog.inputs[1], Some(&recipe.base));
                    insert(&mut catalog.inputs[2], recipe.addition.as_ref());
                }
                RecipeKind::SmithingTrim(recipe) => {
                    catalog.recipe_indices.push(index);
                    insert(&mut catalog.inputs[0], Some(&recipe.template));
                    insert(&mut catalog.inputs[1], Some(&recipe.base));
                    insert(&mut catalog.inputs[2], Some(&recipe.addition));
                }
                _ => {}
            }
        }
        catalog
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct InventoryCrafting {
    publication: Option<CraftingPublication>,
    pub(super) recipes: Option<Arc<RecipeSet>>,
    pub(super) tags: Arc<ItemTagIndex>,
    pub(super) smithing: Option<Arc<SmithingCatalog>>,
    book: RecipeBook,
    recipe_used: Option<String>,
    /// `ResultSlot.removeCount` (`ResultSlot.java:16`): how much of the result
    /// the take *asked* for, which is what the crafted stat is awarded for
    /// even when the slot hands back more.
    result_remove_count: i32,
    pub(super) crafted_counts: BTreeMap<i32, i32>,
    dropped: Vec<DroppedMenuStack>,
    pending_sync: Vec<MenuSync>,
    /// Recipe takes awaiting world settlement.
    pub(super) pending_awards: Vec<CraftedRecipe>,
    /// `GameRules.LIMITED_CRAFTING`, which `RecipeCraftingHolder.setRecipeUsed`
    /// reads (`RecipeCraftingHolder.java:28-35`).
    limited_crafting: bool,
}

impl InventoryCrafting {
    pub(super) fn respawned(&self) -> Self {
        Self {
            publication: self.publication.clone(),
            recipes: self.recipes.clone(),
            tags: Arc::clone(&self.tags),
            smithing: self.smithing.clone(),
            // ServerPlayer.java:1616: recipe unlocks survive every respawn.
            book: self.book.clone(),
            limited_crafting: self.limited_crafting,
            ..Self::default()
        }
    }

    pub(super) fn input(
        &self,
        width: i32,
        height: i32,
        items: &[RuntimeItemStack],
    ) -> Result<CraftingInputPositioned> {
        Ok(CraftingInput::of_positioned(
            width,
            height,
            items.iter().map(|item| item.summary().clone()).collect(),
        ))
    }

    fn select<'a>(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        recipes: &'a RecipeSet,
        input: &CraftingInput,
        hint: Option<&str>,
        maps: Option<&mut ServerMapStore>,
    ) -> Result<Option<&'a bbb_server_datapack::recipe::RecipeElement>> {
        let map_data = maps.and_then(|maps| maps.crafting_map_data(input));
        if let Some(publication) = &self.publication {
            if std::ptr::eq(publication.0.recipes.as_ref(), recipes) {
                return Ok(publication.0.recipe_index.get_recipe_for_crafting(
                    recipes, resolver, input, hint, &self.tags, map_data,
                ));
            }
        }
        Ok(recipes.get_recipe_for_crafting(resolver, input, hint, &self.tags, map_data))
    }

    fn assemble(
        &self,
        recipe: &bbb_server_datapack::recipe::Recipe,
        input: &CraftingInput,
    ) -> Result<RuntimeItemStack> {
        Ok(recipe.assemble_crafting(input, &self.tags).into())
    }
}

impl PlayerInventoryMenu {
    pub(in crate::inventory) fn post_process_crafting_result(
        maps: Option<&mut ServerMapStore>,
        resolver: &dyn PersistentItemStackHolderResolver,
        item: RuntimeItemStack,
    ) -> Result<RuntimeItemStack> {
        match maps {
            Some(maps) => maps.post_process_crafted_map(item, resolver),
            None => Ok(item),
        }
    }

    pub(crate) fn install_crafting_publication(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        startup: Arc<DatapackStartup>,
        tags: impl Into<Arc<ItemTagIndex>>,
    ) -> Result<()> {
        self.reload_crafting_publication(startup, tags);
        self.with_inventory_menu(|menu| menu.rematch_crafting_grid(resolver, maps.as_deref_mut()))?;
        self.rematch_crafter_preview(resolver, maps)?;
        Ok(())
    }

    pub fn reload_crafting_publication(
        &mut self,
        startup: Arc<DatapackStartup>,
        tags: impl Into<Arc<ItemTagIndex>>,
    ) {
        let tags = tags.into();
        let smithing = Arc::new(SmithingCatalog::build(&startup.recipes, &tags));
        self.crafting.recipes = Some(Arc::clone(&startup.recipes));
        self.crafting.publication = Some(CraftingPublication(startup));
        self.crafting.tags = tags;
        self.crafting.smithing = Some(smithing);
    }
    /// Install the recipe catalog used by `InventoryMenu.slotsChanged`.
    #[cfg(test)]
    pub(crate) fn install_crafting_catalog(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        recipes: Arc<RecipeSet>,
        tags: impl Into<Arc<ItemTagIndex>>,
    ) -> Result<()> {
        self.set_crafting_catalog(recipes, tags);
        self.with_inventory_menu(|menu| menu.rematch_crafting_grid(resolver, maps.as_deref_mut()))?;
        self.rematch_crafter_preview(resolver, maps)?;
        Ok(())
    }

    pub fn reload_crafting_catalog(
        &mut self,
        recipes: Arc<RecipeSet>,
        tags: impl Into<Arc<ItemTagIndex>>,
    ) {
        // PlayerList.reloadResources sends recipes and book without rematching slots (PlayerList.java:841-855).
        self.set_crafting_catalog(recipes, tags);
    }

    fn set_crafting_catalog(
        &mut self,
        recipes: Arc<RecipeSet>,
        tags: impl Into<Arc<ItemTagIndex>>,
    ) {
        let tags = tags.into();
        let smithing = Arc::new(SmithingCatalog::build(&recipes, &tags));
        self.crafting.publication = None;
        self.crafting.recipes = Some(recipes);
        self.crafting.tags = tags;
        self.crafting.smithing = Some(smithing);
    }

    /// Takes effect at the next `slotsChanged`; the rule change itself does
    /// not rematch (`MinecraftServer.java:2100-2105`).
    pub fn set_limited_crafting(&mut self, limited: bool) {
        self.crafting.limited_crafting = limited;
    }

    pub fn recipe_book(&self) -> &RecipeBook {
        &self.crafting.book
    }

    pub(crate) fn recipe_book_mut(&mut self) -> &mut RecipeBook {
        &mut self.crafting.book
    }

    pub(super) fn crafting_preview(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        items: Vec<RuntimeItemStack>,
    ) -> Result<RuntimeItemStack> {
        let Some(recipes) = &self.crafting.recipes else {
            return Ok(RuntimeItemStack::empty());
        };
        let input = self.crafting.input(3, 3, &items)?.input;
        match self
            .crafting
            .select(resolver, recipes, &input, None, maps)?
        {
            Some(found) => self.crafting.assemble(&found.recipe, &input),
            None => Ok(RuntimeItemStack::empty()),
        }
    }

    pub fn apply_recipe_book(&mut self, book: RecipeBook) {
        self.crafting.book = book;
    }

    pub fn crafted_count(&self, item_id: i32) -> i32 {
        self.crafting
            .crafted_counts
            .get(&item_id)
            .copied()
            .unwrap_or(0)
    }

    pub fn dropped_from_craft(&self) -> &[DroppedMenuStack] {
        &self.crafting.dropped
    }

    /// What `Player.drop` (`Player.java:582-584`) was handed, moved out for the
    /// caller that owns the world the item entities would spawn in.
    pub(super) fn take_dropped_from_craft(&mut self) -> Vec<DroppedMenuStack> {
        std::mem::take(&mut self.crafting.dropped)
    }

    /// `ResultSlot.remove` (`ResultSlot.java:30-36`), `onQuickCraft`
    /// (`:39-42`) and `onSwapCraft` (`:45-47`) all add to the same counter.
    pub(super) fn note_result_remove_count(&mut self, count: i32) {
        self.crafting.result_remove_count += count;
    }

    pub fn is_crafting_table_menu(&self) -> bool {
        self.menu.layout == MenuLayout::CraftingTable
    }

    /// The open crafting menu's grid width: a table's 3x3 while one is open,
    /// otherwise `Player.inventoryMenu`'s 2x2.
    fn craft_width(&self) -> i32 {
        if self.menu.crafting_table.is_some() {
            TABLE_CRAFT_WIDTH
        } else {
            PLAYER_CRAFT_WIDTH
        }
    }

    pub(super) fn craft_grid(&self) -> &[RuntimeItemStack] {
        match &self.menu.crafting_table {
            Some(table) => &table.grid,
            None => &self.crafting_grid,
        }
    }

    pub(super) fn craft_grid_mut(&mut self) -> &mut [RuntimeItemStack] {
        match &mut self.menu.crafting_table {
            Some(table) => &mut table.grid,
            None => &mut self.crafting_grid,
        }
    }

    pub(super) fn craft_result(&self) -> &RuntimeItemStack {
        match &self.menu.crafting_table {
            Some(table) => &table.result,
            None => &self.crafting_result,
        }
    }

    pub(super) fn craft_result_mut(&mut self) -> &mut RuntimeItemStack {
        match &mut self.menu.crafting_table {
            Some(table) => &mut table.result,
            None => &mut self.crafting_result,
        }
    }

    fn recipe_used_mut(&mut self) -> &mut Option<String> {
        match &mut self.menu.crafting_table {
            Some(table) => &mut table.recipe_used,
            None => &mut self.crafting.recipe_used,
        }
    }

    /// `slotsChanged` of the open crafting menu (`InventoryMenu.java:81-84`,
    /// `CraftingMenu.java:76-84`) through `slotChangedCraftingGrid`
    /// (`CraftingMenu.java:47-73`).
    pub(super) fn rematch_crafting_grid(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
    ) -> Result<()> {
        let Some(recipes) = self.crafting.recipes.clone() else {
            return Ok(());
        };
        let width = self.craft_width();
        let input = self.crafting.input(width, width, self.craft_grid())?.input;
        let hint = self.recipe_used_mut().clone();
        if let Some(found) =
            self.crafting
                .select(resolver, &recipes, &input, hint.as_deref(), maps)?
        {
            // `RecipeCraftingHolder.setRecipeUsed` (`RecipeCraftingHolder.java:28-35`).
            if self.crafting.limited_crafting
                && !found.recipe.is_special()
                && !self.crafting.book.contains(&found.id)
            {
                *self.craft_result_mut() = RuntimeItemStack::empty();
                return Ok(());
            }
            let result = self.crafting.assemble(&found.recipe, &input)?;
            if !result.is_empty() {
                *self.recipe_used_mut() = Some(found.id.clone());
                *self.craft_result_mut() = result;
                return Ok(());
            }
        }
        *self.recipe_used_mut() = None;
        *self.craft_result_mut() = RuntimeItemStack::empty();
        Ok(())
    }

    /// `Player.inventoryMenu.slotsChanged`, whatever menu is open.
    pub(crate) fn crafting_slots_changed(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
    ) -> Result<()> {
        self.with_inventory_menu(|menu| menu.open_crafting_slots_changed(resolver, maps))
    }

    /// `slotsChanged` of the open crafting menu: rematch, then publish the
    /// result on that menu.
    pub(super) fn open_crafting_slots_changed(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
    ) -> Result<()> {
        self.rematch_crafting_grid(resolver, maps)?;
        let sync = self.publish_open_crafting_result();
        self.crafting.pending_sync.push(sync);
        Ok(())
    }

    pub(crate) fn take_crafted_recipes(&mut self) -> Vec<CraftedRecipe> {
        std::mem::take(&mut self.crafting.pending_awards)
    }

    pub fn take_crafting_sync(&mut self) -> Vec<MenuSync> {
        std::mem::take(&mut self.crafting.pending_sync)
    }

    /// Whether a menu slot addresses the 2x2 grid, the slots whose writes run
    /// `slotsChanged` through the container itself
    /// (`TransientCraftingContainer.java:64-67`).
    pub fn is_crafting_grid_slot(&self, slot: i16) -> bool {
        matches!(self.menu.layout.slot(slot), Some(MenuSlot::CraftingGrid(_)))
    }

    /// The publication half of `slotChangedCraftingGrid`
    /// (`CraftingMenu.java:70-72`): the result reaches the result slot of
    /// `Player.inventoryMenu` every time the grid changes, whatever stood
    /// there before, and the remote copy is forced with it so the
    /// `broadcastChanges` that follows does not report it again.
    pub fn publish_crafting_result(&mut self) -> Result<MenuSync> {
        Ok(self.with_inventory_menu(Self::publish_open_crafting_result))
    }

    /// `CraftingMenu.java:70-72` on the open menu; slot 0 is the result in
    /// both `InventoryMenu` and `CraftingMenu`.
    fn publish_open_crafting_result(&mut self) -> MenuSync {
        let item = self.craft_result().clone();
        self.set_remote_slot(MENU_RESULT_SLOT, &item);
        MenuSync::Slot {
            container_id: self.container_id(),
            state_id: self.increment_state_id(),
            slot: MENU_RESULT_SLOT,
            item: item.summary().clone(),
        }
    }

    /// `ResultSlot.onTake` (`ResultSlot.java:82-112`) after
    /// `checkTakeAchievements` (`ResultSlot.java:50-59`).
    pub(super) fn on_take_crafting_result(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        taken: &RuntimeItemStack,
        creative: bool,
    ) -> Result<()> {
        // `:51-53`: the stat is awarded for what `removeCount` collected, not
        // for what left the slot -- `ItemStack.onCraftedBy`
        // (`ItemStack.java:723-726`) is handed the counter, and a take that
        // asked for half of a result still hands the whole of it back.
        let removed = std::mem::replace(&mut self.crafting.result_remove_count, 0);
        if removed > 0 {
            if let Some(item_id) = taken.item_id() {
                *self.crafting.crafted_counts.entry(item_id).or_insert(0) += removed;
            }
        }
        if let Some(recipe_id) = self.recipe_used_mut().clone() {
            let special = self
                .crafting
                .recipes
                .as_ref()
                .and_then(|recipes| recipes.recipe(&recipe_id))
                .is_some_and(|element| element.recipe.is_special());
            // `RecipeCraftingHolder.awardUsedRecipes` (`RecipeCraftingHolder.java:17-26`).
            self.crafting.pending_awards.push(CraftedRecipe {
                unlock: !special && !self.crafting.book.contains(&recipe_id),
                recipe_id,
                ingredients: self.craft_grid().to_vec(),
            });
        }
        let width = self.craft_width();
        let positioned = self.crafting.input(width, width, self.craft_grid())?;
        let recipe = match &self.crafting.recipes {
            Some(recipes) => self.crafting.select(
                resolver,
                recipes,
                &positioned.input,
                None,
                maps.as_deref_mut(),
            )?,
            None => None,
        };
        let input_remainders = recipe.is_none();
        let remainders: Vec<RuntimeItemStack> = recipe
            .map(|element| element.recipe.remaining_items(&positioned.input))
            .unwrap_or_else(|| positioned.input.items().to_vec())
            .into_iter()
            .map(RuntimeItemStack::from_summary)
            .collect();
        let input = &positioned.input;
        for y in 0..input.height() {
            for x in 0..input.width() {
                let grid_index = (x + positioned.left + (y + positioned.top) * width) as usize;
                if grid_index >= self.craft_grid().len() {
                    continue;
                }
                if !self.craft_grid()[grid_index].is_empty() {
                    let _ = self.craft_grid_mut()[grid_index].split(1);
                    self.open_crafting_slots_changed(resolver, maps.as_deref_mut())?;
                }
                let replacement_index = (x + y * input.width()) as usize;
                // Without a recipe the remainders are the grid stacks themselves, read after the shrink.
                let replacement = if input_remainders {
                    Some(self.craft_grid()[grid_index].clone())
                } else {
                    remainders.get(replacement_index).cloned()
                };
                let Some(mut replacement) = replacement else {
                    continue;
                };
                if replacement.is_empty() {
                    continue;
                }
                let held = self.craft_grid()[grid_index].clone();
                if held.is_empty() {
                    // `:101-102`.
                    self.craft_grid_mut()[grid_index] = replacement;
                    self.open_crafting_slots_changed(resolver, maps.as_deref_mut())?;
                } else if held.same_item_same_effective_components(&replacement, resolver)? {
                    // `:103-105`: the remainder grows by what the cell kept.
                    let count = held.count() + replacement.count();
                    replacement.set_count(count);
                    self.craft_grid_mut()[grid_index] = replacement;
                    self.open_crafting_slots_changed(resolver, maps.as_deref_mut())?;
                } else {
                    let added = self.add_anywhere(resolver, &mut replacement, creative)?;
                    if !added {
                        self.crafting.dropped.push(DroppedMenuStack {
                            stack: replacement,
                            thrown_from_hand: false,
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

impl PlayerInventoryMenu {
    /// The grid `CraftingMenu.removed` (`CraftingMenu.java:98-101`) hands back
    /// through `clearContainer`, emptied from the menu.
    pub(super) fn take_crafting_table_grid(&mut self) -> Vec<RuntimeItemStack> {
        let Some(table) = self.menu.crafting_table.as_mut() else {
            return Vec::new();
        };
        table
            .grid
            .iter_mut()
            .map(|item| std::mem::replace(item, RuntimeItemStack::empty()))
            .filter(|item| !item.is_empty())
            .collect()
    }

    /// `CraftingMenu.quickMoveStack` (`CraftingMenu.java:109-153`).
    pub(super) fn crafting_table_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        creative: bool,
        dropped: &mut Vec<DroppedMenuStack>,
    ) -> Result<RuntimeItemStack> {
        let original = self.slot_item(slot).into_owned();
        if original.is_empty() {
            return Ok(RuntimeItemStack::empty());
        }
        let mut stack = original.clone();
        if slot == MENU_RESULT_SLOT
            && stack
                .summary()
                .component_patch
                .map_post_processing
                .is_some()
        {
            if !(TABLE_INV_SLOT_START..TABLE_USE_ROW_SLOT_END)
                .any(|slot| self.slot_item(slot).is_empty())
            {
                return Ok(RuntimeItemStack::empty());
            }
            stack = Self::post_process_crafting_result(maps.as_deref_mut(), resolver, stack)?;
        }
        let moved = if slot == MENU_RESULT_SLOT {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                TABLE_INV_SLOT_START,
                TABLE_USE_ROW_SLOT_END,
                true,
            )?
        } else if (TABLE_INV_SLOT_START..TABLE_USE_ROW_SLOT_END).contains(&slot) {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                1,
                TABLE_INV_SLOT_START,
                false,
            )? || if slot < TABLE_USE_ROW_SLOT_START {
                self.move_item_stack_to(
                    resolver,
                    maps.as_deref_mut(),
                    &mut stack,
                    TABLE_USE_ROW_SLOT_START,
                    TABLE_USE_ROW_SLOT_END,
                    false,
                )?
            } else {
                self.move_item_stack_to(
                    resolver,
                    maps.as_deref_mut(),
                    &mut stack,
                    TABLE_INV_SLOT_START,
                    TABLE_USE_ROW_SLOT_START,
                    false,
                )?
            }
        } else {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                TABLE_INV_SLOT_START,
                TABLE_USE_ROW_SLOT_END,
                false,
            )?
        };
        if !moved {
            return Ok(RuntimeItemStack::empty());
        }
        self.finish_slot_move(resolver, maps.as_deref_mut(), slot, stack.clone())?;
        if stack.count() == original.count() {
            return Ok(RuntimeItemStack::empty());
        }
        if slot == MENU_RESULT_SLOT {
            let taken_count = original.count() - stack.count();
            self.note_result_remove_count(taken_count);
            self.on_take_crafting_result(
                resolver,
                maps,
                &original.with_count(taken_count),
                creative,
            )?;
            // `player.drop(stack, false)` (`CraftingMenu.java:147-149`).
            dropped.push(DroppedMenuStack {
                stack,
                thrown_from_hand: false,
            });
        }
        Ok(original)
    }
}

/// `ServerRecipeBook.Packed` known / highlight lists (`ServerRecipeBook.java:150-158`);
/// a highlight that is not a known recipe is dropped.
pub fn recipe_book_from_id_lists(recipes: &[String], to_be_displayed: &[String]) -> RecipeBook {
    let known: BTreeSet<String> = recipes.iter().cloned().collect();
    let to_be_displayed = to_be_displayed
        .iter()
        .filter(|id| known.contains(*id))
        .cloned()
        .collect();
    RecipeBook {
        known,
        to_be_displayed,
    }
}
