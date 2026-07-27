//! Offscreen whole-frame readback harness.
//!
//! Runs the complete `render()` frame — every `FRAME_STEPS` pass, in
//! production order — against an injected offscreen color target instead of a
//! window swapchain, then reads the presented pixels back through the shared
//! screenshot copy path (`prepare_screenshot_copy` + `read_screenshot_pixels`,
//! the single home of padded-row and BGRA handling). Pixel-proof tests for
//! HUD / screen slices build on these two entry points instead of hand-rolling
//! per-test sub-passes.

use std::sync::Arc;

use anyhow::{bail, Result};
use winit::dpi::PhysicalSize;

use crate::renderer::RenderSurface;
use crate::screenshot::ScreenshotPixels;
use crate::Renderer;

/// The production surface format family `choose_format` prefers (sRGB), so
/// offscreen frames exercise the same color pipeline as the swapchain,
/// including the BGRA byte-order swap on readback.
const OFFSCREEN_SURFACE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

impl Renderer {
    /// Builds a windowless `Renderer` whose frame target is a `width`x`height`
    /// offscreen texture in the production-preferred surface format. Returns
    /// `None` when the machine has no usable GPU adapter or device (callers
    /// skip rather than fail, matching the existing readback-test pattern).
    pub(crate) fn new_offscreen(width: u32, height: u32) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;
        eprintln!(
            "offscreen frame harness adapter: {}",
            adapter.get_info().name
        );
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("bbb-offscreen-frame-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        ))
        .ok()?;

        // Mirrors the swapchain configuration `Renderer::new` builds, minus the
        // window-dependent present/alpha negotiation (both irrelevant offscreen).
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: OFFSCREEN_SURFACE_FORMAT,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bbb-offscreen-frame-target"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: config.usage,
            view_formats: &[],
        });
        let size = PhysicalSize::new(config.width, config.height);
        Some(
            Renderer::with_gpu(
                RenderSurface::Offscreen(Arc::new(target)),
                device,
                queue,
                config,
                size,
            )
            .expect("offscreen renderer construction"),
        )
    }

    /// Renders one full frame (all `FRAME_STEPS`) into the offscreen target
    /// and returns its pixels, read back through the shared screenshot path.
    pub(crate) fn render_offscreen_frame(&mut self) -> Result<ScreenshotPixels> {
        let RenderSurface::Offscreen(target) = &self.surface else {
            bail!("render_offscreen_frame requires a Renderer built by new_offscreen");
        };
        let target = target.clone();
        self.render(None)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bbb-offscreen-frame-readback"),
            });
        let pending = self.prepare_screenshot_copy(&mut encoder, &target)?;
        self.queue.submit(Some(encoder.finish()));
        self.read_screenshot_pixels(pending)
    }
}

mod tests {
    use std::sync::Arc;

    use winit::dpi::PhysicalSize;

    use crate::{
        ClearColor, EntityDefaultPlayerSkin, EntityModelTextureImage, EntityModelTextureRef,
        EntityPlayerSkin, HudAsciiGlyph, HudComponentTextRun, HudDeathLinkConfirmationScreen,
        HudDeathMainScreen, HudDeathQuitConfirmationScreen, HudDeathScreen, HudDeathScreenButton,
        HudDemoPopupScreen, HudFontGlyphMap, HudGameEventButton, HudGameEventScreen,
        HudNineSliceScaling, HudSocialInteractionsFocus,
        HudSocialInteractionsLinkConfirmationScreen, HudSocialInteractionsMainScreen,
        HudSocialInteractionsPage, HudSocialInteractionsPlayer, HudSocialInteractionsPlayerAction,
        HudSocialInteractionsScreen, HudStyledTextRun, HudVignette, HudWinScreen, Renderer,
    };

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;

    /// Harness acceptance: a whole `render()` frame runs headless and the
    /// readback shows a known HUD element (centered crosshair) over the clear
    /// color, with the frame counters proving the step pipeline executed.
    #[test]
    fn offscreen_frame_renders_hud_sentinel_over_clear_color() {
        let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
            // No GPU / software adapter on this machine — skip rather than fail.
            return;
        };
        // Pure blue background: survives the sRGB surface encoding as byte 255.
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        // A 4x4 solid-red crosshair; `collect_hud_draws` centers it, so the
        // framebuffer center pixel must come out red.
        renderer
            .upload_hud_crosshair(4, 4, &[255, 0, 0, 255].repeat(16))
            .expect("upload crosshair");

