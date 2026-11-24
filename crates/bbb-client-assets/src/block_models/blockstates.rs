use anyhow::{bail, Result};
use bbb_registry_codec::legacy_random::LegacyRandomSource;
use std::collections::BTreeMap;

use serde::Deserialize;

pub(super) const MAX_MULTIPART_SELECTORS_PER_BLOCKSTATE: usize = 128;
pub(super) const MAX_WEIGHTED_VARIANTS_PER_SELECTOR: usize = 64;
const MAX_MULTIPART_CONDITION_DEPTH: usize = 16;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RawBlockstate {
    #[serde(default)]
    variants: BTreeMap<String, RawBlockstateVariant>,
    #[serde(default)]
    multipart: Vec<RawMultipartCase>,
}

impl RawBlockstate {
    pub(super) fn validate(&self) -> Result<()> {
        if self.multipart.len() > MAX_MULTIPART_SELECTORS_PER_BLOCKSTATE {
            bail!(
                "blockstate has {} multipart selectors, exceeding the {MAX_MULTIPART_SELECTORS_PER_BLOCKSTATE} selector limit",
                self.multipart.len()
            );
        }
        for (key, variant) in &self.variants {
            variant.validate(&format!("variant {key:?}"))?;
        }
        for (index, part) in self.multipart.iter().enumerate() {
            if let Some(when) = &part.when {
                validate_multipart_condition_depth(when, 0)?;
            }
            part.apply
                .validate(&format!("multipart selector {index}"))?;
        }
        Ok(())
    }

    pub(super) fn multipart_selector_slices(&self) -> impl Iterator<Item = &[RawModelVariant]> {
        self.multipart.iter().map(|part| part.apply.as_slice())
    }

    pub(super) fn all_referenced_variants(&self) -> impl Iterator<Item = &RawModelVariant> {
        self.variants
            .values()
            .flat_map(RawBlockstateVariant::as_slice)
            .chain(self.multipart.iter().flat_map(|part| part.apply.as_slice()))
    }

    pub(super) fn selection_entry_count(&self) -> Option<usize> {
        self.variants
            .values()
            .map(RawBlockstateVariant::entry_count)
            .chain(self.multipart.iter().map(|part| part.apply.entry_count()))
            .try_fold(0_usize, usize::checked_add)
    }

    /// Returns the weighted candidate lists that position-seeded selection picks from: the best
    /// matching `variants` entry, or one list per matching multipart case.
    pub(super) fn model_choices(
        &self,
        properties: &BTreeMap<String, String>,
    ) -> Option<BlockStateModelChoices> {
        let mut best_variant = None;
        let mut best_score = 0usize;
        for (key, variant) in &self.variants {
            let Some(score) = variant_key_match_score(key, properties) else {
                continue;
            };
            if best_variant.is_none() || score >= best_score {
                best_score = score;
                best_variant = Some(variant);
            }
        }
        if let Some(variant) = best_variant.filter(|variant| variant.entry_count() > 0) {
            return Some(BlockStateModelChoices::new(
                vec![variant.all_models()],
                false,
            ));
        }

        if self.multipart.is_empty() {
            return None;
        }

        let parts = self
            .multipart
            .iter()
            .filter(|case| case.matches(properties))
            .map(|case| case.apply.all_models())
            .filter(|models| !models.is_empty())
            .collect();
        Some(BlockStateModelChoices::new(parts, true))
    }

    /// Returns every model whose baked material flags contribute to this block-state model.
    ///
    /// Vanilla's `WeightedVariants.materialFlags` ORs every weighted candidate for the matched
    /// variants entry, while `MultiPartModel.SharedBakedState.materialFlags` ORs every selector
    /// model regardless of whether that selector matches the current state. This is deliberately
    /// separate from `select_variants`, which chooses the geometry rendered for one position.
    pub(super) fn material_flag_variants(
        &self,
        properties: &BTreeMap<String, String>,
    ) -> Option<RawBlockstateSelection> {
        let mut best_variant = None;
        let mut best_score = 0usize;
        for (key, variant) in &self.variants {
            let Some(score) = variant_key_match_score(key, properties) else {
                continue;
            };
            if best_variant.is_none() || score >= best_score {
                best_score = score;
                best_variant = Some(variant);
            }
        }
        if let Some(variant) = best_variant {
            return Some(RawBlockstateSelection::Variants(variant.all_models()));
        }

        if self.multipart.is_empty() {
            return None;
        }

        let variants = self
            .multipart
            .iter()
            .flat_map(|case| case.apply.all_models())
            .collect::<Vec<_>>();
        Some(if variants.is_empty() {
            RawBlockstateSelection::Empty
        } else {
            RawBlockstateSelection::Variants(variants)
        })
    }
}

