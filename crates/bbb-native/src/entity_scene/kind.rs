use super::*;

#[cfg(test)]
pub(super) fn entity_model_kind(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    entity_model_kind_with_registries(entity_type_id, data_values, None)
}

#[cfg(test)]
pub(super) fn entity_model_kind_with_registries(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    variant_registries: Option<&dyn EntityVariantRegistryLookup>,
) -> EntityModelKind {
    entity_model_kind_with_time_and_registries(
        entity_type_id,
        data_values,
        0.0,
        0,
        variant_registries,
    )
}

pub(super) fn entity_model_kind_with_time_and_registries(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    entity_age_ticks: f32,
    game_time: i64,
    variant_registries: Option<&dyn EntityVariantRegistryLookup>,
) -> EntityModelKind {
    if bbb_world::vanilla_entity_renderer_is_noop(entity_type_id) {
        return EntityModelKind::NoRender;
    }
    // Vanilla `ThrownItemRenderer` submits a camera-facing item sprite. The native item-entity
    // billboard layer owns that path, so the 3D model scene suppresses every generated registration.
    if bbb_world::vanilla_entity_renderer_thrown_item_scale(entity_type_id).is_some() {
        return EntityModelKind::NoRender;
    }
    // Vanilla block/item/frame/firework/lightning renderers submit through dedicated native passes,
    // not the entity-model mesh. Resolve their membership from generated `EntityRenderers`
    // registrations so this scene does not retain a parallel entity-type list.
    if bbb_world::vanilla_entity_renderer_uses_native_non_model_pass(entity_type_id) {
        return EntityModelKind::NoRender;
    }
    if let Some(skeleton_kind) = bbb_world::vanilla_entity_renderer_skeleton_kind(entity_type_id) {
        return match skeleton_kind {
            bbb_world::VanillaSkeletonRendererKind::Skeleton => EntityModelKind::Skeleton,
            bbb_world::VanillaSkeletonRendererKind::Stray => EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Stray,
            },
            bbb_world::VanillaSkeletonRendererKind::Parched => EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Parched,
            },
            bbb_world::VanillaSkeletonRendererKind::WitherSkeleton => {
                EntityModelKind::SkeletonVariant {
                    family: SkeletonModelFamily::WitherSkeleton,
                }
            }
            bbb_world::VanillaSkeletonRendererKind::Bogged => EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Bogged {
                    sheared: bogged_sheared(entity_type_id, data_values),
                },
            },
        };
    }
    if let Some(illager_kind) = bbb_world::vanilla_entity_renderer_illager_kind(entity_type_id) {
        return EntityModelKind::Illager {
            family: match illager_kind {
                bbb_world::VanillaIllagerRendererKind::Evoker => IllagerModelFamily::Evoker,
                bbb_world::VanillaIllagerRendererKind::Illusioner => IllagerModelFamily::Illusioner,
                bbb_world::VanillaIllagerRendererKind::Pillager => IllagerModelFamily::Pillager,
                bbb_world::VanillaIllagerRendererKind::Vindicator => IllagerModelFamily::Vindicator,
            },
        };
    }
    if let Some(camel_kind) = bbb_world::vanilla_entity_renderer_camel_kind(entity_type_id) {
        return EntityModelKind::Camel {
            family: match camel_kind {
                bbb_world::VanillaCamelRendererKind::Camel => CamelModelFamily::Camel,
                bbb_world::VanillaCamelRendererKind::CamelHusk => CamelModelFamily::CamelHusk,
            },
            baby: matches!(camel_kind, bbb_world::VanillaCamelRendererKind::Camel)
                && ageable_baby(entity_type_id, data_values),
        };
    }
    if let Some(undead_horse_kind) =
        bbb_world::vanilla_entity_renderer_undead_horse_kind(entity_type_id)
    {
        return undead_horse_model_kind(
            entity_type_id,
            match undead_horse_kind {
                bbb_world::VanillaUndeadHorseRendererKind::Skeleton => {
                    UndeadHorseModelFamily::Skeleton
                }
                bbb_world::VanillaUndeadHorseRendererKind::Zombie => UndeadHorseModelFamily::Zombie,
            },
            data_values,
        );
    }
    if let Some(horse_kind) = bbb_world::vanilla_entity_renderer_horse_kind(entity_type_id) {
        return match horse_kind {
            bbb_world::VanillaHorseRendererKind::Horse => EntityModelKind::Horse {
                baby: ageable_baby(entity_type_id, data_values),
                variant: horse_color_variant(entity_type_id, data_values),
                markings: horse_markings(entity_type_id, data_values),
            },
            bbb_world::VanillaHorseRendererKind::Donkey => {
                donkey_model_kind(entity_type_id, DonkeyModelFamily::Donkey, data_values)
            }
            bbb_world::VanillaHorseRendererKind::Mule => {
                donkey_model_kind(entity_type_id, DonkeyModelFamily::Mule, data_values)
            }
        };
    }
    if let Some(llama_kind) = bbb_world::vanilla_entity_renderer_llama_kind(entity_type_id) {
        return llama_model_kind(
            entity_type_id,
            match llama_kind {
                bbb_world::VanillaLlamaRendererKind::Llama => LlamaModelFamily::Llama,
                bbb_world::VanillaLlamaRendererKind::TraderLlama => LlamaModelFamily::TraderLlama,
            },
            data_values,
        );
    }
    if let Some(boat_kind) = bbb_world::vanilla_entity_renderer_boat_kind(entity_type_id) {
        return boat(
            match boat_kind.family {
                bbb_world::VanillaBoatRendererFamily::Acacia => BoatModelFamily::Acacia,
                bbb_world::VanillaBoatRendererFamily::Bamboo => BoatModelFamily::Bamboo,
                bbb_world::VanillaBoatRendererFamily::Birch => BoatModelFamily::Birch,
                bbb_world::VanillaBoatRendererFamily::Cherry => BoatModelFamily::Cherry,
                bbb_world::VanillaBoatRendererFamily::DarkOak => BoatModelFamily::DarkOak,
                bbb_world::VanillaBoatRendererFamily::Jungle => BoatModelFamily::Jungle,
                bbb_world::VanillaBoatRendererFamily::Mangrove => BoatModelFamily::Mangrove,
                bbb_world::VanillaBoatRendererFamily::Oak => BoatModelFamily::Oak,
                bbb_world::VanillaBoatRendererFamily::PaleOak => BoatModelFamily::PaleOak,
                bbb_world::VanillaBoatRendererFamily::Spruce => BoatModelFamily::Spruce,
            },
            boat_kind.chest,
        );
    }
    if let Some(zombie_kind) = bbb_world::vanilla_entity_renderer_zombie_kind(entity_type_id) {
        return match zombie_kind {
            bbb_world::VanillaZombieRendererKind::Zombie => EntityModelKind::Zombie {
                baby: zombie_baby(entity_type_id, data_values),
            },
            bbb_world::VanillaZombieRendererKind::Husk => EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: zombie_baby(entity_type_id, data_values),
            },
            bbb_world::VanillaZombieRendererKind::Drowned => EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: zombie_baby(entity_type_id, data_values),
            },
            bbb_world::VanillaZombieRendererKind::ZombieVillager => {
                EntityModelKind::ZombieVariant {
                    family: ZombieVariantModelFamily::ZombieVillager,
                    baby: zombie_baby(entity_type_id, data_values),
                }
            }
        };
    }
    if let Some(piglin_kind) = bbb_world::vanilla_entity_renderer_piglin_kind(entity_type_id) {
        return EntityModelKind::Piglin {
            family: match piglin_kind {
                bbb_world::VanillaPiglinRendererKind::Piglin => PiglinModelFamily::Piglin,
                bbb_world::VanillaPiglinRendererKind::PiglinBrute => PiglinModelFamily::PiglinBrute,
                bbb_world::VanillaPiglinRendererKind::ZombifiedPiglin => {
                    PiglinModelFamily::ZombifiedPiglin
                }
            },
            baby: match piglin_kind {
                bbb_world::VanillaPiglinRendererKind::Piglin => {
                    piglin_baby(entity_type_id, data_values)
                }
                bbb_world::VanillaPiglinRendererKind::PiglinBrute => false,
                bbb_world::VanillaPiglinRendererKind::ZombifiedPiglin => {
                    zombie_baby(entity_type_id, data_values)
                }
            },
        };
    }
    if let Some(hoglin_kind) = bbb_world::vanilla_entity_renderer_hoglin_kind(entity_type_id) {
        return EntityModelKind::Hoglin {
            family: match hoglin_kind {
                bbb_world::VanillaHoglinRendererKind::Hoglin => HoglinModelFamily::Hoglin,
                bbb_world::VanillaHoglinRendererKind::Zoglin => HoglinModelFamily::Zoglin,
            },
            baby: ageable_baby(entity_type_id, data_values),
        };
    }
    if let Some(guardian_kind) = bbb_world::vanilla_entity_renderer_guardian_kind(entity_type_id) {
        return EntityModelKind::Guardian {
            elder: matches!(
                guardian_kind,
                bbb_world::VanillaGuardianRendererKind::ElderGuardian
            ),
        };
    }
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHICKEN_ID => {
            chicken_model_kind(entity_type_id, data_values, variant_registries)
        }
        VANILLA_ENTITY_TYPE_PLAYER_ID | VANILLA_ENTITY_TYPE_MANNEQUIN_ID => {
            player_model_kind(entity_type_id, data_values)
        }
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID => armor_stand_model_kind(entity_type_id, data_values),
        VANILLA_ENTITY_TYPE_VILLAGER_ID => EntityModelKind::Villager {
            baby: ageable_baby(entity_type_id, data_values),
        },
        // Vanilla `WanderingTraderRenderer` is a plain `MobRenderer` with
        // `VillagerModel(ModelLayers.WANDERING_TRADER)`, not an `AgeableMobRenderer`.
        // The inherited AgeableMob baby flag therefore does not select a baby
        // villager layer or texture.
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID => EntityModelKind::WanderingTrader,
        VANILLA_ENTITY_TYPE_WITCH_ID => EntityModelKind::Witch,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID => EntityModelKind::Enderman,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => EntityModelKind::IronGolem {
            crackiness: iron_golem_crackiness(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID => EntityModelKind::SnowGolem,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => EntityModelKind::CopperGolem {
            weathering: copper_golem_weathering(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_CREEPER_ID => EntityModelKind::Creeper,
        VANILLA_ENTITY_TYPE_PIG_ID => {
            pig_model_kind(entity_type_id, data_values, variant_registries)
        }
        VANILLA_ENTITY_TYPE_COW_ID => {
            cow_model_kind(entity_type_id, data_values, variant_registries)
        }
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID => mooshroom_model_kind(entity_type_id, data_values),
        VANILLA_ENTITY_TYPE_PANDA_ID => panda_model_kind(entity_type_id, data_values),
        VANILLA_ENTITY_TYPE_SNIFFER_ID => EntityModelKind::Sniffer {
            baby: ageable_baby(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_RAVAGER_ID => EntityModelKind::Ravager,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => EntityModelKind::PolarBear {
            baby: ageable_baby(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_SHEEP_ID => {
            sheep_model_kind(entity_type_id, data_values, entity_age_ticks)
        }
        VANILLA_ENTITY_TYPE_GOAT_ID => goat_model_kind(entity_type_id, data_values),
        VANILLA_ENTITY_TYPE_NAUTILUS_ID => nautilus_model_kind(entity_type_id, data_values),
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
            zombie_nautilus_model_kind(data_values, variant_registries)
        }
        VANILLA_ENTITY_TYPE_WOLF_ID => {
            wolf_model_kind(entity_type_id, data_values, game_time, variant_registries)
        }
        VANILLA_ENTITY_TYPE_FOX_ID => fox_model_kind(entity_type_id, data_values),
        VANILLA_ENTITY_TYPE_CAT_ID => {
            feline_model_kind(entity_type_id, data_values, true, variant_registries)
        }
        VANILLA_ENTITY_TYPE_OCELOT_ID => {
            feline_model_kind(entity_type_id, data_values, false, variant_registries)
        }
        VANILLA_ENTITY_TYPE_RABBIT_ID => rabbit_model_kind(entity_type_id, data_values),
        _ if bbb_world::vanilla_entity_type_is_minecart(entity_type_id) => {
            EntityModelKind::Minecart
        }
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID => EntityModelKind::Arrow {
            texture: ArrowModelTexture::Spectral,
        },
        VANILLA_ENTITY_TYPE_ARROW_ID => EntityModelKind::Arrow {
            // Vanilla `TippableArrowRenderer`: `isTipped = getColor() > 0` swaps to `arrow_tipped.png`.
            texture: if vanilla_entity_metadata_arrow_accessors_for_entity_type(entity_type_id)
                .map(|accessors| {
                    entity_data_int(
                        data_values,
                        accessors.effect_color_data_id,
                        accessors.effect_color_default,
                    ) > 0
                })
                .unwrap_or(false)
            {
                ArrowModelTexture::Tipped
            } else {
                ArrowModelTexture::Normal
            },
        },
        VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID => {
            placeholder("todo_block_display_bounds", 1.0, 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID => {
            entity_type_bounds_placeholder("dragon_fireball_entity_type_bounds", 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => EntityModelKind::EnderDragon,
        VANILLA_ENTITY_TYPE_ALLAY_ID => EntityModelKind::Allay,
        VANILLA_ENTITY_TYPE_ARMADILLO_ID => EntityModelKind::Armadillo {
            baby: ageable_baby(entity_type_id, data_values),
            rolled_up: armadillo_rolled_up(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => {
            let variant =
                vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type(entity_type_id)
                    .map(|(data_id, default)| entity_data_int(data_values, data_id, default))
                    .unwrap_or_default();
            EntityModelKind::Axolotl {
                baby: ageable_baby(entity_type_id, data_values),
                variant: AxolotlModelVariant::from_id(variant),
            }
        }
        VANILLA_ENTITY_TYPE_BAT_ID => EntityModelKind::Bat,
        VANILLA_ENTITY_TYPE_BEE_ID => EntityModelKind::Bee {
            baby: ageable_baby(entity_type_id, data_values),
            angry: bee_is_angry(entity_type_id, data_values, game_time),
            has_nectar: bee_has_nectar(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_BLAZE_ID => EntityModelKind::Blaze,
        VANILLA_ENTITY_TYPE_BREEZE_ID => EntityModelKind::Breeze,
        _ if bbb_world::vanilla_entity_type_is_wind_charge(entity_type_id) => {
            EntityModelKind::WindCharge
        }
        VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID => EntityModelKind::CaveSpider,
        VANILLA_ENTITY_TYPE_COD_ID => EntityModelKind::Cod,
        VANILLA_ENTITY_TYPE_CREAKING_ID => EntityModelKind::Creaking {
            eyes_glowing:
                bbb_world::vanilla_entity_metadata_creaking_active_accessor_for_entity_type(
                    entity_type_id,
                )
                .is_some_and(|(data_id, default)| entity_data_bool(data_values, data_id, default)),
        },
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => EntityModelKind::Dolphin {
            baby: ageable_baby(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_ENDERMITE_ID => EntityModelKind::Endermite,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID => EntityModelKind::EndCrystal,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID => EntityModelKind::EvokerFangs,
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID => {
            entity_type_bounds_placeholder("experience_orb_entity_type_bounds", 0.5, 0.5)
        }
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID => {
            entity_type_bounds_placeholder("fishing_bobber_entity_type_bounds", 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_FROG_ID => frog_model_kind(data_values, variant_registries),
        VANILLA_ENTITY_TYPE_GHAST_ID => {
            let charging =
                vanilla_entity_metadata_ghast_charging_accessor_for_entity_type(entity_type_id)
                    .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
                    .unwrap_or_default();
            EntityModelKind::Ghast { charging }
        }
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => EntityModelKind::HappyGhast,
        VANILLA_ENTITY_TYPE_GIANT_ID => EntityModelKind::Giant,
        VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => EntityModelKind::Squid {
            glow: true,
            baby: ageable_baby(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID => {
            placeholder("todo_item_display_bounds", 1.0, 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID => EntityModelKind::LeashKnot,
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID => EntityModelKind::LlamaSpit,
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => EntityModelKind::MagmaCube {
            size: slime_size(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_PAINTING_ID => placeholder("todo_painting_bounds", 1.0, 1.0, 0.0625),
        VANILLA_ENTITY_TYPE_PARROT_ID => parrot_model_kind(entity_type_id, data_values),
        VANILLA_ENTITY_TYPE_PHANTOM_ID => EntityModelKind::Phantom {
            size: phantom_size(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID => EntityModelKind::Pufferfish {
            puff_state: pufferfish_puff_state(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_SALMON_ID => EntityModelKind::Salmon {
            size: salmon_model_size(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_SHULKER_ID => shulker_model_kind(entity_type_id, data_values),
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID => EntityModelKind::ShulkerBullet,
        VANILLA_ENTITY_TYPE_SILVERFISH_ID => EntityModelKind::Silverfish,
        VANILLA_ENTITY_TYPE_SLIME_ID => EntityModelKind::Slime {
            size: slime_size(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_SPIDER_ID => EntityModelKind::Spider,
        VANILLA_ENTITY_TYPE_SQUID_ID => EntityModelKind::Squid {
            glow: false,
            baby: ageable_baby(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_STRIDER_ID => {
            let cold = vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(
                entity_type_id,
            )
            .is_some_and(|(data_id, default)| entity_data_bool(data_values, data_id, default));
            EntityModelKind::Strider {
                baby: ageable_baby(entity_type_id, data_values),
                cold,
            }
        }
        VANILLA_ENTITY_TYPE_TADPOLE_ID => EntityModelKind::Tadpole,
        VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID => {
            placeholder("todo_text_display_bounds", 1.0, 0.5, 0.0625)
        }
        VANILLA_ENTITY_TYPE_TRIDENT_ID => EntityModelKind::Trident,
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => {
            tropical_fish_model_kind(entity_type_id, data_values)
        }
        VANILLA_ENTITY_TYPE_TURTLE_ID => EntityModelKind::Turtle {
            baby: ageable_baby(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_VEX_ID => EntityModelKind::Vex {
            charging: vanilla_entity_metadata_vex_flags_accessor_for_entity_type(entity_type_id)
                .is_some_and(|(data_id, default, charging_mask)| {
                    entity_data_byte(data_values, data_id, default) & charging_mask != 0
                }),
        },
        VANILLA_ENTITY_TYPE_WARDEN_ID => EntityModelKind::Warden,
        VANILLA_ENTITY_TYPE_WITHER_ID => EntityModelKind::Wither,
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID => EntityModelKind::WitherSkull {
            dangerous: vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(
                entity_type_id,
            )
            .is_some_and(|(data_id, default)| entity_data_bool(data_values, data_id, default)),
        },
        _ => placeholder("todo_unknown_entity_type_bounds", 0.75, 0.75, 0.75),
    }
}
