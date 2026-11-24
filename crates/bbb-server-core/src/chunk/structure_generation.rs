//! Generator-independent structure starts over actual terrain and biome sources.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;
use bbb_server_datapack::debug_generator::DEBUG_GENERATOR_ID;
use bbb_server_datapack::dimension_type::DimensionTypeShape;
use bbb_server_datapack::flat_generator::{
    FlatLevelSource, FlatStructureOverrides, FLAT_GENERATOR_ID,
};
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::fluid::{block_state_blocks_motion, block_state_is_air};
use bbb_server_types::save::SaveResult;
use bbb_server_types::schema_support::invalid_schema;
use bbb_server_worldgen::noise::biome::NoiseBiomeSearch;
use bbb_server_worldgen::noise::buried_treasure::BuriedTreasureStructureStart;
use bbb_server_worldgen::noise::end_city::EndCityStructureStart;
use bbb_server_worldgen::noise::igloo::IglooStructureStart;
use bbb_server_worldgen::noise::mineshaft::MineshaftStructureStart;
use bbb_server_worldgen::noise::nether_fortress::{FortressStructureStart, FORTRESS_TYPE_ID};
use bbb_server_worldgen::noise::nether_fossil::NetherFossilStructureStart;
use bbb_server_worldgen::noise::ocean_monument::{
    required_surrounding_biomes, OceanMonumentStructureStart,
};
use bbb_server_worldgen::noise::ocean_ruin::OceanRuinStructureStart;
use bbb_server_worldgen::noise::ruined_portal::RuinedPortalStructureStart;
use bbb_server_worldgen::noise::scattered::ScatteredFeatureStructureStart;
use bbb_server_worldgen::noise::shipwreck::ShipwreckStructureStart;
use bbb_server_worldgen::noise::stronghold::StrongholdStructureStart;
use bbb_server_worldgen::noise::structure_pieces::JigsawHeightAccess;
use bbb_server_worldgen::noise::structure_starts::{
    JigsawStructureChunkData, JigsawStructureStart, NoiseStructureChunkData,
};
use bbb_server_worldgen::noise::structures::{
    NoiseStructurePlacement, NoiseStructurePlan, StructureHeightmap, StructureStartSetOwner,
};
use bbb_server_worldgen::noise::woodland_mansion::WoodlandMansionStructureStart;

use crate::chunk::noise_generator::{
    biome_cold_enough_to_snow, NoiseFinalDensityProgram, NoiseStructureRingPositions,
};

mod command_queries;
mod families;
mod forced;
mod placement;
mod registry;
pub(crate) mod restored;

type ColumnStateCache = rustc_hash::FxHashMap<(i32, i32), Option<(i32, Vec<i32>)>>;

pub(crate) enum StructureTerrain<'a> {
    Noise(&'a NoiseFinalDensityProgram),
    Flat(&'a FlatLevelSource),
    Debug,
}

pub(crate) struct StructureGenerationContext<'a> {
    pub(crate) item_publication:
        Option<&'a crate::chunk::generation_publication::GenerationItemPublication>,
    pub(crate) startup: &'a DatapackStartup,
    pub(crate) plan: &'a NoiseStructurePlan,
    pub(crate) world_seed: i64,
    pub(crate) dimension_shape: DimensionTypeShape,
    pub(crate) label: &'a str,
    pub(crate) sea_level: i32,
    pub(crate) terrain: StructureTerrain<'a>,
    pub(crate) biome_search: RefCell<Option<NoiseBiomeSearch>>,
}

