use std::borrow::Cow;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use crate::item_models::tooltip::{tooltip_lines_for_stack_with_context, NativeItemTooltipOptions};
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_world::{for_each_shaped_recipe_grid_slot, FxHashMap, WorldStore};
use bbb_protocol::packets::{
    CraftingRecipeDisplaySummary, DataComponentPatchSummary, ItemStackSummary, RecipeDisplayEntry,
    SlotDisplaySummary, SlotDisplayValue,
};
use bbb_protocol::recipe_wire_types::{vanilla_recipe_book_search_category_ids, RecipeBookType};

pub(crate) const RECIPE_BOOK_ITEMS_PER_PAGE: usize = 20;
pub(crate) const RECIPE_BOOK_OVERLAY_BUTTON_SIZE: i32 = 25;
pub(crate) const RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE: i32 = 24;
pub(crate) const RECIPE_BOOK_OVERLAY_BACKGROUND_BORDER: i32 = 8;
pub(crate) const RECIPE_BOOK_OVERLAY_MAX_ROW: usize = 4;
pub(crate) const RECIPE_BOOK_OVERLAY_MAX_ROW_LARGE: usize = 5;
pub(crate) const RECIPE_BOOK_OVERLAY_ITEM_SCALE: f32 = 0.375;

const RECIPE_BOOK_OVERLAY_GRID_ITEM_OFFSET: i32 = 2;
const RECIPE_BOOK_OVERLAY_GRID_ITEM_STRIDE: i32 = 7;

const PLAYER_INVENTORY_SLOT_START: i32 = 0;
const PLAYER_INVENTORY_SLOT_END: i32 = 36;
const LOCAL_INVENTORY_CRAFT_SLOT_START: i16 = 1;
const LOCAL_INVENTORY_CRAFT_SLOT_END: i16 = 5;
const CRAFTING_TABLE_CRAFT_SLOT_START: i16 = 1;
const CRAFTING_TABLE_CRAFT_SLOT_END: i16 = 10;
const FURNACE_CRAFT_SLOT_START: i16 = 0;
const FURNACE_CRAFT_SLOT_END: i16 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RecipeBookCraftingGrid {
    pub(crate) width: i32,
    pub(crate) height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RecipeBookFurnaceFamily {
    Furnace,
    BlastFurnace,
    Smoker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RecipeBookCacheKind {
    Crafting(RecipeBookCraftingGrid),
    Furnace(RecipeBookFurnaceFamily),
}

impl RecipeBookCacheKind {
    fn book_type(self) -> RecipeBookType {
        match self {
            Self::Crafting(_) => RecipeBookType::Crafting,
            Self::Furnace(RecipeBookFurnaceFamily::Furnace) => RecipeBookType::Furnace,
            Self::Furnace(RecipeBookFurnaceFamily::BlastFurnace) => RecipeBookType::BlastFurnace,
            Self::Furnace(RecipeBookFurnaceFamily::Smoker) => RecipeBookType::Smoker,
        }
    }

    fn entry_matches_book(self, entry: &RecipeDisplayEntry) -> bool {
        match self {
            Self::Crafting(grid) => crafting_recipe_fits_grid(entry, grid),
            Self::Furnace(_) => entry.display.furnace.is_some(),
        }
    }

    fn tab_categories(self, selected_tab_index: usize) -> Option<&'static [i32]> {
        recipe_book_tab_categories(self.book_type(), selected_tab_index)
    }

    fn available_item_counts(self, world: &WorldStore) -> BTreeMap<i32, i32> {
        match self {
            Self::Crafting(grid) => crafting_recipe_book_available_item_counts(world, grid),
            Self::Furnace(_) => furnace_recipe_book_available_item_counts(world),
        }
    }

    fn available_items_fingerprint(self, world: &WorldStore) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for slot in &world.inventory().player_slots {
            if (PLAYER_INVENTORY_SLOT_START..PLAYER_INVENTORY_SLOT_END).contains(&slot.slot) {
                if let (Some(item_id), count) = (slot.item.item_id, slot.item.count) {
                    if count > 0 {
                        slot.slot.hash(&mut hasher);
                        item_id.hash(&mut hasher);
                        count.hash(&mut hasher);
                    }
                }
            }
        }
        match self {
            Self::Crafting(grid)
                if world.local_inventory_is_open() && grid.width == 2 && grid.height == 2 =>
            {
                1u8.hash(&mut hasher);
                hash_container_slot_range(
                    world,
                    &world.inventory().inventory_menu,
                    LOCAL_INVENTORY_CRAFT_SLOT_START,
                    LOCAL_INVENTORY_CRAFT_SLOT_END,
                    &mut hasher,
                );
            }
            Self::Crafting(grid) if grid.width == 3 && grid.height == 3 => {
                2u8.hash(&mut hasher);
                if let Some(container) = world.inventory().open_container.as_ref() {
                    hash_container_slot_range(
                        world,
                        container,
                        CRAFTING_TABLE_CRAFT_SLOT_START,
                        CRAFTING_TABLE_CRAFT_SLOT_END,
                        &mut hasher,
                    );
                }
            }
            Self::Crafting(_) => {
                0u8.hash(&mut hasher);
            }
            Self::Furnace(_) => {
                3u8.hash(&mut hasher);
                if let Some(container) = world.inventory().open_container.as_ref() {
                    hash_container_slot_range(
                        world,
                        container,
                        FURNACE_CRAFT_SLOT_START,
                        FURNACE_CRAFT_SLOT_END,
                        &mut hasher,
                    );
                }
            }
        }
        hasher.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecipeBookVisibleTabsKey {
    world_ptr: usize,
    level_generation: u64,
    recipe_revision: u64,
    known_fingerprint: u64,
    book_kind: RecipeBookCacheKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecipeBookSearchIndexKey {
    world_ptr: usize,
    level_generation: u64,
    recipe_revision: u64,
    known_fingerprint: u64,
    item_runtime_ptr: usize,
    resource_generation: u64,
    registry_contents_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecipeBookCollectionsKey {
    world_ptr: usize,
    level_generation: u64,
    recipe_revision: u64,
    known_fingerprint: u64,
    tags_revision: u64,
    item_tags_fingerprint: u64,
    inventory_fingerprint: u64,
    book_kind: RecipeBookCacheKind,
    selected_tab_index: usize,
    only_craftable: bool,
    search_lower: String,
    search_runtime_key: Option<(usize, u64, u64)>,
}

struct CachedRecipeBookCollection {
    entry_ids: Vec<i32>,
    craftable_entries: Vec<bool>,
    has_craftable: bool,
}

struct CachedRecipeSearchText {
    namespace_lower: Option<String>,
    path_lower: Option<String>,
    tooltip_lower: Vec<String>,
}

#[derive(Default)]
struct RecipeBookUiCache {
    visible_tabs_key: Option<RecipeBookVisibleTabsKey>,
    visible_tabs_mask: u16,
    collections_key: Option<RecipeBookCollectionsKey>,
    collections: Vec<CachedRecipeBookCollection>,
    search_index_key: Option<RecipeBookSearchIndexKey>,
    search_index: FxHashMap<i32, CachedRecipeSearchText>,
    #[cfg(test)]
    collection_builds: usize,
    #[cfg(test)]
    visible_tab_scans: usize,
    #[cfg(test)]
    search_entry_builds: usize,
}

thread_local! {
    static RECIPE_BOOK_UI_CACHE: std::cell::RefCell<RecipeBookUiCache> =
        std::cell::RefCell::new(RecipeBookUiCache::default());
}

#[derive(Debug, Clone)]
pub(crate) struct RecipeBookUiCollection<'a> {
    entries: Vec<&'a RecipeDisplayEntry>,
    craftable_entries: Vec<bool>,
    has_craftable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RecipeBookOverlayEntry<'a> {
    pub(crate) recipe_index: i32,
    pub(crate) items: Vec<RecipeBookOverlayItem<'a>>,
    pub(crate) craftable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RecipeBookOverlayItem<'a> {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) stack: Cow<'a, ItemStackSummary>,
}

impl<'a> RecipeBookUiCollection<'a> {
    pub(crate) fn recipe_indices(&self) -> impl Iterator<Item = i32> + '_ {
        self.entries.iter().map(|entry| entry.id.index)
    }

    pub(crate) fn result_stack(&self) -> Option<&'a ItemStackSummary> {
        self.entries
            .first()
            .and_then(|entry| recipe_book_result_stack(entry))
    }

    pub(crate) fn result_stack_at_slot_select_index(
        &self,
        slot_select_index: usize,
    ) -> Option<&'a ItemStackSummary> {
        self.entry_at_slot_select_index(slot_select_index)
            .and_then(recipe_book_result_stack)
    }

    pub(crate) fn recipe_index_and_craftable_at_slot_select_index(
        &self,
        slot_select_index: usize,
    ) -> Option<(i32, bool)> {
        if self.entries.is_empty() {
            return None;
        }
        let index = slot_select_index % self.entries.len();
        Some((
            self.entries[index].id.index,
            self.craftable_entries.get(index).copied().unwrap_or(false),
        ))
    }

    pub(crate) fn has_multiple_recipes(&self) -> bool {
        self.entries.len() > 1
    }

    pub(crate) fn has_craftable(&self) -> bool {
        self.has_craftable
    }

    pub(crate) fn all_result_stacks_same(&self) -> bool {
        let Some(first) = self.result_stack() else {
            return true;
        };
        self.entries
            .iter()
            .all(|entry| recipe_book_result_stack(entry) == Some(first))
    }

    pub(crate) fn overlay_entries(
        &self,
        item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
        slot_select_index: usize,
    ) -> Vec<RecipeBookOverlayEntry<'a>> {
        let mut entries = Vec::with_capacity(self.entries.len());
        self.push_overlay_entries_with_craftability(
            true,
            item_tag_entries,
            slot_select_index,
            &mut entries,
        );
        self.push_overlay_entries_with_craftability(
            false,
            item_tag_entries,
            slot_select_index,
            &mut entries,
        );
        entries
    }

    fn entry_at_slot_select_index(
        &self,
        slot_select_index: usize,
    ) -> Option<&'a RecipeDisplayEntry> {
        if self.entries.is_empty() {
            return None;
        }
        Some(self.entries[slot_select_index % self.entries.len()])
    }

    fn push_overlay_entries_with_craftability(
        &self,
        craftable: bool,
        item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
        slot_select_index: usize,
        out: &mut Vec<RecipeBookOverlayEntry<'a>>,
    ) {
        for (entry, entry_craftable) in self.entries.iter().zip(&self.craftable_entries) {
            if *entry_craftable != craftable {
                continue;
            }
            out.push(RecipeBookOverlayEntry {
                recipe_index: entry.id.index,
                items: recipe_book_overlay_items_for_entry(
                    entry,
                    item_tag_entries,
                    slot_select_index,
                ),
                craftable,
            });
        }
    }
}

