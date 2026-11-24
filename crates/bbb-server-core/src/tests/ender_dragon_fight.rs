//! `EnderDragonFight` (`EnderDragonFight.java`) in a generated End: the
//! dragon and its bar, part hits, crystal healing, the death that opens the
//! exit portal, and the fight's saved data.

use super::*;

use bbb_protocol::block_types::{
    VANILLA_BLOCK_BEDROCK_ID, VANILLA_BLOCK_DRAGON_EGG_ID, VANILLA_BLOCK_END_GATEWAY_ID,
    VANILLA_BLOCK_END_PORTAL_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_END_CRYSTAL_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
};
use bbb_protocol::packets::{BossBarColor, BossEventOperation};

const END: Java26BuiltinDimension = Java26BuiltinDimension::End;

/// A creative operator standing by the End's main island, ticked until the
/// fight has found the arena and summoned its dragon.
fn end_fight() -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    Vec<WorldEvent>,
) {
    end_fight_at(Vec3d {
        x: 8.5,
        y: 120.0,
        z: 0.5,
    })
}

fn end_fight_at(
    position: Vec3d,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    Vec<WorldEvent>,
) {
    end_fight_prepared(position, |_| {})
}

fn end_fight_prepared(
    position: Vec3d,
    prepare: impl FnOnce(&mut ServerWorld),
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    Vec<WorldEvent>,
) {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = datapack_config().datapacks;
    // The sequences outlast a keep-alive round the fixture never answers.
    config.keep_alive_interval_ticks = 100_000;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode creative");
    prepare(&mut world);
    let mut events = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: END.into(),
            position,
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut events,
    ));
    for _ in 0..40 {
        events.extend(world.tick(receiver.drain()).events);
        if world
            .dragon_fight_for_test(END)
            .is_some_and(|fight| fight.dragon_uuid.is_some())
        {
            break;
        }
    }
    (sender, receiver, world, events)
}

fn boss_operations(events: &[WorldEvent]) -> Vec<BossEventOperation> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::BossEvent(boss),
            } => Some(boss.operation.clone()),
            _ => None,
        })
        .collect()
}

fn dragon_id(world: &ServerWorld) -> i32 {
    let uuid = world
        .dragon_fight_for_test(END)
        .and_then(|fight| fight.dragon_uuid)
        .expect("the fight names its dragon");
    world
        .live_entity_id_for_uuid_for_test(END, uuid)
        .expect("the dragon is in the End")
}

#[test]
fn the_fight_runs_beyond_player_simulation_distance_and_releases_its_arena() {
    let (_, receiver, mut world, events) = end_fight_at(Vec3d {
        x: 176.5,
        y: 120.0,
        z: 0.5,
    });
    let dragon = dragon_id(&world);
    assert!(boss_operations(&events)
        .iter()
        .any(|op| matches!(op, BossEventOperation::Add { .. })));
    let before = world
        .stored_entity_for_test(END, dragon)
        .unwrap()
        .position();
    for _ in 0..5 {
        world.tick(receiver.drain());
    }
    assert_ne!(
        world
            .stored_entity_for_test(END, dragon)
            .unwrap()
            .position(),
        before,
        "the dragon keeps flying outside the player's simulation distance"
    );
    assert!(world.entity_ticking_chunks(END).contains(&(0, 0)));

    let mut events = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: END.into(),
            position: Vec3d {
                x: 256.5,
                y: 120.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut events,
    ));
    for _ in 0..25 {
        events.extend(world.tick(receiver.drain()).events);
    }
    assert!(boss_operations(&events)
        .iter()
        .any(|op| matches!(op, BossEventOperation::Remove)));
    assert!(!world.entity_ticking_chunks(END).contains(&(0, 0)));
    let stopped = world
        .stored_entity_for_test(END, dragon)
        .unwrap()
        .position();
    for _ in 0..3 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        world
            .stored_entity_for_test(END, dragon)
            .unwrap()
            .position(),
        stopped
    );
}

