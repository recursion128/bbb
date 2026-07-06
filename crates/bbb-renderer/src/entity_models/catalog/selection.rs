use super::super::model_layers::*;
use super::*;

impl EntityModelKind {
    pub fn model_key(self) -> &'static str {
        match self {
            Self::Chicken { variant, baby } => chicken_model_key(variant, baby),
            Self::Pig { variant, baby } => pig_model_key(variant, baby),
            Self::Player { skin, .. } if skin.is_slim() => "player_slim",
            Self::Player { .. } => "player",
            Self::Humanoid {
                family: HumanoidModelFamily::Player,
                baby: false,
            } => "humanoid_player",
            Self::Humanoid {
                family: HumanoidModelFamily::Player,
                baby: true,
            } => "humanoid_player_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Zombie,
                baby: false,
            } => "humanoid_zombie",
            Self::Humanoid {
                family: HumanoidModelFamily::Zombie,
                baby: true,
            } => "humanoid_zombie_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Skeleton,
                baby: false,
            } => "humanoid_skeleton",
            Self::Humanoid {
                family: HumanoidModelFamily::Skeleton,
                baby: true,
            } => "humanoid_skeleton_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Villager,
                baby: false,
            } => "humanoid_villager",
            Self::Humanoid {
                family: HumanoidModelFamily::Villager,
                baby: true,
            } => "humanoid_villager_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Illager,
                baby: false,
            } => "humanoid_illager",
            Self::Humanoid {
                family: HumanoidModelFamily::Illager,
                baby: true,
            } => "humanoid_illager_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::ArmorStand,
                baby: false,
            } => "humanoid_armor_stand",
            Self::Humanoid {
                family: HumanoidModelFamily::ArmorStand,
                baby: true,
            } => "humanoid_armor_stand_baby",
            Self::ArmorStand { small: false, .. } => "armor_stand",
            Self::ArmorStand { small: true, .. } => "armor_stand_small",
            Self::Slime { .. } => "slime",
            Self::MagmaCube { .. } => "magma_cube",
            Self::Ghast { .. } => "ghast",
            Self::HappyGhast => "happy_ghast",
            Self::Blaze => "blaze",
            Self::Endermite => "endermite",
            Self::Silverfish => "silverfish",
            Self::Vex { .. } => "vex",
            Self::Allay => "allay",
            Self::Strider { baby: false, .. } => "strider",
            Self::Strider { baby: true, .. } => "strider_baby",
            Self::Turtle { baby: false } => "turtle",
            Self::Turtle { baby: true } => "turtle_baby",
            Self::Bat => "bat",
            Self::Bee { baby: false, .. } => "bee",
            Self::Bee { baby: true, .. } => "bee_baby",
            Self::Breeze => "breeze",
            Self::Dolphin { baby: false } => "dolphin",
            Self::Dolphin { baby: true } => "dolphin_baby",
            Self::Guardian { elder: false } => "guardian",
            Self::Guardian { elder: true } => "elder_guardian",
            Self::Frog { .. } => "frog",
            Self::Creaking { .. } => "creaking",
            Self::Sniffer { baby: false } => "sniffer",
            Self::Sniffer { baby: true } => "sniffer_baby",
            Self::Warden => "warden",
            Self::Armadillo { baby: false, .. } => "armadillo",
            Self::Armadillo { baby: true, .. } => "armadillo_baby",
            Self::Axolotl { baby: false, .. } => "axolotl",
            Self::Axolotl { baby: true, .. } => "axolotl_baby",
            Self::Tadpole => "tadpole",
            Self::Parrot { .. } => "parrot",
            Self::Shulker { .. } => "shulker",
            Self::Wither => "wither",
            Self::Giant => "giant",
            Self::EndCrystal => "end_crystal",
            Self::EvokerFangs => "evoker_fangs",
            Self::LeashKnot => "leash_knot",
            Self::Chest {
                half: ChestModelHalf::Single,
                ..
            } => "chest",
            Self::Chest {
                half: ChestModelHalf::Left,
                ..
            } => "chest_left",
            Self::Chest {
                half: ChestModelHalf::Right,
                ..
            } => "chest_right",
            Self::Sign {
                attachment: SignModelAttachment::Standing,
                ..
            } => "sign_standing",
            Self::Sign {
                attachment: SignModelAttachment::Wall,
                ..
            } => "sign_wall",
            Self::Sign {
                attachment: SignModelAttachment::HangingCeiling,
                ..
            } => "hanging_sign_ceiling",
            Self::Sign {
                attachment: SignModelAttachment::HangingCeilingMiddle,
                ..
            } => "hanging_sign_ceiling_middle",
            Self::Sign {
                attachment: SignModelAttachment::HangingWall,
                ..
            } => "hanging_sign_wall",
            Self::Bed {
                part: BedModelPart::Head,
                ..
            } => "bed_head",
            Self::Bed {
                part: BedModelPart::Foot,
                ..
            } => "bed_foot",
            Self::Bell => "bell",
            Self::ShulkerBox { .. } => "shulker_box",
            Self::DecoratedPot { .. } => "decorated_pot",
            Self::Arrow { .. } => "arrow",
            Self::Trident => "trident",
            Self::WitherSkull { .. } => "wither_skull",
            Self::LlamaSpit => "llama_spit",
            Self::ShulkerBullet => "shulker_bullet",
            Self::WindCharge => "wind_charge",
            Self::EnderDragon => "ender_dragon",
            Self::NoRender => "no_render",
            Self::Phantom { .. } => "phantom",
            Self::Pufferfish { puff_state: 0 } => "pufferfish_small",
            Self::Pufferfish { puff_state: 1 } => "pufferfish_mid",
            Self::Pufferfish { .. } => "pufferfish_big",
            Self::Zombie { baby: false } => "zombie",
            Self::Zombie { baby: true } => "zombie_baby",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: false,
            } => "husk",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: true,
            } => "husk_baby",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: false,
            } => "drowned",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: true,
            } => "drowned_baby",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: false,
            } => "zombie_villager",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: true,
            } => "zombie_villager_baby",
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: false,
            } => "piglin",
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: true,
            } => "piglin_baby",
            Self::Piglin {
                family: PiglinModelFamily::PiglinBrute,
                ..
            } => "piglin_brute",
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: false,
            } => "zombified_piglin",
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: true,
            } => "zombified_piglin_baby",
            Self::Hoglin {
                family: HoglinModelFamily::Hoglin,
                baby: false,
            } => "hoglin",
            Self::Hoglin {
                family: HoglinModelFamily::Hoglin,
                baby: true,
            } => "hoglin_baby",
            Self::Hoglin {
                family: HoglinModelFamily::Zoglin,
                baby: false,
            } => "zoglin",
            Self::Hoglin {
                family: HoglinModelFamily::Zoglin,
                baby: true,
            } => "zoglin_baby",
            Self::Ravager => "ravager",
            Self::Skeleton => "skeleton",
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Stray,
            } => "stray",
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Parched,
            } => "parched",
            Self::SkeletonVariant {
                family: SkeletonModelFamily::WitherSkeleton,
            } => "wither_skeleton",
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Bogged { .. },
            } => "bogged",
            Self::Cow { variant, baby } => cow_model_key(variant, baby),
            Self::Mooshroom { baby: false, .. } => "mooshroom",
            Self::Mooshroom { baby: true, .. } => "mooshroom_baby",
            Self::Sheep {
                baby,
                sheared,
                wool_color,
                jeb,
                ..
            } => sheep_model_key(baby, sheared, wool_color, jeb),
            Self::Villager { baby: false } => "villager",
            Self::Villager { baby: true } => "villager_baby",
            Self::WanderingTrader => "wandering_trader",
            Self::Wolf {
                baby, tame, angry, ..
            } => wolf_model_key(baby, tame, angry),
            Self::Horse { baby: false, .. } => "horse",
            Self::Horse { baby: true, .. } => "horse_baby",
            Self::Donkey {
                family: DonkeyModelFamily::Donkey,
                baby: false,
                ..
            } => "donkey",
            Self::Donkey {
                family: DonkeyModelFamily::Donkey,
                baby: true,
                ..
            } => "donkey_baby",
            Self::Donkey {
                family: DonkeyModelFamily::Mule,
                baby: false,
                ..
            } => "mule",
            Self::Donkey {
                family: DonkeyModelFamily::Mule,
                baby: true,
                ..
            } => "mule_baby",
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Skeleton,
                baby: false,
            } => "skeleton_horse",
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Skeleton,
                baby: true,
            } => "skeleton_horse_baby",
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Zombie,
                baby: false,
            } => "zombie_horse",
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Zombie,
                baby: true,
            } => "zombie_horse_baby",
            Self::Camel {
                family: CamelModelFamily::Camel,
                baby: false,
            } => "camel",
            Self::Camel {
                family: CamelModelFamily::Camel,
                baby: true,
            } => "camel_baby",
            Self::Camel {
                family: CamelModelFamily::CamelHusk,
                ..
            } => "camel_husk",
            Self::Llama {
                family,
                variant,
                baby,
                ..
            } => llama_model_key(family, variant, baby),
            Self::Goat { baby: false, .. } => "goat",
            Self::Goat { baby: true, .. } => "goat_baby",
            Self::PolarBear { baby: false } => "polar_bear",
            Self::PolarBear { baby: true } => "polar_bear_baby",
            Self::Panda { baby: false, .. } => "panda",
            Self::Panda { baby: true, .. } => "panda_baby",
            Self::Feline {
                cat: true,
                baby: false,
                ..
            } => "feline_cat",
            Self::Feline {
                cat: false,
                baby: false,
                ..
            } => "feline_ocelot",
            Self::Feline {
                cat: true,
                baby: true,
                ..
            } => "feline_cat_baby",
            Self::Feline {
                cat: false,
                baby: true,
                ..
            } => "feline_ocelot_baby",
            Self::Fox { baby: false, .. } => "fox",
            Self::Fox { baby: true, .. } => "fox_baby",
            Self::Nautilus { baby: false } => "nautilus",
            Self::Nautilus { baby: true } => "nautilus_baby",
            Self::ZombieNautilus { coral: false } => "zombie_nautilus",
            Self::ZombieNautilus { coral: true } => "zombie_nautilus_coral",
            Self::Rabbit { baby: false, .. } => "rabbit",
            Self::Rabbit { baby: true, .. } => "rabbit_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Pig,
                baby: false,
            } => "quadruped_pig",
            Self::Quadruped {
                family: QuadrupedModelFamily::Pig,
                baby: true,
            } => "quadruped_pig_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Cow,
                baby: false,
            } => "quadruped_cow",
            Self::Quadruped {
                family: QuadrupedModelFamily::Cow,
                baby: true,
            } => "quadruped_cow_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Sheep,
                baby: false,
            } => "quadruped_sheep",
            Self::Quadruped {
                family: QuadrupedModelFamily::Sheep,
                baby: true,
            } => "quadruped_sheep_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Horse,
                baby: false,
            } => "quadruped_horse",
            Self::Quadruped {
                family: QuadrupedModelFamily::Horse,
                baby: true,
            } => "quadruped_horse_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Wolf,
                baby: false,
            } => "quadruped_wolf",
            Self::Quadruped {
                family: QuadrupedModelFamily::Wolf,
                baby: true,
            } => "quadruped_wolf_baby",
            Self::Creeper => "creeper",
            Self::Spider => "spider",
            Self::CaveSpider => "cave_spider",
            Self::Enderman => "enderman",
            Self::CopperGolem { .. } => "copper_golem",
            Self::IronGolem { .. } => "iron_golem",
            Self::SnowGolem => "snow_golem",
            Self::Witch => "witch",
            Self::Illager {
                family: IllagerModelFamily::Evoker,
            } => "evoker",
            Self::Illager {
                family: IllagerModelFamily::Illusioner,
            } => "illusioner",
            Self::Illager {
                family: IllagerModelFamily::Pillager,
            } => "pillager",
            Self::Illager {
                family: IllagerModelFamily::Vindicator,
            } => "vindicator",
            Self::Squid { glow, baby } => match (glow, baby) {
                (false, false) => "squid",
                (false, true) => "squid_baby",
                (true, false) => "glow_squid",
                (true, true) => "glow_squid_baby",
            },
            Self::Cod => "cod",
            Self::Salmon { size } => match size {
                SalmonModelSize::Small => "salmon_small",
                SalmonModelSize::Medium => "salmon",
                SalmonModelSize::Large => "salmon_large",
            },
            Self::TropicalFish { shape, .. } => match shape {
                TropicalFishModelShape::Small => "tropical_fish_small",
                TropicalFishModelShape::Large => "tropical_fish_large",
            },
            Self::Minecart => "minecart",
            Self::Boat { family, chest } => boat_model_key(family, chest),
            Self::Placeholder { name, .. } => name,
        }
    }

    pub fn vanilla_texture_ref(self) -> Option<EntityModelTextureRef> {
        match self {
            Self::Chicken { variant, baby } => Some(chicken_texture_ref(variant, baby)),
            Self::Pig { variant, baby } => Some(pig_texture_ref(variant, baby)),
            Self::Player { skin, .. } => Some(default_player_skin_texture_ref(skin.fallback())),
            Self::ArmorStand { .. } => Some(ARMOR_STAND_TEXTURE_REF),
            Self::Slime { .. } => Some(SLIME_TEXTURE_REF),
            Self::MagmaCube { .. } => Some(MAGMA_CUBE_TEXTURE_REF),
            Self::Ghast { charging } => Some(if charging {
                GHAST_SHOOTING_TEXTURE_REF
            } else {
                GHAST_TEXTURE_REF
            }),
            Self::HappyGhast => Some(HAPPY_GHAST_TEXTURE_REF),
            Self::Blaze => Some(BLAZE_TEXTURE_REF),
            Self::Endermite => Some(ENDERMITE_TEXTURE_REF),
            Self::Silverfish => Some(SILVERFISH_TEXTURE_REF),
            Self::LeashKnot => Some(LEASH_KNOT_TEXTURE_REF),
            Self::Chest { texture, half } => Some(chest_texture_ref(texture, half)),
            Self::Sign { wood, attachment } => Some(sign_texture_ref(wood, attachment)),
            Self::Bed { color, .. } => Some(bed_texture_ref(color)),
            Self::Bell => Some(BELL_BODY_TEXTURE_REF),
            // The box shares the shulker mob's texture set (`Sheets.getShulkerBoxSprite` selects
            // the same `entity/shulker/*` sprites the mob binds).
            Self::ShulkerBox { color, .. } => Some(shulker_texture_ref(color)),
            // The pot's base sheet; the four per-sherd side sprites ride their own layer passes.
            Self::DecoratedPot { .. } => Some(DECORATED_POT_BASE_TEXTURE_REF),
            Self::Trident => Some(TRIDENT_TEXTURE_REF),
            Self::EvokerFangs => Some(EVOKER_FANGS_TEXTURE_REF),
            Self::Tadpole => Some(TADPOLE_TEXTURE_REF),
            Self::Creaking { .. } => Some(CREAKING_TEXTURE_REF),
            Self::Sniffer { baby: false } => Some(SNIFFER_TEXTURE_REF),
            Self::Sniffer { baby: true } => Some(SNIFFLET_TEXTURE_REF),
            Self::Parrot { variant } => Some(parrot_texture_ref(variant)),
            Self::Shulker { color } => Some(shulker_texture_ref(color)),
            Self::EnderDragon => Some(ENDER_DRAGON_TEXTURE_REF),
            Self::Arrow { texture } => Some(arrow_texture_ref(texture)),
            Self::LlamaSpit => Some(LLAMA_SPIT_TEXTURE_REF),
            Self::ShulkerBullet => Some(SHULKER_BULLET_TEXTURE_REF),
            Self::WitherSkull { dangerous } => Some(wither_skull_texture_ref(dangerous)),
            Self::Wither => Some(WITHER_TEXTURE_REF),
            Self::WindCharge => Some(WIND_CHARGE_TEXTURE_REF),
            Self::Guardian { elder: false } => Some(GUARDIAN_TEXTURE_REF),
            Self::Guardian { elder: true } => Some(GUARDIAN_ELDER_TEXTURE_REF),
            Self::Warden => Some(WARDEN_TEXTURE_REF),
            Self::Frog { variant } => Some(frog_texture_ref(variant)),
            Self::Armadillo { baby: false, .. } => Some(ARMADILLO_TEXTURE_REF),
            Self::Armadillo { baby: true, .. } => Some(ARMADILLO_BABY_TEXTURE_REF),
            Self::Nautilus { baby: false } => Some(NAUTILUS_TEXTURE_REF),
            Self::Nautilus { baby: true } => Some(NAUTILUS_BABY_TEXTURE_REF),
            Self::ZombieNautilus { coral } => Some(if coral {
                ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF
            } else {
                ZOMBIE_NAUTILUS_TEXTURE_REF
            }),
            Self::Panda { baby, variant } => Some(panda_texture_ref(variant, baby)),
            Self::Axolotl { baby, variant } => Some(axolotl_texture_ref(variant, baby)),
            // The kind carries the variant and age; the sleeping dimension is dynamic render state,
            // so this representative ref is the idle texture (`fox_textured_layer_passes` picks the
            // sleeping cell at render time).
            Self::Fox { baby, variant } => Some(fox_texture_ref(variant, baby, false)),
            Self::Rabbit {
                baby,
                variant,
                toast,
            } => Some(rabbit_texture_ref(variant, baby, toast)),
            Self::Feline {
                cat,
                baby,
                cat_variant,
                ..
            } => Some(feline_texture_ref(cat, baby, cat_variant)),
            Self::Mooshroom { baby, variant } => Some(mooshroom_texture_ref(baby, variant)),
            Self::Vex { charging } => Some(if charging {
                VEX_CHARGING_TEXTURE_REF
            } else {
                VEX_TEXTURE_REF
            }),
            Self::Allay => Some(ALLAY_TEXTURE_REF),
            Self::Strider { baby, cold } => Some(strider_texture_ref(baby, cold)),
            Self::Turtle { baby: false } => Some(TURTLE_TEXTURE_REF),
            Self::Turtle { baby: true } => Some(TURTLE_BABY_TEXTURE_REF),
            Self::Bat => Some(BAT_TEXTURE_REF),
            Self::Bee {
                baby,
                angry,
                has_nectar,
            } => Some(bee_texture_ref(baby, angry, has_nectar)),
            Self::Breeze => Some(BREEZE_TEXTURE_REF),
            Self::Dolphin { baby: false } => Some(DOLPHIN_TEXTURE_REF),
            Self::Dolphin { baby: true } => Some(DOLPHIN_BABY_TEXTURE_REF),
            Self::Phantom { .. } => Some(PHANTOM_TEXTURE_REF),
            Self::Pufferfish { .. } => Some(PUFFERFISH_TEXTURE_REF),
            Self::Zombie { baby: false } => Some(ZOMBIE_TEXTURE_REF),
            Self::Zombie { baby: true } => Some(ZOMBIE_BABY_TEXTURE_REF),
            // Vanilla `GiantMobRenderer` reuses the plain zombie texture on a scaled humanoid.
            Self::Giant => Some(ZOMBIE_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: false,
            } => Some(HUSK_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: true,
            } => Some(HUSK_BABY_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: false,
            } => Some(DROWNED_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: true,
            } => Some(DROWNED_BABY_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: false,
            } => Some(ZOMBIE_VILLAGER_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: true,
            } => Some(ZOMBIE_VILLAGER_BABY_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: false,
            } => Some(PIGLIN_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: true,
            } => Some(PIGLIN_BABY_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::PiglinBrute,
                ..
            } => Some(PIGLIN_BRUTE_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: false,
            } => Some(ZOMBIFIED_PIGLIN_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: true,
            } => Some(ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF),
            Self::Hoglin {
                family: HoglinModelFamily::Hoglin,
                baby: false,
            } => Some(HOGLIN_TEXTURE_REF),
            Self::Hoglin {
                family: HoglinModelFamily::Hoglin,
                baby: true,
            } => Some(HOGLIN_BABY_TEXTURE_REF),
            Self::Hoglin {
                family: HoglinModelFamily::Zoglin,
                baby: false,
            } => Some(ZOGLIN_TEXTURE_REF),
            Self::Hoglin {
                family: HoglinModelFamily::Zoglin,
                baby: true,
            } => Some(ZOGLIN_BABY_TEXTURE_REF),
            Self::Ravager => Some(RAVAGER_TEXTURE_REF),
            Self::Skeleton => Some(SKELETON_TEXTURE_REF),
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Stray,
            } => Some(STRAY_TEXTURE_REF),
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Parched,
            } => Some(PARCHED_TEXTURE_REF),
            Self::SkeletonVariant {
                family: SkeletonModelFamily::WitherSkeleton,
            } => Some(WITHER_SKELETON_TEXTURE_REF),
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Bogged { .. },
            } => Some(BOGGED_TEXTURE_REF),
            Self::Cow { variant, baby } => Some(cow_texture_ref(variant, baby)),
            Self::Sheep { baby: false, .. } => Some(SHEEP_TEXTURE_REF),
            Self::Sheep { baby: true, .. } => Some(SHEEP_BABY_TEXTURE_REF),
            Self::Villager { baby: false } => Some(VILLAGER_TEXTURE_REF),
            Self::Villager { baby: true } => Some(VILLAGER_BABY_TEXTURE_REF),
            Self::WanderingTrader => Some(WANDERING_TRADER_TEXTURE_REF),
            Self::Wolf {
                baby,
                tame,
                angry,
                variant,
                ..
            } => Some(wolf_texture_ref(baby, tame, angry, variant)),
            Self::Horse { baby, variant, .. } => Some(horse_coat_texture_ref(variant, baby)),
            Self::Donkey {
                family: DonkeyModelFamily::Donkey,
                baby: false,
                ..
            } => Some(DONKEY_TEXTURE_REF),
            Self::Donkey {
                family: DonkeyModelFamily::Donkey,
                baby: true,
                ..
            } => Some(DONKEY_BABY_TEXTURE_REF),
            Self::Donkey {
                family: DonkeyModelFamily::Mule,
                baby: false,
                ..
            } => Some(MULE_TEXTURE_REF),
            Self::Donkey {
                family: DonkeyModelFamily::Mule,
                baby: true,
                ..
            } => Some(MULE_BABY_TEXTURE_REF),
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Skeleton,
                baby: false,
            } => Some(SKELETON_HORSE_TEXTURE_REF),
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Skeleton,
                baby: true,
            } => Some(SKELETON_HORSE_BABY_TEXTURE_REF),
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Zombie,
                baby: false,
            } => Some(ZOMBIE_HORSE_TEXTURE_REF),
            Self::UndeadHorse {
                family: UndeadHorseModelFamily::Zombie,
                baby: true,
            } => Some(ZOMBIE_HORSE_BABY_TEXTURE_REF),
            Self::Camel { family, baby } => Some(camel_texture_ref(family, baby)),
            Self::Squid { glow, baby } => Some(squid_texture_ref(glow, baby)),
            Self::Cod => Some(COD_TEXTURE_REF),
            Self::Salmon { .. } => Some(SALMON_TEXTURE_REF),
            Self::TropicalFish { shape, .. } => Some(match shape {
                TropicalFishModelShape::Small => TROPICAL_FISH_SMALL_TEXTURE_REF,
                TropicalFishModelShape::Large => TROPICAL_FISH_LARGE_TEXTURE_REF,
            }),
            Self::Llama { variant, baby, .. } => Some(llama_texture_ref(variant, baby)),
            Self::Goat { baby: false, .. } => Some(GOAT_TEXTURE_REF),
            Self::Goat { baby: true, .. } => Some(GOAT_BABY_TEXTURE_REF),
            Self::PolarBear { baby: false } => Some(POLAR_BEAR_TEXTURE_REF),
            Self::PolarBear { baby: true } => Some(POLAR_BEAR_BABY_TEXTURE_REF),
            Self::Creeper => Some(CREEPER_TEXTURE_REF),
            Self::Spider => Some(SPIDER_TEXTURE_REF),
            Self::CaveSpider => Some(CAVE_SPIDER_TEXTURE_REF),
            Self::Enderman => Some(ENDERMAN_TEXTURE_REF),
            Self::CopperGolem { weathering } => Some(copper_golem_texture_ref(weathering)),
            Self::IronGolem { .. } => Some(IRON_GOLEM_TEXTURE_REF),
            Self::SnowGolem => Some(SNOW_GOLEM_TEXTURE_REF),
            Self::Witch => Some(WITCH_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Evoker,
            } => Some(EVOKER_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Illusioner,
            } => Some(ILLUSIONER_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Pillager,
            } => Some(PILLAGER_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Vindicator,
            } => Some(VINDICATOR_TEXTURE_REF),
            Self::Boat { family, chest } => Some(boat_texture_ref(family, chest)),
            Self::Minecart => Some(MINECART_TEXTURE_REF),
            Self::EndCrystal => Some(END_CRYSTAL_TEXTURE_REF),
            _ => None,
        }
    }

    pub fn vanilla_layer_texture_refs(self) -> &'static [EntityModelTextureRef] {
        match self {
            Self::Sheep {
                baby: false,
                sheared: false,
                jeb: false,
                wool_color: SheepWoolColor::White,
                ..
            } => &SHEEP_WOOL_LAYER_TEXTURE_REFS,
            Self::Sheep {
                baby: false,
                sheared: false,
                ..
            } => &SHEEP_COLORED_WOOL_LAYER_TEXTURE_REFS,
            Self::Sheep {
                baby: false,
                sheared: true,
                jeb: false,
                wool_color: SheepWoolColor::White,
                ..
            } => &[],
            Self::Sheep {
                baby: false,
                sheared: true,
                ..
            } => &SHEEP_UNDERCOAT_LAYER_TEXTURE_REFS,
            Self::Sheep {
                baby: true,
                sheared: false,
                ..
            } => &BABY_SHEEP_WOOL_LAYER_TEXTURE_REFS,
            Self::Sheep {
                baby: true,
                sheared: true,
                ..
            } => &[],
            Self::Wolf {
                baby: false,
                tame: true,
                collar_color: Some(_),
                ..
            } => &WOLF_COLLAR_LAYER_TEXTURE_REFS,
            Self::Wolf {
                baby: true,
                tame: true,
                collar_color: Some(_),
                ..
            } => &WOLF_BABY_COLLAR_LAYER_TEXTURE_REFS,
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Stray,
            } => &STRAY_OVERLAY_LAYER_TEXTURE_REFS,
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Bogged { .. },
            } => &BOGGED_OVERLAY_LAYER_TEXTURE_REFS,
            Self::Spider | Self::CaveSpider => &SPIDER_EYES_LAYER_TEXTURE_REFS,
            Self::Enderman => &ENDERMAN_EYES_LAYER_TEXTURE_REFS,
            Self::CopperGolem { weathering } => copper_golem_eyes_layer_texture_refs(weathering),
            Self::IronGolem {
                crackiness: IronGolemCrackiness::Low,
            } => &IRON_GOLEM_CRACKINESS_LOW_LAYER_TEXTURE_REFS,
            Self::IronGolem {
                crackiness: IronGolemCrackiness::Medium,
            } => &IRON_GOLEM_CRACKINESS_MEDIUM_LAYER_TEXTURE_REFS,
            Self::IronGolem {
                crackiness: IronGolemCrackiness::High,
            } => &IRON_GOLEM_CRACKINESS_HIGH_LAYER_TEXTURE_REFS,
            _ => &[],
        }
    }
}