#[derive(Debug, Clone)]
pub(super) enum RawBlockstateSelection {
    Variants(Vec<RawModelVariant>),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum RawBlockstateVariant {
    One(RawModelVariant),
    Many(Vec<RawModelVariant>),
}

impl RawBlockstateVariant {
    fn entry_count(&self) -> usize {
        match self {
            Self::One(_) => 1,
            Self::Many(models) => models.len(),
        }
    }

    fn as_slice(&self) -> &[RawModelVariant] {
        match self {
            Self::One(model) => std::slice::from_ref(model),
            Self::Many(models) => models.as_slice(),
        }
    }

    fn all_models(&self) -> Vec<RawModelVariant> {
        self.as_slice().to_vec()
    }

    fn validate(&self, label: &str) -> Result<()> {
        let models = self.as_slice();
        if models.len() > MAX_WEIGHTED_VARIANTS_PER_SELECTOR {
            bail!(
                "blockstate {label} has {} weighted variants, exceeding the {MAX_WEIGHTED_VARIANTS_PER_SELECTOR} variant limit",
                models.len()
            );
        }
        // Vanilla `Weighted.java:20-28` / `Variant` weights are positive 32-bit signed integers
        // whose sum must fit in `i32` for `RandomSource.nextInt(int)`.
        let mut total_weight = 0_i32;
        for model in models {
            if model.weight <= 0 {
                bail!(
                    "blockstate {label} has non-positive variant weight {} (must be in 1..={})",
                    model.weight,
                    i32::MAX
                );
            }
            total_weight = total_weight.checked_add(model.weight).ok_or_else(|| {
                anyhow::anyhow!(
                    "blockstate {label} total variant weight overflows i32::MAX ({})",
                    i32::MAX
                )
            })?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RawMultipartCase {
    #[serde(default)]
    when: Option<serde_json::Value>,
    apply: RawBlockstateVariant,
}

impl RawMultipartCase {
    fn matches(&self, properties: &BTreeMap<String, String>) -> bool {
        self.when
            .as_ref()
            .map(|when| multipart_condition_matches(when, properties, 0))
            .unwrap_or(true)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RawModelVariant {
    pub(super) model: String,
    #[serde(default)]
    pub(super) x: i32,
    #[serde(default)]
    pub(super) y: i32,
    #[serde(default)]
    pub(super) z: i32,
    #[serde(default)]
    pub(super) uvlock: bool,
    #[serde(default = "default_model_weight")]
    weight: i32,
}

fn default_model_weight() -> i32 {
    1
}

/// Seed-independent model candidates of one block state. `combination` maps a position seed
/// to an index in `0..combination_count()`; `models` turns that index back into the selected
/// variant of every part.
#[derive(Debug, Clone)]
pub struct BlockStateModelChoices {
    parts: Vec<Vec<RawModelVariant>>,
    multipart: bool,
}

impl BlockStateModelChoices {
    fn new(parts: Vec<Vec<RawModelVariant>>, multipart: bool) -> Self {
        Self { parts, multipart }
    }

    pub fn combination_count(&self) -> usize {
        self.parts
            .iter()
            .filter(|part| !part.is_empty())
            .fold(1usize, |count, part| count.saturating_mul(part.len()))
    }

    pub fn combination(&self, seed: Option<i64>) -> usize {
        // Vanilla `MultiPartModel.java:49-54` feeds every selector the same `nextLong()` of the position seed.
        let seed = self.effective_seed(seed);
        let mut combination = 0usize;
        let mut stride = 1usize;
        for part in &self.parts {
            if part.len() <= 1 {
                continue;
            }
            let index = weighted_index(part, seed);
            let Some(term) = index.checked_mul(stride) else {
                break;
            };
            let Some(next_combination) = combination.checked_add(term) else {
                break;
            };
            combination = next_combination;
            let Some(next_stride) = stride.checked_mul(part.len()) else {
                break;
            };
            stride = next_stride;
        }
        combination
    }

    pub(super) fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    pub(super) fn models(&self, mut combination: usize) -> impl Iterator<Item = &RawModelVariant> {
        self.parts
            .iter()
            .filter(|part| !part.is_empty())
            .map(move |part| {
                let index = combination % part.len();
                combination /= part.len();
                &part[index]
            })
    }

    /// Directly selects one weighted variant per part from `seed` without mixed-radix
    /// packing, so multipart blockstates with many weighted parts cannot overflow `usize`.
    pub(super) fn models_for_seed(
        &self,
        seed: Option<i64>,
    ) -> impl Iterator<Item = &RawModelVariant> {
        let seed = self.effective_seed(seed);
        self.parts
            .iter()
            .filter(|part| !part.is_empty())
            .map(move |part| &part[weighted_index(part, seed)])
    }

    fn effective_seed(&self, seed: Option<i64>) -> Option<i64> {
        if self.multipart {
            seed.map(|seed| LegacyRandomSource::with_seed(seed).next_long())
        } else {
            seed
        }
    }
}

fn weighted_index(models: &[RawModelVariant], seed: Option<i64>) -> usize {
    if models.len() <= 1 {
        return 0;
    }
    let Some(seed) = seed else {
        return 0;
    };
    let Some(total_weight) = models
        .iter()
        .try_fold(0_i32, |total, model| {
            (model.weight > 0)
                .then(|| total.checked_add(model.weight))
                .flatten()
        })
        .filter(|total| *total > 0)
    else {
        return 0;
    };
    let mut selected = legacy_random_next_int(seed, total_weight);
    for (index, model) in models.iter().enumerate() {
        if selected < model.weight {
            return index;
        }
        selected -= model.weight;
    }
    models.len() - 1
}

fn legacy_random_next_int(seed: i64, bound: i32) -> i32 {
    if bound <= 1 {
        return 0;
    }
    let bound_u32 = bound as u32;
    let mut random = LegacyRandomSource::with_seed(seed);
    if bound_u32.is_power_of_two() {
        return ((u64::from(bound_u32) * u64::from(random.next_bits(31))) >> 31) as i32;
    }

    loop {
        let sample = random.next_bits(31) as i32;
        let modulo = sample % bound;
        let java_overflow_check = sample.wrapping_sub(modulo).wrapping_add(bound - 1);
        if java_overflow_check >= 0 {
            return modulo;
        }
    }
}

fn variant_key_match_score(key: &str, properties: &BTreeMap<String, String>) -> Option<usize> {
    if key.is_empty() {
        return Some(0);
    }

    let mut score = 0;
    for assignment in key.split(',') {
        let (name, value) = assignment.split_once('=')?;
        if properties.get(name)? != value {
            return None;
        }
        score += 1;
    }
    Some(score)
}

fn validate_multipart_condition_depth(condition: &serde_json::Value, depth: usize) -> Result<()> {
    if depth > MAX_MULTIPART_CONDITION_DEPTH {
        bail!(
            "blockstate multipart condition nesting exceeds {MAX_MULTIPART_CONDITION_DEPTH} levels"
        );
    }
    let Some(object) = condition.as_object() else {
        return Ok(());
    };
    for (key, value) in object {
        if matches!(key.as_str(), "OR" | "AND") {
            if let Some(items) = value.as_array() {
                for item in items {
                    validate_multipart_condition_depth(item, depth + 1)?;
                }
            }
        }
    }
    Ok(())
}

fn multipart_condition_matches(
    condition: &serde_json::Value,
    properties: &BTreeMap<String, String>,
    depth: usize,
) -> bool {
    if depth > MAX_MULTIPART_CONDITION_DEPTH {
        return false;
    }
    let Some(object) = condition.as_object() else {
        return false;
    };

    for (key, value) in object {
        match key.as_str() {
            "OR" => {
                let Some(items) = value.as_array() else {
                    return false;
                };
                if !items
                    .iter()
                    .any(|item| multipart_condition_matches(item, properties, depth + 1))
                {
                    return false;
                }
            }
            "AND" => {
                let Some(items) = value.as_array() else {
                    return false;
                };
                if !items
                    .iter()
                    .all(|item| multipart_condition_matches(item, properties, depth + 1))
                {
                    return false;
                }
            }
            property => {
                let Some(actual) = properties.get(property) else {
                    return false;
                };
                if !condition_value_matches(value, actual) {
                    return false;
                }
            }
        }
    }

    true
}

fn condition_value_matches(expected: &serde_json::Value, actual: &str) -> bool {
    match expected {
        serde_json::Value::String(value) => value.split('|').any(|candidate| candidate == actual),
        serde_json::Value::Bool(value) => actual == value.to_string(),
        serde_json::Value::Number(value) => actual == value.to_string(),
        _ => false,
    }
}
