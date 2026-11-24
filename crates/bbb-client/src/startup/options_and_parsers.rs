use super::*;

pub(crate) fn apply_loaded_client_options_to_args(
    args: &mut Args,
    options: &ClientOptions,
    explicit: impl Fn(&str) -> bool,
) {
    args.client_key_bindings = KeyBindings::from_options(options);
    crate::options::key_bindings::install_bound_key_display_keys(options);
    args.client_audio_volumes = stored_audio_volume_settings(options);
    args.narrator_voice_volume = args
        .client_audio_volumes
        .final_source_volume(&bbb_client_audio::AudioCategory::Voice);
    args.stored_resource_packs = stored_text_list(options, "resourcePacks");
    args.stored_incompatible_resource_packs =
        stored_text_list(options, "incompatibleResourcePacks");
    if let Some(raw) = options.raw("panoramaScrollSpeed") {
        match parse_client_unit_double(raw) {
            Ok(speed) => args.panorama_scroll_speed = speed,
            Err(error) => {
                tracing::warn!(
                    key = "panoramaScrollSpeed",
                    raw,
                    %error,
                    "ignoring invalid options.txt value"
                );
            }
        }
    }
    bind_stored_client_options(args, StoredOptions::with_explicit(options, explicit));
    apply_stored_window_override(args, options);
}

/// `Minecraft.<init>` applies `overrideWidth`/`overrideHeight` after Main's
/// display arguments and only when both are positive. These diagnostic launch
/// overrides therefore intentionally beat an explicit `--width/--height`,
/// unlike ordinary options mirrored by BBB startup flags.
pub(super) fn apply_stored_window_override(args: &mut Args, options: &ClientOptions) {
    let override_width = options
        .raw("overrideWidth")
        .and_then(|raw| raw.parse::<i32>().ok());
    let override_height = options
        .raw("overrideHeight")
        .and_then(|raw| raw.parse::<i32>().ok());
    if let (Some(width @ 1..), Some(height @ 1..)) = (override_width, override_height) {
        args.window_width = width;
        args.window_height = height;
    }
}

pub(super) fn stored_text_list(options: &ClientOptions, key: &'static str) -> Vec<String> {
    let Some(raw) = options.raw(key) else {
        return Vec::new();
    };
    match serde_json::from_str::<Option<Vec<String>>>(raw) {
        Ok(values) => values.unwrap_or_default(),
        Err(error) => {
            tracing::warn!(key, raw, %error, "ignoring invalid options.txt text list");
            Vec::new()
        }
    }
}

/// `soundDevice` is written through `Codec.STRING`, so unlike the raw `lang`
/// field its text is JSON quoted in `options.txt`. The empty decoded string is
/// the exact `Options.isSoundDeviceDefault` sentinel.
pub(super) fn parse_stored_sound_device(value: &str) -> std::result::Result<String, String> {
    serde_json::from_str::<String>(value)
        .map_err(|error| format!("expected a JSON string: {error}"))
}

/// `GameNarrator.narrateMessage` speaks at
/// `getFinalSoundSourceVolume(SoundSource.VOICE)` — the stored voice volume
/// times the stored master volume, each defaulting to 1.0 and clamped to the
/// accepted 0..=1 range; an unparsable line keeps its default.
pub(super) fn stored_audio_volume_settings(options: &ClientOptions) -> AudioVolumeSettings {
    let source_volume = |key: &str| {
        options
            .dynamic_raw(key)
            .and_then(|raw| raw.parse::<f32>().ok())
            .filter(|volume| volume.is_finite())
            .map(|volume| volume.clamp(0.0, 1.0))
            .unwrap_or(1.0)
    };
    AudioVolumeSettings {
        master: source_volume("soundCategory_master"),
        music: source_volume("soundCategory_music"),
        records: source_volume("soundCategory_record"),
        weather: source_volume("soundCategory_weather"),
        blocks: source_volume("soundCategory_block"),
        hostile: source_volume("soundCategory_hostile"),
        neutral: source_volume("soundCategory_neutral"),
        players: source_volume("soundCategory_player"),
        ambient: source_volume("soundCategory_ambient"),
        voice: source_volume("soundCategory_voice"),
        ui: source_volume("soundCategory_ui"),
    }
}

