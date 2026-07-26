use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
    packets::{EntityDataValue, EntityDataValueKind, ItemStackSummary},
};

use crate::entities::{
    schema::{VanillaHangingEntityMetadataAccessor, VanillaItemFrameMetadataAccessors},
    EntityVec3, ItemFrameFacing,
};
use crate::{BlockPos, PaintingVariantClientProjection};

use super::{
    entity_data_direction, vanilla_direction_from_3d_data, EntityPickBoundsState, VanillaAxis,
    VanillaDirection,
};

const ITEM_FRAME_DEPTH: f32 = 0.0625;
const ITEM_FRAME_DEFAULT_SIZE: f32 = 0.75;
const ITEM_FRAME_MAP_SIZE: f32 = 1.0;
const PAINTING_DEPTH: f32 = 0.0625;
const HANGING_WALL_OFFSET: f64 = 0.46875;

pub(crate) fn prospective_item_frame_geometry(
    pos: BlockPos,
    facing: ItemFrameFacing,
) -> (EntityVec3, EntityPickBoundsState) {
    let direction = direction_from_item_frame_facing(facing);
    let center = hanging_wall_center(block_position(pos), direction);
    let (x_size, y_size, z_size) =
        item_frame_size_for_direction(direction, ITEM_FRAME_DEFAULT_SIZE, ITEM_FRAME_DEPTH);
    (
        center,
        EntityPickBoundsState::from_centered_size(x_size, y_size, z_size, 0.0),
    )
}

pub(crate) fn prospective_painting_geometry(
    pos: BlockPos,
    facing: ItemFrameFacing,
    width: i32,
    height: i32,
) -> Option<(EntityVec3, EntityPickBoundsState)> {
    if !(1..=16).contains(&width) || !(1..=16).contains(&height) {
        return None;
    }
    let direction = direction_from_item_frame_facing(facing);
    let (x_size, y_size, z_size) =
        painting_size_for_direction(direction, width as f32, height as f32)?;
    let mut center = hanging_wall_center(block_position(pos), direction);
    let left = horizontal_counter_clockwise(direction)?;
    if width % 2 == 0 {
        center.x += f64::from(left.step()[0]) * 0.5;
        center.z += f64::from(left.step()[2]) * 0.5;
    }
    if height % 2 == 0 {
        center.y += 0.5;
    }
    Some((
        center,
        EntityPickBoundsState::from_centered_size(x_size, y_size, z_size, 0.0),
    ))
}

