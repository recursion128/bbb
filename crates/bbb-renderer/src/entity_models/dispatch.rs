//! Shared per-entity model/transform selection for dispatch-owned entities. Most entries are
//! "uniform" entities whose colored and textured renders are fully described by a small fixed set of
//! model/root-transform/textured-layer-pass tuples; a few specialized entries use sink methods when
//! the colored fallback and textured submission generation intentionally diverge. The two render loops
//! (colored [`super::colored`] and textured [`super::textured`]) used to each carry their own
//! `match instance.kind` arm picking the model and transform; [`dispatch_uniform_entity_model`] is now
//! the single source of truth for that selection, emitting through whichever [`EntityModelSink`]
//! (colored or textured) the caller supplies. Entities whose two paths still diverge in model
//! structure, part visibility, or colored-only debug placeholders still return `false` for the colored
//! fallback path. The textured path treats dispatch as the authoritative submission source and has no
//! residual mesh-emitting arm. Scroll render types can still be dispatched when the
//! model/root/pass tuple is otherwise uniform; the textured sink folds them into scroll buckets after
//! recording submission metadata.

use glam::{Mat4, Vec3};

use super::catalog::{
    BoatModelFamily, CamelModelFamily, CowModelVariant, DonkeyModelFamily, EntityDyeColor,
    EntityDynamicPlayerSkinAtlasLayout, EntityDynamicPlayerTextureAtlasLayout, EntityModelKind,
    EntityModelTextureAtlasLayout, EntityPlayerSkin, HorseColorVariant, HorseMarkings,
    LlamaModelFamily, LlamaVariant, PigModelVariant, PiglinModelFamily, PlayerModelPartVisibility,
    SkeletonModelFamily, UndeadHorseModelFamily, WolfModelVariant, ZombieVariantModelFamily,
};
use super::colored::{
    arrow_model_root_transform, boat_model_root_transform, camel_model_color,
    cave_spider_model_root_transform, cod_model_root_transform, creeper_model_root_transform,
    emit_donkey_model, emit_horse_model, emit_undead_horse_model, end_crystal_model_root_transform,
    ender_dragon_model_root_transform, entity_model_root_transform,
    evoker_fangs_model_root_transform, fox_model_root_transform, ghast_model_root_transform,
    happy_ghast_model_root_transform, hoglin_model_color, iron_golem_model_root_transform,
    leash_knot_model_root_transform, llama_model_color, llama_spit_model_root_transform,
    magma_cube_model_root_transform, mesh_transformer_scaled_model_root_transform,
    panda_model_root_transform, phantom_model_root_transform, piglin_model_color,
    player_model_root_transform, polar_bear_model_root_transform, pufferfish_model_root_transform,
    salmon_model_root_transform, shulker_bullet_model_root_transform, shulker_model_root_transform,
    slime_model_root_transform, squid_model_root_transform, trident_model_root_transform,
    tropical_fish_model_root_transform, villager_adult_model_root_transform,
    wind_charge_model_root_transform, wither_model_root_transform,
    wither_skeleton_model_root_transform, wither_skull_model_root_transform, zombie_variant_color,
    zombie_variant_root_transform, GIANT_SCALE,
};
use super::geometry::{
    emit_model_cube, emit_model_part, part_pose_transform, EntityModelMesh, PartPose,
};
use super::instances::EntityModelInstance;
use super::model::EntityModel;
use super::model_layers::{
    bee_texture_ref, strider_texture_ref, AllayModel, ArmadilloModel, ArmorStandModel, ArrowModel,
    AxolotlModel, BatModel, BeeModel, BlazeModel, BoatModel, BreezeModel, CamelModel, ChickenModel,
    CodModel, CopperGolemModel, CowModel, CreakingModel, CreeperModel, DolphinModel,
    DrownedOuterModel, EnderDragonModel, EndermanModel, EndermiteModel, EvokerFangsModel,
    FelineModel, FoxModel, FrogModel, GhastModel, GoatModel, GuardianModel, HappyGhastModel,
    HoglinModel, IllagerModel, IronGolemModel, LeashKnotModel, LlamaModel, LlamaSpitModel,
    MagmaCubeModel, MinecartModel, NautilusModel, PandaModel, ParrotModel, PhantomModel, PigModel,
    PiglinModel, PlayerModel, PolarBearModel, PufferfishModel, RabbitModel, RavagerModel,
    SalmonModel, SheepFurModel, SheepModel, ShulkerBulletModel, ShulkerModel, SilverfishModel,
    SkeletonModel, SlimeModel, SlimeOuterModel, SnifferModel, SnowGolemModel, SpiderModel,
    SquidModel, StriderModel, TadpoleModel, TridentModel, TropicalFishModel,
    TropicalFishPatternModel, TurtleModel, VexModel, VillagerModel, WanderingTraderModel,
    WardenModel, WindChargeModel, WitchModel, WitherModel, WitherSkullModel, WolfModel,
    ZombieModel, ZombieVariantModel, ALLAY_TEXTURE_REF, ARMOR_STAND_TEXTURE_REF, BAT_TEXTURE_REF,
    COD_TEXTURE_REF, DOLPHIN_BABY_TEXTURE_REF, DOLPHIN_TEXTURE_REF, END_CRYSTAL_PARTS,
    FELINE_CAT_SCALE, GLOW_SQUID_TEAL, GUARDIAN_ELDER_SCALE, MODEL_LAYER_ALLAY,
    MODEL_LAYER_ARMOR_STAND, MODEL_LAYER_ARMOR_STAND_SMALL, MODEL_LAYER_BAT, MODEL_LAYER_BEE,
    MODEL_LAYER_BEE_BABY, MODEL_LAYER_COD, MODEL_LAYER_DOLPHIN, MODEL_LAYER_DOLPHIN_BABY,
    MODEL_LAYER_PUFFERFISH_BIG, MODEL_LAYER_PUFFERFISH_MEDIUM, MODEL_LAYER_PUFFERFISH_SMALL,
    MODEL_LAYER_STRIDER, MODEL_LAYER_STRIDER_BABY, MODEL_LAYER_TURTLE, MODEL_LAYER_TURTLE_BABY,
    MODEL_LAYER_VEX, PUFFERFISH_TEXTURE_REF, SQUID_BLUE, TURTLE_BABY_TEXTURE_REF,
    TURTLE_EGG_ROOT_DROP_POSE, TURTLE_TEXTURE_REF, VEX_CHARGING_TEXTURE_REF, VEX_TEXTURE_REF,
    WITHER_SKELETON_DARK,
};
use super::textured::{
    armadillo_textured_layer_passes, arrow_textured_layer_passes, axolotl_textured_layer_passes,
    blaze_textured_layer_passes, boat_textured_layer_passes, breeze_textured_layer_passes,
    camel_textured_layer_passes, chicken_textured_layer_passes, copper_golem_textured_layer_passes,
    cow_textured_layer_passes, creaking_textured_layer_passes, creeper_textured_layer_passes,
    drowned_textured_layer_passes, end_crystal_textured_layer_passes,
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
    piglin_textured_layer_passes, polar_bear_textured_layer_passes, rabbit_textured_layer_passes,
    ravager_textured_layer_passes, render_boat_water_mask_submission,
    render_breeze_wind_scroll_model, render_camel_saddle_layer,
    render_charged_creeper_energy_swirl, render_custom_head_skull_layer,
    render_donkey_textured_layers, render_end_crystal_beam, render_end_crystal_textured_layers,
    render_ender_dragon_beam, render_equine_body_armor_layer, render_equine_saddle_layer,
    render_guardian_beam, render_horse_textured_layers, render_llama_decor_layer,
    render_nautilus_body_armor_layer, render_nautilus_saddle_layer,
    render_no_overlay_scrolled_textured_layers, render_pig_saddle_layer, render_player_cape_layer,
    render_player_extra_ears_layer, render_player_parrot_on_shoulder_layer,
    render_player_spin_attack_effect_layer, render_player_textured_layers,
    render_skeleton_clothing_layer, render_strider_saddle_layer, render_textured_layers,
    render_trident_foil_submission, render_undead_horse_textured_layers,
    render_villager_profession_layers, render_wings_layer, render_wither_energy_swirl,
    render_wolf_body_armor_layer, render_worn_humanoid_armor, salmon_textured_layer_passes,
    sheep_textured_layer_passes, shulker_bullet_textured_layer_passes,
    shulker_textured_layer_passes, silverfish_textured_layer_passes,
    skeleton_textured_layer_passes, slime_textured_layer_passes, sniffer_textured_layer_passes,
    snow_golem_textured_layer_passes, spider_textured_layer_passes, squid_textured_layer_passes,
    tadpole_textured_layer_passes, trident_textured_layer_passes,
    tropical_fish_textured_layer_passes, villager_textured_layer_passes,
    wandering_trader_textured_layer_passes, warden_textured_layer_passes,
    wind_charge_textured_layer_passes, witch_textured_layer_passes,
    wither_skull_textured_layer_passes, wither_textured_layer_passes, wolf_textured_layer_passes,
    zombie_nautilus_textured_layer_passes, zombie_textured_layer_passes,
    zombie_villager_textured_layer_passes, EntityModelLayerKind, EntityModelLayerPass,
    EntityModelLayerRenderType, EntityModelLayerVisibility, EntityModelTexturedMeshes,
};

