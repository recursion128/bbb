use super::*;

#[test]
fn now_playing_uses_left_slide_static_pause_and_dynamic_vanilla_width() {
    let mut glyphs = HudFontGlyphMap::default();
    for character in "Song".chars() {
        glyphs.insert_first_wins(
            character,
            HudAsciiGlyph {
                advance: 6.0,
                ..HudAsciiGlyph::default()
            },
        );
    }
    let text_width = hud_raw_string_width("Song", &glyphs);
    let toast = HudNowPlayingToast {
        text: "Song".to_string(),
        visible_portion: 0.5,
        note_color_argb: 0xFF12_3456,
        client_tick: 7,
        placement: HudNowPlayingToastPlacement::SlidingToast,
    };
    let (rect, x) = hud_now_playing_toast_layout(&toast, &glyphs);
    assert_eq!(rect.width, (37 + text_width) as u32);
    assert_eq!(rect.height, 30);
    assert_eq!(x, -(rect.width as f32) / 2.0);

    let paused = HudNowPlayingToast {
        placement: HudNowPlayingToastPlacement::PauseScreen,
        ..toast
    };
    let (_, paused_x) = hud_now_playing_toast_layout(&paused, &glyphs);
    assert_eq!(paused_x, 0.0);
    assert_eq!(
        sanitize_hud_now_playing_toast(paused)
            .expect("valid now-playing projection")
            .visible_portion,
        0.5
    );
}

/// `NowPlayingToast` draws its recorded text as a Component, so the toast width
/// has to come from the same resolved run list the glyph pass reads: the `§c`
/// pair charges no advance and colours the glyphs it precedes.
#[test]
fn now_playing_toast_measures_the_styled_glyphs_and_not_the_code() {
    let mut glyphs = HudFontGlyphMap::default();
    for character in "Song\u{a7}c".chars() {
        glyphs.insert_first_wins(
            character,
            HudAsciiGlyph {
                advance: 6.0,
                ..HudAsciiGlyph::default()
            },
        );
    }
    assert_eq!(
        hud_coded_string_runs("\u{a7}cSong"),
        vec![HudStyledTextRun {
            text: "Song".to_string(),
            color: Some(0xFF_5555),
            ..HudStyledTextRun::default()
        }]
    );
    let toast = HudNowPlayingToast {
        text: "\u{a7}cSong".to_string(),
        visible_portion: 0.5,
        note_color_argb: 0xFF12_3456,
        client_tick: 7,
        placement: HudNowPlayingToastPlacement::SlidingToast,
    };
    let (rect, x) = hud_now_playing_toast_layout(&toast, &glyphs);
    assert_eq!(rect.width, 37 + 24);
    assert_eq!(x, -(rect.width as f32) / 2.0);
    assert_eq!(hud_raw_string_width("\u{a7}cSong", &glyphs), 36);
}

#[test]
fn the_level_loading_screen_replaces_every_hud_draw_and_centers_the_status_line() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer.set_hud_sleep_overlay(Some(HudSleepOverlay { alpha: 255 }));
    renderer.set_hud_hotbar_mode(HudHotbarMode::Ordinary);
    let ordinary = renderer.collect_hud_draws();
    assert!(ordinary
        .commands
        .iter()
        .any(|command| matches!(command, HudDrawCommand::Fill { .. })));
    assert_eq!(
        renderer.hud_sound_subtitle_placement(),
        Some(HudSoundSubtitlePlacement::Deferred)
    );

    renderer.set_hud_level_loading_screen(Some(HudLevelLoadingScreen {
        background: HudLevelLoadingBackground::NetherPortal {
            sprite_uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
        downloading_terrain: "Loading terrain...".to_string(),
    }));
    assert_eq!(renderer.hud_sound_subtitle_placement(), None);
    let loading = renderer.collect_hud_draws();
    assert_eq!(loading.post_gui_item_start, loading.commands.len());
    assert!(matches!(
        loading.commands.as_slice(),
        [HudDrawCommand::TerrainAtlas { .. }]
    ));
}

#[test]
fn sound_subtitle_placement_obeys_screen_and_hide_gui_rules() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    assert_eq!(
        renderer.hud_sound_subtitle_placement(),
        Some(HudSoundSubtitlePlacement::Deferred)
    );

    renderer.hud_chat_context.focused = true;
    assert_eq!(
        renderer.hud_sound_subtitle_placement(),
        Some(HudSoundSubtitlePlacement::BeforeScreen)
    );
    renderer.set_hide_gui(true);
    assert_eq!(renderer.hud_sound_subtitle_placement(), None);

    renderer.hud_chat_context.focused = false;
    renderer.set_hud_screen_presence(true, false);
    renderer.hud_chat_restrictions_screen = Some(HudChatRestrictionsScreen {
        generation: 1,
        title: String::new(),
        reason: HudStyledTextRun::plain(String::new()),
        permissions: Vec::new(),
        done: String::new(),
        cursor_position: None,
    });
    assert_eq!(renderer.hud_sound_subtitle_placement(), None);
    renderer.set_hide_gui(false);
    assert_eq!(
        renderer.hud_sound_subtitle_placement(),
        Some(HudSoundSubtitlePlacement::BeforeScreen)
    );
    renderer.hud_chat_restrictions_screen = None;

    renderer.set_hud_screen_presence(true, true);
    renderer.hud_sign_editor_screen = Some(HudSignEditorScreen {
        kind: HudSignEditorKind::Standing {
            wood: SignModelWood::Oak,
            attachment: SignModelAttachment::Standing,
        },
        sign_preview: None,
        title: String::new(),
        lines: std::array::from_fn(|_| String::new()),
        line: 0,
        cursor: 0,
        selection: 0,
        cursor_visible: false,
        text_tint: [1.0; 4],
        done: String::new(),
        done_hovered: false,
        preedit: None,
    });
    assert_eq!(
        renderer.hud_sound_subtitle_placement(),
        Some(HudSoundSubtitlePlacement::BeforeScreen)
    );
    renderer.set_hide_gui(true);
    assert_eq!(
        renderer.hud_sound_subtitle_placement(),
        Some(HudSoundSubtitlePlacement::BeforeScreen)
    );
}

