use bbb_protocol::{
    block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID, decode_nbt_summary,
    packets::CommandBlockMode, NbtSummaryEntry, NbtSummaryValue, RichTextComponent,
};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, Result, WorldStore};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandBlockEntityDataState {
    pub command: String,
    pub success_count: i32,
    pub custom_name: Option<RichTextComponent>,
    pub track_output: bool,
    pub last_output: Option<RichTextComponent>,
    pub update_last_execution: bool,
    pub last_execution: Option<i64>,
    pub powered: bool,
    pub condition_met: bool,
    pub automatic: bool,
}

impl Default for CommandBlockEntityDataState {
    fn default() -> Self {
        Self {
            command: String::new(),
            success_count: 0,
            custom_name: None,
            track_output: true,
            last_output: None,
            update_last_execution: true,
            last_execution: None,
            powered: false,
            condition_met: false,
            automatic: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandBlockEditorState {
    pub pos: BlockPos,
    pub data: CommandBlockEntityDataState,
    pub mode: CommandBlockMode,
    pub conditional: bool,
    pub controls_enabled: bool,
    #[serde(default)]
    pub authoritative_revision: usize,
}

pub(crate) fn decode_command_block_entity_data(
    raw_nbt: &[u8],
) -> Result<CommandBlockEntityDataState> {
    if raw_nbt.is_empty() {
        return Ok(CommandBlockEntityDataState::default());
    }
    let Some(NbtSummaryValue::Compound(entries)) = decode_nbt_summary(raw_nbt)? else {
        return Ok(CommandBlockEntityDataState::default());
    };

    let command = nbt_string(&entries, "Command").unwrap_or_default();
    let success_count = nbt_int(&entries, "SuccessCount").unwrap_or_default();
    let custom_name = component(&entries, "CustomName");
    let track_output = nbt_bool(&entries, "TrackOutput").unwrap_or(true);
    let last_output = track_output
        .then(|| component(&entries, "LastOutput"))
        .flatten();
    let update_last_execution = nbt_bool(&entries, "UpdateLastExecution").unwrap_or(true);
    let last_execution = update_last_execution
        .then(|| nbt_long(&entries, "LastExecution"))
        .flatten()
        .filter(|value| *value != -1);

    Ok(CommandBlockEntityDataState {
        command,
        success_count,
        custom_name,
        track_output,
        last_output,
        update_last_execution,
        last_execution,
        powered: nbt_bool(&entries, "powered").unwrap_or(false),
        condition_met: nbt_bool(&entries, "conditionMet").unwrap_or(false),
        automatic: nbt_bool(&entries, "auto").unwrap_or(false),
    })
}

impl WorldStore {
    pub fn command_block_data_at(&self, pos: BlockPos) -> Option<&CommandBlockEntityDataState> {
        let record = self.block_entity_record_at(pos)?;
        (record.type_id == VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID)
            .then_some(record.command_block.as_ref())
            .flatten()
    }

    pub fn command_block_editor(&self) -> Option<&CommandBlockEditorState> {
        self.client_ui.current_command_block_editor.as_ref()
    }

    pub fn command_block_editor_open_count(&self) -> usize {
        self.client_ui.command_block_editor_open_count
    }

    pub fn try_open_command_block_editor(&mut self, pos: BlockPos) -> bool {
        if !self.local_player_can_use_gamemaster_blocks() {
            return false;
        }
        let Some((mode, conditional)) = self.command_block_mode_at(pos) else {
            return false;
        };
        let Some(data) = self.command_block_data_at(pos).cloned() else {
            return false;
        };
        self.client_ui.replace_screen();
        self.client_ui.command_block_editor_open_count += 1;
        self.client_ui.current_command_block_editor = Some(CommandBlockEditorState {
            pos,
            data,
            mode,
            conditional,
            controls_enabled: false,
            authoritative_revision: 0,
        });
        true
    }

    pub fn close_command_block_editor(&mut self) -> bool {
        self.client_ui.current_command_block_editor.take().is_some()
    }

    pub fn set_command_block_editor_track_output(&mut self, track_output: bool) -> bool {
        let Some(pos) = self.command_block_editor().map(|editor| editor.pos) else {
            return false;
        };
        let Some(data) = self.command_block_data_at_mut(pos) else {
            return false;
        };
        data.track_output = track_output;
        if let Some(editor) = &mut self.client_ui.current_command_block_editor {
            editor.data.track_output = track_output;
        }
        true
    }

    pub fn clear_command_block_editor_last_output(&mut self) -> bool {
        let Some(pos) = self.command_block_editor().map(|editor| editor.pos) else {
            return false;
        };
        let Some(data) = self.command_block_data_at_mut(pos) else {
            return false;
        };
        data.last_output = None;
        if let Some(editor) = &mut self.client_ui.current_command_block_editor {
            editor.data.last_output = None;
        }
        true
    }

    pub(crate) fn refresh_command_block_editor_from_authoritative_data(&mut self, pos: BlockPos) {
        if self
            .client_ui
            .current_command_block_editor
            .as_ref()
            .is_none_or(|editor| editor.pos != pos)
        {
            return;
        }
        let Some((mode, conditional)) = self.command_block_mode_at(pos) else {
            self.close_command_block_editor();
            return;
        };
        let Some(data) = self.command_block_data_at(pos).cloned() else {
            self.close_command_block_editor();
            return;
        };
        if let Some(editor) = &mut self.client_ui.current_command_block_editor {
            editor.data = data;
            editor.mode = mode;
            editor.conditional = conditional;
            editor.controls_enabled = true;
            editor.authoritative_revision = editor.authoritative_revision.saturating_add(1);
        }
    }

    pub(crate) fn reconcile_command_block_editor_after_block_change(&mut self, pos: BlockPos) {
        if self
            .client_ui
            .current_command_block_editor
            .as_ref()
            .is_none_or(|editor| editor.pos != pos)
        {
            return;
        }
        let Some((mode, conditional)) = self.command_block_mode_at(pos) else {
            self.close_command_block_editor();
            return;
        };
        if let Some(editor) = &mut self.client_ui.current_command_block_editor {
            if editor.mode != mode || editor.conditional != conditional {
                editor.mode = mode;
                editor.conditional = conditional;
                editor.authoritative_revision = editor.authoritative_revision.saturating_add(1);
            }
        }
    }

    pub(crate) fn close_command_block_editor_in_chunk(&mut self, chunk: crate::ChunkPos) {
        if self
            .client_ui
            .current_command_block_editor
            .as_ref()
            .is_some_and(|editor| {
                editor.pos.x.div_euclid(16) == chunk.x && editor.pos.z.div_euclid(16) == chunk.z
            })
        {
            self.close_command_block_editor();
        }
    }

    fn command_block_mode_at(&self, pos: BlockPos) -> Option<(CommandBlockMode, bool)> {
        let probe = self.probe_block(pos)?;
        let mode = match probe.block_name.as_deref()? {
            "minecraft:chain_command_block" => CommandBlockMode::Sequence,
            "minecraft:repeating_command_block" => CommandBlockMode::Auto,
            "minecraft:command_block" => CommandBlockMode::Redstone,
            _ => return None,
        };
        let conditional = probe
            .block_properties
            .get("conditional")
            .is_some_and(|value| value == "true");
        Some((mode, conditional))
    }

    fn command_block_data_at_mut(
        &mut self,
        pos: BlockPos,
    ) -> Option<&mut CommandBlockEntityDataState> {
        let chunk = self.chunks.iter_mut().find(|chunk| {
            chunk.pos.x == pos.x.div_euclid(16) && chunk.pos.z == pos.z.div_euclid(16)
        })?;
        let local_x = pos.x.rem_euclid(16) as u8;
        let local_z = pos.z.rem_euclid(16) as u8;
        let y = i16::try_from(pos.y).ok()?;
        let record = chunk.block_entities.iter_mut().find(|record| {
            record.local_x == local_x && record.y == y && record.local_z == local_z
        })?;
        (record.type_id == VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID)
            .then_some(record.command_block.as_mut())
            .flatten()
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

fn nbt_bool(entries: &[NbtSummaryEntry], key: &str) -> Option<bool> {
    let NbtSummaryValue::Byte(value) = entry(entries, key)? else {
        return None;
    };
    Some(*value != 0)
}

fn component(entries: &[NbtSummaryEntry], key: &str) -> Option<RichTextComponent> {
    RichTextComponent::try_from_component_nbt(entry(entries, key)?.clone()).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::{
        block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
        packets::{BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate, EntityEvent},
    };

    use super::*;
    use crate::{
        BlockEntityRecord, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };

    #[test]
    fn decodes_vanilla_command_block_fields_and_defaults() {
        let decoded =
            decode_command_block_entity_data(&command_block_nbt("say hello", true, true)).unwrap();

        assert_eq!(decoded.command, "say hello");
        assert_eq!(decoded.success_count, 7);
        assert_eq!(decoded.custom_name.as_ref().unwrap().text(), "Builder");
        assert!(decoded.track_output);
        assert_eq!(decoded.last_output.as_ref().unwrap().text(), "done");
        assert!(decoded.update_last_execution);
        assert_eq!(decoded.last_execution, Some(42));
        assert!(decoded.powered);
        assert!(!decoded.condition_met);
        assert!(decoded.automatic);

        let defaults = decode_command_block_entity_data(&[0]).unwrap();
        assert_eq!(defaults, CommandBlockEntityDataState::default());

        let not_tracking =
            decode_command_block_entity_data(&command_block_nbt("say quiet", false, false))
                .unwrap();
        assert!(!not_tracking.track_output);
        assert_eq!(not_tracking.last_output, None);
        assert!(!not_tracking.update_last_execution);
        assert_eq!(not_tracking.last_execution, None);
    }

    #[test]
    fn editor_requires_permission_refreshes_from_server_and_tracks_block_mode() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut store = command_block_world(pos, "say initial");

        assert!(!store.try_open_command_block_editor(pos));
        grant_gamemaster_block_access(&mut store, 7);
        assert!(store.try_open_command_block_editor(pos));
        assert_eq!(store.command_block_editor_open_count(), 1);
        let editor = store.command_block_editor().unwrap();
        assert_eq!(editor.mode, CommandBlockMode::Redstone);
        assert!(!editor.conditional);
        assert!(!editor.controls_enabled);
        assert_eq!(editor.data.command, "say initial");

        assert!(store
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
                raw_nbt: command_block_nbt("say authoritative", true, true),
            })
            .unwrap());
        let editor = store.command_block_editor().unwrap();
        assert!(editor.controls_enabled);
        assert_eq!(editor.authoritative_revision, 1);
        assert_eq!(editor.data.command, "say authoritative");
        assert_eq!(
            store.command_block_data_at(pos).unwrap().last_execution,
            Some(42)
        );
        assert!(store.set_command_block_editor_track_output(false));
        assert!(!store.command_block_data_at(pos).unwrap().track_output);
        assert!(store
            .command_block_data_at(pos)
            .unwrap()
            .last_output
            .is_some());
        assert!(store.clear_command_block_editor_last_output());
        assert_eq!(store.command_block_data_at(pos).unwrap().last_output, None);

        let repeating = command_block_state_id("minecraft:repeating_command_block", "north", true);
        assert!(store.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: repeating,
        }));
        let editor = store.command_block_editor().unwrap();
        assert_eq!(editor.mode, CommandBlockMode::Auto);
        assert!(editor.conditional);
        assert_eq!(editor.authoritative_revision, 2);
        assert_eq!(editor.data.command, "say authoritative");

        let stone = crate::vanilla_block_state_id("minecraft:stone", &BTreeMap::new()).unwrap();
        assert!(store.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: stone,
        }));
        assert!(store.command_block_editor().is_none());
    }

    #[test]
    fn forgetting_editor_chunk_closes_editor() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut store = command_block_world(pos, "say initial");
        grant_gamemaster_block_access(&mut store, 7);
        assert!(store.try_open_command_block_editor(pos));

        assert!(store.forget_chunk(ChunkPos { x: 0, z: 0 }));
        assert!(store.command_block_editor().is_none());
    }

    fn command_block_world(pos: BlockPos, command: &str) -> WorldStore {
        let block_state_id = command_block_state_id("minecraft:command_block", "north", false);
        let mut record = BlockEntityRecord::empty(
            pos.x.rem_euclid(16) as u8,
            i16::try_from(pos.y).unwrap(),
            pos.z.rem_euclid(16) as u8,
            VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
        );
        record.command_block = Some(CommandBlockEntityDataState {
            command: command.to_string(),
            ..CommandBlockEntityDataState::default()
        });
        record.raw_nbt = vec![0];

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

    fn grant_gamemaster_block_access(store: &mut WorldStore, entity_id: i32) {
        store.local_player_id = Some(entity_id);
        assert!(store.apply_entity_event(EntityEvent {
            entity_id,
            event_id: 26,
        }));
        store.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
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

    fn command_block_state_id(name: &str, facing: &str, conditional: bool) -> i32 {
        crate::vanilla_block_state_id(
            name,
            &BTreeMap::from([
                ("conditional".to_string(), conditional.to_string()),
                ("facing".to_string(), facing.to_string()),
            ]),
        )
        .unwrap()
    }

    fn command_block_nbt(
        command: &str,
        track_output: bool,
        update_last_execution: bool,
    ) -> Vec<u8> {
        let mut out = vec![10];
        push_string(&mut out, "Command", command);
        push_int(&mut out, "SuccessCount", 7);
        push_component(&mut out, "CustomName", "Builder");
        push_byte(&mut out, "TrackOutput", i8::from(track_output));
        push_string(&mut out, "LastOutput", "done");
        push_byte(
            &mut out,
            "UpdateLastExecution",
            i8::from(update_last_execution),
        );
        push_long(&mut out, "LastExecution", 42);
        push_byte(&mut out, "powered", 1);
        push_byte(&mut out, "conditionMet", 0);
        push_byte(&mut out, "auto", 1);
        out.push(0);
        out
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

    fn push_string(out: &mut Vec<u8>, name: &str, value: &str) {
        push_tag_name(out, 8, name);
        push_nbt_string(out, value);
    }

    fn push_component(out: &mut Vec<u8>, name: &str, text: &str) {
        push_tag_name(out, 10, name);
        push_string(out, "text", text);
        out.push(0);
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