fn chicken_model_key(variant: ChickenModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (ChickenModelVariant::Temperate, false) => "chicken_temperate",
        (ChickenModelVariant::Temperate, true) => "chicken_temperate_baby",
        (ChickenModelVariant::Warm, false) => "chicken_warm",
        (ChickenModelVariant::Warm, true) => "chicken_warm_baby",
        (ChickenModelVariant::Cold, false) => "chicken_cold",
        (ChickenModelVariant::Cold, true) => "chicken_cold_baby",
    }
}

pub(in crate::entity_models) fn chicken_texture_ref(
    variant: ChickenModelVariant,
    baby: bool,
) -> EntityModelTextureRef {
    match (variant, baby) {
        (ChickenModelVariant::Temperate, false) => CHICKEN_TEMPERATE_TEXTURE_REF,
        (ChickenModelVariant::Temperate, true) => CHICKEN_TEMPERATE_BABY_TEXTURE_REF,
        (ChickenModelVariant::Warm, false) => CHICKEN_WARM_TEXTURE_REF,
        (ChickenModelVariant::Warm, true) => CHICKEN_WARM_BABY_TEXTURE_REF,
        (ChickenModelVariant::Cold, false) => CHICKEN_COLD_TEXTURE_REF,
        (ChickenModelVariant::Cold, true) => CHICKEN_COLD_BABY_TEXTURE_REF,
    }
}

