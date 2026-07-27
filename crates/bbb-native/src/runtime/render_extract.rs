//! World -> renderer per-frame projections.
//!
//! Pure functions that read `WorldStore` (plus camera/effect context) and
//! produce renderer-facing state: lightmap and fog environments, clear/sky
//! colors, cloud frames, weather and lightning render state. No side
//! effects; everything GPU-facing stays in bbb-renderer.

use std::sync::Arc;

use bbb_renderer::{
    BlockDestroyOverlay, DebugFilledBox, DebugFilledRect, DebugLine, DebugTextBillboard,
    DisplayTextSurface, EntityFlameSurface, EntityModelInstance, EntityNameTagSurface,
    EntityShadowSurface, FirstPersonMapBackgroundSurface, FirstPersonMapBackgroundTexture,
    FirstPersonPlayerArm, HudActionBarText, HudAdvancementToast, HudAirSupply, HudBlockItemModel,
    HudBookScreen, HudBossBar, HudBossBarColor, HudBossBarOverlay, HudChatContext,
    HudChatInteraction, HudChatMessage, HudCommandBlockEditorScreen, HudCommandSuggestions,
    HudComponentTextRun, HudDeathLinkConfirmationScreen, HudDeathMainScreen,
    HudDeathQuitConfirmationScreen, HudDeathScreen, HudDebugOptionsScreen, HudDebugOverlay,
    HudFoodEffect, HudGameEventScreen, HudGameRulesScreen, HudHotbarMode, HudInventoryScreen,
    HudJigsawBlockEditorScreen, HudMobEffect, HudPauseScreen, HudPlayerHealth, HudPlayerList,
    HudPlayerListEntry, HudPlayerListScore, HudRecipeToast, HudScoreboardSidebar,
    HudScoreboardSidebarEntry, HudSignEditorScreen, HudSleepOverlay, HudSocialInteractionsFocus,
    HudSocialInteractionsLinkConfirmationScreen, HudSocialInteractionsMainScreen,
    HudSocialInteractionsPage, HudSocialInteractionsPlayer, HudSocialInteractionsPlayerAction,
    HudSocialInteractionsScreen, HudSpectatorGui, HudSpectatorGuiSlot, HudSpectatorIcon,
    HudSpyglassScope, HudStatsScreen, HudStructureBlockEditorScreen, HudStyledTextRun,
    HudSystemToast, HudTestBlockEditorScreen, HudTestInstanceBlockEditorScreen, HudTextStyle,
    HudTitleText, HudVehicleHealth, HudVignette, ItemEntityBillboard,
    ItemFrameMapDecorationSurface, ItemFrameMapDecorationTexture, ItemFrameMapSurface,
    ItemFrameMapTextSurface, ItemFrameMapTexture, ItemModelMesh, ItemModelResidencyFrame, Renderer,
    SelectionOutline, SignTextSurface, WorldBorderRenderState,
};

use crate::component_text::{
    localized_component_runs, visit_localized_template, LocalizedTemplatePart,
};

use super::*;

#[cfg(test)]
pub(crate) fn lightmap_environment_for_world(
    world: &WorldStore,
    brightness_factor: f32,
    block_factor: f32,
) -> LightmapEnvironment {
    lightmap_environment_for_world_at_tick(
        world,
        brightness_factor,
        block_factor,
        0,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    )
}