/// Every startup argument that mirrors an `options.txt` key. The stored line is
/// validated by the same parser the command line uses, so a bad line keeps the
/// current value instead of injecting an out-of-range one.
pub(super) fn bind_stored_client_options(
    args: &mut Args,
    mut stored: StoredOptions<'_>,
) -> Vec<ClientOptionBinding> {
    macro_rules! flag {
        ($id:literal, $key:literal, $field:ident) => {
            stored.bind(
                $id,
                $key,
                ClientOptionEncoding::Boolean,
                &mut args.$field,
                parse_stored_boolean,
            )
        };
    }
    macro_rules! parsed {
        ($id:literal, $key:literal, $encoding:ident, $field:ident, $parse:expr) => {
            stored.bind(
                $id,
                $key,
                ClientOptionEncoding::$encoding,
                &mut args.$field,
                $parse,
            )
        };
    }

    flag!("client_auto_jump", "autoJump", client_auto_jump);
    flag!("client_smooth_lighting", "ao", client_smooth_lighting);
    flag!(
        "client_reduced_debug_info",
        "reducedDebugInfo",
        client_reduced_debug_info
    );
    flag!("client_vignette", "vignette", client_vignette);
    flag!(
        "client_joined_first_server",
        "joinedFirstServer",
        client_joined_first_server
    );
    parsed!(
        "client_narrator",
        "narrator",
        EnumId,
        client_narrator,
        parse_stored_narrator
    );
    flag!(
        "client_narrator_hotkey",
        "narratorHotkey",
        client_narrator_hotkey
    );
    parsed!(
        "client_attack_indicator",
        "attackIndicator",
        EnumId,
        client_attack_indicator,
        parse_stored_attack_indicator
    );
    parsed!(
        "client_screen_effect_scale",
        "screenEffectScale",
        Double,
        client_screen_effect_scale,
        parse_client_screen_effect_scale
    );
    flag!("client_chat_colors", "chatColors", client_chat_colors);
    flag!("client_chat_links", "chatLinks", client_chat_links);
    flag!(
        "client_chat_links_prompt",
        "chatLinksPrompt",
        client_chat_links_prompt
    );
    flag!(
        "client_show_subtitles",
        "showSubtitles",
        client_show_subtitles
    );
    parsed!(
        "client_sound_device",
        "soundDevice",
        Text,
        client_sound_device,
        parse_stored_sound_device
    );
    flag!(
        "client_directional_audio",
        "directionalAudio",
        client_directional_audio
    );
    parsed!(
        "client_music_toast",
        "musicToast",
        EnumName,
        client_music_toast,
        parse_stored_music_toast
    );
    parsed!(
        "client_music_frequency",
        "musicFrequency",
        EnumName,
        client_music_frequency,
        parse_stored_music_frequency
    );
    flag!(
        "client_save_chat_drafts",
        "saveChatDrafts",
        client_save_chat_drafts
    );
    flag!(
        "client_only_show_secure_chat",
        "onlyShowSecureChat",
        client_only_show_secure_chat
    );
    flag!(
        "client_hide_matched_names",
        "hideMatchedNames",
        client_hide_matched_names
    );
    flag!(
        "client_allow_server_listing",
        "allowServerListing",
        client_allow_server_listing
    );
    flag!(
        "high_contrast_block_outline",
        "highContrastBlockOutline",
        high_contrast_block_outline
    );
    flag!("client_view_bobbing", "bobView", client_view_bobbing);
    flag!(
        "background_for_chat_only",
        "backgroundForChatOnly",
        background_for_chat_only
    );
    flag!("cutout_leaves", "cutoutLeaves", cutout_leaves);
    flag!("entity_shadows", "entityShadows", entity_shadows);
    flag!(
        "hide_lightning_flash",
        "hideLightningFlashes",
        hide_lightning_flash
    );
    flag!(
        "advanced_item_tooltips",
        "advancedItemTooltips",
        advanced_item_tooltips
    );
    flag!(
        "pause_on_lost_focus",
        "pauseOnLostFocus",
        pause_on_lost_focus
    );
    flag!(
        "improved_transparency",
        "improvedTransparency",
        improved_transparency
    );
    flag!("client_vsync", "enableVsync", client_vsync);
    flag!("fullscreen", "fullscreen", fullscreen);
    stored.bind_fullscreen_resolution(
        "client_fullscreen_resolution",
        &mut args.client_fullscreen_resolution,
        parse_client_fullscreen_resolution,
    );
    parsed!(
        "client_gui_scale",
        "guiScale",
        Integer,
        client_gui_scale,
        parse_client_gui_scale
    );
    flag!(
        "client_force_unicode_font",
        "forceUnicodeFont",
        client_force_unicode_font
    );
    flag!(
        "client_japanese_glyph_variants",
        "japaneseGlyphVariants",
        client_japanese_glyph_variants
    );
    flag!(
        "client_raw_mouse_input",
        "rawMouseInput",
        client_raw_mouse_input
    );
    flag!(
        "client_allow_cursor_changes",
        "allowCursorChanges",
        client_allow_cursor_changes
    );
    parsed!(
        "client_mouse_sensitivity",
        "mouseSensitivity",
        Double,
        client_mouse_sensitivity,
        parse_client_unit_double
    );
    flag!(
        "client_invert_mouse_x",
        "invertXMouse",
        client_invert_mouse_x
    );
    flag!(
        "client_invert_mouse_y",
        "invertYMouse",
        client_invert_mouse_y
    );
    flag!(
        "client_discrete_mouse_scroll",
        "discrete_mouse_scroll",
        client_discrete_mouse_scroll
    );
    flag!("client_toggle_crouch", "toggleCrouch", client_toggle_crouch);
    flag!("client_toggle_sprint", "toggleSprint", client_toggle_sprint);
    flag!("client_toggle_attack", "toggleAttack", client_toggle_attack);
    flag!("client_toggle_use", "toggleUse", client_toggle_use);
    parsed!(
        "client_mouse_wheel_sensitivity",
        "mouseWheelSensitivity",
        Double,
        client_mouse_wheel_sensitivity,
        parse_client_mouse_wheel_sensitivity
    );

    parsed!(
        "render_distance_chunks",
        "renderDistance",
        Integer,
        render_distance_chunks,
        parse_render_distance
    );
    parsed!(
        "graphics_preset",
        "graphicsPreset",
        EnumName,
        graphics_preset,
        parse_stored_graphics_preset
    );
    parsed!(
        "biome_blend_radius",
        "biomeBlendRadius",
        Integer,
        biome_blend_radius,
        parse_biome_blend_radius
    );
    parsed!(
        "prioritize_chunk_updates",
        "prioritizeChunkUpdates",
        EnumId,
        prioritize_chunk_updates,
        parse_stored_prioritize_chunk_updates
    );
    parsed!(
        "client_simulation_distance_chunks",
        "simulationDistance",
        Integer,
        client_simulation_distance_chunks,
        parse_client_simulation_distance
    );
    parsed!(
        "client_mipmap_levels",
        "mipmapLevels",
        Integer,
        client_mipmap_levels,
        parse_client_mipmap_levels
    );
    parsed!(
        "client_entity_distance_scaling",
        "entityDistanceScaling",
        Double,
        client_entity_distance_scaling,
        parse_client_entity_distance_scaling
    );
    parsed!(
        "client_weather_radius",
        "weatherRadius",
        Integer,
        client_weather_radius,
        parse_client_weather_radius
    );
    parsed!(
        "client_particles",
        "particles",
        EnumId,
        client_particles,
        parse_stored_particle_status
    );
    parsed!(
        "cloud_range_chunks",
        "cloudRange",
        Integer,
        cloud_range_chunks,
        parse_cloud_range
    );
    parsed!(
        "render_clouds",
        "renderClouds",
        EnumNameOrBoolean,
        render_clouds,
        parse_stored_render_clouds
    );
    parsed!(
        "terrain_texture_filtering",
        "textureFiltering",
        EnumId,
        terrain_texture_filtering,
        parse_stored_terrain_texture_filtering
    );
    parsed!(
        "client_max_anisotropy_bit",
        "maxAnisotropyBit",
        Integer,
        client_max_anisotropy_bit,
        parse_client_max_anisotropy_bit
    );
    parsed!(
        "client_framerate_limit",
        "maxFps",
        Integer,
        client_framerate_limit,
        parse_client_framerate_limit
    );
    parsed!(
        "inactivity_fps_limit",
        "inactivityFpsLimit",
        EnumName,
        inactivity_fps_limit,
        parse_stored_inactivity_fps_limit
    );
    parsed!(
        "menu_background_blurriness",
        "menuBackgroundBlurriness",
        Integer,
        menu_background_blurriness,
        parse_menu_background_blurriness
    );
    parsed!(
        "client_chat_delay_seconds",
        "chatDelay",
        Double,
        client_chat_delay_seconds,
        parse_client_chat_delay
    );
    parsed!(
        "client_chat_scale",
        "chatScale",
        Double,
        client_chat_scale,
        parse_client_unit_double
    );
    parsed!(
        "client_chat_width",
        "chatWidth",
        Double,
        client_chat_width,
        parse_client_unit_double
    );
    parsed!(
        "client_chat_height_focused",
        "chatHeightFocused",
        Double,
        client_chat_height_focused,
        parse_client_unit_double
    );
    parsed!(
        "client_chat_height_unfocused",
        "chatHeightUnfocused",
        Double,
        client_chat_height_unfocused,
        parse_client_unit_double
    );
    parsed!(
        "client_chat_line_spacing",
        "chatLineSpacing",
        Double,
        client_chat_line_spacing,
        parse_client_unit_double
    );
    parsed!(
        "client_chat_opacity",
        "chatOpacity",
        Double,
        client_chat_opacity,
        parse_client_unit_double
    );
    parsed!(
        "text_background_opacity",
        "textBackgroundOpacity",
        Double,
        text_background_opacity,
        parse_text_background_opacity
    );
    parsed!(
        "client_gamma",
        "gamma",
        Double,
        client_gamma,
        parse_client_gamma
    );
    parsed!(
        "client_fov_effect_scale",
        "fovEffectScale",
        Double,
        client_fov_effect_scale,
        parse_client_fov_effect_scale
    );
    parsed!(
        "client_glint_speed",
        "glintSpeed",
        Double,
        client_glint_speed,
        parse_client_unit_double
    );
    parsed!(
        "client_glint_strength",
        "glintStrength",
        Double,
        client_glint_strength,
        parse_client_unit_double
    );
    parsed!(
        "damage_tilt_strength",
        "damageTiltStrength",
        Double,
        damage_tilt_strength,
        parse_damage_tilt_strength
    );
    parsed!(
        "notification_display_time",
        "notificationDisplayTime",
        Double,
        notification_display_time,
        parse_notification_display_time
    );
    parsed!("client_fov", "fov", Double, client_fov, stored_fov_degrees);
    parsed!(
        "client_locale",
        "lang",
        Text,
        client_locale,
        |raw: &str| Ok(raw.to_owned())
    );

    apply_graphics_preset(args, &stored);

    stored.bound()
}

