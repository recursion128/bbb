#[cfg(not(target_arch = "wasm32"))]
use super::platform::apply_resource_pack_dirs;
use super::*;

mod cli_options;

#[test]
fn args_accept_render_distance_startup_option() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(
        default_args.render_distance_chunks,
        VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS
    );

    let args = Args::try_parse_from(["bbb-client", "--render-distance", "20"]).unwrap();
    assert_eq!(args.render_distance_chunks, 20);
}

#[test]
fn args_accept_vanilla_menu_background_blurriness_range() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(
        default_args.menu_background_blurriness,
        VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS
    );

    let disabled =
        Args::try_parse_from(["bbb-client", "--menu-background-blurriness", "0"]).unwrap();
    assert_eq!(disabled.menu_background_blurriness, 0);

    let maximum =
        Args::try_parse_from(["bbb-client", "--menu-background-blurriness", "10"]).unwrap();
    assert_eq!(maximum.menu_background_blurriness, 10);
}

#[test]
fn args_reject_menu_background_blurriness_outside_vanilla_range() {
    let err =
        Args::try_parse_from(["bbb-client", "--menu-background-blurriness", "11"]).unwrap_err();
    assert!(err.to_string().contains("between 0 and 10"));
}

#[test]
fn args_reject_render_distance_outside_vanilla_range() {
    let err = Args::try_parse_from(["bbb-client", "--render-distance", "1"]).unwrap_err();
    assert!(err.to_string().contains("between 2 and 32"));

    let err = Args::try_parse_from(["bbb-client", "--render-distance", "33"]).unwrap_err();
    assert!(err.to_string().contains("between 2 and 32"));

    let err = Args::try_parse_from(["bbb-client", "--render-distance", "NaN"]).unwrap_err();
    assert!(err.to_string().contains("integer chunk count"));
}

#[test]
fn args_reject_client_gamma_outside_unit_range() {
    let err = Args::try_parse_from(["bbb-client", "--gamma", "1.25"]).unwrap_err();
    assert!(err.to_string().contains("between 0.0 and 1.0"));

    let err = Args::try_parse_from(["bbb-client", "--gamma", "NaN"]).unwrap_err();
    assert!(err.to_string().contains("must be finite"));
}

#[test]
fn args_accept_fov_startup_options() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(default_args.client_fov, 70);
    assert_eq!(default_args.client_fov_effect_scale, 1.0);

    let args =
        Args::try_parse_from(["bbb-client", "--fov", "95", "--fov-effect-scale", "0.25"]).unwrap();
    assert_eq!(args.client_fov, 95);
    assert_eq!(args.client_fov_effect_scale, 0.25);
}

#[test]
fn args_accept_client_view_bobbing_startup_option() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(default_args.client_view_bobbing);

    let args = Args::try_parse_from(["bbb-client", "--view-bobbing=false"]).unwrap();
    assert!(!args.client_view_bobbing);
}

#[test]
fn args_use_vanilla_fancy_transparency_default_and_accept_explicit_improved_route() {
    let defaults = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(!defaults.improved_transparency);

    let configured = Args::try_parse_from(["bbb-client", "--improved-transparency"]).unwrap();
    assert!(configured.improved_transparency);
}

#[test]
fn args_accept_high_contrast_block_outline_startup_option() {
    let defaults = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(!defaults.high_contrast_block_outline);

    let configured = Args::try_parse_from(["bbb-client", "--high-contrast-block-outline"]).unwrap();
    assert!(configured.high_contrast_block_outline);
}

#[test]
fn args_reject_fov_startup_options_outside_vanilla_ranges() {
    let err = Args::try_parse_from(["bbb-client", "--fov", "29"]).unwrap_err();
    assert!(err.to_string().contains("between 30 and 110"));

    let err = Args::try_parse_from(["bbb-client", "--fov", "111"]).unwrap_err();
    assert!(err.to_string().contains("between 30 and 110"));

    let err = Args::try_parse_from(["bbb-client", "--fov-effect-scale", "1.25"]).unwrap_err();
    assert!(err.to_string().contains("between 0.0 and 1.0"));

    let err = Args::try_parse_from(["bbb-client", "--fov-effect-scale", "NaN"]).unwrap_err();
    assert!(err.to_string().contains("must be finite"));
}

#[test]
fn args_accept_damage_tilt_strength_startup_option() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(default_args.damage_tilt_strength, 1.0);

    let args = Args::try_parse_from(["bbb-client", "--damage-tilt-strength", "0.25"]).unwrap();
    assert_eq!(args.damage_tilt_strength, 0.25);
}

#[test]
fn args_accept_entity_shadows_startup_option() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(default_args.entity_shadows);

    let args = Args::try_parse_from(["bbb-client", "--entity-shadows=false"]).unwrap();
    assert!(!args.entity_shadows);
}

#[test]
fn args_reject_damage_tilt_strength_outside_unit_range() {
    let err = Args::try_parse_from(["bbb-client", "--damage-tilt-strength", "1.25"]).unwrap_err();
    assert!(err.to_string().contains("between 0.0 and 1.0"));

    let err = Args::try_parse_from(["bbb-client", "--damage-tilt-strength", "NaN"]).unwrap_err();
    assert!(err.to_string().contains("must be finite"));
}

#[test]
fn args_accept_hide_lightning_flash_startup_option() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(!default_args.hide_lightning_flash);

    let args = Args::try_parse_from(["bbb-client", "--hide-lightning-flash"]).unwrap();
    assert!(args.hide_lightning_flash);
}

#[test]
fn args_accept_advanced_item_tooltips_startup_option() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(!default_args.advanced_item_tooltips);

    let args = Args::try_parse_from(["bbb-client", "--advanced-item-tooltips"]).unwrap();
    assert!(args.advanced_item_tooltips);
}

