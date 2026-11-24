use crate::block_behavior::command_block;
use crate::block_entity::BlockEntity;
use crate::command_block_execution::{
    begin_command_block_execution, finish_command_block_execution, CommandBlockExecutionPlan,
};
use crate::connection::ConnectionId;
use crate::world::LevelId;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_id_by_name_and_properties,
};
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::{BlockPos, CommandBlockMode, Direction, SetCommandBlock, SystemChat};
use bbb_server_types::directions::relative;

use crate::commands::{self, CommandSource};
use crate::world::{ServerWorld, WorldEvent};

impl ServerWorld {
    pub(in crate::world) fn apply_set_command_block(
        &mut self,
        connection_id: ConnectionId,
        packet: SetCommandBlock,
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
        let Some(original) = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(packet.pos)
            .filter(|entity| entity.command_block().is_some())
            .cloned()
        else {
            return;
        };
        let old_state = original.block_state();
        let old_mode = command_block::mode(old_state);
        let name = match packet.mode {
            CommandBlockMode::Sequence => "minecraft:chain_command_block",
            CommandBlockMode::Auto => "minecraft:repeating_command_block",
            CommandBlockMode::Redstone => "minecraft:command_block",
        };
        let properties = std::collections::BTreeMap::from([
            (
                "facing".to_owned(),
                direction_name(command_block::facing(old_state)).to_owned(),
            ),
            ("conditional".to_owned(), packet.conditional.to_string()),
        ]);
        let state = vanilla_block_state_id_by_name_and_properties(name, &properties)
            .expect("command block mode has generated facing and conditional properties");
        if state != old_state {
            self.set_block(dimension, packet.pos, state, 2);
            let mut entities = self.level_mut_by_id(&dimension).chunks.block_entities_mut();
            entities.add_and_register(original);
            entities.rebind_block_state(packet.pos, state);
        }
        self.with_level(dimension, |level| {
            if let Some(block) = level.command_block_mut(packet.pos) {
                block.set_command(packet.command.clone());
                block.set_track_output(packet.track_output);
                if !packet.track_output {
                    block.set_last_output(None);
                }
            }
            command_block::set_automatic(level, packet.pos, state, packet.automatic);
            if old_mode != packet.mode {
                command_block::mode_switched(level, packet.pos, state);
            }
        });
        self.mark_command_block_changed(
            &dimension,
            packet.pos,
            self.game_rules.command_blocks_work,
        );
        if !packet.command.is_empty() {
            self.send_system_message(
                connection_id,
                SystemChat::from_component(
                    commands::command_block_set_feedback(
                        &packet.command,
                        self.game_rules.command_blocks_work,
                    ),
                    false,
                ),
                events,
            );
        }
    }

    pub(in crate::world) fn tick_command_block(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        state: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(original) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .filter(|entity| entity.command_block().is_some())
            .cloned()
        else {
            return;
        };
        let block = original.command_block().expect("command block was checked");
        let mode = command_block::mode(state);
        // `CommandBlock.tick` (`CommandBlock.java:93-100`): AUTO records the
        // previous `wasConditionMet`, refreshes `markConditionMet` for next
        // tick, and gates this tick's execution on `wasConditionMet`.
        let was_condition_met = block.condition_met();
        if mode == CommandBlockMode::Auto {
            self.with_level(dimension, |level| {
                let _ = command_block::mark_condition_met(level, pos, state);
            });
        }
        if mode != CommandBlockMode::Sequence {
            if was_condition_met {
                if block.command().is_empty() {
                    self.clear_command_block_success(dimension, pos);
                } else {
                    self.perform_command_block(dimension, pos, state, events);
                }
                self.execute_command_block_chain(
                    dimension,
                    pos,
                    command_block::facing(state),
                    events,
                );
            } else if command_block::conditional(state) {
                self.clear_command_block_success(dimension, pos);
            }
        }
        let flags = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .filter(|entity| entity.same_position_and_type(&original))
            .and_then(BlockEntity::command_block)
            .map(|block| (block.powered(), block.automatic()))
            .unwrap_or((block.powered(), block.automatic()));
        if mode == CommandBlockMode::Auto && (flags.0 || flags.1) {
            self.with_level(dimension, |level| {
                if let Some(kind) =
                    bbb_server_types::chunk_ticks::BlockTickType::of_block_state(state)
                {
                    level.schedule_block_tick(
                        pos,
                        kind,
                        1,
                        bbb_server_types::chunk_ticks::TickPriority::Normal,
                    );
                }
            });
        }
        self.update_command_block_output(dimension, pos, state);
    }

