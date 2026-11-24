//! Per-player warden warning counters and their shared warning rules.

/// `WardenSpawnTracker.MAX_WARNING_LEVEL` (`:26`).
pub const MAX_WARNING_LEVEL: i32 = 4;
/// `WardenSpawnTracker.PLAYER_SEARCH_RADIUS` (`:27`).
pub const PLAYER_SEARCH_RADIUS: f64 = 16.0;
/// `WardenSpawnTracker.WARNING_CHECK_DIAMETER` (`:28`), the warden-search box
/// edge (`:95-98`).
pub const WARNING_CHECK_DIAMETER: f64 = 48.0;
/// `WardenSpawnTracker.DECREASE_WARNING_LEVEL_EVERY_INTERVAL` (`:29`).
pub const DECREASE_WARNING_LEVEL_EVERY_INTERVAL: i32 = 12_000;
/// `WardenSpawnTracker.WARNING_LEVEL_INCREASE_COOLDOWN` (`:30`).
pub const WARNING_LEVEL_INCREASE_COOLDOWN: i32 = 200;

/// `WardenSpawnTracker` (`WardenSpawnTracker.java:17-129`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WardenSpawnTracker {
    ticks_since_last_warning: i32,
    warning_level: i32,
    cooldown_ticks: i32,
}

impl WardenSpawnTracker {
    /// `WardenSpawnTracker(int, int, int)` (`:35-39`), the codec's constructor
    /// (`:18-25`).
    #[must_use]
    pub const fn new(
        ticks_since_last_warning: i32,
        warning_level: i32,
        cooldown_ticks: i32,
    ) -> Self {
        Self {
            ticks_since_last_warning,
            warning_level,
            cooldown_ticks,
        }
    }

    /// `WardenSpawnTracker.tick` (`:45-56`), driven from `ServerPlayer.tick`
    /// (`ServerPlayer.java:575`).
    pub fn tick(&mut self) {
        if self.ticks_since_last_warning >= DECREASE_WARNING_LEVEL_EVERY_INTERVAL {
            self.decrease_warning_level();
            self.ticks_since_last_warning = 0;
        } else {
            self.ticks_since_last_warning += 1;
        }
        if self.cooldown_ticks > 0 {
            self.cooldown_ticks -= 1;
        }
    }

    /// `WardenSpawnTracker.reset` (`:58-62`).
    pub fn reset(&mut self) {
        self.ticks_since_last_warning = 0;
        self.warning_level = 0;
        self.cooldown_ticks = 0;
    }

    pub(crate) fn warning_from_trackers(trackers: impl IntoIterator<Item = Self>) -> Option<Self> {
        let mut highest: Option<Self> = None;
        for tracker in trackers {
            if tracker.on_cooldown() {
                return None;
            }
            if highest
                .as_ref()
                .is_none_or(|current| tracker.warning_level() > current.warning_level())
            {
                highest = Some(tracker);
            }
        }
        let mut highest = highest?;
        highest.increase_warning_level();
        Some(highest)
    }

    /// `WardenSpawnTracker.onCooldown` (`:91-93`).
    #[must_use]
    pub const fn on_cooldown(self) -> bool {
        self.cooldown_ticks > 0
    }

    /// `WardenSpawnTracker.increaseWarningLevel` (`:105-111`).
    fn increase_warning_level(&mut self) {
        if !self.on_cooldown() {
            self.ticks_since_last_warning = 0;
            self.cooldown_ticks = WARNING_LEVEL_INCREASE_COOLDOWN;
            self.set_warning_level(self.warning_level + 1);
        }
    }

    /// `WardenSpawnTracker.decreaseWarningLevel` (`:113-115`).
    fn decrease_warning_level(&mut self) {
        self.set_warning_level(self.warning_level - 1);
    }

    /// `WardenSpawnTracker.setWarningLevel` (`:117-119`).
    pub fn set_warning_level(&mut self, warning_level: i32) {
        self.warning_level = warning_level.clamp(0, MAX_WARNING_LEVEL);
    }

    /// `WardenSpawnTracker.getWarningLevel` (`:121-123`).
    #[must_use]
    pub const fn warning_level(self) -> i32 {
        self.warning_level
    }

    /// The codec's first field (`:20`), which the playerdata document writes.
    #[must_use]
    pub const fn ticks_since_last_warning(self) -> i32 {
        self.ticks_since_last_warning
    }

    /// The codec's third field (`:22`).
    #[must_use]
    pub const fn cooldown_ticks(self) -> i32 {
        self.cooldown_ticks
    }
}

#[cfg(test)]
mod tests;