/// `Minecraft` invokes `Options.applyGraphicsPreset` after the window and
/// input owners exist. During the earlier `Options.load`, `Minecraft.running`
/// is still false, so `OptionInstance.set` stores raw values without invoking
/// callbacks; this final application therefore intentionally overwrites every
/// preset-owned stored value. BBB's startup-only CLI is an additional source,
/// and an argument the user actually typed remains the final override.
pub(super) fn apply_graphics_preset(args: &mut Args, stored: &StoredOptions<'_>) {
    macro_rules! preset_value {
        ($arg_id:literal, $field:ident, $value:expr) => {
            if !stored.is_explicit($arg_id) {
                args.$field = $value;
            }
        };
    }

    let preset = args.graphics_preset;
    if preset == GraphicsPresetArg::Custom {
        args.fabulous_texture_filtering = false;
        return;
    }

    match preset {
        GraphicsPresetArg::Fast => {
            preset_value!("biome_blend_radius", biome_blend_radius, 1);
            preset_value!("render_distance_chunks", render_distance_chunks, 8);
            preset_value!(
                "prioritize_chunk_updates",
                prioritize_chunk_updates,
                PrioritizeChunkUpdatesArg::None
            );
            preset_value!(
                "client_simulation_distance_chunks",
                client_simulation_distance_chunks,
                6
            );
            preset_value!("client_smooth_lighting", client_smooth_lighting, false);
            preset_value!("render_clouds", render_clouds, RenderCloudsArg::Fast);
            preset_value!(
                "client_particles",
                client_particles,
                ClientParticleStatusArg::Decreased
            );
            preset_value!("client_mipmap_levels", client_mipmap_levels, 2);
            preset_value!("entity_shadows", entity_shadows, false);
            preset_value!(
                "client_entity_distance_scaling",
                client_entity_distance_scaling,
                0.75
            );
            preset_value!("menu_background_blurriness", menu_background_blurriness, 2);
            preset_value!("cloud_range_chunks", cloud_range_chunks, 32);
            preset_value!("cutout_leaves", cutout_leaves, false);
            preset_value!("improved_transparency", improved_transparency, false);
            preset_value!("client_weather_radius", client_weather_radius, 5);
            preset_value!("client_max_anisotropy_bit", client_max_anisotropy_bit, 1);
            preset_value!(
                "terrain_texture_filtering",
                terrain_texture_filtering,
                TerrainTextureFilteringArg::None
            );
            args.fabulous_texture_filtering = false;
        }
        GraphicsPresetArg::Fancy => {
            preset_value!("biome_blend_radius", biome_blend_radius, 2);
            preset_value!("render_distance_chunks", render_distance_chunks, 16);
            preset_value!(
                "prioritize_chunk_updates",
                prioritize_chunk_updates,
                PrioritizeChunkUpdatesArg::PlayerAffected
            );
            preset_value!(
                "client_simulation_distance_chunks",
                client_simulation_distance_chunks,
                12
            );
            preset_value!("client_smooth_lighting", client_smooth_lighting, true);
            preset_value!("render_clouds", render_clouds, RenderCloudsArg::Fancy);
            preset_value!(
                "client_particles",
                client_particles,
                ClientParticleStatusArg::All
            );
            preset_value!("client_mipmap_levels", client_mipmap_levels, 4);
            preset_value!("entity_shadows", entity_shadows, true);
            preset_value!(
                "client_entity_distance_scaling",
                client_entity_distance_scaling,
                1.0
            );
            preset_value!("menu_background_blurriness", menu_background_blurriness, 5);
            preset_value!("cloud_range_chunks", cloud_range_chunks, 64);
            preset_value!("cutout_leaves", cutout_leaves, true);
            preset_value!("improved_transparency", improved_transparency, false);
            preset_value!("client_weather_radius", client_weather_radius, 10);
            preset_value!("client_max_anisotropy_bit", client_max_anisotropy_bit, 1);
            preset_value!(
                "terrain_texture_filtering",
                terrain_texture_filtering,
                TerrainTextureFilteringArg::Rgss
            );
            args.fabulous_texture_filtering = false;
        }
        GraphicsPresetArg::Fabulous => {
            preset_value!("biome_blend_radius", biome_blend_radius, 2);
            preset_value!("render_distance_chunks", render_distance_chunks, 32);
            preset_value!(
                "prioritize_chunk_updates",
                prioritize_chunk_updates,
                PrioritizeChunkUpdatesArg::PlayerAffected
            );
            preset_value!(
                "client_simulation_distance_chunks",
                client_simulation_distance_chunks,
                12
            );
            preset_value!("client_smooth_lighting", client_smooth_lighting, true);
            preset_value!("render_clouds", render_clouds, RenderCloudsArg::Fancy);
            preset_value!(
                "client_particles",
                client_particles,
                ClientParticleStatusArg::All
            );
            preset_value!("client_mipmap_levels", client_mipmap_levels, 4);
            preset_value!("entity_shadows", entity_shadows, true);
            preset_value!(
                "client_entity_distance_scaling",
                client_entity_distance_scaling,
                1.25
            );
            preset_value!("menu_background_blurriness", menu_background_blurriness, 5);
            preset_value!("cloud_range_chunks", cloud_range_chunks, 128);
            preset_value!("cutout_leaves", cutout_leaves, true);
            preset_value!(
                "improved_transparency",
                improved_transparency,
                super::platform::default_fabulous_improved_transparency()
            );
            preset_value!("client_weather_radius", client_weather_radius, 10);
            preset_value!("client_max_anisotropy_bit", client_max_anisotropy_bit, 2);
            preset_value!(
                "terrain_texture_filtering",
                terrain_texture_filtering,
                TerrainTextureFilteringArg::Anisotropic
            );
            args.fabulous_texture_filtering = !stored.is_explicit("terrain_texture_filtering");
        }
        GraphicsPresetArg::Custom => unreachable!("handled above"),
    }
}

