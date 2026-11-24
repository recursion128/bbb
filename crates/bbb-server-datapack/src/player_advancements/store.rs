use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::JavaString;

use super::file::Java26PlayerAdvancements;
use super::progress::AdvancementProgress;
use super::tree::{AdvancementSummary, AdvancementTree};
use super::visibility::evaluate_visibility;

/// What `award` (`PlayerAdvancements.java:167-190`) moved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AwardOutcome {
    /// The criterion was not obtained and now is.
    pub granted: bool,
    /// The advancement was not done and now is: what `rewards().grant` and the
    /// `display.shouldAnnounceChat` broadcast (`:176-181`) hang off. Both are
    /// the caller's, since neither the reward surface nor the chat surface is
    /// reachable from here.
    pub completed: bool,
}

/// The five fields `ClientboundUpdateAdvancementsPacket` carries
/// (`ClientboundUpdateAdvancementsPacket.java:21-25`): whole advancements to
/// add, ids to remove and the progress of what stayed.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancementsUpdate {
    pub reset: bool,
    pub added: Vec<AdvancementSummary>,
    pub removed: Vec<String>,
    pub progress: Vec<(String, AdvancementProgress)>,
    pub show_advancements: bool,
}

/// `PlayerAdvancements` (`PlayerAdvancements.java:48-61`): one session's
/// progress map, visible set, changed set and roots left to re-evaluate.
///
/// Every advancement is an index into the [`AdvancementTree`] the store was
/// loaded against; vanilla keys the same maps on `AdvancementHolder`, whose
/// identity is its id (`AdvancementHolder.java:16-18`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerAdvancements {
    progress: BTreeMap<usize, AdvancementProgress>,
    visible: BTreeSet<usize>,
    progress_changed: BTreeSet<usize>,
    roots_to_update: BTreeSet<usize>,
    last_selected_tab: Option<usize>,
    is_first_packet: bool,
    /// This session's slice of every trigger's listener sets
    /// (`SimpleCriterionTrigger.java:20`), keyed by trigger id and holding the
    /// (advancement, criterion) each `CriterionTrigger.Listener` names. The
    /// fire sites that would run them belong to other subsystems.
    listeners: BTreeMap<String, BTreeSet<(usize, JavaString)>>,
}

impl PlayerAdvancements {
    /// A session whose save holds no advancement document (`:114`, the file is
    /// not a regular file).
    pub fn created(tree: &AdvancementTree) -> Self {
        Self::from_document(tree, None)
    }

    /// `load` (`:113-128`) over an already-read document.
    pub fn load(tree: &AdvancementTree, data: &Java26PlayerAdvancements) -> Self {
        Self::from_document(tree, Some(data))
    }

    fn from_document(tree: &AdvancementTree, data: Option<&Java26PlayerAdvancements>) -> Self {
        let mut store = Self {
            is_first_packet: true,
            ..Self::default()
        };
        if let Some(data) = data {
            store.apply_from(tree, data);
        }
        store.check_for_automatic_triggers(tree);
        for index in 0..tree.len() {
            store.register_listeners(tree, index);
        }
        store
    }

    /// `reload` (`:85-95`): drop every listener, progress and dirty set, forget
    /// the selected tab, and run `load` again over the new tree.
    pub fn reload(&mut self, tree: &AdvancementTree, data: Option<&Java26PlayerAdvancements>) {
        *self = Self::from_document(tree, data);
    }

    /// `checkForAutomaticTriggers` (`:103-111`) awards every advancement whose
    /// criteria map is empty. `award` can stamp nothing for one: an empty map
    /// holds no `""` criterion (`AdvancementProgress.java:80-88`), so the
    /// instant it would record is never used. The unconditional reward grant
    /// beside it (`:108`) is the reward surface's, not this store's.
    fn check_for_automatic_triggers(&mut self, tree: &AdvancementTree) {
        for index in 0..tree.len() {
            if tree.node(index).criteria().is_empty() {
                self.award(tree, index, &JavaString::default(), 0);
            }
        }
    }

    fn apply_from(&mut self, tree: &AdvancementTree, data: &Java26PlayerAdvancements) {
        for (id, progress) in &data.progress {
            let Some(index) = tree.index_of(id) else {
                continue;
            };
            self.start_progress(tree, index, progress.clone());
            self.progress_changed.insert(index);
            self.mark_for_visibility_update(tree, index);
        }
    }