pub(crate) fn recipe_book_slot_select_index(world: &WorldStore, partial_tick: f32) -> usize {
    let render_time = world.world_time().map_or(0.0, |time| time.game_time as f64)
        + f64::from(partial_tick.max(0.0));
    (render_time / 30.0).floor().max(0.0) as usize
}

pub(crate) fn recipe_book_overlay_max_row(entry_count: usize) -> usize {
    if entry_count <= 16 {
        RECIPE_BOOK_OVERLAY_MAX_ROW
    } else {
        RECIPE_BOOK_OVERLAY_MAX_ROW_LARGE
    }
}

pub(crate) fn recipe_book_overlay_rows(entry_count: usize) -> usize {
    entry_count.div_ceil(recipe_book_overlay_max_row(entry_count))
}

pub(crate) fn recipe_book_overlay_origin(
    button_x: i32,
    button_y: i32,
    entry_count: usize,
) -> (i32, i32) {
    let max_row = recipe_book_overlay_max_row(entry_count);
    let rows = recipe_book_overlay_rows(entry_count);
    let mut x = button_x;
    let mut y = button_y;
    let right_pos = x + i32::try_from(entry_count.min(max_row)).unwrap_or_default()
        * RECIPE_BOOK_OVERLAY_BUTTON_SIZE;
    let max_left_pos = 147 / 2 + 50;
    if right_pos > max_left_pos {
        x -= RECIPE_BOOK_OVERLAY_BUTTON_SIZE
            * ((right_pos - max_left_pos) / RECIPE_BOOK_OVERLAY_BUTTON_SIZE);
    }

    let bottom_pos = y + i32::try_from(rows).unwrap_or_default() * RECIPE_BOOK_OVERLAY_BUTTON_SIZE;
    let max_bottom_pos = 13 + 166 / 2 + 50;
    if bottom_pos > max_bottom_pos {
        y -= RECIPE_BOOK_OVERLAY_BUTTON_SIZE
            * ceil_div_i32(bottom_pos - max_bottom_pos, RECIPE_BOOK_OVERLAY_BUTTON_SIZE);
    }

    let max_top_pos = 13 + 166 / 2 - 100;
    if y < max_top_pos {
        y -= RECIPE_BOOK_OVERLAY_BUTTON_SIZE
            * ceil_div_i32(y - max_top_pos, RECIPE_BOOK_OVERLAY_BUTTON_SIZE);
    }
    (x, y)
}

