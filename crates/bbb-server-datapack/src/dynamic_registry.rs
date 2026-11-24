//! The Java 26.1 data-driven registry surface loaded from datapacks.
//!
//! `RegistryDataLoader.WORLDGEN_REGISTRIES` (`RegistryDataLoader.java:83-129`) lists the
//! 45 registries loaded into the `WORLDGEN` registry layer in this exact order, and
//! `DIMENSION_REGISTRIES` (`RegistryDataLoader.java:130-132`) adds the LEVEL_STEM
//! registry (`minecraft:dimension`) loaded afterwards with the worldgen registries as
//! context (`WorldLoader.java:40-48`). Each registry's element directory is its registry
//! id path (`Registries.elementsDirPath`, `Registries.java:323-329`), e.g. the
//! `minecraft:worldgen/biome` registry loads from `worldgen/biome/`.
//!
//! D1 checks every element as well-formed JSON and retains both the exact winning source
//! bytes and a parsed compatibility view. Deep per-element codecs (chunk generators,
//! density functions, biome/structure payloads, the custom `Timeline.validateRegistry`)
//! consume that lossless source in the deep-codec phase. The
//! `SYNCHRONIZED_REGISTRIES` distinction (`RegistryDataLoader.java:133-162`) selects
//! network codecs for `ClientboundRegistryDataPacket` and is irrelevant until the
//! gameplay route needs registry synchronization.

use bbb_registry_codec::registry_codec::CanonicalRegistryCodecValue;

/// Validation a registry must pass after its elements are loaded
/// (`RegistryDataLoader.RegistryData::requiredValidator`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegistryValidator {
    None,
    /// `RegistryValidator.nonEmpty()`: the registry must contain at least one element.
    NonEmpty,
}

/// One entry of `RegistryDataLoader.RegistryData`: the registry id plus its validator.
/// The element directory is the registry id path (`Registries.elementsDirPath`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DynamicRegistrySpec {
    pub registry_id: &'static str,
    pub validator: RegistryValidator,
}

impl DynamicRegistrySpec {
    const fn new(registry_id: &'static str, validator: RegistryValidator) -> Self {
        Self {
            registry_id,
            validator,
        }
    }

    const fn plain(registry_id: &'static str) -> Self {
        Self::new(registry_id, RegistryValidator::None)
    }

    const fn non_empty(registry_id: &'static str) -> Self {
        Self::new(registry_id, RegistryValidator::NonEmpty)
    }

    /// `Registries.elementsDirPath`: the registry id path is the element directory.
    pub fn elements_dir(&self) -> &'static str {
        self.registry_id
            .strip_prefix("minecraft:")
            .unwrap_or(self.registry_id)
    }
}

