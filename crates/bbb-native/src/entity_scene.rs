use bbb_protocol::packets::{
    EntityDataEnumSerializer, EntityDataRegistryHolder, EntityDataValueKind,
};
use bbb_renderer::{
    ArmorStandModelPose, BoatModelFamily, CamelModelFamily, ChickenModelVariant, CowModelVariant,
    DonkeyModelFamily, EntityDyeColor, EntityModelInstance, EntityModelKind, HoglinModelFamily,
    HumanoidModelFamily, IllagerModelFamily, LlamaModelFamily, LlamaVariant, PigModelVariant,
    PiglinModelFamily, PlayerModelPartVisibility, SalmonModelSize, SelectionBox, SelectionOutline,
    SheepHeadEatPose, SheepWoolColor, SkeletonModelFamily, SleepingPose, TropicalFishModelShape,
    TropicalFishPattern, UndeadHorseModelFamily, ZombieVariantModelFamily,
    DEFAULT_ARMOR_STAND_MODEL_POSE,
};
use bbb_world::{EntityModelSourceState, EntityPickTargetState, RegistryContentState, WorldStore};

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
const AVATAR_MODEL_CUSTOMIZATION_DATA_ID: u8 = 16;
const AVATAR_PLAYER_DEFAULT_MODEL_CUSTOMIZATION: i8 = 0;
const MANNEQUIN_DEFAULT_MODEL_CUSTOMIZATION: i8 = PlayerModelPartVisibility::ALL_MASK as i8;
const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
const ENTITY_SHARED_FLAG_ON_FIRE: i8 = 0x01;
const ENTITY_SHARED_FLAG_INVISIBLE: i8 = 0x20;
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
const PIGLIN_BABY_DATA_ID: u8 = 17;
const BOGGED_SHEARED_DATA_ID: u8 = 16;
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
const ARMOR_STAND_CLIENT_FLAGS_DATA_ID: u8 = 16;
const ARMOR_STAND_HEAD_POSE_DATA_ID: u8 = 17;
const ARMOR_STAND_BODY_POSE_DATA_ID: u8 = 18;
const ARMOR_STAND_LEFT_ARM_POSE_DATA_ID: u8 = 19;
const ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID: u8 = 20;
const ARMOR_STAND_LEFT_LEG_POSE_DATA_ID: u8 = 21;
const ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID: u8 = 22;
const ARMOR_STAND_CLIENT_FLAG_SMALL: i8 = 1;
const ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS: i8 = 4;
const ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE: i8 = 8;
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
const LLAMA_VARIANT_DATA_ID: u8 = 21;
const GOAT_LEFT_HORN_DATA_ID: u8 = 19;
const GOAT_RIGHT_HORN_DATA_ID: u8 = 20;
const CHICKEN_VARIANT_DATA_ID: u8 = 18;
const COW_VARIANT_DATA_ID: u8 = 18;
const PIG_VARIANT_DATA_ID: u8 = 19;
const SHEEP_WOOL_DATA_ID: u8 = 17;
const SHEEP_WOOL_COLOR_MASK: u8 = 0x0f;
const SHEEP_WOOL_SHEARED_FLAG: u8 = 0x10;
const TAMABLE_ANIMAL_FLAGS_DATA_ID: u8 = 18;
const TAMABLE_ANIMAL_TAME_FLAG: i8 = 0x04;
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
/// `Bee.DATA_ANGER_END_TIME` data id (18): the synced `NeutralMob` anger-end game time,
/// defined right after `Bee.DATA_FLAGS_ID` (17). `Bee.isAngry()` is `endTime > 0 &&
/// endTime - gameTime > 0`.
const BEE_ANGER_END_TIME_DATA_ID: u8 = 18;
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
    entity_partial_tick: f32,
) -> Vec<EntityModelInstance> {
    let entity_partial_tick = entity_partial_tick.clamp(0.0, 1.0);
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.local_player().camera.entity_id;
    let chicken_variants = world.registry_content("minecraft:chicken_variant");
    let cow_variants = world.registry_content("minecraft:cow_variant");
    let pig_variants = world.registry_content("minecraft:pig_variant");
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
                game_time,
                entity_partial_tick,
                chicken_variants,
                cow_variants,
                pig_variants,
            )
        })
        .collect()
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

