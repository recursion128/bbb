use super::model::EntityModel;
use super::{
    boat_model_root_transform,
    catalog::squid_texture_ref,
    catalog::{
        ArmorStandModelPose, BoatModelFamily, CamelModelFamily, ChickenModelVariant,
        CowModelVariant, EntityDyeColor, EntityModelKind, EntityModelTextureAtlasEntry,
        EntityModelTextureAtlasLayout, EntityModelTextureRef, HoglinModelFamily,
        IllagerModelFamily, LlamaVariant, PigModelVariant, PiglinModelFamily,
        PlayerModelPartVisibility, SalmonModelSize, SheepWoolColor, SkeletonModelFamily,
        TropicalFishModelShape, TropicalFishPattern, ZombieVariantModelFamily,
    },
    cave_spider_model_root_transform, cod_model_root_transform, creeper_model_root_transform,
    entity_model_root_transform,
    geometry::{
        emit_textured_model_parts, fill_entity_textured_light, fill_entity_textured_overlay,
        part_pose_transform, EntityModelTexturedMesh, TexturedModelPartDesc,
    },
    ghast_model_root_transform, happy_ghast_model_root_transform,
    instances::EntityModelInstance,
    magma_cube_model_root_transform, mesh_transformer_scaled_model_root_transform,
    model_layers::{
        apply_wolf_sitting_pose, head_first_part_index, head_look_at_rest, head_look_pose,
        humanoid_arm_bob_pose, humanoid_arm_swing_pose, humanoid_leg_swing_pose,
        limb_swing_at_rest, parched_head_part_index, quadruped_leg_swing_pose,
        skeleton_head_part_index, wolf_angry_tail_pose, wolf_sitting_part_roles,
        wolf_tail_part_index, wolf_tail_swing_pose, AllayModel, ArmorStandModel, BatModel,
        BeeModel, BlazeModel, BreezeModel, CamelModel, ChickenModel, CodModel, CowModel,
        CreeperModel, DolphinModel, EndermanModel, EndermiteModel, GhastModel, GoatModel,
        HappyGhastModel, HoglinModel, IllagerModel, IronGolemModel, LlamaModel, MagmaCubeModel,
        MinecartModel, PhantomModel, PigModel, PiglinModel, PlayerModel, PolarBearModel,
        PufferfishModel, RavagerModel, SalmonModel, SheepFurModel, SheepModel, SilverfishModel,
        SkeletonModel, SlimeModel, SlimeOuterModel, SnowGolemModel, SpiderModel, SquidModel,
        StriderModel, TropicalFishModel, TropicalFishPatternModel, TurtleModel, VexModel,
        VillagerModel, WanderingTraderModel, WitchModel, ZombieModel, ZombieVariantModel,
        ALLAY_TEXTURE_REF, ARMOR_STAND_TEXTURE_REF, BAT_TEXTURE_REF, BEE_BABY_TEXTURE_REF,
        BEE_TEXTURE_REF, BREEZE_TEXTURE_REF, COD_TEXTURE_REF, DOLPHIN_BABY_TEXTURE_REF,
        DOLPHIN_TEXTURE_REF, PUFFERFISH_TEXTURE_REF, STRIDER_BABY_TEXTURE_REF, STRIDER_TEXTURE_REF,
        TURTLE_BABY_TEXTURE_REF, TURTLE_EGG_ROOT_DROP_POSE, TURTLE_TEXTURE_REF, VEX_TEXTURE_REF,
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
    // The unified `CamelModel` tree drives both render paths; `new` selects the adult / baby / husk mesh
    // and walk, and `setup_anim` clamps the head look and samples the walk (`root` roll, leg / ear / tail
    // swing, `head` pitch added onto the look, baby `body` dip). The camel is a single cutout pass; the
    // family / baby texture comes from the pass.
    let transform = entity_model_root_transform(instance);
    let mut model = CamelModel::new(family, baby);
    model.prepare(&instance);
    for pass in camel_textured_layer_passes(family, baby) {
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

/// The textured strider base layer. The unified [`StriderModel`] tree (selected by `baby`) runs the
/// shared `StriderModel.setupAnim` (legs swing/roll/lift, body sway/bob/look, bristle flow) and draws
/// into the cutout mesh. The ridden pose, the saddle layer, and the cold/suffocating texture are
/// deferred entity-side state.
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
    let transform = entity_model_root_transform(instance);
    let mut model = StriderModel::new(baby);
    model.prepare(&instance);
    model.root().render_textured(
        meshes.mesh_mut(EntityModelLayerRenderType::Cutout),
        transform,
        texture,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
    );
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
    // The unified `TurtleModel` tree drives both render paths; `setup_anim` tracks the head look,
    // swings the legs, and shows the adult `egg_belly` overlay when `hasEgg`. The `root.y--` egg drop
    // lives in the root transform. The base layer draws into the cutout mesh.
    let texture = if baby {
        TURTLE_BABY_TEXTURE_REF
    } else {
        TURTLE_TEXTURE_REF
    };
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
        return;
    };
    let has_egg = !baby && instance.render_state.turtle_has_egg;
    let mut transform = entity_model_root_transform(instance);
    if has_egg {
        transform *= part_pose_transform(TURTLE_EGG_ROOT_DROP_POSE);
    }
    let mut model = TurtleModel::new(baby);
    model.prepare(&instance);
    model.root().render_textured(
        meshes.mesh_mut(EntityModelLayerRenderType::Cutout),
        transform,
        texture,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
    );
}

fn emit_bat_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `BatModel` tree drives both render paths; `setup_anim` samples the looping
    // `BatAnimation.BAT_FLYING` (or the `BAT_RESTING` hanging pose while `isResting`) and turns the
    // resting head by the look yaw. The base layer draws into the cutout mesh (vanilla
    // `RenderTypes::entityCutoutCull`).
    let Some(entry) = entity_model_texture_atlas_entry(atlas, BAT_TEXTURE_REF) else {
        return;
    };
    let transform = entity_model_root_transform(instance);
    let mut model = BatModel::new();
    model.prepare(&instance);
    model.root().render_textured(
        meshes.mesh_mut(EntityModelLayerRenderType::Cutout),
        transform,
        BAT_TEXTURE_REF,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
    );
}