#[test]
fn hide_gui_suppresses_ingame_draws_and_toasts_but_keeps_sleep_and_debug() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .add_test_gui_sprite("hud/crosshair", 2, 2, &[255, 0, 0, 255].repeat(4))
        .expect("crosshair");
    assert_eq!(renderer.collect_hud_draws().commands.len(), 1);

    renderer.set_hide_gui(true);
    assert!(renderer.collect_hud_draws().commands.is_empty());
    assert!(
        renderer.gui_sprite("hud/crosshair").is_some(),
        "F1 must retain the latest ordinary HUD state"
    );

    renderer.set_hud_sleep_overlay(Some(HudSleepOverlay { alpha: 22 }));
    let sleep_commands = renderer.collect_hud_draws().commands.len();
    assert_eq!(sleep_commands, 1, "sleep fade remains visible under F1");

    let glyph = HudAsciiGlyph {
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        width: 1,
        height: 1,
        advance: 1.0,
        bearing_top: HUD_FONT_BASELINE as f32,
        ..HudAsciiGlyph::default()
    };
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins('A', glyph);
    renderer
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
        .expect("font");
    renderer.set_hud_debug_overlay(Some(HudDebugOverlay {
        left_lines: vec!["A".to_string()],
        ..HudDebugOverlay::default()
    }));
    let debug_commands = renderer.collect_hud_draws().commands.len();
    assert!(
        debug_commands > sleep_commands,
        "F3 debug overlay remains visible under F1"
    );

    renderer.set_hud_system_toast(Some(HudSystemToast {
        title: "A".to_string(),
        message_lines: vec!["A".to_string()],
        width: 160,
        visible_portion: 1.0,
    }));
    let toast_commands = renderer.collect_hud_draws().commands.len();
    assert_eq!(
        toast_commands, debug_commands,
        "ToastManager extraction is suppressed under F1"
    );

    renderer.set_hide_gui(false);
    assert!(
        renderer.collect_hud_draws().commands.len() > toast_commands,
        "unchanged crosshair and toast state return when F1 is disabled"
    );
}

#[test]
fn crosshair_visibility_gate_retains_the_uploaded_sprite() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .add_test_gui_sprite("hud/crosshair", 2, 2, &[255, 0, 0, 255].repeat(4))
        .expect("crosshair");
    assert_eq!(renderer.collect_hud_draws().commands.len(), 1);

    renderer.set_hud_crosshair_visible(false);
    assert!(renderer.collect_hud_draws().commands.is_empty());
    assert!(renderer.gui_sprite("hud/crosshair").is_some());

    renderer.set_hud_crosshair_visible(true);
    assert_eq!(renderer.collect_hud_draws().commands.len(), 1);
}

#[test]
fn crosshair_and_attack_indicator_invert_the_frame_and_keep_transparent_pixels() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer.set_clear_color(crate::camera::ClearColor {
        r: 0.25,
        g: 0.5,
        b: 0.75,
        a: 1.0,
    });
    renderer.update_camera();
    let mut rgba = vec![255; 15 * 15 * 4];
    rgba[0..4].copy_from_slice(&[0, 0, 0, 0]);
    renderer
        .add_test_gui_sprite("hud/crosshair", 15, 15, &rgba)
        .unwrap();
    renderer
        .add_test_gui_sprite(
            "hud/crosshair_attack_indicator_full",
            16,
            16,
            &[255; 16 * 16 * 4],
        )
        .unwrap();
    renderer.set_hud_crosshair_attack_indicator(HudCrosshairAttackIndicator::Full);
    let frame = renderer.render_offscreen_frame().unwrap();
    let background = frame.pixel(0, 0);
    let transparent = frame.pixel(152, 112);
    let crosshair = frame.pixel(158, 118);
    let indicator = frame.pixel(158, 132);
    assert_eq!(transparent, background);
    for channel in 0..3 {
        assert!((i16::from(crosshair[channel]) + i16::from(background[channel]) - 255).abs() <= 1);
        assert_eq!(indicator[channel], crosshair[channel]);
    }
}

#[test]
fn crosshair_attack_indicator_draws_after_crosshair_and_obeys_debug_replacement() {
    let Some(mut renderer) = Renderer::new_offscreen(321, 241) else {
        return;
    };
    let solid = |width: u32, height: u32, rgba: [u8; 4]| rgba.repeat((width * height) as usize);
    renderer
        .add_test_gui_sprite("hud/crosshair", 15, 15, &solid(15, 15, [255, 0, 0, 255]))
        .expect("crosshair");
    renderer
        .add_test_gui_sprite(
            "hud/crosshair_attack_indicator_full",
            16,
            16,
            &solid(16, 16, [0, 255, 0, 255]),
        )
        .expect("full indicator");
    renderer
        .add_test_gui_sprite(
            "hud/crosshair_attack_indicator_background",
            16,
            4,
            &solid(16, 4, [0, 0, 255, 255]),
        )
        .expect("indicator background");
    renderer
        .add_test_gui_sprite(
            "hud/crosshair_attack_indicator_progress",
            16,
            4,
            &solid(16, 4, [255, 255, 255, 255]),
        )
        .expect("indicator progress");

    renderer.set_hud_crosshair_attack_indicator(HudCrosshairAttackIndicator::Progress {
        strength_scale: 0.5,
    });
    let draws = renderer.collect_hud_draws();
    assert_eq!(draws.commands.len(), 3);
    for (index, expected) in [
        renderer.gui_sprite("hud/crosshair").unwrap(),
        renderer
            .gui_sprite("hud/crosshair_attack_indicator_background")
            .unwrap(),
        renderer
            .gui_sprite("hud/crosshair_attack_indicator_progress")
            .unwrap(),
    ]
    .into_iter()
    .enumerate()
    {
        let HudDrawCommand::InvertingSprite { sprite, .. } = &draws.commands[index] else {
            panic!("crosshair command {index} must be a sprite");
        };
        assert!(std::ptr::eq(*sprite, expected));
    }
    drop(draws);

    renderer.set_hud_crosshair_attack_indicator(HudCrosshairAttackIndicator::Full);
    let draws = renderer.collect_hud_draws();
    assert_eq!(draws.commands.len(), 2);
    let HudDrawCommand::InvertingSprite { sprite, .. } = &draws.commands[1] else {
        panic!("full indicator command");
    };
    assert!(std::ptr::eq(
        *sprite,
        renderer
            .gui_sprite("hud/crosshair_attack_indicator_full")
            .unwrap()
    ));
    drop(draws);

    renderer.set_hud_debug_overlay(Some(HudDebugOverlay {
        debug_crosshair: Some(HudDebugCrosshair {
            x_rot_degrees: 0.0,
            y_rot_degrees: 0.0,
            gui_scale: 1,
        }),
        ..HudDebugOverlay::default()
    }));
    let draws = renderer.collect_hud_draws();
    assert!(draws.commands.iter().all(|command| {
        let HudDrawCommand::Sprite { sprite, .. } = command else {
            return false;
        };
        std::ptr::eq(*sprite, &renderer.hud_white_pixel)
    }));
    drop(draws);

    renderer.set_hud_debug_overlay(None);
    renderer.set_hud_crosshair_attack_indicator(HudCrosshairAttackIndicator::Progress {
        strength_scale: f32::NAN,
    });
    assert_eq!(
        renderer.hud_crosshair_attack_indicator,
        HudCrosshairAttackIndicator::None
    );
    assert_eq!(renderer.collect_hud_draws().commands.len(), 1);
}

