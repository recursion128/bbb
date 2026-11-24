use super::*;

#[test]
fn item_pop_pose_squeezes_horizontally_and_stretches_vertically_about_the_vanilla_pivot() {
    let rect = absolute_hud_rect(100.0, 200.0, 16, 16);
    let identity = hud_item_pop_corners(rect, 0.0);
    assert_eq!(
        identity,
        [
            [100.0, 200.0],
            [100.0, 216.0],
            [116.0, 216.0],
            [116.0, 200.0]
        ]
    );
    assert_eq!(hud_item_pop_corners(rect, -1.0), identity);
    assert_eq!(hud_item_pop_corners(rect, f32::NAN), identity);

    // `squeeze = 1 + 5/5 = 2`, so x scales by 1/2 about x + 8 = 108 and y
    // scales by (2 + 1)/2 = 1.5 about y + 12 = 212.
    let [top_left, bottom_left, bottom_right, top_right] = hud_item_pop_corners(rect, 5.0);
    assert_eq!(top_left, [104.0, 194.0]);
    assert_eq!(bottom_left, [104.0, 218.0]);
    assert_eq!(bottom_right, [112.0, 218.0]);
    assert_eq!(top_right, [112.0, 194.0]);

    let placement = hud_item_pop_placement(rect, 5.0);
    assert_eq!(hud_item_pop_placement(rect, 0.0), Mat4::IDENTITY);
    let posed = placement.transform_point3(Vec3::new(100.0, 200.0, 0.0));
    assert!((posed.x - 104.0).abs() < 1e-4);
    assert!((posed.y - 194.0).abs() < 1e-4);
    let pivot = placement.transform_point3(Vec3::new(108.0, 212.0, 0.0));
    assert!((pivot.x - 108.0).abs() < 1e-4);
    assert!((pivot.y - 212.0).abs() < 1e-4);
}

#[test]
fn offhand_slot_layout_mirrors_vanilla_hotbar_offsets_on_both_sides() {
    let size = PhysicalSize::new(320, 240);
    let hotbar = hotbar_hud_rect(size, 182, 22);
    assert_eq!(hud_rect_bounds(hotbar), (69.0, 218.0, 251.0, 240.0));

    let left = hotbar_offhand_hud_rect(size, true, 29, 24);
    assert_eq!(hud_rect_bounds(left), (40.0, 217.0, 69.0, 241.0));
    let right = hotbar_offhand_hud_rect(size, false, 29, 24);
    assert_eq!(hud_rect_bounds(right), (251.0, 217.0, 280.0, 241.0));

    let left_item = hotbar_offhand_item_hud_rect(size, true);
    assert_eq!(hud_rect_bounds(left_item), (43.0, 221.0, 59.0, 237.0));
    let right_item = hotbar_offhand_item_hud_rect(size, false);
    assert_eq!(hud_rect_bounds(right_item), (261.0, 221.0, 277.0, 237.0));
}

#[test]
fn hotbar_attack_indicator_layout_fills_upward_on_the_main_arm_side() {
    let size = PhysicalSize::new(320, 240);
    let right_handed = hud_hotbar_attack_indicator_layout(size, 0.5, false);
    assert_eq!(
        hud_rect_bounds(right_handed.background),
        (257.0, 220.0, 275.0, 238.0)
    );
    assert_eq!(
        hud_rect_bounds(right_handed.progress),
        (257.0, 229.0, 275.0, 238.0)
    );
    assert_eq!(
        right_handed.progress_uv,
        HudUvRect {
            min: [0.0, 0.5],
            max: [1.0, 1.0],
        }
    );

    let left_handed = hud_hotbar_attack_indicator_layout(size, 0.5, true);
    assert_eq!(
        hud_rect_bounds(left_handed.background),
        (47.0, 220.0, 65.0, 238.0)
    );

    let empty = hud_hotbar_attack_indicator_layout(size, 0.0, false);
    assert_eq!(empty.progress.height, 0);
    assert_eq!(empty.progress_uv.min[1], 1.0);
    let nearly_full = hud_hotbar_attack_indicator_layout(size, 0.999, false);
    assert_eq!(nearly_full.progress.height, 18);
    assert_eq!(nearly_full.progress_uv.min[1], 0.0);
}

#[test]
fn crosshair_attack_indicator_layout_matches_vanilla_pixels_and_progress_crop() {
    let layout = hud_crosshair_attack_indicator_layout(PhysicalSize::new(321, 241), 0.5);
    assert_eq!(hud_rect_bounds(layout.full), (152.0, 129.0, 168.0, 145.0));
    assert_eq!(
        hud_rect_bounds(layout.background),
        (152.0, 129.0, 168.0, 133.0)
    );
    assert_eq!(
        hud_rect_bounds(layout.progress),
        (152.0, 129.0, 160.0, 133.0)
    );
    assert_eq!(
        layout.progress_uv,
        HudUvRect {
            min: [0.0, 0.0],
            max: [0.5, 1.0],
        }
    );

    let nearly_full = hud_crosshair_attack_indicator_layout(PhysicalSize::new(320, 240), 0.999);
    assert_eq!(nearly_full.progress.width, 16);
    assert_eq!(nearly_full.progress_uv.max[0], 1.0);
}

