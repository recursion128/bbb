use bbb_control::PlayerPose;
use bbb_protocol::packets::{
    BlockHitResult as ProtocolBlockHitResult, BlockPos as ProtocolBlockPos,
    Direction as ProtocolDirection,
};
use bbb_renderer::{CameraPose, SelectionOutline};
use bbb_world::{BlockPos, WorldStore};

const SELECTION_MAX_DISTANCE: f64 = 5.0;
const SELECTION_RAY_STEP: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CrosshairBlockHit {
    pub(crate) pos: BlockPos,
    pub(crate) face: ProtocolDirection,
    pub(crate) cursor: [f32; 3],
    pub(crate) inside: bool,
}

pub(crate) fn selection_outline_from_crosshair(
    world: &WorldStore,
    pose: Option<PlayerPose>,
) -> Option<SelectionOutline> {
    let hit = crosshair_block_hit_from_world(world, pose)?;
    Some(selection_outline_for_block(hit.pos))
}

pub(crate) fn crosshair_block_hit_from_world(
    world: &WorldStore,
    pose: Option<PlayerPose>,
) -> Option<CrosshairBlockHit> {
    raycast_crosshair_block_hit(pose?, SELECTION_MAX_DISTANCE, SELECTION_RAY_STEP, |pos| {
        world.probe_block(pos).map(|probe| probe.material)
    })
}

fn raycast_crosshair_block<F>(
    pose: PlayerPose,
    max_distance: f64,
    step: f64,
    material_at: F,
) -> Option<BlockPos>
where
    F: FnMut(BlockPos) -> Option<bbb_world::TerrainMaterialClass>,
{
    raycast_crosshair_block_hit(pose, max_distance, step, material_at).map(|hit| hit.pos)
}

fn raycast_crosshair_block_hit<F>(
    pose: PlayerPose,
    max_distance: f64,
    step: f64,
    mut material_at: F,
) -> Option<CrosshairBlockHit>
where
    F: FnMut(BlockPos) -> Option<bbb_world::TerrainMaterialClass>,
{
    if max_distance <= 0.0 || step <= 0.0 {
        return None;
    }

    let eye = [
        pose.position.x,
        pose.position.y + f64::from(CameraPose::STANDING_EYE_HEIGHT),
        pose.position.z,
    ];
    let direction = look_direction_from_player_pose(pose);
    if direction == [0.0, 0.0, 0.0] {
        return None;
    }

    let mut distance = 0.0;
    let mut last_pos = None;
    while distance <= max_distance {
        let pos = BlockPos {
            x: (eye[0] + direction[0] * distance).floor() as i32,
            y: (eye[1] + direction[1] * distance).floor() as i32,
            z: (eye[2] + direction[2] * distance).floor() as i32,
        };
        if last_pos != Some(pos) {
            if material_at(pos).is_some_and(is_selectable_crosshair_material) {
                return Some(CrosshairBlockHit {
                    pos,
                    face: block_hit_face(last_pos, pos, direction),
                    cursor: block_hit_cursor(eye, direction, distance, pos),
                    inside: last_pos.is_none(),
                });
            }
            last_pos = Some(pos);
        }
        distance += step;
    }

    None
}

fn block_hit_face(
    previous: Option<BlockPos>,
    current: BlockPos,
    direction: [f64; 3],
) -> ProtocolDirection {
    if let Some(previous) = previous {
        let dx = current.x - previous.x;
        let dy = current.y - previous.y;
        let dz = current.z - previous.z;
        let mut axis = None;
        if dx != 0 {
            axis = Some((0, direction[0].abs(), dx));
        }
        if dy != 0 && axis.is_none_or(|(_, best, _)| direction[1].abs() > best) {
            axis = Some((1, direction[1].abs(), dy));
        }
        if dz != 0 && axis.is_none_or(|(_, best, _)| direction[2].abs() > best) {
            axis = Some((2, direction[2].abs(), dz));
        }
        if let Some((axis, _, delta)) = axis {
            return face_for_axis_delta(axis, delta);
        }
    }
    face_opposing_dominant_direction(direction)
}

fn face_for_axis_delta(axis: u8, delta: i32) -> ProtocolDirection {
    match (axis, delta.signum()) {
        (0, 1) => ProtocolDirection::West,
        (0, -1) => ProtocolDirection::East,
        (1, 1) => ProtocolDirection::Down,
        (1, -1) => ProtocolDirection::Up,
        (2, 1) => ProtocolDirection::North,
        (2, -1) => ProtocolDirection::South,
        _ => ProtocolDirection::North,
    }
}

