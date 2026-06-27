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
    assert_eq!(counters.set_objective_updates_applied, 3);
    assert_eq!(counters.set_objective_updates_ignored, 0);
    assert_eq!(counters.set_display_objective_packets, 1);
    assert_eq!(counters.set_display_objective_updates_applied, 1);
    assert_eq!(counters.set_display_objective_updates_ignored, 0);
    assert_eq!(counters.set_score_packets, 1);
    assert_eq!(counters.set_score_updates_applied, 1);
    assert_eq!(counters.set_score_updates_ignored, 0);
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
    assert_eq!(counters.set_objective_updates_applied, 2);
    assert_eq!(counters.set_objective_updates_ignored, 0);
    assert_eq!(counters.set_score_packets, 3);
    assert_eq!(counters.set_score_updates_applied, 2);
    assert_eq!(counters.set_score_updates_ignored, 1);
    assert_eq!(counters.reset_score_packets, 3);
    assert_eq!(counters.reset_score_updates_applied, 2);
    assert_eq!(counters.reset_score_updates_ignored, 1);
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

    assert!(!store.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::Sidebar,
        objective_name: Some(String::new()),
    }));
    assert_eq!(store.counters().set_display_objective_packets, 3);
    assert_eq!(store.counters().set_display_objective_updates_applied, 2);
    assert_eq!(store.counters().set_display_objective_updates_ignored, 1);
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
    assert_eq!(store.counters().set_player_team_updates_applied, 6);
    assert_eq!(store.counters().set_player_team_updates_ignored, 1);
}

#[test]
fn scoreboard_team_color_rgb_matches_vanilla_chat_formatting() {
    let mut store = WorldStore::new();

    assert!(store.apply_set_player_team(protocol_set_player_team(
        "green",
        PlayerTeamMethod::Add,
        Some(protocol_team_parameters(
            "Green",
            0,
            TeamVisibility::Always,
            TeamCollisionRule::Always,
            ChatFormatting::Green,
            "",
            "",
        )),
        &["Alex"],
    )));
    assert_eq!(
        store
            .scoreboard()
            .team_color_rgb_for_scoreboard_name("Alex"),
        Some(0x55ff55)
    );

    assert!(store.apply_set_player_team(protocol_set_player_team(
        "reset",
        PlayerTeamMethod::Add,
        Some(protocol_team_parameters(
            "Reset",
            0,
            TeamVisibility::Always,
            TeamCollisionRule::Always,
            ChatFormatting::Reset,
            "",
            "",
        )),
        &["Cow"],
    )));
    assert_eq!(
        store.scoreboard().team_color_rgb_for_scoreboard_name("Cow"),
        None
    );
    assert_eq!(
        store
            .scoreboard()
            .team_color_rgb_for_scoreboard_name("Missing"),
        None
    );
}

#[test]
fn scoreboard_ignored_updates_are_counted() {
    let mut store = WorldStore::new();

    assert!(!store.apply_set_objective(protocol_set_objective(
        "bad",
        SetObjectiveMethod::Add,
        None,
    )));
    assert!(!store.apply_set_objective(protocol_set_objective(
        "missing",
        SetObjectiveMethod::Change,
        Some(protocol_objective_parameters(
            "Missing",
            ObjectiveRenderType::Integer,
            None,
        )),
    )));
    assert!(!store.apply_set_score(SetScore {
        owner: "Alex".to_string(),
        objective_name: "missing".to_string(),
        score: 99,
        display: None,
        number_format: None,
    }));
    assert!(!store.apply_reset_score(ResetScore {
        owner: "Alex".to_string(),
        objective_name: Some("missing".to_string()),
    }));
    assert!(!store.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::Sidebar,
        objective_name: Some("missing".to_string()),
    }));
    assert!(!store.apply_set_player_team(protocol_set_player_team(
        "missing",
        PlayerTeamMethod::Join,
        None,
        &["Alex"],
    )));

    let counters = store.counters();
    assert_eq!(counters.set_objective_packets, 2);
    assert_eq!(counters.set_objective_updates_applied, 0);
    assert_eq!(counters.set_objective_updates_ignored, 2);
    assert_eq!(counters.set_score_packets, 1);
    assert_eq!(counters.set_score_updates_applied, 0);
    assert_eq!(counters.set_score_updates_ignored, 1);
    assert_eq!(counters.reset_score_packets, 1);
    assert_eq!(counters.reset_score_updates_applied, 0);
    assert_eq!(counters.reset_score_updates_ignored, 1);
    assert_eq!(counters.set_display_objective_packets, 1);
    assert_eq!(counters.set_display_objective_updates_applied, 0);
    assert_eq!(counters.set_display_objective_updates_ignored, 1);
    assert_eq!(counters.set_player_team_packets, 1);
    assert_eq!(counters.set_player_team_updates_applied, 0);
    assert_eq!(counters.set_player_team_updates_ignored, 1);
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
