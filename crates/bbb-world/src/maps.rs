use std::{
    collections::BTreeMap,
    fmt,
    ops::Deref,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use bbb_protocol::packets::{
    MapColorPatch as ProtocolMapColorPatch, MapDecoration as ProtocolMapDecoration,
    MapItemData as ProtocolMapItemData,
};
use bbb_protocol::RichTextComponent;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::WorldStore;

const MAP_SIZE: usize = 128;
const MAP_COLOR_COUNT: usize = MAP_SIZE * MAP_SIZE;
static NEXT_MAP_COLOR_OWNER_GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct MapColors(Arc<MapColorsInner>);

#[derive(Clone)]
struct MapColorsInner {
    owner_generation: u64,
    content_revision: u64,
    colors: Vec<u8>,
}

impl MapColors {
    fn new(colors: Vec<u8>) -> Self {
        let owner_generation = NEXT_MAP_COLOR_OWNER_GENERATION.fetch_add(1, Ordering::Relaxed);
        assert_ne!(
            owner_generation,
            u64::MAX,
            "map color owner generation overflow"
        );
        Self(Arc::new(MapColorsInner {
            owner_generation,
            content_revision: 0,
            colors,
        }))
    }

    pub fn owner_generation(&self) -> u64 {
        self.0.owner_generation
    }

    pub fn content_revision(&self) -> u64 {
        self.0.content_revision
    }

    fn begin_update(&mut self) -> &mut Vec<u8> {
        let colors = Arc::make_mut(&mut self.0);
        colors.content_revision = colors
            .content_revision
            .checked_add(1)
            .expect("map content revision overflow");
        &mut colors.colors
    }

    #[cfg(test)]
    fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Default for MapColors {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl Deref for MapColors {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0.colors
    }
}

impl fmt::Debug for MapColors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.colors.fmt(formatter)
    }
}

impl PartialEq for MapColors {
    fn eq(&self, other: &Self) -> bool {
        self.0.colors == other.0.colors
    }
}

impl Eq for MapColors {}

