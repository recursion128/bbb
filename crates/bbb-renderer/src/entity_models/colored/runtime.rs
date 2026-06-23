#[cfg(test)]
use std::borrow::Cow;

use glam::{Mat4, Vec3};

use super::super::catalog::{sheep_wool_render_color, *};
use super::super::geometry::*;
use super::super::instances::EntityModelInstance;
use super::super::model::{EntityModel, StaticModel};
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
                FrogModel::new().prepare_and_render(
                    &mut mesh,
                    instance,
                    entity_model_root_transform(*instance),
                );
            }
            EntityModelKind::Creaking => {
                // Colored-only so far (no texture-backed creaking yet), so this arm always emits.
                CreakingModel::new().prepare_and_render(
                    &mut mesh,
                    instance,
                    entity_model_root_transform(*instance),
                );
            }
            EntityModelKind::Sniffer => {
                // Colored-only so far (no texture-backed sniffer yet), so this arm always emits.
                SnifferModel::new().prepare_and_render(
                    &mut mesh,
                    instance,
                    entity_model_root_transform(*instance),
                );
            }
            EntityModelKind::Warden => {
                // Colored-only so far (no texture-backed warden yet), so this arm always emits.
                WardenModel::new().prepare_and_render(
                    &mut mesh,
                    instance,
                    entity_model_root_transform(*instance),
                );
            }
            EntityModelKind::Armadillo { baby, rolled_up } => {
                // Colored-only so far (no texture-backed armadillo yet), so this arm always emits.
                ArmadilloModel::new(baby, rolled_up).prepare_and_render(
                    &mut mesh,
                    instance,
                    entity_model_root_transform(*instance),
                );
            }
            EntityModelKind::Axolotl { baby } => {
                // Colored-only so far (no texture-backed axolotl yet), so this arm always emits.
                AxolotlModel::new(baby).prepare_and_render(
                    &mut mesh,
                    instance,
                    entity_model_root_transform(*instance),
                );
            }
            EntityModelKind::Tadpole => {
                // Colored-only so far (no texture-backed tadpole yet), so this arm always emits.
                TadpoleModel::new().prepare_and_render(
                    &mut mesh,
                    instance,
                    entity_model_root_transform(*instance),
                );
            }
            EntityModelKind::Parrot => {
                // Colored-only so far (no texture-backed parrot yet), so this arm always emits.
                ParrotModel::new().prepare_and_render(
                    &mut mesh,
                    instance,
                    entity_model_root_transform(*instance),
                );
            }
            EntityModelKind::Shulker => {
                // Colored-only so far (no texture-backed shulker yet), so this arm always emits.
                ShulkerModel::new().prepare_and_render(
                    &mut mesh,
                    instance,
                    entity_model_root_transform(*instance),
                );
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
                // `GiantZombieModel extends ZombieModel`: the same non-baby zombie body and
                // `setupAnim`, baked through `MeshTransformer.scaling(6.0)`.
                ZombieModel::new(false).prepare_and_render(
                    &mut mesh,
                    instance,
                    mesh_transformer_scaled_model_root_transform(*instance, GIANT_SCALE),
                );
            }
            EntityModelKind::EndCrystal => {
                // Colored-only so far (no texture-backed end crystal yet), so this arm always emits.
                emit_end_crystal_model(&mut mesh, *instance);
            }
            EntityModelKind::EvokerFangs => {
                // Static (the bite open/close, base drop, and emerge scale are deferred); colored-only.
                StaticModel::new(&EVOKER_FANGS_PARTS).prepare_and_render(
                    &mut mesh,
                    instance,
                    evoker_fangs_model_root_transform(*instance),
                );
            }
            EntityModelKind::LeashKnot => {
                // Static (vanilla `LeashKnotModel` has no `setupAnim`); colored-only.
                StaticModel::new(&LEASH_KNOT_PARTS).prepare_and_render(
                    &mut mesh,
                    instance,
                    leash_knot_model_root_transform(*instance),
                );
            }
            EntityModelKind::Arrow => {
                // Static (the impact-shake wobble is deferred); colored-only.
                StaticModel::new(&ARROW_PARTS).prepare_and_render(
                    &mut mesh,
                    instance,
                    arrow_model_root_transform(*instance),
                );
            }
            EntityModelKind::Trident => {
                // Static (vanilla `TridentModel` has no animation); colored-only.
                StaticModel::new(&TRIDENT_PARTS).prepare_and_render(
                    &mut mesh,
                    instance,
                    trident_model_root_transform(*instance),
                );
            }
            EntityModelKind::LlamaSpit => {
                // Static (vanilla `LlamaSpitModel` has no `setupAnim`); colored-only.
                StaticModel::new(&LLAMA_SPIT_PARTS).prepare_and_render(
                    &mut mesh,
                    instance,
                    llama_spit_model_root_transform(*instance),
                );
            }
            EntityModelKind::ShulkerBullet => {
                // Static (the age-driven tumble and the outer-shell pass are deferred); colored-only.
                StaticModel::new(&SHULKER_BULLET_PARTS).prepare_and_render(
                    &mut mesh,
                    instance,
                    shulker_bullet_model_root_transform(*instance),
                );
            }
            EntityModelKind::WindCharge => {
                // Colored-only so far (no texture-backed wind charge yet), so this arm always emits.
                WindChargeModel::new().prepare_and_render(
                    &mut mesh,
                    instance,
                    wind_charge_model_root_transform(*instance),
                );
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
                    SkeletonModel::new(None).prepare_and_render(
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
    // The unified `SlimeModel` (inner body) and `SlimeOuterModel` (translucent shell) trees drive both
    // render paths; both `setup_anim`s are no-ops (the squish stretch is deferred). The colored
    // fallback draws both layers under one transform, reproducing the combined `SLIME_PARTS` mesh.
    let transform = slime_model_root_transform(instance, size);
    SlimeModel::new().prepare_and_render(mesh, &instance, transform);
    SlimeOuterModel::new().prepare_and_render(mesh, &instance, transform);
}

fn emit_vex_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // The unified `VexModel` tree drives both render paths; `setup_anim` runs the vanilla
    // `VexModel.setupAnim` pose (head look, charging body level / idle tilt, charging arm raise / idle
    // hold, wing flap). The held-item arm variant is deferred. Vex uses `LivingEntityRenderer.setupRotations`.
    VexModel::new().prepare_and_render(mesh, &instance, entity_model_root_transform(instance));
}