        let pixels = renderer.render_offscreen_frame().expect("offscreen frame");

        assert_eq!((pixels.width, pixels.height), (WIDTH, HEIGHT));
        let center = pixels.pixel(WIDTH / 2, HEIGHT / 2);
        let corner = pixels.pixel(8, 8);
        assert!(
            center[0] > 128 && center[2] < 128,
            "center should show the red crosshair, got {center:?}"
        );
        assert!(
            corner[2] > 128 && corner[0] < 128,
            "corner should stay clear-color blue, got {corner:?}"
        );

        // The frame went through finish_frame's counter fold: one frame, a HUD
        // draw for the crosshair, and at least the always-on lightmap +
        // transparency combine + blit draws alongside it.
        assert_eq!(renderer.counters.frame_index, 1);
        assert!(
            renderer.counters.hud_draw_calls >= 1,
            "crosshair HUD draw is tallied, got {}",
            renderer.counters.hud_draw_calls
        );
        assert!(
            renderer.counters.draw_calls >= 4,
            "lightmap/combine/blit/HUD draws are tallied, got {}",
            renderer.counters.draw_calls
        );
        assert_eq!(renderer.counters.screenshots_written, 0);
    }

    #[test]
    fn camera_post_effect_inactive_and_unknown_ids_leave_the_frame_untouched() {
        let Some(mut renderer) = Renderer::new_offscreen(24, 20) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_post_effect(Some("minecraft:invert"), false);

        let inactive = renderer
            .render_offscreen_frame()
            .expect("inactive camera post-effect frame");
        let inactive_corner = inactive.pixel(2, 2);
        assert!(
            inactive_corner[2] > 240 && inactive_corner[0] < 8 && inactive_corner[1] < 8,
            "inactive effect must preserve the blue frame, got {inactive_corner:?}"
        );
        assert_eq!(renderer.counters.post_effect_draw_calls, 0);
        let baseline_draw_calls = renderer.counters.draw_calls;
        let baseline_pipeline_switches = renderer.counters.pipeline_switches;

        renderer.set_camera_post_effect(Some("minecraft:unknown"), true);
        let unknown = renderer
            .render_offscreen_frame()
            .expect("unknown camera post-effect frame");
        let unknown_corner = unknown.pixel(2, 2);
        assert!(
            unknown_corner[2] > 240 && unknown_corner[0] < 8 && unknown_corner[1] < 8,
            "unknown effect must preserve the blue frame, got {unknown_corner:?}"
        );
        assert_eq!(renderer.counters.post_effect_draw_calls, 0);
        assert_eq!(renderer.counters.draw_calls, baseline_draw_calls);
        assert_eq!(
            renderer.counters.pipeline_switches,
            baseline_pipeline_switches
        );

        renderer.set_camera_post_effect(Some("minecraft:invert"), true);
        renderer
            .render_offscreen_frame()
            .expect("active camera post-effect frame");
        assert_eq!(renderer.counters.post_effect_draw_calls, 2);
        assert_eq!(renderer.counters.draw_calls, baseline_draw_calls + 2);
        assert_eq!(
            renderer.counters.pipeline_switches,
            baseline_pipeline_switches + 2
        );
    }

    #[test]
    fn camera_post_effect_invert_runs_before_ordinary_hud() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_post_effect(Some("minecraft:invert"), true);
        renderer
            .upload_hud_crosshair(4, 4, &[255, 0, 0, 255].repeat(16))
            .expect("ordinary HUD sentinel");

        let pixels = renderer
            .render_offscreen_frame()
            .expect("invert camera post-effect frame");
        let corner = pixels.pixel(3, 3);
        let center = pixels.pixel(16, 16);
        assert!(
            corner[0] > corner[2].saturating_add(50) && corner[1] > corner[2].saturating_add(50),
            "0.8 inverse of blue should be yellow-dominant, got {corner:?}"
        );
        assert!(
            center[0] > 200 && center[1] < 32 && center[2] < 32,
            "the later ordinary HUD crosshair must remain red, got {center:?}"
        );
        assert_eq!(renderer.counters.post_effect_draw_calls, 2);
    }

    #[test]
    fn camera_post_effect_creeper_and_spider_execute_exact_graphs() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 24) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        });
        renderer.set_camera_post_effect(Some("minecraft:creeper"), true);

        let creeper = renderer
            .render_offscreen_frame()
            .expect("creeper camera post-effect frame");
        let creeper_corner = creeper.pixel(4, 4);
        assert!(
            creeper_corner[1] > 180 && creeper_corner[0] < 64 && creeper_corner[2] < 64,
            "creeper color matrix and bits pass should retain green only, got {creeper_corner:?}"
        );
        assert_eq!(renderer.counters.post_effect_draw_calls, 2);

        renderer.set_clear_color(ClearColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_post_effect(Some("minecraft:spider"), true);
        let spider = renderer
            .render_offscreen_frame()
            .expect("spider camera post-effect frame");
        let spider_corner = spider.pixel(4, 4);
        assert!(
            spider_corner[0] > spider_corner[1].saturating_add(10)
                && spider_corner[1].abs_diff(spider_corner[2]) < 5,
            "spider's final [1, 0.8, 0.8, 1] blit should red-tint white, got {spider_corner:?}"
        );
        assert_eq!(renderer.counters.post_effect_draw_calls, 10);
    }

    #[test]
    fn camera_post_effect_resize_recreates_targets_and_preserves_selection() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 12) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_post_effect(Some("minecraft:invert"), true);
        renderer
            .resize(PhysicalSize::new(24, 20))
            .expect("resize offscreen renderer");

        let pixels = renderer
            .render_offscreen_frame()
            .expect("resized camera post-effect frame");
        assert_eq!((pixels.width, pixels.height), (24, 20));
        let corner = pixels.pixel(2, 2);
        assert!(
            corner[0] > corner[2].saturating_add(50) && corner[1] > corner[2].saturating_add(50),
            "resized targets should still execute the selected invert graph, got {corner:?}"
        );
        assert_eq!(renderer.current_post_effect_id(), Some("minecraft:invert"));
        assert!(renderer.camera_post_effect_active());
        assert_eq!(renderer.counters.post_effect_draw_calls, 2);
    }

    #[test]
    fn offscreen_frame_applies_vignette_before_ordinary_hud() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        renderer
            .upload_hud_vignette_texture(1, 1, &[255, 255, 255, 255], true, false)
            .expect("upload vignette");
        renderer.set_hud_vignette(Some(HudVignette {
            brightness: 0.5,
            border_warning_strength: 0.5,
            color: [0.25, 0.5, 0.75],
        }));

        let pixels = renderer.render_offscreen_frame().expect("offscreen frame");
        let corner = pixels.pixel(4, 4);

        // ZERO / ONE_MINUS_SRC_COLOR over white produces
        // (0.75, 0.5, 0.25), encoded to the sRGB surface in the same order.
        assert!(
            corner[0] > corner[1] && corner[1] > corner[2],
            "vignette should destination-multiply each RGB channel, got {corner:?}"
        );
        assert_eq!(corner[3], 255, "vignette must preserve destination alpha");
        assert_eq!(renderer.counters.hud_draw_calls, 1);
    }

    #[test]
    fn hud_demo_popup_offscreen_readback_covers_ordinary_hud_with_popup_background() {
        let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer
            .upload_hud_crosshair(4, 4, &[255, 0, 0, 255].repeat(16))
            .expect("ordinary HUD sentinel");
        renderer
            .upload_hud_inworld_menu_background(1, 1, &[16, 32, 64, 255])
            .expect("in-world menu background");
        renderer
            .upload_hud_demo_popup_background(
                1,
                1,
                &[0, 255, 0, 255],
                HudNineSliceScaling {
                    sprite_width: 1,
                    sprite_height: 1,
                    border_left: 0,
                    border_top: 0,
                    border_right: 0,
                    border_bottom: 0,
                    stretch_inner: true,
                },
            )
            .expect("popup background");
        renderer.set_hud_game_event_screen(Some(HudGameEventScreen::DemoPopup(
            HudDemoPopupScreen {
                generation: 1,
                title: "Demo".to_string(),
                controls_message: "Controls".to_string(),
                duration_message: "Duration".to_string(),
                buy_label: "Buy".to_string(),
                later_label: "Later".to_string(),
                focused: HudGameEventButton::DemoBuy,
                hovered: None,
            },
        )));

        let pixels = renderer.render_offscreen_frame().expect("demo popup frame");
        let center = pixels.pixel(160, 120);
        let corner = pixels.pixel(4, 4);
        assert!(
            center[1] > 128 && center[0] < 128,
            "popup center should be green instead of the ordinary red crosshair, got {center:?}"
        );
        assert!(
            corner[0] < corner[1] && corner[1] < corner[2] && corner[2] < 128,
            "popup screen should replace the ordinary HUD with the in-world menu background at the corner, got {corner:?}"
        );
    }

    #[test]
    fn hud_win_screen_offscreen_readback_orders_portal_then_credits_vignette() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        let sky = EntityModelTextureRef {
            path: "textures/environment/end_sky.png",
            size: [128, 128],
        };
        let portal = EntityModelTextureRef {
            path: "textures/entity/end_portal/end_portal.png",
            size: [256, 256],
        };
        renderer
            .upload_entity_model_textures(&[
                EntityModelTextureImage::with_physical_size(sky, [1, 1], vec![255, 255, 255, 255]),
                EntityModelTextureImage::with_physical_size(
                    portal,
                    [1, 1],
                    vec![255, 255, 255, 255],
                ),
            ])
            .expect("portal textures");
        renderer
            .upload_hud_win_credits_vignette(1, 1, &[128, 0, 0, 255], true, false)
            .expect("credits vignette");
        renderer.set_hud_game_event_screen(Some(HudGameEventScreen::Win(HudWinScreen {
            generation: 2,
            scroll: 0.0,
            show_easter_egg_logo: false,
            lines: Arc::from([]),
        })));

        let pixels = renderer.render_offscreen_frame().expect("win frame");
        let center = pixels.pixel(16, 16);
        assert!(
            center[0] < center[1] && center[1] == center[2],
            "red credits-vignette channel should destination-multiply the portal after HUD, got {center:?}"
        );
        assert!(
            renderer.counters.hud_draw_calls >= 2,
            "portal and credits vignette must both be submitted"
        );
    }

    #[test]
    fn offscreen_social_interactions_draws_shared_rows_faces_scroll_empty_and_link_surfaces() {
        let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer
            .upload_hud_crosshair(4, 4, &[255, 0, 0, 255].repeat(16))
            .expect("ordinary HUD sentinel");
        renderer
            .upload_hud_inworld_menu_background(1, 1, &[16, 32, 64, 255])
            .expect("menu background");
        renderer
            .upload_hud_social_interactions_background(
                1,
                1,
                &[180, 16, 32, 255],
                HudNineSliceScaling {
                    sprite_width: 1,
                    sprite_height: 1,
                    border_left: 0,
                    border_top: 0,
                    border_right: 0,
                    border_bottom: 0,
                    stretch_inner: true,
                },
            )
            .expect("social background");
        renderer
            .upload_hud_social_interactions_search(1, 1, &[0, 255, 255, 255])
            .expect("search icon");
        renderer
            .upload_hud_social_interactions_mute(1, 1, &[255, 128, 0, 255])
            .expect("mute");
        renderer
            .upload_hud_social_interactions_mute_highlighted(1, 1, &[255, 255, 0, 255])
            .expect("highlighted mute");
        renderer
            .upload_hud_social_interactions_unmute(1, 1, &[0, 128, 255, 255])
            .expect("unmute");
        renderer
            .upload_hud_social_interactions_unmute_highlighted(1, 1, &[0, 255, 0, 255])
            .expect("highlighted unmute");
        renderer
            .upload_hud_widget_text_field(1, 1, &[8, 8, 8, 255])
            .expect("search field");
        renderer
            .upload_hud_widget_button(1, 1, &[0, 160, 0, 255])
            .expect("button");
        renderer
            .upload_hud_widget_button_highlighted(1, 1, &[160, 160, 0, 255])
            .expect("highlighted button");
        let mut glyphs = HudFontGlyphMap::new();
        for ch in "SocialInteractionsServerAllHiddenBlockedSearchDoneRemoteOfflineManage".chars() {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    width: 1,
                    height: 1,
                    advance: 2,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        renderer
            .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
            .expect("font");
        let mut skin_rgba = vec![0_u8; 64 * 64 * 4];
        for pixel in skin_rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0, 255, 0, 255]);
        }
        renderer
            .upload_entity_model_textures(&[EntityModelTextureImage::with_physical_size(
                EntityModelTextureRef {
                    path: EntityDefaultPlayerSkin::WideSteve.texture_path(),
                    size: [64, 64],
                },
                [64, 64],
                skin_rgba,
            )])
            .expect("default player face");

        let make_player = |id: u128, removed, hidden| HudSocialInteractionsPlayer {
            profile_id: id.to_be_bytes(),
            name: format!("Remote{id}"),
            skin: EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::WideSteve),
            removed,
            hidden,
            blocked: false,
            action: Some(if hidden {
                HudSocialInteractionsPlayerAction::Show
            } else {
                HudSocialInteractionsPlayerAction::Hide
            }),
            status: removed.then(|| "Offline".to_string()),
        };
        let rows = (1_u128..=8)
            .map(|id| make_player(id, id == 2, id == 2))
            .collect::<Vec<_>>();
        renderer.set_hud_screen_blur(false);
        renderer.set_hud_social_interactions_screen(Some(HudSocialInteractionsScreen::Main(
            HudSocialInteractionsMainScreen {
                generation: 9,
                title: "Social Interactions".to_string(),
                server_label: "Server".to_string(),
                page: HudSocialInteractionsPage::All,
                tab_labels: [
                    "All".to_string(),
                    "Hidden".to_string(),
                    "Blocked".to_string(),
                ],
                search_text: String::new(),
                search_hint: "Search".to_string(),
                search_cursor: 0,
                search_selection: 0,
                search_cursor_visible: false,
                rows: rows.clone(),
                scroll_amount: 0,
                empty_text: None,
                blocking_help_label: "Manage".to_string(),
                done_label: "Done".to_string(),
                focus: HudSocialInteractionsFocus::Search,
                hovered: Some(HudSocialInteractionsFocus::Player {
                    profile_id: 1_u128.to_be_bytes(),
                    action: HudSocialInteractionsPlayerAction::Hide,
                }),
                tooltip: None,
            },
        )));
        let pixels = renderer
            .render_offscreen_frame()
            .expect("social interactions frame");
        let corner = pixels.pixel(4, 4);
        let panel = pixels.pixel(45, 70);
        let search_icon = pixels.pixel(55, 77);
        let active_face = pixels.pixel(55, 100);
        let removed_face = pixels.pixel(55, 136);
        let hovered_mute = pixels.pixel(225, 100);
        let scrollbar = pixels.pixel(280, 120);
        assert!(
            corner[0] < corner[1] && corner[1] < corner[2],
            "menu background must cover ordinary HUD outside the panel, got {corner:?}"
        );
        assert!(
            panel[0] > panel[1].saturating_add(80),
            "official social panel owner must draw at y=64, got {panel:?}"
        );
        assert!(
            search_icon[1] > 180 && search_icon[2] > 180,
            "search icon must draw at the shared layout coordinate, got {search_icon:?}"
        );
        assert!(
            active_face[1] > active_face[0].saturating_add(80),
            "default player face must draw at 24 pixels, got {active_face:?}"
        );
        assert!(
            removed_face[1] < active_face[1],
            "removed row must shade the same face, active={active_face:?}, removed={removed_face:?}"
        );
        assert!(
            hovered_mute[0] > 180 && hovered_mute[1] > 180,
            "hovered hide action must use its highlighted sprite, got {hovered_mute:?}"
        );
        assert!(
            scrollbar[0].abs_diff(scrollbar[1]) < 24,
            "overflowing rows must draw a deterministic scrollbar, got {scrollbar:?}"
        );

        renderer.set_hud_social_interactions_screen(Some(HudSocialInteractionsScreen::Main(
            HudSocialInteractionsMainScreen {
                generation: 9,
                title: String::new(),
                server_label: String::new(),
                page: HudSocialInteractionsPage::All,
                tab_labels: [String::new(), String::new(), String::new()],
                search_text: String::new(),
                search_hint: String::new(),
                search_cursor: 0,
                search_selection: 0,
                search_cursor_visible: false,
                rows,
                scroll_amount: 36,
                empty_text: None,
                blocking_help_label: String::new(),
                done_label: String::new(),
                focus: HudSocialInteractionsFocus::Search,
                hovered: None,
                tooltip: None,
            },
        )));
        let scrolled = renderer
            .render_offscreen_frame()
            .expect("scrolled social list");
        assert!(
            scrolled.pixel(55, 100)[1] < active_face[1],
            "one-row scroll must move the removed shaded face into the first visible row"
        );

        renderer.set_hud_social_interactions_screen(Some(HudSocialInteractionsScreen::Main(
            HudSocialInteractionsMainScreen {
                generation: 9,
                title: String::new(),
                server_label: String::new(),
                page: HudSocialInteractionsPage::Blocked,
                tab_labels: [String::new(), String::new(), String::new()],
                search_text: String::new(),
                search_hint: String::new(),
                search_cursor: 0,
                search_selection: 0,
                search_cursor_visible: false,
                rows: Vec::new(),
                scroll_amount: 0,
                empty_text: Some("Blocked".to_string()),
                blocking_help_label: String::new(),
                done_label: String::new(),
                focus: HudSocialInteractionsFocus::BlockingHelp,
                hovered: None,
                tooltip: None,
            },
        )));
        let empty = renderer
            .render_offscreen_frame()
            .expect("empty blocked page");
        assert!(
            empty.pixel(61, 161)[1] > 80,
            "blocked empty page must retain its dedicated help button"
        );

        renderer.set_hud_social_interactions_screen(Some(
            HudSocialInteractionsScreen::LinkConfirmation(
                HudSocialInteractionsLinkConfirmationScreen {
                    generation: 9,
                    title: "Social".to_string(),
                    message: "Server".to_string(),
                    open_label: String::new(),
                    copy_label: String::new(),
                    cancel_label: String::new(),
                    focus: HudSocialInteractionsFocus::LinkOpen,
                    hovered: None,
                },
            ),
        ));
        let link = renderer
            .render_offscreen_frame()
            .expect("social link confirmation");
        assert!(
            link.pixel(10, 10)[2] > link.pixel(10, 10)[0],
            "link confirmation must retain the ordinary menu background"
        );
        let first_link_button = link.pixel(8, 138);
        assert!(
            first_link_button[0] > 80 && first_link_button[1] > 80,
            "link confirmation must draw its highlighted first button, got {first_link_button:?}"
        );
    }

    #[test]
    fn offscreen_death_screen_draws_gradient_after_ordinary_hud_without_blur() {
        let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer
            .upload_hud_crosshair(4, 4, &[255, 0, 0, 255].repeat(16))
            .expect("ordinary HUD sentinel");
        let mut glyphs = HudFontGlyphMap::new();
        for ch in "TCS".chars() {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    width: 5,
                    height: 8,
                    advance: 6,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        renderer
            .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], glyphs)
            .expect("test font");
        renderer
            .upload_hud_widget_button(1, 1, &[0, 255, 0, 255])
            .expect("enabled button");
        renderer
            .upload_hud_widget_button_disabled(1, 1, &[64, 64, 64, 255])
            .expect("disabled button");
        renderer
            .upload_hud_widget_button_highlighted(1, 1, &[255, 255, 0, 255])
            .expect("highlighted button");
        renderer.set_hud_screen_blur(false);
        renderer.set_hud_death_screen(Some(HudDeathScreen::Main(HudDeathMainScreen {
            generation: 1,
            title: "T".to_string(),
            cause_runs: vec![HudComponentTextRun::plain("C")],
            cause_link_run_indices: vec![None],
            score_runs: vec![HudComponentTextRun {
                visual: HudStyledTextRun {
                    text: "S".to_string(),
                    color: Some(0xFF_FF_55),
                    ..HudStyledTextRun::default()
                },
                font: None,
            }],
            respawn_label: String::new(),
            title_screen_label: String::new(),
            buttons_active: false,
            focused: HudDeathScreenButton::TitleScreen,
            hovered: None,
        })));
        assert!(
            renderer.collect_hud_draws().screen_blur.is_none(),
            "DeathScreen does not request menu blur"
        );

        let pixels = renderer
            .render_offscreen_frame()
            .expect("death screen frame");
        let top = pixels.pixel(4, 4);
        let bottom = pixels.pixel(4, HEIGHT - 5);
        let center = pixels.pixel(WIDTH / 2, HEIGHT / 2);
        let beside_center = pixels.pixel(WIDTH / 2 + 8, HEIGHT / 2);
        let title = pixels.pixel(WIDTH / 2, 34);
        let cause = pixels.pixel(WIDTH / 2, 88);
        let score = pixels.pixel(WIDTH / 2, 103);
        let disabled_button = pixels.pixel(WIDTH / 2 - 95, HEIGHT / 4 + 76);

        assert!(
            bottom[0] > top[0],
            "death background must retain its red top-to-bottom gradient, top={top:?}, bottom={bottom:?}"
        );
        assert!(
            center[0] > beside_center[0] && center[2] < beside_center[2],
            "the translucent death gradient must be composited after the ordinary red crosshair, center={center:?}, beside={beside_center:?}"
        );
        assert!(
            title[0] > 220 && title[1] > 220 && title[2] > 220,
            "2x title glyph must be rendered at y=30, got {title:?}"
        );
        assert!(
            cause[0] > 220 && cause[1] > 220 && cause[2] > 220,
            "cause glyph must be rendered at y=85, got {cause:?}"
        );
        assert!(
            score[0] > score[2].saturating_add(40) && score[1] > score[2].saturating_add(40),
            "score argument must use Vanilla yellow at y=100, got {score:?}"
        );
        assert!(
            disabled_button[0].abs_diff(disabled_button[1]) < 8
                && disabled_button[1].abs_diff(disabled_button[2]) < 8,
            "the delayed main button must use the disabled sprite, got {disabled_button:?}"
        );

        renderer.set_hud_death_screen(Some(HudDeathScreen::Main(HudDeathMainScreen {
            generation: 1,
            title: String::new(),
            cause_runs: Vec::new(),
            cause_link_run_indices: Vec::new(),
            score_runs: Vec::new(),
            respawn_label: String::new(),
            title_screen_label: String::new(),
            buttons_active: true,
            focused: HudDeathScreenButton::TitleScreen,
            hovered: None,
        })));
        let enabled = renderer
            .render_offscreen_frame()
            .expect("enabled death buttons")
            .pixel(WIDTH / 2 - 95, HEIGHT / 4 + 76);
        assert!(
            enabled[1] > 220 && enabled[0] < 32,
            "enabled unfocused button must use the normal sprite, got {enabled:?}"
        );

        renderer.set_hud_death_screen(Some(HudDeathScreen::Main(HudDeathMainScreen {
            generation: 1,
            title: String::new(),
            cause_runs: Vec::new(),
            cause_link_run_indices: Vec::new(),
            score_runs: Vec::new(),
            respawn_label: String::new(),
            title_screen_label: String::new(),
            buttons_active: true,
            focused: HudDeathScreenButton::TitleScreen,
            hovered: Some(HudDeathScreenButton::Respawn),
        })));
        let hovered = renderer
            .render_offscreen_frame()
            .expect("hovered death button")
            .pixel(WIDTH / 2 - 95, HEIGHT / 4 + 76);
        assert!(
            hovered[0] > 220 && hovered[1] > 220 && hovered[2] < 32,
            "hovered button must use the highlighted sprite, got {hovered:?}"
        );

        renderer.set_hud_death_screen(Some(HudDeathScreen::QuitConfirmation(
            HudDeathQuitConfirmationScreen {
                generation: 1,
                title: "T".to_string(),
                title_screen_label: String::new(),
                respawn_label: String::new(),
                buttons_active: true,
                focused: HudDeathScreenButton::QuitRespawn,
                hovered: None,
            },
        )));
        let quit = renderer
            .render_offscreen_frame()
            .expect("quit confirmation")
            .pixel(12, 132);
        assert!(
            quit[1] > 220 && quit[0] < 32,
            "quit confirmation must render its 150-wide first button, got {quit:?}"
        );

        renderer.set_hud_death_screen(Some(HudDeathScreen::LinkConfirmation(
            HudDeathLinkConfirmationScreen {
                generation: 1,
                title: "T".to_string(),
                message: "C".to_string(),
                open_label: String::new(),
                copy_label: String::new(),
                cancel_label: String::new(),
                focused: HudDeathScreenButton::LinkCancel,
                hovered: None,
            },
        )));
        let link = renderer
            .render_offscreen_frame()
            .expect("link confirmation")
            .pixel(12, 138);
        assert!(
            link[1] > 220 && link[0] < 32,
            "link confirmation must render its 100-wide first button, got {link:?}"
        );
    }
}
