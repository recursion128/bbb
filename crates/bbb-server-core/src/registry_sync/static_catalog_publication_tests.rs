use super::*;
use bbb_protocol::{
    packets::{decode_configuration_clientbound, ConfigurationClientbound},
    registry_data_types::VANILLA_BUILT_IN_REGISTRY_TYPES,
};

struct Pack(std::path::PathBuf);
impl Pack {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "bbb-static-catalog-wire-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("catalog")).unwrap();
        std::fs::write(root.join("catalog/pack.mcmeta"), r#"{"pack":{"description":"static catalog","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
        Self(root)
    }
    fn tag(&self, registry: &str, name: &str, members: &[&str]) {
        let path = self.0.join(format!(
            "catalog/data/test/tags/{}/{name}.json",
            registry.strip_prefix("minecraft:").unwrap()
        ));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            path,
            serde_json::to_vec(&serde_json::json!({"replace":true,"values":members})).unwrap(),
        )
        .unwrap();
    }
    fn options(&self) -> DatapackStartupOptions {
        DatapackStartupOptions {
            datapacks_dir: Some(self.0.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        }
    }
}
impl Drop for Pack {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn wire(publication: &SynchronizedRegistries) -> UpdateTags {
    let (id, payload) = encode_configuration_update_tags(publication.published_tags()).unwrap();
    assert_eq!(
        id,
        bbb_protocol::ids::configuration::CLIENTBOUND_UPDATE_TAGS
    );
    let ConfigurationClientbound::UpdateTags(decoded) =
        decode_configuration_clientbound(id, &payload).unwrap()
    else {
        panic!("UpdateTags")
    };
    assert_eq!(&decoded, publication.published_tags());
    decoded
}
fn entries<'a>(tags: &'a UpdateTags, registry: &str, name: &str) -> &'a [i32] {
    &tags
        .registries
        .iter()
        .find(|loaded| loaded.registry == registry)
        .unwrap()
        .tags
        .iter()
        .find(|tag| tag.tag == name)
        .unwrap()
        .entries
}
fn members(tags: &RegistryTagSnapshot, registry: &str, name: &str) -> Vec<String> {
    assert!(tags.tag_exists(registry, name), "{registry} {name}");
    tags.members(registry, name)
        .iter()
        .map(|member| member.to_string())
        .collect()
}

#[test]
fn all_static_disk_tags_round_trip_raw_ids_and_keep_old_reload_publication() {
    let catalogs = bbb_server_datapack::tag_loader::network_safe_static_registry_elements();
    let pack = Pack::new();
    for (registry, members) in &catalogs {
        pack.tag(
            registry,
            "selected",
            &[
                members[members.len() - 1],
                members[0],
                members[members.len() / 2],
                members[members.len() - 1],
            ],
        );
        pack.tag(registry, "empty", &[]);
    }
    let startup = DatapackStartup::load(&pack.options()).unwrap();
    let publication = SynchronizedRegistries::load(Some(&startup)).unwrap();
    let old_tags = publication.registry_tags_shared();
    let old_wire = publication.published_tags_shared();
    let decoded = wire(&publication);
    let retained = [
        "minecraft:block",
        "minecraft:item",
        "minecraft:entity_type",
        "minecraft:fluid",
        "minecraft:game_event",
        "minecraft:point_of_interest_type",
        "minecraft:potion",
        "minecraft:mob_effect",
        "minecraft:sound_event",
        "minecraft:attribute",
    ];
    let expected_order = retained
        .into_iter()
        .chain(
            VANILLA_BUILT_IN_REGISTRY_TYPES
                .iter()
                .map(|entry| entry.resource_id)
                .filter(|registry| !retained.contains(registry)),
        )
        .collect::<Vec<_>>();
    let actual_order = decoded
        .registries
        .iter()
        .map(|registry| registry.registry.as_str())
        .filter(|registry| catalogs.iter().any(|(id, _)| id == registry))
        .collect::<Vec<_>>();
    assert_eq!(actual_order, expected_order);
    for (registry, catalog) in &catalogs {
        let indices = [catalog.len() - 1, 0, catalog.len() / 2].into_iter().fold(
            Vec::new(),
            |mut values, index| {
                if !values.contains(&index) {
                    values.push(index);
                }
                values
            },
        );
        assert_eq!(
            entries(&decoded, registry, "test:selected"),
            indices
                .iter()
                .map(|index| *index as i32)
                .collect::<Vec<_>>(),
            "{registry}"
        );
        assert_eq!(
            members(&old_tags, registry, "test:selected"),
            indices
                .iter()
                .map(|index| catalog[*index].to_string())
                .collect::<Vec<_>>(),
            "{registry}"
        );
        assert!(entries(&decoded, registry, "test:empty").is_empty());
        assert!(members(&old_tags, registry, "test:empty").is_empty());
        pack.tag(registry, "selected", &[catalog[0], catalog[0]]);
    }
    let reload = DatapackStartup::reload(&pack.options(), &startup).unwrap();
    let replacement = SynchronizedRegistries::load(Some(&reload)).unwrap();
    let new_wire = wire(&replacement);
    for (registry, catalog) in &catalogs {
        assert_eq!(
            entries(&new_wire, registry, "test:selected"),
            [0],
            "{registry}"
        );
        assert_eq!(
            members(replacement.registry_tags(), registry, "test:selected"),
            [catalog[0].to_string()]
        );
        assert_eq!(
            entries(&decoded, registry, "test:selected"),
            entries(&old_wire, registry, "test:selected")
        );
        let old_indices = entries(&decoded, registry, "test:selected");
        assert_eq!(
            members(&old_tags, registry, "test:selected"),
            old_indices
                .iter()
                .map(|index| catalog[*index as usize].to_string())
                .collect::<Vec<_>>()
        );
    }
}
