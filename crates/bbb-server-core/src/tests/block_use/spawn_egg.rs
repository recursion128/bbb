//! `SpawnEggItem.useOn`'s `Spawner` arm (`SpawnEggItem.java:52-71`), which
//! the spawner and the trial spawner both implement.

use super::*;

use crate::connection::ConnectionId;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::packets::ItemStackSummary;

fn nbt(name: &str, value: NbtSummaryValue) -> bbb_protocol::NbtSummaryEntry {
    bbb_protocol::NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn spawn_egg_stack(name: &str, count: i32) -> ItemStackSummary {
    let item_id = bbb_protocol::item_types::vanilla_item_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("missing built-in item {name}"))
        .id;
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        ..ItemStackSummary::empty()
    }
}

/// The egg goes into the selected hotbar slot, which is inventory slot 0.
fn hold_spawn_eggs(world: &mut ServerWorld, name: &str, count: i32) {
    world.set_inventory_slot_for_test(ConnectionId(1), 0, spawn_egg_stack(name, count));
}

fn spawner_entity_type_id(world: &mut ServerWorld, at: BlockPos) -> Option<String> {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.spawner())
        .expect("the spawner block entity is live")
        .spawner_data()
        .next_spawn_data()
        .and_then(|data| {
            data.entity()
                .iter()
                .find(|entry| entry.name == "id")
                .and_then(|entry| match &entry.value {
                    NbtSummaryValue::String(id) => Some(id.try_to_string().unwrap()),
                    _ => None,
                })
        })
}

#[test]
fn a_spawn_egg_on_a_spawner_sets_its_entity_type_and_consumes_one() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        block_state_id("minecraft:spawner"),
        SET_BLOCK_UPDATE_ALL,
    ));
    hold_spawn_eggs(&mut world, "minecraft:zombie_spawn_egg", 3);
    // `ItemStack.consume` keeps the source for infinite materials
    // (`ServerPlayerGameMode.java:389-393`), so the consume is read in survival.
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );

    let clicked = click_use(&sender, &receiver, &mut world, at, 0.5, 1);

    // `:65`: `spawnerHolder.setEntityId(type, level.getRandom())`.
    assert_eq!(
        spawner_entity_type_id(&mut world, at),
        Some("minecraft:zombie".to_string())
    );
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 2);
    // `:67`: `gameEvent(player, GameEvent.BLOCK_CHANGE, pos)`.
    let (game_events, count) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(count, 1);
    assert_eq!(game_events[0].game_event, VanillaGameEvent::BLOCK_CHANGE);
    assert_eq!(game_events[0].pos, at);
    assert_eq!(
        game_events[0].source_entity_id,
        Some(entity_id_of(&world, 1))
    );
    // `:66`: `sendBlockUpdated` reaches the sessions tracking the column,
    // on the tick whose `broadcast_changed_blocks` reads the queue.
    let published = world.tick(receiver.drain());
    assert_eq!(
        block_update_sent(&clicked, at).or_else(|| block_update_sent(&published, at)),
        Some(block_state_id("minecraft:spawner"))
    );
    // `SpawnerBlockEntity.setEntityId`'s `setChanged` owns the record from
    // here on, so the save writes the id the egg set.
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let created = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the spawner's column is written");
    let parsed = Java26ChunkData::parse(
        created.document.clone(),
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .expect("the runtime's own document is a valid Java 26.1 chunk");
    let record = parsed
        .block_entities()
        .iter()
        .find(|record| {
            field(record, "x") == &NbtSummaryValue::Int(at.x)
                && field(record, "y") == &NbtSummaryValue::Int(at.y)
                && field(record, "z") == &NbtSummaryValue::Int(at.z)
        })
        .expect("the spawner's record is written");
    let NbtSummaryValue::Compound(spawn_data) = field(record, "SpawnData") else {
        panic!("SpawnData is a compound: {record:?}");
    };
    let NbtSummaryValue::Compound(entity) = field(spawn_data, "entity") else {
        panic!("the spawn data carries an entity compound: {spawn_data:?}");
    };
    assert_eq!(
        field(entity, "id"),
        &NbtSummaryValue::string("minecraft:zombie")
    );
}

