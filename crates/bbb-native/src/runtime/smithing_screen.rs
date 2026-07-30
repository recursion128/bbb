use bbb_item_model::NativeItemRuntime;
use bbb_protocol::{
    item_types::{vanilla_item_type, vanilla_item_type_for_resource_id},
    menu_types::VANILLA_MENU_TYPE_SMITHING_ID,
};
use bbb_renderer::{
    HudFontGlyphBook, HudInventoryBackgroundLayer, HudInventoryBackgroundTexture,
    HudInventoryTooltip, HudInventoryTooltipLine, HudSmithingSlotIcon,
};
use bbb_world::WorldStore;

use super::{
    hud_inventory_background_layer, item_stack_is_empty, localized_gui_text,
    open_container_slot_has_item, open_container_slot_item, smithing_should_show_error,
};

const ICON_CHANGE_TICK_RATE: u64 = 30;
const ICON_TRANSITION_TICKS: f32 = 4.0;
const ICON_SIZE: u32 = 16;
const TEMPLATE_SLOT: i16 = 0;
const BASE_SLOT: i16 = 1;
const ADDITION_SLOT: i16 = 2;
const ERROR_X: i32 = 65;
const ERROR_Y: i32 = 46;
const ERROR_WIDTH: i32 = 28;
const ERROR_HEIGHT: i32 = 21;
const TOOLTIP_WIDTH: u32 = 115;

