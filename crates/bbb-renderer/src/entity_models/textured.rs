use super::keyframe::{keyframe_elapsed_seconds, keyframe_walk_sample, sample_bone_offsets};
use super::model::EntityModel;
use super::{
    boat_model_root_transform,
    catalog::squid_texture_ref,
    catalog::{
        ArmorStandModelPose, BoatModelFamily, CamelModelFamily, ChickenModelVariant,
        CowModelVariant, EntityDyeColor, EntityModelKind, EntityModelTextureAtlasEntry,
        EntityModelTextureAtlasLayout, EntityModelTextureRef, EntityModelUvRect, HoglinModelFamily,
        IllagerModelFamily, LlamaVariant, PigModelVariant, PiglinModelFamily,
        PlayerModelPartVisibility, SalmonModelSize, SheepWoolColor, SkeletonModelFamily,
        TropicalFishModelShape, TropicalFishPattern, ZombieVariantModelFamily,
    },
    cave_spider_model_root_transform, cod_model_root_transform, creeper_model_root_transform,
    entity_model_root_transform,
    geometry::{
        emit_textured_model_cube, emit_textured_model_part, emit_textured_model_parts,
        fill_entity_textured_light, fill_entity_textured_overlay, part_pose_transform,
        EntityModelTexturedMesh, ModelPartDesc, PartPose, TexturedModelCubeDesc,
        TexturedModelPartDesc, PART_POSE_ZERO,
    },
    ghast_model_root_transform, happy_ghast_model_root_transform,
    instances::EntityModelInstance,
    magma_cube_model_root_transform, mesh_transformer_scaled_model_root_transform,
    model_layers::{
        allay_arm_idle_bob_amount, allay_body_x_rot, allay_root_y, allay_wing_flap_amount,
        allay_wing_rest_x_rot, apply_polar_bear_standing_pose, apply_wolf_sitting_pose,
        armor_stand_textured_cube, bee_antenna_x_rot, bee_back_leg_x_rot, bee_bone_x_rot,
        bee_bone_y_delta, bee_front_leg_x_rot, bee_wing_z_rot, blaze_rod_offset,
        camel_clamped_head_look, chicken_leg_part_indices, cow_head_part_index, dolphin_wave,
        enderman_arm_swing_pose, enderman_carried_arm_pose, enderman_leg_swing_pose,
        endermite_segment_pose, ghast_tentacle_x_rot, half_amplitude_leg_swing_pose,
        head_first_part_index, head_look_at_rest, head_look_pose, head_look_yaw_pose,
        head_yaw_at_rest, hoglin_ear_sway_pose, hoglin_head_part_index, hoglin_leg_swing_pose,
        humanoid_arm_bob_pose, humanoid_arm_swing_pose, humanoid_crouch_arm_pose,
        humanoid_crouch_body_pose, humanoid_crouch_head_pose, humanoid_crouch_leg_pose,
        humanoid_leg_swing_pose, illager_spellcast_arm_pose, iron_golem_walk_part_roles,
        iron_golem_walk_pose, limb_swing_at_rest, parched_head_part_index, phantom_flap_time,
        phantom_tail_pose, phantom_tail_x_rot, phantom_wing_pose, phantom_wing_z_rot,
        pig_head_part_index, piglin_ear_flap_pose, piglin_head_part_index, player_head_part_index,
        polar_bear_head_part_index, polar_bear_standing_part_roles, pufferfish_fin_pose,
        pufferfish_parts, pufferfish_right_fin_z_rot, quadruped_leg_swing_pose,
        ravager_head_child_index, ravager_leg_swing_pose, ravager_neck_part_index,
        sheep_head_at_rest, sheep_head_part_index, sheep_head_pose, silverfish_layer_pose,
        silverfish_segment_pose, skeleton_head_part_index, snow_golem_arm_pose,
        snow_golem_upper_body_pose, snow_golem_upper_body_yrot, spider_leg_swing_pose,
        spider_leg_swing_roles, squid_textured_model_parts, strider_animation_speed,
        strider_body_y, strider_body_z_rot, strider_bristle_bottom_flow, strider_bristle_flow,
        strider_bristle_middle_flow, strider_bristle_top_flow, strider_leg_x_rot, strider_leg_y,
        strider_leg_z_rot, tropical_fish_tail_yrot, turtle_leg_rotation, vex_left_wing_y_rot,
        vex_moving_arm_z_bob, villager_head_part_index, witch_nose_bob_pose, wolf_angry_tail_pose,
        wolf_sitting_part_roles, wolf_tail_part_index, wolf_tail_swing_pose,
        zombie_arm_held_out_pose, CamelWalkLayout, CodModel, SalmonModel, ADULT_CAMEL_WALK_LAYOUT,
        ADULT_GOAT_HEAD_INDEX, ALLAY_BODY_POSE, ALLAY_HEAD_POSE, ALLAY_LEFT_ARM_POSE,
        ALLAY_LEFT_WING_POSE, ALLAY_RIGHT_ARM_POSE, ALLAY_RIGHT_WING_POSE, ALLAY_TEXTURED_BODY,
        ALLAY_TEXTURED_HEAD, ALLAY_TEXTURED_LEFT_ARM, ALLAY_TEXTURED_RIGHT_ARM,
        ALLAY_TEXTURED_WING, ALLAY_TEXTURE_REF, ALLAY_WING_Y_ROT_BASE, ARMOR_STAND_PARTS,
        ARMOR_STAND_PART_UVS, ARMOR_STAND_TEXTURE_REF, BABY_CAMEL_WALK_LAYOUT,
        BABY_GOAT_HEAD_INDEX, BAT_BODY_POSE, BAT_FEET_POSE, BAT_FLYING, BAT_HEAD_POSE,
        BAT_LEFT_EAR_POSE, BAT_LEFT_WING_POSE, BAT_LEFT_WING_TIP_POSE, BAT_RESTING,
        BAT_RIGHT_EAR_POSE, BAT_RIGHT_WING_POSE, BAT_RIGHT_WING_TIP_POSE, BAT_TEXTURED_BODY,
        BAT_TEXTURED_FEET, BAT_TEXTURED_HEAD, BAT_TEXTURED_LEFT_EAR, BAT_TEXTURED_LEFT_WING,
        BAT_TEXTURED_LEFT_WING_TIP, BAT_TEXTURED_RIGHT_EAR, BAT_TEXTURED_RIGHT_WING,
        BAT_TEXTURED_RIGHT_WING_TIP, BAT_TEXTURE_REF, BEE_BABY_BACK_LEGS_POSE, BEE_BABY_BODY_POSE,
        BEE_BABY_BONE_POSE, BEE_BABY_FRONT_LEGS_POSE, BEE_BABY_LEFT_WING_POSE,
        BEE_BABY_MIDDLE_LEGS_POSE, BEE_BABY_RIGHT_WING_POSE, BEE_BABY_STINGER_POSE,
        BEE_BABY_TEXTURED_BACK_LEGS, BEE_BABY_TEXTURED_BODY, BEE_BABY_TEXTURED_BONE,
        BEE_BABY_TEXTURED_FRONT_LEGS, BEE_BABY_TEXTURED_LEFT_WING, BEE_BABY_TEXTURED_MIDDLE_LEGS,
        BEE_BABY_TEXTURED_RIGHT_WING, BEE_BABY_TEXTURED_STINGER, BEE_BABY_TEXTURE_REF,
        BEE_BACK_LEGS_POSE, BEE_BODY_POSE, BEE_BONE_POSE, BEE_FRONT_LEGS_POSE,
        BEE_LEFT_ANTENNA_POSE, BEE_LEFT_WING_POSE, BEE_MIDDLE_LEGS_POSE, BEE_MID_LEG_FLYING_X_ROT,
        BEE_RIGHT_ANTENNA_POSE, BEE_RIGHT_WING_POSE, BEE_STINGER_POSE, BEE_TEXTURED_BACK_LEGS,
        BEE_TEXTURED_BODY, BEE_TEXTURED_FRONT_LEGS, BEE_TEXTURED_LEFT_ANTENNA,
        BEE_TEXTURED_LEFT_WING, BEE_TEXTURED_MIDDLE_LEGS, BEE_TEXTURED_RIGHT_ANTENNA,
        BEE_TEXTURED_RIGHT_WING, BEE_TEXTURED_STINGER, BEE_TEXTURE_REF, BLAZE_ROD_COUNT,
        BREEZE_BODY_POSE, BREEZE_HEAD_POSE, BREEZE_IDLE, BREEZE_RODS_POSE, BREEZE_ROD_1_POSE,
        BREEZE_ROD_2_POSE, BREEZE_ROD_3_POSE, BREEZE_TEXTURED_HEAD, BREEZE_TEXTURED_ROD,
        BREEZE_TEXTURE_REF, CAMEL_WALK_SCALE_FACTOR, CAMEL_WALK_SPEED_FACTOR, COD_TEXTURE_REF,
        DOLPHIN_BABY_TEXTURE_REF, DOLPHIN_BACK_FIN_POSE, DOLPHIN_BODY_POSE, DOLPHIN_HEAD_POSE,
        DOLPHIN_LEFT_FIN_POSE, DOLPHIN_NOSE_POSE, DOLPHIN_RIGHT_FIN_POSE, DOLPHIN_TAIL_BIND_X_ROT,
        DOLPHIN_TAIL_FIN_POSE, DOLPHIN_TAIL_POSE, DOLPHIN_TEXTURED_BACK_FIN, DOLPHIN_TEXTURED_BODY,
        DOLPHIN_TEXTURED_HEAD, DOLPHIN_TEXTURED_LEFT_FIN, DOLPHIN_TEXTURED_NOSE,
        DOLPHIN_TEXTURED_RIGHT_FIN, DOLPHIN_TEXTURED_TAIL, DOLPHIN_TEXTURED_TAIL_FIN,
        DOLPHIN_TEXTURE_REF, ENDERMAN_TEXTURED_HEAD_CHILDREN_CREEPY, HOGLIN_LEFT_EAR_CHILD_INDEX,
        HOGLIN_RIGHT_EAR_CHILD_INDEX, PHANTOM_BODY_POSE, PHANTOM_BODY_TEXTURED_CUBE,
        PHANTOM_HEAD_POSE, PHANTOM_HEAD_TEXTURED_CUBE, PHANTOM_LEFT_WING_BASE_POSE,
        PHANTOM_LEFT_WING_BASE_TEXTURED_CUBE, PHANTOM_LEFT_WING_TIP_POSE,
        PHANTOM_LEFT_WING_TIP_TEXTURED_CUBE, PHANTOM_RIGHT_WING_BASE_POSE,
        PHANTOM_RIGHT_WING_BASE_TEXTURED_CUBE, PHANTOM_RIGHT_WING_TIP_POSE,
        PHANTOM_RIGHT_WING_TIP_TEXTURED_CUBE, PHANTOM_TAIL_BASE_POSE,
        PHANTOM_TAIL_BASE_TEXTURED_CUBE, PHANTOM_TAIL_TIP_POSE, PHANTOM_TAIL_TIP_TEXTURED_CUBE,
        PIGLIN_ADULT_EAR_ANGLE, PIGLIN_BABY_EAR_ANGLE, PUFFERFISH_TEXTURE_REF,
        RAVAGER_TEXTURED_NECK_CHILDREN, SILVERFISH_LAYER_RULES, SILVERFISH_SEGMENT_COUNT,
        SMALL_ARMOR_STAND_PARTS, SNOW_GOLEM_HEAD_PART_INDEX, SNOW_GOLEM_LEFT_ARM_PART_INDEX,
        SNOW_GOLEM_RIGHT_ARM_PART_INDEX, SNOW_GOLEM_UPPER_BODY_PART_INDEX,
        STRIDER_BABY_BACK_BRISTLE_POSE, STRIDER_BABY_BODY_BASE_Y, STRIDER_BABY_FRONT_BRISTLE_POSE,
        STRIDER_BABY_LEFT_LEG_X, STRIDER_BABY_LEG_BASE_Y, STRIDER_BABY_MIDDLE_BRISTLE_POSE,
        STRIDER_BABY_RIGHT_LEG_X, STRIDER_BABY_TEXTURED_BACK_BRISTLE, STRIDER_BABY_TEXTURED_BODY,
        STRIDER_BABY_TEXTURED_FRONT_BRISTLE, STRIDER_BABY_TEXTURED_LEFT_LEG,
        STRIDER_BABY_TEXTURED_MIDDLE_BRISTLE, STRIDER_BABY_TEXTURED_RIGHT_LEG,
        STRIDER_BABY_TEXTURE_REF, STRIDER_BODY_BASE_Y, STRIDER_LEFT_BOTTOM_BRISTLE_POSE,
        STRIDER_LEFT_LEG_X, STRIDER_LEFT_MIDDLE_BRISTLE_POSE, STRIDER_LEFT_TOP_BRISTLE_POSE,
        STRIDER_LEG_BASE_Y, STRIDER_RIGHT_BOTTOM_BRISTLE_POSE, STRIDER_RIGHT_LEG_X,
        STRIDER_RIGHT_MIDDLE_BRISTLE_POSE, STRIDER_RIGHT_TOP_BRISTLE_POSE, STRIDER_TEXTURED_BODY,
        STRIDER_TEXTURED_LEFT_BOTTOM_BRISTLE, STRIDER_TEXTURED_LEFT_LEG,
        STRIDER_TEXTURED_LEFT_MIDDLE_BRISTLE, STRIDER_TEXTURED_LEFT_TOP_BRISTLE,
        STRIDER_TEXTURED_RIGHT_BOTTOM_BRISTLE, STRIDER_TEXTURED_RIGHT_LEG,
        STRIDER_TEXTURED_RIGHT_MIDDLE_BRISTLE, STRIDER_TEXTURED_RIGHT_TOP_BRISTLE,
        STRIDER_TEXTURE_REF, TROPICAL_FISH_TAIL_PART_INDEX, TURTLE_BABY_BODY_POSE,
        TURTLE_BABY_HEAD_POSE, TURTLE_BABY_LEFT_FRONT_LEG_POSE, TURTLE_BABY_LEFT_HIND_LEG_POSE,
        TURTLE_BABY_RIGHT_FRONT_LEG_POSE, TURTLE_BABY_RIGHT_HIND_LEG_POSE,
        TURTLE_BABY_TEXTURED_BODY, TURTLE_BABY_TEXTURED_HEAD, TURTLE_BABY_TEXTURED_LEFT_FRONT_LEG,
        TURTLE_BABY_TEXTURED_LEFT_HIND_LEG, TURTLE_BABY_TEXTURED_RIGHT_FRONT_LEG,
        TURTLE_BABY_TEXTURED_RIGHT_HIND_LEG, TURTLE_BABY_TEXTURE_REF, TURTLE_BODY_POSE,
        TURTLE_EGG_ROOT_DROP_POSE, TURTLE_HEAD_POSE, TURTLE_LEFT_FRONT_LEG_POSE,
        TURTLE_LEFT_HIND_LEG_POSE, TURTLE_RIGHT_FRONT_LEG_POSE, TURTLE_RIGHT_HIND_LEG_POSE,
        TURTLE_TEXTURED_BODY, TURTLE_TEXTURED_EGG_BELLY, TURTLE_TEXTURED_HEAD,
        TURTLE_TEXTURED_LEFT_FRONT_LEG, TURTLE_TEXTURED_LEFT_HIND_LEG,
        TURTLE_TEXTURED_RIGHT_FRONT_LEG, TURTLE_TEXTURED_RIGHT_HIND_LEG, TURTLE_TEXTURE_REF,
        VEX_ARM_CHARGING_X_ROT, VEX_ARM_CHARGING_Y_ROT, VEX_ARM_CHARGING_Z_ROT, VEX_ARM_REST_Z_ROT,
        VEX_BODY_POSE, VEX_BODY_X_ROT, VEX_HEAD_POSE, VEX_LEFT_ARM_POSE, VEX_LEFT_WING_POSE,
        VEX_RIGHT_ARM_POSE, VEX_RIGHT_WING_POSE, VEX_ROOT_POSE, VEX_TEXTURED_BODY,
        VEX_TEXTURED_HEAD, VEX_TEXTURED_LEFT_ARM, VEX_TEXTURED_LEFT_WING, VEX_TEXTURED_RIGHT_ARM,
        VEX_TEXTURED_RIGHT_WING, VEX_TEXTURE_REF, VEX_WING_X_ROT, VEX_WING_Z_ROT,
        WITCH_NOSE_CHILD_INDEX,
    },
    phantom_model_root_transform, player_model_root_transform, polar_bear_model_root_transform,
    pufferfish_model_root_transform, salmon_model_root_transform, slime_model_root_transform,
    squid_model_root_transform, tropical_fish_model_root_transform,
    villager_adult_model_root_transform, wither_skeleton_model_root_transform, HUSK_SCALE,
};
use glam::Mat4;

