use super::catalog::EntityDynamicPlayerTextureAtlasEntry;
use super::colored::{
    creeper_model_root_transform, drowned_model_root_transform, end_crystal_model_root_transform,
    shulker_bullet_model_root_transform, villager_adult_model_root_transform,
    wind_charge_model_root_transform, wither_model_root_transform, GIANT_SCALE, HORSE_SCALE,
};
use super::dispatch::{dispatch_uniform_entity_model, TexturedSink};
use super::held_item::custom_head_skull_transform;
use super::model::{EntityModel, ModelPart};
#[cfg(test)]
use super::model_layers::PLAYER_WIDE_STEVE_TEXTURE_REF;
use super::{
    catalog::{
        horse_markings_texture_ref, squid_texture_ref, villager_level_texture_ref,
        villager_profession_texture_ref, villager_type_texture_ref,
        zombie_villager_level_texture_ref, zombie_villager_profession_texture_ref,
        zombie_villager_type_texture_ref,
    },
    catalog::{
        CamelModelFamily, DonkeyModelFamily, EntityArmorMaterial, EntityCustomHeadSkull,
        EntityDyeColor, EntityDynamicPlayerSkin, EntityDynamicPlayerSkinAtlasEntry,
        EntityDynamicPlayerSkinAtlasLayout, EntityDynamicPlayerSkinStatus,
        EntityDynamicPlayerTexture, EntityDynamicPlayerTextureAtlasLayout,
        EntityEquipmentLayerTexture, EntityModelKind, EntityModelTextureAtlasEntry,
        EntityModelTextureAtlasLayout, EntityModelTextureRef, EntityModelUvRect, EntityPlayerSkin,
        HoglinModelFamily, HorseMarkings, LlamaModelFamily, LlamaVariant, PiglinModelFamily,
        PlayerModelPartVisibility, SheepWoolColor, SkeletonModelFamily, TropicalFishModelShape,
        TropicalFishPattern, UndeadHorseModelFamily, VillagerModelData, VillagerModelHat,
        ZombieVariantModelFamily,
    },
    entity_model_root_transform,
    geometry::{
        append_scrolled_textured_mesh, emit_textured_model_cube, emit_textured_model_parts,
        fill_entity_textured_light, fill_entity_textured_overlay, part_pose_transform,
        EntityModelScrollMesh, EntityModelScrollVertex, EntityModelTexturedMesh, PartPose,
        TexturedModelCubeDesc, TexturedModelPartDesc,
    },
    instances::EntityModelInstance,
    mesh_transformer_scaled_model_root_transform,
    model_layers::{
        armor_layer_tint, armor_slot_texture_for_layer, default_player_skin_texture_ref,
        end_crystal_bob_y, end_crystal_get_y, end_crystal_glass_quaternions, equine_head_look_pose,
        equine_leg_swing_pose, equine_tail_swing_pose, head_look_at_rest,
        horse_body_armor_texture_layers, limb_swing_at_rest, llama_body_decor_texture_ref,
        nautilus_body_armor_texture_ref, wolf_armor_crackiness_texture_ref,
        wolf_body_armor_texture_layers, BreezeWindModel, CamelModel, CreeperModel,
        CustomHeadDragonSkullModel, CustomHeadPiglinSkullModel, CustomHeadSkullModel,
        DrownedOuterModel, ElytraModel, HoglinModel, HumanoidArmorSlot, HumanoidBabyArmorKind,
        LlamaModel, NautilusModel, PigModel, PiglinModel, PlayerModel, SheepFurModel, SheepModel,
        ShulkerBulletModel, SkeletonClothingModel, SkeletonModel, SlimeModel, SlimeOuterModel,
        SquidModel, StriderModel, TropicalFishModel, TropicalFishPatternModel, VillagerModel,
        WindChargeModel, WitherModel, WolfModel, ZombieModel, ZombieVariantModel,
        ADULT_DONKEY_PARTS_TEXTURED, ADULT_DONKEY_PARTS_WITH_CHEST_TEXTURED,
        ADULT_DONKEY_SADDLE_PARTS_TEXTURED, ADULT_DONKEY_SADDLE_RIDDEN_PARTS_TEXTURED,
        ADULT_HORSE_ARMOR_PARTS_TEXTURED, ADULT_HORSE_PARTS_TEXTURED,
        ADULT_HORSE_SADDLE_PARTS_TEXTURED, ADULT_HORSE_SADDLE_RIDDEN_PARTS_TEXTURED,
        BABY_DONKEY_PARTS_TEXTURED, BABY_HORSE_PARTS_TEXTURED, BREEZE_WIND_TEXTURE_REF,
        CAMEL_HUSK_SADDLE_TEXTURE_REF, CAMEL_SADDLE_TEXTURE_REF, CREEPER_ARMOR_TEXTURE_REF,
        CREEPER_TEXTURE_REF, DONKEY_SADDLE_TEXTURE_REF, ENDER_DRAGON_TEXTURE_REF,
        END_CRYSTAL_BEAM_TEXTURE_REF, END_CRYSTAL_TEXTURED_PARTS, END_CRYSTAL_TEXTURE_REF,
        GUARDIAN_BEAM_TEXTURE_REF, HORSE_SADDLE_TEXTURE_REF, LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
        LLAMA_BODY_TRADER_TEXTURE_REF, MULE_SADDLE_TEXTURE_REF, NAUTILUS_SADDLE_TEXTURE_REF,
        PIGLIN_OUTER_ARMOR_DEFORMATION, PIGLIN_TEXTURE_REF, PIG_SADDLE_TEXTURE_REF,
        PLAYER_PROFILE_CAPE_TEXTURE_REF, PLAYER_PROFILE_ELYTRA_TEXTURE_REF,
        SHULKER_BULLET_TEXTURE_REF, SKELETON_HORSE_SADDLE_TEXTURE_REF, SKELETON_TEXTURE_REF,
        STANDARD_OUTER_ARMOR_DEFORMATION, STRIDER_SADDLE_TEXTURE_REF, WIND_CHARGE_TEXTURE_REF,
        WITHER_ARMOR_TEXTURE_REF, WITHER_SKELETON_TEXTURE_REF, ZOMBIE_HORSE_SADDLE_TEXTURE_REF,
        ZOMBIE_TEXTURE_REF,
    },
    player_model_root_transform, slime_model_root_transform, squid_model_root_transform,
    tropical_fish_model_root_transform, wither_skeleton_model_root_transform, HUSK_SCALE,
};
use glam::{Mat4, Quat, Vec3};

const PLAYER_CAPE_CUBE: TexturedModelCubeDesc = TexturedModelCubeDesc {
    min: [-5.0, 0.0, -1.0],
    size: [10.0, 16.0, 1.0],
    uv_size: [10.0, 16.0, 1.0],
    tex: [0.0, 0.0],
    mirror: false,
};

mod layers;
#[cfg(test)]
pub(super) use layers::player_textured_layer_passes;
#[cfg(test)]
pub(super) use layers::shulker_bullet_textured_layer_passes;
pub(super) use layers::{
    armadillo_textured_layer_passes, arrow_textured_layer_passes, axolotl_textured_layer_passes,
    blaze_textured_layer_passes, boat_textured_layer_passes, camel_textured_layer_passes,
    chicken_textured_layer_passes, copper_golem_textured_layer_passes, cow_textured_layer_passes,
    creaking_textured_layer_passes, creeper_textured_layer_passes, drowned_textured_layer_passes,
    ender_dragon_textured_layer_passes, enderman_textured_layer_passes,
    endermite_textured_layer_passes, evoker_fangs_textured_layer_passes,
    feline_textured_layer_passes, fox_textured_layer_passes, frog_textured_layer_passes,
    ghast_textured_layer_passes, goat_textured_layer_passes, guardian_textured_layer_passes,
    happy_ghast_textured_layer_passes, hoglin_textured_layer_passes, husk_textured_layer_passes,
    illager_textured_layer_passes, iron_golem_textured_layer_passes,
    leash_knot_textured_layer_passes, llama_spit_textured_layer_passes,
    llama_textured_layer_passes, magma_cube_textured_layer_passes, minecart_textured_layer_passes,
    mooshroom_textured_layer_passes, nautilus_textured_layer_passes, panda_textured_layer_passes,
    parrot_textured_layer_passes, phantom_textured_layer_passes, pig_textured_layer_passes,
    piglin_textured_layer_passes, player_textured_layer_passes_with_texture,
    polar_bear_textured_layer_passes, rabbit_textured_layer_passes, ravager_textured_layer_passes,
    salmon_textured_layer_passes, sheep_textured_layer_passes, shulker_textured_layer_passes,
    silverfish_textured_layer_passes, skeleton_textured_layer_passes, slime_textured_layer_passes,
    sniffer_textured_layer_passes, snow_golem_textured_layer_passes, spider_textured_layer_passes,
    tadpole_textured_layer_passes, trident_textured_layer_passes,
    tropical_fish_textured_layer_passes, villager_textured_layer_passes,
    wandering_trader_textured_layer_passes, warden_textured_layer_passes,
    witch_textured_layer_passes, wither_skull_textured_layer_passes, wither_textured_layer_passes,
    wolf_textured_layer_passes, zombie_nautilus_textured_layer_passes,
    zombie_textured_layer_passes, zombie_villager_textured_layer_passes, EntityModelLayerKind,
    EntityModelLayerPass, EntityModelLayerRenderBucket, EntityModelLayerRenderType,
};
#[cfg(test)]
pub(super) use layers::{warden_pulsating_spots_alpha, EntityModelLayerVisibility};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EntityModelRenderSubmission {
    pub(super) render_type: EntityModelLayerRenderType,
    pub(super) texture: EntityModelTextureRef,
    pub(super) dynamic_player_skin: Option<EntityDynamicPlayerSkin>,
    pub(super) dynamic_player_texture: Option<EntityDynamicPlayerTexture>,
    pub(super) tint: [f32; 4],
    pub(super) transform: Mat4,
    pub(super) order: i32,
    pub(super) submit_sequence: u32,
}

pub(super) struct EntityModelTexturedMeshes {
    pub(super) cutout: EntityModelTexturedMesh,
    pub(super) translucent: EntityModelTexturedMesh,
    pub(super) eyes: EntityModelTexturedMesh,
    /// Ready remote player skins are rendered through a dedicated atlas, preserving their vanilla
    /// cutout/translucent render type while swapping only the texture source.
    pub(super) dynamic_player_skin_cutout: EntityModelTexturedMesh,
    pub(super) dynamic_player_skin_translucent: EntityModelTexturedMesh,
    /// Ready remote non-skin player profile textures, such as capes and elytra, use a separate
    /// variable-size atlas while preserving the vanilla render type.
    pub(super) dynamic_player_texture_cutout: EntityModelTexturedMesh,
    pub(super) dynamic_player_texture_translucent: EntityModelTexturedMesh,
    /// Translucent scrolling overlay (vanilla `breezeWind` — the wind charge).
    pub(super) scroll: EntityModelScrollMesh,
    /// Additive scrolling overlay (vanilla `energySwirl` — the charged-creeper / wither glow).
    pub(super) scroll_additive: EntityModelScrollMesh,
    /// Vanilla-shaped submit metadata for textured entity models. The current backend still folds
    /// compatible submits into shared meshes, but this preserves render type, order, tint, texture and
    /// transform so residual emits can migrate to explicit submissions one by one.
    pub(super) submissions: Vec<EntityModelRenderSubmission>,
}

impl EntityModelTexturedMeshes {
    fn new() -> Self {
        Self {
            cutout: EntityModelTexturedMesh::new(),
            translucent: EntityModelTexturedMesh::new(),
            eyes: EntityModelTexturedMesh::new(),
            dynamic_player_skin_cutout: EntityModelTexturedMesh::new(),
            dynamic_player_skin_translucent: EntityModelTexturedMesh::new(),
            dynamic_player_texture_cutout: EntityModelTexturedMesh::new(),
            dynamic_player_texture_translucent: EntityModelTexturedMesh::new(),
            scroll: EntityModelScrollMesh::new(),
            scroll_additive: EntityModelScrollMesh::new(),
            submissions: Vec::new(),
        }
    }