fn face_opposing_dominant_direction(direction: [f64; 3]) -> ProtocolDirection {
    let ax = direction[0].abs();
    let ay = direction[1].abs();
    let az = direction[2].abs();
    if ax >= ay && ax >= az {
        if direction[0] >= 0.0 {
            ProtocolDirection::West
        } else {
            ProtocolDirection::East
        }
    } else if ay >= az {
        if direction[1] >= 0.0 {
            ProtocolDirection::Down
        } else {
            ProtocolDirection::Up
        }
    } else if direction[2] >= 0.0 {
        ProtocolDirection::North
    } else {
        ProtocolDirection::South
    }
}

fn block_hit_cursor(eye: [f64; 3], direction: [f64; 3], distance: f64, pos: BlockPos) -> [f32; 3] {
    [
        ((eye[0] + direction[0] * distance) - f64::from(pos.x)).clamp(0.0, 1.0) as f32,
        ((eye[1] + direction[1] * distance) - f64::from(pos.y)).clamp(0.0, 1.0) as f32,
        ((eye[2] + direction[2] * distance) - f64::from(pos.z)).clamp(0.0, 1.0) as f32,
    ]
}

pub(crate) fn protocol_block_pos_from_world(pos: BlockPos) -> ProtocolBlockPos {
    ProtocolBlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

pub(crate) fn protocol_block_hit_result_from_crosshair_hit(
    hit: CrosshairBlockHit,
) -> ProtocolBlockHitResult {
    ProtocolBlockHitResult {
        pos: protocol_block_pos_from_world(hit.pos),
        direction: hit.face,
        cursor_x: hit.cursor[0],
        cursor_y: hit.cursor[1],
        cursor_z: hit.cursor[2],
        inside: hit.inside,
        world_border_hit: false,
    }
}

fn look_direction_from_player_pose(pose: PlayerPose) -> [f64; 3] {
    let yaw = f64::from(pose.y_rot).to_radians();
    let pitch = f64::from(pose.x_rot).to_radians();
    let cos_pitch = pitch.cos();
    let x = -yaw.sin() * cos_pitch;
    let y = -pitch.sin();
    let z = yaw.cos() * cos_pitch;
    let len = (x * x + y * y + z * z).sqrt();
    if len <= f64::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [x / len, y / len, z / len]
    }
}

fn is_selectable_crosshair_material(material: bbb_world::TerrainMaterialClass) -> bool {
    matches!(
        material,
        bbb_world::TerrainMaterialClass::Opaque
            | bbb_world::TerrainMaterialClass::Cutout
            | bbb_world::TerrainMaterialClass::Translucent
    )
}

fn selection_outline_for_block(pos: BlockPos) -> SelectionOutline {
    SelectionOutline {
        min: [pos.x as f32, pos.y as f32, pos.z as f32],
        max: [(pos.x + 1) as f32, (pos.y + 1) as f32, (pos.z + 1) as f32],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_control::NetVec3;

    #[test]
    fn crosshair_raycast_hits_first_selectable_block() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block(pose, 5.0, 0.05, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                Some(bbb_world::TerrainMaterialClass::Empty)
            }
        });

        assert_eq!(hit, Some(BlockPos { x: 0, y: 1, z: 3 }));
    }

    #[test]
    fn crosshair_raycast_reports_hit_face() {
        let pose = player_pose(0.0, 0.0, 0.0);

        let hit = raycast_crosshair_block_hit(pose, 5.0, 1.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                None
            }
        });

        assert_eq!(
            hit,
            Some(CrosshairBlockHit {
                pos: BlockPos { x: 0, y: 1, z: 3 },
                face: ProtocolDirection::North,
                cursor: [0.0, 0.62, 0.0],
                inside: false,
            })
        );
    }

    #[test]
    fn crosshair_raycast_ignores_fluid_blocks() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block(pose, 5.0, 0.05, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 2 }) {
                Some(bbb_world::TerrainMaterialClass::Fluid)
            } else if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                Some(bbb_world::TerrainMaterialClass::Empty)
            }
        });

        assert_eq!(hit, Some(BlockPos { x: 0, y: 1, z: 3 }));
    }

    #[test]
    fn selection_outline_uses_block_bounds() {
        assert_eq!(
            selection_outline_for_block(BlockPos { x: -2, y: 63, z: 4 }),
            SelectionOutline {
                min: [-2.0, 63.0, 4.0],
                max: [-1.0, 64.0, 5.0],
            }
        );
    }

    fn player_pose(y_rot: f32, x_rot: f32, z: f64) -> PlayerPose {
        PlayerPose {
            position: NetVec3 { x: 0.0, y: 0.0, z },
            y_rot,
            x_rot,
            ..PlayerPose::default()
        }
    }
}
