use super::*;
use bbb_registry_codec::registry_codec::RegistryReferenceLookup;

#[test]
fn reload_publication_preserves_frozen_dialog_refs_while_new_tag_getter_replaces_old() {
    let root = std::env::temp_dir().join(format!(
        "bbb-reload-dialog-publication-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let pack = root.join("datapacks/scope");
    let dialogs = pack.join("data/test/dialog");
    let tags = pack.join("data/test/tags/dialog");
    std::fs::create_dir_all(&dialogs).unwrap();
    std::fs::create_dir_all(&tags).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"test","pack_format":1000,"supported_formats":[0,10000]},"filter":{"block":[{"namespace":"minecraft","path":"(tags/)?dialog/.*"}]}}"#).unwrap();
    std::fs::write(
        dialogs.join("leaf.json"),
        r#"{"type":"notice","title":"L"}"#,
    )
    .unwrap();
    std::fs::write(
        dialogs.join("root.json"),
        r##"{"type":"dialog_list","title":"R","dialogs":"#test:old"}"##,
    )
    .unwrap();
    std::fs::write(tags.join("old.json"), r#"{"values":["test:leaf"]}"#).unwrap();
    let options = DatapackStartupOptions {
        datapacks_dir: Some(root.join("datapacks")),
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    };
    let original = DatapackStartup::load(&options).unwrap();
    let before = SynchronizedRegistries::load(Some(&original)).unwrap();
    std::fs::remove_file(tags.join("old.json")).unwrap();
    std::fs::write(tags.join("new.json"), r#"{"values":["test:leaf"]}"#).unwrap();
    let reloaded = DatapackStartup::reload(&options, &original).unwrap();
    let after = SynchronizedRegistries::load(Some(&reloaded)).unwrap();
    assert_eq!(before.published(), after.published());
    let lookup = FrozenDatapackRegistryLookup::from_startup(&reloaded);
    assert!(!lookup.contains_tag("minecraft:dialog", "test:old"));
    assert!(lookup.contains_tag("minecraft:dialog", "test:new"));
    let registry = after
        .published_tags()
        .registries
        .iter()
        .find(|registry| registry.registry == "minecraft:dialog")
        .unwrap();
    assert!(registry.tags.iter().any(|tag| tag.tag == "test:new"));
    assert!(!registry.tags.iter().any(|tag| tag.tag == "test:old"));
}
