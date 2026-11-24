use super::*;

pub(super) fn maybe_scroll_stonecutter_recipes(
    input: &mut ClientInputState,
    world: &WorldStore,
    delta: &MouseScrollDelta,
) -> bool {
    sync_stonecutter_recipe_scroll(input, world);
    let Some(max_scroll_row) = stonecutter_recipe_max_scroll_row(world) else {
        return false;
    };
    if max_scroll_row <= 0 {
        return false;
    }
    if let Some(wheel) = inventory_wheel_steps_from_scroll(input, delta) {
        input.container_mut().stonecutter_recipe_scroll_row =
            (input.container_mut().stonecutter_recipe_scroll_row - wheel).clamp(0, max_scroll_row);
    }
    true
}

pub(super) fn maybe_scroll_loom_patterns(
    input: &mut ClientInputState,
    world: &WorldStore,
    delta: &MouseScrollDelta,
) -> bool {
    sync_loom_pattern_scroll(input, world);
    let Some(max_scroll_row) = loom_pattern_max_scroll_row(world) else {
        return false;
    };
    if max_scroll_row <= 0 {
        return false;
    }
    if let Some(wheel) = inventory_wheel_steps_from_scroll(input, delta) {
        input.container_mut().loom_pattern_scroll_row =
            (input.container_mut().loom_pattern_scroll_row - wheel).clamp(0, max_scroll_row);
    }
    true
}

pub(super) fn maybe_start_stonecutter_recipe_scroll_drag(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_stonecutter_recipe_scroll(input, world);
    if stonecutter_recipe_max_scroll_row(world).unwrap_or_default() <= 0 {
        return false;
    }
    if !stonecutter_scroller_at_position(world, cursor_position, surface_size) {
        return false;
    }
    input.container_mut().stonecutter_recipe_scrolling = true;
    true
}

pub(super) fn maybe_start_loom_pattern_scroll_drag(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_loom_pattern_scroll(input, world);
    if loom_pattern_max_scroll_row(world).unwrap_or_default() <= 0 {
        return false;
    }
    if !loom_scroller_at_position(world, cursor_position, surface_size) {
        return false;
    }
    input.container_mut().loom_pattern_scrolling = true;
    true
}

pub(super) fn maybe_start_merchant_trade_scroll_drag(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if !merchant_offers_can_scroll(world) {
        return false;
    }
    if !merchant_scroller_track_at_position(world, cursor_position, surface_size) {
        return false;
    }
    input.container_mut().merchant_trade_scrolling = true;
    true
}

pub(super) fn maybe_scroll_merchant_trades(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    delta: &MouseScrollDelta,
) -> bool {
    if !matches!(
        inventory_screen_layout(world).map(|layout| layout.background),
        Some(InventoryScreenBackground::Merchant)
    ) {
        return false;
    }
    if !merchant_offers_can_scroll(world) {
        return false;
    }
    if let Some(wheel) = inventory_wheel_steps_from_scroll(input, delta) {
        world.scroll_local_merchant_offers(-wheel);
    }
    true
}

pub(super) fn merchant_offers_can_scroll(world: &WorldStore) -> bool {
    world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .is_some_and(|offers| offers.offers.len() > MERCHANT_TRADE_BUTTON_COUNT as usize)
}

pub(super) fn merchant_offer_count_and_scroll_offset(world: &WorldStore) -> Option<(usize, i32)> {
    world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .map(|offers| (offers.offers.len(), offers.local_scroll_offset))
}

pub(super) fn merchant_max_scroll_offset(offer_count: usize) -> i32 {
    i32::try_from(offer_count)
        .unwrap_or_default()
        .saturating_sub(MERCHANT_TRADE_BUTTON_COUNT)
        .max(0)
}

