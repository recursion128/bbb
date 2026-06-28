use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, ItemStackSummary};

use crate::entities::{EntityVec3, ItemFrameFacing};

use super::{
    entity_data_direction, vanilla_direction_from_3d_data, EntityPickBoundsState, VanillaAxis,
    VanillaDirection,
};

const HANGING_DATA_DIRECTION_ID: u8 = 8;
const ITEM_FRAME_DATA_ITEM_ID: u8 = 9;
const ITEM_FRAME_DATA_ROTATION_ID: u8 = 10;
const PAINTING_DATA_VARIANT_ID: u8 = 9;
const ITEM_FRAME_DEPTH: f32 = 0.0625;
const ITEM_FRAME_DEFAULT_SIZE: f32 = 0.75;
const ITEM_FRAME_MAP_SIZE: f32 = 1.0;
const MAP_ID_DATA_COMPONENT_TYPE_ID: i32 = 41;
const PAINTING_DEPTH: f32 = 0.0625;
const HANGING_WALL_OFFSET: f64 = 0.46875;

pub(super) fn item_frame_pick_bounds(
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

pub(super) fn painting_pick_bounds(
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

pub(super) fn item_frame_center(
    packet_position: EntityVec3,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> EntityVec3 {
    let direction = hanging_direction(add_entity_data, data_values);
    hanging_wall_center(packet_position, direction)
}

pub(super) fn painting_center(
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

pub(super) fn leash_knot_position(packet_position: EntityVec3) -> EntityVec3 {
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
    entity_data_direction(data_values, HANGING_DATA_DIRECTION_ID)
        .unwrap_or_else(|| vanilla_direction_from_3d_data(add_entity_data))
}

/// The wall the item frame faces (vanilla `ItemFrame.getDirection`), from the synched direction or the
/// spawn-packet `data` fallback.
pub(crate) fn item_frame_facing(
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> ItemFrameFacing {
    match hanging_direction(add_entity_data, data_values) {
        VanillaDirection::Down => ItemFrameFacing::Down,
        VanillaDirection::Up => ItemFrameFacing::Up,
        VanillaDirection::North => ItemFrameFacing::North,
        VanillaDirection::South => ItemFrameFacing::South,
        VanillaDirection::West => ItemFrameFacing::West,
        VanillaDirection::East => ItemFrameFacing::East,
    }
}

/// The `0..=7` item rotation in the frame (vanilla `ItemFrame.getRotation`, `DATA_ROTATION % 8`), or `0`
/// when the entity has not sent the rotation datum.
pub(crate) fn item_frame_rotation(data_values: &[EntityDataValue]) -> u8 {
    data_values
        .iter()
        .find(|value| value.data_id == ITEM_FRAME_DATA_ROTATION_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Int(rotation) => Some(rotation.rem_euclid(8) as u8),
            _ => None,
        })
        .unwrap_or(0)
}

/// The non-empty item displayed in the frame (vanilla `ItemFrame.getItem`, the `DATA_ITEM` datum), or
/// `None` for an empty frame.
pub(crate) fn item_frame_item(data_values: &[EntityDataValue]) -> Option<&ItemStackSummary> {
    data_values
        .iter()
        .find(|value| value.data_id == ITEM_FRAME_DATA_ITEM_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::ItemStack(item) if item.item_id.is_some() && item.count > 0 => {
                Some(item)
            }
            _ => None,
        })
}

fn item_frame_has_framed_map(data_values: &[EntityDataValue]) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == ITEM_FRAME_DATA_ITEM_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::ItemStack(item) => Some(item_stack_has_map_component(item)),
            _ => None,
        })
        .unwrap_or(false)
}

pub(crate) fn item_frame_map_id(data_values: &[EntityDataValue]) -> Option<i32> {
    data_values
        .iter()
        .find(|value| value.data_id == ITEM_FRAME_DATA_ITEM_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::ItemStack(item) => item_stack_map_id(item),
            _ => None,
        })
}

