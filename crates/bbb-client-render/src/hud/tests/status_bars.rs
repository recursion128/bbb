use super::*;

#[test]
fn armor_bar_offscreen_frame_draws_only_when_armor_is_positive() {
    use crate::camera::ClearColor;

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;

    let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
        // No GPU / software adapter on this machine — skip rather than fail the suite.
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.update_camera();

    // 9x9 solid-green armor icons (the vanilla armor sprite size), unambiguously
    // visible against the blue clear color.
    let green: Vec<u8> = (0..81).flat_map(|_| [0u8, 255, 0, 255]).collect();
    renderer
        .add_test_gui_sprite("hud/armor_empty", 9, 9, &green)
        .expect("armor_empty");
    renderer
        .add_test_gui_sprite("hud/armor_half", 9, 9, &green)
        .expect("armor_half");
    renderer
        .add_test_gui_sprite("hud/armor_full", 9, 9, &green)
        .expect("armor_full");

    // Armor icon 0's top-left is (guiWidth/2 - 91, guiHeight - 49); sample its center.
    let armor_px = WIDTH / 2 - 91 + 4;
    let armor_py = HEIGHT - 49 + 4;

    // A full 20-point armor bar paints the armor row green.
    renderer.set_hud_armor(Some(20));
    let pixels = renderer.render_offscreen_frame().expect("armor frame");
    let armor_pixel = pixels.pixel(armor_px, armor_py);
    let corner_pixel = pixels.pixel(0, 0);
    assert!(
        armor_pixel[1] > 128 && armor_pixel[0] < 128 && armor_pixel[2] < 128,
        "armor row should show the green icon, got {armor_pixel:?}"
    );
    assert!(
        corner_pixel[2] > 128 && corner_pixel[1] < 128,
        "corner should stay background, got {corner_pixel:?}"
    );

    // Armor value 0 is under vanilla's `armor > 0` gate: nothing
    // is drawn, so the armor pixel reverts to the blue background.
    renderer.set_hud_armor(Some(0));
    let pixels = renderer.render_offscreen_frame().expect("no-armor frame");
    let armor_pixel = pixels.pixel(armor_px, armor_py);
    assert!(
        armor_pixel[2] > 128 && armor_pixel[1] < 128,
        "armor row should stay background when armor is 0, got {armor_pixel:?}"
    );
}

#[test]
fn air_bubbles_offscreen_frame_draw_only_underwater_or_below_max() {
    use crate::camera::ClearColor;

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;

    let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
        // No GPU / software adapter on this machine — skip rather than fail the suite.
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.update_camera();

    // 9x9 solid-green bubbles against the blue clear color.
    let green: Vec<u8> = (0..81).flat_map(|_| [0u8, 255, 0, 255]).collect();
    renderer
        .add_test_gui_sprite("hud/air", 9, 9, &green)
        .expect("air");
    renderer
        .add_test_gui_sprite("hud/air_bursting", 9, 9, &green)
        .expect("air_bursting");
    renderer
        .add_test_gui_sprite("hud/air_empty", 9, 9, &green)
        .expect("air_empty");

    // Bubble index 0 (the rightmost) sits at (guiWidth/2 + 91 - 9,
    // guiHeight - 49) on foot; sample its center.
    let bubble_px = WIDTH / 2 + 91 - 9 + 4;
    let bubble_py = HEIGHT - 49 + 4;

    // Underwater at the full 300-tick supply the row is visible (vanilla
    // draws it whenever the eye is in water, Gui) and all full.
    renderer.set_hud_air(Some(HudAirSupply {
        air: 300,
        max_air: 300,
        eye_in_water: true,
        tick_count: 0,
    }));
    let pixels = renderer.render_offscreen_frame().expect("underwater frame");
    let bubble_pixel = pixels.pixel(bubble_px, bubble_py);
    assert!(
        bubble_pixel[1] > 128 && bubble_pixel[0] < 128 && bubble_pixel[2] < 128,
        "air row should show the green bubble underwater, got {bubble_pixel:?}"
    );

    // On land at the full supply the `isUnderWater || air < max` gate
    // hides the row: the pixel reverts to the blue background.
    renderer.set_hud_air(Some(HudAirSupply {
        air: 300,
        max_air: 300,
        eye_in_water: false,
        tick_count: 0,
    }));
    let pixels = renderer.render_offscreen_frame().expect("surfaced frame");
    let bubble_pixel = pixels.pixel(bubble_px, bubble_py);
    assert!(
        bubble_pixel[2] > 128 && bubble_pixel[1] < 128,
        "air row should stay background at full air on land, got {bubble_pixel:?}"
    );
}

