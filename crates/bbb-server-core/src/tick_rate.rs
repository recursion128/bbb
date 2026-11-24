//! How fast the world ticks, and whether it ticks at all.
//!
//! `TickRateManager` (`TickRateManager.java:7-66`) and its dedicated-server
//! subclass `ServerTickRateManager` (`ServerTickRateManager.java:11-125`) are
//! the one owner of that answer. Every other tick decision reads it: the server
//! loop asks `checkShouldSprintThisTick` whether to skip its sleep
//! (`MinecraftServer.java:728-733`), and the level asks `runsNormally` whether
//! this tick is allowed to change the world at all
//! (`ServerLevel.java:343-344`).
//!
//! This module is the state machine only. Vanilla's copy also broadcasts two
//! packets from inside its mutators -- `updateStateToClients`
//! (`ServerTickRateManager.java:33-35`) and `updateStepTicks` (`:37-39`) -- and
//! sends the sprint report through a command source (`:89-91`). Those are the
//! caller's, because this crate keeps the mechanism separate from what it
//! makes the server say; the mutators here answer whether the state actually
//! changed so the caller knows when vanilla would have sent something.

use std::fmt::Write as _;

/// `TimeUtil.NANOSECONDS_PER_SECOND` (`TimeUtil.java:7`).
const NANOSECONDS_PER_SECOND: i64 = 1_000_000_000;
/// `TimeUtil.NANOSECONDS_PER_MILLISECOND` (`TimeUtil.java:8`).
const NANOSECONDS_PER_MILLISECOND: f64 = 1_000_000.0;
/// `TimeUtil.MILLISECONDS_PER_SECOND` (`TimeUtil.java:9`).
const MILLISECONDS_PER_SECOND: f64 = 1_000.0;

/// `String.format(Locale.ROOT, "%.{decimals}f", value)`.
///
/// Java rounds a tie away from zero (`RoundingMode.HALF_UP`, which is what
/// `Formatter` applies to `%f`) while Rust's `{:.N}` rounds a tie to even, so
/// `1.25` formats as `1.3` there and `1.2` here. Every string `/tick` reports is
/// built by that Java call, so the rounding has to be Java's: `f64::round`
/// already rounds half away from zero, which makes the scaled-integer form the
/// shortest way to say it.
pub fn java_fixed_decimals(value: f64, decimals: u32) -> String {
    let scale = 10f64.powi(decimals as i32);
    let scaled = (value * scale).round() as i64;
    let negative = scaled < 0;
    let magnitude = scaled.unsigned_abs();
    let divisor = scale as u64;
    let whole = magnitude / divisor;
    let fraction = magnitude % divisor;
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    if decimals == 0 {
        // `%.0f` writes no radix point at all, which is the form
        // `commands.worldborder.get` reports its width in.
        let _ = write!(out, "{whole}");
    } else {
        let _ = write!(out, "{whole}.{fraction:0width$}", width = decimals as usize);
    }
    out
}

/// `TickRateManager.MIN_TICKRATE` (`TickRateManager.java:8`).
pub const MIN_TICKRATE: f32 = 1.0;

/// The default the field carries before anything sets it
/// (`TickRateManager.java:9`).
pub const DEFAULT_TICKRATE: f32 = 20.0;

/// What `finishTickSprint` (`ServerTickRateManager.java:80-95`) reports when a
/// sprint ends.
///
/// Both numbers are measurements of how fast this process happened to run, so
/// no two servers can be expected to agree on them; a differential that
/// compares command feedback has to exclude this one message rather than
/// pretend it is deterministic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprintReport {
    /// The first argument of `commands.tick.sprint.report`, an `int`
    /// (`ServerTickRateManager.java:83`).
    pub ticks_per_second: i32,
    /// The second argument, already formatted the way vanilla formats it:
    /// `String.format(Locale.ROOT, "%.2f", ..)`
    /// (`ServerTickRateManager.java:84-86`).
    pub milliseconds_per_tick: String,
}

