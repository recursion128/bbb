use bbb_protocol::packets::{
    AttributeSnapshot, EntityDataEnumSerializer, EntityDataValue, EntityDataValueKind,
};
use serde::{Deserialize, Serialize};

use super::{
    is_vanilla_boat_type, is_vanilla_minecart_type, EntityClientAnimationState, EntityVec3,
};
use bbb_protocol::entity_types::*;

mod block_attached;

pub(crate) use block_attached::{
    item_frame_facing, item_frame_item, item_frame_map_id, item_frame_rotation,
};

pub(crate) const ITEM_FRAME_ENTITY_TYPE_IDS: [i32; 2] = [
    VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
];

pub(crate) const ENTITY_DATA_POSE_ID: u8 = 6;
const AGEABLE_MOB_BABY_DATA_ID: u8 = 16;
const PIGLIN_BABY_DATA_ID: u8 = 17;
const ZOMBIE_BABY_DATA_ID: u8 = 16;
const INTERACTION_DATA_WIDTH_ID: u8 = 8;
const INTERACTION_DATA_HEIGHT_ID: u8 = 9;
const INTERACTION_DEFAULT_WIDTH: f32 = 1.0;
const INTERACTION_DEFAULT_HEIGHT: f32 = 1.0;
const SLIME_SIZE_DATA_ID: u8 = 16;
const SLIME_BASE_SIZE: f32 = 0.52;
const SLIME_DEFAULT_SIZE: i32 = 1;
// `ArmorStand extends LivingEntity` (not a `Mob`): `DATA_CLIENT_FLAGS` is the first accessor after
// `LivingEntity` (0-14) with no `Mob.DATA_MOB_FLAGS_ID` (15) in between, so it lands at 15.
const ARMOR_STAND_CLIENT_FLAGS_DATA_ID: u8 = 15;
const ARMOR_STAND_CLIENT_FLAG_SMALL: i8 = 1;
const ARMOR_STAND_CLIENT_FLAG_MARKER: i8 = 16;
const ARMOR_STAND_WIDTH: f32 = 0.5;
const ARMOR_STAND_HEIGHT: f32 = 1.975;
const ARMOR_STAND_SMALL_SCALE: f32 = 0.5;
const ARMADILLO_BABY_SCALE: f32 = 0.6;
const CAMEL_SITTING_HEIGHT_DIFFERENCE: f32 = 1.43;
const CAMEL_BABY_SCALE: f32 = 0.6;
const CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID: u8 = 20;
const CAMEL_SITDOWN_DURATION_TICKS: i64 = 40;
const CAMEL_STANDUP_DURATION_TICKS: i64 = 52;
const DOLPHIN_BABY_SCALE: f32 = 0.65;
const FOX_BABY_SCALE: f32 = 0.6;
const GOAT_LONG_JUMPING_SCALE: f32 = 0.7;
const GOAT_BABY_SCALE: f32 = 0.55;
const HAPPY_GHAST_BABY_SCALE: f32 = 0.2375;
const HAPPY_GHAST_MAX_SCALE: f32 = 1.0;
const HORSE_BABY_SCALE: f32 = 0.7;
const SHULKER_ATTACH_FACE_DATA_ID: u8 = 16;
const SHULKER_MAX_SCALE: f32 = 3.0;
const DEFAULT_AGEABLE_BABY_SCALE: f32 = 0.5;
const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
const VANILLA_SCALE_MIN: f64 = 0.0625;
const VANILLA_SCALE_MAX: f64 = 16.0;
const VANILLA_POSE_FALL_FLYING_ID: i32 = 1;
pub(crate) const VANILLA_POSE_SLEEPING_ID: i32 = 2;
const VANILLA_POSE_SWIMMING_ID: i32 = 3;
const VANILLA_POSE_SPIN_ATTACK_ID: i32 = 4;
pub(crate) const VANILLA_POSE_CROUCHING_ID: i32 = 5;
const VANILLA_POSE_LONG_JUMPING_ID: i32 = 6;
const VANILLA_POSE_DYING_ID: i32 = 7;
const VANILLA_POSE_SITTING_ID: i32 = 10;
const VANILLA_POSE_EMERGING_ID: i32 = 13;
const VANILLA_POSE_DIGGING_ID: i32 = 14;
const SNIFFER_STATE_DATA_ID: u8 = 18;
const SNIFFER_STATE_DIGGING_ID: i32 = 5;
const SNIFFER_DIGGING_HEIGHT_OFFSET: f32 = 0.4;
const PUFFERFISH_PUFF_STATE_DATA_ID: u8 = 17;
const SALMON_VARIANT_DATA_ID: u8 = 17;
const PHANTOM_SIZE_DATA_ID: u8 = 16;
const WIND_CHARGE_BOUNDS_SIZE: f32 = 0.3125;
const WIND_CHARGE_BOUNDS_Y_OFFSET: f32 = -0.15;
const WIND_CHARGE_PICK_RADIUS: f32 = 1.0;
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
    VANILLA_ENTITY_PICK_BOUNDS
        .binary_search_by_key(&entity_type_id, |(id, _)| *id)
        .ok()
        .map(|index| VANILLA_ENTITY_PICK_BOUNDS[index].1)
}

