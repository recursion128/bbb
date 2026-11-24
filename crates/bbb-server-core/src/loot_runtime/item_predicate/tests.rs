use super::*;
use bbb_server_datapack::loot::parse_item_predicate;
use serde_json::{json, Value};

fn p(value: Value) -> ItemPredicate {
    parse_item_predicate(&value).unwrap()
}

fn tool(name: &str, count: i32) -> LootTool {
    LootTool {
        stack: Default::default(),
        item_id: Some(vanilla_item_type_for_resource_id(name).unwrap().id),
        count,
        ..Default::default()
    }
}

fn enchanted_tool(name: &str, enchantment: &str, level: i32) -> LootTool {
    let mut tool = tool(name, 1);
    tool.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(
        bbb_protocol::packets::ItemStackSummary {
            item_id: tool.item_id,
            count: 1,
            component_patch: bbb_registry_codec::read_item_component(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                &bbb_protocol::NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                    name: enchantment.into(),
                    value: bbb_protocol::NbtSummaryValue::Int(level),
                }]),
                crate::inventory::test_item_resolver(),
            )
            .unwrap()
            .into(),
        },
    );
    tool
}

#[test]
fn loaded_item_predicate_tags_replace_generated_membership_without_fallback() {
    let predicate = p(json!({"items":"#minecraft:planks"}));
    let oak = tool("minecraft:oak_planks", 1);
    let dirt = tool("minecraft:dirt", 1);
    let mut tags = ItemTagIndex::new();
    tags.insert("minecraft:planks", [dirt.item_id.unwrap()]);
    assert!(matches_item_predicate(&predicate, &dirt, &tags, &UnavailableHolderResolver).unwrap());
    assert!(!matches_item_predicate(&predicate, &oak, &tags, &UnavailableHolderResolver).unwrap());
    let mut tags = ItemTagIndex::new();
    tags.insert("minecraft:planks", []);
    assert!(!matches_item_predicate(&predicate, &oak, &tags, &UnavailableHolderResolver).unwrap());
    assert!(!matches_item_predicate(&predicate, &dirt, &tags, &UnavailableHolderResolver).unwrap());
    assert!(!matches_item_predicate(
        &predicate,
        &oak,
        &ItemTagIndex::new(),
        &UnavailableHolderResolver
    )
    .unwrap());
    assert!(item_predicate_matches(&predicate, &oak, &UnavailableHolderResolver).unwrap());
    assert!(!item_predicate_matches(&predicate, &dirt, &UnavailableHolderResolver).unwrap());
}

#[test]
fn loaded_item_predicate_holder_lists_counts_and_empty_air_use_the_actual_stack() {
    let mut tags = ItemTagIndex::new();
    let dirt = tool("minecraft:dirt", 4);
    let predicate = p(json!({"items":["stone", "dirt"],"count":{"min":2,"max":4}}));
    assert!(matches_item_predicate(&predicate, &dirt, &tags, &UnavailableHolderResolver).unwrap());
    assert!(!matches_item_predicate(
        &predicate,
        &tool("minecraft:dirt", 1),
        &tags,
        &UnavailableHolderResolver
    )
    .unwrap());
    assert!(!matches_item_predicate(
        &predicate,
        &tool("minecraft:dirt", 5),
        &tags,
        &UnavailableHolderResolver
    )
    .unwrap());
    assert!(matches_item_predicate(
        &p(json!({"items":"dirt","count":4})),
        &dirt,
        &tags,
        &UnavailableHolderResolver
    )
    .unwrap());
    let air = tool("minecraft:air", 17);
    tags.insert("test:air", [air.item_id.unwrap()]);
    for empty in [LootTool::empty(), tool("minecraft:dirt", 0), air] {
        assert!(matches_item_predicate(
            &p(json!({"items":"#test:air","count":0})),
            &empty,
            &tags,
            &UnavailableHolderResolver
        )
        .unwrap());
        assert!(!matches_item_predicate(
            &p(json!({"items":"dirt"})),
            &empty,
            &tags,
            &UnavailableHolderResolver
        )
        .unwrap());
    }
}