#[test]
fn args_accept_debug_profile_startup_option() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(default_args.debug_profile, DebugScreenProfileArg::Default);

    let args = Args::try_parse_from(["bbb-client", "--debug-profile", "performance"]).unwrap();
    assert_eq!(args.debug_profile, DebugScreenProfileArg::Performance);
}

#[test]
fn args_accept_debug_profile_store_startup_option() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert_eq!(default_args.debug_profile_store, None);

    let args = Args::try_parse_from([
        "bbb-client",
        "--debug-profile-store",
        "/tmp/debug-profile.json",
    ])
    .unwrap();
    assert_eq!(
        args.debug_profile_store,
        Some(PathBuf::from("/tmp/debug-profile.json"))
    );
}

#[test]
fn renderer_input_latency_diagnostics_are_explicitly_opt_in() {
    assert!(
        !Args::try_parse_from(["bbb-client"])
            .unwrap()
            .renderer_input_latency_diagnostics
    );
    assert!(
        Args::try_parse_from(["bbb-client", "--renderer-input-latency-diagnostics"])
            .unwrap()
            .renderer_input_latency_diagnostics
    );
}

#[test]
fn args_accept_debug_show_local_server_entity_hit_boxes_startup_option() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(!default_args.debug_show_local_server_entity_hit_boxes);

    let args =
        Args::try_parse_from(["bbb-client", "--debug-show-local-server-entity-hit-boxes"]).unwrap();
    assert!(args.debug_show_local_server_entity_hit_boxes);
}

#[test]
fn args_accept_sharedconstants_debug_flag_startup_options() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(!default_args.debug_hotkeys);
    assert!(!default_args.debug_feature_count);

    let args =
        Args::try_parse_from(["bbb-client", "--debug-hotkeys", "--debug-feature-count"]).unwrap();
    assert!(args.debug_hotkeys);
    assert!(args.debug_feature_count);
}

#[test]
fn args_accept_repeatable_remote_debug_renderer_startup_option() {
    let default_args = Args::try_parse_from(["bbb-client"]).unwrap();
    assert!(default_args.remote_debug_renderers.is_empty());

    let args = Args::try_parse_from([
        "bbb-client",
        "--remote-debug-renderer",
        "bees",
        "--remote-debug-renderer",
        "game-event-listeners",
    ])
    .unwrap();
    assert_eq!(
        args.remote_debug_renderers,
        vec![
            RemoteDebugRenderer::Bees,
            RemoteDebugRenderer::GameEventListeners,
        ]
    );
}

#[test]
fn client_locale_rejects_more_than_sixteen_utf8_bytes() {
    let args = Args::try_parse_from(["bbb-client", "--locale", "abcdefghijklmnopq"]).unwrap();

    let err = client_information_from_args(&args).unwrap_err();

    assert!(err.to_string().contains("at most 16 UTF-8 bytes"));
}

