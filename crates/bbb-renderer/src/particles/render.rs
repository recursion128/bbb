use super::*;

impl Renderer {
    pub fn upload_particle_atlas(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        sprite_uvs: Vec<ParticleSpriteUv>,
    ) -> Result<()> {
        self.particle_atlas = Some(create_particle_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            width,
            height,
            rgba,
            sprite_uvs,
        )?);
        Ok(())
    }

    pub fn update_particle_atlas(&mut self, rgba: &[u8]) -> Result<()> {
        let Some(atlas) = self.particle_atlas.as_ref() else {
            return Ok(());
        };
        update_particle_atlas_gpu(&self.queue, atlas, rgba)
    }

    pub fn set_terrain_particle_sprite_uvs(&mut self, sprite_uvs: Vec<ParticleSpriteUv>) {
        let (uvs, translucent_sprites) = particle_sprite_uv_map(sprite_uvs);
        self.terrain_particle_sprite_uvs = uvs;
        self.terrain_particle_translucent_sprites = translucent_sprites;
    }

    pub fn set_item_particle_sprite_uvs(&mut self, sprite_uvs: Vec<ParticleSpriteUv>) {
        let (uvs, translucent_sprites) = particle_sprite_uv_map(sprite_uvs);
        self.item_particle_sprite_uvs = uvs;
        self.item_particle_translucent_sprites = translucent_sprites;
    }

    pub fn submit_particle_spawns(&mut self, batch: ParticleSpawnBatch) {
        let is_empty = batch.is_empty();
        let summary = self.particles.submit_batch(batch);
        if is_empty {
            return;
        }

        self.counters.particle_spawn_batches =
            self.counters.particle_spawn_batches.saturating_add(1);
        self.counters.particle_spawn_commands = self
            .counters
            .particle_spawn_commands
            .saturating_add(summary.requested_spawns as u64);
        self.counters.particle_missing_definitions = self
            .counters
            .particle_missing_definitions
            .saturating_add(summary.missing_definition_count as u64);
        self.counters.particle_missing_sprites = self
            .counters
            .particle_missing_sprites
            .saturating_add(summary.missing_sprite_count as u64);
        self.counters.particle_unknown_types = self
            .counters
            .particle_unknown_types
            .saturating_add(summary.unknown_particle_type_count as u64);
        self.counters.last_particle_spawn_count = summary.queued_spawns;
        self.counters.pending_particle_spawns = summary.pending_spawns;
        self.counters.dropped_particle_spawns = summary.total_dropped_spawns;
    }

    pub fn advance_particles(&mut self, ticks: u32) {
        let summary = self.particles.advance(ticks);
        self.record_particle_advance_summary(summary);
    }

    pub fn advance_particles_with_collision<F>(&mut self, ticks: u32, collide: F)
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
    {
        let summary = self.particles.advance_with_collision(ticks, collide);
        self.record_particle_advance_summary(summary);
    }

    pub fn advance_particles_with_world<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let summary = self
            .particles
            .advance_with_world(ticks, collide, block_fluid_surface);
        self.record_particle_advance_summary(summary);
    }

    pub fn advance_particles_with_world_and_scope_context<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let summary = self.particles.advance_with_world_and_scope_context(
            ticks,
            collide,
            block_fluid_surface,
            scope_context,
        );
        self.record_particle_advance_summary(summary);
    }

    pub fn advance_particles_with_world_and_player_context<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        local_player_motion_context: Option<ParticleLocalPlayerMotionContext>,
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        self.advance_particles_with_world_and_particle_contexts(
            ticks,
            collide,
            block_fluid_surface,
            scope_context,
            local_player_motion_context,
            &[],
        );
    }

    pub fn advance_particles_with_world_and_particle_contexts<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        local_player_motion_context: Option<ParticleLocalPlayerMotionContext>,
        entity_target_contexts: &[ParticleEntityTargetContext],
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let summary = self.particles.advance_with_world_and_particle_contexts(
            ticks,
            collide,
            block_fluid_surface,
            scope_context,
            local_player_motion_context,
            entity_target_contexts,
        );
        self.record_particle_advance_summary(summary);
    }

    pub fn advance_particles_with_world_and_particle_contexts_and_sound_camera<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        local_player_motion_context: Option<ParticleLocalPlayerMotionContext>,
        entity_target_contexts: &[ParticleEntityTargetContext],
        sound_camera_position: Option<[f64; 3]>,
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let summary = self
            .particles
            .advance_with_world_and_particle_contexts_and_sound_camera(
                ticks,
                collide,
                block_fluid_surface,
                scope_context,
                local_player_motion_context,
                entity_target_contexts,
                sound_camera_position,
            );
        self.record_particle_advance_summary(summary);
    }

    pub fn drain_particle_sound_events(&mut self) -> Vec<ParticleSoundEvent> {
        self.particles.drain_sound_events()
    }

    fn record_particle_advance_summary(&mut self, summary: ParticleAdvanceSummary) {
        self.counters.pending_particle_spawns = summary.pending_spawns;
        self.counters.active_particle_instances = summary.active_instances;
        self.counters.last_particle_intake_count = summary.intaken_instances;
        self.counters.last_particle_tick_count = summary.ticks as usize;
        self.counters.last_particle_expired_count = summary.expired_instances;
        self.counters.last_particle_active_drop_count = summary.dropped_active_instances;
        self.counters.last_particle_limited_drop_count = summary.limited_particle_drops;
        self.counters.particle_runtime_ticks = self
            .counters
            .particle_runtime_ticks
            .saturating_add(summary.ticks as u64);
        self.counters.particle_instances_created = summary.total_instances_created;
        self.counters.particle_instances_expired = summary.total_instances_expired;
        self.counters.dropped_active_particle_instances = summary.total_dropped_active_instances;
        self.counters.dropped_limited_particle_instances = summary.total_limited_particle_drops;
    }

    pub fn refresh_particle_lights<F>(&mut self, mut light_at_position: F)
    where
        F: FnMut([f64; 3]) -> [f32; 2],
    {
        self.particles
            .refresh_lights(|position| light_at_position(position));
    }

    pub(crate) fn collect_particle_vertex_batches(&self) -> ParticleVertexBatches {
        let Some(pose) = self.camera_pose else {
            return ParticleVertexBatches::default();
        };
        let particle_sprite_uvs = self.particle_atlas.as_ref().map(|atlas| &atlas.sprite_uvs);
        let item_sprite_uvs = self
            .item_entity_atlas
            .as_ref()
            .map(|_| &self.item_particle_sprite_uvs);
        let atlas_uvs = ParticleAtlasUvSets {
            particles: particle_sprite_uvs,
            terrain: Some(&self.terrain_particle_sprite_uvs),
            items: item_sprite_uvs,
            terrain_translucent_sprites: Some(&self.terrain_particle_translucent_sprites),
            item_translucent_sprites: Some(&self.item_particle_translucent_sprites),
        };
        let axes = camera_billboard_axes(pose);
        ParticleVertexBatches {
            opaque: particle_pipeline_vertex_batch(
                self.particles.active_instances.iter(),
                atlas_uvs,
                axes,
                ParticlePipelineKind::Opaque,
            ),
            translucent: particle_pipeline_vertex_batch(
                self.particles.active_instances.iter(),
                atlas_uvs,
                axes,
                ParticlePipelineKind::Translucent,
            ),
        }
    }

    pub(crate) fn collect_elder_guardian_particle_render_instances(
        &self,
    ) -> Vec<ElderGuardianParticleRenderInstance> {
        let Some(pose) = self.camera_pose else {
            return Vec::new();
        };
        elder_guardian_particle_render_instances(self.particles.active_instances.iter(), pose)
    }

    pub(crate) fn collect_experience_orb_pickup_particle_render_instances(
        &self,
    ) -> Vec<ExperienceOrbPickupParticleRenderInstance> {
        let Some(pose) = self.camera_pose else {
            return Vec::new();
        };
        experience_orb_pickup_particle_render_instances(
            self.particles.active_instances.iter(),
            pose,
        )
    }

    pub fn item_pickup_particle_render_states(&self) -> Vec<ItemPickupParticleRenderState> {
        item_pickup_particle_render_states(self.particles.active_instances.iter())
    }
}