fn pig_model_key(variant: PigModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (PigModelVariant::Temperate, false) => "pig_temperate",
        (PigModelVariant::Temperate, true) => "pig_temperate_baby",
        (PigModelVariant::Warm, false) => "pig_warm",
        (PigModelVariant::Warm, true) => "pig_warm_baby",
        (PigModelVariant::Cold, false) => "pig_cold",
        (PigModelVariant::Cold, true) => "pig_cold_baby",
    }
}

pub(in crate::entity_models) fn pig_texture_ref(
    variant: PigModelVariant,
    baby: bool,
) -> EntityModelTextureRef {
    match (variant, baby) {
        (PigModelVariant::Temperate, false) => PIG_TEMPERATE_TEXTURE_REF,
        (PigModelVariant::Temperate, true) => PIG_TEMPERATE_BABY_TEXTURE_REF,
        (PigModelVariant::Warm, false) => PIG_WARM_TEXTURE_REF,
        (PigModelVariant::Warm, true) => PIG_WARM_BABY_TEXTURE_REF,
        (PigModelVariant::Cold, false) => PIG_COLD_TEXTURE_REF,
        (PigModelVariant::Cold, true) => PIG_COLD_BABY_TEXTURE_REF,
    }
}

fn cow_model_key(variant: CowModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (CowModelVariant::Temperate, false) => "cow_temperate",
        (CowModelVariant::Temperate, true) => "cow_temperate_baby",
        (CowModelVariant::Warm, false) => "cow_warm",
        (CowModelVariant::Warm, true) => "cow_warm_baby",
        (CowModelVariant::Cold, false) => "cow_cold",
        (CowModelVariant::Cold, true) => "cow_cold_baby",
    }
}

