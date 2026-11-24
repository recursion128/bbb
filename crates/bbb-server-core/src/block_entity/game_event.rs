//! Block-entity and entity game-event listener registries and delivery.

use rustc_hash::FxHashMap;

use crate::level_event::{RecordedGameEvent, VanillaGameEvent};
use bbb_protocol::packets::{BlockPos, Vec3d};

use super::{
    chunk_of, sculk_catalyst, sculk_sensor, sculk_shrieker, BlockEntity, BlockEntityPayload,
    ChunkKey,
};
use crate::world::simulation_tick::vibration::WorldVibrationReads;

/// `GameEvent.DEFAULT_NOTIFICATION_RADIUS` (`GameEvent.java:76`), the radius
/// the one-argument `register` passes (`:83-85`).
pub const GAME_EVENT_DEFAULT_NOTIFICATION_RADIUS: i32 = 16;

/// `GameEvent.notificationRadius()` (`GameEvent.java:15`) for a row of the
/// generated `minecraft:game_event` catalog, keyed by its resource id
/// (`GameEvent.java:88`).
///
/// Every row is registered at [`GAME_EVENT_DEFAULT_NOTIFICATION_RADIUS`];
/// `minecraft:jukebox_play` and `minecraft:jukebox_stop_play` (`:46-47`) and
/// `minecraft:shriek` (`:55`) are the three that name their own.
#[must_use]
pub fn game_event_notification_radius(resource_id: &str) -> i32 {
    match resource_id {
        "minecraft:jukebox_play" | "minecraft:jukebox_stop_play" => 10,
        "minecraft:shriek" => 32,
        _ => GAME_EVENT_DEFAULT_NOTIFICATION_RADIUS,
    }
}