pub(crate) fn hud_death_screen(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<HudDeathScreen> {
    let runtime = input.death_screen_runtime()?;
    let canonical = world
        .death_screen()
        .filter(|screen| screen.generation == runtime.generation())?;
    let screen = match runtime {
        DeathScreenRuntime::Main(state) => {
            let localized_cause = localized_component_runs(
                &canonical.cause_of_death,
                &bbb_protocol::ComponentStyle::default(),
                item_runtime,
            );
            let cause_link_run_indices = localized_cause
                .iter()
                .map(|run| {
                    let bbb_protocol::ComponentClickEvent::OpenUrl { .. } =
                        run.style.click_event.as_ref()?
                    else {
                        return None;
                    };
                    canonical
                        .cause_of_death
                        .styled()
                        .iter()
                        .position(|canonical_run| {
                            canonical_run.style.click_event == run.style.click_event
                        })
                        .and_then(|index| u16::try_from(index).ok())
                })
                .collect();
            HudDeathScreen::Main(HudDeathMainScreen {
                generation: state.generation,
                title: localized_gui_text(
                    item_runtime,
                    if canonical.hardcore {
                        "deathScreen.title.hardcore"
                    } else {
                        "deathScreen.title"
                    },
                    if canonical.hardcore {
                        "Game Over!"
                    } else {
                        "You Died!"
                    },
                ),
                cause_runs: localized_cause
                    .iter()
                    .map(|run| HudComponentTextRun {
                        visual: hud_chat_run(&run.text, &run.style),
                        font: run.style.font.clone(),
                    })
                    .collect(),
                cause_link_run_indices,
                score_runs: death_score_runs(canonical.score, item_runtime),
                respawn_label: localized_gui_text(
                    item_runtime,
                    if canonical.hardcore {
                        "deathScreen.spectate"
                    } else {
                        "deathScreen.respawn"
                    },
                    if canonical.hardcore {
                        "Spectate World"
                    } else {
                        "Respawn"
                    },
                ),
                title_screen_label: localized_gui_text(
                    item_runtime,
                    "deathScreen.titleScreen",
                    "Title Screen",
                ),
                buttons_active: state.buttons_active(),
                focused: state.focus,
                hovered: state.hovered,
            })
        }
        DeathScreenRuntime::QuitConfirmation(state) => {
            HudDeathScreen::QuitConfirmation(HudDeathQuitConfirmationScreen {
                generation: state.generation,
                title: localized_gui_text(
                    item_runtime,
                    "deathScreen.quit.confirm",
                    "Are you sure you want to quit?",
                ),
                title_screen_label: localized_gui_text(
                    item_runtime,
                    "deathScreen.titleScreen",
                    "Title Screen",
                ),
                respawn_label: localized_gui_text(item_runtime, "deathScreen.respawn", "Respawn"),
                buttons_active: state.buttons_active(),
                focused: state.focus,
                hovered: state.hovered,
            })
        }
        DeathScreenRuntime::LinkConfirmation(state) => {
            let confirmation = world.dialog_link_confirmation()?;
            HudDeathScreen::LinkConfirmation(HudDeathLinkConfirmationScreen {
                generation: state.generation,
                title: localized_gui_text(
                    item_runtime,
                    "chat.link.confirmTrusted",
                    "Do you want to open this link or copy it to your clipboard?",
                ),
                message: confirmation.url.clone(),
                open_label: localized_gui_text(item_runtime, "chat.link.open", "Open in Browser"),
                copy_label: localized_gui_text(item_runtime, "chat.copy", "Copy to Clipboard"),
                cancel_label: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
                focused: state.focus,
                hovered: state.hovered,
            })
        }
    };
    Some(screen)
}

pub(crate) fn hud_social_interactions_screen(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
    frame_index: u64,
    now: Option<std::time::Instant>,
) -> Option<HudSocialInteractionsScreen> {
    let canonical = world.social_interactions_screen()?;
    let runtime = input
        .social_interactions_runtime()
        .filter(|runtime| runtime.generation() == canonical.generation)?;
    if runtime.link_confirmation_is_open() {
        return Some(HudSocialInteractionsScreen::LinkConfirmation(
            HudSocialInteractionsLinkConfirmationScreen {
                generation: canonical.generation,
                title: localized_gui_text(
                    item_runtime,
                    "chat.link.confirmTrusted",
                    "Do you want to open this link or copy it to your clipboard?",
                ),
                message: crate::input::social::BLOCKING_HELP_URL.to_string(),
                open_label: localized_gui_text(item_runtime, "chat.link.open", "Open in Browser"),
                copy_label: localized_gui_text(item_runtime, "chat.copy", "Copy to Clipboard"),
                cancel_label: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
                focus: runtime.focus(),
                hovered: runtime.hovered(),
            },
        ));
    }

    let player_count = world.player_info().entries.len();
    let server_label = localized_gui_text_with_args(
        item_runtime,
        if player_count > 1 {
            "gui.socialInteractions.server_label.multiple"
        } else {
            "gui.socialInteractions.server_label.single"
        },
        if player_count > 1 {
            "%s - %s players"
        } else {
            "%s - %s player"
        },
        &[
            world.player_social().server_label.as_str(),
            &player_count.to_string(),
        ],
    );
    let local_player = world.local_player_profile_id();
    let rows = runtime
        .rows()
        .iter()
        .map(|player| {
            let action = (Some(player.profile.uuid) != local_player).then_some(if player.hidden {
                HudSocialInteractionsPlayerAction::Show
            } else {
                HudSocialInteractionsPlayerAction::Hide
            });
            HudSocialInteractionsPlayer {
                profile_id: *player.profile.uuid.as_bytes(),
                name: player.profile.name.clone(),
                skin: spectator_player_skin(&player.profile, item_runtime),
                removed: player.removed,
                hidden: player.hidden,
                blocked: player.blocked,
                action,
                status: social_player_status(player, item_runtime),
            }
        })
        .collect::<Vec<_>>();
    let page = match runtime.page() {
        bbb_world::SocialInteractionsPage::All => HudSocialInteractionsPage::All,
        bbb_world::SocialInteractionsPage::Hidden => HudSocialInteractionsPage::Hidden,
        bbb_world::SocialInteractionsPage::Blocked => HudSocialInteractionsPage::Blocked,
    };
    let empty_text = rows.is_empty().then(|| {
        if !runtime.search_text().is_empty() {
            localized_gui_text(
                item_runtime,
                "gui.socialInteractions.search_empty",
                "Couldn't find any players with that name",
            )
        } else {
            match page {
                HudSocialInteractionsPage::Hidden => localized_gui_text(
                    item_runtime,
                    "gui.socialInteractions.empty_hidden",
                    "No players hidden in chat",
                ),
                HudSocialInteractionsPage::Blocked => localized_gui_text(
                    item_runtime,
                    "gui.socialInteractions.empty_blocked",
                    "No blocked players in chat",
                ),
                HudSocialInteractionsPage::All => String::new(),
            }
        }
    });
    let tooltip = match now
        .is_some_and(|now| runtime.tooltip_is_ready(now))
        .then_some(runtime.hovered())
        .flatten()
    {
        Some(HudSocialInteractionsFocus::Player { action, .. }) => {
            cursor_position.map(|(x, y)| bbb_renderer::HudDebugOptionsTooltip {
                text: localized_gui_text(
                    item_runtime,
                    match action {
                        HudSocialInteractionsPlayerAction::Hide => {
                            "gui.socialInteractions.tooltip.hide"
                        }
                        HudSocialInteractionsPlayerAction::Show => {
                            "gui.socialInteractions.tooltip.show"
                        }
                    },
                    match action {
                        HudSocialInteractionsPlayerAction::Hide => "Hide messages",
                        HudSocialInteractionsPlayerAction::Show => "Show messages",
                    },
                ),
                x,
                y,
            })
        }
        _ => None,
    };
    Some(HudSocialInteractionsScreen::Main(
        HudSocialInteractionsMainScreen {
            generation: canonical.generation,
            title: localized_gui_text(
                item_runtime,
                "gui.socialInteractions.title",
                "Social Interactions",
            ),
            server_label,
            page,
            tab_labels: [
                localized_gui_text(item_runtime, "gui.socialInteractions.tab_all", "All"),
                localized_gui_text(item_runtime, "gui.socialInteractions.tab_hidden", "Hidden"),
                localized_gui_text(
                    item_runtime,
                    "gui.socialInteractions.tab_blocked",
                    "Blocked",
                ),
            ],
            search_text: runtime.search_text().to_string(),
            search_hint: localized_gui_text(
                item_runtime,
                "gui.socialInteractions.search_hint",
                "Search...",
            ),
            search_cursor: runtime.search_cursor(),
            search_selection: runtime.search_selection(),
            search_cursor_visible: frame_index % 12 < 6,
            rows,
            scroll_amount: runtime.scroll_amount(),
            empty_text: empty_text.filter(|text| !text.is_empty()),
            blocking_help_label: localized_gui_text(
                item_runtime,
                "gui.socialInteractions.blocking_hint",
                "Manage with Microsoft account",
            ),
            done_label: localized_gui_text(item_runtime, "gui.done", "Done"),
            focus: runtime.focus(),
            hovered: runtime.hovered(),
            tooltip,
        },
    ))
}

fn social_player_status(
    player: &bbb_world::SocialPlayerState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let (key, fallback) = match (player.blocked, player.hidden, player.removed) {
        (true, _, true) => (
            "gui.socialInteractions.status_blocked_offline",
            "Blocked - Offline",
        ),
        (false, true, true) => (
            "gui.socialInteractions.status_hidden_offline",
            "Hidden - Offline",
        ),
        (true, _, false) => ("gui.socialInteractions.status_blocked", "Blocked"),
        (false, true, false) => ("gui.socialInteractions.status_hidden", "Hidden"),
        (false, false, true) => ("gui.socialInteractions.status_offline", "Offline"),
        (false, false, false) => return None,
    };
    Some(localized_gui_text(item_runtime, key, fallback))
}

fn death_score_runs(
    score: i32,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudComponentTextRun> {
    let template = item_runtime
        .map(|runtime| runtime.localized_text("deathScreen.score.value"))
        .filter(|text| *text != "deathScreen.score.value")
        .unwrap_or("Score: %s");
    let score = score.to_string();
    let mut runs = Vec::new();
    visit_localized_template(template, |part| match part {
        LocalizedTemplatePart::Literal(text) => {
            if !text.is_empty() {
                runs.push(HudComponentTextRun::plain(text));
            }
        }
        LocalizedTemplatePart::Argument { index: 0, .. } => {
            runs.push(HudComponentTextRun {
                visual: HudStyledTextRun {
                    text: score.clone(),
                    style: HudTextStyle::default(),
                    color: Some(0xFF_FF_55),
                },
                font: None,
            });
        }
        LocalizedTemplatePart::Argument { raw, .. } => {
            runs.push(HudComponentTextRun::plain(raw));
        }
    });
    if runs.is_empty() {
        vec![
            HudComponentTextRun::plain("Score: "),
            HudComponentTextRun {
                visual: HudStyledTextRun {
                    text: score,
                    style: HudTextStyle::default(),
                    color: Some(0xFF_FF_55),
                },
                font: None,
            },
        ]
    } else {
        runs
    }
}

#[cfg(test)]
pub(crate) fn lightmap_environment_for_world_at_tick(
    world: &WorldStore,
    brightness_factor: f32,
    block_factor: f32,
    camera_tick_count: u64,
    partial_tick: f32,
) -> LightmapEnvironment {
    let darkness_effect_factor = local_player_effect(world, VANILLA_MOB_EFFECT_DARKNESS_ID)
        .map(|effect| effect.blend_factor(partial_tick))
        .unwrap_or(0.0);
    let night_vision_effect = local_player_effect(world, VANILLA_MOB_EFFECT_NIGHT_VISION_ID);
    let conduit_power_effect = local_player_effect(world, VANILLA_MOB_EFFECT_CONDUIT_POWER_ID);
    lightmap_environment_for_world_with_effects(
        world,
        brightness_factor,
        block_factor,
        camera_tick_count,
        partial_tick,
        darkness_effect_factor,
        night_vision_effect,
        conduit_power_effect,
        0.0,
        0.0,
        0.0,
        world.sky_flash_time() > 0,
    )
}

pub(crate) fn lightmap_environment_for_world_with_effects(
    world: &WorldStore,
    brightness_factor: f32,
    block_factor: f32,
    camera_tick_count: u64,
    partial_tick: f32,
    darkness_effect_factor: f32,
    night_vision_effect: Option<MobEffectState>,
    conduit_power_effect: Option<MobEffectState>,
    water_vision: f32,
    end_flash_sky_factor: f32,
    boss_overlay_world_darkening: f32,
    sky_flash_visible: bool,
) -> LightmapEnvironment {
    let mut environment = lightmap_environment_attributes_for_world(world);
    environment.sky_factor += end_flash_sky_factor;
    environment.boss_overlay_world_darkening = boss_overlay_world_darkening;
    let effects = local_player_lightmap_effects(
        brightness_factor,
        camera_tick_count,
        partial_tick,
        darkness_effect_factor,
        night_vision_effect,
        conduit_power_effect,
        water_vision,
    );
    environment.brightness_factor = effects.brightness_factor;
    environment.darkness_scale = effects.darkness_scale;
    environment.night_vision_factor = effects.night_vision_factor;
    environment.block_factor = block_factor;
    if sky_flash_visible {
        environment.sky_factor = 1.0;
    }
    environment.sanitized()
}

pub(crate) fn lightmap_environment_attributes_for_world(world: &WorldStore) -> LightmapEnvironment {
    let Some(level) = world.level_info() else {
        return LightmapEnvironment::default();
    };

    let sampled_attributes =
        sampled_environment_attributes_at_camera(world, camera_pose_from_world(world));
    let mut environment = dimension_lightmap_environment(level, sampled_attributes);
    let is_overworld =
        vanilla_lightmap_dimension_kind(level) == VanillaLightmapDimensionKind::Overworld;
    let uses_builtin_dimension_fallback = world
        .registry_content(bbb_world::DIMENSION_TYPE_REGISTRY)
        .is_none();
    if is_overworld && uses_builtin_dimension_fallback {
        let day_time = world
            .world_time()
            .map(|time| time.day_time)
            .unwrap_or(VANILLA_LIGHTMAP_DEFAULT_DAY_TIME);
        apply_overworld_timeline_lightmap_environment(&mut environment, day_time);
    }
    if level.can_have_weather() {
        apply_weather_lightmap_environment(&mut environment, world.weather());
    }
    environment
}

pub(crate) fn particle_light_for_world(world: &WorldStore, position: [f64; 3]) -> [f32; 2] {
    let light = world
        .sample_block_light(particle_light_block_pos(position))
        .unwrap_or(VANILLA_PARTICLE_MISSING_CHUNK_LIGHT);
    [
        light.block.min(15) as f32 / 15.0,
        light.sky.min(15) as f32 / 15.0,
    ]
}

#[cfg(test)]
pub(crate) fn clear_color_for_world(world: &WorldStore, hide_lightning_flash: bool) -> ClearColor {
    clear_color_for_world_with_environment_colors(
        world,
        CameraEnvironmentColors::default(),
        VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS as u32,
        hide_lightning_flash,
    )
}

#[cfg(test)]
pub(crate) fn clear_color_for_world_at_camera(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    hide_lightning_flash: bool,
) -> ClearColor {
    clear_color_for_world_at_camera_with_render_distance(
        world,
        camera_pose,
        VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS as u32,
        hide_lightning_flash,
    )
}

#[cfg(test)]
pub(crate) fn clear_color_for_world_at_camera_with_render_distance(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    render_distance_chunks: u32,
    hide_lightning_flash: bool,
) -> ClearColor {
    clear_color_for_world_with_environment_colors(
        world,
        camera_environment_colors(world, camera_pose),
        render_distance_chunks,
        hide_lightning_flash,
    )
}

pub(crate) fn clear_color_for_world_at_camera_with_water_vision(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    render_distance_chunks: u32,
    water_vision: f32,
    hide_lightning_flash: bool,
) -> ClearColor {
    clear_color_for_world_with_environment_colors_and_water_vision(
        world,
        camera_environment_colors(world, camera_pose),
        render_distance_chunks,
        water_vision,
        hide_lightning_flash,
    )
}

#[cfg(test)]
pub(crate) fn clear_color_for_world_with_environment_colors(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    render_distance_chunks: u32,
    hide_lightning_flash: bool,
) -> ClearColor {
    clear_color_for_world_with_environment_colors_and_water_vision(
        world,
        colors,
        render_distance_chunks,
        0.0,
        hide_lightning_flash,
    )
}

pub(crate) fn clear_color_for_world_with_environment_colors_and_water_vision(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    render_distance_chunks: u32,
    water_vision: f32,
    hide_lightning_flash: bool,
) -> ClearColor {
    let day_time = world.world_time().map(|time| time.day_time).unwrap_or(6000);
    let weather = world.weather();
    let can_have_weather = world
        .level_info()
        .is_none_or(WorldLevelInfo::can_have_weather);
    let rain = if can_have_weather {
        weather.rain_level.clamp(0.0, 1.0) as f64
    } else {
        0.0
    };
    let thunder = if can_have_weather {
        weather.thunder_level.clamp(0.0, 1.0) as f64
    } else {
        0.0
    };
    let dimension_kind = world
        .level_info()
        .map(vanilla_lightmap_dimension_kind)
        .unwrap_or(VanillaLightmapDimensionKind::Overworld);
    let clear = match colors.fog_type {
        CameraFogType::Atmospheric => clear_color_for_day_time_with_environment_colors_and_camera(
            day_time,
            rain,
            thunder,
            colors
                .fog_color
                .or_else(|| dimension_fog_color_for_kind(dimension_kind)),
            colors
                .sky_color
                .or_else(|| dimension_sky_color_for_kind(dimension_kind)),
            dimension_kind,
            colors.camera_forward,
            render_distance_chunks,
        ),
        CameraFogType::Water => clear_color_from_argb(rgb_u8_to_argb(
            colors
                .water_fog_color
                .unwrap_or(VANILLA_DEFAULT_WATER_FOG_COLOR),
        )),
    };
    let clear = if hide_lightning_flash
        || world.sky_flash_time() == 0
        || colors.fog_type != CameraFogType::Atmospheric
    {
        clear
    } else {
        clear_color_with_sky_flash(clear)
    };
    apply_camera_mob_effect_fog_color(
        world,
        clear,
        colors.fog_type,
        water_vision,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    )
}

fn apply_camera_mob_effect_fog_color(
    world: &WorldStore,
    clear: ClearColor,
    fog_type: CameraFogType,
    water_vision: f32,
    partial_tick: f32,
) -> ClearColor {
    let blindness = camera_entity_effect(world, VANILLA_MOB_EFFECT_BLINDNESS_ID);
    let darkness_effect = camera_entity_effect(world, VANILLA_MOB_EFFECT_DARKNESS_ID);
    let darkness = blindness.map_or_else(
        || {
            darkness_effect
                .map(|effect| effect.blend_factor(partial_tick))
                .unwrap_or(0.0)
        },
        |effect| {
            if mob_effect_ends_within(effect, 19) {
                effect.duration_ticks as f32 / 20.0
            } else {
                1.0
            }
        },
    );
    let mut color = clear_color_to_argb(clear);
    if darkness > 0.0 {
        let brightness = (1.0 - darkness.clamp(0.0, 1.0)).powi(2);
        color = argb_scale_rgb(color, brightness, brightness, brightness);
    }

    let brighten_factor = match fog_type {
        CameraFogType::Water => {
            if world.local_player().camera.follows_player {
                water_vision
            } else {
                1.0
            }
        }
        CameraFogType::Atmospheric if darkness_effect.is_none() => {
            camera_entity_effect(world, VANILLA_MOB_EFFECT_NIGHT_VISION_ID)
                .map(|effect| vanilla_night_vision_scale(effect, partial_tick))
                .unwrap_or(0.0)
        }
        CameraFogType::Atmospheric => 0.0,
    };
    clear_color_from_argb(apply_fog_brightening(color, brighten_factor))
}

pub(crate) fn sky_environment_for_world_at_camera(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    hide_lightning_flash: bool,
) -> SkyEnvironment {
    sky_environment_for_world_with_environment_colors(
        world,
        camera_environment_colors(world, camera_pose),
        hide_lightning_flash,
    )
}

pub(crate) fn sky_environment_for_world_with_environment_colors(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    hide_lightning_flash: bool,
) -> SkyEnvironment {
    // `GameRenderer.renderLevel` passes `!shouldCreateBossFog` as
    // `LevelRenderer.renderLevel`'s `shouldRenderSky` argument. Boss fog only
    // suppresses the sky pass; atmospheric fog and End-flash light remain
    // separate consumers of the same aggregate property.
    if world.boss_overlay_should_create_world_fog() || camera_mob_effect_blocks_sky(world) {
        return SkyEnvironment::disabled();
    }
    match world
        .level_info()
        .map(WorldLevelInfo::skybox)
        .unwrap_or(WorldSkybox::Overworld)
    {
        WorldSkybox::None => return SkyEnvironment::disabled(),
        WorldSkybox::End => return SkyEnvironment::end(),
        WorldSkybox::Overworld => {}
    }
    let dimension_kind = world
        .level_info()
        .map(vanilla_lightmap_dimension_kind)
        .unwrap_or(VanillaLightmapDimensionKind::Overworld);

    let day_time = world.world_time().map(|time| time.day_time).unwrap_or(6000);
    let uses_builtin_dimension_fallback = world
        .registry_content(bbb_world::DIMENSION_TYPE_REGISTRY)
        .is_none();
    let weather = world.weather();
    let can_have_weather = world
        .level_info()
        .is_none_or(WorldLevelInfo::can_have_weather);
    let rain = if can_have_weather {
        weather.rain_level.clamp(0.0, 1.0) as f64
    } else {
        0.0
    };
    let thunder = if can_have_weather {
        weather.thunder_level.clamp(0.0, 1.0) as f64
    } else {
        0.0
    };
    let sunrise_sunset_color = apply_weather_sunrise_sunset_color_layers(
        colors
            .sunrise_sunset_color
            .map(rgba_u8_to_argb)
            .unwrap_or_else(|| {
                if uses_builtin_dimension_fallback {
                    sample_periodic_argb_keyframes(
                        day_time,
                        &VANILLA_OVERWORLD_SUNRISE_SUNSET_COLOR_KEYFRAMES,
                        VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
                    )
                } else {
                    0
                }
            }),
        rain,
        thunder,
    );
    let moon_phase = colors.moon_phase.map(client_moon_phase).unwrap_or_else(|| {
        if uses_builtin_dimension_fallback {
            overworld_moon_phase(day_time)
        } else {
            SkyMoonPhase::FullMoon
        }
    });

    SkyEnvironment::from_rgb(rgb24(sky_disc_color_for_world_with_environment_colors(
        world,
        colors,
        dimension_kind,
        hide_lightning_flash,
    )))
    .with_sunrise_sunset(
        rgba32(sunrise_sunset_color),
        colors
            .sun_angle_degrees
            .unwrap_or_else(|| {
                uses_builtin_dimension_fallback
                    .then(|| overworld_sun_angle(day_time))
                    .unwrap_or(0.0)
            })
            .to_radians(),
    )
    .with_celestial_state(
        colors
            .moon_angle_degrees
            .unwrap_or_else(|| {
                uses_builtin_dimension_fallback
                    .then(|| overworld_moon_angle(day_time))
                    .unwrap_or(0.0)
            })
            .to_radians(),
        1.0 - rain as f32,
        moon_phase,
    )
    .with_star_state(
        colors
            .star_angle_degrees
            .unwrap_or_else(|| {
                uses_builtin_dimension_fallback
                    .then(|| overworld_star_angle(day_time))
                    .unwrap_or(0.0)
            })
            .to_radians(),
        apply_weather_star_brightness_layers(
            colors.star_brightness.unwrap_or_else(|| {
                if uses_builtin_dimension_fallback {
                    sample_periodic_float_keyframes(
                        day_time,
                        &VANILLA_OVERWORLD_STAR_BRIGHTNESS_KEYFRAMES,
                        VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
                    )
                } else {
                    0.0
                }
            }),
            rain as f32,
            thunder as f32,
        ),
    )
}

pub(crate) fn cloud_environment_for_world(world: &WorldStore) -> CloudEnvironment {
    let dimension_kind = world
        .level_info()
        .map(vanilla_lightmap_dimension_kind)
        .unwrap_or(VanillaLightmapDimensionKind::Overworld);
    let attributes = sampled_environment_attributes_at_camera(world, camera_pose_from_world(world));
    let uses_builtin_dimension_fallback = world
        .registry_content(bbb_world::DIMENSION_TYPE_REGISTRY)
        .is_none();
    if attributes.cloud_color.is_some()
        || uses_builtin_dimension_fallback
            && dimension_kind == VanillaLightmapDimensionKind::Overworld
    {
        let day_time = world
            .world_time()
            .map(|time| time.day_time)
            .unwrap_or(VANILLA_LIGHTMAP_DEFAULT_DAY_TIME);
        let weather = world.weather();
        let can_have_weather = world
            .level_info()
            .is_none_or(WorldLevelInfo::can_have_weather);
        let rain = can_have_weather
            .then_some(weather.rain_level as f64)
            .unwrap_or(0.0);
        let thunder = can_have_weather
            .then_some(weather.thunder_level as f64)
            .unwrap_or(0.0);
        let cloud_color = apply_weather_cloud_color_layers(
            attributes
                .cloud_color
                .map(rgba_u8_to_argb)
                .unwrap_or_else(|| {
                    argb_multiply(
                        rgba01_to_argb(VANILLA_DEFAULT_CLOUD_COLOR),
                        sample_periodic_argb_keyframes(
                            day_time,
                            &VANILLA_OVERWORLD_CLOUD_COLOR_MULTIPLIER_KEYFRAMES,
                            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
                        ),
                    )
                }),
            rain,
            thunder,
        );
        CloudEnvironment::with_color_and_height(
            rgba32(cloud_color),
            attributes
                .cloud_height
                .unwrap_or(VANILLA_DEFAULT_CLOUD_HEIGHT),
        )
    } else {
        CloudEnvironment::disabled()
    }
}

fn rgba_u8_to_argb([red, green, blue, alpha]: [u8; 4]) -> i32 {
    i32::from_be_bytes([alpha, red, green, blue])
}

fn client_moon_phase(phase: bbb_world::ClientMoonPhase) -> SkyMoonPhase {
    match phase {
        bbb_world::ClientMoonPhase::FullMoon => SkyMoonPhase::FullMoon,
        bbb_world::ClientMoonPhase::WaningGibbous => SkyMoonPhase::WaningGibbous,
        bbb_world::ClientMoonPhase::ThirdQuarter => SkyMoonPhase::ThirdQuarter,
        bbb_world::ClientMoonPhase::WaningCrescent => SkyMoonPhase::WaningCrescent,
        bbb_world::ClientMoonPhase::NewMoon => SkyMoonPhase::NewMoon,
        bbb_world::ClientMoonPhase::WaxingCrescent => SkyMoonPhase::WaxingCrescent,
        bbb_world::ClientMoonPhase::FirstQuarter => SkyMoonPhase::FirstQuarter,
        bbb_world::ClientMoonPhase::WaxingGibbous => SkyMoonPhase::WaxingGibbous,
    }
}

pub(crate) fn cloud_frame_for_world(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    partial_tick: f32,
) -> CloudFrame {
    let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
    camera_pose
        .map(|pose| CloudFrame::from_camera_pose(pose, game_time, partial_tick))
        .unwrap_or_else(|| CloudFrame::at_camera_position([0.0, 0.0, 0.0], game_time, partial_tick))
}

pub(crate) fn weather_render_state_for_world(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    partial_tick: f32,
) -> WeatherRenderState {
    let Some(camera_pose) = camera_pose else {
        return WeatherRenderState::default();
    };
    let camera_position = camera_eye_position(camera_pose);
    if !camera_position.into_iter().all(f32::is_finite) {
        return WeatherRenderState::default();
    }
    let lightning_bolts = lightning_bolts_for_world(world);
    if !world_can_have_weather(world) {
        return WeatherRenderState::with_lightning(
            WeatherFrame::default(),
            Vec::new(),
            Vec::new(),
            lightning_bolts,
        );
    }
    let rain_level = world.weather().rain_level.clamp(0.0, 1.0);
    let frame = WeatherFrame::at_camera_position(
        camera_position,
        VANILLA_WEATHER_RENDER_RADIUS,
        rain_level,
    );
    if rain_level <= 0.0 {
        return WeatherRenderState::with_lightning(frame, Vec::new(), Vec::new(), lightning_bolts);
    }
    let camera_block_x = camera_position[0].floor() as i32;
    let camera_block_y = camera_position[1].floor() as i32;
    let camera_block_z = camera_position[2].floor() as i32;
    let radius = VANILLA_WEATHER_RENDER_RADIUS as i32;
    let sea_level = world
        .level_info()
        .map(|level| level.sea_level)
        .unwrap_or(63);
    let ticks = world.world_time().map(|time| time.game_time).unwrap_or(0) as i32;
    let mut rain_columns = Vec::new();
    let mut snow_columns = Vec::new();

    for z in (camera_block_z - radius)..=(camera_block_z + radius) {
        for x in (camera_block_x - radius)..=(camera_block_x + radius) {
            let sample_pos = BlockPos {
                x,
                y: camera_block_y,
                z,
            };
            let Some(precipitation) =
                weather_precipitation_at(world, terrain_textures, sample_pos, sea_level)
            else {
                continue;
            };
            let terrain_height =
                weather_motion_blocking_height(world, x, z).unwrap_or(camera_block_y - radius);
            let bottom_y = (camera_block_y - radius).max(terrain_height);
            let top_y = (camera_block_y + radius).max(terrain_height);
            if top_y - bottom_y == 0 {
                continue;
            }

            let light_sample_y = camera_block_y.max(terrain_height);
            let raw_light = world
                .sample_block_light(BlockPos {
                    x,
                    y: light_sample_y,
                    z,
                })
                .unwrap_or(VANILLA_PARTICLE_MISSING_CHUNK_LIGHT);
            match precipitation {
                WeatherPrecipitation::Rain => rain_columns.push(rain_weather_column(
                    x,
                    z,
                    bottom_y,
                    top_y,
                    raw_light,
                    ticks,
                    partial_tick,
                )),
                WeatherPrecipitation::Snow => snow_columns.push(snow_weather_column(
                    x,
                    z,
                    bottom_y,
                    top_y,
                    raw_light,
                    ticks,
                    partial_tick,
                )),
            }
        }
    }

    WeatherRenderState::with_lightning(frame, rain_columns, snow_columns, lightning_bolts)
}

/// Projects the client world border into the renderer's world border state,
/// transcribing vanilla `WorldBorderRenderer.extract`
/// (`WorldBorderRenderer.java:102-124`) plus the render-call inputs vanilla
/// threads alongside the state (`LevelRenderer.addWeatherPass:744,751-758`,
/// `Camera.java:91-92`, `WorldBorderRenderer.render:134`).
///
/// `scroll_millis` is the wall-clock milliseconds feeding vanilla's
/// `Util.getMillis() % 3000L` forcefield UV scroll.
pub(crate) fn world_border_render_state_for_world(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    render_distance_chunks: u32,
    partial_tick: f32,
    scroll_millis: u64,
) -> WorldBorderRenderState {
    let Some(camera_pose) = camera_pose else {
        return WorldBorderRenderState::default();
    };
    let eye = camera_eye_position(camera_pose);
    if !eye.into_iter().all(f32::is_finite) {
        return WorldBorderRenderState::default();
    }
    let camera = [f64::from(eye[0]), f64::from(eye[1]), f64::from(eye[2])];
    let border = world.world_border();
    // Vanilla passes `options.getEffectiveRenderDistance() * 16` blocks to both
    // extract (LevelRenderer.java:583) and render (LevelRenderer.addWeatherPass:744).
    let render_distance = f64::from(render_distance_chunks) * 16.0;
    // Vanilla Camera.update (Camera.java:91-92):
    // `depthFar = max(renderDistanceBlocks * 4, cloudRangeChunks * 16)`; bbb has
    // no cloud-range option, so the vanilla default (128 chunks,
    // Options.java:166-171) applies.
    let depth_far =
        (render_distance * 4.0).max(f64::from(VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS) * 16.0) as f32;
    // Vanilla forcefield UV scroll offset (WorldBorderRenderer.java:134).
    let texture_offset = (scroll_millis % 3000) as f32 / 3000.0;

    // WorldBorderRenderer.extract (WorldBorderRenderer.java:102-106): the state
    // bounds use the frame partial tick.
    let min_x = border.min_x_at(partial_tick);
    let max_x = border.max_x_at(partial_tick);
    let min_z = border.min_z_at(partial_tick);
    let max_z = border.max_z_at(partial_tick);
    // Visibility condition (WorldBorderRenderer.java:107-116): the camera is
    // within renderDistance of some border edge (not strictly inside the
    // shrunk box) AND not further than renderDistance outside the border.
    let near_some_edge = !(camera[0] < max_x - render_distance)
        || !(camera[0] > min_x + render_distance)
        || !(camera[2] < max_z - render_distance)
        || !(camera[2] > min_z + render_distance);
    let inside_expanded_box = !(camera[0] < min_x - render_distance)
        && !(camera[0] > max_x + render_distance)
        && !(camera[2] < min_z - render_distance)
        && !(camera[2] > max_z + render_distance);
    let (alpha, tint) = if near_some_edge && inside_expanded_box {
        // alpha = clamp((1 - distanceToBorder / renderDistance)^4, 0, 1);
        // tint = border.getStatus().getColor()
        // (WorldBorderRenderer.java:117-120).
        let alpha = (1.0 - border.distance_to_border(camera[0], camera[2]) / render_distance)
            .powi(4)
            .clamp(0.0, 1.0);
        (alpha, border.status().color())
    } else {
        // WorldBorderRenderer.java:121-123.
        (0.0, 0)
    };

    WorldBorderRenderState {
        min_x,
        max_x,
        min_z,
        max_z,
        tint,
        alpha,
        camera_position: camera,
        render_distance,
        depth_far,
        texture_offset,
    }
}

pub(crate) fn lightning_bolts_for_world(world: &WorldStore) -> Vec<LightningBoltRenderState> {
    world
        .entity_transforms()
        .into_iter()
        .filter(|entity| entity.entity_type_id == VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID)
        .filter_map(|entity| {
            let position = [
                entity.position.x as f32,
                entity.position.y as f32,
                entity.position.z as f32,
            ];
            position
                .into_iter()
                .all(f32::is_finite)
                .then(|| LightningBoltRenderState {
                    position,
                    seed: lightning_bolt_seed(entity.uuid),
                })
        })
        .collect()
}

pub(crate) fn sky_disc_color_for_world_with_environment_colors(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    dimension_kind: VanillaLightmapDimensionKind,
    hide_lightning_flash: bool,
) -> i32 {
    let day_time = world.world_time().map(|time| time.day_time).unwrap_or(6000);
    let weather = world.weather();
    let rain = weather.rain_level.clamp(0.0, 1.0) as f64;
    let thunder = weather.thunder_level.clamp(0.0, 1.0) as f64;
    let mut sky_color = rgb_u8_to_argb(
        colors
            .sky_color
            .or_else(|| dimension_sky_color_for_kind(dimension_kind))
            .unwrap_or([0, 0, 0]),
    );

    if dimension_kind == VanillaLightmapDimensionKind::Overworld {
        sky_color = argb_multiply(
            sky_color,
            sample_periodic_argb_keyframes(
                day_time,
                &VANILLA_OVERWORLD_SKY_COLOR_MULTIPLIER_KEYFRAMES,
                VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
            ),
        );
    }
    sky_color = apply_atmospheric_sky_weather_darken(sky_color, rain, thunder);
    if !hide_lightning_flash && world.sky_flash_time() > 0 {
        sky_color = argb_srgb_lerp(
            VANILLA_SKY_FLASH_SKY_COLOR_ALPHA,
            sky_color,
            VANILLA_SKY_FLASH_SKY_COLOR,
        );
    }
    sky_color
}

pub(crate) fn fog_environment_for_world_at_camera(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    render_distance_chunks: u32,
    water_vision: f32,
    rain_fog_multiplier: f32,
    hide_lightning_flash: bool,
) -> FogEnvironment {
    fog_environment_for_world_with_environment_colors(
        world,
        camera_environment_colors(world, camera_pose),
        render_distance_chunks,
        water_vision,
        rain_fog_multiplier,
        hide_lightning_flash,
    )
}

pub(crate) fn fog_environment_for_world_with_environment_colors(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    render_distance_chunks: u32,
    water_vision: f32,
    rain_fog_multiplier: f32,
    hide_lightning_flash: bool,
) -> FogEnvironment {
    let fog_color = clear_color_for_world_with_environment_colors_and_water_vision(
        world,
        colors,
        render_distance_chunks,
        water_vision,
        hide_lightning_flash,
    );
    let color = [
        fog_color.r as f32,
        fog_color.g as f32,
        fog_color.b as f32,
        fog_color.a as f32,
    ];
    let base = match colors.fog_type {
        CameraFogType::Atmospheric => {
            let dimension_kind = world
                .level_info()
                .map(vanilla_lightmap_dimension_kind)
                .unwrap_or(VanillaLightmapDimensionKind::Overworld);
            let (mut environmental_start, mut environmental_end) =
                atmospheric_fog_distance_for_dimension(dimension_kind);
            environmental_start = colors.fog_start_distance.unwrap_or(environmental_start);
            environmental_end = colors.fog_end_distance.unwrap_or(environmental_end);
            let mut sky_end = colors
                .sky_fog_end_distance
                .unwrap_or_else(|| atmospheric_sky_fog_end(render_distance_chunks));
            let mut cloud_end = colors
                .cloud_fog_end_distance
                .unwrap_or_else(atmospheric_cloud_fog_end);
            apply_atmospheric_rain_fog_distance(
                &mut environmental_start,
                &mut environmental_end,
                rain_fog_multiplier,
            );
            if world.boss_overlay_should_create_world_fog() {
                environmental_start = environmental_start.min(VANILLA_NETHER_FOG_START_DISTANCE);
                environmental_end = environmental_end.min(VANILLA_NETHER_FOG_END_DISTANCE);
                sky_end = environmental_end;
                cloud_end = environmental_end;
            }
            FogEnvironment::world_with_visibility_ends(
                color,
                environmental_start,
                environmental_end,
                render_distance_chunks,
                sky_end,
                cloud_end,
            )
        }
        CameraFogType::Water => {
            let environmental_end = colors
                .water_fog_end_distance
                .unwrap_or(VANILLA_DEFAULT_WATER_FOG_END_DISTANCE)
                * water_vision.max(0.25);
            FogEnvironment::world_with_visibility_ends(
                color,
                colors
                    .water_fog_start_distance
                    .unwrap_or(VANILLA_DEFAULT_WATER_FOG_START_DISTANCE),
                environmental_end,
                render_distance_chunks,
                environmental_end,
                environmental_end,
            )
        }
    };
    camera_mob_effect_fog_environment(
        world,
        base,
        render_distance_chunks,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    )
}

fn camera_mob_effect_fog_environment(
    world: &WorldStore,
    base: FogEnvironment,
    render_distance_chunks: u32,
    partial_tick: f32,
) -> FogEnvironment {
    let render_distance_blocks = render_distance_chunks.clamp(
        VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
        VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
    ) as f32
        * 16.0;
    if let Some(effect) = camera_entity_effect(world, VANILLA_MOB_EFFECT_BLINDNESS_ID) {
        let distance = if effect.duration_ticks == -1 {
            5.0
        } else {
            let factor = (effect.duration_ticks as f32 / 20.0).min(1.0);
            render_distance_blocks + factor * (5.0 - render_distance_blocks)
        };
        return FogEnvironment::world_with_visibility_ends(
            base.color,
            distance * 0.25,
            distance,
            render_distance_chunks,
            distance * 0.8,
            distance * 0.8,
        );
    }
    if let Some(effect) = camera_entity_effect(world, VANILLA_MOB_EFFECT_DARKNESS_ID) {
        let factor = effect.blend_factor(partial_tick);
        let distance = render_distance_blocks + factor * (15.0 - render_distance_blocks);
        return FogEnvironment::world_with_visibility_ends(
            base.color,
            distance * 0.75,
            distance,
            render_distance_chunks,
            distance,
            distance,
        );
    }
    base
}

/// Projects the world's action-bar overlay state into the renderer's frame
/// vocabulary, mirroring `Gui.extractOverlayMessage`'s inputs (Gui.java:310):
/// present only while `overlayMessageTime > 0`. The world keeps the exact
/// Component and native resolves its localized runs against the selected
/// resource generation. The fade alpha itself is resolved by the renderer from
/// `remaining_ticks` and `partial_tick`.
pub(crate) fn hud_action_bar_text_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    partial_tick: f32,
) -> Option<HudActionBarText> {
    let action_bar = world.action_bar()?;
    if action_bar.display_ticks <= 0 {
        return None;
    }
    Some(HudActionBarText {
        runs: hud_component_text_runs(&action_bar.content, item_runtime),
        remaining_ticks: action_bar.display_ticks,
        partial_tick,
        animate_color: action_bar.animate_color,
    })
}

/// Projects Vanilla's integer `Gui.extractSleepOverlay` alpha from the
/// canonical local `Player.sleepCounter`. The GUI does not interpolate this
/// value with the frame partial tick.
pub(crate) fn hud_sleep_overlay_from_world(world: &WorldStore) -> Option<HudSleepOverlay> {
    hud_sleep_overlay_from_counter(world.local_player_sleep_counter())
}

pub(crate) fn hud_sleep_overlay_from_counter(counter: i32) -> Option<HudSleepOverlay> {
    if counter <= 0 {
        return None;
    }
    let sleep_timer = counter as f32;
    let mut amount = sleep_timer / 100.0;
    if amount > 1.0 {
        amount = 1.0 - (sleep_timer - 100.0) / 10.0;
    }
    let alpha = (220.0 * amount) as u8;
    (alpha > 0).then_some(HudSleepOverlay { alpha })
}

/// Projects the world's title/subtitle overlay state, mirroring
/// `Gui.extractTitle`'s inputs (Gui.java:339): present only while a title is
/// set and `titleTime > 0`. The subtitle rides along only while a title is
/// active (vanilla draws it inside the title branch); fade windows pass
/// through so the renderer computes the vanilla alpha ramp per frame.
pub(crate) fn hud_title_text_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    partial_tick: f32,
) -> Option<HudTitleText> {
    let title = world.title();
    let title_text = title.title.as_ref()?;
    if title.title_time <= 0 {
        return None;
    }
    Some(HudTitleText {
        title_runs: hud_component_text_runs(title_text, item_runtime),
        subtitle_runs: title
            .subtitle
            .as_ref()
            .map(|subtitle| hud_component_text_runs(subtitle, item_runtime))
            .unwrap_or_default(),
        remaining_ticks: title.title_time,
        fade_in: title.fade_in,
        stay: title.stay,
        fade_out: title.fade_out,
        partial_tick,
    })
}

