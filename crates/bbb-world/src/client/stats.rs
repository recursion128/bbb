use bbb_protocol::packets::AwardStats as ProtocolAwardStats;
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientStatsState {
    #[serde(default)]
    pub values: Vec<StatValueState>,
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
            last_entries.push(state);
        }

        self.client_stats.last_update = Some(StatsUpdateState {
            entries: last_entries,
        });
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
}

fn upsert_stat_value(values: &mut Vec<StatValueState>, state: StatValueState) {
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
    use bbb_protocol::packets::{AwardStats, StatUpdate};

    #[test]
    fn award_stats_updates_canonical_values_and_last_batch() {
        let mut store = WorldStore::new();

        store.apply_award_stats(AwardStats {
            stats: vec![
                StatUpdate {
                    stat_type_id: 8,
                    value_id: 10,
                    amount: 3,
                },
                StatUpdate {
                    stat_type_id: 0,
                    value_id: 4,
                    amount: 11,
                },
                StatUpdate {
                    stat_type_id: 8,
                    value_id: 10,
                    amount: 5,
                },
            ],
        });

        assert_eq!(store.stat_value(8, 10), Some(5));
        assert_eq!(store.stat_value(0, 4), Some(11));
        assert_eq!(store.stat_value(1, 4), None);
        assert_eq!(
            store.client_stats().values,
            vec![
                StatValueState {
                    stat_type_id: 0,
                    value_id: 4,
                    amount: 11,
                },
                StatValueState {
                    stat_type_id: 8,
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
                        stat_type_id: 8,
                        value_id: 10,
                        amount: 3,
                    },
                    StatValueState {
                        stat_type_id: 0,
                        value_id: 4,
                        amount: 11,
                    },
                    StatValueState {
                        stat_type_id: 8,
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
                stat_type_id: 0,
                value_id: 1,
                amount: 2,
            }],
        });

        store.apply_award_stats(AwardStats { stats: Vec::new() });

        assert_eq!(store.stat_value(0, 1), Some(2));
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
}
