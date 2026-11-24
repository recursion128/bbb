use crate::command_block_execution::{
    begin_command_block_execution, finish_command_block_execution, CommandBlockExecutionPlan,
};
pub use crate::command_block_execution::{CommandBlockExecutionSnapshot, NO_LAST_EXECUTION};
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID, VANILLA_ENTITY_DATA_SERIALIZER_STRING_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
    VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID,
};
use bbb_protocol::nbt_summary::{compound_last_field as field, entry};
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, Vec3d};
use bbb_protocol::{
    component_appended_nbt, component_hover_uuid_nbt, component_translatable_nbt,
    ComponentHoverEvent, ComponentStyle, NbtSummaryEntry, NbtSummaryValue, RichTextComponent,
};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::item::plain_stack;
use crate::entity::living::SynchedValue;
use crate::entity::minecart::MinecartData;
use crate::entity::{EntityData, EntityStore};

const ACTIVATION_DELAY: i32 = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct PendingCommandMinecartExecution {
    pub entity_id: i32,
    pub entity_uuid: uuid::Uuid,
    pub position: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub command_blocks_work: bool,
    pub snapshot: CommandBlockExecutionSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivatorRailDecision {
    NoExecution,
    Execute(CommandBlockExecutionSnapshot),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandMinecartRun {
    pub command: bbb_protocol::JavaString,
    pub source_name: bbb_protocol::JavaString,
    pub display_name: RichTextComponent,
    pub track_output: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandMinecartExecutionPlan {
    Skip,
    Complete,
    Execute(CommandMinecartRun),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandBlockMinecartData {
    minecart: MinecartData,
    command: bbb_protocol::JavaString,
    synched_command: bbb_protocol::JavaString,
    success_count: i32,
    custom_name: Option<RichTextComponent>,
    track_output: bool,
    last_output: Option<RichTextComponent>,
    synched_last_output: RichTextComponent,
    update_last_execution: bool,
    last_execution: i64,
    last_activated: i32,
}

impl CommandBlockMinecartData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.minecart.restore_command_transients(&previous.minecart);
        self.last_activated = previous.last_activated;
    }

    /// `MinecartCommandBlock.java:32-34` and `BaseCommandBlock.java:24-32`.
    pub fn new() -> Self {
        Self {
            minecart: MinecartData::new(),
            command: Default::default(),
            synched_command: Default::default(),
            success_count: 0,
            custom_name: None,
            track_output: true,
            last_output: None,
            synched_last_output: RichTextComponent::plain(""),
            update_last_execution: true,
            last_execution: NO_LAST_EXECUTION,
            last_activated: 0,
        }
    }

    pub const fn minecart(&self) -> &MinecartData {
        &self.minecart
    }

    pub fn minecart_mut(&mut self) -> &mut MinecartData {
        &mut self.minecart
    }

    pub fn set_minecart(&mut self, minecart: MinecartData) {
        self.minecart = minecart;
    }

    pub fn command(&self) -> &bbb_protocol::JavaString {
        &self.command
    }

    pub fn synched_command(&self) -> &bbb_protocol::JavaString {
        &self.synched_command
    }

    /// `BaseCommandBlock.java:80-83`.
    pub fn set_command(&mut self, command: impl Into<bbb_protocol::JavaString>) {
        self.command = command.into();
        self.success_count = 0;
    }

    pub const fn success_count(&self) -> i32 {
        self.success_count
    }

    pub fn set_success_count(&mut self, success_count: i32) {
        self.success_count = success_count;
    }

    pub const fn custom_name(&self) -> Option<&RichTextComponent> {
        self.custom_name.as_ref()
    }

    pub fn set_custom_name(&mut self, custom_name: Option<RichTextComponent>) {
        self.custom_name = custom_name;
    }

    pub const fn track_output(&self) -> bool {
        self.track_output
    }

    pub fn set_track_output(&mut self, track_output: bool) {
        self.track_output = track_output;
    }

    pub const fn last_output(&self) -> Option<&RichTextComponent> {
        self.last_output.as_ref()
    }

    pub const fn synched_last_output(&self) -> &RichTextComponent {
        &self.synched_last_output
    }

    pub fn set_last_output(&mut self, last_output: Option<RichTextComponent>) {
        self.last_output = last_output;
    }

    pub fn on_updated(&mut self) -> (bool, bool) {
        let command_changed = self.synched_command != self.command;
        let last_output = self
            .last_output
            .clone()
            .unwrap_or_else(|| RichTextComponent::plain(""));
        let last_output_changed = self.synched_last_output != last_output;
        self.synched_command = self.command.clone();
        self.synched_last_output = last_output;
        (command_changed, last_output_changed)
    }

    pub const fn update_last_execution(&self) -> bool {
        self.update_last_execution
    }

    pub fn set_update_last_execution(&mut self, update_last_execution: bool) {
        self.update_last_execution = update_last_execution;
        if !update_last_execution {
            self.last_execution = NO_LAST_EXECUTION;
        }
    }

    pub const fn last_execution(&self) -> i64 {
        self.last_execution
    }

    pub fn set_last_execution(&mut self, last_execution: i64) {
        self.last_execution = if self.update_last_execution {
            last_execution
        } else {
            NO_LAST_EXECUTION
        };
    }

    pub const fn last_activated(&self) -> i32 {
        self.last_activated
    }

    pub fn execution_snapshot(&self) -> CommandBlockExecutionSnapshot {
        CommandBlockExecutionSnapshot {
            command: self.command.clone(),
            success_count: self.success_count,
            custom_name: self.custom_name.clone(),
            track_output: self.track_output,
            last_output: self.last_output.clone(),
            update_last_execution: self.update_last_execution,
            last_execution: self.last_execution,
        }
    }

    fn apply_execution_snapshot(&mut self, state: CommandBlockExecutionSnapshot) {
        self.command = state.command;
        self.success_count = state.success_count;
        self.custom_name = state.custom_name;
        self.track_output = state.track_output;
        self.last_output = state.last_output;
        self.update_last_execution = state.update_last_execution;
        self.last_execution = state.last_execution;
    }

    /// `MinecartCommandBlock.java:81-86`.
    pub fn activate_minecart(&mut self, tick_count: i32, powered: bool) -> ActivatorRailDecision {
        if !powered || tick_count.wrapping_sub(self.last_activated) < ACTIVATION_DELAY {
            return ActivatorRailDecision::NoExecution;
        }
        self.last_activated = tick_count;
        ActivatorRailDecision::Execute(self.execution_snapshot())
    }

    /// `MinecartCommandBlock.java:27-31` and `:50-55`.
    pub(crate) fn append_synched_values(&self, rows: &mut Vec<SynchedValue>) {
        rows.extend([
            SynchedValue {
                value: EntityDataValue {
                    data_id: VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_STRING_ID,
                    value: EntityDataValueKind::String(self.synched_command.clone()),
                },
                is_default: self.synched_command.is_empty(),
            },
            SynchedValue {
                is_default: self.synched_last_output == RichTextComponent::plain(""),
                value: EntityDataValue {
                    data_id: VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID,
                    value: EntityDataValueKind::Component(self.synched_last_output.clone()),
                },
            },
        ]);
    }

    /// `BaseCommandBlock.java:46-59`.
    pub(crate) fn write_save_data(&self, entries: &mut Vec<NbtSummaryEntry>) {
        entries.retain(|entry| {
            !entry
                .name
                .with_utf8(|name| {
                    matches!(
                        name,
                        "Command"
                            | "SuccessCount"
                            | "TrackOutput"
                            | "LastOutput"
                            | "UpdateLastExecution"
                            | "LastExecution"
                    )
                })
                .unwrap_or(false)
                && (self.custom_name.is_none() || entry.name != "CustomName")
        });
        entries.push(entry(
            "Command",
            NbtSummaryValue::string(self.command.clone()),
        ));
        entries.push(entry(
            "SuccessCount",
            NbtSummaryValue::Int(self.success_count),
        ));
        if let Some(custom_name) = &self.custom_name {
            entries.push(entry("CustomName", custom_name.component().clone()));
        }
        entries.push(entry("TrackOutput", byte_bool(self.track_output)));
        if self.track_output {
            if let Some(last_output) = &self.last_output {
                entries.push(entry("LastOutput", last_output.component().clone()));
            }
        }
        entries.push(entry(
            "UpdateLastExecution",
            byte_bool(self.update_last_execution),
        ));
        if self.update_last_execution && self.last_execution != NO_LAST_EXECUTION {
            entries.push(entry(
                "LastExecution",
                NbtSummaryValue::Long(self.last_execution),
            ));
        }
    }

    /// `BaseCommandBlock.java:61-78`.
    pub(crate) fn read_save_data(minecart: MinecartData, entries: &[NbtSummaryEntry]) -> Self {
        let command = string_field(entries, "Command").unwrap_or_default();
        let success_count = numeric_field(entries, "SuccessCount")
            .and_then(numeric_i32)
            .unwrap_or(0);
        let custom_name = component_field(entries, "CustomName");
        let track_output = bool_field(entries, "TrackOutput").unwrap_or(true);
        let last_output = track_output
            .then(|| component_field(entries, "LastOutput"))
            .flatten();
        let update_last_execution = bool_field(entries, "UpdateLastExecution").unwrap_or(true);
        let last_execution = if update_last_execution {
            numeric_field(entries, "LastExecution")
                .and_then(numeric_i64)
                .unwrap_or(NO_LAST_EXECUTION)
        } else {
            NO_LAST_EXECUTION
        };
        Self {
            minecart,
            synched_command: command.clone(),
            command,
            success_count,
            custom_name,
            track_output,
            synched_last_output: last_output
                .clone()
                .unwrap_or_else(|| RichTextComponent::plain("")),
            last_output,
            update_last_execution,
            last_execution,
            last_activated: 0,
        }
    }

    /// `MinecartCommandBlock.java:41-48`.
    pub fn drop_item(&self) -> Option<RuntimeItemStack> {
        plain_stack("minecraft:minecart", 1)
    }

    pub fn pick_result(&self) -> Option<RuntimeItemStack> {
        plain_stack("minecraft:command_block_minecart", 1)
    }
}

impl Default for CommandBlockMinecartData {
    fn default() -> Self {
        Self::new()
    }
}

pub fn edit_command_minecart(
    store: &mut EntityStore,
    entity_id: i32,
    command: bbb_protocol::JavaString,
    track_output: bool,
    command_blocks_work: bool,
) -> bool {
    let Some(entity) = store.get_mut(entity_id) else {
        return false;
    };
    let (command_dirty, output_dirty) = {
        let EntityData::CommandBlockMinecart(data) = &mut entity.data else {
            return false;
        };
        data.set_command(command);
        data.set_track_output(track_output);
        if !track_output {
            data.set_last_output(None);
        }
        if command_blocks_work {
            data.on_updated()
        } else {
            (false, false)
        }
    };
    if command_dirty {
        entity.mark_synched_dirty(VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID);
    }
    if output_dirty {
        entity.mark_synched_dirty(VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID);
    }
    true
}

pub fn begin_command_minecart_execution(
    store: &mut EntityStore,
    request: &PendingCommandMinecartExecution,
    game_time: i64,
) -> CommandMinecartExecutionPlan {
    let Some(entity) = store.get_mut(request.entity_id) else {
        return CommandMinecartExecutionPlan::Skip;
    };
    if entity.uuid() != request.entity_uuid {
        return CommandMinecartExecutionPlan::Skip;
    }
    let display_name = command_minecart_display_name(entity);
    let EntityData::CommandBlockMinecart(data) = &mut entity.data else {
        return CommandMinecartExecutionPlan::Skip;
    };
    let mut state = data.execution_snapshot();
    let plan = begin_command_block_execution(&mut state, game_time, request.command_blocks_work);
    data.apply_execution_snapshot(state);
    match plan {
        CommandBlockExecutionPlan::Skip => return CommandMinecartExecutionPlan::Skip,
        CommandBlockExecutionPlan::Complete => return CommandMinecartExecutionPlan::Complete,
        CommandBlockExecutionPlan::Execute => {}
    }
    CommandMinecartExecutionPlan::Execute(CommandMinecartRun {
        command: data.command().clone(),
        source_name: data.custom_name().map_or_else(
            || "@".into(),
            |name| bbb_server_datapack::language::component_plain_text(name.component()).into(),
        ),
        display_name,
        track_output: data.track_output(),
    })
}

pub fn increment_command_minecart_success(
    store: &mut EntityStore,
    entity_id: i32,
    entity_uuid: uuid::Uuid,
) {
    let Some(entity) = store
        .get_mut(entity_id)
        .filter(|entity| entity.uuid() == entity_uuid)
    else {
        return;
    };
    if let EntityData::CommandBlockMinecart(data) = &mut entity.data {
        data.set_success_count(data.success_count().wrapping_add(1));
    }
}

pub fn finish_command_minecart_execution(
    store: &mut EntityStore,
    entity_id: i32,
    entity_uuid: uuid::Uuid,
    game_time: i64,
    successful_contexts: i32,
    last_output: Option<RichTextComponent>,
) -> bool {
    let Some(entity) = store.get_mut(entity_id) else {
        return false;
    };
    if entity.uuid() != entity_uuid {
        return false;
    }
    let (command_dirty, output_dirty) = {
        let EntityData::CommandBlockMinecart(data) = &mut entity.data else {
            return false;
        };
        let output_received = last_output.is_some();
        let mut state = data.execution_snapshot();
        finish_command_block_execution(&mut state, game_time, successful_contexts, last_output);
        data.apply_execution_snapshot(state);
        if output_received {
            data.on_updated()
        } else {
            (false, false)
        }
    };
    if command_dirty {
        entity.mark_synched_dirty(VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID);
    }
    if output_dirty {
        entity.mark_synched_dirty(VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID);
    }
    true
}

fn command_minecart_display_name(entity: &crate::entity::Entity) -> RichTextComponent {
    let name = entity.custom_name().map_or_else(
        || {
            component_translatable_nbt(
                "entity.minecraft.command_block_minecart",
                Vec::new(),
                &ComponentStyle::default(),
            )
            .expect("static entity display component")
        },
        |name| name.component().clone(),
    );
    let style = ComponentStyle {
        hover_event: Some(ComponentHoverEvent::ShowEntity {
            id: "minecraft:command_block_minecart".to_string(),
            uuid: component_hover_uuid_nbt(entity.uuid()),
            name: None,
            name_component: Some(name.clone()),
        }),
        insertion: Some(entity.uuid().to_string().into()),
        ..ComponentStyle::default()
    };
    let component = component_appended_nbt(&style, vec![name])
        .expect("static command minecart display component");
    RichTextComponent::try_from_component_nbt(component)
        .expect("static command minecart display component")
}

fn string_field(entries: &[NbtSummaryEntry], name: &str) -> Option<bbb_protocol::JavaString> {
    match field(entries, name) {
        Some(NbtSummaryValue::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn numeric_field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    let value = field(entries, name)?;
    matches!(
        value,
        NbtSummaryValue::Byte(_)
            | NbtSummaryValue::Short(_)
            | NbtSummaryValue::Int(_)
            | NbtSummaryValue::Long(_)
            | NbtSummaryValue::Float(_)
            | NbtSummaryValue::Double(_)
    )
    .then_some(value)
}

fn bool_field(entries: &[NbtSummaryEntry], name: &str) -> Option<bool> {
    numeric_field(entries, name)
        .and_then(numeric_i8)
        .map(|value| value != 0)
}

fn component_field(entries: &[NbtSummaryEntry], name: &str) -> Option<RichTextComponent> {
    RichTextComponent::try_from_component_nbt(field(entries, name)?.clone()).ok()
}

fn numeric_i8(value: &NbtSummaryValue) -> Option<i8> {
    Some(match value {
        NbtSummaryValue::Byte(value) => *value,
        NbtSummaryValue::Short(value) => *value as i8,
        NbtSummaryValue::Int(value) => *value as i8,
        NbtSummaryValue::Long(value) => *value as i8,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits).floor() as i32 as i8,
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits).floor() as i32 as i8,
        _ => return None,
    })
}

fn numeric_i32(value: &NbtSummaryValue) -> Option<i32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => i32::from(*value),
        NbtSummaryValue::Short(value) => i32::from(*value),
        NbtSummaryValue::Int(value) => *value,
        NbtSummaryValue::Long(value) => *value as i32,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits).floor() as i32,
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits).floor() as i32,
        _ => return None,
    })
}

