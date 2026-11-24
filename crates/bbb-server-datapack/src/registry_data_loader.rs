//! `RegistryDataLoader` equivalent: load every declared dynamic registry from the
//! merged datapack view, fail-closed.
//!
//! Load order and layering follow `WorldLoader.load` (`WorldLoader.java:25-86`): the
//! worldgen registry list is loaded first and the dimension (LEVEL_STEM) registry
//! second. Elements are retained as `RawRegistryElement`. Exact
//! `ConfiguredFeature`, `ConfiguredWorldCarver`, and `StructureProcessorType`
//! owners keep canonical NBT and unresolved references beside the winning source
//! bytes. `PlacedFeature` still only validates its placement list. D5 additionally
//! runs the worldgen dispatch-envelope validation (`worldgen_codec`) for remaining
//! dispatched families.
//! Element ids follow `FileToIdConverter.registry`: `<namespace>:<path minus .json>`.
//!
//! Errors accumulate per element, attributed with registry id and pack, and any error
//! fails the whole load (`RegistryDataLoader.java:217-225`: collected `loadingErrors`
//! abort startup through `ReportedException`).

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

use bbb_facts::registry_data_types::synchronized_registry_data_type;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_registry_codec::registry_codec::{
    CanonicalRegistryCodecValue, OwnedRegistryCodecReference, RegistryResourceIdentifier,
};
use bbb_registry_codec::{RegistryElementData, RegistryJsonValue as Value};

use crate::mob_spawn::{RawBiomeMobSpawnSettings, RawStructureSpawnOverrides};
use crate::structure_pool::StructurePoolSet;
use crate::worldgen::biome::BiomeFeatures;
use crate::worldgen::structure::{
    DecodedMultiNoiseParameterList, DecodedStructure, DecodedStructureSet, PARAMETER_LIST_REGISTRY,
    STRUCTURE_REGISTRY, STRUCTURE_SET_REGISTRY,
};
use crate::worldgen::{
    BiomeCarvers, BiomeClimate, CachedStructurePools, SourceStamped, WorldgenElement,
    WorldgenRegistries, PLACED_FEATURE_REGISTRY,
};

use super::dynamic_registry::{
    DatapackGeneration, DynamicRegistrySpec, LoadedDynamicRegistry, RawRegistryElement,
    RegistryValidator, DIMENSION_REGISTRIES_26_1, WORLDGEN_REGISTRIES_26_1,
};
use super::resource_overlay::{MergedServerDataView, PackOverlayInput};

/// Budget limits for datapack loads; out-of-budget input is rejected with attribution
/// (the same attributable-failure rule as the save surface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatapackLoadLimits {
    /// Maximum bytes of a single registry element file.
    pub max_element_bytes: usize,
    /// Maximum number of elements in one registry.
    pub max_elements_per_registry: usize,
}

impl Default for DatapackLoadLimits {
    fn default() -> Self {
        Self {
            // Vanilla's largest 26.1 registry element is well under 1 MiB; the
            // built-in snapshot is about 2 MiB across the whole registry surface.
            max_element_bytes: 8 * 1024 * 1024,
            max_elements_per_registry: 1 << 20,
        }
    }
}

/// One attributed element-loading error (`RegistryDataLoader` collects one per failed
/// element before aborting).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapackElementError {
    pub registry_id: String,
    pub element_id: Option<String>,
    pub pack: String,
    pub message: String,
}

impl fmt::Display for DatapackElementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.element_id {
            Some(element) => write!(
                f,
                "registry {} element {} from pack {}: {}",
                self.registry_id, element, self.pack, self.message
            ),
            None => write!(
                f,
                "registry {} (pack {}): {}",
                self.registry_id, self.pack, self.message
            ),
        }
    }
}

/// The aggregate fail-closed error for a datapack registry load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapackLoadError {
    pub errors: Vec<DatapackElementError>,
}

