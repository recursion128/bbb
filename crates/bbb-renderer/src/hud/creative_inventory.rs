use super::{HudInventoryBackgroundLayer, HudInventoryBackgroundTexture, HudUvRect};

pub const HUD_CREATIVE_INVENTORY_WIDTH: u32 = 195;
pub const HUD_CREATIVE_INVENTORY_HEIGHT: u32 = 136;
pub const HUD_CREATIVE_TAB_WIDTH: u32 = 26;
pub const HUD_CREATIVE_TAB_HEIGHT: u32 = 32;
pub const HUD_CREATIVE_SEARCH_X: i32 = 82;
pub const HUD_CREATIVE_SEARCH_Y: i32 = 6;
pub const HUD_CREATIVE_SEARCH_WIDTH: u32 = 80;
pub const HUD_CREATIVE_SEARCH_HEIGHT: u32 = 9;
pub const HUD_CREATIVE_SCROLLER_X: i32 = 175;
pub const HUD_CREATIVE_SCROLLER_Y: i32 = 18;
const HUD_CREATIVE_SCROLLER_WIDTH: u32 = 12;
const HUD_CREATIVE_SCROLLER_HEIGHT: u32 = 15;
const HUD_CREATIVE_SCROLLER_MAX_OFFSET: i32 = 95;
pub const HUD_CREATIVE_TRASH_X: i32 = 173;
pub const HUD_CREATIVE_TRASH_Y: i32 = 112;
pub const HUD_CREATIVE_PICKER_COLUMNS: usize = 9;
pub const HUD_CREATIVE_PICKER_ROWS: usize = 5;
pub const HUD_CREATIVE_PICKER_SLOT_COUNT: usize =
    HUD_CREATIVE_PICKER_COLUMNS * HUD_CREATIVE_PICKER_ROWS;
