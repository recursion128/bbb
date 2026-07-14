use bbb_protocol::{
    canonical_component_json_utf16_len,
    data_component_types::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    packets::inventory::{DataComponentPatchSummary, WrittenBookContentSummary},
    NbtSummaryEntry, NbtSummaryValue,
};

use std::collections::BTreeMap;

use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet, RegistryTagState};

use super::super::{
    apply_item_patch_value, decode_supported_component_value, DialogDataComponentValueState,
};
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
        Some(DialogDataComponentValueState::WrittenBookContent {
            content: content.clone(),
        })
    );

    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
        &DialogDataComponentValueState::WrittenBookContent { content },
        &mut patch,
    ));
    assert_eq!(
        patch.written_book,
        Some(WrittenBookContentSummary {
            title: "A title".to_string(),
            title_filter: None,
            author: "An author".to_string(),
            generation: 2,
            pages: vec!["Compact page".to_string(), "Raw".to_string()],
            page_filters: vec![None, Some("Filtered".to_string())],
            resolved: true,
        })
    );
    assert_eq!(
        patch.written_book_pages_nbt,
        vec![
            Some(NbtSummaryValue::String("Compact page".to_string())),
            Some(compound(vec![string_entry("text", "Raw")])),
        ]
    );
    assert_eq!(
        patch.written_book_page_filters_nbt,
        vec![None, Some(styled)]
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
fn enforces_tool_item_patch_page_boundary_with_session_registries() {
    let mut registries = RegistrySet::default();
    let stone_id = registries
        .holder_id_by_resource_id("minecraft:block", "minecraft:stone")
        .unwrap();
    let dirt_id = registries
        .holder_id_by_resource_id("minecraft:block", "minecraft:dirt")
        .unwrap();
    registries.tags.insert(
        "minecraft:block".to_string(),
        RegistryTagState {
            registry: "minecraft:block".to_string(),
            tags: BTreeMap::from([("example:mineable".to_string(), vec![stone_id, dirt_id])]),
        },
    );
    let prefix = concat!(
        r##"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:tool":{"rules":[{"blocks":"#example:mineable","speed":4.0,"correct_for_drops":true}],"damage_per_block":0,"can_destroy_blocks_in_creative":false}}}}"##
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_tool_item_patch("x".repeat(limit - prefix));
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
fn enforces_equippable_item_patch_page_boundary_with_session_registries() {
    let mut registries = RegistrySet::default();
    let llama_id = registries
        .holder_id_by_resource_id("minecraft:entity_type", "minecraft:llama")
        .unwrap();
    let trader_llama_id = registries
        .holder_id_by_resource_id("minecraft:entity_type", "minecraft:trader_llama")
        .unwrap();
    registries.tags.insert(
        "minecraft:entity_type".to_string(),
        RegistryTagState {
            registry: "minecraft:entity_type".to_string(),
            tags: BTreeMap::from([(
                "example:llamas".to_string(),
                vec![llama_id, trader_llama_id],
            )]),
        },
    );
    let prefix = concat!(
        r##"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:equippable":{"slot":"body","equip_sound":{"sound_id":"example:equip","range":6.5},"asset_id":"example:llama_carpet","camera_overlay":"minecraft:overlay/powder_snow","allowed_entities":"#example:llamas","dispensable":false,"swappable":false,"damage_on_hurt":false,"equip_on_interact":true,"can_be_sheared":true}}}}"##
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_equippable_item_patch("x".repeat(limit - prefix));
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
fn enforces_blocks_attacks_item_patch_page_boundary_with_session_registries() {
    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new("minecraft:damage_type");
    contents.entries = vec![
        RegistryPacketEntry::stub("minecraft:in_fire"),
        RegistryPacketEntry::stub("minecraft:explosion"),
    ];
    registries
        .contents
        .insert("minecraft:damage_type".to_string(), contents);
    registries.tags.insert(
        "minecraft:damage_type".to_string(),
        RegistryTagState {
            registry: "minecraft:damage_type".to_string(),
            tags: BTreeMap::from([("example:fire".to_string(), vec![0])]),
        },
    );
    let prefix = concat!(
        r##"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:blocks_attacks":{"block_delay_seconds":0.25,"disable_cooldown_scale":2.0,"damage_reductions":[{"horizontal_blocking_angle":45.0,"type":"#example:fire","base":-2.0,"factor":0.5}],"item_damage":{"threshold":3.0,"base":1.0,"factor":-1.0},"bypassed_by":"minecraft:explosion","block_sound":"minecraft:item.shield.block","disabled_sound":{"sound_id":"example:disable","range":8.0}}}}}"##
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_blocks_attacks_item_patch("x".repeat(limit - prefix));
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
fn enforces_consume_effect_item_patch_page_boundary_with_session_registries() {
    let mut registries = RegistrySet::default();
    let speed_id = registries
        .holder_id_by_resource_id("minecraft:mob_effect", "minecraft:speed")
        .unwrap();
    let weakness_id = registries
        .holder_id_by_resource_id("minecraft:mob_effect", "minecraft:weakness")
        .unwrap();
    registries.tags.insert(
        "minecraft:mob_effect".to_string(),
        RegistryTagState {
            registry: "minecraft:mob_effect".to_string(),
            tags: BTreeMap::from([("example:negative".to_string(), vec![speed_id, weakness_id])]),
        },
    );
    let prefix = concat!(
        r##"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:consumable":{"consume_seconds":0.5,"animation":"drink","sound":{"sound_id":"example:consume","range":4.0},"has_consume_particles":false,"on_consume_effects":[{"type":"minecraft:apply_effects","effects":[{"id":"minecraft:speed","amplifier":2,"duration":40,"show_icon":true}],"probability":0.25},{"type":"minecraft:remove_effects","effects":"#example:negative"},{"type":"minecraft:clear_all_effects"},{"type":"minecraft:teleport_randomly","diameter":8.0},{"type":"minecraft:play_sound","sound":"minecraft:entity.generic.eat"}]},"minecraft:death_protection":{"death_effects":[{"type":"minecraft:clear_all_effects"}]}}}}"##
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_consume_effect_item_patch("x".repeat(limit - prefix));
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
fn enforces_potion_contents_item_patch_page_boundary() {
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:potion_contents":{"potion":"minecraft:strong_swiftness","custom_color":1193046,"custom_effects":[{"id":"minecraft:speed","amplifier":2,"duration":40,"show_particles":false,"show_icon":true}],"custom_name":"example.potion"}}}}"#
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_potion_contents_item_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok()),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_text_component_item_patch_page_boundary() {
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:custom_name":{"text":"Sword","color":"dark_purple","bold":true},"minecraft:item_name":"Named","minecraft:lore":["First",{"translate":"item.second","italic":false}]}}}"#
    )
    .len();
    let canonical_prefix = crate::client::dialog::canonical_component_json_utf16_len_with_context(
        &component_with_text_item_patch(String::new()),
        DialogDecodeContext::new(None, 0),
    )
    .unwrap()
    .unwrap();
    assert_eq!(canonical_prefix, prefix);
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_text_item_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok()),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_nested_dialog_item_name_page_boundary() {
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:custom_name":{"text":"Nested","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Inner"}}}}}}"#
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_nested_dialog_item_name("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok(), "{result:?}"),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_nested_dialog_written_book_page_boundary() {
    let prefix = r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:written_book_content":{"title":{"raw":"Title"},"author":"Author","pages":[{"raw":{"text":"Nested","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Inner"}}}}]}}}}"#
        .len();
    let canonical_prefix = crate::client::dialog::canonical_component_json_utf16_len_with_context(
        &component_with_nested_dialog_written_book_page(String::new()),
        DialogDecodeContext::new(None, 0),
    )
    .unwrap()
    .unwrap();
    assert_eq!(canonical_prefix, prefix);
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_nested_dialog_written_book_page("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok(), "{result:?}"),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_written_book_content_item_patch_page_boundary() {
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:written_book_content":{"title":{"raw":"Title","filtered":"Safe"},"author":"Author","generation":2,"pages":[{"raw":"Raw","filtered":"Filtered"}],"resolved":true}}}}"#
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_written_book_item_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok(), "{result:?}"),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_default_notice_show_dialog_page_boundary() {
    let prefix = concat!(
        r#"{"text":"Open","extra":[""],"click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Hello"}}}"#
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_default_notice_show_dialog("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok(), "{result:?}"),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_non_default_notice_show_dialog_page_boundary() {
    let prefix = concat!(
        r#"{"text":"Open","extra":[""],"click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Title","external_title":"External","can_close_with_escape":false,"pause":false,"after_action":"none","action":{"label":"Go","tooltip":"Tip","width":200,"action":{"type":"minecraft:run_command","command":"say hi"}}}}}"#
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_non_default_notice_show_dialog("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok(), "{result:?}"),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_notice_body_show_dialog_page_boundary() {
    let prefix = concat!(
        r#"{"text":"Open","extra":[""],"click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Body","body":{"type":"minecraft:plain_message","contents":"Message"}}}}"#
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_notice_body_show_dialog("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok(), "{result:?}"),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_notice_input_show_dialog_page_boundary() {
    let prefix = concat!(
        r#"{"text":"Open","extra":[""],"click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Inputs","inputs":[{"key":"enabled","type":"minecraft:boolean","label":"Enabled"}]}}}"#
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_notice_input_show_dialog("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok(), "{result:?}"),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_confirmation_show_dialog_page_boundary() {
    let prefix = concat!(
        r#"{"text":"Open","extra":[""],"click_event":{"action":"show_dialog","dialog":{"type":"minecraft:confirmation","title":"Confirm","yes":{"label":"Yes"},"no":{"label":"No"}}}}"#
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_confirmation_show_dialog("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok(), "{result:?}"),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_direct_instrument_dialog_description_page_boundary() {
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:instrument":{"sound_event":{"sound_id":"example:horn","range":16.0},"use_duration":7.0,"range":256.0,"description":{"text":"Ponder","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Horn"}}}}}}}"#
    )
    .len();
    let canonical_prefix = crate::client::dialog::canonical_component_json_utf16_len_with_context(
        &component_with_instrument_item_patch(String::new()),
        DialogDecodeContext::new(None, 0),
    )
    .unwrap()
    .unwrap();
    assert_eq!(canonical_prefix, prefix);
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_instrument_item_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok()),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_direct_trim_dialog_descriptions_page_boundary() {
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:provides_trim_material":{"asset_name":"quartz","override_armor_assets":{"example:copper":"nested/copper","minecraft:iron":"iron_darker"},"description":{"text":"Quartz","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Quartz dialog"}}}},"minecraft:trim":{"material":{"asset_name":"resin","description":{"text":"Resin","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Material dialog"}}}},"pattern":{"asset_id":"minecraft:sentry","description":{"text":"Sentry","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Pattern dialog"}}},"decal":false}}}}}"#
    )
    .len();
    let canonical_prefix = crate::client::dialog::canonical_component_json_utf16_len_with_context(
        &component_with_trim_item_patch(String::new()),
        DialogDecodeContext::new(None, 0),
    )
    .unwrap()
    .unwrap();
    assert_eq!(canonical_prefix, prefix);
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_trim_item_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok()),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_attribute_modifier_override_dialog_page_boundary() {
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:attribute_modifiers":[{"type":"minecraft:attack_damage","id":"minecraft:base_attack_damage","amount":-2.5,"operation":"add_value"},{"type":"minecraft:armor","id":"example:hidden","amount":0.25,"operation":"add_multiplied_base","slot":"mainhand","display":{"type":"hidden"}},{"type":"minecraft:movement_speed","id":"example:override","amount":1.5,"operation":"add_multiplied_total","slot":"head","display":{"type":"override","value":{"text":"Boost","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Inner"}}}}}]}}}"#
    )
    .len();
    let canonical_prefix = crate::client::dialog::canonical_component_json_utf16_len_with_context(
        &component_with_attribute_modifier_item_patch(String::new()),
        DialogDecodeContext::new(None, 0),
    )
    .unwrap()
    .unwrap();
    assert_eq!(canonical_prefix, prefix);
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_attribute_modifier_item_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok()),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_predicate_item_patch_page_boundary_with_session_registries() {
    let registries = RegistrySet::default();
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:can_break":{"blocks":"minecraft:stone","state":{"axis":"y"},"components":{"minecraft:max_stack_size":64},"predicates":{"minecraft:damage":{"damage":3}}},"minecraft:lock":{"items":["minecraft:stone","minecraft:dirt"],"count":{"min":1,"max":2},"predicates":{"minecraft:container":{"items":{"contains":[{"items":"minecraft:stone","count":1}]}}}}}}}"#
    )
    .len();
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_predicate_item_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content_with_context(
            &book_with_pages(vec![page]),
            "written book",
            DialogDecodeContext::new(Some(&registries), 0),
        );
        match expected_error {
            None => assert!(result.is_ok(), "{result:?}"),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_dialog_components_in_predicate_tree_page_boundary() {
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:can_break":{"components":{"minecraft:custom_name":{"text":"Break","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Break dialog"}}}}},"minecraft:lock":{"components":{"minecraft:item_name":{"text":"Lock","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Lock dialog"}}}},"predicates":{"minecraft:written_book_content":{"pages":{"contains":[{"text":"Page","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Page dialog"}}}],"count":[{"test":{"text":"Count","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Count dialog"}}},"count":1}]}},"minecraft:container":{"items":{"contains":[{"components":{"minecraft:custom_name":{"text":"Nested","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Nested dialog"}}}}}]}}}}}}}"#
    )
    .len();
    let empty = component_with_dialog_predicate_item_patch(String::new());
    assert_eq!(canonical_component_json_utf16_len(&empty).unwrap(), None);
    let canonical_prefix = crate::client::dialog::canonical_component_json_utf16_len_with_context(
        &empty,
        DialogDecodeContext::new(None, 0),
    )
    .unwrap()
    .unwrap();
    assert_eq!(canonical_prefix, prefix);
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_dialog_predicate_item_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
        match expected_error {
            None => assert!(result.is_ok(), "{result:?}"),
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
        }
    }
}

#[test]
fn enforces_dialog_components_in_recursive_item_templates_page_boundary() {
    let prefix = concat!(
        r#"{"text":"item","extra":[""],"hover_event":{"action":"show_item","id":"minecraft:stone","components":{"minecraft:use_remainder":{"id":"minecraft:glass_bottle","components":{"minecraft:custom_name":{"text":"Remainder","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Remainder dialog"}}}}},"minecraft:charged_projectiles":[{"id":"minecraft:arrow","components":{"minecraft:custom_name":{"text":"Projectile","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Projectile dialog"}}}}},{"id":"minecraft:firework_rocket","count":3,"components":{"minecraft:repair_cost":4}}],"minecraft:bundle_contents":[{"id":"minecraft:diamond","components":{"minecraft:item_name":{"text":"Bundle","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Bundle dialog"}}}}}],"minecraft:container":[{"slot":0,"item":{"id":"minecraft:diamond","components":{"minecraft:custom_name":{"text":"Container","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Container dialog"}}}}}},{"slot":2,"item":{"id":"minecraft:dirt","count":5}}]}}}"#
    )
    .len();
    let empty = component_with_recursive_item_template_patch(String::new());
    assert_eq!(canonical_component_json_utf16_len(&empty).unwrap(), None);
    let canonical_prefix = crate::client::dialog::canonical_component_json_utf16_len_with_context(
        &empty,
        DialogDecodeContext::new(None, 0),
    )
    .unwrap()
    .unwrap();
    assert_eq!(canonical_prefix, prefix);
    for (limit, expected_error) in [(32_767usize, None), (32_768usize, Some("encodes to 32768"))] {
        let page = component_with_recursive_item_template_patch("x".repeat(limit - prefix));
        let result = decode_written_book_content(&book_with_pages(vec![page]), "written book");
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
            book_with_pages(vec![component_with_non_finite_tool_item_patch()]),
            "canonical item component patch contains a non-finite Float/Double value",
        ),
        (
            book_with_pages(vec![component_with_non_finite_equippable_item_patch()]),
            "canonical item component patch contains a non-finite Float/Double value",
        ),
        (
            book_with_pages(vec![component_with_non_finite_blocks_attacks_item_patch()]),
            "canonical item component patch contains a non-finite Float/Double value",
        ),
        (
            book_with_pages(vec![component_with_non_finite_consume_effect_item_patch()]),
            "canonical item component patch contains a non-finite Float/Double value",
        ),
        (
            book_with_pages(vec![component_with_non_finite_text_item_patch()]),
            "canonical JsonWriter rejects non-finite Float/Double values",
        ),
        (
            book_with_pages(vec![component_with_non_finite_attribute_modifier_item_patch()]),
            "canonical item component patch contains a non-finite Float/Double value",
        ),
        (
            book_with_pages(vec![component_with_non_finite_predicate_item_patch()]),
            "canonical item component patch contains a non-finite Float/Double value",
        ),
        (
            book_with_pages(vec![component_with_non_finite_recursive_item_template_patch()]),
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

fn component_with_predicate_item_patch(extra: String) -> NbtSummaryValue {
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
                            "can_break",
                            compound(vec![
                                string_entry("blocks", "stone"),
                                value_entry("state", compound(vec![string_entry("axis", "y")])),
                                value_entry(
                                    "components",
                                    compound(vec![value_entry(
                                        "max_stack_size",
                                        NbtSummaryValue::Byte(64),
                                    )]),
                                ),
                                value_entry(
                                    "predicates",
                                    compound(vec![value_entry(
                                        "damage",
                                        compound(vec![value_entry(
                                            "damage",
                                            NbtSummaryValue::Int(3),
                                        )]),
                                    )]),
                                ),
                            ]),
                        ),
                        value_entry(
                            "lock",
                            compound(vec![
                                value_entry(
                                    "items",
                                    NbtSummaryValue::List(vec![
                                        NbtSummaryValue::String("stone".to_string()),
                                        NbtSummaryValue::String("dirt".to_string()),
                                    ]),
                                ),
                                value_entry(
                                    "count",
                                    compound(vec![
                                        value_entry("min", NbtSummaryValue::Int(1)),
                                        value_entry("max", NbtSummaryValue::Int(2)),
                                    ]),
                                ),
                                value_entry(
                                    "predicates",
                                    compound(vec![value_entry(
                                        "container",
                                        compound(vec![value_entry(
                                            "items",
                                            compound(vec![value_entry(
                                                "contains",
                                                NbtSummaryValue::List(vec![compound(vec![
                                                    string_entry("items", "stone"),
                                                    value_entry("count", NbtSummaryValue::Int(1)),
                                                ])]),
                                            )]),
                                        )]),
                                    )]),
                                ),
                            ]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_dialog_predicate_item_patch(extra: String) -> NbtSummaryValue {
    let dialog_text = |text: &str, title: &str| {
        compound(vec![
            string_entry("text", text),
            value_entry(
                "click_event",
                compound(vec![
                    string_entry("action", "show_dialog"),
                    value_entry(
                        "dialog",
                        compound(vec![
                            string_entry("type", "notice"),
                            string_entry("title", title),
                        ]),
                    ),
                ]),
            ),
        ])
    };
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
                            "can_break",
                            compound(vec![value_entry(
                                "components",
                                compound(vec![value_entry(
                                    "custom_name",
                                    dialog_text("Break", "Break dialog"),
                                )]),
                            )]),
                        ),
                        value_entry(
                            "lock",
                            compound(vec![
                                value_entry(
                                    "components",
                                    compound(vec![value_entry(
                                        "item_name",
                                        dialog_text("Lock", "Lock dialog"),
                                    )]),
                                ),
                                value_entry(
                                    "predicates",
                                    compound(vec![
                                        value_entry(
                                            "written_book_content",
                                            compound(vec![value_entry(
                                                "pages",
                                                compound(vec![
                                                    value_entry(
                                                        "contains",
                                                        NbtSummaryValue::List(vec![dialog_text(
                                                            "Page",
                                                            "Page dialog",
                                                        )]),
                                                    ),
                                                    value_entry(
                                                        "count",
                                                        NbtSummaryValue::List(vec![compound(
                                                            vec![
                                                                value_entry(
                                                                    "test",
                                                                    dialog_text(
                                                                        "Count",
                                                                        "Count dialog",
                                                                    ),
                                                                ),
                                                                value_entry(
                                                                    "count",
                                                                    NbtSummaryValue::Int(1),
                                                                ),
                                                            ],
                                                        )]),
                                                    ),
                                                ]),
                                            )]),
                                        ),
                                        value_entry(
                                            "container",
                                            compound(vec![value_entry(
                                                "items",
                                                compound(vec![value_entry(
                                                    "contains",
                                                    NbtSummaryValue::List(vec![compound(vec![
                                                        value_entry(
                                                            "components",
                                                            compound(vec![value_entry(
                                                                "custom_name",
                                                                dialog_text(
                                                                    "Nested",
                                                                    "Nested dialog",
                                                                ),
                                                            )]),
                                                        ),
                                                    ])]),
                                                )]),
                                            )]),
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

fn component_with_non_finite_predicate_item_patch() -> NbtSummaryValue {
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
                        "lock",
                        compound(vec![value_entry(
                            "predicates",
                            compound(vec![value_entry(
                                "attribute_modifiers",
                                compound(vec![value_entry(
                                    "modifiers",
                                    compound(vec![value_entry(
                                        "contains",
                                        NbtSummaryValue::List(vec![compound(vec![value_entry(
                                            "amount",
                                            NbtSummaryValue::Double(f64::NAN.to_bits()),
                                        )])]),
                                    )]),
                                )]),
                            )]),
                        )]),
                    )]),
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

fn component_with_written_book_item_patch(extra: String) -> NbtSummaryValue {
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
                        "written_book_content",
                        compound(vec![
                            value_entry(
                                "title",
                                compound(vec![
                                    string_entry("raw", "Title"),
                                    string_entry("filtered", "Safe"),
                                ]),
                            ),
                            string_entry("author", "Author"),
                            value_entry("generation", NbtSummaryValue::Int(2)),
                            value_entry(
                                "pages",
                                NbtSummaryValue::List(vec![compound(vec![
                                    string_entry("raw", "Raw"),
                                    string_entry("filtered", "Filtered"),
                                ])]),
                            ),
                            value_entry("resolved", NbtSummaryValue::Byte(1)),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_default_notice_show_dialog(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "Open"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "click_event",
            compound(vec![
                string_entry("action", "show_dialog"),
                value_entry(
                    "dialog",
                    compound(vec![
                        string_entry("type", "notice"),
                        string_entry("title", "Hello"),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_non_default_notice_show_dialog(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "Open"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "click_event",
            compound(vec![
                string_entry("action", "show_dialog"),
                value_entry(
                    "dialog",
                    compound(vec![
                        string_entry("type", "notice"),
                        string_entry("title", "Title"),
                        string_entry("external_title", "External"),
                        value_entry("can_close_with_escape", NbtSummaryValue::Byte(0)),
                        value_entry("pause", NbtSummaryValue::Byte(0)),
                        string_entry("after_action", "none"),
                        value_entry(
                            "action",
                            compound(vec![
                                string_entry("label", "Go"),
                                string_entry("tooltip", "Tip"),
                                value_entry("width", NbtSummaryValue::Int(200)),
                                value_entry(
                                    "action",
                                    compound(vec![
                                        string_entry("type", "run_command"),
                                        string_entry("command", "say hi"),
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

fn component_with_notice_body_show_dialog(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "Open"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "click_event",
            compound(vec![
                string_entry("action", "show_dialog"),
                value_entry(
                    "dialog",
                    compound(vec![
                        string_entry("type", "notice"),
                        string_entry("title", "Body"),
                        value_entry(
                            "body",
                            compound(vec![
                                string_entry("type", "plain_message"),
                                string_entry("contents", "Message"),
                            ]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_notice_input_show_dialog(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "Open"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "click_event",
            compound(vec![
                string_entry("action", "show_dialog"),
                value_entry(
                    "dialog",
                    compound(vec![
                        string_entry("type", "notice"),
                        string_entry("title", "Inputs"),
                        value_entry(
                            "inputs",
                            NbtSummaryValue::List(vec![compound(vec![
                                string_entry("key", "enabled"),
                                string_entry("type", "boolean"),
                                string_entry("label", "Enabled"),
                            ])]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_confirmation_show_dialog(extra: String) -> NbtSummaryValue {
    compound(vec![
        string_entry("text", "Open"),
        value_entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::String(extra)]),
        ),
        value_entry(
            "click_event",
            compound(vec![
                string_entry("action", "show_dialog"),
                value_entry(
                    "dialog",
                    compound(vec![
                        string_entry("type", "confirmation"),
                        string_entry("title", "Confirm"),
                        value_entry("yes", compound(vec![string_entry("label", "Yes")])),
                        value_entry("no", compound(vec![string_entry("label", "No")])),
                    ]),
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

fn component_with_tool_item_patch(extra: String) -> NbtSummaryValue {
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
                        "tool",
                        compound(vec![
                            value_entry(
                                "rules",
                                NbtSummaryValue::List(vec![compound(vec![
                                    string_entry("blocks", "#example:mineable"),
                                    value_entry("speed", NbtSummaryValue::Float(4.0f32.to_bits())),
                                    value_entry("correct_for_drops", NbtSummaryValue::Byte(1)),
                                ])]),
                            ),
                            value_entry("damage_per_block", NbtSummaryValue::Int(0)),
                            value_entry("can_destroy_blocks_in_creative", NbtSummaryValue::Byte(0)),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_equippable_item_patch(extra: String) -> NbtSummaryValue {
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
                        "equippable",
                        compound(vec![
                            string_entry("slot", "body"),
                            value_entry(
                                "equip_sound",
                                compound(vec![
                                    string_entry("sound_id", "example:equip"),
                                    value_entry("range", NbtSummaryValue::Float(6.5f32.to_bits())),
                                ]),
                            ),
                            string_entry("asset_id", "example:llama_carpet"),
                            string_entry("camera_overlay", "overlay/powder_snow"),
                            string_entry("allowed_entities", "#example:llamas"),
                            value_entry("dispensable", NbtSummaryValue::Byte(0)),
                            value_entry("swappable", NbtSummaryValue::Byte(0)),
                            value_entry("damage_on_hurt", NbtSummaryValue::Byte(0)),
                            value_entry("equip_on_interact", NbtSummaryValue::Byte(1)),
                            value_entry("can_be_sheared", NbtSummaryValue::Byte(1)),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_blocks_attacks_item_patch(extra: String) -> NbtSummaryValue {
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
                        "blocks_attacks",
                        compound(vec![
                            value_entry(
                                "block_delay_seconds",
                                NbtSummaryValue::Float(0.25f32.to_bits()),
                            ),
                            value_entry("disable_cooldown_scale", NbtSummaryValue::Int(2)),
                            value_entry(
                                "damage_reductions",
                                NbtSummaryValue::List(vec![compound(vec![
                                    value_entry(
                                        "horizontal_blocking_angle",
                                        NbtSummaryValue::Float(45.0f32.to_bits()),
                                    ),
                                    string_entry("type", "#example:fire"),
                                    value_entry(
                                        "base",
                                        NbtSummaryValue::Float((-2.0f32).to_bits()),
                                    ),
                                    value_entry("factor", NbtSummaryValue::Float(0.5f32.to_bits())),
                                ])]),
                            ),
                            value_entry(
                                "item_damage",
                                compound(vec![
                                    value_entry(
                                        "threshold",
                                        NbtSummaryValue::Float(3.0f32.to_bits()),
                                    ),
                                    value_entry("base", NbtSummaryValue::Float(1.0f32.to_bits())),
                                    value_entry(
                                        "factor",
                                        NbtSummaryValue::Float((-1.0f32).to_bits()),
                                    ),
                                ]),
                            ),
                            string_entry("bypassed_by", "explosion"),
                            string_entry("block_sound", "item.shield.block"),
                            value_entry(
                                "disabled_sound",
                                compound(vec![
                                    string_entry("sound_id", "example:disable"),
                                    value_entry("range", NbtSummaryValue::Float(8.0f32.to_bits())),
                                ]),
                            ),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_consume_effect_item_patch(extra: String) -> NbtSummaryValue {
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
                            "consumable",
                            compound(vec![
                                value_entry(
                                    "consume_seconds",
                                    NbtSummaryValue::Float(0.5f32.to_bits()),
                                ),
                                string_entry("animation", "drink"),
                                value_entry(
                                    "sound",
                                    compound(vec![
                                        string_entry("sound_id", "example:consume"),
                                        value_entry(
                                            "range",
                                            NbtSummaryValue::Float(4.0f32.to_bits()),
                                        ),
                                    ]),
                                ),
                                value_entry("has_consume_particles", NbtSummaryValue::Byte(0)),
                                value_entry(
                                    "on_consume_effects",
                                    NbtSummaryValue::List(vec![
                                        consume_effect(
                                            "apply_effects",
                                            vec![
                                                value_entry(
                                                    "effects",
                                                    NbtSummaryValue::List(vec![compound(vec![
                                                        string_entry("id", "speed"),
                                                        value_entry(
                                                            "amplifier",
                                                            NbtSummaryValue::Int(2),
                                                        ),
                                                        value_entry(
                                                            "duration",
                                                            NbtSummaryValue::Int(40),
                                                        ),
                                                    ])]),
                                                ),
                                                value_entry(
                                                    "probability",
                                                    NbtSummaryValue::Float(0.25f32.to_bits()),
                                                ),
                                            ],
                                        ),
                                        consume_effect(
                                            "remove_effects",
                                            vec![string_entry("effects", "#example:negative")],
                                        ),
                                        consume_effect("clear_all_effects", Vec::new()),
                                        consume_effect(
                                            "teleport_randomly",
                                            vec![value_entry(
                                                "diameter",
                                                NbtSummaryValue::Float(8.0f32.to_bits()),
                                            )],
                                        ),
                                        consume_effect(
                                            "play_sound",
                                            vec![string_entry("sound", "entity.generic.eat")],
                                        ),
                                    ]),
                                ),
                            ]),
                        ),
                        value_entry(
                            "death_protection",
                            compound(vec![value_entry(
                                "death_effects",
                                NbtSummaryValue::List(vec![consume_effect(
                                    "clear_all_effects",
                                    Vec::new(),
                                )]),
                            )]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn consume_effect(effect_type: &str, mut entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    entries.insert(0, string_entry("type", effect_type));
    compound(entries)
}

fn component_with_potion_contents_item_patch(extra: String) -> NbtSummaryValue {
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
                        "potion_contents",
                        compound(vec![
                            string_entry("potion", "strong_swiftness"),
                            value_entry("custom_color", NbtSummaryValue::Int(0x12_34_56)),
                            value_entry(
                                "custom_effects",
                                NbtSummaryValue::List(vec![compound(vec![
                                    string_entry("id", "speed"),
                                    value_entry("amplifier", NbtSummaryValue::Int(2)),
                                    value_entry("duration", NbtSummaryValue::Int(40)),
                                    value_entry("show_particles", NbtSummaryValue::Byte(0)),
                                    value_entry("show_icon", NbtSummaryValue::Byte(1)),
                                ])]),
                            ),
                            string_entry("custom_name", "example.potion"),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_text_item_patch(extra: String) -> NbtSummaryValue {
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
                            "custom_name",
                            compound(vec![
                                string_entry("text", "Sword"),
                                value_entry("bold", NbtSummaryValue::Byte(1)),
                                string_entry("color", "dark_purple"),
                            ]),
                        ),
                        string_entry("item_name", "Named"),
                        value_entry(
                            "lore",
                            NbtSummaryValue::List(vec![
                                NbtSummaryValue::String("First".to_string()),
                                compound(vec![
                                    string_entry("translate", "item.second"),
                                    value_entry("italic", NbtSummaryValue::Byte(0)),
                                ]),
                            ]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_nested_dialog_item_name(extra: String) -> NbtSummaryValue {
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
                        "custom_name",
                        compound(vec![
                            string_entry("text", "Nested"),
                            value_entry(
                                "click_event",
                                compound(vec![
                                    string_entry("action", "show_dialog"),
                                    value_entry(
                                        "dialog",
                                        compound(vec![
                                            string_entry("type", "notice"),
                                            string_entry("title", "Inner"),
                                        ]),
                                    ),
                                ]),
                            ),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_nested_dialog_written_book_page(extra: String) -> NbtSummaryValue {
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
                        "written_book_content",
                        compound(vec![
                            string_entry("title", "Title"),
                            string_entry("author", "Author"),
                            value_entry(
                                "pages",
                                NbtSummaryValue::List(vec![compound(vec![
                                    string_entry("text", "Nested"),
                                    value_entry(
                                        "click_event",
                                        compound(vec![
                                            string_entry("action", "show_dialog"),
                                            value_entry(
                                                "dialog",
                                                compound(vec![
                                                    string_entry("type", "notice"),
                                                    string_entry("title", "Inner"),
                                                ]),
                                            ),
                                        ]),
                                    ),
                                ])]),
                            ),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_instrument_item_patch(extra: String) -> NbtSummaryValue {
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
                        "instrument",
                        compound(vec![
                            value_entry(
                                "sound_event",
                                compound(vec![
                                    string_entry("sound_id", "example:horn"),
                                    value_entry("range", NbtSummaryValue::Float(16.0f32.to_bits())),
                                ]),
                            ),
                            value_entry("use_duration", NbtSummaryValue::Float(7.0f32.to_bits())),
                            value_entry("range", NbtSummaryValue::Float(256.0f32.to_bits())),
                            value_entry(
                                "description",
                                compound(vec![
                                    string_entry("text", "Ponder"),
                                    value_entry(
                                        "click_event",
                                        compound(vec![
                                            string_entry("action", "show_dialog"),
                                            value_entry(
                                                "dialog",
                                                compound(vec![
                                                    string_entry("type", "notice"),
                                                    string_entry("title", "Horn"),
                                                ]),
                                            ),
                                        ]),
                                    ),
                                ]),
                            ),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_trim_item_patch(extra: String) -> NbtSummaryValue {
    let dialog_description = |text: &str, title: &str| {
        compound(vec![
            string_entry("text", text),
            value_entry(
                "click_event",
                compound(vec![
                    string_entry("action", "show_dialog"),
                    value_entry(
                        "dialog",
                        compound(vec![
                            string_entry("type", "notice"),
                            string_entry("title", title),
                        ]),
                    ),
                ]),
            ),
        ])
    };
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
                            "provides_trim_material",
                            compound(vec![
                                string_entry("asset_name", "quartz"),
                                value_entry(
                                    "override_armor_assets",
                                    compound(vec![
                                        string_entry("iron", "iron_darker"),
                                        string_entry("example:copper", "nested/copper"),
                                    ]),
                                ),
                                value_entry(
                                    "description",
                                    dialog_description("Quartz", "Quartz dialog"),
                                ),
                            ]),
                        ),
                        value_entry(
                            "trim",
                            compound(vec![
                                value_entry(
                                    "material",
                                    compound(vec![
                                        string_entry("asset_name", "resin"),
                                        value_entry(
                                            "description",
                                            dialog_description("Resin", "Material dialog"),
                                        ),
                                    ]),
                                ),
                                value_entry(
                                    "pattern",
                                    compound(vec![
                                        string_entry("asset_id", "sentry"),
                                        value_entry(
                                            "description",
                                            dialog_description("Sentry", "Pattern dialog"),
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

fn component_with_attribute_modifier_item_patch(extra: String) -> NbtSummaryValue {
    let modifier = |attribute: &str,
                    id: &str,
                    amount: f64,
                    operation: &str,
                    slot: Option<&str>,
                    display: Option<NbtSummaryValue>| {
        let mut entries = vec![
            string_entry("type", attribute),
            string_entry("id", id),
            value_entry("amount", NbtSummaryValue::Double(amount.to_bits())),
            string_entry("operation", operation),
        ];
        if let Some(slot) = slot {
            entries.push(string_entry("slot", slot));
        }
        if let Some(display) = display {
            entries.push(value_entry("display", display));
        }
        compound(entries)
    };
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
                        "attribute_modifiers",
                        NbtSummaryValue::List(vec![
                            modifier(
                                "attack_damage",
                                "base_attack_damage",
                                -2.5,
                                "add_value",
                                None,
                                None,
                            ),
                            modifier(
                                "armor",
                                "example:hidden",
                                0.25,
                                "add_multiplied_base",
                                Some("mainhand"),
                                Some(compound(vec![string_entry("type", "hidden")])),
                            ),
                            modifier(
                                "movement_speed",
                                "example:override",
                                1.5,
                                "add_multiplied_total",
                                Some("head"),
                                Some(compound(vec![
                                    string_entry("type", "override"),
                                    value_entry(
                                        "value",
                                        compound(vec![
                                            string_entry("text", "Boost"),
                                            value_entry(
                                                "click_event",
                                                compound(vec![
                                                    string_entry("action", "show_dialog"),
                                                    value_entry(
                                                        "dialog",
                                                        compound(vec![
                                                            string_entry("type", "notice"),
                                                            string_entry("title", "Inner"),
                                                        ]),
                                                    ),
                                                ]),
                                            ),
                                        ]),
                                    ),
                                ])),
                            ),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_recursive_item_template_patch(extra: String) -> NbtSummaryValue {
    let dialog_text = |text: &str, title: &str| {
        compound(vec![
            string_entry("text", text),
            value_entry(
                "click_event",
                compound(vec![
                    string_entry("action", "show_dialog"),
                    value_entry(
                        "dialog",
                        compound(vec![
                            string_entry("type", "notice"),
                            string_entry("title", title),
                        ]),
                    ),
                ]),
            ),
        ])
    };
    let named_component = |name: &str, text: &str, title: &str| {
        compound(vec![value_entry(name, dialog_text(text, title))])
    };
    let item = |id: &str, count: Option<i32>, components: Option<NbtSummaryValue>| {
        let mut entries = vec![string_entry("id", id)];
        if let Some(count) = count {
            entries.push(value_entry("count", NbtSummaryValue::Int(count)));
        }
        if let Some(components) = components {
            entries.push(value_entry("components", components));
        }
        compound(entries)
    };
    let slot = |index: i32, item: NbtSummaryValue| {
        compound(vec![
            value_entry("slot", NbtSummaryValue::Int(index)),
            value_entry("item", item),
        ])
    };
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
                            "use_remainder",
                            item(
                                "glass_bottle",
                                None,
                                Some(named_component(
                                    "custom_name",
                                    "Remainder",
                                    "Remainder dialog",
                                )),
                            ),
                        ),
                        value_entry(
                            "charged_projectiles",
                            NbtSummaryValue::List(vec![
                                item(
                                    "arrow",
                                    None,
                                    Some(named_component(
                                        "custom_name",
                                        "Projectile",
                                        "Projectile dialog",
                                    )),
                                ),
                                item(
                                    "firework_rocket",
                                    Some(3),
                                    Some(compound(vec![value_entry(
                                        "repair_cost",
                                        NbtSummaryValue::Int(4),
                                    )])),
                                ),
                            ]),
                        ),
                        value_entry(
                            "bundle_contents",
                            NbtSummaryValue::List(vec![item(
                                "diamond",
                                None,
                                Some(named_component("item_name", "Bundle", "Bundle dialog")),
                            )]),
                        ),
                        value_entry(
                            "container",
                            NbtSummaryValue::List(vec![
                                slot(2, NbtSummaryValue::String("stone".to_string())),
                                slot(
                                    0,
                                    item(
                                        "diamond",
                                        None,
                                        Some(named_component(
                                            "custom_name",
                                            "Container",
                                            "Container dialog",
                                        )),
                                    ),
                                ),
                                slot(2, item("dirt", Some(5), None)),
                            ]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn component_with_non_finite_tool_item_patch() -> NbtSummaryValue {
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
                        "tool",
                        compound(vec![
                            value_entry("rules", NbtSummaryValue::List(Vec::new())),
                            value_entry(
                                "default_mining_speed",
                                NbtSummaryValue::Float(f32::NEG_INFINITY.to_bits()),
                            ),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_non_finite_equippable_item_patch() -> NbtSummaryValue {
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
                        "equippable",
                        compound(vec![
                            string_entry("slot", "head"),
                            value_entry(
                                "equip_sound",
                                compound(vec![
                                    string_entry("sound_id", "example:equip"),
                                    value_entry(
                                        "range",
                                        NbtSummaryValue::Float(f32::NAN.to_bits()),
                                    ),
                                ]),
                            ),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_non_finite_blocks_attacks_item_patch() -> NbtSummaryValue {
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
                        "blocks_attacks",
                        compound(vec![value_entry(
                            "damage_reductions",
                            NbtSummaryValue::List(vec![compound(vec![
                                value_entry("base", NbtSummaryValue::Float(0.0f32.to_bits())),
                                value_entry("factor", NbtSummaryValue::Float(f32::NAN.to_bits())),
                            ])]),
                        )]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_non_finite_consume_effect_item_patch() -> NbtSummaryValue {
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
                        "death_protection",
                        compound(vec![value_entry(
                            "death_effects",
                            NbtSummaryValue::List(vec![consume_effect(
                                "play_sound",
                                vec![value_entry(
                                    "sound",
                                    compound(vec![
                                        string_entry("sound_id", "example:death"),
                                        value_entry(
                                            "range",
                                            NbtSummaryValue::Float(f32::NAN.to_bits()),
                                        ),
                                    ]),
                                )],
                            )]),
                        )]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_non_finite_text_item_patch() -> NbtSummaryValue {
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
                        "custom_name",
                        compound(vec![
                            string_entry("translate", "example.value"),
                            value_entry(
                                "with",
                                NbtSummaryValue::List(vec![NbtSummaryValue::Float(
                                    f32::NAN.to_bits(),
                                )]),
                            ),
                        ]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_non_finite_attribute_modifier_item_patch() -> NbtSummaryValue {
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
                        "attribute_modifiers",
                        NbtSummaryValue::List(vec![compound(vec![
                            string_entry("type", "armor"),
                            string_entry("id", "example:non_finite"),
                            value_entry("amount", NbtSummaryValue::Double(f64::NAN.to_bits())),
                            string_entry("operation", "add_value"),
                        ])]),
                    )]),
                ),
            ]),
        ),
    ])
}

fn component_with_non_finite_recursive_item_template_patch() -> NbtSummaryValue {
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
                        "bundle_contents",
                        NbtSummaryValue::List(vec![compound(vec![
                            string_entry("id", "diamond"),
                            value_entry(
                                "components",
                                compound(vec![value_entry(
                                    "attribute_modifiers",
                                    NbtSummaryValue::List(vec![compound(vec![
                                        string_entry("type", "armor"),
                                        string_entry("id", "example:non_finite"),
                                        value_entry(
                                            "amount",
                                            NbtSummaryValue::Double(f64::NAN.to_bits()),
                                        ),
                                        string_entry("operation", "add_value"),
                                    ])]),
                                )]),
                            ),
                        ])]),
                    )]),
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
