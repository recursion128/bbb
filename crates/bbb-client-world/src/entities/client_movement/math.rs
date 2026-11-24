use super::*;
pub(super) use crate::entities::entity_data::{
    entity_data_bool, entity_data_byte, entity_data_float, entity_data_int,
    entity_data_optional_unsigned_int,
};

pub(super) fn item_entity_has_stack(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    let Some(data_id) =
        vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(entity_type_id)
    else {
        return false;
    };
    data_values.iter().any(|value| {
        value.data_id == data_id
            && matches!(
                &value.value,
                EntityDataValueKind::ItemStack(stack)
                    if stack.item_id.is_some() && stack.count > 0
            )
    })
}

pub(in crate::entities) fn apply_entity_fluid_currents(
    movement: EntityVec3,
    contact: crate::client::EntityFluidContactState,
    lava_current_scale: f64,
) -> EntityVec3 {
    let mut velocity = [movement.x, movement.y, movement.z];
    for (current, count, scale) in [
        (
            contact.water_current,
            contact.water_current_count,
            bbb_facts::fluid::WATER_CURRENT_SCALE,
        ),
        (
            contact.lava_current,
            contact.lava_current_count,
            lava_current_scale,
        ),
    ] {
        let tracker = bbb_facts::fluid::FluidTrackerState {
            accumulated_current: [current.x, current.y, current.z],
            current_count: count,
            ..bbb_facts::fluid::FluidTrackerState::default()
        };
        velocity = tracker.apply_current(velocity, scale, false, 1.0);
    }
    EntityVec3 {
        x: velocity[0],
        y: velocity[1],
        z: velocity[2],
    }
}

pub(super) fn select_experience_orb_player_target(
    orb_position: EntityVec3,
    following_player_uuid: Option<uuid::Uuid>,
    targets: &[EntityExperienceOrbPlayerTarget],
) -> Option<EntityExperienceOrbPlayerTarget> {
    if let Some(current) = following_player_uuid
        .and_then(|uuid| targets.iter().copied().find(|target| target.uuid == uuid))
    {
        if length_squared(subtract(current.position, orb_position)) <= 64.0 {
            return Some(current);
        }
    }
    let mut nearest = None;
    let mut nearest_distance = f64::INFINITY;
    for target in targets.iter().copied() {
        if target.is_dead_or_dying {
            continue;
        }
        let distance = length_squared(subtract(target.position, orb_position));
        // `EntityGetter.getNearestPlayer` uses strict comparisons, preserving
        // the first player at an equal distance and excluding exactly 8 blocks.
        if distance < 64.0 && distance < nearest_distance {
            nearest = Some(target);
            nearest_distance = distance;
        }
    }
    nearest
}

pub(super) fn set_entity_data_byte(metadata: &mut EntityMetadata, data_id: u8, value: i8) {
    let value = bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(value),
    };
    if let Some(existing) = metadata
        .data_values
        .iter_mut()
        .find(|existing| existing.data_id == data_id)
    {
        *existing = value;
    } else {
        metadata.data_values.push(value);
        metadata.data_values.sort_by_key(|value| value.data_id);
    }
}

pub(super) fn set_entity_data_bool(metadata: &mut EntityMetadata, data_id: u8, value: bool) {
    let value = bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(value),
    };
    if let Some(existing) = metadata
        .data_values
        .iter_mut()
        .find(|existing| existing.data_id == data_id)
    {
        *existing = value;
    } else {
        metadata.data_values.push(value);
        metadata.data_values.sort_by_key(|value| value.data_id);
    }
}

pub(super) fn set_entity_data_int(metadata: &mut EntityMetadata, data_id: u8, value: i32) {
    let value = bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(value),
    };
    if let Some(existing) = metadata
        .data_values
        .iter_mut()
        .find(|existing| existing.data_id == data_id)
    {
        *existing = value;
    } else {
        metadata.data_values.push(value);
        metadata.data_values.sort_by_key(|value| value.data_id);
    }
}

pub(super) fn add(left: EntityVec3, right: EntityVec3) -> EntityVec3 {
    EntityVec3 {
        x: left.x + right.x,
        y: left.y + right.y,
        z: left.z + right.z,
    }
}

pub(super) fn horizontal_length_squared(value: EntityVec3) -> f64 {
    value.x * value.x + value.z * value.z
}

