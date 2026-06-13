use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::{
    AdvancementCriterionProgressSummary as ProtocolAdvancementCriterionProgressSummary,
    AdvancementProgressSummary as ProtocolAdvancementProgressSummary,
    AdvancementSummary as ProtocolAdvancementSummary,
    UpdateAdvancements as ProtocolUpdateAdvancements,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientAdvancementsState {
    pub advancements: BTreeMap<String, ProtocolAdvancementSummary>,
    pub progress: BTreeMap<String, ProtocolAdvancementProgressSummary>,
}

impl WorldStore {
    pub fn apply_update_advancements(&mut self, packet: ProtocolUpdateAdvancements) {
        self.counters.update_advancements_packets += 1;
        if packet.reset {
            self.counters.update_advancements_reset_packets += 1;
            self.advancements.advancements.clear();
            self.advancements.progress.clear();
        }
        if packet.show_advancements {
            self.counters.update_advancements_show_packets += 1;
        }

        self.counters.advancements_added_received += packet.added.len();
        self.counters.advancements_removed_received += packet.removed.len();
        self.counters.advancement_progress_received += packet.progress.len();

        for id in packet.removed {
            self.remove_advancement_and_children(&id);
        }

        self.add_advancements_when_parents_are_known(packet.added);

        for mut progress in packet.progress {
            if self.advancements.advancements.contains_key(&progress.id) {
                progress = normalize_progress_for_advancement(
                    self.advancements
                        .advancements
                        .get(&progress.id)
                        .expect("checked above"),
                    progress,
                );
                self.advancements
                    .progress
                    .insert(progress.id.clone(), progress);
            } else {
                self.counters.advancement_progress_updates_ignored += 1;
            }
        }

        self.refresh_advancement_counters();
    }

    pub fn advancements(&self) -> &ClientAdvancementsState {
        &self.advancements
    }

    fn remove_advancement_and_children(&mut self, id: &str) -> usize {
        let children: Vec<String> = self
            .advancements
            .advancements
            .iter()
            .filter_map(|(child_id, advancement)| {
                (advancement.parent.as_deref() == Some(id)).then(|| child_id.clone())
            })
            .collect();

        let mut removed = 0;
        for child in children {
            removed += self.remove_advancement_and_children(&child);
        }

        if self.advancements.advancements.remove(id).is_some() {
            self.advancements.progress.remove(id);
            removed += 1;
        }
        removed
    }

    fn add_advancements_when_parents_are_known(
        &mut self,
        mut pending: Vec<ProtocolAdvancementSummary>,
    ) {
        while !pending.is_empty() {
            let before = pending.len();
            let mut index = 0;
            while index < pending.len() {
                let parent_ready = match pending[index].parent.as_deref() {
                    Some(parent) => self.advancements.advancements.contains_key(parent),
                    None => true,
                };
                if parent_ready {
                    let advancement = pending.remove(index);
                    self.advancements
                        .advancements
                        .insert(advancement.id.clone(), advancement);
                } else {
                    index += 1;
                }
            }

            if pending.len() == before {
                self.counters.advancements_adds_ignored += pending.len();
                break;
            }
        }
    }

    fn refresh_advancement_counters(&mut self) {
        self.counters.advancements_tracked = self.advancements.advancements.len();
        self.counters.advancement_roots_tracked = self
            .advancements
            .advancements
            .values()
            .filter(|advancement| advancement.parent.is_none())
            .count();
        self.counters.advancement_progress_tracked = self.advancements.progress.len();
        self.counters.advancement_progress_criteria_tracked = self
            .advancements
            .progress
            .values()
            .map(|progress| progress.criteria.len())
            .sum();
    }
}

fn normalize_progress_for_advancement(
    advancement: &ProtocolAdvancementSummary,
    mut progress: ProtocolAdvancementProgressSummary,
) -> ProtocolAdvancementProgressSummary {
    let expected_names: BTreeSet<String> = advancement
        .requirements
        .iter()
        .flat_map(|group| group.iter().cloned())
        .collect();

    progress
        .criteria
        .retain(|criterion| expected_names.contains(&criterion.name));
    let existing_names: BTreeSet<String> = progress
        .criteria
        .iter()
        .map(|criterion| criterion.name.clone())
        .collect();
    for name in expected_names.difference(&existing_names) {
        progress
            .criteria
            .push(ProtocolAdvancementCriterionProgressSummary {
                name: name.clone(),
                obtained_epoch_millis: None,
            });
    }
    progress
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        AdvancementCriterionProgressSummary, AdvancementProgressSummary, AdvancementSummary,
        UpdateAdvancements,
    };

    #[test]
    fn update_advancements_applies_reset_removal_and_known_progress_only() {
        let mut store = WorldStore::new();

        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added: vec![
                advancement_with_requirements(
                    "minecraft:story/root",
                    None,
                    vec![vec!["mine_stone", "get_log"]],
                ),
                advancement("minecraft:story/mine_stone", Some("minecraft:story/root")),
                advancement("minecraft:orphan", Some("minecraft:missing_parent")),
            ],
            removed: Vec::new(),
            progress: vec![
                progress("minecraft:story/root", vec![("mine_stone", Some(10))]),
                progress("minecraft:unknown", vec![("ignored", Some(20))]),
            ],
            show_advancements: true,
        });

        assert!(store
            .advancements()
            .advancements
            .contains_key("minecraft:story/root"));
        assert!(store
            .advancements()
            .progress
            .contains_key("minecraft:story/root"));
        assert_eq!(
            store
                .advancements()
                .progress
                .get("minecraft:story/root")
                .unwrap()
                .criteria
                .len(),
            2
        );
        assert!(!store
            .advancements()
            .progress
            .contains_key("minecraft:unknown"));
        assert!(!store
            .advancements()
            .advancements
            .contains_key("minecraft:orphan"));

        store.apply_update_advancements(UpdateAdvancements {
            reset: false,
            added: vec![advancement("minecraft:nether/root", None)],
            removed: vec!["minecraft:story/root".to_string()],
            progress: Vec::new(),
            show_advancements: false,
        });

        assert!(!store
            .advancements()
            .advancements
            .contains_key("minecraft:story/root"));
        assert!(!store
            .advancements()
            .advancements
            .contains_key("minecraft:story/mine_stone"));
        assert!(!store
            .advancements()
            .progress
            .contains_key("minecraft:story/root"));
        assert!(store
            .advancements()
            .advancements
            .contains_key("minecraft:nether/root"));

        let counters = store.counters();
        assert_eq!(counters.update_advancements_packets, 2);
        assert_eq!(counters.update_advancements_reset_packets, 1);
        assert_eq!(counters.update_advancements_show_packets, 1);
        assert_eq!(counters.advancements_added_received, 4);
        assert_eq!(counters.advancements_removed_received, 1);
        assert_eq!(counters.advancements_adds_ignored, 1);
        assert_eq!(counters.advancement_progress_received, 2);
        assert_eq!(counters.advancement_progress_updates_ignored, 1);
        assert_eq!(counters.advancements_tracked, 1);
        assert_eq!(counters.advancement_roots_tracked, 1);
        assert_eq!(counters.advancement_progress_tracked, 0);
        assert_eq!(counters.advancement_progress_criteria_tracked, 0);
    }

    fn advancement(id: &str, parent: Option<&str>) -> AdvancementSummary {
        advancement_with_requirements(id, parent, Vec::new())
    }

    fn advancement_with_requirements(
        id: &str,
        parent: Option<&str>,
        requirements: Vec<Vec<&str>>,
    ) -> AdvancementSummary {
        AdvancementSummary {
            id: id.to_string(),
            parent: parent.map(str::to_string),
            display: None,
            requirements: requirements
                .into_iter()
                .map(|group| group.into_iter().map(str::to_string).collect())
                .collect(),
            sends_telemetry_event: false,
        }
    }

    fn progress(id: &str, criteria: Vec<(&str, Option<i64>)>) -> AdvancementProgressSummary {
        AdvancementProgressSummary {
            id: id.to_string(),
            criteria: criteria
                .into_iter()
                .map(
                    |(name, obtained_epoch_millis)| AdvancementCriterionProgressSummary {
                        name: name.to_string(),
                        obtained_epoch_millis,
                    },
                )
                .collect(),
        }
    }
}