    fn execute_command_block_chain(
        &mut self,
        dimension: &LevelId,
        mut pos: BlockPos,
        mut facing: Direction,
        events: &mut Vec<WorldEvent>,
    ) {
        let mut remaining = match self
            .admin
            .game_rule_value("minecraft:max_command_sequence_length")
        {
            Some(GameRuleValue::Integer(value)) => value,
            _ => return,
        };
        while remaining > 0 {
            remaining -= 1;
            pos = relative(pos, facing);
            let state = super::super::level_blocks::stored_block_state(
                &self.level_by_id(dimension).chunks,
                pos,
            );
            if !command_block::is_command_block(state)
                || command_block::mode(state) != CommandBlockMode::Sequence
            {
                break;
            }
            let Some(block) = self
                .level_by_id(dimension)
                .chunks
                .block_entities()
                .get(pos)
                .and_then(BlockEntity::command_block)
            else {
                break;
            };
            if block.powered() || block.automatic() {
                let met = self.with_level(dimension, |level| {
                    command_block::mark_condition_met(level, pos, state)
                });
                if met {
                    if !self.perform_command_block(dimension, pos, state, events) {
                        break;
                    }
                    self.update_command_block_output(dimension, pos, state);
                } else if command_block::conditional(state) {
                    self.clear_command_block_success(dimension, pos);
                }
            }
            facing = command_block::facing(state);
        }
        if remaining <= 0 {
            tracing::warn!(
                "Command Block chain tried to execute more than the configured number of steps"
            );
        }
    }

    fn perform_command_block(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        state: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some(original) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .cloned()
        else {
            return false;
        };
        let Some(block) = original.command_block() else {
            return false;
        };
        let mut snapshot = block.execution_snapshot();
        let game_time = self.game_time();
        let plan = begin_command_block_execution(
            &mut snapshot,
            game_time,
            self.game_rules.command_blocks_work,
        );
        if plan == CommandBlockExecutionPlan::Skip {
            return false;
        }
        self.level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::command_block_mut)
            .expect("original command block is still present")
            .apply_execution_result(snapshot.clone());
        if plan == CommandBlockExecutionPlan::Execute {
            let yaw = match command_block::facing(state) {
                Direction::South => 0.0,
                Direction::West => 90.0,
                Direction::North => 180.0,
                Direction::East | Direction::Up | Direction::Down => 270.0,
            };
            let (success, output) = self.execute_command_block_command(
                &snapshot.command,
                CommandSource::block(block.name()),
                commands::CommandBlockResultTarget::Block {
                    level: self.level_key(*dimension).clone(),
                    instance: Box::new(original.clone()),
                },
                crate::world::command_minecarts::CommandBlockInvocation {
                    position: [
                        f64::from(pos.x) + 0.5,
                        f64::from(pos.y) + 0.5,
                        f64::from(pos.z) + 0.5,
                    ],
                    rotation: [yaw, 0.0],
                    dimension,
                    track_output: snapshot.track_output,
                },
                events,
            );
            let Some(current) = self
                .level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .filter(|entity| entity.same_position_and_type(&original))
                .and_then(BlockEntity::command_block_mut)
            else {
                return true;
            };
            let mut snapshot = current.execution_snapshot();
            finish_command_block_execution(&mut snapshot, game_time, success, output);
            current.apply_execution_result(snapshot);
        }
        self.mark_command_block_changed(
            dimension,
            pos,
            plan == CommandBlockExecutionPlan::Execute && snapshot.track_output,
        );
        true
    }

    fn clear_command_block_success(&mut self, dimension: &LevelId, pos: BlockPos) {
        if let Some(block) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::command_block_mut)
        {
            block.set_success_count(0);
        }
        self.mark_command_block_changed(dimension, pos, false);
    }

    fn mark_command_block_changed(&mut self, dimension: &LevelId, pos: BlockPos, broadcast: bool) {
        let level = self.level_mut_by_id(dimension);
        level.chunks.block_entities_mut().set_changed(pos);
        if broadcast {
            level.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
        }
    }

    fn update_command_block_output(&mut self, dimension: &LevelId, pos: BlockPos, state: i32) {
        if let Some(block) = vanilla_block_state_block_type_id(state) {
            self.with_level(dimension, |level| {
                level.update_neighbour_for_output_signal(pos, block);
            });
        }
    }
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}
