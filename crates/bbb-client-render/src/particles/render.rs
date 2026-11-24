use super::*;

pub struct ParticleExtractionTarget<'a> {
    pub atlases: ParticleAtlasUvs<'a>,
    pub sources: &'a mut ParticleQuadSources,
    pub light: &'a mut ParticleLightSamples,
    pub frame: &'a mut ParticleFrame,
    pub aspect: f32,
    pub item_atlas_loaded: bool,
}

impl Renderer {
    pub fn upload_particle_atlas(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        sprite_uvs: Vec<ParticleSpriteUv>,
    ) -> Result<()> {
        let atlas = create_particle_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffers.world(),
            width,
            height,
            rgba,
            sprite_uvs,
        )?;
        if let Some(previous) = self.pack.particle_atlas.replace(atlas) {
            crate::frame_buffers::retire_gpu_textures(&self.queue, [previous.texture]);
        }
        self.pack.particle_sources.publication = None;
        Ok(())
    }

    pub fn update_particle_atlas(&mut self, rgba: &[u8]) -> Result<()> {
        let Some(atlas) = self.pack.particle_atlas.as_ref() else {
            return Ok(());
        };
        update_particle_atlas_gpu(&self.queue, atlas, rgba)
    }

    /// Terrain-atlas sprites in terrain texture index order.
    pub fn set_terrain_particle_sprite_uvs(&mut self, sprite_uvs: Vec<ParticleSpriteUv>) {
        self.pack.terrain_particle_sprite_uvs = sprite_uvs;
        self.pack.particle_sources.publication = None;
    }

    /// Item-atlas sprites in item texture index order.
    pub fn set_item_particle_sprite_uvs(&mut self, sprite_uvs: Vec<ParticleSpriteUv>) {
        self.pack.item_particle_sprite_uvs = sprite_uvs;
        self.pack.particle_sources.publication = None;
    }

    pub fn begin_particle_extraction(
        &mut self,
        active_instances: usize,
        pending_spawns: usize,
        has_camera: bool,
    ) -> Option<ParticleExtractionTarget<'_>> {
        self.counters.active_particle_instances = active_instances;
        self.counters.pending_particle_spawns = pending_spawns;
        if !has_camera {
            self.pack.particle_sources.publication = None;
            self.particle_frame = ParticleFrame::default();
            return None;
        }
        let item_atlas_loaded = self.pack.item_entity_atlas.is_some();
        let atlases = ParticleAtlasUvs {
            particles: self
                .pack
                .particle_atlas
                .as_ref()
                .map_or(&[], |atlas| atlas.sprite_uvs.as_slice()),
            terrain: &self.pack.terrain_particle_sprite_uvs,
            items: if item_atlas_loaded {
                &self.pack.item_particle_sprite_uvs
            } else {
                &[]
            },
        };
        Some(ParticleExtractionTarget {
            atlases,
            sources: &mut self.pack.particle_sources,
            light: &mut self.pack.particle_light,
            frame: &mut self.particle_frame,
            aspect: self.config.width as f32 / self.config.height as f32,
            item_atlas_loaded,
        })
    }

    pub fn particle_frame(&self) -> &ParticleFrame {
        &self.particle_frame
    }
}

impl ParticleLightSamples {
    pub fn for_revision<'a>(
        &'a mut self,
        revision: u64,
        mut sample: impl FnMut([f64; 3]) -> [f32; 2] + 'a,
    ) -> impl FnMut(usize, [f64; 3]) -> [f32; 2] + 'a {
        if self.world_revision != revision {
            self.samples.clear();
            self.world_revision = revision;
        }
        move |index, position| {
            if self.samples.len() <= index {
                self.samples.resize(index + 1, None);
            }
            *self.samples[index].get_or_insert_with(|| sample(position))
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct ParticleSourceKey {
    pub world_revision: u64,
    pub terrain_sections_revision: u64,
    pub pose: crate::CameraPose,
    pub eye: [f64; 3],
    pub aspect: f32,
    pub culling_fov: f32,
    pub depth_far: f32,
    pub item_atlas_loaded: bool,
}

#[derive(Default)]
pub struct ParticleQuadSources {
    pub publication: Option<ParticleSourceKey>,
    pub layers: [Vec<(usize, ParticleUvRect)>; 6],
}

#[derive(Clone, Copy)]
pub struct ParticleAtlasUvs<'a> {
    pub particles: &'a [ParticleUvRect],
    pub terrain: &'a [ParticleSpriteUv],
    pub items: &'a [ParticleSpriteUv],
}

impl ParticleAtlasUvs<'_> {
    /// The sprite rect and whether that sprite has translucent texels.
    pub fn sprite(
        self,
        texture_atlas: ParticleTextureAtlasKind,
        sprite: Option<u32>,
    ) -> Option<(ParticleUvRect, bool)> {
        let index = usize::try_from(sprite?).ok()?;
        match texture_atlas {
            ParticleTextureAtlasKind::Particles => self.particles.get(index).map(|uv| (*uv, false)),
            ParticleTextureAtlasKind::Terrain => self
                .terrain
                .get(index)
                .map(|sprite| (sprite.uv, sprite.has_translucent)),
            ParticleTextureAtlasKind::Items => self
                .items
                .get(index)
                .map(|sprite| (sprite.uv, sprite.has_translucent)),
        }
    }
}