pub(in crate::entity_models) fn cow_texture_ref(
    variant: CowModelVariant,
    baby: bool,
) -> EntityModelTextureRef {
    match (variant, baby) {
        (CowModelVariant::Temperate, false) => COW_TEMPERATE_TEXTURE_REF,
        (CowModelVariant::Temperate, true) => COW_TEMPERATE_BABY_TEXTURE_REF,
        (CowModelVariant::Warm, false) => COW_WARM_TEXTURE_REF,
        (CowModelVariant::Warm, true) => COW_WARM_BABY_TEXTURE_REF,
        (CowModelVariant::Cold, false) => COW_COLD_TEXTURE_REF,
        (CowModelVariant::Cold, true) => COW_COLD_BABY_TEXTURE_REF,
    }
}

pub(in crate::entity_models) fn villager_type_texture_ref(
    villager_type: VillagerModelType,
    baby: bool,
) -> EntityModelTextureRef {
    let refs = if baby {
        &VILLAGER_BABY_TYPE_TEXTURE_REFS
    } else {
        &VILLAGER_TYPE_TEXTURE_REFS
    };
    refs[villager_type_index(villager_type)]
}

pub(in crate::entity_models) fn zombie_villager_type_texture_ref(
    villager_type: VillagerModelType,
    baby: bool,
) -> EntityModelTextureRef {
    let refs = if baby {
        &ZOMBIE_VILLAGER_BABY_TYPE_TEXTURE_REFS
    } else {
        &ZOMBIE_VILLAGER_TYPE_TEXTURE_REFS
    };
    refs[villager_type_index(villager_type)]
}

