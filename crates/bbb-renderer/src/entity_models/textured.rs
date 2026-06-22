use super::{
    boat_model_root_transform,
    catalog::squid_texture_ref,
    catalog::{
        ArmorStandModelPose, BoatModelFamily, CamelModelFamily, ChickenModelVariant,
        CowModelVariant, EntityDyeColor, EntityModelKind, EntityModelTextureAtlasEntry,
        EntityModelTextureAtlasLayout, EntityModelTextureRef, HoglinModelFamily,
        IllagerModelFamily, LlamaVariant, PigModelVariant, PiglinModelFamily,
        PlayerModelPartVisibility, SalmonModelSize, SheepWoolColor, SkeletonModelFamily,
        TropicalFishModelShape, ZombieVariantModelFamily,
    },
    cave_spider_model_root_transform, cod_model_root_transform, entity_model_root_transform,
    geometry::{
        emit_textured_model_cube, emit_textured_model_parts, fill_entity_textured_light,
        fill_entity_textured_overlay, part_pose_transform, EntityModelTexturedMesh, ModelPartDesc,
        PartPose, TexturedModelPartDesc,
    },
    ghast_model_root_transform, happy_ghast_model_root_transform,
    instances::EntityModelInstance,
    magma_cube_model_root_transform, mesh_transformer_scaled_model_root_transform,
    model_layers::{
        apply_polar_bear_standing_pose, apply_wolf_sitting_pose, armor_stand_textured_cube,
        blaze_rod_offset, chicken_leg_part_indices, cod_tail_fin_yrot, cow_head_part_index,
        enderman_arm_swing_pose, enderman_leg_swing_pose, endermite_segment_pose,
        ghast_tentacle_x_rot, half_amplitude_leg_swing_pose, head_first_part_index,
        head_look_at_rest, head_look_pose, head_look_yaw_pose, head_yaw_at_rest,
        hoglin_ear_sway_pose, hoglin_head_part_index, hoglin_leg_swing_pose,
        humanoid_arm_swing_pose, humanoid_leg_swing_pose, iron_golem_walk_part_roles,
        iron_golem_walk_pose, limb_swing_at_rest, parched_head_part_index, phantom_flap_time,
        phantom_tail_pose, phantom_tail_x_rot, phantom_wing_pose, phantom_wing_z_rot,
        pig_head_part_index, piglin_ear_flap_pose, piglin_head_part_index, player_head_part_index,
        polar_bear_head_part_index, polar_bear_standing_part_roles, pufferfish_fin_pose,
        pufferfish_parts, pufferfish_right_fin_z_rot, quadruped_leg_swing_pose,
        ravager_head_child_index, ravager_leg_swing_pose, ravager_neck_part_index,
        salmon_body_back_yrot, sheep_head_at_rest, sheep_head_part_index, sheep_head_pose,
        silverfish_layer_pose, silverfish_segment_pose, skeleton_head_part_index,
        snow_golem_arm_pose, snow_golem_upper_body_pose, snow_golem_upper_body_yrot,
        spider_leg_swing_pose, spider_leg_swing_roles, squid_textured_model_parts,
        tropical_fish_tail_yrot, villager_head_part_index, witch_nose_bob_pose,
        wolf_angry_tail_pose, wolf_sitting_part_roles, wolf_tail_part_index, wolf_tail_swing_pose,
        ADULT_GOAT_HEAD_INDEX, ARMOR_STAND_PARTS, ARMOR_STAND_PART_UVS, ARMOR_STAND_TEXTURE_REF,
        BABY_GOAT_HEAD_INDEX, BLAZE_ROD_COUNT, COD_TAIL_FIN_PART_INDEX,
        HOGLIN_LEFT_EAR_CHILD_INDEX, HOGLIN_RIGHT_EAR_CHILD_INDEX, PHANTOM_BODY_POSE,
        PHANTOM_BODY_TEXTURED_CUBE, PHANTOM_HEAD_POSE, PHANTOM_HEAD_TEXTURED_CUBE,
        PHANTOM_LEFT_WING_BASE_POSE, PHANTOM_LEFT_WING_BASE_TEXTURED_CUBE,
        PHANTOM_LEFT_WING_TIP_POSE, PHANTOM_LEFT_WING_TIP_TEXTURED_CUBE,
        PHANTOM_RIGHT_WING_BASE_POSE, PHANTOM_RIGHT_WING_BASE_TEXTURED_CUBE,
        PHANTOM_RIGHT_WING_TIP_POSE, PHANTOM_RIGHT_WING_TIP_TEXTURED_CUBE, PHANTOM_TAIL_BASE_POSE,
        PHANTOM_TAIL_BASE_TEXTURED_CUBE, PHANTOM_TAIL_TIP_POSE, PHANTOM_TAIL_TIP_TEXTURED_CUBE,
        PIGLIN_ADULT_EAR_ANGLE, PIGLIN_BABY_EAR_ANGLE, PUFFERFISH_TEXTURE_REF,
        RAVAGER_TEXTURED_NECK_CHILDREN, SALMON_BODY_BACK_PART_INDEX, SILVERFISH_LAYER_RULES,
        SILVERFISH_SEGMENT_COUNT, SMALL_ARMOR_STAND_PARTS, SNOW_GOLEM_HEAD_PART_INDEX,
        SNOW_GOLEM_LEFT_ARM_PART_INDEX, SNOW_GOLEM_RIGHT_ARM_PART_INDEX,
        SNOW_GOLEM_UPPER_BODY_PART_INDEX, TROPICAL_FISH_TAIL_PART_INDEX, WITCH_NOSE_CHILD_INDEX,
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
    chicken_textured_layer_passes, cod_textured_layer_passes, cow_textured_layer_passes,
    creeper_textured_layer_passes, drowned_textured_layer_passes, enderman_textured_layer_passes,
    endermite_textured_layer_passes, ghast_textured_layer_passes, goat_textured_layer_passes,
    happy_ghast_textured_layer_passes, hoglin_textured_layer_passes, husk_textured_layer_passes,
    illager_textured_layer_passes, iron_golem_textured_layer_passes, llama_textured_layer_passes,
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
            EntityModelKind::TropicalFish { shape } => {
                emit_tropical_fish_textured_model(&mut meshes, *instance, shape, atlas);
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
/// clamp, none of which the colored path applies; the textured path matches that static
/// pose. The camel husk shares the adult mesh, differing only in texture. The keyframe
/// animations and head rotation remain deferred.
fn emit_camel_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: CamelModelFamily,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in camel_textured_layer_passes(family, baby) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
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
    let in_water = instance.render_state.in_water;
    let transform = cod_model_root_transform(instance, in_water);
    let tail_yrot = cod_tail_fin_yrot(instance.render_state.age_in_ticks, in_water);
    for pass in cod_textured_layer_passes() {
        if tail_yrot == 0.0 {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            parts[COD_TAIL_FIN_PART_INDEX].pose.rotation[1] = tail_yrot;
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
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
    let in_water = instance.render_state.in_water;
    let transform = salmon_model_root_transform(instance, in_water, size);
    let body_back_yrot = salmon_body_back_yrot(instance.render_state.age_in_ticks, in_water);
    for pass in salmon_textured_layer_passes(size) {
        if body_back_yrot == 0.0 {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
        } else {
            let mut parts = pass.parts.to_vec();
            parts[SALMON_BODY_BACK_PART_INDEX].pose.rotation[1] = body_back_yrot;
            emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
        }
    }
}

/// The textured tropical fish base layer. The parts are static apart from the tail, which
/// is swayed by the vanilla `TropicalFish{Small,Large}Model.setupAnim`; the swim wiggle,
/// out-of-water flop, and small/large body shape live in
/// [`tropical_fish_model_root_transform`] and the per-shape pass. The twelve pattern
/// overlays and the base/pattern color tints are deferred.
fn emit_tropical_fish_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    shape: TropicalFishModelShape,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let in_water = instance.render_state.in_water;
    let transform = tropical_fish_model_root_transform(instance, in_water);
    let tail_yrot = tropical_fish_tail_yrot(instance.render_state.age_in_ticks, in_water);
    for pass in tropical_fish_textured_layer_passes(shape) {
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
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in passes {
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
    // (full head look + leg swing). The swelling scale and powered layer are deferred.
    emit_quadruped_textured_passes(
        meshes,
        creeper_textured_layer_passes(),
        head_first_part_index(),
        QUADRUPED_LEG_PART_INDICES,
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
    // [4, 5]). The carried-block and creepy poses defer.
    let head_index = head_first_part_index();
    let transform = entity_model_root_transform(instance);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    for pass in enderman_textured_layer_passes() {
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

fn emit_zombie_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Mirrors the colored `emit_zombie_model`: vanilla `HumanoidModel.setupAnim` runs the head
    // look and the leg swing, but `ZombieModel` overrides the arms with the held-out
    // `animateZombieArms` pose, so bbb leaves the visible arms static (that arms-out pose is
    // deferred). Only the head and legs animate. The baby layer's head is part 1 (the body is
    // part 0); the adult head is part 0.
    let head_index = if baby { 1 } else { 0 };
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    let transform = entity_model_root_transform(instance);
    for pass in zombie_textured_layer_passes(baby) {
        if head_resting && limbs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
            continue;
        }
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
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_husk_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Mirrors the colored `emit_zombie_variant_model` husk arm: `HuskRenderer extends
    // ZombieRenderer`, so the husk reuses the zombie body parts and the same `HumanoidModel`
    // head-look + leg-swing animation (the held-out `animateZombieArms` arm pose stays deferred,
    // so the visible arms stay static). Vanilla scales the adult husk mesh by 1.0625
    // (`huskScale`); the baby husk reuses the unscaled `babyZombieLayer`.
    let head_index = if baby { 1 } else { 0 };
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    let transform = if baby {
        entity_model_root_transform(instance)
    } else {
        mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE)
    };
    for pass in husk_textured_layer_passes(baby) {
        if head_resting && limbs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
            continue;
        }
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
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_drowned_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Mirrors the colored `emit_zombie_variant_model` drowned arm: `DrownedModel extends
    // ZombieModel`, so the non-swimming drowned runs the same `HumanoidModel` head-look +
    // leg-swing with the held-out `animateZombieArms` arms (deferred). The `DrownedOuterLayer`,
    // the `setupRotations`/`setupAnim` swim re-pose (needs `swimAmount`), and the trident throw
    // arm pose (needs a held item) all stay deferred. Drowned has no root scale.
    let head_index = if baby { 1 } else { 0 };
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    let transform = entity_model_root_transform(instance);
    for pass in drowned_textured_layer_passes(baby) {
        if head_resting && limbs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
            continue;
        }
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
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
}

fn emit_zombie_villager_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // Mirrors the colored `emit_zombie_variant_model` zombie-villager arm: `ZombieVillagerModel
    // extends HumanoidModel` and runs `super.setupAnim` (head look + leg swing) then
    // `AnimationUtils.animateZombieArms` (the held-out arms, deferred). Only the head and legs
    // animate. The baby layout's head is part 1 (the body is part 0); the adult head is part 0.
    // The hatted base layer is emitted; the no-hat model selection and the
    // profession/type/level overlays stay deferred. Zombie villagers have no root scale.
    let head_index = if baby { 1 } else { 0 };
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    let transform = entity_model_root_transform(instance);
    for pass in zombie_villager_textured_layer_passes(baby) {
        if head_resting && limbs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
            continue;
        }
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
        emit_textured_layer_pass_with_parts(meshes, &pass, &parts, transform, atlas);
    }
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
            for index in HUMANOID_ARM_PART_INDICES {
                if let Some(arm) = parts.get_mut(index) {
                    arm.pose = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
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
    // arms; the evoker/vindicator/illusioner show the static folded `arms` part (vanilla swings
    // the *invisible* separate arms). The arm-pose overrides (attack/spellcast/bow/crossbow/
    // celebrate), the riding sit pose, and the item-in-hand layers stay deferred.
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_resting = head_look_at_rest(head_yaw, head_pitch);
    let limbs_resting = limb_swing_at_rest(limb_swing_amount);
    let transform = villager_adult_model_root_transform(instance);
    let (leg_indices, arm_indices): ([usize; 2], Option<[usize; 2]>) = match family {
        IllagerModelFamily::Pillager => ([2, 3], Some([4, 5])),
        IllagerModelFamily::Evoker
        | IllagerModelFamily::Vindicator
        | IllagerModelFamily::Illusioner => ([3, 4], None),
    };
    for pass in illager_textured_layer_passes(family) {
        if head_resting && limbs_resting {
            emit_textured_layer_pass(meshes, &pass, transform, atlas);
            continue;
        }
        let mut parts = pass.parts.to_vec();
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
        // the arm parts. Held-item/attack/crouch/swim arm poses and the idle bob defer.
        for index in HUMANOID_ARM_PART_INDICES {
            if let Some(arm) = visible_parts.get_mut(index) {
                arm.pose = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
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