/// Vanilla `ArrowRenderer.submit` / `ThrownTridentRenderer.submit`: translate to
/// the extracted (quadratic-interpolated) pickup position, then
/// `Axis.YP.rotationDegrees(yRot - 90)` and `Axis.ZP.rotationDegrees(xRot)`
/// (`xRot + 90` for the trident's upright pole). `ArrowModel.createBodyLayer`
/// bakes `mesh.transformed(pose -> pose.scaled(0.9))`, carried as the trailing
/// scale exactly like `arrow_model_root_transform`.
pub fn projectile_pickup_particle_model_transform(
    kind: ParticleItemPickupProjectileKind,
    y_rot: f32,
    x_rot: f32,
    position: [f32; 3],
) -> Mat4 {
    let base = Mat4::from_translation(Vec3::from_array(position))
        * Mat4::from_rotation_y((y_rot - 90.0).to_radians());
    match kind {
        ParticleItemPickupProjectileKind::Trident { .. } => {
            base * Mat4::from_rotation_z((x_rot + 90.0).to_radians())
        }
        ParticleItemPickupProjectileKind::Arrow
        | ParticleItemPickupProjectileKind::TippedArrow
        | ParticleItemPickupProjectileKind::SpectralArrow => {
            base * Mat4::from_rotation_z(x_rot.to_radians()) * Mat4::from_scale(Vec3::splat(0.9))
        }
    }
}

pub fn elder_guardian_particle_model_transform(pose: crate::CameraPose, age_scale: f32) -> Mat4 {
    camera_to_world_transform(pose)
        * Mat4::from_rotation_x((60.0 - 150.0 * age_scale).to_radians())
        * Mat4::from_scale(Vec3::new(
            ELDER_GUARDIAN_PARTICLE_MODEL_SCALE,
            -ELDER_GUARDIAN_PARTICLE_MODEL_SCALE,
            -ELDER_GUARDIAN_PARTICLE_MODEL_SCALE,
        ))
        * Mat4::from_translation(Vec3::new(0.0, -0.56, 3.5))
}

fn camera_to_world_transform(pose: crate::CameraPose) -> Mat4 {
    let eye = Vec3::from_array(pose.position) + Vec3::Y * pose.eye_height;
    Mat4::from_rotation_translation(pose.rotation(), eye)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_to_world_transform_is_right_handed_inverse_of_view_matrix_including_zenith() {
        for (y_rot, x_rot) in [
            (0.0, 0.0),
            (90.0, 0.0),
            (-135.0, 35.0),
            (45.0, 90.0),
            (45.0, -90.0),
        ] {
            let pose = crate::CameraPose {
                position: [12.5, 64.0, -8.25],
                y_rot,
                x_rot,
                eye_height: crate::CameraPose::STANDING_EYE_HEIGHT,
            };
            let cam_to_world = camera_to_world_transform(pose);
            assert!(
                (cam_to_world.determinant() - 1.0).abs() < 1e-4,
                "camera_to_world_transform must be right-handed (det=+1) at yaw={y_rot}, pitch={x_rot}: det={}",
                cam_to_world.determinant()
            );
            let product = cam_to_world * pose.view_matrix();
            for col in 0..4 {
                for row in 0..4 {
                    let expected = if col == row { 1.0 } else { 0.0 };
                    let actual = product.col(col)[row];
                    assert!(
                        (actual - expected).abs() < 1e-4,
                        "expected identity at ({row},{col}) for yaw={y_rot}, pitch={x_rot}, got {actual}"
                    );
                }
            }
            let guardian = elder_guardian_particle_model_transform(pose, 0.5);
            assert!(
                guardian.to_cols_array().iter().all(|v| v.is_finite()),
                "elder guardian transform must remain finite at pitch={x_rot}"
            );
        }
    }

    #[test]
    fn instanced_particle_billboard_faces_rotated_camera_in_wgsl() {
        assert_eq!(std::mem::size_of::<ParticleGpuInstance>(), 52);
        let Some(mut renderer) = Renderer::new_offscreen(64, 64) else {
            return;
        };
        let colors = [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 0, 255],
        ];
        renderer
            .upload_particle_atlas(2, 2, &colors.concat(), vec![])
            .unwrap();
        let pose = crate::CameraPose {
            position: [10.0, 4.0, 10.0],
            y_rot: 90.0,
            x_rot: 0.0,
            eye_height: 0.0,
        };
        let axes = pose.billboard_axes();
        let center = Vec3::from_array(pose.position) + axes.forward * 2.0;
        renderer.set_camera_pose(Some(pose));
        let batch = &mut renderer.particle_frame.quads.opaque;
        batch.quads.set_camera_axes(axes.right, axes.up);
        batch.quads.push_instance(ParticleGpuInstance::billboard(
            center,
            0.8,
            [0.0, 0.0, 1.0, 1.0],
            ParticleFacingCameraMode::LookAtXyz,
            1.0,
            0.0,
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0],
        ));
        batch.draws.push(ParticleAtlasDrawRange {
            texture_atlas: ParticleTextureAtlasKind::Particles,
            instances: 0..1,
        });
        let frame = renderer.render_offscreen_frame().unwrap();
        let tl = frame.pixel(24, 24);
        let tr = frame.pixel(40, 24);
        let bl = frame.pixel(24, 40);
        assert!(
            tl[0] > 180 && tl[1] < 40,
            "expected red top-left, got {tl:?}"
        );
        assert!(
            tr[1] > 180 && tr[0] < 40,
            "expected green top-right, got {tr:?}"
        );
        assert!(
            bl[2] > 180 && bl[0] < 40,
            "expected blue bottom-left, got {tr:?}"
        );
    }
}
