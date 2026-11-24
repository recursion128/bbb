use std::collections::{BTreeMap, BTreeSet};

mod banner;
mod crafting;
pub(super) mod terrain;

use crate::chunk::ChunkStore;
use crate::connection::ConnectionId;
use crate::entity::FramedMapAdmission;
use bbb_protocol::packets::{
    BlockPos, DataComponentPatchSummary, ItemMapDecorationSummary, ItemStackSummary, MapColorPatch,
    MapDecoration, MapItemData,
};
use bbb_protocol::{JavaString, RichTextComponent};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_persistence::world_storage::Java26ColumnReader;
use bbb_server_types::item_stack::RuntimeItemStack;

fn vanilla_map_decoration_track_count(type_id: i32) -> Option<bool> {
    bbb_facts::vanilla_map_decoration_type(type_id).map(|entry| entry.track_count)
}

const MAP_SIZE: usize = 128;
const MAP_PIXELS: usize = MAP_SIZE * MAP_SIZE;
const PLAYER_DECORATION_TYPE: i32 = 0;
const FRAME_DECORATION_TYPE: i32 = 1;
const PLAYER_OFF_MAP_DECORATION_TYPE: i32 = 6;
const PLAYER_OFF_LIMITS_DECORATION_TYPE: i32 = 7;
const NETHER_DIMENSION: &str = "minecraft:the_nether";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ServerMapPersistentData {
    pub(super) dimension: String,
    pub(super) center_x: i32,
    pub(super) center_z: i32,
    pub(super) scale: i8,
    pub(super) colors: Vec<u8>,
    pub(super) tracking_position: bool,
    pub(super) unlimited_tracking: bool,
    pub(super) locked: bool,
    pub(super) banners: Vec<ServerMapBanner>,
    pub(super) frames: Vec<ServerMapFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ServerMapBanner {
    pub(super) pos: BlockPos,
    pub(super) color: String,
    pub(super) name: Option<RichTextComponent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ServerMapFrame {
    pub(super) pos: BlockPos,
    pub(super) rotation: i32,
    pub(super) entity_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ServerMapFrameDirection {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl ServerMapFrameDirection {
    fn data_2d(self) -> i32 {
        match self {
            Self::Down | Self::Up => -1,
            Self::North => 2,
            Self::South => 0,
            Self::West => 1,
            Self::East => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ServerMapPlacedFrame {
    pub(super) entity_id: i32,
    pub(super) pos: BlockPos,
    pub(super) direction: ServerMapFrameDirection,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ServerMapCarrier {
    pub(super) connection_id: ConnectionId,
    pub(super) plain_name: String,
    pub(super) dimension: String,
    pub(super) x: f64,
    pub(super) z: f64,
    pub(super) y_rot: f64,
    pub(super) removed: bool,
    pub(super) holds_matching_map: bool,
    pub(super) wears_map_invisibility_equipment: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DirtyServerMapSnapshot {
    pub(super) map_id: i32,
    pub(super) epoch: u64,
    pub(super) data: ServerMapPersistentData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ServerMapError {
    InvalidColorCount { actual: usize },
    InvalidColorCoordinate { x: usize, y: usize },
    UnknownBannerColor(String),
    UnknownDecorationType(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KeyedDecoration {
    key: JavaString,
    value: MapDecoration,
}

#[derive(Debug, Clone, PartialEq)]
struct HoldingPlayer {
    carrier: ServerMapCarrier,
    dirty_data: bool,
    min_dirty_x: usize,
    min_dirty_y: usize,
    max_dirty_x: usize,
    max_dirty_y: usize,
    dirty_decorations: bool,
    decoration_tick: i32,
    step: i32,
}

impl HoldingPlayer {
    fn new(carrier: ServerMapCarrier) -> Self {
        Self {
            carrier,
            dirty_data: true,
            min_dirty_x: 0,
            min_dirty_y: 0,
            max_dirty_x: MAP_SIZE - 1,
            max_dirty_y: MAP_SIZE - 1,
            dirty_decorations: true,
            decoration_tick: 0,
            step: 0,
        }
    }

    fn mark_color_dirty(&mut self, x: usize, y: usize) {
        if self.dirty_data {
            self.min_dirty_x = self.min_dirty_x.min(x);
            self.min_dirty_y = self.min_dirty_y.min(y);
            self.max_dirty_x = self.max_dirty_x.max(x);
            self.max_dirty_y = self.max_dirty_y.max(y);
        } else {
            self.dirty_data = true;
            self.min_dirty_x = x;
            self.min_dirty_y = y;
            self.max_dirty_x = x;
            self.max_dirty_y = y;
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ServerMapSavedData {
    dimension: String,
    center_x: i32,
    center_z: i32,
    scale: i8,
    colors: Vec<u8>,
    tracking_position: bool,
    unlimited_tracking: bool,
    locked: bool,
    banners: BTreeMap<(i32, i32, i32), ServerMapBanner>,
    frames: BTreeMap<(i32, i32, i32), ServerMapFrame>,
    decorations: Vec<KeyedDecoration>,
    tracked_decoration_count: usize,
    holding_order: Vec<ConnectionId>,
    holding_players: BTreeMap<ConnectionId, HoldingPlayer>,
    persistent_epoch: u64,
    dirty_epoch: Option<u64>,
}

impl ServerMapSavedData {
    fn from_persistent(data: ServerMapPersistentData) -> Result<Self, ServerMapError> {
        if data.colors.len() != MAP_PIXELS {
            return Err(ServerMapError::InvalidColorCount {
                actual: data.colors.len(),
            });
        }

        let mut map = Self {
            dimension: data.dimension,
            center_x: data.center_x,
            center_z: data.center_z,
            scale: data.scale.clamp(0, 4),
            colors: data.colors,
            tracking_position: data.tracking_position,
            unlimited_tracking: data.unlimited_tracking,
            locked: data.locked,
            banners: BTreeMap::new(),
            frames: BTreeMap::new(),
            decorations: Vec::new(),
            tracked_decoration_count: 0,
            holding_order: Vec::new(),
            holding_players: BTreeMap::new(),
            persistent_epoch: 0,
            dirty_epoch: None,
        };

        // MapItemSavedData.java:120-128: persistent banners reconstruct before frames.
        for banner in data.banners {
            let key = block_pos_key(banner.pos);
            let decoration_key = banner_key(banner.pos);
            let type_id = banner_decoration_type(&banner.color)?;
            map.banners.insert(key, banner.clone());
            map.add_decoration(
                type_id,
                None,
                &decoration_key,
                [f64::from(banner.pos.x), f64::from(banner.pos.z), 180.0],
                banner.name,
            )?;
        }
        for frame in data.frames {
            map.frames.insert(block_pos_key(frame.pos), frame);
            map.add_decoration(
                FRAME_DECORATION_TYPE,
                None,
                &frame_entity_key(frame.entity_id),
                [
                    f64::from(frame.pos.x),
                    f64::from(frame.pos.z),
                    f64::from(frame.rotation),
                ],
                None,
            )?;
        }
        Ok(map)
    }

    fn persistent_data(&self) -> ServerMapPersistentData {
        ServerMapPersistentData {
            dimension: self.dimension.clone(),
            center_x: self.center_x,
            center_z: self.center_z,
            scale: self.scale,
            colors: self.colors.clone(),
            tracking_position: self.tracking_position,
            unlimited_tracking: self.unlimited_tracking,
            locked: self.locked,
            banners: self.banners.values().cloned().collect(),
            frames: self.frames.values().copied().collect(),
        }
    }

    fn tick_carried_by(
        &mut self,
        carrier: ServerMapCarrier,
        live_carriers: &[ServerMapCarrier],
        placed_in_frame: Option<ServerMapPlacedFrame>,
        static_decorations: &[ItemMapDecorationSummary],
        game_time: i64,
    ) -> Result<(), ServerMapError> {
        // MapItemSavedData.java:171-235 fixes carrier, frame, then static ordering.
        for decoration in static_decorations {
            if self.decoration(&decoration.id).is_none()
                && vanilla_map_decoration_track_count(decoration.decoration_type_id).is_none()
            {
                return Err(ServerMapError::UnknownDecorationType(
                    decoration.decoration_type_id,
                ));
            }
        }

        let ticking_id = carrier.connection_id;
        let previous_name = self
            .holding_players
            .get(&ticking_id)
            .filter(|holding| holding.carrier.plain_name != carrier.plain_name)
            .map(|holding| holding.carrier.plain_name.clone());
        if let Some(holding) = self.holding_players.get_mut(&ticking_id) {
            holding.carrier = carrier;
        } else {
            self.holding_order.push(ticking_id);
            self.holding_players
                .insert(ticking_id, HoldingPlayer::new(carrier));
        }
        if let Some(previous_name) = previous_name {
            self.remove_decoration(&previous_name);
        }

        let ticking = self.holding_players[&ticking_id].carrier.clone();
        if !ticking.holds_matching_map {
            self.remove_decoration(&ticking.plain_name);
        }

        let mut index = 0;
        while index < self.holding_order.len() {
            let connection_id = self.holding_order[index];
            let stored = self.holding_players[&connection_id].carrier.clone();
            let live = if connection_id == ticking_id {
                Some(ticking.clone())
            } else {
                live_carriers
                    .iter()
                    .find(|candidate| candidate.connection_id == connection_id)
                    .cloned()
            };
            let Some(other) = live else {
                self.holding_order.remove(index);
                self.holding_players.remove(&connection_id);
                self.remove_decoration(&stored.plain_name);
                continue;
            };
            if stored.plain_name != other.plain_name {
                self.remove_decoration(&stored.plain_name);
            }
            self.holding_players
                .get_mut(&connection_id)
                .expect("holding order and player map stay paired")
                .carrier = other.clone();
            if other.removed || (placed_in_frame.is_none() && !other.holds_matching_map) {
                self.holding_order.remove(index);
                self.holding_players.remove(&connection_id);
                self.remove_decoration(&other.plain_name);
                continue;
            }

            if placed_in_frame.is_none()
                && other.dimension == self.dimension
                && self.tracking_position
            {
                self.add_decoration(
                    PLAYER_DECORATION_TYPE,
                    Some(game_time),
                    &other.plain_name,
                    [other.x, other.z, other.y_rot],
                    None,
                )?;
            }
            if connection_id != ticking_id && other.wears_map_invisibility_equipment {
                self.remove_decoration(&other.plain_name);
            }
            index += 1;
        }

        if let Some(placed) = placed_in_frame.filter(|_| self.tracking_position) {
            let position_key = block_pos_key(placed.pos);
            if let Some(existing) = self.frames.get(&position_key).copied() {
                if existing.entity_id != placed.entity_id {
                    self.remove_decoration(&frame_entity_key(existing.entity_id));
                }
            }

            let rotation = placed.direction.data_2d() * 90;
            let frame = ServerMapFrame {
                pos: placed.pos,
                rotation,
                entity_id: placed.entity_id,
            };
            self.add_decoration(
                FRAME_DECORATION_TYPE,
                Some(game_time),
                &frame_entity_key(placed.entity_id),
                [
                    f64::from(placed.pos.x),
                    f64::from(placed.pos.z),
                    f64::from(rotation),
                ],
                None,
            )?;
            if self.frames.insert(position_key, frame) != Some(frame) {
                self.mark_persistent_dirty();
            }
        }

        for decoration in static_decorations {
            if self.decoration(&decoration.id).is_none() {
                self.add_decoration(
                    decoration.decoration_type_id,
                    Some(game_time),
                    &decoration.id,
                    [
                        f64::from_bits(decoration.x_bits),
                        f64::from_bits(decoration.z_bits),
                        f64::from(f32::from_bits(decoration.rotation_bits)),
                    ],
                    None,
                )?;
            }
        }
        Ok(())
    }

    fn next_update_packet(
        &mut self,
        map_id: i32,
        connection_id: ConnectionId,
    ) -> Option<MapItemData> {
        // MapItemSavedData.java:529-548 distinguishes absent decorations from an empty update.
        let holding = self.holding_players.get_mut(&connection_id)?;
        let color_patch = if holding.dirty_data {
            holding.dirty_data = false;
            let start_x = holding.min_dirty_x;
            let start_y = holding.min_dirty_y;
            let width = holding.max_dirty_x + 1 - start_x;
            let height = holding.max_dirty_y + 1 - start_y;
            let mut colors = vec![0; width * height];
            for x in 0..width {
                for y in 0..height {
                    colors[x + y * width] = self.colors[start_x + x + (start_y + y) * MAP_SIZE];
                }
            }
            Some(MapColorPatch {
                start_x: start_x as u8,
                start_y: start_y as u8,
                width: width as u8,
                height: height as u8,
                colors,
            })
        } else {
            None
        };

        let decorations = if holding.dirty_decorations {
            let due = holding.decoration_tick.wrapping_rem(5) == 0;
            holding.decoration_tick = holding.decoration_tick.wrapping_add(1);
            due.then(|| {
                holding.dirty_decorations = false;
                self.decorations
                    .iter()
                    .map(|decoration| decoration.value.clone())
                    .collect()
            })
        } else {
            None
        };

        if decorations.is_none() && color_patch.is_none() {
            None
        } else {
            Some(MapItemData {
                map_id,
                scale: self.scale,
                locked: self.locked,
                decorations,
                color_patch,
            })
        }
    }

    fn add_decoration<'key, Q: ?Sized>(
        &mut self,
        type_id: i32,
        level_game_time: Option<i64>,
        key: &'key Q,
        [x, z, y_rot]: [f64; 3],
        name: Option<RichTextComponent>,
    ) -> Result<(), ServerMapError>
    where
        JavaString: PartialEq<Q> + From<&'key Q>,
    {
        // MapItemSavedData.java:275-297 preserves insertion order while replacing in place.
        vanilla_map_decoration_track_count(type_id)
            .ok_or(ServerMapError::UnknownDecorationType(type_id))?;
        let scale = (1_i32 << self.scale) as f32;
        let x_delta = ((x - f64::from(self.center_x)) as f32) / scale;
        let z_delta = ((z - f64::from(self.center_z)) as f32) / scale;
        let Some((projected_type, rotation)) =
            self.decoration_location(type_id, level_game_time, y_rot, x_delta, z_delta)
        else {
            self.remove_decoration(key);
            return Ok(());
        };
        let projected_track_count = vanilla_map_decoration_track_count(projected_type)
            .ok_or(ServerMapError::UnknownDecorationType(projected_type))?;
        let new = MapDecoration {
            type_id: projected_type,
            x: clamp_map_coordinate(x_delta),
            y: clamp_map_coordinate(z_delta),
            rot: rotation,
            name,
        };

        if let Some(index) = self.decoration_index(key) {
            if self.decorations[index].value == new {
                return Ok(());
            }
            if vanilla_map_decoration_track_count(self.decorations[index].value.type_id)
                == Some(true)
            {
                self.tracked_decoration_count -= 1;
            }
            self.decorations[index].value = new;
        } else {
            self.decorations.push(KeyedDecoration {
                key: key.into(),
                value: new,
            });
        }
        if projected_track_count {
            self.tracked_decoration_count += 1;
        }
        self.mark_decorations_dirty();
        Ok(())
    }

    fn decoration_location(
        &self,
        type_id: i32,
        level_game_time: Option<i64>,
        y_rot: f64,
        x_delta: f32,
        z_delta: f32,
    ) -> Option<(i32, u8)> {
        if type_id == PLAYER_DECORATION_TYPE {
            if is_inside_map(x_delta, z_delta) {
                Some((type_id, self.calculate_rotation(level_game_time, y_rot)))
            } else if x_delta.abs() < 320.0 && z_delta.abs() < 320.0 {
                Some((PLAYER_OFF_MAP_DECORATION_TYPE, 0))
            } else if self.unlimited_tracking {
                Some((PLAYER_OFF_LIMITS_DECORATION_TYPE, 0))
            } else {
                None
            }
        } else if is_inside_map(x_delta, z_delta) || self.unlimited_tracking {
            Some((type_id, self.calculate_rotation(level_game_time, y_rot)))
        } else {
            None
        }
    }

    fn calculate_rotation(&self, level_game_time: Option<i64>, y_rot: f64) -> u8 {
        if self.dimension == NETHER_DIMENSION {
            if let Some(game_time) = level_game_time {
                let s = (game_time / 10) as i32;
                let value = s
                    .wrapping_mul(s)
                    .wrapping_mul(34_187_121)
                    .wrapping_add(s.wrapping_mul(121));
                return ((value >> 15) & 15) as u8;
            }
        }
        let adjusted = if y_rot < 0.0 {
            y_rot - 8.0
        } else {
            y_rot + 8.0
        };
        ((adjusted * 16.0 / 360.0) as i32 as i8) as u8
    }

    fn remove_decoration<Q: ?Sized>(&mut self, key: &Q)
    where
        JavaString: PartialEq<Q>,
    {
        if let Some(index) = self.decoration_index(key) {
            let removed = self.decorations.remove(index);
            if vanilla_map_decoration_track_count(removed.value.type_id) == Some(true) {
                self.tracked_decoration_count -= 1;
            }
        }
        // MapItemSavedData.java:249-255 marks every holder even when the key was absent.
        self.mark_decorations_dirty();
    }

    fn decoration_index<Q: ?Sized>(&self, key: &Q) -> Option<usize>
    where
        JavaString: PartialEq<Q>,
    {
        self.decorations
            .iter()
            .position(|decoration| decoration.key.eq(key))
    }

    fn decoration<Q: ?Sized>(&self, key: &Q) -> Option<&MapDecoration>
    where
        JavaString: PartialEq<Q>,
    {
        self.decoration_index(key)
            .map(|index| &self.decorations[index].value)
    }

    fn mark_decorations_dirty(&mut self) {
        for holding in self.holding_players.values_mut() {
            holding.dirty_decorations = true;
        }
    }

    fn mark_persistent_dirty(&mut self) {
        self.persistent_epoch = self.persistent_epoch.wrapping_add(1);
        if self.persistent_epoch == 0 {
            self.persistent_epoch = 1;
        }
        self.dirty_epoch = Some(self.persistent_epoch);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ServerMapStore {
    pub(super) maps: BTreeMap<i32, ServerMapSavedData>,
    stored_maps: BTreeSet<i32>,
    reader: Option<Java26ColumnReader>,
    last_id: i32,
    index_dirty: bool,
}

impl Default for ServerMapStore {
    fn default() -> Self {
        Self {
            maps: BTreeMap::new(),
            stored_maps: BTreeSet::new(),
            reader: None,
            last_id: -1,
            index_dirty: false,
        }
    }
}

#[derive(Clone, Copy)]
pub(in crate::world) struct ExplorationMapSpec<'a> {
    pub(in crate::world) dimension: &'a str,
    pub(in crate::world) target: BlockPos,
    pub(in crate::world) zoom: i8,
    pub(in crate::world) decoration: &'a str,
}

impl ServerMapStore {
    #[cfg(test)]
    pub(super) fn from_persistent_parts(
        maps: impl IntoIterator<Item = (i32, ServerMapPersistentData)>,
        last_id: i32,
    ) -> Result<Self, ServerMapError> {
        Self::from_persistent_parts_with_stored(maps, BTreeSet::new(), last_id, None)
    }

    pub(super) fn from_persistent_parts_with_stored(
        maps: impl IntoIterator<Item = (i32, ServerMapPersistentData)>,
        stored_maps: BTreeSet<i32>,
        last_id: i32,
        reader: Option<Java26ColumnReader>,
    ) -> Result<Self, ServerMapError> {
        let mut store = Self {
            stored_maps,
            reader,
            last_id,
            ..Self::default()
        };
        for (map_id, data) in maps {
            store.insert_loaded(map_id, data)?;
        }
        Ok(store)
    }

    pub(super) fn insert_loaded(
        &mut self,
        map_id: i32,
        data: ServerMapPersistentData,
    ) -> Result<Option<ServerMapSavedData>, ServerMapError> {
        self.stored_maps.remove(&map_id);
        Ok(self
            .maps
            .insert(map_id, ServerMapSavedData::from_persistent(data)?))
    }

    #[cfg(test)]
    pub(super) fn insert_new(
        &mut self,
        map_id: i32,
        data: ServerMapPersistentData,
    ) -> Result<Option<ServerMapSavedData>, ServerMapError> {
        let mut map = ServerMapSavedData::from_persistent(data)?;
        map.mark_persistent_dirty();
        self.stored_maps.remove(&map_id);
        Ok(self.maps.insert(map_id, map))
    }

    pub(super) fn ensure_loaded(&mut self, map_id: i32) -> Option<&ServerMapSavedData> {
        if !self.maps.contains_key(&map_id) && self.stored_maps.remove(&map_id) {
            if let Some(reader) = self.reader.as_ref() {
                match reader.read_map(map_id) {
                    Ok(Some(saved)) => match ServerMapPersistentData::from_saved(&saved)
                        .map_err(|error| error.to_string())
                        .and_then(|data| {
                            ServerMapSavedData::from_persistent(data)
                                .map_err(|error| format!("{error:?}"))
                        }) {
                        Ok(map) => {
                            self.maps.insert(map_id, map);
                        }
                        Err(error) => {
                            tracing::warn!(event = "world.map_decode_failed", map_id, %error);
                        }
                    },
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(event = "world.map_read_failed", map_id, %error);
                    }
                }
            }
        }
        self.maps.get(&map_id)
    }

    pub(super) fn ensure_loaded_mut(&mut self, map_id: i32) -> Option<&mut ServerMapSavedData> {
        let _ = self.ensure_loaded(map_id);
        self.maps.get_mut(&map_id)
    }

    #[cfg(test)]
    pub(crate) fn load_all_stored_maps_for_test(&mut self) {
        let ids: Vec<i32> = self.stored_maps.iter().copied().collect();
        for id in ids {
            let _ = self.ensure_loaded(id);
        }
    }

    #[cfg(test)]
    pub(crate) fn is_map_loaded_in_memory_for_test(&self, map_id: i32) -> bool {
        self.maps.contains_key(&map_id)
    }

    pub(super) fn dirty_index(&self) -> Option<i32> {
        self.index_dirty.then_some(self.last_id)
    }

    pub(super) fn acknowledge_index(&mut self, saved: Option<i32>) {
        if saved == Some(self.last_id) {
            self.index_dirty = false;
        }
    }

    // MapItemSavedData.java:131-144: fresh maps retain the supplied scale; only loading clamps it.
    fn create_fresh(
        &mut self,
        dimension: String,
        origin_x: f64,
        origin_z: f64,
        scale: i8,
        tracking_position: bool,
        unlimited_tracking: bool,
    ) -> Result<i32, ServerMapError> {
        let mut map = ServerMapSavedData::from_persistent(ServerMapPersistentData {
            dimension,
            center_x: fresh_map_center(origin_x, scale),
            center_z: fresh_map_center(origin_z, scale),
            scale,
            colors: vec![0; MAP_PIXELS],
            tracking_position,
            unlimited_tracking,
            locked: false,
            banners: Vec::new(),
            frames: Vec::new(),
        })?;
        map.scale = scale;
        map.mark_persistent_dirty();
        // MapIndex.java:29-33: allocation follows the index, not existing saved-map IDs.
        self.last_id = self.last_id.wrapping_add(1);
        self.index_dirty = true;
        self.stored_maps.remove(&self.last_id);
        self.maps.insert(self.last_id, map);
        Ok(self.last_id)
    }

    fn create_map(
        &mut self,
        dimension: &str,
        origin_x: i32,
        origin_z: i32,
        scale: i8,
        tracking_position: bool,
        unlimited_tracking: bool,
    ) -> bbb_server_types::Result<RuntimeItemStack> {
        use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID;
        let id = self.last_id.wrapping_add(1);
        let stack = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                map_id: Some(id),
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID],
                ..Default::default()
            }
            .into(),
        });
        self.create_fresh(
            dimension.to_owned(),
            f64::from(origin_x),
            f64::from(origin_z),
            scale,
            tracking_position,
            unlimited_tracking,
        )
        .map_err(|error| {
            bbb_server_types::ServerWorldError::InvalidConfiguration(format!(
                "map creation: {error:?}"
            ))
        })?;
        Ok(stack)
    }

    pub(in crate::world) fn create_exploration_map(
        &mut self,
        chunks: &ChunkStore,
        seed: i64,
        spec: ExplorationMapSpec<'_>,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> Result<RuntimeItemStack, String> {
        let ExplorationMapSpec {
            dimension,
            target,
            zoom,
            decoration,
        } = spec;
        use bbb_protocol::data_component_types::{
            VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
        };
        let decoration_type = bbb_facts::vanilla_map_decoration_type_for_resource_id(decoration)
            .ok_or_else(|| format!("unknown map decoration type {decoration}"))?;
        // MapItemSavedData.java:258-264: target rotation is a float and the key is '+'.
        use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue as Nbt};
        let field = |name: &str, value| NbtSummaryEntry {
            name: name.into(),
            value,
        };
        let target_component = Nbt::Compound(vec![field(
            "+",
            Nbt::Compound(vec![
                field("type", Nbt::String(decoration_type.resource_id.into())),
                field("x", Nbt::Double(f64::from(target.x).to_bits())),
                field("z", Nbt::Double(f64::from(target.z).to_bits())),
                field("rotation", Nbt::Float(180.0_f32.to_bits())),
            ]),
        )]);
        let watery = exploration_map_biomes(
            chunks,
            seed,
            fresh_map_center(f64::from(target.x), zoom),
            fresh_map_center(f64::from(target.z), zoom),
            zoom,
            resolver,
        )?;
        let mut stack = self
            .create_map(dimension, target.x, target.z, zoom, true, true)
            .map_err(|error| error.to_string())?;
        self.render_biome_preview(self.last_id, &watery)?;
        stack = stack
            .with_component_nbt(
                VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
                Some(&target_component),
                resolver,
            )
            .map_err(|error| error.to_string())?;
        if let Some(color) = decoration_type.map_color {
            stack = stack
                .with_component_nbt(
                    VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID,
                    Some(&Nbt::Int(color)),
                    resolver,
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(stack)
    }

    // MapItem.java:225-272: NONE preserves existing pixels and the one-pixel border.
    fn render_biome_preview(
        &mut self,
        map_id: i32,
        watery: &[bool; MAP_PIXELS],
    ) -> Result<(), String> {
        for x in 1..MAP_SIZE - 1 {
            for z in 1..MAP_SIZE - 1 {
                let mut water_count = 0;
                for dx in -1..=1 {
                    for dz in -1..=1 {
                        if (dx != 0 || dz != 0)
                            && watery
                                [(z as i32 + dz) as usize * MAP_SIZE + (x as i32 + dx) as usize]
                        {
                            water_count += 1;
                        }
                    }
                }
                let color = if watery[z * MAP_SIZE + x] {
                    let brightness = if water_count > 7 {
                        if z % 2 != 0 {
                            continue;
                        }
                        let wave = (bbb_server_types::geometry::mth_sin(z as f64) * 7.0) as i32;
                        match (x as i32 + wave) / 8 % 5 {
                            0 | 4 => 0,
                            1 | 3 => 1,
                            2 => 2,
                            _ => 3,
                        }
                    } else if water_count > 5 {
                        1
                    } else if water_count > 1 {
                        0
                    } else {
                        3
                    };
                    // MapColor.java:24-35 and MapColor.java:107-111 fix color and brightness IDs.
                    15 * 4 + brightness
                } else if water_count > 0 {
                    26 * 4 + if water_count > 3 { 1 } else { 3 }
                } else {
                    continue;
                };
                self.update_color(map_id, x, z, color)
                    .map_err(|error| format!("map preview: {error:?}"))?;
            }
        }
        Ok(())
    }

    pub(super) fn admission(&mut self, map_id: i32) -> FramedMapAdmission {
        match self.ensure_loaded(map_id) {
            Some(map) => FramedMapAdmission::Present {
                tracked_decoration_count: map.tracked_decoration_count,
            },
            None => FramedMapAdmission::SavedDataMissing,
        }
    }

    pub(super) fn tick_carried_by(
        &mut self,
        map_id: i32,
        carrier: ServerMapCarrier,
        live_carriers: &[ServerMapCarrier],
        placed_in_frame: Option<ServerMapPlacedFrame>,
        static_decorations: &[ItemMapDecorationSummary],
        game_time: i64,
    ) -> Result<bool, ServerMapError> {
        let Some(map) = self.ensure_loaded_mut(map_id) else {
            return Ok(false);
        };
        map.tick_carried_by(
            carrier,
            live_carriers,
            placed_in_frame,
            static_decorations,
            game_time,
        )?;
        Ok(true)
    }

    pub(super) fn next_update_packet(
        &mut self,
        map_id: i32,
        connection_id: ConnectionId,
    ) -> Option<MapItemData> {
        self.ensure_loaded_mut(map_id)?
            .next_update_packet(map_id, connection_id)
    }

    pub(super) fn removed_from_frame(
        &mut self,
        map_id: i32,
        pos: BlockPos,
        entity_id: i32,
    ) -> bool {
        let Some(map) = self.ensure_loaded_mut(map_id) else {
            return false;
        };
        // MapItemSavedData.java:442-445: removal never deletes the SavedData itself.
        map.remove_decoration(&frame_entity_key(entity_id));
        map.frames.remove(&block_pos_key(pos));
        map.mark_persistent_dirty();
        true
    }

    pub(super) fn remove_player(&mut self, connection_id: ConnectionId) {
        for map in self.maps.values_mut() {
            let Some(holding) = map.holding_players.remove(&connection_id) else {
                continue;
            };
            map.holding_order.retain(|id| *id != connection_id);
            map.remove_decoration(&holding.carrier.plain_name);
        }
    }

    pub(super) fn update_color(
        &mut self,
        map_id: i32,
        x: usize,
        y: usize,
        color: u8,
    ) -> Result<bool, ServerMapError> {
        if x >= MAP_SIZE || y >= MAP_SIZE {
            return Err(ServerMapError::InvalidColorCoordinate { x, y });
        }
        let Some(map) = self.ensure_loaded_mut(map_id) else {
            return Ok(false);
        };
        let index = x + y * MAP_SIZE;
        if map.colors[index] == color {
            return Ok(false);
        }
        map.colors[index] = color;
        map.mark_persistent_dirty();
        for holding in map.holding_players.values_mut() {
            holding.mark_color_dirty(x, y);
        }
        Ok(true)
    }

    pub(super) fn dirty_persistent_snapshots(&self) -> Vec<DirtyServerMapSnapshot> {
        self.maps
            .iter()
            .filter_map(|(&map_id, map)| {
                Some(DirtyServerMapSnapshot {
                    map_id,
                    epoch: map.dirty_epoch?,
                    data: map.persistent_data(),
                })
            })
            .collect()
    }

    pub(super) fn acknowledge_persistent_snapshot(&mut self, map_id: i32, epoch: u64) -> bool {
        let Some(map) = self.maps.get_mut(&map_id) else {
            return false;
        };
        if map.dirty_epoch == Some(epoch) {
            map.dirty_epoch = None;
            true
        } else {
            false
        }
    }
}

fn block_pos_key(pos: BlockPos) -> (i32, i32, i32) {
    (pos.x, pos.y, pos.z)
}

fn banner_key(pos: BlockPos) -> String {
    format!("banner-{},{},{}", pos.x, pos.y, pos.z)
}

fn frame_entity_key(entity_id: i32) -> String {
    format!("frame-{entity_id}")
}

fn banner_decoration_type(color: &str) -> Result<i32, ServerMapError> {
    let type_id = match color {
        "white" => 10,
        "orange" => 11,
        "magenta" => 12,
        "light_blue" => 13,
        "yellow" => 14,
        "lime" => 15,
        "pink" => 16,
        "gray" => 17,
        "light_gray" => 18,
        "cyan" => 19,
        "purple" => 20,
        "blue" => 21,
        "brown" => 22,
        "green" => 23,
        "red" => 24,
        "black" => 25,
        _ => return Err(ServerMapError::UnknownBannerColor(color.to_owned())),
    };
    Ok(type_id)
}

fn is_inside_map(x: f32, z: f32) -> bool {
    (-63.0..=63.0).contains(&x) && (-63.0..=63.0).contains(&z)
}

fn clamp_map_coordinate(delta: f32) -> i8 {
    if delta <= -63.0 {
        -128
    } else if delta >= 63.0 {
        127
    } else {
        (delta * 2.0 + 0.5) as i32 as i8
    }
}

impl ServerMapPersistentData {
    pub(super) fn from_saved(
        map: &bbb_server_persistence::Java26MapSavedData,
    ) -> bbb_server_types::save::SaveResult<Self> {
        let banners = map
            .banners
            .iter()
            .map(|banner| {
                Ok(ServerMapBanner {
                    pos: BlockPos {
                        x: banner.pos[0],
                        y: banner.pos[1],
                        z: banner.pos[2],
                    },
                    color: banner.color.clone(),
                    name: banner
                        .name
                        .clone()
                        .map(RichTextComponent::try_from_component_nbt)
                        .transpose()
                        .map_err(|detail| bbb_server_types::save::SaveError::InvalidNbt {
                            path: None,
                            detail,
                        })?,
                })
            })
            .collect::<bbb_server_types::save::SaveResult<Vec<_>>>()?;
        Ok(Self {
            dimension: map.dimension.clone(),
            center_x: map.center_x,
            center_z: map.center_z,
            scale: map.scale,
            colors: map.colors.clone(),
            tracking_position: map.tracking_position,
            unlimited_tracking: map.unlimited_tracking,
            locked: map.locked,
            banners,
            frames: map
                .frames
                .iter()
                .map(|frame| ServerMapFrame {
                    pos: BlockPos {
                        x: frame.pos[0],
                        y: frame.pos[1],
                        z: frame.pos[2],
                    },
                    rotation: frame.rotation,
                    entity_id: frame.entity_id,
                })
                .collect(),
        })
    }

    pub(super) fn to_saved(&self) -> bbb_server_persistence::Java26MapSavedData {
        bbb_server_persistence::Java26MapSavedData {
            dimension: self.dimension.clone(),
            center_x: self.center_x,
            center_z: self.center_z,
            scale: self.scale,
            colors: self.colors.clone(),
            tracking_position: self.tracking_position,
            unlimited_tracking: self.unlimited_tracking,
            locked: self.locked,
            banners: self
                .banners
                .iter()
                .map(|banner| bbb_server_persistence::Java26MapBanner {
                    pos: [banner.pos.x, banner.pos.y, banner.pos.z],
                    color: banner.color.clone(),
                    name: banner.name.as_ref().map(|name| name.component().clone()),
                })
                .collect(),
            frames: self
                .frames
                .iter()
                .map(|frame| bbb_server_persistence::Java26MapFrame {
                    pos: [frame.pos.x, frame.pos.y, frame.pos.z],
                    rotation: frame.rotation,
                    entity_id: frame.entity_id,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests;

fn fresh_map_center(origin: f64, scale: i8) -> i32 {
    let size = 128_i32.wrapping_mul(1_i32.wrapping_shl(scale as u32));
    let area = ((origin + 64.0) / f64::from(size)).floor() as i32;
    area.wrapping_mul(size)
        .wrapping_add(size / 2)
        .wrapping_sub(64)
}

// MapItem.java:203-217 samples Level.getBiome through BiomeManager, including unloaded terrain.
fn exploration_map_biomes(
    chunks: &ChunkStore,
    seed: i64,
    center_x: i32,
    center_z: i32,
    zoom: i8,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Result<[bool; MAP_PIXELS], String> {
    use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};
    const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
    const WATER_TAG: &str = "minecraft:water_on_map_outlines";
    let tags = chunks
        .registry_tags()
        .ok_or_else(|| "map preview requires level biome tags".to_owned())?;
    let scale = 1_i32.wrapping_shl(zoom as u32);
    let start_x = center_x.wrapping_div(scale).wrapping_sub(64);
    let start_z = center_z.wrapping_div(scale).wrapping_sub(64);
    let zoom_seed = biome_zoom_seed(seed);
    let mut watery = [false; MAP_PIXELS];
    for row in 0..MAP_SIZE {
        for column in 0..MAP_SIZE {
            let x = start_x.wrapping_add(column as i32).wrapping_mul(scale);
            let z = start_z.wrapping_add(row as i32).wrapping_mul(scale);
            let (qx, qy, qz) = fuzzy_biome_quart_position(zoom_seed, x, 0, z);
            let id = chunks
                .biome_id_at_quart_or_source(qx, qy, qz)
                .ok_or_else(|| format!("map preview biome unavailable at {x},0,{z}"))?;
            let resource = resolver
                .holder_resource_id(BIOME_REGISTRY, id)
                .ok_or_else(|| format!("map preview biome holder {id} is unpublished"))?;
            watery[row * MAP_SIZE + column] = tags.contains(BIOME_REGISTRY, WATER_TAG, &resource);
        }
    }
    Ok(watery)
}

impl super::ServerWorld {
    // MapItem.java:43-76: a fresh filled-map stack and one canonical SavedData allocation.
    pub(super) fn create_map(
        &mut self,
        dimension: impl Into<super::LevelId>,
        origin_x: i32,
        origin_z: i32,
        scale: i8,
        tracking_position: bool,
        unlimited_tracking: bool,
    ) -> bbb_server_types::Result<bbb_server_types::item_stack::RuntimeItemStack> {
        let dimension = dimension.into();
        let key = self.level_key(dimension).clone();
        self.maps.create_map(
            key.as_str(),
            origin_x,
            origin_z,
            scale,
            tracking_position,
            unlimited_tracking,
        )
    }
}

#[cfg(test)]
impl super::ServerWorld {
    pub(crate) fn install_map_for_test(
        &mut self,
        map_id: i32,
        data: &bbb_server_persistence::Java26MapSavedData,
        dirty: bool,
    ) -> std::result::Result<(), String> {
        let data = ServerMapPersistentData::from_saved(data).map_err(|error| error.to_string())?;
        let result = if dirty {
            self.maps.insert_new(map_id, data)
        } else {
            self.maps.insert_loaded(map_id, data)
        };
        result.map(|_| ()).map_err(|error| format!("{error:?}"))
    }

    pub(crate) fn update_map_color_for_test(
        &mut self,
        map_id: i32,
        x: usize,
        y: usize,
        color: u8,
    ) -> std::result::Result<bool, String> {
        self.maps
            .update_color(map_id, x, y, color)
            .map_err(|error| format!("{error:?}"))
    }

    pub(crate) fn map_admission_for_test(&mut self, map_id: i32) -> FramedMapAdmission {
        self.maps.admission(map_id)
    }

    pub(crate) fn is_map_loaded_in_memory_for_test(&self, map_id: i32) -> bool {
        self.maps.is_map_loaded_in_memory_for_test(map_id)
    }

    pub(crate) fn maps_are_clean_for_test(&self) -> bool {
        self.maps.dirty_persistent_snapshots().is_empty() && self.maps.dirty_index().is_none()
    }
}