fn particle_sprite_uv_map(
    sprite_uvs: Vec<ParticleSpriteUv>,
) -> (BTreeMap<String, ParticleUvRect>, BTreeSet<String>) {
    let mut uvs = BTreeMap::new();
    let mut translucent_sprites = BTreeSet::new();
    for sprite in sprite_uvs {
        if sprite.has_translucent {
            translucent_sprites.insert(sprite.id.clone());
        }
        uvs.insert(sprite.id, sprite.uv);
    }
    (uvs, translucent_sprites)
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ParticleAtlasUvSets<'a> {
    pub(super) particles: Option<&'a BTreeMap<String, ParticleUvRect>>,
    pub(super) terrain: Option<&'a BTreeMap<String, ParticleUvRect>>,
    pub(super) items: Option<&'a BTreeMap<String, ParticleUvRect>>,
    pub(super) terrain_translucent_sprites: Option<&'a BTreeSet<String>>,
    pub(super) item_translucent_sprites: Option<&'a BTreeSet<String>>,
}

impl<'a> ParticleAtlasUvSets<'a> {
    fn for_texture_atlas(
        self,
        texture_atlas: ParticleTextureAtlasKind,
    ) -> Option<&'a BTreeMap<String, ParticleUvRect>> {
        match texture_atlas {
            ParticleTextureAtlasKind::Particles => self.particles,
            ParticleTextureAtlasKind::Terrain => self.terrain,
            ParticleTextureAtlasKind::Items => self.items,
        }
    }

    fn has_translucent_sprite(
        self,
        texture_atlas: ParticleTextureAtlasKind,
        sprite_id: &str,
    ) -> bool {
        match texture_atlas {
            ParticleTextureAtlasKind::Particles => false,
            ParticleTextureAtlasKind::Terrain => self
                .terrain_translucent_sprites
                .is_some_and(|sprites| sprites.contains(sprite_id)),
            ParticleTextureAtlasKind::Items => self
                .item_translucent_sprites
                .is_some_and(|sprites| sprites.contains(sprite_id)),
        }
    }
}