    /// `asData` (`:157-165`): only advancements that have obtained something.
    pub fn to_data(&self, tree: &AdvancementTree) -> Java26PlayerAdvancements {
        Java26PlayerAdvancements {
            progress: self
                .progress
                .iter()
                .filter(|(_, progress)| progress.has_progress())
                .map(|(index, progress)| (tree.node(*index).id().to_string(), progress.clone()))
                .collect(),
        }
    }

    /// `award` (`:167-190`).
    pub fn award(
        &mut self,
        tree: &AdvancementTree,
        index: usize,
        criterion: &JavaString,
        epoch_millis: i64,
    ) -> AwardOutcome {
        let progress = self.get_or_start_progress(tree, index);
        let was_done = progress.is_done();
        let granted = progress.grant_progress(criterion, epoch_millis);
        let is_done = progress.is_done();
        if granted {
            self.unregister_listeners(tree, index);
            self.progress_changed.insert(index);
        }
        if !was_done && is_done {
            self.mark_for_visibility_update(tree, index);
        }
        AwardOutcome {
            granted,
            completed: granted && !was_done && is_done,
        }
    }

    /// `revoke` (`:192-207`).
    pub fn revoke(&mut self, tree: &AdvancementTree, index: usize, criterion: &JavaString) -> bool {
        let progress = self.get_or_start_progress(tree, index);
        let was_done = progress.is_done();
        let revoked = progress.revoke_progress(criterion);
        let is_done = progress.is_done();
        if revoked {
            self.register_listeners(tree, index);
            self.progress_changed.insert(index);
        }
        if was_done && !is_done {
            self.mark_for_visibility_update(tree, index);
        }
        revoked
    }

    /// `registerListeners(AdvancementHolder)` (`:216-226`): while the
    /// advancement is not done, every criterion not yet obtained listens on
    /// its trigger.
    fn register_listeners(&mut self, tree: &AdvancementTree, index: usize) {
        if self.get_or_start_progress(tree, index).is_done() {
            return;
        }
        let progress = &self.progress[&index];
        for (name, trigger) in tree.node(index).criteria() {
            if !matches!(progress.criterion(name), Some(criterion) if !criterion.is_done()) {
                continue;
            }
            self.listeners
                .entry(trigger.clone())
                .or_default()
                .insert((index, name.clone()));
        }
    }

    /// `unregisterListeners` (`:232-245`): an obtained criterion stops
    /// listening, and a done advancement takes the rest of its criteria with
    /// it. An emptied trigger set is dropped the way
    /// `SimpleCriterionTrigger.removePlayerListener` drops its player entry
    /// (`SimpleCriterionTrigger.java:28-36`).
    fn unregister_listeners(&mut self, tree: &AdvancementTree, index: usize) {
        self.get_or_start_progress(tree, index);
        let progress = &self.progress[&index];
        let advancement_done = progress.is_done();
        for (name, trigger) in tree.node(index).criteria() {
            let Some(criterion) = progress.criterion(name) else {
                continue;
            };
            if !(criterion.is_done() || advancement_done) {
                continue;
            }
            if let Some(listeners) = self.listeners.get_mut(trigger) {
                listeners.remove(&(index, name.clone()));
                if listeners.is_empty() {
                    self.listeners.remove(trigger);
                }
            }
        }
    }

