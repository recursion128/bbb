use super::entity_data::{entity_data_byte, entity_data_int};
use super::schema::{
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type,
};
use super::*;

pub(super) fn entity_model_source_full_bright_light() -> TerrainLight {
    ENTITY_LIGHT_PROBE_FULL_BRIGHT
}

/// Vanilla `ExperienceOrb.getIcon()`: the 4×4 sheet cell picked from the synced orb value.
pub fn experience_orb_icon(value: i32) -> i32 {
    if value >= 2477 {
        10
    } else if value >= 1237 {
        9
    } else if value >= 617 {
        8
    } else if value >= 307 {
        7
    } else if value >= 149 {
        6
    } else if value >= 73 {
        5
    } else if value >= 37 {
        4
    } else if value >= 17 {
        3
    } else if value >= 7 {
        2
    } else if value >= 3 {
        1
    } else {
        0
    }
}

/// Vanilla per-renderer `getBlockLightLevel` additive overrides applied to a sampled light
/// probe: `ExperienceOrbRenderer` returns `clamp(super + 7, 0, 15)`. Shared by the entity
/// model-source projection and the pickup-particle light capture.
pub(super) fn vanilla_entity_renderer_block_light(
    entity_type_id: i32,
    light: TerrainLight,
) -> TerrainLight {
    if entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
        TerrainLight {
            block: light.block.saturating_add(7).min(15),
            sky: light.sky,
        }
    } else {
        light
    }
}

/// Final model light from a sampled probe and source-derived renderer/metadata overrides.
pub(super) fn entity_light_coords(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    light: TerrainLight,
) -> u32 {
    let light = vanilla_entity_renderer_block_light(entity_type_id, light);
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    let on_fire = (entity_data_byte(
        data_values,
        accessors.shared_flags_data_id,
        accessors.shared_flags_default,
    ) & accessors.on_fire_mask)
        != 0;
    let mut block = if on_fire { 15 } else { light.block };
    block = block.max(vanilla_entity_renderer_block_light_floor(entity_type_id));
    // Vanilla `GlowSquidRenderer.getBlockLightLevel`: a bioluminescent boost
    // `max(blockLight, (int)clampedLerp(1 − darkTicks/10, 0, 15))`. Undamaged (`darkTicks == 0`) it is fully
    // bright (`15`); a hurt drops `darkTicks` to `100` (dark), and it ramps back to full over the final 10
    // ticks. The vanilla `glow == 15 ? 15 : max(glow, super)` ternary is just `max(super, glow)` (super ≤ 15).
    if let Some((data_id, default)) =
        vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type(entity_type_id)
    {
        let dark_ticks = entity_data_int(data_values, data_id, default);
        let glow = ((1.0 - dark_ticks as f32 / 10.0).clamp(0.0, 1.0) * 15.0) as u8;
        block = block.max(glow);
    }
    bbb_render_types::VanillaLightCoords::from_stored(block, light.sky, 0, false).packed()
}