mod layers;

pub(super) use layers::{
    blaze_textured_layer_passes, boat_textured_layer_passes, camel_textured_layer_passes,
    chicken_textured_layer_passes, cow_textured_layer_passes, creeper_textured_layer_passes,
    drowned_textured_layer_passes, enderman_textured_layer_passes, endermite_textured_layer_passes,
    ghast_textured_layer_passes, goat_textured_layer_passes, happy_ghast_textured_layer_passes,
    hoglin_textured_layer_passes, husk_textured_layer_passes, illager_textured_layer_passes,
    illager_textured_spellcasting_parts, iron_golem_textured_layer_passes,
    llama_textured_layer_passes, magma_cube_textured_layer_passes, minecart_textured_layer_passes,
    phantom_textured_layer_passes, pig_textured_layer_passes, piglin_textured_layer_passes,
    player_textured_layer_passes, polar_bear_textured_layer_passes, ravager_textured_layer_passes,
    salmon_textured_layer_passes, sheep_textured_layer_passes, silverfish_textured_layer_passes,
    skeleton_textured_layer_passes, slime_textured_layer_passes, snow_golem_textured_layer_passes,
    spider_textured_layer_passes, tropical_fish_textured_layer_passes,
    villager_textured_layer_passes, wandering_trader_textured_layer_passes,
    witch_textured_layer_passes, wolf_textured_layer_passes, zombie_textured_layer_passes,
    zombie_villager_textured_layer_passes, EntityModelLayerPass, EntityModelLayerRenderType,
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
            EntityModelKind::Llama {
                variant,
                baby,
                has_chest,
                ..
            } => {
                emit_llama_textured_model(&mut meshes, *instance, variant, baby, has_chest, atlas);
            }
            EntityModelKind::Camel { family, baby } => {
                emit_camel_textured_model(&mut meshes, *instance, family, baby, atlas);
            }
            EntityModelKind::Squid { glow, baby } => {
                emit_squid_textured_model(&mut meshes, *instance, glow, baby, atlas);
            }
            EntityModelKind::Cod => {
                emit_cod_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Salmon { size } => {
                emit_salmon_textured_model(&mut meshes, *instance, size, atlas);
            }
            EntityModelKind::TropicalFish {
                shape,
                base_color,
                pattern,
                pattern_color,
            } => {
                emit_tropical_fish_textured_model(
                    &mut meshes,
                    *instance,
                    shape,
                    base_color,
                    pattern,
                    pattern_color,
                    atlas,
                );
            }
            EntityModelKind::Vex => {
                emit_vex_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Allay => {
                emit_allay_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Strider { baby } => {
                emit_strider_textured_model(&mut meshes, *instance, baby, atlas);
            }
            EntityModelKind::Turtle { baby } => {
                emit_turtle_textured_model(&mut meshes, *instance, baby, atlas);
            }
            EntityModelKind::Bat => {
                emit_bat_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Bee { baby } => {
                emit_bee_textured_model(&mut meshes, *instance, baby, atlas);
            }
            EntityModelKind::Breeze => {
                emit_breeze_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Dolphin { baby } => {
                emit_dolphin_textured_model(&mut meshes, *instance, baby, atlas);
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
            EntityModelKind::Ghast => {
                emit_ghast_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::HappyGhast => {
                emit_happy_ghast_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Minecart => {
                emit_minecart_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::ArmorStand {
                small,
                show_arms,
                show_base_plate,
                pose,
            } => {
                emit_armor_stand_textured_model(
                    &mut meshes,
                    *instance,
                    small,
                    show_arms,
                    show_base_plate,
                    pose,
                    atlas,
                );
            }
            EntityModelKind::Zombie { baby } => {
                emit_zombie_textured_model(&mut meshes, *instance, baby, atlas);
            }
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby,
            } => {
                emit_husk_textured_model(&mut meshes, *instance, baby, atlas);
            }
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby,
            } => {
                emit_drowned_textured_model(&mut meshes, *instance, baby, atlas);
            }
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby,
            } => {
                emit_zombie_villager_textured_model(&mut meshes, *instance, baby, atlas);
            }
            EntityModelKind::Piglin { family, baby } => {
                emit_piglin_textured_model(&mut meshes, *instance, family, baby, atlas);
            }
            EntityModelKind::Blaze => {
                emit_blaze_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Endermite => {
                emit_endermite_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Silverfish => {
                emit_silverfish_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Phantom { size } => {
                emit_phantom_textured_model(&mut meshes, *instance, size, atlas);
            }
            EntityModelKind::Pufferfish { puff_state } => {
                emit_pufferfish_textured_model(&mut meshes, *instance, puff_state, atlas);
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
            EntityModelKind::Illager { family } => {
                emit_illager_textured_model(&mut meshes, *instance, family, atlas);
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
    // Vanilla `ChickenModel.setupAnim` swings the two legs with the `HumanoidModel`
    // phase `cos(pos * 0.6662 [+ π]) * 1.4 * speed` (right leg in phase, left out). The
    // chicken has no head look; its wing flap is driven by the untracked `flap`/
    // `flapSpeed` state (deferred). Every pass shares the body-layer part layout.
    let transform = entity_model_root_transform(instance);
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    let leg_indices = chicken_leg_part_indices(baby);
    for pass in chicken_textured_layer_passes(variant, baby) {
        if legs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            for index in leg_indices {
                if let Some(leg) = parts.get_mut(index) {
                    leg.pose = humanoid_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                }
            }
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
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

/// The textured camel base layer. Vanilla `CamelModel.setupAnim` drives every limb via
/// baked `KeyframeAnimation`s (walk/sit/standup/idle/dash) plus a direct head yaw/pitch
/// clamp ([`camel_clamped_head_look`]). The head look and the walk (adult/husk `CAMEL_WALK`,
/// baby `CAMEL_BABY_WALK`) are reproduced here; the sit/standup/idle/dash animations remain
/// deferred. The camel husk shares the adult mesh, differing only in texture.
fn emit_camel_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: CamelModelFamily,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    let (head_yaw, head_pitch) = camel_clamped_head_look(
        instance.render_state.head_yaw,
        instance.render_state.head_pitch,
    );
    // The adult camel and the husk share the adult mesh/walk; the camel baby uses its own mesh/walk.
    let layout = if family == CamelModelFamily::Camel && baby {
        &BABY_CAMEL_WALK_LAYOUT
    } else {
        &ADULT_CAMEL_WALK_LAYOUT
    };
    emit_camel_walk_textured(
        meshes, instance, transform, atlas, family, baby, layout, head_yaw, head_pitch,
    );
}

/// Hand-walks the camel's textured passes through its walk ([`CamelWalkLayout::walk`]), composing the
/// walk onto the clamped head look — the textured twin of the colored `emit_camel_walk_colored`. The
/// `root` channel rolls the whole model, the four legs swing (rotation + position), the `head` adds a
/// pitch (and, for the baby, a position nudge) onto the look, the two ears flap, the tail swishes, and
/// the baby `body` dips. A still camel samples amplitude 0, collapsing to the bind pose plus the look.
#[allow(clippy::too_many_arguments)]
fn emit_camel_walk_textured(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
    family: CamelModelFamily,
    baby: bool,
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
    let sample = |bone: &str| sample_bone_offsets(layout.walk, bone, seconds, scale);
    let (root_pos, root_rot) = sample("root");
    let (body_pos, body_rot) = sample("body");
    let (head_walk_pos, head_walk_rot) = sample("head");

    for pass in camel_textured_layer_passes(family, baby) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        let uv = entry.uv;
        let texture = pass.texture;
        let tint = pass.tint;
        let parts = pass.parts;
        let mesh = meshes.mesh_mut(pass.render_type);

        // `root` rolls the whole model at the entity root.
        let root_t = transform
            * part_pose_transform(keyframe_textured_pose(PART_POSE_ZERO, root_pos, root_rot));

        // `body` (root child 0): not animated on the adult; the baby walk dips it via a `body` channel.
        let body = &parts[0];
        let body_t =
            root_t * part_pose_transform(keyframe_textured_pose(body.pose, body_pos, body_rot));
        for cube in body.cubes {
            emit_textured_model_cube(mesh, body_t, *cube, texture, uv, tint);
        }

        // The body's children: the head (clamped look + walk), the tail (walk swish), and — on the
        // adult — the static hump, in declared order (preserving the depth-first emit order).
        for (index, child) in body.children.iter().enumerate() {
            if index == layout.head_child {
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
                    emit_textured_model_cube(mesh, head_t, *cube, texture, uv, tint);
                }
                for (ear_index, ear_bone) in layout.ears {
                    let ear = &child.children[ear_index];
                    let (ear_pos, ear_rot) = sample(ear_bone);
                    emit_textured_model_part(
                        mesh,
                        &TexturedModelPartDesc {
                            pose: keyframe_textured_pose(ear.pose, ear_pos, ear_rot),
                            ..*ear
                        },
                        head_t,
                        texture,
                        uv,
                        tint,
                    );
                }
            } else if index == layout.tail_child {
                let (tail_pos, tail_rot) = sample("tail");
                emit_textured_model_part(
                    mesh,
                    &TexturedModelPartDesc {
                        pose: keyframe_textured_pose(child.pose, tail_pos, tail_rot),
                        ..*child
                    },
                    body_t,
                    texture,
                    uv,
                    tint,
                );
            } else {
                // The adult hump (static).
                emit_textured_model_part(mesh, child, body_t, texture, uv, tint);
            }
        }

        // The four legs (root children 1..=4): the walk rotation + position.
        for (index, bone) in layout.legs {
            let (leg_pos, leg_rot) = sample(bone);
            let leg = &parts[index];
            emit_textured_model_part(
                mesh,
                &TexturedModelPartDesc {
                    pose: keyframe_textured_pose(leg.pose, leg_pos, leg_rot),
                    ..*leg
                },
                root_t,
                texture,
                uv,
                tint,
            );
        }
    }
}

/// The textured cod base layer. The cod parts are static, so the body/head/nose/fins
/// emit through the standard pass while only the tail fin is re-posed by the vanilla
/// `CodModel.setupAnim` sway; the swim wiggle and out-of-water flop live in
/// [`cod_model_root_transform`].
fn emit_cod_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `CodModel` tree drives both render paths: `setup_anim` sways the tail fin once,
    // and the textured pass walks the posed tree (vanilla `CodRenderer` is a single cutout layer).
    let in_water = instance.render_state.in_water;
    let transform = cod_model_root_transform(instance, in_water);
    let mut model = CodModel::new();
    model.prepare(&instance);
    if let Some(entry) = entity_model_texture_atlas_entry(atlas, COD_TEXTURE_REF) {
        model.root().render_textured(
            meshes.mesh_mut(EntityModelLayerRenderType::Cutout),
            transform,
            COD_TEXTURE_REF,
            entry.uv,
            [1.0, 1.0, 1.0, 1.0],
        );
    }
}

/// The textured salmon base layer. The salmon parts are static apart from the back body
/// segment, which carries the tail and rear top fin and is swayed by the vanilla
/// `SalmonModel.setupAnim`; the swim wiggle, out-of-water flop, and small/medium/large
/// mesh scale live in [`salmon_model_root_transform`].
fn emit_salmon_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    size: SalmonModelSize,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `SalmonModel` tree drives both render paths; `setup_anim` sways the back body
    // segment once. Each layer pass supplies the texture / render type / tint, and the posed tree
    // supplies the geometry (vanilla `SalmonRenderer` is a single cutout layer per size).
    let in_water = instance.render_state.in_water;
    let transform = salmon_model_root_transform(instance, in_water, size);
    let mut model = SalmonModel::new();
    model.prepare(&instance);
    for pass in salmon_textured_layer_passes(size) {
        if let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) {
            model.root().render_textured(
                meshes.mesh_mut(pass.render_type),
                transform,
                pass.texture,
                entry.uv,
                pass.tint,
            );
        }
    }
}

/// The textured tropical fish base layer plus the `TropicalFishPatternLayer` overlay. The
/// parts are static apart from the tail, which is swayed by the vanilla
/// `TropicalFish{Small,Large}Model.setupAnim`; the swim wiggle, out-of-water flop, and
/// small/large body shape live in [`tropical_fish_model_root_transform`] and the per-shape
/// pass. The base body is tinted by `getModelTint` = `getBaseColor().getTextureDiffuseColor()`,
/// and the pattern overlay (the body inflated by `FISH_PATTERN_DEFORMATION`) by
/// `getPatternColor().getTextureDiffuseColor()`.
#[allow(clippy::too_many_arguments)]
fn emit_tropical_fish_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    shape: TropicalFishModelShape,
    base_color: EntityDyeColor,
    pattern: TropicalFishPattern,
    pattern_color: EntityDyeColor,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let in_water = instance.render_state.in_water;
    let transform = tropical_fish_model_root_transform(instance, in_water);
    let tail_yrot = tropical_fish_tail_yrot(instance.render_state.age_in_ticks, in_water);
    for pass in tropical_fish_textured_layer_passes(shape, base_color, pattern, pattern_color) {
        if tail_yrot == 0.0 {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            parts[TROPICAL_FISH_TAIL_PART_INDEX].pose.rotation[1] = tail_yrot;
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
}

/// The textured squid / glow squid base layer. The procedural tentacle ring is not a
/// `&'static` part list, so the squid is hand-emitted: the variant texture's atlas UV is
/// resolved once and the body + animated tentacle ring (built by
/// [`squid_textured_model_parts`]) are emitted under [`squid_model_root_transform`]. The
/// glow squid differs only by texture (its emissive light boost is deferred lighting).
fn emit_squid_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    glow: bool,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let texture = squid_texture_ref(glow, baby);
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
        return;
    };
    let transform = squid_model_root_transform(instance, baby);
    let parts = squid_textured_model_parts(instance.render_state.squid_tentacle_angle);
    let mesh = meshes.mesh_mut(EntityModelLayerRenderType::Cutout);
    emit_textured_model_parts(
        mesh,
        &parts,
        transform,
        texture,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
    );
}

/// Emit one cube group at `parent · pose` into a textured mesh, mirroring the colored
/// [`emit_model_cubes_at_pose`] but for the textured atlas path. Used by the hand-emitted
/// nested hierarchies (vex, allay) where the animated children are not `&'static` parts.
fn emit_textured_cubes_at_pose(
    mesh: &mut EntityModelTexturedMesh,
    parent_transform: Mat4,
    pose: PartPose,
    cubes: &[TexturedModelCubeDesc],
    texture: EntityModelTextureRef,
    uv: EntityModelUvRect,
) {
    let transform = parent_transform * part_pose_transform(pose);
    for cube in cubes {
        emit_textured_model_cube(mesh, transform, *cube, texture, uv, [1.0, 1.0, 1.0, 1.0]);
    }
}

/// The textured vex base layer. The arms and wings hang under the body and are swayed by
/// the vanilla `VexModel.setupAnim` (idle / non-charging), so the part list is animated
/// per frame and the hierarchy is walked by hand exactly like the colored
/// [`emit_vex_model`]. Vex uses `RenderTypes::entityTranslucent`, so it draws into the
/// translucent mesh. The charging texture/pose and the held-item arms are deferred
/// entity-side state, and the vanilla full-bright block light (`getBlockLightLevel` → 15)
/// is deferred lighting.
fn emit_vex_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(entry) = entity_model_texture_atlas_entry(atlas, VEX_TEXTURE_REF) else {
        return;
    };
    let uv = entry.uv;
    let age = instance.render_state.age_in_ticks;
    let charging = instance.render_state.vex_charging;
    let root = entity_model_root_transform(instance) * part_pose_transform(VEX_ROOT_POSE);
    let mesh = meshes.mesh_mut(EntityModelLayerRenderType::Translucent);

    // Head (child of root) tracks the look yaw/pitch.
    let head_pose = PartPose {
        offset: VEX_HEAD_POSE.offset,
        rotation: [
            instance.render_state.head_pitch.to_radians(),
            instance.render_state.head_yaw.to_radians(),
            0.0,
        ],
    };
    emit_textured_cubes_at_pose(
        mesh,
        root,
        head_pose,
        &VEX_TEXTURED_HEAD,
        VEX_TEXTURE_REF,
        uv,
    );

    // Body (child of root) levels while charging, else holds the idle tilt; it carries the
    // arms and wings. While `Vex.isCharging`, `setArmsCharging` raises both arms (the
    // both-hands-empty branch — held items are not projected, so the held-item arm variant
    // `xRot = π·7/6` stays deferred).
    let body_pose = PartPose {
        offset: VEX_BODY_POSE.offset,
        rotation: [if charging { 0.0 } else { VEX_BODY_X_ROT }, 0.0, 0.0],
    };
    let body_t = root * part_pose_transform(body_pose);
    emit_textured_cubes_at_pose(
        mesh,
        root,
        body_pose,
        &VEX_TEXTURED_BODY,
        VEX_TEXTURE_REF,
        uv,
    );

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
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: VEX_RIGHT_ARM_POSE.offset,
            rotation: right_arm_rot,
        },
        &VEX_TEXTURED_RIGHT_ARM,
        VEX_TEXTURE_REF,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: VEX_LEFT_ARM_POSE.offset,
            rotation: left_arm_rot,
        },
        &VEX_TEXTURED_LEFT_ARM,
        VEX_TEXTURE_REF,
        uv,
    );

    let left_wing_yrot = vex_left_wing_y_rot(age);
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: VEX_LEFT_WING_POSE.offset,
            rotation: [VEX_WING_X_ROT, left_wing_yrot, -VEX_WING_Z_ROT],
        },
        &VEX_TEXTURED_LEFT_WING,
        VEX_TEXTURE_REF,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: VEX_RIGHT_WING_POSE.offset,
            rotation: [VEX_WING_X_ROT, -left_wing_yrot, VEX_WING_Z_ROT],
        },
        &VEX_TEXTURED_RIGHT_WING,
        VEX_TEXTURE_REF,
        uv,
    );
}

