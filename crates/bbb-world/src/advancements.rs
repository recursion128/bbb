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
    pub background: Option<String>,
    pub display_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancementWidgetSummary {
    pub id: String,
    pub parent_id: Option<String>,
    pub icon: ProtocolAdvancementIconSummary,
    pub frame_type: bbb_protocol::packets::AdvancementFrameType,
    pub x: i32,
    pub y: i32,
    pub hidden: bool,
    pub done: bool,
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

    pub fn selected_advancement_widgets(&self) -> Vec<AdvancementWidgetSummary> {
        let Some(tab) = self.selected_advancement_root_tab() else {
            return Vec::new();
        };
        let mut widgets = Vec::new();
        self.collect_advancement_widgets(&tab.id, &mut widgets);
        widgets
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
            background: display.background.clone(),
            display_index,
        })
    }

    fn collect_advancement_widgets(&self, id: &str, widgets: &mut Vec<AdvancementWidgetSummary>) {
        if let Some(widget) = self.advancement_widget_summary(id) {
            widgets.push(widget);
        }

        let children: Vec<String> = self
            .advancements
            .advancements
            .iter()
            .filter_map(|(child_id, advancement)| {
                (advancement.parent.as_deref() == Some(id)).then(|| child_id.clone())
            })
            .collect();
        for child in children {
            self.collect_advancement_widgets(&child, widgets);
        }
    }

    fn advancement_widget_summary(&self, id: &str) -> Option<AdvancementWidgetSummary> {
        let advancement = self.advancements.advancements.get(id)?;
        let display = advancement.display.as_ref()?;
        let done = self
            .advancements
            .progress
            .get(id)
            .is_some_and(|progress| advancement_progress_is_done(advancement, progress));
        Some(AdvancementWidgetSummary {
            id: id.to_string(),
            parent_id: self.first_visible_advancement_parent_id(advancement),
            icon: display.icon.clone(),
            frame_type: display.frame_type,
            x: (display.x * 28.0).floor() as i32,
            y: (display.y * 27.0).floor() as i32,
            hidden: display.hidden,
            done,
        })
    }

    fn first_visible_advancement_parent_id(
        &self,
        advancement: &ProtocolAdvancementSummary,
    ) -> Option<String> {
        let mut parent_id = advancement.parent.as_deref();
        while let Some(id) = parent_id {
            let parent = self.advancements.advancements.get(id)?;
            if parent.display.is_some() {
                return Some(id.to_string());
            }
            parent_id = parent.parent.as_deref();
        }
        None
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

fn advancement_progress_is_done(
    advancement: &ProtocolAdvancementSummary,
    progress: &ProtocolAdvancementProgressSummary,
) -> bool {
    if advancement.requirements.is_empty() {
        return false;
    }
    advancement.requirements.iter().all(|group| {
        group.iter().any(|name| {
            progress.criteria.iter().any(|criterion| {
                criterion.name == *name && criterion.obtained_epoch_millis.is_some()
            })
        })
    })
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
        let mut first_displayed = displayed_advancement("minecraft:displayed/root_00", None);
        first_displayed.display.as_mut().unwrap().background =
            Some("minecraft:gui/advancements/backgrounds/stone".to_string());
        added.push(first_displayed);
        for index in 0..27 {
            if index != 0 {
                added.push(displayed_advancement(
                    &format!("minecraft:displayed/root_{index:02}"),
                    None,
                ));
            }
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
        assert_eq!(
            tabs[0].background.as_deref(),
            Some("minecraft:gui/advancements/backgrounds/stone")
        );
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
    fn selected_advancement_widgets_project_display_geometry_and_done_state() {
        let mut root = displayed_advancement("minecraft:story/root", None);
        root.requirements = vec![vec!["root".to_string()]];
        let mut child =
            displayed_advancement("minecraft:story/mine_stone", Some("minecraft:story/root"));
        child.requirements = vec![
            vec!["has_stone".to_string(), "has_deepslate".to_string()],
            vec!["has_pickaxe".to_string()],
        ];
        let child_display = child.display.as_mut().unwrap();
        child_display.frame_type = AdvancementFrameType::Goal;
        child_display.x = 2.0;
        child_display.y = 1.0;
        let mut hidden_child =
            displayed_advancement("minecraft:story/hidden", Some("minecraft:story/root"));
        hidden_child.display.as_mut().unwrap().hidden = true;
        let mut grandchild = displayed_advancement(
            "minecraft:story/through_no_display",
            Some("minecraft:story/no_display"),
        );
        grandchild.display.as_mut().unwrap().x = 3.0;
        grandchild.display.as_mut().unwrap().y = 2.0;
        let mut store = WorldStore::new();
        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added: vec![
                root,
                child,
                hidden_child,
                advancement("minecraft:story/no_display", Some("minecraft:story/root")),
                grandchild,
            ],
            removed: Vec::new(),
            progress: vec![progress(
                "minecraft:story/mine_stone",
                vec![
                    ("has_stone", Some(1)),
                    ("has_deepslate", None),
                    ("has_pickaxe", Some(2)),
                ],
            )],
            show_advancements: false,
        });
        assert_eq!(
            store.ensure_advancements_screen_selected_tab(),
            Some("minecraft:story/root".to_string())
        );

        let widgets = store.selected_advancement_widgets();
        assert_eq!(widgets.len(), 4);
        assert_eq!(widgets[0].id, "minecraft:story/root");
        assert_eq!(widgets[0].parent_id, None);
        assert_eq!(widgets[0].x, 0);
        assert_eq!(widgets[0].y, 0);
        assert!(!widgets[0].done);
        assert_eq!(widgets[1].id, "minecraft:story/hidden");
        assert_eq!(
            widgets[1].parent_id.as_deref(),
            Some("minecraft:story/root")
        );
        assert!(widgets[1].hidden);
        assert!(!widgets[1].done);
        assert_eq!(widgets[2].id, "minecraft:story/mine_stone");
        assert_eq!(
            widgets[2].parent_id.as_deref(),
            Some("minecraft:story/root")
        );
        assert_eq!(widgets[2].frame_type, AdvancementFrameType::Goal);
        assert_eq!(widgets[2].x, 56);
        assert_eq!(widgets[2].y, 27);
        assert!(widgets[2].done);
        assert_eq!(widgets[3].id, "minecraft:story/through_no_display");
        assert_eq!(
            widgets[3].parent_id.as_deref(),
            Some("minecraft:story/root")
        );
        assert_eq!(widgets[3].x, 84);
        assert_eq!(widgets[3].y, 54);
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