pub(super) fn inventory_wheel_steps_from_scroll(
    input: &mut ClientInputState,
    delta: &MouseScrollDelta,
) -> Option<i32> {
    let (x, y) = crate::input::scroll_delta_lines(*delta);

    if input.scroll_accumulated_x != 0.0
        && scroll_signum(x) != scroll_signum(input.scroll_accumulated_x)
    {
        input.scroll_accumulated_x = 0.0;
    }
    if input.scroll_accumulated_y != 0.0
        && scroll_signum(y) != scroll_signum(input.scroll_accumulated_y)
    {
        input.scroll_accumulated_y = 0.0;
    }

    input.scroll_accumulated_x += x;
    input.scroll_accumulated_y += y;
    let wheel_x = input.scroll_accumulated_x as i32;
    let wheel_y = input.scroll_accumulated_y as i32;
    if wheel_x == 0 && wheel_y == 0 {
        return None;
    }

    input.scroll_accumulated_x -= f64::from(wheel_x);
    input.scroll_accumulated_y -= f64::from(wheel_y);
    Some(if wheel_y == 0 { -wheel_x } else { wheel_y })
}

pub(super) fn scroll_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

pub(super) fn inventory_screen_hovered_slot(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<i16> {
    match inventory_screen_click_target(world, cursor_position, surface_size) {
        Some(InventoryClickTarget::Slot(slot)) => Some(slot),
        _ => None,
    }
}

pub(super) fn inventory_screen_cursor_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<(i32, i32)> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    let cursor = cursor_position?;
    if layout.background == InventoryScreenBackground::Lectern {
        return (cursor.x.is_finite() && cursor.y.is_finite())
            .then(|| (cursor.x.floor() as i32, cursor.y.floor() as i32));
    }
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    (x.is_finite() && y.is_finite()).then(|| (x.floor() as i32, y.floor() as i32))
}

pub(crate) fn recipe_book_button_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<RecipeBookType> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    let (button_x, button_y) = recipe_book_button_position(layout.background)?;
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let main_offset_x = layout
        .recipe_book
        .map(|recipe_book| recipe_book.main_gui_x_offset)
        .unwrap_or_default();
    let x = cursor.x - origin_x - f64::from(main_offset_x + button_x);
    let y = cursor.y - origin_y - f64::from(button_y);
    if x >= 0.0
        && x < f64::from(RECIPE_BOOK_BUTTON_WIDTH)
        && y >= 0.0
        && y < f64::from(RECIPE_BOOK_BUTTON_HEIGHT)
    {
        return recipe_book_type_for_background(layout.background);
    }
    None
}

pub(super) fn recipe_book_filter_button_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<RecipeBookType> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    let book_type = recipe_book_type_for_background(layout.background)?;
    let recipe_book = layout.recipe_book?;
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x - f64::from(recipe_book.x + RECIPE_BOOK_FILTER_BUTTON_X);
    let y = cursor.y - origin_y - f64::from(RECIPE_BOOK_FILTER_BUTTON_Y);
    if x >= 0.0
        && x < f64::from(RECIPE_BOOK_FILTER_BUTTON_WIDTH)
        && y >= 0.0
        && y < f64::from(RECIPE_BOOK_FILTER_BUTTON_HEIGHT)
    {
        return Some(book_type);
    }
    None
}

pub(super) fn recipe_book_search_box_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(layout) = inventory_screen_layout_for_surface(world, surface_size) else {
        return false;
    };
    if recipe_book_type_for_background(layout.background).is_none() {
        return false;
    }
    let Some(recipe_book) = layout.recipe_book else {
        return false;
    };
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x - f64::from(recipe_book.x + RECIPE_BOOK_SEARCH_BOX_X);
    let y = cursor.y - origin_y - f64::from(RECIPE_BOOK_SEARCH_BOX_Y);
    x >= 0.0
        && x < f64::from(RECIPE_BOOK_SEARCH_BOX_WIDTH)
        && y >= 0.0
        && y < f64::from(RECIPE_BOOK_SEARCH_BOX_HEIGHT)
}