/// The textured allay base layer. Like the vex, the arms and wings hang under the body and
/// are swayed by the vanilla `AllayModel.setupAnim` (non-dancing idle / flying pose) plus
/// the vertical root bob, so the part list is animated per frame and the hierarchy is walked
/// by hand exactly like the colored [`emit_allay_model`]. Allay uses
/// `RenderTypes::entityTranslucent`, so it draws into the translucent mesh. The dance pose
/// (`isDancing`/`isSpinning`) and held-item arms are deferred entity-side state, and the
/// vanilla full-bright block light (`getBlockLightLevel` → 15) is deferred lighting.
fn emit_allay_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(entry) = entity_model_texture_atlas_entry(atlas, ALLAY_TEXTURE_REF) else {
        return;
    };
    let uv = entry.uv;
    let age = instance.render_state.age_in_ticks;
    let walk_pos = instance.render_state.walk_animation_pos;
    let walk_speed = instance.render_state.walk_animation_speed;
    let root_pose = PartPose {
        offset: [0.0, allay_root_y(age, walk_speed), 0.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let root = entity_model_root_transform(instance) * part_pose_transform(root_pose);
    let mesh = meshes.mesh_mut(EntityModelLayerRenderType::Translucent);

    // Head (child of root) tracks the look yaw/pitch.
    let head_pose = PartPose {
        offset: ALLAY_HEAD_POSE.offset,
        rotation: [
            instance.render_state.head_pitch.to_radians(),
            instance.render_state.head_yaw.to_radians(),
            0.0,
        ],
    };
    emit_textured_cubes_at_pose(
        mesh,
        root,
        head_pose,
        &ALLAY_TEXTURED_HEAD,
        ALLAY_TEXTURE_REF,
        uv,
    );

    // Body (child of root) tilts toward the flying pose and carries the arms and wings.
    let body_pose = PartPose {
        offset: ALLAY_BODY_POSE.offset,
        rotation: [allay_body_x_rot(walk_speed), 0.0, 0.0],
    };
    let body_t = root * part_pose_transform(body_pose);
    emit_textured_cubes_at_pose(
        mesh,
        root,
        body_pose,
        &ALLAY_TEXTURED_BODY,
        ALLAY_TEXTURE_REF,
        uv,
    );

    let arm_bob = allay_arm_idle_bob_amount(age, walk_speed);
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: ALLAY_RIGHT_ARM_POSE.offset,
            rotation: [0.0, 0.0, arm_bob],
        },
        &ALLAY_TEXTURED_RIGHT_ARM,
        ALLAY_TEXTURE_REF,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: ALLAY_LEFT_ARM_POSE.offset,
            rotation: [0.0, 0.0, -arm_bob],
        },
        &ALLAY_TEXTURED_LEFT_ARM,
        ALLAY_TEXTURE_REF,
        uv,
    );

    let wing_x_rot = allay_wing_rest_x_rot(walk_speed);
    let flap = allay_wing_flap_amount(age, walk_pos, walk_speed);
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: ALLAY_RIGHT_WING_POSE.offset,
            rotation: [wing_x_rot, -ALLAY_WING_Y_ROT_BASE + flap, 0.0],
        },
        &ALLAY_TEXTURED_WING,
        ALLAY_TEXTURE_REF,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        PartPose {
            offset: ALLAY_LEFT_WING_POSE.offset,
            rotation: [wing_x_rot, ALLAY_WING_Y_ROT_BASE - flap, 0.0],
        },
        &ALLAY_TEXTURED_WING,
        ALLAY_TEXTURE_REF,
        uv,
    );
}