fn hud_component_text_runs(
    component: &bbb_protocol::RichTextComponent,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudComponentTextRun> {
    localized_component_runs(
        component,
        &bbb_protocol::ComponentStyle::default(),
        item_runtime,
    )
    .into_iter()
    .map(|run| HudComponentTextRun {
        visual: hud_chat_run(&run.text, &run.style),
        font: run.style.font,
    })
    .collect()
}

pub(crate) fn hud_chat_messages_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudChatMessage> {
    world
        .client_chat()
        .messages
        .iter()
        .map(|message| hud_chat_message(message, item_runtime))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectedChatRun {
    visual: HudStyledTextRun,
    click_event: Option<bbb_protocol::ComponentClickEvent>,
    hover_event: Option<bbb_protocol::ComponentHoverEvent>,
    font: Option<String>,
}

fn hud_chat_message(
    message: &bbb_world::ChatMessageState,
    item_runtime: Option<&NativeItemRuntime>,
) -> HudChatMessage {
    let projected = hud_chat_message_runs(message, item_runtime);
    let mut char_offset = 0usize;
    let mut interactions = Vec::new();
    let mut runs = Vec::with_capacity(projected.len());
    for (target_index, run) in projected.into_iter().enumerate() {
        let start_char = char_offset;
        char_offset = char_offset.saturating_add(run.visual.text.chars().count());
        if run.click_event.is_some() || run.hover_event.is_some() {
            if let Ok(target_index) = u16::try_from(target_index) {
                interactions.push(HudChatInteraction {
                    start_char,
                    end_char: char_offset,
                    target_index,
                    tooltip_runs: hud_chat_hover_runs(run.hover_event.as_ref()),
                });
            }
        }
        runs.push(run.visual);
    }
    HudChatMessage {
        display_sequence: message.display_sequence,
        added_tick: message.added_tick,
        runs,
        interactions,
        tag: message
            .tag
            .as_ref()
            .map(|tag| hud_chat_tag(tag, item_runtime)),
    }
}

fn hud_chat_message_runs(
    message: &bbb_world::ChatMessageState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<ProjectedChatRun> {
    if let Some(decoration) = &message.decoration {
        let template = localized_chat_text(
            item_runtime,
            &decoration.translation_key,
            chat_translation_fallback(&decoration.translation_key)
                .unwrap_or(&decoration.translation_key),
        );
        let parent = &decoration.style;
        let arguments = decoration
            .arguments
            .iter()
            .map(|argument| {
                if argument.styled.is_empty() {
                    vec![projected_chat_run(&argument.summary, parent)]
                } else {
                    argument
                        .styled
                        .iter()
                        .map(|run| {
                            let style = run.style.apply_to(parent);
                            projected_chat_run(&run.text, &style)
                        })
                        .collect()
                }
            })
            .collect::<Vec<_>>();
        return format_localized_projected_chat_runs(template, parent, &arguments);
    }

    if let Some(component) = message.content_component.clone().and_then(|component| {
        bbb_protocol::RichTextComponent::try_from_component_nbt(component).ok()
    }) {
        return localized_component_runs(
            &component,
            &bbb_protocol::ComponentStyle::default(),
            item_runtime,
        )
        .into_iter()
        .map(|run| projected_chat_run(&run.text, &run.style))
        .collect();
    }

    if let Some(key) = message.content_translation_key.as_deref() {
        let text = localized_chat_text(
            item_runtime,
            key,
            chat_translation_fallback(key).unwrap_or(key),
        );
        let style = message
            .styled_content
            .first()
            .map(|run| run.style.clone())
            .unwrap_or_default();
        return vec![projected_chat_run(text, &style)];
    }
    if message.styled_content.is_empty() {
        vec![ProjectedChatRun {
            visual: HudStyledTextRun::plain(message.content.clone()),
            click_event: None,
            hover_event: None,
            font: None,
        }]
    } else {
        message
            .styled_content
            .iter()
            .map(|run| projected_chat_run(&run.text, &run.style))
            .collect()
    }
}

fn projected_chat_run(text: &str, style: &bbb_protocol::ComponentStyle) -> ProjectedChatRun {
    let mut visual = hud_chat_run(text, style);
    visual.text.retain(|ch| ch == '\n' || !ch.is_control());
    ProjectedChatRun {
        visual,
        click_event: style.click_event.clone(),
        hover_event: style.hover_event.clone(),
        font: style.font.clone(),
    }
}

fn hud_chat_hover_runs(hover: Option<&bbb_protocol::ComponentHoverEvent>) -> Vec<HudStyledTextRun> {
    use bbb_protocol::ComponentHoverEvent;
    match hover {
        Some(ComponentHoverEvent::ShowText { value }) => value
            .iter()
            .map(|run| hud_chat_run(&run.text, &run.style))
            .collect(),
        Some(ComponentHoverEvent::ShowItem { id, count, .. }) => {
            vec![HudStyledTextRun::plain(if *count == 1 {
                id.clone()
            } else {
                format!("{id} x{count}")
            })]
        }
        Some(ComponentHoverEvent::ShowEntity { id, name, .. }) => name
            .as_ref()
            .filter(|name| !name.is_empty())
            .map(|name| {
                name.iter()
                    .map(|run| hud_chat_run(&run.text, &run.style))
                    .collect()
            })
            .unwrap_or_else(|| vec![HudStyledTextRun::plain(id)]),
        None => Vec::new(),
    }
}

pub(crate) fn hud_chat_component_click_event(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    display_sequence: u64,
    target_index: u16,
) -> Option<bbb_protocol::ComponentClickEvent> {
    let message = world
        .client_chat()
        .messages
        .iter()
        .find(|message| message.display_sequence == display_sequence)?;
    hud_chat_message_runs(message, item_runtime)
        .get(usize::from(target_index))?
        .click_event
        .clone()
}

pub(crate) fn chat_narration_text(
    narration: &bbb_world::ChatNarrationRequest,
    item_runtime: Option<&NativeItemRuntime>,
) -> String {
    if let Some(component) = narration.content_component.clone().and_then(|component| {
        bbb_protocol::RichTextComponent::try_from_component_nbt(component).ok()
    }) {
        return localized_component_runs(
            &component,
            &bbb_protocol::ComponentStyle::default(),
            item_runtime,
        )
        .into_iter()
        .map(|run| run.text)
        .collect();
    }
    if let Some(key) = narration.translation_key.as_deref() {
        let template = localized_chat_text(
            item_runtime,
            key,
            chat_translation_fallback(key).unwrap_or(key),
        );
        let arguments = narration
            .arguments
            .iter()
            .map(|argument| {
                if argument.styled.is_empty() {
                    vec![projected_chat_run(
                        &argument.summary,
                        &bbb_protocol::ComponentStyle::default(),
                    )]
                } else {
                    argument
                        .styled
                        .iter()
                        .map(|run| projected_chat_run(&run.text, &run.style))
                        .collect()
                }
            })
            .collect::<Vec<_>>();
        return format_localized_projected_chat_runs(
            template,
            &bbb_protocol::ComponentStyle::default(),
            &arguments,
        )
        .into_iter()
        .map(|run| run.visual.text)
        .collect();
    }
    if narration.styled_content.is_empty() {
        narration.content.clone()
    } else {
        narration
            .styled_content
            .iter()
            .map(|run| run.text.as_str())
            .collect()
    }
}

fn format_localized_projected_chat_runs(
    template: &str,
    style: &bbb_protocol::ComponentStyle,
    arguments: &[Vec<ProjectedChatRun>],
) -> Vec<ProjectedChatRun> {
    let mut runs = Vec::new();
    visit_localized_template(template, |part| match part {
        LocalizedTemplatePart::Literal(text) => {
            push_projected_chat_literal(&mut runs, text, style);
        }
        LocalizedTemplatePart::Argument { index, raw } => {
            if let Some(argument) = arguments.get(index) {
                runs.extend(argument.iter().cloned());
            } else {
                push_projected_chat_literal(&mut runs, raw, style);
            }
        }
    });
    if runs.is_empty() {
        vec![projected_chat_run(template, style)]
    } else {
        runs
    }
}

fn push_projected_chat_literal(
    runs: &mut Vec<ProjectedChatRun>,
    text: &str,
    style: &bbb_protocol::ComponentStyle,
) {
    if text.is_empty() {
        return;
    }
    let run = projected_chat_run(text, style);
    if let Some(previous) = runs.last_mut().filter(|previous| {
        previous.visual.style == run.visual.style
            && previous.visual.color == run.visual.color
            && previous.click_event == run.click_event
            && previous.hover_event == run.hover_event
            && previous.font == run.font
    }) {
        previous.visual.text.push_str(text);
    } else {
        runs.push(run);
    }
}

fn hud_chat_run(text: &str, style: &bbb_protocol::ComponentStyle) -> HudStyledTextRun {
    HudStyledTextRun {
        text: text.to_string(),
        style: HudTextStyle {
            bold: style.bold == Some(true),
            italic: style.italic == Some(true),
            underlined: style.underlined == Some(true),
            strikethrough: style.strikethrough == Some(true),
            obfuscated: style.obfuscated == Some(true),
        },
        color: style.color,
    }
}

fn hud_chat_tag(
    tag: &bbb_world::ChatMessageTagState,
    item_runtime: Option<&NativeItemRuntime>,
) -> HudChatTag {
    use bbb_world::ChatMessageTagState;
    let (kind, indicator_color) = match tag {
        ChatMessageTagState::System => (HudChatTagKind::System, 13_684_944),
        ChatMessageTagState::SystemSinglePlayer => (HudChatTagKind::SystemSinglePlayer, 13_684_944),
        ChatMessageTagState::NotSecure => (HudChatTagKind::NotSecure, 13_684_944),
        ChatMessageTagState::Modified { .. } => (HudChatTagKind::Modified, 6_316_128),
        ChatMessageTagState::Error => (HudChatTagKind::Error, 16_733_525),
    };
    let (translation_key, fallback) = match tag {
        ChatMessageTagState::System => ("chat.tag.system", "Server message. Cannot be reported."),
        ChatMessageTagState::SystemSinglePlayer => {
            ("chat.tag.system_single_player", "Server message.")
        }
        ChatMessageTagState::NotSecure => (
            "chat.tag.not_secure",
            "Unverified message. Cannot be reported.",
        ),
        ChatMessageTagState::Modified { .. } => (
            "chat.tag.modified",
            "Message modified by the server. Original:",
        ),
        ChatMessageTagState::Error => ("chat.tag.error", "Server sent invalid message."),
    };
    let mut tooltip_runs = vec![HudStyledTextRun::plain(localized_chat_text(
        item_runtime,
        translation_key,
        fallback,
    ))];
    if let ChatMessageTagState::Modified { original_content } = tag {
        tooltip_runs.push(HudStyledTextRun {
            text: format!("\n{original_content}"),
            style: HudTextStyle::default(),
            color: Some(0xAA_AA_AA),
        });
    }
    HudChatTag {
        kind,
        indicator_color,
        tooltip_runs,
    }
}

fn localized_chat_text<'a>(
    item_runtime: Option<&'a NativeItemRuntime>,
    key: &'a str,
    fallback: &'a str,
) -> &'a str {
    item_runtime
        .map(|runtime| runtime.localized_text(key))
        .filter(|text| *text != key)
        .unwrap_or(fallback)
}

fn chat_translation_fallback(key: &str) -> Option<&'static str> {
    match key {
        "chat.type.text" => Some("<%s> %s"),
        "chat.type.announcement" => Some("[%s] %s"),
        "chat.type.emote" => Some("* %s %s"),
        "chat.deleted_marker" => Some("This chat message has been deleted by the server."),
        "chat.queue" => Some("%s messages waiting"),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn format_localized_chat_runs(
    template: &str,
    style: &bbb_protocol::ComponentStyle,
    arguments: &[Vec<HudStyledTextRun>],
) -> Vec<HudStyledTextRun> {
    let mut runs = Vec::new();
    visit_localized_template(template, |part| match part {
        LocalizedTemplatePart::Literal(text) => push_hud_chat_literal(&mut runs, text, style),
        LocalizedTemplatePart::Argument { index, raw } => {
            if let Some(argument) = arguments.get(index) {
                runs.extend(argument.iter().cloned());
            } else {
                push_hud_chat_literal(&mut runs, raw, style);
            }
        }
    });
    if runs.is_empty() {
        vec![hud_chat_run(template, style)]
    } else {
        runs
    }
}

#[cfg(test)]
fn push_hud_chat_literal(
    runs: &mut Vec<HudStyledTextRun>,
    text: &str,
    style: &bbb_protocol::ComponentStyle,
) {
    if text.is_empty() {
        return;
    }
    let run = hud_chat_run(text, style);
    if let Some(previous) = runs
        .last_mut()
        .filter(|previous| previous.style == run.style && previous.color == run.color)
    {
        previous.text.push_str(text);
    } else {
        runs.push(run);
    }
}

/// Projects the world's tracked boss bars into the renderer's ordered frame
/// list, mirroring `BossHealthOverlay.extractRenderState`'s walk over its
/// event map (BossHealthOverlay.java:57-82). The world retains the
/// `LinkedHashMap` packet-add order, including in-place replacement and
/// remove/re-add behavior. Names retain their trusted inherited styled runs;
/// the `HudRenderRuntime` caller replaces the latest target progress with
/// `LerpingBossEvent`'s 100ms wall-clock presentation value. Bars whose stored
/// color/overlay names fall outside the vanilla `BossEvent` getName vocabularies
/// (impossible via the protocol path, reachable only through deserialized
/// state) are dropped. The darken-screen / world-fog flags stay behind the
/// world's `boss_overlay_should_*` queries; they are sky/fog effects, not
/// bar draws.
#[cfg(test)]
pub(crate) fn hud_boss_bars_from_world(world: &WorldStore) -> Vec<HudBossBar> {
    world
        .boss_bars()
        .iter()
        .filter_map(|bar| hud_boss_bar_from_world_state(bar, bar.progress))
        .collect()
}

pub(crate) fn hud_boss_bar_from_world_state(
    bar: &bbb_world::BossBarState,
    progress: f32,
) -> Option<HudBossBar> {
    Some(HudBossBar {
        name_runs: bar
            .name
            .styled()
            .iter()
            .map(|run| hud_chat_run(&run.text, &run.style))
            .collect(),
        progress,
        color: HudBossBarColor::from_name(&bar.color)?,
        overlay: HudBossBarOverlay::from_name(&bar.overlay)?,
    })
}

pub(crate) fn hud_player_list_from_world(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    tick_count: u64,
    show_heads: bool,
) -> Option<HudPlayerList> {
    if input.hide_gui() || !input.player_list_key_down() || input_screen_is_open(input, world) {
        return None;
    }
    let scoreboard = world.scoreboard();
    let mut entries: Vec<_> = world
        .listed_players()
        .iter()
        .filter_map(|id| world.player_info_entry(*id))
        .collect();
    entries.sort_by(|left, right| {
        let left_team = scoreboard
            .team_for_scoreboard_name(&left.profile.name)
            .map(|team| team.name.as_str())
            .unwrap_or("");
        let right_team = scoreboard
            .team_for_scoreboard_name(&right.profile.name)
            .map(|team| team.name.as_str())
            .unwrap_or("");
        right
            .list_order
            .cmp(&left.list_order)
            .then_with(|| left.is_spectator().cmp(&right.is_spectator()))
            .then_with(|| left_team.cmp(right_team))
            .then_with(|| {
                left.profile
                    .name
                    .to_lowercase()
                    .cmp(&right.profile.name.to_lowercase())
            })
    });
    let entries = entries
        .into_iter()
        .take(80)
        .map(|entry| {
            let mut name_runs = entry.display_name.as_ref().map_or_else(
                || {
                    hud_scoreboard_text_runs(
                        &scoreboard.team_formatted_literal_name(&entry.profile.name),
                        item_runtime,
                    )
                },
                |display_name| vec![HudStyledTextRun::plain(display_name)],
            );
            if entry.is_spectator() {
                for run in &mut name_runs {
                    run.style.italic = true;
                }
            }
            let score =
                scoreboard
                    .player_list_score(&entry.profile.name)
                    .map(|score| match score {
                        bbb_world::ScoreboardPlayerListScoreState::Integer(text) => {
                            HudPlayerListScore::Integer(hud_scoreboard_text_runs(
                                &text,
                                item_runtime,
                            ))
                        }
                        bbb_world::ScoreboardPlayerListScoreState::Hearts(value) => {
                            let hearts = bbb_protocol::java_float_to_string(value as f32 / 2.0_f32);
                            HudPlayerListScore::Hearts {
                                value,
                                hp_text: localized_gui_text_with_args(
                                    item_runtime,
                                    "multiplayer.player.list.hp",
                                    "%shp",
                                    &[&hearts],
                                ),
                                compact_text: hearts,
                            }
                        }
                    });
            HudPlayerListEntry {
                profile_id: entry.profile.uuid.as_u128(),
                name_runs,
                latency: entry.latency,
                spectator: entry.is_spectator(),
                score,
            }
        })
        .collect::<Vec<_>>();
    let header = world
        .tab_list()
        .header
        .as_ref()
        .map(|component| hud_tab_list_component_runs(component, item_runtime));
    let footer = world
        .tab_list()
        .footer
        .as_ref()
        .map(|component| hud_tab_list_component_runs(component, item_runtime));
    Some(HudPlayerList {
        header,
        footer,
        entries,
        show_heads,
        tick_count,
    })
}

fn hud_tab_list_component_runs(
    component: &bbb_protocol::RichTextComponent,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudStyledTextRun> {
    hud_component_text_runs(component, item_runtime)
        .into_iter()
        .map(|run| run.visual)
        .collect()
}

pub(crate) fn hud_scoreboard_sidebar_from_world(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<HudScoreboardSidebar> {
    if input.hide_gui() {
        return None;
    }
    let sidebar = world.scoreboard_sidebar()?;
    Some(HudScoreboardSidebar {
        title_runs: hud_scoreboard_text_runs(&sidebar.title, item_runtime),
        entries: sidebar
            .entries
            .iter()
            .map(|entry| HudScoreboardSidebarEntry {
                name_runs: hud_scoreboard_text_runs(&entry.name, item_runtime),
                score_runs: hud_scoreboard_text_runs(&entry.score, item_runtime),
            })
            .collect(),
    })
}

pub(crate) fn hud_hotbar_mode_from_world(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    now: Instant,
) -> HudHotbarMode {
    if input.hide_gui() {
        return HudHotbarMode::Hidden;
    }
    if !world.local_player_is_spectator() {
        return HudHotbarMode::Ordinary;
    }
    let Some(page) = input.spectator_menu_page_at(now) else {
        return HudHotbarMode::Spectator(None);
    };
    let prompt_runs = match page.prompt {
        crate::input::spectator_menu::SpectatorMenuPrompt::Root => spectator_localized_runs(
            item_runtime,
            "spectatorMenu.root.prompt",
            "Press a key to select a command, and again to use it.",
        ),
        crate::input::spectator_menu::SpectatorMenuPrompt::Player => spectator_localized_runs(
            item_runtime,
            "spectatorMenu.teleport.prompt",
            "Select a player to teleport to",
        ),
        crate::input::spectator_menu::SpectatorMenuPrompt::Team => spectator_localized_runs(
            item_runtime,
            "spectatorMenu.team_teleport.prompt",
            "Select a team to teleport to",
        ),
    };
    HudHotbarMode::Spectator(Some(HudSpectatorGui {
        alpha: page.alpha,
        selected_slot: u8::try_from(page.selected_slot).ok(),
        prompt_runs,
        slots: page.slots.map(|item| {
            use crate::input::spectator_menu::SpectatorMenuItemView;
            let (icon, enabled, name_runs) = match item {
                SpectatorMenuItemView::Empty => return None,
                SpectatorMenuItemView::PlayersCategory { enabled } => (
                    HudSpectatorIcon::TeleportToPlayer,
                    enabled,
                    spectator_localized_runs(
                        item_runtime,
                        "spectatorMenu.teleport",
                        "Teleport to Player",
                    ),
                ),
                SpectatorMenuItemView::TeamsCategory { enabled } => (
                    HudSpectatorIcon::TeleportToTeam,
                    enabled,
                    spectator_localized_runs(
                        item_runtime,
                        "spectatorMenu.team_teleport",
                        "Teleport to Team Member",
                    ),
                ),
                SpectatorMenuItemView::Player(player) => (
                    HudSpectatorIcon::Player {
                        skin: spectator_player_skin(&player.profile, item_runtime),
                    },
                    item.enabled(),
                    vec![HudStyledTextRun::plain(&player.profile.name)],
                ),
                SpectatorMenuItemView::Team(team) => (
                    HudSpectatorIcon::Team {
                        skin: spectator_player_skin(&team.icon_player.profile, item_runtime),
                        color_rgb: team.color_rgb,
                    },
                    item.enabled(),
                    hud_scoreboard_text_runs(&team.display_name, item_runtime),
                ),
                SpectatorMenuItemView::PreviousPage => (
                    HudSpectatorIcon::PreviousPage,
                    item.enabled(),
                    spectator_localized_runs(
                        item_runtime,
                        "spectatorMenu.previous_page",
                        "Previous Page",
                    ),
                ),
                SpectatorMenuItemView::NextPage { .. } => (
                    HudSpectatorIcon::NextPage,
                    item.enabled(),
                    spectator_localized_runs(item_runtime, "spectatorMenu.next_page", "Next Page"),
                ),
                SpectatorMenuItemView::Close => (
                    HudSpectatorIcon::Close,
                    item.enabled(),
                    spectator_localized_runs(item_runtime, "spectatorMenu.close", "Close Menu"),
                ),
            };
            Some(HudSpectatorGuiSlot {
                icon,
                enabled,
                name_runs,
            })
        }),
    }))
}

fn spectator_localized_runs(
    item_runtime: Option<&NativeItemRuntime>,
    translation_key: &str,
    fallback: &str,
) -> Vec<HudStyledTextRun> {
    vec![HudStyledTextRun::plain(localized_gui_text(
        item_runtime,
        translation_key,
        fallback,
    ))]
}

fn spectator_player_skin(
    profile: &bbb_world::PlayerInfoProfileState,
    item_runtime: Option<&NativeItemRuntime>,
) -> bbb_renderer::EntityPlayerSkin {
    item_runtime.map_or_else(
        || {
            bbb_renderer::EntityPlayerSkin::ProfiledDefault(
                bbb_item_model::default_player_skin_for_profile_id(profile.uuid.as_u128()),
            )
        },
        |item_runtime| {
            item_runtime.player_skin_for_profile(
                &crate::entity_scene::player_info_resolvable_profile(profile),
            )
        },
    )
}

fn hud_scoreboard_text_runs(
    text: &bbb_world::ScoreboardTextState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudStyledTextRun> {
    text.segments
        .iter()
        .flat_map(|segment| match segment {
            bbb_world::ScoreboardTextSegmentState::Rich(component) => {
                localized_component_runs(component, &text.inherited_style, item_runtime)
            }
            bbb_world::ScoreboardTextSegmentState::Styled(runs) => runs
                .iter()
                .map(|run| bbb_protocol::StyledTextRun {
                    text: run.text.clone(),
                    style: run.style.apply_to(&text.inherited_style),
                })
                .collect(),
        })
        .map(|run| hud_chat_run(&run.text, &run.style))
        .collect()
}

/// One frame's worth of world->renderer state.
///
/// `pump_network_and_terrain` extracts each field at a vanilla-verified
/// client-tick sequence point, then commits the whole frame to the renderer in
/// one `apply_renderer_frame` call. Sky-flash-dependent environment fields read
/// after the `ClientLevel.tick`-equivalent `advance_sky_flash_time`.
pub(crate) enum RendererFrameUpdate<T> {
    Unchanged,
    Replace(T),
}

pub(crate) struct RendererFrame<'a> {
    pub(crate) lightmap_environment: LightmapEnvironment,
    pub(crate) clear_color: ClearColor,
    pub(crate) fog_environment: FogEnvironment,
    pub(crate) sky_environment: SkyEnvironment,
    pub(crate) cloud_environment: CloudEnvironment,
    pub(crate) hide_gui: bool,
    pub(crate) hud_player_health: Option<HudPlayerHealth>,
    pub(crate) hud_sleep_overlay: Option<HudSleepOverlay>,
    pub(crate) hud_food: Option<i32>,
    pub(crate) hud_food_effect: HudFoodEffect,
    pub(crate) hud_mob_effects: Vec<HudMobEffect>,
    pub(crate) hud_inventory_shows_mob_effects: bool,
    pub(crate) hud_armor: Option<i32>,
    pub(crate) hud_air: Option<HudAirSupply>,
    pub(crate) hud_vehicle_health: Option<HudVehicleHealth>,
    pub(crate) hud_vignette: Option<HudVignette>,
    pub(crate) hud_spyglass_scope: Option<HudSpyglassScope>,
    pub(crate) hud_contextual_bar: HudContextualBar,
    pub(crate) hud_experience_level: Option<i32>,
    pub(crate) hud_hotbar_mode: HudHotbarMode,
    pub(crate) hud_selected_slot: u8,
    pub(crate) hud_hotbar_item_icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS],
    pub(crate) hud_hotbar_block_item_models: RendererFrameUpdate<Vec<Option<HudBlockItemModel>>>,
    pub(crate) hud_inventory_screen: Option<HudInventoryScreen>,
    pub(crate) hud_book_screen: Option<HudBookScreen>,
    pub(crate) hud_screen_blur: bool,
    pub(crate) hud_sign_editor_screen: Option<HudSignEditorScreen>,
    pub(crate) hud_command_block_editor_screen: Option<HudCommandBlockEditorScreen>,
    pub(crate) hud_structure_block_editor_screen: Option<HudStructureBlockEditorScreen>,
    pub(crate) hud_jigsaw_block_editor_screen: Option<HudJigsawBlockEditorScreen>,
    pub(crate) hud_test_block_editor_screen: Option<HudTestBlockEditorScreen>,
    pub(crate) hud_test_instance_block_editor_screen: Option<HudTestInstanceBlockEditorScreen>,
    pub(crate) hud_pause_screen: Option<HudPauseScreen>,
    pub(crate) hud_game_rules_screen: Option<HudGameRulesScreen>,
    pub(crate) hud_death_screen: Option<HudDeathScreen>,
    pub(crate) hud_game_event_screen: Option<HudGameEventScreen>,
    pub(crate) hud_social_interactions_screen: Option<HudSocialInteractionsScreen>,
    pub(crate) hud_stats_screen: Option<HudStatsScreen>,
    pub(crate) hud_system_toast: Option<HudSystemToast>,
    pub(crate) hud_advancement_toasts: Vec<HudAdvancementToast>,
    pub(crate) hud_recipe_toasts: Vec<HudRecipeToast>,
    pub(crate) hud_debug_options_screen: Option<HudDebugOptionsScreen>,
    pub(crate) hud_action_bar_text: Option<HudActionBarText>,
    pub(crate) hud_title_text: Option<HudTitleText>,
    pub(crate) hud_scoreboard_sidebar: Option<HudScoreboardSidebar>,
    pub(crate) hud_player_list: Option<HudPlayerList>,
    pub(crate) hud_chat_messages: RendererFrameUpdate<Vec<HudChatMessage>>,
    pub(crate) hud_chat_context: HudChatContext,
    pub(crate) hud_command_suggestions: Option<HudCommandSuggestions>,
    pub(crate) hud_debug_overlay: Option<HudDebugOverlay>,
    pub(crate) hud_boss_bars: RendererFrameUpdate<Vec<HudBossBar>>,
    pub(crate) item_entity_billboards: Vec<ItemEntityBillboard>,
    pub(crate) item_model_residency: ItemModelResidencyFrame,
    pub(crate) block_item_model_meshes: Vec<ItemModelMesh>,
    pub(crate) block_item_model_z_offset_forward_meshes: Vec<ItemModelMesh>,
    pub(crate) block_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) flat_item_model_meshes: Vec<ItemModelMesh>,
    pub(crate) flat_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) item_model_glint_meshes: Vec<ItemModelMesh>,
    pub(crate) item_model_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) item_pickup_block_item_model_meshes: Vec<ItemModelMesh>,
    pub(crate) item_pickup_block_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) item_pickup_flat_item_model_meshes: Vec<ItemModelMesh>,
    pub(crate) item_pickup_flat_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) item_pickup_item_model_glint_meshes: Vec<ItemModelMesh>,
    pub(crate) item_pickup_item_model_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_block_item_model_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_block_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_flat_item_model_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_flat_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_item_model_glint_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_item_model_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_player_arms: Vec<FirstPersonPlayerArm>,
    pub(crate) first_person_map_background_textures: Vec<FirstPersonMapBackgroundTexture>,
    pub(crate) first_person_map_background_surfaces: Vec<FirstPersonMapBackgroundSurface>,
    pub(crate) first_person_map_textures: Vec<ItemFrameMapTexture>,
    pub(crate) first_person_map_surfaces: Vec<ItemFrameMapSurface>,
    pub(crate) first_person_map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    pub(crate) first_person_map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    pub(crate) first_person_map_text_surfaces: Vec<ItemFrameMapTextSurface>,
    pub(crate) item_frame_map_textures: Vec<ItemFrameMapTexture>,
    pub(crate) item_frame_map_surfaces: Vec<ItemFrameMapSurface>,
    pub(crate) item_frame_map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    pub(crate) item_frame_map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    pub(crate) item_frame_map_text_surfaces: Vec<ItemFrameMapTextSurface>,
    pub(crate) sign_text_surfaces: Vec<SignTextSurface>,
    pub(crate) display_text_surfaces: Vec<DisplayTextSurface>,
    pub(crate) entity_name_tag_surfaces: Vec<EntityNameTagSurface>,
    pub(crate) entity_flame_surfaces: Vec<EntityFlameSurface>,
    pub(crate) entity_shadow_surfaces: Vec<EntityShadowSurface>,
    pub(crate) entity_model_instances: Vec<EntityModelInstance>,
    pub(crate) camera_pose: Option<CameraPose>,
    pub(crate) camera_hurt_effect: Option<bbb_renderer::CameraHurtEffect>,
    pub(crate) camera_spinning_effect: Option<bbb_renderer::CameraSpinningEffect>,
    pub(crate) camera_fov_degrees: f32,
    pub(crate) camera_hud_fov_degrees: f32,
    pub(crate) current_post_effect_id: Option<&'a str>,
    pub(crate) camera_post_effect_active: bool,
    pub(crate) shader_game_time_ticks: f64,
    pub(crate) cloud_frame: CloudFrame,
    pub(crate) weather_render_state: WeatherRenderState,
    pub(crate) world_border_render_state: WorldBorderRenderState,
    pub(crate) selection_outline: Option<SelectionOutline>,
    pub(crate) chunk_border_outline: Option<SelectionOutline>,
    pub(crate) entity_scene_outline: Option<SelectionOutline>,
    pub(crate) entity_target_outline: Option<SelectionOutline>,
    pub(crate) debug_filled_boxes: Vec<DebugFilledBox>,
    pub(crate) debug_filled_rects: Vec<DebugFilledRect>,
    pub(crate) debug_text_billboards: Vec<DebugTextBillboard>,
    pub(crate) debug_lines: Vec<DebugLine>,
    pub(crate) block_destroy_overlays: RendererFrameUpdate<Vec<BlockDestroyOverlay>>,
}

