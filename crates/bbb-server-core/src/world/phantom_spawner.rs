use super::{ServerWorld, WorldEvent};
use crate::connection::ConnectionId;
use crate::world::LevelId;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::RichTextComponent;
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::stats::{custom_stat, CUSTOM_STAT_TIME_SINCE_REST};

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct PhantomSpawner {
    pub(super) next_tick: i32,
}

impl ServerWorld {
    pub(super) fn tick_phantom_spawner(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        if dimension.builtin() != Some(Java26BuiltinDimension::Overworld)
            || !self
                .game_rules
                .spawning_monsters(self.admin.settings().difficulty)
            || self.admin.game_rule_value("minecraft:spawn_phantoms")
                != Some(GameRuleValue::Boolean(true))
        {
            return;
        }
        if let Err(detail) = self.advance_phantom_spawner(dimension, events) {
            events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                format!("phantom spawn: {detail}"),
            )));
        }
    }

    fn advance_phantom_spawner(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        // PhantomSpawner.java:29-32.
        self.cold.phantom_spawner.next_tick = self.cold.phantom_spawner.next_tick.wrapping_sub(1);
        if self.cold.phantom_spawner.next_tick > 0 {
            return Ok(());
        }
        let delay = (60 + self.random.next_int(60)) * 20;
        self.cold.phantom_spawner.next_tick =
            self.cold.phantom_spawner.next_tick.wrapping_add(delay);
        let level = &self.levels[dimension];
        let has_skylight = level.chunks.has_skylight();
        if has_skylight && level.sky_darkening.ok_or("missing sky darkening context")? < 5 {
            return Ok(());
        }
        for id in level.player_order.clone() {
            if let Err(detail) = self.try_spawn_player_phantoms(dimension, id, has_skylight, events)
            {
                events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                    format!("phantom spawn for {id:?}: {detail}"),
                )));
            }
        }
        Ok(())
    }

    fn try_spawn_player_phantoms(
        &mut self,
        dimension: &LevelId,
        id: ConnectionId,
        has_skylight: bool,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let player = self
            .levels
            .player(id)
            .ok_or("ordered player missing from world")?;
        if player.game_type == GameType::Spectator {
            return Ok(());
        }
        let position = BlockPos {
            x: player.position().x.floor() as i32,
            y: player.position().y.floor() as i32,
            z: player.position().z.floor() as i32,
        };
        if has_skylight {
            if position.y < self.levels[dimension].chunks.sea_level() {
                return Ok(());
            }
            if !self.flush_level_light(dimension, "phantom-spawn", events) {
                return Err("player sky light could not be committed".into());
            }
            if self.levels[dimension]
                .chunks
                .sky_light_level(position.x, position.y, position.z)
                < 15
            {
                return Ok(());
            }
        }
        let difficulty = self
            .effective_difficulty_at(dimension, position)
            .ok_or("missing positional difficulty context")?;
        if difficulty.partial_cmp(&(self.random.next_float() * 3.0))
            != Some(std::cmp::Ordering::Greater)
        {
            return Ok(());
        }
        let rest = self.levels[id]
            .cold
            .stats
            .value(custom_stat(CUSTOM_STAT_TIME_SINCE_REST))
            .clamp(1, i32::MAX);
        let random = &self.random;
        if random.next_int(rest) < 72_000 {
            return Ok(());
        }
        // PhantomSpawner.java:40-43: candidate draws are Y, X, then Z.
        let spawn_position = BlockPos {
            y: position.y.wrapping_add(20 + random.next_int(15)),
            x: position.x.wrapping_add(-10 + random.next_int(21)),
            z: position.z.wrapping_add(-10 + random.next_int(21)),
        };
        self.spawn_phantom_group(dimension, spawn_position, position, difficulty)?;
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests;