#[test]
fn vehicle_hearts_offscreen_frame_replace_the_food_row() {
    use crate::camera::ClearColor;

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;

    let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
        // No GPU / software adapter on this machine — skip rather than fail the suite.
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.update_camera();

    // Red food icons vs green vehicle hearts, both 9x9, on the shared
    // right-anchored baseline slot (guiWidth/2 + 91 - 9, guiHeight - 39).
    let red: Vec<u8> = (0..81).flat_map(|_| [255u8, 0, 0, 255]).collect();
    let green: Vec<u8> = (0..81).flat_map(|_| [0u8, 255, 0, 255]).collect();
    renderer
        .add_test_gui_sprite("hud/food_empty", 9, 9, &red)
        .expect("food_empty");
    renderer
        .add_test_gui_sprite("hud/food_full", 9, 9, &red)
        .expect("food_full");
    renderer
        .add_test_gui_sprite("hud/food_half", 9, 9, &red)
        .expect("food_half");
    renderer
        .add_test_gui_sprite("hud/heart/vehicle_container", 9, 9, &green)
        .expect("vehicle_container");
    renderer
        .add_test_gui_sprite("hud/heart/vehicle_full", 9, 9, &green)
        .expect("vehicle_full");
    renderer
        .add_test_gui_sprite("hud/heart/vehicle_half", 9, 9, &green)
        .expect("vehicle_half");
    renderer.set_hud_food(Some(20));

    let slot_px = WIDTH / 2 + 91 - 9 + 4;
    let slot_py = HEIGHT - 39 + 4;

    // On foot the slot shows the red food icon.
    let pixels = renderer.render_offscreen_frame().expect("food frame");
    let slot_pixel = pixels.pixel(slot_px, slot_py);
    assert!(
        slot_pixel[0] > 128 && slot_pixel[1] < 128,
        "the baseline slot should show the red food icon on foot, got {slot_pixel:?}"
    );

    // Riding a living vehicle with hearts: the food row is suppressed
    // (vanilla `vehicleHearts == 0` gate) and the same
    // slot draws the green vehicle heart instead.
    renderer.set_hud_vehicle_health(Some(HudVehicleHealth {
        health: 10.0,
        max_health: 20.0,
    }));
    let pixels = renderer.render_offscreen_frame().expect("vehicle frame");
    let slot_pixel = pixels.pixel(slot_px, slot_py);
    assert!(
        slot_pixel[1] > 128 && slot_pixel[0] < 128 && slot_pixel[2] < 128,
        "the baseline slot should show the green vehicle heart while riding, got {slot_pixel:?}"
    );

    // A 0-heart vehicle (1.0 max health) keeps the food row (vanilla
    // `getVehicleMaxHearts` -> 0 -> food drawn).
    renderer.set_hud_vehicle_health(Some(HudVehicleHealth {
        health: 1.0,
        max_health: 1.0,
    }));
    let pixels = renderer
        .render_offscreen_frame()
        .expect("zero-heart vehicle frame");
    let slot_pixel = pixels.pixel(slot_px, slot_py);
    assert!(
        slot_pixel[0] > 128 && slot_pixel[1] < 128,
        "a 0-heart vehicle should keep the red food row, got {slot_pixel:?}"
    );
}

