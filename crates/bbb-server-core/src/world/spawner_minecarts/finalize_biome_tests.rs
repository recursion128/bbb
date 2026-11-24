use super::*;
use crate::entity::natural_spawn::create_natural_spawn_root;
use crate::entity::{read_entity_chunk, write_entity_chunk, EntityIdCounter};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_FROG_ID,
    VANILLA_ENTITY_TYPE_PIG_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};
use std::sync::OnceLock;

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}

fn world() -> ServerWorld {
    static CONFIG: OnceLock<crate::world::ServerWorldConfig> = OnceLock::new();
    ServerWorld::new(CONFIG.get_or_init(crate::tests::datapack_config).clone()).unwrap()
}

fn set_biome(world: &mut ServerWorld, quart: (i32, i32, i32), biome: &str) {
    let id = world
        .registries
        .registry_element_index(BIOME_REGISTRY, biome)
        .unwrap();
    let chunks = &mut world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap()
        .chunks;
    if chunks.column(quart.0 >> 2, quart.2 >> 2).is_none() {
        let column = chunks.generated_column();
        chunks.insert_column(quart.0 >> 2, quart.2 >> 2, column);
    }
    chunks
        .column_mut(quart.0 >> 2, quart.2 >> 2)
        .unwrap()
        .set_noise_biome(quart.0 & 3, quart.1, quart.2 & 3, id)
        .unwrap();
}

fn runtime(world: &mut ServerWorld) -> MinecartSpawnerRuntime<'_> {
    let context = world.cold.dimension_sky_lights.spawn_context(
        Some("minecraft:overworld"),
        &world.registries,
        &world.world_clocks,
    );
    let level_names = Arc::clone(&world.levels.names);
    let dimension = world.require_level_id(&key());
    let (level, others) = world.levels.split_mut(&dimension).unwrap();
    MinecartSpawnerRuntime::new(
        &mut level.chunks,
        &mut world.maps,
        world.cold.generate_structures,
        &mut level.entities,
        &[],
        Difficulty::Normal,
        Java26BuiltinDimension::Overworld,
        &world.registries,
        world.cold.config.datapacks.as_deref(),
        &world.datapack_tables.enchantment_registry,
        &world.cold.loot_item_tags,
        None,
        world.cold.seed,
        world.game_time,
        0,
        false,
        None,
        context,
        &mut world.scoreboard,
        level_names,
        LootEntityLevels::Other(others),
        &world.world_clocks,
        world.cold.dimension_sky_lights.loot_environment_context(
            Some("minecraft:overworld"),
            &world.registries,
            &world.world_clocks,
            ScalarWeather::from_level(&level.weather),
        ),
        level.sky_darkening.map(i32::from),
        LootWeather {
            raining: level.weather.is_raining(),
            thundering: level.weather.is_thundering(),
        },
        &world.cold.command_storage,
    )
}

