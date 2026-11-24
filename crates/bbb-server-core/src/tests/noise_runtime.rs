//! End-to-end join coverage for the served built-in Overworld noise profile.

use super::*;
use bbb_server_types::save::Java26LevelKey;

mod background_generation;
mod early_structures;
mod original_mobs;
mod reload;
mod saved_full;

fn published_level_column_shape(
    world: &ServerWorld,
    dimension: Java26BuiltinDimension,
) -> (i32, usize) {
    let (min_y, height) = world
        .dimension_type_height(dimension)
        .expect("every loaded level has a published dimension type");
    (
        min_y,
        usize::try_from(height / 16).expect("dimension height is a section multiple"),
    )
}

fn compound_field_mut<'a>(
    entries: &'a mut [NbtSummaryEntry],
    name: &str,
) -> &'a mut Vec<NbtSummaryEntry> {
    let entry = entries.iter_mut().find(|entry| entry.name == name).unwrap();
    let NbtSummaryValue::Compound(value) = &mut entry.value else {
        panic!("{name} is not a compound")
    };
    value
}

fn noise_candidate() -> Java26WorldCandidate {
    let mut candidate = restore_candidate_without_tickets(0, None, Vec::new());
    let mut document = candidate.world_gen_settings.document.clone();
    let data = compound_field_mut(document.entries_mut(), "data");
    data.iter_mut()
        .find(|entry| entry.name == "seed")
        .unwrap()
        .value = NbtSummaryValue::Long(0);
    data.iter_mut()
        .find(|entry| entry.name == "generate_structures")
        .unwrap()
        .value = NbtSummaryValue::Byte(1);
    let dimensions = compound_field_mut(data, "dimensions");
    let overworld = compound_field_mut(dimensions, "minecraft:overworld");
    overworld
        .iter_mut()
        .find(|entry| entry.name == "generator")
        .unwrap()
        .value = crate::chunk::noise_generator::overworld_noise_stem().raw_generator;
    candidate.world_gen_settings =
        bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
            document, 0,
        )
        .unwrap();
    candidate
}

fn registry_biome_source_candidate(
    biome_source: NbtSummaryValue,
    generate_structures: bool,
) -> Java26WorldCandidate {
    overworld_noise_candidate_with_source_and_settings(
        generate_structures,
        NbtSummaryValue::string("minecraft:overworld"),
        biome_source,
    )
}

fn overworld_noise_candidate_with_source_and_settings(
    generate_structures: bool,
    settings: NbtSummaryValue,
    biome_source: NbtSummaryValue,
) -> Java26WorldCandidate {
    let mut candidate = restore_candidate_without_tickets(0, None, Vec::new());
    let mut document = candidate.world_gen_settings.document.clone();
    let data = compound_field_mut(document.entries_mut(), "data");
    data.iter_mut()
        .find(|entry| entry.name == "seed")
        .unwrap()
        .value = NbtSummaryValue::Long(0);
    data.iter_mut()
        .find(|entry| entry.name == "generate_structures")
        .unwrap()
        .value = NbtSummaryValue::Byte(i8::from(generate_structures));
    let dimensions = compound_field_mut(data, "dimensions");
    let overworld = compound_field_mut(dimensions, "minecraft:overworld");
    let mut generator = crate::chunk::noise_generator::overworld_noise_stem().raw_generator;
    let NbtSummaryValue::Compound(fields) = &mut generator else {
        unreachable!()
    };
    fields
        .iter_mut()
        .find(|entry| entry.name == "biome_source")
        .unwrap()
        .value = biome_source;
    fields
        .iter_mut()
        .find(|entry| entry.name == "settings")
        .unwrap()
        .value = settings;
    overworld
        .iter_mut()
        .find(|entry| entry.name == "generator")
        .unwrap()
        .value = generator;
    candidate.world_gen_settings =
        bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
            document, 0,
        )
        .unwrap();
    candidate
}

fn fixed_biome_source(biome: NbtSummaryValue) -> NbtSummaryValue {
    crate::chunk::noise_generator::test_support::compound(vec![
        ("type", NbtSummaryValue::string("minecraft:fixed")),
        ("biome", biome),
    ])
}

fn checkerboard_biome_source(biomes: NbtSummaryValue, scale: Option<i32>) -> NbtSummaryValue {
    let mut fields = vec![
        ("type", NbtSummaryValue::string("minecraft:checkerboard")),
        ("biomes", biomes),
    ];
    if let Some(scale) = scale {
        fields.push(("scale", NbtSummaryValue::Int(scale)));
    }
    crate::chunk::noise_generator::test_support::compound(fields)
}

fn direct_multi_noise_biome_source(biome: NbtSummaryValue) -> NbtSummaryValue {
    let parameter = |name: &str| NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Float(0.0_f32.to_bits()),
    };
    crate::chunk::noise_generator::test_support::compound(vec![
        ("type", NbtSummaryValue::string("minecraft:multi_noise")),
        (
            "biomes",
            NbtSummaryValue::List(vec![crate::chunk::noise_generator::test_support::compound(
                vec![
                    ("biome", biome),
                    (
                        "parameters",
                        NbtSummaryValue::Compound(vec![
                            parameter("temperature"),
                            parameter("humidity"),
                            parameter("continentalness"),
                            parameter("erosion"),
                            parameter("depth"),
                            parameter("weirdness"),
                            parameter("offset"),
                        ]),
                    ),
                ],
            )]),
        ),
    ])
}

fn nether_noise_candidate(generate_structures: bool) -> Java26WorldCandidate {
    nether_noise_candidate_with_settings(
        generate_structures,
        NbtSummaryValue::string("minecraft:nether"),
    )
}

fn nether_noise_candidate_with_settings(
    generate_structures: bool,
    settings: NbtSummaryValue,
) -> Java26WorldCandidate {
    nether_noise_candidate_with_source_and_settings(
        generate_structures,
        settings,
        crate::chunk::noise_generator::test_support::compound(vec![
            ("type", NbtSummaryValue::string("minecraft:multi_noise")),
            ("preset", NbtSummaryValue::string("minecraft:nether")),
        ]),
    )
}

fn nether_noise_candidate_with_source_and_settings(
    generate_structures: bool,
    settings: NbtSummaryValue,
    biome_source: NbtSummaryValue,
) -> Java26WorldCandidate {
    let mut candidate = restore_candidate_without_tickets(0, None, Vec::new());
    let mut document = candidate.world_gen_settings.document.clone();
    let data = compound_field_mut(document.entries_mut(), "data");
    data.iter_mut()
        .find(|entry| entry.name == "seed")
        .unwrap()
        .value = NbtSummaryValue::Long(0);
    data.iter_mut()
        .find(|entry| entry.name == "generate_structures")
        .unwrap()
        .value = NbtSummaryValue::Byte(i8::from(generate_structures));
    let dimensions = compound_field_mut(data, "dimensions");
    let nether = compound_field_mut(dimensions, "minecraft:the_nether");
    let mut generator =
        crate::chunk::noise_generator::test_support::nether_noise_stem().raw_generator;
    let NbtSummaryValue::Compound(fields) = &mut generator else {
        unreachable!()
    };
    fields
        .iter_mut()
        .find(|entry| entry.name == "biome_source")
        .unwrap()
        .value = biome_source;
    fields
        .iter_mut()
        .find(|entry| entry.name == "settings")
        .unwrap()
        .value = settings;
    nether
        .iter_mut()
        .find(|entry| entry.name == "generator")
        .unwrap()
        .value = generator;
    candidate.world_gen_settings =
        bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
            document, 0,
        )
        .unwrap();
    candidate
}

