use super::*;
use crate::world::{ServerWorld, ServerWorldConfig};
use bbb_protocol::game_types::GameType;
use bbb_server_datapack::dimension_type::builtin_dimension_type_shape;
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::DatapackStartupOptions;

fn builtin_startup() -> DatapackStartup {
    DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap()
}

fn published_column_shape(world: &ServerWorld, dimension: Java26BuiltinDimension) -> (i32, usize) {
    let (min_y, height) = world.dimension_type_height(dimension).unwrap();
    (
        min_y,
        usize::try_from(height / 16).expect("dimension height is a section multiple"),
    )
}

#[test]
fn dimension_spawn_info_and_height_come_from_the_published_holder() {
    let world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let registries = world.synchronized_registries();
    for dimension in world.served_builtin_dimensions() {
        let spawn = world.common_player_spawn_info(dimension, GameType::Creative);
        assert_eq!(
            Some(spawn.dimension_type_id),
            registries.dimension_type_index(dimension.resource_id())
        );
        assert_eq!(spawn.dimension, dimension.resource_id());
        assert_eq!(
            world.level_column_shape(dimension),
            published_column_shape(&world, dimension)
        );
        assert_eq!(
            world.dimension_type_height(dimension).unwrap(),
            published_dimension_type_height(registries, dimension.resource_id()).unwrap()
        );
    }
    assert!(published_dimension_type_height(registries, "minecraft:nowhere").is_err());
}

#[test]
fn dimension_height_follows_a_replaced_published_type() {
    let min_y = -16;
    let height = 64;
    let mut startup = builtin_startup();
    let registry = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == DIMENSION_TYPE_REGISTRY)
        .unwrap();
    let overworld = registry
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:overworld")
        .unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&overworld.source_json).unwrap();
    json["min_y"] = serde_json::json!(min_y);
    overworld.source_json = serde_json::to_vec(&json).unwrap().into();
    overworld.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&overworld.source_json).unwrap());
    let mut json: serde_json::Value = serde_json::from_slice(&overworld.source_json).unwrap();
    json["height"] = serde_json::json!(height);
    overworld.source_json = serde_json::to_vec(&json).unwrap().into();
    overworld.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&overworld.source_json).unwrap());
    let mut json: serde_json::Value = serde_json::from_slice(&overworld.source_json).unwrap();
    json["logical_height"] = serde_json::json!(height);
    overworld.source_json = serde_json::to_vec(&json).unwrap().into();
    overworld.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&overworld.source_json).unwrap());
    overworld.source_json = serde_json::to_vec(&overworld.json()).unwrap().into();
    overworld.codec_value = None;
    let world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let builtin = builtin_dimension_type_shape("minecraft:overworld").unwrap();
    let published = world
        .dimension_type_height(Java26BuiltinDimension::Overworld)
        .unwrap();
    assert_ne!(published, (builtin.min_y, builtin.height));
    assert_eq!(published, (min_y, height));
    assert_eq!(
        world.level_column_shape(Java26BuiltinDimension::Overworld),
        published_column_shape(&world, Java26BuiltinDimension::Overworld)
    );
}

#[test]
fn dimension_spawn_info_index_follows_published_holder_order() {
    let mut startup = builtin_startup();
    let registry = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == DIMENSION_TYPE_REGISTRY)
        .unwrap();
    let overworld = registry
        .elements
        .iter()
        .position(|element| element.id == "minecraft:overworld")
        .unwrap();
    let nether = registry
        .elements
        .iter()
        .position(|element| element.id == "minecraft:the_nether")
        .unwrap();
    registry.elements.swap(overworld, nether);
    let world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let spawn_index = |dimension| {
        world
            .common_player_spawn_info(dimension, GameType::Creative)
            .dimension_type_id
    };
    let registries = world.synchronized_registries();
    assert_eq!(
        Some(spawn_index(Java26BuiltinDimension::Overworld)),
        registries.dimension_type_index("minecraft:overworld")
    );
    assert_eq!(
        Some(spawn_index(Java26BuiltinDimension::Nether)),
        registries.dimension_type_index("minecraft:the_nether")
    );
    assert_ne!(
        spawn_index(Java26BuiltinDimension::Overworld),
        spawn_index(Java26BuiltinDimension::Nether)
    );
}
