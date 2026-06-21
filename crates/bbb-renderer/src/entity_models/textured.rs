use super::{
    boat_model_root_transform,
    catalog::{
        BoatModelFamily, ChickenModelVariant, CowModelVariant, EntityDyeColor, EntityModelKind,
        EntityModelTextureAtlasEntry, EntityModelTextureAtlasLayout, EntityModelTextureRef,
        HoglinModelFamily, PigModelVariant, PlayerModelPartVisibility, SheepWoolColor,
        SkeletonModelFamily,
    },
    cave_spider_model_root_transform, entity_model_root_transform,
    geometry::{
        emit_textured_model_cube, emit_textured_model_parts, fill_entity_textured_light,
        fill_entity_textured_overlay, part_pose_transform, EntityModelTexturedMesh,
        TexturedModelPartDesc,
    },
    instances::EntityModelInstance,
    magma_cube_model_root_transform,
    model_layers::{
        apply_polar_bear_standing_pose, cow_head_part_index, half_amplitude_leg_swing_pose,
        head_first_part_index, head_look_at_rest, head_look_pose, head_look_yaw_pose,
        head_yaw_at_rest, hoglin_head_part_index, hoglin_leg_swing_pose, humanoid_leg_swing_pose,
        limb_swing_at_rest, parched_head_part_index, pig_head_part_index, player_head_part_index,
        polar_bear_head_part_index, polar_bear_standing_part_roles, quadruped_leg_swing_pose,
        ravager_head_child_index, ravager_neck_part_index, sheep_head_at_rest,
        sheep_head_part_index, sheep_head_pose, skeleton_head_part_index, villager_head_part_index,
        ADULT_GOAT_HEAD_INDEX, BABY_GOAT_HEAD_INDEX, RAVAGER_TEXTURED_NECK_CHILDREN,
        RAVAGER_TEXTURED_PARTS,
    },
    player_model_root_transform, polar_bear_model_root_transform, slime_model_root_transform,
    villager_adult_model_root_transform, wither_skeleton_model_root_transform,
};
use glam::Mat4;

mod layers;

pub(super) use layers::{
    boat_textured_layer_passes, chicken_textured_layer_passes, cow_textured_layer_passes,
    creeper_textured_layer_passes, enderman_textured_layer_passes, goat_textured_layer_passes,
    hoglin_textured_layer_passes, iron_golem_textured_layer_passes,
    magma_cube_textured_layer_passes, pig_textured_layer_passes, player_textured_layer_passes,
    polar_bear_textured_layer_passes, ravager_textured_layer_passes, sheep_textured_layer_passes,
    skeleton_textured_layer_passes, slime_textured_layer_passes, snow_golem_textured_layer_passes,
    spider_textured_layer_passes, villager_textured_layer_passes,
    wandering_trader_textured_layer_passes, witch_textured_layer_passes,
    wolf_textured_layer_passes, EntityModelLayerPass, EntityModelLayerRenderType,
};
use layers::{goat_visible_textured_model_parts, player_visible_textured_model_parts};
#[cfg(test)]
pub(super) use layers::{EntityModelLayerKind, EntityModelLayerVisibility};

pub(super) struct EntityModelTexturedMeshes {
    pub(super) cutout: EntityModelTexturedMesh,
    pub(super) translucent: EntityModelTexturedMesh,
    pub(super) eyes: EntityModelTexturedMesh,
}

impl EntityModelTexturedMeshes {
    fn new() -> Self {
        Self {
            cutout: EntityModelTexturedMesh::new(),
            translucent: EntityModelTexturedMesh::new(),
            eyes: EntityModelTexturedMesh::new(),
        }
    }

    fn mesh_mut(
        &mut self,
        render_type: EntityModelLayerRenderType,
    ) -> &mut EntityModelTexturedMesh {
        match render_type {
            EntityModelLayerRenderType::Cutout => &mut self.cutout,
            EntityModelLayerRenderType::Translucent => &mut self.translucent,
            EntityModelLayerRenderType::Eyes => &mut self.eyes,
        }
    }
}

