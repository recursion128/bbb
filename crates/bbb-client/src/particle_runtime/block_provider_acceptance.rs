use super::*;

pub(super) fn terrain_particle_provider_accepts_block_state(block_state_id: i32) -> bool {
    let Some(block_name) =
        bbb_protocol::block_state_types::vanilla_block_state_name(block_state_id)
    else {
        return true;
    };
    !block_name_is_air(block_name)
        && block_name != "minecraft:moving_piston"
        && block_name_should_spawn_terrain_particles(block_name)
}

pub(super) fn destroy_block_effect_accepts_block_state(block_state_id: i32) -> bool {
    let Some(block_name) =
        bbb_protocol::block_state_types::vanilla_block_state_name(block_state_id)
    else {
        return true;
    };
    !block_name_is_air(block_name) && block_name_should_spawn_terrain_particles(block_name)
}
