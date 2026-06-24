//! Shared per-entity model/transform selection for the "uniform" entities — those whose BOTH the
//! colored and the textured render is fully described by `(one model, one root transform, its
//! textured layer passes)`. The two render loops (colored [`super::colored`] and textured
//! [`super::textured`]) used to each carry their own `match instance.kind` arm picking the model and
//! transform; [`dispatch_uniform_entity_model`] is now the single source of truth for that selection,
//! emitting through whichever [`EntityModelSink`] (colored or textured) the caller supplies. Entities
//! whose two paths diverge (recolor, two model trees, family helpers, part visibility, single-pass
//! `render_textured_pass` emits, bespoke hand-walks, …) stay out of here and keep their own per-path
//! residual arm.

use glam::Mat4;

use super::catalog::{CowModelVariant, EntityModelKind, EntityModelTextureAtlasLayout};
use super::colored::{
    arrow_model_root_transform, boat_model_root_transform, cave_spider_model_root_transform,
    cod_model_root_transform, creeper_model_root_transform, ender_dragon_model_root_transform,
    entity_model_root_transform, evoker_fangs_model_root_transform, ghast_model_root_transform,
    happy_ghast_model_root_transform, leash_knot_model_root_transform,
    llama_spit_model_root_transform, magma_cube_model_root_transform,
    mesh_transformer_scaled_model_root_transform, phantom_model_root_transform,
    polar_bear_model_root_transform, pufferfish_model_root_transform, salmon_model_root_transform,
    shulker_bullet_model_root_transform, trident_model_root_transform,
    villager_adult_model_root_transform, wind_charge_model_root_transform,
    wither_model_root_transform, wither_skull_model_root_transform, GIANT_SCALE,
};
use super::geometry::{part_pose_transform, EntityModelMesh};
use super::instances::EntityModelInstance;
use super::model::EntityModel;
use super::model_layers::{
    strider_texture_ref, AllayModel, ArmadilloModel, ArmorStandModel, ArrowModel, AxolotlModel,
    BatModel, BeeModel, BlazeModel, BoatModel, BreezeModel, ChickenModel, CodModel, CowModel,
    CreakingModel, CreeperModel, DolphinModel, EnderDragonModel, EndermanModel, EndermiteModel,
    EvokerFangsModel, FelineModel, FoxModel, FrogModel, GhastModel, GoatModel, GuardianModel,
    HappyGhastModel, IllagerModel, IronGolemModel, LeashKnotModel, LlamaSpitModel, MagmaCubeModel,
    MinecartModel, NautilusModel, PandaModel, ParrotModel, PhantomModel, PigModel, PolarBearModel,
    PufferfishModel, RabbitModel, RavagerModel, SalmonModel, ShulkerBulletModel, ShulkerModel,
    SilverfishModel, SnifferModel, SnowGolemModel, SpiderModel, StriderModel, TadpoleModel,
    TridentModel, TurtleModel, VexModel, VillagerModel, WanderingTraderModel, WardenModel,
    WindChargeModel, WitchModel, WitherModel, WitherSkullModel, WolfModel, ZombieModel,
    ALLAY_TEXTURE_REF, ARMOR_STAND_TEXTURE_REF, BAT_TEXTURE_REF, BEE_BABY_TEXTURE_REF,
    BEE_TEXTURE_REF, BREEZE_TEXTURE_REF, COD_TEXTURE_REF, DOLPHIN_BABY_TEXTURE_REF,
    DOLPHIN_TEXTURE_REF, FELINE_CAT_SCALE, GUARDIAN_ELDER_SCALE, PUFFERFISH_TEXTURE_REF,
    TURTLE_BABY_TEXTURE_REF, TURTLE_EGG_ROOT_DROP_POSE, TURTLE_TEXTURE_REF, VEX_TEXTURE_REF,
};
use super::textured::{
    armadillo_textured_layer_passes, arrow_textured_layer_passes, axolotl_textured_layer_passes,
    blaze_textured_layer_passes, boat_textured_layer_passes, chicken_textured_layer_passes,
    cow_textured_layer_passes, creaking_textured_layer_passes, creeper_textured_layer_passes,
    ender_dragon_textured_layer_passes, enderman_textured_layer_passes,
    endermite_textured_layer_passes, evoker_fangs_textured_layer_passes,
    feline_textured_layer_passes, fox_textured_layer_passes, frog_textured_layer_passes,
    ghast_textured_layer_passes, goat_textured_layer_passes, guardian_textured_layer_passes,
    happy_ghast_textured_layer_passes, illager_textured_layer_passes,
    iron_golem_textured_layer_passes, leash_knot_textured_layer_passes,
    llama_spit_textured_layer_passes, magma_cube_textured_layer_passes,
    minecart_textured_layer_passes, mooshroom_textured_layer_passes,
    nautilus_textured_layer_passes, panda_textured_layer_passes, parrot_textured_layer_passes,
    phantom_textured_layer_passes, pig_textured_layer_passes, polar_bear_textured_layer_passes,
    rabbit_textured_layer_passes, ravager_textured_layer_passes, render_textured_layers,
    salmon_textured_layer_passes, shulker_bullet_textured_layer_passes,
    shulker_textured_layer_passes, silverfish_textured_layer_passes, sniffer_textured_layer_passes,
    snow_golem_textured_layer_passes, spider_textured_layer_passes, tadpole_textured_layer_passes,
    trident_textured_layer_passes, villager_textured_layer_passes,
    wandering_trader_textured_layer_passes, warden_textured_layer_passes,
    wind_charge_textured_layer_passes, witch_textured_layer_passes,
    wither_skull_textured_layer_passes, wither_textured_layer_passes, wolf_textured_layer_passes,
    zombie_textured_layer_passes, EntityModelLayerPass, EntityModelLayerRenderType,
    EntityModelTexturedMeshes,
};

