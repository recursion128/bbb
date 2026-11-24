use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::{
    hud_game_rules_confirmation_buttons, hud_game_rules_layout, HudDebugOptionsTooltip,
    HudGameRulesDiscardConfirmation, HudGameRulesRow, HudGameRulesScreen,
};
use bbb_client_world::GameRuleEditValueState;
use winit::dpi::PhysicalSize;

use crate::input::{
    text_field::TextField, GameRuleSearchEntry, GameRulesInteractionRow, GameRulesScreen,
    GameRulesScreenFocus,
};

use super::localized_gui_text;

pub(super) fn sync_game_rules_screen_projection(
    state: &mut GameRulesScreen,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: PhysicalSize<u32>,
) {
    let entries = state
        .rules
        .rules
        .iter()
        .map(|rule| game_rule_search_entry(rule, item_runtime))
        .collect();
    state.sync_projection(entries, surface_size);
}

pub(super) fn hud_game_rules_screen(
    state: &GameRulesScreen,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: PhysicalSize<u32>,
) -> HudGameRulesScreen {
    let entries = state
        .rules
        .rules
        .iter()
        .map(|rule| game_rule_search_entry(rule, item_runtime))
        .collect::<Vec<_>>();
    let view = state.project_view(&entries, surface_size);
    let screen = &state.rules;
    let layout = hud_game_rules_layout(surface_size);
    let hovered_rule = view.hovered_rule();
    let rows = view
        .rows
        .iter()
        .filter_map(|row| match row {
            GameRulesInteractionRow::Category { category, label } => {
                Some(HudGameRulesRow::Category {
                    category: category.clone(),
                    label: label.clone(),
                })
            }
            GameRulesInteractionRow::Rule { rule } => {
                let edit = screen.rules.iter().find(|edit| edit.rule == *rule)?;
                let label = localized_game_rule_label(item_runtime, rule);
                let focused = matches!(
                    view.focus,
                    GameRulesScreenFocus::Rule(focused) if focused == rule
                );
                let hovered = hovered_rule == Some(rule.as_str());
                match &edit.current_value {
                    GameRuleEditValueState::Boolean { value } => Some(HudGameRulesRow::Boolean {
                        rule: rule.clone(),
                        label,
                        value: *value,
                        value_label: localized_gui_text(
                            item_runtime,
                            if *value { "options.on" } else { "options.off" },
                            if *value { "ON" } else { "OFF" },
                        ),
                        hovered,
                        focused,
                    }),
                    GameRuleEditValueState::Integer {
                        input: canonical_input,
                        parsed_value,
                    } => {
                        let input = state.integer_input(rule).map_or_else(
                            || TextField::with_text(canonical_input, 32).hud(focused, hovered),
                            |field| field.hud(focused, hovered),
                        );
                        Some(HudGameRulesRow::Integer {
                            rule: rule.clone(),
                            label,
                            input,
                            valid: parsed_value.is_some(),
                        })
                    }
                }
            }
        })
        .collect::<Vec<_>>();
    let tooltip = hovered_rule.and_then(|rule| {
        let key = format!("{}.description", game_rule_translation_key(rule));
        localized_optional(item_runtime, &key).map(|text| HudDebugOptionsTooltip {
            text,
            x: state.cursor_position().map_or(0, |(x, _)| x + 12),
            y: state.cursor_position().map_or(0, |(_, y)| y + 12),
        })
    });
    let cursor = state.cursor_position();
    let discard_confirmation = screen.discard_confirmation_open.then(|| {
        let (discard, keep_editing) = hud_game_rules_confirmation_buttons(surface_size);
        HudGameRulesDiscardConfirmation {
            title: localized_gui_text(
                item_runtime,
                "editGamerule.inGame.discardChanges.title",
                "Discard changes?",
            ),
            message: localized_gui_text(
                item_runtime,
                "editGamerule.inGame.discardChanges.message",
                "Your changes will be lost.",
            ),
            discard: localized_gui_text(item_runtime, "gui.yes", "Discard"),
            keep_editing: localized_gui_text(item_runtime, "gui.no", "Keep Editing"),
            discard_hovered: discard.contains(cursor),
            keep_editing_hovered: keep_editing.contains(cursor),
            discard_focused: matches!(view.focus, GameRulesScreenFocus::Discard),
            keep_editing_focused: matches!(view.focus, GameRulesScreenFocus::KeepEditing),
        }
    });
    HudGameRulesScreen {
        title: localized_gui_text(item_runtime, "editGamerule.title", "Game Rules"),
        search_hint: localized_gui_text(item_runtime, "gui.game_rule.search", "Search"),
        search: state
            .search()
            .hud(matches!(view.focus, GameRulesScreenFocus::Search), false),
        loading_text: localized_gui_text(
            item_runtime,
            "editGamerule.inGame.downloadingGamerules",
            "Downloading game rules...",
        ),
        is_loading: screen.loading,
        rows,
        scroll_amount: view.scroll_amount,
        max_scroll_amount: view.max_scroll_amount,
        scroller_height: view.scroller_height,
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        done_enabled: screen.can_submit(),
        done_hovered: layout.done.contains(cursor),
        cancel_hovered: layout.cancel.contains(cursor),
        done_focused: matches!(view.focus, GameRulesScreenFocus::Done),
        cancel_focused: matches!(view.focus, GameRulesScreenFocus::Cancel),
        tooltip,
        discard_confirmation,
    }
}

