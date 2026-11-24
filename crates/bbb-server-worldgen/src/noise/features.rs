//! Compile biome feature holders into Vanilla's cross-biome decoration order.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use crate::random::WorldgenRandom;
use bbb_protocol::packets::BlockPos;

use crate::fast_hash::{FastHashMap, FastHashSet};
use crate::placed_feature::place_placed_feature_with_biome_check;
use crate::WorldGenLevel;
use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::tag_loader::TagLoadReport;
use bbb_server_datapack::worldgen::{PlacedFeature, PlacedFeatureHolder};

pub mod end;
pub mod nether;

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
const PLACED_FEATURE_REGISTRY: &str = "minecraft:worldgen/placed_feature";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoiseFeatureInvocation {
    pub step: usize,
    pub global_index: usize,
    pub placed_feature: String,
    pub feature_seed: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoiseFeaturePlacementSummary {
    pub attempted: usize,
    pub placed: usize,
    pub placed_invocations: Vec<NoiseFeatureInvocation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoiseFeaturesError {
    path: String,
    reason: String,
}

impl fmt::Display for NoiseFeaturesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.reason)
    }
}

impl std::error::Error for NoiseFeaturesError {}

#[derive(Clone, Debug)]
pub struct NoiseFeaturesPlan {
    known_biomes: FastHashSet<String>,
    biome_step_indices: FastHashMap<String, Vec<Vec<usize>>>,
    biome_feature_sets: FastHashMap<String, FastHashSet<String>>,
    steps: Vec<FeatureStepPlan>,
}

#[derive(Clone, Debug)]
struct FeatureStepPlan {
    features: Vec<String>,
    holders: Vec<Arc<PlacedFeature>>,
}

#[derive(Clone, Debug)]
struct BiomeFeatureSource {
    id: String,
    steps: Vec<Vec<String>>,
}

pub struct BiomeFeatureSelection<'a> {
    pub copy_original: bool,
    pub excluded_steps: &'a [usize],
    pub appended: &'a [(usize, PlacedFeatureHolder)],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FeatureNode {
    step: usize,
    feature_index: usize,
}

impl NoiseFeaturesPlan {
    /// `FeatureSorter.buildFeaturesPerStep` (`FeatureSorter.java:28-109`).
    pub fn from_registry(
        generation: &DatapackGeneration,
        possible_biome_ids: &[&str],
    ) -> Result<Self, NoiseFeaturesError> {
        Self::from_registry_inner(generation, None, possible_biome_ids, None)
    }

    /// Registry-aware `HolderSet<PlacedFeature>` decoding
    /// (`BiomeGenerationSettings.java:29-52`, `HolderSetCodec.java:25-67`).
    pub fn from_registry_with_tags(
        generation: &DatapackGeneration,
        tags: &TagLoadReport,
        possible_biome_ids: &[&str],
    ) -> Result<Self, NoiseFeaturesError> {
        Self::from_registry_inner(generation, Some(tags), possible_biome_ids, None)
    }

    pub fn from_registry_with_biome_feature_selection(
        generation: &DatapackGeneration,
        tags: &TagLoadReport,
        biome: &str,
        selection: BiomeFeatureSelection<'_>,
    ) -> Result<Self, NoiseFeaturesError> {
        Self::from_registry_inner(generation, Some(tags), &[biome], Some(selection))
    }

