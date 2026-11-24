use crate::connection::ConnectionId;
use crate::entity::{
    begin_command_minecart_execution, edit_command_minecart, finish_command_minecart_execution,
    CommandMinecartExecutionPlan,
};
use crate::world::LevelId;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::{SetCommandMinecart, SystemChat};
use bbb_protocol::{
    component_appended_nbt, component_literal_nbt, ComponentStyle, RichTextComponent,
};
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;
use chrono::Local;

use crate::commands::{self, CommandPermission, CommandRun, CommandSource};

use super::{
    command_feedback_channel, CommandFeedbackChannel, ServerWorld, WorldEvent,
    RULE_SEND_COMMAND_FEEDBACK,
};

const RULE_COMMAND_BLOCK_OUTPUT: &str = "minecraft:command_block_output";
const RULE_LOG_ADMIN_COMMANDS: &str = "minecraft:log_admin_commands";

pub(super) struct CommandBlockInvocation<'a> {
    pub(super) position: [f64; 3],
    pub(super) rotation: [f32; 2],
    pub(super) dimension: &'a LevelId,
    pub(super) track_output: bool,
}

impl ServerWorld {
    // BaseCommandBlock.java:107-110: increment the execution instance after each successful command.
    pub(crate) fn increment_command_block_success(
        &mut self,
        target: &commands::CommandBlockResultTarget,
    ) {
        match target {
            commands::CommandBlockResultTarget::Block { level, instance } => {
                let pos = instance.block_pos();
                let mut entities = self
                    .level_mut_by_id(&self.require_level_id(level))
                    .chunks
                    .block_entities_mut();
                if let Some(block) = entities
                    .reborrow()
                    .get_mut(pos)
                    .filter(|current| current.same_position_and_type(instance))
                    .and_then(crate::block_entity::BlockEntity::command_block_mut)
                {
                    block.set_success_count(block.success_count().wrapping_add(1));
                    entities.set_changed(pos);
                }
            }
            commands::CommandBlockResultTarget::Minecart {
                level,
                entity_id,
                entity_uuid,
            } => {
                crate::entity::minecart_command_block::increment_command_minecart_success(
                    &mut self.level_mut_by_id(&self.require_level_id(level)).entities,
                    *entity_id,
                    *entity_uuid,
                );
            }
        }
    }

    pub(super) fn command_block_success_count(
        &self,
        target: &commands::CommandBlockResultTarget,
    ) -> Option<i32> {
        match target {
            commands::CommandBlockResultTarget::Block { level, instance } => self
                .level_by_id(&self.require_level_id(level))
                .chunks
                .block_entities()
                .get(instance.block_pos())
                .filter(|current| current.same_position_and_type(instance))
                .and_then(crate::block_entity::BlockEntity::command_block)
                .map(|block| block.success_count()),
            commands::CommandBlockResultTarget::Minecart {
                level,
                entity_id,
                entity_uuid,
            } => {
                let entity = self
                    .level_by_id(&self.require_level_id(level))
                    .entities
                    .get(*entity_id)
                    .filter(|entity| entity.uuid() == *entity_uuid)?;
                if let crate::entity::EntityData::CommandBlockMinecart(data) = entity.data() {
                    Some(data.success_count())
                } else {
                    None
                }
            }
        }
    }

    pub(super) fn apply_set_command_minecart(
        &mut self,
        connection_id: ConnectionId,
        packet: SetCommandMinecart,
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.can_use_game_master_blocks(connection_id) {
            self.send_system_message(
                connection_id,
                SystemChat::from_component(commands::command_block_not_allowed(), false),
                events,
            );
            return;
        }
        let dimension = self.levels[connection_id].dimension;
        let command_blocks_work = self.game_rules.command_blocks_work;
        if !edit_command_minecart(
            &mut self.level_mut_by_id(&dimension).entities,
            packet.entity_id,
            packet.command.clone(),
            packet.track_output,
            command_blocks_work,
        ) {
            return;
        }
        if !packet.command.is_empty() {
            self.send_system_message(
                connection_id,
                SystemChat::from_component(
                    commands::command_block_set_feedback(&packet.command, command_blocks_work),
                    false,
                ),
                events,
            );
        }
    }

    pub(in crate::world) fn run_command_minecart(
        &mut self,
        dimension: &LevelId,
        request: crate::entity::PendingCommandMinecartExecution,
        events: &mut Vec<WorldEvent>,
    ) {
        let game_time = self.game_time;
        let plan = begin_command_minecart_execution(
            &mut self.level_mut_by_id(dimension).entities,
            &request,
            game_time,
        );
        let CommandMinecartExecutionPlan::Execute(run_request) = plan else {
            return;
        };
        let source = CommandSource::stored_entity(
            run_request.source_name,
            request.entity_uuid,
            request.entity_id,
            self.level_key(*dimension).clone(),
            run_request.display_name,
            CommandPermission::GameMaster,
        );
        let (successful_contexts, last_output) = self.execute_command_block_command(
            &run_request.command,
            source,
            commands::CommandBlockResultTarget::Minecart {
                level: self.level_key(*dimension).clone(),
                entity_id: request.entity_id,
                entity_uuid: request.entity_uuid,
            },
            CommandBlockInvocation {
                position: [request.position.x, request.position.y, request.position.z],
                rotation: [request.y_rot, request.x_rot],
                dimension,
                track_output: run_request.track_output,
            },
            events,
        );
        finish_command_minecart_execution(
            &mut self.level_mut_by_id(dimension).entities,
            request.entity_id,
            request.entity_uuid,
            game_time,
            successful_contexts,
            last_output,
        );
    }

