use super::*;

pub(super) const ADVANCEMENTS_WINDOW_WIDTH: u32 = 252;
pub(super) const ADVANCEMENTS_WINDOW_HEIGHT: u32 = 140;
pub(super) const ADVANCEMENTS_WINDOW_INSIDE_X: i32 = 9;
pub(super) const ADVANCEMENTS_WINDOW_INSIDE_Y: i32 = 18;
pub(super) const ADVANCEMENTS_WINDOW_INSIDE_WIDTH: u32 = 234;
pub(super) const ADVANCEMENTS_WINDOW_INSIDE_HEIGHT: u32 = 113;
pub(super) const ADVANCEMENTS_WINDOW_TITLE_X: i32 = 8;
pub(super) const ADVANCEMENTS_WINDOW_TITLE_Y: i32 = 6;
pub(super) const ADVANCEMENTS_EMPTY_TEXT_CENTER_X: i32 = 126;
pub(super) const ADVANCEMENTS_EMPTY_TEXT_Y: i32 = 70;
pub(super) const ADVANCEMENTS_SAD_TEXT_Y: i32 = 122;
const ADVANCEMENTS_HEADER_FOOTER_HEIGHT: i32 = 33;
pub(super) const ADVANCEMENTS_DONE_BUTTON_WIDTH: u32 = 200;
pub(super) const ADVANCEMENTS_DONE_BUTTON_HEIGHT: u32 = 20;
pub(super) const ADVANCEMENTS_DONE_BUTTON_TEXT_Y_OFFSET: i32 = 6;
pub(super) const ADVANCEMENTS_TITLE_TEXT: &str = "Advancements";
pub(super) const ADVANCEMENTS_EMPTY_TEXT: &str = "There doesn't seem to be anything here...";
pub(super) const ADVANCEMENTS_SAD_TEXT: &str = ":(";
pub(super) const ADVANCEMENTS_DONE_TEXT: &str = "Done";
pub(super) const ADVANCEMENTS_TITLE_TEXT_COLOR: [f32; 4] = rgba32(-12_566_464);
pub(super) const ADVANCEMENTS_EMPTY_TEXT_COLOR: [f32; 4] = rgba32(-1);
pub(super) const ADVANCEMENTS_EMPTY_BACKGROUND_TINT: [f32; 4] = rgba32(-16_777_216);
const ADVANCEMENTS_BACKGROUND_TILE_SIZE: i32 = 16;
const ADVANCEMENTS_TAB_ABOVE_ICON_OFFSET: (i32, i32) = (6, 9);
const ADVANCEMENTS_TAB_BELOW_ICON_OFFSET: (i32, i32) = (6, 6);
const ADVANCEMENTS_TAB_LEFT_ICON_OFFSET: (i32, i32) = (10, 5);
const ADVANCEMENTS_TAB_RIGHT_ICON_OFFSET: (i32, i32) = (6, 5);
pub(super) const ADVANCEMENTS_WIDGET_FRAME_OFFSET_X: i32 = 3;
pub(super) const ADVANCEMENTS_WIDGET_FRAME_WIDTH: u32 = 26;
pub(super) const ADVANCEMENTS_WIDGET_FRAME_HEIGHT: u32 = 26;
const ADVANCEMENTS_WIDGET_ICON_OFFSET: (i32, i32) = (8, 5);
const ADVANCEMENTS_WIDGET_ICON_SIZE: i32 = 16;
const ADVANCEMENTS_WIDGET_BOUNDS_WIDTH: i32 = 28;
const ADVANCEMENTS_WIDGET_BOUNDS_HEIGHT: i32 = 27;
pub(super) const ADVANCEMENTS_HOVER_MAX_FADE: f32 = 0.3;
pub(super) const ADVANCEMENTS_HOVER_BOX_SOURCE_WIDTH: u32 = 200;
pub(super) const ADVANCEMENTS_HOVER_FRAME_SIZE: u32 = 26;
pub(super) const ADVANCEMENTS_HOVER_TITLE_MIN_WIDTH: u32 = 80;
pub(super) const ADVANCEMENTS_HOVER_TITLE_MAX_WIDTH: u32 = 163;
pub(super) const ADVANCEMENTS_HOVER_TITLE_X: i32 = 32;
pub(super) const ADVANCEMENTS_HOVER_TITLE_PADDING_LEFT: u32 = 3;
pub(super) const ADVANCEMENTS_HOVER_TITLE_PADDING_RIGHT: u32 = 5;
pub(super) const ADVANCEMENTS_HOVER_TITLE_PADDING_TOP: i32 = 9;
pub(super) const ADVANCEMENTS_HOVER_TITLE_PADDING_BOTTOM: i32 = 8;
pub(super) const ADVANCEMENTS_HOVER_DESCRIPTION_PADDING_TOP: i32 = 6;
pub(super) const ADVANCEMENTS_HOVER_LINE_HEIGHT: i32 = 9;
pub(super) const ADVANCEMENTS_HOVER_PROGRESS_PADDING: u32 = 8;
pub(super) const ADVANCEMENTS_HOVER_PROGRESS_PREFIX_WIDTH: u32 = 29;
pub(super) const ADVANCEMENTS_HOVER_RIGHT_MARGIN: i32 = 6;
pub(super) const ADVANCEMENTS_HOVER_TITLE_TEXT_COLOR: [f32; 4] = rgba32(-1);
const ADVANCEMENTS_HOVER_TASK_DESCRIPTION_TEXT_COLOR: [f32; 4] =
    [85.0 / 255.0, 1.0, 85.0 / 255.0, 1.0];
pub(super) const ADVANCEMENTS_HOVER_CHALLENGE_DESCRIPTION_TEXT_COLOR: [f32; 4] =
    [170.0 / 255.0, 0.0, 170.0 / 255.0, 1.0];