/// The textured strider base layer. The legs swing/roll/lift, the body sways/bobs/tracks the
/// look, and the bristles flow with the walk plus an `ageInTicks` ripple, so the part list is
/// animated per frame and the hierarchy is walked by hand exactly like the colored
/// [`emit_strider_model`]. Strider uses the default `RenderTypes::entityCutout`, so it draws
/// into the cutout mesh. The ridden pose, the saddle layer, and the cold/suffocating texture
/// are deferred entity-side state.
fn emit_strider_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let texture = if baby {
        STRIDER_BABY_TEXTURE_REF
    } else {
        STRIDER_TEXTURE_REF
    };
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
        return;
    };
    let uv = entry.uv;
    let age = instance.render_state.age_in_ticks;
    let pos = instance.render_state.walk_animation_pos;
    let speed = strider_animation_speed(instance.render_state.walk_animation_speed);
    let root = entity_model_root_transform(instance);
    let body_pitch = instance.render_state.head_pitch.to_radians();
    let body_yaw = instance.render_state.head_yaw.to_radians();
    let flow = strider_bristle_flow(pos, speed);
    let mesh = meshes.mesh_mut(EntityModelLayerRenderType::Cutout);

    if baby {
        emit_textured_cubes_at_pose(
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
            &STRIDER_BABY_TEXTURED_RIGHT_LEG,
            texture,
            uv,
        );
        emit_textured_cubes_at_pose(
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
            &STRIDER_BABY_TEXTURED_LEFT_LEG,
            texture,
            uv,
        );

        let body_pose = PartPose {
            offset: [
                0.0,
                strider_body_y(STRIDER_BABY_BODY_BASE_Y, 1.0, pos, speed),
                0.0,
            ],
            rotation: [body_pitch, body_yaw, strider_body_z_rot(pos, speed)],
        };
        let body_t = root * part_pose_transform(body_pose);
        emit_textured_cubes_at_pose(
            mesh,
            root,
            body_pose,
            &STRIDER_BABY_TEXTURED_BODY,
            texture,
            uv,
        );

        for (pose_const, cubes, add) in [
            (
                STRIDER_BABY_FRONT_BRISTLE_POSE,
                &STRIDER_BABY_TEXTURED_FRONT_BRISTLE,
                strider_bristle_top_flow(flow, age),
            ),
            (
                STRIDER_BABY_MIDDLE_BRISTLE_POSE,
                &STRIDER_BABY_TEXTURED_MIDDLE_BRISTLE,
                strider_bristle_middle_flow(flow, age),
            ),
            (
                STRIDER_BABY_BACK_BRISTLE_POSE,
                &STRIDER_BABY_TEXTURED_BACK_BRISTLE,
                strider_bristle_bottom_flow(flow, age),
            ),
        ] {
            let mut pose = pose_const;
            pose.rotation[0] += add;
            emit_textured_cubes_at_pose(mesh, body_t, pose, cubes, texture, uv);
        }
        return;
    }

    emit_textured_cubes_at_pose(
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
        &STRIDER_TEXTURED_RIGHT_LEG,
        texture,
        uv,
    );
    emit_textured_cubes_at_pose(
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
        &STRIDER_TEXTURED_LEFT_LEG,
        texture,
        uv,
    );

    let body_pose = PartPose {
        offset: [
            0.0,
            strider_body_y(STRIDER_BODY_BASE_Y, 2.0, pos, speed),
            0.0,
        ],
        rotation: [body_pitch, body_yaw, strider_body_z_rot(pos, speed)],
    };
    let body_t = root * part_pose_transform(body_pose);
    emit_textured_cubes_at_pose(mesh, root, body_pose, &STRIDER_TEXTURED_BODY, texture, uv);

    let top = strider_bristle_top_flow(flow, age);
    let middle = strider_bristle_middle_flow(flow, age);
    let bottom = strider_bristle_bottom_flow(flow, age);
    for (pose_const, cubes, add) in [
        (
            STRIDER_RIGHT_TOP_BRISTLE_POSE,
            &STRIDER_TEXTURED_RIGHT_TOP_BRISTLE,
            top,
        ),
        (
            STRIDER_RIGHT_MIDDLE_BRISTLE_POSE,
            &STRIDER_TEXTURED_RIGHT_MIDDLE_BRISTLE,
            middle,
        ),
        (
            STRIDER_RIGHT_BOTTOM_BRISTLE_POSE,
            &STRIDER_TEXTURED_RIGHT_BOTTOM_BRISTLE,
            bottom,
        ),
        (
            STRIDER_LEFT_TOP_BRISTLE_POSE,
            &STRIDER_TEXTURED_LEFT_TOP_BRISTLE,
            top,
        ),
        (
            STRIDER_LEFT_MIDDLE_BRISTLE_POSE,
            &STRIDER_TEXTURED_LEFT_MIDDLE_BRISTLE,
            middle,
        ),
        (
            STRIDER_LEFT_BOTTOM_BRISTLE_POSE,
            &STRIDER_TEXTURED_LEFT_BOTTOM_BRISTLE,
            bottom,
        ),
    ] {
        let mut pose = pose_const;
        pose.rotation[2] += add;
        emit_textured_cubes_at_pose(mesh, body_t, pose, cubes, texture, uv);
    }
}

/// The textured turtle base layer. The head tracks the look, the body holds its fixed shell
/// tilt, and the four legs walk (land) or paddle (water) per [`turtle_leg_rotation`], so the
/// part list is animated per frame and emitted by hand exactly like the colored
/// [`emit_turtle_model`]. Turtle uses the default `RenderTypes::entityCutout`, so it draws into
/// the cutout mesh. The adult `egg_belly` overlay shell + `root.y--` shift follow `hasEgg`; only
/// `AdultTurtleModel` has them, so they are gated on `!baby` (the baby model has no egg belly).
/// The egg-laying leg amplitude stays deferred entity-side state.
fn emit_turtle_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let texture = if baby {
        TURTLE_BABY_TEXTURE_REF
    } else {
        TURTLE_TEXTURE_REF
    };
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
        return;
    };
    let uv = entry.uv;
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
            &TURTLE_BABY_TEXTURED_HEAD[..],
            TURTLE_BABY_HEAD_POSE,
            &TURTLE_BABY_TEXTURED_BODY[..],
            TURTLE_BABY_BODY_POSE,
            [
                (
                    &TURTLE_BABY_TEXTURED_RIGHT_HIND_LEG[..],
                    TURTLE_BABY_RIGHT_HIND_LEG_POSE,
                    false,
                    true,
                ),
                (
                    &TURTLE_BABY_TEXTURED_LEFT_HIND_LEG[..],
                    TURTLE_BABY_LEFT_HIND_LEG_POSE,
                    false,
                    false,
                ),
                (
                    &TURTLE_BABY_TEXTURED_RIGHT_FRONT_LEG[..],
                    TURTLE_BABY_RIGHT_FRONT_LEG_POSE,
                    true,
                    true,
                ),
                (
                    &TURTLE_BABY_TEXTURED_LEFT_FRONT_LEG[..],
                    TURTLE_BABY_LEFT_FRONT_LEG_POSE,
                    true,
                    false,
                ),
            ],
        )
    } else {
        (
            &TURTLE_TEXTURED_HEAD[..],
            TURTLE_HEAD_POSE,
            &TURTLE_TEXTURED_BODY[..],
            TURTLE_BODY_POSE,
            [
                (
                    &TURTLE_TEXTURED_RIGHT_HIND_LEG[..],
                    TURTLE_RIGHT_HIND_LEG_POSE,
                    false,
                    true,
                ),
                (
                    &TURTLE_TEXTURED_LEFT_HIND_LEG[..],
                    TURTLE_LEFT_HIND_LEG_POSE,
                    false,
                    false,
                ),
                (
                    &TURTLE_TEXTURED_RIGHT_FRONT_LEG[..],
                    TURTLE_RIGHT_FRONT_LEG_POSE,
                    true,
                    true,
                ),
                (
                    &TURTLE_TEXTURED_LEFT_FRONT_LEG[..],
                    TURTLE_LEFT_FRONT_LEG_POSE,
                    true,
                    false,
                ),
            ],
        )
    };

    let mesh = meshes.mesh_mut(EntityModelLayerRenderType::Cutout);

    let head_pose = PartPose {
        offset: head_pose.offset,
        rotation: [head_pitch, head_yaw, 0.0],
    };
    emit_textured_cubes_at_pose(mesh, root, head_pose, head_cubes, texture, uv);
    emit_textured_cubes_at_pose(mesh, root, body_pose, body_cubes, texture, uv);
    // The `egg_belly` overlay shell shares the body pose; only the adult model has it (the
    // projection clears `hasEgg` for babies).
    if has_egg {
        emit_textured_cubes_at_pose(
            mesh,
            root,
            TURTLE_BODY_POSE,
            &TURTLE_TEXTURED_EGG_BELLY,
            texture,
            uv,
        );
    }

    for (cubes, leg_pose, front, right) in legs {
        emit_textured_cubes_at_pose(
            mesh,
            root,
            PartPose {
                offset: leg_pose.offset,
                rotation: turtle_leg_rotation(pos, speed, on_land, front, right, laying),
            },
            cubes,
            texture,
            uv,
        );
    }
}

/// Combine a bind pose with the keyframe position/rotation offsets, mirroring the colored
/// `keyframe_animated_pose` (vanilla `ModelPart::offsetPos` / `offsetRotation` add to the bind
/// pose). Shared by the textured keyframe-animated entities.
fn keyframe_textured_pose(bind: PartPose, position: [f32; 3], rotation: [f32; 3]) -> PartPose {
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

fn emit_bat_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let texture = BAT_TEXTURE_REF;
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
        return;
    };
    let uv = entry.uv;

    // Mirror the colored `emit_bat_model`: sample the looping `BatAnimation.BAT_FLYING` (or the
    // static `BAT_RESTING` hanging pose while `isResting`) from `age_in_ticks` and walk the
    // head/ears + body/wings/feet hierarchy by hand, drawing the textured base layer into the
    // cutout mesh (vanilla `RenderTypes::entityCutoutCull`).
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
    let mesh = meshes.mesh_mut(EntityModelLayerRenderType::Cutout);

    // Head (root child) carries the two ears at their bind poses. While resting the head also
    // turns by the look yaw (`applyHeadRotation`, additive to the pose's `yRot`).
    let (head_pos, head_rot) = sample("head");
    let head_rot = [head_rot[0], head_rot[1] + head_look_yaw, head_rot[2]];
    let head_pose = keyframe_textured_pose(BAT_HEAD_POSE, head_pos, head_rot);
    let head_t = root * part_pose_transform(head_pose);
    emit_textured_cubes_at_pose(mesh, root, head_pose, &BAT_TEXTURED_HEAD, texture, uv);
    emit_textured_cubes_at_pose(
        mesh,
        head_t,
        BAT_RIGHT_EAR_POSE,
        &BAT_TEXTURED_RIGHT_EAR,
        texture,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        head_t,
        BAT_LEFT_EAR_POSE,
        &BAT_TEXTURED_LEFT_EAR,
        texture,
        uv,
    );

    // Body (root child) carries the wings and feet.
    let (body_pos, body_rot) = sample("body");
    let body_pose = keyframe_textured_pose(BAT_BODY_POSE, body_pos, body_rot);
    let body_t = root * part_pose_transform(body_pose);
    emit_textured_cubes_at_pose(mesh, root, body_pose, &BAT_TEXTURED_BODY, texture, uv);

    let (_, feet_rot) = sample("feet");
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        keyframe_textured_pose(BAT_FEET_POSE, [0.0; 3], feet_rot),
        &BAT_TEXTURED_FEET,
        texture,
        uv,
    );

    // Each wing (body child) carries its tip; the resting pose also shifts the wings by a
    // position channel (`+1` z), so sample and apply the wing positions too.
    let (right_wing_pos, right_wing_rot) = sample("right_wing");
    let right_wing_pose =
        keyframe_textured_pose(BAT_RIGHT_WING_POSE, right_wing_pos, right_wing_rot);
    let right_wing_t = body_t * part_pose_transform(right_wing_pose);
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        right_wing_pose,
        &BAT_TEXTURED_RIGHT_WING,
        texture,
        uv,
    );
    let (_, right_tip_rot) = sample("right_wing_tip");
    emit_textured_cubes_at_pose(
        mesh,
        right_wing_t,
        keyframe_textured_pose(BAT_RIGHT_WING_TIP_POSE, [0.0; 3], right_tip_rot),
        &BAT_TEXTURED_RIGHT_WING_TIP,
        texture,
        uv,
    );

    let (left_wing_pos, left_wing_rot) = sample("left_wing");
    let left_wing_pose = keyframe_textured_pose(BAT_LEFT_WING_POSE, left_wing_pos, left_wing_rot);
    let left_wing_t = body_t * part_pose_transform(left_wing_pose);
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        left_wing_pose,
        &BAT_TEXTURED_LEFT_WING,
        texture,
        uv,
    );
    let (_, left_tip_rot) = sample("left_wing_tip");
    emit_textured_cubes_at_pose(
        mesh,
        left_wing_t,
        keyframe_textured_pose(BAT_LEFT_WING_TIP_POSE, [0.0; 3], left_tip_rot),
        &BAT_TEXTURED_LEFT_WING_TIP,
        texture,
        uv,
    );
}

