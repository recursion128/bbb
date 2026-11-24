use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::ItemStackTemplateSummary;

use crate::registry_codec::{
    RegistryCodecError, RegistryElementSource, RegistryElementValueRef, RegistryReferenceLookup,
};

use super::{
    nested_item_stack::project_item_stack_template, ItemStackPersistentProjectionError,
    PersistentItemStackHolderResolver,
};

/// Reads a datapack `ItemStackTemplate.CODEC` value (JSON or NBT) through the
/// same reader a saved stack uses, using `lookup.content_registry()`.
pub fn item_stack_template_from_source(
    input: RegistryElementValueRef<'_>,
    source: RegistryElementSource,
    lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
) -> Result<ItemStackTemplateSummary, RegistryCodecError> {
    item_stack_template_from_source_with_content_registry(
        input,
        source,
        lookup,
        lookup.content_registry(),
    )
}

/// Registry-aware form of [`item_stack_template_from_source`].
pub fn item_stack_template_from_source_with_content_registry(
    input: RegistryElementValueRef<'_>,
    _source: RegistryElementSource,
    lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
    content_registry: &ContentRegistry,
) -> Result<ItemStackTemplateSummary, RegistryCodecError> {
    let error = |reason: String| {
        RegistryCodecError::native_semantic(
            "minecraft:recipe",
            "ItemStackTemplate.CODEC",
            "result",
            reason,
        )
    };
    let value = crate::json_nbt::element_value_to_nbt(input).map_err(error)?;
    let template = project_item_stack_template(
        -1,
        "minecraft:recipe",
        &value,
        &|registry, resource| lookup.holder_id(registry, resource),
        content_registry,
        0,
        0,
    )
    .map_err(|projection| {
        let abort = matches!(
            &projection,
            ItemStackPersistentProjectionError::UnsupportedComponent { .. }
                | ItemStackPersistentProjectionError::UnresolvedRegistryHolder { .. }
                | ItemStackPersistentProjectionError::UnresolvedRegistryHolderName { .. }
        );
        let err = error(projection.to_string());
        if abort {
            err.aborting_speculation()
        } else {
            err
        }
    })?;
    bbb_protocol::packets::data_components::validate_persistent_data_component_patch_summary_with_registry(
        &template.component_patch,
        content_registry,
    )
    .map_err(|validation| error(validation.to_string()))?;
    Ok(template)
}
