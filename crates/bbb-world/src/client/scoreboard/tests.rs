use super::*;
use bbb_protocol::packets::{
    ChatFormatting, ObjectiveRenderType, PlayerTeamMethod, PlayerTeamOperation,
    PlayerTeamParameters, ResetScore, ScoreboardDisplaySlot, SetDisplayObjective, SetObjective,
    SetObjectiveMethod, SetObjectiveOperation, SetObjectiveParameters, SetPlayerTeam, SetScore,
    TeamCollisionRule, TeamVisibility,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

#[test]
fn scoreboard_objective_lifecycle_clears_display_and_scores() {
    let mut store = WorldStore::new();

    assert!(store.apply_set_objective(protocol_set_objective(
        "kills",
        SetObjectiveMethod::Add,
        Some(protocol_objective_parameters(
            "Kills",
            ObjectiveRenderType::Integer,
            Some(NumberFormatSummary::Styled(ComponentStyle {
                color: Some(0xFF_55_55),
                ..ComponentStyle::default()
            })),
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
        display: Some(RichTextComponent::plain("Alice")),
        number_format: Some(NumberFormatSummary::Blank),
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
    assert_eq!(score.number_format, Some(NumberFormatSummary::Blank));

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
fn scoreboard_preserves_rich_components_and_formats_below_name_scores() {
    let mut store = WorldStore::new();
    let objective_name = styled_component("Health", "green", true);
    assert!(store.apply_set_objective(protocol_set_objective(
        "health",
        SetObjectiveMethod::Add,
        Some(SetObjectiveParameters {
            display_name: objective_name,
            render_type: ObjectiveRenderType::Integer,
            number_format: Some(NumberFormatSummary::Styled(ComponentStyle {
                color: Some(0xFF_55_55),
                ..ComponentStyle::default()
            })),
        }),
    )));
    assert!(store.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::BelowName,
        objective_name: Some("health".to_string()),
    }));

    let missing_score = store.scoreboard().below_name_display_runs("Alice").unwrap();
    assert_eq!(
        missing_score
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "0 Health"
    );
    assert_eq!(missing_score[0].style.color, Some(0xFF_55_55));
    assert_eq!(missing_score[2].style.color, Some(0x55_FF_55));
    assert_eq!(missing_score[2].style.bold, Some(true));
    assert!(matches!(
        store.scoreboard().objectives["health"]
            .display_name_component
            .as_ref(),
        Some(NbtSummaryValue::Compound(_))
    ));

    assert!(store.apply_set_score(SetScore {
        owner: "Alice".to_string(),
        objective_name: "health".to_string(),
        score: 19,
        display: Some(styled_component("Alice", "yellow", false)),
        number_format: Some(NumberFormatSummary::Fixed(styled_component(
            "MAX", "aqua", false,
        ))),
    }));
    let score = &store.scoreboard().scores["Alice"]["health"];
    assert_eq!(score.display.as_deref(), Some("Alice"));
    assert_eq!(
        score.display_styled.as_ref().unwrap()[0].style.color,
        Some(0xFF_FF_55)
    );
    assert!(matches!(
        score.display_component.as_ref(),
        Some(NbtSummaryValue::Compound(_))
    ));
    let fixed = store.scoreboard().below_name_display_runs("Alice").unwrap();
    assert_eq!(
        fixed
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "MAX Health"
    );
    assert_eq!(fixed[0].style.color, Some(0x55_FF_FF));

    assert!(store.apply_set_score(SetScore {
        owner: "Alice".to_string(),
        objective_name: "health".to_string(),
        score: 19,
        display: None,
        number_format: Some(NumberFormatSummary::Blank),
    }));
    let blank = store.scoreboard().below_name_display_runs("Alice").unwrap();
    assert_eq!(
        blank
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        " Health"
    );
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
fn configuration_reentry_rebuilds_listener_scoreboard_and_preserves_packet_counters() {
    let mut store = WorldStore::new();

    assert!(store.apply_set_objective(protocol_set_objective(
        "old_session",
        SetObjectiveMethod::Add,
        Some(protocol_objective_parameters(
            "Old session",
            ObjectiveRenderType::Integer,
            None,
        )),
    )));

    store.clear_client_level();

    assert_eq!(store.scoreboard(), &ScoreboardState::default());
    assert_eq!(store.counters().set_objective_packets, 1);
    assert_eq!(store.counters().set_objective_updates_applied, 1);
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
    assert!(store
        .scoreboard()
        .team_for_scoreboard_name("Alice")
        .is_none());
    assert!(store
        .scoreboard()
        .team_for_scoreboard_name("Cara")
        .is_none());
    assert_eq!(store.counters().set_player_team_packets, 7);
    assert_eq!(store.counters().set_player_team_updates_applied, 6);
    assert_eq!(store.counters().set_player_team_updates_ignored, 1);
}

#[test]
fn scoreboard_preserves_team_component_styles_and_empty_components() {
    let mut store = WorldStore::new();
    assert!(store.apply_set_player_team(protocol_set_player_team(
        "gold",
        PlayerTeamMethod::Add,
        Some(PlayerTeamParameters {
            display_name: styled_component("Gold Team", "gold", true),
            options: 3,
            nametag_visibility: TeamVisibility::Always,
            collision_rule: TeamCollisionRule::Always,
            color: ChatFormatting::Gold,
            player_prefix: styled_component("[G] ", "yellow", false),
            player_suffix: RichTextComponent::plain(""),
        }),
        &["Alice"],
    )));

    let parameters = store.scoreboard().teams["gold"]
        .parameters
        .as_ref()
        .unwrap();
    assert_eq!(parameters.display_name, "Gold Team");
    assert_eq!(parameters.display_name_styled[0].style.bold, Some(true));
    assert_eq!(parameters.player_prefix, "[G] ");
    assert_eq!(
        parameters.player_prefix_styled[0].style.color,
        Some(0xFF_FF_55)
    );
    assert_eq!(parameters.player_suffix, "");
    assert!(parameters.player_suffix_styled.is_empty());
    assert!(matches!(
        parameters.player_prefix_component.as_ref(),
        Some(NbtSummaryValue::Compound(_))
    ));
    assert_eq!(
        parameters.player_suffix_component.as_ref(),
        Some(&NbtSummaryValue::String(String::new()))
    );
}

#[test]
fn scoreboard_collision_rules_match_entity_selector_pushable_by() {
    let mut store = WorldStore::new();
    assert!(store
        .scoreboard()
        .entity_can_push_scoreboard_target("boat", Some("player")));

    assert!(store.apply_set_player_team(protocol_set_player_team(
        "boat-team",
        PlayerTeamMethod::Add,
        Some(protocol_team_parameters(
            "boat-team",
            0,
            TeamVisibility::Always,
            TeamCollisionRule::PushOtherTeams,
            ChatFormatting::White,
            "",
            "",
        )),
        &["boat", "ally"],
    )));
    assert!(!store
        .scoreboard()
        .entity_can_push_scoreboard_target("boat", Some("player")));
    assert!(store
        .scoreboard()
        .entity_can_push_scoreboard_target("boat", Some("ally")));

    assert!(store.apply_set_player_team(protocol_set_player_team(
        "boat-team",
        PlayerTeamMethod::Change,
        Some(protocol_team_parameters(
            "boat-team",
            0,
            TeamVisibility::Always,
            TeamCollisionRule::PushOwnTeam,
            ChatFormatting::White,
            "",
            "",
        )),
        &[],
    )));
    assert!(!store
        .scoreboard()
        .entity_can_push_scoreboard_target("boat", Some("ally")));
    assert!(store
        .scoreboard()
        .entity_can_push_scoreboard_target("boat", Some("player")));

    assert!(store.apply_set_player_team(protocol_set_player_team(
        "boat-team",
        PlayerTeamMethod::Change,
        Some(protocol_team_parameters(
            "boat-team",
            0,
            TeamVisibility::Always,
            TeamCollisionRule::Never,
            ChatFormatting::White,
            "",
            "",
        )),
        &[],
    )));
    assert!(!store
        .scoreboard()
        .entity_can_push_scoreboard_target("boat", Some("player")));
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
fn scoreboard_same_team_friendly_invisibles_uses_team_options_bit() {
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
        &["Viewer", "Target"],
    )));
    assert!(!store
        .scoreboard()
        .same_team_can_see_friendly_invisibles("Viewer", "Target"));

    assert!(store.apply_set_player_team(protocol_set_player_team(
        "green",
        PlayerTeamMethod::Change,
        Some(protocol_team_parameters(
            "Green",
            2,
            TeamVisibility::Always,
            TeamCollisionRule::Always,
            ChatFormatting::Green,
            "",
            "",
        )),
        &[],
    )));
    assert!(store
        .scoreboard()
        .same_team_can_see_friendly_invisibles("Viewer", "Target"));
    assert!(!store
        .scoreboard()
        .same_team_can_see_friendly_invisibles("Viewer", "Missing"));
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

#[test]
fn scoreboard_unknown_methods_and_duplicate_objective_adds_are_noops() {
    let mut store = WorldStore::new();
    assert!(store.apply_set_objective(protocol_set_objective(
        "kills",
        SetObjectiveMethod::Add,
        Some(protocol_objective_parameters(
            "Kills",
            ObjectiveRenderType::Integer,
            None,
        )),
    )));
    assert!(!store.apply_set_objective(protocol_set_objective(
        "kills",
        SetObjectiveMethod::Add,
        Some(protocol_objective_parameters(
            "Replacement",
            ObjectiveRenderType::Hearts,
            Some(NumberFormatSummary::Blank),
        )),
    )));
    assert!(!store.apply_set_objective(SetObjective {
        objective_name: "kills".to_string(),
        method: SetObjectiveOperation::from_raw(3),
        parameters: None,
    }));
    assert_eq!(store.scoreboard().objectives["kills"].display_name, "Kills");
    assert_eq!(
        store.scoreboard().objectives["kills"].render_type,
        "integer"
    );

    assert!(store.apply_set_player_team(protocol_set_player_team(
        "red",
        PlayerTeamMethod::Add,
        Some(protocol_team_parameters(
            "Red",
            0,
            TeamVisibility::Always,
            TeamCollisionRule::Always,
            ChatFormatting::Red,
            "[",
            "]",
        )),
        &["Alice"],
    )));
    assert!(!store.apply_set_player_team(SetPlayerTeam {
        name: "red".to_string(),
        method: PlayerTeamOperation::from_raw(-1),
        parameters: None,
        players: Vec::new(),
    }));
    assert_eq!(team_players(&store.scoreboard().teams["red"]), ["Alice"]);

    let counters = store.counters();
    assert_eq!(counters.set_objective_packets, 3);
    assert_eq!(counters.set_objective_updates_applied, 1);
    assert_eq!(counters.set_objective_updates_ignored, 2);
    assert_eq!(counters.set_player_team_packets, 2);
    assert_eq!(counters.set_player_team_updates_applied, 1);
    assert_eq!(counters.set_player_team_updates_ignored, 1);
}

#[test]
fn scoreboard_sidebar_prefers_team_slot_and_filters_sorts_and_limits_scores() {
    let mut store = WorldStore::new();
    for objective in ["generic", "red"] {
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
    assert!(store.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::Sidebar,
        objective_name: Some("generic".to_string()),
    }));
    assert!(store.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::TeamRed,
        objective_name: Some("red".to_string()),
    }));
    assert!(store.apply_set_player_team(protocol_set_player_team(
        "viewers",
        PlayerTeamMethod::Add,
        Some(protocol_team_parameters(
            "Viewers",
            0,
            TeamVisibility::Always,
            TeamCollisionRule::Always,
            ChatFormatting::Red,
            "<",
            ">",
        )),
        &["Local", "Player03"],
    )));
    assert!(store.apply_set_score(SetScore {
        owner: "#hidden".to_string(),
        objective_name: "red".to_string(),
        score: 1000,
        display: None,
        number_format: None,
    }));
    for index in 0..17 {
        let owner = format!("Player{index:02}");
        assert!(store.apply_set_score(SetScore {
            owner,
            objective_name: "red".to_string(),
            score: index / 2,
            display: None,
            number_format: None,
        }));
    }

    let sidebar = store
        .scoreboard()
        .sidebar_state(Some("Local"))
        .expect("team sidebar");
    assert_eq!(sidebar.objective_name, "red");
    assert_eq!(sidebar.entries.len(), 15);
    assert!(sidebar.entries.iter().all(|entry| entry.owner != "#hidden"));
    assert_eq!(sidebar.entries[0].owner, "Player16");
    assert_eq!(sidebar.entries[1].owner, "Player14");
    assert_eq!(sidebar.entries[2].owner, "Player15");
    assert_eq!(sidebar.entries.last().unwrap().owner, "Player03");
    let formatted = sidebar.entries.last().unwrap();
    assert_eq!(formatted.owner, "Player03");
    assert_eq!(scoreboard_text(&formatted.name), "<Player03>");
    assert_eq!(formatted.name.inherited_style.color, Some(0xFF5555));
    assert_eq!(scoreboard_text(&formatted.score), "1");
    assert_eq!(
        scoreboard_first_run(&formatted.score).style.color,
        Some(0xFF5555)
    );

    let generic = store
        .scoreboard()
        .sidebar_state(None)
        .expect("generic sidebar");
    assert_eq!(generic.objective_name, "generic");
    assert!(generic.entries.is_empty());
}

#[test]
fn scoreboard_player_list_formats_missing_override_and_heart_scores() {
    let mut store = WorldStore::new();
    assert!(store.apply_set_objective(protocol_set_objective(
        "list",
        SetObjectiveMethod::Add,
        Some(protocol_objective_parameters(
            "List",
            ObjectiveRenderType::Integer,
            None,
        )),
    )));
    assert!(store.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::List,
        objective_name: Some("list".to_string()),
    }));
    let missing = store
        .scoreboard()
        .player_list_score("Missing")
        .expect("list objective");
    let ScoreboardPlayerListScoreState::Integer(missing) = missing else {
        panic!("integer score");
    };
    assert_eq!(scoreboard_text(&missing), "0");
    assert_eq!(scoreboard_first_run(&missing).style.color, Some(0xFFFF55));

    assert!(store.apply_set_score(SetScore {
        owner: "Blank".to_string(),
        objective_name: "list".to_string(),
        score: 7,
        display: None,
        number_format: Some(NumberFormatSummary::Blank),
    }));
    let ScoreboardPlayerListScoreState::Integer(blank) = store
        .scoreboard()
        .player_list_score("Blank")
        .expect("list objective")
    else {
        panic!("integer score");
    };
    assert!(blank.segments.is_empty());

    assert!(store.apply_set_objective(protocol_set_objective(
        "list",
        SetObjectiveMethod::Change,
        Some(protocol_objective_parameters(
            "Health",
            ObjectiveRenderType::Hearts,
            None,
        )),
    )));
    assert_eq!(
        store.scoreboard().player_list_score("Blank"),
        Some(ScoreboardPlayerListScoreState::Hearts(7))
    );
    assert_eq!(
        store.scoreboard().player_list_score("Missing"),
        Some(ScoreboardPlayerListScoreState::Hearts(0))
    );
}