fn emit_allay_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // The unified `AllayModel` tree drives both render paths; `setup_anim` runs the vanilla
    // `AllayModel.setupAnim` non-dancing, empty-handed idle / flying pose (root bob, head look, body
    // flying tilt, arm idle bob, wing flap). The dance pose and held-item arms are deferred. Allay
    // uses `LivingEntityRenderer.setupRotations`.
    AllayModel::new().prepare_and_render(mesh, &instance, entity_model_root_transform(instance));
}

fn emit_strider_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    // The unified `StriderModel` tree drives both render paths; `setup_anim` swings/rolls/lifts the
    // legs, sways and bobs the body (tracking the look), and flows the bristles (adult six on `zRot`,
    // baby three on `xRot`) with the walk plus an `ageInTicks` ripple. The ridden pose, saddle layer,
    // and cold texture are deferred. Strider uses `LivingEntityRenderer.setupRotations`.
    StriderModel::new(baby).prepare_and_render(
        mesh,
        &instance,
        entity_model_root_transform(instance),
    );
}

fn emit_turtle_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    // The unified `TurtleModel` tree drives both render paths; `setup_anim` tracks the head look and
    // swings the legs (`TurtleModel.setupAnim` land walk / water paddle), and shows the adult
    // `egg_belly` overlay when `hasEgg`. When the adult carries an egg, vanilla also drops the whole
    // model `root.y--`; that lives in the root transform here. Turtle uses
    // `LivingEntityRenderer.setupRotations`.
    let has_egg = !baby && instance.render_state.turtle_has_egg;
    let mut root = entity_model_root_transform(instance);
    if has_egg {
        root *= part_pose_transform(TURTLE_EGG_ROOT_DROP_POSE);
    }
    TurtleModel::new(baby).prepare_and_render(mesh, &instance, root);
}