#[test]
fn loaded_item_predicate_keeps_enchantment_matching() {
    let tags = ItemTagIndex::new();
    let mut pickaxe = enchanted_tool("minecraft:diamond_pickaxe", "minecraft:fortune", 2);
    let predicate = p(
        json!({"predicates":{"minecraft:enchantments":[{"enchantments":"fortune","levels":{"min":2}}]}}),
    );
    assert!(matches_item_predicate(
        &predicate,
        &pickaxe,
        &tags,
        crate::inventory::test_item_resolver()
    )
    .unwrap());
    assert!(
        item_predicate_matches(&predicate, &pickaxe, crate::inventory::test_item_resolver())
            .unwrap()
    );
    pickaxe.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(
        bbb_protocol::packets::ItemStackSummary {
            item_id: pickaxe.item_id,
            count: 1,
            component_patch: Default::default(),
        },
    );
    assert!(!matches_item_predicate(
        &predicate,
        &pickaxe,
        &tags,
        crate::inventory::test_item_resolver()
    )
    .unwrap());
}

#[test]
fn enchantment_tags_resolve_through_the_item_publication() {
    let sword = enchanted_tool("minecraft:diamond_sword", "minecraft:fire_aspect", 1);
    let smelts = p(
        json!({"predicates":{"minecraft:enchantments":[{"enchantments":"#minecraft:smelts_loot"}]}}),
    );
    assert!(matches_item_predicate(
        &smelts,
        &sword,
        &ItemTagIndex::new(),
        crate::inventory::test_item_resolver()
    )
    .unwrap());
    let plain = enchanted_tool("minecraft:diamond_sword", "minecraft:sharpness", 5);
    assert!(!matches_item_predicate(
        &smelts,
        &plain,
        &ItemTagIndex::new(),
        crate::inventory::test_item_resolver()
    )
    .unwrap());
    let unknown =
        p(json!({"predicates":{"minecraft:enchantments":[{"enchantments":"#test:enchants"}]}}));
    assert!(!matches_item_predicate(
        &unknown,
        &sword,
        &ItemTagIndex::new(),
        crate::inventory::test_item_resolver()
    )
    .unwrap());
}

#[test]
fn custom_data_item_predicate_matches_real_stack_subsets_and_defaults() {
    let resolver = crate::inventory::test_item_resolver();
    let mut stack = tool("minecraft:stone", 1);
    let data = bbb_protocol::nbt_summary::parse_snbt_compound(
        "{quest:{stage:2b,finished:0b},steps:[{id:1,done:1b},{id:2}],extra:42}",
    )
    .unwrap();
    stack.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(
        bbb_protocol::packets::ItemStackSummary {
            item_id: stack.item_id,
            count: 1,
            component_patch: bbb_registry_codec::read_item_component(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                &data,
                resolver,
            )
            .unwrap()
            .into(),
        },
    );
    let matches = |data: &str, stack: &LootTool| {
        item_predicate_matches(
            &p(json!({"predicates":{"minecraft:custom_data":data}})),
            stack,
            resolver,
        )
        .unwrap()
    };
    assert!(matches("{quest:{stage:2b},steps:[{id:2},{id:1}]}", &stack));
    assert!(!matches("{quest:{stage:3b}}", &stack));
    stack.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(
        bbb_protocol::packets::ItemStackSummary {
            item_id: stack.item_id,
            count: 1,
            component_patch: Default::default(),
        },
    );
    for absent in [&stack, &LootTool::empty()] {
        assert!(matches("{}", absent));
        assert!(!matches("{quest:{}}", absent));
    }
}

fn component_tool(name: &str, component: i32, contents: &str) -> LootTool {
    component_tool_with_resolver(
        name,
        component,
        contents,
        crate::inventory::test_item_resolver(),
    )
}

