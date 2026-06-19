use super::local_player::LocalPlayerPoseState;
use super::local_player_collision::LocalPlayerBounds;
use bbb_protocol::packets::Vec3d as ProtocolVec3d;

use crate::{
    BlockPos, BlockProbe, TerrainFluidKind, TerrainFluidState, TerrainMaterialClass, WorldStore,
};

const LOCAL_PLAYER_STANDING_EYE_HEIGHT: f64 = 1.62;
const FLUID_INTERACTION_BOX_DEFLATE: f64 = 0.001;
const FLUID_DOWNWARD_FLOW_WEIGHT: f64 = 6.0;
const FLUID_FALLING_HEIGHT_OFFSET: f64 = 8.0 / 9.0;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct LocalPlayerFluidContactState {
    pub(super) water_height: f64,
    pub(super) lava_height: f64,
    pub(super) eye_in_water: bool,
    pub(super) eye_in_lava: bool,
    pub(super) water_current: ProtocolVec3d,
    pub(super) water_current_count: u32,
    pub(super) lava_current: ProtocolVec3d,
    pub(super) lava_current_count: u32,
}

impl LocalPlayerFluidContactState {
    pub(super) fn in_water(self) -> bool {
        self.water_height > 0.0
    }

    pub(super) fn in_lava(self) -> bool {
        self.lava_height > 0.0
    }
}

pub(super) fn local_player_fluid_contact(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
) -> LocalPlayerFluidContactState {
    let bounds = LocalPlayerBounds::at(pose.position).deflated(FLUID_INTERACTION_BOX_DEFLATE);
    let eye_y = pose.position.y + LOCAL_PLAYER_STANDING_EYE_HEIGHT;
    let eye_block_x = block_floor(pose.position.x);
    let eye_block_z = block_floor(pose.position.z);
    local_player_fluid_contact_in_bounds(
        world,
        bounds,
        pose.position.y,
        eye_y,
        eye_block_x,
        eye_block_z,
    )
}

pub(super) fn local_player_eye_in_water(world: &WorldStore) -> bool {
    world
        .local_player()
        .pose
        .is_some_and(|pose| local_player_fluid_contact(world, pose).eye_in_water)
}

pub(super) fn local_player_bounds_contains_any_fluid(
    world: &WorldStore,
    bounds: LocalPlayerBounds,
) -> bool {
    let min_x = block_floor(bounds.min_x());
    let max_x = block_ceil(bounds.max_x());
    let min_y = block_floor(bounds.min_y());
    let max_y = block_ceil(bounds.max_y());
    let min_z = block_floor(bounds.min_z());
    let max_z = block_ceil(bounds.max_z());

    for x in min_x..max_x {
        for y in min_y..max_y {
            for z in min_z..max_z {
                if world
                    .probe_block(BlockPos { x, y, z })
                    .and_then(|block| block.fluid)
                    .is_some()
                {
                    return true;
                }
            }
        }
    }

    false
}

fn local_player_fluid_contact_in_bounds(
    world: &WorldStore,
    bounds: LocalPlayerBounds,
    entity_y: f64,
    eye_y: f64,
    eye_block_x: i32,
    eye_block_z: i32,
) -> LocalPlayerFluidContactState {
    let mut contact = LocalPlayerFluidContactState::default();
    let min_x = block_floor(bounds.min_x());
    let max_x = block_ceil(bounds.max_x()) - 1;
    let min_y = block_floor(bounds.min_y());
    let max_y = block_ceil(bounds.max_y()) - 1;
    let min_z = block_floor(bounds.min_z());
    let max_z = block_ceil(bounds.max_z()) - 1;

    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                let pos = BlockPos { x, y, z };
                let Some(block) = world.probe_block(pos) else {
                    continue;
                };
                let Some(fluid) = block.fluid else {
                    continue;
                };
                let fluid_bottom = f64::from(y);
                let fluid_top = fluid_bottom + probe_fluid_height(world, pos, fluid);
                if fluid_top < bounds.min_y() {
                    continue;
                }

                let height = (fluid_top - entity_y).max(0.0);
                let eyes_inside = x == eye_block_x
                    && z == eye_block_z
                    && eye_y >= fluid_bottom
                    && eye_y <= fluid_top;
                match fluid.kind {
                    TerrainFluidKind::Water => {
                        contact.water_height = contact.water_height.max(height);
                        contact.eye_in_water |= eyes_inside;
                        let flow = local_player_fluid_flow(world, pos, fluid)
                            .scaled(fluid_contact_current_scale(contact.water_height));
                        contact.water_current = contact.water_current.add(flow);
                        contact.water_current_count += 1;
                    }
                    TerrainFluidKind::Lava => {
                        contact.lava_height = contact.lava_height.max(height);
                        contact.eye_in_lava |= eyes_inside;
                        let flow = local_player_fluid_flow(world, pos, fluid)
                            .scaled(fluid_contact_current_scale(contact.lava_height));
                        contact.lava_current = contact.lava_current.add(flow);
                        contact.lava_current_count += 1;
                    }
                }
            }
        }
    }

    contact
}

