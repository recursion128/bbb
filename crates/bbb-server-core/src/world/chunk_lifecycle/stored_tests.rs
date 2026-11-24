use crate::entity::spawn_armor_stand;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::world::{LevelId, ServerWorld, ServerWorldConfig};
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::packets::{BlockPos, ItemStackSummary, Vec3d};
use bbb_protocol::NbtSummaryValue;
use bbb_server_persistence::{
    Java26WorldCandidate, Java26WorldStorage, RegionCompression, RegionFileStore,
};
use bbb_server_types::save::{
    Java26BuiltinDimension, Java26LevelKey, WorldSaveLimits, WorldSavePaths,
};
use std::path::PathBuf;

const VISITED: (i32, i32) = (0, 0);
/// Far enough that no ticket around [`VISITED`] reaches it, in its own region.
const UNVISITED: (i32, i32) = (40, 0);
const MARKER: BlockPos = BlockPos { x: 3, y: -60, z: 3 };

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id).unwrap()
}

fn overworld() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}

struct Save {
    root: PathBuf,
    storage: Java26WorldStorage,
}

impl Save {
    /// A flat save holding [`VISITED`] with a gold marker and [`UNVISITED`].
    fn create(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "bbb-stored-{name}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let storage = Java26WorldStorage::open(
            WorldSavePaths::new(&root).unwrap(),
            WorldSaveLimits::default(),
        )
        .unwrap();
        let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
        let source = crate::world_creation::default_overworld_level_source().unwrap();
        let mut visited = source
            .generate_chunk(VISITED.0, VISITED.1, fixture.biome_id())
            .into_column();
        visited.set_block_state(MARKER.x, MARKER.y, MARKER.z, state("minecraft:gold_block"));
        let unvisited = source
            .generate_chunk(UNVISITED.0, UNVISITED.1, fixture.biome_id())
            .into_column();
        let created = crate::world_creation::created_save(
            &bbb_server_persistence::Java26WorldCreation {
                level_name: name.into(),
                game_type: 0,
                difficulty: "normal".into(),
                spawn_position: [0, -60, 0],
                last_played: 1,
            },
            &fixture,
            &[
                (VISITED.0, VISITED.1, &visited),
                (UNVISITED.0, UNVISITED.1, &unvisited),
            ],
        )
        .unwrap();
        storage
            .create_world(&created, RegionCompression::Zlib, 1)
            .unwrap();
        Self { root, storage }
    }

    fn reopen(self) -> Self {
        self.storage.close().unwrap();
        let storage = Java26WorldStorage::open(
            WorldSavePaths::new(&self.root).unwrap(),
            WorldSaveLimits::default(),
        )
        .unwrap();
        Self {
            root: self.root,
            storage,
        }
    }

    fn region(&self, chunk: (i32, i32)) -> PathBuf {
        self.storage
            .paths()
            .dimension_dir(&overworld())
            .join("region")
            .join(format!("r.{}.{}.mca", chunk.0 >> 5, chunk.1 >> 5))
    }

    fn region_bytes(&self, chunk: (i32, i32)) -> Vec<u8> {
        std::fs::read(self.region(chunk)).unwrap()
    }

    /// Overwrites the compressed body of `chunk`, leaving its header entry.
    fn corrupt(&self, chunk: (i32, i32)) {
        let mut bytes = self.region_bytes(chunk);
        let index = (chunk.0.rem_euclid(32) + chunk.1.rem_euclid(32) * 32) as usize * 4;
        let sector = u32::from_be_bytes([0, bytes[index], bytes[index + 1], bytes[index + 2]]);
        let start = sector as usize * 4096 + 5;
        bytes[start..start + 64].fill(0xA5);
        std::fs::write(self.region(chunk), bytes).unwrap();
    }

    fn rewrite_data_version(&self, chunk: (i32, i32), version: i32) {
        let directory = self.region(chunk).parent().unwrap().to_path_buf();
        let store = RegionFileStore::new(self.region(chunk), directory, WorldSaveLimits::default())
            .unwrap();
        let mut document = store.read_chunk(chunk.0, chunk.1).unwrap().unwrap();
        document
            .entries_mut()
            .iter_mut()
            .find(|entry| entry.name == "DataVersion")
            .unwrap()
            .value = NbtSummaryValue::Int(version);
        store
            .write_chunk(chunk.0, chunk.1, &document, RegionCompression::Zlib, 1)
            .unwrap();
    }

    fn save(&self, world: &mut ServerWorld, candidate: &Java26WorldCandidate) {
        let request = world.save_request(2, 2, RegionCompression::Zlib).unwrap();
        self.storage.save_snapshot(candidate, &request).unwrap();
        world.acknowledge_save_success(&request);
    }

    fn remove(self) {
        self.storage.close().unwrap();
        let _ = std::fs::remove_dir_all(self.root);
    }
}