fn component_tool_with_resolver(
    name: &str,
    component: i32,
    contents: &str,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> LootTool {
    let mut book = tool(name, 1);
    let bbb_protocol::NbtSummaryValue::Compound(mut wrapper) =
        bbb_protocol::nbt_summary::parse_snbt_compound(&format!("{{value:{contents}}}")).unwrap()
    else {
        unreachable!()
    };
    let contents = wrapper.remove(0).value;
    book.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(
        bbb_protocol::packets::ItemStackSummary {
            item_id: book.item_id,
            count: 1,
            component_patch: bbb_registry_codec::read_item_component(
                component, &contents, resolver,
            )
            .unwrap()
            .into(),
        },
    );
    book
}

#[test]
fn writable_book_predicate_matches_raw_pages_with_collection_constraints() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID;
    let book = component_tool(
        "minecraft:writable_book",
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        r#"{pages:[{raw:"secret",filtered:"safe"},"secret","other"]}"#,
    );
    let matches = |pages| {
        item_predicate_matches(
            &p(json!({"predicates":{"minecraft:writable_book_content":{"pages":pages}}})),
            &book,
            crate::inventory::test_item_resolver(),
        )
        .unwrap()
    };
    assert!(matches(json!({
        "contains":["secret","secret","other"],
        "count":[{"test":"secret","count":2},{"test":"safe","count":0}],
        "size":3
    })));
    assert!(!matches(json!({"contains":["safe"]})));
    assert!(!matches(json!({"count":[{"test":"secret","count":1}]})));
    assert!(!matches(json!({"size":{"max":2}})));
    assert!(matches(json!({"contains":[],"count":[]})));
}

#[test]
fn written_book_predicate_matches_raw_title_and_component_page_style() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID;
    let book = component_tool(
        "minecraft:written_book",
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
        r#"{title:{raw:"Secret",filtered:"Safe"},author:"Writer",generation:2,resolved:1b,pages:[{raw:{text:"hello",bold:1b,extra:["!"]},filtered:"safe"}]}"#,
    );
    let matches = |predicate| {
        item_predicate_matches(
            &p(json!({"predicates":{"minecraft:written_book_content":predicate}})),
            &book,
            crate::inventory::test_item_resolver(),
        )
        .unwrap()
    };
    assert!(matches(json!({
        "author":"Writer","title":"Secret","generation":{"min":1,"max":2},"resolved":true,
        "pages":{"contains":[{"text":"hello","bold":true,"extra":["!"]}],
        "count":[{"test":{"text":"hello","bold":true,"extra":["!"]},"count":1}],"size":1}
    })));
    for predicate in [
        json!({"author":"Other"}),
        json!({"title":"Safe"}),
        json!({"generation":1}),
        json!({"resolved":false}),
        json!({"pages":{"contains":["safe"]}}),
        json!({"pages":{"contains":["hello!"]}}),
        json!({"pages":{"contains":[{"text":"hello","bold":false,"extra":["!"]}]}}),
    ] {
        assert!(!matches(predicate));
    }
}

#[test]
fn empty_value_predicates_require_present_components() {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    };
    let resolver = crate::inventory::test_item_resolver();
    for (name, type_id, item, contents) in [
        (
            "minecraft:attribute_modifiers",
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
            "minecraft:stone",
            "[]",
        ),
        (
            "minecraft:trim",
            VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
            "minecraft:diamond_chestplate",
            r#"{material:"minecraft:gold",pattern:"minecraft:sentry"}"#,
        ),
        (
            "minecraft:jukebox_playable",
            VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
            "minecraft:stone",
            r#""minecraft:cat""#,
        ),
        (
            "minecraft:firework_explosion",
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            "minecraft:firework_star",
            r#"{shape:"small_ball"}"#,
        ),
        (
            "minecraft:fireworks",
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            "minecraft:firework_rocket",
            "{}",
        ),
        (
            "minecraft:writable_book_content",
            VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
            "minecraft:writable_book",
            "{}",
        ),
        (
            "minecraft:written_book_content",
            VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
            "minecraft:written_book",
            r#"{title:"Title",author:"Writer"}"#,
        ),
    ] {
        let predicate = p(json!({"predicates":{name:{}}}));
        let mut book = component_tool(item, type_id, contents);
        assert!(item_predicate_matches(&predicate, &book, resolver).unwrap());
        book.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(
            bbb_protocol::packets::ItemStackSummary {
                item_id: book.item_id,
                count: 1,
                component_patch: bbb_protocol::packets::DataComponentPatchSummary {
                    removed_type_ids: vec![type_id],
                    ..Default::default()
                }
                .into(),
            },
        );
        assert!(!item_predicate_matches(&predicate, &book, resolver).unwrap());
        let mut stone = tool("minecraft:stone", 1);
        stone.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(
            bbb_protocol::packets::ItemStackSummary {
                item_id: stone.item_id,
                count: 1,
                component_patch: Default::default(),
            },
        );
        assert_eq!(
            item_predicate_matches(&predicate, &stone, resolver).unwrap(),
            name == "minecraft:attribute_modifiers",
        );
        assert!(!item_predicate_matches(&predicate, &LootTool::empty(), resolver).unwrap());
    }
}

