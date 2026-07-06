mod mounts;
mod runtime;
mod selection;
mod transforms;

pub(in crate::entity_models) use mounts::{
    emit_donkey_model, emit_horse_model, emit_undead_horse_model,
};
pub(in crate::entity_models) use selection::{
    camel_model_color, hoglin_model_color, llama_model_color, piglin_model_color,
};

#[cfg(test)]
pub(super) use runtime::entity_model_mesh;
pub(super) use runtime::{
    entity_model_colored_runtime_mesh, zombie_variant_color, zombie_variant_root_transform,
};
#[cfg(test)]
pub(super) use runtime::{
    humanoid_arm_swing_parts, humanoid_limb_swing_parts, quadruped_leg_x_rotations,
    quadruped_limb_swing_parts, HUMANOID_ARM_PART_INDICES, HUMANOID_LEG_PART_INDICES,
};
pub(crate) use transforms::sign_base_transformation;
pub(super) use transforms::{
    arrow_model_root_transform, bed_model_root_transform, bell_model_root_transform,
    boat_model_root_transform, cave_spider_model_root_transform, chest_model_root_transform,
    cod_model_root_transform, creeper_model_root_transform, drowned_model_root_transform,
    end_crystal_model_root_transform, ender_dragon_model_root_transform,
    entity_model_root_transform, evoker_fangs_model_root_transform, feline_model_root_transform,
    fox_model_root_transform, ghast_model_root_transform, happy_ghast_model_root_transform,
    illusioner_model_root_transform, iron_golem_model_root_transform,
    leash_knot_model_root_transform, llama_spit_model_root_transform,
    magma_cube_model_root_transform, mesh_transformer_scale_transform,
    mesh_transformer_scaled_model_root_transform, minecart_display_block_content_transform,
    minecart_model_root_transform, panda_model_root_transform, phantom_model_root_transform,
    player_model_root_transform, polar_bear_model_root_transform, pufferfish_model_root_transform,
    salmon_model_root_transform, shulker_bullet_model_root_transform, shulker_model_root_transform,
    sign_model_root_transform, slime_model_root_transform, squid_model_root_transform,
    trident_model_root_transform, tropical_fish_model_root_transform,
    villager_adult_model_root_transform, wind_charge_model_root_transform,
    wither_model_root_transform, wither_skeleton_model_root_transform,
    wither_skull_model_root_transform, GIANT_SCALE, HORSE_SCALE, HUSK_SCALE,
};
#[cfg(test)]
pub(super) use transforms::{
    boat_bubble_transform, boat_damage_roll_degrees, death_fall_factor,
    enderman_creepy_render_offset, entity_flip_degrees, minecart_damage_roll_degrees,
    minecart_render_jitter_offset,
};
