//! Live target avoidance and panic navigation.

use super::*;
use crate::commands::CommandDamageSourceEntity;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_PIG_ID,
    VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_SQUID_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_SQUID_INK_ID;

const ALICE: ConnectionId = ConnectionId(1);

fn fixture(commands: &[&str]) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    config.difficulty = Difficulty::Normal;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    );
    let base = ["gamerule spawn_mobs false", "weather clear"];
    for (id, command) in base.iter().chain(commands).enumerate() {
        let tick = console_command(&sender, &receiver, &mut world, 10 + id as u64, command);
        assert!(console_command_answer(&tick).0.is_some(), "{command}");
    }
    (sender, receiver, world)
}

fn spawn_still(world: &mut ServerWorld, entity_type_id: i32, position: Vec3d) -> (i32, Uuid) {
    let (id, uuid) = spawn(world, entity_type_id, position);
    world.load_stored_entity_record_for_test(
        DIMENSION,
        id,
        &[NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(1),
        }],
    );
    world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_pos(position);
    (id, uuid)
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position()
}

fn distance(world: &ServerWorld, a: i32, b: i32) -> f64 {
    let (a, b) = (position(world, a), position(world, b));
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

fn speed(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .speed()
}

fn hurt(
    world: &mut ServerWorld,
    id: i32,
    damage_type: &str,
    by: Option<CommandDamageSourceEntity>,
) {
    let outcome = world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, damage_type, 1.0, by)
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded, "{damage_type}");
}

fn panda_command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    command: &str,
) {
    let operation = world.publication().tick;
    let tick = console_command(sender, receiver, world, operation, command);
    assert!(!admin_outcome(&tick).replayed);
    assert!(console_command_answer(&tick).0.is_some(), "{command}");
}

fn summon_panda(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    gene: &str,
) -> i32 {
    panda_command(sender, receiver, world, &format!(
        "summon minecraft:panda 4.5 65 8.5 {{MainGene:\"{gene}\",HiddenGene:\"normal\",Age:6000,PersistenceRequired:1b}}"
    ));
    let id = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIMENSION, *id)
                .is_some_and(|body| {
                    body.entity_type_id()
                        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PANDA_ID
                })
        })
        .unwrap();
    assert!(
        matches!(world.stored_entity_for_test(DIMENSION, id).unwrap().data(),
        EntityData::GeneralLiving(data) if !data.is_no_ai())
    );
    id
}

#[test]
fn worried_pandas_really_flee_players_and_monsters_while_normal_pandas_walk() {
    for player_threat in [true, false] {
        for gene in ["normal", "worried"] {
            let (sender, receiver, mut world) =
                fixture(&["time set midnight", "gamerule advance_time false"]);
            world.tick_rate_mut().set_frozen(true);
            world.set_player_position_for_test(
                ALICE,
                Vec3d {
                    x: if player_threat { 8.5 } else { 12.5 },
                    y: 65.0,
                    z: 8.5,
                },
            );
            if player_threat {
                world.set_player_mode_and_abilities_for_test(
                    ALICE,
                    GameType::Survival,
                    PlayerAbilities::for_game_type(GameType::Survival),
                );
            }
            let panda = summon_panda(&sender, &receiver, &mut world, gene);
            let threat = if player_threat {
                world.publication().players[0].position
            } else {
                let at = Vec3d {
                    x: 6.5,
                    y: 65.0,
                    z: 8.5,
                };
                spawn_still(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID, at);
                at
            };
            let distance_from = |at: Vec3d| (at.x - threat.x).hypot(at.z - threat.z);
            let start_distance = distance_from(position(&world, panda));
            let mut ran = false;
            let mut escaped = false;
            world.tick_rate_mut().set_frozen(false);
            for _ in 0..200 {
                world.tick(receiver.drain());
                let at = position(&world, panda);
                ran |= speed(&world, panda) > 0.25;
                escaped |= distance_from(at) > start_distance + 2.0;
                if gene == "worried" && ran && escaped {
                    break;
                }
            }
            let last = position(&world, panda);
            if gene == "worried" {
                assert!(ran && escaped, "player={player_threat}: {last:?}");
            } else {
                assert!(
                    !ran,
                    "a normal panda keeps ordinary walking speed: {last:?}"
                );
            }
        }
    }
}

