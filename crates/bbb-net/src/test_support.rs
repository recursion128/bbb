use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn known_packs_manager(label: &str) -> bbb_pack::KnownPacksManager {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bbb-net-known-packs-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    write_file(
        &root.join("version.json"),
        &format!(
            r#"{{"id":"26.1","protocol_version":{}}}"#,
            bbb_protocol::PROTOCOL_VERSION
        ),
    );
    for registry in bbb_protocol::registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
        let (_, registry_path) = registry.resource_id.split_once(':').unwrap();
        write_file(
            &root
                .join("data/minecraft")
                .join(registry_path)
                .join("base.json"),
            "{}",
        );
    }
    let roots = bbb_pack::PackRoots {
        mc_code_root: root.clone(),
        sources_dir: root.clone(),
        assets_dir: root.join("assets/minecraft"),
        generated_assets_dir: None,
        resource_pack_dirs: Vec::new(),
    };
    let manager = bbb_pack::KnownPacksManager::load(&roots).unwrap();
    fs::remove_dir_all(root).unwrap();
    manager
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}
