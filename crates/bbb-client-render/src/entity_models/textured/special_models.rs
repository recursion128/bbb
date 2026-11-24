use super::*;
use bbb_registry_codec::legacy_random::LegacyRandomSource;

const DRAGON_RAYS_RANDOM_SEED: i64 = 432;

const DRAGON_RAYS_HALF_SQRT_3: f32 = 0.866_025_4;

const END_PORTAL_BOTTOM: f32 = 0.375;

const END_PORTAL_HEIGHT: f32 = 0.375;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElderGuardianParticleRenderInstance {
    pub transform: Mat4,
    pub tint: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExperienceOrbPickupParticleRenderInstance {
    pub transform: Mat4,
    pub icon: i32,
    pub age_ticks: f32,
    pub light: [f32; 2],
}

/// One item-pickup particle whose carried model is an arrow / trident, posed at
/// the world-space interpolated pickup position (vanilla
/// `ItemPickupParticleGroup.State.submit` renders the extracted projectile
/// render state through the entity render dispatcher).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectilePickupParticleRenderInstance {
    pub transform: Mat4,
    pub kind: ParticleItemPickupProjectileKind,
    pub light: [f32; 2],
}

pub(in crate::entity_models) fn elder_guardian_particle_textured_meshes(
    instances: &[ElderGuardianParticleRenderInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new(None);
    // Vanilla `ElderGuardianParticle` bakes `ModelLayers.ELDER_GUARDIAN`, whose root carries the
    // 2.35 mesh scale.
    let model = ModelLayer::ELDER_GUARDIAN.bake();
    let base_pass = guardian_textured_layer_passes(true)[0];
    for instance in instances.iter().copied() {
        let pass = EntityModelLayerPass {
            render_type: EntityModelLayerRenderType::EntityTranslucent,
            tint: instance.tint,
            ..base_pass
        };
        let submit =
            no_overlay_layer_submission_with_tint(&meshes, pass, instance.tint, instance.transform)
                .with_light(ENTITY_VERTEX_FULL_BRIGHT_LIGHT);
        render_textured_submission(&mut meshes, submit, atlas, |mesh, entry| {
            model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
        });
    }
    meshes
}

/// Bakes the item-pickup carried arrow / trident models. Vanilla
/// `ItemPickupParticleGroup.State.submit` runs the picked-up projectile's
/// extracted render state through `EntityRenderDispatcher.submit`
/// (`ArrowRenderer` / `ThrownTridentRenderer`); bbb draws the pickup group into
/// the blended particles target, so the passes are forced to
/// `EntityTranslucent` (alpha stays 1.0) like the elder guardian particle
/// model. The trident foil keeps its vanilla second submit
/// (`entityGlint` over the same model), carried here as a statically-sampled
/// glint pass for the 3-tick flash.
pub(in crate::entity_models) fn projectile_pickup_particle_textured_meshes(
    instances: &[ProjectilePickupParticleRenderInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new(None);
    for instance in instances.iter().copied() {
        match instance.kind {
            ParticleItemPickupProjectileKind::Arrow
            | ParticleItemPickupProjectileKind::TippedArrow
            | ParticleItemPickupProjectileKind::SpectralArrow => {
                let texture = match instance.kind {
                    ParticleItemPickupProjectileKind::TippedArrow => ArrowModelTexture::Tipped,
                    ParticleItemPickupProjectileKind::SpectralArrow => ArrowModelTexture::Spectral,
                    _ => ArrowModelTexture::Normal,
                };
                render_projectile_pickup_particle_pass(
                    &mut meshes,
                    &ModelLayer::ARROW.bake(),
                    arrow_textured_layer_passes(texture)[0],
                    instance,
                    atlas,
                );
            }
            ParticleItemPickupProjectileKind::Trident { foil } => {
                let model = ModelLayer::TRIDENT.bake();
                let passes = trident_textured_layer_passes();
                render_projectile_pickup_particle_pass(
                    &mut meshes,
                    &model,
                    passes[0],
                    instance,
                    atlas,
                );
                if foil {
                    render_projectile_pickup_particle_pass(
                        &mut meshes,
                        &model,
                        passes[1],
                        instance,
                        atlas,
                    );
                }
            }
        }
    }
    meshes
}

fn render_projectile_pickup_particle_pass(
    meshes: &mut EntityModelTexturedMeshes,
    model: &ModelPart,
    base_pass: EntityModelLayerPass,
    instance: ProjectilePickupParticleRenderInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let pass = EntityModelLayerPass {
        render_type: EntityModelLayerRenderType::EntityTranslucent,

        ..base_pass
    };
    let submit =
        no_overlay_layer_submission(meshes, pass, instance.transform).with_light(instance.light);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

pub(in crate::entity_models) fn experience_orb_pickup_particle_textured_mesh(
    instances: &[ExperienceOrbPickupParticleRenderInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMesh {
    let mut mesh = EntityModelTexturedMesh::new();
    let Some(entry) = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == EXPERIENCE_ORB_TEXTURE_REF)
    else {
        return mesh;
    };

    for instance in instances {
        append_experience_orb_pickup_particle_quad(&mut mesh, *instance, entry.uv);
    }
    mesh
}

fn append_experience_orb_pickup_particle_quad(
    mesh: &mut EntityModelTexturedMesh,
    instance: ExperienceOrbPickupParticleRenderInstance,
    texture_uv: EntityModelUvRect,
) {
    let tint = experience_orb_pickup_particle_tint(instance.age_ticks);
    append_experience_orb_quad(
        mesh,
        instance.transform,
        instance.icon,
        tint,
        instance.light,
        texture_uv,
    );
}

/// Vanilla `ExperienceOrbRenderer.submit` quad: the shared billboard corners on the 16px icon
/// cell `icon = ExperienceOrb.getIcon()` of the 64×64 `entity/experience/experience_orb.png`
/// sheet.
fn append_experience_orb_quad(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    icon: i32,
    tint: [f32; 4],
    light: [f32; 2],
    texture_uv: EntityModelUvRect,
) {
    let icon = icon.clamp(0, 10);
    let icon_u0 = (icon % 4 * 16) as f32 / 64.0;
    let icon_u1 = (icon % 4 * 16 + 16) as f32 / 64.0;
    let icon_v0 = (icon / 4 * 16) as f32 / 64.0;
    let icon_v1 = (icon / 4 * 16 + 16) as f32 / 64.0;
    append_camera_billboard_quad(
        mesh,
        transform,
        [
            [icon_u0, icon_v1],
            [icon_u1, icon_v1],
            [icon_u1, icon_v0],
            [icon_u0, icon_v0],
        ],
        tint,
        light,
        texture_uv,
    );
}

/// The camera-billboard quad shared by vanilla `ExperienceOrbRenderer` and
/// `DragonFireballRenderer` `vertex` helpers: corners `(x - 0.5, y - 0.25, 0)` for x/y in
/// {0, 1} with a +Y normal, UVs supplied per renderer.
fn append_camera_billboard_quad(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    uvs: [[f32; 2]; 4],
    tint: [f32; 4],
    light: [f32; 2],
    texture_uv: EntityModelUvRect,
) {
    let normal = transform
        .transform_vector3(Vec3::Y)
        .normalize_or_zero()
        .to_array();
    let corners = [
        Vec3::new(-0.5, -0.25, 0.0),
        Vec3::new(0.5, -0.25, 0.0),
        Vec3::new(0.5, 0.75, 0.0),
        Vec3::new(-0.5, 0.75, 0.0),
    ];
    let base = mesh.vertex_count() as u32;
    mesh.vertices.extend(
        corners
            .iter()
            .copied()
            .zip(uvs.iter().copied())
            .map(|(corner, uv)| EntityModelTexturedVertex {
                position: (transform * corner.extend(1.0)).truncate().to_array(),
                uv: normalized_entity_texture_uv(uv, texture_uv),
                tint,
                light,
                overlay: ENTITY_VERTEX_NO_OVERLAY,
                normal,
                standalone_local_uv: [0.0; 2],
                standalone_pixel_origin: [0.0; 2],
                standalone_size_flags: [0.0; 4],
            }),
    );
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.cutout_faces += 1;
}

fn normalized_entity_texture_uv(uv: [f32; 2], rect: EntityModelUvRect) -> [f32; 2] {
    [
        rect.min[0] + uv[0] * (rect.max[0] - rect.min[0]),
        rect.min[1] + uv[1] * (rect.max[1] - rect.min[1]),
    ]
}

/// Vanilla 26.1 `ExperienceOrbRenderer.submit`: translate up 0.1, face the camera
/// (`mulPose(camera.orientation)`), scale 0.3, and emit one icon quad as
/// `RenderTypes.entityTranslucentCullItemTarget` with the age-driven red/blue sine tint at
/// alpha 128. Without a camera orientation (no frame rendered yet) the billboard is skipped; the
/// mesh rebuild on the first camera update restores it.
pub(in crate::entity_models) fn render_experience_orb(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let EntityModelKind::ExperienceOrb { icon } = instance.kind else {
        return;
    };
    let Some(camera_orientation) = meshes.camera_orientation else {
        return;
    };
    let transform = Mat4::from_translation(Vec3::from_array(instance.position) + Vec3::Y * 0.1)
        * camera_orientation
        * Mat4::from_scale(Vec3::splat(0.3));
    let tint = experience_orb_pickup_particle_tint(instance.render_state.age_in_ticks);
    let light = instance.render_state.shader_light();
    let submit = meshes
        .submission(
            EntityModelLayerRenderType::EntityTranslucentCullItemTarget,
            EXPERIENCE_ORB_TEXTURE_REF,
            tint,
            transform,
            0,
        )
        .with_light(light);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        append_experience_orb_quad(mesh, transform, icon, tint, light, entry.uv);
    });
}

/// Vanilla 26.1 `DragonFireballRenderer.submit`: scale 2, face the camera
/// (`mulPose(camera.orientation)`), and emit one full-sheet white quad as
/// `RenderTypes.entityCutout`. The constant `getBlockLightLevel` = 15 arrives through the
/// instance light coords (generated static-full-bright renderer catalog). Without a camera
/// orientation (no frame rendered yet) the billboard is skipped; the mesh rebuild on the first
/// camera update restores it.
pub(in crate::entity_models) fn render_dragon_fireball(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::DragonFireball) {
        return;
    }
    let Some(camera_orientation) = meshes.camera_orientation else {
        return;
    };
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_scale(Vec3::splat(2.0))
        * camera_orientation;
    let light = instance.render_state.shader_light();
    let submit = meshes
        .submission(
            EntityModelLayerRenderType::EntityCutout,
            DRAGON_FIREBALL_TEXTURE_REF,
            [1.0; 4],
            transform,
            0,
        )
        .with_light(light);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        append_camera_billboard_quad(
            mesh,
            transform,
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            submit.tint,
            light,
            entry.uv,
        );
    });
}