fn end_noise_candidate(generate_structures: bool) -> Java26WorldCandidate {
    end_noise_candidate_with_settings(
        generate_structures,
        NbtSummaryValue::string("minecraft:end"),
    )
}

fn end_noise_candidate_with_settings(
    generate_structures: bool,
    settings: NbtSummaryValue,
) -> Java26WorldCandidate {
    end_noise_candidate_with_source_and_settings(
        generate_structures,
        settings,
        crate::chunk::noise_generator::test_support::compound(vec![(
            "type",
            NbtSummaryValue::string("minecraft:the_end"),
        )]),
    )
}

fn end_noise_candidate_with_source_and_settings(
    generate_structures: bool,
    settings: NbtSummaryValue,
    biome_source: NbtSummaryValue,
) -> Java26WorldCandidate {
    let mut candidate = restore_candidate_without_tickets(0, None, Vec::new());
    let mut document = candidate.world_gen_settings.document.clone();
    let data = compound_field_mut(document.entries_mut(), "data");
    data.iter_mut()
        .find(|entry| entry.name == "seed")
        .unwrap()
        .value = NbtSummaryValue::Long(0);
    data.iter_mut()
        .find(|entry| entry.name == "generate_structures")
        .unwrap()
        .value = NbtSummaryValue::Byte(i8::from(generate_structures));
    let dimensions = compound_field_mut(data, "dimensions");
    let end = compound_field_mut(dimensions, "minecraft:the_end");
    end.iter_mut()
        .find(|entry| entry.name == "generator")
        .unwrap()
        .value = crate::chunk::noise_generator::test_support::compound(vec![
        ("type", NbtSummaryValue::string("minecraft:noise")),
        ("biome_source", biome_source),
        ("settings", settings),
    ]);
    candidate.world_gen_settings =
        bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
            document, 0,
        )
        .unwrap();
    candidate
}

fn alternate_end_startup() -> (
    Arc<bbb_server_datapack::startup::DatapackStartup>,
    NbtSummaryValue,
) {
    let mut startup =
        Arc::new(crate::chunk::noise_generator::test_support::fresh_overworld_startup(true));
    let registry = Arc::get_mut(&mut startup)
        .expect("the startup has not been shared")
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap();
    let mut alternate = registry
        .elements
        .iter()
        .find(|element| element.id == "minecraft:end")
        .unwrap()
        .clone();
    alternate.id = "example:alternate_end".to_string();
    alternate.source_pack = "file/alternate_end".to_string();
    let json = serde_json::from_slice(&alternate.source_json).unwrap();
    let inline = bbb_protocol::json_value_to_nbt_summary(&json).unwrap();
    crate::chunk::noise_generator::test_support::publish_worldgen_element(
        &mut Arc::get_mut(&mut startup).unwrap().registries,
        "minecraft:worldgen/noise_settings",
        alternate,
    );
    (startup, inline)
}

fn alternate_nether_startup() -> (
    Arc<bbb_server_datapack::startup::DatapackStartup>,
    NbtSummaryValue,
) {
    let mut startup =
        Arc::new(crate::chunk::noise_generator::test_support::fresh_overworld_startup(true));
    let registry = Arc::get_mut(&mut startup)
        .expect("the startup has not been shared")
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap();
    let mut alternate = registry
        .elements
        .iter()
        .find(|element| element.id == "minecraft:nether")
        .unwrap()
        .clone();
    alternate.id = "example:alternate_nether".to_string();
    alternate.source_pack = "file/alternate_nether".to_string();
    let json = serde_json::from_slice(&alternate.source_json).unwrap();
    let inline = bbb_protocol::json_value_to_nbt_summary(&json).unwrap();
    crate::chunk::noise_generator::test_support::publish_worldgen_element(
        &mut Arc::get_mut(&mut startup).unwrap().registries,
        "minecraft:worldgen/noise_settings",
        alternate,
    );
    (startup, inline)
}

fn preset_multi_noise_biome_source(preset: &str) -> NbtSummaryValue {
    crate::chunk::noise_generator::test_support::compound(vec![
        ("type", NbtSummaryValue::string("minecraft:multi_noise")),
        ("preset", NbtSummaryValue::string(preset)),
    ])
}

fn inline_preset_multi_noise_biome_source(preset: &str) -> NbtSummaryValue {
    crate::chunk::noise_generator::test_support::compound(vec![
        ("type", NbtSummaryValue::string("minecraft:multi_noise")),
        (
            "preset",
            crate::chunk::noise_generator::test_support::compound(vec![(
                "preset",
                NbtSummaryValue::string(preset),
            )]),
        ),
    ])
}

fn assert_profile_served_in_all_builtin_levels(
    startup: &Arc<bbb_server_datapack::startup::DatapackStartup>,
    settings: NbtSummaryValue,
    biome_source: NbtSummaryValue,
) {
    for dimension in Java26BuiltinDimension::ALL {
        let candidate = match dimension {
            Java26BuiltinDimension::Overworld => {
                overworld_noise_candidate_with_source_and_settings(
                    false,
                    settings.clone(),
                    biome_source.clone(),
                )
            }
            Java26BuiltinDimension::Nether => nether_noise_candidate_with_source_and_settings(
                false,
                settings.clone(),
                biome_source.clone(),
            ),
            Java26BuiltinDimension::End => end_noise_candidate_with_source_and_settings(
                false,
                settings.clone(),
                biome_source.clone(),
            ),
        };
        let world = ServerWorld::restore(
            ServerWorldConfig {
                datapacks: Some(Arc::clone(startup)),
                ..ServerWorldConfig::default()
            },
            &candidate,
        )
        .unwrap();
        assert!(world.served_builtin_dimensions().contains(&dimension));
        assert!(!world
            .unserved_levels()
            .iter()
            .any(|(level, _)| *level == dimension.resource_id()));
    }
}

fn noise_world(candidate: &Java26WorldCandidate) -> ServerWorld {
    ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(crate::chunk::noise_generator::overworld_startup(true)),
            ..ServerWorldConfig::default()
        },
        candidate,
    )
    .unwrap()
}