#[test]
fn contextual_bar_jump_replaces_experience_and_uses_cooldown_overlay() {
    use crate::camera::ClearColor;

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;

    let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
        // No GPU / software adapter on this machine — skip rather than fail the suite.
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    renderer.update_camera();

    let solid_bar = |rgba: [u8; 4]| -> Vec<u8> { (0..(182 * 5)).flat_map(|_| rgba).collect() };
    let red = solid_bar([255, 0, 0, 255]);
    let green = solid_bar([0, 255, 0, 255]);
    let yellow = solid_bar([255, 255, 0, 255]);
    let cyan = solid_bar([0, 255, 255, 255]);
    renderer
        .add_test_gui_sprite("hud/experience_bar_background", 182, 5, &red)
        .expect("experience background");
    renderer
        .add_test_gui_sprite("hud/experience_bar_progress", 182, 5, &red)
        .expect("experience progress");
    renderer
        .add_test_gui_sprite("hud/jump_bar_background", 182, 5, &green)
        .expect("jump background");
    renderer
        .add_test_gui_sprite("hud/jump_bar_progress", 182, 5, &yellow)
        .expect("jump progress");
    renderer
        .add_test_gui_sprite("hud/jump_bar_cooldown", 182, 5, &cyan)
        .expect("jump cooldown");

    let left_px = WIDTH / 2 - 91 + 4;
    let right_px = WIDTH / 2 + 91 - 4;
    let bar_py = HEIGHT - 24 - 5 + 2;

    renderer.set_hud_contextual_bar(HudContextualBar::Experience { progress: 1.0 });
    let pixels = renderer.render_offscreen_frame().expect("experience frame");
    let left = pixels.pixel(left_px, bar_py);
    assert!(
        left[0] > 128 && left[1] < 128 && left[2] < 128,
        "experience bar should draw the red sprite before a jump bar is projected, got {left:?}"
    );

    renderer.set_hud_contextual_bar(HudContextualBar::Jump(HudJumpBar {
        progress: 0.5,
        cooldown: false,
    }));
    let pixels = renderer.render_offscreen_frame().expect("jump frame");
    let left = pixels.pixel(left_px, bar_py);
    let right = pixels.pixel(right_px, bar_py);
    assert!(
        left[0] > 128 && left[1] > 128 && left[2] < 128,
        "jump progress should cover the left side with yellow, got {left:?}"
    );
    assert!(
        right[1] > 128 && right[0] < 128 && right[2] < 128,
        "jump background should remain visible past the clipped progress, got {right:?}"
    );

    renderer.set_hud_contextual_bar(HudContextualBar::Jump(HudJumpBar {
        progress: 0.5,
        cooldown: true,
    }));
    let pixels = renderer.render_offscreen_frame().expect("cooldown frame");
    let left = pixels.pixel(left_px, bar_py);
    let right = pixels.pixel(right_px, bar_py);
    assert!(
        left[1] > 128 && left[2] > 128 && left[0] < 128,
        "cooldown overlay should cover the left side with cyan, got {left:?}"
    );
    assert!(
        right[1] > 128 && right[2] > 128 && right[0] < 128,
        "cooldown overlay should cover the whole bar, got {right:?}"
    );
}

