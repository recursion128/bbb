use super::*;

pub(super) fn entity_model_kind(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    entity_model_kind_with_registries(
        entity_type_id,
        data_values,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}

pub(super) fn entity_model_kind_with_registries(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    chicken_variants: Option<&RegistryContentState>,
    cow_variants: Option<&RegistryContentState>,
    pig_variants: Option<&RegistryContentState>,
    frog_variants: Option<&RegistryContentState>,
    cat_variants: Option<&RegistryContentState>,
    wolf_variants: Option<&RegistryContentState>,
) -> EntityModelKind {
    entity_model_kind_with_time_and_registries(
        entity_type_id,
        data_values,
        0.0,
        0,
        chicken_variants,
        cow_variants,
        pig_variants,
        frog_variants,
        cat_variants,
        wolf_variants,
    )
}

pub(super) fn entity_model_kind_with_time_and_registries(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    entity_age_ticks: f32,
    game_time: i64,
    chicken_variants: Option<&RegistryContentState>,
    cow_variants: Option<&RegistryContentState>,
    pig_variants: Option<&RegistryContentState>,
    frog_variants: Option<&RegistryContentState>,
    cat_variants: Option<&RegistryContentState>,
    wolf_variants: Option<&RegistryContentState>,
) -> EntityModelKind {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHICKEN_ID => chicken_model_kind(data_values, chicken_variants),
        VANILLA_ENTITY_TYPE_PLAYER_ID | VANILLA_ENTITY_TYPE_MANNEQUIN_ID => {
            player_model_kind(entity_type_id, data_values)
        }
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID => armor_stand_model_kind(data_values),
        VANILLA_ENTITY_TYPE_ZOMBIE_ID => EntityModelKind::Zombie {
            baby: zombie_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_HUSK_ID => EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: zombie_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_DROWNED_ID => EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: zombie_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: zombie_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: zombie_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_PIGLIN_ID => EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: piglin_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_SKELETON_ID => EntityModelKind::Skeleton,
        VANILLA_ENTITY_TYPE_STRAY_ID => EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Stray,
        },
        VANILLA_ENTITY_TYPE_PARCHED_ID => EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Parched,
        },
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID => EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::WitherSkeleton,
        },
        VANILLA_ENTITY_TYPE_BOGGED_ID => EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged {
                sheared: bogged_sheared(data_values),
            },
        },
        VANILLA_ENTITY_TYPE_VILLAGER_ID => EntityModelKind::Villager {
            baby: ageable_baby(data_values),
        },
        // Vanilla `WanderingTraderRenderer` is a plain `MobRenderer` with
        // `VillagerModel(ModelLayers.WANDERING_TRADER)`, not an `AgeableMobRenderer`.
        // The inherited AgeableMob baby flag therefore does not select a baby
        // villager layer or texture.
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID => EntityModelKind::WanderingTrader,
        VANILLA_ENTITY_TYPE_EVOKER_ID => EntityModelKind::Illager {
            family: IllagerModelFamily::Evoker,
        },
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID => EntityModelKind::Illager {
            family: IllagerModelFamily::Illusioner,
        },
        VANILLA_ENTITY_TYPE_PILLAGER_ID => EntityModelKind::Illager {
            family: IllagerModelFamily::Pillager,
        },
        VANILLA_ENTITY_TYPE_VINDICATOR_ID => EntityModelKind::Illager {
            family: IllagerModelFamily::Vindicator,
        },
        VANILLA_ENTITY_TYPE_WITCH_ID => EntityModelKind::Witch,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID => EntityModelKind::Enderman,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => EntityModelKind::IronGolem {
            crackiness: iron_golem_crackiness(data_values),
        },
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID => EntityModelKind::SnowGolem,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => EntityModelKind::CopperGolem {
            weathering: copper_golem_weathering(data_values),
        },
        VANILLA_ENTITY_TYPE_CREEPER_ID => EntityModelKind::Creeper,
        VANILLA_ENTITY_TYPE_PIG_ID => pig_model_kind(data_values, pig_variants),
        VANILLA_ENTITY_TYPE_COW_ID => cow_model_kind(data_values, cow_variants),
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID => mooshroom_model_kind(data_values),
        VANILLA_ENTITY_TYPE_PANDA_ID => panda_model_kind(data_values),
        VANILLA_ENTITY_TYPE_SNIFFER_ID => EntityModelKind::Sniffer {
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_RAVAGER_ID => EntityModelKind::Ravager,
        VANILLA_ENTITY_TYPE_HOGLIN_ID => EntityModelKind::Hoglin {
            family: HoglinModelFamily::Hoglin,
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_ZOGLIN_ID => EntityModelKind::Hoglin {
            family: HoglinModelFamily::Zoglin,
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => EntityModelKind::PolarBear {
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_SHEEP_ID => sheep_model_kind(data_values, entity_age_ticks),
        VANILLA_ENTITY_TYPE_HORSE_ID => EntityModelKind::Horse {
            baby: ageable_baby(data_values),
            variant: horse_color_variant(data_values),
            markings: horse_markings(data_values),
        },
        VANILLA_ENTITY_TYPE_DONKEY_ID => donkey_model_kind(DonkeyModelFamily::Donkey, data_values),
        VANILLA_ENTITY_TYPE_MULE_ID => donkey_model_kind(DonkeyModelFamily::Mule, data_values),
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => {
            undead_horse_model_kind(UndeadHorseModelFamily::Skeleton, data_values)
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => {
            undead_horse_model_kind(UndeadHorseModelFamily::Zombie, data_values)
        }
        VANILLA_ENTITY_TYPE_CAMEL_ID => EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID => EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            baby: false,
        },
        VANILLA_ENTITY_TYPE_LLAMA_ID => llama_model_kind(LlamaModelFamily::Llama, data_values),
        VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => {
            llama_model_kind(LlamaModelFamily::TraderLlama, data_values)
        }
        VANILLA_ENTITY_TYPE_GOAT_ID => goat_model_kind(data_values),
        VANILLA_ENTITY_TYPE_NAUTILUS_ID => nautilus_model_kind(data_values),
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => zombie_nautilus_model_kind(data_values),
        VANILLA_ENTITY_TYPE_WOLF_ID => wolf_model_kind(data_values, game_time, wolf_variants),
        VANILLA_ENTITY_TYPE_FOX_ID => fox_model_kind(data_values),
        VANILLA_ENTITY_TYPE_CAT_ID => feline_model_kind(data_values, true, cat_variants),
        VANILLA_ENTITY_TYPE_OCELOT_ID => feline_model_kind(data_values, false, cat_variants),
        VANILLA_ENTITY_TYPE_RABBIT_ID => rabbit_model_kind(data_values),
        VANILLA_ENTITY_TYPE_MINECART_ID
        | VANILLA_ENTITY_TYPE_CHEST_MINECART_ID
        | VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID
        | VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
        | VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID
        | VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID
        | VANILLA_ENTITY_TYPE_TNT_MINECART_ID => EntityModelKind::Minecart,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID => EntityModelKind::Arrow {
            texture: ArrowModelTexture::Spectral,
        },
        VANILLA_ENTITY_TYPE_ARROW_ID => EntityModelKind::Arrow {
            // Vanilla `TippableArrowRenderer`: `isTipped = getColor() > 0` swaps to `arrow_tipped.png`.
            texture: if entity_data_int(data_values, ARROW_EFFECT_COLOR_DATA_ID, -1) > 0 {
                ArrowModelTexture::Tipped
            } else {
                ArrowModelTexture::Normal
            },
        },
        VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID => {
            placeholder("todo_block_display_bounds", 1.0, 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID => {
            placeholder("todo_dragon_fireball_bounds", 1.0, 1.0, 1.0)
        }
        // Thrown-item projectiles (vanilla `ThrownItemRenderer`) render as a camera-facing item sprite,
        // emitted by the item-entity billboard layer (`thrown_item_projectile_billboards_from_world`),
        // so the 3D model scene draws nothing for them.
        VANILLA_ENTITY_TYPE_EGG_ID | VANILLA_ENTITY_TYPE_SNOWBALL_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => EntityModelKind::EnderDragon,
        VANILLA_ENTITY_TYPE_ENDER_PEARL_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID => boat(BoatModelFamily::Acacia, false),
        VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID => boat(BoatModelFamily::Acacia, true),
        VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID => boat(BoatModelFamily::Bamboo, false),
        VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID => boat(BoatModelFamily::Bamboo, true),
        VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID => boat(BoatModelFamily::Birch, false),
        VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID => boat(BoatModelFamily::Birch, true),
        VANILLA_ENTITY_TYPE_CHERRY_BOAT_ID => boat(BoatModelFamily::Cherry, false),
        VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID => boat(BoatModelFamily::Cherry, true),
        VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID => boat(BoatModelFamily::DarkOak, false),
        VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID => boat(BoatModelFamily::DarkOak, true),
        VANILLA_ENTITY_TYPE_JUNGLE_BOAT_ID => boat(BoatModelFamily::Jungle, false),
        VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID => boat(BoatModelFamily::Jungle, true),
        VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID => boat(BoatModelFamily::Mangrove, false),
        VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID => boat(BoatModelFamily::Mangrove, true),
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID => boat(BoatModelFamily::Oak, false),
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID => boat(BoatModelFamily::Oak, true),
        VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID => boat(BoatModelFamily::PaleOak, false),
        VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID => boat(BoatModelFamily::PaleOak, true),
        VANILLA_ENTITY_TYPE_SPRUCE_BOAT_ID => boat(BoatModelFamily::Spruce, false),
        VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID => boat(BoatModelFamily::Spruce, true),
        VANILLA_ENTITY_TYPE_ALLAY_ID => EntityModelKind::Allay,
        VANILLA_ENTITY_TYPE_ARMADILLO_ID => EntityModelKind::Armadillo {
            baby: ageable_baby(data_values),
            rolled_up: armadillo_rolled_up(data_values),
        },
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => EntityModelKind::Axolotl {
            baby: ageable_baby(data_values),
            variant: AxolotlModelVariant::from_id(entity_data_int(
                data_values,
                AXOLOTL_VARIANT_DATA_ID,
                0,
            )),
        },
        VANILLA_ENTITY_TYPE_BAT_ID => EntityModelKind::Bat,
        VANILLA_ENTITY_TYPE_BEE_ID => EntityModelKind::Bee {
            baby: ageable_baby(data_values),
            angry: bee_is_angry(entity_type_id, data_values, game_time),
            has_nectar: bee_has_nectar(entity_type_id, data_values),
        },
        VANILLA_ENTITY_TYPE_BLAZE_ID => EntityModelKind::Blaze,
        VANILLA_ENTITY_TYPE_BREEZE_ID => EntityModelKind::Breeze,
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID => EntityModelKind::WindCharge,
        VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID => EntityModelKind::CaveSpider,
        VANILLA_ENTITY_TYPE_COD_ID => EntityModelKind::Cod,
        VANILLA_ENTITY_TYPE_CREAKING_ID => EntityModelKind::Creaking {
            eyes_glowing: entity_data_bool(data_values, CREAKING_IS_ACTIVE_DATA_ID, false),
        },
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => EntityModelKind::Dolphin {
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID => EntityModelKind::Guardian { elder: true },
        VANILLA_ENTITY_TYPE_ENDERMITE_ID => EntityModelKind::Endermite,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID => EntityModelKind::EndCrystal,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID => EntityModelKind::EvokerFangs,
        // Thrown bottles/potions also render as item sprites via the billboard layer (see above).
        VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID
        | VANILLA_ENTITY_TYPE_SPLASH_POTION_ID
        | VANILLA_ENTITY_TYPE_LINGERING_POTION_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID => {
            placeholder("todo_experience_orb_bounds", 0.5, 0.5, 0.5)
        }
        // The eye of ender also renders as an item sprite via the billboard layer (see above).
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID => {
            placeholder("todo_falling_block_bounds", 0.98, 0.98, 0.98)
        }
        // The large fireball also renders as a (3× scaled) item sprite via the billboard layer.
        VANILLA_ENTITY_TYPE_FIREBALL_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID => {
            placeholder("todo_firework_rocket_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID => {
            placeholder("todo_fishing_bobber_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_FROG_ID => frog_model_kind(data_values, frog_variants),
        VANILLA_ENTITY_TYPE_GHAST_ID => EntityModelKind::Ghast {
            charging: entity_data_bool(data_values, GHAST_IS_CHARGING_DATA_ID, false),
        },
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => EntityModelKind::HappyGhast,
        VANILLA_ENTITY_TYPE_GIANT_ID => EntityModelKind::Giant,
        // Item frames render via the 3D item-model pass (border + framed item, native `item_frames`),
        // not the entity-model scene.
        VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => EntityModelKind::Squid {
            glow: true,
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_GUARDIAN_ID => EntityModelKind::Guardian { elder: false },
        VANILLA_ENTITY_TYPE_INTERACTION_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_ITEM_ID => placeholder("todo_item_entity_bounds", 0.25, 0.25, 0.25),
        VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID => {
            placeholder("todo_item_display_bounds", 1.0, 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID => EntityModelKind::LeashKnot,
        // Lightning bolts render as custom `RenderTypes.lightning()` geometry on the weather target.
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID => EntityModelKind::LlamaSpit,
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => EntityModelKind::MagmaCube {
            size: slime_size(data_values),
        },
        VANILLA_ENTITY_TYPE_MARKER_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID => {
            placeholder("todo_ominous_item_spawner_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_PAINTING_ID => placeholder("todo_painting_bounds", 1.0, 1.0, 0.0625),
        VANILLA_ENTITY_TYPE_PARROT_ID => parrot_model_kind(data_values),
        VANILLA_ENTITY_TYPE_PHANTOM_ID => EntityModelKind::Phantom {
            size: phantom_size(data_values),
        },
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: false,
        },
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID => EntityModelKind::Pufferfish {
            puff_state: pufferfish_puff_state(data_values),
        },
        VANILLA_ENTITY_TYPE_SALMON_ID => EntityModelKind::Salmon {
            size: salmon_model_size(data_values),
        },
        VANILLA_ENTITY_TYPE_SHULKER_ID => shulker_model_kind(data_values),
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID => EntityModelKind::ShulkerBullet,
        VANILLA_ENTITY_TYPE_SILVERFISH_ID => EntityModelKind::Silverfish,
        VANILLA_ENTITY_TYPE_SLIME_ID => EntityModelKind::Slime {
            size: slime_size(data_values),
        },
        // The small fireball also renders as a (0.75× scaled) item sprite via the billboard layer.
        VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_SPIDER_ID => EntityModelKind::Spider,
        VANILLA_ENTITY_TYPE_SQUID_ID => EntityModelKind::Squid {
            glow: false,
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_STRIDER_ID => EntityModelKind::Strider {
            baby: ageable_baby(data_values),
            cold: entity_data_bool(data_values, STRIDER_SUFFOCATING_DATA_ID, false),
        },
        VANILLA_ENTITY_TYPE_TADPOLE_ID => EntityModelKind::Tadpole,
        VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID => {
            placeholder("todo_text_display_bounds", 1.0, 0.5, 0.0625)
        }
        VANILLA_ENTITY_TYPE_TNT_ID => placeholder("todo_tnt_bounds", 0.98, 0.98, 0.98),
        VANILLA_ENTITY_TYPE_TRIDENT_ID => EntityModelKind::Trident,
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => EntityModelKind::TropicalFish {
            shape: tropical_fish_shape(data_values),
            base_color: tropical_fish_base_color(data_values),
            pattern: tropical_fish_pattern(data_values),
            pattern_color: tropical_fish_pattern_color(data_values),
        },
        VANILLA_ENTITY_TYPE_TURTLE_ID => EntityModelKind::Turtle {
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_VEX_ID => EntityModelKind::Vex {
            charging: (entity_data_byte(data_values, VEX_FLAGS_DATA_ID, 0) & VEX_FLAG_IS_CHARGING)
                != 0,
        },
        VANILLA_ENTITY_TYPE_WARDEN_ID => EntityModelKind::Warden,
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID => EntityModelKind::WindCharge,
        VANILLA_ENTITY_TYPE_WITHER_ID => EntityModelKind::Wither,
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID => EntityModelKind::WitherSkull {
            dangerous: entity_data_bool(data_values, WITHER_SKULL_DANGEROUS_DATA_ID, false),
        },
        _ => placeholder("todo_unknown_entity_type_bounds", 0.75, 0.75, 0.75),
    }
}
