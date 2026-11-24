//! The `minecraft:enchantment` registry as `/enchant` reads it.
//!
//! `Enchantment.getMaxLevel` (`Enchantment.java:150-152`), `canEnchant`
//! (`:186-188`) and `areCompatible` (`:167-168`) all read the loaded document
//! plus the two holder sets it names. This crate owns the datapack generation,
//! so the facts live here rather than as a table written beside the command.

use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use bbb_protocol::nbt_summary::value_field as field;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::RegistryElementValueRef as V;

use crate::startup::DatapackStartup;
use crate::vanilla_snapshot::VanillaDatapackSnapshot;

/// `Registries.ENCHANTMENT` (`EnchantCommand.java:46`).
pub const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";

/// `EnchantmentTags.CURSE` (`Enchantment.java:173`).
const CURSE_TAG: &str = "minecraft:curse";

/// One loaded enchantment as `/enchant` consults it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnchantmentFacts {
    /// The registry id `ResourceArgument` resolved.
    pub resource_id: String,
    /// The synced registry index `ItemEnchantments` stores as `holder_id`.
    pub registry_id: i32,
    /// `Enchantment.definition.maxLevel()` (`Enchantment.java:150-152`).
    pub max_level: i32,
    /// `Enchantment.description` (`Enchantment.java:60`), copied by `getFullname` (`:171-172`).
    pub description: NbtSummaryValue,
    /// `enchantment.is(EnchantmentTags.CURSE)` (`Enchantment.java:173`).
    pub is_curse: bool,
    /// Flattened `definition.supportedItems()` (`Enchantment.java:186-188`).
    pub supported_items: BTreeSet<String>,
    /// Flattened `exclusiveSet` (`Enchantment.java:167-168`).
    pub exclusive_set: BTreeSet<String>,
}

impl EnchantmentFacts {
    /// `Enchantment.canEnchant` (`Enchantment.java:186-188`).
    #[must_use]
    pub fn can_enchant(&self, item_resource_id: &str) -> bool {
        self.supported_items.contains(item_resource_id)
    }

    /// `Enchantment.areCompatible` (`Enchantment.java:167-168`).
    #[must_use]
    pub fn are_compatible(&self, other: &Self) -> bool {
        self.resource_id != other.resource_id
            && !self.exclusive_set.contains(&other.resource_id)
            && !other.exclusive_set.contains(&self.resource_id)
    }
}

/// The facts of the whole `minecraft:enchantment` registry, in
/// `RegistryDataLoader` order (`RegistryDataLoader.java:83-129`).
fn enchantment_registry<'a>(
    elements: impl Iterator<Item = (&'a str, &'a NbtSummaryValue)>,
    tag_members: impl Fn(&str, &str) -> Vec<String>,
) -> Vec<EnchantmentFacts> {
    let curse: BTreeSet<String> = tag_members(ENCHANTMENT_REGISTRY, CURSE_TAG)
        .into_iter()
        .collect();
    elements
        .enumerate()
        .map(|(index, (resource_id, network_nbt))| {
            let description = field(network_nbt, "description")
                .expect("validated enchantments have a description")
                .clone();
            let value = V::Nbt(network_nbt);
            let max_level = value.field("max_level").and_then(V::codec_i32).unwrap_or(1);
            EnchantmentFacts {
                resource_id: resource_id.to_string(),
                registry_id: i32::try_from(index).expect("a registry is shorter than i32::MAX"),
                max_level,
                description,
                is_curse: curse.contains(resource_id),
                supported_items: holder_set_ids(
                    value.field("supported_items"),
                    "minecraft:item",
                    &tag_members,
                ),
                exclusive_set: holder_set_ids(
                    value.field("exclusive_set"),
                    ENCHANTMENT_REGISTRY,
                    &tag_members,
                ),
            }
        })
        .collect()
}

/// The live generation when the runtime wired one, otherwise the built-in pack.
pub fn load_enchantment_facts(datapacks: Option<&DatapackStartup>) -> Arc<Vec<EnchantmentFacts>> {
    let loaded = datapacks
        .and_then(|datapacks| {
            let registry = datapacks.registries.registry(ENCHANTMENT_REGISTRY)?;
            let elements = registry.elements.iter().map(|element| {
                (
                    element.id.as_str(),
                    &element
                        .codec_value
                        .as_ref()
                        .expect("enchantment native codec is retained")
                        .network_nbt,
                )
            });
            Some(enchantment_registry(elements, |registry_id, tag_id| {
                datapacks
                    .tags
                    .registry(registry_id)
                    .and_then(|tags| tags.tag(tag_id))
                    .map(<[String]>::to_vec)
                    .unwrap_or_default()
            }))
        })
        .unwrap_or_else(|| builtin_enchantment_registry().to_vec());
    Arc::new(loaded)
}

