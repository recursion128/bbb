use anyhow::{bail, Context, Result};
use bbb_render_types::{CoreShaderSourceProjection, PostEffectResourceProjection};

use crate::{
    post_effect_graph::PostEffectGraphRuntime,
    renderer::{CorePipelines, PackResources},
    Renderer,
};

/// The accepted resource generation, kept whole while a reload stages the next one in the
/// renderer. Commit drops it; a failed stage puts it back, so no owner is re-decoded or
/// re-uploaded and no mixed generation escapes the render-thread barrier.
pub(crate) struct RendererResourceReloadCheckpoint {
    pack: PackResources,
    // Font uploads rewrap live chat; rollback restores the accepted wrapping.
    chat_lines: Vec<crate::hud::HudChatLine>,
    pipelines: Option<CorePipelines>,
    core_shader_generation: Option<u64>,
    post_effect_graphs: Option<Option<PostEffectGraphRuntime>>,
    counters: crate::RendererCounters,
}

impl Renderer {
    pub(crate) fn reload_core_shaders(
        &mut self,
        resources: &CoreShaderSourceProjection,
        allow_embedded_fallback: bool,
    ) -> Result<()> {
        let shaders = crate::renderer::SelectedCoreShaders::adapt(
            resources,
            self.terrain_texture_filtering,
            allow_embedded_fallback,
        )?;
        let next = (shaders != self.pipelines.shaders)
            .then(|| {
                crate::post_effect_graph::capture_gpu_errors(&self.device, || {
                    crate::renderer::CorePipelines::new(&self.core_pipeline_layouts(), shaders)
                })
            })
            .transpose()?;
        if let Some(next) = next {
            let entity_vertex_layout_changed = next.shaders.uses_embedded_entity()
                != self.pipelines.shaders.uses_embedded_entity();
            let previous = std::mem::replace(&mut self.pipelines, next);
            if entity_vertex_layout_changed {
                self.refresh_entity_model_meshes();
            }
            if let Some(checkpoint) = self.resource_reload_checkpoint.as_mut() {
                checkpoint.pipelines.get_or_insert(previous);
            }
        }
        self.core_shader_generation = Some(resources.generation());
        Ok(())
    }

    pub fn core_shader_generation(&self) -> Option<u64> {
        self.core_shader_generation
    }

    pub(crate) fn reload_post_effect_graphs(
        &mut self,
        resources: &PostEffectResourceProjection,
    ) -> Result<()> {
        let next = PostEffectGraphRuntime::new(
            &self.device,
            &self.queue,
            self.config.format,
            self.config.width,
            self.config.height,
            resources,
        )?;
        let previous = self.post_effect_graphs.replace(next);
        if let Some(checkpoint) = self.resource_reload_checkpoint.as_mut() {
            if checkpoint.post_effect_graphs.is_none() {
                checkpoint.post_effect_graphs = Some(previous);
            } else if let Some(replaced) = previous {
                replaced.retire(&self.queue);
            }
        } else if let Some(replaced) = previous {
            replaced.retire(&self.queue);
        }
        Ok(())
    }

    pub fn post_effect_graph_generation(&self) -> Option<u64> {
        self.post_effect_graphs
            .as_ref()
            .map(PostEffectGraphRuntime::generation)
    }

    /// Installs one CPU-prepared shader generation and any generation-matched
    /// renderer resources in a single render-thread transaction.
    pub fn transactional_shader_resource_reload<T>(
        &mut self,
        resources: &PostEffectResourceProjection,
        allow_embedded_fallback: bool,
        apply: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        if resources.core_shader_sources().generation() != resources.generation() {
            bail!(
                "core shader generation {} does not match post-effect generation {}",
                resources.core_shader_sources().generation(),
                resources.generation()
            );
        }
        self.transactional_resource_reload(|renderer| {
            renderer
                .reload_core_shaders(resources.core_shader_sources(), allow_embedded_fallback)
                .context("compile core shader pipelines during resource reload")?;
            renderer
                .reload_post_effect_graphs(resources)
                .context("compile post-effect graphs during resource reload")?;
            apply(renderer)
        })
    }

