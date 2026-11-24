//! `CommandBlockEntity` (`CommandBlockEntity.java:22-186`).
//!
//! Nested `BaseCommandBlock` NBT round-trips. Command execution stays on the
//! server command surface.

use crate::command_block_execution::{CommandBlockExecutionSnapshot, NO_LAST_EXECUTION};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};

use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::value_input::{boolean_or, int_or, long_or};
use super::{entry, unique_field};

const TAG_COMMAND: &str = "Command";
const TAG_SUCCESS_COUNT: &str = "SuccessCount";
const TAG_CUSTOM_NAME: &str = "CustomName";
const TAG_TRACK_OUTPUT: &str = "TrackOutput";
const TAG_LAST_OUTPUT: &str = "LastOutput";
const TAG_UPDATE_LAST_EXECUTION: &str = "UpdateLastExecution";
const TAG_LAST_EXECUTION: &str = "LastExecution";
const TAG_POWERED: &str = "powered";
const TAG_CONDITION_MET: &str = "conditionMet";
const TAG_AUTO: &str = "auto";

/// `CommandBlockEntity` (`CommandBlockEntity.java:22-28`) plus nested
/// `BaseCommandBlock` (`BaseCommandBlock.java:23-32`).
#[derive(Debug, Clone, PartialEq)]
pub struct CommandBlockEntity {
    command: bbb_protocol::JavaString,
    success_count: i32,
    custom_name: Option<NbtSummaryValue>,
    track_output: bool,
    last_output: Option<NbtSummaryValue>,
    update_last_execution: bool,
    last_execution: i64,
    powered: bool,
    condition_met: bool,
    auto: bool,
}

impl Default for CommandBlockEntity {
    /// `new CommandBlockEntity` (`CommandBlockEntity.java:64-66`) and the
    /// nested `BaseCommandBlock` field defaults (`BaseCommandBlock.java:26-32`).
    fn default() -> Self {
        Self {
            command: Default::default(),
            success_count: 0,
            custom_name: None,
            track_output: true,
            last_output: None,
            update_last_execution: true,
            last_execution: NO_LAST_EXECUTION,
            powered: false,
            condition_met: false,
            auto: false,
        }
    }
}