fn strider_base_layer_pass(baby: bool, cold: bool) -> EntityModelLayerPass {
    EntityModelLayerPass {
        kind: EntityModelLayerKind::StriderBase,
        render_type: EntityModelLayerRenderType::EntityCutout,
        model_layer: if baby {
            MODEL_LAYER_STRIDER_BABY
        } else {
            MODEL_LAYER_STRIDER
        },
        texture: strider_texture_ref(baby, cold),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        order: 0,
        submit_sequence: 0,
    }
}

/// A render-path-agnostic sink for each model/root-transform/layer-pass tuple in a uniform entity.
/// [`dispatch_uniform_entity_model`] drives this; the colored implementation renders the cube tree
/// (ignoring `passes`), the textured implementation walks `passes`. `passes` is empty for
/// colored-only entities, which therefore emit nothing on the textured path. `textured_only_model`
/// covers vanilla re-submits such as the shulker-bullet translucent shell that should not duplicate
/// the legacy colored fallback mesh.
pub(in crate::entity_models) trait EntityModelSink {
    fn model<M: EntityModel>(
        &mut self,
        model: M,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
    );

    fn textured_only_model<M: EntityModel>(
        &mut self,
        model: M,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
    ) {
        self.model(model, transform, instance, passes);
    }

    fn model_with_colored_override<M: EntityModel>(
        &mut self,
        model: M,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
        _color: [f32; 4],
    ) {
        self.model(model, transform, instance, passes);
    }

    fn no_overlay_scrolled_model<M: EntityModel>(
        &mut self,
        model: M,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
        _uv_offset: [f32; 2],
    ) {
        self.model(model, transform, instance, passes);
    }

    fn end_crystal_model(
        &mut self,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
    );

    fn ender_dragon_model(&mut self, instance: &EntityModelInstance) {
        let passes = ender_dragon_textured_layer_passes();
        self.model(
            EnderDragonModel::new(),
            ender_dragon_model_root_transform(*instance),
            instance,
            &passes[0..2],
        );
    }

    fn guardian_model(&mut self, elder: bool, instance: &EntityModelInstance) {
        let scale = if elder { GUARDIAN_ELDER_SCALE } else { 1.0 };
        let passes = guardian_textured_layer_passes(elder);
        self.model(
            GuardianModel::new(),
            mesh_transformer_scaled_model_root_transform(*instance, scale),
            instance,
            &passes[0..1],
        );
    }

    fn pig_model(&mut self, variant: PigModelVariant, baby: bool, instance: &EntityModelInstance) {
        self.model(
            PigModel::new(variant, baby),
            entity_model_root_transform(*instance),
            instance,
            &pig_textured_layer_passes(variant, baby),
        );
    }

    fn strider_model(&mut self, baby: bool, cold: bool, instance: &EntityModelInstance) {
        self.model(
            StriderModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &[strider_base_layer_pass(baby, cold)],
        );
    }

    fn camel_model(
        &mut self,
        family: CamelModelFamily,
        baby: bool,
        instance: &EntityModelInstance,
    ) {
        let transform = entity_model_root_transform(*instance);
        let passes = camel_textured_layer_passes(family, baby);
        if matches!(family, CamelModelFamily::CamelHusk) {
            self.model_with_colored_override(
                CamelModel::new(family, baby),
                transform,
                instance,
                &passes,
                camel_model_color(family),
            );
        } else {
            self.model(CamelModel::new(family, baby), transform, instance, &passes);
        }
    }

    fn llama_model(
        &mut self,
        family: LlamaModelFamily,
        variant: LlamaVariant,
        baby: bool,
        has_chest: bool,
        instance: &EntityModelInstance,
    ) {
        self.model_with_colored_override(
            LlamaModel::new(baby, has_chest),
            entity_model_root_transform(*instance),
            instance,
            &llama_textured_layer_passes(variant, baby, has_chest),
            llama_model_color(family, variant),
        );
    }

    fn nautilus_model(&mut self, baby: bool, instance: &EntityModelInstance) {
        self.model(
            NautilusModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &nautilus_textured_layer_passes(baby),
        );
    }

    fn zombie_nautilus_model(&mut self, coral: bool, instance: &EntityModelInstance) {
        self.model(
            if coral {
                NautilusModel::new_coral()
            } else {
                NautilusModel::new(false)
            },
            entity_model_root_transform(*instance),
            instance,
            &zombie_nautilus_textured_layer_passes(coral),
        );
    }

    fn wolf_model(
        &mut self,
        baby: bool,
        tame: bool,
        angry: bool,
        collar_color: Option<EntityDyeColor>,
        variant: WolfModelVariant,
        instance: &EntityModelInstance,
    ) {
        self.model(
            WolfModel::new(baby, angry),
            entity_model_root_transform(*instance),
            instance,
            &wolf_textured_layer_passes(
                baby,
                tame,
                angry,
                collar_color,
                variant,
                instance.render_state.wolf_wet_shade,
            ),
        );
    }

    fn player_model(
        &mut self,
        skin: EntityPlayerSkin,
        parts: PlayerModelPartVisibility,
        instance: &EntityModelInstance,
    );

    fn player_post_wings_layers(&mut self, _instance: &EntityModelInstance) {}

    fn wolf_body_armor_layer(
        &mut self,
        _instance: &EntityModelInstance,
        submit_sequence_start: u32,
    ) -> u32 {
        submit_sequence_start
    }

    fn worn_humanoid_armor(&mut self, _instance: &EntityModelInstance) {}

    fn custom_head_skull_layer(&mut self, _instance: &EntityModelInstance) {}

    fn wings_layer(&mut self, _instance: &EntityModelInstance) {}

    fn skeleton_clothing_layer(&mut self, _instance: &EntityModelInstance) {}

    fn villager_profession_layers(&mut self, _instance: &EntityModelInstance) {}

    fn horse_model(
        &mut self,
        variant: HorseColorVariant,
        baby: bool,
        markings: HorseMarkings,
        instance: &EntityModelInstance,
    );

    fn donkey_model(
        &mut self,
        family: DonkeyModelFamily,
        baby: bool,
        has_chest: bool,
        instance: &EntityModelInstance,
    );

    fn undead_horse_model(
        &mut self,
        family: UndeadHorseModelFamily,
        baby: bool,
        instance: &EntityModelInstance,
    );

    fn boat_model(&mut self, family: BoatModelFamily, chest: bool, instance: &EntityModelInstance);