#[test]
fn spyglass_scope_layout_centers_square_and_preserves_vanilla_mask_order() {
    let layout = hud_spyglass_scope_layout(PhysicalSize::new(1920, 1080), 0.5);
    assert_eq!(
        layout.texture,
        HudRect {
            x: 690.0,
            y: 270.0,
            width: 540,
            height: 540,
        }
    );
    assert_eq!(
        layout.masks,
        [
            HudRect {
                x: 0.0,
                y: 810.0,
                width: 1920,
                height: 270,
            },
            HudRect {
                x: 0.0,
                y: 0.0,
                width: 1920,
                height: 270,
            },
            HudRect {
                x: 0.0,
                y: 270.0,
                width: 690,
                height: 540,
            },
            HudRect {
                x: 1230.0,
                y: 270.0,
                width: 690,
                height: 540,
            },
        ]
    );

    let oversized = hud_spyglass_scope_layout(PhysicalSize::new(1920, 1080), 1.125);
    assert_eq!(
        oversized.texture,
        HudRect {
            x: 352.0,
            y: -67.0,
            width: 1215,
            height: 1215,
        }
    );
    assert_eq!(
        oversized.masks,
        [
            HudRect {
                x: 0.0,
                y: 1080.0,
                width: 1920,
                height: 68,
            },
            HudRect {
                x: 0.0,
                y: -67.0,
                width: 1920,
                height: 67,
            },
            HudRect {
                x: 0.0,
                y: -67.0,
                width: 352,
                height: 1215,
            },
            HudRect {
                x: 1567.0,
                y: -67.0,
                width: 353,
                height: 1215,
            },
        ]
    );

    let tall = hud_spyglass_scope_layout(PhysicalSize::new(800, 1200), 0.5);
    assert_eq!(
        tall.texture,
        HudRect {
            x: 200.0,
            y: 400.0,
            width: 400,
            height: 400,
        }
    );
    assert_eq!(tall.masks[0].height, 400);
    assert_eq!(tall.masks[1].height, 400);
    assert_eq!(tall.masks[2].width, 200);
    assert_eq!(tall.masks[3].width, 200);
}

#[test]
fn hud_rect_intersection_uv_span_returns_visible_rect_and_source_span() {
    let rect = HudRect {
        x: 100.0,
        y: 50.0,
        width: 16,
        height: 16,
    };
    let scissor = HudRect {
        x: 108.0,
        y: 46.0,
        width: 20,
        height: 12,
    };

    let (visible, uv_min, uv_max) = hud_rect_intersection_uv_span(rect, scissor).unwrap();

    assert_eq!(
        visible,
        HudRect {
            x: 108.0,
            y: 50.0,
            width: 8,
            height: 8,
        }
    );
    assert_eq!(uv_min, [0.5, 0.0]);
    assert_eq!(uv_max, [1.0, 0.5]);
    assert!(hud_rect_intersection_uv_span(
        rect,
        HudRect {
            x: 116.0,
            y: 50.0,
            width: 10,
            height: 10,
        }
    )
    .is_none());
}

#[test]
fn gui_item_slot_placement_respects_non_square_item_rect_height() {
    let placement = gui_item_slot_placement(HudRect {
        x: 10.0,
        y: 20.0,
        width: 16,
        height: 24,
    });

    let top_left = placement * glam::Vec4::new(-0.5, 0.5, 0.0, 1.0);
    let bottom_right = placement * glam::Vec4::new(0.5, -0.5, 0.0, 1.0);
    assert_f32_near(top_left.x, 10.0);
    assert_f32_near(top_left.y, 20.0);
    assert_f32_near(bottom_right.x, 26.0);
    assert_f32_near(bottom_right.y, 44.0);
}

#[test]
fn hud_locator_layout_matches_vanilla_dot_and_arrow_positions() {
    let surface_size = PhysicalSize::new(320, 240);
    assert_eq!(
        hud_locator_marker_rect(surface_size, 0.0),
        HudRect {
            x: 156.0,
            y: 209.0,
            width: 9,
            height: 9,
        }
    );
    assert_eq!(hud_locator_marker_rect(surface_size, 60.0).x, 242.0);
    assert_eq!(
        hud_locator_arrow_rect(surface_size, 0.0, true),
        HudRect {
            x: 157.0,
            y: 205.0,
            width: 7,
            height: 5,
        }
    );
    assert_eq!(hud_locator_arrow_rect(surface_size, 0.0, false).y, 217.0);
}

