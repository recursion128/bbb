mod java_string;
mod spin_lock;
pub use java_string::{
    code_of_conduct_text_hash, java_char_is_letter_or_digit, java_character_is_whitespace,
    java_string_hash_code, serialize_java_string_map, to_display_json_value, JavaString,
};
pub mod component;
pub mod environment_audio;
pub mod environment_particles;
pub mod java_number;
pub mod nbt_summary;
#[cfg(test)]
mod test_stack_budget;

pub use component::{
    canonical_component_codec_value, collapsed_component_nbt, command_item_predicate_component,
    command_known_item_predicate, command_nbt_is_too_deep, command_persistent_data_component,
    command_resource_selector_matches, command_stat_key_for_resource_location,
    command_syntax_error, command_utf16_byte_cursor, component_appended_nbt,
    component_codec_values_equal, component_hover_uuid, component_hover_uuid_nbt,
    component_keybind_nbt, component_literal_nbt, component_node_style_from_nbt_value,
    component_style_from_nbt_value, component_style_from_validated_nbt_value,
    component_styled_literal_nbt, component_translatable_nbt, component_with_siblings_nbt,
    decode_sign_block_entity_nbt, decode_styled_component_summary,
    decode_styled_json_component_summary, java_double_to_string, java_float_to_string,
    merge_command_nbt_compound, parse_command_coordinates_prefix,
    parse_command_coordinates_prefix_utf16, parse_command_entity_selector_prefix,
    parse_command_entity_selector_prefix_utf16, parse_command_java_uuid,
    parse_command_nbt_path_prefix, parse_command_nbt_path_value_prefix,
    parse_command_nbt_path_value_prefix_utf16, parse_command_snbt_prefix,
    parse_command_snbt_value_prefix, parse_command_snbt_value_prefix_utf16,
    parse_command_uuid_prefix, parse_command_uuid_prefix_utf16, read_command_string_value,
    read_command_string_value_utf16, scan_command_angle_argument,
    scan_command_angle_argument_utf16, scan_command_block_body, scan_command_block_body_utf16,
    scan_command_block_body_with_validation, scan_command_block_identifier,
    scan_command_block_identifier_utf16, scan_command_catalog_argument_utf16,
    scan_command_color_argument, scan_command_context_free_argument,
    scan_command_context_free_argument_utf16, scan_command_coordinates_argument,
    scan_command_coordinates_argument_utf16, scan_command_degrees_range_argument_utf16,
    scan_command_entity_anchor_argument, scan_command_entity_argument,
    scan_command_float_range_argument, scan_command_game_mode_argument,
    scan_command_game_profile_argument, scan_command_game_profile_argument_utf16,
    scan_command_heightmap_argument, scan_command_hex_color_argument,
    scan_command_hex_color_argument_utf16, scan_command_int_range_argument,
    scan_command_item_predicate_head, scan_command_item_predicate_head_utf16,
    scan_command_item_slot_argument, scan_command_nbt_path_argument, scan_command_number_lexeme,
    scan_command_number_lexeme_utf16, scan_command_operation_argument,
    scan_command_operation_argument_utf16, scan_command_particle_options,
    scan_command_particle_options_utf16, scan_command_quotable_string,
    scan_command_quotable_string_utf16, scan_command_range_argument_utf16,
    scan_command_resource_location, scan_command_resource_location_argument,
    scan_command_resource_location_argument_utf16, scan_command_resource_location_utf16,
    scan_command_resource_or_id, scan_command_resource_or_id_utf16, scan_command_resource_selector,
    scan_command_resource_selector_utf16, scan_command_score_holder_argument,
    scan_command_scoreboard_slot_argument, scan_command_snbt_argument,
    scan_command_snbt_argument_utf16, scan_command_swizzle_argument,
    scan_command_swizzle_argument_utf16, scan_command_template_mirror_argument,
    scan_command_template_rotation_argument, scan_command_text_argument,
    scan_command_text_argument_utf16, scan_command_time_argument, scan_command_time_argument_utf16,
    scan_command_token, scan_command_token_utf16, scan_command_unquoted_string,
    scan_command_unquoted_string_utf16, scan_command_uuid_argument,
    scan_command_uuid_argument_utf16, scan_command_whitespace, scan_command_whitespace_utf16,
    styled_component_runs_from_nbt_value, styled_runs_summary_text, validate_chat_string_units,
    validate_component_nbt, validate_component_nbt_with_registry, validate_url,
    visit_component_payloads, visit_component_payloads_with_registry, CanonicalComponentCodecValue,
    CanonicalComponentOwnerValue, CommandCoordinatesKind, CommandEntitySelectorOption,
    CommandEntitySelectorOptionKind, CommandEntitySelectorPrefix, CommandItemComponentPatchScanner,
    CommandItemComponentPatchScannerUtf16, CommandItemComponentSuggestions,
    CommandItemPredicateSuggestions, CommandItemPredicateTestScanner,
    CommandItemPredicateTestScannerUtf16, CommandNbtPath, CommandNbtPathErrorKind,
    CommandNbtPathNode, CommandNbtPathNodeKind, CommandNbtPathParseError, CommandSnbtKind,
    CommandSyntaxError, CommandSyntaxErrorKind, CommandSyntaxErrorUtf16, ComponentClickEvent,
    ComponentHoverEvent, ComponentPayload, ComponentStyle, RichTextComponent, RichTextStyle,
    ScannedCommandBlockBody, ScannedCommandBlockBodyUtf16, ScannedCommandBlockIdentifier,
    ScannedCommandBlockIdentifierUtf16, ScannedCommandItemComponent,
    ScannedCommandItemComponentUtf16, ScannedCommandItemPredicateHead,
    ScannedCommandItemPredicateHeadUtf16, ScannedCommandItemPredicateTerm,
    ScannedCommandItemPredicateTermUtf16, ScannedCommandItemPredicateTest,
    ScannedCommandItemPredicateTestUtf16, ScannedCommandParticleOptions,
    ScannedCommandParticleOptionsUtf16, ScannedCommandResourceOrId,
    ScannedCommandResourceOrIdUtf16, SignBlockEntityNbt, SignTextNbt, StyledTextRun,
    VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS,
};
// Facts re-exported for downstream protocol/client consumers.
pub use bbb_facts::{
    animal_sound_variant_types, attribute_modifier_types, attribute_types, block_entity_types,
    block_items, block_sound_facts_26_1, block_speed_factors, block_state_types, block_types,
    builtin_enum_component_values, chat_formattings, damage_types, data_component_types,
    debug_subscription_types, difficulties, enchantment_types, entity_anchors,
    entity_data_serializers, entity_poses, entity_synched_data, entity_types,
    environment_attribute_types, equipment_slots, game_event_types, game_rules, game_types,
    heightmap_types, item_component_value_types, item_types, jukebox_song_types, menu_types,
    mob_effect_types, note_block_instruments, objective_render_types, painting_variant_types,
    particle_types, player_command_actions, player_info_actions, player_model_parts, player_names,
    potion_types, recipe_wire_types, registry_data_types, resource_pack_response_actions,
    scoreboard_display_slots, sound_event_types, stat_types, static_registry_elements,
    tropical_fish_patterns, world_clock_types,
};
pub use bbb_facts::{
    vanilla_item_prototype_component_catalog, PersistentRegistryMembership,
    PersistentRegistryResolver, PersistentRegistryTagMembers, MC_BUILD_TIME, MC_DATA_PACK_FORMAT,
    MC_DATA_VERSION, MC_DATA_VERSION_SERIES, MC_RESOURCE_PACK_FORMAT, MC_STABLE, MC_VERSION,
};
pub use nbt_summary::{
    canonical_snbt_java_string, canonical_snbt_string, decode_file_nbt_summary,
    decode_file_nbt_summary_prefix, decode_file_saved_nbt_summary, decode_named_nbt_summary,
    decode_named_saved_nbt_summary, decode_nbt_summary, decode_nbt_summary_with_limits,
    encode_named_nbt_summary, encode_named_nbt_summary_java_string,
    encode_named_nbt_summary_java_string_saved_file, encode_named_nbt_summary_saved_file,
    encode_nbt_summary, encode_nbt_summary_network_quota, json_value_to_nbt_summary,
    nbt_summary_to_json_value, parse_snbt_compound, parse_snbt_compound_prefix,
    parse_snbt_compound_prefix_utf16, parse_snbt_compound_utf16, parse_snbt_prefix_utf16,
    NbtSummaryDecodeLimits, NbtSummaryEntry, NbtSummaryValue, SnbtParseFailure,
    VANILLA_DEFAULT_NBT_MAX_DEPTH, VANILLA_DEFAULT_NBT_QUOTA,
};