fn load(world: &mut ServerWorld, chunk: (i32, i32)) {
    let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
    world
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .add_unknown(chunk, 33);
    world.refresh_loading_chunks(&dimension).unwrap();
}

fn loaded(world: &ServerWorld, chunk: (i32, i32)) -> bool {
    world
        .level(Java26BuiltinDimension::Overworld)
        .chunks
        .is_loaded(chunk.0, chunk.1)
}

#[test]
fn a_saved_world_reads_a_column_only_when_a_ticket_reaches_it() {
    let save = Save::create("on-demand");
    // A broken body in a chunk nobody visits costs nothing at startup.
    save.corrupt(UNVISITED);
    let unvisited_bytes = save.region_bytes(UNVISITED);
    let candidate = save.storage.load_candidate().unwrap();
    assert_eq!(
        candidate.dimensions[&overworld()]
            .stored_chunks
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![VISITED, UNVISITED]
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert!(!loaded(&world, VISITED) && !loaded(&world, UNVISITED));

    load(&mut world, VISITED);
    assert!(loaded(&world, VISITED));
    assert!(!loaded(&world, UNVISITED));
    assert_eq!(
        world
            .level(Java26BuiltinDimension::Overworld)
            .chunks
            .get_block_state(MARKER.x, MARKER.y, MARKER.z),
        Some(state("minecraft:gold_block"))
    );
    for _ in 0..3 {
        world.tick(Vec::new());
    }
    save.save(&mut world, &candidate);
    assert_eq!(save.region_bytes(UNVISITED), unvisited_bytes);

    // The unreadable chunk, once needed, is generated afresh; its first
    // change writes it whole over the broken record.
    load(&mut world, UNVISITED);
    assert!(loaded(&world, UNVISITED));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: UNVISITED.0 * 16,
            y: 100,
            z: 0,
        },
        state("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    save.save(&mut world, &candidate);
    let save = save.reopen();
    let reopened = save.storage.load_candidate().unwrap();
    let column = reopened
        .column_reader
        .as_ref()
        .unwrap()
        .read_column(&overworld(), UNVISITED.0, UNVISITED.1)
        .unwrap();
    assert!(column.chunk.is_some());
    save.remove();
}

#[test]
fn a_visited_column_brings_back_its_blocks_entities_and_poi() {
    let save = Save::create("round-trip");
    let candidate = save.storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
    load(&mut world, VISITED);
    let bell = BlockPos { x: 2, y: -60, z: 2 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        bell,
        state("minecraft:bell"),
        SET_BLOCK_UPDATE_ALL,
    );
    let stand = spawn_armor_stand(
        &mut world.level_mut_by_id(&dimension).entities,
        Vec3d {
            x: 8.5,
            y: -60.0,
            z: 8.5,
        },
    )
    .unwrap();
    let uuid = world
        .level_by_id(&dimension)
        .entities
        .get(stand)
        .unwrap()
        .uuid();
    save.save(&mut world, &candidate);
    drop(world);

    let save = save.reopen();
    let candidate = save.storage.load_candidate().unwrap();
    assert!(candidate.dimensions[&overworld()]
        .stored_entity_chunks
        .contains(&VISITED));
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert!(world
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, uuid)
        .is_none());
    load(&mut world, VISITED);
    world.refresh_loading_chunks(&dimension).unwrap();
    let level = world.level_mut_by_id(&dimension);
    assert_eq!(
        level.chunks.get_block_state(bell.x, bell.y, bell.z),
        Some(state("minecraft:bell"))
    );
    assert!(level
        .chunks
        .poi_mut()
        .record([bell.x, bell.y, bell.z])
        .is_some());
    assert!(world
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, uuid)
        .is_some());
    save.remove();
}

