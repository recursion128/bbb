use bbb_protocol::packets::{
    InitializeBorder as ProtocolInitializeBorder, SetBorderCenter as ProtocolSetBorderCenter,
    SetBorderLerpSize as ProtocolSetBorderLerpSize, SetBorderSize as ProtocolSetBorderSize,
    SetBorderWarningDelay as ProtocolSetBorderWarningDelay,
    SetBorderWarningDistance as ProtocolSetBorderWarningDistance,
};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, WorldStore};

const DEFAULT_WORLD_BORDER_SIZE: f64 = 5.999997E7;
const DEFAULT_WORLD_BORDER_ABSOLUTE_MAX_SIZE: i32 = 29_999_984;
const DEFAULT_WORLD_BORDER_WARNING_BLOCKS: i32 = 5;
const DEFAULT_WORLD_BORDER_WARNING_TIME: i32 = 15;

/// Vanilla `Mth.lerp(delta, start, end) = start + delta * (end - start)`.
fn lerp(delta: f64, start: f64, end: f64) -> f64 {
    start + delta * (end - start)
}

/// Vanilla `BorderStatus` (`world/level/border/BorderStatus.java:3-6`):
/// `GROWING(4259712)`, `SHRINKING(16724016)`, `STATIONARY(2138367)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldBorderStatus {
    Growing,
    Shrinking,
    Stationary,
}

