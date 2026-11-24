use std::collections::BTreeMap;
#[cfg(test)]
use std::sync::Arc;

use anyhow::Result;
use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::entity_models::{
    banner_pattern_texture_ref, BannerPatternTexture, EntityDyeColor, EntityModelInstance,
    EntityModelUvRect, SignModelAttachment, SignModelWood, ENTITY_FULL_BRIGHT_LIGHT_COORDS,
};
use crate::item_models::{
    GuiItemLightingEntry, HudBlockItemModel, HudResidentItemLayer, ItemModelAssetRequest,
    ItemModelDrawPhase, ItemModelDrawTarget, ItemModelFoil, ItemModelGeometryClass,
    ItemModelInstanceRequest, ItemModelResidencyFrame, SpecialItemInstanceRequest,
    ITEM_MODEL_FULL_BRIGHT_LIGHT, ITEM_MODEL_NO_OVERLAY,
};
#[cfg(test)]
use crate::item_models::{ItemModelAssetSource, ItemModelAtlasKind};
use crate::Renderer;

mod book;
mod chat_input;
mod code_of_conduct;
mod command_confirmation;
mod creative_inventory;
mod death_screen;
mod disconnected_screen;
mod game_event;
mod game_rules;
mod gpu;
mod gui_atlas;
mod ime_preedit;
mod layout;
mod level_loading_screen;
mod player_list;
mod scoreboard;
mod social;
mod sound_subtitles;
mod spectator;
mod text_measurements;