pub(super) fn recipe_book_tab_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<(RecipeBookType, usize)> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    let book_type = recipe_book_type_for_background(layout.background)?;
    let recipe_book = layout.recipe_book?;
    let visible_tabs = recipe_book_visible_tab_indices(world, layout.background);
    if visible_tabs.is_empty() {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x - f64::from(recipe_book.x);
    let y = cursor.y - origin_y;
    for (visible_index, tab_index) in visible_tabs.into_iter().enumerate() {
        let tab_y = RECIPE_BOOK_TAB_Y + RECIPE_BOOK_TAB_STRIDE_Y * visible_index as i32;
        if x >= f64::from(RECIPE_BOOK_TAB_X)
            && x < f64::from(RECIPE_BOOK_TAB_X + RECIPE_BOOK_TAB_WIDTH)
            && y >= f64::from(tab_y)
            && y < f64::from(tab_y + RECIPE_BOOK_TAB_HEIGHT)
        {
            return Some((book_type, tab_index));
        }
    }
    None
}

pub(super) fn recipe_book_page_button_at_position(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<(RecipeBookType, RecipeBookPageTurn, usize)> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    let book_type = recipe_book_type_for_background(layout.background)?;
    let recipe_book = layout.recipe_book?;
    let collection_count = recipe_book_collections_for_background(
        input,
        world,
        item_runtime,
        layout.background,
        book_type,
    )?
    .len();
    let page_count = recipe_book_page_count(collection_count);
    if page_count <= 1 {
        return None;
    }
    let current_page = clamped_recipe_book_page(
        selected_recipe_book_page_index(input, book_type),
        collection_count,
    );
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x - f64::from(recipe_book.x);
    let y = cursor.y - origin_y;
    if current_page + 1 < page_count
        && recipe_book_page_button_contains(
            x,
            y,
            RECIPE_BOOK_PAGE_FORWARD_BUTTON_X,
            RECIPE_BOOK_PAGE_BUTTON_Y,
        )
    {
        return Some((book_type, RecipeBookPageTurn::Next, page_count));
    }
    if current_page > 0
        && recipe_book_page_button_contains(
            x,
            y,
            RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X,
            RECIPE_BOOK_PAGE_BUTTON_Y,
        )
    {
        return Some((book_type, RecipeBookPageTurn::Previous, page_count));
    }
    None
}

pub(super) fn recipe_book_recipe_button_at_position(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<(i32, bool)> {
    let (_, _, _, collection) = recipe_book_collection_at_position(
        input,
        world,
        item_runtime,
        cursor_position,
        surface_size,
    )?;
    let slot_select_index = recipe_book_slot_select_index(world, 0.0);
    collection.recipe_index_and_craftable_at_slot_select_index(slot_select_index)
}

pub(super) fn recipe_book_collection_at_position<'a>(
    input: &ClientInputState,
    world: &'a WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<(RecipeBookType, usize, usize, RecipeBookUiCollection<'a>)> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    let book_type = recipe_book_type_for_background(layout.background)?;
    let _recipe_book = layout.recipe_book?;
    let collections = recipe_book_collections_for_background(
        input,
        world,
        item_runtime,
        layout.background,
        book_type,
    )?;
    let page = clamped_recipe_book_page(
        selected_recipe_book_page_index(input, book_type),
        collections.len(),
    );
    let visible_index =
        recipe_book_recipe_button_index_at_position(cursor_position, surface_size, &layout)?;
    let collection_index = page * RECIPE_BOOK_ITEMS_PER_PAGE + visible_index;
    collections
        .get(collection_index)
        .cloned()
        .map(|collection| (book_type, page, visible_index, collection))
}

