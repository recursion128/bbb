use std::borrow::Cow;

use glam::{Mat4, Vec3};

use super::super::catalog::{sheep_wool_render_color, *};
use super::super::geometry::*;
use super::super::instances::EntityModelInstance;
use super::super::keyframe::*;
use super::super::model_layers::*;
use super::armor_stand::emit_armor_stand_model;
use super::mounts::{
    emit_camel_model, emit_donkey_model, emit_horse_model, emit_llama_model,
    emit_undead_horse_model,
};
use super::selection::{
    chicken_model_parts, cow_model_parts, hoglin_model_color, humanoid_model_color,
    pig_model_parts, piglin_model_color, quadruped_model_color,
};
use super::transforms::{
    arrow_model_root_transform, boat_model_root_transform, cave_spider_model_root_transform,
    cod_model_root_transform, end_crystal_model_root_transform, entity_model_root_transform,
    evoker_fangs_model_root_transform, ghast_model_root_transform,
    happy_ghast_model_root_transform, leash_knot_model_root_transform,
    magma_cube_model_root_transform, mesh_transformer_scaled_model_root_transform,
    phantom_model_root_transform, player_model_root_transform, polar_bear_model_root_transform,
    pufferfish_model_root_transform, salmon_model_root_transform, scaled_model_root_transform,
    slime_model_root_transform, squid_model_root_transform, trident_model_root_transform,
    tropical_fish_model_root_transform, villager_adult_model_root_transform,
    wither_skeleton_model_root_transform, GIANT_SCALE, HUSK_SCALE,
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
        let light_start = mesh.vertices.len();
        match instance.kind {
            EntityModelKind::Chicken { variant, baby } => {
                if !skip_texture_backed_entities {
                    // Vanilla `ChickenModel.setupAnim` swings the two legs with the
                    // `HumanoidModel` phase `cos(pos * 0.6662 [+ π]) * 1.4 * speed` (right
                    // leg in phase, left leg out). The chicken has no head look. The wing
                    // flap is driven by the untracked `flap`/`flapSpeed` state (deferred).
                    let parts = humanoid_limb_swing_parts(
                        Cow::Borrowed(chicken_model_parts(variant, baby)),
                        chicken_leg_part_indices(baby),
                        instance.render_state.walk_animation_pos,
                        instance.render_state.walk_animation_speed,
                    );
                    emit_model_parts(&mut mesh, &parts, entity_model_root_transform(*instance));
                }
            }
            EntityModelKind::Pig { variant, baby } => {
                if !skip_texture_backed_entities {
                    emit_pig_model(&mut mesh, *instance, variant, baby);
                }
            }
            EntityModelKind::Player { slim, .. } => {
                if !skip_texture_backed_entities {
                    emit_player_model(&mut mesh, *instance, slim);
                }
            }
            EntityModelKind::Humanoid { family, baby } => {
                emit_humanoid_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::ArmorStand {
                small,
                show_arms,
                show_base_plate,
                pose,
            } => {
                if !skip_texture_backed_entities {
                    emit_armor_stand_model(
                        &mut mesh,
                        *instance,
                        small,
                        show_arms,
                        show_base_plate,
                        pose,
                    );
                }
            }
            EntityModelKind::Slime { size } => {
                if !skip_texture_backed_entities {
                    emit_slime_model(&mut mesh, *instance, size);
                }
            }
            EntityModelKind::MagmaCube { size } => {
                if !skip_texture_backed_entities {
                    emit_magma_cube_model(&mut mesh, *instance, size);
                }
            }
            EntityModelKind::Ghast => {
                if !skip_texture_backed_entities {
                    emit_ghast_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::HappyGhast => {
                if !skip_texture_backed_entities {
                    emit_happy_ghast_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Blaze => {
                if !skip_texture_backed_entities {
                    emit_blaze_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Endermite => {
                if !skip_texture_backed_entities {
                    emit_endermite_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Silverfish => {
                if !skip_texture_backed_entities {
                    emit_silverfish_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Vex => {
                if !skip_texture_backed_entities {
                    emit_vex_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Allay => {
                if !skip_texture_backed_entities {
                    emit_allay_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Strider { baby } => {
                if !skip_texture_backed_entities {
                    emit_strider_model(&mut mesh, *instance, baby);
                }
            }
            EntityModelKind::Turtle { baby } => {
                if !skip_texture_backed_entities {
                    emit_turtle_model(&mut mesh, *instance, baby);
                }
            }
            EntityModelKind::Bat => {
                if !skip_texture_backed_entities {
                    emit_bat_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Bee { baby } => {
                if !skip_texture_backed_entities {
                    emit_bee_model(&mut mesh, *instance, baby);
                }
            }
            EntityModelKind::Breeze => {
                if !skip_texture_backed_entities {
                    emit_breeze_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Dolphin { baby } => {
                if !skip_texture_backed_entities {
                    emit_dolphin_model(&mut mesh, *instance, baby);
                }
            }
            EntityModelKind::Guardian { elder } => {
                // Colored-only so far (no texture-backed guardian yet), so this arm is always
                // emitted rather than gated behind `skip_texture_backed_entities`.
                emit_guardian_model(&mut mesh, *instance, elder);
            }
            EntityModelKind::Frog => {
                // Colored-only so far (no texture-backed frog yet), so this arm is always emitted.
                emit_frog_model(&mut mesh, *instance);
            }
            EntityModelKind::Creaking => {
                // Colored-only so far (no texture-backed creaking yet), so this arm always emits.
                emit_creaking_model(&mut mesh, *instance);
            }
            EntityModelKind::Sniffer => {
                // Colored-only so far (no texture-backed sniffer yet), so this arm always emits.
                emit_sniffer_model(&mut mesh, *instance);
            }
            EntityModelKind::Warden => {
                // Colored-only so far (no texture-backed warden yet), so this arm always emits.
                emit_warden_model(&mut mesh, *instance);
            }
            EntityModelKind::Armadillo { baby } => {
                // Colored-only so far (no texture-backed armadillo yet), so this arm always emits.
                emit_armadillo_model(&mut mesh, *instance, baby);
            }
            EntityModelKind::Axolotl { baby } => {
                // Colored-only so far (no texture-backed axolotl yet), so this arm always emits.
                emit_axolotl_model(&mut mesh, *instance, baby);
            }
            EntityModelKind::Tadpole => {
                // Colored-only so far (no texture-backed tadpole yet), so this arm always emits.
                emit_tadpole_model(&mut mesh, *instance);
            }
            EntityModelKind::Parrot => {
                // Colored-only so far (no texture-backed parrot yet), so this arm always emits.
                emit_parrot_model(&mut mesh, *instance);
            }
            EntityModelKind::Shulker => {
                // Colored-only so far (no texture-backed shulker yet), so this arm always emits.
                emit_shulker_model(&mut mesh, *instance);
            }
            EntityModelKind::Wither => {
                // Colored-only so far (no texture-backed wither yet), so this arm always emits.
                emit_wither_model(&mut mesh, *instance);
            }
            EntityModelKind::Giant => {
                // Colored-only so far (no texture-backed giant yet), so this arm always emits.
                emit_giant_model(&mut mesh, *instance);
            }
            EntityModelKind::EndCrystal => {
                // Colored-only so far (no texture-backed end crystal yet), so this arm always emits.
                emit_end_crystal_model(&mut mesh, *instance);
            }
            EntityModelKind::EvokerFangs => {
                // Colored-only so far (no texture-backed evoker fangs yet), so this arm always emits.
                emit_evoker_fangs_model(&mut mesh, *instance);
            }
            EntityModelKind::LeashKnot => {
                // Colored-only so far (no texture-backed leash knot yet), so this arm always emits.
                emit_leash_knot_model(&mut mesh, *instance);
            }
            EntityModelKind::Arrow => {
                // Colored-only so far (no texture-backed arrow yet), so this arm always emits.
                emit_arrow_model(&mut mesh, *instance);
            }
            EntityModelKind::Trident => {
                // Colored-only so far (no texture-backed trident yet), so this arm always emits.
                emit_trident_model(&mut mesh, *instance);
            }
            EntityModelKind::Phantom { size } => {
                if !skip_texture_backed_entities {
                    emit_phantom_model(&mut mesh, *instance, size);
                }
            }
            EntityModelKind::Pufferfish { puff_state } => {
                if !skip_texture_backed_entities {
                    emit_pufferfish_model(&mut mesh, *instance, puff_state);
                }
            }
            EntityModelKind::Zombie { baby } => {
                if !skip_texture_backed_entities {
                    emit_zombie_model(&mut mesh, *instance, baby);
                }
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
            EntityModelKind::Ravager => {
                if !skip_texture_backed_entities {
                    emit_ravager_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Skeleton => {
                if !skip_texture_backed_entities {
                    emit_skeleton_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::SkeletonVariant { family } => {
                if !skip_texture_backed_entities {
                    emit_skeleton_variant_model(&mut mesh, *instance, family)
                }
            }
            EntityModelKind::Cow { variant, baby } => {
                if !skip_texture_backed_entities {
                    emit_cow_model(&mut mesh, *instance, variant, baby);
                }
            }
            EntityModelKind::Sheep {
                baby,
                sheared,
                wool_color,
                invisible,
                jeb,
                age_ticks,
            } => {
                if !skip_texture_backed_entities {
                    emit_sheep_model(
                        &mut mesh, *instance, baby, sheared, wool_color, invisible, jeb, age_ticks,
                    );
                }
            }
            EntityModelKind::Villager { baby } => {
                if !skip_texture_backed_entities {
                    emit_villager_model(&mut mesh, *instance, baby);
                }
            }
            EntityModelKind::WanderingTrader => {
                if !skip_texture_backed_entities {
                    emit_wandering_trader_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Wolf { baby, angry, .. } => {
                if !skip_texture_backed_entities {
                    emit_wolf_model(&mut mesh, *instance, baby, angry);
                }
            }
            EntityModelKind::Horse { baby } => emit_horse_model(&mut mesh, *instance, baby),
            EntityModelKind::Donkey {
                family,
                baby,
                has_chest,
            } => emit_donkey_model(&mut mesh, *instance, family, baby, has_chest),
            EntityModelKind::UndeadHorse { family, baby } => {
                emit_undead_horse_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Camel { family, baby } => {
                if !skip_texture_backed_entities {
                    emit_camel_model(&mut mesh, *instance, family, baby);
                }
            }
            EntityModelKind::Llama {
                family,
                variant,
                baby,
                has_chest,
            } => {
                if !skip_texture_backed_entities {
                    emit_llama_model(&mut mesh, *instance, family, variant, baby, has_chest);
                }
            }
            EntityModelKind::Goat {
                baby,
                left_horn,
                right_horn,
            } => {
                if !skip_texture_backed_entities {
                    emit_goat_model(&mut mesh, *instance, baby, left_horn, right_horn);
                }
            }
            EntityModelKind::PolarBear { baby } => {
                if !skip_texture_backed_entities {
                    emit_polar_bear_model(&mut mesh, *instance, baby);
                }
            }
            EntityModelKind::Quadruped { family, baby } => {
                emit_quadruped_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Creeper => {
                if !skip_texture_backed_entities {
                    emit_creeper_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Spider => {
                if !skip_texture_backed_entities {
                    emit_spider_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::CaveSpider => {
                if !skip_texture_backed_entities {
                    emit_cave_spider_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Enderman => {
                if !skip_texture_backed_entities {
                    emit_enderman_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::IronGolem => {
                if !skip_texture_backed_entities {
                    emit_iron_golem_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::SnowGolem => {
                if !skip_texture_backed_entities {
                    emit_snow_golem_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Witch => {
                if !skip_texture_backed_entities {
                    emit_witch_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Squid { glow, baby } => {
                if !skip_texture_backed_entities {
                    emit_squid_model(&mut mesh, *instance, glow, baby);
                }
            }
            EntityModelKind::Cod => {
                if !skip_texture_backed_entities {
                    emit_cod_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Salmon { size } => {
                if !skip_texture_backed_entities {
                    emit_salmon_model(&mut mesh, *instance, size);
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
            EntityModelKind::Illager { family } => {
                if !skip_texture_backed_entities {
                    emit_illager_model(&mut mesh, *instance, family)
                }
            }
            EntityModelKind::Minecart => {
                if !skip_texture_backed_entities {
                    emit_minecart_model(&mut mesh, *instance);
                }
            }
            EntityModelKind::Boat { family, chest } => {
                if !skip_texture_backed_entities {
                    emit_boat_model(&mut mesh, *instance, family, chest);
                }
            }
            EntityModelKind::Placeholder { bounds, .. } => {
                emit_placeholder_bounds_model(&mut mesh, *instance, bounds)
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

fn emit_slime_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, size: i32) {
    emit_model_parts(
        mesh,
        &SLIME_PARTS,
        slime_model_root_transform(instance, size),
    );
}

fn emit_magma_cube_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, size: i32) {
    emit_model_parts(
        mesh,
        &MAGMA_CUBE_PARTS,
        magma_cube_model_root_transform(instance, size),
    );
}

fn emit_ghast_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `GhastModel.setupAnim` waves each of the nine tentacles by `ageInTicks`
    // (`tentacle.xRot = 0.2 * sin(ageInTicks * 0.3 + i) + 0.4`, never at rest), so the
    // tentacles are always re-posed. The body is part 0; tentacles `i` are parts 1..=9.
    let age_in_ticks = instance.render_state.age_in_ticks;
    let mut parts = GHAST_PARTS.to_vec();
    for (tentacle, part) in parts.iter_mut().skip(1).enumerate() {
        part.pose.rotation[0] = ghast_tentacle_x_rot(tentacle, age_in_ticks);
    }
    emit_model_parts(mesh, &parts, ghast_model_root_transform(instance));
}

fn emit_happy_ghast_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `HappyGhastModel.setupAnim` reuses `GhastModel.animateTentacles` verbatim
    // (`tentacle.xRot = 0.2 * sin(ageInTicks * 0.3 + i) + 0.4`, never at rest), so the nine
    // tentacles always wave. The body is part 0; tentacles `i` are parts 1..=9. The body-item
    // squeeze (`xScale/yScale/zScale = 0.9375` when a harness is equipped) is deferred with the
    // harness equipment layer, so an unharnessed happy ghast renders at full scale.
    let age_in_ticks = instance.render_state.age_in_ticks;
    let mut parts = HAPPY_GHAST_PARTS.to_vec();
    for (tentacle, part) in parts.iter_mut().skip(1).enumerate() {
        part.pose.rotation[0] = ghast_tentacle_x_rot(tentacle, age_in_ticks);
    }
    emit_model_parts(mesh, &parts, happy_ghast_model_root_transform(instance));
}

fn emit_blaze_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `BlazeModel.setupAnim` re-positions all twelve rods from `ageInTicks` every
    // frame (`blaze_rod_offset`), orbiting in three rings; the head (part 0) follows the
    // plain `head_look_pose`. The rods are parts 1..=12. There is no walk swing — a blaze
    // floats — and no synced render state beyond the head look, so the model is fully
    // animated from `ageInTicks` plus the look angles.
    let age_in_ticks = instance.render_state.age_in_ticks;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let mut parts = BLAZE_PARTS.to_vec();
    if !head_look_at_rest(head_yaw, head_pitch) {
        parts[0].pose = head_look_pose(parts[0].pose, head_yaw, head_pitch);
    }
    for index in 0..BLAZE_ROD_COUNT {
        parts[index + 1].pose.offset = blaze_rod_offset(index, age_in_ticks);
    }
    emit_model_parts(mesh, &parts, entity_model_root_transform(instance));
}

fn emit_endermite_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `EndermiteModel.setupAnim` wiggles all four chitin segments from `ageInTicks`
    // every frame (`endermite_segment_pose` sets each segment's `x`/`yRot`); there is no head
    // look or walk swing. The endermite has no MeshTransformer scaling (unit model root).
    let age_in_ticks = instance.render_state.age_in_ticks;
    let mut parts = ENDERMITE_PARTS.to_vec();
    for (index, part) in parts.iter_mut().enumerate() {
        part.pose = endermite_segment_pose(part.pose, index, age_in_ticks);
    }
    emit_model_parts(mesh, &parts, entity_model_root_transform(instance));
}

fn emit_silverfish_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `SilverfishModel.setupAnim` wiggles all seven body segments from `ageInTicks`
    // every frame (`silverfish_segment_pose`), then the three overlay layers copy segments
    // 2/4/1 (`silverfish_layer_pose` per `SILVERFISH_LAYER_RULES`). There is no head look or
    // walk swing, and no MeshTransformer scaling (unit model root).
    let age_in_ticks = instance.render_state.age_in_ticks;
    let mut parts = SILVERFISH_PARTS.to_vec();
    for index in 0..SILVERFISH_SEGMENT_COUNT {
        parts[index].pose = silverfish_segment_pose(parts[index].pose, index, age_in_ticks);
    }
    for (layer, &(source, copy_x)) in SILVERFISH_LAYER_RULES.iter().enumerate() {
        let source_pose = parts[source].pose;
        let part = &mut parts[SILVERFISH_SEGMENT_COUNT + layer];
        part.pose = silverfish_layer_pose(part.pose, source_pose, copy_x);
    }
    emit_model_parts(mesh, &parts, entity_model_root_transform(instance));
}

fn emit_vex_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `VexModel.setupAnim` (non-charging idle): the head tracks the look angles, the
    // arms hold `±π/5` with a small `ageInTicks` z-bob, the body tilts `π/20`, and the wings
    // flap on `ageInTicks`. The arms and wings hang under the body, so the body tilt carries
    // them; the hierarchy is walked by hand. The charging pose and held-item arms are
    // deferred entity-side state. Vex uses the standard `LivingEntityRenderer.setupRotations`.
    let age = instance.render_state.age_in_ticks;
    let root = entity_model_root_transform(instance) * part_pose_transform(VEX_ROOT_POSE);

    // Head (child of root) tracks the look yaw/pitch.
    let head_pose = PartPose {
        offset: VEX_HEAD_POSE.offset,
        rotation: [
            instance.render_state.head_pitch.to_radians(),
            instance.render_state.head_yaw.to_radians(),
            0.0,
        ],
    };
    emit_model_cubes_at_pose(mesh, root, head_pose, &VEX_HEAD);

    // Body (child of root) holds the idle tilt and carries the arms and wings.
    let body_pose = PartPose {
        offset: VEX_BODY_POSE.offset,
        rotation: [VEX_BODY_X_ROT, 0.0, 0.0],
    };
    let body_t = root * part_pose_transform(body_pose);
    emit_model_cubes_at_pose(mesh, root, body_pose, &VEX_BODY);

    let bob = vex_moving_arm_z_bob(age);
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: VEX_RIGHT_ARM_POSE.offset,
            rotation: [0.0, 0.0, VEX_ARM_REST_Z_ROT + bob],
        },
        &VEX_RIGHT_ARM,
    );
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: VEX_LEFT_ARM_POSE.offset,
            rotation: [0.0, 0.0, -(VEX_ARM_REST_Z_ROT + bob)],
        },
        &VEX_LEFT_ARM,
    );

    let left_wing_yrot = vex_left_wing_y_rot(age);
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: VEX_LEFT_WING_POSE.offset,
            rotation: [VEX_WING_X_ROT, left_wing_yrot, -VEX_WING_Z_ROT],
        },
        &VEX_WING,
    );
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: VEX_RIGHT_WING_POSE.offset,
            rotation: [VEX_WING_X_ROT, -left_wing_yrot, VEX_WING_Z_ROT],
        },
        &VEX_WING,
    );
}

fn emit_allay_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `AllayModel.setupAnim` (non-dancing, non-holding idle / flying pose): the head
    // tracks the look angles, the wings flap on `ageInTicks` + the walk animation, the arms
    // bob, the body tilts toward the flying pose, and the whole root bobs vertically while
    // idle. The arms and wings hang under the body, so the body tilt carries them; the
    // hierarchy is walked by hand. The dance pose (`isDancing`/`isSpinning`) and held-item
    // arms are deferred entity-side state. Allay uses `LivingEntityRenderer.setupRotations`.
    let age = instance.render_state.age_in_ticks;
    let walk_pos = instance.render_state.walk_animation_pos;
    let walk_speed = instance.render_state.walk_animation_speed;

    // The root bobs vertically while idle (height fades out as the allay starts flying).
    let root_pose = PartPose {
        offset: [0.0, allay_root_y(age, walk_speed), 0.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let root = entity_model_root_transform(instance) * part_pose_transform(root_pose);

    // Head (child of root) tracks the look yaw/pitch.
    let head_pose = PartPose {
        offset: ALLAY_HEAD_POSE.offset,
        rotation: [
            instance.render_state.head_pitch.to_radians(),
            instance.render_state.head_yaw.to_radians(),
            0.0,
        ],
    };
    emit_model_cubes_at_pose(mesh, root, head_pose, &ALLAY_HEAD);

    // Body (child of root) tilts toward the flying pose and carries the arms and wings.
    let body_pose = PartPose {
        offset: ALLAY_BODY_POSE.offset,
        rotation: [allay_body_x_rot(walk_speed), 0.0, 0.0],
    };
    let body_t = root * part_pose_transform(body_pose);
    emit_model_cubes_at_pose(mesh, root, body_pose, &ALLAY_BODY);

    let arm_bob = allay_arm_idle_bob_amount(age, walk_speed);
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: ALLAY_RIGHT_ARM_POSE.offset,
            rotation: [0.0, 0.0, arm_bob],
        },
        &ALLAY_RIGHT_ARM,
    );
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: ALLAY_LEFT_ARM_POSE.offset,
            rotation: [0.0, 0.0, -arm_bob],
        },
        &ALLAY_LEFT_ARM,
    );

    let wing_x_rot = allay_wing_rest_x_rot(walk_speed);
    let flap = allay_wing_flap_amount(age, walk_pos, walk_speed);
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: ALLAY_RIGHT_WING_POSE.offset,
            rotation: [wing_x_rot, -ALLAY_WING_Y_ROT_BASE + flap, 0.0],
        },
        &ALLAY_WING,
    );
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: ALLAY_LEFT_WING_POSE.offset,
            rotation: [wing_x_rot, ALLAY_WING_Y_ROT_BASE - flap, 0.0],
        },
        &ALLAY_WING,
    );
}

fn emit_strider_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    if baby {
        emit_strider_baby_model(mesh, instance);
    } else {
        emit_strider_adult_model(mesh, instance);
    }
}

fn emit_strider_adult_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `StriderModel.setupAnim` + `AdultStriderModel.customAnimations`: the body sways
    // and bobs, the legs swing/roll/lift in opposition, and the six bristles flow with the
    // walk plus an idle `ageInTicks` ripple. Legs are children of the root; the bristles hang
    // under the body, so the body sway carries them — the hierarchy is walked by hand. The
    // ridden pose (`isRidden` zeroing the body look), the saddle layer, and the cold texture
    // are deferred entity-side state. Strider uses `LivingEntityRenderer.setupRotations`.
    let age = instance.render_state.age_in_ticks;
    let pos = instance.render_state.walk_animation_pos;
    let speed = strider_animation_speed(instance.render_state.walk_animation_speed);
    let root = entity_model_root_transform(instance);

    // Legs (children of root) swing on `xRot`, roll on `zRot`, and lift on `y`.
    emit_model_cubes_at_pose(
        mesh,
        root,
        PartPose {
            offset: [
                STRIDER_RIGHT_LEG_X,
                strider_leg_y(STRIDER_LEG_BASE_Y, pos, speed, true),
                0.0,
            ],
            rotation: [
                strider_leg_x_rot(pos, speed, true),
                0.0,
                strider_leg_z_rot(pos, speed, true),
            ],
        },
        &STRIDER_RIGHT_LEG,
    );
    emit_model_cubes_at_pose(
        mesh,
        root,
        PartPose {
            offset: [
                STRIDER_LEFT_LEG_X,
                strider_leg_y(STRIDER_LEG_BASE_Y, pos, speed, false),
                0.0,
            ],
            rotation: [
                strider_leg_x_rot(pos, speed, false),
                0.0,
                strider_leg_z_rot(pos, speed, false),
            ],
        },
        &STRIDER_LEFT_LEG,
    );

    // Body (child of root) tracks the look, sways on `zRot`, and bobs on `y`.
    let body_pose = PartPose {
        offset: [
            0.0,
            strider_body_y(STRIDER_BODY_BASE_Y, 2.0, pos, speed),
            0.0,
        ],
        rotation: [
            instance.render_state.head_pitch.to_radians(),
            instance.render_state.head_yaw.to_radians(),
            strider_body_z_rot(pos, speed),
        ],
    };
    let body_t = root * part_pose_transform(body_pose);
    emit_model_cubes_at_pose(mesh, root, body_pose, &STRIDER_BODY);

    // The six bristles (children of body) hold their rest roll plus the flow ripple.
    let flow = strider_bristle_flow(pos, speed);
    let top = strider_bristle_top_flow(flow, age);
    let middle = strider_bristle_middle_flow(flow, age);
    let bottom = strider_bristle_bottom_flow(flow, age);
    for (pose_const, cubes, add) in [
        (STRIDER_RIGHT_TOP_BRISTLE_POSE, &STRIDER_RIGHT_BRISTLE, top),
        (
            STRIDER_RIGHT_MIDDLE_BRISTLE_POSE,
            &STRIDER_RIGHT_BRISTLE,
            middle,
        ),
        (
            STRIDER_RIGHT_BOTTOM_BRISTLE_POSE,
            &STRIDER_RIGHT_BRISTLE,
            bottom,
        ),
        (STRIDER_LEFT_TOP_BRISTLE_POSE, &STRIDER_LEFT_BRISTLE, top),
        (
            STRIDER_LEFT_MIDDLE_BRISTLE_POSE,
            &STRIDER_LEFT_BRISTLE,
            middle,
        ),
        (
            STRIDER_LEFT_BOTTOM_BRISTLE_POSE,
            &STRIDER_LEFT_BRISTLE,
            bottom,
        ),
    ] {
        let mut pose = pose_const;
        pose.rotation[2] += add;
        emit_model_cubes_at_pose(mesh, body_t, pose, cubes);
    }
}

fn emit_strider_baby_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // The baby strider shares `StriderModel.setupAnim` (body sway + leg swing/roll) but has its
    // own smaller geometry and `BabyStriderModel.customAnimations`: a shallower body bob, the
    // leg lift, and three bristles that flap on `xRot` (no rest roll).
    let age = instance.render_state.age_in_ticks;
    let pos = instance.render_state.walk_animation_pos;
    let speed = strider_animation_speed(instance.render_state.walk_animation_speed);
    let root = entity_model_root_transform(instance);

    emit_model_cubes_at_pose(
        mesh,
        root,
        PartPose {
            offset: [
                STRIDER_BABY_RIGHT_LEG_X,
                strider_leg_y(STRIDER_BABY_LEG_BASE_Y, pos, speed, true),
                0.0,
            ],
            rotation: [
                strider_leg_x_rot(pos, speed, true),
                0.0,
                strider_leg_z_rot(pos, speed, true),
            ],
        },
        &STRIDER_BABY_RIGHT_LEG,
    );
    emit_model_cubes_at_pose(
        mesh,
        root,
        PartPose {
            offset: [
                STRIDER_BABY_LEFT_LEG_X,
                strider_leg_y(STRIDER_BABY_LEG_BASE_Y, pos, speed, false),
                0.0,
            ],
            rotation: [
                strider_leg_x_rot(pos, speed, false),
                0.0,
                strider_leg_z_rot(pos, speed, false),
            ],
        },
        &STRIDER_BABY_LEFT_LEG,
    );

    let body_pose = PartPose {
        offset: [
            0.0,
            strider_body_y(STRIDER_BABY_BODY_BASE_Y, 1.0, pos, speed),
            0.0,
        ],
        rotation: [
            instance.render_state.head_pitch.to_radians(),
            instance.render_state.head_yaw.to_radians(),
            strider_body_z_rot(pos, speed),
        ],
    };
    let body_t = root * part_pose_transform(body_pose);
    emit_model_cubes_at_pose(mesh, root, body_pose, &STRIDER_BABY_BODY);

    let flow = strider_bristle_flow(pos, speed);
    for (pose_const, add) in [
        (
            STRIDER_BABY_FRONT_BRISTLE_POSE,
            strider_bristle_top_flow(flow, age),
        ),
        (
            STRIDER_BABY_MIDDLE_BRISTLE_POSE,
            strider_bristle_middle_flow(flow, age),
        ),
        (
            STRIDER_BABY_BACK_BRISTLE_POSE,
            strider_bristle_bottom_flow(flow, age),
        ),
    ] {
        let mut pose = pose_const;
        pose.rotation[0] += add;
        emit_model_cubes_at_pose(mesh, body_t, pose, &STRIDER_BABY_BRISTLE);
    }
}

fn emit_turtle_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    // Vanilla `QuadrupedModel.setupAnim` (head look + the diagonal leg swing) plus
    // `TurtleModel.setupAnim`: on land the turtle adds a `yRot` walk swing to the legs, in
    // water it paddles (hind `xRot` / front `zRot`). `isOnLand = !isInWater && onGround` is
    // projected from the real water + ground state. The legs and head/body are direct children
    // of the root, so each is posed from the root transform. The egg-laying leg amplitude
    // (`isLayingEgg`) and the `egg_belly` shell (`hasEgg`) are deferred entity-side state.
    // Turtle uses `LivingEntityRenderer.setupRotations`.
    let pos = instance.render_state.walk_animation_pos;
    let speed = instance.render_state.walk_animation_speed;
    let on_land = !instance.render_state.in_water && instance.render_state.on_ground;
    let root = entity_model_root_transform(instance);
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();

    let (head_cubes, head_pose, body_cubes, body_pose, legs): (_, _, _, _, [_; 4]) = if baby {
        (
            &TURTLE_BABY_HEAD[..],
            TURTLE_BABY_HEAD_POSE,
            &TURTLE_BABY_BODY[..],
            TURTLE_BABY_BODY_POSE,
            [
                (
                    &TURTLE_BABY_RIGHT_HIND_LEG[..],
                    TURTLE_BABY_RIGHT_HIND_LEG_POSE,
                    false,
                    true,
                ),
                (
                    &TURTLE_BABY_LEFT_HIND_LEG[..],
                    TURTLE_BABY_LEFT_HIND_LEG_POSE,
                    false,
                    false,
                ),
                (
                    &TURTLE_BABY_RIGHT_FRONT_LEG[..],
                    TURTLE_BABY_RIGHT_FRONT_LEG_POSE,
                    true,
                    true,
                ),
                (
                    &TURTLE_BABY_LEFT_FRONT_LEG[..],
                    TURTLE_BABY_LEFT_FRONT_LEG_POSE,
                    true,
                    false,
                ),
            ],
        )
    } else {
        (
            &TURTLE_HEAD[..],
            TURTLE_HEAD_POSE,
            &TURTLE_BODY[..],
            TURTLE_BODY_POSE,
            [
                (
                    &TURTLE_RIGHT_HIND_LEG[..],
                    TURTLE_RIGHT_HIND_LEG_POSE,
                    false,
                    true,
                ),
                (
                    &TURTLE_LEFT_HIND_LEG[..],
                    TURTLE_LEFT_HIND_LEG_POSE,
                    false,
                    false,
                ),
                (
                    &TURTLE_RIGHT_FRONT_LEG[..],
                    TURTLE_RIGHT_FRONT_LEG_POSE,
                    true,
                    true,
                ),
                (
                    &TURTLE_LEFT_FRONT_LEG[..],
                    TURTLE_LEFT_FRONT_LEG_POSE,
                    true,
                    false,
                ),
            ],
        )
    };

    // Head tracks the look; the body holds its fixed shell tilt.
    let head_pose = PartPose {
        offset: head_pose.offset,
        rotation: [head_pitch, head_yaw, 0.0],
    };
    emit_model_cubes_at_pose(mesh, root, head_pose, head_cubes);
    emit_model_cubes_at_pose(mesh, root, body_pose, body_cubes);

    for (cubes, leg_pose, front, right) in legs {
        emit_model_cubes_at_pose(
            mesh,
            root,
            PartPose {
                offset: leg_pose.offset,
                rotation: turtle_leg_rotation(pos, speed, on_land, front, right),
            },
            cubes,
        );
    }
}

/// Combine a bind pose with the keyframe position/rotation offsets (vanilla `ModelPart::offsetPos`
/// / `offsetRotation` add to the bind pose). Shared by the keyframe-animated entities.
fn keyframe_animated_pose(bind: PartPose, position: [f32; 3], rotation: [f32; 3]) -> PartPose {
    PartPose {
        offset: [
            bind.offset[0] + position[0],
            bind.offset[1] + position[1],
            bind.offset[2] + position[2],
        ],
        rotation: [
            bind.rotation[0] + rotation[0],
            bind.rotation[1] + rotation[1],
            bind.rotation[2] + rotation[2],
        ],
    }
}

fn emit_bat_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `BatModel.setupAnim` applies the keyframe `BatAnimation.BAT_FLYING` wing flap /
    // body bob, driven by the entity's `flyAnimationState`. The state's exact start tick is
    // deferred entity-side data, so the looping animation is sampled from `age_in_ticks` (the
    // phase offset is imperceptible for a continuous flap). The resting pose (`isResting`,
    // `BAT_RESTING`, the head look) is deferred entity-side state. The wings/tips and feet hang
    // under the body and the ears under the head, so the hierarchy is walked by hand. Bat uses
    // `LivingEntityRenderer.setupRotations`.
    let seconds = keyframe_elapsed_seconds(&BAT_FLYING, instance.render_state.age_in_ticks * 0.05);
    let sample = |bone: &str| sample_bone_offsets(&BAT_FLYING, bone, seconds, 1.0);
    let root = entity_model_root_transform(instance);

    // Head (root child) carries the two ears at their bind poses.
    let (head_pos, head_rot) = sample("head");
    let head_pose = keyframe_animated_pose(BAT_HEAD_POSE, head_pos, head_rot);
    let head_t = root * part_pose_transform(head_pose);
    emit_model_cubes_at_pose(mesh, root, head_pose, &BAT_HEAD);
    emit_model_cubes_at_pose(mesh, head_t, BAT_RIGHT_EAR_POSE, &BAT_RIGHT_EAR);
    emit_model_cubes_at_pose(mesh, head_t, BAT_LEFT_EAR_POSE, &BAT_LEFT_EAR);

    // Body (root child) carries the wings and feet.
    let (body_pos, body_rot) = sample("body");
    let body_pose = keyframe_animated_pose(BAT_BODY_POSE, body_pos, body_rot);
    let body_t = root * part_pose_transform(body_pose);
    emit_model_cubes_at_pose(mesh, root, body_pose, &BAT_BODY);

    let (_, feet_rot) = sample("feet");
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        keyframe_animated_pose(BAT_FEET_POSE, [0.0; 3], feet_rot),
        &BAT_FEET,
    );

    // Each wing (body child) carries its tip.
    let (_, right_wing_rot) = sample("right_wing");
    let right_wing_pose = keyframe_animated_pose(BAT_RIGHT_WING_POSE, [0.0; 3], right_wing_rot);
    let right_wing_t = body_t * part_pose_transform(right_wing_pose);
    emit_model_cubes_at_pose(mesh, body_t, right_wing_pose, &BAT_RIGHT_WING);
    let (_, right_tip_rot) = sample("right_wing_tip");
    emit_model_cubes_at_pose(
        mesh,
        right_wing_t,
        keyframe_animated_pose(BAT_RIGHT_WING_TIP_POSE, [0.0; 3], right_tip_rot),
        &BAT_RIGHT_WING_TIP,
    );

    let (_, left_wing_rot) = sample("left_wing");
    let left_wing_pose = keyframe_animated_pose(BAT_LEFT_WING_POSE, [0.0; 3], left_wing_rot);
    let left_wing_t = body_t * part_pose_transform(left_wing_pose);
    emit_model_cubes_at_pose(mesh, body_t, left_wing_pose, &BAT_LEFT_WING);
    let (_, left_tip_rot) = sample("left_wing_tip");
    emit_model_cubes_at_pose(
        mesh,
        left_wing_t,
        keyframe_animated_pose(BAT_LEFT_WING_TIP_POSE, [0.0; 3], left_tip_rot),
        &BAT_LEFT_WING_TIP,
    );
}

fn emit_bee_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    // Vanilla `BeeModel.setupAnim`: while airborne (`!isOnGround`) the wings flap on `ageInTicks`
    // and the non-angry `bobUpAndDown` rocks the bone pivot, front/back legs (and, on adults, the
    // antennae), with all three legs first set to `π/4` so the middle leg holds that angle. On the
    // ground the model rests at its bind pose. The anger pose (`isAngry`), the rolled-up fall pose
    // (`rollAmount`) and the stinger-loss visibility (`hasStinger`) are deferred entity-side state.
    // The body (carrying the stinger and antennae), the wings, and the legs hang under the `bone`
    // pivot, so the hierarchy is walked by hand. Bee uses `LivingEntityRenderer.setupRotations`.
    let age = instance.render_state.age_in_ticks;
    let flying = !instance.render_state.on_ground;
    let root = entity_model_root_transform(instance);

    // Bone pivot (root child): the airborne bob rocks it forward and lifts/drops it.
    let bone_bind = if baby {
        BEE_BABY_BONE_POSE
    } else {
        BEE_BONE_POSE
    };
    let bone_pose = if flying {
        PartPose {
            offset: [
                bone_bind.offset[0],
                bone_bind.offset[1] + bee_bone_y_delta(age),
                bone_bind.offset[2],
            ],
            rotation: [bee_bone_x_rot(age), 0.0, 0.0],
        }
    } else {
        bone_bind
    };
    let bone_t = root * part_pose_transform(bone_pose);
    if baby {
        emit_model_cubes_at_pose(mesh, root, bone_pose, &BEE_BABY_BONE);
    }

    // Body (bone child) carries the stinger and, on adults, the two antennae.
    let body_pose = if baby {
        BEE_BABY_BODY_POSE
    } else {
        BEE_BODY_POSE
    };
    let body_t = bone_t * part_pose_transform(body_pose);
    emit_model_cubes_at_pose(
        mesh,
        bone_t,
        body_pose,
        if baby { &BEE_BABY_BODY } else { &BEE_BODY },
    );
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        if baby {
            BEE_BABY_STINGER_POSE
        } else {
            BEE_STINGER_POSE
        },
        if baby {
            &BEE_BABY_STINGER
        } else {
            &BEE_STINGER
        },
    );
    if !baby {
        let antenna_x_rot = if flying { bee_antenna_x_rot(age) } else { 0.0 };
        emit_model_cubes_at_pose(
            mesh,
            body_t,
            PartPose {
                offset: BEE_LEFT_ANTENNA_POSE.offset,
                rotation: [antenna_x_rot, 0.0, 0.0],
            },
            &BEE_LEFT_ANTENNA,
        );
        emit_model_cubes_at_pose(
            mesh,
            body_t,
            PartPose {
                offset: BEE_RIGHT_ANTENNA_POSE.offset,
                rotation: [antenna_x_rot, 0.0, 0.0],
            },
            &BEE_RIGHT_ANTENNA,
        );
    }

    // Wings (bone children): the flap overrides the bind yaw to 0 and drives `zRot`, mirrored on
    // the left, while the bind pitch (0 on adults, `0.2182` on babies) is preserved.
    let (right_wing_pose, left_wing_pose, right_wing, left_wing): (_, _, &[ModelCubeDesc], _) =
        if baby {
            (
                BEE_BABY_RIGHT_WING_POSE,
                BEE_BABY_LEFT_WING_POSE,
                &BEE_BABY_RIGHT_WING,
                &BEE_BABY_LEFT_WING,
            )
        } else {
            (
                BEE_RIGHT_WING_POSE,
                BEE_LEFT_WING_POSE,
                &BEE_RIGHT_WING,
                &BEE_LEFT_WING,
            )
        };
    let wing_z_rot = bee_wing_z_rot(age);
    emit_model_cubes_at_pose(
        mesh,
        bone_t,
        if flying {
            PartPose {
                offset: right_wing_pose.offset,
                rotation: [right_wing_pose.rotation[0], 0.0, wing_z_rot],
            }
        } else {
            right_wing_pose
        },
        right_wing,
    );
    emit_model_cubes_at_pose(
        mesh,
        bone_t,
        if flying {
            PartPose {
                offset: left_wing_pose.offset,
                rotation: [left_wing_pose.rotation[0], 0.0, -wing_z_rot],
            }
        } else {
            left_wing_pose
        },
        left_wing,
    );

    // Legs (bone children): airborne, the front/back pair bob while the middle leg holds `π/4`.
    let (front_x, mid_x, back_x) = if flying {
        (
            bee_front_leg_x_rot(age),
            BEE_MID_LEG_FLYING_X_ROT,
            bee_back_leg_x_rot(age),
        )
    } else {
        (0.0, 0.0, 0.0)
    };
    let (front_pose, mid_pose, back_pose, front_cubes, mid_cubes, back_cubes): (
        _,
        _,
        _,
        &[ModelCubeDesc],
        &[ModelCubeDesc],
        &[ModelCubeDesc],
    ) = if baby {
        (
            BEE_BABY_FRONT_LEGS_POSE,
            BEE_BABY_MIDDLE_LEGS_POSE,
            BEE_BABY_BACK_LEGS_POSE,
            &BEE_BABY_FRONT_LEGS,
            &BEE_BABY_MIDDLE_LEGS,
            &BEE_BABY_BACK_LEGS,
        )
    } else {
        (
            BEE_FRONT_LEGS_POSE,
            BEE_MIDDLE_LEGS_POSE,
            BEE_BACK_LEGS_POSE,
            &BEE_FRONT_LEGS,
            &BEE_MIDDLE_LEGS,
            &BEE_BACK_LEGS,
        )
    };
    emit_model_cubes_at_pose(
        mesh,
        bone_t,
        PartPose {
            offset: front_pose.offset,
            rotation: [front_x, 0.0, 0.0],
        },
        front_cubes,
    );
    emit_model_cubes_at_pose(
        mesh,
        bone_t,
        PartPose {
            offset: mid_pose.offset,
            rotation: [mid_x, 0.0, 0.0],
        },
        mid_cubes,
    );
    emit_model_cubes_at_pose(
        mesh,
        bone_t,
        PartPose {
            offset: back_pose.offset,
            rotation: [back_x, 0.0, 0.0],
        },
        back_cubes,
    );
}

fn emit_breeze_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `BreezeModel.setupAnim` applies the looping `BreezeAnimation.IDLE` to the base body.
    // The idle `AnimationState` runs continuously, so the looping definition is sampled from
    // `age_in_ticks` (the exact start tick is deferred entity-side data, imperceptible for a
    // continuous idle). The translucent wind layer, the emissive eyes, and the
    // shoot/slide/inhale/jump action animations are deferred entity-side state. The head and the
    // three rods hang under the `body` pivot, so the hierarchy is walked by hand. Breeze uses
    // `LivingEntityRenderer.setupRotations`.
    let seconds = keyframe_elapsed_seconds(&BREEZE_IDLE, instance.render_state.age_in_ticks * 0.05);
    let sample = |bone: &str| sample_bone_offsets(&BREEZE_IDLE, bone, seconds, 1.0);
    let root = entity_model_root_transform(instance);

    // Body pivot (root child): no IDLE channel, so it holds its identity bind pose.
    let body_t = root * part_pose_transform(BREEZE_BODY_POSE);

    // Head (body child): the IDLE position bob (CATMULLROM).
    let (head_pos, _) = sample("head");
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        keyframe_animated_pose(BREEZE_HEAD_POSE, head_pos, [0.0; 3]),
        &BREEZE_HEAD,
    );

    // Rods pivot (body child): the IDLE yaw spin plus the position bob, carrying the three rods at
    // their fixed bind poses.
    let (rods_pos, rods_rot) = sample("rods");
    let rods_t =
        body_t * part_pose_transform(keyframe_animated_pose(BREEZE_RODS_POSE, rods_pos, rods_rot));
    emit_model_cubes_at_pose(mesh, rods_t, BREEZE_ROD_1_POSE, &BREEZE_ROD);
    emit_model_cubes_at_pose(mesh, rods_t, BREEZE_ROD_2_POSE, &BREEZE_ROD);
    emit_model_cubes_at_pose(mesh, rods_t, BREEZE_ROD_3_POSE, &BREEZE_ROD);
}

/// The bind pose of the dolphin `body` part with the `DolphinModel.setupAnim` rotations applied:
/// the body steers by the look pitch/yaw, and while moving it adds the swim tilt. Returns the
/// animated body pose plus the tail and tail-fin pitches (which also depend on the move state).
fn dolphin_body_pose(
    instance: EntityModelInstance,
    moving: bool,
) -> (PartPose, [f32; 3], [f32; 3]) {
    let age = instance.render_state.age_in_ticks;
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();
    let wave = dolphin_wave(age);
    let body_x_rot = head_pitch + if moving { -0.05 - 0.05 * wave } else { 0.0 };
    let body_pose = PartPose {
        offset: DOLPHIN_BODY_POSE.offset,
        rotation: [body_x_rot, head_yaw, 0.0],
    };
    // Moving overrides the tail's bind pitch with the wave; the tail fin is `0` at rest.
    let tail_rotation = [
        if moving {
            -0.1 * wave
        } else {
            DOLPHIN_TAIL_BIND_X_ROT
        },
        0.0,
        0.0,
    ];
    let tail_fin_rotation = [if moving { -0.2 * wave } else { 0.0 }, 0.0, 0.0];
    (body_pose, tail_rotation, tail_fin_rotation)
}

fn emit_dolphin_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    // Vanilla `DolphinModel.setupAnim` steers the `body` part by the look pitch/yaw and, while
    // moving, adds the swim body tilt and the tail / tail-fin wave (`cos(ageInTicks · 0.3)`). The
    // baby uses the `MeshTransformer.scaling(0.5)` body layer, composed innermost. The held-item
    // carry layer is deferred entity-side state. Dolphin uses `LivingEntityRenderer.setupRotations`.
    let moving = instance.render_state.is_moving;
    let root = mesh_transformer_scaled_model_root_transform(instance, if baby { 0.5 } else { 1.0 });
    let (body_pose, tail_rotation, tail_fin_rotation) = dolphin_body_pose(instance, moving);

    // Body (root child) carries the fins, the tail chain, and the head chain.
    let body_t = root * part_pose_transform(body_pose);
    emit_model_cubes_at_pose(mesh, body_t, PART_POSE_ZERO, &DOLPHIN_BODY);
    emit_model_cubes_at_pose(mesh, body_t, DOLPHIN_BACK_FIN_POSE, &DOLPHIN_BACK_FIN);
    emit_model_cubes_at_pose(mesh, body_t, DOLPHIN_LEFT_FIN_POSE, &DOLPHIN_SIDE_FIN);
    emit_model_cubes_at_pose(mesh, body_t, DOLPHIN_RIGHT_FIN_POSE, &DOLPHIN_SIDE_FIN);

    // Tail (body child) carries the tail fin; both pitch with the swim wave while moving.
    let tail_pose = PartPose {
        offset: DOLPHIN_TAIL_POSE.offset,
        rotation: tail_rotation,
    };
    let tail_t = body_t * part_pose_transform(tail_pose);
    emit_model_cubes_at_pose(mesh, body_t, tail_pose, &DOLPHIN_TAIL);
    emit_model_cubes_at_pose(
        mesh,
        tail_t,
        PartPose {
            offset: DOLPHIN_TAIL_FIN_POSE.offset,
            rotation: tail_fin_rotation,
        },
        &DOLPHIN_TAIL_FIN,
    );

    // Head (body child) carries the nose.
    let head_t = body_t * part_pose_transform(DOLPHIN_HEAD_POSE);
    emit_model_cubes_at_pose(mesh, body_t, DOLPHIN_HEAD_POSE, &DOLPHIN_HEAD);
    emit_model_cubes_at_pose(mesh, head_t, DOLPHIN_NOSE_POSE, &DOLPHIN_NOSE);
}

fn emit_guardian_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, elder: bool) {
    // Vanilla `GuardianModel` hangs the whole model off one `head` part (`PartPose.ZERO`): the
    // body shell, twelve spikes, the eye, and the nested three-segment tail. The elder guardian
    // is the same mesh scaled 2.35× by `ELDER_GUARDIAN_SCALE` (a `MeshTransformer`, composed at
    // the root). The `setupAnim` head look, spike age pulse + `spikesAnimation` withdrawal, eye
    // tracking, tail sway, and attack beam are deferred, so the model renders at its rest pose.
    let scale = if elder { GUARDIAN_ELDER_SCALE } else { 1.0 };
    let head_t = mesh_transformer_scaled_model_root_transform(instance, scale);

    emit_model_cubes_at_pose(mesh, head_t, PART_POSE_ZERO, &GUARDIAN_HEAD);
    for i in 0..GUARDIAN_SPIKE_X.len() {
        emit_model_cubes_at_pose(mesh, head_t, guardian_spike_bind_pose(i), &GUARDIAN_SPIKE);
    }
    emit_model_cubes_at_pose(mesh, head_t, GUARDIAN_EYE_POSE, &GUARDIAN_EYE_CUBE);

    // Tail: tail0 (`PartPose.ZERO`) → tail1 → tail2.
    emit_model_cubes_at_pose(mesh, head_t, PART_POSE_ZERO, &GUARDIAN_TAIL0);
    let tail1_t = head_t * part_pose_transform(GUARDIAN_TAIL1_POSE);
    emit_model_cubes_at_pose(mesh, head_t, GUARDIAN_TAIL1_POSE, &GUARDIAN_TAIL1);
    emit_model_cubes_at_pose(mesh, tail1_t, GUARDIAN_TAIL2_POSE, &GUARDIAN_TAIL2);
}

fn emit_frog_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `FrogModel` is a static nested hierarchy at rest (`root` → body/legs, body → head
    // /tongue/arms). All of `FrogModel.setupAnim`'s keyframe animations are deferred, so the
    // bind-pose part tree is emitted directly. Frogs use `LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    emit_model_parts(mesh, &FROG_PARTS, root);
}

fn emit_creaking_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `CreakingModel` is a static nested hierarchy at rest (`root` → upper_body/legs,
    // upper_body → head/body/arms). All of `CreakingModel.setupAnim` (head look, walk, attack,
    // invulnerable, death) is deferred, so the bind-pose part tree is emitted directly. Creaking
    // uses `LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    emit_model_parts(mesh, &CREAKING_PARTS, root);
}

fn emit_sniffer_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `SnifferModel` is a static nested hierarchy at rest (`bone` → body/legs, body →
    // head → ears/nose/beak). All of `SnifferModel.setupAnim` (head look, search/walk, dig,
    // long-sniff, stand-up, happy, scenting) is deferred, so the bind-pose part tree is emitted
    // directly. Sniffer uses `LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    emit_model_parts(mesh, &SNIFFER_PARTS, root);
}

fn emit_warden_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `WardenModel` is a static nested hierarchy at rest (`bone` → body/legs, body →
    // ribcages/head/arms, head → tendrils). All of `WardenModel.setupAnim` (head look, walk,
    // idle wobble, tendril sway, and the action keyframe animations) is deferred, so the
    // bind-pose part tree is emitted directly. Warden uses `LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    emit_model_parts(mesh, &WARDEN_PARTS, root);
}

fn emit_armadillo_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    // Vanilla `AdultArmadilloModel`/`BabyArmadilloModel` are static nested hierarchies at rest
    // (root → body/legs, body → tail/head, head → ears). All of `ArmadilloModel.setupAnim` (the
    // clamped head look, `applyWalk`, and the roll-out / roll-up / peek keyframes) plus the
    // `isHidingInShell` shell-ball swap are deferred, so the bind-pose part tree is emitted
    // directly. The baby flag (synced `AgeableMob.DATA_BABY_ID`) selects the baby body layer, as
    // in the vanilla `AgeableMobRenderer`. Armadillo uses `LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    let parts: &[ModelPartDesc] = if baby {
        &BABY_ARMADILLO_PARTS
    } else {
        &ADULT_ARMADILLO_PARTS
    };
    emit_model_parts(mesh, parts, root);
}

fn emit_axolotl_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    // Vanilla `AdultAxolotlModel`/`BabyAxolotlModel` are static nested hierarchies at rest. All of
    // the adult procedural sways, the baby keyframe animations, the play-dead pose, and the
    // mirror-leg copy are deferred, so the bind-pose part tree is emitted directly. The baby flag
    // (synced `AgeableMob.DATA_BABY_ID`) selects the baby body layer, as in the vanilla
    // `AgeableMobRenderer`. Axolotl uses `AgeableMobRenderer`/`LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    let parts: &[ModelPartDesc] = if baby {
        &BABY_AXOLOTL_PARTS
    } else {
        &ADULT_AXOLOTL_PARTS
    };
    emit_model_parts(mesh, parts, root);
}

fn emit_tadpole_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `TadpoleModel` is two static sibling parts (body box, tail fin) at rest. The only
    // animation — the tail yaw sway — is deferred, so the bind-pose part tree is emitted directly.
    // Tadpole uses a plain `MobRenderer`/`LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    emit_model_parts(mesh, &TADPOLE_PARTS, root);
}

fn emit_parrot_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `ParrotModel` is seven static sibling parts (body, tail, wings, head with its beak /
    // crest children, legs) at their baked rest poses. All of `ParrotModel.setupAnim` (the head
    // look, the per-pose `prepare` offsets, the leg/tail walk swing, the wing flap, the flap bob,
    // and the PARTY dance) is deferred, so the STANDING bind-pose part tree is emitted directly.
    // Parrot uses a plain `MobRenderer`/`LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    emit_model_parts(mesh, &PARROT_PARTS, root);
}

fn emit_shulker_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `ShulkerModel` is three static sibling parts (lid, base, head) at rest; the closed
    // pose equals the bind pose. The peek open/close, the head look, and the
    // `ShulkerRenderer.setupRotations` attach-face rotation / body-yaw inversion are deferred, so
    // the bind-pose part tree is emitted directly at the floor rest pose.
    let root = entity_model_root_transform(instance);
    emit_model_parts(mesh, &SHULKER_PARTS, root);
}

fn emit_wither_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `WitherBossModel` is six static sibling parts (shoulders, ribcage, tail, the three
    // heads) at their baked rest poses. The procedural ribcage/tail breathing sway, the head look,
    // and the invulnerable-shimmer overlay layer are deferred, so the bind-pose part tree is
    // emitted directly. Wither uses a plain `MobRenderer`/`LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    emit_model_parts(mesh, &WITHER_PARTS, root);
}

fn emit_giant_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `GiantZombieModel` is the standard `HumanoidModel` (zombie) mesh, baked through
    // `humanoidBodyLayer.apply(MeshTransformer.scaling(6.0))` — i.e. the adult zombie body layer
    // scaled 6×, exactly the husk's `MeshTransformer` pattern but with the giant's 6.0 factor and no
    // baby variant. The head look and limb swing match the zombie (the giant extracts the same
    // `ZombieRenderState`); the `HumanoidArmorLayer`, the `ItemInHandLayer`, and the zombie texture
    // are deferred.
    let parts = humanoid_limb_swing_parts(
        zombie_colored_head_look_parts(&ADULT_ZOMBIE_PARTS, instance, false),
        HUMANOID_LEG_PART_INDICES,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts(
        mesh,
        &parts,
        mesh_transformer_scaled_model_root_transform(instance, GIANT_SCALE),
    );
}

fn emit_end_crystal_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `EndCrystalModel` is the base slab plus the concentric glass/core stack (the per-part
    // `withScale` baked into the cube dimensions). The diagonal spin, the vertical bob, and the
    // `showsBottom` toggle are deferred, so the bind-pose part tree is emitted at the static
    // `EndCrystalRenderer` transform (`scale(2.0)` + `translate(0, -0.5, 0)`, no living flip).
    let root = end_crystal_model_root_transform(instance);
    emit_model_parts(mesh, &END_CRYSTAL_PARTS, root);
}

fn emit_evoker_fangs_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `EvokerFangsModel` is the base block parenting the two jaws, whose bind rotations are
    // the closed-jaw `biteProgress = 0` rest. The bite open/close, the base drop, and the emerge
    // scale are deferred, so the bind-pose part tree is emitted at the `EvokerFangsRenderer`
    // transform (`Ry(90 - yRot)` plus the standard flip and `-1.501` y-offset).
    let root = evoker_fangs_model_root_transform(instance);
    emit_model_parts(mesh, &EVOKER_FANGS_PARTS, root);
}

fn emit_leash_knot_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `LeashKnotModel` is a single static knot box with no `setupAnim`, so the bind-pose
    // part is emitted directly at the `LeashKnotRenderer` flip-only transform.
    let root = leash_knot_model_root_transform(instance);
    emit_model_parts(mesh, &LEASH_KNOT_PARTS, root);
}

fn emit_arrow_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `ArrowModel` is three static planes (the arrowhead plus the two crossed fletching
    // planes). The impact-shake wobble is deferred, so the bind-pose part tree is emitted at the
    // `ArrowRenderer` flight-oriented transform (`Ry(yRot - 90) · Rz(xRot) · scale(0.9)`).
    let root = arrow_model_root_transform(instance);
    emit_model_parts(mesh, &ARROW_PARTS, root);
}

fn emit_trident_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `TridentModel` is the pole parenting the crossguard and three spikes, with no
    // animation, so the bind-pose part tree is emitted directly at the `ThrownTridentRenderer`
    // flight-oriented transform (`Ry(yRot - 90) · Rz(xRot + 90)`).
    let root = trident_model_root_transform(instance);
    emit_model_parts(mesh, &TRIDENT_PARTS, root);
}

fn emit_phantom_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, size: i32) {
    // Vanilla `PhantomModel.setupAnim` flaps the nested wing/tail chains from `flapTime`
    // (`id*3 + ageInTicks`) every frame, while the body and head hold their rest tilt. The
    // hierarchy (body → tail chain / wing chains / head) is walked by hand so the animated
    // descendants can be re-posed; the size scale and body pitch live in the root transform.
    let root = phantom_model_root_transform(instance, size);
    let flap = phantom_flap_time(instance.entity_id, instance.render_state.age_in_ticks);
    let wing_z = phantom_wing_z_rot(flap);
    let tail_x = phantom_tail_x_rot(flap);

    let body_t = root * part_pose_transform(PHANTOM_BODY_POSE);
    emit_model_cube(mesh, body_t, PHANTOM_BODY_CUBE);

    let tail_base_t =
        body_t * part_pose_transform(phantom_tail_pose(PHANTOM_TAIL_BASE_POSE, tail_x));
    emit_model_cube(mesh, tail_base_t, PHANTOM_TAIL_BASE_CUBE);
    let tail_tip_t =
        tail_base_t * part_pose_transform(phantom_tail_pose(PHANTOM_TAIL_TIP_POSE, tail_x));
    emit_model_cube(mesh, tail_tip_t, PHANTOM_TAIL_TIP_CUBE);

    let left_base_t =
        body_t * part_pose_transform(phantom_wing_pose(PHANTOM_LEFT_WING_BASE_POSE, wing_z));
    emit_model_cube(mesh, left_base_t, PHANTOM_LEFT_WING_BASE_CUBE);
    let left_tip_t =
        left_base_t * part_pose_transform(phantom_wing_pose(PHANTOM_LEFT_WING_TIP_POSE, wing_z));
    emit_model_cube(mesh, left_tip_t, PHANTOM_LEFT_WING_TIP_CUBE);

    let right_base_t =
        body_t * part_pose_transform(phantom_wing_pose(PHANTOM_RIGHT_WING_BASE_POSE, -wing_z));
    emit_model_cube(mesh, right_base_t, PHANTOM_RIGHT_WING_BASE_CUBE);
    let right_tip_t =
        right_base_t * part_pose_transform(phantom_wing_pose(PHANTOM_RIGHT_WING_TIP_POSE, -wing_z));
    emit_model_cube(mesh, right_tip_t, PHANTOM_RIGHT_WING_TIP_CUBE);

    emit_model_cube(
        mesh,
        body_t * part_pose_transform(PHANTOM_HEAD_POSE),
        PHANTOM_HEAD_CUBE,
    );
}

fn emit_pufferfish_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    puff_state: i32,
) {
    // Vanilla picks the small/mid/big model by puff state; each wiggles its two fins on
    // `ageInTicks` (the rest are static). The body bob lives in the root transform.
    let root = pufferfish_model_root_transform(instance);
    let (parts, fins) = pufferfish_parts(puff_state);
    let fin_z = pufferfish_right_fin_z_rot(instance.render_state.age_in_ticks);
    for (index, part) in parts.iter().enumerate() {
        let pose = if index == fins[0] {
            pufferfish_fin_pose(part.pose(), fin_z)
        } else if index == fins[1] {
            pufferfish_fin_pose(part.pose(), -fin_z)
        } else {
            part.pose()
        };
        emit_model_cubes_at_pose(mesh, root, pose, &[part.colored_cube()]);
    }
}

fn emit_cod_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `CodModel.setupAnim` sways only the tail fin (`yRot`); the swim wiggle and
    // out-of-water flop live in `cod_model_root_transform`.
    let in_water = instance.render_state.in_water;
    let root = cod_model_root_transform(instance, in_water);
    let tail_yrot = cod_tail_fin_yrot(instance.render_state.age_in_ticks, in_water);
    let mut parts = COD_PARTS.to_vec();
    parts[COD_TAIL_FIN_PART_INDEX].pose.rotation[1] = tail_yrot;
    emit_model_parts_with_color(mesh, &parts, root, COD_TAN);
}

fn emit_salmon_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    size: SalmonModelSize,
) {
    // Vanilla `SalmonModel.setupAnim` sways only the back body segment (`bodyBack.yRot`),
    // which carries the tail and rear top fin as children; the swim wiggle, out-of-water
    // flop, and the small/medium/large mesh scale live in `salmon_model_root_transform`.
    let in_water = instance.render_state.in_water;
    let root = salmon_model_root_transform(instance, in_water, size);
    let body_back_yrot = salmon_body_back_yrot(instance.render_state.age_in_ticks, in_water);
    let mut parts = SALMON_PARTS.to_vec();
    parts[SALMON_BODY_BACK_PART_INDEX].pose.rotation[1] = body_back_yrot;
    emit_model_parts_with_color(mesh, &parts, root, SALMON_RED);
}

fn emit_tropical_fish_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    shape: TropicalFishModelShape,
    base_color: EntityDyeColor,
) {
    // Vanilla `TropicalFish{Small,Large}Model.setupAnim` sways only the tail (`yRot`); the
    // swim wiggle and out-of-water flop live in `tropical_fish_model_root_transform`. The
    // kob-style small body and flopper-style large body differ only in geometry. The body is
    // tinted by the vanilla `getModelTint` = `getBaseColor().getTextureDiffuseColor()`.
    let in_water = instance.render_state.in_water;
    let root = tropical_fish_model_root_transform(instance, in_water);
    let tail_yrot = tropical_fish_tail_yrot(instance.render_state.age_in_ticks, in_water);
    let mut parts = match shape {
        TropicalFishModelShape::Small => TROPICAL_FISH_SMALL_PARTS.to_vec(),
        TropicalFishModelShape::Large => TROPICAL_FISH_LARGE_PARTS.to_vec(),
    };
    parts[TROPICAL_FISH_TAIL_PART_INDEX].pose.rotation[1] = tail_yrot;
    emit_model_parts_with_color(mesh, &parts, root, base_color.texture_diffuse_color());
}