    fn breeze_model(&mut self, instance: &EntityModelInstance);

    fn trident_model(&mut self, instance: &EntityModelInstance);

    fn creeper_model(&mut self, instance: &EntityModelInstance);

    fn wither_model(&mut self, instance: &EntityModelInstance);
}

/// The colored sink: render the posed cube tree into the colored mesh. Texture-backed entities (those
/// with non-empty textured passes) are skipped in the colored-runtime mesh, matching the former
/// `if !skip_texture_backed_entities` arm wrapper.
pub(in crate::entity_models) struct ColoredSink<'a> {
    pub(in crate::entity_models) mesh: &'a mut EntityModelMesh,
    pub(in crate::entity_models) skip_texture_backed: bool,
}

impl EntityModelSink for ColoredSink<'_> {
    fn model<M: EntityModel>(
        &mut self,
        mut model: M,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
    ) {
        if self.skip_texture_backed && !passes.is_empty() {
            return;
        }
        model.prepare_and_render(self.mesh, instance, transform);
    }

    fn textured_only_model<M: EntityModel>(
        &mut self,
        _model: M,
        _transform: Mat4,
        _instance: &EntityModelInstance,
        _passes: &[EntityModelLayerPass],
    ) {
    }

    fn model_with_colored_override<M: EntityModel>(
        &mut self,
        mut model: M,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
        color: [f32; 4],
    ) {
        if self.skip_texture_backed && !passes.is_empty() {
            return;
        }
        model.prepare_and_render_with_color(self.mesh, instance, transform, color);
    }

    fn end_crystal_model(
        &mut self,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
    ) {
        if self.skip_texture_backed && !passes.is_empty() {
            return;
        }
        render_colored_end_crystal_model(self.mesh, instance, transform);
    }

    fn player_model(
        &mut self,
        skin: EntityPlayerSkin,
        _parts: PlayerModelPartVisibility,
        instance: &EntityModelInstance,
    ) {
        if self.skip_texture_backed {
            return;
        }
        PlayerModel::new(skin.is_slim()).prepare_and_render(
            self.mesh,
            instance,
            player_model_root_transform(*instance),
        );
    }

    fn horse_model(
        &mut self,
        _variant: HorseColorVariant,
        baby: bool,
        _markings: HorseMarkings,
        instance: &EntityModelInstance,
    ) {
        if self.skip_texture_backed {
            return;
        }
        emit_horse_model(self.mesh, *instance, baby);
    }

    fn donkey_model(
        &mut self,
        family: DonkeyModelFamily,
        baby: bool,
        has_chest: bool,
        instance: &EntityModelInstance,
    ) {
        if self.skip_texture_backed {
            return;
        }
        emit_donkey_model(self.mesh, *instance, family, baby, has_chest);
    }

    fn undead_horse_model(
        &mut self,
        family: UndeadHorseModelFamily,
        baby: bool,
        instance: &EntityModelInstance,
    ) {
        if self.skip_texture_backed {
            return;
        }
        emit_undead_horse_model(self.mesh, *instance, family, baby);
    }

    fn boat_model(&mut self, family: BoatModelFamily, chest: bool, instance: &EntityModelInstance) {
        let passes = boat_textured_layer_passes(family, chest);
        let body_passes = [passes[0]];
        self.model(
            BoatModel::new(family, chest),
            boat_model_root_transform(*instance),
            instance,
            &body_passes,
        );
    }

    fn breeze_model(&mut self, instance: &EntityModelInstance) {
        let passes = breeze_textured_layer_passes();
        let body_passes = [passes[0]];
        self.model(
            BreezeModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &body_passes,
        );
    }

    fn trident_model(&mut self, instance: &EntityModelInstance) {
        let passes = trident_textured_layer_passes();
        let body_passes = [passes[0]];
        self.model(
            TridentModel::new(),
            trident_model_root_transform(*instance),
            instance,
            &body_passes,
        );
    }

    fn creeper_model(&mut self, instance: &EntityModelInstance) {
        let passes = creeper_textured_layer_passes();
        let body_passes = [passes[0]];
        self.model(
            CreeperModel::new(),
            creeper_model_root_transform(*instance),
            instance,
            &body_passes,
        );
    }

    fn wither_model(&mut self, instance: &EntityModelInstance) {
        let passes = wither_textured_layer_passes(instance.render_state.wither_invulnerable_ticks);
        let body_passes = [passes[0]];
        self.model(
            WitherModel::new(),
            wither_model_root_transform(*instance),
            instance,
            &body_passes,
        );
    }
}

/// The textured sink: walk the entity's textured layer passes over the posed tree. Colored-only
/// entities pass an empty slice here, so this emits nothing for them (correct: they produce no
/// textured geometry).
pub(in crate::entity_models) struct TexturedSink<'a> {
    pub(in crate::entity_models) meshes: &'a mut EntityModelTexturedMeshes,
    pub(in crate::entity_models) atlas: &'a EntityModelTextureAtlasLayout,
    pub(in crate::entity_models) dynamic_player_skin_atlas:
        Option<&'a EntityDynamicPlayerSkinAtlasLayout>,
    pub(in crate::entity_models) dynamic_player_texture_atlas:
        Option<&'a EntityDynamicPlayerTextureAtlasLayout>,
}

