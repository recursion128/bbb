//! The typed reads and writes the per-recipe overlays make on a rebuilt stack,
//! through the shared template runtime.

use bbb_protocol::builtin_enum_component_values::{
    builtin_enum_component_value_for_name, normalize_vanilla_dye_color_id,
    vanilla_dye_color_texture_diffuse_color,
};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::nbt_summary::NbtSummaryValue;
use bbb_protocol::packets::data_components::fields::copy_component_value;
use bbb_protocol::packets::{
    BannerPatternLayerSummary, DataComponentPatchSummary, FireworkExplosionSummary,
    ItemStackSummary, WrittenBookContentSummary,
};
use serde_json::Value;

use crate::item_stack_template::{
    self, prototype_component_value, prototype_has_component, PrototypeDefaults,
};

/// `DyeColor.WHITE` (`DyeColor.java:30`), the default every `getOrDefault`
/// against `DataComponents.DYE` falls back to.
pub(super) const DYE_COLOR_WHITE_ID: i32 = 0;

/// `WrittenBookContent.MAX_GENERATION` (`WrittenBookContent.java:41`), which the
/// record constructor (`:68-78`) refuses to exceed.
pub(super) const WRITTEN_BOOK_MAX_GENERATION: i32 = 3;

/// `DyedItemColor.applyDyes` (`DyedItemColor.java:40-79`): the channel average
/// of the current colour and every dye, rescaled so the brightest channel of the
/// result carries the average of the inputs' brightest channels. `dyes` is never
/// empty, because `DyeRecipe.assemble` (`DyeRecipe.java:116`) returns before the
/// call when it is.
pub(super) fn apply_dyes(current: Option<i32>, dyes: &[i32]) -> i32 {
    let mut red_total = 0;
    let mut green_total = 0;
    let mut blue_total = 0;
    let mut intensity_total = 0;
    let mut color_count = 0;
    for color in current.into_iter().chain(
        dyes.iter()
            .map(|dye| vanilla_dye_color_texture_diffuse_color(*dye) as i32),
    ) {
        let (red, green, blue) = (argb_red(color), argb_green(color), argb_blue(color));
        intensity_total += red.max(green).max(blue);
        red_total += red;
        green_total += green;
        blue_total += blue;
        color_count += 1;
    }
    let red = red_total / color_count;
    let green = green_total / color_count;
    let blue = blue_total / color_count;
    let average_intensity = intensity_total as f32 / color_count as f32;
    let result_intensity = red.max(green).max(blue) as f32;
    let rescale = |channel: i32| (channel as f32 * average_intensity / result_intensity) as i32;
    argb_color(0, rescale(red), rescale(green), rescale(blue))
}

/// `ARGB.red` (`ARGB.java:56-58`).
fn argb_red(color: i32) -> i32 {
    color >> 16 & 0xFF
}

/// `ARGB.green` (`ARGB.java:60-62`).
fn argb_green(color: i32) -> i32 {
    color >> 8 & 0xFF
}

/// `ARGB.blue` (`ARGB.java:64-66`).
fn argb_blue(color: i32) -> i32 {
    color & 0xFF
}

/// `ARGB.color` (`ARGB.java:68-70`).
fn argb_color(alpha: i32, red: i32, green: i32, blue: i32) -> i32 {
    (alpha & 0xFF) << 24 | (red & 0xFF) << 16 | (green & 0xFF) << 8 | blue & 0xFF
}

/// One typed value a per-recipe overlay writes, as `ItemStack.set`
/// (`ItemStack.java:761-763`) hands it to `PatchedDataComponentMap.set`
/// (`PatchedDataComponentMap.java:69-80`).
pub(super) enum ComponentValue {
    DyedColor(i32),
    MapPostProcessing(bbb_protocol::packets::MapPostProcessingSummary),
    FireworkExplosion(FireworkExplosionSummary),
    BannerPatterns(Vec<BannerPatternLayerSummary>),
    BaseColor(i32),
    WrittenBookContent {
        content: WrittenBookContentSummary,
        pages_nbt: Vec<Option<NbtSummaryValue>>,
        page_filters_nbt: Vec<Option<NbtSummaryValue>>,
    },
}

