use super::*;

const RECIPE_BOOK_SEARCH_TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const RECIPE_BOOK_RECIPE_ICON_OFFSET: i32 = 4;
const RECIPE_BOOK_PAGE_TEXT_CENTER_X: i32 = 73;
const RECIPE_BOOK_PAGE_TEXT_Y: i32 = 141;
pub(super) const RECIPE_BOOK_GHOST_PRE_ITEM_TINT: [f32; 4] = [1.0, 0.0, 0.0, 48.0 / 255.0];
pub(super) const RECIPE_BOOK_GHOST_POST_ITEM_TINT: [f32; 4] = [1.0, 1.0, 1.0, 48.0 / 255.0];
pub(super) fn hud_recipe_book_background_layer() -> HudInventoryBackgroundLayer {
    hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Texture("textures/gui/recipe_book.png"),
        0,
        0,
        147,
        166,
        [1.0 / 256.0, 1.0 / 256.0],
        [148.0 / 256.0, 167.0 / 256.0],
    )
}

#[derive(Debug, Clone, Copy)]
struct RecipeBookTabIconSpec {
    primary: &'static str,
    secondary: Option<&'static str>,
}

const CRAFTING_RECIPE_BOOK_TAB_ICONS: [RecipeBookTabIconSpec; 5] = [
    RecipeBookTabIconSpec {
        primary: "minecraft:compass",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:iron_axe",
        secondary: Some("minecraft:golden_sword"),
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:bricks",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:lava_bucket",
        secondary: Some("minecraft:apple"),
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:redstone",
        secondary: None,
    },
];

const FURNACE_RECIPE_BOOK_TAB_ICONS: [RecipeBookTabIconSpec; 4] = [
    RecipeBookTabIconSpec {
        primary: "minecraft:compass",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:porkchop",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:stone",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:lava_bucket",
        secondary: Some("minecraft:emerald"),
    },
];

const BLAST_FURNACE_RECIPE_BOOK_TAB_ICONS: [RecipeBookTabIconSpec; 3] = [
    RecipeBookTabIconSpec {
        primary: "minecraft:compass",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:redstone_ore",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:iron_shovel",
        secondary: Some("minecraft:golden_leggings"),
    },
];

const SMOKER_RECIPE_BOOK_TAB_ICONS: [RecipeBookTabIconSpec; 2] = [
    RecipeBookTabIconSpec {
        primary: "minecraft:compass",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:porkchop",
        secondary: None,
    },
];
const RECIPE_BOOK_TAB_ANIMATION_TIME: f32 = 15.0;
const RECIPE_BOOK_TAB_ANIMATION_PIVOT_Y: f32 = 12.0;
const RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_X: f32 = 8.0;
const RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_Y: f32 = 12.0;

pub(super) fn hud_recipe_book_tab_layers(
    world: &WorldStore,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    presentation: &RecipeBookPresentationHudState,
    partial_tick: f32,
) -> Vec<HudInventoryBackgroundLayer> {
    if recipe_book_main_gui_offset(world, background) == 0 {
        return Vec::new();
    }
    let visible_tabs = recipe_book_visible_tab_indices(world, background);
    if visible_tabs.is_empty() {
        return Vec::new();
    };
    let selected_index = recipe_book_selected_tab_index(background, tabs).unwrap_or_default();
    visible_tabs
        .into_iter()
        .enumerate()
        .map(|(visible_index, tab_index)| {
            let selected = tab_index == selected_index;
            let scale_y = recipe_book_animation_scale(
                presentation
                    .tab_animation_ticks
                    .get(tab_index)
                    .copied()
                    .unwrap_or_default(),
                partial_tick,
            );
            let y = RECIPE_BOOK_TAB_Y + RECIPE_BOOK_TAB_STRIDE_Y * visible_index as i32;
            let (y, height) =
                scaled_recipe_book_tab_y_and_height(y, RECIPE_BOOK_TAB_HEIGHT, scale_y);
            hud_inventory_background_layer(
                if selected {
                    HudInventoryBackgroundTexture::Sprite("recipe_book/tab_selected")
                } else {
                    HudInventoryBackgroundTexture::Sprite("recipe_book/tab")
                },
                RECIPE_BOOK_TAB_X
                    + if selected {
                        RECIPE_BOOK_SELECTED_TAB_X_OFFSET
                    } else {
                        0
                    },
                y,
                u32::try_from(RECIPE_BOOK_TAB_WIDTH).unwrap_or_default(),
                height,
                [0.0, 0.0],
                [1.0, 1.0],
            )
        })
        .collect()
}

fn recipe_book_animation_scale(remaining: u8, partial_tick: f32) -> f32 {
    if remaining == 0 {
        return 1.0;
    }
    let animation_time = (remaining as f32 - partial_tick.clamp(0.0, 1.0)).max(0.0);
    if animation_time <= 0.0 {
        return 1.0;
    }
    1.0 + 0.1 * (animation_time / RECIPE_BOOK_TAB_ANIMATION_TIME * std::f32::consts::PI).sin()
}

fn scaled_recipe_book_tab_y_and_height(y: i32, height: i32, scale_y: f32) -> (i32, u32) {
    if (scale_y - 1.0).abs() <= f32::EPSILON {
        return (y, u32::try_from(height).unwrap_or_default());
    }
    let pivot_y = y as f32 + RECIPE_BOOK_TAB_ANIMATION_PIVOT_Y;
    let top = pivot_y + (y as f32 - pivot_y) * scale_y;
    let bottom = pivot_y + ((y + height) as f32 - pivot_y) * scale_y;
    let scaled_y = top.round() as i32;
    let scaled_height = (bottom - top).round().max(1.0) as u32;
    (scaled_y, scaled_height)
}

