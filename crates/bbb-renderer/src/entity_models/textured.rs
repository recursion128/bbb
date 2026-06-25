use super::colored::{
    creeper_model_root_transform, wind_charge_model_root_transform, wither_model_root_transform,
    HORSE_SCALE,
};
use super::dispatch::{dispatch_uniform_entity_model, TexturedSink};
use super::model::{EntityModel, ModelPart};
use super::{
    catalog::horse_markings_texture_ref,
    catalog::squid_texture_ref,
    catalog::{
        CamelModelFamily, DonkeyModelFamily, EntityDyeColor, EntityModelKind,
        EntityModelTextureAtlasEntry, EntityModelTextureAtlasLayout, EntityModelTextureRef,
        EntityModelUvRect, HoglinModelFamily, HorseMarkings, LlamaVariant, PiglinModelFamily,
        PlayerModelPartVisibility, SheepWoolColor, SkeletonModelFamily, TropicalFishModelShape,
        TropicalFishPattern, ZombieVariantModelFamily,
    },
    entity_model_root_transform,
    geometry::{
        append_scrolled_textured_mesh, emit_textured_model_cube, emit_textured_model_parts,
        fill_entity_textured_light, fill_entity_textured_overlay, part_pose_transform,
        EntityModelScrollMesh, EntityModelScrollVertex, EntityModelTexturedMesh,
        TexturedModelPartDesc,
    },
    instances::EntityModelInstance,
    mesh_transformer_scaled_model_root_transform,
    model_layers::{
        armor_layer_tint, armor_slot_texture, equine_head_look_pose, equine_leg_swing_pose,
        equine_tail_swing_pose, head_look_at_rest, limb_swing_at_rest, BreezeWindModel, CamelModel,
        CreeperModel, DrownedOuterModel, HoglinModel, HumanoidArmorSlot, LlamaModel, PiglinModel,
        PlayerModel, SheepFurModel, SheepModel, SkeletonClothingModel, SkeletonModel, SlimeModel,
        SlimeOuterModel, SquidModel, TropicalFishModel, TropicalFishPatternModel, WindChargeModel,
        WitherModel, ZombieModel, ZombieVariantModel, ADULT_DONKEY_PARTS_TEXTURED,
        ADULT_DONKEY_PARTS_WITH_CHEST_TEXTURED, ADULT_HORSE_PARTS_TEXTURED,
        BABY_HORSE_PARTS_TEXTURED, BREEZE_WIND_TEXTURE_REF, CREEPER_ARMOR_TEXTURE_REF,
        GUARDIAN_BEAM_TEXTURE_REF, PIGLIN_OUTER_ARMOR_DEFORMATION,
        STANDARD_OUTER_ARMOR_DEFORMATION, WIND_CHARGE_TEXTURE_REF, WITHER_ARMOR_TEXTURE_REF,
    },
    player_model_root_transform, slime_model_root_transform, squid_model_root_transform,
    tropical_fish_model_root_transform, wither_skeleton_model_root_transform, HUSK_SCALE,
};
use glam::{Mat4, Vec3};

mod layers;
pub(super) use layers::{
    armadillo_textured_layer_passes, arrow_textured_layer_passes, axolotl_textured_layer_passes,
    blaze_textured_layer_passes, boat_textured_layer_passes, camel_textured_layer_passes,
    chicken_textured_layer_passes, cow_textured_layer_passes, creaking_textured_layer_passes,
    creeper_textured_layer_passes, drowned_textured_layer_passes,
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
    piglin_textured_layer_passes, player_textured_layer_passes, polar_bear_textured_layer_passes,
    rabbit_textured_layer_passes, ravager_textured_layer_passes, salmon_textured_layer_passes,
    sheep_textured_layer_passes, shulker_bullet_textured_layer_passes,
    shulker_textured_layer_passes, silverfish_textured_layer_passes,
    skeleton_textured_layer_passes, slime_textured_layer_passes, sniffer_textured_layer_passes,
    snow_golem_textured_layer_passes, spider_textured_layer_passes, tadpole_textured_layer_passes,
    trident_textured_layer_passes, tropical_fish_textured_layer_passes,
    villager_textured_layer_passes, wandering_trader_textured_layer_passes,
    warden_textured_layer_passes, witch_textured_layer_passes, wither_skull_textured_layer_passes,
    wither_textured_layer_passes, wolf_textured_layer_passes,
    zombie_nautilus_textured_layer_passes, zombie_textured_layer_passes,
    zombie_villager_textured_layer_passes, EntityModelLayerKind, EntityModelLayerPass,
    EntityModelLayerRenderType,
};
#[cfg(test)]
pub(super) use layers::{warden_pulsating_spots_alpha, EntityModelLayerVisibility};