pub(super) fn parse_client_gamma(value: &str) -> std::result::Result<f32, String> {
    let gamma = value
        .parse::<f32>()
        .map_err(|err| format!("client gamma must be a number: {err}"))?;
    if !gamma.is_finite() {
        return Err("client gamma must be finite".to_string());
    }
    if !(0.0..=1.0).contains(&gamma) {
        return Err("client gamma must be between 0.0 and 1.0".to_string());
    }
    Ok(gamma)
}

pub(super) fn parse_text_background_opacity(value: &str) -> std::result::Result<f32, String> {
    let opacity = value
        .parse::<f32>()
        .map_err(|err| format!("text background opacity must be a number: {err}"))?;
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err("text background opacity must be finite and between 0.0 and 1.0".to_string());
    }
    Ok(opacity)
}

pub(super) fn parse_notification_display_time(value: &str) -> std::result::Result<f32, String> {
    let multiplier = value
        .parse::<f32>()
        .map_err(|err| format!("notification display time must be a number: {err}"))?;
    if !multiplier.is_finite() || !(0.5..=10.0).contains(&multiplier) {
        return Err(
            "notification display time must be finite and between 0.5 and 10.0".to_string(),
        );
    }
    Ok(multiplier)
}

pub(super) fn parse_client_fov(value: &str) -> std::result::Result<u16, String> {
    let degrees = value
        .parse::<u16>()
        .map_err(|_| format!("client FOV must be an integer degree count: {value}"))?;
    if (VANILLA_MIN_CAMERA_FOV_DEGREES..=VANILLA_MAX_CAMERA_FOV_DEGREES).contains(&degrees) {
        Ok(degrees)
    } else {
        Err(format!(
            "client FOV must be between {VANILLA_MIN_CAMERA_FOV_DEGREES} and \
             {VANILLA_MAX_CAMERA_FOV_DEGREES} degrees"
        ))
    }
}

