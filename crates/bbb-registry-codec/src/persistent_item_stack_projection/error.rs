//! Every refusal this projection can return, and the constructor the
//! component transcoders use to name the component they refused.

/// Why a summary has no persistent `ItemStack.CODEC` encoding.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ItemStackPersistentProjectionError {
    /// `ItemStack.CODEC` cannot encode `ItemStack.EMPTY`: `Item.CODEC` refuses
    /// `minecraft:air` and the count range starts at one.
    /// Callers store the field conditionally the way
    /// `ItemEntity.addAdditionalSaveData` does, or
    /// use [`crate::optional_item_stack_summary_to_persistent_nbt`].
    #[error("empty item stacks have no ItemStack.CODEC encoding")]
    EmptyStack,
    #[error("item stack count {count} is outside the persistent range 1..=99")]
    CountOutOfRange { count: i32 },
    #[error("invalid vanilla item holder id {item_id}")]
    UnknownItem { item_id: i32 },
    #[error("duplicate or overlapping data component type id {type_id}")]
    DuplicateComponent { type_id: i32 },
    #[error("unknown data component type id {type_id}")]
    UnknownComponent { type_id: i32 },
    /// The patch declares a component bbb decoded into typed fields that no
    /// longer carry its codec NBT, so the save form cannot be produced without
    /// re-deriving it.
    #[error(
        "data component {resource_id} ({type_id}) has no retained persistent codec value in the summary"
    )]
    UnsupportedComponent {
        type_id: i32,
        resource_id: &'static str,
    },
    #[error("invalid persistent data component {resource_id} ({type_id}): {detail}")]
    InvalidPersistentComponent {
        type_id: i32,
        resource_id: &'static str,
        detail: String,
    },
    #[error("persistent {registry_id} holder {resource_id} has no published wire id")]
    UnresolvedRegistryHolder {
        registry_id: &'static str,
        resource_id: String,
    },
    #[error("persistent {registry_id} holder id {holder_id} has no published resource id")]
    UnresolvedRegistryHolderName {
        registry_id: &'static str,
        holder_id: i32,
    },
    #[error("projected data component patch is not wire-canonical: {detail}")]
    InvalidWireProjection { detail: String },
}

pub(super) type Result<T> = std::result::Result<T, ItemStackPersistentProjectionError>;

pub(super) fn invalid_persistent_component(
    type_id: i32,
    resource_id: &'static str,
    detail: String,
) -> ItemStackPersistentProjectionError {
    ItemStackPersistentProjectionError::InvalidPersistentComponent {
        type_id,
        resource_id,
        detail,
    }
}