fn emit_squid_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    glow: bool,
    baby: bool,
) {
    // Vanilla `SquidModel.setupAnim` only sweeps the eight tentacles by the lerped
    // `tentacleAngle` (`tentacle.xRot = tentacleAngle`); the body is static. The swim
    // body tilt and the `0.5/1.2` translate live in `squid_model_root_transform`.
    let root = squid_model_root_transform(instance, baby);
    let color = if glow { GLOW_SQUID_TEAL } else { SQUID_BLUE };
    let parts = squid_model_parts(instance.render_state.squid_tentacle_angle);
    emit_model_parts_with_color(mesh, &parts, root, color);
}

fn emit_player_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, slim: bool) {
    let transform = player_model_root_transform(instance);
    let parts: &[ModelPartDesc] = if slim {
        &PLAYER_SLIM_PARTS
    } else {
        &PLAYER_WIDE_PARTS
    };
    // `PlayerModel extends HumanoidModel`: its `setupAnim` only toggles part
    // visibility before `super.setupAnim`, so the legs and arms swing exactly as in the
    // inherited `HumanoidModel.setupAnim` (the pants/sleeve children ride the limb
    // parts). The held-item/attack arm poses, crouch, swim, the idle bob, and the
    // elytra `speedValue` are deferred.
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let parts = colored_head_look_parts(
        parts,
        player_head_part_index(),
        instance.render_state.head_yaw,
        instance.render_state.head_pitch,
    );
    let parts = humanoid_limb_swing_parts(
        parts,
        HUMANOID_LEG_PART_INDICES,
        limb_swing,
        limb_swing_amount,
    );
    let parts = humanoid_arm_swing_parts(
        parts,
        HUMANOID_ARM_PART_INDICES,
        limb_swing,
        limb_swing_amount,
    );
    emit_model_parts(mesh, &parts, transform);
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

fn emit_zombie_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    let parts: &[ModelPartDesc] = if baby {
        &BABY_ZOMBIE_PARTS
    } else {
        &ADULT_ZOMBIE_PARTS
    };
    let parts = humanoid_limb_swing_parts(
        zombie_colored_head_look_parts(parts, instance, baby),
        HUMANOID_LEG_PART_INDICES,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts(mesh, &parts, entity_model_root_transform(instance));
}

fn emit_zombie_variant_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: ZombieVariantModelFamily,
    baby: bool,
) {
    let (parts, transform, color): (&[ModelPartDesc], _, _) = match (family, baby) {
        (ZombieVariantModelFamily::Husk, false) => (
            &ADULT_ZOMBIE_PARTS,
            mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE),
            HUSK_TAN,
        ),
        (ZombieVariantModelFamily::Husk, true) => (
            &BABY_ZOMBIE_PARTS,
            entity_model_root_transform(instance),
            HUSK_TAN,
        ),
        (ZombieVariantModelFamily::Drowned, false) => (
            &ADULT_ZOMBIE_PARTS,
            entity_model_root_transform(instance),
            DROWNED_BLUE,
        ),
        (ZombieVariantModelFamily::Drowned, true) => (
            &BABY_ZOMBIE_PARTS,
            entity_model_root_transform(instance),
            DROWNED_BLUE,
        ),
        (ZombieVariantModelFamily::ZombieVillager, false) => (
            &ADULT_ZOMBIE_VILLAGER_PARTS,
            entity_model_root_transform(instance),
            ZOMBIE_VILLAGER_ROBE,
        ),
        (ZombieVariantModelFamily::ZombieVillager, true) => (
            &BABY_ZOMBIE_VILLAGER_PARTS,
            entity_model_root_transform(instance),
            ZOMBIE_VILLAGER_ROBE,
        ),
    };
    let parts = humanoid_limb_swing_parts(
        zombie_colored_head_look_parts(parts, instance, baby),
        HUMANOID_LEG_PART_INDICES,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts_with_color(mesh, &parts, transform, color);
}

