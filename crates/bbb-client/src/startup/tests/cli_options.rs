use super::*;
use crate::narrator_runtime::NarratorStatus;
#[cfg(not(target_arch = "wasm32"))]
use crate::startup::platform::apply_probe_timeout;
use bbb_client_net::ServerResourcePackPolicy;
use bbb_client_render::TerrainTextureFiltering;
use bbb_client_world::TutorialStep;
use bbb_protocol::packets::{ClientChatVisibility, ClientMainHand, ClientParticleStatus};

#[test]
fn lockstep_accepts_control_or_mcp_and_requires_an_endpoint() {
    for endpoint in ["--control", "--mcp-bind"] {
        assert!(
            Args::try_parse_from(["bbb-client", "--lockstep", endpoint, "127.0.0.1:12345"])
                .unwrap()
                .lockstep
        );
    }
    assert!(Args::try_parse_from(["bbb-client", "--lockstep"]).is_err());
}

#[test]
fn network_forwarder_preserves_visibility_before_simulation_wake() {
    // Same haystack as the former inline module: startup.rs plus this file.
    let source = [
        include_str!("../../startup.rs"),
        include_str!("../platform.rs"),
        include_str!("cli_options.rs"),
    ]
    .concat();
    let forwarded = source
        .find("simulation_tx.send(event).await")
        .expect("ordered network event forward");
    let wake = source
        .find("network_wake.notify_external_work()")
        .expect("simulation network wake");
    assert!(forwarded < wake);
}

#[test]
fn args_accept_delegated_status_server_and_reject_probe_conflict() {
    let args = Args::try_parse_from(["bbb-client", "--status-server", "127.0.0.1:25565"]).unwrap();
    assert_eq!(args.status_server.as_deref(), Some("127.0.0.1:25565"));
    assert!(Args::try_parse_from([
        "bbb-client",
        "--status-server",
        "127.0.0.1:25565",
        "--probe-server",
    ])
    .is_err());
}

#[test]
fn interaction_replay_requires_the_headless_probe() {
    assert!(Args::try_parse_from(["bbb-client", "--probe-server-interactions"]).is_err());
    let args = Args::try_parse_from([
        "bbb-client",
        "--probe-server",
        "--probe-server-interactions",
    ])
    .unwrap();
    assert!(args.probe_server);
    assert!(args.probe_server_interactions);
}

#[test]
fn dimension_transition_probe_is_explicit_and_isolated() {
    assert!(Args::try_parse_from(["bbb-client", "--probe-server-dimension-transitions",]).is_err());
    let args = Args::try_parse_from([
        "bbb-client",
        "--probe-server",
        "--probe-server-dimension-transitions",
    ])
    .unwrap();
    assert!(args.probe_server_dimension_transitions);
    assert!(Args::try_parse_from([
        "bbb-client",
        "--probe-server",
        "--probe-server-dimension-transitions",
        "--probe-server-interactions",
    ])
    .is_err());
    assert!(Args::try_parse_from([
        "bbb-client",
        "--probe-server",
        "--probe-server-dimension-transitions",
        "--vanilla-root",
        "/tmp/vanilla",
    ])
    .is_err());
}

#[test]
fn entity_runtime_probe_is_explicit_and_isolated() {
    assert!(Args::try_parse_from(["bbb-client", "--probe-server-entity-runtime"]).is_err());
    let args = Args::try_parse_from([
        "bbb-client",
        "--probe-server",
        "--probe-server-entity-runtime",
    ])
    .unwrap();
    assert!(args.probe_server_entity_runtime);
    for conflict in [
        "--probe-server-interactions",
        "--probe-server-dimension-transitions",
        "--probe-hold-seconds",
        "--vanilla-root",
    ] {
        let mut argv = vec![
            "bbb-client",
            "--probe-server",
            "--probe-server-entity-runtime",
            conflict,
        ];
        if conflict == "--probe-hold-seconds" {
            argv.push("5");
        } else if conflict == "--vanilla-root" {
            argv.push("/tmp/vanilla");
        }
        assert!(Args::try_parse_from(argv).is_err(), "accepted {conflict}");
    }
}

#[test]
fn container_menu_probe_is_explicit_and_isolated() {
    assert!(Args::try_parse_from(["bbb-client", "--probe-server-container-menu"]).is_err());
    let args = Args::try_parse_from([
        "bbb-client",
        "--probe-server",
        "--probe-server-container-menu",
    ])
    .unwrap();
    assert!(args.probe_server_container_menu);
    for conflict in [
        "--probe-server-interactions",
        "--probe-server-dimension-transitions",
        "--probe-server-entity-runtime",
        "--probe-hold-seconds",
        "--vanilla-root",
    ] {
        let mut argv = vec![
            "bbb-client",
            "--probe-server",
            "--probe-server-container-menu",
            conflict,
        ];
        if conflict == "--probe-hold-seconds" {
            argv.push("5");
        } else if conflict == "--vanilla-root" {
            argv.push("/tmp/vanilla");
        }
        assert!(Args::try_parse_from(argv).is_err(), "accepted {conflict}");
    }
}