#[test]
fn generic_spawner_fuzzy_variants_publish_and_survive_entity_save() {
    let mut world = world();
    let pos = (4..12)
        .flat_map(|x| (4..12).map(move |z| BlockPos { x, y: 70, z }))
        .find(|p| {
            fuzzy_biome_quart_position(biome_zoom_seed(world.cold.seed), p.x, p.y, p.z)
                != (p.x >> 2, p.y >> 2, p.z >> 2)
        })
        .unwrap();
    let fuzzy = fuzzy_biome_quart_position(biome_zoom_seed(world.cold.seed), pos.x, pos.y, pos.z);
    set_biome(&mut world, fuzzy, "minecraft:desert");
    set_biome(
        &mut world,
        (pos.x >> 2, pos.y >> 2, pos.z >> 2),
        "minecraft:plains",
    );
    let mut runtime = runtime(&mut world);
    let raw = runtime.chunks.biome_id_at(pos.x, pos.y, pos.z).unwrap();
    assert_eq!(
        runtime.registries.registry_element_id(BIOME_REGISTRY, raw),
        Some("minecraft:plains")
    );
    let prepared = runtime.finalize_context(pos, "minecraft:cow").unwrap();
    assert_eq!(prepared.biome.as_deref(), Some("minecraft:desert"));
    assert!(prepared
        .biome_tags
        .contains(&"minecraft:spawns_warm_variant_farm_animals"));
    for entity_type in [
        VANILLA_ENTITY_TYPE_COW_ID,
        VANILLA_ENTITY_TYPE_PIG_ID,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        VANILLA_ENTITY_TYPE_FROG_ID,
    ] {
        let resource = vanilla_entity_type(entity_type).unwrap().resource_id;
        let mut tree = create_natural_spawn_root(
            runtime.entities,
            entity_type,
            Vec3d {
                x: f64::from(pos.x),
                y: f64::from(pos.y),
                z: f64::from(pos.z),
            },
        )
        .unwrap();
        assert!(
            finalize_spawn(
                &mut runtime,
                &mut tree,
                SpawnReason::Spawner,
                None,
                &GameRandom::with_seed(17)
            )
            .is_some(),
            "{resource}"
        );
        runtime.entities.add_fresh(tree);
    }
    let saved = write_entity_chunk(runtime.entities, (0, 0)).unwrap();
    assert_eq!(saved.entities().len(), 4);
    for record in saved.entities() {
        assert!(
            record.raw().iter().any(|entry| entry.name == "variant"
                && entry.value == NbtSummaryValue::String("minecraft:warm".into())),
            "{}: {:?}",
            record.id(),
            record.raw()
        );
    }
    let mut restored = EntityStore::new(91, EntityIdCounter::new());
    read_entity_chunk(&mut restored, &saved).unwrap();
    let resaved = write_entity_chunk(&restored, (0, 0)).unwrap();
    let variants = |data: &bbb_server_persistence::Java26EntityChunkData| {
        data.entities()
            .map(|record| {
                (
                    record.uuid(),
                    record
                        .raw()
                        .iter()
                        .find(|entry| entry.name == "variant")
                        .unwrap()
                        .value
                        .clone(),
                )
            })
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(variants(&resaved), variants(&saved));
}

#[test]
fn generic_spawner_missing_fuzzy_column_uses_source_without_loading() {
    let mut world = world();
    let pos = BlockPos { x: 0, y: 70, z: 0 };
    world.cold.seed = (0..1024)
        .find(|&seed| {
            let q = fuzzy_biome_quart_position(biome_zoom_seed(seed), pos.x, pos.y, pos.z);
            q.0 < 0 || q.2 < 0
        })
        .unwrap();
    set_biome(&mut world, (0, pos.y >> 2, 0), "minecraft:desert");
    let mut runtime = runtime(&mut world);
    let count = runtime.chunks.column_count();
    let fuzzy = fuzzy_biome_quart_position(runtime.biome_zoom_seed, pos.x, pos.y, pos.z);
    assert!(runtime.chunks.column(fuzzy.0 >> 2, fuzzy.2 >> 2).is_none());
    let context = runtime.finalize_context(pos, "minecraft:cow").unwrap();
    assert_eq!(context.biome.as_deref(), Some("minecraft:plains"));
    assert!(!context
        .biome_tags
        .contains(&"minecraft:spawns_warm_variant_farm_animals"));
    let mut tree = create_natural_spawn_root(
        runtime.entities,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 0.0,
            y: 70.0,
            z: 0.0,
        },
    )
    .unwrap();
    let random = GameRandom::with_seed(7);
    assert!(finalize_spawn(&mut runtime, &mut tree, SpawnReason::Spawner, None, &random).is_some());
    runtime.entities.add_fresh(tree);
    let saved = write_entity_chunk(runtime.entities, (0, 0)).unwrap();
    assert_eq!(saved.entities().len(), 1);
    assert!(saved
        .entities()
        .next()
        .unwrap()
        .raw()
        .iter()
        .any(|entry| entry.name == "variant"
            && entry.value == NbtSummaryValue::String("minecraft:temperate".into())));
    assert_eq!(runtime.chunks.column_count(), count);
}

