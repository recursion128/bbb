use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::{
    AdvancementCriterionProgressSummary as ProtocolAdvancementCriterionProgressSummary,
    AdvancementIconSummary as ProtocolAdvancementIconSummary,
    AdvancementProgressSummary as ProtocolAdvancementProgressSummary,
    AdvancementSummary as ProtocolAdvancementSummary,
    SelectAdvancementsTab as ProtocolSelectAdvancementsTab,
    UpdateAdvancements as ProtocolUpdateAdvancements,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

pub const MAX_ADVANCEMENT_ROOT_TABS: usize = 26;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientAdvancementsState {
    pub advancements: BTreeMap<String, ProtocolAdvancementSummary>,
    pub progress: BTreeMap<String, ProtocolAdvancementProgressSummary>,
    #[serde(default)]
    pub selected_tab: Option<String>,
    #[serde(default)]
    pub root_order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancementRootTabSummary {
    pub id: String,
    pub title: String,
    pub icon: ProtocolAdvancementIconSummary,
    pub display_index: usize,
}

impl WorldStore {
    pub fn apply_select_advancements_tab(&mut self, packet: ProtocolSelectAdvancementsTab) {
        self.counters.select_advancements_tab_packets += 1;
        self.advancements.selected_tab = packet
            .tab
            .filter(|tab| self.advancements.advancements.contains_key(tab));
    }

    pub fn apply_update_advancements(&mut self, packet: ProtocolUpdateAdvancements) {
        self.counters.update_advancements_packets += 1;
        if packet.reset {
            self.counters.update_advancements_reset_packets += 1;
            self.advancements.advancements.clear();
            self.advancements.progress.clear();
            self.advancements.root_order.clear();
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

    pub fn client_advancements(&self) -> &ClientAdvancementsState {
        self.advancements()
    }

    pub fn selected_advancements_tab(&self) -> Option<&str> {
        self.advancements.selected_tab.as_deref()
    }

    pub fn advancement_root_tabs(&self) -> Vec<AdvancementRootTabSummary> {
        let mut tabs = Vec::new();
        for root in &self.advancements.root_order {
            if tabs.len() >= MAX_ADVANCEMENT_ROOT_TABS {
                break;
            }
            if let Some(summary) = self.advancement_root_tab_summary(root, tabs.len()) {
                tabs.push(summary);
            }
        }
        tabs
    }

    pub fn selected_advancement_root_tab(&self) -> Option<AdvancementRootTabSummary> {
        let selected = self.advancements.selected_tab.as_deref()?;
        self.advancement_root_tabs()
            .into_iter()
            .find(|tab| tab.id == selected)
    }

    pub fn select_advancements_root_tab(&mut self, id: &str) -> Option<String> {
        if !self.advancement_is_root_tab(id) {
            return None;
        }
        let id = id.to_string();
        self.advancements.selected_tab = Some(id.clone());
        Some(id)
    }

    pub fn ensure_advancements_screen_selected_tab(&mut self) -> Option<String> {
        if self
            .advancements
            .selected_tab
            .as_deref()
            .is_some_and(|tab| self.advancement_is_root_tab(tab))
        {
            return None;
        }

        let first_root = self.first_advancement_root_tab_id();
        self.advancements.selected_tab = first_root.clone();
        first_root
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

        if let Some(removed_advancement) = self.advancements.advancements.remove(id) {
            if removed_advancement.parent.is_none() {
                self.advancements.root_order.retain(|root| root != id);
            }
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
                    self.insert_advancement(advancement);
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

    fn insert_advancement(&mut self, advancement: ProtocolAdvancementSummary) {
        let id = advancement.id.clone();
        let is_root = advancement.parent.is_none();
        let previous = self
            .advancements
            .advancements
            .insert(id.clone(), advancement);
        if previous
            .as_ref()
            .is_some_and(|advancement| advancement.parent.is_none() && !is_root)
        {
            self.advancements.root_order.retain(|root| root != &id);
        }
        if is_root && !self.advancements.root_order.iter().any(|root| root == &id) {
            self.advancements.root_order.push(id);
        }
    }

    fn first_advancement_root_tab_id(&self) -> Option<String> {
        self.advancement_root_tabs()
            .first()
            .map(|tab| tab.id.clone())
    }

    fn advancement_is_root_tab(&self, id: &str) -> bool {
        self.advancement_root_tabs().iter().any(|tab| tab.id == id)
    }

    fn advancement_root_tab_summary(
        &self,
        root: &str,
        display_index: usize,
    ) -> Option<AdvancementRootTabSummary> {
        let advancement = self.advancements.advancements.get(root)?;
        if advancement.parent.is_some() {
            return None;
        }
        let display = advancement.display.as_ref()?;
        Some(AdvancementRootTabSummary {
            id: root.to_string(),
            title: display.title.clone(),
            icon: display.icon.clone(),
            display_index,
        })
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
        AdvancementCriterionProgressSummary, AdvancementDisplaySummary, AdvancementFrameType,
        AdvancementIconSummary, AdvancementProgressSummary, AdvancementSummary,
        DataComponentPatchSummary, SelectAdvancementsTab, UpdateAdvancements,
    };

    #[test]
    fn select_advancements_tab_tracks_nullable_tab_and_counter() {
        let mut store = WorldStore::new();
        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added: vec![advancement("minecraft:story/root", None)],
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        });

        store.apply_select_advancements_tab(SelectAdvancementsTab {
            tab: Some("minecraft:story/root".to_string()),
        });

        assert_eq!(
            store.client_advancements().selected_tab.as_deref(),
            Some("minecraft:story/root")
        );
        assert_eq!(
            store.selected_advancements_tab(),
            Some("minecraft:story/root")
        );
        assert_eq!(store.counters().select_advancements_tab_packets, 1);

        store.apply_select_advancements_tab(SelectAdvancementsTab {
            tab: Some("minecraft:missing".to_string()),
        });

        assert_eq!(store.advancements().selected_tab, None);
        assert_eq!(store.selected_advancements_tab(), None);
        assert_eq!(store.counters().select_advancements_tab_packets, 2);

        store.apply_select_advancements_tab(SelectAdvancementsTab {
            tab: Some("minecraft:story/root".to_string()),
        });
        store.apply_select_advancements_tab(SelectAdvancementsTab { tab: None });

        assert_eq!(store.advancements().selected_tab, None);
        assert_eq!(store.selected_advancements_tab(), None);
        assert_eq!(store.counters().select_advancements_tab_packets, 4);
    }

    #[test]
    fn screen_open_selects_first_root_in_packet_order() {
        let mut store = WorldStore::new();
        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added: vec![
                advancement("minecraft:z/root", None),
                displayed_advancement("minecraft:y/root", None),
                displayed_advancement("minecraft:a/root", None),
                advancement("minecraft:z/task", Some("minecraft:z/root")),
            ],
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        });

        assert_eq!(
            store.advancements().root_order,
            vec!["minecraft:z/root", "minecraft:y/root", "minecraft:a/root"]
        );
        assert_eq!(
            store.ensure_advancements_screen_selected_tab(),
            Some("minecraft:y/root".to_string())
        );
        assert_eq!(store.selected_advancements_tab(), Some("minecraft:y/root"));
        assert_eq!(store.ensure_advancements_screen_selected_tab(), None);

        store.apply_select_advancements_tab(SelectAdvancementsTab {
            tab: Some("minecraft:z/task".to_string()),
        });
        assert_eq!(store.selected_advancements_tab(), Some("minecraft:z/task"));
        assert_eq!(
            store.ensure_advancements_screen_selected_tab(),
            Some("minecraft:y/root".to_string())
        );
    }

    #[test]
    fn advancement_root_tabs_skip_hidden_roots_and_stop_after_vanilla_capacity() {
        let mut added = vec![advancement("minecraft:hidden/root", None)];
        for index in 0..27 {
            added.push(displayed_advancement(
                &format!("minecraft:displayed/root_{index:02}"),
                None,
            ));
        }
        let mut store = WorldStore::new();
        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added,
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        });

        let tabs = store.advancement_root_tabs();
        assert_eq!(tabs.len(), MAX_ADVANCEMENT_ROOT_TABS);
        assert_eq!(tabs[0].id, "minecraft:displayed/root_00");
        assert_eq!(tabs[0].title, "minecraft:displayed/root_00");
        assert_eq!(tabs[0].display_index, 0);
        assert_eq!(tabs[25].id, "minecraft:displayed/root_25");
        assert_eq!(tabs[25].display_index, 25);
        assert!(!tabs.iter().any(|tab| tab.id == "minecraft:hidden/root"));
        assert!(!tabs
            .iter()
            .any(|tab| tab.id == "minecraft:displayed/root_26"));
    }

    #[test]
    fn select_advancements_root_tab_accepts_only_visible_display_roots() {
        let mut added = vec![
            advancement("minecraft:hidden/root", None),
            displayed_advancement("minecraft:visible/root", None),
            displayed_advancement("minecraft:visible/child", Some("minecraft:visible/root")),
        ];
        for index in 0..26 {
            added.push(displayed_advancement(
                &format!("minecraft:extra/root_{index:02}"),
                None,
            ));
        }
        let mut store = WorldStore::new();
        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added,
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        });

        assert_eq!(
            store.select_advancements_root_tab("minecraft:visible/root"),
            Some("minecraft:visible/root".to_string())
        );
        assert_eq!(
            store.selected_advancement_root_tab().map(|tab| tab.id),
            Some("minecraft:visible/root".to_string())
        );
        assert_eq!(
            store.select_advancements_root_tab("minecraft:hidden/root"),
            None
        );
        assert_eq!(
            store.select_advancements_root_tab("minecraft:visible/child"),
            None
        );
        assert_eq!(
            store.select_advancements_root_tab("minecraft:extra/root_25"),
            None
        );
    }

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
        assert_eq!(
            store.advancements().root_order,
            vec!["minecraft:story/root"]
        );

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
        assert_eq!(
            store.advancements().root_order,
            vec!["minecraft:nether/root"]
        );

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

    fn displayed_advancement(id: &str, parent: Option<&str>) -> AdvancementSummary {
        let mut advancement = advancement(id, parent);
        advancement.display = Some(AdvancementDisplaySummary {
            title: id.to_string(),
            description: String::new(),
            icon: AdvancementIconSummary {
                item_id: 1,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
            frame_type: AdvancementFrameType::Task,
            show_toast: false,
            hidden: false,
            background: None,
            x: 0.0,
            y: 0.0,
        });
        advancement
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