impl StructureGenerationContext<'_> {
    // WorldGenerationContext.java:10-12: generation height and dimension bounds differ.
    fn generation_bounds(&self) -> (i32, i32) {
        let (min_y, height) = match &self.terrain {
            StructureTerrain::Noise(program) => (program.min_y(), program.height()),
            StructureTerrain::Flat(_) | StructureTerrain::Debug => (0, 384),
        };
        (
            min_y.max(self.dimension_shape.min_y),
            height.min(self.dimension_shape.height),
        )
    }

    fn biome_name_at(&self, pos: BlockPos) -> Option<String> {
        match &self.terrain {
            StructureTerrain::Flat(generator) => Some(generator.settings().biome().to_owned()),
            StructureTerrain::Debug => Some("minecraft:plains".to_owned()),
            StructureTerrain::Noise(program) => {
                let mut search = self.biome_search.borrow_mut();
                let biome = program.structure_biome_at_quart(
                    pos.x.div_euclid(4),
                    pos.y.div_euclid(4),
                    pos.z.div_euclid(4),
                    search.as_mut()?,
                )?;
                self.startup
                    .registries
                    .registry("minecraft:worldgen/biome")?
                    .elements
                    .get(usize::try_from(biome).ok()?)
                    .map(|element| element.id.clone())
            }
        }
    }

    fn fresh_base_height(&self, x: i32, z: i32, kind: i32) -> Option<i32> {
        match &self.terrain {
            StructureTerrain::Noise(program) => program.fresh_base_height(x, z, kind),
            StructureTerrain::Flat(generator) => generator.base_height(kind),
            // DebugLevelSource.java:85-92: structure terrain excludes the FEATURES display grid.
            StructureTerrain::Debug => Some(0),
        }
    }

    fn fresh_column_state(&self, x: i32, y: i32, z: i32) -> Option<i32> {
        let air = bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
        )?;
        match &self.terrain {
            StructureTerrain::Noise(program) => program.fresh_column_state(x, y, z),
            // FlatLevelSource.java:108-118: only the dimension-height prefix belongs to the base column.
            StructureTerrain::Flat(generator) => {
                let index = y - generator.min_y();
                if index < 0 || index >= generator.height() {
                    return Some(air);
                }
                generator.base_column_state(y)
            }
            StructureTerrain::Debug => Some(air),
        }
    }

    fn fresh_column_state_cached(
        &self,
        cache: &mut ColumnStateCache,
        x: i32,
        y: i32,
        z: i32,
    ) -> Option<i32> {
        if let StructureTerrain::Noise(program) = &self.terrain {
            let column = cache
                .entry((x, z))
                .or_insert_with(|| program.fresh_base_column(x, z));
            let (min_y, states) = column.as_ref()?;
            let air = bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
            )?;
            let Some(offset) = y.checked_sub(*min_y).and_then(|d| usize::try_from(d).ok()) else {
                return Some(air);
            };
            return Some(states.get(offset).copied().unwrap_or(air));
        }
        self.fresh_column_state(x, y, z)
    }

    fn fresh_column_opaque_cached(
        &self,
        cache: &mut ColumnStateCache,
        x: i32,
        y: i32,
        z: i32,
        heightmap: StructureHeightmap,
    ) -> Option<bool> {
        let state = self.fresh_column_state_cached(cache, x, y, z)?;
        match heightmap {
            StructureHeightmap::WorldSurfaceWg => Some(!block_state_is_air(state)),
            StructureHeightmap::OceanFloorWg => Some(block_state_blocks_motion(state)),
            _ => None,
        }
    }
}

struct StructureHeightAccess<'a, 'b> {
    context: &'a StructureGenerationContext<'b>,
    heights: RefCell<BTreeMap<(i32, i32, i32), i32>>,
}

impl<'a, 'b> StructureHeightAccess<'a, 'b> {
    fn new(context: &'a StructureGenerationContext<'b>) -> Self {
        Self {
            context,
            heights: RefCell::new(BTreeMap::new()),
        }
    }
}

impl JigsawHeightAccess for StructureHeightAccess<'_, '_> {
    fn generation_bounds(&self) -> Option<(i32, i32)> {
        Some(self.context.generation_bounds())
    }