#[test]
fn a_spawn_egg_on_a_trial_spawner_overrides_both_configs_and_returns_to_inactive() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let active = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:trial_spawner",
        &std::collections::BTreeMap::from([
            ("ominous".to_string(), "true".to_string()),
            ("trial_spawner_state".to_string(), "active".to_string()),
        ]),
    )
    .expect("minecraft:trial_spawner has that state");
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        active,
        SET_BLOCK_UPDATE_ALL,
    ));
    hold_spawn_eggs(&mut world, "minecraft:zombie_spawn_egg", 3);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );

    click_use(&sender, &receiver, &mut world, at, 0.5, 1);

    // `TrialSpawner.overrideEntityToSpawn` (`TrialSpawner.java:340-344`).
    let trial_spawner = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.trial_spawner())
        .expect("the trial spawner block entity is live");
    for ominous in [false, true] {
        let config = trial_spawner.config().active(ominous);
        assert_eq!(config.spawn_potentials().len(), 1);
        assert_eq!(
            config.spawn_potentials()[0].data.entity_resource_id(),
            Some(&bbb_protocol::JavaString::from("minecraft:zombie"))
        );
    }
    assert!(trial_spawner.state_data().registered_players().is_empty());

    // The `setState(level, INACTIVE)` half writes the block state property,
    // and the ticker `TrialSpawnerBlock.getTicker` returns
    // (`TrialSpawnerBlock.java:46-59`) runs later in the same tick, so the
    // fresh override is already one state on when the use returns.
    let identity = bbb_protocol::block_state_types::vanilla_block_state_identity(
        live_block(&mut world, at).expect("the trial spawner is still there"),
    )
    .expect("the block state is a built-in identity");
    assert_eq!(
        identity
            .properties
            .get("trial_spawner_state")
            .map(String::as_str),
        Some("waiting_for_players")
    );
    // The override leaves the `ominous` property alone
    // (`TrialSpawner.overrideEntityToSpawn` only writes the state).
    assert_eq!(
        identity.properties.get("ominous").map(String::as_str),
        Some("true")
    );
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 2);
}

#[test]
fn a_spawn_egg_on_a_spawner_whose_potentials_are_set_draws_from_them() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        block_state_id("minecraft:spawner"),
        SET_BLOCK_UPDATE_ALL,
    ));
    // `getOrCreateNextSpawnData` (`BaseSpawner.java:257-263`) takes the entry
    // only when no `SpawnData` is set yet.
    let spawner = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::spawner_mut)
        .expect("the spawner block entity is live");
    let data = spawner.spawner_data_mut();
    data.set_spawn_potentials(vec![crate::entity::WeightedSpawnerSpawnData {
        data: crate::entity::SpawnerSpawnData::new(
            vec![nbt("id", NbtSummaryValue::string("minecraft:pig"))],
            None,
            None,
        ),
        weight: 1,
    }]);
    hold_spawn_eggs(&mut world, "minecraft:zombie_spawn_egg", 1);

    click_use(&sender, &receiver, &mut world, at, 0.5, 1);

    // The drawn potential's other fields survive; only `id` is overwritten.
    let spawner = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::spawner_mut)
        .expect("the spawner block entity is live");
    let next = spawner
        .spawner_data()
        .next_spawn_data()
        .expect("the draw set the next spawn data");
    assert_eq!(
        next.entity_resource_id(),
        Some(&bbb_protocol::JavaString::from("minecraft:zombie")),
        "{next:?}"
    );
}

