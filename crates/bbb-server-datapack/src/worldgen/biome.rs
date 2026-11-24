//! Typed biome generation holders and their bone meal feature lists.

use std::collections::HashSet;
use std::sync::Arc;

use bbb_registry_codec::RegistryJsonValue as Json;

use super::json::{self, fail, At, Obj, Result};
use super::{
    ConfiguredFeature, Holder, PlacedFeature, PlacedFeatureHolder, PlacedFeatureSet, Refs,
    WorldgenRegistries, CONFIGURED_FEATURE_REGISTRY, PLACED_FEATURE_REGISTRY,
};
use crate::tag_loader::TagLoadReport;

pub type BiomeFeatureStep = Vec<(String, Arc<PlacedFeature>)>;

#[derive(Debug, Clone, PartialEq)]
pub struct BiomeFeatures {
    steps: Vec<PlacedFeatureSet>,
}

impl BiomeFeatures {
    pub fn decode(biome: &Json) -> Result<Self> {
        let steps = Obj::of(biome)?.req("features", |value| {
            json::list(value, |step| {
                if let Json::String(tag) = step {
                    return match tag.strip_prefix('#') {
                        Some(tag) => Ok(PlacedFeatureSet::Tag(json::id(&Json::String(tag))?)),
                        None => {
                            fail("a direct holder set must be a list; only a #tag may be a string")
                        }
                    };
                }
                let mut kind = None;
                let holders = json::list(step, |holder| {
                    let named = matches!(holder, Json::String(_));
                    if kind.is_some_and(|expected| expected != named) {
                        return fail("direct holder list must be homogeneous");
                    }
                    kind = Some(named);
                    PlacedFeatureHolder::decode_placed(holder)
                })?;
                Ok(PlacedFeatureSet::Direct(holders))
            })
        })?;
        Ok(Self { steps })
    }

    pub(crate) fn refs(&self, refs: &mut Refs) {
        for step in &self.steps {
            match step {
                PlacedFeatureSet::Tag(tag) => refs.tag(PLACED_FEATURE_REGISTRY, tag),
                PlacedFeatureSet::Direct(holders) => {
                    for holder in holders {
                        holder.refs(refs);
                    }
                }
            }
        }
    }

    /// Resolved holders retain their source keys for generation's step ordering.
    pub fn resolve_steps(
        &self,
        registries: &WorldgenRegistries,
        tags: Option<&TagLoadReport>,
        biome: &str,
    ) -> Result<Vec<BiomeFeatureStep>> {
        self.steps
            .iter()
            .enumerate()
            .map(|(step, set)| {
                resolve_set(set, registries, tags)?
                    .into_iter()
                    .enumerate()
                    .map(|(index, holder)| {
                        let key = match &holder {
                            Holder::Named(id) => id.clone(),
                            Holder::Inline(_) => format!(
                                "minecraft:worldgen/biome.{biome}.features[{step}][{index}]"
                            ),
                        };
                        Ok((key, resolve_placed(&holder, registries)?))
                    })
                    .collect()
            })
            .collect()
    }
}

fn resolve_set(
    set: &PlacedFeatureSet,
    registries: &WorldgenRegistries,
    tags: Option<&TagLoadReport>,
) -> Result<Vec<PlacedFeatureHolder>> {
    match set {
        PlacedFeatureSet::Direct(holders) => Ok(holders.clone()),
        PlacedFeatureSet::Tag(tag) => tags
            .ok_or_else(|| json::DecodeError {
                path: tag.clone(),
                message: "unresolved placed-feature tag".into(),
            })?
            .registry(PLACED_FEATURE_REGISTRY)
            .and_then(|registry| registry.tag(tag))
            .unwrap_or(&[])
            .iter()
            .map(|id| {
                if registries.placed_features.contains_key(id) {
                    Ok(Holder::Named(id.clone()))
                } else {
                    fail(format!("unresolved placed-feature holder {id}"))
                }
            })
            .collect(),
    }
}

