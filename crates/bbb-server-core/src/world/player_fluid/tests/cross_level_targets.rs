use super::*;
use crate::admin::{AdminEnvelope, AdminRequest, AdminResult, AdminScope};
use crate::entity::{EntityData, GeneralLivingData};
use crate::world::{WorldCommand, WorldEvent};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_FOX_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_server_types::save::Java26BuiltinDimension;
use uuid::Uuid;

const OW: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const NETHER: Java26BuiltinDimension = Java26BuiltinDimension::Nether;
fn command(
    world: &mut ServerWorld,
    sender: &crate::mailbox::WorldCommandSender,
    receiver: &crate::mailbox::WorldCommandReceiver,
    id: u64,
    text: &str,
) {
    sender
        .try_send(WorldCommand::Admin {
            envelope: AdminEnvelope {
                operation_id: id,
                request_id: id.to_string(),
                principal: "cross-level-test".into(),
                scope: AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 1_786_000_007_000,
            },
            request: AdminRequest::ExecuteCommand {
                command: text.into(),
            },
        })
        .unwrap();
    let tick = world.tick(receiver.drain());
    let succeeded = tick.events.iter().any(|event| {
        let WorldEvent::Admin(outcome) = event else {
            return false;
        };
        let Ok(AdminResult::Command(report)) = &outcome.result else {
            return false;
        };
        report
            .result
            .is_some_and(|n| n > 0 || text == "tick unfreeze" && n == 0)
    });
    assert!(succeeded, "{text}: {:?}", tick.events);
}
fn move_level(world: &mut ServerWorld, dimension: Java26BuiltinDimension, position: Vec3d) {
    world.ensure_entity_transfer_full_chunk_for_test(
        dimension,
        (
            position.x.floor() as i32 >> 4,
            position.z.floor() as i32 >> 4,
        ),
    );
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: dimension.into(),
            position,
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0
        },
        &mut Vec::new()
    ));
    let id = world.levels[ConnectionId(1)].awaiting_teleport.unwrap();
    world.apply_play(
        ConnectionId(1),
        PlayServerbound::AcceptTeleportation { id },
        &mut Vec::new(),
    );
    assert!(!world.levels[ConnectionId(1)].is_changing_dimension);
}
fn spawn(
    world: &mut ServerWorld,
    dim: Java26BuiltinDimension,
    kind: i32,
    position: Vec3d,
) -> (i32, Uuid) {
    world.spawn_stored_entity_for_test(
        dim,
        kind,
        position,
        EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
    )
}
fn uuid_snbt(uuid: Uuid) -> String {
    let words: [i32; 4] = [96, 64, 32, 0].map(|shift| (uuid.as_u128() >> shift) as i32);
    format!("[I;{},{},{},{}]", words[0], words[1], words[2], words[3])
}
fn remote_player(
    world: &ServerWorld,
    dim: Java26BuiltinDimension,
) -> crate::entity::level::NearbyPlayer {
    let Some(player) = world
        .level(dim)
        .cross_level_entities
        .target_input(Uuid::from_u128(1))
    else {
        panic!("remote session input missing in {dim:?}")
    };
    *player
}
fn floor(world: &mut ServerWorld, dim: Java26BuiltinDimension, position: Vec3d) {
    for x in -2..=2 {
        for z in -2..=2 {
            world.ensure_entity_transfer_full_chunk_for_test(dim, (x, z));
        }
    }
    world.ensure_entity_transfer_full_chunk_for_test(
        dim,
        (
            position.x.floor() as i32 >> 4,
            position.z.floor() as i32 >> 4,
        ),
    );
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    for x in position.x.floor() as i32 - 4..=position.x.floor() as i32 + 4 {
        for z in position.z.floor() as i32 - 4..=position.z.floor() as i32 + 4 {
            world
                .level_mut(dim)
                .chunks
                .set_block_state(x, 79, z, state)
                .unwrap();
            for y in 80..=84 {
                world
                    .level_mut(dim)
                    .chunks
                    .set_block_state(x, y, z, 0)
                    .unwrap();
            }
        }
    }
}
#[test]
fn cross_level_targets_fox_defends_actual_remote_session_hurt_memory() {
    let (mut world, sender, receiver) = world_with_mailbox();
    for _ in 0..80 {
        world.tick(receiver.drain());
    }
    command(&mut world, &sender, &receiver, 1, "tick freeze");
    let position = world.levels[ConnectionId(1)].position();
    floor(&mut world, OW, position);
    command(
        &mut world,
        &sender,
        &receiver,
        90,
        &format!(
            "execute in minecraft:overworld run forceload add {} {}",
            position.x.floor(),
            position.z.floor()
        ),
    );
    floor(&mut world, NETHER, position);
    let (fox, fox_uuid) = spawn(
        &mut world,
        OW,
        VANILLA_ENTITY_TYPE_FOX_ID,
        Vec3d {
            x: position.x + 3.0,
            ..position
        },
    );
    move_level(&mut world, NETHER, position);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    command(
        &mut world,
        &sender,
        &receiver,
        2,
        &format!(
            "execute in minecraft:the_nether run tp bob {} {} {}",
            position.x + 1.0,
            position.y,
            position.z
        ),
    );
    let id = world.levels[ConnectionId(2)].awaiting_teleport.unwrap();
    world.apply_play(
        ConnectionId(2),
        PlayServerbound::AcceptTeleportation { id },
        &mut Vec::new(),
    );
    let attacker_uuid = Uuid::from_u128(2);
    command(
        &mut world,
        &sender,
        &receiver,
        3,
        &format!(
            "data merge entity {fox_uuid} {{Trusted:[{}]}}",
            uuid_snbt(Uuid::from_u128(1))
        ),
    );
    let health = world.levels[ConnectionId(1)].damage.health;
    let (victim, victim_uuid) = spawn(
        &mut world,
        NETHER,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: position.x + 2.0,
            ..position
        },
    );
    let before = world
        .level(NETHER)
        .entities
        .get(victim)
        .unwrap()
        .living()
        .unwrap()
        .health();
    sender
        .try_send(WorldCommand::Play {
            connection_id: ConnectionId(2),
            packet: PlayServerbound::Attack(bbb_protocol::packets::AttackEntity {
                entity_id: victim,
            }),
        })
        .unwrap();
    world.tick(receiver.drain());
    assert!(
        world
            .level(NETHER)
            .entities
            .get(victim)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < before
    );
    assert_eq!(
        world.levels[ConnectionId(2)].cold.last_hurt_mob,
        Some((victim, victim_uuid))
    );
    command(
        &mut world,
        &sender,
        &receiver,
        4,
        "execute in minecraft:the_nether run damage alice 1 minecraft:generic by bob",
    );

    let player = world.levels.player_at(ConnectionId(1));
    assert!(player.damage.health < health);
    assert_eq!(player.cold.last_hurt_by_mob, Some(attacker_uuid));
    assert!(player.cold.last_hurt_by_mob_timestamp > 0);
    command(&mut world, &sender, &receiver, 5, "tick unfreeze");
    for _ in 0..100 {
        world.tick(Vec::new());
        if world.level(OW).entities.mob_target(fox) == Some(attacker_uuid) {
            break;
        }
    }
    assert!(
        world.level(OW).entities.get(fox).unwrap().tick_count() > 0,
        "Fox rolled back: {:?}; player {:?}; remote attacker {:?}",
        world.mob_goal_debug_for_test(OW, fox),
        remote_player(&world, OW),
        world
            .level(OW)
            .cross_level_entities
            .target_input(attacker_uuid)
    );
    assert_eq!(
        world.level(OW).entities.mob_target(fox),
        Some(attacker_uuid)
    );
    let projected = remote_player(&world, OW);
    assert_eq!(projected.hurt.last_hurt_by_mob, Some(attacker_uuid));
    assert_eq!(
        projected.hurt.last_hurt_by_mob_timestamp,
        world.levels[ConnectionId(1)]
            .cold
            .last_hurt_by_mob_timestamp
    );
    assert_eq!(
        world.levels[ConnectionId(2)].dimension,
        crate::world::LevelId::from(NETHER)
    );
}
#[test]
fn cross_level_targets_foreign_session_is_not_a_local_nearest_candidate() {
    let (mut world, sender, receiver) = world_with_mailbox();
    for _ in 0..80 {
        world.tick(receiver.drain());
    }
    command(&mut world, &sender, &receiver, 1, "tick freeze");
    let position = world.levels[ConnectionId(1)].position();
    floor(&mut world, OW, position);
    command(
        &mut world,
        &sender,
        &receiver,
        90,
        &format!(
            "execute in minecraft:overworld run forceload add {} {}",
            position.x.floor(),
            position.z.floor()
        ),
    );
    let (zombie, _) = spawn(
        &mut world,
        OW,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: position.x + 2.0,
            ..position
        },
    );
    move_level(&mut world, NETHER, position);
    command(&mut world, &sender, &receiver, 2, "team add remote");
    command(&mut world, &sender, &receiver, 3, "team join remote alice");
    command(
        &mut world,
        &sender,
        &receiver,
        4,
        "gamemode spectator alice",
    );
    world.tick_level(&crate::world::LevelId::from(OW), &mut Vec::new());
    assert!(remote_player(&world, OW).spectator);
    let uuid = remote_player(&world, OW).uuid;
    world.with_stored_hurt_level(&OW.into(), |store, level| {
        assert!(level.player_team(store, uuid).is_some());
    });
    command(&mut world, &sender, &receiver, 5, "gamemode survival alice");
    command(&mut world, &sender, &receiver, 6, "tick unfreeze");
    for _ in 0..80 {
        world.tick(Vec::new());
    }
    assert!(!remote_player(&world, OW).spectator);
    assert_eq!(world.level(OW).entities.mob_target(zombie), None);
    move_level(&mut world, OW, position);
    for _ in 0..80 {
        world.tick(Vec::new());
        if world.level(OW).entities.mob_target(zombie).is_some() {
            break;
        }
    }
    assert!(
        world.level(OW).entities.get(zombie).unwrap().tick_count() > 0,
        "Zombie tick rollback"
    );
    assert_eq!(
        world.level(OW).entities.mob_target(zombie),
        Some(Uuid::from_u128(1))
    );
}