#[test]
fn holding_a_session_open_also_requires_the_headless_probe() {
    assert!(Args::try_parse_from(["bbb-client", "--probe-hold-seconds", "5"]).is_err());
    let args = Args::try_parse_from(["bbb-client", "--probe-server", "--probe-hold-seconds", "5"])
        .unwrap();
    assert_eq!(args.probe_hold_seconds, Some(5));
    // Absent means the probe leaves as soon as its program is done, which
    // is what every existing caller relies on.
    let args = Args::try_parse_from(["bbb-client", "--probe-server"]).unwrap();
    assert_eq!(args.probe_hold_seconds, None);
}

#[test]
fn probe_timeout_is_probe_only_and_positive() {
    let command = Args::command();
    let probe_timeout = command
        .get_arguments()
        .find(|arg| arg.get_id() == "probe_timeout_seconds")
        .expect("--probe-timeout-seconds should be defined");
    assert!(probe_timeout.is_hide_set());
    let help = probe_timeout
        .get_help()
        .map(ToString::to_string)
        .unwrap_or_default();
    assert!(help.contains("headless probe's base connection deadline"));
    assert!(help.contains("omitted keeps the default"));

    assert!(Args::try_parse_from(["bbb-client", "--probe-timeout-seconds", "90"]).is_err());
    assert!(Args::try_parse_from([
        "bbb-client",
        "--probe-server",
        "--probe-timeout-seconds",
        "0",
    ])
    .is_err());

    let args = Args::try_parse_from([
        "bbb-client",
        "--probe-server",
        "--probe-timeout-seconds",
        "90",
    ])
    .unwrap();
    assert_eq!(args.probe_timeout_seconds, Some(90));
}

#[test]
fn probe_timeout_overrides_only_when_supplied() {
    let args = Args::try_parse_from(["bbb-client", "--probe-server"]).unwrap();
    assert_eq!(args.probe_timeout_seconds, None);
    let identity = client_identity_from_args(&args);
    let mut options = connection_options_from_args(&args, &identity).unwrap();
    apply_probe_timeout(&args, &mut options);
    assert_eq!(options.timeout, Duration::from_secs(20));

    let args = Args::try_parse_from([
        "bbb-client",
        "--probe-server",
        "--probe-timeout-seconds",
        "90",
    ])
    .unwrap();
    let identity = client_identity_from_args(&args);
    let mut options = connection_options_from_args(&args, &identity).unwrap();
    apply_probe_timeout(&args, &mut options);
    assert_eq!(options.timeout, Duration::from_secs(90));
}