pub(super) fn parse_client_fov_effect_scale(value: &str) -> std::result::Result<f32, String> {
    let scale = value
        .parse::<f32>()
        .map_err(|err| format!("client FOV effect scale must be a number: {err}"))?;
    if !scale.is_finite() {
        return Err("client FOV effect scale must be finite".to_string());
    }
    if !(0.0..=1.0).contains(&scale) {
        return Err("client FOV effect scale must be between 0.0 and 1.0".to_string());
    }
    Ok(scale)
}

pub(super) fn parse_client_chat_delay(value: &str) -> std::result::Result<f64, String> {
    let delay = value
        .parse::<f64>()
        .map_err(|error| format!("invalid client chat delay: {error}"))?;
    if !delay.is_finite() || !(0.0..=6.0).contains(&delay) {
        return Err("client chat delay must be finite and between 0.0 and 6.0 seconds".to_string());
    }
    Ok(delay)
}

pub(super) fn parse_damage_tilt_strength(value: &str) -> std::result::Result<f32, String> {
    let strength = value
        .parse::<f32>()
        .map_err(|err| format!("damage tilt strength must be a number: {err}"))?;
    if !strength.is_finite() {
        return Err("damage tilt strength must be finite".to_string());
    }
    if !(0.0..=1.0).contains(&strength) {
        return Err("damage tilt strength must be between 0.0 and 1.0".to_string());
    }
    Ok(strength)
}

