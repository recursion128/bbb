use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
    VANILLA_ENTITY_TYPE_SLIME_ID, VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TURTLE_ID,
    VANILLA_ENTITY_TYPE_VILLAGER_ID,
};

use super::content_facts::{entity_is_mob, entity_metadata_is_living, mod_entity_spec};
use super::renderer_catalog::vanilla_entity_renderer_base_shadow;
use super::schema::{
    vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type,
    vanilla_entity_metadata_slime_size_accessor_for_entity_type,
};
use super::{EntityRenderCandidateState, EntityRenderSource, EntityVec3};
use crate::client::block_collision_shape_is_full_block;
use crate::terrain::block_name_has_invisible_render_shape;
use crate::{BlockPos, WorldStore};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityShadowPieceState {
    pub relative_position: [f32; 3],
    pub shape_min: [f32; 3],
    pub shape_max: [f32; 3],
    pub alpha: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityShadowRenderSourceState {
    pub entity_id: i32,
    /// Raw interpolated entity origin. Dispatcher render offsets are applied later by native.
    pub origin: EntityVec3,
    pub radius: f32,
    pub pieces: Vec<EntityShadowPieceState>,
}

pub(super) fn fill_entity_shadow_render_sources(
    world: &WorldStore,
    survivors: &[EntityRenderCandidateState],
    model_sources: &[EntityRenderSource],
    partial_ticks: f32,
    camera_position: EntityVec3,
    sky_darken: u8,
    sources: &mut Vec<EntityShadowRenderSourceState>,
) {
    sources.clear();
    sources.reserve(model_sources.len());
    let ambient_light = world
        .level_info()
        .map_or(0.0, |level| level.ambient_light());
    // Model sources are an ordered subsequence of survivors.
    let mut survivors = survivors.iter();
    for source in model_sources {
        let Some(candidate) = survivors.find(|candidate| candidate.entity_id == source.entity_id)
        else {
            continue;
        };
        if candidate.is_invisible {
            continue;
        }
        let (radius, strength) = entity_shadow_radius_and_strength(world, source, partial_ticks);
        let Some(radius) = vanilla_shadow_radius(radius) else {
            continue;
        };
        let dx = candidate.current_position.x - camera_position.x;
        let dy = candidate.current_position.y - camera_position.y;
        let dz = candidate.current_position.z - camera_position.z;
        let distance_squared = dx * dx + dy * dy + dz * dz;
        let power = ((1.0 - distance_squared / 256.0) * f64::from(strength)) as f32;
        if power <= 0.0 || power.is_nan() {
            continue;
        }

        let origin = source.distance_position;
        if !origin.x.is_finite() || !origin.y.is_finite() || !origin.z.is_finite() {
            continue;
        }
        let radius = radius.clamp(0.0, 16.0);
        let x0 = (origin.x - f64::from(radius)).floor() as i32;
        let x1 = (origin.x + f64::from(radius)).floor() as i32;
        let z0 = (origin.z - f64::from(radius)).floor() as i32;
        let z1 = (origin.z + f64::from(radius)).floor() as i32;
        let depth = (power / 0.5 - 1.0).min(radius);
        let dim = world.dimension();
        let dim_min_y = dim.min_y;
        let dim_max_y = dim.min_y.saturating_add(dim.height).saturating_sub(1);
        let y0 = ((origin.y - f64::from(depth)).floor() as i32).max(dim_min_y);
        let y1 = (origin.y.floor() as i32).min(dim_max_y);
        let mut pieces = Vec::new();
        for z in z0..=z1 {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    let pos = BlockPos { x, y, z };
                    let below_pos = BlockPos {
                        x,
                        y: y.saturating_sub(1),
                        z,
                    };
                    let Some(below) = world.block_view(below_pos) else {
                        continue;
                    };
                    if below
                        .block_name
                        .is_some_and(block_name_has_invisible_render_shape)
                        || !block_collision_shape_is_full_block(&below, below_pos)
                    {
                        continue;
                    }
                    let Some(light) = world.sample_block_light(pos) else {
                        continue;
                    };
                    let brightness = light.block.max(light.sky.saturating_sub(sky_darken));
                    if brightness <= 3 {
                        continue;
                    }
                    let at_depth = power - (origin.y - f64::from(y)) as f32 * 0.5;
                    let alpha =
                        (at_depth * 0.5 * vanilla_lightmap_brightness(ambient_light, brightness))
                            .clamp(0.0, 1.0);
                    pieces.push(EntityShadowPieceState {
                        relative_position: [
                            (f64::from(x) - origin.x) as f32,
                            (f64::from(y) - origin.y) as f32,
                            (f64::from(z) - origin.z) as f32,
                        ],
                        // `isCollisionShapeFullBlock` guarantees the shape consumed by the
                        // shadow plane spans the full block cell in bbb's collision catalog.
                        shape_min: [0.0; 3],
                        shape_max: [1.0; 3],
                        alpha,
                    });
                }
            }
        }
        if !pieces.is_empty() {
            sources.push(EntityShadowRenderSourceState {
                entity_id: source.entity_id,
                origin,
                radius,
                pieces,
            });
        }
    }
}