#[test]
fn generic_codec_backed_biome_sources() {
    let mut startup =
        Arc::new(crate::chunk::noise_generator::test_support::fresh_overworld_startup(true));
    let parameter_lists = Arc::get_mut(&mut startup)
        .expect("the startup has not been shared")
        .registries
        .registries
        .iter_mut()
        .find(|registry| {
            registry.registry_id == "minecraft:worldgen/multi_noise_biome_source_parameter_list"
        })
        .unwrap();
    for (canonical, alias) in [
        ("minecraft:overworld", "example:temperate"),
        ("minecraft:nether", "example:infernal"),
    ] {
        let mut winner = parameter_lists
            .elements
            .iter()
            .find(|element| element.id == canonical)
            .unwrap()
            .clone();
        winner.id = alias.to_string();
        winner.source_pack = "file/biome_sources".to_string();
        parameter_lists.elements.push(winner);
    }

    for source in [
        preset_multi_noise_biome_source("example:temperate"),
        inline_preset_multi_noise_biome_source("minecraft:overworld"),
        direct_multi_noise_biome_source(NbtSummaryValue::string("minecraft:plains")),
        fixed_biome_source(NbtSummaryValue::string("minecraft:plains")),
    ] {
        assert_profile_served_in_all_builtin_levels(
            &startup,
            NbtSummaryValue::string("minecraft:overworld"),
            source,
        );
    }
    for source in [
        preset_multi_noise_biome_source("example:infernal"),
        inline_preset_multi_noise_biome_source("minecraft:nether"),
    ] {
        assert_profile_served_in_all_builtin_levels(
            &startup,
            NbtSummaryValue::string("minecraft:nether"),
            source,
        );
    }

    let candidate = overworld_noise_candidate_with_source_and_settings(
        false,
        NbtSummaryValue::string("minecraft:overworld"),
        preset_multi_noise_biome_source("example:temperate"),
    );
    let stem = candidate
        .world_gen_settings
        .dimensions
        .get(&Java26LevelKey::from(Java26BuiltinDimension::Overworld))
        .unwrap();
    let aliased = crate::chunk::noise_generator::compile_noise_level_source(
        stem,
        Arc::clone(&startup),
        0,
        "dimensions.minecraft:overworld.generator",
    )
    .unwrap();
    let canonical = crate::chunk::noise_generator::compile_noise_level_source(
        &overworld_noise_candidate_with_source_and_settings(
            false,
            NbtSummaryValue::string("minecraft:overworld"),
            preset_multi_noise_biome_source("minecraft:overworld"),
        )
        .world_gen_settings
        .dimensions[&Java26LevelKey::from(Java26BuiltinDimension::Overworld)],
        Arc::clone(&startup),
        0,
        "dimensions.minecraft:overworld.generator",
    )
    .unwrap();
    let aliased_chunk = aliased.generate_fresh_base_chunk(0, 0).unwrap();
    let canonical_chunk = canonical.generate_fresh_base_chunk(0, 0).unwrap();
    for (quart_x, quart_y, quart_z) in [(0, -16, 0), (3, 0, 2), (1, 79, 3)] {
        assert_eq!(
            aliased_chunk
                .column()
                .get_noise_biome(quart_x, quart_y, quart_z),
            canonical_chunk
                .column()
                .get_noise_biome(quart_x, quart_y, quart_z),
        );
    }
}

fn join_noise_player(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
) -> TickResult {
    let name = format!("noise_join_{connection_id}");
    enqueue(
        sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(connection_id),
            profile: profile(&name, u128::from(connection_id)),
        },
    )
    .unwrap();
    world.tick(receiver.drain())
}

fn acknowledge_noise_join(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
) {
    let tick = play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::ChunkBatchReceived {
            desired_chunks_per_tick: 9.0,
        },
    );
    assert!(closed_reasons(&tick).is_empty());
}

fn tick_until_noise_full(
    world: &mut ServerWorld,
    dimension: Java26BuiltinDimension,
    chunk: (i32, i32),
) -> Vec<WorldEvent> {
    let mut events = Vec::new();
    for _ in 0..64 {
        events.extend(world.tick(Vec::new()).events);
        if matches!(
            world
                .chunk_store_for_test(dimension)
                .generation_state(chunk.0, chunk.1),
            Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
        ) {
            return events;
        }
    }
    let closes: Vec<_> = events
        .iter()
        .filter(|event| matches!(event, WorldEvent::Close { .. }))
        .collect();
    panic!("{dimension:?} chunk {chunk:?} did not reach noise FULL: {closes:#?}")
}

