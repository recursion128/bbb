//! Automatic scoreboard criteria from session values, deaths, kills and statistics.

use crate::connection::ConnectionId;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ARMOR_ID;

use super::display_names::player_display_name;
use super::server_scoreboard::ServerScoreboard;
use super::server_scoreboard::ServerScoreboardEffect;
use super::{LevelId, PlayerState, ServerWorld, WorldEvent};
use bbb_server_types::stats::StatKey;

/// `ServerPlayer.lastRecorded*` (`ServerPlayer.java:232-237`). A new
/// `ServerPlayer` starts every one at its type's minimum, so login and respawn
/// both rewrite all six scores on their first tick; `restoreFrom` copies none.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RecordedCriteria {
    health_and_absorption: f32,
    food_level: i32,
    air: i32,
    armor: i32,
    experience: i32,
    level: i32,
}

impl Default for RecordedCriteria {
    fn default() -> Self {
        Self {
            // Java's `Float.MIN_VALUE`, the smallest denormal.
            health_and_absorption: f32::from_bits(1),
            food_level: i32::MIN,
            air: i32::MIN,
            armor: i32::MIN,
            experience: i32::MIN,
            level: i32::MIN,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum KillScoreVictim<'a> {
    Player(ConnectionId),
    Stored {
        dimension: &'a LevelId,
        entity_id: i32,
    },
}

impl ServerWorld {
    /// `ServerPlayer.doTick`'s criteria arm (`ServerPlayer.java:680-708`),
    /// between the health packet and the experience packet and, like both,
    /// outside the body-tick gate.
    pub(super) fn tick_player_score_criteria(&mut self, events: &mut Vec<WorldEvent>) {
        if !self.scoreboard.board().has_ticked_criteria_objectives() {
            for player in self.levels.players_mut() {
                player.cold.recorded_criteria = RecordedCriteria {
                    health_and_absorption: player.damage.health + player.damage.absorption_amount,
                    food_level: player.food.food_level(),
                    air: player.air_supply,
                    // `LivingEntity.getArmorValue` (`LivingEntity.java:1845-1847`).
                    armor: player.attribute_value(VANILLA_ATTRIBUTE_ARMOR_ID).floor() as i32,
                    experience: player.experience.total(),
                    level: player.experience.level(),
                };
            }
            self.flush_scoreboard_effects(events);
            return;
        }
        let mut changed_players = Vec::new();
        for player in self.levels.players_mut() {
            let recorded = player.cold.recorded_criteria;
            let current = RecordedCriteria {
                health_and_absorption: player.damage.health + player.damage.absorption_amount,
                food_level: player.food.food_level(),
                air: player.air_supply,
                // `LivingEntity.getArmorValue` (`LivingEntity.java:1845-1847`).
                armor: player.attribute_value(VANILLA_ATTRIBUTE_ARMOR_ID).floor() as i32,
                experience: player.experience.total(),
                level: player.experience.level(),
            };
            if current == recorded {
                continue;
            }
            player.cold.recorded_criteria = current;
            changed_players.push((player.connection_id, recorded, current));
        }
        for (connection_id, recorded, current) in changed_players {
            // Java's float `!=` and `Mth.ceil`.
            #[allow(clippy::float_cmp)]
            let health_changed = current.health_and_absorption != recorded.health_and_absorption;
            let updates = [
                (
                    health_changed,
                    "health",
                    current.health_and_absorption.ceil() as i32,
                ),
                (
                    current.food_level != recorded.food_level,
                    "food",
                    current.food_level,
                ),
                (current.air != recorded.air, "air", current.air),
                (current.armor != recorded.armor, "armor", current.armor),
                (
                    current.experience != recorded.experience,
                    "xp",
                    current.experience,
                ),
                (current.level != recorded.level, "level", current.level),
            ];
            for (changed, criteria, value) in updates {
                if changed {
                    self.for_player_objectives(connection_id, criteria, |_| value);
                }
            }
        }
        self.flush_scoreboard_effects(events);
    }

    /// `ServerPlayer.die`'s `forAllObjectives(DEATH_COUNT, this,
    /// ScoreAccess::increment)` (`ServerPlayer.java:911`).
    pub(super) fn increment_death_count_scores(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        self.for_player_objectives(connection_id, "deathCount", |score| score.wrapping_add(1));
        self.flush_scoreboard_effects(events);
    }

    /// `ServerPlayer.awardKillScore` and `handleTeamKill` (`ServerPlayer.java:941-968`).
    pub(super) fn increment_kill_count_scores(
        &mut self,
        killer: ConnectionId,
        victim: KillScoreVictim<'_>,
    ) {
        if self.scoreboard.board().objectives().is_empty() {
            return;
        }
        self.for_player_objectives(killer, "totalKillCount", |score| score.wrapping_add(1));
        let victim_owner = match victim {
            KillScoreVictim::Player(player) => {
                self.for_player_objectives(killer, "playerKillCount", |score| {
                    score.wrapping_add(1)
                });
                self.levels.player_at(player).profile.name.clone()
            }
            KillScoreVictim::Stored {
                dimension,
                entity_id,
            } => {
                let Some(entity) = self.level_by_id(dimension).entities.get(entity_id) else {
                    return;
                };
                entity.uuid().to_string()
            }
        };
        let board = self.scoreboard.board();
        let victim_color = board
            .players_team(&victim_owner)
            .map(|team| team.color)
            .filter(|color| color.color().is_some());
        let killer_color = board
            .players_team(&self.levels.player_at(killer).profile.name)
            .map(|team| team.color)
            .filter(|color| color.color().is_some());
        if let Some(color) = victim_color {
            self.for_player_objectives(killer, &format!("teamkill.{}", color.as_str()), |score| {
                score.wrapping_add(1)
            });
        }
        if let Some(color) = killer_color {
            let criteria = format!("killedByTeam.{}", color.as_str());
            match victim {
                KillScoreVictim::Player(player) => {
                    self.for_player_objectives(player, &criteria, |score| score.wrapping_add(1));
                }
                KillScoreVictim::Stored {
                    dimension,
                    entity_id,
                } => {
                    let display =
                        self.scoreboard
                            .criteria_reads_display_name(&criteria)
                            .then(|| {
                                super::display_names::stored_display_name(
                                    self.level_by_id(dimension)
                                        .entities
                                        .get(entity_id)
                                        .expect("victim is still stored"),
                                    self.scoreboard.board().players_team(&victim_owner),
                                )
                            });
                    self.scoreboard.for_all_objectives(
                        &criteria,
                        &victim_owner,
                        display.as_ref(),
                        |score| score.wrapping_add(1),
                    );
                }
            }
        }
    }

    /// `Scoreboard.forAllObjectives` with the session as the score holder: its
    /// scoreboard name, and `Player.getDisplayName` only when an auto-updating
    /// objective will read it (`Scoreboard.java:93-98`).
    fn for_player_objectives(
        &mut self,
        connection_id: ConnectionId,
        criteria: &str,
        value: impl Fn(i32) -> i32,
    ) {
        self.levels
            .player_at(connection_id)
            .for_objectives(&mut self.scoreboard, criteria, value);
    }

    /// Broadcasts what the live scoreboard queued, the way
    /// `PlayerList.broadcastAll` sends `ServerScoreboard`'s packets.
    pub(super) fn flush_scoreboard_effects(&mut self, events: &mut Vec<WorldEvent>) {
        for effect in self.scoreboard.take_pending_effects() {
            match effect {
                ServerScoreboardEffect::Packet(packet) => self.broadcast(*packet, events),
                ServerScoreboardEffect::RemakePlayerWaypoint {
                    player,
                    team_color_rgb,
                    all_levels,
                } => {
                    player.with_utf8(|player| {
                        self.waypoint_remake_player_by_name(
                            player,
                            team_color_rgb,
                            all_levels,
                            events,
                        )
                    });
                }
            }
        }
    }
}

impl PlayerState {
    /// `ServerPlayer.awardStat` (`ServerPlayer.java:1497-1500`).
    pub(super) fn award_stat(
        &mut self,
        scoreboard: &mut ServerScoreboard,
        stat: StatKey,
        count: i32,
    ) {
        self.cold.stats.increment(stat, count);
        if !scoreboard.board().has_stat_objectives() {
            return;
        }
        if let Some(criteria) = stat_criteria(stat) {
            self.for_objectives(scoreboard, &criteria, |value| value.wrapping_add(count));
        }
    }

    pub(super) fn award_stat_once(&mut self, scoreboard: &mut ServerScoreboard, stat: StatKey) {
        self.award_stat(scoreboard, stat, 1);
    }

    /// `ServerPlayer.resetStat` (`ServerPlayer.java:1503-1506`).
    pub(super) fn reset_stat(&mut self, scoreboard: &mut ServerScoreboard, stat: StatKey) {
        self.cold.stats.set_value(stat, 0);
        if !scoreboard.board().has_stat_objectives() {
            return;
        }
        if let Some(criteria) = stat_criteria(stat) {
            self.for_objectives(scoreboard, &criteria, |_| 0);
        }
    }

    fn for_objectives(
        &self,
        scoreboard: &mut ServerScoreboard,
        criteria: &str,
        value: impl Fn(i32) -> i32,
    ) {
        let owner = &self.profile.name;
        let display_name = scoreboard
            .criteria_reads_display_name(criteria)
            .then(|| player_display_name(&self.profile, scoreboard.board().players_team(owner)));
        scoreboard.for_all_objectives(criteria, owner, display_name.as_ref(), value);
    }
}

/// `Stat.buildName` (`Stat.java:27-32`).
fn stat_criteria(stat: StatKey) -> Option<String> {
    let (kind, value) = stat.resource_ids()?;
    let mut criteria = String::with_capacity(kind.len() + value.len() + 1);
    criteria.extend(kind.chars().map(|c| if c == ':' { '.' } else { c }));
    criteria.push(':');
    criteria.extend(value.chars().map(|c| if c == ':' { '.' } else { c }));
    Some(criteria)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{join_loaded, mailbox};
    use crate::world::ServerWorldConfig;

    #[test]
    fn scoreboard_statistic_uses_the_award_delta_independently_of_the_counter() {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let stat = bbb_server_types::stats::custom_stat("minecraft:jump");
        let criteria = "minecraft.custom:minecraft.jump";
        world.scoreboard.add_objective(
            "jumps",
            criteria,
            bbb_protocol::NbtSummaryValue::String("Jumps".into()),
            bbb_protocol::objective_render_types::ObjectiveRenderType::Integer,
        );
        let player = &mut world.levels[ConnectionId(1)];
        player.cold.stats.set_value(stat, i32::MAX);
        world
            .scoreboard
            .for_all_objectives(criteria, "alice", None, |_| i32::MAX - 1);
        player.award_stat(&mut world.scoreboard, stat, 3);
        assert_eq!(player.cold.stats.value(stat), i32::MAX);
        assert_eq!(
            world
                .scoreboard
                .board()
                .score("alice", "jumps")
                .unwrap()
                .value,
            i32::MIN + 1
        );
        player.award_stat(&mut world.scoreboard, stat, -2);
        assert_eq!(player.cold.stats.value(stat), i32::MAX - 2);
        assert_eq!(
            world
                .scoreboard
                .board()
                .score("alice", "jumps")
                .unwrap()
                .value,
            i32::MAX
        );
        player.reset_stat(&mut world.scoreboard, stat);
        assert_eq!(player.cold.stats.value(stat), 0);
        assert_eq!(
            world
                .scoreboard
                .board()
                .score("alice", "jumps")
                .unwrap()
                .value,
            0
        );
    }
}