#[test]
fn locator_bar_offscreen_frame_tints_dot_but_not_pitch_arrow() {
    use crate::camera::ClearColor;

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;
    let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    renderer.update_camera();

    let solid =
        |count: usize, rgba: [u8; 4]| -> Vec<u8> { (0..count).flat_map(|_| rgba).collect() };
    renderer
        .add_test_gui_sprite_scaled(
            "hud/locator_bar_background",
            12,
            5,
            &solid(12 * 5, [0, 255, 0, 255]),
            HudGuiSpriteScaling::NineSlice(HudNineSliceScaling {
                sprite_width: 12,
                sprite_height: 5,
                border_left: 5,
                border_top: 1,
                border_right: 5,
                border_bottom: 1,
                stretch_inner: false,
            }),
        )
        .expect("locator background");
    renderer
        .add_test_gui_sprite(
            "hud/locator_bar_arrow_up",
            7,
            5,
            &solid(7 * 5, [0, 0, 255, 255]),
        )
        .expect("locator arrow up");
    renderer
        .add_test_gui_sprite(
            "minecraft:hud/locator_bar_dot/test",
            9,
            9,
            &solid(9 * 9, [255, 255, 255, 255]),
        )
        .expect("locator marker");
    renderer
        .add_test_gui_sprite(
            "minecraft:missingno",
            16,
            16,
            &solid(16 * 16, [255, 0, 255, 255]),
        )
        .expect("missing marker");
    renderer.set_hud_locator_styles(BTreeMap::from([
        (
            "minecraft:test".to_string(),
            HudLocatorStyle {
                near_distance: 10,
                far_distance: 20,
                sprite_ids: vec!["minecraft:hud/locator_bar_dot/test".to_string()],
            },
        ),
        (
            "minecraft:broken".to_string(),
            HudLocatorStyle {
                near_distance: 10,
                far_distance: 20,
                sprite_ids: vec!["minecraft:hud/locator_bar_dot/absent".to_string()],
            },
        ),
    ]));
    renderer.set_hud_contextual_bar(HudContextualBar::Locator(HudLocatorBar {
        markers: vec![HudLocatorMarker {
            angle_degrees: 0.0,
            pitch: HudLocatorPitch::Up,
            distance: 5.0,
            style_id: "minecraft:test".to_string(),
            color_argb: 0xFFFF0000,
        }],
        client_tick: 0,
    }));

    let pixels = renderer.render_offscreen_frame().expect("locator frame");
    for x in [WIDTH / 2 - 91 + 4, WIDTH / 2 + 91 - 2] {
        let background = pixels.pixel(x, HEIGHT - 24 - 5 + 2);
        assert!(
            background[1] > 128 && background[0] < 128 && background[2] < 128,
            "locator background should span 182 px with the green bar sprite, got {background:?} at {x}"
        );
    }
    let dot = pixels.pixel(160, 214);
    assert!(
        dot[0] > 128 && dot[1] < 128 && dot[2] < 128,
        "locator dot should receive its red marker tint, got {dot:?}"
    );
    let arrow = pixels.pixel(158, 206);
    assert!(
        arrow[2] > 128 && arrow[0] < 128 && arrow[1] < 128,
        "pitch arrow should remain untinted blue, got {arrow:?}"
    );

    renderer.set_hud_contextual_bar(HudContextualBar::Locator(HudLocatorBar {
        markers: vec![
            HudLocatorMarker {
                angle_degrees: -20.0,
                pitch: HudLocatorPitch::None,
                distance: 5.0,
                style_id: "example:unknown".to_string(),
                color_argb: 0xFFFF_FFFF,
            },
            HudLocatorMarker {
                angle_degrees: 20.0,
                pitch: HudLocatorPitch::None,
                distance: 5.0,
                style_id: "minecraft:broken".to_string(),
                color_argb: 0xFFFF_FFFF,
            },
        ],
        client_tick: 0,
    }));
    let pixels = renderer
        .render_offscreen_frame()
        .expect("missing locator frame");
    for x in [131, 188] {
        let marker = pixels.pixel(x, 214);
        assert!(
            marker[0] > 128 && marker[2] > 128 && marker[1] < 128,
            "unknown styles and absent sprites should use missingno, got {marker:?} at {x}"
        );
    }
}

#[test]
fn poison_heart_offscreen_frame_uses_the_poisoned_sprite() {
    use crate::camera::ClearColor;

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;

    let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
        // No GPU / software adapter on this machine — skip rather than fail.
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    renderer.update_camera();

    // A blue container under a red normal fill vs a green poisoned fill,
    // all 9x9 and opaque so the fill covers the container.
    let red: Vec<u8> = (0..81).flat_map(|_| [255u8, 0, 0, 255]).collect();
    let green: Vec<u8> = (0..81).flat_map(|_| [0u8, 255, 0, 255]).collect();
    let blue: Vec<u8> = (0..81).flat_map(|_| [0u8, 0, 255, 255]).collect();
    renderer
        .add_test_gui_sprite(
            &format!(
                "hud/heart/{}",
                HudHeartKind::Container.sprite_name(false, false, false)
            ),
            9,
            9,
            &blue,
        )
        .expect("container");
    renderer
        .add_test_gui_sprite(
            &format!(
                "hud/heart/{}",
                HudHeartKind::Normal.sprite_name(false, false, false)
            ),
            9,
            9,
            &red,
        )
        .expect("normal full");
    renderer
        .add_test_gui_sprite(
            &format!(
                "hud/heart/{}",
                HudHeartKind::Poisoned.sprite_name(false, false, false)
            ),
            9,
            9,
            &green,
        )
        .expect("poisoned full");

    // The leftmost heart (container index 0, column 0) at xLeft = W/2 - 91,
    // yLineBase = H - 39, sampled at its center.
    let heart_px = WIDTH / 2 - 91 + 4;
    let heart_py = HEIGHT - 39 + 4;

    let full_normal = HudPlayerHealth {
        health: 20.0,
        display_health: 20,
        blinking: false,
        max_health: 20.0,
        absorption: 0.0,
        heart_type: HudHeartKind::Normal,
        hardcore: false,
        regen: false,
        tick_count: 0,
    };
    renderer.set_hud_player_health(Some(full_normal));
    let pixels = renderer.render_offscreen_frame().expect("normal frame");
    let px = pixels.pixel(heart_px, heart_py);
    assert!(
        px[0] > 128 && px[1] < 128,
        "a normal heart should draw the red normal fill, got {px:?}"
    );

    // `HeartType.forPlayer` picks POISONED under the Poison effect: the same
    // slot now draws the green poisoned fill.
    renderer.set_hud_player_health(Some(HudPlayerHealth {
        heart_type: HudHeartKind::Poisoned,
        ..full_normal
    }));
    let pixels = renderer.render_offscreen_frame().expect("poison frame");
    let px = pixels.pixel(heart_px, heart_py);
    assert!(
        px[1] > 128 && px[0] < 128,
        "a poisoned heart should draw the green poisoned fill, got {px:?}"
    );
}

