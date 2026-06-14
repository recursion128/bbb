use bbb_protocol::packets::{
    AttributeSnapshot, EntityDataValue, EntityDataValueKind, ItemStackSummary,
};
use serde::{Deserialize, Serialize};

use super::EntityVec3;

const VANILLA_ENTITY_TYPE_ARMOR_STAND_ID: i32 = 5;
const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
const VANILLA_ENTITY_TYPE_COW_ID: i32 = 30;
const VANILLA_ENTITY_TYPE_DROWNED_ID: i32 = 38;
const VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID: i32 = 60;
const VANILLA_ENTITY_TYPE_HUSK_ID: i32 = 67;
const VANILLA_ENTITY_TYPE_INTERACTION_ID: i32 = 69;
const VANILLA_ENTITY_TYPE_ITEM_FRAME_ID: i32 = 73;
const VANILLA_ENTITY_TYPE_LEASH_KNOT_ID: i32 = 76;
const VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID: i32 = 80;
const VANILLA_ENTITY_TYPE_MANNEQUIN_ID: i32 = 83;
const VANILLA_ENTITY_TYPE_MOOSHROOM_ID: i32 = 86;
const VANILLA_ENTITY_TYPE_PAINTING_ID: i32 = 93;
const VANILLA_ENTITY_TYPE_PIG_ID: i32 = 100;
const VANILLA_ENTITY_TYPE_PIGLIN_ID: i32 = 101;
const VANILLA_ENTITY_TYPE_PLAYER_ID: i32 = 155;
const VANILLA_ENTITY_TYPE_SLIME_ID: i32 = 117;
const VANILLA_ENTITY_TYPE_VILLAGER_ID: i32 = 139;
const VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID: i32 = 141;
const VANILLA_ENTITY_TYPE_ZOMBIE_ID: i32 = 150;
const VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID: i32 = 153;
const VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID: i32 = 154;
const ENTITY_DATA_POSE_ID: u8 = 6;
const AGEABLE_MOB_BABY_DATA_ID: u8 = 16;
const PIGLIN_BABY_DATA_ID: u8 = 17;
const ZOMBIE_BABY_DATA_ID: u8 = 16;
const HANGING_DATA_DIRECTION_ID: u8 = 8;
const ITEM_FRAME_DATA_ITEM_ID: u8 = 9;
const PAINTING_DATA_VARIANT_ID: u8 = 9;
const INTERACTION_DATA_WIDTH_ID: u8 = 8;
const INTERACTION_DATA_HEIGHT_ID: u8 = 9;
const INTERACTION_DEFAULT_WIDTH: f32 = 1.0;
const INTERACTION_DEFAULT_HEIGHT: f32 = 1.0;
const ITEM_FRAME_DEPTH: f32 = 0.0625;
const ITEM_FRAME_DEFAULT_SIZE: f32 = 0.75;
const ITEM_FRAME_MAP_SIZE: f32 = 1.0;
const MAP_ID_DATA_COMPONENT_TYPE_ID: i32 = 41;
const PAINTING_DEPTH: f32 = 0.0625;
const HANGING_WALL_OFFSET: f64 = 0.46875;
const SLIME_SIZE_DATA_ID: u8 = 16;
const SLIME_BASE_SIZE: f32 = 0.52;
const SLIME_DEFAULT_SIZE: i32 = 1;
const ARMOR_STAND_CLIENT_FLAGS_DATA_ID: u8 = 16;
const ARMOR_STAND_CLIENT_FLAG_SMALL: i8 = 1;
const ARMOR_STAND_CLIENT_FLAG_MARKER: i8 = 16;
const ARMOR_STAND_WIDTH: f32 = 0.5;
const ARMOR_STAND_HEIGHT: f32 = 1.975;
const ARMOR_STAND_SMALL_SCALE: f32 = 0.5;
const VANILLA_ATTRIBUTE_SCALE_ID: i32 = 25;
const VANILLA_SCALE_MIN: f64 = 0.0625;
const VANILLA_SCALE_MAX: f64 = 16.0;
const VANILLA_POSE_FALL_FLYING_ID: i32 = 1;
const VANILLA_POSE_SLEEPING_ID: i32 = 2;
const VANILLA_POSE_SWIMMING_ID: i32 = 3;
const VANILLA_POSE_SPIN_ATTACK_ID: i32 = 4;
const VANILLA_POSE_CROUCHING_ID: i32 = 5;
const VANILLA_POSE_DYING_ID: i32 = 7;

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
    } else if let Some(bounds) = baby_pick_bounds(entity_type_id, data_values) {
        bounds
    } else if entity_type_id == VANILLA_ENTITY_TYPE_INTERACTION_ID {
        interaction_pick_bounds(data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_ITEM_FRAME_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
    {
        item_frame_pick_bounds(add_entity_data, data_values)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID {
        painting_pick_bounds(add_entity_data, data_values)?
    } else if entity_type_id == VANILLA_ENTITY_TYPE_LEASH_KNOT_ID {
        EntityPickBoundsState::from_base_size(0.375, 0.5, 0.0)
    } else if entity_type_id == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_SLIME_ID
    {
        slime_pick_bounds(data_values)
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

pub(crate) fn vanilla_client_position_for_entity_data(
    entity_type_id: i32,
    packet_position: EntityVec3,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityVec3> {
    if entity_type_id == VANILLA_ENTITY_TYPE_ITEM_FRAME_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
    {
        return Some(item_frame_center(
            packet_position,
            add_entity_data,
            data_values,
        ));
    }
    if entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID {
        return painting_center(packet_position, add_entity_data, data_values);
    }
    if entity_type_id == VANILLA_ENTITY_TYPE_LEASH_KNOT_ID {
        return Some(leash_knot_position(packet_position));
    }
    None
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

fn baby_pick_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityPickBoundsState> {
    let baby = match entity_type_id {
        VANILLA_ENTITY_TYPE_DROWNED_ID
        | VANILLA_ENTITY_TYPE_HUSK_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
        | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => {
            entity_data_bool(data_values, ZOMBIE_BABY_DATA_ID, false)
        }
        VANILLA_ENTITY_TYPE_PIGLIN_ID => entity_data_bool(data_values, PIGLIN_BABY_DATA_ID, false),
        VANILLA_ENTITY_TYPE_CHICKEN_ID
        | VANILLA_ENTITY_TYPE_COW_ID
        | VANILLA_ENTITY_TYPE_MOOSHROOM_ID
        | VANILLA_ENTITY_TYPE_PIG_ID
        | VANILLA_ENTITY_TYPE_VILLAGER_ID
        | VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID => {
            entity_data_bool(data_values, AGEABLE_MOB_BABY_DATA_ID, false)
        }
        _ => false,
    };
    if !baby {
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
        _ => vanilla_pick_bounds_for_type(entity_type_id)?.scale_dimensions(0.5),
    })
}

fn item_frame_pick_bounds(
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> EntityPickBoundsState {
    let direction = hanging_direction(add_entity_data, data_values);
    let frame_size = if item_frame_has_framed_map(data_values) {
        ITEM_FRAME_MAP_SIZE
    } else {
        ITEM_FRAME_DEFAULT_SIZE
    };
    let (x_size, y_size, z_size) =
        item_frame_size_for_direction(direction, frame_size, ITEM_FRAME_DEPTH);
    EntityPickBoundsState::from_centered_size(x_size, y_size, z_size, 0.0)
}

fn painting_pick_bounds(
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityPickBoundsState> {
    let direction = hanging_direction(add_entity_data, data_values);
    let (width, height) = painting_variant_size(data_values);
    let (x_size, y_size, z_size) = painting_size_for_direction(direction, width, height)?;
    Some(EntityPickBoundsState::from_centered_size(
        x_size, y_size, z_size, 0.0,
    ))
}

fn item_frame_center(
    packet_position: EntityVec3,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> EntityVec3 {
    let direction = hanging_direction(add_entity_data, data_values);
    hanging_wall_center(packet_position, direction)
}

fn painting_center(
    packet_position: EntityVec3,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> Option<EntityVec3> {
    let direction = hanging_direction(add_entity_data, data_values);
    let (width, height) = painting_variant_size(data_values);
    let left = horizontal_counter_clockwise(direction)?;
    let mut center = hanging_wall_center(packet_position, direction);
    if (width as i32) % 2 == 0 {
        center.x += f64::from(left.step()[0]) * 0.5;
        center.z += f64::from(left.step()[2]) * 0.5;
    }
    if (height as i32) % 2 == 0 {
        center.y += 0.5;
    }
    Some(center)
}

fn leash_knot_position(packet_position: EntityVec3) -> EntityVec3 {
    EntityVec3 {
        x: block_coord(packet_position.x) + 0.5,
        y: block_coord(packet_position.y) + 0.375,
        z: block_coord(packet_position.z) + 0.5,
    }
}

fn hanging_wall_center(packet_position: EntityVec3, direction: VanillaDirection) -> EntityVec3 {
    let step = direction.step();
    EntityVec3 {
        x: block_coord(packet_position.x) + 0.5 - f64::from(step[0]) * HANGING_WALL_OFFSET,
        y: block_coord(packet_position.y) + 0.5 - f64::from(step[1]) * HANGING_WALL_OFFSET,
        z: block_coord(packet_position.z) + 0.5 - f64::from(step[2]) * HANGING_WALL_OFFSET,
    }
}

fn block_coord(value: f64) -> f64 {
    value.floor()
}

fn item_frame_size_for_direction(
    direction: VanillaDirection,
    frame_size: f32,
    depth: f32,
) -> (f32, f32, f32) {
    match direction.axis() {
        VanillaAxis::X => (depth, frame_size, frame_size),
        VanillaAxis::Y => (frame_size, depth, frame_size),
        VanillaAxis::Z => (frame_size, frame_size, depth),
    }
}

fn painting_size_for_direction(
    direction: VanillaDirection,
    width: f32,
    height: f32,
) -> Option<(f32, f32, f32)> {
    match direction.axis() {
        VanillaAxis::X => Some((PAINTING_DEPTH, height, width)),
        VanillaAxis::Z => Some((width, height, PAINTING_DEPTH)),
        VanillaAxis::Y => None,
    }
}

fn hanging_direction(add_entity_data: i32, data_values: &[EntityDataValue]) -> VanillaDirection {
    data_values
        .iter()
        .find(|value| value.data_id == HANGING_DATA_DIRECTION_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Direction(value) => Some(vanilla_direction_from_3d_data(*value)),
            _ => None,
        })
        .unwrap_or_else(|| vanilla_direction_from_3d_data(add_entity_data))
}

fn item_frame_has_framed_map(data_values: &[EntityDataValue]) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == ITEM_FRAME_DATA_ITEM_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::ItemStack(item) => Some(item_stack_has_map_id(item)),
            _ => None,
        })
        .unwrap_or(false)
}

fn item_stack_has_map_id(item: &ItemStackSummary) -> bool {
    item.component_patch
        .added_type_ids
        .contains(&MAP_ID_DATA_COMPONENT_TYPE_ID)
        && !item
            .component_patch
            .removed_type_ids
            .contains(&MAP_ID_DATA_COMPONENT_TYPE_ID)
}

fn painting_variant_size(data_values: &[EntityDataValue]) -> (f32, f32) {
    data_values
        .iter()
        .find(|value| value.data_id == PAINTING_DATA_VARIANT_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::PaintingVariant(variant) => {
                if let Some(direct) = &variant.direct {
                    return positive_painting_size(direct.width, direct.height);
                }
                variant
                    .registry_id
                    .and_then(|id| PAINTING_VARIANT_SIZES.get(id as usize).copied())
            }
            _ => None,
        })
        .unwrap_or((1.0, 1.0))
}

fn positive_painting_size(width: i32, height: i32) -> Option<(f32, f32)> {
    (width > 0 && height > 0).then_some((width as f32, height as f32))
}

fn slime_pick_bounds(data_values: &[EntityDataValue]) -> EntityPickBoundsState {
    let size = entity_data_int(data_values, SLIME_SIZE_DATA_ID, SLIME_DEFAULT_SIZE) as f32;
    EntityPickBoundsState::from_base_size(SLIME_BASE_SIZE * size, SLIME_BASE_SIZE * size, 0.0)
}

fn armor_stand_pick_bounds(data_values: &[EntityDataValue]) -> Option<EntityPickBoundsState> {
    let flags = entity_data_byte(data_values, ARMOR_STAND_CLIENT_FLAGS_DATA_ID, 0);
    if flags & ARMOR_STAND_CLIENT_FLAG_MARKER != 0 {
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
    bounds.scale_dimensions(scale)
}

fn vanilla_scale_attribute(attributes: &[AttributeSnapshot]) -> Option<f32> {
    attributes
        .iter()
        .find(|attribute| attribute.attribute_id == VANILLA_ATTRIBUTE_SCALE_ID)
        .map(vanilla_attribute_value)
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

fn sanitize_vanilla_scale(value: f64) -> f64 {
    if value.is_nan() {
        VANILLA_SCALE_MIN
    } else {
        value.clamp(VANILLA_SCALE_MIN, VANILLA_SCALE_MAX)
    }
}

fn vanilla_living_entity_type(entity_type_id: i32) -> bool {
    VANILLA_LIVING_ENTITY_TYPE_IDS
        .binary_search(&entity_type_id)
        .is_ok()
}

fn scales_with_living_scale_attribute(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    vanilla_living_entity_type(entity_type_id)
        && !(is_living_sleeping(entity_type_id, data_values)
            || is_avatar_dying_pose(entity_type_id, data_values))
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

fn entity_data_pose(data_values: &[EntityDataValue]) -> i32 {
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
}

fn vanilla_direction_from_3d_data(data: i32) -> VanillaDirection {
    match (data % 6).abs() {
        0 => VanillaDirection::Down,
        1 => VanillaDirection::Up,
        2 => VanillaDirection::North,
        3 => VanillaDirection::South,
        4 => VanillaDirection::West,
        _ => VanillaDirection::East,
    }
}

fn horizontal_counter_clockwise(direction: VanillaDirection) -> Option<VanillaDirection> {
    match direction {
        VanillaDirection::North => Some(VanillaDirection::West),
        VanillaDirection::South => Some(VanillaDirection::East),
        VanillaDirection::West => Some(VanillaDirection::South),
        VanillaDirection::East => Some(VanillaDirection::North),
        VanillaDirection::Down | VanillaDirection::Up => None,
    }
}

const fn pick(width: f32, height: f32, pick_radius: f32) -> EntityPickBoundsState {
    EntityPickBoundsState::from_base_size(width, height, pick_radius)
}

// IDs follow the PaintingVariants.java bootstrap registration order.
const PAINTING_VARIANT_SIZES: &[(f32, f32)] = &[
    (1.0, 1.0), // kebab
    (1.0, 1.0), // aztec
    (1.0, 1.0), // alban
    (1.0, 1.0), // aztec2
    (1.0, 1.0), // bomb
    (1.0, 1.0), // plant
    (1.0, 1.0), // wasteland
    (2.0, 1.0), // pool
    (2.0, 1.0), // courbet
    (2.0, 1.0), // sea
    (2.0, 1.0), // sunset
    (2.0, 1.0), // creebet
    (1.0, 2.0), // wanderer
    (1.0, 2.0), // graham
    (2.0, 2.0), // match
    (2.0, 2.0), // bust
    (2.0, 2.0), // stage
    (2.0, 2.0), // void
    (2.0, 2.0), // skull_and_roses
    (2.0, 2.0), // wither
    (4.0, 2.0), // fighters
    (4.0, 4.0), // pointer
    (4.0, 4.0), // pigscene
    (4.0, 4.0), // burning_skull
    (4.0, 3.0), // skeleton
    (2.0, 2.0), // earth
    (2.0, 2.0), // wind
    (2.0, 2.0), // water
    (2.0, 2.0), // fire
    (4.0, 3.0), // donkey_kong
    (2.0, 2.0), // baroque
    (2.0, 2.0), // humble
    (1.0, 1.0), // meditative
    (1.0, 2.0), // prairie_ride
    (4.0, 4.0), // unpacked
    (3.0, 4.0), // backyard
    (3.0, 3.0), // bouquet
    (3.0, 3.0), // cavebird
    (4.0, 2.0), // changing
    (3.0, 3.0), // cotan
    (3.0, 3.0), // endboss
    (3.0, 3.0), // fern
    (4.0, 2.0), // finding
    (4.0, 2.0), // lowmist
    (4.0, 4.0), // orb
    (3.0, 3.0), // owlemons
    (4.0, 2.0), // passage
    (3.0, 4.0), // pond
    (3.0, 3.0), // sunflowers
    (3.0, 3.0), // tides
    (3.0, 3.0), // dennis
];

// IDs are the vanilla 26.1 EntityType registry order from EntityType.java.
const VANILLA_ENTITY_PICK_BOUNDS: &[(i32, EntityPickBoundsState)] = &[
    (0, pick(1.375, 0.5625, 0.0)),    // minecraft:acacia_boat
    (1, pick(1.375, 0.5625, 0.0)),    // minecraft:acacia_chest_boat
    (2, pick(0.35, 0.6, 0.0)),        // minecraft:allay
    (4, pick(0.7, 0.65, 0.0)),        // minecraft:armadillo
    (5, pick(0.5, 1.975, 0.0)),       // minecraft:armor_stand
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
    (38, pick(0.6, 1.95, 0.0)),       // minecraft:drowned
    (40, pick(1.9975, 1.9975, 0.0)),  // minecraft:elder_guardian
    (41, pick(0.6, 2.9, 0.0)),        // minecraft:enderman
    (42, pick(0.4, 0.3, 0.0)),        // minecraft:endermite
    (45, pick(2.0, 2.0, 0.0)),        // minecraft:end_crystal
    (46, pick(0.6, 1.95, 0.0)),       // minecraft:evoker
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
