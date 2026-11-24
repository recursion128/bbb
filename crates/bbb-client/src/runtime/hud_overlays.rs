use super::*;

pub(super) const VANILLA_SPYGLASS_SCOPE_TARGET_SCALE: f32 = 1.125;
const VANILLA_NETHER_PORTAL_BLOCK: &str = "minecraft:nether_portal";
pub(super) const VANILLA_SPYGLASS_SCOPE_RESET_SCALE: f32 = 0.5;

/// The hotbar's 3D block items (vanilla inventory item rendering): for each slot holding a block item,
/// its [`block_item_3d_model`], so the renderer draws it as a 3D icon instead of the flat 2D sprite.
/// `None` for empty slots and flat items (which keep the 2D sprite drawn by the HUD layer).
/// Vanilla `Gui.extractItemHotbar`: the offhand background
/// and slot are drawn only for a non-empty offhand stack, on the side opposite
/// `Player.getMainArm()`, and the slot seeds continue after the nine hotbar
/// slots.
/// Vanilla `Gui.extractCameraOverlays`: the nausea overlay
/// is the portal overlay's `else` branch, so an active portal effect hides it
/// entirely, and `screenEffectScale` at its default `1.0` suppresses it too.
pub(super) fn hud_confusion_overlay(
    world: &WorldStore,
    hide_gui: bool,
    screen_effect_scale: f32,
    partial_tick: f32,
) -> Option<HudConfusionOverlay> {
    if hide_gui || world.local_player_portal_effect_intensity(partial_tick) > 0.0 {
        return None;
    }
    let scale = if screen_effect_scale.is_finite() {
        screen_effect_scale.clamp(0.0, 1.0)
    } else {
        1.0
    };
    if scale >= 1.0 {
        return None;
    }
    let nausea =
        local_player_effect(world, VANILLA_MOB_EFFECT_NAUSEA_ID)?.blend_factor(partial_tick);
    if nausea.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    Some(HudConfusionOverlay {
        strength: nausea * (1.0 - scale),
    })
}

/// Vanilla `Gui.extractCameraOverlays`: the portal overlay
/// follows the partial-tick interpolation of `Player.portalEffectIntensity`
/// and blits the nether-portal block's own particle sprite, so it keeps
/// animating with the rest of the blocks atlas.
/// Vanilla `LevelLoadingScreen` (shown for the whole
/// `ClientPacketListener.startWaitingForNewLevel` window): the reason picks the
/// background and the centered `multiplayer.downloadingTerrain` line is the
/// only status text an external client can reach, because
/// `LevelLoadTracker.hasProgress()`/`statusView()` stay integrated-server-only.
pub(super) fn hud_level_loading_screen(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    item_runtime: Option<&NativeItemRuntime>,
    panorama_rotation_degrees: f32,
) -> Option<HudLevelLoadingScreen> {
    if !world.level_load().screen_is_up() {
        return None;
    }
    let background = match world.level_load().reason {
        WorldLevelLoadReason::NetherPortal => nether_portal_sprite_uv(terrain_textures)
            .map(|sprite_uv| HudLevelLoadingBackground::NetherPortal { sprite_uv })
            .unwrap_or(HudLevelLoadingBackground::Panorama {
                rotation_degrees: panorama_rotation_degrees,
            }),
        WorldLevelLoadReason::EndPortal => HudLevelLoadingBackground::EndPortal,
        WorldLevelLoadReason::Other => HudLevelLoadingBackground::Panorama {
            rotation_degrees: panorama_rotation_degrees,
        },
    };
    Some(HudLevelLoadingScreen {
        background,
        downloading_terrain: localized_gui_text(
            item_runtime,
            "multiplayer.downloadingTerrain",
            "Loading terrain...",
        ),
    })
}

fn nether_portal_sprite_uv(terrain_textures: &TerrainTextureState) -> Option<HudUvRect> {
    let block_state_id = bbb_client_world::vanilla_block_state_id(
        VANILLA_NETHER_PORTAL_BLOCK,
        &std::collections::BTreeMap::new(),
    )
    .or_else(|| {
        bbb_client_world::BlockStateRegistry::vanilla_26_1()
            .iter()
            .find(|state| state.name == VANILLA_NETHER_PORTAL_BLOCK)
            .map(|state| state.id)
    })?;
    let sprite_id = terrain_textures.particle_sprite_id_for_block_state(block_state_id)?;
    let uv = terrain_textures.sprite_uv_rect(&sprite_id);
    Some(HudUvRect {
        min: uv.min,
        max: uv.max,
    })
}