#[test]
fn build_info_is_machine_readable_and_uses_protocol_constants() {
    let args = Args::try_parse_from(["bbb-client", "--build-info"]).unwrap();
    assert!(args.build_info);

    let value = serde_json::to_value(BuildInfo::current()).unwrap();
    assert_eq!(value["schema"], 2);
    assert_eq!(value["client"], "bbb-client");
    assert_eq!(value["client_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["source_commit"], env!("BBB_SOURCE_COMMIT"));
    assert_eq!(value["source_dirty"], env!("BBB_SOURCE_DIRTY") == "true");
    assert_eq!(value["minecraft_version"], bbb_protocol::MC_VERSION);
    assert_eq!(value["protocol_version"], bbb_protocol::PROTOCOL_VERSION);
    assert_eq!(
        value["resource_pack_format"],
        bbb_protocol::MC_RESOURCE_PACK_FORMAT.to_vanilla_string()
    );
    assert_eq!(
        value["data_pack_format"],
        bbb_protocol::MC_DATA_PACK_FORMAT.to_vanilla_string()
    );
    assert_eq!(value["target_os"], std::env::consts::OS);
    assert_eq!(value["target_arch"], std::env::consts::ARCH);
    assert_eq!(value["wgpu_version"], "30.0.0");
    assert_eq!(value["wgpu_core_version"], "30.0.0");
    assert_eq!(value["wgpu_hal_version"], "30.0.0");
    assert_eq!(value["wgpu_types_version"], "30.0.0");
    assert_eq!(value["naga_version"], "30.0.0");
    assert_eq!(
        value["upstream_metal_fence_commit"],
        "e0a6065c65b5b6c2e92ee7052ed084cd8910d486"
    );
    assert_eq!(
        value["upstream_metal_fence_strategy"],
        "completion-handler-condition-variable"
    );
}

#[test]
fn tracing_keeps_machine_readable_cli_stdout_clean() {
    let source = include_str!("../platform.rs");
    assert!(source.contains(".with_writer(std::io::stderr)"));
}

#[cfg(target_os = "linux")]
#[test]
fn render_thread_name_is_visible_to_linux_profilers() {
    name_current_render_thread().unwrap();
    assert_eq!(
        std::fs::read_to_string("/proc/thread-self/comm")
            .unwrap()
            .trim_end(),
        "Render thread"
    );
}

#[test]
fn server_list_owner_requires_explicit_store_name_and_live_connection() {
    assert!(
        Args::try_parse_from(["bbb-client", "--server-list-store", "/tmp/servers.dat"]).is_err()
    );
    assert!(Args::try_parse_from(["bbb-client", "--server-list-name", "Native Server"]).is_err());
    let args = Args::try_parse_from([
        "bbb-client",
        "--connect-server",
        "--server",
        "example.org:25565",
        "--server-list-store",
        "/tmp/servers.dat",
        "--server-list-name",
        "Native Server",
    ])
    .unwrap();
    assert_eq!(
        args.server_list_store,
        Some(PathBuf::from("/tmp/servers.dat"))
    );
    assert_eq!(args.server_list_name.as_deref(), Some("Native Server"));
}

#[test]
fn args_accept_repeated_resource_pack_dirs() {
    let args = Args::try_parse_from([
        "bbb-client",
        "--resource-pack-dir",
        "packs/base",
        "--resource-pack-dir",
        "packs/overlay",
    ])
    .unwrap();

    assert_eq!(
        args.resource_pack_dirs,
        vec![PathBuf::from("packs/base"), PathBuf::from("packs/overlay")]
    );
}

#[test]
fn social_interactions_blocked_players_are_startup_uuid_inputs() {
    let first = Uuid::from_u128(0x11111111111141118111111111111111);
    let second = Uuid::from_u128(0x22222222222242228222222222222222);
    let args = Args::try_parse_from([
        "bbb-client",
        "--blocked-player",
        &first.to_string(),
        "--blocked-player",
        &second.to_string(),
    ])
    .unwrap();

    assert_eq!(args.client_blocked_players, vec![first, second]);
    assert!(Args::try_parse_from(["bbb-client", "--blocked-player", "not-a-uuid",]).is_err());
}

#[test]
fn args_select_server_resource_pack_policy_and_cache_directory() {
    let defaults = Args::try_parse_from(["bbb-client"]).unwrap();
    let default_identity = client_identity_from_args(&defaults);
    assert_eq!(defaults.server_resource_pack_policy, None);
    assert_eq!(defaults.server_resource_pack_cache_dir, None);
    assert_eq!(defaults.proxy_host, None);
    assert_eq!(defaults.proxy_port, 8080);
    assert!(server_resource_pack_proxy_from_args(&defaults).is_none());
    assert_eq!(
        connection_options_from_args(&defaults, &default_identity)
            .unwrap()
            .server_resource_pack_policy,
        ServerResourcePackPolicy::Prompt
    );

    let prompt =
        Args::try_parse_from(["bbb-client", "--server-resource-pack-policy", "prompt"]).unwrap();
    assert_eq!(
        prompt.server_resource_pack_policy,
        Some(ServerResourcePackPolicyArg::Prompt)
    );

    let accepted = Args::try_parse_from([
        "bbb-client",
        "--server-resource-pack-policy",
        "accept",
        "--server-resource-pack-cache-dir",
        "/tmp/bbb-server-packs",
    ])
    .unwrap();
    let accepted_identity = client_identity_from_args(&accepted);
    assert_eq!(
        accepted.server_resource_pack_policy,
        Some(ServerResourcePackPolicyArg::Accept)
    );
    assert_eq!(
        accepted.server_resource_pack_cache_dir,
        Some(PathBuf::from("/tmp/bbb-server-packs"))
    );
    assert_eq!(
        connection_options_from_args(&accepted, &accepted_identity)
            .unwrap()
            .server_resource_pack_policy,
        ServerResourcePackPolicy::Accept
    );
}

#[test]
fn args_project_vanilla_socks_proxy_without_exposing_password() {
    let args = Args::try_parse_from([
        "bbb-client",
        "--proxy-host",
        "proxy.example",
        "--proxy-port",
        "1080",
        "--proxy-user",
        "proxy-user",
        "--proxy-pass",
        "proxy-password",
    ])
    .unwrap();

    assert_eq!(args.proxy_host.as_deref(), Some("proxy.example"));
    assert_eq!(args.proxy_port, 1080);
    assert!(server_resource_pack_proxy_from_args(&args).is_some());
    let debug = format!("{args:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("proxy-password"));

    let invalid_port = Args::try_parse_from([
        "bbb-client",
        "--proxy-host",
        "proxy.example",
        "--proxy-port",
        "65536",
    ])
    .unwrap();
    assert!(server_resource_pack_proxy_from_args(&invalid_port).is_none());
}

#[test]
fn args_accept_player_skin_cache_dir() {
    let args =
        Args::try_parse_from(["bbb-client", "--player-skin-cache-dir", "/tmp/bbb-skins"]).unwrap();

    assert_eq!(
        args.player_skin_cache_dir,
        Some(PathBuf::from("/tmp/bbb-skins"))
    );
}

#[test]
fn args_accept_explicit_crash_report_directory() {
    let defaults = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(defaults.crash_report_dir, None);

    let configured =
        Args::try_parse_from(["bbb-client", "--crash-report-dir", "/tmp/bbb-crash-reports"])
            .unwrap();
    assert_eq!(
        configured.crash_report_dir,
        Some(PathBuf::from("/tmp/bbb-crash-reports"))
    );
}

#[test]
fn args_accept_narrator_status_command_and_repeated_arguments() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(default_args.client_narrator, ClientNarratorArg::Off);
    assert_eq!(default_args.narrator_command, None);
    assert!(default_args.narrator_command_args.is_empty());

    let args = Args::try_parse_from([
        "bbb-client",
        "--narrator",
        "system",
        "--narrator-command",
        "/usr/bin/spd-say",
        "--narrator-command-arg=--wait",
        "--narrator-command-arg={text}",
    ])
    .unwrap();
    assert_eq!(args.client_narrator, ClientNarratorArg::System);
    assert_eq!(
        args.narrator_command,
        Some(PathBuf::from("/usr/bin/spd-say"))
    );
    assert_eq!(args.narrator_command_args, ["--wait", "{text}"]);
    assert_eq!(
        NarratorStatus::from(args.client_narrator),
        NarratorStatus::System
    );
}

#[test]
fn startup_sound_subtitle_options_use_vanilla_defaults_and_bounds() {
    let defaults = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(!defaults.client_show_subtitles);
    assert_eq!(defaults.notification_display_time, 1.0);

    let configured = Args::try_parse_from([
        "bbb-client",
        "--show-subtitles=true",
        "--notification-display-time",
        "10.0",
    ])
    .unwrap();
    assert!(configured.client_show_subtitles);
    assert_eq!(configured.notification_display_time, 10.0);

    for invalid in ["0.49", "10.01", "NaN", "inf"] {
        assert!(
            Args::try_parse_from(["bbb-client", "--notification-display-time", invalid,]).is_err()
        );
    }
}

#[test]
fn startup_music_options_use_vanilla_defaults_and_cli_names() {
    let defaults = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(
        defaults.client_music_toast,
        MusicToastDisplayStateArg::Never
    );
    assert_eq!(defaults.client_music_frequency, MusicFrequencyArg::Default);

    let configured = Args::try_parse_from([
        "bbb-client",
        "--music-toast",
        "pause-and-toast",
        "--music-frequency",
        "constant",
    ])
    .unwrap();
    assert_eq!(
        configured.client_music_toast,
        MusicToastDisplayStateArg::PauseAndToast
    );
    assert_eq!(
        configured.client_music_frequency,
        MusicFrequencyArg::Constant
    );
}

#[test]
fn stored_music_options_reach_startup_and_explicit_cli_wins() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("musicToast", r#""pause_and_toast""#);
    options.set_raw_for_test("musicFrequency", r#""FREQUENT""#);

    let mut stored_args = Args::parse_from(["bbb-client"]);
    bind_stored_client_options(
        &mut stored_args,
        StoredOptions::without_command_line(&options),
    );
    assert_eq!(
        stored_args.client_music_toast,
        MusicToastDisplayStateArg::PauseAndToast
    );
    assert_eq!(
        stored_args.client_music_frequency,
        MusicFrequencyArg::Frequent
    );

    let matches = Args::command().get_matches_from([
        "bbb-client",
        "--music-toast",
        "pause",
        "--music-frequency",
        "constant",
    ]);
    let mut explicit_args = Args::from_arg_matches(&matches).unwrap();
    bind_stored_client_options(&mut explicit_args, StoredOptions::new(&options, &matches));
    assert_eq!(
        explicit_args.client_music_toast,
        MusicToastDisplayStateArg::Pause
    );
    assert_eq!(
        explicit_args.client_music_frequency,
        MusicFrequencyArg::Constant
    );
}

#[test]
fn tutorial_step_is_startup_configured_and_defaults_to_movement() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(default_args.tutorial_step, TutorialStepArg::Movement);
    assert_eq!(
        TutorialStep::from(default_args.tutorial_step),
        TutorialStep::Movement
    );

    let args = Args::try_parse_from(["bbb-client", "--tutorial-step", "craft-planks"]).unwrap();
    assert_eq!(args.tutorial_step, TutorialStepArg::CraftPlanks);
    assert_eq!(
        TutorialStep::from(args.tutorial_step),
        TutorialStep::CraftPlanks
    );
}

#[test]
fn joined_first_server_is_consumed_at_startup_and_explicit_cli_wins() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("joinedFirstServer", "true");

    let mut stored_args = Args::parse_from(["bbb-client"]);
    bind_stored_client_options(
        &mut stored_args,
        StoredOptions::without_command_line(&options),
    );
    assert!(stored_args.client_joined_first_server);

    let matches =
        Args::command().get_matches_from(["bbb-client", "--joined-first-server", "false"]);
    let mut explicit_args = Args::from_arg_matches(&matches).unwrap();
    bind_stored_client_options(&mut explicit_args, StoredOptions::new(&options, &matches));
    assert!(!explicit_args.client_joined_first_server);
}

#[test]
fn args_accept_vanilla_debug_framerate_limits() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(
        default_args.client_framerate_limit,
        VANILLA_UNLIMITED_FRAMERATE_LIMIT
    );

    let capped = Args::try_parse_from(["bbb-client", "--framerate-limit", "120"]).unwrap();
    assert_eq!(capped.client_framerate_limit, 120);

    let unlimited = Args::try_parse_from(["bbb-client", "--framerate-limit", "inf"]).unwrap();
    assert_eq!(
        unlimited.client_framerate_limit,
        VANILLA_UNLIMITED_FRAMERATE_LIMIT
    );

    // `Codec.intRange(10, 260)` loads every in-range value; the slider
    // granularity never restricts the stored one.
    let odd = Args::try_parse_from(["bbb-client", "--framerate-limit", "251"]).unwrap();
    assert_eq!(odd.client_framerate_limit, 251);
    assert!(Args::try_parse_from(["bbb-client", "--framerate-limit", "9"]).is_err());
}