/// Applies the vanilla `HumanoidModel.setupAnim` head look to a zombie-family
/// layer's head part (index `baby ? 1 : 0`).
fn zombie_colored_head_look_parts(
    parts: &[ModelPartDesc],
    instance: EntityModelInstance,
    baby: bool,
) -> Cow<'_, [ModelPartDesc]> {
    colored_head_look_parts(
        parts,
        zombie_head_part_index(baby),
        instance.render_state.head_yaw,
        instance.render_state.head_pitch,
    )
}

fn emit_piglin_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: PiglinModelFamily,
    baby: bool,
) {
    let baby_layout = baby && family != PiglinModelFamily::PiglinBrute;
    let parts: &[ModelPartDesc] = if baby_layout {
        &BABY_PIGLIN_PARTS
    } else {
        &ADULT_PIGLIN_PARTS
    };
    // `AbstractPiglinModel extends HumanoidModel`: its `setupAnim` runs
    // `super.setupAnim` (the inherited leg and arm swing) before flapping the ears.
    // `PiglinModel` (adult/baby piglin and the brute, which reuses `AdultPiglinModel`)
    // overrides the arms only in its dance/attack/crossbow/admire poses (deferred), so
    // the default arms keep the `HumanoidModel.setupAnim` counter-swing. The zombified
    // piglin instead overwrites the arms with `AnimationUtils.animateZombieArms` (the
    // held-out zombie pose, deferred), so its arms stay at rest. Every subclass, however,
    // runs `super.setupAnim`, so the ears always flap (`piglin_ear_flap_pose`): the ears
    // are nested children of the head, so the head subtree is hand-emitted with the flapped
    // ears (the dance/attack/crossbow/admire arm poses stay deferred).
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let age_in_ticks = instance.render_state.age_in_ticks;
    let head_index = piglin_head_part_index(baby_layout);
    let parts = humanoid_limb_swing_parts(
        colored_head_look_parts(
            parts,
            head_index,
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        ),
        HUMANOID_LEG_PART_INDICES,
        limb_swing,
        limb_swing_amount,
    );
    let parts = if family == PiglinModelFamily::ZombifiedPiglin {
        parts
    } else {
        humanoid_arm_swing_parts(
            parts,
            HUMANOID_ARM_PART_INDICES,
            limb_swing,
            limb_swing_amount,
        )
    };
    let transform = entity_model_root_transform(instance);
    let color = piglin_model_color(family);
    let (left_ear, right_ear) = piglin_ear_child_indices(baby_layout);
    let default_ear_angle = piglin_default_ear_angle(baby_layout);
    for (index, part) in parts.iter().enumerate() {
        if index == head_index {
            // The ears are `&'static` head children, so emit the head cubes then the
            // children with the flapped ear poses (the ravager/hoglin pattern).
            let head_transform = transform * part_pose_transform(part.pose);
            for cube in part.cubes {
                emit_model_cube_with_color(mesh, head_transform, *cube, color);
            }
            let mut head_children = part.children.to_vec();
            head_children[left_ear].pose = piglin_ear_flap_pose(
                head_children[left_ear].pose,
                true,
                default_ear_angle,
                age_in_ticks,
                limb_swing,
                limb_swing_amount,
            );
            head_children[right_ear].pose = piglin_ear_flap_pose(
                head_children[right_ear].pose,
                false,
                default_ear_angle,
                age_in_ticks,
                limb_swing,
                limb_swing_amount,
            );
            emit_model_parts_with_color(mesh, &head_children, head_transform, color);
        } else {
            emit_model_part_with_color(mesh, part, transform, color);
        }
    }
}