#[test]
fn a_panicking_panda_stops_walking_after_real_bamboo_feeding_seats_it() {
    let (sender, receiver, mut world) =
        fixture(&["time set midnight", "gamerule advance_time false"]);
    world.tick_rate_mut().set_frozen(true);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 12.5,
            y: 65.0,
            z: 12.5,
        },
    );
    let panda = summon_panda(&sender, &receiver, &mut world, "normal");
    hurt(&mut world, panda, "minecraft:in_fire", None);
    let start = position(&world, panda);
    let mut running = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..200 {
        let before = position(&world, panda);
        world.tick(receiver.drain());
        let at = position(&world, panda);
        if speed(&world, panda) > 0.25
            && (at.x - before.x).hypot(at.z - before.z) > 0.01
            && (at.x - start.x).hypot(at.z - start.z) > 2.0
        {
            running = true;
            break;
        }
    }
    assert!(
        running,
        "the actual environmental injury first causes a real escape"
    );
    world.tick_rate_mut().set_frozen(true);
    let at = position(&world, panda);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            z: at.z + 2.0,
            ..at
        },
    );
    world.set_player_mode_and_abilities_for_test(
        ALICE,
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    world.set_inventory_slot_for_test(
        ALICE,
        0,
        ItemStackSummary {
            item_id: Some(
                bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:bamboo")
                    .unwrap()
                    .id,
            ),
            count: 2,
            ..ItemStackSummary::empty()
        },
    );
    let fed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(bbb_protocol::packets::InteractEntity {
            entity_id: panda,
            hand: bbb_protocol::packets::InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    assert_eq!(fed.snapshot.players[0].inventory.slots[0].count, 1);
    let sitting = |world: &ServerWorld| {
        matches!(world.stored_entity_for_test(DIMENSION, panda).unwrap().data(),
        EntityData::GeneralLiving(data) if !data.is_no_ai()
            && data.panda_retained().is_some_and(|state| state.is_sitting()))
    };
    assert!(sitting(&world));
    let health = world
        .stored_entity_for_test(DIMENSION, panda)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let mut seated_packet = fed.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        if packet.id == panda && packet.values.iter().any(|row|
            row.data_id == bbb_protocol::entity_synched_data::VANILLA_PANDA_FLAGS_DATA_ID
                && matches!(row.value, bbb_protocol::packets::EntityDataValueKind::Byte(flags) if flags & 8 != 0))));
    let mut settled = at;
    world.tick_rate_mut().set_frozen(false);
    for tick_index in 0..50 {
        let tick = world.tick(receiver.drain());
        seated_packet |= tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
            if packet.id == panda && packet.values.iter().any(|row|
                row.data_id == bbb_protocol::entity_synched_data::VANILLA_PANDA_FLAGS_DATA_ID
                    && matches!(row.value, bbb_protocol::packets::EntityDataValueKind::Byte(flags) if flags & 8 != 0))));
        assert!(sitting(&world));
        assert_eq!(
            world
                .stored_entity_for_test(DIMENSION, panda)
                .unwrap()
                .living()
                .unwrap()
                .health(),
            health
        );
        if tick_index == 10 {
            settled = position(&world, panda);
        }
    }
    let end = position(&world, panda);
    assert!(
        (end.x - settled.x).hypot(end.z - settled.z) < 0.25,
        "seated walking resumed: {settled:?} -> {end:?}"
    );
    assert!(
        seated_packet,
        "the real feeder receives the seated Panda flag"
    );
}

/// `Creeper.java:67-68` and `AbstractSkeleton.java:79`: each walks off once
/// the cat or wolf is within six blocks.
#[test]
fn a_creeper_walks_away_from_a_cat_and_a_skeleton_from_a_wolf() {
    let (_sender, receiver, mut world) = fixture(&["time set midnight"]);
    let y = 65.0;
    let (creeper, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        Vec3d { x: 3.5, y, z: 3.5 },
    );
    let (cat, _) = spawn_still(
        &mut world,
        VANILLA_ENTITY_TYPE_CAT_ID,
        Vec3d { x: 5.5, y, z: 3.5 },
    );
    let (skeleton, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_SKELETON_ID,
        Vec3d { x: 3.5, y, z: 12.5 },
    );
    let (wolf, _) = spawn_still(
        &mut world,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        Vec3d { x: 5.5, y, z: 12.5 },
    );
    let (mut creeper_fled, mut skeleton_fled) = (false, false);
    for _ in 0..200 {
        world.tick(receiver.drain());
        creeper_fled |= distance(&world, creeper, cat) > 6.0;
        skeleton_fled |= distance(&world, skeleton, wolf) > 6.0;
        if creeper_fled && skeleton_fled {
            return;
        }
    }
    panic!("creeper fled: {creeper_fled}, skeleton fled: {skeleton_fled}");
}

