use crate::{BlockPos, ClientEnvironmentAttributes, WorldStore};

use super::environment_attributes::{
    apply_spatial_environment_attribute_maps, sanitize_environment_attributes,
    ClientEnvironmentAttributeMap, EnvironmentAttributeProjectionError,
};
use super::timelines::{apply_environment_track, TimelineProjectionError};

const GAUSSIAN_SAMPLE_KERNEL: [f64; 7] = [0.0, 1.0, 4.0, 6.0, 4.0, 1.0, 0.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentBiomeSampling {
    Direct,
    Gaussian,
}

impl WorldStore {
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
    ) -> Result<ClientEnvironmentAttributes, TimelineProjectionError> {
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
    ) -> Result<ClientEnvironmentAttributes, TimelineProjectionError> {
        let Some(mut attributes) = self.dimension_environment_attributes() else {
            return Ok(ClientEnvironmentAttributes::default());
        };
        match sampling {
            EnvironmentBiomeSampling::Direct => self
                .environment_attribute_map_at_position(position)
                .apply_to(&mut attributes)
                .map_err(environment_sampling_error)?,
            EnvironmentBiomeSampling::Gaussian => {
                self.environment_attribute_map_at_position(position)
                    .apply_non_spatial_to(&mut attributes)
                    .map_err(environment_sampling_error)?;
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
                .map(|projection| projection.environment_attributes)
                .unwrap_or_else(|| level.environment_attributes()),
        )
    }

    fn apply_environment_timelines(
        &self,
        attributes: &mut ClientEnvironmentAttributes,
    ) -> Result<(), TimelineProjectionError> {
        for (timeline, total_ticks) in self.resolved_dimension_timelines()? {
            for track in &timeline.tracks {
                apply_environment_track(attributes, &timeline, track, total_ticks)?;
            }
        }
        Ok(())
    }

    fn environment_attribute_map_at_position(
        &self,
        position: [f64; 3],
    ) -> ClientEnvironmentAttributeMap {
        let [Some(quart_x), Some(quart_y), Some(quart_z)] = position.map(quart_coordinate) else {
            return ClientEnvironmentAttributeMap::default();
        };
        self.environment_attribute_map_at_quart(quart_x, quart_y, quart_z)
    }

    fn environment_attribute_map_at_quart(
        &self,
        quart_x: i32,
        quart_y: i32,
        quart_z: i32,
    ) -> ClientEnvironmentAttributeMap {
        self.noise_biome_holder_id_at_quart(quart_x, quart_y, quart_z)
            .and_then(|holder_id| {
                self.registries
                    .biome_client_projection(holder_id)
                    .ok()
                    .flatten()
            })
            .map(|projection| projection.environment_attribute_modifiers)
            .unwrap_or_default()
    }

    fn gaussian_environment_attribute_maps(
        &self,
        position: [f64; 3],
    ) -> Vec<(ClientEnvironmentAttributeMap, f64)> {
        if !position.into_iter().all(f64::is_finite) {
            return vec![(ClientEnvironmentAttributeMap::default(), 1.0)];
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
        let mut weighted_sources: Vec<(ClientEnvironmentAttributeMap, f64)> = Vec::new();
        for z in 0..6 {
            let weight_z = gaussian_axis_weight(z, relative[2]);
            let sample_z = integral[2].saturating_add(z as i32 - 2);
            for x in 0..6 {
                let weight_x = gaussian_axis_weight(x, relative[0]);
                let sample_x = integral[0].saturating_add(x as i32 - 2);
                for y in 0..6 {
                    let weight_y = gaussian_axis_weight(y, relative[1]);
                    let sample_y = integral[1].saturating_add(y as i32 - 2);
                    let weight = weight_x * weight_y * weight_z;
                    let source =
                        self.environment_attribute_map_at_quart(sample_x, sample_y, sample_z);
                    if let Some((_, accumulated_weight)) = weighted_sources
                        .iter_mut()
                        .find(|(existing, _)| existing.same_source(&source))
                    {
                        *accumulated_weight += weight;
                    } else {
                        weighted_sources.push((source, weight));
                    }
                }
            }
        }
        weighted_sources
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
) -> TimelineProjectionError {
    TimelineProjectionError {
        resource_id: "minecraft:worldgen/biome".to_string(),
        field: error.field.to_string(),
        reason: error.reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::packets::{ChunkBiomeData, ChunkPos as ProtocolChunkPos, ChunksBiomes};
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use super::*;
    use crate::{
        BiomeClientProjection, BiomeGrassColorModifier, BiomeTemperatureModifier, ChunkColumn,
        ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, RegistryContentState, RegistryElementSource, RegistryEntrySource,
        RegistryPacketEntry, TimelineClockReference, TimelineDefinition, TimelineEasing,
        TimelineKeyframe, TimelineModifier, TimelineTrackDefinition, WorldCardinalLighting,
        WorldDimension, WorldDimensionTypeProperties, WorldLevelInfo, WorldSkybox,
    };

    use super::super::environment_attributes::{
        decode_client_environment_attribute_map, EnvironmentTrackArgument,
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
        assert_eq!(gaussian.sun_angle_degrees, Some(180.0));
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
            block_entities: Vec::new(),
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
                NbtSummaryValue::String("#000000".to_string()),
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
        ]);
        let right = environment_map([
            modifier(
                "minecraft:visual/water_fog_end_distance",
                "multiply",
                NbtSummaryValue::Float(1.0_f32.to_bits()),
            ),
            entry(
                "minecraft:visual/fog_color",
                NbtSummaryValue::String("#64c8fa".to_string()),
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
        ]);
        let entries = vec![
            RegistryPacketEntry::stub("example:left"),
            RegistryPacketEntry::stub("example:right"),
        ];
        world.registries.contents.insert(
            "minecraft:worldgen/biome".to_string(),
            RegistryContentState {
                registry: "minecraft:worldgen/biome".to_string(),
                packet_count: 1,
                entries,
                duplicate_entry_ids: BTreeMap::new(),
            },
        );
        world.registries.typed.biomes = BTreeMap::from([
            ("example:left".to_string(), biome_projection(left)),
            ("example:right".to_string(), biome_projection(right)),
        ]);
    }

    fn install_timeline(world: &mut WorldStore) {
        world.registries.contents.insert(
            "minecraft:timeline".to_string(),
            RegistryContentState {
                registry: "minecraft:timeline".to_string(),
                packet_count: 1,
                entries: vec![RegistryPacketEntry::stub("example:environment")],
                duplicate_entry_ids: BTreeMap::new(),
            },
        );
        world.registries.typed.timelines.insert(
            "example:environment".to_string(),
            TimelineDefinition {
                resource_id: "example:environment".to_string(),
                element_source: RegistryElementSource::KnownPackJson,
                clock: TimelineClockReference {
                    holder_id: 0,
                    resource_id: "example:clock".to_string(),
                    source: RegistryEntrySource::Server,
                },
                period_ticks: None,
                tracks: vec![TimelineTrackDefinition {
                    attribute: "minecraft:visual/water_fog_end_distance".to_string(),
                    modifier: TimelineModifier::Add,
                    keyframes: vec![TimelineKeyframe {
                        ticks: 0,
                        value: NbtSummaryValue::Float(10.0_f32.to_bits()),
                        argument: EnvironmentTrackArgument::Float(10.0),
                    }],
                    easing: TimelineEasing::Simple("linear".to_string()),
                }],
                time_markers: Vec::new(),
            },
        );
    }

    fn environment_map<const N: usize>(
        entries: [NbtSummaryEntry; N],
    ) -> ClientEnvironmentAttributeMap {
        decode_client_environment_attribute_map(&entries, true).unwrap()
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
            block_entities: Vec::new(),
            light: LightData::default(),
        }
    }

    fn environment_section(biomes: PalettedContainerData) -> ChunkSection {
        ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
            biomes,
        }
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
                entry("modifier", NbtSummaryValue::String(operation.to_string())),
                entry("argument", argument),
            ]),
        )
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }
}