pub use self::book::{
    hud_book_layout, HudBookHitTarget, HudBookInteraction, HudBookLayout, HudBookMode,
    HudBookScreen, HUD_BOOK_HEIGHT, HUD_BOOK_LINE_HEIGHT, HUD_BOOK_MAX_LINES, HUD_BOOK_PAGE_BACK_X,
    HUD_BOOK_PAGE_BUTTON_HEIGHT, HUD_BOOK_PAGE_BUTTON_WIDTH, HUD_BOOK_PAGE_BUTTON_Y,
    HUD_BOOK_PAGE_FORWARD_X, HUD_BOOK_PAGE_INDICATOR_RIGHT, HUD_BOOK_PAGE_INDICATOR_Y,
    HUD_BOOK_TEXT_WIDTH, HUD_BOOK_TEXT_X, HUD_BOOK_TEXT_Y, HUD_BOOK_TOP, HUD_BOOK_WIDTH,
};
pub use self::code_of_conduct::{HudCodeOfConductRect, HudCodeOfConductScreen};
pub use self::command_confirmation::{
    HudCommandConfirmation, HudCommandConfirmationButton, HudCommandConfirmationHitTarget,
};
pub use self::creative_inventory::{
    hud_creative_inventory_background_layers, hud_creative_inventory_scroller_y,
    hud_creative_inventory_slot_layouts, hud_creative_inventory_tab_hit_y,
    hud_creative_inventory_tab_x, HudCreativeInventoryBackground, HudCreativeInventoryChrome,
    HudCreativeInventorySlot, HudCreativeInventorySlotLayout, HudCreativeInventorySlotLayoutKind,
    HudCreativeInventoryTab, HudCreativeInventoryTabRow, HUD_CREATIVE_INVENTORY_HEIGHT,
    HUD_CREATIVE_INVENTORY_WIDTH, HUD_CREATIVE_PICKER_COLUMNS, HUD_CREATIVE_PICKER_ROWS,
    HUD_CREATIVE_PICKER_SLOT_COUNT, HUD_CREATIVE_SCROLLER_X, HUD_CREATIVE_SCROLLER_Y,
    HUD_CREATIVE_SEARCH_HEIGHT, HUD_CREATIVE_SEARCH_WIDTH, HUD_CREATIVE_SEARCH_X,
    HUD_CREATIVE_SEARCH_Y, HUD_CREATIVE_TAB_HEIGHT, HUD_CREATIVE_TAB_WIDTH, HUD_CREATIVE_TRASH_X,
    HUD_CREATIVE_TRASH_Y,
};
pub use self::death_screen::{
    HudDeathLinkConfirmationScreen, HudDeathMainScreen, HudDeathQuitConfirmationScreen,
    HudDeathScreen, HudDeathScreenButton, HudDeathScreenHitTarget, HudDeathScreenHitTargetKind,
};
pub use self::disconnected_screen::{
    HudDisconnectedScreen, HudDisconnectedScreenButton, HudDisconnectedScreenHitTarget,
};
pub use self::game_event::{
    HudDemoLinkConfirmationScreen, HudDemoPopupScreen, HudGameEventButton, HudGameEventHitTarget,
    HudGameEventScreen, HudWinScreen, HudWinScreenLine,
};
pub use self::game_rules::{
    hud_game_rules_confirmation_buttons, hud_game_rules_layout, HudGameRulesDiscardConfirmation,
    HudGameRulesLayout, HudGameRulesRect, HudGameRulesRow, HudGameRulesScreen,
    HUD_GAME_RULES_FOOTER_HEIGHT, HUD_GAME_RULES_HEADER_HEIGHT, HUD_GAME_RULES_LIST_WIDTH,
    HUD_GAME_RULES_ROW_HEIGHT, HUD_GAME_RULES_SEARCH_HEIGHT, HUD_GAME_RULES_SEARCH_WIDTH,
    HUD_GAME_RULES_VALUE_HEIGHT, HUD_GAME_RULES_VALUE_WIDTH,
};
pub(super) use self::gpu::{
    create_hud_bind_group_layout, create_hud_gui_pipelines, create_hud_item_glint_pipeline,
    create_hud_nausea_overlay_pipeline, create_hud_pipeline, create_hud_sprite_gpu,
    create_standalone_hud_sprite_gpu, HudGuiPipelines, HudSpriteGpu, HUD_CROSSHAIR_BLEND,
};
pub(crate) use self::gui_atlas::GuiSprites;
pub use self::gui_atlas::GUI_TEXTURES;
pub use self::ime_preedit::{
    hud_inventory_text_field_ime_cursor_area, hud_text_field_preedit_overlay,
};
#[cfg(test)]
use self::layout::hud_rect_bounds;
use self::layout::{
    absolute_hud_rect, air_bubble_hud_rect, armor_hud_rect, boss_bar_hud_rect, centered_hud_rect,
    clip_hud_rect_to_viewport, experience_bar_hud_rect, food_hud_rect, full_screen_hud_rect,
    gui_item_slot_placement, hotbar_hud_rect, hotbar_item_hud_rect, hotbar_offhand_hud_rect,
    hotbar_offhand_item_hud_rect, hotbar_selection_hud_rect, hud_air_bubble_icons,
    hud_air_bubble_pop as air_bubble_pop, hud_air_bubble_wobble_offsets, hud_air_bubbles_visible,
    hud_armor_fill, hud_boss_bar_fill_uv, hud_boss_bar_name_origin, hud_boss_bar_progress_width,
    hud_boss_bar_rows, hud_confusion_overlay_corners, hud_contextual_bar_progress_width,
    hud_crosshair_attack_indicator_layout, hud_experience_level_text_origin, hud_food_fill,
    hud_food_jitter_offsets, hud_health_row_geometry, hud_hotbar_attack_indicator_layout,
    hud_inventory_text_label_origin, hud_inventory_tooltip_background_hud_rect,
    hud_inventory_tooltip_line_origin, hud_inventory_tooltip_text_height, hud_item_cooldown_rect,
    hud_item_count_digit_quad, hud_item_durability_bar_rect, hud_item_pop_corners,
    hud_item_pop_placement, hud_locator_arrow_rect, hud_locator_marker_rect,
    hud_overlay_message_text_origin, hud_player_heart_instances, hud_quad_vertices,
    hud_rect_intersection_uv_span, hud_selected_item_name_origin, hud_spyglass_scope_layout,
    hud_styled_quad_vertices, hud_subtitle_text_origin, hud_title_text_origin,
    hud_vehicle_heart_fill, hud_vehicle_max_hearts, inventory_background_hud_rect,
    inventory_slot_highlight_hud_rect, inventory_slot_item_hud_rect, nine_slice_segments,
    tiled_sprite_segments, vehicle_heart_hud_rect, HudAirBubbleIcon, HudIconFill, HudRect,
    HUD_AIR_BUBBLES_PER_ROW, HUD_ARMOR_ICONS_PER_ROW, HUD_BOSS_BAR_WIDTH, HUD_FOOD_ICONS_PER_ROW,
    HUD_INVENTORY_ITEM_SIZE, HUD_SINGLE_HEALTH_ROW_HEIGHT, HUD_SUBTITLE_TEXT_SCALE,
    HUD_TITLE_TEXT_SCALE, HUD_VEHICLE_HEARTS_PER_ROW,
};
pub use self::level_loading_screen::{HudLevelLoadingBackground, HudLevelLoadingScreen};
pub use self::player_list::{
    HudPlayerList, HudPlayerListEntry, HudPlayerListPingSprite, HudPlayerListScore,
};
pub(crate) use self::player_list::{HudPlayerListHeartSprites, HudPlayerListRuntime};
pub use self::scoreboard::{HudScoreboardSidebar, HudScoreboardSidebarEntry};
pub use self::social::{
    hud_social_interactions_layout, hud_social_interactions_max_scroll_amount,
    HudSocialInteractionsFocus, HudSocialInteractionsHitTarget, HudSocialInteractionsHitTargetKind,
    HudSocialInteractionsLayout, HudSocialInteractionsLinkConfirmationScreen,
    HudSocialInteractionsMainScreen, HudSocialInteractionsPage, HudSocialInteractionsPlayer,
    HudSocialInteractionsPlayerAction, HudSocialInteractionsRect, HudSocialInteractionsScreen,
};
pub use self::sound_subtitles::{
    HudSoundSubtitleDirection, HudSoundSubtitleRow, HudSoundSubtitles, HUD_SOUND_SUBTITLE_MAX_ROWS,
};
pub(crate) use self::spectator::HudPlayerFaceGpu;
pub use self::spectator::{HudHotbarMode, HudSpectatorGui, HudSpectatorGuiSlot, HudSpectatorIcon};