#[test]
fn player_heart_damage_blink_offscreen_frame_consumes_ordered_sprite_variants() {
    use crate::camera::ClearColor;

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;

    let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
        // No GPU / software adapter on this machine — skip rather than fail.
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    renderer.update_camera();

    let solid = |color: [u8; 4]| (0..81).flat_map(|_| color).collect::<Vec<_>>();
    renderer
        .add_test_gui_sprite(
            &format!(
                "hud/heart/{}",
                HudHeartKind::Container.sprite_name(false, false, true)
            ),
            9,
            9,
            &solid([255, 255, 0, 255]),
        )
        .expect("blinking container");
    renderer
        .add_test_gui_sprite(
            &format!(
                "hud/heart/{}",
                HudHeartKind::Normal.sprite_name(false, false, true)
            ),
            9,
            9,
            &solid([0, 255, 0, 255]),
        )
        .expect("blinking full");
    renderer
        .add_test_gui_sprite(
            &format!(
                "hud/heart/{}",
                HudHeartKind::Normal.sprite_name(false, true, true)
            ),
            9,
            9,
            &solid([0, 255, 0, 255]),
        )
        .expect("blinking half");
    renderer
        .add_test_gui_sprite(
            &format!(
                "hud/heart/{}",
                HudHeartKind::Normal.sprite_name(false, false, false)
            ),
            9,
            9,
            &solid([255, 0, 0, 255]),
        )
        .expect("current full");

    renderer.set_hud_player_health(Some(HudPlayerHealth {
        health: 18.0,
        display_health: 19,
        blinking: true,
        max_health: 20.0,
        absorption: 0.0,
        heart_type: HudHeartKind::Normal,
        hardcore: false,
        regen: false,
        tick_count: 0,
    }));
    let pixels = renderer.render_offscreen_frame().expect("damage blink");
    let y = HEIGHT - 39 + 4;
    let x_left = WIDTH / 2 - 91;

    // Container index 9 has only the lost old half-heart, so the blinking
    // green fill remains visible above the blinking container.
    let lost = pixels.pixel(x_left + 9 * 8 + 4, y);
    assert!(
        lost[1] > 128 && lost[0] < 128,
        "lost health should retain the blinking old fill, got {lost:?}"
    );
    // Index 8 has both old and current fills; Vanilla submits the current
    // non-blinking red fill last.
    let retained = pixels.pixel(x_left + 8 * 8 + 4, y);
    assert!(
        retained[0] > 128 && retained[1] < 128,
        "current health should cover the blinking old fill, got {retained:?}"
    );

    // Absorption keeps low-health shake disabled without covering the
    // ordinary empty slot, exposing the blinking yellow container sprite.
    renderer.set_hud_player_health(Some(HudPlayerHealth {
        health: 0.0,
        display_health: 0,
        blinking: true,
        absorption: 5.0,
        ..renderer.hud_player_health.expect("damage health")
    }));
    let pixels = renderer
        .render_offscreen_frame()
        .expect("blinking container");
    let container = pixels.pixel(x_left + 4, y);
    assert!(
        container[0] > 128 && container[1] > 128 && container[2] < 128,
        "empty health should expose the blinking container, got {container:?}"
    );
}

