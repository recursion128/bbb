use std::borrow::Cow;

use glam::{Mat4, Vec3};

use super::super::catalog::{sheep_wool_render_color, *};
use super::super::geometry::*;
use super::super::instances::EntityModelInstance;
use super::super::keyframe::*;
use super::super::model::EntityModel;
use super::super::model_layers::*;
use super::armor_stand::emit_armor_stand_model;
use super::mounts::{
    emit_camel_model, emit_donkey_model, emit_horse_model, emit_llama_model,
    emit_undead_horse_model,
};
use super::selection::{
    hoglin_model_color, humanoid_model_color, piglin_model_color, quadruped_model_color,
};
use super::transforms::{
    arrow_model_root_transform, boat_model_root_transform, cave_spider_model_root_transform,
    cod_model_root_transform, creeper_model_root_transform, end_crystal_model_root_transform,
    ender_dragon_model_root_transform, entity_model_root_transform,
    evoker_fangs_model_root_transform, ghast_model_root_transform,
    happy_ghast_model_root_transform, leash_knot_model_root_transform,
    llama_spit_model_root_transform, magma_cube_model_root_transform,
    mesh_transformer_scaled_model_root_transform, phantom_model_root_transform,
    player_model_root_transform, polar_bear_model_root_transform, pufferfish_model_root_transform,
    salmon_model_root_transform, scaled_model_root_transform, shulker_bullet_model_root_transform,
    slime_model_root_transform, squid_model_root_transform, trident_model_root_transform,
    tropical_fish_model_root_transform, villager_adult_model_root_transform,
    wind_charge_model_root_transform, wither_skeleton_model_root_transform, GIANT_SCALE,
    HUSK_SCALE,
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
                    ChickenModel::new(variant, baby).prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::Pig { variant, baby } => {
                if !skip_texture_backed_entities {
                    PigModel::new(variant, baby).prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::Player { slim, .. } => {
                if !skip_texture_backed_entities {
                    PlayerModel::new(slim).prepare_and_render(
                        &mut mesh,
                        instance,
                        player_model_root_transform(*instance),
                    );
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
                    MagmaCubeModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        magma_cube_model_root_transform(*instance, size),
                    );
                }
            }
            EntityModelKind::Ghast => {
                if !skip_texture_backed_entities {
                    GhastModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        ghast_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::HappyGhast => {
                if !skip_texture_backed_entities {
                    HappyGhastModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        happy_ghast_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::Blaze => {
                if !skip_texture_backed_entities {
                    BlazeModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::Endermite => {
                if !skip_texture_backed_entities {
                    EndermiteModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::Silverfish => {
                if !skip_texture_backed_entities {
                    SilverfishModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
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
            EntityModelKind::Armadillo { baby, rolled_up } => {
                // Colored-only so far (no texture-backed armadillo yet), so this arm always emits.
                emit_armadillo_model(&mut mesh, *instance, baby, rolled_up);
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
                // First entity on the mutable `ModelPart` tree: build, run `setup_anim`, render.
                WitherModel::new().prepare_and_render(
                    &mut mesh,
                    instance,
                    entity_model_root_transform(*instance),
                );
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
            EntityModelKind::LlamaSpit => {
                // Colored-only so far (no texture-backed llama spit yet), so this arm always emits.
                emit_llama_spit_model(&mut mesh, *instance);
            }
            EntityModelKind::ShulkerBullet => {
                // Colored-only so far (no texture-backed shulker bullet yet), so this arm always emits.
                emit_shulker_bullet_model(&mut mesh, *instance);
            }
            EntityModelKind::WindCharge => {
                // Colored-only so far (no texture-backed wind charge yet), so this arm always emits.
                emit_wind_charge_model(&mut mesh, *instance);
            }
            EntityModelKind::EnderDragon => {
                // Colored-only so far (no texture-backed ender dragon yet), so this arm always emits.
                emit_ender_dragon_model(&mut mesh, *instance);
            }
            EntityModelKind::NoRender => {
                // Vanilla `NoopRenderer` entities (area effect cloud, marker, interaction) render no
                // model, so this arm emits nothing — exact parity with vanilla.
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
                    ZombieModel::new(baby).prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
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
                    RavagerModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::Skeleton => {
                if !skip_texture_backed_entities {
                    SkeletonModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::SkeletonVariant { family } => {
                if !skip_texture_backed_entities {
                    emit_skeleton_variant_model(&mut mesh, *instance, family)
                }
            }
            EntityModelKind::Cow { variant, baby } => {
                if !skip_texture_backed_entities {
                    CowModel::new(variant, baby).prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
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
                    let transform = if baby {
                        entity_model_root_transform(*instance)
                    } else {
                        villager_adult_model_root_transform(*instance)
                    };
                    VillagerModel::new(baby).prepare_and_render(&mut mesh, instance, transform);
                }
            }
            EntityModelKind::WanderingTrader => {
                if !skip_texture_backed_entities {
                    WanderingTraderModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        villager_adult_model_root_transform(*instance),
                    );
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
                    GoatModel::new(baby, left_horn, right_horn).prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::PolarBear { baby } => {
                if !skip_texture_backed_entities {
                    let transform = if baby {
                        entity_model_root_transform(*instance)
                    } else {
                        polar_bear_model_root_transform(*instance)
                    };
                    PolarBearModel::new(baby).prepare_and_render(&mut mesh, instance, transform);
                }
            }
            EntityModelKind::Quadruped { family, baby } => {
                emit_quadruped_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Creeper => {
                if !skip_texture_backed_entities {
                    CreeperModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        creeper_model_root_transform(*instance),
                    );
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
                    IronGolemModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::SnowGolem => {
                if !skip_texture_backed_entities {
                    SnowGolemModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::Witch => {
                if !skip_texture_backed_entities {
                    WitchModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        villager_adult_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::Squid { glow, baby } => {
                if !skip_texture_backed_entities {
                    emit_squid_model(&mut mesh, *instance, glow, baby);
                }
            }
            EntityModelKind::Cod => {
                if !skip_texture_backed_entities {
                    let in_water = instance.render_state.in_water;
                    CodModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        cod_model_root_transform(*instance, in_water),
                    );
                }
            }
            EntityModelKind::Salmon { size } => {
                if !skip_texture_backed_entities {
                    let in_water = instance.render_state.in_water;
                    SalmonModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        salmon_model_root_transform(*instance, in_water, size),
                    );
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
                    IllagerModel::new(instance, family).prepare_and_render(
                        &mut mesh,
                        instance,
                        villager_adult_model_root_transform(*instance),
                    );
                }
            }
            EntityModelKind::Minecart => {
                if !skip_texture_backed_entities {
                    MinecartModel::new().prepare_and_render(
                        &mut mesh,
                        instance,
                        entity_model_root_transform(*instance),
                    );
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

fn emit_vex_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `VexModel.setupAnim`: the head tracks the look angles and the wings flap on
    // `ageInTicks`. While idle the arms hold `±π/5` with a small `ageInTicks` z-bob and the
    // body tilts `π/20`; while `Vex.isCharging` the body levels (`xRot = 0`) and
    // `setArmsCharging` raises both arms (the both-hands-empty branch — held items are not
    // projected, so the held-item arm variant `xRot = π·7/6` stays deferred). The arms and
    // wings hang under the body, so the body tilt carries them; the hierarchy is walked by
    // hand. Vex uses the standard `LivingEntityRenderer.setupRotations`.
    let age = instance.render_state.age_in_ticks;
    let charging = instance.render_state.vex_charging;
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

    // Body (child of root) levels while charging, else holds the idle tilt; it carries the
    // arms and wings.
    let body_pose = PartPose {
        offset: VEX_BODY_POSE.offset,
        rotation: [if charging { 0.0 } else { VEX_BODY_X_ROT }, 0.0, 0.0],
    };
    let body_t = root * part_pose_transform(body_pose);
    emit_model_cubes_at_pose(mesh, root, body_pose, &VEX_BODY);

    let bob = vex_moving_arm_z_bob(age);
    let (right_arm_rot, left_arm_rot) = if charging {
        (
            [
                VEX_ARM_CHARGING_X_ROT,
                VEX_ARM_CHARGING_Y_ROT,
                -VEX_ARM_CHARGING_Z_ROT - bob,
            ],
            [
                VEX_ARM_CHARGING_X_ROT,
                -VEX_ARM_CHARGING_Y_ROT,
                VEX_ARM_CHARGING_Z_ROT + bob,
            ],
        )
    } else {
        (
            [0.0, 0.0, VEX_ARM_REST_Z_ROT + bob],
            [0.0, 0.0, -(VEX_ARM_REST_Z_ROT + bob)],
        )
    };
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: VEX_RIGHT_ARM_POSE.offset,
            rotation: right_arm_rot,
        },
        &VEX_RIGHT_ARM,
    );
    emit_model_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: VEX_LEFT_ARM_POSE.offset,
            rotation: left_arm_rot,
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
    // of the root, so each is posed from the root transform. When the adult turtle carries an egg
    // (`hasEgg`), `AdultTurtleModel.setupAnim` shows the `egg_belly` overlay shell and drops the
    // whole model `root.y--` by one unit. The egg-laying leg amplitude (`isLayingEgg`) stays
    // deferred. Turtle uses `LivingEntityRenderer.setupRotations`.
    let pos = instance.render_state.walk_animation_pos;
    let speed = instance.render_state.walk_animation_speed;
    let on_land = !instance.render_state.in_water && instance.render_state.on_ground;
    // Only the adult model carries the egg belly; the baby model class has no such part.
    let has_egg = !baby && instance.render_state.turtle_has_egg;
    // The egg-laying front-leg amplitude lives in the shared `TurtleModel` (adult + baby).
    let laying = instance.render_state.turtle_laying_egg;
    let mut root = entity_model_root_transform(instance);
    if has_egg {
        // Vanilla `root.y--`: a model-local one-unit drop applied to every part (egg and all).
        root *= part_pose_transform(TURTLE_EGG_ROOT_DROP_POSE);
    }
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
    // The `egg_belly` overlay shell shares the body pose; only the adult model has it (the
    // projection clears `hasEgg` for babies).
    if has_egg {
        emit_model_cubes_at_pose(mesh, root, TURTLE_BODY_POSE, &TURTLE_EGG_BELLY);
    }

    for (cubes, leg_pose, front, right) in legs {
        emit_model_cubes_at_pose(
            mesh,
            root,
            PartPose {
                offset: leg_pose.offset,
                rotation: turtle_leg_rotation(pos, speed, on_land, front, right, laying),
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
    // phase offset is imperceptible for a continuous flap). While `isResting` the model swaps
    // to the static `BAT_RESTING` hanging pose (head/body flipped 180° about X, wings folded)
    // and `applyHeadRotation` turns the head by the look yaw. The wings/tips and feet hang
    // under the body and the ears under the head, so the hierarchy is walked by hand. Bat uses
    // `LivingEntityRenderer.setupRotations`.
    let resting = instance.render_state.bat_resting;
    let animation = if resting { &BAT_RESTING } else { &BAT_FLYING };
    let head_look_yaw = if resting {
        instance.render_state.head_yaw.to_radians()
    } else {
        0.0
    };
    let seconds = keyframe_elapsed_seconds(animation, instance.render_state.age_in_ticks * 0.05);
    let sample = |bone: &str| sample_bone_offsets(animation, bone, seconds, 1.0);
    let root = entity_model_root_transform(instance);

    // Head (root child) carries the two ears at their bind poses. While resting the head also
    // turns by the look yaw (`applyHeadRotation`, additive to the pose's `yRot`).
    let (head_pos, head_rot) = sample("head");
    let head_rot = [head_rot[0], head_rot[1] + head_look_yaw, head_rot[2]];
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

    // Each wing (body child) carries its tip; the resting pose also shifts the wings by a
    // position channel (`+1` z), so sample and apply the wing positions too.
    let (right_wing_pos, right_wing_rot) = sample("right_wing");
    let right_wing_pose =
        keyframe_animated_pose(BAT_RIGHT_WING_POSE, right_wing_pos, right_wing_rot);
    let right_wing_t = body_t * part_pose_transform(right_wing_pose);
    emit_model_cubes_at_pose(mesh, body_t, right_wing_pose, &BAT_RIGHT_WING);
    let (_, right_tip_rot) = sample("right_wing_tip");
    emit_model_cubes_at_pose(
        mesh,
        right_wing_t,
        keyframe_animated_pose(BAT_RIGHT_WING_TIP_POSE, [0.0; 3], right_tip_rot),
        &BAT_RIGHT_WING_TIP,
    );

    let (left_wing_pos, left_wing_rot) = sample("left_wing");
    let left_wing_pose = keyframe_animated_pose(BAT_LEFT_WING_POSE, left_wing_pos, left_wing_rot);
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
    // ground the model rests at its bind pose. An angry bee (`isAngry`) keeps flapping but skips
    // the `bobUpAndDown`, and the stinger cube is hidden once the bee has stung (`stinger.visible
    // = hasStinger`, gated below). The rolled-up fall pose (`rollAmount`) is deferred entity-side
    // state. The body (carrying the stinger and antennae), the wings, and the legs hang under the
    // `bone` pivot, so the hierarchy is walked by hand. Bee uses `LivingEntityRenderer.setupRotations`.
    let age = instance.render_state.age_in_ticks;
    let flying = !instance.render_state.on_ground;
    // Vanilla gates `bobUpAndDown` on `!isAngry && !isOnGround`: an angry airborne bee still
    // flaps its wings and splays its legs to `π/4`, but its body, front/back legs and antennae
    // hold still instead of rocking.
    let bob = flying && !instance.render_state.bee_angry;
    let root = entity_model_root_transform(instance);

    // Bone pivot (root child): the airborne bob rocks it forward and lifts/drops it.
    let bone_bind = if baby {
        BEE_BABY_BONE_POSE
    } else {
        BEE_BONE_POSE
    };
    let bone_pose = if bob {
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
    // The stinger cube is drawn only while the bee still carries it (`stinger.visible`).
    if instance.render_state.bee_has_stinger {
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
    }
    if !baby {
        let antenna_x_rot = if bob { bee_antenna_x_rot(age) } else { 0.0 };
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

    // Legs (bone children): airborne, all three splay to `π/4`; the non-angry bob then overrides
    // the front/back pair, while an angry bee holds all three at `π/4`.
    let (front_x, mid_x, back_x) = if flying {
        (
            if bob {
                bee_front_leg_x_rot(age)
            } else {
                BEE_MID_LEG_FLYING_X_ROT
            },
            BEE_MID_LEG_FLYING_X_ROT,
            if bob {
                bee_back_leg_x_rot(age)
            } else {
                BEE_MID_LEG_FLYING_X_ROT
            },
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
    // the root). `setupAnim` sets `head.yRot/xRot` from the plain look, and since every part is a
    // child of `head` the whole guardian turns with it — reproduced by folding `head_look_pose`
    // into `head_t`. The spike age pulse + `spikesAnimation` withdrawal, the eye tracking, the tail
    // sway, and the attack beam are deferred, so those stay at their rest pose.
    let scale = if elder { GUARDIAN_ELDER_SCALE } else { 1.0 };
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let base_root = mesh_transformer_scaled_model_root_transform(instance, scale);
    let head_t = if head_look_at_rest(head_yaw, head_pitch) {
        base_root
    } else {
        base_root * part_pose_transform(head_look_pose(PART_POSE_ZERO, head_yaw, head_pitch))
    };

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
    // Vanilla `FrogModel` is a nested hierarchy (`root` → body/legs, body → head/tongue/arms).
    // `setupAnim` applies the looping `FROG_WALK` keyframe cycle via
    // `applyWalk(walkAnimationPos, walkAnimationSpeed, 1.5, 2.5)`: the walk position drives the sample
    // time and the speed scales the amplitude (so a still frog samples the cycle's rest frame). The
    // animation offsets the `body` (rotation), the two arms (`body` children), and the two legs
    // (`root` children), so the spine is hand-walked. The jump / croak / tongue / in-water swim+idle
    // keyframe animations need un-projected `AnimationState`s and stay deferred. Frogs use
    // `LivingEntityRenderer.setupRotations`.
    let root_transform = entity_model_root_transform(instance);
    let (seconds, scale) = keyframe_walk_sample(
        &FROG_WALK,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
        FROG_WALK_SPEED_FACTOR,
        FROG_WALK_SCALE_FACTOR,
    );
    let animated = |bone: &str, bind: PartPose| {
        let (position, rotation) = sample_bone_offsets(&FROG_WALK, bone, seconds, scale);
        keyframe_animated_pose(bind, position, rotation)
    };

    let root = &FROG_PARTS[0];
    let root_t = root_transform * part_pose_transform(root.pose);

    // `body` (root child 0): a rotation channel, carrying head/tongue/arms beneath it.
    let body = &root.children[0];
    let body_t = root_t * part_pose_transform(animated("body", body.pose));
    for cube in body.cubes {
        emit_model_cube(mesh, body_t, *cube);
    }
    // head (0) and tongue (1) are not animated by the walk.
    emit_model_part(mesh, &body.children[0], body_t);
    emit_model_part(mesh, &body.children[1], body_t);
    // The two arms (body children 2/3) take their own rotation + position offsets, carrying the
    // webbed hands beneath them.
    for (index, bone) in [(2, "left_arm"), (3, "right_arm")] {
        let arm = &body.children[index];
        let posed = ModelPartDesc {
            pose: animated(bone, arm.pose),
            ..*arm
        };
        emit_model_part(mesh, &posed, body_t);
    }
    // The two legs (root children 1/2) take their offsets, carrying the feet beneath them.
    for (index, bone) in [(1, "left_leg"), (2, "right_leg")] {
        let leg = &root.children[index];
        let posed = ModelPartDesc {
            pose: animated(bone, leg.pose),
            ..*leg
        };
        emit_model_part(mesh, &posed, root_t);
    }
}

/// Hand-walks the camel's colored mesh through its walk ([`CamelWalkLayout::walk`]), composing the
/// walk onto the clamped head look. Vanilla `CamelModel.setupAnim` samples the walk via
/// `applyWalk(walkAnimationPos, walkAnimationSpeed, 2.0, 2.5)`: the `root` channel rolls the whole
/// model, the four legs swing (rotation + position), the `head` adds a pitch (and, for the baby, a
/// position nudge) onto the look, the two ears flap, the tail swishes, and the baby `body` dips. A
/// still camel samples amplitude 0, collapsing to the bind pose plus the head look. The `layout`
/// carries the per-variant head/tail/ear/leg indices, so this serves the adult, husk, and baby.
pub(in crate::entity_models::colored) fn emit_camel_walk_colored(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    transform: Mat4,
    color: [f32; 4],
    parts: &[ModelPartDesc],
    layout: &CamelWalkLayout,
    head_yaw: f32,
    head_pitch: f32,
) {
    let (seconds, scale) = keyframe_walk_sample(
        layout.walk,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
        CAMEL_WALK_SPEED_FACTOR,
        CAMEL_WALK_SCALE_FACTOR,
    );
    let animated = |bone: &str, bind: PartPose| {
        let (position, rotation) = sample_bone_offsets(layout.walk, bone, seconds, scale);
        keyframe_animated_pose(bind, position, rotation)
    };

    // `root` rolls the whole model: no bind offset/rotation, so the z-sway applies at the entity root.
    let root_t = transform * part_pose_transform(animated("root", PART_POSE_ZERO));

    // `body` (root child 0): not animated on the adult; the baby walk dips it via a `body` position.
    let body = &parts[0];
    let body_t = root_t * part_pose_transform(animated("body", body.pose));
    for cube in body.cubes {
        emit_model_cube_with_color(mesh, body_t, *cube, color);
    }

    // The body's children: the head (clamped look + walk), the tail (walk swish), and — on the adult —
    // the static hump. Iterating in declared order preserves the depth-first emit order.
    for (index, child) in body.children.iter().enumerate() {
        if index == layout.head_child {
            let (head_walk_pos, head_walk_rot) =
                sample_bone_offsets(layout.walk, "head", seconds, scale);
            let head_pose = PartPose {
                offset: [
                    child.pose.offset[0] + head_walk_pos[0],
                    child.pose.offset[1] + head_walk_pos[1],
                    child.pose.offset[2] + head_walk_pos[2],
                ],
                rotation: [
                    head_pitch.to_radians() + head_walk_rot[0],
                    head_yaw.to_radians() + head_walk_rot[1],
                    child.pose.rotation[2] + head_walk_rot[2],
                ],
            };
            let head_t = body_t * part_pose_transform(head_pose);
            for cube in child.cubes {
                emit_model_cube_with_color(mesh, head_t, *cube, color);
            }
            for (ear_index, ear_bone) in layout.ears {
                let ear = &child.children[ear_index];
                emit_model_part_with_color(
                    mesh,
                    &ModelPartDesc {
                        pose: animated(ear_bone, ear.pose),
                        ..*ear
                    },
                    head_t,
                    color,
                );
            }
        } else if index == layout.tail_child {
            emit_model_part_with_color(
                mesh,
                &ModelPartDesc {
                    pose: animated("tail", child.pose),
                    ..*child
                },
                body_t,
                color,
            );
        } else {
            // The adult hump (static).
            emit_model_part_with_color(mesh, child, body_t, color);
        }
    }

    // The four legs (root children 1..=4): the walk rotation + position.
    for (index, bone) in layout.legs {
        let leg = &parts[index];
        emit_model_part_with_color(
            mesh,
            &ModelPartDesc {
                pose: animated(bone, leg.pose),
                ..*leg
            },
            root_t,
            color,
        );
    }
}

fn emit_creaking_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `CreakingModel` is a nested hierarchy (`root` → upper_body/legs, upper_body →
    // head/body/arms). `setupAnim` sets `head.xRot/yRot` from the plain look, then (while `canMove`)
    // applies the looping `CREAKING_WALK` via `applyWalk(walkAnimationPos, walkAnimationSpeed, 1, 1)`,
    // which offsets the upper body, head (ADDING onto the look), arms, and legs. The `canMove` freeze
    // gate is un-projected, but a frozen creaking has walk speed ≈ 0 so the amplitude already
    // collapses to rest; the attack / invulnerable / death keyframe animations stay deferred. The
    // spine is hand-walked. Creaking uses `LivingEntityRenderer.setupRotations`.
    let root_transform = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let (seconds, scale) = keyframe_walk_sample(
        &CREAKING_WALK,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
        1.0,
        1.0,
    );
    let animated = |bone: &str, bind: PartPose| {
        let (position, rotation) = sample_bone_offsets(&CREAKING_WALK, bone, seconds, scale);
        keyframe_animated_pose(bind, position, rotation)
    };

    let root = &CREAKING_PARTS[0];
    let root_t = root_transform * part_pose_transform(root.pose);

    // `upper_body` (root child 0, empty pivot): the walk rotation, carrying head/body/arms.
    let upper_body = &root.children[0];
    let upper_t = root_t * part_pose_transform(animated("upper_body", upper_body.pose));

    // `head` (upper_body child 0): the look (set) plus the walk rotation (added). The walk has no
    // head position channel, so the bind offset is kept.
    let head = &upper_body.children[0];
    let (_, head_walk_rot) = sample_bone_offsets(&CREAKING_WALK, "head", seconds, scale);
    let head_pose = PartPose {
        offset: head.pose.offset,
        rotation: [
            head_pitch.to_radians() + head_walk_rot[0],
            head_yaw.to_radians() + head_walk_rot[1],
            head.pose.rotation[2] + head_walk_rot[2],
        ],
    };
    emit_model_part(
        mesh,
        &ModelPartDesc {
            pose: head_pose,
            ..*head
        },
        upper_t,
    );

    // `body` (upper_body child 1) is not animated by the walk.
    emit_model_part(mesh, &upper_body.children[1], upper_t);

    // The two arms (upper_body children 2/3) take their walk rotation.
    for (index, bone) in [(2, "right_arm"), (3, "left_arm")] {
        let arm = &upper_body.children[index];
        emit_model_part(
            mesh,
            &ModelPartDesc {
                pose: animated(bone, arm.pose),
                ..*arm
            },
            upper_t,
        );
    }

    // The two legs (root children 1/2) take their walk rotation + position.
    for (index, bone) in [(1, "left_leg"), (2, "right_leg")] {
        let leg = &root.children[index];
        emit_model_part(
            mesh,
            &ModelPartDesc {
                pose: animated(bone, leg.pose),
                ..*leg
            },
            root_t,
        );
    }
}

fn emit_sniffer_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `SnifferModel` is a nested hierarchy (`bone` → body/legs, body → head →
    // ears/nose/beak). `setupAnim` sets `head.xRot/yRot` from the plain look, then applies a walk:
    // while not searching it samples `SNIFFER_WALK` via `applyWalk(..., 9, 100)`, rocking the body,
    // the head (the walk pitch ADDS onto the look), the two ears, and the six legs. A still sniffer
    // samples amplitude 0, collapsing to the bind pose plus the head look. The `bone → body → head`
    // spine and the six legs are hand-walked. The search-walk variant (gated on the un-synced
    // `isSearching`) and the dig / long-sniff / stand-up / happy / scenting keyframe animations stay
    // deferred. Sniffer uses `LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let (seconds, scale) = keyframe_walk_sample(
        &SNIFFER_WALK,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
        SNIFFER_WALK_SPEED_FACTOR,
        SNIFFER_WALK_SCALE_FACTOR,
    );
    let animated = |bone: &str, bind: PartPose| {
        let (position, rotation) = sample_bone_offsets(&SNIFFER_WALK, bone, seconds, scale);
        keyframe_animated_pose(bind, position, rotation)
    };

    // `bone` (the lone root, no cubes) is not animated; it parents the body and the six legs.
    let bone = &SNIFFER_PARTS[0];
    let bone_t = root * part_pose_transform(bone.pose);

    // `body` (bone child 0): the walk sway/dip, carrying the head.
    let body = &bone.children[0];
    let body_t = bone_t * part_pose_transform(animated("body", body.pose));
    for cube in body.cubes {
        emit_model_cube(mesh, body_t, *cube);
    }

    // `head` (body child 0): the plain look (set) plus the walk pitch (added). The walk has no head
    // position channel, so the bind offset is kept.
    let head = &body.children[0];
    let (_, head_walk_rot) = sample_bone_offsets(&SNIFFER_WALK, "head", seconds, scale);
    let head_pose = PartPose {
        offset: head.pose.offset,
        rotation: [
            head_pitch.to_radians() + head_walk_rot[0],
            head_yaw.to_radians() + head_walk_rot[1],
            head.pose.rotation[2] + head_walk_rot[2],
        ],
    };
    let head_t = body_t * part_pose_transform(head_pose);
    for cube in head.cubes {
        emit_model_cube(mesh, head_t, *cube);
    }

    // The head's children: the two ears take a walk z-roll; the nose and lower beak ride the head.
    for (index, bone_name) in [
        (0, Some("left_ear")),
        (1, Some("right_ear")),
        (2, None),
        (3, None),
    ] {
        let child = &head.children[index];
        let pose = match bone_name {
            Some(name) => animated(name, child.pose),
            None => child.pose,
        };
        emit_model_part(mesh, &ModelPartDesc { pose, ..*child }, head_t);
    }

    // The six legs (bone children 1..=6) take their walk rotation + position.
    for (index, bone_name) in [
        (1, "right_front_leg"),
        (2, "right_mid_leg"),
        (3, "right_hind_leg"),
        (4, "left_front_leg"),
        (5, "left_mid_leg"),
        (6, "left_hind_leg"),
    ] {
        let leg = &bone.children[index];
        emit_model_part(
            mesh,
            &ModelPartDesc {
                pose: animated(bone_name, leg.pose),
                ..*leg
            },
            bone_t,
        );
    }
}

fn emit_warden_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `WardenModel` is a nested hierarchy (`bone` → body/legs, body → ribcages/head/arms,
    // head → tendrils). Four non-keyframe `setupAnim` motions are reproduced: the head look
    // (`animateHeadLookTarget` sets `head.xRot/yRot` from the look angles, so the head and its
    // tendrils track the target), the always-on idle wobble (`animateIdlePose` rolls the body
    // `±0.025` and the head `±0.06` off `ageInTicks`), the walk (`animateWalk` swings the head,
    // body, two legs, and two arms off `walkAnimationPos/Speed`), and the tendril sway
    // (`animateTendrils` swings the two head tendrils off the projected `tendrilAnimation` pulse and
    // `ageInTicks`). The walk offsets ADD onto the look/idle composition — addition is commutative,
    // so applying them after the look/idle pass through `warden_add_x_z_rot` preserves the vanilla
    // order. The attack / sonic-boom / digging / emerge / roar / sniff keyframe animations stay
    // deferred. Warden uses `LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let age = instance.render_state.age_in_ticks;
    let walk = warden_walk_pose(
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );

    // `bone` is the lone root part and carries no cubes; the two legs hang off it and swing with the walk.
    let bone = &WARDEN_PARTS[0];
    let bone_t = root * part_pose_transform(bone.pose);

    // `body` rolls with the idle wobble and the walk; its `head` child takes the look plus the idle
    // and walk head rolls; the two arms swing their `xRot` with the walk.
    let body = &bone.children[WARDEN_BODY_BONE_CHILD_INDEX];
    let body_pose = warden_add_x_z_rot(
        warden_idle_body_pose(body.pose, age),
        walk.body_x_rot,
        walk.body_z_rot,
    );
    let body_t = bone_t * part_pose_transform(body_pose);
    for cube in body.cubes {
        emit_model_cube(mesh, body_t, *cube);
    }
    let tendril_x = warden_tendril_x_rot(instance.render_state.tendril_animation, age);
    for (index, child) in body.children.iter().enumerate() {
        if index == WARDEN_HEAD_BODY_CHILD_INDEX {
            // The head takes the look + idle + walk roll; its two tendrils then sway their `xRot`
            // off the tendril pulse (`leftTendril += tendrilXRot`, `rightTendril -= tendrilXRot`),
            // so the head subtree is hand-walked to re-pose the tendrils under the moved head.
            let head_pose = warden_add_x_z_rot(
                warden_head_pose(child.pose, head_yaw, head_pitch, age),
                walk.head_x_rot,
                walk.head_z_rot,
            );
            let head_t = body_t * part_pose_transform(head_pose);
            for cube in child.cubes {
                emit_model_cube(mesh, head_t, *cube);
            }
            for (tendril_index, tendril) in child.children.iter().enumerate() {
                let x_rot = if tendril_index == WARDEN_RIGHT_TENDRIL_HEAD_CHILD_INDEX {
                    -tendril_x
                } else if tendril_index == WARDEN_LEFT_TENDRIL_HEAD_CHILD_INDEX {
                    tendril_x
                } else {
                    0.0
                };
                let tendril_posed = ModelPartDesc {
                    pose: warden_add_x_z_rot(tendril.pose, x_rot, 0.0),
                    ..*tendril
                };
                emit_model_part(mesh, &tendril_posed, head_t);
            }
            continue;
        }
        let pose = if index == WARDEN_RIGHT_ARM_BODY_CHILD_INDEX {
            warden_add_x_z_rot(child.pose, walk.right_arm_x_rot, 0.0)
        } else if index == WARDEN_LEFT_ARM_BODY_CHILD_INDEX {
            warden_add_x_z_rot(child.pose, walk.left_arm_x_rot, 0.0)
        } else {
            child.pose
        };
        emit_model_part(mesh, &ModelPartDesc { pose, ..*child }, body_t);
    }

    for (index, leg) in bone.children.iter().enumerate().skip(1) {
        let x_rot = if index == WARDEN_RIGHT_LEG_BONE_CHILD_INDEX {
            walk.right_leg_x_rot
        } else if index == WARDEN_LEFT_LEG_BONE_CHILD_INDEX {
            walk.left_leg_x_rot
        } else {
            0.0
        };
        let leg_posed = ModelPartDesc {
            pose: warden_add_x_z_rot(leg.pose, x_rot, 0.0),
            ..*leg
        };
        emit_model_part(mesh, &leg_posed, bone_t);
    }
}

fn emit_armadillo_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    baby: bool,
    rolled_up: bool,
) {
    // Vanilla `AdultArmadilloModel`/`BabyArmadilloModel` are nested hierarchies (root → body/legs,
    // body → tail/head, head → ears). When `isHidingInShell` (the synced `ArmadilloState.SCARED`),
    // `setupAnim` hides the body cubes (`skipDraw`), the tail, and both hind legs and shows the
    // shell-ball `cube` — the head, ears, and front legs stay drawn — so the rolled-up part tree is
    // emitted instead, with no head look. While NOT hiding, `setupAnim` sets the clamped head look
    // (`head.xRot/yRot` clamped to [-22.5, 25] / [-32.5, 32.5]) on the body-nested head pivot, then
    // `applyWalk` rocks the body, tail, four legs, and head as the armadillo moves. Both the adult
    // ([`ARMADILLO_WALK`]) and the baby ([`ARMADILLO_BABY_WALK`]) walks are hand-walked; the
    // roll-out / roll-up / peek keyframe transitions stay deferred. The baby flag (synced
    // `AgeableMob.DATA_BABY_ID`) selects the baby body layer, as in the vanilla `AgeableMobRenderer`.
    // Armadillo uses `LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    if rolled_up {
        let parts: &[ModelPartDesc] = if baby {
            &BABY_ARMADILLO_ROLLED_PARTS
        } else {
            &ADULT_ARMADILLO_ROLLED_PARTS
        };
        emit_model_parts(mesh, parts, root);
        return;
    }
    let (head_yaw, head_pitch) = armadillo_clamped_head_look(
        instance.render_state.head_yaw,
        instance.render_state.head_pitch,
    );
    let (parts, walk): (&[ModelPartDesc], &AnimationDefinition) = if baby {
        (&BABY_ARMADILLO_PARTS, &ARMADILLO_BABY_WALK)
    } else {
        (&ADULT_ARMADILLO_PARTS, &ARMADILLO_WALK)
    };
    emit_armadillo_walk(mesh, instance, root, parts, walk, head_yaw, head_pitch);
}

fn emit_armadillo_walk(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    root: Mat4,
    parts: &[ModelPartDesc],
    walk: &AnimationDefinition,
    head_yaw: f32,
    head_pitch: f32,
) {
    // The walk is sampled via `applyWalk(walkAnimationPos, walkAnimationSpeed, 16.5, 2.5)`: the `body`
    // rolls (a CatmullRom z-sway with a small y-bob) carrying the tail and head, the four legs swing
    // (rotation + position), the `tail` rocks, and the `head` channel adds a z-roll onto the clamped
    // look. A still armadillo samples amplitude 0, collapsing to the bind pose plus the head look. The
    // adult and baby share this `body → tail/head` + four-leg topology, so one hand-walk serves both.
    let (seconds, scale) = keyframe_walk_sample(
        walk,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
        ARMADILLO_WALK_SPEED_FACTOR,
        ARMADILLO_WALK_SCALE_FACTOR,
    );
    let animated = |bone: &str, bind: PartPose| {
        let (position, rotation) = sample_bone_offsets(walk, bone, seconds, scale);
        keyframe_animated_pose(bind, position, rotation)
    };

    // `body` (root child 0): the walk roll/bob, carrying the tail and head.
    let body = &parts[0];
    let body_t = root * part_pose_transform(animated("body", body.pose));
    for cube in body.cubes {
        emit_model_cube(mesh, body_t, *cube);
    }

    // `tail` (body child 0): the walk rock added onto its bind pitch (the baby tail also carries its
    // stub cube, which has no walk channel of its own).
    let tail = &body.children[0];
    emit_model_part(
        mesh,
        &ModelPartDesc {
            pose: animated("tail", tail.pose),
            ..*tail
        },
        body_t,
    );

    // `head` (body child 1): the clamped look (set) plus the walk z-roll (added). The walk has no
    // head position channel, so the bind offset is kept.
    let head = &body.children[1];
    let (_, head_walk_rot) = sample_bone_offsets(walk, "head", seconds, scale);
    let head_pose = PartPose {
        offset: head.pose.offset,
        rotation: [
            head_pitch.to_radians() + head_walk_rot[0],
            head_yaw.to_radians() + head_walk_rot[1],
            head.pose.rotation[2] + head_walk_rot[2],
        ],
    };
    emit_model_part(
        mesh,
        &ModelPartDesc {
            pose: head_pose,
            ..*head
        },
        body_t,
    );

    // The four legs (root children 1..=4) take their walk rotation + position.
    for (index, bone) in [
        (1, "right_hind_leg"),
        (2, "left_hind_leg"),
        (3, "right_front_leg"),
        (4, "left_front_leg"),
    ] {
        let leg = &parts[index];
        emit_model_part(
            mesh,
            &ModelPartDesc {
                pose: animated(bone, leg.pose),
                ..*leg
            },
            root,
        );
    }
}

fn emit_axolotl_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    // Vanilla `AdultAxolotlModel`/`BabyAxolotlModel` are nested hierarchies rooted at the `body`
    // part. `AdultAxolotlModel.setupAnim` first turns the whole body toward the look target —
    // `body.yRot += yRot·π/180` — unconditionally, before the factor-blended swimming / hovering /
    // crawling / lay-still / play-dead sways; that body yaw is reproduced here on the adult root
    // body. The blended procedural sways, the mirror-leg copy, and the baby keyframe animations stay
    // deferred. The baby flag (synced `AgeableMob.DATA_BABY_ID`) selects the baby body layer, as in
    // the vanilla `AgeableMobRenderer`. Axolotl uses `AgeableMobRenderer`/
    // `LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    if baby || head_yaw_at_rest(head_yaw) {
        let parts: &[ModelPartDesc] = if baby {
            &BABY_AXOLOTL_PARTS
        } else {
            &ADULT_AXOLOTL_PARTS
        };
        emit_model_parts(mesh, parts, root);
        return;
    }
    let mut parts = ADULT_AXOLOTL_PARTS.to_vec();
    parts[0].pose.rotation[1] += head_yaw.to_radians();
    emit_model_parts(mesh, &parts, root);
}

fn emit_tadpole_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `TadpoleModel` is two static sibling parts (body box, tail fin). `setupAnim` sways
    // only the tail fin's `yRot` ([`tadpole_tail_yrot`], from the projected `age_in_ticks` +
    // `in_water`). Tadpole uses a plain `MobRenderer`/`LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    let tail_yrot = tadpole_tail_yrot(
        instance.render_state.age_in_ticks,
        instance.render_state.in_water,
    );
    let mut parts = TADPOLE_PARTS.to_vec();
    parts[TADPOLE_TAIL_PART_INDEX].pose.rotation[1] = tail_yrot;
    emit_model_parts(mesh, &parts, root);
}

fn emit_parrot_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `ParrotModel` is seven static sibling parts (body, tail, wings, head with its beak /
    // crest children, legs) at their baked rest poses. The SITTING perch pose is projected:
    // `Parrot.isInSittingPose()` (the synced `TamableAnimal.DATA_FLAGS_ID` sitting bit) runs
    // `prepare(SITTING)`, which raises every part `y += 1.9`, folds the legs (`xRot += π/2`),
    // pitches the tail (`xRot += π/6`), and tucks the wings (`zRot = ±0.0873`); the `setupAnim`
    // `SITTING` branch then adds nothing more. `setupAnim` also sets `head.xRot/yRot` from the look
    // angles before the per-pose switch, so the head look applies at both projected poses (STANDING
    // and SITTING) — reproduced here on the top-level head part. The STANDING walk swing is also
    // reproduced: the legs add `xRot += cos(pos·0.6662 [+π])·1.4·speed` (left in phase, right out)
    // and the tail adds `xRot += cos(pos·0.6662)·0.3·speed` onto their baked pitch (the swing is
    // reached only through the STANDING fall-through, so a sitting parrot skips it). The wing flap
    // and the body/tail/head flap bob need the un-projected `flapAngle`, and the PARTY dance and
    // FLYING leg pitch are not projected, so they stay deferred. Parrot uses a plain
    // `MobRenderer`/`LivingEntityRenderer.setupRotations`.
    let root = entity_model_root_transform(instance);
    let sitting = instance.render_state.parrot_sitting;
    let mut parts = parrot_pose_parts(sitting);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    if !head_look_at_rest(head_yaw, head_pitch) {
        parts[PARROT_HEAD_PART_INDEX].pose =
            head_look_pose(parts[PARROT_HEAD_PART_INDEX].pose, head_yaw, head_pitch);
    }
    let walk_pos = instance.render_state.walk_animation_pos;
    let walk_speed = instance.render_state.walk_animation_speed;
    if !sitting && !limb_swing_at_rest(walk_speed) {
        parts[PARROT_TAIL_PART_INDEX].pose =
            parrot_tail_swing_pose(parts[PARROT_TAIL_PART_INDEX].pose, walk_pos, walk_speed);
        for index in PARROT_LEG_PART_INDICES {
            parts[index].pose = parrot_leg_swing_pose(parts[index].pose, walk_pos, walk_speed);
        }
    }
    emit_model_parts(mesh, &parts, root);
}

fn emit_shulker_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `ShulkerModel.setupAnim`: the lid (part 0) opens with the synced peek and the head
    // (part 2, a root sibling) tracks the look angles; the base holds still. With
    // `bs = (0.5 + peek)·π` the lid rises to `y = 16 + sin(bs)·8` (plus an `sin(ageInTicks·0.1)·0.7`
    // bob once `bs > π`, i.e. the lid is past half-open) and twists
    // `lid.yRot = (−1 + sin(bs))⁴ · π · 0.125` once `peek > 0.3`. At `peek = 0` the lid sits back at
    // its `y = 24` bind offset, so the closed pose equals the bind pose. The head look is
    // `head.xRot = xRot`, `head.yRot = (yHeadRot − 180 − yBodyRot)` — and that yaw is exactly the
    // already-projected `head_yaw − 180` (since `head_yaw = wrapDegrees(yHeadRot − yBodyRot)` and a
    // 360° offset is a no-op rotation). The `−180` is vanilla's cancel for
    // `ShulkerRenderer.setupRotations`' `bodyRot + 180`; bbb keeps the standard `180 − bodyRot`
    // root, whose floor-shulker orientation differs from vanilla's by exactly 180° about Y — invisible
    // on the 180°-symmetric square shell — so the literal head-vs-shell angle reproduces vanilla for
    // the floor (`attachFace = DOWN`) case. The non-floor attach-face rotation / body-yaw inversion
    // (and the Dinnerbone-negated head sign) stay deferred and the floor rest orientation is used.
    let (lid_y, lid_yrot) = shulker_lid_pose(
        instance.render_state.shulker_peek,
        instance.render_state.age_in_ticks,
    );
    let mut parts = SHULKER_PARTS.to_vec();
    parts[0].pose.offset[1] = lid_y;
    parts[0].pose.rotation[1] = lid_yrot;
    parts[2].pose.rotation[0] = instance.render_state.head_pitch.to_radians();
    parts[2].pose.rotation[1] = (instance.render_state.head_yaw - 180.0).to_radians();
    let root = entity_model_root_transform(instance);
    emit_model_parts(mesh, &parts, root);
}

fn emit_giant_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `GiantZombieModel` is the standard `HumanoidModel` (zombie) mesh, baked through
    // `humanoidBodyLayer.apply(MeshTransformer.scaling(6.0))` — i.e. the adult zombie body layer
    // scaled 6×, exactly the husk's `MeshTransformer` pattern but with the giant's 6.0 factor and no
    // baby variant. The head look, limb swing, and held-out `animateZombieArms` arm pose match
    // the zombie (`GiantZombieModel extends ZombieModel`, the giant extracts the same
    // `ZombieRenderState`); the `HumanoidArmorLayer`, the `ItemInHandLayer`, and the zombie texture
    // are deferred.
    let parts = humanoid_limb_swing_parts(
        zombie_colored_head_look_parts(&ADULT_ZOMBIE_PARTS, instance, false),
        HUMANOID_LEG_PART_INDICES,
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    );
    let parts = zombie_arm_held_out_parts(
        parts,
        HUMANOID_ARM_PART_INDICES,
        instance.render_state.is_aggressive,
        instance.render_state.age_in_ticks,
    );
    emit_model_parts(
        mesh,
        &parts,
        mesh_transformer_scaled_model_root_transform(instance, GIANT_SCALE),
    );
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

fn emit_llama_spit_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `LlamaSpitModel` is a single static cross of seven 2×2×2 boxes with no `setupAnim`, so
    // the bind-pose part is emitted directly at the `LlamaSpitRenderer` flight-oriented transform
    // (`translate(0, 0.15, 0) · Ry(yRot - 90) · Rz(xRot)`).
    let root = llama_spit_model_root_transform(instance);
    emit_model_parts(mesh, &LLAMA_SPIT_PARTS, root);
}

fn emit_shulker_bullet_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `ShulkerBulletModel` is a single `main` part of three interlocking slabs. The geometry
    // and the facing are emitted at the `ShulkerBulletRenderer` static transform (lift + flip/half
    // scale + the `setupAnim` yaw/pitch); the age-driven tumble and the translucent outer-shell pass
    // are deferred.
    let root = shulker_bullet_model_root_transform(instance);
    emit_model_parts(mesh, &SHULKER_BULLET_PARTS, root);
}

fn emit_wind_charge_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `WindChargeModel` is the `bone` root (no cubes) parenting the `wind` shell and the
    // `wind_charge` core. `setupAnim` counter-spins them off `ageInTicks`: `wind.yRot = age·16°` (a
    // set that overwrites the -π/4 bind) and `windCharge.yRot = -age·16°`. The bone carries no cubes,
    // so its two children are posed and emitted at the position-only `WindChargeRenderer` transform;
    // the translucent scrolling texture stays deferred.
    let root = wind_charge_model_root_transform(instance);
    let bone = &WIND_CHARGE_PARTS[0];
    let bone_t = root * part_pose_transform(bone.pose);
    let spin = wind_charge_spin_yrot(instance.render_state.age_in_ticks);
    let mut children = bone.children.to_vec();
    children[WIND_CHARGE_WIND_CHILD_INDEX].pose.rotation[1] = spin;
    children[WIND_CHARGE_CORE_CHILD_INDEX].pose.rotation[1] = -spin;
    emit_model_parts(mesh, &children, bone_t);
}

fn emit_ender_dragon_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `EnderDragonModel` is a deep static hierarchy at its bind layout (head/jaw, the neck
    // and tail spine segments, the body with wings and legs). The whole `setupAnim` is procedural
    // (the flight-history neck/tail placement, the wing flap, the jaw, the root bounce) and deferred,
    // so the bind-pose part tree is emitted directly at the `EnderDragonRenderer` transform.
    let root = ender_dragon_model_root_transform(instance);
    emit_model_parts(mesh, &ENDER_DRAGON_PARTS, root);
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
/// `huskScale` 1.0625, a `MeshTransformer.scaling` baked by `HuskRenderer`); the baby husk, the
/// drowned, and the zombie villager render at the unscaled humanoid root.
fn zombie_variant_root_transform(
    instance: EntityModelInstance,
    family: ZombieVariantModelFamily,
    baby: bool,
) -> Mat4 {
    if family == ZombieVariantModelFamily::Husk && !baby {
        mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE)
    } else {
        entity_model_root_transform(instance)
    }
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
            instance.render_state.age_in_ticks,
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
        instance.render_state.age_in_ticks,
    )
}

/// Vanilla `QuadrupedModel` leg part indices in the cow and pig body layers: the
/// head and body occupy slots `0` and `1` (in either order — the baby layers swap
/// them), then the four legs. The variants order the legs differently (adult layers
/// list them hind-first, baby layers front-first), so [`quadruped_limb_swing_parts`]
/// resolves each leg's phase from its offset rather than its slot.
pub(in crate::entity_models) const QUADRUPED_LEG_PART_INDICES: [usize; 4] = [2, 3, 4, 5];

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

/// Applies the vanilla `HumanoidModel.setupAnim` arm animation to a colored layer's two
/// arm parts at `arm_indices`: the walk swing ([`humanoid_arm_swing_pose`], only while
/// the limbs move) plus the always-on `ageInTicks` idle bob ([`humanoid_arm_bob_pose`]).
/// Because the idle bob advances every frame, the arms are always re-posed (the parts are
/// never borrowed unchanged). Callers whose subclass keeps the inherited default arms use
/// this (the player, the skeleton family, and the non-zombified piglin family); the
/// zombie / zombified-piglin constant arms-out poses (which carry their own bob) stay
/// deferred.
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

/// Applies the vanilla `ZombieModel.setupAnim` held-out arm pose
/// ([`zombie_arm_held_out_pose`]) to a colored zombie-family layer's two arm parts at
/// `arm_indices`, overriding the inherited walk arm swing. Always re-poses the arms (the
/// idle bob folded into the pose advances every frame).
pub(in crate::entity_models) fn zombie_arm_held_out_parts(
    parts: Cow<'_, [ModelPartDesc]>,
    arm_indices: [usize; 2],
    aggressive: bool,
    age_in_ticks: f32,
) -> Cow<'_, [ModelPartDesc]> {
    let mut owned = parts.into_owned();
    for index in arm_indices {
        if let Some(arm) = owned.get_mut(index) {
            arm.pose = zombie_arm_held_out_pose(arm.pose, aggressive, age_in_ticks);
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
    // are at [2, 3], legs at [4, 5]. Carrying a block then *overrides* both arms
    // (`enderman_carried_arm_pose`, held out front), and the creepy stare drops the head
    // `y -= 5` while raising its hat child `y += 5` (`ENDERMAN_HEAD_CHILDREN_CREEPY`).
    let mut parts = enderman_limb_swing_parts(
        head_first_colored_head_look_parts(&ENDERMAN_PARTS, instance),
        instance.render_state.walk_animation_pos,
        instance.render_state.walk_animation_speed,
    )
    .into_owned();
    if instance.render_state.enderman_carrying {
        for index in HUMANOID_ARM_PART_INDICES {
            parts[index].pose = enderman_carried_arm_pose(parts[index].pose);
        }
    }
    if instance.render_state.enderman_creepy {
        let head = &mut parts[head_first_part_index()];
        head.pose.offset[1] -= 5.0;
        head.children = &ENDERMAN_HEAD_CHILDREN_CREEPY;
    }
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