#[test]
fn piglin_brutes_stroll_and_return_to_their_saved_home() {
    use crate::entity::ai::{BrainPacked, MemoryGlobalPos, MemoryModuleType, MemoryValue};
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID;

    let (sender, receiver, mut world) = fixture(&["summon minecraft:piglin_brute 3.5 65 8.5"]);
    world.tick_rate_mut().set_frozen(true);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 3.5,
        },
    );
    let id = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIMENSION, *id)
                .is_some_and(|entity| {
                    entity.entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
                })
        })
        .unwrap();
    let uuid = world.stored_entity_for_test(DIMENSION, id).unwrap().uuid();
    let immune = console_command(
        &sender,
        &receiver,
        &mut world,
        99,
        &format!("data merge entity {uuid} {{IsImmuneToZombification:1b}}"),
    );
    assert_eq!(console_command_answer(&immune).0, Some(1));
    let Some(NbtSummaryValue::Compound(saved)) =
        world.stored_entity_command_data_for_test(DIMENSION, id)
    else {
        panic!("a summoned brute has a saved record");
    };
    let brain = BrainPacked::decode(
        &saved
            .iter()
            .find(|entry| entry.name == "Brain")
            .unwrap()
            .value,
    )
    .unwrap();
    assert!(brain.memories.iter().any(|memory| {
        memory.memory_type == MemoryModuleType::Home
            && memory.value
                == MemoryValue::GlobalPos(MemoryGlobalPos {
                    dimension: "minecraft:overworld".into(),
                    pos: BlockPos { x: 3, y: 65, z: 8 },
                })
    }));
    let home_distance = |world: &ServerWorld| {
        let at = position(world, id);
        (at.x - 3.5).hypot(at.z - 8.5)
    };
    for (attempt, operation) in [100, 101].into_iter().enumerate() {
        if attempt > 0 {
            world.load_stored_entity_record_for_test(DIMENSION, id, &saved);
        }
        let moved = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("tp {uuid} 14.5 65 8.5"),
        );
        assert_eq!(console_command_answer(&moved).0, Some(1));
        assert!(home_distance(&world) > 10.0);
        world.tick_rate_mut().set_frozen(false);
        let mut returned = false;
        for _ in 0..240 {
            world.tick(receiver.drain());
            if home_distance(&world) <= 2.6 {
                returned = true;
                break;
            }
        }
        assert!(returned, "attempt {attempt}: {:?}", position(&world, id));
        let at_home = position(&world, id);
        let mut strolled = false;
        for _ in 0..200 {
            world.tick(receiver.drain());
            let at = position(&world, id);
            if (at.x - at_home.x).hypot(at.z - at_home.z) > 1.0 {
                strolled = true;
                break;
            }
        }
        assert!(strolled, "the idle brute strolls near its home");
        world.tick_rate_mut().set_frozen(true);
    }
}

#[test]
fn spiders_flee_idle_armadillos_but_ignore_rolling_ones() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_ARMADILLO_ID, VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID,
        VANILLA_ENTITY_TYPE_SPIDER_ID,
    };
    for spider_type in [
        VANILLA_ENTITY_TYPE_SPIDER_ID,
        VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID,
    ] {
        let (sender, receiver, mut world) = fixture(&["time set midnight"]);
        let start = Vec3d {
            x: 3.5,
            y: 65.0,
            z: 8.5,
        };
        let (spider, spider_uuid) = spawn(&mut world, spider_type, start);
        let (armadillo, armadillo_uuid) = spawn_still(
            &mut world,
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            Vec3d {
                x: 5.5,
                y: 65.0,
                z: 8.5,
            },
        );
        let base_speed = world
            .stored_entity_for_test(DIMENSION, spider)
            .unwrap()
            .living()
            .unwrap()
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            as f32;
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            30,
            &format!("data merge entity {armadillo_uuid} {{state:\"rolling\"}}"),
        );
        assert!(console_command_answer(&tick).0.is_some());
        for _ in 0..40 {
            world.tick(receiver.drain());
            assert!(
                speed(&world, spider) <= base_speed + 1.0e-6,
                "type {spider_type} must not flee a rolling armadillo"
            );
        }
        for (id, command) in [
            format!("tp {spider_uuid} 3.5 65 8.5"),
            format!("data merge entity {armadillo_uuid} {{state:\"idle\"}}"),
        ]
        .iter()
        .enumerate()
        {
            let tick = console_command(&sender, &receiver, &mut world, 31 + id as u64, command);
            assert!(console_command_answer(&tick).0.is_some(), "{command}");
        }
        let mut fast = false;
        let mut fled = false;
        for _ in 0..200 {
            world.tick(receiver.drain());
            fast |= (speed(&world, spider) - 1.2 * base_speed).abs() < 1.0e-5;
            fled |= distance(&world, spider, armadillo) > 6.0;
            if fast && fled {
                break;
            }
        }
        assert!(
            fast && fled,
            "type {spider_type}: fast={fast}, fled={fled}, at {:?}",
            position(&world, spider)
        );
    }
}