#[test]
fn the_slot_pop_pose_moves_the_item_quad_but_never_its_decorations() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .upload_item_atlas(2, 2, &[255, 255, 255, 255].repeat(4))
        .expect("item atlas");
    renderer.set_hud_hotbar_mode(HudHotbarMode::Ordinary);

    let icon = |pop: f32| HudItemIcon {
        pop,
        durability_bar: Some(HudItemDurabilityBar::new(13, [0.0, 1.0, 0.0])),
        ..HudItemIcon::single(HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        })
    };
    let pixel_xs = |renderer: &mut Renderer| {
        let mut xs = renderer
            .collect_hud_draws()
            .vertices
            .iter()
            .map(|vertex| (vertex.position[0] + 1.0) * 160.0)
            .map(|x| x.round() as i32)
            .collect::<Vec<_>>();
        xs.sort_unstable();
        xs.dedup();
        xs
    };

    let mut icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS] = std::array::from_fn(|_| None);
    icons[0] = Some(icon(0.0));
    renderer.set_hud_hotbar_item_icons(icons.clone());
    let resting = pixel_xs(&mut renderer);
    let slot_left = *resting.first().unwrap();
    // The 13-pixel durability bar sits two pixels inside the slot.
    assert!(resting.contains(&(slot_left + 2)));
    assert!(resting.contains(&(slot_left + 15)));
    assert_eq!(*resting.last().unwrap(), slot_left + 16);

    icons[0] = Some(icon(5.0));
    renderer.set_hud_hotbar_item_icons(icons.clone());
    let popped = pixel_xs(&mut renderer);

    // `squeeze = 2` halves the item quad about its `x + 8` pivot, while the
    // decorations keep their unposed pixels.
    assert!(popped.contains(&(slot_left + 4)));
    assert!(popped.contains(&(slot_left + 12)));
    assert!(!popped.contains(&slot_left));
    assert!(popped.contains(&(slot_left + 2)));
    assert!(popped.contains(&(slot_left + 15)));

    icons[0] = Some(icon(f32::INFINITY));
    renderer.set_hud_hotbar_item_icons(icons);
    assert_eq!(renderer.hud_hotbar_item_icons[0].as_ref().unwrap().pop, 0.0);
}

#[test]
fn the_confusion_overlay_scales_about_the_centre_and_blends_additively() {
    assert_eq!(HudConfusionOverlay { strength: 0.0 }.scale(), 2.0);
    assert_eq!(HudConfusionOverlay { strength: 1.0 }.scale(), 1.0);
    assert_eq!(
        HudConfusionOverlay { strength: 0.5 }.tint(),
        [0.1, 0.2, 0.1, 1.0]
    );

    let corners = hud_confusion_overlay_corners(PhysicalSize::new(320, 240), 2.0);
    assert_eq!(
        corners,
        [
            [-160.0, -120.0],
            [-160.0, 360.0],
            [480.0, 360.0],
            [480.0, -120.0]
        ]
    );
    assert_eq!(
        hud_confusion_overlay_corners(PhysicalSize::new(320, 240), 1.0),
        [[0.0, 0.0], [0.0, 240.0], [320.0, 240.0], [320.0, 0.0]]
    );

    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .upload_gui_texture(
            "textures/misc/nausea.png",
            2,
            2,
            &[255, 255, 255, 255].repeat(4),
            crate::StandaloneTextureSampling::new(false, false),
        )
        .expect("nausea texture");
    renderer.set_hud_confusion_overlay(Some(HudConfusionOverlay { strength: 0.5 }));
    let draws = renderer.collect_hud_draws();
    assert_eq!(draws.commands.len(), 1);
    assert!(matches!(
        draws.commands[0],
        HudDrawCommand::NauseaOverlay { .. }
    ));
    assert!(draws
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [0.1, 0.2, 0.1, 1.0]));
    drop(draws);

    renderer.set_hud_confusion_overlay(Some(HudConfusionOverlay { strength: 0.0 }));
    assert!(renderer.hud_confusion_overlay.is_none());
    renderer.set_hud_confusion_overlay(Some(HudConfusionOverlay { strength: f32::NAN }));
    assert!(renderer.hud_confusion_overlay.is_none());

    renderer.set_hud_confusion_overlay(Some(HudConfusionOverlay { strength: 0.5 }));
    renderer.set_hide_gui(true);
    assert_eq!(renderer.collect_hud_draws().commands.len(), 0);
}

#[test]
fn the_portal_overlay_eases_partial_intensity_and_samples_the_blocks_atlas() {
    // `alpha^4 * 0.8 + 0.2` for a partial intensity, unchanged at full.
    assert_eq!(
        HudPortalOverlay {
            sprite_uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            intensity: 0.5,
        }
        .alpha(),
        0.5_f32.powi(4) * 0.8 + 0.2
    );
    assert_eq!(
        HudPortalOverlay {
            sprite_uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            intensity: 1.0,
        }
        .alpha(),
        1.0
    );

    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let sprite_uv = HudUvRect {
        min: [0.25, 0.5],
        max: [0.5, 0.75],
    };
    renderer.set_hud_portal_overlay(Some(HudPortalOverlay {
        sprite_uv,
        intensity: 1.0,
    }));
    let draws = renderer.collect_hud_draws();
    assert_eq!(draws.commands.len(), 1);
    assert!(matches!(
        draws.commands[0],
        HudDrawCommand::TerrainAtlas { .. }
    ));
    assert!(draws.vertices.iter().all(|vertex| vertex.tint[3] == 1.0));
    assert!(draws
        .vertices
        .iter()
        .all(|vertex| (0.25..=0.5).contains(&vertex.uv[0])));
    drop(draws);

    renderer.set_hud_portal_overlay(Some(HudPortalOverlay {
        sprite_uv,
        intensity: 0.0,
    }));
    assert!(renderer.hud_portal_overlay.is_none());
    renderer.set_hud_portal_overlay(Some(HudPortalOverlay {
        sprite_uv,
        intensity: f32::NAN,
    }));
    assert!(renderer.hud_portal_overlay.is_none());

    renderer.set_hud_portal_overlay(Some(HudPortalOverlay {
        sprite_uv,
        intensity: 1.0,
    }));
    renderer.set_hide_gui(true);
    assert_eq!(renderer.collect_hud_draws().commands.len(), 0);
}

#[test]
fn camera_overlays_blit_full_screen_in_order_and_skip_unknown_textures() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .upload_gui_texture(
            "minecraft:textures/misc/pumpkinblur.png",
            2,
            2,
            &[255, 255, 255, 255].repeat(4),
            crate::StandaloneTextureSampling::new(false, false),
        )
        .expect("pumpkin overlay");

    renderer.set_hud_camera_overlays(vec![
        HudCameraOverlay {
            texture_id: "minecraft:textures/misc/pumpkinblur.png".to_string(),
            alpha: 1.0,
        },
        HudCameraOverlay {
            texture_id: "minecraft:textures/misc/powder_snow_outline.png".to_string(),
            alpha: 0.5,
        },
    ]);
    let draws = renderer.collect_hud_draws();
    assert_eq!(draws.commands.len(), 1);
    assert_eq!(draws.vertices.len(), 6);
    let corners = draws
        .vertices
        .iter()
        .map(|vertex| vertex.position)
        .collect::<Vec<_>>();
    assert!(corners.contains(&[-1.0, 1.0]));
    assert!(corners.contains(&[1.0, -1.0]));
    assert!(draws.vertices.iter().all(|vertex| vertex.tint[3] == 1.0));
    drop(draws);

    renderer.set_hud_camera_overlays(vec![HudCameraOverlay {
        texture_id: "minecraft:textures/misc/pumpkinblur.png".to_string(),
        alpha: 0.25,
    }]);
    assert!(renderer
        .collect_hud_draws()
        .vertices
        .iter()
        .all(|vertex| vertex.tint[3] == 0.25));

    // A non-positive or non-finite alpha never reaches the draw list, and
    // `hideGui` suppresses every camera overlay.
    renderer.set_hud_camera_overlays(vec![HudCameraOverlay {
        texture_id: "minecraft:textures/misc/pumpkinblur.png".to_string(),
        alpha: 0.0,
    }]);
    assert_eq!(renderer.collect_hud_draws().commands.len(), 0);
    renderer.set_hud_camera_overlays(vec![HudCameraOverlay {
        texture_id: "minecraft:textures/misc/pumpkinblur.png".to_string(),
        alpha: 1.0,
    }]);
    renderer.set_hide_gui(true);
    assert_eq!(renderer.collect_hud_draws().commands.len(), 0);
}