pub(in crate::entity_models) fn villager_profession_texture_ref(
    profession: VillagerModelProfession,
) -> Option<EntityModelTextureRef> {
    villager_profession_texture_index(profession)
        .map(|index| VILLAGER_PROFESSION_TEXTURE_REFS[index])
}

pub(in crate::entity_models) fn zombie_villager_profession_texture_ref(
    profession: VillagerModelProfession,
) -> Option<EntityModelTextureRef> {
    villager_profession_texture_index(profession)
        .map(|index| ZOMBIE_VILLAGER_PROFESSION_TEXTURE_REFS[index])
}

pub(in crate::entity_models) fn villager_level_texture_ref(level: i32) -> EntityModelTextureRef {
    VILLAGER_LEVEL_TEXTURE_REFS[villager_level_index(level)]
}

pub(in crate::entity_models) fn zombie_villager_level_texture_ref(
    level: i32,
) -> EntityModelTextureRef {
    ZOMBIE_VILLAGER_LEVEL_TEXTURE_REFS[villager_level_index(level)]
}

fn villager_type_index(villager_type: VillagerModelType) -> usize {
    match villager_type {
        VillagerModelType::Desert => 0,
        VillagerModelType::Jungle => 1,
        VillagerModelType::Plains => 2,
        VillagerModelType::Savanna => 3,
        VillagerModelType::Snow => 4,
        VillagerModelType::Swamp => 5,
        VillagerModelType::Taiga => 6,
    }
}

fn villager_profession_texture_index(profession: VillagerModelProfession) -> Option<usize> {
    Some(match profession {
        VillagerModelProfession::None => return None,
        VillagerModelProfession::Armorer => 0,
        VillagerModelProfession::Butcher => 1,
        VillagerModelProfession::Cartographer => 2,
        VillagerModelProfession::Cleric => 3,
        VillagerModelProfession::Farmer => 4,
        VillagerModelProfession::Fisherman => 5,
        VillagerModelProfession::Fletcher => 6,
        VillagerModelProfession::Leatherworker => 7,
        VillagerModelProfession::Librarian => 8,
        VillagerModelProfession::Mason => 9,
        VillagerModelProfession::Nitwit => 10,
        VillagerModelProfession::Shepherd => 11,
        VillagerModelProfession::Toolsmith => 12,
        VillagerModelProfession::Weaponsmith => 13,
    })
}

fn villager_level_index(level: i32) -> usize {
    (level.clamp(1, 5) - 1) as usize
}

pub(in crate::entity_models) fn squid_texture_ref(glow: bool, baby: bool) -> EntityModelTextureRef {
    match (glow, baby) {
        (false, false) => SQUID_TEXTURE_REF,
        (false, true) => SQUID_BABY_TEXTURE_REF,
        (true, false) => GLOW_SQUID_TEXTURE_REF,
        (true, true) => GLOW_SQUID_BABY_TEXTURE_REF,
    }
}

pub(in crate::entity_models) fn camel_texture_ref(
    family: CamelModelFamily,
    baby: bool,
) -> EntityModelTextureRef {
    match (family, baby) {
        // `CamelHuskRenderer` reuses the adult camel model with `camel_husk.png` and is
        // never a baby, so the husk maps to the husk texture regardless of the age flag.
        (CamelModelFamily::CamelHusk, _) => CAMEL_HUSK_TEXTURE_REF,
        (CamelModelFamily::Camel, false) => CAMEL_TEXTURE_REF,
        (CamelModelFamily::Camel, true) => CAMEL_BABY_TEXTURE_REF,
    }
}

