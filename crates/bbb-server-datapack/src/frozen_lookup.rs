//! Frozen element and tag membership for one registry-publication candidate.
//!
//! Vanilla gives every data-loaded registry a registration lookup before any
//! element codec runs, then freezes the complete set after all load tasks finish
//! (`RegistryDataLoader.java:195-225`, `MappedRegistry.java:275-305,359-380`).
//! Consequently this lookup proves only that a named holder or tag belongs to
//! the candidate; it never follows the holder and therefore permits legal
//! self- and mutual-cycles (`RegistryFileCodec.java:49-72`). Built-in registries
//! remain loading context when the client rebuilds synchronized registries
//! (`RegistryDataCollector.java:73-107`), so datapack contents can never extend
//! their fixed element catalogs here.

use bbb_facts::static_registry_elements::vanilla_static_registry_elements;
use bbb_protocol::{
    content_registry::ContentRegistry, registry_data_types::built_in_registry_type,
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};
use bbb_registry_codec::registry_codec::{RegistryReferenceKind, RegistryReferenceLookup};
use bbb_registry_codec::PersistentItemStackHolderResolver;

use crate::dynamic_registry::DatapackGeneration;
use crate::registry_data_loader::{DatapackElementError, DatapackLoadError};
use crate::tag_loader::TagLoadReport;

/// Exact membership facts from one frozen datapack startup candidate.
///
/// The generation and tag report must be the pair produced by the same startup
/// load. Keeping both as borrowed values prevents this seam from silently
/// consulting a previous or subsequent generation.
pub struct FrozenDatapackRegistryLookup<'a> {
    generation: &'a DatapackGeneration,
    tags: &'a TagLoadReport,
    content_registry: &'a ContentRegistry,
}

impl<'a> FrozenDatapackRegistryLookup<'a> {
    pub fn new(generation: &'a DatapackGeneration, tags: &'a TagLoadReport) -> Self {
        Self::new_with_content_registry(generation, tags, ContentRegistry::vanilla())
    }

    pub fn new_with_content_registry(
        generation: &'a DatapackGeneration,
        tags: &'a TagLoadReport,
        content_registry: &'a ContentRegistry,
    ) -> Self {
        Self {
            generation,
            tags,
            content_registry,
        }
    }

    pub fn with_content_registry(mut self, content_registry: &'a ContentRegistry) -> Self {
        self.content_registry = content_registry;
        self
    }

    pub fn content_registry(&self) -> &'a ContentRegistry {
        self.content_registry
    }

    pub fn from_startup(startup: &'a crate::startup::DatapackStartup) -> Self {
        Self::new_with_content_registry(
            &startup.registries,
            &startup.tags,
            &startup.content_registry,
        )
    }
}

impl RegistryReferenceLookup for FrozenDatapackRegistryLookup<'_> {
    fn registry_available(&self, registry: &str) -> bool {
        if built_in_registry_type(registry).is_some() {
            vanilla_static_registry_elements(registry).is_some()
        } else {
            self.generation.registry(registry).is_some()
        }
    }

    fn element_membership(
        &self,
        registry: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        if !RegistryReferenceLookup::registry_available(self, registry) {
            PersistentRegistryMembership::Unavailable
        } else if self.contains_element(registry, resource_id) {
            PersistentRegistryMembership::Present
        } else {
            PersistentRegistryMembership::Absent
        }
    }

    fn tag_membership(&self, registry: &str, resource_id: &str) -> PersistentRegistryMembership {
        if !RegistryReferenceLookup::registry_available(self, registry) {
            PersistentRegistryMembership::Unavailable
        } else if self.contains_tag(registry, resource_id) {
            PersistentRegistryMembership::Present
        } else {
            PersistentRegistryMembership::Absent
        }
    }

    fn contains_element(&self, registry: &str, resource_id: &str) -> bool {
        if built_in_registry_type(registry).is_some() {
            return static_registry_contains(registry, resource_id)
                || content_registry_contains(self.content_registry, registry, resource_id);
        }

        self.generation
            .registry(registry)
            .is_some_and(|loaded| loaded.element_index(resource_id).is_some())
    }

    fn contains_tag(&self, registry: &str, resource_id: &str) -> bool {
        self.tags
            .registry(registry)
            .and_then(|loaded| loaded.tag(resource_id))
            .is_some()
    }
}

