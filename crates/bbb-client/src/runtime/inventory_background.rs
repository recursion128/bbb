use super::*;

const CRAFTER_GRID_SLOT_COUNT: i16 = 9;
const CRAFTER_POWERED_DATA_ID: i16 = 9;
const CRAFTER_DISABLED_SLOT_SPRITE_SIZE: u32 = 18;
const CRAFTER_REDSTONE_SPRITE_SIZE: u32 = 16;
pub(super) const BEACON_LEVELS_DATA_ID: i16 = 0;
pub(super) const BEACON_PRIMARY_EFFECT_DATA_ID: i16 = 1;
const BEACON_SECONDARY_EFFECT_DATA_ID: i16 = 2;
pub(super) const BEACON_CONFIRM_BUTTON_X: i32 = 164;
pub(super) const BEACON_CANCEL_BUTTON_X: i32 = 190;
pub(super) const BEACON_ACTION_BUTTON_Y: i32 = 107;
const BEACON_ACTION_BUTTON_SIZE: u32 = 22;
const BEACON_ACTION_ICON_OFFSET: i32 = 2;
const BEACON_ACTION_ICON_SIZE: u32 = 18;
pub(super) const BEACON_PRIMARY_LABEL_CENTER_X: i32 = 62;
pub(super) const BEACON_SECONDARY_LABEL_CENTER_X: i32 = 169;
pub(super) const BEACON_LABEL_Y: i32 = 10;
pub(super) const BEACON_LABEL_TEXT_COLOR: [f32; 4] = rgba32(-2_039_584);
pub(super) const BEACON_EXAMPLE_PAYMENT_ITEMS: [(i32, i32, i32); 5] = [
    (VANILLA_ITEM_NETHERITE_INGOT_ID, 20, 109),
    (VANILLA_ITEM_EMERALD_ID, 41, 109),
    (VANILLA_ITEM_DIAMOND_ID, 63, 109),
    (VANILLA_ITEM_GOLD_INGOT_ID, 86, 109),
    (VANILLA_ITEM_IRON_INGOT_ID, 108, 109),
];
const ENCHANTING_TABLE_LAPIS_SLOT_SPRITE_SIZE: u32 = 16;
const ENCHANTING_TABLE_LEVEL_ICON_X_OFFSET: i32 = 1;
const ENCHANTING_TABLE_LEVEL_ICON_Y_OFFSET: i32 = 1;
const ENCHANTING_TABLE_LEVEL_ICON_SIZE: u32 = 16;
pub(super) const ENCHANTING_TABLE_COST_TEXT_X_RIGHT: i32 = 166;
pub(super) const ENCHANTING_TABLE_COST_TEXT_Y_OFFSET: i32 = 23;
pub(super) const ENCHANTING_TABLE_COST_TEXT_ENABLED_COLOR: [f32; 4] =
    [128.0 / 255.0, 1.0, 32.0 / 255.0, 1.0];
pub(super) const ENCHANTING_TABLE_COST_TEXT_DISABLED_COLOR: [f32; 4] =
    [64.0 / 255.0, 127.0 / 255.0, 16.0 / 255.0, 1.0];
pub(super) const ENCHANTING_TABLE_NAME_TEXT_COLOR: [f32; 4] =
    [104.0 / 255.0, 94.0 / 255.0, 74.0 / 255.0, 1.0];
pub(super) const ENCHANTING_TABLE_NAME_TEXT_DISABLED_COLOR: [f32; 4] =
    [52.0 / 255.0, 47.0 / 255.0, 37.0 / 255.0, 1.0];
pub(super) const ENCHANTING_TABLE_NAME_TEXT_HIGHLIGHTED_COLOR: [f32; 4] =
    [1.0, 1.0, 128.0 / 255.0, 1.0];
