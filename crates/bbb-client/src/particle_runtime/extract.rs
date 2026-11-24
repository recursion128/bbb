use bbb_client_render::{
    particles::{
        elder_guardian_particle_model_transform, projectile_pickup_particle_model_transform,
        ElderGuardianParticleRenderInstance, ExperienceOrbPickupParticleRenderInstance,
        ParticleAtlasDrawRange, ParticleAtlasUvs, ParticleBatches, ParticleGpuInstance,
        ParticleQuadBuffer, ParticleQuadSources, ParticleSourceKey,
        ProjectilePickupParticleRenderInstance, SHRIEK_MAGICAL_X_ROT,
    },
    render_core::camera::camera_billboard_orientation,
    CameraPose, ItemPickupParticleRenderState, ParticleItemOptionState,
    ParticleItemPickupProjectileKind, ParticleUvRect, Renderer,
};
use bbb_client_world::{
    particles::{
        ParticleEngine, ParticleInstance, ParticleQuadShape, ParticleRenderGroup,
        ParticleRenderLayer,
    },
    TakeItemEntityPickupProjectileModel,
};
use glam::{EulerRot, Mat4, Quat, Vec3};

/// `ParticleEngine.extract`: what this frame draws of the client-tick engine,
/// at the frame's partial tick. The world revision identifies the immutable
/// particle positions and chunk light for this frame.
#[allow(clippy::too_many_arguments)]
pub(crate) fn extract_particles_to_renderer(
    renderer: &mut Renderer,
    engine: &ParticleEngine,
    particle_runtime: Option<&super::NativeParticleRuntime>,
    world_revision: u64,
    terrain_sections_revision: u64,
    partial_tick: f32,
    camera: Option<(CameraPose, [f64; 3])>,
    culling_fov: f32,
    depth_far: f32,
    occluded: impl Fn([f64; 3], [f64; 3]) -> bool,
    light_at: impl FnMut([f64; 3]) -> [f32; 2],
) {
    let Some(target) =
        renderer.begin_particle_extraction(engine.len(), engine.pending_len(), camera.is_some())
    else {
        return;
    };
    let (pose, eye) = camera.expect("camera checked above");
    let publication = ParticleSourceKey {
        world_revision,
        terrain_sections_revision,
        pose,
        eye,
        aspect: target.aspect,
        culling_fov,
        depth_far,
        item_atlas_loaded: target.item_atlas_loaded,
    };
    if target.sources.publication != Some(publication) {
        let visible = pose.particle_visibility(eye, publication.aspect, culling_fov, depth_far);
        collect_particle_quad_sources(
            target.sources,
            engine,
            particle_runtime,
            target.atlases,
            visible,
            occluded,
        );
        target.sources.publication = Some(publication);
    }
    particle_batches(
        &mut target.frame.quads,
        engine,
        target.sources,
        pose,
        partial_tick,
        target.light.for_revision(world_revision, light_at),
    );
    target.frame.elder_guardians = elder_guardian_particles(engine, pose, partial_tick);
    target.frame.experience_orbs = experience_orb_pickup_particles(engine, pose, partial_tick);
    target.frame.projectiles = projectile_pickup_particles(engine, partial_tick);
    target.frame.live = !engine.is_empty();
}

/// The picked-up item stacks flying to their collectors, for item-model
/// resolution before the frame is drawn.
pub(crate) fn item_pickup_particle_render_states(
    engine: &ParticleEngine,
    partial_tick: f32,
) -> Vec<ItemPickupParticleRenderState> {
    item_pickups(engine, partial_tick)
        .filter_map(|(instance, position)| {
            let pickup = instance.item_pickup.as_deref()?;
            let (item_id, count, component_patch_len) = pickup.item?;
            Some(ItemPickupParticleRenderState {
                source_entity_id: pickup.source_entity_id,
                item: ParticleItemOptionState {
                    item_id,
                    count,
                    component_patch_len,
                },
                component_patch: pickup.component_patch.clone(),
                position,
                age_ticks: pickup.age_ticks,
                bob_offset: pickup.bob_offset,
                light: pickup.light,
            })
        })
        .collect()
}

