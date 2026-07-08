use std::collections::HashMap;

use bbb_protocol::packets::{
    decode_profile_textures_from_properties, EntityDataEnumSerializer, EntityDataRegistryHolder,
    EntityDataValueKind, EquipmentSlot, GameProfilePropertySummary, ItemStackSummary,
    PlayerSkinPatchSummary, ResolvableProfileKindSummary, ResolvableProfileSummary,
    SwingAnimationTypeSummary,
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
    PlayerModelPartVisibility, RabbitModelVariant, SalmonModelSize, SelectionBox,
    SelectionColoredBox, SelectionLine, SelectionOutline, SelectionPoint, SheepHeadEatPose,
    SheepWoolColor, SkeletonModelFamily, SleepingPose, SpearKineticWeapon, TropicalFishModelShape,
    TropicalFishPattern, UndeadHorseModelFamily, VillagerModelData, VillagerModelProfession,
    VillagerModelType, WolfArmorCrackiness, WolfModelVariant, ZombieVariantModelFamily,
    DEFAULT_ARMOR_STAND_MODEL_POSE, ENTITY_DEFAULT_OUTLINE_COLOR,
};
#[cfg(test)]
use bbb_renderer::{EntityDynamicPlayerSkinStatus, EntityPlayerSkinModel};
use bbb_world::{
    vanilla_entity_type_is_living, ArmorMaterialKind as WorldArmorMaterialKind,
    EndCrystalBeamSource as WorldEndCrystalBeamSource,
    EnderDragonBeamSource as WorldEnderDragonBeamSource,
    EntityAttachmentFace as WorldEntityAttachmentFace, EntityModelSourceState,
    EntityPickTargetState, GuardianBeamSource as WorldGuardianBeamSource,
    LlamaBodyDecorColor as WorldLlamaBodyDecorColor, RegistryContentState,
    WolfArmorCrackiness as WorldWolfArmorCrackiness, WorldStore,
};

use bbb_item_model::{default_player_skin_for_profile_id, NativeItemRuntime};
use bbb_protocol::entity_types::*;

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
const ENTITY_HITBOX_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const ENTITY_EYE_HEIGHT_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const ENTITY_DRAGON_PART_HITBOX_COLOR: [f32; 4] = [0.25, 1.0, 0.0, 1.0];
const ENTITY_VIEW_VECTOR_COLOR: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const ENTITY_EYE_HEIGHT_PADDING: f32 = 0.01;
const ENTITY_VIEW_VECTOR_LENGTH: f32 = 2.0;
const ENTITY_POSITION_POINT_SIZE: f32 = 2.0;
const AVATAR_MODEL_CUSTOMIZATION_DATA_ID: u8 = 16;
const AVATAR_PLAYER_DEFAULT_MODEL_CUSTOMIZATION: i8 = 0;
const MANNEQUIN_DEFAULT_MODEL_CUSTOMIZATION: i8 = PlayerModelPartVisibility::ALL_MASK as i8;
const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
const ENTITY_SHARED_FLAG_ON_FIRE: i8 = 0x01;
const ENTITY_SHARED_FLAG_INVISIBLE: i8 = 0x20;
#[cfg(test)]
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
/// Vanilla `AbstractPiglin.DATA_IMMUNE_TO_ZOMBIFICATION`: `Entity` 0..=7,
/// `LivingEntity` 8..=14, `Mob` 15, then the abstract piglin's first accessor.
const PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID: u8 = 16;
const PIGLIN_BABY_DATA_ID: u8 = 17;
/// Vanilla `Hoglin.DATA_IMMUNE_TO_ZOMBIFICATION`: hoglins extend `AgeableMob`,
/// whose baby and age-locked accessors occupy ids 16 and 17.
const HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID: u8 = 18;
const BOGGED_SHEARED_DATA_ID: u8 = 16;
/// Vanilla `CopperGolem.DATA_WEATHER_STATE` data id (16): `CopperGolem` extends `AbstractGolem`
/// without adding inherited synced data after `Mob.DATA_MOB_FLAGS_ID` (15), so its first own
/// accessor is the weathering enum.
const COPPER_GOLEM_WEATHER_STATE_DATA_ID: u8 = 16;
/// Vanilla `CopperGolem.COPPER_GOLEM_STATE` data id (17): the synced `CopperGolemState` enum,
/// declared immediately after `DATA_WEATHER_STATE`.
#[cfg(test)]
const COPPER_GOLEM_STATE_DATA_ID: u8 = 17;
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
    let entity_partial_tick = entity_partial_tick.clamp(0.0, 1.0);
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.local_player().camera.entity_id;
    let sources = world.entity_model_sources_at_partial_tick(entity_partial_tick);
    let sources_by_id: HashMap<_, _> = sources
        .iter()
        .map(|source| (source.entity_id, source))
        .collect();
    let mut boxes = Vec::new();
    let mut lines = Vec::new();
    let mut points = Vec::new();

    for target in world
        .entity_debug_hitbox_targets_at_partial_tick(entity_partial_tick)
        .into_iter()
    {
        if let Some(parent_id) = world.ender_dragon_part_parent_id(target.entity_id) {
            if local_player_id == Some(parent_id) || camera_entity_id == Some(parent_id) {
                continue;
            }
            boxes.push(entity_debug_hitbox_box_with_color(
                target,
                ENTITY_DRAGON_PART_HITBOX_COLOR,
            ));
            continue;
        };
        if local_player_id == Some(target.entity_id) || camera_entity_id == Some(target.entity_id) {
            continue;
        }
        push_entity_debug_gizmos(
            &mut boxes,
            &mut lines,
            &mut points,
            target,
            sources_by_id.get(&target.entity_id).copied(),
            world,
        );
    }

    (!boxes.is_empty() || !lines.is_empty() || !points.is_empty())
        .then(|| SelectionOutline::from_colored_boxes_lines_and_points(boxes, lines, points))
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