#[test]
fn apply_resource_pack_dirs_updates_pack_roots() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp = std::env::temp_dir().join(format!(
        "bbb-startup-resource-packs-{}-{nonce}",
        std::process::id()
    ));
    let base = temp.join("packs/base");
    let overlay = temp.join("packs/overlay");
    for pack in [&base, &overlay] {
        std::fs::create_dir_all(pack).unwrap();
        std::fs::write(
            pack.join("pack.mcmeta"),
            r#"{"pack":{"description":"test","min_format":[84,0],"max_format":[84,0]}}"#,
        )
        .unwrap();
    }
    let roots = PackRoots::from_paths(
        &temp,
        temp.join("sources/26.1"),
        temp.join("sources/26.1/assets/minecraft"),
    );

    let roots = apply_resource_pack_dirs(roots, [base.clone(), overlay.clone()]).unwrap();

    assert_eq!(roots.resource_pack_dirs(), vec![base, overlay]);
    assert_eq!(
        roots
            .selected_client_packs()
            .unwrap()
            .packs()
            .iter()
            .map(|pack| pack.id.as_str())
            .collect::<Vec<_>>(),
        vec!["vanilla", "startup/0", "startup/1"]
    );
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn stored_resource_pack_lists_reconcile_the_game_directory_repository() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp = std::env::temp_dir().join(format!(
        "bbb-startup-options-resource-packs-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(temp.join("vanilla/sources/26.1/assets/minecraft")).unwrap();
    let selected = temp.join("game/resourcepacks/selected");
    let rejected = temp.join("game/resourcepacks/rejected");
    for (pack, format) in [(&selected, 84), (&rejected, 63)] {
        std::fs::create_dir_all(pack).unwrap();
        let metadata = if format > 64 {
            format!(
                r#"{{"pack":{{"description":"test","min_format":[{format},0],"max_format":[{format},0]}}}}"#
            )
        } else {
            format!(r#"{{"pack":{{"description":"test","pack_format":{format}}}}}"#)
        };
        std::fs::write(pack.join("pack.mcmeta"), metadata).unwrap();
    }
    let mut args = Args::parse_from(["bbb-client"]);
    args.vanilla_root = Some(temp.join("vanilla"));
    args.game_dir = temp.join("game");
    args.stored_resource_packs = vec!["selected".to_owned(), "file/rejected".to_owned()];

    let loaded = load_pack_roots_with_options(&args).unwrap();

    assert_eq!(
        loaded
            .roots
            .as_ref()
            .unwrap()
            .selected_client_packs()
            .unwrap()
            .packs()
            .iter()
            .map(|pack| pack.id.as_str())
            .collect::<Vec<_>>(),
        vec!["vanilla", "file/selected"]
    );
    let reconciliation = loaded.options_reconciliation.unwrap();
    assert_eq!(reconciliation.resource_packs, vec!["selected"]);
    assert_eq!(reconciliation.removed_incompatible, vec!["file/rejected"]);
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn stored_text_lists_decode_json_strings_and_fail_closed() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("resourcePacks", r#"["one","file/two"]"#);
    options.set_raw_for_test("incompatibleResourcePacks", "null");

    assert_eq!(
        stored_text_list(&options, "resourcePacks"),
        vec!["one", "file/two"]
    );
    assert!(stored_text_list(&options, "incompatibleResourcePacks").is_empty());

    options.set_raw_for_test("resourcePacks", r#"["one",7]"#);
    assert!(stored_text_list(&options, "resourcePacks").is_empty());
}

#[test]
fn stored_sound_device_decodes_codec_string_and_explicit_cli_wins() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("soundDevice", r#""OpenAL Soft on USB Headset""#);

    let mut stored_args = Args::parse_from(["bbb-client"]);
    bind_stored_client_options(
        &mut stored_args,
        StoredOptions::without_command_line(&options),
    );
    assert_eq!(
        stored_args.client_sound_device,
        "OpenAL Soft on USB Headset"
    );

    let matches =
        Args::command().get_matches_from(["bbb-client", "--sound-device", "Built-in Audio"]);
    let mut explicit_args = Args::from_arg_matches(&matches).unwrap();
    bind_stored_client_options(&mut explicit_args, StoredOptions::new(&options, &matches));
    assert_eq!(explicit_args.client_sound_device, "Built-in Audio");
}

#[test]
fn invalid_stored_sound_device_keeps_the_default_sentinel() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("soundDevice", "not-json");
    let mut args = Args::parse_from(["bbb-client"]);

    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

    assert!(args.client_sound_device.is_empty());
}

#[test]
fn stored_directional_audio_reaches_startup_and_explicit_cli_wins() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("directionalAudio", "true");
    let mut stored_args = Args::parse_from(["bbb-client"]);
    bind_stored_client_options(
        &mut stored_args,
        StoredOptions::without_command_line(&options),
    );
    assert!(stored_args.client_directional_audio);

    let matches = Args::command().get_matches_from(["bbb-client", "--directional-audio", "false"]);
    let mut explicit_args = Args::from_arg_matches(&matches).unwrap();
    bind_stored_client_options(&mut explicit_args, StoredOptions::new(&options, &matches));
    assert!(!explicit_args.client_directional_audio);
}

#[test]
fn stored_audio_volumes_cover_every_vanilla_source_with_safe_defaults() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    assert_eq!(
        stored_audio_volume_settings(&options),
        AudioVolumeSettings::default()
    );

    options.set_dynamic_raw_for_test("soundCategory_voice", "0.5");
    options.set_dynamic_raw_for_test("soundCategory_master", "0.5");
    options.set_dynamic_raw_for_test("soundCategory_music", "0.25");
    options.set_dynamic_raw_for_test("soundCategory_record", "0.75");
    options.set_dynamic_raw_for_test("soundCategory_weather", "0.1");
    options.set_dynamic_raw_for_test("soundCategory_block", "0.2");
    options.set_dynamic_raw_for_test("soundCategory_hostile", "0.3");
    options.set_dynamic_raw_for_test("soundCategory_neutral", "0.4");
    options.set_dynamic_raw_for_test("soundCategory_player", "0.6");
    options.set_dynamic_raw_for_test("soundCategory_ambient", "0.7");
    options.set_dynamic_raw_for_test("soundCategory_ui", "0.8");
    assert_eq!(
        stored_audio_volume_settings(&options),
        AudioVolumeSettings {
            master: 0.5,
            music: 0.25,
            records: 0.75,
            weather: 0.1,
            blocks: 0.2,
            hostile: 0.3,
            neutral: 0.4,
            players: 0.6,
            ambient: 0.7,
            voice: 0.5,
            ui: 0.8,
        }
    );

    options.set_dynamic_raw_for_test("soundCategory_master", "nonsense");
    assert_eq!(stored_audio_volume_settings(&options).master, 1.0);

    options.set_dynamic_raw_for_test("soundCategory_voice", "7.5");
    options.set_dynamic_raw_for_test("soundCategory_master", "0.5");
    let settings = stored_audio_volume_settings(&options);
    assert_eq!(settings.voice, 1.0);
    assert_eq!(
        settings.final_source_volume(&bbb_client_audio::AudioCategory::Voice),
        0.5
    );
}

#[test]
fn stored_options_fill_arguments_the_user_did_not_type() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("graphicsPreset", "\"custom\"");
    options.set_raw_for_test("renderDistance", "16");
    options.set_raw_for_test("entityShadows", "false");
    options.set_raw_for_test("lang", "zh_cn");
    options.set_raw_for_test("fov", "1.0");
    let mut args = Args::parse_from(["bbb-client"]);

    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

    assert_eq!(args.render_distance_chunks, 16);
    assert!(!args.entity_shadows);
    assert_eq!(args.client_locale, "zh_cn");
    // `fov` is stored scaled; 1.0 reads back as the 110-degree maximum.
    assert_eq!(args.client_fov, 110);
}

fn graphics_options_snapshot(args: &Args) -> String {
    format!(
        "{:?}|{}|{}|{:?}|{}|{}|{:?}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{}",
        args.graphics_preset,
        args.biome_blend_radius,
        args.render_distance_chunks,
        args.prioritize_chunk_updates,
        args.client_simulation_distance_chunks,
        args.client_smooth_lighting,
        args.render_clouds,
        args.client_particles,
        args.client_mipmap_levels,
        args.entity_shadows,
        args.client_entity_distance_scaling,
        args.menu_background_blurriness,
        args.cloud_range_chunks,
        args.cutout_leaves,
        args.improved_transparency,
        args.client_weather_radius,
        args.terrain_texture_filtering,
        args.client_max_anisotropy_bit,
    )
}