/// `RegistryDataLoader.WORLDGEN_REGISTRIES` (`RegistryDataLoader.java:83-129`), in the
/// exact vanilla load order.
pub static WORLDGEN_REGISTRIES_26_1: &[DynamicRegistrySpec] = &[
    DynamicRegistrySpec::plain("minecraft:dimension_type"),
    DynamicRegistrySpec::plain("minecraft:worldgen/biome"),
    DynamicRegistrySpec::plain("minecraft:chat_type"),
    DynamicRegistrySpec::plain("minecraft:worldgen/configured_carver"),
    DynamicRegistrySpec::plain("minecraft:worldgen/configured_feature"),
    DynamicRegistrySpec::plain("minecraft:worldgen/placed_feature"),
    DynamicRegistrySpec::plain("minecraft:worldgen/structure"),
    DynamicRegistrySpec::plain("minecraft:worldgen/structure_set"),
    DynamicRegistrySpec::plain("minecraft:worldgen/processor_list"),
    DynamicRegistrySpec::plain("minecraft:worldgen/template_pool"),
    DynamicRegistrySpec::plain("minecraft:worldgen/noise_settings"),
    DynamicRegistrySpec::plain("minecraft:worldgen/noise"),
    DynamicRegistrySpec::plain("minecraft:worldgen/density_function"),
    DynamicRegistrySpec::plain("minecraft:worldgen/world_preset"),
    DynamicRegistrySpec::plain("minecraft:worldgen/flat_level_generator_preset"),
    DynamicRegistrySpec::plain("minecraft:trim_pattern"),
    DynamicRegistrySpec::plain("minecraft:trim_material"),
    DynamicRegistrySpec::plain("minecraft:trial_spawner"),
    DynamicRegistrySpec::non_empty("minecraft:wolf_variant"),
    DynamicRegistrySpec::non_empty("minecraft:wolf_sound_variant"),
    DynamicRegistrySpec::non_empty("minecraft:pig_variant"),
    DynamicRegistrySpec::non_empty("minecraft:pig_sound_variant"),
    DynamicRegistrySpec::non_empty("minecraft:frog_variant"),
    DynamicRegistrySpec::non_empty("minecraft:cat_variant"),
    DynamicRegistrySpec::non_empty("minecraft:cat_sound_variant"),
    DynamicRegistrySpec::non_empty("minecraft:cow_variant"),
    DynamicRegistrySpec::non_empty("minecraft:cow_sound_variant"),
    DynamicRegistrySpec::non_empty("minecraft:chicken_variant"),
    DynamicRegistrySpec::non_empty("minecraft:chicken_sound_variant"),
    DynamicRegistrySpec::non_empty("minecraft:zombie_nautilus_variant"),
    DynamicRegistrySpec::non_empty("minecraft:painting_variant"),
    DynamicRegistrySpec::plain("minecraft:damage_type"),
    DynamicRegistrySpec::plain("minecraft:worldgen/multi_noise_biome_source_parameter_list"),
    DynamicRegistrySpec::plain("minecraft:banner_pattern"),
    DynamicRegistrySpec::plain("minecraft:enchantment"),
    DynamicRegistrySpec::plain("minecraft:enchantment_provider"),
    DynamicRegistrySpec::plain("minecraft:jukebox_song"),
    DynamicRegistrySpec::plain("minecraft:instrument"),
    DynamicRegistrySpec::plain("minecraft:test_environment"),
    DynamicRegistrySpec::plain("minecraft:test_instance"),
    DynamicRegistrySpec::plain("minecraft:dialog"),
    DynamicRegistrySpec::plain("minecraft:world_clock"),
    DynamicRegistrySpec::plain("minecraft:timeline"),
    DynamicRegistrySpec::plain("minecraft:villager_trade"),
    DynamicRegistrySpec::plain("minecraft:trade_set"),
];

/// `RegistryDataLoader.DIMENSION_REGISTRIES` (`RegistryDataLoader.java:130-132`): the
/// LEVEL_STEM registry, loaded after the worldgen registries (`WorldLoader.java:47`).
pub static DIMENSION_REGISTRIES_26_1: &[DynamicRegistrySpec] =
    &[DynamicRegistrySpec::plain("minecraft:dimension")];

/// One loaded registry element attributed to the pack that won the overlay
/// merge.
///
/// `source_json` is the exact winning resource for persistence. `source` owns
/// the parsed source tree for registry codecs and gameplay readers. Typed
/// worldgen registries retain their decoded state instead of that tree.
#[derive(Debug, Clone, PartialEq)]
pub struct RawRegistryElement {
    pub id: String,
    pub source_pack: String,
    pub source_json: std::sync::Arc<[u8]>,
    pub source: Option<bbb_registry_codec::RegistryElementData>,
    /// Exact source-aware codec output when `load_registry` ran a complete owner.
    ///
    /// Non-synchronized registries and invalid projections leave this empty.
    /// Freeze validates the retained references before publication writes the NBT.
    pub codec_value: Option<CanonicalRegistryCodecValue>,
}

impl RawRegistryElement {
    pub fn json(&self) -> Option<&bbb_registry_codec::RegistryJsonValue> {
        self.source.as_ref()?.json()
    }

    pub fn decoded_direct_source(
        &self,
    ) -> Result<
        std::borrow::Cow<'_, bbb_registry_codec::RegistryElementData>,
        bbb_registry_codec::RegistryElementDecodeError,
    > {
        match &self.source {
            Some(source) => Ok(std::borrow::Cow::Borrowed(source)),
            None => bbb_registry_codec::decode_direct_registry_json(&self.source_json)
                .map(std::borrow::Cow::Owned),
        }
    }