pub(crate) fn recipe_book_overlay_entry_position(
    origin_x: i32,
    origin_y: i32,
    entry_index: usize,
    entry_count: usize,
) -> (i32, i32) {
    let max_row = recipe_book_overlay_max_row(entry_count);
    let column = entry_index % max_row;
    let row = entry_index / max_row;
    (
        origin_x + 4 + i32::try_from(column).unwrap_or_default() * RECIPE_BOOK_OVERLAY_BUTTON_SIZE,
        origin_y + 5 + i32::try_from(row).unwrap_or_default() * RECIPE_BOOK_OVERLAY_BUTTON_SIZE,
    )
}

fn ceil_div_i32(value: i32, divisor: i32) -> i32 {
    value.div_euclid(divisor) + i32::from(value.rem_euclid(divisor) != 0)
}

pub(crate) fn crafting_recipe_book_collections<'a>(
    world: &'a WorldStore,
    grid: RecipeBookCraftingGrid,
    selected_tab_index: usize,
    only_craftable: bool,
    search_text: &str,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<RecipeBookUiCollection<'a>> {
    cached_recipe_book_collections(
        world,
        RecipeBookCacheKind::Crafting(grid),
        selected_tab_index,
        only_craftable,
        search_text,
        item_runtime,
    )
}

pub(crate) fn crafting_recipe_book_visible_tab_indices(
    world: &WorldStore,
    grid: RecipeBookCraftingGrid,
    tab_count: usize,
) -> Vec<usize> {
    visible_tab_indices_for_book(world, RecipeBookCacheKind::Crafting(grid), tab_count)
}

pub(crate) fn furnace_recipe_book_collections<'a>(
    world: &'a WorldStore,
    family: RecipeBookFurnaceFamily,
    selected_tab_index: usize,
    only_craftable: bool,
    search_text: &str,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<RecipeBookUiCollection<'a>> {
    cached_recipe_book_collections(
        world,
        RecipeBookCacheKind::Furnace(family),
        selected_tab_index,
        only_craftable,
        search_text,
        item_runtime,
    )
}

pub(crate) fn furnace_recipe_book_visible_tab_indices(
    world: &WorldStore,
    family: RecipeBookFurnaceFamily,
    tab_count: usize,
) -> Vec<usize> {
    visible_tab_indices_for_book(world, RecipeBookCacheKind::Furnace(family), tab_count)
}

pub(crate) fn crafting_recipe_book_tab_has_highlighted_recipe(
    world: &WorldStore,
    grid: RecipeBookCraftingGrid,
    tab_index: usize,
    only_craftable: bool,
) -> bool {
    recipe_book_tab_has_highlighted_recipe(
        world,
        RecipeBookCacheKind::Crafting(grid),
        tab_index,
        only_craftable,
    )
}

pub(crate) fn furnace_recipe_book_tab_has_highlighted_recipe(
    world: &WorldStore,
    family: RecipeBookFurnaceFamily,
    tab_index: usize,
    only_craftable: bool,
) -> bool {
    recipe_book_tab_has_highlighted_recipe(
        world,
        RecipeBookCacheKind::Furnace(family),
        tab_index,
        only_craftable,
    )
}

fn known_recipes_fingerprint(world: &WorldStore) -> u64 {
    let known = &world.recipe_book().known;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    known.len().hash(&mut hasher);
    let hash_entry = |entry: &RecipeDisplayEntry,
                      hasher: &mut std::collections::hash_map::DefaultHasher| {
        entry.id.index.hash(hasher);
        entry.category_id.hash(hasher);
        entry.group.hash(hasher);
        recipe_book_result_stack(entry)
            .and_then(|stack| stack.item_id)
            .hash(hasher);
        if let Some(requirements) = entry.crafting_requirements.as_ref() {
            requirements.len().hash(hasher);
            if let Some(first_req) = requirements.first() {
                first_req.tag.hash(hasher);
                first_req.item_ids.first().hash(hasher);
            }
        }
        (std::ptr::from_ref(entry) as usize).hash(hasher);
    };
    if let Some((_, first)) = known.first_key_value() {
        hash_entry(first, &mut hasher);
    }
    if let Some((_, last)) = known.last_key_value() {
        hash_entry(last, &mut hasher);
    }
    hasher.finish()
}