fn item_stack_map_id(item: &ItemStackSummary) -> Option<i32> {
    if item
        .component_patch
        .removed_type_ids
        .contains(&MAP_ID_DATA_COMPONENT_TYPE_ID)
    {
        return None;
    }
    item.component_patch.map_id
}

fn item_stack_has_map_component(item: &ItemStackSummary) -> bool {
    if item
        .component_patch
        .removed_type_ids
        .contains(&MAP_ID_DATA_COMPONENT_TYPE_ID)
    {
        return false;
    }
    item.component_patch.map_id.is_some()
        || item
            .component_patch
            .added_type_ids
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

fn horizontal_counter_clockwise(direction: VanillaDirection) -> Option<VanillaDirection> {
    match direction {
        VanillaDirection::North => Some(VanillaDirection::West),
        VanillaDirection::South => Some(VanillaDirection::East),
        VanillaDirection::West => Some(VanillaDirection::South),
        VanillaDirection::East => Some(VanillaDirection::North),
        VanillaDirection::Down | VanillaDirection::Up => None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::DataComponentPatchSummary;

    fn datum(data_id: u8, value: EntityDataValueKind) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 0,
            value,
        }
    }

    fn item(item_id: Option<i32>, count: i32, map: bool) -> ItemStackSummary {
        let mut component_patch = DataComponentPatchSummary::default();
        if map {
            component_patch.added_type_ids = vec![MAP_ID_DATA_COMPONENT_TYPE_ID];
            component_patch.map_id = Some(7);
        }
        ItemStackSummary {
            item_id,
            count,
            component_patch,
        }
    }

    #[test]
    fn rotation_reads_datum_and_wraps_mod_8() {
        assert_eq!(
            item_frame_rotation(&[datum(
                ITEM_FRAME_DATA_ROTATION_ID,
                EntityDataValueKind::Int(5)
            )]),
            5
        );
        // Vanilla `ItemFrame.getRotation` wraps `% 8`.
        assert_eq!(
            item_frame_rotation(&[datum(
                ITEM_FRAME_DATA_ROTATION_ID,
                EntityDataValueKind::Int(11)
            )]),
            3
        );
        // A missing datum defaults to 0.
        assert_eq!(item_frame_rotation(&[]), 0);
    }

    #[test]
    fn facing_reads_direction_then_falls_back_to_spawn_data() {
        // The synched direction wins (3 = south in the 3D data encoding).
        assert_eq!(
            item_frame_facing(
                0,
                &[datum(
                    HANGING_DATA_DIRECTION_ID,
                    EntityDataValueKind::Direction(3)
                )]
            ),
            ItemFrameFacing::South
        );
        // Without the datum, the spawn-packet `data` fallback is used (2 = north).
        assert_eq!(item_frame_facing(2, &[]), ItemFrameFacing::North);
    }

    #[test]
    fn item_returns_only_non_empty_stacks() {
        assert_eq!(
            item_frame_item(&[datum(
                ITEM_FRAME_DATA_ITEM_ID,
                EntityDataValueKind::ItemStack(item(Some(1), 1, false))
            )]),
            Some(&item(Some(1), 1, false))
        );
        // An empty stack (no id / zero count) is treated as no item.
        assert_eq!(
            item_frame_item(&[datum(
                ITEM_FRAME_DATA_ITEM_ID,
                EntityDataValueKind::ItemStack(ItemStackSummary::empty())
            )]),
            None
        );
        assert_eq!(item_frame_item(&[]), None);
    }

    #[test]
    fn map_id_detects_the_map_id_component() {
        assert_eq!(
            item_frame_map_id(&[datum(
                ITEM_FRAME_DATA_ITEM_ID,
                EntityDataValueKind::ItemStack(item(Some(2), 1, true))
            )]),
            Some(7)
        );
        assert_eq!(
            item_frame_map_id(&[datum(
                ITEM_FRAME_DATA_ITEM_ID,
                EntityDataValueKind::ItemStack(item(Some(2), 1, false))
            )]),
            None
        );
    }
}