#[test]
fn container_predicate_matches_sparse_stacks_and_preserves_loaded_nested_tags() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID;
    let resolver = crate::inventory::test_item_resolver();
    let container = component_tool(
        "minecraft:shulker_box",
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
        r#"[{slot:2,item:{id:"minecraft:dirt",count:7}},{slot:26,item:{id:"minecraft:diamond",count:3}}]"#,
    );
    let mut tags = ItemTagIndex::new();
    tags.insert(
        "minecraft:planks",
        [vanilla_item_type_for_resource_id("minecraft:dirt")
            .unwrap()
            .id],
    );
    let matches = |items| {
        matches_item_predicate(
            &p(json!({"predicates":{"minecraft:container":{"items":items}}})),
            &container,
            &tags,
            resolver,
        )
        .unwrap()
    };
    assert!(matches(json!({
        "size":2,
        "contains":[{"items":"#minecraft:planks","count":7},{"items":"diamond","count":3}],
        "count":[{"test":{"count":{"min":4}},"count":1}]
    })));
    assert!(!matches(json!({"size":27})));
    assert!(!matches(json!({"size":10})));
    assert!(!matches(json!({"contains":[{"items":"dirt","count":3}]})));
    assert!(!matches(
        json!({"count":[{"test":{"items":"#minecraft:planks"},"count":0}]})
    ));
}

#[test]
fn bundle_predicate_reads_nested_components() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID;
    let bundle = component_tool(
        "minecraft:bundle",
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        r#"[{id:"minecraft:stone",count:4,components:{"minecraft:custom_data":{quest:{stage:2b}}}},{id:"minecraft:writable_book",count:1,components:{"minecraft:writable_book_content":{pages:["draft"]}}}]"#,
    );
    let predicate = p(json!({"predicates":{"minecraft:bundle_contents":{"items":{
        "size":2,
        "contains":[
            {"count":4,"predicates":{"minecraft:custom_data":"{quest:{stage:2b}}"}},
            {"predicates":{"minecraft:writable_book_content":{"pages":{"contains":["draft"]}}}}
        ]
    }}}}));
    assert!(
        item_predicate_matches(&predicate, &bundle, crate::inventory::test_item_resolver())
            .unwrap()
    );
}

#[test]
fn contents_predicates_distinguish_empty_missing_and_removed_components() {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
    };
    let resolver = crate::inventory::test_item_resolver();
    for (id, type_id, item) in [
        (
            "minecraft:container",
            VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
            "minecraft:shulker_box",
        ),
        (
            "minecraft:bundle_contents",
            VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
            "minecraft:bundle",
        ),
    ] {
        let mut contents = component_tool(item, type_id, "[]");
        for value in [json!({}), json!({"items":{"size":0}})] {
            assert!(item_predicate_matches(
                &p(json!({"predicates":{id:value}})),
                &contents,
                resolver
            )
            .unwrap());
        }
        contents.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(
            bbb_protocol::packets::ItemStackSummary {
                item_id: contents.item_id,
                count: 1,
                component_patch: bbb_protocol::packets::DataComponentPatchSummary {
                    removed_type_ids: vec![type_id],
                    ..Default::default()
                }
                .into(),
            },
        );
        let absent = component_tool(
            "minecraft:stone",
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            "{}",
        );
        for tool in [&contents, &absent, &LootTool::empty()] {
            assert!(
                !item_predicate_matches(&p(json!({"predicates":{id:{}}})), tool, resolver).unwrap()
            );
        }
    }
}