fn item_tags_fingerprint(world: &WorldStore) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    if let Some(registry) = world.registry_tags("minecraft:item") {
        registry.tags.len().hash(&mut hasher);
        if let Some((key, ids)) = registry.tags.first_key_value() {
            key.hash(&mut hasher);
            ids.len().hash(&mut hasher);
            ids.first().hash(&mut hasher);
            ids.last().hash(&mut hasher);
        }
        if let Some((key, ids)) = registry.tags.last_key_value() {
            key.hash(&mut hasher);
            ids.len().hash(&mut hasher);
            ids.first().hash(&mut hasher);
            ids.last().hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn visible_tab_indices_for_book(
    world: &WorldStore,
    book_kind: RecipeBookCacheKind,
    tab_count: usize,
) -> Vec<usize> {
    if tab_count == 0 {
        return Vec::new();
    }
    let key = RecipeBookVisibleTabsKey {
        world_ptr: std::ptr::from_ref(world) as usize,
        level_generation: world.client_level_generation(),
        recipe_revision: world.recipe_book().revision,
        known_fingerprint: known_recipes_fingerprint(world),
        book_kind,
    };
    let mask = RECIPE_BOOK_UI_CACHE.with(|cache_cell| {
        let mut cache = cache_cell.borrow_mut();
        if cache.visible_tabs_key.as_ref() != Some(&key) {
            let search_categories = vanilla_recipe_book_search_category_ids(book_kind.book_type());
            let category_count = search_categories.len().min(15);
            let mut visible_mask: u16 = 1;
            if category_count > 0 {
                let full_mask: u16 = ((1u32 << (category_count + 1)) - 1) as u16;
                for entry in world.recipe_book().known.values() {
                    if !book_kind.entry_matches_book(entry) {
                        continue;
                    }
                    if let Some(pos) = search_categories[..category_count]
                        .iter()
                        .position(|&category_id| category_id == entry.category_id)
                    {
                        visible_mask |= 1u16 << (pos + 1);
                        if visible_mask == full_mask {
                            break;
                        }
                    }
                }
            }
            cache.visible_tabs_key = Some(key);
            cache.visible_tabs_mask = visible_mask;
            #[cfg(test)]
            {
                cache.visible_tab_scans += 1;
            }
        }
        cache.visible_tabs_mask
    });
    (0..tab_count)
        .filter(|&index| index < 16 && (mask & (1u16 << index)) != 0)
        .collect()
}

fn recipe_book_tab_has_highlighted_recipe(
    world: &WorldStore,
    book_kind: RecipeBookCacheKind,
    tab_index: usize,
    only_craftable: bool,
) -> bool {
    if tab_index == 0 || world.recipe_book().highlights.is_empty() {
        return false;
    }
    let Some(&category_id) =
        vanilla_recipe_book_search_category_ids(book_kind.book_type()).get(tab_index - 1)
    else {
        return false;
    };
    let known = &world.recipe_book().known;
    let item_tag_entries = world
        .registry_tags("minecraft:item")
        .map(|registry| &registry.tags);
    let mut available_items: Option<BTreeMap<i32, i32>> = None;
    for recipe_id in &world.recipe_book().highlights {
        let Some(entry) = known.get(recipe_id) else {
            continue;
        };
        if entry.category_id != category_id || !book_kind.entry_matches_book(entry) {
            continue;
        }
        if !only_craftable {
            return true;
        }
        let counts = available_items.get_or_insert_with(|| book_kind.available_item_counts(world));
        if recipe_book_entry_is_craftable(entry, counts, item_tag_entries) {
            return true;
        }
    }
    false
}

fn cached_recipe_book_collections<'a>(
    world: &'a WorldStore,
    book_kind: RecipeBookCacheKind,
    selected_tab_index: usize,
    only_craftable: bool,
    search_text: &str,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<RecipeBookUiCollection<'a>> {
    let Some(categories) = book_kind.tab_categories(selected_tab_index) else {
        return Vec::new();
    };
    let search_lower = normalized_recipe_search_text(search_text);
    if search_lower.is_some() && item_runtime.is_none() {
        return Vec::new();
    }
    let world_ptr = std::ptr::from_ref(world) as usize;
    let level_generation = world.client_level_generation();
    let recipe_revision = world.recipe_book().revision;
    let known_fingerprint = known_recipes_fingerprint(world);
    let registry_revision = world.registry_access_revision();
    let item_tags_fp = item_tags_fingerprint(world);
    let inventory_fingerprint = book_kind.available_items_fingerprint(world);
    let search_runtime_key = search_lower.as_ref().and_then(|_| {
        item_runtime.map(|runtime| {
            (
                std::ptr::from_ref(runtime) as usize,
                runtime.resource_generation(),
                registry_revision.contents,
            )
        })
    });
    let collections_key = RecipeBookCollectionsKey {
        world_ptr,
        level_generation,
        recipe_revision,
        known_fingerprint,
        tags_revision: registry_revision.tags,
        item_tags_fingerprint: item_tags_fp,
        inventory_fingerprint,
        book_kind,
        selected_tab_index,
        only_craftable,
        search_lower: search_lower.clone().unwrap_or_default(),
        search_runtime_key,
    };
    let known = &world.recipe_book().known;

    RECIPE_BOOK_UI_CACHE.with(|cache_cell| {
        let mut cache = cache_cell.borrow_mut();
        if cache.collections_key.as_ref() != Some(&collections_key) {
            let available_items = book_kind.available_item_counts(world);
            let item_tag_entries = world
                .registry_tags("minecraft:item")
                .map(|registry| &registry.tags);
            let mut collections = Vec::new();
            for &category_id in categories {
                push_cached_category_collections(
                    world,
                    book_kind,
                    category_id,
                    only_craftable,
                    &available_items,
                    item_tag_entries,
                    &mut collections,
                );
            }
            if let (Some(search_text), Some(item_runtime)) = (search_lower.as_deref(), item_runtime)
            {
                let search_index_key = RecipeBookSearchIndexKey {
                    world_ptr,
                    level_generation,
                    recipe_revision,
                    known_fingerprint,
                    item_runtime_ptr: std::ptr::from_ref(item_runtime) as usize,
                    resource_generation: item_runtime.resource_generation(),
                    registry_contents_revision: registry_revision.contents,
                };
                if cache.search_index_key.as_ref() != Some(&search_index_key) {
                    cache.search_index.clear();
                    cache.search_index_key = Some(search_index_key);
                }
                let RecipeBookUiCache {
                    search_index,
                    #[cfg(test)]
                    search_entry_builds,
                    ..
                } = &mut *cache;
                collections.retain(|collection| {
                    cached_collection_matches_search(
                        world,
                        &collection.entry_ids,
                        search_text,
                        item_runtime,
                        search_index,
                        #[cfg(test)]
                        search_entry_builds,
                    )
                });
            }
            cache.collections = collections;
            cache.collections_key = Some(collections_key);
            #[cfg(test)]
            {
                cache.collection_builds += 1;
            }
        }

        cache
            .collections
            .iter()
            .map(|cached| RecipeBookUiCollection {
                entries: cached
                    .entry_ids
                    .iter()
                    .filter_map(|id| known.get(id))
                    .collect(),
                craftable_entries: cached.craftable_entries.clone(),
                has_craftable: cached.has_craftable,
            })
            .collect()
    })
}

pub(crate) fn recipe_book_page_count(collection_count: usize) -> usize {
    collection_count.div_ceil(RECIPE_BOOK_ITEMS_PER_PAGE)
}

pub(crate) fn clamped_recipe_book_page(page: usize, collection_count: usize) -> usize {
    let page_count = recipe_book_page_count(collection_count);
    page.min(page_count.saturating_sub(1))
}

fn recipe_book_overlay_items_for_entry<'a>(
    entry: &'a RecipeDisplayEntry,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    slot_select_index: usize,
) -> Vec<RecipeBookOverlayItem<'a>> {
    let mut items = Vec::new();
    if let Some(crafting) = entry.display.crafting.as_deref() {
        match crafting {
            CraftingRecipeDisplaySummary::Shapeless { ingredients, .. } => {
                for (index, ingredient) in ingredients.iter().enumerate() {
                    push_recipe_book_overlay_item(
                        ingredient,
                        i32::try_from(index % 3).unwrap_or_default(),
                        i32::try_from(index / 3).unwrap_or_default(),
                        item_tag_entries,
                        slot_select_index,
                        &mut items,
                    );
                }
            }
            CraftingRecipeDisplaySummary::Shaped {
                width,
                height,
                ingredients,
                ..
            } => {
                for_each_shaped_recipe_grid_slot(
                    3,
                    3,
                    *width,
                    *height,
                    ingredients,
                    |ingredient, _grid_index, grid_x, grid_y| {
                        push_recipe_book_overlay_item(
                            ingredient,
                            grid_x,
                            grid_y,
                            item_tag_entries,
                            slot_select_index,
                            &mut items,
                        );
                    },
                );
            }
        }
    } else if let Some(furnace) = entry.display.furnace.as_ref() {
        push_recipe_book_overlay_item(
            &furnace.ingredient,
            1,
            1,
            item_tag_entries,
            slot_select_index,
            &mut items,
        );
    }
    items
}