#[test]
fn hud_mob_effect_overlay_matches_vanilla_order_rows_and_icon_alpha() {
    let effects = vec![
        test_hud_mob_effect(1, true, 1, 100, false, true),
        test_hud_mob_effect(2, false, 2, 60, false, true),
        test_hud_mob_effect(3, true, 3, 200, false, true),
        test_hud_mob_effect(4, false, 4, 1_000, true, true),
        test_hud_mob_effect(5, true, 5, 2_000, false, false),
    ];

    let instances = hud_mob_effect_overlay_instances(PhysicalSize::new(320, 240), &effects);
    assert_eq!(
        instances
            .iter()
            .map(|instance| instance.effect_id)
            .collect::<Vec<_>>(),
        vec![4, 3, 1, 2]
    );
    assert_eq!(
        instances
            .iter()
            .map(|instance| { (instance.background.x as i32, instance.background.y as i32,) })
            .collect::<Vec<_>>(),
        vec![(295, 27), (295, 1), (270, 1), (270, 27)]
    );
    assert_eq!(instances[0].icon_alpha, 1.0);
    assert!((instances[1].icon_alpha - 0.5).abs() < 1e-6);
    assert!((instances[2].icon_alpha - 0.625).abs() < 1e-6);
    assert!((instances[3].icon_alpha - 0.675).abs() < 1e-6);
    assert_eq!(
        (
            instances[0].icon.x as i32,
            instances[0].icon.y as i32,
            instances[0].icon.width,
            instances[0].icon.height,
        ),
        (298, 30, 18, 18)
    );
}

#[test]
fn hud_inventory_mob_effects_match_wide_compact_and_dense_layouts() {
    let screen = hud_inventory_screen_with_entity_previews(Vec::new());
    let glyphs = hud_mob_effect_test_glyphs();
    let mut effects = (0..6)
        .map(|index| {
            test_hud_mob_effect(
                index,
                index % 2 == 0,
                index as u32,
                20 * (index + 1),
                false,
                true,
            )
        })
        .collect::<Vec<_>>();

    let wide =
        hud_inventory_mob_effect_instances(PhysicalSize::new(500, 240), &screen, &effects, &glyphs);
    assert_eq!(wide.len(), 6);
    assert_eq!(
        wide.iter()
            .map(|instance| instance.effect_id)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4, 5]
    );
    assert_eq!(
        (
            wide[0].background_x,
            wide[0].background_y,
            wide[0].background_width,
            wide[0].icon_x,
            wide[0].icon_y,
            wide[5].background_y,
        ),
        (340, 37, 87, 347, 44, 167)
    );
    assert_eq!(
        wide[0].name.as_deref().map(mob_effect_runs_text),
        Some("Effect 0".to_string())
    );
    assert_eq!(
        wide[0].duration.as_deref().map(mob_effect_runs_text),
        Some("00:10".to_string())
    );

    let compact =
        hud_inventory_mob_effect_instances(PhysicalSize::new(400, 240), &screen, &effects, &glyphs);
    assert_eq!(
        (
            compact[0].background_x,
            compact[0].background_width,
            compact[0].icon_x,
            compact[0].name.as_deref(),
            compact[0].duration.as_deref(),
        ),
        (290, 32, 297, None, None)
    );

    effects[0].display_name = "Effect name too wide for the available panel".to_string();
    let clipped =
        hud_inventory_mob_effect_instances(PhysicalSize::new(420, 240), &screen, &effects, &glyphs);
    let clipped_name = clipped[0].name.as_deref().expect("wide panel text");
    let clipped_text = mob_effect_runs_text(clipped_name);
    assert!(clipped_text.len() < effects[0].display_name.len());
    assert!(
        hud_coded_string_width(clipped_name, &glyphs)
            <= clipped[0]
                .background_width
                .saturating_sub(HUD_INVENTORY_EFFECT_TEXT_X_OFFSET as u32)
                .saturating_sub(HUD_INVENTORY_EFFECT_SPACING as u32) as i32
    );
}

