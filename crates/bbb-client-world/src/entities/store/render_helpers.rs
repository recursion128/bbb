use super::*;

pub(super) fn avatar_flight_y_rotation(
    previous_delta_movement: EntityVec3,
    delta_movement: EntityVec3,
    look_angle: EntityVec3,
    partial_ticks: f32,
) -> (bool, f32) {
    let partial_ticks = f64::from(partial_ticks);
    let movement_x =
        previous_delta_movement.x + (delta_movement.x - previous_delta_movement.x) * partial_ticks;
    let movement_z =
        previous_delta_movement.z + (delta_movement.z - previous_delta_movement.z) * partial_ticks;
    let movement_length_squared = movement_x * movement_x + movement_z * movement_z;
    let look_length_squared = look_angle.x * look_angle.x + look_angle.z * look_angle.z;
    if movement_length_squared <= 1.0e-5 || look_length_squared <= 1.0e-5 {
        return (false, 0.0);
    }

    let movement_length = movement_length_squared.sqrt();
    let look_length = look_length_squared.sqrt();
    let dot = movement_x / movement_length * (look_angle.x / look_length)
        + movement_z / movement_length * (look_angle.z / look_length);
    let cross = movement_x * look_angle.z - movement_z * look_angle.x;
    (true, (cross.signum() * dot.abs().min(1.0).acos()) as f32)
}

pub(super) fn end_crystal_renderer_y(time_in_ticks: f32) -> f32 {
    let hh = (time_in_ticks * 0.2).sin() / 2.0 + 0.5;
    (hh * hh + hh) * 0.4 - 1.4
}

pub(super) fn end_crystal_intersects_ender_dragon_search_box(
    dragon_position: super::super::EntityVec3,
    crystal_position: super::super::EntityVec3,
) -> bool {
    // Vanilla 26.1 `EntityType.ENDER_DRAGON.sized(16, 8)` and
    // `EntityType.END_CRYSTAL.sized(2, 2)`, with `EnderDragon.checkCrystals` searching
    // `getBoundingBox().inflate(32)`.
    const DRAGON_HALF_WIDTH: f64 = 8.0;
    const DRAGON_HEIGHT: f64 = 8.0;
    const CRYSTAL_HALF_WIDTH: f64 = 1.0;
    const CRYSTAL_HEIGHT: f64 = 2.0;
    const SEARCH_INFLATE: f64 = 32.0;

    let dragon_min = [
        dragon_position.x - DRAGON_HALF_WIDTH - SEARCH_INFLATE,
        dragon_position.y - SEARCH_INFLATE,
        dragon_position.z - DRAGON_HALF_WIDTH - SEARCH_INFLATE,
    ];
    let dragon_max = [
        dragon_position.x + DRAGON_HALF_WIDTH + SEARCH_INFLATE,
        dragon_position.y + DRAGON_HEIGHT + SEARCH_INFLATE,
        dragon_position.z + DRAGON_HALF_WIDTH + SEARCH_INFLATE,
    ];
    let crystal_min = [
        crystal_position.x - CRYSTAL_HALF_WIDTH,
        crystal_position.y,
        crystal_position.z - CRYSTAL_HALF_WIDTH,
    ];
    let crystal_max = [
        crystal_position.x + CRYSTAL_HALF_WIDTH,
        crystal_position.y + CRYSTAL_HEIGHT,
        crystal_position.z + CRYSTAL_HALF_WIDTH,
    ];

    crystal_max[0] >= dragon_min[0]
        && crystal_min[0] <= dragon_max[0]
        && crystal_max[1] >= dragon_min[1]
        && crystal_min[1] <= dragon_max[1]
        && crystal_max[2] >= dragon_min[2]
        && crystal_min[2] <= dragon_max[2]
}