pub(super) const ANVIL_COST_DATA_ID: i16 = 0;
pub(super) const ANVIL_RESULT_SLOT: i16 = 2;
pub(super) const ANVIL_TOO_EXPENSIVE_LEVEL_COST: i16 = 40;
pub(super) const ANVIL_RENAME_TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
pub(super) const ANVIL_COST_TEXT_COLOR: [f32; 4] = [128.0 / 255.0, 1.0, 32.0 / 255.0, 1.0];
pub(super) const ANVIL_COST_ERROR_TEXT_COLOR: [f32; 4] = [1.0, 96.0 / 255.0, 96.0 / 255.0, 1.0];
pub(super) const ANVIL_COST_BACKGROUND_TINT: [f32; 4] = [0.0, 0.0, 0.0, 79.0 / 255.0];
pub(super) const ANVIL_COST_LABEL_RIGHT: i32 = 168;
pub(super) const ANVIL_COST_LABEL_Y: i32 = 69;
pub(super) const ANVIL_COST_BACKGROUND_Y: i32 = 67;
pub(super) const ANVIL_COST_BACKGROUND_HEIGHT: u32 = 12;
const CARTOGRAPHY_TABLE_MAP_SLOT: i16 = 0;
const CARTOGRAPHY_TABLE_ADDITIONAL_SLOT: i16 = 1;
const CARTOGRAPHY_TABLE_RESULT_SLOT: i16 = 2;
const CARTOGRAPHY_TABLE_ERROR_X: i32 = 35;
const CARTOGRAPHY_TABLE_ERROR_Y: i32 = 31;
const CARTOGRAPHY_TABLE_ERROR_WIDTH: u32 = 28;
const CARTOGRAPHY_TABLE_ERROR_HEIGHT: u32 = 21;
const CARTOGRAPHY_TABLE_MAP_X: i32 = 67;
const CARTOGRAPHY_TABLE_MAP_Y: i32 = 13;
const CARTOGRAPHY_TABLE_MAP_SIZE: u32 = 66;
const CARTOGRAPHY_TABLE_DUPLICATED_MAP_WIDTH: u32 = 50;
const CARTOGRAPHY_TABLE_DUPLICATED_MAP_HEIGHT: u32 = 66;
const CARTOGRAPHY_TABLE_DUPLICATED_MAP_OFFSET: i32 = 16;
const CARTOGRAPHY_TABLE_LOCKED_X: i32 = 118;
const CARTOGRAPHY_TABLE_LOCKED_Y: i32 = 60;
const CARTOGRAPHY_TABLE_LOCKED_WIDTH: u32 = 10;
const CARTOGRAPHY_TABLE_LOCKED_HEIGHT: u32 = 14;
const SMITHING_RECIPE_ERROR_DATA_ID: i16 = 0;
const BREWING_STAND_BREW_TIME_DATA_ID: i16 = 0;
const BREWING_STAND_FUEL_DATA_ID: i16 = 1;
const BREWING_STAND_FUEL_LENGTH_SPRITE_WIDTH: u32 = 18;
const BREWING_STAND_FUEL_LENGTH_SPRITE_HEIGHT: u32 = 4;
const BREWING_STAND_BREW_PROGRESS_SPRITE_WIDTH: u32 = 9;
const BREWING_STAND_BREW_PROGRESS_SPRITE_HEIGHT: u32 = 28;
const BREWING_STAND_BUBBLES_SPRITE_WIDTH: u32 = 12;
const BREWING_STAND_BUBBLES_SPRITE_HEIGHT: u32 = 29;
const BREWING_STAND_BREW_TOTAL_TICKS: f32 = 400.0;
const BREWING_STAND_BUBBLE_LENGTHS: [u32; 7] = [29, 24, 20, 16, 11, 6, 0];
const FURNACE_LIT_TIME_DATA_ID: i16 = 0;
const FURNACE_LIT_DURATION_DATA_ID: i16 = 1;
const FURNACE_COOKING_PROGRESS_DATA_ID: i16 = 2;
const FURNACE_COOKING_TOTAL_TIME_DATA_ID: i16 = 3;
const FURNACE_DEFAULT_LIT_DURATION: i16 = 200;
const FURNACE_LIT_PROGRESS_SPRITE_SIZE: u32 = 14;
const FURNACE_BURN_PROGRESS_SPRITE_WIDTH: u32 = 24;
const FURNACE_BURN_PROGRESS_SPRITE_HEIGHT: u32 = 16;
pub(super) const MERCHANT_VISIBLE_OFFER_COUNT: usize = 7;
pub(super) const MERCHANT_TRADE_BUTTON_X: i32 = 5;
pub(super) const MERCHANT_TRADE_BUTTON_Y: i32 = 18;
pub(super) const MERCHANT_TRADE_BUTTON_WIDTH: u32 = 88;
pub(super) const MERCHANT_TRADE_BUTTON_HEIGHT: u32 = 20;
pub(super) const MERCHANT_TRADE_COST_A_X: i32 = 10;
pub(super) const MERCHANT_TRADE_COST_B_X: i32 = 40;
pub(super) const MERCHANT_TRADE_RESULT_X: i32 = 73;
pub(super) const MERCHANT_TRADE_ITEM_Y: i32 = 19;
pub(super) const MERCHANT_TRADE_ROW_HEIGHT: i32 = 20;
const MERCHANT_TRADE_ARROW_X: i32 = 60;
const MERCHANT_TRADE_ARROW_Y: i32 = 22;
const MERCHANT_TRADE_ARROW_WIDTH: u32 = 10;
const MERCHANT_TRADE_ARROW_HEIGHT: u32 = 9;
const MERCHANT_DISCOUNT_STRIKETHROUGH_X: i32 = 17;
const MERCHANT_DISCOUNT_STRIKETHROUGH_Y: i32 = 31;
const MERCHANT_DISCOUNT_STRIKETHROUGH_WIDTH: u32 = 9;
const MERCHANT_DISCOUNT_STRIKETHROUGH_HEIGHT: u32 = 2;
const MERCHANT_SCROLLER_X: i32 = 94;
const MERCHANT_SCROLLER_Y: i32 = 18;
const MERCHANT_SCROLLER_WIDTH: u32 = 6;
const MERCHANT_SCROLLER_HEIGHT: u32 = 27;
const MERCHANT_SCROLLER_TRACK_HEIGHT: i32 = 139;
const MERCHANT_SCROLLER_MAX_OFFSET: i32 = 113;
const MERCHANT_OUT_OF_STOCK_X: i32 = 182;
const MERCHANT_OUT_OF_STOCK_Y: i32 = 35;
const MERCHANT_OUT_OF_STOCK_WIDTH: u32 = 28;
const MERCHANT_OUT_OF_STOCK_HEIGHT: u32 = 21;
pub(super) const MERCHANT_DEPRECATED_TOOLTIP_X: i32 = 186;
pub(super) const MERCHANT_DEPRECATED_TOOLTIP_Y: i32 = 35;
pub(super) const MERCHANT_DEPRECATED_TOOLTIP_WIDTH: i32 = 22;
pub(super) const MERCHANT_DEPRECATED_TOOLTIP_HEIGHT: i32 = 21;
const MERCHANT_XP_BAR_X: i32 = 136;
const MERCHANT_XP_BAR_Y: i32 = 16;
const MERCHANT_XP_BAR_WIDTH: u32 = 102;
const MERCHANT_XP_BAR_HEIGHT: u32 = 5;
const VILLAGER_NEXT_LEVEL_XP_THRESHOLDS: [i32; 5] = [0, 10, 70, 150, 250];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CartographyResultMode {
    Map,
    Scaled,
    Duplicated,
    Locked,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CartographyTablePresentation {
    pub(super) background_layers: Vec<HudInventoryBackgroundLayer>,
    pub(super) map_previews: Vec<HudInventoryMapPreview>,
    pub(super) mode: Option<CartographyResultMode>,
    pub(super) invalid_transform: bool,
}

pub(super) fn hud_inventory_background_layers(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    local_state: &InventoryHudLocalState,
    enchantment_options: Option<&[EnchantmentOptionHudState; ENCHANTMENT_OPTION_COUNT]>,
) -> Vec<HudInventoryBackgroundLayer> {
    match background {
        InventoryScreenBackground::LocalInventory => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/inventory.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        }
        InventoryScreenBackground::Generic9xRows { rows } => {
            let top_height = u32::from(rows) * 18 + 17;
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Texture("textures/gui/container/generic_54.png"),
                    0,
                    0,
                    176,
                    top_height,
                    [0.0, 0.0],
                    [176.0 / 256.0, top_height as f32 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Texture("textures/gui/container/generic_54.png"),
                    0,
                    i32::try_from(top_height).unwrap_or_default(),
                    176,
                    96,
                    [0.0, 126.0 / 256.0],
                    [176.0 / 256.0, 222.0 / 256.0],
                ),
            ]
        }
        InventoryScreenBackground::Generic3x3 => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/dispenser.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        }
        InventoryScreenBackground::CraftingTable => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/crafting_table.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        }
        InventoryScreenBackground::EnchantmentTable => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture(
                    "textures/gui/container/enchanting_table.png",
                ),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_enchanting_table_state_layers(
                world,
                enchantment_options.map_or(&[], |options| options.as_slice()),
                &mut layers,
            );
            layers
        }
        InventoryScreenBackground::Crafter => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/crafter.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_crafter_state_layers(world, &mut layers);
            layers
        }
        InventoryScreenBackground::Anvil => {
            let mut layers = vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Texture("textures/gui/container/anvil.png"),
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    if anvil_input_slot_has_item(world) {
                        HudInventoryBackgroundTexture::Sprite("container/anvil/text_field")
                    } else {
                        HudInventoryBackgroundTexture::Sprite("container/anvil/text_field_disabled")
                    },
                    59,
                    20,
                    110,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ];
            if anvil_should_show_error(world) {
                layers.push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite("container/anvil/error"),
                    99,
                    45,
                    28,
                    21,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            }
            layers
        }
        InventoryScreenBackground::Beacon => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/beacon.png"),
                0,
                0,
                230,
                219,
                [0.0, 0.0],
                [230.0 / 256.0, 219.0 / 256.0],
            )];
            let selection = local_state
                .beacon_effect_selection
                .unwrap_or_else(|| beacon_effect_selection_from_world(world));
            push_beacon_effect_button_layers(
                world,
                &mut layers,
                selection,
                local_state.cursor_position,
            );
            push_beacon_action_button_layers(
                world,
                &mut layers,
                selection,
                local_state.cursor_position,
            );
            layers
        }
        InventoryScreenBackground::BrewingStand => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/brewing_stand.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_brewing_stand_progress_layers(world, &mut layers);
            layers
        }
        InventoryScreenBackground::CartographyTable => {
            cartography_table_presentation(world, item_runtime).background_layers
        }
        InventoryScreenBackground::BlastFurnace => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/blast_furnace.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_furnace_progress_layers(
                world,
                &mut layers,
                HudInventoryBackgroundTexture::Sprite("container/blast_furnace/lit_progress"),
                HudInventoryBackgroundTexture::Sprite("container/blast_furnace/burn_progress"),
            );
            layers
        }
        InventoryScreenBackground::Furnace => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/furnace.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_furnace_progress_layers(
                world,
                &mut layers,
                HudInventoryBackgroundTexture::Sprite("container/furnace/lit_progress"),
                HudInventoryBackgroundTexture::Sprite("container/furnace/burn_progress"),
            );
            layers
        }
        InventoryScreenBackground::Grindstone => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/grindstone.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            if grindstone_should_show_error(world) {
                layers.push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite("container/grindstone/error"),
                    92,
                    31,
                    28,
                    21,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            }
            layers
        }
        InventoryScreenBackground::Smithing => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/smithing.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            if smithing_should_show_error(world) {
                layers.push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite("container/smithing/error"),
                    65,
                    46,
                    28,
                    21,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            }
            layers
        }
        InventoryScreenBackground::Hopper => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/hopper.png"),
                0,
                0,
                176,
                133,
                [0.0, 0.0],
                [176.0 / 256.0, 133.0 / 256.0],
            )]
        }
        InventoryScreenBackground::Mount {
            kind,
            inventory_columns,
        } => {
            let equipment_slots = world
                .open_mount_equipment_slot_visibility()
                .unwrap_or_default();
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture(match kind {
                    MountInventoryKind::Horse => "textures/gui/container/horse.png",
                    MountInventoryKind::Nautilus => "textures/gui/container/nautilus.png",
                }),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            if equipment_slots.saddle {
                layers.extend([
                    hud_inventory_background_layer(
                        HudInventoryBackgroundTexture::Sprite("container/slot"),
                        7,
                        17,
                        18,
                        18,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                    hud_inventory_background_layer(
                        HudInventoryBackgroundTexture::Sprite("container/slot/saddle"),
                        8,
                        18,
                        16,
                        16,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                ]);
            }
            if let Some(armor_slot) = equipment_slots.body {
                layers.extend([
                    hud_inventory_background_layer(
                        HudInventoryBackgroundTexture::Sprite("container/slot"),
                        7,
                        35,
                        18,
                        18,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                    hud_inventory_background_layer(
                        mount_armor_slot_texture(armor_slot),
                        8,
                        36,
                        16,
                        16,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                ]);
            }
            if inventory_columns > 0 {
                let chest_width = u32::from(inventory_columns) * 18;
                layers.push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite("container/horse/chest_slots"),
                    79,
                    17,
                    chest_width,
                    54,
                    [0.0, 0.0],
                    [chest_width as f32 / 90.0, 1.0],
                ));
            }
            layers
        }
        InventoryScreenBackground::Lectern => {
            unreachable!("lectern presentation is owned by HudBookScreen")
        }
        InventoryScreenBackground::Loom => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/loom.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        }
        InventoryScreenBackground::Merchant => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/villager.png"),
                0,
                0,
                276,
                166,
                [0.0, 0.0],
                [276.0 / 512.0, 166.0 / 256.0],
            )];
            push_merchant_trade_layers(
                world,
                &mut layers,
                local_state.cursor_position,
                local_state.merchant_focused_trade_row,
            );
            layers
        }
        InventoryScreenBackground::ShulkerBox => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/shulker_box.png"),
                0,
                0,
                176,
                167,
                [0.0, 0.0],
                [176.0 / 256.0, 167.0 / 256.0],
            )]
        }
        InventoryScreenBackground::Smoker => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/smoker.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_furnace_progress_layers(
                world,
                &mut layers,
                HudInventoryBackgroundTexture::Sprite("container/smoker/lit_progress"),
                HudInventoryBackgroundTexture::Sprite("container/smoker/burn_progress"),
            );
            layers
        }
        InventoryScreenBackground::Stonecutter => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/stonecutter.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_stonecutter_recipe_layers(
                world,
                &mut layers,
                local_state
                    .stonecutter_recipe_scroll_row
                    .unwrap_or_default(),
            );
            layers
        }
    }
}