/// Ear child indices `(left, right)` under the piglin head part. The adult/brute layout
/// lists the two ears directly at `[0, 1]`; the baby layout (baby piglin / baby zombified
/// piglin) lists the hat at `0` and the ear holders at `[1, 2]`.
fn piglin_ear_child_indices(baby_layout: bool) -> (usize, usize) {
    if baby_layout {
        (1, 2)
    } else {
        (0, 1)
    }
}

/// `AbstractPiglinModel.getDefaultEarAngleInDegrees()` (in radians): `5°` for the baby
/// layout, `30°` for the adult/brute layout.
fn piglin_default_ear_angle(baby_layout: bool) -> f32 {
    if baby_layout {
        PIGLIN_BABY_EAR_ANGLE
    } else {
        PIGLIN_ADULT_EAR_ANGLE
    }
}

fn emit_hoglin_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: HoglinModelFamily,
    baby: bool,
) {
    let parts: &[ModelPartDesc] = if baby {
        &BABY_HOGLIN_PARTS
    } else {
        &ADULT_HOGLIN_PARTS
    };
    // Vanilla `HoglinModel.setupAnim` (zoglin shares it) swings the four legs
    // `cos(pos [+ π]) * 1.2 * speed` (amplitude 1.2, no 0.6662 factor; right-front/
    // left-hind in phase) after the yaw-only head look, and sways the ears
    // `ear.zRot = ±2π/9 ± speed * sin(pos)` (the literal 2π/9, which also overrides the
    // baby layer's wider ear rest angle). Legs are at [2, 3, 4, 5] in both layers; the
    // headbutt head tilt is deferred.
    let head_index = hoglin_head_part_index(baby);
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let transform = entity_model_root_transform(instance);
    let color = hoglin_model_color(family);
    let parts = hoglin_limb_swing_parts(
        hoglin_colored_head_look_parts(parts, head_index, instance.render_state.head_yaw),
        limb_swing,
        limb_swing_amount,
    );
    // The adult ears rest at ±2π/9, so they only need re-posing when walking; the baby ears
    // rest at a wider angle that vanilla overrides to ±2π/9, so they are always re-posed.
    if !baby && limb_swing_at_rest(limb_swing_amount) {
        emit_model_parts_with_color(mesh, &parts, transform, color);
        return;
    }
    // The ears are children of the head, whose children list is static, so emit the head
    // subtree by hand with the posed ears (the horns ride unchanged).
    for (index, part) in parts.iter().enumerate() {
        if index == head_index {
            let head_transform = transform * part_pose_transform(part.pose);
            for cube in part.cubes {
                emit_model_cube_with_color(mesh, head_transform, *cube, color);
            }
            let mut children = part.children.to_vec();
            children[HOGLIN_RIGHT_EAR_CHILD_INDEX].pose = hoglin_ear_sway_pose(
                children[HOGLIN_RIGHT_EAR_CHILD_INDEX].pose,
                false,
                limb_swing,
                limb_swing_amount,
            );
            children[HOGLIN_LEFT_EAR_CHILD_INDEX].pose = hoglin_ear_sway_pose(
                children[HOGLIN_LEFT_EAR_CHILD_INDEX].pose,
                true,
                limb_swing,
                limb_swing_amount,
            );
            emit_model_parts_with_color(mesh, &children, head_transform, color);
        } else {
            emit_model_part_with_color(mesh, part, transform, color);
        }
    }
}

