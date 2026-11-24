//! The client's single source of time.
//!
//! Normally this is the wall clock. In lockstep replay mode it is a virtual
//! clock that only moves when the replay controller steps the client, so every
//! thread sees the same deterministic timeline and a recorded session replays
//! to the same frames, packets and sounds.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        OnceLock,
    },
    time::Duration,
};

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::{Instant, SystemTime, SystemTimeError, UNIX_EPOCH};
#[cfg(target_arch = "wasm32")]
pub use web_time::{Instant, SystemTime, SystemTimeError, UNIX_EPOCH};

static VIRTUAL: AtomicBool = AtomicBool::new(false);
static VIRTUAL_NANOS: AtomicU64 = AtomicU64::new(0);
static ORIGIN: OnceLock<Instant> = OnceLock::new();

/// Virtual wall-clock origin, 2026-01-01T00:00:00Z.
const VIRTUAL_UNIX_ORIGIN_SECS: u64 = 1_767_225_600;

/// Switches the process to the virtual clock. Call once, before any other
/// thread reads time.
pub fn enable_virtual() {
    ORIGIN.get_or_init(Instant::now);
    VIRTUAL.store(true, Ordering::SeqCst);
}

pub fn is_virtual() -> bool {
    VIRTUAL.load(Ordering::Acquire)
}

/// Moves the virtual clock forward. No effect on the wall clock.
pub fn advance(by: Duration) {
    let nanos = u64::try_from(by.as_nanos()).unwrap_or(u64::MAX);
    let _ = VIRTUAL_NANOS.try_update(Ordering::AcqRel, Ordering::Acquire, |current| {
        Some(current.saturating_add(nanos))
    });
}

fn virtual_offset() -> Duration {
    Duration::from_nanos(VIRTUAL_NANOS.load(Ordering::Acquire))
}

pub fn now() -> Instant {
    if is_virtual() {
        *ORIGIN.get().expect("virtual clock origin") + virtual_offset()
    } else {
        Instant::now()
    }
}

pub fn system_now() -> SystemTime {
    if is_virtual() {
        UNIX_EPOCH + Duration::from_secs(VIRTUAL_UNIX_ORIGIN_SECS) + virtual_offset()
    } else {
        SystemTime::now()
    }
}

/// `elapsed()` measured on this clock.
pub trait Elapsed {
    type Output;
    fn clock_elapsed(&self) -> Self::Output;
}

impl Elapsed for Instant {
    type Output = Duration;
    fn clock_elapsed(&self) -> Duration {
        now().saturating_duration_since(*self)
    }
}

impl Elapsed for SystemTime {
    type Output = Result<Duration, SystemTimeError>;
    fn clock_elapsed(&self) -> Self::Output {
        system_now().duration_since(*self)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::*;

    static TEST_CLOCK_LOCK: Mutex<()> = Mutex::new(());

    fn lock_clock_test() -> MutexGuard<'static, ()> {
        TEST_CLOCK_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct VirtualClockTestGuard {
        _lock: MutexGuard<'static, ()>,
    }

    impl VirtualClockTestGuard {
        fn enter() -> Self {
            let lock = lock_clock_test();
            VIRTUAL_NANOS.store(0, Ordering::SeqCst);
            enable_virtual();
            Self { _lock: lock }
        }
    }

    impl Drop for VirtualClockTestGuard {
        fn drop(&mut self) {
            VIRTUAL.store(false, Ordering::SeqCst);
            VIRTUAL_NANOS.store(0, Ordering::SeqCst);
        }
    }

    #[test]
    fn virtual_clock_moves_only_when_advanced() {
        {
            let _guard = VirtualClockTestGuard::enter();
            let start = now();
            let wall = system_now();
            std::thread::sleep(Duration::from_millis(5));
            assert_eq!(now(), start);
            advance(Duration::from_millis(50));
            assert_eq!(now() - start, Duration::from_millis(50));
            assert_eq!(
                system_now().duration_since(wall).unwrap(),
                Duration::from_millis(50)
            );
            assert_eq!(start.clock_elapsed(), Duration::from_millis(50));
        }
        let _lock = lock_clock_test();
        assert!(!is_virtual());
    }

    #[test]
    fn virtual_clock_guard_restores_wall_mode_after_drop_or_unwind() {
        let unwound = std::panic::catch_unwind(|| {
            let _guard = VirtualClockTestGuard::enter();
            advance(Duration::from_millis(25));
            assert!(is_virtual());
            panic!("simulated test failure");
        });
        assert!(unwound.is_err());

        let _lock = lock_clock_test();
        assert!(!is_virtual());
        assert_eq!(VIRTUAL_NANOS.load(Ordering::Acquire), 0);
        let start = now();
        std::thread::sleep(Duration::from_millis(10));
        assert!(now() > start);
    }

    #[test]
    fn virtual_clock_advance_saturates_instead_of_wrapping() {
        let _guard = VirtualClockTestGuard::enter();
        advance(Duration::from_nanos(u64::MAX - 10));
        assert_eq!(VIRTUAL_NANOS.load(Ordering::Acquire), u64::MAX - 10);
        advance(Duration::from_nanos(100));
        assert_eq!(VIRTUAL_NANOS.load(Ordering::Acquire), u64::MAX);
    }
}

/// Progress counters the lockstep replay controller polls to decide that the
/// client has absorbed everything delivered so far: every clientbound frame
/// read and applied, every queued command applied, the resulting state on
/// screen, and no async render work left. Written by the net owner and the
/// client loop; never read by gameplay code.
pub mod replay {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    /// Frames the transport has handed to the event stream.
    pub static NET_FRAMES_HANDED_OUT: AtomicU64 = AtomicU64::new(0);
    /// Frames whose handling finished: updated when the event stream comes
    /// back for the next frame.
    pub static NET_FRAMES_READ: AtomicU64 = AtomicU64::new(0);
    /// Serverbound frames written to the socket.
    pub static NET_FRAMES_WRITTEN: AtomicU64 = AtomicU64::new(0);
    pub static NET_EVENTS_EMITTED: AtomicU64 = AtomicU64::new(0);
    pub static NET_EVENTS_DRAINED: AtomicU64 = AtomicU64::new(0);
    /// Commands queued for the client loop, counted after they are sent.
    pub static COMMANDS_QUEUED: AtomicU64 = AtomicU64::new(0);
    /// `COMMANDS_QUEUED` as the loop read it before draining and applying.
    pub static COMMANDS_APPLIED: AtomicU64 = AtomicU64::new(0);
    pub static CLIENT_TICKS: AtomicU64 = AtomicU64::new(0);
    pub static FRAMES_RENDERED: AtomicU64 = AtomicU64::new(0);
    /// False while the loop runs a simulation step.
    pub static SIMULATION_IDLE: AtomicBool = AtomicBool::new(false);
    /// Virtual time the loop saw when it last started a step; behind
    /// `virtual_nanos` while a `sim.step` has not been picked up yet.
    pub static SIMULATION_SEEN_NANOS: AtomicU64 = AtomicU64::new(0);
    /// Set by a step that changed state, cleared once a frame shows it.
    pub static STATE_UNRENDERED: AtomicBool = AtomicBool::new(false);
    pub static RENDER_WORK_PENDING: AtomicBool = AtomicBool::new(false);