const TEMPLATE_ICONS: &[HudSmithingSlotIcon] = &[
    HudSmithingSlotIcon::TemplateArmorTrim,
    HudSmithingSlotIcon::TemplateNetheriteUpgrade,
];
const ARMOR_TRIM_BASE_ICONS: &[HudSmithingSlotIcon] = &[
    HudSmithingSlotIcon::Helmet,
    HudSmithingSlotIcon::Chestplate,
    HudSmithingSlotIcon::Leggings,
    HudSmithingSlotIcon::Boots,
];
const ARMOR_TRIM_ADDITION_ICONS: &[HudSmithingSlotIcon] = &[
    HudSmithingSlotIcon::Ingot,
    HudSmithingSlotIcon::RedstoneDust,
    HudSmithingSlotIcon::LapisLazuli,
    HudSmithingSlotIcon::Quartz,
    HudSmithingSlotIcon::Diamond,
    HudSmithingSlotIcon::Emerald,
    HudSmithingSlotIcon::AmethystShard,
];
const NETHERITE_UPGRADE_BASE_ICONS: &[HudSmithingSlotIcon] = &[
    HudSmithingSlotIcon::Helmet,
    HudSmithingSlotIcon::Sword,
    HudSmithingSlotIcon::Chestplate,
    HudSmithingSlotIcon::Pickaxe,
    HudSmithingSlotIcon::Leggings,
    HudSmithingSlotIcon::Axe,
    HudSmithingSlotIcon::Boots,
    HudSmithingSlotIcon::Hoe,
    HudSmithingSlotIcon::Shovel,
    HudSmithingSlotIcon::NautilusArmor,
    HudSmithingSlotIcon::Spear,
];
const NETHERITE_UPGRADE_ADDITION_ICONS: &[HudSmithingSlotIcon] = &[HudSmithingSlotIcon::Ingot];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum SmithingTemplateKind {
    #[default]
    None,
    ArmorTrim,
    NetheriteUpgrade,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum SmithingIconList {
    #[default]
    None,
    Template,
    ArmorTrimBase,
    ArmorTrimAddition,
    NetheriteUpgradeBase,
    NetheriteUpgradeAddition,
}

impl SmithingIconList {
    fn icons(self) -> &'static [HudSmithingSlotIcon] {
        match self {
            Self::None => &[],
            Self::Template => TEMPLATE_ICONS,
            Self::ArmorTrimBase => ARMOR_TRIM_BASE_ICONS,
            Self::ArmorTrimAddition => ARMOR_TRIM_ADDITION_ICONS,
            Self::NetheriteUpgradeBase => NETHERITE_UPGRADE_BASE_ICONS,
            Self::NetheriteUpgradeAddition => NETHERITE_UPGRADE_ADDITION_ICONS,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CyclingSlotState {
    list: SmithingIconList,
    tick: u64,
    icon_index: usize,
}

impl CyclingSlotState {
    fn set_list(&mut self, list: SmithingIconList) {
        if self.list != list {
            self.list = list;
            self.icon_index = 0;
        }
    }

    fn advance(&mut self, ticks: u64) {
        let icons = self.list.icons();
        if icons.is_empty() || ticks == 0 {
            return;
        }
        let old_period = self.tick / ICON_CHANGE_TICK_RATE;
        self.tick = self.tick.saturating_add(ticks);
        let advances = self.tick / ICON_CHANGE_TICK_RATE - old_period;
        let advances =
            usize::try_from(advances % icons.len() as u64).expect("icon-list remainder fits usize");
        self.icon_index = (self.icon_index + advances) % icons.len();
    }

    fn layers(
        self,
        occupied: bool,
        x: i32,
        y: i32,
        partial_tick: f32,
    ) -> Vec<HudInventoryBackgroundLayer> {
        let icons = self.list.icons();
        if occupied || icons.is_empty() {
            return Vec::new();
        }
        let transitioning = icons.len() > 1 && self.tick >= ICON_CHANGE_TICK_RATE;
        let alpha = if transitioning {
            ((self.tick % ICON_CHANGE_TICK_RATE) as f32 + partial_tick.clamp(0.0, 1.0))
                .min(ICON_TRANSITION_TICKS)
                / ICON_TRANSITION_TICKS
        } else {
            1.0
        };
        let mut layers = Vec::with_capacity(if alpha < 1.0 { 2 } else { 1 });
        if alpha < 1.0 {
            let previous = (self.icon_index + icons.len() - 1) % icons.len();
            layers.push(slot_icon_layer(icons[previous], 1.0 - alpha, x, y));
        }
        layers.push(slot_icon_layer(icons[self.icon_index], alpha, x, y));
        layers
    }
}

fn slot_icon_layer(
    icon: HudSmithingSlotIcon,
    alpha: f32,
    x: i32,
    y: i32,
) -> HudInventoryBackgroundLayer {
    hud_inventory_background_layer(
        HudInventoryBackgroundTexture::smithing_slot_icon(icon, alpha),
        x,
        y,
        ICON_SIZE,
        ICON_SIZE,
        [0.0, 0.0],
        [1.0, 1.0],
    )
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct SmithingScreenFrame {
    pub(super) background_layers: Vec<HudInventoryBackgroundLayer>,
}

#[derive(Debug, Default)]
pub(super) struct SmithingScreenRuntime {
    screen_key: Option<(u64, u64)>,
    last_client_tick: u64,
    slots: [CyclingSlotState; 3],
}

impl SmithingScreenRuntime {
    pub(super) fn frame(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        client_tick: u64,
        partial_tick: f32,
    ) -> SmithingScreenFrame {
        let Some(container) = world
            .inventory()
            .open_container
            .as_ref()
            .filter(|container| container.menu_type_id == Some(VANILLA_MENU_TYPE_SMITHING_ID))
        else {
            *self = Self::default();
            return SmithingScreenFrame::default();
        };
        let key = (
            world.client_level_generation(),
            world.dialog_screen_revision(),
        );
        let new_screen = self.screen_key != Some(key);
        if new_screen {
            self.screen_key = Some(key);
            self.last_client_tick = client_tick;
            self.slots = [CyclingSlotState::default(); 3];
        }
        let elapsed = if new_screen {
            1
        } else {
            client_tick.saturating_sub(self.last_client_tick)
        };
        self.last_client_tick = self.last_client_tick.max(client_tick);
        if elapsed > 0 {
            let template = smithing_template_kind(world, item_runtime);
            self.slots[TEMPLATE_SLOT as usize].set_list(SmithingIconList::Template);
            self.slots[BASE_SLOT as usize].set_list(match template {
                SmithingTemplateKind::None => SmithingIconList::None,
                SmithingTemplateKind::ArmorTrim => SmithingIconList::ArmorTrimBase,
                SmithingTemplateKind::NetheriteUpgrade => SmithingIconList::NetheriteUpgradeBase,
            });
            self.slots[ADDITION_SLOT as usize].set_list(match template {
                SmithingTemplateKind::None => SmithingIconList::None,
                SmithingTemplateKind::ArmorTrim => SmithingIconList::ArmorTrimAddition,
                SmithingTemplateKind::NetheriteUpgrade => {
                    SmithingIconList::NetheriteUpgradeAddition
                }
            });
            for slot in &mut self.slots {
                slot.advance(elapsed);
            }
        }

        let mut background_layers = Vec::new();
        for (slot, x) in [(TEMPLATE_SLOT, 8), (BASE_SLOT, 26), (ADDITION_SLOT, 44)] {
            background_layers.extend(
                self.slots[slot as usize].layers(
                    container
                        .slots
                        .iter()
                        .find(|candidate| candidate.slot == slot)
                        .is_some_and(|candidate| !item_stack_is_empty(&candidate.item)),
                    x,
                    48,
                    partial_tick,
                ),
            );
        }
        SmithingScreenFrame { background_layers }
    }
}

pub(super) fn tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_slot: Option<i16>,
    cursor: Option<(i32, i32)>,
    fonts: &HudFontGlyphBook,
) -> Option<HudInventoryTooltip> {
    let (x, y) = cursor?;
    let mut tooltip = (smithing_should_show_error(world)
        && x >= ERROR_X
        && x < ERROR_X + ERROR_WIDTH
        && y >= ERROR_Y
        && y < ERROR_Y + ERROR_HEIGHT)
        .then(|| {
            (
                u16::MAX,
                localized_gui_text(
                    item_runtime,
                    "container.upgrade.error_tooltip",
                    "Item can't be upgraded this way",
                ),
            )
        });

    if let Some(slot) = hovered_slot {
        if !open_container_slot_has_item(world, slot) {
            if !open_container_slot_has_item(world, TEMPLATE_SLOT) && slot == TEMPLATE_SLOT {
                tooltip = Some((
                    u16::try_from(slot).unwrap_or(u16::MAX),
                    localized_gui_text(
                        item_runtime,
                        "container.upgrade.missing_template_tooltip",
                        "Add Smithing Template",
                    ),
                ));
            } else {
                let key = match (smithing_template_kind(world, item_runtime), slot) {
                    (SmithingTemplateKind::ArmorTrim, BASE_SLOT) => Some(
                        "item.minecraft.smithing_template.armor_trim.base_slot_description",
                    ),
                    (SmithingTemplateKind::ArmorTrim, ADDITION_SLOT) => Some(
                        "item.minecraft.smithing_template.armor_trim.additions_slot_description",
                    ),
                    (SmithingTemplateKind::NetheriteUpgrade, BASE_SLOT) => Some(
                        "item.minecraft.smithing_template.netherite_upgrade.base_slot_description",
                    ),
                    (SmithingTemplateKind::NetheriteUpgrade, ADDITION_SLOT) => Some(
                        "item.minecraft.smithing_template.netherite_upgrade.additions_slot_description",
                    ),
                    _ => None,
                };
                if let Some(key) = key {
                    tooltip = Some((
                        u16::try_from(slot).unwrap_or(u16::MAX),
                        localized_gui_text(item_runtime, key, key),
                    ));
                }
            }
        }
    }

    let (slot_id, text) = tooltip?;
    let lines = wrap_plain_text(&text, TOOLTIP_WIDTH, fonts)
        .into_iter()
        .map(|text| HudInventoryTooltipLine {
            text,
            tint: [1.0; 4],
            runs: Vec::new(),
        })
        .collect();
    Some(HudInventoryTooltip {
        slot_id,
        x,
        y,
        style: None,
        lines,
    })
}

fn wrap_plain_text(text: &str, max_width: u32, fonts: &HudFontGlyphBook) -> Vec<String> {
    let glyphs = fonts.glyphs(None);
    let mut lines = Vec::new();
    for source in text.split('\n') {
        let mut line = String::new();
        for word in source.split_inclusive(char::is_whitespace) {
            let mut candidate = line.clone();
            candidate.push_str(word);
            if !line.trim().is_empty() && glyphs.plain_text_width(&candidate) > max_width {
                lines.push(line.trim_end().to_string());
                line.clear();
            }
            if glyphs.plain_text_width(word) <= max_width {
                line.push_str(word.trim_start_matches(char::is_whitespace));
                continue;
            }
            for ch in word.chars() {
                let mut candidate = line.clone();
                candidate.push(ch);
                if !line.is_empty() && glyphs.plain_text_width(&candidate) > max_width {
                    lines.push(std::mem::take(&mut line));
                }
                if !ch.is_whitespace() || !line.is_empty() {
                    line.push(ch);
                }
            }
        }
        lines.push(line.trim_end().to_string());
    }
    lines
}

fn smithing_template_kind(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> SmithingTemplateKind {
    let Some(stack) = open_container_slot_item(world, TEMPLATE_SLOT) else {
        return SmithingTemplateKind::None;
    };
    if item_stack_is_empty(stack) {
        return SmithingTemplateKind::None;
    }
    let Some(item_id) = stack.item_id else {
        return SmithingTemplateKind::None;
    };
    let Some(resource_id) = item_runtime
        .and_then(|runtime| runtime.item_resource_id(item_id))
        .or_else(|| vanilla_item_type(item_id).map(|item| item.resource_id))
    else {
        return SmithingTemplateKind::None;
    };
    if resource_id == "minecraft:netherite_upgrade_smithing_template" {
        SmithingTemplateKind::NetheriteUpgrade
    } else if is_armor_trim_template(resource_id) {
        SmithingTemplateKind::ArmorTrim
    } else {
        SmithingTemplateKind::None
    }
}

fn is_armor_trim_template(resource_id: &str) -> bool {
    const ARMOR_TRIM_TEMPLATES: [&str; 18] = [
        "minecraft:bolt_armor_trim_smithing_template",
        "minecraft:coast_armor_trim_smithing_template",
        "minecraft:dune_armor_trim_smithing_template",
        "minecraft:eye_armor_trim_smithing_template",
        "minecraft:flow_armor_trim_smithing_template",
        "minecraft:host_armor_trim_smithing_template",
        "minecraft:raiser_armor_trim_smithing_template",
        "minecraft:rib_armor_trim_smithing_template",
        "minecraft:sentry_armor_trim_smithing_template",
        "minecraft:shaper_armor_trim_smithing_template",
        "minecraft:silence_armor_trim_smithing_template",
        "minecraft:snout_armor_trim_smithing_template",
        "minecraft:spire_armor_trim_smithing_template",
        "minecraft:tide_armor_trim_smithing_template",
        "minecraft:vex_armor_trim_smithing_template",
        "minecraft:ward_armor_trim_smithing_template",
        "minecraft:wayfinder_armor_trim_smithing_template",
        "minecraft:wild_armor_trim_smithing_template",
    ];
    vanilla_item_type_for_resource_id(resource_id).is_some()
        && ARMOR_TRIM_TEMPLATES.contains(&resource_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        item_types::vanilla_item_type_for_resource_id,
        packets::{ContainerSetContent, ItemStackSummary, OpenScreen},
        RichTextComponent,
    };
    use bbb_renderer::{HudAsciiGlyph, HudFontGlyphMap};
    use std::collections::BTreeMap;

    fn stack(resource_id: &str) -> ItemStackSummary {
        let item = vanilla_item_type_for_resource_id(resource_id).expect("vanilla item");
        ItemStackSummary {
            item_id: Some(item.id),
            count: 1,
            ..ItemStackSummary::empty()
        }
    }

    fn smithing_world(template: Option<&str>) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
            title: RichTextComponent::plain("Smithing"),
        });
        let mut items = vec![ItemStackSummary::empty(); 40];
        if let Some(template) = template {
            items[TEMPLATE_SLOT as usize] = stack(template);
        }
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 1,
            items,
            carried_item: ItemStackSummary::empty(),
        });
        world
    }

    fn fonts() -> HudFontGlyphBook {
        let mut glyphs = HudFontGlyphMap::new();
        for ch in ' '..='~' {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    width: 5,
                    height: 8,
                    advance: 6,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([("minecraft:default".to_string(), glyphs)]),
        )
    }

    #[test]
    fn smithing_slot_catalog_matches_vanilla_order_and_paths() {
        assert_eq!(HudSmithingSlotIcon::ALL.len(), 20);
        assert_eq!(TEMPLATE_ICONS[0], HudSmithingSlotIcon::TemplateArmorTrim);
        assert_eq!(
            NETHERITE_UPGRADE_BASE_ICONS,
            &[
                HudSmithingSlotIcon::Helmet,
                HudSmithingSlotIcon::Sword,
                HudSmithingSlotIcon::Chestplate,
                HudSmithingSlotIcon::Pickaxe,
                HudSmithingSlotIcon::Leggings,
                HudSmithingSlotIcon::Axe,
                HudSmithingSlotIcon::Boots,
                HudSmithingSlotIcon::Hoe,
                HudSmithingSlotIcon::Shovel,
                HudSmithingSlotIcon::NautilusArmor,
                HudSmithingSlotIcon::Spear,
            ]
        );
        assert_eq!(
            HudSmithingSlotIcon::NautilusArmor.sprite_path(),
            "container/slot/nautilus_armor"
        );
    }

    #[test]
    fn smithing_icons_cycle_every_thirty_ticks_and_cross_fade_for_four() {
        let world = smithing_world(None);
        let mut runtime = SmithingScreenRuntime::default();
        let first = runtime.frame(&world, None, 0, 0.0);
        assert_eq!(first.background_layers.len(), 1);
        assert_eq!(
            first.background_layers[0].texture,
            HudInventoryBackgroundTexture::smithing_slot_icon(
                HudSmithingSlotIcon::TemplateArmorTrim,
                1.0,
            )
        );

        let transition = runtime.frame(&world, None, 29, 2.0);
        assert_eq!(transition.background_layers.len(), 2);
        assert_eq!(
            transition.background_layers[0].texture,
            HudInventoryBackgroundTexture::smithing_slot_icon(
                HudSmithingSlotIcon::TemplateArmorTrim,
                0.75,
            )
        );
        assert_eq!(
            transition.background_layers[1].texture,
            HudInventoryBackgroundTexture::smithing_slot_icon(
                HudSmithingSlotIcon::TemplateNetheriteUpgrade,
                0.25,
            )
        );
    }

    #[test]
    fn smithing_template_selects_exact_base_and_addition_icons() {
        let world = smithing_world(Some("minecraft:sentry_armor_trim_smithing_template"));
        let mut runtime = SmithingScreenRuntime::default();
        let frame = runtime.frame(&world, None, 5, 0.0);
        assert_eq!(frame.background_layers.len(), 2);
        assert_eq!(
            frame.background_layers[0].texture,
            HudInventoryBackgroundTexture::smithing_slot_icon(HudSmithingSlotIcon::Helmet, 1.0,)
        );
        assert_eq!(
            frame.background_layers[1].texture,
            HudInventoryBackgroundTexture::smithing_slot_icon(HudSmithingSlotIcon::Ingot, 1.0,)
        );
    }

    #[test]
    fn smithing_onboarding_tooltips_follow_slot_override_and_wrap_bound() {
        let world = smithing_world(None);
        let missing = tooltip(&world, None, Some(TEMPLATE_SLOT), Some((66, 47)), &fonts())
            .expect("missing template tooltip");
        assert_eq!(missing.slot_id, 0);
        assert_eq!(
            missing
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Add Smithing", "Template"]
        );
        assert!(missing
            .lines
            .iter()
            .all(|line| fonts().glyphs(None).plain_text_width(&line.text) <= TOOLTIP_WIDTH));
    }

    #[test]
    fn arbitrary_template_suffix_does_not_borrow_vanilla_template_behavior() {
        assert!(!is_armor_trim_template(
            "example:custom_armor_trim_smithing_template"
        ));
        assert!(is_armor_trim_template(
            "minecraft:bolt_armor_trim_smithing_template"
        ));
    }
}
