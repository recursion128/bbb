use bbb_protocol::packets::{EntityDataRegistryHolder, EntityDataValueKind};
use bbb_renderer::{
    ArmorStandModelPose, BoatModelFamily, CamelModelFamily, ChickenModelVariant, CowModelVariant,
    DonkeyModelFamily, EntityDyeColor, EntityModelInstance, EntityModelKind, HoglinModelFamily,
    HumanoidModelFamily, IllagerModelFamily, LlamaModelFamily, LlamaVariant, PigModelVariant,
    PiglinModelFamily, PlayerModelPartVisibility, QuadrupedModelFamily, SelectionBox,
    SelectionOutline, SheepHeadEatPose, SheepWoolColor, SkeletonModelFamily, SleepingPose,
    UndeadHorseModelFamily, ZombieVariantModelFamily, DEFAULT_ARMOR_STAND_MODEL_POSE,
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
/// `TamableAnimal` `DATA_FLAGS_ID` sitting bit (`isInSittingPose()` reads `& 1`).
const TAMABLE_ANIMAL_SITTING_FLAG: i8 = 0x01;
const WOLF_COLLAR_COLOR_DATA_ID: u8 = 21;
const WOLF_ANGER_END_TIME_DATA_ID: u8 = 22;
const WOLF_DEFAULT_COLLAR_COLOR_ID: i32 = 14;
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
        .with_walk_animation(source.walk_animation_position, source.walk_animation_speed)
        .with_age_in_ticks(source.age_ticks as f32 + entity_partial_tick)
        .with_wolf_tail_angle(wolf_tail_angle(
            source.entity_type_id,
            &source.data_values,
            game_time,
        ))
        .with_wolf_sitting(wolf_sitting(source.entity_type_id, &source.data_values))
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
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID
        | VANILLA_ENTITY_TYPE_PANDA_ID
        | VANILLA_ENTITY_TYPE_SNIFFER_ID => {
            quadruped(QuadrupedModelFamily::Cow, ageable_baby(data_values))
        }
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
        VANILLA_ENTITY_TYPE_NAUTILUS_ID | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
            quadruped(QuadrupedModelFamily::Horse, ageable_baby(data_values))
        }
        VANILLA_ENTITY_TYPE_WOLF_ID => wolf_model_kind(data_values, game_time),
        VANILLA_ENTITY_TYPE_CAT_ID
        | VANILLA_ENTITY_TYPE_OCELOT_ID
        | VANILLA_ENTITY_TYPE_FOX_ID
        | VANILLA_ENTITY_TYPE_RABBIT_ID => {
            quadruped(QuadrupedModelFamily::Wolf, ageable_baby(data_values))
        }
        VANILLA_ENTITY_TYPE_MINECART_ID
        | VANILLA_ENTITY_TYPE_CHEST_MINECART_ID
        | VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID
        | VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
        | VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID
        | VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID
        | VANILLA_ENTITY_TYPE_TNT_MINECART_ID => EntityModelKind::Minecart,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID => {
            placeholder("todo_area_effect_cloud_bounds", 1.0, 0.5, 1.0)
        }
        VANILLA_ENTITY_TYPE_ARROW_ID | VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID => {
            placeholder("todo_arrow_bounds", 0.5, 0.5, 0.5)
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
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => {
            placeholder("todo_ender_dragon_bounds", 16.0, 8.0, 16.0)
        }
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
        VANILLA_ENTITY_TYPE_ALLAY_ID => placeholder("todo_allay_bounds", 0.35, 0.6, 0.35),
        VANILLA_ENTITY_TYPE_ARMADILLO_ID => placeholder("todo_armadillo_bounds", 0.7, 0.65, 0.7),
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => placeholder("todo_axolotl_bounds", 0.75, 0.42, 0.75),
        VANILLA_ENTITY_TYPE_BAT_ID => placeholder("todo_bat_bounds", 0.5, 0.9, 0.5),
        VANILLA_ENTITY_TYPE_BEE_ID => placeholder("todo_bee_bounds", 0.7, 0.6, 0.7),
        VANILLA_ENTITY_TYPE_BLAZE_ID => EntityModelKind::Blaze,
        VANILLA_ENTITY_TYPE_BREEZE_ID => placeholder("todo_breeze_bounds", 0.6, 1.77, 0.6),
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID => {
            placeholder("todo_breeze_wind_charge_bounds", 0.3125, 0.3125, 0.3125)
        }
        VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID => EntityModelKind::CaveSpider,
        VANILLA_ENTITY_TYPE_COD_ID => placeholder("todo_cod_bounds", 0.5, 0.3, 0.5),
        VANILLA_ENTITY_TYPE_CREAKING_ID => placeholder("todo_creaking_bounds", 0.9, 2.7, 0.9),
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => placeholder("todo_dolphin_bounds", 0.9, 0.6, 0.9),
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID => {
            placeholder("todo_elder_guardian_bounds", 1.9975, 1.9975, 1.9975)
        }
        VANILLA_ENTITY_TYPE_ENDERMITE_ID => EntityModelKind::Endermite,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID => placeholder("todo_end_crystal_bounds", 2.0, 2.0, 2.0),
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID => {
            placeholder("todo_evoker_fangs_bounds", 0.5, 0.8, 0.5)
        }
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
        VANILLA_ENTITY_TYPE_FROG_ID => placeholder("todo_frog_bounds", 0.5, 0.5, 0.5),
        VANILLA_ENTITY_TYPE_GHAST_ID => EntityModelKind::Ghast,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => placeholder("todo_happy_ghast_bounds", 4.0, 4.0, 4.0),
        VANILLA_ENTITY_TYPE_GIANT_ID => placeholder("todo_giant_bounds", 3.6, 12.0, 3.6),
        VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID => {
            placeholder("todo_glow_item_frame_bounds", 0.75, 0.75, 0.0625)
        }
        VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => placeholder("todo_glow_squid_bounds", 0.8, 0.8, 0.8),
        VANILLA_ENTITY_TYPE_GUARDIAN_ID => placeholder("todo_guardian_bounds", 0.85, 0.85, 0.85),
        VANILLA_ENTITY_TYPE_INTERACTION_ID => placeholder("todo_interaction_bounds", 1.0, 1.0, 1.0),
        VANILLA_ENTITY_TYPE_ITEM_ID => placeholder("todo_item_entity_bounds", 0.25, 0.25, 0.25),
        VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID => {
            placeholder("todo_item_display_bounds", 1.0, 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID => {
            placeholder("todo_item_frame_bounds", 0.75, 0.75, 0.0625)
        }
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID => {
            placeholder("todo_leash_knot_bounds", 0.375, 0.5, 0.375)
        }
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID => {
            placeholder("todo_lightning_bolt_bounds", 0.5, 2.0, 0.5)
        }
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID => {
            placeholder("todo_llama_spit_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => EntityModelKind::MagmaCube {
            size: slime_size(data_values),
        },
        VANILLA_ENTITY_TYPE_MARKER_ID => placeholder("todo_marker_bounds", 0.0625, 0.0625, 0.0625),
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID => {
            placeholder("todo_ominous_item_spawner_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_PAINTING_ID => placeholder("todo_painting_bounds", 1.0, 1.0, 0.0625),
        VANILLA_ENTITY_TYPE_PARROT_ID => placeholder("todo_parrot_bounds", 0.5, 0.9, 0.5),
        VANILLA_ENTITY_TYPE_PHANTOM_ID => EntityModelKind::Phantom {
            size: phantom_size(data_values),
        },
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: false,
        },
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID => placeholder("todo_pufferfish_bounds", 0.7, 0.7, 0.7),
        VANILLA_ENTITY_TYPE_SALMON_ID => placeholder("todo_salmon_bounds", 0.7, 0.4, 0.7),
        VANILLA_ENTITY_TYPE_SHULKER_ID => placeholder("todo_shulker_bounds", 1.0, 1.0, 1.0),
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID => {
            placeholder("todo_shulker_bullet_bounds", 0.3125, 0.3125, 0.3125)
        }
        VANILLA_ENTITY_TYPE_SILVERFISH_ID => EntityModelKind::Silverfish,
        VANILLA_ENTITY_TYPE_SLIME_ID => EntityModelKind::Slime {
            size: slime_size(data_values),
        },
        VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID => {
            placeholder("todo_small_fireball_bounds", 0.3125, 0.3125, 0.3125)
        }
        VANILLA_ENTITY_TYPE_SPIDER_ID => EntityModelKind::Spider,
        VANILLA_ENTITY_TYPE_SQUID_ID => placeholder("todo_squid_bounds", 0.8, 0.8, 0.8),
        VANILLA_ENTITY_TYPE_STRIDER_ID => {
            quadruped(QuadrupedModelFamily::Horse, ageable_baby(data_values))
        }
        VANILLA_ENTITY_TYPE_TADPOLE_ID => placeholder("todo_tadpole_bounds", 0.4, 0.3, 0.4),
        VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID => {
            placeholder("todo_text_display_bounds", 1.0, 0.5, 0.0625)
        }
        VANILLA_ENTITY_TYPE_TNT_ID => placeholder("todo_tnt_bounds", 0.98, 0.98, 0.98),
        VANILLA_ENTITY_TYPE_TRIDENT_ID => placeholder("todo_trident_bounds", 0.5, 0.5, 0.5),
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => {
            placeholder("todo_tropical_fish_bounds", 0.5, 0.4, 0.5)
        }
        VANILLA_ENTITY_TYPE_TURTLE_ID => placeholder("todo_turtle_bounds", 1.2, 0.4, 1.2),
        VANILLA_ENTITY_TYPE_VEX_ID => placeholder("todo_vex_bounds", 0.4, 0.8, 0.4),
        VANILLA_ENTITY_TYPE_WARDEN_ID => placeholder("todo_warden_bounds", 0.9, 2.9, 0.9),
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID => {
            placeholder("todo_wind_charge_bounds", 0.3125, 0.3125, 0.3125)
        }
        VANILLA_ENTITY_TYPE_WITHER_ID => placeholder("todo_wither_bounds", 0.9, 3.5, 0.9),
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID => {
            placeholder("todo_wither_skull_bounds", 0.3125, 0.3125, 0.3125)
        }
        _ => placeholder("todo_unknown_entity_type_bounds", 0.75, 0.75, 0.75),
    }
}

fn humanoid(family: HumanoidModelFamily, baby: bool) -> EntityModelKind {
    EntityModelKind::Humanoid { family, baby }
}

fn quadruped(family: QuadrupedModelFamily, baby: bool) -> EntityModelKind {
    EntityModelKind::Quadruped { family, baby }
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

fn ageable_baby(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_data_bool(values, AGEABLE_MOB_BABY_DATA_ID, false)
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
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_MOOSHROOM_ID, &[]),
            quadruped(QuadrupedModelFamily::Cow, false)
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
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_MOOSHROOM_ID, &[]),
            quadruped(QuadrupedModelFamily::Cow, false)
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
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_MOOSHROOM_ID, &[]),
            quadruped(QuadrupedModelFamily::Cow, false)
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_ravagers() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_RAVAGER_ID, &[]),
            EntityModelKind::Ravager
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SNIFFER_ID, &[]),
            quadruped(QuadrupedModelFamily::Cow, false)
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
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PANDA_ID, &[]),
            quadruped(QuadrupedModelFamily::Cow, false)
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
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_CAT_ID, &[]),
            quadruped(QuadrupedModelFamily::Wolf, false)
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
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_NAUTILUS_ID, &[]),
            quadruped(QuadrupedModelFamily::Horse, false)
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
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_NAUTILUS_ID, &[]),
            quadruped(QuadrupedModelFamily::Horse, false)
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
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, &[]),
            quadruped(QuadrupedModelFamily::Horse, false)
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