#[test]
fn firework_predicates_match_shape_effects_and_rocket_collections() {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID, VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
    };
    let resolver = crate::inventory::test_item_resolver();
    let star = component_tool(
        "minecraft:firework_star",
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
        r#"{shape:"star",has_trail:true,colors:[I;11743532]}"#,
    );
    for (value, expected) in [
        (
            json!({"shape":"star","has_twinkle":false,"has_trail":true}),
            true,
        ),
        (json!({"shape":"burst"}), false),
        (json!({"has_twinkle":true}), false),
        (json!({"has_trail":false}), false),
    ] {
        assert_eq!(
            item_predicate_matches(
                &p(json!({"predicates":{"minecraft:firework_explosion":value}})),
                &star,
                resolver
            )
            .unwrap(),
            expected
        );
    }
    let rocket = component_tool(
        "minecraft:firework_rocket",
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        r#"{flight_duration:2,explosions:[{shape:"star",has_trail:true},{shape:"burst",has_twinkle:true},{shape:"star"}]}"#,
    );
    for (value, expected) in [
        (
            json!({"flight_duration":{"min":1,"max":2},"explosions":{
                "size":3,"contains":[{"shape":"burst","has_twinkle":true}],
                "count":[{"test":{"shape":"star","has_twinkle":false},"count":2}]
            }}),
            true,
        ),
        (json!({"flight_duration":{"min":3}}), false),
        (json!({"explosions":{"size":2}}), false),
        (
            json!({"explosions":{"contains":[{"shape":"creeper"}]}}),
            false,
        ),
        (
            json!({"explosions":{"count":[{"test":{"has_trail":true},"count":2}]}}),
            false,
        ),
    ] {
        assert_eq!(
            item_predicate_matches(
                &p(json!({"predicates":{"minecraft:fireworks":value}})),
                &rocket,
                resolver
            )
            .unwrap(),
            expected
        );
    }
    let empty = component_tool(
        "minecraft:firework_rocket",
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        "{}",
    );
    assert!(item_predicate_matches(&p(json!({"predicates":{"minecraft:fireworks":{"flight_duration":0,"explosions":{"size":0}}}})), &empty, resolver).unwrap());
}

#[test]
fn registry_value_predicates_use_published_holders_and_tags() {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
    };
    use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
    let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
    for registry in [
        "minecraft:trim_material",
        "minecraft:trim_pattern",
        "minecraft:jukebox_song",
    ] {
        startup
            .registries
            .registries
            .iter_mut()
            .find(|loaded| loaded.registry_id == registry)
            .unwrap()
            .elements
            .reverse();
    }
    for (registry, member) in [
        ("minecraft:trim_material", "minecraft:gold"),
        ("minecraft:trim_pattern", "minecraft:sentry"),
        ("minecraft:jukebox_song", "minecraft:cat"),
        ("minecraft:villager_type", "minecraft:desert"),
    ] {
        startup
            .tags
            .registries
            .retain(|loaded| loaded.registry_id != registry);
        startup
            .tags
            .registries
            .push(bbb_server_datapack::tag_loader::LoadedRegistryTags {
                registry_id: registry.into(),
                tags: [("test:chosen".into(), vec![member.into()])]
                    .into_iter()
                    .collect(),
            });
    }
    let resolver = FrozenDatapackRegistryLookup::from_startup(&startup);
    let matches = |name: &str, value: Value, tool: &LootTool| {
        item_predicate_matches(&p(json!({"predicates":{name:value}})), tool, &resolver).unwrap()
    };
    let trim = component_tool_with_resolver(
        "minecraft:diamond_chestplate",
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        r#"{material:"minecraft:gold",pattern:"minecraft:sentry"}"#,
        &resolver,
    );
    assert!(matches(
        "minecraft:trim",
        json!({"material":"#test:chosen","pattern":["sentry"]}),
        &trim
    ));
    assert!(!matches(
        "minecraft:trim",
        json!({"material":"iron","pattern":"sentry"}),
        &trim
    ));
    assert!(!matches(
        "minecraft:trim",
        json!({"material":"gold","pattern":"coast"}),
        &trim
    ));
    assert!(!matches(
        "minecraft:trim",
        json!({"material":"#test:missing"}),
        &trim
    ));
    let song = component_tool_with_resolver(
        "minecraft:stone",
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        r#""minecraft:cat""#,
        &resolver,
    );
    assert!(matches(
        "minecraft:jukebox_playable",
        json!({"song":"#test:chosen"}),
        &song
    ));
    assert!(!matches(
        "minecraft:jukebox_playable",
        json!({"song":"13"}),
        &song
    ));
    let mut inline_song = song.clone();
    let mut summary = inline_song.stack.summary().clone();
    summary.component_patch.jukebox_song_id = None;
    summary.component_patch.jukebox_direct_song = Some(Box::new(
        bbb_protocol::packets::data_components::JukeboxSongSummary {
            sound_event: bbb_protocol::packets::data_components::SoundEventSummary {
                registry_id: None,
                sound_id: Some("minecraft:music_disc.cat".into()),
                fixed_range_bits: None,
            },
            description: "Cat".into(),
            description_styled: None,
            description_nbt: None,
            length_in_seconds_bits: 185.0f32.to_bits(),
            comparator_output: 2,
        },
    ));
    inline_song.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(summary);
    assert!(matches(
        "minecraft:jukebox_playable",
        json!({}),
        &inline_song
    ));
    assert!(!matches(
        "minecraft:jukebox_playable",
        json!({"song":"cat"}),
        &inline_song
    ));
    let mut variant = component_tool(
        "minecraft:stone",
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
        r#""minecraft:desert""#,
    );
    assert!(matches(
        "minecraft:villager/variant",
        json!(["plains", "desert"]),
        &variant
    ));
    assert!(matches(
        "minecraft:villager/variant",
        json!("#test:chosen"),
        &variant
    ));
    assert!(!matches(
        "minecraft:villager/variant",
        json!("plains"),
        &variant
    ));
    assert!(!matches(
        "minecraft:villager/variant",
        json!("#test:missing"),
        &variant
    ));
    let mut summary = variant.stack.summary().clone();
    summary.component_patch = Default::default();
    variant.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(summary.clone());
    assert!(!matches(
        "minecraft:villager/variant",
        json!("desert"),
        &variant
    ));
    summary.component_patch.removed_type_ids =
        vec![VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID];
    variant.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(summary);
    assert!(!matches(
        "minecraft:villager/variant",
        json!("desert"),
        &variant
    ));
}