fn fluid_contact_current_scale(height: f64) -> f64 {
    if height < 0.4 {
        height
    } else {
        1.0
    }
}

fn local_player_fluid_flow(
    world: &WorldStore,
    pos: BlockPos,
    fluid: TerrainFluidState,
) -> ProtocolVec3d {
    let mut flow = ProtocolVec3d::default();
    for direction in HorizontalDirection::ALL {
        let neighbor_pos = direction.offset(pos);
        let neighbor = world.probe_block(neighbor_pos);
        let neighbor_fluid = neighbor.as_ref().and_then(|block| block.fluid);
        if !affects_fluid_flow(fluid.kind, neighbor_fluid) {
            continue;
        }

        let mut neighbor_height = neighbor_fluid
            .filter(|neighbor| neighbor.kind == fluid.kind)
            .map_or(0.0, TerrainFluidState::own_height);
        let mut distance = 0.0;
        if neighbor_height == 0.0 {
            if !block_blocks_fluid_flow(neighbor.as_ref()) {
                let below_neighbor_pos = BlockPos {
                    x: neighbor_pos.x,
                    y: neighbor_pos.y - 1,
                    z: neighbor_pos.z,
                };
                let below_neighbor_fluid = world
                    .probe_block(below_neighbor_pos)
                    .and_then(|block| block.fluid);
                if affects_fluid_flow(fluid.kind, below_neighbor_fluid) {
                    neighbor_height = below_neighbor_fluid
                        .filter(|neighbor| neighbor.kind == fluid.kind)
                        .map_or(0.0, TerrainFluidState::own_height);
                    if neighbor_height > 0.0 {
                        distance =
                            fluid.own_height() - (neighbor_height - FLUID_FALLING_HEIGHT_OFFSET);
                    }
                }
            }
        } else {
            distance = fluid.own_height() - neighbor_height;
        }

        if distance != 0.0 {
            flow.x += f64::from(direction.step_x) * distance;
            flow.z += f64::from(direction.step_z) * distance;
        }
    }

    flow = flow.normalized();
    if fluid.falling
        && HorizontalDirection::ALL.iter().any(|direction| {
            let neighbor_pos = direction.offset(pos);
            block_has_solid_flow_face(world.probe_block(neighbor_pos).as_ref())
                || block_has_solid_flow_face(
                    world
                        .probe_block(BlockPos {
                            x: neighbor_pos.x,
                            y: neighbor_pos.y + 1,
                            z: neighbor_pos.z,
                        })
                        .as_ref(),
                )
        })
    {
        flow.y -= FLUID_DOWNWARD_FLOW_WEIGHT;
        flow = flow.normalized();
    }

    flow
}

fn affects_fluid_flow(kind: TerrainFluidKind, fluid: Option<TerrainFluidState>) -> bool {
    fluid.map_or(true, |fluid| fluid.kind == kind)
}

fn block_blocks_fluid_flow(block: Option<&BlockProbe>) -> bool {
    block.is_some_and(|block| {
        matches!(
            block.material,
            TerrainMaterialClass::Opaque | TerrainMaterialClass::Translucent
        ) || matches!(block.block_name.as_deref(), Some("minecraft:barrier"))
    })
}

fn block_has_solid_flow_face(block: Option<&BlockProbe>) -> bool {
    block_blocks_fluid_flow(block)
}

fn probe_fluid_height(world: &WorldStore, pos: BlockPos, fluid: TerrainFluidState) -> f64 {
    let same_fluid_above = pos.y.checked_add(1).is_some_and(|above_y| {
        world
            .probe_block(BlockPos {
                x: pos.x,
                y: above_y,
                z: pos.z,
            })
            .and_then(|block| block.fluid)
            .is_some_and(|above| above.kind == fluid.kind)
    });
    if same_fluid_above {
        1.0
    } else {
        fluid.own_height()
    }
}

#[derive(Debug, Clone, Copy)]
struct HorizontalDirection {
    step_x: i32,
    step_z: i32,
}

impl HorizontalDirection {
    const ALL: [Self; 4] = [
        Self {
            step_x: 0,
            step_z: -1,
        },
        Self {
            step_x: 1,
            step_z: 0,
        },
        Self {
            step_x: 0,
            step_z: 1,
        },
        Self {
            step_x: -1,
            step_z: 0,
        },
    ];

    fn offset(self, pos: BlockPos) -> BlockPos {
        BlockPos {
            x: pos.x + self.step_x,
            y: pos.y,
            z: pos.z + self.step_z,
        }
    }
}

trait ProtocolVec3dExt {
    fn add(self, other: ProtocolVec3d) -> ProtocolVec3d;
    fn scaled(self, scale: f64) -> ProtocolVec3d;
    fn normalized(self) -> ProtocolVec3d;
}