/// `PanicGoal` (`PanicGoal.java`): a pig a player hit runs at 1.25
/// times its speed; a wolf only panics at `PANIC_ENVIRONMENTAL_CAUSES`
/// (`Wolf.java:122`), at 1.5 times its speed.
#[test]
fn a_hit_pig_runs_off_and_a_wolf_panics_only_at_fire_and_lava() {
    let (_sender, receiver, mut world) = fixture(&["time set midnight"]);
    let y = 65.0;
    let start = Vec3d { x: 3.5, y, z: 3.5 };
    let (pig, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_PIG_ID, start);
    let (burnt, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        Vec3d { x: 12.5, y, z: 3.5 },
    );
    let (scalded, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        Vec3d { x: 3.5, y, z: 12.5 },
    );
    let (cursed, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        Vec3d {
            x: 12.5,
            y,
            z: 12.5,
        },
    );
    world.tick(receiver.drain());
    let alice = CommandDamageSourceEntity::Player {
        user: crate::admin::NameAndId {
            id: Uuid::from_u128(1),
            name: "alice".into(),
        }
        .into(),
        entity_id: world.player_entity_id_for_test(ALICE),
        position: Vec3d { x: 8.5, y, z: 8.5 },
        instabuild: true,
        may_build: true,
        spectator: false,
        display_name: NbtSummaryValue::string("alice"),
    };
    hurt(&mut world, pig, "minecraft:player_attack", Some(alice));
    hurt(&mut world, burnt, "minecraft:in_fire", None);
    hurt(&mut world, scalded, "minecraft:lava", None);
    hurt(&mut world, cursed, "minecraft:magic", None);

    let (mut pig_fast, mut burnt_fast, mut scalded_fast) = (false, false, false);
    let mut pig_away = 0.0_f64;
    for _ in 0..40 {
        world.tick(receiver.drain());
        // Plain walking is `1.0 * MOVEMENT_SPEED`: 0.25 for a pig, 0.3 for a wolf.
        pig_fast |= speed(&world, pig) > 0.3;
        burnt_fast |= speed(&world, burnt) > 0.4;
        scalded_fast |= speed(&world, scalded) > 0.4;
        assert!(
            speed(&world, cursed) <= 0.3 + 1.0e-6,
            "magic is no panic cause"
        );
        let now = position(&world, pig);
        pig_away = pig_away.max(((now.x - start.x).powi(2) + (now.z - start.z).powi(2)).sqrt());
    }
    assert!(
        pig_fast && burnt_fast && scalded_fast,
        "{pig_fast} {burnt_fast} {scalded_fast}"
    );
    assert!(pig_away > 2.0, "the pig ran {pig_away} blocks");
}

#[test]
fn a_hit_strider_runs_across_a_deep_lava_surface() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID;

    let (_sender, receiver, mut world) =
        fixture(&["time set midnight", "fill 0 65 0 15 71 15 minecraft:lava"]);
    let (strider, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        Vec3d {
            x: 8.5,
            y: 71.6,
            z: 8.5,
        },
    );
    for _ in 0..80 {
        world.tick(receiver.drain());
    }
    let start = position(&world, strider);
    assert!((71.5..72.0).contains(&start.y), "surface at {start:?}");
    let entity = world.stored_entity_for_test(DIMENSION, strider).unwrap();
    assert!(entity.is_in_lava());
    let living = entity.living().unwrap();
    let health = living.health();
    let base_speed = living
        .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        as f32;
    let alice = CommandDamageSourceEntity::Player {
        user: crate::admin::NameAndId {
            id: Uuid::from_u128(1),
            name: "alice".into(),
        }
        .into(),
        entity_id: world.player_entity_id_for_test(ALICE),
        position: Vec3d {
            x: 8.5,
            y: 72.0,
            z: 8.5,
        },
        instabuild: true,
        may_build: true,
        spectator: false,
        display_name: NbtSummaryValue::string("alice"),
    };
    hurt(&mut world, strider, "minecraft:player_attack", Some(alice));
    let (mut fast, mut fled) = (false, false);
    for _ in 0..80 {
        world.tick(receiver.drain());
        fast |= (speed(&world, strider) - 1.65 * base_speed).abs() < 1.0e-5;
        let entity = world.stored_entity_for_test(DIMENSION, strider).unwrap();
        let now = entity.position();
        assert!(entity.is_in_lava(), "left the pool at {now:?}");
        assert!((71.5..72.0).contains(&now.y), "left the surface at {now:?}");
        assert_eq!(entity.living().unwrap().health(), health - 1.0);
        fled |= (now.x - start.x).hypot(now.z - start.z) > 2.0;
        if fast && fled {
            break;
        }
    }
    assert!(
        fast && fled,
        "fast={fast}, fled={fled}, at {:?}",
        position(&world, strider)
    );
}

