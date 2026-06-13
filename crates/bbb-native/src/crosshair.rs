use std::collections::BTreeMap;

use bbb_control::PlayerPose;
use bbb_protocol::packets::{
    BlockHitResult as ProtocolBlockHitResult, BlockPos as ProtocolBlockPos,
    Direction as ProtocolDirection,
};
use bbb_renderer::{CameraPose, SelectionOutline};
use bbb_world::{BlockPos, BlockProbe, TerrainMaterialClass, WorldStore};

const SELECTION_MAX_DISTANCE: f64 = 4.5;

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
    match world.probe_block(hit.pos) {
        Some(probe) => selection_outline_for_probe(&probe),
        None => Some(selection_outline_for_block(hit.pos)),
    }
}

pub(crate) fn crosshair_block_hit_from_world(
    world: &WorldStore,
    pose: Option<PlayerPose>,
) -> Option<CrosshairBlockHit> {
    raycast_crosshair_block_hit(pose?, SELECTION_MAX_DISTANCE, |pos| {
        world
            .probe_block(pos)
            .map(|probe| crosshair_target_from_probe(&probe))
    })
}

fn raycast_crosshair_block<F>(
    pose: PlayerPose,
    max_distance: f64,
    mut material_at: F,
) -> Option<BlockPos>
where
    F: FnMut(BlockPos) -> Option<bbb_world::TerrainMaterialClass>,
{
    raycast_crosshair_block_hit(pose, max_distance, |pos| {
        material_at(pos).map(CrosshairBlockTarget::full_block)
    })
    .map(|hit| hit.pos)
}