#[test]
fn attribute_modifier_predicates_match_entries_and_effective_defaults() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID;
    use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
    let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
    startup
        .tags
        .registries
        .retain(|loaded| loaded.registry_id != "minecraft:attribute");
    startup
        .tags
        .registries
        .push(bbb_server_datapack::tag_loader::LoadedRegistryTags {
            registry_id: "minecraft:attribute".into(),
            tags: [("test:damage".into(), vec!["minecraft:attack_damage".into()])]
                .into_iter()
                .collect(),
        });
    let resolver = FrozenDatapackRegistryLookup::from_startup(&startup);
    let modifiers = component_tool(
        "minecraft:stone",
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        r#"[{type:"minecraft:attack_damage",id:"test:power",amount:2.5d,operation:"add_value",slot:"mainhand"},{type:"minecraft:attack_speed",id:"test:speed",amount:0.25d,operation:"add_multiplied_base",slot:"hand"}]"#,
    );
    let matches = |value: Value, tool: &LootTool| {
        item_predicate_matches(
            &p(json!({"predicates":{"minecraft:attribute_modifiers":value}})),
            tool,
            &resolver,
        )
        .unwrap()
    };
    assert!(matches(
        json!({"modifiers":{"size":2,"contains":[{"attribute":"#test:damage","id":"test:power","amount":{"min":2.0,"max":3.0},"operation":"add_value","slot":"mainhand"}],"count":[{"test":{"slot":"hand"},"count":1}]}}),
        &modifiers
    ));
    for entry in [
        json!({"attribute":"#test:missing"}),
        json!({"id":"test:missing"}),
        json!({"attribute":"attack_damage","slot":"hand"}),
        json!({"id":"test:power","amount":1.0}),
        json!({"id":"test:power","operation":"add_multiplied_total"}),
    ] {
        assert!(!matches(
            json!({"modifiers":{"contains":[entry]}}),
            &modifiers
        ));
    }
    let defaults = component_tool(
        "minecraft:diamond_sword",
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        "{}",
    );
    assert!(matches(
        json!({"modifiers":{"contains":[{"attribute":"attack_damage","slot":"mainhand"}]}}),
        &defaults
    ));
    let empty = component_tool(
        "minecraft:stone",
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        "[]",
    );
    assert!(matches(
        json!({"modifiers":{"size":0,"count":[{"test":{},"count":0}]}}),
        &empty
    ));
    assert!(!matches(json!({"modifiers":{"contains":[{}]}}), &empty));
}