fn scaled_recipe_book_tab_child_y(tab_y: i32, child_y: i32, scale_y: f32) -> i32 {
    if (scale_y - 1.0).abs() <= f32::EPSILON {
        return child_y;
    }
    let pivot_y = tab_y as f32 + RECIPE_BOOK_TAB_ANIMATION_PIVOT_Y;
    (pivot_y + (child_y as f32 - pivot_y) * scale_y).round() as i32
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_recipe_book_tab_icon_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    presentation: &RecipeBookPresentationHudState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    if recipe_book_main_gui_offset(world, background) == 0 {
        return Vec::new();
    }
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let Some(selected_index) = recipe_book_selected_tab_index(background, tabs) else {
        return Vec::new();
    };
    let Some(icons) = recipe_book_tab_icons(background) else {
        return Vec::new();
    };
    let visible_tabs = recipe_book_visible_tab_indices(world, background);
    let mut items = Vec::new();
    for (visible_index, tab_index) in visible_tabs.into_iter().enumerate() {
        let Some(icon) = icons.get(tab_index) else {
            continue;
        };
        let move_left = if tab_index == selected_index {
            RECIPE_BOOK_SELECTED_TAB_X_OFFSET
        } else {
            0
        };
        let tab_y = RECIPE_BOOK_TAB_Y + RECIPE_BOOK_TAB_STRIDE_Y * visible_index as i32;
        let scale_y = recipe_book_animation_scale(
            presentation
                .tab_animation_ticks
                .get(tab_index)
                .copied()
                .unwrap_or_default(),
            partial_tick,
        );
        let y = scaled_recipe_book_tab_child_y(tab_y, tab_y + 5, scale_y);
        if let Some(secondary) = icon.secondary {
            push_recipe_book_tab_icon_item(
                &mut items,
                world,
                item_runtime,
                terrain_textures,
                icon.primary,
                RECIPE_BOOK_TAB_X + 3 + move_left,
                y,
                scale_y,
                keybind_context,
                partial_tick,
            );
            push_recipe_book_tab_icon_item(
                &mut items,
                world,
                item_runtime,
                terrain_textures,
                secondary,
                RECIPE_BOOK_TAB_X + 14 + move_left,
                y,
                scale_y,
                keybind_context,
                partial_tick,
            );
        } else {
            push_recipe_book_tab_icon_item(
                &mut items,
                world,
                item_runtime,
                terrain_textures,
                icon.primary,
                RECIPE_BOOK_TAB_X + 9 + move_left,
                y,
                scale_y,
                keybind_context,
                partial_tick,
            );
        }
    }
    items
}

#[allow(clippy::too_many_arguments)]
fn push_recipe_book_tab_icon_item(
    items: &mut Vec<HudInventoryItem>,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    resource_id: &str,
    x: i32,
    y: i32,
    scale_y: f32,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) {
    let item_models = HudItemModelContext::new(world, Some(item_runtime));
    let Some(item_id) = item_runtime.item_protocol_id(resource_id) else {
        return;
    };
    let stack = ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: Default::default(),
    };
    let projection = item_models.project(
        &stack,
        HudItemSlotState {
            keybind: keybind_context,
            ..HudItemSlotState::default()
        },
        None,
    );
    let Some(icon) = projection
        .as_ref()
        .and_then(|projection| projection.icon(partial_tick))
    else {
        return;
    };
    items.push(HudInventoryItem {
        x,
        y,
        scale: 1.0,
        scale_y,
        icon,
        scissor: None,
        draw_decorations: true,
        block_model: projection
            .as_ref()
            .and_then(|projection| projection.block_model(terrain_textures)),
    });
}

fn recipe_book_selected_tab_index(
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
) -> Option<usize> {
    let tab_count = recipe_book_tab_count_for_background(background)?;
    if tab_count == 0 {
        return None;
    }
    let index = match background {
        InventoryScreenBackground::LocalInventory | InventoryScreenBackground::CraftingTable => {
            tabs.crafting
        }
        InventoryScreenBackground::Furnace => tabs.furnace,
        InventoryScreenBackground::BlastFurnace => tabs.blast_furnace,
        InventoryScreenBackground::Smoker => tabs.smoker,
        _ => return None,
    };
    Some(index.min(tab_count - 1))
}

fn recipe_book_selected_page_index(
    background: InventoryScreenBackground,
    pages: &RecipeBookPageHudState,
) -> Option<usize> {
    match background {
        InventoryScreenBackground::LocalInventory | InventoryScreenBackground::CraftingTable => {
            Some(pages.crafting)
        }
        InventoryScreenBackground::Furnace => Some(pages.furnace),
        InventoryScreenBackground::BlastFurnace => Some(pages.blast_furnace),
        InventoryScreenBackground::Smoker => Some(pages.smoker),
        _ => None,
    }
}

