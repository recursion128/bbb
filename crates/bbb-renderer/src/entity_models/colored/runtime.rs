#[cfg(test)]
use std::borrow::Cow;

use glam::{Mat4, Vec3};

use super::super::catalog::{sheep_wool_render_color, *};
use super::super::dispatch::{dispatch_uniform_entity_model, ColoredSink};
use super::super::geometry::*;
use super::super::instances::EntityModelInstance;
use super::super::model::EntityModel;
use super::super::model_layers::*;
use super::mounts::{emit_donkey_model, emit_horse_model, emit_undead_horse_model};
use super::selection::{
    hoglin_model_color, humanoid_model_color, piglin_model_color, quadruped_model_color,
};
use super::transforms::{
    drowned_model_root_transform, end_crystal_model_root_transform, entity_model_root_transform,
    mesh_transformer_scaled_model_root_transform, player_model_root_transform,
    scaled_model_root_transform, squid_model_root_transform, tropical_fish_model_root_transform,
    wind_charge_model_root_transform, HUSK_SCALE,
};

#[cfg(test)]
pub(in crate::entity_models) fn entity_model_mesh(
    instances: &[EntityModelInstance],
) -> EntityModelMesh {
    entity_model_mesh_with_options(instances, false)
}

pub(in crate::entity_models) fn entity_model_colored_runtime_mesh(
    instances: &[EntityModelInstance],
) -> EntityModelMesh {
    entity_model_mesh_with_options(instances, true)
}

fn entity_model_mesh_with_options(
    instances: &[EntityModelInstance],
    skip_texture_backed_entities: bool,
) -> EntityModelMesh {
    let mut mesh = EntityModelMesh::new();
    for instance in instances {
        if instance.render_state.invisible {
            continue;
        }
        let light_start = mesh.vertices.len();
        let handled = {
            let mut sink = ColoredSink {
                mesh: &mut mesh,
                skip_texture_backed: skip_texture_backed_entities,
            };
            dispatch_uniform_entity_model(instance, &mut sink)
        };
        if !handled {
            // Only the bespoke entities remain here — those whose colored and textured paths diverge
            // (recolor, two model trees, family helpers, part visibility, single-pass `render_textured_pass`
            // emits, bespoke hand-walks) and the colored-only nontrivial / placeholder / no-render kinds.
            // The uniform kinds are emitted by `dispatch_uniform_entity_model` above and are unreachable
            // here, so the match ends with `_ => {}`.
            match instance.kind {
                EntityModelKind::Player { skin, .. } => {
                    if !skip_texture_backed_entities {
                        PlayerModel::new(skin.is_slim()).prepare_and_render(
                            &mut mesh,
                            instance,
                            player_model_root_transform(*instance),
                        );
                    }
                }
                EntityModelKind::Humanoid { family, baby } => {
                    emit_humanoid_model(&mut mesh, *instance, family, baby)
                }
                EntityModelKind::WindCharge => {
                    // The wind charge's textured render is the scrolling `breezeWind` overlay; this
                    // colored fallback renders the plain `WindChargeModel` tree when textures are absent.
                    if !skip_texture_backed_entities {
                        WindChargeModel::new().prepare_and_render(
                            &mut mesh,
                            instance,
                            wind_charge_model_root_transform(*instance),
                        );
                    }
                }
                EntityModelKind::EndCrystal => {
                    // The end crystal is texture-backed now; keep the colored fallback mesh for legacy /
                    // missing-atlas callers, but skip it in the runtime colored path.
                    if !skip_texture_backed_entities {
                        emit_end_crystal_model(&mut mesh, *instance);
                    }
                }
                EntityModelKind::NoRender => {
                    // Vanilla `NoopRenderer` entities (area effect cloud, marker, interaction) render no
                    // model, so this arm emits nothing — exact parity with vanilla.
                }
                EntityModelKind::ZombieVariant { family, baby } => {
                    // The husk, drowned, and zombie villager all have wired texture-backed paths now.
                    if !skip_texture_backed_entities {
                        emit_zombie_variant_model(&mut mesh, *instance, family, baby)
                    }
                }
                EntityModelKind::Piglin { family, baby } => {
                    if !skip_texture_backed_entities {
                        emit_piglin_model(&mut mesh, *instance, family, baby)
                    }
                }
                EntityModelKind::Hoglin { family, baby } => {
                    if !skip_texture_backed_entities {
                        emit_hoglin_model(&mut mesh, *instance, family, baby)
                    }
                }
                EntityModelKind::Sheep {
                    baby,
                    sheared,
                    wool_color,
                    jeb,
                    age_ticks,
                } => {
                    if !skip_texture_backed_entities {
                        emit_sheep_model(
                            &mut mesh, *instance, baby, sheared, wool_color, jeb, age_ticks,
                        );
                    }
                }
                EntityModelKind::Horse { baby, .. } => {
                    // The living horse now renders through the textured path (per-coat texture); the
                    // colored emit is the full-mesh fallback only (skipped in the runtime mesh).
                    if !skip_texture_backed_entities {
                        emit_horse_model(&mut mesh, *instance, baby);
                    }
                }
                EntityModelKind::Donkey {
                    family,
                    baby,
                    has_chest,
                } => {
                    // The donkey/mule (adult and baby) now renders through the textured path; the
                    // colored emit is the full-mesh fallback only.
                    if !skip_texture_backed_entities {
                        emit_donkey_model(&mut mesh, *instance, family, baby, has_chest);
                    }
                }
                EntityModelKind::UndeadHorse { family, baby } => {
                    // The skeleton/zombie horse now renders through the textured path; the colored
                    // emit is the full-mesh fallback only (skipped in the texture-backed runtime mesh).
                    if !skip_texture_backed_entities {
                        emit_undead_horse_model(&mut mesh, *instance, family, baby);
                    }
                }
                EntityModelKind::Quadruped { family, baby } => {
                    emit_quadruped_model(&mut mesh, *instance, family, baby)
                }
                EntityModelKind::Squid { glow, baby } => {
                    if !skip_texture_backed_entities {
                        emit_squid_model(&mut mesh, *instance, glow, baby);
                    }
                }
                EntityModelKind::TropicalFish {
                    shape, base_color, ..
                } => {
                    // The colored debug path approximates the textured base body as a solid base-color
                    // box; the `TropicalFishPatternLayer` overlay is a cutout texture (its shape comes
                    // from the texture alpha) and so is only meaningful on the textured path.
                    if !skip_texture_backed_entities {
                        emit_tropical_fish_model(&mut mesh, *instance, shape, base_color);
                    }
                }
                EntityModelKind::Placeholder { bounds, .. } => {
                    emit_placeholder_bounds_model(&mut mesh, *instance, bounds)
                }
                _ => {}
            }
        }
        fill_entity_model_light(&mut mesh, light_start, instance.render_state.shader_light());
        fill_entity_model_overlay(
            &mut mesh,
            light_start,
            instance.render_state.overlay_coords(),
        );
    }
    mesh
}