    fn mesh_mut(
        &mut self,
        render_type: EntityModelLayerRenderType,
    ) -> &mut EntityModelTexturedMesh {
        match render_type.mesh_bucket() {
            EntityModelLayerRenderBucket::Cutout => &mut self.cutout,
            EntityModelLayerRenderBucket::Translucent => &mut self.translucent,
            EntityModelLayerRenderBucket::Eyes => &mut self.eyes,
            EntityModelLayerRenderBucket::Scroll | EntityModelLayerRenderBucket::AdditiveScroll => {
                panic!("scroll render types are not emitted into textured mesh buckets")
            }
        }
    }

    fn dynamic_player_skin_mesh_mut(
        &mut self,
        render_type: EntityModelLayerRenderType,
    ) -> &mut EntityModelTexturedMesh {
        match render_type.mesh_bucket() {
            EntityModelLayerRenderBucket::Cutout => &mut self.dynamic_player_skin_cutout,
            EntityModelLayerRenderBucket::Translucent => &mut self.dynamic_player_skin_translucent,
            EntityModelLayerRenderBucket::Eyes
            | EntityModelLayerRenderBucket::Scroll
            | EntityModelLayerRenderBucket::AdditiveScroll => {
                panic!("unsupported dynamic player skin render type")
            }
        }
    }

    fn dynamic_player_texture_mesh_mut(
        &mut self,
        render_type: EntityModelLayerRenderType,
    ) -> &mut EntityModelTexturedMesh {
        match render_type.mesh_bucket() {
            EntityModelLayerRenderBucket::Cutout => &mut self.dynamic_player_texture_cutout,
            EntityModelLayerRenderBucket::Translucent => {
                &mut self.dynamic_player_texture_translucent
            }
            EntityModelLayerRenderBucket::Eyes
            | EntityModelLayerRenderBucket::Scroll
            | EntityModelLayerRenderBucket::AdditiveScroll => {
                panic!("unsupported dynamic player texture render type")
            }
        }
    }

    fn record_submission(&mut self, submit: EntityModelSubmissionEmit) {
        self.submissions.push(submit.into());
    }
}

#[derive(Clone, Copy)]
struct EntityModelSubmissionEmit {
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    dynamic_player_skin: Option<EntityDynamicPlayerSkin>,
    dynamic_player_texture: Option<EntityDynamicPlayerTexture>,
    tint: [f32; 4],
    transform: Mat4,
    order: i32,
    submit_sequence: u32,
}

impl EntityModelSubmissionEmit {
    fn new(
        render_type: EntityModelLayerRenderType,
        texture: EntityModelTextureRef,
        tint: [f32; 4],
        transform: Mat4,
        order: i32,
        submit_sequence: u32,
    ) -> Self {
        Self {
            render_type,
            texture,
            dynamic_player_skin: None,
            dynamic_player_texture: None,
            tint,
            transform,
            order,
            submit_sequence,
        }
    }

    fn with_dynamic_player_skin(mut self, skin: EntityDynamicPlayerSkin) -> Self {
        self.dynamic_player_skin = Some(skin);
        self
    }

    fn with_dynamic_player_texture(mut self, texture: EntityDynamicPlayerTexture) -> Self {
        self.dynamic_player_texture = Some(texture);
        self
    }
}

impl From<EntityModelSubmissionEmit> for EntityModelRenderSubmission {
    fn from(submit: EntityModelSubmissionEmit) -> Self {
        Self {
            render_type: submit.render_type,
            texture: submit.texture,
            dynamic_player_skin: submit.dynamic_player_skin,
            dynamic_player_texture: submit.dynamic_player_texture,
            tint: submit.tint,
            transform: submit.transform,
            order: submit.order,
            submit_sequence: submit.submit_sequence,
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

#[cfg(test)]
pub(super) fn entity_model_textured_meshes(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    entity_model_textured_meshes_with_dynamic_textures(instances, atlas, None, None)
}

#[cfg(test)]
pub(super) fn entity_model_textured_meshes_with_dynamic_skins(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) -> EntityModelTexturedMeshes {
    entity_model_textured_meshes_with_dynamic_textures(
        instances,
        atlas,
        dynamic_player_skin_atlas,
        None,
    )
}

pub(super) fn entity_model_textured_meshes_with_dynamic_textures(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new();
    for instance in instances {
        if instance.render_state.invisible {
            continue;
        }
        let cutout_start = meshes.cutout.vertices.len();
        let translucent_start = meshes.translucent.vertices.len();
        let eyes_start = meshes.eyes.vertices.len();
        let dynamic_player_skin_cutout_start = meshes.dynamic_player_skin_cutout.vertices.len();
        let dynamic_player_skin_translucent_start =
            meshes.dynamic_player_skin_translucent.vertices.len();
        let dynamic_player_texture_cutout_start =
            meshes.dynamic_player_texture_cutout.vertices.len();
        let dynamic_player_texture_translucent_start =
            meshes.dynamic_player_texture_translucent.vertices.len();
        let handled = {
            let mut sink = TexturedSink {
                meshes: &mut meshes,
                atlas,
            };
            dispatch_uniform_entity_model(instance, &mut sink)
        };
        if !handled {
            // Only the bespoke textured emits remain here — the recolor / two-tree / family / part-vis /
            // single-pass entities that the shared dispatch leaves out. Colored-only uniform kinds emit no
            // textured geometry (their dispatch call walks an empty pass list, a no-op), so they must NOT
            // appear here; every kind without a textured arm falls into `_ => {}`.
            match instance.kind {
                EntityModelKind::WindCharge => {
                    emit_wind_charge_scroll_model(&mut meshes, *instance, atlas);
                }
                EntityModelKind::ShulkerBullet => {
                    emit_shulker_bullet_textured_model(&mut meshes, *instance, atlas);
                }
                EntityModelKind::EndCrystal => {
                    emit_end_crystal_textured_model(&mut meshes, *instance, atlas);
                }
                EntityModelKind::Llama {
                    family,
                    variant,
                    baby,
                    has_chest,
                } => {
                    emit_llama_textured_model(
                        &mut meshes,
                        *instance,
                        family,
                        variant,
                        baby,
                        has_chest,
                        atlas,
                    );
                }
                EntityModelKind::Camel { family, baby } => {
                    emit_camel_textured_model(&mut meshes, *instance, family, baby, atlas);
                }
                EntityModelKind::Squid { glow, baby } => {
                    emit_squid_textured_model(&mut meshes, *instance, glow, baby, atlas);
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
                EntityModelKind::Slime { size } => {
                    emit_slime_textured_model(&mut meshes, *instance, size, atlas);
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
                EntityModelKind::Hoglin { family, baby } => {
                    emit_hoglin_textured_model(&mut meshes, *instance, family, baby, atlas);
                }
                EntityModelKind::Player { skin, parts } => {
                    emit_player_textured_model(
                        &mut meshes,
                        *instance,
                        skin,
                        parts,
                        atlas,
                        dynamic_player_skin_atlas,
                    );
                }
                EntityModelKind::Sheep {
                    baby,
                    sheared,
                    wool_color,
                    jeb,
                    age_ticks,
                } => {
                    emit_sheep_textured_model(
                        &mut meshes,
                        *instance,
                        baby,
                        sheared,
                        wool_color,
                        jeb,
                        age_ticks,
                        atlas,
                    );
                }
                EntityModelKind::Skeleton => {
                    emit_skeleton_textured_model(&mut meshes, *instance, None, atlas);
                }
                EntityModelKind::SkeletonVariant { family } => {
                    emit_skeleton_textured_model(&mut meshes, *instance, Some(family), atlas);
                }
                EntityModelKind::Horse { baby, markings, .. } => {
                    emit_horse_textured_model(&mut meshes, *instance, baby, markings, atlas);
                }
                EntityModelKind::Donkey {
                    family,
                    baby,
                    has_chest,
                } => {
                    emit_donkey_textured_model(
                        &mut meshes,
                        *instance,
                        family,
                        baby,
                        has_chest,
                        atlas,
                    );
                }
                EntityModelKind::UndeadHorse { baby, .. } => {
                    emit_undead_horse_textured_model(&mut meshes, *instance, baby, atlas);
                }
                _ => {}
            }
        }
        // The charged-creeper and powered-wither energy swirls are additive scrolling overlays layered
        // on top of the base model (already emitted by the shared dispatch), so they run regardless of
        // `handled`.
        emit_charged_creeper_energy_swirl(&mut meshes, *instance, atlas);
        emit_wither_energy_swirl(&mut meshes, *instance, atlas);
        // The breeze's swirling wind body is a translucent scrolling overlay (vanilla `BreezeWindLayer`)
        // layered on top of the base body (already emitted by the shared dispatch), so it likewise runs
        // regardless of `handled`.
        emit_breeze_wind_scroll_model(&mut meshes, *instance, atlas);
        // The guardian attack beam is a world-space billboarded prism from the guardian eye to its
        // target; it folds into the scroll (tiled) pass and runs regardless of `handled`.
        emit_guardian_beam(&mut meshes, *instance, atlas);
        // The end-crystal healing beam is custom world-space geometry submitted after the model body.
        emit_end_crystal_beam(&mut meshes, *instance, atlas);
        // The ender-dragon healing beam reuses the same vanilla custom-geometry submit after body+eyes.
        emit_ender_dragon_beam(&mut meshes, *instance, atlas);
        emit_player_cape_layer(&mut meshes, *instance, dynamic_player_texture_atlas);
        // Worn armor is a cutout overlay draped on the host humanoid pose; it runs regardless of
        // `handled` and folds into the cutout pass before the shared light/overlay fill below.
        emit_worn_humanoid_armor(&mut meshes, *instance, atlas);
        // Skull block items in the head slot use vanilla `CustomHeadLayer`'s skull branch: a static
        // `SkullModel` mob head attached to the host head, not the generic item-model HEAD display path.
        emit_custom_head_skull_layer(&mut meshes, *instance, atlas, dynamic_player_skin_atlas);
        // Vanilla `WingsLayer`: a WINGS equipment layer over the ElytraModel for players,
        // humanoid mobs, and armor stands. Only players can replace the equipment texture with
        // a ready profile elytra/cape texture.
        emit_wings_layer(&mut meshes, *instance, atlas, dynamic_player_texture_atlas);
        // The pig saddle is a simple equipment overlay over the adult pig body.
        emit_pig_saddle_layer(&mut meshes, *instance, atlas);
        // Wolf body armor uses the adult WOLF_ARMOR equipment layer and optional damage cracks.
        emit_wolf_body_armor_layer(&mut meshes, *instance, atlas);
        // Horse/zombie-horse body armor uses the adult HORSE_BODY equipment layer.
        emit_equine_body_armor_layer(&mut meshes, *instance, atlas);
        // Horse/donkey/mule/undead-horse saddles use the shared EquineSaddleModel tree.
        emit_equine_saddle_layer(&mut meshes, *instance, atlas);
        // Strider saddles reuse the adult strider body layer with the strider equipment texture.
        emit_strider_saddle_layer(&mut meshes, *instance, atlas);
        // Camel and camel-husk saddles use the adult CamelSaddleModel tree.
        emit_camel_saddle_layer(&mut meshes, *instance, atlas);
        // Living and zombie nautilus body armor uses the adult NautilusArmorModel tree.
        emit_nautilus_body_armor_layer(&mut meshes, *instance, atlas);
        // Living and zombie nautilus saddles use the adult NautilusSaddleModel tree.
        emit_nautilus_saddle_layer(&mut meshes, *instance, atlas);
        // VillagerProfessionLayer overlays (biome type, profession, level badge) are cutout layers
        // over the base villager or zombie-villager model and share the same light/overlay fill.
        emit_villager_profession_layers(&mut meshes, *instance, atlas);
        let light = instance.render_state.shader_light();
        fill_entity_textured_light(&mut meshes.cutout, cutout_start, light);
        fill_entity_textured_light(&mut meshes.translucent, translucent_start, light);
        fill_entity_textured_light(&mut meshes.eyes, eyes_start, light);
        fill_entity_textured_light(
            &mut meshes.dynamic_player_skin_cutout,
            dynamic_player_skin_cutout_start,
            light,
        );
        fill_entity_textured_light(
            &mut meshes.dynamic_player_skin_translucent,
            dynamic_player_skin_translucent_start,
            light,
        );
        fill_entity_textured_light(
            &mut meshes.dynamic_player_texture_cutout,
            dynamic_player_texture_cutout_start,
            light,
        );
        fill_entity_textured_light(
            &mut meshes.dynamic_player_texture_translucent,
            dynamic_player_texture_translucent_start,
            light,
        );
        let overlay = instance.render_state.overlay_coords();
        fill_entity_textured_overlay(&mut meshes.cutout, cutout_start, overlay);
        fill_entity_textured_overlay(&mut meshes.translucent, translucent_start, overlay);
        fill_entity_textured_overlay(&mut meshes.eyes, eyes_start, overlay);
        fill_entity_textured_overlay(
            &mut meshes.dynamic_player_skin_cutout,
            dynamic_player_skin_cutout_start,
            overlay,
        );
        fill_entity_textured_overlay(
            &mut meshes.dynamic_player_skin_translucent,
            dynamic_player_skin_translucent_start,
            overlay,
        );
        fill_entity_textured_overlay(
            &mut meshes.dynamic_player_texture_cutout,
            dynamic_player_texture_cutout_start,
            overlay,
        );
        fill_entity_textured_overlay(
            &mut meshes.dynamic_player_texture_translucent,
            dynamic_player_texture_translucent_start,
            overlay,
        );
    }
    meshes
}

#[cfg(test)]
pub(super) fn dynamic_player_texture_test_meshes(
    render_type: EntityModelLayerRenderType,
    dynamic_player_texture: EntityDynamicPlayerTexture,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new();
    let model = PlayerModel::new(false);
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

/// Vanilla `ShulkerBulletRenderer.submit`: the base `spark.png` model is submitted first, then the same
/// posed model is scaled by 1.5 and submitted as translucent white with alpha `0x26`.
fn emit_shulker_bullet_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let mut model = ShulkerBulletModel::new();
    model.prepare(&instance);
    let transform = shulker_bullet_model_root_transform(instance);
    render_textured_pass_ordered(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::EntityCutout,
        SHULKER_BULLET_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        0,
        0,
        atlas,
    );
    render_textured_pass_ordered(
        meshes,
        &model,
        transform * Mat4::from_scale(Vec3::splat(1.5)),
        EntityModelLayerRenderType::EntityTranslucent,
        SHULKER_BULLET_TEXTURE_REF,
        [1.0, 1.0, 1.0, 38.0 / 255.0],
        1,
        1,
        atlas,
    );
}

/// Vanilla `EndCrystalRenderer.submit`: render `EndCrystalModel` with `end_crystal.png` after the
/// renderer root transform (`scale(2)` + `translate(0,-0.5,0)`). The optional `DATA_BEAM_TARGET`
/// custom geometry is submitted separately by [`emit_end_crystal_beam`].
fn emit_end_crystal_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let root = end_crystal_model_root_transform(instance);
    let tint = [1.0, 1.0, 1.0, 1.0];
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::EntityCutout,
        END_CRYSTAL_TEXTURE_REF,
        tint,
        root,
        0,
        0,
    );
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        if instance.render_state.end_crystal_shows_bottom {
            emit_textured_model_parts(
                mesh,
                &END_CRYSTAL_TEXTURED_PARTS[..1],
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        }

        let age = instance.render_state.age_in_ticks;
        let bob = end_crystal_bob_y(age);
        let (q_outer, q_inner) = end_crystal_glass_quaternions(age);
        let centre = submit.transform
            * part_pose_transform(PartPose {
                offset: [0.0, 24.0 + bob, 0.0],
                rotation: [0.0, 0.0, 0.0],
            });
        let outer_t = centre * Mat4::from_quat(q_outer);
        let inner_t = outer_t * Mat4::from_quat(q_inner);
        let core_t = inner_t * Mat4::from_quat(q_inner);
        for cube in END_CRYSTAL_TEXTURED_PARTS[1].cubes {
            emit_textured_model_cube(mesh, outer_t, *cube, submit.texture, entry.uv, submit.tint);
        }
        for cube in END_CRYSTAL_TEXTURED_PARTS[2].cubes {
            emit_textured_model_cube(mesh, inner_t, *cube, submit.texture, entry.uv, submit.tint);
        }
        for cube in END_CRYSTAL_TEXTURED_PARTS[3].cubes {
            emit_textured_model_cube(mesh, core_t, *cube, submit.texture, entry.uv, submit.tint);
        }
    });
}

/// Render one textured pass of an already-prepared model: look up the texture's atlas entry and,
/// if present, walk the posed tree into the pass's mesh. The shared terminal of every textured
/// emit — the textured analogue of the colored path's `render_colored`.
fn render_textured_pass<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    tint: [f32; 4],
    atlas: &EntityModelTextureAtlasLayout,
) {
    render_textured_pass_ordered(
        meshes,
        model,
        transform,
        render_type,
        texture,
        tint,
        0,
        0,
        atlas,
    );
}

fn render_textured_pass_ordered<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    tint: [f32; 4],
    order: i32,
    submit_sequence: u32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let submit = EntityModelSubmissionEmit::new(
        render_type,
        texture,
        tint,
        transform,
        order,
        submit_sequence,
    );
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

fn render_textured_pass_with_dynamic_player_skin<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    dynamic_player_skin: EntityDynamicPlayerSkin,
    tint: [f32; 4],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let submit = EntityModelSubmissionEmit::new(render_type, texture, tint, transform, 0, 0)
        .with_dynamic_player_skin(dynamic_player_skin);
    if dynamic_player_skin.status == EntityDynamicPlayerSkinStatus::Ready {
        if let Some(entry) =
            dynamic_player_skin_atlas_entry(dynamic_player_skin_atlas, dynamic_player_skin.handle)
        {
            render_textured_dynamic_player_skin_submission(meshes, submit, entry, |mesh, entry| {
                model.root().render_textured(
                    mesh,
                    submit.transform,
                    submit.texture,
                    entry.uv,
                    submit.tint,
                );
            });
            return;
        }
    }

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

#[cfg(test)]
fn render_textured_pass_with_dynamic_player_texture<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    render_type: EntityModelLayerRenderType,
    texture: EntityModelTextureRef,
    dynamic_player_texture: EntityDynamicPlayerTexture,
    tint: [f32; 4],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let submit = EntityModelSubmissionEmit::new(render_type, texture, tint, transform, 0, 0)
        .with_dynamic_player_texture(dynamic_player_texture);
    if let Some(entry) =
        dynamic_player_texture_atlas_entry(dynamic_player_texture_atlas, dynamic_player_texture)
    {
        render_textured_dynamic_player_texture_submission(meshes, submit, entry, |mesh, entry| {
            model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
        return;
    }

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

fn render_textured_dynamic_player_skin_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    entry: EntityDynamicPlayerSkinAtlasEntry,
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityDynamicPlayerSkinAtlasEntry),
) {
    meshes.record_submission(submit);
    emit(
        meshes.dynamic_player_skin_mesh_mut(submit.render_type),
        entry,
    );
}

fn render_textured_dynamic_player_texture_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    entry: EntityDynamicPlayerTextureAtlasEntry,
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityDynamicPlayerTextureAtlasEntry),
) {
    meshes.record_submission(submit);
    emit(
        meshes.dynamic_player_texture_mesh_mut(submit.render_type),
        entry,
    );
}

