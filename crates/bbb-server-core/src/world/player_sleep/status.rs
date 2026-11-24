use super::*;
use crate::world::LevelId;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_server_types::geometry::Aabb;

#[derive(Clone, Default)]
pub(in crate::world) struct SleepStatus {
    pub active: i32,
    pub sleeping: i32,
}

impl ServerWorld {
    pub(in crate::world) fn sleep_percentage(&self) -> i32 {
        match self
            .admin
            .game_rule_value("minecraft:players_sleeping_percentage")
        {
            Some(GameRuleValue::Integer(value)) => value,
            _ => 100,
        }
    }

    fn sleepers_needed(&self, dimension: &LevelId) -> i32 {
        ((self.level_by_id(dimension).sleep_status.active as f32 * self.sleep_percentage() as f32
            / 100.0_f32)
            .ceil() as i32)
            .max(1)
    }

    // SleepStatus.java:33-49; ServerLevel.java:636-657.
    pub(in crate::world) fn update_sleeping_player_list(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let order = &self.level_by_id(dimension).player_order;
        let mut active = 0;
        let mut sleeping = 0;
        for id in order {
            if let Some(player) = self.levels.player(*id) {
                if player.game_type != GameType::Spectator {
                    active += 1;
                    if player.cold.sleeping_pos.is_some() {
                        sleeping += 1;
                    }
                }
            }
        }
        let old = &self.level_by_id(dimension).sleep_status;
        let changed = (old.sleeping > 0 || sleeping > 0)
            && (old.active != active || old.sleeping != sleeping);
        self.level_mut_by_id(dimension).sleep_status = SleepStatus { active, sleeping };
        if !changed || self.sleep_percentage() > 100 {
            return;
        }
        let needed = self.sleepers_needed(dimension);
        let message = if sleeping >= needed {
            translated("sleep.skipping_night", Vec::new())
        } else {
            translated(
                "sleep.players_sleeping",
                vec![
                    bbb_protocol::NbtSummaryValue::Int(sleeping),
                    bbb_protocol::NbtSummaryValue::Int(needed),
                ],
            )
        };
        for id in self.level_by_id(dimension).player_order.clone() {
            self.sleep_message(id, message.clone(), true, events);
        }
    }

    // ServerLevel.java:353-364: the sleep phase runs even while the world is frozen.
    pub(in crate::world) fn tick_sleeping_level(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let needed = self.sleepers_needed(dimension);
        if self.level_by_id(dimension).sleep_status.sleeping < needed {
            return;
        }
        let deep = self
            .level_by_id(dimension)
            .player_order
            .iter()
            .filter(|id| {
                self.levels
                    .player(**id)
                    .is_some_and(|p| p.cold.sleeping_pos.is_some() && p.cold.sleep_timer >= 100)
            })
            .count() as i32;
        if deep < needed {
            return;
        }
        let ids: Vec<_> = self
            .level_by_id(dimension)
            .player_order
            .iter()
            .copied()
            .filter(|id| {
                self.levels
                    .player(*id)
                    .is_some_and(|p| p.cold.sleeping_pos.is_some())
            })
            .collect();
        if ids.iter().any(|&id| self.sleep_wake_landing(id).is_none()) {
            return;
        }
        if self.game_rule_bool("minecraft:advance_time", true) {
            self.move_sleep_clock_to_marker(dimension, events);
        }
        self.level_mut_by_id(dimension).sleep_status.sleeping = 0;
        for id in ids {
            self.stop_player_sleep(id, false, false, events);
        }
        if self.game_rule_bool("minecraft:advance_weather", true)
            && self.level_by_id(dimension).weather.is_raining()
        {
            self.weather.rain_time = 0;
            self.weather.raining = false;
            self.weather.thunder_time = 0;
            self.weather.thundering = false;
        }
    }

    pub(super) fn monster_prevents_rest(&self, id: ConnectionId, pos: BlockPos) -> bool {
        let player = self.levels.player_at(id);
        let center = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y),
            z: f64::from(pos.z) + 0.5,
        };
        let bounds = Aabb {
            min_x: center.x - 8.0,
            min_y: center.y - 5.0,
            min_z: center.z - 8.0,
            max_x: center.x + 8.0,
            max_y: center.y + 5.0,
            max_z: center.z + 8.0,
        };
        let can_attack_player = !player.damage.is_dead_or_dying()
            && !player.abilities.invulnerable
            && !player.entity_invulnerable
            && player.game_type != GameType::Spectator
            && self.admin.settings().difficulty != bbb_protocol::difficulties::Difficulty::Peaceful;
        let entities = &self.level_by_id(&player.dimension).entities;
        entities
            .first_entity_in_box(bounds, |entity| {
                entity.removal_reason().is_none()
                    && entity.is_preventing_player_rest(
                        player.profile.uuid,
                        can_attack_player,
                        self.game_time(),
                        self.game_rule_bool("minecraft:universal_anger", false),
                    )
            })
            .is_some()
    }
}
