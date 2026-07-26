use crate::PaintingVariantClientProjection;
use bbb_protocol::attribute_modifier_types::{
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_DIGGING_ID, VANILLA_ENTITY_POSE_DYING_ID,
    VANILLA_ENTITY_POSE_EMERGING_ID, VANILLA_ENTITY_POSE_FALL_FLYING_ID,
    VANILLA_ENTITY_POSE_LONG_JUMPING_ID, VANILLA_ENTITY_POSE_SITTING_ID,
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_SPIN_ATTACK_ID,
    VANILLA_ENTITY_POSE_SWIMMING_ID,
};
use bbb_protocol::packets::{
    AttributeSnapshot, EntityDataEnumSerializer, EntityDataValue, EntityDataValueKind,
};
use serde::{Deserialize, Serialize};

use super::schema::{
    vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type,
    vanilla_entity_metadata_armor_stand_accessors_for_entity_type,
    vanilla_entity_metadata_baby_accessor_for_entity_type,
    vanilla_entity_metadata_baby_behavior_and_age_scale,
    vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type,
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_hanging_direction_accessor_for_entity_type,
    vanilla_entity_metadata_interaction_accessors_for_entity_type,
    vanilla_entity_metadata_item_frame_accessors_for_entity_type,
    vanilla_entity_metadata_passenger_attachment_owner,
    vanilla_entity_metadata_phantom_size_accessor_for_entity_type,
    vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type,
    vanilla_entity_metadata_salmon_variant_accessor_for_entity_type,
    vanilla_entity_metadata_shulker_accessors_for_entity_type,
    vanilla_entity_metadata_slime_size_accessor_for_entity_type,
    vanilla_entity_metadata_sniffer_state_accessor_for_entity_type,
    vanilla_entity_metadata_type_boat_ride_height_multiplier,
    vanilla_entity_metadata_type_is_animal, vanilla_entity_metadata_type_is_camel,
    vanilla_entity_metadata_type_is_living, vanilla_entity_metadata_type_is_pickable,
    vanilla_entity_metadata_vehicle_attachment_owner,
    vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type,
    vanilla_entity_scale_sanitizer, VanillaArmorStandMetadataAccessors, VanillaEntityBabyBehavior,
    VanillaEntityScaleSanitizer, VANILLA_SNIFFER_STATE_DIGGING_ID as SNIFFER_STATE_DIGGING_ID,
};
use super::{vanilla_entity_type_is_squid, EntityClientAnimationState, EntityVec3};
use bbb_protocol::entity_types::*;

mod block_attached;

pub(crate) use block_attached::{
    hanging_facing, item_frame_facing, item_frame_item, item_frame_map_id, item_frame_rotation,
    painting_facing, prospective_item_frame_geometry, prospective_painting_geometry,
};

const SLIME_BASE_SIZE: f32 = 0.52;
const ARMOR_STAND_WIDTH: f32 = 0.5;
const ARMOR_STAND_HEIGHT: f32 = 1.975;
const ARMOR_STAND_SMALL_SCALE: f32 = 0.5;
const ARMADILLO_BABY_SCALE: f32 = 0.6;
const CAMEL_SITTING_HEIGHT_DIFFERENCE: f32 = 1.43;
const CAMEL_BABY_SCALE: f32 = 0.6;
const CAMEL_SITDOWN_DURATION_TICKS: i64 = 40;
const CAMEL_STANDUP_DURATION_TICKS: i64 = 52;
const DOLPHIN_BABY_SCALE: f32 = 0.65;
const FOX_BABY_SCALE: f32 = 0.6;
const GOAT_LONG_JUMPING_SCALE: f32 = 0.7;
const GOAT_BABY_SCALE: f32 = 0.55;
const HAPPY_GHAST_BABY_SCALE: f32 = 0.2375;
const HORSE_BABY_SCALE: f32 = 0.7;
const DEFAULT_AGEABLE_BABY_SCALE: f32 = 0.5;
const VANILLA_SCALE_MIN: f64 = 0.0625;
const VANILLA_SCALE_MAX: f64 = 16.0;
// Vanilla 26.1 baby humanoid-monster dimensions replace the EntityType VEHICLE attachment.
// The common shape uses 0.1875Y; `ZombieVillager` uses 0.125Y. These dimensions have width 0.49
// before the SCALE attribute and are selected through the generated baby/converting accessors.
const HUMANOID_MONSTER_BABY_WIDTH: f32 = 0.49;
const HUMANOID_MONSTER_BABY_VEHICLE_Y: f32 = 0.1875;
const ZOMBIE_VILLAGER_BABY_VEHICLE_Y: f32 = 0.125;
// Vanilla `Spider.getVehicleAttachmentPoint` and `CaveSpider.getVehicleAttachmentPoint` use these
// offsets when the vehicle is no wider than the passenger.
const SPIDER_VEHICLE_ATTACHMENT_Y: f32 = 0.3125;
const CAVE_SPIDER_VEHICLE_ATTACHMENT_Y: f32 = 0.21875;
const SNIFFER_DIGGING_HEIGHT_OFFSET: f32 = 0.4;
const WIND_CHARGE_BOUNDS_SIZE: f32 = 0.3125;
const WIND_CHARGE_BOUNDS_Y_OFFSET: f32 = -0.15;
const DEFAULT_ENTITY_EYE_HEIGHT_RATIO: f32 = 0.85;
pub(crate) const ENTITY_PASSENGER_VEHICLE_BOX_HEIGHT: f32 = 0.0625;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityPickBoundsState {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub pick_radius: f32,
}

impl EntityPickBoundsState {
    pub const fn from_base_size(width: f32, height: f32, pick_radius: f32) -> Self {
        Self {
            min: [-width / 2.0, 0.0, -width / 2.0],
            max: [width / 2.0, height, width / 2.0],
            pick_radius,
        }
    }

    pub const fn from_centered_size(
        x_size: f32,
        y_size: f32,
        z_size: f32,
        pick_radius: f32,
    ) -> Self {
        Self {
            min: [-x_size / 2.0, -y_size / 2.0, -z_size / 2.0],
            max: [x_size / 2.0, y_size / 2.0, z_size / 2.0],
            pick_radius,
        }
    }

    fn scale_dimensions(self, scale: f32) -> Self {
        Self {
            min: [
                self.min[0] * scale,
                self.min[1] * scale,
                self.min[2] * scale,
            ],
            max: [
                self.max[0] * scale,
                self.max[1] * scale,
                self.max[2] * scale,
            ],
            pick_radius: self.pick_radius,
        }
    }

    fn scale_height(self, scale: f32) -> Self {
        Self {
            min: [self.min[0], self.min[1] * scale, self.min[2]],
            max: [self.max[0], self.max[1] * scale, self.max[2]],
            pick_radius: self.pick_radius,
        }
    }

    pub(crate) fn expand_towards_y(self, delta: f32) -> Self {
        let mut bounds = self;
        if delta >= 0.0 {
            bounds.max[1] += delta;
        } else {
            bounds.min[1] += delta;
        }
        bounds
    }
}

pub(crate) fn vanilla_pick_bounds_for_type(entity_type_id: i32) -> Option<EntityPickBoundsState> {
    vanilla_entity_metadata_type_is_pickable(entity_type_id).then_some(())?;
    vanilla_bounds_for_type(entity_type_id)
}

pub(crate) fn vanilla_bounds_for_type(entity_type_id: i32) -> Option<EntityPickBoundsState> {
    let [width, height] = vanilla_entity_type_dimensions(entity_type_id)?;
    let pick_radius = if entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID
        || vanilla_entity_type_is_redirectable_projectile(entity_type_id)
    {
        1.0
    } else {
        0.0
    };
    Some(EntityPickBoundsState::from_base_size(
        width,
        height,
        pick_radius,
    ))
}

pub(crate) fn vanilla_pick_bounds_for_entity_data(
    entity_type_id: i32,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
    painting_variant: Option<&PaintingVariantClientProjection>,
) -> Option<EntityPickBoundsState> {
    vanilla_entity_metadata_type_is_pickable(entity_type_id).then_some(())?;
    vanilla_bounds_for_entity_data(
        entity_type_id,
        add_entity_data,
        data_values,
        attributes,
        client_animations,
        painting_variant,
    )
}

