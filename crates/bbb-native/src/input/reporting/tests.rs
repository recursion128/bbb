use bbb_protocol::packets::{
    GameProfile, GameType, PlayerInfoAction, PlayerInfoEntry, PlayerInfoUpdate,
};
use bbb_world::{
    PlayerReportDraft, PlayerReportEnvironment, PlayerReportReason, PlayerReportSubmissionStatus,
};

use super::*;

fn reporting_world() -> (WorldStore, Uuid) {
    let local = Uuid::from_u128(1);
    let remote = Uuid::from_u128(2);
    let mut world = WorldStore::new();
    world.set_local_player_profile_id(local);
    world.set_player_report_environment(Some(PlayerReportEnvironment::third_party(
        "26.1",
        "en-US",
        "example.org:25565",
    )));
    world.apply_player_info_update(PlayerInfoUpdate {
        actions: vec![PlayerInfoAction::AddPlayer],
        entries: vec![
            player_info_entry(local, "Local"),
            player_info_entry(remote, "Remote"),
        ],
    });
    (world, remote)
}

fn player_info_entry(profile_id: Uuid, name: &str) -> PlayerInfoEntry {
    PlayerInfoEntry {
        profile_id,
        profile: Some(GameProfile {
            uuid: profile_id,
            name: name.to_string(),
            properties: Vec::new(),
        }),
        listed: true,
        latency: 0,
        game_mode: GameType::default(),
        display_name: None,
        show_hat: true,
        list_order: 0,
        chat_session: None,
    }
}

fn open_reporting(input: &mut ClientInputState, world: &mut WorldStore, remote: Uuid) {
    assert!(input.open_social_interactions_screen(world, false));
    assert!(input.open_player_reporting_screen(world, remote));
}

fn click(input: &mut ClientInputState, world: &mut WorldStore, focus: HudPlayerReportingFocus) {
    let generation = input
        .player_reporting_runtime()
        .expect("reporting runtime")
        .generation();
    assert!(input.handle_player_reporting_mouse_input(
        world,
        MouseButton::Left,
        ElementState::Pressed,
        Some(HudPlayerReportingHitTarget { generation, focus }),
    ));
}

#[test]
fn player_reporting_chooser_editor_comment_limit_and_draft_closeout_are_generation_safe() {
    let (mut world, remote) = reporting_world();
    let mut input = ClientInputState::new(true);
    open_reporting(&mut input, &mut world, remote);
    let generation = input.player_reporting_runtime().unwrap().generation();
    assert!(matches!(
        input.player_reporting_runtime().unwrap().screen(),
        PlayerReportingRuntimeScreen::TypeChooser
    ));
    assert!(world.social_interactions_screen().is_none());

    assert!(input.handle_player_reporting_mouse_input(
        &mut world,
        MouseButton::Left,
        ElementState::Pressed,
        Some(HudPlayerReportingHitTarget {
            generation: generation + 1,
            focus: HudPlayerReportingFocus::ReportType(2),
        }),
    ));
    assert!(world.player_reporting().draft.is_none());

    click(
        &mut input,
        &mut world,
        HudPlayerReportingFocus::ReportType(2),
    );
    let PlayerReportDraft::Username { common, .. } =
        world.player_reporting().draft.as_ref().unwrap()
    else {
        panic!("username draft")
    };
    let report_id = common.report_id;
    click(&mut input, &mut world, HudPlayerReportingFocus::Comments);
    assert!(input.handle_player_reporting_text(&mut world, &"😀".repeat(600)));
    assert_eq!(
        world
            .player_reporting()
            .draft
            .as_ref()
            .unwrap()
            .common()
            .comments
            .encode_utf16()
            .count(),
        MAX_REPORT_COMMENT_UTF16_UNITS
    );
    assert!(input.handle_player_reporting_key(&mut world, KeyCode::Enter, true));
    assert_eq!(
        world
            .player_reporting()
            .draft
            .as_ref()
            .unwrap()
            .common()
            .comments
            .encode_utf16()
            .count(),
        MAX_REPORT_COMMENT_UTF16_UNITS
    );

    click(&mut input, &mut world, HudPlayerReportingFocus::Back);
    assert!(matches!(
        input.player_reporting_runtime().unwrap().screen(),
        PlayerReportingRuntimeScreen::DiscardWarning(PlayerReportType::Username)
    ));
    click(
        &mut input,
        &mut world,
        HudPlayerReportingFocus::WarningSaveDraft,
    );
    assert!(world.player_reporting_screen().is_none());
    assert!(world.social_interactions_screen().is_some());
    assert_eq!(
        world
            .player_reporting()
            .draft
            .as_ref()
            .unwrap()
            .common()
            .report_id,
        report_id
    );

    assert!(input.open_player_reporting_screen(&mut world, remote));
    assert!(matches!(
        input.player_reporting_runtime().unwrap().screen(),
        PlayerReportingRuntimeScreen::DraftChoice
    ));
    click(&mut input, &mut world, HudPlayerReportingFocus::DraftEdit);
    click(&mut input, &mut world, HudPlayerReportingFocus::Back);
    click(
        &mut input,
        &mut world,
        HudPlayerReportingFocus::WarningDiscard,
    );
    assert!(world.player_reporting().draft.is_none());
    assert!(world.social_interactions_screen().is_some());
}