impl PersistentRegistryResolver for FrozenDatapackRegistryLookup<'_> {
    fn registry_available(&self, registry_id: &str) -> bool {
        RegistryReferenceLookup::registry_available(self, registry_id)
    }

    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        self.element_membership(registry_id, resource_id)
    }

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        if !RegistryReferenceLookup::registry_available(self, registry_id) {
            return PersistentRegistryTagMembers::Unavailable;
        }
        self.tags
            .registry(registry_id)
            .and_then(|registry| registry.tag(tag_id))
            .map(|members| PersistentRegistryTagMembers::Present(members.to_vec()))
            .unwrap_or(PersistentRegistryTagMembers::Absent)
    }
}

impl PersistentItemStackHolderResolver for FrozenDatapackRegistryLookup<'_> {
    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        self.generation
            .registry(registry_id)?
            .element_index(resource_id)
            .and_then(|index| i32::try_from(index).ok())
    }

    fn holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        self.generation
            .registry(registry_id)?
            .elements
            .get(usize::try_from(holder_id).ok()?)
            .map(|element| element.id.clone())
    }

    fn content_registry(&self) -> &ContentRegistry {
        self.content_registry
    }
}

fn content_registry_contains(
    content_registry: &ContentRegistry,
    registry: &str,
    resource_id: &str,
) -> bool {
    if content_registry.is_vanilla_only() {
        return false;
    }
    match registry {
        "minecraft:block" => content_registry.block_by_resource_id(resource_id).is_some(),
        "minecraft:item" => content_registry.item_by_resource_id(resource_id).is_some(),
        "minecraft:entity_type" => content_registry
            .entity_by_resource_id(resource_id)
            .is_some(),
        "minecraft:sound_event" => content_registry
            .sound_event_by_resource_id(resource_id)
            .is_some(),
        "minecraft:particle_type" => content_registry
            .particle_by_resource_id(resource_id)
            .is_some(),
        _ => false,
    }
}

fn static_registry_contains(registry: &str, resource_id: &str) -> bool {
    use std::collections::{BTreeMap, HashSet};
    use std::sync::{OnceLock, RwLock};

    type StaticRegistryCache = BTreeMap<String, Option<HashSet<&'static str>>>;

    static CACHE: OnceLock<RwLock<StaticRegistryCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| RwLock::new(BTreeMap::new()));
    if let Some(entry) = cache
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(registry)
    {
        return entry
            .as_ref()
            .is_some_and(|elements| elements.contains(resource_id));
    }
    let set = vanilla_static_registry_elements(registry)
        .map(|elements| elements.iter().copied().collect::<HashSet<_>>());
    let present = set
        .as_ref()
        .is_some_and(|elements| elements.contains(resource_id));
    cache
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(registry.to_string(), set);
    present
}

/// Generation-final membership check for every retained named holder and tag.
///
/// Exact load keeps placeholders so legal cycles do not recurse
/// (`RegistryFileCodec.java:49-72`). Vanilla then freezes every load task and
/// aborts on the collected errors (`RegistryDataLoader.java:216-218`,
/// `MappedRegistry.java:275-285`). BBB runs that check after tags exist because
/// STATIC tags are worldgen decode context (`WorldLoader.java:37-40`).
pub fn validate_frozen_registry_codec_references(
    generation: &DatapackGeneration,
    tags: &TagLoadReport,
) -> Result<(), DatapackLoadError> {
    validate_frozen_registry_codec_references_with_content_registry(
        generation,
        tags,
        ContentRegistry::vanilla(),
    )
}

/// Content-registry-aware form of [`validate_frozen_registry_codec_references`].
pub fn validate_frozen_registry_codec_references_with_content_registry(
    generation: &DatapackGeneration,
    tags: &TagLoadReport,
    content_registry: &ContentRegistry,
) -> Result<(), DatapackLoadError> {
    let lookup =
        FrozenDatapackRegistryLookup::new_with_content_registry(generation, tags, content_registry);
    validate_frozen_references_with_lookup(generation, &lookup)
}

