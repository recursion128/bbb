//! The weather cycle as the running world drives it: the store the save
//! carries, the tick that advances it, the packets it sends and the rain the
//! block bodies read.

use super::*;

use bbb_protocol::game_event_types::{
    VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
    VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID, VANILLA_CLIENT_GAME_EVENT_START_RAINING_ID,
    VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID,
};
use bbb_server_persistence::Java26WeatherData;

fn joined_world() -> (ServerWorld, WorldCommandReceiver, WorldCommandSender) {
    let (sender, receiver) = mailbox();
    let world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    (world, receiver, sender)
}

/// One `command.execute` request, which is the administration surface's route
/// into the same dispatcher a chat command reaches.
fn enqueue_command(sender: &WorldCommandSender, operation_id: u64, command: &str) {
    enqueue(
        sender,
        WorldCommand::Admin {
            envelope: admin_envelope(operation_id, command, AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: command.to_string(),
            },
        },
    )
    .unwrap();
}

fn restore_candidate_with_weather(weather: Java26WeatherData) -> Java26WorldCandidate {
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate.weather = weather;
    candidate
}

fn game_events(tick: &TickResult) -> Vec<(u8, f32)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::GameEvent(update),
                ..
            } => Some((update.event_id, update.param)),
            _ => None,
        })
        .collect()
}

/// A world with nobody online still runs `advanceWeatherCycle`
/// (`ServerLevel.java:349`), which arms both timers off the level random on the
/// first tick that finds them expired (`:716-731`).
#[test]
fn the_cycle_arms_both_timers_on_the_first_tick() {
    let (mut world, receiver, _sender) = joined_world();
    assert_eq!(world.weather_data(), Java26WeatherData::default());
    world.tick(receiver.drain());
    let armed = world.weather_data();
    // Not raining and not thundering, so both delays: `UniformInt.of(12000,
    // 180000)` (`ServerLevel.java:187`, `:189`).
    assert!((12_000..=180_000).contains(&armed.rain_time), "{armed:?}");
    assert!(
        (12_000..=180_000).contains(&armed.thunder_time),
        "{armed:?}"
    );
    assert!(!armed.raining);
    assert!(!armed.thundering);
    // The next tick spends one tick of each, so the cycle is running rather
    // than re-arming.
    world.tick(receiver.drain());
    assert_eq!(world.weather_data().rain_time, armed.rain_time - 1);
    assert_eq!(world.weather_data().thunder_time, armed.thunder_time - 1);
}

/// `Level.canHaveWeather` (`Level.java:933-935`): only the overworld among the
/// three levels this server runs, so the shared store is stepped once a tick
/// rather than three times.
#[test]
fn only_the_overworld_advances_the_shared_store() {
    let (mut world, receiver, _sender) = joined_world();
    world.tick(receiver.drain());
    let armed = world.weather_data().rain_time;
    world.tick(receiver.drain());
    // Three levels ticked; exactly one tick came off the timer.
    assert_eq!(world.weather_data().rain_time, armed - 1);
    assert!(world
        .level_weather(Java26BuiltinDimension::Overworld)
        .can_have_weather());
    assert!(!world
        .level_weather(Java26BuiltinDimension::Nether)
        .can_have_weather());
    assert!(!world
        .level_weather(Java26BuiltinDimension::End)
        .can_have_weather());
}

/// `/weather rain` reaches `setWeatherParameters` (`MinecraftServer.java:1766-1773`),
/// which writes the same duration into both timers.
#[test]
fn the_command_writes_the_duration_into_both_timers() {
    let (mut world, receiver, sender) = joined_world();
    enqueue_command(&sender, 1, "weather rain 400t");
    world.tick(receiver.drain());
    let stored = world.weather_data();
    assert_eq!(stored.clear_weather_time, 0);
    assert_eq!(stored.rain_time, 399);
    // `setThunderTime(rainTime)` (`MinecraftServer.java:1770`) -- the same
    // value, not zero and not a second draw.
    assert_eq!(stored.thunder_time, 399);
    assert!(stored.raining);
    assert!(!stored.thundering);
}

