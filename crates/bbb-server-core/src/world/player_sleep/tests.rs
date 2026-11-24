use super::*;
use crate::entity::{EntityData, GeneralLivingData};
use crate::mailbox::{
    world_command_mailbox, WorldCommandReceiver, WorldCommandSender, WorldMailboxLimits,
};
use crate::world::{TickResult, WorldCommand};
use bbb_protocol::packets::{
    BlockHitResult, Direction, EntityDataValueKind, InteractionHand, PlayServerbound,
    PlayerCommand, UseItemOn,
};
use bbb_protocol::player_command_actions::PlayerCommandAction;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::save::Java26LevelKey;
use serde_json::json;
use std::sync::Arc;

// Block.java:92: place both bed halves before notifying neighbors.
const UPDATE_CLIENTS: i32 = 2;

const ID: ConnectionId = ConnectionId(1);
const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const FOOT: BlockPos = BlockPos { x: 2, y: 101, z: 2 };
const HEAD: BlockPos = BlockPos { x: 3, y: 101, z: 2 };

struct Runtime {
    world: ServerWorld,
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
}

fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        name,
        &properties,
    )
    .unwrap()
}

fn runtime(can_sleep: &str, count: u64) -> Runtime {
    let mut config = crate::tests::datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    startup.advancements.advancements =
        vec![bbb_server_datapack::advancement::AdvancementElement {
            id: "test:sleep_runtime".into(),
            source_pack: "test".into(),
            advancement: crate::tests::parse_advancement(
                &json!({"criteria":{"slept":{"trigger":"minecraft:slept_in_bed"}}}),
            )
            .unwrap(),
        }];
    let dimension = startup
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == "minecraft:dimension_type")
        .unwrap()
        .elements
        .iter_mut()
        .find(|e| e.id == "minecraft:overworld")
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&dimension.source_json).unwrap();
    value["attributes"]["minecraft:gameplay/bed_rule"] =
        json!({"can_sleep":can_sleep,"can_set_spawn":"always"});
    dimension.source_json = serde_json::to_vec(&value).unwrap().into();
    dimension.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(value)).unwrap())
            .unwrap(),
    );
    let mut world = ServerWorld::new(config).unwrap();
    world.set_game_rule_for_test(
        "minecraft:players_sleeping_percentage",
        bbb_protocol::game_rules::GameRuleValue::Integer(101),
    );
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    for id in 1..=count {
        crate::tests::join_loaded(
            &sender,
            &receiver,
            &mut world,
            id,
            &format!("sleeper{id}"),
            u128::from(id),
        );
    }
    let stone = state("minecraft:stone", &[]);
    let air = state("minecraft:air", &[]);
    for x in 0..=5 {
        for z in 0..=5 {
            for y in 100..=104 {
                let expected = if y == 100 { stone } else { air };
                let pos = BlockPos { x, y, z };
                world.set_block(DIM, pos, expected, UPDATE_CLIENTS);
                assert_eq!(world.block_state(DIM, pos), Some(expected));
            }
        }
    }
    for (pos, part) in [(FOOT, "foot"), (HEAD, "head")] {
        assert!(world.set_block(
            DIM,
            pos,
            state(
                "minecraft:red_bed",
                &[("facing", "east"), ("occupied", "false"), ("part", part)]
            ),
            UPDATE_CLIENTS
        ));
    }
    for id in 1..=count {
        let mut events = Vec::new();
        assert!(world.teleport_player(
            ConnectionId(id),
            TeleportTransition {
                new_level: DIM.into(),
                position: Vec3d {
                    x: 1.5,
                    y: 101.0,
                    z: 2.5
                },
                y_rot: 35.0,
                x_rot: 12.0,
                relatives_mask: 0,
            },
            &mut events
        ));
        let teleport = events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::PlayerPosition(update),
                } if *connection_id == ConnectionId(id) => Some(update.id),
                _ => None,
            })
            .unwrap();
        sender
            .try_send(WorldCommand::Play {
                connection_id: ConnectionId(id),
                packet: PlayServerbound::AcceptTeleportation { id: teleport },
            })
            .unwrap();
        world.tick(receiver.drain());
        assert!(world.levels[ConnectionId(id)].awaiting_teleport.is_none());
        assert_eq!(
            world.levels[ConnectionId(id)].position(),
            Vec3d {
                x: 1.5,
                y: 101.0,
                z: 2.5
            }
        );
    }
    Runtime {
        world,
        sender,
        receiver,
    }
}