    fn first_free_height(&self, x: i32, z: i32, heightmap: StructureHeightmap) -> Option<i32> {
        let kind = match heightmap {
            StructureHeightmap::WorldSurfaceWg => VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
            StructureHeightmap::OceanFloorWg => VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID,
            _ => return None,
        };
        if let Some(height) = self.heights.borrow().get(&(x, z, kind)) {
            return Some(*height);
        }
        let height = self.context.fresh_base_height(x, z, kind)?;
        self.heights.borrow_mut().insert((x, z, kind), height);
        Some(height)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OrdinaryStructureProgram {
    item_publication: Option<crate::chunk::generation_publication::GenerationItemPublication>,
    startup: std::sync::Arc<DatapackStartup>,
    plan: NoiseStructurePlan,
    seed: i64,
    shape: DimensionTypeShape,
    rings: NoiseStructureRingPositions,
    ordered_rings: BTreeMap<String, Vec<(i32, i32)>>,
    features: Option<bbb_server_worldgen::noise::features::NoiseFeaturesPlan>,
}

impl OrdinaryStructureProgram {
    pub(crate) fn features(
        &self,
    ) -> Option<&bbb_server_worldgen::noise::features::NoiseFeaturesPlan> {
        self.features.as_ref()
    }
    pub(crate) fn plan(&self) -> &NoiseStructurePlan {
        &self.plan
    }

    pub(crate) fn world_seed(&self) -> i64 {
        self.seed
    }

    pub(crate) fn ordered_ring_positions(&self, set: &str) -> Option<&[(i32, i32)]> {
        self.ordered_rings.get(set).map(Vec::as_slice)
    }

    pub(crate) fn compute_starts_ephemeral(
        &self,
        source: &crate::chunk::ChunkSource,
        x: i32,
        z: i32,
        generate_structures: bool,
    ) -> SaveResult<NoiseStructureChunkData> {
        if generate_structures {
            self.context(source)
                .generate_registered_structure_starts_status_task(x, z, &self.rings)
        } else {
            Ok(NoiseStructureChunkData::from_supplied_starts(x, z, [], []))
        }
    }

    pub(crate) fn context<'a>(
        &'a self,
        source: &'a crate::chunk::ChunkSource,
    ) -> StructureGenerationContext<'a> {
        let (terrain, label, sea_level) = match source {
            crate::chunk::ChunkSource::Flat(flat) => (
                StructureTerrain::Flat(flat),
                FLAT_GENERATOR_ID,
                flat.sea_level(),
            ),
            crate::chunk::ChunkSource::Debug(debug) => (
                StructureTerrain::Debug,
                DEBUG_GENERATOR_ID,
                debug.sea_level(),
            ),
            crate::chunk::ChunkSource::NoiseCarvers(_) => {
                unreachable!("ordinary structure program owns a flat or debug source")
            }
        };
        StructureGenerationContext {
            item_publication: self.item_publication.as_ref(),
            startup: &self.startup,
            plan: &self.plan,
            world_seed: self.seed,
            dimension_shape: self.shape,
            label,
            sea_level,
            terrain,
            biome_search: RefCell::new(None),
        }
    }
}