impl Serialize for MapColors {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.colors.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MapColors {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<u8>::deserialize(deserializer).map(Self::new)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapItemState {
    pub id: i32,
    pub scale: i8,
    pub locked: bool,
    pub decorations: Vec<MapDecorationState>,
    pub colors: MapColors,
    pub last_color_patch: Option<MapColorPatchState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapDecorationState {
    pub type_id: i32,
    pub x: i8,
    pub y: i8,
    pub rot: u8,
    pub name: Option<RichTextComponent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapColorPatchState {
    pub start_x: u8,
    pub start_y: u8,
    pub width: u8,
    pub height: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastMapColorPatchState {
    pub map_id: i32,
    pub start_x: u8,
    pub start_y: u8,
    pub width: u8,
    pub height: u8,
}

impl WorldStore {
    pub fn apply_map_item_data(&mut self, packet: ProtocolMapItemData) -> bool {
        self.counters.map_item_data_packets += 1;
        if !self.preflight_packet_components(&packet, "map item data") {
            return false;
        }
        let map_id = packet.map_id;

        let map = self.maps.entry(map_id).or_insert_with(|| MapItemState {
            id: map_id,
            scale: packet.scale,
            locked: packet.locked,
            colors: MapColors::new(vec![0; MAP_COLOR_COUNT]),
            ..MapItemState::default()
        });

        if let Some(decorations) = packet.decorations {
            map.decorations = decorations.into_iter().map(map_decoration_state).collect();
        }

        let mut applied = true;
        let mut last_color_patch = None;
        if let Some(patch) = packet.color_patch {
            applied = apply_color_patch(map, patch);
            if applied {
                self.counters.map_color_patches_applied += 1;
                last_color_patch = map.last_color_patch.map(|patch| LastMapColorPatchState {
                    map_id,
                    start_x: patch.start_x,
                    start_y: patch.start_y,
                    width: patch.width,
                    height: patch.height,
                });
            } else {
                self.counters.map_color_patches_ignored += 1;
            }
        }

        if let Some(patch) = last_color_patch {
            self.last_map_color_patch = Some(patch);
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

    pub fn last_map_color_patch(&self) -> Option<&LastMapColorPatchState> {
        self.last_map_color_patch.as_ref()
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

    if width == 0 || height == 0 {
        map.last_color_patch = Some(MapColorPatchState {
            start_x: patch.start_x,
            start_y: patch.start_y,
            width: patch.width,
            height: patch.height,
        });
        return true;
    }

    let colors = map.colors.begin_update();
    if colors.len() != MAP_COLOR_COUNT {
        colors.resize(MAP_COLOR_COUNT, 0);
    }

    for x in 0..width {
        for y in 0..height {
            let src = x + y * width;
            let dst = (start_x + x) + (start_y + y) * MAP_SIZE;
            colors[dst] = patch.colors[src];
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
                name: Some(RichTextComponent::plain("Village")),
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
                name: Some(RichTextComponent::plain("Village")),
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
        assert_eq!(
            store.last_map_color_patch(),
            Some(&LastMapColorPatchState {
                map_id: 42,
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
    fn map_colors_share_publication_owner_and_revision_only_on_valid_patch() {
        let mut store = WorldStore::new();
        assert!(store.apply_map_item_data(ProtocolMapItemData {
            map_id: 9,
            scale: 0,
            locked: false,
            decorations: None,
            color_patch: None,
        }));
        let retained = store.clone();
        let retained_colors = &retained.map_item(9).unwrap().colors;
        let live_colors = &store.map_item(9).unwrap().colors;
        assert!(retained_colors.ptr_eq(live_colors));
        assert_eq!(live_colors.content_revision(), 0);

        assert!(!store.apply_map_item_data(ProtocolMapItemData {
            map_id: 9,
            scale: 0,
            locked: false,
            decorations: None,
            color_patch: Some(ProtocolMapColorPatch {
                start_x: 127,
                start_y: 127,
                width: 2,
                height: 1,
                colors: vec![1, 2],
            }),
        }));
        assert!(retained_colors.ptr_eq(&store.map_item(9).unwrap().colors));
        assert_eq!(store.map_item(9).unwrap().colors.content_revision(), 0);

        assert!(store.apply_map_item_data(ProtocolMapItemData {
            map_id: 9,
            scale: 0,
            locked: false,
            decorations: None,
            color_patch: Some(ProtocolMapColorPatch {
                start_x: 1,
                start_y: 2,
                width: 1,
                height: 1,
                colors: vec![17],
            }),
        }));
        let updated = &store.map_item(9).unwrap().colors;
        assert!(!retained_colors.ptr_eq(updated));
        assert_eq!(retained_colors.content_revision(), 0);
        assert_eq!(updated.content_revision(), 1);
        assert_eq!(retained_colors[1 + 2 * MAP_SIZE], 0);
        assert_eq!(updated[1 + 2 * MAP_SIZE], 17);
    }

    #[test]
    fn map_colors_keep_the_existing_serialized_array_shape() {
        let colors = MapColors::new(vec![1, 2, 3]);
        let json = serde_json::to_string(&colors).unwrap();
        assert_eq!(json, "[1,2,3]");
        let decoded: MapColors = serde_json::from_str(&json).unwrap();
        assert_eq!(&*decoded, &[1, 2, 3]);
        assert_eq!(decoded.content_revision(), 0);
        assert_ne!(decoded.owner_generation(), colors.owner_generation());
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
        assert_eq!(
            store.last_map_color_patch(),
            Some(&LastMapColorPatchState {
                map_id: 7,
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
            })
        );
        assert_eq!(store.counters().map_item_data_packets, 2);
    }

    #[test]
    fn map_item_data_rejects_invalid_patch_without_replacing_last_patch() {
        let mut store = WorldStore::new();
        assert!(store.apply_map_item_data(ProtocolMapItemData {
            map_id: 1,
            scale: 0,
            locked: false,
            decorations: None,
            color_patch: Some(ProtocolMapColorPatch {
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
                colors: vec![7],
            }),
        }));

        assert!(!store.apply_map_item_data(ProtocolMapItemData {
            map_id: 2,
            scale: 0,
            locked: false,
            decorations: None,
            color_patch: Some(ProtocolMapColorPatch {
                start_x: 127,
                start_y: 127,
                width: 2,
                height: 2,
                colors: vec![1, 2, 3, 4],
            }),
        }));

        assert_eq!(
            store.last_map_color_patch(),
            Some(&LastMapColorPatchState {
                map_id: 1,
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
            })
        );
        let counters = store.counters();
        assert_eq!(counters.map_color_patches_applied, 1);
        assert_eq!(counters.map_color_patches_ignored, 1);
    }
}
