//! Per-entity model, root transform and layer-pass selection: the single source of truth for every
//! dispatch-owned entity submission, emitted through [`TexturedSink`].

use crate::entity_models::model_layer::ModelLayer;
use glam::{Mat4, Vec3};

use super::catalog::{
    BannerPatternTexture, BoatModelFamily, CamelModelFamily, ConduitModelPart, DonkeyModelFamily,
    EntityCustomHeadSkull, EntityDyeColor, EntityDynamicPlayerSkinAtlasLayout,
    EntityDynamicPlayerTextureAtlasLayout, EntityModelKind, EntityModelOwnedTextureAtlasEntry,
    EntityModelTextureAtlasLayout, EntityPlayerSkin, HorseColorVariant, HorseMarkings,
    IllagerModelFamily, LlamaVariant, PiglinModelFamily, PlayerModelPartVisibility,
    UndeadHorseModelFamily, ZombieVariantModelFamily,
};
use super::equipment::ResolvedEntityEquipmentAssetCatalog;
use super::held_item::illusioner_head_skull_transform;
use super::instances::EntityModelInstance;
use super::model::ModelPart;
use super::model_layers::{
    allay_model, armadillo_model, armor_stand_model, arrow_model, axolotl_model, banner_model,
    bat_model, bee_model, bee_texture_ref, bell_model, blaze_model, boat_model, book_model,
    book_state_openness, breeze_model, camel_model, chest_lid_eased_openness, chest_model,
    chicken_model, cod_model, copper_golem_model, copper_golem_statue_model, cow_model,
    creaking_model, creeper_model, dolphin_model, drowned_outer_model, end_crystal_model,
    ender_dragon_model, enderman_model, endermite_model, evoker_fangs_model, feline_model,
    fox_model, frog_model, ghast_model, giant_model, goat_model, guardian_model, happy_ghast_model,
    hoglin_model, illager_model, iron_golem_model, llama_model, magma_cube_model, nautilus_model,
    panda_model, parrot_model, phantom_model, pig_model, piglin_model, polar_bear_model,
    pufferfish_model, rabbit_model, ravager_model, salmon_model, sheep_fur_model, sheep_model,
    shulker_box_model, shulker_model, sign_model_layer, silverfish_model, skeleton_model,
    sniffer_model, snow_golem_model, spider_model, squid_model, strider_model, strider_texture_ref,
    tadpole_model, tropical_fish_model, tropical_fish_pattern_model, turtle_model, vex_model,
    villager_model, wandering_trader_model, warden_model, wind_charge_model, witch_model,
    wither_model, wolf_model, zombie_model, zombie_variant_model, ALLAY_TEXTURE_REF,
    ARMOR_STAND_TEXTURE_REF, BAT_TEXTURE_REF, COD_TEXTURE_REF, DOLPHIN_BABY_TEXTURE_REF,
    DOLPHIN_TEXTURE_REF, PUFFERFISH_TEXTURE_REF, TURTLE_BABY_TEXTURE_REF, TURTLE_TEXTURE_REF,
    VEX_CHARGING_TEXTURE_REF, VEX_TEXTURE_REF,
};
use super::textured::{
    armadillo_textured_layer_passes, arrow_textured_layer_passes, axolotl_textured_layer_passes,
    banner_textured_layer_passes, bed_model_layer, bed_textured_layer_passes,
    bell_textured_layer_passes, blaze_textured_layer_passes, boat_textured_layer_passes,
    book_textured_layer_passes, breeze_textured_layer_passes, camel_textured_layer_passes,
    chest_textured_layer_passes, chicken_textured_layer_passes, conduit_textured_layer_passes,
    copper_golem_statue_block_layer_passes, copper_golem_textured_layer_passes,
    cow_textured_layer_passes, creaking_textured_layer_passes, creeper_textured_layer_passes,
    decorated_pot_textured_layer_passes, drowned_textured_layer_passes,
    end_crystal_textured_layer_passes, ender_dragon_textured_layer_passes,
    enderman_textured_layer_passes, endermite_textured_layer_passes,
    evoker_fangs_textured_layer_passes, feline_collar_model_layer, feline_model_layer,
    feline_textured_layer_passes, fox_textured_layer_passes, frog_textured_layer_passes,
    ghast_textured_layer_passes, goat_textured_layer_passes, guardian_textured_layer_passes,
    happy_ghast_textured_layer_passes, hoglin_textured_layer_passes, husk_textured_layer_passes,
    illager_textured_layer_passes, iron_golem_textured_layer_passes,
    leash_knot_textured_layer_passes, llama_spit_textured_layer_passes,
    llama_textured_layer_passes, magma_cube_textured_layer_passes, minecart_textured_layer_passes,
    mooshroom_textured_layer_passes, nautilus_textured_layer_passes, panda_textured_layer_passes,
    parrot_textured_layer_passes, phantom_textured_layer_passes, pig_textured_layer_passes,
    piglin_textured_layer_passes, polar_bear_textured_layer_passes, rabbit_textured_layer_passes,
    ravager_textured_layer_passes, render_beacon_beam_model, render_breeze_wind_scroll_model,
    render_camel_saddle_layer, render_charged_creeper_energy_swirl, render_custom_head_skull_layer,
    render_custom_head_skull_layer_at_transform, render_donkey_textured_layers,
    render_dragon_fireball, render_end_crystal_beam, render_end_portal_block_model,
    render_ender_dragon_beam, render_ender_dragon_death_rays, render_equine_body_armor_layer,
    render_equine_saddle_layer, render_experience_orb, render_fishing_hook, render_guardian_beam,
    render_happy_ghast_harness_layer, render_happy_ghast_ropes_layer, render_horse_textured_layers,
    render_llama_decor_layer, render_nautilus_body_armor_layer, render_nautilus_saddle_layer,
    render_no_overlay_scrolled_textured_layers, render_painting_tile, render_pig_saddle_layer,
    render_player_cape_layer, render_player_extra_ears_layer,
    render_player_parrot_on_shoulder_layer, render_player_spin_attack_effect_layer,
    render_player_stinger_layer, render_player_stuck_arrow_layer, render_player_textured_layers,
    render_skeleton_clothing_layer, render_skull_model, render_strider_saddle_layer,
    render_textured_layers, render_textured_layers_with_dynamic_texture,
    render_trident_foil_submission, render_undead_horse_textured_layers,
    render_villager_profession_layers, render_wings_layer, render_wither_energy_swirl,
    render_wolf_body_armor_layer, salmon_textured_layer_passes, sheep_textured_layer_passes,
    shulker_box_textured_layer_passes, shulker_bullet_textured_layer_passes,
    shulker_textured_layer_passes, sign_textured_layer_passes, silverfish_textured_layer_passes,
    skeleton_textured_layer_passes, slime_textured_layer_passes, sniffer_textured_layer_passes,
    snow_golem_textured_layer_passes, spider_textured_layer_passes, squid_textured_layer_passes,
    tadpole_textured_layer_passes, trident_textured_layer_passes,
    tropical_fish_textured_layer_passes, villager_textured_layer_passes,
    wandering_trader_textured_layer_passes, warden_textured_layer_passes,
    wind_charge_textured_layer_passes, witch_textured_layer_passes,
    wither_skull_textured_layer_passes, wither_textured_layer_passes, wolf_textured_layer_passes,
    zombie_nautilus_textured_layer_passes, zombie_textured_layer_passes,
    zombie_villager_textured_layer_passes, EntityModelLayerKind, EntityModelLayerPass,
    EntityModelLayerRenderType, EntityModelTexturedMeshes,
};
use super::transforms::{
    arrow_model_root_transform, banner_model_root_transform, bed_model_root_transform,
    bell_model_root_transform, boat_model_root_transform, chest_model_root_transform,
    cod_model_root_transform, conduit_model_root_transform,
    copper_golem_statue_block_root_transform, creeper_model_root_transform,
    decorated_pot_model_root_transform, enchanting_table_book_model_root_transform,
    end_crystal_model_root_transform, ender_dragon_model_root_transform,
    entity_model_root_transform, evoker_fangs_model_root_transform, feline_model_root_transform,
    fox_model_root_transform, illusioner_model_root_transform, iron_golem_model_root_transform,
    leash_knot_model_root_transform, lectern_book_model_root_transform,
    llama_spit_model_root_transform, magma_cube_model_root_transform,
    minecart_model_root_transform, panda_model_root_transform, phantom_model_root_transform,
    pufferfish_model_root_transform, salmon_model_root_transform, shulker_box_model_root_transform,
    shulker_bullet_model_root_transform, shulker_model_root_transform, sign_model_root_transform,
    skull_block_model_root_transform, slime_model_root_transform, squid_model_root_transform,
    trident_model_root_transform, tropical_fish_model_root_transform,
    wind_charge_model_root_transform, wither_model_root_transform,
    wither_skull_model_root_transform, zombie_variant_root_transform,
};
use super::zombie_nautilus_variants::ZombieNautilusVariantModelType;
use super::{PigModelVariant, WolfModelVariant, ZombieNautilusModelVariant};