#[test]
fn cloud_range_matches_vanilla_option_bounds() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(
        default_args.cloud_range_chunks,
        VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS
    );

    let fancy = Args::try_parse_from(["bbb-client", "--cloud-range", "64"]).unwrap();
    assert_eq!(fancy.cloud_range_chunks, 64);
    assert!(Args::try_parse_from(["bbb-client", "--cloud-range", "1"]).is_err());
    assert!(Args::try_parse_from(["bbb-client", "--cloud-range", "129"]).is_err());
}

#[test]
fn args_accept_vanilla_default_vsync_toggle() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(default_args.client_vsync);

    let disabled = Args::try_parse_from(["bbb-client", "--vsync", "false"]).unwrap();
    assert!(!disabled.client_vsync);
}

#[test]
fn args_accept_explicit_vanilla_terrain_texture_filtering() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(
        default_args.terrain_texture_filtering,
        TerrainTextureFilteringArg::None
    );

    let rgss = Args::try_parse_from(["bbb-client", "--terrain-texture-filtering", "rgss"]).unwrap();
    assert_eq!(
        TerrainTextureFiltering::from(rgss.terrain_texture_filtering),
        TerrainTextureFiltering::Rgss
    );

    let anisotropic =
        Args::try_parse_from(["bbb-client", "--terrain-texture-filtering", "anisotropic"]).unwrap();
    assert_eq!(
        TerrainTextureFiltering::from(anisotropic.terrain_texture_filtering),
        TerrainTextureFiltering::Anisotropic
    );
}

