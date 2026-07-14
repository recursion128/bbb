use bbb_protocol::{
    canonical_component_json_utf16_len,
    data_component_types::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID, NbtSummaryEntry,
    NbtSummaryValue,
};

use std::collections::BTreeMap;

use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet, RegistryTagState};

use super::super::{decode_supported_component_value, DialogDataComponentValueState};
use super::*;

#[test]
fn decodes_written_book_fields_defaults_and_filterable_alternatives() {
    let styled = compound(vec![
        string_entry("text", "Filtered"),
        value_entry("italic", NbtSummaryValue::Byte(1)),
    ]);
    let value = written_book(vec![
        string_entry("title", "A title"),
        string_entry("author", "An author"),
        value_entry("generation", NbtSummaryValue::Byte(2)),
        value_entry(
            "pages",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::String("Compact page".to_string()),
                compound(vec![
                    value_entry("raw", compound(vec![string_entry("text", "Raw")])),
                    value_entry("filtered", styled.clone()),
                ]),
            ]),
        ),
        value_entry("resolved", NbtSummaryValue::Byte(1)),
    ]);
    let content = decode_written_book_content(&value, "written book").unwrap();
    assert_eq!(content.title.raw, "A title");
    assert_eq!(content.title.filtered, None);
    assert_eq!(content.author, "An author");
    assert_eq!(content.generation, 2);
    assert!(content.resolved);
    assert_eq!(content.pages.len(), 2);
    assert_eq!(content.pages[0].raw.text, "Compact page");
    assert_eq!(content.pages[0].filtered, None);
    assert_eq!(content.pages[1].raw.text, "Raw");
    assert_eq!(
        content.pages[1]
            .filtered
            .as_ref()
            .map(|page| &page.component),
        Some(&styled)
    );

    assert_eq!(
        decode_supported_component_value(
            VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
            "minecraft:written_book_content",
            &value,
            None,
        )
        .unwrap(),
        Some(DialogDataComponentValueState::WrittenBookContent { content })
    );

    let title = compound(vec![
        string_entry("raw", "Raw title"),
        string_entry("filtered", "Filtered title"),
    ]);
    let defaults = decode_written_book_content(
        &written_book(vec![
            value_entry("title", title),
            string_entry("author", "Author"),
        ]),
        "written book",
    )
    .unwrap();
    assert_eq!(defaults.title.raw, "Raw title");
    assert_eq!(defaults.title.filtered.as_deref(), Some("Filtered title"));
    assert_eq!(defaults.generation, 0);
    assert!(defaults.pages.is_empty());
    assert!(!defaults.resolved);
}

