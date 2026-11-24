use super::*;

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct WebBootstrapConfig {
    #[serde(default = "default_web_server_addr")]
    pub(crate) server: String,
    #[serde(default)]
    pub(crate) username: String,
    #[serde(default)]
    pub(crate) uuid: Option<String>,
    #[serde(default)]
    pub(crate) render_distance: Option<u32>,
    pub(crate) server_resource_pack_policy: Option<String>,
    #[serde(default)]
    pub(crate) control_enabled: bool,
    #[serde(default)]
    pub(crate) core_bundle_etag: Option<String>,
    #[serde(default = "default_web_asset_index_id")]
    pub(crate) asset_index_id: String,
    #[serde(default, alias = "locale")]
    pub(crate) lang: Option<String>,
    #[serde(default)]
    pub(crate) narrator: Option<String>,
    #[serde(default)]
    pub(crate) packs: Vec<String>,
}

impl Default for WebBootstrapConfig {
    fn default() -> Self {
        Self {
            server: default_web_server_addr(),
            username: String::new(),
            uuid: None,
            render_distance: None,
            server_resource_pack_policy: None,
            control_enabled: false,
            core_bundle_etag: None,
            asset_index_id: default_web_asset_index_id(),
            lang: None,
            narrator: None,
            packs: Vec::new(),
        }
    }
}

fn default_web_asset_index_id() -> String {
    "26.1".to_string()
}

fn default_web_server_addr() -> String {
    "127.0.0.1:25565".to_string()
}

#[derive(Debug)]
pub(crate) struct ParsedWebBootstrapOptions {
    pub(crate) args: Args,
    pub(crate) explicit_arg_ids: Vec<&'static str>,
    pub(crate) control_enabled: bool,
    pub(crate) software_gpu: bool,
    pub(crate) lockstep: bool,
    pub(crate) pack_urls: Vec<String>,
    pub(crate) pre_parse_option_overrides: Vec<(String, String)>,
    #[cfg(test)]
    pub(crate) post_parse_option_overrides: Vec<(String, String)>,
}

fn normalize_web_narrator_param(raw: &str) -> Option<(ClientNarratorArg, &'static str)> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "0" | "off" => Some((ClientNarratorArg::Off, "0")),
        "1" | "all" => Some((ClientNarratorArg::All, "1")),
        "2" | "chat" => Some((ClientNarratorArg::Chat, "2")),
        "3" | "system" => Some((ClientNarratorArg::System, "3")),
        _ => None,
    }
}

fn parse_web_bool_param(name: &str, raw: &str) -> Result<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "on" | "yes" => Ok(true),
        "false" | "0" | "off" | "no" => Ok(false),
        other => anyhow::bail!("invalid {name} '{other}' (expected true/false or 1/0)"),
    }
}

fn parse_web_clouds_param(raw: &str) -> Result<RenderCloudsArg> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "off" | "false" | "0" => Ok(RenderCloudsArg::Off),
        "fast" => Ok(RenderCloudsArg::Fast),
        "fancy" | "true" | "1" => Ok(RenderCloudsArg::Fancy),
        other => anyhow::bail!("invalid clouds '{other}' (expected off, fast, or fancy)"),
    }
}

fn parse_web_particles_param(raw: &str) -> Result<ClientParticleStatusArg> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "all" | "0" => Ok(ClientParticleStatusArg::All),
        "decreased" | "1" => Ok(ClientParticleStatusArg::Decreased),
        "minimal" | "2" => Ok(ClientParticleStatusArg::Minimal),
        other => anyhow::bail!("invalid particles '{other}' (expected all, decreased, or minimal)"),
    }
}

const MAX_WEB_OPTION_VALUE_BYTES: usize = 4096;

fn is_graphics_preset_owned_key(key: &str) -> bool {
    matches!(
        key,
        "biomeBlendRadius"
            | "renderDistance"
            | "prioritizeChunkUpdates"
            | "simulationDistance"
            | "ao"
            | "renderClouds"
            | "particles"
            | "mipmapLevels"
            | "entityShadows"
            | "entityDistanceScaling"
            | "menuBackgroundBlurriness"
            | "cloudRange"
            | "cutoutLeaves"
            | "improvedTransparency"
            | "weatherRadius"
            | "maxAnisotropyBit"
            | "textureFiltering"
    )
}