fn projectile_kind_from_world(
    model: TakeItemEntityPickupProjectileModel,
) -> ParticleItemPickupProjectileKind {
    match model {
        TakeItemEntityPickupProjectileModel::Arrow { tipped: false } => {
            ParticleItemPickupProjectileKind::Arrow
        }
        TakeItemEntityPickupProjectileModel::Arrow { tipped: true } => {
            ParticleItemPickupProjectileKind::TippedArrow
        }
        TakeItemEntityPickupProjectileModel::SpectralArrow => {
            ParticleItemPickupProjectileKind::SpectralArrow
        }
        TakeItemEntityPickupProjectileModel::Trident { foil } => {
            ParticleItemPickupProjectileKind::Trident { foil }
        }
    }
}

fn collect_particle_quad_sources(
    sources: &mut ParticleQuadSources,
    engine: &ParticleEngine,
    particle_runtime: Option<&super::NativeParticleRuntime>,
    atlases: ParticleAtlasUvs<'_>,
    visible: impl Fn([f64; 3]) -> bool,
    occluded: impl Fn([f64; 3], [f64; 3]) -> bool,
) {
    for layer in &mut sources.layers {
        layer.clear();
    }
    for (index, instance) in engine.single_quads().iter().enumerate() {
        if instance.delay_ticks != 0 || !visible(instance.position) {
            continue;
        }
        let extent = f64::from(instance.conservative_cull_extent());
        let min = [0, 1, 2]
            .map(|axis| instance.previous_position[axis].min(instance.position[axis]) - extent);
        let max = [0, 1, 2]
            .map(|axis| instance.previous_position[axis].max(instance.position[axis]) + extent);
        if occluded(min, max) {
            continue;
        }
        let sprite = instance.sprite.or_else(|| {
            if !instance.kind.is_custom() {
                return None;
            }
            particle_runtime?.custom_particle_sprite(
                instance.kind,
                instance.age_ticks,
                instance.lifetime_ticks,
            )
        });
        if let Some((uv, translucent)) =
            atlases.sprite(instance.kind.render_layer().texture_atlas(), sprite)
        {
            let layer = sprite_render_layer(instance.kind.render_layer(), translucent);
            sources.layers[layer as usize].push((index, uv));
        }
    }
}

fn particle_batches(
    batches: &mut ParticleBatches,
    engine: &ParticleEngine,
    sources: &ParticleQuadSources,
    pose: CameraPose,
    partial_tick: f32,
    mut light_at: impl FnMut(usize, [f64; 3]) -> [f32; 2],
) {
    use ParticleRenderLayer::*;
    let instances = engine.single_quads();
    let axes = pose.billboard_axes();
    for (batch, layers) in [
        (&mut batches.opaque, [OpaqueTerrain, OpaqueItems, Opaque]),
        (
            &mut batches.translucent,
            [TranslucentTerrain, TranslucentItems, Translucent],
        ),
    ] {
        batch.clear();
        batch.quads.set_camera_axes(axes.right, axes.up);
        for layer in layers {
            let start = batch.quads.len() as u32;
            for &(index, uv) in &sources.layers[layer as usize] {
                let instance = &instances[index];
                let light = instance.light(partial_tick, || light_at(index, instance.position));
                append_particle_quads(
                    &mut batch.quads,
                    instance,
                    particle_uv_rect_for_instance(instance, uv),
                    partial_tick,
                    light,
                );
            }
            let end = batch.quads.len() as u32;
            if start != end {
                batch.draws.push(ParticleAtlasDrawRange {
                    texture_atlas: layer.texture_atlas(),
                    instances: start..end,
                });
            }
        }
    }
}

fn elder_guardian_particles(
    engine: &ParticleEngine,
    pose: CameraPose,
    partial_tick: f32,
) -> Vec<ElderGuardianParticleRenderInstance> {
    engine
        .instances_in_group(ParticleRenderGroup::ElderGuardians)
        .filter(|instance| instance.delay_ticks == 0)
        .map(|instance| {
            let age_scale =
                (instance.age_ticks as f32 + partial_tick) / instance.lifetime_ticks.max(1) as f32;
            ElderGuardianParticleRenderInstance {
                transform: elder_guardian_particle_model_transform(pose, age_scale),
                tint: [
                    1.0,
                    1.0,
                    1.0,
                    0.05 + 0.5 * (age_scale * std::f32::consts::PI).sin(),
                ],
            }
        })
        .collect()
}

