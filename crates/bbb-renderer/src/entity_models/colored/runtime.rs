use std::borrow::Cow;

use glam::{Mat4, Vec3};

use super::super::catalog::{sheep_wool_render_color, *};
use super::super::geometry::*;
use super::super::instances::EntityModelInstance;
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
    boat_model_root_transform, cave_spider_model_root_transform, entity_model_root_transform,
    magma_cube_model_root_transform, mesh_transformer_scaled_model_root_transform,
    player_model_root_transform, polar_bear_model_root_transform, scaled_model_root_transform,
    slime_model_root_transform, villager_adult_model_root_transform,
    wither_skeleton_model_root_transform, HUSK_SCALE,
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
                    emit_model_parts(
                        &mut mesh,
                        chicken_model_parts(variant, baby),
                        entity_model_root_transform(*instance),
                    );
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
            } => emit_armor_stand_model(
                &mut mesh,
                *instance,
                small,
                show_arms,
                show_base_plate,
                pose,
            ),
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
            EntityModelKind::Zombie { baby } => emit_zombie_model(&mut mesh, *instance, baby),
            EntityModelKind::ZombieVariant { family, baby } => {
                emit_zombie_variant_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Piglin { family, baby } => {
                emit_piglin_model(&mut mesh, *instance, family, baby)
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
            EntityModelKind::Wolf { baby, .. } => {
                if !skip_texture_backed_entities {
                    emit_wolf_model(&mut mesh, *instance, baby);
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
                emit_camel_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Llama {
                family,
                variant,
                baby,
                has_chest,
            } => emit_llama_model(&mut mesh, *instance, family, variant, baby, has_chest),
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
            EntityModelKind::Illager { family } => emit_illager_model(&mut mesh, *instance, family),
            EntityModelKind::Minecart => emit_minecart_model(&mut mesh, *instance),
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

fn emit_player_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, slim: bool) {
    let transform = player_model_root_transform(instance);
    let parts: &[ModelPartDesc] = if slim {
        &PLAYER_SLIM_PARTS
    } else {
        &PLAYER_WIDE_PARTS
    };
    emit_model_parts(
        mesh,
        &colored_head_look_parts(
            parts,
            player_head_part_index(),
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        ),
        transform,
    );
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
    emit_model_parts(
        mesh,
        &zombie_colored_head_look_parts(parts, instance, baby),
        entity_model_root_transform(instance),
    );
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
    emit_model_parts_with_color(
        mesh,
        &zombie_colored_head_look_parts(parts, instance, baby),
        transform,
        color,
    );
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
    emit_model_parts_with_color(
        mesh,
        &colored_head_look_parts(
            parts,
            piglin_head_part_index(baby_layout),
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        ),
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
    emit_model_parts_with_color(
        mesh,
        if baby {
            &BABY_HOGLIN_PARTS
        } else {
            &ADULT_HOGLIN_PARTS
        },
        entity_model_root_transform(instance),
        hoglin_model_color(family),
    );
}

fn emit_ravager_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(mesh, &RAVAGER_PARTS, entity_model_root_transform(instance));
}

fn emit_skeleton_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &skeleton_colored_head_look_parts(&SKELETON_PARTS, skeleton_head_part_index(), instance),
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
            &skeleton_colored_head_look_parts(
                &SKELETON_PARTS,
                skeleton_head_part_index(),
                instance,
            ),
            entity_model_root_transform(instance),
        ),
        SkeletonModelFamily::Parched => emit_model_parts(
            mesh,
            &skeleton_colored_head_look_parts(&PARCHED_PARTS, parched_head_part_index(), instance),
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
                &skeleton_colored_head_look_parts(parts, skeleton_head_part_index(), instance),
                entity_model_root_transform(instance),
            )
        }
        SkeletonModelFamily::WitherSkeleton => emit_model_parts_with_color(
            mesh,
            &skeleton_colored_head_look_parts(
                &SKELETON_PARTS,
                skeleton_head_part_index(),
                instance,
            ),
            wither_skeleton_model_root_transform(instance),
            WITHER_SKELETON_DARK,
        ),
    }
}