fn resolve_placed(
    holder: &PlacedFeatureHolder,
    registries: &WorldgenRegistries,
) -> Result<Arc<PlacedFeature>> {
    match holder {
        Holder::Inline(value) => Ok(value.clone()),
        Holder::Named(id) => {
            registries
                .placed_features
                .get(id)
                .cloned()
                .ok_or_else(|| json::DecodeError {
                    path: id.clone(),
                    message: "unresolved placed-feature holder".into(),
                })
        }
    }
}

impl WorldgenRegistries {
    /// `BiomeGenerationSettings.java:44-56`, `PlacedFeature.java:63-65`:
    /// compute tagged configured roots and selector descendants without mutating
    /// the generation so `/reload` can avoid cloning when unchanged.
    pub(crate) fn compute_bonemeal_features(
        &self,
        tags: &TagLoadReport,
    ) -> Result<std::collections::HashMap<String, Arc<[Arc<ConfiguredFeature>]>>> {
        let mut lists = std::collections::HashMap::new();
        for (id, biome) in &self.biome_features {
            let mut features = Vec::new();
            let mut remaining = 1 << 20;
            for (_, placed) in biome
                .resolve_steps(self, Some(tags), id)?
                .into_iter()
                .flatten()
            {
                collect_features(
                    &placed,
                    self,
                    tags,
                    &mut HashSet::new(),
                    &mut remaining,
                    &mut features,
                )
                .at(id)?;
            }
            lists.insert(id.clone(), Arc::from(features));
        }
        Ok(lists)
    }
}

fn collect_features(
    placed: &PlacedFeature,
    registries: &WorldgenRegistries,
    tags: &TagLoadReport,
    visiting: &mut HashSet<usize>,
    remaining: &mut usize,
    result: &mut Vec<Arc<ConfiguredFeature>>,
) -> Result<()> {
    if *remaining == 0 {
        return fail("biome feature traversal budget exceeded");
    }
    *remaining -= 1;
    let feature = match &placed.feature {
        Holder::Inline(value) => value.clone(),
        Holder::Named(id) => registries
            .configured_features
            .get(id)
            .cloned()
            .ok_or_else(|| json::DecodeError {
                path: id.clone(),
                message: "unresolved configured-feature holder".into(),
            })?,
    };
    let key = Arc::as_ptr(&feature) as usize;
    if visiting.len() >= 256 || !visiting.insert(key) {
        return fail("cyclic or excessively nested biome feature selectors");
    }
    if let Holder::Named(id) = &placed.feature {
        if tags
            .registry(CONFIGURED_FEATURE_REGISTRY)
            .and_then(|registry| registry.tag("minecraft:can_spawn_from_bone_meal"))
            .is_some_and(|members| members.contains(id))
        {
            result.push(feature.clone());
        }
    }
    // Only these three configurations expose subfeatures (`RandomFeatureConfiguration.java:29-30`,
    // `SimpleRandomFeatureConfiguration.java:23-24`, `RandomBooleanFeatureConfiguration.java:27-28`).
    let children = match &*feature {
        ConfiguredFeature::RandomSelector(config) => config
            .features
            .iter()
            .map(|(holder, _)| holder.clone())
            .chain(std::iter::once(config.default.clone()))
            .collect(),
        ConfiguredFeature::SimpleRandomSelector(set) => resolve_set(set, registries, Some(tags))?,
        ConfiguredFeature::RandomBooleanSelector {
            feature_true,
            feature_false,
        } => vec![feature_true.clone(), feature_false.clone()],
        _ => Vec::new(),
    };
    for child in children {
        collect_features(
            resolve_placed(&child, registries)?.as_ref(),
            registries,
            tags,
            visiting,
            remaining,
            result,
        )?;
    }
    visiting.remove(&key);
    Ok(())
}