pub(super) fn particle_pipeline_vertex_batch<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
    atlas_uvs: ParticleAtlasUvSets<'_>,
    axes: ParticleBillboardAxes,
    pipeline_kind: ParticlePipelineKind,
) -> ParticlePipelineVertexBatch {
    let mut batch = ParticlePipelineVertexBatch::default();
    let mut current_draw_start = 0_u32;
    let mut current_texture_atlas = None;
    let mut instances: Vec<_> = instances
        .into_iter()
        .filter(|instance| instance.render_group == ParticleRenderGroup::SingleQuads)
        .filter(|instance| instance.delay_ticks == 0)
        .filter_map(|instance| {
            let sprite_id = instance.current_sprite_id.as_deref()?;
            let render_layer = particle_render_layer_for_sprite(instance, atlas_uvs, sprite_id);
            (render_layer.pipeline_kind() == pipeline_kind).then_some((instance, render_layer))
        })
        .collect();
    instances.sort_by_key(|(_, render_layer)| {
        (
            ParticleRenderGroup::SingleQuads.vanilla_order(),
            render_layer.vanilla_solid_translucent_order(),
        )
    });
    for (instance, _) in instances {
        let Some(sprite_id) = instance.current_sprite_id.as_deref() else {
            continue;
        };
        let Some(sprite_uvs) = atlas_uvs.for_texture_atlas(instance.texture_atlas) else {
            continue;
        };
        let Some(uv) = sprite_uvs.get(sprite_id).copied() else {
            continue;
        };
        if current_texture_atlas != Some(instance.texture_atlas) {
            push_particle_atlas_draw_range(&mut batch, current_texture_atlas, current_draw_start);
            current_texture_atlas = Some(instance.texture_atlas);
            current_draw_start = batch.vertices.len() as u32;
        }
        let uv = particle_uv_rect_for_instance(instance, uv);
        append_particle_instance_vertices(&mut batch.vertices, instance, uv, axes);
    }
    push_particle_atlas_draw_range(&mut batch, current_texture_atlas, current_draw_start);
    batch
}

fn push_particle_atlas_draw_range(
    batch: &mut ParticlePipelineVertexBatch,
    texture_atlas: Option<ParticleTextureAtlasKind>,
    vertex_start: u32,
) {
    let Some(texture_atlas) = texture_atlas else {
        return;
    };
    let vertex_end = batch.vertices.len() as u32;
    if vertex_end <= vertex_start {
        return;
    }
    batch.draws.push(ParticleAtlasDrawRange {
        texture_atlas,
        vertex_start,
        vertex_count: vertex_end - vertex_start,
    });
}

#[cfg(test)]
pub(super) fn particle_billboard_vertices<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
    sprite_uvs: &BTreeMap<String, ParticleUvRect>,
    axes: ParticleBillboardAxes,
    pipeline_kind: Option<ParticlePipelineKind>,
) -> Vec<ParticleVertex> {
    let mut vertices = Vec::new();
    let mut instances: Vec<_> = instances
        .into_iter()
        .filter(|instance| instance.render_group == ParticleRenderGroup::SingleQuads)
        .filter(|instance| instance.delay_ticks == 0)
        .filter(|instance| match pipeline_kind {
            Some(kind) => instance.render_layer.pipeline_kind() == kind,
            None => true,
        })
        .collect();
    instances.sort_by_key(|instance| {
        (
            instance.render_group.vanilla_order(),
            instance.render_layer.vanilla_solid_translucent_order(),
        )
    });
    for instance in instances {
        let Some(sprite_id) = instance.current_sprite_id.as_deref() else {
            continue;
        };
        let Some(uv) = sprite_uvs.get(sprite_id).copied() else {
            continue;
        };
        let uv = particle_uv_rect_for_instance(instance, uv);
        append_particle_instance_vertices(&mut vertices, instance, uv, axes);
    }
    vertices
}