impl Runtime {
    fn play(&mut self, id: u64, packet: PlayServerbound) -> TickResult {
        self.sender
            .try_send(WorldCommand::Play {
                connection_id: ConnectionId(id),
                packet,
            })
            .unwrap();
        self.world.tick(self.receiver.drain())
    }
    fn click(&mut self, id: u64, pos: BlockPos) -> TickResult {
        self.play(
            id,
            PlayServerbound::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos,
                    direction: Direction::Up,
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
    fn tick(&mut self) -> TickResult {
        self.world.tick(self.receiver.drain())
    }
}

fn chats(tick: &TickResult) -> Vec<String> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SystemChat(chat),
                ..
            } => Some(format!("{:?}", chat.content)),
            _ => None,
        })
        .collect()
}

fn occupied(world: &ServerWorld) -> bool {
    bbb_server_types::block_queries::bool_property(
        stored_block_state(&world.level_by_id(&DIM.into()).chunks, HEAD),
        "occupied",
    )
    .unwrap()
}

#[test]
fn sleep_runtime_empty_hand_foot_and_head_clicks_enter_sleep_and_award_stat() {
    for clicked in [FOOT, HEAD] {
        let mut r = runtime("always", 1);
        assert_eq!(r.world.levels[ID].inventory.selected_stack().count(), 0);
        r.click(1, clicked);
        let player = &r.world.levels.player_at(ID);
        assert_eq!(player.cold.sleeping_pos, Some(HEAD));
        assert_eq!(player.pose_id, VANILLA_ENTITY_POSE_SLEEPING_ID);
        assert_eq!(
            player.position(),
            Vec3d {
                x: 3.5,
                y: 101.6875,
                z: 2.5
            }
        );
        assert_eq!(player.dimensions().unwrap(), [0.2, 0.2]);
        assert_eq!(player.cold.sleep_timer, 1);
        assert_eq!(
            player
                .cold
                .stats
                .value(custom_stat("minecraft:sleep_in_bed")),
            1
        );
        assert_eq!(player.cold.respawn.as_ref().unwrap().pos, HEAD);
        assert!(occupied(&r.world));
        assert!(r.world.player_advancement_is_done(ID, "test:sleep_runtime"));
    }
}

#[test]
fn sleep_runtime_sleep_refusal_keeps_the_new_respawn_position() {
    let mut r = runtime("never", 1);
    r.click(1, FOOT);
    let player = &r.world.levels.player_at(ID);
    assert_eq!(player.cold.sleeping_pos, None);
    assert_eq!(player.pose_id, VANILLA_ENTITY_POSE_STANDING_ID);
    let respawn = player.cold.respawn.as_ref().unwrap();
    assert_eq!(respawn.pos, HEAD);
    assert_eq!(respawn.yaw, 35.0);
    assert_eq!(respawn.pitch, 12.0);
    assert!(!respawn.forced);
    assert!(!occupied(&r.world));
}