/// A render-path-agnostic sink for a "uniform" entity (one model under one root transform, with its
/// textured layer passes). [`dispatch_uniform_entity_model`] drives this; the colored implementation
/// renders the cube tree (ignoring `passes`), the textured implementation walks `passes`. `passes` is
/// empty for colored-only entities, which therefore emit nothing on the textured path.
pub(in crate::entity_models) trait EntityModelSink {
    fn model<M: EntityModel>(
        &mut self,
        model: M,
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
}

/// Emits `instance` through `sink` if it is a "uniform" entity — one whose BOTH colored and textured
/// rendering is fully described by `(one model, one root transform, its textured layer passes)`.
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
        EntityModelKind::MagmaCube { size } => sink.model(
            MagmaCubeModel::new(),
            magma_cube_model_root_transform(*instance, size),
            instance,
            &magma_cube_textured_layer_passes(),
        ),
        EntityModelKind::Ghast => sink.model(
            GhastModel::new(),
            ghast_model_root_transform(*instance),
            instance,
            &ghast_textured_layer_passes(),
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
        EntityModelKind::Ravager => sink.model(
            RavagerModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &ravager_textured_layer_passes(),
        ),
        EntityModelKind::Creeper => sink.model(
            CreeperModel::new(),
            creeper_model_root_transform(*instance),
            instance,
            &creeper_textured_layer_passes(),
        ),
        EntityModelKind::IronGolem => sink.model(
            IronGolemModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &iron_golem_textured_layer_passes(),
        ),
        EntityModelKind::SnowGolem => sink.model(
            SnowGolemModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &snow_golem_textured_layer_passes(),
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
        } => sink.model(
            WolfModel::new(baby, angry),
            entity_model_root_transform(*instance),
            instance,
            &wolf_textured_layer_passes(baby, tame, angry, collar_color),
        ),
        EntityModelKind::Boat { family, chest } => sink.model(
            BoatModel::new(family, chest),
            boat_model_root_transform(*instance),
            instance,
            &boat_textured_layer_passes(family, chest),
        ),
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
            show_arms,
            show_base_plate,
            pose,
        } => sink.model(
            ArmorStandModel::new(small, show_arms, show_base_plate, pose),
            entity_model_root_transform(*instance),
            instance,
            &[EntityModelLayerPass::base(
                EntityModelLayerRenderType::Cutout,
                ARMOR_STAND_TEXTURE_REF,
                [1.0, 1.0, 1.0, 1.0],
            )],
        ),
        EntityModelKind::Vex => sink.model(
            VexModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &[EntityModelLayerPass::base(
                EntityModelLayerRenderType::Translucent,
                VEX_TEXTURE_REF,
                [1.0, 1.0, 1.0, 1.0],
            )],
        ),
        EntityModelKind::Allay => sink.model(
            AllayModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &[EntityModelLayerPass::base(
                EntityModelLayerRenderType::Translucent,
                ALLAY_TEXTURE_REF,
                [1.0, 1.0, 1.0, 1.0],
            )],
        ),
        EntityModelKind::Strider { baby, cold } => sink.model(
            StriderModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &[EntityModelLayerPass::base(
                EntityModelLayerRenderType::Cutout,
                strider_texture_ref(baby, cold),
                [1.0, 1.0, 1.0, 1.0],
            )],
        ),
        EntityModelKind::Bat => sink.model(
            BatModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &[EntityModelLayerPass::base(
                EntityModelLayerRenderType::Cutout,
                BAT_TEXTURE_REF,
                [1.0, 1.0, 1.0, 1.0],
            )],
        ),
        EntityModelKind::Bee { baby } => {
            let texture = if baby {
                BEE_BABY_TEXTURE_REF
            } else {
                BEE_TEXTURE_REF
            };
            sink.model(
                BeeModel::new(baby),
                entity_model_root_transform(*instance),
                instance,
                &[EntityModelLayerPass::base(
                    EntityModelLayerRenderType::Cutout,
                    texture,
                    [1.0, 1.0, 1.0, 1.0],
                )],
            )
        }
        EntityModelKind::Breeze => sink.model(
            BreezeModel::new(),
            entity_model_root_transform(*instance),
            instance,
            &[EntityModelLayerPass::base(
                EntityModelLayerRenderType::Translucent,
                BREEZE_TEXTURE_REF,
                [1.0, 1.0, 1.0, 1.0],
            )],
        ),
        EntityModelKind::Cod => {
            let in_water = instance.render_state.in_water;
            sink.model(
                CodModel::new(),
                cod_model_root_transform(*instance, in_water),
                instance,
                &[EntityModelLayerPass::base(
                    EntityModelLayerRenderType::Cutout,
                    COD_TEXTURE_REF,
                    [1.0, 1.0, 1.0, 1.0],
                )],
            )
        }
        EntityModelKind::Pufferfish { puff_state } => sink.model(
            PufferfishModel::new(puff_state),
            pufferfish_model_root_transform(*instance),
            instance,
            &[EntityModelLayerPass::base(
                EntityModelLayerRenderType::Cutout,
                PUFFERFISH_TEXTURE_REF,
                [1.0, 1.0, 1.0, 1.0],
            )],
        ),
        EntityModelKind::Turtle { baby } => {
            let texture = if baby {
                TURTLE_BABY_TEXTURE_REF
            } else {
                TURTLE_TEXTURE_REF
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
                &[EntityModelLayerPass::base(
                    EntityModelLayerRenderType::Cutout,
                    texture,
                    [1.0, 1.0, 1.0, 1.0],
                )],
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
                &[EntityModelLayerPass::base(
                    EntityModelLayerRenderType::Cutout,
                    texture,
                    [1.0, 1.0, 1.0, 1.0],
                )],
            )
        }