impl ProtocolVec3dExt for ProtocolVec3d {
    fn add(self, other: ProtocolVec3d) -> ProtocolVec3d {
        ProtocolVec3d {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    fn scaled(self, scale: f64) -> ProtocolVec3d {
        ProtocolVec3d {
            x: self.x * scale,
            y: self.y * scale,
            z: self.z * scale,
        }
    }

    fn normalized(self) -> ProtocolVec3d {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if length <= f64::EPSILON {
            ProtocolVec3d::default()
        } else {
            self.scaled(1.0 / length)
        }
    }
}

fn block_floor(value: f64) -> i32 {
    value.floor() as i32
}

fn block_ceil(value: f64) -> i32 {
    value.ceil() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        ChunkColumn, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };

    const AIR_BLOCK_STATE_ID: i32 = 0;
    const STONE_BLOCK_STATE_ID: i32 = 1;
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
    const FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID: i32 = 89;
    const FLOWING_WATER_LEVEL_7_BLOCK_STATE_ID: i32 = 93;
    const FALLING_WATER_LEVEL_8_BLOCK_STATE_ID: i32 = 94;

    #[test]
    fn probe_fluid_height_uses_own_height_without_same_fluid_above() {
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID,
        );
        let fluid = world
            .probe_block(BlockPos { x: 0, y: 0, z: 0 })
            .and_then(|block| block.fluid)
            .unwrap();

        assert_eq!(
            probe_fluid_height(&world, BlockPos { x: 0, y: 0, z: 0 }, fluid),
            5.0 / 9.0
        );
    }

    #[test]
    fn probe_fluid_height_is_full_when_same_fluid_is_above() {
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID,
        );
        set_block(
            &mut world,
            BlockPos { x: 0, y: 1, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        let fluid = world
            .probe_block(BlockPos { x: 0, y: 0, z: 0 })
            .and_then(|block| block.fluid)
            .unwrap();

        assert_eq!(
            probe_fluid_height(&world, BlockPos { x: 0, y: 0, z: 0 }, fluid),
            1.0
        );
    }

    #[test]
    fn local_player_fluid_contact_reports_water_height_and_eye_state() {
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        set_block(
            &mut world,
            BlockPos { x: 0, y: 1, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );

        let contact = local_player_fluid_contact(
            &world,
            LocalPlayerPoseState {
                position: vec3(0.5, 0.0, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(contact.water_height, 17.0 / 9.0);
        assert!(contact.eye_in_water);
        assert_eq!(contact.lava_height, 0.0);
        assert!(!contact.eye_in_lava);
    }

    #[test]
    fn local_player_fluid_contact_ignores_low_fluid_below_interaction_box() {
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID,
        );

        let contact = local_player_fluid_contact(
            &world,
            LocalPlayerPoseState {
                position: vec3(0.5, 0.7, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(contact.water_height, 0.0);
        assert!(!contact.eye_in_water);
    }

    #[test]
    fn local_player_fluid_contact_accumulates_vanilla_shaped_flow_current() {
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 1 },
            FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID,
        );

        let contact = local_player_fluid_contact(
            &world,
            LocalPlayerPoseState {
                position: vec3(0.5, 0.0, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(contact.water_current_count, 1);
        assert_f64_near(contact.water_current.x, 0.0, 0.000001);
        assert_f64_near(contact.water_current.y, 0.0, 0.000001);
        assert_f64_near(contact.water_current.z, 1.0, 0.000001);
    }

    #[test]
    fn local_player_fluid_contact_scales_current_when_fluid_height_is_low() {
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 1, z: 0 },
            FLOWING_WATER_LEVEL_7_BLOCK_STATE_ID,
        );
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 1 },
            SOURCE_WATER_BLOCK_STATE_ID,
        );

        let contact = local_player_fluid_contact(
            &world,
            LocalPlayerPoseState {
                position: vec3(0.5, 1.0, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(contact.water_current_count, 1);
        assert_f64_near(contact.water_height, 1.0 / 9.0, 0.000001);
        assert_f64_near(contact.water_current.y, 0.0, 0.000001);
        assert_f64_near(
            (contact.water_current.x * contact.water_current.x
                + contact.water_current.z * contact.water_current.z)
                .sqrt(),
            1.0 / 9.0,
            0.000001,
        );
    }

    #[test]
    fn falling_fluid_current_points_down_when_against_solid_face() {
        let mut world = empty_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            FALLING_WATER_LEVEL_8_BLOCK_STATE_ID,
        );
        set_block(
            &mut world,
            BlockPos { x: 1, y: 0, z: 0 },
            STONE_BLOCK_STATE_ID,
        );

        let contact = local_player_fluid_contact(
            &world,
            LocalPlayerPoseState {
                position: vec3(0.5, 0.0, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(contact.water_current_count, 1);
        assert!(contact.water_current.y < -0.98);
    }

    fn empty_world() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: crate::ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    AIR_BLOCK_STATE_ID,
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_state_id,
            })
        );
    }

    fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
        ProtocolVec3d { x, y, z }
    }

    fn assert_f64_near(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}"
        );
    }
}