#[test]
fn max_anisotropy_bit_matches_the_vanilla_int_range() {
    // Vanilla `maxAnisotropyBit` is `OptionInstance.IntRange(1, 3)` with
    // default 2.
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(default_args.client_max_anisotropy_bit, 2);
    let three = Args::try_parse_from(["bbb-client", "--max-anisotropy-bit", "3"]).unwrap();
    assert_eq!(three.client_max_anisotropy_bit, 3);
    assert!(Args::try_parse_from(["bbb-client", "--max-anisotropy-bit", "0"]).is_err());
    assert!(Args::try_parse_from(["bbb-client", "--max-anisotropy-bit", "4"]).is_err());

    // Stored `maxAnisotropyBit` outside the codec range fails validation
    // and keeps the default, like vanilla's `OptionInstance` read.
    assert_eq!(parse_client_max_anisotropy_bit("1"), Ok(1));
    assert!(parse_client_max_anisotropy_bit("0").is_err());
    assert!(parse_client_max_anisotropy_bit("4").is_err());
    assert!(parse_client_max_anisotropy_bit("x").is_err());
}

#[test]
fn stored_texture_filtering_wraps_ids_like_the_vanilla_legacy_codec() {
    // `TextureFilteringMethod.LEGACY_CODEC` resolves ids through
    // `ByIdMap.continuous(..., OutOfBoundsStrategy.WRAP)` (floorMod).
    assert_eq!(
        parse_stored_terrain_texture_filtering("0"),
        Ok(TerrainTextureFilteringArg::None)
    );
    assert_eq!(
        parse_stored_terrain_texture_filtering("1"),
        Ok(TerrainTextureFilteringArg::Rgss)
    );
    assert_eq!(
        parse_stored_terrain_texture_filtering("2"),
        Ok(TerrainTextureFilteringArg::Anisotropic)
    );
    assert_eq!(
        parse_stored_terrain_texture_filtering("3"),
        Ok(TerrainTextureFilteringArg::None)
    );
    assert_eq!(
        parse_stored_terrain_texture_filtering("-1"),
        Ok(TerrainTextureFilteringArg::Anisotropic)
    );
    assert!(parse_stored_terrain_texture_filtering("fancy").is_err());
}

#[test]
fn terrain_atlas_base_padding_matches_the_vanilla_stitcher_formula() {
    // Vanilla `Stitcher`: padding = `1 << mipLevel << clamp(bit - 1, 0,
    // 4)` with bit 0 outside `ANISOTROPIC`; the packer multiplies the
    // base by `1 << mipLevel`, so the base is the anisotropy factor.
    assert_eq!(
        terrain_atlas_base_padding(TerrainTextureFilteringArg::None, 3),
        1
    );
    assert_eq!(
        terrain_atlas_base_padding(TerrainTextureFilteringArg::Rgss, 3),
        1
    );
    assert_eq!(
        terrain_atlas_base_padding(TerrainTextureFilteringArg::Anisotropic, 1),
        1
    );
    assert_eq!(
        terrain_atlas_base_padding(TerrainTextureFilteringArg::Anisotropic, 2),
        2
    );
    assert_eq!(
        terrain_atlas_base_padding(TerrainTextureFilteringArg::Anisotropic, 3),
        4
    );
    assert_eq!(
        terrain_atlas_base_padding(TerrainTextureFilteringArg::Anisotropic, 9),
        16
    );
}