#[test]
fn registry_backed_biome_sources_fixed_reaches_full_and_persistence() {
    let startup = crate::chunk::noise_generator::overworld_startup(true);
    let candidate = registry_biome_source_candidate(
        fixed_biome_source(NbtSummaryValue::string("minecraft:plains")),
        false,
    );
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(Arc::clone(&startup)),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    assert!(!world
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:overworld"));
    let plains = startup
        .registries
        .registry("minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter()
        .position(|element| element.id == "minecraft:plains")
        .unwrap() as i32;
    let (sender, receiver) = mailbox();
    let joined = join_noise_player(&sender, &receiver, &mut world, 395);
    let packets = chunk_packets(&joined, 395);
    assert_eq!(packets.len(), 9);
    for packet in &packets {
        let column = world
            .chunk_store_for_test(Java26BuiltinDimension::Overworld)
            .column(packet.x, packet.z)
            .unwrap();
        for quart_y in -16..80 {
            assert_eq!(column.get_noise_biome(0, quart_y, 0), Some(plains));
        }
    }
    let request = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    assert!(request.chunk_writes.iter().any(|write| {
        write.dimension == Java26LevelKey::from(Java26BuiltinDimension::Overworld)
            && Java26ChunkData::parse(
                write.document.clone(),
                Java26ChunkContext {
                    expected_x: write.chunk_x,
                    expected_z: write.chunk_z,
                    min_section_y: -4,
                    section_count: 24,
                },
            )
            .is_ok_and(|chunk| chunk.status() == Java26ChunkStatus::Full)
    }));
}

#[test]
fn registry_backed_biome_sources_checkerboard_and_refusals_are_source_exact() {
    let mut startup =
        Arc::new(crate::chunk::noise_generator::test_support::fresh_overworld_startup(true));
    let tag_registry = Arc::get_mut(&mut startup)
        .expect("the startup has not been shared")
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/biome")
        .unwrap();
    tag_registry.tags.insert(
        "test:checkerboard".to_string(),
        vec![
            "minecraft:plains".to_string(),
            "minecraft:desert".to_string(),
        ],
    );
    let biome_id = |id: &str| {
        startup
            .registries
            .registry("minecraft:worldgen/biome")
            .unwrap()
            .elements
            .iter()
            .position(|element| element.id == id)
            .unwrap() as i32
    };
    let plains = biome_id("minecraft:plains");
    let desert = biome_id("minecraft:desert");

    let source = checkerboard_biome_source(
        NbtSummaryValue::List(vec![
            NbtSummaryValue::string("minecraft:plains"),
            NbtSummaryValue::string("minecraft:desert"),
            NbtSummaryValue::string("minecraft:plains"),
        ]),
        Some(62),
    );
    let candidate = registry_biome_source_candidate(source, false);
    let stem = candidate
        .world_gen_settings
        .dimensions
        .get(&Java26LevelKey::from(Java26BuiltinDimension::Overworld))
        .unwrap();
    let program = crate::chunk::noise_generator::compile_noise_level_source(
        stem,
        Arc::clone(&startup),
        0,
        "dimensions.minecraft:overworld.generator",
    )
    .unwrap();
    let generated = program.generate_fresh_base_chunk(0, 0).unwrap();
    assert_eq!(generated.column().get_noise_biome(0, 0, 0), Some(plains));
    assert_eq!(generated.column().get_noise_biome(1, 0, 0), Some(desert));
    assert_eq!(generated.column().get_noise_biome(2, 0, 0), Some(plains));
    assert_eq!(generated.column().get_noise_biome(3, 0, 0), Some(plains));

    let tagged = registry_biome_source_candidate(
        checkerboard_biome_source(NbtSummaryValue::string("#test:checkerboard"), None),
        false,
    );
    let tagged_stem = tagged
        .world_gen_settings
        .dimensions
        .get(&Java26LevelKey::from(Java26BuiltinDimension::Overworld))
        .unwrap();
    let tagged_program = crate::chunk::noise_generator::compile_noise_level_source(
        tagged_stem,
        Arc::clone(&startup),
        0,
        "dimensions.minecraft:overworld.generator",
    )
    .unwrap();
    let origin = tagged_program.generate_fresh_base_chunk(0, 0).unwrap();
    let next_band = tagged_program.generate_fresh_base_chunk(4, 0).unwrap();
    assert_eq!(origin.column().get_noise_biome(0, 0, 0), Some(plains));
    assert_eq!(next_band.column().get_noise_biome(0, 0, 0), Some(desert));
    let checkerboard_world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(Arc::clone(&startup)),
            ..ServerWorldConfig::default()
        },
        &tagged,
    )
    .unwrap();
    assert!(!checkerboard_world
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:overworld"));
}

#[test]
fn fresh_builtin_nether_noise_is_served_and_reopens_from_the_same_stem() {
    let startup = crate::chunk::noise_generator::overworld_startup(true);
    let config = ServerWorldConfig {
        datapacks: Some(Arc::clone(&startup)),
        ..ServerWorldConfig::default()
    };
    let mut fresh = ServerWorld::new(config.clone()).unwrap();
    assert!(fresh
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::Nether));
    assert!(!fresh
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_nether"));
    assert_eq!(
        fresh.level_column_shape(Java26BuiltinDimension::Nether),
        published_level_column_shape(&fresh, Java26BuiltinDimension::Nether)
    );

    let (sender, receiver) = mailbox();
    join_noise_player(&sender, &receiver, &mut fresh, 97);
    acknowledge_noise_join(&sender, &receiver, &mut fresh, 97);
    let mut transition = Vec::new();
    assert!(fresh.teleport_player(
        ConnectionId(97),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut transition,
    ));
    tick_until_noise_full(&mut fresh, Java26BuiltinDimension::Nether, (0, 0));
    assert!(matches!(
        fresh
            .chunk_store_for_test(Java26BuiltinDimension::Nether)
            .generation_state(0, 0),
        Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
    ));

    let mut candidate = restore_candidate_without_tickets(0, None, Vec::new());
    candidate.world_gen_settings = crate::world_creation::default_world_gen_settings().unwrap();
    let reopened = ServerWorld::restore(config, &candidate).unwrap();
    assert!(reopened
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::Nether));
    assert!(!reopened
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_nether"));

    let mut replaced = (*startup).clone();
    replaced
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:nether")
        .unwrap()
        .source_pack = "example:replacement".to_string();
    let replacement = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(Arc::new(replaced)),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    assert_eq!(
        replacement.level_column_shape(Java26BuiltinDimension::Nether),
        reopened.level_column_shape(Java26BuiltinDimension::Nether)
    );
    assert!(!replacement
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_nether"));
}

#[test]
fn structure_disabled_builtin_nether_is_served_joined_and_persisted() {
    let candidate = nether_noise_candidate(false);
    let mut world = noise_world(&candidate);
    assert!(world
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::Nether));
    assert!(!world
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_nether"));
    assert_eq!(
        world.level_column_shape(Java26BuiltinDimension::Nether),
        published_level_column_shape(&world, Java26BuiltinDimension::Nether)
    );

    let (sender, receiver) = mailbox();
    join_noise_player(&sender, &receiver, &mut world, 96);
    acknowledge_noise_join(&sender, &receiver, &mut world, 96);
    let mut transition = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(96),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut transition,
    ));
    let events = tick_until_noise_full(&mut world, Java26BuiltinDimension::Nether, (0, 0));
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(96),
            packet: ClientboundPlayPacket::LevelChunkWithLight(_),
        }
    )));
    assert!(matches!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::Nether)
            .generation_state(0, 0),
        Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
    ));
    let request = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    assert!(request.chunk_writes.iter().any(|write| {
        write.dimension == Java26LevelKey::from(Java26BuiltinDimension::Nether)
            && Java26ChunkData::parse(
                write.document.clone(),
                Java26ChunkContext {
                    expected_x: write.chunk_x,
                    expected_z: write.chunk_z,
                    min_section_y: 0,
                    section_count: 16,
                },
            )
            .is_ok_and(|chunk| chunk.status() == Java26ChunkStatus::Full)
    }));

    let default_structures = noise_world(&nether_noise_candidate(true));
    assert!(default_structures
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::Nether));
    assert!(!default_structures
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_nether"));
}

#[test]
fn restored_forced_nether_ticket_is_full_and_accepts_a_live_write() {
    let mut candidate = nether_noise_candidate(false);
    candidate.chunk_tickets.insert(
        Java26LevelKey::from(Java26BuiltinDimension::Nether),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 31,
            ticks_left: None,
        }],
    );
    let mut world = noise_world(&candidate);
    assert!(matches!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::Nether)
            .generation_state(0, 0),
        Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
    ));
    assert!(matches!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::Nether)
            .generation_state(2, 0),
        Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
    ));
    assert!(!matches!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::Nether)
            .generation_state(3, 0),
        Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
    ));
    assert!(!world
        .block_ticking_chunks(Java26BuiltinDimension::Nether)
        .contains(&(2, 0)));
    assert!(!world
        .entity_ticking_chunks(Java26BuiltinDimension::Nether)
        .contains(&(1, 0)));
    assert!(world.chunk_tick_containers_registered_for_test(Java26BuiltinDimension::Nether, (0, 0)));
    assert!(world.chunk_tick_containers_registered_for_test(Java26BuiltinDimension::Nether, (2, 0)));

    let pos = BlockPos { x: 32, y: 64, z: 0 };
    let diamond = block_state_id("minecraft:diamond_block");
    assert!(world.set_block(
        Java26BuiltinDimension::Nether,
        pos,
        diamond,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Nether, pos),
        Some(diamond)
    );
}

#[test]
fn restored_portal_end_ticket_loads_its_full_radius_without_aging() {
    let mut candidate = end_noise_candidate(false);
    candidate.chunk_tickets.insert(
        Java26LevelKey::from(Java26BuiltinDimension::End),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 30,
            ticks_left: Some(300),
        }],
    );
    let world = noise_world(&candidate);
    assert!(matches!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::End)
            .generation_state(3, 0),
        Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
    ));
    assert!(!matches!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::End)
            .generation_state(4, 0),
        Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
    ));
    assert!(!world
        .block_ticking_chunks(Java26BuiltinDimension::End)
        .contains(&(3, 0)));
    assert_eq!(
        world.restored_chunk_ticket_for_test(Java26BuiltinDimension::End, (0, 0)),
        Some((30, Some(300)))
    );
}