        // ---- Colored-only uniform (no textured arm): empty passes, textured side is a no-op ----
        EntityModelKind::Guardian { elder } => {
            let scale = if elder { GUARDIAN_ELDER_SCALE } else { 1.0 };
            sink.model(
                GuardianModel::new(),
                mesh_transformer_scaled_model_root_transform(*instance, scale),
                instance,
                &guardian_textured_layer_passes(elder),
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
            &warden_textured_layer_passes(instance.render_state.age_in_ticks),
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
            entity_model_root_transform(*instance),
            instance,
            &shulker_textured_layer_passes(color),
        ),
        EntityModelKind::Wither => sink.model(
            WitherModel::new(),
            wither_model_root_transform(*instance),
            instance,
            &wither_textured_layer_passes(instance.render_state.wither_invulnerable_ticks),
        ),
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
        EntityModelKind::Trident => sink.model(
            TridentModel::new(),
            trident_model_root_transform(*instance),
            instance,
            &trident_textured_layer_passes(),
        ),
        EntityModelKind::LlamaSpit => sink.model(
            LlamaSpitModel::new(),
            llama_spit_model_root_transform(*instance),
            instance,
            &llama_spit_textured_layer_passes(),
        ),
        EntityModelKind::WitherSkull => sink.model(
            WitherSkullModel::new(),
            wither_skull_model_root_transform(*instance),
            instance,
            &wither_skull_textured_layer_passes(),
        ),
        EntityModelKind::ShulkerBullet => sink.model(
            ShulkerBulletModel::new(),
            shulker_bullet_model_root_transform(*instance),
            instance,
            &shulker_bullet_textured_layer_passes(),
        ),
        EntityModelKind::WindCharge => sink.model(
            WindChargeModel::new(),
            wind_charge_model_root_transform(*instance),
            instance,
            &wind_charge_textured_layer_passes(),
        ),
        EntityModelKind::EnderDragon => sink.model(
            EnderDragonModel::new(),
            ender_dragon_model_root_transform(*instance),
            instance,
            &ender_dragon_textured_layer_passes(),
        ),
        EntityModelKind::Mooshroom { baby } => sink.model(
            CowModel::new(CowModelVariant::Temperate, baby),
            entity_model_root_transform(*instance),
            instance,
            // Vanilla `MushroomCowRenderer` renders the cow mesh with the mooshroom recolor; reuse the
            // cow geometry and bind the mooshroom texture (the block-mushroom layer stays deferred).
            &mooshroom_textured_layer_passes(baby),
        ),
        EntityModelKind::Panda { baby, variant } => sink.model(
            PandaModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &panda_textured_layer_passes(variant, baby),
        ),
        EntityModelKind::Nautilus { baby } => sink.model(
            NautilusModel::new(baby),
            entity_model_root_transform(*instance),
            instance,
            &nautilus_textured_layer_passes(baby),
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
            entity_model_root_transform(*instance),
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
