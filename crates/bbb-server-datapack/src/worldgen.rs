//! Typed worldgen configuration decoded once from datapack JSON.
//!
//! Configured features, placed features, processor lists and configured
//! carvers are decoded when the datapack loads; illegal content is rejected
//! there and generation reads the typed values. Runtime behaviour lives in
//! `bbb-server-worldgen`.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, OnceLock};

use bbb_registry_codec::registry_codec::RegistryReferenceKind;
use bbb_registry_codec::RegistryJsonValue as Json;

pub mod biome;
pub mod block;
pub mod carver;
pub mod density;
pub mod feature;
pub mod json;
pub mod noise;
pub mod placement;
pub mod processor;
pub mod structure;
pub mod surface;
pub mod tree;
pub mod value;

pub use block::{BlockPredicate, BlockStateProvider, HolderSet, NoiseParams, Offset, RuleTest};
pub use carver::{
    biome_inline_generation_refs, BiomeCarverHolder, BiomeCarvers, CanyonShape,
    CarverDebugSettings, CarverKind, ConfiguredCarver, CONFIGURED_CARVER_REGISTRY,
};
pub use feature::*;
pub use json::DecodeError;
pub use placement::{
    ConfiguredFeatureHolder, Heightmap, Holder, PlacedFeature, PlacedFeatureHolder,
    PlacementModifier, CONFIGURED_FEATURE_REGISTRY, PLACED_FEATURE_REGISTRY,
};
pub use processor::{ProcessorList, ProcessorListHolder, PROCESSOR_LIST_REGISTRY};
pub use value::{FloatProvider, HeightProvider, IntProvider, VerticalAnchor, WeightedList};

/// Named holders and tags a decoded value points at, checked against the
/// frozen registries once every registry has loaded.
#[derive(Default)]
pub struct Refs {
    pub items: Vec<(&'static str, RegistryReferenceKind, String)>,
}

impl Refs {
    pub fn element(&mut self, registry: &'static str, id: &str) {
        self.items
            .push((registry, RegistryReferenceKind::Element, id.to_string()));
    }