#[test]
fn hud_boss_bar_rows_stack_and_truncate_at_a_third_of_the_screen() {
    // guiHeight / 3 = 80: rows 12, 31, 50, 69 fit; after drawing the
    // fourth bar the offset reaches 88 >= 80 and the rest is dropped.
    let surface_size = PhysicalSize::new(320, 240);
    assert_eq!(hud_boss_bar_rows(surface_size, 0), Vec::<i32>::new());
    assert_eq!(hud_boss_bar_rows(surface_size, 2), vec![12, 31]);
    assert_eq!(hud_boss_bar_rows(surface_size, 6), vec![12, 31, 50, 69]);
    // Vanilla draws first and checks after, so one bar always renders
    // even when y=12 already exceeds the cutoff.
    assert_eq!(hud_boss_bar_rows(PhysicalSize::new(320, 30), 3), vec![12]);
}

#[test]
fn hud_boss_bar_progress_width_matches_vanilla_lerp_discrete() {
    // Mth.lerpDiscrete(p, 0, 182) = floor(p * 181) + (p > 0 ? 1 : 0).
    assert_eq!(hud_boss_bar_progress_width(0.0), 0);
    assert_eq!(hud_boss_bar_progress_width(0.001), 1);
    assert_eq!(hud_boss_bar_progress_width(0.5), 91);
    assert_eq!(hud_boss_bar_progress_width(1.0), 182);
    // Out-of-range/non-finite inputs (clamped upstream by the setter)
    // stay inside the sheet.
    assert_eq!(hud_boss_bar_progress_width(-1.0), 0);
    assert_eq!(hud_boss_bar_progress_width(2.0), 182);
    assert_eq!(hud_boss_bar_progress_width(f32::NAN), 0);

    // The crop UV is the left `width / 182` band of the sheet.
    assert_eq!(hud_boss_bar_fill_uv(91).min, [0.0, 0.0]);
    assert_eq!(hud_boss_bar_fill_uv(91).max, [0.5, 1.0]);
    assert_eq!(hud_boss_bar_fill_uv(182).max, [1.0, 1.0]);
    assert_eq!(hud_boss_bar_fill_uv(0).max, [0.0, 1.0]);
}

#[test]
fn hud_player_heart_instances_split_half_and_empty() {
    let surface_size = PhysicalSize::new(1280, 720);
    // ceil(15) = 15 half-hearts: 7 full + 1 half (halves 0..=12 full, 14
    // half), the remaining containers draw background only.
    let fills = fill_hearts(&hud_player_heart_instances(
        surface_size,
        player_health(15.0),
    ));
    assert_eq!(fills.iter().filter(|i| !i.half).count(), 7);
    assert_eq!(fills.iter().filter(|i| i.half).count(), 1);
    assert!(fills.iter().all(|i| i.kind == HudHeartKind::Normal));

    // ceil(0.5) = 1 -> a single half heart; ceil rounds fractional health up.
    let fills = fill_hearts(&hud_player_heart_instances(
        surface_size,
        player_health(0.5),
    ));
    assert_eq!(fills.len(), 1);
    assert!(fills[0].half && fills[0].kind == HudHeartKind::Normal);

    // Zero health: only the 10 container backgrounds, no fills.
    let instances = hud_player_heart_instances(surface_size, player_health(0.0));
    assert_eq!(container_hearts(&instances).len(), 10);
    assert!(fill_hearts(&instances).is_empty());
}

#[test]
fn hud_player_heart_instances_never_blink_absorption_fills() {
    let surface_size = PhysicalSize::new(1280, 720);
    let mut health = player_health(20.0);
    health.absorption = 2.0;
    health.blinking = true;
    let instances = hud_player_heart_instances(surface_size, health);
    let absorbing = instances
        .iter()
        .find(|instance| instance.kind == HudHeartKind::Absorbing)
        .expect("absorption fill");
    assert!(!absorbing.blinking);
    assert!(container_hearts(&instances)
        .iter()
        .all(|instance| instance.blinking));
}

#[test]
fn hud_player_heart_instances_stack_multiple_rows() {
    let surface_size = PhysicalSize::new(1280, 720);
    let mut health = player_health(40.0);
    health.max_health = 40.0; // 20 hearts -> 2 rows
    health.health = 40.0;
    let instances = hud_player_heart_instances(surface_size, health);
    assert_eq!(container_hearts(&instances).len(), 20);
    // numHealthRows = ceil(40/2/10) = 2 -> healthRowHeight = max(10-0,3) = 10.
    assert_eq!(hud_health_rows(40.0, 0.0), (2, 10));
    // Row 0 sits at 681, row 1 one healthRowHeight (10px) above at 671.
    let ys: std::collections::BTreeSet<i64> = container_hearts(&instances)
        .iter()
        .map(|i| i.y as i64)
        .collect();
    assert_eq!(ys.into_iter().collect::<Vec<_>>(), vec![671, 681]);
}