/// The four leg part indices in the hoglin/zoglin body layers (the head and body
/// occupy `0`/`1` in either order). [`hoglin_leg_swing_pose`] resolves each leg's
/// phase from its offset, so the differing head/body order of the adult and baby
/// layers does not matter.
const HOGLIN_LEG_PART_INDICES: [usize; 4] = [2, 3, 4, 5];

/// Applies the vanilla `HoglinModel.setupAnim` leg swing ([`hoglin_leg_swing_pose`])
/// to a colored hoglin layer's four leg parts. Borrows the static parts unchanged at
/// rest (`walkAnimationSpeed == 0`).
fn hoglin_limb_swing_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    limb_swing: f32,
    limb_swing_amount: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if limb_swing_at_rest(limb_swing_amount) {
        return parts;
    }
    let mut owned = parts.into_owned();
    for index in HOGLIN_LEG_PART_INDICES {
        owned[index].pose = hoglin_leg_swing_pose(owned[index].pose, limb_swing, limb_swing_amount);
    }
    Cow::Owned(owned)
}

fn emit_ravager_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    let transform = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    // Vanilla `RavagerModel.setupAnim` swings the four legs `cos(pos * 0.6662 [+ π]) *
    // 0.4 * speed` (`ravager_leg_swing_pose`, legs at [2, 3, 4, 5]). The neck/mouth
    // attack/stun/roar poses are deferred. Pre-pose the legs; the neck (part 0) is
    // untouched by the swing, so the head handling below still works on it.
    let parts = ravager_limb_swing_parts(
        Cow::Borrowed(&RAVAGER_PARTS),
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    if head_look_at_rest(head_yaw, head_pitch) {
        emit_model_parts(mesh, &parts, transform);
        return;
    }
    // Vanilla `RavagerModel.setupAnim` sets `head.xRot`/`head.yRot` from the look,
    // but the head is `neck.getChild("head")`. The neck's children list is static
    // (can't be swapped for an owned copy), so emit the neck subtree by hand: the
    // neck cubes, then the head child carrying the look (its horn/mouth children
    // inherit the rotation as in vanilla), keeping the original emit order.
    let neck = &parts[ravager_neck_part_index()];
    let neck_transform = transform * part_pose_transform(neck.pose);
    for cube in neck.cubes {
        emit_model_cube(mesh, neck_transform, *cube);
    }
    let head = RAVAGER_NECK_CHILDREN[ravager_head_child_index()];
    let looked_head = ModelPartDesc {
        pose: head_look_pose(head.pose, head_yaw, head_pitch),
        ..head
    };
    emit_model_parts(mesh, &[looked_head], neck_transform);
    // The remaining body and (swung) leg parts are unaffected by the head look.
    for part in &parts[ravager_neck_part_index() + 1..] {
        emit_model_part(mesh, part, transform);
    }
}