fn particle_render_layer_for_sprite(
    instance: &ParticleInstance,
    atlas_uvs: ParticleAtlasUvSets<'_>,
    sprite_id: &str,
) -> ParticleRenderLayer {
    let has_translucent = atlas_uvs.has_translucent_sprite(instance.texture_atlas, sprite_id);
    match instance.render_layer {
        ParticleRenderLayer::OpaqueTerrain | ParticleRenderLayer::TranslucentTerrain => {
            if has_translucent {
                ParticleRenderLayer::TranslucentTerrain
            } else {
                ParticleRenderLayer::OpaqueTerrain
            }
        }
        ParticleRenderLayer::OpaqueItems | ParticleRenderLayer::TranslucentItems => {
            if has_translucent {
                ParticleRenderLayer::TranslucentItems
            } else {
                ParticleRenderLayer::OpaqueItems
            }
        }
        render_layer => render_layer,
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ParticleBillboardAxes {
    pub(super) right: Vec3,
    pub(super) up: Vec3,
}

fn camera_billboard_axes(pose: crate::CameraPose) -> ParticleBillboardAxes {
    let yaw = pose.y_rot.to_radians();
    let pitch = pose.x_rot.to_radians();
    let cos_pitch = pitch.cos();
    let forward =
        Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch).normalize_or_zero();
    let forward = if forward.length_squared() > 0.0 {
        forward
    } else {
        Vec3::Z
    };
    let right = Vec3::Y.cross(forward).normalize_or_zero();
    let right = if right.length_squared() > 0.0 {
        right
    } else {
        Vec3::X
    };
    let up = forward.cross(right).normalize_or_zero();
    ParticleBillboardAxes {
        right,
        up: if up.length_squared() > 0.0 {
            up
        } else {
            Vec3::Y
        },
    }
}

pub(super) fn elder_guardian_particle_render_instances<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
    pose: crate::CameraPose,
) -> Vec<ElderGuardianParticleRenderInstance> {
    instances
        .into_iter()
        .filter(|instance| instance.render_group == ParticleRenderGroup::ElderGuardians)
        .filter(|instance| instance.delay_ticks == 0)
        .map(|instance| {
            let age_scale = elder_guardian_particle_age_scale(
                instance.age_ticks,
                instance.lifetime_ticks,
                DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
            );
            ElderGuardianParticleRenderInstance {
                transform: elder_guardian_particle_model_transform(pose, age_scale),
                tint: [1.0, 1.0, 1.0, elder_guardian_particle_alpha(age_scale)],
            }
        })
        .collect()
}

pub(super) fn item_pickup_particle_render_states<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
) -> Vec<ItemPickupParticleRenderState> {
    instances
        .into_iter()
        .filter(|instance| instance.render_group == ParticleRenderGroup::ItemPickup)
        .filter(|instance| instance.delay_ticks == 0)
        .filter_map(|instance| {
            let item = instance.option_item?;
            let source_entity_id = instance.option_item_pickup_source_entity_id?;
            let position = instance
                .item_pickup_position_at_partial_tick(DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)?;
            Some(ItemPickupParticleRenderState {
                source_entity_id,
                item,
                position: [position[0] as f32, position[1] as f32, position[2] as f32],
                age_ticks: instance
                    .option_item_pickup_age_ticks
                    .unwrap_or(instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK),
                light: instance
                    .option_item_pickup_light
                    .unwrap_or(DEFAULT_PARTICLE_LIGHT),
            })
        })
        .collect()
}

pub(super) fn experience_orb_pickup_particle_render_states<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
) -> Vec<ExperienceOrbPickupParticleRenderState> {
    instances
        .into_iter()
        .filter(|instance| instance.render_group == ParticleRenderGroup::ItemPickup)
        .filter(|instance| instance.delay_ticks == 0)
        .filter_map(|instance| {
            let icon = instance.option_item_pickup_experience_orb_icon?;
            let source_entity_id = instance.option_item_pickup_source_entity_id?;
            let position = instance
                .item_pickup_position_at_partial_tick(DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)?;
            Some(ExperienceOrbPickupParticleRenderState {
                source_entity_id,
                icon,
                position: [position[0] as f32, position[1] as f32, position[2] as f32],
                age_ticks: instance
                    .option_item_pickup_age_ticks
                    .unwrap_or(instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK),
                light: instance
                    .option_item_pickup_light
                    .unwrap_or(DEFAULT_PARTICLE_LIGHT),
            })
        })
        .collect()
}

fn experience_orb_pickup_particle_render_instances<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
    pose: crate::CameraPose,
) -> Vec<ExperienceOrbPickupParticleRenderInstance> {
    experience_orb_pickup_particle_render_states(instances)
        .into_iter()
        .map(|state| ExperienceOrbPickupParticleRenderInstance {
            transform: experience_orb_pickup_particle_model_transform(pose, state.position),
            icon: state.icon,
            age_ticks: state.age_ticks,
            light: state.light,
        })
        .collect()
}

fn experience_orb_pickup_particle_model_transform(
    pose: crate::CameraPose,
    position: [f32; 3],
) -> Mat4 {
    let axes = camera_billboard_axes(pose);
    let forward = axes.right.cross(axes.up).normalize_or_zero();
    let forward = if forward.length_squared() > 0.0 {
        forward
    } else {
        Vec3::Z
    };
    let orientation = Mat4::from_cols(
        axes.right.extend(0.0),
        axes.up.extend(0.0),
        (-forward).extend(0.0),
        Vec3::ZERO.extend(1.0),
    );
    Mat4::from_translation(Vec3::from_array(position) + Vec3::Y * 0.1)
        * orientation
        * Mat4::from_scale(Vec3::splat(0.3))
}

