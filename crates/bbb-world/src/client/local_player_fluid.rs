use super::local_player::LocalPlayerPoseState;
use super::local_player_collision::LocalPlayerBounds;
use crate::{BlockPos, TerrainFluidKind, TerrainFluidState, WorldStore};

const LOCAL_PLAYER_STANDING_EYE_HEIGHT: f64 = 1.62;
const FLUID_INTERACTION_BOX_DEFLATE: f64 = 0.001;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct LocalPlayerFluidContactState {
    pub(super) water_height: f64,
    pub(super) lava_height: f64,
    pub(super) eye_in_water: bool,
    pub(super) eye_in_lava: bool,
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
                    }
                    TerrainFluidKind::Lava => {
                        contact.lava_height = contact.lava_height.max(height);
                        contact.eye_in_lava |= eyes_inside;
                    }
                }
            }
        }
    }

    contact
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

fn block_floor(value: f64) -> i32 {
    value.floor() as i32
}

fn block_ceil(value: f64) -> i32 {
    value.ceil() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::Vec3d as ProtocolVec3d;

    use crate::{
        ChunkColumn, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };

    const AIR_BLOCK_STATE_ID: i32 = 0;
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
    const FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID: i32 = 89;

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
}