pub(super) fn item_frame_pick_bounds(
    accessors: VanillaItemFrameMetadataAccessors,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> EntityPickBoundsState {
    let direction = hanging_direction(accessors.direction_data_id, add_entity_data, data_values);
    let frame_size = if item_frame_has_framed_map(accessors, data_values) {
        ITEM_FRAME_MAP_SIZE
    } else {
        ITEM_FRAME_DEFAULT_SIZE
    };
    let (x_size, y_size, z_size) =
        item_frame_size_for_direction(direction, frame_size, ITEM_FRAME_DEPTH);
    EntityPickBoundsState::from_centered_size(x_size, y_size, z_size, 0.0)
}

pub(crate) fn hanging_facing(
    accessor: VanillaHangingEntityMetadataAccessor,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> ItemFrameFacing {
    match hanging_direction(accessor.direction_data_id, add_entity_data, data_values) {
        VanillaDirection::Down => ItemFrameFacing::Down,
        VanillaDirection::Up => ItemFrameFacing::Up,
        VanillaDirection::North => ItemFrameFacing::North,
        VanillaDirection::South => ItemFrameFacing::South,
        VanillaDirection::West => ItemFrameFacing::West,
        VanillaDirection::East => ItemFrameFacing::East,
    }
}

pub(super) fn painting_pick_bounds(
    accessor: VanillaHangingEntityMetadataAccessor,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    variant: Option<&PaintingVariantClientProjection>,
) -> Option<EntityPickBoundsState> {
    let direction = hanging_direction(accessor.direction_data_id, add_entity_data, data_values);
    let (width, height) = painting_variant_size(variant);
    let (x_size, y_size, z_size) = painting_size_for_direction(direction, width, height)?;
    Some(EntityPickBoundsState::from_centered_size(
        x_size, y_size, z_size, 0.0,
    ))
}

pub(super) fn item_frame_center(
    accessors: VanillaItemFrameMetadataAccessors,
    packet_position: EntityVec3,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> EntityVec3 {
    let direction = hanging_direction(accessors.direction_data_id, add_entity_data, data_values);
    hanging_wall_center(packet_position, direction)
}

pub(super) fn painting_center(
    accessor: VanillaHangingEntityMetadataAccessor,
    packet_position: EntityVec3,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    variant: Option<&PaintingVariantClientProjection>,
) -> Option<EntityVec3> {
    let direction = hanging_direction(accessor.direction_data_id, add_entity_data, data_values);
    let (width, height) = painting_variant_size(variant);
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

fn block_position(pos: BlockPos) -> EntityVec3 {
    EntityVec3 {
        x: f64::from(pos.x),
        y: f64::from(pos.y),
        z: f64::from(pos.z),
    }
}

fn direction_from_item_frame_facing(facing: ItemFrameFacing) -> VanillaDirection {
    match facing {
        ItemFrameFacing::Down => VanillaDirection::Down,
        ItemFrameFacing::Up => VanillaDirection::Up,
        ItemFrameFacing::North => VanillaDirection::North,
        ItemFrameFacing::South => VanillaDirection::South,
        ItemFrameFacing::West => VanillaDirection::West,
        ItemFrameFacing::East => VanillaDirection::East,
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

fn hanging_direction(
    direction_data_id: u8,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> VanillaDirection {
    entity_data_direction(data_values, direction_data_id)
        .unwrap_or_else(|| vanilla_direction_from_3d_data(add_entity_data))
}

/// The wall the item frame faces (vanilla `ItemFrame.getDirection`), from the synched direction or the
/// spawn-packet `data` fallback.
pub(crate) fn item_frame_facing(
    accessors: VanillaItemFrameMetadataAccessors,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> ItemFrameFacing {
    hanging_facing(
        VanillaHangingEntityMetadataAccessor {
            direction_data_id: accessors.direction_data_id,
            direction_default: accessors.direction_default,
        },
        add_entity_data,
        data_values,
    )
}

/// The horizontal wall the painting faces, from the synchronized hanging direction or the spawn
/// packet `data` fallback. Vanilla rejects vertical painting directions.
pub(crate) fn painting_facing(
    accessor: VanillaHangingEntityMetadataAccessor,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
) -> Option<ItemFrameFacing> {
    match hanging_direction(accessor.direction_data_id, add_entity_data, data_values) {
        VanillaDirection::North => Some(ItemFrameFacing::North),
        VanillaDirection::South => Some(ItemFrameFacing::South),
        VanillaDirection::West => Some(ItemFrameFacing::West),
        VanillaDirection::East => Some(ItemFrameFacing::East),
        VanillaDirection::Down | VanillaDirection::Up => None,
    }
}

/// The `0..=7` item rotation in the frame (vanilla `ItemFrame.getRotation`, `DATA_ROTATION % 8`), or `0`
/// when the entity has not sent the rotation datum.
pub(crate) fn item_frame_rotation(
    accessors: VanillaItemFrameMetadataAccessors,
    data_values: &[EntityDataValue],
) -> u8 {
    data_values
        .iter()
        .find(|value| value.data_id == accessors.rotation_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Int(rotation) => Some(rotation.rem_euclid(8) as u8),
            _ => None,
        })
        .unwrap_or(accessors.rotation_default as u8)
}

/// The non-empty item displayed in the frame (vanilla `ItemFrame.getItem`, the `DATA_ITEM` datum), or
/// `None` for an empty frame.
pub(crate) fn item_frame_item(
    accessors: VanillaItemFrameMetadataAccessors,
    data_values: &[EntityDataValue],
) -> Option<&ItemStackSummary> {
    data_values
        .iter()
        .find(|value| value.data_id == accessors.item_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::ItemStack(item) if item.item_id.is_some() && item.count > 0 => {
                Some(item)
            }
            _ => None,
        })
}

fn item_frame_has_framed_map(
    accessors: VanillaItemFrameMetadataAccessors,
    data_values: &[EntityDataValue],
) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == accessors.item_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::ItemStack(item) => Some(item_stack_has_map_component(item)),
            _ => None,
        })
        .unwrap_or(false)
}

pub(crate) fn item_frame_map_id(
    accessors: VanillaItemFrameMetadataAccessors,
    data_values: &[EntityDataValue],
) -> Option<i32> {
    data_values
        .iter()
        .find(|value| value.data_id == accessors.item_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::ItemStack(item) => item_stack_map_id(item),
            _ => None,
        })
}

fn item_stack_map_id(item: &ItemStackSummary) -> Option<i32> {
    if item
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID)
    {
        return None;
    }
    item.component_patch.map_id
}

fn item_stack_has_map_component(item: &ItemStackSummary) -> bool {
    if item
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID)
    {
        return false;
    }
    item.component_patch.map_id.is_some()
        || item
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID)
}

