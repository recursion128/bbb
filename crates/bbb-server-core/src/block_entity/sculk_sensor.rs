//! Sculk sensor listeners and activation.

use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_tick::vibration::WorldVibrationReads;
use bbb_protocol::block_types::VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use super::game_event::GameEventContext;
use super::value_input::int_or;
use super::vibration::{
    block_position_source, get_redstone_strength_for_distance, listener_handle_game_event,
    vibration_frequency, vibration_frequency_for_event, VibrationData, VibrationSystem,
    TAG_LISTENER,
};
use super::{entry, unique_field};
use crate::block_behavior::direction_property;
use crate::block_behavior::sculk_sensor as sensor_block;
use bbb_server_types::block_queries::block_type_of;
use bbb_server_types::directions::{opposite, relative};

/// `SculkSensorBlockEntity.DEFAULT_LAST_VIBRATION_FREQUENCY` (`:19`).
const DEFAULT_LAST_VIBRATION_FREQUENCY: i32 = 0;
const TAG_LAST_VIBRATION_FREQUENCY: &str = "last_vibration_frequency";
/// `SculkSensorBlockEntity.VibrationUser.LISTENER_RANGE` (`:77`).
const LISTENER_RANGE: i32 = 8;

pub(super) fn listener_radius(state: i32) -> i32 {
    if block_type_of(state) == Some(VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID) {
        16
    } else {
        LISTENER_RANGE
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SculkSensorBlockEntity {
    last_vibration_frequency: i32,
    /// `VibrationSystem.Data` (`:20`), saved under `listener` (`:51`).
    listener: VibrationData,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PendingActivation {
    power: i32,
    source: Option<Uuid>,
}

impl Default for SculkSensorBlockEntity {
    fn default() -> Self {
        Self {
            last_vibration_frequency: DEFAULT_LAST_VIBRATION_FREQUENCY,
            listener: VibrationData::default(),
        }
    }
}

impl SculkSensorBlockEntity {
    #[must_use]
    pub const fn last_vibration_frequency(&self) -> i32 {
        self.last_vibration_frequency
    }

    #[must_use]
    pub const fn vibration_data(&self) -> &VibrationData {
        &self.listener
    }

    #[cfg(test)]
    pub fn set_last_vibration_frequency(&mut self, last_vibration_frequency: i32) {
        self.last_vibration_frequency = last_vibration_frequency;
    }

    pub(crate) fn vibration_data_mut(&mut self) -> &mut VibrationData {
        &mut self.listener
    }

    /// Test-only: the vibration record, so a scenario can start mid-travel.
    #[cfg(test)]
    pub fn vibration_data_mut_for_test(&mut self) -> &mut VibrationData {
        &mut self.listener
    }

    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        vec![
            entry(
                TAG_LAST_VIBRATION_FREQUENCY,
                NbtSummaryValue::Int(self.last_vibration_frequency),
            ),
            entry(
                TAG_LISTENER,
                NbtSummaryValue::Compound(self.listener.save()),
            ),
        ]
    }

    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.last_vibration_frequency = int_or(
            record,
            TAG_LAST_VIBRATION_FREQUENCY,
            DEFAULT_LAST_VIBRATION_FREQUENCY,
        );
        self.listener = match unique_field(record, TAG_LISTENER) {
            Some(NbtSummaryValue::Compound(entries)) => {
                VibrationData::load(entries).unwrap_or_default()
            }
            _ => VibrationData::default(),
        };
    }

    pub(super) fn handle_game_event(
        &mut self,
        pos: BlockPos,
        level: &mut WorldVibrationReads<'_>,
        event: VanillaGameEvent,
        context: GameEventContext,
        source_position: Vec3d,
    ) -> bool {
        let Some(mut view) = SensorVibration::new(pos, self, level) else {
            return false;
        };
        listener_handle_game_event(&mut view, level, event, context, source_position)
    }

    pub(crate) fn tick_vibration(
        &mut self,
        pos: BlockPos,
        level: &mut WorldVibrationReads<'_>,
    ) -> Option<(PendingActivation, i32)> {
        let state = level.block_state(pos)?;
        let tick = self.listener.tick(
            level.game_time(),
            Some(block_position_source(pos)),
            level.adjacent_chunks_ticking(pos),
        );
        for particle in tick.particles.into_iter().flatten() {
            level.send_vibration_particle(particle.origin, particle.travel_time_in_ticks);
        }
        let mut activation = None;
        if let Some(arrival) = tick.arrival {
            if can_activate(level, pos) {
                self.last_vibration_frequency =
                    vibration_frequency_for_event(arrival.info.game_event);
                activation = Some((
                    PendingActivation {
                        power: get_redstone_strength_for_distance(
                            arrival.distance,
                            listener_radius(state),
                        ),
                        source: arrival
                            .info
                            .entity(|uuid| level.entity_identity_by_uuid(uuid))
                            .map(|entity| entity.uuid),
                    },
                    self.last_vibration_frequency,
                ));
            }
        }
        if tick.changed {
            level.mark_block_entity_changed(pos);
        }
        activation
    }

    pub(crate) fn activate(
        pos: BlockPos,
        level: &mut WorldVibrationReads<'_>,
        activation: PendingActivation,
        last_vibration_frequency: i32,
    ) {
        let Some(state) = level.block_state(pos) else {
            return;
        };
        if !sensor_block::can_activate(state) {
            return;
        }
        let Some(active) = sensor_block::active_state(state, activation.power) else {
            return;
        };
        level.set_block_state(pos, active, SET_BLOCK_UPDATE_ALL);
        level.schedule_block_tick(pos, sensor_block::active_ticks(state));
        if let Some(block_type_id) = block_type_of(state) {
            for pos in sensor_block::update_neighbour_positions(pos) {
                level.update_neighbours_at(pos, block_type_id);
            }
        }
        try_resonate_vibration(level, pos, last_vibration_frequency, activation.source);
        level.raise_game_event(
            VanillaGameEvent::SCULK_SENSOR_TENDRILS_CLICKING,
            pos,
            activation.source,
            None,
        );
        if !sensor_block::is_waterlogged(state) {
            let pitch = level.next_random_float() * 0.2 + 0.8;
            level.play_block_sound(pos, sensor_block::CLICKING_SOUND, 1.0, pitch);
        }
    }
}

/// `SculkSensorBlock.tryResonateVibration` (`SculkSensorBlock.java:232-242`).
fn try_resonate_vibration(
    level: &mut WorldVibrationReads<'_>,
    pos: BlockPos,
    frequency: i32,
    source: Option<Uuid>,
) {
    let Some(event) = VanillaGameEvent::resonate(frequency) else {
        return;
    };
    for direction in bbb_server_types::block_queries::DIRECTION_VALUES {
        let resonator = relative(pos, direction);
        let Some(state) = level.block_state(resonator) else {
            continue;
        };
        if level.block_state_in_tag(state, "minecraft:vibration_resonators") {
            level.raise_game_event(event, resonator, source, Some(state));
            level.play_block_sound(
                resonator,
                "minecraft:block.amethyst_block.resonate",
                1.0,
                resonance_pitch(frequency),
            );
        }
    }
}

/// `SculkSensorBlock.RESONANCE_PITCH_BEND` (`:52-58`) through
/// `NoteBlock.getPitchFromNote` (`NoteBlock.java:143-145`).
fn resonance_pitch(frequency: i32) -> f32 {
    const TONES: [i32; 16] = [0, 0, 2, 4, 6, 7, 9, 10, 12, 14, 15, 18, 19, 21, 22, 24];
    2.0_f64.powf(f64::from(TONES[frequency as usize] - 12) / 12.0) as f32
}

struct SensorVibration<'a> {
    pos: BlockPos,
    state: i32,
    sensor: &'a mut SculkSensorBlockEntity,
}