pub(crate) fn validate_frozen_references_with_lookup(
    generation: &DatapackGeneration,
    lookup: &dyn RegistryReferenceLookup,
) -> Result<(), DatapackLoadError> {
    let mut errors = Vec::new();
    for registry in &generation.registries {
        for element in &registry.elements {
            let typed = generation.worldgen.refs(&registry.registry_id, &element.id);
            let retained = element
                .codec_value
                .iter()
                .flat_map(|value| &value.references)
                .map(|reference| {
                    (
                        reference.target_registry,
                        reference.kind,
                        reference.resource_id.as_str(),
                    )
                });
            let typed = typed
                .iter()
                .flat_map(|refs| &refs.items)
                .map(|(target, kind, resource_id)| (*target, *kind, resource_id.as_str()));
            for (target_registry, kind, resource_id) in retained.chain(typed) {
                let present = match kind {
                    RegistryReferenceKind::Element => {
                        lookup.contains_element(target_registry, resource_id)
                    }
                    RegistryReferenceKind::Tag => lookup.contains_tag(target_registry, resource_id),
                };
                if present {
                    continue;
                }
                let kind = match kind {
                    RegistryReferenceKind::Element => "element",
                    RegistryReferenceKind::Tag => "tag",
                };
                errors.push(DatapackElementError {
                    registry_id: registry.registry_id.clone(),
                    element_id: Some(element.id.clone()),
                    pack: element.source_pack.clone(),
                    message: format!("missing {kind} {resource_id} in {target_registry}"),
                });
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(DatapackLoadError { errors })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_membership_26_1;
    use bbb_protocol::{registry_data_types::built_in_registry_type, PersistentRegistryMembership};
    use bbb_registry_codec::registry_codec::{
        CanonicalRegistryCodecValue, OwnedRegistryCodecReference, RegistryReferenceKind,
        RegistryReferenceLookup, RegistryResourceIdentifier,
    };
    use serde_json::json;

    use crate::dynamic_registry::{DatapackGeneration, LoadedDynamicRegistry, RawRegistryElement};
    use crate::tag_loader::{LoadedRegistryTags, TagLoadReport};

    use super::{validate_frozen_registry_codec_references, FrozenDatapackRegistryLookup};

    fn generation(registries: &[(&str, &[(&str, serde_json::Value)])]) -> DatapackGeneration {
        let mut worldgen = crate::worldgen::WorldgenRegistries::default();
        for (registry, elements) in registries {
            for (id, value) in *elements {
                let source =
                    bbb_registry_codec::parse_registry_json(&serde_json::to_vec(value).unwrap())
                        .unwrap();
                if let Some(typed) = crate::worldgen::WorldgenElement::decode(registry, &source) {
                    worldgen.insert((*id).to_string(), typed.unwrap());
                }
            }
        }
        DatapackGeneration {
            registries: registries
                .iter()
                .map(|(registry_id, elements)| LoadedDynamicRegistry {
                    registry_id: (*registry_id).to_string(),
                    elements: elements
                        .iter()
                        .map(|(id, value)| RawRegistryElement {
                            id: (*id).to_string(),
                            source_pack: "test".to_string(),
                            source_json: serde_json::to_vec(value).unwrap().into(),
                            source: Some(
                                bbb_registry_codec::decode_direct_registry_json(
                                    &serde_json::to_vec(&(value.clone())).unwrap(),
                                )
                                .unwrap(),
                            ),
                            codec_value: None,
                        })
                        .collect(),
                })
                .collect(),
            worldgen,
        }
    }

    fn complete_noise_settings() -> serde_json::Value {
        let noise_router = crate::worldgen_codec::NOISE_ROUTER_FIELDS_26_1
            .into_iter()
            .map(|field| (field.to_string(), json!(0.0)))
            .collect::<serde_json::Map<_, _>>();
        json!({
            "noise": {
                "min_y": 0,
                "height": 128,
                "size_horizontal": 1,
                "size_vertical": 2
            },
            "default_block": {"Name": "minecraft:stone"},
            "default_fluid": {"Name": "minecraft:water"},
            "noise_router": noise_router,
            "surface_rule": {"type": "minecraft:bandlands"},
            "spawn_target": [],
            "sea_level": 63,
            "disable_mob_generation": false,
            "aquifers_enabled": true,
            "ore_veins_enabled": true,
            "legacy_random_source": false
        })
    }

    type TagSpec<'a> = &'a [(&'a str, &'a [(&'a str, &'a [&'a str])])];

    fn tag_report(registries: TagSpec<'_>) -> TagLoadReport {
        TagLoadReport {
            registries: registries
                .iter()
                .map(|(registry_id, tags)| LoadedRegistryTags {
                    registry_id: (*registry_id).to_string(),
                    tags: tags
                        .iter()
                        .map(|(tag_id, members)| {
                            (
                                (*tag_id).to_string(),
                                members.iter().map(|member| (*member).to_string()).collect(),
                            )
                        })
                        .collect::<BTreeMap<_, _>>(),
                })
                .collect(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn native_availability_distinguishes_empty_getter_and_absent_registry() {
        use bbb_protocol::{PersistentRegistryResolver, PersistentRegistryTagMembers};
        let generation = generation(&[("minecraft:dialog", &[])]);
        let tags = TagLoadReport::default();
        let lookup = FrozenDatapackRegistryLookup::new(&generation, &tags);
        assert!(RegistryReferenceLookup::registry_available(
            &lookup,
            "minecraft:dialog"
        ));
        assert_eq!(
            lookup.element_membership("minecraft:dialog", "example:missing"),
            PersistentRegistryMembership::Absent
        );
        assert_eq!(
            lookup.tag_members("minecraft:dialog", "example:missing"),
            PersistentRegistryTagMembers::Absent
        );
        assert!(!RegistryReferenceLookup::registry_available(
            &lookup,
            "example:absent"
        ));
        assert_eq!(
            lookup.element_membership("example:absent", "example:missing"),
            PersistentRegistryMembership::Unavailable
        );
        assert_eq!(
            lookup.tag_members("example:absent", "example:missing"),
            PersistentRegistryTagMembers::Unavailable
        );
        assert!(RegistryReferenceLookup::registry_available(
            &lookup,
            "minecraft:map_decoration_type"
        ));
        assert_eq!(
            lookup.element_membership("minecraft:map_decoration_type", "minecraft:player"),
            PersistentRegistryMembership::Present
        );
        assert_eq!(
            lookup.element_membership("minecraft:map_decoration_type", "example:missing"),
            PersistentRegistryMembership::Absent
        );
    }

    #[test]
    fn current_generation_dynamic_membership_is_exact_and_does_not_follow_cycles() {
        let generation = generation(&[(
            "minecraft:dialog",
            &[
                ("example:self", json!({ "next": "example:self" })),
                ("example:left", json!({ "next": "example:right" })),
                ("example:right", json!({ "next": "example:left" })),
            ],
        )]);
        let tags = TagLoadReport::default();
        let lookup = FrozenDatapackRegistryLookup::new(&generation, &tags);

        assert!(lookup.contains_element("minecraft:dialog", "example:self"));
        assert!(lookup.contains_element("minecraft:dialog", "example:left"));
        assert!(lookup.contains_element("minecraft:dialog", "example:right"));
        assert!(!lookup.contains_element("minecraft:dialog", "example:missing"));
        assert!(!lookup.contains_element("example:absent_registry", "example:self"));
    }

    #[test]
    fn dynamic_membership_never_crosses_generations_or_falls_back_to_vanilla() {
        let old_generation =
            generation(&[("minecraft:dimension_type", &[("example:old", json!({}))])]);
        let new_generation =
            generation(&[("minecraft:dimension_type", &[("example:new", json!({}))])]);
        let tags = TagLoadReport::default();
        let old = FrozenDatapackRegistryLookup::new(&old_generation, &tags);
        let new = FrozenDatapackRegistryLookup::new(&new_generation, &tags);

        assert!(old.contains_element("minecraft:dimension_type", "example:old"));
        assert!(!old.contains_element("minecraft:dimension_type", "example:new"));
        assert!(new.contains_element("minecraft:dimension_type", "example:new"));
        assert!(!new.contains_element("minecraft:dimension_type", "example:old"));
        assert!(!new.contains_element("minecraft:dimension_type", "minecraft:overworld"));
    }

    #[test]
    fn fixed_membership_uses_only_the_exact_generated_vanilla_catalog() {
        let forged_generation = generation(&[
            ("minecraft:attribute", &[("example:forged", json!({}))]),
            (
                "minecraft:block_entity_type",
                &[("example:forged", json!({}))],
            ),
            ("minecraft:potion", &[("example:forged", json!({}))]),
            (
                "minecraft:map_decoration_type",
                &[("example:forged", json!({}))],
            ),
            ("minecraft:particle_type", &[("example:forged", json!({}))]),
            ("minecraft:sound_event", &[("example:forged", json!({}))]),
            ("minecraft:custom_stat", &[("example:forged", json!({}))]),
            ("minecraft:stat_type", &[("example:forged", json!({}))]),
        ]);
        let tags = TagLoadReport::default();
        let lookup = FrozenDatapackRegistryLookup::new(&forged_generation, &tags);

        assert!(lookup.contains_element("minecraft:item", "minecraft:stone"));
        assert!(lookup.contains_element("minecraft:sound_event", "minecraft:ambient.cave"));
        assert!(!lookup.contains_element("minecraft:item", "example:custom"));
        assert!(!lookup.contains_element("minecraft:sound_event", "example:forged"));

        for (registry, known) in [
            ("minecraft:attribute", "minecraft:max_health"),
            ("minecraft:block_entity_type", "minecraft:chest"),
            ("minecraft:custom_stat", "minecraft:jump"),
            ("minecraft:potion", "minecraft:awkward"),
            ("minecraft:map_decoration_type", "minecraft:mansion"),
            ("minecraft:stat_type", "minecraft:custom"),
        ] {
            assert!(built_in_registry_type(registry).is_some());
            assert_eq!(
                vanilla_persistent_registry_membership_26_1(registry, known),
                PersistentRegistryMembership::Present
            );
            assert_eq!(
                vanilla_persistent_registry_membership_26_1(registry, "minecraft:not_registered"),
                PersistentRegistryMembership::Absent
            );
            assert_eq!(
                vanilla_persistent_registry_membership_26_1(registry, "example:custom"),
                PersistentRegistryMembership::Absent
            );
            assert!(lookup.contains_element(registry, known));
            assert!(!lookup.contains_element(registry, "minecraft:not_registered"));
            assert!(!lookup.contains_element(registry, "example:forged"));
        }

        assert!(built_in_registry_type("minecraft:particle_type").is_some());
        assert_eq!(
            lookup.element_membership("minecraft:particle_type", "minecraft:block"),
            PersistentRegistryMembership::Present
        );
        assert!(!lookup.contains_element("minecraft:particle_type", "example:forged"));
    }

    #[test]
    fn tags_come_only_from_the_borrowed_dynamic_and_static_report() {
        let generation = generation(&[("minecraft:dialog", &[("example:a", json!({}))])]);
        let first_report = tag_report(&[
            (
                "minecraft:dialog",
                &[("example:dynamic", &["example:a"][..])],
            ),
            (
                "minecraft:item",
                &[("example:static", &["minecraft:stone"][..])],
            ),
        ]);
        let second_report =
            tag_report(&[("minecraft:dialog", &[("example:other", &["example:a"][..])])]);
        let first = FrozenDatapackRegistryLookup::new(&generation, &first_report);
        let second = FrozenDatapackRegistryLookup::new(&generation, &second_report);

        assert!(first.contains_tag("minecraft:dialog", "example:dynamic"));
        assert!(first.contains_tag("minecraft:item", "example:static"));
        assert!(!first.contains_tag("minecraft:dialog", "example:missing"));
        assert!(!first.contains_tag("minecraft:block", "minecraft:mineable/pickaxe"));
        assert!(!second.contains_tag("minecraft:dialog", "example:dynamic"));
        assert!(second.contains_tag("minecraft:dialog", "example:other"));
        assert!(!second.contains_tag("minecraft:item", "example:static"));
    }

    fn retained(
        id: &str,
        references: &[(&'static str, RegistryReferenceKind, &str)],
    ) -> RawRegistryElement {
        RawRegistryElement {
            id: id.to_string(),
            source_pack: "test".to_string(),
            source_json: b"{}".as_slice().into(),
            source: Some(
                bbb_registry_codec::decode_direct_registry_json(
                    &serde_json::to_vec(&(json!({}))).unwrap(),
                )
                .unwrap(),
            ),
            codec_value: Some(CanonicalRegistryCodecValue {
                network_nbt: bbb_protocol::NbtSummaryValue::Compound(Vec::new()),
                references: references
                    .iter()
                    .map(|(target, kind, resource_id)| OwnedRegistryCodecReference {
                        target_registry: target,
                        kind: *kind,
                        resource_id: RegistryResourceIdentifier::from_normalized(
                            (*resource_id).to_string(),
                        ),
                    })
                    .collect(),
            }),
        }
    }

    #[test]
    fn generation_freeze_accepts_present_element_and_tag_placeholders() {
        let generation = DatapackGeneration {
            registries: vec![LoadedDynamicRegistry {
                registry_id: "minecraft:worldgen/configured_feature".to_string(),
                elements: vec![retained(
                    "example:glow",
                    &[
                        (
                            "minecraft:block",
                            RegistryReferenceKind::Element,
                            "minecraft:stone",
                        ),
                        (
                            "minecraft:dialog",
                            RegistryReferenceKind::Element,
                            "example:present",
                        ),
                        (
                            "minecraft:dialog",
                            RegistryReferenceKind::Tag,
                            "example:present_tag",
                        ),
                    ],
                )],
            }],
            worldgen: Default::default(),
        };
        let tags = tag_report(&[(
            "minecraft:dialog",
            &[("example:present_tag", &["example:present"][..])],
        )]);
        assert!(validate_frozen_registry_codec_references(&generation, &tags).is_err());
        let with_target = DatapackGeneration {
            registries: vec![
                generation.registries[0].clone(),
                LoadedDynamicRegistry {
                    registry_id: "minecraft:dialog".to_string(),
                    elements: vec![RawRegistryElement {
                        id: "example:present".to_string(),
                        source_pack: "test".to_string(),
                        source_json: b"{}".as_slice().into(),
                        source: Some(
                            bbb_registry_codec::decode_direct_registry_json(
                                &serde_json::to_vec(&(json!({}))).unwrap(),
                            )
                            .unwrap(),
                        ),
                        codec_value: None,
                    }],
                },
            ],
            worldgen: Default::default(),
        };
        validate_frozen_registry_codec_references(&with_target, &tags).unwrap();

        let err =
            validate_frozen_registry_codec_references(&with_target, &TagLoadReport::default())
                .unwrap_err();
        assert_eq!(err.errors.len(), 1);
        assert!(
            err.errors[0]
                .message
                .contains("missing tag example:present_tag in minecraft:dialog"),
            "unexpected error: {}",
            err.errors[0].message
        );
    }

    #[test]
    fn generation_freeze_accepts_density_cycles_and_existing_noise_holders() {
        let generation = generation(&[
            (
                "minecraft:worldgen/density_function",
                &[
                    (
                        "example:left",
                        json!({"type": "minecraft:flat_cache", "argument": "example:right"}),
                    ),
                    (
                        "example:right",
                        json!({
                            "type": "minecraft:add",
                            "argument1": "example:left",
                            "argument2": {
                                "type": "minecraft:noise",
                                "noise": "example:present",
                                "xz_scale": 1.0,
                                "y_scale": 1.0
                            }
                        }),
                    ),
                ],
            ),
            (
                "minecraft:worldgen/noise",
                &[(
                    "example:present",
                    json!({"firstOctave": 0, "amplitudes": [1.0]}),
                )],
            ),
        ]);

        validate_frozen_registry_codec_references(&generation, &TagLoadReport::default()).unwrap();
        let mut missing_noise = generation.clone();
        missing_noise
            .registries
            .retain(|registry| registry.registry_id != "minecraft:worldgen/noise");
        assert!(validate_frozen_registry_codec_references(
            &missing_noise,
            &TagLoadReport::default()
        )
        .is_err());
    }

    #[test]
    fn surface_resource_keys_are_not_reinterpreted_as_noise_holders() {
        let mut settings = complete_noise_settings();
        settings["surface_rule"] = json!({
            "type": "minecraft:condition",
            "if_true": {
                "type": "minecraft:noise_threshold",
                "noise": "example:resource_key_only",
                "min_threshold": -1.0,
                "max_threshold": 1.0
            },
            "then_run": {"type": "minecraft:bandlands"}
        });
        let generation = generation(&[(
            "minecraft:worldgen/noise_settings",
            &[("example:surface", settings)],
        )]);

        validate_frozen_registry_codec_references(&generation, &TagLoadReport::default()).unwrap();
    }
}
