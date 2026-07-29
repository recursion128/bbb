//! Vanilla 26.1 Loom screen presentation projection.
//!
//! Canonical menu stacks and synchronized banner-pattern holders remain
//! world-owned. This module owns the screen-local visibility, strata,
//! thumbnail, tooltip, and result-banner descriptors consumed by the HUD.

use bbb_item_model::NativeItemRuntime;
use bbb_protocol::{
    builtin_enum_component_values::{vanilla_dye_color_name, VANILLA_DYE_COLOR_WHITE_ID},
    data_component_types::{
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    },
    item_types::vanilla_item_type,
};
use bbb_renderer::{
    BannerPatternLayer, EntityDyeColor, HudInventoryBackgroundLayer, HudInventoryBackgroundTexture,
    HudInventoryBannerPatternPreview, HudInventoryTooltip, HudInventoryTooltipLine, HudUvRect,
};
use bbb_world::{banner_color_and_form_for_block_name, WorldStore};

use crate::{
    banner_scene::{banner_component_pattern_layers, banner_dye_color, banner_pattern_texture},
    entity_variant_textures::NativeEntityVariantTextureRuntime,
    input::{
        LOOM_PATTERN_BUTTON_COLUMNS, LOOM_PATTERN_BUTTON_ROWS, LOOM_PATTERN_BUTTON_SIZE,
        LOOM_PATTERN_BUTTON_X, LOOM_PATTERN_BUTTON_Y, LOOM_SCROLLER_HEIGHT,
        LOOM_SCROLLER_MAX_OFFSET, LOOM_SCROLLER_WIDTH, LOOM_SCROLLER_X, LOOM_SCROLLER_Y,
        LOOM_SELECTED_PATTERN_DATA_ID,
    },
};

use super::{hud_inventory_background_layer, item_stack_is_empty, localized_gui_text};

