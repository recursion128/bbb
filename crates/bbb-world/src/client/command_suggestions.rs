use std::collections::BTreeMap;

use bbb_protocol::packets::CommandSuggestions as ProtocolCommandSuggestions;
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestionsState {
    pub by_id: BTreeMap<i32, CommandSuggestionsResultState>,
    pub last_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestionsResultState {
    pub id: i32,
    pub start: i32,
    pub length: i32,
    pub suggestions: Vec<CommandSuggestionState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestionState {
    pub text: String,
    pub tooltip: Option<String>,
}

impl From<ProtocolCommandSuggestions> for CommandSuggestionsResultState {
    fn from(packet: ProtocolCommandSuggestions) -> Self {
        Self {
            id: packet.id,
            start: packet.start,
            length: packet.length,
            suggestions: packet
                .suggestions
                .into_iter()
                .map(|suggestion| CommandSuggestionState {
                    text: suggestion.text,
                    tooltip: suggestion.tooltip,
                })
                .collect(),
        }
    }
}

impl WorldStore {
    pub fn apply_command_suggestions(
        &mut self,
        packet: ProtocolCommandSuggestions,
    ) -> &CommandSuggestionsResultState {
        self.counters.command_suggestion_packets += 1;
        let id = packet.id;
        let result = CommandSuggestionsResultState::from(packet);
        self.command_suggestions.last_id = Some(id);
        self.command_suggestions.by_id.insert(id, result);
        self.update_command_suggestion_count();
        self.command_suggestions
            .by_id
            .get(&id)
            .expect("command suggestions result was inserted")
    }

    pub fn command_suggestions(&self) -> &CommandSuggestionsState {
        &self.command_suggestions
    }

    pub fn command_suggestions_by_id(&self, id: i32) -> Option<&CommandSuggestionsResultState> {
        self.command_suggestions.by_id.get(&id)
    }

    pub fn last_command_suggestions(&self) -> Option<&CommandSuggestionsResultState> {
        self.command_suggestions
            .last_id
            .and_then(|id| self.command_suggestions.by_id.get(&id))
    }

    fn update_command_suggestion_count(&mut self) {
        self.counters.command_suggestion_entries_tracked = self
            .command_suggestions
            .by_id
            .values()
            .map(|result| result.suggestions.len())
            .sum();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{CommandSuggestion, CommandSuggestions};

    #[test]
    fn command_suggestions_store_by_id_and_last_response() {
        let mut store = WorldStore::new();
        store.apply_command_suggestions(CommandSuggestions {
            id: 7,
            start: 1,
            length: 4,
            suggestions: vec![
                CommandSuggestion {
                    text: "give".to_string(),
                    tooltip: Some("Run give".to_string()),
                },
                CommandSuggestion {
                    text: "gamemode".to_string(),
                    tooltip: None,
                },
            ],
        });

        let result = store.command_suggestions_by_id(7).unwrap();
        assert_eq!(result.start, 1);
        assert_eq!(result.length, 4);
        assert_eq!(result.suggestions.len(), 2);
        assert_eq!(result.suggestions[0].text, "give");
        assert_eq!(result.suggestions[0].tooltip.as_deref(), Some("Run give"));
        assert_eq!(store.last_command_suggestions(), Some(result));
        assert_eq!(store.counters().command_suggestion_packets, 1);
        assert_eq!(store.counters().command_suggestion_entries_tracked, 2);
    }

    #[test]
    fn command_suggestions_update_existing_response_id() {
        let mut store = WorldStore::new();
        store.apply_command_suggestions(CommandSuggestions {
            id: 7,
            start: 0,
            length: 1,
            suggestions: vec![CommandSuggestion {
                text: "first".to_string(),
                tooltip: None,
            }],
        });
        store.apply_command_suggestions(CommandSuggestions {
            id: 7,
            start: 2,
            length: 3,
            suggestions: vec![CommandSuggestion {
                text: "second".to_string(),
                tooltip: None,
            }],
        });

        assert_eq!(store.command_suggestions().by_id.len(), 1);
        let result = store.command_suggestions_by_id(7).unwrap();
        assert_eq!(result.start, 2);
        assert_eq!(result.suggestions[0].text, "second");
        assert_eq!(store.counters().command_suggestion_packets, 2);
        assert_eq!(store.counters().command_suggestion_entries_tracked, 1);
    }

    #[test]
    fn command_suggestions_count_all_cached_entries() {
        let mut store = WorldStore::new();
        store.apply_command_suggestions(CommandSuggestions {
            id: 7,
            start: 0,
            length: 1,
            suggestions: vec![CommandSuggestion {
                text: "first".to_string(),
                tooltip: None,
            }],
        });
        store.apply_command_suggestions(CommandSuggestions {
            id: 8,
            start: 0,
            length: 1,
            suggestions: vec![
                CommandSuggestion {
                    text: "second".to_string(),
                    tooltip: None,
                },
                CommandSuggestion {
                    text: "third".to_string(),
                    tooltip: None,
                },
            ],
        });

        assert_eq!(store.command_suggestions().by_id.len(), 2);
        assert_eq!(store.counters().command_suggestion_entries_tracked, 3);
    }
}