fn game_rule_search_entry(
    rule: &bbb_client_world::GameRuleEditState,
    item_runtime: Option<&NativeItemRuntime>,
) -> GameRuleSearchEntry {
    let short_id = rule
        .rule
        .split_once(':')
        .map_or(rule.rule.as_str(), |(_, path)| path);
    let label = localized_game_rule_label(item_runtime, &rule.rule);
    let category_key = format!("gamerule.category.minecraft.{}", rule.category);
    let category_label = localized_gui_text(item_runtime, &category_key, &rule.category);
    let description_key = format!("{}.description", game_rule_translation_key(&rule.rule));
    let description = localized_optional(item_runtime, &description_key);
    let searchable_text = [
        short_id,
        label.as_str(),
        category_label.as_str(),
        description.as_deref().unwrap_or_default(),
    ]
    .join(" ")
    .to_lowercase();
    GameRuleSearchEntry {
        rule: rule.rule.clone(),
        category: rule.category.clone(),
        label,
        category_label,
        description,
        searchable_text,
    }
}

fn localized_game_rule_label(item_runtime: Option<&NativeItemRuntime>, rule: &str) -> String {
    let short_id = rule.split_once(':').map_or(rule, |(_, path)| path);
    localized_gui_text(item_runtime, &game_rule_translation_key(rule), short_id)
}

fn game_rule_translation_key(rule: &str) -> String {
    let (namespace, path) = rule.split_once(':').unwrap_or(("minecraft", rule));
    format!("gamerule.{namespace}.{path}")
}

fn localized_optional(
    item_runtime: Option<&NativeItemRuntime>,
    translation_key: &str,
) -> Option<String> {
    item_runtime
        .and_then(|runtime| runtime.localized_text_if_present(translation_key))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{
        CommonPlayerSpawnInfo, EntityEvent, GameRuleValue as ProtocolGameRuleValue, GameRuleValues,
        PlayLogin,
    };

    use super::*;
    use crate::control::NetCounters;
    use crate::input::ClientInputState;
    use bbb_client_world::WorldStore;

    fn operator_world() -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 42,
            event_id: 26,
        }));
        world.take_screen_requests();
        world
    }

    #[test]
    fn projection_waits_for_values_then_emits_typed_rows() {
        let mut world = operator_world();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        assert!(input.open_game_rules_screen(&mut world, &mut counters, &None));

        let loading = hud_game_rules_screen(
            input.game_rules_screen_mut().unwrap(),
            None,
            PhysicalSize::new(854, 480),
        );
        assert!(loading.is_loading);
        assert!(loading.rows.is_empty());

        world.apply_game_rule_values(GameRuleValues {
            values: vec![
                ProtocolGameRuleValue {
                    rule: "minecraft:keep_inventory".to_string(),
                    value: "false".to_string(),
                },
                ProtocolGameRuleValue {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                },
            ],
        });
        input.apply_screen_requests(&mut world, &mut counters, &None);
        let loaded = hud_game_rules_screen(
            input.game_rules_screen_mut().unwrap(),
            None,
            PhysicalSize::new(854, 480),
        );

        assert!(!loaded.is_loading);
        assert_eq!(loaded.rows.len(), 4);
        assert!(matches!(loaded.rows[1], HudGameRulesRow::Boolean { .. }));
        assert!(matches!(loaded.rows[3], HudGameRulesRow::Integer { .. }));
        assert!(loaded.done_enabled);
    }

    #[test]
    fn resource_ids_map_to_vanilla_translation_keys() {
        assert_eq!(
            game_rule_translation_key("minecraft:keep_inventory"),
            "gamerule.minecraft.keep_inventory"
        );
        assert_eq!(
            game_rule_translation_key("custom:rule"),
            "gamerule.custom.rule"
        );
    }
}