fn emit_end_crystal_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `EndCrystalModel` is the base slab plus the concentric glass/core stack (the per-part
    // `withScale` baked into the cube dimensions). `setupAnim` hides the base slab
    // (`END_CRYSTAL_PARTS[0]`) when `!showsBottom`, bobs the glass stack by `getY(age)·8` pixels, and
    // counter-spins the nested glass: `outer_glass` by `Ry(age·3°)·TILT`, then `inner_glass` and the
    // core `cube` by `TILT·Ry(age·3°)` (inheriting the outer rotation through the hierarchy). The
    // renderer flattens the glass stack, so the nested spin is hand-walked here off the shared,
    // bobbing `(0, 24, 0)` centre. Emitted at the static `EndCrystalRenderer` transform (`scale(2.0)`
    // + `translate(0, -0.5, 0)`, no living flip).
    let root = end_crystal_model_root_transform(instance);
    if instance.render_state.end_crystal_shows_bottom {
        emit_model_part(mesh, &END_CRYSTAL_PARTS[0], root);
    }

    let age = instance.render_state.age_in_ticks;
    let bob = end_crystal_bob_y(age);
    let (q_outer, q_inner) = end_crystal_glass_quaternions(age);
    // The shared glass centre, bobbing on Y (offset in model-pixels; `part_pose_transform` applies
    // the model-unit scale). All three glass parts sit at this centre with no rotation in the layer.
    let centre = root
        * part_pose_transform(PartPose {
            offset: [0.0, 24.0 + bob, 0.0],
            rotation: [0.0, 0.0, 0.0],
        });
    let outer_t = centre * Mat4::from_quat(q_outer);
    let inner_t = outer_t * Mat4::from_quat(q_inner);
    let core_t = inner_t * Mat4::from_quat(q_inner);
    for cube in END_CRYSTAL_PARTS[1].cubes {
        emit_model_cube(mesh, outer_t, *cube);
    }
    for cube in END_CRYSTAL_PARTS[2].cubes {
        emit_model_cube(mesh, inner_t, *cube);
    }
    for cube in END_CRYSTAL_PARTS[3].cubes {
        emit_model_cube(mesh, core_t, *cube);
    }
}

