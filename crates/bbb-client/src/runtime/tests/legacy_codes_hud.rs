//! Legacy `§` consumption for HUD component-run projections.

use super::*;
use bbb_protocol::packets::{
    ChatFormatting, NumberFormatSummary, ObjectiveRenderType, PlayerInfoAction, PlayerInfoEntry,
    PlayerInfoUpdate, PlayerTeamMethod, PlayerTeamParameters, ScoreboardDisplaySlot,
    SetDisplayObjective, SetObjective, SetObjectiveMethod, SetObjectiveParameters, SetPlayerTeam,
    SetScore, TabList, TeamCollisionRule, TeamVisibility,
};
use bbb_protocol::RichTextComponent;

fn component_with_text(text: &str, bold: bool) -> RichTextComponent {
    let mut entries = vec![NbtSummaryEntry {
        name: "text".into(),
        value: NbtSummaryValue::string(text),
    }];
    if bold {
        entries.push(NbtSummaryEntry {
            name: "bold".into(),
            value: NbtSummaryValue::Byte(1),
        });
    }
    RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(entries)).unwrap()
}

fn world_with_sidebar_and_team() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 1,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    });
    assert!(world.apply_set_objective(SetObjective {
        objective_name: "kills".into(),
        method: SetObjectiveMethod::Add.into(),
        parameters: Some(SetObjectiveParameters {
            // Bold K, `§c` clears bold, `§r` returns to the inherited bold.
            display_name: component_with_text("K§cills§r!", true),
            render_type: ObjectiveRenderType::Integer,
            number_format: None,
        }),
    }));
    assert!(world.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::Sidebar,
        objective_name: Some("kills".into()),
    }));
    assert!(world.apply_set_player_team(SetPlayerTeam {
        name: "red".into(),
        method: PlayerTeamMethod::Add.into(),
        parameters: Some(PlayerTeamParameters {
            display_name: RichTextComponent::plain("Red"),
            options: 0,
            nametag_visibility: TeamVisibility::Always,
            collision_rule: TeamCollisionRule::Always,
            color: ChatFormatting::Red,
            player_prefix: component_with_text("§l<", false),
            player_suffix: RichTextComponent::plain(">"),
        }),
        players: vec!["Alice".into()],
    }));
    assert!(world.apply_set_score(SetScore {
        owner: "Alice".into(),
        objective_name: "kills".into(),
        score: 9,
        display: Some(component_with_text("A§cce§r!", true)),
        number_format: Some(NumberFormatSummary::Fixed(component_with_text(
            "N§cX§r!",
            true,
        ))),
    }));
    world.apply_tab_list(TabList {
        header: component_with_text("H§aeader§r.", false),
        footer: RichTextComponent::plain("Online"),
    });
    world
}

fn tab_list_input(world: &mut WorldStore) -> ClientInputState {
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id:
            bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
        param: 0.0,
    });
    world.complete_client_level_load(false);
    let mut input = ClientInputState::new(true);
    crate::input::handle_key_input(
        &mut input,
        &mut NetCounters::default(),
        world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    input
}

