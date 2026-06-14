use super::{EntityPickBoundsState, EntityPickTargetState, EntityTransform, EntityVec3};

pub(crate) const VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID: i32 = 43;

const ENDER_DRAGON_PARTS: &[EnderDragonPartSpec] = &[
    EnderDragonPartSpec {
        id_offset: 1,
        width: 1.0,
        height: 1.0,
        offset: EnderDragonPartOffset::Head,
    },
    EnderDragonPartSpec {
        id_offset: 2,
        width: 3.0,
        height: 3.0,
        offset: EnderDragonPartOffset::Neck,
    },
    EnderDragonPartSpec {
        id_offset: 3,
        width: 5.0,
        height: 3.0,
        offset: EnderDragonPartOffset::Body,
    },
    EnderDragonPartSpec {
        id_offset: 4,
        width: 2.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Tail(0),
    },
    EnderDragonPartSpec {
        id_offset: 5,
        width: 2.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Tail(1),
    },
    EnderDragonPartSpec {
        id_offset: 6,
        width: 2.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Tail(2),
    },
    EnderDragonPartSpec {
        id_offset: 7,
        width: 4.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Wing(1.0),
    },
    EnderDragonPartSpec {
        id_offset: 8,
        width: 4.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Wing(-1.0),
    },
];

#[derive(Debug, Clone, Copy)]
struct EnderDragonPartSpec {
    id_offset: i32,
    width: f32,
    height: f32,
    offset: EnderDragonPartOffset,
}

#[derive(Debug, Clone, Copy)]
enum EnderDragonPartOffset {
    Head,
    Neck,
    Body,
    Tail(u8),
    Wing(f64),
}

pub(crate) fn ender_dragon_part_pick_targets(
    parent_id: i32,
    transform: EntityTransform,
) -> Vec<EntityPickTargetState> {
    ENDER_DRAGON_PARTS
        .iter()
        .map(|part| EntityPickTargetState {
            entity_id: parent_id + part.id_offset,
            position: part_position(transform.position, part.offset, transform.y_rot),
            bounds: EntityPickBoundsState::from_base_size(part.width, part.height, 0.0),
        })
        .collect()
}

fn part_position(
    parent_position: EntityVec3,
    offset: EnderDragonPartOffset,
    y_rot: f32,
) -> EntityVec3 {
    let yaw = f64::from(y_rot).to_radians();
    let sin_yaw = yaw.sin();
    let cos_yaw = yaw.cos();
    let (x, y, z) = match offset {
        EnderDragonPartOffset::Head => (sin_yaw * 6.5, 0.0, -cos_yaw * 6.5),
        EnderDragonPartOffset::Neck => (sin_yaw * 5.5, 0.0, -cos_yaw * 5.5),
        EnderDragonPartOffset::Body => (sin_yaw * 0.5, 0.0, -cos_yaw * 0.5),
        EnderDragonPartOffset::Tail(index) => {
            let distance = 1.5 + f64::from(index + 1) * 2.0;
            (-sin_yaw * distance, 1.5, cos_yaw * distance)
        }
        EnderDragonPartOffset::Wing(side) => (cos_yaw * 4.5 * side, 2.0, sin_yaw * 4.5 * side),
    };
    EntityVec3 {
        x: parent_position.x + x,
        y: parent_position.y + y,
        z: parent_position.z + z,
    }
}