fn strider_base_layer_pass(baby: bool, cold: bool) -> EntityModelLayerPass {
    EntityModelLayerPass::new(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        strider_texture_ref(baby, cold),
    )
}

/// The textured sink: walks the entity's layer passes over the posed tree.
pub(in crate::entity_models) struct TexturedSink<'a> {
    pub(in crate::entity_models) meshes: &'a mut EntityModelTexturedMeshes,
    pub(in crate::entity_models) atlas: &'a EntityModelTextureAtlasLayout,
    pub(in crate::entity_models) owned_entries: &'a [EntityModelOwnedTextureAtlasEntry],
    pub(in crate::entity_models) equipment_assets: &'a ResolvedEntityEquipmentAssetCatalog,
    pub(in crate::entity_models) dynamic_player_skin_atlas:
        Option<&'a EntityDynamicPlayerSkinAtlasLayout>,
    pub(in crate::entity_models) dynamic_player_texture_atlas:
        Option<&'a EntityDynamicPlayerTextureAtlasLayout>,
    pub(in crate::entity_models) data_driven_rigs: &'a [super::data_driven::BakedDataDrivenRig],
}

impl TexturedSink<'_> {
    pub(in crate::entity_models) fn model(
        &mut self,
        model: &ModelPart,
        transform: Mat4,
        instance: &EntityModelInstance,
        passes: &[EntityModelLayerPass],
    ) {
        if let EntityModelKind::Banner {
            flag_only, layers, ..
        } = &instance.kind
        {
            // The optional frame, flag, and base-color mask remain in the static entity atlas. Every
            // pattern then selects its own atlas entry while preserving `submitPatterns` order;
            // this avoids forcing one dynamic handle onto the whole banner instance.
            let static_count = if *flag_only { 2 } else { 3 };
            render_textured_layers(
                self.meshes,
                model,
                transform,
                passes.iter().take(static_count).cloned(),
                self.atlas,
            );
            for (layer, pass) in layers
                .iter()
                .flatten()
                .zip(passes.iter().skip(static_count))
            {
                match layer.texture {
                    BannerPatternTexture::Builtin(_) => render_textured_layers(
                        self.meshes,
                        model,
                        transform,
                        std::iter::once(*pass),
                        self.atlas,
                    ),
                    BannerPatternTexture::Runtime(texture) => {
                        render_textured_layers_with_dynamic_texture(
                            self.meshes,
                            model,
                            transform,
                            std::iter::once(*pass),
                            texture,
                            self.atlas,
                            self.dynamic_player_texture_atlas,
                        )
                    }
                }
            }
            return;
        }
        if let Some(dynamic_texture) = instance.dynamic_texture {
            render_textured_layers_with_dynamic_texture(
                self.meshes,
                model,
                transform,
                passes.iter().cloned(),
                dynamic_texture,
                self.atlas,
                self.dynamic_player_texture_atlas,
            );
            return;
        }
        render_textured_layers(
            self.meshes,
            model,
            transform,
            passes.iter().cloned(),
            self.atlas,
        );
    }

    fn no_overlay_scrolled_model(
        &mut self,
        model: &ModelPart,
        transform: Mat4,
        passes: &[EntityModelLayerPass],
        uv_offset: [f32; 2],
    ) {
        render_no_overlay_scrolled_textured_layers(
            self.meshes,
            model,
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
        self.model(&end_crystal_model(instance), transform, instance, passes);
        render_end_crystal_beam(self.meshes, instance, self.atlas);
    }

    fn painting_tile(&mut self, instance: &EntityModelInstance) {
        render_painting_tile(self.meshes, instance, self.dynamic_player_texture_atlas);
    }

    fn experience_orb(&mut self, instance: &EntityModelInstance) {
        render_experience_orb(self.meshes, instance, self.atlas);
    }

    fn dragon_fireball(&mut self, instance: &EntityModelInstance) {
        render_dragon_fireball(self.meshes, instance, self.atlas);
    }

    fn fishing_hook(&mut self, instance: &EntityModelInstance) {
        render_fishing_hook(self.meshes, instance, self.atlas);
    }

    fn ender_dragon_model(&mut self, instance: &EntityModelInstance) {
        let passes = ender_dragon_textured_layer_passes(
            instance.render_state.kind.ender_dragon().death_time,
        );
        self.model(
            &ender_dragon_model(instance),
            ender_dragon_model_root_transform(instance),
            instance,
            &passes[0..2],
        );
        render_ender_dragon_death_rays(self.meshes, instance);
        render_ender_dragon_beam(self.meshes, instance, self.atlas);
    }

    fn guardian_model(&mut self, elder: bool, instance: &EntityModelInstance) {
        let passes = guardian_textured_layer_passes(elder);
        self.model(
            &guardian_model(elder, instance),
            entity_model_root_transform(instance),
            instance,
            &passes[0..1],
        );
        render_guardian_beam(self.meshes, instance, self.atlas);
    }

    fn pig_model(&mut self, variant: PigModelVariant, baby: bool, instance: &EntityModelInstance) {
        self.model(
            &pig_model(super::textured::pig_model_layer(variant, baby), instance),
            entity_model_root_transform(instance),
            instance,
            &pig_textured_layer_passes(variant, baby),
        );
        render_pig_saddle_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
    }

    fn strider_model(&mut self, baby: bool, cold: bool, instance: &EntityModelInstance) {
        self.model(
            &strider_model(baby, instance),
            entity_model_root_transform(instance),
            instance,
            &[strider_base_layer_pass(baby, cold)],
        );
        render_strider_saddle_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
    }

    fn camel_model(
        &mut self,
        family: CamelModelFamily,
        baby: bool,
        instance: &EntityModelInstance,
    ) {
        let transform = entity_model_root_transform(instance);
        let passes = camel_textured_layer_passes(family, baby);
        self.model(
            &camel_model(family, baby, instance),
            transform,
            instance,
            &passes,
        );
        render_camel_saddle_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
    }

    fn llama_model(
        &mut self,
        variant: LlamaVariant,
        baby: bool,
        has_chest: bool,
        instance: &EntityModelInstance,
    ) {
        self.model(
            &llama_model(
                if baby {
                    ModelLayer::LLAMA_BABY
                } else {
                    ModelLayer::LLAMA
                },
                has_chest,
                instance,
            ),
            entity_model_root_transform(instance),
            instance,
            &llama_textured_layer_passes(variant, baby, has_chest),
        );
        render_llama_decor_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
    }

    fn nautilus_model(&mut self, baby: bool, instance: &EntityModelInstance) {
        self.model(
            &nautilus_model(
                if baby {
                    ModelLayer::NAUTILUS_BABY
                } else {
                    ModelLayer::NAUTILUS
                },
                instance,
            ),
            entity_model_root_transform(instance),
            instance,
            &nautilus_textured_layer_passes(baby),
        );
        render_nautilus_body_armor_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
        render_nautilus_saddle_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
    }

    fn happy_ghast_model(&mut self, baby: bool, instance: &EntityModelInstance) {
        self.model(
            &happy_ghast_model(
                if baby {
                    ModelLayer::HAPPY_GHAST_BABY
                } else {
                    ModelLayer::HAPPY_GHAST
                },
                instance,
            ),
            entity_model_root_transform(instance),
            instance,
            &happy_ghast_textured_layer_passes(baby),
        );
    }

    fn happy_ghast_layers(&mut self, instance: &EntityModelInstance) {
        render_happy_ghast_harness_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
        render_happy_ghast_ropes_layer(self.meshes, instance, self.atlas);
    }

    fn zombie_nautilus_model(
        &mut self,
        variant: ZombieNautilusModelVariant,
        instance: &EntityModelInstance,
    ) {
        self.model(
            &nautilus_model(
                match variant.builtin_model_type() {
                    ZombieNautilusVariantModelType::Normal => ModelLayer::ZOMBIE_NAUTILUS,
                    ZombieNautilusVariantModelType::Warm => ModelLayer::ZOMBIE_NAUTILUS_CORAL,
                },
                instance,
            ),
            entity_model_root_transform(instance),
            instance,
            &zombie_nautilus_textured_layer_passes(variant),
        );
        render_nautilus_body_armor_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
        render_nautilus_saddle_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
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
        let transform = entity_model_root_transform(instance);
        let passes = wolf_textured_layer_passes(
            baby,
            tame,
            angry,
            collar_color,
            variant,
            instance.render_state.kind.wolf().wet_shade,
        );
        if self.meshes.current_invisible_base_only() {
            self.model(
                &wolf_model(baby, angry, instance),
                transform,
                instance,
                &passes,
            );
            return;
        }

        let model = wolf_model(baby, angry, instance);
        render_textured_layers(self.meshes, &model, transform, [passes[0]], self.atlas);
        render_wolf_body_armor_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
        if let Some(collar_pass) = passes.get(1) {
            render_textured_layers(self.meshes, &model, transform, [*collar_pass], self.atlas);
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
        render_player_stuck_arrow_layer(self.meshes, instance, self.atlas);
        render_player_extra_ears_layer(
            self.meshes,
            instance,
            self.atlas,
            self.dynamic_player_skin_atlas,
        );
        render_player_cape_layer(
            self.meshes,
            instance,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
    }

    fn player_post_wings_layers(&mut self, instance: &EntityModelInstance) {
        render_player_parrot_on_shoulder_layer(self.meshes, instance, self.atlas);
        render_player_spin_attack_effect_layer(self.meshes, instance, self.atlas);
        render_player_stinger_layer(self.meshes, instance, self.atlas);
    }

    fn player_arrow_layer(&mut self, instance: &EntityModelInstance) {
        render_player_stuck_arrow_layer(self.meshes, instance, self.atlas);
    }

    fn player_spin_attack_effect_layer(&mut self, instance: &EntityModelInstance) {
        render_player_spin_attack_effect_layer(self.meshes, instance, self.atlas);
    }

    fn player_stinger_layer(&mut self, instance: &EntityModelInstance) {
        render_player_stinger_layer(self.meshes, instance, self.atlas);
    }

    fn wolf_body_armor_layer(&mut self, instance: &EntityModelInstance) {
        render_wolf_body_armor_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        )
    }

    fn custom_head_skull_layer(&mut self, instance: &EntityModelInstance) {
        render_custom_head_skull_layer(
            self.meshes,
            instance,
            self.atlas,
            self.dynamic_player_skin_atlas,
        );
    }

    fn skull_block_model(
        &mut self,
        instance: &EntityModelInstance,
        skull: EntityCustomHeadSkull,
        root_transform: Mat4,
    ) {
        render_skull_model(
            self.meshes,
            instance,
            skull,
            root_transform,
            self.atlas,
            self.dynamic_player_skin_atlas,
        );
    }

    fn end_portal_block_model(&mut self, instance: &EntityModelInstance) {
        render_end_portal_block_model(self.meshes, instance, self.atlas);
    }

    fn beacon_beam_model(&mut self, instance: &EntityModelInstance) {
        render_beacon_beam_model(self.meshes, instance, self.atlas);
    }

    fn wings_layer(&mut self, instance: &EntityModelInstance) {
        render_wings_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
    }

    fn skeleton_clothing_layer(&mut self, instance: &EntityModelInstance) {
        render_skeleton_clothing_layer(self.meshes, instance, self.atlas);
    }

    fn villager_profession_layers(&mut self, instance: &EntityModelInstance) {
        render_villager_profession_layers(self.meshes, instance, self.atlas);
    }

    fn horse_model(
        &mut self,
        variant: HorseColorVariant,
        baby: bool,
        markings: HorseMarkings,
        instance: &EntityModelInstance,
    ) {
        render_horse_textured_layers(self.meshes, instance, variant, baby, markings, self.atlas);
        render_equine_body_armor_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
        render_equine_saddle_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
    }

    fn donkey_model(
        &mut self,
        family: DonkeyModelFamily,
        baby: bool,
        has_chest: bool,
        instance: &EntityModelInstance,
    ) {
        render_donkey_textured_layers(self.meshes, instance, family, baby, has_chest, self.atlas);
        render_equine_saddle_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
    }

    fn undead_horse_model(
        &mut self,
        family: UndeadHorseModelFamily,
        baby: bool,
        instance: &EntityModelInstance,
    ) {
        render_undead_horse_textured_layers(self.meshes, instance, family, baby, self.atlas);
        render_equine_body_armor_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
        render_equine_saddle_layer(
            self.meshes,
            instance,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
    }

    fn boat_model(&mut self, family: BoatModelFamily, chest: bool, instance: &EntityModelInstance) {
        let passes = boat_textured_layer_passes(family, chest);
        let body_passes = [passes[0]];
        let mut local = *instance;
        local.position = [0.0; 3];
        self.meshes.current_part_origin = Some(instance.position);
        self.model(
            &boat_model(family, chest, instance),
            boat_model_root_transform(&local),
            instance,
            &body_passes,
        );
        self.meshes.current_part_origin = None;
    }

    fn breeze_model(&mut self, instance: &EntityModelInstance) {
        let passes = breeze_textured_layer_passes();
        let transform = entity_model_root_transform(instance);
        let model = breeze_model(ModelLayer::BREEZE, instance);
        render_textured_layers(self.meshes, &model, transform, [passes[0]], self.atlas);
        render_breeze_wind_scroll_model(self.meshes, instance, self.atlas);
        let eyes = breeze_model(ModelLayer::BREEZE_EYES, instance);
        render_textured_layers(self.meshes, &eyes, transform, [passes[2]], self.atlas);
    }

    fn trident_model(&mut self, instance: &EntityModelInstance) {
        let passes = trident_textured_layer_passes();
        let body_passes = [passes[0]];
        self.model(
            &ModelLayer::TRIDENT.bake(),
            trident_model_root_transform(instance),
            instance,
            &body_passes,
        );
        render_trident_foil_submission(self.meshes, instance, self.atlas);
    }

    fn creeper_model(&mut self, instance: &EntityModelInstance) {
        let passes = creeper_textured_layer_passes();
        let body_passes = [passes[0]];
        self.model(
            &creeper_model(ModelLayer::CREEPER, instance),
            creeper_model_root_transform(instance),
            instance,
            &body_passes,
        );
        render_charged_creeper_energy_swirl(self.meshes, instance, self.atlas);
    }

    fn wither_model(&mut self, instance: &EntityModelInstance) {
        let passes =
            wither_textured_layer_passes(instance.render_state.kind.wither().invulnerable_ticks);
        let body_passes = [passes[0]];
        self.model(
            &wither_model(ModelLayer::WITHER, instance),
            wither_model_root_transform(instance),
            instance,
            &body_passes,
        );
        render_wither_energy_swirl(self.meshes, instance, self.atlas);
    }
}

/// Emits `instance`'s base model submission through `sink`.
pub(in crate::entity_models) fn dispatch_uniform_entity_model(
    instance: &EntityModelInstance,
    sink: &mut TexturedSink<'_>,
) {
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
        EntityModelKind::Chicken { variant, baby } => sink.model(
            &chicken_model(variant, baby, instance),
            entity_model_root_transform(instance),
            instance,
            &chicken_textured_layer_passes(variant, baby),
        ),
        EntityModelKind::Pig { variant, baby } => sink.pig_model(variant, baby, instance),
        EntityModelKind::Cow { variant, baby } => sink.model(
            &cow_model(super::textured::cow_model_layer(variant, baby), instance),
            entity_model_root_transform(instance),
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
            let transform = entity_model_root_transform(instance);
            for pass in sheep_textured_layer_passes(baby, sheared, wool_color, jeb, age_ticks) {
                let passes = [pass];
                match pass.kind {
                    EntityModelLayerKind::InheritOverlay
                    | EntityModelLayerKind::ZeroWhiteOverlay => {
                        sink.model(&sheep_model(baby, instance), transform, instance, &passes)
                    }
                    EntityModelLayerKind::SheepWool => sink.model(
                        &sheep_fur_model(baby, instance),
                        transform,
                        instance,
                        &passes,
                    ),
                    _ => unreachable!("sheep_textured_layer_passes only emits sheep passes"),
                }
            }
        }
        EntityModelKind::MagmaCube { size } => sink.model(
            &magma_cube_model(instance),
            magma_cube_model_root_transform(instance, size),
            instance,
            &magma_cube_textured_layer_passes(),
        ),
        EntityModelKind::Slime { size } => {
            let transform = slime_model_root_transform(instance, size);
            let passes = slime_textured_layer_passes();
            sink.model(
                &ModelLayer::SLIME.bake(),
                transform,
                instance,
                &passes[0..1],
            );
            sink.model(
                &ModelLayer::SLIME_OUTER.bake(),
                transform,
                instance,
                &passes[1..2],
            );
        }
        EntityModelKind::Camel { family, baby } => sink.camel_model(family, baby, instance),
        EntityModelKind::Llama {
            variant,
            baby,
            has_chest,
            ..
        } => sink.llama_model(variant, baby, has_chest, instance),
        EntityModelKind::Squid { glow, baby } => {
            let transform = squid_model_root_transform(instance, baby);
            let passes = squid_textured_layer_passes(glow, baby);
            sink.model(&squid_model(baby, instance), transform, instance, &passes);
        }
        EntityModelKind::TropicalFish {
            shape,
            base_color,
            pattern,
            pattern_color,
        } => {
            let transform =
                tropical_fish_model_root_transform(instance, instance.render_state.in_water);
            let passes =
                tropical_fish_textured_layer_passes(shape, base_color, pattern, pattern_color);
            sink.model(
                &tropical_fish_model(shape, instance),
                transform,
                instance,
                &passes[0..1],
            );
            sink.model(
                &tropical_fish_pattern_model(shape, instance),
                transform,
                instance,
                &passes[1..2],
            );
        }
        EntityModelKind::Hoglin { family, baby } => {
            let transform = entity_model_root_transform(instance);
            let passes = hoglin_textured_layer_passes(family, baby);
            sink.model(&hoglin_model(baby, instance), transform, instance, &passes);
        }
        EntityModelKind::Piglin { family, baby } => {
            let transform = entity_model_root_transform(instance);
            let baby_layout = baby && family != PiglinModelFamily::PiglinBrute;
            let passes = piglin_textured_layer_passes(family, baby_layout);
            sink.model(
                &piglin_model(family, baby, instance),
                transform,
                instance,
                &passes,
            );
        }
        EntityModelKind::ShulkerBullet => {
            let transform = shulker_bullet_model_root_transform(instance);
            let passes = shulker_bullet_textured_layer_passes();
            sink.model(
                &ModelLayer::SHULKER_BULLET.bake(),
                transform,
                instance,
                &passes[0..1],
            );
            sink.model(
                &ModelLayer::SHULKER_BULLET.bake(),
                transform * Mat4::from_scale(Vec3::splat(1.5)),
                instance,
                &passes[1..2],
            );
        }
        EntityModelKind::Skeleton => {
            let transform = entity_model_root_transform(instance);
            let passes = skeleton_textured_layer_passes(None);
            sink.model(
                &skeleton_model(None, instance),
                transform,
                instance,
                &passes[0..1],
            );
        }
        EntityModelKind::SkeletonVariant { family } => {
            let transform = entity_model_root_transform(instance);
            let passes = skeleton_textured_layer_passes(Some(family));
            sink.model(
                &skeleton_model(Some(family), instance),
                transform,
                instance,
                &passes[0..1],
            );
        }
        EntityModelKind::Ghast { charging } => sink.model(
            &ghast_model(instance),
            entity_model_root_transform(instance),
            instance,
            &ghast_textured_layer_passes(charging),
        ),
        EntityModelKind::HappyGhast { baby } => sink.happy_ghast_model(baby, instance),
        EntityModelKind::Blaze => sink.model(
            &blaze_model(instance),
            entity_model_root_transform(instance),
            instance,
            &blaze_textured_layer_passes(),
        ),
        EntityModelKind::Endermite => sink.model(
            &endermite_model(instance),
            entity_model_root_transform(instance),
            instance,
            &endermite_textured_layer_passes(),
        ),
        EntityModelKind::Silverfish => sink.model(
            &silverfish_model(instance),
            entity_model_root_transform(instance),
            instance,
            &silverfish_textured_layer_passes(),
        ),
        EntityModelKind::Zombie { baby } => sink.model(
            &zombie_model(baby, instance),
            entity_model_root_transform(instance),
            instance,
            &zombie_textured_layer_passes(baby),
        ),
        EntityModelKind::ZombieVariant { family, baby } => {
            let transform = zombie_variant_root_transform(instance, family);
            let passes = match family {
                ZombieVariantModelFamily::Husk => husk_textured_layer_passes(baby),
                ZombieVariantModelFamily::Drowned => drowned_textured_layer_passes(baby),
                ZombieVariantModelFamily::ZombieVillager => {
                    zombie_villager_textured_layer_passes(baby)
                }
            };
            sink.model(
                &zombie_variant_model(family, baby, instance),
                transform,
                instance,
                &passes[0..1],
            );
            if family == ZombieVariantModelFamily::Drowned {
                sink.model(
                    &drowned_outer_model(baby, instance),
                    transform,
                    instance,
                    &passes[1..2],
                );
            }
        }
        EntityModelKind::Ravager => sink.model(
            &ravager_model(instance),
            entity_model_root_transform(instance),
            instance,
            &ravager_textured_layer_passes(),
        ),
        EntityModelKind::Creeper => sink.creeper_model(instance),
        EntityModelKind::IronGolem { crackiness } => sink.model(
            &iron_golem_model(instance),
            iron_golem_model_root_transform(instance),
            instance,
            &iron_golem_textured_layer_passes(crackiness),
        ),
        EntityModelKind::SnowGolem => sink.model(
            &snow_golem_model(instance),
            entity_model_root_transform(instance),
            instance,
            &snow_golem_textured_layer_passes(),
        ),
        EntityModelKind::CopperGolem { weathering } => sink.model(
            &copper_golem_model(instance),
            entity_model_root_transform(instance),
            instance,
            &copper_golem_textured_layer_passes(weathering),
        ),
        EntityModelKind::Witch => sink.model(
            &witch_model(instance),
            entity_model_root_transform(instance),
            instance,
            &witch_textured_layer_passes(),
        ),
        EntityModelKind::Minecart => {
            let mut local = *instance;
            local.position = [0.0; 3];
            sink.meshes.current_part_origin = Some(instance.position);
            sink.model(
                &ModelLayer::MINECART.bake(),
                minecart_model_root_transform(&local),
                instance,
                &minecart_textured_layer_passes(),
            );
            sink.meshes.current_part_origin = None;
        }
        EntityModelKind::WanderingTrader => sink.model(
            &wandering_trader_model(instance),
            entity_model_root_transform(instance),
            instance,
            &wandering_trader_textured_layer_passes(),
        ),
        EntityModelKind::Goat {
            baby,
            left_horn,
            right_horn,
        } => sink.model(
            &goat_model(baby, left_horn, right_horn, instance),
            entity_model_root_transform(instance),
            instance,
            &goat_textured_layer_passes(baby),
        ),
        EntityModelKind::Illager {
            family: family @ IllagerModelFamily::Illusioner,
        } if instance.render_state.invisible => {
            let passes = illager_textured_layer_passes(family);
            let model = illager_model(instance, family);
            let head = instance.render_state.custom_head_skull.map(|skull| {
                (
                    skull,
                    model
                        .try_child_attach_transform("head")
                        .expect("illager head"),
                )
            });
            for index in 0..instance.render_state.kind.illager().clone_offsets.len() {
                let root_transform = illusioner_model_root_transform(
                    instance,
                    instance.illusioner_clone_offset(index),
                );
                sink.model(&model, root_transform, instance, &passes);
                if let Some((skull, head)) = head {
                    render_custom_head_skull_layer_at_transform(
                        sink.meshes,
                        instance,
                        skull,
                        illusioner_head_skull_transform(root_transform, head),
                        sink.atlas,
                        sink.dynamic_player_skin_atlas,
                    );
                }
            }
        }
        EntityModelKind::Illager { family } => sink.model(
            &illager_model(instance, family),
            entity_model_root_transform(instance),
            instance,
            &illager_textured_layer_passes(family),
        ),
        EntityModelKind::Villager { baby } => {
            let transform = entity_model_root_transform(instance);
            sink.model(
                &villager_model(baby, instance),
                transform,
                instance,
                &villager_textured_layer_passes(baby),
            )
        }
        EntityModelKind::PolarBear { baby } => {
            let transform = entity_model_root_transform(instance);
            sink.model(
                &polar_bear_model(baby, instance),
                transform,
                instance,
                &polar_bear_textured_layer_passes(baby),
            )
        }
        EntityModelKind::Phantom { size } => sink.model(
            &phantom_model(instance),
            phantom_model_root_transform(instance, size),
            instance,
            &phantom_textured_layer_passes(),
        ),
        EntityModelKind::Salmon { size } => {
            let in_water = instance.render_state.in_water;
            sink.model(
                &salmon_model(size, instance),
                salmon_model_root_transform(instance, in_water),
                instance,
                &salmon_textured_layer_passes(),
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
            &spider_model(false, instance),
            entity_model_root_transform(instance),
            instance,
            &spider_textured_layer_passes(false),
        ),
        EntityModelKind::CaveSpider => sink.model(
            &spider_model(true, instance),
            entity_model_root_transform(instance),
            instance,
            &spider_textured_layer_passes(true),
        ),
        EntityModelKind::Enderman => sink.model(
            &enderman_model(instance),
            entity_model_root_transform(instance),
            instance,
            &enderman_textured_layer_passes(),
        ),

        // Single-pass renderers.
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
            let base_pass = [EntityModelLayerPass::new(
                EntityModelLayerKind::ArmorStandBase,
                render_type,
                ARMOR_STAND_TEXTURE_REF,
            )];
            let base_passes: &[EntityModelLayerPass] = if marker_hidden { &[] } else { &base_pass };
            sink.model(
                &armor_stand_model(small, show_arms, show_base_plate, pose, instance),
                entity_model_root_transform(instance),
                instance,
                base_passes,
            )
        }
        EntityModelKind::Vex { charging } => sink.model(
            &vex_model(instance),
            entity_model_root_transform(instance),
            instance,
            &[EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::EntityTranslucent,
                if charging {
                    VEX_CHARGING_TEXTURE_REF
                } else {
                    VEX_TEXTURE_REF
                },
            )],
        ),
        EntityModelKind::Allay => sink.model(
            &allay_model(instance),
            entity_model_root_transform(instance),
            instance,
            &[EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::EntityTranslucent,
                ALLAY_TEXTURE_REF,
            )],
        ),
        EntityModelKind::Strider { baby, cold } => sink.strider_model(baby, cold, instance),
        EntityModelKind::Bat => sink.model(
            &bat_model(instance),
            entity_model_root_transform(instance),
            instance,
            &[EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::EntityCutoutCull,
                BAT_TEXTURE_REF,
            )],
        ),
        EntityModelKind::Bee {
            baby,
            angry,
            has_nectar,
        } => {
            let texture = bee_texture_ref(baby, angry, has_nectar);
            sink.model(
                &bee_model(baby, instance),
                entity_model_root_transform(instance),
                instance,
                &[EntityModelLayerPass::new(
                    EntityModelLayerKind::InheritOverlay,
                    EntityModelLayerRenderType::EntityCutout,
                    texture,
                )],
            )
        }
        EntityModelKind::Breeze => sink.breeze_model(instance),
        EntityModelKind::Cod => {
            let in_water = instance.render_state.in_water;
            sink.model(
                &cod_model(instance),
                cod_model_root_transform(instance, in_water),
                instance,
                &[EntityModelLayerPass::new(
                    EntityModelLayerKind::InheritOverlay,
                    EntityModelLayerRenderType::EntityCutout,
                    COD_TEXTURE_REF,
                )],
            )
        }
        EntityModelKind::Pufferfish { puff_state } => sink.model(
            &pufferfish_model(puff_state, instance),
            pufferfish_model_root_transform(instance),
            instance,
            &[EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::EntityCutout,
                PUFFERFISH_TEXTURE_REF,
            )],
        ),
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
            let transform = entity_model_root_transform(instance);
            sink.model(
                &turtle_model(baby, instance),
                transform,
                instance,
                &[EntityModelLayerPass::new(
                    EntityModelLayerKind::InheritOverlay,
                    render_type,
                    texture,
                )],
            )
        }
        EntityModelKind::Dolphin { baby } => {
            let texture = if baby {
                DOLPHIN_BABY_TEXTURE_REF
            } else {
                DOLPHIN_TEXTURE_REF
            };
            let transform = entity_model_root_transform(instance);
            sink.model(
                &dolphin_model(baby, instance),
                transform,
                instance,
                &[EntityModelLayerPass::new(
                    EntityModelLayerKind::InheritOverlay,
                    EntityModelLayerRenderType::EntityCutout,
                    texture,
                )],
            )
        }

        EntityModelKind::Guardian { elder } => sink.guardian_model(elder, instance),
        EntityModelKind::Frog { variant } => sink.model(
            &frog_model(instance),
            entity_model_root_transform(instance),
            instance,
            &frog_textured_layer_passes(variant),
        ),
        EntityModelKind::Creaking { eyes_glowing } => sink.model(
            &creaking_model(instance),
            entity_model_root_transform(instance),
            instance,
            &creaking_textured_layer_passes(eyes_glowing),
        ),
        EntityModelKind::Sniffer { baby } => sink.model(
            &sniffer_model(baby, instance),
            entity_model_root_transform(instance),
            instance,
            &sniffer_textured_layer_passes(baby),
        ),
        EntityModelKind::Warden => sink.model(
            &warden_model(instance),
            entity_model_root_transform(instance),
            instance,
            &warden_textured_layer_passes(
                instance.render_state.age_in_ticks,
                instance.render_state.kind.warden().tendril_animation,
                instance.render_state.kind.warden().heart_animation,
            ),
        ),
        EntityModelKind::Armadillo { baby, rolled_up } => sink.model(
            &armadillo_model(baby, rolled_up, instance),
            entity_model_root_transform(instance),
            instance,
            &armadillo_textured_layer_passes(baby),
        ),
        EntityModelKind::Axolotl { baby, variant } => sink.model(
            &axolotl_model(baby, instance),
            entity_model_root_transform(instance),
            instance,
            &axolotl_textured_layer_passes(variant, baby),
        ),
        EntityModelKind::Tadpole => sink.model(
            &tadpole_model(instance),
            entity_model_root_transform(instance),
            instance,
            &tadpole_textured_layer_passes(),
        ),
        EntityModelKind::Parrot { variant } => sink.model(
            &parrot_model(instance),
            entity_model_root_transform(instance),
            instance,
            &parrot_textured_layer_passes(variant),
        ),
        EntityModelKind::Shulker { color } => sink.model(
            &shulker_model(instance),
            shulker_model_root_transform(instance),
            instance,
            &shulker_textured_layer_passes(color),
        ),
        EntityModelKind::Wither => sink.wither_model(instance),
        EntityModelKind::Giant => sink.model(
            &giant_model(instance),
            entity_model_root_transform(instance),
            instance,
            // Vanilla `GiantMobRenderer` renders a `HumanoidModel` with the plain zombie texture, just
            // scaled by `GIANT_SCALE`; reuse the zombie pass (geometry comes from `ZombieModel`).
            &zombie_textured_layer_passes(false),
        ),
        EntityModelKind::EvokerFangs => sink.model(
            &evoker_fangs_model(instance),
            evoker_fangs_model_root_transform(instance),
            instance,
            &evoker_fangs_textured_layer_passes(),
        ),
        EntityModelKind::LeashKnot => sink.model(
            &ModelLayer::LEASH_KNOT.bake(),
            leash_knot_model_root_transform(instance),
            instance,
            &leash_knot_textured_layer_passes(),
        ),
        EntityModelKind::CopperGolemStatueBlock { pose, weathering } => sink.model(
            &copper_golem_statue_model(pose),
            copper_golem_statue_block_root_transform(instance),
            instance,
            &copper_golem_statue_block_layer_passes(weathering),
        ),
        EntityModelKind::Conduit { part } => sink.model(
            &(match part {
                ConduitModelPart::Shell => ModelLayer::CONDUIT_SHELL,
                ConduitModelPart::Cage => ModelLayer::CONDUIT_CAGE,
                ConduitModelPart::OuterWind { .. } | ConduitModelPart::InnerWind { .. } => {
                    ModelLayer::CONDUIT_WIND
                }
                ConduitModelPart::Eye { .. } => ModelLayer::CONDUIT_EYE,
            })
            .bake(),
            conduit_model_root_transform(instance, part),
            instance,
            &conduit_textured_layer_passes(part),
        ),
        EntityModelKind::SkullBlock { skull, attachment } => {
            sink.skull_block_model(
                instance,
                skull,
                skull_block_model_root_transform(instance, attachment),
            );
        }
        EntityModelKind::EndPortalBlock { .. } => sink.end_portal_block_model(instance),
        EntityModelKind::BeaconBeam => sink.beacon_beam_model(instance),
        EntityModelKind::Chest { texture, half } => sink.model(
            &chest_model(
                half,
                chest_lid_eased_openness(instance.block_entity.chest_openness),
            ),
            chest_model_root_transform(instance),
            instance,
            &chest_textured_layer_passes(texture, half),
        ),
        EntityModelKind::Sign { wood, attachment } => sink.model(
            &sign_model_layer(attachment).bake(),
            sign_model_root_transform(instance, attachment),
            instance,
            &sign_textured_layer_passes(wood, attachment),
        ),
        EntityModelKind::Bed { color, part } => sink.model(
            &bed_model_layer(part).bake(),
            bed_model_root_transform(instance),
            instance,
            &bed_textured_layer_passes(color),
        ),
        EntityModelKind::Bell => sink.model(
            &bell_model(instance),
            bell_model_root_transform(instance),
            instance,
            &bell_textured_layer_passes(),
        ),
        EntityModelKind::ShulkerBox { color, facing } => sink.model(
            &shulker_box_model(instance),
            shulker_box_model_root_transform(instance, facing),
            instance,
            &shulker_box_textured_layer_passes(color),
        ),
        EntityModelKind::DecoratedPot {
            back,
            left,
            right,
            front,
        } => sink.model(
            &ModelLayer::DECORATED_POT_BASE.bake_with_children_of(ModelLayer::DECORATED_POT_SIDES),
            decorated_pot_model_root_transform(instance),
            instance,
            &decorated_pot_textured_layer_passes(back, left, right, front),
        ),
        EntityModelKind::Banner {
            wall,
            flag_only,
            base_color,
            layers,
        } => sink.model(
            &banner_model(wall, instance),
            banner_model_root_transform(instance),
            instance,
            &banner_textured_layer_passes(wall, flag_only, base_color, &layers),
        ),
        EntityModelKind::PaintingTile { .. } => sink.painting_tile(instance),
        EntityModelKind::ExperienceOrb { .. } => sink.experience_orb(instance),
        EntityModelKind::DragonFireball => sink.dragon_fireball(instance),
        EntityModelKind::FishingHook => sink.fishing_hook(instance),
        EntityModelKind::EnchantingBook => sink.model(
            &book_model(
                book_state_openness(
                    instance.block_entity.book_progress,
                    instance.block_entity.book_open,
                ),
                instance.block_entity.book_page_flip_1,
                instance.block_entity.book_page_flip_2,
            ),
            enchanting_table_book_model_root_transform(instance),
            instance,
            &book_textured_layer_passes(),
        ),
        EntityModelKind::LecternBook => sink.model(
            &book_model(
                book_state_openness(
                    instance.block_entity.book_progress,
                    instance.block_entity.book_open,
                ),
                instance.block_entity.book_page_flip_1,
                instance.block_entity.book_page_flip_2,
            ),
            lectern_book_model_root_transform(instance),
            instance,
            &book_textured_layer_passes(),
        ),
        EntityModelKind::Arrow { texture } => sink.model(
            &arrow_model(instance),
            arrow_model_root_transform(instance),
            instance,
            &arrow_textured_layer_passes(texture),
        ),
        EntityModelKind::Trident => sink.trident_model(instance),
        EntityModelKind::WindCharge => {
            // Vanilla `WindChargeRenderer.xOffset(t) = t * 0.03`, passed to `breezeWind(...) % 1`.
            let u_offset = (instance.render_state.age_in_ticks * 0.03).rem_euclid(1.0);
            sink.no_overlay_scrolled_model(
                &wind_charge_model(instance),
                wind_charge_model_root_transform(instance),
                &wind_charge_textured_layer_passes(),
                [u_offset, 0.0],
            )
        }
        EntityModelKind::EndCrystal => {
            let passes = end_crystal_textured_layer_passes();
            let body_passes = [passes[0]];
            sink.end_crystal_model(
                end_crystal_model_root_transform(instance),
                instance,
                &body_passes,
            )
        }
        EntityModelKind::LlamaSpit => sink.model(
            &ModelLayer::LLAMA_SPIT.bake(),
            llama_spit_model_root_transform(instance),
            instance,
            &llama_spit_textured_layer_passes(),
        ),
        EntityModelKind::WitherSkull { dangerous } => sink.model(
            &ModelLayer::WITHER_SKULL.bake(),
            wither_skull_model_root_transform(instance),
            instance,
            &wither_skull_textured_layer_passes(dangerous),
        ),
        // EnderDragonRenderer submits body+eyes, optional death rays, then optional nearest-crystal
        // healing-beam custom geometry; the textured sink owns that full sequence.
        EntityModelKind::EnderDragon => sink.ender_dragon_model(instance),
        EntityModelKind::Mooshroom { baby, variant } => sink.model(
            &cow_model(super::textured::mooshroom_model_layer(baby), instance),
            entity_model_root_transform(instance),
            instance,
            // Vanilla `MushroomCowRenderer` renders the cow mesh with the mooshroom recolor; reuse the
            // cow geometry and bind the red/brown mooshroom texture. The adult-only block-mushroom
            // layer is baked separately by the entity-attached block-model path.
            &mooshroom_textured_layer_passes(baby, variant),
        ),
        EntityModelKind::Panda { baby, variant } => sink.model(
            &panda_model(baby, instance),
            panda_model_root_transform(instance),
            instance,
            &panda_textured_layer_passes(variant, baby),
        ),
        EntityModelKind::Nautilus { baby } => sink.nautilus_model(baby, instance),
        EntityModelKind::ZombieNautilus { variant } => {
            sink.zombie_nautilus_model(variant, instance)
        }
        EntityModelKind::Rabbit {
            baby,
            variant,
            toast,
        } => sink.model(
            &rabbit_model(baby, instance),
            entity_model_root_transform(instance),
            instance,
            &rabbit_textured_layer_passes(variant, baby, toast),
        ),
        EntityModelKind::Fox { baby, variant } => sink.model(
            &fox_model(baby, instance),
            fox_model_root_transform(instance),
            instance,
            &fox_textured_layer_passes(variant, baby, instance.render_state.kind.fox().is_sleeping),
        ),
        EntityModelKind::Feline {
            cat,
            baby,
            cat_variant,
            collar,
        } => {
            let transform = feline_model_root_transform(instance);
            let passes = feline_textured_layer_passes(cat, baby, cat_variant, collar);
            sink.model(
                &feline_model(feline_model_layer(cat, baby), baby, instance),
                transform,
                instance,
                &passes[..1],
            );
            if passes.len() > 1 {
                // Vanilla `CatCollarLayer` bakes its own `CAT_COLLAR` / `CAT_BABY_COLLAR` model.
                sink.model(
                    &feline_model(feline_collar_model_layer(baby), baby, instance),
                    transform,
                    instance,
                    &passes[1..],
                );
            }
        }
        EntityModelKind::DataDriven { rig_id, anim_state } => {
            let Some(rig) = sink.data_driven_rigs.get(rig_id as usize) else {
                return;
            };
            let posed = rig.build_posed_root(&instance.render_state, anim_state);
            let passes = [EntityModelLayerPass::new(
                EntityModelLayerKind::InheritOverlay,
                EntityModelLayerRenderType::EntityCutout,
                super::model_layers::ZOMBIE_TEXTURE_REF,
            )];
            sink.model(
                &posed,
                entity_model_root_transform(instance),
                instance,
                &passes,
            );
        }
    }
}

