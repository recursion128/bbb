//! Exact source-aware `Enchantment.DIRECT_CODEC` owner and its recursive kernels.

mod criterion_player_source;
pub use criterion_player_source::canonical_advancement_player_predicate_from_source;

pub(super) mod block_predicate;

pub(super) mod block_state_provider;

mod effect_components;
pub(super) mod entity_effect;

pub(super) mod int_range;

pub(super) mod loot_condition_source;

pub(super) mod loot_context_value;

pub(super) mod number_provider;
mod root;

pub(super) mod scoreboard_name_provider;

pub(super) mod value;

pub(super) use root::decode_enchantment_element_with_content_registry;