fn numeric_i64(value: &NbtSummaryValue) -> Option<i64> {
    Some(match value {
        NbtSummaryValue::Byte(value) => i64::from(*value),
        NbtSummaryValue::Short(value) => i64::from(*value),
        NbtSummaryValue::Int(value) => i64::from(*value),
        NbtSummaryValue::Long(value) => *value,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits) as i64,
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits).floor() as i64,
        _ => return None,
    })
}

fn byte_bool(value: bool) -> NbtSummaryValue {
    NbtSummaryValue::Byte(i8::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityIdCounter;

    fn saved(data: &CommandBlockMinecartData) -> Vec<NbtSummaryEntry> {
        let mut entries = Vec::new();
        data.write_save_data(&mut entries);
        entries
    }

    fn command_store() -> (EntityStore, i32) {
        let mut store = EntityStore::new(0x43_4d_44, EntityIdCounter::new());
        let id = crate::entity::spawn_command_block_minecart(
            &mut store,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            0.0,
        )
        .unwrap();
        (store, id)
    }

    fn request(
        store: &EntityStore,
        id: i32,
        command_blocks_work: bool,
    ) -> PendingCommandMinecartExecution {
        let entity = store.get(id).unwrap();
        let EntityData::CommandBlockMinecart(data) = entity.data() else {
            panic!("expected command minecart")
        };
        PendingCommandMinecartExecution {
            entity_id: id,
            entity_uuid: entity.uuid(),
            position: entity.position(),
            y_rot: entity.y_rot(),
            x_rot: entity.x_rot(),
            command_blocks_work,
            snapshot: data.execution_snapshot(),
        }
    }

    #[test]
    fn defaults_match_vanilla_fields() {
        let data = CommandBlockMinecartData::new();
        assert_eq!(data.minecart(), &MinecartData::new());
        assert_eq!(data.command(), "");
        assert_eq!(data.success_count(), 0);
        assert_eq!(data.custom_name(), None);
        assert!(data.track_output());
        assert_eq!(data.last_output(), None);
        assert!(data.update_last_execution());
        assert_eq!(data.last_execution(), NO_LAST_EXECUTION);
        assert_eq!(data.last_activated(), 0);
    }

    #[test]
    fn powered_activator_rail_has_four_tick_gate() {
        let mut data = CommandBlockMinecartData::new();
        data.set_command("say ready");
        for tick_count in 0..4 {
            assert_eq!(
                data.activate_minecart(tick_count, true),
                ActivatorRailDecision::NoExecution
            );
        }
        let ActivatorRailDecision::Execute(snapshot) = data.activate_minecart(4, true) else {
            panic!("tick four must request execution");
        };
        assert_eq!(snapshot.command, "say ready");
        assert_eq!(data.last_activated(), 4);
        assert_eq!(
            data.activate_minecart(7, true),
            ActivatorRailDecision::NoExecution
        );
        assert!(matches!(
            data.activate_minecart(8, true),
            ActivatorRailDecision::Execute(_)
        ));
    }

    #[test]
    fn unpowered_activator_never_updates_gate() {
        let mut data = CommandBlockMinecartData::new();
        assert_eq!(
            data.activate_minecart(20, false),
            ActivatorRailDecision::NoExecution
        );
        assert_eq!(data.last_activated(), 0);
        assert!(matches!(
            data.activate_minecart(20, true),
            ActivatorRailDecision::Execute(_)
        ));
    }

    #[test]
    fn metadata_appends_command_and_non_optional_last_output() {
        let mut data = CommandBlockMinecartData::new();
        data.set_command("list");
        data.set_last_output(Some(RichTextComponent::plain("done")));
        data.on_updated();
        let mut rows = Vec::new();
        data.append_synched_values(&mut rows);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].value.data_id, 13);
        assert_eq!(rows[0].value.serializer_id, 4);
        assert_eq!(
            rows[0].value.value,
            EntityDataValueKind::String("list".into())
        );
        assert_eq!(rows[1].value.data_id, 14);
        assert_eq!(rows[1].value.serializer_id, 5);
        assert_eq!(
            rows[1].value.value,
            EntityDataValueKind::Component(RichTextComponent::plain("done"))
        );
    }

    #[test]
    fn full_nbt_round_trip_preserves_persistent_runtime() {
        let mut data = CommandBlockMinecartData::new();
        data.set_command("say roundtrip");
        data.set_success_count(3);
        data.set_custom_name(Some(RichTextComponent::plain("runner")));
        data.set_track_output(true);
        data.set_last_output(Some(RichTextComponent::plain("ok")));
        data.set_update_last_execution(true);
        data.set_last_execution(9123);
        data.on_updated();
        let read = CommandBlockMinecartData::read_save_data(MinecartData::new(), &saved(&data));
        assert_eq!(read, data);
    }

    #[test]
    fn last_activated_is_not_persistent() {
        let mut data = CommandBlockMinecartData::new();
        assert!(matches!(
            data.activate_minecart(12, true),
            ActivatorRailDecision::Execute(_)
        ));
        assert_eq!(data.last_activated(), 12);
        let entries = saved(&data);
        assert!(field(&entries, "LastActivated").is_none());
        let read = CommandBlockMinecartData::read_save_data(MinecartData::new(), &entries);
        assert_eq!(read.last_activated(), 0);
    }

    #[test]
    fn disabled_output_is_omitted_and_cleared_only_by_load() {
        let mut data = CommandBlockMinecartData::new();
        data.set_last_output(Some(RichTextComponent::plain("old")));
        data.set_track_output(false);
        assert_eq!(data.last_output().unwrap().text(), "old");
        let entries = saved(&data);
        assert!(field(&entries, "LastOutput").is_none());
        let read = CommandBlockMinecartData::read_save_data(MinecartData::new(), &entries);
        assert!(!read.track_output());
        assert_eq!(read.last_output(), None);
    }

    #[test]
    fn editor_gate_keeps_internal_and_synched_state_distinct() {
        let (mut store, id) = command_store();
        assert!(edit_command_minecart(
            &mut store,
            id,
            "say hidden".into(),
            false,
            false,
        ));
        let entity = store.get_mut(id).unwrap();
        let mut rows = Vec::new();
        {
            let EntityData::CommandBlockMinecart(data) = entity.data() else {
                panic!("expected command minecart")
            };
            assert_eq!(data.command(), "say hidden");
            assert!(!data.track_output());
            data.append_synched_values(&mut rows);
        }
        assert!(entity.take_synched_dirty().is_empty());
        assert!(rows.iter().all(|row| row.is_default));
    }

    #[test]
    fn execution_source_name_retains_custom_name_units() {
        for unit in [0xd800, 0xd801, 0xfffd] {
            let (mut store, id) = command_store();
            let name = bbb_protocol::JavaString::from(vec![unit]);
            let EntityData::CommandBlockMinecart(data) = &mut store.get_mut(id).unwrap().data
            else {
                panic!("expected command minecart");
            };
            data.set_custom_name(Some(
                RichTextComponent::try_from_component_nbt(NbtSummaryValue::String(name.clone()))
                    .unwrap(),
            ));
            edit_command_minecart(&mut store, id, "seed".into(), true, true);
            let pending = request(&store, id, true);
            let CommandMinecartExecutionPlan::Execute(run) =
                begin_command_minecart_execution(&mut store, &pending, 42)
            else {
                panic!("enabled non-empty command must execute");
            };
            assert_eq!(run.source_name, name);
        }
    }

    #[test]
    fn execution_finish_updates_runtime_and_only_published_output_metadata() {
        let (mut store, id) = command_store();
        edit_command_minecart(&mut store, id, "seed".into(), true, true);
        store.get_mut(id).unwrap().take_synched_dirty();
        let pending = request(&store, id, true);
        let CommandMinecartExecutionPlan::Execute(run) =
            begin_command_minecart_execution(&mut store, &pending, 42)
        else {
            panic!("enabled non-empty command must execute")
        };
        assert_eq!(run.command, "seed");
        assert_eq!(run.source_name, "@");
        finish_command_minecart_execution(
            &mut store,
            id,
            pending.entity_uuid,
            42,
            1,
            Some(RichTextComponent::plain("done")),
        );
        let entity = store.get_mut(id).unwrap();
        let EntityData::CommandBlockMinecart(data) = entity.data() else {
            panic!("expected command minecart")
        };
        assert_eq!(data.success_count(), 1);
        assert_eq!(data.last_execution(), 42);
        assert_eq!(data.last_output().unwrap().text(), "done");
        assert_eq!(
            entity.take_synched_dirty(),
            [VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID]
        );
        assert_eq!(
            begin_command_minecart_execution(&mut store, &pending, 42),
            CommandMinecartExecutionPlan::Skip
        );
    }

    #[test]
    fn searge_shortcut_does_not_touch_last_execution_or_synched_projection() {
        let (mut store, id) = command_store();
        edit_command_minecart(&mut store, id, "Searge".into(), true, true);
        store.get_mut(id).unwrap().take_synched_dirty();
        let pending = request(&store, id, true);
        assert_eq!(
            begin_command_minecart_execution(&mut store, &pending, 77),
            CommandMinecartExecutionPlan::Complete
        );
        let entity = store.get_mut(id).unwrap();
        let EntityData::CommandBlockMinecart(data) = entity.data() else {
            panic!("expected command minecart")
        };
        assert_eq!(data.success_count(), 1);
        assert_eq!(data.last_output().unwrap().text(), "#itzlipofutzli");
        assert_eq!(data.last_execution(), NO_LAST_EXECUTION);
        assert!(entity.take_synched_dirty().is_empty());
    }

    #[test]
    fn drop_and_pick_use_distinct_vanilla_items() {
        let data = CommandBlockMinecartData::new();
        assert_eq!(data.drop_item(), plain_stack("minecraft:minecart", 1));
        assert_eq!(
            data.pick_result(),
            plain_stack("minecraft:command_block_minecart", 1)
        );
    }
}