fn health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(END, id)
        .and_then(|entity| entity.living().map(|living| living.health()))
        .expect("the dragon is living")
}

fn block_is(world: &mut ServerWorld, pos: BlockPos, block: i32) -> bool {
    world
        .block_state(END, pos)
        .and_then(bbb_protocol::block_state_types::vanilla_block_state_block_type_id)
        == Some(block)
}

fn exit_portal(world: &ServerWorld) -> BlockPos {
    world
        .dragon_fight_for_test(END)
        .and_then(|fight| fight.exit_portal_location)
        .expect("the fight placed its exit portal")
}

/// `scanState` and `createNewDragon` (`EnderDragonFight.java:221-255`,
/// `:464-478`): a fresh End builds the unlit exit portal and summons a dragon
/// over it, whose pink, music-playing bar reaches the player in range; a
/// melee hit on the head part costs the full damage.
#[test]
fn a_fresh_end_summons_the_dragon_and_shows_its_bar() {
    let (sender, receiver, mut world, events) = end_fight();
    let dragon = dragon_id(&world);
    let body = world.stored_entity_for_test(END, dragon).unwrap();
    let position = body.position();
    assert!(
        position.x.abs() < 1.0 && position.z.abs() < 1.0 && (position.y - 128.0).abs() < 2.0,
        "the dragon starts over the origin: {position:?}"
    );
    let added = boss_operations(&events);
    let Some(BossEventOperation::Add {
        name, color, flags, ..
    }) = added.first()
    else {
        panic!("the End player receives the dragon bar: {added:?}");
    };
    assert_eq!(*color, BossBarColor::Pink);
    assert!(flags.play_music && flags.create_world_fog && !flags.darken_screen);
    assert_eq!(
        name.component(),
        &crate::commands::translatable("entity.minecraft.ender_dragon", Vec::new())
    );
    let portal = exit_portal(&world);
    assert!(block_is(&mut world, portal, VANILLA_BLOCK_BEDROCK_ID));
    assert!(!block_is(
        &mut world,
        BlockPos { x: 1, ..portal },
        VANILLA_BLOCK_END_PORTAL_ID
    ));
    let fight = world.dragon_fight_for_test(END).unwrap();
    assert!(!fight.dragon_killed && !fight.previously_killed);
    assert_eq!(fight.gateways.len(), 20);

    // The head is the part right after the dragon's id.
    let before = health(&world, dragon);
    let head = dragon + 1;
    world.tick(receiver.drain());
    let head_position = world
        .dragon_part_box_for_test(END, head)
        .expect("the head part resolves to its dragon")
        .center();
    let mut events = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: END.into(),
            position: Vec3d {
                x: head_position.x + 1.5,
                y: head_position.y - 1.0,
                z: head_position.z,
            },
            y_rot: 90.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut events,
    ));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: head }),
    );
    let after = health(&world, dragon);
    assert!(
        before - after >= 0.99,
        "a fist on the head costs its whole damage: {before} -> {after}"
    );
}

/// `hasActiveExitPortal` (`EnderDragonFight.java:291-305`): an end gateway
/// block entity anywhere in the arena's 17x17 chunks marks a dragon that was
/// already killed, so the first scan summons no dragon.
#[test]
fn a_gateway_away_from_the_podium_marks_the_dragon_already_killed() {
    let gateway = BlockPos {
        x: 100,
        y: 75,
        z: 0,
    };
    let (_, _, world, _) = end_fight_prepared(
        Vec3d {
            x: 8.5,
            y: 120.0,
            z: 0.5,
        },
        |world| {
            world.ensure_entity_transfer_full_chunk_for_test(END, (gateway.x >> 4, gateway.z >> 4));
            world.set_block(
                END,
                gateway,
                bbb_protocol::block_state_types::vanilla_block_default_state_id(
                    VANILLA_BLOCK_END_GATEWAY_ID,
                )
                .unwrap(),
                crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
            );
        },
    );
    let fight = world.dragon_fight_for_test(END).unwrap();
    assert!(fight.previously_killed && fight.dragon_killed);
    assert_eq!(fight.dragon_uuid, None);
}