fn emit_tropical_fish_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    shape: TropicalFishModelShape,
    base_color: EntityDyeColor,
) {
    // Vanilla `TropicalFish{Small,Large}Model.setupAnim` sways only the tail (`yRot`); the
    // swim wiggle and out-of-water flop live in `tropical_fish_model_root_transform`. The
    // kob-style small body and flopper-style large body differ only in geometry. The colored
    // fallback recolors the whole body with the vanilla `getModelTint` =
    // `getBaseColor().getTextureDiffuseColor()` (the pattern overlay is textured-only).
    let in_water = instance.render_state.in_water;
    let root = tropical_fish_model_root_transform(instance, in_water);
    TropicalFishModel::new(shape).prepare_and_render_with_color(
        mesh,
        &instance,
        root,
        base_color.texture_diffuse_color(),
    );
}

fn emit_squid_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    glow: bool,
    baby: bool,
) {
    // Vanilla `SquidModel.setupAnim` only sweeps the eight tentacles by the lerped
    // `tentacleAngle` (`tentacle.xRot = tentacleAngle`); the body is static. The swim
    // body tilt and the `0.5/1.2` translate live in `squid_model_root_transform`. The
    // colored fallback recolors the whole tree with the squid / glow-squid tint.
    let root = squid_model_root_transform(instance, baby);
    let color = if glow { GLOW_SQUID_TEAL } else { SQUID_BLUE };
    SquidModel::new().prepare_and_render_with_color(mesh, &instance, root, color);
}

fn emit_humanoid_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: HumanoidModelFamily,
    baby: bool,
) {
    let color = humanoid_model_color(family);
    let transform = scaled_model_root_transform(instance, if baby { 0.5 } else { 1.0 });
    emit_model_cube(
        mesh,
        transform * part_pose_transform(PART_POSE_ZERO),
        ModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            color,
        },
    );
    emit_model_cube(
        mesh,
        transform * part_pose_transform(PART_POSE_ZERO),
        ModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            color,
        },
    );

    let limb_width = if family == HumanoidModelFamily::Skeleton {
        2.0
    } else {
        4.0
    };
    let arm_half = limb_width / 2.0;
    for (x, min_x) in [(-5.0, -arm_half), (5.0, -arm_half)] {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: [x, 2.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                }),
            ModelCubeDesc {
                min: [min_x, -2.0, -arm_half],
                size: [limb_width, 12.0, limb_width],
                color,
            },
        );
    }
    for (x, min_x) in [(-1.9, -arm_half), (1.9, -arm_half)] {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: [x, 12.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                }),
            ModelCubeDesc {
                min: [min_x, 0.0, -arm_half],
                size: [limb_width, 12.0, limb_width],
                color,
            },
        );
    }

    if matches!(
        family,
        HumanoidModelFamily::Villager | HumanoidModelFamily::Illager
    ) {
        emit_model_cube(
            mesh,
            transform * part_pose_transform(PART_POSE_ZERO),
            ModelCubeDesc {
                min: [-2.0, -2.0, -6.0],
                size: [4.0, 4.0, 2.0],
                color,
            },
        );
    }
}

fn emit_zombie_variant_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: ZombieVariantModelFamily,
    baby: bool,
) {
    // The unified `ZombieVariantModel` tree drives both render paths; `setup_anim` runs the shared
    // `ZombieModel.setupAnim` (head look + leg swing + held-out arms). The colored fallback recolors
    // the whole model with the family color (the textured path uses the family texture instead).
    let color = zombie_variant_color(family);
    let transform = zombie_variant_root_transform(instance, family, baby);
    ZombieVariantModel::new(family, baby)
        .prepare_and_render_with_color(mesh, &instance, transform, color);
}