fn team_players(team: &ScoreboardTeam) -> Vec<&str> {
    team.players.iter().map(String::as_str).collect()
}

fn scoreboard_text(text: &ScoreboardTextState) -> String {
    text.segments
        .iter()
        .flat_map(|segment| match segment {
            ScoreboardTextSegmentState::Rich(component) => component.styled().to_vec(),
            ScoreboardTextSegmentState::Styled(runs) => runs.clone(),
        })
        .map(|run| run.text)
        .collect()
}

fn scoreboard_first_run(text: &ScoreboardTextState) -> StyledTextRun {
    match &text.segments[0] {
        ScoreboardTextSegmentState::Rich(component) => component.styled()[0].clone(),
        ScoreboardTextSegmentState::Styled(runs) => runs[0].clone(),
    }
}

fn protocol_set_objective(
    objective_name: &str,
    method: SetObjectiveMethod,
    parameters: Option<SetObjectiveParameters>,
) -> SetObjective {
    SetObjective {
        objective_name: objective_name.to_string(),
        method: method.into(),
        parameters,
    }
}

fn protocol_objective_parameters(
    display_name: &str,
    render_type: ObjectiveRenderType,
    number_format: Option<NumberFormatSummary>,
) -> SetObjectiveParameters {
    SetObjectiveParameters {
        display_name: RichTextComponent::plain(display_name),
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
        method: method.into(),
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
        display_name: RichTextComponent::plain(display_name),
        options,
        nametag_visibility,
        collision_rule,
        color,
        player_prefix: RichTextComponent::plain(player_prefix),
        player_suffix: RichTextComponent::plain(player_suffix),
    }
}

fn styled_component(text: &str, color: &str, bold: bool) -> RichTextComponent {
    RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".to_string(),
            value: NbtSummaryValue::String(text.to_string()),
        },
        NbtSummaryEntry {
            name: "color".to_string(),
            value: NbtSummaryValue::String(color.to_string()),
        },
        NbtSummaryEntry {
            name: "bold".to_string(),
            value: NbtSummaryValue::Byte(i8::from(bold)),
        },
    ]))
    .unwrap()
}
