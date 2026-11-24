//! Java 26.1 dedicated-server datapack surface (`server-datapack-surface-26-1`).
//!
//! Vanilla anchors: `ServerPacksSource.java:24-82`, `WorldLoader.java:25-107`,
//! `RegistryDataLoader.java:83-162`, `MinecraftServer.java:1552-1610`.

pub mod advancement;
pub mod damage;
pub mod debug_generator;
pub mod dimension_type;
pub mod dynamic_registry;
pub mod enchantment;
pub mod environment_attributes;
pub mod flat_generator;
pub mod frozen_lookup;
pub mod function;
pub mod item_stack_template;
pub mod language;
pub mod loot;
pub mod mob_spawn;
pub mod pack;
pub mod pack_create;
pub mod pack_mcmeta;
pub mod pack_repository;
pub mod particle_options;
pub mod player_advancements;
pub mod recipe;
pub mod registry_data_loader;
pub mod resource_overlay;
pub mod startup;
pub mod structure_pool;
pub mod structure_template;
pub mod tag_loader;
pub mod trades;
pub mod vanilla_snapshot;
pub mod world_clock;
pub mod worldgen;
pub mod worldgen_codec;