pub(super) fn hud_portal_overlay(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    hide_gui: bool,
    partial_tick: f32,
) -> Option<HudPortalOverlay> {
    if hide_gui {
        return None;
    }
    let intensity = world.local_player_portal_effect_intensity(partial_tick);
    if intensity.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    Some(HudPortalOverlay {
        sprite_uv: nether_portal_sprite_uv(terrain_textures)?,
        intensity,
    })
}

/// Vanilla `Gui.extractCameraOverlays`: the equippable
/// camera overlays are first-person only and are replaced by the spyglass scope
/// while scoping, while the powder-snow outline follows `getTicksFrozen()` in
/// every camera type.
pub(super) fn hud_camera_overlays(
    world: &WorldStore,
    hide_gui: bool,
    first_person: bool,
    scoping: bool,
) -> Vec<HudCameraOverlay> {
    if hide_gui {
        return Vec::new();
    }
    let mut overlays = Vec::new();
    if first_person && !scoping {
        overlays.extend(
            world
                .local_player_camera_overlay_textures()
                .into_iter()
                .map(|texture_id| HudCameraOverlay {
                    texture_id,
                    alpha: 1.0,
                }),
        );
    }
    if world.local_player_ticks_frozen() > 0 {
        overlays.push(HudCameraOverlay {
            texture_id: VANILLA_POWDER_SNOW_OUTLINE_TEXTURE.to_string(),
            alpha: world.local_player_percent_frozen(),
        });
    }
    overlays
}

const DEBUG_GAME_MODE_SWITCHER_BACKGROUND_WIDTH: i32 = 125;
const DEBUG_GAME_MODE_SWITCHER_BACKGROUND_HEIGHT: i32 = 75;
const DEBUG_GAME_MODE_SWITCHER_BACKGROUND_X_OFFSET: i32 = 62;
const DEBUG_GAME_MODE_SWITCHER_BACKGROUND_Y_OFFSET: i32 = 58;
const DEBUG_GAME_MODE_SWITCHER_SLOT_AREA: i32 = 26;
const DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED: i32 = 31;
const DEBUG_GAME_MODE_SWITCHER_ALL_SLOTS_WIDTH: i32 = 4 * DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED - 5;
const DEBUG_GAME_MODE_SWITCHER_SLOT_Y_OFFSET: i32 = 31;
pub(super) const DEBUG_GAME_MODE_SWITCHER_KEY_MAPPING: &str = "key.debug.switchGameMode";

pub(super) fn hud_debug_game_mode_switcher(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    surface_size: winit::dpi::PhysicalSize<u32>,
    partial_tick: f32,
) -> Option<HudDebugGameModeSwitcher> {
    let selected = input.debug_game_mode_switcher_selected()?;
    let item_models = HudItemModelContext::new(world, item_runtime);
    let team_color = world.local_player_team_color_rgb();
    let center_x = i32::try_from(surface_size.width / 2).unwrap_or(i32::MAX);
    let center_y = i32::try_from(surface_size.height / 2).unwrap_or(i32::MAX);
    let slot_start_x = center_x - DEBUG_GAME_MODE_SWITCHER_ALL_SLOTS_WIDTH / 2;
    let slot_y = center_y - DEBUG_GAME_MODE_SWITCHER_SLOT_Y_OFFSET;
    let shift_down = input.shift_down();
    let keybind_context = input.item_model_keybind_context();
    let slots = DEBUG_GAME_MODE_SWITCHER_MODES
        .iter()
        .enumerate()
        .map(|(index, mode)| {
            let stack = hud_game_mode_switcher_icon_stack(*mode, item_runtime);
            let slot = HudItemSlotState {
                shift_down,
                keybind: keybind_context,
                seed: i32::try_from(index + 1).unwrap_or(0),
                ..HudItemSlotState::default()
            };
            let mut icon = stack.as_ref().and_then(|stack| {
                hud_item_icon(&item_models, stack, slot, team_color, partial_tick)
            });
            if let Some(icon) = &mut icon {
                icon.count_label = None;
                icon.durability_bar = None;
                icon.cooldown_progress = None;
            }
            let block_model = stack.as_ref().and_then(|stack| {
                hud_block_item_model(&item_models, stack, slot, team_color, terrain_textures)
            });
            HudDebugGameModeSwitcherSlot {
                mode: *mode,
                x: slot_start_x
                    + i32::try_from(index).unwrap_or(0) * DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED,
                y: slot_y,
                width: DEBUG_GAME_MODE_SWITCHER_SLOT_AREA,
                height: DEBUG_GAME_MODE_SWITCHER_SLOT_AREA,
                selected: *mode == selected,
                icon,
                block_model,
            }
        })
        .collect();
    Some(HudDebugGameModeSwitcher {
        selected,
        title: hud_game_mode_switcher_title(selected).to_string(),
        help_runs: hud_game_mode_switcher_help_runs(item_runtime),
        background_x: center_x - DEBUG_GAME_MODE_SWITCHER_BACKGROUND_X_OFFSET,
        background_y: center_y - DEBUG_GAME_MODE_SWITCHER_BACKGROUND_Y_OFFSET,
        background_width: DEBUG_GAME_MODE_SWITCHER_BACKGROUND_WIDTH,
        background_height: DEBUG_GAME_MODE_SWITCHER_BACKGROUND_HEIGHT,
        slots,
    })
}