#[test]
fn stored_graphics_presets_overwrite_every_owned_value_after_load() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());

    options.set_raw_for_test("graphicsPreset", "\"fast\"");
    let mut fast = Args::parse_from(["bbb-client"]);
    bind_stored_client_options(&mut fast, StoredOptions::without_command_line(&options));
    assert_eq!(
        graphics_options_snapshot(&fast),
        "Fast|1|8|None|6|false|Fast|Decreased|2|false|0.75|2|32|false|false|5|None|1"
    );
    assert!(!fast.fabulous_texture_filtering);

    options.set_raw_for_test("graphicsPreset", "\"fancy\"");
    let mut fancy = Args::parse_from(["bbb-client"]);
    bind_stored_client_options(&mut fancy, StoredOptions::without_command_line(&options));
    assert_eq!(
        graphics_options_snapshot(&fancy),
        "Fancy|2|16|PlayerAffected|12|true|Fancy|All|4|true|1|5|64|true|false|10|Rgss|1"
    );
    assert!(!fancy.fabulous_texture_filtering);

    options.set_raw_for_test("graphicsPreset", "\"fabulous\"");
    let mut fabulous = Args::parse_from(["bbb-client"]);
    bind_stored_client_options(&mut fabulous, StoredOptions::without_command_line(&options));
    assert_eq!(
        graphics_options_snapshot(&fabulous),
        format!(
            "Fabulous|2|32|PlayerAffected|12|true|Fancy|All|4|true|1.25|5|128|true|{}|10|Anisotropic|2",
            !cfg!(target_os = "macos")
        )
    );
    assert!(fabulous.fabulous_texture_filtering);
}

#[test]
fn custom_graphics_preset_preserves_the_individually_stored_values() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("graphicsPreset", "\"custom\"");
    options.set_raw_for_test("renderDistance", "20");
    options.set_raw_for_test("particles", "2");
    options.set_raw_for_test("textureFiltering", "2");
    let mut args = Args::parse_from(["bbb-client"]);

    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

    assert_eq!(args.graphics_preset, GraphicsPresetArg::Custom);
    assert_eq!(args.render_distance_chunks, 20);
    assert_eq!(args.client_particles, ClientParticleStatusArg::Minimal);
    assert_eq!(
        args.terrain_texture_filtering,
        TerrainTextureFilteringArg::Anisotropic
    );
    assert!(!args.fabulous_texture_filtering);
}

#[test]
fn typed_graphics_values_override_a_typed_fabulous_preset() {
    let options = ClientOptions::defaults(ClientOptionHost::default());
    let matches = Args::command().get_matches_from([
        "bbb-client",
        "--graphics-preset",
        "fabulous",
        "--render-distance",
        "20",
        "--terrain-texture-filtering",
        "none",
    ]);
    let mut args = Args::from_arg_matches(&matches).unwrap();

    bind_stored_client_options(&mut args, StoredOptions::new(&options, &matches));

    assert_eq!(args.graphics_preset, GraphicsPresetArg::Fabulous);
    assert_eq!(args.render_distance_chunks, 20);
    assert_eq!(
        args.terrain_texture_filtering,
        TerrainTextureFilteringArg::None
    );
    assert_eq!(args.cloud_range_chunks, 128);
    assert_eq!(args.client_entity_distance_scaling, 1.25);
    assert!(!args.fabulous_texture_filtering);
}

#[test]
fn a_stored_debug_toggle_comes_back_as_the_session_starting_value() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("advancedItemTooltips", "true");
    options.set_raw_for_test("pauseOnLostFocus", "false");
    let mut args = Args::parse_from(["bbb-client"]);
    assert!(!args.advanced_item_tooltips);
    assert!(args.pause_on_lost_focus);

    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

    // Both are what F3+H and F3+P write back, so the write is only useful
    // if the next run starts from it.
    assert!(args.advanced_item_tooltips);
    assert!(!args.pause_on_lost_focus);
}

#[test]
fn the_stored_window_family_options_feed_the_window_owner() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("guiScale", "3");
    options.set_raw_for_test("forceUnicodeFont", "true");
    options.set_raw_for_test("japaneseGlyphVariants", "true");
    options.set_raw_for_test("rawMouseInput", "false");
    options.set_raw_for_test("allowCursorChanges", "false");
    options.set_raw_for_test("fullscreenResolution", "1920x1080@144:24");
    let mut args = Args::parse_from(["bbb-client"]);
    assert!(args.client_fullscreen_resolution.is_none());
    assert_eq!(args.client_gui_scale, 0);
    assert!(!args.client_force_unicode_font);
    assert!(!args.client_japanese_glyph_variants);
    assert!(args.client_raw_mouse_input);
    assert!(args.client_allow_cursor_changes);

    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

    assert_eq!(args.client_gui_scale, 3);
    assert!(args.client_force_unicode_font);
    assert!(args.client_japanese_glyph_variants);
    assert!(!args.client_raw_mouse_input);
    assert!(!args.client_allow_cursor_changes);
    assert_eq!(
        args.client_fullscreen_resolution.map(|mode| mode.write()),
        Some("1920x1080@144:24".to_owned())
    );
}

#[test]
fn a_typed_window_family_argument_wins_over_the_stored_line() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("guiScale", "3");
    options.set_raw_for_test("rawMouseInput", "false");
    let matches = Args::command().get_matches_from(["bbb-client", "--gui-scale", "2"]);
    let mut args = Args::from_arg_matches(&matches).unwrap();

    bind_stored_client_options(&mut args, StoredOptions::new(&options, &matches));

    assert_eq!(args.client_gui_scale, 2);
    assert!(!args.client_raw_mouse_input);
}

#[test]
fn a_typed_fullscreen_resolution_wins_over_the_stored_line() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("fullscreenResolution", "1920x1080@144:24");
    let matches =
        Args::command().get_matches_from(["bbb-client", "--fullscreen-resolution", "1280x720@60"]);
    let mut args = Args::from_arg_matches(&matches).unwrap();
    assert_eq!(
        args.client_fullscreen_resolution.map(|mode| mode.write()),
        Some("1280x720@60:24".to_owned())
    );

    bind_stored_client_options(&mut args, StoredOptions::new(&options, &matches));

    assert_eq!(
        args.client_fullscreen_resolution.map(|mode| mode.write()),
        Some("1280x720@60:24".to_owned())
    );
}