impl EntityModelSink for TexturedSink<'_> {
    fn model<M: EntityModel>(
        &mut self,
        mut model: M,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
    ) {
        model.prepare(instance);
        render_textured_layers(
            self.meshes,
            &model,
            transform,
            passes.iter().cloned(),
            self.atlas,
        );
    }

    fn no_overlay_scrolled_model<M: EntityModel>(
        &mut self,
        mut model: M,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
        uv_offset: [f32; 2],
    ) {
        model.prepare(instance);
        render_no_overlay_scrolled_textured_layers(
            self.meshes,
            &model,
            transform,
            passes.iter().cloned(),
            self.atlas,
            uv_offset,
        );
    }

    fn end_crystal_model(
        &mut self,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
    ) {
        render_end_crystal_textured_layers(
            self.meshes,
            transform,
            instance,
            passes.iter().cloned(),
            self.atlas,
        );
        render_end_crystal_beam(self.meshes, *instance, self.atlas);
    }

    fn ender_dragon_model(&mut self, instance: &EntityModelInstance) {
        let passes = ender_dragon_textured_layer_passes();
        self.model(
            EnderDragonModel::new(),
            ender_dragon_model_root_transform(*instance),
            instance,
            &passes[0..2],
        );
        render_ender_dragon_beam(self.meshes, *instance, self.atlas);
    }

    fn guardian_model(&mut self, elder: bool, instance: &EntityModelInstance) {
        let scale = if elder { GUARDIAN_ELDER_SCALE } else { 1.0 };
        let passes = guardian_textured_layer_passes(elder);
        self.model(
            GuardianModel::new(),
            mesh_transformer_scaled_model_root_transform(*instance, scale),
            instance,
            &passes[0..1],
        );
        render_guardian_beam(self.meshes, *instance, self.atlas);
    }

    fn pig_model(&mut self, variant: PigModelVariant, baby: bool, instance: &EntityModelInstance) {
        self.model(
            PigModel::new(variant, baby),
            entity_model_root_transform(*instance),
            instance,
            &pig_textured_layer_passes(variant, baby),
        );
        render_pig_saddle_layer(self.meshes, *instance, self.atlas);
    }

    fn strider_model(&mut self, baby: bool, cold: bool, instance: &EntityModelInstance) {
        self.model(
            StriderModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &[strider_base_layer_pass(baby, cold)],
        );
        render_strider_saddle_layer(self.meshes, *instance, self.atlas);
    }

    fn camel_model(
        &mut self,
        family: CamelModelFamily,
        baby: bool,
        instance: &EntityModelInstance,
    ) {
        let transform = entity_model_root_transform(*instance);
        let passes = camel_textured_layer_passes(family, baby);
        if matches!(family, CamelModelFamily::CamelHusk) {
            self.model_with_colored_override(
                CamelModel::new(family, baby),
                transform,
                instance,
                &passes,
                camel_model_color(family),
            );
        } else {
            self.model(CamelModel::new(family, baby), transform, instance, &passes);
        }
        render_camel_saddle_layer(self.meshes, *instance, self.atlas);
    }

    fn llama_model(
        &mut self,
        family: LlamaModelFamily,
        variant: LlamaVariant,
        baby: bool,
        has_chest: bool,
        instance: &EntityModelInstance,
    ) {
        self.model_with_colored_override(
            LlamaModel::new(baby, has_chest),
            entity_model_root_transform(*instance),
            instance,
            &llama_textured_layer_passes(variant, baby, has_chest),
            llama_model_color(family, variant),
        );
        render_llama_decor_layer(self.meshes, *instance, self.atlas);
    }

    fn nautilus_model(&mut self, baby: bool, instance: &EntityModelInstance) {
        self.model(
            NautilusModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &nautilus_textured_layer_passes(baby),
        );
        render_nautilus_body_armor_layer(self.meshes, *instance, self.atlas);
        render_nautilus_saddle_layer(self.meshes, *instance, self.atlas);
    }

    fn zombie_nautilus_model(&mut self, coral: bool, instance: &EntityModelInstance) {
        self.model(
            if coral {
                NautilusModel::new_coral()
            } else {
                NautilusModel::new(false)
            },
            entity_model_root_transform(*instance),
            instance,
            &zombie_nautilus_textured_layer_passes(coral),
        );
        render_nautilus_body_armor_layer(self.meshes, *instance, self.atlas);
        render_nautilus_saddle_layer(self.meshes, *instance, self.atlas);
    }

    fn wolf_model(
        &mut self,
        baby: bool,
        tame: bool,
        angry: bool,
        collar_color: Option<EntityDyeColor>,
        variant: WolfModelVariant,
        instance: &EntityModelInstance,
    ) {
        let transform = entity_model_root_transform(*instance);
        let passes = wolf_textured_layer_passes(
            baby,
            tame,
            angry,
            collar_color,
            variant,
            instance.render_state.wolf_wet_shade,
        );
        if self.meshes.current_invisible_base_only() {
            self.model(WolfModel::new(baby, angry), transform, instance, &passes);
            return;
        }

        let mut model = WolfModel::new(baby, angry);
        model.prepare(instance);
        render_textured_layers(self.meshes, &model, transform, [passes[0]], self.atlas);
        let next_submit_sequence =
            render_wolf_body_armor_layer(self.meshes, *instance, self.atlas, 1);
        if let Some(collar_pass) = passes.get(1) {
            let mut collar_pass = *collar_pass;
            collar_pass.submit_sequence = next_submit_sequence;
            render_textured_layers(self.meshes, &model, transform, [collar_pass], self.atlas);
        }
    }

    fn player_model(
        &mut self,
        skin: EntityPlayerSkin,
        parts: PlayerModelPartVisibility,
        instance: &EntityModelInstance,
    ) {
        render_player_textured_layers(
            self.meshes,
            instance,
            skin,
            parts,
            self.atlas,
            self.dynamic_player_skin_atlas,
        );
        render_player_extra_ears_layer(
            self.meshes,
            *instance,
            self.atlas,
            self.dynamic_player_skin_atlas,
        );
        render_player_cape_layer(self.meshes, *instance, self.dynamic_player_texture_atlas);
    }

    fn player_post_wings_layers(&mut self, instance: &EntityModelInstance) {
        render_player_parrot_on_shoulder_layer(self.meshes, *instance, self.atlas);
        render_player_spin_attack_effect_layer(self.meshes, *instance, self.atlas);
    }

    fn wolf_body_armor_layer(
        &mut self,
        instance: &EntityModelInstance,
        submit_sequence_start: u32,
    ) -> u32 {
        render_wolf_body_armor_layer(self.meshes, *instance, self.atlas, submit_sequence_start)
    }

    fn worn_humanoid_armor(&mut self, instance: &EntityModelInstance) {
        render_worn_humanoid_armor(self.meshes, *instance, self.atlas);
    }

    fn custom_head_skull_layer(&mut self, instance: &EntityModelInstance) {
        render_custom_head_skull_layer(
            self.meshes,
            *instance,
            self.atlas,
            self.dynamic_player_skin_atlas,
        );
    }

    fn wings_layer(&mut self, instance: &EntityModelInstance) {
        render_wings_layer(
            self.meshes,
            *instance,
            self.atlas,
            self.dynamic_player_texture_atlas,
        );
    }

    fn skeleton_clothing_layer(&mut self, instance: &EntityModelInstance) {
        render_skeleton_clothing_layer(self.meshes, *instance, self.atlas);
    }

    fn villager_profession_layers(&mut self, instance: &EntityModelInstance) {
        render_villager_profession_layers(self.meshes, *instance, self.atlas);
    }

    fn horse_model(
        &mut self,
        variant: HorseColorVariant,
        baby: bool,
        markings: HorseMarkings,
        instance: &EntityModelInstance,
    ) {
        render_horse_textured_layers(self.meshes, instance, variant, baby, markings, self.atlas);
        render_equine_body_armor_layer(self.meshes, *instance, self.atlas);
        render_equine_saddle_layer(self.meshes, *instance, self.atlas);
    }

    fn donkey_model(
        &mut self,
        family: DonkeyModelFamily,
        baby: bool,
        has_chest: bool,
        instance: &EntityModelInstance,
    ) {
        render_donkey_textured_layers(self.meshes, instance, family, baby, has_chest, self.atlas);
        render_equine_saddle_layer(self.meshes, *instance, self.atlas);
    }

    fn undead_horse_model(
        &mut self,
        family: UndeadHorseModelFamily,
        baby: bool,
        instance: &EntityModelInstance,
    ) {
        render_undead_horse_textured_layers(self.meshes, instance, family, baby, self.atlas);
        render_equine_body_armor_layer(self.meshes, *instance, self.atlas);
        render_equine_saddle_layer(self.meshes, *instance, self.atlas);
    }

    fn boat_model(&mut self, family: BoatModelFamily, chest: bool, instance: &EntityModelInstance) {
        let passes = boat_textured_layer_passes(family, chest);
        let body_passes = [passes[0]];
        self.model(
            BoatModel::new(family, chest),
            boat_model_root_transform(*instance),
            instance,
            &body_passes,
        );
        render_boat_water_mask_submission(self.meshes, *instance);
    }

    fn breeze_model(&mut self, instance: &EntityModelInstance) {
        let passes = breeze_textured_layer_passes();
        let transform = entity_model_root_transform(*instance);
        let mut model = BreezeModel::new();
        model.prepare(instance);
        render_textured_layers(self.meshes, &model, transform, [passes[0]], self.atlas);
        render_breeze_wind_scroll_model(self.meshes, *instance, self.atlas);
        render_textured_layers(self.meshes, &model, transform, [passes[2]], self.atlas);
    }

    fn trident_model(&mut self, instance: &EntityModelInstance) {
        let passes = trident_textured_layer_passes();
        let body_passes = [passes[0]];
        self.model(
            TridentModel::new(),
            trident_model_root_transform(*instance),
            instance,
            &body_passes,
        );
        render_trident_foil_submission(self.meshes, *instance);
    }

    fn creeper_model(&mut self, instance: &EntityModelInstance) {
        let passes = creeper_textured_layer_passes();
        let body_passes = [passes[0]];
        self.model(
            CreeperModel::new(),
            creeper_model_root_transform(*instance),
            instance,
            &body_passes,
        );
        render_charged_creeper_energy_swirl(self.meshes, *instance, self.atlas);
    }

    fn wither_model(&mut self, instance: &EntityModelInstance) {
        let passes = wither_textured_layer_passes(instance.render_state.wither_invulnerable_ticks);
        let body_passes = [passes[0]];
        self.model(
            WitherModel::new(),
            wither_model_root_transform(*instance),
            instance,
            &body_passes,
        );
        render_wither_energy_swirl(self.meshes, *instance, self.atlas);
    }
}

