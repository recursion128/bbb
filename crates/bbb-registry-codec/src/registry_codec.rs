mod element_value;
pub(crate) mod source_nbt;

pub mod animal_variants;
mod biome_source;
pub(crate) mod block_state_source;
pub(crate) mod chat_type_source;
pub(crate) mod dialog_source;
pub(crate) mod dimension_type_source;
pub mod direct_leaf;
mod dispatcher;
pub(crate) mod enchantment_source;
pub(crate) mod environment_particle_source;
pub mod environment_source;
pub mod game_tests;
pub(crate) mod holders;
pub(crate) mod instrument_jukebox_source;
pub(crate) mod native_outcome;
pub(crate) mod painting_variant_source;
pub(crate) mod recursive_values;
pub mod sound_variants;
mod state_properties_source;
pub(crate) mod timeline_source;
pub(crate) mod trim_source;
pub(crate) mod value_provider_source;

pub use biome_source::{
    canonical_biome_environment_attributes,
    canonical_biome_environment_attributes_with_content_registry,
};
pub use block_state_source::{
    decode_block_state_id_nbt, decode_block_state_id_nbt_with_content_registry,
};
pub use enchantment_source::canonical_advancement_player_predicate_from_source;
pub use instrument_jukebox_source::{jukebox_song_registry_value, JukeboxSongRegistryValue};
pub use native_outcome::validate_registry_source_input;
pub use recursive_values::{CanonicalRegistryCodecValue, OwnedRegistryCodecReference};
pub use state_properties_source::canonical_state_properties_predicate_from_source;
pub use timeline_source::{
    canonical_direct_timeline, canonical_direct_timeline_with_content_registry,
};

pub use dimension_type_source::{
    canonical_direct_dimension_type, canonical_direct_dimension_type_with_content_registry,
    decode_direct_dimension_type_facts, decode_direct_dimension_type_facts_with_content_registry,
    DecodedDimensionType, DimensionCardinalLight, DimensionSkybox, DimensionTypeScalars,
    DirectDimensionTypeFacts,
};
pub use dispatcher::{
    decode_exact_synchronized_registry_element,
    decode_exact_synchronized_registry_element_with_content_registry,
    DecodedSynchronizedRegistryElement, SynchronizedRegistryElementValue,
};
pub use element_value::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_loaded_registry_json, decode_network_registry_element, parse_registry_json,
    RegistryElementData, RegistryElementDecodeError, RegistryElementSource,
    RegistryElementValueRef, RegistryJsonValue,
};

use bbb_facts::registry_data_types::synchronized_registry_data_type;
use bbb_protocol::normalize_resource_location;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{registry} {codec} {field}: {reason}")]
pub struct RegistryCodecError {
    pub registry: String,
    pub codec: String,
    pub field: String,
    pub reason: String,
    pub(crate) abort_speculation: bool,
}

impl RegistryCodecError {
    pub(crate) fn aborting_speculation(mut self) -> Self {
        self.abort_speculation = true;
        self
    }

    pub(crate) fn native_semantic(
        registry: &str,
        codec: &str,
        field: &str,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            registry: registry.to_string(),
            codec: codec.to_string(),
            field: field.to_string(),
            reason: reason.into(),
            abort_speculation: false,
        }
    }

    pub(crate) fn coverage(
        registry: &str,
        codec: &str,
        field: &str,
        reason: impl Into<String>,
    ) -> Self {
        Self::native_semantic(registry, codec, field, reason).aborting_speculation()
    }
}

/// One canonical resource identifier retained by a typed registry codec.
///
/// Syntax and the default namespace stay owned by the protocol's shared
/// resource-location parser rather than by each element codec family.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegistryResourceIdentifier(String);

impl RegistryResourceIdentifier {
    pub fn parse(raw: &str) -> Option<Self> {
        normalize_resource_location(raw).ok().map(Self)
    }

    pub(crate) fn parse_java_string(raw: &bbb_protocol::JavaString) -> Option<Self> {
        raw.with_utf8(Self::parse).flatten()
    }

    pub(crate) fn parse_owned(raw: String) -> Option<Self> {
        normalize_resource_location(raw).ok().map(Self)
    }