/// Vanilla 26.1 `FishingHookRenderer.submit`: scale by one half, face the camera,
/// submit the full fishing-hook sheet through `entityCutoutCull`, then submit the
/// owner-dependent parabola through `RenderTypes.lines()`.
pub(in crate::entity_models) fn render_fishing_hook(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::FishingHook)
        || instance
            .render_state
            .kind
            .fishing_hook()
            .line_origin_offset
            .is_none()
    {
        return;
    }
    let Some(camera_orientation) = meshes.camera_orientation else {
        return;
    };
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_scale(Vec3::splat(0.5))
        * camera_orientation;
    let light = instance.render_state.shader_light();
    let submit = meshes
        .submission(
            EntityModelLayerRenderType::EntityCutoutCull,
            FISHING_HOOK_TEXTURE_REF,
            [1.0; 4],
            transform,
            0,
        )
        .with_light(light);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        append_fishing_hook_quad(mesh, transform, light, entry.uv);
    });
    meshes.take_submission_index();
}

fn append_fishing_hook_quad(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    light: [f32; 2],
    texture_uv: EntityModelUvRect,
) {
    let normal = transform
        .transform_vector3(Vec3::Y)
        .normalize_or_zero()
        .to_array();
    let corners = [
        (Vec3::new(-0.5, -0.5, 0.0), [0.0, 1.0]),
        (Vec3::new(0.5, -0.5, 0.0), [1.0, 1.0]),
        (Vec3::new(0.5, 0.5, 0.0), [1.0, 0.0]),
        (Vec3::new(-0.5, 0.5, 0.0), [0.0, 0.0]),
    ];
    let base = mesh.vertex_count() as u32;
    mesh.vertices
        .extend(corners.map(|(corner, uv)| EntityModelTexturedVertex {
            position: (transform * corner.extend(1.0)).truncate().to_array(),
            uv: normalized_entity_texture_uv(uv, texture_uv),
            tint: [1.0; 4],
            light,
            overlay: ENTITY_VERTEX_NO_OVERLAY,
            normal,
            standalone_local_uv: [0.0; 2],
            standalone_pixel_origin: [0.0; 2],
            standalone_size_flags: [0.0; 4],
        }));
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.cutout_faces += 1;
}

/// Vanilla `FishingHookRenderer.stringVertex` sampled over its fixed 16-step
/// parabola. The state offset ends at the rendered hand because its Y component
/// is measured from `(hookPosition + (0, 0.25, 0))`.
pub(crate) fn fishing_hook_line_segments(
    instance: &EntityModelInstance,
    line_width: f32,
) -> Option<[crate::SelectionLine; 16]> {
    if !matches!(instance.kind, EntityModelKind::FishingHook) {
        return None;
    }
    let [x, y, z] = instance
        .render_state
        .kind
        .fishing_hook()
        .line_origin_offset?;
    let point = |fraction: f32| {
        [
            instance.position[0] + x * fraction,
            instance.position[1] + y * (fraction * fraction + fraction) * 0.5 + 0.25,
            instance.position[2] + z * fraction,
        ]
    };
    Some(std::array::from_fn(|step| {
        let from = step as f32 / 16.0;
        let to = (step + 1) as f32 / 16.0;
        crate::SelectionLine {
            from: point(from),
            to: point(to),
            color: [0.0, 0.0, 0.0, 1.0],
            width: line_width,
        }
    }))
}

fn experience_orb_pickup_particle_tint(age_ticks: f32) -> [f32; 4] {
    let rr = age_ticks / 2.0;
    let red = (((rr).sin() + 1.0) * 0.5 * 255.0) as i32;
    let blue = (((rr + std::f32::consts::PI * 4.0 / 3.0).sin() + 1.0) * 0.1 * 255.0) as i32;
    [
        red.clamp(0, 255) as f32 / 255.0,
        1.0,
        blue.clamp(0, 255) as f32 / 255.0,
        128.0 / 255.0,
    ]
}

pub(in crate::entity_models) fn first_person_player_arm_textured_meshes(
    arms: &[FirstPersonPlayerArm],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new(None);
    for arm in arms.iter().copied() {
        render_first_person_player_arm(&mut meshes, arm, atlas, dynamic_player_skin_atlas);
    }
    meshes
}

