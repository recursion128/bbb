use super::*;
use crate::tests::{created_fixture_save, datapack_config, join_loaded, spawn_pinned_restore};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID;
use bbb_server_persistence::RegionCompression;
use bbb_server_types::save::Java26LevelKey;

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}

fn rules(world: &mut ServerWorld, enabled: bool) {
    let value = GameRuleValue::Boolean(enabled);
    assert!(world.admin.set_game_rule("minecraft:spawn_mobs", value));
    world.game_rules.set("minecraft:spawn_mobs", value);
    assert!(world.admin.set_game_rule(RULE, value));
}

fn ready(world: &mut ServerWorld) {
    world.cold.wandering_trader.tick_delay = 1;
    world.cold.wandering_trader.data = Some(Java26WanderingTraderData {
        spawn_delay: 1_200,
        spawn_chance: 100,
    });
}

#[test]
fn wandering_trader_clock_is_lazy_and_uses_old_inclusive_chance() {
    let random = GameRandom::with_seed(0);
    let mut state = WanderingTraderSpawner::default();
    for _ in 0..1_199 {
        assert!(!state.poll(&random));
    }
    assert_eq!(state.data, None);
    assert!(!state.poll(&random));
    assert_eq!(
        state.data,
        Some(Java26WanderingTraderData {
            spawn_delay: 22_800,
            spawn_chance: 25
        })
    );
    // `nextInt(100) <= 0` passes 1 roll in 100: about 20 of 2 000 (sd ~4.5),
    // where the stored next chance of 25 would pass about 520.
    let mut passes = 0;
    for _ in 0..2_000 {
        state.data = Some(Java26WanderingTraderData {
            spawn_delay: 0,
            spawn_chance: 0,
        });
        state.tick_delay = 1;
        passes += usize::from(state.poll(&random));
        assert_eq!(state.data.unwrap().spawn_chance, 25);
    }
    assert!((1..=60).contains(&passes), "{passes}");
}

#[test]
fn wandering_trader_rules_dimensions_and_no_player_success() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    ready(&mut world);
    rules(&mut world, false);
    let mut events = Vec::new();
    world.tick_wandering_trader(&world.require_level_id(&key()), &mut events);
    assert_eq!(world.cold.wandering_trader.tick_delay, 1);
    rules(&mut world, true);
    world.tick_wandering_trader(&Java26BuiltinDimension::Nether.into(), &mut events);
    assert_eq!(world.cold.wandering_trader.tick_delay, 1);
    world.tick_wandering_trader(&world.require_level_id(&key()), &mut events);
    assert_eq!(world.cold.wandering_trader.data.unwrap().spawn_chance, 25);
    assert!(events.is_empty());
}

#[test]
fn wandering_trader_normal_tick_owns_clock_and_pauses_with_rule() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    ready(&mut world);
    rules(&mut world, false);
    world.tick(Vec::new());
    assert_eq!(world.cold.wandering_trader.tick_delay, 1);
    rules(&mut world, true);
    world.tick(Vec::new());
    assert_eq!(world.cold.wandering_trader.tick_delay, 1_200);
    assert_eq!(
        world.cold.wandering_trader.data.unwrap().spawn_delay,
        24_000
    );
}

#[test]
fn wandering_trader_space_uses_live_empty_context_dynamic_shapes() {
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::{VANILLA_BLOCK_POWDER_SNOW_ID, VANILLA_BLOCK_SCAFFOLDING_ID};
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    let pos = BlockPos { x: 0, y: 100, z: 0 };
    let chunks = &mut world
        .level_mut_by_id(&world.require_level_id(&key()))
        .chunks;
    chunks.try_ensure_served_full_column(0, 0).unwrap();
    chunks
        .set_block_state(
            1,
            100,
            0,
            vanilla_block_default_state_id(VANILLA_BLOCK_POWDER_SNOW_ID).unwrap(),
        )
        .unwrap();
    assert!(world
        .trader_spawn_has_space(&world.require_level_id(&key()), pos)
        .unwrap());
    world
        .level_mut_by_id(&world.require_level_id(&key()))
        .chunks
        .set_block_state(
            1,
            100,
            0,
            vanilla_block_default_state_id(VANILLA_BLOCK_SCAFFOLDING_ID).unwrap(),
        )
        .unwrap();
    assert!(!world
        .trader_spawn_has_space(&world.require_level_id(&key()), pos)
        .unwrap());
}