impl crate::chunk::ChunkStore {
    pub fn configure_ordinary_structure_generation(
        &mut self,
        startup: std::sync::Arc<DatapackStartup>,
        seed: i64,
        shape: DimensionTypeShape,
        generate_structures: bool,
        item_publication: Option<crate::chunk::generation_publication::GenerationItemPublication>,
    ) -> SaveResult<()> {
        if self.ordinary_structures.is_some() || !self.chunks.is_empty() {
            return Err(invalid_schema(
                "structure_generation",
                "ordinary structure settings must be frozen before loading columns",
            ));
        }
        if shape.min_y != self.min_y() || shape.height != self.section_count() as i32 * 16 {
            return Err(invalid_schema(
                "structure_generation",
                "structure dimension shape does not match the column store",
            ));
        }
        if let crate::chunk::ChunkSource::Flat(generator) = &mut self.source {
            generator.prepare_decoration_layers();
            self.generated = generator
                .generate_chunk_with_biome_registry(0, 0, self.biome_id, self.biome_registry_len)
                .into_column();
        }
        let biome = match &self.source {
            crate::chunk::ChunkSource::Flat(flat) => flat.settings().biome(),
            crate::chunk::ChunkSource::Debug(_) => "minecraft:plains",
            crate::chunk::ChunkSource::NoiseCarvers(_) => {
                return Err(invalid_schema(
                    "structure_generation",
                    "ordinary structure settings require a flat or debug source",
                ))
            }
        };
        let mut plan = NoiseStructurePlan::from_startup(&startup, &[biome])
            .map_err(|error| invalid_schema("structure_generation", error.to_string()))?;
        if let crate::chunk::ChunkSource::Flat(flat) = &self.source {
            if let Some(overrides) = flat.settings().structure_overrides() {
                let ids = match overrides {
                    FlatStructureOverrides::Sets(ids) => ids.clone(),
                    FlatStructureOverrides::Tag(tag) => {
                        let tag_id = tag.strip_prefix('#').unwrap_or(tag);
                        startup
                            .tags
                            .registries
                            .iter()
                            .find(|registry| {
                                registry.registry_id == "minecraft:worldgen/structure_set"
                            })
                            .and_then(|registry| registry.tags.get(tag_id))
                            .cloned()
                            .ok_or_else(|| {
                                invalid_schema(
                                    "structure_generation",
                                    format!("unknown structure set tag {tag}"),
                                )
                            })?
                    }
                };
                let registry = startup
                    .registries
                    .registry("minecraft:worldgen/structure_set")
                    .ok_or_else(|| {
                        invalid_schema("structure_generation", "structure set registry is absent")
                    })?;
                for id in &ids {
                    if !registry.elements.iter().any(|element| element.id == *id) {
                        return Err(invalid_schema(
                            "structure_generation",
                            format!("unknown structure set {id}"),
                        ));
                    }
                }
                plan.restrict_sets(&ids);
            }
        }
        let biome_id = startup
            .registries
            .registry("minecraft:worldgen/biome")
            .and_then(|registry| {
                registry
                    .elements
                    .iter()
                    .position(|element| element.id == biome)
            })
            .and_then(|index| i32::try_from(index).ok())
            .ok_or_else(|| invalid_schema("structure_generation", "fixed biome is absent"))?;
        let ring_seed = if matches!(self.source, crate::chunk::ChunkSource::Flat(_)) {
            0
        } else {
            seed
        };
        let mut rings = BTreeMap::new();
        let mut ordered_rings = BTreeMap::new();
        for set in plan.sets() {
            if let Some(positions) = set.placement.concentric_ring_positions_with(
                ring_seed,
                |x, z, radius, allowed, search_seed| {
                    if !allowed.contains(&biome_id) {
                        return None;
                    }
                    let mut random =
                        bbb_server_worldgen::random::WorldgenRandom::with_seed(search_seed);
                    bbb_server_worldgen::noise::biome::find_fixed_biome_horizontal(
                        x,
                        0,
                        z,
                        radius,
                        &mut random,
                    )
                    .map(|(x, _, z)| (x, z))
                },
            ) {
                rings.insert(
                    set.id.clone(),
                    positions.iter().map(|&(x, z)| [x, z]).collect(),
                );
                ordered_rings.insert(set.id.clone(), positions);
            }
        }
        let features = match &self.source {
            crate::chunk::ChunkSource::Flat(source) => {
                Some(compile_flat_features(&startup, source)?)
            }
            _ => None,
        };
        let program = OrdinaryStructureProgram {
            item_publication,
            startup,
            plan,
            seed,
            shape,
            rings,
            ordered_rings,
            features,
        };
        if generate_structures {
            program
                .context(&self.source)
                .preflight_registered_structure_start_owners(
                    &program.plan,
                    "structure_generation",
                )?;
        }
        self.ordinary_structures = Some(std::sync::Arc::new(program));
        self.structure_status =
            crate::chunk::structure_status::NoiseStructureStatusCache::new(generate_structures);
        Ok(())
    }

