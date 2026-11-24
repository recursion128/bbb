//! `BrushItem` (`BrushItem.java:38-100`) on suspicious sand and gravel through
//! real `UseItemOn` and look packets, and the `BrushableBlockEntity`
//! (`BrushableBlockEntity.java:59-168`) its strokes drive.

use crate::block_entity::brushable::BrushableBlockEntity;
use crate::block_entity::BlockEntity;
use crate::entity::warden_spawn_tracker::WardenSpawnTracker;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID;

use super::*;

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
/// Eye at y = 65.62, looking down +z at the north face of the block.
const ALICE_AT: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};
const BOB_AT: Vec3d = Vec3d {
    x: 3.5,
    y: 64.0,
    z: 0.5,
};
const BRUSHED: BlockPos = BlockPos { x: 0, y: 65, z: 4 };
const SUPPORT: BlockPos = BlockPos { x: 0, y: 64, z: 4 };
/// `pos.relative(Direction.NORTH)`, the face the ray hits.
const DROP_CELL: BlockPos = BlockPos { x: 0, y: 65, z: 3 };
const DESERT_PYRAMID: &str = "minecraft:archaeology/desert_pyramid";
/// A nonzero seed, so the table draws from a stream of its own.
const DESERT_PYRAMID_SEED: i64 = -5;

fn state(block: &str, dusted: Option<i32>) -> i32 {
    let properties = dusted
        .map(|dusted| BTreeMap::from([("dusted".to_string(), dusted.to_string())]))
        .unwrap_or_default();
    vanilla_block_state_id_by_name_and_properties(block, &properties)
        .unwrap_or_else(|| panic!("the built-in state {block} {properties:?}"))
}

fn stage_block(world: &mut ServerWorld, block: &str) {
    for (pos, state) in [
        (SUPPORT, state("minecraft:stone", None)),
        (BRUSHED, state(block, Some(0))),
    ] {
        assert!(world.set_block(OVERWORLD, pos, state, SET_BLOCK_UPDATE_ALL));
    }
}

fn brushable(world: &ServerWorld) -> Option<&BrushableBlockEntity> {
    world
        .get_block_entity(OVERWORLD, BRUSHED)
        .and_then(BlockEntity::brushable)
}

fn brushable_mut(world: &mut ServerWorld) -> &mut BrushableBlockEntity {
    world
        .level_block_entity_for_test(OVERWORLD, BRUSHED)
        .and_then(BlockEntity::brushable_mut)
        .expect("the staged block has its brushable block entity")
}

fn look(y_rot: f32, x_rot: f32) -> PlayServerbound {
    PlayServerbound::MovePlayer(MovePlayer {
        position: None,
        rotation: Some(PlayerRotation { y_rot, x_rot }),
        on_ground: true,
        horizontal_collision: false,
    })
}

fn brush_on_brushed_block() -> PlayServerbound {
    PlayServerbound::UseItemOn(UseItemOn {
        hand: InteractionHand::MainHand,
        hit: BlockHitResult {
            pos: BRUSHED,
            direction: Direction::North,
            cursor_x: 0.5,
            cursor_y: 0.62,
            cursor_z: 0.0,
            inside: false,
            world_border_hit: false,
        },
        sequence: 1,
    })
}

/// Alice holds a brush in front of `block` on its support; Bob watches from
/// the side.
fn brushing_fixture(block: &str) -> Fixture {
    brushing_fixture_with_config(block, datapack_config())
}

fn brushing_fixture_with_config(block: &str, config: ServerWorldConfig) -> Fixture {
    let mut fixture = Fixture::with_config(GameType::Survival, config);
    fixture
        .world
        .set_player_position_for_test(ConnectionId(ALICE), ALICE_AT);
    fixture
        .world
        .set_player_position_for_test(ConnectionId(BOB), BOB_AT);
    fixture.play(ALICE, look(0.0, 0.0));
    stage_block(&mut fixture.world, block);
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:brush", 1));
    // `BrushableBlock.onPlace`'s delay-2 tick runs before the brush arrives.
    fixture.idle();
    fixture.idle();
    fixture
}

/// `BrushItem.onUseTick`'s `timeElapsed` the last tick ran at: the use tick
/// is 1, and each later tick counts `useItemRemaining` down from 199.
fn elapsed(fixture: &Fixture) -> i32 {
    200 - fixture.use_state().2
}