#[test]
fn wandering_trader_normal_spawn_first_save_retry_and_reload_preserve_family() {
    let root = std::env::temp_dir().join(format!("bbb-trader-family-{}", std::process::id()));
    let storage = created_fixture_save(&root, "trader-family");
    let candidate = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "TraderTest", 71);
    let player = world.levels[crate::connection::ConnectionId(1)].position();
    let meeting = BlockPos {
        x: player.x.floor() as i32 + 3,
        y: player.y.floor() as i32,
        z: player.z.floor() as i32,
    };
    let bell = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_BELL_ID,
    )
    .unwrap();
    assert!(world.set_block(world.require_level_id(&key()), meeting, bell, 3));
    rules(&mut world, true);
    let traders = |world: &ServerWorld| -> Vec<_> {
        world.levels[&world.require_level_id(&key())]
            .entities
            .ids()
            .into_iter()
            .filter(|id| {
                world.levels[&world.require_level_id(&key())]
                    .entities
                    .get(*id)
                    .unwrap()
                    .entity_type_id()
                    == VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
            })
            .collect()
    };
    // Each ready tick spawns at the 1-in-10 roll; 200 all miss with odds of
    // about 7e-10.
    let mut events = Vec::new();
    for _ in 0..200 {
        ready(&mut world);
        events = world.tick(receiver.drain()).events;
        if !traders(&world).is_empty() {
            break;
        }
    }
    let traders = traders(&world);
    assert_eq!(traders.len(), 1, "{events:?}");
    let id = traders[0];
    let trader = world.levels[&world.require_level_id(&key())]
        .entities
        .get(id)
        .unwrap();
    let uuid = trader.uuid();
    let target = trader.wandering_trader_target().unwrap();
    assert_eq!(target, meeting);
    let home = trader.mob_home().unwrap();
    let delay = trader.trader_despawn_delay().unwrap();
    assert!((47_999..=48_000).contains(&delay));
    assert_eq!(home, (target, 16));
    assert_eq!(world.cold.wandering_trader.data.unwrap().spawn_chance, 25);
    let llamas: Vec<_> = world.levels[&world.require_level_id(&key())]
        .entities
        .ids()
        .into_iter()
        .filter_map(|id| {
            world.levels[&world.require_level_id(&key())]
                .entities
                .get(id)
        })
        .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID)
        .map(|entity| (entity.uuid(), entity.trader_despawn_delay().unwrap()))
        .collect();
    assert_eq!(llamas.len(), 2);
    let first = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    let blocked = storage
        .paths()
        .saved_data_dir()
        .join("minecraft")
        .join("wandering_trader.dat");
    std::fs::create_dir_all(&blocked).unwrap();
    assert!(storage.save_snapshot(&candidate, &first).is_err());
    std::fs::remove_dir(&blocked).unwrap();
    let retry = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(retry.wandering_trader, first.wandering_trader);
    assert_eq!(retry.random_sequences, first.random_sequences);
    storage.save_snapshot(&candidate, &retry).unwrap();
    world.acknowledge_save_success(&retry);
    let reloaded = storage.load_candidate().unwrap();
    let mut restored = spawn_pinned_restore(datapack_config(), &reloaded);
    assert_eq!(restored.cold.wandering_trader.data, first.wandering_trader);
    assert_eq!(restored.cold.wandering_trader.tick_delay, 1_200);
    let entities = &restored.levels[&restored.require_level_id(&key())].entities;
    let trader = entities
        .ids()
        .into_iter()
        .filter_map(|id| entities.get(id))
        .find(|entity| entity.uuid() == uuid)
        .unwrap();
    assert_eq!(trader.mob_home(), Some(home));
    assert_eq!(trader.wandering_trader_target(), Some(target));
    assert_eq!(trader.trader_despawn_delay(), Some(delay));
    for (uuid, delay) in llamas {
        let llama = entities
            .ids()
            .into_iter()
            .filter_map(|id| entities.get(id))
            .find(|entity| entity.uuid() == uuid)
            .unwrap();
        assert_eq!(llama.trader_despawn_delay(), Some(delay));
    }
    restored
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    drop(storage);
    std::fs::remove_dir_all(root).unwrap();
}