pub(super) fn parse_client_screen_effect_scale(value: &str) -> std::result::Result<f32, String> {
    let scale = value
        .parse::<f32>()
        .map_err(|err| format!("screen effect scale must be a number: {err}"))?;
    if scale.is_finite() && (0.0..=1.0).contains(&scale) {
        Ok(scale)
    } else {
        Err("screen effect scale must be between 0.0 and 1.0".to_string())
    }
}

pub(super) fn parse_stored_attack_indicator(
    value: &str,
) -> std::result::Result<ClientAttackIndicatorArg, String> {
    match value {
        "0" => Ok(ClientAttackIndicatorArg::Off),
        "1" => Ok(ClientAttackIndicatorArg::Crosshair),
        "2" => Ok(ClientAttackIndicatorArg::Hotbar),
        other => Err(format!("attack indicator must be 0, 1 or 2, got {other}")),
    }
}

pub(super) fn parse_stored_music_toast(
    value: &str,
) -> std::result::Result<MusicToastDisplayStateArg, String> {
    match value.trim_matches('"') {
        "never" => Ok(MusicToastDisplayStateArg::Never),
        "pause" => Ok(MusicToastDisplayStateArg::Pause),
        "pause_and_toast" => Ok(MusicToastDisplayStateArg::PauseAndToast),
        other => Err(format!(
            "music toast must be never, pause or pause_and_toast, got {other}"
        )),
    }
}

pub(super) fn parse_stored_music_frequency(
    value: &str,
) -> std::result::Result<MusicFrequencyArg, String> {
    match value.trim_matches('"') {
        "DEFAULT" => Ok(MusicFrequencyArg::Default),
        "FREQUENT" => Ok(MusicFrequencyArg::Frequent),
        "CONSTANT" => Ok(MusicFrequencyArg::Constant),
        other => Err(format!(
            "music frequency must be DEFAULT, FREQUENT or CONSTANT, got {other}"
        )),
    }
}

pub(super) fn parse_client_unit_double(value: &str) -> std::result::Result<f64, String> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| format!("expected a number, got {value}"))?;
    if !parsed.is_finite() || !(0.0..=1.0).contains(&parsed) {
        return Err(format!("expected a value between 0 and 1, got {value}"));
    }
    Ok(parsed)
}

pub(super) fn parse_client_mouse_wheel_sensitivity(
    value: &str,
) -> std::result::Result<f64, String> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| format!("expected a number, got {value}"))?;
    // `Options.mouseWheelSensitivity` is an `IntRange(-200, 100)` mapped through
    // `logMouse`, so the stored double never leaves `10^-2 ..= 10^1`.
    if !parsed.is_finite() || !(0.01..=10.0).contains(&parsed) {
        return Err(format!("expected a value between 0.01 and 10, got {value}"));
    }
    Ok(parsed)
}

/// `Options.guiScale` (Options.java:853-865): `0` (auto) through
/// `MAX_GUI_SCALE_INCLUSIVE`; `Window.calculateScale` stops the loop at the
/// cap, so the effective maximum is whatever the framebuffer can still fit at
/// 320x240.
pub(super) fn parse_client_gui_scale(value: &str) -> std::result::Result<u32, String> {
    window_surface::parse_gui_scale(value)
}

/// The stored `fullscreenResolution` line (`Options.fullscreenVideoModeString`
/// -> `VideoMode.read`); an invalid line keeps the current (unset) value,
/// matching `Optional.empty()`.
pub(super) fn parse_client_fullscreen_resolution(
    value: &str,
) -> std::result::Result<window_surface::PreferredVideoMode, String> {
    window_surface::PreferredVideoMode::parse(value)
        .ok_or_else(|| format!("invalid fullscreen resolution: {value}"))
}

pub(super) fn parse_stored_narrator(value: &str) -> std::result::Result<ClientNarratorArg, String> {
    match value {
        "0" => Ok(ClientNarratorArg::Off),
        "1" => Ok(ClientNarratorArg::All),
        "2" => Ok(ClientNarratorArg::Chat),
        "3" => Ok(ClientNarratorArg::System),
        other => Err(format!("narrator must be 0, 1, 2 or 3, got {other}")),
    }
}