fn push_recipe_book_overlay_item<'a>(
    display: &'a SlotDisplaySummary,
    grid_x: i32,
    grid_y: i32,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    slot_select_index: usize,
    items: &mut Vec<RecipeBookOverlayItem<'a>>,
) {
    let Some(stack) =
        slot_display_item_stack_at_index(display, item_tag_entries, slot_select_index)
    else {
        return;
    };
    items.push(RecipeBookOverlayItem {
        x: RECIPE_BOOK_OVERLAY_GRID_ITEM_OFFSET + grid_x * RECIPE_BOOK_OVERLAY_GRID_ITEM_STRIDE,
        y: RECIPE_BOOK_OVERLAY_GRID_ITEM_OFFSET + grid_y * RECIPE_BOOK_OVERLAY_GRID_ITEM_STRIDE,
        stack,
    });
}

pub(crate) fn recipe_book_crafting_result_stack(
    entry: &RecipeDisplayEntry,
) -> Option<&ItemStackSummary> {
    match entry.display.crafting.as_deref()? {
        CraftingRecipeDisplaySummary::Shapeless { result, .. }
        | CraftingRecipeDisplaySummary::Shaped { result, .. } => result.item_stack.as_ref(),
    }
}

fn recipe_book_result_stack(entry: &RecipeDisplayEntry) -> Option<&ItemStackSummary> {
    recipe_book_crafting_result_stack(entry).or_else(|| recipe_book_furnace_result_stack(entry))
}

fn recipe_book_furnace_result_stack(entry: &RecipeDisplayEntry) -> Option<&ItemStackSummary> {
    entry.display.furnace.as_ref()?.result.item_stack.as_ref()
}

fn slot_display_item_stack_at_index<'a>(
    display: &'a SlotDisplaySummary,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    slot_select_index: usize,
) -> Option<Cow<'a, ItemStackSummary>> {
    if let Some(stack) = display.item_stack.as_ref() {
        return Some(Cow::Borrowed(stack));
    }
    let mut stacks = Vec::new();
    push_slot_display_item_stack_candidates(display, item_tag_entries, &mut stacks);
    if stacks.is_empty() {
        return None;
    }
    Some(Cow::Owned(stacks[slot_select_index % stacks.len()].clone()))
}

fn push_slot_display_item_stack_candidates(
    display: &SlotDisplaySummary,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    stacks: &mut Vec<ItemStackSummary>,
) {
    if let Some(stack) = display.item_stack.as_ref() {
        stacks.push(stack.clone());
        return;
    }
    if let Some(entries) = display
        .tag
        .as_ref()
        .and_then(|tag| item_tag_entries.and_then(|tags| tags.get(tag)))
    {
        if !entries.is_empty() {
            stacks.extend(entries.iter().map(|item_id| ItemStackSummary {
                item_id: Some(*item_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            }));
            return;
        }
    }
    if let Ok(value) = display.decode_value() {
        push_slot_display_value_candidates(&value, item_tag_entries, stacks);
    }
}

fn push_slot_display_value_candidates(
    display: &SlotDisplayValue,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    stacks: &mut Vec<ItemStackSummary>,
) {
    match display {
        SlotDisplayValue::Item { stack } | SlotDisplayValue::ItemStack { stack } => {
            stacks.push((**stack).clone());
        }
        SlotDisplayValue::Tag { tag } => {
            if let Some(entries) = item_tag_entries.and_then(|tags| tags.get(tag)) {
                stacks.extend(entries.iter().map(|item_id| ItemStackSummary {
                    item_id: Some(*item_id),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default().into(),
                }));
            }
        }
        SlotDisplayValue::WithRemainder { input, .. } => {
            push_slot_display_value_candidates(input, item_tag_entries, stacks);
        }
        SlotDisplayValue::Composite { contents } => {
            for content in contents {
                push_slot_display_value_candidates(content, item_tag_entries, stacks);
            }
        }
        SlotDisplayValue::Empty
        | SlotDisplayValue::AnyFuel
        | SlotDisplayValue::WithAnyPotion { .. }
        | SlotDisplayValue::OnlyWithComponent { .. }
        | SlotDisplayValue::Dyed { .. }
        | SlotDisplayValue::SmithingTrim { .. } => {}
    }
}

fn normalized_recipe_search_text(search_text: &str) -> Option<String> {
    (!search_text.is_empty()).then(|| search_text.to_lowercase())
}

fn build_cached_recipe_search_text(
    world: &WorldStore,
    entry: &RecipeDisplayEntry,
    item_runtime: &NativeItemRuntime,
) -> CachedRecipeSearchText {
    let Some(stack) = recipe_book_result_stack(entry) else {
        return CachedRecipeSearchText {
            namespace_lower: None,
            path_lower: None,
            tooltip_lower: Vec::new(),
        };
    };
    let (namespace_lower, path_lower) = stack
        .item_id
        .and_then(|item_id| item_runtime.item_resource_id(item_id))
        .map_or((None, None), |resource_id| {
            let (resource_namespace, resource_path) =
                resource_id.split_once(':').unwrap_or(("", resource_id));
            (
                Some(resource_namespace.to_lowercase()),
                Some(resource_path.to_lowercase()),
            )
        });
    let effective = world.effective_item_stack(stack);
    let tooltip_lower = tooltip_lines_for_stack_with_context(
        item_runtime,
        effective.as_ref(),
        NativeItemTooltipOptions {
            item_profiles: Some(world.item_profiles()),
            ..NativeItemTooltipOptions::default()
        },
    )
    .map(|lines| {
        lines
            .into_iter()
            .map(|line| line.text.to_lowercase())
            .collect()
    })
    .unwrap_or_default();
    CachedRecipeSearchText {
        namespace_lower,
        path_lower,
        tooltip_lower,
    }
}

fn ensure_cached_recipe_search_text<'a>(
    world: &WorldStore,
    entry_id: i32,
    item_runtime: &NativeItemRuntime,
    search_index: &'a mut FxHashMap<i32, CachedRecipeSearchText>,
    #[cfg(test)] search_entry_builds: &mut usize,
) -> Option<&'a CachedRecipeSearchText> {
    match search_index.entry(entry_id) {
        std::collections::hash_map::Entry::Occupied(entry) => Some(entry.into_mut()),
        std::collections::hash_map::Entry::Vacant(slot) => {
            let entry = world.recipe_book().known.get(&entry_id)?;
            let cached = build_cached_recipe_search_text(world, entry, item_runtime);
            #[cfg(test)]
            {
                *search_entry_builds += 1;
            }
            Some(slot.insert(cached))
        }
    }
}