pub(super) fn elder_guardian_particle_model_transform(
    pose: crate::CameraPose,
    age_scale: f32,
) -> Mat4 {
    camera_to_world_transform(pose)
        * Mat4::from_rotation_x((60.0 - 150.0 * age_scale).to_radians())
        * Mat4::from_scale(Vec3::new(
            ELDER_GUARDIAN_PARTICLE_MODEL_SCALE,
            -ELDER_GUARDIAN_PARTICLE_MODEL_SCALE,
            -ELDER_GUARDIAN_PARTICLE_MODEL_SCALE,
        ))
        * Mat4::from_translation(Vec3::new(0.0, -0.56, 3.5))
        * Mat4::from_scale(Vec3::splat(ELDER_GUARDIAN_PARTICLE_BAKED_LAYER_SCALE))
}

fn elder_guardian_particle_age_scale(
    age_ticks: u32,
    lifetime_ticks: u32,
    partial_tick: f32,
) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    (age_ticks as f32 + partial_tick.clamp(0.0, 1.0)) / lifetime
}

fn elder_guardian_particle_alpha(age_scale: f32) -> f32 {
    0.05 + 0.5 * (age_scale * std::f32::consts::PI).sin()
}

fn camera_to_world_transform(pose: crate::CameraPose) -> Mat4 {
    let eye = Vec3::from_array(pose.position) + Vec3::Y * pose.eye_height;
    let yaw = pose.y_rot.to_radians();
    let pitch = pose.x_rot.to_radians();
    let cos_pitch = pitch.cos();
    let forward =
        Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch).normalize_or_zero();
    let forward = if forward.length_squared() > 0.0 {
        forward
    } else {
        Vec3::Z
    };
    let right = Vec3::Y.cross(forward).normalize_or_zero();
    let right = if right.length_squared() > 0.0 {
        right
    } else {
        Vec3::X
    };
    let up = forward.cross(right).normalize_or_zero();
    let up = if up.length_squared() > 0.0 {
        up
    } else {
        Vec3::Y
    };
    Mat4::from_cols(
        right.extend(0.0),
        up.extend(0.0),
        (-forward).extend(0.0),
        eye.extend(1.0),
    )
}

fn particle_axes_for_instance(
    axes: ParticleBillboardAxes,
    facing_camera_mode: ParticleFacingCameraMode,
) -> ParticleBillboardAxes {
    match facing_camera_mode {
        ParticleFacingCameraMode::LookAtXyz => axes,
        ParticleFacingCameraMode::LookAtY => ParticleBillboardAxes {
            right: axes.right,
            up: Vec3::Y,
        },
    }
}

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

fn append_particle_instance_vertices(
    vertices: &mut Vec<ParticleVertex>,
    instance: &ParticleInstance,
    uv: ParticleUvRect,
    axes: ParticleBillboardAxes,
) {
    if instance.provider == "ShriekParticle.Provider" {
        for rotation in shriek_particle_rotations() {
            append_rotated_particle_quad(vertices, instance, uv, rotation);
        }
        return;
    }

    if instance.provider == "VibrationSignalParticle.Provider" {
        for rotation in vibration_particle_rotations(instance) {
            append_rotated_particle_quad(vertices, instance, uv, rotation);
        }
        return;
    }

    vertices.extend(particle_instance_vertices(
        instance,
        uv,
        particle_axes_for_instance(axes, instance.facing_camera_mode),
    ));
}

fn particle_instance_vertices(
    instance: &ParticleInstance,
    uv: ParticleUvRect,
    axes: ParticleBillboardAxes,
) -> [ParticleVertex; 6] {
    let center = Vec3::new(
        instance.position[0] as f32,
        instance.position[1] as f32,
        instance.position[2] as f32,
    );
    let half_size = instance.render_quad_size() * 0.5;
    let roll = lerp_f32(
        DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
        instance.previous_roll,
        instance.roll,
    );
    let (right_axis, up_axis) = rotated_billboard_axes(axes, roll);
    let right = right_axis * half_size;
    let up = up_axis * half_size;
    let bottom_left = center - right - up;
    let bottom_right = center + right - up;
    let top_right = center + right + up;
    let top_left = center - right + up;
    let tint = particle_render_color(instance);

    [
        particle_vertex(bottom_left, [uv.min[0], uv.max[1]], tint, instance.light),
        particle_vertex(bottom_right, [uv.max[0], uv.max[1]], tint, instance.light),
        particle_vertex(top_right, [uv.max[0], uv.min[1]], tint, instance.light),
        particle_vertex(bottom_left, [uv.min[0], uv.max[1]], tint, instance.light),
        particle_vertex(top_right, [uv.max[0], uv.min[1]], tint, instance.light),
        particle_vertex(top_left, [uv.min[0], uv.min[1]], tint, instance.light),
    ]
}