#[test]
fn hurt_turtles_seek_water_seven_blocks_away_or_flee_on_dry_land() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID;
    for (damage, burning, water) in [
        ("minecraft:mob_attack", false, true),
        ("minecraft:in_fire", true, true),
        ("minecraft:mob_attack", false, false),
    ] {
        let commands = if water {
            vec!["time set midnight", "setblock 10 65 4 minecraft:water"]
        } else {
            vec!["time set midnight"]
        };
        let (_sender, receiver, mut world) = fixture(&commands);
        let start = Vec3d {
            x: 3.5,
            y: 65.0,
            z: 4.5,
        };
        let (turtle, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_TURTLE_ID, start);
        if burning {
            load_fields(
                &mut world,
                turtle,
                &[NbtSummaryEntry {
                    name: "Fire".into(),
                    value: NbtSummaryValue::Short(1000),
                }],
            );
        }
        let base_speed = world
            .stored_entity_for_test(DIMENSION, turtle)
            .unwrap()
            .living()
            .unwrap()
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            as f32;
        hurt(&mut world, turtle, damage, None);
        let mut fast = false;
        let mut fled = false;
        for _ in 0..160 {
            world.tick(receiver.drain());
            fast |= (speed(&world, turtle) - 1.2 * base_speed).abs() < 1.0e-5;
            let entity = world.stored_entity_for_test(DIMENSION, turtle).unwrap();
            let now = entity.position();
            fled |= if water {
                now.x > 9.0 && entity.is_in_water() && entity.remaining_fire_ticks() <= 0
            } else {
                (now.x - start.x).hypot(now.z - start.z) > 2.0
            };
            if fast && fled {
                break;
            }
        }
        assert!(
            fast && fled,
            "{damage}, water={water}: fast={fast}, fled={fled}, at {:?}",
            position(&world, turtle)
        );
    }
}

#[test]
fn a_panicking_tame_wolf_teleports_to_its_owner_unless_ordered_to_sit() {
    let (_sender, receiver, mut world) = fixture(&["time set midnight"]);
    let owner_position = Vec3d {
        x: 1.5,
        y: 65.0,
        z: 8.5,
    };
    world.set_player_position_for_test(ALICE, owner_position);
    let owner = world.publication().players[0].uuid.as_u128();
    let owner_nbt = NbtSummaryValue::IntArray(
        (0..4)
            .map(|word| (owner >> (96 - 32 * word)) as u32 as i32)
            .collect(),
    );
    let mut wolves = Vec::new();
    for (sitting, z) in [(false, 4.5), (true, 12.5)] {
        let start = Vec3d {
            x: 14.5,
            y: 65.0,
            z,
        };
        let (wolf, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_WOLF_ID, start);
        load_fields(
            &mut world,
            wolf,
            &[
                NbtSummaryEntry {
                    name: "Owner".into(),
                    value: owner_nbt.clone(),
                },
                NbtSummaryEntry {
                    name: "Sitting".into(),
                    value: NbtSummaryValue::Byte(i8::from(sitting)),
                },
            ],
        );
        hurt(&mut world, wolf, "minecraft:in_fire", None);
        // Wolf.java:393-400: the hurt stands it up; order it back down.
        let crate::entity::EntityData::GeneralLiving(data) = world
            .stored_entity_for_test(DIMENSION, wolf)
            .unwrap()
            .data()
        else {
            unreachable!()
        };
        assert!(!data.tamable_retained().unwrap().ordered_to_sit());
        if sitting {
            load_fields(
                &mut world,
                wolf,
                &[NbtSummaryEntry {
                    name: "Sitting".into(),
                    value: NbtSummaryValue::Byte(1),
                }],
            );
        }
        wolves.push((wolf, sitting, start));
    }
    let mut teleported = false;
    for _ in 0..40 {
        world.tick(receiver.drain());
        for (wolf, sitting, previous) in &mut wolves {
            let now = position(&world, *wolf);
            if (now.x - previous.x).hypot(now.z - previous.z) > 8.0 {
                assert!(
                    !*sitting,
                    "a sitting wolf must not teleport while panicking"
                );
                assert!((now.x - owner_position.x).abs() <= 3.0);
                assert!((now.z - owner_position.z).abs() <= 3.0);
                assert_eq!(now.y, 65.0, "the wolf landed on the fixture floor");
                assert_eq!(now.x.fract(), 0.5);
                assert_eq!(now.z.fract(), 0.5);
                teleported = true;
            }
            *previous = now;
        }
    }
    assert!(teleported, "the standing wolf must teleport during panic");
}

/// `FleeSunGoal` (`FleeSunGoal.java`): a skeleton the noon sun set alight
/// walks under the nearby roof, and `RestrictSunGoal` keeps its paths there.
#[test]
fn a_burning_skeleton_walks_under_a_roof() {
    let (_sender, receiver, mut world) =
        fixture(&["time set noon", "fill 0 68 0 8 68 8 minecraft:stone"]);
    let (skeleton, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_SKELETON_ID,
        Vec3d {
            x: 12.5,
            y: 65.0,
            z: 4.5,
        },
    );
    let mut burned = false;
    for _ in 0..400 {
        world.tick(receiver.drain());
        let entity = world.stored_entity_for_test(DIMENSION, skeleton).unwrap();
        burned |= entity.remaining_fire_ticks() > 0;
        let at = entity.position();
        if burned && (0.0..9.0).contains(&at.x) && (0.0..9.0).contains(&at.z) {
            return;
        }
    }
    panic!("burned: {burned}, at {:?}", position(&world, skeleton));
}

