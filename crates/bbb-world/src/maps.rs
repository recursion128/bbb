use std::collections::BTreeMap;

use bbb_protocol::packets::{
    MapColorPatch as ProtocolMapColorPatch, MapDecoration as ProtocolMapDecoration,
    MapItemData as ProtocolMapItemData,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

const MAP_SIZE: usize = 128;
const MAP_COLOR_COUNT: usize = MAP_SIZE * MAP_SIZE;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapItemState {
    pub id: i32,
    pub scale: i8,
    pub locked: bool,
    pub decorations: Vec<MapDecorationState>,
    pub colors: Vec<u8>,
    pub last_color_patch: Option<MapColorPatchState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapDecorationState {
    pub type_id: i32,
    pub x: i8,
    pub y: i8,
    pub rot: u8,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapColorPatchState {
    pub start_x: u8,
    pub start_y: u8,
    pub width: u8,
    pub height: u8,
}

impl WorldStore {
    pub fn apply_map_item_data(&mut self, packet: ProtocolMapItemData) -> bool {
        self.counters.map_item_data_packets += 1;

        let map = self
            .maps
            .entry(packet.map_id)
            .or_insert_with(|| MapItemState {
                id: packet.map_id,
                scale: packet.scale,
                locked: packet.locked,
                colors: vec![0; MAP_COLOR_COUNT],
                ..MapItemState::default()
            });

        if let Some(decorations) = packet.decorations {
            map.decorations = decorations.into_iter().map(map_decoration_state).collect();
        }

        let mut applied = true;
        if let Some(patch) = packet.color_patch {
            applied = apply_color_patch(map, patch);
            if applied {
                self.counters.map_color_patches_applied += 1;
            } else {
                self.counters.map_color_patches_ignored += 1;
            }
        }

        self.counters.maps_tracked = self.maps.len();
        self.counters.map_decorations_tracked =
            self.maps.values().map(|map| map.decorations.len()).sum();
        applied
    }

    pub fn map_item(&self, id: i32) -> Option<&MapItemState> {
        self.maps.get(&id)
    }

    pub fn map_items(&self) -> &BTreeMap<i32, MapItemState> {
        &self.maps
    }
}

fn map_decoration_state(decoration: ProtocolMapDecoration) -> MapDecorationState {
    MapDecorationState {
        type_id: decoration.type_id,
        x: decoration.x,
        y: decoration.y,
        rot: decoration.rot,
        name: decoration.name,
    }
}

fn apply_color_patch(map: &mut MapItemState, patch: ProtocolMapColorPatch) -> bool {
    let width = usize::from(patch.width);
    let height = usize::from(patch.height);
    let start_x = usize::from(patch.start_x);
    let start_y = usize::from(patch.start_y);
    let expected_len = width * height;
    if patch.colors.len() != expected_len
        || start_x + width > MAP_SIZE
        || start_y + height > MAP_SIZE
    {
        return false;
    }

    if map.colors.len() != MAP_COLOR_COUNT {
        map.colors.resize(MAP_COLOR_COUNT, 0);
    }

    for x in 0..width {
        for y in 0..height {
            let src = x + y * width;
            let dst = (start_x + x) + (start_y + y) * MAP_SIZE;
            map.colors[dst] = patch.colors[src];
        }
    }
    map.last_color_patch = Some(MapColorPatchState {
        start_x: patch.start_x,
        start_y: patch.start_y,
        width: patch.width,
        height: patch.height,
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_item_data_creates_and_updates_map_state() {
        let mut store = WorldStore::new();

        assert!(store.apply_map_item_data(ProtocolMapItemData {
            map_id: 42,
            scale: 2,
            locked: true,
            decorations: Some(vec![ProtocolMapDecoration {
                type_id: 4,
                x: -20,
                y: 30,
                rot: 7,
                name: Some("Village".to_string()),
            }]),
            color_patch: Some(ProtocolMapColorPatch {
                start_x: 3,
                start_y: 4,
                width: 2,
                height: 2,
                colors: vec![1, 2, 3, 4],
            }),
        }));

        let map = store.map_item(42).expect("map is tracked");
        assert_eq!(map.scale, 2);
        assert!(map.locked);
        assert_eq!(
            map.decorations,
            vec![MapDecorationState {
                type_id: 4,
                x: -20,
                y: 30,
                rot: 7,
                name: Some("Village".to_string()),
            }]
        );
        assert_eq!(map.colors[3 + 4 * MAP_SIZE], 1);
        assert_eq!(map.colors[4 + 4 * MAP_SIZE], 2);
        assert_eq!(map.colors[3 + 5 * MAP_SIZE], 3);
        assert_eq!(map.colors[4 + 5 * MAP_SIZE], 4);
        assert_eq!(
            map.last_color_patch,
            Some(MapColorPatchState {
                start_x: 3,
                start_y: 4,
                width: 2,
                height: 2,
            })
        );
        let counters = store.counters();
        assert_eq!(counters.map_item_data_packets, 1);
        assert_eq!(counters.maps_tracked, 1);
        assert_eq!(counters.map_decorations_tracked, 1);
        assert_eq!(counters.map_color_patches_applied, 1);
    }

    #[test]
    fn map_item_data_absent_optionals_preserve_existing_sections() {
        let mut store = WorldStore::new();
        store.apply_map_item_data(ProtocolMapItemData {
            map_id: 7,
            scale: 1,
            locked: false,
            decorations: Some(vec![ProtocolMapDecoration {
                type_id: 0,
                x: 1,
                y: 2,
                rot: 3,
                name: None,
            }]),
            color_patch: Some(ProtocolMapColorPatch {
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
                colors: vec![9],
            }),
        });

        store.apply_map_item_data(ProtocolMapItemData {
            map_id: 7,
            scale: 4,
            locked: true,
            decorations: None,
            color_patch: None,
        });

        let map = store.map_item(7).expect("map is tracked");
        assert_eq!(map.scale, 1);
        assert!(!map.locked);
        assert_eq!(map.decorations.len(), 1);
        assert_eq!(map.colors[0], 9);
        assert_eq!(store.counters().map_item_data_packets, 2);
    }
}