/// Idles to the tick whose `timeElapsed` is `target`, asserting that only
/// the `timeElapsed % 10 == 5` ticks on the way brushed, and answers that tick.
fn idle_to(fixture: &mut Fixture, target: i32) -> TickResult {
    assert!(elapsed(fixture) < target);
    loop {
        let tick = fixture.idle();
        if elapsed(fixture) == target {
            return tick;
        }
        assert_eq!(
            sounds(&tick, BOB).is_empty(),
            elapsed(fixture) % 10 != 5,
            "elapsed {}",
            elapsed(fixture)
        );
    }
}

fn dusted(world: &mut ServerWorld) -> Option<i32> {
    let current = world.block_state(OVERWORLD, BRUSHED)?;
    (0..4).find(|dusted| {
        ["minecraft:suspicious_sand", "minecraft:suspicious_gravel"]
            .into_iter()
            .any(|block| state(block, Some(*dusted)) == current)
    })
}

/// Every block state `viewer` was sent for [`BRUSHED`].
fn brushed_states_sent(tick: &TickResult, viewer: u64) -> Vec<i32> {
    packets_to(tick, viewer)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::BlockUpdate(update) if update.pos == BRUSHED => {
                Some(update.block_state_id)
            }
            _ => None,
        })
        .collect()
}

fn level_events(tick: &TickResult, viewer: u64) -> Vec<(i32, BlockPos, i32)> {
    packets_to(tick, viewer)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::LevelEvent(event) => {
                Some((event.event_type, event.pos, event.data))
            }
            _ => None,
        })
        .collect()
}

/// The overworld's item entities as `(item, count, position)`.
fn ground_items(world: &ServerWorld) -> Vec<(String, i32, Vec3d)> {
    world
        .stored_entity_ids_for_test(OVERWORLD)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(OVERWORLD, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            let stack = item.item();
            Some((
                stack.item_resource_id()?.to_string(),
                stack.count(),
                entity.position(),
            ))
        })
        .collect()
}

fn brush_damage(fixture: &Fixture) -> Option<i32> {
    fixture.slot(HOTBAR_SLOT).component_patch.damage
}

