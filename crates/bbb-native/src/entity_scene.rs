use bbb_protocol::packets::{
    decode_profile_textures_from_properties, EntityDataEnumSerializer, EntityDataRegistryHolder,
    EntityDataValueKind, EquipmentSlot, GameProfilePropertySummary, ItemStackSummary,
    PlayerSkinPatchSummary, ResolvableProfileKindSummary, ResolvableProfileSummary,
};
use bbb_renderer::{
    ArmorStandModelPose, ArrowModelTexture, AxolotlModelVariant, BoatModelFamily, CamelModelFamily,
    CatModelVariant, ChickenModelVariant, CopperGolemWeathering, CowModelVariant,
    DonkeyModelFamily, EndCrystalBeamRenderState, EnderDragonBeamRenderState, EntityArmorMaterial,
    EntityAttachmentFace, EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityDyeColor,
    EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind, EntityEquipmentLayerTexture,
    EntityModelInstance, EntityModelKind, EntityPlayerSkin, FoxModelVariant, FrogModelVariant,
    GuardianBeamRenderState, HoglinModelFamily, HorseColorVariant, HorseMarkings,
    IllagerModelFamily, IronGolemCrackiness, LlamaModelFamily, LlamaVariant, MooshroomVariant,
    PandaModelVariant, ParrotModelVariant, PigModelVariant, PiglinModelFamily,
    PlayerModelPartVisibility, RabbitModelVariant, SalmonModelSize, SelectionBox, SelectionOutline,
    SheepHeadEatPose, SheepWoolColor, SkeletonModelFamily, SleepingPose, TropicalFishModelShape,
    TropicalFishPattern, UndeadHorseModelFamily, VillagerModelData, VillagerModelProfession,
    VillagerModelType, WolfArmorCrackiness, WolfModelVariant, ZombieVariantModelFamily,
    DEFAULT_ARMOR_STAND_MODEL_POSE, ENTITY_DEFAULT_OUTLINE_COLOR,
};
#[cfg(test)]
use bbb_renderer::{EntityDynamicPlayerSkinStatus, EntityPlayerSkinModel};
use bbb_world::{
    ArmorMaterialKind as WorldArmorMaterialKind, EndCrystalBeamSource as WorldEndCrystalBeamSource,
    EnderDragonBeamSource as WorldEnderDragonBeamSource,
    EntityAttachmentFace as WorldEntityAttachmentFace, EntityModelSourceState,
    EntityPickTargetState, GuardianBeamSource as WorldGuardianBeamSource,
    LlamaBodyDecorColor as WorldLlamaBodyDecorColor, RegistryContentState,
    WolfArmorCrackiness as WorldWolfArmorCrackiness, WorldStore,
};

use crate::item_runtime::{default_player_skin_for_profile_id, NativeItemRuntime};

const VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID: i32 = 0;
const VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID: i32 = 1;
const VANILLA_ENTITY_TYPE_ALLAY_ID: i32 = 2;
const VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID: i32 = 3;
const VANILLA_ENTITY_TYPE_ARMADILLO_ID: i32 = 4;
const VANILLA_ENTITY_TYPE_ARMOR_STAND_ID: i32 = 5;
const VANILLA_ENTITY_TYPE_ARROW_ID: i32 = 6;
const VANILLA_ENTITY_TYPE_AXOLOTL_ID: i32 = 7;
const VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID: i32 = 8;
const VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID: i32 = 9;
const VANILLA_ENTITY_TYPE_BAT_ID: i32 = 10;
const VANILLA_ENTITY_TYPE_BEE_ID: i32 = 11;
const VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID: i32 = 12;
const VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID: i32 = 13;
const VANILLA_ENTITY_TYPE_BLAZE_ID: i32 = 14;
const VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID: i32 = 15;
const VANILLA_ENTITY_TYPE_BOGGED_ID: i32 = 16;
const VANILLA_ENTITY_TYPE_BREEZE_ID: i32 = 17;
const VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID: i32 = 18;
const VANILLA_ENTITY_TYPE_CAMEL_ID: i32 = 19;
const VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID: i32 = 20;
const VANILLA_ENTITY_TYPE_CAT_ID: i32 = 21;
const VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID: i32 = 22;
const VANILLA_ENTITY_TYPE_CHERRY_BOAT_ID: i32 = 23;
const VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID: i32 = 24;
const VANILLA_ENTITY_TYPE_CHEST_MINECART_ID: i32 = 25;
const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
const VANILLA_ENTITY_TYPE_COD_ID: i32 = 27;
const VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID: i32 = 28;
const VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID: i32 = 29;
const VANILLA_ENTITY_TYPE_COW_ID: i32 = 30;
const VANILLA_ENTITY_TYPE_CREAKING_ID: i32 = 31;
const VANILLA_ENTITY_TYPE_CREEPER_ID: i32 = 32;
const VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID: i32 = 33;
const VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID: i32 = 34;
const VANILLA_ENTITY_TYPE_DOLPHIN_ID: i32 = 35;
const VANILLA_ENTITY_TYPE_DONKEY_ID: i32 = 36;
const VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID: i32 = 37;
const VANILLA_ENTITY_TYPE_DROWNED_ID: i32 = 38;
const VANILLA_ENTITY_TYPE_EGG_ID: i32 = 39;
const VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID: i32 = 40;
const VANILLA_ENTITY_TYPE_ENDERMAN_ID: i32 = 41;
const VANILLA_ENTITY_TYPE_ENDERMITE_ID: i32 = 42;
const VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID: i32 = 43;
const VANILLA_ENTITY_TYPE_ENDER_PEARL_ID: i32 = 44;
const VANILLA_ENTITY_TYPE_END_CRYSTAL_ID: i32 = 45;
const VANILLA_ENTITY_TYPE_EVOKER_ID: i32 = 46;
const VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID: i32 = 47;
const VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID: i32 = 48;
const VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID: i32 = 49;
const VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID: i32 = 50;
const VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID: i32 = 51;
const VANILLA_ENTITY_TYPE_FIREBALL_ID: i32 = 52;
const VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID: i32 = 53;
const VANILLA_ENTITY_TYPE_FOX_ID: i32 = 54;
const VANILLA_ENTITY_TYPE_FROG_ID: i32 = 55;
const VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID: i32 = 56;
const VANILLA_ENTITY_TYPE_GHAST_ID: i32 = 57;
const VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID: i32 = 58;
const VANILLA_ENTITY_TYPE_GIANT_ID: i32 = 59;
const VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID: i32 = 60;
const VANILLA_ENTITY_TYPE_GLOW_SQUID_ID: i32 = 61;
const VANILLA_ENTITY_TYPE_GOAT_ID: i32 = 62;
const VANILLA_ENTITY_TYPE_GUARDIAN_ID: i32 = 63;
const VANILLA_ENTITY_TYPE_HOGLIN_ID: i32 = 64;
const VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID: i32 = 65;
const VANILLA_ENTITY_TYPE_HORSE_ID: i32 = 66;
const VANILLA_ENTITY_TYPE_HUSK_ID: i32 = 67;
const VANILLA_ENTITY_TYPE_ILLUSIONER_ID: i32 = 68;
const VANILLA_ENTITY_TYPE_INTERACTION_ID: i32 = 69;
const VANILLA_ENTITY_TYPE_IRON_GOLEM_ID: i32 = 70;
const VANILLA_ENTITY_TYPE_ITEM_ID: i32 = 71;
const VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID: i32 = 72;
const VANILLA_ENTITY_TYPE_ITEM_FRAME_ID: i32 = 73;
const VANILLA_ENTITY_TYPE_JUNGLE_BOAT_ID: i32 = 74;
const VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID: i32 = 75;
const VANILLA_ENTITY_TYPE_LEASH_KNOT_ID: i32 = 76;
const VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID: i32 = 77;
const VANILLA_ENTITY_TYPE_LLAMA_ID: i32 = 78;
const VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID: i32 = 79;
const VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID: i32 = 80;
const VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID: i32 = 81;
const VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID: i32 = 82;
const VANILLA_ENTITY_TYPE_MANNEQUIN_ID: i32 = 83;
const VANILLA_ENTITY_TYPE_MARKER_ID: i32 = 84;
const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;
const VANILLA_ENTITY_TYPE_MOOSHROOM_ID: i32 = 86;
const VANILLA_ENTITY_TYPE_MULE_ID: i32 = 87;
const VANILLA_ENTITY_TYPE_NAUTILUS_ID: i32 = 88;
const VANILLA_ENTITY_TYPE_OAK_BOAT_ID: i32 = 89;
const VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID: i32 = 90;
const VANILLA_ENTITY_TYPE_OCELOT_ID: i32 = 91;
const VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID: i32 = 92;
const VANILLA_ENTITY_TYPE_PAINTING_ID: i32 = 93;
const VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID: i32 = 94;
const VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID: i32 = 95;
const VANILLA_ENTITY_TYPE_PANDA_ID: i32 = 96;
const VANILLA_ENTITY_TYPE_PARCHED_ID: i32 = 97;
const VANILLA_ENTITY_TYPE_PARROT_ID: i32 = 98;
const VANILLA_ENTITY_TYPE_PHANTOM_ID: i32 = 99;
const VANILLA_ENTITY_TYPE_PIG_ID: i32 = 100;
const VANILLA_ENTITY_TYPE_PIGLIN_ID: i32 = 101;
const VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID: i32 = 102;
const VANILLA_ENTITY_TYPE_PILLAGER_ID: i32 = 103;
const VANILLA_ENTITY_TYPE_POLAR_BEAR_ID: i32 = 104;
const VANILLA_ENTITY_TYPE_SPLASH_POTION_ID: i32 = 105;
const VANILLA_ENTITY_TYPE_LINGERING_POTION_ID: i32 = 106;
const VANILLA_ENTITY_TYPE_PUFFERFISH_ID: i32 = 107;
const VANILLA_ENTITY_TYPE_RABBIT_ID: i32 = 108;
const VANILLA_ENTITY_TYPE_RAVAGER_ID: i32 = 109;
const VANILLA_ENTITY_TYPE_SALMON_ID: i32 = 110;
const VANILLA_ENTITY_TYPE_SHEEP_ID: i32 = 111;
const VANILLA_ENTITY_TYPE_SHULKER_ID: i32 = 112;
const VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID: i32 = 113;
const VANILLA_ENTITY_TYPE_SILVERFISH_ID: i32 = 114;
const VANILLA_ENTITY_TYPE_SKELETON_ID: i32 = 115;
const VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID: i32 = 116;
const VANILLA_ENTITY_TYPE_SLIME_ID: i32 = 117;
const VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID: i32 = 118;
const VANILLA_ENTITY_TYPE_SNIFFER_ID: i32 = 119;
const VANILLA_ENTITY_TYPE_SNOWBALL_ID: i32 = 120;
const VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID: i32 = 121;
const VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID: i32 = 122;
const VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID: i32 = 123;
const VANILLA_ENTITY_TYPE_SPIDER_ID: i32 = 124;
const VANILLA_ENTITY_TYPE_SPRUCE_BOAT_ID: i32 = 125;
const VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID: i32 = 126;
const VANILLA_ENTITY_TYPE_SQUID_ID: i32 = 127;
const VANILLA_ENTITY_TYPE_STRAY_ID: i32 = 128;
const VANILLA_ENTITY_TYPE_STRIDER_ID: i32 = 129;
const VANILLA_ENTITY_TYPE_TADPOLE_ID: i32 = 130;
const VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID: i32 = 131;
const VANILLA_ENTITY_TYPE_TNT_ID: i32 = 132;
const VANILLA_ENTITY_TYPE_TNT_MINECART_ID: i32 = 133;
const VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID: i32 = 134;
const VANILLA_ENTITY_TYPE_TRIDENT_ID: i32 = 135;
const VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID: i32 = 136;
const VANILLA_ENTITY_TYPE_TURTLE_ID: i32 = 137;
const VANILLA_ENTITY_TYPE_VEX_ID: i32 = 138;
const VANILLA_ENTITY_TYPE_VILLAGER_ID: i32 = 139;
const VANILLA_ENTITY_TYPE_VINDICATOR_ID: i32 = 140;
const VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID: i32 = 141;
const VANILLA_ENTITY_TYPE_WARDEN_ID: i32 = 142;
const VANILLA_ENTITY_TYPE_WIND_CHARGE_ID: i32 = 143;
const VANILLA_ENTITY_TYPE_WITCH_ID: i32 = 144;
const VANILLA_ENTITY_TYPE_WITHER_ID: i32 = 145;
const VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID: i32 = 146;
const VANILLA_ENTITY_TYPE_WITHER_SKULL_ID: i32 = 147;
const VANILLA_ENTITY_TYPE_WOLF_ID: i32 = 148;
const VANILLA_ENTITY_TYPE_ZOGLIN_ID: i32 = 149;
const VANILLA_ENTITY_TYPE_ZOMBIE_ID: i32 = 150;
const VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID: i32 = 151;
const VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID: i32 = 152;
const VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID: i32 = 153;
const VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID: i32 = 154;
const VANILLA_ENTITY_TYPE_PLAYER_ID: i32 = 155;
const VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID: i32 = 156;

/// The thrown-item projectiles whose vanilla `ThrownItemRenderer` draws a camera-facing item sprite,
/// paired with that renderer's sprite scale (`poseStack.scale(scale)`). They render through the
/// item-entity billboard layer rather than the 3D model scene (see
/// [`crate::item_entities::item_entity_billboards_from_world`]). Most are unit scale; the two fireballs
/// are the only non-unit `ThrownItemRenderer` registrations (`fireball` ×3.0, `small_fireball` ×0.75).
pub(crate) const THROWN_ITEM_PROJECTILE_BILLBOARDS: &[(i32, f32)] = &[
    (VANILLA_ENTITY_TYPE_EGG_ID, 1.0),
    (VANILLA_ENTITY_TYPE_ENDER_PEARL_ID, 1.0),
    (VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID, 1.0),
    (VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, 1.0),
    (VANILLA_ENTITY_TYPE_SPLASH_POTION_ID, 1.0),
    (VANILLA_ENTITY_TYPE_LINGERING_POTION_ID, 1.0),
    (VANILLA_ENTITY_TYPE_SNOWBALL_ID, 1.0),
    (VANILLA_ENTITY_TYPE_FIREBALL_ID, 3.0),
    (VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID, 0.75),
];
const AVATAR_MODEL_CUSTOMIZATION_DATA_ID: u8 = 16;
const AVATAR_PLAYER_DEFAULT_MODEL_CUSTOMIZATION: i8 = 0;
const MANNEQUIN_DEFAULT_MODEL_CUSTOMIZATION: i8 = PlayerModelPartVisibility::ALL_MASK as i8;
const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
const ENTITY_SHARED_FLAG_ON_FIRE: i8 = 0x01;
const ENTITY_SHARED_FLAG_INVISIBLE: i8 = 0x20;
const ENTITY_SHARED_FLAG_GLOWING: i8 = 0x40;
const ENTITY_CUSTOM_NAME_DATA_ID: u8 = 2;
const AGEABLE_MOB_BABY_DATA_ID: u8 = 16;
const ZOMBIE_BABY_DATA_ID: u8 = 16;
/// Vanilla `Zombie.DATA_DROWNED_CONVERSION_ID` (synced boolean): `Entity` 0..=7,
/// `LivingEntity` 8..=14, `Mob` 15, `Zombie` baby 16 / special-type 17 /
/// drowned-conversion 18. Drives `Zombie.isUnderWaterConverting()`.
const ZOMBIE_DROWNED_CONVERSION_DATA_ID: u8 = 18;
/// Vanilla `ZombieVillager.DATA_CONVERTING_ID` (synced boolean, id 19, right
/// after the inherited `Zombie` ids). Drives `ZombieVillager.isConverting()`.
const ZOMBIE_VILLAGER_CONVERTING_DATA_ID: u8 = 19;
/// Vanilla `Villager.DATA_VILLAGER_DATA` (synced `VillagerData`, serializer 18):
/// `AbstractVillager.DATA_UNHAPPY_COUNTER` occupies id 18, so the first `Villager` own accessor is id 19.
const VILLAGER_DATA_DATA_ID: u8 = 19;
/// Vanilla `ZombieVillager.DATA_VILLAGER_DATA`: `Zombie` consumes baby 16, special-type 17, and
/// drowned-conversion 18; `ZombieVillager` then defines converting 19 before villager data at 20.
const ZOMBIE_VILLAGER_DATA_DATA_ID: u8 = 20;
const PIGLIN_BABY_DATA_ID: u8 = 17;
const BOGGED_SHEARED_DATA_ID: u8 = 16;
/// Vanilla `CopperGolem.DATA_WEATHER_STATE` data id (16): `CopperGolem` extends `AbstractGolem`
/// without adding inherited synced data after `Mob.DATA_MOB_FLAGS_ID` (15), so its first own
/// accessor is the weathering enum.
const COPPER_GOLEM_WEATHER_STATE_DATA_ID: u8 = 16;
/// Vanilla `Armadillo.ARMADILLO_STATE` data id (18): the synced `ArmadilloState` enum, the
/// armadillo's first own accessor after `AgeableMob.AGE_LOCKED` (17). Matches the
/// `Sniffer.DATA_STATE` slot (both `extends Animal`).
const ARMADILLO_STATE_DATA_ID: u8 = 18;
/// Vanilla `Armadillo.ArmadilloState.SCARED` ordinal (2): the steady rolled-into-a-ball state
/// whose `shouldHideInShell` is `true` for every `inStateTicks` (server-derivable, unlike the
/// tick-gated ROLLING/UNROLLING transitions).
const ARMADILLO_STATE_SCARED_ID: i32 = 2;
/// Vanilla `SpellcasterIllager.DATA_SPELL_CASTING_ID` data id (17): the byte holding the spell
/// id, the first `SpellcasterIllager` accessor after `Raider.IS_CELEBRATING` (16). The byte is
/// `> 0` while `isCastingSpell()`.
const SPELLCASTER_ILLAGER_CASTING_DATA_ID: u8 = 17;
/// Vanilla `Pillager.IS_CHARGING_CROSSBOW` data id (17): the boolean set while the pillager draws its
/// crossbow, the first `Pillager` accessor after `Raider.IS_CELEBRATING` (16) — the same slot the
/// spellcaster illagers use for `DATA_SPELL_CASTING_ID`, since each illager subclass adds its own
/// accessor in that position. `getArmPose` returns `CROSSBOW_CHARGE` while true, else `CROSSBOW_HOLD`.
const PILLAGER_IS_CHARGING_CROSSBOW_DATA_ID: u8 = 17;
/// Vanilla `Raider.IS_CELEBRATING` data id (16): the boolean set while a raider celebrates a raid
/// victory, the first `Raider` accessor after `Mob.DATA_MOB_FLAGS_ID` (15). The evoker and vindicator
/// `getArmPose` return `CELEBRATING` while it is true (when not casting / not aggressive).
const RAIDER_IS_CELEBRATING_DATA_ID: u8 = 16;
/// Vanilla `Piglin.DATA_IS_DANCING` data id (19): the boolean set while a piglin dances by a soul
/// campfire. The piglin's accessors follow `AbstractPiglin.DATA_IMMUNE_TO_ZOMBIFICATION` (16) — the
/// first accessor after `Mob.DATA_MOB_FLAGS_ID` (15) — then `DATA_BABY_ID` (17) and
/// `DATA_IS_CHARGING_CROSSBOW` (18). `Piglin.getArmPose` returns `DANCING` (top priority) while true.
const PIGLIN_IS_DANCING_DATA_ID: u8 = 19;
/// Vanilla `Piglin.DATA_IS_CHARGING_CROSSBOW` data id (18) — see [`PIGLIN_IS_DANCING_DATA_ID`] for the
/// piglin accessor chain. `Piglin.getArmPose` returns `CROSSBOW_CHARGE` (whose pull-back pose needs the
/// deferred use-tick state) while true, suppressing the `CROSSBOW_HOLD` level-the-crossbow pose.
const PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID: u8 = 18;
// `ArmorStand extends LivingEntity` directly — it is NOT a `Mob`, so there is no
// `Mob.DATA_MOB_FLAGS_ID` (15); `ArmorStand.DATA_CLIENT_FLAGS` is the first accessor after
// `LivingEntity` (0-14) and lands at 15, with the six pose rotations following at 16-21.
const ARMOR_STAND_CLIENT_FLAGS_DATA_ID: u8 = 15;
const ARMOR_STAND_HEAD_POSE_DATA_ID: u8 = 16;
const ARMOR_STAND_BODY_POSE_DATA_ID: u8 = 17;
const ARMOR_STAND_LEFT_ARM_POSE_DATA_ID: u8 = 18;
const ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID: u8 = 19;
const ARMOR_STAND_LEFT_LEG_POSE_DATA_ID: u8 = 20;
const ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID: u8 = 21;
const ARMOR_STAND_CLIENT_FLAG_SMALL: i8 = 1;
const ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS: i8 = 4;
const ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE: i8 = 8;
const ARMOR_STAND_CLIENT_FLAG_MARKER: i8 = 16;
const SLIME_SIZE_DATA_ID: u8 = 16;
const SLIME_DEFAULT_SIZE: i32 = 1;
// Phantom (`Phantom.ID_SIZE`, the first Mob-subclass synced data, index 16). Defaults to 0.
const PHANTOM_SIZE_DATA_ID: u8 = 16;
const PHANTOM_DEFAULT_SIZE: i32 = 0;
// Pufferfish (`Pufferfish.PUFF_STATE`): AbstractFish defines FROM_BUCKET at index 16, so the
// puff state is index 17. Defaults to 0 (deflated).
const PUFFERFISH_PUFF_STATE_DATA_ID: u8 = 17;
const PUFFERFISH_DEFAULT_PUFF_STATE: i32 = 0;
// Salmon (`Salmon.DATA_TYPE`): AbstractFish defines FROM_BUCKET at index 16, so the size
// variant is index 17. Defaults to 1 (`Salmon.Variant.MEDIUM`).
const SALMON_VARIANT_DATA_ID: u8 = 17;
const SALMON_DEFAULT_VARIANT: i32 = 1;
// Tropical fish (`TropicalFish.DATA_ID_TYPE_VARIANT`): AbstractFish defines FROM_BUCKET at
// index 16 (AbstractSchoolingFish adds no synced data), so the packed variant is index 17.
// Defaults to 0 (`DEFAULT_VARIANT` = KOB/white/white, whose pattern bits are 0 → small body).
const TROPICAL_FISH_VARIANT_DATA_ID: u8 = 17;
const TROPICAL_FISH_DEFAULT_VARIANT: i32 = 0;
const ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID: u8 = 19;
/// `Horse.DATA_ID_TYPE_VARIANT` data id (19, INT): packs `color | markings << 8`. `Horse` extends
/// `AbstractHorse` (DATA_ID_FLAGS 18) extends `Animal`/`AgeableMob` (baby 16, age-locked 17), so the
/// horse's first own accessor is 19. The coat color is `variant & 0xFF`; markings (`>> 8`) deferred.
const HORSE_VARIANT_DATA_ID: u8 = 19;
const LLAMA_VARIANT_DATA_ID: u8 = 21;
const GOAT_LEFT_HORN_DATA_ID: u8 = 19;
const GOAT_RIGHT_HORN_DATA_ID: u8 = 20;
const CHICKEN_VARIANT_DATA_ID: u8 = 18;
const COW_VARIANT_DATA_ID: u8 = 18;
/// `MushroomCow.DATA_TYPE` data id (20, INT): the red/brown variant. `MushroomCow` extends `Cow`,
/// whose own accessors are `DATA_VARIANT_ID` (18) + `DATA_SOUND_VARIANT_ID` (19), so the mooshroom's
/// first own accessor is 20.
const MUSHROOM_COW_TYPE_DATA_ID: u8 = 20;
const PIG_VARIANT_DATA_ID: u8 = 19;
// Vanilla Frog.DATA_VARIANT_ID (18, Holder<FrogVariant>): `Frog extends Animal`, so its first own
// accessor follows Mob.DATA_MOB_FLAGS_ID (15) and the two AgeableMob accessors DATA_BABY_ID (16) /
// AGE_LOCKED (17).
const FROG_VARIANT_DATA_ID: u8 = 18;
// Vanilla Fox.DATA_TYPE_ID (18, INT): the first `Fox` accessor (`Fox extends Animal`), holding the
// `Fox.Variant` id; `Fox.DATA_FLAGS_ID` follows at 19.
const FOX_TYPE_DATA_ID: u8 = 18;
// Vanilla Axolotl.DATA_VARIANT (18, INT): the first `Axolotl` accessor (`Axolotl extends Animal`),
// holding the `Axolotl.Variant` id; DATA_PLAYING_DEAD / FROM_BUCKET follow at 19/20.
const AXOLOTL_VARIANT_DATA_ID: u8 = 18;
// Vanilla Rabbit.DATA_TYPE_ID (18, INT): the first `Rabbit` accessor (`Rabbit extends Animal`),
// holding the `Rabbit.Variant` id (note EVIL is id 99).
const RABBIT_TYPE_DATA_ID: u8 = 18;
// Vanilla Shulker.DATA_COLOR_ID (18, BYTE): after Mob.DATA_MOB_FLAGS_ID (15) and the two `Shulker`
// accessors DATA_ATTACH_FACE_ID (16) / DATA_PEEK_ID (17). `getColor()` returns the dye for 0..=15
// and `null` (the default, byte 16) otherwise.
const SHULKER_COLOR_DATA_ID: u8 = 18;
// Vanilla Parrot.DATA_VARIANT_ID (20, INT): after Mob.DATA_MOB_FLAGS_ID (15), the two AgeableMob
// accessors DATA_BABY_ID (16) / AGE_LOCKED (17), and the two TamableAnimal accessors DATA_FLAGS_ID
// (18) / DATA_OWNERUUID_ID (19).
const PARROT_VARIANT_DATA_ID: u8 = 20;
// Vanilla Cat.DATA_VARIANT_ID (20, Holder<CatVariant>): after Mob.DATA_MOB_FLAGS_ID (15), the two
// AgeableMob accessors DATA_BABY_ID (16) / AGE_LOCKED (17), and the two TamableAnimal accessors
// DATA_FLAGS_ID (18) / DATA_OWNERUUID_ID (19). `Cat extends TamableAnimal`; the ocelot has no breed.
const CAT_VARIANT_DATA_ID: u8 = 20;
// Vanilla Cat.DATA_COLLAR_COLOR (23, INT): after DATA_VARIANT_ID (20) come IS_LYING (21) /
// RELAX_STATE_ONE (22) / DATA_COLLAR_COLOR (23). `getCollarColor()` = `DyeColor.byId`, default RED (14).
const CAT_COLLAR_COLOR_DATA_ID: u8 = 23;
const CAT_DEFAULT_COLLAR_COLOR_ID: i32 = 14;
// Vanilla Sheep.DATA_WOOL_ID (18, BYTE): `Sheep extends Animal`, so its first own accessor follows
// Mob.DATA_MOB_FLAGS_ID (15) and the two AgeableMob accessors DATA_BABY_ID (16) / AGE_LOCKED (17).
const SHEEP_WOOL_DATA_ID: u8 = 18;
const SHEEP_WOOL_COLOR_MASK: u8 = 0x0f;
const SHEEP_WOOL_SHEARED_FLAG: u8 = 0x10;
// Vanilla Panda counters / genes / flags: `Panda extends Animal`, so after Mob.DATA_MOB_FLAGS_ID (15) and
// the two AgeableMob accessors (16/17) come UNHAPPY (18), SNEEZE (19), EAT (20), MAIN_GENE (21),
// HIDDEN_GENE (22), and DATA_ID_FLAGS (23).
const PANDA_EAT_COUNTER_DATA_ID: u8 = 20;
const PANDA_MAIN_GENE_DATA_ID: u8 = 21;
const PANDA_HIDDEN_GENE_DATA_ID: u8 = 22;
/// Vanilla `Panda.UNHAPPY_COUNTER` (18, INT): `> 0` drives `PandaRenderState.isUnhappy`, the head-shake
/// + front-leg paddle. `Panda.SNEEZE_COUNTER` (19, INT) is the `sneezeTime` ramp; the `isSneezing` flag
/// is `Panda.DATA_ID_FLAGS` (23, BYTE) bit `0x02` (`getFlag(2)`).
const PANDA_UNHAPPY_COUNTER_DATA_ID: u8 = 18;
const PANDA_SNEEZE_COUNTER_DATA_ID: u8 = 19;
const PANDA_FLAGS_DATA_ID: u8 = 23;
const PANDA_SNEEZING_FLAG: i8 = 0x02;
#[cfg(test)]
const PANDA_ROLLING_FLAG: i8 = 0x04;
const PANDA_SITTING_FLAG: i8 = 0x08;
#[cfg(test)]
const PANDA_ON_BACK_FLAG: i8 = 0x10;
// Vanilla Strider.DATA_SUFFOCATING (19, BOOLEAN): `Strider extends Animal`, so after Mob (15), the
// two AgeableMob accessors (16/17), and DATA_BOOST_TIME (18) comes the cold/suffocating flag.
const STRIDER_SUFFOCATING_DATA_ID: u8 = 19;
// Vanilla Ghast.DATA_IS_CHARGING (16, BOOLEAN): `Ghast extends Mob` directly (NOT AgeableMob), so its
// first own accessor lands right after Mob's DATA_MOB_FLAGS_ID (15) — index 16, no baby/age-locked slots.
const GHAST_IS_CHARGING_DATA_ID: u8 = 16;
// Vanilla Vex.DATA_FLAGS_ID (16, BYTE): `Vex extends Monster extends PathfinderMob extends Mob`, so its
// first own accessor lands right after Mob's DATA_MOB_FLAGS_ID (15) — index 16. `isCharging` is bit 1.
const VEX_FLAGS_DATA_ID: u8 = 16;
const VEX_FLAG_IS_CHARGING: i8 = 1;
// Vanilla WitherSkull.DATA_DANGEROUS (8, BOOLEAN): `Entity` defines ids 0..=7, while `Projectile` and
// `AbstractHurtingProjectile` add no synced accessors, so the wither skull's first own accessor lands at 8.
const WITHER_SKULL_DANGEROUS_DATA_ID: u8 = 8;
// Vanilla Arrow.ID_EFFECT_COLOR (11, INT): `Arrow extends AbstractArrow extends Projectile extends
// Entity`, so after Entity (0-7) come the three AbstractArrow accessors ID_FLAGS (8) / PIERCE_LEVEL
// (9) / IN_GROUND (10), then Arrow's own potion color. `getColor() > 0` marks a tipped arrow.
const ARROW_EFFECT_COLOR_DATA_ID: u8 = 11;
// Vanilla `AbstractArrow.IN_GROUND` (10, BOOLEAN): updating it to true after the first client tick
// starts the seven-tick `shakeTime` impact wobble that `ArrowRenderer.extractRenderState` projects.
#[cfg(test)]
const ARROW_IN_GROUND_DATA_ID: u8 = 10;
// Vanilla `ThrownTrident.ID_FOIL` (12, BOOLEAN): after Entity (0-7) and AbstractArrow's
// ID_FLAGS / PIERCE_LEVEL / IN_GROUND (8/9/10), trident defines ID_LOYALTY (11) then ID_FOIL (12).
const TRIDENT_FOIL_DATA_ID: u8 = 12;
const TAMABLE_ANIMAL_FLAGS_DATA_ID: u8 = 18;
const TAMABLE_ANIMAL_TAME_FLAG: i8 = 0x04;
/// Vanilla `Creaking.IS_ACTIVE` data id (17, BOOLEAN): `Creaking extends Monster` (not ageable), so
/// after `Mob.DATA_MOB_FLAGS_ID` (15) come `CAN_MOVE` (16) / `IS_ACTIVE` (17) / `IS_TEARING_DOWN`
/// (18). The renderer's `eyesGlowing` is `isActive()` for a live creaking (the death-flicker is deferred).
const CREAKING_IS_ACTIVE_DATA_ID: u8 = 17;
/// Vanilla `Turtle.HAS_EGG` data id (18): the synced boolean, the turtle's first own accessor
/// after `AgeableMob.DATA_BABY_ID` (16) and `AGE_LOCKED` (17).
const TURTLE_HAS_EGG_DATA_ID: u8 = 18;
/// Vanilla `Turtle.LAYING_EGG` data id (19): the synced boolean declared right after `HAS_EGG`.
const TURTLE_LAYING_EGG_DATA_ID: u8 = 19;
/// Vanilla `EndCrystal.DATA_SHOW_BOTTOM` data id (9): EndCrystal extends Entity directly (0-7),
/// then declares `DATA_BEAM_TARGET` (8) and `DATA_SHOW_BOTTOM` (9). Synced boolean, default `true`.
const END_CRYSTAL_SHOW_BOTTOM_DATA_ID: u8 = 9;
/// `TamableAnimal` `DATA_FLAGS_ID` sitting bit (`isInSittingPose()` reads `& 1`).
const TAMABLE_ANIMAL_SITTING_FLAG: i8 = 0x01;
const WOLF_COLLAR_COLOR_DATA_ID: u8 = 21;
const WOLF_ANGER_END_TIME_DATA_ID: u8 = 22;
const WOLF_DEFAULT_COLLAR_COLOR_ID: i32 = 14;
/// `Wolf.DATA_VARIANT_ID` data id (23): the synced `Holder<WolfVariant>` registry id, defined right
/// after `DATA_ANGER_END_TIME` (22) and before `DATA_SOUND_VARIANT_ID` (24). `WolfRenderer` keys the
/// base texture on the resolved variant.
const WOLF_VARIANT_DATA_ID: u8 = 23;
/// `ZombieNautilus.DATA_VARIANT_ID` data id (21): the synced `Holder<ZombieNautilusVariant>` registry
/// id. `AbstractNautilus extends TamableAnimal` adds flags(18)+owner(19) then DASH(20), so the
/// `ZombieNautilus`-own variant lands at 21.
const ZOMBIE_NAUTILUS_VARIANT_DATA_ID: u8 = 21;
/// `Bee.DATA_FLAGS_ID` data id (18, BYTE): the bee flags byte, the first `Bee`-own accessor
/// (`AgeableMob` consumes 16 baby + 17 age-locked, so `Bee` starts at 18).
const BEE_FLAGS_DATA_ID: u8 = 18;
/// `Bee.FLAG_HAS_NECTAR` (8): the `DATA_FLAGS_ID` bit set while the bee carries pollen, which
/// `BeeRenderer.getTextureLocation` swaps to the `*_nectar*` texture.
const BEE_FLAG_HAS_NECTAR: i8 = 8;
/// `Bee.DATA_ANGER_END_TIME` data id (19): the synced `NeutralMob` anger-end game time,
/// defined right after `Bee.DATA_FLAGS_ID` (18). `Bee.isAngry()` is `endTime > 0 &&
/// endTime - gameTime > 0`.
const BEE_ANGER_END_TIME_DATA_ID: u8 = 19;
/// `Camel.LAST_POSE_CHANGE_TICK` data id (20): the synced Long that drives the camel's
/// sit/sit-pose/stand-up timing. Its magnitude is the game tick of the last pose change and
/// its SIGN encodes whether the camel is sitting (`< 0` → sitting). Defined right after
/// `Camel.DASH` (19), themselves following `AbstractHorse.DATA_ID_FLAGS` (18).
const CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID: u8 = 20;
/// Vanilla `Camel.SITDOWN_DURATION_TICKS` (40 ticks = 2.0 s `CAMEL_SIT`): the sit-down
/// window, after which `CAMEL_SIT_POSE` takes over.
const CAMEL_SITDOWN_DURATION_TICKS: i64 = 40;
/// Vanilla `Camel.STANDUP_DURATION_TICKS` (52 ticks = 2.6 s `CAMEL_STANDUP`): the stand-up
/// transition window.
const CAMEL_STANDUP_DURATION_TICKS: i64 = 52;
/// `LivingEntity.DATA_HEALTH_ID` — the synced current-health float.
const LIVING_ENTITY_HEALTH_DATA_ID: u8 = 9;

pub(crate) fn entity_scene_outline_from_world_at_partial_tick(
    world: &WorldStore,
    entity_partial_tick: f32,
) -> Option<SelectionOutline> {
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.local_player().camera.entity_id;
    let boxes: Vec<_> = world
        .entity_pick_targets_at_partial_tick(entity_partial_tick.clamp(0.0, 1.0))
        .into_iter()
        .filter(|target| {
            local_player_id != Some(target.entity_id) && camera_entity_id != Some(target.entity_id)
        })
        .map(entity_pick_target_box)
        .collect();
    (!boxes.is_empty()).then(|| SelectionOutline::from_boxes(boxes))
}

pub(crate) fn entity_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_partial_tick: f32,
) -> Vec<EntityModelInstance> {
    let entity_partial_tick = entity_partial_tick.clamp(0.0, 1.0);
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.local_player().camera.entity_id;
    let chicken_variants = world.registry_content("minecraft:chicken_variant");
    let cow_variants = world.registry_content("minecraft:cow_variant");
    let pig_variants = world.registry_content("minecraft:pig_variant");
    let frog_variants = world.registry_content("minecraft:frog_variant");
    let cat_variants = world.registry_content("minecraft:cat_variant");
    let wolf_variants = world.registry_content("minecraft:wolf_variant");
    let villager_types = world.registry_content("minecraft:villager_type");
    let villager_professions = world.registry_content("minecraft:villager_profession");
    let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
    world
        .entity_model_sources_at_partial_tick(entity_partial_tick)
        .into_iter()
        .filter(|source| {
            local_player_id != Some(source.entity_id) && camera_entity_id != Some(source.entity_id)
        })
        .filter_map(|source| {
            entity_model_instance(
                source,
                world,
                item_runtime,
                game_time,
                entity_partial_tick,
                chicken_variants,
                cow_variants,
                pig_variants,
                frog_variants,
                cat_variants,
                wolf_variants,
                villager_types,
                villager_professions,
            )
        })
        .collect()
}

/// Whether the entity's main-hand item is a bow (vanilla `SkeletonRenderState.isHoldingBow =
/// getMainHandItem().is(Items.BOW)`), driving the skeleton's `BOW_AND_ARROW` aim pose. Resolved through
/// the item registry, so it needs the runtime; `false` without it or for any non-bow / empty hand.
fn entity_main_hand_holds_bow(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
) -> bool {
    entity_main_hand_is_item(world, item_runtime, entity_id, "minecraft:bow")
}

/// Whether the entity's main-hand item resolves to a specific item resource id. Used for renderer
/// states whose vanilla extraction calls `getMainHandItem().is(Items.X)`.
fn entity_main_hand_is_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    resource_id: &str,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, false) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some(resource_id)
}

/// Whether the item in the given hand is a trident (vanilla `Items.TRIDENT`). Drives the drowned's
/// `THROW_TRIDENT` raised-arm pose (`DrownedRenderer.getArmPose`'s `item.is(Items.TRIDENT)`, main hand) and
/// the player's use-item `THROW_TRIDENT` charge pose (`TridentItem.getUseAnimation() == TRIDENT`, either
/// hand). Resolved through the item registry, so it needs the runtime; `false` without it or for any
/// non-trident / empty hand.
fn entity_hand_holds_trident(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:trident")
}

/// Whether the item in the given hand is a bow (vanilla `BowItem.getUseAnimation() == BOW`, which only
/// `minecraft:bow` returns). While the entity draws it, `HumanoidModel.poseRightArm`/`poseLeftArm`
/// `BOW_AND_ARROW` raises BOTH arms along the head look (the pose is two-handed + affectsOffhandPose, so the
/// opposite arm's pose is skipped). Resolved through the item registry; `false` without it or for any
/// non-bow / empty hand.
fn entity_hand_holds_bow(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:bow")
}

/// Whether the item in the given hand is a spear — one of the seven tool-material spears whose item
/// prototype sets `DataComponents.SWING_ANIMATION` to `SwingAnimationType.STAB` in `Item.Properties.spear(...)`.
/// A spear's melee swing uses the `STAB` `SpearAnimations.thirdPersonAttackHand` pose instead of the default
/// `WHACK`. That STAB default lives on the item PROTOTYPE (not the network component patch), so it is detected
/// by the resolved item id rather than a component-presence check (a datapack explicitly overriding
/// `SWING_ANIMATION` on a non-spear item is a deferred edge case). Resolved through the item registry; `false`
/// without it or for any non-spear / empty hand.
fn entity_hand_holds_spear(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    matches!(
        item_runtime.item_resource_id(item_id),
        Some(
            "minecraft:wooden_spear"
                | "minecraft:stone_spear"
                | "minecraft:copper_spear"
                | "minecraft:iron_spear"
                | "minecraft:golden_spear"
                | "minecraft:diamond_spear"
                | "minecraft:netherite_spear"
        )
    )
}

/// Whether the item in the given hand is a spyglass (vanilla `ItemStack.getUseAnimation() ==
/// ItemUseAnimation.SPYGLASS`, which only `minecraft:spyglass` returns). While the entity is using it,
/// `HumanoidModel.poseRightArm`/`poseLeftArm` raise that arm to hold the spyglass to the eye. Resolved
/// through the item registry; `false` without it or for any other / empty hand.
fn entity_hand_holds_spyglass(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:spyglass")
}

/// Whether the item in the given hand is a goat horn (vanilla `ItemStack.getUseAnimation() ==
/// ItemUseAnimation.TOOT_HORN`, which only `InstrumentItem` / `minecraft:goat_horn` returns). While the
/// entity is tooting it, `HumanoidModel.poseRightArm`/`poseLeftArm` raise that arm to the mouth. Resolved
/// through the item registry; `false` without it or for any other / empty hand.
fn entity_hand_holds_goat_horn(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:goat_horn")
}

/// Whether the item in the given hand is a brush (vanilla `ItemStack.getUseAnimation() ==
/// ItemUseAnimation.BRUSH`, which only `BrushItem` / `minecraft:brush` returns). While the entity is
/// brushing, `HumanoidModel.poseRightArm`/`poseLeftArm` lower that arm to the brushed block. Resolved
/// through the item registry; `false` without it or for any other / empty hand.
fn entity_hand_holds_brush(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:brush")
}

/// Vanilla `DataComponents.CONSUMABLE` network type id (24, the registry order of `minecraft:consumable` in
/// `DataComponents`). `Item.getUseAnimation` checks `CONSUMABLE` before `BLOCKS_ATTACKS`, so a stack patch
/// that adds both should stay on EAT/DRINK rather than the block pose.
const DATA_COMPONENT_CONSUMABLE_TYPE_ID: i32 = 24;

/// Vanilla `DataComponents.BLOCKS_ATTACKS` network type id (37, the registry order of
/// `minecraft:blocks_attacks` in `DataComponents`). `Item.getUseAnimation` returns `BLOCK` for a non-consumable
/// item carrying this component.
const DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID: i32 = 37;

fn component_patch_has_added_component(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    type_id: i32,
) -> bool {
    patch.added_type_ids.contains(&type_id) && !patch.removed_type_ids.contains(&type_id)
}

/// Whether the item in the given hand has the `BLOCK` use-animation (vanilla `ItemStack.getUseAnimation() ==
/// ItemUseAnimation.BLOCK`, returned by `Item.getUseAnimation` for a non-consumable item carrying
/// `DataComponents.BLOCKS_ATTACKS`). While the entity raises it, `HumanoidModel.poseRightArm`/`poseLeftArm`
/// `poseBlockingArm` tucks that arm's blocking item forward. The vanilla shield is detected by resolved item
/// id because its component is a prototype default; datapack/patch-granted `blocks_attacks` is detected from
/// `added_type_ids` and does not need the item registry.
fn entity_hand_blocks_attacks(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    if component_patch_has_added_component(
        &stack.component_patch,
        DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    ) {
        return false;
    }
    if component_patch_has_added_component(
        &stack.component_patch,
        DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
    ) {
        return true;
    }
    if stack
        .component_patch
        .removed_type_ids
        .contains(&DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID)
    {
        return false;
    }
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:shield")
}

/// Whether the item in the given hand is a crossbow (vanilla `Pillager.isHolding(Items.CROSSBOW)` for the
/// pillager's `CROSSBOW_HOLD`/`CROSSBOW_CHARGE`; also the player's crossbow use poses). Resolved through the
/// item registry, so it needs the runtime; `false` without it or for any non-crossbow / empty hand.
fn entity_hand_holds_crossbow(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:crossbow")
}

/// Whether the item in the given hand routes to a SPECIAL `getArmPose` (not the `ITEM` fallback) when used:
/// the use-animation poses `BOW_AND_ARROW` (bow), `CROSSBOW_CHARGE` (crossbow), `THROW_TRIDENT` (trident),
/// `BLOCK` (non-consumable `BLOCKS_ATTACKS` item, normally the shield), `SPYGLASS`, `TOOT_HORN` (goat horn),
/// `BRUSH`. While the entity uses one of these in this hand, that hand gets its dedicated pose instead of
/// `ITEM`; any OTHER used item (food/potion -> `EAT`/`DRINK`, or any plain item) falls through to `ITEM`.
fn entity_hand_holds_special_use_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    entity_hand_holds_bow(world, item_runtime, entity_id, off_hand)
        || entity_hand_holds_crossbow(world, item_runtime, entity_id, off_hand)
        || entity_hand_holds_trident(world, item_runtime, entity_id, off_hand)
        || entity_hand_blocks_attacks(world, item_runtime, entity_id, off_hand)
        || entity_hand_holds_spyglass(world, item_runtime, entity_id, off_hand)
        || entity_hand_holds_goat_horn(world, item_runtime, entity_id, off_hand)
        || entity_hand_holds_brush(world, item_runtime, entity_id, off_hand)
}

/// Whether the item in the given hand is a *charged* crossbow (vanilla
/// `isHolding(Items.CROSSBOW) && CrossbowItem.isCharged(getWeaponItem())`), driving the piglin's
/// `CROSSBOW_HOLD` arm pose. `CrossbowItem.isCharged` is the held crossbow's `minecraft:charged_projectiles`
/// component being non-empty (decoded into the held stack's component patch). Resolved through the item
/// registry; `false` without it or for any non-crossbow / empty / un-charged hand.
fn entity_hand_holds_charged_crossbow(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:crossbow")
        && !stack.component_patch.charged_projectiles_items.is_empty()
}

/// Vanilla `DataComponents.TOOL` network type id (28, the registry order of `minecraft:tool` in
/// `DataComponents`). `AbstractPiglin.isHoldingMeleeWeapon()` is `getMainHandItem().has(DataComponents.TOOL)`,
/// so a main-hand stack counts as a melee weapon when its decoded component patch added this type.
const DATA_COMPONENT_TOOL_TYPE_ID: i32 = 28;

/// Whether the entity's main-hand item is a melee weapon (vanilla
/// `AbstractPiglin.isHoldingMeleeWeapon()` = `getMainHandItem().has(DataComponents.TOOL)`), driving the
/// piglin/brute `ATTACKING_WITH_MELEE_WEAPON` arm pose when aggressive. The decoded component patch records
/// every added component's type id, so the `minecraft:tool` component shows up as
/// [`DATA_COMPONENT_TOOL_TYPE_ID`] in `added_type_ids` — no item-registry lookup is needed (unlike the
/// crossbow/bow checks, which resolve the item id). `false` for an empty hand or a non-tool main-hand item.
fn entity_main_hand_holds_melee_weapon(world: &WorldStore, entity_id: i32) -> bool {
    let Some(stack) = world.held_item(entity_id, false) else {
        return false;
    };
    stack
        .component_patch
        .added_type_ids
        .contains(&DATA_COMPONENT_TOOL_TYPE_ID)
}

/// Whether the entity's main hand holds any item at all. Vanilla `AvatarRenderer.getArmPose` falls back to
/// the `ITEM` arm pose for a non-empty main hand that is not a spear / charged crossbow / item-in-use; this is
/// the "is the main hand non-empty" half of that fallback. Resolved from the held-item summary only (no item
/// registry needed), so it works without the runtime; `false` for an empty hand.
fn entity_main_hand_non_empty(world: &WorldStore, entity_id: i32) -> bool {
    world
        .held_item(entity_id, false)
        .is_some_and(|stack| item_stack_non_empty(&stack))
}

/// Whether the entity's OFF hand holds any item at all. Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)`
/// likewise falls back to the `ITEM` arm pose for a non-empty off hand that is not a charged crossbow /
/// item-in-use; this is the "is the off hand non-empty" half of that fallback. Resolved from the held-item
/// summary only (no item registry needed); `false` for an empty off hand.
fn entity_offhand_non_empty(world: &WorldStore, entity_id: i32) -> bool {
    world
        .held_item(entity_id, true)
        .is_some_and(|stack| item_stack_non_empty(&stack))
}

fn item_stack_non_empty(stack: &ItemStackSummary) -> bool {
    stack.item_id.is_some() && stack.count > 0
}

/// The supported skull block item in the HEAD equipment slot, if any. Vanilla
/// `LivingEntityRenderer.extractRenderState` routes skull `BlockItem`s to `wornHeadType` and clears the
/// generic head item; bbb mirrors the implemented static mob, player-default, dragon, and piglin skull
/// branches.
fn entity_custom_head_skull(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
) -> Option<EntityCustomHeadSkull> {
    let item_runtime = item_runtime?;
    let stack = world.equipment_item(entity_id, EquipmentSlot::Head)?;
    item_runtime.custom_head_skull_for_stack(&stack)
}

/// Vanilla `ItemTags.PIGLIN_LOVED` tag id — the items a piglin admires.
const PIGLIN_LOVED_ITEM_TAG: &str = "minecraft:piglin_loved";

/// Whether the entity's OFFHAND item is a piglin-loved item (vanilla
/// `PiglinAi.isLovedItem(getOffhandItem())` = `getOffhandItem().is(ItemTags.PIGLIN_LOVED)`), driving the
/// regular piglin's `ADMIRING_ITEM` arm pose. Item tags arrive over the network (`UpdateTags`) into the
/// `minecraft:item` registry tag set, so membership is the offhand item's protocol id appearing in the
/// `minecraft:piglin_loved` tag — no item-registry lookup needed. `false` for an empty offhand, an unknown
/// id, or when the tag set hasn't been received.
fn entity_offhand_holds_loved_item(world: &WorldStore, entity_id: i32) -> bool {
    let Some(stack) = world.held_item(entity_id, true) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    world
        .registry_tags("minecraft:item")
        .and_then(|registry| registry.tags.get(PIGLIN_LOVED_ITEM_TAG))
        .is_some_and(|entries| entries.contains(&item_id))
}

/// Vanilla `Piglin.isChargingCrossbow()` (the synced `DATA_IS_CHARGING_CROSSBOW` boolean, id 18): the
/// piglin is drawing its crossbow, so `getArmPose` returns `CROSSBOW_CHARGE` rather than `CROSSBOW_HOLD`.
/// Only the regular piglin defines that accessor, so the projection is gated to its type.
fn piglin_is_charging_crossbow(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_PIGLIN_ID
        && entity_data_bool(values, PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID, false)
}

/// Vanilla `Pillager.isChargingCrossbow()` (the synced `IS_CHARGING_CROSSBOW` boolean, id 17): the
/// pillager is drawing its crossbow, so `getArmPose` returns `CROSSBOW_CHARGE` rather than
/// `CROSSBOW_HOLD`. Only the pillager defines that accessor, so the projection is gated to its type.
fn pillager_is_charging_crossbow(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_PILLAGER_ID
        && entity_data_bool(values, PILLAGER_IS_CHARGING_CROSSBOW_DATA_ID, false)
}

fn entity_pick_target_box(target: EntityPickTargetState) -> SelectionBox {
    SelectionBox {
        min: [
            (target.position.x + f64::from(target.bounds.min[0])) as f32,
            (target.position.y + f64::from(target.bounds.min[1])) as f32,
            (target.position.z + f64::from(target.bounds.min[2])) as f32,
        ],
        max: [
            (target.position.x + f64::from(target.bounds.max[0])) as f32,
            (target.position.y + f64::from(target.bounds.max[1])) as f32,
            (target.position.z + f64::from(target.bounds.max[2])) as f32,
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn entity_model_instance(
    source: EntityModelSourceState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    game_time: i64,
    entity_partial_tick: f32,
    chicken_variants: Option<&RegistryContentState>,
    cow_variants: Option<&RegistryContentState>,
    pig_variants: Option<&RegistryContentState>,
    frog_variants: Option<&RegistryContentState>,
    cat_variants: Option<&RegistryContentState>,
    wolf_variants: Option<&RegistryContentState>,
    villager_types: Option<&RegistryContentState>,
    villager_professions: Option<&RegistryContentState>,
) -> Option<EntityModelInstance> {
    let mut kind = entity_model_kind_with_time_and_registries(
        source.entity_type_id,
        &source.data_values,
        source.age_ticks as f32 + entity_partial_tick,
        game_time,
        chicken_variants,
        cow_variants,
        pig_variants,
        frog_variants,
        cat_variants,
        wolf_variants,
    );
    // Vanilla `Armadillo.shouldHideInShell()` = `getState().shouldHideInShell(inStateTicks)`: the
    // shell-ball swap is gated on the client `inStateTicks`, which `entity_model_kind` (data-only)
    // cannot see, so it falls back to the steady SCARED hide. Override it with the world-projected
    // `isHidingInShell`, which also covers the ROLLING/UNROLLING transition windows.
    if let EntityModelKind::Armadillo { rolled_up, .. } = &mut kind {
        *rolled_up = source.armadillo_is_hiding_in_shell;
    }
    apply_player_profile_skin(&mut kind, &source, world, item_runtime);
    let player_cape_texture = player_profile_texture(
        &source,
        world,
        item_runtime,
        EntityDynamicPlayerTextureKind::Cape,
    );
    let player_elytra_texture = player_profile_texture(
        &source,
        world,
        item_runtime,
        EntityDynamicPlayerTextureKind::Elytra,
    );
    let (chest_wings_layer, chest_equipment_has_wings, chest_equipment_has_humanoid) =
        chest_equipment_layers(&source, world, item_runtime);
    // Only skeletons drive the `BOW_AND_ARROW` aim pose; resolve the held item just for them to avoid a
    // per-entity item lookup for every mob.
    let main_hand_holds_bow =
        matches!(
            kind,
            EntityModelKind::Skeleton | EntityModelKind::SkeletonVariant { .. }
        ) && entity_main_hand_holds_bow(world, item_runtime, source.entity_id);
    // Vanilla `HumanoidModel.setupAttackAnimation` switches on the held item's `SWING_ANIMATION` type: a
    // spear swings with the `STAB` `SpearAnimations.thirdPersonAttackHand` pose instead of the default
    // `WHACK`. Only `PlayerModel` consumes the shared attack helper (the skeleton/zombie/illager melee
    // models use their own arm poses), so resolve the spear just for the player kind.
    let main_hand_swing_is_stab = matches!(kind, EntityModelKind::Player { .. })
        && entity_hand_holds_spear(world, item_runtime, source.entity_id, false);
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `SPYGLASS`: while a player is using a spyglass
    // (`isUsingItem` + the using hand holds a spyglass), the holding arm raises it to the eye. Only
    // `PlayerModel` consumes the use-item poses, so resolve the using-hand item just for the player kind.
    let player_using_spyglass = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && entity_hand_holds_spyglass(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `TOOT_HORN`: while a player is tooting a goat
    // horn (`isUsingItem` + the using hand holds a goat horn), the holding arm raises it to the mouth.
    let player_tooting_horn = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && entity_hand_holds_goat_horn(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `BRUSH`: while a player is brushing
    // (`isUsingItem` + the using hand holds a brush), the holding arm lowers to the brushed block.
    let player_brushing = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && entity_hand_holds_brush(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `BLOCK` (`poseBlockingArm`): while a player raises a
    // non-consumable `BLOCKS_ATTACKS` item (the vanilla shield or a datapack/patch-granted blocker), the
    // holding arm tucks the blocking item forward along the head look.
    let player_blocking = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && entity_hand_blocks_attacks(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `THROW_TRIDENT`: while a player charges a trident
    // throw (`isUsingItem` + the using hand holds a trident, whose `TridentItem.getUseAnimation()` is
    // `TRIDENT`), the holding arm raises the trident straight overhead. Same `poseRightArm`/`poseLeftArm`
    // case the drowned reaches via aggression, here driven by the player use-item path.
    let player_throwing_trident = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && entity_hand_holds_trident(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` `BOW_AND_ARROW` use-item arm pose: while a player
    // draws a bow (`isUsingItem` + the using hand holds a bow, `BowItem.getUseAnimation() == BOW`), BOTH arms
    // raise along the head look. The pose is two-handed and `affectsOffhandPose`, so `poseRightArm` sets both
    // arms and the opposite arm's pose is skipped. The renderer mirrors the brace yaw when the using hand is
    // off hand.
    let player_drawing_bow = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && entity_hand_holds_bow(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` `CROSSBOW_CHARGE` use-item arm pose
    // (`AnimationUtils.animateCrossbowCharge`, the same one the pillager/piglin use): while a player draws an
    // UNCHARGED crossbow (`isUsingItem` + the using hand holds a crossbow,
    // `CrossbowItem.getUseAnimation() == CROSSBOW`), the holding arm braces and the opposite arm pulls the
    // string back over the draw ticks
    // (`crossbow_charge_ticks`, the shared `getTicksUsingItem` counter advanced off `isUsingItem` in the
    // world tick loop). A CHARGED crossbow is excluded (`getArmPose` returns `CROSSBOW_HOLD` first).
    let player_charging_crossbow = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && entity_hand_holds_crossbow(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        )
        && !entity_hand_holds_charged_crossbow(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `AvatarRenderer.getArmPose` `CROSSBOW_HOLD` (`AnimationUtils.animateCrossbowHold`, the same one
    // the pillager levels): a player holding a CHARGED main-hand crossbow while not mid-swing (`!swinging &&
    // crossbow && isCharged`, checked before the use-item branch) levels the crossbow along the head look.
    // Gated to the player kind, the main hand, and `!is_swinging` (the swing wins). Applied after the ITEM
    // blocks in the model so it overwrites both arms exactly as vanilla's `poseRightArm` runs last for this
    // case.
    let player_crossbow_hold = matches!(kind, EntityModelKind::Player { .. })
        && !source.is_swinging
        && entity_hand_holds_charged_crossbow(world, item_runtime, source.entity_id, false);
    // Vanilla `HumanoidModel.setupAnim` dispatch (lines 245-257): when a hand uses an `affectsOffhandPose`
    // item (the two-handed draws — `BOW_AND_ARROW` / `THROW_TRIDENT` / `CROSSBOW_CHARGE` / `CROSSBOW_HOLD`),
    // the OPPOSITE arm's `poseArm` is SKIPPED, so the opposite hand gets NO pose. Compute it per using-hand to
    // suppress the opposite hand's `ITEM` fallback (the opposite arm keeps its draw-set or walk pose).
    // `SPYGLASS`/`TOOT_HORN`/`BRUSH`/`BLOCK` are NOT `affectsOffhandPose`, so they leave the opposite `ITEM`
    // intact. (The `affectsOffhandPose` use items bbb poses are exactly the bow / trident / crossbow draws.)
    let main_hand_use_affects_offhand = source.is_using_item
        && !source.use_item_off_hand
        && (entity_hand_holds_bow(world, item_runtime, source.entity_id, false)
            || entity_hand_holds_trident(world, item_runtime, source.entity_id, false)
            || entity_hand_holds_crossbow(world, item_runtime, source.entity_id, false));
    let off_hand_use_affects_offhand = source.is_using_item
        && source.use_item_off_hand
        && (entity_hand_holds_bow(world, item_runtime, source.entity_id, true)
            || entity_hand_holds_trident(world, item_runtime, source.entity_id, true)
            || entity_hand_holds_crossbow(world, item_runtime, source.entity_id, true));
    // Vanilla's off-hand `CROSSBOW_HOLD` (`poseLeftArm`) is skipped when the main hand already has an
    // affecting pose (`BOW_AND_ARROW`, `THROW_TRIDENT`, `CROSSBOW_CHARGE`/`HOLD`, or `SPEAR`). Otherwise a
    // charged off-hand crossbow levels the mirrored hold pose after the main hand's non-affecting pose.
    let player_crossbow_hold_off_hand = matches!(kind, EntityModelKind::Player { .. })
        && !source.is_swinging
        && !main_hand_use_affects_offhand
        && !entity_hand_holds_spear(world, item_runtime, source.entity_id, false)
        && !entity_hand_holds_charged_crossbow(world, item_runtime, source.entity_id, false)
        && entity_hand_holds_charged_crossbow(world, item_runtime, source.entity_id, true);
    // Whether a hand is the using hand holding a SPECIAL-pose item (so it gets its dedicated pose, NOT the
    // `ITEM` fallback). Any OTHER used item (food/potion -> `EAT`/`DRINK`, or a plain tool) falls through to
    // `ITEM`, so a player eating/drinking still shows the lowered `ITEM` arm.
    let main_hand_use_is_special = source.is_using_item
        && !source.use_item_off_hand
        && entity_hand_holds_special_use_item(world, item_runtime, source.entity_id, false);
    let off_hand_use_is_special = source.is_using_item
        && source.use_item_off_hand
        && entity_hand_holds_special_use_item(world, item_runtime, source.entity_id, true);
    // Vanilla `AvatarRenderer.getArmPose` fallback `ITEM` arm pose (`HumanoidModel.poseRightArm` ITEM case):
    // a player holding a generic main-hand item lowers and halves the arm swing — the `ITEM` branch reached
    // after the special-pose checks fail. Fires whether or not using, EXCEPT when this hand is using a special
    // item (`main_hand_use_is_special` -> its own pose) or the OFF hand draws an `affectsOffhandPose` item
    // (`off_hand_use_affects_offhand` -> vanilla skips this arm's `poseArm`). Spears (-> `SPEAR`) and charged
    // crossbows (-> `CROSSBOW_HOLD`) are excluded so their dedicated poses win; a non-charged crossbow or a
    // held (not drawn) bow correctly falls through to `ITEM`. Only `PlayerModel` consumes it.
    let player_main_hand_item_pose = matches!(kind, EntityModelKind::Player { .. })
        && !main_hand_use_is_special
        && !off_hand_use_affects_offhand
        && entity_main_hand_non_empty(world, source.entity_id)
        && !entity_hand_holds_spear(world, item_runtime, source.entity_id, false)
        && !entity_hand_holds_charged_crossbow(world, item_runtime, source.entity_id, false);
    // Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM` arm pose, posed onto the OFF (left)
    // arm by `HumanoidModel.poseLeftArm`: a player holding a plain off-hand item (shield/totem/block/food)
    // lowers and halves that arm. Mirror of the main-hand gate: fires whether or not using, EXCEPT when the
    // OFF hand uses a special item (`off_hand_use_is_special`) or the MAIN hand draws an `affectsOffhandPose`
    // item (`main_hand_use_affects_offhand` -> vanilla skips `poseLeftArm`). Excludes off-hand spears and
    // charged crossbows. The `isTwoHanded`-main-hand override (which would FORCE a non-empty off hand to
    // `ITEM`) stays deferred — it only diverges for an off-hand spear / charged crossbow while the main hand
    // draws a two-handed item.
    let player_off_hand_item_pose = matches!(kind, EntityModelKind::Player { .. })
        && !off_hand_use_is_special
        && !main_hand_use_affects_offhand
        && entity_offhand_non_empty(world, source.entity_id)
        && !entity_hand_holds_spear(world, item_runtime, source.entity_id, true)
        && !entity_hand_holds_charged_crossbow(world, item_runtime, source.entity_id, true);
    // Only the pillager drives the `CROSSBOW_HOLD` pose; resolve the held item just for it.
    let main_hand_holds_crossbow =
        matches!(
            kind,
            EntityModelKind::Illager {
                family: IllagerModelFamily::Pillager
            }
        ) && entity_hand_holds_crossbow(world, item_runtime, source.entity_id, false);
    // Vanilla `IllagerModel.setupAnim` `ATTACKING` branch selects empty-handed `animateZombieArms` versus
    // armed `swingWeaponDown` from the rendered main-hand item state.
    let illager_main_hand_empty = matches!(kind, EntityModelKind::Illager { .. })
        && !entity_main_hand_non_empty(world, source.entity_id);
    // Vanilla `DrownedRenderer.getArmPose`: a drowned in its main hand holding a trident while aggressive
    // (`getMainArm() == arm && isAggressive() && item.is(Items.TRIDENT)`) raises the trident overhead to
    // throw it. `isAggressive` is already projected (the drowned is in the zombie model family); resolve
    // the held item just for the drowned.
    let drowned_throw_trident = matches!(
        kind,
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            ..
        }
    ) && source.is_aggressive
        && entity_hand_holds_trident(world, item_runtime, source.entity_id, false);
    // Vanilla `WitchRenderer.extractRenderState`: `isHoldingItem` is any non-empty main hand and
    // `isHoldingPotion` is exactly `Items.POTION`. The former drives the witch model's nose hold pose; the
    // latter selects `WitchItemLayer`'s nose-attached potion branch.
    let witch_holding_item = matches!(kind, EntityModelKind::Witch)
        && entity_main_hand_non_empty(world, source.entity_id);
    let witch_holding_potion = witch_holding_item
        && entity_main_hand_is_item(world, item_runtime, source.entity_id, "minecraft:potion");
    // Vanilla `CopperGolemModel.setupAnim`: either rendered hand item state selects the held-item arm
    // clamp before `ItemInHandLayer` reads the hand transform. The walk-with-item keyframe stays deferred.
    let copper_golem_holding_item = matches!(kind, EntityModelKind::CopperGolem { .. })
        && (entity_main_hand_non_empty(world, source.entity_id)
            || entity_offhand_non_empty(world, source.entity_id));
    let custom_head_skull = entity_custom_head_skull(world, item_runtime, source.entity_id);
    // Vanilla `Piglin.getArmPose` `ADMIRING_ITEM` (`PiglinAi.isLovedItem(getOffhandItem())`): a regular
    // piglin holding a piglin-loved item in its OFFHAND admires it (head tilts down, the off arm lifts the
    // item). Second-highest priority (below DANCING, above ATTACKING / CROSSBOW), so it suppresses those.
    // Only `Piglin.getArmPose` has this branch — the brute's is ATTACKING/DEFAULT only — so gate to the
    // regular piglin. Resolve the offhand item + the `minecraft:piglin_loved` item tag just for it.
    let piglin_admiring = matches!(
        kind,
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            ..
        }
    ) && !piglin_is_dancing(source.entity_type_id, &source.data_values)
        && entity_offhand_holds_loved_item(world, source.entity_id);
    // Vanilla `Piglin.getArmPose` `CROSSBOW_HOLD`: a regular piglin holding a charged crossbow, not
    // dancing (top priority), not admiring (an offhand loved item, higher priority), and not mid-draw
    // (`CROSSBOW_CHARGE`, whose pull-back pose is deferred). The higher-priority `ATTACKING_WITH_MELEE_WEAPON`
    // needs a tool main-hand item, so a charged-crossbow hand excludes it. Resolve the held item just for it.
    let piglin_crossbow_hold =
        matches!(
            kind,
            EntityModelKind::Piglin {
                family: PiglinModelFamily::Piglin,
                ..
            }
        ) && entity_hand_holds_charged_crossbow(world, item_runtime, source.entity_id, false)
            && !piglin_is_charging_crossbow(source.entity_type_id, &source.data_values)
            && !piglin_is_dancing(source.entity_type_id, &source.data_values)
            && !piglin_admiring;
    // Vanilla `Piglin`/`PiglinBrute.getArmPose` `ATTACKING_WITH_MELEE_WEAPON`: a piglin or piglin brute
    // that is aggressive (`Mob.isAggressive()`) and holds a melee weapon (`isHoldingMeleeWeapon()`, a
    // main-hand item with the `tool` component), not dancing, and (for the regular piglin) not admiring an
    // offhand loved item (both higher priority). The brute has no dance/admire/crossbow poses, so
    // `piglin_admiring` is always false for it. The zombified piglin uses its renderer zombie-arm pose
    // instead of this weapon-raised pose, so it is excluded. Resolve the held item just for these families.
    let piglin_attacking_with_melee = matches!(
        kind,
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin | PiglinModelFamily::PiglinBrute,
            ..
        }
    ) && source.is_aggressive
        && entity_main_hand_holds_melee_weapon(world, source.entity_id)
        && !piglin_is_dancing(source.entity_type_id, &source.data_values)
        && !piglin_admiring;
    // Vanilla `Piglin.getArmPose` `CROSSBOW_CHARGE`: a regular piglin drawing its crossbow
    // (`isChargingCrossbow()`, the synced `DATA_IS_CHARGING_CROSSBOW` boolean id 18). Vanilla checks only
    // the flag (no held-item gate), but it ranks below DANCING / ADMIRING / ATTACKING, so suppress it under
    // those (a charging crossbow hand can never hold a melee tool, so the attack gate is also item-exclusive
    // — the explicit `!` is defensive). The pull-back ticks come from the shared `crossbow_charge_ticks`.
    let piglin_crossbow_charge =
        matches!(
            kind,
            EntityModelKind::Piglin {
                family: PiglinModelFamily::Piglin,
                ..
            }
        ) && piglin_is_charging_crossbow(source.entity_type_id, &source.data_values)
            && !piglin_is_dancing(source.entity_type_id, &source.data_values)
            && !piglin_admiring
            && !piglin_attacking_with_melee;
    // Vanilla `Goat.getRammingXHeadRot()`: the world-projected `lowerHeadTick` ram counter scaled by the
    // adult/baby max head pitch. Resolved here because the baby flag lives in the goat kind.
    let goat_ramming_x_head_rot = match kind {
        EntityModelKind::Goat { baby, .. } => {
            goat_ramming_x_head_rot(source.goat_lower_head_tick, baby)
        }
        _ => 0.0,
    };
    let head_eat = sheep_head_eat_pose(
        source.entity_type_id,
        source.sheep_eat_animation_tick,
        entity_partial_tick,
    );
    let light_coords =
        entity_light_coords(source.entity_type_id, &source.data_values, source.light);
    // Vanilla `Camel.setupAnimationStates()`: the sit/sit-pose/stand-up timing projected purely
    // from the synced `LAST_POSE_CHANGE_TICK` (id 20) and the world game time.
    let camel_sit = camel_sit_state(source.entity_type_id, &source.data_values, game_time);
    // Vanilla LivingEntityRenderer.extractRenderState:
    //   state.yRot = Mth.wrapDegrees(headRot - bodyRot)  (net head-look yaw)
    //   state.xRot = entity.getXRot(partialTicks)         (head-look pitch)
    // The net head yaw is taken against the unshaken body yaw; the setupRotations
    // body shake (freezing or a per-renderer conversion) is then folded into the
    // projected body_rot so the whole model jitters while the head turn relative to
    // the body is unchanged.
    // Vanilla LivingEntityRenderer.extractRenderState negates the net head yaw and
    // pitch while the entity is upside down (Dinnerbone/Grumm).
    let head_sign = if source.is_upside_down { -1.0 } else { 1.0 };
    // Vanilla `Squid.aiStep` refines `yBodyRot` from movement independently of
    // the synced transform yaw, and `LivingEntityRenderer.extractRenderState`
    // uses that value as `bodyRot`. Other entities still use the canonical
    // synced yaw projected by WorldStore.
    let projected_body_yaw = if is_squid_entity_type(source.entity_type_id) {
        source.squid_y_body_rot
    } else {
        source.y_rot
    };
    let net_head_yaw = wrap_degrees(source.y_head_rot - projected_body_yaw) * head_sign;
    let head_pitch = source.x_rot * head_sign;
    let is_shaking = entity_shaking(
        source.entity_type_id,
        &source.data_values,
        source.is_fully_frozen,
    );
    let body_rot = projected_body_yaw + entity_body_shake_degrees(source.age_ticks, is_shaking);
    // Vanilla LivingEntityRenderer.setupRotations riptide branch reads the lerped
    // `state.ageInTicks` (= tickCount + partialTick) only while `isAutoSpinAttack`.
    let auto_spin_age_ticks = source
        .is_auto_spin_attack
        .then_some(source.age_ticks as f32 + entity_partial_tick);
    // Vanilla `ArmedEntityRenderState.extractArmedEntityRenderState` fills Vex right/left item states
    // from `getItemHeldByArm(RIGHT/LEFT)`. bbb does not yet project non-player main-arm handedness here;
    // Vexes use the default RIGHT main arm path, so canonical main/off hand feed RIGHT/LEFT respectively.
    let is_vex = matches!(kind, EntityModelKind::Vex { .. });
    let vex_right_hand_item_non_empty =
        is_vex && entity_main_hand_non_empty(world, source.entity_id);
    let vex_left_hand_item_non_empty = is_vex && entity_offhand_non_empty(world, source.entity_id);
    // Vanilla setupRotations lifts the upside-down model by its bounding box height.
    let upside_down_height = source.is_upside_down.then_some(source.bounding_box_height);
    let drowned_bounding_box_height = matches!(
        &kind,
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            ..
        }
    )
    .then_some(source.bounding_box_height)
    .unwrap_or_default();
    // Vanilla setupRotations sleeping branch: the bed yaw (or the body yaw when not
    // in a bed) plus the bed head-offset translate.
    let sleeping = source.is_sleeping.then_some(SleepingPose {
        yaw_angle: source.sleeping_bed_yaw.unwrap_or(body_rot),
        bed_offset: source.sleeping_bed_offset,
    });
    let outline_color = if source.outline_color != 0 {
        source.outline_color
    } else if source.appears_glowing {
        ENTITY_DEFAULT_OUTLINE_COLOR
    } else {
        0
    };
    Some(
        EntityModelInstance::new(
            source.entity_id,
            kind,
            [
                source.position.x as f32,
                source.position.y as f32,
                source.position.z as f32,
            ],
            body_rot,
        )
        .with_head_eat(head_eat)
        .with_head_look(net_head_yaw, head_pitch)
        .with_arrow_shake(source.arrow_shake)
        .with_trident_foil(thrown_trident_foil(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_invisible(entity_invisible(&source.data_values))
        .with_invisible_to_player(source.invisible_to_player)
        .with_outline_color(outline_color)
        .with_polar_bear_stand_scale(source.polar_bear_stand_scale)
        .with_light_coords(light_coords)
        .with_has_red_overlay(source.has_red_overlay)
        .with_death_time(source.death_time)
        .with_auto_spin_age_ticks(auto_spin_age_ticks)
        .with_upside_down_height(upside_down_height)
        .with_bounding_box_height(drowned_bounding_box_height)
        .with_sleeping(sleeping)
        .with_scale(source.scale)
        .with_swim_amount(source.swim_amount)
        .with_in_water(source.in_water)
        .with_on_ground(source.on_ground)
        .with_is_moving(source.is_moving)
        .with_walk_animation(source.walk_animation_position, source.walk_animation_speed)
        .with_worn_head_animation_pos(source.worn_head_animation_position)
        .with_is_riding(source.is_passenger)
        .with_attack_anim(source.attack_anim)
        .with_attack_arm_off_hand(source.attack_arm_off_hand)
        .with_age_in_ticks(source.age_ticks as f32 + entity_partial_tick)
        .with_boat_rowing_time_left(source.boat_rowing_time_left)
        .with_boat_rowing_time_right(source.boat_rowing_time_right)
        .with_boat_hurt_time(source.boat_hurt_time)
        .with_boat_hurt_dir(source.boat_hurt_dir)
        .with_boat_damage_time(source.boat_damage_time)
        .with_boat_bubble_angle(source.boat_bubble_angle)
        .with_boat_underwater(source.boat_underwater)
        .with_is_aggressive(source.is_aggressive)
        .with_main_hand_holds_bow(main_hand_holds_bow)
        .with_main_hand_swing_is_stab(main_hand_swing_is_stab)
        .with_player_using_spyglass(player_using_spyglass)
        .with_player_tooting_horn(player_tooting_horn)
        .with_player_brushing(player_brushing)
        .with_player_blocking(player_blocking)
        .with_player_throwing_trident(player_throwing_trident)
        .with_player_drawing_bow(player_drawing_bow)
        .with_player_charging_crossbow(player_charging_crossbow)
        .with_player_crossbow_hold(player_crossbow_hold)
        .with_player_crossbow_hold_off_hand(player_crossbow_hold_off_hand)
        .with_player_main_hand_item_pose(player_main_hand_item_pose)
        .with_player_off_hand_item_pose(player_off_hand_item_pose)
        .with_player_cape_texture(player_cape_texture)
        .with_player_elytra_texture(player_elytra_texture)
        .with_show_extra_ears(source.show_extra_ears)
        .with_chest_wings_layer(chest_wings_layer)
        .with_chest_equipment_has_wings(chest_equipment_has_wings)
        .with_chest_equipment_has_humanoid(chest_equipment_has_humanoid)
        .with_player_cape_flap(source.player_cape_flap)
        .with_player_cape_lean(source.player_cape_lean)
        .with_player_cape_lean2(source.player_cape_lean2)
        .with_player_left_shoulder_parrot(
            source
                .player_left_shoulder_parrot
                .map(ParrotModelVariant::from_id),
        )
        .with_player_right_shoulder_parrot(
            source
                .player_right_shoulder_parrot
                .map(ParrotModelVariant::from_id),
        )
        .with_use_item_off_hand(source.use_item_off_hand)
        .with_main_hand_holds_crossbow(main_hand_holds_crossbow)
        .with_illager_main_hand_empty(illager_main_hand_empty)
        .with_drowned_throw_trident(drowned_throw_trident)
        .with_is_charging_crossbow(pillager_is_charging_crossbow(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_crossbow_charge_ticks(source.crossbow_charge_ticks)
        .with_enderman_carrying(source.enderman_carrying)
        .with_enderman_creepy(source.enderman_creepy)
        .with_bat_resting(source.bat_resting)
        .with_bee_has_stinger(source.bee_has_stinger)
        .with_bee_roll_amount(source.bee_roll_amount)
        .with_frog_croak_seconds(source.frog_croak_seconds)
        .with_frog_tongue_seconds(source.frog_tongue_seconds)
        .with_frog_jump_seconds(source.frog_jump_seconds)
        .with_frog_swim_idle_seconds(source.frog_swim_idle_seconds)
        .with_sniffer_animation_id(source.sniffer_animation_id)
        .with_sniffer_animation_seconds(source.sniffer_animation_seconds)
        .with_sniffer_is_searching(source.sniffer_is_searching)
        .with_armadillo_is_hiding_in_shell(source.armadillo_is_hiding_in_shell)
        .with_armadillo_roll_up_seconds(source.armadillo_roll_up_seconds)
        .with_armadillo_roll_out_seconds(source.armadillo_roll_out_seconds)
        .with_armadillo_peek_seconds(source.armadillo_peek_seconds)
        .with_fox_head_roll_angle(source.fox_head_roll_angle)
        .with_fox_crouch_amount(source.fox_crouch_amount)
        .with_fox_is_crouching(source.fox_is_crouching)
        .with_fox_is_sleeping(source.fox_is_sleeping)
        .with_fox_is_sitting(source.fox_is_sitting)
        .with_fox_is_pouncing(source.fox_is_pouncing)
        .with_fox_is_faceplanted(source.fox_is_faceplanted)
        .with_feline_is_crouching(source.feline_is_crouching)
        .with_feline_is_sprinting(source.feline_is_sprinting)
        .with_witch_holding_item(witch_holding_item)
        .with_witch_holding_potion(witch_holding_potion)
        .with_copper_golem_holding_item(copper_golem_holding_item)
        .with_custom_head_skull(custom_head_skull)
        .with_bee_angry(bee_is_angry(
            source.entity_type_id,
            &source.data_values,
            game_time,
        ))
        .with_camel_sit_seconds(camel_sit.sit_seconds)
        .with_camel_sit_pose_seconds(camel_sit.sit_pose_seconds)
        .with_camel_standup_seconds(camel_sit.standup_seconds)
        .with_camel_dash_seconds(source.camel_dash_seconds)
        .with_camel_idle_seconds(source.camel_idle_seconds)
        .with_camel_jump_cooldown(source.camel_jump_cooldown)
        .with_vex_charging(source.vex_charging)
        .with_vex_right_hand_item_non_empty(vex_right_hand_item_non_empty)
        .with_vex_left_hand_item_non_empty(vex_left_hand_item_non_empty)
        .with_wither_invulnerable_ticks(source.wither_invulnerable_ticks)
        .with_wither_x_head_rots(source.wither_x_head_rots)
        .with_wither_y_head_rots(source.wither_y_head_rots)
        .with_wither_powered(wither_powered(source.entity_type_id, &source.data_values))
        .with_head_armor(armor_material(source.head_armor))
        .with_chest_armor(armor_material(source.chest_armor))
        .with_legs_armor(armor_material(source.legs_armor))
        .with_feet_armor(armor_material(source.feet_armor))
        .with_head_armor_dye(armor_dye(source.head_armor_dye))
        .with_chest_armor_dye(armor_dye(source.chest_armor_dye))
        .with_legs_armor_dye(armor_dye(source.legs_armor_dye))
        .with_feet_armor_dye(armor_dye(source.feet_armor_dye))
        .with_head_armor_foil(source.head_armor_foil)
        .with_chest_armor_foil(source.chest_armor_foil)
        .with_legs_armor_foil(source.legs_armor_foil)
        .with_feet_armor_foil(source.feet_armor_foil)
        .with_pig_saddle(source.pig_saddle)
        .with_equine_saddle(source.equine_saddle)
        .with_equine_saddle_ridden(source.equine_saddle_ridden)
        .with_equine_animate_tail(source.equine_animate_tail)
        .with_equine_eat_animation(source.equine_eat_animation)
        .with_equine_stand_animation(source.equine_stand_animation)
        .with_equine_feeding_animation(source.equine_feeding_animation)
        .with_equine_body_armor(armor_material(source.equine_body_armor))
        .with_equine_body_armor_dye(armor_dye(source.equine_body_armor_dye))
        .with_wolf_body_armor(armor_material(source.wolf_body_armor))
        .with_wolf_body_armor_dye(armor_dye(source.wolf_body_armor_dye))
        .with_wolf_body_armor_crackiness(wolf_armor_crackiness(source.wolf_body_armor_crackiness))
        .with_wolf_body_armor_foil(source.wolf_body_armor_foil)
        .with_strider_ridden(source.strider_ridden)
        .with_strider_saddle(source.strider_saddle)
        .with_camel_saddle(source.camel_saddle)
        .with_camel_saddle_ridden(source.camel_saddle_ridden)
        .with_nautilus_saddle(source.nautilus_saddle)
        .with_nautilus_body_armor(armor_material(source.nautilus_body_armor))
        .with_llama_body_decor(llama_body_decor_color(source.llama_body_decor))
        .with_guardian_beam(guardian_beam(source.guardian_beam))
        .with_end_crystal_beam(end_crystal_beam(source.end_crystal_beam))
        .with_ender_dragon_beam(ender_dragon_beam(source.ender_dragon_beam))
        .with_is_crouching(source.is_crouching)
        .with_elytra_rot_x(source.elytra_rot_x)
        .with_elytra_rot_y(source.elytra_rot_y)
        .with_elytra_rot_z(source.elytra_rot_z)
        .with_wolf_tail_angle(wolf_tail_angle(
            source.entity_type_id,
            &source.data_values,
            game_time,
        ))
        .with_wolf_sitting(wolf_sitting(source.entity_type_id, &source.data_values))
        .with_wolf_wet_shade(source.wolf_wet_shade)
        .with_wolf_shake_anim(source.wolf_shake_anim)
        .with_wolf_head_roll_angle(source.wolf_head_roll_angle)
        .with_parrot_sitting(parrot_sitting(source.entity_type_id, &source.data_values))
        .with_parrot_party(source.parrot_party)
        .with_illager_spellcasting(illager_spellcasting(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_illager_celebrating(illager_celebrating(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_piglin_dancing(piglin_is_dancing(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_piglin_crossbow_hold(piglin_crossbow_hold)
        .with_piglin_crossbow_charge(piglin_crossbow_charge)
        .with_piglin_attacking_with_melee(piglin_attacking_with_melee)
        .with_piglin_admiring(piglin_admiring)
        .with_panda_unhappy(panda_is_unhappy(source.entity_type_id, &source.data_values))
        .with_panda_sneezing(panda_is_sneezing(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_panda_sneeze_time(panda_sneeze_time(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_panda_eating(panda_is_eating(source.entity_type_id, &source.data_values))
        .with_panda_scared(panda_is_scared(
            source.entity_type_id,
            &source.data_values,
            world,
        ))
        .with_panda_sitting(panda_is_sitting(source.entity_type_id, &source.data_values))
        .with_panda_sit_amount(source.panda_sit_amount)
        .with_panda_lie_on_back_amount(source.panda_lie_on_back_amount)
        .with_panda_roll_amount(source.panda_roll_amount)
        .with_panda_roll_time(source.panda_roll_time)
        .with_goat_ramming_x_head_rot(goat_ramming_x_head_rot)
        .with_iron_golem_attack_ticks_remaining(source.iron_golem_attack_ticks_remaining)
        .with_iron_golem_offer_flower_tick(source.iron_golem_offer_flower_tick)
        .with_snow_golem_pumpkin(source.snow_golem_pumpkin)
        .with_ravager_stunned_ticks_remaining(source.ravager_stunned_ticks_remaining)
        .with_ravager_attack_ticks_remaining(source.ravager_attack_ticks_remaining)
        .with_ravager_roar_animation(source.ravager_roar_animation)
        .with_hoglin_attack_animation_tick(source.hoglin_attack_animation_tick)
        .with_turtle_has_egg(turtle_has_egg(source.entity_type_id, &source.data_values))
        .with_turtle_laying_egg(turtle_laying_egg(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_end_crystal_shows_bottom(end_crystal_shows_bottom(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_creeper_swelling(source.creeper_swelling)
        .with_creeper_powered(creeper_powered(source.entity_type_id, &source.data_values))
        .with_villager_model_data(villager_model_data(
            source.entity_type_id,
            &source.data_values,
            villager_types,
            villager_professions,
        ))
        .with_villager_unhappy(source.villager_unhappy)
        .with_shulker_peek(source.shulker_peek)
        .with_shulker_attach_face(entity_attachment_face(source.shulker_attach_face))
        .with_tendril_animation(source.tendril_animation)
        .with_heart_animation(source.heart_animation)
        .with_warden_roar_seconds(source.warden_roar_seconds)
        .with_warden_sniff_seconds(source.warden_sniff_seconds)
        .with_warden_attack_seconds(source.warden_attack_seconds)
        .with_warden_sonic_boom_seconds(source.warden_sonic_boom_seconds)
        .with_warden_emerge_seconds(source.warden_emerge_seconds)
        .with_warden_dig_seconds(source.warden_dig_seconds)
        .with_rabbit_hop_seconds(source.rabbit_hop_seconds)
        .with_creaking_can_move(source.creaking_can_move)
        .with_creaking_attack_seconds(source.creaking_attack_seconds)
        .with_creaking_invulnerable_seconds(source.creaking_invulnerable_seconds)
        .with_creaking_death_seconds(source.creaking_death_seconds)
        .with_squid_tentacle_angle(source.squid_tentacle_angle)
        .with_squid_body_tilt(source.squid_x_body_rot, source.squid_z_body_rot)
        .with_guardian_tail_animation(source.guardian_tail_animation)
        .with_guardian_spikes_animation(source.guardian_spikes_animation)
        .with_breeze_shoot_seconds(source.breeze_shoot_seconds)
        .with_breeze_slide_seconds(source.breeze_slide_seconds)
        .with_breeze_slide_back_seconds(source.breeze_slide_back_seconds)
        .with_breeze_inhale_seconds(source.breeze_inhale_seconds)
        .with_breeze_long_jump_seconds(source.breeze_long_jump_seconds)
        .with_chicken_flap(source.chicken_flap)
        .with_chicken_flap_speed(source.chicken_flap_speed)
        .with_slime_squish(source.slime_squish)
        .with_evoker_fangs_bite_progress(source.evoker_fangs_bite_progress)
        .with_allay_dancing(source.allay_dancing)
        .with_allay_spinning(source.allay_spinning)
        .with_allay_spinning_progress(source.allay_spinning_progress)
        .with_allay_holding_item_progress(source.allay_holding_item_progress)
        .with_axolotl_playing_dead_factor(source.axolotl_playing_dead_factor)
        .with_axolotl_in_water_factor(source.axolotl_in_water_factor)
        .with_axolotl_on_ground_factor(source.axolotl_on_ground_factor)
        .with_axolotl_moving_factor(source.axolotl_moving_factor)
        .with_parrot_flap_angle(source.parrot_flap_angle)
        .with_white_overlay_progress(creeper_white_overlay_progress(source.creeper_swelling)),
    )
}

/// Vanilla `LivingEntityRenderer.setupRotations` body shake, folded into the
/// projected body yaw: `cos(Mth.floor(ageInTicks) * 3.25) * π * 0.4` degrees while
/// the entity `isShaking`, otherwise `0`. `age_ticks` is the integer tick count,
/// so it already equals `Mth.floor(ageInTicks)`.
fn entity_body_shake_degrees(age_ticks: u32, is_shaking: bool) -> f32 {
    if !is_shaking {
        return 0.0;
    }
    (age_ticks as f32 * 3.25).cos() * std::f32::consts::PI * 0.4
}

/// Vanilla `LivingEntityRenderer.isShaking`: the base renderer's `isFullyFrozen`,
/// plus the per-renderer conversion overrides. `AbstractZombieRenderer.isShaking`
/// ORs in `Zombie.isUnderWaterConverting()` (synced `DATA_DROWNED_CONVERSION_ID`,
/// id 18) for the whole zombie family, and `ZombieVillagerRenderer` additionally
/// ORs in `ZombieVillager.isConverting()` (synced `DATA_CONVERTING_ID`, id 19).
/// `StriderRenderer.isShaking` additionally ORs in `Strider.isSuffocating()`
/// (synced `DATA_SUFFOCATING`, id 19), the same flag that selects the cold
/// texture.
/// The hoglin/piglin zombification shake is environment-attribute-derived (not a
/// synced flag) and the base-`Skeleton` freeze-conversion shake is server-side
/// `conversionTime`, so both remain deferred.
fn entity_shaking(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    is_fully_frozen: bool,
) -> bool {
    if is_fully_frozen {
        return true;
    }
    match entity_type_id {
        VANILLA_ENTITY_TYPE_ZOMBIE_ID
        | VANILLA_ENTITY_TYPE_HUSK_ID
        | VANILLA_ENTITY_TYPE_DROWNED_ID => {
            entity_data_bool(data_values, ZOMBIE_DROWNED_CONVERSION_DATA_ID, false)
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => {
            entity_data_bool(data_values, ZOMBIE_DROWNED_CONVERSION_DATA_ID, false)
                || entity_data_bool(data_values, ZOMBIE_VILLAGER_CONVERTING_DATA_ID, false)
        }
        VANILLA_ENTITY_TYPE_STRIDER_ID => {
            entity_data_bool(data_values, STRIDER_SUFFOCATING_DATA_ID, false)
        }
        _ => false,
    }
}

/// Vanilla `Mth.wrapDegrees`: wraps an angle in degrees to `-180.0..=180.0`.
fn wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

fn is_squid_entity_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_SQUID_ID | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
    )
}

/// Vanilla `CreeperRenderer.getWhiteOverlayProgress`: with `step` =
/// `Creeper.getSwelling`, returns `0.0` while `(int)(step * 10) % 2 == 0` and
/// `clamp(step, 0.5, 1.0)` otherwise, so the creeper strobes white as the fuse
/// nears detonation.
fn creeper_white_overlay_progress(swelling: f32) -> f32 {
    if (swelling * 10.0) as i32 % 2 == 0 {
        0.0
    } else {
        swelling.clamp(0.5, 1.0)
    }
}

/// Packs the entity's sampled block+sky light into vanilla
/// `EntityRenderState.lightCoords` (`LightCoordsUtil.pack(block, sky)`). Mirrors
/// `EntityRenderer.getBlockLightLevel`, which forces block light to `15` while
/// the entity is on fire (shared-flags bit `0x01`); sky light is unchanged.
/// Vanilla `GlowSquid.DATA_DARK_TICKS_REMAINING` synced INT (the first own accessor on the
/// `Squid`/`AgeableWaterCreature` chain, so index `18`: Entity 0-7, LivingEntity 8-14, Mob 15,
/// AgeableMob baby 16 + age-locked 17). Counts down from `100` after a hurt; `0` while undamaged.
const GLOW_SQUID_DARK_TICKS_DATA_ID: u8 = 18;

/// Vanilla `Mth.clampedLerp(factor, min, max)`: `min` for `factor < 0`, `max` for `factor > 1`, else the
/// linear interpolation `min + factor·(max − min)`.
fn clamped_lerp(factor: f32, min: f32, max: f32) -> f32 {
    if factor < 0.0 {
        min
    } else if factor > 1.0 {
        max
    } else {
        min + factor * (max - min)
    }
}

fn entity_light_coords(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    light: bbb_world::TerrainLight,
) -> u32 {
    let on_fire = (entity_data_byte(data_values, ENTITY_SHARED_FLAGS_DATA_ID, 0)
        & ENTITY_SHARED_FLAG_ON_FIRE)
        != 0;
    let mut block = if on_fire {
        15
    } else {
        u32::from(light.block.min(15))
    };
    // Vanilla full-bright renderers (`getBlockLightLevel` returns `15` unconditionally): these glow with
    // their own internal fire / energy regardless of the surrounding block light. The complete 26.1 set is
    // `BlazeRenderer`, `MagmaCubeRenderer`, `WitherBossRenderer`, `WitherSkullRenderer`,
    // `DragonFireballRenderer`, `ShulkerBulletRenderer`, `AllayRenderer`, and `VexRenderer`.
    if matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_BLAZE_ID
            | VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
            | VANILLA_ENTITY_TYPE_WITHER_ID
            | VANILLA_ENTITY_TYPE_WITHER_SKULL_ID
            | VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID
            | VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID
            | VANILLA_ENTITY_TYPE_ALLAY_ID
            | VANILLA_ENTITY_TYPE_VEX_ID
    ) {
        block = 15;
    }
    // Vanilla `ItemFrameRenderer.getBlockLightLevel`: glow item frames keep their surrounding light, but
    // raise the block component to at least `GLOW_FRAME_BRIGHTNESS = 5`.
    if entity_type_id == VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID {
        block = block.max(5);
    }
    // Vanilla `GlowSquidRenderer.getBlockLightLevel`: a bioluminescent boost
    // `max(blockLight, (int)clampedLerp(1 − darkTicks/10, 0, 15))`. Undamaged (`darkTicks == 0`) it is fully
    // bright (`15`); a hurt drops `darkTicks` to `100` (dark), and it ramps back to full over the final 10
    // ticks. The vanilla `glow == 15 ? 15 : max(glow, super)` ternary is just `max(super, glow)` (super ≤ 15).
    if entity_type_id == VANILLA_ENTITY_TYPE_GLOW_SQUID_ID {
        let dark_ticks = entity_data_int(data_values, GLOW_SQUID_DARK_TICKS_DATA_ID, 0);
        let glow = clamped_lerp(1.0 - dark_ticks as f32 / 10.0, 0.0, 15.0) as u32;
        block = block.max(glow);
    }
    let sky = u32::from(light.sky.min(15));
    block << 4 | sky << 20
}

/// Projects the canonical sheep `eatAnimationTick` into the renderer head-eat
/// pose. Vanilla `SheepRenderer.extractRenderState` calls
/// `Sheep.getHeadEatPositionScale`/`getHeadEatAngleScale` with the partial tick;
/// every non-sheep entity resolves to [`SheepHeadEatPose::NONE`].
fn sheep_head_eat_pose(
    entity_type_id: i32,
    sheep_eat_animation_tick: i32,
    partial_tick: f32,
) -> SheepHeadEatPose {
    if entity_type_id == VANILLA_ENTITY_TYPE_SHEEP_ID {
        SheepHeadEatPose::from_eat_tick(sheep_eat_animation_tick, partial_tick)
    } else {
        SheepHeadEatPose::NONE
    }
}

fn entity_model_kind(
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

fn entity_model_kind_with_registries(
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

fn entity_model_kind_with_time_and_registries(
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

fn villager_model_data(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    villager_types: Option<&RegistryContentState>,
    villager_professions: Option<&RegistryContentState>,
) -> VillagerModelData {
    let data_id = match entity_type_id {
        VANILLA_ENTITY_TYPE_VILLAGER_ID => VILLAGER_DATA_DATA_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => ZOMBIE_VILLAGER_DATA_DATA_ID,
        _ => return VillagerModelData::DEFAULT,
    };
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::VillagerData {
                villager_type,
                profession,
                level,
            } => Some((*villager_type, *profession, *level)),
            _ => None,
        })
        .map(|(villager_type, profession, level)| {
            VillagerModelData::new(
                resolve_villager_type(villager_type, villager_types),
                resolve_villager_profession(profession, villager_professions),
                level,
            )
        })
        .unwrap_or(VillagerModelData::DEFAULT)
}

fn resolve_villager_type(
    registry_id: i32,
    registry: Option<&RegistryContentState>,
) -> VillagerModelType {
    if let Some(registry) = registry {
        villager_type_from_registry_id(registry, registry_id).unwrap_or(VillagerModelType::Plains)
    } else {
        villager_type_from_vanilla_registry_id(registry_id)
    }
}

fn villager_type_from_registry_id(
    registry: &RegistryContentState,
    registry_id: i32,
) -> Option<VillagerModelType> {
    if registry_id < 0 {
        return None;
    }
    registry
        .entries
        .get(registry_id as usize)
        .and_then(|entry| villager_type_from_entry_id(entry.id.as_str()))
}

fn villager_type_from_entry_id(id: &str) -> Option<VillagerModelType> {
    match id {
        "minecraft:desert" => Some(VillagerModelType::Desert),
        "minecraft:jungle" => Some(VillagerModelType::Jungle),
        "minecraft:plains" => Some(VillagerModelType::Plains),
        "minecraft:savanna" => Some(VillagerModelType::Savanna),
        "minecraft:snow" => Some(VillagerModelType::Snow),
        "minecraft:swamp" => Some(VillagerModelType::Swamp),
        "minecraft:taiga" => Some(VillagerModelType::Taiga),
        _ => None,
    }
}

fn villager_type_from_vanilla_registry_id(registry_id: i32) -> VillagerModelType {
    match registry_id {
        0 => VillagerModelType::Desert,
        1 => VillagerModelType::Jungle,
        2 => VillagerModelType::Plains,
        3 => VillagerModelType::Savanna,
        4 => VillagerModelType::Snow,
        5 => VillagerModelType::Swamp,
        6 => VillagerModelType::Taiga,
        _ => VillagerModelType::Plains,
    }
}

fn resolve_villager_profession(
    registry_id: i32,
    registry: Option<&RegistryContentState>,
) -> VillagerModelProfession {
    if let Some(registry) = registry {
        villager_profession_from_registry_id(registry, registry_id)
            .unwrap_or(VillagerModelProfession::None)
    } else {
        villager_profession_from_vanilla_registry_id(registry_id)
    }
}

fn villager_profession_from_registry_id(
    registry: &RegistryContentState,
    registry_id: i32,
) -> Option<VillagerModelProfession> {
    if registry_id < 0 {
        return None;
    }
    registry
        .entries
        .get(registry_id as usize)
        .and_then(|entry| villager_profession_from_entry_id(entry.id.as_str()))
}

fn villager_profession_from_entry_id(id: &str) -> Option<VillagerModelProfession> {
    match id {
        "minecraft:none" => Some(VillagerModelProfession::None),
        "minecraft:armorer" => Some(VillagerModelProfession::Armorer),
        "minecraft:butcher" => Some(VillagerModelProfession::Butcher),
        "minecraft:cartographer" => Some(VillagerModelProfession::Cartographer),
        "minecraft:cleric" => Some(VillagerModelProfession::Cleric),
        "minecraft:farmer" => Some(VillagerModelProfession::Farmer),
        "minecraft:fisherman" => Some(VillagerModelProfession::Fisherman),
        "minecraft:fletcher" => Some(VillagerModelProfession::Fletcher),
        "minecraft:leatherworker" => Some(VillagerModelProfession::Leatherworker),
        "minecraft:librarian" => Some(VillagerModelProfession::Librarian),
        "minecraft:mason" => Some(VillagerModelProfession::Mason),
        "minecraft:nitwit" => Some(VillagerModelProfession::Nitwit),
        "minecraft:shepherd" => Some(VillagerModelProfession::Shepherd),
        "minecraft:toolsmith" => Some(VillagerModelProfession::Toolsmith),
        "minecraft:weaponsmith" => Some(VillagerModelProfession::Weaponsmith),
        _ => None,
    }
}

fn villager_profession_from_vanilla_registry_id(registry_id: i32) -> VillagerModelProfession {
    match registry_id {
        0 => VillagerModelProfession::None,
        1 => VillagerModelProfession::Armorer,
        2 => VillagerModelProfession::Butcher,
        3 => VillagerModelProfession::Cartographer,
        4 => VillagerModelProfession::Cleric,
        5 => VillagerModelProfession::Farmer,
        6 => VillagerModelProfession::Fisherman,
        7 => VillagerModelProfession::Fletcher,
        8 => VillagerModelProfession::Leatherworker,
        9 => VillagerModelProfession::Librarian,
        10 => VillagerModelProfession::Mason,
        11 => VillagerModelProfession::Nitwit,
        12 => VillagerModelProfession::Shepherd,
        13 => VillagerModelProfession::Toolsmith,
        14 => VillagerModelProfession::Weaponsmith,
        _ => VillagerModelProfession::None,
    }
}

/// Vanilla `RabbitRenderer` picks `AdultRabbitModel` for an adult and `BabyRabbitModel` for a baby; both
/// render through the dedicated [`EntityModelKind::Rabbit`] (`baby` selecting the body layout),
/// textured by the `Rabbit.Variant` colour (`DATA_TYPE_ID`, 18) with the `Toast` custom-name override
/// (`RabbitRenderer.checkMagicName(entity, "Toast")`).
fn rabbit_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    EntityModelKind::Rabbit {
        baby: ageable_baby(values),
        variant: RabbitModelVariant::from_id(entity_data_int(values, RABBIT_TYPE_DATA_ID, 0)),
        toast: entity_data_optional_component(values, ENTITY_CUSTOM_NAME_DATA_ID)
            .is_some_and(|name| name == "Toast"),
    }
}

/// Vanilla `Shulker.getColor()` reads `DATA_COLOR_ID` (18, byte): `0..=15` map to the dye,
/// everything else (the default byte 16) is `null`, which `ShulkerRenderer.getTextureLocation`
/// renders with the uncolored `shulker.png`.
fn shulker_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    let color_id = entity_data_byte(values, SHULKER_COLOR_DATA_ID, 16);
    let color = (0..=15)
        .contains(&color_id)
        .then(|| EntityDyeColor::from_vanilla_id(color_id as i32));
    EntityModelKind::Shulker { color }
}

/// Vanilla `ParrotRenderer.getVariantTexture` selects the parrot colour from `Parrot.getVariant()`
/// (the synced `DATA_VARIANT_ID` int, mapped through `Parrot.Variant.byId`). Renders through the
/// dedicated [`EntityModelKind::Parrot`] (`variant` selecting the texture).
fn parrot_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    EntityModelKind::Parrot {
        variant: ParrotModelVariant::from_id(entity_data_int(values, PARROT_VARIANT_DATA_ID, 0)),
    }
}

/// Vanilla `PandaRenderer` (an `AgeableMobRenderer`) picks `PandaModel` for an adult and `BabyPandaModel`
/// for a baby; both render through the dedicated [`EntityModelKind::Panda`] (`baby` selecting the
/// layout, `variant` selecting the gene-driven texture). The displayed gene is
/// `Panda.Gene.getVariantFromGenes(mainGene, hiddenGene)` off the two synced gene bytes (21/22).
fn panda_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    let main_gene = entity_data_byte(values, PANDA_MAIN_GENE_DATA_ID, 0) as i32;
    let hidden_gene = entity_data_byte(values, PANDA_HIDDEN_GENE_DATA_ID, 0) as i32;
    EntityModelKind::Panda {
        baby: ageable_baby(values),
        variant: PandaModelVariant::from_genes(main_gene, hidden_gene),
    }
}

/// Vanilla `CatRenderer` / `OcelotRenderer` (both `AgeableMobRenderer`s) pick `AdultCatModel` /
/// `AdultOcelotModel` (the shared `AdultFelineModel` mesh, the cat scaled 0.8) for an adult and the
/// flatter `BabyFelineModel` mesh (unscaled for both breeds) for a baby. Both render through the
/// dedicated [`EntityModelKind::Feline`] (`cat` selecting the breed/scale, `baby` selecting the
/// layout). For cats the `cat_variant` is decoded from `DATA_VARIANT_ID` (20, `Holder<CatVariant>`);
/// the ocelot has no breed, so it carries the default (ignored when `!cat`). `collar` mirrors vanilla
/// `CatRenderer` (`isTame() ? getCollarColor() : null`): the dyed collar of a tame cat only.
fn feline_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
    cat: bool,
    cat_variants: Option<&RegistryContentState>,
) -> EntityModelKind {
    let tame =
        (entity_data_byte(values, TAMABLE_ANIMAL_FLAGS_DATA_ID, 0) & TAMABLE_ANIMAL_TAME_FLAG) != 0;
    EntityModelKind::Feline {
        cat,
        baby: ageable_baby(values),
        cat_variant: if cat {
            cat_model_variant(values, cat_variants)
        } else {
            CatModelVariant::Black
        },
        collar: (cat && tame).then(|| {
            EntityDyeColor::from_vanilla_id(entity_data_int(
                values,
                CAT_COLLAR_COLOR_DATA_ID,
                CAT_DEFAULT_COLLAR_COLOR_ID,
            ))
        }),
    }
}

fn cat_model_variant(
    values: &[bbb_protocol::packets::EntityDataValue],
    variants: Option<&RegistryContentState>,
) -> CatModelVariant {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == CAT_VARIANT_DATA_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::CatVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registry) = variants {
                cat_variant_from_registry_id(registry, id).unwrap_or(CatModelVariant::Black)
            } else {
                cat_variant_from_vanilla_registry_id(id)
            }
        })
        .unwrap_or(CatModelVariant::Black)
}

fn cat_variant_from_registry_id(
    registry: &RegistryContentState,
    registry_id: i32,
) -> Option<CatModelVariant> {
    if registry_id < 0 {
        return None;
    }
    registry
        .entries
        .get(registry_id as usize)
        .and_then(|entry| cat_variant_from_entry_id(entry.id.as_str()))
}

fn cat_variant_from_entry_id(id: &str) -> Option<CatModelVariant> {
    match id {
        "minecraft:tabby" => Some(CatModelVariant::Tabby),
        "minecraft:black" => Some(CatModelVariant::Black),
        "minecraft:red" => Some(CatModelVariant::Red),
        "minecraft:siamese" => Some(CatModelVariant::Siamese),
        "minecraft:british_shorthair" => Some(CatModelVariant::BritishShorthair),
        "minecraft:calico" => Some(CatModelVariant::Calico),
        "minecraft:persian" => Some(CatModelVariant::Persian),
        "minecraft:ragdoll" => Some(CatModelVariant::Ragdoll),
        "minecraft:white" => Some(CatModelVariant::White),
        "minecraft:jellie" => Some(CatModelVariant::Jellie),
        "minecraft:all_black" => Some(CatModelVariant::AllBlack),
        _ => None,
    }
}

// Vanilla `CatVariants.bootstrap` registers tabby/black/red/siamese/british_shorthair/calico/persian/
// ragdoll/white/jellie/all_black in that order, so the static fallback ids (used before the dynamic
// `cat_variant` registry arrives) are 0..=10. The vanilla default is BLACK.
fn cat_variant_from_vanilla_registry_id(registry_id: i32) -> CatModelVariant {
    match registry_id {
        0 => CatModelVariant::Tabby,
        2 => CatModelVariant::Red,
        3 => CatModelVariant::Siamese,
        4 => CatModelVariant::BritishShorthair,
        5 => CatModelVariant::Calico,
        6 => CatModelVariant::Persian,
        7 => CatModelVariant::Ragdoll,
        8 => CatModelVariant::White,
        9 => CatModelVariant::Jellie,
        10 => CatModelVariant::AllBlack,
        _ => CatModelVariant::Black,
    }
}

/// Vanilla `FoxRenderer` (an `AgeableMobRenderer`) picks `AdultFoxModel` for an adult and `BabyFoxModel`
/// for a baby; both render through the dedicated [`EntityModelKind::Fox`] (`baby` selecting the layout).
fn fox_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    EntityModelKind::Fox {
        baby: ageable_baby(values),
        variant: FoxModelVariant::from_id(entity_data_int(values, FOX_TYPE_DATA_ID, 0)),
    }
}

/// Vanilla `NautilusRenderer` (an `AgeableMobRenderer`) picks `NautilusModel.createBodyMesh` for an
/// adult and the smaller `createBabyBodyLayer` for a baby; both render through the dedicated
/// [`EntityModelKind::Nautilus`] (`baby` selecting the layout). The zombie nautilus reuses the same
/// adult body — see [`zombie_nautilus_model_kind`].
fn nautilus_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    EntityModelKind::Nautilus {
        baby: ageable_baby(values),
    }
}

/// Vanilla `ZombieNautilusRenderer` (a plain `MobRenderer`, so never a baby), selected by the synced
/// `ZombieNautilusVariant` holder: `NORMAL`/`TEMPERATE` renders the living adult `NautilusModel` body
/// over `zombie_nautilus.png`, `WARM` renders the `ZombieNautilusCoralModel` (the same body plus the
/// `corals` subtree) over `zombie_nautilus_coral.png`. The saddle equipment layer is driven by
/// render state; the body armor equipment layer defers.
fn zombie_nautilus_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    EntityModelKind::ZombieNautilus {
        coral: zombie_nautilus_coral(values),
    }
}

/// Vanilla `ZombieNautilus.DATA_VARIANT_ID` (21, a `Holder<ZombieNautilusVariant>`): TamableAnimal adds
/// flags(18) + owner(19) and AbstractNautilus adds DASH(20), so the variant lands at index 21. Only two
/// variants exist — `ZombieNautilusVariants.bootstrap` registers TEMPERATE (id 0, `NORMAL` model) then
/// WARM (id 1, coral model) — so registry id ≥ 1 selects the `WARM` coral model. Resolved by the static
/// bootstrap order; the dynamic-registry reorder path defers (a 2-element vanilla registry is in
/// practice never reordered).
fn zombie_nautilus_coral(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == ZOMBIE_NAUTILUS_VARIANT_DATA_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::ZombieNautilusVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| id >= 1)
        .unwrap_or(false)
}

fn boat(family: BoatModelFamily, chest: bool) -> EntityModelKind {
    EntityModelKind::Boat { family, chest }
}

fn chicken_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
    variants: Option<&RegistryContentState>,
) -> EntityModelKind {
    EntityModelKind::Chicken {
        variant: chicken_model_variant(values, variants),
        baby: ageable_baby(values),
    }
}

fn pig_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
    variants: Option<&RegistryContentState>,
) -> EntityModelKind {
    EntityModelKind::Pig {
        variant: pig_model_variant(values, variants),
        baby: ageable_baby(values),
    }
}

fn cow_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
    variants: Option<&RegistryContentState>,
) -> EntityModelKind {
    EntityModelKind::Cow {
        variant: cow_model_variant(values, variants),
        baby: ageable_baby(values),
    }
}

/// Vanilla `MushroomCowRenderer` (an `AgeableMobRenderer`) renders the mooshroom with the shared
/// `CowModel` / `BabyCowModel` body (`ModelLayers.MOOSHROOM` bakes to the temperate `cowBodyLayer`,
/// `MOOSHROOM_BABY` to `BabyCowModel.createBodyLayer()`), so it maps to the dedicated
/// [`EntityModelKind::Mooshroom`] (`baby` selecting the layout) — the real cow body instead of the
/// generic quadruped stand-in. The red/brown body texture and adult-only mushroom block-model layer are
/// projected from the synced `MushroomCow.DATA_TYPE` (index 20).
fn mooshroom_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    EntityModelKind::Mooshroom {
        baby: ageable_baby(values),
        variant: MooshroomVariant::from_vanilla_id(entity_data_int(
            values,
            MUSHROOM_COW_TYPE_DATA_ID,
            0,
        )),
    }
}

fn sheep_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
    age_ticks: f32,
) -> EntityModelKind {
    let wool = entity_data_byte(values, SHEEP_WOOL_DATA_ID, 0) as u8;
    EntityModelKind::Sheep {
        baby: ageable_baby(values),
        sheared: wool & SHEEP_WOOL_SHEARED_FLAG != 0,
        wool_color: SheepWoolColor::from_vanilla_id(wool & SHEEP_WOOL_COLOR_MASK),
        jeb: entity_data_optional_component(values, ENTITY_CUSTOM_NAME_DATA_ID)
            .is_some_and(|name| name == "jeb_"),
        age_ticks,
    }
}

fn player_model_kind(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let fallback = if entity_type_id == VANILLA_ENTITY_TYPE_MANNEQUIN_ID {
        MANNEQUIN_DEFAULT_MODEL_CUSTOMIZATION
    } else {
        AVATAR_PLAYER_DEFAULT_MODEL_CUSTOMIZATION
    };
    let mask = entity_data_byte(values, AVATAR_MODEL_CUSTOMIZATION_DATA_ID, fallback) as u8;
    EntityModelKind::Player {
        skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        parts: PlayerModelPartVisibility::from_vanilla_mask(mask),
    }
}

fn apply_player_profile_skin(
    kind: &mut EntityModelKind,
    source: &EntityModelSourceState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let EntityModelKind::Player { skin, .. } = kind else {
        return;
    };
    if source.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
        return;
    }
    *skin = EntityPlayerSkin::Default(default_player_skin_for_profile_id(source.uuid.as_u128()));
    let Some(item_runtime) = item_runtime else {
        return;
    };
    let Some(info) = world.player_info_entry(source.uuid) else {
        return;
    };
    *skin = item_runtime.player_skin_for_profile(&player_info_profile_resolvable(&info.profile));
}

fn player_profile_texture(
    source: &EntityModelSourceState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    kind: EntityDynamicPlayerTextureKind,
) -> Option<EntityDynamicPlayerTexture> {
    if source.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
        return None;
    }
    let item_runtime = item_runtime?;
    let info = world.player_info_entry(source.uuid)?;
    item_runtime
        .player_profile_texture_for_profile(&player_info_profile_resolvable(&info.profile), kind)
}

fn chest_equipment_layers(
    source: &EntityModelSourceState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> (Option<EntityEquipmentLayerTexture>, bool, bool) {
    let Some(item_runtime) = item_runtime else {
        return (None, false, false);
    };
    let Some(stack) = world.equipment_item(source.entity_id, EquipmentSlot::Chest) else {
        return (None, false, false);
    };
    if !item_stack_non_empty(&stack) {
        return (None, false, false);
    }
    let Some(item_id) = stack.item_id else {
        return (None, false, false);
    };
    (
        item_runtime.item_equipment_wings_layer(item_id),
        item_runtime.item_equipment_asset_has_wings_layer(item_id),
        item_runtime.item_equipment_asset_has_humanoid_layer(item_id),
    )
}

fn player_info_profile_resolvable(
    profile: &bbb_world::PlayerInfoProfileState,
) -> ResolvableProfileSummary {
    let properties: Vec<_> = profile
        .properties
        .iter()
        .map(|property| GameProfilePropertySummary {
            name: property.name.clone(),
            value: property.value.clone(),
            signature: property.signature.clone(),
        })
        .collect();
    let profile_textures = decode_profile_textures_from_properties(
        properties
            .iter()
            .map(|property| (property.name.as_str(), property.value.as_str())),
    );
    ResolvableProfileSummary {
        kind: ResolvableProfileKindSummary::GameProfile,
        uuid: Some(profile.uuid),
        name: Some(profile.name.clone()),
        properties,
        profile_textures,
        skin_patch: PlayerSkinPatchSummary::default(),
    }
}

fn wolf_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
    game_time: i64,
    wolf_variants: Option<&RegistryContentState>,
) -> EntityModelKind {
    let tame =
        (entity_data_byte(values, TAMABLE_ANIMAL_FLAGS_DATA_ID, 0) & TAMABLE_ANIMAL_TAME_FLAG) != 0;
    EntityModelKind::Wolf {
        baby: ageable_baby(values),
        tame,
        angry: wolf_is_angry(values, game_time),
        collar_color: tame.then(|| {
            EntityDyeColor::from_vanilla_id(entity_data_int(
                values,
                WOLF_COLLAR_COLOR_DATA_ID,
                WOLF_DEFAULT_COLLAR_COLOR_ID,
            ))
        }),
        variant: wolf_model_variant(values, wolf_variants),
    }
}

/// Vanilla `WolfRenderer`: resolve the synced `Wolf.DATA_VARIANT_ID` registry holder to the renderer
/// variant. Mirrors [`cat_model_variant`] — prefer the dynamic `wolf_variant` registry order the
/// server sent, falling back to the static vanilla registration order, and to `Pale` (the vanilla
/// `WolfVariants.DEFAULT`) when no holder is present.
fn wolf_model_variant(
    values: &[bbb_protocol::packets::EntityDataValue],
    variants: Option<&RegistryContentState>,
) -> WolfModelVariant {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == WOLF_VARIANT_DATA_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::WolfVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registry) = variants {
                wolf_variant_from_registry_id(registry, id).unwrap_or(WolfModelVariant::Pale)
            } else {
                wolf_variant_from_vanilla_registry_id(id)
            }
        })
        .unwrap_or(WolfModelVariant::Pale)
}

fn wolf_variant_from_registry_id(
    registry: &RegistryContentState,
    registry_id: i32,
) -> Option<WolfModelVariant> {
    if registry_id < 0 {
        return None;
    }
    registry
        .entries
        .get(registry_id as usize)
        .and_then(|entry| wolf_variant_from_entry_id(entry.id.as_str()))
}

fn wolf_variant_from_entry_id(id: &str) -> Option<WolfModelVariant> {
    match id {
        "minecraft:pale" => Some(WolfModelVariant::Pale),
        "minecraft:spotted" => Some(WolfModelVariant::Spotted),
        "minecraft:snowy" => Some(WolfModelVariant::Snowy),
        "minecraft:black" => Some(WolfModelVariant::Black),
        "minecraft:ashen" => Some(WolfModelVariant::Ashen),
        "minecraft:rusty" => Some(WolfModelVariant::Rusty),
        "minecraft:woods" => Some(WolfModelVariant::Woods),
        "minecraft:chestnut" => Some(WolfModelVariant::Chestnut),
        "minecraft:striped" => Some(WolfModelVariant::Striped),
        _ => None,
    }
}

// Vanilla `WolfVariants.bootstrap` registers pale/spotted/snowy/black/ashen/rusty/woods/chestnut/
// striped in that order, so the static fallback ids (used before the dynamic `wolf_variant` registry
// arrives) are 0..=8. The vanilla default is PALE.
fn wolf_variant_from_vanilla_registry_id(registry_id: i32) -> WolfModelVariant {
    match registry_id {
        1 => WolfModelVariant::Spotted,
        2 => WolfModelVariant::Snowy,
        3 => WolfModelVariant::Black,
        4 => WolfModelVariant::Ashen,
        5 => WolfModelVariant::Rusty,
        6 => WolfModelVariant::Woods,
        7 => WolfModelVariant::Chestnut,
        8 => WolfModelVariant::Striped,
        _ => WolfModelVariant::Pale,
    }
}

fn entity_invisible(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    (entity_data_byte(values, ENTITY_SHARED_FLAGS_DATA_ID, 0) & ENTITY_SHARED_FLAG_INVISIBLE) != 0
}

fn wolf_is_angry(values: &[bbb_protocol::packets::EntityDataValue], game_time: i64) -> bool {
    let end_time = entity_data_long(values, WOLF_ANGER_END_TIME_DATA_ID, -1);
    end_time > 0 && end_time - game_time > 0
}

/// Vanilla `BeeRenderState.isAngry` (`Bee.isAngry()`, the `NeutralMob` anger): the synced
/// `DATA_ANGER_END_TIME` is in the future (`endTime > 0 && endTime - gameTime > 0`). An angry
/// bee skips `BeeModel.bobUpAndDown`. Gated to the bee; every other entity is calm.
fn bee_is_angry(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    game_time: i64,
) -> bool {
    if entity_type_id != VANILLA_ENTITY_TYPE_BEE_ID {
        return false;
    }
    let end_time = entity_data_long(values, BEE_ANGER_END_TIME_DATA_ID, -1);
    end_time > 0 && end_time - game_time > 0
}

/// Vanilla `BeeRenderState.hasNectar` (`Bee.hasNectar()`, the synced `DATA_FLAGS_ID & FLAG_HAS_NECTAR`):
/// bit 8 of the bee flags byte. Drives the `BeeRenderer.getTextureLocation` nectar texture swap.
/// Gated to the bee; every other entity reports no nectar.
fn bee_has_nectar(entity_type_id: i32, values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    if entity_type_id != VANILLA_ENTITY_TYPE_BEE_ID {
        return false;
    }
    entity_data_byte(values, BEE_FLAGS_DATA_ID, 0) & BEE_FLAG_HAS_NECTAR != 0
}

/// The three projected camel sit/stand elapsed-seconds values, each `-1.0` when its
/// `AnimationState` is stopped (so the renderer applies no keyframe).
#[derive(Clone, Copy, Debug, PartialEq)]
struct CamelSitState {
    /// Vanilla `Camel.sitAnimationState` elapsed seconds (`CAMEL_SIT`, 2.0 s).
    sit_seconds: f32,
    /// Vanilla `Camel.sitPoseAnimationState` elapsed seconds (`CAMEL_SIT_POSE`, 1.0 s).
    sit_pose_seconds: f32,
    /// Vanilla `Camel.sitUpAnimationState` elapsed seconds (`CAMEL_STANDUP`, 2.6 s).
    standup_seconds: f32,
}

impl CamelSitState {
    const STOPPED: Self = Self {
        sit_seconds: -1.0,
        sit_pose_seconds: -1.0,
        standup_seconds: -1.0,
    };
}

/// Vanilla `Camel.setupAnimationStates()` (client tick) projected purely from the synced
/// `LAST_POSE_CHANGE_TICK` (id 20, a `Long`) and the world game time — no client-side
/// accumulator is needed because the camel's sit/stand timing is a deterministic function of
/// those two values. Mirrors `Camel.getPoseTime()` (`gameTime - |lastPoseChangeTick|`) and the
/// `isCamelSitting`/`isCamelVisuallySitting`/`isVisuallySittingDown`/`isInPoseTransition`
/// predicates, returning each active animation's `(ageInTicks - startTick)` elapsed as raw
/// seconds (the renderer clamps the non-looping tables to their final frame):
///   - `sit` and `standup` start at the pose-change tick, so their elapsed is `getPoseTime`;
///   - `sitPose` starts when the 40-tick sit-down window ends, so its elapsed is
///     `getPoseTime - 40`.
/// The `dash` (rising-edge driven) and `idle` (client random-timer driven) animations are projected
/// by the world client-animation state. Non-camel entities return [`CamelSitState::STOPPED`].
fn camel_sit_state(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    game_time: i64,
) -> CamelSitState {
    if entity_type_id != VANILLA_ENTITY_TYPE_CAMEL_ID
        && entity_type_id != VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
    {
        return CamelSitState::STOPPED;
    }
    let last_pose_change_tick = entity_data_long(values, CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID, 0);
    // Vanilla `Camel.getPoseTime()` and `isCamelSitting()`.
    let pose_time = game_time - last_pose_change_tick.abs();
    let is_sitting = last_pose_change_tick < 0;
    // Vanilla `Camel.isCamelVisuallySitting()`.
    let is_visually_sitting = (pose_time < 0) != is_sitting;
    // Vanilla `Camel.isVisuallySittingDown()`.
    let is_visually_sitting_down =
        is_sitting && pose_time >= 0 && pose_time < CAMEL_SITDOWN_DURATION_TICKS;
    // Vanilla `Camel.isInPoseTransition()`.
    let transition_length = if is_sitting {
        CAMEL_SITDOWN_DURATION_TICKS
    } else {
        CAMEL_STANDUP_DURATION_TICKS
    };
    let is_in_pose_transition = pose_time < transition_length;

    let ticks_to_seconds = |ticks: i64| ticks as f32 / 20.0;
    if is_visually_sitting {
        // `sitUp`/`dash` are stopped; `sit` plays during the sit-down window, then `sitPose`.
        if is_visually_sitting_down {
            CamelSitState {
                sit_seconds: ticks_to_seconds(pose_time),
                sit_pose_seconds: -1.0,
                standup_seconds: -1.0,
            }
        } else {
            CamelSitState {
                sit_seconds: -1.0,
                // `sitPose` starts when the 40-tick sit-down window ends.
                sit_pose_seconds: ticks_to_seconds(pose_time - CAMEL_SITDOWN_DURATION_TICKS),
                standup_seconds: -1.0,
            }
        }
    } else if is_in_pose_transition && pose_time >= 0 {
        // Not visually sitting: `standup` plays during the stand-up transition.
        CamelSitState {
            sit_seconds: -1.0,
            sit_pose_seconds: -1.0,
            standup_seconds: ticks_to_seconds(pose_time),
        }
    } else {
        CamelSitState::STOPPED
    }
}

/// Vanilla `WolfRenderState.tailAngle = Wolf.getTailAngle()`: the wolf tail `xRot`. An angry
/// wolf raises it to the constant `1.5393804`; a tame wolf droops it with damage,
/// `(0.55 - damageRatio * 0.4) * π` where `damageRatio = (maxHealth - health) / maxHealth`
/// and tamed wolves have the constant `maxHealth = 40` (`Wolf.setTame` sets the base value);
/// an untamed wolf returns the `π/5` default. Non-wolf entities return the `π/5` default,
/// which matches the wolf-tail render-state default and so leaves every other model
/// untouched.
fn wolf_tail_angle(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    game_time: i64,
) -> f32 {
    const WILD_TAIL_ANGLE: f32 = std::f32::consts::PI / 5.0;
    if entity_type_id != VANILLA_ENTITY_TYPE_WOLF_ID {
        return WILD_TAIL_ANGLE;
    }
    if wolf_is_angry(values, game_time) {
        // `Wolf.getTailAngle()` angry branch returns this exact constant.
        return 1.5393804;
    }
    let tame =
        (entity_data_byte(values, TAMABLE_ANIMAL_FLAGS_DATA_ID, 0) & TAMABLE_ANIMAL_TAME_FLAG) != 0;
    if !tame {
        return WILD_TAIL_ANGLE;
    }
    const TAME_MAX_HEALTH: f32 = 40.0;
    let health = entity_data_float(values, LIVING_ENTITY_HEALTH_DATA_ID, TAME_MAX_HEALTH).max(0.0);
    let damage_ratio = (TAME_MAX_HEALTH - health) / TAME_MAX_HEALTH;
    (0.55 - damage_ratio * 0.4) * std::f32::consts::PI
}

/// Vanilla `WolfRenderState.isSitting = Wolf.isInSittingPose()`: the `TamableAnimal`
/// `DATA_FLAGS_ID` sitting bit. Only the wolf model renders a sitting pose, so non-wolf
/// entities (and other tamables that are not yet modelled) report `false`.
fn wolf_sitting(entity_type_id: i32, values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID
        && (entity_data_byte(values, TAMABLE_ANIMAL_FLAGS_DATA_ID, 0) & TAMABLE_ANIMAL_SITTING_FLAG)
            != 0
}

/// Vanilla `ParrotModel.getPose == SITTING` (`Parrot.isInSittingPose()`): the `TamableAnimal`
/// `DATA_FLAGS_ID` sitting bit (id 18, the same byte the wolf uses for `isSitting`). Only the
/// parrot renders the `prepare(SITTING)` perch pose, so non-parrot entities report `false`.
fn parrot_sitting(entity_type_id: i32, values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_PARROT_ID
        && (entity_data_byte(values, TAMABLE_ANIMAL_FLAGS_DATA_ID, 0) & TAMABLE_ANIMAL_SITTING_FLAG)
            != 0
}

/// Vanilla `IllagerRenderState.armPose == SPELLCASTING` (`SpellcasterIllager.isCastingSpell()` =
/// the synced `DATA_SPELL_CASTING_ID` byte > 0, id 17 — the byte holds the spell id, so any
/// non-zero value means casting). Only the spellcaster illagers (evoker, illusioner) define that
/// byte and render the raised-arm spell pose, so the projection is gated to them; the
/// vindicator/pillager are `AbstractIllager` but not spellcasters.
fn illager_spellcasting(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    (entity_type_id == VANILLA_ENTITY_TYPE_EVOKER_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_ILLUSIONER_ID)
        && entity_data_byte(values, SPELLCASTER_ILLAGER_CASTING_DATA_ID, 0) > 0
}

/// Vanilla `Raider.isCelebrating()` (the synced `IS_CELEBRATING` boolean, id 16): drives the evoker and
/// vindicator `CELEBRATING` victory-dance arm pose. Only those two render it — the illusioner overrides
/// `getArmPose` (bow, no celebrate) and the pillager never returns `CELEBRATING` — so the projection is
/// gated to the evoker / vindicator types. The renderer additionally suppresses it while casting /
/// aggressive (which take priority in vanilla `getArmPose`).
fn illager_celebrating(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    (entity_type_id == VANILLA_ENTITY_TYPE_EVOKER_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_VINDICATOR_ID)
        && entity_data_bool(values, RAIDER_IS_CELEBRATING_DATA_ID, false)
}

/// Vanilla `Piglin.isDancing()` (the synced `DATA_IS_DANCING` boolean, id 19): drives the piglin's
/// `DANCING` arm pose (the soul-campfire celebration — swaying ears, raised arms, bobbing head/body).
/// Only `Piglin.getArmPose` returns `DANCING`; the piglin brute and zombified piglin never dance, so the
/// projection is gated to the regular piglin type.
fn piglin_is_dancing(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_PIGLIN_ID
        && entity_data_bool(values, PIGLIN_IS_DANCING_DATA_ID, false)
}

/// Vanilla `PandaRenderState.isUnhappy = Panda.getUnhappyCounter() > 0` (the synced `UNHAPPY_COUNTER`
/// int, id 18): the panda shakes its head and paddles its front legs. Gated to the panda type.
fn panda_is_unhappy(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_PANDA_ID
        && entity_data_int(values, PANDA_UNHAPPY_COUNTER_DATA_ID, 0) > 0
}

/// Vanilla `PandaRenderState.isSneezing = Panda.isSneezing()` (the synced `DATA_ID_FLAGS` byte, id 23,
/// bit `0x02`): the panda dips its head into a sneeze. Gated to the panda type.
fn panda_is_sneezing(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_PANDA_ID
        && (entity_data_byte(values, PANDA_FLAGS_DATA_ID, 0) & PANDA_SNEEZING_FLAG) != 0
}

/// Vanilla `PandaRenderState.sneezeTime = Panda.getSneezeCounter()` (the synced `SNEEZE_COUNTER` int, id
/// 19): the 0..20 ramp that drives the sneeze head dip. `0` for a non-panda or a panda not sneezing.
fn panda_sneeze_time(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> i32 {
    if entity_type_id == VANILLA_ENTITY_TYPE_PANDA_ID {
        entity_data_int(values, PANDA_SNEEZE_COUNTER_DATA_ID, 0)
    } else {
        0
    }
}

/// Vanilla `PandaRenderState.isEating = Panda.isEating()` (the synced `EAT_COUNTER` int, id 20, `> 0`).
/// The held-item layer uses this only to bob the item while the sitting gate is active.
fn panda_is_eating(entity_type_id: i32, values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_PANDA_ID
        && entity_data_int(values, PANDA_EAT_COUNTER_DATA_ID, 0) > 0
}

/// Vanilla `PandaRenderState.isSitting = Panda.isSitting()` (the synced `DATA_ID_FLAGS` byte, id 23, bit
/// `0x08`). `PandaHoldsItemLayer` renders only in this state.
fn panda_is_sitting(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_PANDA_ID
        && (entity_data_byte(values, PANDA_FLAGS_DATA_ID, 0) & PANDA_SITTING_FLAG) != 0
}

/// Vanilla `PandaRenderState.isScared = Panda.isScared()` = `isWorried() && level.isThundering()`.
/// `isWorried()` reads the displayed gene from main/hidden genes; `Level.isThundering()` gates weather-capable
/// dimensions and checks `getThunderLevel(1.0F) > 0.9`, where `getThunderLevel` multiplies thunder by rain.
fn panda_is_scared(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
    world: &WorldStore,
) -> bool {
    if entity_type_id != VANILLA_ENTITY_TYPE_PANDA_ID || !world_is_thundering(world) {
        return false;
    }
    let main_gene = entity_data_byte(values, PANDA_MAIN_GENE_DATA_ID, 0) as i32;
    let hidden_gene = entity_data_byte(values, PANDA_HIDDEN_GENE_DATA_ID, 0) as i32;
    PandaModelVariant::from_genes(main_gene, hidden_gene) == PandaModelVariant::Worried
}

fn world_is_thundering(world: &WorldStore) -> bool {
    if !world_can_have_weather(world) {
        return false;
    }
    let weather = world.weather();
    weather.rain_level.clamp(0.0, 1.0) * weather.thunder_level.clamp(0.0, 1.0) > 0.9
}

fn world_can_have_weather(world: &WorldStore) -> bool {
    let Some(level) = world.level_info() else {
        return true;
    };
    let dimension = level.dimension.as_str();
    let dimension_type = level.dimension_type_name.as_deref();
    !matches!(
        (level.dimension_type_id, dimension, dimension_type),
        (1, _, _)
            | (2, _, _)
            | (_, "minecraft:the_nether", _)
            | (_, "minecraft:the_end", _)
            | (_, _, Some("minecraft:the_nether"))
            | (_, _, Some("minecraft:the_end"))
    )
}

/// Vanilla `TurtleRenderState.hasEgg = !isBaby() && Turtle.hasEgg()` (the synced `HAS_EGG`
/// boolean, id 18). Only the adult turtle renders the `egg_belly` overlay shell, so the
/// projection is gated to the turtle type and excludes babies (matching `extractRenderState`).
fn turtle_has_egg(entity_type_id: i32, values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_TURTLE_ID
        && !ageable_baby(values)
        && entity_data_bool(values, TURTLE_HAS_EGG_DATA_ID, false)
}

/// Vanilla `TurtleRenderState.isLayingEgg = Turtle.isLayingEgg()` (the synced `LAYING_EGG`
/// boolean, id 19). The egg-laying front-leg amplitude lives in the shared `TurtleModel`, so —
/// unlike `hasEgg` — babies are NOT excluded; the projection is only gated to the turtle type.
fn turtle_laying_egg(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_TURTLE_ID
        && entity_data_bool(values, TURTLE_LAYING_EGG_DATA_ID, false)
}

/// Vanilla `EndCrystalRenderState.showsBottom = EndCrystal.showsBottom()` (the synced
/// `DATA_SHOW_BOTTOM` boolean, id 9, default `true`). Gated to the end-crystal type; a crystal
/// without the synced value keeps the vanilla `true` default (the bottom slab is shown).
fn end_crystal_shows_bottom(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id != VANILLA_ENTITY_TYPE_END_CRYSTAL_ID
        || entity_data_bool(values, END_CRYSTAL_SHOW_BOTTOM_DATA_ID, true)
}

fn donkey_model_kind(
    family: DonkeyModelFamily,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    EntityModelKind::Donkey {
        family,
        baby: ageable_baby(values),
        has_chest: chested_horse_has_chest(values),
    }
}

fn undead_horse_model_kind(
    family: UndeadHorseModelFamily,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    EntityModelKind::UndeadHorse {
        family,
        baby: ageable_baby(values),
    }
}

/// The living horse's coat color from synced `Horse.DATA_ID_TYPE_VARIANT` (INT, id 19): vanilla
/// `Variant.byId(typeVariant & 0xFF)`, where `byId` is `ByIdMap.continuous(WRAP)` so an out-of-range
/// id wraps modulo the seven colors. The markings nibble (`(typeVariant & 0xFF00) >> 8`) is the
/// deferred `HorseMarkingLayer`.
fn horse_color_variant(values: &[bbb_protocol::packets::EntityDataValue]) -> HorseColorVariant {
    let color = entity_data_int(values, HORSE_VARIANT_DATA_ID, 0) & 0xFF;
    match color.rem_euclid(7) {
        0 => HorseColorVariant::White,
        1 => HorseColorVariant::Creamy,
        2 => HorseColorVariant::Chestnut,
        3 => HorseColorVariant::Brown,
        4 => HorseColorVariant::Black,
        5 => HorseColorVariant::Gray,
        _ => HorseColorVariant::DarkBrown,
    }
}

/// The living horse's white markings from synced `Horse.DATA_ID_TYPE_VARIANT` (INT, id 19): vanilla
/// `Markings.byId((typeVariant & 0xFF00) >> 8)`, where `byId` is `ByIdMap.continuous(WRAP)` so an
/// out-of-range nibble wraps modulo the five markings. `Markings.NONE` draws no overlay.
fn horse_markings(values: &[bbb_protocol::packets::EntityDataValue]) -> HorseMarkings {
    let markings = (entity_data_int(values, HORSE_VARIANT_DATA_ID, 0) & 0xFF00) >> 8;
    match markings.rem_euclid(5) {
        0 => HorseMarkings::None,
        1 => HorseMarkings::White,
        2 => HorseMarkings::WhiteField,
        3 => HorseMarkings::WhiteDots,
        _ => HorseMarkings::BlackDots,
    }
}

fn llama_model_kind(
    family: LlamaModelFamily,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    let baby = ageable_baby(values);
    EntityModelKind::Llama {
        family,
        variant: LlamaVariant::from_vanilla_id(entity_data_int(values, LLAMA_VARIANT_DATA_ID, 0)),
        baby,
        has_chest: !baby && chested_horse_has_chest(values),
    }
}

fn goat_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    EntityModelKind::Goat {
        baby: ageable_baby(values),
        left_horn: entity_data_bool(values, GOAT_LEFT_HORN_DATA_ID, true),
        right_horn: entity_data_bool(values, GOAT_RIGHT_HORN_DATA_ID, true),
    }
}

/// Vanilla `Goat.getRammingXHeadRot()`: `lowerHeadTick / 20 · maxRammingXHeadRot · π/180`, where the max
/// head pitch is `52.5°` for a baby goat and `30°` for an adult. The world projects the `0..=20` ram
/// counter (advanced from entity events 58/59); `GoatModel.setupAnim` SETs `head.xRot` to this while
/// non-zero, overwriting the head-look pitch during a ram.
fn goat_ramming_x_head_rot(lower_head_tick: i32, baby: bool) -> f32 {
    let max_degrees = if baby { 52.5 } else { 30.0 };
    lower_head_tick as f32 / 20.0 * max_degrees * std::f32::consts::PI / 180.0
}

fn placeholder(name: &'static str, width: f32, height: f32, depth: f32) -> EntityModelKind {
    EntityModelKind::Placeholder {
        name,
        bounds: bbb_renderer::EntityModelBounds {
            width,
            height,
            depth,
        },
    }
}

fn armor_stand_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    let flags = entity_data_byte(values, ARMOR_STAND_CLIENT_FLAGS_DATA_ID, 0);
    EntityModelKind::ArmorStand {
        small: flags & ARMOR_STAND_CLIENT_FLAG_SMALL != 0,
        marker: flags & ARMOR_STAND_CLIENT_FLAG_MARKER != 0,
        show_arms: flags & ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS != 0,
        show_base_plate: flags & ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE == 0,
        pose: armor_stand_pose(values),
    }
}

fn armor_stand_pose(values: &[bbb_protocol::packets::EntityDataValue]) -> ArmorStandModelPose {
    ArmorStandModelPose {
        head: entity_data_rotations(
            values,
            ARMOR_STAND_HEAD_POSE_DATA_ID,
            DEFAULT_ARMOR_STAND_MODEL_POSE.head,
        ),
        body: entity_data_rotations(
            values,
            ARMOR_STAND_BODY_POSE_DATA_ID,
            DEFAULT_ARMOR_STAND_MODEL_POSE.body,
        ),
        left_arm: entity_data_rotations(
            values,
            ARMOR_STAND_LEFT_ARM_POSE_DATA_ID,
            DEFAULT_ARMOR_STAND_MODEL_POSE.left_arm,
        ),
        right_arm: entity_data_rotations(
            values,
            ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID,
            DEFAULT_ARMOR_STAND_MODEL_POSE.right_arm,
        ),
        left_leg: entity_data_rotations(
            values,
            ARMOR_STAND_LEFT_LEG_POSE_DATA_ID,
            DEFAULT_ARMOR_STAND_MODEL_POSE.left_leg,
        ),
        right_leg: entity_data_rotations(
            values,
            ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID,
            DEFAULT_ARMOR_STAND_MODEL_POSE.right_leg,
        ),
    }
}

fn slime_size(values: &[bbb_protocol::packets::EntityDataValue]) -> i32 {
    entity_data_int(values, SLIME_SIZE_DATA_ID, SLIME_DEFAULT_SIZE)
}

fn phantom_size(values: &[bbb_protocol::packets::EntityDataValue]) -> i32 {
    entity_data_int(values, PHANTOM_SIZE_DATA_ID, PHANTOM_DEFAULT_SIZE)
}

fn pufferfish_puff_state(values: &[bbb_protocol::packets::EntityDataValue]) -> i32 {
    entity_data_int(
        values,
        PUFFERFISH_PUFF_STATE_DATA_ID,
        PUFFERFISH_DEFAULT_PUFF_STATE,
    )
}

fn salmon_model_size(values: &[bbb_protocol::packets::EntityDataValue]) -> SalmonModelSize {
    SalmonModelSize::from_vanilla_id(entity_data_int(
        values,
        SALMON_VARIANT_DATA_ID,
        SALMON_DEFAULT_VARIANT,
    ))
}

fn tropical_fish_shape(
    values: &[bbb_protocol::packets::EntityDataValue],
) -> TropicalFishModelShape {
    TropicalFishModelShape::from_vanilla_packed_variant(entity_data_int(
        values,
        TROPICAL_FISH_VARIANT_DATA_ID,
        TROPICAL_FISH_DEFAULT_VARIANT,
    ))
}

/// Vanilla `TropicalFish.getBaseColor(packedVariant) = DyeColor.byId(packedVariant >> 16 & 0xFF)`,
/// projected into the renderer body tint (`TropicalFishRenderer.getModelTint = state.baseColor`).
fn tropical_fish_base_color(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityDyeColor {
    let packed = entity_data_int(
        values,
        TROPICAL_FISH_VARIANT_DATA_ID,
        TROPICAL_FISH_DEFAULT_VARIANT,
    );
    EntityDyeColor::from_vanilla_id((packed >> 16) & 0xFF)
}

/// Vanilla `TropicalFish.getPattern(packedVariant) = Pattern.byId(packedVariant & 0xFFFF)`, the
/// `TropicalFishPatternLayer` overlay selector.
fn tropical_fish_pattern(values: &[bbb_protocol::packets::EntityDataValue]) -> TropicalFishPattern {
    TropicalFishPattern::from_vanilla_packed_variant(entity_data_int(
        values,
        TROPICAL_FISH_VARIANT_DATA_ID,
        TROPICAL_FISH_DEFAULT_VARIANT,
    ))
}

/// Vanilla `TropicalFish.getPatternColor(packedVariant) = DyeColor.byId(packedVariant >> 24 &
/// 0xFF)`, the `TropicalFishPatternLayer` tint (`state.patternColor`).
fn tropical_fish_pattern_color(
    values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityDyeColor {
    let packed = entity_data_int(
        values,
        TROPICAL_FISH_VARIANT_DATA_ID,
        TROPICAL_FISH_DEFAULT_VARIANT,
    );
    EntityDyeColor::from_vanilla_id((packed >> 24) & 0xFF)
}

fn ageable_baby(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_data_bool(values, AGEABLE_MOB_BABY_DATA_ID, false)
}

/// Vanilla `ArmadilloModel.setupAnim` `isHidingInShell` swap, projected from the synced
/// `Armadillo.ARMADILLO_STATE` (data id 18, the `ArmadilloState` enum; SCARED = ordinal 2).
/// Only the steady SCARED state is server-derivable: it hides the body in the shell for every
/// `inStateTicks`, whereas ROLLING/UNROLLING gate the hide on the un-synced `inStateTicks`, so
/// they stay deferred (treated as not rolled up). Defaults to IDLE (not rolled up).
fn armadillo_rolled_up(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    values
        .iter()
        .find(|value| value.data_id == ARMADILLO_STATE_DATA_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::EnumId {
                serializer: EntityDataEnumSerializer::ArmadilloState,
                id,
            } => Some(*id),
            _ => None,
        })
        .is_some_and(|id| id == ARMADILLO_STATE_SCARED_ID)
}

fn chicken_model_variant(
    values: &[bbb_protocol::packets::EntityDataValue],
    variants: Option<&RegistryContentState>,
) -> ChickenModelVariant {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == CHICKEN_VARIANT_DATA_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::ChickenVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registry) = variants {
                chicken_variant_from_registry_id(registry, id)
                    .unwrap_or(ChickenModelVariant::Temperate)
            } else {
                chicken_variant_from_vanilla_registry_id(id)
            }
        })
        .unwrap_or(ChickenModelVariant::Temperate)
}

fn chicken_variant_from_registry_id(
    registry: &RegistryContentState,
    registry_id: i32,
) -> Option<ChickenModelVariant> {
    if registry_id < 0 {
        return None;
    }
    registry
        .entries
        .get(registry_id as usize)
        .and_then(|entry| chicken_variant_from_entry_id(entry.id.as_str()))
}

fn chicken_variant_from_entry_id(id: &str) -> Option<ChickenModelVariant> {
    match id {
        "minecraft:temperate" => Some(ChickenModelVariant::Temperate),
        "minecraft:warm" => Some(ChickenModelVariant::Warm),
        "minecraft:cold" => Some(ChickenModelVariant::Cold),
        _ => None,
    }
}

fn chicken_variant_from_vanilla_registry_id(registry_id: i32) -> ChickenModelVariant {
    match registry_id {
        1 => ChickenModelVariant::Warm,
        2 => ChickenModelVariant::Cold,
        _ => ChickenModelVariant::Temperate,
    }
}

fn frog_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
    variants: Option<&RegistryContentState>,
) -> EntityModelKind {
    EntityModelKind::Frog {
        variant: frog_model_variant(values, variants),
    }
}

fn frog_model_variant(
    values: &[bbb_protocol::packets::EntityDataValue],
    variants: Option<&RegistryContentState>,
) -> FrogModelVariant {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == FROG_VARIANT_DATA_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::FrogVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registry) = variants {
                frog_variant_from_registry_id(registry, id).unwrap_or(FrogModelVariant::Temperate)
            } else {
                frog_variant_from_vanilla_registry_id(id)
            }
        })
        .unwrap_or(FrogModelVariant::Temperate)
}

fn frog_variant_from_registry_id(
    registry: &RegistryContentState,
    registry_id: i32,
) -> Option<FrogModelVariant> {
    if registry_id < 0 {
        return None;
    }
    registry
        .entries
        .get(registry_id as usize)
        .and_then(|entry| frog_variant_from_entry_id(entry.id.as_str()))
}

fn frog_variant_from_entry_id(id: &str) -> Option<FrogModelVariant> {
    match id {
        "minecraft:temperate" => Some(FrogModelVariant::Temperate),
        "minecraft:warm" => Some(FrogModelVariant::Warm),
        "minecraft:cold" => Some(FrogModelVariant::Cold),
        _ => None,
    }
}

// Vanilla `FrogVariants.bootstrap` registers TEMPERATE, WARM, COLD in that order, so the static
// fallback ids (used before the dynamic `frog_variant` registry arrives) are 0/1/2.
fn frog_variant_from_vanilla_registry_id(registry_id: i32) -> FrogModelVariant {
    match registry_id {
        1 => FrogModelVariant::Warm,
        2 => FrogModelVariant::Cold,
        _ => FrogModelVariant::Temperate,
    }
}

fn pig_model_variant(
    values: &[bbb_protocol::packets::EntityDataValue],
    variants: Option<&RegistryContentState>,
) -> PigModelVariant {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == PIG_VARIANT_DATA_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::PigVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registry) = variants {
                pig_variant_from_registry_id(registry, id).unwrap_or(PigModelVariant::Temperate)
            } else {
                pig_variant_from_vanilla_registry_id(id)
            }
        })
        .unwrap_or(PigModelVariant::Temperate)
}

fn pig_variant_from_registry_id(
    registry: &RegistryContentState,
    registry_id: i32,
) -> Option<PigModelVariant> {
    if registry_id < 0 {
        return None;
    }
    registry
        .entries
        .get(registry_id as usize)
        .and_then(|entry| pig_variant_from_entry_id(entry.id.as_str()))
}

fn pig_variant_from_entry_id(id: &str) -> Option<PigModelVariant> {
    match id {
        "minecraft:temperate" => Some(PigModelVariant::Temperate),
        "minecraft:warm" => Some(PigModelVariant::Warm),
        "minecraft:cold" => Some(PigModelVariant::Cold),
        _ => None,
    }
}

fn pig_variant_from_vanilla_registry_id(registry_id: i32) -> PigModelVariant {
    match registry_id {
        1 => PigModelVariant::Warm,
        2 => PigModelVariant::Cold,
        _ => PigModelVariant::Temperate,
    }
}

fn cow_model_variant(
    values: &[bbb_protocol::packets::EntityDataValue],
    variants: Option<&RegistryContentState>,
) -> CowModelVariant {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == COW_VARIANT_DATA_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::CowVariant,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(|id| {
            if let Some(registry) = variants {
                cow_variant_from_registry_id(registry, id).unwrap_or(CowModelVariant::Temperate)
            } else {
                cow_variant_from_vanilla_registry_id(id)
            }
        })
        .unwrap_or(CowModelVariant::Temperate)
}

fn cow_variant_from_registry_id(
    registry: &RegistryContentState,
    registry_id: i32,
) -> Option<CowModelVariant> {
    if registry_id < 0 {
        return None;
    }
    registry
        .entries
        .get(registry_id as usize)
        .and_then(|entry| cow_variant_from_entry_id(entry.id.as_str()))
}

fn cow_variant_from_entry_id(id: &str) -> Option<CowModelVariant> {
    match id {
        "minecraft:temperate" => Some(CowModelVariant::Temperate),
        "minecraft:warm" => Some(CowModelVariant::Warm),
        "minecraft:cold" => Some(CowModelVariant::Cold),
        _ => None,
    }
}

fn cow_variant_from_vanilla_registry_id(registry_id: i32) -> CowModelVariant {
    match registry_id {
        1 => CowModelVariant::Warm,
        2 => CowModelVariant::Cold,
        _ => CowModelVariant::Temperate,
    }
}

fn chested_horse_has_chest(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_data_bool(values, ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID, false)
}

fn zombie_baby(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_data_bool(values, ZOMBIE_BABY_DATA_ID, false)
}

fn piglin_baby(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_data_bool(values, PIGLIN_BABY_DATA_ID, false)
}

fn bogged_sheared(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_data_bool(values, BOGGED_SHEARED_DATA_ID, false)
}

fn entity_data_bool(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: bool,
) -> bool {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

/// Vanilla `Creeper.isPowered()` = the synced `DATA_IS_POWERED` boolean (entity-data index `17`:
/// `Entity` `0..=7`, `LivingEntity` `8..=14`, `Mob` `15`, then `Creeper`'s `DATA_SWELL_DIR` `16` and
/// `DATA_IS_POWERED` `17`). Read only for the creeper, gating the `CreeperPowerLayer` energy swirl.
fn creeper_powered(entity_type_id: i32, values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    const CREEPER_IS_POWERED_DATA_ID: u8 = 17;
    entity_type_id == VANILLA_ENTITY_TYPE_CREEPER_ID
        && entity_data_bool(values, CREEPER_IS_POWERED_DATA_ID, false)
}

/// Vanilla `WitherBoss.isPowered()` = `getHealth() <= getMaxHealth() / 2.0`, gating the
/// `WitherArmorLayer` energy swirl. The current health is the synced `LivingEntity.DATA_HEALTH_ID`
/// float (index `9`); the wither's `Attributes.MAX_HEALTH` base is `300` (mirroring the wolf tail's
/// hardcoded `TAME_MAX_HEALTH` precedent — bbb does not yet track per-entity max-health attribute
/// overrides). A wither with no synced health defaults to full, so it reads un-powered.
/// Maps a projected world armor material onto the renderer's `EntityArmorMaterial` for the
/// `HumanoidArmorLayer` overlay (1:1; the two enums mirror the vanilla `ArmorMaterials` set).
fn armor_material(material: Option<WorldArmorMaterialKind>) -> Option<EntityArmorMaterial> {
    material.map(|material| match material {
        WorldArmorMaterialKind::Leather => EntityArmorMaterial::Leather,
        WorldArmorMaterialKind::Copper => EntityArmorMaterial::Copper,
        WorldArmorMaterialKind::Chainmail => EntityArmorMaterial::Chainmail,
        WorldArmorMaterialKind::Iron => EntityArmorMaterial::Iron,
        WorldArmorMaterialKind::Gold => EntityArmorMaterial::Gold,
        WorldArmorMaterialKind::Diamond => EntityArmorMaterial::Diamond,
        WorldArmorMaterialKind::TurtleScute => EntityArmorMaterial::TurtleScute,
        WorldArmorMaterialKind::Netherite => EntityArmorMaterial::Netherite,
        WorldArmorMaterialKind::ArmadilloScute => EntityArmorMaterial::ArmadilloScute,
    })
}

fn wolf_armor_crackiness(crackiness: WorldWolfArmorCrackiness) -> Option<WolfArmorCrackiness> {
    match crackiness {
        WorldWolfArmorCrackiness::None => None,
        WorldWolfArmorCrackiness::Low => Some(WolfArmorCrackiness::Low),
        WorldWolfArmorCrackiness::Medium => Some(WolfArmorCrackiness::Medium),
        WorldWolfArmorCrackiness::High => Some(WolfArmorCrackiness::High),
    }
}

/// Carries a projected per-slot `DyedItemColor` (a packed RGB `i32`) onto the renderer's armor dye
/// tint (`u32`). The renderer forces it opaque and applies it only to leather, matching vanilla
/// `DyedItemColor.getOrDefault` → `EquipmentLayerRenderer.getColorForLayer`.
fn armor_dye(dye: Option<i32>) -> Option<u32> {
    dye.map(|dye| dye as u32)
}

/// Maps the world-owned vanilla `DyeColor` from `Equippable.llamaSwag(color)` onto the renderer's
/// shared dye enum for `LlamaDecorLayer` `LLAMA_BODY` equipment textures.
fn llama_body_decor_color(color: Option<WorldLlamaBodyDecorColor>) -> Option<EntityDyeColor> {
    color.map(|color| match color {
        WorldLlamaBodyDecorColor::White => EntityDyeColor::White,
        WorldLlamaBodyDecorColor::Orange => EntityDyeColor::Orange,
        WorldLlamaBodyDecorColor::Magenta => EntityDyeColor::Magenta,
        WorldLlamaBodyDecorColor::LightBlue => EntityDyeColor::LightBlue,
        WorldLlamaBodyDecorColor::Yellow => EntityDyeColor::Yellow,
        WorldLlamaBodyDecorColor::Lime => EntityDyeColor::Lime,
        WorldLlamaBodyDecorColor::Pink => EntityDyeColor::Pink,
        WorldLlamaBodyDecorColor::Gray => EntityDyeColor::Gray,
        WorldLlamaBodyDecorColor::LightGray => EntityDyeColor::LightGray,
        WorldLlamaBodyDecorColor::Cyan => EntityDyeColor::Cyan,
        WorldLlamaBodyDecorColor::Purple => EntityDyeColor::Purple,
        WorldLlamaBodyDecorColor::Blue => EntityDyeColor::Blue,
        WorldLlamaBodyDecorColor::Brown => EntityDyeColor::Brown,
        WorldLlamaBodyDecorColor::Green => EntityDyeColor::Green,
        WorldLlamaBodyDecorColor::Red => EntityDyeColor::Red,
        WorldLlamaBodyDecorColor::Black => EntityDyeColor::Black,
    })
}

/// Maps `ShulkerRenderState.attachFace` from world metadata onto the renderer root transform input.
fn entity_attachment_face(face: WorldEntityAttachmentFace) -> EntityAttachmentFace {
    match face {
        WorldEntityAttachmentFace::Down => EntityAttachmentFace::Down,
        WorldEntityAttachmentFace::Up => EntityAttachmentFace::Up,
        WorldEntityAttachmentFace::North => EntityAttachmentFace::North,
        WorldEntityAttachmentFace::South => EntityAttachmentFace::South,
        WorldEntityAttachmentFace::West => EntityAttachmentFace::West,
        WorldEntityAttachmentFace::East => EntityAttachmentFace::East,
    }
}

/// Maps a projected guardian attack beam onto the renderer's `GuardianBeamRenderState` (1:1; the two
/// structs mirror vanilla `GuardianRenderState`'s beam fields).
fn guardian_beam(beam: Option<WorldGuardianBeamSource>) -> Option<GuardianBeamRenderState> {
    beam.map(|beam| GuardianBeamRenderState {
        eye_to_target: beam.eye_to_target,
        eye_height: beam.eye_height,
        attack_time: beam.attack_time,
        attack_scale: beam.attack_scale,
    })
}

/// Maps a projected end-crystal healing beam onto the renderer's `EndCrystalBeamRenderState`.
fn end_crystal_beam(beam: Option<WorldEndCrystalBeamSource>) -> Option<EndCrystalBeamRenderState> {
    beam.map(|beam| EndCrystalBeamRenderState {
        beam_offset: beam.beam_offset,
    })
}

/// Maps a projected ender-dragon healing beam onto the renderer's `EnderDragonBeamRenderState`.
fn ender_dragon_beam(
    beam: Option<WorldEnderDragonBeamSource>,
) -> Option<EnderDragonBeamRenderState> {
    beam.map(|beam| EnderDragonBeamRenderState {
        beam_offset: beam.beam_offset,
    })
}

fn wither_powered(entity_type_id: i32, values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    const WITHER_MAX_HEALTH: f32 = 300.0;
    entity_type_id == VANILLA_ENTITY_TYPE_WITHER_ID
        && entity_data_float(values, LIVING_ENTITY_HEALTH_DATA_ID, WITHER_MAX_HEALTH)
            <= WITHER_MAX_HEALTH / 2.0
}

/// Vanilla `IronGolem.getCrackiness()` = `Crackiness.GOLEM.byFraction(getHealth() / getMaxHealth())`,
/// the iron golem's base `Attributes.MAX_HEALTH` being the constant `100.0`. The synced
/// `LivingEntity.DATA_HEALTH_ID` (index 9) drives the damage-crack overlay tier.
fn iron_golem_crackiness(values: &[bbb_protocol::packets::EntityDataValue]) -> IronGolemCrackiness {
    const IRON_GOLEM_MAX_HEALTH: f32 = 100.0;
    let health = entity_data_float(values, LIVING_ENTITY_HEALTH_DATA_ID, IRON_GOLEM_MAX_HEALTH);
    IronGolemCrackiness::from_health_fraction(health / IRON_GOLEM_MAX_HEALTH)
}

/// Vanilla `CopperGolemRenderer.extractRenderState`: `state.weathering = entity.getWeatherState()`.
/// The synced `WeatheringCopper.WeatherState` ordinal maps 0..=3 to unaffected/exposed/weathered/
/// oxidized, clamping out-of-range values like vanilla's `ByIdMap.OutOfBoundsStrategy.CLAMP`.
fn copper_golem_weathering(
    values: &[bbb_protocol::packets::EntityDataValue],
) -> CopperGolemWeathering {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == COPPER_GOLEM_WEATHER_STATE_DATA_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::EnumId {
                serializer: EntityDataEnumSerializer::WeatheringCopperState,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(CopperGolemWeathering::from_vanilla_id)
        .unwrap_or(CopperGolemWeathering::Unaffected)
}

fn entity_data_int(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: i32,
) -> i32 {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Int(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn entity_data_long(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: i64,
) -> i64 {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Long(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn entity_data_optional_component<'a>(
    values: &'a [bbb_protocol::packets::EntityDataValue],
    data_id: u8,
) -> Option<&'a str> {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::OptionalComponent(Some(value)) => Some(value.as_str()),
            _ => None,
        })
}

fn entity_data_byte(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: i8,
) -> i8 {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Byte(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn entity_data_float(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: f32,
) -> f32 {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Float(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn entity_data_rotations(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: [f32; 3],
) -> [f32; 3] {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Rotations { x, y, z } => Some([*x, *y, *z]),
            _ => None,
        })
        .unwrap_or(default)
}

fn thrown_trident_foil(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID
        && entity_data_bool(values, TRIDENT_FOIL_DATA_ID, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        AddEntity, AttributeSnapshot, ChatFormatting, CommonPlayerSpawnInfo,
        DataComponentPatchSummary, EntityDataValue, EntityEvent, EntityPositionSync, EquipmentSlot,
        EquipmentSlotUpdate, GameProfile, GameProfileProperty, GameType, ItemEnchantmentSummary,
        ItemStackSummary, PlayLogin, PlayTime, PlayerInfoAction, PlayerInfoEntry, PlayerInfoUpdate,
        PlayerTeamMethod, PlayerTeamParameters, RegistryTags, SetCamera, SetEntityData,
        SetEquipment, SetPassengers, SetPlayerTeam, TagNetworkPayload, TeamCollisionRule,
        TeamVisibility, UpdateAttributes, UpdateTags, Vec3d,
    };
    use bbb_world::{
        ArmorMaterialKind as WorldArmorMaterialKind, EntityPickBoundsState, EntityVec3,
        ItemEquipmentSlot, LlamaBodyDecorColor as WorldLlamaBodyDecorColor, RegistryPacketEntry,
    };
    use uuid::Uuid;

    const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;

    #[test]
    fn entity_scene_outline_is_none_without_visible_entity_targets() {
        assert_eq!(
            entity_scene_outline_from_world_at_partial_tick(&WorldStore::new(), 1.0),
            None
        );
    }

    #[test]
    fn entity_scene_outline_projects_pick_bounds_for_all_visible_targets() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            11,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [2.0, 1.0, 3.0],
        ));

        let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.5)
            .expect("expected entity scene outline");

        assert_eq!(outline.boxes.len(), 2);
        assert_selection_box_close(outline.boxes[0].min, [-0.49, 1.0, 2.51]);
        assert_selection_box_close(outline.boxes[0].max, [0.49, 1.7, 3.49]);
        assert_selection_box_close(outline.boxes[1].min, [1.51, 1.0, 2.51]);
        assert_selection_box_close(outline.boxes[1].max, [2.49, 1.7, 3.49]);
    }

    #[test]
    fn entity_scene_outline_uses_bounds_without_pick_radius_inflation() {
        let outline_box = entity_pick_target_box(EntityPickTargetState {
            entity_id: 7,
            position: EntityVec3 {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
            bounds: EntityPickBoundsState::from_centered_size(2.0, 4.0, 6.0, 1.5),
        });

        assert_selection_box_close(outline_box.min, [9.0, 18.0, 27.0]);
        assert_selection_box_close(outline_box.max, [11.0, 22.0, 33.0]);
    }

    #[test]
    fn entity_scene_outline_filters_local_player_and_camera_entity() {
        let mut world = WorldStore::new();
        world.apply_login(&protocol_play_login(10));
        world.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            11,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [2.0, 1.0, 3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [4.0, 1.0, 3.0],
        ));
        assert!(world.apply_set_camera(SetCamera { camera_id: 11 }));

        let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
            .expect("expected non-camera entity scene outline");

        assert_eq!(outline.boxes.len(), 1);
        assert_selection_box_close(outline.boxes[0].min, [3.51, 1.0, 2.51]);
        assert_selection_box_close(outline.boxes[0].max, [4.49, 1.7, 3.49]);
    }

    #[test]
    fn entity_model_instances_project_chicken_adult_and_baby_models() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            26,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            27,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [3.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            85,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [5.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 27,
            values: vec![protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![
                    EntityModelInstance::chicken(26, [1.0, 64.0, -2.0], 0.0, false),
                    EntityModelInstance::chicken(27, [3.0, 64.0, -2.0], 0.0, true),
                    EntityModelInstance::new(85, EntityModelKind::Minecart, [5.0, 64.0, -2.0], 0.0,),
                ],
                1.0,
            )
        );
    }

    #[test]
    fn entity_model_instances_project_sheep_eat_grass_head_pose() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            70,
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            71,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [3.0, 64.0, -2.0],
        ));

        // At rest both entities resolve to the resting head pose.
        let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
        assert_eq!(resting[0].render_state.head_eat, SheepHeadEatPose::NONE);
        assert_eq!(resting[1].render_state.head_eat, SheepHeadEatPose::NONE);

        // Vanilla SheepRenderer.extractRenderState projects the eat animation
        // through the partial tick; the chicken stays at rest.
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 70,
            event_id: 10,
        }));
        let eating = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5);
        assert_eq!(
            eating[0].render_state.head_eat,
            SheepHeadEatPose::from_eat_tick(40, 0.5)
        );
        assert_ne!(eating[0].render_state.head_eat, SheepHeadEatPose::NONE);
        assert_eq!(eating[1].render_state.head_eat, SheepHeadEatPose::NONE);

        // The pose follows the canonical countdown as it decrements.
        world.advance_entity_client_animations(20);
        let mid = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
        assert_eq!(
            mid[0].render_state.head_eat,
            SheepHeadEatPose::from_eat_tick(20, 0.0)
        );
    }

    #[test]
    fn entity_model_instances_project_warden_tendril_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            94,
            VANILLA_ENTITY_TYPE_WARDEN_ID,
            [1.0, 64.0, -2.0],
        ));

        // A warden at rest projects no tendril pulse, so WardenModel.animateTendrils holds at bind.
        let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert_eq!(resting[0].render_state.tendril_animation, 0.0);

        // Vanilla Warden.handleEntityEvent(61) resets tendrilAnimation to 10; getTendrilAnimation
        // lerps (tendrilAnimationO, tendrilAnimation) / 10. After three client ticks the pair is
        // (8, 7), so at partialTick 1.0 the projected pulse is 7/10.
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 94,
            event_id: 61,
        }));
        world.advance_entity_client_animations(3);
        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert_eq!(
            instances[0].render_state.tendril_animation,
            7.0 / 10.0,
            "the projected tendril pulse drives the WardenModel.animateTendrils antenna sway"
        );
    }

    #[test]
    fn entity_model_instances_project_warden_heart_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            96,
            VANILLA_ENTITY_TYPE_WARDEN_ID,
            [1.0, 64.0, -2.0],
        ));

        // A warden between heartbeats projects no heart pulse, so the heart overlay's alpha is 0.
        let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert_eq!(resting[0].render_state.heart_animation, 0.0);

        // With no synced anger, vanilla `Warden.getHeartBeatDelay()` is the calm 40, so the heartbeat
        // (`tickCount % 40 == 0`) first fires on the 40th client tick: `heartAnimation = 10`, then
        // `heartAnimationO = 10; heartAnimation--`, leaving the pair (10, 9). At partialTick 1.0
        // `getHeartAnimation` lerps to 9/10.
        world.advance_entity_client_animations(40);
        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert_eq!(
            instances[0].render_state.heart_animation,
            9.0 / 10.0,
            "the projected heartbeat pulse drives the warden heart emissive overlay's alpha"
        );
    }

    #[test]
    fn entity_model_instances_project_squid_out_of_water_tentacle_and_body_tilt_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            95,
            VANILLA_ENTITY_TYPE_SQUID_ID,
            [1.0, 64.0, -2.0],
        ));
        // The out-of-water branch ignores horizontal swim velocity for pose, but the motion packet
        // keeps this test on the same projection path as the in-water branch.
        assert!(world.apply_entity_position_sync(EntityPositionSync {
            id: 95,
            position: Vec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: Vec3d {
                x: 0.2,
                y: -0.1,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }));

        // A floating squid at rest projects the bind pose into the render state.
        let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        let resting = resting
            .iter()
            .find(|instance| instance.entity_id == 95)
            .unwrap();
        assert_eq!(resting.render_state.squid_tentacle_angle, 0.0);
        assert_eq!(resting.render_state.squid_x_body_rot, 0.0);
        assert_eq!(resting.render_state.squid_z_body_rot, 0.0);

        // One tick out of water uses the suffocating branch: tentacles flex with
        // `abs(sin(tentacleMovement))`, xBodyRot eases toward -90 degrees, and zBodyRot is untouched.
        world.advance_entity_client_animations(1);
        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        let squid = instances
            .iter()
            .find(|instance| instance.entity_id == 95)
            .unwrap();
        assert!(
            squid.render_state.squid_tentacle_angle > 0.0,
            "the projected tentacle angle drives SquidModel.setupAnim: {}",
            squid.render_state.squid_tentacle_angle
        );
        assert!(
            squid.render_state.squid_x_body_rot < 0.0,
            "an out-of-water squid projects a negative body pitch: {}",
            squid.render_state.squid_x_body_rot
        );
        assert_eq!(
            squid.render_state.squid_z_body_rot, 0.0,
            "out of water leaves the swim roll untouched"
        );
    }

    #[test]
    fn entity_model_instances_project_squid_body_yaw_from_world_animation() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity_with_rotation(
            96,
            VANILLA_ENTITY_TYPE_SQUID_ID,
            [1.0, 64.0, -2.0],
            20.0,
            5.0,
            30.0,
        ));

        // Vanilla `LivingEntity.recreateFromPacket` seeds squid `yBodyRot` from
        // the head yaw. A dry squid does not refine that yaw in `Squid.aiStep`, so
        // the native instance must use 30 as `bodyRot` and keep the head yaw
        // relative to that projected body yaw, not the synced transform yaw 20.
        world.advance_entity_client_animations(1);
        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        let squid = instances
            .iter()
            .find(|instance| instance.entity_id == 96)
            .unwrap();
        assert_eq!(squid.render_state.body_rot, 30.0);
        assert_eq!(squid.render_state.head_yaw, 0.0);
        assert_eq!(squid.render_state.head_pitch, 5.0);
        assert!(
            squid.render_state.squid_x_body_rot < 0.0,
            "the same world animation state still feeds SquidRenderer body pitch"
        );
    }

    #[test]
    fn entity_model_instances_project_chicken_wing_flap_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            96,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, -2.0],
        ));
        // Mark the chicken airborne so `Chicken.aiStep` builds flap speed and advances
        // the flap phase (vanilla `onGround()` false branch).
        assert!(world.apply_entity_position_sync(EntityPositionSync {
            id: 96,
            position: Vec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: Vec3d {
                x: 0.0,
                y: -0.1,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }));

        // An unticked chicken projects the bind pose (wings held) into the render state.
        let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        let resting = resting
            .iter()
            .find(|instance| instance.entity_id == 96)
            .unwrap();
        assert_eq!(resting.render_state.chicken_flap, 0.0);
        assert_eq!(resting.render_state.chicken_flap_speed, 0.0);

        // After ticking airborne, the world-side flap accumulator develops a non-zero
        // flap phase and a saturated flap speed, both of which flow through
        // EntityModelSourceState into the renderer EntityRenderState (`ChickenModel.setupAnim`
        // wing zRot).
        world.advance_entity_client_animations(3);
        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        let chicken = instances
            .iter()
            .find(|instance| instance.entity_id == 96)
            .unwrap();
        assert!(
            chicken.render_state.chicken_flap > 0.0,
            "the projected flap phase drives ChickenModel.setupAnim: {}",
            chicken.render_state.chicken_flap
        );
        assert!(
            chicken.render_state.chicken_flap_speed > 0.0,
            "an airborne chicken projects a non-zero flap speed: {}",
            chicken.render_state.chicken_flap_speed
        );
    }

    #[test]
    fn entity_model_instances_project_slime_squish_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            97,
            VANILLA_ENTITY_TYPE_SLIME_ID,
            [1.0, 64.0, -2.0],
        ));
        // Mark the slime grounded so `Slime.tick` sees the air→ground transition and
        // seeds the landing squish target (vanilla `onGround()` true branch from rest).
        assert!(world.apply_entity_position_sync(EntityPositionSync {
            id: 97,
            position: Vec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        }));

        // An unticked slime projects the undeformed cube (squish 0) into the render state.
        let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        let resting = resting
            .iter()
            .find(|instance| instance.entity_id == 97)
            .unwrap();
        assert_eq!(resting.render_state.slime_squish, 0.0);

        // After ticking on the ground, the world-side squish accumulator eases toward
        // the negative landing target, and that flows through EntityModelSourceState
        // into the renderer EntityRenderState (`SlimeRenderer.scale` body stretch).
        world.advance_entity_client_animations(2);
        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        let slime = instances
            .iter()
            .find(|instance| instance.entity_id == 97)
            .unwrap();
        assert!(
            slime.render_state.slime_squish < 0.0,
            "the projected landing squish drives SlimeRenderer.scale: {}",
            slime.render_state.slime_squish
        );
    }

    #[test]
    fn entity_model_instances_project_evoker_fangs_bite_progress_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            98,
            VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
            [1.0, 64.0, -2.0],
        ));

        let bite = |world: &WorldStore| {
            entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
                .iter()
                .find(|instance| instance.entity_id == 98)
                .map(|instance| instance.render_state.evoker_fangs_bite_progress)
        };

        // A fang that has not started its attack is hidden underground: biteProgress 0.
        assert_eq!(bite(&world), Some(0.0));

        // Vanilla `EvokerFangs.handleEntityEvent`: event 4 starts the attack, and the
        // `lifeTicks` countdown drives the biteProgress ramp above 0, flowing through
        // EntityModelSourceState into the renderer EntityRenderState
        // (`EvokerFangsModel.setupAnim` jaw snap / rise / vanish).
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 98,
            event_id: 4,
        }));
        world.advance_entity_client_animations(3);
        let progress = bite(&world).expect("the attacking fang projects an instance");
        assert!(
            progress > 0.0,
            "the projected bite ramp drives EvokerFangsModel.setupAnim: {progress}"
        );
    }

    #[test]
    fn entity_model_instances_project_camel_dash_seconds_from_world() {
        const CAMEL_DASH_DATA_ID: u8 = 19;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            99,
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            [1.0, 64.0, -2.0],
        ));

        let dash = |world: &WorldStore| {
            entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
                .iter()
                .find(|instance| instance.entity_id == 99)
                .map(|instance| {
                    (
                        instance.render_state.camel_dash_seconds,
                        instance.render_state.camel_idle_seconds,
                        instance.render_state.camel_jump_cooldown,
                    )
                })
        };

        // A non-dashing camel projects the stopped-animation sentinel.
        assert_eq!(dash(&world), Some((-1.0, -1.0, 0.0)));

        // Vanilla `Camel.setupAnimationStates`: the synced `DASH` boolean starts `dashAnimationState`,
        // and the elapsed seconds flow through EntityModelSourceState into the renderer EntityRenderState
        // (`CamelModel.setupAnim` looping `CAMEL_DASH` gallop).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 99,
            values: vec![protocol_bool_data(CAMEL_DASH_DATA_ID, true)],
        }));
        world.advance_entity_client_animations(2);
        let progress = dash(&world).expect("the dashing camel projects an instance");
        assert!(
            progress.0 >= 0.0,
            "the projected dash timer drives CamelModel.setupAnim: {progress:?}"
        );
        assert!(
            (progress.1 - 0.1).abs() < 1.0e-6,
            "the projected idle timer drives CamelModel.setupAnim: {progress:?}"
        );
        assert_eq!(
            progress.2, 52.0,
            "the projected dash cooldown drives CamelModel.applyHeadRotation"
        );
    }

    #[test]
    fn entity_model_instances_project_armadillo_peek_seconds_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            100,
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            [1.0, 64.0, -2.0],
        ));

        let peek = |world: &WorldStore| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .iter()
                .find(|instance| instance.entity_id == 100)
                .map(|instance| instance.render_state.armadillo_peek_seconds)
        };

        assert_eq!(peek(&world), Some(-1.0));

        // Vanilla `Armadillo.setupAnimationStates`: entering SCARED starts `peekAnimationState`
        // and fast-forwards it by 50 ticks; that elapsed value flows through EntityModelSourceState
        // into the renderer EntityRenderState (`ArmadilloModel.setupAnim` `ARMADILLO_PEEK`).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 100,
            values: vec![protocol_armadillo_state_data(2)],
        }));
        let fast_forwarded = peek(&world).expect("the scared armadillo projects an instance");
        assert!((fast_forwarded - 2.5).abs() < 1.0e-6);

        // Event 64 restarts the peek on the next client tick.
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 100,
            event_id: 64,
        }));
        world.advance_entity_client_animations(1);
        let restarted = peek(&world).expect("the restarted armadillo projects an instance");
        assert!((restarted - 0.0).abs() < 1.0e-6);
    }

    #[test]
    fn entity_model_instances_project_axolotl_play_dead_from_world() {
        const AXOLOTL_PLAYING_DEAD_DATA_ID: u8 = 19;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            95,
            VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            [1.0, 64.0, -2.0],
        ));

        let play_dead = |world: &WorldStore| {
            entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
                .iter()
                .find(|instance| instance.entity_id == 95)
                .map(|instance| instance.render_state.axolotl_playing_dead_factor)
        };

        // An awake axolotl projects no play-dead blend.
        assert_eq!(play_dead(&world), Some(0.0));

        // Vanilla `Axolotl.playingDeadAnimator`: the synced `DATA_PLAYING_DEAD` flag eases the
        // factor up, flowing through EntityModelSourceState into the renderer EntityRenderState
        // (`AdultAxolotlModel.setupPlayDeadAnimation` limp-on-its-side pose).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 95,
            values: vec![protocol_bool_data(AXOLOTL_PLAYING_DEAD_DATA_ID, true)],
        }));
        world.advance_entity_client_animations(3);
        let factor = play_dead(&world).expect("the play-dead axolotl projects an instance");
        assert!(
            factor > 0.0,
            "the projected play-dead factor drives AdultAxolotlModel.setupAnim: {factor}"
        );
    }

    #[test]
    fn entity_model_instances_project_allay_dance_from_world() {
        const ALLAY_DANCING_DATA_ID: u8 = 16;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            96,
            VANILLA_ENTITY_TYPE_ALLAY_ID,
            [1.0, 64.0, -2.0],
        ));

        let dance = |world: &WorldStore| {
            entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
                .iter()
                .find(|instance| instance.entity_id == 96)
                .map(|instance| {
                    (
                        instance.render_state.allay_dancing,
                        instance.render_state.allay_spinning,
                        instance.render_state.allay_spinning_progress,
                    )
                })
        };

        // A non-dancing allay projects the inert dance state (head-look pose, no spin).
        assert_eq!(dance(&world), Some((false, false, 0.0)));

        // Vanilla `Allay.tick`: the synced `DATA_DANCING` flag opens the dance, and the spin
        // sub-window state flows through EntityModelSourceState into the renderer EntityRenderState
        // (`AllayModel.setupAnim` dance branch: body spin/sway + head tilt).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 96,
            values: vec![protocol_bool_data(ALLAY_DANCING_DATA_ID, true)],
        }));
        world.advance_entity_client_animations(1);
        let (dancing, spinning, progress) =
            dance(&world).expect("the dancing allay projects an instance");
        assert!(
            dancing,
            "the synced flag drives AllayModel.setupAnim's dance branch"
        );
        assert!(spinning, "the dance opens in the spin sub-window");
        assert!(
            progress > 0.0,
            "the projected spin ramp drives the body spin: {progress}"
        );
    }

    #[test]
    fn entity_model_instances_project_allay_holding_item_progress_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            97,
            VANILLA_ENTITY_TYPE_ALLAY_ID,
            [1.0, 64.0, -2.0],
        ));

        let holding_progress = |world: &WorldStore, partial: f32| {
            entity_model_instances_from_world_at_partial_tick(world, None, partial)
                .iter()
                .find(|instance| instance.entity_id == 97)
                .map(|instance| instance.render_state.allay_holding_item_progress)
        };

        assert_eq!(holding_progress(&world, 1.0), Some(0.0));
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: 97,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary {
                    item_id: Some(42),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        }));
        world.advance_entity_client_animations(3);

        let progress = holding_progress(&world, 1.0)
            .expect("the allay projects a renderer instance after ticking");
        assert!(
            (progress - 0.6).abs() < 1.0e-6,
            "native forwards Allay.getHoldingItemAnimationProgress into EntityModelRenderState: {progress}"
        );
    }

    #[test]
    fn entity_model_instances_project_head_look_from_world() {
        let mut world = WorldStore::new();
        // Body yaw 30, head yaw 100, pitch -20: net head yaw =
        // wrapDegrees(100 - 30) = 70, head pitch = -20.
        world.apply_add_entity(protocol_add_entity_with_rotation(
            70,
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            [1.0, 64.0, -2.0],
            30.0,
            -20.0,
            100.0,
        ));
        // Head aligned with body and level: no look turn.
        world.apply_add_entity(protocol_add_entity_with_rotation(
            71,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [3.0, 64.0, -2.0],
            45.0,
            0.0,
            45.0,
        ));
        // Body yaw 10, head yaw 200: diff 190 wraps to -170 (shortest turn).
        world.apply_add_entity(protocol_add_entity_with_rotation(
            72,
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            [5.0, 64.0, -2.0],
            10.0,
            5.0,
            200.0,
        ));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        let find = |id: i32| {
            instances
                .iter()
                .find(|instance| instance.entity_id == id)
                .unwrap_or_else(|| panic!("missing entity {id}"))
        };

        let sheep = find(70).render_state;
        assert_eq!(sheep.head_yaw, 70.0);
        assert_eq!(sheep.head_pitch, -20.0);

        let chicken = find(71).render_state;
        assert_eq!(chicken.head_yaw, 0.0);
        assert_eq!(chicken.head_pitch, 0.0);

        let wrapped = find(72).render_state;
        assert_eq!(wrapped.head_yaw, -170.0);
        assert_eq!(wrapped.head_pitch, 5.0);
    }

    #[test]
    fn entity_model_instances_project_polar_bear_standing_scale() {
        const POLAR_BEAR_STANDING_DATA_ID: u8 = 18;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            80,
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            81,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [3.0, 64.0, -2.0],
        ));

        // A polar bear on all fours and any other entity carry a zero scale.
        let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert_eq!(resting[0].render_state.polar_bear_stand_scale, 0.0);
        assert_eq!(resting[1].render_state.polar_bear_stand_scale, 0.0);

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 80,
            values: vec![protocol_bool_data(POLAR_BEAR_STANDING_DATA_ID, true)],
        }));
        world.advance_entity_client_animations(1);

        // Vanilla PolarBearRenderer.extractRenderState reads
        // getStandingAnimationScale(partialTick); after one tick that is
        // lerp(0.5, 0, 1) / 6.
        let standing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5);
        assert_eq!(standing[0].render_state.polar_bear_stand_scale, 0.5 / 6.0);
        assert_eq!(standing[1].render_state.polar_bear_stand_scale, 0.0);
    }

    #[test]
    fn entity_model_instances_project_shulker_peek() {
        // Vanilla Shulker.DATA_PEEK_ID (17, BYTE), a 0..=100 percentage; the client peek state
        // advances 0.05/tick toward raw·0.01 and the render state reads the partial-tick lerp
        // `Shulker.getClientPeekAmount` (`Mth.lerp(partialTick, currentPeekAmountO, current)`).
        const VANILLA_SHULKER_PEEK_DATA_ID: u8 = 17;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            82,
            VANILLA_ENTITY_TYPE_SHULKER_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            83,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [3.0, 64.0, -2.0],
        ));

        let peek = |world: &WorldStore, id: i32, partial: f32| {
            entity_model_instances_from_world_at_partial_tick(world, None, partial)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .shulker_peek
        };

        // A closed shulker and every other entity carry a zero peek (the closed/bind pose).
        assert_eq!(peek(&world, 82, 1.0), 0.0);
        assert_eq!(peek(&world, 83, 1.0), 0.0);

        // Open the lid fully (raw 100 → target 1.0), then advance one tick: the client peek steps
        // 0.05 from 0. At partial-tick 0.5 the render state lerps `0 + 0.5·(0.05 − 0) = 0.025`.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 82,
            values: vec![protocol_byte_data(VANILLA_SHULKER_PEEK_DATA_ID, 100)],
        }));
        world.advance_entity_client_animations(1);
        assert!((peek(&world, 82, 0.5) - 0.025).abs() < 1.0e-6);
        // The chicken has no peek state, so it stays at the closed/bind pose.
        assert_eq!(peek(&world, 83, 0.5), 0.0);
    }

    #[test]
    fn entity_model_instances_project_shulker_attach_face() {
        // Vanilla Shulker.DATA_ATTACH_FACE_ID (16, DIRECTION) feeds
        // `ShulkerRenderState.attachFace`, whose default is `Direction.DOWN`.
        const VANILLA_SHULKER_ATTACH_FACE_DATA_ID: u8 = 16;
        const DIRECTION_NORTH: i32 = 2;
        const DIRECTION_NEGATIVE_ONE: i32 = -1;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            82,
            VANILLA_ENTITY_TYPE_SHULKER_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            83,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [3.0, 64.0, -2.0],
        ));

        let attach_face = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .shulker_attach_face
        };

        assert_eq!(attach_face(&world, 82), EntityAttachmentFace::Down);
        assert_eq!(attach_face(&world, 83), EntityAttachmentFace::Down);

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 82,
            values: vec![protocol_direction_data(
                VANILLA_SHULKER_ATTACH_FACE_DATA_ID,
                DIRECTION_NORTH,
            )],
        }));
        assert_eq!(attach_face(&world, 82), EntityAttachmentFace::North);
        assert_eq!(attach_face(&world, 83), EntityAttachmentFace::Down);

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 82,
            values: vec![protocol_direction_data(
                VANILLA_SHULKER_ATTACH_FACE_DATA_ID,
                DIRECTION_NEGATIVE_ONE,
            )],
        }));
        // Vanilla `Direction.BY_ID` uses positive-modulo wrap, so -1 wraps to EAST.
        assert_eq!(attach_face(&world, 82), EntityAttachmentFace::East);
    }

    #[test]
    fn entity_model_instances_project_death_animation_time() {
        const VANILLA_ENTITY_HEALTH_DATA_ID: u8 = 9;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            82,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, -2.0],
        ));

        // A living entity at rest carries deathTime 0 and no red overlay.
        let alive = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
        assert_eq!(alive[0].render_state.death_time, 0.0);
        assert!(!alive[0].render_state.has_red_overlay);

        // Vanilla isDeadOrDying(): health <= 0 starts the death counter; tickDeath
        // increments it each client tick, projected (plus the partial tick) as
        // LivingEntityRenderState.deathTime and driving the red overlay.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 82,
            values: vec![protocol_float_data(VANILLA_ENTITY_HEALTH_DATA_ID, 0.0)],
        }));
        world.advance_entity_client_animations(2);
        let dying = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);
        assert_eq!(dying[0].render_state.death_time, 2.25);
        assert!(dying[0].render_state.has_red_overlay);
    }

    #[test]
    fn entity_model_instances_fold_freeze_shake_into_body_rot() {
        const VANILLA_ENTITY_TICKS_FROZEN_DATA_ID: u8 = 7;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            83,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, -2.0],
        ));

        // A living entity that is not frozen solid has an unshaken body yaw.
        let warm = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
        assert_eq!(warm[0].render_state.body_rot, 0.0);

        // Vanilla Entity.isFullyFrozen(): ticksFrozen >= 140. setupRotations then
        // adds cos(floor(ageInTicks) * 3.25) * π * 0.4 to the body yaw; the shake
        // uses the floored (integer) tick count, so it does not lerp with partial.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 83,
            values: vec![protocol_int_data(VANILLA_ENTITY_TICKS_FROZEN_DATA_ID, 140)],
        }));
        world.advance_entity_client_animations(2);
        let frozen = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);
        let expected_shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;
        assert!((frozen[0].render_state.body_rot - expected_shake).abs() < 1e-6);
        // The head turn relative to the body is unchanged by the shake.
        assert_eq!(frozen[0].render_state.head_yaw, 0.0);
    }

    #[test]
    fn entity_model_instances_shake_zombie_family_while_converting() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            84,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            85,
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            [3.0, 64.0, -2.0],
        ));
        world.advance_entity_client_animations(2);

        let shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;
        let body_rot = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .body_rot
        };

        // A non-converting zombie / zombie villager does not shake.
        assert_eq!(body_rot(&world, 84), 0.0);
        assert_eq!(body_rot(&world, 85), 0.0);

        // AbstractZombieRenderer.isShaking ORs in DATA_DROWNED_CONVERSION_ID (18).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 84,
            values: vec![protocol_bool_data(ZOMBIE_DROWNED_CONVERSION_DATA_ID, true)],
        }));
        assert!((body_rot(&world, 84) - shake).abs() < 1e-6);

        // ZombieVillagerRenderer additionally ORs in DATA_CONVERTING_ID (19).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 85,
            values: vec![protocol_bool_data(ZOMBIE_VILLAGER_CONVERTING_DATA_ID, true)],
        }));
        assert!((body_rot(&world, 85) - shake).abs() < 1e-6);
    }

    #[test]
    fn entity_model_instances_shake_striders_while_suffocating() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            86,
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            [1.0, 64.0, -2.0],
        ));
        world.advance_entity_client_animations(2);

        let strider = |world: &WorldStore| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.25)
                .into_iter()
                .find(|instance| instance.entity_id == 86)
                .unwrap()
        };

        // A warm strider does not shake and keeps the non-cold texture variant.
        let warm = strider(&world);
        assert_eq!(warm.render_state.body_rot, 0.0);
        assert_eq!(
            warm.kind,
            EntityModelKind::Strider {
                baby: false,
                cold: false
            }
        );

        // StriderRenderer.isShaking ORs in StriderRenderState.isSuffocating,
        // which is extracted from DATA_SUFFOCATING (19). The same flag also
        // selects the cold texture.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 86,
            values: vec![protocol_bool_data(STRIDER_SUFFOCATING_DATA_ID, true)],
        }));
        let shaking = strider(&world);
        let expected_shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;
        assert!((shaking.render_state.body_rot - expected_shake).abs() < 1e-6);
        assert_eq!(shaking.render_state.head_yaw, 0.0);
        assert_eq!(
            shaking.kind,
            EntityModelKind::Strider {
                baby: false,
                cold: true
            }
        );
    }

    #[test]
    fn entity_model_instances_project_auto_spin_attack() {
        // Vanilla LivingEntity.DATA_LIVING_ENTITY_FLAGS id and the spin-attack bit.
        const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
        const LIVING_ENTITY_FLAG_SPIN_ATTACK: i8 = 4;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            86,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, -2.0],
        ));
        // A non-living entity never carries the living-entity flags byte.
        world.apply_add_entity(protocol_add_entity(
            87,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            [3.0, 64.0, -2.0],
        ));

        let auto_spin = |world: &WorldStore, id: i32, partial: f32| {
            entity_model_instances_from_world_at_partial_tick(world, None, partial)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .auto_spin_age_ticks
        };

        // A living entity at rest is not spinning.
        assert_eq!(auto_spin(&world, 86, 0.0), None);

        // Vanilla LivingEntity.isAutoSpinAttack(): DATA_LIVING_ENTITY_FLAGS & 4.
        // setupRotations then reads the lerped ageInTicks (tickCount + partial).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 86,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_SPIN_ATTACK | 0x01,
            )],
        }));
        world.advance_entity_client_animations(3);
        assert_eq!(auto_spin(&world, 86, 0.5), Some(3.5));

        // Clearing the spin bit (other living flags still set) stops the spin.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 86,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                0x01
            )],
        }));
        assert_eq!(auto_spin(&world, 86, 0.5), None);

        // The living-entity gate keeps a non-living entity from ever spinning, even
        // if a stray flags byte is present.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 87,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_SPIN_ATTACK,
            )],
        }));
        assert_eq!(auto_spin(&world, 87, 0.5), None);
    }

    #[test]
    fn entity_model_instances_project_boat_rowing_and_damage_times() {
        const VEHICLE_HURT_TIME_DATA_ID: u8 = 8;
        const VEHICLE_HURT_DIR_DATA_ID: u8 = 9;
        const VEHICLE_DAMAGE_DATA_ID: u8 = 10;
        const BOAT_PADDLE_LEFT_DATA_ID: u8 = 11;
        const BOAT_PADDLE_RIGHT_DATA_ID: u8 = 12;
        const BOAT_BUBBLE_TIME_DATA_ID: u8 = 13;
        const ADVANCE: f32 = std::f32::consts::PI / 8.0;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            90,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            91,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [2.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 90,
            passenger_ids: vec![91],
        }));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 90,
            values: vec![
                protocol_int_data(VEHICLE_HURT_TIME_DATA_ID, 10),
                protocol_int_data(VEHICLE_HURT_DIR_DATA_ID, -1),
                protocol_float_data(VEHICLE_DAMAGE_DATA_ID, 20.0),
                protocol_bool_data(BOAT_PADDLE_LEFT_DATA_ID, true),
                protocol_bool_data(BOAT_PADDLE_RIGHT_DATA_ID, true),
                protocol_int_data(BOAT_BUBBLE_TIME_DATA_ID, 60),
            ],
        }));

        world.advance_entity_client_animations(2);
        let render_state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5)
            .into_iter()
            .find(|instance| instance.entity_id == 90)
            .unwrap()
            .render_state;

        assert!((render_state.boat_rowing_time_left - ADVANCE * 1.5).abs() < 1.0e-6);
        assert!((render_state.boat_rowing_time_right - ADVANCE * 1.5).abs() < 1.0e-6);
        assert!((render_state.boat_hurt_time - 7.5).abs() < 1.0e-6);
        assert_eq!(render_state.boat_hurt_dir, -1);
        assert!((render_state.boat_damage_time - 17.5).abs() < 1.0e-6);
        let first_bubble_angle = 10.0 * (0.5_f32).sin() * 0.05;
        let second_bubble_angle = 10.0 * (1.0_f32).sin() * 0.1;
        let expected_bubble_angle =
            first_bubble_angle + (second_bubble_angle - first_bubble_angle) * 0.5;
        assert!((render_state.boat_bubble_angle - expected_bubble_angle).abs() < 1.0e-6);
    }

    #[test]
    fn entity_model_instance_projects_boat_underwater_from_source() {
        // Vanilla `AbstractBoatRenderer.extractRenderState` copies `AbstractBoat.isUnderWater()`
        // into `BoatRenderState.isUnderWater`; the world layer owns that fluid projection and
        // native must preserve it when building the renderer instance.
        let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
            "entity_id": 92,
            "entity_type_id": VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
            "y_rot": 0.0,
            "boat_bubble_angle": 6.0,
            "boat_underwater": true,
            "data_values": []
        }))
        .unwrap();

        let instance = entity_model_instance(
            source,
            &WorldStore::new(),
            None,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            instance.kind,
            EntityModelKind::Boat {
                family: BoatModelFamily::Oak,
                chest: false,
            }
        );
        assert_eq!(instance.render_state.boat_bubble_angle, 6.0);
        assert!(instance.render_state.boat_underwater);
    }

    #[test]
    fn entity_model_instances_project_aggressive_for_zombie_family() {
        // Vanilla Mob.DATA_MOB_FLAGS_ID (15) and the aggressive bit (4).
        const VANILLA_MOB_FLAGS_DATA_ID: u8 = 15;
        const MOB_FLAG_AGGRESSIVE: i8 = 4;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            90,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            [1.0, 64.0, -2.0],
        ));

        let aggressive = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .is_aggressive
        };

        // A calm zombie projects is_aggressive = false.
        assert!(!aggressive(&world, 90));

        // Setting Mob.isAggressive (DATA_MOB_FLAGS_ID & 4) projects through to the held-out
        // arm render state.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 90,
            values: vec![protocol_byte_data(
                VANILLA_MOB_FLAGS_DATA_ID,
                MOB_FLAG_AGGRESSIVE,
            )],
        }));
        assert!(aggressive(&world, 90));
    }

    #[test]
    fn entity_model_instances_project_enderman_carrying_and_creepy() {
        // Vanilla Enderman accessors: DATA_CARRY_STATE (16, OPTIONAL_BLOCK_STATE serializer
        // 15), DATA_CREEPY (17, BOOLEAN serializer 8).
        const CARRY_STATE_DATA_ID: u8 = 16;
        const CREEPY_DATA_ID: u8 = 17;
        const OPTIONAL_BLOCK_STATE_SERIALIZER_ID: i32 = 15;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            94,
            VANILLA_ENTITY_TYPE_ENDERMAN_ID,
            [1.0, 64.0, -2.0],
        ));

        let state = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
        };

        // A freshly spawned enderman carries nothing and is not creepy.
        let calm = state(&world, 94);
        assert!(!calm.enderman_carrying);
        assert!(!calm.enderman_creepy);

        // A present carried block (`DATA_CARRY_STATE` set) and `DATA_CREEPY` project through
        // to the held-out arm pose and the creepy head/hat shift.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 94,
            values: vec![
                EntityDataValue {
                    data_id: CARRY_STATE_DATA_ID,
                    serializer_id: OPTIONAL_BLOCK_STATE_SERIALIZER_ID,
                    value: EntityDataValueKind::OptionalBlockState(Some(10)),
                },
                protocol_bool_data(CREEPY_DATA_ID, true),
            ],
        }));
        let primed = state(&world, 94);
        assert!(primed.enderman_carrying);
        assert!(primed.enderman_creepy);
    }

    #[test]
    fn entity_model_instances_project_bat_resting() {
        // Vanilla Bat.DATA_ID_FLAGS (16, BYTE) and the resting bit (1).
        const VANILLA_BAT_FLAGS_DATA_ID: u8 = 16;
        const BAT_FLAG_RESTING: i8 = 1;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            95,
            VANILLA_ENTITY_TYPE_BAT_ID,
            [1.0, 64.0, -2.0],
        ));

        let resting = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .bat_resting
        };

        // A flying bat projects bat_resting = false.
        assert!(!resting(&world, 95));

        // Setting Bat.isResting (DATA_ID_FLAGS & 1) projects through to the hanging-pose state.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 95,
            values: vec![protocol_byte_data(
                VANILLA_BAT_FLAGS_DATA_ID,
                BAT_FLAG_RESTING
            )],
        }));
        assert!(resting(&world, 95));
    }

    #[test]
    fn entity_model_instances_project_vex_charging() {
        // Vanilla Vex.DATA_FLAGS_ID (16, BYTE) and the FLAG_IS_CHARGING bit (1).
        const VANILLA_VEX_FLAGS_DATA_ID: u8 = 16;
        const VEX_FLAG_IS_CHARGING: i8 = 1;
        const PLAIN_ITEM_ID: i32 = 702;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            97,
            VANILLA_ENTITY_TYPE_VEX_ID,
            [1.0, 64.0, -2.0],
        ));
        // A bat reuses data id 16 for its OWN resting flag — used below to prove the vex
        // charging projection is gated to the vex and never leaks onto another type.
        world.apply_add_entity(protocol_add_entity(
            98,
            VANILLA_ENTITY_TYPE_BAT_ID,
            [2.0, 64.0, -2.0],
        ));

        let state = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
        };
        let equip =
            |entity_id: i32, slot: EquipmentSlot, item_id: Option<i32>, count: i32| SetEquipment {
                entity_id,
                slots: vec![EquipmentSlotUpdate {
                    slot,
                    item: ItemStackSummary {
                        item_id,
                        count,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                }],
            };

        // An idle vex projects vex_charging = false.
        let idle = state(&world, 97);
        assert!(!idle.vex_charging);
        assert!(!idle.vex_right_hand_item_non_empty);
        assert!(!idle.vex_left_hand_item_non_empty);

        // Setting Vex.isCharging (DATA_FLAGS_ID & 1) projects through to the charging pose.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 97,
            values: vec![protocol_byte_data(
                VANILLA_VEX_FLAGS_DATA_ID,
                VEX_FLAG_IS_CHARGING
            )],
        }));
        assert!(state(&world, 97).vex_charging);

        // Vanilla `ArmedEntityRenderState` checks RIGHT/LEFT hand item-state emptiness. bbb's current
        // Vex projection maps default RIGHT main hand to RIGHT and offhand to LEFT.
        assert!(world.apply_set_equipment(equip(
            97,
            EquipmentSlot::MainHand,
            Some(PLAIN_ITEM_ID),
            1
        )));
        let main_hand_item = state(&world, 97);
        assert!(main_hand_item.vex_right_hand_item_non_empty);
        assert!(!main_hand_item.vex_left_hand_item_non_empty);

        assert!(world.apply_set_equipment(equip(
            97,
            EquipmentSlot::OffHand,
            Some(PLAIN_ITEM_ID),
            1
        )));
        let both_hands = state(&world, 97);
        assert!(both_hands.vex_right_hand_item_non_empty);
        assert!(both_hands.vex_left_hand_item_non_empty);

        assert!(world.apply_set_equipment(equip(97, EquipmentSlot::MainHand, None, 0)));
        let offhand_only = state(&world, 97);
        assert!(!offhand_only.vex_right_hand_item_non_empty);
        assert!(offhand_only.vex_left_hand_item_non_empty);

        // The same flag byte set on a non-vex (bat) does NOT project vex_charging — the
        // derivation is gated to vanilla_is_vex. The same held items also do not project Vex hand state.
        assert!(world.apply_set_equipment(equip(
            98,
            EquipmentSlot::MainHand,
            Some(PLAIN_ITEM_ID),
            1
        )));
        assert!(world.apply_set_equipment(equip(
            98,
            EquipmentSlot::OffHand,
            Some(PLAIN_ITEM_ID),
            1
        )));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 98,
            values: vec![protocol_byte_data(
                VANILLA_VEX_FLAGS_DATA_ID,
                VEX_FLAG_IS_CHARGING
            )],
        }));
        let bat = state(&world, 98);
        assert!(!bat.vex_charging);
        assert!(!bat.vex_right_hand_item_non_empty);
        assert!(!bat.vex_left_hand_item_non_empty);
    }

    #[test]
    fn entity_model_instances_project_turtle_has_egg() {
        // Vanilla Turtle.HAS_EGG (AgeableMob 16/17 then Turtle's BOOLEAN data id 18) and
        // TurtleRenderer.extractRenderState: state.hasEgg = !entity.isBaby() && entity.hasEgg().
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            140,
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            [1.0, 64.0, -2.0],
        ));
        // A second turtle (made a baby below), plus a non-turtle (bat) that reuses data id 18 for
        // its own flag — used to prove the egg projection is gated to adult turtles.
        world.apply_add_entity(protocol_add_entity(
            141,
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            [2.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            142,
            VANILLA_ENTITY_TYPE_BAT_ID,
            [3.0, 64.0, -2.0],
        ));

        let has_egg = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .turtle_has_egg
        };

        // An adult turtle without the flag projects turtle_has_egg = false.
        assert!(!has_egg(&world, 140));

        // Setting Turtle.HAS_EGG (data id 18) on the adult projects the egg belly.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 140,
            values: vec![protocol_bool_data(TURTLE_HAS_EGG_DATA_ID, true)],
        }));
        assert!(has_egg(&world, 140));

        // A baby turtle with HAS_EGG set stays false (gated on !isBaby()).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 141,
            values: vec![
                protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
                protocol_bool_data(TURTLE_HAS_EGG_DATA_ID, true),
            ],
        }));
        assert!(!has_egg(&world, 141));

        // The same flag on a non-turtle (bat) does NOT project turtle_has_egg.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 142,
            values: vec![protocol_bool_data(TURTLE_HAS_EGG_DATA_ID, true)],
        }));
        assert!(!has_egg(&world, 142));
    }

    #[test]
    fn entity_model_instances_project_pillager_charging_crossbow() {
        // Vanilla Pillager.IS_CHARGING_CROSSBOW (BOOLEAN data id 17, after Raider.IS_CELEBRATING 16)
        // and Pillager.getArmPose: a charging pillager renders CROSSBOW_CHARGE, suppressing the
        // CROSSBOW_HOLD pose. The evoker reuses data id 17 for DATA_SPELL_CASTING_ID, so the
        // projection must be gated to the pillager type.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            160,
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            161,
            VANILLA_ENTITY_TYPE_EVOKER_ID,
            [2.0, 64.0, -2.0],
        ));

        let charging = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .is_charging_crossbow
        };

        // A pillager without the flag is not charging.
        assert!(!charging(&world, 160));

        // Setting IS_CHARGING_CROSSBOW (data id 17) projects the charge state.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 160,
            values: vec![protocol_bool_data(
                PILLAGER_IS_CHARGING_CROSSBOW_DATA_ID,
                true
            )],
        }));
        assert!(charging(&world, 160));

        // The same data id 17 on an evoker (its spell-casting byte slot) does NOT project charging.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 161,
            values: vec![protocol_bool_data(
                PILLAGER_IS_CHARGING_CROSSBOW_DATA_ID,
                true
            )],
        }));
        assert!(!charging(&world, 161));
    }

    #[test]
    fn entity_model_instances_project_illager_celebrating() {
        // Vanilla Raider.IS_CELEBRATING (BOOLEAN data id 16) and SpellcasterIllager/Vindicator
        // .getArmPose: the evoker and vindicator render the CELEBRATING dance while it is set. The
        // pillager never returns CELEBRATING, so the projection is gated to the evoker/vindicator.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            170,
            VANILLA_ENTITY_TYPE_VINDICATOR_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            171,
            VANILLA_ENTITY_TYPE_EVOKER_ID,
            [2.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            172,
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
            [3.0, 64.0, -2.0],
        ));

        let celebrating = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .illager_celebrating
        };

        // No flag → not celebrating.
        assert!(!celebrating(&world, 170));

        // Raider.IS_CELEBRATING (data id 16) projects the dance for the vindicator and evoker.
        for id in [170, 171] {
            assert!(world.apply_set_entity_data(SetEntityData {
                id,
                values: vec![protocol_bool_data(RAIDER_IS_CELEBRATING_DATA_ID, true)],
            }));
            assert!(celebrating(&world, id));
        }

        // The same flag on a pillager does NOT project celebrating (it never returns CELEBRATING).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 172,
            values: vec![protocol_bool_data(RAIDER_IS_CELEBRATING_DATA_ID, true)],
        }));
        assert!(!celebrating(&world, 172));
    }

    #[test]
    fn entity_model_instances_project_illager_main_hand_empty() {
        // Vanilla `IllagerModel.setupAnim` ATTACKING chooses empty-hand zombie arms vs armed weapon
        // swing from `state.getMainHandItemState().isEmpty()`. Native projects that from canonical
        // equipment so renderer can choose the right branch.
        const PLAIN_ITEM_ID: i32 = 701;

        let main_hand = |entity_id: i32, item_id: Option<i32>, count: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary {
                    item_id,
                    count,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        };
        let projected = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .illager_main_hand_empty
        };

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            173,
            VANILLA_ENTITY_TYPE_VINDICATOR_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            174,
            VANILLA_ENTITY_TYPE_EVOKER_ID,
            [2.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            175,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            [3.0, 64.0, -2.0],
        ));

        assert!(projected(&world, 173));
        assert!(projected(&world, 174));
        assert!(!projected(&world, 175));

        assert!(world.apply_set_equipment(main_hand(173, Some(PLAIN_ITEM_ID), 1)));
        assert!(!projected(&world, 173));

        assert!(world.apply_set_equipment(main_hand(173, None, 0)));
        assert!(projected(&world, 173));
    }

    #[test]
    fn entity_model_instances_project_piglin_dancing() {
        // Vanilla Piglin.isDancing() (BOOLEAN data id 19) and Piglin.getArmPose → DANCING: a regular
        // piglin dances by a soul campfire. The brute and zombified piglin never return DANCING, so the
        // projection is gated to the regular piglin type.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            180,
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            [1.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            181,
            VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
            [2.0, 64.0, -3.0],
        ));

        let dancing = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .piglin_dancing
        };

        // No flag → not dancing.
        assert!(!dancing(&world, 180));

        // Piglin.DATA_IS_DANCING (data id 19) projects the dance for the regular piglin.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 180,
            values: vec![protocol_bool_data(PIGLIN_IS_DANCING_DATA_ID, true)],
        }));
        assert!(dancing(&world, 180));

        // The same data id on a piglin brute does NOT project dancing (the brute never dances; that id
        // is not even defined on it).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 181,
            values: vec![protocol_bool_data(PIGLIN_IS_DANCING_DATA_ID, true)],
        }));
        assert!(!dancing(&world, 181));
    }

    #[test]
    fn piglin_is_charging_crossbow_is_gated_to_the_regular_piglin() {
        // Vanilla Piglin.DATA_IS_CHARGING_CROSSBOW (BOOLEAN id 18): getArmPose returns CROSSBOW_CHARGE
        // (the pull-back draw) while true, suppressing CROSSBOW_HOLD. Only the regular piglin defines the
        // accessor, so the projection is type-gated.
        let charging = vec![protocol_bool_data(
            PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID,
            true,
        )];
        assert!(piglin_is_charging_crossbow(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            &charging
        ));
        assert!(!piglin_is_charging_crossbow(
            VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
            &charging
        ));
        assert!(!piglin_is_charging_crossbow(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            &[]
        ));
    }

    #[test]
    fn entity_model_instances_project_piglin_crossbow_charge() {
        // Vanilla Piglin.getArmPose → CROSSBOW_CHARGE while isChargingCrossbow() (BOOLEAN id 18): the
        // regular piglin draws its crossbow. Unlike CROSSBOW_HOLD this needs no held-item resolution (the
        // flag alone drives it), so it projects without an item runtime. Gated to the regular piglin: the
        // brute never charges (and slot 18 is not even defined on it).
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            190,
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            [1.0, 64.0, -4.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            191,
            VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
            [2.0, 64.0, -4.0],
        ));

        let charging = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .piglin_crossbow_charge
        };

        // No flag → not drawing.
        assert!(!charging(&world, 190));

        // Piglin.DATA_IS_CHARGING_CROSSBOW (id 18) projects the draw for the regular piglin — no item
        // runtime needed.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 190,
            values: vec![protocol_bool_data(
                PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID,
                true
            )],
        }));
        assert!(charging(&world, 190));

        // The same data id on a piglin brute does NOT project a draw.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 191,
            values: vec![protocol_bool_data(
                PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID,
                true
            )],
        }));
        assert!(!charging(&world, 191));
    }

    #[test]
    fn entity_model_instances_piglin_crossbow_hold_needs_a_resolved_held_item() {
        // The CROSSBOW_HOLD pose needs the held item resolved through the item registry to confirm a
        // charged crossbow; without an item runtime it can never level, so the projection defaults off.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            182,
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            [3.0, 64.0, -3.0],
        ));
        let crossbow_hold = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 182)
            .unwrap()
            .render_state
            .piglin_crossbow_hold;
        assert!(!crossbow_hold);
    }

    #[test]
    fn entity_model_instances_drowned_throw_trident_needs_a_resolved_held_item() {
        // The THROW_TRIDENT pose needs the held item resolved through the item registry to confirm a
        // trident; without an item runtime it can never raise the trident, so the projection defaults off
        // even for an aggressive drowned.
        const VANILLA_ENTITY_TYPE_DROWNED_ID: i32 = 38;
        const VANILLA_MOB_FLAGS_DATA_ID: u8 = 15;
        const MOB_FLAG_AGGRESSIVE: i8 = 4;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            230,
            VANILLA_ENTITY_TYPE_DROWNED_ID,
            [3.0, 64.0, -8.0],
        ));
        // Aggressive (so only the missing item runtime, not the flag, gates the pose off).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 230,
            values: vec![protocol_byte_data(
                VANILLA_MOB_FLAGS_DATA_ID,
                MOB_FLAG_AGGRESSIVE
            )],
        }));
        let throwing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 230)
            .unwrap()
            .render_state
            .drowned_throw_trident;
        assert!(!throwing);
    }

    #[test]
    fn entity_model_instance_projects_drowned_swim_amount_from_source() {
        // Vanilla `HumanoidMobRenderer.extractHumanoidRenderState` copies
        // `LivingEntity.getSwimAmount(partialTicks)` into `state.swimAmount`; `DrownedRenderer`
        // additionally needs `boundingBoxHeight` for its swim rotation pivot.
        const VANILLA_ENTITY_TYPE_DROWNED_ID: i32 = 38;

        let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
            "entity_id": 231,
            "entity_type_id": VANILLA_ENTITY_TYPE_DROWNED_ID,
            "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
            "y_rot": 0.0,
            "swim_amount": 0.27,
            "bounding_box_height": 1.95,
            "data_values": []
        }))
        .unwrap();

        let instance = entity_model_instance(
            source,
            &WorldStore::new(),
            None,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            instance.kind,
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: false,
            }
        );
        assert!((instance.render_state.swim_amount - 0.27).abs() < 1.0e-6);
        assert!((instance.render_state.bounding_box_height - 1.95).abs() < 1.0e-6);
    }

    #[test]
    fn entity_model_instance_projects_elytra_animation_state_from_source() {
        // Vanilla `HumanoidMobRenderer.extractHumanoidRenderState` copies
        // `LivingEntity.elytraAnimationState.getRotX/Y/Z(partialTicks)` into the
        // render state. The world layer owns those timers; native must preserve them
        // when building the renderer instance for `WingsLayer`.
        let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
            "entity_id": 232,
            "entity_type_id": VANILLA_ENTITY_TYPE_PLAYER_ID,
            "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
            "y_rot": 0.0,
            "elytra_rot_x": 0.42,
            "elytra_rot_y": 0.08,
            "elytra_rot_z": -0.64,
            "data_values": []
        }))
        .unwrap();

        let instance = entity_model_instance(
            source,
            &WorldStore::new(),
            None,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert!((instance.render_state.elytra_rot_x - 0.42).abs() < 1.0e-6);
        assert!((instance.render_state.elytra_rot_y - 0.08).abs() < 1.0e-6);
        assert!((instance.render_state.elytra_rot_z + 0.64).abs() < 1.0e-6);
    }

    #[test]
    fn entity_model_instances_project_witch_holding_item_without_runtime() {
        // `WitchRenderState.isHoldingItem` only needs a non-empty main hand, so it projects without the
        // item runtime. `isHoldingPotion` needs registry resolution (`Items.POTION`), so it stays false
        // without the runtime.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            244,
            VANILLA_ENTITY_TYPE_WITCH_ID,
            [3.0, 64.0, -4.0],
        ));
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: 244,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary {
                    item_id: Some(5),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        }));

        let state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 244)
            .unwrap()
            .render_state;
        assert!(state.witch_holding_item);
        assert!(!state.witch_holding_potion);
    }

    #[test]
    fn entity_model_instances_project_copper_golem_holding_item_from_either_hand() {
        // `CopperGolemModel.setupAnim` checks both rendered hand item states before clamping the arms into
        // the held-item pose, so an off-hand-only item is enough.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            245,
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
            [3.0, 64.0, -3.0],
        ));
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: 245,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::OffHand,
                item: ItemStackSummary {
                    item_id: Some(6),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        }));

        let state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 245)
            .unwrap()
            .render_state;
        assert!(state.copper_golem_holding_item);
    }

    #[test]
    fn entity_model_instances_custom_head_skull_needs_item_runtime() {
        // `LivingEntityRenderer.extractRenderState` needs the HEAD stack resolved as an AbstractSkullBlock.
        // Without the item registry runtime, bbb must not guess from the protocol id.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            246,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            [3.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: 246,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: ItemStackSummary {
                    item_id: Some(7),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        }));

        let state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 246)
            .unwrap()
            .render_state;
        assert_eq!(state.custom_head_skull, None);
    }

    #[test]
    fn entity_model_instances_stab_swing_needs_a_resolved_spear() {
        // The STAB swing type needs the held item resolved through the item registry to confirm a spear
        // (the STAB default lives on the item prototype, not the network patch); without an item runtime
        // it can never resolve, so the projection defaults off and the player keeps the WHACK swing. Gated
        // to the player kind, so a non-player entity never gets it either.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            240,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [1.0, 64.0, -9.0],
        ));
        let stab = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 240)
            .unwrap()
            .render_state
            .main_hand_swing_is_stab;
        assert!(!stab);
    }

    #[test]
    fn entity_model_instances_spyglass_pose_needs_a_resolved_spyglass() {
        // The SPYGLASS use-item pose needs the using-hand item resolved through the item registry to
        // confirm a spyglass; without an item runtime it can never resolve, so the projection defaults
        // off even for a player flagged as using an item.
        const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
        const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            250,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [1.0, 64.0, -10.0],
        ));
        // Flag the player as using an item (so only the missing runtime, not the flag, gates the pose off).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 250,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_IS_USING
            )],
        }));
        let spyglass = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 250)
            .unwrap()
            .render_state
            .player_using_spyglass;
        assert!(!spyglass);
    }

    #[test]
    fn entity_model_instances_horn_pose_needs_a_resolved_goat_horn() {
        // The TOOT_HORN use-item pose needs the using-hand item resolved through the item registry to
        // confirm a goat horn; without an item runtime it can never resolve, so the projection defaults
        // off even for a player flagged as using an item.
        const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
        const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            251,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [2.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 251,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_IS_USING
            )],
        }));
        let tooting = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 251)
            .unwrap()
            .render_state
            .player_tooting_horn;
        assert!(!tooting);
    }

    #[test]
    fn entity_model_instances_brush_pose_needs_a_resolved_brush() {
        // The BRUSH use-item pose needs the using-hand item resolved through the item registry to confirm
        // a brush; without an item runtime it can never resolve, so the projection defaults off even for a
        // player flagged as using an item.
        const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
        const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            252,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [3.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 252,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_IS_USING
            )],
        }));
        let brushing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 252)
            .unwrap()
            .render_state
            .player_brushing;
        assert!(!brushing);
    }

    #[test]
    fn entity_model_instances_block_pose_uses_shield_or_blocks_attacks_component() {
        // The BLOCK use-item pose needs a non-consumable `BLOCKS_ATTACKS` item. Vanilla shields resolve from
        // the item registry, while datapack/patch-granted blockers are visible in `added_type_ids` and work
        // without an item runtime.
        const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
        const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;
        const PLAIN_ITEM_ID: i32 = 730;

        let mut world = WorldStore::new();
        let equip = |entity_id: i32, added_type_ids: Vec<i32>| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary {
                    item_id: Some(PLAIN_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids,
                        ..DataComponentPatchSummary::default()
                    },
                },
            }],
        };
        let set_using = |id: i32| SetEntityData {
            id,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_IS_USING,
            )],
        };
        let blocking = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .player_blocking
        };

        world.apply_add_entity(protocol_add_entity(
            253,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [4.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_entity_data(set_using(253)));
        assert!(!blocking(&world, 253));

        world.apply_add_entity(protocol_add_entity(
            254,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [5.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_equipment(equip(254, vec![DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID])));
        assert!(world.apply_set_entity_data(set_using(254)));
        assert!(blocking(&world, 254));

        // `Item.getUseAnimation` checks CONSUMABLE before BLOCKS_ATTACKS, so a stack adding both routes to
        // EAT/DRINK rather than the BLOCK pose.
        world.apply_add_entity(protocol_add_entity(
            255,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [6.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_equipment(equip(
            255,
            vec![
                DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID
            ]
        )));
        assert!(world.apply_set_entity_data(set_using(255)));
        assert!(!blocking(&world, 255));
    }

    #[test]
    fn entity_model_instances_throw_trident_pose_needs_a_resolved_trident() {
        // The THROW_TRIDENT use-item pose needs the using-hand item resolved through the item registry to
        // confirm a trident; without an item runtime it can never resolve, so the projection defaults off
        // even for a player flagged as using an item.
        const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
        const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            254,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [5.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 254,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_IS_USING
            )],
        }));
        let throwing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 254)
            .unwrap()
            .render_state
            .player_throwing_trident;
        assert!(!throwing);
    }

    #[test]
    fn entity_model_instances_bow_draw_pose_needs_a_resolved_bow() {
        // The BOW_AND_ARROW use-item pose needs the using-hand item resolved through the item registry to
        // confirm a bow; without an item runtime it can never resolve, so the projection defaults off even
        // for a player flagged as using an item.
        const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
        const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            255,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [6.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 255,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_IS_USING
            )],
        }));
        let drawing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 255)
            .unwrap()
            .render_state
            .player_drawing_bow;
        assert!(!drawing);
    }

    #[test]
    fn entity_model_instances_crossbow_charge_pose_needs_a_resolved_crossbow() {
        // The CROSSBOW_CHARGE use-item pose needs the using-hand item resolved through the item registry to
        // confirm a crossbow; without an item runtime it can never resolve, so the projection defaults off
        // even for a player flagged as using an item.
        const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
        const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            256,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [7.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 256,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_IS_USING
            )],
        }));
        let charging = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 256)
            .unwrap()
            .render_state
            .player_charging_crossbow;
        assert!(!charging);
    }

    #[test]
    fn entity_model_instances_crossbow_hold_pose_needs_a_resolved_charged_crossbow() {
        // The CROSSBOW_HOLD pose needs the held item resolved through the item registry to confirm a CHARGED
        // crossbow; without an item runtime neither the main-hand nor off-hand projection can resolve, so
        // both default off even for a (non-swinging) player.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            257,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [8.0, 64.0, -10.0],
        ));
        let render_state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 257)
            .unwrap()
            .render_state;
        assert!(!render_state.player_crossbow_hold);
        assert!(!render_state.player_crossbow_hold_off_hand);
    }

    #[test]
    fn entity_model_instances_project_off_hand_bow_and_crossbow_player_poses() {
        // Vanilla `AvatarRenderer.getArmPose` selects the BOW/CROSSBOW use pose from the used hand, and
        // `CROSSBOW_HOLD` before the use-item branch from either hand. The renderer owns the exact arm math;
        // this test proves native projects the correct using-hand/off-hand booleans from resolved items.
        const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
        const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;
        const LIVING_ENTITY_FLAG_OFF_HAND: i8 = 2;
        const BOW_ID: i32 = 0;
        const CROSSBOW_ID: i32 = 1;

        let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
            "resource_ids": [
                "minecraft:bow",
                "minecraft:crossbow"
            ],
            "protocol_ids": {
                "minecraft:bow": BOW_ID,
                "minecraft:crossbow": CROSSBOW_ID
            }
        }))
        .unwrap();
        let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
            registry,
            bbb_pack::EquipmentAssetCatalog::default(),
        );
        let equip =
            |entity_id: i32, slot: EquipmentSlot, item_id: i32, charged: bool| SetEquipment {
                entity_id,
                slots: vec![EquipmentSlotUpdate {
                    slot,
                    item: ItemStackSummary {
                        item_id: Some(item_id),
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            charged_projectiles_items: if charged {
                                vec![bbb_protocol::packets::ItemStackTemplateSummary {
                                    item_id: BOW_ID,
                                    count: 1,
                                    component_patch: DataComponentPatchSummary::default(),
                                }]
                            } else {
                                Vec::new()
                            },
                            ..DataComponentPatchSummary::default()
                        },
                    },
                }],
            };
        let use_off_hand = |id: i32| SetEntityData {
            id,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_IS_USING | LIVING_ENTITY_FLAG_OFF_HAND,
            )],
        };
        let state = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
        };

        let mut world = WorldStore::new();

        world.apply_add_entity(protocol_add_entity(
            258,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [9.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_equipment(equip(258, EquipmentSlot::OffHand, BOW_ID, false)));
        assert!(world.apply_set_entity_data(use_off_hand(258)));
        let drawing_bow = state(&world, 258);
        assert!(drawing_bow.player_drawing_bow);
        assert!(drawing_bow.use_item_off_hand);
        assert!(!drawing_bow.player_off_hand_item_pose);

        world.apply_add_entity(protocol_add_entity(
            259,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [10.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_equipment(equip(259, EquipmentSlot::OffHand, CROSSBOW_ID, false)));
        assert!(world.apply_set_entity_data(use_off_hand(259)));
        let charging_crossbow = state(&world, 259);
        assert!(charging_crossbow.player_charging_crossbow);
        assert!(charging_crossbow.use_item_off_hand);
        assert!(!charging_crossbow.player_crossbow_hold_off_hand);

        world.apply_add_entity(protocol_add_entity(
            260,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [11.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_equipment(equip(260, EquipmentSlot::OffHand, CROSSBOW_ID, true)));
        let off_hand_hold = state(&world, 260);
        assert!(!off_hand_hold.player_crossbow_hold);
        assert!(off_hand_hold.player_crossbow_hold_off_hand);
        assert!(!off_hand_hold.player_off_hand_item_pose);

        world.apply_add_entity(protocol_add_entity(
            261,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [12.0, 64.0, -10.0],
        ));
        assert!(world.apply_set_equipment(equip(261, EquipmentSlot::MainHand, CROSSBOW_ID, true)));
        assert!(world.apply_set_equipment(equip(261, EquipmentSlot::OffHand, CROSSBOW_ID, true)));
        let main_hand_hold = state(&world, 261);
        assert!(main_hand_hold.player_crossbow_hold);
        assert!(!main_hand_hold.player_crossbow_hold_off_hand);
    }

    #[test]
    fn entity_model_instances_project_player_main_hand_item_pose() {
        // Vanilla `AvatarRenderer.getArmPose` fallback `ITEM`: a player holding a plain main-hand item, not
        // using it, lowers/halves the arm. Gated to the player kind, a non-empty main hand, and not using an
        // item; an empty hand, a using-item player, or a non-player mob never reaches the `ITEM` fallback.
        const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
        const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;
        // Any item id resolves the same way without a runtime — only "main hand non-empty" drives the gate.
        const PLAIN_ITEM_ID: i32 = 710;

        let plain_main_hand = |entity_id: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary {
                    item_id: Some(PLAIN_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        };
        let posing = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .player_main_hand_item_pose
        };

        let mut world = WorldStore::new();
        // A player holding a plain item in the main hand reaches the ITEM fallback pose.
        world.apply_add_entity(protocol_add_entity(
            260,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [1.0, 64.0, 8.0],
        ));
        assert!(world.apply_set_equipment(plain_main_hand(260)));
        assert!(posing(&world, 260));

        // The same player with an empty main hand has no item to pose.
        world.apply_add_entity(protocol_add_entity(
            261,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [2.0, 64.0, 8.0],
        ));
        assert!(!posing(&world, 261));

        // A player USING a NON-special main-hand item (here a plain item — `EAT`/`DRINK` or any tool) still
        // falls through to the `ITEM` fallback (vanilla `getArmPose` only special-cases bow/crossbow/trident/
        // shield/spyglass/horn/brush; everything else -> `ITEM`). A SPECIAL using item would suppress it, but
        // that needs the item runtime to resolve, so the no-runtime case treats any using item as non-special.
        world.apply_add_entity(protocol_add_entity(
            262,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [3.0, 64.0, 8.0],
        ));
        assert!(world.apply_set_equipment(plain_main_hand(262)));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 262,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_IS_USING
            )],
        }));
        assert!(posing(&world, 262));

        // A non-player mob holding an item never returns ITEM (`HumanoidMobRenderer.getArmPose`).
        world.apply_add_entity(protocol_add_entity(
            263,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            [4.0, 64.0, 8.0],
        ));
        assert!(world.apply_set_equipment(plain_main_hand(263)));
        assert!(!posing(&world, 263));
    }

    #[test]
    fn entity_model_instances_project_player_off_hand_item_pose() {
        // Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM`: a player holding a plain off-hand
        // item lowers/halves the OFF arm. Gated to the player kind + a non-empty off hand, suppressed only
        // when USING the off hand (its use poses win); using the MAIN hand leaves the off hand on `ITEM`.
        const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
        const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;
        const LIVING_ENTITY_FLAG_OFF_HAND: i8 = 2;
        const PLAIN_ITEM_ID: i32 = 720;

        let plain_off_hand = |entity_id: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::OffHand,
                item: ItemStackSummary {
                    item_id: Some(PLAIN_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        };
        let posing = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .player_off_hand_item_pose
        };

        let mut world = WorldStore::new();
        // A player holding a plain off-hand item reaches the off-hand ITEM fallback pose.
        world.apply_add_entity(protocol_add_entity(
            270,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [1.0, 64.0, 12.0],
        ));
        assert!(world.apply_set_equipment(plain_off_hand(270)));
        assert!(posing(&world, 270));

        // An empty off hand has no item to pose.
        world.apply_add_entity(protocol_add_entity(
            271,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [2.0, 64.0, 12.0],
        ));
        assert!(!posing(&world, 271));

        // USING a NON-special off-hand item (here a plain item — `EAT`/`DRINK`) still falls through to the
        // off-hand `ITEM` fallback (only a special off-hand use item would route to its own pose, which needs
        // the runtime to resolve; the no-runtime case treats any using item as non-special).
        world.apply_add_entity(protocol_add_entity(
            272,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [3.0, 64.0, 12.0],
        ));
        assert!(world.apply_set_equipment(plain_off_hand(272)));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 272,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_IS_USING | LIVING_ENTITY_FLAG_OFF_HAND
            )],
        }));
        assert!(posing(&world, 272));

        // USING the MAIN hand leaves the off hand on its ITEM fallback (the off hand is not the using hand).
        world.apply_add_entity(protocol_add_entity(
            273,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [4.0, 64.0, 12.0],
        ));
        assert!(world.apply_set_equipment(plain_off_hand(273)));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 273,
            values: vec![protocol_byte_data(
                VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                LIVING_ENTITY_FLAG_IS_USING
            )],
        }));
        assert!(posing(&world, 273));

        // A non-player mob never returns ITEM for the off hand either.
        world.apply_add_entity(protocol_add_entity(
            274,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            [5.0, 64.0, 12.0],
        ));
        assert!(world.apply_set_equipment(plain_off_hand(274)));
        assert!(!posing(&world, 274));
    }

    #[test]
    fn entity_model_instances_project_piglin_melee_attack_pose() {
        // Vanilla Piglin/PiglinBrute.getArmPose ATTACKING_WITH_MELEE_WEAPON: aggressive (Mob.isAggressive,
        // DATA_MOB_FLAGS_ID 15 bit 4) AND isHoldingMeleeWeapon (main-hand item with DataComponents.TOOL,
        // wire type 28). Gated to the regular piglin + brute (the zombified piglin uses its renderer
        // zombie-arm pose); the regular piglin is also suppressed while DANCING (higher priority).
        const VANILLA_MOB_FLAGS_DATA_ID: u8 = 15;
        const MOB_FLAG_AGGRESSIVE: i8 = 4;
        // Any item id resolves the same way — only the TOOL component drives the pose, not the item type.
        const MELEE_ITEM_ID: i32 = 700;

        // A main-hand stack carrying the `minecraft:tool` data component (wire type 28).
        let tool_main_hand = |entity_id: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary {
                    item_id: Some(MELEE_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![DATA_COMPONENT_TOOL_TYPE_ID],
                        ..DataComponentPatchSummary::default()
                    },
                },
            }],
        };
        let set_aggressive = |id: i32| SetEntityData {
            id,
            values: vec![protocol_byte_data(
                VANILLA_MOB_FLAGS_DATA_ID,
                MOB_FLAG_AGGRESSIVE,
            )],
        };
        let attacking = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .piglin_attacking_with_melee
        };

        let mut world = WorldStore::new();
        // Regular piglin (210), brute (211), zombified piglin (212): all aggressive, all holding a tool.
        for (id, type_id) in [
            (210, VANILLA_ENTITY_TYPE_PIGLIN_ID),
            (211, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID),
            (212, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID),
        ] {
            world.apply_add_entity(protocol_add_entity(id, type_id, [1.0, 64.0, -6.0]));
            assert!(world.apply_set_equipment(tool_main_hand(id)));
            assert!(world.apply_set_entity_data(set_aggressive(id)));
        }
        // The piglin and the brute raise/swing the melee weapon; the zombified piglin uses zombie arms.
        assert!(attacking(&world, 210));
        assert!(attacking(&world, 211));
        assert!(!attacking(&world, 212));

        // An aggressive piglin with an empty hand (no tool component) does not raise a weapon.
        world.apply_add_entity(protocol_add_entity(
            213,
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            [2.0, 64.0, -6.0],
        ));
        assert!(world.apply_set_entity_data(set_aggressive(213)));
        assert!(!attacking(&world, 213));

        // A non-aggressive piglin holding a tool does not attack.
        world.apply_add_entity(protocol_add_entity(
            214,
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            [3.0, 64.0, -6.0],
        ));
        assert!(world.apply_set_equipment(tool_main_hand(214)));
        assert!(!attacking(&world, 214));

        // A dancing piglin (higher priority) holding a tool while aggressive keeps DANCING, not the attack.
        world.apply_add_entity(protocol_add_entity(
            215,
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            [4.0, 64.0, -6.0],
        ));
        assert!(world.apply_set_equipment(tool_main_hand(215)));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 215,
            values: vec![
                protocol_byte_data(VANILLA_MOB_FLAGS_DATA_ID, MOB_FLAG_AGGRESSIVE),
                protocol_bool_data(PIGLIN_IS_DANCING_DATA_ID, true),
            ],
        }));
        assert!(!attacking(&world, 215));
    }

    #[test]
    fn entity_model_instances_project_piglin_admiring_a_loved_offhand_item() {
        // Vanilla Piglin.getArmPose ADMIRING_ITEM = PiglinAi.isLovedItem(getOffhandItem()) =
        // offhand.is(ItemTags.PIGLIN_LOVED). Gated to the regular piglin (the brute has no admire branch);
        // higher priority than ATTACKING/CROSSBOW (suppresses them), below DANCING.
        const VANILLA_MOB_FLAGS_DATA_ID: u8 = 15;
        const MOB_FLAG_AGGRESSIVE: i8 = 4;
        const LOVED_ITEM_ID: i32 = 800; // stand-in for a `minecraft:piglin_loved` item (e.g. gold_ingot).
        const PLAIN_ITEM_ID: i32 = 801; // not in the tag.
        const TOOL_ITEM_ID: i32 = 802; // a melee weapon (TOOL component) for the suppression check.

        let equip = |entity_id: i32, slot: EquipmentSlot, item_id: i32, tool: bool| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot,
                item: ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: if tool {
                            vec![DATA_COMPONENT_TOOL_TYPE_ID]
                        } else {
                            Vec::new()
                        },
                        ..DataComponentPatchSummary::default()
                    },
                },
            }],
        };
        let admiring = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .piglin_admiring
        };

        let mut world = WorldStore::new();
        // The `minecraft:piglin_loved` item tag arrives via UpdateTags (gold_ingot etc.).
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: PIGLIN_LOVED_ITEM_TAG.to_string(),
                    entries: vec![LOVED_ITEM_ID, TOOL_ITEM_ID],
                }],
            }],
        });

        // A regular piglin with a loved item in its OFFHAND admires it.
        world.apply_add_entity(protocol_add_entity(
            220,
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            [1.0, 64.0, -7.0],
        ));
        assert!(world.apply_set_equipment(equip(
            220,
            EquipmentSlot::OffHand,
            LOVED_ITEM_ID,
            false
        )));
        assert!(admiring(&world, 220));

        // A non-loved offhand item → no admiring.
        world.apply_add_entity(protocol_add_entity(
            221,
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            [2.0, 64.0, -7.0],
        ));
        assert!(world.apply_set_equipment(equip(
            221,
            EquipmentSlot::OffHand,
            PLAIN_ITEM_ID,
            false
        )));
        assert!(!admiring(&world, 221));

        // The piglin brute has no ADMIRING_ITEM branch, even with a loved offhand item.
        world.apply_add_entity(protocol_add_entity(
            222,
            VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
            [3.0, 64.0, -7.0],
        ));
        assert!(world.apply_set_equipment(equip(
            222,
            EquipmentSlot::OffHand,
            LOVED_ITEM_ID,
            false
        )));
        assert!(!admiring(&world, 222));

        // A dancing piglin (higher priority) does not admire.
        world.apply_add_entity(protocol_add_entity(
            223,
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            [4.0, 64.0, -7.0],
        ));
        assert!(world.apply_set_equipment(equip(
            223,
            EquipmentSlot::OffHand,
            LOVED_ITEM_ID,
            false
        )));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 223,
            values: vec![protocol_bool_data(PIGLIN_IS_DANCING_DATA_ID, true)],
        }));
        assert!(!admiring(&world, 223));

        // ADMIRING suppresses ATTACKING: an aggressive piglin with a tool main hand AND a loved offhand
        // admires (vanilla precedence ADMIRING > ATTACKING), so it does not swing.
        world.apply_add_entity(protocol_add_entity(
            224,
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            [5.0, 64.0, -7.0],
        ));
        assert!(world.apply_set_equipment(equip(224, EquipmentSlot::MainHand, TOOL_ITEM_ID, true)));
        assert!(world.apply_set_equipment(equip(
            224,
            EquipmentSlot::OffHand,
            LOVED_ITEM_ID,
            false
        )));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 224,
            values: vec![protocol_byte_data(
                VANILLA_MOB_FLAGS_DATA_ID,
                MOB_FLAG_AGGRESSIVE
            )],
        }));
        let attacking_224 = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 224)
            .unwrap()
            .render_state
            .piglin_attacking_with_melee;
        assert!(
            admiring(&world, 224),
            "the loved offhand item makes it admire"
        );
        assert!(
            !attacking_224,
            "admiring (higher priority) suppresses the melee swing"
        );
    }

    #[test]
    fn entity_model_instances_project_panda_unhappy_and_sneezing() {
        // Vanilla PandaRenderState: isUnhappy = getUnhappyCounter() > 0 (INT id 18); isSneezing =
        // isSneezing() (DATA_ID_FLAGS byte id 23, bit 0x02); sneezeTime = getSneezeCounter() (INT id 19);
        // isEating = EAT_COUNTER > 0 (INT id 20); isSitting = DATA_ID_FLAGS bit 0x08; isScared =
        // worried-gene panda in a thundering level.
        let mut world = WorldStore::new();
        world.apply_login(&protocol_play_login(1));
        world.apply_add_entity(protocol_add_entity(
            190,
            VANILLA_ENTITY_TYPE_PANDA_ID,
            [1.0, 64.0, -4.0],
        ));

        let panda = |world: &WorldStore, partial_tick: f32| {
            entity_model_instances_from_world_at_partial_tick(world, None, partial_tick)
                .into_iter()
                .find(|instance| instance.entity_id == 190)
                .unwrap()
                .render_state
        };

        // No data → content panda.
        let rest = panda(&world, 0.0);
        assert!(!rest.panda_unhappy);
        assert!(!rest.panda_sneezing);
        assert_eq!(rest.panda_sneeze_time, 0);
        assert!(!rest.panda_eating);
        assert!(!rest.panda_sitting);
        assert!(!rest.panda_scared);
        assert_eq!(rest.panda_sit_amount, 0.0);
        assert_eq!(rest.panda_lie_on_back_amount, 0.0);
        assert_eq!(rest.panda_roll_amount, 0.0);
        assert_eq!(rest.panda_roll_time, 0.0);

        // UNHAPPY_COUNTER > 0 projects the unhappy shake; the sneeze flag + counter project the dip; the
        // sitting bit + EAT_COUNTER project the held-item layer state. The same flags byte also feeds
        // the world-side sit/on-back/roll animation amounts that native copies into render state.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 190,
            values: vec![
                protocol_int_data(PANDA_UNHAPPY_COUNTER_DATA_ID, 12),
                protocol_int_data(PANDA_SNEEZE_COUNTER_DATA_ID, 9),
                protocol_int_data(PANDA_EAT_COUNTER_DATA_ID, 4),
                protocol_byte_data(
                    PANDA_FLAGS_DATA_ID,
                    PANDA_SNEEZING_FLAG
                        | PANDA_ROLLING_FLAG
                        | PANDA_SITTING_FLAG
                        | PANDA_ON_BACK_FLAG,
                ),
            ],
        }));
        world.advance_entity_client_animations(1);
        let active = panda(&world, 0.5);
        assert!(active.panda_unhappy);
        assert!(active.panda_sneezing);
        assert_eq!(active.panda_sneeze_time, 9);
        assert!(active.panda_eating);
        assert!(active.panda_sitting);
        assert!(!active.panda_scared);
        assert!((active.panda_sit_amount - 0.075).abs() < 1.0e-6);
        assert!((active.panda_lie_on_back_amount - 0.075).abs() < 1.0e-6);
        assert!((active.panda_roll_amount - 0.075).abs() < 1.0e-6);
        assert!((active.panda_roll_time - 1.5).abs() < 1.0e-6);

        world.apply_game_event(bbb_protocol::packets::GameEvent {
            event_id: 2,
            param: 0.0,
        });
        world.apply_game_event(bbb_protocol::packets::GameEvent {
            event_id: 8,
            param: 0.9,
        });
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 190,
            values: vec![
                protocol_byte_data(PANDA_MAIN_GENE_DATA_ID, 2),
                protocol_byte_data(PANDA_HIDDEN_GENE_DATA_ID, 0),
            ],
        }));
        assert!(!panda(&world, 0.0).panda_scared);
        world.apply_game_event(bbb_protocol::packets::GameEvent {
            event_id: 8,
            param: 1.0,
        });
        assert!(panda(&world, 0.0).panda_scared);

        // A zero unhappy counter is content again; clearing the flag stops the sneeze even with a counter.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 190,
            values: vec![
                protocol_int_data(PANDA_UNHAPPY_COUNTER_DATA_ID, 0),
                protocol_int_data(PANDA_EAT_COUNTER_DATA_ID, 0),
                protocol_byte_data(PANDA_FLAGS_DATA_ID, 0),
            ],
        }));
        let calmed = panda(&world, 0.0);
        assert!(!calmed.panda_unhappy);
        assert!(!calmed.panda_sneezing);
        assert!(!calmed.panda_eating);
        assert!(!calmed.panda_sitting);
        assert!(calmed.panda_scared);
    }

    #[test]
    fn entity_model_instances_project_villager_unhappy() {
        const ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID: u8 = 18;
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            191,
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            [1.0, 64.0, -4.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            192,
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            [2.0, 64.0, -4.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            193,
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            [3.0, 64.0, -4.0],
        ));

        let villager_unhappy = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .villager_unhappy
        };

        assert!(!villager_unhappy(&world, 191));
        assert!(!villager_unhappy(&world, 192));

        for id in [191, 192, 193] {
            assert!(world.apply_set_entity_data(SetEntityData {
                id,
                values: vec![protocol_int_data(
                    ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID,
                    12,
                )],
            }));
        }
        assert!(villager_unhappy(&world, 191));
        assert!(villager_unhappy(&world, 192));
        assert!(
            !villager_unhappy(&world, 193),
            "zombie villagers do not use VillagerRenderState"
        );

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 191,
            values: vec![protocol_int_data(
                ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID,
                0
            )],
        }));
        assert!(!villager_unhappy(&world, 191));
    }

    #[test]
    fn entity_model_instances_project_goat_ramming_head_tilt() {
        use std::f32::consts::PI;

        // Vanilla Goat.getRammingXHeadRot() = lowerHeadTick/20 · (baby ? 52.5 : 30)° · π/180, driven by
        // the world-projected ram counter (entity events 58/59). The baby max head pitch is steeper.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            200,
            VANILLA_ENTITY_TYPE_GOAT_ID,
            [1.0, 64.0, -5.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            201,
            VANILLA_ENTITY_TYPE_GOAT_ID,
            [2.0, 64.0, -5.0],
        ));
        // Make 201 a baby goat (AgeableMob.DATA_BABY_ID id 16).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 201,
            values: vec![protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)],
        }));

        let ramming = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .goat_ramming_x_head_rot
        };

        // At rest both goats project no head tilt.
        assert_eq!(ramming(&world, 200), 0.0);
        assert_eq!(ramming(&world, 201), 0.0);

        // Event 58 begins the ram; after 20 ticks the counter saturates at 20 (full tilt).
        for id in [200, 201] {
            assert!(world.apply_entity_event(EntityEvent {
                entity_id: id,
                event_id: 58,
            }));
        }
        world.advance_entity_client_animations(20);
        // Adult: 30° → π/6; baby: 52.5°.
        assert!((ramming(&world, 200) - PI / 6.0).abs() < 1.0e-5);
        assert!((ramming(&world, 201) - 52.5_f32.to_radians()).abs() < 1.0e-5);
        assert!(
            ramming(&world, 201) > ramming(&world, 200),
            "the baby tilts its head further"
        );
    }

    #[test]
    fn entity_model_instances_project_turtle_laying_egg() {
        // Vanilla Turtle.LAYING_EGG (BOOLEAN data id 19) and TurtleRenderer.extractRenderState:
        // state.isLayingEgg = entity.isLayingEgg(). Unlike hasEgg, this is NOT baby-gated (the
        // egg-laying amplitude lives in the shared TurtleModel).
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            150,
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            [1.0, 64.0, -2.0],
        ));
        // A baby turtle (lays too), plus a non-turtle (bat) used to prove the type gating.
        world.apply_add_entity(protocol_add_entity(
            151,
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            [2.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            152,
            VANILLA_ENTITY_TYPE_BAT_ID,
            [3.0, 64.0, -2.0],
        ));

        let laying = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .turtle_laying_egg
        };

        // A turtle that is not laying projects turtle_laying_egg = false.
        assert!(!laying(&world, 150));

        // Setting Turtle.LAYING_EGG (data id 19) projects through.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 150,
            values: vec![protocol_bool_data(TURTLE_LAYING_EGG_DATA_ID, true)],
        }));
        assert!(laying(&world, 150));

        // A baby turtle DOES lay (no baby exclusion, unlike hasEgg).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 151,
            values: vec![
                protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
                protocol_bool_data(TURTLE_LAYING_EGG_DATA_ID, true),
            ],
        }));
        assert!(laying(&world, 151));

        // The same flag on a non-turtle (bat) does NOT project turtle_laying_egg.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 152,
            values: vec![protocol_bool_data(TURTLE_LAYING_EGG_DATA_ID, true)],
        }));
        assert!(!laying(&world, 152));
    }

    #[test]
    fn entity_model_instances_project_end_crystal_shows_bottom() {
        // Vanilla EndCrystal.DATA_SHOW_BOTTOM (BOOLEAN id 9, default true) and
        // EndCrystalRenderState.showsBottom = entity.showsBottom().
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            160,
            VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
            [1.0, 64.0, -2.0],
        ));
        // A non-crystal: the field defaults true (it is unused and never reads id 9).
        world.apply_add_entity(protocol_add_entity(
            161,
            VANILLA_ENTITY_TYPE_BAT_ID,
            [2.0, 64.0, -2.0],
        ));

        let shows_bottom = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .end_crystal_shows_bottom
        };

        // No synced value → the vanilla default `true` (the bottom slab is shown).
        assert!(shows_bottom(&world, 160));

        // Clearing DATA_SHOW_BOTTOM hides the base slab.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 160,
            values: vec![protocol_bool_data(END_CRYSTAL_SHOW_BOTTOM_DATA_ID, false)],
        }));
        assert!(!shows_bottom(&world, 160));

        // Re-setting it shows the base again.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 160,
            values: vec![protocol_bool_data(END_CRYSTAL_SHOW_BOTTOM_DATA_ID, true)],
        }));
        assert!(shows_bottom(&world, 160));

        // A non-crystal keeps the default `true` (unused).
        assert!(shows_bottom(&world, 161));
    }

    #[test]
    fn entity_model_instances_project_end_crystal_beam_target() {
        // Vanilla EndCrystal.DATA_BEAM_TARGET (OPTIONAL_BLOCK_POS id 8) projects as
        // EndCrystalRenderState.beamOffset = target block center - crystal position.
        const END_CRYSTAL_BEAM_TARGET_DATA_ID: u8 = 8;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            162,
            VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
            [10.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            163,
            VANILLA_ENTITY_TYPE_BAT_ID,
            [10.0, 64.0, -3.0],
        ));

        let beam = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .end_crystal_beam
        };

        assert!(beam(&world, 162).is_none());

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 162,
            values: vec![protocol_optional_block_pos_data(
                END_CRYSTAL_BEAM_TARGET_DATA_ID,
                Some(bbb_protocol::packets::BlockPos {
                    x: 14,
                    y: 67,
                    z: -10,
                }),
            )],
        }));
        assert_eq!(beam(&world, 162).unwrap().beam_offset, [4.5, 3.5, -6.5]);

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 162,
            values: vec![protocol_optional_block_pos_data(
                END_CRYSTAL_BEAM_TARGET_DATA_ID,
                None,
            )],
        }));
        assert!(beam(&world, 162).is_none());

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 163,
            values: vec![protocol_optional_block_pos_data(
                END_CRYSTAL_BEAM_TARGET_DATA_ID,
                Some(bbb_protocol::packets::BlockPos {
                    x: 14,
                    y: 67,
                    z: -10,
                }),
            )],
        }));
        assert!(beam(&world, 163).is_none());
    }

    #[test]
    fn entity_model_instance_projects_ender_dragon_healing_beam_from_source() {
        // Vanilla `EnderDragonRenderer.extractRenderState` stores nullable `beamOffset`; native must
        // preserve the world-projected offset so the renderer can submit `endCrystalBeam` after
        // body+eyes.
        let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
            "entity_id": 164,
            "entity_type_id": VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
            "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
            "y_rot": 0.0,
            "ender_dragon_beam": { "beam_offset": [6.0, -0.1, 8.0] },
            "data_values": []
        }))
        .unwrap();

        let instance = entity_model_instance(
            source,
            &WorldStore::new(),
            None,
            0,
            0.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(instance.kind, EntityModelKind::EnderDragon);
        assert_eq!(
            instance
                .render_state
                .ender_dragon_beam
                .expect("dragon beam source maps to render state")
                .beam_offset,
            [6.0, -0.1, 8.0]
        );
    }

    #[test]
    fn entity_model_instances_project_bee_stinger() {
        // Vanilla Bee.DATA_FLAGS_ID (18, BYTE) and the has-stung bit (4).
        const VANILLA_BEE_FLAGS_DATA_ID: u8 = 18;
        const BEE_FLAG_HAS_STUNG: i8 = 4;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            96,
            VANILLA_ENTITY_TYPE_BEE_ID,
            [1.0, 64.0, -2.0],
        ));

        let has_stinger = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .bee_has_stinger
        };

        // A bee that has not stung keeps its stinger.
        assert!(has_stinger(&world, 96));

        // Setting Bee.hasStung (DATA_FLAGS_ID & 4) hides the stinger cube.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 96,
            values: vec![protocol_byte_data(
                VANILLA_BEE_FLAGS_DATA_ID,
                BEE_FLAG_HAS_STUNG
            )],
        }));
        assert!(!has_stinger(&world, 96));
    }

    #[test]
    fn entity_model_instances_project_bee_angry_from_anger_end_time() {
        // Vanilla Bee.DATA_ANGER_END_TIME (19, LONG): isAngry = endTime > 0 && endTime - gameTime
        // > 0. The world has no time set here, so the game time defaults to 0.
        const VANILLA_BEE_ANGER_END_TIME_DATA_ID: u8 = 19;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            97,
            VANILLA_ENTITY_TYPE_BEE_ID,
            [1.0, 64.0, -2.0],
        ));

        let angry = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .bee_angry
        };

        // A bee with no anger end time (default -1) is calm.
        assert!(!angry(&world, 97));

        // An anger end time in the future (game time 0) makes the bee angry.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 97,
            values: vec![protocol_long_data(VANILLA_BEE_ANGER_END_TIME_DATA_ID, 200)],
        }));
        assert!(angry(&world, 97));

        // A zero/past end time is calm again.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 97,
            values: vec![protocol_long_data(VANILLA_BEE_ANGER_END_TIME_DATA_ID, 0)],
        }));
        assert!(!angry(&world, 97));
    }

    #[test]
    fn entity_model_instances_project_camel_sit_then_sit_pose_from_pose_change_tick() {
        // Vanilla Camel.LAST_POSE_CHANGE_TICK (20, LONG): the SIGN encodes sitting (< 0) and the
        // magnitude is the change tick. getPoseTime = gameTime - |LAST_POSE_CHANGE_TICK|. A camel that
        // sat at game tick 100 (so LAST_POSE_CHANGE_TICK = -100) plays CAMEL_SIT for getPoseTime < 40,
        // then CAMEL_SIT_POSE (which starts when the 40-tick sit-down window ends).
        const VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID: u8 = 20;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            150,
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            [1.0, 64.0, -2.0],
        ));
        // Sat down at game tick 100 (negative magnitude → sitting).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 150,
            values: vec![protocol_long_data(
                VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID,
                -100,
            )],
        }));

        let sit_seconds = |world: &WorldStore, id: i32| {
            let state = entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state;
            (
                state.camel_sit_seconds,
                state.camel_sit_pose_seconds,
                state.camel_standup_seconds,
            )
        };

        // getPoseTime = 120 - 100 = 20 (< 40): inside the sit-down window, so CAMEL_SIT is active at
        // 20/20 = 1.0 s, and sit-pose / standup are the stopped sentinel.
        world.apply_world_time(PlayTime {
            game_time: 120,
            clock_updates: Vec::new(),
        });
        assert_eq!(sit_seconds(&world, 150), (1.0, -1.0, -1.0));

        // getPoseTime = 160 - 100 = 60 (>= 40): past the sit-down window, so CAMEL_SIT_POSE takes over
        // at (60 - 40)/20 = 1.0 s, and sit / standup are stopped.
        world.apply_world_time(PlayTime {
            game_time: 160,
            clock_updates: Vec::new(),
        });
        assert_eq!(sit_seconds(&world, 150), (-1.0, 1.0, -1.0));

        // Standing back up at game tick 200 (positive magnitude → not sitting): getPoseTime = 210 -
        // 200 = 10 (< 52 STANDUP window, >= 0), so CAMEL_STANDUP is active at 10/20 = 0.5 s.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 150,
            values: vec![protocol_long_data(
                VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID,
                200,
            )],
        }));
        world.apply_world_time(PlayTime {
            game_time: 210,
            clock_updates: Vec::new(),
        });
        assert_eq!(sit_seconds(&world, 150), (-1.0, -1.0, 0.5));

        // Long after standing up (getPoseTime = 300 - 200 = 100 >= 52): no transition, all stopped.
        world.apply_world_time(PlayTime {
            game_time: 300,
            clock_updates: Vec::new(),
        });
        assert_eq!(sit_seconds(&world, 150), (-1.0, -1.0, -1.0));
    }

    #[test]
    fn entity_model_instances_project_player_crouch_pose() {
        // Vanilla Entity.isCrouching (Pose.CROUCHING, ordinal 5, POSE serializer 20).
        const ENTITY_DATA_POSE_ID: u8 = 6;
        const POSE_STANDING: i32 = 0;
        const POSE_CROUCHING: i32 = 5;
        const POSE_SERIALIZER_ID: i32 = 20;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            98,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [1.0, 64.0, -2.0],
        ));

        let crouching = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .is_crouching
        };
        let set_pose = |world: &mut WorldStore, id: i32, pose: i32| {
            world.apply_set_entity_data(SetEntityData {
                id,
                values: vec![EntityDataValue {
                    data_id: ENTITY_DATA_POSE_ID,
                    serializer_id: POSE_SERIALIZER_ID,
                    value: EntityDataValueKind::Pose(pose),
                }],
            })
        };

        // A standing player is not crouching.
        assert!(!crouching(&world, 98));
        // Pose.CROUCHING projects the sneaking pose; standing again clears it.
        assert!(set_pose(&mut world, 98, POSE_CROUCHING));
        assert!(crouching(&world, 98));
        assert!(set_pose(&mut world, 98, POSE_STANDING));
        assert!(!crouching(&world, 98));
    }

    #[test]
    fn entity_model_instances_project_upside_down_dinnerbone() {
        let mut world = WorldStore::new();
        // A sheep with a head turn (body 30, head 100, pitch -20): net head yaw 70,
        // head pitch -20 while upright.
        world.apply_add_entity(protocol_add_entity_with_rotation(
            88,
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            [1.0, 64.0, -2.0],
            30.0,
            -20.0,
            100.0,
        ));
        // A non-living entity (boat) is never flipped by LivingEntityRenderer.
        world.apply_add_entity(protocol_add_entity(
            89,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            [3.0, 64.0, -2.0],
        ));

        let render_state = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
        };

        // A normally-named living entity is upright; its head look is unnegated.
        let upright = render_state(&world, 88);
        assert_eq!(upright.upside_down_height, None);
        assert_eq!(upright.head_yaw, 70.0);
        assert_eq!(upright.head_pitch, -20.0);

        // Vanilla LivingEntityRenderer.isUpsideDownName: the "Dinnerbone" name tag
        // flips the model; extractRenderState then negates the head yaw and pitch.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 88,
            values: vec![protocol_optional_component_data(
                ENTITY_CUSTOM_NAME_DATA_ID,
                Some("Dinnerbone"),
            )],
        }));
        let flipped = render_state(&world, 88);
        assert!(flipped
            .upside_down_height
            .is_some_and(|height| height > 0.0));
        assert_eq!(flipped.head_yaw, -70.0);
        assert_eq!(flipped.head_pitch, 20.0);

        // "Grumm" flips too; an unrelated name does not.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 88,
            values: vec![protocol_optional_component_data(
                ENTITY_CUSTOM_NAME_DATA_ID,
                Some("Grumm"),
            )],
        }));
        assert!(render_state(&world, 88).upside_down_height.is_some());
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 88,
            values: vec![protocol_optional_component_data(
                ENTITY_CUSTOM_NAME_DATA_ID,
                Some("Steve"),
            )],
        }));
        assert_eq!(render_state(&world, 88).upside_down_height, None);

        // A non-living entity named Dinnerbone is still never flipped.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 89,
            values: vec![protocol_optional_component_data(
                ENTITY_CUSTOM_NAME_DATA_ID,
                Some("Dinnerbone"),
            )],
        }));
        assert_eq!(render_state(&world, 89).upside_down_height, None);
    }

    #[test]
    fn entity_model_instances_project_sleeping_pose() {
        const ENTITY_DATA_POSE_ID: u8 = 6;
        const POSE_SLEEPING: i32 = 2;
        const POSE_SERIALIZER_ID: i32 = 20;

        let mut world = WorldStore::new();
        // A sheep facing body yaw 45 with no bed resolved (no chunk loaded).
        world.apply_add_entity(protocol_add_entity_with_rotation(
            93,
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            [1.0, 64.0, -2.0],
            45.0,
            0.0,
            45.0,
        ));

        let sleeping = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .sleeping
        };

        // An awake entity is not laid down.
        assert_eq!(sleeping(&world, 93), None);

        // Vanilla Pose.SLEEPING with no resolvable bed falls back to the body yaw and
        // no head offset (setupRotations `angle = bodyRot`).
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 93,
            values: vec![EntityDataValue {
                data_id: ENTITY_DATA_POSE_ID,
                serializer_id: POSE_SERIALIZER_ID,
                value: EntityDataValueKind::Pose(POSE_SLEEPING),
            }],
        }));
        assert_eq!(
            sleeping(&world, 93),
            Some(SleepingPose {
                yaw_angle: 45.0,
                bed_offset: [0.0, 0.0],
            })
        );
    }

    #[test]
    fn entity_model_instances_project_scale_attribute() {
        const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            97,
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            [1.0, 64.0, -2.0],
        ));

        let scale = |world: &WorldStore| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == 97)
                .unwrap()
                .render_state
                .scale
        };

        // Default size projects scale 1.0.
        assert_eq!(scale(&world), 1.0);

        // Vanilla getScale() (the SCALE attribute) flows through to the render state.
        assert!(world.apply_update_attributes(UpdateAttributes {
            entity_id: 97,
            attributes: vec![AttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
                base: 1.25,
                modifiers: Vec::new(),
            }],
        }));
        assert_eq!(scale(&world), 1.25);
    }

    #[test]
    fn entity_model_instances_project_walk_animation() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            98,
            VANILLA_ENTITY_TYPE_COW_ID,
            [0.0, 64.0, 0.0],
        ));

        let walk = |world: &WorldStore| -> (bool, f32, f32, f32) {
            let state = entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
                .into_iter()
                .find(|instance| instance.entity_id == 98)
                .unwrap()
                .render_state;
            (
                state.is_riding,
                state.walk_animation_pos,
                state.walk_animation_speed,
                state.worn_head_animation_pos,
            )
        };
        let sync = |world: &mut WorldStore, id: i32, x: f64| {
            assert!(world.apply_entity_position_sync(EntityPositionSync {
                id,
                position: Vec3d { x, y: 64.0, z: 0.0 },
                delta_movement: Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: true,
            }));
        };

        // A standing cow projects no limb swing.
        world.advance_entity_client_animations(1);
        assert_eq!(walk(&world), (false, 0.0, 0.0, 0.0));

        // After one 0.5-block step, the WalkAnimationState reaches speed = 0.4 and
        // position = 0.4 (targetSpeed = min(0.5 * 4, 1) = 1.0), and both flow through
        // EntityModelSourceState to the renderer EntityRenderState. Vanilla also reuses the same
        // position for LivingEntityRenderState.wornHeadAnimationPos while not riding a living entity.
        sync(&mut world, 98, 0.5);
        world.advance_entity_client_animations(1);
        let (is_riding, pos, speed, worn_head_pos) = walk(&world);
        assert!(!is_riding);
        assert!((speed - 0.4).abs() < 1e-5, "walk speed: {speed}");
        assert!((pos - 0.4).abs() < 1e-5, "walk position: {pos}");
        assert!(
            (worn_head_pos - 0.4).abs() < 1e-5,
            "worn head animation position: {worn_head_pos}"
        );

        // While riding a living entity, vanilla keeps the passenger's limb swing stopped but drives
        // worn skull animation from the vehicle walk animation position.
        world.apply_add_entity(protocol_add_entity(
            99,
            VANILLA_ENTITY_TYPE_COW_ID,
            [0.0, 64.0, 0.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            100,
            VANILLA_ENTITY_TYPE_COW_ID,
            [0.0, 64.0, 0.0],
        ));
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 99,
            passenger_ids: vec![100],
        }));
        sync(&mut world, 99, 0.0);
        world.advance_entity_client_animations(1);
        sync(&mut world, 99, 0.5);
        world.advance_entity_client_animations(1);
        let passenger = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 100)
            .unwrap()
            .render_state;
        assert!(passenger.is_riding);
        assert_eq!(
            (passenger.walk_animation_pos, passenger.walk_animation_speed),
            (0.0, 0.0)
        );
        assert!(
            (passenger.worn_head_animation_pos - 0.4).abs() < 1e-5,
            "passenger worn head animation position: {}",
            passenger.worn_head_animation_pos
        );
    }

    #[test]
    fn entity_light_coords_packs_vanilla_block_and_sky_with_on_fire_override() {
        use bbb_world::TerrainLight;

        // A generic (non-special) entity type — no per-renderer block-light override.
        let generic = VANILLA_ENTITY_TYPE_CHICKEN_ID;
        // Daylight surface (block 0, sky 15) -> LightCoordsUtil.pack(0, 15).
        assert_eq!(
            entity_light_coords(generic, &[], TerrainLight { sky: 15, block: 0 }),
            15 << 20
        );
        // Full-bright fallback (block 15, sky 15) -> LightCoordsUtil.FULL_BRIGHT.
        assert_eq!(
            entity_light_coords(generic, &[], TerrainLight { sky: 15, block: 15 }),
            15_728_880
        );
        // Torch-lit cave (block 14, sky 0) -> pack(14, 0).
        assert_eq!(
            entity_light_coords(generic, &[], TerrainLight { sky: 0, block: 14 }),
            14 << 4
        );
        // EntityRenderer.getBlockLightLevel forces block light to 15 on fire,
        // leaving sky light untouched.
        let on_fire = vec![protocol_byte_data(
            ENTITY_SHARED_FLAGS_DATA_ID,
            ENTITY_SHARED_FLAG_ON_FIRE,
        )];
        assert_eq!(
            entity_light_coords(generic, &on_fire, TerrainLight { sky: 4, block: 0 }),
            (15 << 4) | (4 << 20)
        );
    }

    #[test]
    fn entity_light_coords_applies_vanilla_per_renderer_block_light_overrides() {
        use bbb_world::TerrainLight;

        let dark = TerrainLight { sky: 0, block: 0 };
        // Vanilla `BlazeRenderer`/`MagmaCubeRenderer.getBlockLightLevel` = 15 unconditionally: even in pitch
        // dark, the block light packs to 15 (sky stays 0).
        assert_eq!(
            entity_light_coords(VANILLA_ENTITY_TYPE_BLAZE_ID, &[], dark),
            15 << 4
        );
        assert_eq!(
            entity_light_coords(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, &[], dark),
            15 << 4
        );
        // The full set also covers the wither, wither skull, dragon fireball, shulker bullet, allay, and vex.
        for full_bright in [
            VANILLA_ENTITY_TYPE_WITHER_ID,
            VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
            VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
            VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
            VANILLA_ENTITY_TYPE_ALLAY_ID,
            VANILLA_ENTITY_TYPE_VEX_ID,
        ] {
            assert_eq!(entity_light_coords(full_bright, &[], dark), 15 << 4);
        }

        // Vanilla `ItemFrameRenderer`: normal item frames use the sampled light; glow item frames clamp
        // only the block component to at least `GLOW_FRAME_BRIGHTNESS = 5`.
        assert_eq!(
            entity_light_coords(VANILLA_ENTITY_TYPE_ITEM_FRAME_ID, &[], dark),
            0
        );
        assert_eq!(
            entity_light_coords(VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, &[], dark),
            5 << 4
        );
        assert_eq!(
            entity_light_coords(
                VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
                &[],
                TerrainLight { sky: 3, block: 7 }
            ),
            (7 << 4) | (3 << 20)
        );

        // Vanilla `GlowSquidRenderer.getBlockLightLevel` = max(super, (int)clampedLerp(1 - darkTicks/10, 0,
        // 15)). Undamaged (no DARK_TICKS data, so 0) -> fully bright 15.
        assert_eq!(
            entity_light_coords(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, &[], dark),
            15 << 4
        );
        let dark_ticks = |ticks: i32| vec![protocol_int_data(GLOW_SQUID_DARK_TICKS_DATA_ID, ticks)];
        // Just hurt (darkTicks 100): factor = 1 - 10 = -9 < 0 -> min 0, so the boost is 0 and the squid is as
        // dark as its surroundings (block 0 here).
        assert_eq!(
            entity_light_coords(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, &dark_ticks(100), dark),
            0
        );
        // Mid-ramp (darkTicks 5): factor = 0.5 -> (int) lerp(0.5, 0, 15) = (int) 7.5 = 7.
        assert_eq!(
            entity_light_coords(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, &dark_ticks(5), dark),
            7 << 4
        );
        // darkTicks 10: factor = 0 -> 0 boost.
        assert_eq!(
            entity_light_coords(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, &dark_ticks(10), dark),
            0
        );
        // The boost only ever RAISES the block light: a torch-lit (block 12) hurt glow squid keeps 12.
        assert_eq!(
            entity_light_coords(
                VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
                &dark_ticks(100),
                TerrainLight { sky: 0, block: 12 }
            ),
            12 << 4
        );
    }

    #[test]
    fn entity_model_instances_project_full_bright_light_without_chunk_data() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            90,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, -2.0],
        ));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert_eq!(
            instances[0].render_state.light_coords,
            bbb_renderer::ENTITY_FULL_BRIGHT_LIGHT_COORDS
        );
    }

    #[test]
    fn entity_model_instances_project_hurt_red_overlay_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            91,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, -2.0],
        ));

        let calm = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert!(!calm[0].render_state.has_red_overlay);

        assert!(
            world.apply_hurt_animation(bbb_protocol::packets::HurtAnimation { id: 91, yaw: 0.0 })
        );
        let hurt = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert!(hurt[0].render_state.has_red_overlay);
    }

    #[test]
    fn creeper_white_overlay_progress_matches_vanilla_strobe() {
        // (int)(step * 10) even -> 0.0, odd -> clamp(step, 0.5, 1.0).
        assert_eq!(creeper_white_overlay_progress(0.0), 0.0);
        assert_eq!(creeper_white_overlay_progress(0.15), 0.5); // bucket 1 (odd), clamped up
        assert_eq!(creeper_white_overlay_progress(0.55), 0.55); // bucket 5 (odd)
        assert_eq!(creeper_white_overlay_progress(0.6), 0.0); // bucket 6 (even)
    }

    #[test]
    fn entity_model_instances_project_creeper_white_overlay_from_world() {
        const VANILLA_ENTITY_TYPE_CREEPER_ID: i32 = 32;
        const CREEPER_SWELL_DIR_DATA_ID: u8 = 16;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            92,
            VANILLA_ENTITY_TYPE_CREEPER_ID,
            [1.0, 64.0, -2.0],
        ));
        let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert_eq!(resting[0].render_state.white_overlay_progress, 0.0);

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 92,
            values: vec![EntityDataValue {
                data_id: CREEPER_SWELL_DIR_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(1),
            }],
        }));
        world.advance_entity_client_animations(5);

        // swell = 5, getSwelling(1.0) = 5/28; the strobe lands in an odd bucket
        // so the projected progress is the clamped swelling (>= 0.5).
        let swelling = 5.0 / 28.0;
        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert_eq!(
            instances[0].render_state.white_overlay_progress,
            creeper_white_overlay_progress(swelling)
        );
        assert!(instances[0].render_state.white_overlay_progress >= 0.5);
    }

    #[test]
    fn entity_model_instances_project_creeper_swelling_from_world() {
        const VANILLA_ENTITY_TYPE_CREEPER_ID: i32 = 32;
        const CREEPER_SWELL_DIR_DATA_ID: u8 = 16;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            93,
            VANILLA_ENTITY_TYPE_CREEPER_ID,
            [1.0, 64.0, -2.0],
        ));
        let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert_eq!(
            resting[0].render_state.creeper_swelling, 0.0,
            "a calm creeper carries no swell, so CreeperRenderer.scale is the identity"
        );

        // Prime the creeper: swell direction = 1 advances the swell counter each tick, and
        // vanilla `Creeper.getSwelling(partialTick) = swell / SWELL_MAX (28)`.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 93,
            values: vec![EntityDataValue {
                data_id: CREEPER_SWELL_DIR_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(1),
            }],
        }));
        world.advance_entity_client_animations(5);

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert_eq!(
            instances[0].render_state.creeper_swelling,
            5.0 / 28.0,
            "the projected swell drives the renderer inflate-and-flicker scale"
        );
    }

    #[test]
    fn entity_model_instances_project_charged_creeper_from_world() {
        const VANILLA_ENTITY_TYPE_CREEPER_ID: i32 = 32;
        const CREEPER_IS_POWERED_DATA_ID: u8 = 17;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            97,
            VANILLA_ENTITY_TYPE_CREEPER_ID,
            [1.0, 64.0, -2.0],
        ));
        // A plain creeper is not powered, so it wears no CreeperPowerLayer energy swirl.
        let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert!(!resting[0].render_state.creeper_powered);

        // Vanilla `Creeper.DATA_IS_POWERED` (index 17): set true for a lightning-charged creeper.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 97,
            values: vec![EntityDataValue {
                data_id: CREEPER_IS_POWERED_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert!(
            instances[0].render_state.creeper_powered,
            "the charged creeper projects isPowered, gating the energy-swirl overlay"
        );
    }

    #[test]
    fn entity_model_instances_project_powered_wither_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            145,
            VANILLA_ENTITY_TYPE_WITHER_ID,
            [3.0, 64.0, 1.0],
        ));
        // A wither with no synced health defaults to full (maxHealth 300), so it is not powered.
        let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert!(!resting[0].render_state.wither_powered);

        // A healthy wither (health 200/300 > 150) stays un-powered.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 145,
            values: vec![protocol_float_data(LIVING_ENTITY_HEALTH_DATA_ID, 200.0)],
        }));
        let healthy = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert!(!healthy[0].render_state.wither_powered);

        // Vanilla `WitherBoss.isPowered() = getHealth() <= getMaxHealth() / 2`: at or below half
        // health (120 ≤ 150) the `WitherArmorLayer` energy swirl ignites.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 145,
            values: vec![protocol_float_data(LIVING_ENTITY_HEALTH_DATA_ID, 120.0)],
        }));
        let powered = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert!(
            powered[0].render_state.wither_powered,
            "the half-health wither projects isPowered, gating the energy-swirl overlay"
        );
    }

    #[test]
    fn entity_model_instances_project_wither_side_head_rotations() {
        // Vanilla WitherBoss.DATA_TARGET_B (17): the first side-head target id.
        const VANILLA_WITHER_TARGET_B_DATA_ID: u8 = 17;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            145,
            VANILLA_ENTITY_TYPE_WITHER_ID,
            [0.0, 64.0, 0.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            26,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [0.0, 64.0, 0.0],
        ));
        let target_eye = f64::from(world.probe_entity_camera_pose(26).unwrap().eye_height);
        assert!(world.apply_entity_position_sync(EntityPositionSync {
            id: 26,
            position: Vec3d {
                x: 11.3,
                y: 66.2 - target_eye,
                z: 0.0,
            },
            delta_movement: Vec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        }));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 145,
            values: vec![protocol_int_data(VANILLA_WITHER_TARGET_B_DATA_ID, 26)],
        }));
        world.advance_entity_client_animations(1);

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5);
        let wither = instances
            .iter()
            .find(|instance| instance.entity_id == 145)
            .expect("wither instance");
        assert!(wither.render_state.wither_x_head_rots[0].abs() < 1.0e-5);
        assert_eq!(wither.render_state.wither_x_head_rots[1], 0.0);
        assert_eq!(wither.render_state.wither_y_head_rots, [-10.0, 0.0]);
    }

    #[test]
    fn entity_model_kind_uses_vanilla_chicken_variant_metadata() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_CHICKEN_ID, &[]),
            EntityModelKind::Chicken {
                variant: ChickenModelVariant::Temperate,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_CHICKEN_ID,
                &[protocol_chicken_variant_data(1)]
            ),
            EntityModelKind::Chicken {
                variant: ChickenModelVariant::Warm,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_CHICKEN_ID,
                &[
                    protocol_chicken_variant_data(2),
                    protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
                ]
            ),
            EntityModelKind::Chicken {
                variant: ChickenModelVariant::Cold,
                baby: true
            }
        );
    }

    #[test]
    fn entity_model_instances_project_chicken_variants_from_world_registry_order() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            "minecraft:chicken_variant",
            0,
            vec![
                RegistryPacketEntry::stub("minecraft:cold"),
                RegistryPacketEntry::stub("minecraft:temperate"),
                RegistryPacketEntry::stub("minecraft:warm"),
            ],
        );
        let chicken_registry = world.registry_content("minecraft:chicken_variant").unwrap();
        assert_eq!(
            entity_model_kind_with_registries(
                VANILLA_ENTITY_TYPE_CHICKEN_ID,
                &[protocol_chicken_variant_data(99)],
                Some(chicken_registry),
                None,
                None,
                None,
                None,
                None,
            ),
            EntityModelKind::Chicken {
                variant: ChickenModelVariant::Temperate,
                baby: false
            }
        );
        world.apply_add_entity(protocol_add_entity(
            26,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            27,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [3.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 26,
            values: vec![protocol_chicken_variant_data(0)],
        }));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 27,
            values: vec![
                protocol_chicken_variant_data(2),
                protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
            ],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![
                    EntityModelInstance::chicken_variant(
                        26,
                        [1.0, 64.0, -2.0],
                        0.0,
                        ChickenModelVariant::Cold,
                        false
                    ),
                    EntityModelInstance::chicken_variant(
                        27,
                        [3.0, 64.0, -2.0],
                        0.0,
                        ChickenModelVariant::Warm,
                        true
                    ),
                ],
                1.0,
            )
        );
    }

    #[test]
    fn entity_model_kind_uses_vanilla_cow_variant_metadata() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_COW_ID, &[]),
            EntityModelKind::Cow {
                variant: CowModelVariant::Temperate,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_COW_ID, &[protocol_cow_variant_data(1)]),
            EntityModelKind::Cow {
                variant: CowModelVariant::Warm,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_COW_ID,
                &[
                    protocol_cow_variant_data(2),
                    protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
                ]
            ),
            EntityModelKind::Cow {
                variant: CowModelVariant::Cold,
                baby: true
            }
        );
    }

    #[test]
    fn entity_model_instances_project_cow_variants_from_world_registry_order() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            "minecraft:cow_variant",
            0,
            vec![
                RegistryPacketEntry::stub("minecraft:cold"),
                RegistryPacketEntry::stub("minecraft:temperate"),
                RegistryPacketEntry::stub("minecraft:warm"),
            ],
        );
        let cow_registry = world.registry_content("minecraft:cow_variant").unwrap();
        assert_eq!(
            entity_model_kind_with_registries(
                VANILLA_ENTITY_TYPE_COW_ID,
                &[protocol_cow_variant_data(99)],
                None,
                Some(cow_registry),
                None,
                None,
                None,
                None,
            ),
            EntityModelKind::Cow {
                variant: CowModelVariant::Temperate,
                baby: false
            }
        );
        world.apply_add_entity(protocol_add_entity(
            30,
            VANILLA_ENTITY_TYPE_COW_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            31,
            VANILLA_ENTITY_TYPE_COW_ID,
            [3.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 30,
            values: vec![protocol_cow_variant_data(0)],
        }));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 31,
            values: vec![
                protocol_cow_variant_data(2),
                protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
            ],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![
                    EntityModelInstance::cow_variant(
                        30,
                        [1.0, 64.0, -2.0],
                        0.0,
                        CowModelVariant::Cold,
                        false
                    ),
                    EntityModelInstance::cow_variant(
                        31,
                        [3.0, 64.0, -2.0],
                        0.0,
                        CowModelVariant::Warm,
                        true
                    ),
                ],
                1.0,
            )
        );
    }

    #[test]
    fn entity_model_kind_uses_vanilla_pig_variant_metadata() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PIG_ID, &[]),
            EntityModelKind::Pig {
                variant: PigModelVariant::Temperate,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PIG_ID, &[protocol_pig_variant_data(1)]),
            EntityModelKind::Pig {
                variant: PigModelVariant::Warm,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_PIG_ID,
                &[
                    protocol_pig_variant_data(2),
                    protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
                ]
            ),
            EntityModelKind::Pig {
                variant: PigModelVariant::Cold,
                baby: true
            }
        );
    }

    #[test]
    fn entity_model_instances_project_pig_variants_from_world_registry_order() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            "minecraft:pig_variant",
            0,
            vec![
                RegistryPacketEntry::stub("minecraft:cold"),
                RegistryPacketEntry::stub("minecraft:temperate"),
                RegistryPacketEntry::stub("minecraft:warm"),
            ],
        );
        let pig_registry = world.registry_content("minecraft:pig_variant").unwrap();
        assert_eq!(
            entity_model_kind_with_registries(
                VANILLA_ENTITY_TYPE_PIG_ID,
                &[protocol_pig_variant_data(99)],
                None,
                None,
                Some(pig_registry),
                None,
                None,
                None,
            ),
            EntityModelKind::Pig {
                variant: PigModelVariant::Temperate,
                baby: false
            }
        );
        world.apply_add_entity(protocol_add_entity(
            100,
            VANILLA_ENTITY_TYPE_PIG_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            101,
            VANILLA_ENTITY_TYPE_PIG_ID,
            [3.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 100,
            values: vec![protocol_pig_variant_data(0)],
        }));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 101,
            values: vec![
                protocol_pig_variant_data(2),
                protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
            ],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![
                    EntityModelInstance::pig(
                        100,
                        [1.0, 64.0, -2.0],
                        0.0,
                        PigModelVariant::Cold,
                        false,
                    ),
                    EntityModelInstance::pig(
                        101,
                        [3.0, 64.0, -2.0],
                        0.0,
                        PigModelVariant::Warm,
                        true,
                    ),
                ],
                1.0,
            )
        );
    }

    #[test]
    fn entity_model_instances_project_pig_saddle_render_state() {
        const SADDLE_ITEM_ID: i32 = 740;

        let saddle = |entity_id: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Saddle,
                item: ItemStackSummary {
                    item_id: Some(SADDLE_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        };
        let pig_saddle = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .pig_saddle
        };

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            110,
            VANILLA_ENTITY_TYPE_PIG_ID,
            [1.0, 64.0, -3.0],
        ));
        assert!(world.apply_set_equipment(saddle(110)));
        assert!(
            !pig_saddle(&world, 110),
            "without the item registry's saddle-slot map, a raw item id is not enough"
        );

        world.set_default_item_equipment_slots(std::collections::BTreeMap::from([(
            SADDLE_ITEM_ID,
            ItemEquipmentSlot::Saddle,
        )]));
        assert!(pig_saddle(&world, 110));
    }

    #[test]
    fn entity_model_instances_project_snow_golem_pumpkin_render_state() {
        const SNOW_GOLEM_PUMPKIN_DATA_ID: u8 = 16;

        let pumpkin = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .snow_golem_pumpkin
        };

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            111,
            VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
            [1.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            112,
            VANILLA_ENTITY_TYPE_COW_ID,
            [3.0, 64.0, -3.0],
        ));

        assert!(
            pumpkin(&world, 111),
            "SnowGolem.DATA_PUMPKIN_ID defaults to bit 16"
        );
        assert!(!pumpkin(&world, 112));

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 111,
            values: vec![EntityDataValue {
                data_id: SNOW_GOLEM_PUMPKIN_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(0),
            }],
        }));
        assert!(!pumpkin(&world, 111));
    }

    #[test]
    fn entity_model_instances_project_equine_saddle_and_ridden_render_state() {
        const SADDLE_ITEM_ID: i32 = 741;

        let saddle = |entity_id: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Saddle,
                item: ItemStackSummary {
                    item_id: Some(SADDLE_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        };
        let equine_saddle = |world: &WorldStore, id: i32| {
            let render_state = entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state;
            (
                render_state.equine_saddle,
                render_state.equine_saddle_ridden,
            )
        };

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            111,
            VANILLA_ENTITY_TYPE_HORSE_ID,
            [1.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            112,
            VANILLA_ENTITY_TYPE_COW_ID,
            [2.0, 64.0, -3.0],
        ));
        assert!(world.apply_set_equipment(saddle(111)));
        assert_eq!(
            equine_saddle(&world, 111),
            (false, false),
            "without the item registry's saddle-slot map, a raw item id is not enough"
        );

        world.set_default_item_equipment_slots(std::collections::BTreeMap::from([(
            SADDLE_ITEM_ID,
            ItemEquipmentSlot::Saddle,
        )]));
        assert_eq!(equine_saddle(&world, 111), (true, false));

        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 111,
            passenger_ids: vec![112],
        }));
        assert_eq!(equine_saddle(&world, 111), (true, true));
    }

    #[test]
    fn entity_model_instance_projects_equine_tail_counter_from_source() {
        // Vanilla `AbstractHorseRenderer.extractRenderState` maps `tailCounter > 0`
        // to `EquineRenderState.animateTail`; the world layer owns the client-side
        // random counter and native must preserve that bool in the renderer state.
        let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
            "entity_id": 113,
            "entity_type_id": VANILLA_ENTITY_TYPE_HORSE_ID,
            "position": { "x": 1.0, "y": 64.0, "z": -3.0 },
            "y_rot": 0.0,
            "equine_animate_tail": true,
            "data_values": []
        }))
        .unwrap();

        let instance = entity_model_instance(
            source,
            &WorldStore::new(),
            None,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            instance.kind,
            EntityModelKind::Horse {
                baby: false,
                variant: HorseColorVariant::White,
                markings: HorseMarkings::None,
            }
        );
        assert!(instance.render_state.equine_animate_tail);
    }

    #[test]
    fn entity_model_instance_projects_equine_pose_animations_from_source() {
        // Vanilla `AbstractHorseRenderer.extractRenderState` forwards the partial-lerped
        // eat / stand / mouth animation floats to `EquineRenderState`; native preserves
        // the world-owned source projection.
        let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
            "entity_id": 114,
            "entity_type_id": VANILLA_ENTITY_TYPE_HORSE_ID,
            "position": { "x": 1.0, "y": 64.0, "z": -3.0 },
            "y_rot": 0.0,
            "equine_eat_animation": 0.25,
            "equine_stand_animation": 0.5,
            "equine_feeding_animation": 0.75,
            "data_values": []
        }))
        .unwrap();

        let instance = entity_model_instance(
            source,
            &WorldStore::new(),
            None,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(instance.render_state.equine_eat_animation, 0.25);
        assert_eq!(instance.render_state.equine_stand_animation, 0.5);
        assert_eq!(instance.render_state.equine_feeding_animation, 0.75);
    }

    #[test]
    fn entity_model_instances_project_strider_saddle_and_ridden_render_state() {
        const SADDLE_ITEM_ID: i32 = 742;

        let saddle = |entity_id: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Saddle,
                item: ItemStackSummary {
                    item_id: Some(SADDLE_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        };
        let strider_state = |world: &WorldStore, id: i32| {
            let render_state = entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state;
            (render_state.strider_saddle, render_state.strider_ridden)
        };

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            113,
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            [1.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            114,
            VANILLA_ENTITY_TYPE_COW_ID,
            [2.0, 64.0, -3.0],
        ));
        assert!(world.apply_set_equipment(saddle(113)));
        assert_eq!(
            strider_state(&world, 113),
            (false, false),
            "without the item registry's saddle-slot map, a raw item id is not enough"
        );

        world.set_default_item_equipment_slots(std::collections::BTreeMap::from([(
            SADDLE_ITEM_ID,
            ItemEquipmentSlot::Saddle,
        )]));
        assert_eq!(strider_state(&world, 113), (true, false));

        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 113,
            passenger_ids: vec![114],
        }));
        assert_eq!(strider_state(&world, 113), (true, true));
    }

    #[test]
    fn entity_model_instances_project_camel_saddle_and_ridden_render_state() {
        const SADDLE_ITEM_ID: i32 = 743;

        let saddle = |entity_id: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Saddle,
                item: ItemStackSummary {
                    item_id: Some(SADDLE_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        };
        let camel_saddle = |world: &WorldStore, id: i32| {
            let render_state = entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state;
            (render_state.camel_saddle, render_state.camel_saddle_ridden)
        };

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            115,
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            [1.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            116,
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            [2.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            117,
            VANILLA_ENTITY_TYPE_COW_ID,
            [3.0, 64.0, -3.0],
        ));
        assert!(world.apply_set_equipment(saddle(115)));
        assert_eq!(
            camel_saddle(&world, 115),
            (false, false),
            "without the item registry's saddle-slot map, a raw item id is not enough"
        );

        world.set_default_item_equipment_slots(std::collections::BTreeMap::from([(
            SADDLE_ITEM_ID,
            ItemEquipmentSlot::Saddle,
        )]));
        assert_eq!(camel_saddle(&world, 115), (true, false));

        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 115,
            passenger_ids: vec![117],
        }));
        assert_eq!(camel_saddle(&world, 115), (true, true));

        assert!(world.apply_set_equipment(saddle(116)));
        assert_eq!(camel_saddle(&world, 116), (true, false));
    }

    #[test]
    fn entity_model_instances_project_nautilus_saddle_render_state() {
        const SADDLE_ITEM_ID: i32 = 744;

        let saddle = |entity_id: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Saddle,
                item: ItemStackSummary {
                    item_id: Some(SADDLE_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        };
        let nautilus_saddle = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .nautilus_saddle
        };

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            118,
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            [1.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            119,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            [2.0, 64.0, -3.0],
        ));
        assert!(world.apply_set_equipment(saddle(118)));
        assert_eq!(
            nautilus_saddle(&world, 118),
            false,
            "without the item registry's saddle-slot map, a raw item id is not enough"
        );

        world.set_default_item_equipment_slots(std::collections::BTreeMap::from([(
            SADDLE_ITEM_ID,
            ItemEquipmentSlot::Saddle,
        )]));
        assert!(nautilus_saddle(&world, 118));

        assert!(world.apply_set_equipment(saddle(119)));
        assert!(nautilus_saddle(&world, 119));
    }

    #[test]
    fn entity_model_instances_project_nautilus_body_armor_render_state() {
        const IRON_NAUTILUS_ARMOR_ITEM_ID: i32 = 747;
        const NETHERITE_NAUTILUS_ARMOR_ITEM_ID: i32 = 748;
        const AGEABLE_BABY_DATA_ID: u8 = 16;

        let body_armor = |entity_id: i32, item_id: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        };
        let nautilus_body_armor = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .nautilus_body_armor
        };

        let mut world = WorldStore::new();
        world.set_default_nautilus_body_armor_materials(std::collections::BTreeMap::from([
            (IRON_NAUTILUS_ARMOR_ITEM_ID, WorldArmorMaterialKind::Iron),
            (
                NETHERITE_NAUTILUS_ARMOR_ITEM_ID,
                WorldArmorMaterialKind::Netherite,
            ),
        ]));
        world.apply_add_entity(protocol_add_entity(
            123,
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            [1.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            124,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            [2.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            125,
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            [3.0, 64.0, -3.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 125,
            values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
        }));

        assert!(world.apply_set_equipment(body_armor(123, IRON_NAUTILUS_ARMOR_ITEM_ID)));
        assert!(world.apply_set_equipment(body_armor(124, NETHERITE_NAUTILUS_ARMOR_ITEM_ID)));
        assert!(world.apply_set_equipment(body_armor(125, IRON_NAUTILUS_ARMOR_ITEM_ID)));

        assert_eq!(
            nautilus_body_armor(&world, 123),
            Some(EntityArmorMaterial::Iron)
        );
        assert_eq!(
            nautilus_body_armor(&world, 124),
            Some(EntityArmorMaterial::Netherite)
        );
        assert_eq!(
            nautilus_body_armor(&world, 125),
            None,
            "baby living nautilus skip the body armor equipment layer"
        );
    }

    #[test]
    fn entity_model_instances_project_horse_body_armor_render_state() {
        const LEATHER_HORSE_ARMOR_ITEM_ID: i32 = 749;
        const NETHERITE_HORSE_ARMOR_ITEM_ID: i32 = 750;
        const AGEABLE_BABY_DATA_ID: u8 = 16;
        const LEATHER_DYE: i32 = 0x0033_66CC;

        let body_armor = |entity_id: i32, item_id: i32, dyed_color: Option<i32>| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        dyed_color,
                        ..Default::default()
                    },
                },
            }],
        };
        let horse_body_armor = |world: &WorldStore, id: i32| {
            let render_state = entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state;
            (
                render_state.equine_body_armor,
                render_state.equine_body_armor_dye,
            )
        };

        let mut world = WorldStore::new();
        world.set_default_horse_body_armor_materials(std::collections::BTreeMap::from([
            (LEATHER_HORSE_ARMOR_ITEM_ID, WorldArmorMaterialKind::Leather),
            (
                NETHERITE_HORSE_ARMOR_ITEM_ID,
                WorldArmorMaterialKind::Netherite,
            ),
        ]));
        world.apply_add_entity(protocol_add_entity(
            126,
            VANILLA_ENTITY_TYPE_HORSE_ID,
            [1.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            127,
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
            [2.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            128,
            VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
            [3.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            129,
            VANILLA_ENTITY_TYPE_HORSE_ID,
            [4.0, 64.0, -3.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 129,
            values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
        }));

        assert!(world.apply_set_equipment(body_armor(
            126,
            LEATHER_HORSE_ARMOR_ITEM_ID,
            Some(LEATHER_DYE),
        )));
        assert!(world.apply_set_equipment(body_armor(127, NETHERITE_HORSE_ARMOR_ITEM_ID, None,)));
        assert!(world.apply_set_equipment(body_armor(128, NETHERITE_HORSE_ARMOR_ITEM_ID, None,)));
        assert!(world.apply_set_equipment(body_armor(
            129,
            LEATHER_HORSE_ARMOR_ITEM_ID,
            Some(LEATHER_DYE),
        )));

        assert_eq!(
            horse_body_armor(&world, 126),
            (Some(EntityArmorMaterial::Leather), Some(LEATHER_DYE as u32))
        );
        assert_eq!(
            horse_body_armor(&world, 127),
            (Some(EntityArmorMaterial::Netherite), None)
        );
        assert_eq!(
            horse_body_armor(&world, 128),
            (None, None),
            "skeleton horses are not in vanilla CAN_WEAR_HORSE_ARMOR"
        );
        assert_eq!(
            horse_body_armor(&world, 129),
            (None, None),
            "baby horses skip the body armor equipment layer"
        );
    }

    #[test]
    fn entity_model_instances_project_wolf_body_armor_render_state() {
        const WOLF_ARMOR_ITEM_ID: i32 = 751;
        const AGEABLE_BABY_DATA_ID: u8 = 16;
        const WOLF_ARMOR_DYE: i32 = 0x0033_66CC;

        let body_armor =
            |entity_id: i32, damage: i32, enchantment_glint_override: Option<bool>| SetEquipment {
                entity_id,
                slots: vec![EquipmentSlotUpdate {
                    slot: EquipmentSlot::Body,
                    item: ItemStackSummary {
                        item_id: Some(WOLF_ARMOR_ITEM_ID),
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            dyed_color: Some(WOLF_ARMOR_DYE),
                            damage: Some(damage),
                            enchantments: vec![ItemEnchantmentSummary {
                                holder_id: 12,
                                level: 1,
                            }],
                            enchantment_glint_override,
                            ..Default::default()
                        },
                    },
                }],
            };
        let wolf_body_armor = |world: &WorldStore, id: i32| {
            let render_state = entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state;
            (
                render_state.wolf_body_armor,
                render_state.wolf_body_armor_dye,
                render_state.wolf_body_armor_crackiness,
                render_state.wolf_body_armor_foil,
            )
        };

        let mut world = WorldStore::new();
        world.set_default_wolf_body_armor_materials(std::collections::BTreeMap::from([(
            WOLF_ARMOR_ITEM_ID,
            WorldArmorMaterialKind::ArmadilloScute,
        )]));
        world.set_default_item_max_damage(std::collections::BTreeMap::from([(
            WOLF_ARMOR_ITEM_ID,
            64,
        )]));
        world.apply_add_entity(protocol_add_entity(
            130,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            [1.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            131,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            [2.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            132,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            [3.0, 64.0, -3.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 131,
            values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
        }));

        assert!(world.apply_set_equipment(body_armor(130, 24, None)));
        assert!(world.apply_set_equipment(body_armor(131, 44, None)));
        assert!(world.apply_set_equipment(body_armor(132, 24, Some(false))));

        assert_eq!(
            wolf_body_armor(&world, 130),
            (
                Some(EntityArmorMaterial::ArmadilloScute),
                Some(WOLF_ARMOR_DYE as u32),
                Some(WolfArmorCrackiness::Medium),
                true
            )
        );
        assert_eq!(
            wolf_body_armor(&world, 132),
            (
                Some(EntityArmorMaterial::ArmadilloScute),
                Some(WOLF_ARMOR_DYE as u32),
                Some(WolfArmorCrackiness::Medium),
                false
            ),
            "enchantment_glint_override=false wins over non-empty enchantments"
        );
        assert_eq!(
            wolf_body_armor(&world, 131),
            (None, None, None, false),
            "baby wolves skip the adult-only WolfArmorLayer"
        );
    }

    #[test]
    fn entity_model_instances_project_llama_body_decor_render_state() {
        const WHITE_CARPET_ITEM_ID: i32 = 745;
        const BLACK_CARPET_ITEM_ID: i32 = 746;
        const AGEABLE_BABY_DATA_ID: u8 = 16;

        let body_item = |entity_id: i32, item_id: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        };
        let llama_body_decor = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .llama_body_decor
        };

        let mut world = WorldStore::new();
        world.set_default_llama_body_decor_colors(std::collections::BTreeMap::from([
            (WHITE_CARPET_ITEM_ID, WorldLlamaBodyDecorColor::White),
            (BLACK_CARPET_ITEM_ID, WorldLlamaBodyDecorColor::Black),
        ]));
        world.apply_add_entity(protocol_add_entity(
            120,
            VANILLA_ENTITY_TYPE_LLAMA_ID,
            [1.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            121,
            VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
            [2.0, 64.0, -3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            122,
            VANILLA_ENTITY_TYPE_LLAMA_ID,
            [3.0, 64.0, -3.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 122,
            values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
        }));

        assert!(world.apply_set_equipment(body_item(120, WHITE_CARPET_ITEM_ID)));
        assert!(world.apply_set_equipment(body_item(121, BLACK_CARPET_ITEM_ID)));
        assert!(world.apply_set_equipment(body_item(122, WHITE_CARPET_ITEM_ID)));

        assert_eq!(llama_body_decor(&world, 120), Some(EntityDyeColor::White));
        assert_eq!(llama_body_decor(&world, 121), Some(EntityDyeColor::Black));
        assert_eq!(
            llama_body_decor(&world, 122),
            None,
            "baby llamas ignore body items; renderer handles trader baby fallback separately"
        );
    }

    #[test]
    fn entity_model_instances_project_armor_stand_flags_and_pose() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            5,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            [1.0, 64.0, -2.0],
        ));
        let mut pose = DEFAULT_ARMOR_STAND_MODEL_POSE;
        pose.body = [0.0, 15.0, 0.0];
        pose.left_arm = [-30.0, 0.0, -20.0];
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 5,
            values: vec![
                protocol_byte_data(
                    ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
                    ARMOR_STAND_CLIENT_FLAG_SMALL
                        | ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS
                        | ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE
                        | ARMOR_STAND_CLIENT_FLAG_MARKER,
                ),
                protocol_rotations_data(ARMOR_STAND_BODY_POSE_DATA_ID, pose.body),
                protocol_rotations_data(ARMOR_STAND_LEFT_ARM_POSE_DATA_ID, pose.left_arm),
            ],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![EntityModelInstance::armor_stand_with_marker(
                    5,
                    [1.0, 64.0, -2.0],
                    0.0,
                    true,
                    true,
                    true,
                    false,
                    pose,
                )],
                1.0,
            )
        );
    }

    #[test]
    fn entity_model_instances_project_avatar_model_part_visibility_from_world() {
        let mut world = WorldStore::new();
        let player = protocol_add_entity(1550, VANILLA_ENTITY_TYPE_PLAYER_ID, [1.0, 64.0, -2.0]);
        let player_uuid = player.uuid;
        world.apply_add_entity(player);
        world.apply_add_entity(protocol_add_entity(
            830,
            VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
            [3.0, 64.0, -2.0],
        ));
        let player_parts = PlayerModelPartVisibility::from_vanilla_mask(
            PlayerModelPartVisibility::HAT_MASK
                | PlayerModelPartVisibility::JACKET_MASK
                | PlayerModelPartVisibility::RIGHT_PANTS_MASK,
        );
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 1550,
            values: vec![protocol_byte_data(
                AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
                player_parts.vanilla_mask() as i8,
            )],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![
                    EntityModelInstance::player_with_skin(
                        1550,
                        [1.0, 64.0, -2.0],
                        0.0,
                        EntityPlayerSkin::Default(default_player_skin_for_profile_id(
                            player_uuid.as_u128(),
                        )),
                        player_parts,
                    ),
                    EntityModelInstance::player_with_parts(
                        830,
                        [3.0, 64.0, -2.0],
                        0.0,
                        false,
                        PlayerModelPartVisibility::from_vanilla_mask(
                            PlayerModelPartVisibility::ALL_MASK,
                        ),
                    ),
                ],
                1.0,
            )
        );
    }

    #[test]
    fn entity_model_instances_use_uuid_default_skin_without_player_info() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity_with_uuid(
            1551,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            Uuid::nil(),
            [2.0, 64.0, -2.0],
        ));

        let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 1551)
            .unwrap();

        assert_eq!(
            instance.kind,
            EntityModelKind::Player {
                skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::SlimAlex),
                parts: PlayerModelPartVisibility::from_vanilla_mask(0),
            }
        );
    }

    #[test]
    fn entity_model_instances_use_player_info_profile_skin_for_players() {
        const SLIM_TEXTURES_PROPERTY: &str = "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=";

        let mut world = WorldStore::new();
        let player = protocol_add_entity(1552, VANILLA_ENTITY_TYPE_PLAYER_ID, [1.0, 64.0, -2.0]);
        let profile_id = player.uuid;
        world.apply_add_entity(player);
        world.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![PlayerInfoAction::AddPlayer],
            entries: vec![PlayerInfoEntry {
                profile_id,
                profile: Some(GameProfile {
                    uuid: profile_id,
                    name: "Alex".to_string(),
                    properties: vec![GameProfileProperty {
                        name: "textures".to_string(),
                        value: SLIM_TEXTURES_PROPERTY.to_string(),
                        signature: Some("signature".to_string()),
                    }],
                }),
                listed: true,
                latency: 0,
                game_mode: GameType::Survival,
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            }],
        });
        let runtime = NativeItemRuntime::empty_for_test();

        let instance =
            entity_model_instances_from_world_at_partial_tick(&world, Some(&runtime), 1.0)
                .into_iter()
                .find(|instance| instance.entity_id == 1552)
                .unwrap();

        let EntityModelKind::Player { skin, parts } = instance.kind else {
            panic!("player entity should use the player model");
        };
        let EntityPlayerSkin::Dynamic(skin) = skin else {
            panic!("player info textures property should produce a dynamic player skin");
        };
        assert_eq!(skin.model, EntityPlayerSkinModel::Slim);
        assert_eq!(skin.status, EntityDynamicPlayerSkinStatus::Loading);
        assert_ne!(skin.handle, 0);
        assert_eq!(parts, PlayerModelPartVisibility::from_vanilla_mask(0));
        let cape = instance
            .render_state
            .player_cape_texture
            .expect("profile cape texture");
        let elytra = instance
            .render_state
            .player_elytra_texture
            .expect("profile elytra texture");
        assert_eq!(cape.kind, EntityDynamicPlayerTextureKind::Cape);
        assert_eq!(elytra.kind, EntityDynamicPlayerTextureKind::Elytra);
        assert_ne!(cape.handle, 0);
        assert_ne!(elytra.handle, 0);
        assert_ne!(cape.handle, elytra.handle);
    }

    #[test]
    fn entity_model_instances_forward_player_extra_ears_from_world_source() {
        let mut world = WorldStore::new();
        let deadmau5_uuid = Uuid::from_u128(0xCCCC_CCCC_CCCC_CCCC_CCCC_CCCC_CCCC_CCCC);
        let mixed_case_uuid = Uuid::from_u128(0xDDDD_DDDD_DDDD_DDDD_DDDD_DDDD_DDDD_DDDD);
        world.apply_add_entity(protocol_add_entity_with_uuid(
            1554,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            deadmau5_uuid,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity_with_uuid(
            1555,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            mixed_case_uuid,
            [2.0, 64.0, -2.0],
        ));
        world.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![PlayerInfoAction::AddPlayer],
            entries: vec![
                PlayerInfoEntry {
                    profile_id: deadmau5_uuid,
                    profile: Some(GameProfile {
                        uuid: deadmau5_uuid,
                        name: "deadmau5".to_string(),
                        properties: Vec::new(),
                    }),
                    listed: true,
                    latency: 0,
                    game_mode: GameType::Survival,
                    display_name: None,
                    show_hat: true,
                    list_order: 0,
                    chat_session: None,
                },
                PlayerInfoEntry {
                    profile_id: mixed_case_uuid,
                    profile: Some(GameProfile {
                        uuid: mixed_case_uuid,
                        name: "Deadmau5".to_string(),
                        properties: Vec::new(),
                    }),
                    listed: true,
                    latency: 0,
                    game_mode: GameType::Survival,
                    display_name: None,
                    show_hat: true,
                    list_order: 0,
                    chat_session: None,
                },
            ],
        });

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        let deadmau5 = instances
            .iter()
            .find(|instance| instance.entity_id == 1554)
            .expect("deadmau5 player instance");
        let mixed_case = instances
            .iter()
            .find(|instance| instance.entity_id == 1555)
            .expect("mixed-case player instance");

        assert!(deadmau5.render_state.show_extra_ears);
        assert!(!mixed_case.render_state.show_extra_ears);
    }

    #[test]
    fn entity_model_instances_forward_player_cape_cloak_state() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            1553,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [0.0, 64.0, 0.0],
        ));
        assert!(world.apply_entity_position_sync(EntityPositionSync {
            id: 1553,
            position: Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        }));
        world.advance_entity_client_animations(1);
        assert!(world.apply_entity_position_sync(EntityPositionSync {
            id: 1553,
            position: Vec3d {
                x: 0.0,
                y: 65.0,
                z: 1.0,
            },
            delta_movement: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        }));
        world.advance_entity_client_animations(1);

        let source = world
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 1553)
            .unwrap();
        let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 1553)
            .unwrap();

        assert_eq!(
            (
                instance.render_state.player_cape_flap,
                instance.render_state.player_cape_lean,
                instance.render_state.player_cape_lean2,
            ),
            (
                source.player_cape_flap,
                source.player_cape_lean,
                source.player_cape_lean2,
            )
        );
        assert_eq!(instance.render_state.player_cape_flap, -6.0);
        assert_eq!(instance.render_state.player_cape_lean, 74.25);
        assert_eq!(instance.render_state.player_cape_lean2, 0.0);
    }

    #[test]
    fn entity_model_instances_forward_player_shoulder_parrots_from_world_source() {
        // Vanilla Player shoulder parrots are `OPTIONAL_UNSIGNED_INT` metadata ids 19/20. World keeps
        // the raw `Parrot.Variant` ids; native maps them to renderer `ParrotModelVariant`s.
        const PLAYER_SHOULDER_PARROT_LEFT_DATA_ID: u8 = 19;
        const PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID: u8 = 20;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            1556,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [0.0, 64.0, 0.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 1556,
            values: vec![
                protocol_optional_unsigned_int_data(PLAYER_SHOULDER_PARROT_LEFT_DATA_ID, Some(4),),
                protocol_optional_unsigned_int_data(PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID, Some(1),),
            ],
        }));

        let source = world
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 1556)
            .unwrap();
        let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 1556)
            .unwrap();

        assert_eq!(source.player_left_shoulder_parrot, Some(4));
        assert_eq!(source.player_right_shoulder_parrot, Some(1));
        assert_eq!(
            instance.render_state.player_left_shoulder_parrot,
            Some(ParrotModelVariant::Gray)
        );
        assert_eq!(
            instance.render_state.player_right_shoulder_parrot,
            Some(ParrotModelVariant::Blue)
        );
    }

    #[test]
    fn entity_model_instances_project_chest_equipment_layers() {
        const CHESTPLATE_ID: i32 = 0;
        const ELYTRA_ID: i32 = 1;

        let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
            "resource_ids": [
                "minecraft:diamond_chestplate",
                "minecraft:elytra"
            ],
            "protocol_ids": {
                "minecraft:diamond_chestplate": CHESTPLATE_ID,
                "minecraft:elytra": ELYTRA_ID
            },
            "default_equipment_slots": {
                "minecraft:diamond_chestplate": "chest",
                "minecraft:elytra": "chest"
            },
            "humanoid_armor_assets": {
                "minecraft:diamond_chestplate": "diamond"
            },
            "equippable_assets": {
                "minecraft:diamond_chestplate": "diamond",
                "minecraft:elytra": "elytra"
            }
        }))
        .unwrap();
        let equipment_assets: bbb_pack::EquipmentAssetCatalog =
            serde_json::from_value(serde_json::json!({
                "assets": {
                    "minecraft:diamond": {
                        "layers": {
                            "humanoid": [
                                {
                                    "texture": "minecraft:diamond",
                                    "texture_location": "minecraft:textures/entity/equipment/humanoid/diamond.png",
                                    "use_player_texture": false
                                }
                            ]
                        }
                    },
                    "minecraft:elytra": {
                        "layers": {
                            "wings": [
                                {
                                    "texture": "minecraft:elytra",
                                    "texture_location": "minecraft:textures/entity/equipment/wings/elytra.png",
                                    "use_player_texture": true
                                }
                            ]
                        }
                    }
                }
            }))
            .unwrap();
        let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
            registry,
            equipment_assets,
        );

        let mut world = WorldStore::new();
        world.set_item_armor_materials(runtime.item_armor_materials_by_protocol_id());
        world.apply_add_entity(protocol_add_entity(
            1553,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            1554,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            [3.0, 64.0, -2.0],
        ));
        let equip_with_patch = |entity_id: i32,
                                item_id: Option<i32>,
                                count: i32,
                                component_patch: DataComponentPatchSummary|
         -> SetEquipment {
            SetEquipment {
                entity_id,
                slots: vec![EquipmentSlotUpdate {
                    slot: EquipmentSlot::Chest,
                    item: ItemStackSummary {
                        item_id,
                        count,
                        component_patch,
                    },
                }],
            }
        };
        let equip = |entity_id: i32, item_id: Option<i32>, count: i32| {
            equip_with_patch(
                entity_id,
                item_id,
                count,
                DataComponentPatchSummary::default(),
            )
        };
        let state = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
        };

        assert!(world.apply_set_equipment(equip(1553, Some(ELYTRA_ID), 1)));
        let with_elytra = state(&world, 1553);
        assert!(with_elytra.chest_equipment_has_wings);
        assert!(!with_elytra.chest_equipment_has_humanoid);
        assert_eq!(with_elytra.chest_armor, None);
        assert!(!with_elytra.chest_armor_foil);
        assert_eq!(
            with_elytra.chest_wings_layer,
            Some(EntityEquipmentLayerTexture {
                texture: bbb_renderer::EntityModelTextureRef {
                    path: "textures/entity/equipment/wings/elytra.png",
                    size: [64, 32],
                },
                use_player_texture: true,
            })
        );

        assert!(world.apply_set_equipment(equip_with_patch(
            1553,
            Some(CHESTPLATE_ID),
            1,
            DataComponentPatchSummary {
                enchantments: vec![ItemEnchantmentSummary {
                    holder_id: 12,
                    level: 1,
                }],
                ..Default::default()
            }
        )));
        let with_chestplate = state(&world, 1553);
        assert!(!with_chestplate.chest_equipment_has_wings);
        assert!(with_chestplate.chest_equipment_has_humanoid);
        assert_eq!(
            with_chestplate.chest_armor,
            Some(EntityArmorMaterial::Diamond)
        );
        assert!(with_chestplate.chest_armor_foil);
        assert_eq!(with_chestplate.chest_wings_layer, None);

        assert!(world.apply_set_equipment(equip(1553, None, 0)));
        let empty_chest = state(&world, 1553);
        assert!(!empty_chest.chest_equipment_has_wings);
        assert!(!empty_chest.chest_equipment_has_humanoid);
        assert_eq!(empty_chest.chest_armor, None);
        assert!(!empty_chest.chest_armor_foil);
        assert_eq!(empty_chest.chest_wings_layer, None);

        assert!(world.apply_set_equipment(equip(1554, Some(ELYTRA_ID), 1)));
        let zombie = state(&world, 1554);
        assert!(zombie.chest_equipment_has_wings);
        assert!(!zombie.chest_equipment_has_humanoid);
        assert_eq!(zombie.chest_armor, None);
        assert!(!zombie.chest_armor_foil);
        assert_eq!(
            zombie.chest_wings_layer,
            Some(EntityEquipmentLayerTexture {
                texture: bbb_renderer::EntityModelTextureRef {
                    path: "textures/entity/equipment/wings/elytra.png",
                    size: [64, 32],
                },
                use_player_texture: true,
            })
        );
    }

    #[test]
    fn entity_model_instances_project_slime_and_magma_cube_size() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            117,
            VANILLA_ENTITY_TYPE_SLIME_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            80,
            VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
            [3.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 117,
            values: vec![protocol_int_data(SLIME_SIZE_DATA_ID, 4)],
        }));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 80,
            values: vec![protocol_int_data(SLIME_SIZE_DATA_ID, 3)],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![
                    EntityModelInstance::slime(117, [1.0, 64.0, -2.0], 0.0, 4),
                    EntityModelInstance::magma_cube(80, [3.0, 64.0, -2.0], 0.0, 3),
                ],
                1.0,
            )
        );
    }

    #[test]
    fn entity_model_instances_project_age_in_ticks_from_world_age_and_partial_tick() {
        // Vanilla `EntityRenderState.ageInTicks = entity.tickCount + partialTick`: the world
        // tracks the per-entity client-animation age and the scene lerps it with the partial
        // tick. After 7 client ticks at partial 0.25 the projected age is 7.25.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            70,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, -2.0],
        ));
        world.advance_entity_client_animations(7);

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);

        assert_eq!(instances.len(), 1);
        assert!(
            (instances[0].render_state.age_in_ticks - 7.25).abs() < 1e-6,
            "{}",
            instances[0].render_state.age_in_ticks
        );
    }

    #[test]
    fn entity_model_instances_do_not_drop_pick_target_entity_types() {
        let mut world = WorldStore::new();
        for (index, entity_type_id) in VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS
            .iter()
            .copied()
            .enumerate()
        {
            world.apply_add_entity(protocol_add_entity(
                1000 + index as i32,
                entity_type_id,
                [index as f64 * 2.0, 64.0, 0.0],
            ));
        }

        let sources = world.entity_model_sources_at_partial_tick(1.0);
        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            sources.len(),
            VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS.len()
        );
        assert_eq!(instances.len(), sources.len());
    }

    #[test]
    fn entity_model_kind_maps_all_vanilla_registry_ids() {
        for entity_type_id in 0..=VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID {
            let kind = entity_model_kind(entity_type_id, &[]);
            assert!(
                placeholder_name(kind) != Some("todo_unknown_entity_type_bounds"),
                "vanilla type id {entity_type_id} fell through to unknown renderer entity model fallback"
            );
        }
    }

    #[test]
    fn lightning_bolt_uses_weather_target_renderer_not_entity_placeholder() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, &[]),
            EntityModelKind::NoRender
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_base_zombie_and_skeleton() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_ID, &[]),
            EntityModelKind::Zombie { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                &[protocol_bool_data(ZOMBIE_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Zombie { baby: true }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SKELETON_ID, &[]),
            EntityModelKind::Skeleton
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_HUSK_ID, &[]),
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_HUSK_ID,
                &[protocol_bool_data(ZOMBIE_BABY_DATA_ID, true)]
            ),
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: true
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_DROWNED_ID, &[]),
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_DROWNED_ID,
                &[protocol_bool_data(ZOMBIE_BABY_DATA_ID, true)]
            ),
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: true
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID, &[]),
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
                &[protocol_bool_data(ZOMBIE_BABY_DATA_ID, true)]
            ),
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: true
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PIGLIN_ID, &[]),
            EntityModelKind::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_PIGLIN_ID,
                &[protocol_bool_data(PIGLIN_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: true
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, &[]),
            EntityModelKind::Piglin {
                family: PiglinModelFamily::PiglinBrute,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID, &[]),
            EntityModelKind::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
                &[protocol_bool_data(ZOMBIE_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: true
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_STRAY_ID, &[]),
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Stray
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PARCHED_ID, &[]),
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Parched
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, &[]),
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::WitherSkeleton
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_BOGGED_ID, &[]),
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Bogged { sheared: false }
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_BOGGED_ID,
                &[protocol_bool_data(BOGGED_SHEARED_DATA_ID, true)]
            ),
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Bogged { sheared: true }
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_armor_stands() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, &[]),
            EntityModelKind::ArmorStand {
                small: false,
                marker: false,
                show_arms: false,
                show_base_plate: true,
                pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                &[protocol_byte_data(
                    ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
                    ARMOR_STAND_CLIENT_FLAG_SMALL
                        | ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS
                        | ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE
                        | ARMOR_STAND_CLIENT_FLAG_MARKER,
                )],
            ),
            EntityModelKind::ArmorStand {
                small: true,
                marker: true,
                show_arms: true,
                show_base_plate: false,
                pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
            }
        );

        let mut pose = DEFAULT_ARMOR_STAND_MODEL_POSE;
        pose.head = [1.0, 2.0, 3.0];
        pose.right_arm = [-20.0, 5.0, 10.0];
        pose.left_leg = [4.0, 5.0, 6.0];
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                &[
                    protocol_rotations_data(ARMOR_STAND_HEAD_POSE_DATA_ID, pose.head),
                    protocol_rotations_data(ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID, pose.right_arm),
                    protocol_rotations_data(ARMOR_STAND_LEFT_LEG_POSE_DATA_ID, pose.left_leg),
                ],
            ),
            EntityModelKind::ArmorStand {
                small: false,
                marker: false,
                show_arms: false,
                show_base_plate: true,
                pose,
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_slime_and_magma_cube() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SLIME_ID, &[]),
            EntityModelKind::Slime { size: 1 }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SLIME_ID,
                &[protocol_int_data(SLIME_SIZE_DATA_ID, 4)]
            ),
            EntityModelKind::Slime { size: 4 }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, &[]),
            EntityModelKind::MagmaCube { size: 1 }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
                &[protocol_int_data(SLIME_SIZE_DATA_ID, 3)]
            ),
            EntityModelKind::MagmaCube { size: 3 }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_ghast() {
        // The ghast was a placeholder render box; it now resolves to the real model. The `charging` flag
        // (vanilla `Ghast.DATA_IS_CHARGING`, BOOLEAN at index 16) swaps to the shooting texture.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_GHAST_ID, &[]),
            EntityModelKind::Ghast { charging: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_GHAST_ID,
                &[protocol_bool_data(GHAST_IS_CHARGING_DATA_ID, true)]
            ),
            EntityModelKind::Ghast { charging: true }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_happy_ghast() {
        // The happy ghast was a placeholder render box; it now resolves to the real model.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, &[]),
            EntityModelKind::HappyGhast
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_blaze() {
        // The blaze was a placeholder render box; it now resolves to the real model.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_BLAZE_ID, &[]),
            EntityModelKind::Blaze
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_endermite() {
        // The endermite was a placeholder render box; it now resolves to the real model.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ENDERMITE_ID, &[]),
            EntityModelKind::Endermite
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_silverfish() {
        // The silverfish was a placeholder render box; it now resolves to the real model.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SILVERFISH_ID, &[]),
            EntityModelKind::Silverfish
        );
    }

    #[test]
    fn entity_model_kind_projects_pufferfish_puff_state_from_data() {
        // The pufferfish was a placeholder render box; it now resolves to the real model and
        // projects its synced `PUFF_STATE` (index 17, defaulting to 0 = deflated).
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PUFFERFISH_ID, &[]),
            EntityModelKind::Pufferfish { puff_state: 0 }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
                &[protocol_int_data(PUFFERFISH_PUFF_STATE_DATA_ID, 2)]
            ),
            EntityModelKind::Pufferfish { puff_state: 2 }
        );
    }

    #[test]
    fn entity_model_kind_maps_vex_to_real_model() {
        // The vex resolves to the real `VexModel`. Its idle wing flap / arm bob / head look read the
        // projected age and look angles. `Vex.DATA_FLAGS_ID` (16, BYTE) bit 1 (`isCharging`) projects
        // to `charging`, which vanilla `VexRenderer.getTextureLocation` swaps to `vex_charging.png`.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_VEX_ID, &[]),
            EntityModelKind::Vex { charging: false }
        );
        let charging_values = vec![protocol_byte_data(VEX_FLAGS_DATA_ID, VEX_FLAG_IS_CHARGING)];
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_VEX_ID, &charging_values),
            EntityModelKind::Vex { charging: true }
        );
    }

    #[test]
    fn entity_model_kind_maps_allay_to_real_model() {
        // The allay was a placeholder render box; it now resolves to the real `AllayModel`. Its
        // idle/flying wing flap, arm bob, head look, and vertical bob read the projected age,
        // walk animation, and look angles; the dance pose and held item are deferred
        // entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ALLAY_ID, &[]),
            EntityModelKind::Allay
        );
    }

    #[test]
    fn entity_model_kind_maps_bat_to_real_model() {
        // The bat was a placeholder render box; it now resolves to the real `BatModel`, the
        // first keyframe-animated entity. Its looping `BAT_FLYING` wing flap reads the projected
        // age; the resting pose (`isResting` / `BAT_RESTING`) is deferred entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_BAT_ID, &[]),
            EntityModelKind::Bat
        );
    }

    #[test]
    fn entity_model_kind_projects_bee_baby_from_data() {
        // The bee was a placeholder render box; it now resolves to the real `AdultBeeModel` /
        // `BabyBeeModel`, keyed off the synced `AgeableMob.DATA_BABY_ID` (index 16, default adult).
        // The procedural airborne flap / bob reads the projected age and ground state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_BEE_ID, &[]),
            EntityModelKind::Bee {
                baby: false,
                angry: false,
                has_nectar: false,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_BEE_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Bee {
                baby: true,
                angry: false,
                has_nectar: false,
            }
        );
    }

    #[test]
    fn entity_model_kind_projects_bee_nectar_and_angry_texture_flags() {
        // Vanilla `BeeRenderer.getTextureLocation` keys on `hasNectar` (the synced
        // `DATA_FLAGS_ID & 8`, index 18) and `isAngry` (the synced `DATA_ANGER_END_TIME`, index 19,
        // in the future). A bee carrying nectar swaps to the `*_nectar*` texture.
        assert_eq!(
            entity_model_kind_with_time_and_registries(
                VANILLA_ENTITY_TYPE_BEE_ID,
                &[protocol_byte_data(BEE_FLAGS_DATA_ID, BEE_FLAG_HAS_NECTAR)],
                0.0,
                0,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            EntityModelKind::Bee {
                baby: false,
                angry: false,
                has_nectar: true,
            }
        );
        // An anger-end time past the current game time makes the bee angry (and the roll/stung
        // bits in the flags byte do not flip `hasNectar`).
        assert_eq!(
            entity_model_kind_with_time_and_registries(
                VANILLA_ENTITY_TYPE_BEE_ID,
                &[protocol_long_data(BEE_ANGER_END_TIME_DATA_ID, 100)],
                0.0,
                10,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            EntityModelKind::Bee {
                baby: false,
                angry: true,
                has_nectar: false,
            }
        );
    }

    #[test]
    fn entity_model_kind_projects_dolphin_baby_from_data() {
        // The dolphin was a placeholder render box; it now resolves to the real `DolphinModel`,
        // keyed off the synced `AgeableMob.DATA_BABY_ID` (index 16, default adult). Its swim body
        // tilt / tail wave reads the projected `isMoving` (the synced velocity); the held-item
        // carry layer is deferred entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_DOLPHIN_ID, &[]),
            EntityModelKind::Dolphin { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_DOLPHIN_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Dolphin { baby: true }
        );
    }

    #[test]
    fn entity_model_kind_maps_guardian_and_elder_guardian_to_real_model() {
        // Both guardians were placeholder render boxes; they now resolve to the real
        // `GuardianModel`. The variant is keyed purely off the entity type id (the elder is the
        // same mesh scaled 2.35×), with no synced data. The procedural spike pulse / withdrawal,
        // eye tracking, tail sway, and attack beam are deferred entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_GUARDIAN_ID, &[]),
            EntityModelKind::Guardian { elder: false }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, &[]),
            EntityModelKind::Guardian { elder: true }
        );
    }

    #[test]
    fn entity_model_kind_maps_creaking_to_real_model() {
        // The creaking was a placeholder render box; it now resolves to the real `CreakingModel`
        // at its rest pose. The head look, walk, attack, invulnerable, and death keyframe animations
        // are deferred entity-side state. The emissive eyes layer IS projected: `eyes_glowing` tracks
        // the synced `IS_ACTIVE` flag (17), defaulting to dormant with no data.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_CREAKING_ID, &[]),
            EntityModelKind::Creaking {
                eyes_glowing: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_CREAKING_ID,
                &[protocol_bool_data(CREAKING_IS_ACTIVE_DATA_ID, true)]
            ),
            EntityModelKind::Creaking { eyes_glowing: true }
        );
    }

    #[test]
    fn entity_model_kind_maps_frog_to_real_model() {
        // The frog resolves to the real `FrogModel` at its rest pose, textured by temperature
        // variant. With no synced `DATA_VARIANT_ID` it defaults to TEMPERATE; otherwise the
        // `Holder<FrogVariant>` registry id selects the colour. Without a synced `frog_variant`
        // registry, the static `FrogVariants.bootstrap` order (TEMPERATE=0, WARM=1, COLD=2) applies.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_FROG_ID, &[]),
            EntityModelKind::Frog {
                variant: FrogModelVariant::Temperate
            }
        );
        for (id, variant) in [
            (0, FrogModelVariant::Temperate),
            (1, FrogModelVariant::Warm),
            (2, FrogModelVariant::Cold),
        ] {
            assert_eq!(
                entity_model_kind(
                    VANILLA_ENTITY_TYPE_FROG_ID,
                    &[protocol_frog_variant_data(id)]
                ),
                EntityModelKind::Frog { variant }
            );
        }
    }

    #[test]
    fn entity_model_kind_maps_breeze_to_real_model() {
        // The breeze was a placeholder render box; it now resolves to the real `BreezeModel`, the
        // second keyframe entity (and the first to use CATMULLROM). Its looping `IDLE` head bob /
        // rod spin reads the projected age; the wind layer, eyes, and action animations are
        // deferred entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_BREEZE_ID, &[]),
            EntityModelKind::Breeze
        );
    }

    #[test]
    fn entity_model_kind_projects_turtle_baby_from_data() {
        // The turtle was a placeholder render box; it now resolves to the real
        // `AdultTurtleModel` / `BabyTurtleModel`, keyed off the synced `AgeableMob.DATA_BABY_ID`
        // (index 16, default adult). The head look and land-walk / water-swim leg branch read
        // the projected look angles, walk animation, water, and ground state; the egg-laying
        // amplitude and the egg-belly shell are deferred entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_TURTLE_ID, &[]),
            EntityModelKind::Turtle { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_TURTLE_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Turtle { baby: true }
        );
    }

    #[test]
    fn entity_model_kind_projects_strider_baby_and_cold_from_data() {
        // The strider previously fell back to the horse quadruped; it now resolves to the real
        // `AdultStriderModel` / `BabyStriderModel`, keyed off the synced `AgeableMob.DATA_BABY_ID`
        // (index 16, default adult). The `cold` flag is the synced `DATA_SUFFOCATING` (19), swapping
        // to the `strider_cold` texture; ridden pose and saddle layer are generic render-state flags.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_STRIDER_ID, &[]),
            EntityModelKind::Strider {
                baby: false,
                cold: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_STRIDER_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Strider {
                baby: true,
                cold: false
            }
        );
        // A suffocating strider carries the cold texture.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_STRIDER_ID,
                &[protocol_bool_data(STRIDER_SUFFOCATING_DATA_ID, true)]
            ),
            EntityModelKind::Strider {
                baby: false,
                cold: true
            }
        );
    }

    #[test]
    fn entity_model_kind_maps_cod_to_real_model() {
        // The cod was a placeholder render box; it now resolves to the real `CodModel`. Its
        // tail sway / out-of-water flop read the projected `in_water` render-state flag.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_COD_ID, &[]),
            EntityModelKind::Cod
        );
    }

    #[test]
    fn entity_model_kind_projects_salmon_size_from_variant_data() {
        // The salmon was a placeholder render box; it now resolves to the real `SalmonModel`
        // and projects its synced `DATA_TYPE` size variant (index 17, `Salmon.Variant` ids
        // SMALL=0/MEDIUM=1/LARGE=2 clamped, defaulting to MEDIUM).
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SALMON_ID, &[]),
            EntityModelKind::Salmon {
                size: SalmonModelSize::Medium,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SALMON_ID,
                &[protocol_int_data(SALMON_VARIANT_DATA_ID, 0)]
            ),
            EntityModelKind::Salmon {
                size: SalmonModelSize::Small,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SALMON_ID,
                &[protocol_int_data(SALMON_VARIANT_DATA_ID, 2)]
            ),
            EntityModelKind::Salmon {
                size: SalmonModelSize::Large,
            }
        );
        // Out-of-range ids clamp to the large body, matching `ByIdMap.continuous(CLAMP)`.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SALMON_ID,
                &[protocol_int_data(SALMON_VARIANT_DATA_ID, 9)]
            ),
            EntityModelKind::Salmon {
                size: SalmonModelSize::Large,
            }
        );
    }

    #[test]
    fn entity_model_kind_projects_tropical_fish_shape_from_packed_variant() {
        // The tropical fish was a placeholder render box; it now resolves to the real model
        // and decodes the body shape from the synced packed variant (`DATA_ID_TYPE_VARIANT`,
        // index 17). The default 0 (KOB/white/white) is the small body; a LARGE-base pattern
        // selects the flopper body.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID, &[]),
            EntityModelKind::TropicalFish {
                shape: TropicalFishModelShape::Small,
                base_color: EntityDyeColor::White,
                // pattern bits = 0 → Pattern.byId(0) = KOB; pattern color = (0 >> 24) = WHITE.
                pattern: TropicalFishPattern::Kob,
                pattern_color: EntityDyeColor::White,
            }
        );
        // FLOPPER (LARGE base, index 0) with arbitrary base/pattern color bytes → large body.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
                &[protocol_int_data(
                    TROPICAL_FISH_VARIANT_DATA_ID,
                    0x0405_0001
                )]
            ),
            EntityModelKind::TropicalFish {
                shape: TropicalFishModelShape::Large,
                // base byte = (0x0405_0001 >> 16) & 0xFF = 0x05 → DyeColor.byId(5) = LIME.
                base_color: EntityDyeColor::Lime,
                // pattern bits = 0x0405_0001 & 0xFFFF = 1 → Pattern.byId(1) = FLOPPER.
                pattern: TropicalFishPattern::Flopper,
                // pattern color = (0x0405_0001 >> 24) & 0xFF = 0x04 → DyeColor.byId(4) = YELLOW.
                pattern_color: EntityDyeColor::Yellow,
            }
        );
        // SPOTTY (SMALL base, index 5 → 0x0500) stays the small body.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
                &[protocol_int_data(TROPICAL_FISH_VARIANT_DATA_ID, 0x0500)]
            ),
            EntityModelKind::TropicalFish {
                shape: TropicalFishModelShape::Small,
                // base byte = (0x0500 >> 16) & 0xFF = 0 → DyeColor.byId(0) = WHITE.
                base_color: EntityDyeColor::White,
                // pattern bits = 0x0500 = 1280 → Pattern.byId(1280) = SPOTTY (small, index 5).
                pattern: TropicalFishPattern::Spotty,
                pattern_color: EntityDyeColor::White,
            }
        );
    }

    #[test]
    fn entity_model_kind_projects_tropical_fish_base_color_from_packed_variant() {
        // Vanilla `TropicalFish.getBaseColor(packedVariant) = DyeColor.byId(packedVariant >> 16
        // & 0xFF)`, surfaced by `TropicalFishRenderer.getModelTint = state.baseColor`. Each dye
        // id occupies bits 16..24 of the packed variant; the low 16 bits (pattern) and high 8
        // bits (pattern color) must not bleed into the base color.
        let base_color_of = |packed: i32| match entity_model_kind(
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
            &[protocol_int_data(TROPICAL_FISH_VARIANT_DATA_ID, packed)],
        ) {
            EntityModelKind::TropicalFish { base_color, .. } => base_color,
            other => panic!("expected tropical fish, got {other:?}"),
        };
        // id 0 → WHITE, id 11 → BLUE, id 15 → BLACK, with noise in the other byte ranges.
        assert_eq!(
            base_color_of(0x00FF_FFFF & !0x00FF_0000),
            EntityDyeColor::White
        );
        assert_eq!(base_color_of(0x000B_0000), EntityDyeColor::Blue);
        assert_eq!(base_color_of(0xFF0F_FFFFu32 as i32), EntityDyeColor::Black);
        // Out-of-range base byte (16) falls back to WHITE like `DyeColor.byId` (ZERO strategy).
        assert_eq!(base_color_of(0x0010_0000), EntityDyeColor::White);
    }

    #[test]
    fn entity_model_kind_projects_tropical_fish_pattern_and_pattern_color_from_packed_variant() {
        // Vanilla `getPattern(packed) = Pattern.byId(packed & 0xFFFF)` (sparse, default KOB) and
        // `getPatternColor(packed) = DyeColor.byId(packed >> 24 & 0xFF)`. The pattern occupies the
        // low 16 bits and the pattern color the top byte; the base color byte (bits 16..24) must
        // not bleed into either.
        let decode = |packed: i32| match entity_model_kind(
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
            &[protocol_int_data(TROPICAL_FISH_VARIANT_DATA_ID, packed)],
        ) {
            EntityModelKind::TropicalFish {
                shape,
                pattern,
                pattern_color,
                ..
            } => (shape, pattern, pattern_color),
            other => panic!("expected tropical fish, got {other:?}"),
        };

        // All twelve patterns map from their `base.id | index << 8` packed id.
        for (packed_pattern, expected, shape) in [
            (
                0x0000,
                TropicalFishPattern::Kob,
                TropicalFishModelShape::Small,
            ),
            (
                0x0100,
                TropicalFishPattern::Sunstreak,
                TropicalFishModelShape::Small,
            ),
            (
                0x0200,
                TropicalFishPattern::Snooper,
                TropicalFishModelShape::Small,
            ),
            (
                0x0300,
                TropicalFishPattern::Dasher,
                TropicalFishModelShape::Small,
            ),
            (
                0x0400,
                TropicalFishPattern::Brinely,
                TropicalFishModelShape::Small,
            ),
            (
                0x0500,
                TropicalFishPattern::Spotty,
                TropicalFishModelShape::Small,
            ),
            (
                0x0001,
                TropicalFishPattern::Flopper,
                TropicalFishModelShape::Large,
            ),
            (
                0x0101,
                TropicalFishPattern::Stripey,
                TropicalFishModelShape::Large,
            ),
            (
                0x0201,
                TropicalFishPattern::Glitter,
                TropicalFishModelShape::Large,
            ),
            (
                0x0301,
                TropicalFishPattern::Blockfish,
                TropicalFishModelShape::Large,
            ),
            (
                0x0401,
                TropicalFishPattern::Betty,
                TropicalFishModelShape::Large,
            ),
            (
                0x0501,
                TropicalFishPattern::Clayfish,
                TropicalFishModelShape::Large,
            ),
        ] {
            // Mix in a non-zero base color byte (GRAY = 7) and pattern color byte (BLUE = 11) to
            // prove neither disturbs the pattern decode.
            let packed = packed_pattern | (0x07 << 16) | (0x0B << 24);
            let (got_shape, got_pattern, got_pattern_color) = decode(packed);
            assert_eq!(got_pattern, expected);
            assert_eq!(got_shape, shape);
            assert_eq!(got_shape, expected.shape(), "shape mirrors pattern.shape()");
            assert_eq!(got_pattern_color, EntityDyeColor::Blue);
        }

        // Unknown pattern id falls back to KOB (small body) like `ByIdMap.sparse(..., KOB)`.
        let (shape, pattern, _) = decode(0x00AB);
        assert_eq!(pattern, TropicalFishPattern::Kob);
        assert_eq!(shape, TropicalFishModelShape::Small);
    }

    #[test]
    fn entity_model_kind_projects_squid_glow_and_baby_from_data() {
        // The squid and glow squid were placeholder render boxes; they now resolve to the
        // real `SquidModel`. The glow variant is keyed off the entity type id and the baby
        // flag is the synced `AgeableMob.DATA_BABY_ID` (index 16, default adult). The
        // tentacle sweep / body tilt are projected by the world-side squid animation accumulator.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SQUID_ID, &[]),
            EntityModelKind::Squid {
                glow: false,
                baby: false,
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, &[]),
            EntityModelKind::Squid {
                glow: true,
                baby: false,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SQUID_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Squid {
                glow: false,
                baby: true,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Squid {
                glow: true,
                baby: true,
            }
        );
    }

    #[test]
    fn entity_model_kind_projects_phantom_size_from_data() {
        // The phantom was a placeholder render box; it now resolves to the real model and
        // projects its synced `ID_SIZE` (index 16, defaulting to 0).
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PHANTOM_ID, &[]),
            EntityModelKind::Phantom { size: 0 }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_PHANTOM_ID,
                &[protocol_int_data(PHANTOM_SIZE_DATA_ID, 5)]
            ),
            EntityModelKind::Phantom { size: 5 }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_base_cow_and_sheep() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_COW_ID, &[]),
            EntityModelKind::Cow {
                variant: CowModelVariant::Temperate,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_COW_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Cow {
                variant: CowModelVariant::Temperate,
                baby: true
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SHEEP_ID, &[]),
            EntityModelKind::Sheep {
                baby: false,
                sheared: false,
                wool_color: SheepWoolColor::White,
                jeb: false,
                age_ticks: 0.0,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Sheep {
                baby: true,
                sheared: false,
                wool_color: SheepWoolColor::White,
                jeb: false,
                age_ticks: 0.0,
            }
        );
        // The mooshroom shares the cow body, so it renders through the dedicated `Mooshroom` model
        // (the real cow mesh) rather than the generic quadruped stand-in — adult and baby alike. The
        // default variant (no `DATA_TYPE`) is the vanilla `Red`.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_MOOSHROOM_ID, &[]),
            EntityModelKind::Mooshroom {
                baby: false,
                variant: MooshroomVariant::Red,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Mooshroom {
                baby: true,
                variant: MooshroomVariant::Red,
            }
        );
        // The synced `MushroomCow.DATA_TYPE` (index 20) selects the brown coat (id 1; `ByIdMap` CLAMP
        // folds any id ≥ 1 to brown).
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
                &[protocol_int_data(MUSHROOM_COW_TYPE_DATA_ID, 1)]
            ),
            EntityModelKind::Mooshroom {
                baby: false,
                variant: MooshroomVariant::Brown,
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_vanilla_sheep_wool_metadata() {
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
                &[protocol_byte_data(
                    SHEEP_WOOL_DATA_ID,
                    (SHEEP_WOOL_SHEARED_FLAG | 14) as i8
                )]
            ),
            EntityModelKind::Sheep {
                baby: false,
                sheared: true,
                wool_color: SheepWoolColor::Red,
                jeb: false,
                age_ticks: 0.0,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
                &[
                    protocol_byte_data(SHEEP_WOOL_DATA_ID, 15),
                    protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
                ]
            ),
            EntityModelKind::Sheep {
                baby: true,
                sheared: false,
                wool_color: SheepWoolColor::Black,
                jeb: false,
                age_ticks: 0.0,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
                &[
                    protocol_byte_data(SHEEP_WOOL_DATA_ID, 3),
                    protocol_byte_data(SHEEP_WOOL_DATA_ID, (SHEEP_WOOL_SHEARED_FLAG | 5) as i8),
                ]
            ),
            EntityModelKind::Sheep {
                baby: false,
                sheared: true,
                wool_color: SheepWoolColor::Lime,
                jeb: false,
                age_ticks: 0.0,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
                &[
                    protocol_byte_data(ENTITY_SHARED_FLAGS_DATA_ID, ENTITY_SHARED_FLAG_INVISIBLE),
                    protocol_byte_data(SHEEP_WOOL_DATA_ID, 14),
                ]
            ),
            EntityModelKind::Sheep {
                baby: false,
                sheared: false,
                wool_color: SheepWoolColor::Red,
                jeb: false,
                age_ticks: 0.0,
            }
        );
    }

    #[test]
    fn entity_model_kind_projects_sheep_jeb_custom_name_and_age() {
        assert_eq!(
            entity_model_kind_with_time_and_registries(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
                &[
                    protocol_optional_component_data(ENTITY_CUSTOM_NAME_DATA_ID, Some("jeb_")),
                    protocol_byte_data(SHEEP_WOOL_DATA_ID, 0),
                ],
                12.5,
                0,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            EntityModelKind::Sheep {
                baby: false,
                sheared: false,
                wool_color: SheepWoolColor::White,
                jeb: true,
                age_ticks: 12.5,
            }
        );
        assert_eq!(
            entity_model_kind_with_time_and_registries(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
                &[protocol_optional_component_data(
                    ENTITY_CUSTOM_NAME_DATA_ID,
                    Some("Not jeb_"),
                )],
                25.0,
                0,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            EntityModelKind::Sheep {
                baby: false,
                sheared: false,
                wool_color: SheepWoolColor::White,
                jeb: false,
                age_ticks: 25.0,
            }
        );
        assert_eq!(
            entity_model_kind_with_time_and_registries(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
                &[protocol_optional_component_data(
                    ENTITY_CUSTOM_NAME_DATA_ID,
                    None
                )],
                25.0,
                0,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            EntityModelKind::Sheep {
                baby: false,
                sheared: false,
                wool_color: SheepWoolColor::White,
                jeb: false,
                age_ticks: 25.0,
            }
        );
    }

    #[test]
    fn entity_model_instances_project_sheep_wool_metadata_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            111,
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            [1.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 111,
            values: vec![protocol_byte_data(
                SHEEP_WOOL_DATA_ID,
                (SHEEP_WOOL_SHEARED_FLAG | 14) as i8,
            )],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![EntityModelInstance::sheep_render_state(
                    111,
                    [1.0, 64.0, -2.0],
                    0.0,
                    false,
                    true,
                    SheepWoolColor::Red,
                    false,
                    false,
                    1.0,
                )],
                1.0,
            )
        );
        // A visible entity (no invisible shared flag) projects `invisible == false`.
        assert!(!instances[0].render_state.invisible);
    }

    #[test]
    fn entity_model_instances_project_sheep_jeb_custom_name_and_age_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            112,
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            [1.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 112,
            values: vec![protocol_optional_component_data(
                ENTITY_CUSTOM_NAME_DATA_ID,
                Some("jeb_"),
            )],
        }));
        world.advance_entity_client_animations(12);

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5);

        assert_eq!(
            instances,
            aged(
                vec![EntityModelInstance::sheep_render_state(
                    112,
                    [1.0, 64.0, -2.0],
                    0.0,
                    false,
                    false,
                    SheepWoolColor::White,
                    false,
                    true,
                    12.5,
                )],
                12.5,
            )
        );
    }

    #[test]
    fn entity_model_instances_project_sheep_invisible_shared_flag_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            113,
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            [1.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 113,
            values: vec![
                protocol_byte_data(ENTITY_SHARED_FLAGS_DATA_ID, ENTITY_SHARED_FLAG_INVISIBLE),
                protocol_byte_data(SHEEP_WOOL_DATA_ID, 14),
            ],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);

        assert_eq!(
            instances,
            aged(
                vec![EntityModelInstance::sheep_render_state(
                    113,
                    [1.0, 64.0, -2.0],
                    0.0,
                    false,
                    false,
                    SheepWoolColor::Red,
                    true,
                    false,
                    0.25,
                )],
                0.25,
            )
        );
        // The shared invisible flag is now projected uniformly into the render state.
        assert!(instances[0].render_state.invisible);
        assert!(instances[0].render_state.invisible_to_player);
    }

    #[test]
    fn entity_model_instances_project_spectator_visible_invisible_sheep_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            113,
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            [1.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 113,
            values: vec![
                protocol_byte_data(ENTITY_SHARED_FLAGS_DATA_ID, ENTITY_SHARED_FLAG_INVISIBLE),
                protocol_byte_data(SHEEP_WOOL_DATA_ID, 14),
            ],
        }));
        world.apply_game_event(bbb_protocol::packets::GameEvent {
            event_id: 3,
            param: 3.0,
        });

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);

        assert_eq!(instances.len(), 1);
        assert!(instances[0].render_state.invisible);
        assert!(!instances[0].render_state.invisible_to_player);
    }

    #[test]
    fn entity_model_instances_project_glowing_shared_flag_from_world() {
        let mut world = WorldStore::new();
        let sheep = protocol_add_entity(113, VANILLA_ENTITY_TYPE_SHEEP_ID, [1.0, 64.0, -2.0]);
        let sheep_uuid = sheep.uuid;
        world.apply_add_entity(sheep);
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 113,
            values: vec![
                protocol_byte_data(
                    ENTITY_SHARED_FLAGS_DATA_ID,
                    ENTITY_SHARED_FLAG_INVISIBLE | ENTITY_SHARED_FLAG_GLOWING,
                ),
                protocol_byte_data(SHEEP_WOOL_DATA_ID, 14),
            ],
        }));
        assert!(world.apply_set_player_team(SetPlayerTeam {
            name: "green".to_string(),
            method: PlayerTeamMethod::Add,
            parameters: Some(PlayerTeamParameters {
                display_name: "Green".to_string(),
                options: 0,
                nametag_visibility: TeamVisibility::Always,
                collision_rule: TeamCollisionRule::Always,
                color: ChatFormatting::Green,
                player_prefix: String::new(),
                player_suffix: String::new(),
            }),
            players: vec![sheep_uuid.to_string()],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);

        assert_eq!(instances.len(), 1);
        assert!(instances[0].render_state.invisible);
        assert!(instances[0].render_state.invisible_to_player);
        assert!(instances[0].render_state.appears_glowing);
        assert_eq!(instances[0].render_state.outline_color, 0xff55_ff55);
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_goats() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_GOAT_ID, &[]),
            EntityModelKind::Goat {
                baby: false,
                left_horn: true,
                right_horn: true,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_GOAT_ID,
                &[
                    protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
                    protocol_bool_data(GOAT_LEFT_HORN_DATA_ID, false),
                    protocol_bool_data(GOAT_RIGHT_HORN_DATA_ID, true),
                ]
            ),
            EntityModelKind::Goat {
                baby: true,
                left_horn: false,
                right_horn: true,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_GOAT_ID,
                &[
                    protocol_bool_data(GOAT_LEFT_HORN_DATA_ID, false),
                    protocol_bool_data(GOAT_RIGHT_HORN_DATA_ID, false),
                ]
            ),
            EntityModelKind::Goat {
                baby: false,
                left_horn: false,
                right_horn: false,
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_hoglins_and_zoglins() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_HOGLIN_ID, &[]),
            EntityModelKind::Hoglin {
                family: HoglinModelFamily::Hoglin,
                baby: false,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_HOGLIN_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Hoglin {
                family: HoglinModelFamily::Hoglin,
                baby: true,
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ZOGLIN_ID, &[]),
            EntityModelKind::Hoglin {
                family: HoglinModelFamily::Zoglin,
                baby: false,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ZOGLIN_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Hoglin {
                family: HoglinModelFamily::Zoglin,
                baby: true,
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_ravagers() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_RAVAGER_ID, &[]),
            EntityModelKind::Ravager
        );
    }

    #[test]
    fn entity_model_kind_maps_sniffer_to_real_model() {
        // The sniffer was approximated by the cow quadruped model; it now resolves to the real
        // `SnifferModel` on the adult `ModelLayers.SNIFFER` or baby `ModelLayers.SNIFFER_BABY`
        // baked layer. Vanilla keys the baby renderer off `AgeableMob.DATA_BABY_ID` (index 16) and
        // uses `snifflet.png` while still constructing a `SnifferModel`.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SNIFFER_ID, &[]),
            EntityModelKind::Sniffer { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SNIFFER_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Sniffer { baby: true }
        );
    }

    #[test]
    fn entity_model_kind_maps_warden_to_real_model() {
        // The warden was a placeholder bounds box; it now resolves to the real `WardenModel`. The
        // head look, walk, idle wobble, and tendril sway are driven by projected render state (age,
        // walk, head look, and the event-driven tendril pulse), not by this kind mapping; the attack
        // / sonic-boom / digging / emerge / roar / sniff keyframe animations and the four emissive
        // overlay layers stay deferred. The kind mapping itself reads no synced data.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_WARDEN_ID, &[]),
            EntityModelKind::Warden
        );
    }

    #[test]
    fn entity_model_kind_projects_armadillo_baby_from_data() {
        // The armadillo was a placeholder bounds box; it now resolves to the real
        // `AdultArmadilloModel` / `BabyArmadilloModel`, keyed off the synced `AgeableMob.DATA_BABY_ID`
        // (index 16, default adult), as in the vanilla `AgeableMobRenderer`. The clamped head look,
        // `applyWalk` leg sway, and the roll-out / roll-up / peek keyframe transitions are projected
        // separately from world animation state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ARMADILLO_ID, &[]),
            EntityModelKind::Armadillo {
                baby: false,
                rolled_up: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ARMADILLO_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Armadillo {
                baby: true,
                rolled_up: false
            }
        );
    }

    #[test]
    fn entity_model_kind_projects_armadillo_rolled_up_from_state() {
        // Vanilla `Armadillo.ARMADILLO_STATE` (data id 18, `ArmadilloState` enum). Only the steady
        // SCARED state (id 2) is `shouldHideInShell` for every tick, so it maps to `rolled_up`; the
        // tick-gated ROLLING (1) / UNROLLING (3) transitions and IDLE (0) stay not-rolled-up.
        let kind = |id: i32| {
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ARMADILLO_ID,
                &[protocol_armadillo_state_data(id)],
            )
        };
        assert_eq!(
            kind(2),
            EntityModelKind::Armadillo {
                baby: false,
                rolled_up: true
            }
        );
        for non_scared in [0, 1, 3] {
            assert_eq!(
                kind(non_scared),
                EntityModelKind::Armadillo {
                    baby: false,
                    rolled_up: false
                },
                "state {non_scared} is not the steady SCARED ball"
            );
        }

        // A baby armadillo can roll up too — the state and the baby flag compose.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ARMADILLO_ID,
                &[
                    protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
                    protocol_armadillo_state_data(2),
                ]
            ),
            EntityModelKind::Armadillo {
                baby: true,
                rolled_up: true
            }
        );
    }

    #[test]
    fn entity_model_kind_projects_axolotl_baby_from_data() {
        // The axolotl was a placeholder bounds box; it now resolves to the real `AdultAxolotlModel`
        // / `BabyAxolotlModel`, keyed off the synced `AgeableMob.DATA_BABY_ID` (index 16, default
        // adult), as in the vanilla `AgeableMobRenderer`, and textured by the `Axolotl.Variant`
        // colour read from `DATA_VARIANT` (index 18). The body yaw, the procedural / keyframe
        // swim-walk-idle animations, the play-dead pose, and the mirror-leg copy stay deferred.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_AXOLOTL_ID, &[]),
            EntityModelKind::Axolotl {
                baby: false,
                variant: AxolotlModelVariant::Lucy
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_AXOLOTL_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Axolotl {
                baby: true,
                variant: AxolotlModelVariant::Lucy
            }
        );
        // `DATA_VARIANT` (18, int) selects the colour via `Axolotl.Variant.byId`.
        for (id, variant) in [
            (0, AxolotlModelVariant::Lucy),
            (1, AxolotlModelVariant::Wild),
            (2, AxolotlModelVariant::Gold),
            (3, AxolotlModelVariant::Cyan),
            (4, AxolotlModelVariant::Blue),
            (5, AxolotlModelVariant::Lucy),
        ] {
            assert_eq!(
                entity_model_kind(
                    VANILLA_ENTITY_TYPE_AXOLOTL_ID,
                    &[protocol_int_data(AXOLOTL_VARIANT_DATA_ID, id)]
                ),
                EntityModelKind::Axolotl {
                    baby: false,
                    variant
                }
            );
        }
    }

    #[test]
    fn entity_model_kind_maps_tadpole_to_real_model() {
        // The tadpole was a placeholder bounds box; it now resolves to the real `TadpoleModel` at
        // its rest pose. The tail yaw sway is deferred entity-side state, so no synced data is read
        // (the tadpole is an `AbstractFish`, not an `AgeableMob`, so it carries no baby flag).
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_TADPOLE_ID, &[]),
            EntityModelKind::Tadpole
        );
    }

    #[test]
    fn entity_model_kind_maps_parrot_to_real_model() {
        // The parrot resolves to the real `ParrotModel` at its STANDING rest pose, textured per the
        // five `Parrot.Variant` colours read from the synced `DATA_VARIANT_ID` (19, INT) via
        // `Parrot.Variant.byId`. The head look, per-pose offsets, and wing flap / dance animations are
        // deferred entity-side state.
        let parrot_variant = |id: i32| EntityDataValue {
            data_id: PARROT_VARIANT_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(id),
        };
        // No synced data → the vanilla DEFAULT (RED_BLUE).
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PARROT_ID, &[]),
            EntityModelKind::Parrot {
                variant: ParrotModelVariant::RedBlue,
            }
        );
        // Each id selects its colour; out-of-range folds back to RED_BLUE.
        for (id, variant) in [
            (0, ParrotModelVariant::RedBlue),
            (1, ParrotModelVariant::Blue),
            (2, ParrotModelVariant::Green),
            (3, ParrotModelVariant::YellowBlue),
            (4, ParrotModelVariant::Gray),
            (99, ParrotModelVariant::RedBlue),
        ] {
            assert_eq!(
                entity_model_kind(VANILLA_ENTITY_TYPE_PARROT_ID, &[parrot_variant(id)]),
                EntityModelKind::Parrot { variant }
            );
        }
    }

    #[test]
    fn entity_model_kind_maps_shulker_to_real_model() {
        // The shulker resolves to the real `ShulkerModel` at its closed rest pose, textured by the
        // dye colour read from `DATA_COLOR_ID` (18). With no synced colour it defaults to the
        // uncolored texture (`None`). The peek open/close, head look, and attach-face rotation stay
        // deferred entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SHULKER_ID, &[]),
            EntityModelKind::Shulker { color: None }
        );
        // Byte 16 (the vanilla default) is the uncolored shulker; 0..=15 select a dye.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SHULKER_ID,
                &[protocol_byte_data(SHULKER_COLOR_DATA_ID, 16)]
            ),
            EntityModelKind::Shulker { color: None }
        );
        for (id, color) in [
            (0, EntityDyeColor::White),
            (4, EntityDyeColor::Yellow),
            (11, EntityDyeColor::Blue),
            (15, EntityDyeColor::Black),
        ] {
            assert_eq!(
                entity_model_kind(
                    VANILLA_ENTITY_TYPE_SHULKER_ID,
                    &[protocol_byte_data(SHULKER_COLOR_DATA_ID, id)]
                ),
                EntityModelKind::Shulker { color: Some(color) }
            );
        }
    }

    #[test]
    fn entity_model_kind_maps_wither_to_real_model() {
        // The wither was a placeholder bounds box; it now resolves to the real `WitherBossModel` at
        // its bind rest pose. The procedural ribcage/tail breathing sway, the head look, and the
        // invulnerable-shimmer overlay are deferred entity-side state, so no synced data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_WITHER_ID, &[]),
            EntityModelKind::Wither
        );
    }

    #[test]
    fn entity_model_kind_maps_giant_to_real_model() {
        // The giant was a placeholder bounds box; it now resolves to the real `GiantZombieModel`
        // (the humanoid zombie body layer scaled 6×). The head look and limb swing read the
        // projected look angles and walk animation; renderer/native layer paths cover the zombie
        // texture, armor, and held items. The giant is never a baby, so no baby flag is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_GIANT_ID, &[]),
            EntityModelKind::Giant
        );
    }

    #[test]
    fn entity_model_kind_maps_end_crystal_to_real_model() {
        // The end crystal was a placeholder bounds box; it now resolves to the real `EndCrystalModel`
        // at its rest pose. The model kind itself reads no synced data; age, `showsBottom`, and the
        // optional `DATA_BEAM_TARGET` custom beam are projected into render state separately.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_END_CRYSTAL_ID, &[]),
            EntityModelKind::EndCrystal
        );
    }

    #[test]
    fn entity_model_kind_maps_evoker_fangs_to_real_model() {
        // The evoker fangs were a placeholder bounds box; they now resolve to the real
        // `EvokerFangsModel` at the closed-jaw rest pose. The bite animation, the base drop, and the
        // emerge scale are deferred entity-side state, so no synced data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID, &[]),
            EntityModelKind::EvokerFangs
        );
    }

    #[test]
    fn entity_model_kind_maps_leash_knot_to_real_model() {
        // The leash knot was a placeholder bounds box; it now resolves to the real `LeashKnotModel`.
        // The model has no animation, so the geometry is complete; no synced data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_LEASH_KNOT_ID, &[]),
            EntityModelKind::LeashKnot
        );
    }

    #[test]
    fn entity_model_kind_maps_arrows_to_real_model() {
        // The arrow and spectral arrow were placeholder boxes; they now resolve to the real
        // `ArrowModel`, sharing one model but binding different images. A plain arrow is `Normal`; a
        // tipped arrow (`ID_EFFECT_COLOR` 11 > 0) is `Tipped`; the spectral arrow type is `Spectral`.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ARROW_ID, &[]),
            EntityModelKind::Arrow {
                texture: ArrowModelTexture::Normal
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ARROW_ID,
                &[protocol_int_data(ARROW_EFFECT_COLOR_DATA_ID, 0x385dc6)]
            ),
            EntityModelKind::Arrow {
                texture: ArrowModelTexture::Tipped
            }
        );
        // A potionless arrow (`getColor()` returns the `-1` sentinel) is not tipped.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ARROW_ID,
                &[protocol_int_data(ARROW_EFFECT_COLOR_DATA_ID, -1)]
            ),
            EntityModelKind::Arrow {
                texture: ArrowModelTexture::Normal
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID, &[]),
            EntityModelKind::Arrow {
                texture: ArrowModelTexture::Spectral
            }
        );
    }

    #[test]
    fn entity_model_instances_project_arrow_impact_shake() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            60,
            VANILLA_ENTITY_TYPE_ARROW_ID,
            [1.0, 64.0, -2.0],
        ));
        world.advance_entity_client_animations(1);
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 60,
            values: vec![protocol_bool_data(ARROW_IN_GROUND_DATA_ID, true)],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);
        let arrow = instances
            .iter()
            .find(|instance| instance.entity_id == 60)
            .expect("arrow instance");
        assert_eq!(
            arrow.kind,
            EntityModelKind::Arrow {
                texture: ArrowModelTexture::Normal
            }
        );
        assert_eq!(arrow.render_state.arrow_shake, 6.75);
    }

    #[test]
    fn entity_model_kind_maps_trident_to_real_model() {
        // The thrown trident was a placeholder box; it now resolves to the real `TridentModel`. The
        // model has no animation, so the geometry is complete; the foil flag is projected onto render
        // state separately, so model kind selection still reads no synced data.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_TRIDENT_ID, &[]),
            EntityModelKind::Trident
        );
    }

    #[test]
    fn entity_model_instances_project_thrown_trident_foil_flag() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            135,
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
            [1.0, 64.0, -2.0],
        ));

        let default_instances =
            entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
        let trident = default_instances
            .iter()
            .find(|instance| instance.entity_id == 135)
            .expect("trident instance");
        assert_eq!(trident.kind, EntityModelKind::Trident);
        assert!(!trident.render_state.trident_foil);

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 135,
            values: vec![protocol_bool_data(TRIDENT_FOIL_DATA_ID, true)],
        }));

        let foiled_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
        let trident = foiled_instances
            .iter()
            .find(|instance| instance.entity_id == 135)
            .expect("foiled trident instance");
        assert_eq!(trident.kind, EntityModelKind::Trident);
        assert!(trident.render_state.trident_foil);
    }

    #[test]
    fn entity_model_kind_skips_thrown_item_projectiles_for_the_billboard_layer() {
        // The thrown-item projectiles (vanilla `ThrownItemRenderer`) render as a camera-facing item
        // sprite via the item-entity billboard layer, so the 3D model scene draws nothing for them — the
        // model kind is `NoRender` rather than the former placeholder box.
        for &(type_id, _scale) in THROWN_ITEM_PROJECTILE_BILLBOARDS {
            assert_eq!(
                entity_model_kind(type_id, &[]),
                EntityModelKind::NoRender,
                "thrown-item projectile type {type_id} should be NoRender",
            );
        }
    }

    #[test]
    fn entity_model_kind_maps_wither_skull_to_real_model() {
        // The wither skull was a placeholder box; it now resolves to the real `SkullModel`. Its flight
        // facing comes from the projected yaw/pitch (a plain `EntityRenderer`). Vanilla
        // `WitherSkull.DATA_DANGEROUS` is the synced boolean at id 8 and swaps to
        // `wither_invulnerable.png`.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_WITHER_SKULL_ID, &[]),
            EntityModelKind::WitherSkull { dangerous: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
                &[protocol_bool_data(WITHER_SKULL_DANGEROUS_DATA_ID, true)]
            ),
            EntityModelKind::WitherSkull { dangerous: true }
        );
    }

    #[test]
    fn entity_model_kind_maps_llama_spit_to_real_model() {
        // The llama spit was a placeholder box; it now resolves to the real `LlamaSpitModel`. The
        // model has no `setupAnim`, so the geometry is complete; only the texture is deferred
        // entity-side state, so no synced data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID, &[]),
            EntityModelKind::LlamaSpit
        );
    }

    #[test]
    fn entity_model_kind_maps_shulker_bullet_to_real_model() {
        // The shulker bullet was a placeholder box; it now resolves to the real `ShulkerBulletModel`.
        // Its facing comes from the projected yaw/pitch; the age-driven tumble and the translucent
        // outer-shell pass are deferred entity-side state, so no synced data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID, &[]),
            EntityModelKind::ShulkerBullet
        );
    }

    #[test]
    fn entity_model_kind_maps_wind_charges_to_real_model() {
        // The wind charge and breeze wind charge were placeholder boxes; both share the real
        // `WindChargeModel` (vanilla registers `WindChargeRenderer` for both). The counter-rotation,
        // the scrolling translucent texture, and the texture-backed path are deferred entity-side
        // state, so no synced data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_WIND_CHARGE_ID, &[]),
            EntityModelKind::WindCharge
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID, &[]),
            EntityModelKind::WindCharge
        );
    }

    #[test]
    fn entity_model_kind_maps_ender_dragon_to_real_model() {
        // The ender dragon was a placeholder bounds box; it now resolves to the real
        // `EnderDragonModel` at its bind layout. The fully procedural flight animation, the dying
        // dissolve, and the nearest-crystal healing beam are deferred entity-side state; the emissive
        // eyes pass is renderer-owned and no synced data is read here.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, &[]),
            EntityModelKind::EnderDragon
        );
    }

    #[test]
    fn entity_model_kind_renders_nothing_for_noop_renderer_entities() {
        // The area effect cloud, marker, and interaction use vanilla `NoopRenderer` — they render no
        // model — so they resolve to `EntityModelKind::NoRender`, replacing the former placeholder
        // boxes (which incorrectly drew a debug box where vanilla draws nothing).
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, &[]),
            EntityModelKind::NoRender
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_INTERACTION_ID, &[]),
            EntityModelKind::NoRender
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_MARKER_ID, &[]),
            EntityModelKind::NoRender
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_polar_bears() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, &[]),
            EntityModelKind::PolarBear { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::PolarBear { baby: true }
        );
        // The panda (adult and baby) renders through its dedicated `PandaModel` / `BabyPandaModel`;
        // with no gene metadata the displayed variant is the vanilla default `NORMAL`.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PANDA_ID, &[]),
            EntityModelKind::Panda {
                baby: false,
                variant: PandaModelVariant::Normal
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_PANDA_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Panda {
                baby: true,
                variant: PandaModelVariant::Normal
            }
        );
    }

    #[test]
    fn entity_model_kind_projects_panda_gene_variant_from_data() {
        // Vanilla `Panda.getVariant()` = `Gene.getVariantFromGenes(mainGene, hiddenGene)` off the two
        // synced gene bytes (21/22). A dominant main gene always shows.
        assert_eq!(
            panda_model_kind(&[
                protocol_byte_data(PANDA_MAIN_GENE_DATA_ID, 6),
                protocol_byte_data(PANDA_HIDDEN_GENE_DATA_ID, 0),
            ]),
            EntityModelKind::Panda {
                baby: false,
                variant: PandaModelVariant::Aggressive
            }
        );
        // A recessive main gene (BROWN=4) shows only when both genes match.
        assert_eq!(
            panda_model_kind(&[
                protocol_byte_data(PANDA_MAIN_GENE_DATA_ID, 4),
                protocol_byte_data(PANDA_HIDDEN_GENE_DATA_ID, 4),
            ]),
            EntityModelKind::Panda {
                baby: false,
                variant: PandaModelVariant::Brown
            }
        );
        // An unmatched recessive main gene falls back to NORMAL.
        assert_eq!(
            panda_model_kind(&[
                protocol_byte_data(PANDA_MAIN_GENE_DATA_ID, 4),
                protocol_byte_data(PANDA_HIDDEN_GENE_DATA_ID, 1),
            ]),
            EntityModelKind::Panda {
                baby: false,
                variant: PandaModelVariant::Normal
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_villagers() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_VILLAGER_ID, &[]),
            EntityModelKind::Villager { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Villager { baby: true }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, &[]),
            EntityModelKind::WanderingTrader
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::WanderingTrader
        );
    }

    #[test]
    fn villager_model_data_reads_vanilla_serializer_and_static_fallback_order() {
        assert_eq!(
            villager_model_data(VANILLA_ENTITY_TYPE_VILLAGER_ID, &[], None, None),
            VillagerModelData::DEFAULT
        );
        assert_eq!(
            villager_model_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                &[protocol_villager_data(18, 6, 14, 9)],
                None,
                None,
            ),
            VillagerModelData::DEFAULT,
            "id 18 is AbstractVillager.DATA_UNHAPPY_COUNTER, not Villager.DATA_VILLAGER_DATA"
        );
        assert_eq!(
            villager_model_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                &[protocol_villager_data(VILLAGER_DATA_DATA_ID, 6, 14, 9)],
                None,
                None,
            ),
            VillagerModelData::new(
                VillagerModelType::Taiga,
                VillagerModelProfession::Weaponsmith,
                9,
            )
        );
        assert_eq!(
            villager_model_data(
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
                &[protocol_villager_data(
                    ZOMBIE_VILLAGER_DATA_DATA_ID,
                    4,
                    11,
                    2
                )],
                None,
                None,
            ),
            VillagerModelData::new(VillagerModelType::Snow, VillagerModelProfession::Nitwit, 2,)
        );
        assert_eq!(
            villager_model_data(
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
                &[protocol_villager_data(VILLAGER_DATA_DATA_ID, 4, 11, 2)],
                None,
                None,
            ),
            VillagerModelData::DEFAULT,
            "zombie villager data lives at id 20, not the villager id 19"
        );
    }

    #[test]
    fn villager_model_data_prefers_dynamic_registry_order() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            "minecraft:villager_type",
            0,
            vec![
                RegistryPacketEntry::stub("minecraft:swamp"),
                RegistryPacketEntry::stub("minecraft:desert"),
            ],
        );
        world.record_registry_entries(
            "minecraft:villager_profession",
            0,
            vec![
                RegistryPacketEntry::stub("minecraft:librarian"),
                RegistryPacketEntry::stub("minecraft:farmer"),
            ],
        );
        assert_eq!(
            villager_model_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                &[protocol_villager_data(VILLAGER_DATA_DATA_ID, 1, 0, 5)],
                world.registry_content("minecraft:villager_type"),
                world.registry_content("minecraft:villager_profession"),
            ),
            VillagerModelData::new(
                VillagerModelType::Desert,
                VillagerModelProfession::Librarian,
                5,
            )
        );
        assert_eq!(
            villager_model_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                &[protocol_villager_data(VILLAGER_DATA_DATA_ID, 99, -1, 3)],
                world.registry_content("minecraft:villager_type"),
                world.registry_content("minecraft:villager_profession"),
            ),
            VillagerModelData::new(VillagerModelType::Plains, VillagerModelProfession::None, 3,)
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_wolves() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_WOLF_ID, &[]),
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                collar_color: None,
                variant: WolfModelVariant::Pale,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Wolf {
                baby: true,
                tame: false,
                angry: false,
                collar_color: None,
                variant: WolfModelVariant::Pale,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                &[protocol_byte_data(
                    TAMABLE_ANIMAL_FLAGS_DATA_ID,
                    TAMABLE_ANIMAL_TAME_FLAG
                )]
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: true,
                angry: false,
                collar_color: Some(EntityDyeColor::Red),
                variant: WolfModelVariant::Pale,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                &[
                    protocol_byte_data(TAMABLE_ANIMAL_FLAGS_DATA_ID, TAMABLE_ANIMAL_TAME_FLAG),
                    protocol_int_data(WOLF_COLLAR_COLOR_DATA_ID, 11),
                ]
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: true,
                angry: false,
                collar_color: Some(EntityDyeColor::Blue),
                variant: WolfModelVariant::Pale,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                &[
                    protocol_byte_data(ENTITY_SHARED_FLAGS_DATA_ID, ENTITY_SHARED_FLAG_INVISIBLE),
                    protocol_byte_data(TAMABLE_ANIMAL_FLAGS_DATA_ID, TAMABLE_ANIMAL_TAME_FLAG),
                    protocol_int_data(WOLF_COLLAR_COLOR_DATA_ID, 11),
                ]
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: true,
                angry: false,
                collar_color: Some(EntityDyeColor::Blue),
                variant: WolfModelVariant::Pale,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                &[protocol_int_data(WOLF_COLLAR_COLOR_DATA_ID, 11)]
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                collar_color: None,
                variant: WolfModelVariant::Pale,
            }
        );
        // The adult cat, ocelot, and fox render through their dedicated models (cat = the shared
        // `AdultFelineModel` scaled 0.8, ocelot = the unscaled feline, fox = `AdultFoxModel`); each baby
        // now renders through its own dedicated vanilla mesh.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_CAT_ID, &[]),
            EntityModelKind::Feline {
                cat: true,
                baby: false,
                cat_variant: CatModelVariant::Black,
                collar: None
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_OCELOT_ID, &[]),
            EntityModelKind::Feline {
                cat: false,
                baby: false,
                cat_variant: CatModelVariant::Black,
                collar: None
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_FOX_ID, &[]),
            EntityModelKind::Fox {
                baby: false,
                variant: FoxModelVariant::Red
            }
        );
        // The cat/ocelot babies now render through the dedicated `BabyFelineModel` layout, as does the
        // fox baby through its own `BabyFoxModel`.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_CAT_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Feline {
                cat: true,
                baby: true,
                cat_variant: CatModelVariant::Black,
                collar: None
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_OCELOT_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Feline {
                cat: false,
                baby: true,
                cat_variant: CatModelVariant::Black,
                collar: None
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_FOX_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Fox {
                baby: true,
                variant: FoxModelVariant::Red
            }
        );
        // The fox `DATA_TYPE_ID` (18, int) selects the RED/SNOW variant via `Fox.Variant.byId`.
        for (id, variant) in [
            (0, FoxModelVariant::Red),
            (1, FoxModelVariant::Snow),
            (2, FoxModelVariant::Red),
        ] {
            assert_eq!(
                entity_model_kind(
                    VANILLA_ENTITY_TYPE_FOX_ID,
                    &[protocol_int_data(FOX_TYPE_DATA_ID, id)]
                ),
                EntityModelKind::Fox {
                    baby: false,
                    variant
                }
            );
        }
        // The rabbit (adult and baby) renders through its dedicated `AdultRabbitModel` / `BabyRabbitModel`,
        // textured by the `Rabbit.Variant` colour (`DATA_TYPE_ID`, 18) plus the Toast name override.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_RABBIT_ID, &[]),
            EntityModelKind::Rabbit {
                baby: false,
                variant: RabbitModelVariant::Brown,
                toast: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_RABBIT_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Rabbit {
                baby: true,
                variant: RabbitModelVariant::Brown,
                toast: false
            }
        );
        // `DATA_TYPE_ID` (18, int) selects the colour via `Rabbit.Variant.byId` (sparse; EVIL = 99).
        for (id, variant) in [
            (0, RabbitModelVariant::Brown),
            (1, RabbitModelVariant::White),
            (2, RabbitModelVariant::Black),
            (3, RabbitModelVariant::WhiteSplotched),
            (4, RabbitModelVariant::Gold),
            (5, RabbitModelVariant::Salt),
            (99, RabbitModelVariant::Evil),
            (7, RabbitModelVariant::Brown),
        ] {
            assert_eq!(
                entity_model_kind(
                    VANILLA_ENTITY_TYPE_RABBIT_ID,
                    &[protocol_int_data(RABBIT_TYPE_DATA_ID, id)]
                ),
                EntityModelKind::Rabbit {
                    baby: false,
                    variant,
                    toast: false
                }
            );
        }
        // The custom name "Toast" flips the toast override; any other name does not.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_RABBIT_ID,
                &[protocol_optional_component_data(
                    ENTITY_CUSTOM_NAME_DATA_ID,
                    Some("Toast")
                )]
            ),
            EntityModelKind::Rabbit {
                baby: false,
                variant: RabbitModelVariant::Brown,
                toast: true
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_RABBIT_ID,
                &[protocol_optional_component_data(
                    ENTITY_CUSTOM_NAME_DATA_ID,
                    Some("toast")
                )]
            ),
            EntityModelKind::Rabbit {
                baby: false,
                variant: RabbitModelVariant::Brown,
                toast: false
            }
        );
    }

    #[test]
    fn entity_model_kind_projects_wolf_variant_from_registry_and_fallback() {
        // Vanilla `WolfRenderer` keys the texture on the synced `Wolf.DATA_VARIANT_ID` (index 23)
        // registry holder. The dynamic `wolf_variant` registry order the server sent wins; without it
        // the static `WolfVariants.bootstrap` order is the fallback. The default is `Pale`.
        let mut world = WorldStore::new();
        world.record_registry_entries(
            "minecraft:wolf_variant",
            0,
            vec![
                RegistryPacketEntry::stub("minecraft:striped"),
                RegistryPacketEntry::stub("minecraft:ashen"),
                RegistryPacketEntry::stub("minecraft:woods"),
            ],
        );
        let wolf_registry = world.registry_content("minecraft:wolf_variant").unwrap();

        // Registry id 1 → the second entry the server declared (`ashen`).
        assert_eq!(
            entity_model_kind_with_registries(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                &[protocol_wolf_variant_data(1)],
                None,
                None,
                None,
                None,
                None,
                Some(wolf_registry),
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                collar_color: None,
                variant: WolfModelVariant::Ashen,
            }
        );

        // No dynamic registry → the static vanilla order: id 3 is `black`.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                &[protocol_wolf_variant_data(3)]
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                collar_color: None,
                variant: WolfModelVariant::Black,
            }
        );

        // No variant holder at all → the vanilla `WolfVariants.DEFAULT` (`Pale`).
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_WOLF_ID, &[]),
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                collar_color: None,
                variant: WolfModelVariant::Pale,
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_vanilla_cat_variant_metadata() {
        // Without the dynamic `cat_variant` registry the bootstrap order (tabby=0..all_black=10) is
        // the static fallback; the vanilla default is BLACK. The ocelot has no breed.
        for (id, variant) in [
            (0, CatModelVariant::Tabby),
            (1, CatModelVariant::Black),
            (2, CatModelVariant::Red),
            (3, CatModelVariant::Siamese),
            (4, CatModelVariant::BritishShorthair),
            (5, CatModelVariant::Calico),
            (6, CatModelVariant::Persian),
            (7, CatModelVariant::Ragdoll),
            (8, CatModelVariant::White),
            (9, CatModelVariant::Jellie),
            (10, CatModelVariant::AllBlack),
            (99, CatModelVariant::Black),
        ] {
            assert_eq!(
                entity_model_kind(VANILLA_ENTITY_TYPE_CAT_ID, &[protocol_cat_variant_data(id)]),
                EntityModelKind::Feline {
                    cat: true,
                    baby: false,
                    cat_variant: variant,
                    collar: None
                }
            );
        }
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_OCELOT_ID,
                &[protocol_cat_variant_data(0)]
            ),
            EntityModelKind::Feline {
                cat: false,
                baby: false,
                cat_variant: CatModelVariant::Black,
                collar: None
            }
        );
    }

    #[test]
    fn entity_model_kind_projects_cat_collar_from_tame_and_color() {
        // Vanilla `CatRenderer`: `state.collarColor = isTame() ? getCollarColor() : null`, and
        // `getCollarColor() = DyeColor.byId(DATA_COLLAR_COLOR)` (default RED). The ocelot has no collar.
        fn collar_of(kind: &EntityModelKind) -> Option<EntityDyeColor> {
            match kind {
                EntityModelKind::Feline { collar, .. } => *collar,
                other => panic!("expected feline, got {other:?}"),
            }
        }

        // An untamed cat carries no collar even with a color set.
        assert_eq!(
            collar_of(&feline_model_kind(
                &[protocol_int_data(CAT_COLLAR_COLOR_DATA_ID, 5)],
                true,
                None,
            )),
            None
        );
        // A tame cat with no explicit color defaults to RED (14).
        assert_eq!(
            collar_of(&feline_model_kind(
                &[protocol_byte_data(
                    TAMABLE_ANIMAL_FLAGS_DATA_ID,
                    TAMABLE_ANIMAL_TAME_FLAG
                )],
                true,
                None,
            )),
            Some(EntityDyeColor::Red)
        );
        // A tame cat shows its dyed collar color.
        assert_eq!(
            collar_of(&feline_model_kind(
                &[
                    protocol_byte_data(TAMABLE_ANIMAL_FLAGS_DATA_ID, TAMABLE_ANIMAL_TAME_FLAG),
                    protocol_int_data(CAT_COLLAR_COLOR_DATA_ID, 5),
                ],
                true,
                None,
            )),
            Some(EntityDyeColor::Lime)
        );
        // A tame ocelot still has no collar.
        assert_eq!(
            collar_of(&feline_model_kind(
                &[protocol_byte_data(
                    TAMABLE_ANIMAL_FLAGS_DATA_ID,
                    TAMABLE_ANIMAL_TAME_FLAG
                )],
                false,
                None,
            )),
            None
        );
    }

    #[test]
    fn entity_model_instances_project_cat_variants_from_world_registry_order() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            "minecraft:cat_variant",
            0,
            vec![
                RegistryPacketEntry::stub("minecraft:jellie"),
                RegistryPacketEntry::stub("minecraft:calico"),
                RegistryPacketEntry::stub("minecraft:white"),
            ],
        );
        let cat_registry = world.registry_content("minecraft:cat_variant").unwrap();
        assert_eq!(
            entity_model_kind_with_registries(
                VANILLA_ENTITY_TYPE_CAT_ID,
                &[protocol_cat_variant_data(99)],
                None,
                None,
                None,
                None,
                Some(cat_registry),
                None,
            ),
            EntityModelKind::Feline {
                cat: true,
                baby: false,
                cat_variant: CatModelVariant::Black,
                collar: None
            }
        );
        world.apply_add_entity(protocol_add_entity(
            41,
            VANILLA_ENTITY_TYPE_CAT_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            42,
            VANILLA_ENTITY_TYPE_CAT_ID,
            [3.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 41,
            values: vec![protocol_cat_variant_data(0)],
        }));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 42,
            values: vec![
                protocol_cat_variant_data(2),
                protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
            ],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![
                    EntityModelInstance::feline(
                        41,
                        [1.0, 64.0, -2.0],
                        0.0,
                        true,
                        false,
                        CatModelVariant::Jellie,
                        None,
                    ),
                    EntityModelInstance::feline(
                        42,
                        [3.0, 64.0, -2.0],
                        0.0,
                        true,
                        true,
                        CatModelVariant::White,
                        None,
                    ),
                ],
                1.0,
            )
        );
    }

    #[test]
    fn entity_model_instances_project_feline_crouch_and_sprint_from_world() {
        // Vanilla `CatRenderer` / `OcelotRenderer.extractRenderState` copy `Entity.isCrouching()`
        // (Pose.CROUCHING, ordinal 5) and `Entity.isSprinting()` (shared flags bit 3).
        const ENTITY_DATA_POSE_ID: u8 = 6;
        const ENTITY_SHARED_FLAG_SPRINTING: i8 = 1 << 3;
        const POSE_STANDING: i32 = 0;
        const POSE_CROUCHING: i32 = 5;
        const POSE_SERIALIZER_ID: i32 = 20;

        let pose_data = |pose| EntityDataValue {
            data_id: ENTITY_DATA_POSE_ID,
            serializer_id: POSE_SERIALIZER_ID,
            value: EntityDataValueKind::Pose(pose),
        };
        let feline_state = |world: &WorldStore, id: i32| {
            let instance = entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap();
            (
                instance.render_state.feline_is_crouching,
                instance.render_state.feline_is_sprinting,
            )
        };

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            43,
            VANILLA_ENTITY_TYPE_CAT_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            44,
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            [3.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            45,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [5.0, 64.0, -2.0],
        ));

        assert_eq!(feline_state(&world, 43), (false, false));
        assert_eq!(feline_state(&world, 44), (false, false));

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 43,
            values: vec![
                pose_data(POSE_CROUCHING),
                protocol_byte_data(ENTITY_SHARED_FLAGS_DATA_ID, ENTITY_SHARED_FLAG_SPRINTING),
            ],
        }));
        assert_eq!(feline_state(&world, 43), (true, true));

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 44,
            values: vec![pose_data(POSE_CROUCHING)],
        }));
        assert_eq!(feline_state(&world, 44), (true, false));

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 45,
            values: vec![
                pose_data(POSE_CROUCHING),
                protocol_byte_data(ENTITY_SHARED_FLAGS_DATA_ID, ENTITY_SHARED_FLAG_SPRINTING),
            ],
        }));
        assert_eq!(feline_state(&world, 45), (false, false));

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 43,
            values: vec![
                pose_data(POSE_STANDING),
                protocol_byte_data(ENTITY_SHARED_FLAGS_DATA_ID, 0),
            ],
        }));
        assert_eq!(feline_state(&world, 43), (false, false));
    }

    #[test]
    fn entity_model_kind_uses_vanilla_wolf_anger_end_time_metadata() {
        assert_eq!(
            entity_model_kind_with_time_and_registries(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                &[protocol_long_data(WOLF_ANGER_END_TIME_DATA_ID, 200)],
                0.0,
                199,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: true,
                collar_color: None,
                variant: WolfModelVariant::Pale,
            }
        );
        assert_eq!(
            entity_model_kind_with_time_and_registries(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                &[protocol_long_data(WOLF_ANGER_END_TIME_DATA_ID, 200)],
                0.0,
                200,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                collar_color: None,
                variant: WolfModelVariant::Pale,
            }
        );
        assert_eq!(
            entity_model_kind_with_time_and_registries(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                &[
                    protocol_byte_data(TAMABLE_ANIMAL_FLAGS_DATA_ID, TAMABLE_ANIMAL_TAME_FLAG),
                    protocol_long_data(WOLF_ANGER_END_TIME_DATA_ID, 200),
                ],
                0.0,
                199,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: true,
                angry: true,
                collar_color: Some(EntityDyeColor::Red),
                variant: WolfModelVariant::Pale,
            }
        );
    }

    #[test]
    fn entity_model_instances_project_wolf_anger_from_world_game_time() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            148,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            [1.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 148,
            values: vec![protocol_long_data(WOLF_ANGER_END_TIME_DATA_ID, 130)],
        }));
        world.apply_world_time(PlayTime {
            game_time: 120,
            clock_updates: Vec::new(),
        });

        let angry_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            angry_instances,
            aged(
                vec![EntityModelInstance::wolf_state(
                    148,
                    [1.0, 64.0, -2.0],
                    0.0,
                    false,
                    false,
                    true,
                    false,
                    None,
                )
                // Vanilla `Wolf.getTailAngle()` angry branch raises the tail to 1.5393804.
                .with_wolf_tail_angle(1.5393804)],
                1.0,
            )
        );

        world.apply_world_time(PlayTime {
            game_time: 130,
            clock_updates: Vec::new(),
        });

        let calm_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            calm_instances,
            aged(
                vec![EntityModelInstance::wolf_state(
                    148,
                    [1.0, 64.0, -2.0],
                    0.0,
                    false,
                    false,
                    false,
                    false,
                    None,
                )],
                1.0,
            )
        );
    }

    #[test]
    fn entity_model_instances_project_wolf_invisible_shared_flag_from_world() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            148,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            [1.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 148,
            values: vec![
                protocol_byte_data(ENTITY_SHARED_FLAGS_DATA_ID, ENTITY_SHARED_FLAG_INVISIBLE),
                protocol_byte_data(TAMABLE_ANIMAL_FLAGS_DATA_ID, TAMABLE_ANIMAL_TAME_FLAG),
                protocol_int_data(WOLF_COLLAR_COLOR_DATA_ID, 11),
            ],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![EntityModelInstance::wolf_state(
                    148,
                    [1.0, 64.0, -2.0],
                    0.0,
                    false,
                    true,
                    false,
                    true,
                    Some(EntityDyeColor::Blue),
                )
                // A tame wolf with no synced health defaults to full (maxHealth 40), so
                // `Wolf.getTailAngle()` = (0.55 - 0) * π.
                .with_wolf_tail_angle(0.55 * std::f32::consts::PI)],
                1.0,
            )
        );
        // The shared invisible flag is now projected uniformly into the render state.
        assert!(instances[0].render_state.invisible);
        assert!(instances[0].render_state.invisible_to_player);
    }

    #[test]
    fn entity_model_instances_project_wolf_tame_tail_angle_from_health() {
        // Vanilla `Wolf.getTailAngle()` for a tame wolf droops the tail with damage:
        // (0.55 - damageRatio * 0.4) * π, damageRatio = (maxHealth - health) / maxHealth,
        // with the tame maxHealth constant 40. A hurt tame wolf (health 8/40) lowers its
        // tail off the healthy raise.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            148,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            [1.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 148,
            values: vec![
                protocol_byte_data(TAMABLE_ANIMAL_FLAGS_DATA_ID, TAMABLE_ANIMAL_TAME_FLAG),
                protocol_float_data(LIVING_ENTITY_HEALTH_DATA_ID, 8.0),
            ],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        let tail_angle = instances[0].render_state.wolf_tail_angle;
        let expected = (0.55 - 0.8 * 0.4) * std::f32::consts::PI; // damageRatio 0.8
        assert!(
            (tail_angle - expected).abs() < 1e-6,
            "tame wolf tail droops with health: {tail_angle} vs {expected}"
        );

        // An untamed wolf keeps the π/5 default no matter its health.
        let mut wild = WorldStore::new();
        wild.apply_add_entity(protocol_add_entity(
            149,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            [0.0, 64.0, 0.0],
        ));
        assert!(wild.apply_set_entity_data(SetEntityData {
            id: 149,
            values: vec![protocol_float_data(LIVING_ENTITY_HEALTH_DATA_ID, 4.0)],
        }));
        let wild_instances = entity_model_instances_from_world_at_partial_tick(&wild, None, 1.0);
        assert_eq!(
            wild_instances[0].render_state.wolf_tail_angle,
            std::f32::consts::PI / 5.0
        );
    }

    #[test]
    fn entity_model_instances_project_wolf_sitting_flag_from_world() {
        // Vanilla `WolfRenderState.isSitting = Wolf.isInSittingPose()` = `TamableAnimal`
        // `DATA_FLAGS_ID` bit 1. A sitting (tame) wolf projects `wolf_sitting`; clearing the
        // bit projects `false`.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            148,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            [1.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 148,
            values: vec![protocol_byte_data(
                TAMABLE_ANIMAL_FLAGS_DATA_ID,
                TAMABLE_ANIMAL_TAME_FLAG | TAMABLE_ANIMAL_SITTING_FLAG,
            )],
        }));
        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert!(
            instances[0].render_state.wolf_sitting,
            "a sitting wolf projects wolf_sitting"
        );

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 148,
            values: vec![protocol_byte_data(
                TAMABLE_ANIMAL_FLAGS_DATA_ID,
                TAMABLE_ANIMAL_TAME_FLAG,
            )],
        }));
        let standing = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
        assert!(
            !standing[0].render_state.wolf_sitting,
            "a standing wolf does not project wolf_sitting"
        );
    }

    #[test]
    fn entity_model_instance_projects_wolf_wet_shade_from_source() {
        // Vanilla `WolfRenderer.extractRenderState` copies `Wolf.getWetShade(partialTicks)`
        // into `WolfRenderState.wetShade`, and `WolfRenderer.getModelTint` consumes that
        // render-state field. It also copies `Wolf.getShakeAnim(partialTicks)` for
        // `WolfRenderState.getBodyRollAngle`, and `Wolf.getHeadRollAngle(partialTicks)`
        // into `WolfRenderState.headRollAngle`. The world layer owns the timers; native must
        // preserve the projected values when building `EntityRenderState`.
        let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
            "entity_id": 148,
            "entity_type_id": VANILLA_ENTITY_TYPE_WOLF_ID,
            "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
            "y_rot": 0.0,
            "wolf_wet_shade": 0.75625,
            "wolf_shake_anim": 0.5,
            "wolf_head_roll_angle": 0.188,
            "data_values": []
        }))
        .unwrap();

        let instance = entity_model_instance(
            source,
            &WorldStore::new(),
            None,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            instance.kind,
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                collar_color: None,
                variant: WolfModelVariant::Pale,
            }
        );
        assert!(
            (instance.render_state.wolf_wet_shade - 0.75625).abs() < 1.0e-6,
            "native preserves world-projected WolfRenderState.wetShade: {}",
            instance.render_state.wolf_wet_shade
        );
        assert!(
            (instance.render_state.wolf_shake_anim - 0.5).abs() < 1.0e-6,
            "native preserves world-projected WolfRenderState.shakeAnim: {}",
            instance.render_state.wolf_shake_anim
        );
        assert!(
            (instance.render_state.wolf_head_roll_angle - 0.188).abs() < 1.0e-6,
            "native preserves world-projected WolfRenderState.headRollAngle: {}",
            instance.render_state.wolf_head_roll_angle
        );
    }

    #[test]
    fn entity_model_instances_project_parrot_sitting_flag_from_world() {
        // Vanilla `ParrotModel.getPose == SITTING` = `Parrot.isInSittingPose()` = the same
        // `TamableAnimal.DATA_FLAGS_ID` bit 1 (id 18) the wolf uses. A sitting parrot projects
        // `parrot_sitting`; the projection is gated to the parrot, so the same flag byte on
        // another tamable (wolf) never sets `parrot_sitting`.
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            150,
            VANILLA_ENTITY_TYPE_PARROT_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            151,
            VANILLA_ENTITY_TYPE_WOLF_ID,
            [2.0, 64.0, -2.0],
        ));

        let parrot_sitting = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .parrot_sitting
        };

        // A standing parrot projects parrot_sitting = false.
        assert!(!parrot_sitting(&world, 150));

        // Setting the TamableAnimal sitting bit projects through to the perch pose.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 150,
            values: vec![protocol_byte_data(
                TAMABLE_ANIMAL_FLAGS_DATA_ID,
                TAMABLE_ANIMAL_SITTING_FLAG,
            )],
        }));
        assert!(parrot_sitting(&world, 150));

        // The same sitting bit on a non-parrot (wolf) does NOT project parrot_sitting — the
        // derivation is gated to the parrot type.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 151,
            values: vec![protocol_byte_data(
                TAMABLE_ANIMAL_FLAGS_DATA_ID,
                TAMABLE_ANIMAL_TAME_FLAG | TAMABLE_ANIMAL_SITTING_FLAG,
            )],
        }));
        assert!(!parrot_sitting(&world, 151));
    }

    #[test]
    fn entity_model_instances_preserve_parrot_party_from_world_source() {
        // Vanilla `ParrotRenderer.extractRenderState` copies `ParrotModel.getPose(entity)`, where
        // `isPartyParrot()` wins over sitting/flying. The world layer owns the jukebox proximity
        // projection, so native must preserve `parrot_party` when building `EntityRenderState`.
        let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
            "entity_id": 152,
            "entity_type_id": VANILLA_ENTITY_TYPE_PARROT_ID,
            "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
            "y_rot": 0.0,
            "parrot_party": true,
            "data_values": []
        }))
        .unwrap();

        let instance = entity_model_instance(
            source,
            &WorldStore::new(),
            None,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            instance.kind,
            EntityModelKind::Parrot {
                variant: ParrotModelVariant::RedBlue,
            }
        );
        assert!(
            instance.render_state.parrot_party,
            "native preserves the world-projected ParrotRenderState PARTY pose"
        );
    }

    #[test]
    fn entity_model_instances_project_illager_spellcasting_flag_from_world() {
        // Vanilla `SpellcasterIllager.isCastingSpell()` = the synced `DATA_SPELL_CASTING_ID`
        // byte > 0 (id 17, the byte holds the spell id). A casting evoker/illusioner projects
        // `illager_spellcasting`; the projection is gated to the spellcaster illagers, so the
        // same byte on a vindicator never sets it.
        const VANILLA_SPELLCASTER_CASTING_DATA_ID: u8 = 17;
        const VANILLA_ENTITY_TYPE_VINDICATOR_ID: i32 = 140;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            160,
            VANILLA_ENTITY_TYPE_EVOKER_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            161,
            VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
            [2.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            162,
            VANILLA_ENTITY_TYPE_VINDICATOR_ID,
            [3.0, 64.0, -2.0],
        ));

        let casting = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .illager_spellcasting
        };

        // An idle evoker/illusioner projects illager_spellcasting = false.
        assert!(!casting(&world, 160));
        assert!(!casting(&world, 161));

        // Setting the spell-casting byte > 0 (here 2 = FANGS) projects through to the cast pose.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 160,
            values: vec![protocol_byte_data(VANILLA_SPELLCASTER_CASTING_DATA_ID, 2)],
        }));
        assert!(casting(&world, 160));
        // Any non-zero spell id (1 = SUMMON_VEX, the lowest) also counts as casting.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 161,
            values: vec![protocol_byte_data(VANILLA_SPELLCASTER_CASTING_DATA_ID, 1)],
        }));
        assert!(casting(&world, 161));

        // The same byte on a non-spellcaster (vindicator) does NOT project illager_spellcasting.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 162,
            values: vec![protocol_byte_data(VANILLA_SPELLCASTER_CASTING_DATA_ID, 2)],
        }));
        assert!(!casting(&world, 162));
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_horses() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_HORSE_ID, &[]),
            EntityModelKind::Horse {
                baby: false,
                variant: HorseColorVariant::White,
                markings: HorseMarkings::None
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_HORSE_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Horse {
                baby: true,
                variant: HorseColorVariant::White,
                markings: HorseMarkings::None
            }
        );
        // The packed `DATA_ID_TYPE_VARIANT` (id 19) carries the coat color in the low byte
        // (`& 0xFF`) and the markings in the next nibble (`>> 8`): id `4 | (2 << 8)` = black coat +
        // white-field markings.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_HORSE_ID,
                &[protocol_int_data(HORSE_VARIANT_DATA_ID, 4 | (2 << 8))]
            ),
            EntityModelKind::Horse {
                baby: false,
                variant: HorseColorVariant::Black,
                markings: HorseMarkings::WhiteField
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_DONKEY_ID, &[]),
            EntityModelKind::Donkey {
                family: DonkeyModelFamily::Donkey,
                baby: false,
                has_chest: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_DONKEY_ID,
                &[
                    protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
                    protocol_bool_data(ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID, true),
                ]
            ),
            EntityModelKind::Donkey {
                family: DonkeyModelFamily::Donkey,
                baby: true,
                has_chest: true
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_MULE_ID,
                &[protocol_bool_data(
                    ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID,
                    true
                )]
            ),
            EntityModelKind::Donkey {
                family: DonkeyModelFamily::Mule,
                baby: false,
                has_chest: true
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, &[]),
            EntityModelKind::UndeadHorse {
                family: UndeadHorseModelFamily::Skeleton,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::UndeadHorse {
                family: UndeadHorseModelFamily::Skeleton,
                baby: true
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, &[]),
            EntityModelKind::UndeadHorse {
                family: UndeadHorseModelFamily::Zombie,
                baby: false
            }
        );
        // The nautilus (adult and baby) renders through its dedicated `NautilusModel`
        // (`createBodyMesh` / `createBabyBodyLayer`). The zombie nautilus maps to the dedicated
        // `ZombieNautilus` kind, selected by the synced `ZombieNautilusVariant` holder: the default
        // TEMPERATE → `coral: false` (the `NautilusModel` body + `zombie_nautilus.png`), WARM (registry
        // id ≥ 1) → `coral: true` (the `ZombieNautilusCoralModel` + `zombie_nautilus_coral.png`). It is a
        // plain `MobRenderer`, so always adult.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_NAUTILUS_ID, &[]),
            EntityModelKind::Nautilus { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_NAUTILUS_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Nautilus { baby: true }
        );
        // No variant data → the TEMPERATE default (no corals).
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, &[]),
            EntityModelKind::ZombieNautilus { coral: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
                &[protocol_zombie_nautilus_variant_data(0)]
            ),
            EntityModelKind::ZombieNautilus { coral: false }
        );
        // WARM (registry id 1) → the coral model.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
                &[protocol_zombie_nautilus_variant_data(1)]
            ),
            EntityModelKind::ZombieNautilus { coral: true }
        );
        // The zombie nautilus is never a baby, so the baby flag in its metadata is ignored.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::ZombieNautilus { coral: false }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_camels() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_CAMEL_ID, &[]),
            EntityModelKind::Camel {
                family: CamelModelFamily::Camel,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_CAMEL_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Camel {
                family: CamelModelFamily::Camel,
                baby: true
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, &[]),
            EntityModelKind::Camel {
                family: CamelModelFamily::CamelHusk,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Camel {
                family: CamelModelFamily::CamelHusk,
                baby: false
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_llamas() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_LLAMA_ID, &[]),
            EntityModelKind::Llama {
                family: LlamaModelFamily::Llama,
                variant: LlamaVariant::Creamy,
                baby: false,
                has_chest: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_LLAMA_ID,
                &[
                    protocol_int_data(LLAMA_VARIANT_DATA_ID, 2),
                    protocol_bool_data(ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID, true),
                ]
            ),
            EntityModelKind::Llama {
                family: LlamaModelFamily::Llama,
                variant: LlamaVariant::Brown,
                baby: false,
                has_chest: true
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_LLAMA_ID,
                &[
                    protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true),
                    protocol_bool_data(ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID, true),
                    protocol_int_data(LLAMA_VARIANT_DATA_ID, 3),
                ]
            ),
            EntityModelKind::Llama {
                family: LlamaModelFamily::Llama,
                variant: LlamaVariant::Gray,
                baby: true,
                has_chest: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
                &[protocol_int_data(LLAMA_VARIANT_DATA_ID, 99)]
            ),
            EntityModelKind::Llama {
                family: LlamaModelFamily::TraderLlama,
                variant: LlamaVariant::Gray,
                baby: false,
                has_chest: false
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
                &[protocol_int_data(LLAMA_VARIANT_DATA_ID, -1)]
            ),
            EntityModelKind::Llama {
                family: LlamaModelFamily::TraderLlama,
                variant: LlamaVariant::Creamy,
                baby: false,
                has_chest: false
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_illagers_and_witch() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_WITCH_ID, &[]),
            EntityModelKind::Witch
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_EVOKER_ID, &[]),
            EntityModelKind::Illager {
                family: IllagerModelFamily::Evoker
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ILLUSIONER_ID, &[]),
            EntityModelKind::Illager {
                family: IllagerModelFamily::Illusioner
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PILLAGER_ID, &[]),
            EntityModelKind::Illager {
                family: IllagerModelFamily::Pillager
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_VINDICATOR_ID, &[]),
            EntityModelKind::Illager {
                family: IllagerModelFamily::Vindicator
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_spiders() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SPIDER_ID, &[]),
            EntityModelKind::Spider
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, &[]),
            EntityModelKind::CaveSpider
        );
    }

    #[test]
    fn entity_model_kind_uses_avatar_model_part_visibility_for_players_and_mannequins() {
        let hat_and_left_sleeve = PlayerModelPartVisibility::from_vanilla_mask(
            PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::LEFT_SLEEVE_MASK,
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PLAYER_ID, &[]),
            EntityModelKind::Player {
                skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
                parts: PlayerModelPartVisibility::from_vanilla_mask(0),
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, &[]),
            EntityModelKind::Player {
                skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
                parts: PlayerModelPartVisibility::from_vanilla_mask(
                    PlayerModelPartVisibility::ALL_MASK,
                ),
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                &[protocol_byte_data(
                    AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
                    hat_and_left_sleeve.vanilla_mask() as i8,
                )],
            ),
            EntityModelKind::Player {
                skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
                parts: hat_and_left_sleeve,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
                &[protocol_byte_data(AVATAR_MODEL_CUSTOMIZATION_DATA_ID, 0)],
            ),
            EntityModelKind::Player {
                skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
                parts: PlayerModelPartVisibility::from_vanilla_mask(0),
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_boats_and_rafts() {
        let cases = [
            (
                VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID,
                BoatModelFamily::Acacia,
                false,
            ),
            (
                VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID,
                BoatModelFamily::Acacia,
                true,
            ),
            (
                VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
                BoatModelFamily::Bamboo,
                false,
            ),
            (
                VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID,
                BoatModelFamily::Bamboo,
                true,
            ),
            (
                VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID,
                BoatModelFamily::Birch,
                false,
            ),
            (
                VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID,
                BoatModelFamily::Cherry,
                true,
            ),
            (
                VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID,
                BoatModelFamily::DarkOak,
                false,
            ),
            (
                VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID,
                BoatModelFamily::Jungle,
                true,
            ),
            (
                VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID,
                BoatModelFamily::Mangrove,
                false,
            ),
            (
                VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
                BoatModelFamily::Oak,
                true,
            ),
            (
                VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID,
                BoatModelFamily::PaleOak,
                false,
            ),
            (
                VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID,
                BoatModelFamily::Spruce,
                true,
            ),
        ];

        for (entity_type_id, family, chest) in cases {
            assert_eq!(
                entity_model_kind(entity_type_id, &[]),
                EntityModelKind::Boat { family, chest }
            );
        }
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_enderman() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ENDERMAN_ID, &[]),
            EntityModelKind::Enderman
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, &[]),
            EntityModelKind::SnowGolem
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_iron_golem() {
        // No synced health → the default full-health golem is uncracked.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, &[]),
            EntityModelKind::IronGolem {
                crackiness: IronGolemCrackiness::None,
            }
        );
        // Vanilla `IronGolem.getCrackiness()` = `Crackiness.GOLEM.byFraction(health / 100)`: at 40/100
        // (= 0.4) the medium cracks show.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
                &[protocol_float_data(LIVING_ENTITY_HEALTH_DATA_ID, 40.0)]
            ),
            EntityModelKind::IronGolem {
                crackiness: IronGolemCrackiness::Medium,
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, &[]),
            EntityModelKind::SnowGolem
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_snow_golem() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, &[]),
            EntityModelKind::SnowGolem
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_copper_golem_weathering() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, &[]),
            EntityModelKind::CopperGolem {
                weathering: CopperGolemWeathering::Unaffected,
            }
        );
        for (id, weathering) in [
            (-5, CopperGolemWeathering::Unaffected),
            (0, CopperGolemWeathering::Unaffected),
            (1, CopperGolemWeathering::Exposed),
            (2, CopperGolemWeathering::Weathered),
            (3, CopperGolemWeathering::Oxidized),
            (99, CopperGolemWeathering::Oxidized),
        ] {
            assert_eq!(
                entity_model_kind(
                    VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
                    &[protocol_copper_golem_weathering_data(id)],
                ),
                EntityModelKind::CopperGolem { weathering }
            );
        }
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
                &[protocol_armadillo_state_data(2)],
            ),
            EntityModelKind::CopperGolem {
                weathering: CopperGolemWeathering::Unaffected,
            }
        );
    }

    #[test]
    fn entity_model_kind_uses_explicit_unknown_future_fallback() {
        let kind = entity_model_kind(9999, &[]);

        assert_eq!(
            placeholder_name(kind),
            Some("todo_unknown_entity_type_bounds")
        );
    }

    #[test]
    fn entity_model_instances_filter_local_player_and_camera_entity() {
        let mut world = WorldStore::new();
        world.apply_login(&protocol_play_login(10));
        world.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [0.0, 64.0, 0.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            11,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, 0.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [2.0, 64.0, 0.0],
        ));
        assert!(world.apply_set_camera(SetCamera { camera_id: 11 }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![EntityModelInstance::chicken(
                    12,
                    [2.0, 64.0, 0.0],
                    0.0,
                    false
                )],
                1.0,
            )
        );
    }

    fn protocol_add_entity(id: i32, entity_type_id: i32, position: [f64; 3]) -> AddEntity {
        protocol_add_entity_with_rotation(id, entity_type_id, position, 0.0, 0.0, 0.0)
    }

    fn protocol_add_entity_with_uuid(
        id: i32,
        entity_type_id: i32,
        uuid: Uuid,
        position: [f64; 3],
    ) -> AddEntity {
        let mut entity = protocol_add_entity(id, entity_type_id, position);
        entity.uuid = uuid;
        entity
    }

    /// Stamps the projected `ageInTicks` (= entity `age_ticks` + partial tick) onto every
    /// expected instance, so model-selection assertions need not repeat it per instance.
    fn aged(
        mut instances: Vec<EntityModelInstance>,
        age_in_ticks: f32,
    ) -> Vec<EntityModelInstance> {
        for instance in &mut instances {
            instance.render_state.age_in_ticks = age_in_ticks;
        }
        instances
    }

    fn protocol_add_entity_with_rotation(
        id: i32,
        entity_type_id: i32,
        position: [f64; 3],
        y_rot: f32,
        x_rot: f32,
        y_head_rot: f32,
    ) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678 + id as u128),
            entity_type_id,
            position: Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            delta_movement: Vec3d::default(),
            x_rot,
            y_rot,
            y_head_rot,
            data: 0,
        }
    }

    fn protocol_bool_data(data_id: u8, value: bool) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 8,
            value: EntityDataValueKind::Boolean(value),
        }
    }

    fn protocol_optional_block_pos_data(
        data_id: u8,
        value: Option<bbb_protocol::packets::BlockPos>,
    ) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 11,
            value: EntityDataValueKind::OptionalBlockPos(value),
        }
    }

    fn protocol_chicken_variant_data(id: i32) -> EntityDataValue {
        EntityDataValue {
            data_id: CHICKEN_VARIANT_DATA_ID,
            serializer_id: 30,
            value: EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::ChickenVariant,
                id,
            },
        }
    }

    fn protocol_armadillo_state_data(id: i32) -> EntityDataValue {
        // Vanilla `EntityDataSerializers.ARMADILLO_STATE` is serializer id 36.
        EntityDataValue {
            data_id: ARMADILLO_STATE_DATA_ID,
            serializer_id: 36,
            value: EntityDataValueKind::EnumId {
                serializer: EntityDataEnumSerializer::ArmadilloState,
                id,
            },
        }
    }

    fn protocol_copper_golem_weathering_data(id: i32) -> EntityDataValue {
        // Vanilla `EntityDataSerializers.WEATHERING_COPPER_STATE` is serializer id 38.
        EntityDataValue {
            data_id: COPPER_GOLEM_WEATHER_STATE_DATA_ID,
            serializer_id: 38,
            value: EntityDataValueKind::EnumId {
                serializer: EntityDataEnumSerializer::WeatheringCopperState,
                id,
            },
        }
    }

    fn protocol_villager_data(
        data_id: u8,
        villager_type: i32,
        profession: i32,
        level: i32,
    ) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 18,
            value: EntityDataValueKind::VillagerData {
                villager_type,
                profession,
                level,
            },
        }
    }

    fn protocol_cow_variant_data(id: i32) -> EntityDataValue {
        EntityDataValue {
            data_id: COW_VARIANT_DATA_ID,
            serializer_id: 23,
            value: EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::CowVariant,
                id,
            },
        }
    }

    fn protocol_pig_variant_data(id: i32) -> EntityDataValue {
        EntityDataValue {
            data_id: PIG_VARIANT_DATA_ID,
            serializer_id: 28,
            value: EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::PigVariant,
                id,
            },
        }
    }

    fn protocol_zombie_nautilus_variant_data(id: i32) -> EntityDataValue {
        // Vanilla `EntityDataSerializers.ZOMBIE_NAUTILUS_VARIANT` is serializer id 32.
        EntityDataValue {
            data_id: ZOMBIE_NAUTILUS_VARIANT_DATA_ID,
            serializer_id: 32,
            value: EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::ZombieNautilusVariant,
                id,
            },
        }
    }

    fn protocol_frog_variant_data(id: i32) -> EntityDataValue {
        // Vanilla `EntityDataSerializers.FROG_VARIANT` is serializer id 27.
        EntityDataValue {
            data_id: FROG_VARIANT_DATA_ID,
            serializer_id: 27,
            value: EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::FrogVariant,
                id,
            },
        }
    }

    fn protocol_cat_variant_data(id: i32) -> EntityDataValue {
        // Vanilla `EntityDataSerializers.CAT_VARIANT` is serializer id 21.
        EntityDataValue {
            data_id: CAT_VARIANT_DATA_ID,
            serializer_id: 21,
            value: EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::CatVariant,
                id,
            },
        }
    }

    fn protocol_wolf_variant_data(id: i32) -> EntityDataValue {
        // Vanilla `EntityDataSerializers.WOLF_VARIANT` is serializer id 25.
        EntityDataValue {
            data_id: WOLF_VARIANT_DATA_ID,
            serializer_id: 25,
            value: EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::WolfVariant,
                id,
            },
        }
    }

    fn protocol_int_data(data_id: u8, value: i32) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 1,
            value: EntityDataValueKind::Int(value),
        }
    }

    fn protocol_optional_unsigned_int_data(data_id: u8, value: Option<i32>) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 19,
            value: EntityDataValueKind::OptionalUnsignedInt(value),
        }
    }

    fn protocol_long_data(data_id: u8, value: i64) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 2,
            value: EntityDataValueKind::Long(value),
        }
    }

    fn protocol_float_data(data_id: u8, value: f32) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 3,
            value: EntityDataValueKind::Float(value),
        }
    }

    fn protocol_optional_component_data(data_id: u8, value: Option<&str>) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 6,
            value: EntityDataValueKind::OptionalComponent(value.map(str::to_string)),
        }
    }

    fn protocol_byte_data(data_id: u8, value: i8) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(value),
        }
    }

    fn protocol_direction_data(data_id: u8, value: i32) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 12,
            value: EntityDataValueKind::Direction(value),
        }
    }

    fn protocol_rotations_data(data_id: u8, value: [f32; 3]) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 9,
            value: EntityDataValueKind::Rotations {
                x: value[0],
                y: value[1],
                z: value[2],
            },
        }
    }

    fn protocol_play_login(player_id: i32) -> PlayLogin {
        PlayLogin {
            player_id,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        }
    }

    fn assert_selection_box_close(actual: [f32; 3], expected: [f32; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1.0e-5,
                "expected {expected}, got {actual}"
            );
        }
    }

    const VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS: &[i32] = &[
        0, 1, 2, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
        28, 29, 30, 31, 32, 33, 34, 35, 36, 38, 40, 41, 42, 45, 46, 51, 52, 54, 55, 56, 57, 58, 59,
        61, 62, 63, 64, 65, 66, 67, 68, 70, 74, 75, 78, 80, 81, 82, 83, 85, 86, 87, 88, 89, 90, 91,
        94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 107, 108, 109, 110, 111, 112, 113, 114,
        115, 116, 117, 119, 121, 122, 124, 125, 126, 127, 128, 129, 130, 132, 133, 134, 136, 137,
        138, 139, 140, 141, 142, 143, 144, 145, 146, 148, 149, 150, 151, 152, 153, 154, 155,
    ];

    fn placeholder_name(kind: EntityModelKind) -> Option<&'static str> {
        match kind {
            EntityModelKind::Placeholder { name, .. } => Some(name),
            _ => None,
        }
    }
}