fn render_textured_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    atlas: &EntityModelTextureAtlasLayout,
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityModelTextureAtlasEntry),
) {
    meshes.record_submission(submit);
    if let Some(entry) = entity_model_texture_atlas_entry(atlas, submit.texture) {
        emit(meshes.mesh_mut(submit.render_type), entry);
    }
}

fn scroll_mesh_mut(
    meshes: &mut EntityModelTexturedMeshes,
    render_type: EntityModelLayerRenderType,
) -> &mut EntityModelScrollMesh {
    scroll_bucket_mut(meshes, render_type.mesh_bucket())
}

fn scroll_bucket_mut(
    meshes: &mut EntityModelTexturedMeshes,
    bucket: EntityModelLayerRenderBucket,
) -> &mut EntityModelScrollMesh {
    match bucket {
        EntityModelLayerRenderBucket::Scroll => &mut meshes.scroll,
        EntityModelLayerRenderBucket::AdditiveScroll => &mut meshes.scroll_additive,
        _ => panic!("only scroll render types are emitted through the scroll mesh"),
    }
}

fn render_scrolled_textured_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    atlas: &EntityModelTextureAtlasLayout,
    uv_offset: [f32; 2],
    emit: impl FnOnce(&mut EntityModelTexturedMesh, EntityModelTextureAtlasEntry),
) {
    meshes.record_submission(submit);
    let Some(entry) = entity_model_texture_atlas_entry(atlas, submit.texture) else {
        return;
    };
    let mut scratch = EntityModelTexturedMesh::new();
    emit(&mut scratch, entry);
    append_scrolled_textured_mesh(
        scroll_mesh_mut(meshes, submit.render_type),
        &scratch,
        entry.uv,
        uv_offset,
    );
}

fn render_scroll_geometry_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    target_bucket: EntityModelLayerRenderBucket,
    atlas: &EntityModelTextureAtlasLayout,
    emit: impl FnOnce(
        &mut EntityModelScrollMesh,
        EntityModelTextureAtlasEntry,
        EntityModelSubmissionEmit,
    ),
) {
    meshes.record_submission(submit);
    let Some(entry) = entity_model_texture_atlas_entry(atlas, submit.texture) else {
        return;
    };
    emit(scroll_bucket_mut(meshes, target_bucket), entry, submit);
}

fn render_textured_root_pass(
    meshes: &mut EntityModelTexturedMeshes,
    root: &ModelPart,
    transform: Mat4,
    pass: EntityModelLayerPass,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let submit = EntityModelSubmissionEmit::new(
        pass.render_type,
        pass.texture,
        pass.tint,
        transform,
        pass.order,
        pass.submit_sequence,
    );
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        root.render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

/// Render a model's full textured layer-pass list (already prepared) into `meshes`.
pub(in crate::entity_models) fn render_textured_layers<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    passes: impl IntoIterator<Item = EntityModelLayerPass>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    for pass in passes {
        match pass.visibility {
            // A part-subset emissive overlay (vanilla `retainExactParts`): render only its named parts.
            layers::EntityModelLayerVisibility::RetainedParts(parts) => {
                let submit = EntityModelSubmissionEmit::new(
                    pass.render_type,
                    pass.texture,
                    pass.tint,
                    transform,
                    pass.order,
                    pass.submit_sequence,
                );
                render_textured_submission(meshes, submit, atlas, |mesh, entry| {
                    model.root().render_textured_retained(
                        mesh,
                        submit.transform,
                        submit.texture,
                        entry.uv,
                        submit.tint,
                        "",
                        parts,
                    );
                });
            }
            // `All` (and the player-parts case, whose subset is pre-applied to the tree) render whole.
            _ => render_textured_pass_ordered(
                meshes,
                model,
                transform,
                pass.render_type,
                pass.texture,
                pass.tint,
                pass.order,
                pass.submit_sequence,
                atlas,
            ),
        }
    }
}

/// The textured camel base layer. Vanilla `CamelModel.setupAnim` drives every limb via
/// baked `KeyframeAnimation`s (walk/sit/standup/idle/dash) plus a direct head yaw/pitch
/// clamp ([`camel_clamped_head_look`]). The head look and the walk (adult/husk `CAMEL_WALK`,
/// baby `CAMEL_BABY_WALK`), the sit/standup one-shots, and the dash gallop are reproduced here;
/// the idle timer remains deferred. The camel husk shares the adult mesh, differing only in texture.
fn emit_camel_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: CamelModelFamily,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `CamelModel` tree drives both render paths; `new` selects the adult / baby / husk mesh
    // and walk, and `setup_anim` clamps the head look and samples the walk (`root` roll, leg / ear / tail
    // swing, `head` pitch added onto the look, baby `body` dip). The camel is a single cutout pass; the
    // family / baby texture comes from the pass.
    let transform = entity_model_root_transform(instance);
    let mut model = CamelModel::new(family, baby);
    model.prepare(&instance);
    render_textured_layers(
        meshes,
        &model,
        transform,
        camel_textured_layer_passes(family, baby),
        atlas,
    );
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
        render_textured_root_pass(meshes, root, transform, pass, atlas);
    }
}

