//! Source-owned structure-set planning before STRUCTURE_STARTS generation.

use crate::random::large_feature_random;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use crate::random::WorldgenRandom;

use crate::height_provider::HeightProvider;
use bbb_server_datapack::mob_spawn::StructureMobSpawnOverride;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_datapack::structure_pool::{StructureLiquidSettings, StructurePoolSet};
use bbb_server_datapack::worldgen::structure::{
    BiomeHolderSet, DecodedCommonPlacement, DecodedJigsawSettings, DecodedPoolAliasBinding,
    DecodedStructure, DecodedStructureHolder, DecodedStructurePlacement, DecodedStructureSet,
    DecodedTemplatePoolHolder,
};

use super::biome::NoiseBiomeSource;
use super::buried_treasure::BURIED_TREASURE_ID;
use super::density::NoiseDensitySource;
use super::igloo::IGLOO_TYPE_ID;
use super::ocean_monument::OCEAN_MONUMENT_TYPE_ID;
use super::ocean_ruin::{OceanRuinBiomeTemp, OceanRuinFloat, OceanRuinStructureSettings};
use super::ruined_portal::{ruined_portal_settings_from_decoded, RuinedPortalStructureSettings};
use super::scattered::ScatteredFeatureKind;
use super::shipwreck::ShipwreckStructureSettings;
use super::stronghold;
use super::woodland_mansion::WOODLAND_MANSION_TYPE_ID;

mod decoration;
mod end;
mod nether;
mod registered_random_spread;
pub use registered_random_spread::UnownedStructureStartCandidate;

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
const STRUCTURE_REGISTRY: &str = "minecraft:worldgen/structure";
const STRUCTURE_SET_REGISTRY: &str = "minecraft:worldgen/structure_set";
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoiseStructuresError {
    path: String,
    reason: String,
}

impl fmt::Display for NoiseStructuresError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.reason)
    }
}

impl std::error::Error for NoiseStructuresError {}

