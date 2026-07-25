mod component;
mod nbt_summary;

pub use component::{
    canonical_component_codec_value, canonical_component_json_utf16_len,
    canonical_component_json_utf16_len_with_owner, canonical_f32_json_utf16_len,
    canonical_f64_json_utf16_len, canonical_json_string_utf16_len, canonical_nbt_json_utf16_len,
    canonical_resolvable_profile_json_utf16_len, component_node_style_from_nbt_value,
    component_style_from_nbt_value, decode_sign_block_entity_nbt, decode_styled_component_summary,
    java_float_to_string, parse_command_coordinates_prefix, parse_command_entity_selector_prefix,
    parse_command_nbt_path_prefix, parse_command_snbt_prefix, parse_command_snbt_value_prefix,
    parse_command_uuid_prefix, styled_component_runs_from_nbt_value, styled_runs_summary_text,
    validate_component_nbt, visit_component_payloads, CanonicalComponentCodecValue,
    CanonicalComponentOwnerValue, CommandCoordinatesKind, CommandEntitySelectorOption,
    CommandEntitySelectorOptionKind, CommandEntitySelectorPrefix, CommandSnbtKind,
    ComponentClickEvent, ComponentHoverEvent, ComponentPayload, ComponentStyle, RichTextComponent,
    SignBlockEntityNbt, SignTextNbt, StyledTextRun, VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS,
};
pub use nbt_summary::{
    canonical_snbt_string, decode_nbt_summary, parse_snbt_compound, NbtSummaryEntry,
    NbtSummaryValue,
};

pub mod advancement_frame_types;
pub mod attribute_modifier_types;
pub mod attribute_types;
pub mod block_entity_types;
pub mod block_state_types;
pub mod block_types;
pub mod boss_bar_styles;
pub mod boss_event_operation_types;
pub mod builtin_enum_component_values;
pub mod chat_formattings;
pub mod client_command_actions;
pub mod client_information_options;
pub mod codec;
pub mod consume_effect_types;
pub mod container_inputs;
pub mod custom_chat_completions_actions;
pub mod damage_types;
pub mod data_component_types;
pub mod debug_subscription_types;
pub mod difficulties;
pub mod entity_anchors;
pub mod entity_data_serializers;
pub mod entity_poses;
pub mod entity_types;
pub mod equipment_slots;
pub mod filter_mask_types;
pub mod frame;
pub mod game_event_types;
pub mod game_rules;
pub mod game_types;
pub mod heightmap_types;
pub mod ids;
pub mod interaction_hands;
pub mod item_component_coverage;
pub mod item_component_value_types;
pub mod item_types;
pub mod menu_types;
pub mod mob_effect_types;
pub mod number_format_types;
pub mod objective_render_types;
pub mod packet_catalog;
pub mod packet_coverage;
pub mod packets;
pub mod particle_types;
pub mod player_action_kinds;
pub mod player_command_actions;
pub mod player_info_actions;
pub mod potion_types;
pub mod recipe_wire_types;
pub mod registry_data_types;
pub mod registry_holder_consumers;
pub mod registry_holder_coverage;
pub mod remote_debug_sample_types;
pub mod resource_pack_response_actions;
pub mod scoreboard_display_slots;
pub mod scoreboard_methods;
pub mod scoreboard_team_options;
pub mod sound_event_types;
pub mod sound_sources;
pub mod stat_types;
pub mod tropical_fish_patterns;
pub mod waypoint_wire_types;
pub mod world_clock_types;

pub const MC_VERSION: &str = "26.1";
pub const PROTOCOL_VERSION: i32 = 775;
pub const MC_DATA_VERSION: i32 = 4786;
pub const MC_DATA_VERSION_SERIES: &str = "main";
pub const MC_BUILD_TIME: &str = "2026-03-24T12:09:43+00:00";
pub const MC_RESOURCE_PACK_FORMAT: PackFormatVersion = PackFormatVersion::new(84, 0);
pub const MC_DATA_PACK_FORMAT: PackFormatVersion = PackFormatVersion::new(101, 1);
pub const MC_STABLE: bool = true;

/// Applies the same namespace/path validation and default namespace used by
/// packet resource locations without requiring a wire decoder.
pub fn normalize_resource_location(raw: String) -> codec::Result<String> {
    packets::normalize_resource_location(raw)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackFormatVersion {
    pub major: u32,
    pub minor: u32,
}

impl PackFormatVersion {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub fn to_vanilla_string(self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn version_constants_match_local_vanilla_version_json() {
        let version_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../mc-code/sources")
            .join(MC_VERSION)
            .join("version.json");
        if !version_path.exists() {
            eprintln!(
                "skipping local vanilla version.json check; missing {}",
                version_path.display()
            );
            return;
        }

        let raw = std::fs::read_to_string(&version_path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();

        assert_eq!(value["id"].as_str(), Some(MC_VERSION));
        assert_eq!(value["name"].as_str(), Some(MC_VERSION));
        assert_eq!(
            value["world_version"].as_i64(),
            Some(MC_DATA_VERSION.into())
        );
        assert_eq!(value["series_id"].as_str(), Some(MC_DATA_VERSION_SERIES));
        assert_eq!(
            value["protocol_version"].as_i64(),
            Some(PROTOCOL_VERSION.into())
        );
        assert_eq!(
            value["pack_version"]["resource_major"].as_u64(),
            Some(MC_RESOURCE_PACK_FORMAT.major.into())
        );
        assert_eq!(
            value["pack_version"]["resource_minor"].as_u64(),
            Some(MC_RESOURCE_PACK_FORMAT.minor.into())
        );
        assert_eq!(
            value["pack_version"]["data_major"].as_u64(),
            Some(MC_DATA_PACK_FORMAT.major.into())
        );
        assert_eq!(
            value["pack_version"]["data_minor"].as_u64(),
            Some(MC_DATA_PACK_FORMAT.minor.into())
        );
        assert_eq!(value["build_time"].as_str(), Some(MC_BUILD_TIME));
        assert_eq!(value["stable"].as_bool(), Some(MC_STABLE));
    }

    #[test]
    fn pack_format_version_string_matches_vanilla_to_string() {
        assert_eq!(MC_RESOURCE_PACK_FORMAT.to_vanilla_string(), "84.0");
        assert_eq!(MC_DATA_PACK_FORMAT.to_vanilla_string(), "101.1");
    }
}