pub use bbb_render_types::{
    HudAsciiGlyph, HudComponentTextRun, HudDigitGlyph, HudFontGlyphBook, HudFontGlyphMap,
    HudObfuscatedGlyphPool, HudStyledTextRun, HudSubsequence, HudSubsequenceKey, HudTextStyle,
    HudUvRect, HUD_FONT_BASELINE,
};

use bbb_render_types::{
    visual_order, visual_order_indexed, HudCommandInputRange, HudCommandInputSyntax,
    HudCommandSuggestionEntry, HudEffectRect, HudGlyphArea, HudGlyphQuad,
};

mod debug_draws;
mod debug_layout;
mod inventory_draws;
mod layout_helpers;
mod multiline_selection;
mod sanitize;
mod screen_draws;
mod screen_types;
mod sprite_draws;
mod text_draws;
mod text_layout;
mod toast_draws;
mod types;
use debug_draws::*;
use debug_layout::*;
pub use debug_layout::{
    hud_debug_options_content_x, hud_debug_options_footer_button_xs,
    hud_debug_options_footer_button_y, hud_debug_options_search_box_rects,
};
use inventory_draws::*;
pub use layout_helpers::*;
use multiline_selection::*;
use sanitize::*;
pub use screen_draws::*;
pub use screen_types::*;
use sprite_draws::*;
use text_draws::*;
pub use text_layout::*;
use toast_draws::*;
pub use types::*;

mod draws;
pub(crate) use draws::{append_batched_hud_commands, HudFrameGeometry, RetainedHudCommand};
mod frame;
mod hud_sprites;
mod push;
mod state;
use frame::*;
use push::*;

thread_local! {
    static HUD_FRAME_VARYING_USED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[inline]
pub(super) fn reset_hud_frame_varying_used() {
    HUD_FRAME_VARYING_USED.with(|used| used.set(false));
}

#[inline]
pub(super) fn mark_hud_frame_varying_used() {
    HUD_FRAME_VARYING_USED.with(|used| used.set(true));
}

#[inline]
pub(crate) fn hud_frame_varying_used() -> bool {
    HUD_FRAME_VARYING_USED.with(std::cell::Cell::get)
}

#[cfg(test)]
#[path = "hud/tests/chat_layout.rs"]
mod chat_layout_tests;

#[cfg(test)]
#[path = "hud/tests/chat_page_options.rs"]
mod chat_page_options_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClearColor;
    use bbb_render_types::{HUD_FONT_BOLD_EXTRA_THICKNESS, HUD_FONT_BOLD_OFFSET};
    use glam::{Mat4, Vec3};

    mod action_to_inventory;
    mod codepoints;
    mod fixtures;
    mod hud_draws;
    mod inventory_to_sanitize;
    mod multiline_area;
    mod non_fishy;
    mod sanitize_to_unihex;
    mod screens;
    mod shadow;
    mod status_bars;

    use fixtures::*;
}