/// The textured squid / glow squid base layer. The unified [`SquidModel`] tree (body + the
/// procedural eight-tentacle ring) runs the shared `SquidModel.setupAnim` and renders under
/// [`squid_model_root_transform`]; the variant texture's atlas UV is resolved once. The glow squid
/// differs only by texture (its emissive light boost is deferred lighting).
/// The wind charge's scrolling `breezeWind` overlay (vanilla `WindChargeRenderer`): the whole
/// `WindChargeModel` rendered with the `breezeWind` render type, whose texture matrix scrolls the U
/// coordinate by `xOffset(ageInTicks) % 1 = (ageInTicks · 0.03) % 1` (V fixed at `0`). We render the
/// model once with the normal atlas UVs into a scratch mesh, then fold it into the scrolling-overlay
/// mesh, baking the per-instance U offset and carrying the atlas sub-rect for the shader's `fract` wrap.
fn emit_wind_charge_scroll_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = wind_charge_model_root_transform(instance);
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::BreezeWind,
        WIND_CHARGE_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        transform,
        0,
        0,
    );
    let mut model = WindChargeModel::new();
    model.prepare(&instance);
    // Vanilla `WindChargeRenderer.xOffset(t) = t · 0.03`, taken `% 1.0`; `ageInTicks ≥ 0` so the Java
    // float modulo is `rem_euclid`. V does not scroll.
    let u_offset = (instance.render_state.age_in_ticks * 0.03).rem_euclid(1.0);
    render_scrolled_textured_submission(meshes, submit, atlas, [u_offset, 0.0], |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

/// The breeze's swirling wind body (vanilla `BreezeWindLayer`): the SEPARATE [`BreezeWindModel`] (the
/// `wind_body` shell chain on the 128×128 `breeze_wind.png`) rendered with the `breezeWind` render
/// type, whose texture matrix scrolls the U coordinate by `xOffset(ageInTicks) % 1 = (ageInTicks ·
/// 0.02) % 1` (V fixed at `0`). Like the wind charge, we render the wind model once with the normal
/// atlas UVs into a scratch mesh — its `setup_anim` applies the same idle sway + action swirls/pulses
/// as the base body so the two layers move together — then fold it into the translucent scrolling
/// overlay mesh, baking the per-instance U offset and carrying the atlas sub-rect for the shader wrap.
fn emit_breeze_wind_scroll_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::Breeze) {
        return;
    }
    let transform = entity_model_root_transform(instance);
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::BreezeWind,
        BREEZE_WIND_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        transform,
        1,
        1,
    );
    let mut model = BreezeWindModel::new();
    model.prepare(&instance);
    // Vanilla `BreezeWindLayer.xOffset(t) = t · 0.02`, taken `% 1.0`; `ageInTicks ≥ 0` so the Java
    // float modulo is `rem_euclid`. V does not scroll.
    let u_offset = (instance.render_state.age_in_ticks * 0.02).rem_euclid(1.0);
    render_scrolled_textured_submission(meshes, submit, atlas, [u_offset, 0.0], |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

/// The charged creeper's `CreeperPowerLayer` energy swirl (vanilla `EnergySwirlLayer`): when the
/// synced `isPowered` is set, the inflated `CREEPER_ARMOR` model (`CubeDeformation 2.0`, driven by the
/// same `setup_anim` so it tracks the body pose) is drawn with the additive, emissive `energySwirl`
/// render type — `creeper_armor.png` scrolling on both axes by `xOffset(ageInTicks) % 1 =
/// (ageInTicks · 0.01) % 1`, tinted by the vanilla `0xFF808080` half-grey. Folded into the additive
/// scroll mesh the same way the wind charge folds into the translucent one.
fn emit_charged_creeper_energy_swirl(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !instance.render_state.creeper_powered || !matches!(instance.kind, EntityModelKind::Creeper)
    {
        return;
    }
    let transform = creeper_model_root_transform(instance);
    let grey = 128.0 / 255.0;
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::EnergySwirl,
        CREEPER_ARMOR_TEXTURE_REF,
        [grey, grey, grey, 1.0],
        transform,
        1,
        1,
    );
    let mut model = CreeperModel::new_armor();
    model.prepare(&instance);
    // Vanilla creeper `xOffset(t) = t · 0.01`, taken `% 1.0` on both U and V.
    let offset = (instance.render_state.age_in_ticks * 0.01).rem_euclid(1.0);
    // Vanilla `EnergySwirlLayer` tints by `0xFF808080` (half grey) under additive blend.
    render_scrolled_textured_submission(meshes, submit, atlas, [offset, offset], |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

/// The wither boss's `WitherArmorLayer` energy swirl (vanilla `EnergySwirlLayer`, the same family as
/// the charged creeper): when `isPowered` (the wither sits at or below half health), the inflated
/// `WITHER_ARMOR` model (`INNER_ARMOR_DEFORMATION` = `CubeDeformation 0.5`, driven by the same
/// `setup_anim` so it breathes with the body) is drawn with the additive, emissive `energySwirl`
/// render type — `wither_armor.png` tinted by the vanilla `0xFF808080` half-grey. Unlike the creeper's
/// linear scroll, the wither's `xOffset(t) = cos(t · 0.02) · 3` oscillates the U coordinate while V
/// scrolls linearly at `t · 0.01`; both are taken `% 1.0`. Folded into the same additive scroll mesh.
fn emit_wither_energy_swirl(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !instance.render_state.wither_powered || !matches!(instance.kind, EntityModelKind::Wither) {
        return;
    }
    let transform = wither_model_root_transform(instance);
    let grey = 128.0 / 255.0;
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::EnergySwirl,
        WITHER_ARMOR_TEXTURE_REF,
        [grey, grey, grey, 1.0],
        transform,
        1,
        1,
    );
    let mut model = WitherModel::new_armor();
    model.prepare(&instance);
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
            model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
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
fn emit_guardian_beam(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(beam) = instance.render_state.guardian_beam else {
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
    // `BEAM_RENDER_TYPE = RenderTypes.entityCutout(guardian_beam.png)` on the same collector. Preserve
    // that as a vanilla-shaped submission even though the current backend folds the tiled V coordinates
    // into the scroll mesh.
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::EntityCutout,
        GUARDIAN_BEAM_TEXTURE_REF,
        tint,
        transform,
        0,
        1,
    );
    render_scroll_geometry_submission(
        meshes,
        submit,
        EntityModelLayerRenderBucket::Scroll,
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
            for (x, y, z, u, v) in vertices {
                let world = submit.transform.transform_point3(Vec3::new(x, y, z));
                mesh.vertices.push(EntityModelScrollVertex {
                    position: world.to_array(),
                    local_uv: [u, v],
                    uv_rect_min: rect.min,
                    uv_rect_size: size,
                    tint: submit.tint,
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
fn emit_end_crystal_beam(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::EndCrystal) {
        return;
    }
    let Some(beam) = instance.render_state.end_crystal_beam else {
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
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::EndCrystalBeam,
        END_CRYSTAL_BEAM_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        transform,
        0,
        1,
    );
    emit_crystal_beam_submission(meshes, submit, atlas, delta.length(), age);
}

/// Vanilla `EnderDragonRenderer.submit`: after body and eyes submits, a dragon with
/// `EnderDragonRenderState.beamOffset` calls the same `submitCrystalBeams` helper from the dragon's
/// entity-origin pose. Unlike an end crystal, the dragon does not pre-translate by the offset and does
/// not invert the delta; its `beamOffset` already points from the dragon to the bobbed crystal.
fn emit_ender_dragon_beam(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::EnderDragon) {
        return;
    }
    let Some(beam) = instance.render_state.ender_dragon_beam else {
        return;
    };

    let delta = Vec3::from_array(beam.beam_offset);
    let horizontal_length = (delta.x * delta.x + delta.z * delta.z).sqrt();
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0))
        * Mat4::from_rotation_y(-delta.z.atan2(delta.x) - std::f32::consts::FRAC_PI_2)
        * Mat4::from_rotation_x(-horizontal_length.atan2(delta.y) - std::f32::consts::FRAC_PI_2);
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::EndCrystalBeam,
        END_CRYSTAL_BEAM_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        transform,
        0,
        2,
    );
    emit_crystal_beam_submission(
        meshes,
        submit,
        atlas,
        delta.length(),
        instance.render_state.age_in_ticks,
    );
}

fn emit_crystal_beam_submission(
    meshes: &mut EntityModelTexturedMeshes,
    submit: EntityModelSubmissionEmit,
    atlas: &EntityModelTextureAtlasLayout,
    length: f32,
    age: f32,
) {
    render_scroll_geometry_submission(
        meshes,
        submit,
        submit.render_type.mesh_bucket(),
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
                    mesh.vertices.push(EntityModelScrollVertex {
                        position: submit.transform.transform_point3(position).to_array(),
                        local_uv,
                        uv_rect_min: rect.min,
                        uv_rect_size: size,
                        tint,
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

/// The `HumanoidArmorLayer` worn-armor overlay (vanilla `HumanoidArmorLayer.submit`): for each filled
/// equipment slot the inflated `HumanoidArmorModel` piece (helmet / chestplate / leggings / boots) is
/// draped on the host humanoid's posed limbs ([`ModelPart::copy_child_poses_from`] = vanilla
/// `copyPropertiesTo`) and drawn into the cutout pass with the material's equipment-asset texture. The
/// pieces render in the vanilla order (chest, legs, feet, head). `transform` is the host entity's root
/// transform so the armor sits exactly on the body. The enchant-glint, armor-trim, and leather-dye
/// tint passes are deferred coverage.
fn emit_humanoid_armor(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    host_root: &ModelPart,
    transform: Mat4,
    outer: f32,
    baby_kind: Option<HumanoidBabyArmorKind>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let render_state = &instance.render_state;
    for (slot, material, dye) in [
        (
            HumanoidArmorSlot::Chest,
            render_state.chest_armor,
            render_state.chest_armor_dye,
        ),
        (
            HumanoidArmorSlot::Legs,
            render_state.legs_armor,
            render_state.legs_armor_dye,
        ),
        (
            HumanoidArmorSlot::Feet,
            render_state.feet_armor,
            render_state.feet_armor_dye,
        ),
        (
            HumanoidArmorSlot::Head,
            render_state.head_armor,
            render_state.head_armor_dye,
        ),
    ] {
        let Some(material) = material else {
            continue;
        };
        let Some(texture) = armor_slot_texture_for_layer(material, slot, baby_kind.is_some())
        else {
            continue;
        };
        let submit_sequence = match slot {
            HumanoidArmorSlot::Chest => 1,
            HumanoidArmorSlot::Legs => 2,
            HumanoidArmorSlot::Feet => 3,
            HumanoidArmorSlot::Head => 4,
        };
        let mut tree = if let Some(kind) = baby_kind {
            slot.build_baby_tree(kind)
        } else {
            slot.build_tree(outer)
        };
        if baby_kind.is_some() {
            tree.copy_child_animation_from(host_root, slot.baby_pose_part_names());
        } else {
            tree.copy_child_poses_from(host_root, slot.part_names());
        }
        let submit = EntityModelSubmissionEmit::new(
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            texture,
            armor_layer_tint(material, dye),
            transform,
            1,
            submit_sequence,
        );
        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            tree.render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
    }
}

/// Worn armor for the humanoid armor wearers (vanilla `HumanoidModel.createArmorMeshSet`, `INNER 0.5`
/// / `OUTER 1.0`, the standard baby zombie/husk/drowned `createBabyArmorMeshSet`, or the piglin
/// family's `OUTER 1.02`). The base body is emitted by the shared dispatch / bespoke emits; here we
/// rebuild and pose an identical host humanoid model purely to read its limb poses, then drape the
/// armor pieces on it ([`emit_humanoid_armor`]). Covered: the adult zombie family (zombie, husk,
/// drowned, zombie villager), standard baby zombie/husk/drowned/zombie-villager armor, the skeleton
/// family (skeleton, stray, wither/normal/bogged), the player, the adult piglin family (piglin,
/// piglin brute, zombified piglin), and baby piglin / zombified-piglin armor models. DEFERRED:
/// enchant-glint, armor-trim, and any remaining mob-specific armor models.
fn emit_worn_humanoid_armor(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let render_state = &instance.render_state;
    if render_state.head_armor.is_none()
        && render_state.chest_armor.is_none()
        && render_state.legs_armor.is_none()
        && render_state.feet_armor.is_none()
    {
        return;
    }
    match instance.kind {
        EntityModelKind::Zombie { baby: false } => {
            let mut host = ZombieModel::new(false);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                atlas,
            );
        }
        EntityModelKind::Zombie { baby: true } => {
            let mut host = ZombieModel::new(true);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                Some(HumanoidBabyArmorKind::Standard),
                atlas,
            );
        }
        EntityModelKind::Giant => {
            let mut host = ZombieModel::new(false);
            host.prepare(&instance);
            let transform = mesh_transformer_scaled_model_root_transform(instance, GIANT_SCALE);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                atlas,
            );
        }
        EntityModelKind::ZombieVariant {
            family,
            baby: false,
        } => {
            // The husk wears the `HUSK_SCALE` mesh-transformer scale; the other variants render at 1.0×.
            let transform = if matches!(family, ZombieVariantModelFamily::Husk) {
                mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE)
            } else if matches!(family, ZombieVariantModelFamily::Drowned) {
                drowned_model_root_transform(instance)
            } else {
                entity_model_root_transform(instance)
            };
            let mut host = ZombieVariantModel::new(family, false);
            host.prepare(&instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                atlas,
            );
        }
        EntityModelKind::ZombieVariant {
            family: family @ (ZombieVariantModelFamily::Husk | ZombieVariantModelFamily::Drowned),
            baby: true,
        } => {
            let transform = if matches!(family, ZombieVariantModelFamily::Husk) {
                mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE)
            } else {
                drowned_model_root_transform(instance)
            };
            let mut host = ZombieVariantModel::new(family, true);
            host.prepare(&instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                Some(HumanoidBabyArmorKind::Standard),
                atlas,
            );
        }
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: true,
        } => {
            let mut host = ZombieVariantModel::new(ZombieVariantModelFamily::ZombieVillager, true);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                Some(HumanoidBabyArmorKind::Standard),
                atlas,
            );
        }
        EntityModelKind::Skeleton => {
            let mut host = SkeletonModel::new(None);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                atlas,
            );
        }
        EntityModelKind::SkeletonVariant { family } => {
            let transform = if matches!(family, SkeletonModelFamily::WitherSkeleton) {
                wither_skeleton_model_root_transform(instance)
            } else {
                entity_model_root_transform(instance)
            };
            let mut host = SkeletonModel::new(Some(family));
            host.prepare(&instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                atlas,
            );
        }
        EntityModelKind::Player { skin, .. } => {
            let slim = skin.is_slim();
            let mut host = PlayerModel::new(slim);
            host.prepare(&instance);
            let transform = player_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
                None,
                atlas,
            );
        }
        EntityModelKind::Piglin {
            family,
            baby: false,
        } => {
            // The piglin family (piglin, piglin brute, zombified piglin) wears the same base armor mesh
            // grown by the piglin `1.02` outer deformation (vanilla `AbstractPiglinModel.createArmorMeshSet`
            // = `PlayerModel.createArmorMeshSet(..).map(removeEars)`; the removed ears and the player's
            // empty sleeve/pants parts carry no geometry, so it is the standard mesh).
            let mut host = PiglinModel::new(family, false);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                PIGLIN_OUTER_ARMOR_DEFORMATION,
                None,
                atlas,
            );
        }
        EntityModelKind::Piglin { family, baby: true }
            if family != PiglinModelFamily::PiglinBrute =>
        {
            let mut host = PiglinModel::new(family, true);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                PIGLIN_OUTER_ARMOR_DEFORMATION,
                Some(HumanoidBabyArmorKind::Piglin),
                atlas,
            );
        }
        EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: true,
        } => {
            let mut host = PiglinModel::new(PiglinModelFamily::PiglinBrute, true);
            host.prepare(&instance);
            let transform = entity_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                PIGLIN_OUTER_ARMOR_DEFORMATION,
                None,
                atlas,
            );
        }
        _ => {}
    }
}

