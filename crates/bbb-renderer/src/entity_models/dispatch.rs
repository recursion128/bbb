//! Shared per-entity model/transform selection for the "uniform" entities — those whose BOTH the
//! colored and the textured render is fully described by a small fixed set of model/root-transform/
//! textured-layer-pass tuples. The two render loops (colored [`super::colored`] and textured
//! [`super::textured`]) used to each carry their own `match instance.kind` arm picking the model and
//! transform; [`dispatch_uniform_entity_model`] is now the single source of truth for that selection,
//! emitting through whichever [`EntityModelSink`] (colored or textured) the caller supplies. Entities
//! whose two paths still diverge in model structure, part visibility, single-pass `render_textured_pass`
//! emits, bespoke hand-walks, or custom layer walkers stay out of here and keep their own per-path
//! residual arm. Scroll render types can still be dispatched when the model/root/pass tuple is
//! otherwise uniform; the textured sink folds them into scroll buckets after recording submission
//! metadata.

use glam::{Mat4, Vec3};

use super::catalog::{
    CamelModelFamily, CowModelVariant, EntityModelKind, EntityModelTextureAtlasLayout,
    PiglinModelFamily, SkeletonModelFamily, ZombieVariantModelFamily,
};
use super::colored::{
    arrow_model_root_transform, boat_model_root_transform, camel_model_color,
    cave_spider_model_root_transform, cod_model_root_transform, creeper_model_root_transform,
    end_crystal_model_root_transform, ender_dragon_model_root_transform,
    entity_model_root_transform, evoker_fangs_model_root_transform, fox_model_root_transform,
    ghast_model_root_transform, happy_ghast_model_root_transform, hoglin_model_color,
    iron_golem_model_root_transform, leash_knot_model_root_transform, llama_model_color,
    llama_spit_model_root_transform, magma_cube_model_root_transform,
    mesh_transformer_scaled_model_root_transform, panda_model_root_transform,
    phantom_model_root_transform, piglin_model_color, polar_bear_model_root_transform,
    pufferfish_model_root_transform, salmon_model_root_transform,
    shulker_bullet_model_root_transform, shulker_model_root_transform, slime_model_root_transform,
    squid_model_root_transform, trident_model_root_transform, tropical_fish_model_root_transform,
    villager_adult_model_root_transform, wind_charge_model_root_transform,
    wither_model_root_transform, wither_skeleton_model_root_transform,
    wither_skull_model_root_transform, zombie_variant_color, zombie_variant_root_transform,
    GIANT_SCALE,
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
    PiglinModel, PolarBearModel, PufferfishModel, RabbitModel, RavagerModel, SalmonModel,
    SheepFurModel, SheepModel, ShulkerBulletModel, ShulkerModel, SilverfishModel,
    SkeletonClothingModel, SkeletonModel, SlimeModel, SlimeOuterModel, SnifferModel,
    SnowGolemModel, SpiderModel, SquidModel, StriderModel, TadpoleModel, TridentModel,
    TropicalFishModel, TropicalFishPatternModel, TurtleModel, VexModel, VillagerModel,
    WanderingTraderModel, WardenModel, WindChargeModel, WitchModel, WitherModel, WitherSkullModel,
    WolfModel, ZombieModel, ZombieVariantModel, ALLAY_TEXTURE_REF, ARMOR_STAND_TEXTURE_REF,
    BAT_TEXTURE_REF, COD_TEXTURE_REF, DOLPHIN_BABY_TEXTURE_REF, DOLPHIN_TEXTURE_REF,
    END_CRYSTAL_PARTS, FELINE_CAT_SCALE, GLOW_SQUID_TEAL, GUARDIAN_ELDER_SCALE, MODEL_LAYER_ALLAY,
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
    ravager_textured_layer_passes, render_end_crystal_textured_layers,
    render_no_overlay_scrolled_textured_layers, render_textured_layers,
    salmon_textured_layer_passes, sheep_textured_layer_passes,
    shulker_bullet_textured_layer_passes, shulker_textured_layer_passes,
    silverfish_textured_layer_passes, skeleton_textured_layer_passes, slime_textured_layer_passes,
    sniffer_textured_layer_passes, snow_golem_textured_layer_passes, spider_textured_layer_passes,
    squid_textured_layer_passes, tadpole_textured_layer_passes, trident_textured_layer_passes,
    tropical_fish_textured_layer_passes, villager_textured_layer_passes,
    wandering_trader_textured_layer_passes, warden_textured_layer_passes,
    wind_charge_textured_layer_passes, witch_textured_layer_passes,
    wither_skull_textured_layer_passes, wither_textured_layer_passes, wolf_textured_layer_passes,
    zombie_nautilus_textured_layer_passes, zombie_textured_layer_passes,
    zombie_villager_textured_layer_passes, EntityModelLayerKind, EntityModelLayerPass,
    EntityModelLayerRenderType, EntityModelLayerVisibility, EntityModelTexturedMeshes,
};

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
}

/// The textured sink: walk the entity's textured layer passes over the posed tree. Colored-only
/// entities pass an empty slice here, so this emits nothing for them (correct: they produce no
/// textured geometry).
pub(in crate::entity_models) struct TexturedSink<'a> {
    pub(in crate::entity_models) meshes: &'a mut EntityModelTexturedMeshes,
    pub(in crate::entity_models) atlas: &'a EntityModelTextureAtlasLayout,
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
    }
}