fn mount_armor_slot_texture(kind: MountArmorSlotKind) -> HudInventoryBackgroundTexture {
    HudInventoryBackgroundTexture::Sprite(match kind {
        MountArmorSlotKind::Horse => "container/slot/horse_armor",
        MountArmorSlotKind::Llama => "container/slot/llama_armor",
        MountArmorSlotKind::Nautilus => "container/slot/nautilus_armor_inventory",
    })
}

fn push_merchant_trade_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    cursor_position: Option<(i32, i32)>,
    focused_trade_row: Option<i32>,
) {
    let Some(offers) = merchant_offers_state(world) else {
        return;
    };
    if offers.offers.is_empty() {
        return;
    }

    let visible_offers = offers
        .offers
        .iter()
        .skip(merchant_scroll_offset(offers))
        .take(MERCHANT_VISIBLE_OFFER_COUNT)
        .enumerate();
    for (row, _) in visible_offers.clone() {
        let row = i32::try_from(row).unwrap_or_default();
        let row_y = MERCHANT_TRADE_BUTTON_Y + row * MERCHANT_TRADE_ROW_HEIGHT;
        let highlighted = focused_trade_row == Some(row)
            || cursor_position.is_some_and(|(cursor_x, cursor_y)| {
                cursor_x >= MERCHANT_TRADE_BUTTON_X
                    && cursor_x < MERCHANT_TRADE_BUTTON_X + MERCHANT_TRADE_BUTTON_WIDTH as i32
                    && cursor_y >= row_y
                    && cursor_y < row_y + MERCHANT_TRADE_BUTTON_HEIGHT as i32
            });
        layers.push(hud_inventory_background_layer(
            if highlighted {
                HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
            } else {
                HudInventoryBackgroundTexture::Sprite("widget/button")
            },
            MERCHANT_TRADE_BUTTON_X,
            row_y,
            MERCHANT_TRADE_BUTTON_WIDTH,
            MERCHANT_TRADE_BUTTON_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    for (row, offer) in visible_offers {
        let row = i32::try_from(row).unwrap_or_default();
        layers.push(hud_inventory_background_layer(
            if offer.is_out_of_stock {
                HudInventoryBackgroundTexture::Sprite("container/villager/trade_arrow_out_of_stock")
            } else {
                HudInventoryBackgroundTexture::Sprite("container/villager/trade_arrow")
            },
            MERCHANT_TRADE_ARROW_X,
            MERCHANT_TRADE_ARROW_Y + row * MERCHANT_TRADE_ROW_HEIGHT,
            MERCHANT_TRADE_ARROW_WIDTH,
            MERCHANT_TRADE_ARROW_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    layers.push(hud_inventory_background_layer(
        if offers.offers.len() > MERCHANT_VISIBLE_OFFER_COUNT {
            HudInventoryBackgroundTexture::Sprite("container/villager/scroller")
        } else {
            HudInventoryBackgroundTexture::Sprite("container/villager/scroller_disabled")
        },
        MERCHANT_SCROLLER_X,
        MERCHANT_SCROLLER_Y + merchant_scroller_offset(offers, offers.local_scroll_offset),
        MERCHANT_SCROLLER_WIDTH,
        MERCHANT_SCROLLER_HEIGHT,
        [0.0, 0.0],
        [1.0, 1.0],
    ));

    if merchant_selected_offer(offers).is_some_and(|offer| offer.is_out_of_stock) {
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/villager/out_of_stock"),
            MERCHANT_OUT_OF_STOCK_X,
            MERCHANT_OUT_OF_STOCK_Y,
            MERCHANT_OUT_OF_STOCK_WIDTH,
            MERCHANT_OUT_OF_STOCK_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    if offers.show_progress && offers.villager_level < 5 {
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/villager/experience_bar_background"),
            MERCHANT_XP_BAR_X,
            MERCHANT_XP_BAR_Y,
            MERCHANT_XP_BAR_WIDTH,
            MERCHANT_XP_BAR_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
        if let Some(current_width) = merchant_xp_current_width(offers) {
            layers.push(hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/villager/experience_bar_current"),
                MERCHANT_XP_BAR_X,
                MERCHANT_XP_BAR_Y,
                current_width,
                MERCHANT_XP_BAR_HEIGHT,
                [0.0, 0.0],
                [current_width as f32 / MERCHANT_XP_BAR_WIDTH as f32, 1.0],
            ));
            if let Some(result_width) = merchant_xp_result_width(world, offers, current_width) {
                layers.push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite(
                        "container/villager/experience_bar_result",
                    ),
                    MERCHANT_XP_BAR_X + current_width as i32,
                    MERCHANT_XP_BAR_Y,
                    result_width,
                    MERCHANT_XP_BAR_HEIGHT,
                    [current_width as f32 / MERCHANT_XP_BAR_WIDTH as f32, 0.0],
                    [
                        (current_width + result_width) as f32 / MERCHANT_XP_BAR_WIDTH as f32,
                        1.0,
                    ],
                ));
            }
        }
    }
}

pub(super) fn hud_merchant_discount_layers(world: &WorldStore) -> Vec<HudInventoryBackgroundLayer> {
    let Some(offers) = merchant_offers_state(world) else {
        return Vec::new();
    };
    offers
        .offers
        .iter()
        .skip(merchant_scroll_offset(offers))
        .take(MERCHANT_VISIBLE_OFFER_COUNT)
        .enumerate()
        .filter(|(_, offer)| {
            merchant_offer_base_cost_a_stack(offer).count
                != merchant_offer_cost_a_stack(world, offer).count
        })
        .map(|(row, _)| {
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/villager/discount_strikethrough"),
                MERCHANT_DISCOUNT_STRIKETHROUGH_X,
                MERCHANT_DISCOUNT_STRIKETHROUGH_Y
                    + i32::try_from(row).unwrap_or_default() * MERCHANT_TRADE_ROW_HEIGHT,
                MERCHANT_DISCOUNT_STRIKETHROUGH_WIDTH,
                MERCHANT_DISCOUNT_STRIKETHROUGH_HEIGHT,
                [0.0, 0.0],
                [1.0, 1.0],
            )
        })
        .collect()
}

pub(super) fn merchant_offers_state(world: &WorldStore) -> Option<&MerchantOffersState> {
    world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
}

pub(super) fn merchant_selected_offer(offers: &MerchantOffersState) -> Option<&MerchantOfferState> {
    usize::try_from(offers.local_selected_offer_index)
        .ok()
        .and_then(|index| offers.offers.get(index))
}

pub(super) fn merchant_scroll_offset(offers: &MerchantOffersState) -> usize {
    let max_scroll_offset = offers
        .offers
        .len()
        .saturating_sub(MERCHANT_VISIBLE_OFFER_COUNT);
    usize::try_from(offers.local_scroll_offset)
        .unwrap_or_default()
        .min(max_scroll_offset)
}

pub(super) fn merchant_offer_cost_a_stack(
    world: &WorldStore,
    offer: &MerchantOfferState,
) -> ItemStackSummary {
    let max_stack_size = offer
        .buy_a
        .component_predicate
        .components
        .max_stack_size
        .unwrap_or_else(|| {
            world
                .item_profiles()
                .item_max_stack_size_for_protocol_id(offer.buy_a.item_id)
        })
        .clamp(
            1,
            bbb_protocol::item_types::VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE,
        );
    let demand_diff = (offer.buy_a.count as f32 * offer.demand as f32 * offer.price_multiplier)
        .floor()
        .max(0.0) as i32;
    let count = offer
        .buy_a
        .count
        .wrapping_add(demand_diff)
        .wrapping_add(offer.special_price_diff)
        .clamp(1, max_stack_size);
    item_cost_stack(&offer.buy_a, count)
}

pub(super) fn merchant_offer_base_cost_a_stack(offer: &MerchantOfferState) -> ItemStackSummary {
    item_cost_stack(&offer.buy_a, offer.buy_a.count)
}

pub(super) fn item_cost_stack(cost: &ItemCostSummary, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(cost.item_id),
        count,
        component_patch: cost.component_predicate.display_patch(),
    }
}

fn merchant_scroller_offset(offers: &MerchantOffersState, scroll_offset: i32) -> i32 {
    let steps = offers.offers.len() as i32 + 1 - MERCHANT_VISIBLE_OFFER_COUNT as i32;
    if steps <= 1 {
        return 0;
    }
    let left_over = MERCHANT_SCROLLER_TRACK_HEIGHT
        - (MERCHANT_SCROLLER_HEIGHT as i32 + (steps - 1) * MERCHANT_SCROLLER_TRACK_HEIGHT / steps);
    let step_height = 1 + left_over / steps + MERCHANT_SCROLLER_TRACK_HEIGHT / steps;
    let scroller_offset = (scroll_offset * step_height).min(MERCHANT_SCROLLER_MAX_OFFSET);
    if scroll_offset == steps - 1 {
        MERCHANT_SCROLLER_MAX_OFFSET
    } else {
        scroller_offset
    }
}

fn merchant_xp_current_width(offers: &MerchantOffersState) -> Option<u32> {
    let level = usize::try_from(offers.villager_level).ok()?;
    if !(1..5).contains(&level) {
        return None;
    }
    let min_xp = VILLAGER_NEXT_LEVEL_XP_THRESHOLDS[level - 1];
    let max_xp = VILLAGER_NEXT_LEVEL_XP_THRESHOLDS[level];
    if offers.villager_xp < min_xp || max_xp <= min_xp {
        return None;
    }
    let multiplier = MERCHANT_XP_BAR_WIDTH as f32 / (max_xp - min_xp) as f32;
    let width = (multiplier * (offers.villager_xp - min_xp) as f32).floor() as u32;
    Some(width.min(MERCHANT_XP_BAR_WIDTH))
}

fn merchant_xp_result_width(
    world: &WorldStore,
    offers: &MerchantOffersState,
    current_width: u32,
) -> Option<u32> {
    let offer = merchant_selected_offer(offers)?;
    if offer.is_out_of_stock || offer.xp <= 0 {
        return None;
    }
    let result = open_container_slot_item(world, 2)?;
    if item_stack_is_empty(result) || result != &offer.sell {
        return None;
    }
    let level = usize::try_from(offers.villager_level).ok()?;
    if !(1..5).contains(&level) {
        return None;
    }
    let min_xp = VILLAGER_NEXT_LEVEL_XP_THRESHOLDS[level - 1];
    let max_xp = VILLAGER_NEXT_LEVEL_XP_THRESHOLDS[level];
    if max_xp <= min_xp {
        return None;
    }
    let multiplier = MERCHANT_XP_BAR_WIDTH as f32 / (max_xp - min_xp) as f32;
    let width = (offer.xp as f32 * multiplier).floor() as u32;
    Some(width.min(MERCHANT_XP_BAR_WIDTH.saturating_sub(current_width))).filter(|width| *width > 0)
}

fn anvil_input_slot_has_item(world: &WorldStore) -> bool {
    open_container_slot_has_item(world, 0)
}

fn push_beacon_action_button_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    selection: (Option<i32>, Option<i32>),
    cursor_position: Option<(i32, i32)>,
) {
    let confirm_active = beacon_confirm_button_is_active(world, selection);
    let confirm_texture = if !confirm_active {
        HudInventoryBackgroundTexture::Sprite("container/beacon/button_disabled")
    } else if beacon_button_contains(
        cursor_position,
        BEACON_CONFIRM_BUTTON_X,
        BEACON_ACTION_BUTTON_Y,
    ) {
        HudInventoryBackgroundTexture::Sprite("container/beacon/button_highlighted")
    } else {
        HudInventoryBackgroundTexture::Sprite("container/beacon/button")
    };
    push_beacon_action_button_layer(
        layers,
        confirm_texture,
        HudInventoryBackgroundTexture::Sprite("container/beacon/confirm"),
        BEACON_CONFIRM_BUTTON_X,
    );
    push_beacon_action_button_layer(
        layers,
        if beacon_button_contains(
            cursor_position,
            BEACON_CANCEL_BUTTON_X,
            BEACON_ACTION_BUTTON_Y,
        ) {
            HudInventoryBackgroundTexture::Sprite("container/beacon/button_highlighted")
        } else {
            HudInventoryBackgroundTexture::Sprite("container/beacon/button")
        },
        HudInventoryBackgroundTexture::Sprite("container/beacon/cancel"),
        BEACON_CANCEL_BUTTON_X,
    );
}

fn push_beacon_effect_button_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    selection: (Option<i32>, Option<i32>),
    cursor_position: Option<(i32, i32)>,
) {
    let levels = beacon_levels(world);
    for button in beacon_effect_buttons(
        selection
            .0
            .filter(|effect_id| beacon_primary_effect(*effect_id)),
    ) {
        let selected = if button.primary {
            selection.0 == Some(button.effect_id)
        } else {
            selection.1 == Some(button.effect_id)
        };
        let button_texture = if button.tier >= levels {
            HudInventoryBackgroundTexture::Sprite("container/beacon/button_disabled")
        } else if selected {
            HudInventoryBackgroundTexture::Sprite("container/beacon/button_selected")
        } else if beacon_button_contains(cursor_position, button.x, button.y) {
            HudInventoryBackgroundTexture::Sprite("container/beacon/button_highlighted")
        } else {
            HudInventoryBackgroundTexture::Sprite("container/beacon/button")
        };
        layers.push(hud_inventory_background_layer(
            button_texture,
            button.x,
            button.y,
            BEACON_EFFECT_BUTTON_SIZE,
            BEACON_EFFECT_BUTTON_SIZE,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
        if let Some(icon_texture) = beacon_effect_icon_texture(button.effect_id) {
            layers.push(hud_inventory_background_layer(
                icon_texture,
                button.x + BEACON_ACTION_ICON_OFFSET,
                button.y + BEACON_ACTION_ICON_OFFSET,
                BEACON_ACTION_ICON_SIZE,
                BEACON_ACTION_ICON_SIZE,
                [0.0, 0.0],
                [1.0, 1.0],
            ));
        }
    }
}

pub(super) fn beacon_primary_effect(effect_id: i32) -> bool {
    VANILLA_BEACON_EFFECT_TIERS[..3]
        .iter()
        .copied()
        .flatten()
        .any(|candidate| *candidate == effect_id)
}

pub(super) fn beacon_button_contains(cursor_position: Option<(i32, i32)>, x: i32, y: i32) -> bool {
    let Some((cursor_x, cursor_y)) = cursor_position else {
        return false;
    };
    let size = i32::try_from(BEACON_EFFECT_BUTTON_SIZE).unwrap_or_default();
    cursor_x >= x && cursor_x < x + size && cursor_y >= y && cursor_y < y + size
}

fn beacon_effect_icon_texture(effect_id: i32) -> Option<HudInventoryBackgroundTexture> {
    VANILLA_BEACON_EFFECT_TIERS
        .iter()
        .copied()
        .flatten()
        .any(|candidate| *candidate == effect_id)
        .then_some(HudInventoryBackgroundTexture::MobEffect(effect_id))
}

fn push_beacon_action_button_layer(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    button_texture: HudInventoryBackgroundTexture,
    icon_texture: HudInventoryBackgroundTexture,
    x: i32,
) {
    layers.push(hud_inventory_background_layer(
        button_texture,
        x,
        BEACON_ACTION_BUTTON_Y,
        BEACON_ACTION_BUTTON_SIZE,
        BEACON_ACTION_BUTTON_SIZE,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
    layers.push(hud_inventory_background_layer(
        icon_texture,
        x + BEACON_ACTION_ICON_OFFSET,
        BEACON_ACTION_BUTTON_Y + BEACON_ACTION_ICON_OFFSET,
        BEACON_ACTION_ICON_SIZE,
        BEACON_ACTION_ICON_SIZE,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
}

fn beacon_confirm_button_is_active(
    world: &WorldStore,
    selection: (Option<i32>, Option<i32>),
) -> bool {
    open_container_slot_has_item(world, 0) && selection.0.is_some_and(beacon_primary_effect)
}

pub(super) fn beacon_effect_selection_from_world(world: &WorldStore) -> (Option<i32>, Option<i32>) {
    (
        beacon_data_effect_id(world, BEACON_PRIMARY_EFFECT_DATA_ID),
        beacon_data_effect_id(world, BEACON_SECONDARY_EFFECT_DATA_ID),
    )
}

fn beacon_data_effect_id(world: &WorldStore, data_id: i16) -> Option<i32> {
    let value = world.open_container_data_value(data_id)?;
    (value > 0).then_some(i32::from(value) - 1)
}

fn beacon_levels(world: &WorldStore) -> i16 {
    world
        .open_container_data_value(BEACON_LEVELS_DATA_ID)
        .unwrap_or_default()
}

fn push_crafter_state_layers(world: &WorldStore, layers: &mut Vec<HudInventoryBackgroundLayer>) {
    for slot in 0..CRAFTER_GRID_SLOT_COUNT {
        if world.open_container_data_value(slot).unwrap_or_default() != 1 {
            continue;
        }
        let x = i32::from(slot % 3);
        let y = i32::from(slot / 3);
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/crafter/disabled_slot"),
            25 + x * 18,
            16 + y * 18,
            CRAFTER_DISABLED_SLOT_SPRITE_SIZE,
            CRAFTER_DISABLED_SLOT_SPRITE_SIZE,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    let redstone_texture = if world
        .open_container_data_value(CRAFTER_POWERED_DATA_ID)
        .unwrap_or_default()
        == 1
    {
        HudInventoryBackgroundTexture::Sprite("container/crafter/powered_redstone")
    } else {
        HudInventoryBackgroundTexture::Sprite("container/crafter/unpowered_redstone")
    };
    layers.push(hud_inventory_background_layer(
        redstone_texture,
        97,
        35,
        CRAFTER_REDSTONE_SPRITE_SIZE,
        CRAFTER_REDSTONE_SPRITE_SIZE,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
}

fn push_enchanting_table_state_layers(
    world: &WorldStore,
    options: &[EnchantmentOptionHudState],
    layers: &mut Vec<HudInventoryBackgroundLayer>,
) {
    if open_container_slot_has_item(world, 1) {
        for slot in enchanting_table_option_layers(options) {
            layers.push(slot);
        }
        return;
    }
    layers.push(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Sprite("container/slot/lapis_lazuli"),
        35,
        47,
        ENCHANTING_TABLE_LAPIS_SLOT_SPRITE_SIZE,
        ENCHANTING_TABLE_LAPIS_SLOT_SPRITE_SIZE,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
    for slot in enchanting_table_option_layers(options) {
        layers.push(slot);
    }
}

fn enchanting_table_option_layers(
    options: &[EnchantmentOptionHudState],
) -> Vec<HudInventoryBackgroundLayer> {
    let mut layers = Vec::with_capacity(options.len() * 2);
    for option in options {
        let index = i16::try_from(option.index).expect("three enchanting rows fit i16");
        let y = ENCHANTING_TABLE_OPTION_Y + i32::from(index) * ENCHANTING_TABLE_OPTION_SPACING;
        if option.cost <= 0 {
            layers.push(hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite(
                    "container/enchanting_table/enchantment_slot_disabled",
                ),
                ENCHANTING_TABLE_OPTION_X,
                y,
                ENCHANTING_TABLE_OPTION_WIDTH,
                ENCHANTING_TABLE_OPTION_HEIGHT,
                [0.0, 0.0],
                [1.0, 1.0],
            ));
            continue;
        }

        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite(if !option.enabled {
                "container/enchanting_table/enchantment_slot_disabled"
            } else if option.highlighted {
                "container/enchanting_table/enchantment_slot_highlighted"
            } else {
                "container/enchanting_table/enchantment_slot"
            }),
            ENCHANTING_TABLE_OPTION_X,
            y,
            ENCHANTING_TABLE_OPTION_WIDTH,
            ENCHANTING_TABLE_OPTION_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
        layers.push(hud_inventory_background_layer(
            enchanting_table_level_texture(index, option.enabled),
            ENCHANTING_TABLE_OPTION_X + ENCHANTING_TABLE_LEVEL_ICON_X_OFFSET,
            y + ENCHANTING_TABLE_LEVEL_ICON_Y_OFFSET,
            ENCHANTING_TABLE_LEVEL_ICON_SIZE,
            ENCHANTING_TABLE_LEVEL_ICON_SIZE,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }
    layers
}

fn enchanting_table_level_texture(index: i16, enabled: bool) -> HudInventoryBackgroundTexture {
    HudInventoryBackgroundTexture::Sprite(match (index, enabled) {
        (0, true) => "container/enchanting_table/level_1",
        (1, true) => "container/enchanting_table/level_2",
        (_, true) => "container/enchanting_table/level_3",
        (0, false) => "container/enchanting_table/level_1_disabled",
        (1, false) => "container/enchanting_table/level_2_disabled",
        (_, false) => "container/enchanting_table/level_3_disabled",
    })
}

pub(super) fn cartography_table_presentation(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> CartographyTablePresentation {
    let mut presentation = CartographyTablePresentation {
        background_layers: vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Texture("textures/gui/container/cartography_table.png"),
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )],
        map_previews: Vec::new(),
        mode: None,
        invalid_transform: false,
    };
    let Some(input_map) = open_container_slot_item(world, CARTOGRAPHY_TABLE_MAP_SLOT) else {
        return presentation;
    };
    let Some(map_id) = item_stack_map_id(input_map) else {
        return presentation;
    };

    let additional_item = open_container_slot_item(world, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT);
    let additional =
        additional_item.and_then(|item| cartography_additional_item(world, item_runtime, item));
    let map_state = world.map_item(map_id);
    let result_slot_mode = open_container_slot_item(world, CARTOGRAPHY_TABLE_RESULT_SLOT)
        .and_then(cartography_result_mode);
    let invalid_transform = match additional {
        Some(CartographyAdditionalItemKind::Paper) => {
            map_state.is_some_and(|map| map.locked || map.scale >= 4)
        }
        Some(CartographyAdditionalItemKind::GlassPane) => map_state.is_some_and(|map| map.locked),
        _ => false,
    };

    if invalid_transform {
        presentation
            .background_layers
            .push(hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("container/cartography_table/error"),
                CARTOGRAPHY_TABLE_ERROR_X,
                CARTOGRAPHY_TABLE_ERROR_Y,
                CARTOGRAPHY_TABLE_ERROR_WIDTH,
                CARTOGRAPHY_TABLE_ERROR_HEIGHT,
                [0.0, 0.0],
                [1.0, 1.0],
            ));
    }

    let mode = match additional {
        Some(CartographyAdditionalItemKind::Paper) if invalid_transform => {
            CartographyResultMode::Map
        }
        Some(CartographyAdditionalItemKind::Paper) => CartographyResultMode::Scaled,
        Some(CartographyAdditionalItemKind::EmptyMap) => CartographyResultMode::Duplicated,
        Some(CartographyAdditionalItemKind::GlassPane) => CartographyResultMode::Locked,
        None => result_slot_mode.unwrap_or(CartographyResultMode::Map),
    };
    presentation.mode = Some(mode);
    presentation.invalid_transform = invalid_transform;
    push_cartography_table_mode_layers(&mut presentation, map_state.map(|_| map_id), mode);
    presentation
}

fn push_cartography_table_mode_layers(
    presentation: &mut CartographyTablePresentation,
    map_id: Option<i32>,
    mode: CartographyResultMode,
) {
    match mode {
        CartographyResultMode::Map => {
            push_cartography_table_map_layer(
                &mut presentation.background_layers,
                CARTOGRAPHY_TABLE_MAP_X,
                CARTOGRAPHY_TABLE_MAP_Y,
            );
            push_cartography_table_map_preview(presentation, map_id, 71, 17, 0.45);
        }
        CartographyResultMode::Scaled => {
            presentation
                .background_layers
                .push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite("container/cartography_table/scaled_map"),
                    CARTOGRAPHY_TABLE_MAP_X,
                    CARTOGRAPHY_TABLE_MAP_Y,
                    CARTOGRAPHY_TABLE_MAP_SIZE,
                    CARTOGRAPHY_TABLE_MAP_SIZE,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            push_cartography_table_map_preview(presentation, map_id, 85, 31, 0.226);
        }
        CartographyResultMode::Duplicated => {
            presentation
                .background_layers
                .push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite(
                        "container/cartography_table/duplicated_map",
                    ),
                    CARTOGRAPHY_TABLE_MAP_X + CARTOGRAPHY_TABLE_DUPLICATED_MAP_OFFSET,
                    CARTOGRAPHY_TABLE_MAP_Y,
                    CARTOGRAPHY_TABLE_DUPLICATED_MAP_WIDTH,
                    CARTOGRAPHY_TABLE_DUPLICATED_MAP_HEIGHT,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            push_cartography_table_map_preview(presentation, map_id, 86, 16, 0.34);
            presentation
                .background_layers
                .push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite(
                        "container/cartography_table/duplicated_map",
                    ),
                    CARTOGRAPHY_TABLE_MAP_X,
                    CARTOGRAPHY_TABLE_MAP_Y + CARTOGRAPHY_TABLE_DUPLICATED_MAP_OFFSET,
                    CARTOGRAPHY_TABLE_DUPLICATED_MAP_WIDTH,
                    CARTOGRAPHY_TABLE_DUPLICATED_MAP_HEIGHT,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            push_cartography_table_map_preview(presentation, map_id, 70, 32, 0.34);
        }
        CartographyResultMode::Locked => {
            push_cartography_table_map_layer(
                &mut presentation.background_layers,
                CARTOGRAPHY_TABLE_MAP_X,
                CARTOGRAPHY_TABLE_MAP_Y,
            );
            push_cartography_table_map_preview(presentation, map_id, 71, 17, 0.45);
            presentation
                .background_layers
                .push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Sprite("container/cartography_table/locked"),
                    CARTOGRAPHY_TABLE_LOCKED_X,
                    CARTOGRAPHY_TABLE_LOCKED_Y,
                    CARTOGRAPHY_TABLE_LOCKED_WIDTH,
                    CARTOGRAPHY_TABLE_LOCKED_HEIGHT,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
        }
    }
}

fn push_cartography_table_map_preview(
    presentation: &mut CartographyTablePresentation,
    map_id: Option<i32>,
    x: i32,
    y: i32,
    scale: f32,
) {
    let Some(map_id) = map_id else {
        return;
    };
    presentation.map_previews.push(HudInventoryMapPreview {
        map_id,
        x,
        y,
        scale,
        after_background_layer: presentation.background_layers.len().saturating_sub(1),
    });
}

fn push_cartography_table_map_layer(layers: &mut Vec<HudInventoryBackgroundLayer>, x: i32, y: i32) {
    layers.push(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Sprite("container/cartography_table/map"),
        x,
        y,
        CARTOGRAPHY_TABLE_MAP_SIZE,
        CARTOGRAPHY_TABLE_MAP_SIZE,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
}

fn cartography_result_mode(item: &ItemStackSummary) -> Option<CartographyResultMode> {
    if item_stack_is_empty(item) || item_stack_map_id(item).is_none() {
        return None;
    }
    match item.component_patch.map_post_processing {
        Some(MapPostProcessingSummary::Scale) => Some(CartographyResultMode::Scaled),
        Some(MapPostProcessingSummary::Lock) => Some(CartographyResultMode::Locked),
        None if item.count >= 2 => Some(CartographyResultMode::Duplicated),
        None => Some(CartographyResultMode::Map),
    }
}

fn cartography_additional_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &ItemStackSummary,
) -> Option<CartographyAdditionalItemKind> {
    if item_stack_is_empty(item) {
        return None;
    }
    let item_id = item.item_id?;
    world.cartography_additional_item_kind(item_id).or_else(|| {
        cartography_additional_item_for_resource_id(
            item_runtime?.item_resource_id_for_protocol_id(item_id)?,
        )
    })
}

pub(super) fn cartography_additional_item_for_resource_id(
    resource_id: &str,
) -> Option<CartographyAdditionalItemKind> {
    match resource_id {
        "minecraft:paper" => Some(CartographyAdditionalItemKind::Paper),
        "minecraft:map" => Some(CartographyAdditionalItemKind::EmptyMap),
        "minecraft:glass_pane" => Some(CartographyAdditionalItemKind::GlassPane),
        _ => None,
    }
}

fn item_stack_map_id(item: &ItemStackSummary) -> Option<i32> {
    if item_stack_is_empty(item)
        || item
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID)
    {
        return None;
    }
    item.component_patch.map_id
}

fn anvil_should_show_error(world: &WorldStore) -> bool {
    (open_container_slot_has_item(world, 0) || open_container_slot_has_item(world, 1))
        && !open_container_slot_has_item(world, 2)
}

pub(super) fn open_container_slot_has_item(world: &WorldStore, slot_num: i16) -> bool {
    open_container_slot_item(world, slot_num).is_some_and(|item| !item_stack_is_empty(item))
}

pub(super) fn open_container_slot_item(
    world: &WorldStore,
    slot_num: i16,
) -> Option<&ItemStackSummary> {
    world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| world.inventory().container_slot(container, slot_num))
        .map(|slot| slot.item)
}

fn push_brewing_stand_progress_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
) {
    let fuel = world
        .open_container_data_value(BREWING_STAND_FUEL_DATA_ID)
        .unwrap_or_default();
    let max_fuel_length = i32::try_from(BREWING_STAND_FUEL_LENGTH_SPRITE_WIDTH).unwrap_or_default();
    let fuel_length = ((max_fuel_length * i32::from(fuel) + 20 - 1) / 20).clamp(0, max_fuel_length);
    if fuel_length > 0 {
        let width = u32::try_from(fuel_length).unwrap_or_default();
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/brewing_stand/fuel_length"),
            60,
            44,
            width,
            BREWING_STAND_FUEL_LENGTH_SPRITE_HEIGHT,
            [0.0, 0.0],
            [
                width as f32 / BREWING_STAND_FUEL_LENGTH_SPRITE_WIDTH as f32,
                1.0,
            ],
        ));
    }

    let brew_ticks = world
        .open_container_data_value(BREWING_STAND_BREW_TIME_DATA_ID)
        .unwrap_or_default();
    if brew_ticks <= 0 {
        return;
    }

    let brew_length = (BREWING_STAND_BREW_PROGRESS_SPRITE_HEIGHT as f32
        * (1.0 - f32::from(brew_ticks) / BREWING_STAND_BREW_TOTAL_TICKS))
        as i32;
    if brew_length > 0 {
        let height = u32::try_from(brew_length).unwrap_or_default();
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/brewing_stand/brew_progress"),
            97,
            16,
            BREWING_STAND_BREW_PROGRESS_SPRITE_WIDTH,
            height,
            [0.0, 0.0],
            [
                1.0,
                height as f32 / BREWING_STAND_BREW_PROGRESS_SPRITE_HEIGHT as f32,
            ],
        ));
    }

    let bubble_length =
        BREWING_STAND_BUBBLE_LENGTHS[(usize::try_from(brew_ticks).unwrap_or_default() / 2)
            % BREWING_STAND_BUBBLE_LENGTHS.len()];
    if bubble_length > 0 {
        let y_offset = BREWING_STAND_BUBBLES_SPRITE_HEIGHT - bubble_length;
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite("container/brewing_stand/bubbles"),
            63,
            14 + i32::try_from(y_offset).unwrap_or_default(),
            BREWING_STAND_BUBBLES_SPRITE_WIDTH,
            bubble_length,
            [
                0.0,
                y_offset as f32 / BREWING_STAND_BUBBLES_SPRITE_HEIGHT as f32,
            ],
            [1.0, 1.0],
        ));
    }
}