#[must_use]
pub fn containing_block_pos(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

#[must_use]
pub fn block_pos_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

#[must_use]
pub fn block_pos_dist_sqr(left: BlockPos, right: BlockPos) -> f64 {
    let dx = f64::from(left.x - right.x);
    let dy = f64::from(left.y - right.y);
    let dz = f64::from(left.z - right.z);
    dx * dx + dy * dy + dz * dz
}

#[must_use]
pub const fn block_to_section_coord(coord: i32) -> i32 {
    coord >> 4
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventSectionBounds {
    pub min_x: i32,
    pub min_y: i32,
    pub min_z: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub max_z: i32,
}

#[must_use]
pub fn game_event_section_bounds(center: BlockPos, radius: i32) -> GameEventSectionBounds {
    GameEventSectionBounds {
        min_x: block_to_section_coord(center.x - radius),
        min_y: block_to_section_coord(center.y - radius),
        min_z: block_to_section_coord(center.z - radius),
        max_x: block_to_section_coord(center.x + radius),
        max_y: block_to_section_coord(center.y + radius),
        max_z: block_to_section_coord(center.z + radius),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameEventContext {
    /// `GameEvent.Context.sourceEntity()`, as an entity id.
    pub source_entity_id: Option<i32>,
    /// `GameEvent.Context.affectedState()`, as a global block-state id.
    pub affected_state: Option<i32>,
}

#[must_use]
pub const fn game_event_context(recorded: RecordedGameEvent) -> GameEventContext {
    GameEventContext {
        source_entity_id: recorded.source_entity_id,
        affected_state: recorded.affected_state,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DeliveryMode {
    #[default]
    Unspecified,
    ByDistance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntityGameEventListenerKind {
    /// `VibrationSystem.Listener`.
    Vibration,
    AllayJukebox,
}

/// The object a registered listener resolves to when it is delivered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GameEventListenerOwner {
    /// The block entity at the entry's `pos`.
    #[default]
    BlockEntity,
    /// One of an entity's dynamic listeners.
    Entity {
        entity_id: i32,
        kind: EntityGameEventListenerKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventListenerEntry {
    /// Entity listener positions are resolved live; this position selects
    /// the section where the listener last registered.
    pub pos: BlockPos,
    /// `GameEventListener.getListenerRadius()`.
    pub radius: i32,
    pub delivery_mode: DeliveryMode,
    pub owner: GameEventListenerOwner,
}

fn section_of(pos: BlockPos) -> i32 {
    pos.y >> 4
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GameEventListenerSection {
    listeners: Vec<GameEventListenerEntry>,
}

impl GameEventListenerSection {
    #[must_use]
    pub fn listeners(&self) -> &[GameEventListenerEntry] {
        &self.listeners
    }

    pub fn is_empty(&self) -> bool {
        self.listeners.is_empty()
    }

    pub fn register(&mut self, listener: GameEventListenerEntry) {
        self.listeners.push(listener);
    }

    pub fn unregister(&mut self, listener: GameEventListenerEntry) -> bool {
        if let Some(index) = self
            .listeners
            .iter()
            .position(|candidate| candidate == &listener)
        {
            self.listeners.remove(index);
            return true;
        }
        false
    }
}

pub enum BlockEntityGameEventListener<'a> {
    Catalyst {
        pos: BlockPos,
        listener: &'a mut sculk_catalyst::SculkCatalystBlockEntity,
    },
    Sensor {
        pos: BlockPos,
        radius: i32,
        listener: &'a mut sculk_sensor::SculkSensorBlockEntity,
    },
    Shrieker {
        pos: BlockPos,
        listener: &'a mut sculk_shrieker::SculkShriekerBlockEntity,
    },
}

impl BlockEntityGameEventListener<'_> {
    pub(crate) fn handle_game_event(
        &mut self,
        level: &mut WorldVibrationReads<'_>,
        event: VanillaGameEvent,
        context: GameEventContext,
        source_position: Vec3d,
    ) -> bool {
        match self {
            Self::Catalyst { pos, listener } => {
                listener.handle_game_event(*pos, level, event, context, source_position)
            }
            Self::Sensor { pos, listener, .. } => {
                listener.handle_game_event(*pos, level, event, context, source_position)
            }
            Self::Shrieker { pos, listener } => {
                listener.handle_game_event(*pos, level, event, context, source_position)
            }
        }
    }
}

impl BlockEntity {
    #[must_use]
    pub fn game_event_listener_mut(&mut self) -> Option<BlockEntityGameEventListener<'_>> {
        let pos = self.block_pos();
        let state = self.block_state();
        match &mut self.payload {
            BlockEntityPayload::SculkCatalyst(listener) => {
                Some(BlockEntityGameEventListener::Catalyst { pos, listener })
            }
            BlockEntityPayload::SculkSensor(listener) => {
                Some(BlockEntityGameEventListener::Sensor {
                    pos,
                    radius: sculk_sensor::listener_radius(state),
                    listener,
                })
            }
            BlockEntityPayload::SculkShrieker(listener) => {
                Some(BlockEntityGameEventListener::Shrieker { pos, listener })
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LevelGameEventListeners {
    sections: FxHashMap<(ChunkKey, i32), GameEventListenerSection>,
    dynamic: FxHashMap<(i32, EntityGameEventListenerKind), DynamicListenerSection>,
    lent_sections: Vec<(ChunkKey, i32)>,
    lent_unregisters: Vec<((ChunkKey, i32), GameEventListenerEntry)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DynamicListenerSection {
    section: (ChunkKey, i32),
    entry: GameEventListenerEntry,
}

fn section_of_position(position: Vec3d) -> (ChunkKey, i32) {
    let pos = containing_block_pos(position);
    (chunk_of(pos), section_of(pos))
}

impl LevelGameEventListeners {
    pub fn move_entity_listener(
        &mut self,
        entity_id: i32,
        kind: EntityGameEventListenerKind,
        radius: i32,
        position: Vec3d,
        chunk_exists: impl Fn(ChunkKey) -> bool,
    ) {
        let current = section_of_position(position);
        let key = (entity_id, kind);
        let previous = self.dynamic.get(&key).copied();
        if previous.is_some_and(|last| last.section == current) {
            return;
        }
        if let Some(last) = previous {
            if chunk_exists(last.section.0) {
                self.unregister_in(last.section, last.entry);
            }
        }
        let entry = GameEventListenerEntry {
            pos: containing_block_pos(position),
            radius,
            delivery_mode: DeliveryMode::Unspecified,
            owner: GameEventListenerOwner::Entity { entity_id, kind },
        };
        self.dynamic.insert(
            key,
            DynamicListenerSection {
                section: current,
                entry,
            },
        );
        if chunk_exists(current.0) {
            self.section_mut(current.0, current.1).register(entry);
        }
    }

    pub fn remove_entity_listener(
        &mut self,
        entity_id: i32,
        kind: EntityGameEventListenerKind,
        chunk_exists: impl Fn(ChunkKey) -> bool,
    ) {
        if let Some(last) = self.dynamic.remove(&(entity_id, kind)) {
            if chunk_exists(last.section.0) {
                self.unregister_in(last.section, last.entry);
            }
        }
    }

    /// The column an entity listener's remembered section is in.
    #[must_use]
    pub fn entity_listener_column(
        &self,
        entity_id: i32,
        kind: EntityGameEventListenerKind,
    ) -> Option<ChunkKey> {
        self.dynamic
            .get(&(entity_id, kind))
            .map(|last| last.section.0)
    }

    fn unregister_in(&mut self, key: (ChunkKey, i32), listener: GameEventListenerEntry) {
        let removed_from_map = self
            .sections
            .get_mut(&key)
            .is_some_and(|section| section.unregister(listener));
        if removed_from_map {
            self.drop_empty_section(key);
        } else if self.lent_sections.contains(&key) {
            self.lent_unregisters.push((key, listener));
        }
    }

    fn section_mut(&mut self, chunk: ChunkKey, section_y: i32) -> &mut GameEventListenerSection {
        self.sections.entry((chunk, section_y)).or_default()
    }

    pub(crate) fn register(&mut self, listener: GameEventListenerEntry) {
        self.section_mut(chunk_of(listener.pos), section_of(listener.pos))
            .register(listener);
    }

    pub(crate) fn unregister(&mut self, listener: GameEventListenerEntry) {
        self.unregister_in((chunk_of(listener.pos), section_of(listener.pos)), listener);
    }

    fn drop_empty_section(&mut self, key: (ChunkKey, i32)) {
        if self
            .sections
            .get(&key)
            .is_some_and(GameEventListenerSection::is_empty)
        {
            self.sections.remove(&key);
        }
    }

    /// The section's registry, when it holds anything.
    #[must_use]
    pub fn section(&self, chunk: ChunkKey, section_y: i32) -> Option<&GameEventListenerSection> {
        self.sections.get(&(chunk, section_y))
    }

    /// Every registered listener in this level.
    ///
    /// `DebugGameEventListenerInfo` publication reads this once per debug tick;
    /// the sections' internals stay private.
    pub fn listeners(&self) -> impl Iterator<Item = &GameEventListenerEntry> {
        self.sections
            .values()
            .flat_map(|section| section.listeners().iter())
    }

    /// Release the store borrow while World resolves and delivers listeners.
    pub fn take_section(
        &mut self,
        chunk: ChunkKey,
        section_y: i32,
    ) -> Option<GameEventListenerSection> {
        let key = (chunk, section_y);
        let section = self.sections.remove(&key)?;
        self.lent_sections.push(key);
        Some(section)
    }

    /// Preserve registrations and unregistrations that occurred while this
    /// section was lent to World.
    pub fn restore_section(
        &mut self,
        chunk: ChunkKey,
        section_y: i32,
        mut section: GameEventListenerSection,
    ) {
        let key = (chunk, section_y);
        if let Some(index) = self.lent_sections.iter().position(|&lent| lent == key) {
            self.lent_sections.remove(index);
        }
        if !self.lent_unregisters.is_empty() {
            let mut index = 0;
            while index < self.lent_unregisters.len() {
                if self.lent_unregisters[index].0 == key {
                    let (_, listener) = self.lent_unregisters.swap_remove(index);
                    section.unregister(listener);
                } else {
                    index += 1;
                }
            }
        }
        if let Some(during_walk) = self.sections.remove(&key) {
            section.merge_during_walk(during_walk);
        }
        if !section.is_empty() {
            self.sections.insert(key, section);
        }
    }

    /// Drops every section registry of one column, which a whole-column
    /// arrival rebuilds from the block entities it brings.
    pub(crate) fn remove_chunk(&mut self, chunk: ChunkKey) {
        self.sections
            .retain(|(section_chunk, _), _| *section_chunk != chunk);
        self.lent_sections
            .retain(|(section_chunk, _)| *section_chunk != chunk);
        self.lent_unregisters
            .retain(|((section_chunk, _), _)| *section_chunk != chunk);
    }
}

impl GameEventListenerSection {
    fn merge_during_walk(&mut self, during_walk: GameEventListenerSection) {
        self.listeners.extend(during_walk.listeners);
    }
}

#[cfg(test)]
mod tests {
    use crate::level_event::{LevelEventSink, VanillaGameEvent, MAX_RECORDED_GAME_EVENTS};
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::packets::BlockPos;

    use super::{DeliveryMode, GameEventListenerEntry};
    use crate::block_entity::new_block_entity;

    fn at(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    /// `MAX_RECORDED_GAME_EVENTS` bounds the *record*; the queue a delivery is
    /// posted from keeps every event, so a body raising more events than the
    /// record holds still delivers all of them.
    #[test]
    fn the_delivery_queue_is_not_bounded_by_the_record() {
        let mut sink = LevelEventSink::default();
        for index in 0..(MAX_RECORDED_GAME_EVENTS as i32 + 5) {
            sink.game_event(VanillaGameEvent::BLOCK_CHANGE, at(index, 0, 0), None);
        }
        let pending = sink.drain_game_events();
        assert_eq!(pending.len(), MAX_RECORDED_GAME_EVENTS + 5);
    }

    #[test]
    fn calibrated_sensor_listener_survives_reload() {
        use bbb_protocol::block_types::{
            VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID, VANILLA_BLOCK_SCULK_SENSOR_ID,
        };
        for (block, radius) in [
            (VANILLA_BLOCK_SCULK_SENSOR_ID, 8),
            (VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID, 16),
        ] {
            let pos = at(0, 64, 0);
            let state = vanilla_block_default_state_id(block).unwrap();
            let entity = new_block_entity(pos, state).unwrap();
            let expected = GameEventListenerEntry {
                pos,
                radius,
                delivery_mode: DeliveryMode::Unspecified,
                owner: super::GameEventListenerOwner::BlockEntity,
            };
            assert_eq!(entity.game_event_listener_entry(), Some(expected));
            let saved = entity
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
                .expect("fixture save");
            let reloaded = crate::block_entity::load_test_fixture(pos, state, &saved)
                .expect("fixture load")
                .expect("sensor record");
            assert_eq!(reloaded.game_event_listener_entry(), Some(expected));
        }
    }
}