/// The living horse's base coat (vanilla `HorseRenderer.LOCATION_BY_VARIANT`, then
/// `state.isBaby ? variant.baby : variant.adult`): one of seven `horse_<color>(_baby).png` 64×64
/// textures. The markings overlay is a separate deferred layer.
pub(in crate::entity_models) fn horse_coat_texture_ref(
    variant: HorseColorVariant,
    baby: bool,
) -> EntityModelTextureRef {
    match (variant, baby) {
        (HorseColorVariant::White, false) => HORSE_WHITE_TEXTURE_REF,
        (HorseColorVariant::White, true) => HORSE_WHITE_BABY_TEXTURE_REF,
        (HorseColorVariant::Creamy, false) => HORSE_CREAMY_TEXTURE_REF,
        (HorseColorVariant::Creamy, true) => HORSE_CREAMY_BABY_TEXTURE_REF,
        (HorseColorVariant::Chestnut, false) => HORSE_CHESTNUT_TEXTURE_REF,
        (HorseColorVariant::Chestnut, true) => HORSE_CHESTNUT_BABY_TEXTURE_REF,
        (HorseColorVariant::Brown, false) => HORSE_BROWN_TEXTURE_REF,
        (HorseColorVariant::Brown, true) => HORSE_BROWN_BABY_TEXTURE_REF,
        (HorseColorVariant::Black, false) => HORSE_BLACK_TEXTURE_REF,
        (HorseColorVariant::Black, true) => HORSE_BLACK_BABY_TEXTURE_REF,
        (HorseColorVariant::Gray, false) => HORSE_GRAY_TEXTURE_REF,
        (HorseColorVariant::Gray, true) => HORSE_GRAY_BABY_TEXTURE_REF,
        (HorseColorVariant::DarkBrown, false) => HORSE_DARKBROWN_TEXTURE_REF,
        (HorseColorVariant::DarkBrown, true) => HORSE_DARKBROWN_BABY_TEXTURE_REF,
    }
}

/// The living horse's white-markings overlay texture (vanilla `HorseMarkingLayer.LOCATION_BY_MARKINGS`,
/// then `state.isBaby ? variant.baby : variant.adult`). `Markings.NONE` maps to vanilla's
/// `INVISIBLE_TEXTURE` (no overlay), so it returns `None` and the overlay pass is skipped.
pub(in crate::entity_models) fn horse_markings_texture_ref(
    markings: HorseMarkings,
    baby: bool,
) -> Option<EntityModelTextureRef> {
    Some(match (markings, baby) {
        (HorseMarkings::None, _) => return None,
        (HorseMarkings::White, false) => HORSE_MARKINGS_WHITE_TEXTURE_REF,
        (HorseMarkings::White, true) => HORSE_MARKINGS_WHITE_BABY_TEXTURE_REF,
        (HorseMarkings::WhiteField, false) => HORSE_MARKINGS_WHITEFIELD_TEXTURE_REF,
        (HorseMarkings::WhiteField, true) => HORSE_MARKINGS_WHITEFIELD_BABY_TEXTURE_REF,
        (HorseMarkings::WhiteDots, false) => HORSE_MARKINGS_WHITEDOTS_TEXTURE_REF,
        (HorseMarkings::WhiteDots, true) => HORSE_MARKINGS_WHITEDOTS_BABY_TEXTURE_REF,
        (HorseMarkings::BlackDots, false) => HORSE_MARKINGS_BLACKDOTS_TEXTURE_REF,
        (HorseMarkings::BlackDots, true) => HORSE_MARKINGS_BLACKDOTS_BABY_TEXTURE_REF,
    })
}

const SHEEP_WOOL_COLOR_MODEL_KEYS: [&str; 16] = [
    "sheep",
    "sheep_orange",
    "sheep_magenta",
    "sheep_light_blue",
    "sheep_yellow",
    "sheep_lime",
    "sheep_pink",
    "sheep_gray",
    "sheep_light_gray",
    "sheep_cyan",
    "sheep_purple",
    "sheep_blue",
    "sheep_brown",
    "sheep_green",
    "sheep_red",
    "sheep_black",
];

const BABY_SHEEP_WOOL_COLOR_MODEL_KEYS: [&str; 16] = [
    "sheep_baby",
    "sheep_baby_orange",
    "sheep_baby_magenta",
    "sheep_baby_light_blue",
    "sheep_baby_yellow",
    "sheep_baby_lime",
    "sheep_baby_pink",
    "sheep_baby_gray",
    "sheep_baby_light_gray",
    "sheep_baby_cyan",
    "sheep_baby_purple",
    "sheep_baby_blue",
    "sheep_baby_brown",
    "sheep_baby_green",
    "sheep_baby_red",
    "sheep_baby_black",
];

const SHEEP_SHEARED_COLOR_MODEL_KEYS: [&str; 16] = [
    "sheep_sheared",
    "sheep_orange_sheared",
    "sheep_magenta_sheared",
    "sheep_light_blue_sheared",
    "sheep_yellow_sheared",
    "sheep_lime_sheared",
    "sheep_pink_sheared",
    "sheep_gray_sheared",
    "sheep_light_gray_sheared",
    "sheep_cyan_sheared",
    "sheep_purple_sheared",
    "sheep_blue_sheared",
    "sheep_brown_sheared",
    "sheep_green_sheared",
    "sheep_red_sheared",
    "sheep_black_sheared",
];

const SHEEP_WOOL_LAYER_COLOR_BYTES: [[u8; 3]; 16] = [
    [230, 230, 230],
    [186, 96, 21],
    [149, 58, 141],
    [43, 134, 163],
    [190, 162, 45],
    [96, 149, 23],
    [182, 104, 127],
    [53, 59, 61],
    [117, 117, 113],
    [16, 117, 117],
    [102, 37, 138],
    [45, 51, 127],
    [98, 63, 37],
    [70, 93, 16],
    [132, 34, 28],
    [21, 21, 24],
];

fn sheep_model_key(
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
    jeb: bool,
) -> &'static str {
    if jeb {
        return match (baby, sheared) {
            (false, false) => "sheep_jeb",
            (true, false) => "sheep_jeb_baby",
            (false, true) => "sheep_jeb_sheared",
            (true, true) => "sheep_jeb_baby_sheared",
        };
    }

    let color = wool_color.vanilla_id() as usize;
    match (baby, sheared) {
        (false, false) => SHEEP_WOOL_COLOR_MODEL_KEYS[color],
        (true, false) => BABY_SHEEP_WOOL_COLOR_MODEL_KEYS[color],
        (false, true) => SHEEP_SHEARED_COLOR_MODEL_KEYS[color],
        (true, true) => "sheep_baby_sheared",
    }
}

pub(in crate::entity_models) fn sheep_wool_layer_color(wool_color: SheepWoolColor) -> [f32; 4] {
    let [red, green, blue] = SHEEP_WOOL_LAYER_COLOR_BYTES[wool_color.vanilla_id() as usize];
    sheep_wool_layer_color_from_bytes([red, green, blue])
}

pub(in crate::entity_models) fn sheep_wool_render_color(
    wool_color: SheepWoolColor,
    jeb: bool,
    age_ticks: f32,
) -> [f32; 4] {
    if jeb {
        sheep_jeb_wool_layer_color(age_ticks)
    } else {
        sheep_wool_layer_color(wool_color)
    }
}