#[test]
fn restored_forced_end_ticket_imports_full_worldgen_entities() {
    let spike =
        bbb_server_worldgen::end_spike::spikes_for_seed(bbb_server_persistence::CREATED_WORLD_SEED)
            [0];
    let chunk = (spike.center_x.div_euclid(16), spike.center_z.div_euclid(16));
    let mut candidate = end_noise_candidate(false);
    candidate.chunk_tickets.insert(
        Java26LevelKey::from(Java26BuiltinDimension::End),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: chunk.0,
            chunk_z: chunk.1,
            level: 31,
            ticks_left: None,
        }],
    );
    let world = noise_world(&candidate);
    let chunks = world.chunk_store_for_test(Java26BuiltinDimension::End);
    for dx in -2..=2 {
        for dz in -2..=2 {
            let loaded = (chunk.0 + dx, chunk.1 + dz);
            assert!(matches!(
                chunks.generation_state(loaded.0, loaded.1),
                Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
            ));
            assert!(chunks
                .published_worldgen_entity_records(loaded.0, loaded.1)
                .is_empty());
        }
    }
    let (stored, tracked) = world.entity_tracking_counts(Java26BuiltinDimension::End);
    assert!(stored > 0);
    assert_eq!(tracked, stored);
}

#[test]
fn builtin_end_is_restored_as_served_noise_with_or_without_structures() {
    let world = noise_world(&end_noise_candidate(false));
    assert!(world
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::End));
    assert!(!world
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_end"));
    assert_eq!(
        world.level_column_shape(Java26BuiltinDimension::End),
        published_level_column_shape(&world, Java26BuiltinDimension::End)
    );

    let mut structures = noise_world(&end_noise_candidate(true));
    assert!(structures
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::End));
    assert!(!structures
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_end"));

    let (sender, receiver) = mailbox();
    join_noise_player(&sender, &receiver, &mut structures, 95);
    acknowledge_noise_join(&sender, &receiver, &mut structures, 95);
    let mut transition = Vec::new();
    assert!(structures.teleport_player(
        ConnectionId(95),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::End.into(),
            position: Vec3d {
                x: 96.5,
                y: 64.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut transition,
    ));
    let events = tick_until_noise_full(&mut structures, Java26BuiltinDimension::End, (6, 0));
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(95),
            packet: ClientboundPlayPacket::LevelChunkWithLight(_),
        }
    )));
    assert!(matches!(
        structures
            .chunk_store_for_test(Java26BuiltinDimension::End)
            .generation_state(6, 0),
        Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
    ));
    let request = structures
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    assert!(request.chunk_writes.iter().any(|write| {
        write.dimension == Java26LevelKey::from(Java26BuiltinDimension::End)
            && Java26ChunkData::parse(
                write.document.clone(),
                Java26ChunkContext {
                    expected_x: write.chunk_x,
                    expected_z: write.chunk_z,
                    min_section_y: 0,
                    section_count: 16,
                },
            )
            .is_ok_and(|chunk| chunk.status() == Java26ChunkStatus::Full)
    }));
}

#[test]
fn fresh_builtin_end_noise_is_served_and_reopens_from_the_same_stem() {
    let startup = crate::chunk::noise_generator::overworld_startup(true);
    let config = ServerWorldConfig {
        datapacks: Some(Arc::clone(&startup)),
        ..ServerWorldConfig::default()
    };
    let mut fresh = ServerWorld::new(config.clone()).unwrap();
    assert!(fresh
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::End));
    assert!(!fresh
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_end"));
    assert_eq!(
        fresh.level_column_shape(Java26BuiltinDimension::End),
        published_level_column_shape(&fresh, Java26BuiltinDimension::End)
    );

    let (sender, receiver) = mailbox();
    join_noise_player(&sender, &receiver, &mut fresh, 98);
    acknowledge_noise_join(&sender, &receiver, &mut fresh, 98);
    let mut transition = Vec::new();
    assert!(fresh.teleport_player(
        ConnectionId(98),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::End.into(),
            position: Vec3d {
                x: 96.5,
                y: 64.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut transition,
    ));
    tick_until_noise_full(&mut fresh, Java26BuiltinDimension::End, (6, 0));
    assert!(matches!(
        fresh
            .chunk_store_for_test(Java26BuiltinDimension::End)
            .generation_state(6, 0),
        Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
    ));

    let mut candidate = restore_candidate_without_tickets(0, None, Vec::new());
    candidate.world_gen_settings = crate::world_creation::default_world_gen_settings().unwrap();
    let reopened = ServerWorld::restore(config, &candidate).unwrap();
    assert!(reopened
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::End));
    assert!(!reopened
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_end"));

    let mut replaced = (*startup).clone();
    replaced
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:end")
        .unwrap()
        .source_pack = "example:replacement".to_string();
    let replacement = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(Arc::new(replaced)),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    assert_eq!(
        replacement.level_column_shape(Java26BuiltinDimension::End),
        reopened.level_column_shape(Java26BuiltinDimension::End)
    );
    assert!(!replacement
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_end"));
}

#[test]
fn alternate_end_noise_profile_named_and_inline_reaches_full_without_fallback() {
    let (startup, inline) = alternate_end_startup();
    for (index, settings) in [NbtSummaryValue::string("example:alternate_end"), inline]
        .into_iter()
        .enumerate()
    {
        let candidate = end_noise_candidate_with_settings(true, settings);
        let mut world = ServerWorld::restore(
            ServerWorldConfig {
                datapacks: Some(Arc::clone(&startup)),
                ..ServerWorldConfig::default()
            },
            &candidate,
        )
        .unwrap();
        assert!(world
            .served_builtin_dimensions()
            .contains(&Java26BuiltinDimension::End));
        assert!(!world
            .unserved_levels()
            .iter()
            .any(|(dimension, _)| *dimension == "minecraft:the_end"));

        let connection_id = 195 + u64::try_from(index).unwrap();
        let (sender, receiver) = mailbox();
        join_noise_player(&sender, &receiver, &mut world, connection_id);
        acknowledge_noise_join(&sender, &receiver, &mut world, connection_id);
        let mut transition = Vec::new();
        assert!(world.teleport_player(
            ConnectionId(connection_id),
            crate::world::TeleportTransition {
                new_level: Java26BuiltinDimension::End.into(),
                position: Vec3d {
                    x: 96.5,
                    y: 64.0,
                    z: 0.5,
                },
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: 0,
            },
            &mut transition,
        ));
        let events = tick_until_noise_full(&mut world, Java26BuiltinDimension::End, (6, 0));
        assert!(events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: recipient,
                packet: ClientboundPlayPacket::LevelChunkWithLight(_),
            } if *recipient == ConnectionId(connection_id)
        )));
        assert!(matches!(
            world
                .chunk_store_for_test(Java26BuiltinDimension::End)
                .generation_state(6, 0),
            Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
        ));
        let request = world
            .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
            .unwrap();
        assert!(request.chunk_writes.iter().any(|write| {
            write.dimension == Java26LevelKey::from(Java26BuiltinDimension::End)
                && Java26ChunkData::parse(
                    write.document.clone(),
                    Java26ChunkContext {
                        expected_x: write.chunk_x,
                        expected_z: write.chunk_z,
                        min_section_y: 0,
                        section_count: 16,
                    },
                )
                .is_ok_and(|chunk| chunk.status() == Java26ChunkStatus::Full)
        }));
    }
}