fn entity_model_instance(
    source: EntityModelSourceState,
    game_time: i64,
    entity_partial_tick: f32,
    chicken_variants: Option<&RegistryContentState>,
    cow_variants: Option<&RegistryContentState>,
    pig_variants: Option<&RegistryContentState>,
) -> Option<EntityModelInstance> {
    let kind = entity_model_kind_with_time_and_registries(
        source.entity_type_id,
        &source.data_values,
        source.age_ticks as f32 + entity_partial_tick,
        game_time,
        chicken_variants,
        cow_variants,
        pig_variants,
    );
    let head_eat = sheep_head_eat_pose(
        source.entity_type_id,
        source.sheep_eat_animation_tick,
        entity_partial_tick,
    );
    let light_coords = entity_light_coords(&source.data_values, source.light);
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
    let net_head_yaw = wrap_degrees(source.y_head_rot - source.y_rot) * head_sign;
    let head_pitch = source.x_rot * head_sign;
    let is_shaking = entity_shaking(
        source.entity_type_id,
        &source.data_values,
        source.is_fully_frozen,
    );
    let body_rot = source.y_rot + entity_body_shake_degrees(source.age_ticks, is_shaking);
    // Vanilla LivingEntityRenderer.setupRotations riptide branch reads the lerped
    // `state.ageInTicks` (= tickCount + partialTick) only while `isAutoSpinAttack`.
    let auto_spin_age_ticks = source
        .is_auto_spin_attack
        .then_some(source.age_ticks as f32 + entity_partial_tick);
    // Vanilla setupRotations lifts the upside-down model by its bounding box height.
    let upside_down_height = source.is_upside_down.then_some(source.bounding_box_height);
    // Vanilla setupRotations sleeping branch: the bed yaw (or the body yaw when not
    // in a bed) plus the bed head-offset translate.
    let sleeping = source.is_sleeping.then_some(SleepingPose {
        yaw_angle: source.sleeping_bed_yaw.unwrap_or(body_rot),
        bed_offset: source.sleeping_bed_offset,
    });
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
        .with_polar_bear_stand_scale(source.polar_bear_stand_scale)
        .with_light_coords(light_coords)
        .with_has_red_overlay(source.has_red_overlay)
        .with_death_time(source.death_time)
        .with_auto_spin_age_ticks(auto_spin_age_ticks)
        .with_upside_down_height(upside_down_height)
        .with_sleeping(sleeping)
        .with_scale(source.scale)
        .with_in_water(source.in_water)
        .with_on_ground(source.on_ground)
        .with_is_moving(source.is_moving)
        .with_walk_animation(source.walk_animation_position, source.walk_animation_speed)
        .with_age_in_ticks(source.age_ticks as f32 + entity_partial_tick)
        .with_is_aggressive(source.is_aggressive)
        .with_enderman_carrying(source.enderman_carrying)
        .with_enderman_creepy(source.enderman_creepy)
        .with_bat_resting(source.bat_resting)
        .with_bee_has_stinger(source.bee_has_stinger)
        .with_bee_angry(bee_is_angry(
            source.entity_type_id,
            &source.data_values,
            game_time,
        ))
        .with_vex_charging(source.vex_charging)
        .with_is_crouching(source.is_crouching)
        .with_wolf_tail_angle(wolf_tail_angle(
            source.entity_type_id,
            &source.data_values,
            game_time,
        ))
        .with_wolf_sitting(wolf_sitting(source.entity_type_id, &source.data_values))
        .with_parrot_sitting(parrot_sitting(source.entity_type_id, &source.data_values))
        .with_illager_spellcasting(illager_spellcasting(
            source.entity_type_id,
            &source.data_values,
        ))
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
        .with_shulker_peek(source.shulker_peek)
        .with_tendril_animation(source.tendril_animation)
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
fn entity_light_coords(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    light: bbb_world::TerrainLight,
) -> u32 {
    let on_fire = (entity_data_byte(data_values, ENTITY_SHARED_FLAGS_DATA_ID, 0)
        & ENTITY_SHARED_FLAG_ON_FIRE)
        != 0;
    let block = if on_fire {
        15
    } else {
        u32::from(light.block.min(15))
    };
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
    entity_model_kind_with_registries(entity_type_id, data_values, None, None, None)
}

fn entity_model_kind_with_registries(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    chicken_variants: Option<&RegistryContentState>,
    cow_variants: Option<&RegistryContentState>,
    pig_variants: Option<&RegistryContentState>,
) -> EntityModelKind {
    entity_model_kind_with_time_and_registries(
        entity_type_id,
        data_values,
        0.0,
        0,
        chicken_variants,
        cow_variants,
        pig_variants,
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
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => EntityModelKind::IronGolem,
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID => EntityModelKind::SnowGolem,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => humanoid(HumanoidModelFamily::Player, false),
        VANILLA_ENTITY_TYPE_CREEPER_ID => EntityModelKind::Creeper,
        VANILLA_ENTITY_TYPE_PIG_ID => pig_model_kind(data_values, pig_variants),
        VANILLA_ENTITY_TYPE_COW_ID => cow_model_kind(data_values, cow_variants),
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID => mooshroom_model_kind(data_values),
        VANILLA_ENTITY_TYPE_PANDA_ID => panda_model_kind(data_values),
        VANILLA_ENTITY_TYPE_SNIFFER_ID => EntityModelKind::Sniffer,
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
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => zombie_nautilus_model_kind(),
        VANILLA_ENTITY_TYPE_WOLF_ID => wolf_model_kind(data_values, game_time),
        VANILLA_ENTITY_TYPE_FOX_ID => fox_model_kind(data_values),
        VANILLA_ENTITY_TYPE_CAT_ID => feline_model_kind(data_values, true),
        VANILLA_ENTITY_TYPE_OCELOT_ID => feline_model_kind(data_values, false),
        VANILLA_ENTITY_TYPE_RABBIT_ID => rabbit_model_kind(data_values),
        VANILLA_ENTITY_TYPE_MINECART_ID
        | VANILLA_ENTITY_TYPE_CHEST_MINECART_ID
        | VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID
        | VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
        | VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID
        | VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID
        | VANILLA_ENTITY_TYPE_TNT_MINECART_ID => EntityModelKind::Minecart,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_ARROW_ID | VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID => {
            EntityModelKind::Arrow
        }
        VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID => {
            placeholder("todo_block_display_bounds", 1.0, 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID => {
            placeholder("todo_dragon_fireball_bounds", 1.0, 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_EGG_ID | VANILLA_ENTITY_TYPE_SNOWBALL_ID => {
            placeholder("todo_thrown_item_projectile_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => EntityModelKind::EnderDragon,
        VANILLA_ENTITY_TYPE_ENDER_PEARL_ID => {
            placeholder("todo_ender_pearl_bounds", 0.25, 0.25, 0.25)
        }
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
        },
        VANILLA_ENTITY_TYPE_BAT_ID => EntityModelKind::Bat,
        VANILLA_ENTITY_TYPE_BEE_ID => EntityModelKind::Bee {
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_BLAZE_ID => EntityModelKind::Blaze,
        VANILLA_ENTITY_TYPE_BREEZE_ID => EntityModelKind::Breeze,
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID => EntityModelKind::WindCharge,
        VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID => EntityModelKind::CaveSpider,
        VANILLA_ENTITY_TYPE_COD_ID => EntityModelKind::Cod,
        VANILLA_ENTITY_TYPE_CREAKING_ID => EntityModelKind::Creaking,
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => EntityModelKind::Dolphin {
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID => EntityModelKind::Guardian { elder: true },
        VANILLA_ENTITY_TYPE_ENDERMITE_ID => EntityModelKind::Endermite,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID => EntityModelKind::EndCrystal,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID => EntityModelKind::EvokerFangs,
        VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID
        | VANILLA_ENTITY_TYPE_SPLASH_POTION_ID
        | VANILLA_ENTITY_TYPE_LINGERING_POTION_ID => {
            placeholder("todo_thrown_bottle_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID => {
            placeholder("todo_experience_orb_bounds", 0.5, 0.5, 0.5)
        }
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID => {
            placeholder("todo_eye_of_ender_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID => {
            placeholder("todo_falling_block_bounds", 0.98, 0.98, 0.98)
        }
        VANILLA_ENTITY_TYPE_FIREBALL_ID => placeholder("todo_fireball_bounds", 1.0, 1.0, 1.0),
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID => {
            placeholder("todo_firework_rocket_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID => {
            placeholder("todo_fishing_bobber_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_FROG_ID => EntityModelKind::Frog,
        VANILLA_ENTITY_TYPE_GHAST_ID => EntityModelKind::Ghast,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => EntityModelKind::HappyGhast,
        VANILLA_ENTITY_TYPE_GIANT_ID => EntityModelKind::Giant,
        VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID => {
            placeholder("todo_glow_item_frame_bounds", 0.75, 0.75, 0.0625)
        }
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
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID => {
            placeholder("todo_item_frame_bounds", 0.75, 0.75, 0.0625)
        }
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID => EntityModelKind::LeashKnot,
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID => {
            placeholder("todo_lightning_bolt_bounds", 0.5, 2.0, 0.5)
        }
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID => EntityModelKind::LlamaSpit,
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => EntityModelKind::MagmaCube {
            size: slime_size(data_values),
        },
        VANILLA_ENTITY_TYPE_MARKER_ID => EntityModelKind::NoRender,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID => {
            placeholder("todo_ominous_item_spawner_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_PAINTING_ID => placeholder("todo_painting_bounds", 1.0, 1.0, 0.0625),
        VANILLA_ENTITY_TYPE_PARROT_ID => EntityModelKind::Parrot,
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
        VANILLA_ENTITY_TYPE_SHULKER_ID => EntityModelKind::Shulker,
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID => EntityModelKind::ShulkerBullet,
        VANILLA_ENTITY_TYPE_SILVERFISH_ID => EntityModelKind::Silverfish,
        VANILLA_ENTITY_TYPE_SLIME_ID => EntityModelKind::Slime {
            size: slime_size(data_values),
        },
        VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID => {
            placeholder("todo_small_fireball_bounds", 0.3125, 0.3125, 0.3125)
        }
        VANILLA_ENTITY_TYPE_SPIDER_ID => EntityModelKind::Spider,
        VANILLA_ENTITY_TYPE_SQUID_ID => EntityModelKind::Squid {
            glow: false,
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_STRIDER_ID => EntityModelKind::Strider {
            baby: ageable_baby(data_values),
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
        VANILLA_ENTITY_TYPE_VEX_ID => EntityModelKind::Vex,
        VANILLA_ENTITY_TYPE_WARDEN_ID => EntityModelKind::Warden,
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID => EntityModelKind::WindCharge,
        VANILLA_ENTITY_TYPE_WITHER_ID => EntityModelKind::Wither,
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID => EntityModelKind::WitherSkull,
        _ => placeholder("todo_unknown_entity_type_bounds", 0.75, 0.75, 0.75),
    }
}

fn humanoid(family: HumanoidModelFamily, baby: bool) -> EntityModelKind {
    EntityModelKind::Humanoid { family, baby }
}

/// Vanilla `RabbitRenderer` picks `AdultRabbitModel` for an adult and `BabyRabbitModel` for a baby; both
/// render through the dedicated [`EntityModelKind::Rabbit`] (`baby` selecting the body layout).
fn rabbit_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    EntityModelKind::Rabbit {
        baby: ageable_baby(values),
    }
}

/// Vanilla `PandaRenderer` (an `AgeableMobRenderer`) picks `PandaModel` for an adult and `BabyPandaModel`
/// for a baby; both render through the dedicated [`EntityModelKind::Panda`] (`baby` selecting the layout).
fn panda_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    EntityModelKind::Panda {
        baby: ageable_baby(values),
    }
}

/// Vanilla `CatRenderer` / `OcelotRenderer` (both `AgeableMobRenderer`s) pick `AdultCatModel` /
/// `AdultOcelotModel` (the shared `AdultFelineModel` mesh, the cat scaled 0.8) for an adult and the
/// flatter `BabyFelineModel` mesh (unscaled for both breeds) for a baby. Both render through the
/// dedicated [`EntityModelKind::Feline`] (`cat` selecting the breed/scale, `baby` selecting the layout).
fn feline_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
    cat: bool,
) -> EntityModelKind {
    EntityModelKind::Feline {
        cat,
        baby: ageable_baby(values),
    }
}

/// Vanilla `FoxRenderer` (an `AgeableMobRenderer`) picks `AdultFoxModel` for an adult and `BabyFoxModel`
/// for a baby; both render through the dedicated [`EntityModelKind::Fox`] (`baby` selecting the layout).
fn fox_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    EntityModelKind::Fox {
        baby: ageable_baby(values),
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

/// Vanilla `ZombieNautilusRenderer` (a plain `MobRenderer`, so never a baby) renders the same
/// `NautilusModel.createBodyLayer()` body as the living nautilus (`ModelLayers.ZOMBIE_NAUTILUS` bakes to
/// it), so it shares the dedicated [`EntityModelKind::Nautilus`] adult body. The `WARM` coral variant (a
/// distinct `ZombieNautilusCoralModel` mesh), the separate coral layer, and the armor / saddle equipment
/// layers are deferred, so this replaces the horse-shaped proxy with the real nautilus body.
fn zombie_nautilus_model_kind() -> EntityModelKind {
    EntityModelKind::Nautilus { baby: false }
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
/// generic quadruped stand-in. The mushroom block-model layer and red/brown textures stay deferred.
fn mooshroom_model_kind(values: &[bbb_protocol::packets::EntityDataValue]) -> EntityModelKind {
    EntityModelKind::Mooshroom {
        baby: ageable_baby(values),
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
        invisible: entity_invisible(values),
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
        slim: false,
        parts: PlayerModelPartVisibility::from_vanilla_mask(mask),
    }
}

fn wolf_model_kind(
    values: &[bbb_protocol::packets::EntityDataValue],
    game_time: i64,
) -> EntityModelKind {
    let tame =
        (entity_data_byte(values, TAMABLE_ANIMAL_FLAGS_DATA_ID, 0) & TAMABLE_ANIMAL_TAME_FLAG) != 0;
    EntityModelKind::Wolf {
        baby: ageable_baby(values),
        tame,
        angry: wolf_is_angry(values, game_time),
        invisible: entity_invisible(values),
        collar_color: tame.then(|| {
            EntityDyeColor::from_vanilla_id(entity_data_int(
                values,
                WOLF_COLLAR_COLOR_DATA_ID,
                WOLF_DEFAULT_COLLAR_COLOR_ID,
            ))
        }),
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

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        AddEntity, AttributeSnapshot, CommonPlayerSpawnInfo, EntityDataValue, EntityEvent,
        EntityPositionSync, PlayLogin, PlayTime, SetCamera, SetEntityData, UpdateAttributes, Vec3d,
    };
    use bbb_world::{EntityPickBoundsState, EntityVec3, RegistryPacketEntry};
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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

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
        let resting = entity_model_instances_from_world_at_partial_tick(&world, 0.0);
        assert_eq!(resting[0].render_state.head_eat, SheepHeadEatPose::NONE);
        assert_eq!(resting[1].render_state.head_eat, SheepHeadEatPose::NONE);

        // Vanilla SheepRenderer.extractRenderState projects the eat animation
        // through the partial tick; the chicken stays at rest.
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 70,
            event_id: 10,
        }));
        let eating = entity_model_instances_from_world_at_partial_tick(&world, 0.5);
        assert_eq!(
            eating[0].render_state.head_eat,
            SheepHeadEatPose::from_eat_tick(40, 0.5)
        );
        assert_ne!(eating[0].render_state.head_eat, SheepHeadEatPose::NONE);
        assert_eq!(eating[1].render_state.head_eat, SheepHeadEatPose::NONE);

        // The pose follows the canonical countdown as it decrements.
        world.advance_entity_client_animations(20);
        let mid = entity_model_instances_from_world_at_partial_tick(&world, 0.0);
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
        let resting = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
        assert_eq!(resting[0].render_state.tendril_animation, 0.0);

        // Vanilla Warden.handleEntityEvent(61) resets tendrilAnimation to 10; getTendrilAnimation
        // lerps (tendrilAnimationO, tendrilAnimation) / 10. After three client ticks the pair is
        // (8, 7), so at partialTick 1.0 the projected pulse is 7/10.
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 94,
            event_id: 61,
        }));
        world.advance_entity_client_animations(3);
        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
        assert_eq!(
            instances[0].render_state.tendril_animation,
            7.0 / 10.0,
            "the projected tendril pulse drives the WardenModel.animateTendrils antenna sway"
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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
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
        let resting = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
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
        let standing = entity_model_instances_from_world_at_partial_tick(&world, 0.5);
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
            entity_model_instances_from_world_at_partial_tick(world, partial)
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
    fn entity_model_instances_project_death_animation_time() {
        const VANILLA_ENTITY_HEALTH_DATA_ID: u8 = 9;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            82,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, -2.0],
        ));

        // A living entity at rest carries deathTime 0 and no red overlay.
        let alive = entity_model_instances_from_world_at_partial_tick(&world, 0.0);
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
        let dying = entity_model_instances_from_world_at_partial_tick(&world, 0.25);
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
        let warm = entity_model_instances_from_world_at_partial_tick(&world, 0.0);
        assert_eq!(warm[0].render_state.body_rot, 0.0);

        // Vanilla Entity.isFullyFrozen(): ticksFrozen >= 140. setupRotations then
        // adds cos(floor(ageInTicks) * 3.25) * π * 0.4 to the body yaw; the shake
        // uses the floored (integer) tick count, so it does not lerp with partial.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 83,
            values: vec![protocol_int_data(VANILLA_ENTITY_TICKS_FROZEN_DATA_ID, 140)],
        }));
        world.advance_entity_client_animations(2);
        let frozen = entity_model_instances_from_world_at_partial_tick(&world, 0.25);
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
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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
            entity_model_instances_from_world_at_partial_tick(world, partial)
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
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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

        let charging = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
                .into_iter()
                .find(|instance| instance.entity_id == id)
                .unwrap()
                .render_state
                .vex_charging
        };

        // An idle vex projects vex_charging = false.
        assert!(!charging(&world, 97));

        // Setting Vex.isCharging (DATA_FLAGS_ID & 1) projects through to the charging pose.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 97,
            values: vec![protocol_byte_data(
                VANILLA_VEX_FLAGS_DATA_ID,
                VEX_FLAG_IS_CHARGING
            )],
        }));
        assert!(charging(&world, 97));

        // The same flag byte set on a non-vex (bat) does NOT project vex_charging — the
        // derivation is gated to vanilla_is_vex.
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 98,
            values: vec![protocol_byte_data(
                VANILLA_VEX_FLAGS_DATA_ID,
                VEX_FLAG_IS_CHARGING
            )],
        }));
        assert!(!charging(&world, 98));
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
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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
    fn entity_model_instances_project_bee_stinger() {
        // Vanilla Bee.DATA_FLAGS_ID (17, BYTE) and the has-stung bit (4).
        const VANILLA_BEE_FLAGS_DATA_ID: u8 = 17;
        const BEE_FLAG_HAS_STUNG: i8 = 4;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            96,
            VANILLA_ENTITY_TYPE_BEE_ID,
            [1.0, 64.0, -2.0],
        ));

        let has_stinger = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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
        // Vanilla Bee.DATA_ANGER_END_TIME (18, LONG): isAngry = endTime > 0 && endTime - gameTime
        // > 0. The world has no time set here, so the game time defaults to 0.
        const VANILLA_BEE_ANGER_END_TIME_DATA_ID: u8 = 18;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            97,
            VANILLA_ENTITY_TYPE_BEE_ID,
            [1.0, 64.0, -2.0],
        ));

        let angry = |world: &WorldStore, id: i32| {
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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
            entity_model_instances_from_world_at_partial_tick(world, 1.0)
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
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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
            entity_model_instances_from_world_at_partial_tick(world, 0.0)
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

        let walk = |world: &WorldStore| -> (f32, f32) {
            let state = entity_model_instances_from_world_at_partial_tick(world, 1.0)
                .into_iter()
                .find(|instance| instance.entity_id == 98)
                .unwrap()
                .render_state;
            (state.walk_animation_pos, state.walk_animation_speed)
        };
        let sync = |world: &mut WorldStore, x: f64| {
            assert!(world.apply_entity_position_sync(EntityPositionSync {
                id: 98,
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
        assert_eq!(walk(&world), (0.0, 0.0));

        // After one 0.5-block step, the WalkAnimationState reaches speed = 0.4 and
        // position = 0.4 (targetSpeed = min(0.5 * 4, 1) = 1.0), and both flow through
        // EntityModelSourceState to the renderer EntityRenderState.
        sync(&mut world, 0.5);
        world.advance_entity_client_animations(1);
        let (pos, speed) = walk(&world);
        assert!((speed - 0.4).abs() < 1e-5, "walk speed: {speed}");
        assert!((pos - 0.4).abs() < 1e-5, "walk position: {pos}");
    }

    #[test]
    fn entity_light_coords_packs_vanilla_block_and_sky_with_on_fire_override() {
        use bbb_world::TerrainLight;

        // Daylight surface (block 0, sky 15) -> LightCoordsUtil.pack(0, 15).
        assert_eq!(
            entity_light_coords(&[], TerrainLight { sky: 15, block: 0 }),
            15 << 20
        );
        // Full-bright fallback (block 15, sky 15) -> LightCoordsUtil.FULL_BRIGHT.
        assert_eq!(
            entity_light_coords(&[], TerrainLight { sky: 15, block: 15 }),
            15_728_880
        );
        // Torch-lit cave (block 14, sky 0) -> pack(14, 0).
        assert_eq!(
            entity_light_coords(&[], TerrainLight { sky: 0, block: 14 }),
            14 << 4
        );
        // EntityRenderer.getBlockLightLevel forces block light to 15 on fire,
        // leaving sky light untouched.
        let on_fire = vec![protocol_byte_data(
            ENTITY_SHARED_FLAGS_DATA_ID,
            ENTITY_SHARED_FLAG_ON_FIRE,
        )];
        assert_eq!(
            entity_light_coords(&on_fire, TerrainLight { sky: 4, block: 0 }),
            (15 << 4) | (4 << 20)
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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
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

        let calm = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
        assert!(!calm[0].render_state.has_red_overlay);

        assert!(
            world.apply_hurt_animation(bbb_protocol::packets::HurtAnimation { id: 91, yaw: 0.0 })
        );
        let hurt = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
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
        let resting = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
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
        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
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
        let resting = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
        assert_eq!(
            instances[0].render_state.creeper_swelling,
            5.0 / 28.0,
            "the projected swell drives the renderer inflate-and-flicker scale"
        );
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
                None
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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

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
                None
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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

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
                Some(pig_registry)
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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

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
                        | ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE,
                ),
                protocol_rotations_data(ARMOR_STAND_BODY_POSE_DATA_ID, pose.body),
                protocol_rotations_data(ARMOR_STAND_LEFT_ARM_POSE_DATA_ID, pose.left_arm),
            ],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![EntityModelInstance::armor_stand(
                    5,
                    [1.0, 64.0, -2.0],
                    0.0,
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
        world.apply_add_entity(protocol_add_entity(
            1550,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [1.0, 64.0, -2.0],
        ));
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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

        assert_eq!(
            instances,
            aged(
                vec![
                    EntityModelInstance::player_with_parts(
                        1550,
                        [1.0, 64.0, -2.0],
                        0.0,
                        false,
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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 0.25);

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
        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

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
                        | ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE,
                )],
            ),
            EntityModelKind::ArmorStand {
                small: true,
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
        // The ghast was a placeholder render box; it now resolves to the real model.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_GHAST_ID, &[]),
            EntityModelKind::Ghast
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
        // The vex was a placeholder render box; it now resolves to the real `VexModel`. Its
        // idle wing flap / arm bob / head look read the projected age and look angles; the
        // charging pose is deferred entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_VEX_ID, &[]),
            EntityModelKind::Vex
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
        // The procedural airborne flap / bob reads the projected age and ground state; the anger /
        // rolled-up / nectar states are deferred entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_BEE_ID, &[]),
            EntityModelKind::Bee { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_BEE_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Bee { baby: true }
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
        // at its rest pose. The head look, walk, attack, invulnerable, and death keyframe
        // animations and the emissive eyes layer are deferred entity-side state, so no synced
        // data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_CREAKING_ID, &[]),
            EntityModelKind::Creaking
        );
    }

    #[test]
    fn entity_model_kind_maps_frog_to_real_model() {
        // The frog was a placeholder render box; it now resolves to the real `FrogModel` at its
        // rest pose. The keyframe animations (jump/croak/tongue/swim/walk/idle) and the three
        // texture variants are deferred entity-side state, so no synced data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_FROG_ID, &[]),
            EntityModelKind::Frog
        );
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
    fn entity_model_kind_projects_strider_baby_from_data() {
        // The strider previously fell back to the horse quadruped; it now resolves to the real
        // `AdultStriderModel` / `BabyStriderModel`, keyed off the synced `AgeableMob.DATA_BABY_ID`
        // (index 16, default adult). The body sway/bob, leg swing/lift, and bristle flow read
        // the projected age, walk animation, and look angles; the ridden pose, saddle layer, and
        // cold/suffocating texture are deferred entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_STRIDER_ID, &[]),
            EntityModelKind::Strider { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_STRIDER_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Strider { baby: true }
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
        // tentacle sweep / body tilt are deferred entity-side animation (default rest pose).
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
                invisible: false,
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
                invisible: false,
                jeb: false,
                age_ticks: 0.0,
            }
        );
        // The mooshroom shares the cow body, so it renders through the dedicated `Mooshroom` model
        // (the real cow mesh) rather than the generic quadruped stand-in — adult and baby alike.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_MOOSHROOM_ID, &[]),
            EntityModelKind::Mooshroom { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Mooshroom { baby: true }
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
                invisible: false,
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
                invisible: false,
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
                invisible: false,
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
                invisible: true,
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
            ),
            EntityModelKind::Sheep {
                baby: false,
                sheared: false,
                wool_color: SheepWoolColor::White,
                invisible: false,
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
            ),
            EntityModelKind::Sheep {
                baby: false,
                sheared: false,
                wool_color: SheepWoolColor::White,
                invisible: false,
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
            ),
            EntityModelKind::Sheep {
                baby: false,
                sheared: false,
                wool_color: SheepWoolColor::White,
                invisible: false,
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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 0.5);

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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 0.25);

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
        // `SnifferModel` at its rest pose. The head look, search/walk, and the dig / long-sniff /
        // stand-up / happy / scenting keyframe animations are deferred entity-side state, so no
        // synced data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SNIFFER_ID, &[]),
            EntityModelKind::Sniffer
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
        // `applyWalk` leg sway, and the roll-out / roll-up / peek keyframe transitions are deferred
        // entity-side state.
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
        // adult), as in the vanilla `AgeableMobRenderer`. The body yaw, the procedural / keyframe
        // swim-walk-idle animations, the play-dead pose, the mirror-leg copy, and the five color
        // variants are deferred entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_AXOLOTL_ID, &[]),
            EntityModelKind::Axolotl { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_AXOLOTL_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Axolotl { baby: true }
        );
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
        // The parrot was a placeholder bounds box; it now resolves to the real `ParrotModel` at its
        // STANDING rest pose. The head look, per-pose offsets, wing flap / dance animations, and the
        // five color variants are deferred entity-side state, so no synced data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PARROT_ID, &[]),
            EntityModelKind::Parrot
        );
    }

    #[test]
    fn entity_model_kind_maps_shulker_to_real_model() {
        // The shulker was a placeholder bounds box; it now resolves to the real `ShulkerModel` at
        // its closed rest pose. The peek open/close, head look, attach-face rotation, and the
        // sixteen dye-color variants are deferred entity-side state, so no synced data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SHULKER_ID, &[]),
            EntityModelKind::Shulker
        );
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
        // projected look angles and walk animation; the armor / item-in-hand layers and the zombie
        // texture are deferred. The giant is never a baby, so no baby flag is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_GIANT_ID, &[]),
            EntityModelKind::Giant
        );
    }

    #[test]
    fn entity_model_kind_maps_end_crystal_to_real_model() {
        // The end crystal was a placeholder bounds box; it now resolves to the real `EndCrystalModel`
        // at its rest pose. The diagonal spin, the vertical bob, the `showsBottom` base toggle, and
        // the beam to the dragon are deferred entity-side state, so no synced data is read.
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
        // `ArrowModel`. They share one model, differing only in the deferred tipped/spectral texture,
        // so both type ids map to `Arrow`. The impact-shake wobble is deferred entity-side state.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ARROW_ID, &[]),
            EntityModelKind::Arrow
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID, &[]),
            EntityModelKind::Arrow
        );
    }

    #[test]
    fn entity_model_kind_maps_trident_to_real_model() {
        // The thrown trident was a placeholder box; it now resolves to the real `TridentModel`. The
        // model has no animation, so the geometry is complete; the enchant-foil overlay and the
        // texture are deferred entity-side state, so no synced data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_TRIDENT_ID, &[]),
            EntityModelKind::Trident
        );
    }

    #[test]
    fn entity_model_kind_maps_wither_skull_to_real_model() {
        // The wither skull was a placeholder box; it now resolves to the real `SkullModel`. Its flight
        // facing comes from the projected yaw/pitch (a plain `EntityRenderer`); the wither /
        // invulnerable textures and the `isDangerous` swap are deferred entity-side state, so no synced
        // data is read.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_WITHER_SKULL_ID, &[]),
            EntityModelKind::WitherSkull
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
        // dissolve, the emissive eyes, and the crystal beam are deferred entity-side state, so no
        // synced data is read.
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
        // The panda (adult and baby) renders through its dedicated `PandaModel` / `BabyPandaModel`.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PANDA_ID, &[]),
            EntityModelKind::Panda { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_PANDA_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Panda { baby: true }
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
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_wolves() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_WOLF_ID, &[]),
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                invisible: false,
                collar_color: None,
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
                invisible: false,
                collar_color: None,
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
                invisible: false,
                collar_color: Some(EntityDyeColor::Red),
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
                invisible: false,
                collar_color: Some(EntityDyeColor::Blue),
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
                invisible: true,
                collar_color: Some(EntityDyeColor::Blue),
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
                invisible: false,
                collar_color: None,
            }
        );
        // The adult cat, ocelot, and fox render through their dedicated models (cat = the shared
        // `AdultFelineModel` scaled 0.8, ocelot = the unscaled feline, fox = `AdultFoxModel`); each baby
        // now renders through its own dedicated vanilla mesh.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_CAT_ID, &[]),
            EntityModelKind::Feline {
                cat: true,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_OCELOT_ID, &[]),
            EntityModelKind::Feline {
                cat: false,
                baby: false
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_FOX_ID, &[]),
            EntityModelKind::Fox { baby: false }
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
                baby: true
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_OCELOT_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Feline {
                cat: false,
                baby: true
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_FOX_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Fox { baby: true }
        );
        // The rabbit (adult and baby) renders through its dedicated `AdultRabbitModel` / `BabyRabbitModel`.
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_RABBIT_ID, &[]),
            EntityModelKind::Rabbit { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_RABBIT_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Rabbit { baby: true }
        );
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
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: true,
                invisible: false,
                collar_color: None,
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
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                invisible: false,
                collar_color: None,
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
            ),
            EntityModelKind::Wolf {
                baby: false,
                tame: true,
                angry: true,
                invisible: false,
                collar_color: Some(EntityDyeColor::Red),
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

        let angry_instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

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

        let calm_instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
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
        let wild_instances = entity_model_instances_from_world_at_partial_tick(&wild, 1.0);
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
        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
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
        let standing = entity_model_instances_from_world_at_partial_tick(&world, 1.0);
        assert!(
            !standing[0].render_state.wolf_sitting,
            "a standing wolf does not project wolf_sitting"
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
            entity_model_instances_from_world_at_partial_tick(world, 1.0)
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
            entity_model_instances_from_world_at_partial_tick(world, 1.0)
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
            EntityModelKind::Horse { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_HORSE_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Horse { baby: true }
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
        // (`createBodyMesh` / `createBabyBodyLayer`). The zombie nautilus reuses the same adult nautilus
        // body (`ModelLayers.ZOMBIE_NAUTILUS` bakes to `NautilusModel.createBodyLayer()`), so it too maps
        // to the dedicated model (always adult — it is a plain `MobRenderer`); only its coral layer /
        // `WARM` coral variant stay deferred.
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
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, &[]),
            EntityModelKind::Nautilus { baby: false }
        );
        // The zombie nautilus is never a baby, so the baby flag in its metadata is ignored.
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Nautilus { baby: false }
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
                slim: false,
                parts: PlayerModelPartVisibility::from_vanilla_mask(0),
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, &[]),
            EntityModelKind::Player {
                slim: false,
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
                slim: false,
                parts: hat_and_left_sleeve,
            }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
                &[protocol_byte_data(AVATAR_MODEL_CUSTOMIZATION_DATA_ID, 0)],
            ),
            EntityModelKind::Player {
                slim: false,
                parts: PlayerModelPartVisibility::from_vanilla_mask(0),
            }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, &[]),
            humanoid(HumanoidModelFamily::Player, false)
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
            entity_model_kind(VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, &[]),
            humanoid(HumanoidModelFamily::Player, false)
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, &[]),
            EntityModelKind::SnowGolem
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_iron_golem() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, &[]),
            EntityModelKind::IronGolem
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, &[]),
            humanoid(HumanoidModelFamily::Player, false)
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
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, &[]),
            humanoid(HumanoidModelFamily::Player, false)
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

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

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

    fn protocol_int_data(data_id: u8, value: i32) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 1,
            value: EntityDataValueKind::Int(value),
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