#[derive(Clone, Debug, PartialEq)]
pub struct NoiseStructurePlan {
    possible_biomes: Arc<BTreeSet<String>>,
    pools: Arc<StructurePoolSet>,
    structures: Arc<BTreeMap<String, NoiseStructureDefinition>>,
    structure_order: Arc<[String]>,
    sets: Vec<NoiseStructureSet>,
    set_index: BTreeMap<String, usize>,
    registered_sets: Arc<BTreeMap<String, NoiseStructureSet>>,
    registered_selected_attempt: Option<registered_random_spread::SelectedAttempt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoiseStructureDecorationInvocation {
    pub step: usize,
    pub step_index: usize,
    pub structure: String,
    pub feature_seed: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoiseStructureDefinition {
    pub id: String,
    pub type_id: String,
    pub biomes: BTreeSet<String>,
    pub step: StructureDecorationStep,
    pub terrain_adjustment: StructureTerrainAdjustment,
    pub spawn_overrides: Vec<StructureMobSpawnOverride>,
    pub jigsaw: Option<JigsawStructureSettings>,
    pub mineshaft: Option<MineshaftStructureSettings>,
    pub shipwreck: Option<ShipwreckStructureSettings>,
    pub ocean_ruin: Option<OceanRuinStructureSettings>,
    pub ruined_portal: Option<RuinedPortalStructureSettings>,
    pub nether_fossil_height: Option<HeightProvider>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MineshaftStructureSettings {
    pub kind: MineshaftType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MineshaftType {
    Normal,
    Mesa,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JigsawStructureSettings {
    pub start_pool: String,
    pub start_jigsaw_name: Option<String>,
    pub size: i32,
    pub start_height: HeightProvider,
    pub use_expansion_hack: bool,
    pub project_start_to_heightmap: Option<StructureHeightmap>,
    pub max_distance_from_center: JigsawMaxDistance,
    pub pool_aliases: Vec<PoolAliasBinding>,
    pub dimension_padding: DimensionPadding,
    pub liquid_settings: StructureLiquidSettings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureHeightmap {
    WorldSurfaceWg,
    WorldSurface,
    OceanFloorWg,
    OceanFloor,
    MotionBlocking,
    MotionBlockingNoLeaves,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JigsawMaxDistance {
    pub horizontal: i32,
    pub vertical: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PoolAliasBinding {
    Direct {
        alias: String,
        target: String,
    },
    Random {
        alias: String,
        targets: Vec<WeightedPool>,
    },
    RandomGroup {
        groups: Vec<WeightedAliasGroup>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightedPool {
    pub pool: String,
    pub weight: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightedAliasGroup {
    pub bindings: Vec<PoolAliasBinding>,
    pub weight: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DimensionPadding {
    pub bottom: i32,
    pub top: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructureDecorationStep {
    RawGeneration,
    Lakes,
    LocalModifications,
    UndergroundStructures,
    SurfaceStructures,
    Strongholds,
    UndergroundOres,
    UndergroundDecoration,
    FluidSprings,
    VegetalDecoration,
    TopLayerModification,
}

impl StructureDecorationStep {
    const ALL: [Self; 11] = [
        Self::RawGeneration,
        Self::Lakes,
        Self::LocalModifications,
        Self::UndergroundStructures,
        Self::SurfaceStructures,
        Self::Strongholds,
        Self::UndergroundOres,
        Self::UndergroundDecoration,
        Self::FluidSprings,
        Self::VegetalDecoration,
        Self::TopLayerModification,
    ];

    pub const fn ordinal(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructureTerrainAdjustment {
    None,
    Bury,
    BeardThin,
    BeardBox,
    Encapsulate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NoiseStructureSet {
    pub id: String,
    pub structures: Vec<NoiseStructureSelection>,
    pub placement: NoiseStructurePlacement,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoiseStructureSelection {
    pub structure: String,
    pub weight: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureStartSetOwner {
    Jigsaw,
    BuriedTreasure,
    Mineshaft,
    Shipwreck,
    OceanRuin,
    Igloo,
    DesertPyramid,
    JungleTemple,
    SwampHut,
    RuinedPortal,
    OceanMonument,
    WoodlandMansion,
    Stronghold,
    NetherComplex,
    NetherFossil,
    EndCity,
    RegisteredRandomSpread,
    Unowned,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NoiseStructurePlacement {
    RandomSpread {
        common: CommonPlacement,
        spacing: i32,
        separation: i32,
        spread_type: RandomSpreadType,
    },
    ConcentricRings {
        common: CommonPlacement,
        distance: i32,
        spread: i32,
        count: i32,
        preferred_biomes: BTreeSet<String>,
        preferred_biome_indices: BTreeSet<i32>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommonPlacement {
    pub locate_offset: [i32; 3],
    pub frequency: f32,
    pub salt: i32,
    pub exclusion_zone: Option<ExclusionZone>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExclusionZone {
    pub other_set: String,
    pub chunk_count: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomSpreadType {
    Linear,
    Triangular,
}

impl NoiseStructurePlan {
    /// `ChunkGeneratorStructureState.createForNormal` and
    /// `hasBiomesForStructureSet` (`ChunkGeneratorStructureState.java:52-69`).
    pub fn from_startup(
        startup: &DatapackStartup,
        possible_biome_ids: &[&str],
    ) -> Result<Self, NoiseStructuresError> {
        let biome_registry = registry(startup, BIOME_REGISTRY)?;
        let structure_registry = registry(startup, STRUCTURE_REGISTRY)?;
        let set_registry = registry(startup, STRUCTURE_SET_REGISTRY)?;
        let mut pools = startup
            .structure_pools_shared()
            .map_err(|error| refuse(error.path, error.reason))?;
        let mob_spawns = startup
            .mob_spawn_catalog_shared()
            .map_err(|error| refuse(error.path, error.reason))?;
        let known_biomes = biome_registry
            .elements
            .iter()
            .map(|element| element.id.clone())
            .collect::<BTreeSet<_>>();
        let known_structures = structure_registry
            .elements
            .iter()
            .map(|element| element.id.clone())
            .collect::<BTreeSet<_>>();
        let known_sets = set_registry
            .elements
            .iter()
            .map(|element| element.id.clone())
            .collect::<BTreeSet<_>>();

        let mut possible_biomes = BTreeSet::new();
        for (index, raw_id) in possible_biome_ids.iter().enumerate() {
            let path = format!("{BIOME_REGISTRY}.possible_biomes[{index}]");
            let id = normalize_id(raw_id, &path)?;
            if !known_biomes.contains(&id) {
                return Err(refuse(path, format!("unknown biome {id}")));
            }
            possible_biomes.insert(id);
        }

        // `Structure.StructureSettings.CODEC` (`Structure.java:268-286`).
        let mut definitions = BTreeMap::new();
        for element in &structure_registry.elements {
            let path = format!("{STRUCTURE_REGISTRY}.{}", element.id);
            let decoded_owned;
            let decoded = match startup
                .registries
                .worldgen
                .cached_structure(&element.id, &element.source_json)
            {
                Some(Ok(decoded)) => decoded.as_ref(),
                Some(Err(err)) => return Err(from_decode_error(&path, err)),
                None => {
                    let parsed = element
                        .decoded_loaded_source(usize::MAX)
                        .map_err(|error| refuse(&path, error.to_string()))?;
                    let json = parsed
                        .json()
                        .ok_or_else(|| refuse(&path, "parsed source JSON is absent"))?;
                    decoded_owned = DecodedStructure::decode(json, &path)
                        .map_err(|err| from_decode_error(&path, &err))?;
                    &decoded_owned
                }
            };
            let spawn_settings = mob_spawns
                .structure(&element.id)
                .ok_or_else(|| refuse(&path, "structure is absent from the mob spawn catalog"))?;
            let definition = build_structure_definition(
                startup,
                &known_biomes,
                &mut pools,
                &element.id,
                decoded,
                spawn_settings.overrides.clone(),
                &path,
            )?;
            definitions.insert(element.id.clone(), definition);
        }
        let mut structure_order = structure_registry
            .elements
            .iter()
            .map(|element| element.id.clone())
            .collect::<Vec<_>>();

        let mut sets = Vec::new();
        let mut registered_sets = BTreeMap::new();
        for element in &set_registry.elements {
            let path = format!("{STRUCTURE_SET_REGISTRY}.{}", element.id);
            let decoded_owned;
            let decoded = match startup
                .registries
                .worldgen
                .cached_structure_set(&element.id, &element.source_json)
            {
                Some(Ok(decoded)) => decoded.as_ref(),
                Some(Err(err)) => return Err(from_decode_error(&path, err)),
                None => {
                    let parsed = element
                        .decoded_loaded_source(usize::MAX)
                        .map_err(|error| refuse(&path, error.to_string()))?;
                    let json = parsed
                        .json()
                        .ok_or_else(|| refuse(&path, "parsed source JSON is absent"))?;
                    decoded_owned = DecodedStructureSet::decode(json, &path)
                        .map_err(|err| from_decode_error(&path, &err))?;
                    &decoded_owned
                }
            };
            let mut structures = Vec::with_capacity(decoded.structures.len());
            for (index, selection) in decoded.structures.iter().enumerate() {
                let item_path = format!("{path}.structures[{index}].structure");
                let structure_id = match &selection.structure {
                    DecodedStructureHolder::Named(id) => {
                        if !known_structures.contains(id) {
                            return Err(refuse(&item_path, format!("unknown structure {id}")));
                        }
                        id.clone()
                    }
                    DecodedStructureHolder::Inline(inline_decoded) => {
                        let inline_id = format!("inline:{item_path}");
                        let overrides = inline_decoded
                            .spawn_overrides
                            .as_ref()
                            .map_err(|error| refuse(&error.path, &error.reason))?
                            .resolve(&item_path, &startup.content_registry)
                            .map_err(|error| refuse(error.path, error.reason))?;
                        let definition = build_structure_definition(
                            startup,
                            &known_biomes,
                            &mut pools,
                            &inline_id,
                            inline_decoded,
                            overrides,
                            &item_path,
                        )?;
                        definitions.insert(inline_id.clone(), definition);
                        structure_order.push(inline_id.clone());
                        inline_id
                    }
                };
                structures.push(NoiseStructureSelection {
                    structure: structure_id,
                    weight: selection.weight,
                });
            }
            let placement = build_placement(
                startup,
                &known_biomes,
                &known_sets,
                &decoded.placement,
                &format!("{path}.placement"),
            )?;
            let set = NoiseStructureSet {
                id: element.id.clone(),
                structures,
                placement,
            };
            registered_sets.insert(element.id.clone(), set.clone());
            let intersects = set.structures.iter().any(|selection| {
                definitions
                    .get(&selection.structure)
                    .is_some_and(|definition| !definition.biomes.is_disjoint(&possible_biomes))
            });
            if !intersects {
                continue;
            }
            sets.push(set);
        }
        let set_index = sets
            .iter()
            .enumerate()
            .map(|(index, set)| (set.id.clone(), index))
            .collect();
        Ok(Self {
            possible_biomes: Arc::new(possible_biomes),
            pools,
            structures: Arc::new(definitions),
            structure_order: Arc::from(structure_order),
            sets,
            set_index,
            registered_sets: Arc::new(registered_sets),
            registered_selected_attempt: None,
        })
    }

    pub fn restrict_sets(&mut self, ordered_ids: &[String]) {
        self.sets = ordered_ids
            .iter()
            .filter_map(|id| self.set(id).cloned())
            .collect();
        self.set_index = self
            .sets
            .iter()
            .enumerate()
            .map(|(index, set)| (set.id.clone(), index))
            .collect();
    }

    pub fn sets(&self) -> &[NoiseStructureSet] {
        &self.sets
    }

    pub fn structures(&self) -> &BTreeMap<String, NoiseStructureDefinition> {
        &self.structures
    }

    pub fn registered_structure_owner(&self, id: &str) -> Option<StructureStartSetOwner> {
        self.registered_sets.values().find_map(|set| {
            let selection = set.structures.iter().find(|entry| entry.structure == id)?;
            self.single_selection_owner(set, selection)
                .filter(|owner| *owner != StructureStartSetOwner::Unowned)
        })
    }

    pub fn structure_order(&self) -> &[String] {
        &self.structure_order
    }

    pub fn pools(&self) -> &StructurePoolSet {
        &self.pools
    }

    pub fn set(&self, id: &str) -> Option<&NoiseStructureSet> {
        self.set_index.get(id).map(|index| &self.sets[*index])
    }

    pub fn possible_biomes(&self) -> &BTreeSet<String> {
        &self.possible_biomes
    }

    pub fn structure_start_owner(&self, set: &NoiseStructureSet) -> StructureStartSetOwner {
        if end::owns_set(self, set) {
            return StructureStartSetOwner::EndCity;
        }
        if let Some(owner) = nether::structure_start_owner(self, set) {
            return owner;
        }
        if !set.structures.is_empty()
            && set.structures.iter().all(|selection| {
                self.structures
                    .get(&selection.structure)
                    .is_some_and(|definition| {
                        definition.type_id == BURIED_TREASURE_ID && definition.jigsaw.is_none()
                    })
            })
        {
            return StructureStartSetOwner::BuriedTreasure;
        }
        if !set.structures.is_empty()
            && set.structures.iter().all(|selection| {
                self.structures
                    .get(&selection.structure)
                    .is_some_and(|definition| definition.jigsaw.is_some())
            })
        {
            return StructureStartSetOwner::Jigsaw;
        }
        if !set.structures.is_empty()
            && set.structures.iter().all(|selection| {
                self.structures
                    .get(&selection.structure)
                    .is_some_and(|definition| definition.mineshaft.is_some())
            })
        {
            return StructureStartSetOwner::Mineshaft;
        }
        if !set.structures.is_empty()
            && set.structures.iter().all(|selection| {
                self.structures
                    .get(&selection.structure)
                    .is_some_and(|definition| definition.shipwreck.is_some())
            })
        {
            return StructureStartSetOwner::Shipwreck;
        }
        if !set.structures.is_empty()
            && set.structures.iter().all(|selection| {
                self.structures
                    .get(&selection.structure)
                    .is_some_and(|definition| definition.ocean_ruin.is_some())
            })
        {
            return StructureStartSetOwner::OceanRuin;
        }
        if !set.structures.is_empty()
            && set.structures.iter().all(|selection| {
                self.structures
                    .get(&selection.structure)
                    .is_some_and(|definition| definition.type_id == IGLOO_TYPE_ID)
            })
        {
            return StructureStartSetOwner::Igloo;
        }
        // Structure-set entries carry Structure holders, so the registered id is
        // independent of its codec owner (`StructureSet.java:15-41`).
        if !set.structures.is_empty() {
            if let Some(kind) = set.structures.first().and_then(|selection| {
                self.structures
                    .get(&selection.structure)
                    .and_then(|definition| ScatteredFeatureKind::from_type_id(&definition.type_id))
            }) {
                if set.structures.iter().all(|selection| {
                    self.structures
                        .get(&selection.structure)
                        .is_some_and(|definition| {
                            definition.type_id == kind.type_id()
                                && kind.owns_structure_id(&selection.structure)
                        })
                }) {
                    return match kind {
                        ScatteredFeatureKind::DesertPyramid => {
                            StructureStartSetOwner::DesertPyramid
                        }
                        ScatteredFeatureKind::JungleTemple => StructureStartSetOwner::JungleTemple,
                        ScatteredFeatureKind::SwampHut => StructureStartSetOwner::SwampHut,
                    };
                }
            }
        }
        if !set.structures.is_empty()
            && set.structures.iter().all(|selection| {
                self.structures
                    .get(&selection.structure)
                    .is_some_and(|definition| definition.ruined_portal.is_some())
            })
        {
            return StructureStartSetOwner::RuinedPortal;
        }
        if !set.structures.is_empty()
            && set.structures.iter().all(|selection| {
                self.structures
                    .get(&selection.structure)
                    .is_some_and(|definition| definition.type_id == OCEAN_MONUMENT_TYPE_ID)
            })
        {
            return StructureStartSetOwner::OceanMonument;
        }
        if !set.structures.is_empty()
            && set.structures.iter().all(|selection| {
                self.structures
                    .get(&selection.structure)
                    .is_some_and(|definition| definition.type_id == WOODLAND_MANSION_TYPE_ID)
            })
        {
            return StructureStartSetOwner::WoodlandMansion;
        }
        if stronghold::owns_set(
            set,
            set.structures
                .iter()
                .map(|selection| self.structures.get(&selection.structure)),
        ) {
            return StructureStartSetOwner::Stronghold;
        }
        registered_random_spread::owner(self, set)
    }

    /// Refuses a random-spread candidate that the current typed start owner
    /// cannot evaluate. Concentric-ring membership is supplied by the program's
    /// frozen biome-search result (`ChunkGenerator.java:476-589`).
    pub fn unowned_random_spread_candidate(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Option<UnownedStructureStartCandidate> {
        self.sets.iter().find_map(|set| {
            if self.structure_start_owner(set) != StructureStartSetOwner::Unowned {
                return None;
            }
            let attempts =
                self.random_spread_attempt_order(&set.id, level_seed, chunk_x, chunk_z)?;
            (!attempts.is_empty()).then(|| UnownedStructureStartCandidate {
                set_id: set.id.clone(),
                attempts,
            })
        })
    }

    pub fn ring_positions(
        &self,
        set_id: &str,
        level_seed: i64,
        biome_source: &NoiseBiomeSource,
        density: &NoiseDensitySource,
    ) -> Option<Vec<(i32, i32)>> {
        self.set(set_id)?
            .placement
            .concentric_ring_positions(level_seed, biome_source, density)
    }

    /// Weighted retry for a StructureSet after its placement selected the source chunk
    /// (`ChunkGenerator.java:503-545`).
    pub fn weighted_attempt_order(
        &self,
        set_id: &str,
        level_seed: i64,
        source_x: i32,
        source_z: i32,
    ) -> Option<Vec<String>> {
        let set = self.set(set_id)?;
        if set.structures.len() == 1 {
            return Some(vec![set.structures[0].structure.clone()]);
        }
        let mut options = set.structures.clone();
        let mut total = options
            .iter()
            .map(|selection| selection.weight)
            .sum::<i32>();
        let mut random = large_feature_random(level_seed, source_x, source_z);
        let mut attempts = Vec::with_capacity(options.len());
        while !options.is_empty() {
            let mut choice = random.next_int(total);
            let mut selected_index = 0;
            for (index, option) in options.iter().enumerate() {
                choice -= option.weight;
                if choice < 0 {
                    selected_index = index;
                    break;
                }
            }
            let selected = options.remove(selected_index);
            total -= selected.weight;
            attempts.push(selected.structure);
        }
        Some(attempts)
    }

    pub fn is_random_spread_structure_chunk(
        &self,
        set_id: &str,
        seed: i64,
        x: i32,
        z: i32,
    ) -> Option<bool> {
        if !matches!(
            &self.registered_sets.get(set_id)?.placement,
            NoiseStructurePlacement::RandomSpread { .. }
        ) {
            return None;
        }
        self.is_structure_chunk(set_id, seed, x, z, &BTreeMap::new())
    }

    // StructurePlacement.java:82-94; ChunkGeneratorStructureState.java:191-203.
    pub fn is_structure_chunk(
        &self,
        set_id: &str,
        seed: i64,
        x: i32,
        z: i32,
        rings: &BTreeMap<String, BTreeSet<[i32; 2]>>,
    ) -> Option<bool> {
        self.is_structure_chunk_inner(set_id, seed, x, z, rings, &mut BTreeSet::new())
    }

    fn is_structure_chunk_inner(
        &self,
        set_id: &str,
        seed: i64,
        x: i32,
        z: i32,
        rings: &BTreeMap<String, BTreeSet<[i32; 2]>>,
        visiting: &mut BTreeSet<String>,
    ) -> Option<bool> {
        if !visiting.insert(set_id.to_owned()) {
            return None;
        }
        let result = (|| {
            let placement = &self.registered_sets.get(set_id)?.placement;
            let (candidate, common) = match placement {
                NoiseStructurePlacement::RandomSpread { common, .. } => (
                    placement.random_spread_candidate(seed, x, z) == Some((x, z)),
                    common,
                ),
                NoiseStructurePlacement::ConcentricRings { common, .. } => (
                    rings
                        .get(set_id)
                        .is_some_and(|positions| positions.contains(&[x, z])),
                    common,
                ),
            };
            if !candidate || !common.passes_frequency(seed, x, z) {
                return Some(false);
            }
            if let Some(zone) = &common.exclusion_zone {
                let min_x = x.checked_sub(zone.chunk_count)?;
                let max_x = x.checked_add(zone.chunk_count)?;
                let min_z = z.checked_sub(zone.chunk_count)?;
                let max_z = z.checked_add(zone.chunk_count)?;
                for other_x in min_x..=max_x {
                    for other_z in min_z..=max_z {
                        if self.is_structure_chunk_inner(
                            &zone.other_set,
                            seed,
                            other_x,
                            other_z,
                            rings,
                            visiting,
                        )? {
                            return Some(false);
                        }
                    }
                }
            }
            Some(true)
        })();
        visiting.remove(set_id);
        result
    }
}

/// Keeps the frequency draw independent of the spread draw keyed by the
/// same set and coordinates.
const FREQUENCY_STREAM: i64 = 1;

impl NoiseStructurePlacement {
    /// `RandomSpreadStructurePlacement.getPotentialStructureChunk`
    /// (`RandomSpreadStructurePlacement.java:67-76`).
    pub fn random_spread_candidate(
        &self,
        level_seed: i64,
        source_x: i32,
        source_z: i32,
    ) -> Option<(i32, i32)> {
        let Self::RandomSpread {
            common,
            spacing,
            separation,
            spread_type,
        } = self
        else {
            return None;
        };
        let grid_x = source_x.div_euclid(*spacing);
        let grid_z = source_z.div_euclid(*spacing);
        let mut random = WorldgenRandom::from_parts(&[
            level_seed,
            i64::from(common.salt),
            i64::from(grid_x),
            i64::from(grid_z),
        ]);
        let limit = spacing - separation;
        let mut spread = || match spread_type {
            RandomSpreadType::Linear => random.next_int(limit),
            RandomSpreadType::Triangular => (random.next_int(limit) + random.next_int(limit)) / 2,
        };
        Some((
            grid_x.wrapping_mul(*spacing).wrapping_add(spread()),
            grid_z.wrapping_mul(*spacing).wrapping_add(spread()),
        ))
    }

    /// Ring scheduling from `ChunkGeneratorStructureState.generateRingPositions`
    /// (`ChunkGeneratorStructureState.java:107-160`). The callback owns the
    /// horizontal biome search and returns its selected block x/z.
    pub fn concentric_ring_positions_with(
        &self,
        level_seed: i64,
        mut relocate: impl FnMut(i32, i32, i32, &BTreeSet<i32>, i64) -> Option<(i32, i32)>,
    ) -> Option<Vec<(i32, i32)>> {
        let Self::ConcentricRings {
            distance,
            spread,
            count,
            preferred_biome_indices,
            ..
        } = self
        else {
            return None;
        };
        let mut random = WorldgenRandom::named(level_seed, "concentric_rings");
        let mut angle = random.next_double() * std::f64::consts::TAU;
        let mut position_in_circle = 0;
        let mut circle = 0;
        let mut circle_spread = *spread;
        let mut positions = Vec::with_capacity(*count as usize);
        for index in 0..*count {
            let base_radius = distance
                .wrapping_mul(4)
                .wrapping_add(distance.wrapping_mul(circle).wrapping_mul(6));
            let radius =
                f64::from(base_radius) + (random.next_double() - 0.5) * f64::from(*distance) * 2.5;
            let initial_x = java_round(angle.cos() * radius) as i32;
            let initial_z = java_round(angle.sin() * radius) as i32;
            let search_seed = random.next_long();
            let selected = relocate(
                initial_x.wrapping_mul(16).wrapping_add(8),
                initial_z.wrapping_mul(16).wrapping_add(8),
                112,
                preferred_biome_indices,
                search_seed,
            )
            .map(|(block_x, block_z)| (block_x.div_euclid(16), block_z.div_euclid(16)))
            .unwrap_or((initial_x, initial_z));
            positions.push(selected);

            angle += std::f64::consts::TAU / f64::from(circle_spread);
            position_in_circle += 1;
            if position_in_circle == circle_spread {
                circle += 1;
                position_in_circle = 0;
                circle_spread += 2 * circle_spread / (circle + 1);
                circle_spread = circle_spread.min(*count - index);
                angle += random.next_double() * std::f64::consts::TAU;
            }
        }
        Some(positions)
    }

    pub fn concentric_ring_positions(
        &self,
        level_seed: i64,
        biome_source: &NoiseBiomeSource,
        density: &NoiseDensitySource,
    ) -> Option<Vec<(i32, i32)>> {
        self.concentric_ring_positions_with(
            level_seed,
            |origin_x, origin_z, radius, allowed, search_seed| {
                let mut random = WorldgenRandom::with_seed(search_seed);
                biome_source
                    .find_biome_horizontal(
                        density,
                        origin_x,
                        0,
                        origin_z,
                        radius,
                        allowed,
                        &mut random,
                    )
                    .map(|(block_x, _, block_z)| (block_x, block_z))
            },
        )
    }
}

fn java_round(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

impl CommonPlacement {
    /// `StructurePlacement.isStructureChunk` frequency gate: one keyed draw
    /// per candidate chunk, whatever the configured reduction method.
    fn passes_frequency(&self, level_seed: i64, source_x: i32, source_z: i32) -> bool {
        self.frequency >= 1.0
            || WorldgenRandom::from_parts(&[
                level_seed,
                i64::from(self.salt),
                i64::from(source_x),
                i64::from(source_z),
                FREQUENCY_STREAM,
            ])
            .next_float()
                < self.frequency
    }
}

fn registry<'a>(
    startup: &'a DatapackStartup,
    id: &str,
) -> Result<&'a bbb_server_datapack::dynamic_registry::LoadedDynamicRegistry, NoiseStructuresError>
{
    startup
        .registries
        .registry(id)
        .ok_or_else(|| refuse(id, "registry is absent"))
}

fn from_decode_error(
    base_path: &str,
    error: &bbb_server_datapack::worldgen::DecodeError,
) -> NoiseStructuresError {
    let path = if error.path.is_empty() {
        base_path.to_string()
    } else if error.path.starts_with('[') {
        format!("{base_path}{}", error.path)
    } else {
        format!("{base_path}.{}", error.path)
    };
    refuse(path, &error.message)
}

fn build_structure_definition(
    startup: &DatapackStartup,
    known_biomes: &BTreeSet<String>,
    pools: &mut Arc<StructurePoolSet>,
    id: &str,
    decoded: &DecodedStructure,
    spawn_overrides: Vec<StructureMobSpawnOverride>,
    path: &str,
) -> Result<NoiseStructureDefinition, NoiseStructuresError> {
    let biomes = resolve_biome_holder_set(
        startup,
        known_biomes,
        &decoded.biomes,
        &format!("{path}.biomes"),
    )?;
    let step = match decoded.step.as_str() {
        "raw_generation" => StructureDecorationStep::RawGeneration,
        "lakes" => StructureDecorationStep::Lakes,
        "local_modifications" => StructureDecorationStep::LocalModifications,
        "underground_structures" => StructureDecorationStep::UndergroundStructures,
        "surface_structures" => StructureDecorationStep::SurfaceStructures,
        "strongholds" => StructureDecorationStep::Strongholds,
        "underground_ores" => StructureDecorationStep::UndergroundOres,
        "underground_decoration" => StructureDecorationStep::UndergroundDecoration,
        "fluid_springs" => StructureDecorationStep::FluidSprings,
        "vegetal_decoration" => StructureDecorationStep::VegetalDecoration,
        "top_layer_modification" => StructureDecorationStep::TopLayerModification,
        value => {
            return Err(refuse(
                format!("{path}.step"),
                format!("unknown decoration step {value}"),
            ))
        }
    };
    let terrain_adjustment = match decoded.terrain_adaptation.as_str() {
        "none" => StructureTerrainAdjustment::None,
        "bury" => StructureTerrainAdjustment::Bury,
        "beard_thin" => StructureTerrainAdjustment::BeardThin,
        "beard_box" => StructureTerrainAdjustment::BeardBox,
        "encapsulate" => StructureTerrainAdjustment::Encapsulate,
        value => {
            return Err(refuse(
                format!("{path}.terrain_adaptation"),
                format!("unknown terrain adaptation {value}"),
            ))
        }
    };
    let jigsaw = decoded
        .jigsaw
        .as_ref()
        .map(|settings| build_jigsaw_settings(startup, pools, settings, path))
        .transpose()?;
    let mineshaft = decoded
        .mineshaft_is_mesa
        .map(|is_mesa| MineshaftStructureSettings {
            kind: if is_mesa {
                MineshaftType::Mesa
            } else {
                MineshaftType::Normal
            },
        });
    let shipwreck = decoded
        .shipwreck_is_beached
        .map(|is_beached| ShipwreckStructureSettings { is_beached });
    let ocean_ruin = decoded.ocean_ruin.map(|ruin| OceanRuinStructureSettings {
        biome_temp: if ruin.warm {
            OceanRuinBiomeTemp::Warm
        } else {
            OceanRuinBiomeTemp::Cold
        },
        large_probability: OceanRuinFloat::from_f32(ruin.large_probability),
        cluster_probability: OceanRuinFloat::from_f32(ruin.cluster_probability),
    });
    let ruined_portal = decoded
        .ruined_portal
        .as_deref()
        .map(|setups| {
            ruined_portal_settings_from_decoded(setups)
                .ok_or_else(|| refuse(format!("{path}.setups"), "invalid ruined portal setups"))
        })
        .transpose()?;
    Ok(NoiseStructureDefinition {
        id: id.to_string(),
        type_id: decoded.type_id.clone(),
        biomes,
        step,
        terrain_adjustment,
        spawn_overrides,
        jigsaw,
        mineshaft,
        shipwreck,
        ocean_ruin,
        ruined_portal,
        nether_fossil_height: decoded.nether_fossil_height.clone(),
    })
}

fn build_jigsaw_settings(
    startup: &DatapackStartup,
    pools: &mut Arc<StructurePoolSet>,
    decoded: &DecodedJigsawSettings,
    path: &str,
) -> Result<JigsawStructureSettings, NoiseStructuresError> {
    let start_pool_path = format!("{path}.start_pool");
    let start_pool = match &decoded.start_pool {
        DecodedTemplatePoolHolder::Named(id) => {
            if !pools.contains(id) {
                return Err(refuse(
                    &start_pool_path,
                    format!("unknown template pool {id}"),
                ));
            }
            id.clone()
        }
        DecodedTemplatePoolHolder::Inline(json) => {
            let inline_id = format!("inline:{start_pool_path}");
            Arc::make_mut(pools)
                .register_inline_pool(inline_id, json, &startup.registries, &start_pool_path)
                .map_err(|error| refuse(error.path, error.reason))?
        }
    };
    let project_start_to_heightmap = decoded
        .project_start_to_heightmap
        .as_deref()
        .map(|name| match name {
            "WORLD_SURFACE_WG" => Ok(StructureHeightmap::WorldSurfaceWg),
            "WORLD_SURFACE" => Ok(StructureHeightmap::WorldSurface),
            "OCEAN_FLOOR_WG" => Ok(StructureHeightmap::OceanFloorWg),
            "OCEAN_FLOOR" => Ok(StructureHeightmap::OceanFloor),
            "MOTION_BLOCKING" => Ok(StructureHeightmap::MotionBlocking),
            "MOTION_BLOCKING_NO_LEAVES" => Ok(StructureHeightmap::MotionBlockingNoLeaves),
            other => Err(refuse(
                format!("{path}.project_start_to_heightmap"),
                format!("unknown heightmap {other}"),
            )),
        })
        .transpose()?;
    Ok(JigsawStructureSettings {
        start_pool,
        start_jigsaw_name: decoded.start_jigsaw_name.clone(),
        size: decoded.size,
        start_height: decoded.start_height.clone(),
        use_expansion_hack: decoded.use_expansion_hack,
        project_start_to_heightmap,
        max_distance_from_center: JigsawMaxDistance {
            horizontal: decoded.max_distance_horizontal,
            vertical: decoded.max_distance_vertical,
        },
        pool_aliases: decoded
            .pool_aliases
            .iter()
            .map(convert_pool_alias)
            .collect(),
        dimension_padding: DimensionPadding {
            bottom: decoded.dimension_padding_bottom,
            top: decoded.dimension_padding_top,
        },
        liquid_settings: decoded.liquid_settings,
    })
}

fn convert_pool_alias(alias: &DecodedPoolAliasBinding) -> PoolAliasBinding {
    match alias {
        DecodedPoolAliasBinding::Direct { alias, target } => PoolAliasBinding::Direct {
            alias: alias.clone(),
            target: target.clone(),
        },
        DecodedPoolAliasBinding::Random { alias, targets } => PoolAliasBinding::Random {
            alias: alias.clone(),
            targets: targets
                .iter()
                .map(|target| WeightedPool {
                    pool: target.pool.clone(),
                    weight: target.weight,
                })
                .collect(),
        },
        DecodedPoolAliasBinding::RandomGroup { groups } => PoolAliasBinding::RandomGroup {
            groups: groups
                .iter()
                .map(|group| WeightedAliasGroup {
                    bindings: group.bindings.iter().map(convert_pool_alias).collect(),
                    weight: group.weight,
                })
                .collect(),
        },
    }
}

fn build_placement(
    startup: &DatapackStartup,
    known_biomes: &BTreeSet<String>,
    known_sets: &BTreeSet<String>,
    decoded: &DecodedStructurePlacement,
    path: &str,
) -> Result<NoiseStructurePlacement, NoiseStructuresError> {
    match decoded {
        DecodedStructurePlacement::RandomSpread {
            common,
            spacing,
            separation,
            triangular,
        } => Ok(NoiseStructurePlacement::RandomSpread {
            common: build_common_placement(common, known_sets, path)?,
            spacing: *spacing,
            separation: *separation,
            spread_type: if *triangular {
                RandomSpreadType::Triangular
            } else {
                RandomSpreadType::Linear
            },
        }),
        DecodedStructurePlacement::ConcentricRings {
            common,
            distance,
            spread,
            count,
            preferred_biomes,
        } => {
            let biomes_path = format!("{path}.preferred_biomes");
            let preferred_biomes =
                resolve_biome_holder_set(startup, known_biomes, preferred_biomes, &biomes_path)?;
            let preferred_biome_indices = biome_indices(startup, &preferred_biomes, &biomes_path)?;
            Ok(NoiseStructurePlacement::ConcentricRings {
                common: build_common_placement(common, known_sets, path)?,
                distance: *distance,
                spread: *spread,
                count: *count,
                preferred_biomes,
                preferred_biome_indices,
            })
        }
    }
}

fn build_common_placement(
    decoded: &DecodedCommonPlacement,
    known_sets: &BTreeSet<String>,
    path: &str,
) -> Result<CommonPlacement, NoiseStructuresError> {
    let exclusion_zone = decoded
        .exclusion_zone
        .as_ref()
        .map(|(other_set, chunk_count)| {
            if !known_sets.contains(other_set) {
                return Err(refuse(
                    format!("{path}.exclusion_zone.other_set"),
                    format!("unknown structure set {other_set}"),
                ));
            }
            Ok(ExclusionZone {
                other_set: other_set.clone(),
                chunk_count: *chunk_count,
            })
        })
        .transpose()?;
    Ok(CommonPlacement {
        locate_offset: decoded.locate_offset,
        frequency: decoded.frequency,
        salt: decoded.salt,
        exclusion_zone,
    })
}

fn resolve_biome_holder_set(
    startup: &DatapackStartup,
    known_biomes: &BTreeSet<String>,
    set: &BiomeHolderSet,
    path: &str,
) -> Result<BTreeSet<String>, NoiseStructuresError> {
    let members: &[String] = match set {
        BiomeHolderSet::Tag(tag) => startup
            .tags
            .registry(BIOME_REGISTRY)
            .and_then(|tags| tags.tag(tag))
            .ok_or_else(|| refuse(path, format!("unknown biome tag #{tag}")))?,
        BiomeHolderSet::Direct(members) => members,
    };
    let mut result = BTreeSet::new();
    for member in members {
        if !known_biomes.contains(member) {
            return Err(refuse(path, format!("unknown biome {member}")));
        }
        result.insert(member.clone());
    }
    Ok(result)
}

fn biome_indices(
    startup: &DatapackStartup,
    biomes: &BTreeSet<String>,
    path: &str,
) -> Result<BTreeSet<i32>, NoiseStructuresError> {
    let registry = registry(startup, BIOME_REGISTRY)?;
    biomes
        .iter()
        .map(|biome| {
            registry
                .elements
                .iter()
                .position(|element| element.id == *biome)
                .and_then(|index| i32::try_from(index).ok())
                .ok_or_else(|| refuse(path, format!("biome {biome} has no runtime index")))
        })
        .collect()
}

fn normalize_id(raw: &str, path: &str) -> Result<String, NoiseStructuresError> {
    bbb_protocol::normalize_resource_location(raw.to_string())
        .map_err(|error| refuse(path, error.to_string()))
}

fn refuse(path: impl Into<String>, reason: impl Into<String>) -> NoiseStructuresError {
    NoiseStructuresError {
        path: path.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests;