impl<'a> SensorVibration<'a> {
    fn new(
        pos: BlockPos,
        sensor: &'a mut SculkSensorBlockEntity,
        level: &WorldVibrationReads<'_>,
    ) -> Option<Self> {
        Some(Self {
            pos,
            state: level.block_state(pos)?,
            sensor,
        })
    }
}

impl VibrationSystem for SensorVibration<'_> {
    fn can_trigger_avoid_vibration(&self) -> bool {
        true
    }

    fn vibration_data(&self) -> &VibrationData {
        &self.sensor.listener
    }

    fn vibration_data_mut(&mut self) -> &mut VibrationData {
        &mut self.sensor.listener
    }

    fn vibration_listener_position(&self) -> Option<Vec3d> {
        Some(block_position_source(self.pos))
    }

    fn can_receive_vibration(
        &self,
        level: &mut WorldVibrationReads<'_>,
        pos: BlockPos,
        event: VanillaGameEvent,
        _context: GameEventContext,
    ) -> bool {
        let back_signal = calibrated_back_direction(self.state)
            .and_then(|back| level.signal(relative(self.pos, back), back));
        accepts_vibration(
            self.state,
            self.pos,
            pos,
            event,
            back_signal,
            level.block_state(self.pos),
        )
    }
}

pub(crate) fn calibrated_back_direction(state: i32) -> Option<bbb_protocol::packets::Direction> {
    if block_type_of(state) != Some(VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID) {
        return None;
    }
    direction_property(state, "facing").map(opposite)
}

pub(crate) fn accepts_vibration(
    state: i32,
    sensor_pos: BlockPos,
    event_pos: BlockPos,
    event: VanillaGameEvent,
    back_signal: Option<i32>,
    live_state: Option<i32>,
) -> bool {
    let frequency = vibration_frequency(event);
    if block_type_of(state) == Some(VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID) {
        if calibrated_back_direction(state).is_none() {
            return false;
        }
        let Some(signal) = back_signal else {
            return false;
        };
        if signal != 0 && signal != frequency {
            return false;
        }
    }
    if event_pos == sensor_pos
        && (event == VanillaGameEvent::BLOCK_DESTROY || event == VanillaGameEvent::BLOCK_PLACE)
    {
        return false;
    }
    frequency != 0 && live_state.is_some_and(sensor_block::can_activate)
}