    pub(super) fn execute_command_block_command(
        &mut self,
        command: &bbb_protocol::JavaString,
        source: CommandSource,
        target: commands::CommandBlockResultTarget,
        invocation: CommandBlockInvocation<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> (i32, Option<RichTextComponent>) {
        let CommandBlockInvocation {
            position,
            rotation,
            dimension,
            track_output,
        } = invocation;
        let command = command.units();
        let counter = commands::ResultAccumulator::new();
        let origin = commands::CommandOrigin {
            position,
            rotation,
            level: self.level_key(*dimension).clone(),
            anchor: bbb_protocol::entity_anchors::EntityAnchor::Feet,
            eye_height: None,
        };
        let mut report = super::server_functions::CommandContinuationReport {
            command_block: Some((source.clone(), track_output)),
            ..Default::default()
        };
        self.perform_command(
            std::sync::Arc::from(command.strip_prefix(&[u16::from(b'/')]).unwrap_or(command)),
            &source,
            &origin,
            commands::ResultCallback::CountSuccess(counter.clone(), target.clone()),
            Some(&mut report),
            events,
        );
        (
            self.command_block_success_count(&target)
                .unwrap_or(counter.take().1),
            report.last_output,
        )
    }

    pub(super) fn apply_command_block_run(
        &mut self,
        source: CommandSource,
        run: CommandRun,
        track_output: bool,
        events: &mut Vec<WorldEvent>,
    ) -> Option<RichTextComponent> {
        if !track_output || run.feedback.is_empty() {
            for message in run.broadcast {
                self.broadcast_command_message(message, events);
            }
            return None;
        }
        let send_command_feedback = matches!(
            self.admin.game_rule_value(RULE_SEND_COMMAND_FEEDBACK),
            Some(GameRuleValue::Boolean(true))
        );
        let command_block_output = matches!(
            self.admin.game_rule_value(RULE_COMMAND_BLOCK_OUTPUT),
            Some(GameRuleValue::Boolean(true))
        );
        let log_admin_commands = matches!(
            self.admin.game_rule_value(RULE_LOG_ADMIN_COMMANDS),
            Some(GameRuleValue::Boolean(true))
        );
        let mut operator_sessions: Option<Vec<crate::connection::ConnectionId>> = None;
        let mut last_output = None;
        for feedback in run.feedback {
            let success = command_feedback_channel(&feedback) == CommandFeedbackChannel::Success;
            if !success || send_command_feedback {
                last_output = Some(timestamped_command_output(
                    &feedback.message,
                    self.plugins().content_registry(),
                ));
            }
            if !command_block_output || !feedback.inform_admins {
                continue;
            }
            let copy = commands::admin_broadcast_copy(
                &source,
                &feedback.message,
                self.scoreboard(),
                self.plugins().content_registry(),
            );
            if send_command_feedback {
                let sessions = operator_sessions.get_or_insert_with(|| {
                    self.levels
                        .player_entries()
                        .filter(|(_, state)| {
                            self.admin
                                .lists()
                                .operators
                                .contains_key(&state.profile.uuid)
                        })
                        .map(|(connection_id, _)| *connection_id)
                        .collect()
                });
                for connection_id in sessions.iter().copied() {
                    self.send_system_message(
                        connection_id,
                        SystemChat::from_component(copy.clone(), false),
                        events,
                    );
                }
            }
            if log_admin_commands {
                events.push(WorldEvent::ConsoleMessage(copy));
            }
        }
        for message in run.broadcast {
            self.broadcast_command_message(message, events);
        }
        last_output
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn spawn_command_minecart_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        position: bbb_protocol::packets::Vec3d,
    ) -> i32 {
        crate::entity::spawn_command_block_minecart(
            &mut self.level_mut(dimension).entities,
            position,
            0.0,
        )
        .expect("the command minecart test fixture has a free entity id and UUID")
    }

    pub(crate) fn edit_command_minecart_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
        command: &str,
        track_output: bool,
    ) {
        assert!(edit_command_minecart(
            &mut self.level_mut(dimension).entities,
            entity_id,
            command.to_string().into(),
            track_output,
            true,
        ));
    }

    pub(crate) fn queue_command_minecart_execution_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) {
        let request = {
            let entity = self
                .level(dimension)
                .entities
                .get(entity_id)
                .expect("command minecart is live");
            let crate::entity::EntityData::CommandBlockMinecart(data) = entity.data() else {
                panic!("expected command minecart")
            };
            crate::entity::PendingCommandMinecartExecution {
                entity_id,
                entity_uuid: entity.uuid(),
                position: entity.position(),
                y_rot: entity.y_rot(),
                x_rot: entity.x_rot(),
                command_blocks_work: true,
                snapshot: data.execution_snapshot(),
            }
        };
        self.level_mut(dimension)
            .entities
            .queue_command(crate::entity::EntityCommand::CommandMinecart(request));
    }

    pub(crate) fn run_pending_command_minecart_executions_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.run_entity_commands(&dimension.into(), events);
    }
}

fn timestamped_command_output(
    message: &RichTextComponent,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> RichTextComponent {
    let prefix = component_literal_nbt(format!("[{}] ", Local::now().format("%H:%M:%S")));
    let component = component_appended_nbt(
        &ComponentStyle::default(),
        vec![prefix, message.component().clone()],
    )
    .expect("command output components are validated");
    RichTextComponent::try_from_component_nbt_with_registry(component, registry)
        .expect("command output components are validated")
}