    pub fn decoded_loaded_source(
        &self,
        max_bytes: usize,
    ) -> Result<
        std::borrow::Cow<'_, bbb_registry_codec::RegistryElementData>,
        bbb_registry_codec::RegistryElementDecodeError,
    > {
        match &self.source {
            Some(source) => Ok(std::borrow::Cow::Borrowed(source)),
            None => bbb_registry_codec::decode_loaded_registry_json(&self.source_json, max_bytes)
                .map(std::borrow::Cow::Owned),
        }
    }
}

/// Whether gameplay readers borrow `RawRegistryElement::source` directly at runtime.
pub fn retains_runtime_source_ast(registry_id: &str) -> bool {
    matches!(
        registry_id,
        "minecraft:enchantment"
            | "minecraft:enchantment_provider"
            | "minecraft:trial_spawner"
            | "minecraft:trade_set"
            | "minecraft:cat_variant"
            | "minecraft:chicken_variant"
            | "minecraft:cow_variant"
            | "minecraft:frog_variant"
            | "minecraft:pig_variant"
            | "minecraft:wolf_variant"
            | "minecraft:zombie_nautilus_variant"
    )
}

/// Vanilla `Identifier.compareTo` registration key: resource path first, then namespace.
pub(crate) fn registration_key(resource_id: &str) -> (&str, &str) {
    let (namespace, path) = resource_id
        .split_once(':')
        .unwrap_or(("minecraft", resource_id));
    (path, namespace)
}

/// One fully loaded dynamic registry; elements are in Vanilla registration order:
/// resource path first, then namespace (`Identifier.compareTo`).
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedDynamicRegistry {
    pub registry_id: String,
    pub elements: Vec<RawRegistryElement>,
}

impl LoadedDynamicRegistry {
    /// Looks up an element's registration index in O(log N) when elements are in
    /// Vanilla registration order, scanning linearly when a registry uses custom
    /// element order.
    pub fn element_index(&self, element_id: &str) -> Option<usize> {
        if self.elements.len() > 16 {
            let target = registration_key(element_id);
            if let Ok(index) = self
                .elements
                .binary_search_by(|element| registration_key(&element.id).cmp(&target))
            {
                if self.elements[index].id == element_id {
                    return Some(index);
                }
            }
        }
        self.elements
            .iter()
            .position(|element| element.id == element_id)
    }

    pub fn element(&self, element_id: &str) -> Option<&RawRegistryElement> {
        self.element_index(element_id)
            .map(|index| &self.elements[index])
    }
}

/// Shared copy-on-write handle around [`DatapackGeneration`] so `/reload` and
/// clones share immutable dynamic registries without deep-copying all 46
/// registries while preserving direct `&DatapackGeneration` / `&mut` access.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SharedDatapackGeneration(std::sync::Arc<DatapackGeneration>);

impl SharedDatapackGeneration {
    pub fn new(generation: DatapackGeneration) -> Self {
        Self(std::sync::Arc::new(generation))
    }

    pub fn from_arc(generation: std::sync::Arc<DatapackGeneration>) -> Self {
        Self(generation)
    }

    pub fn as_arc(&self) -> &std::sync::Arc<DatapackGeneration> {
        &self.0
    }

    pub fn same_frozen_registries(&self, other: &DatapackGeneration) -> bool {
        std::ptr::eq(std::sync::Arc::as_ptr(&self.0), other) || self.0.same_frozen_registries(other)
    }
}

impl From<DatapackGeneration> for SharedDatapackGeneration {
    fn from(generation: DatapackGeneration) -> Self {
        Self::new(generation)
    }
}

impl std::ops::Deref for SharedDatapackGeneration {
    type Target = DatapackGeneration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SharedDatapackGeneration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        std::sync::Arc::make_mut(&mut self.0)
    }
}

/// The immutable datapack registry generation: every declared registry in vanilla
/// load order (worldgen registries first, then the dimension registry).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DatapackGeneration {
    pub registries: Vec<LoadedDynamicRegistry>,
    pub worldgen: crate::worldgen::WorldgenRegistries,
}

impl DatapackGeneration {
    pub fn registry(&self, registry_id: &str) -> Option<&LoadedDynamicRegistry> {
        self.registries
            .iter()
            .find(|registry| registry.registry_id == registry_id)
    }

    pub fn element(&self, registry_id: &str, element_id: &str) -> Option<&RawRegistryElement> {
        self.registry(registry_id)?.element(element_id)
    }

