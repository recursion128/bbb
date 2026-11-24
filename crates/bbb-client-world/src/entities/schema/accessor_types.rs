use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEndermanRenderMetadataAccessors {
    pub carried_block_state_data_id: u8,
    pub carried_block_state_default: Option<i32>,
    pub creepy_data_id: u8,
    pub creepy_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEndCrystalMetadataAccessors {
    pub beam_target_data_id: u8,
    pub show_bottom_data_id: u8,
    pub show_bottom_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaGoatMetadataAccessors {
    pub screaming_data_id: u8,
    pub screaming_default: bool,
    pub left_horn_data_id: u8,
    pub left_horn_default: bool,
    pub right_horn_data_id: u8,
    pub right_horn_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaPandaMetadataAccessors {
    pub unhappy_counter_data_id: u8,
    pub unhappy_counter_default: i32,
    pub sneeze_counter_data_id: u8,
    pub sneeze_counter_default: i32,
    pub eat_counter_data_id: u8,
    pub eat_counter_default: i32,
    pub main_gene_data_id: u8,
    pub main_gene_default: i8,
    pub hidden_gene_data_id: u8,
    pub hidden_gene_default: i8,
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub sneezing_mask: i8,
    pub rolling_mask: i8,
    pub sitting_mask: i8,
    pub on_back_mask: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaAbstractHorseMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub tame_mask: i8,
    pub bred_mask: i8,
    pub eating_mask: i8,
    pub standing_mask: i8,
    pub open_mouth_mask: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaTamableAnimalMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub sitting_mask: i8,
    pub tame_mask: i8,
    pub owner_data_id: u8,
    pub owner_default: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaLlamaMetadataAccessors {
    pub strength_data_id: u8,
    pub strength_default: i32,
    pub variant_data_id: u8,
    pub variant_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaFarmAnimalVariantMetadataAccessors {
    pub variant_data_id: u8,
    pub variant_default_resource_id: &'static str,
    pub sound_variant_data_id: u8,
    pub sound_variant_default_resource_id: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaCompanionVariantMetadataAccessors {
    pub variant_data_id: u8,
    pub variant_default_resource_id: &'static str,
    pub sound_variant_data_id: u8,
    pub sound_variant_default_resource_id: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaCatMetadataAccessors {
    pub lying_data_id: u8,
    pub lying_default: bool,
    pub relax_state_one_data_id: u8,
    pub relax_state_one_default: bool,
    pub collar_color_data_id: u8,
    pub collar_color_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaBeeMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub roll_mask: i8,
    pub has_stung_mask: i8,
    pub has_nectar_mask: i8,
    pub anger_end_time_data_id: u8,
    pub anger_end_time_default: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaDolphinMetadataAccessors {
    pub got_fish_data_id: u8,
    pub got_fish_default: bool,
    pub moistness_data_id: u8,
    pub moistness_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaFoxMetadataAccessors {
    pub type_data_id: u8,
    pub type_default: i32,
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub sitting_mask: i8,
    pub crouching_mask: i8,
    pub interested_mask: i8,
    pub pouncing_mask: i8,
    pub sleeping_mask: i8,
    pub faceplanted_mask: i8,
    pub defending_mask: i8,
    pub trusted_data_ids: [u8; 2],
    pub trusted_default: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaWolfMetadataAccessors {
    pub interested_data_id: u8,
    pub interested_default: bool,
    pub collar_color_data_id: u8,
    pub collar_color_default: i32,
    pub anger_end_time_data_id: u8,
    pub anger_end_time_default: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaTropicalFishVariantAccessors {
    pub variant_data_id: u8,
    pub variant_default: i32,
    pub pattern_mask: i32,
    pub base_color_mask: i32,
    pub base_color_shift: i32,
    pub pattern_color_mask: i32,
    pub pattern_color_shift: i32,
}

impl VanillaTropicalFishVariantAccessors {
    pub fn pattern_id(self, packed: i32) -> i32 {
        packed & self.pattern_mask
    }

    pub fn base_color_id(self, packed: i32) -> i32 {
        (packed >> self.base_color_shift) & self.base_color_mask
    }

    pub fn pattern_color_id(self, packed: i32) -> i32 {
        (packed >> self.pattern_color_shift) & self.pattern_color_mask
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaMobMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub no_ai_mask: i8,
    pub left_handed_mask: i8,
    pub aggressive_mask: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaEntityMetadataAccessors {
    pub shared_flags_data_id: u8,
    pub shared_flags_default: i8,
    pub on_fire_mask: i8,
    pub shift_key_down_mask: i8,
    pub sprinting_mask: i8,
    pub swimming_mask: i8,
    pub invisible_mask: i8,
    pub glowing_mask: i8,
    pub fall_flying_mask: i8,
    pub air_supply_data_id: u8,
    pub custom_name_data_id: u8,
    pub custom_name_visible_data_id: u8,
    pub custom_name_visible_default: bool,
    pub silent_data_id: u8,
    pub silent_default: bool,
    pub no_gravity_data_id: u8,
    pub no_gravity_default: bool,
    pub pose_data_id: u8,
    pub pose_default: i32,
    pub ticks_frozen_data_id: u8,
    pub ticks_frozen_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaLivingEntityMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub is_using_mask: i8,
    pub off_hand_mask: i8,
    pub spin_attack_mask: i8,
    pub health_data_id: u8,
    pub health_default: f32,
    pub arrow_count_data_id: u8,
    pub arrow_count_default: i32,
    pub stinger_count_data_id: u8,
    pub stinger_count_default: i32,
    pub sleeping_pos_data_id: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaTurtleMetadataAccessors {
    pub has_egg_data_id: u8,
    pub has_egg_default: bool,
    pub laying_egg_data_id: u8,
    pub laying_egg_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaHangingEntityMetadataAccessor {
    pub direction_data_id: u8,
    pub direction_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaItemFrameMetadataAccessors {
    pub direction_data_id: u8,
    pub direction_default: i32,
    pub item_data_id: u8,
    pub rotation_data_id: u8,
    pub rotation_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaAbstractBoatMetadataAccessors {
    pub paddle_left_data_id: u8,
    pub paddle_left_default: bool,
    pub paddle_right_data_id: u8,
    pub paddle_right_default: bool,
    pub bubble_time_data_id: u8,
    pub bubble_time_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaVehicleMetadataAccessors {
    pub hurt_time_data_id: u8,
    pub hurt_time_default: i32,
    pub hurt_dir_data_id: u8,
    pub hurt_dir_default: i32,
    pub damage_data_id: u8,
    pub damage_default: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaPrimedTntMetadataAccessors {
    pub fuse_data_id: u8,
    pub fuse_default: i32,
    pub block_state_data_id: u8,
    pub block_state_default: VanillaEntityMetadataDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaFireworkRocketMetadataAccessors {
    pub item_data_id: u8,
    pub item_default_resource_id: &'static str,
    pub attached_to_target_data_id: u8,
    pub attached_to_target_default: Option<i32>,
    pub shot_at_angle_data_id: u8,
    pub shot_at_angle_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VanillaGuardianMetadataAccessors {
    pub(crate) moving_data_id: u8,
    pub(crate) moving_default: bool,
    pub(crate) attack_target_data_id: u8,
    pub(crate) attack_target_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VanillaWitherSideHeadTargetMetadataAccessors {
    pub(crate) target_b_data_id: u8,
    pub(crate) target_b_default: i32,
    pub(crate) target_c_data_id: u8,
    pub(crate) target_c_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaShulkerMetadataAccessors {
    pub attach_face_data_id: u8,
    pub attach_face_default: i32,
    pub peek_data_id: u8,
    pub peek_default: i8,
    pub color_data_id: u8,
    pub color_default: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaAbstractArrowMetadataAccessors {
    pub flags_data_id: u8,
    pub flags_default: i8,
    pub crit_mask: i8,
    pub no_physics_mask: i8,
    pub pierce_level_data_id: u8,
    pub pierce_level_default: i8,
    pub in_ground_data_id: u8,
    pub in_ground_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaArrowMetadataAccessors {
    pub effect_color_data_id: u8,
    pub effect_color_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaThrownTridentMetadataAccessors {
    pub loyalty_data_id: u8,
    pub loyalty_default: i8,
    pub foil_data_id: u8,
    pub foil_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaZombieMetadataAccessors {
    pub baby_data_id: u8,
    pub baby_default: bool,
    pub special_type_data_id: u8,
    pub special_type_default: i32,
    pub drowned_conversion_data_id: u8,
    pub drowned_conversion_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaAbstractMinecartMetadataAccessors {
    pub custom_display_block_data_id: u8,
    pub display_offset_data_id: u8,
    pub display_offset_default: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaArmorStandMetadataAccessors {
    pub client_flags_data_id: u8,
    pub client_flags_default: i8,
    pub small_mask: i8,
    pub show_arms_mask: i8,
    pub no_baseplate_mask: i8,
    pub marker_mask: i8,
    pub head_pose_data_id: u8,
    pub head_pose_default: [f32; 3],
    pub body_pose_data_id: u8,
    pub body_pose_default: [f32; 3],
    pub left_arm_pose_data_id: u8,
    pub left_arm_pose_default: [f32; 3],
    pub right_arm_pose_data_id: u8,
    pub right_arm_pose_default: [f32; 3],
    pub left_leg_pose_data_id: u8,
    pub left_leg_pose_default: [f32; 3],
    pub right_leg_pose_data_id: u8,
    pub right_leg_pose_default: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaInteractionMetadataAccessors {
    pub width_data_id: u8,
    pub width_default: f32,
    pub height_data_id: u8,
    pub height_default: f32,
    pub response_data_id: u8,
    pub response_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaCreeperMetadataAccessors {
    pub swell_dir_data_id: u8,
    pub swell_dir_default: i32,
    pub powered_data_id: u8,
    pub powered_default: bool,
    pub ignited_data_id: u8,
    pub ignited_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaHappyGhastMetadataAccessors {
    pub is_leash_holder_data_id: u8,
    pub is_leash_holder_default: bool,
    pub stays_still_data_id: u8,
    pub stays_still_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaCopperGolemMetadataAccessors {
    pub weather_state_data_id: u8,
    pub weather_state_default: i32,
    pub state_data_id: u8,
    pub state_default: i32,
}