pub(super) fn parse_render_distance(value: &str) -> std::result::Result<u32, String> {
    let chunks = value
        .parse::<u32>()
        .map_err(|_| format!("render distance must be an integer chunk count: {value}"))?;
    if (VANILLA_MIN_RENDER_DISTANCE_CHUNKS..=VANILLA_MAX_RENDER_DISTANCE_CHUNKS).contains(&chunks) {
        Ok(chunks)
    } else {
        Err(format!(
            "render distance must be between {VANILLA_MIN_RENDER_DISTANCE_CHUNKS} and {VANILLA_MAX_RENDER_DISTANCE_CHUNKS} chunks"
        ))
    }
}

pub(super) fn parse_dynamic_lights(value: &str) -> std::result::Result<DynamicLightsArg, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "off" | "0" | "false" => Ok(DynamicLightsArg::Off),
        "fast" | "1" => Ok(DynamicLightsArg::Fast),
        "fancy" | "2" | "true" => Ok(DynamicLightsArg::Fancy),
        other => Err(format!(
            "dynamic-lights must be off, fast, or fancy, got {other}"
        )),
    }
}

pub(super) fn parse_lod_distance(value: &str) -> std::result::Result<u32, String> {
    let chunks = value
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("lod-distance must be an integer chunk count: {value}"))?;
    if chunks == 0 || (16..=256).contains(&chunks) {
        Ok(chunks)
    } else {
        Err("lod-distance must be 0 (disabled) or between 16 and 256 chunks".to_owned())
    }
}

pub(super) fn parse_client_entity_distance_scaling(
    value: &str,
) -> std::result::Result<f64, String> {
    let scaling = value
        .parse::<f64>()
        .map_err(|_| format!("entity distance scaling must be a number: {value}"))?;
    if scaling.is_finite() && (0.5..=5.0).contains(&scaling) {
        Ok(scaling)
    } else {
        Err("entity distance scaling must be between 0.5 and 5.0".to_owned())
    }
}

pub(super) fn parse_stored_graphics_preset(
    value: &str,
) -> std::result::Result<GraphicsPresetArg, String> {
    match value.trim_matches('"') {
        "fast" => Ok(GraphicsPresetArg::Fast),
        "fancy" => Ok(GraphicsPresetArg::Fancy),
        "fabulous" => Ok(GraphicsPresetArg::Fabulous),
        "custom" => Ok(GraphicsPresetArg::Custom),
        other => Err(format!(
            "graphics preset must be fast, fancy, fabulous or custom, got {other}"
        )),
    }
}

pub(super) fn parse_stored_prioritize_chunk_updates(
    value: &str,
) -> std::result::Result<PrioritizeChunkUpdatesArg, String> {
    let id = value
        .trim_matches('"')
        .parse::<i64>()
        .map_err(|_| format!("chunk update priority must be an integer id, got {value}"))?;
    Ok(match id.rem_euclid(3) {
        0 => PrioritizeChunkUpdatesArg::None,
        1 => PrioritizeChunkUpdatesArg::PlayerAffected,
        _ => PrioritizeChunkUpdatesArg::Nearby,
    })
}

pub(super) fn parse_stored_particle_status(
    value: &str,
) -> std::result::Result<ClientParticleStatusArg, String> {
    let id = value
        .trim_matches('"')
        .parse::<i64>()
        .map_err(|_| format!("particle status must be an integer id, got {value}"))?;
    Ok(match id.rem_euclid(3) {
        0 => ClientParticleStatusArg::All,
        1 => ClientParticleStatusArg::Decreased,
        _ => ClientParticleStatusArg::Minimal,
    })
}

pub(super) fn parse_biome_blend_radius(value: &str) -> std::result::Result<u32, String> {
    parse_bounded_u32(value, 0, 7, "biome blend radius")
}

pub(super) fn parse_client_simulation_distance(value: &str) -> std::result::Result<u32, String> {
    parse_bounded_u32(value, 5, 32, "client simulation distance")
}

pub(super) fn parse_client_mipmap_levels(value: &str) -> std::result::Result<u32, String> {
    parse_bounded_u32(value, 0, 4, "mipmap levels")
}

pub(super) fn parse_client_weather_radius(value: &str) -> std::result::Result<u32, String> {
    parse_bounded_u32(value, 3, 10, "weather radius")
}

pub(super) fn parse_bounded_u32(
    value: &str,
    minimum: u32,
    maximum: u32,
    label: &str,
) -> std::result::Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("{label} must be an integer: {value}"))?;
    if (minimum..=maximum).contains(&parsed) {
        Ok(parsed)
    } else {
        Err(format!(
            "{label} must be between {minimum} and {maximum}, got {parsed}"
        ))
    }
}