/// Emits `instance` through `sink` if its base submission is dispatch-owned. Returns `true` if emitted,
/// `false` for the colored-only fallback kinds (`Humanoid`, `Quadruped`, `Placeholder`). This is the
/// single source of truth for texture-backed model/transform selection, replacing the duplicated arms
/// in the colored and textured render matches.
pub(in crate::entity_models) fn dispatch_uniform_entity_model<S: EntityModelSink>(
    instance: &EntityModelInstance,
    sink: &mut S,
) -> bool {
    match instance.kind {
        EntityModelKind::NoRender => {
            // Vanilla `NoopRenderer` entities submit no model.
        }
        EntityModelKind::Player { skin, parts } => sink.player_model(skin, parts, instance),
        EntityModelKind::Horse {
            variant,
            baby,
            markings,
        } => sink.horse_model(variant, baby, markings, instance),
        EntityModelKind::Donkey {
            family,
            baby,
            has_chest,
        } => sink.donkey_model(family, baby, has_chest, instance),
        EntityModelKind::UndeadHorse { family, baby } => {
            sink.undead_horse_model(family, baby, instance)
        }
        // ---- Both-uniform (colored + textured), passes from a `*_textured_layer_passes` fn ----
        EntityModelKind::Chicken { variant, baby } => sink.model(
            ChickenModel::new(variant, baby),
            entity_model_root_transform(*instance),
            instance,
            &chicken_textured_layer_passes(variant, baby),
        ),
        EntityModelKind::Pig { variant, baby } => sink.pig_model(variant, baby, instance),
        EntityModelKind::Cow { variant, baby } => sink.model(
            CowModel::new(variant, baby),
            entity_model_root_transform(*instance),
            instance,
            &cow_textured_layer_passes(variant, baby),
        ),
        EntityModelKind::Sheep {
            baby,
            sheared,
            wool_color,
            jeb,
            age_ticks,
        } => {
            let transform = entity_model_root_transform(*instance);
            for pass in sheep_textured_layer_passes(baby, sheared, wool_color, jeb, age_ticks) {
                let passes = [pass];
                match pass.kind {
                    EntityModelLayerKind::SheepBase => {
                        sink.model(SheepModel::new(baby), transform, instance, &passes)
                    }
                    EntityModelLayerKind::SheepWoolUndercoat => sink.model_with_colored_override(
                        SheepModel::new(baby),
                        transform,
                        instance,
                        &passes,
                        pass.tint,
                    ),
                    EntityModelLayerKind::SheepWool => sink.model_with_colored_override(
                        SheepFurModel::new(baby),
                        transform,
                        instance,
                        &passes,
                        pass.tint,
                    ),
                    _ => unreachable!("sheep_textured_layer_passes only emits sheep passes"),
                }
            }
        }
        EntityModelKind::MagmaCube { size } => sink.model(
            MagmaCubeModel::new(),
            magma_cube_model_root_transform(*instance, size),
            instance,
            &magma_cube_textured_layer_passes(),
        ),
        EntityModelKind::Slime { size } => {
            let transform = slime_model_root_transform(*instance, size);
            let passes = slime_textured_layer_passes();
            sink.model(SlimeModel::new(), transform, instance, &passes[0..1]);
            sink.model(SlimeOuterModel::new(), transform, instance, &passes[1..2]);
        }
        EntityModelKind::Camel { family, baby } => sink.camel_model(family, baby, instance),
        EntityModelKind::Llama {
            family,
            variant,
            baby,
            has_chest,
        } => sink.llama_model(family, variant, baby, has_chest, instance),
        EntityModelKind::Squid { glow, baby } => {
            let transform = squid_model_root_transform(*instance, baby);
            let passes = squid_textured_layer_passes(glow, baby);
            let color = if glow { GLOW_SQUID_TEAL } else { SQUID_BLUE };
            sink.model_with_colored_override(
                SquidModel::new(),
                transform,
                instance,
                &passes,
                color,
            );
        }
        EntityModelKind::TropicalFish {
            shape,
            base_color,
            pattern,
            pattern_color,
        } => {
            let transform =
                tropical_fish_model_root_transform(*instance, instance.render_state.in_water);
            let passes =
                tropical_fish_textured_layer_passes(shape, base_color, pattern, pattern_color);
            sink.model_with_colored_override(
                TropicalFishModel::new(shape),
                transform,
                instance,
                &passes[0..1],
                base_color.texture_diffuse_color(),
            );
            sink.textured_only_model(
                TropicalFishPatternModel::new(shape),
                transform,
                instance,
                &passes[1..2],
            );
        }
        EntityModelKind::Hoglin { family, baby } => {
            let transform = entity_model_root_transform(*instance);
            let passes = hoglin_textured_layer_passes(family, baby);
            sink.model_with_colored_override(
                HoglinModel::new(baby),
                transform,
                instance,
                &passes,
                hoglin_model_color(family),
            );
        }
        EntityModelKind::Piglin { family, baby } => {
            let transform = entity_model_root_transform(*instance);
            let baby_layout = baby && family != PiglinModelFamily::PiglinBrute;
            let passes = piglin_textured_layer_passes(family, baby_layout);
            sink.model_with_colored_override(
                PiglinModel::new(family, baby),
                transform,
                instance,
                &passes,
                piglin_model_color(family),
            );
        }
        EntityModelKind::ShulkerBullet => {
            let transform = shulker_bullet_model_root_transform(*instance);
            let passes = shulker_bullet_textured_layer_passes();
            sink.model(
                ShulkerBulletModel::new(),
                transform,
                instance,
                &passes[0..1],
            );
            sink.textured_only_model(
                ShulkerBulletModel::new(),
                transform * Mat4::from_scale(Vec3::splat(1.5)),
                instance,
                &passes[1..2],
            );
        }
        EntityModelKind::Skeleton => {
            let transform = entity_model_root_transform(*instance);
            let passes = skeleton_textured_layer_passes(None);
            sink.model(SkeletonModel::new(None), transform, instance, &passes[0..1]);
        }
        EntityModelKind::SkeletonVariant { family } => {
            let transform = if matches!(family, SkeletonModelFamily::WitherSkeleton) {
                wither_skeleton_model_root_transform(*instance)
            } else {
                entity_model_root_transform(*instance)
            };
            let passes = skeleton_textured_layer_passes(Some(family));
            if matches!(family, SkeletonModelFamily::WitherSkeleton) {
                sink.model_with_colored_override(
                    SkeletonModel::new(Some(family)),
                    transform,
                    instance,
                    &passes[0..1],
                    WITHER_SKELETON_DARK,
                );
            } else {
                sink.model(
                    SkeletonModel::new(Some(family)),
                    transform,
                    instance,
                    &passes[0..1],
                );
            }
        }
        EntityModelKind::Ghast { charging } => sink.model(
            GhastModel::new(),
            ghast_model_root_transform(*instance),
            instance,
            &ghast_textured_layer_passes(charging),
        ),
        EntityModelKind::HappyGhast => sink.model(
            HappyGhastModel::new(),
            happy_ghast_model_root_transform(*instance),
            instance,
            &happy_ghast_textured_layer_passes(),
        ),
        EntityModelKind::Blaze => sink.model(
            BlazeModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &blaze_textured_layer_passes(),
        ),
        EntityModelKind::Endermite => sink.model(
            EndermiteModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &endermite_textured_layer_passes(),
        ),
        EntityModelKind::Silverfish => sink.model(
            SilverfishModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &silverfish_textured_layer_passes(),
        ),
        EntityModelKind::Zombie { baby } => sink.model(
            ZombieModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &zombie_textured_layer_passes(baby),
        ),
        EntityModelKind::ZombieVariant {
            family: family @ ZombieVariantModelFamily::Husk,
            baby,
        } => {
            let transform = zombie_variant_root_transform(*instance, family, baby);
            let passes = husk_textured_layer_passes(baby);
            sink.model_with_colored_override(
                ZombieVariantModel::new(family, baby),
                transform,
                instance,
                &passes,
                zombie_variant_color(family),
            );
        }
        EntityModelKind::ZombieVariant {
            family: family @ ZombieVariantModelFamily::Drowned,
            baby,
        } => {
            let transform = zombie_variant_root_transform(*instance, family, baby);
            let passes = drowned_textured_layer_passes(baby);
            sink.model_with_colored_override(
                ZombieVariantModel::new(family, baby),
                transform,
                instance,
                &passes[0..1],
                zombie_variant_color(family),
            );
            sink.textured_only_model(
                DrownedOuterModel::new(baby),
                transform,
                instance,
                &passes[1..2],
            );
        }
        EntityModelKind::ZombieVariant {
            family: family @ ZombieVariantModelFamily::ZombieVillager,
            baby,
        } => {
            let transform = zombie_variant_root_transform(*instance, family, baby);
            let passes = zombie_villager_textured_layer_passes(baby);
            sink.model_with_colored_override(
                ZombieVariantModel::new(family, baby),
                transform,
                instance,
                &passes,
                zombie_variant_color(family),
            );
        }
        EntityModelKind::Ravager => sink.model(
            RavagerModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &ravager_textured_layer_passes(),
        ),
        EntityModelKind::Creeper => sink.creeper_model(instance),
        EntityModelKind::IronGolem { crackiness } => sink.model(
            IronGolemModel::new(),
            iron_golem_model_root_transform(*instance),
            instance,
            &iron_golem_textured_layer_passes(crackiness),
        ),
        EntityModelKind::SnowGolem => sink.model(
            SnowGolemModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &snow_golem_textured_layer_passes(),
        ),
        EntityModelKind::CopperGolem { weathering } => sink.model(
            CopperGolemModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &copper_golem_textured_layer_passes(weathering),
        ),
        EntityModelKind::Witch => sink.model(
            WitchModel::new(),
            villager_adult_model_root_transform(*instance),
            instance,
            &witch_textured_layer_passes(),
        ),
        EntityModelKind::Minecart => sink.model(
            MinecartModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &minecart_textured_layer_passes(),
        ),
        EntityModelKind::WanderingTrader => sink.model(
            WanderingTraderModel::new(),
            villager_adult_model_root_transform(*instance),
            instance,
            &wandering_trader_textured_layer_passes(),
        ),
        EntityModelKind::Goat {
            baby,
            left_horn,
            right_horn,
        } => sink.model(
            GoatModel::new(baby, left_horn, right_horn),
            entity_model_root_transform(*instance),
            instance,
            &goat_textured_layer_passes(baby),
        ),
        EntityModelKind::Illager { family } => sink.model(
            IllagerModel::new(instance, family),
            villager_adult_model_root_transform(*instance),
            instance,
            &illager_textured_layer_passes(family),
        ),
        EntityModelKind::Villager { baby } => {
            let transform = if baby {
                entity_model_root_transform(*instance)
            } else {
                villager_adult_model_root_transform(*instance)
            };
            sink.model(
                VillagerModel::new(baby),
                transform,
                instance,
                &villager_textured_layer_passes(baby),
            )
        }
        EntityModelKind::PolarBear { baby } => {
            let transform = if baby {
                entity_model_root_transform(*instance)
            } else {
                polar_bear_model_root_transform(*instance)
            };
            sink.model(
                PolarBearModel::new(baby),
                transform,
                instance,
                &polar_bear_textured_layer_passes(baby),
            )
        }
        EntityModelKind::Phantom { size } => sink.model(
            PhantomModel::new(),
            phantom_model_root_transform(*instance, size),
            instance,
            &phantom_textured_layer_passes(),
        ),
        EntityModelKind::Salmon { size } => {
            let in_water = instance.render_state.in_water;
            sink.model(
                SalmonModel::new(),
                salmon_model_root_transform(*instance, in_water, size),
                instance,
                &salmon_textured_layer_passes(size),
            )
        }
        EntityModelKind::Wolf {
            baby,
            tame,
            angry,
            collar_color,
            variant,
        } => sink.wolf_model(baby, tame, angry, collar_color, variant, instance),
        EntityModelKind::Boat { family, chest } => sink.boat_model(family, chest, instance),
        EntityModelKind::Spider => sink.model(
            SpiderModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &spider_textured_layer_passes(false),
        ),
        EntityModelKind::CaveSpider => sink.model(
            SpiderModel::new(),
            cave_spider_model_root_transform(*instance),
            instance,
            &spider_textured_layer_passes(true),
        ),
        EntityModelKind::Enderman => sink.model(
            EndermanModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &enderman_textured_layer_passes(),
        ),

        // ---- Both-uniform single-pass: one plain `render_textured_pass`, no `*_textured_layer_passes` fn ----
        EntityModelKind::ArmorStand {
            small,
            marker,
            show_arms,
            show_base_plate,
            pose,
        } => {
            let marker_hidden = marker
                && instance.render_state.invisible
                && instance.render_state.invisible_to_player;
            let marker_force_transparent = marker
                && instance.render_state.invisible
                && !instance.render_state.invisible_to_player;
            let render_type = if marker_force_transparent {
                EntityModelLayerRenderType::EntityTranslucent
            } else {
                EntityModelLayerRenderType::EntityCutout
            };
            let base_pass = [EntityModelLayerPass {
                kind: EntityModelLayerKind::ArmorStandBase,
                render_type,
                model_layer: if small {
                    MODEL_LAYER_ARMOR_STAND_SMALL
                } else {
                    MODEL_LAYER_ARMOR_STAND
                },
                texture: ARMOR_STAND_TEXTURE_REF,
                visibility: EntityModelLayerVisibility::All,
                tint: [1.0, 1.0, 1.0, 1.0],
                order: 0,
                submit_sequence: 0,
            }];
            let base_passes: &[EntityModelLayerPass] = if marker_hidden { &[] } else { &base_pass };
            sink.model(
                ArmorStandModel::new(small, show_arms, show_base_plate, pose),
                entity_model_root_transform(*instance),
                instance,
                base_passes,
            )
        }
        EntityModelKind::Vex { charging } => sink.model(
            VexModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &[EntityModelLayerPass {
                kind: EntityModelLayerKind::VexBase,
                render_type: EntityModelLayerRenderType::EntityTranslucent,
                model_layer: MODEL_LAYER_VEX,
                texture: if charging {
                    VEX_CHARGING_TEXTURE_REF
                } else {
                    VEX_TEXTURE_REF
                },
                visibility: EntityModelLayerVisibility::All,
                tint: [1.0, 1.0, 1.0, 1.0],
                order: 0,
                submit_sequence: 0,
            }],
        ),
        EntityModelKind::Allay => sink.model(
            AllayModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &[EntityModelLayerPass {
                kind: EntityModelLayerKind::AllayBase,
                render_type: EntityModelLayerRenderType::EntityTranslucent,
                model_layer: MODEL_LAYER_ALLAY,
                texture: ALLAY_TEXTURE_REF,
                visibility: EntityModelLayerVisibility::All,
                tint: [1.0, 1.0, 1.0, 1.0],
                order: 0,
                submit_sequence: 0,
            }],
        ),
        EntityModelKind::Strider { baby, cold } => sink.strider_model(baby, cold, instance),
        EntityModelKind::Bat => sink.model(
            BatModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &[EntityModelLayerPass {
                kind: EntityModelLayerKind::BatBase,
                render_type: EntityModelLayerRenderType::EntityCutoutCull,
                model_layer: MODEL_LAYER_BAT,
                texture: BAT_TEXTURE_REF,
                visibility: EntityModelLayerVisibility::All,
                tint: [1.0, 1.0, 1.0, 1.0],
                order: 0,
                submit_sequence: 0,
            }],
        ),
        EntityModelKind::Bee {
            baby,
            angry,
            has_nectar,
        } => {
            let texture = bee_texture_ref(baby, angry, has_nectar);
            sink.model(
                BeeModel::new(baby),
                entity_model_root_transform(*instance),
                instance,
                &[EntityModelLayerPass {
                    kind: EntityModelLayerKind::BeeBase,
                    render_type: EntityModelLayerRenderType::EntityCutout,
                    model_layer: if baby {
                        MODEL_LAYER_BEE_BABY
                    } else {
                        MODEL_LAYER_BEE
                    },
                    texture,
                    visibility: EntityModelLayerVisibility::All,
                    tint: [1.0, 1.0, 1.0, 1.0],
                    order: 0,
                    submit_sequence: 0,
                }],
            )
        }
        EntityModelKind::Breeze => sink.breeze_model(instance),
        EntityModelKind::Cod => {
            let in_water = instance.render_state.in_water;
            sink.model(
                CodModel::new(),
                cod_model_root_transform(*instance, in_water),
                instance,
                &[EntityModelLayerPass {
                    kind: EntityModelLayerKind::CodBase,
                    render_type: EntityModelLayerRenderType::EntityCutout,
                    model_layer: MODEL_LAYER_COD,
                    texture: COD_TEXTURE_REF,
                    visibility: EntityModelLayerVisibility::All,
                    tint: [1.0, 1.0, 1.0, 1.0],
                    order: 0,
                    submit_sequence: 0,
                }],
            )
        }
        EntityModelKind::Pufferfish { puff_state } => {
            let model_layer = match puff_state {
                0 => MODEL_LAYER_PUFFERFISH_SMALL,
                1 => MODEL_LAYER_PUFFERFISH_MEDIUM,
                _ => MODEL_LAYER_PUFFERFISH_BIG,
            };
            sink.model(
                PufferfishModel::new(puff_state),
                pufferfish_model_root_transform(*instance),
                instance,
                &[EntityModelLayerPass {
                    kind: EntityModelLayerKind::PufferfishBase,
                    render_type: EntityModelLayerRenderType::EntityCutout,
                    model_layer,
                    texture: PUFFERFISH_TEXTURE_REF,
                    visibility: EntityModelLayerVisibility::All,
                    tint: [1.0, 1.0, 1.0, 1.0],
                    order: 0,
                    submit_sequence: 0,
                }],
            )
        }
        EntityModelKind::Turtle { baby } => {
            let texture = if baby {
                TURTLE_BABY_TEXTURE_REF
            } else {
                TURTLE_TEXTURE_REF
            };
            let render_type = if baby {
                EntityModelLayerRenderType::EntityCutoutCull
            } else {
                EntityModelLayerRenderType::EntityCutout
            };
            let has_egg = !baby && instance.render_state.turtle_has_egg;
            let mut transform = entity_model_root_transform(*instance);
            if has_egg {
                transform *= part_pose_transform(TURTLE_EGG_ROOT_DROP_POSE);
            }
            sink.model(
                TurtleModel::new(baby),
                transform,
                instance,
                &[EntityModelLayerPass {
                    kind: EntityModelLayerKind::TurtleBase,
                    render_type,
                    model_layer: if baby {
                        MODEL_LAYER_TURTLE_BABY
                    } else {
                        MODEL_LAYER_TURTLE
                    },
                    texture,
                    visibility: EntityModelLayerVisibility::All,
                    tint: [1.0, 1.0, 1.0, 1.0],
                    order: 0,
                    submit_sequence: 0,
                }],
            )
        }
        EntityModelKind::Dolphin { baby } => {
            let texture = if baby {
                DOLPHIN_BABY_TEXTURE_REF
            } else {
                DOLPHIN_TEXTURE_REF
            };
            let transform = mesh_transformer_scaled_model_root_transform(
                *instance,
                if baby { 0.5 } else { 1.0 },
            );
            sink.model(
                DolphinModel::new(),
                transform,
                instance,
                &[EntityModelLayerPass {
                    kind: EntityModelLayerKind::DolphinBase,
                    render_type: EntityModelLayerRenderType::EntityCutout,
                    model_layer: if baby {
                        MODEL_LAYER_DOLPHIN_BABY
                    } else {
                        MODEL_LAYER_DOLPHIN
                    },
                    texture,
                    visibility: EntityModelLayerVisibility::All,
                    tint: [1.0, 1.0, 1.0, 1.0],
                    order: 0,
                    submit_sequence: 0,
                }],
            )
        }

        // ---- Colored-only uniform (no textured arm): empty passes, textured side is a no-op ----
        EntityModelKind::Guardian { elder } => sink.guardian_model(elder, instance),
        EntityModelKind::Frog { variant } => sink.model(
            FrogModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &frog_textured_layer_passes(variant),
        ),
        EntityModelKind::Creaking { eyes_glowing } => sink.model(
            CreakingModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &creaking_textured_layer_passes(eyes_glowing),
        ),
        EntityModelKind::Sniffer { baby } => sink.model(
            SnifferModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &sniffer_textured_layer_passes(baby),
        ),
        EntityModelKind::Warden => sink.model(
            WardenModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &warden_textured_layer_passes(
                instance.render_state.age_in_ticks,
                instance.render_state.tendril_animation,
                instance.render_state.heart_animation,
            ),
        ),
        EntityModelKind::Armadillo { baby, rolled_up } => sink.model(
            ArmadilloModel::new(baby, rolled_up),
            entity_model_root_transform(*instance),
            instance,
            &armadillo_textured_layer_passes(baby),
        ),
        EntityModelKind::Axolotl { baby, variant } => sink.model(
            AxolotlModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &axolotl_textured_layer_passes(variant, baby),
        ),
        EntityModelKind::Tadpole => sink.model(
            TadpoleModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &tadpole_textured_layer_passes(),
        ),
        EntityModelKind::Parrot { variant } => sink.model(
            ParrotModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &parrot_textured_layer_passes(variant),
        ),
        EntityModelKind::Shulker { color } => sink.model(
            ShulkerModel::new(),
            shulker_model_root_transform(*instance),
            instance,
            &shulker_textured_layer_passes(color),
        ),
        EntityModelKind::Wither => sink.wither_model(instance),
        EntityModelKind::Giant => sink.model(
            ZombieModel::new(false),
            mesh_transformer_scaled_model_root_transform(*instance, GIANT_SCALE),
            instance,
            // Vanilla `GiantMobRenderer` renders a `HumanoidModel` with the plain zombie texture, just
            // scaled by `GIANT_SCALE`; reuse the zombie pass (geometry comes from `ZombieModel`).
            &zombie_textured_layer_passes(false),
        ),
        EntityModelKind::EvokerFangs => sink.model(
            EvokerFangsModel::new(),
            evoker_fangs_model_root_transform(*instance),
            instance,
            &evoker_fangs_textured_layer_passes(),
        ),
        EntityModelKind::LeashKnot => sink.model(
            LeashKnotModel::new(),
            leash_knot_model_root_transform(*instance),
            instance,
            &leash_knot_textured_layer_passes(),
        ),
        EntityModelKind::Arrow { texture } => sink.model(
            ArrowModel::new(),
            arrow_model_root_transform(*instance),
            instance,
            &arrow_textured_layer_passes(texture),
        ),
        EntityModelKind::Trident => sink.trident_model(instance),
        EntityModelKind::WindCharge => {
            // Vanilla `WindChargeRenderer.xOffset(t) = t * 0.03`, passed to `breezeWind(...) % 1`.
            let u_offset = (instance.render_state.age_in_ticks * 0.03).rem_euclid(1.0);
            sink.no_overlay_scrolled_model(
                WindChargeModel::new(),
                wind_charge_model_root_transform(*instance),
                instance,
                &wind_charge_textured_layer_passes(),
                [u_offset, 0.0],
            )
        }
        EntityModelKind::EndCrystal => {
            let passes = end_crystal_textured_layer_passes();
            let body_passes = [passes[0]];
            sink.end_crystal_model(
                end_crystal_model_root_transform(*instance),
                instance,
                &body_passes,
            )
        }
        EntityModelKind::LlamaSpit => sink.model(
            LlamaSpitModel::new(),
            llama_spit_model_root_transform(*instance),
            instance,
            &llama_spit_textured_layer_passes(),
        ),
        EntityModelKind::WitherSkull { dangerous } => sink.model(
            WitherSkullModel::new(),
            wither_skull_model_root_transform(*instance),
            instance,
            &wither_skull_textured_layer_passes(dangerous),
        ),
        // EnderDragonRenderer submits body+eyes, then optional nearest-crystal healing-beam custom
        // geometry; the textured sink owns that full sequence.
        EntityModelKind::EnderDragon => sink.ender_dragon_model(instance),
        EntityModelKind::Mooshroom { baby, variant } => sink.model(
            CowModel::new(CowModelVariant::Temperate, baby),
            entity_model_root_transform(*instance),
            instance,
            // Vanilla `MushroomCowRenderer` renders the cow mesh with the mooshroom recolor; reuse the
            // cow geometry and bind the red/brown mooshroom texture. The adult-only block-mushroom
            // layer is baked separately by the entity-attached block-model path.
            &mooshroom_textured_layer_passes(baby, variant),
        ),
        EntityModelKind::Panda { baby, variant } => sink.model(
            PandaModel::new(baby),
            panda_model_root_transform(*instance),
            instance,
            &panda_textured_layer_passes(variant, baby),
        ),
        EntityModelKind::Nautilus { baby } => sink.nautilus_model(baby, instance),
        EntityModelKind::ZombieNautilus { coral } => sink.zombie_nautilus_model(coral, instance),
        EntityModelKind::Rabbit {
            baby,
            variant,
            toast,
        } => sink.model(
            RabbitModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &rabbit_textured_layer_passes(variant, baby, toast),
        ),
        EntityModelKind::Fox { baby, variant } => sink.model(
            FoxModel::new(baby),
            fox_model_root_transform(*instance),
            instance,
            &fox_textured_layer_passes(variant, baby, instance.render_state.fox_is_sleeping),
        ),
        EntityModelKind::Feline {
            cat,
            baby,
            cat_variant,
            collar,
        } => {
            let transform = if cat && !baby {
                mesh_transformer_scaled_model_root_transform(*instance, FELINE_CAT_SCALE)
            } else {
                entity_model_root_transform(*instance)
            };
            sink.model(
                FelineModel::new(baby),
                transform,
                instance,
                &feline_textured_layer_passes(cat, baby, cat_variant, collar),
            )
        }

        // Everything else is colored-only fallback/debug geometry; the textured path has no residual
        // mesh-emitting arm.
        _ => return false,
    }
    true
}