fn emit_custom_head_skull_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let Some(skull) = instance.render_state.custom_head_skull else {
        return;
    };
    let Some(transform) = custom_head_skull_transform(&instance) else {
        return;
    };
    match skull {
        EntityCustomHeadSkull::Dragon => {
            let mut model = CustomHeadDragonSkullModel::new();
            model.prepare(&instance);
            render_textured_pass(
                meshes,
                &model,
                transform,
                EntityModelLayerRenderType::EntityCutoutZOffset,
                custom_head_skull_texture_ref(skull),
                [1.0, 1.0, 1.0, 1.0],
                atlas,
            );
            return;
        }
        EntityCustomHeadSkull::Piglin => {
            let mut model = CustomHeadPiglinSkullModel::new();
            model.prepare(&instance);
            render_textured_pass(
                meshes,
                &model,
                transform,
                EntityModelLayerRenderType::EntityCutoutZOffset,
                custom_head_skull_texture_ref(skull),
                [1.0, 1.0, 1.0, 1.0],
                atlas,
            );
            return;
        }
        _ => {}
    }

    let mut model = CustomHeadSkullModel::new(matches!(skull, EntityCustomHeadSkull::Player(_)));
    model.prepare(&instance);
    let render_type = custom_head_skull_render_type(skull);
    let texture = custom_head_skull_texture_ref(skull);
    if let Some(dynamic_player_skin) = custom_head_dynamic_player_skin(skull) {
        render_textured_pass_with_dynamic_player_skin(
            meshes,
            &model,
            transform,
            render_type,
            texture,
            dynamic_player_skin,
            [1.0, 1.0, 1.0, 1.0],
            atlas,
            dynamic_player_skin_atlas,
        );
        return;
    }
    render_textured_pass(
        meshes,
        &model,
        transform,
        render_type,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
    );
}

fn custom_head_skull_render_type(skull: EntityCustomHeadSkull) -> EntityModelLayerRenderType {
    match skull {
        EntityCustomHeadSkull::Player(EntityPlayerSkin::ProfiledDefault(_))
        | EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(_)) => {
            EntityModelLayerRenderType::EntityTranslucent
        }
        _ => EntityModelLayerRenderType::EntityCutoutZOffset,
    }
}

fn custom_head_dynamic_player_skin(
    skull: EntityCustomHeadSkull,
) -> Option<EntityDynamicPlayerSkin> {
    match skull {
        EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(skin)) => Some(skin),
        _ => None,
    }
}

fn dynamic_player_skin_atlas_entry(
    atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    handle: u64,
) -> Option<EntityDynamicPlayerSkinAtlasEntry> {
    atlas?
        .entries
        .iter()
        .copied()
        .find(|entry| entry.handle == handle)
}

fn dynamic_player_texture_atlas_entry(
    atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    texture: EntityDynamicPlayerTexture,
) -> Option<EntityDynamicPlayerTextureAtlasEntry> {
    atlas?
        .entries
        .iter()
        .copied()
        .find(|entry| entry.handle == texture.handle)
}

fn custom_head_skull_texture_ref(skull: EntityCustomHeadSkull) -> EntityModelTextureRef {
    match skull {
        EntityCustomHeadSkull::Skeleton => SKELETON_TEXTURE_REF,
        EntityCustomHeadSkull::WitherSkeleton => WITHER_SKELETON_TEXTURE_REF,
        EntityCustomHeadSkull::Player(skin) => default_player_skin_texture_ref(skin.fallback()),
        EntityCustomHeadSkull::Zombie => ZOMBIE_TEXTURE_REF,
        EntityCustomHeadSkull::Creeper => CREEPER_TEXTURE_REF,
        EntityCustomHeadSkull::Dragon => ENDER_DRAGON_TEXTURE_REF,
        EntityCustomHeadSkull::Piglin => PIGLIN_TEXTURE_REF,
    }
}

/// Vanilla `PigRenderer` `SimpleEquipmentLayer(PIG_SADDLE)`: when the saddle slot contains the
/// saddle item, render an adult `PigModel.createBodyLayer(CubeDeformation(0.5F))` over the base pig
/// with the `pig_saddle/saddle.png` equipment texture. The vanilla layer has no baby model, so baby
/// pigs skip it even if the slot is filled.
fn emit_pig_saddle_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !instance.render_state.pig_saddle {
        return;
    }
    if !matches!(instance.kind, EntityModelKind::Pig { baby: false, .. }) {
        return;
    }

    let transform = entity_model_root_transform(instance);
    let mut model = PigModel::new_saddle();
    model.prepare(&instance);
    render_textured_pass_ordered(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        PIG_SADDLE_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        0,
        1,
        atlas,
    );
}

/// Vanilla `WolfArmorLayer`: adult wolves with a body armor item render the `WOLF_BODY` equipment
/// asset over `AdultWolfModel(ModelLayers.WOLF_ARMOR)`, baked with `CubeDeformation(0.2)`. The
/// armadillo-scute asset has a white base layer plus a dye-only overlay; damaged armor then adds an
/// `armorTranslucent` crack texture.
fn emit_wolf_body_armor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(material) = instance.render_state.wolf_body_armor else {
        return;
    };
    if !matches!(instance.kind, EntityModelKind::Wolf { baby: false, .. }) {
        return;
    }
    let Some(layers) = wolf_body_armor_texture_layers(material) else {
        return;
    };

    let transform = entity_model_root_transform(instance);
    let mut model = WolfModel::armor(wolf_is_angry(instance.kind));
    model.prepare(&instance);
    let mut submit_sequence = if wolf_has_collar(instance.kind) { 2 } else { 1 };
    for (layer_index, layer) in layers.iter().enumerate() {
        let Some(tint) =
            wolf_body_armor_layer_tint(layer.dyeable, instance.render_state.wolf_body_armor_dye)
        else {
            continue;
        };
        render_textured_pass_ordered(
            meshes,
            &model,
            transform,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            layer.texture,
            tint,
            1 + layer_index as i32,
            submit_sequence,
            atlas,
        );
        submit_sequence += 1;
    }

    if let Some(crackiness) = instance.render_state.wolf_body_armor_crackiness {
        render_textured_pass_ordered(
            meshes,
            &model,
            transform,
            EntityModelLayerRenderType::ArmorTranslucent,
            wolf_armor_crackiness_texture_ref(crackiness),
            [1.0, 1.0, 1.0, 1.0],
            3,
            submit_sequence,
            atlas,
        );
    }
}

fn wolf_body_armor_layer_tint(dyeable: bool, dye: Option<u32>) -> Option<[f32; 4]> {
    if dyeable {
        dye.map(opaque_wolf_armor_rgb_to_tint)
    } else {
        Some([1.0, 1.0, 1.0, 1.0])
    }
}

fn opaque_wolf_armor_rgb_to_tint(rgb: u32) -> [f32; 4] {
    [
        ((rgb >> 16) & 0xFF) as f32 / 255.0,
        ((rgb >> 8) & 0xFF) as f32 / 255.0,
        (rgb & 0xFF) as f32 / 255.0,
        1.0,
    ]
}

fn wolf_is_angry(kind: EntityModelKind) -> bool {
    matches!(kind, EntityModelKind::Wolf { angry: true, .. })
}

fn wolf_has_collar(kind: EntityModelKind) -> bool {
    matches!(
        kind,
        EntityModelKind::Wolf {
            tame: true,
            collar_color: Some(_),
            ..
        }
    )
}