pub(super) fn recipe_book_overlay_recipe_at_position(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<(i32, bool)> {
    let overlay = input.container().recipe_book_overlay?;
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    let book_type = recipe_book_type_for_background(layout.background)?;
    let recipe_book = layout.recipe_book?;
    if overlay.book_type != book_type {
        return None;
    }
    let collections = recipe_book_collections_for_background(
        input,
        world,
        item_runtime,
        layout.background,
        book_type,
    )?;
    let page = clamped_recipe_book_page(overlay.page_index, collections.len());
    let collection_index = page * RECIPE_BOOK_ITEMS_PER_PAGE + overlay.button_index;
    let item_tag_entries = world.registry_tags("minecraft:item").map(|tags| &tags.tags);
    let entries = collections
        .get(collection_index)?
        .overlay_entries(item_tag_entries, recipe_book_slot_select_index(world, 0.0));
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x - f64::from(recipe_book.x);
    let y = cursor.y - origin_y;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    for (index, entry) in entries.iter().enumerate() {
        let (button_x, button_y) =
            recipe_book_overlay_entry_position(overlay.x, overlay.y, index, entries.len());
        if x >= f64::from(button_x)
            && x < f64::from(button_x + RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE)
            && y >= f64::from(button_y)
            && y < f64::from(button_y + RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE)
        {
            return Some((entry.recipe_index, entry.craftable));
        }
    }
    None
}

pub(super) fn recipe_book_collections_for_background<'a>(
    input: &ClientInputState,
    world: &'a WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    book_type: RecipeBookType,
) -> Option<Vec<RecipeBookUiCollection<'a>>> {
    let tab_count = recipe_book_tab_count_for_background(background)?;
    if tab_count == 0 {
        return None;
    }
    let selected_tab = selected_recipe_book_tab_index(input, book_type).min(tab_count - 1);
    let only_craftable = recipe_book_type_settings(world, book_type).filtering;
    if let Some(grid) = recipe_book_crafting_grid_for_background(background) {
        return Some(crafting_recipe_book_collections(
            world,
            grid,
            selected_tab,
            only_craftable,
            &input.container().recipe_book_search.text(),
            item_runtime,
        ));
    }
    recipe_book_furnace_family_for_background(background).map(|family| {
        furnace_recipe_book_collections(
            world,
            family,
            selected_tab,
            only_craftable,
            &input.container().recipe_book_search.text(),
            item_runtime,
        )
    })
}

pub(super) fn recipe_book_recipe_button_index_at_position(
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    layout: &InventoryScreenLayout,
) -> Option<usize> {
    let recipe_book = layout.recipe_book?;
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, layout);
    let x = cursor.x - origin_x - f64::from(recipe_book.x + RECIPE_BOOK_RECIPE_BUTTON_X);
    let y = cursor.y - origin_y - f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y);
    if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
        return None;
    }
    let column = (x / f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE)).floor();
    let row = (y / f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE)).floor();
    if column < 0.0
        || column >= RECIPE_BOOK_RECIPE_BUTTON_COLUMNS as f64
        || row < 0.0
        || row >= (RECIPE_BOOK_ITEMS_PER_PAGE / RECIPE_BOOK_RECIPE_BUTTON_COLUMNS) as f64
    {
        return None;
    }
    let local_x = x - column * f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE);
    let local_y = y - row * f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE);
    if local_x >= f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE)
        || local_y >= f64::from(RECIPE_BOOK_RECIPE_BUTTON_SIZE)
    {
        return None;
    }
    let column = column as usize;
    let row = row as usize;
    Some(row * RECIPE_BOOK_RECIPE_BUTTON_COLUMNS + column)
}

pub(super) fn recipe_book_page_button_contains(
    x: f64,
    y: f64,
    button_x: i32,
    button_y: i32,
) -> bool {
    x >= f64::from(button_x)
        && x < f64::from(button_x + RECIPE_BOOK_PAGE_BUTTON_WIDTH)
        && y >= f64::from(button_y)
        && y < f64::from(button_y + RECIPE_BOOK_PAGE_BUTTON_HEIGHT)
}

pub(super) fn recipe_book_crafting_grid_for_background(
    background: InventoryScreenBackground,
) -> Option<RecipeBookCraftingGrid> {
    match background {
        InventoryScreenBackground::LocalInventory => Some(RecipeBookCraftingGrid {
            width: 2,
            height: 2,
        }),
        InventoryScreenBackground::CraftingTable => Some(RecipeBookCraftingGrid {
            width: 3,
            height: 3,
        }),
        _ => None,
    }
}

pub(super) fn recipe_book_furnace_family_for_background(
    background: InventoryScreenBackground,
) -> Option<RecipeBookFurnaceFamily> {
    match background {
        InventoryScreenBackground::Furnace => Some(RecipeBookFurnaceFamily::Furnace),
        InventoryScreenBackground::BlastFurnace => Some(RecipeBookFurnaceFamily::BlastFurnace),
        InventoryScreenBackground::Smoker => Some(RecipeBookFurnaceFamily::Smoker),
        _ => None,
    }
}