fn raycast_crosshair_block_hit<F>(
    pose: PlayerPose,
    max_distance: f64,
    mut target_at: F,
) -> Option<CrosshairBlockHit>
where
    F: FnMut(BlockPos) -> Option<CrosshairBlockTarget>,
{
    if max_distance <= 0.0 {
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

    let mut pos = block_pos_containing(eye);
    if let Some(hit) =
        target_at(pos).and_then(|target| target.clip(eye, direction, max_distance, pos))
    {
        return Some(CrosshairBlockHit {
            pos,
            face: hit.face,
            cursor: block_hit_cursor(eye, direction, hit.distance, pos),
            inside: hit.inside,
        });
    }

    let mut cursor = RayGridCursor::new(eye, direction);
    while let Some(step) = cursor.next_step(max_distance) {
        pos = offset_block_pos_axis(pos, step.axis, step.delta);
        if let Some(hit) =
            target_at(pos).and_then(|target| target.clip(eye, direction, max_distance, pos))
        {
            return Some(CrosshairBlockHit {
                pos,
                face: hit.face,
                cursor: block_hit_cursor(eye, direction, hit.distance, pos),
                inside: hit.inside,
            });
        }
    }

    None
}

#[derive(Debug, Clone, Copy)]
struct CrosshairBlockTarget {
    material: TerrainMaterialClass,
    outline: Option<BlockOutlineBox>,
}

impl CrosshairBlockTarget {
    fn full_block(material: TerrainMaterialClass) -> Self {
        Self {
            material,
            outline: Some(BlockOutlineBox::FULL),
        }
    }

    fn clip(
        self,
        eye: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        pos: BlockPos,
    ) -> Option<BlockOutlineHit> {
        if !is_selectable_crosshair_material(self.material) {
            return None;
        }
        self.outline?.clip(eye, direction, max_distance, pos)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BlockOutlineBox {
    min: [f64; 3],
    max: [f64; 3],
}

impl BlockOutlineBox {
    const FULL: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    const BOTTOM_SLAB: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 0.5, 1.0],
    };
    const TOP_SLAB: Self = Self {
        min: [0.0, 0.5, 0.0],
        max: [1.0, 1.0, 1.0],
    };

    fn clip(
        self,
        eye: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        pos: BlockPos,
    ) -> Option<BlockOutlineHit> {
        let min = [
            f64::from(pos.x) + self.min[0],
            f64::from(pos.y) + self.min[1],
            f64::from(pos.z) + self.min[2],
        ];
        let max = [
            f64::from(pos.x) + self.max[0],
            f64::from(pos.y) + self.max[1],
            f64::from(pos.z) + self.max[2],
        ];

        if contains_point(min, max, eye) {
            return Some(BlockOutlineHit {
                distance: 0.0,
                face: face_opposing_dominant_direction(direction),
                inside: true,
            });
        }

        let mut entry = f64::NEG_INFINITY;
        let mut exit = f64::INFINITY;
        let mut face = face_opposing_dominant_direction(direction);

        for axis in 0..3 {
            if direction[axis] == 0.0 {
                if eye[axis] < min[axis] || eye[axis] > max[axis] {
                    return None;
                }
                continue;
            }

            let t0 = (min[axis] - eye[axis]) / direction[axis];
            let t1 = (max[axis] - eye[axis]) / direction[axis];
            let (near, far, near_face) = if t0 <= t1 {
                (t0, t1, face_for_axis_delta(axis as u8, 1))
            } else {
                (t1, t0, face_for_axis_delta(axis as u8, -1))
            };

            if near > entry {
                entry = near;
                face = near_face;
            }
            exit = exit.min(far);
            if entry > exit {
                return None;
            }
        }

        if entry < 0.0 || entry > max_distance {
            return None;
        }

        Some(BlockOutlineHit {
            distance: entry,
            face,
            inside: false,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BlockOutlineHit {
    distance: f64,
    face: ProtocolDirection,
    inside: bool,
}

#[derive(Debug, Clone, Copy)]
struct RayGridCursor {
    x: AxisStep,
    y: AxisStep,
    z: AxisStep,
}

impl RayGridCursor {
    fn new(origin: [f64; 3], direction: [f64; 3]) -> Self {
        Self {
            x: AxisStep::new(origin[0], direction[0]),
            y: AxisStep::new(origin[1], direction[1]),
            z: AxisStep::new(origin[2], direction[2]),
        }
    }

    fn next_step(&mut self, max_distance: f64) -> Option<GridStep> {
        let axis = if self.x.next_distance < self.y.next_distance {
            if self.x.next_distance < self.z.next_distance {
                0
            } else {
                2
            }
        } else if self.y.next_distance < self.z.next_distance {
            1
        } else {
            2
        };

        let step = match axis {
            0 => self.x.advance(axis),
            1 => self.y.advance(axis),
            _ => self.z.advance(axis),
        };
        if step.distance <= max_distance {
            Some(step)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct AxisStep {
    delta: i32,
    next_distance: f64,
    distance_delta: f64,
}

impl AxisStep {
    fn new(origin: f64, direction: f64) -> Self {
        let delta = if direction > 0.0 {
            1
        } else if direction < 0.0 {
            -1
        } else {
            0
        };
        if delta == 0 {
            return Self {
                delta,
                next_distance: f64::INFINITY,
                distance_delta: f64::INFINITY,
            };
        }

        let boundary = if delta > 0 {
            origin.floor() + 1.0
        } else {
            origin.floor()
        };
        let next_distance = (boundary - origin) / direction;
        Self {
            delta,
            next_distance: next_distance.max(0.0),
            distance_delta: 1.0 / direction.abs(),
        }
    }

    fn advance(&mut self, axis: u8) -> GridStep {
        let step = GridStep {
            axis,
            delta: self.delta,
            distance: self.next_distance,
        };
        self.next_distance += self.distance_delta;
        step
    }
}

#[derive(Debug, Clone, Copy)]
struct GridStep {
    axis: u8,
    delta: i32,
    distance: f64,
}

fn offset_block_pos_axis(pos: BlockPos, axis: u8, delta: i32) -> BlockPos {
    match axis {
        0 => BlockPos {
            x: pos.x + delta,
            ..pos
        },
        1 => BlockPos {
            y: pos.y + delta,
            ..pos
        },
        _ => BlockPos {
            z: pos.z + delta,
            ..pos
        },
    }
}

fn block_pos_containing(point: [f64; 3]) -> BlockPos {
    BlockPos {
        x: point[0].floor() as i32,
        y: point[1].floor() as i32,
        z: point[2].floor() as i32,
    }
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

fn contains_point(min: [f64; 3], max: [f64; 3], point: [f64; 3]) -> bool {
    (0..3).all(|axis| point[axis] >= min[axis] && point[axis] <= max[axis])
}

fn crosshair_target_from_probe(probe: &BlockProbe) -> CrosshairBlockTarget {
    CrosshairBlockTarget {
        material: probe.material,
        outline: outline_box_for_block(probe.block_name.as_deref(), &probe.block_properties),
    }
}

fn outline_box_for_block(
    block_name: Option<&str>,
    properties: &BTreeMap<String, String>,
) -> Option<BlockOutlineBox> {
    let block_name = block_name?;
    if block_name == "minecraft:snow" {
        return snow_layer_outline_box(properties);
    }
    if is_slab_block_name(block_name) {
        return match properties.get("type").map(String::as_str) {
            Some("bottom") => Some(BlockOutlineBox::BOTTOM_SLAB),
            Some("top") => Some(BlockOutlineBox::TOP_SLAB),
            Some("double") => Some(BlockOutlineBox::FULL),
            _ => None,
        };
    }
    Some(BlockOutlineBox::FULL)
}

fn snow_layer_outline_box(properties: &BTreeMap<String, String>) -> Option<BlockOutlineBox> {
    let layers = properties.get("layers")?.parse::<u8>().ok()?;
    if !(1..=8).contains(&layers) {
        return None;
    }
    Some(BlockOutlineBox {
        min: [0.0, 0.0, 0.0],
        max: [1.0, f64::from(layers) / 8.0, 1.0],
    })
}

fn is_slab_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_slab"))
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

fn selection_outline_for_probe(probe: &BlockProbe) -> Option<SelectionOutline> {
    outline_box_for_block(probe.block_name.as_deref(), &probe.block_properties)
        .map(|outline| selection_outline_for_box(probe.pos, outline))
}

fn selection_outline_for_block(pos: BlockPos) -> SelectionOutline {
    selection_outline_for_box(pos, BlockOutlineBox::FULL)
}

fn selection_outline_for_box(pos: BlockPos, outline: BlockOutlineBox) -> SelectionOutline {
    SelectionOutline {
        min: [
            pos.x as f32 + outline.min[0] as f32,
            pos.y as f32 + outline.min[1] as f32,
            pos.z as f32 + outline.min[2] as f32,
        ],
        max: [
            pos.x as f32 + outline.max[0] as f32,
            pos.y as f32 + outline.max[1] as f32,
            pos.z as f32 + outline.max[2] as f32,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_control::NetVec3;

    #[test]
    fn crosshair_raycast_hits_first_selectable_block() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block(pose, 5.0, |pos| {
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

        let hit = raycast_crosshair_block_hit(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(CrosshairBlockTarget::full_block(
                    bbb_world::TerrainMaterialClass::Opaque,
                ))
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
    fn crosshair_raycast_clips_top_slab_outline_shape() {
        let pose = player_pose(0.0, 0.0, 0.0);

        let hit = raycast_crosshair_block_hit(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(CrosshairBlockTarget {
                    material: bbb_world::TerrainMaterialClass::Opaque,
                    outline: Some(BlockOutlineBox::TOP_SLAB),
                })
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
    fn crosshair_raycast_skips_bottom_slab_above_outline_shape() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block_hit(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(CrosshairBlockTarget {
                    material: bbb_world::TerrainMaterialClass::Opaque,
                    outline: Some(BlockOutlineBox::BOTTOM_SLAB),
                })
            } else if pos == (BlockPos { x: 0, y: 1, z: 4 }) {
                Some(CrosshairBlockTarget::full_block(
                    bbb_world::TerrainMaterialClass::Opaque,
                ))
            } else {
                None
            }
        });

        assert_eq!(
            hit,
            Some(CrosshairBlockHit {
                pos: BlockPos { x: 0, y: 1, z: 4 },
                face: ProtocolDirection::North,
                cursor: [0.0, 0.62, 0.0],
                inside: false,
            })
        );
    }

    #[test]
    fn crosshair_raycast_clips_snow_layer_outline_shape() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block_hit(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(CrosshairBlockTarget {
                    material: bbb_world::TerrainMaterialClass::Opaque,
                    outline: snow_layer_outline_box(&snow_properties(5)),
                })
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
    fn crosshair_raycast_skips_snow_layers_above_outline_shape() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block_hit(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(CrosshairBlockTarget {
                    material: bbb_world::TerrainMaterialClass::Opaque,
                    outline: snow_layer_outline_box(&snow_properties(4)),
                })
            } else if pos == (BlockPos { x: 0, y: 1, z: 4 }) {
                Some(CrosshairBlockTarget::full_block(
                    bbb_world::TerrainMaterialClass::Opaque,
                ))
            } else {
                None
            }
        });

        assert_eq!(
            hit,
            Some(CrosshairBlockHit {
                pos: BlockPos { x: 0, y: 1, z: 4 },
                face: ProtocolDirection::North,
                cursor: [0.0, 0.62, 0.0],
                inside: false,
            })
        );
    }

    #[test]
    fn crosshair_raycast_visits_blocks_by_grid_boundary() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 1 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                Some(bbb_world::TerrainMaterialClass::Empty)
            }
        });

        assert_eq!(hit, Some(BlockPos { x: 0, y: 1, z: 1 }));
    }

    #[test]
    fn crosshair_raycast_ignores_fluid_blocks() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block(pose, 5.0, |pos| {
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
    fn crosshair_raycast_uses_vanilla_default_block_interaction_range() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block(pose, SELECTION_MAX_DISTANCE, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 5 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                Some(bbb_world::TerrainMaterialClass::Empty)
            }
        });

        assert_eq!(SELECTION_MAX_DISTANCE, 4.5);
        assert_eq!(hit, None);
    }

    #[test]
    fn outline_box_uses_vanilla_snow_layers_property() {
        assert_eq!(
            outline_box_for_block(Some("minecraft:snow"), &snow_properties(1)),
            Some(BlockOutlineBox {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 0.125, 1.0],
            })
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:snow"), &snow_properties(8)),
            Some(BlockOutlineBox::FULL)
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:snow"), &BTreeMap::new()),
            None
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:snow"), &snow_properties(9)),
            None
        );
    }

    #[test]
    fn outline_box_uses_vanilla_slab_type_property() {
        assert_eq!(
            outline_box_for_block(Some("minecraft:oak_slab"), &slab_properties("bottom"),),
            Some(BlockOutlineBox::BOTTOM_SLAB)
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:smooth_stone_slab"), &slab_properties("top"),),
            Some(BlockOutlineBox::TOP_SLAB)
        );
        assert_eq!(
            outline_box_for_block(
                Some("minecraft:petrified_oak_slab"),
                &slab_properties("double"),
            ),
            Some(BlockOutlineBox::FULL)
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:oak_slab"), &BTreeMap::new()),
            None
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:oak_slab"), &slab_properties("unexpected"),),
            None
        );
    }

    #[test]
    fn selection_outline_uses_slab_bounds() {
        assert_eq!(
            selection_outline_for_box(
                BlockPos { x: -2, y: 63, z: 4 },
                BlockOutlineBox::BOTTOM_SLAB,
            ),
            SelectionOutline {
                min: [-2.0, 63.0, 4.0],
                max: [-1.0, 63.5, 5.0],
            }
        );
        assert_eq!(
            selection_outline_for_box(BlockPos { x: -2, y: 63, z: 4 }, BlockOutlineBox::TOP_SLAB,),
            SelectionOutline {
                min: [-2.0, 63.5, 4.0],
                max: [-1.0, 64.0, 5.0],
            }
        );
    }

    #[test]
    fn selection_outline_uses_snow_layer_bounds() {
        assert_eq!(
            selection_outline_for_box(
                BlockPos { x: -2, y: 63, z: 4 },
                snow_layer_outline_box(&snow_properties(3)).unwrap(),
            ),
            SelectionOutline {
                min: [-2.0, 63.0, 4.0],
                max: [-1.0, 63.375, 5.0],
            }
        );
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

    fn slab_properties(slab_type: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("type".to_string(), slab_type.to_string())])
    }

    fn snow_properties(layers: u8) -> BTreeMap<String, String> {
        BTreeMap::from([("layers".to_string(), layers.to_string())])
    }
}