    pub fn tag(&mut self, registry: &'static str, id: &str) {
        self.items
            .push((registry, RegistryReferenceKind::Tag, id.to_string()));
    }
}

/// A lazily built runtime companion of a decoded value (for example the
/// noise a provider samples), owned by the value so it is built once.
#[derive(Default)]
pub struct Compiled(OnceLock<Box<dyn Any + Send + Sync>>);

impl Compiled {
    pub fn get_or_init<T: Any + Send + Sync>(&self, init: impl FnOnce() -> T) -> &T {
        self.0
            .get_or_init(|| Box::new(init()))
            .downcast_ref()
            .expect("one compiled type per slot")
    }
}

impl Clone for Compiled {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for Compiled {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl fmt::Debug for Compiled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Compiled")
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiomeClimate {
    pub has_precipitation: bool,
    pub temperature: f32,
    pub modifier: bbb_registry_codec::biome_climate::TemperatureModifier,
}

impl BiomeClimate {
    pub(crate) fn decode(value: &bbb_registry_codec::RegistryJsonValue) -> Result<Self, String> {
        use bbb_registry_codec::biome_climate::TemperatureModifier;

        let fields = value;
        fields.as_object().ok_or("biome value is not an object")?;
        let has_precipitation = fields
            .get("has_precipitation")
            .and_then(bbb_registry_codec::RegistryJsonValue::as_bool)
            .ok_or("has_precipitation is not a boolean")?;
        let temperature = fields
            .get("temperature")
            .and_then(bbb_registry_codec::RegistryJsonValue::as_f64)
            .map(|value| value as f32)
            .filter(|value| value.is_finite())
            .ok_or("temperature is not a finite float")?;
        let modifier = match fields
            .get("temperature_modifier")
            .and_then(bbb_registry_codec::RegistryJsonValue::utf8_string)
            .as_deref()
            .unwrap_or("none")
        {
            "none" => TemperatureModifier::None,
            "frozen" => TemperatureModifier::Frozen,
            value => return Err(format!("unknown temperature_modifier {value}")),
        };
        Ok(Self {
            has_precipitation,
            temperature,
            modifier,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SourceStamped<T> {
    pub(crate) source_json: Arc<[u8]>,
    pub(crate) value: T,
}

impl<T: PartialEq> PartialEq for SourceStamped<T> {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.source_json, &other.source_json)
            || self.source_json == other.source_json)
            && self.value == other.value
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CachedStructurePools {
    pub(crate) sources: Vec<(String, Arc<[u8]>)>,
    pub(crate) pools: Result<
        Arc<crate::structure_pool::StructurePoolSet>,
        crate::structure_pool::StructurePoolError,
    >,
}

impl PartialEq for CachedStructurePools {
    fn eq(&self, other: &Self) -> bool {
        self.pools == other.pools
    }
}

/// The typed elements of the worldgen registries generation reads by id.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorldgenRegistries {
    /// Climate values in the frozen biome registry's numeric ID order.
    pub biome_climates: Vec<BiomeClimate>,
    pub biome_features: HashMap<String, biome::BiomeFeatures>,
    pub biome_bonemeal_features: HashMap<String, Arc<[Arc<ConfiguredFeature>]>>,
    pub density_functions: HashMap<String, Arc<density::DensityFunction>>,
    pub noises: HashMap<String, Arc<noise::NoiseParameters>>,
    pub noise_settings: HashMap<String, Arc<noise::NoiseGeneratorSettings>>,
    pub configured_features: HashMap<String, Arc<ConfiguredFeature>>,
    pub placed_features: HashMap<String, Arc<PlacedFeature>>,
    pub processor_lists: HashMap<String, Arc<ProcessorList>>,
    pub configured_carvers: HashMap<String, Arc<ConfiguredCarver>>,
    pub(crate) biome_carvers:
        HashMap<String, SourceStamped<Result<Option<carver::BiomeCarvers>, DecodeError>>>,
    pub(crate) biome_mob_spawns: HashMap<
        String,
        SourceStamped<
            Result<
                crate::mob_spawn::RawBiomeMobSpawnSettings,
                crate::mob_spawn::MobSpawnCatalogError,
            >,
        >,
    >,
    pub(crate) structure_mob_spawns: HashMap<
        String,
        SourceStamped<
            Result<
                crate::mob_spawn::RawStructureSpawnOverrides,
                crate::mob_spawn::MobSpawnCatalogError,
            >,
        >,
    >,
    pub(crate) multi_noise_parameter_lists: HashMap<
        String,
        SourceStamped<Result<structure::DecodedMultiNoiseParameterList, DecodeError>>,
    >,
    pub(crate) structures:
        HashMap<String, SourceStamped<Result<Arc<structure::DecodedStructure>, DecodeError>>>,
    pub(crate) structure_sets:
        HashMap<String, SourceStamped<Result<Arc<structure::DecodedStructureSet>, DecodeError>>>,
    pub(crate) structure_pools: Option<CachedStructurePools>,
}

/// One decoded element of a typed worldgen registry.
pub enum WorldgenElement {
    DensityFunction(Arc<density::DensityFunction>),
    Noise(Arc<noise::NoiseParameters>),
    NoiseSettings(Arc<noise::NoiseGeneratorSettings>),
    ConfiguredFeature(Arc<ConfiguredFeature>),
    PlacedFeature(Arc<PlacedFeature>),
    ProcessorList(Arc<ProcessorList>),
    ConfiguredCarver(Arc<ConfiguredCarver>),
}

impl WorldgenElement {
    pub fn is_typed_registry(registry_id: &str) -> bool {
        matches!(
            registry_id,
            density::DENSITY_FUNCTION_REGISTRY
                | noise::NOISE_REGISTRY
                | noise::NOISE_SETTINGS_REGISTRY
                | CONFIGURED_FEATURE_REGISTRY
                | PLACED_FEATURE_REGISTRY
                | PROCESSOR_LIST_REGISTRY
                | CONFIGURED_CARVER_REGISTRY
        )
    }

    /// Decodes `json` when `registry_id` is a typed worldgen registry.
    pub fn decode(registry_id: &str, json: &Json) -> Option<json::Result<Self>> {
        Some(match registry_id {
            density::DENSITY_FUNCTION_REGISTRY => density::DensityFunction::decode(json)
                .map(|value| Self::DensityFunction(Arc::new(value))),
            noise::NOISE_REGISTRY => {
                noise::NoiseParameters::decode(json).map(|value| Self::Noise(Arc::new(value)))
            }
            noise::NOISE_SETTINGS_REGISTRY => noise::NoiseGeneratorSettings::decode(json)
                .map(|value| Self::NoiseSettings(Arc::new(value))),
            CONFIGURED_FEATURE_REGISTRY => ConfiguredFeature::decode(json)
                .map(|value| Self::ConfiguredFeature(Arc::new(value))),
            PLACED_FEATURE_REGISTRY => {
                PlacedFeature::decode(json).map(|value| Self::PlacedFeature(Arc::new(value)))
            }
            PROCESSOR_LIST_REGISTRY => {
                ProcessorList::decode_direct(json).map(|value| Self::ProcessorList(Arc::new(value)))
            }
            CONFIGURED_CARVER_REGISTRY => {
                ConfiguredCarver::decode(json).map(|value| Self::ConfiguredCarver(Arc::new(value)))
            }
            _ => return None,
        })
    }
}

impl WorldgenRegistries {
    pub fn insert(&mut self, id: String, element: WorldgenElement) {
        match element {
            WorldgenElement::DensityFunction(value) => {
                self.density_functions.insert(id, value);
            }
            WorldgenElement::Noise(value) => {
                self.noises.insert(id, value);
            }
            WorldgenElement::NoiseSettings(value) => {
                self.noise_settings.insert(id, value);
            }
            WorldgenElement::ConfiguredFeature(value) => {
                self.configured_features.insert(id, value);
            }
            WorldgenElement::PlacedFeature(value) => {
                self.placed_features.insert(id, value);
            }
            WorldgenElement::ProcessorList(value) => {
                self.processor_lists.insert(id, value);
            }
            WorldgenElement::ConfiguredCarver(value) => {
                self.configured_carvers.insert(id, value);
            }
        }
    }

    /// The references of one typed element, or `None` when `registry_id` is
    /// not typed or holds no such element.
    pub fn refs(&self, registry_id: &str, id: &str) -> Option<Refs> {
        let mut refs = Refs::default();
        match registry_id {
            density::DENSITY_FUNCTION_REGISTRY => self.density_functions.get(id)?.refs(&mut refs),
            noise::NOISE_REGISTRY => {
                self.noises.get(id)?;
            }
            noise::NOISE_SETTINGS_REGISTRY => self.noise_settings.get(id)?.refs(&mut refs),
            CONFIGURED_FEATURE_REGISTRY => self.configured_features.get(id)?.refs(&mut refs),
            PLACED_FEATURE_REGISTRY => self.placed_features.get(id)?.refs(&mut refs),
            PROCESSOR_LIST_REGISTRY => self.processor_lists.get(id)?.refs(&mut refs),
            CONFIGURED_CARVER_REGISTRY => self.configured_carvers.get(id)?.refs(&mut refs),
            _ => return None,
        }
        Some(refs)
    }

    pub fn same_frozen_registries(&self, other: &Self) -> bool {
        self.biome_climates == other.biome_climates
            && self.biome_features == other.biome_features
            && self.density_functions == other.density_functions
            && self.noises == other.noises
            && self.noise_settings == other.noise_settings
            && self.configured_features == other.configured_features
            && self.placed_features == other.placed_features
            && self.processor_lists == other.processor_lists
            && self.configured_carvers == other.configured_carvers
    }

    pub fn cached_biome_carvers(
        &self,
        id: &str,
        source_json: &Arc<[u8]>,
    ) -> Option<Result<Option<&carver::BiomeCarvers>, &DecodeError>> {
        let entry = self.biome_carvers.get(id)?;
        Arc::ptr_eq(&entry.source_json, source_json)
            .then(|| entry.value.as_ref().map(|opt| opt.as_ref()))
    }

    pub(crate) fn cached_biome_mob_spawns(
        &self,
        id: &str,
        source_json: &Arc<[u8]>,
    ) -> Option<
        Result<&crate::mob_spawn::RawBiomeMobSpawnSettings, crate::mob_spawn::MobSpawnCatalogError>,
    > {
        let entry = self.biome_mob_spawns.get(id)?;
        Arc::ptr_eq(&entry.source_json, source_json).then(|| match &entry.value {
            Ok(value) => Ok(value),
            Err(error) => Err(error.clone()),
        })
    }

    pub(crate) fn cached_structure_mob_spawns(
        &self,
        id: &str,
        source_json: &Arc<[u8]>,
    ) -> Option<
        Result<
            &crate::mob_spawn::RawStructureSpawnOverrides,
            crate::mob_spawn::MobSpawnCatalogError,
        >,
    > {
        let entry = self.structure_mob_spawns.get(id)?;
        Arc::ptr_eq(&entry.source_json, source_json).then(|| match &entry.value {
            Ok(value) => Ok(value),
            Err(error) => Err(error.clone()),
        })
    }

    pub fn cached_multi_noise_parameter_list(
        &self,
        id: &str,
        source_json: &Arc<[u8]>,
    ) -> Option<Result<&structure::DecodedMultiNoiseParameterList, &DecodeError>> {
        let entry = self.multi_noise_parameter_lists.get(id)?;
        Arc::ptr_eq(&entry.source_json, source_json).then_some(entry.value.as_ref())
    }

    pub fn cached_structure(
        &self,
        id: &str,
        source_json: &Arc<[u8]>,
    ) -> Option<Result<&Arc<structure::DecodedStructure>, &DecodeError>> {
        let entry = self.structures.get(id)?;
        Arc::ptr_eq(&entry.source_json, source_json).then_some(entry.value.as_ref())
    }

    pub fn cached_structure_set(
        &self,
        id: &str,
        source_json: &Arc<[u8]>,
    ) -> Option<Result<&Arc<structure::DecodedStructureSet>, &DecodeError>> {
        let entry = self.structure_sets.get(id)?;
        Arc::ptr_eq(&entry.source_json, source_json).then_some(entry.value.as_ref())
    }

    pub(crate) fn cached_structure_pools(
        &self,
        elements: &[crate::dynamic_registry::RawRegistryElement],
    ) -> Option<
        Result<
            Arc<crate::structure_pool::StructurePoolSet>,
            crate::structure_pool::StructurePoolError,
        >,
    > {
        let cached = self.structure_pools.as_ref()?;
        if cached.sources.len() != elements.len() {
            return None;
        }
        cached
            .sources
            .iter()
            .zip(elements)
            .all(|((id, source), element)| {
                id == &element.id && Arc::ptr_eq(source, &element.source_json)
            })
            .then(|| cached.pools.clone())
    }
}