/// Render-thread-only observations for heap-backed non-entity extraction.
///
/// These values are copied into the next immutable backend-telemetry
/// generation. The scratch itself never crosses the render/simulation
/// boundary and therefore needs neither locking nor atomics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NonEntityRenderExtractionObservation {
    pub(crate) hud_nanos: u64,
    pub(crate) hud_peak_nanos: u64,
    pub(crate) item_nanos: u64,
    pub(crate) item_peak_nanos: u64,
    pub(crate) block_entity_nanos: u64,
    pub(crate) block_entity_peak_nanos: u64,
    pub(crate) world_debug_nanos: u64,
    pub(crate) world_debug_peak_nanos: u64,
    pub(crate) apply_nanos: u64,
    pub(crate) apply_peak_nanos: u64,
    pub(crate) capacity_growths: u64,
    pub(crate) cache_hits: u64,
    pub(crate) cache_misses: u64,
    pub(crate) rebuilt_owners: u64,
}

#[derive(Debug, Default)]
pub(crate) struct NonEntityRenderExtractionScratch {
    observation: NonEntityRenderExtractionObservation,
    stable_world: Option<Arc<WorldStore>>,
    item_registry_keys: CachedItemRegistryKeys,
    map_textures: crate::map_textures::MapTextureExtractionCache,
    map_texture_level_generation: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ItemRegistryKeySet {
    pub(crate) trim_materials: Option<Arc<[String]>>,
    pub(crate) trim_patterns: Option<Arc<[String]>>,
    pub(crate) enchantments: Option<Arc<[String]>>,
    pub(crate) damage_types: Option<Arc<[String]>>,
    pub(crate) attributes: Option<Arc<[String]>>,
}

#[derive(Debug, Default)]
struct CachedItemRegistryKeys {
    revision: Option<bbb_world::RegistryAccessRevision>,
    keys: ItemRegistryKeySet,
}

impl NonEntityRenderExtractionScratch {
    pub(crate) fn observation(&self) -> NonEntityRenderExtractionObservation {
        self.observation
    }