    /// Takes an identifier already normalized by its decoding owner.
    pub fn from_normalized(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

/// A reference retained by a typed registry codec until the complete
/// generation can validate membership. Decoders intentionally do not resolve
/// the target value: Vanilla creates registration placeholders before loading
/// elements, so named self- and mutual-cycles are legal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryCodecReference<'a> {
    pub target_registry: &'static str,
    pub kind: RegistryReferenceKind,
    pub resource_id: &'a RegistryResourceIdentifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryReferenceKind {
    Element,
    Tag,
}

pub use bbb_facts::PersistentRegistryMembership as RegistryReferenceMembership;

/// Frozen membership facts used immediately before a typed registry element
/// is encoded for publication. Decode retains placeholders so legal named
/// cycles do not recurse; encode must still prove that every placeholder and
/// tag belongs to the generation being published.
pub trait RegistryReferenceLookup {
    fn contains_element(&self, registry: &str, resource_id: &str) -> bool;
    fn contains_tag(&self, registry: &str, resource_id: &str) -> bool;

    /// Legacy membership views are complete; partial views must report missing getters.
    fn registry_available(&self, _registry: &str) -> bool {
        true
    }

    fn element_membership(&self, registry: &str, resource_id: &str) -> RegistryReferenceMembership {
        if self.contains_element(registry, resource_id) {
            RegistryReferenceMembership::Present
        } else {
            RegistryReferenceMembership::Absent
        }
    }

    fn tag_membership(&self, registry: &str, resource_id: &str) -> RegistryReferenceMembership {
        if self.contains_tag(registry, resource_id) {
            RegistryReferenceMembership::Present
        } else {
            RegistryReferenceMembership::Absent
        }
    }
}

pub(crate) fn registry_membership_present(
    membership: RegistryReferenceMembership,
    owner_registry: &str,
    codec: &str,
    field: &str,
    target_registry: &str,
) -> Result<bool, RegistryCodecError> {
    match membership {
        RegistryReferenceMembership::Present => Ok(true),
        RegistryReferenceMembership::Absent => Ok(false),
        RegistryReferenceMembership::Unavailable => Err(RegistryCodecError::native_semantic(
            owner_registry,
            codec,
            field,
            format!("registry {target_registry} is unavailable"),
        )
        .aborting_speculation()),
    }
}

pub fn validate_registry_codec_references<'a>(
    owner_registry: &str,
    codec: &str,
    references: impl IntoIterator<Item = RegistryCodecReference<'a>>,
    lookup: &(impl RegistryReferenceLookup + ?Sized),
) -> Result<(), RegistryCodecError> {
    for reference in references {
        let membership = match reference.kind {
            RegistryReferenceKind::Element => {
                lookup.element_membership(reference.target_registry, reference.resource_id.as_str())
            }
            RegistryReferenceKind::Tag => {
                lookup.tag_membership(reference.target_registry, reference.resource_id.as_str())
            }
        };
        let present = registry_membership_present(
            membership,
            owner_registry,
            codec,
            "<references>",
            reference.target_registry,
        )?;
        if !present {
            let kind = match reference.kind {
                RegistryReferenceKind::Element => "element",
                RegistryReferenceKind::Tag => "tag",
            };
            return Err(RegistryCodecError::native_semantic(
                owner_registry,
                codec,
                "<references>",
                format!(
                    "missing {kind} {} in {}",
                    reference.resource_id.as_str(),
                    reference.target_registry
                ),
            ));
        }
    }
    Ok(())
}

/// Returns the generated codec that Vanilla applies to this element origin.
///
/// Server datapack JSON is decoded by the persistent WORLDGEN codec. Client
/// known-pack JSON and network NBT are both decoded by the synchronized
/// network codec (`NetworkRegistryLoadTask`).
pub fn synchronized_registry_element_codec(
    registry: &str,
    source: RegistryElementSource,
) -> Result<&'static str, RegistryCodecError> {
    let data_type = synchronized_registry_data_type(registry).ok_or_else(|| {
        RegistryCodecError::native_semantic(
            registry,
            "RegistryDataLoader.catalog",
            "<registry>",
            "registry is missing from the generated synchronized catalog",
        )
        .aborting_speculation()
    })?;
    Ok(match source {
        RegistryElementSource::ServerDatapackJson => data_type.persistent_codec,
        RegistryElementSource::ClientKnownPackJson | RegistryElementSource::NetworkNbt => {
            data_type.network_codec
        }
    })
}

#[cfg(test)]
mod component_numeric;

#[cfg(test)]
mod component_numeric_families;

