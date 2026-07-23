use std::collections::BTreeMap;

use bbb_protocol::{
    game_rules::{vanilla_game_rule, GameRuleValue, VANILLA_GAME_RULES},
    packets::{GameRuleValues as ProtocolGameRuleValues, SetGameRule, SetGameRuleEntry},
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRuleValuesState {
    #[serde(default)]
    pub values: Vec<GameRuleValueState>,
}

impl GameRuleValuesState {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRuleValueState {
    pub rule: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRulesScreenState {
    pub generation: u64,
    #[serde(default)]
    pub loading: bool,
    #[serde(default)]
    pub discard_confirmation_open: bool,
    #[serde(default)]
    pub rules: Vec<GameRuleEditState>,
}

impl GameRulesScreenState {
    pub fn has_pending_changes(&self) -> bool {
        self.rules.iter().any(GameRuleEditState::has_pending_change)
    }

    pub fn can_submit(&self) -> bool {
        !self.loading
            && !self.discard_confirmation_open
            && self
                .rules
                .iter()
                .all(|rule| rule.current_value.parsed_value().is_some())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRuleEditState {
    pub rule: String,
    pub category: String,
    pub initial_value: GameRuleValue,
    pub current_value: GameRuleEditValueState,
}

impl GameRuleEditState {
    pub fn has_pending_change(&self) -> bool {
        self.current_value.parsed_value() != Some(self.initial_value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameRuleEditValueState {
    Boolean {
        value: bool,
    },
    Integer {
        input: String,
        parsed_value: Option<i32>,
    },
}

impl GameRuleEditValueState {
    pub fn parsed_value(&self) -> Option<GameRuleValue> {
        match self {
            Self::Boolean { value } => Some(GameRuleValue::Boolean(*value)),
            Self::Integer { parsed_value, .. } => parsed_value.map(GameRuleValue::Integer),
        }
    }

    pub fn display_value(&self) -> String {
        match self {
            Self::Boolean { value } => value.to_string(),
            Self::Integer { input, .. } => input.clone(),
        }
    }
}

impl From<GameRuleValue> for GameRuleEditValueState {
    fn from(value: GameRuleValue) -> Self {
        match value {
            GameRuleValue::Boolean(value) => Self::Boolean { value },
            GameRuleValue::Integer(value) => Self::Integer {
                input: value.to_string(),
                parsed_value: Some(value),
            },
        }
    }
}

impl WorldStore {
    pub fn open_game_rules_screen(&mut self) -> bool {
        if !self.local_player_has_gamemaster_permission()
            || self.client_ui.current_game_rules_screen.is_some()
        {
            return false;
        }
        self.client_ui.replace_screen();
        self.client_ui.game_rules_screen_generation = self
            .client_ui
            .game_rules_screen_generation
            .saturating_add(1);
        self.client_ui.current_game_rules_screen = Some(GameRulesScreenState {
            generation: self.client_ui.game_rules_screen_generation,
            loading: true,
            ..GameRulesScreenState::default()
        });
        true
    }

    pub fn game_rules_screen(&self) -> Option<&GameRulesScreenState> {
        self.client_ui.current_game_rules_screen.as_ref()
    }

    pub fn set_game_rule_boolean(&mut self, rule: &str, value: bool) -> bool {
        let Some(edit) = self.game_rule_edit_mut(rule) else {
            return false;
        };
        let GameRuleEditValueState::Boolean {
            value: current_value,
        } = &mut edit.current_value
        else {
            return false;
        };
        if *current_value == value {
            return false;
        }
        *current_value = value;
        true
    }

    pub fn set_game_rule_integer_input(&mut self, rule: &str, input: String) -> bool {
        let parsed_value = vanilla_game_rule(rule)
            .and_then(|catalog| catalog.parse_value(&input))
            .and_then(|value| match value {
                GameRuleValue::Integer(value) => Some(value),
                GameRuleValue::Boolean(_) => None,
            });
        let Some(edit) = self.game_rule_edit_mut(rule) else {
            return false;
        };
        let GameRuleEditValueState::Integer {
            input: current_input,
            parsed_value: current_parsed_value,
        } = &mut edit.current_value
        else {
            return false;
        };
        if *current_input == input {
            return false;
        }
        *current_input = input;
        *current_parsed_value = parsed_value;
        true
    }

    pub fn request_close_game_rules_screen(&mut self) -> bool {
        let Some(screen) = self.client_ui.current_game_rules_screen.as_mut() else {
            return false;
        };
        if screen.has_pending_changes() {
            screen.discard_confirmation_open = true;
        } else {
            self.client_ui.current_game_rules_screen = None;
        }
        true
    }

    pub fn cancel_game_rules_screen_discard(&mut self) -> bool {
        let Some(screen) = self.client_ui.current_game_rules_screen.as_mut() else {
            return false;
        };
        if !screen.discard_confirmation_open {
            return false;
        }
        screen.discard_confirmation_open = false;
        true
    }

    pub fn confirm_game_rules_screen_discard(&mut self) -> bool {
        if !self
            .client_ui
            .current_game_rules_screen
            .as_ref()
            .is_some_and(|screen| screen.discard_confirmation_open)
        {
            return false;
        }
        self.client_ui.current_game_rules_screen = None;
        true
    }

    pub fn finish_game_rules_screen(&mut self) -> Option<SetGameRule> {
        let screen = self
            .client_ui
            .current_game_rules_screen
            .as_ref()
            .filter(|screen| screen.can_submit())?;
        let entries = screen
            .rules
            .iter()
            .filter_map(|rule| {
                let value = rule.current_value.parsed_value()?;
                (value != rule.initial_value).then(|| SetGameRuleEntry {
                    rule: rule.rule.clone(),
                    value: value.serialize(),
                })
            })
            .collect();
        self.client_ui.current_game_rules_screen = None;
        Some(SetGameRule { entries })
    }

    pub(crate) fn close_game_rules_screen(&mut self) -> bool {
        self.client_ui.current_game_rules_screen.take().is_some()
    }

    pub fn apply_game_rule_values(&mut self, packet: ProtocolGameRuleValues) {
        self.counters.game_rule_value_packets += 1;
        let values = packet
            .values
            .into_iter()
            .map(|value| GameRuleValueState {
                rule: value.rule,
                value: value.value,
            })
            .collect::<Vec<_>>();
        self.client_debug_game.last_game_rule_values = Some(GameRuleValuesState {
            values: values.clone(),
        });

        let Some(screen) = self
            .client_ui
            .current_game_rules_screen
            .as_mut()
            .filter(|screen| screen.loading)
        else {
            return;
        };
        let mut raw_values = BTreeMap::new();
        for value in values {
            raw_values.insert(value.rule, value.value);
        }
        screen.rules = VANILLA_GAME_RULES
            .iter()
            .filter_map(|catalog| {
                let initial_value = catalog.parse_value(raw_values.get(catalog.resource_id)?)?;
                Some(GameRuleEditState {
                    rule: catalog.resource_id.to_string(),
                    category: catalog.category.to_string(),
                    initial_value,
                    current_value: initial_value.into(),
                })
            })
            .collect();
        screen.rules.sort_by(|left, right| {
            (&left.category, &left.rule).cmp(&(&right.category, &right.rule))
        });
        screen.loading = false;
        screen.discard_confirmation_open = false;
    }

    pub fn last_game_rule_values(&self) -> Option<&GameRuleValuesState> {
        self.client_debug_game.last_game_rule_values.as_ref()
    }

    fn game_rule_edit_mut(&mut self, rule: &str) -> Option<&mut GameRuleEditState> {
        let screen = self
            .client_ui
            .current_game_rules_screen
            .as_mut()
            .filter(|screen| !screen.loading && !screen.discard_confirmation_open)?;
        screen.rules.iter_mut().find(|edit| edit.rule == rule)
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        game_rules::GameRuleValue,
        packets::{EntityEvent, GameRuleValue as ProtocolGameRuleValue, GameRuleValues},
    };

    use super::*;
    use crate::LocalPlayerPermissionLevel;

    fn operator_store() -> WorldStore {
        let mut store = WorldStore::new();
        store.local_player_id = Some(7);
        store.local_player.permission_level = LocalPlayerPermissionLevel::Gamemaster;
        store
    }

    fn game_rule_values(values: &[(&str, &str)]) -> GameRuleValues {
        GameRuleValues {
            values: values
                .iter()
                .map(|(rule, value)| ProtocolGameRuleValue {
                    rule: (*rule).to_string(),
                    value: (*value).to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn game_rules_screen_requires_permission_and_consumes_only_first_response() {
        let mut denied = WorldStore::new();
        denied.local_player_id = Some(7);
        assert!(!denied.open_game_rules_screen());

        let mut store = operator_store();
        assert!(store.open_game_rules_screen());
        assert!(store.game_rules_screen().unwrap().loading);
        assert!(!store.open_game_rules_screen());

        store.apply_game_rule_values(game_rule_values(&[
            ("minecraft:keep_inventory", "false"),
            ("minecraft:unknown", "true"),
            ("minecraft:max_snow_accumulation_height", "9"),
            ("minecraft:random_tick_speed", "6"),
            ("minecraft:keep_inventory", "true"),
        ]));

        let screen = store.game_rules_screen().unwrap();
        assert!(!screen.loading);
        assert_eq!(
            screen
                .rules
                .iter()
                .map(|rule| (rule.category.as_str(), rule.rule.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("player", "minecraft:keep_inventory"),
                ("updates", "minecraft:random_tick_speed"),
            ]
        );
        assert_eq!(screen.rules[0].initial_value, GameRuleValue::Boolean(true));
        assert_eq!(screen.rules[1].initial_value, GameRuleValue::Integer(6));

        store.apply_game_rule_values(game_rule_values(&[("minecraft:keep_inventory", "false")]));
        assert_eq!(
            store.game_rules_screen().unwrap().rules[0].initial_value,
            GameRuleValue::Boolean(true)
        );
        assert_eq!(
            store.last_game_rule_values().unwrap().values,
            vec![GameRuleValueState {
                rule: "minecraft:keep_inventory".to_string(),
                value: "false".to_string(),
            }]
        );
        assert_eq!(store.counters().game_rule_value_packets, 2);
        assert!(store.finish_game_rules_screen().unwrap().entries.is_empty());
        assert!(store.game_rules_screen().is_none());
    }

    #[test]
    fn game_rules_screen_validates_edits_and_submits_only_typed_changes() {
        let mut store = operator_store();
        assert!(store.open_game_rules_screen());
        store.apply_game_rule_values(game_rule_values(&[
            ("minecraft:keep_inventory", "false"),
            ("minecraft:max_snow_accumulation_height", "1"),
            ("minecraft:random_tick_speed", "3"),
        ]));

        assert!(store.set_game_rule_boolean("minecraft:keep_inventory", true));
        assert!(store.set_game_rule_integer_input("minecraft:random_tick_speed", "03".to_string()));
        assert!(store.game_rules_screen().unwrap().has_pending_changes());
        assert!(store.game_rules_screen().unwrap().can_submit());

        assert!(store.set_game_rule_integer_input(
            "minecraft:max_snow_accumulation_height",
            "9".to_string()
        ));
        assert!(!store.game_rules_screen().unwrap().can_submit());
        assert!(store.finish_game_rules_screen().is_none());
        assert!(store.game_rules_screen().is_some());
        assert!(store.request_close_game_rules_screen());
        assert!(store.game_rules_screen().unwrap().discard_confirmation_open);
        assert!(store.cancel_game_rules_screen_discard());

        assert!(store.set_game_rule_integer_input(
            "minecraft:max_snow_accumulation_height",
            "8".to_string()
        ));
        let packet = store.finish_game_rules_screen().unwrap();
        assert_eq!(
            packet.entries,
            vec![
                SetGameRuleEntry {
                    rule: "minecraft:keep_inventory".to_string(),
                    value: "true".to_string(),
                },
                SetGameRuleEntry {
                    rule: "minecraft:max_snow_accumulation_height".to_string(),
                    value: "8".to_string(),
                },
            ]
        );
        assert!(store.game_rules_screen().is_none());
    }

    #[test]
    fn game_rules_screen_closes_on_discard_permission_loss_level_reset_and_replacement() {
        let mut store = operator_store();
        assert!(store.open_game_rules_screen());
        store.apply_game_rule_values(game_rule_values(&[("minecraft:keep_inventory", "false")]));
        assert!(store.request_close_game_rules_screen());
        assert!(store.game_rules_screen().is_none());

        assert!(store.open_game_rules_screen());
        store.apply_game_rule_values(game_rule_values(&[("minecraft:keep_inventory", "false")]));
        assert!(store.set_game_rule_boolean("minecraft:keep_inventory", true));
        assert!(store.request_close_game_rules_screen());
        assert!(store.confirm_game_rules_screen_discard());
        assert!(store.game_rules_screen().is_none());

        assert!(store.open_game_rules_screen());
        assert!(store.apply_entity_event(EntityEvent {
            entity_id: 7,
            event_id: 25,
        }));
        assert!(store.game_rules_screen().is_none());

        store.local_player.permission_level = LocalPlayerPermissionLevel::Gamemaster;
        assert!(store.open_game_rules_screen());
        store.clear_client_level();
        assert!(store.game_rules_screen().is_none());

        store.local_player_id = Some(7);
        store.local_player.permission_level = LocalPlayerPermissionLevel::Gamemaster;
        assert!(store.open_game_rules_screen());
        assert!(store.open_advancements_screen());
        assert!(store.game_rules_screen().is_none());
    }
}