    /// Runs renderer-owned resource updates as one all-or-nothing transaction.
    /// Crate-internal focused tests also use this to inject failures between
    /// individual owners.
    pub(crate) fn transactional_resource_reload<T>(
        &mut self,
        apply: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        self.begin_resource_reload()?;
        match apply(self) {
            Ok(value) => {
                if let Some(checkpoint) = self.resource_reload_checkpoint.take() {
                    checkpoint.pack.retire(&self.queue);
                    if let Some(Some(previous_graphs)) = checkpoint.post_effect_graphs {
                        previous_graphs.retire(&self.queue);
                    }
                }
                self.sync_panorama_resource_counters();
                Ok(value)
            }
            Err(error) => {
                self.rollback_resource_reload();
                Err(error)
            }
        }
    }

    pub(super) fn resource_reload_in_progress(&self) -> bool {
        self.resource_reload_checkpoint.is_some()
    }

    fn begin_resource_reload(&mut self) -> Result<()> {
        assert!(
            self.resource_reload_checkpoint.is_none(),
            "renderer resource reload transactions cannot nest"
        );
        let next = PackResources::new(
            &self.pack_context(),
            self.pack.item_model_residency.empty_for_resource_reload(),
            self.pack
                .entity_geometry
                .empty_for_resource_reload(&self.device),
        )?;
        let pack = std::mem::replace(&mut self.pack, next);
        self.resource_reload_checkpoint = Some(RendererResourceReloadCheckpoint {
            pack,
            chat_lines: self.hud_chat_lines.clone(),
            pipelines: None,
            core_shader_generation: self.core_shader_generation,
            post_effect_graphs: None,
            counters: self.counters.clone(),
        });
        Ok(())
    }

    fn rollback_resource_reload(&mut self) {
        let checkpoint = self
            .resource_reload_checkpoint
            .take()
            .expect("resource reload rollback without a checkpoint");
        let aborted_pack = std::mem::replace(&mut self.pack, checkpoint.pack);
        aborted_pack.retire(&self.queue);
        self.hud_chat_lines = checkpoint.chat_lines;
        if let Some(pipelines) = checkpoint.pipelines {
            self.pipelines = pipelines;
        }
        self.core_shader_generation = checkpoint.core_shader_generation;
        if let Some(post_effect_graphs) = checkpoint.post_effect_graphs {
            if let Some(aborted_graphs) =
                std::mem::replace(&mut self.post_effect_graphs, post_effect_graphs)
            {
                aborted_graphs.retire(&self.queue);
            }
        }
        self.counters = checkpoint.counters;
        self.sync_panorama_resource_counters();
        self.sky.refresh_celestials(
            &self.device,
            &mut self.frame_uploads,
            self.pack.celestial_atlas.as_ref(),
        );
        // Cloud dimensions are written to a permanent uniform buffer. Restore
        // its contents after restoring the exact previous CPU texture owner.
        self.restore_cloud_uniform_after_resource_reload();
    }
}

