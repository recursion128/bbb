use bbb_protocol::packets::{decode_configuration_clientbound, ConfigurationClientbound};
use bbb_protocol::{ids, packets::encode_configuration_registry_data, NbtSummaryValue};
use serde_json::Value;

use super::*;

fn builtin_startup() -> DatapackStartup {
    DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap()
}

#[test]
fn gameplay_tags_freeze_fixed_and_worldgen_membership_from_the_builtin_startup() {
    let registries = SynchronizedRegistries::load(None).unwrap();
    let tags = registries.registry_tags();

    assert!(tags.contains(
        "minecraft:block",
        "minecraft:mineable/pickaxe",
        "minecraft:stone"
    ));
    assert!(tags.contains(
        "minecraft:timeline",
        "minecraft:in_overworld",
        "minecraft:day"
    ));
}

#[test]
fn gameplay_tags_distinguish_missing_and_empty_tags() {
    let mut startup = builtin_startup();
    startup
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:block")
        .unwrap()
        .tags
        .insert("example:empty".to_string(), Vec::new());
    let registries = SynchronizedRegistries::load(Some(&startup)).unwrap();
    let tags = registries.registry_tags();

    assert!(!tags.contains("minecraft:block", "example:missing", "minecraft:stone"));
    assert!(!tags.tag_exists("minecraft:block", "example:missing"));
    assert!(tags.tag_exists("minecraft:block", "example:empty"));
    assert!(tags.members("minecraft:block", "example:empty").is_empty());
}

#[test]
fn supplied_startup_fixed_tags_replace_and_extend_builtin_membership() {
    let mut startup = builtin_startup();
    let block_tags = &mut startup
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:block")
        .unwrap()
        .tags;
    block_tags.insert("minecraft:mineable/pickaxe".to_string(), Vec::new());
    block_tags.insert(
        "example:runtime_only".to_string(),
        vec!["minecraft:stone".to_string()],
    );

    let registries = SynchronizedRegistries::load(Some(&startup)).unwrap();
    assert!(!registries.registry_tags().contains(
        "minecraft:block",
        "minecraft:mineable/pickaxe",
        "minecraft:stone"
    ));
    assert!(registries.registry_tags().contains(
        "minecraft:block",
        "example:runtime_only",
        "minecraft:stone"
    ));
}

/// The built-in pack publishes its four dimension types in the order the
/// official server registers them, and every index this server hands out is a
/// position in that same list.
///
/// The order is not alphabetical on the element id by accident: vanilla sorts
/// the *resource* ids the elements were listed under
/// (`ResourceManagerRegistryLoadTask.java:60-62`), which is what puts
/// `overworld_caves` between `overworld` and `the_end`.
#[test]
fn the_published_dimension_types_are_the_built_in_pack_in_registration_order() {
    let registries = SynchronizedRegistries::load(None).unwrap();
    assert_eq!(
        registries.dimension_type_ids().collect::<Vec<_>>(),
        vec![
            "minecraft:overworld",
            "minecraft:overworld_caves",
            "minecraft:the_end",
            "minecraft:the_nether",
        ]
    );
    // The three levels a save declares therefore do not share one index, which
    // is the whole reason the registry is published.
    assert_eq!(
        registries.dimension_type_index("minecraft:overworld"),
        Some(0)
    );
    assert_eq!(
        registries.dimension_type_index("minecraft:the_end"),
        Some(2)
    );
    assert_eq!(
        registries.dimension_type_index("minecraft:the_nether"),
        Some(3)
    );
    assert_eq!(registries.dimension_type_index("minecraft:nowhere"), None);
}

/// The index a level is described by is the position of its element in the
/// packet the configuration phase actually writes.
///
/// This is the assertion the wire contract rests on: the client rebuilds the
/// registry by registering the entries in arrival order
/// (`RegistryLoadTask.java:62-71`), so an index that is not a position in the
/// encoded packet names a different dimension type than the one the server
/// meant.
#[test]
fn every_published_index_is_a_position_in_the_encoded_packet() {
    let registries = SynchronizedRegistries::load(None).unwrap();
    let published = registries.published();
    assert_eq!(published.len(), 28);
    let dimension = published
        .iter()
        .find(|registry| registry.registry == "minecraft:dimension_type")
        .expect("the synchronized catalog contains dimension types");

    let (packet_id, payload) =
        encode_configuration_registry_data(&dimension.registry, &dimension.entries).unwrap();
    assert_eq!(packet_id, ids::configuration::CLIENTBOUND_REGISTRY_DATA);
    let ConfigurationClientbound::RegistryData(data) =
        decode_configuration_clientbound(packet_id, &payload).unwrap()
    else {
        panic!("the configuration reader answers with the registry it was sent");
    };

    for element_id in registries.dimension_type_ids() {
        let index = registries
            .dimension_type_index(element_id)
            .expect("a published element has an index");
        assert_eq!(
            data.entries[usize::try_from(index).unwrap()].id,
            element_id,
            "index {index} does not name {element_id} on the wire"
        );
    }
    assert_eq!(data.entries.len(), registries.dimension_type_ids().count());
}