fn cached_collection_matches_search(
    world: &WorldStore,
    entry_ids: &[i32],
    search_text: &str,
    item_runtime: &NativeItemRuntime,
    search_index: &mut FxHashMap<i32, CachedRecipeSearchText>,
    #[cfg(test)] search_entry_builds: &mut usize,
) -> bool {
    if let Some((namespace, path)) = search_text.split_once(':') {
        let namespace = namespace.trim();
        let path = path.trim();
        let mut namespace_matches = false;
        let mut path_or_name_matches = false;
        for &entry_id in entry_ids {
            let Some(cached) = ensure_cached_recipe_search_text(
                world,
                entry_id,
                item_runtime,
                search_index,
                #[cfg(test)]
                search_entry_builds,
            ) else {
                continue;
            };
            if !namespace_matches
                && cached
                    .namespace_lower
                    .as_deref()
                    .is_some_and(|ns| ns.contains(namespace))
            {
                namespace_matches = true;
            }
            if !path_or_name_matches
                && (cached
                    .path_lower
                    .as_deref()
                    .is_some_and(|p| p.contains(path))
                    || cached.tooltip_lower.iter().any(|line| line.contains(path)))
            {
                path_or_name_matches = true;
            }
            if namespace_matches && path_or_name_matches {
                return true;
            }
        }
        return false;
    }
    for &entry_id in entry_ids {
        let Some(cached) = ensure_cached_recipe_search_text(
            world,
            entry_id,
            item_runtime,
            search_index,
            #[cfg(test)]
            search_entry_builds,
        ) else {
            continue;
        };
        if cached
            .tooltip_lower
            .iter()
            .any(|line| line.contains(search_text))
        {
            return true;
        }
    }
    false
}

fn push_cached_category_collections(
    world: &WorldStore,
    book_kind: RecipeBookCacheKind,
    category_id: i32,
    only_craftable: bool,
    available_items: &BTreeMap<i32, i32>,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    collections: &mut Vec<CachedRecipeBookCollection>,
) {
    let mut group_indexes: FxHashMap<i32, usize> = FxHashMap::default();
    for entry in world.recipe_book().known.values() {
        if entry.category_id != category_id || !book_kind.entry_matches_book(entry) {
            continue;
        }
        let craftable = recipe_book_entry_is_craftable(entry, available_items, item_tag_entries);
        if only_craftable && !craftable {
            continue;
        }
        if let Some(group_id) = entry.group {
            if let Some(index) = group_indexes.get(&group_id).copied() {
                collections[index].entry_ids.push(entry.id.index);
                collections[index].craftable_entries.push(craftable);
                collections[index].has_craftable |= craftable;
            } else {
                let index = collections.len();
                group_indexes.insert(group_id, index);
                collections.push(CachedRecipeBookCollection {
                    entry_ids: vec![entry.id.index],
                    craftable_entries: vec![craftable],
                    has_craftable: craftable,
                });
            }
        } else {
            collections.push(CachedRecipeBookCollection {
                entry_ids: vec![entry.id.index],
                craftable_entries: vec![craftable],
                has_craftable: craftable,
            });
        }
    }
}

fn recipe_book_tab_categories(
    book_type: RecipeBookType,
    selected_tab_index: usize,
) -> Option<&'static [i32]> {
    let search_categories = vanilla_recipe_book_search_category_ids(book_type);
    if selected_tab_index == 0 {
        return Some(search_categories);
    }
    search_categories
        .get(selected_tab_index - 1)
        .map(std::slice::from_ref)
}

fn crafting_recipe_fits_grid(entry: &RecipeDisplayEntry, grid: RecipeBookCraftingGrid) -> bool {
    match entry.display.crafting.as_deref() {
        Some(CraftingRecipeDisplaySummary::Shapeless { ingredients, .. }) => {
            i32::try_from(ingredients.len()).is_ok_and(|count| count <= grid.width * grid.height)
        }
        Some(CraftingRecipeDisplaySummary::Shaped { width, height, .. }) => {
            *width > 0 && *height > 0 && *width <= grid.width && *height <= grid.height
        }
        None => false,
    }
}

fn crafting_recipe_book_available_item_counts(
    world: &WorldStore,
    grid: RecipeBookCraftingGrid,
) -> BTreeMap<i32, i32> {
    let mut counts = player_inventory_item_counts(world);

    if world.local_inventory_is_open() && grid.width == 2 && grid.height == 2 {
        let container = &world.inventory().inventory_menu;
        add_container_slot_range_counts(
            world,
            &mut counts,
            container,
            LOCAL_INVENTORY_CRAFT_SLOT_START,
            LOCAL_INVENTORY_CRAFT_SLOT_END,
        );
        return counts;
    }

    if grid.width == 3 && grid.height == 3 {
        let Some(container) = world.inventory().open_container.as_ref() else {
            return counts;
        };
        add_container_slot_range_counts(
            world,
            &mut counts,
            container,
            CRAFTING_TABLE_CRAFT_SLOT_START,
            CRAFTING_TABLE_CRAFT_SLOT_END,
        );
    }

    counts
}

fn furnace_recipe_book_available_item_counts(world: &WorldStore) -> BTreeMap<i32, i32> {
    let mut counts = player_inventory_item_counts(world);

    let Some(container) = world.inventory().open_container.as_ref() else {
        return counts;
    };
    add_container_slot_range_counts(
        world,
        &mut counts,
        container,
        FURNACE_CRAFT_SLOT_START,
        FURNACE_CRAFT_SLOT_END,
    );
    counts
}

