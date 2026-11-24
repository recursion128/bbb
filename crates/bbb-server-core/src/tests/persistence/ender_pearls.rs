use super::*;
use bbb_server_persistence::Java26EntityRecordRef;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

const PLAYER: u128 = 0x661;
const PEARL: u128 = 0x662;
const PASSENGER: u128 = 0x663;
const VEHICLE: u128 = 0x664;
const INNER_PEARL: u128 = 0x665;
const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::End;

fn uuid(value: u128) -> NbtSummaryValue {
    crate::entity::save::uuid_int_array(Uuid::from_u128(value))
}

fn doubles(values: [f64; 3]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        values
            .map(|value| NbtSummaryValue::Double(value.to_bits()))
            .to_vec(),
    )
}

fn native_tree(nested: bool) -> NbtSummaryValue {
    let body = |id: &str, identity, mut fields: Vec<NbtSummaryEntry>| {
        fields.extend([
            nbt("id", NbtSummaryValue::String(id.into())),
            nbt("UUID", uuid(identity)),
            nbt("Pos", doubles([8.5, 200.0, 8.5])),
        ]);
        NbtSummaryValue::Compound(fields)
    };
    let marker = body(
        "minecraft:marker",
        PASSENGER,
        vec![nbt(
            "data",
            NbtSummaryValue::Compound(vec![nbt("fixture", NbtSummaryValue::Int(17))]),
        )],
    );
    let mut passengers = vec![marker];
    if nested {
        passengers.push(body(
            "minecraft:ender_pearl",
            INNER_PEARL,
            vec![nbt("Owner", uuid(PLAYER))],
        ));
    }
    let pearl = body(
        "minecraft:ender_pearl",
        PEARL,
        vec![
            nbt("Owner", uuid(PLAYER)),
            nbt("Motion", doubles([0.5, 0.1, 0.0])),
            nbt("LeftOwner", NbtSummaryValue::Byte(1)),
            nbt("HasBeenShot", NbtSummaryValue::Byte(1)),
            nbt(
                "data",
                NbtSummaryValue::Compound(vec![nbt("pearl_data", NbtSummaryValue::Int(23))]),
            ),
            nbt(
                "Item",
                NbtSummaryValue::Compound(vec![
                    nbt(
                        "id",
                        NbtSummaryValue::String("minecraft:ender_pearl".into()),
                    ),
                    nbt("count", NbtSummaryValue::Int(1)),
                    nbt(
                        "components",
                        NbtSummaryValue::Compound(vec![nbt(
                            "minecraft:custom_data",
                            NbtSummaryValue::Compound(vec![nbt("kept", NbtSummaryValue::Int(31))]),
                        )]),
                    ),
                ]),
            ),
            nbt("Passengers", NbtSummaryValue::List(passengers)),
        ],
    );
    if !nested {
        return pearl;
    }
    body(
        "minecraft:cow",
        VEHICLE,
        vec![nbt("Passengers", NbtSummaryValue::List(vec![pearl]))],
    )
}

fn candidate_with_native_pearl(storage: &Java26WorldStorage, nested: bool) -> Java26WorldCandidate {
    let mut candidate = storage.load_candidate().unwrap();
    let data = Java26EntityChunkData::parse(
        NamedCompound::new(
            "",
            vec![
                nbt("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                nbt("Position", NbtSummaryValue::IntArray(vec![0, 0])),
                nbt("Entities", NbtSummaryValue::List(vec![native_tree(nested)])),
            ],
        ),
        0,
        0,
    )
    .unwrap();
    candidate
        .dimensions
        .get_mut(&level_key(DIMENSION))
        .unwrap()
        .entity_chunks
        .insert(
            (0, 0),
            Java26LoadedEntityChunk {
                timestamp: 100,
                data,
            },
        );
    candidate
}

fn assert_live_tree(world: &ServerWorld) -> i32 {
    let id = world
        .live_entity_id_for_uuid_for_test(DIMENSION, Uuid::from_u128(PEARL))
        .unwrap();
    let pearl = world.stored_entity_for_test(DIMENSION, id).unwrap();
    let EntityData::EnderPearl(data) = pearl.data() else {
        panic!("restored pearl")
    };
    assert_eq!(data.projectile().owner(), Some(Uuid::from_u128(PLAYER)));
    let passenger = world
        .live_entity_id_for_uuid_for_test(DIMENSION, Uuid::from_u128(PASSENGER))
        .unwrap();
    assert!(world
        .stored_passengers_for_test(DIMENSION, id)
        .contains(&passenger));
    assert_eq!(
        world
            .stored_entity_ids_for_test(DIMENSION)
            .iter()
            .filter(|id| {
                world
                    .stored_entity_for_test(DIMENSION, **id)
                    .unwrap()
                    .uuid()
                    == Uuid::from_u128(PEARL)
            })
            .count(),
        1
    );
    id
}

fn assert_offline(world: &ServerWorld) {
    for dimension in [
        Java26BuiltinDimension::Overworld,
        Java26BuiltinDimension::Nether,
        DIMENSION,
    ] {
        for identity in [PEARL, PASSENGER, INNER_PEARL] {
            assert!(world
                .live_entity_id_for_uuid_for_test(dimension, Uuid::from_u128(identity))
                .is_none());
        }
    }
}

fn field<'a>(record: Java26EntityRecordRef<'a>, name: &str) -> Option<&'a NbtSummaryValue> {
    record
        .raw()
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