pub(in crate::entity_models) fn sheep_jeb_wool_layer_color(age_ticks: f32) -> [f32; 4] {
    const SHEEP_COLOR_DURATION_TICKS: i32 = 25;
    let tick = age_ticks.max(0.0);
    let tick_floor = tick.floor() as i32;
    let color_step = tick_floor / SHEEP_COLOR_DURATION_TICKS;
    let color_count = SHEEP_WOOL_LAYER_COLOR_BYTES.len() as i32;
    let from = color_step.rem_euclid(color_count) as usize;
    let to = (color_step + 1).rem_euclid(color_count) as usize;
    let alpha = ((tick_floor % SHEEP_COLOR_DURATION_TICKS) as f32 + tick.fract())
        / SHEEP_COLOR_DURATION_TICKS as f32;
    sheep_wool_layer_color_from_bytes(sheep_lerp_color_bytes(
        SHEEP_WOOL_LAYER_COLOR_BYTES[from],
        SHEEP_WOOL_LAYER_COLOR_BYTES[to],
        alpha,
    ))
}

fn sheep_wool_layer_color_from_bytes([red, green, blue]: [u8; 3]) -> [f32; 4] {
    [
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        1.0,
    ]
}

fn sheep_lerp_color_bytes(from: [u8; 3], to: [u8; 3], alpha: f32) -> [u8; 3] {
    [
        sheep_lerp_color_channel(from[0], to[0], alpha),
        sheep_lerp_color_channel(from[1], to[1], alpha),
        sheep_lerp_color_channel(from[2], to[2], alpha),
    ]
}

fn sheep_lerp_color_channel(from: u8, to: u8, alpha: f32) -> u8 {
    let from = i32::from(from);
    let to = i32::from(to);
    (from + (alpha * (to - from) as f32).floor() as i32).clamp(0, 255) as u8
}

fn wolf_model_key(baby: bool, tame: bool, angry: bool) -> &'static str {
    match (baby, tame, angry) {
        (false, true, _) => "wolf_tame",
        (false, false, true) => "wolf_angry",
        (false, false, false) => "wolf",
        (true, true, _) => "wolf_tame_baby",
        (true, false, true) => "wolf_angry_baby",
        (true, false, false) => "wolf_baby",
    }
}

pub(in crate::entity_models) fn mooshroom_texture_ref(
    baby: bool,
    variant: MooshroomVariant,
) -> EntityModelTextureRef {
    // Vanilla `MushroomCowRenderer.getTextureLocation`: `isBaby ? variant.baby : variant.adult`.
    match (variant, baby) {
        (MooshroomVariant::Red, false) => MOOSHROOM_TEXTURE_REF,
        (MooshroomVariant::Red, true) => MOOSHROOM_BABY_TEXTURE_REF,
        (MooshroomVariant::Brown, false) => MOOSHROOM_BROWN_TEXTURE_REF,
        (MooshroomVariant::Brown, true) => MOOSHROOM_BROWN_BABY_TEXTURE_REF,
    }
}

pub(in crate::entity_models) fn wolf_texture_ref(
    baby: bool,
    tame: bool,
    angry: bool,
    variant: WolfModelVariant,
) -> EntityModelTextureRef {
    // Each variant's `[wild, tame, angry, baby_wild, baby_tame, baby_angry]` face set, matching the
    // vanilla `WolfVariant.AssetInfo` adult/baby pair (`WolfVariants.register`).
    let set = match variant {
        WolfModelVariant::Pale => [
            WOLF_TEXTURE_REF,
            WOLF_TAME_TEXTURE_REF,
            WOLF_ANGRY_TEXTURE_REF,
            WOLF_BABY_TEXTURE_REF,
            WOLF_TAME_BABY_TEXTURE_REF,
            WOLF_ANGRY_BABY_TEXTURE_REF,
        ],
        WolfModelVariant::Spotted => [
            WOLF_SPOTTED_TEXTURE_REF,
            WOLF_SPOTTED_TAME_TEXTURE_REF,
            WOLF_SPOTTED_ANGRY_TEXTURE_REF,
            WOLF_SPOTTED_BABY_TEXTURE_REF,
            WOLF_SPOTTED_TAME_BABY_TEXTURE_REF,
            WOLF_SPOTTED_ANGRY_BABY_TEXTURE_REF,
        ],
        WolfModelVariant::Snowy => [
            WOLF_SNOWY_TEXTURE_REF,
            WOLF_SNOWY_TAME_TEXTURE_REF,
            WOLF_SNOWY_ANGRY_TEXTURE_REF,
            WOLF_SNOWY_BABY_TEXTURE_REF,
            WOLF_SNOWY_TAME_BABY_TEXTURE_REF,
            WOLF_SNOWY_ANGRY_BABY_TEXTURE_REF,
        ],
        WolfModelVariant::Black => [
            WOLF_BLACK_TEXTURE_REF,
            WOLF_BLACK_TAME_TEXTURE_REF,
            WOLF_BLACK_ANGRY_TEXTURE_REF,
            WOLF_BLACK_BABY_TEXTURE_REF,
            WOLF_BLACK_TAME_BABY_TEXTURE_REF,
            WOLF_BLACK_ANGRY_BABY_TEXTURE_REF,
        ],
        WolfModelVariant::Ashen => [
            WOLF_ASHEN_TEXTURE_REF,
            WOLF_ASHEN_TAME_TEXTURE_REF,
            WOLF_ASHEN_ANGRY_TEXTURE_REF,
            WOLF_ASHEN_BABY_TEXTURE_REF,
            WOLF_ASHEN_TAME_BABY_TEXTURE_REF,
            WOLF_ASHEN_ANGRY_BABY_TEXTURE_REF,
        ],
        WolfModelVariant::Rusty => [
            WOLF_RUSTY_TEXTURE_REF,
            WOLF_RUSTY_TAME_TEXTURE_REF,
            WOLF_RUSTY_ANGRY_TEXTURE_REF,
            WOLF_RUSTY_BABY_TEXTURE_REF,
            WOLF_RUSTY_TAME_BABY_TEXTURE_REF,
            WOLF_RUSTY_ANGRY_BABY_TEXTURE_REF,
        ],
        WolfModelVariant::Woods => [
            WOLF_WOODS_TEXTURE_REF,
            WOLF_WOODS_TAME_TEXTURE_REF,
            WOLF_WOODS_ANGRY_TEXTURE_REF,
            WOLF_WOODS_BABY_TEXTURE_REF,
            WOLF_WOODS_TAME_BABY_TEXTURE_REF,
            WOLF_WOODS_ANGRY_BABY_TEXTURE_REF,
        ],
        WolfModelVariant::Chestnut => [
            WOLF_CHESTNUT_TEXTURE_REF,
            WOLF_CHESTNUT_TAME_TEXTURE_REF,
            WOLF_CHESTNUT_ANGRY_TEXTURE_REF,
            WOLF_CHESTNUT_BABY_TEXTURE_REF,
            WOLF_CHESTNUT_TAME_BABY_TEXTURE_REF,
            WOLF_CHESTNUT_ANGRY_BABY_TEXTURE_REF,
        ],
        WolfModelVariant::Striped => [
            WOLF_STRIPED_TEXTURE_REF,
            WOLF_STRIPED_TAME_TEXTURE_REF,
            WOLF_STRIPED_ANGRY_TEXTURE_REF,
            WOLF_STRIPED_BABY_TEXTURE_REF,
            WOLF_STRIPED_TAME_BABY_TEXTURE_REF,
            WOLF_STRIPED_ANGRY_BABY_TEXTURE_REF,
        ],
    };
    // Vanilla `Wolf.getTexture`: pick adult/baby info, then tame → angry → wild.
    match (baby, tame, angry) {
        (false, true, _) => set[1],
        (false, false, true) => set[2],
        (false, false, false) => set[0],
        (true, true, _) => set[4],
        (true, false, true) => set[5],
        (true, false, false) => set[3],
    }
}