/// Vanilla `SimpleEquipmentLayer` over `EquineSaddleModel` for horse, donkey, mule, skeleton-horse,
/// and zombie-horse saddles. The layer has no baby model, so baby equines skip it. The two bridle line
/// parts are visible only while `EquineRenderState.isRidden` is true.
fn emit_equine_saddle_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !instance.render_state.equine_saddle {
        return;
    }

    let ridden = instance.render_state.equine_saddle_ridden;
    let (parts, transform, texture, order, submit_sequence): (
        &[TexturedModelPartDesc],
        Mat4,
        EntityModelTextureRef,
        i32,
        u32,
    ) = match instance.kind {
        EntityModelKind::Horse { baby: false, .. } => {
            let body_layer_count = instance
                .render_state
                .equine_body_armor
                .and_then(horse_body_armor_texture_layers)
                .map(|layers| layers.len())
                .unwrap_or(0);
            (
                if ridden {
                    &ADULT_HORSE_SADDLE_RIDDEN_PARTS_TEXTURED
                } else {
                    &ADULT_HORSE_SADDLE_PARTS_TEXTURED
                },
                mesh_transformer_scaled_model_root_transform(instance, HORSE_SCALE),
                HORSE_SADDLE_TEXTURE_REF,
                2,
                2 + body_layer_count as u32,
            )
        }
        EntityModelKind::Donkey {
            family,
            baby: false,
            ..
        } => {
            let scale = match family {
                DonkeyModelFamily::Donkey => 0.87,
                DonkeyModelFamily::Mule => 0.92,
            };
            let texture = match family {
                DonkeyModelFamily::Donkey => DONKEY_SADDLE_TEXTURE_REF,
                DonkeyModelFamily::Mule => MULE_SADDLE_TEXTURE_REF,
            };
            (
                if ridden {
                    &ADULT_DONKEY_SADDLE_RIDDEN_PARTS_TEXTURED
                } else {
                    &ADULT_DONKEY_SADDLE_PARTS_TEXTURED
                },
                mesh_transformer_scaled_model_root_transform(instance, scale),
                texture,
                0,
                1,
            )
        }
        EntityModelKind::UndeadHorse {
            family,
            baby: false,
        } => {
            let texture = match family {
                UndeadHorseModelFamily::Skeleton => SKELETON_HORSE_SADDLE_TEXTURE_REF,
                UndeadHorseModelFamily::Zombie => ZOMBIE_HORSE_SADDLE_TEXTURE_REF,
            };
            let body_layer_count = match family {
                UndeadHorseModelFamily::Zombie => instance
                    .render_state
                    .equine_body_armor
                    .and_then(horse_body_armor_texture_layers)
                    .map(|layers| layers.len())
                    .unwrap_or(0),
                UndeadHorseModelFamily::Skeleton => 0,
            };
            (
                if ridden {
                    &ADULT_HORSE_SADDLE_RIDDEN_PARTS_TEXTURED
                } else {
                    &ADULT_HORSE_SADDLE_PARTS_TEXTURED
                },
                entity_model_root_transform(instance),
                texture,
                0,
                1 + body_layer_count as u32,
            )
        }
        _ => return,
    };

    emit_equine_textured_submission(
        meshes,
        parts,
        [2, 3, 4, 5],
        1,
        0.0,
        1.0,
        EntityModelSubmissionEmit::new(
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            texture,
            [1.0, 1.0, 1.0, 1.0],
            transform,
            order,
            submit_sequence,
        ),
        instance,
        atlas,
    );
}

/// Vanilla `HorseRenderer` / `UndeadHorseRenderer` `SimpleEquipmentLayer(HORSE_BODY)`: an adult horse
/// or zombie horse with a body armor item renders `HorseModel(ModelLayers.*_HORSE_ARMOR)`. The living
/// horse armor model inherits the 1.1 `livingHorseScale`; the zombie horse armor model is unscaled.
/// Vanilla supplies no baby model. Skeleton horses use the same renderer class but the vanilla
/// `CAN_WEAR_HORSE_ARMOR` tag excludes them, so the world projection never sets this layer for them.
fn emit_equine_body_armor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(material) = instance.render_state.equine_body_armor else {
        return;
    };
    let Some(layers) = horse_body_armor_texture_layers(material) else {
        return;
    };
    let (transform, order, first_submit_sequence) = match instance.kind {
        EntityModelKind::Horse { baby: false, .. } => (
            mesh_transformer_scaled_model_root_transform(instance, HORSE_SCALE),
            2,
            2,
        ),
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: false,
        } => (entity_model_root_transform(instance), 0, 1),
        _ => return,
    };

    for (layer_index, layer) in layers.iter().enumerate() {
        let tint = if layer.dyeable {
            armor_layer_tint(
                EntityArmorMaterial::Leather,
                instance.render_state.equine_body_armor_dye,
            )
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };
        emit_equine_textured_submission(
            meshes,
            &ADULT_HORSE_ARMOR_PARTS_TEXTURED,
            [2, 3, 4, 5],
            1,
            0.0,
            1.0,
            EntityModelSubmissionEmit::new(
                EntityModelLayerRenderType::ArmorCutoutNoCull,
                layer.texture,
                tint,
                transform,
                order + layer_index as i32,
                first_submit_sequence + layer_index as u32,
            ),
            instance,
            atlas,
        );
    }
}

/// Vanilla `StriderRenderer` `SimpleEquipmentLayer(STRIDER_SADDLE)`: a non-empty saddle item renders
/// `AdultStriderModel(ModelLayers.STRIDER_SADDLE)` with `strider_saddle/saddle.png`. The layer has no
/// baby model, so baby striders skip it.
fn emit_strider_saddle_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !instance.render_state.strider_saddle {
        return;
    }
    if !matches!(instance.kind, EntityModelKind::Strider { baby: false, .. }) {
        return;
    }

    let transform = entity_model_root_transform(instance);
    let mut model = StriderModel::new(false);
    model.prepare(&instance);
    render_textured_pass_ordered(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        STRIDER_SADDLE_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        0,
        1,
        atlas,
    );
}

/// Vanilla `CamelRenderer` / `CamelHuskRenderer` `SimpleEquipmentLayer`: a non-empty saddle item
/// renders `CamelSaddleModel(ModelLayers.CAMEL*_SADDLE)` with the family-specific equipment texture.
/// The layer has no baby model, so baby camels skip it; camel husks are adult-only and always use the
/// adult saddle model.
fn emit_camel_saddle_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !instance.render_state.camel_saddle {
        return;
    }

    let texture = match instance.kind {
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: false,
        } => CAMEL_SADDLE_TEXTURE_REF,
        EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            ..
        } => CAMEL_HUSK_SADDLE_TEXTURE_REF,
        _ => return,
    };

    let transform = entity_model_root_transform(instance);
    let mut model = CamelModel::new_saddle();
    model.prepare(&instance);
    render_textured_pass_ordered(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        0,
        1,
        atlas,
    );
}

/// Vanilla `NautilusRenderer` / `ZombieNautilusRenderer` `SimpleEquipmentLayer(NAUTILUS_SADDLE)`:
/// a non-empty saddle item renders `NautilusSaddleModel(ModelLayers.NAUTILUS_SADDLE)` over adult
/// living nautilus and zombie nautilus. The layer has no baby model, so baby living nautilus skip it.
fn emit_nautilus_saddle_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !instance.render_state.nautilus_saddle {
        return;
    }
    if !matches!(
        instance.kind,
        EntityModelKind::Nautilus { baby: false } | EntityModelKind::ZombieNautilus { .. }
    ) {
        return;
    }

    let transform = entity_model_root_transform(instance);
    let mut model = NautilusModel::new_saddle();
    model.prepare(&instance);
    let body_layer_count = instance
        .render_state
        .nautilus_body_armor
        .and_then(nautilus_body_armor_texture_ref)
        .map(|_| 1)
        .unwrap_or(0);
    render_textured_pass_ordered(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        NAUTILUS_SADDLE_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        0,
        1 + body_layer_count,
        atlas,
    );
}

/// Vanilla `NautilusRenderer` / `ZombieNautilusRenderer` `SimpleEquipmentLayer(NAUTILUS_BODY)`:
/// a non-empty nautilus body armor item renders `NautilusArmorModel(ModelLayers.NAUTILUS_ARMOR)` over
/// adult living nautilus and zombie nautilus. The layer has no baby model, so baby living nautilus
/// skip it.
fn emit_nautilus_body_armor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(material) = instance.render_state.nautilus_body_armor else {
        return;
    };
    if !matches!(
        instance.kind,
        EntityModelKind::Nautilus { baby: false } | EntityModelKind::ZombieNautilus { .. }
    ) {
        return;
    }
    let Some(texture) = nautilus_body_armor_texture_ref(material) else {
        return;
    };

    let transform = entity_model_root_transform(instance);
    let mut model = NautilusModel::new_armor();
    model.prepare(&instance);
    render_textured_pass_ordered(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        0,
        1,
        atlas,
    );
}

fn emit_squid_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    glow: bool,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let texture = squid_texture_ref(glow, baby);
    let transform = squid_model_root_transform(instance, baby);
    let mut model = SquidModel::new();
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::EntityCutout,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
    );
}

/// The textured llama base layer plus vanilla `LlamaDecorLayer`: adult carpet body equipment uses the
/// matching `LLAMA_BODY` equipment texture, otherwise trader llamas render their built-in adult/baby
/// trader overlay. The unified `LlamaModel` tree drives both render paths; `setup_anim` is the standard
/// `QuadrupedModel` head look plus the diagonal leg swing.
fn emit_llama_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: LlamaModelFamily,
    variant: LlamaVariant,
    baby: bool,
    has_chest: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    let mut model = LlamaModel::new(baby, has_chest);
    model.prepare(&instance);
    render_textured_layers(
        meshes,
        &model,
        transform,
        llama_textured_layer_passes(variant, baby, has_chest),
        atlas,
    );
    emit_llama_decor_layer(meshes, instance, family, baby, has_chest, transform, atlas);
}

fn emit_llama_decor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: LlamaModelFamily,
    baby: bool,
    has_chest: bool,
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let texture = match (baby, instance.render_state.llama_body_decor, family) {
        (false, Some(color), _) => llama_body_decor_texture_ref(color),
        (_, _, LlamaModelFamily::TraderLlama) if baby => LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
        (_, _, LlamaModelFamily::TraderLlama) => LLAMA_BODY_TRADER_TEXTURE_REF,
        _ => return,
    };

    let mut model = LlamaModel::new_decor(baby, has_chest);
    model.prepare(&instance);
    // Vanilla `EquipmentLayerRenderer.renderLayers(..., order = 1)` renders LLAMA_BODY with
    // `RenderTypes.armorCutoutNoCull`, even though the current backend folds it into the cutout mesh.
    render_textured_pass_ordered(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        1,
        1,
        atlas,
    );
}

fn emit_slime_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    size: i32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `SlimeModel` (inner body, cutout) and `SlimeOuterModel` (shell, translucent) trees
    // drive both render paths; both `setup_anim`s are no-ops (vanilla's squish stretch lives in the
    // renderer `scale`, applied by `slime_model_root_transform`, not in `setupAnim`). Each pass routes
    // to the inner or outer root in the pre-sorted layer order.
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
        render_textured_root_pass(meshes, root, transform, pass, atlas);
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
    render_textured_layers(
        meshes,
        &model,
        transform,
        husk_textured_layer_passes(baby),
        atlas,
    );
}