fn recipe_book_tab_icons(
    background: InventoryScreenBackground,
) -> Option<&'static [RecipeBookTabIconSpec]> {
    match background {
        InventoryScreenBackground::LocalInventory | InventoryScreenBackground::CraftingTable => {
            Some(&CRAFTING_RECIPE_BOOK_TAB_ICONS)
        }
        InventoryScreenBackground::Furnace => Some(&FURNACE_RECIPE_BOOK_TAB_ICONS),
        InventoryScreenBackground::BlastFurnace => Some(&BLAST_FURNACE_RECIPE_BOOK_TAB_ICONS),
        InventoryScreenBackground::Smoker => Some(&SMOKER_RECIPE_BOOK_TAB_ICONS),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_recipe_book_recipe_button_layers(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    presentation: &RecipeBookPresentationHudState,
    partial_tick: f32,
) -> Vec<HudInventoryBackgroundLayer> {
    hud_recipe_book_visible_collections(world, item_runtime, background, tabs, pages, search)
        .into_iter()
        .enumerate()
        .map(|(index, collection)| {
            let (x, y) = recipe_book_recipe_button_position(index);
            let scale = recipe_book_animation_scale(
                presentation
                    .button_animation_ticks
                    .get(index)
                    .copied()
                    .unwrap_or_default(),
                partial_tick,
            );
            let (x, y, width, height) = scaled_recipe_book_button_rect(
                x,
                y,
                RECIPE_BOOK_RECIPE_BUTTON_SIZE,
                RECIPE_BOOK_RECIPE_BUTTON_SIZE,
                scale,
            );
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite(
                    match (
                        collection.has_craftable(),
                        collection.has_multiple_recipes(),
                    ) {
                        (true, true) => "recipe_book/slot_many_craftable",
                        (true, false) => "recipe_book/slot_craftable",
                        (false, true) => "recipe_book/slot_many_uncraftable",
                        (false, false) => "recipe_book/slot_uncraftable",
                    },
                ),
                x,
                y,
                width,
                height,
                [0.0, 0.0],
                [1.0, 1.0],
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_recipe_book_recipe_button_icon_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    presentation: &RecipeBookPresentationHudState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let item_models = HudItemModelContext::new(world, item_runtime);
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let collections = hud_recipe_book_visible_collections(
        world,
        Some(item_runtime),
        background,
        tabs,
        pages,
        search,
    );
    let mut items = Vec::new();
    let slot_select_index = recipe_book_slot_select_index(world, partial_tick);
    for (index, collection) in collections.into_iter().enumerate() {
        let Some(stack) = collection.result_stack_at_slot_select_index(slot_select_index) else {
            continue;
        };
        let projection = item_models.project(
            stack,
            HudItemSlotState {
                keybind: keybind_context,
                ..HudItemSlotState::default()
            },
            None,
        );
        let Some(icon) = projection
            .as_ref()
            .and_then(|projection| projection.icon(partial_tick))
        else {
            continue;
        };
        let (button_x, button_y) = recipe_book_recipe_button_position(index);
        let scale = recipe_book_animation_scale(
            presentation
                .button_animation_ticks
                .get(index)
                .copied()
                .unwrap_or_default(),
            partial_tick,
        );
        let block_model = projection
            .as_ref()
            .and_then(|projection| projection.block_model(terrain_textures));
        if collection.has_multiple_recipes() && collection.all_result_stacks_same() {
            let (x, y) = scaled_recipe_book_button_child_position(
                button_x,
                button_y,
                button_x + RECIPE_BOOK_RECIPE_ICON_OFFSET + 1,
                button_y + RECIPE_BOOK_RECIPE_ICON_OFFSET + 1,
                scale,
            );
            items.push(HudInventoryItem {
                x,
                y,
                scale,
                scale_y: scale,
                icon: icon.clone(),
                scissor: None,
                draw_decorations: false,
                block_model: block_model.clone(),
            });
            let (x, y) = scaled_recipe_book_button_child_position(
                button_x,
                button_y,
                button_x + RECIPE_BOOK_RECIPE_ICON_OFFSET - 1,
                button_y + RECIPE_BOOK_RECIPE_ICON_OFFSET - 1,
                scale,
            );
            items.push(HudInventoryItem {
                x,
                y,
                scale,
                scale_y: scale,
                icon,
                scissor: None,
                draw_decorations: false,
                block_model,
            });
            continue;
        }
        let (x, y) = scaled_recipe_book_button_child_position(
            button_x,
            button_y,
            button_x + RECIPE_BOOK_RECIPE_ICON_OFFSET,
            button_y + RECIPE_BOOK_RECIPE_ICON_OFFSET,
            scale,
        );
        items.push(HudInventoryItem {
            x,
            y,
            scale,
            scale_y: scale,
            icon,
            scissor: None,
            draw_decorations: false,
            block_model,
        });
    }
    items
}

fn scaled_recipe_book_button_rect(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scale: f32,
) -> (i32, i32, u32, u32) {
    let pivot_x = x as f32 + RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_X;
    let pivot_y = y as f32 + RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_Y;
    let left = pivot_x + (x as f32 - pivot_x) * scale;
    let top = pivot_y + (y as f32 - pivot_y) * scale;
    let right = pivot_x + ((x + width) as f32 - pivot_x) * scale;
    let bottom = pivot_y + ((y + height) as f32 - pivot_y) * scale;
    (
        left.round() as i32,
        top.round() as i32,
        (right - left).round().max(1.0) as u32,
        (bottom - top).round().max(1.0) as u32,
    )
}

fn scaled_recipe_book_button_child_position(
    button_x: i32,
    button_y: i32,
    child_x: i32,
    child_y: i32,
    scale: f32,
) -> (i32, i32) {
    let pivot_x = button_x as f32 + RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_X;
    let pivot_y = button_y as f32 + RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_Y;
    (
        (pivot_x + (child_x as f32 - pivot_x) * scale).round() as i32,
        (pivot_y + (child_y as f32 - pivot_y) * scale).round() as i32,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_recipe_book_overlay_layers(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    overlay: Option<RecipeBookOverlayHudState>,
    cursor_position: Option<(i32, i32)>,
    partial_tick: f32,
) -> Vec<HudInventoryBackgroundLayer> {
    let Some((overlay, entries)) = hud_recipe_book_overlay_entries(
        world,
        item_runtime,
        background,
        tabs,
        pages,
        search,
        overlay,
        partial_tick,
    ) else {
        return Vec::new();
    };
    let columns = entries
        .len()
        .min(recipe_book_overlay_max_row(entries.len()));
    let rows = recipe_book_overlay_rows(entries.len());
    let mut layers = vec![hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Sprite("recipe_book/overlay_recipe"),
        overlay.x,
        overlay.y,
        u32::try_from(
            i32::try_from(columns).unwrap_or_default() * RECIPE_BOOK_OVERLAY_BUTTON_SIZE
                + RECIPE_BOOK_OVERLAY_BACKGROUND_BORDER,
        )
        .unwrap_or_default(),
        u32::try_from(
            i32::try_from(rows).unwrap_or_default() * RECIPE_BOOK_OVERLAY_BUTTON_SIZE
                + RECIPE_BOOK_OVERLAY_BACKGROUND_BORDER,
        )
        .unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    )];
    let furnace_overlay = recipe_book_furnace_family_for_background(background).is_some();
    for (index, entry) in entries.iter().enumerate() {
        let (x, y) = recipe_book_overlay_entry_position(overlay.x, overlay.y, index, entries.len());
        let highlighted = cursor_position.is_some_and(|(cursor_x, cursor_y)| {
            cursor_x >= x
                && cursor_x < x + RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE
                && cursor_y >= y
                && cursor_y < y + RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE
        });
        layers.push(hud_inventory_background_layer(
            recipe_book_overlay_button_texture(furnace_overlay, entry.craftable, highlighted),
            x,
            y,
            u32::try_from(RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE).unwrap_or_default(),
            u32::try_from(RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE).unwrap_or_default(),
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }
    layers
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_recipe_book_overlay_icon_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    overlay: Option<RecipeBookOverlayHudState>,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let item_models = HudItemModelContext::new(world, item_runtime);
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let Some((overlay, entries)) = hud_recipe_book_overlay_entries(
        world,
        Some(item_runtime),
        background,
        tabs,
        pages,
        search,
        overlay,
        partial_tick,
    ) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let (button_x, button_y) =
            recipe_book_overlay_entry_position(overlay.x, overlay.y, index, entries.len());
        for overlay_item in &entry.items {
            let projection = item_models.project(
                &overlay_item.stack,
                HudItemSlotState {
                    keybind: keybind_context,
                    ..HudItemSlotState::default()
                },
                None,
            );
            let Some(icon) = projection
                .as_ref()
                .and_then(|projection| projection.icon(partial_tick))
            else {
                continue;
            };
            items.push(HudInventoryItem {
                x: button_x + overlay_item.x,
                y: button_y + overlay_item.y,
                scale: RECIPE_BOOK_OVERLAY_ITEM_SCALE,
                scale_y: RECIPE_BOOK_OVERLAY_ITEM_SCALE,
                icon,
                scissor: None,
                draw_decorations: false,
                block_model: projection
                    .as_ref()
                    .and_then(|projection| projection.block_model(terrain_textures)),
            });
        }
    }
    items
}

#[allow(clippy::too_many_arguments)]
fn hud_recipe_book_overlay_entries<'a>(
    world: &'a WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    overlay: Option<RecipeBookOverlayHudState>,
    partial_tick: f32,
) -> Option<(RecipeBookOverlayHudState, Vec<RecipeBookOverlayEntry<'a>>)> {
    let overlay = overlay?;
    if recipe_book_type_for_background(background)? != overlay.book_type {
        return None;
    }
    if recipe_book_selected_tab_index(background, tabs)? != overlay.tab_index {
        return None;
    }
    if recipe_book_selected_page_index(background, pages)? != overlay.page_index {
        return None;
    }
    let collections = hud_recipe_book_collections(world, item_runtime, background, tabs, search)?;
    let page = clamped_recipe_book_page(overlay.page_index, collections.len());
    let collection_index = page * RECIPE_BOOK_ITEMS_PER_PAGE + overlay.button_index;
    let item_tag_entries = world.registry_tags("minecraft:item").map(|tags| &tags.tags);
    let entries = collections.get(collection_index)?.overlay_entries(
        item_tag_entries,
        recipe_book_slot_select_index(world, partial_tick),
    );
    (entries.len() > 1).then_some((overlay, entries))
}

fn recipe_book_overlay_button_texture(
    furnace_overlay: bool,
    craftable: bool,
    highlighted: bool,
) -> HudInventoryBackgroundTexture {
    HudInventoryBackgroundTexture::Sprite(match (furnace_overlay, craftable, highlighted) {
        (false, true, false) => "recipe_book/crafting_overlay",
        (false, true, true) => "recipe_book/crafting_overlay_highlighted",
        (false, false, false) => "recipe_book/crafting_overlay_disabled",
        (false, false, true) => "recipe_book/crafting_overlay_disabled_highlighted",
        (true, true, false) => "recipe_book/furnace_overlay",
        (true, true, true) => "recipe_book/furnace_overlay_highlighted",
        (true, false, false) => "recipe_book/furnace_overlay_disabled",
        (true, false, true) => "recipe_book/furnace_overlay_disabled_highlighted",
    })
}

pub(super) fn hud_recipe_book_ghost_layers_and_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    layout_slots: &[crate::input::InventorySlotLayout],
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> (Vec<HudInventoryFillLayer>, Vec<HudInventoryGhostItem>) {
    let item_models = HudItemModelContext::new(world, item_runtime);
    if item_runtime.is_none() {
        return (Vec::new(), Vec::new());
    }
    let Some(ghost_recipe) = world.last_ghost_recipe() else {
        return (Vec::new(), Vec::new());
    };
    if world.open_container_id() != Some(ghost_recipe.container_id) {
        return (Vec::new(), Vec::new());
    }
    if recipe_book_type_for_background(background).is_none() {
        return (Vec::new(), Vec::new());
    }
    let slot_select_index = recipe_book_slot_select_index(world, partial_tick);
    let mut fill_layers = Vec::new();
    let mut ghost_items = Vec::new();
    for ghost_slot in &ghost_recipe.slots {
        let Some(stack) = ghost_slot
            .items
            .get(slot_select_index % ghost_slot.items.len().max(1))
        else {
            continue;
        };
        let Some(layout_slot) = layout_slots
            .iter()
            .find(|layout_slot| layout_slot.slot_id == ghost_slot.slot_id)
        else {
            continue;
        };
        let Some(mut icon) = hud_item_icon(
            &item_models,
            stack,
            HudItemSlotState {
                keybind: keybind_context,
                ..HudItemSlotState::default()
            },
            None,
            partial_tick,
        ) else {
            continue;
        };
        if !ghost_slot.is_result {
            icon.count_label = None;
            icon.durability_bar = None;
            icon.cooldown_progress = None;
        }

        let (pre_fill_x, pre_fill_y, pre_fill_width, pre_fill_height) =
            if ghost_slot.is_result && recipe_book_result_slot_fill_is_big(background) {
                (layout_slot.x - 4, layout_slot.y - 4, 24, 24)
            } else {
                (layout_slot.x, layout_slot.y, 16, 16)
            };
        fill_layers.push(HudInventoryFillLayer {
            x: pre_fill_x,
            y: pre_fill_y,
            width: pre_fill_width,
            height: pre_fill_height,
            tint: RECIPE_BOOK_GHOST_PRE_ITEM_TINT,
            stage: HudInventoryFillStage::BeforeGhostItem,
        });
        ghost_items.push(HudInventoryGhostItem {
            x: layout_slot.x,
            y: layout_slot.y,
            icon,
            draw_decorations: ghost_slot.is_result,
        });
        fill_layers.push(HudInventoryFillLayer {
            x: layout_slot.x,
            y: layout_slot.y,
            width: 16,
            height: 16,
            tint: RECIPE_BOOK_GHOST_POST_ITEM_TINT,
            stage: HudInventoryFillStage::AfterGhostItem,
        });
    }

    (fill_layers, ghost_items)
}

fn recipe_book_result_slot_fill_is_big(background: InventoryScreenBackground) -> bool {
    matches!(background, InventoryScreenBackground::CraftingTable)
}

fn hud_recipe_book_visible_collections<'a>(
    world: &'a WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
) -> Vec<RecipeBookUiCollection<'a>> {
    let Some(collections) =
        hud_recipe_book_collections(world, item_runtime, background, tabs, search)
    else {
        return Vec::new();
    };
    let page = recipe_book_selected_page_index(background, pages)
        .map(|page| clamped_recipe_book_page(page, collections.len()))
        .unwrap_or_default();
    collections
        .into_iter()
        .skip(page * RECIPE_BOOK_ITEMS_PER_PAGE)
        .take(RECIPE_BOOK_ITEMS_PER_PAGE)
        .collect()
}

fn hud_recipe_book_collections<'a>(
    world: &'a WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    search: &RecipeBookSearchHudState,
) -> Option<Vec<RecipeBookUiCollection<'a>>> {
    if recipe_book_main_gui_offset(world, background) == 0 {
        return None;
    }
    let book_type = recipe_book_type_for_background(background)?;
    let selected_tab = recipe_book_selected_tab_index(background, tabs)?;
    let only_craftable = recipe_book_type_settings(world, book_type).filtering;
    if let Some(grid) = recipe_book_crafting_grid_for_background(background) {
        return Some(crafting_recipe_book_collections(
            world,
            grid,
            selected_tab,
            only_craftable,
            &search.edit.text(),
            item_runtime,
        ));
    }
    recipe_book_furnace_family_for_background(background).map(|family| {
        furnace_recipe_book_collections(
            world,
            family,
            selected_tab,
            only_craftable,
            &search.edit.text(),
            item_runtime,
        )
    })
}