/// `/weather thunder` differs from `rain` only in the `thundering` flag
/// (`WeatherCommand.java:55` against `:61`).
#[test]
fn the_thunder_arm_sets_the_thundering_flag() {
    let (mut world, receiver, sender) = joined_world();
    enqueue_command(&sender, 1, "weather thunder 400t");
    world.tick(receiver.drain());
    assert!(world.weather_data().raining);
    assert!(world.weather_data().thundering);
}

/// `/weather clear` puts the duration in the clear timer and clears both flags
/// (`WeatherCommand.java:49`).
#[test]
fn the_clear_arm_arms_the_clear_countdown() {
    let (mut world, receiver, sender) = joined_world();
    enqueue_command(&sender, 1, "weather rain 400t");
    world.tick(receiver.drain());
    enqueue_command(&sender, 2, "weather clear 200t");
    world.tick(receiver.drain());
    let stored = world.weather_data();
    assert_eq!(stored.clear_weather_time, 199);
    assert!(!stored.raining);
    assert!(!stored.thundering);
}

/// A bare `/weather rain` draws its duration off the source level's random
/// (`WeatherCommand.java:44-46`), which is `RAIN_DURATION`
/// (`ServerLevel.java:188`) rather than the delay.
#[test]
fn a_bare_literal_samples_the_kinds_own_provider() {
    let (mut world, receiver, sender) = joined_world();
    enqueue_command(&sender, 1, "weather rain");
    world.tick(receiver.drain());
    let stored = world.weather_data();
    // 12000..=24000 minus the tick the cycle immediately spent.
    assert!((11_999..=23_999).contains(&stored.rain_time), "{stored:?}");
    assert!(stored.raining);
}

/// `PlayerList.sendLevelInfo`'s rain arm (`PlayerList.java:649-653`) sends
/// nothing at all for a level that is not raining.
#[test]
fn a_dry_join_carries_no_weather_packets() {
    let (mut world, receiver, sender) = joined_world();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    let ids = game_events(&tick)
        .into_iter()
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    assert!(
        !ids.contains(&VANILLA_CLIENT_GAME_EVENT_START_RAINING_ID),
        "{ids:?}"
    );
    assert!(
        !ids.contains(&VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID),
        "{ids:?}"
    );
}

/// A join into a raining level opens with the three weather packets, in the
/// order `sendLevelInfo` sends them and ahead of the chunk-load announcement
/// (`PlayerList.java:649-655`).
#[test]
fn a_join_into_a_storm_is_told_the_start_and_both_levels() {
    let (mut world, receiver, sender) = joined_world();
    enqueue_command(&sender, 1, "weather rain 4000t");
    // Twenty-one ticks put the rain level above the 0.2 `isRaining` needs.
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert!(world
        .level_weather(Java26BuiltinDimension::Overworld)
        .is_raining());
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    // The join is answered before this tick's own weather step, so the levels
    // the packets carry are the ones standing when it arrived.
    let sent_rain = world
        .level_weather(Java26BuiltinDimension::Overworld)
        .rain_level();
    let sent_thunder = world
        .level_weather(Java26BuiltinDimension::Overworld)
        .thunder_level();
    let tick = world.tick(receiver.drain());
    let events = game_events(&tick);
    let start = events
        .iter()
        .position(|(id, _)| *id == VANILLA_CLIENT_GAME_EVENT_START_RAINING_ID)
        .expect("a raining level announces the start");
    assert_eq!(
        events[start],
        (VANILLA_CLIENT_GAME_EVENT_START_RAINING_ID, 0.0)
    );
    assert_eq!(
        events[start + 1],
        (VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID, sent_rain)
    );
    // `getThunderLevel(1.0F)` (`PlayerList.java:652`), the scaled one.
    assert_eq!(
        events[start + 2],
        (
            VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID,
            sent_thunder
        )
    );
    assert_eq!(
        events[start + 3].0,
        VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID
    );
}

/// The store travels into the save request whole (`WeatherData.java:11-15`).
#[test]
fn the_save_request_carries_the_live_store() {
    let (mut world, receiver, sender) = joined_world();
    enqueue_command(&sender, 1, "weather thunder 400t");
    world.tick(receiver.drain());
    let live = world.weather_data();
    let request = world
        .save_request(0, 0, bbb_server_persistence::RegionCompression::Zlib)
        .expect("the fixture world builds a save request");
    assert_eq!(request.weather, live);
    assert!(request.weather.thundering);
}