fn has_opt_key(overrides: &[(String, String)], key: &str) -> bool {
    overrides.iter().any(|(existing, _)| existing == key)
}

fn upsert_option_override(overrides: &mut Vec<(String, String)>, key: String, value: String) {
    if let Some((_, existing)) = overrides.iter_mut().find(|(k, _)| k == &key) {
        *existing = value;
    } else {
        overrides.push((key, value));
    }
}

fn normalize_quoted_enum_value(value: &str, key: &str) -> Result<String> {
    if value.starts_with('"') {
        let parsed: String = serde_json::from_str(value)
            .map_err(|err| anyhow::anyhow!("invalid quoted value for opt.{key}: {err}"))?;
        anyhow::ensure!(
            !parsed.is_empty() && !parsed.chars().any(char::is_control),
            "empty or invalid value for opt.{key}"
        );
        Ok(value.to_string())
    } else {
        anyhow::ensure!(!value.is_empty(), "empty value for opt.{key}");
        serde_json::to_string(value).map_err(|err| anyhow::anyhow!("encode opt.{key}: {err}"))
    }
}

fn validate_and_normalize_web_option_override(key: &str, value: &str) -> Result<(String, String)> {
    anyhow::ensure!(
        !key.is_empty() && !key.chars().any(char::is_control),
        "invalid option key '{key}'"
    );
    anyhow::ensure!(
        value.len() <= MAX_WEB_OPTION_VALUE_BYTES && !value.chars().any(char::is_control),
        "invalid value for opt.{key}"
    );
    let trimmed = value.trim();
    if let Some(mapping_name) = key.strip_prefix("key_") {
        anyhow::ensure!(
            crate::options::key_bindings::vanilla_key_mapping(mapping_name).is_some(),
            "unknown keybind mapping 'key_{mapping_name}' in opt.{key}"
        );
        anyhow::ensure!(
            crate::options::key_bindings::named_key(trimmed).is_some(),
            "invalid key name '{trimmed}' for opt.{key}"
        );
        return Ok((key.to_string(), trimmed.to_string()));
    }
    if let Some(category) = key.strip_prefix("soundCategory_") {
        anyhow::ensure!(
            WEB_AUDIO_VOLUME_CATEGORIES.contains(&category),
            "unknown sound category '{category}' in opt.{key}"
        );
        let parsed: f32 = trimmed
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid sound volume '{trimmed}' for opt.{key}"))?;
        anyhow::ensure!(
            parsed.is_finite() && (0.0..=1.0).contains(&parsed),
            "sound volume for opt.{key} must be between 0.0 and 1.0"
        );
        return Ok((key.to_string(), parsed.to_string()));
    }
    if let Some(part) = key.strip_prefix("modelPart_") {
        anyhow::ensure!(
            matches!(
                part,
                "cape"
                    | "jacket"
                    | "left_sleeve"
                    | "right_sleeve"
                    | "left_pants_leg"
                    | "right_pants_leg"
                    | "hat"
            ),
            "unknown model part '{part}' in opt.{key}"
        );
        parse_stored_boolean(trimmed)
            .map_err(|err| anyhow::anyhow!("invalid boolean for opt.{key}: {err}"))?;
        return Ok((key.to_string(), trimmed.to_string()));
    }
    if key == crate::options::FULLSCREEN_RESOLUTION_KEY {
        anyhow::ensure!(
            window_surface::PreferredVideoMode::parse(trimmed).is_some(),
            "invalid fullscreenResolution '{trimmed}' for opt.{key}"
        );
        return Ok((key.to_string(), trimmed.to_string()));
    }

    let spec = crate::options::client_option_spec(key)
        .ok_or_else(|| anyhow::anyhow!("unknown options.txt key '{key}' in opt.{key}"))?;
    let normalized = match spec.encoding {
        crate::options::ClientOptionEncoding::Boolean => {
            parse_stored_boolean(trimmed)
                .map_err(|err| anyhow::anyhow!("invalid boolean for opt.{key}: {err}"))?;
            trimmed.to_string()
        }
        crate::options::ClientOptionEncoding::Integer
        | crate::options::ClientOptionEncoding::EnumId => {
            trimmed
                .parse::<i32>()
                .map_err(|_| anyhow::anyhow!("expected integer for opt.{key}, got '{trimmed}'"))?;
            trimmed.to_string()
        }
        crate::options::ClientOptionEncoding::Double => {
            let parsed: f64 = trimmed
                .parse()
                .map_err(|_| anyhow::anyhow!("expected number for opt.{key}, got '{trimmed}'"))?;
            anyhow::ensure!(parsed.is_finite(), "non-finite number for opt.{key}");
            trimmed.to_string()
        }
        crate::options::ClientOptionEncoding::EnumName => {
            normalize_quoted_enum_value(trimmed, key)?
        }
        crate::options::ClientOptionEncoding::EnumNameOrBoolean => {
            if trimmed == "true" || trimmed == "false" {
                trimmed.to_string()
            } else {
                normalize_quoted_enum_value(trimmed, key)?
            }
        }
        crate::options::ClientOptionEncoding::TutorialStep => {
            let step = trimmed.trim_matches('"');
            anyhow::ensure!(
                matches!(
                    step,
                    "movement"
                        | "find_tree"
                        | "punch_tree"
                        | "open_inventory"
                        | "craft_planks"
                        | "none"
                ),
                "invalid tutorialStep '{trimmed}' for opt.{key}"
            );
            step.to_string()
        }
        crate::options::ClientOptionEncoding::Text => trimmed.to_string(),
        crate::options::ClientOptionEncoding::TextList => {
            let parsed: Vec<String> = serde_json::from_str(trimmed)
                .map_err(|err| anyhow::anyhow!("invalid JSON string list for opt.{key}: {err}"))?;
            anyhow::ensure!(
                parsed
                    .iter()
                    .all(|item| !item.chars().any(char::is_control)),
                "invalid control character in opt.{key}"
            );
            trimmed.to_string()
        }
    };
    Ok((key.to_string(), normalized))
}