fn recipe_book_crafting_grid_for_background(
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

fn recipe_book_furnace_family_for_background(
    background: InventoryScreenBackground,
) -> Option<RecipeBookFurnaceFamily> {
    match background {
        InventoryScreenBackground::Furnace => Some(RecipeBookFurnaceFamily::Furnace),
        InventoryScreenBackground::BlastFurnace => Some(RecipeBookFurnaceFamily::BlastFurnace),
        InventoryScreenBackground::Smoker => Some(RecipeBookFurnaceFamily::Smoker),
        _ => None,
    }
}

pub(super) fn hud_recipe_book_page_button_layers(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    cursor_position: Option<(i32, i32)>,
) -> Vec<HudInventoryBackgroundLayer> {
    let Some(collections) =
        hud_recipe_book_collections(world, item_runtime, background, tabs, search)
    else {
        return Vec::new();
    };
    let page_count = recipe_book_page_count(collections.len());
    if page_count <= 1 {
        return Vec::new();
    }
    let page = recipe_book_selected_page_index(background, pages)
        .map(|page| clamped_recipe_book_page(page, collections.len()))
        .unwrap_or_default();
    let mut layers = Vec::new();
    if page + 1 < page_count {
        layers.push(hud_recipe_book_page_button_layer(
            RecipeBookPageTurn::Next,
            cursor_position,
        ));
    }
    if page > 0 {
        layers.push(hud_recipe_book_page_button_layer(
            RecipeBookPageTurn::Previous,
            cursor_position,
        ));
    }
    layers
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecipeBookPageTurn {
    Previous,
    Next,
}

fn hud_recipe_book_page_button_layer(
    turn: RecipeBookPageTurn,
    cursor_position: Option<(i32, i32)>,
) -> HudInventoryBackgroundLayer {
    let x = match turn {
        RecipeBookPageTurn::Previous => RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X,
        RecipeBookPageTurn::Next => RECIPE_BOOK_PAGE_FORWARD_BUTTON_X,
    };
    let highlighted = cursor_position.is_some_and(|(cursor_x, cursor_y)| {
        cursor_x >= x
            && cursor_x < x + RECIPE_BOOK_PAGE_BUTTON_WIDTH
            && (RECIPE_BOOK_PAGE_BUTTON_Y
                ..RECIPE_BOOK_PAGE_BUTTON_Y + RECIPE_BOOK_PAGE_BUTTON_HEIGHT)
                .contains(&cursor_y)
    });
    hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Sprite(match (turn, highlighted) {
            (RecipeBookPageTurn::Previous, false) => "recipe_book/page_backward",
            (RecipeBookPageTurn::Previous, true) => "recipe_book/page_backward_highlighted",
            (RecipeBookPageTurn::Next, false) => "recipe_book/page_forward",
            (RecipeBookPageTurn::Next, true) => "recipe_book/page_forward_highlighted",
        }),
        x,
        RECIPE_BOOK_PAGE_BUTTON_Y,
        u32::try_from(RECIPE_BOOK_PAGE_BUTTON_WIDTH).unwrap_or_default(),
        u32::try_from(RECIPE_BOOK_PAGE_BUTTON_HEIGHT).unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    )
}

pub(super) fn hud_recipe_book_search_box_layer(
    world: &WorldStore,
    background: InventoryScreenBackground,
    search: &RecipeBookSearchHudState,
) -> Option<HudInventoryBackgroundLayer> {
    recipe_book_type_for_background(background)?;
    if recipe_book_main_gui_offset(world, background) == 0 {
        return None;
    }
    Some(hud_inventory_background_layer(
        if search.focused {
            HudInventoryBackgroundTexture::Sprite("widget/text_field_highlighted")
        } else {
            HudInventoryBackgroundTexture::Sprite("widget/text_field")
        },
        RECIPE_BOOK_SEARCH_BOX_X,
        RECIPE_BOOK_SEARCH_BOX_Y,
        u32::try_from(RECIPE_BOOK_SEARCH_BOX_WIDTH).unwrap_or_default(),
        u32::try_from(RECIPE_BOOK_SEARCH_BOX_HEIGHT).unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    ))
}

pub(super) fn hud_recipe_book_button_layer(
    background: InventoryScreenBackground,
    main_offset_x: i32,
    cursor_position: Option<(i32, i32)>,
) -> Option<HudInventoryBackgroundLayer> {
    let (button_x, button_y) = recipe_book_button_position(background)?;
    let x = main_offset_x + button_x;
    let highlighted = cursor_position.is_some_and(|(cursor_x, cursor_y)| {
        cursor_x >= x
            && cursor_x < x + RECIPE_BOOK_BUTTON_WIDTH
            && cursor_y >= button_y
            && cursor_y < button_y + RECIPE_BOOK_BUTTON_HEIGHT
    });
    Some(hud_inventory_background_layer(
        if highlighted {
            HudInventoryBackgroundTexture::Sprite("recipe_book/button_highlighted")
        } else {
            HudInventoryBackgroundTexture::Sprite("recipe_book/button")
        },
        x,
        button_y,
        u32::try_from(RECIPE_BOOK_BUTTON_WIDTH).unwrap_or_default(),
        u32::try_from(RECIPE_BOOK_BUTTON_HEIGHT).unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    ))
}

pub(super) fn hud_recipe_book_filter_button_layer(
    world: &WorldStore,
    background: InventoryScreenBackground,
    cursor_position: Option<(i32, i32)>,
) -> Option<HudInventoryBackgroundLayer> {
    let book_type = recipe_book_type_for_background(background)?;
    if recipe_book_main_gui_offset(world, background) == 0 {
        return None;
    }
    let filtering = recipe_book_type_settings(world, book_type).filtering;
    let highlighted = cursor_position.is_some_and(|(cursor_x, cursor_y)| {
        (RECIPE_BOOK_FILTER_BUTTON_X..RECIPE_BOOK_FILTER_BUTTON_X + RECIPE_BOOK_FILTER_BUTTON_WIDTH)
            .contains(&cursor_x)
            && (RECIPE_BOOK_FILTER_BUTTON_Y
                ..RECIPE_BOOK_FILTER_BUTTON_Y + RECIPE_BOOK_FILTER_BUTTON_HEIGHT)
                .contains(&cursor_y)
    });
    Some(hud_inventory_background_layer(
        hud_recipe_book_filter_texture(background, filtering, highlighted),
        RECIPE_BOOK_FILTER_BUTTON_X,
        RECIPE_BOOK_FILTER_BUTTON_Y,
        u32::try_from(RECIPE_BOOK_FILTER_BUTTON_WIDTH).unwrap_or_default(),
        u32::try_from(RECIPE_BOOK_FILTER_BUTTON_HEIGHT).unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    ))
}

fn hud_recipe_book_filter_texture(
    background: InventoryScreenBackground,
    filtering: bool,
    highlighted: bool,
) -> HudInventoryBackgroundTexture {
    let furnace_filter = matches!(
        background,
        InventoryScreenBackground::Furnace
            | InventoryScreenBackground::BlastFurnace
            | InventoryScreenBackground::Smoker
    );
    HudInventoryBackgroundTexture::Sprite(match (furnace_filter, filtering, highlighted) {
        (false, true, false) => "recipe_book/filter_enabled",
        (false, false, false) => "recipe_book/filter_disabled",
        (false, true, true) => "recipe_book/filter_enabled_highlighted",
        (false, false, true) => "recipe_book/filter_disabled_highlighted",
        (true, true, false) => "recipe_book/furnace_filter_enabled",
        (true, false, false) => "recipe_book/furnace_filter_disabled",
        (true, true, true) => "recipe_book/furnace_filter_enabled_highlighted",
        (true, false, true) => "recipe_book/furnace_filter_disabled_highlighted",
    })
}

pub(super) fn hud_recipe_book_search_text_label(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    search: &RecipeBookSearchHudState,
) -> Option<HudInventoryTextLabel> {
    recipe_book_type_for_background(background)?;
    if recipe_book_main_gui_offset(world, background) == 0 {
        return None;
    }
    let show_hint = search.edit.is_empty() && !search.focused;
    let text = if show_hint {
        localized_gui_text(item_runtime, "gui.recipebook.search_hint", "Search...")
    } else {
        search.edit.text()
    };
    let runs = if show_hint {
        vec![HudStyledTextRun {
            text: text.clone(),
            style: HudTextStyle {
                italic: true,
                ..Default::default()
            },
            color: Some(0xAA_AA_AA),
            shadow_color: None,
        }]
    } else {
        Vec::new()
    };
    Some(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: RECIPE_BOOK_SEARCH_BOX_X + RECIPE_BOOK_SEARCH_TEXT_X_OFFSET,
        y: RECIPE_BOOK_SEARCH_BOX_Y + RECIPE_BOOK_SEARCH_TEXT_Y_OFFSET,
        width: u32::try_from(RECIPE_BOOK_SEARCH_BOX_WIDTH - 8).unwrap_or_default(),
        text,
        tint: RECIPE_BOOK_SEARCH_TEXT_COLOR,
        background: None,
        input: (!show_hint).then(|| {
            search
                .edit
                .hud(search.focused, false)
                .decoration(RECIPE_BOOK_SEARCH_TEXT_COLOR)
        }),
        scrolling: None,
        shadow: show_hint,
        font: None,
        runs,
    })
}