/// Dispatch-owned renderer layers in the same order vanilla registers them in each renderer
/// constructor. The `SubmitNodeCollector.order(n)` value still controls cross-order traversal; this
/// ordering preserves the same-order append sequence inside each order collection.
pub(in crate::entity_models) fn dispatch_vanilla_entity_layers<S: EntityModelSink>(
    instance: &EntityModelInstance,
    sink: &mut S,
) {
    match instance.kind {
        EntityModelKind::Player { .. } => {
            sink.worn_humanoid_armor(instance);
            sink.custom_head_skull_layer(instance);
            sink.wings_layer(instance);
            sink.player_post_wings_layers(instance);
        }
        EntityModelKind::ArmorStand { .. } => {
            sink.worn_humanoid_armor(instance);
            sink.wings_layer(instance);
            sink.custom_head_skull_layer(instance);
        }
        EntityModelKind::Zombie { .. } | EntityModelKind::Piglin { .. } => {
            sink.custom_head_skull_layer(instance);
            sink.wings_layer(instance);
            sink.worn_humanoid_armor(instance);
        }
        EntityModelKind::ZombieVariant { family, .. } => {
            sink.custom_head_skull_layer(instance);
            sink.wings_layer(instance);
            sink.worn_humanoid_armor(instance);
            if matches!(family, ZombieVariantModelFamily::ZombieVillager) {
                sink.villager_profession_layers(instance);
            }
        }
        EntityModelKind::Skeleton => {
            sink.custom_head_skull_layer(instance);
            sink.wings_layer(instance);
            sink.worn_humanoid_armor(instance);
        }
        EntityModelKind::SkeletonVariant { .. } => {
            sink.custom_head_skull_layer(instance);
            sink.wings_layer(instance);
            sink.worn_humanoid_armor(instance);
            sink.skeleton_clothing_layer(instance);
        }
        EntityModelKind::Giant => {
            sink.worn_humanoid_armor(instance);
        }
        EntityModelKind::Illager { .. }
        | EntityModelKind::WanderingTrader
        | EntityModelKind::CopperGolem { .. } => {
            sink.custom_head_skull_layer(instance);
        }
        EntityModelKind::Villager { .. } => {
            sink.custom_head_skull_layer(instance);
            sink.villager_profession_layers(instance);
        }
        _ => {}
    }
}

