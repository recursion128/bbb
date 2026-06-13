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

        match packet.method {
            ProtocolSetObjectiveMethod::Add => {
                let Some(parameters) = packet.parameters else {
                    return false;
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
                    return false;
                };
                let Some(objective) = self.scoreboard.objectives.get_mut(&packet.objective_name)
                else {
                    return false;
                };
                objective.display_name = parameters.display_name;
                objective.render_type = objective_render_type_name(parameters.render_type);
                objective.number_format = parameters.number_format;
                true
            }
        }
    }

    pub fn apply_set_score(&mut self, packet: ProtocolSetScore) -> bool {
        self.counters.set_score_packets += 1;
        if !self
            .scoreboard
            .objectives
            .contains_key(&packet.objective_name)
        {
            return false;
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
        true
    }

    pub fn apply_reset_score(&mut self, packet: ProtocolResetScore) -> bool {
        self.counters.reset_score_packets += 1;
        let Some(objective_name) = packet.objective_name else {
            return self.scoreboard.scores.remove(&packet.owner).is_some();
        };
        if !self.scoreboard.objectives.contains_key(&objective_name) {
            return false;
        }

        let Some(scores) = self.scoreboard.scores.get_mut(&packet.owner) else {
            return false;
        };
        let removed = scores.remove(&objective_name).is_some();
        if scores.is_empty() {
            self.scoreboard.scores.remove(&packet.owner);
        }
        removed
    }

    pub fn apply_set_display_objective(&mut self, packet: ProtocolSetDisplayObjective) -> bool {
        self.counters.set_display_objective_packets += 1;
        let slot = scoreboard_display_slot_name(packet.slot);
        let Some(objective_name) = packet.objective_name.filter(|name| !name.is_empty()) else {
            return self.scoreboard.display_slots.remove(&slot).is_some();
        };
        if !self.scoreboard.objectives.contains_key(&objective_name) {
            return self.scoreboard.display_slots.remove(&slot).is_some();
        }

        self.scoreboard.display_slots.insert(slot, objective_name);
        true
    }

    pub fn apply_set_player_team(&mut self, packet: ProtocolSetPlayerTeam) -> bool {
        self.counters.set_player_team_packets += 1;

        match packet.method {
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
                    return false;
                };
                let Some(parameters) = packet.parameters else {
                    return false;
                };
                team.parameters = Some(scoreboard_team_parameters(parameters));
                true
            }
            ProtocolPlayerTeamMethod::Join => {
                if !self.scoreboard.teams.contains_key(&packet.name) {
                    return false;
                }
                self.add_players_to_scoreboard_team(&packet.name, packet.players);
                true
            }
            ProtocolPlayerTeamMethod::Leave => {
                let Some(team) = self.scoreboard.teams.get_mut(&packet.name) else {
                    return false;
                };
                for player in packet.players {
                    team.players.remove(&player);
                }
                true
            }
        }
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
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        ChatFormatting, ObjectiveRenderType, PlayerTeamMethod, PlayerTeamParameters, ResetScore,
        ScoreboardDisplaySlot, SetDisplayObjective, SetObjective, SetObjectiveMethod,
        SetObjectiveParameters, SetPlayerTeam, SetScore, TeamCollisionRule, TeamVisibility,
    };

    #[test]
    fn scoreboard_objective_lifecycle_clears_display_and_scores() {
        let mut store = WorldStore::new();

        assert!(store.apply_set_objective(protocol_set_objective(
            "kills",
            SetObjectiveMethod::Add,
            Some(protocol_objective_parameters(
                "Kills",
                ObjectiveRenderType::Integer,
                Some(vec![1, 2, 3]),
            )),
        )));
        assert!(store.apply_set_display_objective(SetDisplayObjective {
            slot: ScoreboardDisplaySlot::Sidebar,
            objective_name: Some("kills".to_string()),
        }));
        assert!(store.apply_set_score(SetScore {
            owner: "Alice".to_string(),
            objective_name: "kills".to_string(),
            score: 4,
            display: Some("Alice".to_string()),
            number_format: Some(vec![9]),
        }));
        assert!(store.apply_set_objective(protocol_set_objective(
            "kills",
            SetObjectiveMethod::Change,
            Some(protocol_objective_parameters(
                "Final Kills",
                ObjectiveRenderType::Hearts,
                None,
            )),
        )));

        let objective = store.scoreboard().objectives.get("kills").unwrap();
        assert_eq!(objective.display_name, "Final Kills");
        assert_eq!(objective.render_type, "hearts");
        assert_eq!(objective.number_format, None);
        assert_eq!(
            store
                .scoreboard()
                .display_slots
                .get("sidebar")
                .map(String::as_str),
            Some("kills")
        );
        let score = &store.scoreboard().scores["Alice"]["kills"];
        assert_eq!(score.value, 4);
        assert_eq!(score.display.as_deref(), Some("Alice"));
        assert_eq!(score.number_format, Some(vec![9]));

        assert!(store.apply_set_objective(protocol_set_objective(
            "kills",
            SetObjectiveMethod::Remove,
            None,
        )));
        assert!(store.scoreboard().objectives.is_empty());
        assert!(store.scoreboard().display_slots.is_empty());
        assert!(store.scoreboard().scores.is_empty());

        let counters = store.counters();
        assert_eq!(counters.set_objective_packets, 3);
        assert_eq!(counters.set_display_objective_packets, 1);
        assert_eq!(counters.set_score_packets, 1);
    }

    #[test]
    fn scoreboard_sets_and_resets_scores() {
        let mut store = WorldStore::new();

        assert!(!store.apply_set_score(SetScore {
            owner: "Alex".to_string(),
            objective_name: "missing".to_string(),
            score: 99,
            display: None,
            number_format: None,
        }));
        for objective in ["kills", "deaths"] {
            assert!(store.apply_set_objective(protocol_set_objective(
                objective,
                SetObjectiveMethod::Add,
                Some(protocol_objective_parameters(
                    objective,
                    ObjectiveRenderType::Integer,
                    None,
                )),
            )));
        }
        assert!(store.apply_set_score(SetScore {
            owner: "Alex".to_string(),
            objective_name: "kills".to_string(),
            score: 7,
            display: None,
            number_format: None,
        }));
        assert!(store.apply_set_score(SetScore {
            owner: "Alex".to_string(),
            objective_name: "deaths".to_string(),
            score: 2,
            display: None,
            number_format: None,
        }));

        assert!(store.apply_reset_score(ResetScore {
            owner: "Alex".to_string(),
            objective_name: Some("kills".to_string()),
        }));
        assert!(!store.scoreboard().scores["Alex"].contains_key("kills"));
        assert_eq!(store.scoreboard().scores["Alex"]["deaths"].value, 2);

        assert!(!store.apply_reset_score(ResetScore {
            owner: "Alex".to_string(),
            objective_name: Some("unknown".to_string()),
        }));
        assert!(store.apply_reset_score(ResetScore {
            owner: "Alex".to_string(),
            objective_name: None,
        }));
        assert!(!store.scoreboard().scores.contains_key("Alex"));

        let counters = store.counters();
        assert_eq!(counters.set_objective_packets, 2);
        assert_eq!(counters.set_score_packets, 3);
        assert_eq!(counters.reset_score_packets, 3);
    }

    #[test]
    fn scoreboard_display_objective_can_be_cleared_by_empty_name() {
        let mut store = WorldStore::new();

        assert!(store.apply_set_objective(protocol_set_objective(
            "health",
            SetObjectiveMethod::Add,
            Some(protocol_objective_parameters(
                "Health",
                ObjectiveRenderType::Hearts,
                None,
            )),
        )));
        assert!(store.apply_set_display_objective(SetDisplayObjective {
            slot: ScoreboardDisplaySlot::Sidebar,
            objective_name: Some("health".to_string()),
        }));
        assert_eq!(
            store
                .scoreboard()
                .display_slots
                .get("sidebar")
                .map(String::as_str),
            Some("health")
        );

        assert!(store.apply_set_display_objective(SetDisplayObjective {
            slot: ScoreboardDisplaySlot::Sidebar,
            objective_name: Some(String::new()),
        }));
        assert!(store.scoreboard().display_slots.is_empty());
        assert_eq!(store.counters().set_display_objective_packets, 2);
    }

    #[test]
    fn scoreboard_teams_add_change_join_leave_and_remove() {
        let mut store = WorldStore::new();

        assert!(!store.apply_set_player_team(protocol_set_player_team(
            "missing",
            PlayerTeamMethod::Join,
            None,
            &["Alice"],
        )));
        assert!(store.scoreboard().teams.is_empty());

        assert!(store.apply_set_player_team(protocol_set_player_team(
            "red",
            PlayerTeamMethod::Add,
            Some(protocol_team_parameters(
                "red",
                3,
                TeamVisibility::Always,
                TeamCollisionRule::PushOtherTeams,
                ChatFormatting::Red,
                "[R] ",
                "",
            )),
            &["Alice", "Bob"],
        )));
        assert_eq!(
            team_players(&store.scoreboard().teams["red"]),
            vec!["Alice", "Bob"]
        );

        assert!(store.apply_set_player_team(protocol_set_player_team(
            "red",
            PlayerTeamMethod::Change,
            Some(protocol_team_parameters(
                "Red Team",
                1,
                TeamVisibility::HideForOtherTeams,
                TeamCollisionRule::Never,
                ChatFormatting::DarkRed,
                "[RED] ",
                "!",
            )),
            &[],
        )));
        let parameters = store.scoreboard().teams["red"].parameters.as_ref().unwrap();
        assert_eq!(parameters.display_name, "Red Team");
        assert_eq!(parameters.options, 1);
        assert_eq!(parameters.nametag_visibility, "hideForOtherTeams");
        assert_eq!(parameters.collision_rule, "never");
        assert_eq!(parameters.color, "dark_red");
        assert_eq!(parameters.player_suffix, "!");

        assert!(store.apply_set_player_team(protocol_set_player_team(
            "blue",
            PlayerTeamMethod::Add,
            Some(protocol_team_parameters(
                "Blue",
                0,
                TeamVisibility::Always,
                TeamCollisionRule::Always,
                ChatFormatting::Blue,
                "",
                "",
            )),
            &["Cara"],
        )));
        assert!(store.apply_set_player_team(protocol_set_player_team(
            "red",
            PlayerTeamMethod::Join,
            None,
            &["Cara"],
        )));
        assert!(!store.scoreboard().teams["blue"].players.contains("Cara"));
        assert_eq!(
            team_players(&store.scoreboard().teams["red"]),
            vec!["Alice", "Bob", "Cara"]
        );

        assert!(store.apply_set_player_team(protocol_set_player_team(
            "red",
            PlayerTeamMethod::Leave,
            None,
            &["Bob", "Nobody"],
        )));
        assert_eq!(
            team_players(&store.scoreboard().teams["red"]),
            vec!["Alice", "Cara"]
        );

        assert!(store.apply_set_player_team(protocol_set_player_team(
            "red",
            PlayerTeamMethod::Remove,
            None,
            &[],
        )));
        assert!(!store.scoreboard().teams.contains_key("red"));
        assert!(store.scoreboard().teams.contains_key("blue"));
        assert_eq!(store.counters().set_player_team_packets, 7);
    }

    fn team_players(team: &ScoreboardTeam) -> Vec<&str> {
        team.players.iter().map(String::as_str).collect()
    }

    fn protocol_set_objective(
        objective_name: &str,
        method: SetObjectiveMethod,
        parameters: Option<SetObjectiveParameters>,
    ) -> SetObjective {
        SetObjective {
            objective_name: objective_name.to_string(),
            method,
            parameters,
        }
    }

    fn protocol_objective_parameters(
        display_name: &str,
        render_type: ObjectiveRenderType,
        number_format: Option<Vec<u8>>,
    ) -> SetObjectiveParameters {
        SetObjectiveParameters {
            display_name: display_name.to_string(),
            render_type,
            number_format,
        }
    }

    fn protocol_set_player_team(
        name: &str,
        method: PlayerTeamMethod,
        parameters: Option<PlayerTeamParameters>,
        players: &[&str],
    ) -> SetPlayerTeam {
        SetPlayerTeam {
            name: name.to_string(),
            method,
            parameters,
            players: players.iter().map(|player| player.to_string()).collect(),
        }
    }

    fn protocol_team_parameters(
        display_name: &str,
        options: u8,
        nametag_visibility: TeamVisibility,
        collision_rule: TeamCollisionRule,
        color: ChatFormatting,
        player_prefix: &str,
        player_suffix: &str,
    ) -> PlayerTeamParameters {
        PlayerTeamParameters {
            display_name: display_name.to_string(),
            options,
            nametag_visibility,
            collision_rule,
            color,
            player_prefix: player_prefix.to_string(),
            player_suffix: player_suffix.to_string(),
        }
    }
}