#[test]
fn spyglass_scope_draws_texture_and_masks_before_crosshair_and_obeys_hide_gui() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .upload_gui_texture(
            "textures/misc/spyglass_scope.png",
            2,
            2,
            &[0, 255, 0, 255].repeat(4),
            crate::StandaloneTextureSampling::default(),
        )
        .expect("scope");
    renderer
        .add_test_gui_sprite("hud/crosshair", 2, 2, &[255, 0, 0, 255].repeat(4))
        .expect("crosshair");
    renderer.set_hud_spyglass_scope(Some(HudSpyglassScope { scale: 0.5 }));

    let draws = renderer.collect_hud_draws();
    assert_eq!(draws.commands.len(), 6);
    assert_eq!(draws.vertices.len(), 36);
    let HudDrawCommand::Sprite {
        sprite: scope_sprite,
        ..
    } = &draws.commands[0]
    else {
        panic!("scope texture command");
    };
    assert!(std::ptr::eq(
        *scope_sprite,
        renderer
            .gui_texture("textures/misc/spyglass_scope.png")
            .unwrap()
    ));
    for command in &draws.commands[1..=4] {
        let HudDrawCommand::Fill { start, end } = command else {
            panic!("scope mask command");
        };
        assert!(draws.vertices[*start as usize..*end as usize]
            .iter()
            .all(|vertex| vertex.tint == [0.0, 0.0, 0.0, 1.0]));
    }
    let HudDrawCommand::InvertingSprite {
        sprite: crosshair_sprite,
        ..
    } = &draws.commands[5]
    else {
        panic!("crosshair command");
    };
    assert!(std::ptr::eq(
        *crosshair_sprite,
        renderer.gui_sprite("hud/crosshair").unwrap()
    ));
    drop(draws);

    renderer.set_hide_gui(true);
    assert!(renderer.collect_hud_draws().commands.is_empty());
    assert!(renderer.hud_spyglass_scope.is_some());
    renderer.set_hide_gui(false);
    renderer.set_hud_spyglass_scope(Some(HudSpyglassScope { scale: f32::NAN }));
    assert!(renderer.hud_spyglass_scope.is_none());
}

#[test]
fn stats_screen_sprite_catalog_and_states_match_vanilla() {
    assert_eq!(
        HudStatsSprite::ALL.map(HudStatsSprite::sprite_path),
        [
            "container/slot",
            "statistics/header",
            "statistics/sort_up",
            "statistics/sort_down",
            "statistics/block_mined",
            "statistics/item_broken",
            "statistics/item_crafted",
            "statistics/item_used",
            "statistics/item_picked_up",
            "statistics/item_dropped",
        ]
    );
    assert_eq!(
        (0..6)
            .map(|column| HudStatsSprite::column(column).expect("column"))
            .collect::<Vec<_>>(),
        vec![
            HudStatsSprite::BlockMined,
            HudStatsSprite::ItemBroken,
            HudStatsSprite::ItemCrafted,
            HudStatsSprite::ItemUsed,
            HudStatsSprite::ItemPickedUp,
            HudStatsSprite::ItemDropped,
        ]
    );
    assert_eq!(HudStatsSprite::column(6), None);
    assert_eq!(
        hud_stats_header_background_sprite(false),
        HudStatsSprite::Header
    );
    assert_eq!(
        hud_stats_header_background_sprite(true),
        HudStatsSprite::Slot
    );
    assert_eq!(hud_stats_sort_sprite(-1), HudStatsSprite::SortDown);
    assert_eq!(hud_stats_sort_sprite(1), HudStatsSprite::SortUp);
    assert_eq!(
        HudStatsScrollbarSprite::ALL.map(HudStatsScrollbarSprite::sprite_path),
        ["widget/scroller", "widget/scroller_background"]
    );
}

#[test]
fn stats_screen_scrollbar_and_scissor_match_vanilla_geometry() {
    let screen = HudStatsScreen {
        title: "Stats".to_string(),
        loading_text: "Loading".to_string(),
        is_loading: false,
        general_tab: "General".to_string(),
        items_tab: "Items".to_string(),
        mobs_tab: "Mobs".to_string(),
        active_tab: 0,
        items_tab_enabled: false,
        mobs_tab_enabled: false,
        rows: Vec::new(),
        item_rows: Vec::new(),
        item_column_labels: std::array::from_fn(|index| format!("Column {index}")),
        item_no_value: "-".to_string(),
        item_sort_column: None,
        item_sort_order: 0,
        item_hovered_column: None,
        item_focused_column: None,
        scroll_row: 0,
        scroll_amount: 70,
        max_scroll_amount: 140,
        scroller_height: 80,
        total_rows: 20,
        visible_rows: 12,
        row_height: 14,
        tooltip: None,
        done: "Done".to_string(),
        done_hovered: false,
        done_focused: false,
    };
    assert_eq!(
        hud_stats_scrollbar_rects(PhysicalSize::new(320, 240), &screen),
        Some((
            absolute_hud_rect(308.0, 24.0, 6, 183),
            absolute_hud_rect(308.0, 75.0, 6, 80),
        ))
    );
    let mut commands = Vec::new();
    push_hud_stats_list_scissor(&mut commands, PhysicalSize::new(320, 240));
    push_hud_stats_list_scissor_reset(&mut commands, PhysicalSize::new(320, 240));
    assert!(matches!(
        commands.as_slice(),
        [
            HudDrawCommand::Scissor {
                x: 0,
                y: 24,
                width: 320,
                height: 183,
            },
            HudDrawCommand::Scissor {
                x: 0,
                y: 0,
                width: 320,
                height: 240,
            }
        ]
    ));
}

#[test]
fn system_toast_sanitizes_text_and_visible_portion() {
    let toast = sanitize_hud_system_toast(HudSystemToast {
        title: "Low\ndisk".to_string(),
        message_lines: vec!["Might not\tsave".to_string()],
        width: 160,
        visible_portion: 2.0,
    })
    .unwrap();
    assert_eq!(toast.title, "Lowdisk");
    assert_eq!(toast.message_lines, ["Might notsave"]);
    assert_eq!(toast.visible_portion, 1.0);
    assert!(sanitize_hud_system_toast(HudSystemToast {
        title: "Low disk".to_string(),
        message_lines: vec!["Warning".to_string()],
        width: 160,
        visible_portion: f32::NAN,
    })
    .is_none());
    assert_eq!(
        sanitize_hud_system_toast(HudSystemToast {
            title: "Resource reload failed".to_string(),
            message_lines: Vec::new(),
            width: 160,
            visible_portion: 1.0,
        })
        .map(|toast| toast.message_lines),
        Some(Vec::new())
    );
}

