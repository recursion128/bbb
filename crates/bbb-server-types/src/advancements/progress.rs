use std::collections::BTreeMap;

use bbb_protocol::JavaString;

use super::requirements::AdvancementRequirements;

/// `CriterionProgress` (`CriterionProgress.java:7-31`): the instant a criterion
/// was obtained, absent while it has not been.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CriterionProgress {
    obtained_epoch_millis: Option<i64>,
}

impl CriterionProgress {
    pub const fn obtained(epoch_millis: i64) -> Self {
        Self {
            obtained_epoch_millis: Some(epoch_millis),
        }
    }

    pub const fn is_done(&self) -> bool {
        self.obtained_epoch_millis.is_some()
    }

    /// `grant` (`:21-23`) stamps `Instant.now()`. The instant arrives from the
    /// caller here so a save this server writes is reproducible.
    pub const fn grant(&mut self, epoch_millis: i64) {
        self.obtained_epoch_millis = Some(epoch_millis);
    }

    /// `revoke` (`:25-27`).
    pub const fn revoke(&mut self) {
        self.obtained_epoch_millis = None;
    }

    pub const fn obtained_epoch_millis(&self) -> Option<i64> {
        self.obtained_epoch_millis
    }
}

/// `AdvancementProgress` (`AdvancementProgress.java:25-53`): one criterion
/// progress per requirement name, against the requirement grid that decides
/// whether they add up to done.
///
/// Vanilla's criteria map is a `HashMap`; this is ordered so the document it
/// writes is reproducible.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdvancementProgress {
    criteria: BTreeMap<JavaString, CriterionProgress>,
    requirements: AdvancementRequirements,
}

impl AdvancementProgress {
    /// The criteria half `AdvancementProgress.CODEC` (`:37-43`) builds, before
    /// `update` fits it to a live advancement.
    pub fn from_criteria(criteria: BTreeMap<JavaString, CriterionProgress>) -> Self {
        Self {
            criteria,
            requirements: AdvancementRequirements::default(),
        }
    }

    /// `update` (`:55-64`): drop what the advancement no longer names, add what
    /// it has gained, and adopt its requirement grid.
    pub fn update(&mut self, requirements: AdvancementRequirements) {
        let names = requirements.names();
        self.criteria.retain(|name, _| names.contains(name));
        for name in names {
            self.criteria.entry(name.clone()).or_default();
        }
        self.requirements = requirements;
    }

    pub fn requirements(&self) -> &AdvancementRequirements {
        &self.requirements
    }

    pub fn criteria(&self) -> &BTreeMap<JavaString, CriterionProgress> {
        &self.criteria
    }

    /// `isDone` (`:66-68`).
    pub fn is_done(&self) -> bool {
        self.requirements.test(|name| self.is_criterion_done(name))
    }

    /// `hasProgress` (`:70-78`), which decides whether the advancement reaches
    /// the save file at all (`PlayerAdvancements.java:157-165`).
    pub fn has_progress(&self) -> bool {
        self.criteria.values().any(CriterionProgress::is_done)
    }

    /// `grantProgress` (`:80-88`).
    pub fn grant_progress(&mut self, name: &JavaString, epoch_millis: i64) -> bool {
        match self.criteria.get_mut(name) {
            Some(progress) if !progress.is_done() => {
                progress.grant(epoch_millis);
                true
            }
            _ => false,
        }
    }

    /// `revokeProgress` (`:90-98`).
    pub fn revoke_progress(&mut self, name: &JavaString) -> bool {
        match self.criteria.get_mut(name) {
            Some(progress) if progress.is_done() => {
                progress.revoke();
                true
            }
            _ => false,
        }
    }

    /// `getCriterion` (`:114-116`).
    pub fn criterion(&self, name: &JavaString) -> Option<&CriterionProgress> {
        self.criteria.get(name)
    }

    fn is_criterion_done(&self, name: &JavaString) -> bool {
        self.criteria
            .get(name)
            .is_some_and(CriterionProgress::is_done)
    }

    /// `getPercent` (`:123-131`).
    pub fn percent(&self) -> f32 {
        if self.criteria.is_empty() {
            return 0.0;
        }
        self.count_completed_requirements() as f32 / self.requirements.size() as f32
    }

    /// `countCompletedRequirements` (`:147-149`).
    pub fn count_completed_requirements(&self) -> usize {
        self.requirements.count(|name| self.is_criterion_done(name))
    }

    /// `getRemainingCriteria` (`:151-161`).
    pub fn remaining_criteria(&self) -> Vec<&JavaString> {
        self.criteria
            .iter()
            .filter(|(_, progress)| !progress.is_done())
            .map(|(name, _)| name)
            .collect()
    }

    /// `getCompletedCriteria` (`:163-173`).
    pub fn completed_criteria(&self) -> Vec<&JavaString> {
        self.criteria
            .iter()
            .filter(|(_, progress)| progress.is_done())
            .map(|(name, _)| name)
            .collect()
    }
}
