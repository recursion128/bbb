use super::*;

/// `EntityTypePredicate.matches` reads the entity-type tag the datapacks
/// loaded, not the built-in membership.
#[test]
fn death_loot_type_terms_use_published_entity_type_tags() {
    let root = std::env::temp_dir().join(format!("bbb-death-loot-type-tags-{}", Uuid::new_v4()));
    let pack = root.join("datapacks/test");
    for directory in [
        "data/test/tags/entity_type",
        "data/minecraft/loot_table/entities",
    ] {
        std::fs::create_dir_all(pack.join(directory)).unwrap();
    }
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"death loot type tags","pack_format":1000,"supported_formats":[0,10000]}}"#,
    )
    .unwrap();
    std::fs::write(
        pack.join("data/test/tags/entity_type/marked.json"),
        r#"{"values":["minecraft:cow"]}"#,
    )
    .unwrap();
    let table = serde_json::json!({"type":"minecraft:entity","pools":[{
        "rolls":1,
        "conditions":[{
            "condition":"minecraft:entity_properties",
            "entity":"this",
            "predicate":{"type":"#test:marked"}
        }],
        "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]
    }]});
    std::fs::write(
        pack.join("data/minecraft/loot_table/entities/cow.json"),
        serde_json::to_vec(&table).unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.join("datapacks")),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..ServerWorldConfig::default()
    });

    let cow = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
    kill_stored(&mut world, cow);
    assert_eq!(dropped_ids(&world), ["minecraft:diamond"]);
}
