use super::{ServerWorld, WorldEvent};
use crate::world::LevelId;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::game_types::GameType;
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::RichTextComponent;
use bbb_server_types::save::Java26BuiltinDimension;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct PatrolSpawner {
    next_tick: i32,
}

impl ServerWorld {
    pub(super) fn tick_patrol_spawner(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        if dimension.builtin() != Some(Java26BuiltinDimension::Overworld)
            || !self
                .game_rules
                .spawning_monsters(self.admin.settings().difficulty)
            || self.admin.game_rule_value("minecraft:spawn_patrols")
                != Some(GameRuleValue::Boolean(true))
        {
            return;
        }
        if let Err(detail) = self.advance_patrol_spawner(dimension, events) {
            events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                format!("patrol spawn: {detail}"),
            )));
        }
    }

    fn advance_patrol_spawner(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        // PatrolSpawner.java:23-38: the reset adds to the decremented counter before daylight.
        self.cold.patrol_spawner.next_tick = self.cold.patrol_spawner.next_tick.wrapping_sub(1);
        if self.cold.patrol_spawner.next_tick > 0 {
            return Ok(());
        }
        let delay = self.random.next_int(1_200);
        self.cold.patrol_spawner.next_tick = self
            .cold
            .patrol_spawner
            .next_tick
            .wrapping_add(12_000)
            .wrapping_add(delay);
        if !self
            .is_bright_outside(dimension)
            .ok_or("missing dimension brightness context")?
            || self.random.next_int(5) != 0
        {
            return Ok(());
        }
        let count = self.levels[dimension].player_order.len();
        if count == 0 {
            return Ok(());
        }
        let index = self
            .random
            .next_int(i32::try_from(count).map_err(|_| "player count exceeds Java list bounds")?)
            as usize;
        let id = self.levels[dimension].player_order[index];
        let player = self
            .levels
            .player(id)
            .ok_or("ordered player missing from world")?;
        if player.game_type == GameType::Spectator {
            return Ok(());
        }
        let center = BlockPos {
            x: player.position().x.floor() as i32,
            y: player.position().y.floor() as i32,
            z: player.position().z.floor() as i32,
        };
        if self
            .is_close_to_village_id(dimension, [center.x, center.y, center.z], 2)
            .ok_or("missing village POI tag context")?
        {
            return Ok(());
        }
        let random = &self.random;
        let x = (24 + random.next_int(24)) * if random.next_boolean() { -1 } else { 1 };
        let z = (24 + random.next_int(24)) * if random.next_boolean() { -1 } else { 1 };
        let mut position = BlockPos {
            x: center.x.wrapping_add(x),
            y: center.y,
            z: center.z.wrapping_add(z),
        };
        if !self.custom_spawn_has_chunks_at(dimension, position)
            || !self
                .can_pillager_patrol_spawn(dimension, position)
                .ok_or("missing patrol environment attribute context")?
        {
            return Ok(());
        }
        // PatrolSpawner.java:39-54: sample difficulty before height adjustment; failed followers continue.
        let count = self
            .effective_difficulty_at(dimension, position)
            .ok_or("missing positional difficulty context")?
            .ceil() as i32
            + 1;
        for index in 0..count {
            position.y = self.custom_spawn_height(
                dimension,
                position.x,
                position.z,
                VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
            )?;
            if !self.flush_level_light(dimension, "patrol-spawn", events) {
                return Err("candidate light could not be committed".into());
            }
            if !self.spawn_patrol_member(dimension, position, index == 0)? && index == 0 {
                break;
            }
            let random = &self.random;
            position.x = position
                .x
                .wrapping_add(random.next_int(5))
                .wrapping_sub(random.next_int(5));
            position.z = position
                .z
                .wrapping_add(random.next_int(5))
                .wrapping_sub(random.next_int(5));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
