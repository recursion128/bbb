//! Shared `BaseCommandBlock` execution state transitions.

use bbb_protocol::RichTextComponent;

pub const NO_LAST_EXECUTION: i64 = -1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandBlockExecutionSnapshot {
    pub command: bbb_protocol::JavaString,
    pub success_count: i32,
    pub custom_name: Option<RichTextComponent>,
    pub track_output: bool,
    pub last_output: Option<RichTextComponent>,
    pub update_last_execution: bool,
    pub last_execution: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBlockExecutionPlan {
    Skip,
    Complete,
    Execute,
}

/// `BaseCommandBlock.performCommand` (`BaseCommandBlock.java:89-129`).
pub fn begin_command_block_execution(
    state: &mut CommandBlockExecutionSnapshot,
    game_time: i64,
    enabled: bool,
) -> CommandBlockExecutionPlan {
    if state.last_execution == game_time {
        return CommandBlockExecutionPlan::Skip;
    }
    if state.command.units().len() == 6
        && state
            .command
            .units()
            .iter()
            .copied()
            .zip("searge".encode_utf16())
            .all(|(unit, lower)| unit == lower || unit == lower - 32)
    {
        state.last_output = Some(RichTextComponent::plain("#itzlipofutzli"));
        state.success_count = 1;
        return CommandBlockExecutionPlan::Complete;
    }

    state.success_count = 0;
    if !enabled || state.command.is_empty() {
        record_execution(state, game_time);
        return CommandBlockExecutionPlan::Complete;
    }
    state.last_output = None;
    CommandBlockExecutionPlan::Execute
}

/// `BaseCommandBlock.performCommand` (`BaseCommandBlock.java:107-129`).
pub fn finish_command_block_execution(
    state: &mut CommandBlockExecutionSnapshot,
    game_time: i64,
    successful_contexts: i32,
    last_output: Option<RichTextComponent>,
) {
    state.success_count = successful_contexts;
    if let Some(last_output) = last_output {
        state.last_output = Some(last_output);
    }
    record_execution(state, game_time);
}

fn record_execution(state: &mut CommandBlockExecutionSnapshot, game_time: i64) {
    state.last_execution = if state.update_last_execution {
        game_time
    } else {
        NO_LAST_EXECUTION
    };
}