/// `EnderDragon.tickDeath` into `setDragonKilled` (`EnderDragon.java:505-553`,
/// `EnderDragonFight.java:400-417`): 200 ticks after its death the first
/// dragon leaves 12000 experience, lights the exit portal, lays the egg on it,
/// opens one gateway and hides the bar; the fight's saved data says so and
/// round-trips through a restore.
#[test]
fn the_first_dragon_death_lights_the_portal_lays_the_egg_and_opens_a_gateway() {
    let (_sender, receiver, mut world, _) = end_fight();
    let dragon = dragon_id(&world);
    let gateway = *world
        .dragon_fight_for_test(END)
        .unwrap()
        .gateways
        .last()
        .unwrap();
    crate::entity::living::set_health(world.stored_entity_mut_for_test(END, dragon).unwrap(), 0.0);
    let mut hidden = false;
    let dies = |world: &ServerWorld| {
        world
            .positioned_game_events_for_test(END)
            .iter()
            .filter(|event| {
                event.recorded.game_event == crate::level_event::VanillaGameEvent::ENTITY_DIE
                    && event.recorded.source_entity_id == Some(dragon)
            })
            .count()
    };
    let mut died_at = Vec::new();
    for tick in 0..205 {
        let before = dies(&world);
        hidden |= boss_operations(&world.tick(receiver.drain()).events)
            .contains(&BossEventOperation::Remove);
        if dies(&world) > before {
            died_at.push(tick);
        }
    }
    // EnderDragon.java:551: the end of the death animation raises a second
    // ENTITY_DIE after the one `LivingEntity.die` raised.
    assert!(
        died_at.iter().any(|&tick| tick >= 150),
        "ENTITY_DIE ticks: {died_at:?}"
    );
    assert!(world.stored_entity_for_test(END, dragon).is_none());
    assert!(hidden, "the bar hides once the dragon is dead");
    let experience: i32 = world
        .stored_experience_orbs_for_test(END)
        .iter()
        .map(|orb| orb.2 * orb.3.max(1))
        .sum();
    assert!(
        experience >= 12000,
        "the first kill pays 12000 experience, got {experience}"
    );
    let _ = VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID;

    let portal = exit_portal(&world);
    assert!(block_is(
        &mut world,
        BlockPos { x: 1, ..portal },
        VANILLA_BLOCK_END_PORTAL_ID
    ));
    assert!(block_is(
        &mut world,
        BlockPos {
            y: portal.y + 4,
            ..portal
        },
        VANILLA_BLOCK_DRAGON_EGG_ID
    ));
    let angle = 2.0 * (-std::f64::consts::PI + std::f64::consts::PI / 20.0 * f64::from(gateway));
    let gateway_pos = BlockPos {
        x: (96.0 * angle.cos()).floor() as i32,
        y: 75,
        z: (96.0 * angle.sin()).floor() as i32,
    };
    assert!(block_is(
        &mut world,
        gateway_pos,
        VANILLA_BLOCK_END_GATEWAY_ID
    ));

    let fight = world.dragon_fight_for_test(END).unwrap().clone();
    assert!(fight.dragon_killed && fight.previously_killed);
    assert_eq!(fight.gateways.len(), 19);
    let request = world
        .save_request(0, 1, bbb_server_persistence::RegionCompression::Zlib)
        .unwrap();
    let saved = request
        .ender_dragon_fights
        .get(&level_key(END))
        .expect("the End files its fight");
    assert_eq!(saved, &fight);
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate
        .ender_dragon_fights
        .insert(level_key(END), saved.clone());
    let restored = ServerWorld::restore(datapack_config(), &candidate).unwrap();
    assert_eq!(restored.dragon_fight_for_test(END), Some(&fight));
}