#[test]
fn an_invalid_stored_fullscreen_resolution_keeps_the_current_value() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("fullscreenResolution", "not a video mode");
    let mut args = Args::parse_from(["bbb-client"]);

    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

    assert!(args.client_fullscreen_resolution.is_none());
}

#[test]
fn client_gui_scale_rejects_values_outside_the_vanilla_bound() {
    assert!(parse_client_gui_scale("0").is_ok());
    assert!(parse_client_gui_scale("4").is_ok());
    assert!(parse_client_gui_scale("2147483646").is_ok());
    assert!(parse_client_gui_scale("2147483647").is_err());
    assert!(parse_client_gui_scale("-1").is_err());
    assert!(parse_client_gui_scale("auto").is_err());
}

#[test]
fn client_fullscreen_resolution_rejects_invalid_forms_at_parse_time() {
    assert_eq!(
        parse_client_fullscreen_resolution("1920x1080@144:24")
            .unwrap()
            .write(),
        "1920x1080@144:24"
    );
    assert!(parse_client_fullscreen_resolution("").is_err());
    assert!(parse_client_fullscreen_resolution("1920x1080@").is_err());
    assert!(parse_client_fullscreen_resolution("1920x1080@144:").is_err());
}

#[test]
fn window_config_clamps_dimensions_to_minimum_nonzero() {
    let config = WindowConfig {
        width: 0,
        height: -10,
        ..WindowConfig::default()
    };
    assert_eq!(config.physical_size(), PhysicalSize::new(1, 1));

    let config = WindowConfig::default();
    assert_eq!(config.physical_size(), PhysicalSize::new(854, 480));
}

#[test]
fn vanilla_bootstrap_arguments_feed_the_single_external_startup_path() {
    let mut args = Args::parse_from([
        "bbb-client",
        "--game-dir",
        "/tmp/vanilla-game",
        "--width",
        "1280",
        "--height",
        "720",
        "--fullscreen",
        "--quick-play-multiplayer",
        "127.0.0.1:25570",
    ]);
    apply_vanilla_bootstrap_arguments(&mut args).unwrap();

    assert_eq!(args.game_dir, PathBuf::from("/tmp/vanilla-game"));
    assert_eq!(args.server, "127.0.0.1:25570");
    assert!(args.connect_server);
    assert!(args.fullscreen);
    assert_eq!(
        WindowConfig::from_args(&args).physical_size(),
        PhysicalSize::new(1280, 720)
    );
}

#[test]
fn vanilla_launcher_uuid_is_lenient_and_one_identity_feeds_login() {
    let expected = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
    for value in [
        "00112233-4455-6677-8899-aabbccddeeff",
        "00112233445566778899aabbccddeeff",
    ] {
        let args = Args::parse_from(["bbb-client", "--username", "identity-test", "--uuid", value]);
        let identity = client_identity_from_args(&args);
        assert_eq!(identity.username, "identity-test");
        assert_eq!(identity.profile_id, expected);
        let connection = connection_options_from_args(&args, &identity).unwrap();
        assert_eq!(connection.username, identity.username);
        assert_eq!(connection.profile_id, identity.profile_id);
    }

    assert_eq!(
        parse_vanilla_launcher_uuid("1-2-3-4-5"),
        Some(Uuid::from_u128(0x00000001_0002_0003_0004_000000000005))
    );
}

#[test]
fn invalid_launcher_uuid_falls_back_to_the_username_offline_identity() {
    let args = Args::parse_from([
        "bbb-client",
        "--username",
        "identity-test",
        "--uuid",
        "not-a-uuid",
    ]);
    let identity = client_identity_from_args(&args);

    assert_eq!(identity, ClientIdentity::offline("identity-test"));
    assert_eq!(parse_vanilla_launcher_uuid("not-a-uuid"), None);
    assert_eq!(parse_vanilla_launcher_uuid("_".repeat(32).as_str()), None);
}

#[test]
fn vanilla_quick_play_unescapes_java_and_cli_fullscreen_dimensions_are_fallback_only() {
    assert_eq!(unescape_java_argument(r"\uu+0041\q").unwrap(), "Aq");
    assert_eq!(unescape_java_argument(r"\uD83D\uDE00").unwrap(), "😀");
    assert!(unescape_java_argument(r"\u12").is_err());
    let mut args = Args::parse_from([
        "bbb-client",
        "--quick-play-multiplayer",
        r"example\u002eorg\07225565",
        "--fullscreen-width",
        "1920",
        "--fullscreen-height",
        "1080",
    ]);
    apply_vanilla_bootstrap_arguments(&mut args).unwrap();
    assert_eq!(args.server, "example.org:25565");
    assert_eq!(
        args.client_fullscreen_resolution.map(|mode| mode.write()),
        Some("1920x1080@60:24".to_string())
    );

    args.client_fullscreen_resolution =
        window_surface::PreferredVideoMode::parse("2560x1440@144:30");
    apply_vanilla_bootstrap_arguments(&mut args).unwrap();
    assert_eq!(
        args.client_fullscreen_resolution.map(|mode| mode.write()),
        Some("2560x1440@144:30".to_string())
    );
}

#[test]
fn stored_positive_window_override_beats_main_display_arguments_only_as_a_pair() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("overrideWidth", "1600");
    options.set_raw_for_test("overrideHeight", "900");
    let mut args = Args::parse_from(["bbb-client", "--width", "1280", "--height", "720"]);

    apply_stored_window_override(&mut args, &options);
    assert_eq!(
        WindowConfig::from_args(&args).physical_size(),
        PhysicalSize::new(1600, 900)
    );

    options.set_raw_for_test("overrideHeight", "0");
    args.window_width = 1280;
    args.window_height = 720;
    apply_stored_window_override(&mut args, &options);
    assert_eq!(
        WindowConfig::from_args(&args).physical_size(),
        PhysicalSize::new(1280, 720)
    );
}