pub(crate) fn vanilla_bounds_for_entity_data(
    entity_type_id: i32,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
    painting_variant: Option<&PaintingVariantClientProjection>,
) -> Option<EntityPickBoundsState> {
    let scale_dimensions = scales_with_living_scale_attribute(entity_type_id, data_values);
    let bounds = if entity_type_id == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
        armor_stand_pick_bounds(entity_type_id, data_values)?
    } else if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_MANNEQUIN_ID
    {
        avatar_pick_bounds(data_values)
    } else if is_living_sleeping(entity_type_id, data_values) {
        living_sleeping_pick_bounds()
    } else if is_warden_fixed_pose(entity_type_id, data_values) {
        EntityPickBoundsState::from_base_size(0.9, 1.0, 0.0)
    } else if vanilla_entity_metadata_type_is_camel(entity_type_id) {
        camel_pick_bounds(entity_type_id, data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_GOAT_ID {
        goat_pick_bounds(entity_type_id, data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_SNIFFER_ID {
        sniffer_pick_bounds(entity_type_id, data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_POLAR_BEAR_ID {
        polar_bear_pick_bounds(entity_type_id, data_values, client_animations)?
    } else if entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_ID {
        shulker_pick_bounds(entity_type_id, data_values, client_animations)?
    } else if let Some(bounds) = baby_pick_bounds(entity_type_id, data_values) {
        bounds
    } else if entity_type_id == VANILLA_ENTITY_TYPE_INTERACTION_ID {
        interaction_pick_bounds(entity_type_id, data_values)?
    } else if entity_type_id == VANILLA_ENTITY_TYPE_ITEM_FRAME_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
    {
        let accessors =
            vanilla_entity_metadata_item_frame_accessors_for_entity_type(entity_type_id)?;
        block_attached::item_frame_pick_bounds(accessors, add_entity_data, data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID {
        let accessor =
            vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(entity_type_id)?;
        block_attached::painting_pick_bounds(
            accessor,
            add_entity_data,
            data_values,
            painting_variant,
        )?
    } else if entity_type_id == VANILLA_ENTITY_TYPE_LEASH_KNOT_ID {
        EntityPickBoundsState::from_base_size(0.375, 0.5, 0.0)
    } else if let Some(bounds) = slime_pick_bounds(entity_type_id, data_values) {
        bounds
    } else if let Some(bounds) = area_effect_cloud_bounds(entity_type_id, data_values) {
        bounds
    } else if let Some(bounds) = pufferfish_pick_bounds(entity_type_id, data_values) {
        bounds
    } else if let Some(bounds) = salmon_pick_bounds(entity_type_id, data_values) {
        bounds
    } else if let Some(bounds) = phantom_pick_bounds(entity_type_id, data_values) {
        bounds
    } else if super::vanilla_entity_type_is_wind_charge(entity_type_id) {
        wind_charge_pick_bounds(entity_type_id)
    } else {
        vanilla_bounds_for_type(entity_type_id)?
    };

    Some(apply_living_scale(
        entity_type_id,
        bounds,
        attributes,
        scale_dimensions,
    ))
}

pub(crate) fn vanilla_model_source_bounds_for_entity_data(
    entity_type_id: i32,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
    painting_variant: Option<&PaintingVariantClientProjection>,
) -> Option<EntityPickBoundsState> {
    if entity_type_id == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
        && armor_stand_is_marker(entity_type_id, data_values)
    {
        // Vanilla `ArmorStand.MARKER_DIMENSIONS = EntityDimensions.fixed(0.0F, 0.0F)`:
        // marker stands are not pickable, but they still submit their marker render model.
        let bounds = EntityPickBoundsState::from_base_size(0.0, 0.0, 0.0);
        return Some(apply_living_scale(
            entity_type_id,
            bounds,
            attributes,
            scales_with_living_scale_attribute(entity_type_id, data_values),
        ));
    }
    vanilla_bounds_for_entity_data(
        entity_type_id,
        add_entity_data,
        data_values,
        attributes,
        client_animations,
        painting_variant,
    )
}

pub(crate) fn vanilla_eye_height_for_entity_data(
    entity_type_id: i32,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
    painting_variant: Option<&PaintingVariantClientProjection>,
) -> Option<f32> {
    if entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
        // Vanilla 26.1 `EntityDimensions.defaultEyeHeight`: `height * 0.85F`.
        return Some(8.0 * DEFAULT_ENTITY_EYE_HEIGHT_RATIO);
    }
    let bounds = vanilla_bounds_for_entity_data(
        entity_type_id,
        add_entity_data,
        data_values,
        attributes,
        client_animations,
        painting_variant,
    )?;
    let height = bounds_height(bounds);
    let base_eye_height = match vanilla_variant_eye_height_basis(entity_type_id, data_values) {
        Some((declared_eye, declared_height)) if declared_height > 0.0 => {
            declared_eye * height / declared_height
        }
        _ => vanilla_eye_height_override_for_type(entity_type_id)
            .and_then(|eye_height| scaled_eye_height_for_bounds(entity_type_id, eye_height, height))
            .unwrap_or(height * DEFAULT_ENTITY_EYE_HEIGHT_RATIO),
    };
    Some(base_eye_height)
}

/// Variant dimensions that vanilla declares with an explicit `withEyeHeight` decouple the eye
/// ratio from the base type table, so the proportional projection substitutes the declared
/// (eye height, height) pair. Age and scale-attribute factors still flow through the
/// actual-to-declared height ratio because vanilla `EntityDimensions.scale` multiplies the eye
/// height by the same height factor; variants without an explicit eye height (goat long-jump,
/// warden digging, plain baby scaling) keep the base ratio and need no entry here.
fn vanilla_variant_eye_height_basis(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<(f32, f32)> {
    if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_MANNEQUIN_ID
    {
        return Some(match entity_data_pose(data_values) {
            VANILLA_ENTITY_POSE_SLEEPING_ID => (0.2, 0.2),
            // Vanilla `Avatar.POSES` keeps the standing eye height on the dying fixed box.
            VANILLA_ENTITY_POSE_DYING_ID => (1.62, 0.2),
            VANILLA_ENTITY_POSE_FALL_FLYING_ID
            | VANILLA_ENTITY_POSE_SWIMMING_ID
            | VANILLA_ENTITY_POSE_SPIN_ATTACK_ID => (0.4, 0.6),
            VANILLA_ENTITY_POSE_CROUCHING_ID => (1.27, 1.5),
            _ => (1.62, 1.8),
        });
    }
    if is_living_sleeping(entity_type_id, data_values) {
        // `LivingEntity.SLEEPING_DIMENSIONS = fixed(0.2, 0.2).withEyeHeight(0.2)`.
        return Some((0.2, 0.2));
    }
    if vanilla_entity_metadata_type_is_camel(entity_type_id)
        && entity_data_pose(data_values) == VANILLA_ENTITY_POSE_SITTING_ID
    {
        return Some((0.845, 2.375 - CAMEL_SITTING_HEIGHT_DIFFERENCE));
    }
    if entity_type_id == VANILLA_ENTITY_TYPE_SNIFFER_ID && sniffer_is_digging(data_values) {
        return Some((0.81, 1.75 - SNIFFER_DIGGING_HEIGHT_OFFSET));
    }
    if entity_type_id == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
        && armor_stand_is_marker(entity_type_id, data_values)
    {
        // Marker stands use `fixed(0, 0)`; the proportional path already collapses to zero.
        return None;
    }
    if !vanilla_is_baby(entity_type_id, data_values) {
        return None;
    }
    Some(match entity_type_id {
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID => (0.9875, 0.9875),
        VANILLA_ENTITY_TYPE_CHICKEN_ID => (0.28, 0.4),
        VANILLA_ENTITY_TYPE_VILLAGER_ID => (0.63, 0.99),
        VANILLA_ENTITY_TYPE_DROWNED_ID | VANILLA_ENTITY_TYPE_ZOMBIE_ID => (0.775, 0.99),
        VANILLA_ENTITY_TYPE_HUSK_ID => (0.825, 0.99),
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => (0.67, 0.99),
        VANILLA_ENTITY_TYPE_PIGLIN_ID | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => (0.78, 0.99),
        VANILLA_ENTITY_TYPE_RABBIT_ID => (0.39, 0.4),
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => (0.2, 0.25),
        VANILLA_ENTITY_TYPE_FOX_ID => (0.2975, 0.42),
        VANILLA_ENTITY_TYPE_COW_ID | VANILLA_ENTITY_TYPE_MOOSHROOM_ID => (0.665, 0.7),
        _ if vanilla_entity_type_is_squid(entity_type_id) => (0.37, 0.63),
        _ => return None,
    })
}

pub(crate) fn vanilla_client_position_for_entity_data(
    entity_type_id: i32,
    packet_position: EntityVec3,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    painting_variant: Option<&PaintingVariantClientProjection>,
) -> Option<EntityVec3> {
    if entity_type_id == VANILLA_ENTITY_TYPE_ITEM_FRAME_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
    {
        let accessors =
            vanilla_entity_metadata_item_frame_accessors_for_entity_type(entity_type_id)?;
        return Some(block_attached::item_frame_center(
            accessors,
            packet_position,
            add_entity_data,
            data_values,
        ));
    }
    if entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID {
        let accessor =
            vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(entity_type_id)?;
        return block_attached::painting_center(
            accessor,
            packet_position,
            add_entity_data,
            data_values,
            painting_variant,
        );
    }
    if entity_type_id == VANILLA_ENTITY_TYPE_LEASH_KNOT_ID {
        return Some(block_attached::leash_knot_position(packet_position));
    }
    None
}

pub(crate) fn vanilla_shulker_attached_position(position: EntityVec3) -> EntityVec3 {
    EntityVec3 {
        x: position.x.floor() + 0.5,
        y: (position.y + 0.5).floor(),
        z: position.z.floor() + 0.5,
    }
}

pub(crate) fn vanilla_body_anchor_y_offset_for_entity_data(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    game_time: i64,
    is_front: bool,
    partial_ticks: f32,
) -> Option<f32> {
    if !vanilla_entity_metadata_type_is_camel(entity_type_id) {
        return None;
    }

    let dimensions = vanilla_pick_bounds_for_entity_data(
        entity_type_id,
        0,
        data_values,
        attributes,
        None,
        None,
    )?;
    let scale = camel_age_scale(entity_type_id, data_values)
        * vanilla_render_scale(entity_type_id, attributes);
    let (data_id, default) =
        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            entity_type_id,
        )?;
    let last_pose_change_tick = entity_data_long(data_values, data_id, default);
    Some(camel_body_anchor_animation_y_offset(
        dimensions,
        scale,
        last_pose_change_tick,
        game_time,
        is_front,
        partial_ticks,
    ))
}

/// Vanilla `Camel.getLeashOffset(partialTicks)`.
///
/// The current dimensions include both age and `SCALE` attribute scaling, while
/// the animation curve and the final `0.2` subtraction intentionally receive
/// `getAgeScale()` only. This differs from the Camel passenger-attachment path,
/// whose explicit scale argument is `getScale() * getAgeScale()`.
pub(crate) fn vanilla_camel_leash_offset_for_entity_data(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    game_time: i64,
    partial_ticks: f32,
) -> Option<[f64; 3]> {
    if !vanilla_entity_metadata_type_is_camel(entity_type_id) {
        return None;
    }

    let dimensions = vanilla_pick_bounds_for_entity_data(
        entity_type_id,
        0,
        data_values,
        attributes,
        None,
        None,
    )?;
    let age_scale = camel_age_scale(entity_type_id, data_values);
    let (data_id, default) =
        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            entity_type_id,
        )?;
    let last_pose_change_tick = entity_data_long(data_values, data_id, default);
    let y = camel_body_anchor_animation_y_offset(
        dimensions,
        age_scale,
        last_pose_change_tick,
        game_time,
        true,
        partial_ticks,
    ) - 0.2 * age_scale;
    Some([
        0.0,
        f64::from(y),
        f64::from(bounds_width(dimensions) * 0.56),
    ])
}

pub(crate) fn vanilla_passenger_vehicle_debug_target_for_entity_data(
    vehicle_entity_type_id: i32,
    vehicle_position: EntityVec3,
    vehicle_y_rot: f32,
    vehicle_bounds: EntityPickBoundsState,
    vehicle_data_values: &[EntityDataValue],
    vehicle_attributes: &[AttributeSnapshot],
    vehicle_client_animations: Option<EntityClientAnimationState>,
    vehicle_passengers: &[i32],
    passenger_id: i32,
    passenger_entity_type_id: i32,
    passenger_bounds: EntityPickBoundsState,
    game_time: i64,
) -> Option<(EntityVec3, EntityPickBoundsState)> {
    let position = vanilla_passenger_riding_position_for_entity_data(
        vehicle_entity_type_id,
        vehicle_position,
        vehicle_y_rot,
        vehicle_bounds,
        vehicle_data_values,
        vehicle_attributes,
        vehicle_client_animations,
        vehicle_passengers,
        passenger_id,
        passenger_entity_type_id,
        game_time,
    )?;
    let half_width = 0.5 * bounds_width(vehicle_bounds).min(bounds_width(passenger_bounds));
    Some((
        position,
        EntityPickBoundsState::from_base_size(
            half_width * 2.0,
            ENTITY_PASSENGER_VEHICLE_BOX_HEIGHT,
            0.0,
        ),
    ))
}

pub(crate) fn vanilla_passenger_position_for_entity_data(
    vehicle_entity_type_id: i32,
    vehicle_position: EntityVec3,
    vehicle_y_rot: f32,
    vehicle_bounds: EntityPickBoundsState,
    vehicle_data_values: &[EntityDataValue],
    vehicle_attributes: &[AttributeSnapshot],
    vehicle_client_animations: Option<EntityClientAnimationState>,
    vehicle_passengers: &[i32],
    passenger_id: i32,
    passenger_entity_type_id: i32,
    passenger_y_rot: f32,
    passenger_bounds: EntityPickBoundsState,
    passenger_data_values: &[EntityDataValue],
    game_time: i64,
) -> Option<EntityVec3> {
    let riding_position = vanilla_passenger_riding_position_for_entity_data(
        vehicle_entity_type_id,
        vehicle_position,
        vehicle_y_rot,
        vehicle_bounds,
        vehicle_data_values,
        vehicle_attributes,
        vehicle_client_animations,
        vehicle_passengers,
        passenger_id,
        passenger_entity_type_id,
        game_time,
    )?;
    let vehicle_attachment = vanilla_vehicle_attachment_point(
        passenger_entity_type_id,
        passenger_y_rot,
        vehicle_bounds,
        passenger_bounds,
        passenger_data_values,
    );
    Some(EntityVec3 {
        x: riding_position.x - vehicle_attachment[0],
        y: riding_position.y - vehicle_attachment[1],
        z: riding_position.z - vehicle_attachment[2],
    })
}

pub(crate) fn vanilla_local_player_passenger_position_for_entity_data(
    vehicle_entity_type_id: i32,
    vehicle_position: EntityVec3,
    vehicle_y_rot: f32,
    vehicle_bounds: EntityPickBoundsState,
    vehicle_data_values: &[EntityDataValue],
    vehicle_attributes: &[AttributeSnapshot],
    vehicle_client_animations: Option<EntityClientAnimationState>,
    vehicle_passengers: &[i32],
    passenger_id: i32,
    passenger_is_swimming: bool,
    game_time: i64,
) -> Option<EntityVec3> {
    let riding_position = vanilla_passenger_riding_position_for_entity_data(
        vehicle_entity_type_id,
        vehicle_position,
        vehicle_y_rot,
        vehicle_bounds,
        vehicle_data_values,
        vehicle_attributes,
        vehicle_client_animations,
        vehicle_passengers,
        passenger_id,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        game_time,
    )?;
    let vehicle_attachment = if passenger_is_swimming {
        [0.0; 3]
    } else {
        vanilla_entity_type_vehicle_attachment(VANILLA_ENTITY_TYPE_PLAYER_ID)?
    };
    Some(EntityVec3 {
        x: riding_position.x - f64::from(vehicle_attachment[0]),
        y: riding_position.y - f64::from(vehicle_attachment[1]),
        z: riding_position.z - f64::from(vehicle_attachment[2]),
    })
}

fn vanilla_passenger_riding_position_for_entity_data(
    vehicle_entity_type_id: i32,
    vehicle_position: EntityVec3,
    vehicle_y_rot: f32,
    vehicle_bounds: EntityPickBoundsState,
    vehicle_data_values: &[EntityDataValue],
    vehicle_attributes: &[AttributeSnapshot],
    vehicle_client_animations: Option<EntityClientAnimationState>,
    vehicle_passengers: &[i32],
    passenger_id: i32,
    passenger_entity_type_id: i32,
    game_time: i64,
) -> Option<EntityVec3> {
    let passenger_index = vehicle_passengers
        .iter()
        .position(|id| *id == passenger_id)?;
    let attachment = vanilla_passenger_attachment_point(
        vehicle_entity_type_id,
        vehicle_y_rot,
        vehicle_bounds,
        vehicle_data_values,
        vehicle_attributes,
        vehicle_client_animations,
        vehicle_passengers,
        passenger_index,
        passenger_entity_type_id,
        game_time,
    );
    Some(EntityVec3 {
        x: vehicle_position.x + attachment[0],
        y: vehicle_position.y + attachment[1],
        z: vehicle_position.z + attachment[2],
    })
}

fn bounds_height(bounds: EntityPickBoundsState) -> f32 {
    bounds.max[1] - bounds.min[1]
}

fn bounds_width(bounds: EntityPickBoundsState) -> f32 {
    bounds.max[0] - bounds.min[0]
}

fn vanilla_passenger_attachment_point(
    vehicle_entity_type_id: i32,
    vehicle_y_rot: f32,
    vehicle_bounds: EntityPickBoundsState,
    vehicle_data_values: &[EntityDataValue],
    vehicle_attributes: &[AttributeSnapshot],
    vehicle_client_animations: Option<EntityClientAnimationState>,
    vehicle_passengers: &[i32],
    passenger_index: usize,
    passenger_entity_type_id: i32,
    game_time: i64,
) -> [f64; 3] {
    let attachment_owner =
        vanilla_entity_metadata_passenger_attachment_owner(vehicle_entity_type_id);
    if attachment_owner == Some("net.minecraft.world.entity.vehicle.boat.AbstractBoat") {
        return rotated_y(
            boat_passenger_attachment_point(
                vehicle_entity_type_id,
                vehicle_bounds,
                vehicle_passengers,
                passenger_index,
                passenger_entity_type_id,
            ),
            vehicle_y_rot,
        );
    }

    if attachment_owner == Some("net.minecraft.world.entity.animal.camel.Camel") {
        return rotated_y(
            camel_passenger_attachment_point(
                vehicle_entity_type_id,
                vehicle_bounds,
                vehicle_data_values,
                vehicle_attributes,
                vehicle_passengers,
                passenger_index,
                passenger_entity_type_id,
                game_time,
            ),
            vehicle_y_rot,
        );
    }

    if attachment_owner == Some("net.minecraft.world.entity.vehicle.minecart.AbstractMinecart")
        && (passenger_entity_type_id == VANILLA_ENTITY_TYPE_VILLAGER_ID
            || passenger_entity_type_id == VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID)
    {
        return [0.0, 0.0, 0.0];
    }

    if vehicle_entity_type_id == VANILLA_ENTITY_TYPE_TURTLE_ID
        && vanilla_is_baby(vehicle_entity_type_id, vehicle_data_values)
    {
        let base_bounds = vanilla_pick_bounds_for_type(vehicle_entity_type_id)
            .expect("generated Turtle dimensions must resolve");
        let points = vanilla_entity_type_passenger_attachments(vehicle_entity_type_id)
            .expect("generated Turtle passenger attachment must resolve");
        let point = points[passenger_index.min(points.len() - 1)];
        let xz_scale = scale_axis(bounds_width(vehicle_bounds), bounds_width(base_bounds));
        return rotated_y(
            [
                point[0] * xz_scale,
                bounds_height(vehicle_bounds),
                point[2] * xz_scale,
            ],
            vehicle_y_rot,
        );
    }

    let mut point = vanilla_default_passenger_attachment_point(
        vehicle_entity_type_id,
        vehicle_bounds,
        passenger_index,
        vehicle_data_values,
    );

    if attachment_owner == Some("net.minecraft.world.entity.monster.Strider") {
        // Vanilla `Strider.getPassengerAttachmentPoint` adds this only on the
        // client. `position()` / `speed()` are the current (non-partial)
        // WalkAnimationState values used by `positionRider` each client tick;
        // render interpolation comes from the root-to-leaf passenger pose.
        let (animation_position, animation_speed) = vehicle_client_animations
            .map(|animations| {
                (
                    animations.walk_animation_position(1.0),
                    animations.walk_animation_speed(1.0).min(0.25),
                )
            })
            .unwrap_or((0.0, 0.0));
        let scale = vanilla_age_scale(vehicle_entity_type_id, vehicle_data_values)
            * vanilla_render_scale(vehicle_entity_type_id, vehicle_attributes) as f32;
        point[1] += 0.12 * (animation_position * 1.5).cos() * 2.0 * animation_speed * scale;
    } else if attachment_owner == Some("net.minecraft.world.entity.monster.Slime") {
        let (data_id, default) =
            vanilla_entity_metadata_slime_size_accessor_for_entity_type(vehicle_entity_type_id)
                .expect("generated Slime attachment owner must expose its size accessor");
        point = [
            0.0,
            bounds_height(vehicle_bounds)
                - 0.015625
                    * entity_data_int(vehicle_data_values, data_id, default) as f32
                    * vanilla_render_scale(vehicle_entity_type_id, vehicle_attributes) as f32,
            0.0,
        ];
    } else if attachment_owner == Some("net.minecraft.world.entity.animal.equine.AbstractHorse") {
        let stand_animation = vehicle_client_animations
            .map(|animations| animations.equine_stand_animation(0.0))
            .unwrap_or(0.0);
        let render_scale = vanilla_render_scale(vehicle_entity_type_id, vehicle_attributes) as f32;
        let offset = rotated_y(
            [
                0.0,
                0.15 * stand_animation * render_scale,
                -0.7 * stand_animation * render_scale,
            ],
            vehicle_y_rot,
        );
        let base = rotated_y(point, vehicle_y_rot);
        return [
            base[0] + offset[0],
            base[1] + offset[1],
            base[2] + offset[2],
        ];
    }

    rotated_y(point, vehicle_y_rot)
}

fn vanilla_vehicle_attachment_point(
    entity_type_id: i32,
    y_rot: f32,
    vehicle_bounds: EntityPickBoundsState,
    bounds: EntityPickBoundsState,
    data_values: &[EntityDataValue],
) -> [f64; 3] {
    if is_living_sleeping(entity_type_id, data_values) {
        return [0.0; 3];
    }
    if (entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_MANNEQUIN_ID)
        && matches!(
            entity_data_pose(data_values),
            VANILLA_ENTITY_POSE_FALL_FLYING_ID
                | VANILLA_ENTITY_POSE_SWIMMING_ID
                | VANILLA_ENTITY_POSE_SPIN_ATTACK_ID
                | VANILLA_ENTITY_POSE_DYING_ID
        )
    {
        return [0.0; 3];
    }

    let vehicle_width = bounds_width(vehicle_bounds);
    let passenger_width = bounds_width(bounds);
    if vehicle_width <= passenger_width {
        let spider_y = match vanilla_entity_metadata_vehicle_attachment_owner(entity_type_id) {
            Some("net.minecraft.world.entity.monster.spider.Spider") => {
                Some(SPIDER_VEHICLE_ATTACHMENT_Y)
            }
            Some("net.minecraft.world.entity.monster.spider.CaveSpider") => {
                Some(CAVE_SPIDER_VEHICLE_ATTACHMENT_Y)
            }
            _ => None,
        };
        if let Some(y) = spider_y {
            let base_bounds = vanilla_pick_bounds_for_type(entity_type_id).unwrap_or(bounds);
            let scale = scale_axis(passenger_width, bounds_width(base_bounds));
            return rotated_y([0.0, y * scale, 0.0], y_rot);
        }
    }

    let baby_bounds = baby_pick_bounds(entity_type_id, data_values);
    let base_vehicle_attachment = vanilla_entity_type_vehicle_attachment(entity_type_id);
    if baby_bounds.is_some_and(|baby_bounds| {
        (bounds_width(baby_bounds) - HUMANOID_MONSTER_BABY_WIDTH).abs() <= f32::EPSILON
    }) && base_vehicle_attachment.is_some_and(|point| point != [0.0; 3])
    {
        let y = if vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type(
            entity_type_id,
        )
        .is_some()
        {
            ZOMBIE_VILLAGER_BABY_VEHICLE_Y
        } else {
            HUMANOID_MONSTER_BABY_VEHICLE_Y
        };
        let scale = scale_axis(passenger_width, HUMANOID_MONSTER_BABY_WIDTH);
        return rotated_y([0.0, y * scale, 0.0], y_rot);
    }

    let Some(mut point) = vanilla_entity_type_vehicle_attachment(entity_type_id) else {
        return [0.0; 3];
    };
    if point == [0.0; 3] {
        return [0.0; 3];
    }

    let base_bounds = vanilla_pick_bounds_for_type(entity_type_id).unwrap_or(bounds);
    let scale = scale_axis(bounds_width(bounds), bounds_width(base_bounds));
    point[0] *= scale;
    point[1] *= scale;
    point[2] *= scale;
    rotated_y(point, y_rot)
}

fn boat_passenger_attachment_point(
    vehicle_entity_type_id: i32,
    vehicle_bounds: EntityPickBoundsState,
    vehicle_passengers: &[i32],
    passenger_index: usize,
    passenger_entity_type_id: i32,
) -> [f32; 3] {
    let mut offset = if is_vanilla_chest_boat_or_raft_type(vehicle_entity_type_id) {
        0.15
    } else {
        0.0
    };
    if vehicle_passengers.len() > 1 {
        offset = if passenger_index == 0 { 0.2 } else { -0.6 };
        if vanilla_animal_entity_type(passenger_entity_type_id) {
            offset += 0.2;
        }
    }

    let ride_height_multiplier =
        vanilla_entity_metadata_type_boat_ride_height_multiplier(vehicle_entity_type_id)
            .expect("generated vanilla boat type must have a ride-height multiplier");
    let ride_height = bounds_height(vehicle_bounds) * ride_height_multiplier;
    [0.0, ride_height, offset]
}

fn camel_passenger_attachment_point(
    vehicle_entity_type_id: i32,
    vehicle_bounds: EntityPickBoundsState,
    vehicle_data_values: &[EntityDataValue],
    vehicle_attributes: &[AttributeSnapshot],
    vehicle_passengers: &[i32],
    passenger_index: usize,
    passenger_entity_type_id: i32,
    game_time: i64,
) -> [f32; 3] {
    let is_front = passenger_index == 0;
    let mut offset = 0.5;
    if vehicle_passengers.len() > 1 {
        if !is_front {
            offset = -0.7;
        }
        if vanilla_animal_entity_type(passenger_entity_type_id) {
            offset += 0.2;
        }
    }

    let scale = camel_age_scale(vehicle_entity_type_id, vehicle_data_values)
        * vanilla_render_scale(vehicle_entity_type_id, vehicle_attributes) as f32;
    let last_pose_change_tick =
        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            vehicle_entity_type_id,
        )
        .map(|(data_id, default)| entity_data_long(vehicle_data_values, data_id, default))
        .unwrap_or_default();
    [
        0.0,
        camel_body_anchor_animation_y_offset(
            vehicle_bounds,
            scale,
            last_pose_change_tick,
            game_time,
            is_front,
            0.0,
        ),
        offset * scale,
    ]
}

fn vanilla_default_passenger_attachment_point(
    entity_type_id: i32,
    bounds: EntityPickBoundsState,
    passenger_index: usize,
    data_values: &[EntityDataValue],
) -> [f32; 3] {
    if vanilla_is_baby(entity_type_id, data_values) {
        if let Some((point, declared_width, declared_height)) =
            vanilla_baby_passenger_attachment_basis(entity_type_id)
        {
            let xz_scale = scale_axis(bounds_width(bounds), declared_width);
            let y_scale = scale_axis(bounds_height(bounds), declared_height);
            return [point[0] * xz_scale, point[1] * y_scale, point[2] * xz_scale];
        }
    }
    let Some(points) = vanilla_passenger_attachment_overrides(entity_type_id) else {
        return [0.0, bounds_height(bounds), 0.0];
    };
    let base_bounds = vanilla_pick_bounds_for_type(entity_type_id).unwrap_or(bounds);
    let xz_scale = scale_axis(bounds_width(bounds), bounds_width(base_bounds));
    let y_scale = scale_axis(bounds_height(bounds), bounds_height(base_bounds));
    let point = points[passenger_index.min(points.len() - 1)];
    [point[0] * xz_scale, point[1] * y_scale, point[2] * xz_scale]
}

/// Vanilla baby variants that re-declare their passenger attachment through `withAttachments`
/// decouple the riding point from the adult table, mirroring the variant eye-height basis: the
/// declared point is paired with the (width, height) it was declared at, so age and
/// scale-attribute factors flow through the actual-to-declared ratio. Equine and llama babies
/// declare before their dimension scale (adult-size basis); the panda declares after its 0.5
/// scale, so its basis is the baby size.
fn vanilla_baby_passenger_attachment_basis(entity_type_id: i32) -> Option<([f32; 3], f32, f32)> {
    Some(match entity_type_id {
        // `EntityType.HORSE.getHeight() - 0.125F` at the adult 1.396_484_4 x 1.6 size.
        VANILLA_ENTITY_TYPE_HORSE_ID => ([0.0, 1.475, 0.0], 1.396_484_4, 1.6),
        // `getHeight() - 0.25F` at the adult size.
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => {
            ([0.0, 1.35, 0.0], 1.396_484_4, 1.6)
        }
        // `AbstractChestedHorse`: `getHeight() + 0.03125F`, z -0.3125 at the adult size.
        VANILLA_ENTITY_TYPE_DONKEY_ID => ([0.0, 1.531_25, -0.3125], 1.396_484_4, 1.5),
        VANILLA_ENTITY_TYPE_MULE_ID => ([0.0, 1.631_25, -0.3125], 1.396_484_4, 1.6),
        // `EntityType.LLAMA.getHeight() - 0.25F`, z -0.3 at the adult 0.9 x 1.87 size.
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => {
            ([0.0, 1.62, -0.3], 0.9, 1.87)
        }
        // Panda attaches 0.40625 after `scale(0.5)`, so the basis is the 0.65 x 0.625 baby box.
        VANILLA_ENTITY_TYPE_PANDA_ID => ([0.0, 0.406_25, 0.0], 0.65, 0.625),
        _ => return None,
    })
}

fn vanilla_passenger_attachment_overrides(entity_type_id: i32) -> Option<&'static [[f32; 3]]> {
    vanilla_entity_type_passenger_attachments(entity_type_id)
}

fn scale_axis(current: f32, base: f32) -> f32 {
    if base.abs() > f32::EPSILON {
        current / base
    } else {
        1.0
    }
}

fn rotated_y(point: [f32; 3], y_rot: f32) -> [f64; 3] {
    let radians = -y_rot.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    [
        f64::from(point[0] * cos + point[2] * sin),
        f64::from(point[1]),
        f64::from(point[2] * cos - point[0] * sin),
    ]
}

fn is_vanilla_chest_boat_or_raft_type(entity_type_id: i32) -> bool {
    super::vanilla_entity_type_is_chest_boat(entity_type_id)
}

fn lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

fn scaled_eye_height_for_bounds(
    entity_type_id: i32,
    base_eye_height: f32,
    actual_height: f32,
) -> Option<f32> {
    let base_height = vanilla_bounds_for_type(entity_type_id).map(bounds_height)?;
    (base_height > 0.0).then_some(base_eye_height * actual_height / base_height)
}

fn vanilla_eye_height_override_for_type(entity_type_id: i32) -> Option<f32> {
    vanilla_entity_type_eye_height(entity_type_id)
}

fn polar_bear_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    client_animations: Option<EntityClientAnimationState>,
) -> Option<EntityPickBoundsState> {
    let bounds = if vanilla_is_baby(entity_type_id, data_values) {
        vanilla_pick_bounds_for_type(VANILLA_ENTITY_TYPE_POLAR_BEAR_ID)?
            .scale_dimensions(DEFAULT_AGEABLE_BABY_SCALE)
    } else {
        vanilla_pick_bounds_for_type(VANILLA_ENTITY_TYPE_POLAR_BEAR_ID)?
    };

    let height_scale = client_animations
        .and_then(|animations| animations.polar_bear_standing)
        .map(|standing| standing.dimensions_height_scale())
        .unwrap_or(1.0);
    Some(bounds.scale_height(height_scale))
}

fn shulker_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    client_animations: Option<EntityClientAnimationState>,
) -> Option<EntityPickBoundsState> {
    let accessors = vanilla_entity_metadata_shulker_accessors_for_entity_type(entity_type_id)?;
    let direction = entity_data_direction(data_values, accessors.attach_face_data_id)
        .unwrap_or_else(|| vanilla_direction_from_3d_data(accessors.attach_face_default))
        .opposite();
    let peek_amount = client_animations
        .and_then(|animations| animations.shulker_peek)
        .map(|peek| peek.current_peek_amount)
        .unwrap_or(0.0);
    let physical_peek = 0.5 - ((0.5 + peek_amount) * std::f32::consts::PI).sin() * 0.5;
    Some(shulker_progress_pick_bounds(direction, physical_peek))
}

fn shulker_progress_pick_bounds(
    direction: VanillaDirection,
    progress: f32,
) -> EntityPickBoundsState {
    let mut bounds = EntityPickBoundsState::from_base_size(1.0, 1.0, 0.0);
    let step = direction.step();
    for (axis, step) in step.into_iter().enumerate() {
        let delta = step as f32 * progress;
        if delta < 0.0 {
            bounds.min[axis] += delta;
        } else {
            bounds.max[axis] += delta;
        }
    }
    bounds
}

fn interaction_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityPickBoundsState> {
    let accessors = vanilla_entity_metadata_interaction_accessors_for_entity_type(entity_type_id)?;
    Some(EntityPickBoundsState::from_base_size(
        entity_data_float(
            data_values,
            accessors.width_data_id,
            accessors.width_default,
        ),
        entity_data_float(
            data_values,
            accessors.height_data_id,
            accessors.height_default,
        ),
        0.0,
    ))
}

fn avatar_pick_bounds(data_values: &[EntityDataValue]) -> EntityPickBoundsState {
    match entity_data_pose(data_values) {
        VANILLA_ENTITY_POSE_SLEEPING_ID | VANILLA_ENTITY_POSE_DYING_ID => {
            living_sleeping_pick_bounds()
        }
        VANILLA_ENTITY_POSE_FALL_FLYING_ID
        | VANILLA_ENTITY_POSE_SWIMMING_ID
        | VANILLA_ENTITY_POSE_SPIN_ATTACK_ID => {
            EntityPickBoundsState::from_base_size(0.6, 0.6, 0.0)
        }
        VANILLA_ENTITY_POSE_CROUCHING_ID => EntityPickBoundsState::from_base_size(0.6, 1.5, 0.0),
        _ => EntityPickBoundsState::from_base_size(0.6, 1.8, 0.0),
    }
}

fn living_sleeping_pick_bounds() -> EntityPickBoundsState {
    EntityPickBoundsState::from_base_size(0.2, 0.2, 0.0)
}

fn camel_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> EntityPickBoundsState {
    let height = if entity_data_pose(data_values) == VANILLA_ENTITY_POSE_SITTING_ID {
        2.375 - CAMEL_SITTING_HEIGHT_DIFFERENCE
    } else {
        2.375
    };
    let bounds = EntityPickBoundsState::from_base_size(1.7, height, 0.0);

    if entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_ID
        && vanilla_is_baby(entity_type_id, data_values)
    {
        bounds.scale_dimensions(CAMEL_BABY_SCALE)
    } else {
        bounds
    }
}

fn camel_age_scale(entity_type_id: i32, data_values: &[EntityDataValue]) -> f32 {
    if entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_ID
        && vanilla_is_baby(entity_type_id, data_values)
    {
        CAMEL_BABY_SCALE
    } else {
        1.0
    }
}

fn camel_body_anchor_animation_y_offset(
    dimensions: EntityPickBoundsState,
    scale: f32,
    last_pose_change_tick: i64,
    game_time: i64,
    is_front: bool,
    partial_ticks: f32,
) -> f32 {
    let partial_ticks = if partial_ticks.is_finite() {
        partial_ticks.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut base_sit_offset = bounds_height(dimensions) - 0.375 * scale;
    let sitting_height_difference = scale * CAMEL_SITTING_HEIGHT_DIFFERENCE;
    let vertical_drop = sitting_height_difference - scale * 0.2;
    let bottom_point = sitting_height_difference - vertical_drop;
    let pose_time = game_time - last_pose_change_tick.saturating_abs();
    let is_sitting = last_pose_change_tick < 0;
    let transition_duration = if is_sitting {
        CAMEL_SITDOWN_DURATION_TICKS
    } else {
        CAMEL_STANDUP_DURATION_TICKS
    };
    let is_in_transition = pose_time < transition_duration;

    if is_in_transition {
        let (half_point, flex_point_offset) = if is_sitting {
            (28, if is_front { 0.5 } else { 0.1 })
        } else {
            (
                if is_front { 24 } else { 32 },
                if is_front { 0.6 } else { 0.35 },
            )
        };
        let pose_time = (pose_time as f32 + partial_ticks).clamp(0.0, transition_duration as f32);
        let is_first_part = pose_time < half_point as f32;
        let part = if is_first_part {
            pose_time / half_point as f32
        } else {
            (pose_time - half_point as f32) / (transition_duration - half_point) as f32
        };
        let flex_point = sitting_height_difference - flex_point_offset * vertical_drop;
        base_sit_offset += if is_sitting {
            if is_first_part {
                lerp(part, sitting_height_difference, flex_point)
            } else {
                lerp(part, flex_point, bottom_point)
            }
        } else if is_first_part {
            lerp(
                part,
                bottom_point - sitting_height_difference,
                bottom_point - flex_point,
            )
        } else {
            lerp(part, bottom_point - flex_point, 0.0)
        };
    }

    if is_sitting && !is_in_transition {
        base_sit_offset += bottom_point;
    }

    base_sit_offset
}

fn goat_pick_bounds(entity_type_id: i32, data_values: &[EntityDataValue]) -> EntityPickBoundsState {
    let bounds = if entity_data_pose(data_values) == VANILLA_ENTITY_POSE_LONG_JUMPING_ID {
        EntityPickBoundsState::from_base_size(0.9, 1.3, 0.0)
            .scale_dimensions(GOAT_LONG_JUMPING_SCALE)
    } else {
        EntityPickBoundsState::from_base_size(0.9, 1.3, 0.0)
    };

    if vanilla_is_baby(entity_type_id, data_values) {
        bounds.scale_dimensions(GOAT_BABY_SCALE)
    } else {
        bounds
    }
}

pub(crate) fn sniffer_is_digging(data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(VANILLA_ENTITY_TYPE_SNIFFER_ID)
        .is_some_and(|(data_id, default)| {
            entity_data_enum_id(
                data_values,
                data_id,
                EntityDataEnumSerializer::SnifferState,
                default,
            ) == SNIFFER_STATE_DIGGING_ID
        })
}

fn sniffer_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> EntityPickBoundsState {
    let digging =
        entity_type_id == VANILLA_ENTITY_TYPE_SNIFFER_ID && sniffer_is_digging(data_values);
    let height = if digging {
        1.75 - SNIFFER_DIGGING_HEIGHT_OFFSET
    } else {
        1.75
    };
    let bounds = EntityPickBoundsState::from_base_size(1.9, height, 0.0);

    if vanilla_is_baby(entity_type_id, data_values) {
        bounds.scale_dimensions(DEFAULT_AGEABLE_BABY_SCALE)
    } else {
        bounds
    }
}

/// Vanilla `LivingEntity.isBaby`: follows the source-derived nearest class implementation. Most
/// ageable/zombie/piglin/zoglin families read their generated `DATA_BABY_ID`; ArmorStand reads its
/// generated small flag; explicit false overrides such as CamelHusk, Frog, Parrot, and
/// ZombieNautilus remain adults even though they inherit an ageable metadata accessor.
pub(crate) fn vanilla_is_baby(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    let Some((behavior, _)) = vanilla_entity_metadata_baby_behavior_and_age_scale(entity_type_id)
    else {
        return false;
    };
    match behavior {
        VanillaEntityBabyBehavior::Never => false,
        VanillaEntityBabyBehavior::Metadata => {
            let Some((data_id, default)) =
                vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
            else {
                return false;
            };
            entity_data_bool(data_values, data_id, default)
        }
        VanillaEntityBabyBehavior::ArmorStandSmall => {
            armor_stand_client_flags(entity_type_id, data_values)
                .is_some_and(|(accessors, flags)| flags & accessors.small_mask != 0)
        }
    }
}

/// Vanilla `LivingEntity.getAgeScale`, kept separate from the SCALE attribute multiplier. Adults,
/// non-living entities, and unknown ids use `1`; baby values come from the generated nearest
/// override (including the non-default Armadillo/Camel/Dolphin/Goat/HappyGhast/Turtle values).
pub(crate) fn vanilla_age_scale(entity_type_id: i32, data_values: &[EntityDataValue]) -> f32 {
    let Some((_, baby_scale)) = vanilla_entity_metadata_baby_behavior_and_age_scale(entity_type_id)
    else {
        return 1.0;
    };
    if vanilla_is_baby(entity_type_id, data_values) {
        baby_scale
    } else {
        1.0
    }
}

fn baby_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityPickBoundsState> {
    if !vanilla_is_baby(entity_type_id, data_values) {
        return None;
    }

    Some(match entity_type_id {
        VANILLA_ENTITY_TYPE_CHICKEN_ID => EntityPickBoundsState::from_base_size(0.3, 0.4, 0.0),
        VANILLA_ENTITY_TYPE_VILLAGER_ID => EntityPickBoundsState::from_base_size(0.49, 0.99, 0.0),
        VANILLA_ENTITY_TYPE_DROWNED_ID
        | VANILLA_ENTITY_TYPE_HUSK_ID
        | VANILLA_ENTITY_TYPE_PIGLIN_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
        | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => {
            EntityPickBoundsState::from_base_size(0.49, 0.99, 0.0)
        }
        VANILLA_ENTITY_TYPE_ARMADILLO_ID => {
            vanilla_pick_bounds_for_type(entity_type_id)?.scale_dimensions(ARMADILLO_BABY_SCALE)
        }
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => {
            vanilla_pick_bounds_for_type(entity_type_id)?.scale_dimensions(DOLPHIN_BABY_SCALE)
        }
        VANILLA_ENTITY_TYPE_FOX_ID => {
            vanilla_pick_bounds_for_type(entity_type_id)?.scale_dimensions(FOX_BABY_SCALE)
        }
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => {
            vanilla_pick_bounds_for_type(entity_type_id)?.scale_dimensions(HAPPY_GHAST_BABY_SCALE)
        }
        VANILLA_ENTITY_TYPE_HORSE_ID
        | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => {
            vanilla_pick_bounds_for_type(entity_type_id)?.scale_dimensions(HORSE_BABY_SCALE)
        }
        VANILLA_ENTITY_TYPE_RABBIT_ID => EntityPickBoundsState::from_base_size(0.24, 0.4, 0.0),
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => EntityPickBoundsState::from_base_size(0.5, 0.25, 0.0),
        _ if vanilla_entity_type_is_squid(entity_type_id) => {
            EntityPickBoundsState::from_base_size(0.5, 0.63, 0.0)
        }
        VANILLA_ENTITY_TYPE_TURTLE_ID => {
            vanilla_pick_bounds_for_type(entity_type_id)?.scale_dimensions(0.3)
        }
        _ => vanilla_pick_bounds_for_type(entity_type_id)?
            .scale_dimensions(DEFAULT_AGEABLE_BABY_SCALE),
    })
}

/// Vanilla `AreaEffectCloud.getDimensions` is `EntityDimensions.scalable(getRadius() * 2.0F,
/// 0.5F)`, refreshed client-side whenever the synced `DATA_RADIUS` value changes. The cloud is
/// not pickable, so these bounds only reach the debug-hitbox and model-source projections.
fn area_effect_cloud_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityPickBoundsState> {
    let (data_id, default) =
        vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type(entity_type_id)?;
    let radius = entity_data_float(data_values, data_id, default);
    Some(EntityPickBoundsState::from_base_size(
        radius * 2.0,
        0.5,
        0.0,
    ))
}

fn slime_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityPickBoundsState> {
    let (data_id, default) =
        vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id)?;
    let size = entity_data_int(data_values, data_id, default) as f32;
    Some(EntityPickBoundsState::from_base_size(
        SLIME_BASE_SIZE * size,
        SLIME_BASE_SIZE * size,
        0.0,
    ))
}

fn pufferfish_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityPickBoundsState> {
    let (data_id, default) =
        vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(entity_type_id)?;
    let puff_state = entity_data_int(data_values, data_id, default);
    let scale = match puff_state {
        0 => 0.5,
        1 => 0.7,
        _ => 1.0,
    };
    Some(EntityPickBoundsState::from_base_size(0.7, 0.7, 0.0).scale_dimensions(scale))
}

fn salmon_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityPickBoundsState> {
    let (data_id, default) =
        vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(entity_type_id)?;
    let variant = entity_data_int(data_values, data_id, default);
    let scale = match variant {
        i32::MIN..=0 => 0.5,
        1 => 1.0,
        _ => 1.5,
    };
    Some(EntityPickBoundsState::from_base_size(0.7, 0.4, 0.0).scale_dimensions(scale))
}

fn phantom_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityPickBoundsState> {
    let (data_id, default) =
        vanilla_entity_metadata_phantom_size_accessor_for_entity_type(entity_type_id)?;
    let size = entity_data_int(data_values, data_id, default) as f32;
    Some(EntityPickBoundsState::from_base_size(0.9, 0.5, 0.0).scale_dimensions(1.0 + 0.15 * size))
}

fn wind_charge_pick_bounds(entity_type_id: i32) -> EntityPickBoundsState {
    let half_width = WIND_CHARGE_BOUNDS_SIZE / 2.0;
    EntityPickBoundsState {
        min: [-half_width, WIND_CHARGE_BOUNDS_Y_OFFSET, -half_width],
        max: [
            half_width,
            WIND_CHARGE_BOUNDS_Y_OFFSET + WIND_CHARGE_BOUNDS_SIZE,
            half_width,
        ],
        pick_radius: if vanilla_entity_type_is_redirectable_projectile(entity_type_id) {
            1.0
        } else {
            0.0
        },
    }
}

fn armor_stand_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityPickBoundsState> {
    let (accessors, flags) = armor_stand_client_flags(entity_type_id, data_values)?;
    if flags & accessors.marker_mask != 0 {
        return None;
    }
    if entity_data_pose(data_values) == VANILLA_ENTITY_POSE_SLEEPING_ID {
        return Some(living_sleeping_pick_bounds());
    }
    let scale = if flags & accessors.small_mask != 0 {
        ARMOR_STAND_SMALL_SCALE
    } else {
        1.0
    };
    Some(EntityPickBoundsState::from_base_size(
        ARMOR_STAND_WIDTH * scale,
        ARMOR_STAND_HEIGHT * scale,
        0.0,
    ))
}

fn armor_stand_client_flags(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<(VanillaArmorStandMetadataAccessors, i8)> {
    let accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(entity_type_id)?;
    Some((
        accessors,
        entity_data_byte(
            data_values,
            accessors.client_flags_data_id,
            accessors.client_flags_default,
        ),
    ))
}

fn armor_stand_is_marker(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    armor_stand_client_flags(entity_type_id, data_values)
        .is_some_and(|(accessors, flags)| flags & accessors.marker_mask != 0)
}

fn apply_living_scale(
    entity_type_id: i32,
    bounds: EntityPickBoundsState,
    attributes: &[AttributeSnapshot],
    scale_dimensions: bool,
) -> EntityPickBoundsState {
    if !scale_dimensions {
        return bounds;
    }
    if !vanilla_living_entity_type(entity_type_id) {
        return bounds;
    }
    let Some(scale) = vanilla_scale_attribute(attributes) else {
        return bounds;
    };
    bounds.scale_dimensions(entity_scale(entity_type_id, scale))
}

fn vanilla_scale_attribute(attributes: &[AttributeSnapshot]) -> Option<f32> {
    attributes
        .iter()
        .find(|attribute| attribute.attribute_id == VANILLA_ATTRIBUTE_SCALE_ID)
        .map(vanilla_attribute_value)
}

/// Vanilla `LivingEntity.getScale` used as `LivingEntityRenderState.scale`: the
/// `SCALE` attribute value (clamped to `[0.0625, 16.0]`) passed through the
/// generated per-entity `sanitizeScale` override behavior. Defaults to `1.0` when no
/// `SCALE` attribute is synced. `LivingEntityRenderer.submit` applies it as a
/// uniform `poseStack.scale`.
pub(crate) fn vanilla_render_scale(entity_type_id: i32, attributes: &[AttributeSnapshot]) -> f32 {
    let scale = vanilla_scale_attribute(attributes).unwrap_or(1.0);
    entity_scale(entity_type_id, scale)
}

fn vanilla_attribute_value(attribute: &AttributeSnapshot) -> f32 {
    let mut base = attribute.base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id != VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID
            && modifier.operation_id != VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID
        {
            base += modifier.amount;
        }
    }

    let mut result = base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID {
            result += base * modifier.amount;
        }
    }
    for modifier in &attribute.modifiers {
        if modifier.operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID {
            result *= 1.0 + modifier.amount;
        }
    }

    sanitize_vanilla_scale(result) as f32
}

