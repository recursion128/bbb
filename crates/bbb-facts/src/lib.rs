//! Generated Vanilla 26.1 fact and catalog tables shared by the protocol,
//! registry-codec and world crates. Hand-written code lives downstream.

#[cfg(feature = "server-facts")]
mod bootstrap_registration_tags;
#[cfg(feature = "server-facts")]
mod entity_pick_results;
mod item_prototype_components;
mod lookup_tables;
mod mc_version;
mod persistent_registry_membership;

#[cfg(feature = "server-facts")]
pub use bootstrap_registration_tags::{
    vanilla_bootstrap_registration_tags, BootstrapRegistrationTagsError,
    VanillaBootstrapRegistrationTags,
};
#[cfg(feature = "server-facts")]
pub use entity_pick_results::{vanilla_entity_pick_result, VanillaEntityPickResult};
pub use item_prototype_components::{
    vanilla_item_prototype_component_catalog, ItemPrototypeComponentCatalogError,
    VanillaItemPrototypeComponentCatalog,
};
pub use mc_version::{
    mc_build_time_java_date_string, PackFormatVersion, MC_BUILD_TIME, MC_DATA_PACK_FORMAT,
    MC_DATA_VERSION, MC_DATA_VERSION_SERIES, MC_RESOURCE_PACK_FORMAT, MC_STABLE, MC_VERSION,
};
pub use persistent_registry_membership::{
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityModelTextureRef {
    pub path: &'static str,
    /// Logical model UV extent from the baked LayerDefinition. Pack images
    /// may use any positive physical resolution and are stretched across
    /// this extent just as Vanilla's NativeImageBackedTexture is.
    pub size: [u32; 2],
}

pub mod activity_types;
pub mod advancement_frame_types;
pub mod animal_sound_variant_types;
pub mod attribute_modifier_types;
pub mod attribute_types;
pub mod banner_pattern_types;
pub mod block_entity_types;
pub mod block_items;
pub mod block_sound_facts_26_1;
pub mod block_speed_factors;
pub mod block_state_types;
pub mod block_types;
pub mod boss_bar_styles;
pub mod boss_event_operation_types;
pub mod builtin_enum_component_values;
pub mod cat_variants;
pub mod chat_formattings;
pub mod client_command_actions;
pub mod client_information_options;
pub mod collision;
pub mod color;
pub mod consume_effect_types;
pub mod container_inputs;
pub mod custom_chat_completions_actions;
pub mod damage_types;
pub mod data_component_types;
pub mod debug_subscription_types;
pub mod default_attributes_26_1;
pub mod difficulties;
pub mod dye_colors;
pub mod enchantment_types;
#[cfg(feature = "server-facts")]
pub mod entity_ambient_sounds;
pub mod entity_anchors;
pub mod entity_data_serializers;
pub mod entity_poses;
pub mod entity_sound_sources;
pub mod entity_synched_data;
pub mod entity_types;
pub mod environment_attribute_types;
pub mod equipment_slots;
pub mod explosion;
pub mod filter_mask_types;
pub mod fluid;
pub mod fuel_burn_times;
pub mod game_event_types;
pub mod game_rules;
pub mod game_types;
pub mod heightmap_types;
pub mod interaction_hands;
pub mod item_component_value_types;
pub mod item_types;
pub mod jukebox_song_types;
pub mod map_decorations;
pub mod menu_types;
pub mod mob_effect_types;
pub mod movement;
pub mod note_block_instruments;
pub mod number_format_types;
pub mod objective_render_types;
pub mod painting_variant_types;
pub mod particle_types;
#[cfg(feature = "server-facts")]
pub mod persistent_registry_facts_26_1;
pub mod player_action_kinds;
pub mod player_command_actions;
pub mod player_info_actions;
pub mod player_model_parts;
pub mod player_names;
#[cfg(feature = "server-facts")]
pub mod poi_types;
pub mod potion_types;
pub mod recipe_wire_types;
pub mod registry_data_types;
pub mod remote_debug_sample_types;
pub mod resource_pack_response_actions;
pub mod scoreboard_display_slots;
pub mod scoreboard_methods;
pub mod scoreboard_team_options;
pub mod sound_event_types;
pub mod sound_sources;
pub mod stat_types;
pub mod static_registry_elements;
pub mod temperature_animal_variants;
pub mod tropical_fish_patterns;
pub mod villager_model_professions;
pub mod villager_model_types;
pub mod waypoint_wire_types;
pub mod wolf_variants;
pub mod world_clock_types;
pub mod zombie_nautilus_variants;

pub use banner_pattern_types::BannerPatternKind;
pub use cat_variants::CatModelVariant;
pub use dye_colors::{
    VanillaDyeColor, VanillaDyeColorEntry, VANILLA_DYE_COLORS, VANILLA_DYE_COLOR_COUNT,
    VANILLA_SHEEP_COLOR_DURATION_TICKS,
};
pub use map_decorations::{
    vanilla_map_decoration_type, vanilla_map_decoration_type_for_resource_id,
    vanilla_map_decoration_types, MapDecorationTypeCatalogEntry, VanillaMapDecorationType,
};
pub use temperature_animal_variants::{
    ChickenModelVariant, CowModelVariant, FrogModelVariant, PigModelVariant,
};
pub use villager_model_professions::VillagerModelProfession;
pub use villager_model_types::VillagerModelType;
pub use wolf_variants::WolfModelVariant;
pub use zombie_nautilus_variants::ZombieNautilusModelVariant;
