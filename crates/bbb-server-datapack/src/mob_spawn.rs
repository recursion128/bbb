use std::collections::BTreeMap;
use std::fmt;

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::entity_types::{
    vanilla_entity_mob_category_for_serialized_name, vanilla_entity_type_for_resource_id,
    VanillaEntityMobCategory, VANILLA_ENTITY_TYPE_PIG_ID,
};
use bbb_registry_codec::{RegistryElementValueRef as CodecValue, RegistryJsonValue};

use crate::dynamic_registry::{DatapackGeneration, RawRegistryElement};

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
const STRUCTURE_REGISTRY: &str = "minecraft:worldgen/structure";
const DEFAULT_CREATURE_SPAWN_PROBABILITY: f32 = 0.1;
const MAX_CREATURE_SPAWN_PROBABILITY: f32 = 0.999_999_9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobSpawnCatalogError {
    pub path: String,
    pub reason: String,
}

impl fmt::Display for MobSpawnCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.reason)
    }
}

impl std::error::Error for MobSpawnCatalogError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobSpawnerData {
    pub entity_type_id: i32,
    pub entity_type: String,
    pub min_count: i32,
    pub max_count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightedMobSpawnerData {
    pub data: MobSpawnerData,
    pub weight: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MobSpawnCost {
    pub entity_type_id: i32,
    pub entity_type: String,
    pub energy_budget: f64,
    pub charge: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MobSpawnList {
    pub entries: Vec<WeightedMobSpawnerData>,
    pub total_weight: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BiomeMobSpawnSettings {
    pub biome_id: String,
    pub creature_spawn_probability: f32,
    pub categories: Vec<(VanillaEntityMobCategory, MobSpawnList)>,
    pub costs: Vec<MobSpawnCost>,
}

impl BiomeMobSpawnSettings {
    pub fn spawns(&self, category: VanillaEntityMobCategory) -> &MobSpawnList {
        self.categories
            .iter()
            .find_map(|(candidate, spawns)| (*candidate == category).then_some(spawns))
            .unwrap_or(&EMPTY_MOB_SPAWN_LIST)
    }

    pub fn spawn_cost(&self, entity_type: &str) -> Option<&MobSpawnCost> {
        let entity_type = normalize_id(entity_type, "entity type query").ok()?;
        self.costs
            .iter()
            .find(|cost| cost.entity_type == entity_type)
    }

    pub fn spawn_cost_by_id(&self, entity_type_id: i32) -> Option<&MobSpawnCost> {
        self.costs
            .iter()
            .find(|cost| cost.entity_type_id == entity_type_id)
    }
}

static EMPTY_MOB_SPAWN_LIST: MobSpawnList = MobSpawnList {
    entries: Vec::new(),
    total_weight: 0,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureSpawnBoundingBox {
    Piece,
    Full,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureMobSpawnOverride {
    pub category: VanillaEntityMobCategory,
    pub bounding_box: StructureSpawnBoundingBox,
    pub spawns: MobSpawnList,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureMobSpawnSettings {
    pub structure_id: String,
    pub overrides: Vec<StructureMobSpawnOverride>,
}

impl StructureMobSpawnSettings {
    pub fn override_for(
        &self,
        category: VanillaEntityMobCategory,
    ) -> Option<&StructureMobSpawnOverride> {
        self.overrides
            .iter()
            .find(|spawn_override| spawn_override.category == category)
    }
}

/// Content-registry-independent pre-decoded biome mob spawn settings so biome
/// JSON is parsed once at registry-load time and resolved against any
/// `ContentRegistry` without re-parsing.
#[derive(Clone, Debug, PartialEq)]
pub struct RawBiomeMobSpawnSettings {
    pub biome_id: String,
    pub creature_spawn_probability: f32,
    pub categories: Vec<(VanillaEntityMobCategory, RawMobSpawnList)>,
    pub costs: Vec<RawMobSpawnCost>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RawMobSpawnList {
    pub entries: Vec<RawWeightedMobSpawnerData>,
    pub total_weight: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawWeightedMobSpawnerData {
    pub entity_type: String,
    pub min_count: i32,
    pub max_count: i32,
    pub weight: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawMobSpawnCost {
    pub entity_type: String,
    pub energy_budget: f64,
    pub charge: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawStructureMobSpawnOverride {
    pub category: VanillaEntityMobCategory,
    pub bounding_box: StructureSpawnBoundingBox,
    pub spawns: RawMobSpawnList,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawStructureSpawnOverrides {
    pub overrides: Vec<RawStructureMobSpawnOverride>,
}

impl RawBiomeMobSpawnSettings {
    pub fn parse_from_root(id: &str, root: CodecValue<'_>) -> Result<Self, MobSpawnCatalogError> {
        let path = format!("{BIOME_REGISTRY}.{id}");
        let fields = required_map(root, &path)?;
        let creature_spawn_probability = match fields.field("creature_spawn_probability") {
            Some(value) => ranged_f32(
                value,
                0.0,
                MAX_CREATURE_SPAWN_PROBABILITY,
                &format!("{path}.creature_spawn_probability"),
            )?,
            None => DEFAULT_CREATURE_SPAWN_PROBABILITY,
        };
        let spawners_path = format!("{path}.spawners");
        let spawners = required_map(
            fields
                .field("spawners")
                .ok_or_else(|| refuse(&spawners_path, "object is absent"))?,
            &spawners_path,
        )?;
        let categories = map_entries(spawners)
            .enumerate()
            .map(|(index, entry)| {
                let (raw_category, value) = entry.ok_or_else(|| {
                    refuse(format!("{spawners_path}[{index}]"), "map entry is absent")
                })?;
                let category_path = format!("{path}.spawners.{raw_category}");
                let category = parse_category(&raw_category, &category_path)?;
                let spawns = parse_raw_spawn_list(value, &category_path)?;
                Ok((category, spawns))
            })
            .collect::<Result<Vec<_>, MobSpawnCatalogError>>()?;
        let costs_path = format!("{path}.spawn_costs");
        let costs_map = required_map(
            fields
                .field("spawn_costs")
                .ok_or_else(|| refuse(&costs_path, "object is absent"))?,
            &costs_path,
        )?;
        let costs = map_entries(costs_map)
            .enumerate()
            .map(|(index, entry)| {
                let (raw_entity_type, value) = entry.ok_or_else(|| {
                    refuse(format!("{costs_path}[{index}]"), "map entry is absent")
                })?;
                let entry_path = format!("{path}.spawn_costs.{raw_entity_type}");
                let fields = required_map(value, &entry_path)?;
                Ok(RawMobSpawnCost {
                    entity_type: raw_entity_type,
                    energy_budget: required_f64(
                        fields.field("energy_budget"),
                        &format!("{entry_path}.energy_budget"),
                    )?,
                    charge: required_f64(fields.field("charge"), &format!("{entry_path}.charge"))?,
                })
            })
            .collect::<Result<Vec<_>, MobSpawnCatalogError>>()?;
        Ok(Self {
            biome_id: normalize_id(id, &path)?,
            creature_spawn_probability,
            categories,
            costs,
        })
    }

    pub fn resolve(
        &self,
        content_registry: &ContentRegistry,
    ) -> Result<BiomeMobSpawnSettings, MobSpawnCatalogError> {
        let path = format!("{BIOME_REGISTRY}.{}", self.biome_id);
        let categories = self
            .categories
            .iter()
            .map(|(category, spawns)| {
                let category_path = format!("{path}.spawners.{category:?}");
                Ok((*category, spawns.resolve(&category_path, content_registry)?))
            })
            .collect::<Result<Vec<_>, MobSpawnCatalogError>>()?;
        let costs = self
            .costs
            .iter()
            .map(|cost| {
                let entry_path = format!("{path}.spawn_costs.{}", cost.entity_type);
                let entity_type =
                    resolve_entity_type(&cost.entity_type, &entry_path, content_registry)?;
                Ok(MobSpawnCost {
                    entity_type_id: entity_type.id,
                    entity_type: entity_type.resource_id.to_string(),
                    energy_budget: cost.energy_budget,
                    charge: cost.charge,
                })
            })
            .collect::<Result<Vec<_>, MobSpawnCatalogError>>()?;
        Ok(BiomeMobSpawnSettings {
            biome_id: self.biome_id.clone(),
            creature_spawn_probability: self.creature_spawn_probability,
            categories,
            costs,
        })
    }
}

impl RawStructureSpawnOverrides {
    pub fn parse_from_root(root: CodecValue<'_>, path: &str) -> Result<Self, MobSpawnCatalogError> {
        let fields = required_map(root, path)?;
        let overrides_path = format!("{path}.spawn_overrides");
        let overrides_map = required_map(
            fields
                .field("spawn_overrides")
                .ok_or_else(|| refuse(&overrides_path, "object is absent"))?,
            &overrides_path,
        )?;
        let overrides = map_entries(overrides_map)
            .enumerate()
            .map(|(index, entry)| {
                let (raw_category, value) = entry.ok_or_else(|| {
                    refuse(format!("{overrides_path}[{index}]"), "map entry is absent")
                })?;
                let category_path = format!("{path}.spawn_overrides.{raw_category}");
                let category = parse_category(&raw_category, &category_path)?;
                let fields = required_map(value, &category_path)?;
                let bounding_box = match required_string(
                    fields.field("bounding_box"),
                    &format!("{category_path}.bounding_box"),
                )?
                .as_str()
                {
                    "piece" => StructureSpawnBoundingBox::Piece,
                    "full" => StructureSpawnBoundingBox::Full,
                    value => {
                        return Err(refuse(
                            format!("{category_path}.bounding_box"),
                            format!("unknown bounding-box type {value}"),
                        ))
                    }
                };
                let spawns = parse_raw_spawn_list(
                    fields.field("spawns").ok_or_else(|| {
                        refuse(format!("{category_path}.spawns"), "field is absent")
                    })?,
                    &format!("{category_path}.spawns"),
                )?;
                Ok(RawStructureMobSpawnOverride {
                    category,
                    bounding_box,
                    spawns,
                })
            })
            .collect::<Result<Vec<_>, MobSpawnCatalogError>>()?;
        Ok(Self { overrides })
    }

    pub fn resolve(
        &self,
        path: &str,
        content_registry: &ContentRegistry,
    ) -> Result<Vec<StructureMobSpawnOverride>, MobSpawnCatalogError> {
        self.overrides
            .iter()
            .map(|entry| {
                let spawns_path = format!("{path}.spawn_overrides.spawns");
                Ok(StructureMobSpawnOverride {
                    category: entry.category,
                    bounding_box: entry.bounding_box,
                    spawns: entry.spawns.resolve(&spawns_path, content_registry)?,
                })
            })
            .collect()
    }
}

impl RawMobSpawnList {
    fn resolve(
        &self,
        path: &str,
        content_registry: &ContentRegistry,
    ) -> Result<MobSpawnList, MobSpawnCatalogError> {
        let entries = self
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let type_path = format!("{path}[{index}].type");
                let mut entity_type =
                    resolve_entity_type(&entry.entity_type, &type_path, content_registry)?;
                // `SpawnerData` canonicalizes every MISC entity to pig
                // (`MobSpawnSettings.java:100-124`).
                if entity_type.mob_category == VanillaEntityMobCategory::Misc {
                    entity_type =
                        resolve_entity_type("minecraft:pig", &type_path, content_registry)?;
                    debug_assert_eq!(entity_type.id, VANILLA_ENTITY_TYPE_PIG_ID);
                }
                Ok(WeightedMobSpawnerData {
                    data: MobSpawnerData {
                        entity_type_id: entity_type.id,
                        entity_type: entity_type.resource_id.to_string(),
                        min_count: entry.min_count,
                        max_count: entry.max_count,
                    },
                    weight: entry.weight,
                })
            })
            .collect::<Result<Vec<_>, MobSpawnCatalogError>>()?;
        Ok(MobSpawnList {
            entries,
            total_weight: self.total_weight,
        })
    }
}

pub fn parse_structure_spawn_overrides_json(
    value: &RegistryJsonValue,
    content_registry: &ContentRegistry,
    path: &str,
) -> Result<Vec<StructureMobSpawnOverride>, MobSpawnCatalogError> {
    RawStructureSpawnOverrides::parse_from_root(CodecValue::Json(value), path)?
        .resolve(path, content_registry)
}

#[derive(Clone, Debug, PartialEq)]
pub struct DatapackMobSpawnCatalog {
    biomes: Vec<BiomeMobSpawnSettings>,
    biome_index: BTreeMap<String, usize>,
    has_any_spawn_costs: bool,
    costed_entity_types: Vec<i32>,
    structures: Vec<StructureMobSpawnSettings>,
    structure_index: BTreeMap<String, usize>,
}

impl DatapackMobSpawnCatalog {
    pub fn from_generation(generation: &DatapackGeneration) -> Result<Self, MobSpawnCatalogError> {
        Self::from_generation_with_registry(generation, ContentRegistry::vanilla())
    }

    pub fn from_generation_with_registry(
        generation: &DatapackGeneration,
        content_registry: &ContentRegistry,
    ) -> Result<Self, MobSpawnCatalogError> {
        let biome_registry = generation
            .registry(BIOME_REGISTRY)
            .ok_or_else(|| refuse(BIOME_REGISTRY, "registry is absent"))?;
        let structure_registry = generation
            .registry(STRUCTURE_REGISTRY)
            .ok_or_else(|| refuse(STRUCTURE_REGISTRY, "registry is absent"))?;

        let biomes = biome_registry
            .elements
            .iter()
            .map(|element| {
                if let Some(cached) = generation
                    .worldgen
                    .cached_biome_mob_spawns(&element.id, &element.source_json)
                {
                    return cached?.resolve(content_registry);
                }
                parse_biome_element(element, content_registry)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut costed_entity_types = biomes
            .iter()
            .flat_map(|biome| biome.costs.iter().map(|cost| cost.entity_type_id))
            .collect::<Vec<_>>();
        costed_entity_types.sort_unstable();
        costed_entity_types.dedup();
        let has_any_spawn_costs = !costed_entity_types.is_empty();
        let structures = structure_registry
            .elements
            .iter()
            .map(|element| {
                let path = format!("{STRUCTURE_REGISTRY}.{}", element.id);
                if let Some(cached) = generation
                    .worldgen
                    .cached_structure_mob_spawns(&element.id, &element.source_json)
                {
                    let overrides = cached?.resolve(&path, content_registry)?;
                    return Ok(StructureMobSpawnSettings {
                        structure_id: normalize_id(&element.id, &path)?,
                        overrides,
                    });
                }
                parse_structure_element(element, content_registry)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let biome_index = make_index(&biomes, |biome| &biome.biome_id);
        let structure_index = make_index(&structures, |structure| &structure.structure_id);
        Ok(Self {
            biomes,
            biome_index,
            has_any_spawn_costs,
            costed_entity_types,
            structures,
            structure_index,
        })
    }

    pub const fn has_any_spawn_costs(&self) -> bool {
        self.has_any_spawn_costs
    }

    pub fn entity_type_has_any_spawn_cost(&self, entity_type_id: i32) -> bool {
        self.costed_entity_types
            .binary_search(&entity_type_id)
            .is_ok()
    }

    pub fn biomes(&self) -> &[BiomeMobSpawnSettings] {
        &self.biomes
    }

    pub fn biome_by_index(&self, index: usize) -> Option<&BiomeMobSpawnSettings> {
        self.biomes.get(index)
    }

    pub fn biome(&self, id: &str) -> Option<&BiomeMobSpawnSettings> {
        let id = normalize_id(id, "biome query").ok()?;
        self.biome_index.get(&id).map(|index| &self.biomes[*index])
    }

    pub fn structures(&self) -> &[StructureMobSpawnSettings] {
        &self.structures
    }

    pub fn structure(&self, id: &str) -> Option<&StructureMobSpawnSettings> {
        let id = normalize_id(id, "structure query").ok()?;
        self.structure_index
            .get(&id)
            .map(|index| &self.structures[*index])
    }
}

fn make_index<T>(values: &[T], id: impl Fn(&T) -> &String) -> BTreeMap<String, usize> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| (id(value).clone(), index))
        .collect()
}

fn parse_biome_element(
    element: &RawRegistryElement,
    content_registry: &ContentRegistry,
) -> Result<BiomeMobSpawnSettings, MobSpawnCatalogError> {
    let path = format!("{BIOME_REGISTRY}.{}", element.id);
    let decoded = element
        .decoded_loaded_source(usize::MAX)
        .map_err(|error| refuse(&path, format!("invalid source JSON: {error}")))?;
    RawBiomeMobSpawnSettings::parse_from_root(&element.id, decoded.root())?
        .resolve(content_registry)
}

fn parse_structure_element(
    element: &RawRegistryElement,
    content_registry: &ContentRegistry,
) -> Result<StructureMobSpawnSettings, MobSpawnCatalogError> {
    let path = format!("{STRUCTURE_REGISTRY}.{}", element.id);
    let decoded = element
        .decoded_loaded_source(usize::MAX)
        .map_err(|error| refuse(&path, format!("invalid source JSON: {error}")))?;
    let overrides = RawStructureSpawnOverrides::parse_from_root(decoded.root(), &path)?
        .resolve(&path, content_registry)?;
    Ok(StructureMobSpawnSettings {
        structure_id: normalize_id(&element.id, &path)?,
        overrides,
    })
}

fn parse_raw_spawn_list(
    value: CodecValue<'_>,
    path: &str,
) -> Result<RawMobSpawnList, MobSpawnCatalogError> {
    let len = value
        .list_len()
        .ok_or_else(|| refuse(path, "weighted spawn list is absent"))?;
    let entries = (0..len)
        .map(|index| {
            parse_raw_weighted_spawn(
                value
                    .list_element(index)
                    .ok_or_else(|| refuse(format!("{path}[{index}]"), "entry is absent"))?,
                &format!("{path}[{index}]"),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    // `WeightedList` accepts zero weights and empty/all-zero lists, but rejects
    // a sum outside signed-int range (`Weighted.java:17-40`, `WeightedRandom.java:13-24`).
    let total = entries
        .iter()
        .map(|entry| i64::from(entry.weight))
        .sum::<i64>();
    let total_weight =
        i32::try_from(total).map_err(|_| refuse(path, "sum of weights exceeds 2147483647"))?;
    Ok(RawMobSpawnList {
        entries,
        total_weight,
    })
}

fn parse_raw_weighted_spawn(
    value: CodecValue<'_>,
    path: &str,
) -> Result<RawWeightedMobSpawnerData, MobSpawnCatalogError> {
    let fields = required_map(value, path)?;
    let entity_type = required_string(fields.field("type"), &format!("{path}.type"))?;
    let min_count = positive_i32(fields.field("minCount"), &format!("{path}.minCount"))?;
    let max_count = positive_i32(fields.field("maxCount"), &format!("{path}.maxCount"))?;
    if min_count > max_count {
        return Err(refuse(path, "minCount must not exceed maxCount"));
    }
    Ok(RawWeightedMobSpawnerData {
        entity_type,
        min_count,
        max_count,
        weight: non_negative_i32(fields.field("weight"), &format!("{path}.weight"))?,
    })
}

fn required_map<'a>(
    value: CodecValue<'a>,
    path: &str,
) -> Result<CodecValue<'a>, MobSpawnCatalogError> {
    value
        .map_len()
        .map(|_| value)
        .ok_or_else(|| refuse(path, "object is absent"))
}

fn map_entries(value: CodecValue<'_>) -> impl Iterator<Item = Option<(String, CodecValue<'_>)>> {
    (0..value.map_len().unwrap_or(0)).map(move |index| {
        let (key, value) = value.map_entry_java_string_ref(index)?;
        Some((key.try_to_string().ok()?, value))
    })
}

fn required_string<'a>(
    value: Option<CodecValue<'a>>,
    path: &str,
) -> Result<String, MobSpawnCatalogError> {
    value
        .and_then(CodecValue::utf8_string)
        .ok_or_else(|| refuse(path, "string is absent"))
}

fn required_f64(value: Option<CodecValue<'_>>, path: &str) -> Result<f64, MobSpawnCatalogError> {
    value
        .and_then(CodecValue::codec_f64)
        .ok_or_else(|| refuse(path, "number is absent"))
}

fn ranged_f32(
    value: CodecValue<'_>,
    min: f32,
    max: f32,
    path: &str,
) -> Result<f32, MobSpawnCatalogError> {
    let value = value
        .codec_f32()
        .filter(|value| *value >= min && *value <= max)
        .ok_or_else(|| refuse(path, format!("number must be in {min}..={max}")))?;
    Ok(value)
}

fn positive_i32(value: Option<CodecValue<'_>>, path: &str) -> Result<i32, MobSpawnCatalogError> {
    let value = integer_i32(value, path)?;
    if value < 1 {
        return Err(refuse(path, "integer must be positive"));
    }
    Ok(value)
}

fn non_negative_i32(
    value: Option<CodecValue<'_>>,
    path: &str,
) -> Result<i32, MobSpawnCatalogError> {
    let value = integer_i32(value, path)?;
    if value < 0 {
        return Err(refuse(path, "integer must be non-negative"));
    }
    Ok(value)
}

fn integer_i32(value: Option<CodecValue<'_>>, path: &str) -> Result<i32, MobSpawnCatalogError> {
    value
        .and_then(CodecValue::codec_i32)
        .ok_or_else(|| refuse(path, "signed 32-bit integer is absent"))
}

fn parse_category(
    value: &str,
    path: &str,
) -> Result<VanillaEntityMobCategory, MobSpawnCatalogError> {
    vanilla_entity_mob_category_for_serialized_name(value)
        .ok_or_else(|| refuse(path, format!("unknown mob category {value}")))
}

struct ResolvedEntityType {
    id: i32,
    resource_id: &'static str,
    mob_category: VanillaEntityMobCategory,
}

fn resolve_entity_type(
    value: &str,
    path: &str,
    content_registry: &ContentRegistry,
) -> Result<ResolvedEntityType, MobSpawnCatalogError> {
    let id = normalize_id(value, path)?;
    if let Some(entry) = vanilla_entity_type_for_resource_id(&id) {
        return Ok(ResolvedEntityType {
            id: entry.id,
            resource_id: entry.resource_id,
            mob_category: entry.mob_category,
        });
    }
    if let Some(spec) = content_registry.entity_by_resource_id(&id) {
        return Ok(ResolvedEntityType {
            id: spec.id,
            resource_id: spec.resource_id_static(),
            mob_category: spec.category,
        });
    }
    Err(refuse(path, format!("unknown entity type {id}")))
}

fn normalize_id(value: &str, path: &str) -> Result<String, MobSpawnCatalogError> {
    bbb_protocol::normalize_resource_location(value.to_string()).map_err(|error| {
        refuse(
            path,
            format!("invalid resource location {value:?}: {error}"),
        )
    })
}

fn refuse(path: impl Into<String>, reason: impl Into<String>) -> MobSpawnCatalogError {
    MobSpawnCatalogError {
        path: path.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
#[path = "mob_spawn/tests.rs"]
mod tests;