fn emit_bat_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // The unified `BatModel` tree drives both render paths; `setup_anim` samples the looping
    // `BatAnimation.BAT_FLYING` wing flap / body bob, or, while `isResting`, the `BAT_RESTING` hanging
    // pose with the head turned by the look yaw. The exact animation start tick is deferred. Bat uses
    // `LivingEntityRenderer.setupRotations`.
    BatModel::new().prepare_and_render(mesh, &instance, entity_model_root_transform(instance));
}

fn emit_bee_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    // The unified `BeeModel` tree drives both render paths; `setup_anim` flaps the wings, rocks the
    // non-angry `bobUpAndDown` (bone/legs/antennae), splays the legs to `π/4` while airborne, and
    // hides the stinger once the bee has stung. The rolled-up fall pose (`rollAmount`) is deferred
    // entity-side state. Bee uses `LivingEntityRenderer.setupRotations`.
    BeeModel::new(baby).prepare_and_render(mesh, &instance, entity_model_root_transform(instance));
}

fn emit_breeze_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // The unified `BreezeModel` tree drives both render paths; `setup_anim` samples the looping
    // `BreezeAnimation.IDLE` from `age_in_ticks` (head bob + rods spin). The translucent wind layer,
    // the emissive eyes, and the shoot/slide/inhale/jump action animations are deferred entity-side
    // state. Breeze uses `LivingEntityRenderer.setupRotations`.
    BreezeModel::new().prepare_and_render(mesh, &instance, entity_model_root_transform(instance));
}

fn emit_dolphin_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    // The unified `DolphinModel` tree drives both render paths; `setup_anim` steers the body by the
    // look pitch/yaw and adds the swim body tilt and tail/tail-fin wave while moving. The baby uses
    // the `MeshTransformer.scaling(0.5)` body layer; the held-item carry layer is deferred. Dolphin
    // uses `LivingEntityRenderer.setupRotations`.
    let root = mesh_transformer_scaled_model_root_transform(instance, if baby { 0.5 } else { 1.0 });
    DolphinModel::new().prepare_and_render(mesh, &instance, root);
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

fn emit_ender_dragon_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // Vanilla `EnderDragonModel` is a deep static hierarchy at its bind layout (head/jaw, the neck
    // and tail spine segments, the body with wings and legs). The whole `setupAnim` is procedural
    // (the flight-history neck/tail placement, the wing flap, the jaw, the root bounce) and deferred,
    // so the bind-pose part tree is emitted directly at the `EnderDragonRenderer` transform.
    let root = ender_dragon_model_root_transform(instance);
    emit_model_parts(mesh, &ENDER_DRAGON_PARTS, root);
}

fn emit_phantom_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, size: i32) {
    // The unified `PhantomModel` tree drives both render paths; `setup_anim` flaps the nested
    // wing/tail chains from `flapTime` (`id*3 + ageInTicks`), while the body and head hold their rest
    // tilt. The size scale and body pitch live in the root transform.
    let root = phantom_model_root_transform(instance, size);
    PhantomModel::new().prepare_and_render(mesh, &instance, root);
}