/// The colored-fallback recolor for a zombie variant: the husk's tan, the drowned's blue, or the
/// zombie villager's robe. The textured path uses the family texture instead of this override.
fn zombie_variant_color(family: ZombieVariantModelFamily) -> [f32; 4] {
    match family {
        ZombieVariantModelFamily::Husk => HUSK_TAN,
        ZombieVariantModelFamily::Drowned => DROWNED_BLUE,
        ZombieVariantModelFamily::ZombieVillager => ZOMBIE_VILLAGER_ROBE,
    }
}

/// The model→world transform for a zombie variant. Only the adult husk is scaled (vanilla
/// `huskScale` 1.0625, a `MeshTransformer.scaling` baked by `HuskRenderer`); the drowned adds its
/// swim `setupRotations` pitch, and the zombie villager renders at the unscaled humanoid root.
pub(in crate::entity_models) fn zombie_variant_root_transform(
    instance: EntityModelInstance,
    family: ZombieVariantModelFamily,
    baby: bool,
) -> Mat4 {
    if family == ZombieVariantModelFamily::Husk && !baby {
        mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE)
    } else if family == ZombieVariantModelFamily::Drowned {
        drowned_model_root_transform(instance)
    } else {
        entity_model_root_transform(instance)
    }
}

fn emit_piglin_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: PiglinModelFamily,
    baby: bool,
) {
    // The unified `PiglinModel` tree drives both render paths; `setup_anim` runs the head look, the
    // humanoid walk (legs only for the zombified piglin), and the ear flap. The colored fallback
    // recolors the whole model with the family skin; the textured path uses the family texture.
    PiglinModel::new(family, baby).prepare_and_render_with_color(
        mesh,
        &instance,
        entity_model_root_transform(instance),
        piglin_model_color(family),
    );
}

fn emit_hoglin_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: HoglinModelFamily,
    baby: bool,
) {
    // The unified `HoglinModel` tree drives both render paths; `setup_anim` runs the yaw-only head
    // look, ear sway, and four-leg swing. The colored fallback recolors the whole model with the
    // family color (hoglin red / zoglin gray); the textured path uses the family texture instead.
    HoglinModel::new(baby).prepare_and_render_with_color(
        mesh,
        &instance,
        entity_model_root_transform(instance),
        hoglin_model_color(family),
    );
}

/// Applies the vanilla `QuadrupedModel.setupAnim` leg swing
/// ([`quadruped_leg_swing_pose`]) to a colored layer's four leg parts at
/// `leg_indices`. Borrows the static parts unchanged at rest
/// (`walkAnimationSpeed == 0`). The quadruped models now swing their legs through their
/// `setup_anim` directly, so this `Cow`-slice variant is retained only as the reference the
/// leg-phase unit test asserts against.
#[cfg(test)]
pub(in crate::entity_models) fn quadruped_limb_swing_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    leg_indices: [usize; 4],
    limb_swing: f32,
    limb_swing_amount: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if limb_swing_at_rest(limb_swing_amount) {
        return parts;
    }
    let mut owned = parts.into_owned();
    for index in leg_indices {
        if let Some(leg) = owned.get_mut(index) {
            leg.pose = quadruped_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
        }
    }
    Cow::Owned(owned)
}

/// Vanilla `HumanoidModel` leg part indices: the head, body, and the two arms
/// occupy the lower slots, then the right and left legs. Every humanoid body layer
/// here lists the legs last at `[4, 5]` (the baby layers swap head/body to `1`/`0`
/// but keep arms at `2`/`3` and legs at `4`/`5`). Now that the humanoid models swing their
/// legs through `setup_anim` directly, this index pair is retained only for the unit tests.
#[cfg(test)]
pub(in crate::entity_models) const HUMANOID_LEG_PART_INDICES: [usize; 2] = [4, 5];

