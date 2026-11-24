use bbb_protocol::{packets::DataComponentPatchSummary, NbtSummaryValue};
use bbb_registry_codec::{
    materialize_patch_named_holder_sets_with_resolver,
    read_persistent_component_into_with_resolver, PersistentItemStackHolderResolver,
};

use super::FrozenRegistryAccess;

pub(crate) fn read_persistent_component_into(
    type_id: i32,
    value: &NbtSummaryValue,
    access: Option<FrozenRegistryAccess<'_>>,
    patch: &mut DataComponentPatchSummary,
) -> Result<(), String> {
    let resolver = access
        .as_ref()
        .map(|access| access as &dyn PersistentItemStackHolderResolver);
    read_persistent_component_into_with_resolver(type_id, value, resolver, patch)
}

pub(crate) fn materialize_patch_named_holder_sets(
    patch: &mut DataComponentPatchSummary,
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<(), String> {
    let resolver = registries
        .as_ref()
        .map(|access| access as &dyn PersistentItemStackHolderResolver);
    materialize_patch_named_holder_sets_with_resolver(patch, resolver)
}