impl WorldBorderStatus {
    /// Vanilla `BorderStatus.getColor()` values (`BorderStatus.java:4-6`):
    /// GROWING = 4259712 = 0x40FF80, SHRINKING = 16724016 = 0xFF3030,
    /// STATIONARY = 2138367 = 0x20A0FF.
    pub fn color(self) -> u32 {
        match self {
            WorldBorderStatus::Growing => 0x40FF80,
            WorldBorderStatus::Shrinking => 0xFF3030,
            WorldBorderStatus::Stationary => 0x20A0FF,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorldBorderState {
    pub center_x: f64,
    pub center_z: f64,
    /// Lerp start size while moving (vanilla `MovingBorderExtent.from`),
    /// otherwise the static size (`StaticBorderExtent.size`).
    pub size: f64,
    pub lerp_target: f64,
    /// Remaining lerp ticks (vanilla `MovingBorderExtent.lerpProgress`,
    /// decremented once per client tick in `WorldBorder.tick()`,
    /// `WorldBorder.java:281-283`).
    pub lerp_time: i64,
    /// Total lerp duration in ticks (vanilla `MovingBorderExtent.lerpDuration`,
    /// `WorldBorder.java:340-349`).
    pub lerp_duration: i64,
    /// Vanilla `MovingBorderExtent.size`: the per-tick recomputed current size.
    pub current_size: f64,
    /// Vanilla `MovingBorderExtent.previousSize`: last tick's `current_size`,
    /// used for partial-tick interpolation (`WorldBorder.java:352-386`).
    pub previous_size: f64,
    pub absolute_max_size: i32,
    pub warning_blocks: i32,
    pub warning_time: i32,
}

impl Default for WorldBorderState {
    fn default() -> Self {
        Self {
            center_x: 0.0,
            center_z: 0.0,
            size: DEFAULT_WORLD_BORDER_SIZE,
            lerp_target: DEFAULT_WORLD_BORDER_SIZE,
            lerp_time: 0,
            lerp_duration: 0,
            current_size: DEFAULT_WORLD_BORDER_SIZE,
            previous_size: DEFAULT_WORLD_BORDER_SIZE,
            absolute_max_size: DEFAULT_WORLD_BORDER_ABSOLUTE_MAX_SIZE,
            warning_blocks: DEFAULT_WORLD_BORDER_WARNING_BLOCKS,
            warning_time: DEFAULT_WORLD_BORDER_WARNING_TIME,
        }
    }
}

impl WorldBorderState {
    /// Vanilla `WorldBorder.setSize` -> `new StaticBorderExtent(size)`
    /// (`WorldBorder.java:186-188`).
    fn set_size(&mut self, size: f64) {
        self.size = size;
        self.lerp_target = size;
        self.lerp_time = 0;
        self.lerp_duration = 0;
        self.current_size = size;
        self.previous_size = size;
    }

    /// Vanilla `WorldBorder.lerpSizeBetween(from, to, ticks, gameTime)`
    /// (`WorldBorder.java:195-197`): `from == to` collapses to a
    /// `StaticBorderExtent`, otherwise a `MovingBorderExtent` whose
    /// constructor seeds `size = previousSize = calculateSize()`
    /// (`WorldBorder.java:340-349`).
    fn lerp_size_between(&mut self, old_size: f64, new_size: f64, lerp_time: i64) {
        if old_size == new_size {
            self.set_size(new_size);
            return;
        }
        self.size = old_size;
        self.lerp_target = new_size;
        self.lerp_time = lerp_time;
        self.lerp_duration = lerp_time;
        let seeded = self.calculate_size();
        self.current_size = seeded;
        self.previous_size = seeded;
    }

    /// Vanilla `MovingBorderExtent.calculateSize()` (`WorldBorder.java:397-400`):
    /// `progress = (lerpDuration - lerpProgress) / lerpDuration`,
    /// `progress < 1.0 ? Mth.lerp(progress, from, to) : to`.
    fn calculate_size(&self) -> f64 {
        let progress = (self.lerp_duration - self.lerp_time) as f64 / self.lerp_duration as f64;
        if progress < 1.0 {
            lerp(progress, self.size, self.lerp_target)
        } else {
            self.lerp_target
        }
    }

    /// One client tick, mirroring `WorldBorder.tick()` -> `extent.update()`
    /// (`WorldBorder.java:281-283`): `StaticBorderExtent.update` is a no-op
    /// (`WorldBorder.java:582-584`); `MovingBorderExtent.update`
    /// (`WorldBorder.java:431-441`) decrements `lerpProgress`, rolls
    /// `previousSize = size; size = calculateSize()`, and collapses into
    /// `new StaticBorderExtent(to)` once `lerpProgress <= 0`.
    fn tick(&mut self) {
        if self.lerp_time <= 0 {
            return;
        }
        self.lerp_time -= 1;
        self.previous_size = self.current_size;
        self.current_size = self.calculate_size();
        if self.lerp_time <= 0 {
            self.set_size(self.lerp_target);
        }
    }

    /// Vanilla `MovingBorderExtent.getStatus()` (`WorldBorder.java:418-420`):
    /// `to < from ? SHRINKING : GROWING`; `StaticBorderExtent.getStatus()`
    /// (`WorldBorder.java:533-535`) is `STATIONARY`.
    pub fn status(&self) -> WorldBorderStatus {
        if self.lerp_time > 0 {
            if self.lerp_target < self.size {
                WorldBorderStatus::Shrinking
            } else {
                WorldBorderStatus::Growing
            }
        } else {
            WorldBorderStatus::Stationary
        }
    }

    /// Vanilla `WorldBorder.getMinX(deltaPartialTick)`: `MovingBorderExtent`
    /// interpolates `center - Mth.lerp(pt, previousSize, size) / 2` clamped to
    /// `absoluteMaxSize` (`WorldBorder.java:353-359`); `StaticBorderExtent`
    /// clamps `center - size / 2` (`WorldBorder.java:552-556`), which the
    /// shared formula reproduces because `previous_size == current_size`.
    pub fn min_x_at(&self, partial_tick: f32) -> f64 {
        self.clamp_to_absolute_max(self.center_x - self.interpolated_size(partial_tick) / 2.0)
    }

    /// Vanilla `WorldBorder.getMinZ(deltaPartialTick)` (`WorldBorder.java:362-368`).
    pub fn min_z_at(&self, partial_tick: f32) -> f64 {
        self.clamp_to_absolute_max(self.center_z - self.interpolated_size(partial_tick) / 2.0)
    }

    /// Vanilla `WorldBorder.getMaxX(deltaPartialTick)` (`WorldBorder.java:371-377`).
    pub fn max_x_at(&self, partial_tick: f32) -> f64 {
        self.clamp_to_absolute_max(self.center_x + self.interpolated_size(partial_tick) / 2.0)
    }

    /// Vanilla `WorldBorder.getMaxZ(deltaPartialTick)` (`WorldBorder.java:380-386`).
    pub fn max_z_at(&self, partial_tick: f32) -> f64 {
        self.clamp_to_absolute_max(self.center_z + self.interpolated_size(partial_tick) / 2.0)
    }

    fn interpolated_size(&self, partial_tick: f32) -> f64 {
        lerp(
            f64::from(partial_tick),
            self.previous_size,
            self.current_size,
        )
    }

    /// Vanilla `WorldBorder.getDistanceToBorder(x, z)` (`WorldBorder.java:104-112`):
    /// the minimum of the four edge distances, using the partial-tick-0 bounds
    /// (`getMinX()` delegates to `extent.getMinX(0.0F)`, `WorldBorder.java:123-128`).
    pub fn distance_to_border(&self, x: f64, z: f64) -> f64 {
        let from_north = z - self.min_z_at(0.0);
        let from_south = self.max_z_at(0.0) - z;
        let from_west = x - self.min_x_at(0.0);
        let from_east = self.max_x_at(0.0) - x;
        from_west.min(from_east).min(from_north).min(from_south)
    }

    pub fn contains_block_pos(&self, pos: BlockPos) -> bool {
        self.contains_xz(f64::from(pos.x), f64::from(pos.z))
    }

    fn contains_xz(&self, x: f64, z: f64) -> bool {
        x >= self.min_x() && x < self.max_x() && z >= self.min_z() && z < self.max_z()
    }

    // Vanilla `WorldBorder.getMinX()` etc. delegate to `extent.getMinX(0.0F)`
    // (`WorldBorder.java:123-152`), i.e. the partial-tick-0 interpolated bounds.
    fn min_x(&self) -> f64 {
        self.min_x_at(0.0)
    }

    fn min_z(&self) -> f64 {
        self.min_z_at(0.0)
    }

    fn max_x(&self) -> f64 {
        self.max_x_at(0.0)
    }

    fn max_z(&self) -> f64 {
        self.max_z_at(0.0)
    }

    fn clamp_to_absolute_max(&self, value: f64) -> f64 {
        let absolute_max = f64::from(self.absolute_max_size);
        value.clamp(-absolute_max, absolute_max)
    }
}

impl WorldStore {
    pub fn apply_initialize_border(&mut self, packet: ProtocolInitializeBorder) {
        self.counters.world_border_initializes_received += 1;
        self.world_border.center_x = packet.new_center_x;
        self.world_border.center_z = packet.new_center_z;
        if packet.lerp_time > 0 {
            self.world_border
                .lerp_size_between(packet.old_size, packet.new_size, packet.lerp_time);
        } else {
            self.world_border.set_size(packet.new_size);
        }
        self.world_border.absolute_max_size = packet.new_absolute_max_size;
        self.world_border.warning_blocks = packet.warning_blocks;
        self.world_border.warning_time = packet.warning_time;
    }

    pub fn apply_set_border_center(&mut self, packet: ProtocolSetBorderCenter) {
        self.counters.world_border_center_updates_received += 1;
        self.world_border.center_x = packet.new_center_x;
        self.world_border.center_z = packet.new_center_z;
    }

    pub fn apply_set_border_lerp_size(&mut self, packet: ProtocolSetBorderLerpSize) {
        self.counters.world_border_lerp_size_updates_received += 1;
        self.world_border
            .lerp_size_between(packet.old_size, packet.new_size, packet.lerp_time);
    }

    pub fn apply_set_border_size(&mut self, packet: ProtocolSetBorderSize) {
        self.counters.world_border_size_updates_received += 1;
        self.world_border.set_size(packet.size);
    }

    pub fn apply_set_border_warning_delay(&mut self, packet: ProtocolSetBorderWarningDelay) {
        self.counters.world_border_warning_delay_updates_received += 1;
        self.world_border.warning_time = packet.warning_delay;
    }

    pub fn apply_set_border_warning_distance(&mut self, packet: ProtocolSetBorderWarningDistance) {
        self.counters.world_border_warning_distance_updates_received += 1;
        self.world_border.warning_blocks = packet.warning_blocks;
    }

    pub fn world_border(&self) -> &WorldBorderState {
        &self.world_border
    }

    /// Advances the client world border by `ticks` client ticks, mirroring
    /// vanilla `ClientLevel.tick` running `this.getWorldBorder().tick()` when
    /// the tick rate manager runs normally (`ClientLevel.java:276-281`).
    pub fn advance_world_border(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.world_border.tick();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_border_defaults_match_vanilla_client() {
        let store = WorldStore::new();
        let border = store.world_border();

        assert_eq!(border.center_x, 0.0);
        assert_eq!(border.center_z, 0.0);
        assert_eq!(border.size, DEFAULT_WORLD_BORDER_SIZE);
        assert_eq!(border.lerp_target, DEFAULT_WORLD_BORDER_SIZE);
        assert_eq!(border.lerp_time, 0);
        assert_eq!(border.lerp_duration, 0);
        assert_eq!(border.current_size, DEFAULT_WORLD_BORDER_SIZE);
        assert_eq!(border.previous_size, DEFAULT_WORLD_BORDER_SIZE);
        assert_eq!(border.status(), WorldBorderStatus::Stationary);
        assert_eq!(
            border.absolute_max_size,
            DEFAULT_WORLD_BORDER_ABSOLUTE_MAX_SIZE
        );
        assert_eq!(border.warning_blocks, DEFAULT_WORLD_BORDER_WARNING_BLOCKS);
        assert_eq!(border.warning_time, DEFAULT_WORLD_BORDER_WARNING_TIME);
    }

    #[test]
    fn initialize_border_without_lerp_sets_static_size() {
        let mut store = WorldStore::new();

        store.apply_initialize_border(ProtocolInitializeBorder {
            new_center_x: 10.0,
            new_center_z: -20.0,
            old_size: 100.0,
            new_size: 200.0,
            lerp_time: 0,
            new_absolute_max_size: 400,
            warning_blocks: 8,
            warning_time: 9,
        });

        let border = store.world_border();
        assert_eq!(border.center_x, 10.0);
        assert_eq!(border.center_z, -20.0);
        assert_eq!(border.size, 200.0);
        assert_eq!(border.lerp_target, 200.0);
        assert_eq!(border.lerp_time, 0);
        assert_eq!(border.lerp_duration, 0);
        assert_eq!(border.current_size, 200.0);
        assert_eq!(border.previous_size, 200.0);
        assert_eq!(border.status(), WorldBorderStatus::Stationary);
        assert_eq!(border.absolute_max_size, 400);
        assert_eq!(border.warning_blocks, 8);
        assert_eq!(border.warning_time, 9);
        assert_eq!(store.counters().world_border_initializes_received, 1);
    }

    #[test]
    fn initialize_border_with_lerp_records_old_new_and_time() {
        let mut store = WorldStore::new();

        store.apply_initialize_border(ProtocolInitializeBorder {
            new_center_x: 1.5,
            new_center_z: -2.5,
            old_size: 300.0,
            new_size: 150.0,
            lerp_time: 60,
            new_absolute_max_size: 500,
            warning_blocks: 6,
            warning_time: 7,
        });

        let border = store.world_border();
        assert_eq!(border.center_x, 1.5);
        assert_eq!(border.center_z, -2.5);
        assert_eq!(border.size, 300.0);
        assert_eq!(border.lerp_target, 150.0);
        assert_eq!(border.lerp_time, 60);
        assert_eq!(border.lerp_duration, 60);
        // Vanilla MovingBorderExtent constructor seeds size = previousSize =
        // calculateSize() = from while lerpProgress == lerpDuration.
        assert_eq!(border.current_size, 300.0);
        assert_eq!(border.previous_size, 300.0);
        assert_eq!(border.status(), WorldBorderStatus::Shrinking);
        assert_eq!(border.absolute_max_size, 500);
        assert_eq!(border.warning_blocks, 6);
        assert_eq!(border.warning_time, 7);
        assert_eq!(store.counters().world_border_initializes_received, 1);
    }

    #[test]
    fn border_incremental_updates_mutate_only_expected_fields() {
        let mut store = WorldStore::new();
        store.apply_initialize_border(ProtocolInitializeBorder {
            new_center_x: 1.0,
            new_center_z: 2.0,
            old_size: 100.0,
            new_size: 200.0,
            lerp_time: 30,
            new_absolute_max_size: 500,
            warning_blocks: 6,
            warning_time: 7,
        });

        let mut expected = *store.world_border();
        expected.center_x = 3.0;
        expected.center_z = 4.0;
        store.apply_set_border_center(ProtocolSetBorderCenter {
            new_center_x: 3.0,
            new_center_z: 4.0,
        });
        assert_eq!(*store.world_border(), expected);

        expected.size = 200.0;
        expected.lerp_target = 300.0;
        expected.lerp_time = 50;
        expected.lerp_duration = 50;
        expected.current_size = 200.0;
        expected.previous_size = 200.0;
        store.apply_set_border_lerp_size(ProtocolSetBorderLerpSize {
            old_size: 200.0,
            new_size: 300.0,
            lerp_time: 50,
        });
        assert_eq!(*store.world_border(), expected);

        expected.size = 250.0;
        expected.lerp_target = 250.0;
        expected.lerp_time = 0;
        expected.lerp_duration = 0;
        expected.current_size = 250.0;
        expected.previous_size = 250.0;
        store.apply_set_border_size(ProtocolSetBorderSize { size: 250.0 });
        assert_eq!(*store.world_border(), expected);

        expected.warning_time = 9;
        store.apply_set_border_warning_delay(ProtocolSetBorderWarningDelay { warning_delay: 9 });
        assert_eq!(*store.world_border(), expected);

        expected.warning_blocks = 8;
        store.apply_set_border_warning_distance(ProtocolSetBorderWarningDistance {
            warning_blocks: 8,
        });
        assert_eq!(*store.world_border(), expected);

        let counters = store.counters();
        assert_eq!(counters.world_border_initializes_received, 1);
        assert_eq!(counters.world_border_center_updates_received, 1);
        assert_eq!(counters.world_border_lerp_size_updates_received, 1);
        assert_eq!(counters.world_border_size_updates_received, 1);
        assert_eq!(counters.world_border_warning_delay_updates_received, 1);
        assert_eq!(counters.world_border_warning_distance_updates_received, 1);
    }

    #[test]
    fn world_border_contains_block_pos_uses_vanilla_min_inclusive_max_exclusive_bounds() {
        let mut store = WorldStore::new();
        store.apply_initialize_border(ProtocolInitializeBorder {
            new_center_x: 10.0,
            new_center_z: -20.0,
            old_size: 4.0,
            new_size: 4.0,
            lerp_time: 0,
            new_absolute_max_size: DEFAULT_WORLD_BORDER_ABSOLUTE_MAX_SIZE,
            warning_blocks: DEFAULT_WORLD_BORDER_WARNING_BLOCKS,
            warning_time: DEFAULT_WORLD_BORDER_WARNING_TIME,
        });

        let border = store.world_border();
        assert!(border.contains_block_pos(BlockPos {
            x: 8,
            y: -64,
            z: -22
        }));
        assert!(border.contains_block_pos(BlockPos {
            x: 11,
            y: 320,
            z: -19
        }));
        assert!(!border.contains_block_pos(BlockPos {
            x: 12,
            y: 0,
            z: -20
        }));
        assert!(!border.contains_block_pos(BlockPos {
            x: 10,
            y: 0,
            z: -18
        }));
        assert!(!border.contains_block_pos(BlockPos { x: 7, y: 0, z: -20 }));
        assert!(!border.contains_block_pos(BlockPos {
            x: 10,
            y: 0,
            z: -23
        }));
    }

    #[test]
    fn world_border_contains_block_pos_clamps_bounds_to_absolute_max_size() {
        let mut store = WorldStore::new();
        store.apply_initialize_border(ProtocolInitializeBorder {
            new_center_x: 0.0,
            new_center_z: 0.0,
            old_size: 100.0,
            new_size: 100.0,
            lerp_time: 0,
            new_absolute_max_size: 3,
            warning_blocks: DEFAULT_WORLD_BORDER_WARNING_BLOCKS,
            warning_time: DEFAULT_WORLD_BORDER_WARNING_TIME,
        });

        let border = store.world_border();
        assert!(border.contains_block_pos(BlockPos { x: -3, y: 0, z: 0 }));
        assert!(border.contains_block_pos(BlockPos { x: 2, y: 0, z: 0 }));
        assert!(!border.contains_block_pos(BlockPos { x: 3, y: 0, z: 0 }));
    }

    #[test]
    fn world_border_tick_follows_vanilla_moving_extent_update() {
        // Vanilla MovingBorderExtent.update (WorldBorder.java:431-441):
        // lerpProgress--, previousSize = size, size = calculateSize(), and a
        // collapse into StaticBorderExtent(to) once lerpProgress <= 0.
        let mut store = WorldStore::new();
        store.apply_set_border_lerp_size(ProtocolSetBorderLerpSize {
            old_size: 100.0,
            new_size: 200.0,
            lerp_time: 4,
        });
        assert_eq!(store.world_border().status(), WorldBorderStatus::Growing);

        store.advance_world_border(1);
        let border = store.world_border();
        // progress = (4 - 3) / 4 = 0.25 -> lerp(0.25, 100, 200) = 125.
        assert_eq!(border.previous_size, 100.0);
        assert_eq!(border.current_size, 125.0);
        assert_eq!(border.lerp_time, 3);

        store.advance_world_border(1);
        let border = store.world_border();
        assert_eq!(border.previous_size, 125.0);
        assert_eq!(border.current_size, 150.0);

        // Vanilla MovingBorderExtent.getMinX(pt) lerps between previousSize and
        // size at the frame partial tick (WorldBorder.java:353-359).
        assert_eq!(border.min_x_at(0.5), -(137.5 / 2.0));
        assert_eq!(border.max_x_at(0.5), 137.5 / 2.0);

        store.advance_world_border(2);
        let border = store.world_border();
        assert_eq!(border.lerp_time, 0);
        assert_eq!(border.size, 200.0);
        assert_eq!(border.current_size, 200.0);
        assert_eq!(border.previous_size, 200.0);
        assert_eq!(border.status(), WorldBorderStatus::Stationary);

        // Static extents ignore further ticks (WorldBorder.java:582-584).
        store.advance_world_border(3);
        assert_eq!(store.world_border().current_size, 200.0);
    }

    #[test]
    fn world_border_status_colors_match_vanilla_border_status() {
        // Vanilla BorderStatus colors (BorderStatus.java:4-6).
        assert_eq!(WorldBorderStatus::Growing.color(), 4_259_712);
        assert_eq!(WorldBorderStatus::Shrinking.color(), 16_724_016);
        assert_eq!(WorldBorderStatus::Stationary.color(), 2_138_367);

        let mut store = WorldStore::new();
        store.apply_set_border_lerp_size(ProtocolSetBorderLerpSize {
            old_size: 200.0,
            new_size: 100.0,
            lerp_time: 10,
        });
        assert_eq!(store.world_border().status(), WorldBorderStatus::Shrinking);

        // Vanilla WorldBorder.lerpSizeBetween collapses from == to into a
        // StaticBorderExtent (WorldBorder.java:195-197).
        store.apply_set_border_lerp_size(ProtocolSetBorderLerpSize {
            old_size: 100.0,
            new_size: 100.0,
            lerp_time: 10,
        });
        assert_eq!(store.world_border().status(), WorldBorderStatus::Stationary);
        assert_eq!(store.world_border().lerp_time, 0);
    }

    #[test]
    fn world_border_distance_to_border_is_minimum_edge_distance() {
        // Vanilla WorldBorder.getDistanceToBorder(x, z) (WorldBorder.java:104-112).
        let mut store = WorldStore::new();
        store.apply_initialize_border(ProtocolInitializeBorder {
            new_center_x: 0.0,
            new_center_z: 0.0,
            old_size: 100.0,
            new_size: 100.0,
            lerp_time: 0,
            new_absolute_max_size: DEFAULT_WORLD_BORDER_ABSOLUTE_MAX_SIZE,
            warning_blocks: DEFAULT_WORLD_BORDER_WARNING_BLOCKS,
            warning_time: DEFAULT_WORLD_BORDER_WARNING_TIME,
        });

        let border = store.world_border();
        // Bounds are [-50, 50]^2; the closest edge from (40, -10) is east: 10.
        assert_eq!(border.distance_to_border(40.0, -10.0), 10.0);
        // From (0, -45) the closest edge is north: 5.
        assert_eq!(border.distance_to_border(0.0, -45.0), 5.0);
    }
}