    /// The (advancement, criterion) pairs listening on `trigger`, in place of
    /// the `CriterionTrigger.Listener` set a fire site would run
    /// (`SimpleCriterionTrigger.java:43-70`).
    pub fn trigger_listeners(
        &self,
        trigger: &str,
    ) -> impl Iterator<Item = (usize, &JavaString)> + '_ {
        self.listeners
            .get(trigger)
            .into_iter()
            .flatten()
            .map(|(index, name)| (*index, name))
    }

    pub fn is_listening(&self, trigger: &str, index: usize, criterion: &JavaString) -> bool {
        self.listeners
            .get(trigger)
            .is_some_and(|listeners| listeners.contains(&(index, criterion.clone())))
    }

    pub fn listened_triggers(&self) -> impl Iterator<Item = &str> + '_ {
        self.listeners.keys().map(String::as_str)
    }

    /// `markForVisibilityUpdate` (`:209-214`).
    fn mark_for_visibility_update(&mut self, tree: &AdvancementTree, index: usize) {
        self.roots_to_update.insert(tree.node(index).root());
    }

    /// `getOrStartProgress` (`:287-295`).
    pub fn get_or_start_progress(
        &mut self,
        tree: &AdvancementTree,
        index: usize,
    ) -> &mut AdvancementProgress {
        if !self.progress.contains_key(&index) {
            self.start_progress(tree, index, AdvancementProgress::default());
        }
        self.progress
            .get_mut(&index)
            .expect("the progress entry was just started")
    }

    /// `startProgress` (`:297-300`).
    fn start_progress(
        &mut self,
        tree: &AdvancementTree,
        index: usize,
        mut progress: AdvancementProgress,
    ) {
        progress.update(tree.node(index).requirements().clone());
        self.progress.insert(index, progress);
    }

    /// `setSelectedTab` (`:274-285`), answering the selection the client is
    /// owed a `ClientboundSelectAdvancementsTabPacket` for.
    pub fn set_selected_tab(
        &mut self,
        tree: &AdvancementTree,
        tab: Option<usize>,
    ) -> Option<Option<String>> {
        let old = self.last_selected_tab;
        self.last_selected_tab = tab.filter(|index| {
            let node = tree.node(*index);
            node.is_root() && node.display().is_some()
        });
        if old == self.last_selected_tab {
            return None;
        }
        Some(
            self.last_selected_tab
                .map(|index| tree.node(index).id().to_string()),
        )
    }

    pub fn selected_tab(&self) -> Option<usize> {
        self.last_selected_tab
    }

    pub fn progress(&self, index: usize) -> Option<&AdvancementProgress> {
        self.progress.get(&index)
    }

    pub fn is_visible(&self, index: usize) -> bool {
        self.visible.contains(&index)
    }

    /// `flushDirty` (`:247-272`): the update the session owes its client, and
    /// nothing when the three dirty sets have nothing to publish.
    pub fn flush_dirty(
        &mut self,
        tree: &AdvancementTree,
        show_advancements: bool,
    ) -> Option<AdvancementsUpdate> {
        let mut update = None;
        if self.is_first_packet
            || !self.roots_to_update.is_empty()
            || !self.progress_changed.is_empty()
        {
            let mut added = Vec::new();
            let mut removed = Vec::new();
            for root in std::mem::take(&mut self.roots_to_update) {
                self.update_tree_visibility(tree, root, &mut added, &mut removed);
            }
            let mut progress = Vec::new();
            for index in std::mem::take(&mut self.progress_changed) {
                if self.visible.contains(&index) {
                    progress.push((
                        tree.node(index).id().to_string(),
                        self.progress[&index].clone(),
                    ));
                }
            }
            if !progress.is_empty() || !added.is_empty() || !removed.is_empty() {
                update = Some(AdvancementsUpdate {
                    reset: self.is_first_packet,
                    added,
                    removed,
                    progress,
                    show_advancements,
                });
            }
        }
        self.is_first_packet = false;
        update
    }

    /// `updateTreeVisibility` (`:302-316`).
    fn update_tree_visibility(
        &mut self,
        tree: &AdvancementTree,
        root: usize,
        added: &mut Vec<AdvancementSummary>,
        removed: &mut Vec<String>,
    ) {
        // The done test is `getOrStartProgress` (`:303`), so the walk starts an
        // entry for every node it reaches; that entry is what puts a newly
        // visible advancement into `progressChanged` below (`:308-310`).
        let mut pending = vec![root];
        while let Some(index) = pending.pop() {
            self.get_or_start_progress(tree, index);
            pending.extend(tree.node(index).children());
        }
        let mut evaluated = Vec::new();
        let started = &self.progress;
        evaluate_visibility(
            tree,
            root,
            &mut |index| started[&index].is_done(),
            &mut |index, visible| evaluated.push((index, visible)),
        );
        for (index, visible) in evaluated {
            if visible {
                if self.visible.insert(index) {
                    added.push(tree.summary(index));
                    self.progress_changed.insert(index);
                }
            } else if self.visible.remove(&index) {
                removed.push(tree.node(index).id().to_string());
            }
        }
    }
}
