use crate::{BlockPos, ClientEnvironmentAttributes, WorldStore};
use bbb_protocol::environment_particles::AmbientParticle;

use super::environment_attributes::{
    apply_selected_spatial_environment_attribute_maps, apply_spatial_environment_attribute_maps,
    sanitize_environment_attributes, ClientEnvironmentAttributeMap,
    EnvironmentAttributeProjectionError,
};
use super::timelines::apply_environment_track;
use super::RegistryProjectionError;

const GAUSSIAN_SAMPLE_KERNEL: [f64; 7] = [0.0, 1.0, 4.0, 6.0, 4.0, 1.0, 0.0];
const AMBIENT_PARTICLES_ATTRIBUTE: &str = "minecraft:visual/ambient_particles";
const CAMERA_ENVIRONMENT_ATTRIBUTES: [&str; 15] = [
    "minecraft:visual/fog_color",
    "minecraft:visual/fog_start_distance",
    "minecraft:visual/fog_end_distance",
    "minecraft:visual/sky_fog_end_distance",
    "minecraft:visual/cloud_fog_end_distance",
    "minecraft:visual/water_fog_color",
    "minecraft:visual/water_fog_start_distance",
    "minecraft:visual/water_fog_end_distance",
    "minecraft:visual/sky_color",
    "minecraft:visual/sunrise_sunset_color",
    "minecraft:visual/sun_angle",
    "minecraft:visual/moon_angle",
    "minecraft:visual/star_angle",
    "minecraft:visual/moon_phase",
    "minecraft:visual/star_brightness",
];
const CAMERA_NON_SPATIAL_ENVIRONMENT_ATTRIBUTES: [&str; 1] = ["minecraft:visual/moon_phase"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentBiomeSampling {
    Direct,
    Gaussian,
}

impl WorldStore {
    /// Samples only the attributes consumed by the rendered camera's sky and
    /// fog projection. Vanilla owns samplers per environment attribute; avoid
    /// cloning and applying unrelated particle, audio and gameplay values on
    /// every presented frame.
    pub fn sampled_client_camera_environment_attributes_at(
        &self,
        position: [f64; 3],
    ) -> Result<ClientEnvironmentAttributes, RegistryProjectionError> {
        let Some(mut attributes) = self.camera_dimension_environment_attributes() else {
            return Ok(ClientEnvironmentAttributes::default());
        };
        if let Some(center) = self.environment_attribute_map_at_position(position) {
            for attribute in CAMERA_NON_SPATIAL_ENVIRONMENT_ATTRIBUTES {
                center
                    .apply_attribute_to(attribute, &mut attributes)
                    .map_err(environment_sampling_error)?;
            }
        }
        let weighted_sources = self.gaussian_environment_attribute_maps(position);
        apply_selected_spatial_environment_attribute_maps(
            &mut attributes,
            &weighted_sources,
            &CAMERA_ENVIRONMENT_ATTRIBUTES,
        )
        .map_err(environment_sampling_error)?;
        self.apply_selected_environment_timelines(&mut attributes, &CAMERA_ENVIRONMENT_ATTRIBUTES)?;
        sanitize_environment_attributes(&mut attributes);
        Ok(attributes)
    }

    /// Samples only Vanilla's `AMBIENT_PARTICLES` value for `ClientLevel.doAnimateTick`.
    ///
    /// Vanilla bakes one sampler per environment attribute and does not project every unrelated
    /// visual/audio/gameplay value for each of the 1,334 random animate-tick positions. Preserve
    /// the same dimension -> biome -> timeline order while keeping this hot query field-local.
    pub fn sampled_client_ambient_particles_at(
        &self,
        position: [f64; 3],
    ) -> Result<Vec<AmbientParticle>, RegistryProjectionError> {
        let Some(level) = self.level_info() else {
            return Ok(Vec::new());
        };
        let dimension = self
            .registries
            .dimension_type_client_projection(level.dimension_type_id);
        let ambient_particles = match dimension.as_ref() {
            Some(projection) => projection.environment_attributes.ambient_particles.clone(),
            None => level.environment_attributes.ambient_particles.clone(),
        };
        let mut attributes = ClientEnvironmentAttributes {
            ambient_particles,
            ..ClientEnvironmentAttributes::default()
        };
        if let Some(biome) = self.environment_attribute_map_at_position(position) {
            biome
                .apply_attribute_to(AMBIENT_PARTICLES_ATTRIBUTE, &mut attributes)
                .map_err(environment_sampling_error)?;
        }
        self.visit_resolved_dimension_timelines(|timeline, total_ticks| {
            for track in timeline
                .tracks
                .iter()
                .filter(|track| track.attribute == AMBIENT_PARTICLES_ATTRIBUTE)
            {
                apply_environment_track(&mut attributes, timeline, track, total_ticks)?;
            }
            Ok(())
        })?;
        Ok(attributes.ambient_particles.unwrap_or_default())
    }

    /// Samples Vanilla's positional `WATER_EVAPORATES` environment attribute.
    ///
    /// Legacy sessions without synchronized dimension-type contents retain the
    /// pre-26.1 Nether identity fallback used by the native event dispatcher.
    pub fn water_evaporates_at(&self, pos: BlockPos) -> bool {
        if let Some(value) = self
            .sampled_client_environment_attributes_at(
                [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)],
                EnvironmentBiomeSampling::Direct,
            )
            .ok()
            .and_then(|attributes| attributes.water_evaporates)
        {
            return value;
        }
        if self
            .registry_content(crate::DIMENSION_TYPE_REGISTRY)
            .is_some()
        {
            return false;
        }
        self.level_info().is_some_and(|level| {
            level.dimension_type_id == 1
                || level.dimension == "minecraft:the_nether"
                || level.dimension_type_name.as_deref() == Some("minecraft:the_nether")
        })
    }

    pub fn sampled_client_environment_attributes(
        &self,
    ) -> Result<ClientEnvironmentAttributes, RegistryProjectionError> {
        let Some(mut attributes) = self.dimension_environment_attributes() else {
            return Ok(ClientEnvironmentAttributes::default());
        };
        self.apply_environment_timelines(&mut attributes)?;
        sanitize_environment_attributes(&mut attributes);
        Ok(attributes)
    }

    pub fn sampled_client_environment_attributes_at(
        &self,
        position: [f64; 3],
        sampling: EnvironmentBiomeSampling,
    ) -> Result<ClientEnvironmentAttributes, RegistryProjectionError> {
        let Some(mut attributes) = self.dimension_environment_attributes() else {
            return Ok(ClientEnvironmentAttributes::default());
        };
        match sampling {
            EnvironmentBiomeSampling::Direct => {
                if let Some(biome) = self.environment_attribute_map_at_position(position) {
                    biome
                        .apply_to(&mut attributes)
                        .map_err(environment_sampling_error)?;
                }
            }
            EnvironmentBiomeSampling::Gaussian => {
                if let Some(biome) = self.environment_attribute_map_at_position(position) {
                    biome
                        .apply_non_spatial_to(&mut attributes)
                        .map_err(environment_sampling_error)?;
                }
                let weighted_sources = self.gaussian_environment_attribute_maps(position);
                apply_spatial_environment_attribute_maps(&mut attributes, &weighted_sources)
                    .map_err(environment_sampling_error)?;
            }
        }
        self.apply_environment_timelines(&mut attributes)?;
        sanitize_environment_attributes(&mut attributes);
        Ok(attributes)
    }

    fn dimension_environment_attributes(&self) -> Option<ClientEnvironmentAttributes> {
        let level = self.level_info()?;
        Some(
            self.registries
                .dimension_type_client_projection(level.dimension_type_id)
                .map(|projection| projection.environment_attributes.clone())
                .unwrap_or_else(|| level.environment_attributes()),
        )
    }

    fn camera_dimension_environment_attributes(&self) -> Option<ClientEnvironmentAttributes> {
        let level = self.level_info()?;
        let dimension = self
            .registries
            .dimension_type_client_projection(level.dimension_type_id);
        let source = dimension
            .as_ref()
            .map(|projection| &projection.environment_attributes)
            .unwrap_or(&level.environment_attributes);
        Some(camera_environment_attributes(source))
    }

    fn apply_environment_timelines(
        &self,
        attributes: &mut ClientEnvironmentAttributes,
    ) -> Result<(), RegistryProjectionError> {
        self.visit_resolved_dimension_timelines(|timeline, total_ticks| {
            for track in &timeline.tracks {
                apply_environment_track(attributes, timeline, track, total_ticks)?;
            }
            Ok(())
        })
    }

    fn apply_selected_environment_timelines(
        &self,
        attributes: &mut ClientEnvironmentAttributes,
        selected_attributes: &[&str],
    ) -> Result<(), RegistryProjectionError> {
        self.visit_resolved_dimension_timelines(|timeline, total_ticks| {
            for track in timeline
                .tracks
                .iter()
                .filter(|track| selected_attributes.contains(&track.attribute.as_str()))
            {
                apply_environment_track(attributes, timeline, track, total_ticks)?;
            }
            Ok(())
        })
    }

    fn environment_attribute_map_at_position(
        &self,
        position: [f64; 3],
    ) -> Option<&ClientEnvironmentAttributeMap> {
        let [Some(quart_x), Some(quart_y), Some(quart_z)] = position.map(quart_coordinate) else {
            return None;
        };
        self.noise_biome_holder_id_at_quart(quart_x, quart_y, quart_z)
            .and_then(|holder_id| self.registries.biome_client_projection(holder_id))
            .map(|projection| &projection.environment_attribute_modifiers)
    }

    fn gaussian_environment_attribute_maps(
        &self,
        position: [f64; 3],
    ) -> Vec<(ClientEnvironmentAttributeMap, f64)> {
        if !position.into_iter().all(f64::is_finite) {
            return vec![(empty_environment_attribute_map(), 1.0)];
        }
        let sample_position = [
            position[0] * 0.25 - 0.5,
            position[1] * 0.25 - 0.5,
            position[2] * 0.25 - 0.5,
        ];
        let integral = sample_position.map(|coordinate| coordinate.floor() as i32);
        let relative = [
            sample_position[0] - f64::from(integral[0]),
            sample_position[1] - f64::from(integral[1]),
            sample_position[2] - f64::from(integral[2]),
        ];
        // Pre-resolve the <= 3x3 chunk columns and 6 vertical section indices
        // covering the 6x6x6 quart neighborhood once instead of hashing
        // ChunkPos 216 times per camera/environment sample.
        let sample_xs: [i32; 6] = std::array::from_fn(|x| integral[0].saturating_add(x as i32 - 2));
        let sample_zs: [i32; 6] = std::array::from_fn(|z| integral[2].saturating_add(z as i32 - 2));
        let min_cx = sample_xs[0].div_euclid(4);
        let min_cz = sample_zs[0].div_euclid(4);
        let max_cx = sample_xs[5].div_euclid(4);
        let max_cz = sample_zs[5].div_euclid(4);
        let span_cx = usize::try_from(max_cx.saturating_sub(min_cx))
            .unwrap_or(2)
            .min(2);
        let span_cz = usize::try_from(max_cz.saturating_sub(min_cz))
            .unwrap_or(2)
            .min(2);
        let mut chunk_grid: [Option<&crate::ChunkColumn>; 9] = [None; 9];
        for dz in 0..=span_cz {
            for dx in 0..=span_cx {
                chunk_grid[dz * 3 + dx] = self.probe_chunk(crate::ChunkPos {
                    x: min_cx.saturating_add(dx as i32),
                    z: min_cz.saturating_add(dz as i32),
                });
            }
        }
        let y_samples: [(f64, Option<(usize, u8)>); 6] = std::array::from_fn(|y| {
            let sample_y = integral[1].saturating_add(y as i32 - 2);
            (
                gaussian_axis_weight(y, relative[1]),
                self.clamped_biome_section_and_local_y(sample_y),
            )
        });
        let x_samples: [(f64, usize, u8); 6] = std::array::from_fn(|x| {
            let qx = sample_xs[x];
            let cx_idx = usize::try_from(qx.div_euclid(4).saturating_sub(min_cx))
                .unwrap_or(0)
                .min(2);
            (
                gaussian_axis_weight(x, relative[0]),
                cx_idx,
                qx.rem_euclid(4) as u8,
            )
        });
        // Vanilla's 6x6x6 cubic sampler often visits the same biome holder for
        // every source cell. Aggregate those cheap integer holder samples
        // before resolving registry projections: otherwise one camera sample
        // repeats the holder -> resource id -> projection lookup 216 times.
        let mut weighted_holders: Vec<(Option<i32>, f64)> = Vec::with_capacity(4);
        for (z, &qz) in sample_zs.iter().enumerate() {
            let weight_z = gaussian_axis_weight(z, relative[2]);
            let cz_idx = usize::try_from(qz.div_euclid(4).saturating_sub(min_cz))
                .unwrap_or(0)
                .min(2);
            let local_qz = qz.rem_euclid(4) as u8;
            let row_offset = cz_idx * 3;
            for &(weight_x, cx_idx, local_qx) in &x_samples {
                let chunk = chunk_grid[row_offset + cx_idx];
                let weight_xz = weight_x * weight_z;
                for &(weight_y, y_cell) in &y_samples {
                    let weight = weight_xz * weight_y;
                    let holder_id =
                        chunk
                            .zip(y_cell)
                            .and_then(|(chunk, (section_index, local_qy))| {
                                Self::noise_biome_holder_id_in_chunk(
                                    chunk,
                                    section_index,
                                    local_qx,
                                    local_qy,
                                    local_qz,
                                )
                            });
                    if let Some((_, accumulated_weight)) = weighted_holders
                        .iter_mut()
                        .find(|(existing, _)| *existing == holder_id)
                    {
                        *accumulated_weight += weight;
                    } else {
                        weighted_holders.push((holder_id, weight));
                    }
                }
            }
        }
        let mut weighted_sources: Vec<(ClientEnvironmentAttributeMap, f64)> =
            Vec::with_capacity(weighted_holders.len());
        for (holder_id, weight) in weighted_holders {
            let source = holder_id
                .and_then(|holder_id| self.registries.biome_client_projection(holder_id))
                .map(|projection| projection.environment_attribute_modifiers.clone())
                .unwrap_or_else(empty_environment_attribute_map);
            if let Some((_, accumulated_weight)) = weighted_sources
                .iter_mut()
                .find(|(existing, _)| existing.same_source(&source))
            {
                *accumulated_weight += weight;
            } else {
                weighted_sources.push((source, weight));
            }
        }
        weighted_sources
    }
}