#[test]
fn system_toast_uses_vanilla_width_height_and_right_edge_slide() {
    let toast = HudSystemToast {
        title: "Low disk space!".to_string(),
        message_lines: vec!["Might not be able to save the world.".to_string()],
        width: 160,
        visible_portion: 1.0,
    };
    let (_, x, width, height) =
        hud_system_toast_layout(PhysicalSize::new(320, 240), &toast, &HudFontGlyphMap::new());
    assert_eq!(width, 160);
    assert_eq!(height, 32);
    assert_eq!(x, 160.0);

    let (_, hidden_x, _, _) = hud_system_toast_layout(
        PhysicalSize::new(320, 240),
        &HudSystemToast {
            visible_portion: 0.0,
            ..toast
        },
        &HudFontGlyphMap::new(),
    );
    assert_eq!(hidden_x, 320.0);

    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'a',
        HudAsciiGlyph {
            advance: 6.0,
            width: 5,
            height: 8,
            ..HudAsciiGlyph::default()
        },
    );
    let (_, wide_x, wide_width, _) = hud_system_toast_layout(
        PhysicalSize::new(320, 240),
        &HudSystemToast {
            title: "a".to_string(),
            message_lines: vec!["a".repeat(30)],
            width: 210,
            visible_portion: 1.0,
        },
        &glyphs,
    );
    assert_eq!(wide_width, 210);
    assert_eq!(wide_x, 110.0);
}

#[test]
fn system_toast_instances_use_distinct_vanilla_slots() {
    let toast = HudSystemToast {
        title: "Download failed".to_string(),
        message_lines: Vec::new(),
        width: 160,
        visible_portion: 1.0,
    };
    let (first, _, _, first_height, first_offset) = hud_system_toast_instance_layout(
        PhysicalSize::new(320, 240),
        &toast,
        0,
        &HudFontGlyphMap::new(),
    );
    let (second, _, _, second_height, second_offset) = hud_system_toast_instance_layout(
        PhysicalSize::new(320, 240),
        &toast,
        1,
        &HudFontGlyphMap::new(),
    );

    assert_eq!(first_height, 32);
    assert_eq!(second_height, 32);
    assert_eq!(first_offset, 0.0);
    assert_eq!(second_offset, 32.0);
    assert_eq!(second.y, first.y + first_height as f32);
}

#[test]
fn multiline_system_toast_uses_frozen_lines_and_height_scaled_slot_position() {
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.insert_first_wins(
        'a',
        HudAsciiGlyph {
            advance: 6.0,
            width: 5,
            height: 8,
            ..HudAsciiGlyph::default()
        },
    );
    let toast = HudSystemToast {
        title: "Warning".to_string(),
        message_lines: vec!["a".repeat(33), "a".repeat(7)],
        width: 230,
        visible_portion: 1.0,
    };

    let (_, x, width, height) =
        hud_system_toast_layout(PhysicalSize::new(320, 240), &toast, &glyphs);
    let (second, _, _, _, second_offset) =
        hud_system_toast_instance_layout(PhysicalSize::new(320, 240), &toast, 1, &glyphs);

    assert_eq!(
        toast
            .message_lines
            .iter()
            .map(String::len)
            .collect::<Vec<_>>(),
        [33, 7]
    );
    assert_eq!(width, 230);
    assert_eq!(height, 44);
    assert_eq!(x, 90.0);
    assert_eq!(second_offset, 44.0);
    assert_eq!(second.y, 44.0);
    assert!(
        sanitize_hud_system_toast_instance(HudSystemToastInstance { toast, slot: 4 }).is_none(),
        "a 44-pixel system toast cannot begin in the final slot"
    );
}

#[test]
fn unified_toast_sanitizer_preserves_mixed_order_and_rejects_overlap() {
    let system = HudToast::System(HudSystemToastInstance {
        toast: HudSystemToast {
            title: "System".to_string(),
            message_lines: Vec::new(),
            width: 160,
            visible_portion: 1.0,
        },
        slot: 0,
    });
    let recipe = |slot| {
        HudToast::Recipe(HudRecipeToast {
            title: "Recipe".to_string(),
            description: "Description".to_string(),
            category_icon: None,
            category_block_model: None,
            unlocked_icon: None,
            unlocked_block_model: None,
            slot,
            visible_portion: 1.0,
        })
    };
    let mut occupied = [false; HUD_TOAST_SLOTS];
    let sanitized = vec![system, recipe(1), recipe(0)]
        .into_iter()
        .filter_map(|toast| sanitize_hud_toast(toast, &mut occupied))
        .collect::<Vec<_>>();
    assert!(matches!(
        sanitized.as_slice(),
        [HudToast::System(_), HudToast::Recipe(_)]
    ));
}

#[test]
fn tutorial_toast_height_slot_layout_and_progress_follow_vanilla() {
    let toast = sanitize_hud_tutorial_toast(HudTutorialToast {
        icon: HudTutorialToastIcon::MovementKeys,
        lines: HudTutorialToastLines::from_wrapped(vec![
            vec![
                HudStyledTextRun {
                    shadow_color: None,
                    text: "Move with W".to_string(),
                    style: HudTextStyle {
                        bold: true,
                        ..HudTextStyle::default()
                    },
                    color: Some(0x50_0050),
                },
                HudStyledTextRun::plain("\u{7}"),
            ],
            vec![HudStyledTextRun::plain("continued")],
            vec![HudStyledTextRun::plain("Jump with Space")],
        ]),
        slot: 1,
        visible_portion: 0.25,
        progress: Some(1.5),
        progress_increasing: true,
    })
    .unwrap();
    assert_eq!(toast.lines.height(), 43);
    assert!(toast.lines.as_slice()[0][0].style.bold);
    assert_eq!(toast.lines.as_slice()[0][0].color, Some(0x50_0050));
    assert_eq!(
        toast.lines.as_slice()[0].len(),
        1,
        "control-only runs are removed"
    );
    assert_eq!(toast.progress, Some(1.0));
    assert_eq!(
        hud_tutorial_toast_rect(PhysicalSize::new(320, 240), &toast),
        absolute_hud_rect(280.0, 43.0, 160, 43)
    );
    assert!(
        sanitize_hud_tutorial_toast(HudTutorialToast { slot: 4, ..toast }).is_none(),
        "a 43-pixel tutorial toast needs two contiguous slots"
    );
}

