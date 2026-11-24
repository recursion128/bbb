use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
    vanilla_block_state_with_property,
};
use bbb_protocol::block_types::{vanilla_block_type, vanilla_block_type_for_resource_id};
use bbb_protocol::packets::BlockPos;
use bbb_server_worldgen::WorldGenLevel;
const UPDATE_CLIENTS: i32 = 2;
const UPDATE_NONE: i32 = 260;
const FEATURE_SET_BLOCK: i32 = 3;
fn catalog_default_state(name: &str) -> i32 {
    vanilla_block_type_for_resource_id(name)
        .and_then(|block| vanilla_block_default_state_id(block.id))
        .unwrap()
}
fn block_set_property(state: i32, property: &str, value: &str) -> i32 {
    vanilla_block_state_with_property(state, property, value).unwrap_or(state)
}
macro_rules! host_shape_hooks {
    (edge) => {
        fn update_shape_at_edge(
            &mut self,
            update_mode: i32,
            shape: &bbb_server_worldgen::tree::voxel_shape::BitSetDiscreteVoxelShape,
            start_x: i32,
            start_y: i32,
            start_z: i32,
        ) {
            bbb_server_core::worldgen_shape::update_shape_at_edge(
                self,
                update_mode,
                shape,
                start_x,
                start_y,
                start_z,
            )
        }
    };
    (all) => {
        host_shape_hooks!(edge);
        fn update_template_shape_at_edge(
            &mut self,
            update_mode: i32,
            shape: &bbb_server_worldgen::tree::voxel_shape::BitSetDiscreteVoxelShape,
            start_x: i32,
            start_y: i32,
            start_z: i32,
        ) {
            bbb_server_core::worldgen_shape::update_template_shape_at_edge(
                self,
                update_mode,
                shape,
                start_x,
                start_y,
                start_z,
            )
        }
        fn update_from_neighbour_shapes(
            &mut self,
            state: i32,
            pos: bbb_protocol::packets::BlockPos,
        ) -> i32 {
            bbb_server_core::worldgen_shape::update_from_neighbour_shapes(self, state, pos)
        }
        fn can_survive_at(&self, state: i32, pos: bbb_protocol::packets::BlockPos) -> bool {
            bbb_server_core::worldgen_shape::can_survive_at(self, state, pos)
        }
        fn collision_shape_full_block_at(
            &self,
            state: i32,
            pos: bbb_protocol::packets::BlockPos,
        ) -> bool {
            bbb_server_core::block_behavior::collision_shape_full_block_at(state, pos)
        }
        fn fire_state_for_placement(&self, pos: bbb_protocol::packets::BlockPos) -> Option<i32> {
            bbb_server_core::block_behavior::fire::state_for_placement(
                pos,
                bbb_server_core::registry_tags::RegistryTagSnapshot::builtin(),
                &mut |probe| self.get_block(probe),
            )
        }
    };
}
#[path = "worldgen_host_features/block_predicates.rs"]
mod block_predicates;
#[path = "worldgen_host_features/end_spike.rs"]
mod end_spike;
#[path = "worldgen_host_features/fossil.rs"]
mod fossil;
#[path = "worldgen_host_features/random_selectors.rs"]
mod random_selectors;
#[path = "worldgen_host_features/root_system.rs"]
mod root_system;
#[path = "worldgen_host_features/seam.rs"]
mod seam;
#[path = "worldgen_host_features/simple_block.rs"]
mod simple_block;
#[path = "worldgen_host_features/support.rs"]
mod support;
#[path = "worldgen_host_features/tree.rs"]
mod tree;

fn block_is(state: i32, name: &str) -> bool {
    vanilla_block_type_for_resource_id(name)
        .is_some_and(|block| vanilla_block_state_block_type_id(state) == Some(block.id))
}
#[path = "worldgen_host_features/placed_feature.rs"]
mod placed_feature;

#[path = "worldgen_host_features/sculk_patch.rs"]
mod sculk_patch;
fn block_set_bool(state: i32, property: &str, value: bool) -> i32 {
    block_set_property(state, property, if value { "true" } else { "false" })
}
fn offset_block_pos(pos: BlockPos, direction: bbb_protocol::packets::Direction) -> BlockPos {
    bbb_server_worldgen::tree::relative(pos, direction, 1)
}

#[path = "worldgen_host_features/vegetation_patch.rs"]
mod vegetation_patch;

#[path = "worldgen_host_features/structure_processor.rs"]
mod structure_processor;