fn entity_shadow_radius_and_strength(
    world: &WorldStore,
    source: &EntityRenderSource,
    partial_ticks: f32,
) -> (f32, f32) {
    if let Some(display) = world
        .entities
        .display_render_source_at_partial_tick(source.entity_id, partial_ticks)
    {
        return (display.shadow_radius, display.shadow_strength);
    }
    let (base_radius, strength) = mod_entity_spec(source.entity_type_id, world.content_registry())
        .map(|spec| (spec.width * 0.5, 1.0))
        .unwrap_or_else(|| vanilla_entity_renderer_base_shadow(source.entity_type_id));
    let mut radius = if source.entity_type_id == VANILLA_ENTITY_TYPE_SLIME_ID
        || source.entity_type_id == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
    {
        vanilla_entity_metadata_slime_size_accessor_for_entity_type(source.entity_type_id)
            .and_then(|(data_id, default)| {
                world
                    .entities
                    .metadata_int_for_entity(source.entity_id, data_id, default)
            })
            .map_or(base_radius, |size| size as f32 * 0.25)
    } else if source.entity_type_id == VANILLA_ENTITY_TYPE_PUFFERFISH_ID {
        vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(
            source.entity_type_id,
        )
        .and_then(|(data_id, default)| {
            world
                .entities
                .metadata_int_for_entity(source.entity_id, data_id, default)
        })
        .map_or(base_radius, |puff_state| 0.1 + 0.1 * puff_state as f32)
    } else {
        let living_scale =
            if entity_metadata_is_living(source.entity_type_id, world.content_registry()) {
                source.render.scale
            } else {
                1.0
            };
        let age_scale = if entity_is_mob(source.entity_type_id, world.content_registry()) {
            source.render.age_scale
        } else {
            1.0
        };
        base_radius * living_scale * age_scale
    };
    if world.entities.is_baby(source.entity_id).unwrap_or(false) {
        radius *= match source.entity_type_id {
            VANILLA_ENTITY_TYPE_TURTLE_ID => 0.83,
            VANILLA_ENTITY_TYPE_VILLAGER_ID | VANILLA_ENTITY_TYPE_STRIDER_ID => 0.5,
            _ => 1.0,
        };
    }
    (radius, strength)
}

pub(crate) fn vanilla_lightmap_brightness(ambient_light: f32, brightness: u8) -> f32 {
    let linear = f32::from(brightness.min(15)) / 15.0;
    let curved = linear / (4.0 - 3.0 * linear);
    ambient_light + (1.0 - ambient_light) * curved
}

fn vanilla_shadow_radius(radius: f32) -> Option<f32> {
    if radius.is_nan() {
        return None;
    }
    let radius = radius.min(32.0);
    (radius > 0.0).then_some(radius)
}

#[cfg(test)]
mod tests {
    use super::{vanilla_lightmap_brightness, vanilla_shadow_radius};

    #[test]
    fn dimension_brightness_curve_matches_vanilla_lightmap() {
        assert_eq!(vanilla_lightmap_brightness(0.0, 0), 0.0);
        assert_eq!(vanilla_lightmap_brightness(0.0, 15), 1.0);
        assert!((vanilla_lightmap_brightness(0.2, 3) - 0.247_058_84).abs() < 1.0e-6);
    }

    #[test]
    fn shadow_radius_matches_java_min_and_positive_gate() {
        assert_eq!(vanilla_shadow_radius(f32::INFINITY), Some(32.0));
        assert_eq!(vanilla_shadow_radius(0.5), Some(0.5));
        assert_eq!(vanilla_shadow_radius(0.0), None);
        assert_eq!(vanilla_shadow_radius(f32::NAN), None);
    }
}
