use bbb_facts::{data_component_types, PersistentRegistryMembership, PersistentRegistryResolver};
use bbb_protocol::NbtSummaryValue;

pub use source_codec::decode_loot_holder_set_parameter_source;
pub use source_codec::{
    decode_item_stack_template_parameter_source,
    decode_item_stack_template_parameter_source_with_content_registry,
    decode_loot_book_parameter_source, decode_loot_component_parameter_source,
    decode_loot_component_parameter_source_with_content_registry,
    decode_loot_text_parameter_source, decode_loot_text_parameter_source_with_content_registry,
};
pub(crate) mod source_codec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistentNbtDecodeLimits {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_collection: usize,
}

impl PersistentNbtDecodeLimits {
    pub const DEFAULT: Self = Self {
        max_depth: 64,
        max_nodes: 65_536,
        max_collection: 4_096,
    };
}

impl Default for PersistentNbtDecodeLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Clone, Copy, Default)]
pub struct PersistentNbtDecodeContext<'a> {
    resolver: Option<&'a dyn PersistentRegistryResolver>,
}

impl<'a> PersistentNbtDecodeContext<'a> {
    pub fn with_resolver(resolver: &'a dyn PersistentRegistryResolver) -> Self {
        Self {
            resolver: Some(resolver),
        }
    }

    pub fn resolver(self) -> Option<&'a dyn PersistentRegistryResolver> {
        self.resolver
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PersistentComponentNbtError {
    #[error("unknown persistent component type id {type_id}")]
    Unknown { type_id: i32 },
    #[error("component {resource_id} ({type_id}) is transient")]
    Transient {
        type_id: i32,
        resource_id: &'static str,
    },
    #[error(
        "component resource mismatch for type {type_id}: expected {expected}, received {actual}"
    )]
    ResourceMismatch {
        type_id: i32,
        expected: &'static str,
        actual: String,
    },
    #[error("invalid persistent component NBT at {path}: {detail}")]
    Invalid { path: String, detail: String },
    #[error("registry {registry_id} is unavailable")]
    RegistryUnavailable { registry_id: String },
    #[error("registry {registry_id} is missing resource {resource_id}")]
    RegistryMissing {
        registry_id: String,
        resource_id: String,
    },
    #[error("persistent component codec coverage gap for {resource_id} ({type_id})")]
    CoverageGap {
        type_id: i32,
        resource_id: &'static str,
    },
    #[error("invalid Vanilla item prototype catalog: {detail}")]
    PrototypeCatalog { detail: String },
    #[error("invalid persistent item NBT structure: {detail}")]
    Structural { detail: String },
}

/// Validates one component value by reading it with the item reader. With a
/// resolver, a named holder the resolver reports absent is refused; without
/// one, names are taken as published.
pub fn validate_persistent_component_nbt(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
    context: PersistentNbtDecodeContext<'_>,
) -> Result<(), PersistentComponentNbtError> {
    read_component(type_id, resource_id, value, context).map(drop)
}

fn read_component(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
    context: PersistentNbtDecodeContext<'_>,
) -> Result<bbb_protocol::packets::DataComponentPatchSummary, PersistentComponentNbtError> {
    let component = component_type(type_id)?;
    if component.resource_id != resource_id {
        return Err(PersistentComponentNbtError::ResourceMismatch {
            type_id,
            expected: component.resource_id,
            actual: resource_id.to_string(),
        });
    }
    if !component.persistent {
        return Err(PersistentComponentNbtError::Transient {
            type_id,
            resource_id: component.resource_id,
        });
    }
    let unavailable = std::cell::RefCell::new(None);
    let holder_id = |registry: &'static str, name: &str| match context
        .resolver()
        .map(|resolver| resolver.resource_membership(registry, name))
    {
        Some(PersistentRegistryMembership::Absent) => None,
        Some(PersistentRegistryMembership::Unavailable) => {
            unavailable
                .borrow_mut()
                .get_or_insert_with(|| registry.to_string());
            None
        }
        _ => Some(stable_holder_id(registry, name)),
    };
    let read = crate::persistent_item_stack_projection::read_persistent_component(
        type_id,
        component.resource_id,
        value,
        &holder_id,
    );
    if let (Err(_), Some(registry_id)) = (&read, unavailable.into_inner()) {
        return Err(PersistentComponentNbtError::RegistryUnavailable { registry_id });
    }
    read.map_err(|error| match error {
        crate::ItemStackPersistentProjectionError::UnresolvedRegistryHolder {
            registry_id,
            resource_id,
        } => PersistentComponentNbtError::RegistryMissing {
            registry_id: registry_id.to_string(),
            resource_id,
        },
        crate::ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id,
        } => PersistentComponentNbtError::CoverageGap {
            type_id,
            resource_id,
        },
        error => PersistentComponentNbtError::Invalid {
            path: format!("$.components.{}", component.resource_id),
            detail: error.to_string(),
        },
    })
}

/// A holder id that only has to be equal for equal names, so two values read
/// without a publication compare as their names do.
fn stable_holder_id(registry: &str, name: &str) -> i32 {
    let mut hash = 0x811c_9dc5_u32;
    for byte in registry.bytes().chain([0]).chain(name.bytes()) {
        hash = (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193);
    }
    (hash & 0x7fff_ffff) as i32
}

pub(crate) fn prototype_component_value(
    item_type_id: i32,
    item_resource_id: &str,
    type_id: i32,
) -> Result<Option<NbtSummaryValue>, PersistentComponentNbtError> {
    let catalog = bbb_facts::vanilla_item_prototype_component_catalog().map_err(|error| {
        PersistentComponentNbtError::PrototypeCatalog {
            detail: error.to_string(),
        }
    })?;
    let prototype = catalog
        .component_codec_values(item_type_id)
        .ok_or_else(|| PersistentComponentNbtError::PrototypeCatalog {
            detail: format!("missing prototype row for item {item_resource_id} ({item_type_id})"),
        })?;
    prototype
        .get(&type_id)
        .map(bbb_protocol::json_value_to_nbt_summary)
        .transpose()
        .map_err(|error| PersistentComponentNbtError::PrototypeCatalog {
            detail: format!(
                "prototype value for item {item_resource_id} component {type_id} is not representable as NBT: {error}"
            ),
        })
}

fn component_type(
    type_id: i32,
) -> Result<&'static data_component_types::DataComponentTypeCatalogEntry, PersistentComponentNbtError>
{
    data_component_types::vanilla_data_component_type(type_id)
        .ok_or(PersistentComponentNbtError::Unknown { type_id })
}
