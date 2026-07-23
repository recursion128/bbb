use bbb_protocol::{
    block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
    decode_nbt_summary, normalize_resource_location,
    packets::{StructureBlockMirror, StructureBlockMode, StructureBlockRotation},
    NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, Result, WorldStore};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureBlockEntityDataState {
    pub name: String,
    pub author: String,
    pub metadata: String,
    pub offset: BlockPos,
    pub size: StructureBlockSizeState,
    pub mirror: StructureBlockMirror,
    pub rotation: StructureBlockRotation,
    pub mode: StructureBlockMode,
    pub ignore_entities: bool,
    pub strict: bool,
    pub powered: bool,
    pub show_air: bool,
    pub show_bounding_box: bool,
    pub integrity_bits: u32,
    pub seed: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureBlockSizeState {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Default for StructureBlockEntityDataState {
    fn default() -> Self {
        Self {
            name: String::new(),
            author: String::new(),
            metadata: String::new(),
            offset: BlockPos { x: 0, y: 1, z: 0 },
            size: StructureBlockSizeState::default(),
            mirror: StructureBlockMirror::None,
            rotation: StructureBlockRotation::None,
            mode: StructureBlockMode::Data,
            ignore_entities: true,
            strict: false,
            powered: false,
            show_air: false,
            show_bounding_box: true,
            integrity_bits: 1.0_f32.to_bits(),
            seed: 0,
        }
    }
}

impl StructureBlockEntityDataState {
    pub fn integrity(&self) -> f32 {
        f32::from_bits(self.integrity_bits)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureBlockEditorState {
    pub pos: BlockPos,
    pub data: StructureBlockEntityDataState,
    #[serde(default)]
    pub authoritative_revision: usize,
}

pub(crate) fn decode_structure_block_entity_data(
    raw_nbt: &[u8],
) -> Result<StructureBlockEntityDataState> {
    let Some(root) = decode_nbt_summary(raw_nbt)? else {
        return Ok(StructureBlockEntityDataState::default());
    };
    let NbtSummaryValue::Compound(entries) = root else {
        return Err(bbb_protocol::codec::ProtocolError::InvalidData(
            "structure block entity root is not a compound".to_string(),
        )
        .into());
    };

    let defaults = StructureBlockEntityDataState::default();
    let name = nbt_string(&entries, "name")
        .filter(|name| !name.is_empty())
        .and_then(|name| normalize_resource_location(name).ok())
        .unwrap_or_default();
    Ok(StructureBlockEntityDataState {
        name,
        author: nbt_string(&entries, "author").unwrap_or_default(),
        metadata: nbt_string(&entries, "metadata").unwrap_or_default(),
        offset: BlockPos {
            x: nbt_int(&entries, "posX")
                .unwrap_or(defaults.offset.x)
                .clamp(-48, 48),
            y: nbt_int(&entries, "posY")
                .unwrap_or(defaults.offset.y)
                .clamp(-48, 48),
            z: nbt_int(&entries, "posZ")
                .unwrap_or(defaults.offset.z)
                .clamp(-48, 48),
        },
        size: StructureBlockSizeState {
            x: nbt_int(&entries, "sizeX")
                .unwrap_or(defaults.size.x)
                .clamp(0, 48),
            y: nbt_int(&entries, "sizeY")
                .unwrap_or(defaults.size.y)
                .clamp(0, 48),
            z: nbt_int(&entries, "sizeZ")
                .unwrap_or(defaults.size.z)
                .clamp(0, 48),
        },
        mirror: match nbt_string(&entries, "mirror").as_deref() {
            Some("LEFT_RIGHT") => StructureBlockMirror::LeftRight,
            Some("FRONT_BACK") => StructureBlockMirror::FrontBack,
            _ => StructureBlockMirror::None,
        },
        rotation: match nbt_string(&entries, "rotation").as_deref() {
            Some("CLOCKWISE_90") => StructureBlockRotation::Clockwise90,
            Some("CLOCKWISE_180") => StructureBlockRotation::Clockwise180,
            Some("COUNTERCLOCKWISE_90") => StructureBlockRotation::Counterclockwise90,
            _ => StructureBlockRotation::None,
        },
        mode: match nbt_string(&entries, "mode").as_deref() {
            Some("SAVE") => StructureBlockMode::Save,
            Some("LOAD") => StructureBlockMode::Load,
            Some("CORNER") => StructureBlockMode::Corner,
            _ => StructureBlockMode::Data,
        },
        ignore_entities: nbt_bool(&entries, "ignoreEntities").unwrap_or(defaults.ignore_entities),
        strict: nbt_bool(&entries, "strict").unwrap_or(defaults.strict),
        powered: nbt_bool(&entries, "powered").unwrap_or(defaults.powered),
        show_air: nbt_bool(&entries, "showair").unwrap_or(defaults.show_air),
        show_bounding_box: nbt_bool(&entries, "showboundingbox")
            .unwrap_or(defaults.show_bounding_box),
        integrity_bits: nbt_float(&entries, "integrity")
            .unwrap_or_else(|| defaults.integrity())
            .to_bits(),
        seed: nbt_long(&entries, "seed").unwrap_or(defaults.seed),
    })
}

impl WorldStore {
    pub fn structure_block_data_at(&self, pos: BlockPos) -> Option<&StructureBlockEntityDataState> {
        let record = self.block_entity_record_at(pos)?;
        (record.type_id == VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID)
            .then_some(record.structure_block.as_ref())
            .flatten()
    }

    pub fn structure_block_editor(&self) -> Option<&StructureBlockEditorState> {
        self.client_ui.current_structure_block_editor.as_ref()
    }

    pub fn structure_block_editor_open_count(&self) -> usize {
        self.client_ui.structure_block_editor_open_count
    }

    pub fn try_open_structure_block_editor(&mut self, pos: BlockPos) -> bool {
        if !self.local_player_has_gamemaster_permission()
            || !self.structure_block_is_present_at(pos)
        {
            return false;
        }
        let Some(data) = self.structure_block_data_at(pos).cloned() else {
            return false;
        };
        self.client_ui.replace_screen();
        self.client_ui.structure_block_editor_open_count += 1;
        self.client_ui.current_structure_block_editor = Some(StructureBlockEditorState {
            pos,
            data,
            authoritative_revision: 0,
        });
        true
    }

    pub fn close_structure_block_editor(&mut self) -> bool {
        self.client_ui
            .current_structure_block_editor
            .take()
            .is_some()
    }

    pub(crate) fn refresh_structure_block_editor_from_authoritative_data(&mut self, pos: BlockPos) {
        if self
            .client_ui
            .current_structure_block_editor
            .as_ref()
            .is_none_or(|editor| editor.pos != pos)
        {
            return;
        }
        if !self.structure_block_is_present_at(pos) {
            self.close_structure_block_editor();
            return;
        }
        let Some(data) = self.structure_block_data_at(pos).cloned() else {
            self.close_structure_block_editor();
            return;
        };
        if let Some(editor) = &mut self.client_ui.current_structure_block_editor {
            editor.data = data;
            editor.authoritative_revision = editor.authoritative_revision.saturating_add(1);
        }
    }

    pub(crate) fn reconcile_structure_block_editor_after_block_change(&mut self, pos: BlockPos) {
        if self
            .client_ui
            .current_structure_block_editor
            .as_ref()
            .is_some_and(|editor| editor.pos == pos)
            && (!self.structure_block_is_present_at(pos)
                || self.structure_block_data_at(pos).is_none())
        {
            self.close_structure_block_editor();
        }
    }

    pub(crate) fn close_structure_block_editor_in_chunk(&mut self, chunk: crate::ChunkPos) {
        if self
            .client_ui
            .current_structure_block_editor
            .as_ref()
            .is_some_and(|editor| {
                editor.pos.x.div_euclid(16) == chunk.x && editor.pos.z.div_euclid(16) == chunk.z
            })
        {
            self.close_structure_block_editor();
        }
    }

    fn structure_block_is_present_at(&self, pos: BlockPos) -> bool {
        self.probe_block(pos)
            .and_then(|probe| probe.block_name)
            .is_some_and(|name| name == "minecraft:structure_block")
    }
}

fn entry<'a>(entries: &'a [NbtSummaryEntry], key: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
}

fn nbt_string(entries: &[NbtSummaryEntry], key: &str) -> Option<String> {
    let NbtSummaryValue::String(value) = entry(entries, key)? else {
        return None;
    };
    Some(value.clone())
}

fn nbt_int(entries: &[NbtSummaryEntry], key: &str) -> Option<i32> {
    let NbtSummaryValue::Int(value) = entry(entries, key)? else {
        return None;
    };
    Some(*value)
}

fn nbt_long(entries: &[NbtSummaryEntry], key: &str) -> Option<i64> {
    let NbtSummaryValue::Long(value) = entry(entries, key)? else {
        return None;
    };
    Some(*value)
}

fn nbt_float(entries: &[NbtSummaryEntry], key: &str) -> Option<f32> {
    let NbtSummaryValue::Float(bits) = entry(entries, key)? else {
        return None;
    };
    Some(f32::from_bits(*bits))
}

fn nbt_bool(entries: &[NbtSummaryEntry], key: &str) -> Option<bool> {
    let NbtSummaryValue::Byte(value) = entry(entries, key)? else {
        return None;
    };
    Some(*value != 0)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::{
        block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
        packets::{BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate, EntityEvent},
    };

    use super::*;
    use crate::{
        BlockEntityRecord, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };

    #[test]
    fn decodes_vanilla_structure_fields_bounds_and_defaults() {
        let decoded = decode_structure_block_entity_data(&structure_block_nbt()).unwrap();
        assert_eq!(decoded.name, "example:house");
        assert_eq!(decoded.author, "Builder");
        assert_eq!(decoded.metadata, "custom data");
        assert_eq!(
            decoded.offset,
            BlockPos {
                x: -48,
                y: 2,
                z: 48
            }
        );
        assert_eq!(decoded.size, StructureBlockSizeState { x: 0, y: 12, z: 48 });
        assert_eq!(decoded.mirror, StructureBlockMirror::FrontBack);
        assert_eq!(decoded.rotation, StructureBlockRotation::Counterclockwise90);
        assert_eq!(decoded.mode, StructureBlockMode::Load);
        assert!(!decoded.ignore_entities);
        assert!(decoded.strict);
        assert!(decoded.powered);
        assert!(decoded.show_air);
        assert!(!decoded.show_bounding_box);
        assert_eq!(decoded.integrity(), 1.5);
        assert_eq!(decoded.seed, i64::MIN);

        assert_eq!(
            decode_structure_block_entity_data(&[0]).unwrap(),
            StructureBlockEntityDataState::default()
        );
        let wrong_types = vec![10, 3, 0, 4, b'm', b'o', b'd', b'e', 0, 0, 0, 7, 0];
        assert_eq!(
            decode_structure_block_entity_data(&wrong_types)
                .unwrap()
                .mode,
            StructureBlockMode::Data
        );
    }

    #[test]
    fn editor_requires_permission_refreshes_and_rejects_bad_root_atomically() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut store = structure_block_world(pos);
        assert!(!store.try_open_structure_block_editor(pos));

        store.local_player_id = Some(11);
        assert!(store.apply_entity_event(EntityEvent {
            entity_id: 11,
            event_id: 26,
        }));
        assert!(store.try_open_structure_block_editor(pos));
        assert_eq!(store.structure_block_editor_open_count(), 1);
        assert_eq!(
            store.structure_block_editor().unwrap().data.name,
            "example:house"
        );

        let mut updated = structure_block_nbt();
        replace_ascii(&mut updated, b"custom data", b"server data");
        assert!(store
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
                raw_nbt: updated,
            })
            .unwrap());
        let editor = store.structure_block_editor().unwrap();
        assert_eq!(editor.authoritative_revision, 1);
        assert_eq!(editor.data.metadata, "server data");

        let original = store.structure_block_data_at(pos).cloned().unwrap();
        assert!(store
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
                raw_nbt: vec![3, 0, 0, 0, 1],
            })
            .is_err());
        assert_eq!(store.structure_block_data_at(pos), Some(&original));

        let stone = crate::vanilla_block_state_id("minecraft:stone", &BTreeMap::new()).unwrap();
        assert!(store.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: stone,
        }));
        assert!(store.structure_block_editor().is_none());
    }

    #[test]
    fn editor_closes_on_chunk_forget_and_level_clear() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut store = structure_block_world(pos);
        grant_permission(&mut store);
        assert!(store.try_open_structure_block_editor(pos));
        assert!(store.forget_chunk(ChunkPos { x: 0, z: 0 }));
        assert!(store.structure_block_editor().is_none());

        let mut store = structure_block_world(pos);
        grant_permission(&mut store);
        assert!(store.try_open_structure_block_editor(pos));
        store.clear_client_level();
        assert!(store.structure_block_editor().is_none());
    }

    fn grant_permission(store: &mut WorldStore) {
        store.local_player_id = Some(11);
        assert!(store.apply_entity_event(EntityEvent {
            entity_id: 11,
            event_id: 26,
        }));
    }

    fn structure_block_world(pos: BlockPos) -> WorldStore {
        let block_state_id = crate::vanilla_block_state_id(
            "minecraft:structure_block",
            &BTreeMap::from([("mode".to_string(), "load".to_string())]),
        )
        .unwrap();
        let mut record = BlockEntityRecord::empty(
            pos.x.rem_euclid(16) as u8,
            i16::try_from(pos.y).unwrap(),
            pos.z.rem_euclid(16) as u8,
            VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
        );
        record.raw_nbt = structure_block_nbt();
        record.structure_block = Some(
            decode_structure_block_entity_data(&record.raw_nbt)
                .expect("valid structure-block fixture"),
        );

        let mut store = WorldStore::new();
        store.dimension = WorldDimension {
            min_y: 0,
            height: 16,
        };
        store.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 4096,
                fluid_count: 0,
                block_states: single_value_palette(
                    PaletteDomain::BlockStates,
                    block_state_id,
                    4096,
                ),
                biomes: single_value_palette(PaletteDomain::Biomes, 0, 64),
            }],
            block_entities: vec![record],
            light: LightData::default(),
        });
        store
    }

    fn single_value_palette(
        domain: PaletteDomain,
        global_id: i32,
        entry_count: usize,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn structure_block_nbt() -> Vec<u8> {
        let mut out = vec![10];
        push_string(&mut out, "name", "example:house");
        push_string(&mut out, "author", "Builder");
        push_string(&mut out, "metadata", "custom data");
        push_int(&mut out, "posX", -60);
        push_int(&mut out, "posY", 2);
        push_int(&mut out, "posZ", 70);
        push_int(&mut out, "sizeX", -3);
        push_int(&mut out, "sizeY", 12);
        push_int(&mut out, "sizeZ", 80);
        push_string(&mut out, "mirror", "FRONT_BACK");
        push_string(&mut out, "rotation", "COUNTERCLOCKWISE_90");
        push_string(&mut out, "mode", "LOAD");
        push_byte(&mut out, "ignoreEntities", 0);
        push_byte(&mut out, "strict", 1);
        push_byte(&mut out, "powered", 1);
        push_byte(&mut out, "showair", 1);
        push_byte(&mut out, "showboundingbox", 0);
        push_float(&mut out, "integrity", 1.5);
        push_long(&mut out, "seed", i64::MIN);
        out.push(0);
        out
    }

    fn replace_ascii(bytes: &mut [u8], old: &[u8], new: &[u8]) {
        assert_eq!(old.len(), new.len());
        let start = bytes
            .windows(old.len())
            .position(|value| value == old)
            .unwrap();
        bytes[start..start + old.len()].copy_from_slice(new);
    }

    fn push_byte(out: &mut Vec<u8>, name: &str, value: i8) {
        push_tag_name(out, 1, name);
        out.push(value as u8);
    }

    fn push_int(out: &mut Vec<u8>, name: &str, value: i32) {
        push_tag_name(out, 3, name);
        out.extend_from_slice(&value.to_be_bytes());
    }

    fn push_long(out: &mut Vec<u8>, name: &str, value: i64) {
        push_tag_name(out, 4, name);
        out.extend_from_slice(&value.to_be_bytes());
    }

    fn push_float(out: &mut Vec<u8>, name: &str, value: f32) {
        push_tag_name(out, 5, name);
        out.extend_from_slice(&value.to_bits().to_be_bytes());
    }

    fn push_string(out: &mut Vec<u8>, name: &str, value: &str) {
        push_tag_name(out, 8, name);
        push_nbt_string(out, value);
    }

    fn push_tag_name(out: &mut Vec<u8>, tag: u8, name: &str) {
        out.push(tag);
        push_nbt_string(out, name);
    }

    fn push_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