#[test]
fn player_ender_pearl_tree_saves_once_and_reopens_offline_until_owner_joins() {
    let root =
        std::env::temp_dir().join(format!("bbb-player-pearl-restart-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-player-pearl-restart");
    let paths = storage.paths().clone();
    let candidate = candidate_with_native_pearl(&storage, true);
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    world.tick_rate_mut().set_frozen(true);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "owner", PLAYER);
    let unload = world
        .runtime_unload_save_request(399, RegionCompression::Zlib)
        .unwrap();
    assert!(unload.player_patches.is_empty());
    world.acknowledge_save_success(&unload);
    assert_live_tree(&world);
    let mut request = world
        .save_request(1_786_000_022_100, 400, RegionCompression::Zlib)
        .unwrap();
    let saved = request.player_patches[0].ender_pearls[0].clone();
    assert_eq!(request.player_patches[0].ender_pearls.len(), 1);
    assert_eq!(saved.dimension, level_key(DIMENSION));
    assert_eq!(saved.entity.as_ref().uuid(), Uuid::from_u128(PEARL));
    assert_eq!(saved.entity.as_ref().passengers().len(), 2);
    assert_eq!(
        saved.entity.as_ref().passengers().nth(1).unwrap().uuid(),
        Uuid::from_u128(INNER_PEARL)
    );
    assert_eq!(
        saved.entity.as_ref().passengers().next().unwrap().uuid(),
        Uuid::from_u128(PASSENGER)
    );
    for write in &request.entity_writes {
        let chunk =
            Java26EntityChunkData::parse(write.document.clone(), write.chunk_x, write.chunk_z)
                .unwrap();
        assert!(chunk
            .entities()
            .all(|record| !record.contains_uuid(Uuid::from_u128(PEARL))
                && !record.contains_uuid(Uuid::from_u128(PASSENGER))));
    }
    // Official saves can retain the same tree in both playerdata and entity regions.
    let region = request
        .entity_writes
        .iter_mut()
        .find(|write| write.dimension == saved.dimension)
        .unwrap();
    let entities = region
        .document
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Entities")
        .unwrap();
    let NbtSummaryValue::List(entities) = &mut entities.value else {
        panic!("native entity list")
    };
    entities.push(NbtSummaryValue::Compound(
        saved.entity.as_ref().raw().to_vec(),
    ));
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    assert_eq!(
        candidate
            .load_player(Uuid::from_u128(PLAYER))
            .unwrap()
            .unwrap()
            .1
            .ender_pearls()
            .len(),
        1
    );
    let mut restarted = spawn_pinned_restore(datapack_config(), &candidate);
    restarted.tick_rate_mut().set_frozen(true);
    assert_offline(&restarted);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut restarted, 7, "owner", PLAYER);
    assert_live_tree(&restarted);
    let request = restarted
        .save_request(1_786_000_022_101, 401, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(request.player_patches[0].ender_pearls.len(), 1);
    let restored = request.player_patches[0].ender_pearls[0].entity.as_ref();
    assert_eq!(restored.passengers().len(), 2);
    assert_eq!(
        restored.passengers().nth(1).unwrap().uuid(),
        Uuid::from_u128(INNER_PEARL)
    );
    for name in [
        "UUID",
        "Owner",
        "Motion",
        "Item",
        "data",
        "LeftOwner",
        "HasBeenShot",
    ] {
        assert_eq!(
            field(restored, name),
            field(saved.entity.as_ref(), name),
            "{name}"
        );
    }
    assert_eq!(
        field(restored.passengers().next().unwrap(), "data"),
        field(saved.entity.as_ref().passengers().next().unwrap(), "data")
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn player_ender_pearl_disconnect_unloads_tree_and_new_connection_resumes_flight() {
    let root =
        std::env::temp_dir().join(format!("bbb-player-pearl-reconnect-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-player-pearl-reconnect");
    let candidate = candidate_with_native_pearl(&storage, false);
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    world.tick_rate_mut().set_frozen(true);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "owner", PLAYER);
    assert_live_tree(&world);
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert_offline(&world);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..8 {
        world.tick(Vec::new());
        assert_offline(&world);
    }
    world.tick_rate_mut().set_frozen(true);
    join_loaded(&sender, &receiver, &mut world, 9, "owner", PLAYER);
    let id = assert_live_tree(&world);
    let before = world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position();
    world.tick_rate_mut().set_frozen(false);
    world.tick(Vec::new());
    let after = world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position();
    assert!(after.x > before.x);
    assert!(world
        .entity_ticking_chunks(DIMENSION)
        .contains(&(before.x as i32 >> 4, before.z as i32 >> 4)));
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