fn emit_bee_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let texture = if baby {
        BEE_BABY_TEXTURE_REF
    } else {
        BEE_TEXTURE_REF
    };
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
        return;
    };
    let uv = entry.uv;

    // Mirror the colored `emit_bee_model`: while airborne the wings flap and the non-angry bob
    // rocks the bone/legs/antennae; on the ground the model rests. Draws the textured base layer
    // into the cutout mesh (vanilla `RenderTypes::entityCutoutCull`).
    let age = instance.render_state.age_in_ticks;
    let flying = !instance.render_state.on_ground;
    // An angry airborne bee flaps but skips `bobUpAndDown` (see the colored path).
    let bob = flying && !instance.render_state.bee_angry;
    let root = entity_model_root_transform(instance);
    let mesh = meshes.mesh_mut(EntityModelLayerRenderType::Cutout);

    // Bone pivot (root child).
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
        emit_textured_cubes_at_pose(mesh, root, bone_pose, &BEE_BABY_TEXTURED_BONE, texture, uv);
    }

    // Body (bone child) carries the stinger and, on adults, the antennae.
    let body_pose = if baby {
        BEE_BABY_BODY_POSE
    } else {
        BEE_BODY_POSE
    };
    let body_t = bone_t * part_pose_transform(body_pose);
    emit_textured_cubes_at_pose(
        mesh,
        bone_t,
        body_pose,
        if baby {
            &BEE_BABY_TEXTURED_BODY
        } else {
            &BEE_TEXTURED_BODY
        },
        texture,
        uv,
    );
    // The stinger cube is drawn only while the bee still carries it (`stinger.visible`).
    if instance.render_state.bee_has_stinger {
        emit_textured_cubes_at_pose(
            mesh,
            body_t,
            if baby {
                BEE_BABY_STINGER_POSE
            } else {
                BEE_STINGER_POSE
            },
            if baby {
                &BEE_BABY_TEXTURED_STINGER
            } else {
                &BEE_TEXTURED_STINGER
            },
            texture,
            uv,
        );
    }
    if !baby {
        let antenna_x_rot = if bob { bee_antenna_x_rot(age) } else { 0.0 };
        emit_textured_cubes_at_pose(
            mesh,
            body_t,
            PartPose {
                offset: BEE_LEFT_ANTENNA_POSE.offset,
                rotation: [antenna_x_rot, 0.0, 0.0],
            },
            &BEE_TEXTURED_LEFT_ANTENNA,
            texture,
            uv,
        );
        emit_textured_cubes_at_pose(
            mesh,
            body_t,
            PartPose {
                offset: BEE_RIGHT_ANTENNA_POSE.offset,
                rotation: [antenna_x_rot, 0.0, 0.0],
            },
            &BEE_TEXTURED_RIGHT_ANTENNA,
            texture,
            uv,
        );
    }

    // Wings (bone children): the flap overrides the bind yaw to 0 and drives `zRot`.
    let (right_wing_pose, left_wing_pose, right_wing, left_wing): (
        _,
        _,
        &[TexturedModelCubeDesc],
        _,
    ) = if baby {
        (
            BEE_BABY_RIGHT_WING_POSE,
            BEE_BABY_LEFT_WING_POSE,
            &BEE_BABY_TEXTURED_RIGHT_WING,
            &BEE_BABY_TEXTURED_LEFT_WING,
        )
    } else {
        (
            BEE_RIGHT_WING_POSE,
            BEE_LEFT_WING_POSE,
            &BEE_TEXTURED_RIGHT_WING,
            &BEE_TEXTURED_LEFT_WING,
        )
    };
    let wing_z_rot = bee_wing_z_rot(age);
    emit_textured_cubes_at_pose(
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
        texture,
        uv,
    );
    emit_textured_cubes_at_pose(
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
        texture,
        uv,
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
        &[TexturedModelCubeDesc],
        &[TexturedModelCubeDesc],
        &[TexturedModelCubeDesc],
    ) = if baby {
        (
            BEE_BABY_FRONT_LEGS_POSE,
            BEE_BABY_MIDDLE_LEGS_POSE,
            BEE_BABY_BACK_LEGS_POSE,
            &BEE_BABY_TEXTURED_FRONT_LEGS,
            &BEE_BABY_TEXTURED_MIDDLE_LEGS,
            &BEE_BABY_TEXTURED_BACK_LEGS,
        )
    } else {
        (
            BEE_FRONT_LEGS_POSE,
            BEE_MIDDLE_LEGS_POSE,
            BEE_BACK_LEGS_POSE,
            &BEE_TEXTURED_FRONT_LEGS,
            &BEE_TEXTURED_MIDDLE_LEGS,
            &BEE_TEXTURED_BACK_LEGS,
        )
    };
    emit_textured_cubes_at_pose(
        mesh,
        bone_t,
        PartPose {
            offset: front_pose.offset,
            rotation: [front_x, 0.0, 0.0],
        },
        front_cubes,
        texture,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        bone_t,
        PartPose {
            offset: mid_pose.offset,
            rotation: [mid_x, 0.0, 0.0],
        },
        mid_cubes,
        texture,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        bone_t,
        PartPose {
            offset: back_pose.offset,
            rotation: [back_x, 0.0, 0.0],
        },
        back_cubes,
        texture,
        uv,
    );
}

fn emit_breeze_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let texture = BREEZE_TEXTURE_REF;
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
        return;
    };
    let uv = entry.uv;

    // Mirror the colored `emit_breeze_model`: sample the looping `BreezeAnimation.IDLE` from
    // `age_in_ticks` and walk the body→head/rods hierarchy by hand. The base body draws into the
    // translucent mesh (vanilla `BreezeModel` uses `RenderTypes::entityTranslucent`).
    let seconds = keyframe_elapsed_seconds(&BREEZE_IDLE, instance.render_state.age_in_ticks * 0.05);
    let sample = |bone: &str| sample_bone_offsets(&BREEZE_IDLE, bone, seconds, 1.0);
    let root = entity_model_root_transform(instance);
    let mesh = meshes.mesh_mut(EntityModelLayerRenderType::Translucent);

    // Body pivot (root child): no IDLE channel, identity bind pose.
    let body_t = root * part_pose_transform(BREEZE_BODY_POSE);

    // Head (body child): the IDLE position bob (CATMULLROM).
    let (head_pos, _) = sample("head");
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        keyframe_textured_pose(BREEZE_HEAD_POSE, head_pos, [0.0; 3]),
        &BREEZE_TEXTURED_HEAD,
        texture,
        uv,
    );

    // Rods pivot (body child): the IDLE yaw spin plus the position bob, carrying the three rods.
    let (rods_pos, rods_rot) = sample("rods");
    let rods_t =
        body_t * part_pose_transform(keyframe_textured_pose(BREEZE_RODS_POSE, rods_pos, rods_rot));
    emit_textured_cubes_at_pose(
        mesh,
        rods_t,
        BREEZE_ROD_1_POSE,
        &BREEZE_TEXTURED_ROD,
        texture,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        rods_t,
        BREEZE_ROD_2_POSE,
        &BREEZE_TEXTURED_ROD,
        texture,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        rods_t,
        BREEZE_ROD_3_POSE,
        &BREEZE_TEXTURED_ROD,
        texture,
        uv,
    );
}

fn emit_dolphin_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let texture = if baby {
        DOLPHIN_BABY_TEXTURE_REF
    } else {
        DOLPHIN_TEXTURE_REF
    };
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
        return;
    };
    let uv = entry.uv;

    // Mirror the colored `emit_dolphin_model`: steer the body by the look pitch/yaw and add the
    // swim wave while moving. The base body draws into the cutout mesh (the `DolphinModel` default
    // `RenderTypes::entityCutoutNoCull`). The baby uses the `MeshTransformer.scaling(0.5)` layer.
    let moving = instance.render_state.is_moving;
    let age = instance.render_state.age_in_ticks;
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();
    let wave = dolphin_wave(age);
    let root = mesh_transformer_scaled_model_root_transform(instance, if baby { 0.5 } else { 1.0 });
    let mesh = meshes.mesh_mut(EntityModelLayerRenderType::Cutout);

    // Body (root child) carries the fins, the tail chain, and the head chain.
    let body_pose = PartPose {
        offset: DOLPHIN_BODY_POSE.offset,
        rotation: [
            head_pitch + if moving { -0.05 - 0.05 * wave } else { 0.0 },
            head_yaw,
            0.0,
        ],
    };
    let body_t = root * part_pose_transform(body_pose);
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        PART_POSE_ZERO,
        &DOLPHIN_TEXTURED_BODY,
        texture,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        DOLPHIN_BACK_FIN_POSE,
        &DOLPHIN_TEXTURED_BACK_FIN,
        texture,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        DOLPHIN_LEFT_FIN_POSE,
        &DOLPHIN_TEXTURED_LEFT_FIN,
        texture,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        DOLPHIN_RIGHT_FIN_POSE,
        &DOLPHIN_TEXTURED_RIGHT_FIN,
        texture,
        uv,
    );

    // Tail (body child) carries the tail fin; both pitch with the swim wave while moving.
    let tail_pose = PartPose {
        offset: DOLPHIN_TAIL_POSE.offset,
        rotation: [
            if moving {
                -0.1 * wave
            } else {
                DOLPHIN_TAIL_BIND_X_ROT
            },
            0.0,
            0.0,
        ],
    };
    let tail_t = body_t * part_pose_transform(tail_pose);
    emit_textured_cubes_at_pose(mesh, body_t, tail_pose, &DOLPHIN_TEXTURED_TAIL, texture, uv);
    emit_textured_cubes_at_pose(
        mesh,
        tail_t,
        PartPose {
            offset: DOLPHIN_TAIL_FIN_POSE.offset,
            rotation: [if moving { -0.2 * wave } else { 0.0 }, 0.0, 0.0],
        },
        &DOLPHIN_TEXTURED_TAIL_FIN,
        texture,
        uv,
    );

    // Head (body child) carries the nose.
    let head_t = body_t * part_pose_transform(DOLPHIN_HEAD_POSE);
    emit_textured_cubes_at_pose(
        mesh,
        body_t,
        DOLPHIN_HEAD_POSE,
        &DOLPHIN_TEXTURED_HEAD,
        texture,
        uv,
    );
    emit_textured_cubes_at_pose(
        mesh,
        head_t,
        DOLPHIN_NOSE_POSE,
        &DOLPHIN_TEXTURED_NOSE,
        texture,
        uv,
    );
}

/// The four leg part indices in the llama body layers, matching the colored
/// `emit_llama_model`: the adult layer lists head/body at `0`/`1` then legs at
/// `[2, 3, 4, 5]`; the chest layer inserts the two chests at `2`/`3`, pushing legs to
/// `[4, 5, 6, 7]`; the baby layer lists the head at `0`, legs at `[1, 2, 3, 4]`, body
/// last. [`quadruped_leg_swing_pose`] resolves each leg's phase from its offset.
fn llama_leg_part_indices(baby: bool, has_chest: bool) -> [usize; 4] {
    if baby {
        [1, 2, 3, 4]
    } else if has_chest {
        [4, 5, 6, 7]
    } else {
        [2, 3, 4, 5]
    }
}

