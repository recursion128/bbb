//! `SculkShriekerBlockEntity` (`SculkShriekerBlockEntity.java`): the
//! `VibrationSystem` whose arrival escalates a player's warden warning and
//! shrieks.
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use crate::world::simulation_tick::vibration::WorldVibrationReads;
use bbb_protocol::block_state_types::{
    vanilla_block_state_property, vanilla_block_state_with_property,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use super::game_event::GameEventContext;
use super::value_input::int_or;
use super::vibration::{
    block_position_source, listener_handle_game_event, GameEventTag, VibrationData,
    VibrationSystem, TAG_LISTENER,
};
use super::{entry, unique_field};

/// `SculkShriekerBlockEntity.SHRIEKING_TICKS` (`:47`).
pub const SHRIEKING_TICKS: i32 = 90;
/// `LevelEvent.PARTICLES_SCULK_SHRIEK` (`LevelEvent.java:72`).
pub const LEVEL_EVENT_PARTICLES_SCULK_SHRIEK: i32 = 3007;
/// `Block.UPDATE_CLIENTS` (`Block.java:92`).
pub const UPDATE_CLIENTS: i32 = 2;
/// `SculkShriekerBlock.SHRIEKING` (`SculkShriekerBlock.java:35`).
const PROPERTY_SHRIEKING: &str = "shrieking";
/// `SculkShriekerBlock.CAN_SUMMON` (`SculkShriekerBlock.java:37`).
const PROPERTY_CAN_SUMMON: &str = "can_summon";
/// `SculkShriekerBlockEntity.DEFAULT_WARNING_LEVEL` (`:54`).
const DEFAULT_WARNING_LEVEL: i32 = 0;
const TAG_WARNING_LEVEL: &str = "warning_level";

/// `SculkShriekerBlockEntity` (`SculkShriekerBlockEntity.java`).
#[derive(Debug, Clone, PartialEq)]
pub struct SculkShriekerBlockEntity {
    warning_level: i32,
    /// `VibrationSystem.Data` (`:57`), saved under `listener` (`:85`).
    listener: VibrationData,
    /// Test-only `isValidVibration` override; see
    /// [`Self::set_listenable_events_for_test`].
    #[cfg(test)]
    listenable_events: Option<GameEventTag>,
}

impl Default for SculkShriekerBlockEntity {
    fn default() -> Self {
        Self {
            warning_level: DEFAULT_WARNING_LEVEL,
            listener: VibrationData::default(),
            #[cfg(test)]
            listenable_events: None,
        }
    }
}

impl SculkShriekerBlockEntity {
    #[must_use]
    pub const fn warning_level(&self) -> i32 {
        self.warning_level
    }

    #[must_use]
    pub const fn vibration_data(&self) -> &VibrationData {
        &self.listener
    }

    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        vec![
            entry(TAG_WARNING_LEVEL, NbtSummaryValue::Int(self.warning_level)),
            entry(
                TAG_LISTENER,
                NbtSummaryValue::Compound(self.listener.save()),
            ),
        ]
    }

    #[cfg(test)]
    pub fn set_warning_level_for_test(&mut self, level: i32) {
        self.warning_level = level;
    }

    /// Test-only: the vibration record, so a scenario can start mid-travel.
    #[cfg(test)]
    pub fn vibration_data_mut_for_test(&mut self) -> &mut VibrationData {
        &mut self.listener
    }

    /// Test-only: the tag `isValidVibration` filters on, so a scenario can
    /// deliver a row outside the production set. The production answer is
    /// always `shrieker_can_listen`, whose one row is
    /// `minecraft:sculk_sensor_tendrils_clicking`
    /// (`VanillaGameEvent::SCULK_SENSOR_TENDRILS_CLICKING`); see
    /// `the_shrieker_listens_only_to_the_shrieker_tag`.
    #[cfg(test)]
    pub fn set_listenable_events_for_test(&mut self, tag: GameEventTag) {
        self.listenable_events = Some(tag);
    }

    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.warning_level = int_or(record, TAG_WARNING_LEVEL, DEFAULT_WARNING_LEVEL);
        self.listener = match unique_field(record, TAG_LISTENER) {
            Some(NbtSummaryValue::Compound(entries)) => {
                VibrationData::load(entries).unwrap_or_default()
            }
            _ => VibrationData::default(),
        };
    }

    /// `VibrationSystem.Listener.handleGameEvent`
    /// (`VibrationSystem.java`) as this listener runs it: it schedules
    /// the vibration, and `tryShriek` (`SculkShriekerBlockEntity.java`)
    /// runs at the arrival.
    pub(super) fn handle_game_event(
        &mut self,
        pos: BlockPos,
        level: &mut WorldVibrationReads<'_>,
        event: VanillaGameEvent,
        context: GameEventContext,
        source_position: Vec3d,
    ) -> bool {
        let mut view = ShriekerVibration {
            pos,
            shrieker: self,
        };
        listener_handle_game_event(&mut view, level, event, context, source_position)
    }

    pub(crate) fn tick_vibration(
        &mut self,
        pos: BlockPos,
        level: &mut WorldVibrationReads<'_>,
    ) -> Option<Uuid> {
        let tick = self.listener.tick(
            level.game_time(),
            Some(block_position_source(pos)),
            level.adjacent_chunks_ticking(pos),
        );
        for particle in tick.particles.into_iter().flatten() {
            level.send_vibration_particle(particle.origin, particle.travel_time_in_ticks);
        }
        let player = tick.arrival.and_then(|arrival| {
            let source = arrival
                .info
                .entity(|uuid| level.entity_identity_by_uuid(uuid));
            let owner = arrival
                .info
                .projectile_owner(source, |uuid| level.entity_identity_by_uuid(uuid));
            owner
                .or(source)
                .and_then(|entity| entity.shriek_player_with(|uuid| level.is_online_player(uuid)))
        });
        if tick.changed {
            level.mark_block_entity_changed(pos);
        }
        player
    }

    pub(crate) fn set_warning_level(&mut self, warning_level: i32) {
        self.warning_level = warning_level;
    }

    pub(crate) fn try_shriek_in_place(
        pos: BlockPos,
        level: &mut LevelCtx<'_, '_>,
        player: Uuid,
        source_entity_id: Option<i32>,
        warden_nearby: Option<bool>,
    ) -> Option<()> {
        let state = level.sculk_shrieker_state(pos)?;
        if shrieking(state) {
            return Some(());
        }
        let can_respond = can_summon(state) && level.sculk_shrieker_can_respond()?;
        if can_respond {
            warden_nearby?;
        }
        let mut warning_level = DEFAULT_WARNING_LEVEL;
        if can_respond {
            let Some(warning) =
                level.try_sculk_shrieker_warning(pos, player, warden_nearby.unwrap())
            else {
                level.set_sculk_shrieker_warning_level(pos, DEFAULT_WARNING_LEVEL);
                return Some(());
            };
            warning_level = warning;
        }
        level.set_sculk_shrieker_warning_level(pos, warning_level);
        level.shriek(pos, state, source_entity_id);
        Some(())
    }
}

