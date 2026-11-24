use super::*;
use crate::wandering_trader_schema::{wandering_trader_dat, Java26WanderingTraderData};

#[test]
fn wandering_trader_world_save_some_and_none_preserve_lazy_creation() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-wandering-trader-lazy-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert_eq!(baseline.wandering_trader, None);
    let mut request = save_patch_request(Vec::new());
    storage.save_snapshot(&baseline, &request).unwrap();
    assert!(!wandering_trader_dat(&paths).exists());
    let baseline = storage.load_candidate().unwrap();
    request.wandering_trader = Some(Java26WanderingTraderData {
        spawn_delay: -123,
        spawn_chance: 99,
    });
    storage.save_snapshot(&baseline, &request).unwrap();
    let bytes = fs::read(wandering_trader_dat(&paths)).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert_eq!(baseline.wandering_trader, request.wandering_trader);
    request.wandering_trader = None;
    storage.save_snapshot(&baseline, &request).unwrap();
    assert_eq!(fs::read(wandering_trader_dat(&paths)).unwrap(), bytes);
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let bootstrap = storage.load_bootstrap().unwrap();
    let geometries =
        Java26DimensionGeometries::from_builtin_settings(bootstrap.world_gen_settings());
    assert_eq!(
        storage
            .load_candidate_with_geometries(bootstrap, geometries, &test_world_clock_ids())
            .unwrap()
            .wandering_trader,
        baseline.wandering_trader
    );
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