#[test]
fn farm_animal_finalization_uses_published_sound_registry_and_saves_selection() {
    use bbb_protocol::entity_synched_data::{
        VANILLA_CHICKEN_SOUND_VARIANT_DATA_ID, VANILLA_COW_SOUND_VARIANT_DATA_ID,
        VANILLA_PIG_SOUND_VARIANT_DATA_ID,
    };
    use bbb_protocol::packets::{EntityDataRegistryReference, EntityDataValueKind};
    use std::sync::Arc;

    let cases = [
        (
            "cow",
            VANILLA_ENTITY_TYPE_COW_ID,
            VANILLA_COW_SOUND_VARIANT_DATA_ID,
        ),
        (
            "chicken",
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            VANILLA_CHICKEN_SOUND_VARIANT_DATA_ID,
        ),
        (
            "pig",
            VANILLA_ENTITY_TYPE_PIG_ID,
            VANILLA_PIG_SOUND_VARIANT_DATA_ID,
        ),
    ];
    let mut config = crate::tests::datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (animal, _, _) in cases {
        let name = format!("minecraft:{animal}_sound_variant");
        let registry = startup
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == name)
            .unwrap();
        let mut sound = registry
            .elements
            .iter()
            .find(|element| element.id == "minecraft:classic")
            .unwrap()
            .clone();
        sound.id = format!("bbb:{animal}_voice");
        registry.elements = vec![sound];
    }
    let mut world = ServerWorld::new(config).unwrap();
    let pos = BlockPos { x: 0, y: 70, z: 0 };
    set_biome(&mut world, (0, pos.y >> 2, 0), "minecraft:plains");
    let mut runtime = runtime(&mut world);
    for (animal, entity_type, data_id) in cases {
        let resource = format!("minecraft:{animal}");
        let registry = format!("minecraft:{animal}_sound_variant");
        let sound = format!("bbb:{animal}_voice");
        let holder = runtime
            .registries
            .registry_element_index(&registry, &sound)
            .unwrap();
        let mut tree = create_natural_spawn_root(
            runtime.entities,
            entity_type,
            Vec3d {
                x: 0.5,
                y: 70.0,
                z: 0.5,
            },
        )
        .unwrap();
        assert!(
            finalize_spawn(
                &mut runtime,
                &mut tree,
                SpawnReason::Spawner,
                None,
                &GameRandom::with_seed(17)
            )
            .is_some(),
            "{resource}"
        );
        let id = runtime.entities.add_fresh(tree);
        let entity = runtime.entities.get(id).unwrap();
        let metadata = crate::entity::tracking::tracked_data_values(entity);
        assert!(
            metadata.iter().any(|value| value.data_id == data_id
                && matches!(
                    &value.value, EntityDataValueKind::RegistryId { value, .. }
                        if value == &EntityDataRegistryReference::unresolved(holder)
                )),
            "{resource}: {metadata:?}"
        );
        let NbtSummaryValue::Compound(fields) = runtime.entities.command_data(id).unwrap() else {
            panic!("compound");
        };
        assert!(fields.iter().any(|entry| entry.name == "sound_variant"
            && entry.value == NbtSummaryValue::string(sound.clone())));
        assert!(fields
            .iter()
            .any(|entry| entry.name == "Age" && entry.value == NbtSummaryValue::Int(0)));
    }
    let saved = write_entity_chunk(runtime.entities, (0, 0)).unwrap();
    for record in saved.entities() {
        let animal = record.id().strip_prefix("minecraft:").unwrap();
        assert!(record
            .raw()
            .iter()
            .any(|entry| entry.name == "sound_variant"
                && entry.value == NbtSummaryValue::string(format!("bbb:{animal}_voice"))));
        assert!(record.raw().iter().any(|entry| entry.name == "variant"
            && entry.value == NbtSummaryValue::String("minecraft:temperate".into())));
    }
    let mut restored = EntityStore::new(91, EntityIdCounter::new());
    read_entity_chunk(&mut restored, &saved).unwrap();
    let resaved = write_entity_chunk(&restored, (0, 0)).unwrap();
    for record in resaved.entities() {
        let animal = record.id().strip_prefix("minecraft:").unwrap();
        assert!(record
            .raw()
            .iter()
            .any(|entry| entry.name == "sound_variant"
                && entry.value == NbtSummaryValue::string(format!("bbb:{animal}_voice"))));
    }
}