#[test]
fn direct_multi_noise_end_profile_reaches_full_with_source_exact_structures() {
    let (startup, inline) = alternate_end_startup();
    let highlands = startup
        .registries
        .registry("minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter()
        .position(|element| element.id == "minecraft:end_highlands")
        .unwrap() as i32;
    for (index, settings) in [NbtSummaryValue::string("example:alternate_end"), inline]
        .into_iter()
        .enumerate()
    {
        let candidate = end_noise_candidate_with_source_and_settings(
            true,
            settings,
            direct_multi_noise_biome_source(NbtSummaryValue::String(
                "minecraft:end_highlands".into(),
            )),
        );
        let mut world = ServerWorld::restore(
            ServerWorldConfig {
                datapacks: Some(Arc::clone(&startup)),
                ..ServerWorldConfig::default()
            },
            &candidate,
        )
        .unwrap();
        assert!(world
            .served_builtin_dimensions()
            .contains(&Java26BuiltinDimension::End));
        assert!(!world
            .unserved_levels()
            .iter()
            .any(|(dimension, _)| *dimension == "minecraft:the_end"));

        if index == 0 {
            let (sender, receiver) = mailbox();
            join_noise_player(&sender, &receiver, &mut world, 495);
            acknowledge_noise_join(&sender, &receiver, &mut world, 495);
            let mut transition = Vec::new();
            assert!(world.teleport_player(
                ConnectionId(495),
                crate::world::TeleportTransition {
                    new_level: Java26BuiltinDimension::End.into(),
                    position: Vec3d {
                        x: 96.5,
                        y: 64.0,
                        z: 0.5,
                    },
                    y_rot: 0.0,
                    x_rot: 0.0,
                    relatives_mask: 0,
                },
                &mut transition,
            ));
            let events = tick_until_noise_full(&mut world, Java26BuiltinDimension::End, (6, 0));
            assert!(events.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(495),
                    packet: ClientboundPlayPacket::LevelChunkWithLight(_),
                }
            )));
            let store = world.chunk_store_for_test(Java26BuiltinDimension::End);
            assert!(matches!(
                store.generation_state(6, 0),
                Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
            ));
            assert_eq!(
                store.column(6, 0).unwrap().get_noise_biome(0, 0, 0),
                Some(highlands)
            );
            let request = world
                .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
                .unwrap();
            assert!(request.chunk_writes.iter().any(|write| {
                write.dimension == Java26LevelKey::from(Java26BuiltinDimension::End)
                    && write.chunk_x == 6
                    && write.chunk_z == 0
                    && Java26ChunkData::parse(
                        write.document.clone(),
                        Java26ChunkContext {
                            expected_x: 6,
                            expected_z: 0,
                            min_section_y: 0,
                            section_count: 16,
                        },
                    )
                    .is_ok_and(|chunk| chunk.status() == Java26ChunkStatus::Full)
            }));
        }
    }

    let incompatible = end_noise_candidate_with_source_and_settings(
        true,
        NbtSummaryValue::string("example:alternate_end"),
        direct_multi_noise_biome_source(NbtSummaryValue::string("minecraft:plains")),
    );
    let incompatible_stem = incompatible
        .world_gen_settings
        .dimensions
        .get(&Java26LevelKey::from(Java26BuiltinDimension::End))
        .unwrap();
    assert!(crate::chunk::noise_generator::compile_noise_level_source(
        incompatible_stem,
        Arc::clone(&startup),
        0,
        "dimensions.minecraft:the_end.generator",
    )
    .is_ok());
    let world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup),
            ..ServerWorldConfig::default()
        },
        &incompatible,
    )
    .unwrap();
    assert!(world
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::End));
    assert!(!world
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_end"));
}

#[test]
fn alternate_nether_noise_profile_named_and_inline_reaches_full_without_fallback() {
    let (startup, inline) = alternate_nether_startup();
    for (index, settings) in [NbtSummaryValue::string("example:alternate_nether"), inline]
        .into_iter()
        .enumerate()
    {
        let candidate = nether_noise_candidate_with_settings(true, settings);
        let mut world = ServerWorld::restore(
            ServerWorldConfig {
                datapacks: Some(Arc::clone(&startup)),
                ..ServerWorldConfig::default()
            },
            &candidate,
        )
        .unwrap();
        assert!(world
            .served_builtin_dimensions()
            .contains(&Java26BuiltinDimension::Nether));
        assert!(!world
            .unserved_levels()
            .iter()
            .any(|(dimension, _)| *dimension == "minecraft:the_nether"));

        let connection_id = 295 + u64::try_from(index).unwrap();
        let (sender, receiver) = mailbox();
        join_noise_player(&sender, &receiver, &mut world, connection_id);
        acknowledge_noise_join(&sender, &receiver, &mut world, connection_id);
        let mut transition = Vec::new();
        assert!(world.teleport_player(
            ConnectionId(connection_id),
            crate::world::TeleportTransition {
                new_level: Java26BuiltinDimension::Nether.into(),
                position: Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: 0,
            },
            &mut transition,
        ));
        let events = tick_until_noise_full(&mut world, Java26BuiltinDimension::Nether, (0, 0));
        assert!(events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: recipient,
                packet: ClientboundPlayPacket::LevelChunkWithLight(_),
            } if *recipient == ConnectionId(connection_id)
        )));
        assert!(matches!(
            world
                .chunk_store_for_test(Java26BuiltinDimension::Nether)
                .generation_state(0, 0),
            Some(bbb_server_types::chunk_data::ChunkGenerationState::NoiseFull(_))
        ));
        let request = world
            .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
            .unwrap();
        assert!(request.chunk_writes.iter().any(|write| {
            write.dimension == Java26LevelKey::from(Java26BuiltinDimension::Nether)
                && Java26ChunkData::parse(
                    write.document.clone(),
                    Java26ChunkContext {
                        expected_x: write.chunk_x,
                        expected_z: write.chunk_z,
                        min_section_y: 0,
                        section_count: 16,
                    },
                )
                .is_ok_and(|chunk| chunk.status() == Java26ChunkStatus::Full)
        }));
    }
}

fn chunk_packets(tick: &TickResult, connection_id: u64) -> Vec<LevelChunkWithLight> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: recipient,
                packet: ClientboundPlayPacket::LevelChunkWithLight(chunk),
            } if *recipient == ConnectionId(connection_id) => Some((**chunk).clone()),
            _ => None,
        })
        .collect()
}

fn assert_same_column_cells(
    original: &bbb_server_types::chunk_data::ChunkColumn,
    restored: &bbb_server_types::chunk_data::ChunkColumn,
    chunk: (i32, i32),
) {
    assert_eq!(restored.min_y(), original.min_y());
    assert_eq!(restored.section_count(), original.section_count());
    assert_eq!(restored.air_state_id(), original.air_state_id());
    assert_eq!(restored.biome_registry_len(), original.biome_registry_len());
    assert_eq!(restored.block_ticks(), original.block_ticks());
    assert_eq!(restored.fluid_ticks(), original.fluid_ticks());
    let max_y = original.min_y() + i32::try_from(original.section_count()).unwrap() * 16;
    for y in original.min_y()..max_y {
        for z in 0..16 {
            for x in 0..16 {
                assert_eq!(
                    restored.get_block_state(x, y, z),
                    original.get_block_state(x, y, z),
                    "restored block differs at chunk {chunk:?} local [{x},{y},{z}]"
                );
            }
        }
    }
    for section in 0..original.section_count() {
        for y in 0..4 {
            for z in 0..4 {
                for x in 0..4 {
                    assert_eq!(
                        restored.section_biomes(section).get(x, y, z),
                        original.section_biomes(section).get(x, y, z),
                        "restored biome differs at chunk {chunk:?} section {section} local [{x},{y},{z}]"
                    );
                }
            }
        }
    }
}