#[test]
fn advancement_toast_block_icon_uses_after_blur_gui_residency() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    let model = test_hud_block_item_model_with_quads(
        vec![crate::item_models::ItemModelQuad {
            corners: [
                [0.0, 0.0, 8.0],
                [16.0, 0.0, 8.0],
                [16.0, 16.0, 8.0],
                [0.0, 16.0, 8.0],
            ],
            uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            tint: [1.0, 1.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            shade: 1.0,
            translucent: false,
        }],
        glam::Mat4::IDENTITY,
        GuiItemLightingEntry::Items3d,
        false,
    );
    renderer.set_hud_advancement_toasts(vec![HudAdvancementToast {
        frame_label: "Advancement Made!".to_string(),
        frame_label_color: 0xFF_FF_00,
        title_runs: vec![HudStyledTextRun::plain("Stone Age")],
        icon: HudItemIcon::single(HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }),
        block_model: Some(model.clone()),
        slot: 2,
        visible_portion: 1.0,
        fully_visible_ms: 0,
    }]);
    renderer.set_hud_recipe_toasts(vec![HudRecipeToast {
        title: "New Recipe(s) Unlocked!".to_string(),
        description: "Check your recipe book".to_string(),
        category_icon: None,
        category_block_model: Some(model.clone()),
        unlocked_icon: None,
        unlocked_block_model: Some(model),
        slot: 3,
        visible_portion: 1.0,
    }]);
    renderer.set_hud_screen_blur(true);

    let mut residency = ItemModelResidencyFrame::default();
    renderer.append_hud_block_item_residency(&mut residency);
    assert_eq!(residency.instances.len(), 3);
    assert!(residency
        .instances
        .iter()
        .all(|instance| instance.target == ItemModelDrawTarget::GuiAfterBlur));
}

#[test]
fn overlay_message_alpha_matches_vanilla_ramp_and_cap() {
    // Vanilla Gui.extractOverlayMessage:313-316: alpha = (int)(t*255/20),
    // capped at 255; the draw gate is alpha > 0.
    assert_eq!(hud_overlay_message_alpha(60.0), 255);
    assert_eq!(hud_overlay_message_alpha(20.0), 255);
    assert_eq!(hud_overlay_message_alpha(10.0), 127);
    assert_eq!(hud_overlay_message_alpha(0.05), 0);
    assert!(hud_overlay_message_alpha(-0.5) <= 0);
}

#[test]
fn overlay_message_rainbow_rgb_is_deterministic_and_matches_the_hsv_quirk() {
    // hue = t / 50: remaining-time driven, so the same
    // frame state always yields the same colour (no wall clock).
    let mid = hud_overlay_message_rainbow_rgb(25.0);
    // hue 0.5 -> h=3, f=0: (p, q, v) = (0.18, 0.6, 0.6) -> (45, 153, 153).
    assert_eq!(mid, [45.0 / 255.0, 153.0 / 255.0, 153.0 / 255.0]);
    assert_eq!(mid, hud_overlay_message_rainbow_rgb(25.0));
    // hue 1.2 -> hue*6 = 7.2: vanilla wraps h to 1 but keeps f = 6.2, so
    // q goes negative and clamps to 0 (Mth.hsvToArgb quirk kept verbatim).
    assert_eq!(
        hud_overlay_message_rainbow_rgb(60.0),
        [0.0, 153.0 / 255.0, 45.0 / 255.0]
    );
}

#[test]
fn selected_item_name_draw_centers_above_the_hotbar_and_drops_without_hearts() {
    let fonts = HudFontGlyphBook::from(styled_test_glyphs());
    let surface = PhysicalSize::new(320, 240);
    let state = HudSelectedItemName {
        runs: vec![HudComponentTextRun::plain("ab")],
        highlight_ticks: 40,
        raised: false,
    };
    let draw = hud_selected_item_name_draw(&state, &fonts, surface, 0.0).expect("visible");
    assert_eq!(draw.origin, (154.0, 181.0));
    assert_eq!(draw.scale, 1.0);
    assert_eq!(draw.tint, [1.0, 1.0, 1.0, 1.0]);

    let raised = HudSelectedItemName {
        raised: true,
        ..state.clone()
    };
    assert_eq!(
        hud_selected_item_name_draw(&raised, &fonts, surface, 0.0)
            .expect("visible")
            .origin,
        (154.0, 195.0)
    );

    let fading = HudSelectedItemName {
        highlight_ticks: 5,
        ..state.clone()
    };
    assert_eq!(
        hud_selected_item_name_draw(&fading, &fonts, surface, 0.0)
            .expect("visible")
            .tint[3],
        128.0 / 255.0
    );

    let expired = HudSelectedItemName {
        highlight_ticks: 0,
        ..state
    };
    assert!(hud_selected_item_name_draw(&expired, &fonts, surface, 0.0).is_none());
}

#[test]
fn action_bar_draw_centers_above_the_hotbar_and_fades_out() {
    let fonts = HudFontGlyphBook::from(styled_test_glyphs());
    let surface = PhysicalSize::new(320, 240);
    let state = HudActionBarText {
        runs: vec![HudComponentTextRun::plain("ab")],
        remaining_ticks: 60,
        partial_tick: 0.5,
        animate_color: false,
    };
    let draw = hud_action_bar_text_draw(&state, &fonts, surface, 0.0).expect("visible");
    // (guiWidth/2 - width/2, guiHeight - 68 - 4) = (160 - 6, 240 - 72).
    assert_eq!(draw.origin, (154.0, 168.0));
    assert_eq!(draw.scale, 1.0);
    // t = 59.5 -> alpha 758 capped at 255 -> opaque white.
    assert_eq!(draw.tint, [1.0, 1.0, 1.0, 1.0]);

    // t = 10 -> alpha 127.
    let fading = HudActionBarText {
        remaining_ticks: 10,
        partial_tick: 0.0,
        ..state.clone()
    };
    assert_eq!(
        hud_action_bar_text_draw(&fading, &fonts, surface, 0.0)
            .expect("fading")
            .tint,
        [1.0, 1.0, 1.0, 127.0 / 255.0]
    );

    // alpha == 0 is dropped (vanilla `if (alpha > 0)`).
    let nearly_out = HudActionBarText {
        remaining_ticks: 1,
        partial_tick: 0.999,
        ..state.clone()
    };
    assert_eq!(
        hud_action_bar_text_draw(&nearly_out, &fonts, surface, 0.0),
        None
    );
    // An expired timer never draws.
    let expired = HudActionBarText {
        remaining_ticks: 0,
        partial_tick: 0.0,
        ..state.clone()
    };
    assert_eq!(
        hud_action_bar_text_draw(&expired, &fonts, surface, 0.0),
        None
    );

    // Jukebox rainbow: hue from remaining time, fade alpha kept.
    let rainbow = HudActionBarText {
        remaining_ticks: 25,
        partial_tick: 0.0,
        animate_color: true,
        ..state.clone()
    };
    assert_eq!(
        hud_action_bar_text_draw(&rainbow, &fonts, surface, 0.0)
            .expect("rainbow")
            .tint,
        [45.0 / 255.0, 153.0 / 255.0, 153.0 / 255.0, 1.0]
    );
}