impl CommandBlockEntity {
    // CommandBlockEntity.java:162-165; BaseCommandBlock.java:144-146 assigns the name directly.
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        self.custom_name = components.read("minecraft:custom_name")?;
        Ok(())
    }

    pub fn execution_snapshot(&self) -> CommandBlockExecutionSnapshot {
        CommandBlockExecutionSnapshot {
            command: self.command.clone(),
            success_count: self.success_count,
            custom_name: self.custom_name(),
            track_output: self.track_output,
            last_output: self.last_output(),
            update_last_execution: self.update_last_execution,
            last_execution: self.last_execution,
        }
    }

    pub fn apply_execution_snapshot(&mut self, state: CommandBlockExecutionSnapshot) {
        self.command = state.command;
        self.success_count = state.success_count;
        self.set_custom_name(state.custom_name);
        self.track_output = state.track_output;
        self.set_last_output(state.last_output);
        self.update_last_execution = state.update_last_execution;
        self.last_execution = state.last_execution;
    }

    pub fn apply_execution_result(&mut self, state: CommandBlockExecutionSnapshot) {
        self.success_count = state.success_count;
        self.set_last_output(state.last_output);
        self.last_execution = state.last_execution;
    }

    pub fn command(&self) -> &bbb_protocol::JavaString {
        &self.command
    }

    /// `BaseCommandBlock.setCommand` (`BaseCommandBlock.java:80-83`).
    pub fn set_command(&mut self, command: impl Into<bbb_protocol::JavaString>) {
        self.command = command.into();
        self.success_count = 0;
    }

    pub fn custom_name(&self) -> Option<RichTextComponent> {
        self.custom_name
            .clone()
            .and_then(|value| RichTextComponent::try_from_component_nbt(value).ok())
    }

    pub fn set_custom_name(&mut self, name: Option<RichTextComponent>) {
        self.custom_name = name.map(|name| name.component().clone());
    }

    /// `BaseCommandBlock.getName` (`BaseCommandBlock.java:136-138`).
    pub fn name(&self) -> RichTextComponent {
        self.custom_name()
            .unwrap_or_else(|| RichTextComponent::plain("@"))
    }

    pub const fn track_output(&self) -> bool {
        self.track_output
    }

    pub fn set_track_output(&mut self, track_output: bool) {
        self.track_output = track_output;
    }

    pub fn last_output(&self) -> Option<RichTextComponent> {
        self.last_output
            .clone()
            .and_then(|value| RichTextComponent::try_from_component_nbt(value).ok())
    }

    pub fn set_last_output(&mut self, output: Option<RichTextComponent>) {
        self.last_output = output.map(|output| output.component().clone());
    }

    pub const fn powered(&self) -> bool {
        self.powered
    }

    pub fn set_powered(&mut self, powered: bool) {
        self.powered = powered;
    }

    pub const fn automatic(&self) -> bool {
        self.auto
    }

    pub fn set_automatic(&mut self, automatic: bool) {
        self.auto = automatic;
    }

    pub const fn condition_met(&self) -> bool {
        self.condition_met
    }

    pub fn set_condition_met(&mut self, condition_met: bool) {
        self.condition_met = condition_met;
    }

    /// `CommandBlockEntity.collectImplicitComponents` (`:168-171`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        components.set("minecraft:custom_name", self.custom_name.clone());
    }

    /// `BaseCommandBlock.getSuccessCount` (`BaseCommandBlock.java:34-36`),
    /// which is `CommandBlock.getAnalogOutputSignal`'s whole body
    /// (`CommandBlock.java:146-149`).
    #[must_use]
    pub const fn success_count(&self) -> i32 {
        self.success_count
    }

    /// `BaseCommandBlock.setSuccessCount` (`BaseCommandBlock.java:38-40`).
    pub fn set_success_count(&mut self, success_count: i32) {
        self.success_count = success_count;
    }

    /// `CommandBlockEntity.saveAdditional` (`CommandBlockEntity.java:69-75`)
    /// then `BaseCommandBlock.save` (`BaseCommandBlock.java:46-59`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        let mut entries = vec![
            entry(TAG_COMMAND, NbtSummaryValue::string(self.command.clone())),
            entry(TAG_SUCCESS_COUNT, NbtSummaryValue::Int(self.success_count)),
        ];
        if let Some(name) = &self.custom_name {
            entries.push(entry(TAG_CUSTOM_NAME, name.clone()));
        }
        entries.push(bool_entry(TAG_TRACK_OUTPUT, self.track_output));
        if self.track_output {
            if let Some(output) = &self.last_output {
                entries.push(entry(TAG_LAST_OUTPUT, output.clone()));
            }
        }
        entries.push(bool_entry(
            TAG_UPDATE_LAST_EXECUTION,
            self.update_last_execution,
        ));
        if self.update_last_execution && self.last_execution != NO_LAST_EXECUTION {
            entries.push(entry(
                TAG_LAST_EXECUTION,
                NbtSummaryValue::Long(self.last_execution),
            ));
        }
        entries.push(bool_entry(TAG_POWERED, self.powered));
        entries.push(bool_entry(TAG_CONDITION_MET, self.condition_met));
        entries.push(bool_entry(TAG_AUTO, self.auto));
        entries
    }

    /// `CommandBlockEntity.loadAdditional` (`CommandBlockEntity.java:78-84`)
    /// then `BaseCommandBlock.load` (`BaseCommandBlock.java:61-78`).
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.command = nbt_string(record, TAG_COMMAND).cloned().unwrap_or_default();
        self.success_count = int_or(record, TAG_SUCCESS_COUNT, 0);
        self.custom_name = unique_field(record, TAG_CUSTOM_NAME).cloned();
        self.track_output = boolean_or(record, TAG_TRACK_OUTPUT, true);
        if self.track_output {
            self.last_output = unique_field(record, TAG_LAST_OUTPUT).cloned();
        } else {
            self.last_output = None;
        }
        self.update_last_execution = boolean_or(record, TAG_UPDATE_LAST_EXECUTION, true);
        if self.update_last_execution {
            self.last_execution = long_or(record, TAG_LAST_EXECUTION, NO_LAST_EXECUTION);
        } else {
            self.last_execution = NO_LAST_EXECUTION;
        }
        self.powered = boolean_or(record, TAG_POWERED, false);
        self.condition_met = boolean_or(record, TAG_CONDITION_MET, false);
        self.auto = boolean_or(record, TAG_AUTO, false);
    }
}