const WEB_AUDIO_VOLUME_CATEGORIES: [&str; 11] = [
    "master", "music", "record", "weather", "block", "hostile", "neutral", "player", "ambient",
    "voice", "ui",
];

pub(crate) fn parse_web_bootstrap_from_config_and_query(
    config: &WebBootstrapConfig,
    search: &str,
) -> Result<ParsedWebBootstrapOptions> {
    let raw_query = search.strip_prefix('?').unwrap_or(search);
    let pairs: Vec<(String, String)> = url::form_urlencoded::parse(raw_query.as_bytes())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let get_param = |key: &str| -> Option<&str> {
        pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    let get_bool = |key: &str| -> bool {
        get_param(key).is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    };

    let mut args = Args::default_values();
    args.connect_server = true;
    args.server = get_param("server")
        .filter(|s| !s.is_empty())
        .unwrap_or(config.server.as_str())
        .to_string();
    args.username = get_param("username")
        .filter(|s| !s.is_empty())
        .unwrap_or(config.username.as_str())
        .to_string();
    args.uuid = get_param("uuid")
        .map(str::to_owned)
        .or_else(|| config.uuid.clone())
        .filter(|s| !s.is_empty());

    args.server_resource_pack_policy = get_param("server_resource_pack_policy")
        .or_else(|| get_param("resource_pack_policy"))
        .filter(|s| !s.trim().is_empty())
        .or(config.server_resource_pack_policy.as_deref())
        .filter(|s| !s.trim().is_empty())
        .map(|raw| match raw.trim().to_ascii_lowercase().as_str() {
            "prompt" => Ok(ServerResourcePackPolicyArg::Prompt),
            "accept" => Ok(ServerResourcePackPolicyArg::Accept),
            "decline" => Ok(ServerResourcePackPolicyArg::Decline),
            _ => anyhow::bail!("invalid server_resource_pack_policy '{raw}' (expected 'prompt', 'accept' or 'decline')"),
        }).transpose()?;

    let mut pre_parse_option_overrides = Vec::new();
    let mut opt_modified_preset_owned = false;
    for (param_key, param_value) in &pairs {
        if let Some(option_key) = param_key.strip_prefix("opt.") {
            let (normalized_key, normalized_value) =
                validate_and_normalize_web_option_override(option_key, param_value)?;
            opt_modified_preset_owned |= is_graphics_preset_owned_key(&normalized_key);
            upsert_option_override(
                &mut pre_parse_option_overrides,
                normalized_key,
                normalized_value,
            );
        }
    }
    if opt_modified_preset_owned && !has_opt_key(&pre_parse_option_overrides, "graphicsPreset") {
        upsert_option_override(
            &mut pre_parse_option_overrides,
            "graphicsPreset".to_string(),
            "\"custom\"".to_string(),
        );
    }
    for category in WEB_AUDIO_VOLUME_CATEGORIES {
        let dynamic_key = format!("soundCategory_{category}");
        let alias_key = format!("volume_{category}");
        if let Some(raw_vol) = get_param(&dynamic_key)
            .or_else(|| get_param(&alias_key))
            .filter(|s| !s.trim().is_empty())
        {
            if let Ok(parsed) = raw_vol.trim().parse::<f32>() {
                if parsed.is_finite() {
                    upsert_option_override(
                        &mut pre_parse_option_overrides,
                        dynamic_key,
                        format!("{}", parsed.clamp(0.0, 1.0)),
                    );
                }
            }
        }
    }

    let mut explicit_arg_ids = Vec::new();
    #[cfg(test)]
    let mut post_parse_option_overrides = Vec::new();

    let render_distance = get_param("render_distance")
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            (!has_opt_key(&pre_parse_option_overrides, "renderDistance"))
                .then(|| config.render_distance.map(|distance| distance.to_string()))
                .flatten()
        });
    if let Some(dist) = render_distance {
        args.render_distance_chunks =
            parse_render_distance(&dist).map_err(|err| anyhow::anyhow!("{err}"))?;
        explicit_arg_ids.push("render_distance_chunks");
    }

    if let Some(fov) = get_param("fov").filter(|s| !s.is_empty()) {
        args.client_fov = parse_client_fov(fov).map_err(|err| anyhow::anyhow!("{err}"))?;
        explicit_arg_ids.push("client_fov");
    }

    if let Some(gui_scale) = get_param("gui_scale").filter(|s| !s.is_empty()) {
        args.client_gui_scale =
            parse_client_gui_scale(gui_scale).map_err(|err| anyhow::anyhow!("{err}"))?;
        explicit_arg_ids.push("client_gui_scale");
    }

    if let Some(raw_lights) = get_param("dynamic_lights").filter(|s| !s.trim().is_empty()) {
        args.dynamic_lights =
            parse_dynamic_lights(raw_lights).map_err(|err| anyhow::anyhow!("{err}"))?;
        explicit_arg_ids.push("dynamic_lights");
    }

    if let Some(raw_lod) = get_param("lod_distance").filter(|s| !s.trim().is_empty()) {
        args.lod_distance_chunks =
            parse_lod_distance(raw_lod).map_err(|err| anyhow::anyhow!("{err}"))?;
        explicit_arg_ids.push("lod_distance_chunks");
    }

    if let Some(raw_graphics) = get_param("graphics").filter(|s| !s.trim().is_empty()) {
        let lower = raw_graphics.trim().to_ascii_lowercase();
        args.graphics_preset =
            parse_stored_graphics_preset(&lower).map_err(|err| anyhow::anyhow!("{err}"))?;
        explicit_arg_ids.push("graphics_preset");
    }

    if let Some(raw_it) = get_param("improved_transparency").filter(|s| !s.trim().is_empty()) {
        args.improved_transparency = parse_web_bool_param("improved_transparency", raw_it)?;
        explicit_arg_ids.push("improved_transparency");
    }

    if let Some(raw_clouds) = get_param("clouds").filter(|s| !s.trim().is_empty()) {
        args.render_clouds = parse_web_clouds_param(raw_clouds)?;
        explicit_arg_ids.push("render_clouds");
    }

    if let Some(raw_vsync) = get_param("vsync").filter(|s| !s.trim().is_empty()) {
        args.client_vsync = parse_web_bool_param("vsync", raw_vsync)?;
        explicit_arg_ids.push("client_vsync");
    }

    if let Some(raw_gamma) = get_param("gamma").filter(|s| !s.trim().is_empty()) {
        args.client_gamma =
            parse_client_gamma(raw_gamma.trim()).map_err(|err| anyhow::anyhow!("{err}"))?;
        explicit_arg_ids.push("client_gamma");
    }

    if let Some(raw_ao) = get_param("smooth_lighting").filter(|s| !s.trim().is_empty()) {
        args.client_smooth_lighting = parse_web_bool_param("smooth_lighting", raw_ao)?;
        explicit_arg_ids.push("client_smooth_lighting");
    }

    if let Some(raw_particles) = get_param("particles").filter(|s| !s.trim().is_empty()) {
        args.client_particles = parse_web_particles_param(raw_particles)?;
        explicit_arg_ids.push("client_particles");
    }

    if let Some(raw_subtitles) = get_param("subtitles").filter(|s| !s.trim().is_empty()) {
        args.client_show_subtitles = parse_web_bool_param("subtitles", raw_subtitles)?;
        explicit_arg_ids.push("client_show_subtitles");
    }

    if let Some(raw_sens) = get_param("sensitivity").filter(|s| !s.trim().is_empty()) {
        args.client_mouse_sensitivity =
            parse_client_unit_double(raw_sens.trim()).map_err(|err| anyhow::anyhow!("{err}"))?;
        explicit_arg_ids.push("client_mouse_sensitivity");
    }

    if let Some(raw_bob) = get_param("bob_view").filter(|s| !s.trim().is_empty()) {
        args.client_view_bobbing = parse_web_bool_param("bob_view", raw_bob)?;
        explicit_arg_ids.push("client_view_bobbing");
    }

    if let Some(raw_fov_scale) = get_param("fov_effect_scale").filter(|s| !s.trim().is_empty()) {
        args.client_fov_effect_scale = parse_client_fov_effect_scale(raw_fov_scale.trim())
            .map_err(|err| anyhow::anyhow!("{err}"))?;
        explicit_arg_ids.push("client_fov_effect_scale");
    }

    let raw_exts: Vec<&str> = pairs
        .iter()
        .filter(|(k, _)| k == "ext")
        .flat_map(|(_, v)| v.split(','))
        .map(str::trim)
        .filter(|slug| !slug.is_empty())
        .collect();
    if !raw_exts.is_empty() {
        args.extensions = raw_exts
            .into_iter()
            .map(|slug| {
                crate::qol_extensions::parse_client_extension_slug(slug)
                    .map_err(|err| anyhow::anyhow!("{err}"))
            })
            .collect::<Result<Vec<_>>>()?;
        explicit_arg_ids.push("extensions");
    }

    let query_locale = get_param("lang")
        .or_else(|| get_param("locale"))
        .filter(|s| !s.trim().is_empty());
    let locale = query_locale.or_else(|| {
        (!has_opt_key(&pre_parse_option_overrides, "lang"))
            .then(|| config.lang.as_deref().filter(|s| !s.trim().is_empty()))
            .flatten()
    });
    if let Some(locale) = locale {
        args.client_locale = locale.trim().to_ascii_lowercase();
        explicit_arg_ids.push("client_locale");
    }

    let query_narrator = get_param("narrator").filter(|s| !s.trim().is_empty());
    let raw_narrator = query_narrator.or_else(|| {
        (!has_opt_key(&pre_parse_option_overrides, "narrator"))
            .then(|| config.narrator.as_deref().filter(|s| !s.trim().is_empty()))
            .flatten()
    });
    if let Some(raw_narrator) = raw_narrator {
        let (narrator_arg, _stored_id) =
            normalize_web_narrator_param(raw_narrator).ok_or_else(|| {
                anyhow::anyhow!(
                    "invalid narrator '{}' (expected 'off', 'all', 'chat', 'system', or 0..=3)",
                    raw_narrator
                )
            })?;
        #[cfg(test)]
        if query_narrator.is_some() {
            post_parse_option_overrides.push(("narrator".to_string(), _stored_id.to_string()));
        }
        args.client_narrator = narrator_arg;
        explicit_arg_ids.push("client_narrator");
    }

    let mut pack_urls = config
        .packs
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    for (key, value) in &pairs {
        if key == "pack" {
            let trimmed = value.trim();
            if !trimmed.is_empty() && !pack_urls.iter().any(|existing| existing == trimmed) {
                pack_urls.push(trimmed.to_string());
            }
        }
    }

    Ok(ParsedWebBootstrapOptions {
        args,
        explicit_arg_ids,
        control_enabled: config.control_enabled || get_bool("control"),
        software_gpu: get_bool("software_gpu"),
        lockstep: get_bool("lockstep"),
        pack_urls,
        pre_parse_option_overrides,
        #[cfg(test)]
        post_parse_option_overrides,
    })
}