impl ComponentValue {
    fn type_id(&self) -> i32 {
        match self {
            Self::MapPostProcessing(_) => bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID,
            Self::DyedColor(_) => VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            Self::FireworkExplosion(_) => VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            Self::BannerPatterns(_) => VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            Self::BaseColor(_) => VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
            Self::WrittenBookContent { .. } => VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
        }
    }
}

/// `ItemStack.set` (`ItemStack.java:761-763`) with one typed overlay value.
pub(super) fn set_component(
    patch: &mut DataComponentPatchSummary,
    defaults: &PrototypeDefaults,
    value: ComponentValue,
) {
    let type_id = value.type_id();
    let mut written = DataComponentPatchSummary::default();
    write_component_value(&mut written, value);
    item_stack_template::set_component(patch, defaults, type_id, &written)
        .expect("special recipe component is modelled");
}

fn write_component_value(patch: &mut DataComponentPatchSummary, value: ComponentValue) {
    match value {
        ComponentValue::MapPostProcessing(operation) => patch.map_post_processing = Some(operation),
        ComponentValue::DyedColor(rgb) => patch.dyed_color = Some(rgb),
        ComponentValue::FireworkExplosion(explosion) => {
            patch.firework_explosion = Some(explosion);
        }
        ComponentValue::BannerPatterns(layers) => patch.banner_pattern_layers = layers,
        ComponentValue::BaseColor(color) => patch.base_color_id = Some(color),
        ComponentValue::WrittenBookContent {
            content,
            pages_nbt,
            page_filters_nbt,
        } => {
            patch.written_book = Some(content);
            patch.written_book_pages_nbt = pages_nbt;
            patch.written_book_page_filters_nbt = page_filters_nbt;
        }
    }
}

/// `ItemStack.set` with a null value.
pub(super) fn set_component_to_null(
    patch: &mut DataComponentPatchSummary,
    defaults: &PrototypeDefaults,
    type_id: i32,
) {
    item_stack_template::remove_component(patch, defaults, type_id)
        .expect("special recipe component is modelled");
}

fn patch_declares(stack: &ItemStackSummary, type_id: i32) -> Option<bool> {
    item_stack_template::patch_declares(&stack.component_patch, type_id)
}

/// `itemStack.getOrDefault(DataComponents.DYE, DyeColor.WHITE)` as
/// `DyeRecipe.assemble` (`DyeRecipe.java:110`) and
/// `FireworkStarFadeRecipe.assemble` (`FireworkStarFadeRecipe.java:82`) read it.
pub(super) fn dye_color_or_white(stack: &ItemStackSummary) -> i32 {
    match patch_declares(stack, VANILLA_DATA_COMPONENT_DYE_TYPE_ID) {
        Some(false) => DYE_COLOR_WHITE_ID,
        // `DyeColor.byId` (`DyeColor.java:96-98`) over the `ZERO` out-of-bounds
        // strategy (`:48`) is what any id outside the sixteen resolves through.
        Some(true) => stack
            .component_patch
            .dye_color_id
            .map_or(DYE_COLOR_WHITE_ID, normalize_vanilla_dye_color_id),
        None => stack
            .item_id
            .and_then(|item_id| {
                prototype_component_value(item_id, VANILLA_DATA_COMPONENT_DYE_TYPE_ID)
            })
            .and_then(Value::as_str)
            .and_then(|name| {
                builtin_enum_component_value_for_name(VANILLA_DATA_COMPONENT_DYE_TYPE_ID, name)
            })
            .map_or(DYE_COLOR_WHITE_ID, |entry| entry.id),
    }
}

