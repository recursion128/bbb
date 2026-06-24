use super::dispatch::{dispatch_uniform_entity_model, TexturedSink};
use super::model::EntityModel;
use super::{
    catalog::squid_texture_ref,
    catalog::{
        ArmorStandModelPose, CamelModelFamily, EntityDyeColor, EntityModelKind,
        EntityModelTextureAtlasEntry, EntityModelTextureAtlasLayout, EntityModelTextureRef,
        HoglinModelFamily, LlamaVariant, PiglinModelFamily, PlayerModelPartVisibility,
        SheepWoolColor, SkeletonModelFamily, TropicalFishModelShape, TropicalFishPattern,
        ZombieVariantModelFamily,
    },
    cod_model_root_transform, entity_model_root_transform,
    geometry::{
        fill_entity_textured_light, fill_entity_textured_overlay, part_pose_transform,
        EntityModelTexturedMesh,
    },
    instances::EntityModelInstance,
    mesh_transformer_scaled_model_root_transform,
    model_layers::{
        AllayModel, ArmorStandModel, BatModel, BeeModel, BreezeModel, CamelModel, CodModel,
        DolphinModel, HoglinModel, LlamaModel, PiglinModel, PlayerModel, PufferfishModel,
        SheepFurModel, SheepModel, SkeletonClothingModel, SkeletonModel, SlimeModel,
        SlimeOuterModel, SquidModel, StriderModel, TropicalFishModel, TropicalFishPatternModel,
        TurtleModel, VexModel, ZombieVariantModel, ALLAY_TEXTURE_REF, ARMOR_STAND_TEXTURE_REF,
        BAT_TEXTURE_REF, BEE_BABY_TEXTURE_REF, BEE_TEXTURE_REF, BREEZE_TEXTURE_REF,
        COD_TEXTURE_REF, DOLPHIN_BABY_TEXTURE_REF, DOLPHIN_TEXTURE_REF, PUFFERFISH_TEXTURE_REF,
        STRIDER_BABY_TEXTURE_REF, STRIDER_TEXTURE_REF, TURTLE_BABY_TEXTURE_REF,
        TURTLE_EGG_ROOT_DROP_POSE, TURTLE_TEXTURE_REF, VEX_TEXTURE_REF,
    },
    player_model_root_transform, pufferfish_model_root_transform, slime_model_root_transform,
    squid_model_root_transform, tropical_fish_model_root_transform,
    wither_skeleton_model_root_transform, HUSK_SCALE,
};
use glam::Mat4;