fn empty_environment_attribute_map() -> ClientEnvironmentAttributeMap {
    static EMPTY: std::sync::OnceLock<ClientEnvironmentAttributeMap> = std::sync::OnceLock::new();
    EMPTY
        .get_or_init(ClientEnvironmentAttributeMap::default)
        .clone()
}

fn camera_environment_attributes(
    source: &ClientEnvironmentAttributes,
) -> ClientEnvironmentAttributes {
    ClientEnvironmentAttributes {
        fog_color: source.fog_color,
        fog_start_distance: source.fog_start_distance,
        fog_end_distance: source.fog_end_distance,
        sky_fog_end_distance: source.sky_fog_end_distance,
        cloud_fog_end_distance: source.cloud_fog_end_distance,
        water_fog_color: source.water_fog_color,
        water_fog_start_distance: source.water_fog_start_distance,
        water_fog_end_distance: source.water_fog_end_distance,
        sky_color: source.sky_color,
        sunrise_sunset_color: source.sunrise_sunset_color,
        sun_angle_degrees: source.sun_angle_degrees,
        moon_angle_degrees: source.moon_angle_degrees,
        star_angle_degrees: source.star_angle_degrees,
        moon_phase: source.moon_phase,
        star_brightness: source.star_brightness,
        ..ClientEnvironmentAttributes::default()
    }
}