fn append_rotated_particle_quad(
    vertices: &mut Vec<ParticleVertex>,
    instance: &ParticleInstance,
    uv: ParticleUvRect,
    rotation: Quat,
) {
    let center = Vec3::new(
        instance.position[0] as f32,
        instance.position[1] as f32,
        instance.position[2] as f32,
    );
    let half_size = instance.render_quad_size() * 0.5;
    let bottom_left = center + rotation * Vec3::new(-half_size, -half_size, 0.0);
    let bottom_right = center + rotation * Vec3::new(half_size, -half_size, 0.0);
    let top_right = center + rotation * Vec3::new(half_size, half_size, 0.0);
    let top_left = center + rotation * Vec3::new(-half_size, half_size, 0.0);
    let tint = particle_render_color(instance);

    vertices.extend([
        particle_vertex(bottom_left, [uv.min[0], uv.max[1]], tint, instance.light),
        particle_vertex(bottom_right, [uv.max[0], uv.max[1]], tint, instance.light),
        particle_vertex(top_right, [uv.max[0], uv.min[1]], tint, instance.light),
        particle_vertex(bottom_left, [uv.min[0], uv.max[1]], tint, instance.light),
        particle_vertex(top_right, [uv.max[0], uv.min[1]], tint, instance.light),
        particle_vertex(top_left, [uv.min[0], uv.min[1]], tint, instance.light),
    ]);
}

fn shriek_particle_rotations() -> [Quat; 2] {
    [
        Quat::from_rotation_x(-SHRIEK_MAGICAL_X_ROT),
        Quat::from_euler(
            EulerRot::YXZ,
            -std::f32::consts::PI,
            SHRIEK_MAGICAL_X_ROT,
            0.0,
        ),
    ]
}

fn vibration_particle_rotations(instance: &ParticleInstance) -> [Quat; 2] {
    let random_sway =
        vibration_particle_sway(instance.age_ticks, DEFAULT_PARTICLE_RENDER_PARTIAL_TICK);
    let yaw = lerp_f32(
        DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
        instance.previous_yaw,
        instance.yaw,
    );
    let pitch = lerp_f32(
        DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
        instance.previous_pitch,
        instance.pitch,
    ) + std::f32::consts::FRAC_PI_2;
    [
        Quat::from_rotation_y(yaw)
            * Quat::from_rotation_x(-pitch)
            * Quat::from_rotation_y(random_sway),
        Quat::from_rotation_y(-std::f32::consts::PI + yaw)
            * Quat::from_rotation_x(pitch)
            * Quat::from_rotation_y(random_sway),
    ]
}

pub(super) fn particle_render_color(instance: &ParticleInstance) -> [f32; 4] {
    let mut color = instance.color;
    if let Some(target) = instance.color_transition_target {
        let alpha = (instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)
            / (instance.lifetime_ticks as f32 + 1.0).max(1.0);
        color[0] = lerp_f32(alpha, color[0], target[0]);
        color[1] = lerp_f32(alpha, color[1], target[1]);
        color[2] = lerp_f32(alpha, color[2], target[2]);
    }
    match instance.alpha_curve {
        ParticleAlphaCurve::Constant => {}
        ParticleAlphaCurve::SimpleAnimatedFade => {}
        ParticleAlphaCurve::FlashOverlayFade => {
            color[3] =
                flash_overlay_alpha(instance.age_ticks, DEFAULT_PARTICLE_RENDER_PARTIAL_TICK);
        }
        ParticleAlphaCurve::FireworkSparkFade => {}
        ParticleAlphaCurve::ShriekFade => {
            let lifetime = instance.lifetime_ticks.max(1) as f32;
            color[3] = 1.0
                - ((instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK) / lifetime)
                    .clamp(0.0, 1.0);
        }
        ParticleAlphaCurve::VaultConnectionFade => {
            color[3] = vault_connection_alpha(
                instance.age_ticks,
                instance.lifetime_ticks,
                DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
            );
        }
        ParticleAlphaCurve::FireflyFade => {
            let progress = (instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)
                / instance.lifetime_ticks.max(1) as f32;
            color[3] = firefly_fade_amount(progress, 0.3, 0.5);
        }
    }
    if firework_twinkle_hidden(instance) {
        color[3] = 0.0;
    }
    color
}

fn firework_twinkle_hidden(instance: &ParticleInstance) -> bool {
    instance.firework_twinkle
        && instance.age_ticks >= instance.lifetime_ticks / 3
        && ((instance.age_ticks + instance.lifetime_ticks) / 3) % 2 != 0
}

pub(super) fn simple_animated_alpha(age_ticks: u32, lifetime_ticks: u32) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    let half_lifetime = lifetime_ticks / 2;
    if age_ticks <= half_lifetime {
        1.0
    } else {
        1.0 - (age_ticks.saturating_sub(half_lifetime) as f32 / lifetime).clamp(0.0, 1.0)
    }
}

