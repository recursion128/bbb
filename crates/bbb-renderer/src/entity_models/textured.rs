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
        apply_wolf_sitting_pose, armor_stand_textured_cube, bee_antenna_x_rot, bee_back_leg_x_rot,
        bee_bone_x_rot, bee_bone_y_delta, bee_front_leg_x_rot, bee_wing_z_rot,
        camel_clamped_head_look, dolphin_wave, head_first_part_index, head_look_at_rest,
        head_look_pose, humanoid_arm_bob_pose, humanoid_arm_swing_pose, humanoid_leg_swing_pose,
        limb_swing_at_rest, parched_head_part_index, pufferfish_fin_pose, pufferfish_parts,
        pufferfish_right_fin_z_rot, quadruped_leg_swing_pose, skeleton_head_part_index,
        strider_animation_speed, strider_body_y, strider_body_z_rot, strider_bristle_bottom_flow,
        strider_bristle_flow, strider_bristle_middle_flow, strider_bristle_top_flow,
        strider_leg_x_rot, strider_leg_y, strider_leg_z_rot, turtle_leg_rotation,
        wolf_angry_tail_pose, wolf_sitting_part_roles, wolf_tail_part_index, wolf_tail_swing_pose,
        AllayModel, BlazeModel, CamelWalkLayout, ChickenModel, CodModel, CowModel, CreeperModel,
        EndermanModel, EndermiteModel, GhastModel, GoatModel, HappyGhastModel, HoglinModel,
        IllagerModel, IronGolemModel, LlamaModel, MagmaCubeModel, MinecartModel, PhantomModel,
        PigModel, PiglinModel, PlayerModel, PolarBearModel, RavagerModel, SalmonModel,
        SheepFurModel, SheepModel, SilverfishModel, SkeletonModel, SlimeModel, SlimeOuterModel,
        SnowGolemModel, SpiderModel, SquidModel, TropicalFishModel, TropicalFishPatternModel,
        VexModel, VillagerModel, WanderingTraderModel, WitchModel, ZombieModel, ZombieVariantModel,
        ADULT_CAMEL_WALK_LAYOUT, ALLAY_TEXTURE_REF, ARMOR_STAND_PARTS, ARMOR_STAND_PART_UVS,
        ARMOR_STAND_TEXTURE_REF, BABY_CAMEL_WALK_LAYOUT, BAT_BODY_POSE, BAT_FEET_POSE, BAT_FLYING,
        BAT_HEAD_POSE, BAT_LEFT_EAR_POSE, BAT_LEFT_WING_POSE, BAT_LEFT_WING_TIP_POSE, BAT_RESTING,
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
        BEE_TEXTURED_RIGHT_WING, BEE_TEXTURED_STINGER, BEE_TEXTURE_REF, BREEZE_BODY_POSE,
        BREEZE_HEAD_POSE, BREEZE_IDLE, BREEZE_RODS_POSE, BREEZE_ROD_1_POSE, BREEZE_ROD_2_POSE,
        BREEZE_ROD_3_POSE, BREEZE_TEXTURED_HEAD, BREEZE_TEXTURED_ROD, BREEZE_TEXTURE_REF,
        CAMEL_WALK_SCALE_FACTOR, CAMEL_WALK_SPEED_FACTOR, COD_TEXTURE_REF,
        DOLPHIN_BABY_TEXTURE_REF, DOLPHIN_BACK_FIN_POSE, DOLPHIN_BODY_POSE, DOLPHIN_HEAD_POSE,
        DOLPHIN_LEFT_FIN_POSE, DOLPHIN_NOSE_POSE, DOLPHIN_RIGHT_FIN_POSE, DOLPHIN_TAIL_BIND_X_ROT,
        DOLPHIN_TAIL_FIN_POSE, DOLPHIN_TAIL_POSE, DOLPHIN_TEXTURED_BACK_FIN, DOLPHIN_TEXTURED_BODY,
        DOLPHIN_TEXTURED_HEAD, DOLPHIN_TEXTURED_LEFT_FIN, DOLPHIN_TEXTURED_NOSE,
        DOLPHIN_TEXTURED_RIGHT_FIN, DOLPHIN_TEXTURED_TAIL, DOLPHIN_TEXTURED_TAIL_FIN,
        DOLPHIN_TEXTURE_REF, PUFFERFISH_TEXTURE_REF, SMALL_ARMOR_STAND_PARTS,
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
        STRIDER_TEXTURE_REF, TURTLE_BABY_BODY_POSE, TURTLE_BABY_HEAD_POSE,
        TURTLE_BABY_LEFT_FRONT_LEG_POSE, TURTLE_BABY_LEFT_HIND_LEG_POSE,
        TURTLE_BABY_RIGHT_FRONT_LEG_POSE, TURTLE_BABY_RIGHT_HIND_LEG_POSE,
        TURTLE_BABY_TEXTURED_BODY, TURTLE_BABY_TEXTURED_HEAD, TURTLE_BABY_TEXTURED_LEFT_FRONT_LEG,
        TURTLE_BABY_TEXTURED_LEFT_HIND_LEG, TURTLE_BABY_TEXTURED_RIGHT_FRONT_LEG,
        TURTLE_BABY_TEXTURED_RIGHT_HIND_LEG, TURTLE_BABY_TEXTURE_REF, TURTLE_BODY_POSE,
        TURTLE_EGG_ROOT_DROP_POSE, TURTLE_HEAD_POSE, TURTLE_LEFT_FRONT_LEG_POSE,
        TURTLE_LEFT_HIND_LEG_POSE, TURTLE_RIGHT_FRONT_LEG_POSE, TURTLE_RIGHT_HIND_LEG_POSE,
        TURTLE_TEXTURED_BODY, TURTLE_TEXTURED_EGG_BELLY, TURTLE_TEXTURED_HEAD,
        TURTLE_TEXTURED_LEFT_FRONT_LEG, TURTLE_TEXTURED_LEFT_HIND_LEG,
        TURTLE_TEXTURED_RIGHT_FRONT_LEG, TURTLE_TEXTURED_RIGHT_HIND_LEG, TURTLE_TEXTURE_REF,
        VEX_TEXTURE_REF,
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
    iron_golem_textured_layer_passes, llama_textured_layer_passes,
    magma_cube_textured_layer_passes, minecart_textured_layer_passes,
    phantom_textured_layer_passes, pig_textured_layer_passes, piglin_textured_layer_passes,
    player_textured_layer_passes, polar_bear_textured_layer_passes, ravager_textured_layer_passes,
    salmon_textured_layer_passes, sheep_textured_layer_passes, silverfish_textured_layer_passes,
    skeleton_textured_layer_passes, slime_textured_layer_passes, snow_golem_textured_layer_passes,
    spider_textured_layer_passes, tropical_fish_textured_layer_passes,
    villager_textured_layer_passes, wandering_trader_textured_layer_passes,
    witch_textured_layer_passes, wolf_textured_layer_passes, zombie_textured_layer_passes,
    zombie_villager_textured_layer_passes, EntityModelLayerPass, EntityModelLayerRenderType,
};
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
                // The unified `SkeletonModel` tree drives both render paths; `setup_anim` looks the
                // head and runs the shared humanoid arm + leg walk swing once. Variants (stray/bogged
                // clothing, wither/parched) keep the family-parameterized emitter below.
                let transform = entity_model_root_transform(*instance);
                let mut model = SkeletonModel::new();
                model.prepare(instance);
                for pass in skeleton_textured_layer_passes(None) {
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
    // The unified `ChickenModel` tree drives both render paths; `setup_anim` swings the two legs once.
    // The chicken has no head look; its wing flap is driven by the untracked `flap`/`flapSpeed` state.
    let transform = entity_model_root_transform(instance);
    let mut model = ChickenModel::new(variant, baby);
    model.prepare(&instance);
    for pass in chicken_textured_layer_passes(variant, baby) {
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

fn emit_pig_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    variant: PigModelVariant,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `PigModel` tree drives both render paths; `setup_anim` looks the head and swings
    // the four legs once.
    let transform = entity_model_root_transform(instance);
    let mut model = PigModel::new(variant, baby);
    model.prepare(&instance);
    for pass in pig_textured_layer_passes(variant, baby) {
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

fn emit_cow_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    variant: CowModelVariant,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `CowModel` tree drives both render paths; `setup_anim` looks the head and swings
    // the four legs once.
    let transform = entity_model_root_transform(instance);
    let mut model = CowModel::new(variant, baby);
    model.prepare(&instance);
    for pass in cow_textured_layer_passes(variant, baby) {
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

/// The textured tropical fish base layer plus the `TropicalFishPatternLayer` overlay. The unified
/// [`TropicalFishModel`] (base body) and [`TropicalFishPatternModel`] (the overlay, inflated by
/// `FISH_PATTERN_DEFORMATION`) trees both run the shared `TropicalFish{Small,Large}Model.setupAnim`
/// tail sway; the swim wiggle, out-of-water flop, and small/large body shape live in
/// [`tropical_fish_model_root_transform`]. Each pass routes to the base body (tinted by `getModelTint`
/// = `getBaseColor().getTextureDiffuseColor()`) or the pattern overlay (tinted by
/// `getPatternColor().getTextureDiffuseColor()`), in the pre-sorted layer order.
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
    let mut body = TropicalFishModel::new(shape);
    body.prepare(&instance);
    let mut overlay = TropicalFishPatternModel::new(shape);
    overlay.prepare(&instance);
    for pass in tropical_fish_textured_layer_passes(shape, base_color, pattern, pattern_color) {
        let root = if pass.kind == layers::EntityModelLayerKind::TropicalFishPattern {
            overlay.root()
        } else {
            body.root()
        };
        if let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) {
            root.render_textured(
                meshes.mesh_mut(pass.render_type),
                transform,
                pass.texture,
                entry.uv,
                pass.tint,
            );
        }
    }
}

/// The textured squid / glow squid base layer. The unified [`SquidModel`] tree (body + the
/// procedural eight-tentacle ring) runs the shared `SquidModel.setupAnim` and renders under
/// [`squid_model_root_transform`]; the variant texture's atlas UV is resolved once. The glow squid
/// differs only by texture (its emissive light boost is deferred lighting).
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
    let mut model = SquidModel::new();
    model.prepare(&instance);
    model.root().render_textured(
        meshes.mesh_mut(EntityModelLayerRenderType::Cutout),
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

/// The textured vex base layer. The unified [`VexModel`] tree runs the shared `VexModel.setupAnim`
/// (head look, charging/idle body + arms, wing flap) and draws into the translucent mesh. The
/// charging texture swap and the held-item arms are deferred entity-side state, and the vanilla
/// full-bright block light (`getBlockLightLevel` → 15) is deferred lighting.
fn emit_vex_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(entry) = entity_model_texture_atlas_entry(atlas, VEX_TEXTURE_REF) else {
        return;
    };
    let transform = entity_model_root_transform(instance);
    let mut model = VexModel::new();
    model.prepare(&instance);
    model.root().render_textured(
        meshes.mesh_mut(EntityModelLayerRenderType::Translucent),
        transform,
        VEX_TEXTURE_REF,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
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
    // The unified `AllayModel` tree drives both render paths; `setup_anim` runs the shared
    // `AllayModel.setupAnim` idle/flying pose. Allay draws into a single translucent layer.
    let Some(entry) = entity_model_texture_atlas_entry(atlas, ALLAY_TEXTURE_REF) else {
        return;
    };
    let transform = entity_model_root_transform(instance);
    let mut model = AllayModel::new();
    model.prepare(&instance);
    model.root().render_textured(
        meshes.mesh_mut(EntityModelLayerRenderType::Translucent),
        transform,
        ALLAY_TEXTURE_REF,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
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

/// The textured llama base layer. The trader llama shares this geometry/texture; its distinguishing
/// `LlamaDecorLayer` overlay is a deferred equipment layer, so `family` is not consumed here. The
/// unified `LlamaModel` tree drives both render paths; `setup_anim` is the standard `QuadrupedModel`
/// head look plus the diagonal leg swing. `new` selects the baby / adult / chested tree; the variant
/// chooses the texture.
fn emit_llama_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    variant: LlamaVariant,
    baby: bool,
    has_chest: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    let mut model = LlamaModel::new(baby, has_chest);
    model.prepare(&instance);
    for pass in llama_textured_layer_passes(variant, baby, has_chest) {
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

/// `HumanoidModel` leg part indices in the skeleton-family body and clothing
/// layers: the head, body, and two arms occupy the lower slots (in either order),
/// then the right and left legs. [`humanoid_leg_swing_pose`] resolves each leg's
/// phase from its offset, so the parched layer's head/body swap does not matter.
const HUMANOID_LEG_PART_INDICES: [usize; 2] = [4, 5];

/// `HumanoidModel` arm part indices (head/body at `0`/`1`, arms at `[2, 3]`).
const HUMANOID_ARM_PART_INDICES: [usize; 2] = [2, 3];

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

fn emit_creeper_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `CreeperModel` tree drives both render paths; `setup_anim` follows the head look and
    // applies the standard `QuadrupedModel` leg swing once. The swell is folded into the root
    // transform; the powered charge layer is deferred.
    let transform = creeper_model_root_transform(instance);
    let mut model = CreeperModel::new();
    model.prepare(&instance);
    for pass in creeper_textured_layer_passes() {
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

fn emit_spider_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    cave: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `SpiderModel` tree drives both render paths; `setup_anim` looks the head and
    // sweeps/steps the eight legs once. Both the base and eyes passes read this one posed tree. The
    // cave spider shares the model and differs only by its smaller root transform.
    let transform = if cave {
        cave_spider_model_root_transform(instance)
    } else {
        entity_model_root_transform(instance)
    };
    let mut model = SpiderModel::new();
    model.prepare(&instance);
    for pass in spider_textured_layer_passes(cave) {
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

fn emit_enderman_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `EndermanModel` tree drives both render paths; `setup_anim` looks the head, swings
    // the clamped arms/legs, overrides the arms when carrying a block, and applies the creepy
    // head/hat shift. Both the base and eyes passes read this one posed tree.
    let transform = entity_model_root_transform(instance);
    let mut model = EndermanModel::new();
    model.prepare(&instance);
    for pass in enderman_textured_layer_passes() {
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

fn emit_iron_golem_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `IronGolemModel` tree drives both render paths; `setup_anim` follows the head look
    // then swings the arms and legs once. The attack swing and offer-flower arm pose are deferred.
    let transform = entity_model_root_transform(instance);
    let mut model = IronGolemModel::new();
    model.prepare(&instance);
    for pass in iron_golem_textured_layer_passes() {
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

fn emit_snow_golem_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `SnowGolemModel` tree drives both render paths; `setup_anim` looks the head, twists
    // the upper body by a quarter of the head yaw, and orbits the two stick arms once.
    let transform = entity_model_root_transform(instance);
    let mut model = SnowGolemModel::new();
    model.prepare(&instance);
    for pass in snow_golem_textured_layer_passes() {
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

fn emit_witch_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `WitchModel` tree drives both render paths; `setup_anim` looks the head, swings the
    // legs at the villager-family half amplitude, and bobs the nose (the head's nose child, so it
    // inherits the head look). The `isHoldingItem` nose hold pose and combined `arms` part defer.
    let transform = villager_adult_model_root_transform(instance);
    let mut model = WitchModel::new();
    model.prepare(&instance);
    for pass in witch_textured_layer_passes() {
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

fn emit_slime_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    size: i32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `SlimeModel` (inner body, cutout) and `SlimeOuterModel` (shell, translucent) trees
    // drive both render paths; both `setup_anim`s are no-ops. Each pass routes to the inner or outer
    // root in the pre-sorted layer order.
    let transform = slime_model_root_transform(instance, size);
    let mut inner = SlimeModel::new();
    inner.prepare(&instance);
    let mut outer = SlimeOuterModel::new();
    outer.prepare(&instance);
    for pass in slime_textured_layer_passes() {
        let root = if pass.kind == layers::EntityModelLayerKind::SlimeOuter {
            outer.root()
        } else {
            inner.root()
        };
        if let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) {
            root.render_textured(
                meshes.mesh_mut(pass.render_type),
                transform,
                pass.texture,
                entry.uv,
                pass.tint,
            );
        }
    }
}

fn emit_magma_cube_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    size: i32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `MagmaCubeModel` tree drives both render paths; its `setup_anim` is a no-op (the
    // squish stretch is deferred), so this renders the rest pose under the per-size root transform.
    let transform = magma_cube_model_root_transform(instance, size);
    let mut model = MagmaCubeModel::new();
    model.prepare(&instance);
    for pass in magma_cube_textured_layer_passes() {
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

fn emit_ghast_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `GhastModel` tree drives both render paths; `setup_anim` waves the nine tentacles
    // from `ageInTicks` once. The layer pass supplies the texture / render type / tint.
    let transform = ghast_model_root_transform(instance);
    let mut model = GhastModel::new();
    model.prepare(&instance);
    for pass in ghast_textured_layer_passes() {
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

fn emit_happy_ghast_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `HappyGhastModel` tree drives both render paths; `setup_anim` reuses the ghast
    // tentacle wave from `ageInTicks` once. The layer pass supplies the texture / render type / tint.
    let transform = happy_ghast_model_root_transform(instance);
    let mut model = HappyGhastModel::new();
    model.prepare(&instance);
    for pass in happy_ghast_textured_layer_passes() {
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

fn emit_minecart_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `MinecartModel` tree drives both render paths; its `setup_anim` is a no-op (static
    // box), so this renders the rest pose under the entity root transform.
    let transform = entity_model_root_transform(instance);
    let mut model = MinecartModel::new();
    model.prepare(&instance);
    for pass in minecart_textured_layer_passes() {
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

fn emit_zombie_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `ZombieModel` tree drives both render paths; `setup_anim` looks the head, runs the
    // humanoid leg swing, then overrides the arms with the held-out `animateZombieArms` pose.
    let transform = entity_model_root_transform(instance);
    let mut model = ZombieModel::new(baby);
    model.prepare(&instance);
    for pass in zombie_textured_layer_passes(baby) {
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

fn emit_husk_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `ZombieVariantModel` tree drives both render paths; `setup_anim` runs the shared
    // `ZombieModel.setupAnim` (head look + leg swing + held-out arms). `HuskRenderer extends
    // ZombieRenderer`, so the husk reuses the zombie body; vanilla scales the adult husk mesh by
    // 1.0625 (`huskScale`), while the baby husk reuses the unscaled `babyZombieLayer`.
    let transform = if baby {
        entity_model_root_transform(instance)
    } else {
        mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE)
    };
    let mut model = ZombieVariantModel::new(ZombieVariantModelFamily::Husk, baby);
    model.prepare(&instance);
    for pass in husk_textured_layer_passes(baby) {
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

fn emit_drowned_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `ZombieVariantModel` tree drives both render paths; `setup_anim` runs the shared
    // `ZombieModel.setupAnim` (head look + leg swing + held-out arms). `DrownedModel extends
    // ZombieModel`, so the non-swimming drowned reuses the zombie body. The `DrownedOuterLayer`, the
    // swim re-pose (needs `swimAmount`), and the trident throw arm pose all stay deferred. No root scale.
    let transform = entity_model_root_transform(instance);
    let mut model = ZombieVariantModel::new(ZombieVariantModelFamily::Drowned, baby);
    model.prepare(&instance);
    for pass in drowned_textured_layer_passes(baby) {
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

fn emit_zombie_villager_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `ZombieVariantModel` tree drives both render paths; `setup_anim` runs the shared
    // `ZombieModel.setupAnim` (head look + leg swing + held-out arms). `ZombieVillagerModel extends
    // HumanoidModel` over its own robed body layer. The hatted base layer is emitted; the no-hat
    // model selection and the profession/type/level overlays stay deferred. No root scale.
    let transform = entity_model_root_transform(instance);
    let mut model = ZombieVariantModel::new(ZombieVariantModelFamily::ZombieVillager, baby);
    model.prepare(&instance);
    for pass in zombie_villager_textured_layer_passes(baby) {
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

fn emit_piglin_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: PiglinModelFamily,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `PiglinModel` tree drives both render paths; `setup_anim` runs the head look, the
    // humanoid walk (legs only for the zombified piglin), and the ear flap (head children). `new`
    // selects the adult/baby tree; the family chooses the texture. The brute is never baby. The
    // dance/attack/crossbow/admire arm poses and held items defer.
    let baby_layout = baby && family != PiglinModelFamily::PiglinBrute;
    let transform = entity_model_root_transform(instance);
    let mut model = PiglinModel::new(family, baby);
    model.prepare(&instance);
    for pass in piglin_textured_layer_passes(family, baby_layout) {
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

fn emit_illager_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: IllagerModelFamily,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `IllagerModel` tree drives both render paths; `new` selects the crossed/uncrossed
    // tree by family and spell-cast state, and `setup_anim` looks the head, swings the legs at the
    // villager-family half amplitude, then swings the pillager's separate arms or raises a
    // spellcasting evoker/illusioner's arms into the `SPELLCASTING` pose. The other arm-pose
    // overrides (attack/bow/crossbow/celebrate), the riding sit pose, and the item-in-hand layers
    // stay deferred.
    let transform = villager_adult_model_root_transform(instance);
    let mut model = IllagerModel::new(&instance, family);
    model.prepare(&instance);
    for pass in illager_textured_layer_passes(family) {
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

fn emit_blaze_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `BlazeModel` tree drives both render paths; `setup_anim` follows the head look and
    // SETs all twelve rod offsets from `ageInTicks` once. The layer pass supplies the texture /
    // render type / tint.
    let transform = entity_model_root_transform(instance);
    let mut model = BlazeModel::new();
    model.prepare(&instance);
    for pass in blaze_textured_layer_passes() {
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

fn emit_endermite_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `EndermiteModel` tree drives both render paths; `setup_anim` wiggles the four
    // chitin segments once. The layer pass supplies the texture / render type / tint.
    let transform = entity_model_root_transform(instance);
    let mut model = EndermiteModel::new();
    model.prepare(&instance);
    for pass in endermite_textured_layer_passes() {
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

fn emit_silverfish_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `SilverfishModel` tree drives both render paths; `setup_anim` wiggles the seven
    // body segments and copies the three overlay layers once. The layer pass supplies the texture /
    // render type / tint.
    let transform = entity_model_root_transform(instance);
    let mut model = SilverfishModel::new();
    model.prepare(&instance);
    for pass in silverfish_textured_layer_passes() {
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

fn emit_phantom_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    size: i32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `PhantomModel` tree drives both render paths; `setup_anim` flaps the nested
    // wing/tail chains from `flapTime` (`id*3 + ageInTicks`). The cutout base layer and the emissive
    // eyes overlay both re-render the same posed tree. The size scale and body pitch live in the root
    // transform.
    let transform = phantom_model_root_transform(instance, size);
    let mut model = PhantomModel::new();
    model.prepare(&instance);
    for pass in phantom_textured_layer_passes() {
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
    // The unified `PolarBearModel` tree drives both render paths; `setup_anim` runs the head look and
    // four-leg swing, then adds the standing rear-up deltas on top when `standScale != 0`.
    let transform = if baby {
        entity_model_root_transform(instance)
    } else {
        polar_bear_model_root_transform(instance)
    };
    let mut model = PolarBearModel::new(baby);
    model.prepare(&instance);
    for pass in polar_bear_textured_layer_passes(baby) {
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

fn emit_hoglin_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: HoglinModelFamily,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `HoglinModel` tree drives both render paths; `setup_anim` runs the yaw-only head
    // look, ear sway (head children), and four-leg swing. `new` selects the adult/baby tree; the
    // family only chooses the texture (hoglin vs zoglin). The headbutt head tilt defers.
    let transform = entity_model_root_transform(instance);
    let mut model = HoglinModel::new(baby);
    model.prepare(&instance);
    for pass in hoglin_textured_layer_passes(family, baby) {
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

fn emit_ravager_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `RavagerModel` tree drives both render paths; `setup_anim` swings the four legs and
    // looks the head (nested under the neck, so its horn/mouth descendants inherit the look). The
    // neck/mouth attack/stun/roar poses are deferred.
    let transform = entity_model_root_transform(instance);
    let mut model = RavagerModel::new();
    model.prepare(&instance);
    for pass in ravager_textured_layer_passes() {
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

fn emit_villager_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `VillagerModel` tree drives both render paths; `setup_anim` looks the head and
    // swings the legs at the villager-family half amplitude once.
    let transform = if baby {
        entity_model_root_transform(instance)
    } else {
        villager_adult_model_root_transform(instance)
    };
    let mut model = VillagerModel::new(baby);
    model.prepare(&instance);
    for pass in villager_textured_layer_passes(baby) {
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

fn emit_wandering_trader_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `WanderingTraderModel` tree drives both render paths; `setup_anim` looks the head and
    // swings the legs at the villager-family half amplitude once.
    let transform = villager_adult_model_root_transform(instance);
    let mut model = WanderingTraderModel::new();
    model.prepare(&instance);
    for pass in wandering_trader_textured_layer_passes() {
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

fn emit_player_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    slim: bool,
    parts: PlayerModelPartVisibility,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `PlayerModel` tree drives both render paths; `setup_anim` looks the head, runs the
    // inherited `HumanoidModel` walk swing + idle arm bob, and applies the crouch sneaking pose. The
    // six skin overlay parts (hat/jacket/sleeves/pants) are toggled by the player's part visibility
    // after `prepare` (the colored fallback shows every overlay). Held-item/attack/swim arm poses,
    // the cape, and the elytra defer.
    let transform = player_model_root_transform(instance);
    let mut model = PlayerModel::new(slim);
    model.prepare(&instance);
    model.apply_part_visibility(parts);
    for pass in player_textured_layer_passes(slim, parts) {
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
    // The unified `SheepModel` (body) and `SheepFurModel` (wool) trees drive both render paths; both
    // run the shared `SheepModel.setupAnim` (leg swing + eat-grass head pose). Each pass routes to the
    // body tree (base + dyed undercoat) or the fur tree (wool), in the pre-sorted layer order; the
    // wool tint and per-state visibility are baked into the passes.
    let transform = entity_model_root_transform(instance);
    let mut body = SheepModel::new(baby);
    body.prepare(&instance);
    let mut fur = SheepFurModel::new(baby);
    fur.prepare(&instance);
    for pass in sheep_textured_layer_passes(baby, sheared, wool_color, invisible, jeb, age_ticks) {
        let root = if pass.kind == layers::EntityModelLayerKind::SheepWool {
            fur.root()
        } else {
            body.root()
        };
        if let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) {
            root.render_textured(
                meshes.mesh_mut(pass.render_type),
                transform,
                pass.texture,
                entry.uv,
                pass.tint,
            );
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
    // The unified `GoatModel` tree drives both render paths; `setup_anim` looks the head, swings the
    // four legs, and toggles each horn child's visibility from the `left_horn`/`right_horn` flags.
    let transform = entity_model_root_transform(instance);
    let mut model = GoatModel::new(baby, left_horn, right_horn);
    model.prepare(&instance);
    for pass in goat_textured_layer_passes(baby) {
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