fn painting_variant_size(variant: Option<&PaintingVariantClientProjection>) -> (f32, f32) {
    variant
        .and_then(|variant| positive_painting_size(variant.width, variant.height))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::schema::vanilla_entity_metadata_item_frame_accessors_for_entity_type;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_FRAME_ID;
    use bbb_protocol::packets::DataComponentPatchSummary;

    fn item_frame_accessors() -> VanillaItemFrameMetadataAccessors {
        vanilla_entity_metadata_item_frame_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        )
        .expect("generated ItemFrame accessors")
    }

    fn datum(data_id: u8, serializer_id: i32, value: EntityDataValueKind) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id,
            value,
        }
    }

    fn item(item_id: Option<i32>, count: i32, map: bool) -> ItemStackSummary {
        let mut component_patch = DataComponentPatchSummary::default();
        if map {
            component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID];
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
        let accessors = item_frame_accessors();
        assert_eq!(
            item_frame_rotation(
                accessors,
                &[datum(
                    accessors.rotation_data_id,
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    EntityDataValueKind::Int(5)
                )]
            ),
            5
        );
        // Vanilla `ItemFrame.getRotation` wraps `% 8`.
        assert_eq!(
            item_frame_rotation(
                accessors,
                &[datum(
                    accessors.rotation_data_id,
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    EntityDataValueKind::Int(11)
                )]
            ),
            3
        );
        // A missing datum defaults to 0.
        assert_eq!(item_frame_rotation(accessors, &[]), 0);
    }

    #[test]
    fn facing_reads_direction_then_falls_back_to_spawn_data() {
        let accessors = item_frame_accessors();
        // The synched direction wins (3 = south in the 3D data encoding).
        assert_eq!(
            item_frame_facing(
                accessors,
                0,
                &[datum(
                    accessors.direction_data_id,
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
                    EntityDataValueKind::Direction(3)
                )]
            ),
            ItemFrameFacing::South
        );
        // Without the datum, the spawn-packet `data` fallback is used (2 = north).
        assert_eq!(item_frame_facing(accessors, 2, &[]), ItemFrameFacing::North);
    }

    #[test]
    fn item_returns_only_non_empty_stacks() {
        let accessors = item_frame_accessors();
        assert_eq!(
            item_frame_item(
                accessors,
                &[datum(
                    accessors.item_data_id,
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                    EntityDataValueKind::ItemStack(item(Some(1), 1, false))
                )]
            ),
            Some(&item(Some(1), 1, false))
        );
        // An empty stack (no id / zero count) is treated as no item.
        assert_eq!(
            item_frame_item(
                accessors,
                &[datum(
                    accessors.item_data_id,
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                    EntityDataValueKind::ItemStack(ItemStackSummary::empty())
                )]
            ),
            None
        );
        assert_eq!(item_frame_item(accessors, &[]), None);
    }

    #[test]
    fn map_id_detects_the_map_id_component() {
        let accessors = item_frame_accessors();
        assert_eq!(
            item_frame_map_id(
                accessors,
                &[datum(
                    accessors.item_data_id,
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                    EntityDataValueKind::ItemStack(item(Some(2), 1, true))
                )]
            ),
            Some(7)
        );
        assert_eq!(
            item_frame_map_id(
                accessors,
                &[datum(
                    accessors.item_data_id,
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                    EntityDataValueKind::ItemStack(item(Some(2), 1, false))
                )]
            ),
            None
        );
    }
}