/// The `VibrationSystem.Listener`/`VibrationSystem.User` pair one shrieker
/// presents at one position (`SculkShriekerBlockEntity.java`).
struct ShriekerVibration<'a> {
    pos: BlockPos,
    shrieker: &'a mut SculkShriekerBlockEntity,
}

impl VibrationSystem for ShriekerVibration<'_> {
    fn vibration_data(&self) -> &VibrationData {
        &self.shrieker.listener
    }

    fn vibration_data_mut(&mut self) -> &mut VibrationData {
        &mut self.shrieker.listener
    }

    fn vibration_listener_position(&self) -> Option<Vec3d> {
        Some(block_position_source(self.pos))
    }

    fn vibration_listenable_events(&self) -> GameEventTag {
        #[cfg(test)]
        if let Some(tag) = self.shrieker.listenable_events {
            return tag;
        }
        GameEventTag::ShriekerCanListen
    }

    fn can_receive_vibration(
        &self,
        level: &mut WorldVibrationReads<'_>,
        _pos: BlockPos,
        _event: VanillaGameEvent,
        context: GameEventContext,
    ) -> bool {
        let not_shrieking = level
            .block_state(self.pos)
            .is_none_or(|state| !shrieking(state));
        let player = context
            .source_entity_id
            .and_then(|entity_id| level.entity_identity(entity_id))
            .and_then(|entity| entity.shriek_player_with(|uuid| level.is_online_player(uuid)));
        not_shrieking && player.is_some()
    }
}

#[must_use]
pub fn removal_response(
    shrieker: &SculkShriekerBlockEntity,
    old_state: i32,
) -> super::BlockEntityRemovalDrops {
    if !shrieking(old_state) {
        return super::BlockEntityRemovalDrops::None;
    }
    super::BlockEntityRemovalDrops::SculkShriekerResponse {
        warning_level: shrieker.warning_level,
        can_summon: can_summon(old_state),
    }
}

/// `SculkShriekerBlock.SHRIEKING` (`SculkShriekerBlock.java`).
#[must_use]
pub fn is_shrieking(state: i32) -> bool {
    property(state, PROPERTY_SHRIEKING) == Some("true")
}

fn shrieking(state: i32) -> bool {
    is_shrieking(state)
}

/// `SculkShriekerBlock.CAN_SUMMON` (`SculkShriekerBlock.java`).
fn can_summon(state: i32) -> bool {
    property(state, PROPERTY_CAN_SUMMON) == Some("true")
}

fn property(state: i32, name: &str) -> Option<&'static str> {
    vanilla_block_state_property(state, name)
}

/// `state.setValue(SculkShriekerBlock.SHRIEKING, shrieking)`
/// (`SculkShriekerBlockEntity.java`, `SculkShriekerBlock.java`),
/// resolved back to a global state id.
#[must_use]
pub fn state_with_shrieking(state: i32, shrieking: bool) -> Option<i32> {
    state_with_property(
        state,
        PROPERTY_SHRIEKING,
        if shrieking { "true" } else { "false" },
    )
}

/// `state.setValue(property, value)` (`SculkShriekerBlockEntity.java`),
/// resolved back to a global state id.
fn state_with_property(state: i32, name: &str, value: &str) -> Option<i32> {
    vanilla_block_state_with_property(state, name, value)
}

#[cfg(test)]
mod tests;