pub(super) fn length_squared(value: EntityVec3) -> f64 {
    horizontal_length_squared(value) + value.y * value.y
}

pub(super) fn subtract(left: EntityVec3, right: EntityVec3) -> EntityVec3 {
    EntityVec3 {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

pub(super) fn scale(value: EntityVec3, scale: f64) -> EntityVec3 {
    EntityVec3 {
        x: value.x * scale,
        y: value.y * scale,
        z: value.z * scale,
    }
}

pub(super) fn normalize(value: EntityVec3) -> EntityVec3 {
    let [x, y, z] = bbb_facts::collision::vec3_normalize([value.x, value.y, value.z]);
    EntityVec3 { x, y, z }
}

pub(crate) fn calculate_view_vector(x_rot: f32, y_rot: f32) -> EntityVec3 {
    let radians_per_degree = (std::f64::consts::PI / 180.0) as f32;
    let real_x_rot = x_rot * radians_per_degree;
    let real_y_rot = -y_rot * radians_per_degree;
    let y_cos = vanilla_mth_cos(f64::from(real_y_rot));
    let y_sin = vanilla_mth_sin(f64::from(real_y_rot));
    let x_cos = vanilla_mth_cos(f64::from(real_x_rot));
    let x_sin = vanilla_mth_sin(f64::from(real_x_rot));
    EntityVec3 {
        x: f64::from(y_sin * x_cos),
        y: f64::from(-x_sin),
        z: f64::from(y_cos * x_cos),
    }
}

pub(in crate::entities) fn vanilla_mth_sin(angle: f64) -> f32 {
    vanilla_mth_sin_table_value((angle * 10_430.378_350_470_453) as i64)
}

pub(in crate::entities) fn vanilla_mth_cos(angle: f64) -> f32 {
    vanilla_mth_sin_table_value((angle * 10_430.378_350_470_453 + 16_384.0) as i64)
}

pub(super) fn vanilla_mth_sin_table_value(raw_index: i64) -> f32 {
    let index = raw_index & 65_535;
    (index as f64 / 10_430.378_350_470_453).sin() as f32
}

pub(super) fn advance_position(transform: &mut EntityTransform) {
    transform.position.x += transform.delta_movement.x;
    transform.position.y += transform.delta_movement.y;
    transform.position.z += transform.delta_movement.z;
}

pub(super) fn rotate_towards_movement(transform: &mut EntityTransform, rotation_speed: f32) {
    let movement = transform.delta_movement;
    if movement == EntityVec3::default() {
        return;
    }
    let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
    let target_y_rot = vanilla_degrees(movement.z.atan2(movement.x)) + 90.0;
    let target_x_rot = vanilla_degrees(horizontal.atan2(movement.y)) - 90.0;
    transform.old_x_rot = unwrap_old_rotation(transform.old_x_rot, target_x_rot);
    transform.old_y_rot = unwrap_old_rotation(transform.old_y_rot, target_y_rot);
    transform.x_rot = lerp(rotation_speed, transform.old_x_rot, target_x_rot);
    transform.y_rot = lerp(rotation_speed, transform.old_y_rot, target_y_rot);
}

pub(super) fn update_projectile_rotation(transform: &mut EntityTransform) {
    let movement = transform.delta_movement;
    let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
    let target_x_rot = vanilla_degrees(movement.y.atan2(horizontal));
    let target_y_rot = vanilla_degrees(movement.x.atan2(movement.z));
    transform.old_x_rot = unwrap_old_rotation(transform.old_x_rot, target_x_rot);
    transform.old_y_rot = unwrap_old_rotation(transform.old_y_rot, target_y_rot);
    transform.x_rot = lerp(0.2, transform.old_x_rot, target_x_rot);
    transform.y_rot = lerp(0.2, transform.old_y_rot, target_y_rot);
}

pub(super) fn vanilla_degrees(radians: f64) -> f32 {
    (radians * f64::from(180.0_f32) / f64::from(std::f32::consts::PI)) as f32
}

pub(super) fn unwrap_old_rotation(old: f32, target: f32) -> f32 {
    if !old.is_finite() || !target.is_finite() {
        return if target.is_finite() { target } else { 0.0 };
    }
    let delta = (target - old + 180.0).rem_euclid(360.0) - 180.0;
    target - delta
}

pub(super) fn lerp(alpha: f32, start: f32, end: f32) -> f32 {
    start + alpha * (end - start)
}
