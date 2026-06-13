use bbb_protocol::packets::{
    InitializeBorder as ProtocolInitializeBorder, SetBorderCenter as ProtocolSetBorderCenter,
    SetBorderLerpSize as ProtocolSetBorderLerpSize, SetBorderSize as ProtocolSetBorderSize,
    SetBorderWarningDelay as ProtocolSetBorderWarningDelay,
    SetBorderWarningDistance as ProtocolSetBorderWarningDistance,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

const DEFAULT_WORLD_BORDER_SIZE: f64 = 5.999997E7;
const DEFAULT_WORLD_BORDER_ABSOLUTE_MAX_SIZE: i32 = 29_999_984;
const DEFAULT_WORLD_BORDER_WARNING_BLOCKS: i32 = 5;
const DEFAULT_WORLD_BORDER_WARNING_TIME: i32 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorldBorderState {
    pub center_x: f64,
    pub center_z: f64,
    pub size: f64,
    pub lerp_target: f64,
    pub lerp_time: i64,
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
            absolute_max_size: DEFAULT_WORLD_BORDER_ABSOLUTE_MAX_SIZE,
            warning_blocks: DEFAULT_WORLD_BORDER_WARNING_BLOCKS,
            warning_time: DEFAULT_WORLD_BORDER_WARNING_TIME,
        }
    }
}

impl WorldBorderState {
    fn set_size(&mut self, size: f64) {
        self.size = size;
        self.lerp_target = size;
        self.lerp_time = 0;
    }

    fn lerp_size_between(&mut self, old_size: f64, new_size: f64, lerp_time: i64) {
        self.size = old_size;
        self.lerp_target = new_size;
        self.lerp_time = lerp_time;
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
        store.apply_set_border_lerp_size(ProtocolSetBorderLerpSize {
            old_size: 200.0,
            new_size: 300.0,
            lerp_time: 50,
        });
        assert_eq!(*store.world_border(), expected);

        expected.size = 250.0;
        expected.lerp_target = 250.0;
        expected.lerp_time = 0;
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
}