impl fmt::Display for DatapackLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "datapack registry load failed ({} errors):",
            self.errors.len()
        )?;
        for error in &self.errors {
            write!(f, "\n  - {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for DatapackLoadError {}

type PendingTemplatePoolEntry = (String, Arc<[u8]>, RegistryElementData);

/// Load every declared dynamic registry from the merged selected-pack view:
/// `WORLDGEN_REGISTRIES` first, then `DIMENSION_REGISTRIES` (`WorldLoader.java:37-48`).
///
/// Any element error, budget violation or failed registry validator aborts the load
/// with all collected errors; nothing partial is returned.
pub fn load_dynamic_registries(
    view: &MergedServerDataView,
    packs: &[PackOverlayInput<'_>],
    limits: DatapackLoadLimits,
) -> Result<DatapackGeneration, DatapackLoadError> {
    load_dynamic_registries_with_content_registry(view, packs, limits, ContentRegistry::vanilla())
}

/// Content-registry-aware form of [`load_dynamic_registries`].
pub fn load_dynamic_registries_with_content_registry(
    view: &MergedServerDataView,
    packs: &[PackOverlayInput<'_>],
    limits: DatapackLoadLimits,
    content_registry: &ContentRegistry,
) -> Result<DatapackGeneration, DatapackLoadError> {
    let mut errors = Vec::new();
    let mut registries = Vec::new();
    let mut worldgen = WorldgenRegistries::default();
    let mut pending_template_pools: Option<Vec<PendingTemplatePoolEntry>> = None;
    for spec in WORLDGEN_REGISTRIES_26_1
        .iter()
        .chain(DIMENSION_REGISTRIES_26_1.iter())
    {
        if let Some(registry) = load_registry(
            view,
            packs,
            spec,
            limits,
            content_registry,
            &mut worldgen,
            &mut pending_template_pools,
            &mut errors,
        ) {
            registries.push(registry);
        }
    }
    if errors.is_empty() {
        if let Some(mut pool_entries) = pending_template_pools {
            pool_entries.sort_by(|left, right| {
                crate::dynamic_registry::registration_key(&left.0)
                    .cmp(&crate::dynamic_registry::registration_key(&right.0))
            });
            let sources = pool_entries
                .iter()
                .map(|(id, bytes, _)| (id.clone(), Arc::clone(bytes)))
                .collect::<Vec<_>>();
            let parsed = pool_entries
                .into_iter()
                .map(|(id, _, data)| (id, data))
                .collect::<Vec<_>>();
            let known_features = registries
                .iter()
                .find(|registry| registry.registry_id == PLACED_FEATURE_REGISTRY)
                .map(|registry| {
                    registry
                        .elements
                        .iter()
                        .map(|element| element.id.clone())
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            let pools = StructurePoolSet::from_parsed_elements(
                &parsed,
                &worldgen.processor_lists,
                &known_features,
            )
            .map(Arc::new);
            worldgen.structure_pools = Some(CachedStructurePools { sources, pools });
        }
        Ok(DatapackGeneration {
            registries,
            worldgen,
        })
    } else {
        Err(DatapackLoadError { errors })
    }
}

#[allow(clippy::too_many_arguments)]
fn load_registry(
    view: &MergedServerDataView,
    packs: &[PackOverlayInput<'_>],
    spec: &DynamicRegistrySpec,
    limits: DatapackLoadLimits,
    content_registry: &ContentRegistry,
    worldgen: &mut WorldgenRegistries,
    pending_template_pools: &mut Option<Vec<PendingTemplatePoolEntry>>,
    errors: &mut Vec<DatapackElementError>,
) -> Option<LoadedDynamicRegistry> {
    let dir = spec.elements_dir();
    let dir_prefix = format!("{dir}/");
    let mut elements = Vec::new();
    for namespace in view.namespaces() {
        for path in view.list(namespace, dir) {
            // `FileToIdConverter.registry`: the element id path is relative to the
            // elements directory, without the `.json` suffix.
            let Some(relative) = path.strip_prefix(&dir_prefix) else {
                continue;
            };
            let Some(element_path) = relative.strip_suffix(".json") else {
                continue;
            };
            let element_id = format!("{namespace}:{element_path}");
            if elements.len() >= limits.max_elements_per_registry {
                errors.push(DatapackElementError {
                    registry_id: spec.registry_id.to_string(),
                    element_id: Some(element_id),
                    pack: String::new(),
                    message: format!(
                        "registry exceeds the {} element budget",
                        limits.max_elements_per_registry
                    ),
                });
                break;
            }
            let Some((pack_id, resources)) = view.winner(packs, namespace, path) else {
                continue;
            };
            let Some(bytes) = resources.read_shared(namespace, path) else {
                continue;
            };
            if bytes.len() > limits.max_element_bytes {
                errors.push(DatapackElementError {
                    registry_id: spec.registry_id.to_string(),
                    element_id: Some(element_id),
                    pack: pack_id.to_string(),
                    message: format!(
                        "element is {} bytes, above the {} byte budget",
                        bytes.len(),
                        limits.max_element_bytes
                    ),
                });
                continue;
            }
            let strict = WorldgenElement::is_typed_registry(spec.registry_id)
                || synchronized_registry_data_type(spec.registry_id).is_some();
            let decoded = if strict {
                bbb_registry_codec::decode_direct_registry_json(&bytes)
            } else {
                bbb_registry_codec::decode_loaded_registry_json(&bytes, limits.max_element_bytes)
            };
            let data = match decoded {
                Ok(data) => data,
                Err(error) => {
                    errors.push(DatapackElementError {
                        registry_id: spec.registry_id.to_string(),
                        element_id: Some(element_id),
                        pack: pack_id.to_string(),
                        message: format!("malformed JSON: {error}"),
                    });
                    continue;
                }
            };
            let json = data.json().expect("datapack JSON source");
            let codec_value =
                retain_loaded_registry_codec(spec.registry_id, &data, content_registry);
            let mut codec_value = codec_value;
            if spec.registry_id == "minecraft:worldgen/biome" {
                let decoded_carvers = BiomeCarvers::decode_optional(json);
                match add_biome_inline_refs(
                    json,
                    decoded_carvers.as_ref().ok().and_then(Option::as_ref),
                    codec_value.as_mut(),
                ) {
                    Ok(features) => {
                        worldgen.biome_features.insert(element_id.clone(), features);
                    }
                    Err(message) => {
                        errors.push(DatapackElementError {
                            registry_id: spec.registry_id.to_string(),
                            element_id: Some(element_id),
                            pack: pack_id.to_string(),
                            message,
                        });
                        continue;
                    }
                }
                worldgen.biome_carvers.insert(
                    element_id.clone(),
                    SourceStamped {
                        source_json: Arc::clone(&bytes),
                        value: decoded_carvers,
                    },
                );
                worldgen.biome_mob_spawns.insert(
                    element_id.clone(),
                    SourceStamped {
                        source_json: Arc::clone(&bytes),
                        value: RawBiomeMobSpawnSettings::parse_from_root(&element_id, data.root()),
                    },
                );
            }
            if let Some(typed) = decode_worldgen_element(spec.registry_id, json) {
                match typed {
                    Ok(element) => worldgen.insert(element_id.clone(), element),
                    Err(message) => {
                        errors.push(DatapackElementError {
                            registry_id: spec.registry_id.to_string(),
                            element_id: Some(element_id),
                            pack: pack_id.to_string(),
                            message,
                        });
                        continue;
                    }
                }
            }
            // D5: dispatched worldgen families validate their top-level
            // `type` envelope against the 26.1 whitelists.
            if !WorldgenElement::is_typed_registry(spec.registry_id) {
                if let Err(codec) = super::worldgen_codec::validate_worldgen_element(
                    spec.registry_id,
                    &element_id,
                    json,
                ) {
                    errors.push(DatapackElementError {
                        registry_id: spec.registry_id.to_string(),
                        element_id: Some(element_id),
                        pack: pack_id.to_string(),
                        message: codec.message,
                    });
                    continue;
                }
            }
            match spec.registry_id {
                PARAMETER_LIST_REGISTRY => {
                    worldgen.multi_noise_parameter_lists.insert(
                        element_id.clone(),
                        SourceStamped {
                            source_json: Arc::clone(&bytes),
                            value: DecodedMultiNoiseParameterList::decode(json),
                        },
                    );
                }
                STRUCTURE_REGISTRY => {
                    let path = format!("{STRUCTURE_REGISTRY}.{element_id}");
                    let decoded_structure = DecodedStructure::decode(json, &path).map(Arc::new);
                    let spawn_overrides = match &decoded_structure {
                        Ok(structure) => structure.spawn_overrides.clone(),
                        Err(_) => RawStructureSpawnOverrides::parse_from_root(data.root(), &path),
                    };
                    worldgen.structure_mob_spawns.insert(
                        element_id.clone(),
                        SourceStamped {
                            source_json: Arc::clone(&bytes),
                            value: spawn_overrides,
                        },
                    );
                    worldgen.structures.insert(
                        element_id.clone(),
                        SourceStamped {
                            source_json: Arc::clone(&bytes),
                            value: decoded_structure,
                        },
                    );
                }
                STRUCTURE_SET_REGISTRY => {
                    let path = format!("{STRUCTURE_SET_REGISTRY}.{element_id}");
                    worldgen.structure_sets.insert(
                        element_id.clone(),
                        SourceStamped {
                            source_json: Arc::clone(&bytes),
                            value: DecodedStructureSet::decode(json, &path).map(Arc::new),
                        },
                    );
                }
                _ => {}
            }
            let keep_loaded_ast = !WorldgenElement::is_typed_registry(spec.registry_id)
                && !matches!(
                    spec.registry_id,
                    "minecraft:worldgen/template_pool"
                        | "minecraft:worldgen/structure"
                        | "minecraft:worldgen/structure_set"
                        | "minecraft:worldgen/world_preset"
                        | "minecraft:worldgen/flat_level_generator_preset"
                        | "minecraft:worldgen/multi_noise_biome_source_parameter_list"
                )
                && (codec_value.is_none()
                    || matches!(
                        spec.registry_id,
                        "minecraft:worldgen/biome"
                            | "minecraft:dimension_type"
                            | "minecraft:damage_type"
                            | "minecraft:enchantment"
                            | "minecraft:timeline"
                    ));
            let retained_source = if spec.registry_id == "minecraft:worldgen/template_pool" {
                pending_template_pools.get_or_insert_with(Vec::new).push((
                    element_id.clone(),
                    Arc::clone(&bytes),
                    data,
                ));
                None
            } else {
                keep_loaded_ast.then_some(data)
            };
            elements.push(RawRegistryElement {
                id: element_id,
                source_pack: pack_id.to_string(),
                source_json: bytes,
                source: retained_source,
                codec_value,
            });
        }
    }
    // ResourceManagerRegistryLoadTask sorts registrations by `Identifier`, whose
    // natural order compares resource path first and namespace second
    // (`Identifier.java:142-149`). This registration order becomes the holder id
    // order, so ordinary namespace-first string sorting is not equivalent.
    elements.sort_by(|left, right| {
        crate::dynamic_registry::registration_key(&left.id)
            .cmp(&crate::dynamic_registry::registration_key(&right.id))
    });
    if spec.validator == RegistryValidator::NonEmpty && elements.is_empty() {
        errors.push(DatapackElementError {
            registry_id: spec.registry_id.to_string(),
            element_id: None,
            pack: String::new(),
            message: "registry must contain at least one element".to_string(),
        });
        return None;
    }
    if spec.registry_id == "minecraft:worldgen/biome" {
        for element in &elements {
            match BiomeClimate::decode(element.json().expect("biome JSON is retained")) {
                Ok(climate) => worldgen.biome_climates.push(climate),
                Err(message) => errors.push(DatapackElementError {
                    registry_id: spec.registry_id.to_string(),
                    element_id: Some(element.id.clone()),
                    pack: element.source_pack.clone(),
                    message,
                }),
            }
        }
    }
    Some(LoadedDynamicRegistry {
        registry_id: spec.registry_id.to_string(),
        elements,
    })
}

/// The typed decode of an executable worldgen registry element.
fn decode_worldgen_element(
    registry_id: &str,
    json: &Value,
) -> Option<Result<WorldgenElement, String>> {
    if !WorldgenElement::is_typed_registry(registry_id) {
        return None;
    }
    Some(
        WorldgenElement::decode(registry_id, json)
            .expect("typed registry")
            .map_err(|error| error.to_string()),
    )
}

/// Validates a biome's inline carvers and placed features and records their
/// references for the frozen lookup.
fn add_biome_inline_refs(
    json: &Value,
    predecoded_carvers: Option<&BiomeCarvers>,
    codec_value: Option<&mut CanonicalRegistryCodecValue>,
) -> Result<BiomeFeatures, String> {
    let features = BiomeFeatures::decode(json).map_err(|e| e.to_string())?;
    let refs = if let Some(carvers) = predecoded_carvers {
        let mut refs = crate::worldgen::Refs::default();
        carvers.inline_refs(&mut refs);
        features.refs(&mut refs);
        refs
    } else {
        crate::worldgen::biome_inline_generation_refs(json, &features).map_err(|e| e.to_string())?
    };
    if let Some(codec_value) = codec_value {
        codec_value.references.extend(refs.items.into_iter().map(
            |(target_registry, kind, resource_id)| OwnedRegistryCodecReference {
                target_registry,
                kind,
                resource_id: RegistryResourceIdentifier::from_normalized(resource_id),
            },
        ));
    }
    Ok(features)
}

fn retain_loaded_registry_codec(
    registry_id: &str,
    data: &bbb_registry_codec::RegistryElementData,
    content_registry: &ContentRegistry,
) -> Option<CanonicalRegistryCodecValue> {
    synchronized_registry_data_type(registry_id)?;
    // Publication validates sources without a retained projection.
    bbb_registry_codec::decode_exact_synchronized_registry_element_with_content_registry(
        registry_id,
        data,
        content_registry,
    )
    .and_then(|decoded| decoded.into_canonical())
    .ok()
}

#[cfg(test)]
mod tests {
    use super::super::resource_overlay::PackPathFilter;
    use super::*;
    use crate::dynamic_registry::registration_key;
    use bbb_registry_codec::registry_codec::RegistryReferenceKind;
    use std::collections::BTreeMap as Map;

    struct MockPack {
        namespaces: Vec<String>,
        files: Map<(String, String), Vec<u8>>,
    }

    impl MockPack {
        fn new(files: &[(&str, &str, &str)]) -> Self {
            let mut namespaces = Vec::new();
            let mut map = Map::new();
            for (namespace, path, content) in files {
                if !namespaces.contains(&namespace.to_string()) {
                    namespaces.push(namespace.to_string());
                }
                map.insert(
                    (namespace.to_string(), path.to_string()),
                    content.as_bytes().to_vec(),
                );
            }
            namespaces.sort();
            Self {
                namespaces,
                files: map,
            }
        }
    }

    impl super::super::pack::ServerPackResources for MockPack {
        fn provided_namespaces(&self) -> Vec<String> {
            self.namespaces.clone()
        }

        fn list_paths(&self, namespace: &str, dir: &str) -> Vec<String> {
            let prefix = if dir.is_empty() {
                String::new()
            } else {
                format!("{dir}/")
            };
            self.files
                .keys()
                .filter(|(ns, path)| ns == namespace && path.starts_with(&prefix))
                .map(|(_, path)| path.clone())
                .collect()
        }

        fn read(&self, namespace: &str, path: &str) -> Option<Vec<u8>> {
            self.files
                .get(&(namespace.to_string(), path.to_string()))
                .cloned()
        }

        fn read_root(&self, _path: &str) -> Option<Vec<u8>> {
            None
        }
    }

    fn input<'a>(pack_id: &'a str, resources: &'a MockPack) -> PackOverlayInput<'a> {
        PackOverlayInput {
            pack_id,
            resources,
            filter: None,
        }
    }

    /// One dummy element per `nonEmpty` registry so loads not focused on the
    /// validators succeed (mirrors the never-empty built-in vanilla pack).
    fn variant_filler() -> MockPack {
        MockPack::new(&[
            ("minecraft", "wolf_variant/x.json", "{}"),
            ("minecraft", "wolf_sound_variant/x.json", "{}"),
            ("minecraft", "pig_variant/x.json", "{}"),
            ("minecraft", "pig_sound_variant/x.json", "{}"),
            ("minecraft", "frog_variant/x.json", "{}"),
            ("minecraft", "cat_variant/x.json", "{}"),
            ("minecraft", "cat_sound_variant/x.json", "{}"),
            ("minecraft", "cow_variant/x.json", "{}"),
            ("minecraft", "cow_sound_variant/x.json", "{}"),
            ("minecraft", "chicken_variant/x.json", "{}"),
            ("minecraft", "chicken_sound_variant/x.json", "{}"),
            ("minecraft", "zombie_nautilus_variant/x.json", "{}"),
            ("minecraft", "painting_variant/x.json", "{}"),
        ])
    }

    fn load_with_limits(
        packs: &[PackOverlayInput<'_>],
        limits: DatapackLoadLimits,
    ) -> Result<DatapackGeneration, DatapackLoadError> {
        let view = MergedServerDataView::merge(packs);
        load_dynamic_registries(&view, packs, limits)
    }

    fn load(packs: &[PackOverlayInput<'_>]) -> Result<DatapackGeneration, DatapackLoadError> {
        load_with_limits(packs, DatapackLoadLimits::default())
    }

    #[test]
    fn loads_every_declared_registry_in_vanilla_order() {
        let base = MockPack::new(&[
            ("minecraft", "dimension_type/overworld.json", "{}"),
            (
                "minecraft",
                "worldgen/biome/plains.json",
                "{\"temperature\":0.8,\"has_precipitation\":true,\"features\":[]}",
            ),
            (
                "minecraft",
                "dimension/overworld.json",
                "{\"type\":\"minecraft:overworld\",\"generator\":{\"type\":\"minecraft:flat\"}}",
            ),
        ]);
        let filler = variant_filler();
        let packs = [input("vanilla", &base), input("builtin-filler", &filler)];
        let generation = load(&packs).unwrap();

        assert_eq!(generation.registries.len(), 46);
        assert_eq!(
            generation.registries[0].registry_id,
            "minecraft:dimension_type"
        );
        assert_eq!(generation.registries[45].registry_id, "minecraft:dimension");
        let biome = generation.registry("minecraft:worldgen/biome").unwrap();
        assert_eq!(biome.elements.len(), 1);
        assert_eq!(biome.elements[0].id, "minecraft:plains");
        assert_eq!(biome.elements[0].source_pack, "vanilla");
        assert_eq!(
            biome.elements[0]
                .json()
                .unwrap()
                .get("temperature")
                .and_then(Value::as_f64),
            Some(0.8)
        );
    }

    #[test]
    fn external_pack_overrides_and_extends_with_new_namespace() {
        let base = MockPack::new(&[("minecraft", "dimension_type/overworld.json", "{\"a\":1}")]);
        let ext = MockPack::new(&[
            ("minecraft", "dimension_type/overworld.json", "{\"a\":2}"),
            ("example", "dimension_type/moon.json", "{}"),
        ]);
        let filler = variant_filler();
        let packs = [
            input("vanilla", &base),
            input("file/moon", &ext),
            input("builtin-filler", &filler),
        ];
        let generation = load(&packs).unwrap();

        let registry = generation.registry("minecraft:dimension_type").unwrap();
        let ids: Vec<&str> = registry.elements.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["example:moon", "minecraft:overworld"]);
        let overworld = &registry.elements[1];
        assert_eq!(overworld.source_pack, "file/moon");
        assert_eq!(
            overworld.json().unwrap().get("a").and_then(Value::as_i64),
            Some(2)
        );
    }

    #[test]
    fn nonexact_registry_source_keeps_raw_text_and_existing_resource_budget() {
        let large = vec!["0"; 5000].join(",");
        let mut wide = (0..5000)
            .map(|index| format!("\"k{index}\":0"))
            .collect::<Vec<_>>()
            .join(",");
        wide.push_str(",\"k0\":1");
        let nested = format!("{}0{}", "[".repeat(70), "]".repeat(70));
        let source = format!(
            r#"{{"trades":["example:trade"],"amount":1,"large":[{large}],"wide":{{{wide}}},"nested":{nested},"\ud800":"\udc00"}}"#
        );
        let base = MockPack::new(&[("example", "trade_set/raw.json", &source)]);
        let filler = variant_filler();
        let generation = load(&[input("file/raw", &base), input("filler", &filler)]).unwrap();
        let element = generation
            .element("minecraft:trade_set", "example:raw")
            .unwrap();
        assert_eq!(element.source_json.as_ref(), source.as_bytes());
        let json = element.json().unwrap();
        assert_eq!(json.get("large").unwrap().as_array().unwrap().len(), 5000);
        let wide = json.get("wide").unwrap();
        assert_eq!(wide.as_object().unwrap().len(), 5000);
        assert_eq!(wide.as_object().unwrap()[0].0, "k0");
        assert_eq!(wide.get("k0").and_then(Value::as_i64), Some(1));
        let raw = json
            .as_object()
            .unwrap()
            .iter()
            .find(|(name, _)| name.units() == [0xd800])
            .unwrap();
        let Value::String(text) = &raw.1 else {
            panic!("raw string");
        };
        assert_eq!(text.units(), [0xdc00]);
        let trade_set = crate::trades::parse_trade_set(json).unwrap();
        assert_eq!(
            trade_set.trades,
            crate::trades::TradeHolderSet::Items(vec!["example:trade".into()])
        );
        assert!(element.codec_value.is_none());

        let overwritten = format!(
            r#"{{"trades":[],"amount":1,"x":{}0{},"x":0}}"#,
            "[".repeat(127),
            "]".repeat(127)
        );
        let deep = MockPack::new(&[("example", "trade_set/deep.json", &overwritten)]);
        assert!(load(&[input("file/deep", &deep), input("filler", &filler)]).is_err());

        let mut oversized = source;
        oversized.extend(std::iter::repeat_n(
            ' ',
            8 * 1024 * 1024 + 1 - oversized.len(),
        ));
        let custom = MockPack::new(&[("example", "trade_set/large.json", &oversized)]);
        let generation = load_with_limits(
            &[input("file/large", &custom), input("filler", &filler)],
            DatapackLoadLimits {
                max_element_bytes: oversized.len(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            generation
                .element("minecraft:trade_set", "example:large")
                .unwrap()
                .source_json
                .len(),
            oversized.len()
        );
    }

    #[test]
    fn registration_order_compares_path_before_namespace() {
        assert_eq!(registration_key("same"), ("same", "minecraft"));

        let base = MockPack::new(&[
            ("zeta", "dimension_type/same.json", "{}"),
            ("alpha", "dimension_type/same.json", "{}"),
            ("aaa", "dimension_type/z_path.json", "{}"),
            ("zzz", "dimension_type/a_path.json", "{}"),
            (
                "aaa",
                "worldgen/biome/z_path.json",
                r#"{"temperature":1.0,"has_precipitation":false,"features":[]}"#,
            ),
            (
                "zzz",
                "worldgen/biome/a_path.json",
                r#"{"temperature":0.0,"has_precipitation":true,"features":[]}"#,
            ),
        ]);
        let filler = variant_filler();
        let packs = [input("vanilla", &base), input("builtin-filler", &filler)];
        let generation = load(&packs).unwrap();

        let ids: Vec<&str> = generation
            .registry("minecraft:dimension_type")
            .unwrap()
            .elements
            .iter()
            .map(|element| element.id.as_str())
            .collect();
        assert_eq!(ids, ["zzz:a_path", "alpha:same", "zeta:same", "aaa:z_path"]);
        assert_eq!(
            generation
                .worldgen
                .biome_climates
                .iter()
                .map(|climate| climate.temperature)
                .collect::<Vec<_>>(),
            [0.0, 1.0],
        );
    }

    #[test]
    fn processor_lists_decode_both_shapes_and_retain_references() {
        let valid = MockPack::new(&[
            (
                "example",
                "worldgen/processor_list/object.json",
                r##"{"processors":[{"processor_type":"minecraft:block_rot","rottable_blocks":"#minecraft:features_cannot_replace","integrity":0.5}]}"##,
            ),
            (
                "example",
                "worldgen/processor_list/bare.json",
                r#"[{"processor_type":"minecraft:nop"}]"#,
            ),
        ]);
        let filler = variant_filler();
        let packs = [
            input("file/valid", &valid),
            input("builtin-filler", &filler),
        ];
        let generation = load(&packs).unwrap();
        let registry = generation
            .registry("minecraft:worldgen/processor_list")
            .unwrap();
        assert_eq!(registry.elements.len(), 2);
        let object = generation
            .worldgen
            .refs("minecraft:worldgen/processor_list", "example:object")
            .unwrap();
        assert_eq!(
            object.items,
            [(
                "minecraft:block",
                RegistryReferenceKind::Tag,
                "minecraft:features_cannot_replace".to_string()
            )]
        );
        assert!(generation
            .worldgen
            .processor_lists
            .contains_key("example:bare"));
    }

    #[test]
    fn non_json_files_in_registry_dirs_are_ignored() {
        let base = MockPack::new(&[
            ("minecraft", "dimension_type/overworld.json", "{}"),
            ("minecraft", "dimension_type/notes.txt", "not an element"),
        ]);
        let filler = variant_filler();
        let packs = [input("vanilla", &base), input("builtin-filler", &filler)];
        let generation = load(&packs).unwrap();
        let registry = generation.registry("minecraft:dimension_type").unwrap();
        assert_eq!(registry.elements.len(), 1);
    }

    #[test]
    fn filter_blocked_elements_are_not_loaded() {
        let base = MockPack::new(&[("minecraft", "dimension_type/overworld.json", "{}")]);
        let blocker = MockPack::new(&[]);
        let filler = variant_filler();
        struct BlockAll;
        impl PackPathFilter for BlockAll {
            fn is_blocked(&self, namespace: &str, path: &str) -> bool {
                namespace == "minecraft" && path.starts_with("dimension_type/")
            }
        }
        let packs = [
            input("vanilla", &base),
            PackOverlayInput {
                pack_id: "file/filter",
                resources: &blocker,
                filter: Some(&BlockAll),
            },
            input("builtin-filler", &filler),
        ];
        let generation = load(&packs).unwrap();
        assert_eq!(
            generation
                .registry("minecraft:dimension_type")
                .unwrap()
                .elements
                .len(),
            0
        );
    }

    /// End-to-end: the embedded built-in vanilla pack loads every declared registry
    /// without a single error.
    #[test]
    fn embedded_vanilla_pack_loads_all_declared_registries() {
        use super::super::dynamic_registry::{DIMENSION_REGISTRIES_26_1, WORLDGEN_REGISTRIES_26_1};
        use super::super::vanilla_snapshot::VanillaDatapackSnapshot;

        let snapshot = std::sync::Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
        let vanilla_pack = snapshot.vanilla_pack();
        let packs = [PackOverlayInput {
            pack_id: "vanilla",
            resources: &vanilla_pack,
            filter: None,
        }];
        let generation = load(&packs).unwrap();
        assert_eq!(generation.registries.len(), 46);

        // Self-consistency: the loaded element count equals the number of `.json`
        // files under the declared element directories outside `datapacks/`.
        let declared_dirs: Vec<String> = WORLDGEN_REGISTRIES_26_1
            .iter()
            .chain(DIMENSION_REGISTRIES_26_1.iter())
            .map(|spec| format!("{}/", spec.elements_dir()))
            .collect();
        let expected_elements = snapshot
            .paths()
            .filter(|path| !path.starts_with("datapacks/"))
            .filter(|path| {
                path.ends_with(".json") && declared_dirs.iter().any(|dir| path.starts_with(dir))
            })
            .count();
        let loaded_elements: usize = generation
            .registries
            .iter()
            .map(|registry| registry.elements.len())
            .sum();
        assert_eq!(loaded_elements, expected_elements);
        assert!(loaded_elements > 1500);
        let typed = |registry: &str| generation.registry(registry).unwrap().elements.len();
        assert_eq!(
            generation.worldgen.configured_features.len(),
            typed("minecraft:worldgen/configured_feature")
        );
        assert_eq!(
            generation.worldgen.placed_features.len(),
            typed("minecraft:worldgen/placed_feature")
        );
        assert_eq!(
            generation.worldgen.processor_lists.len(),
            typed("minecraft:worldgen/processor_list")
        );

        assert_eq!(
            generation.worldgen.density_functions.len(),
            typed("minecraft:worldgen/density_function")
        );
        assert_eq!(
            generation.worldgen.noises.len(),
            typed("minecraft:worldgen/noise")
        );
        assert_eq!(
            generation.worldgen.noise_settings.len(),
            typed("minecraft:worldgen/noise_settings")
        );

        // 26.1 ships no built-in `dimension/` tree (`WorldPresets`/`WorldDimensions`
        // bootstrap the three stems in code).
        assert_eq!(
            generation
                .registry("minecraft:dimension")
                .unwrap()
                .elements
                .len(),
            0
        );
        let biome = generation.registry("minecraft:worldgen/biome").unwrap();
        assert!(biome.elements.iter().any(|e| e.id == "minecraft:plains"));
        let dimension_type = generation.registry("minecraft:dimension_type").unwrap();
        assert!(dimension_type
            .elements
            .iter()
            .any(|e| e.id == "minecraft:overworld"));
        assert!(dimension_type
            .elements
            .iter()
            .all(|e| e.source_pack == "vanilla"));
    }
}