pub(crate) fn vanilla_pick_bounds_for_entity_data(
    entity_type_id: i32,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
) -> Option<EntityPickBoundsState> {
    let scale_dimensions = scales_with_living_scale_attribute(entity_type_id, data_values);
    let bounds = if entity_type_id == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
        armor_stand_pick_bounds(data_values)?
    } else if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_MANNEQUIN_ID
    {
        avatar_pick_bounds(data_values)
    } else if is_living_sleeping(entity_type_id, data_values) {
        living_sleeping_pick_bounds()
    } else if is_warden_fixed_pose(entity_type_id, data_values) {
        EntityPickBoundsState::from_base_size(0.9, 1.0, 0.0)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
    {
        camel_pick_bounds(entity_type_id, data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_GOAT_ID {
        goat_pick_bounds(data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_SNIFFER_ID {
        sniffer_pick_bounds(data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_POLAR_BEAR_ID {
        polar_bear_pick_bounds(data_values, client_animations)?
    } else if entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_ID {
        shulker_pick_bounds(data_values, client_animations)
    } else if let Some(bounds) = baby_pick_bounds(entity_type_id, data_values) {
        bounds
    } else if entity_type_id == VANILLA_ENTITY_TYPE_INTERACTION_ID {
        interaction_pick_bounds(data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_ITEM_FRAME_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
    {
        block_attached::item_frame_pick_bounds(add_entity_data, data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID {
        block_attached::painting_pick_bounds(add_entity_data, data_values)?
    } else if entity_type_id == VANILLA_ENTITY_TYPE_LEASH_KNOT_ID {
        EntityPickBoundsState::from_base_size(0.375, 0.5, 0.0)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_SLIME_ID
    {
        slime_pick_bounds(data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_PUFFERFISH_ID {
        pufferfish_pick_bounds(data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_SALMON_ID {
        salmon_pick_bounds(data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_PHANTOM_ID {
        phantom_pick_bounds(data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_WIND_CHARGE_ID
    {
        wind_charge_pick_bounds()
    } else {
        vanilla_pick_bounds_for_type(entity_type_id)?
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
) -> Option<EntityPickBoundsState> {
    if entity_type_id == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID && armor_stand_is_marker(data_values) {
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
    vanilla_pick_bounds_for_entity_data(
        entity_type_id,
        add_entity_data,
        data_values,
        attributes,
        client_animations,
    )
}

pub(crate) fn vanilla_eye_height_for_entity_data(
    entity_type_id: i32,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
) -> Option<f32> {
    if entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
        // Vanilla 26.1 `EntityDimensions.defaultEyeHeight`: `height * 0.85F`.
        return Some(8.0 * DEFAULT_ENTITY_EYE_HEIGHT_RATIO);
    }
    let bounds = vanilla_pick_bounds_for_entity_data(
        entity_type_id,
        add_entity_data,
        data_values,
        attributes,
        client_animations,
    )?;
    let height = bounds_height(bounds);
    let base_eye_height = vanilla_eye_height_override_for_type(entity_type_id)
        .and_then(|eye_height| scaled_eye_height_for_bounds(entity_type_id, eye_height, height))
        .unwrap_or(height * DEFAULT_ENTITY_EYE_HEIGHT_RATIO);
    Some(base_eye_height)
}

pub(crate) fn vanilla_client_position_for_entity_data(
    entity_type_id: i32,
    packet_position: EntityVec3,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityVec3> {
    if entity_type_id == VANILLA_ENTITY_TYPE_ITEM_FRAME_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
    {
        return Some(block_attached::item_frame_center(
            packet_position,
            add_entity_data,
            data_values,
        ));
    }
    if entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID {
        return block_attached::painting_center(packet_position, add_entity_data, data_values);
    }
    if entity_type_id == VANILLA_ENTITY_TYPE_LEASH_KNOT_ID {
        return Some(block_attached::leash_knot_position(packet_position));
    }
    None
}

pub(crate) fn vanilla_body_anchor_y_offset_for_entity_data(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    game_time: i64,
    is_front: bool,
    partial_ticks: f32,
) -> Option<f32> {
    if entity_type_id != VANILLA_ENTITY_TYPE_CAMEL_ID
        && entity_type_id != VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
    {
        return None;
    }

    let dimensions =
        vanilla_pick_bounds_for_entity_data(entity_type_id, 0, data_values, attributes, None)?;
    let scale = camel_age_scale(entity_type_id, data_values)
        * vanilla_render_scale(entity_type_id, attributes);
    let last_pose_change_tick =
        entity_data_long(data_values, CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID, 0);
    Some(camel_body_anchor_animation_y_offset(
        dimensions,
        scale,
        last_pose_change_tick,
        game_time,
        is_front,
        partial_ticks,
    ))
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
    let passenger_index = vehicle_passengers
        .iter()
        .position(|id| *id == passenger_id)?;
    let half_width = 0.5 * bounds_width(vehicle_bounds).min(bounds_width(passenger_bounds));
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
    Some((
        EntityVec3 {
            x: vehicle_position.x + attachment[0],
            y: vehicle_position.y + attachment[1],
            z: vehicle_position.z + attachment[2],
        },
        EntityPickBoundsState::from_base_size(
            half_width * 2.0,
            ENTITY_PASSENGER_VEHICLE_BOX_HEIGHT,
            0.0,
        ),
    ))
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
    if is_vanilla_boat_type(vehicle_entity_type_id) {
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

    if vehicle_entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_ID
        || vehicle_entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
    {
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

    if is_vanilla_minecart_type(vehicle_entity_type_id)
        && (passenger_entity_type_id == VANILLA_ENTITY_TYPE_VILLAGER_ID
            || passenger_entity_type_id == VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID)
    {
        return [0.0, 0.0, 0.0];
    }

    let mut point = vanilla_default_passenger_attachment_point(
        vehicle_entity_type_id,
        vehicle_bounds,
        passenger_index,
    );

    if vehicle_entity_type_id == VANILLA_ENTITY_TYPE_SLIME_ID
        || vehicle_entity_type_id == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
    {
        point = [
            0.0,
            bounds_height(vehicle_bounds)
                - 0.015625
                    * entity_data_int(vehicle_data_values, SLIME_SIZE_DATA_ID, SLIME_DEFAULT_SIZE)
                        as f32
                    * vanilla_render_scale(vehicle_entity_type_id, vehicle_attributes) as f32,
            0.0,
        ];
    } else if is_vanilla_rearing_horse_passenger_attachment_type(vehicle_entity_type_id) {
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

    let ride_height = if is_vanilla_raft_type(vehicle_entity_type_id) {
        bounds_height(vehicle_bounds) * 0.888_888_9
    } else {
        bounds_height(vehicle_bounds) / 3.0
    };
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
        entity_data_long(vehicle_data_values, CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID, 0);
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
) -> [f32; 3] {
    let Some(points) = vanilla_passenger_attachment_overrides(entity_type_id) else {
        return [0.0, bounds_height(bounds), 0.0];
    };
    let base_bounds = vanilla_pick_bounds_for_type(entity_type_id).unwrap_or(bounds);
    let xz_scale = scale_axis(bounds_width(bounds), bounds_width(base_bounds));
    let y_scale = scale_axis(bounds_height(bounds), bounds_height(base_bounds));
    let point = points[passenger_index.min(points.len() - 1)];
    [point[0] * xz_scale, point[1] * y_scale, point[2] * xz_scale]
}

fn vanilla_passenger_attachment_overrides(entity_type_id: i32) -> Option<&'static [[f32; 3]]> {
    VANILLA_PASSENGER_ATTACHMENT_OVERRIDES
        .binary_search_by_key(&entity_type_id, |(id, _)| *id)
        .ok()
        .map(|index| VANILLA_PASSENGER_ATTACHMENT_OVERRIDES[index].1)
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
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID
            | VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID
    )
}

fn is_vanilla_raft_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID | VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID
    )
}

fn is_vanilla_rearing_horse_passenger_attachment_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_DONKEY_ID
            | VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_MULE_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
    )
}

fn lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

fn scaled_eye_height_for_bounds(
    entity_type_id: i32,
    base_eye_height: f32,
    actual_height: f32,
) -> Option<f32> {
    let base_height = vanilla_pick_bounds_for_type(entity_type_id).map(bounds_height)?;
    (base_height > 0.0).then_some(base_eye_height * actual_height / base_height)
}

fn vanilla_eye_height_override_for_type(entity_type_id: i32) -> Option<f32> {
    VANILLA_ENTITY_EYE_HEIGHT_OVERRIDES
        .binary_search_by_key(&entity_type_id, |(id, _)| *id)
        .ok()
        .map(|index| VANILLA_ENTITY_EYE_HEIGHT_OVERRIDES[index].1)
}

fn polar_bear_pick_bounds(
    data_values: &[EntityDataValue],
    client_animations: Option<EntityClientAnimationState>,
) -> Option<EntityPickBoundsState> {
    let bounds = if entity_data_bool(data_values, AGEABLE_MOB_BABY_DATA_ID, false) {
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
    data_values: &[EntityDataValue],
    client_animations: Option<EntityClientAnimationState>,
) -> EntityPickBoundsState {
    let direction = entity_data_direction(data_values, SHULKER_ATTACH_FACE_DATA_ID)
        .unwrap_or(VanillaDirection::Down)
        .opposite();
    let peek_amount = client_animations
        .and_then(|animations| animations.shulker_peek)
        .map(|peek| peek.current_peek_amount)
        .unwrap_or(0.0);
    let physical_peek = 0.5 - ((0.5 + peek_amount) * std::f32::consts::PI).sin() * 0.5;
    shulker_progress_pick_bounds(direction, physical_peek)
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

fn interaction_pick_bounds(data_values: &[EntityDataValue]) -> EntityPickBoundsState {
    EntityPickBoundsState::from_base_size(
        entity_data_float(
            data_values,
            INTERACTION_DATA_WIDTH_ID,
            INTERACTION_DEFAULT_WIDTH,
        ),
        entity_data_float(
            data_values,
            INTERACTION_DATA_HEIGHT_ID,
            INTERACTION_DEFAULT_HEIGHT,
        ),
        0.0,
    )
}

fn avatar_pick_bounds(data_values: &[EntityDataValue]) -> EntityPickBoundsState {
    match entity_data_pose(data_values) {
        VANILLA_POSE_SLEEPING_ID | VANILLA_POSE_DYING_ID => living_sleeping_pick_bounds(),
        VANILLA_POSE_FALL_FLYING_ID | VANILLA_POSE_SWIMMING_ID | VANILLA_POSE_SPIN_ATTACK_ID => {
            EntityPickBoundsState::from_base_size(0.6, 0.6, 0.0)
        }
        VANILLA_POSE_CROUCHING_ID => EntityPickBoundsState::from_base_size(0.6, 1.5, 0.0),
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
    let height = if entity_data_pose(data_values) == VANILLA_POSE_SITTING_ID {
        2.375 - CAMEL_SITTING_HEIGHT_DIFFERENCE
    } else {
        2.375
    };
    let bounds = EntityPickBoundsState::from_base_size(1.7, height, 0.0);

    if entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_ID
        && entity_data_bool(data_values, AGEABLE_MOB_BABY_DATA_ID, false)
    {
        bounds.scale_dimensions(CAMEL_BABY_SCALE)
    } else {
        bounds
    }
}

fn camel_age_scale(entity_type_id: i32, data_values: &[EntityDataValue]) -> f32 {
    if entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_ID
        && entity_data_bool(data_values, AGEABLE_MOB_BABY_DATA_ID, false)
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

fn goat_pick_bounds(data_values: &[EntityDataValue]) -> EntityPickBoundsState {
    let bounds = if entity_data_pose(data_values) == VANILLA_POSE_LONG_JUMPING_ID {
        EntityPickBoundsState::from_base_size(0.9, 1.3, 0.0)
            .scale_dimensions(GOAT_LONG_JUMPING_SCALE)
    } else {
        EntityPickBoundsState::from_base_size(0.9, 1.3, 0.0)
    };

    if entity_data_bool(data_values, AGEABLE_MOB_BABY_DATA_ID, false) {
        bounds.scale_dimensions(GOAT_BABY_SCALE)
    } else {
        bounds
    }
}

fn sniffer_pick_bounds(data_values: &[EntityDataValue]) -> EntityPickBoundsState {
    let height = if entity_data_enum_id(
        data_values,
        SNIFFER_STATE_DATA_ID,
        EntityDataEnumSerializer::SnifferState,
        0,
    ) == SNIFFER_STATE_DIGGING_ID
    {
        1.75 - SNIFFER_DIGGING_HEIGHT_OFFSET
    } else {
        1.75
    };
    let bounds = EntityPickBoundsState::from_base_size(1.9, height, 0.0);

    if entity_data_bool(data_values, AGEABLE_MOB_BABY_DATA_ID, false) {
        bounds.scale_dimensions(DEFAULT_AGEABLE_BABY_SCALE)
    } else {
        bounds
    }
}

/// Vanilla `Mob.isBaby` (`AgeableMob.DATA_BABY_ID`, `Zombie.DATA_BABY_ID`,
/// `AbstractPiglin.DATA_BABY_ID`): reads the synced baby flag at the per-family
/// metadata id. Zombies/piglins keep the flag at their own ids; every other
/// ageable mob uses the shared `AgeableMob` id. Non-ageable entities are never
/// babies.
pub(crate) fn vanilla_is_baby(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_DROWNED_ID
        | VANILLA_ENTITY_TYPE_HUSK_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
        | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => {
            entity_data_bool(data_values, ZOMBIE_BABY_DATA_ID, false)
        }
        VANILLA_ENTITY_TYPE_PIGLIN_ID => entity_data_bool(data_values, PIGLIN_BABY_DATA_ID, false),
        VANILLA_ENTITY_TYPE_ARMADILLO_ID
        | VANILLA_ENTITY_TYPE_AXOLOTL_ID
        | VANILLA_ENTITY_TYPE_BEE_ID
        | VANILLA_ENTITY_TYPE_CAT_ID
        | VANILLA_ENTITY_TYPE_CHICKEN_ID
        | VANILLA_ENTITY_TYPE_COW_ID
        | VANILLA_ENTITY_TYPE_DOLPHIN_ID
        | VANILLA_ENTITY_TYPE_DONKEY_ID
        | VANILLA_ENTITY_TYPE_FOX_ID
        | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
        | VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        | VANILLA_ENTITY_TYPE_HOGLIN_ID
        | VANILLA_ENTITY_TYPE_HORSE_ID
        | VANILLA_ENTITY_TYPE_LLAMA_ID
        | VANILLA_ENTITY_TYPE_MOOSHROOM_ID
        | VANILLA_ENTITY_TYPE_MULE_ID
        | VANILLA_ENTITY_TYPE_NAUTILUS_ID
        | VANILLA_ENTITY_TYPE_OCELOT_ID
        | VANILLA_ENTITY_TYPE_PANDA_ID
        | VANILLA_ENTITY_TYPE_PIG_ID
        | VANILLA_ENTITY_TYPE_POLAR_BEAR_ID
        | VANILLA_ENTITY_TYPE_RABBIT_ID
        | VANILLA_ENTITY_TYPE_SHEEP_ID
        | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
        | VANILLA_ENTITY_TYPE_SQUID_ID
        | VANILLA_ENTITY_TYPE_STRIDER_ID
        | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
        | VANILLA_ENTITY_TYPE_TURTLE_ID
        | VANILLA_ENTITY_TYPE_VILLAGER_ID
        | VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
        | VANILLA_ENTITY_TYPE_WOLF_ID
        | VANILLA_ENTITY_TYPE_ZOGLIN_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => {
            entity_data_bool(data_values, AGEABLE_MOB_BABY_DATA_ID, false)
        }
        _ => false,
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
        VANILLA_ENTITY_TYPE_GLOW_SQUID_ID | VANILLA_ENTITY_TYPE_SQUID_ID => {
            EntityPickBoundsState::from_base_size(0.5, 0.63, 0.0)
        }
        VANILLA_ENTITY_TYPE_TURTLE_ID => {
            vanilla_pick_bounds_for_type(entity_type_id)?.scale_dimensions(0.3)
        }
        _ => vanilla_pick_bounds_for_type(entity_type_id)?
            .scale_dimensions(DEFAULT_AGEABLE_BABY_SCALE),
    })
}

fn slime_pick_bounds(data_values: &[EntityDataValue]) -> EntityPickBoundsState {
    let size = entity_data_int(data_values, SLIME_SIZE_DATA_ID, SLIME_DEFAULT_SIZE) as f32;
    EntityPickBoundsState::from_base_size(SLIME_BASE_SIZE * size, SLIME_BASE_SIZE * size, 0.0)
}

fn pufferfish_pick_bounds(data_values: &[EntityDataValue]) -> EntityPickBoundsState {
    let puff_state = entity_data_int(data_values, PUFFERFISH_PUFF_STATE_DATA_ID, 0);
    let scale = match puff_state {
        0 => 0.5,
        1 => 0.7,
        _ => 1.0,
    };
    EntityPickBoundsState::from_base_size(0.7, 0.7, 0.0).scale_dimensions(scale)
}

fn salmon_pick_bounds(data_values: &[EntityDataValue]) -> EntityPickBoundsState {
    let variant = entity_data_int(data_values, SALMON_VARIANT_DATA_ID, 1);
    let scale = match variant {
        i32::MIN..=0 => 0.5,
        1 => 1.0,
        _ => 1.5,
    };
    EntityPickBoundsState::from_base_size(0.7, 0.4, 0.0).scale_dimensions(scale)
}

fn phantom_pick_bounds(data_values: &[EntityDataValue]) -> EntityPickBoundsState {
    let size = entity_data_int(data_values, PHANTOM_SIZE_DATA_ID, 0) as f32;
    EntityPickBoundsState::from_base_size(0.9, 0.5, 0.0).scale_dimensions(1.0 + 0.15 * size)
}

fn wind_charge_pick_bounds() -> EntityPickBoundsState {
    let half_width = WIND_CHARGE_BOUNDS_SIZE / 2.0;
    EntityPickBoundsState {
        min: [-half_width, WIND_CHARGE_BOUNDS_Y_OFFSET, -half_width],
        max: [
            half_width,
            WIND_CHARGE_BOUNDS_Y_OFFSET + WIND_CHARGE_BOUNDS_SIZE,
            half_width,
        ],
        pick_radius: WIND_CHARGE_PICK_RADIUS,
    }
}

fn armor_stand_pick_bounds(data_values: &[EntityDataValue]) -> Option<EntityPickBoundsState> {
    let flags = armor_stand_client_flags(data_values);
    if armor_stand_is_marker(data_values) {
        return None;
    }
    if entity_data_pose(data_values) == VANILLA_POSE_SLEEPING_ID {
        return Some(living_sleeping_pick_bounds());
    }
    let scale = if flags & ARMOR_STAND_CLIENT_FLAG_SMALL != 0 {
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

fn armor_stand_client_flags(data_values: &[EntityDataValue]) -> i8 {
    entity_data_byte(data_values, ARMOR_STAND_CLIENT_FLAGS_DATA_ID, 0)
}

fn armor_stand_is_marker(data_values: &[EntityDataValue]) -> bool {
    armor_stand_client_flags(data_values) & ARMOR_STAND_CLIENT_FLAG_MARKER != 0
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
/// per-entity `sanitizeScale` overrides (`HappyGhast` ≤ 1.0, `Shulker` ≤ 3.0,
/// captured by [`entity_scale`]). Defaults to `1.0` when no `SCALE` attribute is
/// synced. `LivingEntityRenderer.submit` applies it as a uniform `poseStack.scale`.
pub(crate) fn vanilla_render_scale(entity_type_id: i32, attributes: &[AttributeSnapshot]) -> f32 {
    let scale = vanilla_scale_attribute(attributes).unwrap_or(1.0);
    entity_scale(entity_type_id, scale)
}

fn vanilla_attribute_value(attribute: &AttributeSnapshot) -> f32 {
    let mut base = attribute.base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id != 1 && modifier.operation_id != 2 {
            base += modifier.amount;
        }
    }

    let mut result = base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id == 1 {
            result += base * modifier.amount;
        }
    }
    for modifier in &attribute.modifiers {
        if modifier.operation_id == 2 {
            result *= 1.0 + modifier.amount;
        }
    }

    sanitize_vanilla_scale(result) as f32
}

fn entity_scale(entity_type_id: i32, scale: f32) -> f32 {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => scale.min(HAPPY_GHAST_MAX_SCALE),
        VANILLA_ENTITY_TYPE_SHULKER_ID => scale.min(SHULKER_MAX_SCALE),
        _ => scale,
    }
}

fn sanitize_vanilla_scale(value: f64) -> f64 {
    if value.is_nan() {
        VANILLA_SCALE_MIN
    } else {
        value.clamp(VANILLA_SCALE_MIN, VANILLA_SCALE_MAX)
    }
}

pub(crate) fn vanilla_living_entity_type(entity_type_id: i32) -> bool {
    VANILLA_LIVING_ENTITY_TYPE_IDS
        .binary_search(&entity_type_id)
        .is_ok()
}

/// Vanilla entity classes that inherit `Animal` and therefore use
/// `Animal.handleEntityEvent(18)` for the seven-heart love burst. This is not
/// the same as the registry's creature/water categories: Allay has a separate
/// 3-heart event-18 branch, while Dolphin/fish/squid are not `Animal`.
pub(crate) fn vanilla_animal_entity_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_ARMADILLO_ID
            | VANILLA_ENTITY_TYPE_AXOLOTL_ID
            | VANILLA_ENTITY_TYPE_BEE_ID
            | VANILLA_ENTITY_TYPE_CAMEL_ID
            | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
            | VANILLA_ENTITY_TYPE_CAT_ID
            | VANILLA_ENTITY_TYPE_CHICKEN_ID
            | VANILLA_ENTITY_TYPE_COW_ID
            | VANILLA_ENTITY_TYPE_DONKEY_ID
            | VANILLA_ENTITY_TYPE_FOX_ID
            | VANILLA_ENTITY_TYPE_FROG_ID
            | VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            | VANILLA_ENTITY_TYPE_GOAT_ID
            | VANILLA_ENTITY_TYPE_HOGLIN_ID
            | VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_LLAMA_ID
            | VANILLA_ENTITY_TYPE_MOOSHROOM_ID
            | VANILLA_ENTITY_TYPE_MULE_ID
            | VANILLA_ENTITY_TYPE_NAUTILUS_ID
            | VANILLA_ENTITY_TYPE_OCELOT_ID
            | VANILLA_ENTITY_TYPE_PANDA_ID
            | VANILLA_ENTITY_TYPE_PARROT_ID
            | VANILLA_ENTITY_TYPE_PIG_ID
            | VANILLA_ENTITY_TYPE_POLAR_BEAR_ID
            | VANILLA_ENTITY_TYPE_RABBIT_ID
            | VANILLA_ENTITY_TYPE_SHEEP_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_SNIFFER_ID
            | VANILLA_ENTITY_TYPE_STRIDER_ID
            | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            | VANILLA_ENTITY_TYPE_TURTLE_ID
            | VANILLA_ENTITY_TYPE_WOLF_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
    )
}

/// Entities rendered with the vanilla `ZombieModel` / `GiantZombieModel`, whose
/// `setupAnim` overrides the arms with `AnimationUtils.animateZombieArms` (the held-out
/// pose, whose `armDrop` deepens for an aggressive mob). Together with
/// [`vanilla_piglin_melee_attack_family`] these are the consumers of the synced `Mob`
/// aggressive flag, so the `is_aggressive` projection is gated to them — every one is a
/// `Mob` carrying the `DATA_MOB_FLAGS_ID` byte.
pub(crate) fn vanilla_zombie_model_family(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_DROWNED_ID
            | VANILLA_ENTITY_TYPE_HUSK_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
            | VANILLA_ENTITY_TYPE_GIANT_ID
    )
}

/// The regular piglin and the piglin brute, whose vanilla `getArmPose` returns
/// `ATTACKING_WITH_MELEE_WEAPON` when `isAggressive() && isHoldingMeleeWeapon()` —
/// `PiglinModel` then raises (`holdWeaponHigh`) and swings (`swingWeaponDown`) the weapon.
/// Both consume the synced `Mob` aggressive flag, so the `is_aggressive` projection covers
/// them. The zombified piglin is excluded: it is a `Zombie` whose `animateZombieArms` pose
/// is deferred (covered by [`vanilla_zombie_model_family`] semantics, not the piglin pose).
pub(crate) fn vanilla_piglin_melee_attack_family(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_PIGLIN_ID | VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
    )
}

/// The illagers whose vanilla `getArmPose` branches on `isAggressive()`, and whose bbb arm pose is
/// resolved from it: the vindicator (`isAggressive → ATTACKING`, the `swingWeaponDown` axe), the
/// illusioner (`!casting && isAggressive → BOW_AND_ARROW`, the bow aim), and the pillager
/// (`!isChargingCrossbow && !isHolding(CROSSBOW) && isAggressive → ATTACKING`). They consume the synced
/// `Mob` aggressive flag, so the `is_aggressive` projection covers them. The evoker never reads
/// `isAggressive` (`getArmPose` is `SPELLCASTING > CELEBRATING > CROSSED`), so it remains gated out.
pub(crate) fn vanilla_illager_aggressive_arm_pose_family(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_VINDICATOR_ID
            | VANILLA_ENTITY_TYPE_ILLUSIONER_ID
            | VANILLA_ENTITY_TYPE_PILLAGER_ID
    )
}

/// Whether the entity is rendered with the vanilla `BatModel`. Its `setupAnim` swaps to
/// the `BatAnimation.BAT_RESTING` hanging pose (and applies a head look) while
/// `Bat.isResting` (the synced `DATA_ID_FLAGS & 1`) is set, so the resting projection is
/// gated to this one type — only the bat defines that flags byte.
pub(crate) fn vanilla_is_bat(entity_type_id: i32) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_BAT_ID
}

/// Whether the entity is rendered with the vanilla `BeeModel`. Its `setupAnim` hides the
/// stinger cube once the bee has stung (`hasStinger = !Bee.hasStung()`, the synced
/// `DATA_FLAGS_ID & 4`), so the stinger projection is gated to this one type.
pub(crate) fn vanilla_is_bee(entity_type_id: i32) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_BEE_ID
}

/// Whether the entity is rendered with the vanilla `FoxModel`. Its `setupAnim` reads the
/// fox's own `DATA_FLAGS_ID` (19) crouch/sleep/sit/pounce/faceplant bits and the two eased
/// `interestedAngle`/`crouchAmount` accumulators, so those projections are gated to this one
/// type — only the fox defines that flags byte.
pub(crate) fn vanilla_is_fox(entity_type_id: i32) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_FOX_ID
}

/// Whether the entity renders through vanilla `AbstractFelineModel`: cat and ocelot both feed
/// `FelineRenderState.isCrouching` / `isSprinting` into the same pose branches.
pub(crate) fn vanilla_feline_family(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAT_ID | VANILLA_ENTITY_TYPE_OCELOT_ID
    )
}

/// Whether the entity renders through vanilla `CatRenderer` rather than `OcelotRenderer`. Only cats
/// project `TamableAnimal.isInSittingPose()` into `FelineRenderState.isSitting`.
pub(crate) fn vanilla_is_cat(entity_type_id: i32) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_CAT_ID
}

/// Whether the entity is rendered with the vanilla `EndermanModel`. Its `setupAnim`
/// poses the arms forward to hold a block (`!carriedBlock.isEmpty()`) and drops the
/// head/raises the hat when staring at a player (`isCreepy`), so the carried-block and
/// creepy projections are gated to this one type — the synced `DATA_CARRY_STATE` /
/// `DATA_CREEPY` accessors only exist on the enderman.
pub(crate) fn vanilla_is_enderman(entity_type_id: i32) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_ENDERMAN_ID
}

/// Whether the entity is rendered with the vanilla `VexModel`. Its `setupAnim` levels the
/// body (`xRot = 0`) and raises both arms into the charging pose (`setArmsCharging`) while
/// `Vex.isCharging` (the synced `DATA_FLAGS_ID & 1`), so the charging projection is gated to
/// this one type — only the vex defines that flags byte.
pub(crate) fn vanilla_is_vex(entity_type_id: i32) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_VEX_ID
}

/// Whether the entity is the wither boss, rendered with the vanilla `WitherBossRenderer`. Only the
/// wither carries the `DATA_ID_INV` invulnerable-ticks accessor (driving the spawn-charge shrink and
/// the `wither_invulnerable.png` texture flicker), so that projection is gated to this one type.
pub(crate) fn vanilla_is_wither(entity_type_id: i32) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_WITHER_ID
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
        && entity_data_pose(data_values) == VANILLA_POSE_SLEEPING_ID
}

fn is_avatar_dying_pose(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    (entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_MANNEQUIN_ID)
        && entity_data_pose(data_values) == VANILLA_POSE_DYING_ID
}

fn is_warden_fixed_pose(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID
        && matches!(
            entity_data_pose(data_values),
            VANILLA_POSE_EMERGING_ID | VANILLA_POSE_DIGGING_ID
        )
}

pub(crate) fn entity_data_pose(data_values: &[EntityDataValue]) -> i32 {
    data_values
        .iter()
        .find(|value| value.data_id == ENTITY_DATA_POSE_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Pose(value) => Some(*value),
            _ => None,
        })
        .filter(|value| (0..=17).contains(value))
        .unwrap_or(0)
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

const fn pick(width: f32, height: f32, pick_radius: f32) -> EntityPickBoundsState {
    EntityPickBoundsState::from_base_size(width, height, pick_radius)
}

// IDs and points follow vanilla 26.1 `EntityType.Builder.passengerAttachments(...)`.
const VANILLA_PASSENGER_ATTACHMENT_OVERRIDES: &[(i32, &[[f32; 3]])] = &[
    (VANILLA_ENTITY_TYPE_CAT_ID, &[[0.0, 0.5125, 0.0]]),
    (VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, &[[0.0, 0.1875, 0.0]]),
    (VANILLA_ENTITY_TYPE_CHICKEN_ID, &[[0.0, 0.7, -0.1]]),
    (
        VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        &[[0.0, 0.1875, 0.0]],
    ),
    (VANILLA_ENTITY_TYPE_COW_ID, &[[0.0, 1.36875, 0.0]]),
    (VANILLA_ENTITY_TYPE_DONKEY_ID, &[[0.0, 1.1125, 0.0]]),
    (VANILLA_ENTITY_TYPE_DROWNED_ID, &[[0.0, 2.0125, 0.0]]),
    (
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID,
        &[[0.0, 2.350625, 0.0]],
    ),
    (VANILLA_ENTITY_TYPE_ENDERMAN_ID, &[[0.0, 2.80625, 0.0]]),
    (VANILLA_ENTITY_TYPE_ENDERMITE_ID, &[[0.0, 0.2375, 0.0]]),
    (VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, &[[0.0, 3.0, 0.0]]),
    (VANILLA_ENTITY_TYPE_EVOKER_ID, &[[0.0, 2.0, 0.0]]),
    (VANILLA_ENTITY_TYPE_FOX_ID, &[[0.0, 0.6375, -0.25]]),
    (VANILLA_ENTITY_TYPE_FROG_ID, &[[0.0, 0.375, -0.25]]),
    (
        VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
        &[[0.0, 0.1875, 0.0]],
    ),
    (VANILLA_ENTITY_TYPE_GHAST_ID, &[[0.0, 4.0625, 0.0]]),
    (
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        &[
            [0.0, 4.0, 1.7],
            [-1.7, 4.0, 0.0],
            [0.0, 4.0, -1.7],
            [1.7, 4.0, 0.0],
        ],
    ),
    (VANILLA_ENTITY_TYPE_GOAT_ID, &[[0.0, 1.1125, 0.0]]),
    (VANILLA_ENTITY_TYPE_GUARDIAN_ID, &[[0.0, 0.975, 0.0]]),
    (VANILLA_ENTITY_TYPE_HOGLIN_ID, &[[0.0, 1.49375, 0.0]]),
    (
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
        &[[0.0, 0.1875, 0.0]],
    ),
    (VANILLA_ENTITY_TYPE_HORSE_ID, &[[0.0, 1.44375, 0.0]]),
    (VANILLA_ENTITY_TYPE_HUSK_ID, &[[0.0, 2.075, 0.0]]),
    (VANILLA_ENTITY_TYPE_ILLUSIONER_ID, &[[0.0, 2.0, 0.0]]),
    (VANILLA_ENTITY_TYPE_LLAMA_ID, &[[0.0, 1.37, -0.3]]),
    (VANILLA_ENTITY_TYPE_MINECART_ID, &[[0.0, 0.1875, 0.0]]),
    (VANILLA_ENTITY_TYPE_MOOSHROOM_ID, &[[0.0, 1.36875, 0.0]]),
    (VANILLA_ENTITY_TYPE_MULE_ID, &[[0.0, 1.2125, 0.0]]),
    (VANILLA_ENTITY_TYPE_NAUTILUS_ID, &[[0.0, 1.1375, 0.0]]),
    (VANILLA_ENTITY_TYPE_OCELOT_ID, &[[0.0, 0.6375, 0.0]]),
    (VANILLA_ENTITY_TYPE_PARROT_ID, &[[0.0, 0.4625, 0.0]]),
    (VANILLA_ENTITY_TYPE_PHANTOM_ID, &[[0.0, 0.3375, 0.0]]),
    (VANILLA_ENTITY_TYPE_PIG_ID, &[[0.0, 0.86875, 0.0]]),
    (VANILLA_ENTITY_TYPE_PIGLIN_ID, &[[0.0, 2.0125, 0.0]]),
    (VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, &[[0.0, 2.0125, 0.0]]),
    (VANILLA_ENTITY_TYPE_PILLAGER_ID, &[[0.0, 2.0, 0.0]]),
    (VANILLA_ENTITY_TYPE_RAVAGER_ID, &[[0.0, 2.2625, -0.0625]]),
    (VANILLA_ENTITY_TYPE_SHEEP_ID, &[[0.0, 1.2375, 0.0]]),
    (VANILLA_ENTITY_TYPE_SILVERFISH_ID, &[[0.0, 0.2375, 0.0]]),
    (
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
        &[[0.0, 1.31875, 0.0]],
    ),
    (VANILLA_ENTITY_TYPE_SNIFFER_ID, &[[0.0, 2.09375, 0.0]]),
    (
        VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
        &[[0.0, 0.1875, 0.0]],
    ),
    (VANILLA_ENTITY_TYPE_SPIDER_ID, &[[0.0, 0.765, 0.0]]),
    (VANILLA_ENTITY_TYPE_TNT_MINECART_ID, &[[0.0, 0.1875, 0.0]]),
    (VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, &[[0.0, 1.37, -0.3]]),
    (VANILLA_ENTITY_TYPE_TURTLE_ID, &[[0.0, 0.55625, -0.25]]),
    (VANILLA_ENTITY_TYPE_VEX_ID, &[[0.0, 0.7375, 0.0]]),
    (VANILLA_ENTITY_TYPE_VINDICATOR_ID, &[[0.0, 2.0, 0.0]]),
    (VANILLA_ENTITY_TYPE_WARDEN_ID, &[[0.0, 3.15, 0.0]]),
    (VANILLA_ENTITY_TYPE_WITCH_ID, &[[0.0, 2.2625, 0.0]]),
    (VANILLA_ENTITY_TYPE_WOLF_ID, &[[0.0, 0.81875, -0.0625]]),
    (VANILLA_ENTITY_TYPE_ZOGLIN_ID, &[[0.0, 1.49375, 0.0]]),
    (VANILLA_ENTITY_TYPE_ZOMBIE_ID, &[[0.0, 2.0125, 0.0]]),
    (VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, &[[0.0, 1.31875, 0.0]]),
    (
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        &[[0.0, 1.1375, 0.0]],
    ),
    (VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID, &[[0.0, 2.125, 0.0]]),
    (VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID, &[[0.0, 2.0, 0.0]]),
];

// IDs and explicit eye-height values follow the vanilla 26.1 EntityType.java registration.
const VANILLA_ENTITY_EYE_HEIGHT_OVERRIDES: &[(i32, f32)] = &[
    (0, 0.5625),       // minecraft:acacia_boat
    (1, 0.5625),       // minecraft:acacia_chest_boat
    (2, 0.36),         // minecraft:allay
    (4, 0.26),         // minecraft:armadillo
    (5, 1.7775),       // minecraft:armor_stand
    (7, 0.2751),       // minecraft:axolotl
    (8, 0.5625),       // minecraft:bamboo_chest_raft
    (9, 0.5625),       // minecraft:bamboo_raft
    (10, 0.45),        // minecraft:bat
    (11, 0.3),         // minecraft:bee
    (12, 0.5625),      // minecraft:birch_boat
    (13, 0.5625),      // minecraft:birch_chest_boat
    (16, 1.74),        // minecraft:bogged
    (17, 1.3452),      // minecraft:breeze
    (18, 0.0),         // minecraft:breeze_wind_charge
    (19, 2.275),       // minecraft:camel
    (20, 2.275),       // minecraft:camel_husk
    (21, 0.35),        // minecraft:cat
    (22, 0.45),        // minecraft:cave_spider
    (23, 0.5625),      // minecraft:cherry_boat
    (24, 0.5625),      // minecraft:cherry_chest_boat
    (26, 0.644),       // minecraft:chicken
    (27, 0.195),       // minecraft:cod
    (28, 0.8125),      // minecraft:copper_golem
    (30, 1.3),         // minecraft:cow
    (31, 2.3),         // minecraft:creaking
    (33, 0.5625),      // minecraft:dark_oak_boat
    (34, 0.5625),      // minecraft:dark_oak_chest_boat
    (35, 0.3),         // minecraft:dolphin
    (36, 1.425),       // minecraft:donkey
    (38, 1.74),        // minecraft:drowned
    (40, 0.99875),     // minecraft:elder_guardian
    (41, 2.55),        // minecraft:enderman
    (42, 0.13),        // minecraft:endermite
    (54, 0.4),         // minecraft:fox
    (57, 2.6),         // minecraft:ghast
    (58, 2.6),         // minecraft:happy_ghast
    (59, 10.44),       // minecraft:giant
    (61, 0.4),         // minecraft:glow_squid
    (63, 0.425),       // minecraft:guardian
    (66, 1.52),        // minecraft:horse
    (67, 1.74),        // minecraft:husk
    (74, 0.5625),      // minecraft:jungle_boat
    (75, 0.5625),      // minecraft:jungle_chest_boat
    (78, 1.7765),      // minecraft:llama
    (80, 0.325),       // minecraft:magma_cube
    (81, 0.5625),      // minecraft:mangrove_boat
    (82, 0.5625),      // minecraft:mangrove_chest_boat
    (83, 1.62),        // minecraft:mannequin
    (86, 1.3),         // minecraft:mooshroom
    (87, 1.52),        // minecraft:mule
    (88, 0.2751),      // minecraft:nautilus
    (89, 0.5625),      // minecraft:oak_boat
    (90, 0.5625),      // minecraft:oak_chest_boat
    (94, 0.5625),      // minecraft:pale_oak_boat
    (95, 0.5625),      // minecraft:pale_oak_chest_boat
    (97, 1.74),        // minecraft:parched
    (98, 0.54),        // minecraft:parrot
    (99, 0.175),       // minecraft:phantom
    (101, 1.79),       // minecraft:piglin
    (102, 1.79),       // minecraft:piglin_brute
    (107, 0.455),      // minecraft:pufferfish
    (108, 0.59),       // minecraft:rabbit
    (110, 0.26),       // minecraft:salmon
    (111, 1.235),      // minecraft:sheep
    (112, 0.5),        // minecraft:shulker
    (114, 0.13),       // minecraft:silverfish
    (115, 1.74),       // minecraft:skeleton
    (116, 1.52),       // minecraft:skeleton_horse
    (117, 0.325),      // minecraft:slime
    (119, 1.05),       // minecraft:sniffer
    (121, 1.7),        // minecraft:snow_golem
    (124, 0.65),       // minecraft:spider
    (125, 0.5625),     // minecraft:spruce_boat
    (126, 0.5625),     // minecraft:spruce_chest_boat
    (127, 0.4),        // minecraft:squid
    (128, 1.74),       // minecraft:stray
    (130, 0.19500001), // minecraft:tadpole
    (132, 0.15),       // minecraft:tnt
    (134, 1.7765),     // minecraft:trader_llama
    (136, 0.26),       // minecraft:tropical_fish
    (138, 0.51875),    // minecraft:vex
    (139, 1.62),       // minecraft:villager
    (141, 1.62),       // minecraft:wandering_trader
    (143, 0.0),        // minecraft:wind_charge
    (144, 1.62),       // minecraft:witch
    (146, 2.1),        // minecraft:wither_skeleton
    (148, 0.68),       // minecraft:wolf
    (150, 1.74),       // minecraft:zombie
    (151, 1.52),       // minecraft:zombie_horse
    (152, 0.2751),     // minecraft:zombie_nautilus
    (153, 1.74),       // minecraft:zombie_villager
    (154, 1.79),       // minecraft:zombified_piglin
    (155, 1.62),       // minecraft:player
];

// IDs are the vanilla 26.1 EntityType registry order from EntityType.java.
const VANILLA_ENTITY_PICK_BOUNDS: &[(i32, EntityPickBoundsState)] = &[
    (0, pick(1.375, 0.5625, 0.0)),    // minecraft:acacia_boat
    (1, pick(1.375, 0.5625, 0.0)),    // minecraft:acacia_chest_boat
    (2, pick(0.35, 0.6, 0.0)),        // minecraft:allay
    (4, pick(0.7, 0.65, 0.0)),        // minecraft:armadillo
    (5, pick(0.5, 1.975, 0.0)),       // minecraft:armor_stand
    (6, pick(0.5, 0.5, 0.0)),         // minecraft:arrow
    (7, pick(0.75, 0.42, 0.0)),       // minecraft:axolotl
    (8, pick(1.375, 0.5625, 0.0)),    // minecraft:bamboo_chest_raft
    (9, pick(1.375, 0.5625, 0.0)),    // minecraft:bamboo_raft
    (10, pick(0.5, 0.9, 0.0)),        // minecraft:bat
    (11, pick(0.7, 0.6, 0.0)),        // minecraft:bee
    (12, pick(1.375, 0.5625, 0.0)),   // minecraft:birch_boat
    (13, pick(1.375, 0.5625, 0.0)),   // minecraft:birch_chest_boat
    (14, pick(0.6, 1.8, 0.0)),        // minecraft:blaze
    (16, pick(0.6, 1.99, 0.0)),       // minecraft:bogged
    (17, pick(0.6, 1.77, 0.0)),       // minecraft:breeze
    (18, pick(0.3125, 0.3125, 1.0)),  // minecraft:breeze_wind_charge
    (19, pick(1.7, 2.375, 0.0)),      // minecraft:camel
    (20, pick(1.7, 2.375, 0.0)),      // minecraft:camel_husk
    (21, pick(0.6, 0.7, 0.0)),        // minecraft:cat
    (22, pick(0.7, 0.5, 0.0)),        // minecraft:cave_spider
    (23, pick(1.375, 0.5625, 0.0)),   // minecraft:cherry_boat
    (24, pick(1.375, 0.5625, 0.0)),   // minecraft:cherry_chest_boat
    (25, pick(0.98, 0.7, 0.0)),       // minecraft:chest_minecart
    (26, pick(0.4, 0.7, 0.0)),        // minecraft:chicken
    (27, pick(0.5, 0.3, 0.0)),        // minecraft:cod
    (28, pick(0.49, 0.98, 0.0)),      // minecraft:copper_golem
    (29, pick(0.98, 0.7, 0.0)),       // minecraft:command_block_minecart
    (30, pick(0.9, 1.4, 0.0)),        // minecraft:cow
    (31, pick(0.9, 2.7, 0.0)),        // minecraft:creaking
    (32, pick(0.6, 1.7, 0.0)),        // minecraft:creeper
    (33, pick(1.375, 0.5625, 0.0)),   // minecraft:dark_oak_boat
    (34, pick(1.375, 0.5625, 0.0)),   // minecraft:dark_oak_chest_boat
    (35, pick(0.9, 0.6, 0.0)),        // minecraft:dolphin
    (36, pick(1.3964844, 1.5, 0.0)),  // minecraft:donkey
    (37, pick(1.0, 1.0, 0.0)),        // minecraft:dragon_fireball
    (38, pick(0.6, 1.95, 0.0)),       // minecraft:drowned
    (40, pick(1.9975, 1.9975, 0.0)),  // minecraft:elder_guardian
    (41, pick(0.6, 2.9, 0.0)),        // minecraft:enderman
    (42, pick(0.4, 0.3, 0.0)),        // minecraft:endermite
    (45, pick(2.0, 2.0, 0.0)),        // minecraft:end_crystal
    (46, pick(0.6, 1.95, 0.0)),       // minecraft:evoker
    (47, pick(0.5, 0.8, 0.0)),        // minecraft:evoker_fangs
    (51, pick(0.98, 0.98, 0.0)),      // minecraft:falling_block
    (52, pick(1.0, 1.0, 1.0)),        // minecraft:fireball
    (54, pick(0.6, 0.7, 0.0)),        // minecraft:fox
    (55, pick(0.5, 0.5, 0.0)),        // minecraft:frog
    (56, pick(0.98, 0.7, 0.0)),       // minecraft:furnace_minecart
    (57, pick(4.0, 4.0, 0.0)),        // minecraft:ghast
    (58, pick(4.0, 4.0, 0.0)),        // minecraft:happy_ghast
    (59, pick(3.6, 12.0, 0.0)),       // minecraft:giant
    (61, pick(0.8, 0.8, 0.0)),        // minecraft:glow_squid
    (62, pick(0.9, 1.3, 0.0)),        // minecraft:goat
    (63, pick(0.85, 0.85, 0.0)),      // minecraft:guardian
    (64, pick(1.3964844, 1.4, 0.0)),  // minecraft:hoglin
    (65, pick(0.98, 0.7, 0.0)),       // minecraft:hopper_minecart
    (66, pick(1.3964844, 1.6, 0.0)),  // minecraft:horse
    (67, pick(0.6, 1.95, 0.0)),       // minecraft:husk
    (68, pick(0.6, 1.95, 0.0)),       // minecraft:illusioner
    (70, pick(1.4, 2.7, 0.0)),        // minecraft:iron_golem
    (74, pick(1.375, 0.5625, 0.0)),   // minecraft:jungle_boat
    (75, pick(1.375, 0.5625, 0.0)),   // minecraft:jungle_chest_boat
    (78, pick(0.9, 1.87, 0.0)),       // minecraft:llama
    (80, pick(0.52, 0.52, 0.0)),      // minecraft:magma_cube
    (81, pick(1.375, 0.5625, 0.0)),   // minecraft:mangrove_boat
    (82, pick(1.375, 0.5625, 0.0)),   // minecraft:mangrove_chest_boat
    (83, pick(0.6, 1.8, 0.0)),        // minecraft:mannequin
    (85, pick(0.98, 0.7, 0.0)),       // minecraft:minecart
    (86, pick(0.9, 1.4, 0.0)),        // minecraft:mooshroom
    (87, pick(1.3964844, 1.6, 0.0)),  // minecraft:mule
    (88, pick(0.875, 0.95, 0.0)),     // minecraft:nautilus
    (89, pick(1.375, 0.5625, 0.0)),   // minecraft:oak_boat
    (90, pick(1.375, 0.5625, 0.0)),   // minecraft:oak_chest_boat
    (91, pick(0.6, 0.7, 0.0)),        // minecraft:ocelot
    (94, pick(1.375, 0.5625, 0.0)),   // minecraft:pale_oak_boat
    (95, pick(1.375, 0.5625, 0.0)),   // minecraft:pale_oak_chest_boat
    (96, pick(1.3, 1.25, 0.0)),       // minecraft:panda
    (97, pick(0.6, 1.99, 0.0)),       // minecraft:parched
    (98, pick(0.5, 0.9, 0.0)),        // minecraft:parrot
    (99, pick(0.9, 0.5, 0.0)),        // minecraft:phantom
    (100, pick(0.9, 0.9, 0.0)),       // minecraft:pig
    (101, pick(0.6, 1.95, 0.0)),      // minecraft:piglin
    (102, pick(0.6, 1.95, 0.0)),      // minecraft:piglin_brute
    (103, pick(0.6, 1.95, 0.0)),      // minecraft:pillager
    (104, pick(1.4, 1.4, 0.0)),       // minecraft:polar_bear
    (107, pick(0.7, 0.7, 0.0)),       // minecraft:pufferfish
    (108, pick(0.49, 0.6, 0.0)),      // minecraft:rabbit
    (109, pick(1.95, 2.2, 0.0)),      // minecraft:ravager
    (110, pick(0.7, 0.4, 0.0)),       // minecraft:salmon
    (111, pick(0.9, 1.3, 0.0)),       // minecraft:sheep
    (112, pick(1.0, 1.0, 0.0)),       // minecraft:shulker
    (113, pick(0.3125, 0.3125, 1.0)), // minecraft:shulker_bullet
    (114, pick(0.4, 0.3, 0.0)),       // minecraft:silverfish
    (115, pick(0.6, 1.99, 0.0)),      // minecraft:skeleton
    (116, pick(1.3964844, 1.6, 0.0)), // minecraft:skeleton_horse
    (117, pick(0.52, 0.52, 0.0)),     // minecraft:slime
    (119, pick(1.9, 1.75, 0.0)),      // minecraft:sniffer
    (121, pick(0.7, 1.9, 0.0)),       // minecraft:snow_golem
    (122, pick(0.98, 0.7, 0.0)),      // minecraft:spawner_minecart
    (123, pick(0.5, 0.5, 0.0)),       // minecraft:spectral_arrow
    (124, pick(1.4, 0.9, 0.0)),       // minecraft:spider
    (125, pick(1.375, 0.5625, 0.0)),  // minecraft:spruce_boat
    (126, pick(1.375, 0.5625, 0.0)),  // minecraft:spruce_chest_boat
    (127, pick(0.8, 0.8, 0.0)),       // minecraft:squid
    (128, pick(0.6, 1.99, 0.0)),      // minecraft:stray
    (129, pick(0.9, 1.7, 0.0)),       // minecraft:strider
    (130, pick(0.4, 0.3, 0.0)),       // minecraft:tadpole
    (132, pick(0.98, 0.98, 0.0)),     // minecraft:tnt
    (133, pick(0.98, 0.7, 0.0)),      // minecraft:tnt_minecart
    (134, pick(0.9, 1.87, 0.0)),      // minecraft:trader_llama
    (135, pick(0.5, 0.5, 0.0)),       // minecraft:trident
    (136, pick(0.5, 0.4, 0.0)),       // minecraft:tropical_fish
    (137, pick(1.2, 0.4, 0.0)),       // minecraft:turtle
    (138, pick(0.4, 0.8, 0.0)),       // minecraft:vex
    (139, pick(0.6, 1.95, 0.0)),      // minecraft:villager
    (140, pick(0.6, 1.95, 0.0)),      // minecraft:vindicator
    (141, pick(0.6, 1.95, 0.0)),      // minecraft:wandering_trader
    (142, pick(0.9, 2.9, 0.0)),       // minecraft:warden
    (143, pick(0.3125, 0.3125, 1.0)), // minecraft:wind_charge
    (144, pick(0.6, 1.95, 0.0)),      // minecraft:witch
    (145, pick(0.9, 3.5, 0.0)),       // minecraft:wither
    (146, pick(0.7, 2.4, 0.0)),       // minecraft:wither_skeleton
    (147, pick(0.3125, 0.3125, 0.0)), // minecraft:wither_skull
    (148, pick(0.6, 0.85, 0.0)),      // minecraft:wolf
    (149, pick(1.3964844, 1.4, 0.0)), // minecraft:zoglin
    (150, pick(0.6, 1.95, 0.0)),      // minecraft:zombie
    (151, pick(1.3964844, 1.6, 0.0)), // minecraft:zombie_horse
    (152, pick(0.875, 0.95, 0.0)),    // minecraft:zombie_nautilus
    (153, pick(0.6, 1.95, 0.0)),      // minecraft:zombie_villager
    (154, pick(0.6, 1.95, 0.0)),      // minecraft:zombified_piglin
    (155, pick(0.6, 1.8, 0.0)),       // minecraft:player
];

// IDs are vanilla 26.1 EntityType registry ids whose client class extends
// LivingEntity. ClientboundUpdateAttributes is only valid for this set.
const VANILLA_LIVING_ENTITY_TYPE_IDS: &[i32] = &[
    2,   // minecraft:allay
    4,   // minecraft:armadillo
    5,   // minecraft:armor_stand
    7,   // minecraft:axolotl
    10,  // minecraft:bat
    11,  // minecraft:bee
    14,  // minecraft:blaze
    16,  // minecraft:bogged
    17,  // minecraft:breeze
    19,  // minecraft:camel
    20,  // minecraft:camel_husk
    21,  // minecraft:cat
    22,  // minecraft:cave_spider
    26,  // minecraft:chicken
    27,  // minecraft:cod
    28,  // minecraft:copper_golem
    30,  // minecraft:cow
    31,  // minecraft:creaking
    32,  // minecraft:creeper
    35,  // minecraft:dolphin
    36,  // minecraft:donkey
    38,  // minecraft:drowned
    40,  // minecraft:elder_guardian
    41,  // minecraft:enderman
    42,  // minecraft:endermite
    46,  // minecraft:evoker
    54,  // minecraft:fox
    55,  // minecraft:frog
    57,  // minecraft:ghast
    58,  // minecraft:happy_ghast
    59,  // minecraft:giant
    61,  // minecraft:glow_squid
    62,  // minecraft:goat
    63,  // minecraft:guardian
    64,  // minecraft:hoglin
    66,  // minecraft:horse
    67,  // minecraft:husk
    68,  // minecraft:illusioner
    70,  // minecraft:iron_golem
    78,  // minecraft:llama
    80,  // minecraft:magma_cube
    83,  // minecraft:mannequin
    86,  // minecraft:mooshroom
    87,  // minecraft:mule
    88,  // minecraft:nautilus
    91,  // minecraft:ocelot
    96,  // minecraft:panda
    97,  // minecraft:parched
    98,  // minecraft:parrot
    99,  // minecraft:phantom
    100, // minecraft:pig
    101, // minecraft:piglin
    102, // minecraft:piglin_brute
    103, // minecraft:pillager
    104, // minecraft:polar_bear
    107, // minecraft:pufferfish
    108, // minecraft:rabbit
    109, // minecraft:ravager
    110, // minecraft:salmon
    111, // minecraft:sheep
    112, // minecraft:shulker
    114, // minecraft:silverfish
    115, // minecraft:skeleton
    116, // minecraft:skeleton_horse
    117, // minecraft:slime
    119, // minecraft:sniffer
    121, // minecraft:snow_golem
    124, // minecraft:spider
    127, // minecraft:squid
    128, // minecraft:stray
    129, // minecraft:strider
    130, // minecraft:tadpole
    134, // minecraft:trader_llama
    136, // minecraft:tropical_fish
    137, // minecraft:turtle
    138, // minecraft:vex
    139, // minecraft:villager
    140, // minecraft:vindicator
    141, // minecraft:wandering_trader
    142, // minecraft:warden
    144, // minecraft:witch
    145, // minecraft:wither
    146, // minecraft:wither_skeleton
    148, // minecraft:wolf
    149, // minecraft:zoglin
    150, // minecraft:zombie
    151, // minecraft:zombie_horse
    152, // minecraft:zombie_nautilus
    153, // minecraft:zombie_villager
    154, // minecraft:zombified_piglin
    155, // minecraft:player
];