#[test]
fn hud_player_heart_instances_regeneration_lifts_one_heart() {
    let surface_size = PhysicalSize::new(1280, 720);
    let mut health = player_health(20.0);
    health.regen = true;
    // heartOffsetIndex = tickCount % ceil(maxHealth + 5) = 3 % ceil(25) = 3.
    health.tick_count = 3;
    let instances = hud_player_heart_instances(surface_size, health);
    let lifted: Vec<_> = container_hearts(&instances)
        .into_iter()
        .filter(|i| i.y == 679.0)
        .collect();
    // Container index 3 = column 3 (x = 549 + 3*8 = 573) lifted by 2px.
    assert_eq!(lifted.len(), 1);
    assert_eq!(lifted[0].x, 573.0);

    // No regeneration: no heart is lifted.
    health.regen = false;
    let instances = hud_player_heart_instances(surface_size, health);
    assert!(container_hearts(&instances).iter().all(|i| i.y == 681.0));
}

#[test]
fn hud_player_heart_instances_shake_only_at_low_health() {
    let surface_size = PhysicalSize::new(1280, 720);
    // 2 health (<= 4): every container jitters by nextInt(2) from the
    // tickCount * 312871 seed, drawn in descending container order.
    let mut health = player_health(2.0);
    health.tick_count = 5;
    let shaken = hud_player_heart_instances(surface_size, health);
    let seed = 5i32.wrapping_mul(312871) as i64 as u64;
    let mut rng = HudObfuscatedRandom::with_seed(seed);
    for container in container_hearts(&shaken) {
        assert_eq!(container.y, 681.0 + rng.next_int_bound(2) as f32);
    }
    // Deterministic for a given tick.
    assert_eq!(shaken, hud_player_heart_instances(surface_size, health));

    // Above the 4-half-heart threshold nothing shakes.
    let mut steady = player_health(6.0);
    steady.tick_count = 5;
    assert!(hud_player_heart_instances(surface_size, steady)
        .iter()
        .all(|i| i.y == 681.0));
}

#[test]
fn hud_layout_places_armor_row_one_row_above_the_hearts() {
    let surface_size = PhysicalSize::new(1280, 720);
    // Single health row (numHealthRows = 1): the armor row sits the fixed
    // 10px above the heart baseline (720 - 39 - 10 = 671), the same as the
    // pre-multi-row layout, regardless of healthRowHeight.
    let first = armor_hud_rect(surface_size, 0, 1, HUD_SINGLE_HEALTH_ROW_HEIGHT, 9, 9);
    let last = armor_hud_rect(surface_size, 9, 1, HUD_SINGLE_HEALTH_ROW_HEIGHT, 9, 9);
    assert_eq!(first.x, 549.0);
    assert_eq!(last.x, 621.0);
    assert_eq!(first.y, 671.0);
    assert_eq!(last.y, 671.0);
    // The heart baseline (yLineBase) is 720 - 39 = 681, one 10px gap below.
    assert_eq!(681.0 - first.y, 10.0);

    // Multi-row health lifts the armor row by (numHealthRows - 1) *
    // healthRowHeight on top of the 10px gap: 2 rows @ 10px -> 671 - 10 =
    // 661; 3 rows @ 9px -> 671 - 18 = 653.
    assert_eq!(armor_hud_rect(surface_size, 0, 2, 10, 9, 9).y, 661.0);
    assert_eq!(armor_hud_rect(surface_size, 0, 3, 9, 9, 9).y, 653.0);
}

#[test]
fn hud_layout_centers_vanilla_inventory_background() {
    let surface_size = PhysicalSize::new(1280, 720);
    let background = inventory_background_hud_rect(surface_size, 176, 166, 0, 0, 176, 166);
    assert_eq!(background.x, 552.0);
    assert_eq!(background.y, 277.0);
    assert_eq!(background.width, 176);
    assert_eq!(background.height, 166);

    let generic_top = inventory_background_hud_rect(surface_size, 176, 222, 0, 0, 176, 125);
    assert_eq!(generic_top.x, 552.0);
    assert_eq!(generic_top.y, 249.0);
    assert_eq!(generic_top.width, 176);
    assert_eq!(generic_top.height, 125);

    let generic_bottom = inventory_background_hud_rect(surface_size, 176, 222, 0, 125, 176, 96);
    assert_eq!(generic_bottom.x, 552.0);
    assert_eq!(generic_bottom.y, 374.0);
    assert_eq!(generic_bottom.width, 176);
    assert_eq!(generic_bottom.height, 96);

    let oversized_background =
        inventory_background_hud_rect(surface_size, 176, 166, 4, 6, 200, 180);
    assert_eq!(oversized_background.x, 556.0);
    assert_eq!(oversized_background.y, 283.0);
    assert_eq!(oversized_background.width, 200);
    assert_eq!(oversized_background.height, 180);
}

