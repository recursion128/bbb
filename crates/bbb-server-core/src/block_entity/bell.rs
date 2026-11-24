//! Bell ringing and raider resonance timers.

use bbb_protocol::packets::Direction;

/// `BellBlock.EVENT_BELL_RING` (`BellBlock.java:56`).
pub const EVENT_BELL_RING: i32 = 1;
/// `BellBlockEntity.DURATION` (`:22`).
pub const DURATION: i32 = 50;

/// `Direction.get3DDataValue` (`Direction.java:155-157`).
#[must_use]
pub const fn direction_3d_data_value(direction: Direction) -> i32 {
    match direction {
        Direction::Down => 0,
        Direction::Up => 1,
        Direction::North => 2,
        Direction::South => 3,
        Direction::West => 4,
        Direction::East => 5,
    }
}

/// `Direction.from3DDataValue` (`Direction.java:283-285`).
#[must_use]
pub const fn direction_from_3d_data_value(value: i32) -> Direction {
    match value {
        0 => Direction::Down,
        1 => Direction::Up,
        3 => Direction::South,
        4 => Direction::West,
        5 => Direction::East,
        _ => Direction::North,
    }
}

/// `BellBlockEntity` (`:21-40`).
#[derive(Debug, Clone, PartialEq)]
pub struct BellBlockEntity {
    ticks: i32,
    shaking: bool,
    click_direction: Direction,
    resonation_ticks: Option<i32>,
}

impl Default for BellBlockEntity {
    fn default() -> Self {
        Self {
            ticks: 0,
            shaking: false,
            click_direction: Direction::North,
            resonation_ticks: None,
        }
    }
}

impl BellBlockEntity {
    #[must_use]
    pub const fn shaking(&self) -> bool {
        self.shaking
    }

    #[must_use]
    pub const fn ticks(&self) -> i32 {
        self.ticks
    }

    #[must_use]
    pub const fn click_direction(&self) -> Direction {
        self.click_direction
    }

    /// `BellBlockEntity.onHit` (`:91-101`) without the `blockEvent` call.
    pub fn on_hit(&mut self, direction: Direction) {
        self.click_direction = direction;
        if self.shaking {
            self.ticks = 0;
        } else {
            self.shaking = true;
        }
    }

    /// `BellBlockEntity.triggerEvent` (`:42-54`) for `EVENT_BELL_RING`.
    pub fn trigger_event(&mut self, param_a: i32, param_b: i32) -> bool {
        if param_a != EVENT_BELL_RING {
            return false;
        }
        self.resonation_ticks = None;
        self.click_direction = direction_from_3d_data_value(param_b);
        self.ticks = 0;
        self.shaking = true;
        true
    }

    pub(crate) fn start_resonating(&mut self) {
        self.resonation_ticks = Some(0);
    }

    /// `BellBlockEntity.tick` (`:56-81`): 5-tick delay and 40-tick resonance.
    pub(crate) fn tick(&mut self) -> BellTick {
        if self.shaking {
            self.ticks += 1;
        }
        if self.ticks >= DURATION {
            self.shaking = false;
            self.ticks = 0;
        }
        if let Some(ticks) = &mut self.resonation_ticks {
            if *ticks < 40 {
                *ticks += 1;
                if *ticks == 40 {
                    return BellTick::Glow;
                }
            }
        } else if self.shaking && self.ticks >= 5 && self.ticks % 5 == 0 {
            // Bound searches while ringing without keeping stale entity snapshots.
            return BellTick::Search;
        }
        BellTick::None
    }
}

pub(crate) enum BellTick {
    None,
    Search,
    Glow,
}