#[test]
fn sleep_runtime_monster_class_blocks_survival_after_spawn_but_cow_and_creative_allow_sleep() {
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID};
    for (entity_type, creative, expected) in [
        (VANILLA_ENTITY_TYPE_ZOMBIE_ID, false, false),
        (VANILLA_ENTITY_TYPE_COW_ID, false, true),
        (VANILLA_ENTITY_TYPE_ZOMBIE_ID, true, true),
    ] {
        let mut r = runtime("always", 1);
        if !creative {
            let mut abilities = r.world.player_abilities_for_test(ID);
            abilities.invulnerable = false;
            abilities.instabuild = false;
            abilities.flying = false;
            r.world
                .set_player_mode_and_abilities_for_test(ID, GameType::Survival, abilities);
        }
        r.world.spawn_stored_entity_for_test(
            DIM,
            entity_type,
            Vec3d {
                x: 5.0,
                y: 101.0,
                z: 3.5,
            },
            EntityData::GeneralLiving(GeneralLivingData::new(entity_type).unwrap()),
        );
        let tick = r.click(1, FOOT);
        assert_eq!(r.world.levels[ID].cold.sleeping_pos.is_some(), expected);
        assert_eq!(r.world.levels[ID].cold.respawn.as_ref().unwrap().pos, HEAD);
        assert_eq!(
            chats(&tick)
                .iter()
                .any(|c| c.contains("block.minecraft.bed.not_safe")),
            !expected
        );
    }
}

#[test]
fn sleep_runtime_occupied_bed_kicks_first_saved_sleeping_villager_and_preserves_other_sleeper() {
    let mut r = runtime("always", 1);
    let mut ids = Vec::new();
    for _ in 0..2 {
        let entity_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID;
        let (id, _) = r.world.spawn_stored_entity_for_test(
            DIM,
            entity_type,
            Vec3d {
                x: 3.5,
                y: 101.6875,
                z: 2.5,
            },
            EntityData::GeneralLiving(GeneralLivingData::new(entity_type).unwrap()),
        );
        let NbtSummaryValue::Compound(mut entries) = r
            .world
            .stored_entity_command_data_for_test(DIM, id)
            .unwrap()
        else {
            panic!("entity record")
        };
        entries.push(NbtSummaryEntry {
            name: "sleeping_pos".into(),
            value: NbtSummaryValue::IntArray(vec![HEAD.x, HEAD.y, HEAD.z]),
        });
        // Without AI the daytime `WakeUp` leaves both sleepers in bed.
        entries.retain(|entry| entry.name != "NoAI");
        entries.push(NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(1),
        });
        r.world
            .load_stored_entity_record_for_test(DIM, id, &entries);
        ids.push(id);
    }
    r.world.set_bed_occupied(&DIM.into(), HEAD, true);
    let tick = r.click(1, FOOT);
    let store = &r.world.level(DIM).entities;
    assert_eq!(store.get(ids[0]).unwrap().sleeping_pos(), None);
    assert_eq!(store.get(ids[1]).unwrap().sleeping_pos(), Some(HEAD));
    assert_eq!(r.world.levels[ID].cold.sleeping_pos, None);
    assert!(!chats(&tick)
        .iter()
        .any(|c| c.contains("block.minecraft.bed.occupied")));
}

#[test]
fn sleep_runtime_occupied_player_bed_reports_occupied_without_displacing_the_sleeper() {
    let mut r = runtime("always", 2);
    r.click(1, FOOT);
    let tick = r.click(2, FOOT);
    assert_eq!(r.world.levels[ID].cold.sleeping_pos, Some(HEAD));
    assert_eq!(r.world.levels[ConnectionId(2)].cold.sleeping_pos, None);
    assert!(chats(&tick)
        .iter()
        .any(|c| c.contains("block.minecraft.bed.occupied")));
}