#[test]
fn action_bar_component_runs_select_custom_font_for_width_geometry_and_obfuscation() {
    let fonts = component_test_font_book();
    let runs = vec![
        HudComponentTextRun::plain("a"),
        HudComponentTextRun {
            visual: HudStyledTextRun {
                shadow_color: None,
                text: "a".to_string(),
                style: HudTextStyle {
                    obfuscated: true,
                    ..HudTextStyle::default()
                },
                color: Some(0x12_34_56),
            },
            font: Some("example:wide".to_string()),
        },
        HudComponentTextRun {
            visual: HudStyledTextRun::plain("a"),
            font: Some("example:missing".to_string()),
        },
    ];
    assert_eq!(hud_component_runs_width(&runs, &fonts), Some(22));
    let state = HudActionBarText {
        runs: runs.clone(),
        remaining_ticks: 60,
        partial_tick: 0.0,
        animate_color: false,
    };
    assert_eq!(
        hud_action_bar_text_draw(&state, &fonts, PhysicalSize::new(320, 240), 0.0)
            .expect("visible")
            .origin,
        (149.0, 168.0)
    );

    let geometry = hud_component_text_geometry(
        &runs,
        &fonts,
        7,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    assert_eq!(geometry.glyph_quads.len(), 3);
    assert_eq!(geometry.glyph_quads[0].0.uv.min[0], 0.1);
    assert_eq!(geometry.glyph_quads[0].0.corners[0][0], 0.0);
    assert!(
        geometry.glyph_quads[1].0.uv.min[0] >= 0.6,
        "obfuscated glyph must come from the selected custom font"
    );
    assert_eq!(geometry.glyph_quads[1].0.corners[0][0], 6.0);
    assert_eq!(
        geometry.glyph_quads[1].1,
        [
            0x12 as f32 / 255.0,
            0x34 as f32 / 255.0,
            0x56 as f32 / 255.0,
            1.0
        ]
    );
    assert_eq!(
        geometry.glyph_quads[2].0.uv.min[0], 0.1,
        "unknown font id must use the selected default font"
    );
    assert_eq!(geometry.glyph_quads[2].0.corners[0][0], 16.0);
}

#[test]
fn action_bar_and_title_draw_the_resolved_runs_that_centred_them() {
    // `bbb-client`'s `action_bar_and_title_runs_consume_legacy_codes_with_the_root_style_as_reset`
    // pins the projection output for `AB§cCD§rEF§?GH§`; this is the draw the
    // same list produces: `Gui.extractOverlayMessage` centres on
    // `font.width(Component)` and then draws those glyphs.
    let fonts = HudFontGlyphBook::from(styled_test_glyphs());
    let surface = PhysicalSize::new(320, 240);
    let underlined = HudTextStyle {
        underlined: true,
        ..HudTextStyle::default()
    };
    let runs = vec![
        HudComponentTextRun {
            visual: HudStyledTextRun {
                text: "a".into(),
                style: underlined,
                ..HudStyledTextRun::plain("")
            },
            font: None,
        },
        HudComponentTextRun {
            visual: HudStyledTextRun {
                text: "b".into(),
                style: HudTextStyle::default(),
                ..HudStyledTextRun::plain("")
            },
            font: None,
        },
        HudComponentTextRun {
            visual: HudStyledTextRun {
                text: "ab".into(),
                style: underlined,
                ..HudStyledTextRun::plain("")
            },
            font: None,
        },
    ];
    // Four drawn glyphs, six pixels each: the two codes and the trailing `§`
    // have no advance, so the overlay centres on 24 and not on the raw string.
    assert_eq!(hud_component_runs_width(&runs, &fonts), Some(24));
    let state = HudActionBarText {
        runs: runs.clone(),
        remaining_ticks: 60,
        partial_tick: 0.0,
        animate_color: false,
    };
    assert_eq!(
        hud_action_bar_text_draw(&state, &fonts, surface, 0.0)
            .expect("visible")
            .origin,
        (148.0, 168.0)
    );

    let geometry = hud_component_text_geometry(
        &runs,
        &fonts,
        9,
        (0.0, 0.0),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    let advances = geometry
        .glyph_quads
        .iter()
        .map(|quad| quad.0.corners[0][0])
        .collect::<Vec<_>>();
    assert_eq!(
        advances,
        [0.0, 6.0, 12.0, 18.0],
        "the glyph pen must advance over drawn characters only"
    );

    // The title is the same list scaled 4x; its origin is what the width decides.
    let title = HudTitleText {
        title_runs: runs.clone(),
        subtitle_runs: vec![HudComponentTextRun::plain("a")],
        remaining_ticks: 40,
        fade_in: 5,
        stay: 40,
        fade_out: 15,
        partial_tick: 0.0,
    };
    let draws = hud_title_text_draws(&title, &fonts, surface, 0.0);
    assert_eq!(draws.len(), 2);
    assert_eq!(draws[0].scale, HUD_TITLE_TEXT_SCALE);
    // guiWidth/2 - resolvedWidth*4/2 = 160 - 48, and the subtitle at scale 2.
    assert_eq!(draws[0].origin.0, 112.0);
    assert_eq!(draws[1].origin.0, 154.0);
}

#[test]
fn action_bar_collect_draws_uses_the_full_font_book_for_both_text_passes() {
    let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
        return;
    };
    renderer
        .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], component_test_font_book())
        .expect("font atlas");
    renderer.set_hud_action_bar_text(Some(HudActionBarText {
        runs: vec![
            HudComponentTextRun::plain("a"),
            HudComponentTextRun {
                visual: HudStyledTextRun::plain("a"),
                font: Some("example:wide".to_string()),
            },
            HudComponentTextRun {
                visual: HudStyledTextRun::plain("a"),
                font: Some("example:missing".to_string()),
            },
        ],
        remaining_ticks: 60,
        partial_tick: 0.0,
        animate_color: false,
    }));

    let draws = renderer.collect_hud_draws();
    let first_u = draws
        .vertices
        .as_chunks::<6>()
        .0
        .iter()
        .map(|quad| quad[0].uv[0])
        .collect::<Vec<_>>();
    assert_eq!(first_u, [0.1, 0.1, 0.6, 0.6, 0.1, 0.1]);
}

#[test]
fn title_draws_center_and_scale_title_4x_and_subtitle_2x() {
    let fonts = HudFontGlyphBook::from(styled_test_glyphs());
    let surface = PhysicalSize::new(320, 240);
    let state = HudTitleText {
        title_runs: vec![HudComponentTextRun::plain("ab")],
        subtitle_runs: vec![HudComponentTextRun::plain("a")],
        remaining_ticks: 50,
        fade_in: 10,
        stay: 70,
        fade_out: 20,
        partial_tick: 0.25,
    };
    let draws = hud_title_text_draws(&state, &fonts, surface, 0.5);
    assert_eq!(draws.len(), 2);
    // Title: center (160, 120) + 4 * (-12/2, -10) = (136, 80).
    assert_eq!(draws[0].origin, (136.0, 80.0));
    assert_eq!(draws[0].scale, 4.0);
    assert_eq!(draws[0].backdrop_opacity, 0.5);
    // Stay window -> full alpha.
    assert_eq!(draws[0].tint, [1.0, 1.0, 1.0, 1.0]);
    // Subtitle: center + 2 * (-6/2, 5) = (154, 130), same tint.
    assert_eq!(draws[1].origin, (154.0, 130.0));
    assert_eq!(draws[1].scale, 2.0);
    assert_eq!(draws[1].backdrop_opacity, 0.5);
    assert_eq!(draws[1].tint, draws[0].tint);

    // No subtitle set -> only the title line.
    let title_only = HudTitleText {
        subtitle_runs: Vec::new(),
        ..state.clone()
    };
    assert_eq!(
        hud_title_text_draws(&title_only, &fonts, surface, 0.0).len(),
        1
    );

    // Expired timer -> nothing.
    let expired = HudTitleText {
        remaining_ticks: 0,
        ..state.clone()
    };
    assert!(hud_title_text_draws(&expired, &fonts, surface, 0.0).is_empty());

    // The very first frame after SetTitleText (t == fadeIn+stay+fadeOut at
    // partial 0) computes fade-in alpha 0 and is dropped, like vanilla.
    let first_frame = HudTitleText {
        remaining_ticks: 100,
        partial_tick: 0.0,
        ..state.clone()
    };
    assert!(hud_title_text_draws(&first_frame, &fonts, surface, 0.0).is_empty());
}

