//! Shared server values, save contracts and geometry.

pub mod advancements;
pub mod block_queries;
pub mod block_state_parse;
pub mod bounding_box;
pub mod chunk_data;
pub mod chunk_ticks;
pub mod dimension_type;
pub mod directions;
pub mod environment;
pub mod experience;
pub mod fixture;
pub mod fluid;
pub mod food;
pub use bbb_facts::fuel_burn_times;
pub mod geometry;
pub mod interaction;
pub mod item_stack;
pub mod player_damage;
pub mod random_sequences_saved;
pub mod save;
pub mod schema_support;
pub mod sculk_cursor;
pub mod slot_ranges;
pub mod stats;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerWorldError {
    #[error("invalid server world configuration: {0}")]
    InvalidConfiguration(String),
    #[error("generated Vanilla catalog is missing required fixture fact: {0}")]
    MissingCatalogFact(&'static str),
}

pub type Result<T> = std::result::Result<T, ServerWorldError>;