#[test]
fn scoreboard_sidebar_runs_consume_legacy_codes_with_the_inherited_style_as_reset() {
    let mut world = world_with_sidebar_and_team();
    let input = tab_list_input(&mut world);

    let sidebar = hud_scoreboard_sidebar_from_world(&input, &world, None)
        .expect("scoreboard sidebar projection");
    // `Gui.displayScoreboardSidebar` measures and draws Components
    // so the codes shape the runs the right-edge layout
    // and the glyph pass both read.
    assert_eq!(
        sidebar
            .title_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["K", "ills", "!"]
    );
    assert!(sidebar.title_runs[0].style.bold);
    assert_eq!(sidebar.title_runs[0].color, None);
    // A colour code clears the five decorations ...
    assert_eq!(sidebar.title_runs[1].color, Some(0xFF_55_55));
    assert!(!sidebar.title_runs[1].style.bold);
    // ... and `§r` returns to that part's inherited style, not to the default.
    assert!(sidebar.title_runs[2].style.bold);

    // The entry name is the team prefix, the score's display Component and the
    // suffix as three separate parts, each carrying the team red; the prefix
    // and the display root are not joined even though they share a style.
    assert_eq!(
        sidebar.entries[0]
            .name_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["<", "A", "ce", "!", ">"]
    );
    assert!(sidebar.entries[0].name_runs[0].style.bold);
    assert_eq!(sidebar.entries[0].name_runs[0].color, Some(0xFF5555));
    assert!(sidebar.entries[0].name_runs[1].style.bold);
    assert!(!sidebar.entries[0].name_runs[2].style.bold);
    assert_eq!(sidebar.entries[0].name_runs[2].color, Some(0xFF5555));
    assert!(
        sidebar.entries[0].name_runs[3].style.bold,
        "§r must restore the display component's own bold"
    );
    assert!(!sidebar.entries[0].name_runs[4].style.bold);

    assert_eq!(
        sidebar.entries[0]
            .score_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["N", "X", "!"]
    );
    assert!(sidebar.entries[0].score_runs[0].style.bold);
    assert!(!sidebar.entries[0].score_runs[1].style.bold);
    assert_eq!(sidebar.entries[0].score_runs[1].color, Some(0xFF_55_55));
    assert!(sidebar.entries[0].score_runs[2].style.bold);
}

#[test]
fn player_list_runs_consume_legacy_codes_for_header_and_display_name() {
    let mut world = world_with_sidebar_and_team();
    world.apply_player_info_update(PlayerInfoUpdate {
        actions: vec![
            PlayerInfoAction::AddPlayer,
            PlayerInfoAction::UpdateListed,
            PlayerInfoAction::UpdateDisplayName,
        ],
        entries: vec![PlayerInfoEntry {
            profile_id: uuid::Uuid::from_u128(3),
            profile: Some(bbb_protocol::packets::GameProfile {
                uuid: uuid::Uuid::from_u128(3),
                name: "Alice".into(),
                properties: Vec::new(),
            }),
            listed: true,
            latency: 120,
            game_mode: bbb_protocol::packets::GameType::Survival,
            display_name: Some(component_with_text("A§llice§r!", false)),
            show_hat: false,
            list_order: 0,
            chat_session: None,
        }],
    });
    let input = tab_list_input(&mut world);

    let list = hud_player_list_from_world(&input, &world, None, 0, false).expect("Tab list");
    // `PlayerTabOverlay` draws header and display name as Components
    let header = list.header.as_deref().expect("styled header");
    assert_eq!(
        header
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["H", "eader", "."]
    );
    assert_eq!(header[1].color, Some(0x55_FF_55));
    assert_eq!(header[2].color, None);

    let display = list
        .entries
        .iter()
        .find(|entry| entry.profile_id == 3)
        .expect("Alice entry");
    assert_eq!(
        display
            .name_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["A", "lice", "!"]
    );
    assert!(!display.name_runs[0].style.bold);
    assert_eq!(display.name_runs[0].color, None);
    assert!(display.name_runs[1].style.bold);
    assert_eq!(display.name_runs[1].color, None);
    // `§r` restores the display Component's own style, so `!` is no longer bold
    // even though the preceding run was.
    assert!(!display.name_runs[2].style.bold);
    assert_eq!(display.name_runs[2].color, None);
}

#[test]
fn container_label_and_toast_component_runs_consume_legacy_codes() {
    use bbb_protocol::NbtSummaryValue;
    use bbb_protocol::RichTextComponent;

    let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::string("C§lhest§r!"),
        },
    ]))
    .unwrap();
    // `hud_styled_runs_from_rich_text` is the projection behind the container
    // title, the advancement toast
    // title and the test-instance status lines.
    let runs = hud_styled_runs_from_rich_text(&component);
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
        ["C", "hest", "!"]
    );
    assert!(!runs[0].style.bold);
    assert!(runs[1].style.bold);
    assert!(!runs[2].style.bold);

    let mut labels = Vec::new();
    push_component_container_label(&mut labels, component.styled().to_vec(), 5, 6);
    assert_eq!(labels[0].runs, runs);
    assert_eq!(labels[0].text, "C§lhest§r!");
    assert_eq!((labels[0].x, labels[0].y), (5, 6));
    assert_eq!(labels[0].width, CONTAINER_LABEL_WIDTH);
}
