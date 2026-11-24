//! Canonical codec output paired with Vanilla loot-context usage facts.

use crate::{
    loot_context_types::VanillaLootContextParam,
    registry_codec::recursive_values::CanonicalRegistryCodecValue,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LootContextCodecValue {
    pub(super) canonical: CanonicalRegistryCodecValue,
    pub(super) referenced_context_params: Vec<VanillaLootContextParam>,
    /// Semantic `ConstantValue` fact retained independently from the NbtOps
    /// projection, which normalizes signed zero and cannot distinguish typed
    /// and inline providers on its own.
    pub(super) semantic_constant_value_bits: Option<u32>,
}

impl LootContextCodecValue {
    pub(super) fn new(canonical: CanonicalRegistryCodecValue) -> Self {
        Self {
            canonical,
            referenced_context_params: Vec::new(),
            semantic_constant_value_bits: None,
        }
    }

    pub(super) fn constant(canonical: CanonicalRegistryCodecValue, value: f32) -> Self {
        Self {
            canonical,
            referenced_context_params: Vec::new(),
            semantic_constant_value_bits: Some(value.to_bits()),
        }
    }

    pub(super) fn semantic_constant_value_bits(&self) -> Option<u32> {
        self.semantic_constant_value_bits
    }
}

pub(super) fn extend_context_params(
    params: &mut Vec<VanillaLootContextParam>,
    nested: impl IntoIterator<Item = VanillaLootContextParam>,
) {
    for param in nested {
        if !params.contains(&param) {
            params.push(param);
        }
    }
}