pub(super) fn renderer_special_culling_state(
    entity_type_id: i32,
    add_entity_data: i32,
    position: super::super::EntityVec3,
    data_values: &[ProtocolEntityDataValue],
    attributes: &[ProtocolAttributeSnapshot],
    animations: EntityClientAnimationState,
) -> super::super::EntityRendererSpecialCullingState {
    if vanilla_entity_metadata_type_is_guardian(entity_type_id) {
        return vanilla_eye_height_for_entity_data(
            entity_type_id,
            add_entity_data,
            data_values,
            attributes,
            Some(animations),
            None,
        )
        .map_or(
            super::super::EntityRendererSpecialCullingState::None,
            |eye_height| super::super::EntityRendererSpecialCullingState::Guardian {
                attack_target_id: guardian_attack_target_id(entity_type_id, data_values),
                eye_height,
            },
        );
    }

    if let Some(accessors) =
        vanilla_entity_metadata_end_crystal_accessors_for_entity_type(entity_type_id)
    {
        let has_beam_target = data_values.iter().any(|value| {
            value.data_id == accessors.beam_target_data_id
                && matches!(value.value, EntityDataValueKind::OptionalBlockPos(Some(_)))
        });
        return super::super::EntityRendererSpecialCullingState::EndCrystal { has_beam_target };
    }

    if vanilla_entity_metadata_shulker_accessors_for_entity_type(entity_type_id).is_some() {
        let Some(bounds) = vanilla_pick_bounds_for_type(entity_type_id) else {
            return super::super::EntityRendererSpecialCullingState::None;
        };
        let scale = vanilla_render_scale(entity_type_id, attributes);
        return super::super::EntityRendererSpecialCullingState::Shulker {
            render_offset_at_zero: animations.shulker_render_offset(position, 0.0, scale),
            half_width: (bounds.max[0] - bounds.min[0]) * 0.5,
            half_height: (bounds.max[1] - bounds.min[1]) * 0.5,
        };
    }

    super::super::EntityRendererSpecialCullingState::None
}

pub(super) fn solve_living_passenger_body_y_rot(head_y_rot: f32, vehicle_body_y_rot: f32) -> f32 {
    let head_difference = wrap_degrees_f32(head_y_rot - vehicle_body_y_rot).clamp(-85.0, 85.0);
    let mut body_y_rot = head_y_rot - head_difference;
    if head_difference.abs() > 50.0 {
        body_y_rot += head_difference * 0.2;
    }
    body_y_rot
}

pub(super) fn wrap_degrees_f32(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

pub(super) fn vanilla_attribute_value(attribute: &ProtocolAttributeSnapshot) -> f64 {
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
    if let Some(descriptor) = vanilla_attribute_type(attribute.attribute_id) {
        let min = f64::from_bits(descriptor.min_value_bits);
        let max = f64::from_bits(descriptor.max_value_bits);
        if result.is_nan() {
            return min;
        }
        return crate::client::mth_clamp_f64(result, min, max);
    }
    if result.is_finite() {
        result
    } else {
        0.0
    }
}

pub(super) fn shulker_attach_face(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityAttachmentFace {
    let Some(accessors) = vanilla_entity_metadata_shulker_accessors_for_entity_type(entity_type_id)
    else {
        return EntityAttachmentFace::Down;
    };
    data_values
        .iter()
        .find_map(|value| {
            if value.data_id != accessors.attach_face_data_id {
                return None;
            }
            let EntityDataValueKind::Direction(direction) = &value.value else {
                return None;
            };
            Some(EntityAttachmentFace::from_3d_data(*direction))
        })
        .unwrap_or_else(|| EntityAttachmentFace::from_3d_data(accessors.attach_face_default))
}

pub(super) fn firework_rocket_render_stack(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    accessors: VanillaFireworkRocketMetadataAccessors,
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    metadata_item_stack(data_values, accessors.item_data_id)
        .filter(|stack| stack.item_id.is_some() && stack.count > 0)
}

pub(super) fn ominous_item_spawner_render_stack(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    let data_id =
        vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type(entity_type_id)?;
    metadata_item_stack(data_values, data_id)
        .filter(|stack| stack.item_id.is_some() && stack.count > 0)
}

pub(super) fn metadata_item_stack(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    data_values.iter().find_map(|value| {
        if value.data_id != data_id {
            return None;
        }
        let EntityDataValueKind::ItemStack(stack) = &value.value else {
            return None;
        };
        Some(stack)
    })
}

pub(super) fn firework_rocket_attached_to_entity(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    accessors: VanillaFireworkRocketMetadataAccessors,
) -> bool {
    data_values.iter().any(|value| {
        value.data_id == accessors.attached_to_target_data_id
            && matches!(
                &value.value,
                EntityDataValueKind::OptionalUnsignedInt(Some(_))
            )
    })
}

pub(super) fn firework_rocket_shot_at_angle(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    accessors: VanillaFireworkRocketMetadataAccessors,
) -> bool {
    data_values.iter().any(|value| {
        value.data_id == accessors.shot_at_angle_data_id
            && matches!(&value.value, EntityDataValueKind::Boolean(true))
    })
}
