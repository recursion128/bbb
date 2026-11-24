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

/// The edits of an open `InGameGameRulesScreen`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRulesScreenState {
    #[serde(default)]
    pub loading: bool,
    #[serde(default)]
    pub discard_confirmation_open: bool,
    #[serde(default)]
    pub rules: Vec<GameRuleEditState>,
}

impl GameRulesScreenState {
    pub fn loading() -> Self {
        Self {
            loading: true,
            ..Self::default()
        }
    }

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

    /// The first `GameRuleValues` after opening fills the list; later ones
    /// leave the edits alone.
    pub fn load(&mut self, values: &GameRuleValuesState) {
        if !self.loading {
            return;
        }
        let raw_values = values
            .values
            .iter()
            .map(|value| (value.rule.as_str(), value.value.as_str()))
            .collect::<BTreeMap<_, _>>();
        self.rules = VANILLA_GAME_RULES
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
        self.rules.sort_by(|left, right| {
            (&left.category, &left.rule).cmp(&(&right.category, &right.rule))
        });
        self.loading = false;
        self.discard_confirmation_open = false;
    }

    pub fn set_boolean(&mut self, rule: &str, value: bool) -> bool {
        let Some(edit) = self.edit_mut(rule) else {
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

    pub fn set_integer_input(&mut self, rule: &str, input: String) -> bool {
        let parsed_value = vanilla_game_rule(rule)
            .and_then(|catalog| catalog.parse_value(&input))
            .and_then(|value| match value {
                GameRuleValue::Integer(value) => Some(value),
                GameRuleValue::Boolean(_) => None,
            });
        let Some(edit) = self.edit_mut(rule) else {
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

    /// `onClose`: returns whether the screen closes; pending changes ask first.
    pub fn request_close(&mut self) -> bool {
        if self.has_pending_changes() {
            self.discard_confirmation_open = true;
            false
        } else {
            true
        }
    }

    pub fn cancel_discard(&mut self) -> bool {
        std::mem::replace(&mut self.discard_confirmation_open, false)
    }

    /// The Done button's `SetGameRule` with only the changed rules.
    pub fn finish(&self) -> Option<SetGameRule> {
        if !self.can_submit() {
            return None;
        }
        let entries = self
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
        Some(SetGameRule { entries })
    }

    fn edit_mut(&mut self, rule: &str) -> Option<&mut GameRuleEditState> {
        if self.loading || self.discard_confirmation_open {
            return None;
        }
        self.rules.iter_mut().find(|edit| edit.rule == rule)
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
    pub fn apply_game_rule_values(&mut self, packet: ProtocolGameRuleValues) {
        self.counters.game_rule_value_packets += 1;
        let values = packet
            .values
            .into_iter()
            .map(|value| GameRuleValueState {
                rule: value.rule,
                value: value.value,
            })
            .collect();
        self.client_debug_game.last_game_rule_values = Some(GameRuleValuesState { values });
        self.push_screen_request(crate::ScreenRequest::GameRuleValues);
    }

    pub fn last_game_rule_values(&self) -> Option<&GameRuleValuesState> {
        self.client_debug_game.last_game_rule_values.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        game_rules::GameRuleValue,
        packets::{GameRuleValue as ProtocolGameRuleValue, GameRuleValues},
    };

    use super::*;

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

    fn loaded_screen(values: &[(&str, &str)]) -> GameRulesScreenState {
        let mut store = WorldStore::new();
        store.apply_game_rule_values(game_rule_values(values));
        let mut screen = GameRulesScreenState::loading();
        screen.load(store.last_game_rule_values().unwrap());
        screen
    }

    #[test]
    fn game_rules_screen_consumes_only_first_response() {
        let mut store = WorldStore::new();
        store.apply_game_rule_values(game_rule_values(&[
            ("minecraft:keep_inventory", "false"),
            ("minecraft:unknown", "true"),
            ("minecraft:max_snow_accumulation_height", "9"),
            ("minecraft:random_tick_speed", "6"),
            ("minecraft:keep_inventory", "true"),
        ]));
        assert_eq!(
            store.screen_requests(),
            [crate::ScreenRequest::GameRuleValues]
        );
        let mut screen = GameRulesScreenState::loading();
        screen.load(store.last_game_rule_values().unwrap());

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
        screen.load(store.last_game_rule_values().unwrap());
        assert_eq!(screen.rules[0].initial_value, GameRuleValue::Boolean(true));
        assert_eq!(
            store.last_game_rule_values().unwrap().values,
            vec![GameRuleValueState {
                rule: "minecraft:keep_inventory".to_string(),
                value: "false".to_string(),
            }]
        );
        assert_eq!(store.counters().game_rule_value_packets, 2);
        assert!(screen.finish().unwrap().entries.is_empty());
    }

    #[test]
    fn game_rules_screen_validates_edits_and_submits_only_typed_changes() {
        let mut screen = loaded_screen(&[
            ("minecraft:keep_inventory", "false"),
            ("minecraft:max_snow_accumulation_height", "1"),
            ("minecraft:random_tick_speed", "3"),
        ]);

        assert!(screen.set_boolean("minecraft:keep_inventory", true));
        assert!(screen.set_integer_input("minecraft:random_tick_speed", "03".to_string()));
        assert!(screen.has_pending_changes());
        assert!(screen.can_submit());

        assert!(screen.set_integer_input("minecraft:max_snow_accumulation_height", "9".to_string()));
        assert!(!screen.can_submit());
        assert!(screen.finish().is_none());
        assert!(!screen.request_close());
        assert!(screen.discard_confirmation_open);
        assert!(screen.cancel_discard());

        assert!(screen.set_integer_input("minecraft:max_snow_accumulation_height", "8".to_string()));
        let packet = screen.finish().unwrap();
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
    }
}