fn emit_drowned_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `ZombieVariantModel` tree drives the base body; `setup_anim` runs the shared
    // `ZombieModel.setupAnim` (head look + leg swing + held-out arms) plus the drowned trident throw.
    // `DrownedModel extends ZombieModel`, so the non-swimming drowned reuses the zombie body. The
    // always-on `DrownedOuterLayer` is a second white cutout pass driven by a `DrownedOuterModel`
    // (the inflated `createBodyLayer(0.25)` shell — the adult humanoid mesh or the distinct baby-zombie
    // mesh) posed by the SAME animator, so it tracks the limbs. `DrownedRenderer.setupRotations`
    // adds the swim body pitch onto both passes. No root scale.
    let transform = drowned_model_root_transform(instance);
    let mut base = ZombieVariantModel::new(ZombieVariantModelFamily::Drowned, baby);
    base.prepare(&instance);
    for pass in drowned_textured_layer_passes(baby) {
        if matches!(pass.kind, EntityModelLayerKind::DrownedOuter) {
            let mut outer = DrownedOuterModel::new(baby);
            outer.prepare(&instance);
            render_textured_root_pass(meshes, outer.root(), transform, pass, atlas);
        } else {
            render_textured_root_pass(meshes, base.root(), transform, pass, atlas);
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
    render_textured_layers(
        meshes,
        &model,
        transform,
        zombie_villager_textured_layer_passes(baby),
        atlas,
    );
}

const VILLAGER_NO_HAT_EXCLUDED_PARTS: [&str; 2] = ["hat", "hat_rim"];

fn emit_villager_profession_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    match instance.kind {
        EntityModelKind::Villager { baby } => {
            let transform = if baby {
                entity_model_root_transform(instance)
            } else {
                villager_adult_model_root_transform(instance)
            };
            let mut model = VillagerModel::new(baby);
            model.prepare(&instance);
            emit_villager_data_layers(
                meshes,
                &model,
                transform,
                baby,
                false,
                instance.render_state.villager_model_data,
                atlas,
            );
        }
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby,
        } => {
            let transform = entity_model_root_transform(instance);
            let mut model = ZombieVariantModel::new(ZombieVariantModelFamily::ZombieVillager, baby);
            model.prepare(&instance);
            emit_villager_data_layers(
                meshes,
                &model,
                transform,
                baby,
                true,
                instance.render_state.villager_model_data,
                atlas,
            );
        }
        _ => {}
    }
}

fn emit_villager_data_layers<M: EntityModel>(
    meshes: &mut EntityModelTexturedMeshes,
    model: &M,
    transform: Mat4,
    baby: bool,
    zombie: bool,
    data: VillagerModelData,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let type_texture = if zombie {
        zombie_villager_type_texture_ref(data.villager_type, baby)
    } else {
        villager_type_texture_ref(data.villager_type, baby)
    };
    emit_villager_profession_layer(
        meshes,
        model.root(),
        transform,
        type_texture,
        !villager_type_hat_visible(data, zombie),
        1,
        1,
        atlas,
    );

    if baby {
        return;
    }
    let profession_texture = if zombie {
        zombie_villager_profession_texture_ref(data.profession)
    } else {
        villager_profession_texture_ref(data.profession)
    };
    let Some(profession_texture) = profession_texture else {
        return;
    };
    emit_villager_profession_layer(
        meshes,
        model.root(),
        transform,
        profession_texture,
        false,
        2,
        2,
        atlas,
    );

    if data.profession.renders_level_badge() {
        let level_texture = if zombie {
            zombie_villager_level_texture_ref(data.level)
        } else {
            villager_level_texture_ref(data.level)
        };
        emit_villager_profession_layer(
            meshes,
            model.root(),
            transform,
            level_texture,
            false,
            3,
            3,
            atlas,
        );
    }
}

fn villager_type_hat_visible(data: VillagerModelData, zombie: bool) -> bool {
    let type_hat = if zombie {
        VillagerModelHat::None
    } else {
        data.villager_type.hat()
    };
    let profession_hat = data.profession.hat();
    profession_hat == VillagerModelHat::None
        || profession_hat == VillagerModelHat::Partial && type_hat != VillagerModelHat::Full
}

fn emit_villager_profession_layer(
    meshes: &mut EntityModelTexturedMeshes,
    root: &ModelPart,
    transform: Mat4,
    texture: EntityModelTextureRef,
    no_hat: bool,
    order: i32,
    submit_sequence: u32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let tint = [1.0, 1.0, 1.0, 1.0];
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::EntityCutout,
        texture,
        tint,
        transform,
        order,
        submit_sequence,
    );
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        if no_hat {
            root.render_textured_excluding(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
                "",
                &VILLAGER_NO_HAT_EXCLUDED_PARTS,
            );
        } else {
            root.render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        }
    });
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
    render_textured_layers(
        meshes,
        &model,
        transform,
        piglin_textured_layer_passes(family, baby_layout),
        atlas,
    );
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
    render_textured_layers(
        meshes,
        &model,
        transform,
        hoglin_textured_layer_passes(family, baby),
        atlas,
    );
}

fn emit_player_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    skin: EntityPlayerSkin,
    parts: PlayerModelPartVisibility,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    // The unified `PlayerModel` tree drives both render paths; `setup_anim` looks the head, runs the
    // inherited `HumanoidModel` walk swing + idle arm bob, and applies the crouch sneaking pose. The
    // six skin overlay parts (hat/jacket/sleeves/pants) are toggled by the player's part visibility
    // after `prepare` (the colored fallback shows every overlay). Held-item/attack/swim arm poses still
    // defer; the profile cape and WingsLayer emit as separate layers after the base body.
    let transform = player_model_root_transform(instance);
    let slim = skin.is_slim();
    let mut model = PlayerModel::new(slim);
    model.prepare(&instance);
    model.apply_part_visibility(parts);
    let texture = default_player_skin_texture_ref(skin.fallback());
    if let EntityPlayerSkin::Dynamic(dynamic_player_skin) = skin {
        render_textured_pass_with_dynamic_player_skin(
            meshes,
            &model,
            transform,
            EntityModelLayerRenderType::EntityCutout,
            texture,
            dynamic_player_skin,
            [1.0, 1.0, 1.0, 1.0],
            atlas,
            dynamic_player_skin_atlas,
        );
        return;
    }
    render_textured_layers(
        meshes,
        &model,
        transform,
        player_textured_layer_passes_with_texture(slim, parts, texture),
        atlas,
    );
}

fn emit_player_cape_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let EntityModelKind::Player { skin, parts } = instance.kind else {
        return;
    };
    if !parts.cape {
        return;
    }
    if instance.render_state.chest_equipment_has_wings {
        return;
    }
    let Some(cape_texture) = instance.render_state.player_cape_texture else {
        return;
    };
    let Some(entry) =
        dynamic_player_texture_atlas_entry(dynamic_player_texture_atlas, cape_texture)
    else {
        return;
    };

    let root = player_model_root_transform(instance);
    let layer_transform = root * player_cape_chest_equipment_transform(&instance);
    let mut model = PlayerModel::new(skin.is_slim());
    model.prepare(&instance);
    let Some(body_transform) = model.root().try_descendant_attach_transform(&["body"]) else {
        return;
    };
    let cape_transform = layer_transform
        * body_transform
        * part_pose_transform(PartPose {
            offset: [0.0, 0.0, 2.0],
            rotation: [0.0, std::f32::consts::PI, 0.0],
        })
        * player_cape_animation_transform(&instance);
    let tint = [1.0, 1.0, 1.0, 1.0];
    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::EntitySolid,
        PLAYER_PROFILE_CAPE_TEXTURE_REF,
        tint,
        layer_transform,
        0,
        1,
    )
    .with_dynamic_player_texture(cape_texture);
    render_textured_dynamic_player_texture_submission(meshes, submit, entry, |mesh, entry| {
        emit_textured_model_cube(
            mesh,
            cape_transform,
            PLAYER_CAPE_CUBE,
            PLAYER_PROFILE_CAPE_TEXTURE_REF,
            entry.uv,
            tint,
        );
    });
}

fn player_cape_chest_equipment_transform(instance: &EntityModelInstance) -> Mat4 {
    if instance.render_state.chest_equipment_has_humanoid {
        Mat4::from_translation(Vec3::new(0.0, -0.053125, 0.06875))
    } else {
        Mat4::IDENTITY
    }
}

fn player_cape_animation_transform(instance: &EntityModelInstance) -> Mat4 {
    let state = &instance.render_state;
    let rotation = Quat::from_rotation_y(-std::f32::consts::PI)
        * Quat::from_rotation_x(
            (6.0 + state.player_cape_lean / 2.0 + state.player_cape_flap).to_radians(),
        )
        * Quat::from_rotation_z((state.player_cape_lean2 / 2.0).to_radians())
        * Quat::from_rotation_y((180.0 - state.player_cape_lean2 / 2.0).to_radians());
    Mat4::from_quat(rotation)
}

fn emit_wings_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let Some(layer) = instance.render_state.chest_wings_layer else {
        return;
    };
    let Some((transform, baby)) = wings_layer_transform_and_baby(instance) else {
        return;
    };

    let mut model = ElytraModel::new(baby);
    model.prepare(&instance);
    let tint = [1.0, 1.0, 1.0, 1.0];

    if let Some(profile_texture) = player_wings_profile_texture(&instance, layer) {
        let Some(entry) =
            dynamic_player_texture_atlas_entry(dynamic_player_texture_atlas, profile_texture)
        else {
            return;
        };
        let submit = EntityModelSubmissionEmit::new(
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            player_profile_wings_texture_ref(profile_texture),
            tint,
            transform,
            0,
            2,
        )
        .with_dynamic_player_texture(profile_texture);
        render_textured_dynamic_player_texture_submission(meshes, submit, entry, |mesh, entry| {
            model.root().render_textured(
                mesh,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
        return;
    }

    let submit = EntityModelSubmissionEmit::new(
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        layer.texture,
        tint,
        transform,
        0,
        2,
    );
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.root().render_textured(
            mesh,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
        );
    });
}

fn wings_layer_transform_and_baby(instance: EntityModelInstance) -> Option<(Mat4, bool)> {
    let (root, baby) = match instance.kind {
        EntityModelKind::Player { .. } => (player_model_root_transform(instance), false),
        EntityModelKind::Zombie { baby } => (entity_model_root_transform(instance), baby),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby,
        } => {
            let transform = if baby {
                entity_model_root_transform(instance)
            } else {
                mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE)
            };
            (transform, baby)
        }
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby,
        } => (drowned_model_root_transform(instance), baby),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby,
        } => (entity_model_root_transform(instance), baby),
        EntityModelKind::Skeleton => (entity_model_root_transform(instance), false),
        EntityModelKind::SkeletonVariant { family } => {
            let transform = if matches!(family, SkeletonModelFamily::WitherSkeleton) {
                wither_skeleton_model_root_transform(instance)
            } else {
                entity_model_root_transform(instance)
            };
            (transform, false)
        }
        EntityModelKind::Piglin { baby, .. } => (entity_model_root_transform(instance), baby),
        EntityModelKind::ArmorStand { small, .. } => (entity_model_root_transform(instance), small),
        _ => return None,
    };
    Some((root * Mat4::from_translation(Vec3::Z * 0.125), baby))
}

fn player_wings_profile_texture(
    instance: &EntityModelInstance,
    layer: EntityEquipmentLayerTexture,
) -> Option<EntityDynamicPlayerTexture> {
    if !layer.use_player_texture {
        return None;
    }
    let EntityModelKind::Player { parts, .. } = instance.kind else {
        return None;
    };
    instance.render_state.player_elytra_texture.or_else(|| {
        parts
            .cape
            .then_some(instance.render_state.player_cape_texture)
            .flatten()
    })
}

fn player_profile_wings_texture_ref(texture: EntityDynamicPlayerTexture) -> EntityModelTextureRef {
    match texture.kind {
        super::catalog::EntityDynamicPlayerTextureKind::Cape => PLAYER_PROFILE_CAPE_TEXTURE_REF,
        super::catalog::EntityDynamicPlayerTextureKind::Elytra => PLAYER_PROFILE_ELYTRA_TEXTURE_REF,
    }
}

fn emit_sheep_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
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
    for pass in sheep_textured_layer_passes(baby, sheared, wool_color, jeb, age_ticks) {
        let root = if pass.kind == layers::EntityModelLayerKind::SheepWool {
            fur.root()
        } else {
            body.root()
        };
        render_textured_root_pass(meshes, root, transform, pass, atlas);
    }
}

/// The body part index in every equine layer, and the tail's child index under the body. The body is
/// always first; the tail is its first child. (Single source of truth lives in `colored::mounts`; these
/// mirror it for the textured path and are pinned identical by the textured-vs-colored rest test.)
const EQUINE_BODY_PART_INDEX: usize = 0;
const EQUINE_TAIL_CHILD_INDEX: usize = 0;