/// `ServerLevel.prepareWeather` (`ServerLevel.java:271-273`, `:686-693`) runs
/// at restore, so a save that stopped mid-storm is raining on its first tick
/// rather than fading in over a hundred of them.
#[test]
fn a_restored_storm_is_already_raining() {
    let candidate = restore_candidate_with_weather(Java26WeatherData {
        clear_weather_time: 0,
        rain_time: 4000,
        thunder_time: 4000,
        raining: true,
        thundering: true,
    });
    let world = ServerWorld::restore(ServerWorldConfig::default(), &candidate)
        .expect("the fixture save restores");
    assert_eq!(world.weather_data().rain_time, 4000);
    let weather = world.level_weather(Java26BuiltinDimension::Overworld);
    assert!(weather.is_raining());
    assert!(weather.is_thundering());
    assert_eq!(weather.rain_level(), 1.0);
}

/// A save that never wrote the document restores dry.
#[test]
fn a_restored_clear_sky_is_not_raining() {
    let candidate = restore_candidate_with_weather(Java26WeatherData::default());
    let world = ServerWorld::restore(ServerWorldConfig::default(), &candidate)
        .expect("the fixture save restores");
    assert!(!world
        .level_weather(Java26BuiltinDimension::Overworld)
        .is_raining());
}

/// A world built with no datapack generation has no biome documents, so
/// `Level.precipitationAt`'s biome read (`Level.java:962-963`) has nothing to
/// answer from and the whole predicate refuses rather than guessing.
#[test]
fn a_level_without_a_generation_reports_no_precipitation() {
    let (mut world, receiver, sender) = joined_world();
    enqueue_command(&sender, 1, "weather rain 4000t");
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert!(world
        .level_weather(Java26BuiltinDimension::Overworld)
        .is_raining());
    let fixture = FlatFixture::new().unwrap();
    let open = BlockPos {
        x: 3,
        y: fixture.floor_y() + 1,
        z: 5,
    };
    assert_eq!(
        world.precipitation_at(Java26BuiltinDimension::Overworld, open),
        crate::world::Precipitation::None
    );
}

/// The same reads against a world that does carry the biome registry:
/// `Level.precipitationAt` (`Level.java:949-964`) refuses while it is dry,
/// answers above the `MOTION_BLOCKING` heightmap once it rains, and refuses
/// below it (`:958-960`).
#[test]
fn a_generated_level_reports_rain_only_above_the_heightmap() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(startup_generation()),
        ..ServerWorldConfig::default()
    })
    .unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    for _ in 0..4 {
        world.tick(receiver.drain());
    }
    let (x, z) = (0, 0);
    let height = world
        .motion_blocking_height(Java26BuiltinDimension::Overworld, x, z)
        .expect("the spawn column is loaded");
    let open = BlockPos { x, y: height, z };
    let covered = BlockPos {
        x,
        y: height - 2,
        z,
    };
    // Dry: nothing anywhere, whatever the heightmap says.
    assert_eq!(
        world.precipitation_at(Java26BuiltinDimension::Overworld, open),
        crate::world::Precipitation::None
    );

    enqueue_command(&sender, 1, "weather rain 4000t");
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert!(world
        .level_weather(Java26BuiltinDimension::Overworld)
        .is_raining());
    // Above the heightmap the biome decides between rain and snow; below it
    // nothing falls at all.
    assert_ne!(
        world.precipitation_at(Java26BuiltinDimension::Overworld, open),
        crate::world::Precipitation::None
    );
    assert_eq!(
        world.precipitation_at(Java26BuiltinDimension::Overworld, covered),
        crate::world::Precipitation::None
    );
    // `isRainingAt` is the rain arm of that answer, so the covered position is
    // not rained on either way.
    assert!(!world.is_raining_at(Java26BuiltinDimension::Overworld, covered));
}

/// `Level.canHaveWeather` gates `isRaining` (`Level.java:941-943`), so the
/// nether never reports rain however the shared store stands.
#[test]
fn the_nether_never_reports_rain() {
    let (mut world, receiver, sender) = joined_world();
    enqueue_command(&sender, 1, "weather rain 4000t");
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert!(world.weather_data().raining);
    let pos = BlockPos { x: 3, y: 70, z: 5 };
    assert!(!world.is_raining_at(Java26BuiltinDimension::Nether, pos));
}

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

