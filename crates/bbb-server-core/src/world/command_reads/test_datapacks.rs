use super::test_fixture::CommandTestWorld;
use super::*;
use crate::admin::NameAndId;
use crate::world::ServerWorldConfig;
use bbb_protocol::game_types::GameType;

impl CommandTestWorld {
    pub(crate) fn with_builtin_data(players: &[NameAndId]) -> Self {
        Self::new(
            ServerWorldConfig {
                game_type: GameType::Survival,
                datapacks: Some(crate::registry_sync::builtin_datapack_startup_shared()),
                ..Default::default()
            },
            players,
        )
    }

    pub(crate) fn with_dialog_data(players: &[NameAndId], dialogs: &[String]) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "bbb-command-dialog-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        ));
        let pack = path.join("test");
        std::fs::create_dir_all(&pack).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"test","pack_format":1000,"supported_formats":[0,10000]},"filter":{"block":[{"namespace":"minecraft","path":"(tags/)?dialog/.*"}]}}"#).unwrap();
        for id in dialogs {
            let (namespace, name) = id.split_once(':').unwrap();
            let file = pack.join(format!("data/{namespace}/dialog/{name}.json"));
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, r#"{"type":"notice","title":"Test"}"#).unwrap();
        }
        let startup = bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: Some(path.clone()),
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            },
        )
        .unwrap();
        std::fs::remove_dir_all(path).unwrap();
        Self::new(
            ServerWorldConfig {
                game_type: GameType::Survival,
                datapacks: Some(std::sync::Arc::new(startup)),
                ..Default::default()
            },
            players,
        )
    }
}

impl CommandTestWorld {
    pub(crate) fn with_flat_data(players: &[NameAndId]) -> Self {
        let config = ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(crate::registry_sync::builtin_datapack_startup_shared()),
            ..Default::default()
        };
        let sources = super::super::default_level_sources()
            .unwrap()
            .into_iter()
            .map(|(level, source)| (level, super::super::ServerLevelSource::Flat(source)))
            .collect();
        let types = super::super::default_level_dimension_types().unwrap();
        let mut world = ServerWorld::with_level_sources(
            config,
            &sources,
            &types,
            super::super::CREATED_WORLD_SEED,
            false,
        )
        .unwrap();
        world.finish_debug_dimension_initialization(false).unwrap();
        world.refresh_all_sky_darkening();
        Self::from_world(world, players)
    }
}