#[test]
fn live_block_writes_wait_for_noise_full_and_survive_restart() {
    let dimension = Java26BuiltinDimension::Overworld;
    let root = std::env::temp_dir().join(format!("bbb-live-full-write-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let storage = Java26WorldStorage::open(
        WorldSavePaths::new(&root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    let candidate = noise_candidate();
    storage
        .create_world(
            &Java26CreatedSave {
                level: candidate.level.raw().clone(),
                world_gen_settings: candidate.world_gen_settings.document.clone(),
                chunks: Vec::new(),
            },
            RegionCompression::Zlib,
            99,
        )
        .unwrap();
    let baseline = storage.load_candidate().unwrap();
    let mut world = noise_world(&baseline);
    let mut chunks = ChunkStore::new_overworld_carvers_with_structure_generation(
        crate::chunk::noise_generator::overworld_program(true, 0),
        true,
    )
    .unwrap();
    chunks.try_load_column(1, 0).unwrap();
    assert!(!chunks.is_loaded(0, 0));
    assert!(!chunks.is_ticking_full(1, 0));
    world.install_chunk_store_for_test(dimension, chunks);
    world.queue_full_generation_for_test(dimension, (2, 0));
    let writes = [
        (
            BlockPos { x: 8, y: 200, z: 8 },
            block_state_id("minecraft:gold_block"),
        ),
        (
            BlockPos {
                x: 24,
                y: 200,
                z: 8,
            },
            block_state_id("minecraft:diamond_block"),
        ),
    ];
    let write = |world: &mut ServerWorld| {
        world.with_level_for_test(dimension, |level| {
            let written = writes
                .iter()
                .map(|&(pos, replacement)| {
                    level.set_block(
                        pos,
                        replacement,
                        SET_BLOCK_UPDATE_ALL,
                        crate::neighbor_updater::UPDATE_LIMIT,
                    ) && level.block_state(pos) == replacement
                })
                .collect::<Vec<_>>();
            written.iter().all(|&written| written)
        })
    };
    // Columns that are not FULL yet refuse the write and queue themselves.
    assert!(!write(&mut world));
    world.finish_generation_for_test();
    world.tick(Vec::new());
    assert!(write(&mut world));
    for &(pos, replacement) in &writes {
        let chunks = world.chunk_store_for_test(dimension);
        assert!(chunks.is_ticking_full(pos.x >> 4, pos.z >> 4));
        assert_eq!(
            chunks.get_block_state(pos.x, pos.y, pos.z),
            Some(replacement)
        );
    }
    let request = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&baseline, &request).unwrap();
    let mut restored = noise_world(&storage.load_candidate().unwrap());
    for &(pos, replacement) in &writes {
        assert_eq!(restored.block_state(dimension, pos), Some(replacement));
    }
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();

    let flat = ChunkStore::new(default_overworld_level_source().unwrap(), 0);
    assert!(!flat.is_loaded(0, 0));
    assert!(!flat.is_ticking_full(0, 0));
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    let replacement = block_state_id("minecraft:stone");
    world.install_chunk_store_for_test(dimension, flat);
    assert!(world.set_block(dimension, pos, replacement, SET_BLOCK_UPDATE_ALL));
    assert!(world.chunk_store_for_test(dimension).is_ticking_full(0, 0));
    assert_eq!(
        world
            .chunk_store_for_test(dimension)
            .get_block_state(pos.x, pos.y, pos.z),
        Some(replacement)
    );
}

#[test]
fn served_overworld_noise_join_sends_full_heightmaps_and_light() {
    let mut world = noise_world(&noise_candidate());
    let (sender, receiver) = mailbox();
    let joined = join_noise_player(&sender, &receiver, &mut world, 97);
    assert!(
        !joined
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })),
        "{joined:?}"
    );
    let login = joined
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Login(login),
                ..
            } => Some(login),
            _ => None,
        })
        .unwrap();
    assert_eq!(login.common_spawn_info.sea_level, 63);

    let chunks = joined
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::LevelChunkWithLight(chunk),
                ..
            } => Some(chunk),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        joined
            .events
            .iter()
            .filter(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::ChunkBatchStart,
                    ..
                }
            ))
            .count(),
        1
    );
    assert!(joined.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ChunkBatchFinished { batch_size: 9 },
            ..
        }
    )));
    assert_eq!(chunks.len(), 9);
    let spawn = world.publication().players[0].position;
    let center = (
        (spawn.x.floor() as i32).div_euclid(16),
        (spawn.z.floor() as i32).div_euclid(16),
    );
    let expected_chunks = vec![
        center,
        (center.0 - 1, center.1),
        (center.0, center.1 - 1),
        (center.0, center.1 + 1),
        (center.0 + 1, center.1),
        (center.0 - 1, center.1 - 1),
        (center.0 - 1, center.1 + 1),
        (center.0 + 1, center.1 - 1),
        (center.0 + 1, center.1 + 1),
    ];
    assert_eq!(
        chunks
            .iter()
            .map(|chunk| (chunk.x, chunk.z))
            .collect::<Vec<_>>(),
        expected_chunks
    );
    // `DistanceManager.getPlayerTicketLevel` (`DistanceManager.java:131-133`):
    // a simulation distance of 2 entity-ticks the 5x5 square and block-ticks
    // one ring further, all inside the columns the 2-chunk view loads FULL.
    let square = |radius: i32| {
        (center.0 - radius..=center.0 + radius)
            .flat_map(|x| (center.1 - radius..=center.1 + radius).map(move |z| (x, z)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        world.entity_ticking_chunks(Java26BuiltinDimension::Overworld),
        square(2)
    );
    assert_eq!(
        world.block_ticking_chunks(Java26BuiltinDimension::Overworld),
        square(3)
    );
    for chunk in chunks {
        assert_eq!(
            chunk
                .chunk_data
                .heightmaps
                .iter()
                .map(|heightmap| heightmap.kind_id)
                .collect::<Vec<_>>(),
            vec![
                bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
                bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
            ]
        );
        assert!(chunk
            .chunk_data
            .heightmaps
            .iter()
            .all(|heightmap| heightmap.data.len() == 37));
        let (id, payload) = encode_play_clientbound_level_chunk_with_light(chunk).unwrap();
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::LevelChunkWithLight((**chunk).clone())
        );
    }
}