/// `EnderDragon.checkCrystals` and `onCrystalDestroyed`
/// (`EnderDragon.java:351-375`, `:802-815`): a crystal within 32 blocks heals
/// the dragon, and breaking it blows 10 damage into the dragon's head.
#[test]
fn a_crystal_heals_the_dragon_until_it_is_broken() {
    let (sender, receiver, mut world, _) = end_fight();
    let dragon = dragon_id(&world);
    let entity = world.stored_entity_mut_for_test(END, dragon).unwrap();
    let max = entity.living().unwrap().max_health();
    crate::entity::living::set_health(entity, max - 50.0);
    let position = entity.position();
    // Out of the crystal's own blast radius, inside the dragon's 32-block search.
    let crystal_position = Vec3d {
        x: position.x + 20.0,
        y: position.y,
        z: position.z,
    };
    let (crystal, _) = world.spawn_stored_entity_for_test(
        END,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        crystal_position,
        crate::entity::EntityData::EndCrystal(crate::entity::EndCrystalData::new(0, true)),
    );
    // NoAI keeps the dragon by the crystal; it still heals (`aiStep` checks
    // crystals before the AI gate).
    let NbtSummaryValue::Compound(mut record) = world
        .stored_entity_command_data_for_test(END, dragon)
        .unwrap()
    else {
        unreachable!()
    };
    record.retain(|entry| entry.name != "NoAI");
    record.push(NbtSummaryEntry {
        name: "NoAI".into(),
        value: NbtSummaryValue::Byte(1),
    });
    world.load_stored_entity_record_for_test(END, dragon, &record);
    let start = health(&world, dragon);
    for _ in 0..200 {
        world.tick(receiver.drain());
    }
    let healed = health(&world, dragon);
    assert!(healed > start, "the crystal heals: {start} -> {healed}");

    let mut events = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: END.into(),
            position: Vec3d {
                x: crystal_position.x + 2.0,
                y: crystal_position.y,
                z: crystal_position.z,
            },
            y_rot: 90.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut events,
    ));
    world.tick(receiver.drain());
    let before = health(&world, dragon);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: crystal }),
    );
    world.tick(receiver.drain());
    assert!(world.stored_entity_for_test(END, crystal).is_none());
    let after = health(&world, dragon);
    assert!(
        before - after >= 9.0,
        "the broken crystal costs the head 10: {before} -> {after}"
    );
    // EnderDragon.java:802-808: the creative breaker, not the nearest
    // targetable player, is blamed.
    assert_eq!(
        world
            .stored_entity_for_test(END, dragon)
            .unwrap()
            .living()
            .unwrap()
            .last_damage_source_entity(),
        Some(world.player_entity_id_for_test(ConnectionId(1)))
    );
}

/// `EndCrystal.tick` (`EndCrystal.java:53-62`): in a level with a dragon
/// fight a crystal relights the air it stands in.
#[test]
fn an_end_crystal_keeps_fire_burning_in_its_block() {
    let (_sender, receiver, mut world, _) = end_fight();
    let base = BlockPos {
        x: 30,
        y: 100,
        z: 30,
    };
    let fire_pos = BlockPos { y: 101, ..base };
    world.set_block(
        END,
        base,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(VANILLA_BLOCK_BEDROCK_ID)
            .unwrap(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    world.spawn_stored_entity_for_test(
        END,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        Vec3d {
            x: 30.5,
            y: 101.0,
            z: 30.5,
        },
        crate::entity::EntityData::EndCrystal(crate::entity::EndCrystalData::new(0, true)),
    );
    world.tick(receiver.drain());
    assert!(block_is(
        &mut world,
        fire_pos,
        bbb_protocol::block_types::VANILLA_BLOCK_FIRE_ID
    ));
}

/// `teleport_player` with the client's confirmation, so block use is not
/// refused while the teleport is outstanding.
fn teleport(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    position: Vec3d,
) {
    let mut events = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: END.into(),
            position,
            y_rot: 0.0,
            x_rot: 90.0,
            relatives_mask: 0,
        },
        &mut events,
    ));
    let id = events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .expect("the teleport is sent");
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::AcceptTeleportation { id },
    );
}