pub(super) fn hud_recipe_book_page_text_label(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
) -> Option<HudInventoryTextLabel> {
    let collections = hud_recipe_book_collections(world, item_runtime, background, tabs, search)?;
    let page_count = recipe_book_page_count(collections.len());
    if page_count <= 1 {
        return None;
    }
    let page = recipe_book_selected_page_index(background, pages)
        .map(|page| clamped_recipe_book_page(page, collections.len()))
        .unwrap_or_default();
    let text = format!("{}/{}", page + 1, page_count);
    let width = hud_ascii_approx_text_width(&text)?;
    let width_i32 = i32::try_from(width).ok()?;
    Some(HudInventoryTextLabel {
        component_runs: Vec::new(),
        x: RECIPE_BOOK_PAGE_TEXT_CENTER_X - width_i32 / 2,
        y: RECIPE_BOOK_PAGE_TEXT_Y,
        width,
        text,
        tint: RECIPE_BOOK_SEARCH_TEXT_COLOR,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_recipe_book_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    overlay: Option<RecipeBookOverlayHudState>,
    cursor_position: Option<(i32, i32)>,
    tooltip_x_offset: i32,
    advanced: bool,
    creative: bool,
    partial_tick: f32,
) -> Option<HudInventoryTooltip> {
    if overlay.is_some() {
        return None;
    }
    let book_type = recipe_book_type_for_background(background)?;
    if !recipe_book_type_settings(world, book_type).open {
        return None;
    }
    let (cursor_x, cursor_y) = cursor_position?;
    let collections = hud_recipe_book_collections(world, item_runtime, background, tabs, search)?;
    let page = clamped_recipe_book_page(
        recipe_book_selected_page_index(background, pages).unwrap_or_default(),
        collections.len(),
    );
    for (index, collection) in collections
        .iter()
        .skip(page * RECIPE_BOOK_ITEMS_PER_PAGE)
        .take(RECIPE_BOOK_ITEMS_PER_PAGE)
        .enumerate()
    {
        let (x, y) = recipe_book_recipe_button_position(index);
        if cursor_x < x
            || cursor_x >= x + RECIPE_BOOK_RECIPE_BUTTON_SIZE
            || cursor_y < y
            || cursor_y >= y + RECIPE_BOOK_RECIPE_BUTTON_SIZE
        {
            continue;
        }
        let stack = collection.result_stack_at_slot_select_index(recipe_book_slot_select_index(
            world,
            partial_tick,
        ))?;
        let mut tooltip = hud_item_tooltip(world, item_runtime, stack, advanced, creative)?;
        if collection.has_multiple_recipes() {
            tooltip.lines.push(HudInventoryTooltipLine {
                text: localized_gui_text(
                    item_runtime,
                    "gui.recipebook.moreRecipes",
                    "Right Click for More",
                ),
                tint: [1.0, 1.0, 1.0, 1.0],
                runs: Vec::new(),
            });
        }
        return Some(HudInventoryTooltip {
            slot_id: u16::MAX,
            x: cursor_x + tooltip_x_offset,
            y: cursor_y,
            style: tooltip.style,
            lines: tooltip.lines,
        });
    }

    if (RECIPE_BOOK_FILTER_BUTTON_X..RECIPE_BOOK_FILTER_BUTTON_X + RECIPE_BOOK_FILTER_BUTTON_WIDTH)
        .contains(&cursor_x)
        && (RECIPE_BOOK_FILTER_BUTTON_Y
            ..RECIPE_BOOK_FILTER_BUTTON_Y + RECIPE_BOOK_FILTER_BUTTON_HEIGHT)
            .contains(&cursor_y)
    {
        let filtering = recipe_book_type_settings(world, book_type).filtering;
        let (key, fallback) = recipe_book_filter_tooltip_text(book_type, filtering);
        return Some(hud_recipe_book_control_tooltip(
            item_runtime,
            key,
            fallback,
            cursor_x + tooltip_x_offset,
            cursor_y,
        ));
    }

    let page_count = recipe_book_page_count(collections.len());
    if page + 1 < page_count
        && recipe_book_page_control_contains(cursor_x, cursor_y, RECIPE_BOOK_PAGE_FORWARD_BUTTON_X)
    {
        return Some(hud_recipe_book_control_tooltip(
            item_runtime,
            "gui.recipebook.next_page",
            "Next Page",
            cursor_x + tooltip_x_offset,
            cursor_y,
        ));
    }
    if page > 0
        && recipe_book_page_control_contains(cursor_x, cursor_y, RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X)
    {
        return Some(hud_recipe_book_control_tooltip(
            item_runtime,
            "gui.recipebook.previous_page",
            "Previous Page",
            cursor_x + tooltip_x_offset,
            cursor_y,
        ));
    }
    None
}

fn recipe_book_page_control_contains(cursor_x: i32, cursor_y: i32, button_x: i32) -> bool {
    cursor_x >= button_x
        && cursor_x < button_x + RECIPE_BOOK_PAGE_BUTTON_WIDTH
        && (RECIPE_BOOK_PAGE_BUTTON_Y..RECIPE_BOOK_PAGE_BUTTON_Y + RECIPE_BOOK_PAGE_BUTTON_HEIGHT)
            .contains(&cursor_y)
}

fn hud_recipe_book_control_tooltip(
    item_runtime: Option<&NativeItemRuntime>,
    translation_key: &str,
    fallback: &str,
    x: i32,
    y: i32,
) -> HudInventoryTooltip {
    HudInventoryTooltip {
        slot_id: u16::MAX,
        x,
        y,
        style: None,
        lines: vec![HudInventoryTooltipLine {
            text: localized_gui_text(item_runtime, translation_key, fallback),
            tint: [1.0, 1.0, 1.0, 1.0],
            runs: Vec::new(),
        }],
    }
}

pub(super) fn recipe_book_narration_frame(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
    partial_tick: f32,
) -> Option<ScreenNarrationFrame> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    let book_type = recipe_book_type_for_background(layout.background)?;
    if !recipe_book_type_settings(world, book_type).open {
        return None;
    }
    let search = input.recipe_book_search_hud_state();
    let mut frame = ScreenNarrationFrame::default();
    if search.focused {
        frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "gui.narrate.editBox",
                "%s edit box: %s",
                &[
                    &localized_gui_text(item_runtime, "itemGroup.search", "Search Items"),
                    &search.edit.text(),
                ],
            ),
        );
        return Some(frame);
    }

    let recipe_book_x = layout
        .recipe_book
        .map(|recipe_book| recipe_book.x)
        .unwrap_or_default();
    let cursor = input
        .inventory_cursor_position()
        .map(|(x, y)| (x - recipe_book_x, y));
    let Some((cursor_x, cursor_y)) = cursor else {
        return Some(frame);
    };
    let tabs = input.recipe_book_tab_selection_hud_state();
    let pages = input.recipe_book_page_hud_state();
    let collections =
        hud_recipe_book_collections(world, item_runtime, layout.background, &tabs, &search)?;
    let page = clamped_recipe_book_page(
        recipe_book_selected_page_index(layout.background, &pages).unwrap_or_default(),
        collections.len(),
    );

    if input.recipe_book_overlay_hud_state().is_none() {
        for (index, collection) in collections
            .iter()
            .skip(page * RECIPE_BOOK_ITEMS_PER_PAGE)
            .take(RECIPE_BOOK_ITEMS_PER_PAGE)
            .enumerate()
        {
            let (x, y) = recipe_book_recipe_button_position(index);
            if cursor_x < x
                || cursor_x >= x + RECIPE_BOOK_RECIPE_BUTTON_SIZE
                || cursor_y < y
                || cursor_y >= y + RECIPE_BOOK_RECIPE_BUTTON_SIZE
            {
                continue;
            }
            let stack = collection.result_stack_at_slot_select_index(
                recipe_book_slot_select_index(world, partial_tick),
            )?;
            let item_name = hud_item_tooltip(world, item_runtime, stack, false, false)?
                .lines
                .into_iter()
                .next()?
                .text;
            frame.add(
                NarratedElementKind::Title,
                1,
                localized_gui_text_with_args(
                    item_runtime,
                    "narration.recipe",
                    "Recipe for %s",
                    &[&item_name],
                ),
            );
            frame.add(
                NarratedElementKind::Usage,
                1,
                localized_gui_text(
                    item_runtime,
                    "narration.button.usage.hovered",
                    "Left click to activate",
                ),
            );
            if collection.has_multiple_recipes() {
                frame.add(
                    NarratedElementKind::Usage,
                    2,
                    localized_gui_text(
                        item_runtime,
                        "narration.recipe.usage.more",
                        "Right click to show more recipes",
                    ),
                );
            }
            return Some(frame);
        }
    }

    if (RECIPE_BOOK_SEARCH_BOX_X..RECIPE_BOOK_SEARCH_BOX_X + RECIPE_BOOK_SEARCH_BOX_WIDTH)
        .contains(&cursor_x)
        && (RECIPE_BOOK_SEARCH_BOX_Y..RECIPE_BOOK_SEARCH_BOX_Y + RECIPE_BOOK_SEARCH_BOX_HEIGHT)
            .contains(&cursor_y)
    {
        frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "gui.narrate.editBox",
                "%s edit box: %s",
                &[
                    &localized_gui_text(item_runtime, "itemGroup.search", "Search Items"),
                    &search.edit.text(),
                ],
            ),
        );
        return Some(frame);
    }

    if (RECIPE_BOOK_FILTER_BUTTON_X..RECIPE_BOOK_FILTER_BUTTON_X + RECIPE_BOOK_FILTER_BUTTON_WIDTH)
        .contains(&cursor_x)
        && (RECIPE_BOOK_FILTER_BUTTON_Y
            ..RECIPE_BOOK_FILTER_BUTTON_Y + RECIPE_BOOK_FILTER_BUTTON_HEIGHT)
            .contains(&cursor_y)
    {
        let filtering = recipe_book_type_settings(world, book_type).filtering;
        let (key, fallback) = recipe_book_filter_tooltip_text(book_type, filtering);
        add_recipe_book_button_narration(
            &mut frame,
            item_runtime,
            &localized_gui_text(item_runtime, key, fallback),
        );
        return Some(frame);
    }

    let visible_tabs = recipe_book_visible_tab_indices(world, layout.background);
    for (visible_index, _tab_index) in visible_tabs.into_iter().enumerate() {
        let tab_y = RECIPE_BOOK_TAB_Y + RECIPE_BOOK_TAB_STRIDE_Y * visible_index as i32;
        if (RECIPE_BOOK_TAB_X..RECIPE_BOOK_TAB_X + RECIPE_BOOK_TAB_WIDTH).contains(&cursor_x)
            && cursor_y >= tab_y
            && cursor_y < tab_y + RECIPE_BOOK_TAB_HEIGHT
        {
            return Some(frame);
        }
    }

    let page_count = recipe_book_page_count(collections.len());
    if page + 1 < page_count
        && recipe_book_page_control_contains(cursor_x, cursor_y, RECIPE_BOOK_PAGE_FORWARD_BUTTON_X)
    {
        add_recipe_book_button_narration(
            &mut frame,
            item_runtime,
            &localized_gui_text(item_runtime, "gui.recipebook.next_page", "Next Page"),
        );
    } else if page > 0
        && recipe_book_page_control_contains(cursor_x, cursor_y, RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X)
    {
        add_recipe_book_button_narration(
            &mut frame,
            item_runtime,
            &localized_gui_text(
                item_runtime,
                "gui.recipebook.previous_page",
                "Previous Page",
            ),
        );
    }
    Some(frame)
}