#[test]
fn hud_layout_places_inventory_slot_icons_relative_to_screen_origin() {
    let surface_size = PhysicalSize::new(1280, 720);
    let item = inventory_slot_item_hud_rect(surface_size, 176, 166, 8, 84);
    assert_eq!(item.x, 560.0);
    assert_eq!(item.y, 361.0);
    assert_eq!(item.width, 16);
    assert_eq!(item.height, 16);

    let generic_item = inventory_slot_item_hud_rect(surface_size, 176, 222, 8, 197);
    assert_eq!(generic_item.x, 560.0);
    assert_eq!(generic_item.y, 446.0);

    let highlight = inventory_slot_highlight_hud_rect(surface_size, 176, 166, 8, 84);
    assert_eq!(highlight.x, 556.0);
    assert_eq!(highlight.y, 357.0);
    assert_eq!(highlight.width, 24);
    assert_eq!(highlight.height, 24);
}

#[test]
fn hud_inventory_tooltip_layout_uses_vanilla_default_offsets() {
    let surface_size = PhysicalSize::new(320, 240);
    assert_eq!(hud_inventory_tooltip_text_height(1), Some(8));
    assert_eq!(hud_inventory_tooltip_text_height(2), Some(20));

    let background =
        hud_inventory_tooltip_background_hud_rect(surface_size, 176, 166, 8, 84, 36, 8);
    assert_eq!(background.x, 80.0);
    assert_eq!(background.y, 97.0);
    assert_eq!(background.width, 60);
    assert_eq!(background.height, 32);

    let glyph = HudAsciiGlyph {
        width: 8,
        height: 8,
        advance: 6.0,
        ..HudAsciiGlyph::default()
    };
    // Line 1 pen origin + a pen advance of 6 with the 1px shadow offset:
    // the glyph quad is the axis-aligned 8x8 cell at (99, 122).
    let (line_x, line_y) =
        hud_inventory_tooltip_line_origin(surface_size, 176, 166, 8, 84, 36, 8, 1);
    let quads = glyph.styled_quads(
        line_x + 6.0 + 1.0,
        line_y + 1.0,
        HudTextStyle::default(),
        false,
    );
    assert_eq!(quads.len(), 1);
    assert_eq!(
        quads[0].corners,
        [[99.0, 122.0], [99.0, 130.0], [107.0, 130.0], [107.0, 122.0],]
    );
}

#[test]
fn hud_inventory_tooltip_layout_matches_vanilla_edge_fallbacks() {
    let surface_size = PhysicalSize::new(100, 100);

    let right = hud_inventory_tooltip_background_hud_rect(surface_size, 80, 80, 70, 20, 60, 8);
    assert_eq!(right.x, -4.0);
    assert_eq!(right.y, 6.0);
    assert_eq!(right.width, 84);

    let bottom = hud_inventory_tooltip_background_hud_rect(surface_size, 80, 80, 20, 96, 30, 20);
    assert_eq!(bottom.x, 30.0);
    assert_eq!(bottom.y, 65.0);
    assert_eq!(bottom.height, 44);
}

#[test]
fn nine_slice_segments_stretch_inner_splits_into_nine_vanilla_regions() {
    let target = HudRect {
        x: 100.0,
        y: 50.0,
        width: 64,
        height: 32,
    };
    let segments = nine_slice_segments(target, uniform_nine_slice(100, 9, true));

    // 100x100 sprite, border 9 (well below target/2), stretched inner => exactly nine quads.
    assert_eq!(segments.len(), 9);

    // Top-left corner: 9x9 at the target origin, sampling the sprite's top-left 9x9 texels.
    assert_eq!(
        segments[0],
        NineSliceSegment {
            rect: HudRect {
                x: 100.0,
                y: 50.0,
                width: 9,
                height: 9,
            },
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [9.0 / 100.0, 9.0 / 100.0],
            },
        }
    );

    // Center: the 46x14 inner span stretched across the sprite's 82x82 inner texels.
    assert_eq!(
        segments[4],
        NineSliceSegment {
            rect: HudRect {
                x: 109.0,
                y: 59.0,
                width: 46,
                height: 14,
            },
            uv: HudUvRect {
                min: [9.0 / 100.0, 9.0 / 100.0],
                max: [91.0 / 100.0, 91.0 / 100.0],
            },
        }
    );

    // Bottom-right corner anchored to the target's far edge and the sprite's far texels.
    assert_eq!(
        segments[8],
        NineSliceSegment {
            rect: HudRect {
                x: 155.0,
                y: 73.0,
                width: 9,
                height: 9,
            },
            uv: HudUvRect {
                min: [91.0 / 100.0, 91.0 / 100.0],
                max: [1.0, 1.0],
            },
        }
    );
}

#[test]
fn nine_slice_segments_clamp_borders_and_drop_center_when_target_smaller_than_borders() {
    let target = HudRect {
        x: 20.0,
        y: 30.0,
        width: 10,
        height: 10,
    };
    let segments = nine_slice_segments(target, uniform_nine_slice(100, 9, true));

    // Vanilla clamps each border to target/2 (=5 here), collapsing the inner spans to zero, so
    // only the four 5x5 corner quads survive.
    assert_eq!(segments.len(), 4);
    assert_eq!(
        segments[0].rect,
        HudRect {
            x: 20.0,
            y: 30.0,
            width: 5,
            height: 5,
        }
    );
    assert_eq!(segments[0].uv.max, [5.0 / 100.0, 5.0 / 100.0]);
    assert_eq!(
        segments[3].rect,
        HudRect {
            x: 25.0,
            y: 35.0,
            width: 5,
            height: 5,
        }
    );
    assert_eq!(segments[3].uv.min, [95.0 / 100.0, 95.0 / 100.0]);
    assert_eq!(segments[3].uv.max, [1.0, 1.0]);
}