fn hud_game_mode_switcher_icon_stack(
    mode: GameType,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ItemStackSummary> {
    let item_id = item_runtime?.item_protocol_id(hud_game_mode_switcher_icon_resource_id(mode))?;
    Some(ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: Default::default(),
    })
}

pub(super) const DEBUG_GAME_MODE_SWITCHER_MODES: [GameType; 4] = [
    GameType::Creative,
    GameType::Survival,
    GameType::Adventure,
    GameType::Spectator,
];

/// `GameModeSwitcherScreen`: the hint is the `select_next`
/// translation with the switcher binding's own key message as its single
/// argument, AQUA applied to that argument alone.
///
/// The argument is the keybind content for the same mapping
/// which resolves through `KeyMapping.createNameSupplier`
/// -- the very method backing
/// `getTranslatedKeyMessage` -- so the run carries the currently bound key's
/// name rather than the default one.
pub(super) fn hud_game_mode_switcher_help_runs(
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudStyledTextRun> {
    let key_message_style = ComponentStyle {
        color: bbb_protocol::chat_formattings::ChatFormatting::Aqua.color(),
        ..ComponentStyle::default()
    };
    bbb_protocol::component_keybind_nbt(DEBUG_GAME_MODE_SWITCHER_KEY_MAPPING, &key_message_style)
        .and_then(|key_message| {
            bbb_protocol::component_translatable_nbt(
                "debug.gamemodes.select_next",
                vec![key_message],
                &ComponentStyle::default(),
            )
        })
        .ok()
        .and_then(|hint| RichTextComponent::try_from_component_nbt(hint).ok())
        .map(|hint| {
            hud_styled_runs_from_component_runs(&localized_component_runs(
                &hint,
                &ComponentStyle::default(),
                item_runtime,
            ))
        })
        .unwrap_or_default()
}

fn hud_game_mode_switcher_title(mode: GameType) -> &'static str {
    match mode {
        GameType::Creative => "Creative Mode",
        GameType::Survival => "Survival Mode",
        GameType::Adventure => "Adventure Mode",
        GameType::Spectator => "Spectator Mode",
    }
}

pub(super) fn hud_game_mode_switcher_icon_resource_id(mode: GameType) -> &'static str {
    match mode {
        GameType::Creative => "minecraft:grass_block",
        GameType::Survival => "minecraft:iron_sword",
        GameType::Adventure => "minecraft:map",
        GameType::Spectator => "minecraft:ender_eye",
    }
}

pub(super) fn hud_crosshair_attack_indicator(
    visible: bool,
    state: Option<LocalPlayerCrosshairAttackIndicatorState>,
) -> HudCrosshairAttackIndicator {
    if !visible {
        return HudCrosshairAttackIndicator::None;
    }
    match state {
        Some(state) if state.full => HudCrosshairAttackIndicator::Full,
        Some(state) if state.strength_scale < 1.0 => HudCrosshairAttackIndicator::Progress {
            strength_scale: state.strength_scale,
        },
        _ => HudCrosshairAttackIndicator::None,
    }
}

pub(super) fn hud_hotbar_attack_indicator(
    visible: bool,
    state: Option<LocalPlayerCrosshairAttackIndicatorState>,
    main_arm_left: bool,
) -> HudHotbarAttackIndicator {
    if !visible {
        return HudHotbarAttackIndicator::None;
    }
    match state {
        Some(state) if state.strength_scale < 1.0 => HudHotbarAttackIndicator::Progress {
            strength_scale: state.strength_scale,
            main_arm_left,
        },
        _ => HudHotbarAttackIndicator::None,
    }
}