pub(super) fn parse_menu_background_blurriness(value: &str) -> std::result::Result<u8, String> {
    let blurriness = value
        .parse::<u8>()
        .map_err(|_| format!("menu background blurriness must be an integer: {value}"))?;
    if blurriness <= VANILLA_MAX_MENU_BACKGROUND_BLURRINESS {
        Ok(blurriness)
    } else {
        Err(format!(
            "menu background blurriness must be between 0 and {VANILLA_MAX_MENU_BACKGROUND_BLURRINESS}"
        ))
    }
}

pub(super) fn parse_client_framerate_limit(value: &str) -> std::result::Result<u32, String> {
    if value.eq_ignore_ascii_case("inf") || value.eq_ignore_ascii_case("unlimited") {
        return Ok(VANILLA_UNLIMITED_FRAMERATE_LIMIT);
    }
    let limit = value
        .parse::<u32>()
        .map_err(|_| format!("client framerate limit must be an integer or inf: {value}"))?;
    // Vanilla `Options.framerateLimit` persists through `Codec.intRange(10,
    // 260)`: the slider only reaches multiples of ten, but any stored value in
    // range loads, and only `UNLIMITED_FRAMERATE_CUTOFF` (260) lifts the cap.
    if (10..=VANILLA_UNLIMITED_FRAMERATE_LIMIT).contains(&limit) {
        Ok(limit)
    } else {
        Err("client framerate limit must be 10..=260, inf, or unlimited".to_string())
    }
}

pub(super) fn parse_cloud_range(value: &str) -> std::result::Result<u32, String> {
    let range = value
        .parse::<u32>()
        .map_err(|_| format!("cloud range must be an integer: {value}"))?;
    if (VANILLA_MIN_CLOUD_RANGE_CHUNKS..=VANILLA_MAX_CLOUD_RANGE_CHUNKS).contains(&range) {
        Ok(range)
    } else {
        Err(format!(
            "cloud range must be between {VANILLA_MIN_CLOUD_RANGE_CHUNKS} and {VANILLA_MAX_CLOUD_RANGE_CHUNKS} chunks"
        ))
    }
}

/// `CloudStatus.CODEC` reads the legacy serialized names; the pre-enum
/// `options.txt` shape also stored bare booleans, which
/// `EnumNameOrBoolean` still loads. The friendly names stay accepted for
/// the command line.
pub(super) fn parse_stored_terrain_texture_filtering(
    value: &str,
) -> std::result::Result<TerrainTextureFilteringArg, String> {
    // Vanilla `TextureFilteringMethod.LEGACY_CODEC` reads the stored int via
    // `ByIdMap.continuous(..., OutOfBoundsStrategy.WRAP)`, so out-of-range ids
    // wrap modulo the three variants instead of failing.
    let id = value
        .trim_matches('"')
        .parse::<i64>()
        .map_err(|_| format!("texture filtering must be an integer id, got {value}"))?;
    Ok(match id.rem_euclid(3) {
        0 => TerrainTextureFilteringArg::None,
        1 => TerrainTextureFilteringArg::Rgss,
        _ => TerrainTextureFilteringArg::Anisotropic,
    })
}

pub(super) fn parse_client_max_anisotropy_bit(value: &str) -> std::result::Result<u32, String> {
    // Vanilla `maxAnisotropyBit` is `Codec.intRange(1, 3)` via
    // `OptionInstance.IntRange`; out-of-range stored values fail codec
    // validation and keep the default.
    let bit = value
        .parse::<u32>()
        .map_err(|_| format!("max anisotropy bit must be an integer: {value}"))?;
    if (1..=3).contains(&bit) {
        Ok(bit)
    } else {
        Err(format!("max anisotropy bit must be 1, 2 or 3, got {bit}"))
    }
}

pub(super) fn parse_stored_render_clouds(
    value: &str,
) -> std::result::Result<RenderCloudsArg, String> {
    match value.trim_matches('"') {
        "true" | "fancy" => Ok(RenderCloudsArg::Fancy),
        "fast" => Ok(RenderCloudsArg::Fast),
        "false" | "off" => Ok(RenderCloudsArg::Off),
        other => Err(format!(
            "render clouds must be true, fast, false, fancy or off, got {other}"
        )),
    }
}

pub(super) fn parse_stored_inactivity_fps_limit(
    value: &str,
) -> std::result::Result<InactivityFpsLimitArg, String> {
    match value.trim_matches('"') {
        "minimized" => Ok(InactivityFpsLimitArg::Minimized),
        "afk" => Ok(InactivityFpsLimitArg::Afk),
        other => Err(format!(
            "inactivity fps limit must be minimized or afk, got {other}"
        )),
    }
}