#[test]
fn accepts_written_book_title_page_and_collection_boundaries() {
    let title = "😀".repeat(16);
    let pages = vec![NbtSummaryValue::String(String::new()); 101];
    let content = decode_written_book_content(
        &written_book(vec![
            string_entry("title", &title),
            string_entry("author", "Author"),
            value_entry("generation", NbtSummaryValue::Int(3)),
            value_entry("pages", NbtSummaryValue::List(pages)),
        ]),
        "written book",
    )
    .unwrap();
    assert_eq!(content.title.raw, title);
    assert_eq!(content.pages.len(), 101);
    assert_eq!(content.generation, 3);

    for page in [
        "x".repeat(32_765),
        "\"".repeat(16_382),
        "\u{2028}".repeat(5_460),
        "😀".repeat(16_382),
    ] {
        assert!(decode_written_book_content(
            &book_with_pages(vec![NbtSummaryValue::String(page)]),
            "written book",
        )
        .is_ok());
    }

    let exact_structured_page = compound(vec![
        string_entry("text", &"x".repeat(32_744)),
        value_entry("bold", NbtSummaryValue::Byte(1)),
    ]);
    assert_eq!(
        canonical_component_json_utf16_len(&exact_structured_page).unwrap(),
        Some(32_767)
    );
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_structured_page]),
        "written book",
    )
    .is_ok());

    let player_prefix = r#"{"player":{"name":"Alex"},"fallback":""}"#.len();
    let exact_player_page = compound(vec![
        string_entry("object", "player"),
        string_entry("player", "Alex"),
        string_entry("fallback", &"x".repeat(32_767 - player_prefix)),
    ]);
    assert!(
        decode_written_book_content(&book_with_pages(vec![exact_player_page]), "written book",)
            .is_ok()
    );

    let item_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:max_stack_size":64,"!minecraft:damage":{}}}}"#
    )
    .len();
    let exact_item_patch_page =
        component_with_scalar_item_patch("x".repeat(32_767 - item_patch_prefix));
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_item_patch_page]),
        "written book",
    )
    .is_ok());

    let flat_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:use_effects":{},"minecraft:food":{"nutrition":4,"saturation":0.5,"can_always_eat":true}}}}"#
    )
    .len();
    let exact_flat_patch_page =
        component_with_flat_item_patch("x".repeat(32_767 - flat_patch_prefix));
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_flat_patch_page]),
        "written book",
    )
    .is_ok());

    let firework_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:firework_explosion":{"shape":"star","colors":[1193046],"has_trail":true}}}}"#
    )
    .len();
    let exact_firework_patch_page =
        component_with_firework_item_patch("x".repeat(32_767 - firework_patch_prefix));
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_firework_patch_page]),
        "written book",
    )
    .is_ok());

    let profile_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:profile":{"name":"Alex"}}}}"#
    )
    .len();
    let exact_profile_patch_page =
        component_with_profile_item_patch("x".repeat(32_767 - profile_patch_prefix));
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_profile_patch_page]),
        "written book",
    )
    .is_ok());

    let map_decorations_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:map_decorations":{"home":{"type":"minecraft:frame","x":0.0,"z":0.0,"rotation":0.0}}}}}"#
    )
    .len();
    let exact_map_decorations_patch_page = component_with_map_decorations_item_patch(
        "x".repeat(32_767 - map_decorations_patch_prefix),
    );
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_map_decorations_patch_page]),
        "written book",
    )
    .is_ok());

    let holder_collections_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:suspicious_stew_effects":[{"id":"minecraft:speed"}],"minecraft:pot_decorations":["minecraft:brick","minecraft:angler_pottery_sherd","minecraft:brick","minecraft:brick"]}}}"#
    )
    .len();
    let exact_holder_collections_patch_page = component_with_holder_collections_item_patch(
        "x".repeat(32_767 - holder_collections_patch_prefix),
    );
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_holder_collections_patch_page]),
        "written book",
    )
    .is_ok());

    let break_sound_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:break_sound":{"sound_id":"example:glass","range":12.5}}}}"#
    )
    .len();
    let exact_break_sound_patch_page =
        component_with_break_sound_item_patch("x".repeat(32_767 - break_sound_patch_prefix), 12.5);
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_break_sound_patch_page]),
        "written book",
    )
    .is_ok());

    let weapon_records_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:piercing_weapon":{"deals_knockback":false,"sound":{"sound_id":"example:throw"}},"minecraft:kinetic_weapon":{"contact_cooldown_ticks":5,"damage_conditions":{"max_duration_ticks":20,"min_relative_speed":2.0},"forward_movement":0.25,"hit_sound":{"sound_id":"example:hit","range":8.0}}}}}"#
    )
    .len();
    let exact_weapon_records_patch_page =
        component_with_weapon_records_item_patch("x".repeat(32_767 - weapon_records_patch_prefix));
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_weapon_records_patch_page]),
        "written book",
    )
    .is_ok());

    let banner_patterns_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:banner_patterns":[{"pattern":{"asset_id":"example:spiral","translation_key":"block.example.spiral"},"color":"blue"}]}}}"#
    )
    .len();
    let exact_banner_patterns_patch_page = component_with_banner_patterns_item_patch(
        "x".repeat(32_767 - banner_patterns_patch_prefix),
    );
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_banner_patterns_patch_page]),
        "written book",
    )
    .is_ok());

    let typed_nbt_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:custom_data":{"value":"typed"}}}}"#
    )
    .len();
    let exact_typed_nbt_patch_page =
        component_with_typed_nbt_item_patch("x".repeat(32_767 - typed_nbt_patch_prefix));
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_typed_nbt_patch_page]),
        "written book",
    )
    .is_ok());

    let debug_stick_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:debug_stick_state":{"minecraft:oak_log":"axis"}}}}"#
    )
    .len();
    let exact_debug_stick_patch_page =
        component_with_debug_stick_item_patch("x".repeat(32_767 - debug_stick_patch_prefix));
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_debug_stick_patch_page]),
        "written book",
    )
    .is_ok());

    let writable_book_patch_prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:writable_book_content":{"pages":[{"raw":"page"}]}}}}"#
    )
    .len();
    let exact_writable_book_patch_page =
        component_with_writable_book_item_patch("x".repeat(32_767 - writable_book_patch_prefix));
    assert!(decode_written_book_content(
        &book_with_pages(vec![exact_writable_book_patch_page]),
        "written book",
    )
    .is_ok());
}