pub(super) fn firework_spark_alpha(age_ticks: u32, lifetime_ticks: u32) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    let half_lifetime = lifetime_ticks / 2;
    if age_ticks <= half_lifetime {
        0.99
    } else {
        1.0 - (age_ticks.saturating_sub(half_lifetime) as f32 / lifetime).clamp(0.0, 1.0)
    }
}

pub(super) fn apply_particle_power(velocity: [f64; 3], power: f32) -> [f64; 3] {
    let power = f64::from(power);
    [
        velocity[0] * power,
        (velocity[1] - 0.1) * power + 0.1,
        velocity[2] * power,
    ]
}

pub(super) fn trail_particle_color(color: [f32; 4], random: &mut ParticleRandom) -> [f32; 4] {
    [
        color[0] * (0.875 + random.next_f32() * 0.25),
        color[1] * (0.875 + random.next_f32() * 0.25),
        color[2] * (0.875 + random.next_f32() * 0.25),
        color[3],
    ]
}

pub(super) fn random_sign(random: &mut ParticleRandom) -> f64 {
    if random.next_bool() {
        1.0
    } else {
        -1.0
    }
}

pub(super) fn dust_particle_color(
    color: [f32; 4],
    base_factor: f32,
    random: &mut ParticleRandom,
) -> [f32; 4] {
    [
        (random.next_f32() * 0.2 + 0.8) * color[0] * base_factor,
        (random.next_f32() * 0.2 + 0.8) * color[1] * base_factor,
        (random.next_f32() * 0.2 + 0.8) * color[2] * base_factor,
        color[3],
    ]
}

pub(super) fn clamp_particle_option_scale(scale: f32) -> f32 {
    scale.clamp(0.01, 4.0)
}

pub(super) fn flash_overlay_alpha(age_ticks: u32, partial_tick: f32) -> f32 {
    0.6 - (age_ticks as f32 + partial_tick.clamp(0.0, 1.0) - 1.0) * 0.25 * 0.5
}

pub(super) fn vibration_particle_angles(position: [f64; 3], target: [f64; 3]) -> (f32, f32) {
    let dx = position[0] - target[0];
    let dy = position[1] - target[1];
    let dz = position[2] - target[2];
    let yaw = dx.atan2(dz) as f32;
    let pitch = dy.atan2((dx * dx + dz * dz).sqrt()) as f32;
    (yaw, pitch)
}

pub(super) fn vibration_particle_sway(age_ticks: u32, partial_tick: f32) -> f32 {
    ((age_ticks as f32 + partial_tick.clamp(0.0, 1.0) - std::f32::consts::TAU) * 0.05).sin() * 2.0
}

pub(super) fn lerp_f64(alpha: f64, start: f64, end: f64) -> f64 {
    start + alpha * (end - start)
}

pub(super) fn lerp_f32(alpha: f32, start: f32, end: f32) -> f32 {
    start + alpha * (end - start)
}

pub(super) fn motion_length_squared(movement: [f64; 3]) -> f64 {
    movement[0] * movement[0] + movement[1] * movement[1] + movement[2] * movement[2]
}

pub(super) fn argb_srgb_lerp_color(alpha: f32, start: u32, end: u32) -> [f32; 4] {
    let lerp_channel = |shift: u32| -> f32 {
        let from = ((start >> shift) & 0xFF) as i32;
        let to = ((end >> shift) & 0xFF) as i32;
        (from + (alpha * (to - from) as f32).floor() as i32) as f32 / 255.0
    };
    [
        lerp_channel(16),
        lerp_channel(8),
        lerp_channel(0),
        lerp_channel(24),
    ]
}

fn rotated_billboard_axes(axes: ParticleBillboardAxes, roll: f32) -> (Vec3, Vec3) {
    if roll == 0.0 {
        return (axes.right, axes.up);
    }
    let (sin, cos) = roll.sin_cos();
    (
        axes.right * cos + axes.up * sin,
        -axes.right * sin + axes.up * cos,
    )
}

pub(super) fn vault_connection_alpha(
    age_ticks: u32,
    lifetime_ticks: u32,
    partial_tick: f32,
) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    let normalized = (age_ticks as f32 + partial_tick.clamp(0.0, 1.0)) / lifetime;
    let time = ((normalized - 0.25) / 0.75).clamp(0.0, 1.0);
    time * 0.6
}

pub(super) fn particle_render_group_for_particle(particle_id: &str) -> ParticleRenderGroup {
    match particle_id {
        ITEM_PICKUP_PARTICLE_ID => ParticleRenderGroup::ItemPickup,
        ELDER_GUARDIAN_PARTICLE_ID => ParticleRenderGroup::ElderGuardians,
        _ => ParticleRenderGroup::SingleQuads,
    }
}