/// Dispatch-owned renderer layers in the same order vanilla registers them in each renderer
/// constructor. The `SubmitNodeCollector.order(n)` value still controls cross-order traversal; this
/// ordering preserves the same-order append sequence inside each order collection.
pub(in crate::entity_models) fn dispatch_vanilla_entity_layers(
    instance: &EntityModelInstance,
    sink: &mut TexturedSink<'_>,
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
        EntityModelKind::Illager {
            family: IllagerModelFamily::Illusioner,
        } if instance.render_state.invisible => {}
        EntityModelKind::Illager { .. }
        | EntityModelKind::WanderingTrader
        | EntityModelKind::CopperGolem { .. } => {
            sink.custom_head_skull_layer(instance);
        }
        EntityModelKind::Villager { .. } => {
            sink.custom_head_skull_layer(instance);
            sink.villager_profession_layers(instance);
        }
        EntityModelKind::HappyGhast { .. } => sink.happy_ghast_layers(instance),
        _ => {}
    }
}

/// Dispatch-owned invisible living layers whose own submit paths do not gate on
/// `state.isInvisible`, matching vanilla's layer loop after a null base render type.
pub(in crate::entity_models) fn dispatch_invisible_living_ungated_layers(
    instance: &EntityModelInstance,
    sink: &mut TexturedSink<'_>,
    body_dispatched: bool,
) {
    if matches!(instance.kind, EntityModelKind::Wolf { .. }) {
        sink.wolf_body_armor_layer(instance);
    }
    match instance.kind {
        EntityModelKind::Player { .. } => {
            sink.worn_humanoid_armor(instance);
            // Force-transparent / outline players already submitted ArrowLayer beside their base
            // model. A completely hidden base bypassed `player_model`, so restore that ungated layer
            // here before the later constructor layers.
            if !body_dispatched {
                sink.player_arrow_layer(instance);
            }
            sink.custom_head_skull_layer(instance);
            sink.wings_layer(instance);
            sink.player_spin_attack_effect_layer(instance);
            sink.player_stinger_layer(instance);
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
        EntityModelKind::HappyGhast { .. } => sink.happy_ghast_layers(instance),
        // Vanilla ungated layers submitted alongside the base model in `dispatch_uniform_entity_model`:
        // when `body_dispatched` is false (completely invisible), submit them directly here.
        EntityModelKind::Spider if !body_dispatched => {
            let passes = spider_textured_layer_passes(false);
            sink.model(
                &spider_model(false, instance),
                entity_model_root_transform(instance),
                instance,
                &passes[1..],
            );
        }
        EntityModelKind::CaveSpider if !body_dispatched => {
            let passes = spider_textured_layer_passes(true);
            sink.model(
                &spider_model(true, instance),
                entity_model_root_transform(instance),
                instance,
                &passes[1..],
            );
        }
        EntityModelKind::Enderman if !body_dispatched => {
            let passes = enderman_textured_layer_passes();
            sink.model(
                &enderman_model(instance),
                entity_model_root_transform(instance),
                instance,
                &passes[1..],
            );
        }
        EntityModelKind::Phantom { size } if !body_dispatched => {
            let passes = phantom_textured_layer_passes();
            sink.model(
                &phantom_model(instance),
                phantom_model_root_transform(instance, size),
                instance,
                &passes[1..],
            );
        }
        EntityModelKind::Creaking { eyes_glowing: true } if !body_dispatched => {
            let passes = creaking_textured_layer_passes(true);
            sink.model(
                &creaking_model(instance),
                entity_model_root_transform(instance),
                instance,
                &passes[1..],
            );
        }
        EntityModelKind::Breeze if !body_dispatched => {
            let passes = breeze_textured_layer_passes();
            let transform = entity_model_root_transform(instance);
            render_breeze_wind_scroll_model(sink.meshes, instance, sink.atlas);
            let eyes = breeze_model(ModelLayer::BREEZE_EYES, instance);
            render_textured_layers(sink.meshes, &eyes, transform, [passes[2]], sink.atlas);
        }
        EntityModelKind::Creeper if !body_dispatched => {
            render_charged_creeper_energy_swirl(sink.meshes, instance, sink.atlas);
        }
        EntityModelKind::Wither if !body_dispatched => {
            render_wither_energy_swirl(sink.meshes, instance, sink.atlas);
        }
        EntityModelKind::Guardian { .. } if !body_dispatched => {
            render_guardian_beam(sink.meshes, instance, sink.atlas);
        }
        EntityModelKind::Horse { .. } | EntityModelKind::UndeadHorse { .. } if !body_dispatched => {
            render_equine_body_armor_layer(
                sink.meshes,
                instance,
                sink.atlas,
                sink.owned_entries,
                sink.equipment_assets,
                sink.dynamic_player_texture_atlas,
            );
            render_equine_saddle_layer(
                sink.meshes,
                instance,
                sink.atlas,
                sink.owned_entries,
                sink.equipment_assets,
                sink.dynamic_player_texture_atlas,
            );
        }
        EntityModelKind::Donkey { .. } if !body_dispatched => {
            render_equine_saddle_layer(
                sink.meshes,
                instance,
                sink.atlas,
                sink.owned_entries,
                sink.equipment_assets,
                sink.dynamic_player_texture_atlas,
            );
        }
        EntityModelKind::Pig { .. } if !body_dispatched => {
            render_pig_saddle_layer(
                sink.meshes,
                instance,
                sink.atlas,
                sink.owned_entries,
                sink.equipment_assets,
                sink.dynamic_player_texture_atlas,
            );
        }
        EntityModelKind::Strider { .. } if !body_dispatched => {
            render_strider_saddle_layer(
                sink.meshes,
                instance,
                sink.atlas,
                sink.owned_entries,
                sink.equipment_assets,
                sink.dynamic_player_texture_atlas,
            );
        }
        EntityModelKind::Camel { .. } if !body_dispatched => {
            render_camel_saddle_layer(
                sink.meshes,
                instance,
                sink.atlas,
                sink.owned_entries,
                sink.equipment_assets,
                sink.dynamic_player_texture_atlas,
            );
        }
        EntityModelKind::Nautilus { .. } | EntityModelKind::ZombieNautilus { .. }
            if !body_dispatched =>
        {
            render_nautilus_body_armor_layer(
                sink.meshes,
                instance,
                sink.atlas,
                sink.owned_entries,
                sink.equipment_assets,
                sink.dynamic_player_texture_atlas,
            );
            render_nautilus_saddle_layer(
                sink.meshes,
                instance,
                sink.atlas,
                sink.owned_entries,
                sink.equipment_assets,
                sink.dynamic_player_texture_atlas,
            );
        }
        EntityModelKind::Llama { .. } if !body_dispatched => {
            render_llama_decor_layer(
                sink.meshes,
                instance,
                sink.atlas,
                sink.owned_entries,
                sink.equipment_assets,
                sink.dynamic_player_texture_atlas,
            );
        }
        _ => {}
    }
}
