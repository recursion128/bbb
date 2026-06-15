use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::{
    ChatFormatting as ProtocolChatFormatting, ObjectiveRenderType as ProtocolObjectiveRenderType,
    PlayerTeamMethod as ProtocolPlayerTeamMethod,
    PlayerTeamParameters as ProtocolPlayerTeamParameters, ResetScore as ProtocolResetScore,
    ScoreboardDisplaySlot as ProtocolScoreboardDisplaySlot,
    SetDisplayObjective as ProtocolSetDisplayObjective, SetObjective as ProtocolSetObjective,
    SetObjectiveMethod as ProtocolSetObjectiveMethod, SetPlayerTeam as ProtocolSetPlayerTeam,
    SetScore as ProtocolSetScore, TeamCollisionRule as ProtocolTeamCollisionRule,
    TeamVisibility as ProtocolTeamVisibility,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreboardState {
    pub objectives: BTreeMap<String, ScoreboardObjective>,
    pub display_slots: BTreeMap<String, String>,
    pub scores: BTreeMap<String, BTreeMap<String, ScoreboardScore>>,
    pub teams: BTreeMap<String, ScoreboardTeam>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreboardObjective {
    pub name: String,
    pub display_name: String,
    pub render_type: String,
    pub number_format: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreboardScore {
    pub value: i32,
    pub display: Option<String>,
    pub number_format: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreboardTeam {
    pub name: String,
    pub parameters: Option<ScoreboardTeamParameters>,
    pub players: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreboardTeamParameters {
    pub display_name: String,
    pub options: i32,
    pub nametag_visibility: String,
    pub collision_rule: String,
    pub color: String,
    pub player_prefix: String,
    pub player_suffix: String,
}

impl WorldStore {
    pub fn apply_set_objective(&mut self, packet: ProtocolSetObjective) -> bool {
        self.counters.set_objective_packets += 1;

        let applied = match packet.method {
            ProtocolSetObjectiveMethod::Add => {
                let Some(parameters) = packet.parameters else {
                    return self.finish_set_objective_update(false);
                };
                let objective = ScoreboardObjective {
                    name: packet.objective_name.clone(),
                    display_name: parameters.display_name,
                    render_type: objective_render_type_name(parameters.render_type),
                    number_format: parameters.number_format,
                };
                self.scoreboard
                    .objectives
                    .insert(packet.objective_name, objective);
                true
            }
            ProtocolSetObjectiveMethod::Remove => {
                self.remove_scoreboard_objective(&packet.objective_name)
            }
            ProtocolSetObjectiveMethod::Change => {
                let Some(parameters) = packet.parameters else {
                    return self.finish_set_objective_update(false);
                };
                let Some(objective) = self.scoreboard.objectives.get_mut(&packet.objective_name)
                else {
                    return self.finish_set_objective_update(false);
                };
                objective.display_name = parameters.display_name;
                objective.render_type = objective_render_type_name(parameters.render_type);
                objective.number_format = parameters.number_format;
                true
            }
        };
        self.finish_set_objective_update(applied)
    }

    pub fn apply_set_score(&mut self, packet: ProtocolSetScore) -> bool {
        self.counters.set_score_packets += 1;
        if !self
            .scoreboard
            .objectives
            .contains_key(&packet.objective_name)
        {
            return self.finish_set_score_update(false);
        }

        self.scoreboard
            .scores
            .entry(packet.owner)
            .or_default()
            .insert(
                packet.objective_name,
                ScoreboardScore {
                    value: packet.score,
                    display: packet.display,
                    number_format: packet.number_format,
                },
            );
        self.finish_set_score_update(true)
    }

    pub fn apply_reset_score(&mut self, packet: ProtocolResetScore) -> bool {
        self.counters.reset_score_packets += 1;
        let Some(objective_name) = packet.objective_name else {
            let applied = self.scoreboard.scores.remove(&packet.owner).is_some();
            return self.finish_reset_score_update(applied);
        };
        if !self.scoreboard.objectives.contains_key(&objective_name) {
            return self.finish_reset_score_update(false);
        }

        let Some(scores) = self.scoreboard.scores.get_mut(&packet.owner) else {
            return self.finish_reset_score_update(false);
        };
        let removed = scores.remove(&objective_name).is_some();
        if scores.is_empty() {
            self.scoreboard.scores.remove(&packet.owner);
        }
        self.finish_reset_score_update(removed)
    }

    pub fn apply_set_display_objective(&mut self, packet: ProtocolSetDisplayObjective) -> bool {
        self.counters.set_display_objective_packets += 1;
        let slot = scoreboard_display_slot_name(packet.slot);
        let Some(objective_name) = packet.objective_name.filter(|name| !name.is_empty()) else {
            let applied = self.scoreboard.display_slots.remove(&slot).is_some();
            return self.finish_set_display_objective_update(applied);
        };
        if !self.scoreboard.objectives.contains_key(&objective_name) {
            let applied = self.scoreboard.display_slots.remove(&slot).is_some();
            return self.finish_set_display_objective_update(applied);
        }

        self.scoreboard.display_slots.insert(slot, objective_name);
        self.finish_set_display_objective_update(true)
    }

    pub fn apply_set_player_team(&mut self, packet: ProtocolSetPlayerTeam) -> bool {
        self.counters.set_player_team_packets += 1;

        let applied = match packet.method {
            ProtocolPlayerTeamMethod::Add => {
                self.scoreboard
                    .teams
                    .entry(packet.name.clone())
                    .or_insert_with(|| ScoreboardTeam {
                        name: packet.name.clone(),
                        parameters: None,
                        players: BTreeSet::new(),
                    });
                if let Some(parameters) = packet.parameters {
                    if let Some(team) = self.scoreboard.teams.get_mut(&packet.name) {
                        team.parameters = Some(scoreboard_team_parameters(parameters));
                    }
                }
                self.add_players_to_scoreboard_team(&packet.name, packet.players);
                true
            }
            ProtocolPlayerTeamMethod::Remove => {
                self.scoreboard.teams.remove(&packet.name).is_some()
            }
            ProtocolPlayerTeamMethod::Change => {
                let Some(team) = self.scoreboard.teams.get_mut(&packet.name) else {
                    return self.finish_set_player_team_update(false);
                };
                let Some(parameters) = packet.parameters else {
                    return self.finish_set_player_team_update(false);
                };
                team.parameters = Some(scoreboard_team_parameters(parameters));
                true
            }
            ProtocolPlayerTeamMethod::Join => {
                if !self.scoreboard.teams.contains_key(&packet.name) {
                    return self.finish_set_player_team_update(false);
                }
                self.add_players_to_scoreboard_team(&packet.name, packet.players);
                true
            }
            ProtocolPlayerTeamMethod::Leave => {
                let Some(team) = self.scoreboard.teams.get_mut(&packet.name) else {
                    return self.finish_set_player_team_update(false);
                };
                for player in packet.players {
                    team.players.remove(&player);
                }
                true
            }
        };
        self.finish_set_player_team_update(applied)
    }

    pub fn scoreboard(&self) -> &ScoreboardState {
        &self.scoreboard
    }

    fn remove_scoreboard_objective(&mut self, objective_name: &str) -> bool {
        if self.scoreboard.objectives.remove(objective_name).is_none() {
            return false;
        }

        self.scoreboard
            .display_slots
            .retain(|_, displayed_objective| displayed_objective != objective_name);
        self.scoreboard.scores.retain(|_, scores| {
            scores.remove(objective_name);
            !scores.is_empty()
        });
        true
    }

    fn add_players_to_scoreboard_team(&mut self, team_name: &str, players: Vec<String>) {
        for player in players {
            self.remove_scoreboard_player_from_other_teams(team_name, &player);
            if let Some(team) = self.scoreboard.teams.get_mut(team_name) {
                team.players.insert(player);
            }
        }
    }

    fn remove_scoreboard_player_from_other_teams(&mut self, team_name: &str, player: &str) {
        for (name, team) in &mut self.scoreboard.teams {
            if name.as_str() != team_name {
                team.players.remove(player);
            }
        }
    }

    fn finish_set_objective_update(&mut self, applied: bool) -> bool {
        if applied {
            self.counters.set_objective_updates_applied += 1;
        } else {
            self.counters.set_objective_updates_ignored += 1;
        }
        applied
    }

    fn finish_set_score_update(&mut self, applied: bool) -> bool {
        if applied {
            self.counters.set_score_updates_applied += 1;
        } else {
            self.counters.set_score_updates_ignored += 1;
        }
        applied
    }

    fn finish_reset_score_update(&mut self, applied: bool) -> bool {
        if applied {
            self.counters.reset_score_updates_applied += 1;
        } else {
            self.counters.reset_score_updates_ignored += 1;
        }
        applied
    }

    fn finish_set_display_objective_update(&mut self, applied: bool) -> bool {
        if applied {
            self.counters.set_display_objective_updates_applied += 1;
        } else {
            self.counters.set_display_objective_updates_ignored += 1;
        }
        applied
    }

    fn finish_set_player_team_update(&mut self, applied: bool) -> bool {
        if applied {
            self.counters.set_player_team_updates_applied += 1;
        } else {
            self.counters.set_player_team_updates_ignored += 1;
        }
        applied
    }
}

fn objective_render_type_name(render_type: ProtocolObjectiveRenderType) -> String {
    match render_type {
        ProtocolObjectiveRenderType::Integer => "integer",
        ProtocolObjectiveRenderType::Hearts => "hearts",
    }
    .to_string()
}

fn scoreboard_display_slot_name(slot: ProtocolScoreboardDisplaySlot) -> String {
    match slot {
        ProtocolScoreboardDisplaySlot::List => "list",
        ProtocolScoreboardDisplaySlot::Sidebar => "sidebar",
        ProtocolScoreboardDisplaySlot::BelowName => "below_name",
        ProtocolScoreboardDisplaySlot::TeamBlack => "sidebar.team.black",
        ProtocolScoreboardDisplaySlot::TeamDarkBlue => "sidebar.team.dark_blue",
        ProtocolScoreboardDisplaySlot::TeamDarkGreen => "sidebar.team.dark_green",
        ProtocolScoreboardDisplaySlot::TeamDarkAqua => "sidebar.team.dark_aqua",
        ProtocolScoreboardDisplaySlot::TeamDarkRed => "sidebar.team.dark_red",
        ProtocolScoreboardDisplaySlot::TeamDarkPurple => "sidebar.team.dark_purple",
        ProtocolScoreboardDisplaySlot::TeamGold => "sidebar.team.gold",
        ProtocolScoreboardDisplaySlot::TeamGray => "sidebar.team.gray",
        ProtocolScoreboardDisplaySlot::TeamDarkGray => "sidebar.team.dark_gray",
        ProtocolScoreboardDisplaySlot::TeamBlue => "sidebar.team.blue",
        ProtocolScoreboardDisplaySlot::TeamGreen => "sidebar.team.green",
        ProtocolScoreboardDisplaySlot::TeamAqua => "sidebar.team.aqua",
        ProtocolScoreboardDisplaySlot::TeamRed => "sidebar.team.red",
        ProtocolScoreboardDisplaySlot::TeamLightPurple => "sidebar.team.light_purple",
        ProtocolScoreboardDisplaySlot::TeamYellow => "sidebar.team.yellow",
        ProtocolScoreboardDisplaySlot::TeamWhite => "sidebar.team.white",
    }
    .to_string()
}

fn team_visibility_name(visibility: ProtocolTeamVisibility) -> String {
    match visibility {
        ProtocolTeamVisibility::Always => "always",
        ProtocolTeamVisibility::Never => "never",
        ProtocolTeamVisibility::HideForOtherTeams => "hideForOtherTeams",
        ProtocolTeamVisibility::HideForOwnTeam => "hideForOwnTeam",
    }
    .to_string()
}

fn team_collision_rule_name(rule: ProtocolTeamCollisionRule) -> String {
    match rule {
        ProtocolTeamCollisionRule::Always => "always",
        ProtocolTeamCollisionRule::Never => "never",
        ProtocolTeamCollisionRule::PushOtherTeams => "pushOtherTeams",
        ProtocolTeamCollisionRule::PushOwnTeam => "pushOwnTeam",
    }
    .to_string()
}

fn chat_formatting_name(color: ProtocolChatFormatting) -> String {
    match color {
        ProtocolChatFormatting::Black => "black",
        ProtocolChatFormatting::DarkBlue => "dark_blue",
        ProtocolChatFormatting::DarkGreen => "dark_green",
        ProtocolChatFormatting::DarkAqua => "dark_aqua",
        ProtocolChatFormatting::DarkRed => "dark_red",
        ProtocolChatFormatting::DarkPurple => "dark_purple",
        ProtocolChatFormatting::Gold => "gold",
        ProtocolChatFormatting::Gray => "gray",
        ProtocolChatFormatting::DarkGray => "dark_gray",
        ProtocolChatFormatting::Blue => "blue",
        ProtocolChatFormatting::Green => "green",
        ProtocolChatFormatting::Aqua => "aqua",
        ProtocolChatFormatting::Red => "red",
        ProtocolChatFormatting::LightPurple => "light_purple",
        ProtocolChatFormatting::Yellow => "yellow",
        ProtocolChatFormatting::White => "white",
        ProtocolChatFormatting::Obfuscated => "obfuscated",
        ProtocolChatFormatting::Bold => "bold",
        ProtocolChatFormatting::Strikethrough => "strikethrough",
        ProtocolChatFormatting::Underline => "underline",
        ProtocolChatFormatting::Italic => "italic",
        ProtocolChatFormatting::Reset => "reset",
    }
    .to_string()
}

fn scoreboard_team_parameters(
    parameters: ProtocolPlayerTeamParameters,
) -> ScoreboardTeamParameters {
    ScoreboardTeamParameters {
        display_name: parameters.display_name,
        options: i32::from(parameters.options),
        nametag_visibility: team_visibility_name(parameters.nametag_visibility),
        collision_rule: team_collision_rule_name(parameters.collision_rule),
        color: chat_formatting_name(parameters.color),
        player_prefix: parameters.player_prefix,
        player_suffix: parameters.player_suffix,
    }
}

#[cfg(test)]
mod tests;