fn render_first_person_player_arm(
    meshes: &mut EntityModelTexturedMeshes,
    arm: FirstPersonPlayerArm,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let arm_name = if arm.left { "left_arm" } else { "right_arm" };
    let mut model = player_model_layer(arm.skin.is_slim()).bake();
    let root = &mut model;
    let selected = root.child_mut(arm_name);
    selected.reset_pose();
    selected.visible = true;
    selected.pose.rotation[2] = if arm.left { -0.1 } else { 0.1 };
    let sleeve = if arm.left {
        "left_sleeve"
    } else {
        "right_sleeve"
    };
    selected.child_mut(sleeve).visible = arm.sleeve_visible;

    let texture = default_player_skin_texture_ref(arm.skin.fallback());
    let mut pass =
        player_textured_layer_passes_with_texture(PLAYER_MODEL_PARTS_ALL_VISIBLE, texture)[0];
    pass.render_type = EntityModelLayerRenderType::EntityTranslucent;

    let submit = textured_layer_submission(meshes, pass, arm.transform)
        .with_light(arm.light)
        .with_overlay(ENTITY_VERTEX_NO_OVERLAY);
    let Some(arm_part) = model.try_child(arm_name) else {
        return;
    };

    if let EntityPlayerSkin::Dynamic(dynamic_player_skin) = arm.skin {
        if dynamic_player_skin.status == EntityDynamicPlayerSkinStatus::Ready {
            if let Some(entry) = dynamic_player_skin_atlas_entry(
                dynamic_player_skin_atlas,
                dynamic_player_skin.handle,
            ) {
                render_textured_dynamic_player_skin_submission(
                    meshes,
                    submit,
                    entry,
                    |mesh, entry| {
                        arm_part.render_textured(mesh, submit.transform, entry.uv, submit.tint);
                    },
                );
                return;
            }
        }

        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            arm_part.render_textured(mesh, submit.transform, entry.uv, submit.tint);
        });
        return;
    }

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        arm_part.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

pub(in crate::entity_models) fn entity_model_water_mask_mesh(
    instances: &[EntityModelInstance],
) -> EntityModelMesh {
    let mut mesh = EntityModelMesh::new();
    for instance in instances {
        emit_entity_water_mask(&mut mesh, instance);
    }
    mesh
}

pub(super) fn emit_entity_water_mask(mesh: &mut EntityModelMesh, instance: &EntityModelInstance) {
    if !instance.render_state.invisible && boat_water_mask_layer_pass(instance).is_some() {
        append_boat_water_mask_mesh(mesh, boat_model_root_transform(instance));
    }
}

#[cfg(test)]
pub(in crate::entity_models) fn dynamic_player_texture_test_meshes(
    render_type: EntityModelLayerRenderType,
    dynamic_player_texture: EntityDynamicPlayerTexture,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new(None);
    let model = player_model_layer(false).bake();
    render_textured_pass_with_dynamic_player_texture(
        &mut meshes,
        &model,
        Mat4::IDENTITY,
        render_type,
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        dynamic_player_texture,
        [0.25, 0.5, 0.75, 1.0],
        atlas,
        dynamic_player_texture_atlas,
    );
    meshes
}

fn boat_water_mask_layer_pass(instance: &EntityModelInstance) -> Option<EntityModelLayerPass> {
    let EntityModelKind::Boat { family, chest } = instance.kind else {
        return None;
    };
    // Vanilla `BoatRenderer.submitTypeAdditions`: wooden boats and chest boats submit the
    // `ModelLayers.BOAT_WATER_PATCH` depth-only `RenderTypes.waterMask()` model when not underwater.
    // Bamboo rafts use `RaftRenderer`, which does not override `submitTypeAdditions`.
    if instance.render_state.kind.boat().underwater {
        return None;
    }
    let passes = boat_textured_layer_passes(family, chest);
    passes.get(1).copied()
}

fn append_boat_water_mask_mesh(mesh: &mut EntityModelMesh, transform: Mat4) {
    let model = ModelLayer::BOAT_WATER_PATCH.bake();
    model.render_untextured(mesh, transform);
}

pub(in crate::entity_models) fn render_trident_foil_submission(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    if !matches!(instance.kind, EntityModelKind::Trident)
        || !instance.render_state.kind.trident().foil
    {
        return;
    }
    let passes = trident_textured_layer_passes();
    let pass = passes[1];
    let model = ModelLayer::TRIDENT.bake();
    render_textured_no_overlay_layer_pass(
        meshes,
        &model,
        trident_model_root_transform(instance),
        pass,
        atlas,
    );
}

fn render_scrolled_textured_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submission: EntityModelRenderSubmission,
    atlas: &EntityModelTextureAtlasLayout,
    uv_offset: [f32; 2],
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityModelTextureAtlasEntry),
) {
    let owner = scroll_owner_for_render_type(submission.render_type);
    render_scroll_geometry_submission(meshes, submission, owner, atlas, |target, entry, submit| {
        let mut scratch = EntityModelTexturedMesh::new();
        emit(&mut scratch, entry);
        fill_textured_submission_vertices(&mut scratch, 0, submit);
        append_scrolled_textured_mesh(target, &scratch, entry.uv, uv_offset);
    });
}

pub(in crate::entity_models) fn render_no_overlay_scrolled_textured_layers(
    meshes: &mut EntityModelTexturedMeshes,
    model: &ModelPart,
    transform: Mat4,
    passes: impl IntoIterator<Item = EntityModelLayerPass>,
    atlas: &EntityModelTextureAtlasLayout,
    uv_offset: [f32; 2],
) {
    for pass in passes {
        let submit = no_overlay_layer_submission(meshes, pass, transform);
        render_scrolled_textured_submission(meshes, submit, atlas, uv_offset, |mesh, entry| {
            model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
        });
    }
}

fn render_scroll_geometry_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submission: EntityModelRenderSubmission,
    owner: EntityModelScrollOwner,
    atlas: &EntityModelTextureAtlasLayout,
    emit: impl FnOnce(
        &mut EntityModelScrollMesh,
        EntityModelTextureAtlasEntry,
        EntityModelRenderSubmission,
    ),
) {
    let insertion_index = meshes.take_submission_index();
    let Some(entry) = entity_model_texture_atlas_entry(atlas, submission.texture) else {
        return;
    };
    let sort_key = meshes.sorted_upload_key(submission, insertion_index);
    let target = scroll_owner_mesh_mut(meshes, owner);
    let index_start = u32::try_from(target.indices.len()).expect("scroll index start fits in u32");
    emit(target, entry, submission);
    let index_end = u32::try_from(target.indices.len()).expect("scroll index end fits in u32");
    let index_count = index_end - index_start;
    if let Some(sort_key) = sort_key {
        meshes.push_geometry_draw_range(submission.render_type, sort_key, index_start, index_count);
    } else if owner.is_opaque_beam() && index_count > 0 {
        meshes.opaque_beam_draws.push(EntityModelDrawRange {
            render_type: submission.render_type,
            sort_key: EntityModelSubmitSortKey {
                order: submission.order,
                distance_sq: 0.0,
                position: submission.transform.transform_point3(Vec3::ZERO).to_array(),
                insertion_index,
            },
            index_start,
            index_count,
        });
    }
}