pub(super) fn particle_render_layer_for_particle(particle_id: &str) -> ParticleRenderLayer {
    match particle_id {
        "minecraft:block"
        | "minecraft:block_marker"
        | "minecraft:dust_pillar"
        | "minecraft:block_crumble" => ParticleRenderLayer::OpaqueTerrain,
        "minecraft:item"
        | "minecraft:item_slime"
        | "minecraft:item_cobweb"
        | "minecraft:item_snowball" => ParticleRenderLayer::OpaqueItems,
        "minecraft:cloud"
        | "minecraft:campfire_cosy_smoke"
        | "minecraft:campfire_signal_smoke"
        | "minecraft:sneeze"
        | "minecraft:totem_of_undying"
        | "minecraft:squid_ink"
        | "minecraft:glow_squid_ink"
        | "minecraft:end_rod"
        | "minecraft:soul"
        | "minecraft:sculk_soul"
        | "minecraft:sculk_charge"
        | "minecraft:sculk_charge_pop"
        | "minecraft:shriek"
        | "minecraft:vibration"
        | "minecraft:vault_connection"
        | "minecraft:effect"
        | "minecraft:instant_effect"
        | "minecraft:entity_effect"
        | "minecraft:flash"
        | "minecraft:firework"
        | "minecraft:firefly"
        | ELDER_GUARDIAN_PARTICLE_ID
        | "minecraft:infested"
        | "minecraft:raid_omen"
        | "minecraft:trial_omen"
        | "minecraft:witch" => ParticleRenderLayer::Translucent,
        _ => ParticleRenderLayer::Opaque,
    }
}

pub(super) fn fixed_item_particle_sprite_id(particle_id: &str) -> Option<&'static str> {
    match particle_id {
        "minecraft:item_slime" => Some("minecraft:item/slime_ball"),
        "minecraft:item_cobweb" => Some("minecraft:block/cobweb"),
        "minecraft:item_snowball" => Some("minecraft:item/snowball"),
        _ => None,
    }
}

pub(super) fn particle_atlas_uv_sub_rect_for_particle(
    particle_id: &str,
    random: &mut ParticleRandom,
) -> Option<ParticleAtlasUvSubRect> {
    matches!(
        particle_id,
        "minecraft:block"
            | "minecraft:dust_pillar"
            | "minecraft:block_crumble"
            | "minecraft:item"
            | "minecraft:item_slime"
            | "minecraft:item_cobweb"
            | "minecraft:item_snowball"
    )
    .then(|| ParticleAtlasUvSubRect {
        u_offset: random.next_f32() * 3.0,
        v_offset: random.next_f32() * 3.0,
    })
}

fn particle_vertex(
    position: Vec3,
    uv: [f32; 2],
    color: [f32; 4],
    light: [f32; 2],
) -> ParticleVertex {
    ParticleVertex {
        position: position.to_array(),
        uv,
        color,
        light,
    }
}

pub(super) fn sanitize_particle_light(light: [f32; 2]) -> [f32; 2] {
    [
        if light[0].is_finite() {
            light[0].clamp(0.0, 1.0)
        } else {
            DEFAULT_PARTICLE_LIGHT[0]
        },
        if light[1].is_finite() {
            light[1].clamp(0.0, 1.0)
        } else {
            DEFAULT_PARTICLE_LIGHT[1]
        },
    ]
}

pub(super) fn particle_light_with_emission(
    instance: &ParticleInstance,
    sampled_light: [f32; 2],
) -> [f32; 2] {
    match instance.light_emission {
        ParticleLightEmissionDescriptor::World => sampled_light,
        ParticleLightEmissionDescriptor::FullBright => [1.0, 1.0],
        ParticleLightEmissionDescriptor::FullBlock => [1.0, sampled_light[1]],
        ParticleLightEmissionDescriptor::SmoothBlockByAge => {
            let emission = particle_light_emission_progress(
                instance.age_ticks,
                instance.lifetime_ticks,
                DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
            );
            [(sampled_light[0] + emission).min(1.0), sampled_light[1]]
        }
        ParticleLightEmissionDescriptor::SmoothBlockByAgeQuartic => {
            let age = instance.age_ticks as f32 / instance.lifetime_ticks.max(1) as f32;
            let emission = age * age * age * age;
            [(sampled_light[0] + emission).min(1.0), sampled_light[1]]
        }
        ParticleLightEmissionDescriptor::Firefly => {
            let progress = (instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)
                / instance.lifetime_ticks.max(1) as f32;
            [firefly_fade_amount(progress, 0.1, 0.3), 0.0]
        }
    }
}

fn particle_light_emission_progress(age_ticks: u32, lifetime_ticks: u32, partial_tick: f32) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    ((age_ticks as f32 + partial_tick.clamp(0.0, 1.0)) / lifetime).clamp(0.0, 1.0)
}

pub(super) fn firefly_fade_amount(
    lifetime_progress: f32,
    fade_in_time: f32,
    fade_out_time: f32,
) -> f32 {
    let lifetime_progress = lifetime_progress.clamp(0.0, 1.0);
    if lifetime_progress >= 1.0 - fade_in_time {
        (1.0 - lifetime_progress) / fade_in_time
    } else if lifetime_progress <= fade_out_time {
        lifetime_progress / fade_out_time
    } else {
        1.0
    }
}