/// Emits `instance` through `sink` if it is a "uniform" entity — one whose BOTH colored and textured
/// rendering is fully described by a fixed sequence of model/root-transform/layer-pass tuples.
/// Returns `true` if emitted, `false` for bespoke entities (the caller then renders them through its
/// own path-specific residual arm). This is the single source of truth for uniform entities'
/// model/transform selection, replacing the duplicated arms in the two render matches.
pub(in crate::entity_models) fn dispatch_uniform_entity_model<S: EntityModelSink>(
    instance: &EntityModelInstance,
    sink: &mut S,
) -> bool {
    match instance.kind {
        // ---- Both-uniform (colored + textured), passes from a `*_textured_layer_passes` fn ----
        EntityModelKind::Chicken { variant, baby } => sink.model(
            ChickenModel::new(variant, baby),
            entity_model_root_transform(*instance),
            instance,
            &chicken_textured_layer_passes(variant, baby),
        ),
        EntityModelKind::Pig { variant, baby } => sink.model(
            PigModel::new(variant, baby),
            entity_model_root_transform(*instance),
            instance,
            &pig_textured_layer_passes(variant, baby),
        ),
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
        EntityModelKind::Camel { family, baby } => {
            let transform = entity_model_root_transform(*instance);
            let passes = camel_textured_layer_passes(family, baby);
            if matches!(family, CamelModelFamily::CamelHusk) {
                sink.model_with_colored_override(
                    CamelModel::new(family, baby),
                    transform,
                    instance,
                    &passes,
                    camel_model_color(family),
                );
            } else {
                sink.model(CamelModel::new(family, baby), transform, instance, &passes);
            }
        }
        EntityModelKind::Llama {
            family,
            variant,
            baby,
            has_chest,
        } => {
            let transform = entity_model_root_transform(*instance);
            let passes = llama_textured_layer_passes(variant, baby, has_chest);
            sink.model_with_colored_override(
                LlamaModel::new(baby, has_chest),
                transform,
                instance,
                &passes,
                llama_model_color(family, variant),
            );
        }
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
            if passes.len() > 1 {
                sink.textured_only_model(
                    SkeletonClothingModel::new(Some(family)),
                    transform,
                    instance,
                    &passes[1..2],
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
        EntityModelKind::Creeper => {
            let passes = creeper_textured_layer_passes();
            let body_passes = [passes[0]];
            sink.model(
                CreeperModel::new(),
                creeper_model_root_transform(*instance),
                instance,
                &body_passes,
            )
        }
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
        } => {
            let wet_shade = instance.render_state.wolf_wet_shade;
            sink.model(
                WolfModel::new(baby, angry),
                entity_model_root_transform(*instance),
                instance,
                &wolf_textured_layer_passes(baby, tame, angry, collar_color, variant, wet_shade),
            )
        }
        EntityModelKind::Boat { family, chest } => {
            let passes = boat_textured_layer_passes(family, chest);
            let body_passes = [passes[0]];
            sink.model(
                BoatModel::new(family, chest),
                boat_model_root_transform(*instance),
                instance,
                &body_passes,
            )
        }
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
        EntityModelKind::Strider { baby, cold } => sink.model(
            StriderModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &[EntityModelLayerPass {
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
            }],
        ),
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
        EntityModelKind::Breeze => {
            let passes = breeze_textured_layer_passes();
            let body_passes = [passes[0], passes[2]];
            sink.model(
                BreezeModel::new(),
                entity_model_root_transform(*instance),
                instance,
                &body_passes,
            )
        }
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
        EntityModelKind::Guardian { elder } => {
            let scale = if elder { GUARDIAN_ELDER_SCALE } else { 1.0 };
            let passes = guardian_textured_layer_passes(elder);
            sink.model(
                GuardianModel::new(),
                mesh_transformer_scaled_model_root_transform(*instance, scale),
                instance,
                &passes[0..1],
            )
        }
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
        EntityModelKind::Sniffer => sink.model(
            SnifferModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &sniffer_textured_layer_passes(),
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
        EntityModelKind::Wither => {
            let passes =
                wither_textured_layer_passes(instance.render_state.wither_invulnerable_ticks);
            let body_passes = [passes[0]];
            sink.model(
                WitherModel::new(),
                wither_model_root_transform(*instance),
                instance,
                &body_passes,
            )
        }
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
        EntityModelKind::Trident => {
            let passes = trident_textured_layer_passes();
            let body_passes = [passes[0]];
            sink.model(
                TridentModel::new(),
                trident_model_root_transform(*instance),
                instance,
                &body_passes,
            )
        }
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
        // The shulker bullet is not uniform: vanilla renders the base model, then re-submits the same
        // posed model as a larger translucent shell. Colored/textured residual arms handle that split.
        EntityModelKind::EnderDragon => {
            let passes = ender_dragon_textured_layer_passes();
            sink.model(
                EnderDragonModel::new(),
                ender_dragon_model_root_transform(*instance),
                instance,
                &passes[0..2],
            )
        }
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
        EntityModelKind::Nautilus { baby } => sink.model(
            NautilusModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &nautilus_textured_layer_passes(baby),
        ),
        EntityModelKind::ZombieNautilus { coral } => sink.model(
            if coral {
                NautilusModel::new_coral()
            } else {
                NautilusModel::new(false)
            },
            entity_model_root_transform(*instance),
            instance,
            &zombie_nautilus_textured_layer_passes(coral),
        ),
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

        // Everything else is bespoke: leave it for the caller's per-path residual arm.
        _ => return false,
    }
    true
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