    /// Returns whether the exact immutable world owner was already applied to
    /// the renderer. Retaining this Arc pins at most the most recently rendered
    /// generation and never exposes a mutable world path.
    pub(crate) fn begin_stable_world_frame(&mut self, world: &Arc<WorldStore>) -> bool {
        let unchanged = self
            .stable_world
            .as_ref()
            .is_some_and(|previous| Arc::ptr_eq(previous, world));
        if unchanged {
            self.observation.cache_hits = self.observation.cache_hits.saturating_add(3);
        } else {
            self.stable_world = Some(Arc::clone(world));
            self.observation.cache_misses = self.observation.cache_misses.saturating_add(3);
            self.observation.rebuilt_owners = self.observation.rebuilt_owners.saturating_add(3);
        }
        unchanged
    }

    pub(crate) fn invalidate_resources(&mut self) {
        self.stable_world = None;
        self.map_textures.clear();
    }

    /// Invalidates render-thread map pixels when the canonical level owner
    /// changes. Map ids are scoped to that owner and must not retain decoded
    /// pixels or GPU slots across disconnect, respawn, or dimension reset.
    pub(crate) fn synchronize_map_texture_level(&mut self, level_generation: u64) -> bool {
        let changed = self
            .map_texture_level_generation
            .replace(level_generation)
            .is_some_and(|previous| previous != level_generation);
        if changed {
            self.map_textures.clear();
        }
        changed
    }