/// Dispatch-owned invisible living layers whose own submit paths do not gate on
/// `state.isInvisible`, matching vanilla's layer loop after a null base render type.
pub(in crate::entity_models) fn dispatch_invisible_living_ungated_layers<S: EntityModelSink>(
    instance: &EntityModelInstance,
    sink: &mut S,
    wolf_submit_sequence_start: u32,
) {
    if matches!(instance.kind, EntityModelKind::Wolf { .. }) {
        sink.wolf_body_armor_layer(instance, wolf_submit_sequence_start);
    }
    match instance.kind {
        EntityModelKind::Player { .. } => {
            sink.worn_humanoid_armor(instance);
            sink.custom_head_skull_layer(instance);
            sink.wings_layer(instance);
        }
        EntityModelKind::ArmorStand { .. } => {
            sink.worn_humanoid_armor(instance);
            sink.wings_layer(instance);
            sink.custom_head_skull_layer(instance);
        }
        EntityModelKind::Zombie { .. }
        | EntityModelKind::ZombieVariant { .. }
        | EntityModelKind::Piglin { .. }
        | EntityModelKind::Skeleton
        | EntityModelKind::SkeletonVariant { .. } => {
            sink.custom_head_skull_layer(instance);
            sink.wings_layer(instance);
            sink.worn_humanoid_armor(instance);
        }
        EntityModelKind::Giant => {
            sink.worn_humanoid_armor(instance);
        }
        EntityModelKind::Illager { .. }
        | EntityModelKind::Villager { .. }
        | EntityModelKind::WanderingTrader
        | EntityModelKind::CopperGolem { .. } => {
            sink.custom_head_skull_layer(instance);
        }
        _ => {}
    }
}

fn render_colored_end_crystal_model(
    mesh: &mut EntityModelMesh,
    instance: &EntityModelInstance,
    transform: Mat4,
) {
    // Vanilla `EndCrystalModel.setupAnim`: `base.visible = showsBottom`, the glass stack bobs by
    // `EndCrystalRenderer.getY(age) * 8` model pixels, and the nested glass/core rotate by the two
    // quaternions captured in `end_crystal_glass_quaternions`.
    if instance.render_state.end_crystal_shows_bottom {
        emit_model_part(mesh, &END_CRYSTAL_PARTS[0], transform);
    }

    let age = instance.render_state.age_in_ticks;
    let bob = super::model_layers::end_crystal_bob_y(age);
    let (q_outer, q_inner) = super::model_layers::end_crystal_glass_quaternions(age);
    let centre = transform
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
