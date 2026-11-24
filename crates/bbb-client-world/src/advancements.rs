use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::{
    AdvancementCriterionProgressSummary as ProtocolAdvancementCriterionProgressSummary,
    AdvancementIconSummary as ProtocolAdvancementIconSummary,
    AdvancementProgressSummary as ProtocolAdvancementProgressSummary,
    AdvancementSummary as ProtocolAdvancementSummary,
    SelectAdvancementsTab as ProtocolSelectAdvancementsTab,
    UpdateAdvancements as ProtocolUpdateAdvancements,
};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

pub const MAX_ADVANCEMENT_ROOT_TABS: usize = 26;
const MAX_TRACKED_ADVANCEMENTS: usize = 65_536;

#[derive(Debug, Clone, Default)]
struct AdvancementChildrenIndex(FxHashMap<String, Vec<String>>);

impl PartialEq for AdvancementChildrenIndex {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientAdvancementsState {
    pub advancements: BTreeMap<String, ProtocolAdvancementSummary>,
    pub progress: BTreeMap<String, ProtocolAdvancementProgressSummary>,
    #[serde(default)]
    pub selected_tab: Option<String>,
    #[serde(default)]
    pub root_order: Vec<String>,
    #[serde(default)]
    pub node_order: Vec<String>,
    #[serde(skip)]
    children_by_parent: AdvancementChildrenIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancementRootTabSummary {
    pub id: String,
    pub title: bbb_protocol::RichTextComponent,
    pub icon: ProtocolAdvancementIconSummary,
    pub background: Option<String>,
    pub display_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancementWidgetSummary {
    pub id: String,
    pub parent_id: Option<String>,
    pub title: bbb_protocol::RichTextComponent,
    pub description: bbb_protocol::RichTextComponent,
    pub icon: ProtocolAdvancementIconSummary,
    pub frame_type: bbb_protocol::packets::AdvancementFrameType,
    pub x: i32,
    pub y: i32,
    pub hidden: bool,
    pub done: bool,
    pub progress_done: usize,
    pub progress_total: usize,
}

impl WorldStore {
    pub fn apply_select_advancements_tab(&mut self, packet: ProtocolSelectAdvancementsTab) {
        self.counters.select_advancements_tab_packets += 1;
        self.advancements.selected_tab = packet
            .tab
            .filter(|tab| self.advancements.advancements.contains_key(tab));
    }

    pub fn apply_update_advancements(&mut self, packet: ProtocolUpdateAdvancements) {
        let reset = packet.reset;
        let show_advancements = packet.show_advancements;
        self.counters.update_advancements_packets += 1;
        if !self.preflight_packet_components(&packet, "update advancements") {
            return;
        }
        if reset {
            self.counters.update_advancements_reset_packets += 1;
            self.advancements.advancements.clear();
            self.advancements.progress.clear();
            self.advancements.root_order.clear();
            self.advancements.node_order.clear();
            self.advancements.children_by_parent.0.clear();
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
                let toast_display = (!reset
                    && show_advancements
                    && advancement_progress_is_done(
                        self.advancements
                            .advancements
                            .get(&progress.id)
                            .expect("checked above"),
                        &progress,
                    ))
                .then(|| {
                    self.advancements
                        .advancements
                        .get(&progress.id)
                        .and_then(|advancement| advancement.display.as_ref())
                        .filter(|display| display.show_toast)
                        .cloned()
                })
                .flatten();
                let advancement_id = progress.id.clone();
                self.advancements
                    .progress
                    .insert(progress.id.clone(), progress);
                if let Some(display) = toast_display {
                    self.push_advancement_toast_event(advancement_id, display);
                }
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

    pub(crate) fn reset_advancements_for_new_listener(&mut self) {
        self.advancements = ClientAdvancementsState::default().into();
        self.refresh_advancement_counters();
    }

    fn ensure_advancement_children_index(&mut self) {
        let state = &mut *self.advancements;
        if state.children_by_parent.0.is_empty() && !state.advancements.is_empty() {
            for id in &state.node_order {
                if let Some(parent) = state
                    .advancements
                    .get(id)
                    .and_then(|adv| adv.parent.as_ref())
                {
                    state
                        .children_by_parent
                        .0
                        .entry(parent.clone())
                        .or_default()
                        .push(id.clone());
                }
            }
        }
    }

    fn remove_advancement_and_children(&mut self, id: &str) -> usize {
        if !self.advancements.advancements.contains_key(id) {
            return 0;
        }
        self.ensure_advancement_children_index();
        let mut stack = vec![id.to_string()];
        let mut visited = FxHashSet::default();
        let mut to_remove = Vec::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(children) = self.advancements.children_by_parent.0.get(&current) {
                for child_id in children {
                    if !visited.contains(child_id) {
                        stack.push(child_id.clone());
                    }
                }
            }
            to_remove.push(current);
        }

        let mut removed = 0;
        let mut removed_ids = FxHashSet::default();
        let mut removed_roots = FxHashSet::default();
        for target in to_remove {
            if let Some(removed_advancement) = self.advancements.advancements.remove(&target) {
                if let Some(parent) = &removed_advancement.parent {
                    if let Some(siblings) = self.advancements.children_by_parent.0.get_mut(parent) {
                        siblings.retain(|child| child != &target);
                        if siblings.is_empty() {
                            self.advancements.children_by_parent.0.remove(parent);
                        }
                    }
                } else {
                    removed_roots.insert(target.clone());
                }
                self.advancements.children_by_parent.0.remove(&target);
                self.advancements.progress.remove(&target);
                removed_ids.insert(target);
                removed += 1;
            }
        }
        if !removed_ids.is_empty() {
            self.advancements
                .node_order
                .retain(|node| !removed_ids.contains(node));
        }
        if !removed_roots.is_empty() {
            self.advancements
                .root_order
                .retain(|root| !removed_roots.contains(root));
        }
        removed
    }

    fn advancement_parent_creates_cycle(&self, child_id: &str, parent_id: &str) -> bool {
        // A brand-new node not yet in the map cannot be an ancestor of any
        // existing node, so cycle checks only matter when overwriting an
        // existing id with a new parent.
        if !self.advancements.advancements.contains_key(child_id) {
            return child_id == parent_id;
        }
        let mut cursor = Some(parent_id);
        let mut steps = 0usize;
        let limit = self.advancements.advancements.len().saturating_add(1);
        while let Some(current) = cursor {
            if current == child_id || steps > limit {
                return true;
            }
            steps += 1;
            cursor = self
                .advancements
                .advancements
                .get(current)
                .and_then(|advancement| advancement.parent.as_deref());
        }
        false
    }

    fn add_advancements_when_parents_are_known(
        &mut self,
        pending: Vec<ProtocolAdvancementSummary>,
    ) {
        self.ensure_advancement_children_index();
        let mut ready = std::collections::VecDeque::new();
        let mut waiting_by_parent: FxHashMap<String, Vec<ProtocolAdvancementSummary>> =
            FxHashMap::default();
        for advancement in pending {
            match advancement.parent.as_deref() {
                None => ready.push_back(advancement),
                Some(parent) if self.advancements.advancements.contains_key(parent) => {
                    ready.push_back(advancement);
                }
                Some(parent) => {
                    waiting_by_parent
                        .entry(parent.to_string())
                        .or_default()
                        .push(advancement);
                }
            }
        }

        while let Some(advancement) = ready.pop_front() {
            let is_existing = self.advancements.advancements.contains_key(&advancement.id);
            if !is_existing && self.advancements.advancements.len() >= MAX_TRACKED_ADVANCEMENTS {
                self.counters.advancements_adds_ignored += 1;
                continue;
            }
            if advancement.parent.as_deref().is_some_and(|parent| {
                self.advancement_parent_creates_cycle(&advancement.id, parent)
            }) {
                self.counters.advancements_adds_ignored += 1;
                continue;
            }
            let inserted_id = advancement.id.clone();
            self.insert_advancement(advancement);
            if let Some(children) = waiting_by_parent.remove(&inserted_id) {
                for child in children {
                    ready.push_back(child);
                }
            }
        }

        let remaining: usize = waiting_by_parent.values().map(Vec::len).sum();
        self.counters.advancements_adds_ignored += remaining;
    }

    fn insert_advancement(&mut self, advancement: ProtocolAdvancementSummary) {
        let id = advancement.id.clone();
        let new_parent = advancement.parent.clone();
        let is_root = new_parent.is_none();
        let previous = self
            .advancements
            .advancements
            .insert(id.clone(), advancement);
        if let Some(old_parent) = previous.as_ref().and_then(|adv| adv.parent.as_ref()) {
            if new_parent.as_ref() != Some(old_parent) {
                if let Some(siblings) = self.advancements.children_by_parent.0.get_mut(old_parent) {
                    siblings.retain(|child| child != &id);
                    if siblings.is_empty() {
                        self.advancements.children_by_parent.0.remove(old_parent);
                    }
                }
            }
        }
        if let Some(parent) = new_parent {
            let children = self
                .advancements
                .children_by_parent
                .0
                .entry(parent)
                .or_default();
            if previous.as_ref().and_then(|adv| adv.parent.as_deref()) != Some(id.as_str())
                && !children.iter().any(|child| child == &id)
            {
                children.push(id.clone());
            }
        }
        if previous.is_none() {
            self.advancements.node_order.push(id.clone());
        }
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

    fn collect_advancement_widgets(
        &self,
        root_id: &str,
        widgets: &mut Vec<AdvancementWidgetSummary>,
    ) {
        // Rebuild the adjacency map only when deserialized without the index.
        let rebuilt_children: Option<FxHashMap<&str, Vec<&str>>> =
            (self.advancements.children_by_parent.0.is_empty()
                && self.advancements.advancements.len() > 1)
                .then(|| {
                    let mut map: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
                    for node_id in &self.advancements.node_order {
                        if let Some(parent) = self
                            .advancements
                            .advancements
                            .get(node_id)
                            .and_then(|adv| adv.parent.as_deref())
                        {
                            map.entry(parent).or_default().push(node_id.as_str());
                        }
                    }
                    map
                });
        let mut visited: FxHashSet<&str> = FxHashSet::default();
        let mut stack: Vec<&str> = vec![root_id];
        while let Some(id) = stack.pop() {
            if !visited.insert(id) {
                continue;
            }
            if let Some(widget) = self.advancement_widget_summary(id) {
                widgets.push(widget);
            }
            if let Some(children) = self.advancements.children_by_parent.0.get(id) {
                for child in children.iter().rev() {
                    stack.push(child.as_str());
                }
            } else if let Some(children) = rebuilt_children.as_ref().and_then(|map| map.get(id)) {
                for child in children.iter().rev() {
                    stack.push(child);
                }
            }
        }
    }

    fn advancement_widget_summary(&self, id: &str) -> Option<AdvancementWidgetSummary> {
        let advancement = self.advancements.advancements.get(id)?;
        let display = advancement.display.as_ref()?;
        let progress = self.advancements.progress.get(id);
        let done =
            progress.is_some_and(|progress| advancement_progress_is_done(advancement, progress));
        let progress_done = progress
            .map(|progress| advancement_progress_completed_requirements(advancement, progress))
            .unwrap_or_default();
        let progress_total = progress
            .filter(|_| !advancement.requirements.is_empty())
            .map(|_| advancement.requirements.len())
            .unwrap_or_default();
        let safe_coord = |value: f32, scale: f64| -> i32 {
            let scaled = f64::from(value) * scale;
            if scaled.is_finite() {
                scaled.floor().clamp(-1_000_000.0, 1_000_000.0) as i32
            } else {
                0
            }
        };
        Some(AdvancementWidgetSummary {
            id: id.to_string(),
            parent_id: self.first_visible_advancement_parent_id(advancement),
            title: display.title.clone(),
            description: display.description.clone(),
            icon: display.icon.clone(),
            frame_type: display.frame_type,
            x: safe_coord(display.x, 28.0),
            y: safe_coord(display.y, 27.0),
            hidden: display.hidden,
            done,
            progress_done,
            progress_total,
        })
    }

    fn first_visible_advancement_parent_id(
        &self,
        advancement: &ProtocolAdvancementSummary,
    ) -> Option<String> {
        let mut visited = FxHashSet::default();
        visited.insert(advancement.id.as_str());
        let mut parent_id = advancement.parent.as_deref();
        while let Some(id) = parent_id {
            if !visited.insert(id) {
                return None;
            }
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
    let expected_names: BTreeSet<bbb_protocol::JavaString> = advancement
        .requirements
        .iter()
        .flat_map(|group| group.iter().cloned())
        .collect();

    progress
        .criteria
        .retain(|criterion| expected_names.contains(&criterion.name));
    let existing_names: BTreeSet<bbb_protocol::JavaString> = progress
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

fn advancement_progress_completed_requirements(
    advancement: &ProtocolAdvancementSummary,
    progress: &ProtocolAdvancementProgressSummary,
) -> usize {
    advancement
        .requirements
        .iter()
        .filter(|group| {
            group.iter().any(|name| {
                progress.criteria.iter().any(|criterion| {
                    criterion.name == *name && criterion.obtained_epoch_millis.is_some()
                })
            })
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        AdvancementCriterionProgressSummary, AdvancementDisplaySummary, AdvancementFrameType,
        AdvancementIconSummary, AdvancementProgressSummary, AdvancementSummary,
        DataComponentPatchSummary, SelectAdvancementsTab, UpdateAdvancements,
    };
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};

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
    fn configuration_reentry_rebuilds_advancement_manager_and_preserves_packet_counters() {
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

        store.clear_client_level();

        assert_eq!(
            store.client_advancements(),
            &ClientAdvancementsState::default()
        );
        assert_eq!(store.counters().update_advancements_packets, 1);
        assert_eq!(store.counters().update_advancements_reset_packets, 1);
        assert_eq!(store.counters().advancements_added_received, 1);
        assert_eq!(store.counters().select_advancements_tab_packets, 1);
        assert_eq!(store.counters().advancements_tracked, 0);
        assert_eq!(store.counters().advancement_roots_tracked, 0);
        assert_eq!(store.counters().advancement_progress_tracked, 0);
        assert_eq!(store.counters().advancement_progress_criteria_tracked, 0);
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
        assert_eq!(tabs[0].title.text(), "minecraft:displayed/root_00");
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
        root.requirements = vec![vec!["root".into()]];
        let mut child =
            displayed_advancement("minecraft:story/mine_stone", Some("minecraft:story/root"));
        child.requirements = vec![
            vec!["has_stone".into(), "has_deepslate".into()],
            vec!["has_pickaxe".into()],
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
        assert_eq!(widgets[1].id, "minecraft:story/mine_stone");
        assert_eq!(
            widgets[1].parent_id.as_deref(),
            Some("minecraft:story/root")
        );
        assert_eq!(widgets[1].title.text(), "minecraft:story/mine_stone");
        assert_eq!(widgets[1].description.text(), "Break stone");
        assert_eq!(widgets[1].frame_type, AdvancementFrameType::Goal);
        assert_eq!(widgets[1].x, 56);
        assert_eq!(widgets[1].y, 27);
        assert!(widgets[1].done);
        assert_eq!(widgets[1].progress_done, 2);
        assert_eq!(widgets[1].progress_total, 2);
        assert_eq!(widgets[2].id, "minecraft:story/hidden");
        assert_eq!(
            widgets[2].parent_id.as_deref(),
            Some("minecraft:story/root")
        );
        assert!(widgets[2].hidden);
        assert!(!widgets[2].done);
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
                    vec![vec!["mine_😀", "get_log"]],
                ),
                advancement("minecraft:story/mine_stone", Some("minecraft:story/root")),
                advancement("minecraft:orphan", Some("minecraft:missing_parent")),
            ],
            removed: Vec::new(),
            progress: vec![
                progress("minecraft:story/root", vec![("mine_😀", Some(10))]),
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
        assert!(store.advancements().progress["minecraft:story/root"]
            .criteria
            .iter()
            .any(|criterion| criterion.name == "mine_😀"
                && criterion.obtained_epoch_millis == Some(10)));
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

    #[test]
    fn update_advancements_retains_rich_display_and_packet_node_order() {
        let mut child_a = displayed_advancement("minecraft:story/a", Some("minecraft:story/root"));
        child_a.display.as_mut().unwrap().title = styled_component("Alpha", "gold", true);
        let child_b = displayed_advancement("minecraft:story/b", Some("minecraft:story/root"));
        let mut store = WorldStore::new();
        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added: vec![
                child_a,
                child_b,
                displayed_advancement("minecraft:story/root", None),
            ],
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        });
        store.ensure_advancements_screen_selected_tab();

        assert_eq!(
            store.advancements().node_order,
            vec![
                "minecraft:story/root",
                "minecraft:story/a",
                "minecraft:story/b"
            ]
        );
        let widgets = store.selected_advancement_widgets();
        assert_eq!(
            widgets
                .iter()
                .map(|widget| widget.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "minecraft:story/root",
                "minecraft:story/a",
                "minecraft:story/b"
            ]
        );
        assert_eq!(widgets[1].title.text(), "Alpha");
        assert_eq!(widgets[1].title.styled()[0].style.color, Some(0xFF_AA00));
        assert_eq!(widgets[1].title.styled()[0].style.bold, Some(true));
    }

    #[test]
    fn completed_progress_queues_each_vanilla_eligible_toast_once() {
        let mut toast = displayed_advancement("minecraft:story/toast", None);
        toast.requirements = vec![vec!["done".into()]];
        toast.display.as_mut().unwrap().show_toast = true;
        toast.display.as_mut().unwrap().title = styled_component("Toast", "green", false);
        let mut silent = displayed_advancement("minecraft:story/silent", None);
        silent.requirements = vec![vec!["done".into()]];
        let mut store = WorldStore::new();
        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added: vec![toast, silent],
            removed: Vec::new(),
            progress: vec![progress("minecraft:story/toast", vec![("done", Some(1))])],
            show_advancements: true,
        });
        assert!(store.client_toast_events().is_empty());

        for obtained in [2, 3] {
            store.apply_update_advancements(UpdateAdvancements {
                reset: false,
                added: Vec::new(),
                removed: Vec::new(),
                progress: vec![progress(
                    "minecraft:story/toast",
                    vec![("done", Some(obtained))],
                )],
                show_advancements: true,
            });
        }
        store.apply_update_advancements(UpdateAdvancements {
            reset: false,
            added: Vec::new(),
            removed: Vec::new(),
            progress: vec![
                progress("minecraft:story/toast", vec![("done", Some(4))]),
                progress("minecraft:story/silent", vec![("done", Some(4))]),
                progress("minecraft:story/unknown", vec![("done", Some(4))]),
            ],
            show_advancements: false,
        });

        let events = store.client_toast_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence, 0);
        assert_eq!(events[1].sequence, 1);
        let (advancement_id, display) = events[0].advancement().expect("advancement toast");
        assert_eq!(advancement_id, "minecraft:story/toast");
        assert_eq!(display.title.text(), "Toast");
        assert_eq!(display.title.styled()[0].style.color, Some(0x55_FF55));
        assert_eq!(store.counters().advancement_toast_events, 2);
        assert_eq!(store.counters().advancement_toast_events_dropped, 0);
    }

    fn advancement(id: &str, parent: Option<&str>) -> AdvancementSummary {
        advancement_with_requirements(id, parent, Vec::new())
    }

    fn displayed_advancement(id: &str, parent: Option<&str>) -> AdvancementSummary {
        let mut advancement = advancement(id, parent);
        advancement.display = Some(AdvancementDisplaySummary {
            title: bbb_protocol::RichTextComponent::plain(id),
            description: bbb_protocol::RichTextComponent::plain("Break stone"),
            icon: AdvancementIconSummary {
                item_id: 1,
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
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

    fn styled_component(text: &str, color: &str, bold: bool) -> RichTextComponent {
        RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::string(text),
            },
            NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string(color),
            },
            NbtSummaryEntry {
                name: "bold".into(),
                value: NbtSummaryValue::Byte(i8::from(bold)),
            },
        ]))
        .unwrap()
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
                .map(|group| group.into_iter().map(Into::into).collect())
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
                        name: name.into(),
                        obtained_epoch_millis,
                    },
                )
                .collect(),
        }
    }