#[test]
fn published_synchronized_catalog_includes_biome_payloads() {
    let registries = SynchronizedRegistries::load(None).unwrap();
    let published = registries.published();
    assert_eq!(
        published
            .iter()
            .map(|registry| registry.registry.as_str())
            .collect::<Vec<_>>(),
        bbb_protocol::registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES
            .iter()
            .map(|data_type| data_type.resource_id)
            .collect::<Vec<_>>()
    );
    let biome = published
        .iter()
        .find(|registry| registry.registry == "minecraft:worldgen/biome")
        .expect("the synchronized catalog contains biomes");
    assert!(!biome.entries.is_empty());
    assert!(biome.entries.iter().all(|entry| entry
        .raw_data
        .as_ref()
        .is_some_and(|payload| !payload.is_empty())));
    let plains = biome
        .entries
        .iter()
        .find(|entry| entry.id == "minecraft:plains")
        .expect("the built-in pack declares plains");
    let plains_index = biome
        .entries
        .iter()
        .position(|entry| entry.id == "minecraft:plains")
        .expect("the built-in pack declares plains");
    assert_ne!(plains_index, 0, "plains is not the first published biome");
    assert_eq!(
        registries.registry_element_index("minecraft:worldgen/biome", "minecraft:plains"),
        i32::try_from(plains_index).ok()
    );
    assert_eq!(
        registries.registry_element_index("minecraft:worldgen/biome", "minecraft:missing"),
        None
    );
    assert_eq!(
        registries.registry_element_id("minecraft:worldgen/biome", plains_index as i32),
        Some("minecraft:plains")
    );
    assert_eq!(
        registries.registry_len("minecraft:worldgen/biome"),
        Some(biome.entries.len())
    );
    assert_eq!(
        registries.registry_element_id("minecraft:worldgen/biome", -1),
        None
    );
    assert_eq!(
        registries.registry_element_index("minecraft:missing", "minecraft:plains"),
        None
    );
    let NbtSummaryValue::Compound(fields) = bbb_protocol::decode_nbt_summary(
        plains.raw_data.as_ref().expect("contents are never elided"),
    )
    .unwrap()
    .unwrap() else {
        panic!("a biome is a compound");
    };
    assert!(fields.iter().any(|entry| entry.name == "has_precipitation"));
    assert!(!fields.iter().any(|entry| entry.name == "carvers"));
    assert!(!fields.iter().any(|entry| entry.name == "features"));
}

/// Every published element carries its contents, because this server offers no
/// known packs for a client to have accepted
/// (`RegistrySynchronization.java:51-57`), and those contents decode back to
/// the element the built-in pack declares.
#[test]
fn a_published_element_carries_the_document_the_built_in_pack_declares() {
    let registries = SynchronizedRegistries::load(None).unwrap();
    let published = registries.published();
    let dimension = published
        .iter()
        .find(|registry| registry.registry == "minecraft:dimension_type")
        .expect("the synchronized catalog contains dimension types");
    let nether = dimension
        .entries
        .iter()
        .find(|entry| entry.id == "minecraft:the_nether")
        .expect("the built-in pack declares the nether");
    let raw = nether.raw_data.as_ref().expect("contents are never elided");
    let value = bbb_protocol::decode_nbt_summary(raw)
        .unwrap()
        .expect("the payload is a tag, not TAG_End");
    let NbtSummaryValue::Compound(fields) = value else {
        panic!("a dimension type is a compound");
    };
    let field = |name: &str| {
        fields
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.value.clone())
    };
    // `data/minecraft/dimension_type/the_nether.json`, which is what
    // `DimensionTypes.NETHER` bakes (`DimensionTypes.java:64-99`).
    assert_eq!(field("min_y"), Some(NbtSummaryValue::Int(0)));
    assert_eq!(field("height"), Some(NbtSummaryValue::Int(256)));
    assert_eq!(field("logical_height"), Some(NbtSummaryValue::Int(128)));
    assert_eq!(field("has_ceiling"), Some(NbtSummaryValue::Byte(1)));
    assert_eq!(field("has_skylight"), Some(NbtSummaryValue::Byte(0)));
    assert_eq!(
        field("coordinate_scale"),
        Some(NbtSummaryValue::Double(8.0_f64.to_bits()))
    );
    assert_eq!(
        field("infiniburn"),
        Some(NbtSummaryValue::string("#minecraft:infiniburn_nether"))
    );
    // The optional fields the record defaults are absent when the document
    // leaves them out and present when it does not
    // (`DimensionType.java:87`, `:98`).
    assert_eq!(field("has_fixed_time"), Some(NbtSummaryValue::Byte(1)));
    assert_eq!(field("skybox"), Some(NbtSummaryValue::string("none")));
    assert!(field("default_clock").is_none());
    // The overworld's own document leaves `has_fixed_time` and `skybox` out, so
    // the publish is carrying the document rather than filling it in.
    let overworld = dimension
        .entries
        .iter()
        .find(|entry| entry.id == "minecraft:overworld")
        .expect("the built-in pack declares the overworld");
    let NbtSummaryValue::Compound(overworld) = bbb_protocol::decode_nbt_summary(
        overworld
            .raw_data
            .as_ref()
            .expect("contents are never elided"),
    )
    .unwrap()
    .unwrap() else {
        panic!("a dimension type is a compound");
    };
    assert!(!overworld.iter().any(|entry| entry.name == "has_fixed_time"));
    assert!(!overworld.iter().any(|entry| entry.name == "skybox"));
    assert!(overworld.iter().any(|entry| entry.name == "attributes"));
}