    pub fn count(counter: &AtomicU64) {
        counter.fetch_add(1, Ordering::AcqRel);
    }

    pub fn get(counter: &AtomicU64) -> u64 {
        counter.load(Ordering::Acquire)
    }

    pub fn set(counter: &AtomicU64, value: u64) {
        counter.store(value, Ordering::Release);
    }

    pub fn flag(flag: &AtomicBool, value: bool) {
        flag.store(value, Ordering::Release);
    }

    /// Called by the loop after clearing `SIMULATION_IDLE` and before reading
    /// the clock for a step.
    pub fn observe_virtual_clock() {
        set(
            &SIMULATION_SEEN_NANOS,
            super::VIRTUAL_NANOS.load(Ordering::Acquire),
        );
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Status {
        pub virtual_nanos: u64,
        pub net_frames_read: u64,
        pub net_frames_written: u64,
        pub net_events_emitted: u64,
        pub net_events_drained: u64,
        pub commands_queued: u64,
        pub commands_applied: u64,
        pub client_ticks: u64,
        pub frames_rendered: u64,
        pub simulation_seen_nanos: u64,
        pub simulation_idle: bool,
        pub state_unrendered: bool,
        pub render_work_pending: bool,
    }

    impl Status {
        /// Every delivered event and queued command is applied and the latest
        /// state is on screen with no background render work left.
        pub fn quiescent(&self) -> bool {
            self.simulation_quiescent() && !self.render_work_pending && !self.state_unrendered
        }

        /// Every delivered event and queued command is applied; render work
        /// may still be running.
        pub fn simulation_quiescent(&self) -> bool {
            self.simulation_idle
                && self.simulation_seen_nanos == self.virtual_nanos
                && self.net_events_emitted == self.net_events_drained
                && self.commands_queued == self.commands_applied
        }
    }

    /// Loads follow the writers' release order (net read → emit → drain,
    /// queue → apply → idle → render) so a quiescent answer is never built
    /// from a stale earlier stage.
    pub fn status() -> Status {
        let net_frames_read = NET_FRAMES_READ.load(Ordering::Acquire);
        let net_frames_written = NET_FRAMES_WRITTEN.load(Ordering::Acquire);
        let net_events_emitted = NET_EVENTS_EMITTED.load(Ordering::Acquire);
        let commands_queued = COMMANDS_QUEUED.load(Ordering::Acquire);
        let net_events_drained = NET_EVENTS_DRAINED.load(Ordering::Acquire);
        let commands_applied = COMMANDS_APPLIED.load(Ordering::Acquire);
        let simulation_seen_nanos = SIMULATION_SEEN_NANOS.load(Ordering::Acquire);
        let simulation_idle = SIMULATION_IDLE.load(Ordering::Acquire);
        let client_ticks = CLIENT_TICKS.load(Ordering::Acquire);
        let state_unrendered = STATE_UNRENDERED.load(Ordering::Acquire);
        let render_work_pending = RENDER_WORK_PENDING.load(Ordering::Acquire);
        Status {
            // Last: the clock only moves forward, so a seen value equal to
            // this one was current for the whole read.
            virtual_nanos: super::VIRTUAL_NANOS.load(Ordering::Acquire),
            net_frames_read,
            net_frames_written,
            net_events_emitted,
            net_events_drained,
            commands_queued,
            commands_applied,
            client_ticks,
            frames_rendered: FRAMES_RENDERED.load(Ordering::Acquire),
            simulation_seen_nanos,
            simulation_idle,
            state_unrendered,
            render_work_pending,
        }
    }
}