#[test]
fn a_record_of_another_data_version_stops_the_world_and_stays_on_disk() {
    let save = Save::create("old-version");
    save.rewrite_data_version(UNVISITED, 3953);
    let save = save.reopen();
    let before = save.region_bytes(UNVISITED);
    let candidate = save.storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert!(world.storage_refusal().is_none());
    load(&mut world, UNVISITED);
    let refusal = world.storage_refusal().unwrap().to_string();
    assert!(refusal.contains("[40, 0]"), "{refusal}");
    assert!(refusal.contains("3953"), "{refusal}");
    assert!(refusal.contains("--forceUpgrade"), "{refusal}");
    assert!(!loaded(&world, UNVISITED));
    for _ in 0..3 {
        world.tick(Vec::new());
    }
    assert!(!loaded(&world, UNVISITED));
    save.save(&mut world, &candidate);
    assert_eq!(save.region_bytes(UNVISITED), before);
    save.remove();
}

/// `SerializableChunkData.java:357-363`: a saved column writes every block
/// entity it holds as it is now, not only the ones marked changed.
#[test]
fn a_column_saved_for_another_change_writes_its_block_entities_current_state() {
    let save = Save::create("block-entities");
    let candidate = save.storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
    let crafter = BlockPos { x: 5, y: -60, z: 5 };
    load(&mut world, VISITED);
    world.set_block(
        Java26BuiltinDimension::Overworld,
        crafter,
        state("minecraft:crafter"),
        SET_BLOCK_UPDATE_ALL,
    );
    save.save(&mut world, &candidate);
    drop(world);

    let save = save.reopen();
    let candidate = save.storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    load(&mut world, VISITED);
    world
        .level_mut_by_id(&dimension)
        .chunks
        .block_entities_mut()
        .get_mut(crafter)
        .unwrap()
        .crafter_mut()
        .unwrap()
        .items_mut()[0] =
        bbb_server_types::item_stack::RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_OAK_LOG_ID),
            count: 3,
            component_patch: Default::default(),
        });
    world.set_block(
        Java26BuiltinDimension::Overworld,
        MARKER,
        state("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    );
    save.save(&mut world, &candidate);
    let save = save.reopen();
    let reopened = save.storage.load_candidate().unwrap();
    let chunk = reopened
        .column_reader
        .as_ref()
        .unwrap()
        .read_column(&overworld(), VISITED.0, VISITED.1)
        .unwrap()
        .chunk
        .unwrap();
    let record = chunk
        .data
        .block_entities()
        .iter()
        .find(|record| {
            record.iter().any(|entry| {
                entry.name == "id"
                    && entry.value == NbtSummaryValue::String("minecraft:crafter".into())
            })
        })
        .unwrap();
    let items = record.iter().find(|entry| entry.name == "Items").unwrap();
    assert!(matches!(&items.value, NbtSummaryValue::List(items) if items.len() == 1));
    save.remove();
}

/// `VariantUtils.java:30-32`: a variant id the registry lacks leaves the
/// entity at its default, and the rest of the chunk loads.
#[test]
fn an_unknown_variant_loads_as_the_default_and_the_chunk_still_loads() {
    use bbb_protocol::NbtSummaryEntry;
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
    world
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .entity_loads
        .insert(VISITED, super::EntityChunkLoadStatus::Pending);
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let record = |id: &str, uuid: uuid::Uuid, mut extra: Vec<NbtSummaryEntry>| {
        let mut fields = vec![
            entry("id", NbtSummaryValue::String(id.into())),
            entry("UUID", crate::entity::save::uuid_int_array(uuid)),
            entry(
                "Pos",
                NbtSummaryValue::List(
                    [4.5_f64, -60.0, 4.5]
                        .map(|value| NbtSummaryValue::Double(value.to_bits()))
                        .to_vec(),
                ),
            ),
        ];
        fields.append(&mut extra);
        fields
    };
    let wolf = uuid::Uuid::from_u128(0x77);
    let stand = uuid::Uuid::from_u128(0x78);
    let document = super::full_preparation::worldgen_entity_document(
        VISITED,
        vec![
            record(
                "minecraft:wolf",
                wolf,
                vec![entry(
                    "variant",
                    NbtSummaryValue::String("minecraft:no_such_wolf".into()),
                )],
            ),
            record("minecraft:armor_stand", stand, Vec::new()),
        ],
    )
    .unwrap();
    world
        .complete_runtime_entity_chunk(
            &overworld(),
            VISITED,
            Some(bbb_server_persistence::Java26LoadedEntityChunk {
                timestamp: 0,
                data: document,
            }),
        )
        .unwrap();
    let level = world.level(Java26BuiltinDimension::Overworld);
    assert!(level.entities.entity_id_for_uuid(stand).is_some());
    let wolf = level
        .entities
        .get(level.entities.entity_id_for_uuid(wolf).unwrap())
        .unwrap();
    let crate::entity::EntityData::GeneralLiving(data) = wolf.data() else {
        panic!("a wolf is a general living entity");
    };
    assert_eq!(
        data.registry_variant("minecraft:wolf_variant"),
        Some("minecraft:pale")
    );
}

fn reader(
    candidate: &Java26WorldCandidate,
) -> &bbb_server_persistence::world_storage::Java26ColumnReader {
    candidate.column_reader.as_ref().unwrap()
}

/// `SectionStorage.getOrLoad` (`SectionStorage.java:124-187`): a POI query
/// reads a column's stored POI whether or not its chunk is loaded, and what
/// it changes there outlives the chunk's own load.
#[test]
fn poi_of_an_unread_column_answers_queries_and_keeps_live_changes() {
    use crate::chunk::poi::PoiOccupancy;
    let save = Save::create("poi-on-demand");
    let candidate = save.storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
    load(&mut world, UNVISITED);
    let bell = BlockPos {
        x: UNVISITED.0 * 16 + 2,
        y: -60,
        z: 2,
    };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        bell,
        state("minecraft:bell"),
        SET_BLOCK_UPDATE_ALL,
    ));
    save.save(&mut world, &candidate);
    drop(world);

    let save = save.reopen();
    let candidate = save.storage.load_candidate().unwrap();
    assert!(candidate.dimensions[&overworld()]
        .stored_poi_chunks
        .contains(&UNVISITED));
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert!(!loaded(&world, UNVISITED));
    let pos = [bell.x, bell.y, bell.z];
    let mut seen = Vec::new();
    world
        .level(Java26BuiltinDimension::Overworld)
        .chunks
        .poi()
        .visit_records_in_range(pos, 2, PoiOccupancy::Any, |record| seen.push(record.pos));
    assert_eq!(seen, vec![pos]);
    let poi = world.level_mut_by_id(&dimension).chunks.poi_mut();
    assert!(poi.acquire_ticket(pos));
    let tickets = poi.record(pos).unwrap().free_tickets;
    load(&mut world, UNVISITED);
    assert!(loaded(&world, UNVISITED));
    assert_eq!(
        world
            .level_mut_by_id(&dimension)
            .chunks
            .poi_mut()
            .record(pos)
            .unwrap()
            .free_tickets,
        tickets
    );
    save.save(&mut world, &candidate);
    let save = save.reopen();
    let reopened = save.storage.load_candidate().unwrap();
    let stored = reader(&reopened)
        .read_poi(&overworld(), UNVISITED.0, UNVISITED.1)
        .unwrap()
        .unwrap();
    let record = stored.data.records().next().unwrap();
    assert_eq!((record.pos(), record.free_tickets()), (pos, tickets));
    save.remove();
}