/// `EnderDragon.kill`, `EndCrystalItem.useOn` into `tryRespawn` and the
/// `DragonRespawnStage` sequence (`EnderDragon.java:495-502`,
/// `EnderDragonFight.java:518-572`, `DragonRespawnStage.java`): `/kill` ends
/// the fight at once; a crystal on each side of the lit portal douses it,
/// beams at the spawn point and, once the pillars are rebuilt, summons a new
/// dragon and spends the crystals.
#[test]
fn four_crystals_on_the_exit_portal_summon_a_new_dragon() {
    let (sender, receiver, mut world, _) = end_fight();
    assert!(!world.player_advancement_is_done(ConnectionId(1), "minecraft:end/respawn_dragon"));
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let first = dragon_id(&world);
    let first_uuid = world.stored_entity_for_test(END, first).unwrap().uuid();
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "kill @e[type=minecraft:ender_dragon]",
    );
    assert!(world.stored_entity_for_test(END, first).is_none());
    assert!(world.dragon_fight_for_test(END).unwrap().dragon_killed);
    let portal = exit_portal(&world);
    assert!(block_is(
        &mut world,
        BlockPos { x: 1, ..portal },
        VANILLA_BLOCK_END_PORTAL_ID
    ));

    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        bbb_protocol::packets::ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_END_CRYSTAL_ID),
            count: 4,
            component_patch: Default::default(),
        },
    );
    for (sequence, (dx, dz)) in [(3, 0), (-3, 0), (0, 3), (0, -3)].into_iter().enumerate() {
        let ring = BlockPos {
            x: portal.x + dx,
            y: portal.y,
            z: portal.z + dz,
        };
        teleport(
            &sender,
            &receiver,
            &mut world,
            Vec3d {
                x: f64::from(ring.x) + 0.5 + f64::from(dx.signum()) * 2.0,
                y: f64::from(ring.y) + 1.0,
                z: f64::from(ring.z) + 0.5 + f64::from(dz.signum()) * 2.0,
            },
        );
        right_click(&sender, &receiver, &mut world, 1, ring, sequence as i32 + 1);
    }
    let fight = world.dragon_fight_for_test(END).unwrap().clone();
    assert_eq!(fight.respawn_crystals.len(), 4, "{fight:?}");
    assert!(!block_is(
        &mut world,
        BlockPos { x: 1, ..portal },
        VANILLA_BLOCK_END_PORTAL_ID
    ));
    let crystals: Vec<i32> = fight
        .respawn_crystals
        .iter()
        .map(|&uuid| world.live_entity_id_for_uuid_for_test(END, uuid).unwrap())
        .collect();
    world.tick(receiver.drain());
    for &crystal in &crystals {
        let crate::entity::EntityData::EndCrystal(data) =
            world.stored_entity_for_test(END, crystal).unwrap().data()
        else {
            unreachable!()
        };
        assert_eq!(data.beam_target(), Some(BlockPos { x: 0, y: 128, z: 0 }));
    }

    for _ in 0..700 {
        world.tick(receiver.drain());
        if world
            .dragon_fight_for_test(END)
            .is_some_and(|fight| !fight.dragon_killed)
        {
            break;
        }
    }
    let fight = world.dragon_fight_for_test(END).unwrap().clone();
    assert!(
        !fight.dragon_killed && fight.respawn_stage.is_none(),
        "{fight:?}"
    );
    assert_ne!(fight.dragon_uuid, Some(first_uuid));
    dragon_id(&world);
    assert!(world.player_advancement_is_done(ConnectionId(1), "minecraft:end/respawn_dragon"));
    assert!(!world.player_advancement_is_done(ConnectionId(2), "minecraft:end/respawn_dragon"));
    let tick = world.tick(receiver.drain());
    assert!(
        boss_operations(&tick.events)
            .iter()
            .any(|operation| matches!(operation, BossEventOperation::Add { .. })),
        "the bar comes back with the new dragon"
    );
    assert!(crystals
        .iter()
        .all(|&crystal| world.stored_entity_for_test(END, crystal).is_none()));
}