fn player_inventory_item_counts(world: &WorldStore) -> BTreeMap<i32, i32> {
    let mut counts = BTreeMap::new();
    for slot in &world.inventory().player_slots {
        if (PLAYER_INVENTORY_SLOT_START..PLAYER_INVENTORY_SLOT_END).contains(&slot.slot) {
            add_item_stack_count(&mut counts, &slot.item);
        }
    }
    counts
}

fn add_container_slot_range_counts(
    world: &WorldStore,
    counts: &mut BTreeMap<i32, i32>,
    container: &bbb_client_world::ContainerState,
    start: i16,
    end: i16,
) {
    for slot in start..end {
        if let Some(row) = world.inventory().container_slot(container, slot) {
            add_item_stack_count(counts, row.item);
        }
    }
}

fn hash_container_slot_range(
    world: &WorldStore,
    container: &bbb_client_world::ContainerState,
    start: i16,
    end: i16,
    hasher: &mut std::collections::hash_map::DefaultHasher,
) {
    for slot in start..end {
        if let Some(row) = world.inventory().container_slot(container, slot) {
            if let (Some(item_id), count) = (row.item.item_id, row.item.count) {
                if count > 0 {
                    slot.hash(hasher);
                    item_id.hash(hasher);
                    count.hash(hasher);
                }
            }
        }
    }
}

fn add_item_stack_count(counts: &mut BTreeMap<i32, i32>, stack: &ItemStackSummary) {
    let (Some(item_id), count) = (stack.item_id, stack.count) else {
        return;
    };
    if count <= 0 {
        return;
    }
    let entry = counts.entry(item_id).or_default();
    *entry = entry.saturating_add(count);
}

fn recipe_book_entry_is_craftable(
    entry: &RecipeDisplayEntry,
    available_items: &BTreeMap<i32, i32>,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
) -> bool {
    let Some(requirements) = entry.crafting_requirements.as_ref() else {
        return false;
    };
    if requirements.is_empty() {
        return true;
    }
    if available_items.is_empty() {
        return false;
    }
    let mut item_index: bbb_client_world::FxHashMap<i32, usize> =
        bbb_client_world::FxHashMap::default();
    let mut right_caps: Vec<usize> = Vec::new();
    let mut options: Vec<Vec<usize>> = Vec::with_capacity(requirements.len());
    let mut total_available: usize = 0;
    for requirement in requirements {
        let raw_candidates: &[i32] = if let Some(tag) = requirement.tag.as_ref() {
            let Some(ids) = item_tag_entries.and_then(|tags| tags.get(tag)) else {
                return false;
            };
            ids.as_slice()
        } else {
            requirement.item_ids.as_slice()
        };
        let mut matching = Vec::new();
        for &item_id in raw_candidates {
            let count = available_items.get(&item_id).copied().unwrap_or_default();
            if count <= 0 {
                continue;
            }
            let right_id = *item_index.entry(item_id).or_insert_with(|| {
                let idx = right_caps.len();
                let capped = (count as usize).min(requirements.len());
                total_available = total_available.saturating_add(capped);
                right_caps.push(capped);
                idx
            });
            matching.push(right_id);
        }
        if matching.is_empty() {
            return false;
        }
        if matching.len() > 1 {
            matching.sort_unstable();
            matching.dedup();
        }
        options.push(matching);
    }
    if options.len() == 1 {
        return true;
    }
    if total_available < options.len() {
        return false;
    }
    options.sort_by_key(Vec::len);
    recipe_book_can_satisfy_ingredients(&options, &right_caps)
}

