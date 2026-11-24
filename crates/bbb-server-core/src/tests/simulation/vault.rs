use super::*;
use crate::block_entity::vault::{VaultBlockEntity, VaultState};
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID;
use bbb_protocol::item_types::VANILLA_ITEM_TRIAL_KEY_ID;
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;
use serde_json::json;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const VAULT: BlockPos = BlockPos { x: 8, y: 64, z: 8 };
const PLAYER: ConnectionId = ConnectionId(1);
const TABLE: &str = "minecraft:chests/trial_chambers/reward";

fn config(empty: bool) -> ServerWorldConfig {
    let pools = if empty {
        json!([])
    } else {
        let conditions = json!([{
            "condition":"minecraft:entity_properties", "entity":"this",
            "predicate":{"type_specific":{"type":"minecraft:player","advancements":{"test:gate":false}}}
        }, {
            "condition":"minecraft:location_check",
            "predicate":{"block":{"nbt":"{id:'minecraft:vault',x:8,y:64,z:8,config:{key_item:{id:'minecraft:trial_key',count:2}}}"}}
        }, {
            "condition":"minecraft:location_check", "offsetX":1,
            "predicate":{"block":{"nbt":"{id:'minecraft:chest',x:9,y:64,z:8,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}
        }, {
            "condition":"minecraft:location_check", "offsetY":4,
            "predicate":{"biomes":["minecraft:plains"]}
        }]);
        let mut wrong_biome = conditions.clone();
        wrong_biome[3]["predicate"]["biomes"] = json!(["minecraft:the_end"]);
        json!([
            {"rolls":1,"conditions":conditions,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
            {"rolls":1,"conditions":conditions,"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
            {"rolls":1,"conditions":wrong_biome,"entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}
        ])
    };
    let root = std::env::temp_dir().join(format!("bbb-vault-location-{}", Uuid::new_v4()));
    let pack = root.join("test");
    let (namespace, name) = TABLE.split_once(':').unwrap();
    let table = pack
        .join("data")
        .join(namespace)
        .join("loot_table")
        .join(format!("{name}.json"));
    std::fs::create_dir_all(table.parent().unwrap()).unwrap();
    std::fs::create_dir_all(pack.join("data/test/advancement")).unwrap();
    std::fs::write(
        pack.join("data/test/advancement/gate.json"),
        r#"{"criteria":{"gate":{"trigger":"minecraft:impossible"}}}"#,
    )
    .unwrap();
    std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
    std::fs::write(
        pack.join("data/test/loot_table/display.json"),
        r#"{"type":"minecraft:vault","pools":[{"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]}]}"#,
    )
    .unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"vault location rewards","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(
        table,
        serde_json::to_vec(&json!({"type":"minecraft:vault","pools":pools})).unwrap(),
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
    std::fs::remove_dir_all(root).unwrap();
    ServerWorldConfig {
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        datapacks: Some(std::sync::Arc::new(startup)),
        ..creative_fixture_config()
    }
}

fn candidate() -> Java26WorldCandidate {
    let chunk = Java26ChunkData::parse(candidate_raw(), restore_chunk_context()).unwrap();
    restore_candidate(0, Some(chunk), Vec::new())
}

fn candidate_raw() -> bbb_server_types::save::NamedCompound {
    let record = vec![
        nbt("id", NbtSummaryValue::String("minecraft:vault".into())),
        nbt("x", NbtSummaryValue::Int(VAULT.x)),
        nbt("y", NbtSummaryValue::Int(VAULT.y)),
        nbt("z", NbtSummaryValue::Int(VAULT.z)),
        nbt("components", NbtSummaryValue::Compound(Vec::new())),
        nbt(
            "config",
            NbtSummaryValue::Compound(vec![
                nbt(
                    "key_item",
                    NbtSummaryValue::Compound(vec![
                        nbt("id", NbtSummaryValue::String("minecraft:trial_key".into())),
                        nbt("count", NbtSummaryValue::Int(2)),
                    ]),
                ),
                nbt(
                    "override_loot_table_to_display",
                    NbtSummaryValue::String("test:display".into()),
                ),
            ]),
        ),
    ];
    let mut raw = restore_fixture_chunk_raw(vec![restore_section_with_cells(
        4,
        "minecraft:air",
        &[(
            [8, 0, 8],
            "minecraft:vault",
            &[
                ("facing", "north"),
                ("ominous", "false"),
                ("vault_state", "inactive"),
            ],
        )],
    )]);
    for entry in raw.entries_mut() {
        if entry.name == "block_entities" {
            entry.value = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(record.clone())]);
        }
    }
    raw
}

fn position_player(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    id: u64,
) {
    stand_at(
        sender,
        receiver,
        world,
        id,
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 6.5,
        },
    );
}

fn open_world(
    empty: bool,
    candidate: &Java26WorldCandidate,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let mut world = spawn_pinned_restore(config(empty), candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    position_player(&sender, &receiver, &mut world, 1);
    (sender, receiver, world)
}

fn vault(world: &ServerWorld) -> &VaultBlockEntity {
    world.get_block_entity(DIM, VAULT).unwrap().vault().unwrap()
}

fn state(world: &mut ServerWorld) -> VaultState {
    VaultState::from_block_state(world.block_state(DIM, VAULT).unwrap()).unwrap()
}

fn advance_to(world: &mut ServerWorld, time: i64) -> TickResult {
    assert!(world.game_time() < time);
    let mut last = world.tick(Vec::new());
    while world.game_time() < time {
        last = world.tick(Vec::new());
    }
    last
}

fn activate(world: &mut ServerWorld) -> TickResult {
    let time = vault(world)
        .server_data()
        .state_updating_resumes_at()
        .max(world.game_time() + 1);
    let tick = advance_to(world, time);
    assert_eq!(state(world), VaultState::Active);
    tick
}

fn key_stat(world: &ServerWorld) -> i32 {
    world.player_stat_for_test(
        PLAYER,
        StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, VANILLA_ITEM_TRIAL_KEY_ID),
    )
}

fn held_count(world: &ServerWorld) -> i32 {
    let snapshot = world.publication();
    let player = snapshot
        .players
        .iter()
        .find(|player| player.connection_id == PLAYER)
        .unwrap();
    player.inventory.slots[usize::from(player.inventory.selected_slot)].count
}

fn use_key(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: VAULT,
                direction: Direction::North,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    )
}

fn has_sound(tick: &TickResult, sound: &str) -> bool {
    let registry_id = vanilla_sound_event_registry_id(sound).unwrap();
    sounds_sent(tick)
        .iter()
        .any(|sound| sound.sound == SoundEventHolder::Reference { registry_id })
}

fn has_event(tick: &TickResult, id: i32) -> bool {
    level_events_sent(tick)
        .iter()
        .any(|(_, event)| event.pos == VAULT && event.event_type == id)
}

fn item_stacks(world: &ServerWorld) -> Vec<(String, i32)> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(DIM, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            let stack = item.item();
            Some((stack.item_resource_id()?.to_owned(), stack.count()))
        })
        .collect()
}