pub(super) fn enchantment_button_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<i32> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    if layout.background != InventoryScreenBackground::EnchantmentTable {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    for button_id in 0..ENCHANTMENT_BUTTON_COUNT {
        let button_y = ENCHANTMENT_BUTTON_Y + button_id * ENCHANTMENT_BUTTON_SPACING;
        if x >= f64::from(ENCHANTMENT_BUTTON_X)
            && x < f64::from(ENCHANTMENT_BUTTON_X + ENCHANTMENT_BUTTON_WIDTH)
            && y >= f64::from(button_y)
            && y < f64::from(button_y + ENCHANTMENT_BUTTON_HEIGHT)
        {
            return Some(button_id);
        }
    }
    None
}

pub(super) fn loom_click_target_at_position(
    world: &WorldStore,
    start_index: i32,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<LoomClickTarget> {
    let selectable_count = loom_selectable_pattern_count(world)?;
    if selectable_count <= 0 {
        return None;
    }
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    if layout.background != InventoryScreenBackground::Loom {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x - f64::from(LOOM_PATTERN_BUTTON_X);
    let y = cursor.y - origin_y - f64::from(LOOM_PATTERN_BUTTON_Y);
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let column = (x / f64::from(LOOM_PATTERN_BUTTON_SIZE)) as i32;
    let row = (y / f64::from(LOOM_PATTERN_BUTTON_SIZE)) as i32;
    if column >= LOOM_PATTERN_BUTTON_COLUMNS || row >= LOOM_PATTERN_BUTTON_ROWS {
        return None;
    }
    let button_id = start_index + row * LOOM_PATTERN_BUTTON_COLUMNS + column;
    (button_id < selectable_count).then_some(LoomClickTarget::Pattern(button_id))
}

pub(super) fn merchant_trade_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<i32> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    if layout.background != InventoryScreenBackground::Merchant {
        return None;
    }
    let (offer_count, scroll_offset) = world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
        .map(|offers| (offers.offers.len(), offers.local_scroll_offset))
        .unwrap_or_default();
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    if x < f64::from(MERCHANT_TRADE_BUTTON_X)
        || x >= f64::from(MERCHANT_TRADE_BUTTON_X + MERCHANT_TRADE_BUTTON_WIDTH)
    {
        return None;
    }
    for row in 0..MERCHANT_TRADE_BUTTON_COUNT {
        let button_y = MERCHANT_TRADE_BUTTON_Y + row * MERCHANT_TRADE_BUTTON_HEIGHT;
        let item = scroll_offset + row;
        if y >= f64::from(button_y)
            && y < f64::from(button_y + MERCHANT_TRADE_BUTTON_HEIGHT)
            && usize::try_from(item).is_ok_and(|item| item < offer_count)
        {
            return Some(item);
        }
    }
    None
}

pub(super) fn merchant_scroller_track_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if !merchant_offers_can_scroll(world) {
        return false;
    }
    let Some(layout) = inventory_screen_layout_for_surface(world, surface_size) else {
        return false;
    };
    if layout.background != InventoryScreenBackground::Merchant {
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    x > f64::from(MERCHANT_SCROLLER_X)
        && x < f64::from(MERCHANT_SCROLLER_X + MERCHANT_SCROLLER_WIDTH)
        && y > f64::from(MERCHANT_SCROLLER_Y)
        && y <= f64::from(MERCHANT_SCROLLER_Y + MERCHANT_SCROLLER_FULL_HEIGHT + 1)
}

pub(super) fn loom_scroller_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if loom_pattern_max_scroll_row(world).unwrap_or_default() <= 0 {
        return false;
    }
    let Some(layout) = inventory_screen_layout_for_surface(world, surface_size) else {
        return false;
    };
    if layout.background != InventoryScreenBackground::Loom {
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    x >= f64::from(LOOM_SCROLLER_X)
        && x < f64::from(LOOM_SCROLLER_X + LOOM_SCROLLER_WIDTH)
        && y >= f64::from(LOOM_SCROLLER_CLICK_Y)
        && y < f64::from(LOOM_SCROLLER_CLICK_Y + LOOM_SCROLLER_FULL_HEIGHT)
}

pub(super) fn update_merchant_trade_scroll_from_cursor(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((offer_count, current_scroll_offset)) = merchant_offer_count_and_scroll_offset(world)
    else {
        input.container_mut().merchant_trade_scrolling = false;
        return false;
    };
    let max_scroll_offset = merchant_max_scroll_offset(offer_count);
    if max_scroll_offset <= 0 {
        input.container_mut().merchant_trade_scrolling = false;
        return false;
    }
    let Some(layout) = inventory_screen_layout_for_surface(world, surface_size) else {
        input.container_mut().merchant_trade_scrolling = false;
        return false;
    };
    if layout.background != InventoryScreenBackground::Merchant {
        input.container_mut().merchant_trade_scrolling = false;
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (_, origin_y) = inventory_screen_origin(surface_size, &layout);
    let y = cursor.y - origin_y;
    let drag_range = f64::from(MERCHANT_SCROLLER_FULL_HEIGHT - MERCHANT_SCROLLER_HEIGHT);
    let scroll_offset =
        (((y - f64::from(MERCHANT_SCROLLER_Y) - f64::from(MERCHANT_SCROLLER_HEIGHT) * 0.5)
            / drag_range)
            * f64::from(max_scroll_offset)
            + 0.5) as i32;
    let scroll_offset = scroll_offset.clamp(0, max_scroll_offset);
    if scroll_offset != current_scroll_offset {
        world.scroll_local_merchant_offers(scroll_offset - current_scroll_offset);
    }
    true
}

pub(super) fn update_loom_pattern_scroll_from_cursor(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_loom_pattern_scroll(input, world);
    let Some(max_scroll_row) = loom_pattern_max_scroll_row(world) else {
        input.container_mut().loom_pattern_scrolling = false;
        return false;
    };
    if max_scroll_row <= 0 {
        input.container_mut().loom_pattern_scrolling = false;
        return false;
    }
    let Some(layout) = inventory_screen_layout_for_surface(world, surface_size) else {
        input.container_mut().loom_pattern_scrolling = false;
        return false;
    };
    if layout.background != InventoryScreenBackground::Loom {
        input.container_mut().loom_pattern_scrolling = false;
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (_, origin_y) = inventory_screen_origin(surface_size, &layout);
    let y = cursor.y - origin_y;
    let drag_range = f64::from(LOOM_SCROLLER_FULL_HEIGHT - LOOM_SCROLLER_HEIGHT);
    let scroll_offs =
        ((y - f64::from(LOOM_SCROLLER_DRAG_Y) - f64::from(LOOM_SCROLLER_HEIGHT) * 0.5)
            / drag_range)
            .clamp(0.0, 1.0);
    input.container_mut().loom_pattern_scroll_row =
        (scroll_offs * f64::from(max_scroll_row) + 0.5) as i32;
    true
}

pub(super) fn lectern_button_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<LecternClickTarget> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    if layout.background != InventoryScreenBackground::Lectern {
        return None;
    }
    let lectern = world.lectern_book_view()?;
    let cursor = cursor_position?;
    let book_layout = bbb_client_render::hud_book_layout(surface_size);
    let x = cursor.x - f64::from(book_layout.origin_x);
    let y = cursor.y - f64::from(book_layout.origin_y);
    if y >= f64::from(LECTERN_MENU_BUTTON_Y)
        && y < f64::from(LECTERN_MENU_BUTTON_Y + LECTERN_MENU_BUTTON_HEIGHT)
    {
        if lectern.may_build {
            if x >= f64::from(LECTERN_MENU_DONE_BUTTON_X)
                && x < f64::from(LECTERN_MENU_DONE_BUTTON_X + LECTERN_MENU_BUTTON_WIDTH)
            {
                return Some(LecternClickTarget::Done);
            }
            if x >= f64::from(LECTERN_MENU_TAKE_BOOK_BUTTON_X)
                && x < f64::from(LECTERN_MENU_TAKE_BOOK_BUTTON_X + LECTERN_MENU_BUTTON_WIDTH)
            {
                return Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_TAKE_BOOK));
            }
        } else if x >= f64::from(LECTERN_MENU_DONE_BUTTON_X)
            && x < f64::from(LECTERN_MENU_DONE_BUTTON_X + 200)
        {
            return Some(LecternClickTarget::Done);
        }
        return None;
    }
    if y < f64::from(LECTERN_PAGE_BUTTON_Y)
        || y >= f64::from(LECTERN_PAGE_BUTTON_Y + LECTERN_PAGE_BUTTON_HEIGHT)
    {
        return None;
    }
    let current_page = lectern.current_page;
    let page_count = lectern.pages.len();
    if current_page > 0
        && x >= f64::from(LECTERN_PAGE_BACK_BUTTON_X)
        && x < f64::from(LECTERN_PAGE_BACK_BUTTON_X + LECTERN_PAGE_BUTTON_WIDTH)
    {
        return Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_PREV_PAGE));
    }
    if current_page + 1 < page_count
        && x >= f64::from(LECTERN_PAGE_FORWARD_BUTTON_X)
        && x < f64::from(LECTERN_PAGE_FORWARD_BUTTON_X + LECTERN_PAGE_BUTTON_WIDTH)
    {
        return Some(LecternClickTarget::MenuButton(LECTERN_BUTTON_NEXT_PAGE));
    }
    None
}