mod layers;
#[cfg(test)]
pub(super) use layers::EntityModelLayerVisibility;
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
    zombie_villager_textured_layer_passes, EntityModelLayerKind, EntityModelLayerPass,
    EntityModelLayerRenderType,
};

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
        if instance.render_state.invisible {
            continue;
        }
        let cutout_start = meshes.cutout.vertices.len();
        let translucent_start = meshes.translucent.vertices.len();
        let eyes_start = meshes.eyes.vertices.len();
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
                EntityModelKind::Llama {
                    variant,
                    baby,
                    has_chest,
                    ..
                } => {
                    emit_llama_textured_model(
                        &mut meshes,
                        *instance,
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
                EntityModelKind::Cod => {
                    emit_cod_textured_model(&mut meshes, *instance, atlas);
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
                EntityModelKind::Slime { size } => {
                    emit_slime_textured_model(&mut meshes, *instance, size, atlas);
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
                EntityModelKind::Pufferfish { puff_state } => {
                    emit_pufferfish_textured_model(&mut meshes, *instance, puff_state, atlas);
                }
                EntityModelKind::Hoglin { family, baby } => {
                    emit_hoglin_textured_model(&mut meshes, *instance, family, baby, atlas);
                }
                EntityModelKind::Player { slim, parts } => {
                    emit_player_textured_model(&mut meshes, *instance, slim, parts, atlas);
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
                _ => {}
            }
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
    if let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) {
        model.root().render_textured(
            meshes.mesh_mut(render_type),
            transform,
            texture,
            entry.uv,
            tint,
        );
    }
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
        render_textured_pass(
            meshes,
            model,
            transform,
            pass.render_type,
            pass.texture,
            pass.tint,
            atlas,
        );
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
    render_textured_layers(
        meshes,
        &model,
        transform,
        camel_textured_layer_passes(family, baby),
        atlas,
    );
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
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Cutout,
        COD_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
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
    let transform = squid_model_root_transform(instance, baby);
    let mut model = SquidModel::new();
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Cutout,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
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
    let transform = entity_model_root_transform(instance);
    let mut model = VexModel::new();
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Translucent,
        VEX_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
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
    let transform = entity_model_root_transform(instance);
    let mut model = AllayModel::new();
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Translucent,
        ALLAY_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
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
    let transform = entity_model_root_transform(instance);
    let mut model = StriderModel::new(baby);
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Cutout,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
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
    let has_egg = !baby && instance.render_state.turtle_has_egg;
    let mut transform = entity_model_root_transform(instance);
    if has_egg {
        transform *= part_pose_transform(TURTLE_EGG_ROOT_DROP_POSE);
    }
    let mut model = TurtleModel::new(baby);
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Cutout,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
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
    let transform = entity_model_root_transform(instance);
    let mut model = BatModel::new();
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Cutout,
        BAT_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
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
    let transform = entity_model_root_transform(instance);
    let mut model = BeeModel::new(baby);
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Cutout,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
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
    let transform = entity_model_root_transform(instance);
    let mut model = BreezeModel::new();
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Translucent,
        BREEZE_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
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
    let transform =
        mesh_transformer_scaled_model_root_transform(instance, if baby { 0.5 } else { 1.0 });
    let mut model = DolphinModel::new();
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Cutout,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
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
    render_textured_layers(
        meshes,
        &model,
        transform,
        llama_textured_layer_passes(variant, baby, has_chest),
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
    let transform = entity_model_root_transform(instance);
    let mut model = ArmorStandModel::new(small, show_arms, show_base_plate, pose);
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Cutout,
        ARMOR_STAND_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
        atlas,
    );
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
    // The unified `ZombieVariantModel` tree drives both render paths; `setup_anim` runs the shared
    // `ZombieModel.setupAnim` (head look + leg swing + held-out arms). `DrownedModel extends
    // ZombieModel`, so the non-swimming drowned reuses the zombie body. The `DrownedOuterLayer`, the
    // swim re-pose (needs `swimAmount`), and the trident throw arm pose all stay deferred. No root scale.
    let transform = entity_model_root_transform(instance);
    let mut model = ZombieVariantModel::new(ZombieVariantModelFamily::Drowned, baby);
    model.prepare(&instance);
    render_textured_layers(
        meshes,
        &model,
        transform,
        drowned_textured_layer_passes(baby),
        atlas,
    );
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

fn emit_pufferfish_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    puff_state: i32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    // The unified `PufferfishModel` tree drives both render paths; `new` picks the small/mid/big parts
    // by puff state and `setup_anim` wiggles its two fins on `ageInTicks`. A single cutout pass over
    // `pufferfish.png` (no eyes layer).
    let transform = pufferfish_model_root_transform(instance);
    let mut model = PufferfishModel::new(puff_state);
    model.prepare(&instance);
    render_textured_pass(
        meshes,
        &model,
        transform,
        EntityModelLayerRenderType::Cutout,
        PUFFERFISH_TEXTURE_REF,
        [1.0, 1.0, 1.0, 1.0],
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
    render_textured_layers(
        meshes,
        &model,
        transform,
        player_textured_layer_passes(slim, parts),
        atlas,
    );
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
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        let mesh = meshes.mesh_mut(pass.render_type);
        if matches!(pass.kind, EntityModelLayerKind::SkeletonClothing) {
            let mut clothing = SkeletonClothingModel::new(family);
            clothing.prepare(&instance);
            clothing
                .root()
                .render_textured(mesh, transform, pass.texture, entry.uv, pass.tint);
        } else {
            base.root()
                .render_textured(mesh, transform, pass.texture, entry.uv, pass.tint);
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