pub(crate) fn entity_model_instance_from_world_entity_at_partial_tick(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    entity_partial_tick: f32,
) -> Option<EntityModelInstance> {
    let entity_partial_tick = entity_partial_tick.clamp(0.0, 1.0);
    let source = world.entity_model_source_at_partial_tick(entity_id, entity_partial_tick)?;
    let chicken_variants = world.registry_content("minecraft:chicken_variant");
    let cow_variants = world.registry_content("minecraft:cow_variant");
    let pig_variants = world.registry_content("minecraft:pig_variant");
    let frog_variants = world.registry_content("minecraft:frog_variant");
    let cat_variants = world.registry_content("minecraft:cat_variant");
    let wolf_variants = world.registry_content("minecraft:wolf_variant");
    let villager_types = world.registry_content("minecraft:villager_type");
    let villager_professions = world.registry_content("minecraft:villager_profession");
    let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
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
}

pub(crate) fn entity_model_kind_for_world_entity_type_at_partial_tick(
    world: &WorldStore,
    entity_type_id: i32,
    entity_age_ticks: f32,
) -> EntityModelKind {
    let data_values: &[bbb_protocol::packets::EntityDataValue] = &[];
    entity_model_kind_with_time_and_registries(
        entity_type_id,
        data_values,
        entity_age_ticks,
        world.world_time().map(|time| time.game_time).unwrap_or(0),
        world.registry_content("minecraft:chicken_variant"),
        world.registry_content("minecraft:cow_variant"),
        world.registry_content("minecraft:pig_variant"),
        world.registry_content("minecraft:frog_variant"),
        world.registry_content("minecraft:cat_variant"),
        world.registry_content("minecraft:wolf_variant"),
    )
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

fn push_entity_debug_gizmos(
    boxes: &mut Vec<SelectionColoredBox>,
    lines: &mut Vec<SelectionLine>,
    points: &mut Vec<SelectionPoint>,
    target: EntityPickTargetState,
    source: Option<&EntityModelSourceState>,
    world: &WorldStore,
) {
    boxes.push(entity_debug_hitbox_box(target));
    points.push(entity_debug_position_point([
        target.position.x,
        target.position.y,
        target.position.z,
    ]));
    let Some(pose) = world.probe_entity_camera_pose(target.entity_id) else {
        return;
    };
    let entity_type_id = source
        .map(|source| source.entity_type_id)
        .or_else(|| world.entity_type_id(target.entity_id));
    if entity_type_id.is_some_and(entity_debug_target_type_is_living) {
        boxes.push(entity_debug_eye_height_box(target, pose.eye_height));
    }
    let y_rot = source.map_or(pose.y_rot, |source| source.y_rot);
    let x_rot = source.map_or(pose.x_rot, |source| source.x_rot);
    lines.push(entity_debug_view_vector_line(
        [target.position.x, target.position.y, target.position.z],
        pose.eye_height,
        y_rot,
        x_rot,
    ));
}

fn entity_debug_target_type_is_living(entity_type_id: i32) -> bool {
    vanilla_entity_type_is_living(entity_type_id)
        || entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
}

fn entity_debug_hitbox_box(target: EntityPickTargetState) -> SelectionColoredBox {
    entity_debug_hitbox_box_with_color(target, ENTITY_HITBOX_COLOR)
}

fn entity_debug_hitbox_box_with_color(
    target: EntityPickTargetState,
    color: [f32; 4],
) -> SelectionColoredBox {
    let selection_box = entity_pick_target_box(target);
    SelectionColoredBox {
        min: selection_box.min,
        max: selection_box.max,
        color,
    }
}

fn entity_debug_eye_height_box(
    target: EntityPickTargetState,
    eye_height: f32,
) -> SelectionColoredBox {
    let selection_box = entity_pick_target_box(target);
    let eye_y = selection_box.min[1] + eye_height;
    SelectionColoredBox {
        min: [
            selection_box.min[0],
            eye_y - ENTITY_EYE_HEIGHT_PADDING,
            selection_box.min[2],
        ],
        max: [
            selection_box.max[0],
            eye_y + ENTITY_EYE_HEIGHT_PADDING,
            selection_box.max[2],
        ],
        color: ENTITY_EYE_HEIGHT_COLOR,
    }
}

fn entity_debug_position_point(position: [f64; 3]) -> SelectionPoint {
    SelectionPoint {
        position: [position[0] as f32, position[1] as f32, position[2] as f32],
        color: ENTITY_HITBOX_COLOR,
        size: ENTITY_POSITION_POINT_SIZE,
    }
}

fn entity_debug_view_vector_line(
    position: [f64; 3],
    eye_height: f32,
    y_rot: f32,
    x_rot: f32,
) -> SelectionLine {
    let direction = entity_view_vector(y_rot, x_rot);
    let from = [
        position[0] as f32,
        position[1] as f32 + eye_height,
        position[2] as f32,
    ];
    let to = [
        from[0] + direction[0] * ENTITY_VIEW_VECTOR_LENGTH,
        from[1] + direction[1] * ENTITY_VIEW_VECTOR_LENGTH,
        from[2] + direction[2] * ENTITY_VIEW_VECTOR_LENGTH,
    ];
    SelectionLine {
        from,
        to,
        color: ENTITY_VIEW_VECTOR_COLOR,
    }
}

fn entity_view_vector(y_rot: f32, x_rot: f32) -> [f32; 3] {
    let yaw = y_rot.to_radians();
    let pitch = x_rot.to_radians();
    let cos_pitch = pitch.cos();
    let x = -yaw.sin() * cos_pitch;
    let y = -pitch.sin();
    let z = yaw.cos() * cos_pitch;
    let len = (x * x + y * y + z * z).sqrt();
    if len <= f32::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [x / len, y / len, z / len]
    }
}

#[allow(clippy::too_many_arguments)]
/// Appends the pure-passthrough `EntityModelSourceState` -> `EntityRenderState`
/// projections to an [`EntityModelInstance`] builder: each `with_field field`
/// entry expands to `.with_field(source.field)`. Only fields copied verbatim
/// (identical builder and source name, no computation) belong here; every derived
/// projection stays in the hand-written chain above the invocation. `macro_rules!`
/// cannot synthesize the `with_` builder name, so each entry spells it out,
/// mirroring the renderer-side `entity_render_state!` per-field list.
macro_rules! entity_render_state_passthrough {
    (
        $base:expr, $source:expr
        $(, $with:ident $field:ident )* $(,)?
    ) => {
        $base $( .$with($source.$field) )*
    };
}

mod entity_data;
mod hands;
mod instance;
mod kind;
mod variants;

pub(crate) use entity_data::armor_material;
use entity_data::*;
use hands::*;
use instance::*;
use kind::*;
use variants::*;

pub(crate) fn default_spear_kinetic_weapon_for_resource_id(
    resource_id: &str,
) -> Option<SpearKineticWeapon> {
    hands::spear_kinetic_weapon_for_resource_id(resource_id)
}

#[cfg(test)]
mod tests;
