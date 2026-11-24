//! Dedicated server gameplay and authoritative world runtime.

pub(crate) mod block_events;
pub(crate) mod chunk_sender;
pub(crate) mod command_block_execution;
pub mod connection;
pub(crate) mod entity_runtime_facts_26_1;
pub(crate) mod explosion;
pub(crate) mod fall_clip;
pub mod fluid;
pub mod level_event;
pub(crate) mod level_ticks;
pub(crate) mod movement_speed;
pub(crate) mod neighbor_updater;
pub mod random;
pub mod random_sequences;
pub mod registry_tags;
pub mod tick_rate;

pub mod block_behavior;
pub(crate) mod block_destroy;
pub(crate) mod block_entity;
pub(crate) mod block_getter;
pub mod chunk;
pub(crate) mod enchantment_combat_effects;
pub(crate) mod enchantment_entity_effects;
pub(crate) mod enchantment_facts;
pub(crate) mod enchantment_location_effects;
pub(crate) mod enchantment_weapon_effects;
pub mod entity;
pub(crate) mod equipment_drop_effects;
pub(crate) mod hit_block_effects;
pub mod inventory;
pub(crate) mod item_cooldowns;
pub(crate) mod item_durability;
pub(crate) mod jukebox_song_facts;
pub(crate) mod loot_runtime;
pub(crate) mod potion_brewing;
pub(crate) mod random_ticks;
pub(crate) mod redstone;
pub(crate) mod upgrade_data;
pub mod worldgen_shape;

pub mod admin;
pub mod commands;
pub mod mailbox;
pub mod plugin;
pub mod registry_sync;
pub mod world;
pub mod world_creation;

#[cfg(test)]
mod tests;