fn bool_entry(name: &str, value: bool) -> NbtSummaryEntry {
    entry(name, NbtSummaryValue::Byte(i8::from(value)))
}

fn nbt_string<'a>(
    record: &'a [NbtSummaryEntry],
    name: &str,
) -> Option<&'a bbb_protocol::JavaString> {
    match unique_field(record, name) {
        Some(NbtSummaryValue::String(value)) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_COMMAND_BLOCK_ID;
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::NbtSummaryValue;

    use super::{
        entry, TAG_AUTO, TAG_COMMAND, TAG_CONDITION_MET, TAG_CUSTOM_NAME, TAG_LAST_EXECUTION,
        TAG_LAST_OUTPUT, TAG_POWERED, TAG_SUCCESS_COUNT, TAG_TRACK_OUTPUT,
        TAG_UPDATE_LAST_EXECUTION,
    };
    use crate::block_entity::{new_block_entity, unique_field};

    fn pos() -> BlockPos {
        BlockPos { x: 1, y: 2, z: 3 }
    }

    fn command_block() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_COMMAND_BLOCK_ID)
            .expect("minecraft:command_block has a default state")
    }

    fn overlay(
        record: &mut Vec<bbb_protocol::NbtSummaryEntry>,
        fields: Vec<bbb_protocol::NbtSummaryEntry>,
    ) {
        for field in fields {
            if let Some(existing) = record.iter_mut().find(|entry| entry.name == field.name) {
                existing.value = field.value;
            } else {
                record.push(field);
            }
        }
    }

    #[test]
    fn command_block_save_load_round_trips_owned_fields() {
        let state = command_block();
        let mut record = new_block_entity(pos(), state)
            .expect("a command block builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let command = NbtSummaryValue::string("say hi");
        let custom_name = NbtSummaryValue::string("Box");
        let last_output = NbtSummaryValue::string("[12:00:00] done");
        overlay(
            &mut record,
            vec![
                entry(TAG_COMMAND, command.clone()),
                entry(TAG_SUCCESS_COUNT, NbtSummaryValue::Int(4)),
                entry(TAG_CUSTOM_NAME, custom_name.clone()),
                entry(TAG_TRACK_OUTPUT, NbtSummaryValue::Byte(1)),
                entry(TAG_LAST_OUTPUT, last_output.clone()),
                entry(TAG_UPDATE_LAST_EXECUTION, NbtSummaryValue::Byte(1)),
                entry(TAG_LAST_EXECUTION, NbtSummaryValue::Long(99)),
                entry(TAG_POWERED, NbtSummaryValue::Byte(1)),
                entry(TAG_CONDITION_MET, NbtSummaryValue::Byte(1)),
                entry(TAG_AUTO, NbtSummaryValue::Byte(1)),
            ],
        );
        let mut loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("command block loads");
        let snapshot = loaded
            .command_block()
            .expect("command block")
            .execution_snapshot();
        loaded
            .command_block_mut()
            .expect("command block")
            .apply_execution_snapshot(snapshot);
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(unique_field(&saved, TAG_COMMAND), Some(&command));
        assert_eq!(
            unique_field(&saved, TAG_SUCCESS_COUNT),
            Some(&NbtSummaryValue::Int(4))
        );
        assert_eq!(unique_field(&saved, TAG_CUSTOM_NAME), Some(&custom_name));
        assert_eq!(unique_field(&saved, TAG_LAST_OUTPUT), Some(&last_output));
        assert_eq!(
            unique_field(&saved, TAG_LAST_EXECUTION),
            Some(&NbtSummaryValue::Long(99))
        );
        assert_eq!(
            unique_field(&saved, TAG_POWERED),
            Some(&NbtSummaryValue::Byte(1))
        );
        assert_eq!(
            unique_field(&saved, TAG_CONDITION_MET),
            Some(&NbtSummaryValue::Byte(1))
        );
        assert_eq!(
            unique_field(&saved, TAG_AUTO),
            Some(&NbtSummaryValue::Byte(1))
        );
        let again = crate::block_entity::load_test_fixture(pos(), state, &saved)
            .expect("fixture codec succeeds")
            .expect("second load");
        assert_eq!(
            unique_field(
                &again
                    .save_with_full_metadata_with_resolver(
                        crate::block_entity::test_item_resolver(),
                    )
                    .expect("fixture codec succeeds"),
                TAG_COMMAND
            ),
            Some(&command)
        );
    }
}