fn quart_coordinate(coordinate: f64) -> Option<i32> {
    coordinate
        .is_finite()
        .then(|| (coordinate.floor() as i32).div_euclid(4))
}

fn gaussian_axis_weight(index: usize, relative: f64) -> f64 {
    let start = GAUSSIAN_SAMPLE_KERNEL[index + 1];
    let end = GAUSSIAN_SAMPLE_KERNEL[index];
    start + relative * (end - start)
}

fn environment_sampling_error(
    error: EnvironmentAttributeProjectionError,
) -> RegistryProjectionError {
    RegistryProjectionError {
        resource_id: "minecraft:worldgen/biome".to_string(),
        field: error.field.to_string(),
        reason: error.reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc};

    use bbb_protocol::environment_particles::ParticleOptions;
    use bbb_protocol::packets::{ChunkBiomeData, ChunkPos as ProtocolChunkPos, ChunksBiomes};
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use super::*;
    use crate::{
        BiomeClientProjection, BiomeGrassColorModifier, BiomeTemperatureModifier, ChunkColumn,
        ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, RegistryContentState, RegistryPacketEntry, WorldCardinalLighting,
        WorldDimension, WorldDimensionTypeProperties, WorldLevelInfo, WorldSkybox,
    };

    use super::super::environment_attributes::{
        decode_client_environment_attribute_map, EnvironmentModifier, EnvironmentTrackArgument,
    };

    use super::super::timelines::{
        TimelineDefinition, TimelineEasing, TimelineKeyframe, TimelineTrackDefinition,
    };

    #[test]
    fn environment_position_orders_dimension_biome_timeline_and_sanitization() {
        let mut world = environment_world();

        let direct = world
            .sampled_client_environment_attributes_at(
                [8.0, 32.0, 8.0],
                EnvironmentBiomeSampling::Direct,
            )
            .unwrap();
        assert_eq!(direct.fog_color, Some([100, 200, 250]));
        assert_eq!(direct.water_fog_end_distance, Some(210.0));
        assert_eq!(direct.sun_angle_degrees, Some(180.0));
        assert_eq!(direct.music_volume, Some(0.8));
        assert_eq!(direct.sky_light_factor, Some(1.0));

        insert_gaussian_neighborhood(&mut world);
        assert_eq!(
            world
                .gaussian_environment_attribute_maps([8.0, 32.0, 8.0])
                .len(),
            2
        );
        let gaussian = world
            .sampled_client_environment_attributes_at(
                [8.0, 32.0, 8.0],
                EnvironmentBiomeSampling::Gaussian,
            )
            .unwrap();
        assert_eq!(gaussian.fog_color, Some([50, 100, 125]));
        assert!((gaussian.water_fog_end_distance.unwrap() - 160.0).abs() < 0.0001);
        assert_eq!(gaussian.sun_angle_degrees, Some(90.0));
        assert_eq!(gaussian.music_volume, Some(0.8));
        assert_eq!(gaussian.sky_light_factor, Some(1.0));
    }

    #[test]
    fn environment_position_observes_holder_reorder_and_biome_replacement() {
        let mut world = environment_world();
        world.insert_decoded_chunk(environment_chunk(
            ChunkPos { x: 0, z: 0 },
            single_value_container(PaletteDomain::Biomes, 64, 0),
        ));
        let sample = |world: &WorldStore| {
            world
                .sampled_client_environment_attributes_at(
                    [1.0, 32.0, 1.0],
                    EnvironmentBiomeSampling::Direct,
                )
                .unwrap()
                .water_fog_end_distance
                .unwrap()
        };
        assert_eq!(sample(&world), 110.0);

        world
            .registries
            .contents
            .get_mut("minecraft:worldgen/biome")
            .unwrap()
            .entries
            .swap(0, 1);
        assert_eq!(sample(&world), 210.0);

        world
            .registries
            .contents
            .get_mut("minecraft:worldgen/biome")
            .unwrap()
            .entries
            .swap(0, 1);
        world
            .apply_biome_update(ChunksBiomes {
                chunks: vec![ChunkBiomeData {
                    pos: ProtocolChunkPos { x: 0, z: 0 },
                    raw_biomes: single_biome_payload(1, world.dimension().section_count()),
                }],
            })
            .unwrap();
        assert_eq!(sample(&world), 210.0);
    }

    #[test]
    fn ambient_particle_query_matches_full_projection_order() {
        let mut world = environment_world();
        world.level.as_mut().unwrap().timeline_resource_ids.clear();
        let position = [8.0, 32.0, 8.0];
        let unloaded_position = [10_000.0, 32.0, 10_000.0];
        assert!(world
            .sampled_client_ambient_particles_at(unloaded_position)
            .unwrap()
            .is_empty());
        let dimension_particles = vec![ambient_particle("minecraft:crimson_spore", 0.25)];
        world
            .level
            .as_mut()
            .unwrap()
            .environment_attributes
            .ambient_particles = Some(dimension_particles.clone());
        assert_eq!(
            world
                .sampled_client_ambient_particles_at(unloaded_position)
                .unwrap(),
            dimension_particles
        );

        let full_biome = world
            .sampled_client_environment_attributes_at(position, EnvironmentBiomeSampling::Direct)
            .unwrap()
            .ambient_particles
            .unwrap();
        assert_eq!(
            world.sampled_client_ambient_particles_at(position).unwrap(),
            full_biome
        );
        assert_eq!(full_biome[0].particle.resource_id, "minecraft:white_ash");

        world.level.as_mut().unwrap().timeline_resource_ids =
            vec!["example:environment".to_string()];
        let full_timeline = world
            .sampled_client_environment_attributes_at(position, EnvironmentBiomeSampling::Direct)
            .unwrap()
            .ambient_particles
            .unwrap();
        assert_eq!(
            world.sampled_client_ambient_particles_at(position).unwrap(),
            full_timeline
        );
        assert_eq!(full_timeline[0].particle.resource_id, "minecraft:ash");
        world
            .level
            .as_mut()
            .unwrap()
            .environment_attributes
            .ambient_particles = None;
        assert_eq!(
            world
                .sampled_client_ambient_particles_at(unloaded_position)
                .unwrap(),
            full_timeline
        );
    }

    #[test]
    fn camera_query_matches_full_gaussian_visual_projection_only() {
        let mut world = environment_world();
        insert_gaussian_neighborhood(&mut world);
        let position = [8.0, 32.0, 8.0];
        let full = world
            .sampled_client_environment_attributes_at(position, EnvironmentBiomeSampling::Gaussian)
            .unwrap();
        let camera = world
            .sampled_client_camera_environment_attributes_at(position)
            .unwrap();

        assert_eq!(camera, camera_environment_attributes(&full));
        assert_eq!(camera.fog_color, Some([50, 100, 125]));
        assert!((camera.water_fog_end_distance.unwrap() - 160.0).abs() < 0.0001);
        assert_eq!(camera.sun_angle_degrees, Some(90.0));
        assert_eq!(camera.music_volume, None);
        assert_eq!(camera.ambient_particles, None);
        assert_eq!(camera.sky_light_factor, None);
    }

    #[test]
    fn environment_position_clamps_quart_y_and_missing_data_is_noop() {
        let mut world = environment_world();
        let mut sections = Vec::new();
        for index in 0..world.dimension().section_count() {
            sections.push(environment_section(single_value_container(
                PaletteDomain::Biomes,
                64,
                if index + 1 == world.dimension().section_count() {
                    1
                } else {
                    0
                },
            )));
        }
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections,
            block_entities: std::sync::Arc::default(),
            light: LightData::default(),
        });

        let clamped = world
            .sampled_client_environment_attributes_at(
                [1.0, 10_000.0, 1.0],
                EnvironmentBiomeSampling::Direct,
            )
            .unwrap();
        assert_eq!(clamped.water_fog_end_distance, Some(210.0));

        let missing = world
            .sampled_client_environment_attributes_at(
                [10_000.0, 32.0, 10_000.0],
                EnvironmentBiomeSampling::Gaussian,
            )
            .unwrap();
        assert_eq!(missing.water_fog_end_distance, Some(210.0));
        assert_eq!(missing.fog_color, None);

        world.clear_client_level();
        let reset = world
            .sampled_client_environment_attributes_at(
                [1.0, 32.0, 1.0],
                EnvironmentBiomeSampling::Direct,
            )
            .unwrap();
        assert_eq!(reset, ClientEnvironmentAttributes::default());
    }

    fn environment_world() -> WorldStore {
        let dimension = WorldDimension {
            min_y: 0,
            height: 64,
        };
        let mut world = WorldStore::with_dimension(dimension);
        world.level = Some(WorldLevelInfo {
            dimension: "example:dimension".to_string(),
            dimension_type_id: i32::MAX,
            dimension_type_name: None,
            dimension_type_properties: WorldDimensionTypeProperties::default(),
            biome_zoom_seed: 0,
            default_clock_resource_id: None,
            timeline_resource_ids: vec!["example:environment".to_string()],
            has_sky_light: true,
            has_ceiling: false,
            cardinal_lighting: WorldCardinalLighting::Default,
            skybox: WorldSkybox::Overworld,
            ambient_light: 0.0,
            environment_attributes: ClientEnvironmentAttributes {
                water_fog_end_distance: Some(200.0),
                sky_light_factor: Some(0.8),
                ..ClientEnvironmentAttributes::default()
            },
            last_death_location: None,
            sea_level: 63,
            is_debug: false,
            is_flat: false,
        });
        install_biomes(&mut world);
        install_timeline(&mut world);
        world.insert_decoded_chunk(environment_chunk(
            ChunkPos { x: 0, z: 0 },
            split_x_biome_container(),
        ));
        world
    }

    fn install_biomes(world: &mut WorldStore) {
        let left = environment_map([
            modifier(
                "minecraft:visual/water_fog_end_distance",
                "multiply",
                NbtSummaryValue::Float(0.5_f32.to_bits()),
            ),
            entry(
                "minecraft:visual/fog_color",
                NbtSummaryValue::string("#000000"),
            ),
            entry(
                "minecraft:visual/sun_angle",
                NbtSummaryValue::Float(0.0_f32.to_bits()),
            ),
            entry(
                "minecraft:audio/music_volume",
                NbtSummaryValue::Float(0.2_f32.to_bits()),
            ),
            modifier(
                "minecraft:visual/sky_light_factor",
                "multiply",
                NbtSummaryValue::Float(2.0_f32.to_bits()),
            ),
            entry(
                "minecraft:visual/ambient_particles",
                ambient_particles_value("minecraft:crimson_spore", 0.25),
            ),
        ]);
        let right = environment_map([
            modifier(
                "minecraft:visual/water_fog_end_distance",
                "multiply",
                NbtSummaryValue::Float(1.0_f32.to_bits()),
            ),
            entry(
                "minecraft:visual/fog_color",
                NbtSummaryValue::string("#64c8fa"),
            ),
            entry(
                "minecraft:visual/sun_angle",
                NbtSummaryValue::Float(180.0_f32.to_bits()),
            ),
            entry(
                "minecraft:audio/music_volume",
                NbtSummaryValue::Float(0.8_f32.to_bits()),
            ),
            modifier(
                "minecraft:visual/sky_light_factor",
                "multiply",
                NbtSummaryValue::Float(2.0_f32.to_bits()),
            ),
            entry(
                "minecraft:visual/ambient_particles",
                ambient_particles_value("minecraft:white_ash", 0.5),
            ),
        ]);
        let entries = vec![
            RegistryPacketEntry::stub("example:left"),
            RegistryPacketEntry::stub("example:right"),
        ];
        world.registries.contents.insert(
            "minecraft:worldgen/biome".to_string(),
            RegistryContentState::from_entries("minecraft:worldgen/biome", entries),
        );
        world.registries.typed.biomes = BTreeMap::from([
            ("example:left".to_string(), biome_projection(left)),
            ("example:right".to_string(), biome_projection(right)),
        ]);
    }

    fn install_timeline(world: &mut WorldStore) {
        world.registries.contents.insert(
            "minecraft:timeline".to_string(),
            RegistryContentState::from_entries(
                "minecraft:timeline",
                vec![RegistryPacketEntry::stub("example:environment")],
            ),
        );
        world.registries.typed.timelines.insert(
            "example:environment".to_string(),
            TimelineDefinition {
                resource_id: "example:environment".to_string(),
                clock_id: 0,
                period_ticks: None,
                tracks: vec![
                    TimelineTrackDefinition {
                        attribute: "minecraft:visual/water_fog_end_distance".to_string(),
                        modifier: EnvironmentModifier::Add,
                        keyframes: vec![TimelineKeyframe {
                            ticks: 0,
                            argument: EnvironmentTrackArgument::Float(10.0),
                        }],
                        easing: TimelineEasing::Simple("linear".to_string()),
                    },
                    TimelineTrackDefinition {
                        attribute: AMBIENT_PARTICLES_ATTRIBUTE.to_string(),
                        modifier: EnvironmentModifier::Override,
                        keyframes: vec![TimelineKeyframe {
                            ticks: 0,
                            argument: EnvironmentTrackArgument::AmbientParticles(Arc::from([
                                ambient_particle("minecraft:ash", 0.75),
                            ])),
                        }],
                        easing: TimelineEasing::Simple("linear".to_string()),
                    },
                ],
            },
        );
    }

    fn environment_map<const N: usize>(
        entries: [NbtSummaryEntry; N],
    ) -> ClientEnvironmentAttributeMap {
        let registries = crate::RegistrySet::vanilla_26_1();
        decode_client_environment_attribute_map(&entries, true, registries.frozen_access()).unwrap()
    }

    fn biome_projection(
        environment_attribute_modifiers: ClientEnvironmentAttributeMap,
    ) -> BiomeClientProjection {
        BiomeClientProjection {
            has_precipitation: true,
            temperature: 0.5,
            temperature_modifier: BiomeTemperatureModifier::None,
            downfall: 0.5,
            water_color: [0x3f, 0x76, 0xe4],
            foliage_color: None,
            dry_foliage_color: None,
            grass_color: None,
            grass_color_modifier: BiomeGrassColorModifier::None,
            environment_attribute_modifiers,
        }
    }

    fn insert_gaussian_neighborhood(world: &mut WorldStore) {
        for z in -1..=1 {
            for x in -1..=1 {
                world.insert_decoded_chunk(environment_chunk(
                    ChunkPos { x, z },
                    match x.cmp(&0) {
                        std::cmp::Ordering::Less => {
                            single_value_container(PaletteDomain::Biomes, 64, 0)
                        }
                        std::cmp::Ordering::Equal => split_x_biome_container(),
                        std::cmp::Ordering::Greater => {
                            single_value_container(PaletteDomain::Biomes, 64, 1)
                        }
                    },
                ));
            }
        }
    }

    fn environment_chunk(pos: ChunkPos, biomes: PalettedContainerData) -> ChunkColumn {
        ChunkColumn {
            pos,
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..4)
                .map(|_| environment_section(biomes.clone()))
                .collect(),
            block_entities: std::sync::Arc::default(),
            light: LightData::default(),
        }
    }

    fn environment_section(biomes: PalettedContainerData) -> std::sync::Arc<ChunkSection> {
        std::sync::Arc::new(ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
            biomes,
        })
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        value: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![value],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn split_x_biome_container() -> PalettedContainerData {
        let mut packed = 0_u64;
        for index in 0..64 {
            if index & 3 >= 2 {
                packed |= 1_u64 << index;
            }
        }
        PalettedContainerData {
            domain: PaletteDomain::Biomes,
            bits_per_entry: 1,
            palette_kind: PaletteKind::Local,
            palette_global_ids: vec![0, 1],
            packed_data: vec![packed as i64],
            entry_count: 64,
        }
    }

    fn single_biome_payload(holder_id: u8, section_count: usize) -> Vec<u8> {
        [0, holder_id].repeat(section_count)
    }

    fn modifier(name: &str, operation: &str, argument: NbtSummaryValue) -> NbtSummaryEntry {
        entry(
            name,
            NbtSummaryValue::Compound(vec![
                entry("modifier", NbtSummaryValue::string(operation)),
                entry("argument", argument),
            ]),
        )
    }

    fn ambient_particle(resource_id: &str, probability: f32) -> AmbientParticle {
        AmbientParticle {
            particle: ParticleOptions {
                resource_id: resource_id.to_string(),
                fields: vec![entry("type", NbtSummaryValue::string(resource_id))],
                option: Default::default(),
            },
            probability_bits: probability.to_bits(),
        }
    }

    fn ambient_particles_value(resource_id: &str, probability: f32) -> NbtSummaryValue {
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            entry(
                "particle",
                NbtSummaryValue::Compound(vec![entry(
                    "type",
                    NbtSummaryValue::string(resource_id),
                )]),
            ),
            entry("probability", NbtSummaryValue::Float(probability.to_bits())),
        ])])
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }
}
