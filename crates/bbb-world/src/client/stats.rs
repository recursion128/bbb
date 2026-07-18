use bbb_protocol::packets::AwardStats as ProtocolAwardStats;
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientStatsState {
    #[serde(default)]
    pub values: Vec<StatValueState>,
    #[serde(default)]
    pub identities: Vec<StatIdentityState>,
    #[serde(default)]
    pub last_update: Option<StatsUpdateState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsUpdateState {
    pub entries: Vec<StatValueState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatValueState {
    pub stat_type_id: i32,
    pub value_id: i32,
    pub amount: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatIdentityState {
    pub stat_type_id: i32,
    pub value_id: i32,
    pub stat_type_resource_id: Option<String>,
    pub value_registry: Option<String>,
    pub value_resource_id: Option<String>,
    pub formatter: Option<String>,
}

impl WorldStore {
    pub fn apply_award_stats(&mut self, packet: ProtocolAwardStats) {
        self.counters.award_stats_packets += 1;
        self.counters.award_stats_entries_received += packet.stats.len();

        let mut last_entries = Vec::with_capacity(packet.stats.len());
        for stat in packet.stats {
            let state = StatValueState {
                stat_type_id: stat.stat_type_id,
                value_id: stat.value_id,
                amount: stat.amount,
            };
            upsert_stat_value(&mut self.client_stats.values, state);
            let identity = self.resolve_stat_identity(stat.stat_type_id, stat.value_id);
            upsert_stat_identity(&mut self.client_stats.identities, identity);
            last_entries.push(state);
        }

        self.client_stats.last_update = Some(StatsUpdateState {
            entries: last_entries,
        });
        if self.client_ui.current_stats_screen {
            self.client_ui.stats_screen_loading = false;
        }
        self.counters.stats_tracked = self.client_stats.values.len();
        self.counters.last_award_stats_entry_count = self
            .client_stats
            .last_update
            .as_ref()
            .map_or(0, |update| update.entries.len());
    }

    pub fn client_stats(&self) -> &ClientStatsState {
        &self.client_stats
    }

    pub fn stat_value(&self, stat_type_id: i32, value_id: i32) -> Option<i32> {
        self.client_stats
            .values
            .iter()
            .find(|stat| stat.stat_type_id == stat_type_id && stat.value_id == value_id)
            .map(|stat| stat.amount)
    }

    pub fn last_stats_update(&self) -> Option<&StatsUpdateState> {
        self.client_stats.last_update.as_ref()
    }

    pub fn stat_identity(&self, stat_type_id: i32, value_id: i32) -> Option<&StatIdentityState> {
        self.client_stats
            .identities
            .iter()
            .find(|identity| identity.stat_type_id == stat_type_id && identity.value_id == value_id)
    }

    pub fn has_positive_stats_for_type(&self, stat_type_resource_id: &str) -> bool {
        self.client_stats.identities.iter().any(|identity| {
            identity.stat_type_resource_id.as_deref() == Some(stat_type_resource_id)
                && self
                    .stat_value(identity.stat_type_id, identity.value_id)
                    .is_some_and(|amount| amount > 0)
        })
    }

    fn resolve_stat_identity(&self, stat_type_id: i32, value_id: i32) -> StatIdentityState {
        let stat_type_resource_id = self
            .registry_resource_id_by_holder_id("minecraft:stat_type", stat_type_id)
            .map(str::to_owned);
        let descriptor = stat_type_resource_id
            .as_deref()
            .and_then(bbb_protocol::stat_types::vanilla_stat_type_by_resource_id);
        let value_registry = descriptor.map(|descriptor| descriptor.value_registry.to_string());
        let value_resource_id = value_registry
            .as_deref()
            .and_then(|registry| self.registry_resource_id_by_holder_id(registry, value_id))
            .map(str::to_owned);
        let formatter = (value_registry.as_deref() == Some("minecraft:custom_stat"))
            .then(|| {
                value_resource_id
                    .as_deref()
                    .and_then(bbb_protocol::stat_types::vanilla_custom_stat_by_resource_id)
                    .map(|entry| entry.formatter.to_string())
            })
            .flatten();
        StatIdentityState {
            stat_type_id,
            value_id,
            stat_type_resource_id,
            value_registry,
            value_resource_id,
            formatter,
        }
    }
}

fn upsert_stat_value(values: &mut Vec<StatValueState>, state: StatValueState) {
    match values.binary_search_by_key(&(state.stat_type_id, state.value_id), |value| {
        (value.stat_type_id, value.value_id)
    }) {
        Ok(index) => values[index] = state,
        Err(index) => values.insert(index, state),
    }
}

fn upsert_stat_identity(values: &mut Vec<StatIdentityState>, state: StatIdentityState) {
    match values.binary_search_by_key(&(state.stat_type_id, state.value_id), |value| {
        (value.stat_type_id, value.value_id)
    }) {
        Ok(index) => values[index] = state,
        Err(index) => values.insert(index, state),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;
    use bbb_protocol::packets::{AwardStats, StatUpdate};
    use bbb_protocol::stat_types::{
        VANILLA_STAT_TYPES, VANILLA_STAT_TYPE_BLOCK_MINED_ID, VANILLA_STAT_TYPE_CUSTOM_ID,
        VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
    };

    #[test]
    fn award_stats_updates_canonical_values_and_last_batch() {
        let mut store = WorldStore::new();

        store.apply_award_stats(AwardStats {
            stats: vec![
                StatUpdate {
                    stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                    value_id: 10,
                    amount: 3,
                },
                StatUpdate {
                    stat_type_id: VANILLA_STAT_TYPE_BLOCK_MINED_ID,
                    value_id: 4,
                    amount: 11,
                },
                StatUpdate {
                    stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                    value_id: 10,
                    amount: 5,
                },
            ],
        });

        assert_eq!(store.stat_value(VANILLA_STAT_TYPE_CUSTOM_ID, 10), Some(5));
        assert_eq!(
            store.stat_value(VANILLA_STAT_TYPE_BLOCK_MINED_ID, 4),
            Some(11)
        );
        assert_eq!(store.stat_value(VANILLA_STAT_TYPE_ITEM_CRAFTED_ID, 4), None);
        assert_eq!(
            store.stat_identity(VANILLA_STAT_TYPE_CUSTOM_ID, 10),
            Some(&StatIdentityState {
                stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                value_id: 10,
                stat_type_resource_id: Some("minecraft:custom".to_string()),
                value_registry: Some("minecraft:custom_stat".to_string()),
                value_resource_id: Some("minecraft:fall_one_cm".to_string()),
                formatter: Some("DISTANCE".to_string()),
            })
        );
        assert!(store
            .stat_identity(VANILLA_STAT_TYPE_BLOCK_MINED_ID, 4)
            .is_some_and(|identity| identity.stat_type_resource_id.as_deref()
                == Some("minecraft:mined")
                && identity.value_registry.as_deref() == Some("minecraft:block")
                && identity.value_resource_id.is_some()));
        assert_eq!(
            store.client_stats().values,
            vec![
                StatValueState {
                    stat_type_id: VANILLA_STAT_TYPE_BLOCK_MINED_ID,
                    value_id: 4,
                    amount: 11,
                },
                StatValueState {
                    stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                    value_id: 10,
                    amount: 5,
                },
            ]
        );
        assert_eq!(
            store.last_stats_update(),
            Some(&StatsUpdateState {
                entries: vec![
                    StatValueState {
                        stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                        value_id: 10,
                        amount: 3,
                    },
                    StatValueState {
                        stat_type_id: VANILLA_STAT_TYPE_BLOCK_MINED_ID,
                        value_id: 4,
                        amount: 11,
                    },
                    StatValueState {
                        stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                        value_id: 10,
                        amount: 5,
                    },
                ],
            })
        );

        let counters = store.counters();
        assert_eq!(counters.award_stats_packets, 1);
        assert_eq!(counters.award_stats_entries_received, 3);
        assert_eq!(counters.last_award_stats_entry_count, 3);
        assert_eq!(counters.stats_tracked, 2);
    }

    #[test]
    fn award_stats_empty_batch_keeps_values_and_records_last_empty_update() {
        let mut store = WorldStore::new();
        store.apply_award_stats(AwardStats {
            stats: vec![StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_BLOCK_MINED_ID,
                value_id: 1,
                amount: 2,
            }],
        });

        store.apply_award_stats(AwardStats { stats: Vec::new() });

        assert_eq!(
            store.stat_value(VANILLA_STAT_TYPE_BLOCK_MINED_ID, 1),
            Some(2)
        );
        assert_eq!(
            store.last_stats_update(),
            Some(&StatsUpdateState {
                entries: Vec::new(),
            })
        );
        let counters = store.counters();
        assert_eq!(counters.award_stats_packets, 2);
        assert_eq!(counters.award_stats_entries_received, 1);
        assert_eq!(counters.last_award_stats_entry_count, 0);
        assert_eq!(counters.stats_tracked, 1);
    }

    #[test]
    fn award_stats_retains_explicit_unresolved_identity_without_guessing() {
        let mut store = WorldStore::new();
        let unknown_stat_type_id = i32::try_from(VANILLA_STAT_TYPES.len()).unwrap();
        store.apply_award_stats(AwardStats {
            stats: vec![StatUpdate {
                stat_type_id: unknown_stat_type_id,
                value_id: 123,
                amount: 7,
            }],
        });

        assert_eq!(store.stat_value(unknown_stat_type_id, 123), Some(7));
        assert_eq!(
            store.stat_identity(unknown_stat_type_id, 123),
            Some(&StatIdentityState {
                stat_type_id: unknown_stat_type_id,
                value_id: 123,
                stat_type_resource_id: None,
                value_registry: None,
                value_resource_id: None,
                formatter: None,
            })
        );
    }

    #[test]
    fn award_stats_only_completes_an_open_stats_screen_request() {
        let mut store = WorldStore::new();
        store.apply_award_stats(AwardStats { stats: Vec::new() });
        assert!(!store.stats_screen_is_loading());

        assert!(store.open_stats_screen());
        assert!(store.stats_screen_is_loading());
        store.apply_award_stats(AwardStats { stats: Vec::new() });
        assert!(!store.stats_screen_is_loading());

        assert!(store.close_stats_screen());
        assert!(store.open_stats_screen());
        assert!(store.stats_screen_is_loading());
    }

    #[test]
    fn award_stats_resolves_reordered_session_holder_identity() {
        let mut store = WorldStore::new();
        let stat_type_entries = vec![RegistryPacketEntry {
            id: "minecraft:custom".to_string(),
            has_data: false,
            raw_data_len: 0,
            raw_data: None,
            nbt: None,
            known_pack_data: None,
        }];
        let reordered_custom_stat_type_id = i32::try_from(
            stat_type_entries
                .iter()
                .position(|entry| entry.id == "minecraft:custom")
                .unwrap(),
        )
        .unwrap();
        store.record_registry_entries("minecraft:stat_type", 0, stat_type_entries);
        store.record_registry_entries(
            "minecraft:custom_stat",
            0,
            vec![RegistryPacketEntry {
                id: "minecraft:walk_one_cm".to_string(),
                has_data: false,
                raw_data_len: 0,
                raw_data: None,
                nbt: None,
                known_pack_data: None,
            }],
        );
        store.apply_award_stats(AwardStats {
            stats: vec![StatUpdate {
                stat_type_id: reordered_custom_stat_type_id,
                value_id: 0,
                amount: 250,
            }],
        });

        let identity = store
            .stat_identity(reordered_custom_stat_type_id, 0)
            .unwrap();
        assert_eq!(
            identity.stat_type_resource_id.as_deref(),
            Some("minecraft:custom")
        );
        assert_eq!(
            identity.value_registry.as_deref(),
            Some("minecraft:custom_stat")
        );
        assert_eq!(
            identity.value_resource_id.as_deref(),
            Some("minecraft:walk_one_cm")
        );
        assert_eq!(identity.formatter.as_deref(), Some("DISTANCE"));
    }
}