/// The breeze's swirling wind body (vanilla `BreezeWindLayer`): the wind-layer [`BreezeModel`] (the
/// `wind_body` shell chain on the 128×128 `breeze_wind.png`) rendered with the `breezeWind` render
/// type, whose texture matrix scrolls the U coordinate by `xOffset(ageInTicks) % 1 = (ageInTicks ·
/// 0.02) % 1` (V fixed at `0`). Like the wind charge, we render the wind model once with the normal
/// atlas UVs into a scratch mesh — its `setup_anim` applies the same idle sway + action swirls/pulses
/// as the base body so the two layers move together — then fold it into the translucent scrolling
/// overlay mesh, baking the per-instance U offset and carrying the atlas sub-rect for the shader wrap.
pub(in crate::entity_models) fn render_breeze_wind_scroll_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::Breeze) {
        return;
    }
    let transform = entity_model_root_transform(instance);
    let passes = breeze_textured_layer_passes();
    let pass = passes[1];
    let submit = no_overlay_layer_submission(meshes, pass, transform);
    let model = breeze_model(ModelLayer::BREEZE_WIND, instance);
    // Vanilla `BreezeWindLayer.xOffset(t) = t · 0.02`, taken `% 1.0`; `ageInTicks ≥ 0` so the Java
    // float modulo is `rem_euclid`. V does not scroll.
    let u_offset = (instance.render_state.age_in_ticks * 0.02).rem_euclid(1.0);
    render_scrolled_textured_submission(meshes, submit, atlas, [u_offset, 0.0], |mesh, entry| {
        model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

/// The charged creeper's `CreeperPowerLayer` energy swirl (vanilla `EnergySwirlLayer`): when the
/// synced `isPowered` is set, the inflated `CREEPER_ARMOR` model (`CubeDeformation 2.0`, driven by the
/// same `setup_anim` so it tracks the body pose) is drawn with the additive, emissive `energySwirl`
/// render type — `creeper_armor.png` scrolling on both axes by `xOffset(ageInTicks) % 1 =
/// (ageInTicks · 0.01) % 1`, tinted by the vanilla `0xFF808080` half-grey. Folded into the additive
/// scroll mesh the same way the wind charge folds into the translucent one.
pub(in crate::entity_models) fn render_charged_creeper_energy_swirl(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !instance.render_state.kind.creeper().powered
        || !matches!(instance.kind, EntityModelKind::Creeper)
    {
        return;
    }
    let transform = creeper_model_root_transform(instance);
    let passes = creeper_textured_layer_passes();
    let pass = passes[1];
    let submit = no_overlay_layer_submission(meshes, pass, transform);
    let model = creeper_model(ModelLayer::CREEPER_ARMOR, instance);
    // Vanilla creeper `xOffset(t) = t · 0.01`, taken `% 1.0` on both U and V.
    let offset = (instance.render_state.age_in_ticks * 0.01).rem_euclid(1.0);
    // Vanilla `EnergySwirlLayer` tints by `0xFF808080` (half grey) under additive blend.
    render_scrolled_textured_submission(meshes, submit, atlas, [offset, offset], |mesh, entry| {
        model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

/// The wither boss's `WitherArmorLayer` energy swirl (vanilla `EnergySwirlLayer`, the same family as
/// the charged creeper): when `isPowered` (the wither sits at or below half health), the inflated
/// `WITHER_ARMOR` model (`INNER_ARMOR_DEFORMATION` = `CubeDeformation 0.5`, driven by the same
/// `setup_anim` so it breathes with the body) is drawn with the additive, emissive `energySwirl`
/// render type — `wither_armor.png` tinted by the vanilla `0xFF808080` half-grey. Unlike the creeper's
/// linear scroll, the wither's `xOffset(t) = cos(t · 0.02) · 3` oscillates the U coordinate while V
/// scrolls linearly at `t · 0.01`; both are taken `% 1.0`. Folded into the same additive scroll mesh.
pub(in crate::entity_models) fn render_wither_energy_swirl(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !instance.render_state.kind.wither().powered
        || !matches!(instance.kind, EntityModelKind::Wither)
    {
        return;
    }
    let transform = wither_model_root_transform(instance);
    let passes =
        wither_textured_layer_passes(instance.render_state.kind.wither().invulnerable_ticks);
    let pass = passes[1];
    let submit = no_overlay_layer_submission(meshes, pass, transform);
    let model = wither_model(ModelLayer::WITHER_ARMOR, instance);
    // Vanilla `WitherArmorLayer.xOffset(t) = cos(t · 0.02) · 3` on U (oscillating, not linear like the
    // creeper), `t · 0.01` on V, each taken `% 1.0`. Java float modulo of a possibly-negative U keeps
    // the sign, then the shader's `fract` re-wraps it into `[0, 1)`, so plain `% 1.0` (`Rust` `rem`,
    // not `rem_euclid`) reproduces the vanilla offset exactly.
    let age = instance.render_state.age_in_ticks;
    let u_offset = ((age * 0.02).cos() * 3.0) % 1.0;
    let v_offset = (age * 0.01).rem_euclid(1.0);
    // Vanilla `EnergySwirlLayer` tints by `0xFF808080` (half grey) under additive blend.
    render_scrolled_textured_submission(
        meshes,
        submit,
        atlas,
        [u_offset, v_offset],
        |mesh, entry| {
            model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
        },
    );
}

/// The guardian attack beam (vanilla `GuardianRenderer.renderBeam`). When the guardian has an active
/// attack target, a world-space twisted prism is drawn from the guardian eye toward the target along
/// the world `beamVector` (`eye_to_target`): two crossed longitudinal strips (the inner `0.2`-radius
/// rays) plus a twisting `0.282`-radius top cap, the whole thing spun by `rot = attackTime · 0.05 ·
/// -1.5` and tinted by the attack-scale color ramp (`colorScale = scale²`). The `guardian_beam.png`
/// texture tiles vertically (V spans `length · 2.5` units, scrolled by `texVOff`) via the scroll
/// (fract-wrap) pass. Built in a world-aligned frame (`translate(pos) · translate(0, eyeHeight, 0) ·
/// rotY(yRot) · rotX(xRot)`, no body yaw / model flip), mirroring vanilla where the beam draws after
/// `super.submit` has popped the model's `setupRotations` back to the entity-origin frame.
pub(in crate::entity_models) fn render_guardian_beam(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::Guardian { .. }) {
        return;
    }
    let Some(beam) = instance.render_state.kind.guardian().beam else {
        return;
    };

    // Orient local +Y onto the world beam direction, then lift the origin from the entity feet to the
    // eye. Vanilla: `xRot = acos(dir.y)`, `yRot = π/2 − atan2(dir.z, dir.x)`.
    let beam_vector = Vec3::from_array(beam.eye_to_target);
    let length = beam_vector.length() + 1.0;
    let dir = beam_vector.normalize_or_zero();
    let x_rot = dir.y.clamp(-1.0, 1.0).acos();
    let y_rot = std::f32::consts::FRAC_PI_2 - dir.z.atan2(dir.x);
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(Vec3::new(0.0, beam.eye_height, 0.0))
        * Mat4::from_rotation_y(y_rot)
        * Mat4::from_rotation_x(x_rot);

    // The prism cross-section: four inner rays at radius 0.2 and four outer cap rays at 0.282, each
    // offset around the beam axis by a fixed angle plus the time spin `rot`.
    use std::f32::consts::PI;
    let rot = beam.attack_time * 0.05 * -1.5;
    let ring = |angle: f32, radius: f32| {
        let a = rot + angle;
        (a.cos() * radius, a.sin() * radius)
    };
    let (wnx, wnz) = ring(PI * 3.0 / 4.0, 0.282);
    let (enx, enz) = ring(PI / 4.0, 0.282);
    let (wsx, wsz) = ring(PI * 5.0 / 4.0, 0.282);
    let (esx, esz) = ring(PI * 7.0 / 4.0, 0.282);
    let (wx, wz) = ring(PI, 0.2);
    let (ex, ez) = ring(0.0, 0.2);
    let (nx, nz) = ring(PI / 2.0, 0.2);
    let (sx, sz) = ring(PI * 3.0 / 2.0, 0.2);

    // Vanilla color ramp from the attack scale, truncated to ints exactly as the `(int)` casts do.
    let color_scale = beam.attack_scale * beam.attack_scale;
    let tint = [
        (64 + (color_scale * 191.0) as i32) as f32 / 255.0,
        (32 + (color_scale * 191.0) as i32) as f32 / 255.0,
        (128 - (color_scale * 64.0) as i32) as f32 / 255.0,
        1.0,
    ];
    // `GuardianRenderer.submit` calls `super.submit` first, then submits custom geometry with
    // `BEAM_RENDER_TYPE = RenderTypes.entityCutout(guardian_beam.png)` on the same collector. Consume
    // the explicit beam pass metadata before folding the tiled V coordinates into the scroll mesh.
    let elder = matches!(instance.kind, EntityModelKind::Guardian { elder: true });
    let passes = guardian_textured_layer_passes(elder);
    let beam_pass = passes[1];
    let submit = no_overlay_layer_submission_with_tint(meshes, beam_pass, tint, transform)
        .with_light(ENTITY_VERTEX_FULL_BRIGHT_LIGHT);
    render_scroll_geometry_submission(
        meshes,
        submit,
        EntityModelScrollOwner::GuardianBeam,
        atlas,
        |mesh, entry, submit| {
            let top = length;
            let tex_v_off = (beam.attack_time * 0.5).rem_euclid(1.0);
            let min_v = -1.0 + tex_v_off;
            let max_v = min_v + length * 2.5;
            let v_base = if (beam.attack_time.floor() as i32).rem_euclid(2) == 0 {
                0.5
            } else {
                0.0
            };

            // 12 vertices in three quads (W↔E strip, N↔S strip, twisting top cap), local UVs in
            // `0..1` for U and tiling for V — matching `GuardianRenderer.vertex` exactly.
            let vertices: [(f32, f32, f32, f32, f32); 12] = [
                (wx, top, wz, 0.4999, max_v),
                (wx, 0.0, wz, 0.4999, min_v),
                (ex, 0.0, ez, 0.0, min_v),
                (ex, top, ez, 0.0, max_v),
                (nx, top, nz, 0.4999, max_v),
                (nx, 0.0, nz, 0.4999, min_v),
                (sx, 0.0, sz, 0.0, min_v),
                (sx, top, sz, 0.0, max_v),
                (wnx, top, wnz, 0.5, v_base + 0.5),
                (enx, top, enz, 1.0, v_base + 0.5),
                (esx, top, esz, 1.0, v_base),
                (wsx, top, wsz, 0.5, v_base),
            ];
            let rect = entry.uv;
            let size = [rect.max[0] - rect.min[0], rect.max[1] - rect.min[1]];
            let base = u32::try_from(mesh.vertices.len()).expect("scroll vertex count fits in u32");
            let normal = submit
                .transform
                .transform_vector3(Vec3::Y)
                .normalize_or_zero()
                .to_array();
            for (x, y, z, u, v) in vertices {
                let world = submit.transform.transform_point3(Vec3::new(x, y, z));
                mesh.vertices.push(EntityModelScrollVertex {
                    position: world.to_array(),
                    local_uv: [u, v],
                    uv_rect_min: rect.min,
                    uv_rect_size: size,
                    tint: submit.tint,
                    light: submit.light,
                    overlay: submit.overlay,
                    normal,
                });
            }
            // Each quad → two triangles (the scroll pipeline renders cull-off, so winding is
            // immaterial).
            for quad in 0..3u32 {
                let o = base + quad * 4;
                mesh.indices
                    .extend_from_slice(&[o, o + 1, o + 2, o, o + 2, o + 3]);
            }
        },
    );
}

/// Vanilla `EnderDragonRenderer.submitCrystalBeams`, as called by `EndCrystalRenderer.submit` when
/// `EndCrystalRenderState.beamOffset` is present. The crystal renderer first translates by the beam
/// target offset, then the shared helper translates up by two units, rotates local +Z toward the
/// crystal→beam-target delta (including the crystal bob), and submits eight prism quads with black
/// inner vertices, white outer vertices, and a vertically tiled `end_crystal_beam.png` texture.
pub(in crate::entity_models) fn render_end_crystal_beam(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::EndCrystal) {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    let Some(beam) = instance.render_state.kind.end_crystal().beam else {
        return;
    };

    let beam_offset = Vec3::from_array(beam.beam_offset);
    let age = instance.render_state.age_in_ticks;
    let delta = Vec3::new(
        -beam_offset.x,
        -beam_offset.y + end_crystal_get_y(age),
        -beam_offset.z,
    );
    let horizontal_length = (delta.x * delta.x + delta.z * delta.z).sqrt();
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(beam_offset)
        * Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0))
        * Mat4::from_rotation_y(-delta.z.atan2(delta.x) - std::f32::consts::FRAC_PI_2)
        * Mat4::from_rotation_x(-horizontal_length.atan2(delta.y) - std::f32::consts::FRAC_PI_2);
    let passes = end_crystal_textured_layer_passes();
    let beam_pass = passes[1];
    let submit = no_overlay_layer_submission(meshes, beam_pass, transform);
    emit_crystal_beam_submission(meshes, submit, atlas, delta.length(), age);
}

/// Vanilla `EnderDragonRenderer.submit`: after body and eyes submits, a dragon with
/// `EnderDragonRenderState.beamOffset` calls the same `submitCrystalBeams` helper from the dragon's
/// entity-origin pose. Unlike an end crystal, the dragon does not pre-translate by the offset and does
/// not invert the delta; its `beamOffset` already points from the dragon to the bobbed crystal.
pub(in crate::entity_models) fn render_ender_dragon_beam(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::EnderDragon) {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    let Some(beam) = instance.render_state.kind.ender_dragon().beam else {
        return;
    };

    let delta = Vec3::from_array(beam.beam_offset);
    let horizontal_length = (delta.x * delta.x + delta.z * delta.z).sqrt();
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0))
        * Mat4::from_rotation_y(-delta.z.atan2(delta.x) - std::f32::consts::FRAC_PI_2)
        * Mat4::from_rotation_x(-horizontal_length.atan2(delta.y) - std::f32::consts::FRAC_PI_2);
    let passes =
        ender_dragon_textured_layer_passes(instance.render_state.kind.ender_dragon().death_time);
    let beam_pass = passes[2];
    let submit = no_overlay_layer_submission(meshes, beam_pass, transform);
    emit_crystal_beam_submission(
        meshes,
        submit,
        atlas,
        delta.length(),
        instance.render_state.age_in_ticks,
    );
}