#[test]
fn follows_filterable_full_first_then_component_fallback() {
    let fallback_component = compound(vec![
        value_entry("raw", NbtSummaryValue::Int(1)),
        string_entry("text", "Compact component"),
    ]);
    let content = decode_written_book_content(
        &book_with_pages(vec![fallback_component.clone()]),
        "written book",
    )
    .unwrap();
    assert_eq!(content.pages[0].raw.text, "Compact component");
    assert_eq!(content.pages[0].raw.component, fallback_component);
    assert_eq!(content.pages[0].filtered, None);

    for object in [
        compound(vec![string_entry("sprite", "minecraft:block/stone")]),
        compound(vec![string_entry("player", "Player")]),
    ] {
        assert!(
            decode_written_book_content(&book_with_pages(vec![object]), "written book",).is_ok()
        );
    }
}

#[test]
fn enforces_named_holder_item_patch_page_boundary_with_session_registries() {
    let mut registries = RegistrySet::default();
    for (registry, resource_id) in [
        ("minecraft:wolf_variant", "example:selected"),
        ("minecraft:damage_type", "minecraft:generic_kill"),
        ("minecraft:jukebox_song", "minecraft:cat"),
        ("minecraft:painting_variant", "minecraft:kebab"),
    ] {
        let mut contents = RegistryContentState::new(registry);
        contents.entries = vec![RegistryPacketEntry::stub(resource_id)];
        registries.contents.insert(registry.to_string(), contents);
    }
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:villager/variant":"minecraft:plains","minecraft:wolf/variant":"example:selected","minecraft:damage_type":"minecraft:generic_kill","minecraft:jukebox_playable":"minecraft:cat","minecraft:painting/variant":"minecraft:kebab"}}}"#
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_named_holder_item_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content_with_context(
            &book_with_pages(vec![page]),
            "written book",
            DialogDecodeContext::new(Some(&registries), 0),
        );
        match expected_error {
            None => assert!(result.is_ok()),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_enchantment_item_patch_page_boundary_with_session_registries() {
    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new("minecraft:enchantment");
    contents.entries = vec![
        RegistryPacketEntry::stub("example:first"),
        RegistryPacketEntry::stub("minecraft:sharpness"),
    ];
    registries
        .contents
        .insert("minecraft:enchantment".to_string(), contents);
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:enchantments":{"example:first":1},"minecraft:stored_enchantments":{"minecraft:sharpness":255}}}}"#
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_enchantment_item_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content_with_context(
            &book_with_pages(vec![page]),
            "written book",
            DialogDecodeContext::new(Some(&registries), 0),
        );
        match expected_error {
            None => assert!(result.is_ok()),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_holder_set_item_patch_page_boundary_with_session_registries() {
    let mut registries = RegistrySet::default();
    let stone_id = registries
        .holder_id_by_resource_id("minecraft:item", "minecraft:stone")
        .unwrap();
    let dirt_id = registries
        .holder_id_by_resource_id("minecraft:item", "minecraft:dirt")
        .unwrap();
    registries.tags.insert(
        "minecraft:item".to_string(),
        RegistryTagState {
            registry: "minecraft:item".to_string(),
            tags: BTreeMap::from([("example:repair_items".to_string(), vec![stone_id, dirt_id])]),
        },
    );
    for (registry, resource_ids) in [
        ("minecraft:banner_pattern", &["minecraft:base"][..]),
        (
            "minecraft:damage_type",
            &["minecraft:generic", "minecraft:fall"][..],
        ),
    ] {
        let mut contents = RegistryContentState::new(registry);
        contents.entries = resource_ids
            .iter()
            .map(|resource_id| RegistryPacketEntry::stub(*resource_id))
            .collect();
        registries.contents.insert(registry.to_string(), contents);
    }
    let prefix = concat!(
        r##"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:repairable":{"items":"#example:repair_items"},"minecraft:provides_banner_patterns":"minecraft:base","minecraft:damage_resistant":{"types":["minecraft:generic","minecraft:fall"]}}}}"##
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_holder_set_item_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content_with_context(
            &book_with_pages(vec![page]),
            "written book",
            DialogDecodeContext::new(Some(&registries), 0),
        );
        match expected_error {
            None => assert!(result.is_ok()),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn rejects_malformed_written_book_fields_filterables_and_exact_canonical_page_limit() {
    let cases = [
        (NbtSummaryValue::List(Vec::new()), "is not a compound"),
        (
            compound(vec![string_entry("author", "Author")]),
            "is missing field title",
        ),
        (
            compound(vec![string_entry("title", "Title")]),
            "is missing field author",
        ),
        (
            written_book(vec![
                value_entry("title", NbtSummaryValue::Int(1)),
                string_entry("author", "Author"),
            ]),
            "field title is neither a string nor a compound",
        ),
        (
            written_book(vec![
                string_entry("title", "😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀"),
                string_entry("author", "Author"),
            ]),
            "field title has 34 UTF-16 units, maximum is 32",
        ),
        (
            written_book(vec![
                string_entry("title", "Title"),
                value_entry("author", NbtSummaryValue::Int(1)),
            ]),
            "field author is not a string",
        ),
        (
            written_book(vec![
                string_entry("title", "Title"),
                string_entry("author", "Author"),
                value_entry("generation", NbtSummaryValue::Int(-1)),
            ]),
            "field generation -1 is outside 0..=3",
        ),
        (
            written_book(vec![
                string_entry("title", "Title"),
                string_entry("author", "Author"),
                value_entry("generation", NbtSummaryValue::Int(4)),
            ]),
            "field generation 4 is outside 0..=3",
        ),
        (
            written_book(vec![
                string_entry("title", "Title"),
                string_entry("author", "Author"),
                string_entry("pages", "not a list"),
            ]),
            "field pages is not a list",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::Int(1)]),
            "pages[0] is not a component",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::List(Vec::new())]),
            "pages[0] is not a component",
        ),
        (
            book_with_pages(vec![compound(vec![value_entry(
                "raw",
                NbtSummaryValue::Int(1),
            )])]),
            "pages[0] field raw is not a component",
        ),
        (
            book_with_pages(vec![compound(vec![
                string_entry("raw", "Raw"),
                value_entry("filtered", NbtSummaryValue::Int(1)),
            ])]),
            "pages[0] field filtered is not a component",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::String("x".repeat(32_766))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::String("\"".repeat(16_383))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::String("\u{2028}".repeat(5_461))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![NbtSummaryValue::String("😀".repeat(16_383))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![compound(vec![
                string_entry("text", &"x".repeat(32_745)),
                value_entry("bold", NbtSummaryValue::Byte(1)),
            ])]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![compound(vec![
                string_entry("object", "player"),
                string_entry("player", "Alex"),
                string_entry(
                    "fallback",
                    &"x".repeat(32_768 - r#"{"player":{"name":"Alex"},"fallback":""}"#.len()),
                ),
            ])]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_scalar_item_patch("x".repeat(
                32_768
                    - concat!(
                        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:max_stack_size":64,"!minecraft:damage":{}}}}"#
                    )
                    .len(),
            ))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_flat_item_patch("x".repeat(
                32_768
                    - concat!(
                        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:use_effects":{},"minecraft:food":{"nutrition":4,"saturation":0.5,"can_always_eat":true}}}}"#
                    )
                    .len(),
            ))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_firework_item_patch("x".repeat(
                32_768
                    - concat!(
                        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:firework_explosion":{"shape":"star","colors":[1193046],"has_trail":true}}}}"#
                    )
                    .len(),
            ))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_profile_item_patch("x".repeat(
                32_768
                    - concat!(
                        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:profile":{"name":"Alex"}}}}"#
                    )
                    .len(),
            ))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_map_decorations_item_patch(
                "x".repeat(
                    32_768
                        - concat!(
                            r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:map_decorations":{"home":{"type":"minecraft:frame","x":0.0,"z":0.0,"rotation":0.0}}}}}"#
                        )
                        .len(),
                ),
            )]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_holder_collections_item_patch(
                "x".repeat(
                    32_768
                        - concat!(
                            r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:suspicious_stew_effects":[{"id":"minecraft:speed"}],"minecraft:pot_decorations":["minecraft:brick","minecraft:angler_pottery_sherd","minecraft:brick","minecraft:brick"]}}}"#
                        )
                        .len(),
                ),
            )]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_break_sound_item_patch(
                "x".repeat(
                    32_768
                        - concat!(
                            r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:break_sound":{"sound_id":"example:glass","range":12.5}}}}"#
                        )
                        .len(),
                ),
                12.5,
            )]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_weapon_records_item_patch("x".repeat(
                32_768
                    - concat!(
                        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:piercing_weapon":{"deals_knockback":false,"sound":{"sound_id":"example:throw"}},"minecraft:kinetic_weapon":{"contact_cooldown_ticks":5,"damage_conditions":{"max_duration_ticks":20,"min_relative_speed":2.0},"forward_movement":0.25,"hit_sound":{"sound_id":"example:hit","range":8.0}}}}}"#
                    )
                    .len(),
            ))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_banner_patterns_item_patch("x".repeat(
                32_768
                    - concat!(
                        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:banner_patterns":[{"pattern":{"asset_id":"example:spiral","translation_key":"block.example.spiral"},"color":"blue"}]}}}"#
                    )
                    .len(),
            ))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_typed_nbt_item_patch("x".repeat(
                32_768
                    - concat!(
                        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:custom_data":{"value":"typed"}}}}"#
                    )
                    .len(),
            ))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_non_finite_typed_nbt_item_patch()]),
            "canonical JsonWriter rejects non-finite Float/Double values",
        ),
        (
            book_with_pages(vec![component_with_debug_stick_item_patch("x".repeat(
                32_768
                    - concat!(
                        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:debug_stick_state":{"minecraft:oak_log":"axis"}}}}"#
                    )
                    .len(),
            ))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_writable_book_item_patch("x".repeat(
                32_768
                    - concat!(
                        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:writable_book_content":{"pages":[{"raw":"page"}]}}}}"#
                    )
                    .len(),
            ))]),
            "encodes to 32768 JSON UTF-16 units, maximum is 32767",
        ),
        (
            book_with_pages(vec![component_with_non_finite_item_patch()]),
            "canonical item component patch contains a non-finite Float/Double value",
        ),
        (
            book_with_pages(vec![component_with_break_sound_item_patch(
                String::new(),
                f32::NAN,
            )]),
            "canonical item component patch contains a non-finite Float/Double value",
        ),
        (
            book_with_pages(vec![component_with_non_finite_weapon_item_patch()]),
            "canonical item component patch contains a non-finite Float/Double value",
        ),
        (
            written_book(vec![
                string_entry("title", "Title"),
                string_entry("author", "Author"),
                string_entry("resolved", "true"),
            ]),
            "field resolved is not a boolean",
        ),
    ];
    for (value, expected) in cases {
        let error = decode_written_book_content(&value, "written book").unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} does not contain {expected:?}"
        );
    }
}