/// The four leg part indices in the ravager body layer: the neck and body occupy
/// `0`/`1`, then the right/left hind and right/left front legs.
const RAVAGER_LEG_PART_INDICES: [usize; 4] = [2, 3, 4, 5];

/// Applies the vanilla `RavagerModel.setupAnim` leg swing
/// ([`ravager_leg_swing_pose`]) to a colored ravager layer's four leg parts. Borrows
/// the static parts unchanged at rest (`walkAnimationSpeed == 0`).
fn ravager_limb_swing_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    limb_swing: f32,
    limb_swing_amount: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if limb_swing_at_rest(limb_swing_amount) {
        return parts;
    }
    let mut owned = parts.into_owned();
    for index in RAVAGER_LEG_PART_INDICES {
        owned[index].pose =
            ravager_leg_swing_pose(owned[index].pose, limb_swing, limb_swing_amount);
    }
    Cow::Owned(owned)
}

fn emit_skeleton_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &skeleton_colored_posed_parts(&SKELETON_PARTS, skeleton_head_part_index(), instance),
        entity_model_root_transform(instance),
    );
}

fn emit_skeleton_variant_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: SkeletonModelFamily,
) {
    match family {
        SkeletonModelFamily::Stray => emit_model_parts(
            mesh,
            &skeleton_colored_posed_parts(&SKELETON_PARTS, skeleton_head_part_index(), instance),
            entity_model_root_transform(instance),
        ),
        SkeletonModelFamily::Parched => emit_model_parts(
            mesh,
            &skeleton_colored_posed_parts(&PARCHED_PARTS, parched_head_part_index(), instance),
            entity_model_root_transform(instance),
        ),
        SkeletonModelFamily::Bogged { sheared } => {
            let parts: &[ModelPartDesc] = if sheared {
                &BOGGED_SHEARED_PARTS
            } else {
                &BOGGED_PARTS
            };
            emit_model_parts(
                mesh,
                &skeleton_colored_posed_parts(parts, skeleton_head_part_index(), instance),
                entity_model_root_transform(instance),
            )
        }
        SkeletonModelFamily::WitherSkeleton => emit_model_parts_with_color(
            mesh,
            &skeleton_colored_posed_parts(&SKELETON_PARTS, skeleton_head_part_index(), instance),
            wither_skeleton_model_root_transform(instance),
            WITHER_SKELETON_DARK,
        ),
    }
}

/// Applies the vanilla `HumanoidModel.setupAnim` head look, leg swing, and arm swing to
/// a skeleton-family layer. `SkeletonModel extends HumanoidModel` and overrides the arms
/// only in its melee branch (`isAggressive && !isHoldingBow`, deferred) and the bow
/// aiming is a deferred `ArmPose`, so in the default state the legs and arms swing
/// exactly as in the inherited `HumanoidModel.setupAnim` (arms at `[2, 3]`).
fn skeleton_colored_posed_parts(
    parts: &[ModelPartDesc],
    head_index: usize,
    instance: EntityModelInstance,
) -> Cow<'_, [ModelPartDesc]> {
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let parts = colored_head_look_parts(
        parts,
        head_index,
        instance.render_state.head_yaw,
        instance.render_state.head_pitch,
    );
    let parts = humanoid_limb_swing_parts(
        parts,
        HUMANOID_LEG_PART_INDICES,
        limb_swing,
        limb_swing_amount,
    );
    humanoid_arm_swing_parts(
        parts,
        HUMANOID_ARM_PART_INDICES,
        limb_swing,
        limb_swing_amount,
    )
}

/// Vanilla `QuadrupedModel` leg part indices in the cow and pig body layers: the
/// head and body occupy slots `0` and `1` (in either order — the baby layers swap
/// them), then the four legs. The variants order the legs differently (adult layers
/// list them hind-first, baby layers front-first), so [`quadruped_limb_swing_parts`]
/// resolves each leg's phase from its offset rather than its slot.
pub(in crate::entity_models) const QUADRUPED_LEG_PART_INDICES: [usize; 4] = [2, 3, 4, 5];

fn emit_cow_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    variant: CowModelVariant,
    baby: bool,
) {
    let parts = colored_head_look_parts(
        cow_model_parts(variant, baby),
        cow_head_part_index(baby),
        instance.render_state.head_yaw,
        instance.render_state.head_pitch,
    );
    let parts = quadruped_limb_swing_parts(
        parts,
        QUADRUPED_LEG_PART_INDICES,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts(mesh, &parts, entity_model_root_transform(instance));
}

/// Applies the vanilla `QuadrupedModel.setupAnim` leg swing
/// ([`quadruped_leg_swing_pose`]) to a colored layer's four leg parts at
/// `leg_indices`. Borrows the static parts unchanged at rest
/// (`walkAnimationSpeed == 0`).
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
/// but keep arms at `2`/`3` and legs at `4`/`5`).
pub(in crate::entity_models) const HUMANOID_LEG_PART_INDICES: [usize; 2] = [4, 5];

/// Applies the vanilla `HumanoidModel.setupAnim` leg swing
/// ([`humanoid_leg_swing_pose`]) to a colored layer's two leg parts at
/// `leg_indices`. Borrows the static parts unchanged at rest
/// (`walkAnimationSpeed == 0`). The arm swing is left to each humanoid subclass,
/// which overrides the arms (e.g. the zombie held-out pose), so only the legs —
/// which subclasses inherit unchanged from `HumanoidModel` — are swung here.
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
/// the right and left arms at `[2, 3]` (every humanoid layer, adult or baby).
pub(in crate::entity_models) const HUMANOID_ARM_PART_INDICES: [usize; 2] = [2, 3];

/// Applies the vanilla `HumanoidModel.setupAnim` arm swing ([`humanoid_arm_swing_pose`])
/// to a colored layer's two arm parts at `arm_indices`. Borrows the static parts
/// unchanged at rest (`walkAnimationSpeed == 0`). Callers whose subclass keeps the
/// inherited default arms use this (the player and the skeleton family); the
/// zombie/piglin constant arms-out poses stay deferred.
pub(in crate::entity_models) fn humanoid_arm_swing_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    arm_indices: [usize; 2],
    limb_swing: f32,
    limb_swing_amount: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if limb_swing_at_rest(limb_swing_amount) {
        return parts;
    }
    let mut owned = parts.into_owned();
    for index in arm_indices {
        if let Some(arm) = owned.get_mut(index) {
            arm.pose = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
        }
    }
    Cow::Owned(owned)
}

/// Applies the vanilla `QuadrupedModel`/`HumanoidModel.setupAnim` head look to a
/// colored layer's head part, borrowing the static parts unchanged while the
/// head is level and aligned with the body.
fn colored_head_look_parts(
    parts: &[ModelPartDesc],
    head_index: usize,
    head_yaw: f32,
    head_pitch: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if head_look_at_rest(head_yaw, head_pitch) {
        return Cow::Borrowed(parts);
    }
    let mut parts = parts.to_vec();
    if let Some(head) = parts.get_mut(head_index) {
        head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
    }
    Cow::Owned(parts)
}

/// Applies the vanilla `HoglinModel.setupAnim` yaw-only head look to a colored
/// hoglin layer's head part. Vanilla sets `head.yRot = yRot * π/180` but leaves
/// `head.xRot` at the headbutt animation value (the fixed `HOGLIN_HEAD_X_ROT`
/// rest tilt baked into the base pose), so only the yaw is applied here.
fn hoglin_colored_head_look_parts(
    parts: &[ModelPartDesc],
    head_index: usize,
    head_yaw: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if head_yaw_at_rest(head_yaw) {
        return Cow::Borrowed(parts);
    }
    let mut parts = parts.to_vec();
    if let Some(head) = parts.get_mut(head_index) {
        head.pose = head_look_yaw_pose(head.pose, head_yaw);
    }
    Cow::Owned(parts)
}

fn emit_sheep_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
    invisible: bool,
    jeb: bool,
    age_ticks: f32,
) {
    let transform = entity_model_root_transform(instance);
    let head_eat = instance.render_state.head_eat;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    // Vanilla `SheepModel.setupAnim` runs `super.setupAnim` (the `QuadrupedModel`
    // head look + leg swing) before its eat-grass head pose, so every sheep layer
    // (body and wool) swings its legs. `sheep_colored_head_parts` poses the head;
    // the leg swing is layered on top for each part set.
    let posed = |parts: &'static [ModelPartDesc]| {
        quadruped_limb_swing_parts(
            sheep_colored_head_parts(parts, baby, head_eat, head_yaw, head_pitch),
            QUADRUPED_LEG_PART_INDICES,
            limb_swing,
            limb_swing_amount,
        )
    };
    let base_parts: &[ModelPartDesc] = if baby {
        &BABY_SHEEP_PARTS
    } else {
        &ADULT_SHEEP_PARTS
    };
    emit_model_parts(mesh, &posed(base_parts), transform);
    let wool_layer_color = sheep_wool_render_color(wool_color, jeb, age_ticks);
    if !invisible && !baby && (jeb || wool_color != SheepWoolColor::White) {
        emit_model_parts_with_color(
            mesh,
            &posed(&ADULT_SHEEP_PARTS),
            transform,
            wool_layer_color,
        );
    }
    if !invisible && !sheared {
        let wool_parts: &[ModelPartDesc] = if baby {
            &BABY_SHEEP_PARTS
        } else {
            &ADULT_SHEEP_WOOL_PARTS
        };
        emit_model_parts_with_color(mesh, &posed(wool_parts), transform, wool_layer_color);
    }
}