fn experience_orb_pickup_particles(
    engine: &ParticleEngine,
    pose: CameraPose,
    partial_tick: f32,
) -> Vec<ExperienceOrbPickupParticleRenderInstance> {
    item_pickups(engine, partial_tick)
        .filter_map(|(instance, position)| {
            let pickup = instance.item_pickup.as_deref()?;
            let icon = pickup.experience_orb_icon?;
            Some(ExperienceOrbPickupParticleRenderInstance {
                transform: Mat4::from_translation(Vec3::from_array(position) + Vec3::Y * 0.1)
                    * camera_billboard_orientation(pose)
                    * Mat4::from_scale(Vec3::splat(0.3)),
                icon,
                age_ticks: pickup.age_ticks,
                light: pickup.light,
            })
        })
        .collect()
}

fn projectile_pickup_particles(
    engine: &ParticleEngine,
    partial_tick: f32,
) -> Vec<ProjectilePickupParticleRenderInstance> {
    item_pickups(engine, partial_tick)
        .filter_map(|(instance, position)| {
            let pickup = instance.item_pickup.as_deref()?;
            let kind = projectile_kind_from_world(pickup.projectile_model?);
            Some(ProjectilePickupParticleRenderInstance {
                transform: projectile_pickup_particle_model_transform(
                    kind,
                    pickup.y_rot,
                    pickup.x_rot,
                    position,
                ),
                kind,
                light: pickup.light,
            })
        })
        .collect()
}

fn item_pickups(
    engine: &ParticleEngine,
    partial_tick: f32,
) -> impl Iterator<Item = (&ParticleInstance, [f32; 3])> {
    engine
        .instances_in_group(ParticleRenderGroup::ItemPickup)
        .filter(|instance| instance.delay_ticks == 0)
        .filter_map(move |instance| {
            let position = instance.item_pickup_position(partial_tick)?;
            Some((instance, position.map(|value| value as f32)))
        })
}

/// Terrain and item particles render translucent when their sprite has
/// translucent texels (`TerrainParticle`/`BreakingItemParticle.getLayer`).
fn sprite_render_layer(
    layer: ParticleRenderLayer,
    translucent_sprite: bool,
) -> ParticleRenderLayer {
    match (layer, translucent_sprite) {
        (ParticleRenderLayer::OpaqueTerrain | ParticleRenderLayer::TranslucentTerrain, true) => {
            ParticleRenderLayer::TranslucentTerrain
        }
        (ParticleRenderLayer::OpaqueTerrain | ParticleRenderLayer::TranslucentTerrain, false) => {
            ParticleRenderLayer::OpaqueTerrain
        }
        (ParticleRenderLayer::OpaqueItems | ParticleRenderLayer::TranslucentItems, true) => {
            ParticleRenderLayer::TranslucentItems
        }
        (ParticleRenderLayer::OpaqueItems | ParticleRenderLayer::TranslucentItems, false) => {
            ParticleRenderLayer::OpaqueItems
        }
        (layer, _) => layer,
    }
}

/// `TerrainParticle`/`BreakingItemParticle` draw a random quarter of their sprite.
fn particle_uv_rect_for_instance(
    instance: &ParticleInstance,
    uv: ParticleUvRect,
) -> ParticleUvRect {
    let Some(sub_rect) = instance.atlas_uv_sub_rect else {
        return uv;
    };
    let u_span = uv.max[0] - uv.min[0];
    let v_span = uv.max[1] - uv.min[1];
    ParticleUvRect {
        min: [
            uv.min[0] + u_span * ((sub_rect.u_offset + 1.0) / 4.0),
            uv.min[1] + v_span * (sub_rect.v_offset / 4.0),
        ],
        max: [
            uv.min[0] + u_span * (sub_rect.u_offset / 4.0),
            uv.min[1] + v_span * ((sub_rect.v_offset + 1.0) / 4.0),
        ],
    }
}