const LOOM_BANNER_SLOT: i16 = 0;
const LOOM_DYE_SLOT: i16 = 1;
const LOOM_PATTERN_SLOT: i16 = 2;
const LOOM_RESULT_SLOT: i16 = 3;
const LOOM_SLOT_SPRITE_SIZE: u32 = 16;
const LOOM_MAX_INPUT_PATTERNS: usize = 6;
const LOOM_RESULT_ERROR_X: i32 = 138;
const LOOM_RESULT_ERROR_Y: i32 = 52;
const LOOM_RESULT_ERROR_SIZE: u32 = 26;
const LOOM_THUMBNAIL_X_OFFSET: i32 = 4;
const LOOM_THUMBNAIL_Y_OFFSET: i32 = 2;
const LOOM_THUMBNAIL_WIDTH: u32 = 5;
const LOOM_THUMBNAIL_HEIGHT: u32 = 10;
const LOOM_PATTERN_TEXTURE_SIZE: f32 = 64.0;
const LOOM_PATTERN_TEXTURE_WIDTH: f32 = 21.0;
const LOOM_PATTERN_TEXTURE_Y: f32 = 1.0;
const LOOM_PATTERN_TEXTURE_HEIGHT: f32 = 40.0;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LoomTablePresentation {
    pub background_layers: Vec<HudInventoryBackgroundLayer>,
    pub pattern_previews: Vec<HudInventoryBannerPatternPreview>,
    pub result: Option<LoomBannerResultPresentation>,
    pub tooltip: Option<HudInventoryTooltip>,
    pub display_patterns: bool,
    pub has_max_patterns: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LoomBannerResultPresentation {
    pub base_color: EntityDyeColor,
    pub layers: [Option<BannerPatternLayer>; 16],
    pub after_background_layer: usize,
}

pub(super) fn loom_table_presentation(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    textures: Option<&NativeEntityVariantTextureRuntime>,
    scroll_row: i32,
    selected_pattern_index: Option<i32>,
    cursor_position: Option<(i32, i32)>,
) -> LoomTablePresentation {
    let mut background_layers = Vec::new();
    for (slot, texture, x, y) in [
        (
            LOOM_BANNER_SLOT,
            HudInventoryBackgroundTexture::LoomBannerSlot,
            13,
            26,
        ),
        (
            LOOM_DYE_SLOT,
            HudInventoryBackgroundTexture::LoomDyeSlot,
            33,
            26,
        ),
        (
            LOOM_PATTERN_SLOT,
            HudInventoryBackgroundTexture::LoomPatternSlot,
            23,
            45,
        ),
    ] {
        if open_container_slot_item(world, slot).is_some_and(|item| !item_stack_is_empty(item)) {
            continue;
        }
        background_layers.push(hud_inventory_background_layer(
            texture,
            x,
            y,
            LOOM_SLOT_SPRITE_SIZE,
            LOOM_SLOT_SPRITE_SIZE,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    let has_max_patterns = loom_has_max_patterns(world);
    let selectable_ids = world.loom_selectable_banner_pattern_ids();
    let selectable_count = selectable_ids
        .as_ref()
        .and_then(|ids| i32::try_from(ids.len()).ok());
    let display_patterns =
        !has_max_patterns && selectable_count.is_some_and(|selectable_count| selectable_count > 0);
    let can_scroll = display_patterns
        && selectable_count
            .is_some_and(|count| count > LOOM_PATTERN_BUTTON_COLUMNS * LOOM_PATTERN_BUTTON_ROWS);
    background_layers.push(hud_inventory_background_layer(
        if can_scroll {
            HudInventoryBackgroundTexture::LoomScroller
        } else {
            HudInventoryBackgroundTexture::LoomScrollerDisabled
        },
        LOOM_SCROLLER_X,
        LOOM_SCROLLER_Y
            + selectable_count
                .map(|count| loom_scroller_offset(count, scroll_row))
                .unwrap_or_default(),
        u32::try_from(LOOM_SCROLLER_WIDTH).unwrap_or_default(),
        u32::try_from(LOOM_SCROLLER_HEIGHT).unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    ));
    // The base Loom texture is background layer zero. The current supplemental
    // length therefore equals the full-screen index of its just-pushed layer.
    let scroller_layer = background_layers.len();

    let result = (!has_max_patterns)
        .then(|| loom_result_presentation(world, item_runtime, textures, scroller_layer))
        .flatten();
    if has_max_patterns {
        background_layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::LoomError,
            LOOM_RESULT_ERROR_X,
            LOOM_RESULT_ERROR_Y,
            LOOM_RESULT_ERROR_SIZE,
            LOOM_RESULT_ERROR_SIZE,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    let mut pattern_previews = Vec::new();
    let mut tooltip = None;
    if display_patterns {
        let selectable_ids = selectable_ids.as_deref().unwrap_or_default();
        let selectable_count = i32::try_from(selectable_ids.len()).unwrap_or_default();
        let selected_pattern_index =
            selected_pattern_index.or_else(|| loom_selected_pattern_index(world));
        let start_index =
            loom_pattern_start_index(selectable_count, scroll_row).unwrap_or_default();
        for position in 0..(LOOM_PATTERN_BUTTON_COLUMNS * LOOM_PATTERN_BUTTON_ROWS) {
            let pattern_index = start_index + position;
            let Some(holder_id) = usize::try_from(pattern_index)
                .ok()
                .and_then(|index| selectable_ids.get(index))
                .copied()
            else {
                break;
            };
            let column = position % LOOM_PATTERN_BUTTON_COLUMNS;
            let row = position / LOOM_PATTERN_BUTTON_COLUMNS;
            let x = LOOM_PATTERN_BUTTON_X + column * LOOM_PATTERN_BUTTON_SIZE;
            let y = LOOM_PATTERN_BUTTON_Y + row * LOOM_PATTERN_BUTTON_SIZE;
            let selected = selected_pattern_index == Some(pattern_index);
            let hovered = cursor_position.is_some_and(|(cursor_x, cursor_y)| {
                cursor_x >= x
                    && cursor_x < x + LOOM_PATTERN_BUTTON_SIZE
                    && cursor_y >= y
                    && cursor_y < y + LOOM_PATTERN_BUTTON_SIZE
            });
            background_layers.push(hud_inventory_background_layer(
                if selected {
                    HudInventoryBackgroundTexture::LoomPatternSelected
                } else if hovered {
                    HudInventoryBackgroundTexture::LoomPatternHighlighted
                } else {
                    HudInventoryBackgroundTexture::LoomPattern
                },
                x,
                y,
                u32::try_from(LOOM_PATTERN_BUTTON_SIZE).unwrap_or_default(),
                u32::try_from(LOOM_PATTERN_BUTTON_SIZE).unwrap_or_default(),
                [0.0, 0.0],
                [1.0, 1.0],
            ));
            let presentation = world
                .synchronized_banner_pattern_presentation(holder_id)
                .ok()
                .flatten();
            pattern_previews.push(HudInventoryBannerPatternPreview {
                texture: presentation.as_ref().and_then(|presentation| {
                    banner_pattern_texture(&presentation.asset_id, textures)
                }),
                x: x + LOOM_THUMBNAIL_X_OFFSET,
                y: y + LOOM_THUMBNAIL_Y_OFFSET,
                width: LOOM_THUMBNAIL_WIDTH,
                height: LOOM_THUMBNAIL_HEIGHT,
                uv: HudUvRect {
                    min: [0.0, LOOM_PATTERN_TEXTURE_Y / LOOM_PATTERN_TEXTURE_SIZE],
                    max: [
                        LOOM_PATTERN_TEXTURE_WIDTH / LOOM_PATTERN_TEXTURE_SIZE,
                        (LOOM_PATTERN_TEXTURE_Y + LOOM_PATTERN_TEXTURE_HEIGHT)
                            / LOOM_PATTERN_TEXTURE_SIZE,
                    ],
                },
                after_background_layer: background_layers.len(),
            });
            if hovered && !selected {
                tooltip = presentation.map(|presentation| {
                    loom_pattern_tooltip(
                        world,
                        item_runtime,
                        &presentation.translation_key,
                        cursor_position.expect("hovered Loom button has cursor"),
                    )
                });
            }
        }
    }

    LoomTablePresentation {
        background_layers,
        pattern_previews,
        result,
        tooltip,
        display_patterns,
        has_max_patterns,
    }
}

fn loom_has_max_patterns(world: &WorldStore) -> bool {
    let Some(banner) = open_container_slot_item(world, LOOM_BANNER_SLOT) else {
        return false;
    };
    world.item_stack_has_effective_component(banner, VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID)
        && world
            .effective_item_stack(banner)
            .component_patch
            .banner_pattern_layers
            .len()
            >= LOOM_MAX_INPUT_PATTERNS
}

fn loom_result_presentation(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    textures: Option<&NativeEntityVariantTextureRuntime>,
    after_background_layer: usize,
) -> Option<LoomBannerResultPresentation> {
    let result = open_container_slot_item(world, LOOM_RESULT_SLOT)?;
    if item_stack_is_empty(result) {
        return None;
    }
    let item_id = result.item_id?;
    let resource_id = item_runtime
        .and_then(|runtime| runtime.item_resource_id(item_id))
        .or_else(|| vanilla_item_type(item_id).map(|item| item.resource_id))?;
    let (base_color, _) = banner_color_and_form_for_block_name(resource_id)?;
    let effective = world.effective_item_stack(result);
    let layers = world
        .item_stack_has_effective_component(result, VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID)
        .then(|| {
            banner_component_pattern_layers(
                world,
                &effective.component_patch.banner_pattern_layers,
                textures,
            )
        })
        .unwrap_or([None; 16]);
    Some(LoomBannerResultPresentation {
        base_color: banner_dye_color(base_color),
        layers,
        after_background_layer,
    })
}

fn loom_pattern_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    translation_key: &str,
    cursor_position: (i32, i32),
) -> HudInventoryTooltip {
    let color = open_container_slot_item(world, LOOM_DYE_SLOT)
        .map(|dye| {
            if world.item_stack_has_effective_component(dye, VANILLA_DATA_COMPONENT_DYE_TYPE_ID) {
                world
                    .effective_item_stack(dye)
                    .component_patch
                    .dye_color_id
                    .unwrap_or(VANILLA_DYE_COLOR_WHITE_ID)
            } else {
                VANILLA_DYE_COLOR_WHITE_ID
            }
        })
        .unwrap_or(VANILLA_DYE_COLOR_WHITE_ID);
    let key = format!("{translation_key}.{}", vanilla_dye_color_name(color));
    HudInventoryTooltip {
        slot_id: u16::MAX,
        x: cursor_position.0,
        y: cursor_position.1,
        style: None,
        lines: vec![HudInventoryTooltipLine {
            text: localized_gui_text(item_runtime, &key, &key),
            tint: [1.0, 1.0, 1.0, 1.0],
            runs: Vec::new(),
        }],
    }
}

fn open_container_slot_item(
    world: &WorldStore,
    slot_num: i16,
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.slots.iter().find(|slot| slot.slot == slot_num))
        .map(|slot| &slot.item)
}

fn loom_selected_pattern_index(world: &WorldStore) -> Option<i32> {
    world
        .open_container_data_value(LOOM_SELECTED_PATTERN_DATA_ID)
        .and_then(|value| (value >= 0).then_some(i32::from(value)))
}

fn loom_pattern_start_index(selectable_count: i32, scroll_row: i32) -> Option<i32> {
    if selectable_count <= 0 {
        return None;
    }
    let max_scroll_row =
        (loom_pattern_row_count(selectable_count) - LOOM_PATTERN_BUTTON_ROWS).max(0);
    Some(scroll_row.clamp(0, max_scroll_row) * LOOM_PATTERN_BUTTON_COLUMNS)
}

fn loom_pattern_row_count(selectable_count: i32) -> i32 {
    if selectable_count <= 0 {
        0
    } else {
        (selectable_count + LOOM_PATTERN_BUTTON_COLUMNS - 1) / LOOM_PATTERN_BUTTON_COLUMNS
    }
}

fn loom_scroller_offset(selectable_count: i32, scroll_row: i32) -> i32 {
    let max_scroll_row =
        (loom_pattern_row_count(selectable_count) - LOOM_PATTERN_BUTTON_ROWS).max(0);
    if max_scroll_row <= 0 {
        return 0;
    }
    let scroll_row = scroll_row.clamp(0, max_scroll_row);
    ((scroll_row as f32 / max_scroll_row as f32) * LOOM_SCROLLER_MAX_OFFSET as f32) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        builtin_enum_component_values::{VANILLA_DYE_COLOR_BLUE_ID, VANILLA_DYE_COLOR_RED_ID},
        data_component_types::{
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
        },
        item_types::{
            VANILLA_ITEM_RED_BANNER_ID, VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_WHITE_BANNER_ID,
        },
        packets::{
            BannerPatternLayerSummary, ContainerSetContent, ItemStackSummary, OpenScreen,
            TagNetworkPayload, UpdateTags,
        },
    };
    use bbb_renderer::{BannerPatternKind, BannerPatternTexture};
    use bbb_world::RegistryPacketEntry;

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }

    fn banner_pattern_registry_nbt(asset_id: &str, translation_key: &str) -> Vec<u8> {
        let mut payload = vec![10, 8];
        write_mutf8(&mut payload, "asset_id");
        write_mutf8(&mut payload, asset_id);
        payload.push(8);
        write_mutf8(&mut payload, "translation_key");
        write_mutf8(&mut payload, translation_key);
        payload.push(0);
        payload
    }

    fn stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn install_registry(world: &mut WorldStore) {
        world.record_registry_entries(
            bbb_world::BANNER_PATTERN_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "minecraft:stripe_top",
                    banner_pattern_registry_nbt(
                        "minecraft:creeper",
                        "block.minecraft.banner.creeper",
                    ),
                ),
                RegistryPacketEntry::with_raw_data(
                    "minecraft:creeper",
                    banner_pattern_registry_nbt(
                        "minecraft:stripe_top",
                        "block.minecraft.banner.stripe_top",
                    ),
                ),
            ],
        );
        world.apply_update_tags(UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: bbb_world::BANNER_PATTERN_REGISTRY.to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:no_item_required".to_string(),
                    entries: vec![0, 1],
                }],
            }],
        });
    }

    fn open_loom(
        world: &mut WorldStore,
        banner: ItemStackSummary,
        dye: ItemStackSummary,
        result: ItemStackSummary,
    ) {
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: bbb_protocol::menu_types::VANILLA_MENU_TYPE_LOOM_ID,
            title: bbb_protocol::RichTextComponent::plain("Loom"),
        });
        let mut items = vec![ItemStackSummary::empty(); 40];
        items[LOOM_BANNER_SLOT as usize] = banner;
        items[LOOM_DYE_SLOT as usize] = dye;
        items[LOOM_RESULT_SLOT as usize] = result;
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: ItemStackSummary::empty(),
        });
    }

    fn red_dye() -> ItemStackSummary {
        let mut dye = stack(VANILLA_ITEM_RED_DYE_ID, 1);
        dye.component_patch.added = 1;
        dye.component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_DYE_TYPE_ID);
        dye.component_patch.dye_color_id = Some(VANILLA_DYE_COLOR_RED_ID);
        dye
    }

    fn patterned_result() -> ItemStackSummary {
        let mut result = stack(VANILLA_ITEM_RED_BANNER_ID, 1);
        result.component_patch.added = 1;
        result
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID);
        result
            .component_patch
            .banner_pattern_layers
            .push(BannerPatternLayerSummary {
                registry_id: Some(1),
                asset_id: None,
                translation_key: None,
                color_id: VANILLA_DYE_COLOR_BLUE_ID,
            });
        result
    }

    #[test]
    fn projects_current_session_thumbnails_hover_tooltip_and_result() {
        let mut world = WorldStore::new();
        install_registry(&mut world);
        open_loom(
            &mut world,
            stack(VANILLA_ITEM_WHITE_BANNER_ID, 1),
            red_dye(),
            patterned_result(),
        );

        let presentation = loom_table_presentation(&world, None, None, 0, None, Some((61, 14)));

        assert!(presentation.display_patterns);
        assert!(!presentation.has_max_patterns);
        let pattern_buttons = presentation
            .background_layers
            .iter()
            .filter(|layer| {
                matches!(
                    layer.texture,
                    HudInventoryBackgroundTexture::LoomPattern
                        | HudInventoryBackgroundTexture::LoomPatternSelected
                        | HudInventoryBackgroundTexture::LoomPatternHighlighted
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(pattern_buttons.len(), 2);
        assert_eq!(
            pattern_buttons[0].texture,
            HudInventoryBackgroundTexture::LoomPatternHighlighted
        );
        assert_eq!(presentation.pattern_previews.len(), 2);
        assert_eq!(
            presentation.pattern_previews[0].texture,
            Some(BannerPatternTexture::Builtin(BannerPatternKind::Creeper))
        );
        assert_eq!(
            presentation.pattern_previews[1].texture,
            Some(BannerPatternTexture::Builtin(BannerPatternKind::StripeTop))
        );
        assert_eq!(
            (
                presentation.pattern_previews[0].x,
                presentation.pattern_previews[0].y,
                presentation.pattern_previews[0].width,
                presentation.pattern_previews[0].height,
                presentation.pattern_previews[0].after_background_layer,
            ),
            (64, 15, 5, 10, 3)
        );
        assert_eq!(
            presentation.pattern_previews[0].uv,
            HudUvRect {
                min: [0.0, 1.0 / 64.0],
                max: [21.0 / 64.0, 41.0 / 64.0],
            }
        );
        assert_eq!(
            presentation
                .tooltip
                .as_ref()
                .and_then(|tooltip| tooltip.lines.first())
                .map(|line| line.text.as_str()),
            Some("block.minecraft.banner.creeper.red")
        );

        let result = presentation.result.expect("result banner");
        assert_eq!(result.base_color, EntityDyeColor::Red);
        assert_eq!(result.after_background_layer, 2);
        assert_eq!(
            result.layers[0],
            Some(BannerPatternLayer {
                texture: BannerPatternTexture::Builtin(BannerPatternKind::StripeTop),
                color: EntityDyeColor::Blue,
            })
        );
    }

    #[test]
    fn selected_button_wins_over_hover_and_suppresses_tooltip() {
        let mut world = WorldStore::new();
        install_registry(&mut world);
        open_loom(
            &mut world,
            stack(VANILLA_ITEM_WHITE_BANNER_ID, 1),
            red_dye(),
            ItemStackSummary::empty(),
        );

        let presentation = loom_table_presentation(&world, None, None, 0, Some(0), Some((61, 14)));

        assert_eq!(
            presentation
                .background_layers
                .iter()
                .find(|layer| {
                    matches!(
                        layer.texture,
                        HudInventoryBackgroundTexture::LoomPatternSelected
                            | HudInventoryBackgroundTexture::LoomPatternHighlighted
                            | HudInventoryBackgroundTexture::LoomPattern
                    )
                })
                .map(|layer| layer.texture),
            Some(HudInventoryBackgroundTexture::LoomPatternSelected)
        );
        assert!(presentation.tooltip.is_none());
    }

    #[test]
    fn six_input_patterns_show_only_the_error_after_the_scroller() {
        let mut world = WorldStore::new();
        install_registry(&mut world);
        let mut banner = stack(VANILLA_ITEM_WHITE_BANNER_ID, 1);
        banner.component_patch.added = 1;
        banner
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID);
        banner.component_patch.banner_pattern_layers =
            vec![BannerPatternLayerSummary::default(); 6];
        open_loom(&mut world, banner, red_dye(), patterned_result());

        let presentation = loom_table_presentation(&world, None, None, 0, None, Some((61, 14)));

        assert!(presentation.has_max_patterns);
        assert!(!presentation.display_patterns);
        assert!(presentation.result.is_none());
        assert!(presentation.pattern_previews.is_empty());
        assert!(presentation.tooltip.is_none());
        assert_eq!(
            presentation
                .background_layers
                .last()
                .map(|layer| (layer.texture, layer.x, layer.y)),
            Some((HudInventoryBackgroundTexture::LoomError, 138, 52))
        );
        assert_eq!(presentation.background_layers.len(), 3);
    }
}
