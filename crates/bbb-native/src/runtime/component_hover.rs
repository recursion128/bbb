use bbb_item_model::NativeItemRuntime;
use bbb_protocol::{
    component_hover_uuid, packets::GameType, ComponentHoverEvent, ComponentStyle,
    RichTextComponent, StyledTextRun,
};
use bbb_renderer::{HudComponentTextRun, HudRichTooltip, HudRichTooltipLine, HudStyledTextRun};
use bbb_world::WorldStore;

use crate::component_text::{localized_component_runs, localized_or_fallback};

use super::{hud_item_tooltip, localized_gui_text_with_args};

pub(super) fn hud_component_hover_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    hover: Option<&ComponentHoverEvent>,
) -> Option<HudRichTooltip> {
    match hover? {
        ComponentHoverEvent::ShowText { value, component } => {
            let runs = component
                .as_ref()
                .and_then(|component| {
                    RichTextComponent::try_from_component_nbt(component.clone()).ok()
                })
                .map(|component| {
                    localized_component_runs(&component, &ComponentStyle::default(), item_runtime)
                })
                .unwrap_or_else(|| value.clone());
            Some(HudRichTooltip {
                style: None,
                lines: vec![component_tooltip_line(runs)],
                advanced_lines: None,
                wrap_text: true,
            })
        }
        ComponentHoverEvent::ShowItem {
            id,
            count,
            components,
        } => {
            let fallback = || HudRichTooltip {
                style: None,
                lines: vec![HudRichTooltipLine::plain(if *count == 1 {
                    id.clone()
                } else {
                    format!("{id} x{count}")
                })],
                advanced_lines: None,
                wrap_text: false,
            };
            let Some(stack) = world.component_hover_item_stack(id, *count, components.as_ref())
            else {
                return Some(fallback());
            };
            let creative = world.gameplay().game_type == GameType::Creative.id();
            let Some(normal) = hud_item_tooltip(world, item_runtime, &stack, false, creative)
            else {
                return Some(fallback());
            };
            let lines = inventory_tooltip_lines(normal.lines);
            let advanced_lines = hud_item_tooltip(world, item_runtime, &stack, true, creative)
                .map(|tooltip| inventory_tooltip_lines(tooltip.lines))
                .filter(|advanced| advanced != &lines);
            Some(HudRichTooltip {
                style: normal.style,
                lines,
                advanced_lines,
                wrap_text: false,
            })
        }
        ComponentHoverEvent::ShowEntity {
            id,
            uuid,
            name,
            name_component,
        } => {
            let mut lines = Vec::new();
            if let Some(runs) = name_component
                .as_ref()
                .and_then(|component| {
                    RichTextComponent::try_from_component_nbt(component.clone()).ok()
                })
                .map(|component| {
                    localized_component_runs(&component, &ComponentStyle::default(), item_runtime)
                })
                .or_else(|| name.clone())
                .filter(|runs| !runs.is_empty())
            {
                lines.push(component_tooltip_line(runs));
            }
            let description_key = entity_description_key(id);
            let description = localized_or_fallback(item_runtime, &description_key, id).to_string();
            lines.push(HudRichTooltipLine::plain(localized_gui_text_with_args(
                item_runtime,
                "gui.entity_tooltip.type",
                "Type: %s",
                &[&description],
            )));
            lines.push(HudRichTooltipLine::plain(
                component_hover_uuid(uuid)?.to_string(),
            ));
            Some(HudRichTooltip {
                style: None,
                lines: Vec::new(),
                advanced_lines: Some(lines),
                wrap_text: false,
            })
        }
    }
}

fn component_tooltip_line(runs: Vec<StyledTextRun>) -> HudRichTooltipLine {
    HudRichTooltipLine {
        runs: runs.into_iter().map(hud_component_run).collect(),
        tint: [1.0; 4],
    }
}

fn hud_component_run(run: StyledTextRun) -> HudComponentTextRun {
    HudComponentTextRun {
        font: run.style.font.clone(),
        visual: HudStyledTextRun {
            text: run.text,
            style: bbb_renderer::HudTextStyle {
                bold: run.style.bold == Some(true),
                italic: run.style.italic == Some(true),
                underlined: run.style.underlined == Some(true),
                strikethrough: run.style.strikethrough == Some(true),
                obfuscated: run.style.obfuscated == Some(true),
            },
            color: run.style.color,
        },
    }
}

fn inventory_tooltip_lines(
    lines: Vec<bbb_renderer::HudInventoryTooltipLine>,
) -> Vec<HudRichTooltipLine> {
    lines
        .into_iter()
        .map(|line| HudRichTooltipLine {
            runs: line
                .runs
                .into_iter()
                .map(|visual| HudComponentTextRun { visual, font: None })
                .collect(),
            tint: line.tint,
        })
        .collect()
}

fn entity_description_key(resource_id: &str) -> String {
    let (namespace, path) = resource_id
        .split_once(':')
        .unwrap_or(("minecraft", resource_id));
    format!("entity.{namespace}.{}", path.replace('/', "."))
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        ComponentHoverEvent, ComponentStyle, NbtSummaryEntry, NbtSummaryValue, StyledTextRun,
    };
    use bbb_world::WorldStore;

    use super::hud_component_hover_tooltip;

    #[test]
    fn component_hover_show_text_uses_exact_nested_component_and_font() {
        let hover = ComponentHoverEvent::ShowText {
            value: vec![StyledTextRun {
                text: "fallback.key".to_string(),
                style: ComponentStyle::default(),
            }],
            component: Some(NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "text".to_string(),
                    value: NbtSummaryValue::String("details".to_string()),
                },
                NbtSummaryEntry {
                    name: "font".to_string(),
                    value: NbtSummaryValue::String("example:compact".to_string()),
                },
                NbtSummaryEntry {
                    name: "italic".to_string(),
                    value: NbtSummaryValue::Byte(1),
                },
            ])),
        };
        let tooltip = hud_component_hover_tooltip(&WorldStore::new(), None, Some(&hover)).unwrap();
        assert!(tooltip.wrap_text);
        assert_eq!(tooltip.lines[0].runs[0].visual.text, "details");
        assert_eq!(
            tooltip.lines[0].runs[0].font.as_deref(),
            Some("example:compact")
        );
        assert!(tooltip.lines[0].runs[0].visual.style.italic);
    }

    #[test]
    fn component_hover_show_entity_keeps_vanilla_advanced_line_order() {
        let hover = ComponentHoverEvent::ShowEntity {
            id: "minecraft:pig".to_string(),
            uuid: NbtSummaryValue::IntArray(vec![0, 0, 0, 1]),
            name: Some(vec![StyledTextRun {
                text: "Porky".to_string(),
                style: ComponentStyle::default(),
            }]),
            name_component: None,
        };
        let tooltip = hud_component_hover_tooltip(&WorldStore::new(), None, Some(&hover)).unwrap();
        assert!(tooltip.lines.is_empty());
        let advanced = tooltip.advanced_lines.unwrap();
        assert_eq!(advanced[0].runs[0].visual.text, "Porky");
        assert_eq!(advanced[1].runs[0].visual.text, "Type: minecraft:pig");
        assert_eq!(
            advanced[2].runs[0].visual.text,
            "00000000-0000-0000-0000-000000000001"
        );
    }
}