/// `Squid.hurtServer` (`Squid.java:178-185`) squirts thirty ink particles at
/// a living attacker's blow, and `SquidFleeGoal` swims away from it.
#[test]
fn a_hurt_squid_squirts_ink_and_swims_away() {
    let (_sender, receiver, mut world) =
        fixture(&["time set midnight", "fill 0 65 0 15 70 15 minecraft:water"]);
    let (squid, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_SQUID_ID,
        Vec3d {
            x: 8.5,
            y: 67.0,
            z: 8.5,
        },
    );
    let zombie_position = Vec3d {
        x: 10.5,
        y: 67.0,
        z: 8.5,
    };
    let (zombie, zombie_uuid) =
        spawn_still(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID, zombie_position);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    let before = distance(&world, squid, zombie);
    hurt(
        &mut world,
        squid,
        "minecraft:mob_attack",
        Some(CommandDamageSourceEntity::Stored {
            entity_id: zombie,
            uuid: Some(zombie_uuid),
            position: Some(zombie_position),
            entity_resource_id: "minecraft:zombie".to_string(),
            is_mob: true,
            is_living_non_player: true,
            display_name: NbtSummaryValue::string("minecraft:zombie"),
        }),
    );
    let tick = world.tick(receiver.drain());
    let ink = tick
        .events
        .iter()
        .filter(|event| {
            matches!(event, WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::LevelParticles(particles),
            } if particles.particle.particle_type_id == VANILLA_PARTICLE_TYPE_SQUID_INK_ID)
        })
        .count();
    assert_eq!(ink, 30);
    tick_until(&mut world, 100, |world| {
        distance(world, squid, zombie) > before + 3.0
    });
}

fn load_fields(world: &mut ServerWorld, id: i32, fields: &[NbtSummaryEntry]) {
    let NbtSummaryValue::Compound(mut record) = world
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap()
    else {
        panic!("entity record");
    };
    record.retain(|entry| !fields.iter().any(|field| field.name == entry.name));
    record.extend_from_slice(fields);
    world.load_stored_entity_record_for_test(DIMENSION, id, &record);
}

#[test]
fn hurt_camels_run_at_quadruple_speed_and_stand_up_from_saved_sitting_pose() {
    use bbb_protocol::entity_poses::{
        VANILLA_ENTITY_POSE_SITTING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
    };
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID,
    };
    let (_sender, receiver, mut world) = fixture(&["time set midnight"]);
    while world.game_time() < 160 {
        world.tick(receiver.drain());
    }
    for (entity_type, z) in [
        (VANILLA_ENTITY_TYPE_CAMEL_ID, 3.5),
        (VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, 12.5),
    ] {
        let start = Vec3d { x: 4.5, y: 65.0, z };
        let (id, _) = spawn(&mut world, entity_type, start);
        load_fields(
            &mut world,
            id,
            &[NbtSummaryEntry {
                name: "LastPoseTick".into(),
                value: NbtSummaryValue::Long(-100),
            }],
        );
        assert_eq!(
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .pose_id(),
            VANILLA_ENTITY_POSE_SITTING_ID
        );
        let base_speed = world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .living()
            .unwrap()
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            as f32;
        hurt(&mut world, id, "minecraft:mob_attack", None);
        let mut fast = false;
        let mut moved = false;
        for _ in 0..40 {
            world.tick(receiver.drain());
            fast |= (speed(&world, id) - 4.0 * base_speed).abs() < 1.0e-5;
            let now = position(&world, id);
            moved |= (now.x - start.x).hypot(now.z - start.z) > 2.0;
        }
        assert!(
            fast && moved,
            "type {entity_type}: fast={fast}, moved={moved}"
        );
        assert_eq!(
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .pose_id(),
            VANILLA_ENTITY_POSE_STANDING_ID
        );
        let NbtSummaryValue::Compound(record) = world
            .stored_entity_command_data_for_test(DIMENSION, id)
            .unwrap()
        else {
            panic!("entity record");
        };
        assert!(record.iter().any(|entry| entry.name == "LastPoseTick"
            && matches!(entry.value, NbtSummaryValue::Long(tick) if tick >= 0)));
    }
}

#[test]
fn a_mob_riding_a_camel_husk_prevents_panic() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID;
    let (sender, receiver, mut world) = fixture(&["time set midnight"]);
    let start = Vec3d {
        x: 4.5,
        y: 65.0,
        z: 4.5,
    };
    let (camel, camel_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, start);
    let (_, rider_uuid) = spawn_still(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID, start);
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        30,
        &format!("ride {rider_uuid} mount {camel_uuid}"),
    );
    assert!(console_command_answer(&tick).0.is_some());
    hurt(&mut world, camel, "minecraft:mob_attack", None);
    for _ in 0..40 {
        world.tick(receiver.drain());
        assert!(speed(&world, camel) <= 0.1, "the ridden husk must not flee");
    }
    let now = position(&world, camel);
    assert!((now.x - start.x).hypot(now.z - start.z) < 1.0);
}