    #[test]
    fn cyclic_parent_updates_and_removal_do_not_loop_or_overflow() {
        let mut store = WorldStore::new();
        // Establish root -> child, then attempt to re-parent root under child (cycle) and self-parent.
        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added: vec![
                advancement_with_requirements("test:root", None, vec![]),
                advancement_with_requirements("test:child", Some("test:root"), vec![]),
            ],
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        });
        store.apply_update_advancements(UpdateAdvancements {
            reset: false,
            added: vec![
                advancement_with_requirements("test:root", Some("test:child"), vec![]),
                advancement_with_requirements("test:self", Some("test:self"), vec![]),
            ],
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        });
        // Cyclic updates must be rejected rather than corrupting the tree.
        assert_eq!(
            store
                .advancements()
                .advancements
                .get("test:root")
                .unwrap()
                .parent,
            None
        );
        assert!(!store.advancements().advancements.contains_key("test:self"));

        // Even if a pre-existing cycle exists in state, removal must terminate cleanly.
        store
            .advancements
            .advancements
            .get_mut("test:root")
            .unwrap()
            .parent = Some("test:child".to_string());
        store.apply_update_advancements(UpdateAdvancements {
            reset: false,
            added: Vec::new(),
            removed: vec!["test:root".to_string()],
            progress: Vec::new(),
            show_advancements: false,
        });
        assert!(!store.advancements().advancements.contains_key("test:root"));
        assert!(!store.advancements().advancements.contains_key("test:child"));
    }
}