#[test]
fn startup_client_auto_jump_uses_vanilla_default_and_explicit_toggle() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(default_args.client_auto_jump);

    let disabled = Args::try_parse_from(["bbb-client", "--auto-jump", "false"]).unwrap();
    assert!(!disabled.client_auto_jump);
}

#[test]
fn args_accept_vanilla_default_cutout_leaves_toggle() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(default_args.cutout_leaves);

    let disabled = Args::try_parse_from(["bbb-client", "--cutout-leaves", "false"]).unwrap();
    assert!(!disabled.cutout_leaves);
}

#[test]
fn args_accept_vanilla_chat_link_toggles() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(default_args.client_chat_links);
    assert!(default_args.client_chat_links_prompt);

    let disabled = Args::try_parse_from([
        "bbb-client",
        "--chat-links",
        "false",
        "--chat-links-prompt",
        "false",
    ])
    .unwrap();
    assert!(!disabled.client_chat_links);
    assert!(!disabled.client_chat_links_prompt);
}

#[test]
fn client_framerate_limit_interval_matches_unlimited_and_finite_modes() {
    assert_eq!(
        client_framerate_limit_interval(VANILLA_UNLIMITED_FRAMERATE_LIMIT),
        None
    );
    assert_eq!(
        client_framerate_limit_interval(120),
        Some(Duration::from_nanos(8_333_333))
    );
    // 251-259 load through `Codec.intRange(10, 260)` as literal caps; only
    // the 260 cutoff itself lifts the limiter.
    assert_eq!(
        client_framerate_limit_interval(259),
        Some(Duration::from_nanos(3_861_004))
    );
}

#[test]
fn client_framerate_limit_accepts_the_vanilla_codec_range() {
    assert_eq!(parse_client_framerate_limit("255"), Ok(255));
    assert_eq!(parse_client_framerate_limit("10"), Ok(10));
    assert_eq!(
        parse_client_framerate_limit("260"),
        Ok(VANILLA_UNLIMITED_FRAMERATE_LIMIT)
    );
    assert!(parse_client_framerate_limit("9").is_err());
    assert!(parse_client_framerate_limit("261").is_err());
}

#[test]
fn effective_framerate_limit_matches_the_vanilla_tracker() {
    // Vanilla `FramerateLimitTracker.getFramerateLimit`: NONE keeps the
    // option, iconified/long-AFK force 10, short AFK caps at 30, and an
    // out-of-level menu runs at exactly 60 even above the configured cap.
    let base = 120;
    let afk = InactivityFpsLimitArg::Afk;
    assert_eq!(
        effective_framerate_limit(base, afk, false, 0, true, false),
        120
    );
    assert_eq!(
        effective_framerate_limit(base, afk, true, 0, true, false),
        10
    );
    assert_eq!(
        effective_framerate_limit(base, afk, false, 600_001, true, false),
        10
    );
    assert_eq!(
        effective_framerate_limit(base, afk, false, 60_001, true, false),
        30
    );
    assert_eq!(
        effective_framerate_limit(20, afk, false, 60_001, true, false),
        20,
        "a short AFK takes min(limit, 30)"
    );
    assert_eq!(
        effective_framerate_limit(20, afk, false, 0, false, true),
        60,
        "the out-of-level menu limit overrides even a lower option"
    );
    assert_eq!(
        effective_framerate_limit(base, afk, false, 0, false, false),
        120
    );
    // `inactivityFpsLimit: minimized` skips the AFK branches but keeps the
    // iconified clamp.
    let minimized = InactivityFpsLimitArg::Minimized;
    assert_eq!(
        effective_framerate_limit(base, minimized, false, 600_001, true, false),
        120
    );
    assert_eq!(
        effective_framerate_limit(base, minimized, true, 0, true, false),
        10
    );
}

#[test]
fn stored_render_clouds_reads_legacy_names_and_booleans() {
    // `CloudStatus` legacy names: "true" fancy, "fast", "false" off; the
    // EnumNameOrBoolean shape also loads bare booleans.
    assert_eq!(
        parse_stored_render_clouds("\"true\""),
        Ok(RenderCloudsArg::Fancy)
    );
    assert_eq!(
        parse_stored_render_clouds("true"),
        Ok(RenderCloudsArg::Fancy)
    );
    assert_eq!(
        parse_stored_render_clouds("\"fast\""),
        Ok(RenderCloudsArg::Fast)
    );
    assert_eq!(
        parse_stored_render_clouds("false"),
        Ok(RenderCloudsArg::Off)
    );
    assert_eq!(parse_stored_render_clouds("off"), Ok(RenderCloudsArg::Off));
    assert!(parse_stored_render_clouds("\"sometimes\"").is_err());
}

#[test]
fn stored_inactivity_fps_limit_reads_quoted_enum_names() {
    assert_eq!(
        parse_stored_inactivity_fps_limit("\"minimized\""),
        Ok(InactivityFpsLimitArg::Minimized)
    );
    assert_eq!(
        parse_stored_inactivity_fps_limit("afk"),
        Ok(InactivityFpsLimitArg::Afk)
    );
    assert!(parse_stored_inactivity_fps_limit("\"other\"").is_err());
}