#[cfg(test)]
mod tests {
    mod gui_highlight;
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::Arc,
    };

    use anyhow::{anyhow, Result};
    use bbb_render_types::{CoreShaderSourceProjection, PostEffectResourceProjection};

    use crate::{
        entity_model_texture_refs, ClearColor, EntityModelTextureImage, HudAsciiGlyph,
        HudChatMessage, HudComponentTextRun, HudCrosshairAttackIndicator, HudFontGlyphBook,
        HudFontGlyphMap, HudVignette, ParticleSpriteUv, ParticleUvRect, Renderer,
    };

    #[test]
    fn shader_resource_generation_rejects_mismatched_projection_atomically() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        let previous_core = renderer.core_shader_generation();
        let previous_post = renderer.post_effect_graph_generation();
        let resources = PostEffectResourceProjection::new(
            2,
            Arc::new(BTreeMap::new()),
            Arc::new(BTreeSet::new()),
            CoreShaderSourceProjection::new(1, Arc::new(BTreeMap::new())),
            Arc::new(BTreeMap::new()),
        );

        let error = renderer
            .transactional_shader_resource_reload(&resources, false, |_| Ok(()))
            .expect_err("mismatched shader generation should be rejected");
        assert!(error
            .to_string()
            .contains("core shader generation 1 does not match post-effect generation 2"));
        assert_eq!(renderer.core_shader_generation(), previous_core);
        assert_eq!(renderer.post_effect_graph_generation(), previous_post);
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
    fn local_shader_resource_generation_commits_and_rolls_back_as_one_owner() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        let roots = bbb_client_assets::PackRoots::from_environment()
            .expect("discover local Vanilla resources");
        let selected = roots.selected_client_packs_or_unmanaged();
        let first_roots = roots
            .clone()
            .with_client_resource_generation(91, selected.clone())
            .expect("prepare first selected generation");
        let second_roots = roots
            .with_client_resource_generation(92, selected)
            .expect("prepare second selected generation");
        let first = bbb_client_assets::PostEffectCatalog::load(&first_roots)
            .expect("prepare first shader resource generation")
            .renderer_projection();
        let second = bbb_client_assets::PostEffectCatalog::load(&second_roots)
            .expect("prepare second shader resource generation")
            .renderer_projection();

        renderer
            .transactional_shader_resource_reload(&first, false, |_| Ok(()))
            .expect("install first shader resource generation");
        assert_eq!(renderer.core_shader_generation(), Some(91));
        assert_eq!(renderer.post_effect_graph_generation(), Some(91));
        renderer.counters.draw_calls = 123;

        renderer
            .transactional_shader_resource_reload(&second, false, |renderer| -> Result<()> {
                renderer.counters.draw_calls = 999;
                anyhow::bail!("injected failure after shader generation commit")
            })
            .expect_err("later owner failure should roll back the whole shader generation");
        assert_eq!(renderer.core_shader_generation(), Some(91));
        assert_eq!(renderer.post_effect_graph_generation(), Some(91));
        assert_eq!(renderer.counters.draw_calls, 123);

        renderer
            .transactional_shader_resource_reload(&second, false, |_| Ok(()))
            .expect("install second shader resource generation");
        assert_eq!(renderer.core_shader_generation(), Some(92));
        assert_eq!(renderer.post_effect_graph_generation(), Some(92));
    }

    #[test]
    fn injected_failure_after_each_reload_stage_restores_exact_accepted_owners() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        let old_rgba = [255, 0, 0, 255].repeat(16);
        renderer
            .add_test_gui_sprite("hud/crosshair", 4, 4, &old_rgba)
            .expect("old crosshair");
        renderer
            .add_test_gui_sprite(
                "hud/crosshair_attack_indicator_full",
                16,
                16,
                &[0, 255, 0, 255].repeat(256),
            )
            .expect("old full attack indicator");
        renderer.set_hud_crosshair_attack_indicator(HudCrosshairAttackIndicator::Full);
        for fail_after in 0..6 {
            let previous_counters = renderer.counters().clone();
            let error = renderer
                .transactional_resource_reload(|renderer| -> Result<()> {
                    renderer.upload_particle_atlas(
                        1,
                        1,
                        &[255, 255, 255, 255],
                        vec![ParticleSpriteUv {
                            uv: ParticleUvRect {
                                min: [0.0, 0.0],
                                max: [1.0, 1.0],
                            },
                            has_translucent: false,
                        }],
                    )?;
                    renderer.upload_item_atlas(1, 1, &[255, 255, 255, 255])?;
                    fail_at(0, fail_after)?;

                    renderer.upload_terrain_texture_atlas(2, 2, &[255; 16])?;
                    fail_at(1, fail_after)?;

                    renderer.add_test_gui_sprite(
                        "hud/crosshair",
                        2,
                        2,
                        &[0, 0, 255, 255].repeat(4),
                    )?;
                    renderer.add_test_gui_sprite(
                        "hud/crosshair_attack_indicator_full",
                        16,
                        16,
                        &[255, 0, 255, 255].repeat(256),
                    )?;
                    fail_at(2, fail_after)?;

                    let texture = entity_model_texture_refs()[0];
                    let rgba = vec![255; (texture.size[0] * texture.size[1] * 4) as usize];
                    renderer.upload_entity_model_textures(&[EntityModelTextureImage::new(
                        texture, rgba,
                    )])?;
                    fail_at(3, fail_after)?;

                    renderer.upload_end_sky_texture(1, 1, &[255, 255, 255, 255])?;
                    fail_at(4, fail_after)?;

                    renderer.invalidate_map_texture_residency();
                    renderer.invalidate_item_model_resources();
                    fail_at(5, fail_after)
                })
                .expect_err("injected stage failure");
            assert_eq!(
                error.to_string(),
                format!("injected failure after stage {fail_after}")
            );
            assert!(renderer.pack.particle_atlas.is_none());
            assert!(renderer.pack.item_entity_atlas.is_none());
            assert!(renderer.pack.entity_model_texture_atlas.is_none());
            assert!(renderer.pack.end_sky_texture.is_none());
            assert_eq!(renderer.counters(), previous_counters);

            let pixels = renderer
                .render_offscreen_frame()
                .expect("frame after rollback");
            let center = pixels.pixel(16, 16);
            assert!(
                center[0] > 128 && center[2] < 128,
                "rollback should retain the old red HUD owner, got {center:?}"
            );
            let attack_indicator = pixels.pixel(16, 25);
            assert!(
                attack_indicator[1] > 128 && attack_indicator[0] < 128 && attack_indicator[2] < 128,
                "rollback should retain the old green attack indicator, got {attack_indicator:?}"
            );
        }
    }

    #[test]
    fn reload_installs_one_whole_generation_and_failure_keeps_the_accepted_one() {
        let scene = || -> Option<Renderer> {
            let mut renderer = Renderer::new_offscreen(32, 32)?;
            let vertex = |position| {
                crate::gpu::TerrainGpuVertex::pack(
                    position,
                    [0.5, 0.5],
                    0x00F0_00F0,
                    crate::terrain::TerrainTint::WHITE,
                    1.0,
                    1.0,
                )
            };
            renderer.show_test_terrain_mesh(crate::terrain::TerrainMesh {
                vertices: vec![
                    vertex([-0.75, -0.75, 0.0]),
                    vertex([0.75, -0.75, 0.0]),
                    vertex([0.75, 0.75, 0.0]),
                    vertex([-0.75, 0.75, 0.0]),
                ],
                indices: vec![0, 1, 2, 0, 2, 3],
                source_sections: 1,
                opaque_faces: 1,
                ..crate::terrain::TerrainMesh::default()
            });
            Some(renderer)
        };
        let upload = |renderer: &mut Renderer, terrain: [u8; 4], crosshair: [u8; 4]| {
            renderer.upload_terrain_texture_atlas(1, 1, &terrain)?;
            renderer.add_test_gui_sprite("hud/crosshair", 4, 4, &crosshair.repeat(16))
        };
        let frame = |renderer: &mut Renderer| {
            renderer.render_offscreen_frame().unwrap();
            renderer.render_offscreen_frame().unwrap().rgba
        };
        let Some(mut renderer) = scene() else {
            return;
        };
        upload(&mut renderer, [255, 0, 0, 255], [0, 255, 0, 255]).unwrap();
        renderer
            .upload_particle_atlas(
                1,
                1,
                &[255; 4],
                vec![ParticleSpriteUv {
                    uv: ParticleUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    has_translucent: false,
                }],
            )
            .unwrap();
        let accepted = frame(&mut renderer);

        renderer
            .transactional_resource_reload(|renderer| -> Result<()> {
                upload(renderer, [0, 0, 255, 255], [255, 0, 255, 255])?;
                Err(anyhow!("injected failure after every upload"))
            })
            .expect_err("injected reload failure");
        assert!(renderer.pack.particle_atlas.is_some());
        assert!(
            frame(&mut renderer) == accepted,
            "rollback keeps the accepted terrain atlas and HUD"
        );

        renderer
            .transactional_resource_reload(|renderer| {
                upload(renderer, [0, 0, 255, 255], [255, 0, 255, 255])
            })
            .expect("commit reload");
        assert!(renderer.resource_reload_checkpoint.is_none());
        assert!(
            renderer.pack.particle_atlas.is_none(),
            "an owner the new generation did not upload is gone"
        );
        let mut fresh = scene().unwrap();
        upload(&mut fresh, [0, 0, 255, 255], [255, 0, 255, 255]).unwrap();
        let committed = frame(&mut renderer);
        assert!(committed != accepted);
        assert!(
            committed == frame(&mut fresh),
            "the committed generation is the uploaded one"
        );
    }

    #[test]
    fn failed_vignette_reload_restores_the_previous_pixel_owner() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_hud_vignette(Some(HudVignette {
            brightness: 0.5,
            border_warning_strength: 0.5,
            color: [0.25, 0.5, 0.75],
        }));
        renderer
            .upload_hud_vignette_texture(1, 1, &[255, 255, 255, 255], true, false)
            .expect("old vignette");
        let before = renderer
            .render_offscreen_frame()
            .expect("frame before reload")
            .pixel(2, 2);

        renderer
            .transactional_resource_reload(|renderer| -> Result<()> {
                renderer.upload_hud_vignette_texture(1, 1, &[0, 0, 0, 255], true, false)?;
                Err(anyhow!("injected vignette failure"))
            })
            .expect_err("injected reload failure");

        let after = renderer
            .render_offscreen_frame()
            .expect("frame after rollback")
            .pixel(2, 2);
        assert_eq!(after, before);
    }

    #[test]
    fn failed_spyglass_scope_reload_restores_the_previous_pixel_owner() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_hud_spyglass_scope(Some(crate::HudSpyglassScope { scale: 1.0 }));
        renderer
            .upload_gui_texture(
                "textures/misc/spyglass_scope.png",
                1,
                1,
                &[255, 0, 0, 255],
                crate::StandaloneTextureSampling::default(),
            )
            .expect("old scope");
        let before = renderer
            .render_offscreen_frame()
            .expect("frame before reload")
            .pixel(8, 8);

        renderer
            .transactional_resource_reload(|renderer| -> Result<()> {
                renderer.upload_gui_texture(
                    "textures/misc/spyglass_scope.png",
                    1,
                    1,
                    &[0, 0, 255, 255],
                    crate::StandaloneTextureSampling::default(),
                )?;
                Err(anyhow!("injected scope failure"))
            })
            .expect_err("injected reload failure");

        let after = renderer
            .render_offscreen_frame()
            .expect("frame after rollback")
            .pixel(8, 8);
        assert_eq!(after, before);
        assert!(after[0] > 128 && after[2] < 128);
    }

    #[test]
    fn failed_hud_font_reload_restores_exact_book_and_wrapped_chat_cache() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        let mut old_glyphs = HudFontGlyphMap::new();
        old_glyphs.insert_first_wins(
            'a',
            HudAsciiGlyph {
                advance: 6.0,
                ..HudAsciiGlyph::default()
            },
        );
        let mut old_custom_glyphs = HudFontGlyphMap::new();
        old_custom_glyphs.insert_first_wins(
            'a',
            HudAsciiGlyph {
                advance: 9.0,
                ..HudAsciiGlyph::default()
            },
        );
        old_glyphs.set_missing_glyph(HudAsciiGlyph {
            advance: 6.0,
            width: 5,
            height: 8,
            ..Default::default()
        });
        old_glyphs.insert_first_wins(
            '1',
            HudAsciiGlyph {
                advance: 2.5,
                ..Default::default()
            },
        );
        let old_fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            std::collections::BTreeMap::from([
                ("example:old".to_string(), old_custom_glyphs),
                ("minecraft:default".to_string(), old_glyphs),
            ]),
        );
        renderer
            .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], old_fonts)
            .expect("old font");
        renderer.set_default_text_right_to_left(false);
        renderer.set_hud_chat_messages(vec![HudChatMessage {
            display_sequence: 1,
            added_tick: 0,
            runs: vec![HudComponentTextRun {
                visual: crate::HudStyledTextRun::plain("a".repeat(60)),
                font: Some("example:old".to_string()),
            }],
            interactions: Vec::new(),
            tag: None,
        }]);
        let previous_lines = renderer.hud_chat_lines.clone();
        let previous_fonts = renderer.pack.hud_font_book.clone();
        let previous_digits = renderer.pack.hud_digit_glyphs;
        assert_eq!(previous_digits[1].advance, 2.5);
        assert_eq!(previous_digits[2].advance, 6.0);
        assert_eq!(previous_digits[2].width, 5);

        let error = renderer
            .transactional_resource_reload(|renderer| -> Result<()> {
                let mut new_glyphs = HudFontGlyphMap::new();
                new_glyphs.insert_first_wins(
                    'a',
                    HudAsciiGlyph {
                        advance: 1.0,
                        ..HudAsciiGlyph::default()
                    },
                );
                new_glyphs.set_missing_glyph(HudAsciiGlyph {
                    advance: 9.0,
                    width: 5,
                    height: 8,
                    ..Default::default()
                });
                let new_fonts = HudFontGlyphBook::from_fonts(
                    "minecraft:default",
                    std::collections::BTreeMap::from([(
                        "minecraft:default".to_string(),
                        new_glyphs,
                    )]),
                );
                renderer.upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], new_fonts)?;
                renderer.set_default_text_right_to_left(true);
                assert_ne!(renderer.hud_chat_lines, previous_lines);
                assert_ne!(renderer.pack.hud_font_book, previous_fonts);
                assert_eq!(renderer.pack.hud_digit_glyphs[1].advance, 9.0);
                assert_ne!(renderer.pack.hud_digit_glyphs, previous_digits);
                assert!(renderer.default_text_right_to_left());
                Err(anyhow!("injected font failure"))
            })
            .expect_err("font reload failure");

        assert_eq!(error.to_string(), "injected font failure");
        assert_eq!(renderer.hud_chat_lines, previous_lines);
        assert_eq!(renderer.pack.hud_font_book, previous_fonts);
        assert_eq!(renderer.pack.hud_digit_glyphs, previous_digits);
        assert!(!renderer.default_text_right_to_left());
    }

    #[test]
    fn failed_waypoint_locator_reload_restores_background_style_and_marker_sprite() {
        let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
            return;
        };
        let solid =
            |count: usize, rgba: [u8; 4]| -> Vec<u8> { (0..count).flat_map(|_| rgba).collect() };
        renderer
            .add_test_gui_sprite(
                "hud/locator_bar_background",
                182,
                5,
                &solid(182 * 5, [255, 0, 0, 255]),
            )
            .expect("old background");
        renderer
            .add_test_gui_sprite("old", 9, 9, &solid(9 * 9, [0, 255, 0, 255]))
            .expect("old marker");
        renderer.set_hud_locator_styles(std::collections::BTreeMap::from([(
            "test".to_string(),
            crate::HudLocatorStyle {
                near_distance: 1,
                far_distance: 2,
                sprite_ids: vec!["old".to_string()],
            },
        )]));
        renderer.set_hud_contextual_bar(crate::HudContextualBar::Locator(crate::HudLocatorBar {
            markers: vec![crate::HudLocatorMarker {
                angle_degrees: 0.0,
                pitch: crate::HudLocatorPitch::None,
                distance: 0.0,
                style_id: "test".to_string(),
                color_argb: 0xFFFF_FFFF,
            }],
            client_tick: 0,
        }));

        renderer
            .transactional_resource_reload(|renderer| -> Result<()> {
                renderer.add_test_gui_sprite(
                    "hud/locator_bar_background",
                    182,
                    5,
                    &solid(182 * 5, [0, 0, 255, 255]),
                )?;
                renderer.add_test_gui_sprite("new", 9, 9, &solid(9 * 9, [255, 0, 255, 255]))?;
                renderer.set_hud_locator_styles(std::collections::BTreeMap::from([(
                    "test".to_string(),
                    crate::HudLocatorStyle {
                        near_distance: 1,
                        far_distance: 2,
                        sprite_ids: vec!["new".to_string()],
                    },
                )]));
                Err(anyhow!("injected locator failure"))
            })
            .expect_err("locator reload should fail");

        assert!(renderer.gui_sprite("old").is_some());
        assert!(renderer.gui_sprite("new").is_none());
        let pixels = renderer
            .render_offscreen_frame()
            .expect("frame after rollback");
        let background = pixels.pixel(73, 213);
        assert!(
            background[0] > 128 && background[1] < 128 && background[2] < 128,
            "rollback should restore the old red locator background, got {background:?}"
        );
        let marker = pixels.pixel(160, 214);
        assert!(
            marker[1] > 128 && marker[0] < 128 && marker[2] < 128,
            "rollback should restore the old green locator marker/style, got {marker:?}"
        );
    }

    fn fail_at(stage: usize, fail_after: usize) -> Result<()> {
        if stage == fail_after {
            return Err(anyhow!("injected failure after stage {stage}"));
        }
        Ok(())
    }
}