    pub(crate) fn ensure_ordinary_structure_references(
        &mut self,
        x: i32,
        z: i32,
    ) -> SaveResult<()> {
        self.prepare_generation_admission((x, z))?;
        let Some(program) = self.ordinary_structures.as_ref().cloned() else {
            return Ok(());
        };
        if self.structure_status.references_published([x, z]) {
            // A column past STRUCTURE_REFERENCES needs neither its window nor its starts loaded.
            return Ok(());
        }
        self.prepare_structure_window_admission((x, z))?;
        let context = program.context(&self.source);
        self.structure_status
            .ensure_references_chunk_with(x, z, |enabled, x, z| {
                if enabled {
                    context.generate_registered_structure_starts_status_task(x, z, &program.rings)
                } else {
                    Ok(NoiseStructureChunkData::from_supplied_starts(x, z, [], []))
                }
            })?;
        let sources = self
            .structure_status
            .component(x, z)
            .expect("ready references")
            .referenced_source_chunks();
        for source in sources {
            self.publish_structure_starts_column(source)?;
        }
        Ok(())
    }

    pub(crate) fn ensure_ordinary_structure_starts(&mut self, x: i32, z: i32) -> SaveResult<()> {
        self.prepare_generation_admission((x, z))?;
        let Some(program) = self.ordinary_structures.as_ref() else {
            return Ok(());
        };
        let context = program.context(&self.source);
        self.structure_status
            .ensure_starts_chunk_with(x, z, |enabled| {
                if enabled {
                    context.generate_registered_structure_starts_status_task(x, z, &program.rings)
                } else {
                    Ok(NoiseStructureChunkData::from_supplied_starts(x, z, [], []))
                }
            })
    }
}

pub(crate) fn is_empty_structure_document(value: &NbtSummaryValue) -> bool {
    let NbtSummaryValue::Compound(fields) = value else {
        return false;
    };
    if fields.len() != 2 {
        return false;
    }
    ["starts", "References"].into_iter().all(|name| {
        let mut matching = fields.iter().filter(|field| field.name == name);
        matches!(
            (matching.next(), matching.next()),
            (
                Some(bbb_protocol::NbtSummaryEntry {
                    value: NbtSummaryValue::Compound(entries),
                    ..
                }),
                None
            ) if entries.is_empty()
        )
    })
}

fn compile_flat_features(
    startup: &DatapackStartup,
    source: &FlatLevelSource,
) -> SaveResult<bbb_server_worldgen::noise::features::NoiseFeaturesPlan> {
    use bbb_server_datapack::worldgen::{
        ConfiguredFeature, ConfiguredFeatureHolder, PlacedFeature, PlacedFeatureHolder,
    };
    use bbb_server_worldgen::noise::features::{BiomeFeatureSelection, NoiseFeaturesPlan};
    use std::sync::Arc;

    let settings = source.settings();
    let path = "structure_generation.flat.features";
    let mut appended = Vec::new();
    // FlatLevelGeneratorSettings.java:137-162: optional lakes replace the biome's lake step.
    if settings.add_lakes() {
        for lake in [
            "minecraft:lake_lava_underground",
            "minecraft:lake_lava_surface",
        ] {
            appended.push((1, PlacedFeatureHolder::Named(lake.to_string())));
        }
    }
    // FlatLevelGeneratorSettings.java:165-175: inline holders follow copied biome features.
    for (y, state) in source.deferred_decoration_layers() {
        appended.push((
            10,
            PlacedFeatureHolder::Inline(Arc::new(PlacedFeature {
                feature: ConfiguredFeatureHolder::Inline(Arc::new(ConfiguredFeature::FillLayer {
                    height: y - source.min_y(),
                    state,
                })),
                placement: Vec::new(),
            })),
        ));
    }
    NoiseFeaturesPlan::from_registry_with_biome_feature_selection(
        &startup.registries,
        &startup.tags,
        settings.biome(),
        BiomeFeatureSelection {
            copy_original: settings.decoration()
                && (!settings.is_void_gen() || settings.biome() == "minecraft:the_void"),
            excluded_steps: if settings.add_lakes() {
                &[1, 3, 4]
            } else {
                &[3, 4]
            },
            appended: &appended,
        },
    )
    .map_err(|error| invalid_schema(path, error.to_string()))
}