fn append_particle_quads(
    quads: &mut ParticleQuadBuffer,
    instance: &ParticleInstance,
    uv: ParticleUvRect,
    partial_tick: f32,
    light: [f32; 2],
) {
    let center = Vec3::from_array(
        instance
            .render_position(partial_tick)
            .map(|value| value as f32),
    );
    let half_size = instance.quad_size(partial_tick);
    let tint = instance.render_color(partial_tick);
    let uv = [uv.min[0], uv.min[1], uv.max[0], uv.max[1]];
    let rotated =
        |rotation: Quat| ParticleGpuInstance::rotated(center, half_size, uv, rotation, tint, light);
    match instance.kind.quad_shape() {
        ParticleQuadShape::Shriek => {
            // `ShriekParticle.extract`: two quads tilted by the "magical" x rotation.
            quads.push_instance(rotated(Quat::from_rotation_x(-SHRIEK_MAGICAL_X_ROT)));
            quads.push_instance(rotated(Quat::from_euler(
                EulerRot::YXZ,
                -std::f32::consts::PI,
                SHRIEK_MAGICAL_X_ROT,
                0.0,
            )));
        }
        ParticleQuadShape::Vibration => {
            // `VibrationSignalParticle.extract`.
            let sway = ((instance.age_ticks as f32 + partial_tick - std::f32::consts::TAU) * 0.05)
                .sin()
                * 2.0;
            let yaw = lerp(partial_tick, instance.previous_yaw, instance.yaw);
            let pitch = lerp(partial_tick, instance.previous_pitch, instance.pitch)
                + std::f32::consts::FRAC_PI_2;
            quads.push_instance(rotated(
                Quat::from_rotation_y(yaw)
                    * Quat::from_rotation_x(-pitch)
                    * Quat::from_rotation_y(sway),
            ));
            quads.push_instance(rotated(
                Quat::from_rotation_y(-std::f32::consts::PI + yaw)
                    * Quat::from_rotation_x(pitch)
                    * Quat::from_rotation_y(sway),
            ));
        }
        ParticleQuadShape::Billboard => {
            let (sin_roll, cos_roll) = if instance.roll == 0.0 {
                (0.0, 1.0)
            } else {
                lerp(partial_tick, instance.previous_roll, instance.roll).sin_cos()
            };
            quads.push_instance(ParticleGpuInstance::billboard(
                center,
                half_size,
                uv,
                instance.kind.facing_camera_mode(),
                cos_roll,
                sin_roll,
                tint,
                light,
            ));
        }
    }
}