#[test]
fn vault_world_tick_uses_key_restores_queue_and_ejects_lifo_before_excluding_rewarded_player() {
    let original = candidate();
    let (sender, receiver, mut world) = open_world(false, &original);
    assert!(world.player_abilities_for_test(PLAYER).instabuild);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode spectator bob");
    position_player(&sender, &receiver, &mut world, 2);
    for command in [
        "setblock 9 64 8 chest",
        "item replace block 9 64 8 container.0 with diamond 3",
        "execute if biome 8 68 8 minecraft:plains",
    ] {
        let changed = console_command(&sender, &receiver, &mut world, 1, command);
        assert_eq!(console_command_answer(&changed).0, Some(1));
    }
    let activated = activate(&mut world);
    assert_eq!(
        vault(&world).shared_data().connected_players(),
        &[Uuid::from_u128(1)]
    );
    assert!(!vault(&world).shared_data().display_item().is_empty());
    assert!(has_event(&activated, 3015));

    chat_command(&sender, &receiver, &mut world, 1, "gamemode survival alice");
    for patch in [r#"rarity="uncommon""#, "!rarity"] {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("give alice minecraft:trial_key[{patch}] 3"),
        );
        assert_eq!(held_count(&world), 3);
        let rejected = use_key(&sender, &receiver, &mut world);
        assert_eq!(held_count(&world), 3);
        assert_eq!(key_stat(&world), 0);
        assert_eq!(state(&mut world), VaultState::Active);
        assert!(vault(&world).server_data().rewarded_players().is_empty());
        assert!(vault(&world).server_data().items_to_eject().is_empty());
        assert!(item_stacks(&world).is_empty());
        assert!(!has_sound(&rejected, "minecraft:block.vault.insert_item"));
        chat_command(&sender, &receiver, &mut world, 1, "clear alice");
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        r#"give alice minecraft:trial_key[rarity="common"] 3"#,
    );
    assert_eq!(held_count(&world), 3);
    let inserted_at = world.game_time();
    let inserted = use_key(&sender, &receiver, &mut world);
    assert_eq!(held_count(&world), 1);
    assert_eq!(key_stat(&world), 1);
    assert_eq!(state(&mut world), VaultState::Unlocking);
    assert!(has_sound(&inserted, "minecraft:block.vault.insert_item"));
    assert_eq!(
        vault(&world).server_data().rewarded_players(),
        &[Uuid::from_u128(1)]
    );
    assert!(vault(&world).shared_data().connected_players().is_empty());
    let opens_at = inserted_at + 14;
    assert_eq!(
        vault(&world).server_data().state_updating_resumes_at(),
        opens_at
    );
    let queue = vault(&world).server_data().items_to_eject().to_vec();
    assert_eq!(
        queue.iter().map(|item| item.count()).collect::<Vec<_>>(),
        vec![1, 1]
    );
    assert!(item_stacks(&world).is_empty());

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .unwrap();
    let chunk = Java26ChunkData::parse(&write.document, restore_chunk_context()).unwrap();
    let _ = &original;
    let saved = restore_candidate(world.game_time(), Some(chunk), Vec::new());
    let (_, _, mut restored) = open_world(false, &saved);
    assert_eq!(state(&mut restored), VaultState::Unlocking);
    assert_eq!(vault(&restored).server_data().items_to_eject(), queue);
    assert_eq!(
        vault(&restored).server_data().state_updating_resumes_at(),
        opens_at
    );
    assert_eq!(
        vault(&restored).server_data().rewarded_players(),
        &[Uuid::from_u128(1)]
    );
    advance_to(&mut restored, opens_at - 1);
    assert_eq!(state(&mut restored), VaultState::Unlocking);
    let opened = advance_to(&mut restored, opens_at);
    assert_eq!(state(&mut restored), VaultState::Ejecting);
    assert!(has_sound(&opened, "minecraft:block.vault.open_shutter"));
    assert!(item_stacks(&restored).is_empty());
    assert_eq!(
        vault(&restored).server_data().state_updating_resumes_at(),
        opens_at + 20
    );

    advance_to(&mut restored, opens_at + 19);
    assert!(item_stacks(&restored).is_empty());
    advance_to(&mut restored, opens_at + 20);
    assert_eq!(
        item_stacks(&restored),
        vec![("minecraft:diamond".into(), 1)]
    );
    let item_id = restored
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            restored
                .stored_entity_for_test(DIM, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_ITEM_ID)
        })
        .unwrap();
    let entity = restored.stored_entity_for_test(DIM, item_id).unwrap();
    assert_eq!(entity.position().x, f64::from(VAULT.x) + 0.5);
    assert_eq!(entity.position().y, f64::from(VAULT.y) + 1.2 - 0.125);
    assert_eq!(entity.position().z, f64::from(VAULT.z) + 0.5);
    let crate::entity::EntityData::Item(item) = entity.data() else {
        panic!("vault ejected an item");
    };
    assert_eq!(item.pickup_delay(), 0);
    assert_eq!(vault(&restored).server_data().items_to_eject().len(), 1);
    advance_to(&mut restored, opens_at + 40);
    assert_eq!(
        item_stacks(&restored),
        vec![
            ("minecraft:diamond".into(), 1),
            ("minecraft:apple".into(), 1)
        ]
    );
    let finished = advance_to(&mut restored, opens_at + 60);
    assert_eq!(state(&mut restored), VaultState::Inactive);
    assert!(vault(&restored)
        .shared_data()
        .connected_players()
        .is_empty());
    assert!(has_sound(&finished, "minecraft:block.vault.close_shutter"));
    assert!(has_event(&finished, 3016));
}

#[test]
fn vault_world_empty_reward_does_not_consume_key_or_award_usage() {
    let (sender, receiver, mut world) = open_world(true, &candidate());
    activate(&mut world);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode survival alice");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "give alice minecraft:trial_key 3",
    );
    let used = use_key(&sender, &receiver, &mut world);
    assert_eq!(held_count(&world), 3);
    assert_eq!(key_stat(&world), 0);
    assert_eq!(state(&mut world), VaultState::Active);
    assert!(vault(&world).server_data().rewarded_players().is_empty());
    assert!(vault(&world).server_data().items_to_eject().is_empty());
    assert!(item_stacks(&world).is_empty());
    assert!(!has_sound(&used, "minecraft:block.vault.insert_item"));
}