fn recipe_book_filter_tooltip_text(
    book_type: RecipeBookType,
    filtering: bool,
) -> (&'static str, &'static str) {
    if !filtering {
        return ("gui.recipebook.toggleRecipes.all", "Showing All");
    }
    match book_type {
        RecipeBookType::Crafting => (
            "gui.recipebook.toggleRecipes.craftable",
            "Showing Craftable",
        ),
        RecipeBookType::Furnace => (
            "gui.recipebook.toggleRecipes.smeltable",
            "Showing Smeltable",
        ),
        RecipeBookType::BlastFurnace => (
            "gui.recipebook.toggleRecipes.blastable",
            "Showing Blastable",
        ),
        RecipeBookType::Smoker => ("gui.recipebook.toggleRecipes.smokable", "Showing Smokable"),
    }
}

fn add_recipe_book_button_narration(
    frame: &mut ScreenNarrationFrame,
    item_runtime: Option<&NativeItemRuntime>,
    label: &str,
) {
    frame.add(
        NarratedElementKind::Title,
        1,
        localized_gui_text_with_args(item_runtime, "gui.narrate.button", "%s button", &[label]),
    );
    frame.add(
        NarratedElementKind::Usage,
        1,
        localized_gui_text(
            item_runtime,
            "narration.button.usage.hovered",
            "Left click to activate",
        ),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_recipe_book_ghost_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    hovered_slot_id: Option<i16>,
    layout_slots: &[crate::input::InventorySlotLayout],
    advanced: bool,
    creative: bool,
    partial_tick: f32,
) -> Option<HudInventoryTooltip> {
    let book_type = recipe_book_type_for_background(background)?;
    if !recipe_book_type_settings(world, book_type).open {
        return None;
    }
    let ghost_recipe = world.last_ghost_recipe()?;
    if world.open_container_id() != Some(ghost_recipe.container_id) {
        return None;
    }
    let slot_id = hovered_slot_id?;
    let ghost_slot = ghost_recipe
        .slots
        .iter()
        .find(|slot| slot.slot_id == slot_id)?;
    let slot_select_index = recipe_book_slot_select_index(world, partial_tick);
    let item = ghost_slot
        .items
        .get(slot_select_index % ghost_slot.items.len().max(1))?;
    let layout = layout_slots
        .iter()
        .find(|layout| layout.slot_id == slot_id)?;
    let tooltip = hud_item_tooltip(world, item_runtime, item, advanced, creative)?;
    Some(HudInventoryTooltip {
        slot_id: u16::try_from(slot_id).ok()?,
        x: layout.x,
        y: layout.y,
        style: tooltip.style,
        lines: tooltip.lines,
    })
}