#[test]
fn sleep_runtime_stop_sleeping_packet_publishes_clear_position_and_animation_then_expires_counter()
{
    let mut r = runtime("always", 2);
    r.click(1, FOOT);
    let entity_id = r.world.levels[ID].entity_id();
    let tick = r.play(
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id,
            action: PlayerCommandAction::StopSleeping,
            data: 0,
        }),
    );
    let player = &r.world.levels.player_at(ID);
    assert_eq!(player.cold.sleeping_pos, None);
    assert_eq!(player.pose_id, VANILLA_ENTITY_POSE_STANDING_ID);
    assert_eq!(player.dimensions().unwrap(), [0.6, 1.8]);
    assert_eq!(player.cold.sleep_timer, 101);
    assert_eq!(player.rotation().x_rot, 0.0);
    assert!(!occupied(&r.world));
    assert!(tick.events.iter().any(|event| matches!(event.sent_packet(), Some(ClientboundPlayPacket::EntityAnimation(animation)) if animation.id == entity_id && animation.action == 2)));
    assert!(tick.events.iter().any(|event| matches!(
        event.sent_packet(),
        Some(ClientboundPlayPacket::PlayerPosition(_))
    )));
    assert!(tick.events.iter().any(|event| matches!(event.sent_packet(), Some(ClientboundPlayPacket::SetEntityData(update)) if update.id == entity_id && update.values.iter().any(|value| value.data_id == 14 && value.value == EntityDataValueKind::OptionalBlockPos(None)))));
    for expected in 102..=109 {
        r.tick();
        assert_eq!(r.world.levels[ID].cold.sleep_timer, expected);
    }
    r.tick();
    assert_eq!(r.world.levels[ID].cold.sleep_timer, 0);
}

#[test]
fn sleep_runtime_removed_bed_forcefully_wakes_the_player() {
    let mut r = runtime("always", 1);
    r.click(1, FOOT);
    r.world
        .set_block(DIM, HEAD, state("minecraft:air", &[]), UPDATE_CLIENTS);
    let tick = r.tick();
    assert_eq!(r.world.levels[ID].cold.sleeping_pos, None);
    assert_eq!(r.world.levels[ID].cold.sleep_timer, 0);
    assert_eq!(r.world.levels[ID].pose_id, VANILLA_ENTITY_POSE_STANDING_ID);
    assert!(tick.events.iter().any(|event| matches!(event,WorldEvent::Send {packet:ClientboundPlayPacket::EntityAnimation(animation),..} if animation.action==2)));
}

#[test]
fn sleep_runtime_local_percentage_waits_for_deep_sleep_then_skips_while_frozen_without_advancing_disabled_rules(
) {
    use bbb_protocol::game_rules::GameRuleValue;
    let mut r = runtime("always", 2);
    r.world.set_game_rule_for_test(
        "minecraft:players_sleeping_percentage",
        GameRuleValue::Integer(100),
    );
    r.world
        .set_game_rule_for_test("minecraft:advance_time", GameRuleValue::Boolean(false));
    r.world
        .set_game_rule_for_test("minecraft:advance_weather", GameRuleValue::Boolean(false));
    r.click(1, FOOT);
    for _ in 0..99 {
        r.tick();
    }
    assert_eq!(r.world.levels[ID].cold.sleep_timer, 100);
    r.tick();
    assert_eq!(r.world.levels[ID].cold.sleeping_pos, Some(HEAD));
    let clocks = r.world.world_clock_states_for_test();
    let weather = r.world.weather_data();
    r.world.tick_rate_mut().set_frozen(true);
    r.world.set_game_rule_for_test(
        "minecraft:players_sleeping_percentage",
        GameRuleValue::Integer(50),
    );
    r.tick();
    assert_eq!(r.world.levels[ID].cold.sleeping_pos, None);
    assert_eq!(r.world.world_clock_states_for_test(), clocks);
    assert_eq!(r.world.weather_data(), weather);
    assert_eq!(r.world.levels[ConnectionId(2)].cold.sleeping_pos, None);

    let mut r = runtime("always", 1);
    r.click(1, FOOT);
    for _ in 0..98 {
        r.tick();
    }
    r.world.weather.raining = true;
    r.world.weather.thundering = true;
    r.world.weather.rain_time = 400;
    r.world.weather.thunder_time = 400;
    r.world.weather.clear_weather_time = 0;
    let weather = r.world.weather;
    r.world
        .level_mut_by_id(&DIM.into())
        .weather
        .prepare(weather);
    r.tick();
    assert_eq!(r.world.levels[ID].cold.sleep_timer, 100);
    let clock = r
        .world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    let mut clock_state = r.world.world_clock_states_for_test()[clock];
    clock_state.total_ticks = 22000;
    r.world.set_world_clock_state_for_test(clock, clock_state);
    r.world.weather.clear_weather_time = 47;
    r.world.tick_rate_mut().set_frozen(true);
    r.world.set_game_rule_for_test(
        "minecraft:players_sleeping_percentage",
        GameRuleValue::Integer(0),
    );
    r.tick();
    assert_eq!(r.world.levels[ID].cold.sleeping_pos, None);
    assert!(r
        .world
        .is_at_dimension_time_marker(&DIM.into(), "minecraft:wake_up_from_sleep")
        .unwrap());
    assert!(r.world.world_clock_states_for_test()[clock].total_ticks > 22000);
    let weather = r.world.weather_data();
    assert!(!weather.raining);
    assert!(!weather.thundering);
    assert_eq!((weather.rain_time, weather.thunder_time), (0, 0));
    assert_eq!(weather.clear_weather_time, 47);
}