#[test]
fn a_burning_wide_camel_seeks_a_two_by_two_water_surface() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID;
    let (_sender, receiver, mut world) = fixture(&[
        "time set midnight",
        "setblock 4 64 6 minecraft:water",
        "fill 9 64 7 10 64 8 minecraft:water",
    ]);
    let (camel, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 8.5,
        },
    );
    load_fields(
        &mut world,
        camel,
        &[NbtSummaryEntry {
            name: "Fire".into(),
            value: NbtSummaryValue::Short(1000),
        }],
    );
    hurt(&mut world, camel, "minecraft:in_fire", None);
    let mut reached_pool = false;
    for _ in 0..100 {
        world.tick(receiver.drain());
        let entity = world.stored_entity_for_test(DIMENSION, camel).unwrap();
        reached_pool |= entity.position().x > 8.0 && entity.remaining_fire_ticks() <= 0;
        if reached_pool {
            break;
        }
    }
    assert!(reached_pool, "camel at {:?}", position(&world, camel));
}

#[test]
fn a_hurt_goat_runs_at_double_speed() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID;
    let (_sender, receiver, mut world) = fixture(&["time set midnight"]);
    let start = Vec3d {
        x: 4.5,
        y: 65.0,
        z: 4.5,
    };
    let (goat, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_GOAT_ID, start);
    let base_speed = world
        .stored_entity_for_test(DIMENSION, goat)
        .unwrap()
        .living()
        .unwrap()
        .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        as f32;
    hurt(&mut world, goat, "minecraft:mob_attack", None);
    let mut fast = false;
    let mut moved = false;
    for _ in 0..40 {
        world.tick(receiver.drain());
        fast |= (speed(&world, goat) - 2.0 * base_speed).abs() < 1.0e-5;
        let now = position(&world, goat);
        moved |= (now.x - start.x).hypot(now.z - start.z) > 2.0;
    }
    assert!(fast && moved, "fast={fast}, moved={moved}");
}

#[test]
fn a_burning_goat_seeks_a_single_water_block_and_extinguishes() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID;
    let (_sender, receiver, mut world) = fixture(&[
        "time set midnight",
        "fill 3 64 5 5 64 7 minecraft:stone_slab[type=bottom]",
        "setblock 4 64 6 minecraft:water",
    ]);
    let (goat, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_GOAT_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 8.5,
        },
    );
    load_fields(
        &mut world,
        goat,
        &[NbtSummaryEntry {
            name: "Fire".into(),
            value: NbtSummaryValue::Short(1000),
        }],
    );
    hurt(&mut world, goat, "minecraft:in_fire", None);
    let mut extinguished_in_water = false;
    for _ in 0..100 {
        world.tick(receiver.drain());
        let entity = world.stored_entity_for_test(DIMENSION, goat).unwrap();
        let at = entity.position();
        extinguished_in_water |=
            (at.x - 4.5).hypot(at.z - 6.5) < 1.0 && entity.remaining_fire_ticks() <= 0;
        if extinguished_in_water {
            break;
        }
    }
    assert!(
        extinguished_in_water,
        "goat at {:?}, fire={}",
        position(&world, goat),
        world
            .stored_entity_for_test(DIMENSION, goat)
            .unwrap()
            .remaining_fire_ticks()
    );
}

#[test]
fn adult_and_baby_armadillos_flee_only_environmental_damage_at_double_speed() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMADILLO_ID;
    for age in [0, -24_000] {
        let (_sender, receiver, mut world) = fixture(&["time set midnight"]);
        let mut cases = Vec::new();
        for (damage, x, z, panics) in [
            ("minecraft:in_fire", 3.5, 3.5, true),
            ("minecraft:lava", 12.5, 3.5, true),
            ("minecraft:magic", 3.5, 12.5, false),
            ("minecraft:player_attack", 12.5, 12.5, false),
        ] {
            let start = Vec3d { x, y: 65.0, z };
            let (id, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_ARMADILLO_ID, start);
            load_fields(
                &mut world,
                id,
                &[NbtSummaryEntry {
                    name: "Age".into(),
                    value: NbtSummaryValue::Int(age),
                }],
            );
            let base_speed = world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .living()
                .unwrap()
                .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
                as f32;
            hurt(&mut world, id, damage, None);
            cases.push((id, damage, start, base_speed, panics, false, false));
        }
        for _ in 0..40 {
            world.tick(receiver.drain());
            for (id, damage, start, base_speed, panics, fast, moved) in &mut cases {
                let current_speed = speed(&world, *id);
                if *panics {
                    *fast |= (current_speed - 2.0 * *base_speed).abs() < 1.0e-5;
                    let now = position(&world, *id);
                    *moved |= (now.x - start.x).hypot(now.z - start.z) > 1.0;
                } else {
                    assert!(
                        current_speed <= *base_speed + 1.0e-6,
                        "age={age}, {damage}: speed={current_speed}"
                    );
                }
            }
        }
        for (_, damage, _, _, panics, fast, moved) in cases {
            if panics {
                assert!(
                    fast && moved,
                    "age={age}, {damage}: fast={fast}, moved={moved}"
                );
            }
        }
    }
}