fn book_with_pages(pages: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    written_book(vec![
        string_entry("title", "Title"),
        string_entry("author", "Author"),
        value_entry("pages", NbtSummaryValue::List(pages)),
    ])
}

fn component_with_scalar_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![
                        value_entry("max_stack_size", NbtSummaryValue::Byte(64)),
                        value_entry("!damage", compound(Vec::new())),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_flat_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![
                        value_entry("use_effects", compound(Vec::new())),
                        value_entry(
                            "food",
                            compound(vec![
                                value_entry("nutrition", NbtSummaryValue::Int(4)),
                                value_entry("saturation", NbtSummaryValue::Float(0.5f32.to_bits())),
                                value_entry("can_always_eat", NbtSummaryValue::Byte(1)),
                            ]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_non_finite_item_patch() -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![value_entry(
                        "custom_model_data",
                        compound(vec![value_entry(
                            "floats",
                            NbtSummaryValue::List(vec![NbtSummaryValue::Float(f32::NAN.to_bits())]),
                        )]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_firework_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![value_entry(
                        "firework_explosion",
                        compound(vec![
                            string_entry("shape", "star"),
                            value_entry(
                                "colors",
                                NbtSummaryValue::List(vec![NbtSummaryValue::Int(0x12_34_56)]),
                            ),
                            value_entry("has_trail", NbtSummaryValue::Byte(1)),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_profile_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![string_entry("profile", "Alex")]),
                ),
            ]),
        ),
    ])
}

fn component_with_map_decorations_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![value_entry(
                        "map_decorations",
                        compound(vec![value_entry(
                            "home",
                            compound(vec![
                                string_entry("type", "frame"),
                                value_entry("x", NbtSummaryValue::Int(0)),
                                value_entry("z", NbtSummaryValue::Int(0)),
                                value_entry("rotation", NbtSummaryValue::Int(0)),
                            ]),
                        )]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_holder_collections_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![
                        value_entry(
                            "suspicious_stew_effects",
                            NbtSummaryValue::List(vec![compound(vec![string_entry(
                                "id", "speed",
                            )])]),
                        ),
                        value_entry(
                            "pot_decorations",
                            NbtSummaryValue::List(vec![
                                NbtSummaryValue::String("brick".to_string()),
                                NbtSummaryValue::String("angler_pottery_sherd".to_string()),
                            ]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_named_holder_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![
                        string_entry("villager/variant", "plains"),
                        string_entry("wolf/variant", "example:selected"),
                        string_entry("damage_type", "generic_kill"),
                        string_entry("jukebox_playable", "cat"),
                        string_entry("painting/variant", "kebab"),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_enchantment_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![
                        value_entry(
                            "enchantments",
                            compound(vec![value_entry("example:first", NbtSummaryValue::Int(1))]),
                        ),
                        value_entry(
                            "stored_enchantments",
                            compound(vec![value_entry("sharpness", NbtSummaryValue::Int(255))]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_break_sound_item_patch(extra: String, range: f32) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![value_entry(
                        "break_sound",
                        compound(vec![
                            string_entry("sound_id", "example:glass"),
                            value_entry("range", NbtSummaryValue::Float(range.to_bits())),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_weapon_records_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![
                        value_entry(
                            "piercing_weapon",
                            compound(vec![
                                value_entry("deals_knockback", NbtSummaryValue::Byte(0)),
                                value_entry(
                                    "sound",
                                    compound(vec![string_entry("sound_id", "example:throw")]),
                                ),
                            ]),
                        ),
                        value_entry(
                            "kinetic_weapon",
                            compound(vec![
                                value_entry("contact_cooldown_ticks", NbtSummaryValue::Int(5)),
                                value_entry(
                                    "damage_conditions",
                                    compound(vec![
                                        value_entry("max_duration_ticks", NbtSummaryValue::Int(20)),
                                        value_entry(
                                            "min_relative_speed",
                                            NbtSummaryValue::Float(2.0f32.to_bits()),
                                        ),
                                    ]),
                                ),
                                value_entry(
                                    "forward_movement",
                                    NbtSummaryValue::Float(0.25f32.to_bits()),
                                ),
                                value_entry(
                                    "hit_sound",
                                    compound(vec![
                                        string_entry("sound_id", "example:hit"),
                                        value_entry(
                                            "range",
                                            NbtSummaryValue::Float(8.0f32.to_bits()),
                                        ),
                                    ]),
                                ),
                            ]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_non_finite_weapon_item_patch() -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![value_entry(
                        "kinetic_weapon",
                        compound(vec![value_entry(
                            "forward_movement",
                            NbtSummaryValue::Float(f32::NAN.to_bits()),
                        )]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_banner_patterns_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![value_entry(
                        "banner_patterns",
                        NbtSummaryValue::List(vec![compound(vec![
                            value_entry(
                                "pattern",
                                compound(vec![
                                    string_entry("asset_id", "example:spiral"),
                                    string_entry("translation_key", "block.example.spiral"),
                                ]),
                            ),
                            string_entry("color", "blue"),
                        ])]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_typed_nbt_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![value_entry(
                        "custom_data",
                        compound(vec![string_entry("value", "typed")]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_non_finite_typed_nbt_item_patch() -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![value_entry(
                        "custom_data",
                        compound(vec![value_entry(
                            "value",
                            NbtSummaryValue::Double(f64::NAN.to_bits()),
                        )]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_debug_stick_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![value_entry(
                        "debug_stick_state",
                        compound(vec![string_entry("oak_log", "axis")]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_writable_book_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![value_entry(
                        "writable_book_content",
                        compound(vec![value_entry(
                            "pages",
                            NbtSummaryValue::List(vec![NbtSummaryValue::String(
                                "page".to_string(),
                            )]),
                        )]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_holder_set_item_patch(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "item"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "hover_event",
            compound(vec![
                string_entry("action", "show_item"),
                string_entry("id", "stone"),
                value_entry(
                    "components",
                    compound(vec![
                        value_entry(
                            "repairable",
                            compound(vec![string_entry("items", "#example:repair_items")]),
                        ),
                        string_entry("provides_banner_patterns", "base"),
                        value_entry(
                            "damage_resistant",
                            compound(vec![value_entry(
                                "types",
                                NbtSummaryValue::List(vec![
                                    NbtSummaryValue::String("generic".to_string()),
                                    NbtSummaryValue::String("fall".to_string()),
                                ]),
                            )]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn written_book(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    compound(entries)
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::String(value.to_string()))
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}