#[test]
fn sleep_runtime_charged_nether_anchor_sets_spawn_once_and_empty_anchor_passes() {
    let mut r = runtime("always", 1);
    let dimension: Java26LevelKey = Java26BuiltinDimension::Nether.into();
    let mut events = Vec::new();
    assert!(r.world.teleport_player(
        ID,
        TeleportTransition {
            new_level: dimension.clone(),
            position: Vec3d {
                x: 1.5,
                y: 101.0,
                z: 2.5
            },
            y_rot: 35.0,
            x_rot: 12.0,
            relatives_mask: 0,
        },
        &mut events
    ));
    let teleport = events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    r.play(1, PlayServerbound::AcceptTeleportation { id: teleport });
    r.play(1, PlayServerbound::PlayerLoaded);
    r.world
        .install_full_chunks_by_key(&r.world.require_level_id(&dimension), &[(0, 0)])
        .unwrap();
    let anchor = FOOT;
    for charges in ["0", "1"] {
        assert!(r.world.set_block(
            Java26BuiltinDimension::Nether,
            anchor,
            state("minecraft:respawn_anchor", &[("charges", charges)]),
            UPDATE_CLIENTS
        ));
        let tick = r.click(1, anchor);
        if charges == "0" {
            assert!(r.world.levels[ID].cold.respawn.is_none());
        } else {
            let respawn = r.world.levels[ID].cold.respawn.as_ref().unwrap();
            assert_eq!(respawn.dimension, dimension);
            assert_eq!(respawn.pos, anchor);
            assert_eq!((respawn.yaw, respawn.pitch), (0.0, 0.0));
            assert!(chats(&tick)
                .iter()
                .any(|c| c.contains("block.minecraft.set_spawn")));
            assert_eq!(
                crate::block_behavior::respawn_anchor::charges(
                    r.world
                        .block_state(Java26BuiltinDimension::Nether, anchor)
                        .unwrap()
                ),
                Some(1)
            );
            let repeated = r.click(1, anchor);
            assert!(!chats(&repeated)
                .iter()
                .any(|c| c.contains("block.minecraft.set_spawn")));
            assert!(!repeated.events.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::Sound(_),
                    ..
                }
            )));
        }
    }
}

mod hurt_and_move;
mod respawn_explosion;

#[test]
fn sleep_runtime_disconnecting_sleeper_frees_the_bed_and_is_saved_awake() {
    let mut r = runtime("always", 1);
    r.click(1, FOOT);
    assert!(occupied(&r.world));
    let uuid = r.world.levels[ID].profile.uuid;
    r.world.remove_player_for_test(ID);
    assert!(!occupied(&r.world));
    assert_eq!(
        r.world.cold.departed_player_snapshots[&uuid]
            .patch
            .created
            .sleeping_pos,
        None
    );
}