fn can_activate(level: &WorldVibrationReads<'_>, pos: BlockPos) -> bool {
    level
        .block_state(pos)
        .is_some_and(sensor_block::can_activate)
}

#[cfg(test)]
mod tests {
    use super::{entry, TAG_LAST_VIBRATION_FREQUENCY, TAG_LISTENER};
    use crate::block_entity::vibration::{
        get_redstone_strength_for_distance, vibration_frequency_for_event, VibrationData,
        VibrationInfo,
    };
    use crate::block_entity::{new_block_entity, unique_field, BlockEntityPayload};
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_SCULK_SENSOR_ID;
    use bbb_protocol::packets::{BlockPos, Vec3d};
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    use uuid::Uuid;
    fn pos() -> BlockPos {
        BlockPos { x: 3, y: 4, z: 5 }
    }

    fn sculk_sensor() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_SCULK_SENSOR_ID)
            .expect("minecraft:sculk_sensor has a default state")
    }

    fn sample_data() -> VibrationData {
        let mut data = VibrationData::default();
        data.set_current_vibration(Some(VibrationInfo {
            game_event: "minecraft:block_change",
            distance: 2.5,
            pos: Vec3d {
                x: 0.5,
                y: 4.5,
                z: 5.5,
            },
            source: Some(Uuid::from_u128(9)),
            projectile_owner: None,
        }));
        data.set_travel_time_in_ticks(3);
        data
    }

    fn upsert(record: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
        if let Some(existing) = record.iter_mut().find(|entry| entry.name == name) {
            existing.value = value;
        } else {
            record.push(entry(name, value));
        }
    }

    #[test]
    fn sculk_sensor_save_load_round_trips_frequency_and_listener() {
        let state = sculk_sensor();
        let data = sample_data();
        let mut record = new_block_entity(pos(), state)
            .expect("a sculk sensor builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        upsert(
            &mut record,
            TAG_LAST_VIBRATION_FREQUENCY,
            NbtSummaryValue::Int(7),
        );
        upsert(
            &mut record,
            TAG_LISTENER,
            NbtSummaryValue::Compound(data.save()),
        );
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("sensor loads");
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(
            unique_field(&saved, TAG_LAST_VIBRATION_FREQUENCY),
            Some(&NbtSummaryValue::Int(7))
        );
        assert_eq!(
            unique_field(&saved, TAG_LISTENER),
            Some(&NbtSummaryValue::Compound(data.save())),
            "the listener compound keeps its NBT shape"
        );
        let sensor = match loaded.payload() {
            BlockEntityPayload::SculkSensor(payload) => payload,
            other => panic!("expected a sculk sensor payload, got {other:?}"),
        };
        assert_eq!(sensor.last_vibration_frequency(), 7);
        assert_eq!(sensor.vibration_data().travel_time_in_ticks(), 3);
        assert_eq!(
            sensor.vibration_data().current_vibration(),
            data.current_vibration(),
            "the listener compound decodes into the real VibrationSystem.Data"
        );
        assert!(loaded
            .update_tag_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds")
            .is_empty());
        assert!(
            crate::block_entity::block_entity_update_packet_with_resolver(
                &loaded,
                crate::block_entity::test_item_resolver()
            )
            .expect("fixture codec succeeds")
            .is_none()
        );
    }

    #[test]
    fn the_power_curve_and_the_frequency_rows_match_vanilla() {
        assert_eq!(get_redstone_strength_for_distance(0.0, 8), 15);
        assert_eq!(get_redstone_strength_for_distance(1.0, 8), 14);
        assert_eq!(get_redstone_strength_for_distance(2.0, 8), 12);
        assert_eq!(get_redstone_strength_for_distance(3.0, 8), 10);
        assert_eq!(get_redstone_strength_for_distance(4.0, 8), 8);
        assert_eq!(get_redstone_strength_for_distance(5.0, 8), 6);
        assert_eq!(get_redstone_strength_for_distance(6.0, 8), 4);
        assert_eq!(get_redstone_strength_for_distance(7.0, 8), 2);
        assert_eq!(
            get_redstone_strength_for_distance(8.0, 8),
            1,
            "at the listener radius the floor reaches 15, and the answer floors at 1"
        );
        assert_eq!(get_redstone_strength_for_distance(8.5, 8), 1);
        assert_eq!(get_redstone_strength_for_distance(40.0, 8), 1);

        assert_eq!(vibration_frequency_for_event("minecraft:step"), 1);
        assert_eq!(vibration_frequency_for_event("minecraft:block_change"), 11);
        assert_eq!(vibration_frequency_for_event("minecraft:block_place"), 13);
        assert_eq!(vibration_frequency_for_event("minecraft:block_destroy"), 12);
        assert_eq!(vibration_frequency_for_event("minecraft:entity_die"), 15);
    }
}