/// An entity that walks into a chunk whose entity record is still being read
/// is saved there once the read lands, beside the entities the record held.
#[test]
fn an_entity_that_enters_a_chunk_being_read_is_saved_with_it() {
    use bbb_protocol::packets::movement::PlayerPositionState;
    let save = Save::create("entity-into-pending");
    let candidate = save.storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
    let stand = |world: &mut ServerWorld, x: f64| {
        let entities = &mut world.level_mut_by_id(&dimension).entities;
        let id = spawn_armor_stand(
            entities,
            Vec3d {
                x,
                y: -60.0,
                z: 8.5,
            },
        )
        .unwrap();
        entities.get(id).unwrap().uuid()
    };
    load(&mut world, VISITED);
    let walker = stand(&mut world, 8.5);
    load(&mut world, UNVISITED);
    let resident = stand(&mut world, f64::from(UNVISITED.0 * 16) + 8.5);
    save.save(&mut world, &candidate);
    drop(world);

    let save = save.reopen();
    let candidate = save.storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    load(&mut world, VISITED);
    let level = world.level_mut_by_id(&dimension);
    level.chunk_lifecycle.request_entities(UNVISITED);
    assert!(level.chunk_lifecycle.entities_pending(UNVISITED));
    let id = level.entities.entity_id_for_uuid(walker).unwrap();
    assert!(level.entities.teleport_stored_entity(
        id,
        PlayerPositionState {
            position: Vec3d {
                x: f64::from(UNVISITED.0 * 16) + 4.5,
                y: -60.0,
                z: 4.5,
            },
            delta_movement: Vec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
        },
        0,
    ));
    // The read lands before the save freezes, the way the runtime settles
    // its reads before every save.
    let record = reader(&candidate)
        .read_entity_chunk(&overworld(), UNVISITED.0, UNVISITED.1)
        .unwrap();
    world
        .complete_runtime_entity_chunk(&overworld(), UNVISITED, record)
        .unwrap();
    save.save(&mut world, &candidate);

    let save = save.reopen();
    let reopened = save.storage.load_candidate().unwrap();
    let uuids = |chunk: (i32, i32)| {
        reader(&reopened)
            .read_entity_chunk(&overworld(), chunk.0, chunk.1)
            .unwrap()
            .map(|record| {
                record
                    .data
                    .entities()
                    .map(|entity| entity.uuid())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let mut there = uuids(UNVISITED);
    there.sort();
    let mut expected = vec![walker, resident];
    expected.sort();
    assert_eq!(there, expected);
    assert!(!uuids(VISITED).contains(&walker));
    save.remove();
}

/// A bell saved in [`UNVISITED`], with the save reopened so the column is
/// unread again.
fn save_with_bell(name: &str) -> (Save, [i32; 3]) {
    let save = Save::create(name);
    let candidate = save.storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    load(&mut world, UNVISITED);
    let bell = BlockPos {
        x: UNVISITED.0 * 16 + 2,
        y: -60,
        z: 2,
    };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        bell,
        state("minecraft:bell"),
        SET_BLOCK_UPDATE_ALL,
    ));
    save.save(&mut world, &candidate);
    drop(world);
    (save.reopen(), [bell.x, bell.y, bell.z])
}

/// `PoiManager.java:87-110` over `SectionStorage.getOrLoad`: a range query,
/// as a villager's `AcquirePoi` runs, reads the unread columns it covers.
#[test]
fn a_range_query_reads_the_poi_of_unread_columns() {
    use crate::chunk::poi::PoiOccupancy;
    let (save, bell) = save_with_bell("poi-range");
    let candidate = save.storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
    let center = [bell[0] - 20, bell[1], bell[2]];
    let found = world
        .level_mut_by_id(&dimension)
        .chunks
        .poi_mut()
        .find_closest_with_type(center, 48, PoiOccupancy::HasSpace, |record| {
            record.type_id == "minecraft:meeting"
        });
    assert_eq!(found.map(|record| record.pos), Some(bell));
    assert!(!loaded(&world, UNVISITED));
    save.remove();
}

/// The POI of a column evicted at unload stays answerable: a ticket released
/// there is kept when the column comes back and is saved.
#[test]
fn a_ticket_released_in_an_evicted_column_is_kept_and_saved() {
    let (save, bell) = save_with_bell("poi-evicted");
    let candidate = save.storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
    let key = overworld();
    load(&mut world, UNVISITED);
    assert!(world
        .level_mut_by_id(&dimension)
        .chunks
        .poi_mut()
        .acquire_ticket(bell));
    save.save(&mut world, &candidate);

    for _ in 0..2 {
        world
            .level_mut_by_id(&dimension)
            .chunk_lifecycle
            .purge_unknown();
    }
    world.refresh_loading_chunks(&dimension).unwrap();
    let request = world
        .runtime_unload_save_request(3, RegionCompression::Zlib)
        .unwrap();
    save.storage
        .save_runtime_unloads(&candidate, &request)
        .unwrap();
    let column = save
        .storage
        .read_runtime_column(&candidate, &key, UNVISITED.0, UNVISITED.1)
        .unwrap();
    world.retain_runtime_unload_columns(&key, vec![(UNVISITED, column)]);
    world.acknowledge_save_success(&request);
    assert!(!loaded(&world, UNVISITED));

    let poi = world.level_mut_by_id(&dimension).chunks.poi_mut();
    assert_eq!(poi.release_ticket(bell), Some(true));
    let free = poi.record(bell).unwrap().free_tickets;
    load(&mut world, UNVISITED);
    assert!(loaded(&world, UNVISITED));
    assert_eq!(
        world
            .level_mut_by_id(&dimension)
            .chunks
            .poi_mut()
            .record(bell)
            .unwrap()
            .free_tickets,
        free
    );
    save.save(&mut world, &candidate);
    let save = save.reopen();
    let reopened = save.storage.load_candidate().unwrap();
    let stored = reader(&reopened)
        .read_poi(&key, UNVISITED.0, UNVISITED.1)
        .unwrap()
        .unwrap();
    assert_eq!(stored.data.records().next().unwrap().free_tickets(), free);
    save.remove();
}