/// Vanilla `EnderDragonRenderer.submit`: immediately after body and eyes, a dying dragon pushes the
/// current dragon pose by `(0, -1, -2)` and submits the same seeded custom ray geometry twice:
/// `RenderTypes.dragonRays()` for additive colour and `RenderTypes.dragonRaysDepth()` for the
/// depth-only replay. The geometry is no-texture position-colour custom geometry, so it records a
/// separate custom submission instead of pretending to be texture-backed.
pub(in crate::entity_models) fn render_ender_dragon_death_rays(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
) {
    if !matches!(instance.kind, EntityModelKind::EnderDragon) {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    let death_time = instance.render_state.kind.ender_dragon().death_time;
    if death_time <= 0.0 {
        return;
    }

    let transform = ender_dragon_model_root_transform(instance)
        * Mat4::from_translation(Vec3::new(0.0, -1.0, -2.0));
    let normalized_death_time = death_time / 200.0;
    emit_dragon_rays_custom_submission(
        meshes,
        EntityModelLayerRenderType::DragonRays,
        transform,
        normalized_death_time,
    );
    emit_dragon_rays_custom_submission(
        meshes,
        EntityModelLayerRenderType::DragonRaysDepth,
        transform,
        normalized_death_time,
    );
}

/// Vanilla `LeashFeatureRenderer.LEASH_RENDER_STEPS`.
const LEASH_RENDER_STEPS: u32 = 24;

/// Vanilla `LeashFeatureRenderer.LEASH_WIDTH`.
const LEASH_WIDTH: f32 = 0.05;

/// Vanilla `addVertexPair`'s constant rope colour before the alternating 0.7 modifier.
const LEASH_COLOR: [f32; 3] = [0.5, 0.4, 0.3];

pub(super) fn emit_leash_rope_custom_submission(
    meshes: &mut EntityModelTexturedMeshes,
    entity_position: [f32; 3],
    rope: LeashRopeRenderState,
) {
    let render_type = EntityModelLayerRenderType::Leash;
    // Vanilla translates the entity render pose by `leashState.offset`, so the rope's local origin
    // is the world-space rope start.
    let transform =
        Mat4::from_translation(Vec3::from_array(entity_position) + Vec3::from_array(rope.offset));
    let insertion_index = meshes.take_submission_index();
    let mut mesh = EntityModelMesh::new();
    append_leash_rope_mesh(&mut mesh, transform, rope);
    let index_count = u32::try_from(mesh.indices.len()).expect("leash index count fits in u32");
    let index_start = append_entity_model_mesh(&mut meshes.leash, mesh);
    if let Some(sort_key) = meshes.geometry_draw_key(transform, 0, insertion_index) {
        meshes.push_geometry_draw_range(render_type, sort_key, index_start, index_count);
    }
}

/// Vanilla `LeashFeatureRenderer.renderLeash`: two 25-pair passes over the same vertex stream (the
/// forward pass fudges the first vertex of each pair up by the rope width, the backward pass the
/// second), submitted as one `TRIANGLE_STRIP`. The shared mesh is an indexed triangle list, so strip
/// triangle `i` unrolls to vertices `(i, i+1, i+2)`; the LEASH pipeline never culls, which makes the
/// strip's alternating winding irrelevant.
fn append_leash_rope_mesh(mesh: &mut EntityModelMesh, transform: Mat4, rope: LeashRopeRenderState) {
    let [dx, _, dz] = rope.delta;
    let horizontal_sq = dx * dx + dz * dz;
    // Vanilla `Mth.invSqrt(0)` would poison a perfectly vertical rope with NaN cross-section
    // offsets; collapse the cross-section instead so the rope degenerates to a line.
    let offset_factor = if horizontal_sq > 0.0 {
        horizontal_sq.sqrt().recip() * LEASH_WIDTH / 2.0
    } else {
        0.0
    };
    let dx_off = dz * offset_factor;
    let dz_off = dx * offset_factor;
    let base_vertex = u32::try_from(mesh.vertices.len()).expect("leash vertex count fits in u32");
    for step in 0..=LEASH_RENDER_STEPS {
        append_leash_vertex_pair(
            mesh,
            transform,
            rope,
            LEASH_WIDTH,
            dx_off,
            dz_off,
            step,
            false,
        );
    }
    for step in (0..=LEASH_RENDER_STEPS).rev() {
        append_leash_vertex_pair(mesh, transform, rope, 0.0, dx_off, dz_off, step, true);
    }
    let vertex_count =
        u32::try_from(mesh.vertices.len()).expect("leash vertex count fits in u32") - base_vertex;
    for index in 0..vertex_count.saturating_sub(2) {
        mesh.indices.extend_from_slice(&[
            base_vertex + index,
            base_vertex + index + 1,
            base_vertex + index + 2,
        ]);
    }
}

/// Vanilla `LeashFeatureRenderer.addVertexPair` for one strip step.
#[allow(clippy::too_many_arguments)]
fn append_leash_vertex_pair(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    rope: LeashRopeRenderState,
    fudge: f32,
    dx_off: f32,
    dz_off: f32,
    step: u32,
    backwards: bool,
) {
    let [dx, dy, dz] = rope.delta;
    let progress = step as f32 / LEASH_RENDER_STEPS as f32;
    let block = leash_lerped_light(progress, rope.start_block_light, rope.end_block_light);
    let sky = leash_lerped_light(progress, rope.start_sky_light, rope.end_sky_light);
    let light = [block / 15.0, sky / 15.0];
    let color_modifier = if step % 2 == u32::from(backwards) {
        0.7
    } else {
        1.0
    };
    let color = [
        LEASH_COLOR[0] * color_modifier,
        LEASH_COLOR[1] * color_modifier,
        LEASH_COLOR[2] * color_modifier,
        1.0,
    ];
    let x = dx * progress;
    let y = if rope.slack {
        // The sag curve leans toward the lower endpoint regardless of which side is higher.
        if dy > 0.0 {
            dy * progress * progress
        } else {
            dy - dy * (1.0 - progress) * (1.0 - progress)
        }
    } else {
        dy * progress
    };
    let z = dz * progress;
    for position in [
        [x - dx_off, y + fudge, z + dz_off],
        [x + dx_off, y + LEASH_WIDTH - fudge, z - dz_off],
    ] {
        let world = transform.transform_point3(Vec3::from_array(position));
        mesh.vertices.push(EntityModelVertex {
            position: world.to_array(),
            color,
            light,
            overlay: ENTITY_VERTEX_NO_OVERLAY,
            normal: [0.0, 1.0, 0.0],
        });
    }
}

/// Vanilla `(int)Mth.lerp(progress, start, end)`: the lerped light truncates to an integer level
/// before packing into the light coords.
fn leash_lerped_light(progress: f32, start: u8, end: u8) -> f32 {
    (f32::from(start) + progress * (f32::from(end) - f32::from(start))).trunc()
}

pub(in crate::entity_models) fn render_end_portal_block_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let EntityModelKind::EndPortalBlock { kind, faces } = instance.kind else {
        return;
    };
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }

    let render_type = match kind {
        EndPortalModelKind::EndPortal => EntityModelLayerRenderType::EndPortal,
        EndPortalModelKind::EndGateway => EntityModelLayerRenderType::EndGateway,
    };
    let cube_transform = end_portal_block_root_transform(instance, kind);
    let insertion_index = meshes.take_submission_index();
    let effective_faces = match kind {
        EndPortalModelKind::EndPortal => EndPortalModelFaces::from_faces(
            &[EndPortalModelFace::Down, EndPortalModelFace::Up]
                .into_iter()
                .filter(|&face| faces.contains(face))
                .collect::<Vec<_>>(),
        ),
        EndPortalModelKind::EndGateway => faces,
    };
    if !effective_faces.is_empty() {
        if let (Some(sky), Some(portal)) = (
            entity_model_texture_atlas_entry(atlas, END_SKY_TEXTURE_REF),
            entity_model_texture_atlas_entry(atlas, END_PORTAL_TEXTURE_REF),
        ) {
            let mut mesh = EntityModelPortalMesh::new();
            append_end_portal_cube_mesh(
                &mut mesh,
                cube_transform,
                effective_faces,
                sky.uv,
                portal.uv,
            );
            let index_count =
                u32::try_from(mesh.indices.len()).expect("end portal cube index count fits in u32");
            let index_start = match kind {
                EndPortalModelKind::EndPortal => append_portal_mesh(&mut meshes.end_portal, mesh),
                EndPortalModelKind::EndGateway => append_portal_mesh(&mut meshes.end_gateway, mesh),
            };
            if let Some(sort_key) = meshes.geometry_draw_key(cube_transform, 0, insertion_index) {
                meshes.push_geometry_draw_range(render_type, sort_key, index_start, index_count);
            }
        }
    }
    if kind == EndPortalModelKind::EndGateway {
        render_end_gateway_beam(meshes, instance, atlas);
    }
}