/// A restored flat world with the built-in datapack, an operator online and
/// `weather rain` already at full strength.
fn raining_flat_world() -> (ServerWorld, WorldCommandReceiver, WorldCommandSender) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_restore(
        ServerWorldConfig {
            datapacks: datapack_config().datapacks,
            ..operator_config(&[("alice", 1)])
        },
        &restore_candidate(0, None, Vec::new()),
    );
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(&sender, &receiver, &mut world, 1, "weather rain 100000t");
    for _ in 0..100 {
        world.tick(receiver.drain());
    }
    assert!(world.level_weather(OVERWORLD).is_raining());
    (world, receiver, sender)
}

fn block_name_and(world: &mut ServerWorld, pos: BlockPos, property: &str) -> (String, String) {
    let state = world.block_state(OVERWORLD, pos).unwrap();
    let identity = bbb_protocol::block_state_types::vanilla_block_state_identity(state).unwrap();
    (
        identity.name.to_string(),
        identity
            .properties
            .get(property)
            .cloned()
            .unwrap_or_default(),
    )
}

/// `ServerLevel.tickPrecipitation` (`ServerLevel.java:564-594`) in a snowy
/// biome: snow settles on the floor and grows a layer per draw up to
/// `max_snow_accumulation_height`, and a lone water source freezes.
#[test]
fn snowfall_stacks_layers_to_the_rule_and_freezes_open_water() {
    let (mut world, receiver, sender) = raining_flat_world();
    for command in [
        "fillbiome -8 56 -8 23 79 23 minecraft:snowy_plains",
        "gamerule max_snow_accumulation_height 3",
        "setblock 6 63 6 minecraft:water",
    ] {
        chat_command(&sender, &receiver, &mut world, 1, command);
    }
    let floor = BlockPos { x: 4, y: 64, z: 4 };
    world.tick_precipitation_for_test(OVERWORLD, floor.x, floor.z);
    assert_eq!(
        block_name_and(&mut world, floor, "layers"),
        ("minecraft:snow".to_string(), "1".to_string())
    );
    for _ in 0..5 {
        world.tick_precipitation_for_test(OVERWORLD, floor.x, floor.z);
    }
    assert_eq!(
        block_name_and(&mut world, floor, "layers"),
        ("minecraft:snow".to_string(), "3".to_string())
    );

    let water = BlockPos { x: 6, y: 63, z: 6 };
    world.tick_precipitation_for_test(OVERWORLD, water.x, water.z);
    assert_eq!(block_name_and(&mut world, water, "").0, "minecraft:ice");
}

/// `CauldronBlock.handlePrecipitation` / `LayeredCauldronBlock.handlePrecipitation`:
/// rain in a temperate biome turns an empty cauldron into a water cauldron and
/// raises it to full, never further.
#[test]
fn rain_fills_a_cauldron_to_the_brim() {
    let (mut world, receiver, sender) = raining_flat_world();
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 4 64 4 minecraft:cauldron",
    );
    let cauldron = BlockPos { x: 4, y: 64, z: 4 };
    let mut levels = Vec::new();
    for _ in 0..2000 {
        world.tick_precipitation_for_test(OVERWORLD, cauldron.x, cauldron.z);
        let (name, level) = block_name_and(&mut world, cauldron, "level");
        if name == "minecraft:water_cauldron" && levels.last() != Some(&level) {
            levels.push(level);
        }
    }
    assert_eq!(levels, ["1", "2", "3"]);
}

/// The draws ride the chunk tick: with a high `random_tick_speed` snow
/// settles somewhere on a chunk turned snowy within a few ticks.
#[test]
fn the_chunk_tick_draws_precipitation_columns() {
    let (mut world, receiver, sender) = raining_flat_world();
    for command in [
        "fillbiome 0 56 0 15 79 15 minecraft:snowy_plains",
        "gamerule random_tick_speed 1000",
    ] {
        chat_command(&sender, &receiver, &mut world, 1, command);
    }
    let snowed = |world: &mut ServerWorld| {
        (0..16).any(|x| {
            (0..16)
                .any(|z| block_name_and(world, BlockPos { x, y: 64, z }, "").0 == "minecraft:snow")
        })
    };
    for _ in 0..20 {
        world.tick(receiver.drain());
    }
    assert!(snowed(&mut world));
}