pub(super) struct EntityModelTexturedMeshes {
    pub(super) cutout: EntityModelTexturedMesh,
    pub(super) translucent: EntityModelTexturedMesh,
    pub(super) eyes: EntityModelTexturedMesh,
    /// Translucent scrolling overlay (vanilla `breezeWind` — the wind charge).
    pub(super) scroll: EntityModelScrollMesh,
    /// Additive scrolling overlay (vanilla `energySwirl` — the charged-creeper / wither glow).
    pub(super) scroll_additive: EntityModelScrollMesh,
}

impl EntityModelTexturedMeshes {
    fn new() -> Self {
        Self {
            cutout: EntityModelTexturedMesh::new(),
            translucent: EntityModelTexturedMesh::new(),
            eyes: EntityModelTexturedMesh::new(),
            scroll: EntityModelScrollMesh::new(),
            scroll_additive: EntityModelScrollMesh::new(),
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
                EntityModelKind::WindCharge => {
                    emit_wind_charge_scroll_model(&mut meshes, *instance, atlas);
                }
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
                EntityModelKind::Horse { baby, markings, .. } => {
                    emit_horse_textured_model(&mut meshes, *instance, baby, markings, atlas);
                }
                EntityModelKind::Donkey {
                    family,
                    baby: false,
                    has_chest,
                } => {
                    emit_donkey_textured_model(&mut meshes, *instance, family, has_chest, atlas);
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
        // Worn armor is a cutout overlay draped on the host humanoid pose; it runs regardless of
        // `handled` and folds into the cutout pass before the shared light/overlay fill below.
        emit_worn_humanoid_armor(&mut meshes, *instance, atlas);
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
        match pass.visibility {
            // A part-subset emissive overlay (vanilla `retainExactParts`): render only its named parts.
            layers::EntityModelLayerVisibility::RetainedParts(parts) => {
                if let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) {
                    model.root().render_textured_retained(
                        meshes.mesh_mut(pass.render_type),
                        transform,
                        pass.texture,
                        entry.uv,
                        pass.tint,
                        "",
                        parts,
                    );
                }
            }
            // `All` (and the player-parts case, whose subset is pre-applied to the tree) render whole.
            _ => render_textured_pass(
                meshes,
                model,
                transform,
                pass.render_type,
                pass.texture,
                pass.tint,
                atlas,
            ),
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
    let Some(entry) = entity_model_texture_atlas_entry(atlas, WIND_CHARGE_TEXTURE_REF) else {
        return;
    };
    let transform = wind_charge_model_root_transform(instance);
    let mut model = WindChargeModel::new();
    model.prepare(&instance);
    let mut scratch = EntityModelTexturedMesh::new();
    model.root().render_textured(
        &mut scratch,
        transform,
        WIND_CHARGE_TEXTURE_REF,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
    );
    // Vanilla `WindChargeRenderer.xOffset(t) = t · 0.03`, taken `% 1.0`; `ageInTicks ≥ 0` so the Java
    // float modulo is `rem_euclid`. V does not scroll.
    let u_offset = (instance.render_state.age_in_ticks * 0.03).rem_euclid(1.0);
    append_scrolled_textured_mesh(&mut meshes.scroll, &scratch, entry.uv, [u_offset, 0.0]);
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
    let Some(entry) = entity_model_texture_atlas_entry(atlas, BREEZE_WIND_TEXTURE_REF) else {
        return;
    };
    let transform = entity_model_root_transform(instance);
    let mut model = BreezeWindModel::new();
    model.prepare(&instance);
    let mut scratch = EntityModelTexturedMesh::new();
    model.root().render_textured(
        &mut scratch,
        transform,
        BREEZE_WIND_TEXTURE_REF,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
    );
    // Vanilla `BreezeWindLayer.xOffset(t) = t · 0.02`, taken `% 1.0`; `ageInTicks ≥ 0` so the Java
    // float modulo is `rem_euclid`. V does not scroll.
    let u_offset = (instance.render_state.age_in_ticks * 0.02).rem_euclid(1.0);
    append_scrolled_textured_mesh(&mut meshes.scroll, &scratch, entry.uv, [u_offset, 0.0]);
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
    let Some(entry) = entity_model_texture_atlas_entry(atlas, CREEPER_ARMOR_TEXTURE_REF) else {
        return;
    };
    let transform = creeper_model_root_transform(instance);
    let mut model = CreeperModel::new_armor();
    model.prepare(&instance);
    let mut scratch = EntityModelTexturedMesh::new();
    // Vanilla `EnergySwirlLayer` tints by `0xFF808080` (half grey) under additive blend.
    let grey = 128.0 / 255.0;
    model.root().render_textured(
        &mut scratch,
        transform,
        CREEPER_ARMOR_TEXTURE_REF,
        entry.uv,
        [grey, grey, grey, 1.0],
    );
    // Vanilla creeper `xOffset(t) = t · 0.01`, taken `% 1.0` on both U and V.
    let offset = (instance.render_state.age_in_ticks * 0.01).rem_euclid(1.0);
    append_scrolled_textured_mesh(
        &mut meshes.scroll_additive,
        &scratch,
        entry.uv,
        [offset, offset],
    );
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
    let Some(entry) = entity_model_texture_atlas_entry(atlas, WITHER_ARMOR_TEXTURE_REF) else {
        return;
    };
    let transform = wither_model_root_transform(instance);
    let mut model = WitherModel::new_armor();
    model.prepare(&instance);
    let mut scratch = EntityModelTexturedMesh::new();
    // Vanilla `EnergySwirlLayer` tints by `0xFF808080` (half grey) under additive blend.
    let grey = 128.0 / 255.0;
    model.root().render_textured(
        &mut scratch,
        transform,
        WITHER_ARMOR_TEXTURE_REF,
        entry.uv,
        [grey, grey, grey, 1.0],
    );
    // Vanilla `WitherArmorLayer.xOffset(t) = cos(t · 0.02) · 3` on U (oscillating, not linear like the
    // creeper), `t · 0.01` on V, each taken `% 1.0`. Java float modulo of a possibly-negative U keeps
    // the sign, then the shader's `fract` re-wraps it into `[0, 1)`, so plain `% 1.0` (`Rust` `rem`,
    // not `rem_euclid`) reproduces the vanilla offset exactly.
    let age = instance.render_state.age_in_ticks;
    let u_offset = ((age * 0.02).cos() * 3.0) % 1.0;
    let v_offset = (age * 0.01).rem_euclid(1.0);
    append_scrolled_textured_mesh(
        &mut meshes.scroll_additive,
        &scratch,
        entry.uv,
        [u_offset, v_offset],
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
    let Some(entry) = entity_model_texture_atlas_entry(atlas, GUARDIAN_BEAM_TEXTURE_REF) else {
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

    let top = length;
    let tex_v_off = (beam.attack_time * 0.5).rem_euclid(1.0);
    let min_v = -1.0 + tex_v_off;
    let max_v = min_v + length * 2.5;
    let v_base = if (beam.attack_time.floor() as i32).rem_euclid(2) == 0 {
        0.5
    } else {
        0.0
    };

    // 12 vertices in three quads (W↔E strip, N↔S strip, twisting top cap), local UVs in `0..1` for U and
    // tiling for V — matching `GuardianRenderer.vertex` exactly.
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
    let base =
        u32::try_from(meshes.scroll.vertices.len()).expect("scroll vertex count fits in u32");
    for (x, y, z, u, v) in vertices {
        let world = transform.transform_point3(Vec3::new(x, y, z));
        meshes.scroll.vertices.push(EntityModelScrollVertex {
            position: world.to_array(),
            local_uv: [u, v],
            uv_rect_min: rect.min,
            uv_rect_size: size,
            tint,
        });
    }
    // Each quad → two triangles (the scroll pipeline renders cull-off, so winding is immaterial).
    for quad in 0..3u32 {
        let o = base + quad * 4;
        meshes
            .scroll
            .indices
            .extend_from_slice(&[o, o + 1, o + 2, o, o + 2, o + 3]);
    }
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
        let texture = armor_slot_texture(material, slot);
        let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
            continue;
        };
        let mut tree = slot.build_tree(outer);
        tree.copy_child_poses_from(host_root, slot.part_names());
        tree.render_textured(
            meshes.mesh_mut(EntityModelLayerRenderType::Cutout),
            transform,
            texture,
            entry.uv,
            armor_layer_tint(material, dye),
        );
    }
}

/// Worn armor for the humanoid armor wearers (vanilla `HumanoidModel.createArmorMeshSet`, `INNER 0.5`
/// / `OUTER 1.0`, or the piglin family's `OUTER 1.02`). The base body is emitted by the shared dispatch
/// / bespoke emits; here we rebuild and pose an identical host humanoid model purely to read its limb
/// poses, then drape the armor pieces on it ([`emit_humanoid_armor`]). Covered: the zombie family
/// (zombie, husk, drowned, zombie villager), the skeleton family (skeleton, stray, wither/normal/bogged),
/// the player, and the piglin family (piglin, piglin brute, zombified piglin). DEFERRED: baby variants
/// (a distinct `createBabyArmorMesh`).
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
                atlas,
            );
        }
        EntityModelKind::Player { slim, .. } => {
            let mut host = PlayerModel::new(slim);
            host.prepare(&instance);
            let transform = player_model_root_transform(instance);
            emit_humanoid_armor(
                meshes,
                instance,
                host.root(),
                transform,
                STANDARD_OUTER_ARMOR_DEFORMATION,
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
                atlas,
            );
        }
        _ => {}
    }
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
    // mesh) posed by the SAME animator, so it tracks the limbs. The swim re-pose (needs `swimAmount`)
    // stays deferred. No root scale.
    let transform = entity_model_root_transform(instance);
    let mut base = ZombieVariantModel::new(ZombieVariantModelFamily::Drowned, baby);
    base.prepare(&instance);
    for pass in drowned_textured_layer_passes(baby) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        let mesh = meshes.mesh_mut(pass.render_type);
        if matches!(pass.kind, EntityModelLayerKind::DrownedOuter) {
            let mut outer = DrownedOuterModel::new(baby);
            outer.prepare(&instance);
            outer
                .root()
                .render_textured(mesh, transform, pass.texture, entry.uv, pass.tint);
        } else {
            base.root()
                .render_textured(mesh, transform, pass.texture, entry.uv, pass.tint);
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

/// The textured adult donkey / mule base layer. Vanilla `DonkeyModel` is the shared
/// `AbstractEquineModel.createBodyMesh` with `modifyMesh` (bigger ears replacing the horse ears, plus the
/// two side chest boxes shown when `hasChest`), on the 64×64 `donkey.png` / `mule.png` at the
/// `DonkeyModel.DONKEY_SCALE` 0.87 / `MULE_SCALE` 0.92 mesh-transformer scale. It takes the same equine
/// leg swing / head look/bob / tail walk lift as the horse (`AbstractEquineModel.setupAnim`), so it rides
/// `emit_equine_textured_posed`. The baby donkey/mule (a distinct re-parented `BabyDonkeyModel` mesh that
/// also forces `xRot = -30°`) stays on the colored path — its bespoke geometry is a deferred transcription.
fn emit_donkey_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: DonkeyModelFamily,
    has_chest: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(texture) = instance.kind.vanilla_texture_ref() else {
        return;
    };
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
        return;
    };
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
    emit_equine_textured_posed(
        &mut meshes.cutout,
        parts,
        [2, 3, 4, 5],
        1,
        0.0,
        1.0,
        mesh_transformer_scaled_model_root_transform(instance, scale),
        texture,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
        instance,
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
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
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
    emit_equine_textured_posed(
        &mut meshes.cutout,
        parts,
        leg_indices,
        head_parts_index,
        tail_x_rot_offset,
        age_scale,
        transform,
        texture,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
        instance,
    );
    // `HorseMarkingLayer`: a translucent white overlay of the SAME posed model, drawn after the base
    // when the coat carries markings (`Markings.NONE` → `INVISIBLE_TEXTURE`, skipped). It rides the
    // identical pose, so re-emitting the same tree into the translucent mesh tracks the body.
    if let Some(markings_texture) = horse_markings_texture_ref(markings, baby) {
        if let Some(markings_entry) = entity_model_texture_atlas_entry(atlas, markings_texture) {
            emit_equine_textured_posed(
                &mut meshes.translucent,
                parts,
                leg_indices,
                head_parts_index,
                tail_x_rot_offset,
                age_scale,
                transform,
                markings_texture,
                markings_entry.uv,
                [1.0, 1.0, 1.0, 1.0],
                instance,
            );
        }
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
    let Some(entry) = entity_model_texture_atlas_entry(atlas, texture) else {
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
    emit_equine_textured_posed(
        &mut meshes.cutout,
        parts,
        leg_indices,
        head_parts_index,
        tail_x_rot_offset,
        age_scale,
        entity_model_root_transform(instance),
        texture,
        entry.uv,
        [1.0, 1.0, 1.0, 1.0],
        instance,
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