#[test]
fn a_spawn_egg_on_a_spawner_with_the_rule_off_only_warns() {
    let (sender, receiver) = mailbox();
    // `/gamerule` needs the operator level the rule command asks for.
    let mut world = spawn_pinned_world(ServerWorldConfig {
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        block_state_id("minecraft:spawner"),
        SET_BLOCK_UPDATE_ALL,
    ));
    hold_spawn_eggs(&mut world, "minecraft:zombie_spawn_egg", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule spawner_blocks_work false",
    );

    let ticked = click_use(&sender, &receiver, &mut world, at, 0.5, 1);

    assert_eq!(spawner_entity_type_id(&mut world, at), None);
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 1);
    assert_eq!(
        system_chats(&ticked),
        vec![(ConnectionId(1), "advMode.notEnabled.spawner".to_string())]
    );
    let (game_events, count) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(count, 0, "{game_events:?}");
}

/// `:97`: `ItemStack.consume` keeps the source for infinite materials
/// (`ServerPlayerGameMode.java:389-393`), so the creative fixture uses the egg
/// without spending it.
#[test]
fn a_creative_use_spends_no_egg() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    hold_spawn_eggs(&mut world, "minecraft:zombie_spawn_egg", 1);

    click_use(&sender, &receiver, &mut world, at, 0.5, 1);

    assert_eq!(
        live_block(&mut world, at),
        Some(block_state_id("minecraft:stone"))
    );
    assert!(spawned_entity(&world, VANILLA_ENTITY_TYPE_ZOMBIE_ID).is_some());
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 1);
}

/// `click_use` with the clicked face under test.
fn click_use_face(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    pos: BlockPos,
    direction: Direction,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos,
                direction,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        }),
    )
}

/// The one live entity of the level with that type, as `(position, yaw)`.
fn spawned_entity(world: &ServerWorld, entity_type_id: i32) -> Option<(Vec3d, f32)> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find_map(|id| {
            let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            (entity.entity_type_id() == entity_type_id).then(|| (entity.position(), entity.y_rot()))
        })
}

/// `SpawnEggItem.useOn`'s non-spawner arm (`:72-80`): a block whose collision
/// shape is empty takes the clicked cell, and the entity walks down onto the
/// floor below it (`EntityType.getYOffset`, `EntityType.java:1354-1362`).
#[test]
fn a_spawn_egg_on_a_replaceable_block_spawns_in_that_cell() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        block_state_id("minecraft:short_grass"),
        SET_BLOCK_UPDATE_ALL,
    ));
    hold_spawn_eggs(&mut world, "minecraft:pig_spawn_egg", 3);

    click_use_face(&sender, &receiver, &mut world, at, Direction::Up, 1);

    let (position, _) =
        spawned_entity(&world, VANILLA_ENTITY_TYPE_PIG_ID).expect("the egg spawned its pig");
    // `setPos(x + 0.5, y + 1, z + 0.5)` then the walk onto the floor at y = 63.
    assert_eq!(
        position,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 1.5,
        }
    );
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 2);
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(game_events
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::ENTITY_PLACE && event.pos == at));
}

/// `EntityType.spawn` plays the new mob's ambient sound at once
/// (`EntityType.java:1310-1312`).
#[test]
fn a_spawned_mob_plays_its_ambient_sound() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 63, z: 1 };
    hold_spawn_eggs(&mut world, "minecraft:sheep_spawn_egg", 1);

    let tick = click_use_face(&sender, &receiver, &mut world, at, Direction::Up, 1);

    let ambient = vanilla_sound_event_registry_id("minecraft:entity.sheep.ambient").unwrap();
    assert!(tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::Sound(sound),
        } if sound.sound == SoundEventHolder::Reference { registry_id: ambient }
            && sound.source == SoundSource::Neutral)));
}