/// The textured llama base layer. The trader llama shares this geometry/texture; its
/// distinguishing `LlamaDecorLayer` overlay is a deferred equipment layer, so `family`
/// is not consumed here. Vanilla `LlamaModel.setupAnim` is the standard
/// `QuadrupedModel` head look plus the diagonal leg swing, both handled by
/// [`emit_quadruped_textured_passes`]; the head is part `0` in every layout.
fn emit_llama_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    variant: LlamaVariant,
    baby: bool,
    has_chest: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_quadruped_textured_passes(
        meshes,
        llama_textured_layer_passes(variant, baby, has_chest),
        0,
        llama_leg_part_indices(baby, has_chest),
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

/// `HumanoidModel` arm part indices (head/body at `0`/`1`, arms at `[2, 3]`).
const HUMANOID_ARM_PART_INDICES: [usize; 2] = [2, 3];

/// The `HumanoidModel` body part index (`head` is `0`, `body` is `1`).
const HUMANOID_BODY_PART_INDEX: usize = 1;

/// Emits the skeleton family's textured layer passes, applying the vanilla
/// `HumanoidModel.setupAnim` head look ([`head_look_pose`]) to the head part at
/// `head_index`, the leg swing ([`humanoid_leg_swing_pose`]) to the two leg parts at
/// `leg_indices`, and the inherited arm counter-swing ([`humanoid_arm_swing_pose`]) to
/// the arms at `[2, 3]`. `SkeletonModel` overrides the arms only in its melee branch
/// (`isAggressive && !isHoldingBow`) and the bow aiming is a deferred `ArmPose`, so in
/// the default state the arms swing as inherited. The static parts are reused unchanged
/// while the head is level/aligned and the limbs are at rest.
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
    let age_in_ticks = instance.render_state.age_in_ticks;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in passes {
        // The inherited `HumanoidModel.setupAnim` idle arm bob advances every frame, so the
        // arms are always re-posed — there is no static rest fast path for a humanoid.
        let mut parts = pass.parts.to_vec();
        if !head_resting {
            if let Some(head) = parts.get_mut(head_index) {
                head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
            }
        }
        if !limbs_resting {
            for index in leg_indices {
                if let Some(leg) = parts.get_mut(index) {
                    leg.pose = humanoid_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                }
            }
            for index in HUMANOID_ARM_PART_INDICES {
                if let Some(arm) = parts.get_mut(index) {
                    arm.pose = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
                }
            }
        }
        for index in HUMANOID_ARM_PART_INDICES {
            if let Some(arm) = parts.get_mut(index) {
                arm.pose = humanoid_arm_bob_pose(arm.pose, age_in_ticks);
            }
        }
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

/// Right/left leg part indices in the adult villager / witch / wandering-trader
/// textured layers: the combined `arms` part is at slot `2`, then the legs at
/// `[3, 4]`.
const VILLAGER_ADULT_LEG_PART_INDICES: [usize; 2] = [3, 4];

/// Right/left leg part indices in the baby villager textured layer, which reorders
/// the parts and lists the legs at `[1, 2]`.
const VILLAGER_BABY_LEG_PART_INDICES: [usize; 2] = [1, 2];

/// Emits a `VillagerModel`/`WanderingTraderModel` family entity's textured
/// layer passes, applying the vanilla head look ([`head_look_pose`]) to the head
/// part at `head_index` and the half-amplitude leg swing
/// ([`half_amplitude_leg_swing_pose`]) to the two leg parts at `leg_indices`. The
/// static parts are reused unchanged while both the head is level/aligned and the
/// legs are at rest. The villager unhappy head shake is deferred. The witch shares
/// this family's body layer but bobs its nose continuously, so it has its own
/// emitter ([`emit_witch_textured_model`]) rather than this shared path.
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

fn emit_creeper_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `CreeperModel.setupAnim` leg swing is the standard `QuadrupedModel`
    // formula (legs at [2, 3, 4, 5]), so reuse the quadruped textured pass emitter
    // (full head look + leg swing). The `CreeperRenderer.scale` swell inflate-and-flicker
    // is folded into the root transform; the powered charge layer is deferred.
    emit_quadruped_textured_passes(
        meshes,
        creeper_textured_layer_passes(),
        head_first_part_index(),
        QUADRUPED_LEG_PART_INDICES,
        creeper_model_root_transform(instance),
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
    // Vanilla `SpiderModel.setupAnim`: full head look, then the eight legs sweep about
    // their yRot and step about their zRot (`spider_leg_swing_pose`). Both the base and
    // eyes passes carry every part, so the swing is applied per pass. The cave spider
    // shares the model and differs only by its smaller root transform.
    let head_index = head_first_part_index();
    let transform = if cave {
        cave_spider_model_root_transform(instance)
    } else {
        entity_model_root_transform(instance)
    };
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in spider_textured_layer_passes(cave) {
        if head_resting && legs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
            continue;
        }
        let mut parts = pass.parts.to_vec();
        if !head_resting {
            if let Some(head) = parts.get_mut(head_index) {
                head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
            }
        }
        if !legs_resting {
            for (index, phase, side_sign) in spider_leg_swing_roles() {
                if let Some(leg) = parts.get_mut(index) {
                    leg.pose = spider_leg_swing_pose(
                        leg.pose,
                        phase,
                        side_sign,
                        limb_swing,
                        limb_swing_amount,
                    );
                }
            }
        }
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_enderman_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `EndermanModel extends HumanoidModel`: full head look, then the inherited
    // arm and leg swing halved and clamped to `[-0.4, 0.4]`
    // (`enderman_arm_swing_pose`/`enderman_leg_swing_pose`, arms at [2, 3], legs at
    // [4, 5]). Carrying a block then overrides both arms (`enderman_carried_arm_pose`),
    // and the creepy stare drops the head `y -= 5` while raising its hat child `y += 5`
    // (`ENDERMAN_TEXTURED_HEAD_CHILDREN_CREEPY`).
    let head_index = head_first_part_index();
    let transform = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let carrying = instance.render_state.enderman_carrying;
    let creepy = instance.render_state.enderman_creepy;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in enderman_textured_layer_passes() {
        if head_resting && limbs_resting && !carrying && !creepy {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            if !head_resting {
                if let Some(head) = parts.get_mut(head_index) {
                    head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
                }
            }
            if !limbs_resting {
                for index in HUMANOID_ARM_PART_INDICES {
                    if let Some(arm) = parts.get_mut(index) {
                        arm.pose = enderman_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
                    }
                }
                for index in HUMANOID_LEG_PART_INDICES {
                    if let Some(leg) = parts.get_mut(index) {
                        leg.pose = enderman_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                    }
                }
            }
            // Carrying a block overrides the arm swing entirely (held out front).
            if carrying {
                for index in HUMANOID_ARM_PART_INDICES {
                    if let Some(arm) = parts.get_mut(index) {
                        arm.pose = enderman_carried_arm_pose(arm.pose);
                    }
                }
            }
            // The creepy stare drops the head and raises the hat into the screech pose.
            if creepy {
                if let Some(head) = parts.get_mut(head_index) {
                    head.pose.offset[1] -= 5.0;
                    head.children = &ENDERMAN_TEXTURED_HEAD_CHILDREN_CREEPY;
                }
            }
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
}

fn emit_iron_golem_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `IronGolemModel.setupAnim`: full head look, then the legs swing
    // `±1.5 * triangleWave(pos, 13) * speed` and (default branch) the arms
    // `(-0.2 ± 1.5 * triangleWave(pos, 13)) * speed` (`iron_golem_walk_pose`). The
    // attack swing and offer-flower arm pose are deferred.
    let head_index = head_first_part_index();
    let transform = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in iron_golem_textured_layer_passes() {
        if head_resting && limbs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            if !head_resting {
                if let Some(head) = parts.get_mut(head_index) {
                    head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
                }
            }
            if !limbs_resting {
                for (index, part) in iron_golem_walk_part_roles() {
                    if let Some(limb) = parts.get_mut(index) {
                        limb.pose =
                            iron_golem_walk_pose(limb.pose, limb_swing, limb_swing_amount, part);
                    }
                }
            }
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
}

fn emit_snow_golem_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `SnowGolemModel.setupAnim`: head look, upper-body quarter-yaw twist, and
    // the two stick arms orbiting that twist (yRot + recomputed x/z). The arm orbit
    // overwrites the body-layer x/z even at rest, so the parts are always rebuilt.
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let upper_body_yrot = snow_golem_upper_body_yrot(head_yaw);
    let transform = entity_model_root_transform(instance);
    for pass in snow_golem_textured_layer_passes() {
        let mut parts = pass.parts.to_vec();
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
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_witch_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `WitchModel.setupAnim` runs the villager head look and the half-amplitude
    // leg swing (legs at `[3, 4]`), then bobs the nose continuously
    // (`witch_nose_bob_pose`, driven by `ageInTicks` and the entity id). The nose is a
    // `&'static` head child, so the head subtree is always hand-emitted with the bobbed
    // nose — its zRot is `cos(...) * 2.5°`, which is never at rest, so there is no static
    // fast path. The `isHoldingItem` nose hold pose and the combined `arms` part defer.
    let head_index = villager_head_part_index(false);
    let transform = villager_adult_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    let age_in_ticks = instance.render_state.age_in_ticks;
    let entity_id = instance.entity_id;
    for pass in witch_textured_layer_passes() {
        let mut parts = pass.parts.to_vec();
        if !head_resting {
            if let Some(head) = parts.get_mut(head_index) {
                head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
            }
        }
        if !legs_resting {
            for index in VILLAGER_ADULT_LEG_PART_INDICES {
                if let Some(leg) = parts.get_mut(index) {
                    leg.pose =
                        half_amplitude_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                }
            }
        }
        // The nose is a child of the head, whose children list is static, so emit the head
        // subtree by hand with the bobbed nose (the hat rides unchanged; the mole rides the
        // nose as its own child).
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        let mesh = meshes.mesh_mut(pass.render_type);
        for (index, part) in parts.iter().enumerate() {
            if index == head_index {
                let head_transform = transform * part_pose_transform(part.pose);
                for cube in part.cubes {
                    emit_textured_model_cube(
                        mesh,
                        head_transform,
                        *cube,
                        pass.texture,
                        entry.uv,
                        pass.tint,
                    );
                }
                let mut children = part.children.to_vec();
                children[WITCH_NOSE_CHILD_INDEX].pose = witch_nose_bob_pose(
                    children[WITCH_NOSE_CHILD_INDEX].pose,
                    age_in_ticks,
                    entity_id,
                );
                emit_textured_model_parts(
                    mesh,
                    &children,
                    head_transform,
                    pass.texture,
                    entry.uv,
                    pass.tint,
                );
            } else {
                emit_textured_model_parts(
                    mesh,
                    std::slice::from_ref(part),
                    transform,
                    pass.texture,
                    entry.uv,
                    pass.tint,
                );
            }
        }
    }
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

fn emit_ghast_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `GhastModel.setupAnim` waves the nine tentacles by `ageInTicks`
    // (`tentacle.xRot = 0.2 * sin(ageInTicks * 0.3 + i) + 0.4`, never at rest), so the
    // tentacles are always re-posed. The body is part 0; tentacles `i` are parts 1..=9.
    let age_in_ticks = instance.render_state.age_in_ticks;
    let transform = ghast_model_root_transform(instance);
    for pass in ghast_textured_layer_passes() {
        let mut parts = pass.parts.to_vec();
        for (tentacle, part) in parts.iter_mut().skip(1).enumerate() {
            part.pose.rotation[0] = ghast_tentacle_x_rot(tentacle, age_in_ticks);
        }
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_happy_ghast_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `HappyGhastModel.setupAnim` reuses `GhastModel.animateTentacles`
    // (`tentacle.xRot = 0.2 * sin(ageInTicks * 0.3 + i) + 0.4`, never at rest), so the nine
    // tentacles are always re-posed. The body is part 0; tentacles `i` are parts 1..=9.
    let age_in_ticks = instance.render_state.age_in_ticks;
    let transform = happy_ghast_model_root_transform(instance);
    for pass in happy_ghast_textured_layer_passes() {
        let mut parts = pass.parts.to_vec();
        for (tentacle, part) in parts.iter_mut().skip(1).enumerate() {
            part.pose.rotation[0] = ghast_tentacle_x_rot(tentacle, age_in_ticks);
        }
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_minecart_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `MinecartModel` has no `setupAnim`, so the cart is a static box; the textured
    // path emits the shared geometry exactly like the colored path.
    let transform = entity_model_root_transform(instance);
    for pass in minecart_textured_layer_passes() {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_armor_stand_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    small: bool,
    show_arms: bool,
    show_base_plate: bool,
    pose: ArmorStandModelPose,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Mirrors the colored `emit_armor_stand_model`: vanilla `ArmorStandModel.setupAnim` poses
    // each part from the synced pose (degrees), hides the arms/base plate by visibility, and
    // yaws the base plate by `-yRot`. The body, both body sticks, and the shoulder stick all
    // share the body pose. The geometry comes from the shared colored parts so the colored and
    // textured meshes stay identical; only the UVs differ.
    let Some(entry) = entity_model_texture_atlas_entry(atlas, ARMOR_STAND_TEXTURE_REF) else {
        return;
    };
    let mesh = meshes.mesh_mut(EntityModelLayerRenderType::Cutout);
    let parts: &[ModelPartDesc] = if small {
        &SMALL_ARMOR_STAND_PARTS
    } else {
        &ARMOR_STAND_PARTS
    };
    let transform = entity_model_root_transform(instance);
    let mut emit_part = |index: usize, rotation: [f32; 3]| {
        let part = &parts[index];
        let cube = armor_stand_textured_cube(part, ARMOR_STAND_PART_UVS[index]);
        let part_pose = PartPose {
            offset: part.pose.offset,
            rotation,
        };
        emit_textured_model_cube(
            mesh,
            transform * part_pose_transform(part_pose),
            cube,
            ARMOR_STAND_TEXTURE_REF,
            entry.uv,
            [1.0, 1.0, 1.0, 1.0],
        );
    };

    let body = degrees_to_radians3(pose.body);
    emit_part(0, degrees_to_radians3(pose.head));
    emit_part(1, body);
    if show_arms {
        emit_part(2, degrees_to_radians3(pose.right_arm));
        emit_part(3, degrees_to_radians3(pose.left_arm));
    }
    emit_part(4, degrees_to_radians3(pose.right_leg));
    emit_part(5, degrees_to_radians3(pose.left_leg));
    emit_part(6, body);
    emit_part(7, body);
    emit_part(8, body);
    if show_base_plate {
        emit_part(9, [0.0, -instance.render_state.body_rot.to_radians(), 0.0]);
    }
}

fn degrees_to_radians3(rotation: [f32; 3]) -> [f32; 3] {
    [
        rotation[0].to_radians(),
        rotation[1].to_radians(),
        rotation[2].to_radians(),
    ]
}

/// Emits a zombie-family entity's textured layer passes, applying the vanilla
/// `HumanoidModel.setupAnim` head look ([`head_look_pose`]) and leg swing
/// ([`humanoid_leg_swing_pose`]) plus the `ZombieModel.setupAnim` held-out arm pose
/// ([`zombie_arm_held_out_pose`], which overrides the inherited arm swing and carries the
/// always-on idle bob). Shared by the zombie, husk, drowned, and zombie-villager textured
/// renders; the caller supplies the per-family passes, head index, and root transform.
/// Because the held-out arms re-pose every frame, there is no static rest fast path.
fn emit_zombie_family_textured_passes(
    meshes: &mut EntityModelTexturedMeshes,
    passes: Vec<EntityModelLayerPass>,
    head_index: usize,
    transform: Mat4,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let age_in_ticks = instance.render_state.age_in_ticks;
    let aggressive = instance.render_state.is_aggressive;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in passes {
        let mut parts = pass.parts.to_vec();
        if !head_resting {
            if let Some(head) = parts.get_mut(head_index) {
                head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
            }
        }
        if !limbs_resting {
            for index in HUMANOID_LEG_PART_INDICES {
                if let Some(leg) = parts.get_mut(index) {
                    leg.pose = humanoid_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                }
            }
        }
        for index in HUMANOID_ARM_PART_INDICES {
            if let Some(arm) = parts.get_mut(index) {
                arm.pose = zombie_arm_held_out_pose(arm.pose, aggressive, age_in_ticks);
            }
        }
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_zombie_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Mirrors the colored `emit_zombie_model`: vanilla `HumanoidModel.setupAnim` runs the head
    // look and the leg swing, then `ZombieModel` overrides the arms with the held-out
    // `animateZombieArms` pose (`zombie_arm_held_out_pose`). The baby layer's head is part 1
    // (the body is part 0); the adult head is part 0.
    let head_index = if baby { 1 } else { 0 };
    let transform = entity_model_root_transform(instance);
    emit_zombie_family_textured_passes(
        meshes,
        zombie_textured_layer_passes(baby),
        head_index,
        transform,
        instance,
        atlas,
    );
}

fn emit_husk_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Mirrors the colored `emit_zombie_variant_model` husk arm: `HuskRenderer extends
    // ZombieRenderer`, so the husk reuses the zombie body parts and the same `HumanoidModel`
    // head-look + leg-swing plus the held-out `animateZombieArms` arm pose. Vanilla scales the
    // adult husk mesh by 1.0625 (`huskScale`); the baby husk reuses the unscaled `babyZombieLayer`.
    let head_index = if baby { 1 } else { 0 };
    let transform = if baby {
        entity_model_root_transform(instance)
    } else {
        mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE)
    };
    emit_zombie_family_textured_passes(
        meshes,
        husk_textured_layer_passes(baby),
        head_index,
        transform,
        instance,
        atlas,
    );
}

fn emit_drowned_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Mirrors the colored `emit_zombie_variant_model` drowned arm: `DrownedModel extends
    // ZombieModel`, so the non-swimming drowned runs the same `HumanoidModel` head-look +
    // leg-swing with the held-out `animateZombieArms` arms. The `DrownedOuterLayer`, the
    // `setupRotations`/`setupAnim` swim re-pose (needs `swimAmount`), and the trident throw
    // arm pose (needs a held item) all stay deferred. Drowned has no root scale.
    let head_index = if baby { 1 } else { 0 };
    let transform = entity_model_root_transform(instance);
    emit_zombie_family_textured_passes(
        meshes,
        drowned_textured_layer_passes(baby),
        head_index,
        transform,
        instance,
        atlas,
    );
}

fn emit_zombie_villager_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Mirrors the colored `emit_zombie_variant_model` zombie-villager arm: `ZombieVillagerModel
    // extends HumanoidModel` and runs `super.setupAnim` (head look + leg swing) then
    // `AnimationUtils.animateZombieArms` (the held-out arms). The baby layout's head is part 1
    // (the body is part 0); the adult head is part 0. The hatted base layer is emitted; the
    // no-hat model selection and the profession/type/level overlays stay deferred. Zombie
    // villagers have no root scale.
    let head_index = if baby { 1 } else { 0 };
    let transform = entity_model_root_transform(instance);
    emit_zombie_family_textured_passes(
        meshes,
        zombie_villager_textured_layer_passes(baby),
        head_index,
        transform,
        instance,
        atlas,
    );
}

fn emit_piglin_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: PiglinModelFamily,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Mirrors the colored `emit_piglin_model`: `AbstractPiglinModel.setupAnim` runs the inherited
    // head look + leg swing + arm counter-swing, then always flaps the ears (`piglin_ear_flap_pose`)
    // — the ears are nested head children, so the head subtree is hand-emitted with the flapped
    // ears (the hoglin pattern). The zombified piglin overwrites the arms with the held-out
    // `animateZombieArms` pose (deferred), so it skips the arm swing; the brute is never baby.
    // The dance/attack/crossbow/admire arm poses and the held items stay deferred.
    let baby_layout = baby && family != PiglinModelFamily::PiglinBrute;
    let head_index = piglin_head_part_index(baby_layout);
    let (left_ear, right_ear) = if baby_layout { (1, 2) } else { (0, 1) };
    let default_ear_angle = if baby_layout {
        PIGLIN_BABY_EAR_ANGLE
    } else {
        PIGLIN_ADULT_EAR_ANGLE
    };
    let swing_arms = family != PiglinModelFamily::ZombifiedPiglin;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let age_in_ticks = instance.render_state.age_in_ticks;
    let transform = entity_model_root_transform(instance);
    for pass in piglin_textured_layer_passes(family, baby_layout) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        let mut parts = pass.parts.to_vec();
        if let Some(head) = parts.get_mut(head_index) {
            head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
        }
        for index in HUMANOID_LEG_PART_INDICES {
            if let Some(leg) = parts.get_mut(index) {
                leg.pose = humanoid_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
            }
        }
        if swing_arms {
            // The inherited arm counter-swing plus the always-on `HumanoidModel.setupAnim`
            // idle arm bob (`humanoid_arm_bob_pose`). The zombified piglin's deferred held-out
            // arms carry their own bob, so this whole branch is skipped for it.
            for index in HUMANOID_ARM_PART_INDICES {
                if let Some(arm) = parts.get_mut(index) {
                    let swung = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
                    arm.pose = humanoid_arm_bob_pose(swung, age_in_ticks);
                }
            }
        }
        let mesh = meshes.mesh_mut(pass.render_type);
        for (index, part) in parts.iter().enumerate() {
            if index == head_index {
                // The ears are nested head children, so emit the head cubes then the children
                // with the flapped ear poses (vanilla flaps the ears every frame).
                let head_transform = transform * part_pose_transform(part.pose);
                for cube in part.cubes {
                    emit_textured_model_cube(
                        mesh,
                        head_transform,
                        *cube,
                        pass.texture,
                        entry.uv,
                        pass.tint,
                    );
                }
                let mut children = part.children.to_vec();
                children[left_ear].pose = piglin_ear_flap_pose(
                    children[left_ear].pose,
                    true,
                    default_ear_angle,
                    age_in_ticks,
                    limb_swing,
                    limb_swing_amount,
                );
                children[right_ear].pose = piglin_ear_flap_pose(
                    children[right_ear].pose,
                    false,
                    default_ear_angle,
                    age_in_ticks,
                    limb_swing,
                    limb_swing_amount,
                );
                emit_textured_model_parts(
                    mesh,
                    &children,
                    head_transform,
                    pass.texture,
                    entry.uv,
                    pass.tint,
                );
            } else {
                emit_textured_model_parts(
                    mesh,
                    std::slice::from_ref(part),
                    transform,
                    pass.texture,
                    entry.uv,
                    pass.tint,
                );
            }
        }
    }
}

fn emit_illager_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: IllagerModelFamily,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Mirrors the colored `emit_illager_model`: `IllagerModel.setupAnim` runs the head look, then
    // the half-amplitude leg swing (`cos(pos * 0.6662 [+ π]) * 1.4 * speed * 0.5`). The separate
    // arms swing with the `HumanoidModel` amplitude, but only the pillager renders the uncrossed
    // arms; the idle evoker/vindicator/illusioner show the static folded `arms` part (vanilla
    // swings the *invisible* separate arms). When `SpellcasterIllager.isCastingSpell()`, the
    // evoker/illusioner swap to the uncrossed layout (hiding the crossed `arms`) and raise both
    // arms into the `SPELLCASTING` pose ([`illager_spellcast_arm_pose`]). The other arm-pose
    // overrides (attack/bow/crossbow/celebrate), the riding sit pose, and the item-in-hand layers
    // stay deferred.
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let age = instance.render_state.age_in_ticks;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    let spellcasting = instance.render_state.illager_spellcasting
        && matches!(
            family,
            IllagerModelFamily::Evoker | IllagerModelFamily::Illusioner
        );
    let transform = villager_adult_model_root_transform(instance);
    // The spellcasting evoker/illusioner use the uncrossed layout (legs `[2, 3]`, arms `[4, 5]`).
    let (leg_indices, arm_indices): ([usize; 2], Option<[usize; 2]>) = if spellcasting {
        ([2, 3], Some([4, 5]))
    } else {
        match family {
            IllagerModelFamily::Pillager => ([2, 3], Some([4, 5])),
            IllagerModelFamily::Evoker
            | IllagerModelFamily::Vindicator
            | IllagerModelFamily::Illusioner => ([3, 4], None),
        }
    };
    for pass in illager_textured_layer_passes(family) {
        // The spellcasting arms are raised even at rest, so the static fast path cannot be used.
        if head_resting && limbs_resting && !spellcasting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
            continue;
        }
        let base_parts: &[TexturedModelPartDesc] = if spellcasting {
            illager_textured_spellcasting_parts(family)
        } else {
            pass.parts
        };
        let mut parts = base_parts.to_vec();
        if !head_resting {
            if let Some(head) = parts.get_mut(0) {
                head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
            }
        }
        if !limbs_resting {
            for index in leg_indices {
                if let Some(leg) = parts.get_mut(index) {
                    leg.pose =
                        half_amplitude_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                }
            }
        }
        if spellcasting {
            // Vanilla overwrites both arms' rotations with the spellcasting pose; arms `[4, 5]`
            // are right then left.
            if let Some(arm) = parts.get_mut(4) {
                arm.pose = illager_spellcast_arm_pose(arm.pose, age, true);
            }
            if let Some(arm) = parts.get_mut(5) {
                arm.pose = illager_spellcast_arm_pose(arm.pose, age, false);
            }
        } else if !limbs_resting {
            if let Some(arm_indices) = arm_indices {
                for index in arm_indices {
                    if let Some(arm) = parts.get_mut(index) {
                        arm.pose = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
                    }
                }
            }
        }
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_blaze_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `BlazeModel.setupAnim` re-positions all twelve rods from `ageInTicks` every
    // frame (`blaze_rod_offset`); the head (part 0) follows the plain `head_look_pose`. The
    // rods are parts 1..=12; there is no walk swing.
    let age_in_ticks = instance.render_state.age_in_ticks;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let transform = entity_model_root_transform(instance);
    for pass in blaze_textured_layer_passes() {
        let mut parts = pass.parts.to_vec();
        if !head_resting {
            parts[0].pose = head_look_pose(parts[0].pose, head_yaw, head_pitch);
        }
        for index in 0..BLAZE_ROD_COUNT {
            parts[index + 1].pose.offset = blaze_rod_offset(index, age_in_ticks);
        }
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_endermite_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `EndermiteModel.setupAnim` wiggles all four chitin segments from `ageInTicks`
    // every frame (`endermite_segment_pose`); there is no head look or walk swing.
    let age_in_ticks = instance.render_state.age_in_ticks;
    let transform = entity_model_root_transform(instance);
    for pass in endermite_textured_layer_passes() {
        let mut parts = pass.parts.to_vec();
        for (index, part) in parts.iter_mut().enumerate() {
            part.pose = endermite_segment_pose(part.pose, index, age_in_ticks);
        }
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_silverfish_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `SilverfishModel.setupAnim` wiggles all seven body segments from `ageInTicks`
    // every frame (`silverfish_segment_pose`), then the three overlay layers copy segments
    // 2/4/1 (`silverfish_layer_pose` per `SILVERFISH_LAYER_RULES`).
    let age_in_ticks = instance.render_state.age_in_ticks;
    let transform = entity_model_root_transform(instance);
    for pass in silverfish_textured_layer_passes() {
        let mut parts = pass.parts.to_vec();
        for index in 0..SILVERFISH_SEGMENT_COUNT {
            parts[index].pose = silverfish_segment_pose(parts[index].pose, index, age_in_ticks);
        }
        for (layer, &(source, copy_x)) in SILVERFISH_LAYER_RULES.iter().enumerate() {
            let source_pose = parts[source].pose;
            let part = &mut parts[SILVERFISH_SEGMENT_COUNT + layer];
            part.pose = silverfish_layer_pose(part.pose, source_pose, copy_x);
        }
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_phantom_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    size: i32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla `PhantomModel.setupAnim` flaps the nested wing/tail chains from `flapTime`
    // (`id*3 + ageInTicks`); the hierarchy is walked by hand so the animated descendants can
    // be re-posed. The size scale and body pitch live in the root transform.
    let root = phantom_model_root_transform(instance, size);
    let flap = phantom_flap_time(instance.entity_id, instance.render_state.age_in_ticks);
    let wing_z = phantom_wing_z_rot(flap);
    let tail_x = phantom_tail_x_rot(flap);
    for pass in phantom_textured_layer_passes() {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        let mesh = meshes.mesh_mut(pass.render_type);
        let (tex, uv, tint) = (pass.texture, entry.uv, pass.tint);
        let mut emit = |transform: Mat4, cube| {
            emit_textured_model_cube(mesh, transform, cube, tex, uv, tint);
        };

        let body_t = root * part_pose_transform(PHANTOM_BODY_POSE);
        emit(body_t, PHANTOM_BODY_TEXTURED_CUBE);

        let tail_base_t =
            body_t * part_pose_transform(phantom_tail_pose(PHANTOM_TAIL_BASE_POSE, tail_x));
        emit(tail_base_t, PHANTOM_TAIL_BASE_TEXTURED_CUBE);
        let tail_tip_t =
            tail_base_t * part_pose_transform(phantom_tail_pose(PHANTOM_TAIL_TIP_POSE, tail_x));
        emit(tail_tip_t, PHANTOM_TAIL_TIP_TEXTURED_CUBE);

        let left_base_t =
            body_t * part_pose_transform(phantom_wing_pose(PHANTOM_LEFT_WING_BASE_POSE, wing_z));
        emit(left_base_t, PHANTOM_LEFT_WING_BASE_TEXTURED_CUBE);
        let left_tip_t = left_base_t
            * part_pose_transform(phantom_wing_pose(PHANTOM_LEFT_WING_TIP_POSE, wing_z));
        emit(left_tip_t, PHANTOM_LEFT_WING_TIP_TEXTURED_CUBE);

        let right_base_t =
            body_t * part_pose_transform(phantom_wing_pose(PHANTOM_RIGHT_WING_BASE_POSE, -wing_z));
        emit(right_base_t, PHANTOM_RIGHT_WING_BASE_TEXTURED_CUBE);
        let right_tip_t = right_base_t
            * part_pose_transform(phantom_wing_pose(PHANTOM_RIGHT_WING_TIP_POSE, -wing_z));
        emit(right_tip_t, PHANTOM_RIGHT_WING_TIP_TEXTURED_CUBE);

        emit(
            body_t * part_pose_transform(PHANTOM_HEAD_POSE),
            PHANTOM_HEAD_TEXTURED_CUBE,
        );
    }
}

fn emit_pufferfish_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    puff_state: i32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Vanilla picks the small/mid/big model by puff state; each wiggles its two fins on
    // `ageInTicks`. A single cutout pass over `pufferfish.png` (no eyes layer).
    let Some(entry) = entity_model_texture_atlas_entry(atlas, PUFFERFISH_TEXTURE_REF) else {
        return;
    };
    let mesh = meshes.mesh_mut(EntityModelLayerRenderType::Cutout);
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
        emit_textured_model_cube(
            mesh,
            root * part_pose_transform(pose),
            part.textured_cube(),
            PUFFERFISH_TEXTURE_REF,
            entry.uv,
            [1.0, 1.0, 1.0, 1.0],
        );
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
    // left-hind in phase) after the yaw-only head look, and sways the ears
    // `ear.zRot = ±2π/9 ± speed * sin(pos)` (the literal 2π/9, which also overrides the
    // baby layer's wider ear rest angle). Legs are at [2, 3, 4, 5]; the headbutt is deferred.
    let head_index = hoglin_head_part_index(baby);
    let transform = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_yaw_at_rest(head_yaw);
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    // The adult ears rest at ±2π/9, so they only need re-posing when walking; the baby ears
    // rest at a wider angle that vanilla overrides to ±2π/9, so they are always re-posed.
    let pose_ears = baby || !legs_resting;
    for pass in hoglin_textured_layer_passes(family, baby) {
        if !pose_ears && head_resting && legs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
            continue;
        }
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
        if !pose_ears {
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
            continue;
        }
        // The ears are children of the head, whose children list is static, so emit the
        // head subtree by hand with the posed ears (the horns ride unchanged).
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        let mesh = meshes.mesh_mut(pass.render_type);
        for (index, part) in parts.iter().enumerate() {
            if index == head_index {
                let head_transform = transform * part_pose_transform(part.pose);
                for cube in part.cubes {
                    emit_textured_model_cube(
                        mesh,
                        head_transform,
                        *cube,
                        pass.texture,
                        entry.uv,
                        pass.tint,
                    );
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
                emit_textured_model_parts(
                    mesh,
                    &children,
                    head_transform,
                    pass.texture,
                    entry.uv,
                    pass.tint,
                );
            } else {
                emit_textured_model_parts(
                    mesh,
                    std::slice::from_ref(part),
                    transform,
                    pass.texture,
                    entry.uv,
                    pass.tint,
                );
            }
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
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    // Vanilla `RavagerModel.setupAnim` swings the four legs `cos(pos * 0.6662 [+ π]) *
    // 0.4 * speed` (legs at [2, 3, 4, 5]); the neck (part 0) is untouched by the swing.
    let leg_indices: [usize; 4] = [2, 3, 4, 5];
    for pass in ravager_textured_layer_passes() {
        if head_resting && legs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
            continue;
        }
        let mut parts = pass.parts.to_vec();
        if !legs_resting {
            for index in leg_indices {
                if let Some(leg) = parts.get_mut(index) {
                    leg.pose = ravager_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                }
            }
        }
        if head_resting {
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
            continue;
        }
        // Vanilla nests the ravager head inside the neck (`neck.getChild("head")`).
        // The neck's children list is static, so emit the neck subtree by hand,
        // applying the look to the head child (its horn/mouth children inherit it).
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        let mesh = meshes.mesh_mut(pass.render_type);
        let neck = &parts[ravager_neck_part_index()];
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
            &parts[ravager_neck_part_index() + 1..],
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
    let age_in_ticks = instance.render_state.age_in_ticks;
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
        // `PlayerModel` inherits the `HumanoidModel` arm swing (its `setupAnim` only
        // toggles visibility), so the arms counter-swing too; the sleeve children ride
        // the arm parts. Held-item/attack/crouch/swim arm poses still defer.
        for index in HUMANOID_ARM_PART_INDICES {
            if let Some(arm) = visible_parts.get_mut(index) {
                arm.pose = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
            }
        }
    }
    // The inherited `HumanoidModel.setupAnim` idle arm bob (`humanoid_arm_bob_pose`) rides
    // on top of the swing every frame, so it is applied unconditionally — even a standing
    // player's arms bob with `ageInTicks`.
    for index in HUMANOID_ARM_PART_INDICES {
        if let Some(arm) = visible_parts.get_mut(index) {
            arm.pose = humanoid_arm_bob_pose(arm.pose, age_in_ticks);
        }
    }
    // The `HumanoidModel.setupAnim` crouch (`isCrouching`) sneaking pose: lean the body, drop
    // the head, tilt the arms and tuck the legs (the hat/jacket/sleeve/pants children ride the
    // shifted parts). Applied after the swing/bob, exactly as vanilla does.
    if instance.render_state.is_crouching {
        if let Some(head) = visible_parts.get_mut(player_head_part_index()) {
            head.pose = humanoid_crouch_head_pose(head.pose);
        }
        if let Some(body) = visible_parts.get_mut(HUMANOID_BODY_PART_INDEX) {
            body.pose = humanoid_crouch_body_pose(body.pose);
        }
        for index in HUMANOID_ARM_PART_INDICES {
            if let Some(arm) = visible_parts.get_mut(index) {
                arm.pose = humanoid_crouch_arm_pose(arm.pose);
            }
        }
        for index in HUMANOID_LEG_PART_INDICES {
            if let Some(leg) = visible_parts.get_mut(index) {
                leg.pose = humanoid_crouch_leg_pose(leg.pose);
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
    // Vanilla `WolfModel.setupAnim` (adult and baby) sets `tail.yRot` (angry → 0, else the
    // wag), then either folds into the sitting pose or swings the four legs with the
    // `QuadrupedModel` diagonal phase, then applies the head look, then sets `tail.xRot =
    // tailAngle` — the `π/5` rest droop for an untamed wolf or the tame/health droop `(0.55
    // - damageRatio * 0.4) * π` from `wolf_tail_angle`. A sitting wolf (`isSitting`) tilts
    // its body and tucks its legs (`setSittingPose`) instead of the leg swing; the head
    // still follows the look. Every pass (base, collar) shares the body-layer part layout,
    // so the poses apply per pass. The adult layer lists the legs at [3, 4, 5, 6] and the
    // tail at 7 (head/body/mane at 0/1/2); the baby layer drops the mane, so the legs are at
    // [2, 3, 4, 5] and the tail at 6. The water-shake body roll is deferred.
    let leg_indices: [usize; 4] = if baby { [2, 3, 4, 5] } else { [3, 4, 5, 6] };
    let tail_index = wolf_tail_part_index(baby);
    let head_index = head_first_part_index();
    let transform = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let tail_angle = instance.render_state.wolf_tail_angle;
    let sitting = instance.render_state.wolf_sitting;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in wolf_textured_layer_passes(baby, tame, angry, invisible, collar_color) {
        // A sitting or angry wolf always re-poses (the sitting fold / tail raise override the
        // layer rest even when standing); a standing non-angry one re-poses only when the wag
        // or the `tail_angle` droop moves the tail off its layer rest pose, so an untamed
        // standing wolf can still take the borrow fast path.
        let tail_moves = angry
            || sitting
            || pass.parts.get(tail_index).is_some_and(|tail| {
                wolf_tail_swing_pose(tail.pose, tail_angle, limb_swing, limb_swing_amount)
                    != tail.pose
            });
        if head_resting && limbs_resting && !tail_moves {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            if !head_resting {
                if let Some(head) = parts.get_mut(head_index) {
                    head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
                }
            }
            if sitting {
                for (index, role) in wolf_sitting_part_roles(baby) {
                    if let Some(part) = parts.get_mut(index) {
                        apply_wolf_sitting_pose(&mut part.pose, role, baby);
                    }
                }
            } else if !limbs_resting {
                for index in leg_indices {
                    if let Some(leg) = parts.get_mut(index) {
                        leg.pose =
                            quadruped_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
                    }
                }
            }
            if let Some(tail) = parts.get_mut(tail_index) {
                // The sitting role already lifted the tail offset (if sitting); layer on the
                // normal tail rotation, which preserves the offset.
                tail.pose = if angry {
                    wolf_angry_tail_pose(tail.pose)
                } else {
                    wolf_tail_swing_pose(tail.pose, tail_angle, limb_swing, limb_swing_amount)
                };
            }
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
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