/// Applies the vanilla `HumanoidModel.setupAnim` leg swing
/// ([`humanoid_leg_swing_pose`]) to a colored layer's two leg parts at
/// `leg_indices`. Borrows the static parts unchanged at rest
/// (`walkAnimationSpeed == 0`). The arm swing is left to each humanoid subclass,
/// which overrides the arms (e.g. the zombie held-out pose), so only the legs —
/// which subclasses inherit unchanged from `HumanoidModel` — are swung here. The humanoid models
/// now swing their legs through the name-based humanoid leg-swing animator directly, so this
/// `Cow`-slice variant is retained only as the reference the leg-phase unit tests assert against.
#[cfg(test)]
pub(in crate::entity_models) fn humanoid_limb_swing_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    leg_indices: [usize; 2],
    limb_swing: f32,
    limb_swing_amount: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if limb_swing_at_rest(limb_swing_amount) {
        return parts;
    }
    let mut owned = parts.into_owned();
    for index in leg_indices {
        if let Some(leg) = owned.get_mut(index) {
            leg.pose = humanoid_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
        }
    }
    Cow::Owned(owned)
}

/// Vanilla `HumanoidModel` arm part indices: the head and body occupy `0`/`1`, then
/// the right and left arms at `[2, 3]` (every humanoid layer, adult or baby). Now that the
/// humanoid models pose their arms through `setup_anim` directly, this index pair is retained only
/// for the unit tests.
#[cfg(test)]
pub(in crate::entity_models) const HUMANOID_ARM_PART_INDICES: [usize; 2] = [2, 3];

/// Applies the vanilla `HumanoidModel.setupAnim` arm animation to a colored layer's two
/// arm parts at `arm_indices`: the walk swing ([`humanoid_arm_swing_pose`], only while
/// the limbs move) plus the always-on `ageInTicks` idle bob ([`humanoid_arm_bob_pose`]).
/// Because the idle bob advances every frame, the arms are always re-posed (the parts are
/// never borrowed unchanged). The humanoid models now pose their arms through
/// [`apply_humanoid_walk`] directly, so this `Cow`-slice variant is retained only as the
/// reference the arm-phase unit test asserts against.
#[cfg(test)]
pub(in crate::entity_models) fn humanoid_arm_swing_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    arm_indices: [usize; 2],
    limb_swing: f32,
    limb_swing_amount: f32,
    age_in_ticks: f32,
) -> Cow<'_, [ModelPartDesc]> {
    let swing = !limb_swing_at_rest(limb_swing_amount);
    let mut owned = parts.into_owned();
    for index in arm_indices {
        if let Some(arm) = owned.get_mut(index) {
            let mut pose = arm.pose;
            if swing {
                pose = humanoid_arm_swing_pose(pose, limb_swing, limb_swing_amount);
            }
            arm.pose = humanoid_arm_bob_pose(pose, age_in_ticks);
        }
    }
    Cow::Owned(owned)
}

fn emit_sheep_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
    jeb: bool,
    age_ticks: f32,
) {
    // The unified `SheepModel` (body) and `SheepFurModel` (wool) trees drive both render paths; both
    // run the shared `SheepModel.setupAnim` (leg swing + eat-grass head pose). The colored fallback
    // renders the body with baked colors, optionally recolors the body undercoat (non-white adult),
    // then renders the wool tinted (unless sheared).
    let transform = entity_model_root_transform(instance);
    let wool_layer_color = sheep_wool_render_color(wool_color, jeb, age_ticks);
    let mut body = SheepModel::new(baby);
    body.prepare(&instance);
    body.root().render_colored(mesh, transform);
    if !baby && (jeb || wool_color != SheepWoolColor::White) {
        body.root()
            .render_colored_with_color(mesh, transform, wool_layer_color);
    }
    if !sheared {
        let mut fur = SheepFurModel::new(baby);
        fur.prepare(&instance);
        fur.root()
            .render_colored_with_color(mesh, transform, wool_layer_color);
    }
}