#[test]
fn nine_slice_segments_tile_inner_repeats_and_clips_last_tile() {
    let target = HudRect {
        x: 0.0,
        y: 0.0,
        width: 28,
        height: 12,
    };
    let tiled = nine_slice_segments(target, uniform_nine_slice(20, 4, false));
    let stretched = nine_slice_segments(target, uniform_nine_slice(20, 4, true));

    // Inner span (20 wide) exceeds the sprite's inner texel span (12), so tiling emits an extra
    // clipped quad per horizontal inner slice; the stretched variant stays at nine.
    assert_eq!(stretched.len(), 9);
    assert_eq!(tiled.len(), 12);

    // Ordered row-major, the center row's tiles are indices 5 and 6.
    assert_eq!(
        tiled[5],
        NineSliceSegment {
            rect: HudRect {
                x: 4.0,
                y: 4.0,
                width: 12,
                height: 4,
            },
            uv: HudUvRect {
                min: [4.0 / 20.0, 4.0 / 20.0],
                max: [16.0 / 20.0, 8.0 / 20.0],
            },
        }
    );
    // Final center tile is clipped to the remaining 8px, sampling only 8 of the 12 inner texels.
    assert_eq!(
        tiled[6],
        NineSliceSegment {
            rect: HudRect {
                x: 16.0,
                y: 4.0,
                width: 8,
                height: 4,
            },
            uv: HudUvRect {
                min: [4.0 / 20.0, 4.0 / 20.0],
                max: [12.0 / 20.0, 8.0 / 20.0],
            },
        }
    );
}

#[test]
fn glyph_quads_align_pages_on_the_vanilla_baseline() {
    // `GlyphBitmap.getTop()` = 7 - ascent: an accented-page glyph (é,
    // height 12, ascent 10) starts 3px above an ascii-page glyph (e,
    // ascent 7) drawn at the same pen position.
    let surface_size = PhysicalSize::new(320, 240);
    let ascii_e = HudAsciiGlyph {
        width: 8,
        height: 8,
        advance: 6.0,
        bearing_top: 7.0,
        ..HudAsciiGlyph::default()
    };
    let accented_e = HudAsciiGlyph {
        width: 9,
        height: 12,
        advance: 6.0,
        bearing_top: 10.0,
        ..HudAsciiGlyph::default()
    };

    let (label_x, label_y) = hud_inventory_text_label_origin(surface_size, 176, 166, 62, 24);
    let plain = HudTextStyle::default();
    let label_e = glyph_top_left(ascii_e, label_x, label_y, plain);
    let label_e_accent = glyph_top_left(accented_e, label_x, label_y, plain);
    assert_eq!(label_e_accent[1], label_e[1] - 3.0);

    let (tooltip_x, tooltip_y) =
        hud_inventory_tooltip_line_origin(surface_size, 176, 166, 8, 84, 36, 8, 0);
    let tooltip_e = glyph_top_left(ascii_e, tooltip_x, tooltip_y, plain);
    let tooltip_e_accent = glyph_top_left(accented_e, tooltip_x, tooltip_y, plain);
    assert_eq!(tooltip_e_accent[1], tooltip_e[1] - 3.0);
}

#[test]
fn hud_health_rows_scale_rows_and_compress_height() {
    // 20 max / no absorption -> 1 row (healthRowHeight is unused for a
    // single row but follows max(10-(1-2),3) = 11).
    assert_eq!(hud_health_rows(20.0, 0.0), (1, 11));
    // Health Boost / absorption push into more rows and shrink the step.
    assert_eq!(hud_health_rows(40.0, 0.0), (2, 10));
    assert_eq!(hud_health_rows(20.0, 20.0), (2, 10));
    assert_eq!(hud_health_rows(60.0, 0.0), (3, 9));
    // The row height floors at 3 for very tall stacks.
    assert_eq!(hud_health_rows(200.0, 0.0).1, 3);
    // Absorption is ceiled before adding: 20 max + ceil(0.5) = 1 -> 21 HP,
    // ceil(21 / 2 / 10) = 2 rows.
    assert_eq!(hud_health_rows(20.0, 0.5), (2, 10));

    // hud_health_row_geometry resolves currentHealth/maxHealth first:
    // maxHealth falls back to currentHealth when the attribute is lower.
    let mut health = player_health(30.0);
    health.max_health = 20.0; // ceil(30) forces maxHealth up to 30
    assert_eq!(hud_health_row_geometry(health), (2, 10));

    health.health = 20.0;
    health.display_health = 40;
    assert_eq!(
        hud_health_row_geometry(health),
        (2, 10),
        "held displayHealth also expands Vanilla maxHealth"
    );
}