#[test]
fn title_custom_font_controls_scaled_centering_and_unknown_font_falls_back() {
    let fonts = component_test_font_book();
    let state = HudTitleText {
        title_runs: vec![HudComponentTextRun {
            visual: HudStyledTextRun::plain("a"),
            font: Some("example:wide".to_string()),
        }],
        subtitle_runs: vec![HudComponentTextRun {
            visual: HudStyledTextRun::plain("a"),
            font: Some("example:missing".to_string()),
        }],
        remaining_ticks: 50,
        fade_in: 10,
        stay: 70,
        fade_out: 20,
        partial_tick: 0.0,
    };

    let draws = hud_title_text_draws(&state, &fonts, PhysicalSize::new(320, 240), 0.0);
    assert_eq!(draws.len(), 2);
    // Custom width 10: center + 4 * (-10/2, -10).
    assert_eq!(draws[0].origin, (140.0, 80.0));
    // Unknown font falls back to default width 6:
    // center + 2 * (-6/2, 5).
    assert_eq!(draws[1].origin, (154.0, 130.0));
}

#[test]
fn title_alpha_follows_the_fade_in_stay_fade_out_windows() {
    let base = HudTitleText {
        title_runs: vec![HudComponentTextRun::plain("ab")],
        subtitle_runs: Vec::new(),
        remaining_ticks: 50,
        fade_in: 10,
        stay: 70,
        fade_out: 20,
        partial_tick: 0.0,
    };
    // Stay window (fade_out < remaining <= fade_out + stay): full alpha.
    assert_eq!(hud_title_alpha(&base), 255);
    // Fade in (remaining > fade_out + stay): (total - t) * 255 / fade_in;
    // t = 94.5 -> 5.5 * 25.5 = 140.25 -> 140.
    let fade_in = HudTitleText {
        remaining_ticks: 95,
        partial_tick: 0.5,
        ..base.clone()
    };
    assert_eq!(hud_title_alpha(&fade_in), 140);
    // Fade out (remaining <= fade_out): t * 255 / fade_out;
    // t = 9.75 -> 9.75 * 12.75 = 124.3125 -> 124.
    let fade_out = HudTitleText {
        remaining_ticks: 10,
        partial_tick: 0.25,
        ..base.clone()
    };
    assert_eq!(hud_title_alpha(&fade_out), 124);
    // Freshly set title at partial 0: fade-in alpha starts at 0.
    let fresh = HudTitleText {
        remaining_ticks: 100,
        partial_tick: 0.0,
        ..base.clone()
    };
    assert_eq!(hud_title_alpha(&fresh), 0);
}

#[test]
fn food_sprite_variant_prefers_the_hunger_icon_only_under_the_effect() {
    // Under the Hunger effect the variant wins, with the base as fallback
    // when the variant is not loaded.
    assert_eq!(
        hud_food_sprite_variant(true, Some("hunger"), Some("base")),
        Some("hunger")
    );
    assert_eq!(
        hud_food_sprite_variant(true, None, Some("base")),
        Some("base")
    );
    assert_eq!(hud_food_sprite_variant::<&str>(true, None, None), None);
    // Without the effect the base sprite always wins, even if a hunger
    // variant happens to be loaded.
    assert_eq!(
        hud_food_sprite_variant(false, Some("hunger"), Some("base")),
        Some("base")
    );
    assert_eq!(hud_food_sprite_variant(false, Some("hunger"), None), None);
}

#[test]
fn boss_bar_layers_follow_vanilla_background_then_cropped_fill_order() {
    let plain = HudBossBar {
        name_runs: Vec::new(),
        progress: 0.5,
        color: HudBossBarColor::Red,
        overlay: HudBossBarOverlay::Progress,
    };
    // Progress overlay: colored background, then the fill cropped to
    // lerpDiscrete(0.5) = 91.
    assert_eq!(
        hud_boss_bar_layers(&plain),
        vec![
            HudBossBarLayer {
                sheet: HudBossBarSheet::ColorBackground(HudBossBarColor::Red),
                width: 182,
            },
            HudBossBarLayer {
                sheet: HudBossBarSheet::ColorProgress(HudBossBarColor::Red),
                width: 91,
            },
        ]
    );

    // Notched overlays double both the background and the fill
    // sharing the same crop width.
    let notched = HudBossBar {
        overlay: HudBossBarOverlay::Notched10,
        ..plain.clone()
    };
    assert_eq!(
        hud_boss_bar_layers(&notched),
        vec![
            HudBossBarLayer {
                sheet: HudBossBarSheet::ColorBackground(HudBossBarColor::Red),
                width: 182,
            },
            HudBossBarLayer {
                sheet: HudBossBarSheet::NotchedBackground(HudBossBarOverlay::Notched10),
                width: 182,
            },
            HudBossBarLayer {
                sheet: HudBossBarSheet::ColorProgress(HudBossBarColor::Red),
                width: 91,
            },
            HudBossBarLayer {
                sheet: HudBossBarSheet::NotchedProgress(HudBossBarOverlay::Notched10),
                width: 91,
            },
        ]
    );

    // Zero progress skips both fill layers (vanilla `if (width > 0)`,
    // BossHealthOverlay), keeping the two backgrounds.
    let empty = HudBossBar {
        progress: 0.0,
        overlay: HudBossBarOverlay::Notched20,
        ..plain
    };
    let layers = hud_boss_bar_layers(&empty);
    assert_eq!(layers.len(), 2);
    assert_eq!(
        layers[1].sheet,
        HudBossBarSheet::NotchedBackground(HudBossBarOverlay::Notched20)
    );
}

#[test]
fn boss_bar_draws_preserve_styled_name_runs() {
    let glyphs = styled_test_glyphs();
    let styled_name = HudStyledTextRun {
        shadow_color: None,
        text: "Boss".to_string(),
        style: HudTextStyle {
            bold: true,
            underlined: true,
            ..HudTextStyle::default()
        },
        color: Some(0xFF_55_55),
    };
    let bars = [HudBossBar {
        name_runs: vec![styled_name.clone()],
        progress: 1.0,
        color: HudBossBarColor::Purple,
        overlay: HudBossBarOverlay::Progress,
    }];

    let draws = hud_boss_bar_draws(&bars, &glyphs, PhysicalSize::new(320, 240));

    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].name.runs, &[styled_name]);
}