    pub(crate) fn begin_map_texture_frame(
        &mut self,
    ) -> &mut crate::map_textures::MapTextureExtractionCache {
        self.map_textures.begin_frame();
        &mut self.map_textures
    }

    pub(crate) fn item_registry_keys(&mut self, world: &WorldStore) -> ItemRegistryKeySet {
        let revision = world.registry_access_revision();
        if self.item_registry_keys.revision == Some(revision) {
            self.observation.cache_hits = self.observation.cache_hits.saturating_add(5);
            return self.item_registry_keys.keys.clone();
        }

        self.item_registry_keys = CachedItemRegistryKeys {
            revision: Some(revision),
            keys: ItemRegistryKeySet {
                trim_materials: world_trim_material_keys(world).map(Arc::from),
                trim_patterns: world_trim_pattern_keys(world).map(Arc::from),
                enchantments: world_enchantment_keys(world).map(Arc::from),
                damage_types: world_damage_type_keys(world).map(Arc::from),
                attributes: world_attribute_keys(world).map(Arc::from),
            },
        };
        self.observation.cache_misses = self.observation.cache_misses.saturating_add(5);
        self.item_registry_keys.keys.clone()
    }

    pub(crate) fn record_frame(
        &mut self,
        hud: Duration,
        item: Duration,
        block_entity: Duration,
        world_debug: Duration,
        apply: Duration,
    ) {
        let hud_nanos = duration_nanos(hud);
        let item_nanos = duration_nanos(item);
        let block_entity_nanos = duration_nanos(block_entity);
        let world_debug_nanos = duration_nanos(world_debug);
        let apply_nanos = duration_nanos(apply);
        self.observation.hud_nanos = hud_nanos;
        self.observation.hud_peak_nanos = self.observation.hud_peak_nanos.max(hud_nanos);
        self.observation.item_nanos = item_nanos;
        self.observation.item_peak_nanos = self.observation.item_peak_nanos.max(item_nanos);
        self.observation.block_entity_nanos = block_entity_nanos;
        self.observation.block_entity_peak_nanos = self
            .observation
            .block_entity_peak_nanos
            .max(block_entity_nanos);
        self.observation.world_debug_nanos = world_debug_nanos;
        self.observation.world_debug_peak_nanos = self
            .observation
            .world_debug_peak_nanos
            .max(world_debug_nanos);
        self.observation.apply_nanos = apply_nanos;
        self.observation.apply_peak_nanos = self.observation.apply_peak_nanos.max(apply_nanos);
    }
}

fn duration_nanos(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

/// Commits one extracted frame to the renderer in a single call.
pub(crate) fn apply_renderer_frame(
    renderer: &mut Renderer,
    frame: RendererFrame<'_>,
    entity_render_extraction: &mut crate::entity_scene::EntityRenderExtractionScratch,
) {
    renderer.set_lightmap_environment(frame.lightmap_environment);
    renderer.set_clear_color(frame.clear_color);
    renderer.set_fog_environment(frame.fog_environment);
    renderer.set_sky_environment(frame.sky_environment);
    renderer.set_cloud_environment(frame.cloud_environment);
    renderer.set_hide_gui(frame.hide_gui);
    renderer.set_hud_player_health(frame.hud_player_health);
    renderer.set_hud_sleep_overlay(frame.hud_sleep_overlay);
    renderer.set_hud_food(frame.hud_food);
    renderer.set_hud_food_effect(frame.hud_food_effect);
    renderer.set_hud_mob_effects(frame.hud_mob_effects, frame.hud_inventory_shows_mob_effects);
    renderer.set_hud_armor(frame.hud_armor);
    renderer.set_hud_air(frame.hud_air);
    renderer.set_hud_vehicle_health(frame.hud_vehicle_health);
    renderer.set_hud_vignette(frame.hud_vignette);
    renderer.set_hud_spyglass_scope(frame.hud_spyglass_scope);
    renderer.set_hud_contextual_bar(frame.hud_contextual_bar);
    renderer.set_hud_experience_level(frame.hud_experience_level);
    renderer.set_hud_hotbar_mode(frame.hud_hotbar_mode);
    renderer.set_hud_selected_slot(frame.hud_selected_slot);
    renderer.set_hud_hotbar_item_icons(frame.hud_hotbar_item_icons);
    if let RendererFrameUpdate::Replace(models) = frame.hud_hotbar_block_item_models {
        renderer.set_hud_hotbar_block_item_models(models);
    }
    renderer.set_hud_inventory_screen(frame.hud_inventory_screen);
    renderer.set_hud_book_screen(frame.hud_book_screen);
    renderer.set_hud_screen_blur(frame.hud_screen_blur);
    renderer.set_hud_sign_editor_screen(frame.hud_sign_editor_screen);
    renderer.set_hud_command_block_editor_screen(frame.hud_command_block_editor_screen);
    renderer.set_hud_structure_block_editor_screen(frame.hud_structure_block_editor_screen);
    renderer.set_hud_jigsaw_block_editor_screen(frame.hud_jigsaw_block_editor_screen);
    renderer.set_hud_test_block_editor_screen(frame.hud_test_block_editor_screen);
    renderer.set_hud_test_instance_block_editor_screen(frame.hud_test_instance_block_editor_screen);
    renderer.set_hud_pause_screen(frame.hud_pause_screen);
    renderer.set_hud_game_rules_screen(frame.hud_game_rules_screen);
    renderer.set_hud_death_screen(frame.hud_death_screen);
    renderer.set_hud_game_event_screen(frame.hud_game_event_screen);
    renderer.set_hud_social_interactions_screen(frame.hud_social_interactions_screen);
    renderer.set_hud_stats_screen(frame.hud_stats_screen);
    renderer.set_hud_system_toast(frame.hud_system_toast);
    renderer.set_hud_advancement_toasts(frame.hud_advancement_toasts);
    renderer.set_hud_recipe_toasts(frame.hud_recipe_toasts);
    renderer.set_hud_debug_options_screen(frame.hud_debug_options_screen);
    renderer.set_hud_action_bar_text(frame.hud_action_bar_text);
    renderer.set_hud_title_text(frame.hud_title_text);
    renderer.set_hud_scoreboard_sidebar(frame.hud_scoreboard_sidebar);
    if let RendererFrameUpdate::Replace(messages) = frame.hud_chat_messages {
        renderer.set_hud_chat_messages(messages);
    }
    renderer.set_hud_chat_context(frame.hud_chat_context);
    renderer.set_hud_command_suggestions(frame.hud_command_suggestions);
    renderer.set_hud_player_list(frame.hud_player_list);
    renderer.set_hud_debug_overlay(frame.hud_debug_overlay);
    if let RendererFrameUpdate::Replace(bars) = frame.hud_boss_bars {
        renderer.set_hud_boss_bars(bars);
    }
    renderer.set_item_entity_billboards(frame.item_entity_billboards);
    renderer
        .set_item_model_residency_frame(frame.item_model_residency)
        .expect("resident item-model frame must reference available exact owners");
    renderer.set_block_item_model_meshes(frame.block_item_model_meshes);
    renderer.set_block_item_model_z_offset_forward_meshes(
        frame.block_item_model_z_offset_forward_meshes,
    );
    renderer.set_block_item_model_translucent_meshes(frame.block_item_model_translucent_meshes);
    renderer.set_flat_item_model_meshes(frame.flat_item_model_meshes);
    renderer.set_flat_item_model_translucent_meshes(frame.flat_item_model_translucent_meshes);
    renderer.set_item_model_glint_meshes(frame.item_model_glint_meshes);
    renderer.set_item_model_glint_translucent_meshes(frame.item_model_glint_translucent_meshes);
    renderer.set_item_pickup_block_item_model_meshes(frame.item_pickup_block_item_model_meshes);
    renderer.set_item_pickup_block_item_model_translucent_meshes(
        frame.item_pickup_block_item_model_translucent_meshes,
    );
    renderer.set_item_pickup_flat_item_model_meshes(frame.item_pickup_flat_item_model_meshes);
    renderer.set_item_pickup_flat_item_model_translucent_meshes(
        frame.item_pickup_flat_item_model_translucent_meshes,
    );
    renderer.set_item_pickup_item_model_glint_meshes(frame.item_pickup_item_model_glint_meshes);
    renderer.set_item_pickup_item_model_glint_translucent_meshes(
        frame.item_pickup_item_model_glint_translucent_meshes,
    );
    renderer.set_first_person_block_item_model_meshes(frame.first_person_block_item_model_meshes);
    renderer.set_first_person_block_item_model_translucent_meshes(
        frame.first_person_block_item_model_translucent_meshes,
    );
    renderer.set_first_person_flat_item_model_meshes(frame.first_person_flat_item_model_meshes);
    renderer.set_first_person_flat_item_model_translucent_meshes(
        frame.first_person_flat_item_model_translucent_meshes,
    );
    renderer.set_first_person_item_model_glint_meshes(frame.first_person_item_model_glint_meshes);
    renderer.set_first_person_item_model_glint_translucent_meshes(
        frame.first_person_item_model_glint_translucent_meshes,
    );
    renderer.set_first_person_player_arms(frame.first_person_player_arms);
    renderer.set_first_person_map_background_surfaces(
        frame.first_person_map_background_textures,
        frame.first_person_map_background_surfaces,
    );
    renderer.set_map_surfaces(
        frame.item_frame_map_textures,
        frame.first_person_map_textures,
        frame.item_frame_map_surfaces,
        frame.first_person_map_surfaces,
    );
    renderer.set_map_decoration_surfaces(
        frame.item_frame_map_decoration_textures,
        frame.first_person_map_decoration_textures,
        frame.item_frame_map_decoration_surfaces,
        frame.first_person_map_decoration_surfaces,
    );
    renderer.set_map_text_surfaces(
        frame.item_frame_map_text_surfaces,
        frame.first_person_map_text_surfaces,
    );
    renderer.set_sign_text_surfaces(frame.sign_text_surfaces);
    renderer.set_display_text_surfaces(frame.display_text_surfaces);
    renderer.set_entity_name_tag_surfaces(frame.entity_name_tag_surfaces);
    renderer.set_entity_flame_surfaces(frame.entity_flame_surfaces);
    renderer.set_entity_shadow_surfaces(frame.entity_shadow_surfaces);
    let recycled_instances = renderer.replace_entity_model_instances(frame.entity_model_instances);
    entity_render_extraction.recycle_instances(recycled_instances);
    renderer.set_camera_hurt_effect(frame.camera_hurt_effect);
    renderer.set_camera_spinning_effect(frame.camera_spinning_effect);
    renderer.set_camera_fov_degrees(frame.camera_fov_degrees, frame.camera_hud_fov_degrees);
    renderer.set_camera_pose(frame.camera_pose);
    renderer.set_camera_post_effect(
        frame.current_post_effect_id,
        frame.camera_post_effect_active,
    );
    renderer.set_shader_game_time_ticks(frame.shader_game_time_ticks);
    renderer.set_cloud_frame(frame.cloud_frame);
    renderer.set_weather_render_state(frame.weather_render_state);
    renderer.set_world_border_render_state(frame.world_border_render_state);
    renderer.set_selection_outline(frame.selection_outline);
    renderer.set_chunk_border_outline(frame.chunk_border_outline);
    renderer.set_entity_scene_outline(frame.entity_scene_outline);
    renderer.set_entity_target_outline(frame.entity_target_outline);
    renderer.set_debug_filled_boxes(frame.debug_filled_boxes);
    renderer.set_debug_filled_rects(frame.debug_filled_rects);
    renderer.set_debug_text_billboards(frame.debug_text_billboards);
    renderer.set_debug_lines(frame.debug_lines);
    if let RendererFrameUpdate::Replace(overlays) = frame.block_destroy_overlays {
        renderer.set_block_destroy_overlays(overlays);
    }
}
