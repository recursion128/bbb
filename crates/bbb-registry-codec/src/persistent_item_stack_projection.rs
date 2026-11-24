//! Persistent `ItemStack.CODEC` NBT reader and writer for
//! [`bbb_protocol::packets::ItemStackSummary`].
//!
//! Fails closed on unreadable components, resolves registry holders to
//! publication ids (keeping canonical sorted order), and delegates verbatim-NBT
//! components to [`PERSISTENT_CODEC_BACKED_COMPONENT_TYPE_IDS`].

mod attribute_modifier_components;
pub mod book_components;
mod builtin_enum_components;
mod component_matchers;
mod component_sets;
pub mod consumable_components;
mod embedded_record_components;
pub mod encoding;
mod equippable_components;
mod error;
pub mod firework_components;
mod holder;
mod holder_component_projection;
mod item_stack;
mod mob_effect_details;
mod nbt_support;
mod nested_item_stack;
mod persistent_to_wire_components;
pub mod potion_contents;
mod profile_components;
mod simple_record_components;
mod source;
mod tag_and_data_components;
mod variant_components;
pub mod weapon_and_block_attacks;
mod wire_to_persistent_components;

pub use component_sets::{
    PERSISTENT_CODEC_BACKED_COMPONENT_TYPE_IDS, TYPED_PERSISTENT_ENCODED_COMPONENT_TYPE_IDS,
};
pub use embedded_record_components::read_lodestone_tracker;
pub use error::ItemStackPersistentProjectionError;
pub use holder::{
    project_sound_event as read_sound_event, PersistentItemStackHolderResolver,
    RegistryAccessRevision, UnavailableHolderResolver,
};
pub(crate) use item_stack::{canonical_persistent_component, read_persistent_component};
pub use item_stack::{
    data_component_patch_to_persistent_nbt_with_resolver,
    data_component_patch_to_persistent_nbt_with_resolver_and_content_registry, item_component_nbt,
    item_component_nbt_with_content_registry, item_stack_summary_is_empty,
    item_stack_summary_to_persistent_nbt, item_stack_summary_to_persistent_nbt_with_resolver,
    item_stack_summary_to_persistent_nbt_with_resolver_and_content_registry, read_component_patch,
    read_component_patch_into, read_component_patch_into_with_content_registry,
    read_component_patch_with_content_registry, read_item_component, read_item_component_into,
    read_item_component_into_with_content_registry, read_item_component_with_content_registry,
    read_item_stack, read_item_stack_with_content_registry,
};
pub use nested_item_stack::{
    read_item_stack_template_fields, read_item_stack_template_fields_with_content_registry,
    ItemStackTemplateFields,
};
pub use persistent_to_wire_components::validate_persistent_scalar_component;
pub(crate) use profile_components::canonical_profile_source_nbt;
pub use source::{
    item_stack_template_from_source, item_stack_template_from_source_with_content_registry,
};

#[cfg(test)]
mod tests;