#[cfg(any(target_arch = "wasm32", test))]
fn build_web_args_with_options_path(
    config: &WebBootstrapConfig,
    search: &str,
    option_host: ClientOptionHost,
    options_path_override: Option<PathBuf>,
) -> Result<(
    Args,
    ParsedWebBootstrapOptions,
    Option<crate::options::ClientOptionsFile>,
)> {
    let mut parsed = parse_web_bootstrap_from_config_and_query(config, search)?;
    let options_path =
        options_path_override.unwrap_or_else(|| parsed.args.game_dir.join("options.txt"));
    let client_options =
        crate::options::ClientOptionsFile::load(options_path.clone(), option_host).ok();
    let mut loaded_options = ClientOptions::load(&options_path, option_host)
        .unwrap_or_else(|_| ClientOptions::defaults(option_host));
    if !parsed.pre_parse_option_overrides.is_empty() {
        let pre_pairs = parsed
            .pre_parse_option_overrides
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>();
        // URL query overrides apply to this tab only and must not pollute persisted storage.
        loaded_options.apply_raw_overrides(&pre_pairs);
    }
    apply_loaded_client_options_to_args(&mut parsed.args, &loaded_options, |arg_id| {
        parsed.explicit_arg_ids.contains(&arg_id)
    });
    let args = std::mem::replace(&mut parsed.args, Args::default_values());
    Ok((args, parsed, client_options))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn build_web_args_from_config_and_query(
    config: &WebBootstrapConfig,
    search: &str,
    option_host: ClientOptionHost,
) -> Result<(
    Args,
    ParsedWebBootstrapOptions,
    Option<crate::options::ClientOptionsFile>,
)> {
    build_web_args_with_options_path(config, search, option_host, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_url_overrides_apply_in_memory_without_persisting_to_options_storage() {
        let dir =
            std::env::temp_dir().join(format!("bbb-web-url-overrides-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let options_path = dir.join("options.txt");
        let _ = std::fs::remove_file(&options_path);

        let mut initial = crate::options::ClientOptionsFile::load(
            options_path.clone(),
            ClientOptionHost::default(),
        )
        .unwrap();
        initial
            .save_web_raw_overrides(&[
                ("renderDistance", "8"),
                ("soundCategory_master", "0.9"),
                ("narrator", "0"),
            ])
            .unwrap();

        let (args, _parsed, client_options) = build_web_args_with_options_path(
            &WebBootstrapConfig::default(),
            "?opt.renderDistance=14&volume_master=0.25&soundCategory_music=0.1&narrator=chat",
            ClientOptionHost::default(),
            Some(options_path.clone()),
        )
        .unwrap();

        assert_eq!(args.render_distance_chunks, 14);
        assert!((args.client_audio_volumes.master - 0.25).abs() < 1e-6);
        assert!((args.client_audio_volumes.music - 0.1).abs() < 1e-6);
        assert_eq!(args.client_narrator, ClientNarratorArg::Chat);

        // Persisted storage and the live ClientOptionsFile handle must retain original values.
        let reloaded = ClientOptions::load(&options_path, ClientOptionHost::default()).unwrap();
        assert_eq!(reloaded.raw("renderDistance"), Some("8"));
        assert_eq!(reloaded.dynamic_raw("soundCategory_master"), Some("0.9"));
        assert_eq!(reloaded.dynamic_raw("soundCategory_music"), None);
        assert_eq!(reloaded.raw("narrator"), Some("0"));
        let live_file = client_options.expect("loaded ClientOptionsFile");
        assert_eq!(live_file.options().raw("renderDistance"), Some("8"));
        assert_eq!(
            live_file.options().dynamic_raw("soundCategory_master"),
            Some("0.9")
        );
        assert_eq!(live_file.options().raw("narrator"), Some("0"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