fn entity_scale(entity_type_id: i32, scale: f32) -> f32 {
    match vanilla_entity_scale_sanitizer(entity_type_id) {
        Some(VanillaEntityScaleSanitizer::ClampMax(maximum)) => scale.min(maximum),
        Some(VanillaEntityScaleSanitizer::Fixed(value)) => value,
        None => scale,
    }
}

fn sanitize_vanilla_scale(value: f64) -> f64 {
    if value.is_nan() {
        VANILLA_SCALE_MIN
    } else {
        value.clamp(VANILLA_SCALE_MIN, VANILLA_SCALE_MAX)
    }
}

/// Whether the entity uses the shared living-entity metadata and presentation path.
///
/// The generated class catalog proves that the ender dragon inherits `LivingEntity`, but vanilla
/// gives it a dedicated renderer/lifecycle path rather than the generic 20-tick living death flip.
pub(crate) fn vanilla_living_entity_type(entity_type_id: i32) -> bool {
    entity_type_id != VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
        && vanilla_entity_metadata_type_is_living(entity_type_id)
}

/// Vanilla entity classes that inherit `Animal` and therefore use
/// `Animal.handleEntityEvent(18)` for the seven-heart love burst. This is not
/// the same as the registry's creature/water categories: Allay has a separate
/// 3-heart event-18 branch, while Dolphin/fish/squid are not `Animal`.
pub(crate) fn vanilla_animal_entity_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_animal(entity_type_id)
}