/// The clicked cell of a full block is taken, so the egg spawns in the cell the
/// clicked face points at; the top face also sets `movedUp` (`:74-80`).
#[test]
fn a_spawn_egg_on_a_solid_block_uses_the_face_cell() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    hold_spawn_eggs(&mut world, "minecraft:cow_spawn_egg", 2);

    click_use_face(&sender, &receiver, &mut world, at, Direction::Up, 1);
    let (position, _) =
        spawned_entity(&world, VANILLA_ENTITY_TYPE_COW_ID).expect("the egg spawned its cow");
    // `spawnPos = at.above()`, `movedUp` sweeps -2 from y = 66 onto the stone.
    assert_eq!(
        position,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 1.5,
        }
    );
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 1);

    // A side face spawns beside the block without `movedUp`.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    hold_spawn_eggs(&mut world, "minecraft:cow_spawn_egg", 2);
    click_use_face(&sender, &receiver, &mut world, at, Direction::South, 1);
    let (position, _) =
        spawned_entity(&world, VANILLA_ENTITY_TYPE_COW_ID).expect("the side face spawned its cow");
    assert_eq!(
        position,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 2.5,
        }
    );
}

/// `:89-91`: a monster egg in peaceful refuses without consuming.
#[test]
fn a_monster_egg_refuses_in_peaceful() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        difficulty: Difficulty::Peaceful,
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        block_state_id("minecraft:short_grass"),
        SET_BLOCK_UPDATE_ALL,
    ));
    hold_spawn_eggs(&mut world, "minecraft:zombie_spawn_egg", 3);

    click_use_face(&sender, &receiver, &mut world, at, Direction::Up, 1);

    assert!(spawned_entity(&world, VANILLA_ENTITY_TYPE_ZOMBIE_ID).is_none());
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 3);
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(game_events
        .iter()
        .all(|event| event.game_event != VanillaGameEvent::ENTITY_PLACE));
}

/// `SpawnEggItem.getType` (`:146-149`): the egg's `minecraft:entity_data`
/// names the type, and its tag merges onto the fresh entity.
#[test]
fn the_entity_data_component_picks_the_type_and_applies_its_tag() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        block_state_id("minecraft:short_grass"),
        SET_BLOCK_UPDATE_ALL,
    ));
    let mut stack = spawn_egg_stack("minecraft:pig_spawn_egg", 2);
    stack.component_patch.added_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID];
    stack.component_patch.entity_data_entity_type_id = Some(VANILLA_ENTITY_TYPE_COW_ID);
    stack.component_patch.entity_data_tag = Some(NbtSummaryValue::Compound(vec![nbt(
        "CustomName",
        NbtSummaryValue::string("Moolissa"),
    )]));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack);

    click_use_face(&sender, &receiver, &mut world, at, Direction::Up, 1);

    assert!(spawned_entity(&world, VANILLA_ENTITY_TYPE_PIG_ID).is_none());
    let cow = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_COW_ID)
        })
        .expect("the component's type was spawned");
    assert_eq!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow)
            .and_then(crate::entity::Entity::custom_name)
            .map(|name| name.text().to_string()),
        Some("Moolissa".to_string()),
        "the tag's custom name reached the entity"
    );
}

/// `EntityType.spawn` (`EntityType.java:1296-1312`) creates any type, so an egg
/// whose `entity_data` names primed TNT spawns it.
#[test]
fn an_egg_naming_a_non_mob_type_spawns_it() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 63, z: 1 };
    let mut stack = spawn_egg_stack("minecraft:pig_spawn_egg", 2);
    stack.component_patch.added_type_ids =
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID];
    stack.component_patch.entity_data_entity_type_id =
        Some(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_ID);
    stack.component_patch.entity_data_tag = Some(NbtSummaryValue::Compound(vec![nbt(
        "fuse",
        NbtSummaryValue::Short(200),
    )]));
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack);

    click_use_face(&sender, &receiver, &mut world, at, Direction::Up, 1);

    let (position, _) = spawned_entity(
        &world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_ID,
    )
    .expect("the egg spawned primed TNT");
    assert_eq!(position.y, 64.0);
}
