use super::*;

pub(super) fn terrain_particle_provider_accepts_block_state(block_state_id: i32) -> bool {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let Some(block_state) = block_states.by_id(block_state_id) else {
        return true;
    };
    !block_name_is_air(&block_state.name)
        && block_state.name != "minecraft:moving_piston"
        && block_name_should_spawn_terrain_particles(&block_state.name)
}

pub(super) fn destroy_block_effect_accepts_block_state(block_state_id: i32) -> bool {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let Some(block_state) = block_states.by_id(block_state_id) else {
        return true;
    };
    !block_name_is_air(&block_state.name)
        && block_name_should_spawn_terrain_particles(&block_state.name)
}