// Iterative bipartite augmenting-path matching matching Vanilla `StackedContents.RecipePicker`
// (`StackedContents.java:116-256`) in polynomial time with zero call-stack recursion.
fn recipe_book_can_satisfy_ingredients(options: &[Vec<usize>], right_caps: &[usize]) -> bool {
    let left_count = options.len();
    let right_count = right_caps.len();
    let mut matched_right_of_left: Vec<Option<usize>> = vec![None; left_count];
    let mut matched_lefts_of_right: Vec<Vec<usize>> = vec![Vec::new(); right_count];

    // Greedy pre-pass handles the common case in O(E).
    let mut unmatched_lefts: Vec<usize> = Vec::new();
    for (left, adj) in options.iter().enumerate() {
        if let Some(&right) = adj
            .iter()
            .find(|&&r| matched_lefts_of_right[r].len() < right_caps[r])
        {
            matched_right_of_left[left] = Some(right);
            matched_lefts_of_right[right].push(left);
        } else {
            unmatched_lefts.push(left);
        }
    }
    if unmatched_lefts.is_empty() {
        return true;
    }

    let mut visited_right: Vec<usize> = vec![0; right_count];
    let mut parent_right: Vec<usize> = vec![0; right_count];
    let mut parent_left: Vec<usize> = vec![0; left_count];
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();

    for (stamp_idx, &start_left) in unmatched_lefts.iter().enumerate() {
        let stamp = stamp_idx + 1;
        queue.clear();
        queue.push_back(start_left);
        let mut found_free_right: Option<usize> = None;

        'bfs: while let Some(left) = queue.pop_front() {
            for &right in &options[left] {
                if visited_right[right] == stamp {
                    continue;
                }
                visited_right[right] = stamp;
                parent_right[right] = left;
                if matched_lefts_of_right[right].len() < right_caps[right] {
                    found_free_right = Some(right);
                    break 'bfs;
                }
                for &next_left in &matched_lefts_of_right[right] {
                    parent_left[next_left] = right;
                    queue.push_back(next_left);
                }
            }
        }

        let Some(mut cur_right) = found_free_right else {
            return false;
        };
        loop {
            let cur_left = parent_right[cur_right];
            let prev_right = matched_right_of_left[cur_left];
            matched_right_of_left[cur_left] = Some(cur_right);
            matched_lefts_of_right[cur_right].push(cur_left);
            if cur_left == start_left {
                break;
            }
            let old_right = prev_right.expect("alternating path left node must have prior match");
            if let Some(pos) = matched_lefts_of_right[old_right]
                .iter()
                .position(|&l| l == cur_left)
            {
                matched_lefts_of_right[old_right].swap_remove(pos);
            }
            cur_right = parent_left[cur_left];
            debug_assert_eq!(cur_right, old_right);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        IngredientSummary, RecipeBookAdd, RecipeBookAddEntry, RecipeDisplayId,
        RecipeDisplaySummary, RecipeDisplayType, SetPlayerInventory,
    };
    use bbb_protocol::recipe_wire_types::{
        VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_BUILDING_BLOCKS_ID,
        VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID, VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
    };

    fn test_crafting_entry(
        id: i32,
        category_id: i32,
        result_item_id: i32,
        ingredient_item_id: i32,
    ) -> RecipeBookAddEntry {
        let stack = ItemStackSummary {
            item_id: Some(result_item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let display = SlotDisplaySummary {
            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
            item_stack: Some(stack),
            tag: None,
            raw_payload: Vec::new(),
        };
        RecipeBookAddEntry::new(
            RecipeDisplayEntry {
                id: RecipeDisplayId { index: id },
                display: RecipeDisplaySummary {
                    display_type: RecipeDisplayType::CraftingShapeless,
                    raw_body: Vec::new(),
                    crafting: Some(Box::new(CraftingRecipeDisplaySummary::Shapeless {
                        ingredients: vec![display.clone()],
                        result: display.clone(),
                        crafting_station: display,
                    })),
                    furnace: None,
                    stonecutter: None,
                    smithing: None,
                },
                group: None,
                category_id,
                crafting_requirements: Some(vec![IngredientSummary {
                    tag: None,
                    item_ids: vec![ingredient_item_id],
                }]),
            },
            false,
            true,
        )
    }

    #[test]
    fn recipe_book_cache_reuses_collections_and_visible_tabs_and_invalidates_on_changes() {
        let mut world = WorldStore::new();
        world.apply_recipe_book_add(RecipeBookAdd {
            replace: true,
            entries: vec![test_crafting_entry(
                10,
                VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID,
                bbb_protocol::item_types::VANILLA_ITEM_STICK_ID,
                bbb_protocol::item_types::VANILLA_ITEM_OAK_PLANKS_ID,
            )],
        });
        let grid = RecipeBookCraftingGrid {
            width: 2,
            height: 2,
        };

        let tab_scans_before = RECIPE_BOOK_UI_CACHE.with(|cell| cell.borrow().visible_tab_scans);
        let tabs_a = crafting_recipe_book_visible_tab_indices(&world, grid, 5);
        let tabs_b = crafting_recipe_book_visible_tab_indices(&world, grid, 5);
        let tab_scans_after = RECIPE_BOOK_UI_CACHE.with(|cell| cell.borrow().visible_tab_scans);
        assert_eq!(tabs_a, vec![0, 3]);
        assert_eq!(tabs_b, vec![0, 3]);
        assert_eq!(tab_scans_after, tab_scans_before + 1);

        // Highlighted tab fast path respects craftability filter.
        assert!(crafting_recipe_book_tab_has_highlighted_recipe(
            &world, grid, 3, false
        ));
        assert!(!crafting_recipe_book_tab_has_highlighted_recipe(
            &world, grid, 3, true
        ));

        let builds_before = RECIPE_BOOK_UI_CACHE.with(|cell| cell.borrow().collection_builds);
        let first = crafting_recipe_book_collections(&world, grid, 0, false, "", None);
        let second = crafting_recipe_book_collections(&world, grid, 0, false, "", None);
        let builds_after_hit = RECIPE_BOOK_UI_CACHE.with(|cell| cell.borrow().collection_builds);
        assert_eq!(first.len(), 1);
        assert!(!first[0].has_craftable());
        assert_eq!(second.len(), 1);
        assert_eq!(builds_after_hit, builds_before + 1);

        // Adding the required ingredient to inventory invalidates the collections cache.
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_OAK_PLANKS_ID),
                count: 2,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        });
        let after_inv = crafting_recipe_book_collections(&world, grid, 0, false, "", None);
        let builds_after_inv = RECIPE_BOOK_UI_CACHE.with(|cell| cell.borrow().collection_builds);
        assert_eq!(after_inv.len(), 1);
        assert!(after_inv[0].has_craftable());
        assert_eq!(builds_after_inv, builds_after_hit + 1);
        assert!(crafting_recipe_book_tab_has_highlighted_recipe(
            &world, grid, 3, true
        ));

        // Adding a recipe in another category invalidates both visible tabs and collections.
        world.apply_recipe_book_add(RecipeBookAdd {
            replace: false,
            entries: vec![test_crafting_entry(
                11,
                VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_BUILDING_BLOCKS_ID,
                bbb_protocol::item_types::VANILLA_ITEM_STONE_ID,
                bbb_protocol::item_types::VANILLA_ITEM_COBBLESTONE_ID,
            )],
        });
        let tabs_after_add = crafting_recipe_book_visible_tab_indices(&world, grid, 5);
        assert_eq!(tabs_after_add, vec![0, 2, 3]);
        let after_add = crafting_recipe_book_collections(&world, grid, 0, false, "", None);
        assert_eq!(after_add.len(), 2);
    }

    #[test]
    fn recipe_book_bipartite_matching_handles_adversarial_and_deep_ingredients_without_backtracking(
    ) {
        // 51 ingredients accepting {0, 1} + 1 accepting {2}, with caps [25, 25, 10]: unsatisfiable
        // (would explore C(50, 25) ~ 1.26e14 states with naive DFS backtracking).
        let mut options = vec![vec![0, 1]; 51];
        options.push(vec![2]);
        assert!(!recipe_book_can_satisfy_ingredients(
            &options,
            &[25, 25, 10]
        ));

        // With caps [25, 26, 1] the exact same 52 ingredients are satisfiable.
        assert!(recipe_book_can_satisfy_ingredients(&options, &[25, 26, 1]));

        // Requires an augmenting-path rematch: left 0 takes right 0 greedily, left 1 needs right 0
        // so left 0 must move to right 1.
        let rematch = vec![vec![0, 1], vec![0]];
        assert!(recipe_book_can_satisfy_ingredients(&rematch, &[1, 1]));

        // 4,096 ingredients must succeed iteratively without call-stack recursion.
        let deep = vec![vec![0]; 4_096];
        assert!(recipe_book_can_satisfy_ingredients(&deep, &[4_096]));

        // `add_item_stack_count` saturates instead of overflowing `i32`.
        let mut counts = BTreeMap::new();
        let stack = ItemStackSummary {
            item_id: Some(1),
            count: i32::MAX,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        add_item_stack_count(&mut counts, &stack);
        add_item_stack_count(&mut counts, &stack);
        assert_eq!(counts.get(&1), Some(&i32::MAX));
    }
}