#[cfg(test)]
pub(super) fn entity_model_textured_mesh(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMesh {
    entity_model_textured_meshes(instances, atlas).cutout
}

pub(super) fn entity_model_textured_meshes(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new();
    for instance in instances {
        let cutout_start = meshes.cutout.vertices.len();
        let translucent_start = meshes.translucent.vertices.len();
        let eyes_start = meshes.eyes.vertices.len();
        match instance.kind {
            EntityModelKind::Chicken { variant, baby } => {
                emit_chicken_textured_model(&mut meshes, *instance, variant, baby, atlas);
            }
            EntityModelKind::Pig { variant, baby } => {
                emit_pig_textured_model(&mut meshes, *instance, variant, baby, atlas);
            }
            EntityModelKind::Cow { variant, baby } => {
                emit_cow_textured_model(&mut meshes, *instance, variant, baby, atlas);
            }
            EntityModelKind::Creeper => {
                emit_creeper_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Spider => {
                emit_spider_textured_model(&mut meshes, *instance, false, atlas);
            }
            EntityModelKind::CaveSpider => {
                emit_spider_textured_model(&mut meshes, *instance, true, atlas);
            }
            EntityModelKind::Enderman => {
                emit_enderman_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::IronGolem => {
                emit_iron_golem_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::SnowGolem => {
                emit_snow_golem_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Witch => {
                emit_witch_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Slime { size } => {
                emit_slime_textured_model(&mut meshes, *instance, size, atlas);
            }
            EntityModelKind::MagmaCube { size } => {
                emit_magma_cube_textured_model(&mut meshes, *instance, size, atlas);
            }
            EntityModelKind::PolarBear { baby } => {
                emit_polar_bear_textured_model(&mut meshes, *instance, baby, atlas);
            }
            EntityModelKind::Hoglin { family, baby } => {
                emit_hoglin_textured_model(&mut meshes, *instance, family, baby, atlas);
            }
            EntityModelKind::Ravager => {
                emit_ravager_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Villager { baby } => {
                emit_villager_textured_model(&mut meshes, *instance, baby, atlas);
            }
            EntityModelKind::WanderingTrader => {
                emit_wandering_trader_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Player { slim, parts } => {
                emit_player_textured_model(&mut meshes, *instance, slim, parts, atlas);
            }
            EntityModelKind::Sheep {
                baby,
                sheared,
                wool_color,
                invisible,
                jeb,
                age_ticks,
            } => {
                emit_sheep_textured_model(
                    &mut meshes,
                    *instance,
                    baby,
                    sheared,
                    wool_color,
                    invisible,
                    jeb,
                    age_ticks,
                    atlas,
                );
            }
            EntityModelKind::Wolf {
                baby,
                tame,
                angry,
                invisible,
                collar_color,
            } => {
                emit_wolf_textured_model(
                    &mut meshes,
                    *instance,
                    baby,
                    tame,
                    angry,
                    invisible,
                    collar_color,
                    atlas,
                );
            }
            EntityModelKind::Goat {
                baby,
                left_horn,
                right_horn,
            } => {
                emit_goat_textured_model(
                    &mut meshes,
                    *instance,
                    baby,
                    left_horn,
                    right_horn,
                    atlas,
                );
            }
            EntityModelKind::Skeleton => {
                emit_skeleton_textured_model(&mut meshes, *instance, None, atlas);
            }
            EntityModelKind::SkeletonVariant { family } => {
                emit_skeleton_textured_model(&mut meshes, *instance, Some(family), atlas);
            }
            EntityModelKind::Boat { family, chest } => {
                emit_boat_textured_model(&mut meshes, *instance, family, chest, atlas);
            }
            _ => {}
        }
        let light = instance.render_state.shader_light();
        fill_entity_textured_light(&mut meshes.cutout, cutout_start, light);
        fill_entity_textured_light(&mut meshes.translucent, translucent_start, light);
        fill_entity_textured_light(&mut meshes.eyes, eyes_start, light);
        let overlay = instance.render_state.overlay_coords();
        fill_entity_textured_overlay(&mut meshes.cutout, cutout_start, overlay);
        fill_entity_textured_overlay(&mut meshes.translucent, translucent_start, overlay);
        fill_entity_textured_overlay(&mut meshes.eyes, eyes_start, overlay);
    }
    meshes
}

fn emit_boat_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: BoatModelFamily,
    chest: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = boat_model_root_transform(instance);
    for pass in boat_textured_layer_passes(family, chest) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_chicken_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    variant: ChickenModelVariant,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in chicken_textured_layer_passes(variant, baby) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_pig_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    variant: PigModelVariant,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_quadruped_textured_passes(
        meshes,
        pig_textured_layer_passes(variant, baby),
        pig_head_part_index(baby),
        QUADRUPED_LEG_PART_INDICES,
        entity_model_root_transform(instance),
        instance,
        atlas,
    );
}

/// `QuadrupedModel` leg part indices in the cow and pig body layers (the head and
/// body occupy slots `0`/`1` in either order). [`quadruped_leg_swing_pose`] resolves
/// each leg's phase from its offset, so the differing leg order of the adult
/// (hind-first) and baby (front-first) layers does not matter.
const QUADRUPED_LEG_PART_INDICES: [usize; 4] = [2, 3, 4, 5];

fn emit_cow_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    variant: CowModelVariant,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_quadruped_textured_passes(
        meshes,
        cow_textured_layer_passes(variant, baby),
        cow_head_part_index(baby),
        QUADRUPED_LEG_PART_INDICES,
        entity_model_root_transform(instance),
        instance,
        atlas,
    );
}

/// Emits a quadruped's textured layer passes, applying the vanilla
/// `QuadrupedModel.setupAnim` head look ([`head_look_pose`]) to the head part at
/// `head_index` and the leg swing ([`quadruped_leg_swing_pose`]) to the four leg
/// parts at `leg_indices`. The static parts are reused unchanged while both the
/// head is level/aligned and the legs are at rest.
#[allow(clippy::too_many_arguments)]
fn emit_quadruped_textured_passes(
    meshes: &mut EntityModelTexturedMeshes,
    passes: Vec<EntityModelLayerPass>,
    head_index: usize,
    leg_indices: [usize; 4],
    transform: Mat4,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in passes {
        if head_resting && legs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            if !head_resting {
                if let Some(head) = parts.get_mut(head_index) {
                    head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
                }
            }
            if !legs_resting {
                for index in leg_indices {
                    if let Some(leg) = parts.get_mut(index) {
                        leg.pose =
                            quadruped_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                    }
                }
            }
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
}

/// `HumanoidModel` leg part indices in the skeleton-family body and clothing
/// layers: the head, body, and two arms occupy the lower slots (in either order),
/// then the right and left legs. [`humanoid_leg_swing_pose`] resolves each leg's
/// phase from its offset, so the parched layer's head/body swap does not matter.
const HUMANOID_LEG_PART_INDICES: [usize; 2] = [4, 5];

/// Emits a humanoid's textured layer passes, applying the vanilla
/// `HumanoidModel.setupAnim` head look ([`head_look_pose`]) to the head part at
/// `head_index` and the leg swing ([`humanoid_leg_swing_pose`]) to the two leg
/// parts at `leg_indices`. The static parts are reused unchanged while both the
/// head is level/aligned and the legs are at rest. The arms are left to each
/// subclass override (e.g. the skeleton aiming pose), which is deferred.
#[allow(clippy::too_many_arguments)]
fn emit_humanoid_textured_passes(
    meshes: &mut EntityModelTexturedMeshes,
    passes: Vec<EntityModelLayerPass>,
    head_index: usize,
    leg_indices: [usize; 2],
    transform: Mat4,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in passes {
        if head_resting && legs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            if !head_resting {
                if let Some(head) = parts.get_mut(head_index) {
                    head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
                }
            }
            if !legs_resting {
                for index in leg_indices {
                    if let Some(leg) = parts.get_mut(index) {
                        leg.pose = humanoid_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                    }
                }
            }
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
}

/// Right/left leg part indices in the adult villager / witch / wandering-trader
/// textured layers: the combined `arms` part is at slot `2`, then the legs at
/// `[3, 4]`.
const VILLAGER_ADULT_LEG_PART_INDICES: [usize; 2] = [3, 4];

/// Right/left leg part indices in the baby villager textured layer, which reorders
/// the parts and lists the legs at `[1, 2]`.
const VILLAGER_BABY_LEG_PART_INDICES: [usize; 2] = [1, 2];

/// Emits an `IllagerModel`/`VillagerModel`/`WitchModel` family entity's textured
/// layer passes, applying the vanilla head look ([`head_look_pose`]) to the head
/// part at `head_index` and the half-amplitude leg swing
/// ([`half_amplitude_leg_swing_pose`]) to the two leg parts at `leg_indices`. The
/// static parts are reused unchanged while both the head is level/aligned and the
/// legs are at rest. The combined `arms` part, the witch nose bob, and the villager
/// unhappy head shake are deferred.
#[allow(clippy::too_many_arguments)]
fn emit_villager_family_textured_passes(
    meshes: &mut EntityModelTexturedMeshes,
    passes: Vec<EntityModelLayerPass>,
    head_index: usize,
    leg_indices: [usize; 2],
    transform: Mat4,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in passes {
        if head_resting && legs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            if !head_resting {
                if let Some(head) = parts.get_mut(head_index) {
                    head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
                }
            }
            if !legs_resting {
                for index in leg_indices {
                    if let Some(leg) = parts.get_mut(index) {
                        leg.pose =
                            half_amplitude_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                    }
                }
            }
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
}

/// Emits textured layer passes, applying the vanilla `QuadrupedModel`/
/// `HumanoidModel.setupAnim` head look to each pass's head part at `head_index`.
/// The static parts are reused unchanged while the head is level and aligned
/// with the body. `transform` is taken explicitly so callers with a non-default
/// model root transform (e.g. the wither skeleton scale) stay correct.
fn emit_textured_passes_with_head_look(
    meshes: &mut EntityModelTexturedMeshes,
    passes: Vec<EntityModelLayerPass>,
    head_index: usize,
    transform: Mat4,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    for pass in passes {
        if head_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            if let Some(head) = parts.get_mut(head_index) {
                head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
            }
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
}

fn emit_creeper_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_textured_passes_with_head_look(
        meshes,
        creeper_textured_layer_passes(),
        head_first_part_index(),
        entity_model_root_transform(instance),
        instance,
        atlas,
    );
}

fn emit_spider_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    cave: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = if cave {
        cave_spider_model_root_transform(instance)
    } else {
        entity_model_root_transform(instance)
    };
    emit_textured_passes_with_head_look(
        meshes,
        spider_textured_layer_passes(cave),
        head_first_part_index(),
        transform,
        instance,
        atlas,
    );
}

fn emit_enderman_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_textured_passes_with_head_look(
        meshes,
        enderman_textured_layer_passes(),
        head_first_part_index(),
        entity_model_root_transform(instance),
        instance,
        atlas,
    );
}

fn emit_iron_golem_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_textured_passes_with_head_look(
        meshes,
        iron_golem_textured_layer_passes(),
        head_first_part_index(),
        entity_model_root_transform(instance),
        instance,
        atlas,
    );
}

fn emit_snow_golem_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_textured_passes_with_head_look(
        meshes,
        snow_golem_textured_layer_passes(),
        head_first_part_index(),
        entity_model_root_transform(instance),
        instance,
        atlas,
    );
}

fn emit_witch_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_villager_family_textured_passes(
        meshes,
        witch_textured_layer_passes(),
        villager_head_part_index(false),
        VILLAGER_ADULT_LEG_PART_INDICES,
        villager_adult_model_root_transform(instance),
        instance,
        atlas,
    );
}

fn emit_slime_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    size: i32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = slime_model_root_transform(instance, size);
    for pass in slime_textured_layer_passes() {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_magma_cube_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    size: i32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = magma_cube_model_root_transform(instance, size);
    for pass in magma_cube_textured_layer_passes() {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_polar_bear_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = if baby {
        entity_model_root_transform(instance)
    } else {
        polar_bear_model_root_transform(instance)
    };
    let stand_scale = instance.render_state.polar_bear_stand_scale;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    let head_index = polar_bear_head_part_index(baby);
    for pass in polar_bear_textured_layer_passes(baby) {
        if stand_scale == 0.0 && head_resting && legs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            // Vanilla runs `super.setupAnim` (the head look and four-leg swing) before
            // the standing rear adds its deltas on top (`frontLeg.xRot -= ...` on top
            // of the swing), so apply the look and leg swing before the standing pose.
            let mut parts = pass.parts.to_vec();
            if !head_resting {
                if let Some(head) = parts.get_mut(head_index) {
                    head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
                }
            }
            if !legs_resting {
                for index in QUADRUPED_LEG_PART_INDICES {
                    if let Some(leg) = parts.get_mut(index) {
                        leg.pose =
                            quadruped_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                    }
                }
            }
            if stand_scale != 0.0 {
                for (index, part) in polar_bear_standing_part_roles(baby) {
                    apply_polar_bear_standing_pose(&mut parts[index].pose, part, baby, stand_scale);
                }
            }
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
}

fn emit_hoglin_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: HoglinModelFamily,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `HoglinModel.setupAnim` (zoglin shares it) swings the four legs
    // `cos(pos [+ π]) * 1.2 * speed` (amplitude 1.2, no 0.6662 factor; right-front/
    // left-hind in phase) after the yaw-only head look. Legs are at [2, 3, 4, 5].
    let head_index = hoglin_head_part_index(baby);
    let transform = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_yaw_at_rest(head_yaw);
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in hoglin_textured_layer_passes(family, baby) {
        if head_resting && legs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            if !head_resting {
                if let Some(head) = parts.get_mut(head_index) {
                    head.pose = head_look_yaw_pose(head.pose, head_yaw);
                }
            }
            if !legs_resting {
                for index in HOGLIN_LEG_PART_INDICES {
                    if let Some(leg) = parts.get_mut(index) {
                        leg.pose = hoglin_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                    }
                }
            }
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
}

/// The four leg part indices in the hoglin/zoglin textured body layers (the head
/// and body occupy `0`/`1` in either order). [`hoglin_leg_swing_pose`] resolves each
/// leg's phase from its offset.
const HOGLIN_LEG_PART_INDICES: [usize; 4] = [2, 3, 4, 5];

fn emit_ravager_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    for pass in ravager_textured_layer_passes() {
        if head_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
            continue;
        }
        // Vanilla nests the ravager head inside the neck (`neck.getChild("head")`).
        // The neck's children list is static, so emit the neck subtree by hand,
        // applying the look to the head child (its horn/mouth children inherit it).
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        let mesh = meshes.mesh_mut(pass.render_type);
        let neck = &RAVAGER_TEXTURED_PARTS[ravager_neck_part_index()];
        let neck_transform = transform * part_pose_transform(neck.pose);
        for cube in neck.cubes {
            emit_textured_model_cube(
                mesh,
                neck_transform,
                *cube,
                pass.texture,
                entry.uv,
                pass.tint,
            );
        }
        let head = RAVAGER_TEXTURED_NECK_CHILDREN[ravager_head_child_index()];
        let looked_head = TexturedModelPartDesc {
            pose: head_look_pose(head.pose, head_yaw, head_pitch),
            ..head
        };
        emit_textured_model_parts(
            mesh,
            &[looked_head],
            neck_transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
        emit_textured_model_parts(
            mesh,
            &RAVAGER_TEXTURED_PARTS[ravager_neck_part_index() + 1..],
            transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
    }
}

fn emit_villager_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = if baby {
        entity_model_root_transform(instance)
    } else {
        villager_adult_model_root_transform(instance)
    };
    let leg_indices = if baby {
        VILLAGER_BABY_LEG_PART_INDICES
    } else {
        VILLAGER_ADULT_LEG_PART_INDICES
    };
    emit_villager_family_textured_passes(
        meshes,
        villager_textured_layer_passes(baby),
        villager_head_part_index(baby),
        leg_indices,
        transform,
        instance,
        atlas,
    );
}

fn emit_wandering_trader_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_villager_family_textured_passes(
        meshes,
        wandering_trader_textured_layer_passes(),
        villager_head_part_index(false),
        VILLAGER_ADULT_LEG_PART_INDICES,
        villager_adult_model_root_transform(instance),
        instance,
        atlas,
    );
}

fn emit_player_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    slim: bool,
    parts: PlayerModelPartVisibility,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = player_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    // All passes share one visibility-filtered part array, so the head look and
    // the inherited `HumanoidModel` leg swing are applied once to the head and leg
    // parts before emitting every pass (the pants children ride the leg parts).
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let mut visible_parts = player_visible_textured_model_parts(slim, parts);
    if !head_look_at_rest(head_yaw, head_pitch) {
        if let Some(head) = visible_parts.get_mut(player_head_part_index()) {
            head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
        }
    }
    if !limb_swing_at_rest(limb_swing_amount) {
        for index in HUMANOID_LEG_PART_INDICES {
            if let Some(leg) = visible_parts.get_mut(index) {
                leg.pose = humanoid_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
            }
        }
    }
    for pass in player_textured_layer_passes(slim, parts) {
        emit_textured_layer_pass_with_parts(
            meshes,
            &pass,
            visible_parts.as_slice(),
            transform,
            atlas,
        );
    }
}

fn emit_sheep_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
    invisible: bool,
    jeb: bool,
    age_ticks: f32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    let head_eat = instance.render_state.head_eat;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let head_index = sheep_head_part_index(baby);
    let head_resting = sheep_head_at_rest(head_eat, head_yaw, head_pitch);
    // Vanilla `SheepModel.setupAnim` runs `super.setupAnim` (the `QuadrupedModel` leg
    // swing) before its eat-grass head pose, so every sheep layer (body and wool)
    // swings its legs.
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in sheep_textured_layer_passes(baby, sheared, wool_color, invisible, jeb, age_ticks) {
        if head_resting && legs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            if !head_resting {
                if let Some(head) = parts.get_mut(head_index) {
                    head.pose = sheep_head_pose(head.pose, baby, head_eat, head_yaw, head_pitch);
                }
            }
            if !legs_resting {
                for index in QUADRUPED_LEG_PART_INDICES {
                    if let Some(leg) = parts.get_mut(index) {
                        leg.pose =
                            quadruped_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                    }
                }
            }
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
}

fn emit_wolf_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    tame: bool,
    angry: bool,
    invisible: bool,
    collar_color: Option<EntityDyeColor>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_textured_passes_with_head_look(
        meshes,
        wolf_textured_layer_passes(baby, tame, angry, invisible, collar_color),
        head_first_part_index(),
        entity_model_root_transform(instance),
        instance,
        atlas,
    );
}

fn emit_goat_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    left_horn: bool,
    right_horn: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    let head_index = if baby {
        BABY_GOAT_HEAD_INDEX
    } else {
        ADULT_GOAT_HEAD_INDEX
    };
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    // All passes share one visibility-filtered part array (like the player), so the
    // head look and the `QuadrupedModel` leg swing are applied once to the head and
    // four leg parts before emitting every pass. The adult layer lists the legs at
    // [2, 3, 4, 5], the baby layer at [0, 1, 2, 3].
    let leg_indices: [usize; 4] = if baby { [0, 1, 2, 3] } else { [2, 3, 4, 5] };
    let mut visible_parts = goat_visible_textured_model_parts(baby, left_horn, right_horn);
    if !head_look_at_rest(head_yaw, head_pitch) {
        if let Some(head) = visible_parts.get_mut(head_index) {
            head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
        }
    }
    if !limb_swing_at_rest(limb_swing_amount) {
        for index in leg_indices {
            if let Some(leg) = visible_parts.get_mut(index) {
                leg.pose = quadruped_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
            }
        }
    }
    for pass in goat_textured_layer_passes(baby) {
        emit_textured_layer_pass_with_parts(
            meshes,
            &pass,
            visible_parts.as_slice(),
            transform,
            atlas,
        );
    }
}

fn emit_skeleton_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: Option<SkeletonModelFamily>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = if matches!(family, Some(SkeletonModelFamily::WitherSkeleton)) {
        wither_skeleton_model_root_transform(instance)
    } else {
        entity_model_root_transform(instance)
    };
    let head_index = if matches!(family, Some(SkeletonModelFamily::Parched)) {
        parched_head_part_index()
    } else {
        skeleton_head_part_index()
    };
    emit_humanoid_textured_passes(
        meshes,
        skeleton_textured_layer_passes(family),
        head_index,
        HUMANOID_LEG_PART_INDICES,
        transform,
        instance,
        atlas,
    );
}

fn emit_textured_layer_pass(
    meshes: &mut EntityModelTexturedMeshes,
    pass: &EntityModelLayerPass,
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_textured_layer_pass_with_parts(meshes, pass, pass.parts, transform, atlas);
}

fn emit_textured_layer_pass_with_parts(
    meshes: &mut EntityModelTexturedMeshes,
    pass: &EntityModelLayerPass,
    parts: &[TexturedModelPartDesc],
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
        return;
    };
    emit_textured_model_parts(
        meshes.mesh_mut(pass.render_type),
        parts,
        transform,
        pass.texture,
        entry.uv,
        pass.tint,
    );
}

fn entity_model_texture_atlas_entry(
    atlas: &EntityModelTextureAtlasLayout,
    texture: EntityModelTextureRef,
) -> Option<EntityModelTextureAtlasEntry> {
    atlas
        .entries
        .iter()
        .copied()
        .find(|entry| entry.texture == texture)
}