/// Whether the entity renders through vanilla `CatRenderer` rather than `OcelotRenderer`. Only cats
/// project `TamableAnimal.isInSittingPose()` into `FelineRenderState.isSitting`.
pub(crate) fn vanilla_is_cat(entity_type_id: i32) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_CAT_ID
}

fn scales_with_living_scale_attribute(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    vanilla_living_entity_type(entity_type_id)
        && !(is_living_sleeping(entity_type_id, data_values)
            || is_avatar_dying_pose(entity_type_id, data_values))
        && !is_warden_fixed_pose(entity_type_id, data_values)
}

fn is_living_sleeping(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_living_entity_type(entity_type_id)
        && entity_data_pose(data_values) == VANILLA_ENTITY_POSE_SLEEPING_ID
}

fn is_avatar_dying_pose(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    (entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_MANNEQUIN_ID)
        && entity_data_pose(data_values) == VANILLA_ENTITY_POSE_DYING_ID
}

fn is_warden_fixed_pose(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID
        && matches!(
            entity_data_pose(data_values),
            VANILLA_ENTITY_POSE_EMERGING_ID | VANILLA_ENTITY_POSE_DIGGING_ID
        )
}

pub(crate) fn entity_data_pose(data_values: &[EntityDataValue]) -> i32 {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    data_values
        .iter()
        .find(|value| value.data_id == accessors.pose_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Pose(value) => Some(*value),
            _ => None,
        })
        .filter(|value| (0..=17).contains(value))
        .unwrap_or(accessors.pose_default)
}

