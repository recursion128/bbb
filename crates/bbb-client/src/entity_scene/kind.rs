use super::*;

#[cfg(test)]
pub(super) fn entity_model_kind(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    entity_model_kind_with_time(entity_type_id, data_values, 0.0, 0)
}

pub(super) fn entity_model_kind_with_time(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    entity_age_ticks: f32,
    game_time: i64,
) -> EntityModelKind {
    use bbb_client_world::{
        VanillaEntityRendererModelKind as Kind, VanillaEntityRendererPresentationOwner as Owner,
    };

    match bbb_client_world::vanilla_entity_renderer_presentation_owner(entity_type_id) {
        Owner::Unknown
        | Owner::Noop
        | Owner::ThrownItem
        | Owner::BlockAttachment
        | Owner::FireworkItem
        | Owner::ItemFrame
        | Owner::ItemCluster
        | Owner::Lightning
        | Owner::Display
        | Owner::Painting => return EntityModelKind::NoRender,
        Owner::AvatarModel | Owner::EntityModel => {}
    }
    if let Some(kind) = bbb_client_world::vanilla_entity_renderer_model_kind(entity_type_id) {
        return match kind {
            Kind::Skeleton => EntityModelKind::Skeleton,
            Kind::Stray => EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Stray,
            },
            Kind::Parched => EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Parched,
            },
            Kind::WitherSkeleton => EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::WitherSkeleton,
            },
            Kind::Bogged => EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Bogged {
                    sheared: bogged_sheared(entity_type_id, data_values),
                },
            },
            Kind::Evoker => EntityModelKind::Illager {
                family: IllagerModelFamily::Evoker,
            },
            Kind::Illusioner => EntityModelKind::Illager {
                family: IllagerModelFamily::Illusioner,
            },
            Kind::Pillager => EntityModelKind::Illager {
                family: IllagerModelFamily::Pillager,
            },
            Kind::Vindicator => EntityModelKind::Illager {
                family: IllagerModelFamily::Vindicator,
            },
            Kind::Camel => EntityModelKind::Camel {
                family: CamelModelFamily::Camel,
                baby: ageable_baby(entity_type_id, data_values),
            },
            Kind::CamelHusk => EntityModelKind::Camel {
                family: CamelModelFamily::CamelHusk,
                baby: false,
            },
            Kind::UndeadHorse(family) => {
                undead_horse_model_kind(entity_type_id, family, data_values)
            }
            Kind::Horse(kind) => match kind {
                bbb_client_world::VanillaHorseRendererKind::Horse => EntityModelKind::Horse {
                    baby: ageable_baby(entity_type_id, data_values),
                    variant: horse_color_variant(entity_type_id, data_values),
                    markings: horse_markings(entity_type_id, data_values),
                },
                bbb_client_world::VanillaHorseRendererKind::Donkey => {
                    donkey_model_kind(entity_type_id, DonkeyModelFamily::Donkey, data_values)
                }
                bbb_client_world::VanillaHorseRendererKind::Mule => {
                    donkey_model_kind(entity_type_id, DonkeyModelFamily::Mule, data_values)
                }
            },
            Kind::Llama(family) => llama_model_kind(entity_type_id, family, data_values),
            Kind::Boat(kind) => EntityModelKind::Boat {
                family: kind.family,
                chest: kind.chest,
            },
            Kind::Zombie => EntityModelKind::Zombie {
                baby: zombie_baby(entity_type_id, data_values),
            },
            Kind::Husk => EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: zombie_baby(entity_type_id, data_values),
            },
            Kind::Drowned => EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: zombie_baby(entity_type_id, data_values),
            },
            Kind::ZombieVillager => EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: zombie_baby(entity_type_id, data_values),
            },
            Kind::Piglin(family) => EntityModelKind::Piglin {
                family,
                baby: match family {
                    PiglinModelFamily::Piglin => piglin_baby(entity_type_id, data_values),
                    PiglinModelFamily::PiglinBrute => false,
                    PiglinModelFamily::ZombifiedPiglin => zombie_baby(entity_type_id, data_values),
                },
            },
            Kind::Hoglin => EntityModelKind::Hoglin {
                family: HoglinModelFamily::Hoglin,
                baby: ageable_baby(entity_type_id, data_values),
            },
            Kind::Zoglin => EntityModelKind::Hoglin {
                family: HoglinModelFamily::Zoglin,
                baby: ageable_baby(entity_type_id, data_values),
            },
            Kind::Guardian => EntityModelKind::Guardian { elder: false },
            Kind::ElderGuardian => EntityModelKind::Guardian { elder: true },
            Kind::Squid | Kind::GlowSquid => EntityModelKind::Squid {
                glow: matches!(kind, Kind::GlowSquid),
                baby: ageable_baby(entity_type_id, data_values),
            },
        };
    }
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHICKEN_ID => EntityModelKind::Chicken {
            variant: chicken_model_variant(entity_type_id, data_values),
            baby: ageable_baby(entity_type_id, data_values),
        },
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
        VANILLA_ENTITY_TYPE_PIG_ID => EntityModelKind::Pig {
            variant: pig_model_variant(entity_type_id, data_values),
            baby: ageable_baby(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_COW_ID => EntityModelKind::Cow {
            variant: cow_model_variant(entity_type_id, data_values),
            baby: ageable_baby(entity_type_id, data_values),
        },
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
        VANILLA_ENTITY_TYPE_NAUTILUS_ID => EntityModelKind::Nautilus {
            baby: ageable_baby(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => EntityModelKind::ZombieNautilus {
            variant: zombie_nautilus_model_variant(data_values),
        },
        VANILLA_ENTITY_TYPE_WOLF_ID => wolf_model_kind(entity_type_id, data_values, game_time),
        VANILLA_ENTITY_TYPE_FOX_ID => fox_model_kind(entity_type_id, data_values),
        VANILLA_ENTITY_TYPE_CAT_ID => feline_model_kind(entity_type_id, data_values, true),
        VANILLA_ENTITY_TYPE_OCELOT_ID => feline_model_kind(entity_type_id, data_values, false),
        VANILLA_ENTITY_TYPE_RABBIT_ID => rabbit_model_kind(entity_type_id, data_values),
        _ if bbb_client_world::vanilla_entity_type_is_minecart(entity_type_id) => {
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
        VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID => EntityModelKind::DragonFireball,
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
        _ if bbb_client_world::vanilla_entity_type_is_wind_charge(entity_type_id) => {
            EntityModelKind::WindCharge
        }
        VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID => EntityModelKind::CaveSpider,
        VANILLA_ENTITY_TYPE_COD_ID => EntityModelKind::Cod,
        VANILLA_ENTITY_TYPE_CREAKING_ID => EntityModelKind::Creaking {
            eyes_glowing:
                bbb_client_world::vanilla_entity_metadata_creaking_active_accessor_for_entity_type(
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
            let value =
                bbb_client_world::vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type(
                    entity_type_id,
                )
                .map(|(data_id, default)| entity_data_int(data_values, data_id, default))
                .unwrap_or_default();
            EntityModelKind::ExperienceOrb {
                icon: bbb_client_world::experience_orb_icon(value),
            }
        }
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID => EntityModelKind::FishingHook,
        VANILLA_ENTITY_TYPE_FROG_ID => EntityModelKind::Frog {
            variant: frog_model_variant(data_values),
        },
        VANILLA_ENTITY_TYPE_GHAST_ID => {
            let charging =
                vanilla_entity_metadata_ghast_charging_accessor_for_entity_type(entity_type_id)
                    .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
                    .unwrap_or_default();
            EntityModelKind::Ghast { charging }
        }
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => EntityModelKind::HappyGhast {
            baby: ageable_baby(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_GIANT_ID => EntityModelKind::Giant,
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID => EntityModelKind::LeashKnot,
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID => EntityModelKind::LlamaSpit,
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => EntityModelKind::MagmaCube {
            size: slime_size(entity_type_id, data_values),
        },
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
        _ => unreachable!("fixed entity renderer owner lacked native model selection"),
    }
}