#[test]
fn args_build_client_information_from_startup_options() {
    let args = Args::try_parse_from([
        "bbb-client",
        "--locale",
        "zh_cn",
        "--render-distance",
        "12",
        "--chat-visibility",
        "system",
        "--chat-colors",
        "false",
        "--skin-parts",
        "21",
        "--main-hand",
        "left",
        "--text-filtering",
        "true",
        "--allow-server-listing",
        "true",
        "--particles",
        "minimal",
    ])
    .unwrap();

    let information = client_information_from_args(&args).unwrap();

    assert_eq!(information.language, "zh_cn");
    // Vanilla reports `renderDistance.get()`, so there is no second
    // view-distance setting that could disagree with what is rendered.
    assert_eq!(information.view_distance, 12);
    assert_eq!(
        i64::from(information.view_distance),
        i64::from(args.render_distance_chunks)
    );
    assert_eq!(information.chat_visibility, ClientChatVisibility::System);
    assert!(!information.chat_colors);
    assert_eq!(information.displayed_skin_parts, 21);
    assert_eq!(information.main_hand, ClientMainHand::Left);
    assert!(information.text_filtering_enabled);
    assert!(information.allows_listing);
    assert_eq!(information.particle_status, ClientParticleStatus::Minimal);
}

#[test]
fn args_accept_chat_listener_presentation_options() {
    let defaults = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(!defaults.client_only_show_secure_chat);
    assert!(defaults.client_hide_matched_names);
    assert_eq!(defaults.client_chat_delay_seconds, 0.0);
    assert!(!defaults.client_save_chat_drafts);

    let configured = Args::try_parse_from([
        "bbb-client",
        "--only-show-secure-chat",
        "true",
        "--hide-matched-names",
        "false",
        "--chat-delay-seconds",
        "6.0",
        "--save-chat-drafts",
        "true",
    ])
    .unwrap();
    assert!(configured.client_only_show_secure_chat);
    assert!(!configured.client_hide_matched_names);
    assert_eq!(configured.client_chat_delay_seconds, 6.0);
    assert!(configured.client_save_chat_drafts);
    assert!(Args::try_parse_from(["bbb-client", "--chat-delay-seconds", "6.1",]).is_err());
}

#[test]
fn args_accept_client_gamma_startup_option() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(
        default_args.client_gamma,
        VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR
    );

    let args = Args::try_parse_from(["bbb-client", "--gamma", "0.75"]).unwrap();
    assert_eq!(args.client_gamma, 0.75);
}

#[test]
fn args_accept_vanilla_text_background_options() {
    let defaults = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(
        defaults.text_background_opacity,
        VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY
    );
    assert!(defaults.background_for_chat_only);

    let configured = Args::try_parse_from([
        "bbb-client",
        "--text-background-opacity",
        "0.75",
        "--background-for-chat-only",
        "false",
    ])
    .unwrap();
    assert_eq!(configured.text_background_opacity, 0.75);
    assert!(!configured.background_for_chat_only);
}