/// Applies the vanilla sheep head pose (eat-grass animation plus head look) to a
/// colored body/wool layer's head part, borrowing the static parts unchanged
/// while the head is fully at rest.
fn sheep_colored_head_parts(
    parts: &[ModelPartDesc],
    baby: bool,
    head_eat: SheepHeadEatPose,
    head_yaw: f32,
    head_pitch: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if sheep_head_at_rest(head_eat, head_yaw, head_pitch) {
        return Cow::Borrowed(parts);
    }
    let head_index = sheep_head_part_index(baby);
    let mut parts = parts.to_vec();
    if let Some(head) = parts.get_mut(head_index) {
        head.pose = sheep_head_pose(head.pose, baby, head_eat, head_yaw, head_pitch);
    }
    Cow::Owned(parts)
}

fn emit_villager_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    let (parts, transform): (&[ModelPartDesc], _) = if baby {
        (&BABY_VILLAGER_PARTS, entity_model_root_transform(instance))
    } else {
        (
            &ADULT_VILLAGER_PARTS,
            villager_adult_model_root_transform(instance),
        )
    };
    // `VillagerModel.setupAnim` swings the legs `cos(pos * 0.6662 [+ π]) * 1.4 *
    // speed * 0.5` (half the `HumanoidModel` amplitude, no riding branch) after the
    // head look. The combined `arms` part and the unhappy head shake are deferred.
    let parts = half_amplitude_limb_swing_parts(
        villager_colored_head_look_parts(parts, villager_head_part_index(baby), instance),
        villager_leg_part_indices(baby),
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts(mesh, &parts, transform);
}

/// The right/left leg part indices in the villager body layers. The adult layer
/// lists the combined `arms` part at slot `2` then the legs at `[3, 4]`; the baby
/// layer reorders the parts and lists the legs at `[1, 2]`.
/// [`half_amplitude_leg_swing_pose`] resolves each leg's phase from its offset, so
/// only the slot positions differ.
fn villager_leg_part_indices(baby: bool) -> [usize; 2] {
    if baby {
        [1, 2]
    } else {
        [3, 4]
    }
}

fn emit_wandering_trader_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // The wandering trader uses the adult `VillagerModel` layer, so its legs swing
    // the same half-amplitude swing (legs at `[3, 4]`).
    let parts = half_amplitude_limb_swing_parts(
        villager_colored_head_look_parts(
            &ADULT_VILLAGER_PARTS,
            villager_head_part_index(false),
            instance,
        ),
        villager_leg_part_indices(false),
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts(mesh, &parts, villager_adult_model_root_transform(instance));
}

/// Applies the vanilla `VillagerModel`/`IllagerModel`/`WitchModel.setupAnim` head
/// look to a villager-family layer's head part at `head_index`.
fn villager_colored_head_look_parts(
    parts: &[ModelPartDesc],
    head_index: usize,
    instance: EntityModelInstance,
) -> Cow<'_, [ModelPartDesc]> {
    colored_head_look_parts(
        parts,
        head_index,
        instance.render_state.head_yaw,
        instance.render_state.head_pitch,
    )
}

fn emit_wolf_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    baby: bool,
    angry: bool,
) {
    let parts: &[ModelPartDesc] = if baby {
        &BABY_WOLF_PARTS
    } else {
        &ADULT_WOLF_PARTS
    };
    // Vanilla `WolfModel.setupAnim` (shared by adult and baby) sets `tail.yRot` (angry → 0,
    // else the wag), then either folds into the sitting pose or swings the four legs with
    // the `QuadrupedModel` diagonal phase `cos(pos * 0.6662 [+ π]) * 1.4 * speed`, then
    // applies the head look, then sets `tail.xRot = tailAngle` — the `π/5` rest droop for an
    // untamed wolf or the tame/health droop `(0.55 - damageRatio * 0.4) * π` projected into
    // `wolf_tail_angle`. A sitting wolf (`isSitting`) tilts its body and tucks its legs
    // (`setSittingPose`) instead of the leg swing; the head still follows the look. The
    // water-shake body roll is deferred.
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let tail_angle = instance.render_state.wolf_tail_angle;
    let tail_index = wolf_tail_part_index(baby);
    let head_looked = head_first_colored_head_look_parts(parts, instance);
    let posed = if instance.render_state.wolf_sitting {
        wolf_sitting_parts(
            head_looked,
            baby,
            angry,
            tail_angle,
            limb_swing,
            limb_swing_amount,
        )
    } else {
        let legs_and_head = quadruped_limb_swing_parts(
            head_looked,
            wolf_leg_part_indices(baby),
            limb_swing,
            limb_swing_amount,
        );
        if angry {
            wolf_angry_tail_parts(legs_and_head, tail_index)
        } else {
            wolf_tail_wag_parts(
                legs_and_head,
                tail_index,
                tail_angle,
                limb_swing,
                limb_swing_amount,
            )
        }
    };
    emit_model_parts(mesh, &posed, entity_model_root_transform(instance));
}

/// Folds a colored wolf layer into the vanilla `WolfModel.setSittingPose`: the body, hind
/// legs, front legs, and tail are repositioned ([`apply_wolf_sitting_pose`]) instead of
/// swinging the legs, and the tail still carries its normal `tailAngle`/wag rotation on top
/// of the sitting offset lift.
fn wolf_sitting_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    baby: bool,
    angry: bool,
    tail_angle: f32,
    limb_swing: f32,
    limb_swing_amount: f32,
) -> Cow<'_, [ModelPartDesc]> {
    let mut owned = parts.into_owned();
    for (index, role) in wolf_sitting_part_roles(baby) {
        if let Some(part) = owned.get_mut(index) {
            apply_wolf_sitting_pose(&mut part.pose, role, baby);
        }
    }
    let tail_index = wolf_tail_part_index(baby);
    if let Some(tail) = owned.get_mut(tail_index) {
        // The sitting role already lifted the tail offset; layer on the normal tail
        // rotation (both helpers preserve the offset).
        tail.pose = if angry {
            wolf_angry_tail_pose(tail.pose)
        } else {
            wolf_tail_swing_pose(tail.pose, tail_angle, limb_swing, limb_swing_amount)
        };
    }
    Cow::Owned(owned)
}

/// Holds the wolf tail straight and raised for an angry wolf ([`wolf_angry_tail_pose`]).
/// Unlike the wag, this always re-poses the tail (the `1.5393804` raise overrides the
/// layer's `π/5` rest droop even when standing).
fn wolf_angry_tail_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    tail_index: usize,
) -> Cow<'_, [ModelPartDesc]> {
    let mut owned = parts.into_owned();
    if let Some(tail) = owned.get_mut(tail_index) {
        tail.pose = wolf_angry_tail_pose(tail.pose);
    }
    Cow::Owned(owned)
}

/// Applies the vanilla `WolfModel.setupAnim` tail wag ([`wolf_tail_swing_pose`]) to a
/// colored wolf layer's tail part. Borrows the static parts unchanged when the resulting
/// pose is byte-identical to the layer rest — a standing (`walkAnimationSpeed == 0`) wolf
/// whose `tail_angle` equals the layer's `π/5` rest droop (i.e. an untamed wolf).
fn wolf_tail_wag_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    tail_index: usize,
    tail_angle: f32,
    limb_swing: f32,
    limb_swing_amount: f32,
) -> Cow<'_, [ModelPartDesc]> {
    let Some(base) = parts.get(tail_index) else {
        return parts;
    };
    let posed = wolf_tail_swing_pose(base.pose, tail_angle, limb_swing, limb_swing_amount);
    if posed == base.pose {
        return parts;
    }
    let mut owned = parts.into_owned();
    owned[tail_index].pose = posed;
    Cow::Owned(owned)
}

/// The four leg part indices in the wolf body layers. The adult layer lists the head,
/// body, and mane (`upper_body`) at `0`/`1`/`2` then the legs at `[3, 4, 5, 6]`; the
/// baby layer drops the mane, so the head and body sit at `0`/`1` and the legs at
/// `[2, 3, 4, 5]`. [`quadruped_leg_swing_pose`] resolves each leg's phase from its
/// offset, so only the slot positions differ.
fn wolf_leg_part_indices(baby: bool) -> [usize; 4] {
    if baby {
        [2, 3, 4, 5]
    } else {
        [3, 4, 5, 6]
    }
}

fn emit_goat_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    baby: bool,
    left_horn: bool,
    right_horn: bool,
) {
    let (parts, head_index, left_horn_child_index, right_horn_child_index): (
        &[ModelPartDesc],
        usize,
        usize,
        usize,
    ) = if baby {
        (
            &BABY_GOAT_PARTS,
            BABY_GOAT_HEAD_INDEX,
            BABY_GOAT_LEFT_HORN_CHILD_INDEX,
            BABY_GOAT_RIGHT_HORN_CHILD_INDEX,
        )
    } else {
        (
            &ADULT_GOAT_PARTS,
            ADULT_GOAT_HEAD_INDEX,
            ADULT_GOAT_LEFT_HORN_CHILD_INDEX,
            ADULT_GOAT_RIGHT_HORN_CHILD_INDEX,
        )
    };
    let transform = entity_model_root_transform(instance);
    // Vanilla `GoatModel extends QuadrupedModel`: `setupAnim` runs `super.setupAnim`
    // (the `QuadrupedModel` leg swing) before its horn visibility and ramming head
    // override, so the four legs swing. Pre-pose the legs (the swing touches only the
    // leg parts, leaving the head for `emit_goat_parts` to look at). The ramming head
    // tilt is a deferred event animation.
    let posed = quadruped_limb_swing_parts(
        Cow::Borrowed(parts),
        goat_leg_part_indices(baby),
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_goat_parts(
        mesh,
        &posed,
        transform,
        head_index,
        left_horn_child_index,
        right_horn_child_index,
        left_horn,
        right_horn,
        instance.render_state.head_yaw,
        instance.render_state.head_pitch,
    );
}

/// The four leg part indices in the goat body layers. The adult layer lists the
/// head and body at `0`/`1` then the legs at `[2, 3, 4, 5]`; the baby layer lists
/// the legs first at `[0, 1, 2, 3]` (head at `5`). [`quadruped_leg_swing_pose`]
/// resolves each leg's phase from its offset, so only the slot positions differ.
fn goat_leg_part_indices(baby: bool) -> [usize; 4] {
    if baby {
        [0, 1, 2, 3]
    } else {
        [2, 3, 4, 5]
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_goat_parts(
    mesh: &mut EntityModelMesh,
    parts: &[ModelPartDesc],
    parent_transform: Mat4,
    head_index: usize,
    left_horn_child_index: usize,
    right_horn_child_index: usize,
    left_horn: bool,
    right_horn: bool,
    head_yaw: f32,
    head_pitch: f32,
) {
    let head = &parts[head_index];
    // Vanilla GoatModel extends QuadrupedModel: the head look (set by the super
    // setupAnim) survives because the ramming override only fires when the goat
    // is actively ramming, which is an untracked event animation.
    let head_pose = if head_look_at_rest(head_yaw, head_pitch) {
        head.pose
    } else {
        head_look_pose(head.pose, head_yaw, head_pitch)
    };
    let head_transform = parent_transform * part_pose_transform(head_pose);
    for cube in head.cubes {
        emit_model_cube(mesh, head_transform, *cube);
    }
    for (index, child) in head.children.iter().enumerate() {
        if (index == left_horn_child_index && !left_horn)
            || (index == right_horn_child_index && !right_horn)
        {
            continue;
        }
        emit_model_part(mesh, child, head_transform);
    }
    for (index, part) in parts.iter().enumerate() {
        if index != head_index {
            emit_model_part(mesh, part, parent_transform);
        }
    }
}

fn emit_polar_bear_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    let transform = if baby {
        entity_model_root_transform(instance)
    } else {
        polar_bear_model_root_transform(instance)
    };
    let static_parts: &[ModelPartDesc] = if baby {
        &BABY_POLAR_BEAR_PARTS
    } else {
        &ADULT_POLAR_BEAR_PARTS
    };
    let stand_scale = instance.render_state.polar_bear_stand_scale;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    if stand_scale == 0.0
        && head_look_at_rest(head_yaw, head_pitch)
        && limb_swing_at_rest(limb_swing_amount)
    {
        emit_model_parts(mesh, static_parts, transform);
        return;
    }
    // Vanilla `PolarBearModel.setupAnim` first runs `super.setupAnim` (the
    // `QuadrupedModel` head look and four-leg swing), then the standing rear adds its
    // deltas on top — including `frontLeg.xRot -= standScale * π * 0.45` on top of the
    // swing — so apply the look and leg swing before the standing pose.
    let mut parts = static_parts.to_vec();
    if let Some(head) = parts.get_mut(polar_bear_head_part_index(baby)) {
        head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
    }
    if !limb_swing_at_rest(limb_swing_amount) {
        for index in QUADRUPED_LEG_PART_INDICES {
            parts[index].pose =
                quadruped_leg_swing_pose(parts[index].pose, limb_swing, limb_swing_amount);
        }
    }
    if stand_scale != 0.0 {
        for (index, part) in polar_bear_standing_part_roles(baby) {
            apply_polar_bear_standing_pose(&mut parts[index].pose, part, baby, stand_scale);
        }
    }
    emit_model_parts(mesh, &parts, transform);
}

fn emit_witch_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // `WitchModel.setupAnim` swings the legs `cos(pos * 0.6662 [+ π]) * 1.4 * speed *
    // 0.5` (half amplitude, legs at `[3, 4]`) after the head look, then bobs the nose
    // continuously (`witch_nose_bob_pose`, driven by `ageInTicks` and the entity id). The
    // nose is a `&'static` head child, so the head subtree is hand-emitted with the bobbed
    // nose. The `isHoldingItem` nose hold pose and the combined `arms` part are deferred.
    let head_index = villager_head_part_index(false);
    let parts = half_amplitude_limb_swing_parts(
        villager_colored_head_look_parts(&WITCH_PARTS, head_index, instance),
        villager_leg_part_indices(false),
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    let transform = villager_adult_model_root_transform(instance);
    let age_in_ticks = instance.render_state.age_in_ticks;
    let entity_id = instance.entity_id;
    for (index, part) in parts.iter().enumerate() {
        if index == head_index {
            let head_transform = transform * part_pose_transform(part.pose);
            for cube in part.cubes {
                emit_model_cube(mesh, head_transform, *cube);
            }
            let mut head_children = part.children.to_vec();
            head_children[WITCH_NOSE_CHILD_INDEX].pose = witch_nose_bob_pose(
                head_children[WITCH_NOSE_CHILD_INDEX].pose,
                age_in_ticks,
                entity_id,
            );
            emit_model_parts(mesh, &head_children, head_transform);
        } else {
            emit_model_part(mesh, part, transform);
        }
    }
}

fn emit_illager_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: IllagerModelFamily,
) {
    // `IllagerModel.setupAnim` (the non-riding branch) swings the legs
    // `cos(pos * 0.6662 [+ π]) * 1.4 * speed * 0.5` after the head look. The legs are not a
    // `HumanoidModel` swing (the extra `0.5` factor and the per-family part order differ),
    // so they use the dedicated `half_amplitude_leg_swing_pose`. The separate arms, however,
    // swing with the exact `HumanoidModel` amplitude `cos(pos * 0.6662 [+ π]) * 2.0 *
    // speed * 0.5` ([`humanoid_arm_swing_pose`]) — but only the pillager renders the
    // separate uncrossed arms; the evoker/vindicator/illusioner show the static crossed
    // `arms` part (vanilla swings the *invisible* separate arms, so their visible arms hold
    // still). The arm-pose overrides (attack/spellcast/bow/crossbow/celebrate) and the
    // riding sit pose are deferred (they need the `IllagerArmPose`/riding render state).
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let legs_swung = half_amplitude_limb_swing_parts(
        villager_colored_head_look_parts(
            illager_model_parts(family),
            villager_head_part_index(false),
            instance,
        ),
        illager_leg_part_indices(family),
        limb_swing,
        limb_swing_amount,
    );
    let parts = match illager_arm_part_indices(family) {
        Some(arm_indices) => {
            illager_arm_swing_parts(legs_swung, arm_indices, limb_swing, limb_swing_amount)
        }
        None => legs_swung,
    };
    emit_model_parts(mesh, &parts, villager_adult_model_root_transform(instance));
}

/// The two separate arm part indices in an illager body layer, if the family renders the
/// uncrossed (separate) arms. Only the pillager does (`ILLAGER_SHARED_UNCROSSED_PARTS`:
/// head/body/leg/leg/right_arm/left_arm); the evoker/vindicator/illusioner show the static
/// crossed `arms` part instead, so they have no separate arms to swing.
fn illager_arm_part_indices(family: IllagerModelFamily) -> Option<[usize; 2]> {
    match family {
        IllagerModelFamily::Pillager => Some([4, 5]),
        IllagerModelFamily::Evoker
        | IllagerModelFamily::Vindicator
        | IllagerModelFamily::Illusioner => None,
    }
}

/// Applies the vanilla `IllagerModel.setupAnim` arm swing ([`humanoid_arm_swing_pose`]) to
/// an illager layer's two separate arm parts. Borrows the static parts unchanged at rest
/// (`walkAnimationSpeed == 0`).
fn illager_arm_swing_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    arm_indices: [usize; 2],
    limb_swing: f32,
    limb_swing_amount: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if limb_swing_at_rest(limb_swing_amount) {
        return parts;
    }
    let mut owned = parts.into_owned();
    for index in arm_indices {
        if let Some(arm) = owned.get_mut(index) {
            arm.pose = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
        }
    }
    Cow::Owned(owned)
}

