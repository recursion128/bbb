#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugScreenEntryStatus {
    AlwaysOn,
    InOverlay,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugScreenProfile {
    Default,
    Performance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugScreenEntryId {
    ThreeDimensionalCrosshair,
    GameVersion,
    Fps,
    Tps,
    Memory,
    DetailedMemory,
    SystemSpecs,
    PlayerPosition,
    PlayerSectionPosition,
    DayCount,
    LightLevels,
    SimplePerformanceImpactors,
    EntityHitboxes,
    ChunkBorders,
    GpuUtilization,
}

impl DebugScreenEntryId {
    pub(crate) fn is_allowed(self, reduced_debug_info: bool) -> bool {
        !reduced_debug_info
            || matches!(
                self,
                Self::GameVersion
                    | Self::Fps
                    | Self::Tps
                    | Self::Memory
                    | Self::DetailedMemory
                    | Self::SystemSpecs
                    | Self::PlayerSectionPosition
                    | Self::SimplePerformanceImpactors
                    | Self::GpuUtilization
            )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DebugScreenEntryList {
    overlay_visible: bool,
    profile: Option<DebugScreenProfile>,
    statuses: Vec<(DebugScreenEntryId, DebugScreenEntryStatus)>,
}

impl Default for DebugScreenEntryList {
    fn default() -> Self {
        let mut entries = Self {
            overlay_visible: false,
            profile: None,
            statuses: Vec::new(),
        };
        entries.load_profile(DebugScreenProfile::Default);
        entries
    }
}

impl DebugScreenEntryList {
    pub(crate) fn is_overlay_visible(&self) -> bool {
        self.overlay_visible
    }

    pub(crate) fn set_overlay_visible(&mut self, visible: bool) {
        if self.overlay_visible != visible {
            self.overlay_visible = visible;
        }
    }

    pub(crate) fn toggle_overlay(&mut self) {
        self.set_overlay_visible(!self.overlay_visible);
    }

    pub(crate) fn load_profile(&mut self, profile: DebugScreenProfile) {
        self.profile = Some(profile);
        self.statuses.clear();
        self.statuses
            .extend_from_slice(debug_screen_profile_entries(profile));
    }

    #[cfg(test)]
    fn is_using_profile(&self, profile: DebugScreenProfile) -> bool {
        self.profile == Some(profile)
    }

    pub(crate) fn status(&self, entry: DebugScreenEntryId) -> DebugScreenEntryStatus {
        self.statuses
            .iter()
            .find_map(|(id, status)| (*id == entry).then_some(*status))
            .unwrap_or(DebugScreenEntryStatus::Never)
    }

    pub(crate) fn set_status(&mut self, entry: DebugScreenEntryId, status: DebugScreenEntryStatus) {
        self.profile = None;
        if let Some((_, existing)) = self.statuses.iter_mut().find(|(id, _)| *id == entry) {
            *existing = status;
        } else {
            self.statuses.push((entry, status));
        }
    }

    pub(crate) fn toggle_status(&mut self, entry: DebugScreenEntryId) -> bool {
        let enabled = match self.status(entry) {
            DebugScreenEntryStatus::AlwaysOn => {
                self.set_status(entry, DebugScreenEntryStatus::Never);
                false
            }
            DebugScreenEntryStatus::InOverlay => {
                if self.overlay_visible {
                    self.set_status(entry, DebugScreenEntryStatus::Never);
                    false
                } else {
                    self.set_status(entry, DebugScreenEntryStatus::AlwaysOn);
                    true
                }
            }
            DebugScreenEntryStatus::Never => {
                if self.overlay_visible {
                    self.set_status(entry, DebugScreenEntryStatus::InOverlay);
                } else {
                    self.set_status(entry, DebugScreenEntryStatus::AlwaysOn);
                }
                true
            }
        };
        enabled
    }

    pub(crate) fn is_currently_enabled(
        &self,
        entry: DebugScreenEntryId,
        reduced_debug_info: bool,
    ) -> bool {
        let enabled_by_status = self.status(entry) == DebugScreenEntryStatus::AlwaysOn
            || (self.overlay_visible && self.status(entry) == DebugScreenEntryStatus::InOverlay);
        enabled_by_status && entry.is_allowed(reduced_debug_info)
    }
}

fn debug_screen_profile_entries(
    profile: DebugScreenProfile,
) -> &'static [(DebugScreenEntryId, DebugScreenEntryStatus)] {
    match profile {
        DebugScreenProfile::Default => &[
            (
                DebugScreenEntryId::ThreeDimensionalCrosshair,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::GameVersion,
                DebugScreenEntryStatus::InOverlay,
            ),
            (DebugScreenEntryId::Tps, DebugScreenEntryStatus::InOverlay),
            (DebugScreenEntryId::Fps, DebugScreenEntryStatus::InOverlay),
            (
                DebugScreenEntryId::Memory,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::SystemSpecs,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::PlayerPosition,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::PlayerSectionPosition,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::SimplePerformanceImpactors,
                DebugScreenEntryStatus::InOverlay,
            ),
        ],
        DebugScreenProfile::Performance => &[
            (DebugScreenEntryId::Tps, DebugScreenEntryStatus::InOverlay),
            (DebugScreenEntryId::Fps, DebugScreenEntryStatus::AlwaysOn),
            (
                DebugScreenEntryId::GpuUtilization,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::Memory,
                DebugScreenEntryStatus::InOverlay,
            ),
            (
                DebugScreenEntryId::SimplePerformanceImpactors,
                DebugScreenEntryStatus::InOverlay,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_matches_vanilla_entry_statuses() {
        let entries = DebugScreenEntryList::default();

        for entry in [
            DebugScreenEntryId::ThreeDimensionalCrosshair,
            DebugScreenEntryId::GameVersion,
            DebugScreenEntryId::Tps,
            DebugScreenEntryId::Fps,
            DebugScreenEntryId::Memory,
            DebugScreenEntryId::SystemSpecs,
            DebugScreenEntryId::PlayerPosition,
            DebugScreenEntryId::PlayerSectionPosition,
            DebugScreenEntryId::SimplePerformanceImpactors,
        ] {
            assert_eq!(entries.status(entry), DebugScreenEntryStatus::InOverlay);
        }
        assert_eq!(
            entries.status(DebugScreenEntryId::EntityHitboxes),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::DetailedMemory),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::DayCount),
            DebugScreenEntryStatus::Never
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::LightLevels),
            DebugScreenEntryStatus::Never
        );
        assert!(entries.is_using_profile(DebugScreenProfile::Default));
    }

    #[test]
    fn performance_profile_keeps_fps_always_on() {
        let mut entries = DebugScreenEntryList::default();

        entries.load_profile(DebugScreenProfile::Performance);

        assert_eq!(
            entries.status(DebugScreenEntryId::Fps),
            DebugScreenEntryStatus::AlwaysOn
        );
        assert_eq!(
            entries.status(DebugScreenEntryId::GpuUtilization),
            DebugScreenEntryStatus::InOverlay
        );
        assert!(entries.is_using_profile(DebugScreenProfile::Performance));
    }

    #[test]
    fn toggle_status_follows_overlay_visibility() {
        let mut entries = DebugScreenEntryList::default();

        assert!(entries.toggle_status(DebugScreenEntryId::EntityHitboxes));
        assert_eq!(
            entries.status(DebugScreenEntryId::EntityHitboxes),
            DebugScreenEntryStatus::AlwaysOn
        );
        assert!(!entries.toggle_status(DebugScreenEntryId::EntityHitboxes));
        assert_eq!(
            entries.status(DebugScreenEntryId::EntityHitboxes),
            DebugScreenEntryStatus::Never
        );

        entries.set_overlay_visible(true);
        assert!(entries.toggle_status(DebugScreenEntryId::EntityHitboxes));
        assert_eq!(
            entries.status(DebugScreenEntryId::EntityHitboxes),
            DebugScreenEntryStatus::InOverlay
        );
        entries.set_overlay_visible(false);
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::EntityHitboxes, false));
    }

    #[test]
    fn reduced_debug_info_filters_disallowed_entries() {
        let mut entries = DebugScreenEntryList::default();
        entries.set_overlay_visible(true);

        assert!(entries.is_currently_enabled(DebugScreenEntryId::GameVersion, true));
        assert!(entries.is_currently_enabled(DebugScreenEntryId::PlayerSectionPosition, true));
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::PlayerPosition, true));
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::ThreeDimensionalCrosshair, true));
        entries.set_status(
            DebugScreenEntryId::DayCount,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::DayCount, true));
        entries.set_status(
            DebugScreenEntryId::DetailedMemory,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(entries.is_currently_enabled(DebugScreenEntryId::DetailedMemory, true));
        entries.set_status(
            DebugScreenEntryId::LightLevels,
            DebugScreenEntryStatus::AlwaysOn,
        );
        assert!(!entries.is_currently_enabled(DebugScreenEntryId::LightLevels, true));
    }
}