    fn from_registry_inner(
        generation: &DatapackGeneration,
        tags: Option<&TagLoadReport>,
        possible_biome_ids: &[&str],
        selection: Option<BiomeFeatureSelection<'_>>,
    ) -> Result<Self, NoiseFeaturesError> {
        let biome_registry = generation
            .registry(BIOME_REGISTRY)
            .ok_or_else(|| refuse(BIOME_REGISTRY, "registry is absent"))?;
        generation
            .registry(PLACED_FEATURE_REGISTRY)
            .ok_or_else(|| refuse(PLACED_FEATURE_REGISTRY, "registry is absent"))?;
        let known_biomes = biome_registry
            .elements
            .iter()
            .map(|element| element.id.clone())
            .collect::<FastHashSet<_>>();
        let mut possible_biomes = FastHashSet::default();
        let mut sources = Vec::new();
        let mut holders = BTreeMap::new();
        for (index, raw_id) in possible_biome_ids.iter().enumerate() {
            let input_path = format!("{BIOME_REGISTRY}.possible_biomes[{index}]");
            let id = normalize_id(raw_id, &input_path)?;
            if !known_biomes.contains(&id) {
                return Err(refuse(input_path, format!("unknown biome {id}")));
            }
            if possible_biomes.insert(id.clone()) {
                let mut source = compile_biome_features(generation, tags, &id, &mut holders)?;
                if let Some(selection) = &selection {
                    for (step, holders) in source.steps.iter_mut().enumerate() {
                        if !selection.copy_original || selection.excluded_steps.contains(&step) {
                            holders.clear();
                        }
                    }
                    // Runtime PlainBuilder additions follow the decoded biome HolderSets.
                    for (index, (step, holder)) in selection.appended.iter().enumerate() {
                        source
                            .steps
                            .resize_with(source.steps.len().max(step + 1), Vec::new);
                        let path = format!("{BIOME_REGISTRY}.{id}.runtime_features[{index}]");
                        let key = match holder {
                            PlacedFeatureHolder::Named(id) => {
                                if !holders.contains_key(id) {
                                    let feature = generation
                                        .worldgen
                                        .placed_features
                                        .get(id)
                                        .cloned()
                                        .ok_or_else(|| {
                                            refuse(
                                                &path,
                                                format!("unresolved placed-feature holder {id}"),
                                            )
                                        })?;
                                    holders.insert(id.clone(), feature);
                                }
                                id.clone()
                            }
                            PlacedFeatureHolder::Inline(feature) => {
                                holders.insert(path.clone(), Arc::clone(feature));
                                path
                            }
                        };
                        source.steps[*step].push(key);
                    }
                }
                sources.push(source);
            }
        }
        let selected = sources
            .iter()
            .flat_map(|source| source.steps.iter().flatten())
            .collect::<BTreeSet<_>>();
        holders.retain(|key, _| selected.contains(key));
        let sorted_steps = sort_features_per_step(&sources)
            .map_err(|()| refuse(BIOME_REGISTRY, "feature order cycle found"))?;
        let mut step_index_maps = Vec::with_capacity(sorted_steps.len());
        let mut steps = Vec::with_capacity(sorted_steps.len());
        for features in sorted_steps {
            let mut index_by_feature = FastHashMap::default();
            let mut step_holders = Vec::with_capacity(features.len());
            for (index, feature) in features.iter().enumerate() {
                index_by_feature.insert(feature.clone(), index);
                step_holders.push(Arc::clone(&holders[feature]));
            }
            step_index_maps.push(index_by_feature);
            steps.push(FeatureStepPlan {
                features,
                holders: step_holders,
            });
        }
        let mut biome_step_indices = FastHashMap::default();
        let mut biome_feature_sets = FastHashMap::default();
        for source in sources {
            let feature_set = source
                .steps
                .iter()
                .flatten()
                .cloned()
                .collect::<FastHashSet<_>>();
            let mut resolved_steps = Vec::with_capacity(source.steps.len());
            for (step, step_features) in source.steps.iter().enumerate() {
                let mut indices = Vec::with_capacity(step_features.len());
                let index_map = &step_index_maps[step];
                for feature in step_features {
                    let index = index_map.get(feature).copied().ok_or_else(|| {
                        refuse(
                            format!("{BIOME_REGISTRY}.{}.features[{step}]", source.id),
                            format!("compiled order has no entry for {feature}"),
                        )
                    })?;
                    indices.push(index);
                }
                indices.sort_unstable();
                indices.dedup();
                resolved_steps.push(indices);
            }
            biome_feature_sets.insert(source.id.clone(), feature_set);
            biome_step_indices.insert(source.id, resolved_steps);
        }

        Ok(Self {
            known_biomes,
            biome_step_indices,
            biome_feature_sets,
            steps,
        })
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn biome_contains_placed_feature(&self, biome_id: &str, placed_feature_id: &str) -> bool {
        self.biome_feature_sets
            .get(biome_id)
            .is_some_and(|features| features.contains(placed_feature_id))
    }

    /// Whether the biome declares this top-feature plan key.
    pub fn biome_contains_top_feature(&self, biome_id: &str, top_feature: &str) -> bool {
        self.biome_contains_placed_feature(biome_id, top_feature)
    }

    /// `ChunkGenerator.applyBiomeDecoration` (`ChunkGenerator.java:313-390`);
    /// each invocation's seed hashes the chunk's decoration seed with its
    /// step and index.
    pub fn invocations_for_chunk(
        &self,
        world_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        neighborhood_biome_ids: &[&str],
    ) -> Result<Vec<NoiseFeatureInvocation>, NoiseFeaturesError> {
        self.invocations_for_decoration_seed(
            decoration_seed(world_seed, chunk_x, chunk_z),
            neighborhood_biome_ids,
            None,
        )
    }

    fn invocations_for_decoration_seed(
        &self,
        decoration_seed: i64,
        neighborhood_biome_ids: &[&str],
        only_step: Option<usize>,
    ) -> Result<Vec<NoiseFeatureInvocation>, NoiseFeaturesError> {
        let mut seen_biomes: Vec<&str> = Vec::with_capacity(neighborhood_biome_ids.len());
        let mut neighborhood: Vec<&[Vec<usize>]> = Vec::with_capacity(neighborhood_biome_ids.len());
        for (index, raw_id) in neighborhood_biome_ids.iter().enumerate() {
            let entry = if self.known_biomes.contains(*raw_id) {
                self.biome_step_indices.get_key_value(*raw_id)
            } else {
                let path = format!("{BIOME_REGISTRY}.neighborhood_biomes[{index}]");
                let id = normalize_id(raw_id, &path)?;
                if !self.known_biomes.contains(&id) {
                    return Err(refuse(path, format!("unknown biome {id}")));
                }
                self.biome_step_indices.get_key_value(&id)
            };
            if let Some((canonical, steps)) = entry {
                let canonical = canonical.as_str();
                if !seen_biomes.contains(&canonical) {
                    seen_biomes.push(canonical);
                    neighborhood.push(steps.as_slice());
                }
            }
        }

        let mut invocations = Vec::new();
        let step_range = match only_step {
            Some(step) if step < self.steps.len() => step..step + 1,
            Some(_) => return Ok(invocations),
            None => 0..self.steps.len(),
        };
        let mut scratch_indices = Vec::new();
        for step in step_range {
            let step_plan = &self.steps[step];
            let java_step = i32::try_from(step)
                .map_err(|_| refuse(BIOME_REGISTRY, "feature step does not fit Java int"))?;
            let step_indices: &[usize] = if neighborhood.len() == 1 {
                neighborhood[0].get(step).map_or(&[], Vec::as_slice)
            } else {
                scratch_indices.clear();
                for biome_steps in &neighborhood {
                    if let Some(features) = biome_steps.get(step) {
                        scratch_indices.extend_from_slice(features);
                    }
                }
                scratch_indices.sort_unstable();
                scratch_indices.dedup();
                &scratch_indices
            };
            for &global_index in step_indices {
                let index_seed = i32::try_from(global_index).map_err(|_| {
                    refuse(
                        PLACED_FEATURE_REGISTRY,
                        "feature index does not fit Java int",
                    )
                })?;
                invocations.push(NoiseFeatureInvocation {
                    step,
                    global_index,
                    placed_feature: step_plan.features[global_index].clone(),
                    feature_seed: expected_feature_seed(decoration_seed, index_seed, java_step),
                });
            }
        }
        Ok(invocations)
    }

    /// `ChunkGenerator.java:361-394`.
    #[allow(clippy::too_many_arguments)]
    pub fn place_step_for_chunk<L: WorldGenLevel>(
        &self,
        world_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        step: usize,
        neighborhood_biome_ids: &[&str],
        level: &mut L,
    ) -> Result<(Vec<NoiseFeatureInvocation>, NoiseFeaturePlacementSummary), NoiseFeaturesError>
    {
        let origin_x = chunk_x.wrapping_mul(16);
        let origin_z = chunk_z.wrapping_mul(16);
        let origin = BlockPos {
            x: origin_x,
            y: min_y,
            z: origin_z,
        };
        let decoration_seed = decoration_seed(world_seed, chunk_x, chunk_z);
        let selected = self.invocations_for_decoration_seed(
            decoration_seed,
            neighborhood_biome_ids,
            Some(step),
        )?;
        let summary = self.place_invocations(&selected, origin, level);
        Ok((selected, summary))
    }

    fn place_invocations<L: WorldGenLevel>(
        &self,
        invocations: &[NoiseFeatureInvocation],
        origin: BlockPos,
        level: &mut L,
    ) -> NoiseFeaturePlacementSummary {
        let mut summary = NoiseFeaturePlacementSummary::default();
        for invocation in invocations {
            let step_plan = &self.steps[invocation.step];
            let key = &step_plan.features[invocation.global_index];
            let holder = &step_plan.holders[invocation.global_index];
            summary.attempted += 1;
            let mut random = WorldgenRandom::with_seed(invocation.feature_seed);
            if place_placed_feature_with_biome_check(holder, level, &mut random, origin, key) {
                summary.placed += 1;
                summary.placed_invocations.push(invocation.clone());
            }
        }
        summary
    }
}

/// The seed of one feature or structure placement in a chunk, keyed by its
/// decoration step and index there.
pub fn expected_feature_seed(decoration_seed: i64, global_index: i32, step: i32) -> i64 {
    crate::random::hash_seed(&[decoration_seed, i64::from(step), i64::from(global_index)])
}

fn compile_biome_features(
    generation: &DatapackGeneration,
    tags: Option<&TagLoadReport>,
    id: &str,
    compiled_holders: &mut BTreeMap<String, Arc<PlacedFeature>>,
) -> Result<BiomeFeatureSource, NoiseFeaturesError> {
    let source = generation.worldgen.biome_features.get(id).ok_or_else(|| {
        refuse(
            format!("{BIOME_REGISTRY}.{id}.features"),
            "typed biome features are absent",
        )
    })?;
    let steps = source
        .resolve_steps(&generation.worldgen, tags, id)
        .map_err(|error| refuse(format!("{BIOME_REGISTRY}.{id}.features"), error.to_string()))?
        .into_iter()
        .map(|holders| {
            holders
                .into_iter()
                .map(|(key, feature)| {
                    compiled_holders.entry(key.clone()).or_insert(feature);
                    key
                })
                .collect()
        })
        .collect();
    Ok(BiomeFeatureSource {
        id: id.to_string(),
        steps,
    })
}

fn sort_features_per_step(sources: &[BiomeFeatureSource]) -> Result<Vec<Vec<String>>, ()> {
    let mut feature_indices = BTreeMap::<String, usize>::new();
    let mut feature_ids = Vec::<String>::new();
    let mut edges = BTreeMap::<FeatureNode, BTreeSet<FeatureNode>>::new();
    let mut max_step = 0;

    for source in sources {
        max_step = max_step.max(source.steps.len());
        let mut flat = Vec::new();
        for (step, features) in source.steps.iter().enumerate() {
            for feature in features {
                let feature_index = match feature_indices.get(feature) {
                    Some(index) => *index,
                    None => {
                        let index = feature_ids.len();
                        feature_ids.push(feature.clone());
                        feature_indices.insert(feature.clone(), index);
                        index
                    }
                };
                flat.push(FeatureNode {
                    step,
                    feature_index,
                });
            }
        }
        for (index, node) in flat.iter().copied().enumerate() {
            let successors = edges.entry(node).or_default();
            if let Some(next) = flat.get(index + 1) {
                successors.insert(*next);
            }
        }
    }

    let mut discovered = BTreeSet::new();
    let mut visiting = BTreeSet::new();
    let mut reverse_topological = Vec::new();
    for node in edges.keys().copied() {
        depth_first_search(
            node,
            &edges,
            &mut discovered,
            &mut visiting,
            &mut reverse_topological,
        )?;
    }
    reverse_topological.reverse();

    let mut result = vec![Vec::new(); max_step];
    for node in reverse_topological {
        result[node.step].push(feature_ids[node.feature_index].clone());
    }
    Ok(result)
}

fn depth_first_search(
    current: FeatureNode,
    edges: &BTreeMap<FeatureNode, BTreeSet<FeatureNode>>,
    discovered: &mut BTreeSet<FeatureNode>,
    visiting: &mut BTreeSet<FeatureNode>,
    reverse_topological: &mut Vec<FeatureNode>,
) -> Result<(), ()> {
    if discovered.contains(&current) {
        return Ok(());
    }
    if !visiting.insert(current) {
        return Err(());
    }
    if let Some(successors) = edges.get(&current) {
        for successor in successors {
            depth_first_search(*successor, edges, discovered, visiting, reverse_topological)?;
        }
    }
    visiting.remove(&current);
    discovered.insert(current);
    reverse_topological.push(current);
    Ok(())
}

pub fn decoration_seed(world_seed: i64, chunk_x: i32, chunk_z: i32) -> i64 {
    crate::random::hash_seed(&[world_seed, i64::from(chunk_x), i64::from(chunk_z)])
}

fn normalize_id(raw: &str, path: &str) -> Result<String, NoiseFeaturesError> {
    bbb_protocol::normalize_resource_location(raw.to_string())
        .map_err(|error| refuse(path, error.to_string()))
}

pub(super) fn refuse(path: impl Into<String>, reason: impl Into<String>) -> NoiseFeaturesError {
    NoiseFeaturesError {
        path: path.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
pub mod tests;