pub(super) const ADVANCEMENTS_HOVER_FADE_TINT: [f32; 4] =
    [0.0, 0.0, 0.0, ADVANCEMENTS_HOVER_MAX_FADE];
#[allow(clippy::too_many_arguments)]
pub(super) fn hud_advancements_screen(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    advancement_scroll_delta: Option<(f64, f64)>,
    advancement_hover_fade: f32,
    surface_size: winit::dpi::PhysicalSize<u32>,
    cursor_position: Option<(i32, i32)>,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
    fonts: &HudFontGlyphBook,
) -> HudInventoryScreen {
    let screen_width = surface_size.width.max(1);
    let screen_height = surface_size.height.max(1);
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let (done_button_x, done_button_y) = advancements_done_button_origin_for_surface(surface_size);
    let root_tabs = world.advancement_root_tabs();
    let selected_tab = world.selected_advancement_root_tab();
    let selected_widgets = world.selected_advancement_widgets();
    let selected_tab_id = selected_tab.as_ref().map(|tab| tab.id.as_str());
    let show_root_tabs = root_tabs.len() > 1;
    let done_button_highlighted = cursor_position.is_some_and(|(cursor_x, cursor_y)| {
        cursor_x >= done_button_x
            && cursor_x < done_button_x + ADVANCEMENTS_DONE_BUTTON_WIDTH as i32
            && cursor_y >= done_button_y
            && cursor_y < done_button_y + ADVANCEMENTS_DONE_BUTTON_HEIGHT as i32
    });
    let done_button_texture = if done_button_highlighted {
        HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
    } else {
        HudInventoryBackgroundTexture::Sprite("widget/button")
    };
    let mut background_layers = vec![hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Texture("textures/gui/advancements/window.png"),
        window_x,
        window_y,
        ADVANCEMENTS_WINDOW_WIDTH,
        ADVANCEMENTS_WINDOW_HEIGHT,
        [0.0, 0.0],
        [
            ADVANCEMENTS_WINDOW_WIDTH as f32 / 256.0,
            ADVANCEMENTS_WINDOW_HEIGHT as f32 / 256.0,
        ],
    )];
    background_layers.extend(advancements_background_layers(
        selected_tab.as_ref(),
        &selected_widgets,
        advancement_scroll_delta,
        window_x,
        window_y,
    ));
    background_layers.extend(advancements_widget_connection_layers(
        &selected_widgets,
        advancement_scroll_delta,
        window_x,
        window_y,
    ));
    background_layers.extend(advancements_widget_frame_layers(
        &selected_widgets,
        advancement_scroll_delta,
        window_x,
        window_y,
    ));
    background_layers.push(hud_inventory_background_layer(
        done_button_texture,
        done_button_x,
        done_button_y,
        ADVANCEMENTS_DONE_BUTTON_WIDTH,
        ADVANCEMENTS_DONE_BUTTON_HEIGHT,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
    if show_root_tabs {
        background_layers.extend(advancements_tab_background_layers(
            &root_tabs,
            selected_tab_id,
            window_x,
            window_y,
        ));
    }
    let mut floating_items = Vec::new();
    if show_root_tabs {
        floating_items.extend(advancements_tab_icon_items(
            world,
            item_runtime,
            terrain_textures,
            &root_tabs,
            window_x,
            window_y,
            keybind_context,
            partial_tick,
        ));
    }
    floating_items.extend(advancements_widget_icon_items(
        world,
        item_runtime,
        terrain_textures,
        &selected_widgets,
        advancement_scroll_delta,
        window_x,
        window_y,
        keybind_context,
        partial_tick,
    ));
    let hover = advancements_hover_projection(
        world,
        item_runtime,
        terrain_textures,
        &selected_widgets,
        advancement_scroll_delta,
        advancement_hover_fade,
        window_x,
        window_y,
        cursor_position,
        screen_width,
        keybind_context,
        partial_tick,
        fonts,
    );
    let mut fill_layers = if selected_tab.is_some() {
        Vec::new()
    } else {
        vec![HudInventoryFillLayer {
            x: window_x + ADVANCEMENTS_WINDOW_INSIDE_X,
            y: window_y + ADVANCEMENTS_WINDOW_INSIDE_Y,
            width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
            height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
            tint: ADVANCEMENTS_EMPTY_BACKGROUND_TINT,
            stage: HudInventoryFillStage::BeforeGhostItem,
        }]
    };
    fill_layers.extend(hover.fill_layers);
    let mut text_labels = advancements_screen_text_labels(
        selected_tab.as_ref().map(|tab| &tab.title),
        window_x,
        window_y,
        done_button_x,
        done_button_y,
        fonts,
        item_runtime,
    );
    push_centered_advancements_label(
        &mut text_labels,
        ("gui.advancements", ADVANCEMENTS_TITLE_TEXT),
        screen_width as i32 / 2,
        (ADVANCEMENTS_HEADER_FOOTER_HEIGHT - 9) / 2,
        fonts,
        item_runtime,
    );
    text_labels.last_mut().unwrap().shadow = true;
    text_labels.extend(hover.text_labels);
    HudInventoryScreen {
        width: screen_width,
        height: screen_height,
        background_layers,
        foreground_layers: hover.background_layers,
        fill_layers,
        slots: Vec::new(),
        floating_items,
        foreground_items: hover.foreground_items,
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels,
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    }
}

impl AdvancementTabType {
    fn icon_offset(self) -> (i32, i32) {
        match self {
            Self::Above => ADVANCEMENTS_TAB_ABOVE_ICON_OFFSET,
            Self::Below => ADVANCEMENTS_TAB_BELOW_ICON_OFFSET,
            Self::Left => ADVANCEMENTS_TAB_LEFT_ICON_OFFSET,
            Self::Right => ADVANCEMENTS_TAB_RIGHT_ICON_OFFSET,
        }
    }

    fn sprite(self, index: usize, selected: bool) -> HudAdvancementTabSprite {
        let first = index == 0;
        let last = index == self.max() - 1;
        match (self, selected, first, last) {
            (Self::Above, true, true, _) => HudAdvancementTabSprite::AboveLeftSelected,
            (Self::Above, true, _, true) => HudAdvancementTabSprite::AboveRightSelected,
            (Self::Above, true, _, _) => HudAdvancementTabSprite::AboveMiddleSelected,
            (Self::Above, false, true, _) => HudAdvancementTabSprite::AboveLeft,
            (Self::Above, false, _, true) => HudAdvancementTabSprite::AboveRight,
            (Self::Above, false, _, _) => HudAdvancementTabSprite::AboveMiddle,
            (Self::Below, true, true, _) => HudAdvancementTabSprite::BelowLeftSelected,
            (Self::Below, true, _, true) => HudAdvancementTabSprite::BelowRightSelected,
            (Self::Below, true, _, _) => HudAdvancementTabSprite::BelowMiddleSelected,
            (Self::Below, false, true, _) => HudAdvancementTabSprite::BelowLeft,
            (Self::Below, false, _, true) => HudAdvancementTabSprite::BelowRight,
            (Self::Below, false, _, _) => HudAdvancementTabSprite::BelowMiddle,
            (Self::Left, true, true, _) => HudAdvancementTabSprite::LeftTopSelected,
            (Self::Left, true, _, true) => HudAdvancementTabSprite::LeftBottomSelected,
            (Self::Left, true, _, _) => HudAdvancementTabSprite::LeftMiddleSelected,
            (Self::Left, false, true, _) => HudAdvancementTabSprite::LeftTop,
            (Self::Left, false, _, true) => HudAdvancementTabSprite::LeftBottom,
            (Self::Left, false, _, _) => HudAdvancementTabSprite::LeftMiddle,
            (Self::Right, true, true, _) => HudAdvancementTabSprite::RightTopSelected,
            (Self::Right, true, _, true) => HudAdvancementTabSprite::RightBottomSelected,
            (Self::Right, true, _, _) => HudAdvancementTabSprite::RightMiddleSelected,
            (Self::Right, false, true, _) => HudAdvancementTabSprite::RightTop,
            (Self::Right, false, _, true) => HudAdvancementTabSprite::RightBottom,
            (Self::Right, false, _, _) => HudAdvancementTabSprite::RightMiddle,
        }
    }
}

fn advancement_tab_geometry(
    display_index: usize,
    window_x: i32,
    window_y: i32,
) -> Option<(AdvancementTabType, usize, i32, i32, u32, u32)> {
    let (tab_type, tab_index) = AdvancementTabType::for_display_index(display_index)?;
    let (x, y) = tab_type.origin(tab_index);
    Some((
        tab_type,
        tab_index,
        window_x + x,
        window_y + y,
        tab_type.width(),
        tab_type.height(),
    ))
}

fn advancements_tab_background_layers(
    root_tabs: &[bbb_client_world::AdvancementRootTabSummary],
    selected_tab_id: Option<&str>,
    window_x: i32,
    window_y: i32,
) -> Vec<HudInventoryBackgroundLayer> {
    let mut layers = Vec::new();
    for tab in root_tabs {
        let Some((tab_type, tab_index, x, y, width, height)) =
            advancement_tab_geometry(tab.display_index, window_x, window_y)
        else {
            continue;
        };
        let selected = selected_tab_id == Some(tab.id.as_str());
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite(
                tab_type.sprite(tab_index, selected).sprite_path(),
            ),
            x,
            y,
            width,
            height,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }
    layers
}

fn advancements_background_layers(
    selected_tab: Option<&bbb_client_world::AdvancementRootTabSummary>,
    widgets: &[bbb_client_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
    window_x: i32,
    window_y: i32,
) -> Vec<HudInventoryBackgroundLayer> {
    let Some(selected_tab) = selected_tab else {
        return Vec::new();
    };
    let texture = selected_tab
        .background
        .as_deref()
        .and_then(HudAdvancementBackgroundTexture::from_resource_id)
        .unwrap_or(HudAdvancementBackgroundTexture::Missing);
    let (scroll_x, scroll_y) = advancements_widget_scroll(widgets, scroll_delta).unwrap_or((0, 0));
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let left = scroll_x % ADVANCEMENTS_BACKGROUND_TILE_SIZE;
    let top = scroll_y % ADVANCEMENTS_BACKGROUND_TILE_SIZE;
    let mut layers = Vec::new();
    for tile_x in -1..=15 {
        for tile_y in -1..=8 {
            let x = inside_x + left + ADVANCEMENTS_BACKGROUND_TILE_SIZE * tile_x;
            let y = inside_y + top + ADVANCEMENTS_BACKGROUND_TILE_SIZE * tile_y;
            if let Some(layer) =
                clipped_advancement_background_tile(texture, x, y, inside_x, inside_y)
            {
                layers.push(layer);
            }
        }
    }
    layers
}

fn clipped_advancement_background_tile(
    texture: HudAdvancementBackgroundTexture,
    tile_x: i32,
    tile_y: i32,
    inside_x: i32,
    inside_y: i32,
) -> Option<HudInventoryBackgroundLayer> {
    let inside_right = inside_x + ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32;
    let inside_bottom = inside_y + ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32;
    let tile_right = tile_x + ADVANCEMENTS_BACKGROUND_TILE_SIZE;
    let tile_bottom = tile_y + ADVANCEMENTS_BACKGROUND_TILE_SIZE;
    let x = tile_x.max(inside_x);
    let y = tile_y.max(inside_y);
    let right = tile_right.min(inside_right);
    let bottom = tile_bottom.min(inside_bottom);
    if x >= right || y >= bottom {
        return None;
    }

    let uv_min = [
        (x - tile_x) as f32 / ADVANCEMENTS_BACKGROUND_TILE_SIZE as f32,
        (y - tile_y) as f32 / ADVANCEMENTS_BACKGROUND_TILE_SIZE as f32,
    ];
    let uv_max = [
        (right - tile_x) as f32 / ADVANCEMENTS_BACKGROUND_TILE_SIZE as f32,
        (bottom - tile_y) as f32 / ADVANCEMENTS_BACKGROUND_TILE_SIZE as f32,
    ];
    Some(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::advancement_background(texture),
        x,
        y,
        (right - x) as u32,
        (bottom - y) as u32,
        uv_min,
        uv_max,
    ))
}

pub(super) fn advancements_widget_connection_layers(
    widgets: &[bbb_client_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
    window_x: i32,
    window_y: i32,
) -> Vec<HudInventoryBackgroundLayer> {
    let Some((scroll_x, scroll_y)) = advancements_widget_scroll(widgets, scroll_delta) else {
        return Vec::new();
    };
    let by_id: BTreeMap<&str, &bbb_client_world::AdvancementWidgetSummary> = widgets
        .iter()
        .map(|widget| (widget.id.as_str(), widget))
        .collect();
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let mut layers = Vec::new();
    for widget in widgets {
        let Some(parent_id) = widget.parent_id.as_deref() else {
            continue;
        };
        let Some(parent) = by_id.get(parent_id).copied() else {
            continue;
        };
        let dep_x = inside_x + scroll_x + parent.x + 13;
        let split_x = inside_x + scroll_x + parent.x + 26 + 4;
        let dep_y = inside_y + scroll_y + parent.y + 13;
        let my_x = inside_x + scroll_x + widget.x + 13;
        let my_y = inside_y + scroll_y + widget.y + 13;
        push_advancement_connection_layers(
            &mut layers,
            HudAdvancementLineTexture::Background,
            dep_x,
            split_x,
            dep_y,
            my_x,
            my_y,
            inside_x,
            inside_y,
        );
        push_advancement_connection_layers(
            &mut layers,
            HudAdvancementLineTexture::Foreground,
            dep_x,
            split_x,
            dep_y,
            my_x,
            my_y,
            inside_x,
            inside_y,
        );
    }
    layers
}

#[allow(clippy::too_many_arguments)]
fn push_advancement_connection_layers(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    texture: HudAdvancementLineTexture,
    dep_x: i32,
    split_x: i32,
    dep_y: i32,
    my_x: i32,
    my_y: i32,
    inside_x: i32,
    inside_y: i32,
) {
    if matches!(texture, HudAdvancementLineTexture::Background) {
        push_advancement_horizontal_line(
            layers,
            texture,
            split_x,
            dep_x,
            dep_y - 1,
            inside_x,
            inside_y,
        );
        push_advancement_horizontal_line(
            layers,
            texture,
            split_x + 1,
            dep_x,
            dep_y,
            inside_x,
            inside_y,
        );
        push_advancement_horizontal_line(
            layers,
            texture,
            split_x,
            dep_x,
            dep_y + 1,
            inside_x,
            inside_y,
        );
        push_advancement_horizontal_line(
            layers,
            texture,
            my_x,
            split_x - 1,
            my_y - 1,
            inside_x,
            inside_y,
        );
        push_advancement_horizontal_line(
            layers,
            texture,
            my_x,
            split_x - 1,
            my_y,
            inside_x,
            inside_y,
        );
        push_advancement_horizontal_line(
            layers,
            texture,
            my_x,
            split_x - 1,
            my_y + 1,
            inside_x,
            inside_y,
        );
        push_advancement_vertical_line(
            layers,
            texture,
            split_x - 1,
            my_y,
            dep_y,
            inside_x,
            inside_y,
        );
        push_advancement_vertical_line(
            layers,
            texture,
            split_x + 1,
            my_y,
            dep_y,
            inside_x,
            inside_y,
        );
    } else {
        push_advancement_horizontal_line(
            layers, texture, split_x, dep_x, dep_y, inside_x, inside_y,
        );
        push_advancement_horizontal_line(layers, texture, my_x, split_x, my_y, inside_x, inside_y);
        push_advancement_vertical_line(layers, texture, split_x, my_y, dep_y, inside_x, inside_y);
    }
}

fn push_advancement_horizontal_line(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    texture: HudAdvancementLineTexture,
    mut x0: i32,
    mut x1: i32,
    y: i32,
    inside_x: i32,
    inside_y: i32,
) {
    if x1 < x0 {
        std::mem::swap(&mut x0, &mut x1);
    }
    push_clipped_advancement_line_layer(layers, texture, x0, y, x1 + 1, y + 1, inside_x, inside_y);
}

fn push_advancement_vertical_line(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    texture: HudAdvancementLineTexture,
    x: i32,
    mut y0: i32,
    mut y1: i32,
    inside_x: i32,
    inside_y: i32,
) {
    if y1 < y0 {
        std::mem::swap(&mut y0, &mut y1);
    }
    push_clipped_advancement_line_layer(layers, texture, x, y0 + 1, x + 1, y1, inside_x, inside_y);
}

#[allow(clippy::too_many_arguments)]
fn push_clipped_advancement_line_layer(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    texture: HudAdvancementLineTexture,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    inside_x: i32,
    inside_y: i32,
) {
    let inside_right = inside_x + ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32;
    let inside_bottom = inside_y + ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32;
    let x = x0.max(inside_x);
    let y = y0.max(inside_y);
    let right = x1.min(inside_right);
    let bottom = y1.min(inside_bottom);
    if x >= right || y >= bottom {
        return;
    }
    layers.push(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::AdvancementLine(texture),
        x,
        y,
        (right - x) as u32,
        (bottom - y) as u32,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
}

#[allow(clippy::too_many_arguments)]
fn advancements_tab_icon_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    root_tabs: &[bbb_client_world::AdvancementRootTabSummary],
    window_x: i32,
    window_y: i32,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let item_models = HudItemModelContext::new(world, item_runtime);
    if item_runtime.is_none() {
        return Vec::new();
    }
    let mut items = Vec::new();
    for tab in root_tabs {
        let Some((tab_type, _, x, y, _, _)) =
            advancement_tab_geometry(tab.display_index, window_x, window_y)
        else {
            continue;
        };
        let stack = ItemStackSummary {
            item_id: Some(tab.icon.item_id),
            count: tab.icon.count,
            component_patch: tab.icon.component_patch.clone(),
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
            continue;
        };
        let (icon_x, icon_y) = tab_type.icon_offset();
        items.push(HudInventoryItem {
            x: x + icon_x,
            y: y + icon_y,
            scale: 1.0,
            scale_y: 1.0,
            icon,
            scissor: None,
            draw_decorations: false,
            block_model: projection
                .as_ref()
                .and_then(|projection| projection.block_model(terrain_textures)),
        });
    }
    items
}

fn advancements_widget_frame_layers(
    widgets: &[bbb_client_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
    window_x: i32,
    window_y: i32,
) -> Vec<HudInventoryBackgroundLayer> {
    let mut layers = Vec::new();
    let Some((scroll_x, scroll_y)) = advancements_widget_scroll(widgets, scroll_delta) else {
        return layers;
    };
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    for widget in widgets
        .iter()
        .filter(|widget| advancement_widget_visible(widget))
    {
        let frame_x = inside_x + scroll_x + widget.x + ADVANCEMENTS_WIDGET_FRAME_OFFSET_X;
        let frame_y = inside_y + scroll_y + widget.y;
        if let Some(layer) = clipped_advancement_widget_frame_layer(
            advancement_widget_frame_sprite(widget.frame_type, widget.done),
            frame_x,
            frame_y,
            inside_x,
            inside_y,
        ) {
            layers.push(layer);
        }
    }
    layers
}

pub(super) fn clipped_advancement_widget_frame_layer(
    sprite: HudAdvancementWidgetFrameSprite,
    frame_x: i32,
    frame_y: i32,
    inside_x: i32,
    inside_y: i32,
) -> Option<HudInventoryBackgroundLayer> {
    let inside_right = inside_x + ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32;
    let inside_bottom = inside_y + ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32;
    let frame_right = frame_x + ADVANCEMENTS_WIDGET_FRAME_WIDTH as i32;
    let frame_bottom = frame_y + ADVANCEMENTS_WIDGET_FRAME_HEIGHT as i32;
    let x = frame_x.max(inside_x);
    let y = frame_y.max(inside_y);
    let right = frame_right.min(inside_right);
    let bottom = frame_bottom.min(inside_bottom);
    if x >= right || y >= bottom {
        return None;
    }
    let uv_min = [
        (x - frame_x) as f32 / ADVANCEMENTS_WIDGET_FRAME_WIDTH as f32,
        (y - frame_y) as f32 / ADVANCEMENTS_WIDGET_FRAME_HEIGHT as f32,
    ];
    let uv_max = [
        (right - frame_x) as f32 / ADVANCEMENTS_WIDGET_FRAME_WIDTH as f32,
        (bottom - frame_y) as f32 / ADVANCEMENTS_WIDGET_FRAME_HEIGHT as f32,
    ];
    Some(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Sprite(sprite.sprite_path()),
        x,
        y,
        (right - x) as u32,
        (bottom - y) as u32,
        uv_min,
        uv_max,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn advancements_widget_icon_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    widgets: &[bbb_client_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
    window_x: i32,
    window_y: i32,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let item_models = HudItemModelContext::new(world, item_runtime);
    if item_runtime.is_none() {
        return Vec::new();
    }
    let Some((scroll_x, scroll_y)) = advancements_widget_scroll(widgets, scroll_delta) else {
        return Vec::new();
    };
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let mut items = Vec::new();
    for widget in widgets
        .iter()
        .filter(|widget| advancement_widget_visible(widget))
    {
        let (icon_x, icon_y) = ADVANCEMENTS_WIDGET_ICON_OFFSET;
        let x = inside_x + scroll_x + widget.x + icon_x;
        let y = inside_y + scroll_y + widget.y + icon_y;
        if !advancement_widget_icon_intersects_content(x, y, inside_x, inside_y) {
            continue;
        }
        let stack = ItemStackSummary {
            item_id: Some(widget.icon.item_id),
            count: widget.icon.count,
            component_patch: widget.icon.component_patch.clone(),
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
            continue;
        };
        items.push(HudInventoryItem {
            x,
            y,
            scale: 1.0,
            scale_y: 1.0,
            icon,
            scissor: Some(advancement_widget_content_scissor(window_x, window_y)),
            draw_decorations: false,
            block_model: projection
                .as_ref()
                .and_then(|projection| projection.block_model(terrain_textures)),
        });
    }
    items
}

fn advancement_widget_content_scissor(window_x: i32, window_y: i32) -> HudInventoryItemScissor {
    HudInventoryItemScissor {
        x: ADVANCEMENTS_WINDOW_INSIDE_X + window_x,
        y: ADVANCEMENTS_WINDOW_INSIDE_Y + window_y,
        width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
        height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
    }
}

fn advancement_widget_icon_intersects_content(
    x: i32,
    y: i32,
    inside_x: i32,
    inside_y: i32,
) -> bool {
    let right = x + ADVANCEMENTS_WIDGET_ICON_SIZE;
    let bottom = y + ADVANCEMENTS_WIDGET_ICON_SIZE;
    right > inside_x
        && x < inside_x + ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32
        && bottom > inside_y
        && y < inside_y + ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32
}

#[derive(Debug, Default)]
struct AdvancementHoverProjection {
    fill_layers: Vec<HudInventoryFillLayer>,
    background_layers: Vec<HudInventoryBackgroundLayer>,
    text_labels: Vec<HudInventoryTextLabel>,
    foreground_items: Vec<HudInventoryItem>,
}

#[allow(clippy::too_many_arguments)]
fn advancements_hover_projection(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    widgets: &[bbb_client_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
    hover_fade: f32,
    window_x: i32,
    window_y: i32,
    cursor_position: Option<(i32, i32)>,
    screen_width: u32,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
    fonts: &HudFontGlyphBook,
) -> AdvancementHoverProjection {
    let mut projection = AdvancementHoverProjection::default();
    let Some((scroll_x, scroll_y)) = advancements_widget_scroll(widgets, scroll_delta) else {
        return projection;
    };
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let hover_fade_tint = advancement_hover_fade_tint(hover_fade);
    if hover_fade_tint[3] > 0.0 {
        projection.fill_layers.push(HudInventoryFillLayer {
            x: inside_x,
            y: inside_y,
            width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
            height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
            tint: hover_fade_tint,
            stage: HudInventoryFillStage::Foreground,
        });
    }
    let Some(widget) = advancement_hovered_widget(
        widgets,
        scroll_x,
        scroll_y,
        inside_x,
        inside_y,
        cursor_position,
    ) else {
        return projection;
    };
    push_advancement_hover_boxes_and_text(
        &mut projection.background_layers,
        &mut projection.text_labels,
        widget,
        scroll_x,
        scroll_y,
        window_x,
        inside_x,
        inside_y,
        screen_width,
        fonts,
        item_runtime,
    );
    if let Some(item) = advancement_hover_icon_item(
        world,
        item_runtime,
        terrain_textures,
        widget,
        scroll_x,
        scroll_y,
        inside_x,
        inside_y,
        keybind_context,
        partial_tick,
    ) {
        projection.foreground_items.push(item);
    }
    projection
}

fn advancement_hover_fade_tint(hover_fade: f32) -> [f32; 4] {
    let alpha = if hover_fade.is_finite() {
        hover_fade.clamp(0.0, ADVANCEMENTS_HOVER_MAX_FADE)
    } else {
        0.0
    };
    let mut tint = ADVANCEMENTS_HOVER_FADE_TINT;
    tint[3] = alpha;
    tint
}

pub(super) fn advancement_hovered_widget(
    widgets: &[bbb_client_world::AdvancementWidgetSummary],
    scroll_x: i32,
    scroll_y: i32,
    inside_x: i32,
    inside_y: i32,
    cursor_position: Option<(i32, i32)>,
) -> Option<&bbb_client_world::AdvancementWidgetSummary> {
    let (cursor_x, cursor_y) = cursor_position?;
    let mouse_x = cursor_x - inside_x;
    let mouse_y = cursor_y - inside_y;
    if mouse_x <= 0
        || mouse_x >= ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32
        || mouse_y <= 0
        || mouse_y >= ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32
    {
        return None;
    }
    widgets.iter().find(|widget| {
        if !advancement_widget_visible(widget) {
            return false;
        }
        let x0 = scroll_x + widget.x;
        let y0 = scroll_y + widget.y;
        mouse_x >= x0
            && mouse_x <= x0 + ADVANCEMENTS_HOVER_FRAME_SIZE as i32
            && mouse_y >= y0
            && mouse_y <= y0 + ADVANCEMENTS_HOVER_FRAME_SIZE as i32
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_advancement_hover_progress_box_segment(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    sprite: HudAdvancementHoverBoxSprite,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    source_x0: u32,
    source_x1: u32,
) {
    if width == 0 {
        return;
    }
    let source_width = ADVANCEMENTS_HOVER_BOX_SOURCE_WIDTH.max(1) as f32;
    layers.push(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Sprite(sprite.sprite_path()),
        x,
        y,
        width,
        height,
        [source_x0 as f32 / source_width, 0.0],
        [source_x1 as f32 / source_width, 1.0],
    ));
}

pub(super) fn advancement_hover_progress_box_state(
    widget: &bbb_client_world::AdvancementWidgetSummary,
    tooltip_width: u32,
) -> (
    HudAdvancementHoverBoxSprite,
    HudAdvancementHoverBoxSprite,
    u32,
    bool,
) {
    let amount = advancement_progress_percent(widget);
    let mut first_half_width = (amount * tooltip_width as f32).floor() as u32;
    if amount >= 1.0 {
        first_half_width = tooltip_width / 2;
        (
            HudAdvancementHoverBoxSprite::Obtained,
            HudAdvancementHoverBoxSprite::Obtained,
            first_half_width,
            true,
        )
    } else if first_half_width < 2 {
        first_half_width = tooltip_width / 2;
        (
            HudAdvancementHoverBoxSprite::Unobtained,
            HudAdvancementHoverBoxSprite::Unobtained,
            first_half_width,
            false,
        )
    } else if first_half_width > tooltip_width.saturating_sub(2) {
        first_half_width = tooltip_width / 2;
        (
            HudAdvancementHoverBoxSprite::Obtained,
            HudAdvancementHoverBoxSprite::Obtained,
            first_half_width,
            false,
        )
    } else {
        (
            HudAdvancementHoverBoxSprite::Obtained,
            HudAdvancementHoverBoxSprite::Unobtained,
            first_half_width,
            false,
        )
    }
}

fn advancement_progress_percent(widget: &bbb_client_world::AdvancementWidgetSummary) -> f32 {
    if widget.progress_total == 0 {
        0.0
    } else {
        (widget.progress_done as f32 / widget.progress_total as f32).clamp(0.0, 1.0)
    }
}

pub(super) fn advancement_hover_description_text_color(
    frame_type: AdvancementFrameType,
) -> [f32; 4] {
    match frame_type {
        AdvancementFrameType::Task | AdvancementFrameType::Goal => {
            ADVANCEMENTS_HOVER_TASK_DESCRIPTION_TEXT_COLOR
        }
        AdvancementFrameType::Challenge => ADVANCEMENTS_HOVER_CHALLENGE_DESCRIPTION_TEXT_COLOR,
    }
}

#[allow(clippy::too_many_arguments)]
fn advancement_hover_icon_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    widget: &bbb_client_world::AdvancementWidgetSummary,
    scroll_x: i32,
    scroll_y: i32,
    inside_x: i32,
    inside_y: i32,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Option<HudInventoryItem> {
    let item_models = HudItemModelContext::new(world, item_runtime);
    item_runtime?;
    let stack = ItemStackSummary {
        item_id: Some(widget.icon.item_id),
        count: widget.icon.count,
        component_patch: widget.icon.component_patch.clone(),
    };
    let projection = item_models.project(
        &stack,
        HudItemSlotState {
            keybind: keybind_context,
            ..HudItemSlotState::default()
        },
        None,
    );
    let icon = projection
        .as_ref()
        .and_then(|projection| projection.icon(partial_tick))?;
    let (icon_x, icon_y) = ADVANCEMENTS_WIDGET_ICON_OFFSET;
    Some(HudInventoryItem {
        x: inside_x + scroll_x + widget.x + icon_x,
        y: inside_y + scroll_y + widget.y + icon_y,
        scale: 1.0,
        scale_y: 1.0,
        icon,
        scissor: None,
        draw_decorations: false,
        block_model: projection
            .as_ref()
            .and_then(|projection| projection.block_model(terrain_textures)),
    })
}

pub(super) fn advancements_widget_scroll(
    widgets: &[bbb_client_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
) -> Option<(i32, i32)> {
    let mut iter = widgets.iter();
    let first = iter.next()?;
    let bounds_w = i64::from(ADVANCEMENTS_WIDGET_BOUNDS_WIDTH);
    let bounds_h = i64::from(ADVANCEMENTS_WIDGET_BOUNDS_HEIGHT);
    let inside_w = i64::from(ADVANCEMENTS_WINDOW_INSIDE_WIDTH);
    let inside_h = i64::from(ADVANCEMENTS_WINDOW_INSIDE_HEIGHT);
    let mut min_x = i64::from(first.x);
    let mut max_x = i64::from(first.x) + bounds_w;
    let mut min_y = i64::from(first.y);
    let mut max_y = i64::from(first.y) + bounds_h;
    for widget in iter {
        let wx = i64::from(widget.x);
        let wy = i64::from(widget.y);
        min_x = min_x.min(wx);
        max_x = max_x.max(wx + bounds_w);
        min_y = min_y.min(wy);
        max_y = max_y.max(wy + bounds_h);
    }
    let center_x = inside_w / 2 - (max_x + min_x) / 2;
    let center_y = inside_h / 2 - (max_y + min_y) / 2;
    let (raw_dx, raw_dy) = scroll_delta.unwrap_or((0.0, 0.0));
    let delta_x = if raw_dx.is_finite() { raw_dx } else { 0.0 };
    let delta_y = if raw_dy.is_finite() { raw_dy } else { 0.0 };
    // Vanilla `AdvancementTab.java:166-174` uses `Mth.clamp(v, -(max - inside), 0.0)`.
    // Guard `min <= max` when negative/offset trees have `max < inside`.
    let scroll_x = if max_x - min_x > inside_w {
        let min_scroll = (-((max_x - inside_w) as f64)).min(0.0);
        (center_x as f64 + delta_x).clamp(min_scroll, 0.0)
    } else {
        center_x as f64
    };
    let scroll_y = if max_y - min_y > inside_h {
        let min_scroll = (-((max_y - inside_h) as f64)).min(0.0);
        (center_y as f64 + delta_y).clamp(min_scroll, 0.0)
    } else {
        center_y as f64
    };
    let x = scroll_x
        .floor()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
    let y = scroll_y
        .floor()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
    Some((x, y))
}

fn advancement_widget_visible(widget: &bbb_client_world::AdvancementWidgetSummary) -> bool {
    !widget.hidden || widget.done
}

pub(super) fn advancement_widget_frame_sprite(
    frame_type: AdvancementFrameType,
    done: bool,
) -> HudAdvancementWidgetFrameSprite {
    match (frame_type, done) {
        (AdvancementFrameType::Task, true) => HudAdvancementWidgetFrameSprite::TaskObtained,
        (AdvancementFrameType::Task, false) => HudAdvancementWidgetFrameSprite::TaskUnobtained,
        (AdvancementFrameType::Challenge, true) => {
            HudAdvancementWidgetFrameSprite::ChallengeObtained
        }
        (AdvancementFrameType::Challenge, false) => {
            HudAdvancementWidgetFrameSprite::ChallengeUnobtained
        }
        (AdvancementFrameType::Goal, true) => HudAdvancementWidgetFrameSprite::GoalObtained,
        (AdvancementFrameType::Goal, false) => HudAdvancementWidgetFrameSprite::GoalUnobtained,
    }
}

pub(crate) fn advancements_window_origin_for_surface(
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> (i32, i32) {
    let screen_width = i32::try_from(surface_size.width.max(1)).unwrap_or(i32::MAX);
    let screen_height = i32::try_from(surface_size.height.max(1)).unwrap_or(i32::MAX);
    (
        (screen_width - ADVANCEMENTS_WINDOW_WIDTH as i32) / 2,
        (screen_height - ADVANCEMENTS_WINDOW_HEIGHT as i32) / 2,
    )
}

pub(crate) fn advancements_done_button_origin_for_surface(
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> (i32, i32) {
    let screen_width = i32::try_from(surface_size.width.max(1)).unwrap_or(i32::MAX);
    let screen_height = i32::try_from(surface_size.height.max(1)).unwrap_or(i32::MAX);
    // HeaderAndFooterLayout, 99-102; AbstractLayout.
    (
        (screen_width - ADVANCEMENTS_DONE_BUTTON_WIDTH as i32) / 2,
        screen_height - ADVANCEMENTS_HEADER_FOOTER_HEIGHT
            + (ADVANCEMENTS_HEADER_FOOTER_HEIGHT - ADVANCEMENTS_DONE_BUTTON_HEIGHT as i32 + 1) / 2,
    )
}

pub(super) fn advancements_screen_text_labels(
    selected_title: Option<&RichTextComponent>,
    window_x: i32,
    window_y: i32,
    done_button_x: i32,
    done_button_y: i32,
    fonts: &HudFontGlyphBook,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudInventoryTextLabel> {
    let component_runs = selected_title
        .map(|title| {
            let raw = localized_component_runs(title, &ComponentStyle::default(), item_runtime)
                .into_iter()
                .map(component_hover::hud_component_run)
                .collect::<Vec<_>>();
            bbb_client_render::hud_resolve_formatted_component_text(&raw)
        })
        .unwrap_or_else(|| {
            advancement_text::advancement_label_runs(
                "gui.advancements",
                ADVANCEMENTS_TITLE_TEXT,
                item_runtime,
            )
        });
    let title = component_runs
        .iter()
        .map(|run| run.visual.text.as_str())
        .collect::<String>();
    let width = bbb_client_render::hud_component_text_width(&component_runs, fonts).max(0) as u32;
    let mut labels = vec![HudInventoryTextLabel {
        component_runs,
        x: window_x + ADVANCEMENTS_WINDOW_TITLE_X,
        y: window_y + ADVANCEMENTS_WINDOW_TITLE_Y,
        width,
        text: title,
        tint: ADVANCEMENTS_TITLE_TEXT_COLOR,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    }];
    if selected_title.is_none() {
        push_centered_advancements_label(
            &mut labels,
            ("advancements.empty", ADVANCEMENTS_EMPTY_TEXT),
            window_x + ADVANCEMENTS_EMPTY_TEXT_CENTER_X,
            window_y + ADVANCEMENTS_EMPTY_TEXT_Y,
            fonts,
            item_runtime,
        );
        push_centered_advancements_label(
            &mut labels,
            ("advancements.sad_label", ADVANCEMENTS_SAD_TEXT),
            window_x + ADVANCEMENTS_EMPTY_TEXT_CENTER_X,
            window_y + ADVANCEMENTS_SAD_TEXT_Y,
            fonts,
            item_runtime,
        );
    }
    push_centered_advancements_label(
        &mut labels,
        ("gui.done", ADVANCEMENTS_DONE_TEXT),
        done_button_x + ADVANCEMENTS_DONE_BUTTON_WIDTH as i32 / 2,
        done_button_y + ADVANCEMENTS_DONE_BUTTON_TEXT_Y_OFFSET,
        fonts,
        item_runtime,
    );
    labels
}

fn push_centered_advancements_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    translation: (&str, &str),
    center_x: i32,
    y: i32,
    fonts: &HudFontGlyphBook,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let component_runs =
        advancement_text::advancement_label_runs(translation.0, translation.1, item_runtime);
    let text = component_runs
        .iter()
        .map(|run| run.visual.text.as_str())
        .collect::<String>();
    let width_i32 = bbb_client_render::hud_component_text_width(&component_runs, fonts);
    let width = width_i32.max(0) as u32;
    labels.push(HudInventoryTextLabel {
        component_runs,
        x: center_x - width_i32 / 2,
        y,
        width,
        text: text.to_string(),
        tint: ADVANCEMENTS_EMPTY_TEXT_COLOR,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    });
}

#[cfg(test)]
mod scroll_tests {
    use super::*;

    #[test]
    fn advancements_widget_scroll_handles_negative_and_extreme_bounds_without_panic() {
        let make = |x: i32, y: i32| bbb_client_world::AdvancementWidgetSummary {
            id: format!("test:{x}_{y}"),
            parent_id: None,
            title: bbb_protocol::RichTextComponent::plain("t"),
            description: bbb_protocol::RichTextComponent::plain("d"),
            icon: bbb_protocol::packets::AdvancementIconSummary {
                item_id: 1,
                count: 1,
                component_patch: bbb_protocol::packets::DataComponentPatchSummary::default().into(),
            },
            frame_type: AdvancementFrameType::Task,
            hidden: false,
            x,
            y,
            done: false,
            progress_done: 0,
            progress_total: 1,
        };
        // Negative offset tree where max_x < 234 yet span > 234 previously panicked in f64::clamp.
        let scroll =
            advancements_widget_scroll(&[make(-400, -300), make(-50, -20)], Some((10.0, -5.0)));
        assert!(scroll.is_some());
        // Extreme i32 bounds and non-finite scroll delta must not overflow or panic.
        let extreme = advancements_widget_scroll(
            &[make(i32::MIN, i32::MIN), make(i32::MAX, i32::MAX)],
            Some((f64::NAN, f64::INFINITY)),
        );
        assert!(extreme.is_some());
    }
}