#[test]
fn hud_food_fill_uses_server_food_halves() {
    assert_eq!(hud_food_fill(0, 0), HudIconFill::Empty);
    assert_eq!(hud_food_fill(5, 0), HudIconFill::Full);
    assert_eq!(hud_food_fill(5, 2), HudIconFill::Half);
    assert_eq!(hud_food_fill(6, 2), HudIconFill::Full);
    assert_eq!(hud_food_fill(20, 9), HudIconFill::Full);
    assert_eq!(hud_food_fill(25, 9), HudIconFill::Full);
}

#[test]
fn hud_armor_fill_splits_full_half_empty_on_the_armor_value() {
    // Vanilla `Gui.extractArmor` compares `i * 2 + 1` to the armor value.
    // armor = 7 -> icons 0..3 full (thresholds 1,3,5 < 7), icon 3 half
    // (threshold 7 == 7), icons 4..9 empty (thresholds 9.. > 7): 3 full + 1
    // half + 6 empty.
    let fills: Vec<HudIconFill> = (0..HUD_ARMOR_ICONS_PER_ROW)
        .map(|index| hud_armor_fill(7, index))
        .collect();
    assert_eq!(
        fills,
        vec![
            HudIconFill::Full,
            HudIconFill::Full,
            HudIconFill::Full,
            HudIconFill::Half,
            HudIconFill::Empty,
            HudIconFill::Empty,
            HudIconFill::Empty,
            HudIconFill::Empty,
            HudIconFill::Empty,
            HudIconFill::Empty,
        ]
    );
    assert_eq!(fills.iter().filter(|&&f| f == HudIconFill::Full).count(), 3);
    assert_eq!(fills.iter().filter(|&&f| f == HudIconFill::Half).count(), 1);
    assert_eq!(
        fills.iter().filter(|&&f| f == HudIconFill::Empty).count(),
        6
    );

    // A full 20-point armor bar is 10 full icons, no half.
    assert!(
        (0..HUD_ARMOR_ICONS_PER_ROW).all(|index| hud_armor_fill(20, index) == HudIconFill::Full)
    );
    // An odd armor of 1 is a single half icon in slot 0.
    assert_eq!(hud_armor_fill(1, 0), HudIconFill::Half);
    assert_eq!(hud_armor_fill(1, 1), HudIconFill::Empty);
}

#[test]
fn hud_air_bubble_wobble_needs_all_empty_and_an_even_tick() {
    // Not all empty: never wobbles.
    assert_eq!(hud_air_bubble_wobble_offsets(false, 4, 7), [0; 10]);
    // All empty on an odd tick: no wobble (vanilla `tickCount % 2 == 0`).
    assert_eq!(hud_air_bubble_wobble_offsets(true, 3, 7), [0; 10]);
    // All empty on an even tick: each shell shifts by a deterministic
    // `nextInt(2)` from the frame seed's LCG.
    let offsets = hud_air_bubble_wobble_offsets(true, 4, 7);
    let mut random = HudObfuscatedRandom::with_seed(7);
    let expected: [i32; 10] = std::array::from_fn(|_| random.next_int_bound(2) as i32);
    assert_eq!(offsets, expected);
    assert!(offsets.iter().all(|&offset| (0..=1).contains(&offset)));
    // Same seed -> same offsets: a redraw of the same frame is stable.
    assert_eq!(
        hud_air_bubble_wobble_offsets(true, 4, 42),
        hud_air_bubble_wobble_offsets(true, 4, 42)
    );
}

#[test]
fn hud_food_jitter_shakes_only_when_saturation_empty_and_the_tick_hits() {
    // Saturation above zero never shakes, whatever the tick.
    assert_eq!(hud_food_jitter_offsets(18, false, 0, 7), [0; 10]);
    assert_eq!(hud_food_jitter_offsets(18, false, 55, 7), [0; 10]);

    // Saturation empty but the tick misses the `food*3+1` modulo (food = 18
    // -> divisor 55; 1 % 55 != 0): still no shake.
    assert_eq!(hud_food_jitter_offsets(18, true, 1, 7), [0; 10]);

    // Saturation empty and the tick hits the modulo: every icon shifts by a
    // deterministic `nextInt(3) - 1` drawn from the frame seed's LCG.
    let offsets = hud_food_jitter_offsets(18, true, 55, 7);
    let mut random = HudObfuscatedRandom::with_seed(7);
    let expected: [i32; 10] = std::array::from_fn(|_| random.next_int_bound(3) as i32 - 1);
    assert_eq!(offsets, expected);
    assert!(offsets.iter().all(|&offset| (-1..=1).contains(&offset)));
    assert!(
        offsets.iter().any(|&offset| offset != 0),
        "seed 7 shakes at least one icon"
    );

    // food = 0 -> divisor 1, so any tick hits while saturation is empty.
    let starving = hud_food_jitter_offsets(0, true, 3, 11);
    assert!(starving.iter().all(|&offset| (-1..=1).contains(&offset)));
    // Same seed -> same offsets, so a redraw of the same frame is stable.
    assert_eq!(
        hud_food_jitter_offsets(0, true, 3, 42),
        hud_food_jitter_offsets(0, true, 3, 42)
    );
}