const HUD_CREATIVE_SLOT_SIZE: i32 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudCreativeInventoryBackground {
    Items,
    ItemSearch,
    Inventory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudCreativeInventoryTabRow {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudCreativeInventoryTab {
    pub row: HudCreativeInventoryTabRow,
    pub column: u8,
    pub aligned_right: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudCreativeInventoryChrome {
    pub background: HudCreativeInventoryBackground,
    pub tabs: Vec<HudCreativeInventoryTab>,
    pub can_scroll: bool,
    pub scroll_enabled: bool,
    pub scroller_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudCreativeInventorySlot {
    Picker(usize),
    Hotbar(usize),
    Inventory(i16),
    Trash,
}

impl HudCreativeInventorySlot {
    pub fn hud_id(self) -> u16 {
        match self {
            Self::Picker(index) => u16::try_from(index).unwrap_or_default(),
            Self::Hotbar(index) => 45 + u16::try_from(index).unwrap_or_default(),
            Self::Inventory(slot) => 64 + u16::try_from(slot.max(0)).unwrap_or_default(),
            Self::Trash => 127,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudCreativeInventorySlotLayout {
    pub slot: HudCreativeInventorySlot,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudCreativeInventorySlotLayoutKind {
    Picker,
    Inventory,
}

pub fn hud_creative_inventory_slot_layouts(
    kind: HudCreativeInventorySlotLayoutKind,
) -> Vec<HudCreativeInventorySlotLayout> {
    if kind == HudCreativeInventorySlotLayoutKind::Inventory {
        let mut slots = Vec::with_capacity(42);
        for slot in 5_i16..=45 {
            let (x, y) = creative_inventory_player_slot_position(slot);
            slots.push(HudCreativeInventorySlotLayout {
                slot: HudCreativeInventorySlot::Inventory(slot),
                x,
                y,
            });
        }
        slots.push(HudCreativeInventorySlotLayout {
            slot: HudCreativeInventorySlot::Trash,
            x: HUD_CREATIVE_TRASH_X,
            y: HUD_CREATIVE_TRASH_Y,
        });
        return slots;
    }

    let mut slots = Vec::with_capacity(HUD_CREATIVE_PICKER_SLOT_COUNT + 9);
    for index in 0..HUD_CREATIVE_PICKER_SLOT_COUNT {
        slots.push(HudCreativeInventorySlotLayout {
            slot: HudCreativeInventorySlot::Picker(index),
            x: 9 + i32::try_from(index % HUD_CREATIVE_PICKER_COLUMNS).unwrap_or_default()
                * HUD_CREATIVE_SLOT_SIZE,
            y: 18
                + i32::try_from(index / HUD_CREATIVE_PICKER_COLUMNS).unwrap_or_default()
                    * HUD_CREATIVE_SLOT_SIZE,
        });
    }
    for index in 0..9 {
        slots.push(HudCreativeInventorySlotLayout {
            slot: HudCreativeInventorySlot::Hotbar(index),
            x: 9 + i32::try_from(index).unwrap_or_default() * HUD_CREATIVE_SLOT_SIZE,
            y: 112,
        });
    }
    slots
}

pub fn hud_creative_inventory_tab_x(column: u8, aligned_right: bool) -> i32 {
    if aligned_right {
        i32::try_from(HUD_CREATIVE_INVENTORY_WIDTH).unwrap_or_default()
            - 27 * (7 - i32::from(column))
            + 1
    } else {
        27 * i32::from(column)
    }
}

pub fn hud_creative_inventory_tab_hit_y(row: HudCreativeInventoryTabRow) -> i32 {
    match row {
        HudCreativeInventoryTabRow::Top => -32,
        HudCreativeInventoryTabRow::Bottom => {
            i32::try_from(HUD_CREATIVE_INVENTORY_HEIGHT).unwrap_or_default()
        }
    }
}

pub fn hud_creative_inventory_scroller_y(scroll_row: usize, max_scroll_row: usize) -> i32 {
    if max_scroll_row == 0 {
        return HUD_CREATIVE_SCROLLER_Y;
    }
    let fraction = scroll_row.min(max_scroll_row) as f32 / max_scroll_row as f32;
    HUD_CREATIVE_SCROLLER_Y + (fraction * HUD_CREATIVE_SCROLLER_MAX_OFFSET as f32) as i32
}

/// Projects the exact Vanilla CreativeModeInventoryScreen chrome ordering:
/// unselected tabs, the selected background, the scroller, then the selected tab.
pub fn hud_creative_inventory_background_layers(
    chrome: &HudCreativeInventoryChrome,
) -> Vec<HudInventoryBackgroundLayer> {
    let mut layers = Vec::with_capacity(chrome.tabs.len() + 2);
    for tab in chrome.tabs.iter().filter(|tab| !tab.selected) {
        layers.push(tab_layer(*tab));
    }
    layers.push(HudInventoryBackgroundLayer {
        texture: match chrome.background {
            HudCreativeInventoryBackground::Items => HudInventoryBackgroundTexture::CreativeItems,
            HudCreativeInventoryBackground::ItemSearch => {
                HudInventoryBackgroundTexture::CreativeItemSearch
            }
            HudCreativeInventoryBackground::Inventory => {
                HudInventoryBackgroundTexture::CreativeInventory
            }
        },
        x: 0,
        y: 0,
        width: HUD_CREATIVE_INVENTORY_WIDTH,
        height: HUD_CREATIVE_INVENTORY_HEIGHT,
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [
                HUD_CREATIVE_INVENTORY_WIDTH as f32 / 256.0,
                HUD_CREATIVE_INVENTORY_HEIGHT as f32 / 256.0,
            ],
        },
    });
    if chrome.can_scroll {
        layers.push(HudInventoryBackgroundLayer {
            texture: if chrome.scroll_enabled {
                HudInventoryBackgroundTexture::CreativeScroller
            } else {
                HudInventoryBackgroundTexture::CreativeScrollerDisabled
            },
            x: HUD_CREATIVE_SCROLLER_X,
            y: chrome.scroller_y,
            width: HUD_CREATIVE_SCROLLER_WIDTH,
            height: HUD_CREATIVE_SCROLLER_HEIGHT,
            uv: full_uv(),
        });
    }
    if let Some(tab) = chrome.tabs.iter().find(|tab| tab.selected) {
        layers.push(tab_layer(*tab));
    }
    layers
}

fn tab_layer(tab: HudCreativeInventoryTab) -> HudInventoryBackgroundLayer {
    let x = hud_creative_inventory_tab_x(tab.column, tab.aligned_right);
    let y = match tab.row {
        HudCreativeInventoryTabRow::Top => -28,
        HudCreativeInventoryTabRow::Bottom => {
            i32::try_from(HUD_CREATIVE_INVENTORY_HEIGHT).unwrap_or_default() - 4
        }
    };
    HudInventoryBackgroundLayer {
        texture: HudInventoryBackgroundTexture::CreativeTab {
            row: tab.row,
            selected: tab.selected,
            column: tab.column,
        },
        x,
        y,
        width: HUD_CREATIVE_TAB_WIDTH,
        height: HUD_CREATIVE_TAB_HEIGHT,
        uv: full_uv(),
    }
}

fn creative_inventory_player_slot_position(slot: i16) -> (i32, i32) {
    match slot {
        5..=8 => {
            let position = i32::from(slot - 5);
            (54 + (position / 2) * 54, 6 + (position.rem_euclid(2)) * 27)
        }
        45 => (35, 20),
        _ => {
            let position = i32::from(slot - 9);
            let x = 9 + position.rem_euclid(9) * HUD_CREATIVE_SLOT_SIZE;
            let y = if slot >= 36 {
                112
            } else {
                54 + (position / 9) * HUD_CREATIVE_SLOT_SIZE
            };
            (x, y)
        }
    }
}

fn full_uv() -> HudUvRect {
    HudUvRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    }
}

#[cfg(test)]
mod tests {
    use crate::{HudInventoryScreen, Renderer};

    use super::*;

    #[test]
    fn slot_tab_and_scroller_geometry_matches_vanilla() {
        let picker =
            hud_creative_inventory_slot_layouts(HudCreativeInventorySlotLayoutKind::Picker);
        assert_eq!(picker.len(), 54);
        assert_eq!((picker[0].x, picker[0].y), (9, 18));
        assert_eq!((picker[44].x, picker[44].y), (153, 90));
        assert_eq!((picker[45].x, picker[45].y), (9, 112));
        assert_eq!((picker[53].x, picker[53].y), (153, 112));

        let inventory =
            hud_creative_inventory_slot_layouts(HudCreativeInventorySlotLayoutKind::Inventory);
        assert_eq!(inventory.len(), 42);
        let position = |slot| {
            inventory
                .iter()
                .find(|layout| layout.slot == HudCreativeInventorySlot::Inventory(slot))
                .map(|layout| (layout.x, layout.y))
                .unwrap()
        };
        assert_eq!(position(5), (54, 6));
        assert_eq!(position(8), (108, 33));
        assert_eq!(position(9), (9, 54));
        assert_eq!(position(36), (9, 112));
        assert_eq!(position(45), (35, 20));
        assert_eq!(
            inventory.last().unwrap(),
            &HudCreativeInventorySlotLayout {
                slot: HudCreativeInventorySlot::Trash,
                x: 173,
                y: 112,
            }
        );

        assert_eq!(hud_creative_inventory_tab_x(0, false), 0);
        assert_eq!(hud_creative_inventory_tab_x(6, true), 169);
        assert_eq!(
            hud_creative_inventory_tab_hit_y(HudCreativeInventoryTabRow::Top),
            -32
        );
        assert_eq!(
            hud_creative_inventory_tab_hit_y(HudCreativeInventoryTabRow::Bottom),
            136
        );
        assert_eq!(hud_creative_inventory_scroller_y(0, 6), 18);
        assert_eq!(hud_creative_inventory_scroller_y(3, 6), 65);
        assert_eq!(hud_creative_inventory_scroller_y(6, 6), 113);
    }

    #[test]
    fn layers_preserve_vanilla_chrome_order_and_geometry() {
        let layers = hud_creative_inventory_background_layers(&HudCreativeInventoryChrome {
            background: HudCreativeInventoryBackground::ItemSearch,
            tabs: vec![
                HudCreativeInventoryTab {
                    row: HudCreativeInventoryTabRow::Top,
                    column: 0,
                    aligned_right: false,
                    selected: false,
                },
                HudCreativeInventoryTab {
                    row: HudCreativeInventoryTabRow::Top,
                    column: 6,
                    aligned_right: true,
                    selected: true,
                },
                HudCreativeInventoryTab {
                    row: HudCreativeInventoryTabRow::Bottom,
                    column: 6,
                    aligned_right: true,
                    selected: false,
                },
            ],
            can_scroll: true,
            scroll_enabled: true,
            scroller_y: 113,
        });
        assert_eq!(layers.len(), 5);
        assert_eq!(
            layers[0].texture,
            HudInventoryBackgroundTexture::CreativeTab {
                row: HudCreativeInventoryTabRow::Top,
                selected: false,
                column: 0,
            }
        );
        assert_eq!((layers[0].x, layers[0].y), (0, -28));
        assert_eq!(
            layers[1].texture,
            HudInventoryBackgroundTexture::CreativeTab {
                row: HudCreativeInventoryTabRow::Bottom,
                selected: false,
                column: 6,
            }
        );
        assert_eq!((layers[1].x, layers[1].y), (169, 132));
        assert_eq!(
            layers[2].texture,
            HudInventoryBackgroundTexture::CreativeItemSearch
        );
        assert_eq!(
            layers[2].uv,
            HudUvRect {
                min: [0.0, 0.0],
                max: [195.0 / 256.0, 136.0 / 256.0],
            }
        );
        assert_eq!(
            layers[3].texture,
            HudInventoryBackgroundTexture::CreativeScroller
        );
        assert_eq!((layers[3].x, layers[3].y), (175, 113));
        assert_eq!(
            layers[4].texture,
            HudInventoryBackgroundTexture::CreativeTab {
                row: HudCreativeInventoryTabRow::Top,
                selected: true,
                column: 6,
            }
        );
    }

    #[test]
    fn disabled_scroller_stays_at_track_origin() {
        let layers = hud_creative_inventory_background_layers(&HudCreativeInventoryChrome {
            background: HudCreativeInventoryBackground::Items,
            tabs: Vec::new(),
            can_scroll: true,
            scroll_enabled: false,
            scroller_y: 18,
        });
        assert_eq!(layers.len(), 2);
        assert_eq!(
            layers[1].texture,
            HudInventoryBackgroundTexture::CreativeScrollerDisabled
        );
        assert_eq!(layers[1].y, 18);
    }

    #[test]
    fn creative_chrome_renders_deterministic_offscreen_layer_pixels() {
        let Some(mut renderer) = Renderer::new_offscreen(240, 200) else {
            return;
        };
        renderer
            .upload_hud_creative_item_search_background(
                195,
                136,
                &[255, 0, 0, 255].repeat(195 * 136),
            )
            .unwrap();
        renderer
            .upload_hud_creative_scroller(false, 12, 15, &[0, 0, 255, 255].repeat(12 * 15))
            .unwrap();
        renderer
            .upload_hud_creative_tab(
                HudCreativeInventoryTabRow::Top,
                true,
                6,
                26,
                32,
                &[0, 255, 0, 255].repeat(26 * 32),
            )
            .unwrap();
        let background_layers =
            hud_creative_inventory_background_layers(&HudCreativeInventoryChrome {
                background: HudCreativeInventoryBackground::ItemSearch,
                tabs: vec![HudCreativeInventoryTab {
                    row: HudCreativeInventoryTabRow::Top,
                    column: 6,
                    aligned_right: true,
                    selected: true,
                }],
                can_scroll: true,
                scroll_enabled: true,
                scroller_y: 18,
            });
        renderer.set_hud_inventory_screen(Some(HudInventoryScreen {
            width: 195,
            height: 136,
            background_layers,
            foreground_layers: Vec::new(),
            fill_layers: Vec::new(),
            slots: Vec::new(),
            floating_items: Vec::new(),
            foreground_items: Vec::new(),
            ghost_items: Vec::new(),
            entity_previews: Vec::new(),
            map_previews: Vec::new(),
            text_labels: Vec::new(),
            scroll_area: None,
            preedit_overlay: None,
            hovered_slot_id: None,
            tooltip: None,
        }));

        let first = renderer.render_offscreen_frame().unwrap();
        assert_eq!(first.pixel(30, 40), [255, 0, 0, 255]);
        assert_eq!(first.pixel(200, 55), [0, 0, 255, 255]);
        assert_eq!(first.pixel(200, 10), [0, 255, 0, 255]);
        let second = renderer.render_offscreen_frame().unwrap();
        assert_eq!(first.rgba, second.rgba);
    }

    #[test]
    fn creative_chrome_keeps_one_to_one_gui_geometry_on_larger_surfaces() {
        let Some(mut renderer) = Renderer::new_offscreen(480, 400) else {
            return;
        };
        renderer
            .upload_hud_creative_items_background(195, 136, &[255, 0, 0, 255].repeat(195 * 136))
            .unwrap();
        renderer.set_hud_inventory_screen(Some(HudInventoryScreen {
            width: 195,
            height: 136,
            background_layers: hud_creative_inventory_background_layers(
                &HudCreativeInventoryChrome {
                    background: HudCreativeInventoryBackground::Items,
                    tabs: Vec::new(),
                    can_scroll: false,
                    scroll_enabled: false,
                    scroller_y: 18,
                },
            ),
            foreground_layers: Vec::new(),
            fill_layers: Vec::new(),
            slots: Vec::new(),
            floating_items: Vec::new(),
            foreground_items: Vec::new(),
            ghost_items: Vec::new(),
            entity_previews: Vec::new(),
            map_previews: Vec::new(),
            text_labels: Vec::new(),
            scroll_area: None,
            preedit_overlay: None,
            hovered_slot_id: None,
            tooltip: None,
        }));

        let frame = renderer.render_offscreen_frame().unwrap();
        let clear = frame.pixel(0, 0);
        assert_ne!(clear, [255, 0, 0, 255]);
        assert_eq!(frame.pixel(141, 132), clear);
        assert_eq!(frame.pixel(142, 132), [255, 0, 0, 255]);
        assert_eq!(frame.pixel(336, 267), [255, 0, 0, 255]);
        assert_eq!(frame.pixel(337, 267), clear);
    }

    #[test]
    fn creative_chrome_clips_safely_when_surface_is_smaller_than_screen() {
        let Some(mut renderer) = Renderer::new_offscreen(100, 80) else {
            return;
        };
        renderer
            .upload_hud_creative_inventory_background(195, 136, &[255, 0, 0, 255].repeat(195 * 136))
            .unwrap();
        renderer.set_hud_inventory_screen(Some(HudInventoryScreen {
            width: 195,
            height: 136,
            background_layers: hud_creative_inventory_background_layers(
                &HudCreativeInventoryChrome {
                    background: HudCreativeInventoryBackground::Inventory,
                    tabs: Vec::new(),
                    can_scroll: false,
                    scroll_enabled: false,
                    scroller_y: 18,
                },
            ),
            foreground_layers: Vec::new(),
            fill_layers: Vec::new(),
            slots: Vec::new(),
            floating_items: Vec::new(),
            foreground_items: Vec::new(),
            ghost_items: Vec::new(),
            entity_previews: Vec::new(),
            map_previews: Vec::new(),
            text_labels: Vec::new(),
            scroll_area: None,
            preedit_overlay: None,
            hovered_slot_id: None,
            tooltip: None,
        }));

        let frame = renderer.render_offscreen_frame().unwrap();
        assert_eq!(frame.pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(frame.pixel(99, 79), [255, 0, 0, 255]);
    }
}