#[test]
fn player_reporting_reason_selection_commits_only_on_done() {
    let (mut world, remote) = reporting_world();
    let mut input = ClientInputState::new(true);
    open_reporting(&mut input, &mut world, remote);
    click(
        &mut input,
        &mut world,
        HudPlayerReportingFocus::ReportType(1),
    );
    click(
        &mut input,
        &mut world,
        HudPlayerReportingFocus::SelectReason,
    );
    click(&mut input, &mut world, HudPlayerReportingFocus::Reason(0));
    click(&mut input, &mut world, HudPlayerReportingFocus::ReadInfo);
    assert_eq!(
        input.take_player_reporting_platform_requests(),
        vec![PlayerReportingPlatformRequest::OpenUri(
            REPORTING_HELP_URL.to_string()
        )]
    );
    click(&mut input, &mut world, HudPlayerReportingFocus::Back);
    assert_eq!(selected_reason(&world), None);

    click(
        &mut input,
        &mut world,
        HudPlayerReportingFocus::SelectReason,
    );
    click(&mut input, &mut world, HudPlayerReportingFocus::Reason(0));
    click(&mut input, &mut world, HudPlayerReportingFocus::Done);
    assert_eq!(
        selected_reason(&world),
        Some(PlayerReportReason::IWantToReportThem)
    );
}

#[test]
fn player_reporting_submission_matches_report_attempt_and_preserves_retry_draft() {
    let (mut world, remote) = reporting_world();
    let mut input = ClientInputState::new(true);
    open_reporting(&mut input, &mut world, remote);
    click(
        &mut input,
        &mut world,
        HudPlayerReportingFocus::ReportType(2),
    );
    click(&mut input, &mut world, HudPlayerReportingFocus::Attestation);
    click(&mut input, &mut world, HudPlayerReportingFocus::Send);
    let request = input
        .take_player_reporting_submissions()
        .pop()
        .expect("submission request");
    let report_id = request.report.id;
    assert!(world.begin_player_report_submission(7, report_id, request.attempt));
    assert!(!input.apply_player_reporting_submission_result(
        &mut world,
        7,
        report_id,
        request.attempt + 1,
        true,
    ));
    assert!(input.apply_player_reporting_submission_result(
        &mut world,
        7,
        report_id,
        request.attempt,
        false,
    ));
    assert_eq!(
        world.player_reporting().submission.unwrap().status,
        PlayerReportSubmissionStatus::Failed
    );
    assert!(world.player_reporting().draft.is_some());
    assert!(matches!(
        input.player_reporting_runtime().unwrap().screen(),
        PlayerReportingRuntimeScreen::Failure
    ));

    click(&mut input, &mut world, HudPlayerReportingFocus::Retry);
    let retry = input
        .take_player_reporting_submissions()
        .pop()
        .expect("retry request");
    assert_eq!(retry.report.id, report_id);
    assert!(retry.attempt > request.attempt);
    assert!(world.begin_player_report_submission(7, report_id, retry.attempt));
    assert!(input.apply_player_reporting_submission_result(
        &mut world,
        7,
        report_id,
        retry.attempt,
        true,
    ));
    assert!(world.player_reporting().draft.is_none());
    assert_eq!(
        world.player_reporting().submission.unwrap().status,
        PlayerReportSubmissionStatus::Succeeded
    );
    assert!(matches!(
        input.player_reporting_runtime().unwrap().screen(),
        PlayerReportingRuntimeScreen::Success
    ));
    click(&mut input, &mut world, HudPlayerReportingFocus::Done);
    assert!(world.player_reporting_screen().is_none());
    assert!(world.social_interactions_screen().is_none());
    assert!(world.player_reporting().submission.is_none());
}

#[test]
fn player_reporting_sending_cancel_returns_to_editor_and_rejects_completion() {
    let (mut world, remote) = reporting_world();
    let mut input = ClientInputState::new(true);
    open_reporting(&mut input, &mut world, remote);
    click(
        &mut input,
        &mut world,
        HudPlayerReportingFocus::ReportType(2),
    );
    click(&mut input, &mut world, HudPlayerReportingFocus::Attestation);
    click(&mut input, &mut world, HudPlayerReportingFocus::Send);
    let request = input
        .take_player_reporting_submissions()
        .pop()
        .expect("submission request");
    assert!(world.begin_player_report_submission(9, request.report.id, request.attempt));

    click(&mut input, &mut world, HudPlayerReportingFocus::Cancel);
    assert!(matches!(
        input.player_reporting_runtime().unwrap().screen(),
        PlayerReportingRuntimeScreen::Editor(PlayerReportType::Username)
    ));
    assert!(world.player_reporting().draft.is_some());
    assert!(world.player_reporting().submission.is_none());
    assert_eq!(
        input.take_cancelled_player_reporting_submissions(),
        vec![(request.report.id, request.attempt)]
    );
    assert!(!input.apply_player_reporting_submission_result(
        &mut world,
        9,
        request.report.id,
        request.attempt,
        true,
    ));
}