fn emit_pufferfish_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    puff_state: i32,
) {
    // The unified `PufferfishModel` tree drives both render paths; `new` picks the small/mid/big parts
    // by puff state and `setup_anim` wiggles its two fins on `ageInTicks`. The body bob lives in the
    // pufferfish root transform.
    let root = pufferfish_model_root_transform(instance);
    PufferfishModel::new(puff_state).prepare_and_render(mesh, &instance, root);
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

fn emit_skeleton_variant_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: SkeletonModelFamily,
) {
    // The unified `SkeletonModel` tree (selected by family) drives both render paths; `setup_anim` runs
    // the shared humanoid head look + arm/leg walk swing. The clothing / mushroom overlay is a
    // textured-only pass, so the colored fallback renders only the base body. The wither skeleton reuses
    // the plain mesh with the dark tint and its own root transform.
    let mut model = SkeletonModel::new(Some(family));
    if family == SkeletonModelFamily::WitherSkeleton {
        model.prepare_and_render_with_color(
            mesh,
            &instance,
            wither_skeleton_model_root_transform(instance),
            WITHER_SKELETON_DARK,
        );
    } else {
        model.prepare_and_render(mesh, &instance, entity_model_root_transform(instance));
    }
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
/// now swing their legs through [`apply_humanoid_leg_swing`] directly, so this `Cow`-slice variant
/// is retained only as the reference the leg-phase unit tests assert against.
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
    invisible: bool,
    jeb: bool,
    age_ticks: f32,
) {
    // The unified `SheepModel` (body) and `SheepFurModel` (wool) trees drive both render paths; both
    // run the shared `SheepModel.setupAnim` (leg swing + eat-grass head pose). The colored fallback
    // renders the body with baked colors, optionally recolors the body undercoat (non-white adult),
    // then renders the wool tinted (unless sheared). Invisible sheep render the body only.
    let transform = entity_model_root_transform(instance);
    let wool_layer_color = sheep_wool_render_color(wool_color, jeb, age_ticks);
    let mut body = SheepModel::new(baby);
    body.prepare(&instance);
    body.root().render_colored(mesh, transform);
    if !invisible && !baby && (jeb || wool_color != SheepWoolColor::White) {
        body.root()
            .render_colored_with_color(mesh, transform, wool_layer_color);
    }
    if !invisible && !sheared {
        let mut fur = SheepFurModel::new(baby);
        fur.prepare(&instance);
        fur.root()
            .render_colored_with_color(mesh, transform, wool_layer_color);
    }
}

fn emit_wolf_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    baby: bool,
    angry: bool,
) {
    // The unified `WolfModel` tree drives both render paths; `setup_anim` looks the head, folds the
    // `setSittingPose` or swings the four legs at the `QuadrupedModel` diagonal phase, then sets the
    // tail `xRot = tailAngle` + wag `yRot` (angry → the raised constant, no wag). The water-shake body
    // roll is deferred. The colored fallback renders the baked wolf-gray tree.
    WolfModel::new(baby, angry).prepare_and_render(
        mesh,
        &instance,
        entity_model_root_transform(instance),
    );
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
    // The unified `SpiderModel` tree drives both render paths; `setup_anim` looks the head and
    // sweeps/steps the eight legs once.
    SpiderModel::new().prepare_and_render(mesh, &instance, entity_model_root_transform(instance));
}

fn emit_cave_spider_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // The cave spider shares `SpiderModel`, differing only by its smaller root transform.
    SpiderModel::new().prepare_and_render(
        mesh,
        &instance,
        cave_spider_model_root_transform(instance),
    );
}

fn emit_enderman_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    // The unified `EndermanModel` tree drives both render paths; `setup_anim` looks the head, swings
    // the clamped arms/legs, overrides the arms when carrying a block, and applies the creepy
    // head/hat shift.
    EndermanModel::new().prepare_and_render(mesh, &instance, entity_model_root_transform(instance));
}

fn emit_boat_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: BoatModelFamily,
    chest: bool,
) {
    // The unified `BoatModel` tree drives both render paths; `new` selects the boat / raft / chest tree.
    // The boat is a static mesh (the vanilla paddle swing is deferred), so the colored fallback just
    // renders the baked wood-colored tree at its bind pose.
    BoatModel::new(family, chest).prepare_and_render(
        mesh,
        &instance,
        boat_model_root_transform(instance),
    );
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