fn entity_data_int(data_values: &[EntityDataValue], data_id: u8, fallback: i32) -> i32 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Int(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn entity_data_long(data_values: &[EntityDataValue], data_id: u8, fallback: i64) -> i64 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Long(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn entity_data_byte(data_values: &[EntityDataValue], data_id: u8, fallback: i8) -> i8 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Byte(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn entity_data_direction(data_values: &[EntityDataValue], data_id: u8) -> Option<VanillaDirection> {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Direction(value) => Some(vanilla_direction_from_3d_data(*value)),
            _ => None,
        })
}

fn entity_data_bool(data_values: &[EntityDataValue], data_id: u8, fallback: bool) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn entity_data_float(data_values: &[EntityDataValue], data_id: u8, fallback: f32) -> f32 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Float(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn entity_data_enum_id(
    data_values: &[EntityDataValue],
    data_id: u8,
    serializer: EntityDataEnumSerializer,
    fallback: i32,
) -> i32 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::EnumId {
                serializer: value_serializer,
                id,
            } if *value_serializer == serializer => Some(*id),
            _ => None,
        })
        .unwrap_or(fallback)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VanillaAxis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VanillaDirection {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl VanillaDirection {
    fn axis(self) -> VanillaAxis {
        match self {
            Self::Down | Self::Up => VanillaAxis::Y,
            Self::North | Self::South => VanillaAxis::Z,
            Self::West | Self::East => VanillaAxis::X,
        }
    }

    fn step(self) -> [i32; 3] {
        match self {
            Self::Down => [0, -1, 0],
            Self::Up => [0, 1, 0],
            Self::North => [0, 0, -1],
            Self::South => [0, 0, 1],
            Self::West => [-1, 0, 0],
            Self::East => [1, 0, 0],
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }
}

fn vanilla_direction_from_3d_data(data: i32) -> VanillaDirection {
    match data.rem_euclid(6) {
        0 => VanillaDirection::Down,
        1 => VanillaDirection::Up,
        2 => VanillaDirection::North,
        3 => VanillaDirection::South,
        4 => VanillaDirection::West,
        _ => VanillaDirection::East,
    }
}