#[test]
fn hud_food_rect_applies_the_per_icon_shake_offset() {
    let surface_size = PhysicalSize::new(400, 300);
    let base = food_hud_rect(surface_size, 0, 9, 9, 0);
    let shaken_up = food_hud_rect(surface_size, 0, 9, 9, -1);
    let shaken_down = food_hud_rect(surface_size, 0, 9, 9, 1);
    assert_eq!(shaken_up.y, base.y - 1.0);
    assert_eq!(shaken_down.y, base.y + 1.0);
    // Only y moves; x/size are untouched.
    assert_eq!(shaken_up.x, base.x);
    assert_eq!(shaken_up.width, base.width);
}

#[test]
fn hud_experience_level_text_origin_centers_above_the_bar() {
    // Vanilla: x = (guiWidth - width) / 2 (int division), y = guiHeight - 35.
    assert_eq!(
        hud_experience_level_text_origin(PhysicalSize::new(400, 300), 12),
        (194.0, 265.0)
    );
    // Odd numerator truncates toward zero, matching Java int division.
    assert_eq!(
        hud_experience_level_text_origin(PhysicalSize::new(401, 300), 10),
        (195.0, 265.0)
    );
    assert_eq!(
        hud_experience_level_text_origin(PhysicalSize::new(400, 300), 13),
        (193.0, 265.0)
    );
}

#[test]
fn hud_item_durability_bar_rect_uses_vanilla_item_bar_position() {
    let rect = hud_item_durability_bar_rect(
        HudRect {
            x: 10.0,
            y: 20.0,
            width: 16,
            height: 16,
        },
        13,
        2,
    );

    assert_eq!(rect.x, 12.0);
    assert_eq!(rect.y, 33.0);
    assert_eq!(rect.width, 13);
    assert_eq!(rect.height, 2);
}

#[test]
fn hud_item_cooldown_rect_uses_vanilla_fill_position() {
    let item = HudRect {
        x: 10.0,
        y: 20.0,
        width: 16,
        height: 16,
    };

    assert!(hud_item_cooldown_rect(item, 0.0).is_none());
    assert!(hud_item_cooldown_rect(item, -1.0).is_none());
    let partial = hud_item_cooldown_rect(item, 0.5).unwrap();
    assert_eq!(partial.x, 10.0);
    assert_eq!(partial.y, 28.0);
    assert_eq!(partial.width, 16);
    assert_eq!(partial.height, 8);
    let clamped = hud_item_cooldown_rect(item, 2.0).unwrap();
    assert_eq!(clamped.x, 10.0);
    assert_eq!(clamped.y, 20.0);
    assert_eq!(clamped.width, 16);
    assert_eq!(clamped.height, 16);
}

#[test]
fn hud_player_heart_instances_caps_extreme_health_and_absorption_without_overflow() {
    let surface_size = PhysicalSize::new(854, 480);
    let extreme = HudPlayerHealth {
        health: 1e9,
        display_health: i32::MAX,
        max_health: f32::MAX,
        absorption: 1e9,
        heart_type: HudHeartKind::Normal,
        hardcore: false,
        blinking: true,
        regen: true,
        tick_count: 100,
    };
    let (rows, row_height) = hud_health_rows(f32::MAX, 1e9);
    assert!(rows <= MAX_HUD_HEART_CONTAINERS as u32);
    assert!(row_height >= 3);

    let instances = hud_player_heart_instances(surface_size, extreme);
    let containers = container_hearts(&instances).len();
    assert!(containers > 0 && containers <= MAX_HUD_HEART_CONTAINERS as usize);

    let non_finite = HudPlayerHealth {
        health: f32::NAN,
        ..extreme
    };
    assert!(hud_player_heart_instances(surface_size, non_finite).is_empty());
}

#[test]
fn tiled_and_nine_slice_segments_cap_extreme_tile_counts_and_overflow() {
    let huge = HudRect {
        x: 0.0,
        y: 0.0,
        width: u32::MAX,
        height: u32::MAX,
    };
    let tiled = tiled_sprite_segments(huge, 1, 1);
    assert_eq!(
        tiled.len(),
        1,
        "oversized tiled sprite falls back to a single stretched quad"
    );

    let nine = nine_slice_segments(
        huge,
        HudNineSliceScaling {
            sprite_width: 16,
            sprite_height: 16,
            border_left: 2,
            border_right: 2,
            border_top: 2,
            border_bottom: 2,
            stretch_inner: false,
        },
    );
    assert!(
        nine.len() <= 9,
        "oversized tiled nine-slice falls back to stretched regions instead of OOMing"
    );
}
