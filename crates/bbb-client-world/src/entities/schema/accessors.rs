//! Typed `(data_id, default)` views over the generated metadata families.
//!
//! Every accessor is a compile-time constant gated by family membership of
//! the entity type's metadata class.

use super::*;

/// Returns `value` when the entity type's metadata class is `family` or inherits it.
fn for_family<T>(entity_type_id: i32, family: VanillaEntityMetadataFamily, value: T) -> Option<T> {
    vanilla_entity_type_extends(entity_type_id, family).then_some(value)
}

macro_rules! field_accessors {
    ($($vis:vis fn $name:ident -> $kind:ident($($field:ident)|+);)*) => {$(
        $vis fn $name(entity_type_id: i32) -> Option<field_accessors!(@ty $kind)> {
            $(
                if vanilla_entity_type_extends(entity_type_id, $field.family) {
                    const VALUE: field_accessors!(@ty $kind) = field_accessors!(@value $kind $field);
                    return Some(VALUE);
                }
            )+
            None
        }
    )*};
    (@ty id) => { u8 };
    (@ty bool) => { (u8, bool) };
    (@ty byte) => { (u8, i8) };
    (@ty int) => { (u8, i32) };
    (@ty long) => { (u8, i64) };
    (@ty float) => { (u8, f32) };
    (@ty resource) => { (u8, &'static str) };
    (@value id $field:ident) => { $field.data_id };
    (@value bool $field:ident) => { ($field.data_id, $field.bool_default()) };
    (@value byte $field:ident) => { ($field.data_id, $field.byte_default()) };
    (@value int $field:ident) => { ($field.data_id, $field.int_default()) };
    (@value long $field:ident) => { ($field.data_id, $field.long_default()) };
    (@value float $field:ident) => { ($field.data_id, $field.float_default()) };
    (@value resource $field:ident) => { ($field.data_id, $field.resource_default()) };
}

field_accessors! {
    pub fn vanilla_entity_metadata_player_absorption_accessor_for_entity_type
        -> float(FIELD_PLAYER_DATA_PLAYER_ABSORPTION_ID);
    pub fn vanilla_entity_metadata_player_score_accessor_for_entity_type
        -> int(FIELD_PLAYER_DATA_SCORE_ID);
    pub fn vanilla_entity_metadata_villager_data_accessor_for_entity_type
        -> id(FIELD_VILLAGER_DATA_VILLAGER_DATA | FIELD_ZOMBIE_VILLAGER_DATA_VILLAGER_DATA);
    pub fn vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type
        -> bool(FIELD_SKELETON_DATA_STRAY_CONVERSION_ID);
    pub(crate) fn vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type
        -> int(FIELD_PIG_DATA_BOOST_TIME | FIELD_STRIDER_DATA_BOOST_TIME);
    pub fn vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type
        -> bool(FIELD_STRIDER_DATA_SUFFOCATING);
    pub fn vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type
        -> bool(FIELD_ZOMBIE_VILLAGER_DATA_CONVERTING_ID);
    pub fn vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type
        -> bool(FIELD_ABSTRACT_PIGLIN_DATA_IMMUNE_TO_ZOMBIFICATION
            | FIELD_HOGLIN_DATA_IMMUNE_TO_ZOMBIFICATION);
    pub fn vanilla_entity_metadata_ghast_charging_accessor_for_entity_type
        -> bool(FIELD_GHAST_DATA_IS_CHARGING);
    pub fn vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type
        -> int(FIELD_GLOW_SQUID_DATA_DARK_TICKS_REMAINING);
    pub fn vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type
        -> bool(FIELD_WITHER_SKULL_DATA_DANGEROUS);
    pub fn vanilla_entity_metadata_painting_variant_accessor_for_entity_type
        -> id(FIELD_PAINTING_DATA_PAINTING_VARIANT_ID);
    pub fn vanilla_entity_metadata_rabbit_type_accessor_for_entity_type
        -> int(FIELD_RABBIT_DATA_TYPE_ID);
    pub fn vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type
        -> int(FIELD_PUFFERFISH_PUFF_STATE);
    pub fn vanilla_entity_metadata_slime_size_accessor_for_entity_type
        -> int(FIELD_SLIME_ID_SIZE);
    pub fn vanilla_entity_metadata_phantom_size_accessor_for_entity_type
        -> int(FIELD_PHANTOM_ID_SIZE);
    pub fn vanilla_entity_metadata_salmon_variant_accessor_for_entity_type
        -> int(FIELD_SALMON_DATA_TYPE);
    pub fn vanilla_entity_metadata_parrot_variant_accessor_for_entity_type
        -> int(FIELD_PARROT_DATA_VARIANT_ID);
    pub fn vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type
        -> int(FIELD_AXOLOTL_DATA_VARIANT);
    pub fn vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type
        -> int(FIELD_MUSHROOM_COW_DATA_TYPE);
    pub fn vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type
        -> bool(FIELD_BOGGED_DATA_SHEARED);
    pub fn vanilla_entity_metadata_allay_dancing_accessor_for_entity_type
        -> bool(FIELD_ALLAY_DATA_DANCING);
    pub fn vanilla_entity_metadata_allay_can_duplicate_accessor_for_entity_type
        -> bool(FIELD_ALLAY_DATA_CAN_DUPLICATE);
    pub fn vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type
        -> bool(FIELD_AXOLOTL_DATA_PLAYING_DEAD);
    pub fn vanilla_entity_metadata_camel_dash_accessor_for_entity_type
        -> bool(FIELD_CAMEL_DASH);
    pub fn vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type
        -> long(FIELD_CAMEL_LAST_POSE_CHANGE_TICK);
    pub fn vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type
        -> bool(FIELD_CREAKING_CAN_MOVE);
    pub fn vanilla_entity_metadata_creaking_active_accessor_for_entity_type
        -> bool(FIELD_CREAKING_IS_ACTIVE);
    pub fn vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type
        -> bool(FIELD_CREAKING_IS_TEARING_DOWN);
    pub fn vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type
        -> id(FIELD_CREAKING_HOME_POS);
    pub fn vanilla_entity_metadata_sniffer_state_accessor_for_entity_type
        -> int(FIELD_SNIFFER_DATA_STATE);
    pub fn vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type
        -> float(FIELD_AREA_EFFECT_CLOUD_DATA_RADIUS);
    pub fn vanilla_entity_metadata_area_effect_cloud_waiting_accessor_for_entity_type
        -> bool(FIELD_AREA_EFFECT_CLOUD_DATA_WAITING);
    pub fn vanilla_entity_metadata_area_effect_cloud_particle_accessor_for_entity_type
        -> id(FIELD_AREA_EFFECT_CLOUD_DATA_PARTICLE);
    pub fn vanilla_entity_metadata_living_entity_effect_particles_accessor_for_entity_type
        -> id(FIELD_LIVING_ENTITY_DATA_EFFECT_PARTICLES);
    pub fn vanilla_entity_metadata_living_entity_effect_ambience_accessor_for_entity_type
        -> bool(FIELD_LIVING_ENTITY_DATA_EFFECT_AMBIENCE_ID);
    pub fn vanilla_entity_metadata_blaze_flags_accessor_for_entity_type
        -> byte(FIELD_BLAZE_DATA_FLAGS_ID);
    pub fn vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type
        -> id(FIELD_OMINOUS_ITEM_SPAWNER_DATA_ITEM);
    pub fn vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type
        -> id(FIELD_ITEM_ENTITY_DATA_ITEM);
    pub fn vanilla_entity_metadata_armadillo_state_accessor_for_entity_type
        -> int(FIELD_ARMADILLO_ARMADILLO_STATE);
    pub fn vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type
        -> int(FIELD_WARDEN_CLIENT_ANGER_LEVEL);
    pub fn vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type
        -> int(FIELD_ENDER_DRAGON_DATA_PHASE);
    pub fn vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type
        -> bool(FIELD_PILLAGER_IS_CHARGING_CROSSBOW);
    pub fn vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type
        -> bool(FIELD_PIGLIN_DATA_IS_CHARGING_CROSSBOW);
    pub fn vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type
        -> bool(FIELD_POLAR_BEAR_DATA_STANDING_ID);
    pub fn vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type
        -> bool(FIELD_MINECART_FURNACE_DATA_ID_FUEL);
    pub fn vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type
        -> int(FIELD_EXPERIENCE_ORB_DATA_VALUE);
    pub fn vanilla_entity_metadata_horse_variant_accessor_for_entity_type
        -> int(FIELD_HORSE_DATA_ID_TYPE_VARIANT);
    pub fn vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type
        -> bool(FIELD_ABSTRACT_CHESTED_HORSE_DATA_ID_CHEST);
    pub fn vanilla_entity_metadata_frog_variant_accessor_for_entity_type
        -> resource(FIELD_FROG_DATA_VARIANT_ID);
    pub fn vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type
        -> resource(FIELD_ZOMBIE_NAUTILUS_DATA_VARIANT_ID);
    pub fn vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type
        -> bool(FIELD_RAIDER_IS_CELEBRATING);
    pub fn vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type
        -> byte(FIELD_SPELLCASTER_ILLAGER_DATA_SPELL_CASTING_ID);
    pub fn vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type
        -> int(FIELD_ABSTRACT_VILLAGER_DATA_UNHAPPY_COUNTER);
    pub fn vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type
        -> int(FIELD_WITHER_BOSS_DATA_ID_INV);
    pub fn vanilla_entity_metadata_baby_accessor_for_entity_type
        -> bool(FIELD_AGEABLE_MOB_DATA_BABY_ID
            | FIELD_ZOMBIE_DATA_BABY_ID
            | FIELD_ZOGLIN_DATA_BABY_ID
            | FIELD_PIGLIN_DATA_BABY_ID);
    pub fn vanilla_entity_metadata_age_locked_accessor_for_entity_type
        -> bool(FIELD_AGEABLE_MOB_AGE_LOCKED | FIELD_TADPOLE_AGE_LOCKED);
    pub fn vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type
        -> bool(FIELD_PIGLIN_DATA_IS_DANCING);
}

/// Resolves the shared Entity accessors, typed defaults, and flag masks.
pub fn vanilla_entity_metadata_entity_accessors() -> Option<VanillaEntityMetadataAccessors> {
    const VALUE: VanillaEntityMetadataAccessors = VanillaEntityMetadataAccessors {
        shared_flags_data_id: FIELD_ENTITY_DATA_SHARED_FLAGS_ID.data_id,
        shared_flags_default: FIELD_ENTITY_DATA_SHARED_FLAGS_ID.byte_default(),
        on_fire_mask: VANILLA_ENTITY_SHARED_FLAG_ON_FIRE,
        shift_key_down_mask: VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
        sprinting_mask: VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
        swimming_mask: VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
        invisible_mask: VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
        glowing_mask: VANILLA_ENTITY_SHARED_FLAG_GLOWING,
        fall_flying_mask: VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING,
        air_supply_data_id: FIELD_ENTITY_DATA_AIR_SUPPLY_ID.data_id,
        custom_name_data_id: FIELD_ENTITY_DATA_CUSTOM_NAME.data_id,
        custom_name_visible_data_id: FIELD_ENTITY_DATA_CUSTOM_NAME_VISIBLE.data_id,
        custom_name_visible_default: FIELD_ENTITY_DATA_CUSTOM_NAME_VISIBLE.bool_default(),
        silent_data_id: FIELD_ENTITY_DATA_SILENT.data_id,
        silent_default: FIELD_ENTITY_DATA_SILENT.bool_default(),
        no_gravity_data_id: FIELD_ENTITY_DATA_NO_GRAVITY.data_id,
        no_gravity_default: FIELD_ENTITY_DATA_NO_GRAVITY.bool_default(),
        pose_data_id: FIELD_ENTITY_DATA_POSE.data_id,
        pose_default: FIELD_ENTITY_DATA_POSE.pose_default(),
        ticks_frozen_data_id: FIELD_ENTITY_DATA_TICKS_FROZEN.data_id,
        ticks_frozen_default: FIELD_ENTITY_DATA_TICKS_FROZEN.int_default(),
    };
    Some(VALUE)
}

pub fn vanilla_entity_metadata_living_entity_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaLivingEntityMetadataAccessors> {
    const VALUE: VanillaLivingEntityMetadataAccessors = VanillaLivingEntityMetadataAccessors {
        flags_data_id: FIELD_LIVING_ENTITY_DATA_LIVING_ENTITY_FLAGS.data_id,
        flags_default: FIELD_LIVING_ENTITY_DATA_LIVING_ENTITY_FLAGS.byte_default(),
        is_using_mask: VANILLA_LIVING_ENTITY_FLAG_IS_USING,
        off_hand_mask: VANILLA_LIVING_ENTITY_FLAG_OFF_HAND,
        spin_attack_mask: VANILLA_LIVING_ENTITY_FLAG_SPIN_ATTACK,
        health_data_id: FIELD_LIVING_ENTITY_DATA_HEALTH_ID.data_id,
        health_default: FIELD_LIVING_ENTITY_DATA_HEALTH_ID.float_default(),
        arrow_count_data_id: FIELD_LIVING_ENTITY_DATA_ARROW_COUNT_ID.data_id,
        arrow_count_default: FIELD_LIVING_ENTITY_DATA_ARROW_COUNT_ID.int_default(),
        stinger_count_data_id: FIELD_LIVING_ENTITY_DATA_STINGER_COUNT_ID.data_id,
        stinger_count_default: FIELD_LIVING_ENTITY_DATA_STINGER_COUNT_ID.int_default(),
        sleeping_pos_data_id: FIELD_LIVING_ENTITY_SLEEPING_POS_ID.data_id,
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::LivingEntity,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_mob_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaMobMetadataAccessors> {
    const VALUE: VanillaMobMetadataAccessors = VanillaMobMetadataAccessors {
        flags_data_id: FIELD_MOB_DATA_MOB_FLAGS_ID.data_id,
        flags_default: FIELD_MOB_DATA_MOB_FLAGS_ID.byte_default(),
        no_ai_mask: VANILLA_MOB_FLAG_NO_AI,
        left_handed_mask: VANILLA_MOB_FLAG_LEFT_HANDED,
        aggressive_mask: VANILLA_MOB_FLAG_AGGRESSIVE,
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Mob, VALUE)
}

pub fn vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaEndermanRenderMetadataAccessors> {
    const VALUE: VanillaEndermanRenderMetadataAccessors = VanillaEndermanRenderMetadataAccessors {
        carried_block_state_data_id: FIELD_ENDER_MAN_DATA_CARRY_STATE.data_id,
        carried_block_state_default: None,
        creepy_data_id: FIELD_ENDER_MAN_DATA_CREEPY.data_id,
        creepy_default: FIELD_ENDER_MAN_DATA_CREEPY.bool_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::EnderMan, VALUE)
}

pub fn vanilla_entity_metadata_end_crystal_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaEndCrystalMetadataAccessors> {
    const VALUE: VanillaEndCrystalMetadataAccessors = VanillaEndCrystalMetadataAccessors {
        beam_target_data_id: FIELD_END_CRYSTAL_DATA_BEAM_TARGET.data_id,
        show_bottom_data_id: FIELD_END_CRYSTAL_DATA_SHOW_BOTTOM.data_id,
        show_bottom_default: FIELD_END_CRYSTAL_DATA_SHOW_BOTTOM.bool_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::EndCrystal,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaHappyGhastMetadataAccessors> {
    const VALUE: VanillaHappyGhastMetadataAccessors = VanillaHappyGhastMetadataAccessors {
        is_leash_holder_data_id: FIELD_HAPPY_GHAST_IS_LEASH_HOLDER.data_id,
        is_leash_holder_default: FIELD_HAPPY_GHAST_IS_LEASH_HOLDER.bool_default(),
        stays_still_data_id: FIELD_HAPPY_GHAST_STAYS_STILL.data_id,
        stays_still_default: FIELD_HAPPY_GHAST_STAYS_STILL.bool_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::HappyGhast,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_goat_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaGoatMetadataAccessors> {
    const VALUE: VanillaGoatMetadataAccessors = VanillaGoatMetadataAccessors {
        screaming_data_id: FIELD_GOAT_DATA_IS_SCREAMING_GOAT.data_id,
        screaming_default: FIELD_GOAT_DATA_IS_SCREAMING_GOAT.bool_default(),
        left_horn_data_id: FIELD_GOAT_DATA_HAS_LEFT_HORN.data_id,
        left_horn_default: FIELD_GOAT_DATA_HAS_LEFT_HORN.bool_default(),
        right_horn_data_id: FIELD_GOAT_DATA_HAS_RIGHT_HORN.data_id,
        right_horn_default: FIELD_GOAT_DATA_HAS_RIGHT_HORN.bool_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Goat, VALUE)
}

pub fn vanilla_entity_metadata_turtle_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaTurtleMetadataAccessors> {
    const VALUE: VanillaTurtleMetadataAccessors = VanillaTurtleMetadataAccessors {
        has_egg_data_id: FIELD_TURTLE_HAS_EGG.data_id,
        has_egg_default: FIELD_TURTLE_HAS_EGG.bool_default(),
        laying_egg_data_id: FIELD_TURTLE_LAYING_EGG.data_id,
        laying_egg_default: FIELD_TURTLE_LAYING_EGG.bool_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Turtle, VALUE)
}

pub fn vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaHangingEntityMetadataAccessor> {
    const VALUE: VanillaHangingEntityMetadataAccessor = VanillaHangingEntityMetadataAccessor {
        direction_data_id: FIELD_HANGING_ENTITY_DATA_DIRECTION.data_id,
        direction_default: FIELD_HANGING_ENTITY_DATA_DIRECTION.direction_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::HangingEntity,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_item_frame_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaItemFrameMetadataAccessors> {
    const VALUE: VanillaItemFrameMetadataAccessors = VanillaItemFrameMetadataAccessors {
        direction_data_id: FIELD_HANGING_ENTITY_DATA_DIRECTION.data_id,
        direction_default: FIELD_HANGING_ENTITY_DATA_DIRECTION.direction_default(),
        item_data_id: FIELD_ITEM_FRAME_DATA_ITEM.data_id,
        rotation_data_id: FIELD_ITEM_FRAME_DATA_ROTATION.data_id,
        rotation_default: FIELD_ITEM_FRAME_DATA_ROTATION.int_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::ItemFrame,
        VALUE,
    )
}

/// Returns the Bat flags `(data_id, default, resting mask)`.
pub fn vanilla_entity_metadata_bat_flags_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8, i8)> {
    const VALUE: (u8, i8, i8) = (
        FIELD_BAT_DATA_ID_FLAGS.data_id,
        FIELD_BAT_DATA_ID_FLAGS.byte_default(),
        VANILLA_BAT_FLAG_RESTING,
    );
    for_family(entity_type_id, VanillaEntityMetadataFamily::Bat, VALUE)
}

/// Returns the Vex flags `(data_id, default, charging mask)`.
pub fn vanilla_entity_metadata_vex_flags_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8, i8)> {
    const VALUE: (u8, i8, i8) = (
        FIELD_VEX_DATA_FLAGS_ID.data_id,
        FIELD_VEX_DATA_FLAGS_ID.byte_default(),
        VANILLA_VEX_FLAG_CHARGING,
    );
    for_family(entity_type_id, VanillaEntityMetadataFamily::Vex, VALUE)
}

/// Returns the SnowGolem pumpkin `(data_id, default, pumpkin mask)`.
pub fn vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8, i8)> {
    const VALUE: (u8, i8, i8) = (
        FIELD_SNOW_GOLEM_DATA_PUMPKIN_ID.data_id,
        FIELD_SNOW_GOLEM_DATA_PUMPKIN_ID.byte_default(),
        VANILLA_SNOW_GOLEM_PUMPKIN_FLAG,
    );
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::SnowGolem,
        VALUE,
    )
}

/// Returns the Sheep wool `(data_id, default, sheared mask, color mask)`.
pub fn vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, i8, i8, u8)> {
    const VALUE: (u8, i8, i8, u8) = (
        FIELD_SHEEP_DATA_WOOL_ID.data_id,
        FIELD_SHEEP_DATA_WOOL_ID.byte_default(),
        VANILLA_SHEEP_FLAG_SHEARED,
        VANILLA_SHEEP_WOOL_COLOR_MASK,
    );
    for_family(entity_type_id, VanillaEntityMetadataFamily::Sheep, VALUE)
}

pub fn vanilla_entity_metadata_bee_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaBeeMetadataAccessors> {
    const VALUE: VanillaBeeMetadataAccessors = VanillaBeeMetadataAccessors {
        flags_data_id: FIELD_BEE_DATA_FLAGS_ID.data_id,
        flags_default: FIELD_BEE_DATA_FLAGS_ID.byte_default(),
        roll_mask: VANILLA_BEE_FLAG_ROLL,
        has_stung_mask: VANILLA_BEE_FLAG_HAS_STUNG,
        has_nectar_mask: VANILLA_BEE_FLAG_HAS_NECTAR,
        anger_end_time_data_id: FIELD_BEE_DATA_ANGER_END_TIME.data_id,
        anger_end_time_default: FIELD_BEE_DATA_ANGER_END_TIME.long_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Bee, VALUE)
}

pub fn vanilla_entity_metadata_dolphin_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaDolphinMetadataAccessors> {
    const VALUE: VanillaDolphinMetadataAccessors = VanillaDolphinMetadataAccessors {
        got_fish_data_id: FIELD_DOLPHIN_GOT_FISH.data_id,
        got_fish_default: FIELD_DOLPHIN_GOT_FISH.bool_default(),
        moistness_data_id: FIELD_DOLPHIN_MOISTNESS_LEVEL.data_id,
        moistness_default: FIELD_DOLPHIN_MOISTNESS_LEVEL.int_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Dolphin, VALUE)
}

pub fn vanilla_entity_metadata_fox_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaFoxMetadataAccessors> {
    const VALUE: VanillaFoxMetadataAccessors = VanillaFoxMetadataAccessors {
        type_data_id: FIELD_FOX_DATA_TYPE_ID.data_id,
        type_default: FIELD_FOX_DATA_TYPE_ID.int_default(),
        flags_data_id: FIELD_FOX_DATA_FLAGS_ID.data_id,
        flags_default: FIELD_FOX_DATA_FLAGS_ID.byte_default(),
        sitting_mask: VANILLA_FOX_FLAG_SITTING,
        crouching_mask: VANILLA_FOX_FLAG_CROUCHING,
        interested_mask: VANILLA_FOX_FLAG_INTERESTED,
        pouncing_mask: VANILLA_FOX_FLAG_POUNCING,
        sleeping_mask: VANILLA_FOX_FLAG_SLEEPING,
        faceplanted_mask: VANILLA_FOX_FLAG_FACEPLANTED,
        defending_mask: VANILLA_FOX_FLAG_DEFENDING,
        trusted_data_ids: [
            FIELD_FOX_DATA_TRUSTED_ID_0.data_id,
            FIELD_FOX_DATA_TRUSTED_ID_1.data_id,
        ],
        trusted_default: None,
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Fox, VALUE)
}

pub fn vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaTropicalFishVariantAccessors> {
    const VALUE: VanillaTropicalFishVariantAccessors = VanillaTropicalFishVariantAccessors {
        variant_data_id: FIELD_TROPICAL_FISH_DATA_ID_TYPE_VARIANT.data_id,
        variant_default: FIELD_TROPICAL_FISH_DATA_ID_TYPE_VARIANT.int_default(),
        pattern_mask: VANILLA_TROPICAL_FISH_PATTERN_MASK,
        base_color_mask: VANILLA_TROPICAL_FISH_BASE_COLOR_MASK,
        base_color_shift: VANILLA_TROPICAL_FISH_BASE_COLOR_SHIFT,
        pattern_color_mask: VANILLA_TROPICAL_FISH_PATTERN_COLOR_MASK,
        pattern_color_shift: VANILLA_TROPICAL_FISH_PATTERN_COLOR_SHIFT,
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::TropicalFish,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_cat_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaCatMetadataAccessors> {
    const VALUE: VanillaCatMetadataAccessors = VanillaCatMetadataAccessors {
        lying_data_id: FIELD_CAT_IS_LYING.data_id,
        lying_default: FIELD_CAT_IS_LYING.bool_default(),
        relax_state_one_data_id: FIELD_CAT_RELAX_STATE_ONE.data_id,
        relax_state_one_default: FIELD_CAT_RELAX_STATE_ONE.bool_default(),
        collar_color_data_id: FIELD_CAT_DATA_COLLAR_COLOR.data_id,
        collar_color_default: FIELD_CAT_DATA_COLLAR_COLOR.int_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Cat, VALUE)
}

pub fn vanilla_entity_metadata_cat_lying_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    vanilla_entity_metadata_cat_accessors_for_entity_type(entity_type_id)
        .map(|accessors| (accessors.lying_data_id, accessors.lying_default))
}

pub fn vanilla_entity_metadata_cat_relax_state_one_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    vanilla_entity_metadata_cat_accessors_for_entity_type(entity_type_id).map(|accessors| {
        (
            accessors.relax_state_one_data_id,
            accessors.relax_state_one_default,
        )
    })
}

pub fn vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaCopperGolemMetadataAccessors> {
    const VALUE: VanillaCopperGolemMetadataAccessors = VanillaCopperGolemMetadataAccessors {
        weather_state_data_id: FIELD_COPPER_GOLEM_DATA_WEATHER_STATE.data_id,
        weather_state_default: FIELD_COPPER_GOLEM_DATA_WEATHER_STATE.int_default(),
        state_data_id: FIELD_COPPER_GOLEM_COPPER_GOLEM_STATE.data_id,
        state_default: FIELD_COPPER_GOLEM_COPPER_GOLEM_STATE.int_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::CopperGolem,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_wolf_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaWolfMetadataAccessors> {
    const VALUE: VanillaWolfMetadataAccessors = VanillaWolfMetadataAccessors {
        interested_data_id: FIELD_WOLF_DATA_INTERESTED_ID.data_id,
        interested_default: FIELD_WOLF_DATA_INTERESTED_ID.bool_default(),
        collar_color_data_id: FIELD_WOLF_DATA_COLLAR_COLOR.data_id,
        collar_color_default: FIELD_WOLF_DATA_COLLAR_COLOR.int_default(),
        anger_end_time_data_id: FIELD_WOLF_DATA_ANGER_END_TIME.data_id,
        anger_end_time_default: FIELD_WOLF_DATA_ANGER_END_TIME.long_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Wolf, VALUE)
}

pub fn vanilla_entity_metadata_wolf_interested_accessor_for_entity_type(
    entity_type_id: i32,
) -> Option<(u8, bool)> {
    vanilla_entity_metadata_wolf_accessors_for_entity_type(entity_type_id)
        .map(|accessors| (accessors.interested_data_id, accessors.interested_default))
}

pub fn vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaTamableAnimalMetadataAccessors> {
    const VALUE: VanillaTamableAnimalMetadataAccessors = VanillaTamableAnimalMetadataAccessors {
        flags_data_id: FIELD_TAMABLE_ANIMAL_DATA_FLAGS_ID.data_id,
        flags_default: FIELD_TAMABLE_ANIMAL_DATA_FLAGS_ID.byte_default(),
        sitting_mask: VANILLA_TAMABLE_ANIMAL_FLAG_SITTING,
        tame_mask: VANILLA_TAMABLE_ANIMAL_FLAG_TAME,
        owner_data_id: FIELD_TAMABLE_ANIMAL_DATA_OWNERUUID_ID.data_id,
        owner_default: None,
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::TamableAnimal,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaAbstractBoatMetadataAccessors> {
    const VALUE: VanillaAbstractBoatMetadataAccessors = VanillaAbstractBoatMetadataAccessors {
        paddle_left_data_id: FIELD_ABSTRACT_BOAT_DATA_ID_PADDLE_LEFT.data_id,
        paddle_left_default: FIELD_ABSTRACT_BOAT_DATA_ID_PADDLE_LEFT.bool_default(),
        paddle_right_data_id: FIELD_ABSTRACT_BOAT_DATA_ID_PADDLE_RIGHT.data_id,
        paddle_right_default: FIELD_ABSTRACT_BOAT_DATA_ID_PADDLE_RIGHT.bool_default(),
        bubble_time_data_id: FIELD_ABSTRACT_BOAT_DATA_ID_BUBBLE_TIME.data_id,
        bubble_time_default: FIELD_ABSTRACT_BOAT_DATA_ID_BUBBLE_TIME.int_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::AbstractBoat,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_vehicle_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaVehicleMetadataAccessors> {
    const VALUE: VanillaVehicleMetadataAccessors = VanillaVehicleMetadataAccessors {
        hurt_time_data_id: FIELD_VEHICLE_ENTITY_DATA_ID_HURT.data_id,
        hurt_time_default: FIELD_VEHICLE_ENTITY_DATA_ID_HURT.int_default(),
        hurt_dir_data_id: FIELD_VEHICLE_ENTITY_DATA_ID_HURTDIR.data_id,
        hurt_dir_default: FIELD_VEHICLE_ENTITY_DATA_ID_HURTDIR.int_default(),
        damage_data_id: FIELD_VEHICLE_ENTITY_DATA_ID_DAMAGE.data_id,
        damage_default: FIELD_VEHICLE_ENTITY_DATA_ID_DAMAGE.float_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::VehicleEntity,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaPrimedTntMetadataAccessors> {
    const VALUE: VanillaPrimedTntMetadataAccessors = VanillaPrimedTntMetadataAccessors {
        fuse_data_id: FIELD_PRIMED_TNT_DATA_FUSE_ID.data_id,
        fuse_default: FIELD_PRIMED_TNT_DATA_FUSE_ID.int_default(),
        block_state_data_id: FIELD_PRIMED_TNT_DATA_BLOCK_STATE_ID.data_id,
        block_state_default: FIELD_PRIMED_TNT_DATA_BLOCK_STATE_ID.typed_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::PrimedTnt,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaFireworkRocketMetadataAccessors> {
    const VALUE: VanillaFireworkRocketMetadataAccessors = VanillaFireworkRocketMetadataAccessors {
        item_data_id: FIELD_FIREWORK_ROCKET_DATA_ID_FIREWORKS_ITEM.data_id,
        item_default_resource_id: FIELD_FIREWORK_ROCKET_DATA_ID_FIREWORKS_ITEM.resource_default(),
        attached_to_target_data_id: FIELD_FIREWORK_ROCKET_DATA_ATTACHED_TO_TARGET.data_id,
        attached_to_target_default: None,
        shot_at_angle_data_id: FIELD_FIREWORK_ROCKET_DATA_SHOT_AT_ANGLE.data_id,
        shot_at_angle_default: FIELD_FIREWORK_ROCKET_DATA_SHOT_AT_ANGLE.bool_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::FireworkRocket,
        VALUE,
    )
}

pub(crate) fn vanilla_entity_metadata_guardian_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaGuardianMetadataAccessors> {
    const VALUE: VanillaGuardianMetadataAccessors = VanillaGuardianMetadataAccessors {
        moving_data_id: FIELD_GUARDIAN_DATA_ID_MOVING.data_id,
        moving_default: FIELD_GUARDIAN_DATA_ID_MOVING.bool_default(),
        attack_target_data_id: FIELD_GUARDIAN_DATA_ID_ATTACK_TARGET.data_id,
        attack_target_default: FIELD_GUARDIAN_DATA_ID_ATTACK_TARGET.int_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Guardian, VALUE)
}

pub fn vanilla_entity_metadata_shulker_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaShulkerMetadataAccessors> {
    const VALUE: VanillaShulkerMetadataAccessors = VanillaShulkerMetadataAccessors {
        attach_face_data_id: FIELD_SHULKER_DATA_ATTACH_FACE_ID.data_id,
        attach_face_default: FIELD_SHULKER_DATA_ATTACH_FACE_ID.direction_default(),
        peek_data_id: FIELD_SHULKER_DATA_PEEK_ID.data_id,
        peek_default: FIELD_SHULKER_DATA_PEEK_ID.byte_default(),
        color_data_id: FIELD_SHULKER_DATA_COLOR_ID.data_id,
        color_default: FIELD_SHULKER_DATA_COLOR_ID.byte_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Shulker, VALUE)
}

pub fn vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaAbstractArrowMetadataAccessors> {
    const VALUE: VanillaAbstractArrowMetadataAccessors = VanillaAbstractArrowMetadataAccessors {
        flags_data_id: FIELD_ABSTRACT_ARROW_ID_FLAGS.data_id,
        flags_default: FIELD_ABSTRACT_ARROW_ID_FLAGS.byte_default(),
        crit_mask: VANILLA_ABSTRACT_ARROW_FLAG_CRIT,
        no_physics_mask: VANILLA_ABSTRACT_ARROW_FLAG_NO_PHYSICS,
        pierce_level_data_id: FIELD_ABSTRACT_ARROW_PIERCE_LEVEL.data_id,
        pierce_level_default: FIELD_ABSTRACT_ARROW_PIERCE_LEVEL.byte_default(),
        in_ground_data_id: FIELD_ABSTRACT_ARROW_IN_GROUND.data_id,
        in_ground_default: FIELD_ABSTRACT_ARROW_IN_GROUND.bool_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::AbstractArrow,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaAbstractMinecartMetadataAccessors> {
    vanilla_entity_type_extends(
        entity_type_id,
        VanillaEntityMetadataFamily::AbstractMinecart,
    )
    .then_some(())?;
    Some(VanillaAbstractMinecartMetadataAccessors {
        custom_display_block_data_id: FIELD_ABSTRACT_MINECART_DATA_ID_CUSTOM_DISPLAY_BLOCK.data_id,
        display_offset_data_id: FIELD_ABSTRACT_MINECART_DATA_ID_DISPLAY_OFFSET.data_id,
        display_offset_default: vanilla_minecart_default_display_offset(entity_type_id)?,
    })
}

pub fn vanilla_entity_metadata_arrow_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaArrowMetadataAccessors> {
    const VALUE: VanillaArrowMetadataAccessors = VanillaArrowMetadataAccessors {
        effect_color_data_id: FIELD_ARROW_ID_EFFECT_COLOR.data_id,
        effect_color_default: FIELD_ARROW_ID_EFFECT_COLOR.int_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Arrow, VALUE)
}

pub fn vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaThrownTridentMetadataAccessors> {
    const VALUE: VanillaThrownTridentMetadataAccessors = VanillaThrownTridentMetadataAccessors {
        loyalty_data_id: FIELD_THROWN_TRIDENT_ID_LOYALTY.data_id,
        loyalty_default: FIELD_THROWN_TRIDENT_ID_LOYALTY.byte_default(),
        foil_data_id: FIELD_THROWN_TRIDENT_ID_FOIL.data_id,
        foil_default: FIELD_THROWN_TRIDENT_ID_FOIL.bool_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::ThrownTrident,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaArmorStandMetadataAccessors> {
    const VALUE: VanillaArmorStandMetadataAccessors = VanillaArmorStandMetadataAccessors {
        client_flags_data_id: FIELD_ARMOR_STAND_DATA_CLIENT_FLAGS.data_id,
        client_flags_default: FIELD_ARMOR_STAND_DATA_CLIENT_FLAGS.byte_default(),
        small_mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL,
        show_arms_mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS,
        no_baseplate_mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE,
        marker_mask: VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER,
        head_pose_data_id: FIELD_ARMOR_STAND_DATA_HEAD_POSE.data_id,
        head_pose_default: FIELD_ARMOR_STAND_DATA_HEAD_POSE.rotations_default(),
        body_pose_data_id: FIELD_ARMOR_STAND_DATA_BODY_POSE.data_id,
        body_pose_default: FIELD_ARMOR_STAND_DATA_BODY_POSE.rotations_default(),
        left_arm_pose_data_id: FIELD_ARMOR_STAND_DATA_LEFT_ARM_POSE.data_id,
        left_arm_pose_default: FIELD_ARMOR_STAND_DATA_LEFT_ARM_POSE.rotations_default(),
        right_arm_pose_data_id: FIELD_ARMOR_STAND_DATA_RIGHT_ARM_POSE.data_id,
        right_arm_pose_default: FIELD_ARMOR_STAND_DATA_RIGHT_ARM_POSE.rotations_default(),
        left_leg_pose_data_id: FIELD_ARMOR_STAND_DATA_LEFT_LEG_POSE.data_id,
        left_leg_pose_default: FIELD_ARMOR_STAND_DATA_LEFT_LEG_POSE.rotations_default(),
        right_leg_pose_data_id: FIELD_ARMOR_STAND_DATA_RIGHT_LEG_POSE.data_id,
        right_leg_pose_default: FIELD_ARMOR_STAND_DATA_RIGHT_LEG_POSE.rotations_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::ArmorStand,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_interaction_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaInteractionMetadataAccessors> {
    const VALUE: VanillaInteractionMetadataAccessors = VanillaInteractionMetadataAccessors {
        width_data_id: FIELD_INTERACTION_DATA_WIDTH_ID.data_id,
        width_default: FIELD_INTERACTION_DATA_WIDTH_ID.float_default(),
        height_data_id: FIELD_INTERACTION_DATA_HEIGHT_ID.data_id,
        height_default: FIELD_INTERACTION_DATA_HEIGHT_ID.float_default(),
        response_data_id: FIELD_INTERACTION_DATA_RESPONSE_ID.data_id,
        response_default: FIELD_INTERACTION_DATA_RESPONSE_ID.bool_default(),
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::Interaction,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_creeper_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaCreeperMetadataAccessors> {
    const VALUE: VanillaCreeperMetadataAccessors = VanillaCreeperMetadataAccessors {
        swell_dir_data_id: FIELD_CREEPER_DATA_SWELL_DIR.data_id,
        swell_dir_default: FIELD_CREEPER_DATA_SWELL_DIR.int_default(),
        powered_data_id: FIELD_CREEPER_DATA_IS_POWERED.data_id,
        powered_default: FIELD_CREEPER_DATA_IS_POWERED.bool_default(),
        ignited_data_id: FIELD_CREEPER_DATA_IS_IGNITED.data_id,
        ignited_default: FIELD_CREEPER_DATA_IS_IGNITED.bool_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Creeper, VALUE)
}

pub fn vanilla_entity_metadata_panda_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaPandaMetadataAccessors> {
    const VALUE: VanillaPandaMetadataAccessors = VanillaPandaMetadataAccessors {
        unhappy_counter_data_id: FIELD_PANDA_UNHAPPY_COUNTER.data_id,
        unhappy_counter_default: FIELD_PANDA_UNHAPPY_COUNTER.int_default(),
        sneeze_counter_data_id: FIELD_PANDA_SNEEZE_COUNTER.data_id,
        sneeze_counter_default: FIELD_PANDA_SNEEZE_COUNTER.int_default(),
        eat_counter_data_id: FIELD_PANDA_EAT_COUNTER.data_id,
        eat_counter_default: FIELD_PANDA_EAT_COUNTER.int_default(),
        main_gene_data_id: FIELD_PANDA_MAIN_GENE_ID.data_id,
        main_gene_default: FIELD_PANDA_MAIN_GENE_ID.byte_default(),
        hidden_gene_data_id: FIELD_PANDA_HIDDEN_GENE_ID.data_id,
        hidden_gene_default: FIELD_PANDA_HIDDEN_GENE_ID.byte_default(),
        flags_data_id: FIELD_PANDA_DATA_ID_FLAGS.data_id,
        flags_default: FIELD_PANDA_DATA_ID_FLAGS.byte_default(),
        sneezing_mask: VANILLA_PANDA_FLAG_SNEEZING,
        rolling_mask: VANILLA_PANDA_FLAG_ROLLING,
        sitting_mask: VANILLA_PANDA_FLAG_SITTING,
        on_back_mask: VANILLA_PANDA_FLAG_ON_BACK,
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Panda, VALUE)
}

pub fn vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaAbstractHorseMetadataAccessors> {
    const VALUE: VanillaAbstractHorseMetadataAccessors = VanillaAbstractHorseMetadataAccessors {
        flags_data_id: FIELD_ABSTRACT_HORSE_DATA_ID_FLAGS.data_id,
        flags_default: FIELD_ABSTRACT_HORSE_DATA_ID_FLAGS.byte_default(),
        tame_mask: VANILLA_ABSTRACT_HORSE_FLAG_TAME,
        bred_mask: VANILLA_ABSTRACT_HORSE_FLAG_BRED,
        eating_mask: VANILLA_ABSTRACT_HORSE_FLAG_EATING,
        standing_mask: VANILLA_ABSTRACT_HORSE_FLAG_STANDING,
        open_mouth_mask: VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH,
    };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::AbstractHorse,
        VALUE,
    )
}

pub fn vanilla_entity_metadata_llama_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaLlamaMetadataAccessors> {
    const VALUE: VanillaLlamaMetadataAccessors = VanillaLlamaMetadataAccessors {
        strength_data_id: FIELD_LLAMA_DATA_STRENGTH_ID.data_id,
        strength_default: FIELD_LLAMA_DATA_STRENGTH_ID.int_default(),
        variant_data_id: FIELD_LLAMA_DATA_VARIANT_ID.data_id,
        variant_default: FIELD_LLAMA_DATA_VARIANT_ID.int_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Llama, VALUE)
}

type VariantMetadataPair = (u8, &'static str, u8, &'static str);

const fn variant_pair(
    variant: VanillaEntityMetadataFieldSchema,
    sound_variant: VanillaEntityMetadataFieldSchema,
) -> VariantMetadataPair {
    (
        variant.data_id,
        variant.resource_default(),
        sound_variant.data_id,
        sound_variant.resource_default(),
    )
}

pub fn vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaFarmAnimalVariantMetadataAccessors> {
    const FAMILIES: [(VanillaEntityMetadataFamily, VariantMetadataPair); 3] = [
        (
            VanillaEntityMetadataFamily::Chicken,
            variant_pair(
                FIELD_CHICKEN_DATA_VARIANT_ID,
                FIELD_CHICKEN_DATA_SOUND_VARIANT_ID,
            ),
        ),
        (
            VanillaEntityMetadataFamily::Cow,
            variant_pair(FIELD_COW_DATA_VARIANT_ID, FIELD_COW_DATA_SOUND_VARIANT_ID),
        ),
        (
            VanillaEntityMetadataFamily::Pig,
            variant_pair(FIELD_PIG_DATA_VARIANT_ID, FIELD_PIG_DATA_SOUND_VARIANT_ID),
        ),
    ];
    let (_, (variant, variant_default, sound, sound_default)) = FAMILIES
        .into_iter()
        .find(|(family, _)| vanilla_entity_type_extends(entity_type_id, *family))?;
    Some(VanillaFarmAnimalVariantMetadataAccessors {
        variant_data_id: variant,
        variant_default_resource_id: variant_default,
        sound_variant_data_id: sound,
        sound_variant_default_resource_id: sound_default,
    })
}

pub fn vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaCompanionVariantMetadataAccessors> {
    const FAMILIES: [(VanillaEntityMetadataFamily, VariantMetadataPair); 2] = [
        (
            VanillaEntityMetadataFamily::Cat,
            variant_pair(FIELD_CAT_DATA_VARIANT_ID, FIELD_CAT_DATA_SOUND_VARIANT_ID),
        ),
        (
            VanillaEntityMetadataFamily::Wolf,
            variant_pair(FIELD_WOLF_DATA_VARIANT_ID, FIELD_WOLF_DATA_SOUND_VARIANT_ID),
        ),
    ];
    let (_, (variant, variant_default, sound, sound_default)) = FAMILIES
        .into_iter()
        .find(|(family, _)| vanilla_entity_type_extends(entity_type_id, *family))?;
    Some(VanillaCompanionVariantMetadataAccessors {
        variant_data_id: variant,
        variant_default_resource_id: variant_default,
        sound_variant_data_id: sound,
        sound_variant_default_resource_id: sound_default,
    })
}

pub fn vanilla_entity_metadata_zombie_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaZombieMetadataAccessors> {
    const VALUE: VanillaZombieMetadataAccessors = VanillaZombieMetadataAccessors {
        baby_data_id: FIELD_ZOMBIE_DATA_BABY_ID.data_id,
        baby_default: FIELD_ZOMBIE_DATA_BABY_ID.bool_default(),
        special_type_data_id: FIELD_ZOMBIE_DATA_SPECIAL_TYPE_ID.data_id,
        special_type_default: FIELD_ZOMBIE_DATA_SPECIAL_TYPE_ID.int_default(),
        drowned_conversion_data_id: FIELD_ZOMBIE_DATA_DROWNED_CONVERSION_ID.data_id,
        drowned_conversion_default: FIELD_ZOMBIE_DATA_DROWNED_CONVERSION_ID.bool_default(),
    };
    for_family(entity_type_id, VanillaEntityMetadataFamily::Zombie, VALUE)
}

pub(crate) fn vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type(
    entity_type_id: i32,
) -> Option<VanillaWitherSideHeadTargetMetadataAccessors> {
    const VALUE: VanillaWitherSideHeadTargetMetadataAccessors =
        VanillaWitherSideHeadTargetMetadataAccessors {
            target_b_data_id: FIELD_WITHER_BOSS_DATA_TARGET_B.data_id,
            target_b_default: FIELD_WITHER_BOSS_DATA_TARGET_B.int_default(),
            target_c_data_id: FIELD_WITHER_BOSS_DATA_TARGET_C.data_id,
            target_c_default: FIELD_WITHER_BOSS_DATA_TARGET_C.int_default(),
        };
    for_family(
        entity_type_id,
        VanillaEntityMetadataFamily::WitherBoss,
        VALUE,
    )
}