#[test]
fn the_stored_reduced_debug_info_option_becomes_the_session_starting_value() {
    // `Minecraft.showOnlyReducedInfo` reads
    // `options.reducedDebugInfo()`, which only ever comes from
    // `options.txt` for an external client.
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("reducedDebugInfo", "true");
    let mut args = Args::parse_from(["bbb-client"]);
    assert!(!args.client_reduced_debug_info);

    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

    assert!(args.client_reduced_debug_info);
}

#[test]
fn the_stored_vignette_option_defaults_on_and_comes_back_off() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("vignette", "false");
    let mut args = Args::parse_from(["bbb-client"]);
    assert!(args.client_vignette);

    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

    assert!(!args.client_vignette);
}

#[test]
fn the_stored_screen_effect_scale_defaults_full_and_rejects_out_of_range_lines() {
    let mut args = Args::parse_from(["bbb-client"]);
    assert_eq!(args.client_screen_effect_scale, 1.0);

    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("screenEffectScale", "0.25");
    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));
    assert_eq!(args.client_screen_effect_scale, 0.25);

    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("screenEffectScale", "2.0");
    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));
    assert_eq!(args.client_screen_effect_scale, 0.25);
}

#[test]
fn the_stored_attack_indicator_option_reads_back_the_vanilla_enum_ids() {
    let mut args = Args::parse_from(["bbb-client"]);
    assert_eq!(
        args.client_attack_indicator,
        ClientAttackIndicatorArg::Crosshair
    );

    for (stored, expected) in [
        ("0", ClientAttackIndicatorArg::Off),
        ("2", ClientAttackIndicatorArg::Hotbar),
        ("1", ClientAttackIndicatorArg::Crosshair),
    ] {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_raw_for_test("attackIndicator", stored);
        let mut args = Args::parse_from(["bbb-client"]);
        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));
        assert_eq!(args.client_attack_indicator, expected);
    }

    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("attackIndicator", "9");
    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));
    assert_eq!(
        args.client_attack_indicator,
        ClientAttackIndicatorArg::Crosshair
    );
}

#[test]
fn an_out_of_range_stored_line_keeps_the_argument_value() {
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    options.set_raw_for_test("graphicsPreset", "\"custom\"");
    options.set_raw_for_test("renderDistance", "9999");
    let mut args = Args::parse_from(["bbb-client"]);
    let default_distance = args.render_distance_chunks;

    bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

    // The command line's own parser rejects it, so the value stands —
    // `FieldAccess.process` keeps the current value the same way.
    assert_eq!(args.render_distance_chunks, default_distance);
}

#[test]
fn every_binding_names_a_real_argument_and_a_catalogued_key() {
    let mut args = Args::parse_from(["bbb-client"]);
    let options = ClientOptions::defaults(ClientOptionHost::default());

    let bindings =
        bind_stored_client_options(&mut args, StoredOptions::without_command_line(&options));

    assert!(!bindings.is_empty());
    let command = Args::command();
    for binding in bindings {
        assert!(
            command
                .get_arguments()
                .any(|arg| arg.get_id() == binding.arg_id),
            "no argument named {}",
            binding.arg_id
        );
    }
}

