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
    ) -> Option<Vec<RawModelVariant>> {
        let mut best_variant = None;
        let mut best_score = 0usize;
        for (key, variant) in &self.variants {
            let Some(score) = variant_key_match_score(key, properties) else {
                continue;
            };
            if best_variant.is_none() || score >= best_score {
                best_score = score;
                best_variant = variant.first_model();
            }
        }
        if best_variant.is_some() {
            return best_variant.map(|variant| vec![variant]);
        }

        let variants: Vec<_> = self
            .multipart
            .iter()
            .filter(|case| case.matches(properties))
            .filter_map(|case| case.apply.first_model())
            .collect();
        (!variants.is_empty()).then_some(variants)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RawBlockstateVariant {
    One(RawModelVariant),
    Many(Vec<RawModelVariant>),
}

impl RawBlockstateVariant {
    fn first_model(&self) -> Option<RawModelVariant> {
        match self {
            Self::One(model) => Some(model.clone()),
            Self::Many(models) => models.first().cloned(),
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