/// `EffectsInInventory` sizes the background from
/// `font.width(effectName)`/`font.width(duration)` and breaks the
/// name at that same width through `ComponentRenderUtils.clipText`.
/// Both texts are Components there, so the
/// width, the break point and the glyphs read one resolved list per text.
#[test]
fn mob_effect_panel_width_break_point_and_glyphs_share_the_resolved_name() {
    let screen = hud_inventory_screen_with_entity_previews(Vec::new());
    let mut glyphs = HudFontGlyphMap::new();
    for character in (' '..='~').chain(std::iter::once('\u{a7}')) {
        glyphs.insert_first_wins(
            character,
            HudAsciiGlyph {
                advance: 6.0,
                width: 5,
                height: 8,
                ..HudAsciiGlyph::default()
            },
        );
    }
    // "§c" plus fifteen glyphs: the code pair is consumed, so the measured width
    // is 90 rather than the 102 the raw face would charge.
    let mut effects = vec![test_hud_mob_effect(1, true, 0, 200, false, true)];
    effects[0].display_name = format!("\u{a7}c{}", "A".repeat(15));
    let instances =
        hud_inventory_mob_effect_instances(PhysicalSize::new(500, 240), &screen, &effects, &glyphs);
    let instance = &instances[0];
    assert_eq!(instance.background_width, 32 + 90 + 7);
    let name = instance.name.as_deref().expect("name fits the panel");
    assert_eq!(mob_effect_runs_text(name), "A".repeat(15));
    assert_eq!(name.len(), 1);
    assert_eq!(name[0].color, Some(0xFF_5555));
    assert_eq!(
        hud_coded_string_width(name, &glyphs),
        90,
        "the kept runs measure exactly the panel's text budget"
    );

    // A longer name is clipped at the coded break point, not at a raw one.
    effects[0].display_name = format!("\u{a7}c{}", "A".repeat(30));
    let clipped =
        hud_inventory_mob_effect_instances(PhysicalSize::new(500, 240), &screen, &effects, &glyphs);
    let instance = &clipped[0];
    assert_eq!(instance.background_width, 153);
    let name = instance.name.as_deref().expect("clipped name");
    assert_eq!(mob_effect_runs_text(name), "A".repeat(19));
    assert!(hud_coded_string_width(name, &glyphs) <= 153 - 32 - 7);
    assert_eq!(
        mob_effect_runs_text(instance.duration.as_deref().expect("duration")),
        "00:10"
    );
    assert_eq!(instance.duration.as_deref().map(|runs| runs.len()), Some(1));
}

fn mob_effect_runs_text(runs: &[HudStyledTextRun]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect()
}

#[test]
fn locator_bar_animated_marker_advances_frames_with_client_tick() {
    use crate::camera::ClearColor;
    use std::collections::BTreeMap;

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;
    let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
        return;
    };
    renderer.set_clear_color(ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    renderer.update_camera();

    let solid =
        |count: usize, rgba: [u8; 4]| -> Vec<u8> { (0..count).flat_map(|_| rgba).collect() };
    renderer
        .add_test_animated_gui_sprite(
            "minecraft:hud/locator_bar_dot/animated",
            9,
            9,
            vec![
                solid(9 * 9, [255, 0, 0, 255]),
                solid(9 * 9, [0, 255, 0, 255]),
            ],
            2,
        )
        .expect("animated locator marker");
    renderer.set_hud_locator_styles(BTreeMap::from([(
        "minecraft:animated".to_string(),
        HudLocatorStyle {
            near_distance: 10,
            far_distance: 20,
            sprite_ids: vec!["minecraft:hud/locator_bar_dot/animated".to_string()],
        },
    )]));

    let mut sample_at_tick = |client_tick: u64| {
        renderer.set_hud_contextual_bar(HudContextualBar::Locator(HudLocatorBar {
            markers: vec![HudLocatorMarker {
                angle_degrees: 0.0,
                pitch: HudLocatorPitch::None,
                distance: 5.0,
                style_id: "minecraft:animated".to_string(),
                color_argb: 0xFFFF_FFFF,
            }],
            client_tick,
        }));
        renderer
            .render_offscreen_frame()
            .expect("locator frame")
            .pixel(160, 214)
    };

    for tick in [0, 1, 4] {
        let pixel = sample_at_tick(tick);
        assert!(
            pixel[0] > 128 && pixel[1] < 128 && pixel[2] < 128,
            "tick {tick} should sample red frame 0, got {pixel:?}"
        );
    }
    for tick in [2, 3] {
        let pixel = sample_at_tick(tick);
        assert!(
            pixel[1] > 128 && pixel[0] < 128 && pixel[2] < 128,
            "tick {tick} should sample green frame 1, got {pixel:?}"
        );
    }
}