pub mod chunk_light;
pub mod codec;
pub mod content_registry;
pub mod entity_body_rotation;
pub mod frame;
pub mod ids;
pub mod packet_catalog;
pub mod packets;
pub mod profile_resolution;
pub mod services_rsa;

pub use frame::{PacketDecoder, PacketEncoder};
pub use profile_resolution::{
    is_dynamic_resolvable_profile, profile_texture_handle, vanilla_profile_texture_cache_hash,
    vanilla_profile_texture_hash, DynamicPlayerTextureKind, DynamicTextureFetchKind,
    DynamicTextureFetchRequest, DynamicTextureFetchResult, ProfileResolutionKey,
    ProfileResolutionResult, ResolvedGameProfile, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
    MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES, MAX_PROFILE_TEXTURE_URL_BYTES,
};
pub use services_rsa::{
    parse_cached_services_rsa_public_key, verify_services_rsa_sha1_signature, ServicesRsaPublicKey,
};

pub const PROTOCOL_VERSION: i32 = 775;

/// Applies the same namespace/path validation and default namespace used by
/// packet resource locations without requiring a wire decoder.
pub use packets::normalize_resource_location;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_matches_local_vanilla_version_json() {
        let Some(root) = std::env::var_os("BBB_VANILLA_ROOT") else {
            eprintln!("skipping local vanilla version.json check; BBB_VANILLA_ROOT is unset");
            return;
        };
        let version_path = std::path::PathBuf::from(root)
            .join("sources")
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
        assert_eq!(
            value["protocol_version"].as_i64(),
            Some(PROTOCOL_VERSION.into())
        );
    }
}