/// The `minecraft:adventure/salvage_sherd` criteria `connection_id` holds.
fn salvage_criteria(world: &ServerWorld, connection_id: u64) -> Vec<bbb_protocol::JavaString> {
    world
        .player_advancements_to_data(ConnectionId(connection_id))
        .progress
        .get("minecraft:adventure/salvage_sherd")
        .map(|progress| {
            progress
                .criteria()
                .iter()
                .filter(|(_, criterion)| criterion.is_done())
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// A stroke every ten ticks from the fifth: the first sets `dusted` 1, the
/// third 2, the sixth 3, and the tenth drops the revealed item on the hit
/// face's side, plays the completion event and leaves plain sand, wearing the
/// brush by one.
#[test]
fn ten_strokes_reveal_the_item_and_leave_sand() {
    let mut fixture = brushing_fixture("minecraft:suspicious_sand");
    brushable_mut(&mut fixture.world)
        .set_item(crate::tests::native_stack(stack("minecraft:diamond", 1)));
    let alice = fixture.alice;

    let started = fixture.play(ALICE, brush_on_brushed_block());
    assert_eq!(living_flags(&started, BOB, alice), [1]);
    assert_eq!(elapsed(&fixture), 1);
    assert_eq!(fixture.item_used("minecraft:brush"), 1);
    assert!(sounds(&started, BOB).is_empty());

    let mut progress = Vec::new();
    for stroke in 1..10 {
        let tick = idle_to(&mut fixture, stroke * 10 - 5);
        assert_eq!(
            sounds(&tick, BOB),
            ["minecraft:item.brush.brushing.sand"],
            "stroke {stroke} plays for everyone but the brusher"
        );
        assert!(sounds(&tick, ALICE).is_empty());
        progress.push(dusted(&mut fixture.world));
        // The stroke runs after this tick's chunk broadcast, so the next one
        // carries its block change, and with it the block entity's update tag.
        let next = fixture.idle();
        let sent = brushed_states_sent(&next, BOB);
        let tagged = packets_to(&next, BOB).any(|packet| {
            matches!(packet, ClientboundPlayPacket::BlockEntityData(update) if update.pos == BRUSHED)
        });
        if matches!(stroke, 1 | 3 | 6) {
            assert_eq!(
                sent,
                [state(
                    "minecraft:suspicious_sand",
                    dusted(&mut fixture.world)
                )]
            );
            assert!(tagged, "stroke {stroke} republishes the block entity");
        } else {
            assert!(sent.is_empty(), "stroke {stroke} leaves the state alone");
            assert!(!tagged, "stroke {stroke} changes no block");
        }
    }
    assert_eq!(
        progress,
        [1, 1, 2, 2, 2, 3, 3, 3, 3].map(Some),
        "`getCompletionState` over brush counts 1..9"
    );
    assert_eq!(
        brush_damage(&fixture),
        None,
        "no stroke before the last wears it"
    );
    assert_eq!(
        brushable(&fixture.world).map(BrushableBlockEntity::hit_direction),
        Some(Some(Direction::North))
    );

    let completed = idle_to(&mut fixture, 95);
    assert_eq!(
        sounds(&completed, BOB),
        ["minecraft:item.brush.brushing.sand"]
    );
    assert_eq!(
        level_events(&completed, ALICE),
        [(3008, BRUSHED, state("minecraft:suspicious_sand", Some(3)))]
    );
    assert_eq!(
        fixture.world.block_state(OVERWORLD, BRUSHED),
        Some(state("minecraft:sand", None))
    );
    assert!(brushable(&fixture.world).is_none());
    assert_eq!(brush_damage(&fixture), Some(1));
    let items = ground_items(&fixture.world);
    assert_eq!(items.len(), 1);
    let (item, count, position) = &items[0];
    assert_eq!((item.as_str(), *count), ("minecraft:diamond", 1));
    // `dropContent` centres the item in the north neighbour at
    // `y + 0.5 + height / 2` with no motion, so only gravity moves it.
    assert_eq!(
        (position.x, position.z),
        (0.5, f64::from(DROP_CELL.z) + 0.5)
    );
    assert!(position.y <= 65.625 && position.y > 64.0);

    // The item falls through the line of sight without stopping the brush:
    // `ItemEntity` is not pickable. Sand is no `BrushableBlock`, so the next
    // stroke only plays the generic sound.
    let generic = idle_to(&mut fixture, 105);
    assert!(fixture.use_state().0);
    assert_eq!(
        sounds(&generic, BOB),
        ["minecraft:item.brush.brushing.generic"]
    );
    assert_eq!(brush_damage(&fixture), Some(1));
}

/// Suspicious gravel sounds like gravel, turns into gravel, and an empty
/// block drops nothing.
#[test]
fn an_empty_suspicious_gravel_turns_into_gravel_without_a_drop() {
    let mut fixture = brushing_fixture("minecraft:suspicious_gravel");
    fixture.play(ALICE, brush_on_brushed_block());
    let first = idle_to(&mut fixture, 5);
    assert_eq!(
        sounds(&first, BOB),
        ["minecraft:item.brush.brushing.gravel"]
    );

    let completed = idle_to(&mut fixture, 95);
    assert_eq!(
        level_events(&completed, BOB),
        [(3008, BRUSHED, state("minecraft:suspicious_gravel", Some(3)))]
    );
    assert_eq!(
        fixture.world.block_state(OVERWORLD, BRUSHED),
        Some(state("minecraft:gravel", None))
    );
    assert!(ground_items(&fixture.world).is_empty());
    assert_eq!(brush_damage(&fixture), Some(1));
}

/// Looking off the block releases the use on the next tick
/// (`BrushItem.java:90-91`), and with no stroke left to push
/// `brushCountResetsAtTick` on, `checkReset` takes two brushes off forty ticks
/// after the last one and the rest four ticks later, forgetting the face.
#[test]
fn looking_away_releases_the_brush_and_the_dust_settles() {
    let mut fixture = brushing_fixture("minecraft:suspicious_sand");
    brushable_mut(&mut fixture.world)
        .set_item(crate::tests::native_stack(stack("minecraft:diamond", 1)));
    let alice = fixture.alice;
    fixture.play(ALICE, brush_on_brushed_block());
    idle_to(&mut fixture, 25);
    assert_eq!(dusted(&mut fixture.world), Some(2));

    fixture.play(ALICE, look(0.0, -90.0));
    assert!(!fixture.use_state().0);
    // The release runs after this tick's entity data went out.
    assert_eq!(living_flags(&fixture.idle(), BOB, alice), [0]);

    let mut settled = Vec::new();
    for since_stroke in 3..=46 {
        let tick = fixture.idle();
        if let Some(sent) = brushed_states_sent(&tick, BOB).last() {
            settled.push((since_stroke, *sent));
        }
    }
    assert_eq!(
        settled,
        [
            (40, state("minecraft:suspicious_sand", Some(1))),
            (44, state("minecraft:suspicious_sand", Some(0))),
        ]
    );
    let entity = brushable(&fixture.world).expect("the block is still there");
    assert_eq!(entity.hit_direction(), None);
    assert_eq!(entity.item().item_id(), Some(item_id("minecraft:diamond")));
}

/// `ProjectileUtil.getHitResultOnViewVector` picks a player standing in front
/// of the block, so the brush answers the click without starting; a spectator
/// is not pickable (`Player.java:1657-1659`).
#[test]
fn a_player_in_front_of_the_block_keeps_the_brush_from_starting() {
    let mut fixture = brushing_fixture("minecraft:suspicious_sand");
    fixture.world.set_player_position_for_test(
        ConnectionId(BOB),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 2.5,
        },
    );
    fixture.play(ALICE, brush_on_brushed_block());
    assert!(!fixture.use_state().0);
    assert_eq!(fixture.item_used("minecraft:brush"), 1);

    fixture.world.set_player_mode_and_abilities_for_test(
        ConnectionId(BOB),
        GameType::Spectator,
        PlayerAbilities::for_game_type(GameType::Spectator),
    );
    fixture.play(ALICE, brush_on_brushed_block());
    assert!(fixture.use_state().0);
    assert_eq!(fixture.item_used("minecraft:brush"), 2);
}

/// The first stroke unpacks the seeded archaeology table under the brushing
/// player (`BrushableBlockEntity.java:88-114`), which is what the
/// `player_generates_container_loot` criterion of `salvage_sherd` listens for.
#[test]
fn the_first_stroke_unpacks_the_loot_table_and_credits_the_salvage_criterion() {
    let table = serde_json::json!({
        "type": "minecraft:archaeology",
        "pools": [{
            "rolls": 1,
            "conditions": [{"condition":"minecraft:weather_check", "raining":false}, {
                "condition":"minecraft:entity_properties", "entity":"this",
                "predicate":{"type_specific":{"type":"minecraft:player","advancements":{"test:gate":false}}}
            }, {
                "condition": "minecraft:location_check", "offsetX": 1,
                "predicate": {"block": {"nbt": "{id:'minecraft:chest',x:1,y:65,z:4,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}
            }, {
                "condition":"minecraft:location_check", "offsetX":4, "offsetY":4,
                "predicate":{"biomes":["minecraft:plains"]}
            }],
            "entries": [{
                "type": "minecraft:item",
                "name": "minecraft:prize_pottery_sherd",
                "functions": [{
                    "function": "minecraft:copy_custom_data",
                    "source": "this",
                    "ops": [
                        {"source": "SelectedItem.id", "target": "selected", "op": "replace"},
                        {"source": "warden_spawn_tracker", "target": "tracker", "op": "replace"}
                    ]
                }, {
                    "function": "minecraft:copy_custom_data",
                    "source": {"type": "minecraft:storage", "source": "bbb:archaeology"},
                    "ops": [{"source": "marker", "target": "stored", "op": "replace"}]
                }]
            }]
        }]
    });
    let root = std::env::temp_dir().join(format!("bbb-brush-weather-{}", Uuid::new_v4()));
    let pack = root.join("weather");
    std::fs::create_dir_all(pack.join("data/minecraft/loot_table/archaeology")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/advancement")).unwrap();
    std::fs::write(
        pack.join("data/test/advancement/gate.json"),
        r#"{"criteria":{"gate":{"trigger":"minecraft:impossible"}}}"#,
    )
    .unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"archaeology weather","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(
        pack.join("data/minecraft/loot_table/archaeology/desert_pyramid.json"),
        serde_json::to_vec(&table).unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..ServerWorldConfig::default()
    };
    std::fs::remove_dir_all(root).unwrap();
    let prepare_table = |fixture: &mut Fixture| {
        for (operation, command) in [
            "setblock 1 65 4 chest",
            "item replace block 1 65 4 container.0 with diamond 3",
        ]
        .into_iter()
        .enumerate()
        {
            let tick = console_command(
                &fixture.sender,
                &fixture.receiver,
                &mut fixture.world,
                operation as u64,
                command,
            );
            assert_eq!(
                crate::tests::entities::console_command_answer(&tick).0,
                Some(1)
            );
        }
        fixture.command("data merge storage bbb:archaeology {marker:73}");
        fixture.world.set_warden_spawn_tracker_for_test(
            Uuid::from_u128(1),
            Some(WardenSpawnTracker::new(41, 2, 9)),
        );
        brushable_mut(&mut fixture.world)
            .set_loot_table(NbtSummaryValue::string(DESERT_PYRAMID), DESERT_PYRAMID_SEED);
    };
    let set_weather = |fixture: &mut Fixture, raining: bool| {
        let command = if raining {
            "weather rain 100000t"
        } else {
            "weather clear 100000t"
        };
        let tick = console_command(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            900,
            command,
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&tick).0,
            Some(100000)
        );
        for _ in 0..200 {
            if fixture.world.level_weather(OVERWORLD).is_raining() == raining {
                break;
            }
            fixture.idle();
        }
        assert_eq!(fixture.world.level_weather(OVERWORLD).is_raining(), raining);
    };
    let set_biome = |fixture: &mut Fixture, biome: &str| {
        // Cover every quart candidate for the offset biome sample.
        let filled = console_command(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            901,
            &format!("fillbiome 0 64 0 7 71 7 {biome}"),
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&filled).0,
            Some(8)
        );
        let checked = console_command(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            902,
            &format!("execute if biome 4 69 4 {biome}"),
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&checked).0,
            Some(1)
        );
    };
    let complete_empty = |fixture: &mut Fixture| {
        fixture.play(ALICE, brush_on_brushed_block());
        let mut unpacked_empty = false;
        let mut completed_event = false;
        for _ in 0..200 {
            let tick = fixture.idle();
            completed_event |= level_events(&tick, ALICE)
                .iter()
                .any(|(event, pos, _)| *event == 3008 && *pos == BRUSHED);
            if let Some(entity) = brushable(&fixture.world) {
                if entity.loot_table().is_none() {
                    assert!(entity.item().is_empty());
                    unpacked_empty = true;
                }
            }
            if fixture.world.block_state(OVERWORLD, BRUSHED) == Some(state("minecraft:sand", None))
            {
                break;
            }
        }
        assert!(unpacked_empty);
        assert!(completed_event);
        assert_eq!(
            fixture.world.block_state(OVERWORLD, BRUSHED),
            Some(state("minecraft:sand", None))
        );
        assert!(brushable(&fixture.world).is_none());
        assert_eq!(brush_damage(fixture), Some(1));
        assert!(ground_items(&fixture.world).is_empty());
    };
    let mut fixture = brushing_fixture_with_config("minecraft:suspicious_sand", config.clone());
    prepare_table(&mut fixture);
    set_weather(&mut fixture, false);
    set_biome(&mut fixture, "minecraft:plains");
    fixture.play(ALICE, brush_on_brushed_block());
    idle_to(&mut fixture, 4);
    assert!(brushable(&fixture.world)
        .and_then(BrushableBlockEntity::loot_table)
        .is_some());
    assert!(salvage_criteria(&fixture.world, ALICE).is_empty());

    idle_to(&mut fixture, 5);
    let entity = brushable(&fixture.world).expect("the block is still there");
    assert_eq!(entity.loot_table(), None);
    assert_eq!(
        (entity.item().item_id(), entity.item().count()),
        (Some(item_id("minecraft:prize_pottery_sherd")), 1)
    );
    let copied = entity
        .item()
        .component_nbt(
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            &fixture.world.item_stack_resolver(),
        )
        .unwrap()
        .expect("archaeology copied custom_data");
    use crate::loot_runtime::{nbt_field, nbt_string};
    assert_eq!(
        nbt_string(&copied, "selected").as_deref(),
        Some("minecraft:brush")
    );
    assert_eq!(
        nbt_field(&copied, "stored").and_then(bbb_protocol::nbt_summary::codec_int_value),
        Some(73)
    );
    let snapshot = fixture.world.command_entity_data_snapshot_for_test();
    let alice = &snapshot
        .iter()
        .find(|entry| entry.uuid == Uuid::from_u128(1))
        .unwrap()
        .data;
    assert_eq!(
        nbt_field(&copied, "tracker"),
        nbt_field(alice, "warden_spawn_tracker")
    );
    assert_eq!(salvage_criteria(&fixture.world, ALICE), ["desert_pyramid"]);
    assert!(salvage_criteria(&fixture.world, BOB).is_empty());

    idle_to(&mut fixture, 95);
    let items = ground_items(&fixture.world);
    assert_eq!(items.len(), 1);
    assert_eq!(
        (items[0].0.as_str(), items[0].1),
        ("minecraft:prize_pottery_sherd", 1)
    );

    let mut rainy = brushing_fixture_with_config("minecraft:suspicious_sand", config.clone());
    prepare_table(&mut rainy);
    set_weather(&mut rainy, true);
    set_biome(&mut rainy, "minecraft:plains");
    complete_empty(&mut rainy);
    assert!(rainy.world.level_weather(OVERWORLD).is_raining());

    let mut wrong_biome = brushing_fixture_with_config("minecraft:suspicious_sand", config);
    prepare_table(&mut wrong_biome);
    set_weather(&mut wrong_biome, false);
    set_biome(&mut wrong_biome, "minecraft:the_end");
    complete_empty(&mut wrong_biome);
    assert!(!wrong_biome.world.level_weather(OVERWORLD).is_raining());
}

/// Only the unpacked item and the `dusted` state are saved
/// (`BrushableBlockEntity.java:218-223`); the brush count is not, so the first
/// stroke after a restart counts from zero and takes the dust back to 1.
#[test]
fn a_restart_keeps_the_revealed_item_and_the_dust_but_not_the_strokes() {
    let root = std::env::temp_dir().join(format!("bbb-brush-restart-{}", std::process::id()));
    let storage = created_fixture_save(&root, "brush");
    let candidate = storage.load_candidate().unwrap();
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    let stage_alice = |world: &mut ServerWorld| {
        join_loaded(&sender, &receiver, world, ALICE, "alice", 1);
        world.set_player_mode_and_abilities_for_test(
            ConnectionId(ALICE),
            GameType::Survival,
            PlayerAbilities::for_game_type(GameType::Survival),
        );
        world.set_player_position_for_test(ConnectionId(ALICE), ALICE_AT);
        world.stock_menu_slot_for_test(
            ConnectionId(ALICE),
            HOTBAR_MENU_SLOT,
            stack("minecraft:brush", 1),
        );
        play(&sender, &receiver, world, ALICE, look(0.0, 0.0));
    };
    stage_alice(&mut world);
    stage_block(&mut world, "minecraft:suspicious_sand");
    brushable_mut(&mut world)
        .set_loot_table(NbtSummaryValue::string(DESERT_PYRAMID), DESERT_PYRAMID_SEED);
    play(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        brush_on_brushed_block(),
    );
    // Strokes at `timeElapsed` 5, 15 and 25.
    for _ in 1..25 {
        world.tick(receiver.drain());
    }
    assert_eq!(dusted(&mut world), Some(2));
    let revealed = brushable(&world).expect("the block").item().item_id();
    assert!(revealed.is_some());

    let request = world
        .save_request(1_786_000_000_500, 210, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(
        WorldSavePaths::new(&root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(datapack_config(), &reloaded);
    assert_eq!(dusted(&mut world), Some(2));
    let entity = brushable(&world).expect("the saved record came back");
    assert_eq!(entity.loot_table(), None);
    assert_eq!(entity.item().item_id(), revealed);

    stage_alice(&mut world);
    play(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        brush_on_brushed_block(),
    );
    for _ in 1..4 {
        world.tick(receiver.drain());
    }
    assert_eq!(dusted(&mut world), Some(2));
    world.tick(receiver.drain());
    assert_eq!(dusted(&mut world), Some(1));

    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}