/// `targetStack.get(DataComponents.DYED_COLOR)` (`DyeRecipe.java:117`). No 26.1
/// item registers a prototype default for it, so an absent patch entry is null.
pub(super) fn dyed_color(stack: &ItemStackSummary) -> Option<i32> {
    match patch_declares(stack, VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID) {
        Some(false) | None => None,
        Some(true) => stack.component_patch.dyed_color,
    }
}

/// `getOrDefault(DataComponents.FIREWORK_EXPLOSION, FireworkExplosion.DEFAULT)`
/// as `ItemStack.update` (`ItemStack.java:773-775`) reads it for
/// `FireworkStarFadeRecipe.assemble` (`FireworkStarFadeRecipe.java:91`).
/// `FireworkExplosion.DEFAULT` is `FireworkExplosion.java:26`.
pub(super) fn firework_explosion_or_default(stack: &ItemStackSummary) -> FireworkExplosionSummary {
    let patch = &stack.component_patch;
    match patch_declares(stack, VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID) {
        Some(true) => patch.firework_explosion.clone().unwrap(),
        Some(false) | None => FireworkExplosionSummary::default(),
    }
}

/// `itemStack.get(DataComponents.BANNER_PATTERNS)`
/// (`ShieldDecorationRecipe.java:92`), whose prototype default on every banner
/// and on the shield is `BannerPatternLayers.EMPTY` (`Items.java:1727-1731`).
pub(super) fn banner_patterns(stack: &ItemStackSummary) -> Option<Vec<BannerPatternLayerSummary>> {
    match patch_declares(stack, VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID) {
        Some(false) => None,
        Some(true) => Some(stack.component_patch.banner_pattern_layers.clone()),
        None => stack
            .item_id
            .filter(|item_id| {
                prototype_has_component(*item_id, VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID)
            })
            .map(|_| Vec::new()),
    }
}

/// `source.get(DataComponents.WRITTEN_BOOK_CONTENT)` (`BookCloningRecipe.java:116`).
/// No 26.1 item registers a prototype default for it.
pub(super) fn written_book_content(stack: &ItemStackSummary) -> Option<&WrittenBookContentSummary> {
    match patch_declares(stack, VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID) {
        Some(false) | None => None,
        Some(true) => stack.component_patch.written_book.as_ref(),
    }
}

/// A patch carrying one set entry, for `DataComponentPatch.builder().set(...)`.
pub(super) fn single_component_patch(type_id: i32) -> DataComponentPatchSummary {
    DataComponentPatchSummary {
        added_type_ids: vec![type_id],
        ..Default::default()
    }
}

/// `ItemStackTemplate.apply(components)` (`ItemStackTemplate.java:88-90`).
pub(super) fn apply_template(
    template: &crate::recipe::ItemStackTemplate,
    additional: &DataComponentPatchSummary,
) -> ItemStackSummary {
    template
        .apply(template.count, additional)
        .expect("special recipe component is modelled")
}

/// `result.set(type, source.get(type))`.
pub(super) fn copy_component(
    patch: &mut DataComponentPatchSummary,
    defaults: &PrototypeDefaults,
    source: &ItemStackSummary,
    type_id: i32,
) {
    match patch_declares(source, type_id) {
        Some(true) => {
            let mut written = DataComponentPatchSummary::default();
            copy_component_value(type_id, &source.component_patch, &mut written)
                .expect("special recipe component is modelled");
            item_stack_template::set_component(patch, defaults, type_id, &written)
                .expect("special recipe component is modelled");
        }
        Some(false) => set_component_to_null(patch, defaults, type_id),
        // Every 26.1 prototype default a recipe source copies equals the
        // result's own default.
        None => {
            if !source
                .item_id
                .is_some_and(|item_id| prototype_has_component(item_id, type_id))
            {
                set_component_to_null(patch, defaults, type_id);
            }
        }
    }
}