/// Applies the vanilla `HumanoidModel.setupAnim` head look to a skeleton-family
/// layer's head part at `head_index`.
fn skeleton_colored_head_look_parts(
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

fn emit_cow_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    variant: CowModelVariant,
    baby: bool,
) {
    emit_model_parts(
        mesh,
        &colored_head_look_parts(
            cow_model_parts(variant, baby),
            cow_head_part_index(baby),
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        ),
        entity_model_root_transform(instance),
    );
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
    let base_parts: &[ModelPartDesc] = if baby {
        &BABY_SHEEP_PARTS
    } else {
        &ADULT_SHEEP_PARTS
    };
    emit_model_parts(
        mesh,
        &sheep_colored_head_parts(base_parts, baby, head_eat, head_yaw, head_pitch),
        transform,
    );
    let wool_layer_color = sheep_wool_render_color(wool_color, jeb, age_ticks);
    if !invisible && !baby && (jeb || wool_color != SheepWoolColor::White) {
        emit_model_parts_with_color(
            mesh,
            &sheep_colored_head_parts(&ADULT_SHEEP_PARTS, baby, head_eat, head_yaw, head_pitch),
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
        emit_model_parts_with_color(
            mesh,
            &sheep_colored_head_parts(wool_parts, baby, head_eat, head_yaw, head_pitch),
            transform,
            wool_layer_color,
        );
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
    emit_model_parts(
        mesh,
        &villager_colored_head_look_parts(parts, villager_head_part_index(baby), instance),
        transform,
    );
}

fn emit_wandering_trader_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &villager_colored_head_look_parts(
            &ADULT_VILLAGER_PARTS,
            villager_head_part_index(false),
            instance,
        ),
        villager_adult_model_root_transform(instance),
    );
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

fn emit_wolf_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    let parts: &[ModelPartDesc] = if baby {
        &BABY_WOLF_PARTS
    } else {
        &ADULT_WOLF_PARTS
    };
    emit_model_parts(
        mesh,
        &head_first_colored_head_look_parts(parts, instance),
        entity_model_root_transform(instance),
    );
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
    emit_goat_parts(
        mesh,
        parts,
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
    if stand_scale == 0.0 {
        emit_model_parts(mesh, static_parts, transform);
    } else {
        let mut parts = static_parts.to_vec();
        for (index, part) in polar_bear_standing_part_roles(baby) {
            apply_polar_bear_standing_pose(&mut parts[index].pose, part, baby, stand_scale);
        }
        emit_model_parts(mesh, &parts, transform);
    }
}

fn emit_witch_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &villager_colored_head_look_parts(&WITCH_PARTS, villager_head_part_index(false), instance),
        villager_adult_model_root_transform(instance),
    );
}

fn emit_illager_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: IllagerModelFamily,
) {
    emit_model_parts(
        mesh,
        &villager_colored_head_look_parts(
            illager_model_parts(family),
            villager_head_part_index(false),
            instance,
        ),
        villager_adult_model_root_transform(instance),
    );
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
    for (x, z) in [(-leg_x, 7.0), (leg_x, 7.0), (-leg_x, -5.0), (leg_x, -5.0)] {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: [x, 24.0 - leg_size, z],
                    rotation: [0.0, 0.0, 0.0],
                }),
            ModelCubeDesc {
                min: [-2.0, 0.0, -2.0],
                size: [4.0, leg_size, 4.0],
                color,
            },
        );
    }
}

fn emit_pig_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    variant: PigModelVariant,
    baby: bool,
) {
    emit_model_parts(
        mesh,
        &colored_head_look_parts(
            pig_model_parts(variant, baby),
            pig_head_part_index(baby),
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        ),
        entity_model_root_transform(instance),
    );
}

fn emit_creeper_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &head_first_colored_head_look_parts(&CREEPER_PARTS, instance),
        entity_model_root_transform(instance),
    );
}

fn emit_spider_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &head_first_colored_head_look_parts(&SPIDER_PARTS, instance),
        entity_model_root_transform(instance),
    );
}

fn emit_cave_spider_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &head_first_colored_head_look_parts(&SPIDER_PARTS, instance),
        cave_spider_model_root_transform(instance),
    );
}

fn emit_enderman_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &head_first_colored_head_look_parts(&ENDERMAN_PARTS, instance),
        entity_model_root_transform(instance),
    );
}

fn emit_iron_golem_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &head_first_colored_head_look_parts(&IRON_GOLEM_PARTS, instance),
        entity_model_root_transform(instance),
    );
}

fn emit_snow_golem_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &head_first_colored_head_look_parts(&SNOW_GOLEM_PARTS, instance),
        entity_model_root_transform(instance),
    );
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
    let transform = entity_model_root_transform(instance);
    for (min, size, pose) in [
        (
            [-10.0, -8.0, -1.0],
            [20.0, 16.0, 2.0],
            PartPose {
                offset: [0.0, 4.0, 0.0],
                rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [-9.0, 4.0, 0.0],
                rotation: [0.0, std::f32::consts::PI * 1.5, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [9.0, 4.0, 0.0],
                rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [0.0, 4.0, -7.0],
                rotation: [0.0, std::f32::consts::PI, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [0.0, 4.0, 7.0],
                rotation: [0.0, 0.0, 0.0],
            },
        ),
    ] {
        emit_model_cube(
            mesh,
            transform * part_pose_transform(pose),
            ModelCubeDesc {
                min,
                size,
                color: MINECART_GRAY,
            },
        );
    }
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