/// Login and the player's own column do not wait for the view area, which
/// streams in once its batch commits and matches what a blocking tick builds.
#[test]
fn background_generation_streams_the_view_area_after_login() {
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(crate::chunk::noise_generator::overworld_startup(true)),
            background_generation: true,
            ..ServerWorldConfig::default()
        },
        &noise_candidate(),
    )
    .unwrap();
    // Frozen, neither world ticks blocks between generation and comparison.
    world.tick_rate_mut().set_frozen(true);
    let (sender, receiver) = mailbox();
    let joined = join_noise_player(&sender, &receiver, &mut world, 98);
    let sent = |events: &[WorldEvent]| {
        events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::LevelChunkWithLight(chunk),
                    ..
                } => Some((chunk.x, chunk.z)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert!(joined.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Login(_),
            ..
        }
    )));
    let spawn = world.publication().players[0].position;
    let center = (
        (spawn.x.floor() as i32).div_euclid(16),
        (spawn.z.floor() as i32).div_euclid(16),
    );
    let mut delivered = sent(&joined.events);
    assert_eq!(delivered.first(), Some(&center));

    world.finish_generation_for_test();
    let streamed = play(
        &sender,
        &receiver,
        &mut world,
        98,
        PlayServerbound::ChunkBatchReceived {
            desired_chunks_per_tick: 9.0,
        },
    );
    delivered.extend(sent(&streamed.events));
    let square = (center.0 - 1..=center.0 + 1)
        .flat_map(|x| (center.1 - 1..=center.1 + 1).map(move |z| (x, z)))
        .collect::<Vec<_>>();
    assert!(
        square.iter().all(|chunk| delivered.contains(chunk)),
        "{delivered:?}"
    );

    let mut blocking = noise_world(&noise_candidate());
    blocking.tick_rate_mut().set_frozen(true);
    join_noise_player(&sender, &receiver, &mut blocking, 98);
    for &(x, z) in &square {
        assert_eq!(
            world
                .chunk_store_for_test(Java26BuiltinDimension::Overworld)
                .column(x, z),
            blocking
                .chunk_store_for_test(Java26BuiltinDimension::Overworld)
                .column(x, z)
        );
    }
}

pub(super) fn end_spike_worldgen_entity_fixture() -> (
    ServerWorld,
    (i32, i32),
    bbb_server_persistence::Java26EntityChunkData,
) {
    let startup = crate::chunk::noise_generator::overworld_startup(true);
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(startup),
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let spike =
        bbb_server_worldgen::end_spike::spikes_for_seed(bbb_server_persistence::CREATED_WORLD_SEED)
            [0];
    let chunk = (spike.center_x.div_euclid(16), spike.center_z.div_euclid(16));
    world.ensure_entity_transfer_full_chunk_for_test(Java26BuiltinDimension::End, chunk);
    let records = world
        .chunk_store_for_test(Java26BuiltinDimension::End)
        .published_worldgen_entity_records(chunk.0, chunk.1)
        .to_vec();
    let document = ServerWorld::parse_worldgen_entity_records_for_test(chunk, records).unwrap();
    assert_eq!(document.entities().len(), 1);
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:end_crystal"
    );
    (world, chunk, document)
}

/// Teleports the session into `chunk` of the end, answering the events the
/// teleport itself sent.
fn move_test_player_to_end_chunk(
    world: &mut ServerWorld,
    connection_id: u64,
    chunk: (i32, i32),
) -> Vec<WorldEvent> {
    let mut transition = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(connection_id),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::End.into(),
            position: Vec3d {
                x: f64::from(chunk.0 * 16 + 8) + 0.5,
                y: 80.0,
                z: f64::from(chunk.1 * 16 + 8) + 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut transition,
    ));
    world.reset_pending_chunks_for_test(ConnectionId(connection_id), &[chunk]);
    transition
}

#[test]
fn full_chunk_batch_imports_worldgen_entities_before_tracking_once() {
    let (mut world, chunk, document) = end_spike_worldgen_entity_fixture();
    let record = document.entities().next().unwrap();
    let uuid = record.uuid();
    let position = record.position();
    let (sender, receiver) = mailbox();
    join_noise_player(&sender, &receiver, &mut world, 901);
    let moved = move_test_player_to_end_chunk(&mut world, 901, chunk);

    let packets = end_crystal_pairing(&mut world, &sender, &receiver, 901, uuid, moved);
    assert!(packets.iter().any(|packet| {
        matches!(packet, ClientboundPlayPacket::LevelChunkWithLight(packet)
            if (packet.x, packet.z) == chunk)
    }));
    assert_eq!(
        packets
            .iter()
            .filter(|packet| is_crystal_add(packet, uuid))
            .count(),
        1
    );
    let entity_id = world
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::End, uuid)
        .unwrap();
    let entity = world
        .stored_entity_for_test(Java26BuiltinDimension::End, entity_id)
        .unwrap();
    assert_eq!(entity.uuid(), uuid);
    assert_eq!(
        entity.position(),
        Vec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        }
    );
    assert!(matches!(
        entity.data(),
        crate::entity::EntityData::EndCrystal(data) if data.shows_bottom()
    ));
    assert!(world
        .chunk_store_for_test(Java26BuiltinDimension::End)
        .published_worldgen_entity_records(chunk.0, chunk.1)
        .is_empty());

    // A second session receives the same crystal once; it is not imported a
    // second time.
    join_noise_player(&sender, &receiver, &mut world, 902);
    let moved = move_test_player_to_end_chunk(&mut world, 902, chunk);
    let packets = end_crystal_pairing(&mut world, &sender, &receiver, 902, uuid, moved);
    assert_eq!(
        packets
            .iter()
            .filter(|packet| is_crystal_add(packet, uuid))
            .count(),
        1
    );
    let copies = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::End)
        .into_iter()
        .filter(|&id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::End, id)
                .is_some_and(|entity| entity.uuid() == uuid)
        })
        .count();
    assert_eq!(copies, 1);
}

fn is_crystal_add(packet: &ClientboundPlayPacket, uuid: Uuid) -> bool {
    matches!(packet, ClientboundPlayPacket::AddEntity(add) if add.uuid == uuid)
}

/// Every packet `connection_id` receives, from the `moved` teleport events and
/// the chunk batch that follows them, over the ticks it takes to be paired
/// with the end crystal `uuid`, plus one more tick to catch a duplicate.
/// The session acknowledges its batches and sends a move packet each tick,
/// which is where `ChunkMap.move` pairs it with entities in sent chunks.
fn end_crystal_pairing(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    connection_id: u64,
    uuid: Uuid,
    moved: Vec<WorldEvent>,
) -> Vec<ClientboundPlayPacket> {
    let to_session = |event: WorldEvent| match event {
        WorldEvent::Send {
            connection_id: ConnectionId(id),
            packet,
        } if id == connection_id => Some(packet),
        _ => None,
    };
    let mut packets: Vec<_> = moved
        .into_iter()
        .chain(world.tick_chunk_batches_for_test())
        .filter_map(to_session)
        .collect();
    // Moves count only once the client has loaded the new level and confirmed
    // every teleport it was sent.
    let mut answers = vec![PlayServerbound::PlayerLoaded];
    let mut answered = 0;
    for _ in 0..20 {
        let paired = packets.iter().any(|packet| is_crystal_add(packet, uuid));
        answers.extend(
            packets[answered..]
                .iter()
                .filter_map(|packet| match packet {
                    ClientboundPlayPacket::PlayerPosition(update) => {
                        Some(PlayServerbound::AcceptTeleportation { id: update.id })
                    }
                    _ => None,
                }),
        );
        answered = packets.len();
        answers.push(PlayServerbound::ChunkBatchReceived {
            desired_chunks_per_tick: 64.0,
        });
        for packet in answers.drain(..) {
            enqueue(
                sender,
                WorldCommand::Play {
                    connection_id: ConnectionId(connection_id),
                    packet,
                },
            )
            .unwrap();
        }
        let tick = play(
            sender,
            receiver,
            world,
            connection_id,
            PlayServerbound::MovePlayer(MovePlayer {
                position: None,
                rotation: None,
                on_ground: false,
                horizontal_collision: false,
            }),
        );
        packets.extend(tick.events.into_iter().filter_map(to_session));
        if paired {
            return packets;
        }
    }
    panic!("{connection_id} was never paired with the crystal {uuid}");
}
