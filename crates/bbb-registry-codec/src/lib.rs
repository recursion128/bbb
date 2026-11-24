//! Registry, persistent-NBT and item-stack codecs over the shared facts and
//! the Component/NBT primitives of bbb-protocol.

pub mod component_select;
pub(crate) mod component_source_codec;
pub mod item_hash;
mod json_nbt;
pub mod pack_filter;
pub use item_hash::{
    hash_component_value, hashed_component_patch_from_summary, hashed_patch_matches,
    hashed_stack_from_summary, hashed_stack_matches, ItemHashLookup, VanillaHashLookup,
};
pub use json_nbt::json_to_nbt;
mod persistent_component_nbt;
mod persistent_item_nbt;
pub mod persistent_item_stack_projection;

pub use component_select::{
    decode_persistent_component_select_value_from_nbt_with_resolver,
    decode_persistent_component_select_value_from_source_with_resolver,
    decode_persistent_component_select_value_with_resolver,
    materialize_patch_named_holder_sets_with_resolver,
    read_persistent_component_into_with_resolver,
};
pub use persistent_component_nbt::decode_loot_holder_set_parameter_source;

pub use persistent_component_nbt::{
    decode_item_stack_template_parameter_source,
    decode_item_stack_template_parameter_source_with_content_registry,
    decode_loot_book_parameter_source, decode_loot_component_parameter_source,
    decode_loot_component_parameter_source_with_content_registry,
    decode_loot_text_parameter_source, decode_loot_text_parameter_source_with_content_registry,
};

pub use persistent_component_nbt::{
    validate_persistent_component_nbt, PersistentComponentNbtError, PersistentNbtDecodeContext,
    PersistentNbtDecodeLimits,
};
pub use persistent_item_stack_projection::{
    data_component_patch_to_persistent_nbt_with_resolver,
    data_component_patch_to_persistent_nbt_with_resolver_and_content_registry, item_component_nbt,
    item_component_nbt_with_content_registry, item_stack_summary_is_empty,
    item_stack_summary_to_persistent_nbt, item_stack_summary_to_persistent_nbt_with_resolver,
    item_stack_summary_to_persistent_nbt_with_resolver_and_content_registry,
    item_stack_template_from_source, item_stack_template_from_source_with_content_registry,
    read_component_patch, read_component_patch_into,
    read_component_patch_into_with_content_registry, read_component_patch_with_content_registry,
    read_item_component, read_item_component_into, read_item_component_into_with_content_registry,
    read_item_component_with_content_registry, read_item_stack, read_item_stack_template_fields,
    read_item_stack_template_fields_with_content_registry, read_item_stack_with_content_registry,
    validate_persistent_scalar_component, ItemStackPersistentProjectionError,
    PersistentItemStackHolderResolver, RegistryAccessRevision, UnavailableHolderResolver,
};
pub use registry_codec::value_provider_source::{FloatProvider, IntProvider, WeightedIntProvider};
pub use registry_codec::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_exact_synchronized_registry_element,
    decode_exact_synchronized_registry_element_with_content_registry, decode_loaded_registry_json,
    decode_network_registry_element, parse_registry_json, synchronized_registry_element_codec,
    validate_registry_source_input, RegistryCodecError, RegistryElementData,
    RegistryElementDecodeError, RegistryElementSource, RegistryElementValueRef, RegistryJsonValue,
};

pub mod biome_climate;
pub mod codec_dispatch_types;
pub(crate) mod enchantment_codec_types;
pub mod enchantment_value_effect;
pub mod legacy_random;
pub mod level_based_value;
/// The generated `LootContextParams` / `LootContextParamSets` catalog. It is
/// `pub` because `bbb-server-block`'s loot evaluator binds a table to one of
/// these sets and must not keep a second, hand-written copy of the table.
pub mod loot_context_types;
pub mod random_source;
pub mod registry_codec;
pub mod xoroshiro_random;