#[cfg(test)]
pub(in crate::entity_models) fn player_texture_ref(slim: bool) -> EntityModelTextureRef {
    if slim {
        PLAYER_SLIM_STEVE_TEXTURE_REF
    } else {
        PLAYER_WIDE_STEVE_TEXTURE_REF
    }
}

fn llama_model_key(family: LlamaModelFamily, variant: LlamaVariant, baby: bool) -> &'static str {
    match (family, variant, baby) {
        (LlamaModelFamily::Llama, LlamaVariant::Creamy, false) => "llama_creamy",
        (LlamaModelFamily::Llama, LlamaVariant::Creamy, true) => "llama_creamy_baby",
        (LlamaModelFamily::Llama, LlamaVariant::White, false) => "llama_white",
        (LlamaModelFamily::Llama, LlamaVariant::White, true) => "llama_white_baby",
        (LlamaModelFamily::Llama, LlamaVariant::Brown, false) => "llama_brown",
        (LlamaModelFamily::Llama, LlamaVariant::Brown, true) => "llama_brown_baby",
        (LlamaModelFamily::Llama, LlamaVariant::Gray, false) => "llama_gray",
        (LlamaModelFamily::Llama, LlamaVariant::Gray, true) => "llama_gray_baby",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Creamy, false) => "trader_llama_creamy",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Creamy, true) => "trader_llama_creamy_baby",
        (LlamaModelFamily::TraderLlama, LlamaVariant::White, false) => "trader_llama_white",
        (LlamaModelFamily::TraderLlama, LlamaVariant::White, true) => "trader_llama_white_baby",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Brown, false) => "trader_llama_brown",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Brown, true) => "trader_llama_brown_baby",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Gray, false) => "trader_llama_gray",
        (LlamaModelFamily::TraderLlama, LlamaVariant::Gray, true) => "trader_llama_gray_baby",
    }
}

pub(in crate::entity_models) fn llama_texture_ref(
    variant: LlamaVariant,
    baby: bool,
) -> EntityModelTextureRef {
    match (variant, baby) {
        (LlamaVariant::Creamy, false) => LLAMA_CREAMY_TEXTURE_REF,
        (LlamaVariant::Creamy, true) => LLAMA_CREAMY_BABY_TEXTURE_REF,
        (LlamaVariant::White, false) => LLAMA_WHITE_TEXTURE_REF,
        (LlamaVariant::White, true) => LLAMA_WHITE_BABY_TEXTURE_REF,
        (LlamaVariant::Brown, false) => LLAMA_BROWN_TEXTURE_REF,
        (LlamaVariant::Brown, true) => LLAMA_BROWN_BABY_TEXTURE_REF,
        (LlamaVariant::Gray, false) => LLAMA_GRAY_TEXTURE_REF,
        (LlamaVariant::Gray, true) => LLAMA_GRAY_BABY_TEXTURE_REF,
    }
}

fn boat_model_key(family: BoatModelFamily, chest: bool) -> &'static str {
    match (family, chest) {
        (BoatModelFamily::Acacia, false) => "boat_acacia",
        (BoatModelFamily::Acacia, true) => "chest_boat_acacia",
        (BoatModelFamily::Bamboo, false) => "boat_bamboo",
        (BoatModelFamily::Bamboo, true) => "chest_boat_bamboo",
        (BoatModelFamily::Birch, false) => "boat_birch",
        (BoatModelFamily::Birch, true) => "chest_boat_birch",
        (BoatModelFamily::Cherry, false) => "boat_cherry",
        (BoatModelFamily::Cherry, true) => "chest_boat_cherry",
        (BoatModelFamily::DarkOak, false) => "boat_dark_oak",
        (BoatModelFamily::DarkOak, true) => "chest_boat_dark_oak",
        (BoatModelFamily::Jungle, false) => "boat_jungle",
        (BoatModelFamily::Jungle, true) => "chest_boat_jungle",
        (BoatModelFamily::Mangrove, false) => "boat_mangrove",
        (BoatModelFamily::Mangrove, true) => "chest_boat_mangrove",
        (BoatModelFamily::Oak, false) => "boat_oak",
        (BoatModelFamily::Oak, true) => "chest_boat_oak",
        (BoatModelFamily::PaleOak, false) => "boat_pale_oak",
        (BoatModelFamily::PaleOak, true) => "chest_boat_pale_oak",
        (BoatModelFamily::Spruce, false) => "boat_spruce",
        (BoatModelFamily::Spruce, true) => "chest_boat_spruce",
    }
}

pub(in crate::entity_models) fn boat_texture_ref(
    family: BoatModelFamily,
    chest: bool,
) -> EntityModelTextureRef {
    match (family, chest) {
        (BoatModelFamily::Acacia, false) => BOAT_ACACIA_TEXTURE_REF,
        (BoatModelFamily::Acacia, true) => CHEST_BOAT_ACACIA_TEXTURE_REF,
        (BoatModelFamily::Bamboo, false) => BOAT_BAMBOO_TEXTURE_REF,
        (BoatModelFamily::Bamboo, true) => CHEST_BOAT_BAMBOO_TEXTURE_REF,
        (BoatModelFamily::Birch, false) => BOAT_BIRCH_TEXTURE_REF,
        (BoatModelFamily::Birch, true) => CHEST_BOAT_BIRCH_TEXTURE_REF,
        (BoatModelFamily::Cherry, false) => BOAT_CHERRY_TEXTURE_REF,
        (BoatModelFamily::Cherry, true) => CHEST_BOAT_CHERRY_TEXTURE_REF,
        (BoatModelFamily::DarkOak, false) => BOAT_DARK_OAK_TEXTURE_REF,
        (BoatModelFamily::DarkOak, true) => CHEST_BOAT_DARK_OAK_TEXTURE_REF,
        (BoatModelFamily::Jungle, false) => BOAT_JUNGLE_TEXTURE_REF,
        (BoatModelFamily::Jungle, true) => CHEST_BOAT_JUNGLE_TEXTURE_REF,
        (BoatModelFamily::Mangrove, false) => BOAT_MANGROVE_TEXTURE_REF,
        (BoatModelFamily::Mangrove, true) => CHEST_BOAT_MANGROVE_TEXTURE_REF,
        (BoatModelFamily::Oak, false) => BOAT_OAK_TEXTURE_REF,
        (BoatModelFamily::Oak, true) => CHEST_BOAT_OAK_TEXTURE_REF,
        (BoatModelFamily::PaleOak, false) => BOAT_PALE_OAK_TEXTURE_REF,
        (BoatModelFamily::PaleOak, true) => CHEST_BOAT_PALE_OAK_TEXTURE_REF,
        (BoatModelFamily::Spruce, false) => BOAT_SPRUCE_TEXTURE_REF,
        (BoatModelFamily::Spruce, true) => CHEST_BOAT_SPRUCE_TEXTURE_REF,
    }
}