/// `ServerTickRateManager` (`ServerTickRateManager.java:11-125`), which is
/// `TickRateManager` (`TickRateManager.java:7-66`) plus the sprint bookkeeping
/// only a server does.
#[derive(Debug, Clone)]
pub struct TickRateManager {
    /// `TickRateManager.tickrate` (`TickRateManager.java:9`).
    tickrate: f32,
    /// `TickRateManager.nanosecondsPerTick` (`TickRateManager.java:10`).
    nanoseconds_per_tick: i64,
    /// `TickRateManager.frozenTicksToRun` (`TickRateManager.java:11`).
    frozen_ticks_to_run: i32,
    /// `TickRateManager.runGameElements` (`TickRateManager.java:12`).
    run_game_elements: bool,
    /// `TickRateManager.isFrozen` (`TickRateManager.java:13`).
    is_frozen: bool,
    /// `ServerTickRateManager.remainingSprintTicks`
    /// (`ServerTickRateManager.java:12`).
    remaining_sprint_ticks: i64,
    /// `ServerTickRateManager.sprintTickStartTime` (`:13`), in nanoseconds.
    sprint_tick_start_nanos: i64,
    /// `ServerTickRateManager.sprintTimeSpend` (`:14`), in nanoseconds.
    sprint_time_spend_nanos: i64,
    /// `ServerTickRateManager.scheduledCurrentSprintTicks` (`:15`).
    scheduled_current_sprint_ticks: i64,
    /// `ServerTickRateManager.previousIsFrozen` (`:16`).
    previous_is_frozen: bool,
}

impl Default for TickRateManager {
    fn default() -> Self {
        Self {
            tickrate: DEFAULT_TICKRATE,
            nanoseconds_per_tick: NANOSECONDS_PER_SECOND / 20,
            frozen_ticks_to_run: 0,
            run_game_elements: true,
            is_frozen: false,
            remaining_sprint_ticks: 0,
            sprint_tick_start_nanos: 0,
            sprint_time_spend_nanos: 0,
            scheduled_current_sprint_ticks: 0,
            previous_is_frozen: false,
        }
    }
}

impl TickRateManager {
    /// `TickRateManager.setTickRate` (`TickRateManager.java:15-18`), with the
    /// override's extra work (`ServerTickRateManager.java:115-119`) left to the
    /// caller: the rate reaches the clients through the same state packet a
    /// freeze does.
    pub fn set_tick_rate(&mut self, rate: f32) {
        self.tickrate = rate.max(MIN_TICKRATE);
        self.nanoseconds_per_tick =
            (NANOSECONDS_PER_SECOND as f64 / f64::from(self.tickrate)) as i64;
    }

    /// `TickRateManager.tickrate` (`TickRateManager.java:20-22`).
    pub fn tickrate(&self) -> f32 {
        self.tickrate
    }

    /// `TickRateManager.millisecondsPerTick` (`TickRateManager.java:24-26`).
    pub fn milliseconds_per_tick(&self) -> f32 {
        self.nanoseconds_per_tick as f32 / NANOSECONDS_PER_MILLISECOND as f32
    }

    /// `TickRateManager.nanosecondsPerTick` (`TickRateManager.java:28-30`).
    pub fn nanoseconds_per_tick(&self) -> i64 {
        self.nanoseconds_per_tick
    }

    /// `TickRateManager.runsNormally` (`TickRateManager.java:32-34`): whether
    /// this tick is allowed to change the world.
    pub fn runs_normally(&self) -> bool {
        self.run_game_elements
    }

    /// `TickRateManager.isSteppingForward` (`TickRateManager.java:36-38`).
    pub fn is_stepping_forward(&self) -> bool {
        self.frozen_ticks_to_run > 0
    }

    /// `TickRateManager.frozenTicksToRun` (`TickRateManager.java:44-46`).
    pub fn frozen_ticks_to_run(&self) -> i32 {
        self.frozen_ticks_to_run
    }

    /// `TickRateManager.isFrozen` (`TickRateManager.java:52-54`).
    pub fn is_frozen(&self) -> bool {
        self.is_frozen
    }

    /// `ServerTickRateManager.isSprinting` (`ServerTickRateManager.java:23-25`).
    pub fn is_sprinting(&self) -> bool {
        self.scheduled_current_sprint_ticks > 0
    }

    /// Whether `remainingSprintTicks` is still positive, the separate question
    /// `stopSprinting` (`ServerTickRateManager.java:62`) and
    /// `requestGameToSprint` (`:71`) each ask before deciding what to report.
    pub fn has_remaining_sprint_ticks(&self) -> bool {
        self.remaining_sprint_ticks > 0
    }

    /// `TickRateManager.setFrozen` (`TickRateManager.java:48-50`). The override
    /// (`ServerTickRateManager.java:28-31`) only adds the broadcast, which the
    /// caller owns.
    pub fn set_frozen(&mut self, state: bool) {
        self.is_frozen = state;
    }

    /// `TickRateManager.tick` (`TickRateManager.java:56-61`), run once per
    /// server tick before anything else reads [`Self::runs_normally`]
    /// (`MinecraftServer.java:975-977`).
    pub fn tick(&mut self) {
        self.run_game_elements = !self.is_frozen || self.frozen_ticks_to_run > 0;
        if self.frozen_ticks_to_run > 0 {
            self.frozen_ticks_to_run -= 1;
        }
    }