#[test]
fn a_burning_armadillo_seeks_a_single_water_block_and_extinguishes() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMADILLO_ID;
    let (_sender, receiver, mut world) = fixture(&[
        "time set midnight",
        "fill 3 64 5 5 64 7 minecraft:stone_slab[type=bottom]",
        "setblock 4 64 6 minecraft:water",
    ]);
    let (armadillo, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ARMADILLO_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 8.5,
        },
    );
    load_fields(
        &mut world,
        armadillo,
        &[NbtSummaryEntry {
            name: "Fire".into(),
            value: NbtSummaryValue::Short(1000),
        }],
    );
    hurt(&mut world, armadillo, "minecraft:in_fire", None);
    for _ in 0..100 {
        world.tick(receiver.drain());
        let entity = world.stored_entity_for_test(DIMENSION, armadillo).unwrap();
        let at = entity.position();
        if (at.x - 4.5).hypot(at.z - 6.5) < 1.0 && entity.remaining_fire_ticks() <= 0 {
            return;
        }
    }
    panic!(
        "armadillo at {:?}, fire={}",
        position(&world, armadillo),
        world
            .stored_entity_for_test(DIMENSION, armadillo)
            .unwrap()
            .remaining_fire_ticks()
    );
}

#[test]
fn zombies_and_drowned_approach_and_stomp_whole_turtle_egg_blocks() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DROWNED_ID;
    use bbb_protocol::packets::SoundEventHolder;
    use bbb_protocol::sound_event_types::vanilla_sound_event_resource_id;

    let egg_pos = BlockPos { x: 11, y: 65, z: 8 };
    let eggs = crate::fluid::with_property(
        crate::block_behavior::default_block_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_TURTLE_EGG_ID,
        ),
        "eggs",
        "4",
    )
    .unwrap();
    for entity_type in [
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_DROWNED_ID,
    ] {
        for griefing in [false, true] {
            let (sender, receiver, mut world) = fixture(&[
                "time set midnight",
                if griefing {
                    "gamerule mob_griefing true"
                } else {
                    "gamerule mob_griefing false"
                },
            ]);
            world.set_player_position_for_test(
                ALICE,
                Vec3d {
                    x: 8.5,
                    y: 65.0,
                    z: 3.5,
                },
            );
            assert!(world.set_block(DIMENSION, egg_pos, eggs, SET_BLOCK_UPDATE_ALL));
            let start = Vec3d {
                x: 3.5,
                y: 65.0,
                z: 8.5,
            };
            let (id, _) = spawn(&mut world, entity_type, start);
            let mut progress_sound = false;
            let mut break_sound = false;
            let mut jumped = false;
            let mut approached = false;
            let mut last_egg_count = 4;
            let mut removed = false;
            for _ in 0..if griefing { 1800 } else { 400 } {
                let tick = world.tick(receiver.drain());
                for event in tick.events {
                    match event {
                        WorldEvent::Send {
                            connection_id: ALICE,
                            packet: ClientboundPlayPacket::KeepAlive { id },
                        } => {
                            enqueue(
                                &sender,
                                WorldCommand::Play {
                                    connection_id: ALICE,
                                    packet: PlayServerbound::KeepAlive { id },
                                },
                            )
                            .unwrap();
                        }
                        WorldEvent::Send {
                            packet: ClientboundPlayPacket::Sound(sound),
                            ..
                        } => {
                            if let SoundEventHolder::Reference { registry_id } = sound.sound {
                                match vanilla_sound_event_resource_id(registry_id) {
                                    Some("minecraft:entity.zombie.destroy_egg") => {
                                        progress_sound = true
                                    }
                                    Some("minecraft:entity.turtle.egg_break") => break_sound = true,
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                let mob = world.stored_entity_for_test(DIMENSION, id).unwrap();
                approached |= mob.position().x > 9.5;
                jumped |= progress_sound && mob.position().y > 65.5;
                let state = world.block_state(DIMENSION, egg_pos).unwrap();
                if bbb_server_types::fluid::block_state_is_air(state) {
                    removed = true;
                    break;
                }
                last_egg_count =
                    bbb_server_types::block_queries::int_property(state, "eggs").unwrap();
            }
            assert_eq!(removed, griefing, "type={entity_type}, griefing={griefing}");
            assert_eq!(progress_sound, griefing);
            assert_eq!(break_sound, griefing);
            if griefing {
                assert!(
                    approached && jumped,
                    "type={entity_type}, approached={approached}, jumped={jumped}"
                );
                assert!(
                    last_egg_count >= 2,
                    "the goal removes the whole remaining block, not one egg"
                );
            }
            assert!(!world
                .stored_entity_ids_for_test(DIMENSION)
                .into_iter()
                .any(|id| {
                    world
                        .stored_entity_for_test(DIMENSION, id)
                        .unwrap()
                        .entity_type_id()
                        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID
                }));
        }
    }
}