#[test]
fn web_bootstrap_passes_server_resource_pack_policy_with_url_override() {
    let resolve_for_test = |cfg: &WebBootstrapConfig,
                            search: &str|
     -> Result<(Args, web::ParsedWebBootstrapOptions)> {
        let mut parsed = parse_web_bootstrap_from_config_and_query(cfg, search)?;
        let defaults = ClientOptions::defaults(ClientOptionHost::default());
        apply_loaded_client_options_to_args(&mut parsed.args, &defaults, |arg_id| {
            parsed.explicit_arg_ids.contains(&arg_id)
        });
        let args = std::mem::replace(&mut parsed.args, Args::default_values());
        Ok((args, parsed))
    };

    let default_cfg: WebBootstrapConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(default_cfg.core_bundle_etag, None);
    assert_eq!(default_cfg.asset_index_id, "26.1");
    let (args_default, _) = resolve_for_test(&default_cfg, "").unwrap();
    assert_eq!(args_default.server_resource_pack_policy, None);
    assert_eq!(args_default.server, "127.0.0.1:25565");
    assert!(args_default.username.is_empty());

    let prompt_cfg: WebBootstrapConfig =
        serde_json::from_str(r#"{"server_resource_pack_policy":"prompt"}"#).unwrap();
    let (args_prompt, _) = resolve_for_test(&prompt_cfg, "").unwrap();
    assert_eq!(
        args_prompt.server_resource_pack_policy,
        Some(ServerResourcePackPolicyArg::Prompt)
    );

    let decline_cfg: WebBootstrapConfig =
        serde_json::from_str(r#"{"server_resource_pack_policy":"decline"}"#).unwrap();
    let (args_decline, _) = resolve_for_test(&decline_cfg, "").unwrap();
    assert_eq!(
        args_decline.server_resource_pack_policy,
        Some(ServerResourcePackPolicyArg::Decline)
    );

    let (args_override, parsed_override) = resolve_for_test(
        &decline_cfg,
        "?server_resource_pack_policy=accept&lockstep=1&control=true&software_gpu=1",
    )
    .unwrap();
    assert!(parsed_override.lockstep);
    assert!(parsed_override.control_enabled);
    assert!(parsed_override.software_gpu);
    assert_eq!(
        args_override.server_resource_pack_policy,
        Some(ServerResourcePackPolicyArg::Accept)
    );

    let (explicit_prompt, _) =
        resolve_for_test(&decline_cfg, "?server_resource_pack_policy=prompt").unwrap();
    assert_eq!(
        explicit_prompt.server_resource_pack_policy,
        Some(ServerResourcePackPolicyArg::Prompt)
    );

    let (args_alias, _) = resolve_for_test(&default_cfg, "?resource_pack_policy=decline").unwrap();
    assert_eq!(
        args_alias.server_resource_pack_policy,
        Some(ServerResourcePackPolicyArg::Decline)
    );

    assert!(parse_web_bootstrap_from_config_and_query(
        &default_cfg,
        "?resource_pack_policy=unknown"
    )
    .is_err());

    let (args_parity, parsed_parity) = resolve_for_test(
        &default_cfg,
        "?lang=JA_JP&narrator=chat&pack=%2Fpacks%2Falpha.zip&pack=%2Fpacks%2Fbeta.zip&volume_master=0.5&render_distance=14&fov=90",
    )
    .unwrap();
    assert_eq!(
        parsed_parity.pack_urls,
        vec![
            "/packs/alpha.zip".to_string(),
            "/packs/beta.zip".to_string()
        ]
    );
    assert_eq!(
        parsed_parity.pre_parse_option_overrides,
        vec![("soundCategory_master".to_string(), "0.5".to_string())]
    );
    assert_eq!(
        parsed_parity.post_parse_option_overrides,
        vec![("narrator".to_string(), "2".to_string())]
    );
    assert_eq!(args_parity.client_locale, "ja_jp");
    assert_eq!(args_parity.client_narrator, ClientNarratorArg::Chat);
    assert_eq!(args_parity.render_distance_chunks, 14);
    assert_eq!(args_parity.client_fov, 90);

    let (args_render_ext, _) = resolve_for_test(
        &default_cfg,
        "?dynamic_lights=fancy&lod_distance=64&ext=zoom,hud_info,item_inspector",
    )
    .unwrap();
    assert_eq!(args_render_ext.dynamic_lights, DynamicLightsArg::Fancy);
    assert_eq!(args_render_ext.lod_distance_chunks, 64);
    assert_eq!(
        args_render_ext.extension_set(),
        crate::qol_extensions::ClientExtensionSet {
            zoom: true,
            hud_info: true,
            item_inspector: true,
        }
    );

    let cli_ext = Args::parse_from([
        "bbb-client",
        "--ext",
        "zoom,hud_info",
        "--ext",
        "item_inspector",
    ]);
    assert_eq!(
        cli_ext.extension_set(),
        crate::qol_extensions::ClientExtensionSet {
            zoom: true,
            hud_info: true,
            item_inspector: true,
        }
    );

    assert!(parse_web_bootstrap_from_config_and_query(&default_cfg, "?lod_distance=4").is_err());
    assert!(Args::try_parse_from(["bbb-client", "--lod-distance", "4"]).is_err());
    assert!(parse_web_bootstrap_from_config_and_query(&default_cfg, "?ext=unknown").is_err());
    assert!(Args::try_parse_from(["bbb-client", "--ext", "unknown"]).is_err());

    let (args_video_ux, _) = resolve_for_test(
        &default_cfg,
        "?graphics=fast&improved_transparency=true&clouds=fancy&vsync=false&gamma=0.75&smooth_lighting=true&particles=minimal&subtitles=true&sensitivity=0.25&bob_view=false&fov_effect_scale=0.4",
    )
    .unwrap();
    assert_eq!(args_video_ux.graphics_preset, GraphicsPresetArg::Fast);
    assert!(args_video_ux.improved_transparency);
    assert_eq!(args_video_ux.render_clouds, RenderCloudsArg::Fancy);
    assert!(!args_video_ux.client_vsync);
    assert!((args_video_ux.client_gamma - 0.75).abs() < 1e-6);
    assert!(args_video_ux.client_smooth_lighting);
    assert_eq!(
        args_video_ux.client_particles,
        ClientParticleStatusArg::Minimal
    );
    assert!(args_video_ux.client_show_subtitles);
    assert!((args_video_ux.client_mouse_sensitivity - 0.25).abs() < 1e-9);
    assert!(!args_video_ux.client_view_bobbing);
    assert!((args_video_ux.client_fov_effect_scale - 0.4).abs() < 1e-6);
    // Preset-owned fields not overridden explicitly still follow ?graphics=fast.
    assert_eq!(args_video_ux.render_distance_chunks, 8);
    assert!(!args_video_ux.entity_shadows);

    assert!(parse_web_bootstrap_from_config_and_query(&default_cfg, "?graphics=ultra").is_err());
    assert!(parse_web_bootstrap_from_config_and_query(&default_cfg, "?clouds=stormy").is_err());
    assert!(parse_web_bootstrap_from_config_and_query(&default_cfg, "?particles=none").is_err());
    assert!(parse_web_bootstrap_from_config_and_query(&default_cfg, "?vsync=maybe").is_err());
    assert!(parse_web_bootstrap_from_config_and_query(&default_cfg, "?gamma=2.0").is_err());
    assert!(parse_web_bootstrap_from_config_and_query(&default_cfg, "?sensitivity=1.5").is_err());
    assert!(
        parse_web_bootstrap_from_config_and_query(&default_cfg, "?fov_effect_scale=-0.1").is_err()
    );
}

#[test]
fn auto_discover_vanilla_root_skips_probe_server_and_finds_launcher_jar() {
    use std::io::Write;
    let tmp = std::env::temp_dir().join(format!(
        "bbb-auto-vanilla-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let version_dir = tmp.join("versions").join(bbb_protocol::MC_VERSION);
    std::fs::create_dir_all(&version_dir).unwrap();
    let jar_path = version_dir.join(format!("{}.jar", bbb_protocol::MC_VERSION));
    {
        let file = std::fs::File::create(&jar_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("version.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"{"id":"26.1"}"#).unwrap();
        zip.finish().unwrap();
    }

    let mut probe_args = Args::parse_from(["bbb-client", "--probe-server"]);
    probe_args.game_dir = tmp.clone();
    assert_eq!(auto_discover_vanilla_root(&probe_args), None);

    if std::env::var_os("BBB_VANILLA_ROOT").is_none()
        && std::env::var_os("BBB_VANILLA_DIR").is_none()
    {
        let mut client_args = Args::parse_from(["bbb-client"]);
        client_args.game_dir = tmp.clone();
        assert_eq!(auto_discover_vanilla_root(&client_args), Some(tmp.clone()));
    }

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn desktop_pack_root_check_fails_fast_with_candidates_and_advice() {
    let missing_root = std::env::temp_dir().join(format!(
        "bbb-missing-vanilla-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let mut explicit_missing = Args::parse_from([
        "bbb-client",
        "--vanilla-root",
        missing_root.to_str().unwrap(),
    ]);
    let err = load_required_desktop_pack_roots(&mut explicit_missing)
        .err()
        .expect("interactive desktop startup must reject an unusable vanilla root");
    let message = format!("{err:#}");
    assert!(
        message.contains(&missing_root.display().to_string()),
        "{message}"
    );
    assert!(
        message.contains("cargo dev vanilla prepare 26.1"),
        "{message}"
    );
    assert!(message.contains("--vanilla-root <PATH>"), "{message}");
    assert!(message.contains(".minecraft"), "{message}");
}

#[test]
fn server_flag_implies_connecting_except_status_or_probe() {
    let parse = |argv: &[&str]| {
        let matches = Args::command().get_matches_from(argv);
        parse_args_from_matches(&matches).unwrap()
    };

    let idle = parse(&["bbb-client"]);
    assert!(!idle.connect_server);

    let with_server = parse(&["bbb-client", "--server", "127.0.0.1:25565"]);
    assert!(with_server.connect_server);

    let custom_server = parse(&["bbb-client", "--server", "play.example.org:25566"]);
    assert!(custom_server.connect_server);

    let status = parse(&[
        "bbb-client",
        "--server",
        "127.0.0.1:25565",
        "--status-server",
        "127.0.0.1:25565",
    ]);
    assert!(!status.connect_server);

    let probe = parse(&[
        "bbb-client",
        "--server",
        "127.0.0.1:25565",
        "--probe-server",
    ]);
    assert!(!probe.connect_server);
}

#[test]
fn default_desktop_username_is_random_player_digits_and_explicit_wins() {
    let parse = |argv: &[&str]| {
        let matches = Args::command().get_matches_from(argv);
        parse_args_from_matches(&matches).unwrap()
    };

    let clap_default = Args::parse_from(["bbb-client"]);
    let first = parse(&["bbb-client"]);
    let second = parse(&["bbb-client"]);
    for args in [&clap_default, &first, &second] {
        assert!(args.username.starts_with("Player"), "{}", args.username);
        assert!(args.username.len() <= 16, "{}", args.username);
        assert!(
            args.username["Player".len()..]
                .bytes()
                .all(|b| b.is_ascii_digit()),
            "{}",
            args.username
        );
    }
    assert_ne!(first.username, second.username);

    let explicit = parse(&["bbb-client", "--username", "CustomTester"]);
    assert_eq!(explicit.username, "CustomTester");
}

#[test]
fn web_opt_query_parameters_validate_and_merge_options_including_keybinds() {
    let default_cfg = WebBootstrapConfig::default();
    let parsed = parse_web_bootstrap_from_config_and_query(
        &default_cfg,
        "?opt.renderDistance=12&opt.fov=0.5&opt.mainHand=left&opt.modelPart_cape=false&opt.key_key.attack=key.mouse.right&opt.soundCategory_music=0.25&opt.bobView=false",
    )
    .unwrap();
    assert!(parsed
        .pre_parse_option_overrides
        .contains(&("renderDistance".to_string(), "12".to_string())));
    assert!(parsed
        .pre_parse_option_overrides
        .contains(&("fov".to_string(), "0.5".to_string())));
    assert!(parsed
        .pre_parse_option_overrides
        .contains(&("mainHand".to_string(), "\"left\"".to_string())));
    assert!(parsed
        .pre_parse_option_overrides
        .contains(&("modelPart_cape".to_string(), "false".to_string())));
    assert!(parsed
        .pre_parse_option_overrides
        .contains(&("key_key.attack".to_string(), "key.mouse.right".to_string())));
    assert!(parsed
        .pre_parse_option_overrides
        .contains(&("soundCategory_music".to_string(), "0.25".to_string())));
    assert!(parsed
        .pre_parse_option_overrides
        .contains(&("bobView".to_string(), "false".to_string())));

    // Verify merged options flow through ClientOptions -> apply_loaded_client_options_to_args
    // including rebound key mappings.
    let mut options = ClientOptions::defaults(ClientOptionHost::default());
    for (k, v) in &parsed.pre_parse_option_overrides {
        options.set_raw_for_test(k, v);
    }
    let mut args = parsed.args;
    apply_loaded_client_options_to_args(&mut args, &options, |arg_id| {
        parsed.explicit_arg_ids.contains(&arg_id)
    });
    assert_eq!(args.render_distance_chunks, 12);
    assert_eq!(args.client_fov, 90);
    assert!(!args.client_view_bobbing);
    assert_eq!(
        args.client_key_bindings.input("key.attack"),
        crate::options::key_bindings::BoundInput::Mouse(winit::event::MouseButton::Right)
    );

    // Invalid keys, keybind values, out-of-range volumes, and control characters fail closed.
    for invalid in [
        "?opt.notAnOption=1",
        "?opt.key_key.notReal=key.keyboard.w",
        "?opt.key_key.attack=not.a.key",
        "?opt.renderDistance=abc",
        "?opt.bobView=maybe",
        "?opt.soundCategory_master=1.5",
        "?opt.lang=en_us%0Aao:false",
    ] {
        assert!(
            parse_web_bootstrap_from_config_and_query(&default_cfg, invalid).is_err(),
            "expected failure for {invalid}"
        );
    }
}