    /// `ServerTickRateManager.stepGameIfPaused`
    /// (`ServerTickRateManager.java:41-49`): stepping is only meaningful while
    /// frozen, and the `false` return is what `/tick step` turns into
    /// `commands.tick.step.fail`.
    pub fn step_game_if_paused(&mut self, ticks: i32) -> bool {
        if !self.is_frozen() {
            return false;
        }
        self.frozen_ticks_to_run = ticks;
        true
    }

    /// `ServerTickRateManager.stopStepping` (`ServerTickRateManager.java:51-59`).
    pub fn stop_stepping(&mut self) -> bool {
        if self.frozen_ticks_to_run > 0 {
            self.frozen_ticks_to_run = 0;
            true
        } else {
            false
        }
    }

    /// `ServerTickRateManager.requestGameToSprint`
    /// (`ServerTickRateManager.java:70-78`). The `true` return means a sprint
    /// was already running and has been replaced, which is what `/tick sprint`
    /// reports as `commands.tick.sprint.stop.success` before it reports the new
    /// sprint (`TickCommand.java:99-107`).
    ///
    /// Note the order vanilla writes these in: `previousIsFrozen` is captured
    /// before `setFrozen(false)`, so a sprint asked for while frozen returns to
    /// frozen when it finishes.
    pub fn request_game_to_sprint(&mut self, time: i32) -> bool {
        let interrupted = self.remaining_sprint_ticks > 0;
        self.sprint_time_spend_nanos = 0;
        self.scheduled_current_sprint_ticks = i64::from(time);
        self.remaining_sprint_ticks = i64::from(time);
        self.previous_is_frozen = self.is_frozen();
        self.set_frozen(false);
        interrupted
    }

    /// `ServerTickRateManager.stopSprinting` (`ServerTickRateManager.java:61-68`).
    ///
    /// The report is `Some` exactly when vanilla returned `true`, because
    /// vanilla's `true` arm is the one that runs `finishTickSprint` and so the
    /// one that sends `commands.tick.sprint.report`.
    pub fn stop_sprinting(&mut self) -> Option<SprintReport> {
        if self.remaining_sprint_ticks > 0 {
            Some(self.finish_tick_sprint())
        } else {
            None
        }
    }

    /// `ServerTickRateManager.finishTickSprint`
    /// (`ServerTickRateManager.java:80-95`).
    fn finish_tick_sprint(&mut self) -> SprintReport {
        let completed_ticks = self.scheduled_current_sprint_ticks - self.remaining_sprint_ticks;
        let milliseconds_to_complete =
            (self.sprint_time_spend_nanos as f64).max(1.0) / NANOSECONDS_PER_MILLISECOND;
        let ticks_per_second =
            (MILLISECONDS_PER_SECOND * completed_ticks as f64 / milliseconds_to_complete) as i32;
        let formatted = if completed_ticks == 0 {
            f64::from(self.milliseconds_per_tick())
        } else {
            milliseconds_to_complete / completed_ticks as f64
        };
        let milliseconds_per_tick = java_fixed_decimals(formatted, 2);
        self.scheduled_current_sprint_ticks = 0;
        self.sprint_time_spend_nanos = 0;
        self.remaining_sprint_ticks = 0;
        self.set_frozen(self.previous_is_frozen);
        SprintReport {
            ticks_per_second,
            milliseconds_per_tick,
        }
    }

    /// `ServerTickRateManager.checkShouldSprintThisTick`
    /// (`ServerTickRateManager.java:97-108`): whether the server loop should
    /// run this tick without waiting for the clock. The report is `Some` on the
    /// tick the sprint ran out, which is the tick vanilla sends
    /// `commands.tick.sprint.report` on.
    ///
    /// Only call this while [`Self::is_sprinting`] holds. Vanilla's own call
    /// site guards on it (`MinecraftServer.java:728`), and the guard is load
    /// bearing: the final arm finishes a sprint unconditionally, so calling it
    /// with no sprint scheduled would report one that never ran.
    pub fn check_should_sprint_this_tick(
        &mut self,
        now_nanos: i64,
    ) -> (bool, Option<SprintReport>) {
        if !self.run_game_elements {
            (false, None)
        } else if self.remaining_sprint_ticks > 0 {
            self.sprint_tick_start_nanos = now_nanos;
            self.remaining_sprint_ticks -= 1;
            (true, None)
        } else {
            (false, Some(self.finish_tick_sprint()))
        }
    }

    /// `ServerTickRateManager.endTickWork` (`ServerTickRateManager.java:110-112`).
    pub fn end_tick_work(&mut self, now_nanos: i64) {
        self.sprint_time_spend_nanos += now_nanos - self.sprint_tick_start_nanos;
    }
}

#[cfg(test)]
mod tests;