pub(super) fn beacon_button_at_position(
    input: &ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<BeaconClickTarget> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    if layout.background != InventoryScreenBackground::Beacon {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;

    if y >= f64::from(BEACON_ACTION_BUTTON_Y)
        && y < f64::from(BEACON_ACTION_BUTTON_Y + BEACON_ACTION_BUTTON_SIZE)
    {
        if x >= f64::from(BEACON_CONFIRM_BUTTON_X)
            && x < f64::from(BEACON_CONFIRM_BUTTON_X + BEACON_ACTION_BUTTON_SIZE)
        {
            return Some(BeaconClickTarget::Confirm);
        }
        if x >= f64::from(BEACON_CANCEL_BUTTON_X)
            && x < f64::from(BEACON_CANCEL_BUTTON_X + BEACON_ACTION_BUTTON_SIZE)
        {
            return Some(BeaconClickTarget::Cancel);
        }
        return None;
    }

    let levels = beacon_levels(world);
    for button in beacon_effect_buttons(input.container().beacon_primary_effect) {
        if button.tier >= levels {
            continue;
        }
        if x >= f64::from(button.x)
            && x < f64::from(button.x + BEACON_EFFECT_BUTTON_SIZE as i32)
            && y >= f64::from(button.y)
            && y < f64::from(button.y + BEACON_EFFECT_BUTTON_SIZE as i32)
        {
            return Some(BeaconClickTarget::Effect {
                primary: button.primary,
                effect_id: button.effect_id,
            });
        }
    }
    None
}

pub(super) fn stonecutter_recipe_button_at_position(
    world: &WorldStore,
    start_index: i32,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<i32> {
    if stonecutter_visible_recipe_count(world)? <= 0 {
        return None;
    }
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    if layout.background != InventoryScreenBackground::Stonecutter {
        return None;
    }
    let cursor = cursor_position?;
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x - f64::from(STONECUTTER_RECIPE_BUTTON_X);
    let y = cursor.y - origin_y - f64::from(STONECUTTER_RECIPE_BUTTON_Y);
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let column = (x / f64::from(STONECUTTER_RECIPE_BUTTON_WIDTH)) as i32;
    let row = (y / f64::from(STONECUTTER_RECIPE_BUTTON_HEIGHT)) as i32;
    if column >= STONECUTTER_RECIPE_BUTTON_COLUMNS || row >= STONECUTTER_RECIPE_BUTTON_ROWS {
        return None;
    }
    let button_id = start_index + row * STONECUTTER_RECIPE_BUTTON_COLUMNS + column;
    Some(button_id)
}

pub(super) fn stonecutter_scroller_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(layout) = inventory_screen_layout_for_surface(world, surface_size) else {
        return false;
    };
    if layout.background != InventoryScreenBackground::Stonecutter {
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, &layout);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    x >= f64::from(STONECUTTER_SCROLLER_X)
        && x < f64::from(STONECUTTER_SCROLLER_X + STONECUTTER_SCROLLER_WIDTH)
        && y >= f64::from(STONECUTTER_SCROLLER_CLICK_Y)
        && y < f64::from(STONECUTTER_SCROLLER_CLICK_Y + STONECUTTER_SCROLLER_FULL_HEIGHT)
}

pub(super) fn update_stonecutter_recipe_scroll_from_cursor(
    input: &mut ClientInputState,
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    sync_stonecutter_recipe_scroll(input, world);
    let Some(max_scroll_row) = stonecutter_recipe_max_scroll_row(world) else {
        input.container_mut().stonecutter_recipe_scrolling = false;
        return false;
    };
    if max_scroll_row <= 0 {
        input.container_mut().stonecutter_recipe_scrolling = false;
        return false;
    }
    let Some(layout) = inventory_screen_layout_for_surface(world, surface_size) else {
        input.container_mut().stonecutter_recipe_scrolling = false;
        return false;
    };
    if layout.background != InventoryScreenBackground::Stonecutter {
        input.container_mut().stonecutter_recipe_scrolling = false;
        return false;
    }
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (_, origin_y) = inventory_screen_origin(surface_size, &layout);
    let y = cursor.y - origin_y;
    let drag_range = f64::from(STONECUTTER_SCROLLER_FULL_HEIGHT - STONECUTTER_SCROLLER_HEIGHT);
    let scroll_offs = ((y
        - f64::from(STONECUTTER_SCROLLER_DRAG_Y)
        - f64::from(STONECUTTER_SCROLLER_HEIGHT) * 0.5)
        / drag_range)
        .clamp(0.0, 1.0);
    input.container_mut().stonecutter_recipe_scroll_row =
        (scroll_offs * f64::from(max_scroll_row) + 0.5) as i32;
    true
}

pub(super) fn stonecutter_visible_recipe_count(world: &WorldStore) -> Option<i32> {
    i32::try_from(world.stonecutter_visible_recipe_count()?).ok()
}

pub(super) fn stonecutter_recipe_max_scroll_row(world: &WorldStore) -> Option<i32> {
    let visible_recipes = stonecutter_visible_recipe_count(world)?;
    Some((stonecutter_recipe_row_count(visible_recipes) - STONECUTTER_RECIPE_BUTTON_ROWS).max(0))
}

pub(super) fn loom_pattern_max_scroll_row(world: &WorldStore) -> Option<i32> {
    let selectable_count = loom_selectable_pattern_count(world)?;
    Some((loom_pattern_row_count(selectable_count) - LOOM_PATTERN_BUTTON_ROWS).max(0))
}

pub(super) fn loom_selectable_pattern_count(world: &WorldStore) -> Option<i32> {
    i32::try_from(world.loom_selectable_banner_pattern_ids()?.len()).ok()
}

pub(super) fn stonecutter_recipe_row_count(visible_recipes: i32) -> i32 {
    if visible_recipes <= 0 {
        0
    } else {
        (visible_recipes + STONECUTTER_RECIPE_BUTTON_COLUMNS - 1)
            / STONECUTTER_RECIPE_BUTTON_COLUMNS
    }
}

pub(super) fn beacon_set_command(
    input: &ClientInputState,
    world: &WorldStore,
) -> Option<SetBeacon> {
    if !inventory_slot_has_item(world, 0) {
        return None;
    }
    let primary_effect = input.container().beacon_primary_effect?;
    Some(SetBeacon {
        primary_effect: Some(primary_effect),
        secondary_effect: input.container().beacon_secondary_effect,
    })
}

pub(super) fn beacon_levels(world: &WorldStore) -> i16 {
    world
        .open_container_data_value(BEACON_LEVELS_DATA_ID)
        .unwrap_or_default()
}

pub(super) fn beacon_data_effect_id(world: &WorldStore, data_id: i16) -> Option<i32> {
    let value = world.open_container_data_value(data_id)?;
    (value > 0).then_some(i32::from(value) - 1)
}