fn grindstone_should_show_error(world: &WorldStore) -> bool {
    let Some(container) = world.inventory().open_container.as_ref() else {
        return false;
    };
    let input_has_item = [0, 1].into_iter().any(|slot_num| {
        world
            .inventory()
            .container_slot(container, slot_num)
            .is_some_and(|slot| !item_stack_is_empty(slot.item))
    });
    let result_has_item = world
        .inventory()
        .container_slot(container, 2)
        .is_some_and(|slot| !item_stack_is_empty(slot.item));
    input_has_item && !result_has_item
}

pub(super) fn smithing_should_show_error(world: &WorldStore) -> bool {
    world
        .open_container_data_value(SMITHING_RECIPE_ERROR_DATA_ID)
        .unwrap_or_default()
        > 0
}

fn push_furnace_progress_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    lit_texture: HudInventoryBackgroundTexture,
    burn_texture: HudInventoryBackgroundTexture,
) {
    let lit_time = world
        .open_container_data_value(FURNACE_LIT_TIME_DATA_ID)
        .unwrap_or_default();
    if lit_time > 0 {
        let lit_duration = world
            .open_container_data_value(FURNACE_LIT_DURATION_DATA_ID)
            .filter(|duration| *duration != 0)
            .unwrap_or(FURNACE_DEFAULT_LIT_DURATION);
        let lit_progress = (f32::from(lit_time) / f32::from(lit_duration)).clamp(0.0, 1.0);
        let lit_height = (lit_progress * 13.0).ceil() as u32 + 1;
        let lit_offset = FURNACE_LIT_PROGRESS_SPRITE_SIZE - lit_height;
        layers.push(hud_inventory_background_layer(
            lit_texture,
            56,
            36 + i32::try_from(lit_offset).unwrap_or_default(),
            FURNACE_LIT_PROGRESS_SPRITE_SIZE,
            lit_height,
            [
                0.0,
                lit_offset as f32 / FURNACE_LIT_PROGRESS_SPRITE_SIZE as f32,
            ],
            [1.0, 1.0],
        ));
    }

    let burn_current = world
        .open_container_data_value(FURNACE_COOKING_PROGRESS_DATA_ID)
        .unwrap_or_default();
    let burn_total = world
        .open_container_data_value(FURNACE_COOKING_TOTAL_TIME_DATA_ID)
        .unwrap_or_default();
    let burn_progress = if burn_total != 0 && burn_current != 0 {
        (f32::from(burn_current) / f32::from(burn_total)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let burn_width = (burn_progress * FURNACE_BURN_PROGRESS_SPRITE_WIDTH as f32).ceil() as u32;
    if burn_width > 0 {
        layers.push(hud_inventory_background_layer(
            burn_texture,
            79,
            34,
            burn_width,
            FURNACE_BURN_PROGRESS_SPRITE_HEIGHT,
            [0.0, 0.0],
            [
                burn_width as f32 / FURNACE_BURN_PROGRESS_SPRITE_WIDTH as f32,
                1.0,
            ],
        ));
    }
}

pub(super) fn hud_inventory_background_layer(
    texture: HudInventoryBackgroundTexture,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
) -> HudInventoryBackgroundLayer {
    HudInventoryBackgroundLayer {
        texture,
        x,
        y,
        width,
        height,
        uv: HudUvRect {
            min: uv_min,
            max: uv_max,
        },
    }
}