fn illager_model_parts(family: IllagerModelFamily) -> &'static [ModelPartDesc] {
    match family {
        IllagerModelFamily::Evoker | IllagerModelFamily::Vindicator => {
            &ILLAGER_SHARED_CROSSED_PARTS
        }
        IllagerModelFamily::Illusioner => &ILLAGER_ILLUSIONER_PARTS,
        IllagerModelFamily::Pillager => &ILLAGER_SHARED_UNCROSSED_PARTS,
    }
}

/// The right/left leg part indices in each illager body layer. The crossed-arms
/// layouts (evoker, vindicator, illusioner) carry one combined crossed-arm part at
/// slot `2` and list the legs at `[3, 4]`; the uncrossed pillager layout lists the
/// legs at `[2, 3]` before its two separate arms. [`half_amplitude_leg_swing_pose`]
/// resolves each leg's phase from its offset, so only the slot positions differ.
fn illager_leg_part_indices(family: IllagerModelFamily) -> [usize; 2] {
    match family {
        IllagerModelFamily::Pillager => [2, 3],
        IllagerModelFamily::Evoker
        | IllagerModelFamily::Vindicator
        | IllagerModelFamily::Illusioner => [3, 4],
    }
}

/// Applies the vanilla half-amplitude leg swing ([`half_amplitude_leg_swing_pose`])
/// to a colored `IllagerModel`/`VillagerModel`/`WitchModel` layer's two leg parts at
/// `leg_indices`. Borrows the static parts unchanged at rest
/// (`walkAnimationSpeed == 0`). The arm/nose poses and the illager riding sit pose
/// are left to the deferred animations.
fn half_amplitude_limb_swing_parts(
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
            leg.pose = half_amplitude_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
        }
    }
    Cow::Owned(owned)
}

fn emit_quadruped_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: QuadrupedModelFamily,
    baby: bool,
) {
    if family == QuadrupedModelFamily::Pig {
        emit_pig_model(mesh, instance, PigModelVariant::Temperate, baby);
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

fn emit_pig_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    variant: PigModelVariant,
    baby: bool,
) {
    let parts = colored_head_look_parts(
        pig_model_parts(variant, baby),
        pig_head_part_index(baby),
        instance.render_state.head_yaw,
        instance.render_state.head_pitch,
    );
    let parts = quadruped_limb_swing_parts(
        parts,
        QUADRUPED_LEG_PART_INDICES,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts(mesh, &parts, entity_model_root_transform(instance));
}

fn emit_creeper_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `CreeperModel` is a custom `EntityModel`, but its `setupAnim` leg swing
    // is exactly the `QuadrupedModel` formula (`cos(pos * 0.6662 [+ π]) * 1.4 * speed`,
    // hind-right/front-left in phase), so the shared quadruped swing applies. Legs are
    // at [2, 3, 4, 5]. The swelling scale and powered charge layer are deferred.
    let parts = quadruped_limb_swing_parts(
        head_first_colored_head_look_parts(&CREEPER_PARTS, instance),
        QUADRUPED_LEG_PART_INDICES,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts(mesh, &parts, entity_model_root_transform(instance));
}

fn emit_spider_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `SpiderModel.setupAnim` sweeps each of the eight legs about its yRot and
    // steps it about its zRot after the head look (`spider_leg_swing_pose`).
    let parts = spider_limb_swing_parts(
        head_first_colored_head_look_parts(&SPIDER_PARTS, instance),
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts(mesh, &parts, entity_model_root_transform(instance));
}

fn emit_cave_spider_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // The cave spider shares `SpiderModel`, so it sweeps and steps its legs identically;
    // only the root transform (a smaller scale) differs.
    let parts = spider_limb_swing_parts(
        head_first_colored_head_look_parts(&SPIDER_PARTS, instance),
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts(mesh, &parts, cave_spider_model_root_transform(instance));
}

/// Applies the vanilla `SpiderModel.setupAnim` walking swing ([`spider_leg_swing_pose`])
/// to a colored spider layer's eight leg parts. Borrows the static parts unchanged at
/// rest (`walkAnimationSpeed == 0`).
fn spider_limb_swing_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    limb_swing: f32,
    limb_swing_amount: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if limb_swing_at_rest(limb_swing_amount) {
        return parts;
    }
    let mut owned = parts.into_owned();
    for (index, phase, side_sign) in spider_leg_swing_roles() {
        owned[index].pose = spider_leg_swing_pose(
            owned[index].pose,
            phase,
            side_sign,
            limb_swing,
            limb_swing_amount,
        );
    }
    Cow::Owned(owned)
}

fn emit_enderman_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `EndermanModel extends HumanoidModel`: `setupAnim` runs `super.setupAnim`
    // (the inherited arm and leg swing) then halves and clamps both the arms and the
    // legs to `[-0.4, 0.4]` (`enderman_arm_swing_pose`/`enderman_leg_swing_pose`). Arms
    // are at [2, 3], legs at [4, 5]. The carried-block arm pose and the creepy attack
    // pose are deferred.
    let parts = enderman_limb_swing_parts(
        head_first_colored_head_look_parts(&ENDERMAN_PARTS, instance),
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts(mesh, &parts, entity_model_root_transform(instance));
}

/// Applies the vanilla `EndermanModel.setupAnim` arm and leg swing
/// ([`enderman_arm_swing_pose`]/[`enderman_leg_swing_pose`]: the inherited
/// `HumanoidModel` swing, halved and clamped to `[-0.4, 0.4]`) to a colored enderman
/// layer's two arm parts at `[2, 3]` and two leg parts at `[4, 5]`. Borrows the static
/// parts unchanged at rest (`walkAnimationSpeed == 0`).
fn enderman_limb_swing_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    limb_swing: f32,
    limb_swing_amount: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if limb_swing_at_rest(limb_swing_amount) {
        return parts;
    }
    let mut owned = parts.into_owned();
    for index in HUMANOID_ARM_PART_INDICES {
        owned[index].pose =
            enderman_arm_swing_pose(owned[index].pose, limb_swing, limb_swing_amount);
    }
    for index in HUMANOID_LEG_PART_INDICES {
        owned[index].pose =
            enderman_leg_swing_pose(owned[index].pose, limb_swing, limb_swing_amount);
    }
    Cow::Owned(owned)
}

fn emit_iron_golem_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `IronGolemModel.setupAnim` swings the legs `±1.5 * triangleWave(pos, 13)
    // * speed` and, in the default (non-attack, non-flower) branch, the arms
    // `(-0.2 ± 1.5 * triangleWave(pos, 13)) * speed`, after the full head look. The
    // attack swing and the offer-flower arm pose are deferred event animations.
    let parts = iron_golem_walk_parts(
        head_first_colored_head_look_parts(&IRON_GOLEM_PARTS, instance),
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    emit_model_parts(mesh, &parts, entity_model_root_transform(instance));
}

/// Applies the vanilla `IronGolemModel.setupAnim` walking limb swing
/// ([`iron_golem_walk_pose`]) to a colored iron golem layer's arm and leg parts.
/// Borrows the static parts unchanged at rest (`walkAnimationSpeed == 0`).
fn iron_golem_walk_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    limb_swing: f32,
    limb_swing_amount: f32,
) -> Cow<'_, [ModelPartDesc]> {
    if limb_swing_at_rest(limb_swing_amount) {
        return parts;
    }
    let mut owned = parts.into_owned();
    for (index, part) in iron_golem_walk_part_roles() {
        owned[index].pose =
            iron_golem_walk_pose(owned[index].pose, limb_swing, limb_swing_amount, part);
    }
    Cow::Owned(owned)
}

fn emit_snow_golem_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `SnowGolemModel.setupAnim` looks the head, twists the middle snow ball by a
    // quarter of the head yaw (`upperBody.yRot = headYaw * 0.25`), and orbits the two
    // stick arms around that twist (`leftArm.yRot = upperBodyYRot`, `rightArm.yRot =
    // upperBodyYRot + π`, with `x`/`z` recomputed from cos/sin). The arm orbit overwrites
    // the body-layer `x`/`z` even at rest, so the parts are always rebuilt.
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let upper_body_yrot = snow_golem_upper_body_yrot(head_yaw);
    let mut parts = SNOW_GOLEM_PARTS;
    parts[SNOW_GOLEM_HEAD_PART_INDEX].pose =
        head_look_pose(parts[SNOW_GOLEM_HEAD_PART_INDEX].pose, head_yaw, head_pitch);
    parts[SNOW_GOLEM_UPPER_BODY_PART_INDEX].pose = snow_golem_upper_body_pose(
        parts[SNOW_GOLEM_UPPER_BODY_PART_INDEX].pose,
        upper_body_yrot,
    );
    parts[SNOW_GOLEM_LEFT_ARM_PART_INDEX].pose = snow_golem_arm_pose(
        parts[SNOW_GOLEM_LEFT_ARM_PART_INDEX].pose,
        upper_body_yrot,
        false,
    );
    parts[SNOW_GOLEM_RIGHT_ARM_PART_INDEX].pose = snow_golem_arm_pose(
        parts[SNOW_GOLEM_RIGHT_ARM_PART_INDEX].pose,
        upper_body_yrot,
        true,
    );
    emit_model_parts(mesh, &parts, entity_model_root_transform(instance));
}

/// Applies the vanilla `setupAnim` head look to a standalone head-first colored
/// model's head part (index 0): creeper, spider/cave spider, enderman, iron
/// golem, snow golem.
fn head_first_colored_head_look_parts(
    parts: &[ModelPartDesc],
    instance: EntityModelInstance,
) -> Cow<'_, [ModelPartDesc]> {
    colored_head_look_parts(
        parts,
        head_first_part_index(),
        instance.render_state.head_yaw,
        instance.render_state.head_pitch,
    )
}

fn emit_minecart_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `MinecartModel.createBodyLayer()`: the floor panel plus four boxed-in wall
    // panels. There is no `setupAnim`, so the cart is static; the shared `MINECART_PARTS`
    // back both render paths so the colored and textured geometry stay identical.
    emit_model_parts(mesh, &MINECART_PARTS, entity_model_root_transform(instance));
}

fn emit_boat_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: BoatModelFamily,
    chest: bool,
) {
    let transform = boat_model_root_transform(instance);
    if family == BoatModelFamily::Bamboo {
        emit_model_parts(mesh, &RAFT_COMMON_PARTS, transform);
        if chest {
            emit_model_parts(mesh, &RAFT_CHEST_PARTS, transform);
        }
    } else {
        emit_model_parts(mesh, &BOAT_COMMON_PARTS, transform);
        if chest {
            emit_model_parts(mesh, &BOAT_CHEST_PARTS, transform);
        }
    }
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