fn lerp(alpha: f32, start: f32, end: f32) -> f32 {
    start + alpha * (end - start)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_render::{particles::ParticleLightSamples, ParticleSpriteUv};
    use bbb_client_world::particles::{ParticleKind, ParticleSpawn, ParticleSpriteTable};

    #[test]
    fn visible_particle_quads_follow_atlas_and_projection_within_one_tick() {
        let Some(mut renderer) = Renderer::new_offscreen(320, 180) else {
            return;
        };
        let mut engine = ParticleEngine::default();
        engine.set_sprites(ParticleSpriteTable::new(
            |_| vec![0].into_boxed_slice(),
            [None; 3],
        ));
        engine.spawn(ParticleSpawn::new(
            ParticleKind::SMOKE,
            [4.0, 0.0, 9.0],
            [0.0; 3],
        ));
        engine.advance(1, &bbb_client_world::WorldStore::new(), Default::default());
        let camera = Some((
            CameraPose {
                position: [0.0; 3],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 0.0,
            },
            [0.0; 3],
        ));
        let frame = |renderer: &mut Renderer, camera, fov| {
            extract_particles_to_renderer(
                renderer,
                &engine,
                None,
                7,
                0,
                0.5,
                camera,
                fov,
                2048.0,
                |_, _| false,
                |_| [0.0; 2],
            );
            renderer
                .particle_frame()
                .quads
                .opaque
                .quads
                .first()
                .map(|quad| quad.uv)
        };
        assert_eq!(frame(&mut renderer, camera, 70.0), None);
        for min in [0.0, 0.25] {
            let sprite = ParticleSpriteUv {
                uv: ParticleUvRect {
                    min: [min; 2],
                    max: [1.0; 2],
                },
                has_translucent: false,
            };
            renderer
                .upload_particle_atlas(1, 1, &[255; 4], vec![sprite])
                .unwrap();
            assert_eq!(
                frame(&mut renderer, camera, 70.0),
                Some([min, min, 1.0, 1.0])
            );
        }
        assert_eq!(frame(&mut renderer, camera, 5.0), None);
        assert_eq!(frame(&mut renderer, None, 70.0), None);
        assert_eq!(
            frame(&mut renderer, camera, 70.0),
            Some([0.25, 0.25, 1.0, 1.0])
        );
    }

    #[test]
    fn particle_light_reuses_world_samples_as_camera_and_partial_tick_change() {
        let mut engine = ParticleEngine::default();
        engine.set_sprites(ParticleSpriteTable::new(
            |_| vec![0].into_boxed_slice(),
            [None; 3],
        ));
        for (kind, x) in [
            (ParticleKind::SMOKE, 0.0),
            (ParticleKind::FLAME, 2.0),
            (ParticleKind::END_ROD, 3.0),
        ] {
            engine.spawn(ParticleSpawn::new(kind, [x, 0.0, 0.0], [0.0; 3]));
        }
        engine.advance(1, &bbb_client_world::WorldStore::new(), Default::default());
        let atlas = ParticleAtlasUvs {
            particles: &[ParticleUvRect {
                min: [0.0; 2],
                max: [1.0; 2],
            }],
            terrain: &[],
            items: &[],
        };
        let camera = CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        };
        let mut samples = ParticleLightSamples::default();
        let mut sources = ParticleQuadSources::default();
        let mut batches = ParticleBatches::default();
        let mut frame = |revision, partial_tick, min_x, sky| {
            collect_particle_quad_sources(
                &mut sources,
                &engine,
                None,
                atlas,
                |pos| pos[0] >= min_x,
                |_, _| false,
            );
            let mut reads = Vec::new();
            particle_batches(
                &mut batches,
                &engine,
                &sources,
                camera,
                partial_tick,
                samples.for_revision(revision, |pos| {
                    reads.push(pos[0]);
                    [0.0, sky]
                }),
            );
            (batches.clone(), reads)
        };
        let (first, reads) = frame(1, 0.0, 1.0, 0.25);
        assert_eq!(reads, [2.0]);
        assert_eq!(first.opaque.quads[0].light_f32(), [0.0, 0.25]);
        assert_eq!(first.translucent.quads[0].light_f32(), [1.0; 2]);

        let (turned, reads) = frame(1, 0.5, 0.0, 0.25);
        assert_eq!(reads, [0.0]);
        assert_eq!(turned.opaque.quads[0].light_f32(), [0.0, 0.25]);
        assert!(turned.opaque.quads[1].light_f32()[0] > first.opaque.quads[0].light_f32()[0]);

        let (relit, reads) = frame(2, 0.5, 0.0, 0.75);
        assert_eq!(reads, [0.0, 2.0]);
        assert_eq!(relit.opaque.quads[1].light_f32()[1], 0.75);

        let (hidden, _) = frame(2, 0.75, 4.0, 0.75);
        assert_eq!(hidden, ParticleBatches::default());
    }

    #[test]
    fn occluded_particles_skip_light_sampling_and_quad_upload_without_affecting_engine() {
        use crate::occlusion_culling::tests::{stone_state_id, test_world_with_blocks};
        use crate::occlusion_culling::OpaqueTerrainOccluder;
        use crate::terrain_upload::TerrainUploadState;

        let stone = stone_state_id();
        let mut blocks = Vec::new();
        for y in 63..=67 {
            for x in -2..=2 {
                blocks.push((x, y, 5, stone));
            }
        }
        let world = test_world_with_blocks(&blocks);
        let mut upload = TerrainUploadState::default();
        upload.mark_section_occluder_ready_for_test(&world, 0, 3, 0, 1.0);
        upload.mark_section_occluder_ready_for_test(&world, 0, 4, 0, 1.0);
        upload.mark_section_occluder_ready_for_test(&world, -1, 3, 0, 1.0);
        upload.mark_section_occluder_ready_for_test(&world, -1, 4, 0, 1.0);

        let mut engine = ParticleEngine::default();
        engine.set_sprites(ParticleSpriteTable::new(
            |_| vec![0].into_boxed_slice(),
            [None; 3],
        ));
        engine.spawn(ParticleSpawn::new(
            ParticleKind::SMOKE,
            [0.5, 65.0, 8.0],
            [0.0; 3],
        ));
        engine.spawn(ParticleSpawn::new(
            ParticleKind::SMOKE,
            [0.5, 65.0, 3.0],
            [0.0; 3],
        ));
        engine.advance(1, &world, Default::default());
        assert_eq!(engine.len(), 2);

        let atlas = ParticleAtlasUvs {
            particles: &[ParticleUvRect {
                min: [0.0; 2],
                max: [1.0; 2],
            }],
            terrain: &[],
            items: &[],
        };
        let eye = [0.5, 65.5, 1.0];
        let camera = CameraPose {
            position: [0.5, 65.5, 1.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        };
        let occluder = OpaqueTerrainOccluder::new(&world, &upload, eye);
        let mut sources = ParticleQuadSources::default();
        collect_particle_quad_sources(
            &mut sources,
            &engine,
            None,
            atlas,
            |_| true,
            |min, max| occluder.is_aabb_occluded(min, max),
        );
        let mut samples = ParticleLightSamples::default();
        let mut batches = ParticleBatches::default();
        let mut sampled_positions = Vec::new();
        particle_batches(
            &mut batches,
            &engine,
            &sources,
            camera,
            0.5,
            samples.for_revision(1, |pos| {
                sampled_positions.push(pos[2]);
                [0.5, 0.5]
            }),
        );
        assert_eq!(sampled_positions, vec![3.0]);
        assert_eq!(batches.opaque.quads.len(), 1);
        assert_eq!(engine.len(), 2);
    }

    #[test]
    fn custom_mod_particle_resolves_from_payload_and_extracts_quad() {
        use bbb_protocol::content_registry::{ContentRegistryBuilder, ModParticleRegistration};
        use bbb_protocol::packets::{ParticlePayload, Vec3d};
        use std::sync::Arc;

        let mut builder = ContentRegistryBuilder::new();
        builder.register_particle(ModParticleRegistration {
            resource_id: "demo:spark".to_string(),
            override_limiter: true,
        });
        let registry = Arc::new(builder.freeze_for_session().unwrap());
        let mod_particle_id = registry.particle_by_resource_id("demo:spark").unwrap().id;

        let mut runtime = super::super::NativeParticleRuntime::for_tests();
        runtime.fallback_custom_particle_sprites = Box::from([0u32]);
        runtime.sync_content_registry(Arc::clone(&registry), None);

        let payload = ParticlePayload {
            particle_type_id: mod_particle_id,
            raw_options: Vec::new(),
        };
        let particle =
            super::super::ClientParticle::from_payload_with_registry(&payload, &registry)
                .expect("custom particle decodes with ContentRegistry");
        assert!(particle.kind.is_custom());

        let mut engine = ParticleEngine::default();
        runtime.spawn_level_particles(
            &mut engine,
            &particle,
            super::super::LevelParticleEmission::single(
                Vec3d {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                Vec3d::default(),
            ),
            super::super::LevelParticleSpawnContext::default(),
            None,
            None,
        );
        engine.advance(1, &bbb_client_world::WorldStore::new(), Default::default());
        assert_eq!(engine.len(), 1);

        let atlas = ParticleAtlasUvs {
            particles: &[ParticleUvRect {
                min: [0.0; 2],
                max: [1.0; 2],
            }],
            terrain: &[],
            items: &[],
        };
        let mut sources = ParticleQuadSources::default();
        collect_particle_quad_sources(
            &mut sources,
            &engine,
            Some(&runtime),
            atlas,
            |_| true,
            |_, _| false,
        );
        let total_sources: usize = sources.layers.iter().map(Vec::len).sum();
        assert_eq!(
            total_sources, 1,
            "custom mod particle must extract a visible quad via NativeParticleRuntime"
        );
    }
}
