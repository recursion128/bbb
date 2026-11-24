use super::*;

fn reload_tag_members<'a>(
    startup: &'a mut bbb_server_datapack::startup::DatapackStartup,
    registry_id: &str,
) -> &'a mut BTreeMap<String, Vec<String>> {
    let index = startup
        .tags
        .registries
        .iter()
        .position(|registry| registry.registry_id == registry_id)
        .unwrap_or_else(|| {
            startup
                .tags
                .registries
                .push(bbb_server_datapack::tag_loader::LoadedRegistryTags {
                    registry_id: registry_id.to_string(),
                    tags: BTreeMap::new(),
                });
            startup.tags.registries.len() - 1
        });
    &mut startup.tags.registries[index].tags
}

#[test]
fn persistent_runtime_uses_live_fixed_tags_and_keeps_the_last_valid_generation() {
    const TAG: &str = "example:persistent_items";
    let mut startup = crate::chunk::noise_generator::test_support::fresh_overworld_startup(true);
    reload_tag_members(&mut startup, "minecraft:item")
        .insert(TAG.to_string(), vec!["minecraft:stone".to_string()]);
    let startup = Arc::new(startup);
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::clone(&startup)),
        ..ServerWorldConfig::default()
    })
    .unwrap();
    assert_eq!(
        world.runtime_registry_tag_members_for_test("minecraft:item", TAG),
        bbb_protocol::PersistentRegistryTagMembers::Present(vec!["minecraft:stone".to_string()])
    );

    let mut replacement = (*startup).clone();
    reload_tag_members(&mut replacement, "minecraft:item")
        .insert(TAG.to_string(), vec!["minecraft:netherrack".to_string()]);
    let replacement = Arc::new(replacement);
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: ReloadedDatapacks::Built(Arc::clone(&replacement)),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert_eq!(
        world.runtime_registry_tag_members_for_test("minecraft:item", TAG),
        bbb_protocol::PersistentRegistryTagMembers::Present(vec![
            "minecraft:netherrack".to_string()
        ])
    );

    let mut invalid = (*replacement).clone();
    reload_tag_members(&mut invalid, "minecraft:item")
        .insert(TAG.to_string(), vec!["example:not_an_item".to_string()]);
    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: ReloadedDatapacks::Built(Arc::new(invalid)),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert_eq!(
        world.runtime_registry_tag_members_for_test("minecraft:item", TAG),
        bbb_protocol::PersistentRegistryTagMembers::Present(vec![
            "minecraft:netherrack".to_string()
        ])
    );
}

#[test]
fn noise_reloadable_resources_replace_all_served_levels_and_template_lookup() {
    let mut startup = crate::chunk::noise_generator::test_support::fresh_overworld_startup(true);
    let end_biome = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:the_end")
        .unwrap();
    let mut end_biome_source: serde_json::Value =
        serde_json::from_slice(&end_biome.source_json).unwrap();
    end_biome_source["features"] = serde_json::json!(["#example:reload_feature"]);
    end_biome.source = Some(
        bbb_registry_codec::decode_direct_registry_json(
            &serde_json::to_vec(&(end_biome_source.clone())).unwrap(),
        )
        .unwrap(),
    );
    end_biome.source_json = serde_json::to_vec(&end_biome_source).unwrap().into();
    let features = bbb_server_datapack::worldgen::biome::BiomeFeatures::decode(
        &bbb_registry_codec::parse_registry_json(&end_biome.source_json).unwrap(),
    )
    .unwrap();
    startup
        .registries
        .worldgen
        .biome_features
        .insert("minecraft:the_end".into(), features);
    let end_city = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/structure")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:end_city")
        .unwrap();
    let mut end_city_source: serde_json::Value =
        serde_json::from_slice(&end_city.source_json).unwrap();
    end_city_source["biomes"] = serde_json::json!("#example:reload_end_city_biomes");
    end_city.source = Some(
        bbb_registry_codec::decode_direct_registry_json(
            &serde_json::to_vec(&(end_city_source.clone())).unwrap(),
        )
        .unwrap(),
    );
    end_city.source_json = serde_json::to_vec(&end_city_source).unwrap().into();
    reload_tag_members(&mut startup, "minecraft:worldgen/placed_feature").insert(
        "example:reload_feature".to_string(),
        vec!["minecraft:end_spike".to_string()],
    );
    reload_tag_members(&mut startup, "minecraft:worldgen/biome").insert(
        "example:reload_end_city_biomes".to_string(),
        vec!["minecraft:end_highlands".to_string()],
    );
    let startup = Arc::new(startup);
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::clone(&startup)),
        ..ServerWorldConfig::default()
    })
    .unwrap();
    assert_eq!(
        world.served_builtin_dimensions().as_slice(),
        Java26BuiltinDimension::ALL.as_slice()
    );

    let mut replacement = (*startup).clone();
    reload_tag_members(&mut replacement, "minecraft:worldgen/placed_feature").insert(
        "example:reload_feature".to_string(),
        vec!["minecraft:end_gateway_return".to_string()],
    );
    reload_tag_members(&mut replacement, "minecraft:worldgen/biome")
        .insert("example:reload_end_city_biomes".to_string(), Vec::new());
    reload_tag_members(&mut replacement, "minecraft:block").insert(
        "minecraft:nether_carver_replaceables".to_string(),
        Vec::new(),
    );
    let end_city = replacement
        .structures
        .templates
        .iter_mut()
        .find(|template| template.id == "minecraft:end_city/base_floor")
        .expect("the frozen End City catalog contains base_floor");
    end_city.source_pack = "file/reloaded_end_city".to_string();
    let replacement = Arc::new(replacement);

    let end_store = world.chunk_store_for_test(Java26BuiltinDimension::End);
    assert!(end_store
        .noise_reloadable_feature_contains_for_test("minecraft:the_end", "minecraft:end_spike"));
    assert!(!end_store.noise_reloadable_feature_contains_for_test(
        "minecraft:the_end",
        "minecraft:end_gateway_return"
    ));
    let prepared = world
        .prepare_noise_reloadable_resources(&replacement)
        .unwrap();
    for dimension in [Java26BuiltinDimension::Nether, Java26BuiltinDimension::End] {
        assert!(!world
            .chunk_store_for_test(dimension)
            .noise_reloadable_resources_startup_is_for_test(&replacement));
    }
    world.install_noise_reloadable_resources(prepared);

    for dimension in [Java26BuiltinDimension::Nether, Java26BuiltinDimension::End] {
        assert!(world
            .chunk_store_for_test(dimension)
            .noise_reloadable_resources_startup_is_for_test(&replacement));
    }
    assert_eq!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::End)
            .noise_reloadable_structure_template_source_for_test("minecraft:end_city/base_floor"),
        Some("file/reloaded_end_city")
    );
    let end_store = world.chunk_store_for_test(Java26BuiltinDimension::End);
    assert!(!end_store
        .noise_reloadable_feature_contains_for_test("minecraft:the_end", "minecraft:end_spike"));
    assert!(end_store.noise_reloadable_feature_contains_for_test(
        "minecraft:the_end",
        "minecraft:end_gateway_return"
    ));
}