/// The `minecraft:enchantment` registry of the built-in datapack alone.
#[must_use]
pub fn builtin_enchantment_registry() -> &'static [EnchantmentFacts] {
    static BUILTIN: OnceLock<Vec<EnchantmentFacts>> = OnceLock::new();
    BUILTIN.get_or_init(load_builtin_enchantment_registry)
}

fn load_builtin_enchantment_registry() -> Vec<EnchantmentFacts> {
    let snapshot =
        VanillaDatapackSnapshot::embedded_shared().expect("the embedded built-in datapack decodes");
    let mut paths: Vec<&str> = snapshot
        .paths()
        .filter(|path| path.starts_with("enchantment/") && path.ends_with(".json"))
        .collect();
    paths.sort_unstable();
    let elements: Vec<(String, NbtSummaryValue)> = paths
        .into_iter()
        .map(|path| {
            let name = path
                .trim_start_matches("enchantment/")
                .trim_end_matches(".json");
            let bytes = snapshot
                .read_path(path)
                .expect("the built-in enchantment exists");
            let source = bbb_registry_codec::decode_direct_registry_json(bytes)
                .expect("the built-in enchantment source decodes");
            let network_nbt = bbb_registry_codec::decode_exact_synchronized_registry_element(
                ENCHANTMENT_REGISTRY,
                &source,
            )
            .expect("the built-in enchantment validates")
            .projected_network_nbt()
            .expect("the built-in enchantment projects");
            (format!("minecraft:{name}"), network_nbt)
        })
        .collect();
    enchantment_registry(
        elements.iter().map(|(id, nbt)| (id.as_str(), nbt)),
        builtin_tag_members,
    )
}

fn builtin_tag_members(registry_id: &str, tag_id: &str) -> Vec<String> {
    match bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1(
        registry_id,
        tag_id,
    ) {
        bbb_protocol::PersistentRegistryTagMembers::Present(members) => members,
        bbb_protocol::PersistentRegistryTagMembers::Absent
        | bbb_protocol::PersistentRegistryTagMembers::Unavailable => Vec::new(),
    }
}

fn holder_set_ids(
    value: Option<V<'_>>,
    registry: &str,
    tag_members: &impl Fn(&str, &str) -> Vec<String>,
) -> BTreeSet<String> {
    let Some(value) = value else {
        return BTreeSet::new();
    };
    if let Some(reference) = value.utf8_string() {
        if let Some(tag) = reference.strip_prefix('#') {
            let Ok(tag) = bbb_protocol::normalize_resource_location(tag.to_string()) else {
                return BTreeSet::new();
            };
            tag_members(registry, &tag).into_iter().collect()
        } else {
            bbb_protocol::normalize_resource_location(reference)
                .ok()
                .into_iter()
                .collect()
        }
    } else {
        (0..value.list_len().unwrap_or_default())
            .flat_map(|index| holder_set_ids(value.list_element(index), registry, tag_members))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(id: &str) -> &'static EnchantmentFacts {
        builtin_enchantment_registry()
            .iter()
            .find(|entry| entry.resource_id == id)
            .unwrap_or_else(|| panic!("{id} is in the built-in pack"))
    }

    #[test]
    fn sharpness_matches_the_built_in_document() {
        let sharpness = facts("minecraft:sharpness");
        assert_eq!(sharpness.max_level, 5);
        assert!(!sharpness.is_curse);
        assert!(sharpness.can_enchant("minecraft:diamond_sword"));
        assert!(!sharpness.can_enchant("minecraft:dirt"));
        assert!(!sharpness.are_compatible(facts("minecraft:smite")));
        assert!(!sharpness.are_compatible(sharpness));
        assert!(sharpness.are_compatible(facts("minecraft:unbreaking")));
    }

    #[test]
    fn binding_curse_is_the_curse_tag_and_max_level_one() {
        let curse = facts("minecraft:binding_curse");
        assert_eq!(curse.max_level, 1);
        assert!(curse.is_curse);
        assert_eq!(
            field(&curse.description, "translate"),
            Some(&NbtSummaryValue::String(
                "enchantment.minecraft.binding_curse".into()
            ))
        );
    }
}