#[cfg(test)]
mod content_registry_plumbing_tests {
    use super::*;
    use crate::{
        decode_item_stack_template_parameter_source,
        decode_item_stack_template_parameter_source_with_content_registry,
        item_stack_summary_to_persistent_nbt_with_resolver, item_stack_template_from_source,
        read_item_stack, PersistentItemStackHolderResolver, PersistentNbtDecodeLimits,
        UnavailableHolderResolver,
    };
    use bbb_facts::{
        PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
    };
    use bbb_protocol::content_registry::{
        ContentRegistry, ContentRegistryBuilder, ModBlockRegistration, ModEntityRegistration,
        ModItemRegistration, ModParticleRegistration,
    };
    use bbb_protocol::environment_particles::ParticleOptionValue;
    use bbb_protocol::nbt_summary::entry;
    use bbb_protocol::NbtSummaryValue;

    struct ModLookup {
        registry: ContentRegistry,
    }

    impl PersistentRegistryResolver for ModLookup {
        fn resource_membership(
            &self,
            registry_id: &str,
            resource_id: &str,
        ) -> PersistentRegistryMembership {
            UnavailableHolderResolver.resource_membership(registry_id, resource_id)
        }
        fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
            UnavailableHolderResolver.tag_members(registry_id, tag_id)
        }
    }

    impl PersistentItemStackHolderResolver for ModLookup {
        fn holder_id(&self, _registry_id: &str, _resource_id: &str) -> Option<i32> {
            None
        }
        fn content_registry(&self) -> &ContentRegistry {
            &self.registry
        }
    }

    impl RegistryReferenceLookup for ModLookup {
        fn contains_element(&self, _registry: &str, _resource_id: &str) -> bool {
            true
        }
        fn contains_tag(&self, _registry: &str, _resource_id: &str) -> bool {
            true
        }
    }

    fn test_mod_registry() -> ContentRegistry {
        let mut builder = ContentRegistryBuilder::new();
        builder.register_item(ModItemRegistration {
            resource_id: "example:ruby_sword".to_string(),
            max_stack_size: 1,
            is_block_item: false,
        });
        builder.register_block(ModBlockRegistration {
            resource_id: "example:ruby_ore".to_string(),
            state_count: 1,
            light_emission: 0,
            destroy_speed: 3.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        builder.register_entity(ModEntityRegistration {
            resource_id: "example:ruby_golem".to_string(),
            ..ModEntityRegistration::default()
        });
        builder.register_particle(ModParticleRegistration {
            resource_id: "example:ruby_sparkle".to_string(),
            override_limiter: false,
        });
        builder.freeze_for_session().unwrap()
    }

    #[test]
    fn custom_content_registry_plumbs_through_item_projection_templates_particles_blocks_and_biomes(
    ) {
        let registry = test_mod_registry();
        let custom_item_id = registry
            .item_by_resource_id("example:ruby_sword")
            .unwrap()
            .id;
        let custom_block_state_id = registry
            .block_by_resource_id("example:ruby_ore")
            .unwrap()
            .first_state_id;
        let lookup = ModLookup {
            registry: registry.clone(),
        };

        // R9-15: `read_item_stack` and `item_stack_summary_to_persistent_nbt_with_resolver`
        // use `resolver.content_registry()` directly.
        let saved = NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("example:ruby_sword")),
            entry("count", NbtSummaryValue::Int(1)),
            entry(
                "components",
                NbtSummaryValue::Compound(vec![
                    entry(
                        "minecraft:use_remainder",
                        NbtSummaryValue::string("example:ruby_sword"),
                    ),
                    entry(
                        "minecraft:can_break",
                        NbtSummaryValue::Compound(vec![entry(
                            "blocks",
                            NbtSummaryValue::string("example:ruby_ore"),
                        )]),
                    ),
                ]),
            ),
        ]);
        assert!(read_item_stack(&saved, &UnavailableHolderResolver).is_err());
        let summary = read_item_stack(&saved, &lookup).unwrap();
        assert_eq!(summary.item_id, Some(custom_item_id));
        let reencoded =
            item_stack_summary_to_persistent_nbt_with_resolver(&summary, &lookup).unwrap();
        assert_eq!(read_item_stack(&reencoded, &lookup).unwrap(), summary);

        // R16-04: `item_stack_template_from_source` + `decode_item_stack_template_parameter_source`.
        let template_json =
            decode_direct_registry_json(br#"{"id":"example:ruby_sword","count":2}"#).unwrap();
        let template =
            item_stack_template_from_source(template_json.root(), template_json.source, &lookup)
                .unwrap();
        assert_eq!(template.item_id, custom_item_id);
        assert_eq!(template.count, 2);
        assert!(decode_item_stack_template_parameter_source(
            template_json.root(),
            template_json.source,
            PersistentNbtDecodeLimits::DEFAULT,
            PersistentNbtDecodeLimits::DEFAULT,
        )
        .is_err());
        assert!(
            decode_item_stack_template_parameter_source_with_content_registry(
                template_json.root(),
                template_json.source,
                PersistentNbtDecodeLimits::DEFAULT,
                PersistentNbtDecodeLimits::DEFAULT,
                &registry,
            )
            .is_ok()
        );

        // R16-04: `block_state_source`.
        let block_state_nbt = NbtSummaryValue::Compound(vec![entry(
            "Name",
            NbtSummaryValue::string("example:ruby_ore"),
        )]);
        assert!(decode_block_state_id_nbt(
            &block_state_nbt,
            "state",
            PersistentNbtDecodeLimits::DEFAULT
        )
        .is_err());
        assert_eq!(
            decode_block_state_id_nbt_with_content_registry(
                &block_state_nbt,
                "state",
                PersistentNbtDecodeLimits::DEFAULT,
                &registry,
            )
            .unwrap(),
            custom_block_state_id
        );

        // R16-04: `environment_particle_source` + `environment_source::particles`
        // for custom simple particle, custom block particle, and custom item particle.
        let simple_particle =
            decode_direct_registry_json(br#"{"type":"example:ruby_sparkle"}"#).unwrap();
        let parsed = environment_source::particles::particle_from_source(
            simple_particle.root(),
            simple_particle.source,
            PersistentNbtDecodeLimits::DEFAULT,
            Some(&lookup),
        )
        .unwrap();
        assert_eq!(parsed.resource_id, "example:ruby_sparkle");
        assert_eq!(parsed.option, ParticleOptionValue::Empty);

        let block_particle = decode_direct_registry_json(
            br#"{"type":"minecraft:block","block_state":{"Name":"example:ruby_ore"}}"#,
        )
        .unwrap();
        let parsed_block = environment_source::particles::particle_from_source(
            block_particle.root(),
            block_particle.source,
            PersistentNbtDecodeLimits::DEFAULT,
            Some(&lookup),
        )
        .unwrap();
        assert_eq!(
            parsed_block.option,
            ParticleOptionValue::Block {
                block_state_id: custom_block_state_id
            }
        );

        let item_particle = decode_direct_registry_json(
            br#"{"type":"minecraft:item","item":"example:ruby_sword"}"#,
        )
        .unwrap();
        let parsed_item = environment_source::particles::particle_from_source(
            item_particle.root(),
            item_particle.source,
            PersistentNbtDecodeLimits::DEFAULT,
            Some(&lookup),
        )
        .unwrap();
        assert!(matches!(
            parsed_item.option,
            ParticleOptionValue::Item { ref stack } if stack.item_id == Some(custom_item_id)
        ));

        // R16-04: `biome_source` + `decode_exact_synchronized_registry_element_with_content_registry`
        // with custom entity in spawners/spawn_costs and custom particle in attributes.
        let biome_json = decode_direct_registry_json(
            br#"{
                "has_precipitation": true,
                "temperature": 0.5,
                "downfall": 0.5,
                "attributes": {
                    "minecraft:visual/ambient_particles": [
                        {"particle": {"type": "example:ruby_sparkle"}, "probability": 0.01}
                    ]
                },
                "effects": {"water_color": 4159204},
                "carvers": [],
                "features": [],
                "spawners": {
                    "creature": [
                        {"type": "example:ruby_golem", "weight": 5, "minCount": 1, "maxCount": 2}
                    ]
                },
                "spawn_costs": {
                    "example:ruby_golem": {"energy_budget": 1.0, "charge": 0.5}
                }
            }"#,
        )
        .unwrap();
        assert!(decode_exact_synchronized_registry_element(
            "minecraft:worldgen/biome",
            &biome_json
        )
        .is_err());
        assert!(
            decode_exact_synchronized_registry_element_with_content_registry(
                "minecraft:worldgen/biome",
                &biome_json,
                &registry,
            )
            .is_ok()
        );
    }
}
