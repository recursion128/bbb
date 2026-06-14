use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawBlockstate {
    #[serde(default)]
    variants: BTreeMap<String, RawBlockstateVariant>,
    #[serde(default)]
    multipart: Vec<RawMultipartCase>,
}

impl RawBlockstate {
    pub(super) fn select_variants(
        &self,
        properties: &BTreeMap<String, String>,
        seed: Option<i64>,
    ) -> Option<RawBlockstateSelection> {
        let mut best_variant = None;
        let mut best_score = 0usize;
        for (key, variant) in &self.variants {
            let Some(score) = variant_key_match_score(key, properties) else {
                continue;
            };
            if best_variant.is_none() || score >= best_score {
                best_score = score;
                best_variant = variant.select_model(seed);
            }
        }
        if best_variant.is_some() {
            return best_variant.map(|variant| RawBlockstateSelection::Variants(vec![variant]));
        }

        if self.multipart.is_empty() {
            return None;
        }

        let multipart_seed = seed.map(legacy_random_next_long);
        let variants: Vec<_> = self
            .multipart
            .iter()
            .filter(|case| case.matches(properties))
            .filter_map(|case| case.apply.select_model(multipart_seed))
            .collect();
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

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RawBlockstateVariant {
    One(RawModelVariant),
    Many(Vec<RawModelVariant>),
}

impl RawBlockstateVariant {
    fn select_model(&self, seed: Option<i64>) -> Option<RawModelVariant> {
        match self {
            Self::One(model) => Some(model.clone()),
            Self::Many(models) => select_weighted_model(models, seed),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawMultipartCase {
    #[serde(default)]
    when: Option<serde_json::Value>,
    apply: RawBlockstateVariant,
}

impl RawMultipartCase {
    fn matches(&self, properties: &BTreeMap<String, String>) -> bool {
        self.when
            .as_ref()
            .map(|when| multipart_condition_matches(when, properties))
            .unwrap_or(true)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawModelVariant {
    pub(super) model: String,
    #[serde(default)]
    pub(super) x: i32,
    #[serde(default)]
    pub(super) y: i32,
    #[serde(default)]
    pub(super) uvlock: bool,
    #[serde(default = "default_model_weight")]
    weight: u32,
}

fn default_model_weight() -> u32 {
    1
}

fn select_weighted_model(models: &[RawModelVariant], seed: Option<i64>) -> Option<RawModelVariant> {
    let Some(seed) = seed else {
        return models.first().cloned();
    };
    let total_weight = models
        .iter()
        .try_fold(0u32, |total, model| total.checked_add(model.weight))?;
    if total_weight == 0 {
        return models.first().cloned();
    }
    let mut selected = legacy_random_next_int(seed, total_weight);
    for model in models {
        if selected < model.weight {
            return Some(model.clone());
        }
        selected -= model.weight;
    }
    models.last().cloned()
}

fn legacy_random_next_int(seed: i64, bound: u32) -> u32 {
    debug_assert!(bound > 0);
    let mut random = LegacyRandom::new(seed);
    if bound.is_power_of_two() {
        return ((u64::from(bound) * u64::from(random.next_bits(31))) >> 31) as u32;
    }

    loop {
        let sample = random.next_bits(31) as u32;
        let modulo = sample % bound;
        let java_overflow_check = (sample as i32)
            .wrapping_sub(modulo as i32)
            .wrapping_add((bound - 1) as i32);
        if java_overflow_check >= 0 {
            return modulo;
        }
    }
}

fn legacy_random_next_long(seed: i64) -> i64 {
    let mut random = LegacyRandom::new(seed);
    let upper = random.next_bits(32) as i32 as i64;
    let lower = random.next_bits(32) as i32 as i64;
    (upper << 32).wrapping_add(lower)
}

struct LegacyRandom {
    seed: u64,
}

impl LegacyRandom {
    const MULTIPLIER: u64 = 25_214_903_917;
    const INCREMENT: u64 = 11;
    const MODULUS_MASK: u64 = (1 << 48) - 1;

    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ Self::MULTIPLIER) & Self::MODULUS_MASK,
        }
    }

    fn next_bits(&mut self, bits: u32) -> u64 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT)
            & Self::MODULUS_MASK;
        self.seed >> (48 - bits)
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

fn multipart_condition_matches(
    condition: &serde_json::Value,
    properties: &BTreeMap<String, String>,
) -> bool {
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
                    .any(|item| multipart_condition_matches(item, properties))
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
                    .all(|item| multipart_condition_matches(item, properties))
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