#[test]
fn stored_chat_scale_reaches_startup_and_explicit_cli_wins() {
    assert_eq!(Args::parse_from(["bbb-client"]).client_chat_scale, 1.0);
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("chatScale", "0.5");
    let mut stored_args = Args::parse_from(["bbb-client"]);
    bind_stored_client_options(
        &mut stored_args,
        StoredOptions::without_command_line(&options),
    );
    assert_eq!(stored_args.client_chat_scale, 0.5);

    let matches = Args::command().get_matches_from(["bbb-client", "--chat-scale", "0.25"]);
    let mut explicit_args = Args::from_arg_matches(&matches).unwrap();
    bind_stored_client_options(&mut explicit_args, StoredOptions::new(&options, &matches));
    assert_eq!(explicit_args.client_chat_scale, 0.25);

    // `UnitDouble` rejects the line and the option keeps its value.
    options.set_raw_for_test("chatScale", "1.5");
    let mut rejected_args = Args::parse_from(["bbb-client"]);
    bind_stored_client_options(
        &mut rejected_args,
        StoredOptions::without_command_line(&options),
    );
    assert_eq!(rejected_args.client_chat_scale, 1.0);
    for invalid in ["--chat-scale=-0.1", "--chat-scale=1.01", "--chat-scale=NaN"] {
        assert!(
            Args::try_parse_from(["bbb-client", invalid]).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn stored_chat_page_options_reach_the_renderer_and_explicit_cli_wins() {
    use bbb_client_render::HudChatOptions;
    let keys = [
        "chatWidth",
        "chatHeightFocused",
        "chatHeightUnfocused",
        "chatLineSpacing",
        "chatOpacity",
    ];
    assert_eq!(
        hud_chat_options_from_args(&Args::parse_from(["bbb-client"])),
        HudChatOptions::default()
    );
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    for (key, value) in keys.into_iter().zip(["0.3", "0.25", "0.1", "0.5", "0.7"]) {
        options.set_raw_for_test(key, value);
    }
    let stored = HudChatOptions {
        scale: 1.0,
        width: 0.3,
        height_focused: 0.25,
        height_unfocused: 0.1,
        line_spacing: 0.5,
        opacity: 0.7,
    };
    let mut stored_args = Args::parse_from(["bbb-client"]);
    bind_stored_client_options(
        &mut stored_args,
        StoredOptions::without_command_line(&options),
    );
    assert_eq!(hud_chat_options_from_args(&stored_args), stored);

    let matches = Args::command().get_matches_from([
        "bbb-client",
        "--chat-width",
        "0.9",
        "--chat-line-spacing=0",
        "--chat-opacity=1",
    ]);
    let mut explicit_args = Args::from_arg_matches(&matches).unwrap();
    bind_stored_client_options(&mut explicit_args, StoredOptions::new(&options, &matches));
    assert_eq!(
        hud_chat_options_from_args(&explicit_args),
        HudChatOptions {
            width: 0.9,
            line_spacing: 0.0,
            opacity: 1.0,
            ..stored
        }
    );

    // `UnitDouble` rejects each line and the option keeps its initial value.
    for key in keys {
        options.set_raw_for_test(key, "1.5");
    }
    let mut rejected_args = Args::parse_from(["bbb-client"]);
    bind_stored_client_options(
        &mut rejected_args,
        StoredOptions::without_command_line(&options),
    );
    assert_eq!(
        hud_chat_options_from_args(&rejected_args),
        HudChatOptions::default()
    );
    for invalid in [
        "--chat-width=1.01",
        "--chat-height-focused=-0.1",
        "--chat-height-unfocused=NaN",
        "--chat-line-spacing=2",
        "--chat-opacity=-1",
    ] {
        assert!(
            Args::try_parse_from(["bbb-client", invalid]).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn args_reject_text_background_opacity_outside_unit_range() {
    for invalid in [
        "--text-background-opacity=-0.1",
        "--text-background-opacity=1.1",
        "--text-background-opacity=NaN",
    ] {
        let error = Args::try_parse_from(["bbb-client", invalid]).unwrap_err();
        assert!(error.to_string().contains("between 0.0 and 1.0"));
    }
}

#[test]
fn cli_flags_have_docs_headings_and_hidden_diagnostics() {
    let command = Args::command();
    for arg in command.get_arguments() {
        let id = arg.get_id().as_str();
        if matches!(id, "help" | "version") {
            continue;
        }
        assert!(
            arg.get_help().is_some() || arg.get_long_help().is_some(),
            "CLI flag `{id}` is missing a doc comment / help description"
        );
        assert!(
            arg.get_help_heading().is_some(),
            "CLI flag `{id}` is missing a clap help_heading"
        );
        let expect_hidden = id.starts_with("probe_")
            || id.starts_with("debug_")
            || matches!(
                id,
                "renderer_input_latency_diagnostics" | "remote_debug_renderers"
            );
        assert_eq!(
            arg.is_hide_set(),
            expect_hidden,
            "unexpected visibility for CLI flag `{id}`"
        );
    }
    let long_help = Args::command().render_long_help().to_string();
    assert!(long_help.contains("--status-server"));
    assert!(!long_help.contains("--probe-server"));
    assert!(!long_help.contains("--debug-profile"));
}

#[test]
fn fullscreen_flag_handles_bare_and_explicit_bool_and_removed_aliases_fail() {
    assert!(!Args::try_parse_from(["bbb-client"]).unwrap().fullscreen);
    assert!(
        Args::try_parse_from(["bbb-client", "--fullscreen"])
            .unwrap()
            .fullscreen
    );
    assert!(
        Args::try_parse_from(["bbb-client", "--fullscreen", "true"])
            .unwrap()
            .fullscreen
    );
    assert!(
        !Args::try_parse_from(["bbb-client", "--fullscreen", "false"])
            .unwrap()
            .fullscreen
    );
    assert!(
        !Args::try_parse_from(["bbb-client", "--fullscreen=false"])
            .unwrap()
            .fullscreen
    );

    let mut combined = Args::try_parse_from([
        "bbb-client",
        "--fullscreen",
        "--quick-play-multiplayer",
        "127.0.0.1:25570",
    ])
    .unwrap();
    apply_vanilla_bootstrap_arguments(&mut combined).unwrap();
    assert!(combined.fullscreen);
    assert_eq!(combined.server, "127.0.0.1:25570");

    for removed in [
        "--client-fullscreen",
        "--client-locale",
        "--client-simulation-distance",
        "--gameDir",
        "--quickPlayMultiplayer",
        "--fullscreenWidth",
        "--fullscreenHeight",
        "--proxyHost",
        "--proxyPort",
        "--proxyUser",
        "--proxyPass",
        "--panorama-scroll-speed",
    ] {
        assert!(
            Args::try_parse_from(["bbb-client", removed]).is_err(),
            "legacy alias or removed flag {removed} must be rejected"
        );
    }
}

#[test]
fn stored_panorama_scroll_speed_loads_without_cli_flag() {
    let mut args = Args::default_values();
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("panoramaScrollSpeed", "0.25");
    apply_loaded_client_options_to_args(&mut args, &options, |_| false);
    assert_eq!(args.panorama_scroll_speed, 0.25);
}
