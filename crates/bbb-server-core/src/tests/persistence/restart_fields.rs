//! State with no block edit of its own that still has to survive a save and a restart.

use super::*;

fn saved_biome(
    candidate: &Java26WorldCandidate,
    chunk: (i32, i32),
    quart: (usize, usize),
) -> String {
    let saved = candidate.dimensions[&level_key(Java26BuiltinDimension::Overworld)]
        .chunks
        .get(&chunk)
        .expect("the filled column reached the region file");
    let section = saved
        .data
        .sections()
        .iter()
        .find(|section| section.y == -4)
        .expect("the bottom section");
    section
        .biomes
        .as_ref()
        .unwrap()
        .value_at(quart.0, 0, quart.1)
        .unwrap()
        .clone()
}

/// `FillBiomeCommand.java:143-144` marks every filled chunk unsaved, the stored
/// spawn column and a column the save has never held alike.
#[test]
fn fillbiome_survives_a_save_and_a_restart() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-fillbiome-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-fillbiome");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(operator_config(&[("op", 1)]), &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let filled = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "fillbiome 0 -64 0 19 -61 3 minecraft:desert",
    );
    assert!(system_chats(&filled)
        .iter()
        .any(|(_, text)| text.starts_with("commands.fillbiome.success")));
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 1 100 1 minecraft:diamond_block",
    );
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_detached_save(&request);
    let latest = world.save_request(2, 202, RegionCompression::Zlib).unwrap();
    storage.save_snapshot(&candidate, &latest).unwrap();
    world.acknowledge_save_success(&latest);
    let saved = storage.load_candidate().unwrap();
    let mut restarted = ServerWorld::restore(ServerWorldConfig::default(), &saved).unwrap();
    assert_eq!(
        restarted.block_state(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 1, y: 100, z: 1 },
        ),
        Some(block_state_id("minecraft:diamond_block"))
    );

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "fillbiome 0 -64 0 19 -61 3 minecraft:swamp",
    );
    let frozen = world.save_request(3, 203, RegionCompression::Zlib).unwrap();
    storage.save_snapshot(&candidate, &frozen).unwrap();
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "fillbiome 0 -64 0 19 -61 3 minecraft:desert",
    );
    world.acknowledge_detached_save(&frozen);
    let latest = world.save_request(4, 204, RegionCompression::Zlib).unwrap();
    storage.save_snapshot(&candidate, &latest).unwrap();
    world.acknowledge_save_success(&latest);
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
    for chunk in [(0, 0), (1, 0)] {
        assert_eq!(saved_biome(&reloaded, chunk, (0, 0)), "minecraft:desert");
        assert_ne!(saved_biome(&reloaded, chunk, (1, 1)), "minecraft:desert");
    }
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// One tick of a session restored from the fixture player with `patch`
/// applied to its document, and the save request that follows.
fn rejoined_save(patch: impl FnOnce(&mut Vec<NbtSummaryEntry>)) -> Java26SaveRequest {
    let uuid = Uuid::from_u128(0x5eed);
    let player = restore_fixture_player(uuid, [3.5, 70.0, 4.5], [90.0, -30.0], true);
    let mut raw = player.into_raw();
    patch(raw.entries_mut());
    let player = Java26PlayerData::parse(raw, uuid).unwrap();
    let candidate = restore_candidate(4_200, None, vec![(uuid, player)]);
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(7),
            profile: profile("alice", 0x5eed),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap()
}

fn uuid_ints(uuid: Uuid) -> NbtSummaryValue {
    let bits = uuid.as_u128();
    NbtSummaryValue::IntArray((0..4).map(|i| (bits >> (96 - 32 * i)) as i32).collect())
}

fn put(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    entries.retain(|entry| entry.name != name);
    entries.push(NbtSummaryEntry {
        name: name.into(),
        value,
    });
}

/// `ServerPlayer.java:392-394`: a player saved asleep loads awake.
#[test]
fn a_player_saved_asleep_wakes_on_load() {
    let request = rejoined_save(|entries| {
        put(
            entries,
            "sleeping_pos",
            NbtSummaryValue::IntArray(vec![3, 69, 4]),
        );
        put(entries, "SleepTimer", NbtSummaryValue::Short(40));
    });
    let created = &request.player_patches[0].created;
    assert_eq!(created.sleeping_pos, None);
    assert_eq!(created.sleep_timer, 0);
}

/// `Player.java:640`: the movement speed base follows `abilities.walkSpeed`.
#[test]
fn the_movement_speed_base_loads_from_the_walk_speed() {
    let request = rejoined_save(|entries| {
        let Some(NbtSummaryValue::Compound(abilities)) = entries
            .iter_mut()
            .find(|entry| entry.name == "abilities")
            .map(|entry| &mut entry.value)
        else {
            panic!("the fixture player has abilities");
        };
        put(
            abilities,
            "walkSpeed",
            NbtSummaryValue::Float(0.2f32.to_bits()),
        );
    });
    let movement_speed = request.player_patches[0]
        .created
        .attributes
        .iter()
        .find_map(|attribute| match attribute {
            NbtSummaryValue::Compound(fields)
                if fields.iter().any(|field| {
                    field.name == "id"
                        && field.value == NbtSummaryValue::String("minecraft:movement_speed".into())
                }) =>
            {
                fields
                    .iter()
                    .find(|field| field.name == "base")
                    .map(|field| field.value.clone())
            }
            _ => None,
        })
        .expect("the movement speed attribute is saved");
    assert_eq!(
        movement_speed,
        NbtSummaryValue::Double(f64::from(0.2f32).to_bits())
    );
}

/// `LivingEntity.java:739-747,812-815`: the remembered attackers come back.
#[test]
fn the_players_last_attackers_survive_a_reload() {
    let attacker = Uuid::from_u128(0xa77ac);
    let mob = Uuid::from_u128(0x0b);
    let request = rejoined_save(|entries| {
        put(entries, "last_hurt_by_player", uuid_ints(attacker));
        put(
            entries,
            "last_hurt_by_player_memory_time",
            NbtSummaryValue::Int(60),
        );
        put(entries, "last_hurt_by_mob", uuid_ints(mob));
        put(
            entries,
            "ticks_since_last_hurt_by_mob",
            NbtSummaryValue::Int(7),
        );
    });
    let saved = request.player_patches[0].created.last_hurt_by;
    let (player, memory) = saved.player.expect("the attacking player is remembered");
    assert_eq!(player, attacker);
    assert!((55..60).contains(&memory), "{memory}");
    assert_eq!(saved.mob.map(|(uuid, _)| uuid), Some(mob));
    assert_eq!(saved.hurt_by_timestamp, 7);
}
