use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use serde::Deserialize;

use crate::{registry_data_types::VANILLA_BUILT_IN_REGISTRY_TYPES, MC_VERSION};

const ARTIFACT: &str = include_str!("../data/builtin_registration_tags_26_1.json");
const SOURCE: &str =
    "Bootstrap.bootStrap BuiltInRegistries.REGISTRY.getTags before VanillaRegistries.createLookup";
const EXPECTED_TAG_COUNT: usize = 35;
static CATALOG: OnceLock<Result<VanillaBootstrapRegistrationTags, BootstrapRegistrationTagsError>> =
    OnceLock::new();

#[derive(Debug)]
pub struct VanillaBootstrapRegistrationTags {
    registries: BTreeMap<String, BTreeSet<String>>,
}

impl VanillaBootstrapRegistrationTags {
    pub fn contains(&self, registry: &str, tag: &str) -> bool {
        self.registries
            .get(registry)
            .is_some_and(|tags| tags.contains(tag))
    }

    pub fn registries(&self) -> &BTreeMap<String, BTreeSet<String>> {
        &self.registries
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapRegistrationTagsError(String);

impl std::fmt::Display for BootstrapRegistrationTagsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid bootstrap registration tags: {}", self.0)
    }
}

impl std::error::Error for BootstrapRegistrationTagsError {}

pub fn vanilla_bootstrap_registration_tags(
) -> Result<&'static VanillaBootstrapRegistrationTags, BootstrapRegistrationTagsError> {
    CATALOG
        .get_or_init(|| parse(ARTIFACT))
        .as_ref()
        .map_err(Clone::clone)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    vanilla_version: String,
    source: String,
    registry_count: usize,
    tag_count: usize,
    registries: Vec<RegistryRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryRow {
    resource_id: String,
    tags: Vec<String>,
}

fn resource_id(value: &str) -> bool {
    let Some((namespace, path)) = value.split_once(':') else {
        return false;
    };
    !namespace.is_empty()
        && !path.is_empty()
        && namespace
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        && path
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
}

fn parse(json: &str) -> Result<VanillaBootstrapRegistrationTags, BootstrapRegistrationTagsError> {
    let invalid = |message: &str| BootstrapRegistrationTagsError(message.into());
    let artifact: Artifact = serde_json::from_str(json)
        .map_err(|error| BootstrapRegistrationTagsError(error.to_string()))?;
    if artifact.vanilla_version != MC_VERSION {
        return Err(invalid("version mismatch"));
    }
    if artifact.source != SOURCE {
        return Err(invalid("source mismatch"));
    }
    let expected: BTreeSet<_> = VANILLA_BUILT_IN_REGISTRY_TYPES
        .iter()
        .map(|row| row.resource_id)
        .collect();
    if artifact.registry_count != expected.len() || artifact.registries.len() != expected.len() {
        return Err(invalid("registry count mismatch"));
    }
    if artifact.tag_count != EXPECTED_TAG_COUNT {
        return Err(invalid("tag count mismatch"));
    }
    let mut registries = BTreeMap::new();
    let mut previous = None;
    let mut tag_count = 0usize;
    for row in artifact.registries {
        if !expected.contains(row.resource_id.as_str()) {
            return Err(invalid("unknown registry"));
        }
        if previous
            .as_ref()
            .is_some_and(|value: &String| value >= &row.resource_id)
        {
            return Err(invalid("duplicate or unordered registry"));
        }
        previous = Some(row.resource_id.clone());
        if row.tags.iter().any(|tag| !resource_id(tag)) {
            return Err(invalid("invalid tag identifier"));
        }
        if row.tags.windows(2).any(|tags| tags[0] >= tags[1]) {
            return Err(invalid("duplicate or unordered tag"));
        }
        tag_count = tag_count
            .checked_add(row.tags.len())
            .ok_or_else(|| invalid("tag count overflow"))?;
        registries.insert(row.resource_id, row.tags.into_iter().collect());
    }
    if tag_count != artifact.tag_count {
        return Err(invalid("tag count mismatch"));
    }
    Ok(VanillaBootstrapRegistrationTags { registries })
}

#[cfg(test)]
mod tests;