/// A datapack that replaced a dimension type is published as the pack loaded
/// it, because the loaded generation is what a level was sized from.
#[test]
fn a_loaded_generation_wins_over_the_built_in_pack() {
    let mut startup = builtin_startup();
    let registry = startup
        .registries
        .registry(DIMENSION_TYPE_REGISTRY)
        .unwrap();
    let mut json = serde_json::from_slice::<serde_json::Value>(
        &(registry
            .elements
            .iter()
            .find(|element| element.id == "minecraft:overworld")
            .unwrap())
        .source_json,
    )
    .unwrap();
    json["min_y"] = Value::from(-32);
    json["height"] = Value::from(128);
    json["logical_height"] = Value::from(128);
    let overworld = bbb_server_datapack::dynamic_registry::RawRegistryElement {
        id: "minecraft:overworld".to_string(),
        source_pack: "test".to_string(),
        source_json: serde_json::to_vec(&json).unwrap().into(),
        source: Some(
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(json)).unwrap())
                .unwrap(),
        ),
        codec_value: None,
    };
    startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == DIMENSION_TYPE_REGISTRY)
        .unwrap()
        .elements = vec![overworld];
    for animal in ["cow", "chicken", "pig"] {
        let registry = startup
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == format!("minecraft:{animal}_sound_variant"))
            .unwrap();
        let mut json =
            serde_json::from_slice::<serde_json::Value>(&registry.elements[0].source_json).unwrap();
        let inline = serde_json::json!({"sound_id": "example:ambient", "range": 7.5});
        if animal == "cow" {
            json["ambient_sound"] = inline;
        } else {
            json["adult_sounds"]["ambient_sound"] = Value::from("minecraft:entity.cow.ambient");
            json["baby_sounds"]["ambient_sound"] = inline;
        }
        registry
            .elements
            .push(bbb_server_datapack::dynamic_registry::RawRegistryElement {
                id: "example:custom".to_string(),
                source_pack: "test".to_string(),
                source_json: serde_json::to_vec(&json).unwrap().into(),
                source: Some(
                    bbb_registry_codec::decode_direct_registry_json(
                        &serde_json::to_vec(&(json)).unwrap(),
                    )
                    .unwrap(),
                ),
                codec_value: None,
            });
    }
    let registries = SynchronizedRegistries::load(Some(&startup)).unwrap();
    use bbb_registry_codec::registry_codec::sound_variants::SoundEventHolder;
    let SoundEventHolder::Inline(sound) = &registries
        .cow_sound_variant("example:custom")
        .unwrap()
        .ambient_sound
    else {
        panic!("the loaded cow's inline sound is retained");
    };
    assert_eq!(sound.sound_id.as_str(), "example:ambient");
    assert_eq!(sound.range.unwrap().to_f32(), 7.5);
    let chicken = registries.chicken_sound_variant("example:custom").unwrap();
    let pig = registries.pig_sound_variant("example:custom").unwrap();
    for (adult, baby) in [
        (
            &chicken.adult_sounds.ambient_sound,
            &chicken.baby_sounds.ambient_sound,
        ),
        (
            &pig.adult_sounds.ambient_sound,
            &pig.baby_sounds.ambient_sound,
        ),
    ] {
        let SoundEventHolder::Named(sound) = adult else {
            panic!("the loaded adult sound is named");
        };
        assert_eq!(sound.as_str(), "minecraft:entity.cow.ambient");
        let SoundEventHolder::Inline(sound) = baby else {
            panic!("the loaded baby sound is inline");
        };
        assert_eq!(sound.sound_id.as_str(), "example:ambient");
        assert_eq!(sound.range.unwrap().to_f32(), 7.5);
    }
    assert_eq!(
        registries.dimension_type_ids().collect::<Vec<_>>(),
        vec!["minecraft:overworld"]
    );
    // The replacement is what travels, and the built-in elements the pack did
    // not declare are gone with it -- which is what a replaced registry is.
    assert_eq!(
        registries.dimension_type_index("minecraft:the_nether"),
        None
    );
    let published = registries.published();
    let NbtSummaryValue::Compound(fields) = bbb_protocol::decode_nbt_summary(
        published
            .iter()
            .find(|registry| registry.registry == "minecraft:dimension_type")
            .expect("the synchronized catalog contains dimension types")
            .entries[0]
            .raw_data
            .as_ref()
            .unwrap(),
    )
    .unwrap()
    .unwrap() else {
        panic!("a dimension type is a compound");
    };
    assert!(fields
        .iter()
        .any(|entry| entry.name == "min_y" && entry.value == NbtSummaryValue::Int(-32)));
}