    pub fn trim_startup_sources(&mut self) {
        for registry in &mut self.registries {
            if !retains_runtime_source_ast(&registry.registry_id) {
                for element in &mut registry.elements {
                    element.source = None;
                }
            }
        }
    }

    pub fn same_frozen_registries(&self, other: &Self) -> bool {
        self.registries.len() == other.registries.len()
            && self
                .registries
                .iter()
                .zip(&other.registries)
                .all(|(left, right)| {
                    left.registry_id == right.registry_id
                        && left.elements.len() == right.elements.len()
                        && left.elements.iter().zip(&right.elements).all(|(a, b)| {
                            a.id == b.id
                                && a.source_pack == b.source_pack
                                && (std::sync::Arc::ptr_eq(&a.source_json, &b.source_json)
                                    || a.source_json == b.source_json)
                                && a.codec_value == b.codec_value
                        })
                })
            && self.worldgen.same_frozen_registries(&other.worldgen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worldgen_registry_table_matches_vanilla_count_and_dirs() {
        assert_eq!(WORLDGEN_REGISTRIES_26_1.len(), 45);
        assert_eq!(DIMENSION_REGISTRIES_26_1.len(), 1);
        assert_eq!(WORLDGEN_REGISTRIES_26_1[0].elements_dir(), "dimension_type");
        assert_eq!(WORLDGEN_REGISTRIES_26_1[1].elements_dir(), "worldgen/biome");
        assert_eq!(DIMENSION_REGISTRIES_26_1[0].elements_dir(), "dimension");
        // Every spec keeps the `minecraft:` namespace so the dir derivation holds.
        for spec in WORLDGEN_REGISTRIES_26_1
            .iter()
            .chain(DIMENSION_REGISTRIES_26_1.iter())
        {
            assert!(spec.registry_id.starts_with("minecraft:"), "{spec:?}");
            assert!(!spec.elements_dir().is_empty());
        }
    }

    #[test]
    fn non_empty_validators_match_vanilla_marks() {
        let non_empty: Vec<&str> = WORLDGEN_REGISTRIES_26_1
            .iter()
            .filter(|spec| spec.validator == RegistryValidator::NonEmpty)
            .map(|spec| spec.registry_id)
            .collect();
        assert_eq!(
            non_empty,
            [
                "minecraft:wolf_variant",
                "minecraft:wolf_sound_variant",
                "minecraft:pig_variant",
                "minecraft:pig_sound_variant",
                "minecraft:frog_variant",
                "minecraft:cat_variant",
                "minecraft:cat_sound_variant",
                "minecraft:cow_variant",
                "minecraft:cow_sound_variant",
                "minecraft:chicken_variant",
                "minecraft:chicken_sound_variant",
                "minecraft:zombie_nautilus_variant",
                "minecraft:painting_variant",
            ]
        );
    }

    #[test]
    fn element_index_binary_search_requires_exact_canonical_id() {
        let mut ids: Vec<String> = (0..18).map(|i| format!("minecraft:biome_{i:02}")).collect();
        ids.push("minecraft:plains".to_string());
        ids.push("custom:plains".to_string());
        ids.sort_by(|a, b| registration_key(a).cmp(&registration_key(b)));

        let registry = LoadedDynamicRegistry {
            registry_id: "minecraft:worldgen/biome".to_string(),
            elements: ids
                .into_iter()
                .map(|id| RawRegistryElement {
                    id,
                    source_pack: "vanilla".to_string(),
                    source_json: std::sync::Arc::from(&b"{}"[..]),
                    source: None,
                    codec_value: None,
                })
                .collect(),
        };

        let plains_idx = registry
            .element_index("minecraft:plains")
            .expect("canonical minecraft id should resolve");
        assert_eq!(registry.elements[plains_idx].id, "minecraft:plains");

        let custom_idx = registry
            .element_index("custom:plains")
            .expect("custom namespace id should resolve");
        assert_eq!(registry.elements[custom_idx].id, "custom:plains");

        assert_eq!(
            registry.element_index("plains"),
            None,
            "unprefixed id must not match minecraft:plains"
        );
        assert_eq!(registry.element_index("minecraft:missing"), None);
    }
}