fn end_portal_block_root_transform(
    instance: &EntityModelInstance,
    kind: EndPortalModelKind,
) -> Mat4 {
    let transform = Mat4::from_translation(Vec3::from_array(instance.position));
    match kind {
        EndPortalModelKind::EndPortal => {
            transform
                * Mat4::from_translation(Vec3::new(0.0, END_PORTAL_BOTTOM, 0.0))
                * Mat4::from_scale(Vec3::new(1.0, END_PORTAL_HEIGHT, 1.0))
        }
        EndPortalModelKind::EndGateway => transform,
    }
}

fn append_end_portal_cube_mesh(
    mesh: &mut EntityModelPortalMesh,
    transform: Mat4,
    faces: EndPortalModelFaces,
    sky_uv: EntityModelUvRect,
    portal_uv: EntityModelUvRect,
) {
    for face in faces.iter() {
        let corners = match face {
            EndPortalModelFace::Down => [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            EndPortalModelFace::Up => [
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 0.0),
            ],
            // Vanilla `FaceInfo.java` winding for cardinal cube faces (outward CCW normals).
            EndPortalModelFace::North => [
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            EndPortalModelFace::South => [
                Vec3::new(0.0, 1.0, 1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
            ],
            EndPortalModelFace::West => [
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(0.0, 1.0, 1.0),
            ],
            EndPortalModelFace::East => [
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
            ],
        };
        append_end_portal_face(mesh, transform, corners, sky_uv, portal_uv);
    }
}

fn append_end_portal_face(
    mesh: &mut EntityModelPortalMesh,
    transform: Mat4,
    corners: [Vec3; 4],
    sky_uv: EntityModelUvRect,
    portal_uv: EntityModelUvRect,
) {
    let base = u32::try_from(mesh.vertices.len()).expect("end portal vertex count fits in u32");
    let sky_rect_size = [sky_uv.max[0] - sky_uv.min[0], sky_uv.max[1] - sky_uv.min[1]];
    let portal_rect_size = [
        portal_uv.max[0] - portal_uv.min[0],
        portal_uv.max[1] - portal_uv.min[1],
    ];
    for corner in corners {
        mesh.vertices.push(EntityModelPortalVertex {
            position: transform.transform_point3(corner).to_array(),
            sky_rect_min: sky_uv.min,
            sky_rect_size,
            portal_rect_min: portal_uv.min,
            portal_rect_size,
        });
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn render_end_gateway_beam(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(beam) = instance.block_entity.end_gateway_beam else {
        return;
    };
    if beam.height <= 0 {
        return;
    }
    // `TheEndGatewayRenderer.submit`: one call spanning `[-height, +height]`.
    emit_beacon_beam_submission(
        meshes,
        atlas,
        end_gateway_beam_layer_passes(),
        Mat4::from_translation(Vec3::from_array(instance.position)),
        beam.color_argb,
        BeaconBeamSubmission {
            beam_start: -beam.height,
            height: beam.height * 2,
            scale: beam.scale,
            animation_time: beam.animation_time,
            solid_radius: 0.15,
            glow_radius: 0.175,
        },
    );
}

pub(in crate::entity_models) fn render_beacon_beam_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(beam) = instance.block_entity.beacon_beam else {
        return;
    };
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    // `BeaconRenderer.submitBeaconBeam(…, 0.2F * scale, 0.25F * scale)` with the
    // fixed `scale = 1.0F` the beacon overload passes.
    emit_beacon_beam_submission(
        meshes,
        atlas,
        beacon_beam_layer_passes(),
        Mat4::from_translation(Vec3::from_array(instance.position)),
        beam.color_argb,
        BeaconBeamSubmission {
            beam_start: beam.beam_start,
            height: beam.height,
            scale: 1.0,
            animation_time: beam.animation_time,
            solid_radius: BEACON_SOLID_BEAM_RADIUS * beam.radius_scale,
            glow_radius: BEACON_BEAM_GLOW_RADIUS * beam.radius_scale,
        },
    );
}

/// `BeaconRenderer.SOLID_BEAM_RADIUS`.
const BEACON_SOLID_BEAM_RADIUS: f32 = 0.2;

/// `BeaconRenderer.BEAM_GLOW_RADIUS`.
const BEACON_BEAM_GLOW_RADIUS: f32 = 0.25;

/// The parameters of one vanilla `BeaconRenderer.submitBeaconBeam` call. Both the
/// beacon and the end gateway route through that single static, so the geometry
/// below is written once and specialised only by these values.
#[derive(Debug, Clone, Copy)]
struct BeaconBeamSubmission {
    beam_start: i32,
    height: i32,
    scale: f32,
    animation_time: f32,
    solid_radius: f32,
    glow_radius: f32,
}

fn emit_beacon_beam_submission(
    meshes: &mut EntityModelTexturedMeshes,
    atlas: &EntityModelTextureAtlasLayout,
    [opaque_pass, translucent_pass]: [EntityModelLayerPass; 2],
    root: Mat4,
    color_argb: u32,
    beam: BeaconBeamSubmission,
) {
    let tint = argb_to_tint(color_argb);
    let opaque_submit = no_overlay_layer_submission_with_tint(meshes, opaque_pass, tint, root)
        .with_light(ENTITY_VERTEX_FULL_BRIGHT_LIGHT);
    let translucent_submit =
        no_overlay_layer_submission_with_tint(meshes, translucent_pass, tint, root)
            .with_light(ENTITY_VERTEX_FULL_BRIGHT_LIGHT);

    let beam_end = beam.beam_start + beam.height;
    let transform = root * Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5));
    let scroll = if beam.height < 0 {
        beam.animation_time
    } else {
        -beam.animation_time
    };
    let tex_v_off = frac(scroll * 0.2 - (scroll * 0.1).floor());
    let vv2 = -1.0 + tex_v_off;

    let solid_radius = beam.solid_radius;
    let solid_vv1 = beam.height as f32 * beam.scale * (0.5 / solid_radius) + vv2;
    let inner_transform =
        transform * Mat4::from_rotation_y((beam.animation_time * 2.25 - 45.0).to_radians());
    render_scroll_geometry_submission(
        meshes,
        opaque_submit,
        EntityModelScrollOwner::BeaconBeamOpaque,
        atlas,
        |mesh, entry, submit| {
            append_beacon_beam_part(
                mesh,
                entry,
                inner_transform,
                beam.beam_start,
                beam_end,
                [
                    (0.0, solid_radius),
                    (solid_radius, 0.0),
                    (-solid_radius, 0.0),
                    (0.0, -solid_radius),
                ],
                submit.tint,
                solid_vv1,
                vv2,
            );
        },
    );

    let glow_radius = beam.glow_radius;
    let glow_vv1 = beam.height as f32 * beam.scale + vv2;
    render_scroll_geometry_submission(
        meshes,
        translucent_submit,
        EntityModelScrollOwner::BeaconBeamTranslucent,
        atlas,
        |mesh, entry, submit| {
            let mut glow_tint = submit.tint;
            glow_tint[3] = 32.0 / 255.0;
            append_beacon_beam_part(
                mesh,
                entry,
                transform,
                beam.beam_start,
                beam_end,
                [
                    (-glow_radius, -glow_radius),
                    (glow_radius, -glow_radius),
                    (-glow_radius, glow_radius),
                    (glow_radius, glow_radius),
                ],
                glow_tint,
                glow_vv1,
                vv2,
            );
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn append_beacon_beam_part(
    mesh: &mut EntityModelScrollMesh,
    entry: EntityModelTextureAtlasEntry,
    transform: Mat4,
    beam_start: i32,
    beam_end: i32,
    corners: [(f32, f32); 4],
    tint: [f32; 4],
    vv1: f32,
    vv2: f32,
) {
    let [(wnx, wnz), (enx, enz), (wsx, wsz), (esx, esz)] = corners;
    append_beacon_beam_quad(
        mesh,
        entry,
        transform,
        beam_start,
        beam_end,
        (wnx, wnz),
        (enx, enz),
        tint,
        vv1,
        vv2,
    );
    append_beacon_beam_quad(
        mesh,
        entry,
        transform,
        beam_start,
        beam_end,
        (esx, esz),
        (wsx, wsz),
        tint,
        vv1,
        vv2,
    );
    append_beacon_beam_quad(
        mesh,
        entry,
        transform,
        beam_start,
        beam_end,
        (enx, enz),
        (esx, esz),
        tint,
        vv1,
        vv2,
    );
    append_beacon_beam_quad(
        mesh,
        entry,
        transform,
        beam_start,
        beam_end,
        (wsx, wsz),
        (wnx, wnz),
        tint,
        vv1,
        vv2,
    );
}

#[allow(clippy::too_many_arguments)]
fn append_beacon_beam_quad(
    mesh: &mut EntityModelScrollMesh,
    entry: EntityModelTextureAtlasEntry,
    transform: Mat4,
    beam_start: i32,
    beam_end: i32,
    first: (f32, f32),
    second: (f32, f32),
    tint: [f32; 4],
    vv1: f32,
    vv2: f32,
) {
    let base =
        u32::try_from(mesh.vertices.len()).expect("end gateway beam vertex count fits in u32");
    for (position, local_uv) in [
        (Vec3::new(first.0, beam_end as f32, first.1), [1.0, vv1]),
        (Vec3::new(first.0, beam_start as f32, first.1), [1.0, vv2]),
        (Vec3::new(second.0, beam_start as f32, second.1), [0.0, vv2]),
        (Vec3::new(second.0, beam_end as f32, second.1), [0.0, vv1]),
    ] {
        append_beacon_beam_vertex(mesh, entry, transform, position, local_uv, tint);
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn append_beacon_beam_vertex(
    mesh: &mut EntityModelScrollMesh,
    entry: EntityModelTextureAtlasEntry,
    transform: Mat4,
    position: Vec3,
    local_uv: [f32; 2],
    tint: [f32; 4],
) {
    let rect = entry.uv;
    let size = [rect.max[0] - rect.min[0], rect.max[1] - rect.min[1]];
    mesh.vertices.push(EntityModelScrollVertex {
        position: transform.transform_point3(position).to_array(),
        local_uv,
        uv_rect_min: rect.min,
        uv_rect_size: size,
        tint,
        light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
        overlay: ENTITY_VERTEX_NO_OVERLAY,
        normal: transform
            .transform_vector3(Vec3::Y)
            .normalize_or_zero()
            .to_array(),
    });
}

fn frac(value: f32) -> f32 {
    value - value.floor()
}

fn emit_dragon_rays_custom_submission(
    meshes: &mut EntityModelTexturedMeshes,
    render_type: EntityModelLayerRenderType,
    transform: Mat4,
    death_time: f32,
) {
    let insertion_index = meshes.take_submission_index();
    let mut mesh = EntityModelMesh::new();
    append_dragon_rays_mesh(&mut mesh, transform, death_time);
    let index_count =
        u32::try_from(mesh.indices.len()).expect("dragon rays index count fits in u32");
    let index_start = match render_type {
        EntityModelLayerRenderType::DragonRays => {
            append_entity_model_mesh(&mut meshes.dragon_rays, mesh)
        }
        EntityModelLayerRenderType::DragonRaysDepth => {
            append_entity_model_mesh(&mut meshes.dragon_rays_depth, mesh)
        }
        _ => unreachable!("dragon rays custom submission render type"),
    };
    if let Some(sort_key) = meshes.geometry_draw_key(transform, 0, insertion_index) {
        meshes.push_geometry_draw_range(render_type, sort_key, index_start, index_count);
    }
}

fn append_dragon_rays_mesh(mesh: &mut EntityModelMesh, transform: Mat4, death_time: f32) {
    let over_drive = if death_time > 0.8 {
        ((death_time - 0.8) / 0.2).min(1.0)
    } else {
        0.0
    };
    let inner_alpha = 1.0 - over_drive;
    let inner_color = [1.0, 1.0, 1.0, inner_alpha];
    // Vanilla `EnderDragonRenderer.java:99,124-131`: `outerColor = 0x00FF00FF` (alpha 0).
    let outer_color = [1.0, 0.0, 1.0, 0.0];
    let ray_count = ((death_time + death_time * death_time) / 2.0 * 60.0).floor() as usize;
    let mut random = LegacyRandomSource::with_seed(DRAGON_RAYS_RANDOM_SEED);
    let mut pose = transform;

    for _ in 0..ray_count {
        let first = Quat::from_euler(
            glam::EulerRot::XYZ,
            random.next_float() * std::f32::consts::TAU,
            random.next_float() * std::f32::consts::TAU,
            random.next_float() * std::f32::consts::TAU,
        );
        let second = Quat::from_euler(
            glam::EulerRot::XYZ,
            random.next_float() * std::f32::consts::TAU,
            random.next_float() * std::f32::consts::TAU,
            random.next_float() * std::f32::consts::TAU + death_time * std::f32::consts::FRAC_PI_2,
        );
        pose *= Mat4::from_quat(first * second);
        let length = random.next_float() * 20.0 + 5.0 + over_drive * 10.0;
        let width = random.next_float() * 2.0 + 1.0 + over_drive * 2.0;
        let origin = Vec3::ZERO;
        let outer_left = Vec3::new(-DRAGON_RAYS_HALF_SQRT_3 * width, length, -0.5 * width);
        let outer_right = Vec3::new(DRAGON_RAYS_HALF_SQRT_3 * width, length, -0.5 * width);
        let outer_bottom = Vec3::new(0.0, length, width);
        append_dragon_ray_triangle(
            mesh,
            pose,
            origin,
            outer_left,
            outer_right,
            inner_color,
            outer_color,
        );
        append_dragon_ray_triangle(
            mesh,
            pose,
            origin,
            outer_right,
            outer_bottom,
            inner_color,
            outer_color,
        );
        append_dragon_ray_triangle(
            mesh,
            pose,
            origin,
            outer_bottom,
            outer_left,
            inner_color,
            outer_color,
        );
    }
}

fn append_dragon_ray_triangle(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    origin: Vec3,
    left: Vec3,
    right: Vec3,
    inner_color: [f32; 4],
    outer_color: [f32; 4],
) {
    let base = u32::try_from(mesh.vertices.len()).expect("dragon rays vertex count fits in u32");
    for (position, color) in [
        (origin, inner_color),
        (left, outer_color),
        (right, outer_color),
    ] {
        mesh.vertices.push(EntityModelVertex {
            position: transform.transform_point3(position).to_array(),
            color,
            light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
            overlay: ENTITY_VERTEX_NO_OVERLAY,
            normal: [0.0, 1.0, 0.0],
        });
    }
    mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
}

fn emit_crystal_beam_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelRenderSubmission,
    atlas: &EntityModelTextureAtlasLayout,
    length: f32,
    age: f32,
) {
    render_scroll_geometry_submission(
        meshes,
        submit,
        EntityModelScrollOwner::EndCrystalBeam,
        atlas,
        |mesh, entry, submit| {
            let rect = entry.uv;
            let size = [rect.max[0] - rect.min[0], rect.max[1] - rect.min[1]];
            let v0 = -age * 0.01;
            let v1 = length / 32.0 - age * 0.01;
            let mut last_sin = 0.0;
            let mut last_cos = 0.75;
            let mut last_u = 0.0;

            for i in 1..=8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                let sin = angle.sin() * 0.75;
                let cos = angle.cos() * 0.75;
                let u = i as f32 / 8.0;
                let base =
                    u32::try_from(mesh.vertices.len()).expect("scroll vertex count fits in u32");
                for (position, local_uv, tint) in [
                    (
                        Vec3::new(last_sin * 0.2, last_cos * 0.2, 0.0),
                        [last_u, v0],
                        [0.0, 0.0, 0.0, 1.0],
                    ),
                    (
                        Vec3::new(last_sin, last_cos, length),
                        [last_u, v1],
                        [1.0, 1.0, 1.0, 1.0],
                    ),
                    (Vec3::new(sin, cos, length), [u, v1], [1.0, 1.0, 1.0, 1.0]),
                    (
                        Vec3::new(sin * 0.2, cos * 0.2, 0.0),
                        [u, v0],
                        [0.0, 0.0, 0.0, 1.0],
                    ),
                ] {
                    let normal = submit
                        .transform
                        .transform_vector3(Vec3::NEG_Y)
                        .normalize_or_zero()
                        .to_array();
                    mesh.vertices.push(EntityModelScrollVertex {
                        position: submit.transform.transform_point3(position).to_array(),
                        local_uv,
                        uv_rect_min: rect.min,
                        uv_rect_size: size,
                        tint,
                        light: submit.light,
                        overlay: submit.overlay,
                        normal,
                    });
                }
                mesh.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base,
                    base + 2,
                    base + 3,
                ]);
                last_sin = sin;
                last_cos = cos;
                last_u = u;
            }
        },
    );
}