fn emit_bee_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `BeeModel` tree drives both render paths; `setup_anim` flaps the wings, rocks the
    // non-angry bob, splays the legs to `π/4` while airborne, and hides the stinger once stung. The
    // textured base layer draws into the cutout mesh (vanilla `RenderTypes::entityCutoutCull`); the
    // baby uses a distinct texture.
    let texture = if baby {
        BEE_BABY_TEXTURE_REF
    } else {
        BEE_TEXTURE_REF
    };
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
        return;
    };
    let transform = entity_model_root_transform(instance);
    let mut model = BeeModel::new(baby);
    model.prepare(&instance);
    model.root().render_textured(
        meshes.mesh_mut(EntityModelLayerRenderType::Cutout),
        transform,
        texture,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
    );
}

fn emit_breeze_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `BreezeModel` tree drives both render paths; `setup_anim` samples the looping
    // `BreezeAnimation.IDLE`. The base body draws into the translucent mesh (vanilla `BreezeModel`
    // uses `RenderTypes::entityTranslucent`).
    let Some(entry) = entity_model_texture_atlas_entry(atlas, BREEZE_TEXTURE_REF) else {
        return;
    };
    let transform = entity_model_root_transform(instance);
    let mut model = BreezeModel::new();
    model.prepare(&instance);
    model.root().render_textured(
        meshes.mesh_mut(EntityModelLayerRenderType::Translucent),
        transform,
        BREEZE_TEXTURE_REF,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
    );
}

fn emit_dolphin_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `DolphinModel` tree drives both render paths; `setup_anim` steers the body and waves
    // the tail. The base body draws into the cutout mesh (the `DolphinModel` default
    // `RenderTypes::entityCutoutNoCull`). The baby uses the `MeshTransformer.scaling(0.5)` layer and a
    // distinct texture.
    let texture = if baby {
        DOLPHIN_BABY_TEXTURE_REF
    } else {
        DOLPHIN_TEXTURE_REF
    };
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
        return;
    };
    let transform =
        mesh_transformer_scaled_model_root_transform(instance, if baby { 0.5 } else { 1.0 });
    let mut model = DolphinModel::new();
    model.prepare(&instance);
    model.root().render_textured(
        meshes.mesh_mut(EntityModelLayerRenderType::Cutout),
        transform,
        texture,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
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
    // The unified `ArmorStandModel` tree drives both render paths; `new` selects the small / full layer
    // and `setup_anim` poses each part from the synced pose (degrees), hides the arms / base plate by
    // visibility, and yaws the base plate by `-bodyRot`. Draws into the cutout mesh.
    let Some(entry) = entity_model_texture_atlas_entry(atlas, ARMOR_STAND_TEXTURE_REF) else {
        return;
    };
    let transform = entity_model_root_transform(instance);
    let mut model = ArmorStandModel::new(small, show_arms, show_base_plate, pose);
    model.prepare(&instance);
    model.root().render_textured(
        meshes.mesh_mut(EntityModelLayerRenderType::Cutout),
        transform,
        ARMOR_STAND_TEXTURE_REF,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
    );
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
    // The unified `PufferfishModel` tree drives both render paths; `new` picks the small/mid/big parts
    // by puff state and `setup_anim` wiggles its two fins on `ageInTicks`. A single cutout pass over
    // `pufferfish.png` (no eyes layer).
    let Some(entry) = entity_model_texture_atlas_entry(atlas, PUFFERFISH_TEXTURE_REF) else {
        return;
    };
    let transform = pufferfish_model_root_transform(instance);
    let mut model = PufferfishModel::new(puff_state);
    model.prepare(&instance);
    model.root().render_textured(
        meshes.mesh_mut(EntityModelLayerRenderType::Cutout),
        transform,
        PUFFERFISH_TEXTURE_REF,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
    );
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