/// Textured counterpart of `colored::mounts::emit_equine_posed`: applies the vanilla
/// `AbstractEquineModel.setupAnim` default-branch poses — the walking leg swing on the four parts at
/// `leg_indices`, the head look/bob on the `head_parts` (neck) at `head_parts_index`, and the tail walk
/// lift (`tail_x_rot_offset` = `getTailXRotOffset()`, `age_scale` = `getAgeScale()`) on the body's tail
/// child — to a [`TexturedModelPartDesc`] tree, emitting into `mesh` against one `texture`/`uv_rect`/
/// `tint`. The static tree is walked unchanged only when the gait, head look, and tail are all at rest;
/// otherwise the body subtree is hand-emitted so the `&'static` tail child can take the swung pose. The
/// pose math is shared with the colored path (the `equine_*_pose` helpers are geometry-agnostic), so the
/// two paths stay in lockstep.
#[allow(clippy::too_many_arguments)]
fn emit_equine_textured_submission(
    meshes: &mut EntityModelTexturedMeshes,
    parts: &[TexturedModelPartDesc],
    leg_indices: [usize; 4],
    head_parts_index: usize,
    tail_x_rot_offset: f32,
    age_scale: f32,
    submit: EntityModelSubmissionEmit,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        emit_equine_textured_posed(
            mesh,
            parts,
            leg_indices,
            head_parts_index,
            tail_x_rot_offset,
            age_scale,
            submit.transform,
            submit.texture,
            entry.uv,
            submit.tint,
            instance,
        );
    });
}

#[allow(clippy::too_many_arguments)]
fn emit_equine_textured_posed(
    mesh: &mut EntityModelTexturedMesh,
    parts: &[TexturedModelPartDesc],
    leg_indices: [usize; 4],
    head_parts_index: usize,
    tail_x_rot_offset: f32,
    age_scale: f32,
    transform: Mat4,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
    instance: EntityModelInstance,
) {
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let in_water = instance.render_state.in_water;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let legs_resting = limb_swing_at_rest(limb_swing_amount);

    let tail_rest = parts[EQUINE_BODY_PART_INDEX].children[EQUINE_TAIL_CHILD_INDEX].pose;
    let posed_tail =
        equine_tail_swing_pose(tail_rest, tail_x_rot_offset, limb_swing_amount, age_scale);
    let tail_resting = posed_tail == tail_rest;

    if legs_resting && head_look_at_rest(head_yaw, head_pitch) && tail_resting {
        emit_textured_model_parts(mesh, parts, transform, texture, uv_rect, tint);
        return;
    }

    let mut posed = parts.to_vec();
    if !legs_resting {
        for index in leg_indices {
            posed[index].pose =
                equine_leg_swing_pose(posed[index].pose, limb_swing, limb_swing_amount, in_water);
        }
    }
    posed[head_parts_index].pose = equine_head_look_pose(
        posed[head_parts_index].pose,
        head_yaw,
        head_pitch,
        limb_swing,
        limb_swing_amount,
    );

    // Hand-emit the body subtree so the tail (a `&'static` child) can take the swung pose, then the
    // remaining parts (neck + legs) in depth-first order via the `[1..]` slice.
    let body = &posed[EQUINE_BODY_PART_INDEX];
    let body_transform = transform * part_pose_transform(body.pose);
    let mut body_children = body.children.to_vec();
    body_children[EQUINE_TAIL_CHILD_INDEX].pose = posed_tail;
    for &cube in body.cubes {
        emit_textured_model_cube(mesh, body_transform, cube, texture, uv_rect, tint);
    }
    emit_textured_model_parts(mesh, &body_children, body_transform, texture, uv_rect, tint);
    emit_textured_model_parts(
        mesh,
        &posed[EQUINE_BODY_PART_INDEX + 1..],
        transform,
        texture,
        uv_rect,
        tint,
    );
}

/// The textured donkey / mule base layer. Vanilla `DonkeyModel` is the shared
/// `AbstractEquineModel.createBodyMesh` with `modifyMesh` (bigger ears replacing the horse ears, plus the
/// two side chest boxes shown when `hasChest`), on the 64×64 `donkey.png` / `mule.png` at the
/// `DonkeyModel.DONKEY_SCALE` 0.87 / `MULE_SCALE` 0.92 mesh-transformer scale. The ADULT takes the same
/// equine leg swing / head look/bob / tail walk lift as the horse (`AbstractEquineModel.setupAnim`), so
/// it rides `emit_equine_textured_posed`. The BABY is the distinct re-parented `BabyDonkeyModel` mesh
/// (legs/head/tail nested under the body) whose `setupAnim` forces `xRot = -30°`, so it emits STATIC
/// (unscaled, no equine posing — matching the colored baby path); its empty chest children make
/// `hasChest` immaterial.
fn emit_donkey_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: DonkeyModelFamily,
    baby: bool,
    has_chest: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(texture) = instance.kind.vanilla_texture_ref() else {
        return;
    };
    if baby {
        let submit = EntityModelSubmissionEmit::new(
            EntityModelLayerRenderType::EntityCutout,
            texture,
            [1.0, 1.0, 1.0, 1.0],
            entity_model_root_transform(instance),
            0,
            0,
        );
        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            emit_textured_model_parts(
                mesh,
                &BABY_DONKEY_PARTS_TEXTURED,
                submit.transform,
                submit.texture,
                entry.uv,
                submit.tint,
            );
        });
        return;
    }
    let parts: &[TexturedModelPartDesc] = if has_chest {
        &ADULT_DONKEY_PARTS_WITH_CHEST_TEXTURED
    } else {
        &ADULT_DONKEY_PARTS_TEXTURED
    };
    // `DonkeyModel.DONKEY_SCALE` / `MULE_SCALE` mesh-transformer scaling (mirrors the colored
    // `donkey_model_scale`).
    let scale = match family {
        DonkeyModelFamily::Donkey => 0.87,
        DonkeyModelFamily::Mule => 0.92,
    };
    emit_equine_textured_submission(
        meshes,
        parts,
        [2, 3, 4, 5],
        1,
        0.0,
        1.0,
        EntityModelSubmissionEmit::new(
            EntityModelLayerRenderType::EntityCutout,
            texture,
            [1.0, 1.0, 1.0, 1.0],
            mesh_transformer_scaled_model_root_transform(instance, scale),
            0,
            0,
        ),
        instance,
        atlas,
    );
}

/// The textured living horse base layer plus the `HorseMarkingLayer` overlay. Vanilla `HorseRenderer`
/// renders `HorseModel` with a per-coat `horse_<color>(_baby).png` base texture, then layers the white
/// markings (`horse_markings_*(_baby).png`, `entityTranslucent`, `order(1)`) on top when the coat has
/// markings. The adult body carries the `livingHorseScale` 1.1 mesh-transformer scale (`emit_horse_model`'s
/// transform); the baby uses the unscaled re-parented layer. The leg swing / head look/bob / tail walk
/// lift are the shared `AbstractEquineModel.setupAnim` default-branch poses (the same as the undead
/// horse), driven on the textured path here. The variant chooses the base coat, the markings the overlay;
/// both ride the same `HorseModel` pose, so the overlay tracks the body for free.
fn emit_horse_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    markings: HorseMarkings,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(texture) = instance.kind.vanilla_texture_ref() else {
        return;
    };
    let (parts, leg_indices, head_parts_index, tail_x_rot_offset, age_scale, transform): (
        &[TexturedModelPartDesc],
        [usize; 4],
        usize,
        f32,
        f32,
        Mat4,
    ) = if baby {
        (
            &BABY_HORSE_PARTS_TEXTURED,
            [1, 2, 3, 4],
            5,
            -std::f32::consts::FRAC_PI_2,
            0.5,
            entity_model_root_transform(instance),
        )
    } else {
        (
            &ADULT_HORSE_PARTS_TEXTURED,
            [2, 3, 4, 5],
            1,
            0.0,
            1.0,
            mesh_transformer_scaled_model_root_transform(instance, HORSE_SCALE),
        )
    };
    emit_equine_textured_submission(
        meshes,
        parts,
        leg_indices,
        head_parts_index,
        tail_x_rot_offset,
        age_scale,
        EntityModelSubmissionEmit::new(
            EntityModelLayerRenderType::EntityCutout,
            texture,
            [1.0, 1.0, 1.0, 1.0],
            transform,
            0,
            0,
        ),
        instance,
        atlas,
    );
    // `HorseMarkingLayer`: a translucent white overlay of the SAME posed model, drawn after the base
    // when the coat carries markings (`Markings.NONE` → `INVISIBLE_TEXTURE`, skipped). It rides the
    // identical pose, so re-emitting the same tree into the translucent mesh tracks the body.
    if let Some(markings_texture) = horse_markings_texture_ref(markings, baby) {
        emit_equine_textured_submission(
            meshes,
            parts,
            leg_indices,
            head_parts_index,
            tail_x_rot_offset,
            age_scale,
            EntityModelSubmissionEmit::new(
                EntityModelLayerRenderType::EntityTranslucent,
                markings_texture,
                [1.0, 1.0, 1.0, 1.0],
                transform,
                1,
                1,
            ),
            instance,
            atlas,
        );
    }
}

/// The textured skeleton / zombie horse base layer. Vanilla `UndeadHorseRenderer extends
/// HorseRenderer`, so the undead horses reuse `HorseModel`; the textured body takes the same equine leg
/// swing, head look/bob, and tail walk lift as the colored fallback ([`emit_undead_horse_model`]). Only
/// the texture differs — the tint is white (the `horse_skeleton` / `horse_zombie` texture, not a per-cube
/// color, carries the look). The adult layer uses `HorseModel.createBodyLayer` (legs `[2, 3, 4, 5]`,
/// neck `1`, `getTailXRotOffset = 0`, `ageScale = 1`); the baby uses `BabyHorseModel.createBabyLayer`,
/// which re-parents the parts (legs `[1, 2, 3, 4]`, neck `5`) and overrides `getTailXRotOffset = −π/2`,
/// `ageScale = 0.5`. The ridden/eat/stand poses and the tail's `ageInTicks` yRot wag are deferred.
fn emit_undead_horse_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(texture) = instance.kind.vanilla_texture_ref() else {
        return;
    };
    let (parts, leg_indices, head_parts_index, tail_x_rot_offset, age_scale): (
        &[TexturedModelPartDesc],
        [usize; 4],
        usize,
        f32,
        f32,
    ) = if baby {
        (
            &BABY_HORSE_PARTS_TEXTURED,
            [1, 2, 3, 4],
            5,
            -std::f32::consts::FRAC_PI_2,
            0.5,
        )
    } else {
        (&ADULT_HORSE_PARTS_TEXTURED, [2, 3, 4, 5], 1, 0.0, 1.0)
    };
    emit_equine_textured_submission(
        meshes,
        parts,
        leg_indices,
        head_parts_index,
        tail_x_rot_offset,
        age_scale,
        EntityModelSubmissionEmit::new(
            EntityModelLayerRenderType::EntityCutout,
            texture,
            [1.0, 1.0, 1.0, 1.0],
            entity_model_root_transform(instance),
            0,
            0,
        ),
        instance,
        atlas,
    );
}

fn emit_skeleton_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: Option<SkeletonModelFamily>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `SkeletonModel` tree (selected by family) drives both render paths; `setup_anim` runs
    // the shared humanoid head look + arm/leg walk swing. The base body draws in the cutout pass; the
    // stray frost / bogged mushroom overlay is a second cutout pass driven by a textured-only
    // `SkeletonClothingModel` posed by the SAME animator, so it tracks the limbs.
    let transform = if matches!(family, Some(SkeletonModelFamily::WitherSkeleton)) {
        wither_skeleton_model_root_transform(instance)
    } else {
        entity_model_root_transform(instance)
    };
    let mut base = SkeletonModel::new(family);
    base.prepare(&instance);
    for pass in skeleton_textured_layer_passes(family) {
        if matches!(pass.kind, EntityModelLayerKind::SkeletonClothing) {
            let mut clothing = SkeletonClothingModel::new(family);
            clothing.prepare(&instance);
            render_textured_root_pass(meshes, clothing.root(), transform, pass, atlas);
        } else {
            render_textured_root_pass(meshes, base.root(), transform, pass, atlas);
        }
    }
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