fn emit_quadruped_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: QuadrupedModelFamily,
    baby: bool,
) {
    if family == QuadrupedModelFamily::Pig {
        PigModel::new(PigModelVariant::Temperate, baby).prepare_and_render(
            mesh,
            &instance,
            entity_model_root_transform(instance),
        );
        return;
    }

    let color = quadruped_model_color(family);
    let scale = if baby { 0.5 } else { 1.0 };
    let transform = scaled_model_root_transform(instance, scale);
    let (head, body, leg_size, head_offset, body_offset, leg_x) = match family {
        QuadrupedModelFamily::Pig => (
            ([-4.0, -4.0, -8.0], [8.0, 8.0, 8.0]),
            ([-5.0, -10.0, -7.0], [10.0, 16.0, 8.0]),
            6.0,
            [0.0, 12.0, -6.0],
            [0.0, 11.0, 2.0],
            3.0,
        ),
        QuadrupedModelFamily::Cow => (
            ([-4.0, -4.0, -6.0], [8.0, 8.0, 6.0]),
            ([-6.0, -10.0, -7.0], [12.0, 18.0, 10.0]),
            12.0,
            [0.0, 4.0, -8.0],
            [0.0, 5.0, 2.0],
            4.0,
        ),
        QuadrupedModelFamily::Sheep => (
            ([-3.0, -4.0, -6.0], [6.0, 6.0, 8.0]),
            ([-4.0, -10.0, -7.0], [8.0, 16.0, 6.0]),
            12.0,
            [0.0, 6.0, -8.0],
            [0.0, 5.0, 2.0],
            3.0,
        ),
        QuadrupedModelFamily::Horse => (
            ([-3.0, -4.0, -8.0], [6.0, 5.0, 7.0]),
            ([-5.0, -8.0, -9.0], [10.0, 10.0, 22.0]),
            12.0,
            [0.0, 7.0, -10.0],
            [0.0, 11.0, 2.0],
            4.0,
        ),
        QuadrupedModelFamily::Wolf => (
            ([-3.0, -3.0, -4.0], [6.0, 6.0, 6.0]),
            ([-4.0, -2.0, -3.0], [8.0, 6.0, 9.0]),
            8.0,
            [0.0, 13.5, -7.0],
            [0.0, 14.0, 2.0],
            2.5,
        ),
    };

    emit_model_cube(
        mesh,
        transform
            * part_pose_transform(PartPose {
                offset: head_offset,
                rotation: [0.0, 0.0, 0.0],
            }),
        ModelCubeDesc {
            min: head.0,
            size: head.1,
            color,
        },
    );
    emit_model_cube(
        mesh,
        transform
            * part_pose_transform(PartPose {
                offset: body_offset,
                rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            }),
        ModelCubeDesc {
            min: body.0,
            size: body.1,
            color,
        },
    );
    // Vanilla QuadrupedModel.setupAnim leg swing: each leg's xRot is
    // `cos(walkAnimationPos * 0.6662 [+ π]) * 1.4 * walkAnimationSpeed`, with the
    // hind-left / front-right legs a half-cycle (π) out of phase with the
    // hind-right / front-left pair. The legs are emitted in the vanilla order
    // [right hind, left hind, right front, left front].
    let leg_x_rots = quadruped_leg_x_rotations(instance);
    for ((x, z), leg_x_rot) in [(-leg_x, 7.0), (leg_x, 7.0), (-leg_x, -5.0), (leg_x, -5.0)]
        .into_iter()
        .zip(leg_x_rots)
    {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: [x, 24.0 - leg_size, z],
                    rotation: [leg_x_rot, 0.0, 0.0],
                }),
            ModelCubeDesc {
                min: [-2.0, 0.0, -2.0],
                size: [4.0, leg_size, 4.0],
                color,
            },
        );
    }
}

/// Vanilla `QuadrupedModel.setupAnim` leg `xRot` values in the model part order
/// `[right hind, left hind, right front, left front]`:
/// `cos(walkAnimationPos * 0.6662 [+ π]) * 1.4 * walkAnimationSpeed`. The
/// hind-left and front-right legs are a half-cycle out of phase. Returns all
/// zeros for a standing entity (`walkAnimationSpeed == 0`).
pub(in crate::entity_models) fn quadruped_leg_x_rotations(
    instance: EntityModelInstance,
) -> [f32; 4] {
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let phase = limb_swing * 0.6662;
    let in_phase = phase.cos() * 1.4 * limb_swing_amount;
    let out_of_phase = (phase + std::f32::consts::PI).cos() * 1.4 * limb_swing_amount;
    [in_phase, out_of_phase, out_of_phase, in_phase]
}

fn emit_placeholder_bounds_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    bounds: EntityModelBounds,
) {
    let width = bounds.width.max(0.0625);
    let height = bounds.height.max(0.0625);
    let depth = bounds.depth.max(0.0625);
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y((180.0 - instance.render_state.body_rot).to_radians());
    emit_model_cube_world_units(
        mesh,
        transform,
        [-width * 0.5, 0.0, -depth * 0.5],
        [width, height, depth],
        PLACEHOLDER_COLOR,
    );
}
