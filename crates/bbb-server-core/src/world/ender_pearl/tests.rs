use super::*;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::item::plain_stack;
use crate::entity::living::FallImpulseContext;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::{spawn_ender_pearl, GeneralLivingData};
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::{ServerWorldConfig, TickResult};
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ENDER_PEARL_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{
    BlockPos, InteractionHand, ItemStackSummary, PlayServerbound, PlayerRotation, UseItem,
};
use bbb_server_types::save::Java26BuiltinDimension;

const PLAYER: ConnectionId = ConnectionId(1);
const OBSERVER: ConnectionId = ConnectionId(2);
fn source() -> LevelId {
    Java26BuiltinDimension::Overworld.into()
}

fn position(x: f64, y: f64, z: f64) -> Vec3d {
    Vec3d { x, y, z }
}

fn fixture() -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..ServerWorldConfig::default()
    })
    .unwrap();
    world.set_test_respawn_radius(0);
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.set_player_position_for_test(PLAYER, position(0.5, 70.0, 0.5));
    world.set_player_position_for_test(OBSERVER, position(0.5, 70.0, 8.5));
    world.ensure_entity_transfer_full_chunk_for_test_by_key(&source(), (0, 0));
    world
}

fn wall(world: &mut ServerWorld) {
    let state = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:stone")
            .unwrap()
            .id,
    )
    .unwrap();
    for y in 70..74 {
        world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 5, y, z: 0 },
            state,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        );
    }
}

fn throw(world: &mut ServerWorld) -> i32 {
    world.stock_menu_slot_for_test(
        PLAYER,
        36,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_ENDER_PEARL_ID),
            count: 2,
            component_patch: Default::default(),
        },
    );
    world.apply_play(
        PLAYER,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence: 1,
            y_rot: -90.0,
            x_rot: 0.0,
        }),
        &mut Vec::new(),
    );
    world
        .level_by_id(&source())
        .entities
        .ids()
        .into_iter()
        .find(|id| {
            world
                .level_by_id(&source())
                .entities
                .get(*id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_ENDER_PEARL_ID
        })
        .unwrap()
}

fn pearl(world: &mut ServerWorld, owner: Option<uuid::Uuid>) -> i32 {
    spawn_ender_pearl(
        &mut world.level_mut_by_id(&source()).entities,
        position(1.5, 71.5, 0.5),
        ProjectileShot::new(position(2.0, 0.0, 0.0)),
        plain_stack("minecraft:ender_pearl", 1).unwrap(),
        owner,
    )
    .unwrap()
}

fn impact(world: &mut ServerWorld, id: i32) -> (Vec3d, TickResult) {
    for _ in 0..12 {
        let old = world
            .level_by_id(&source())
            .entities
            .get(id)
            .unwrap()
            .position();
        let tick = world.tick(Vec::new());
        if world.level_by_id(&source()).entities.get(id).is_none() {
            return (old, tick);
        }
    }
    panic!("the pearl must hit its nearby target");
}

fn teleport_packet(events: &[WorldEvent]) -> &bbb_protocol::packets::PlayerPositionUpdate {
    events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: PLAYER,
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } => Some(update),
            _ => None,
        })
        .expect("the owner receives the teleport")
}

#[test]
fn used_ender_pearl_keeps_loading_and_flying_beyond_all_players() {
    let mut world = fixture();
    world.set_player_position_for_test(PLAYER, position(0.5, 200.0, 0.5));
    let id = throw(&mut world);
    let start = world
        .level_by_id(&source())
        .entities
        .get(id)
        .unwrap()
        .position();
    let start_chunk = crate::world::player_packets::player_chunk(start);
    for connection in [PLAYER, OBSERVER] {
        world.set_player_position_for_test(connection, position(512.5, 200.0, 512.5));
        world.move_player_chunk_tracking(connection, &mut Vec::new());
    }
    for _ in 0..32 {
        let old = world
            .level_by_id(&source())
            .entities
            .get(id)
            .unwrap()
            .position();
        world.tick(Vec::new());
        let level = world.level_by_id(&source());
        let current = level.entities.get(id).expect("the pearl stays in flight");
        assert!(current.position().x > old.x);
        let chunk = crate::world::player_packets::player_chunk(old);
        assert!(level.chunks.is_loaded(chunk.0, chunk.1));
        assert!(world
            .entity_ticking_chunks_by_key(&source())
            .contains(&chunk));
        assert!(level.player_chunk_membership.centres().all(|player| {
            let distance = player.0.abs_diff(chunk.0).max(player.1.abs_diff(chunk.1));
            distance > world.admin.settings().view_distance as u32 + 2
                && distance > world.admin.settings().simulation_distance as u32
        }));
    }
    let level = world.level_by_id(&source());
    let end = level.entities.get(id).unwrap().position();
    let end_chunk = crate::world::player_packets::player_chunk(end);
    assert!(end.x > start.x + 16.0);
    assert_ne!(start_chunk, end_chunk);
    for chunk in [start_chunk, end_chunk] {
        assert!(level.chunks.is_loaded(chunk.0, chunk.1));
        assert!(world
            .entity_ticking_chunks_by_key(&source())
            .contains(&chunk));
    }
}

#[test]
fn ender_pearl_end_exit_requires_online_owner_seen_credits() {
    let mut world = fixture();
    let end = Java26BuiltinDimension::End;
    let entry = BlockPos { x: 0, y: 70, z: 0 };
    world.place_portal_ticket(end, entry).unwrap();
    world.set_block(
        end,
        entry,
        vanilla_block_default_state_id(bbb_protocol::block_types::VANILLA_BLOCK_END_PORTAL_ID)
            .unwrap(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    let owner = world.levels[PLAYER].profile.uuid;
    let spawn = |world: &mut ServerWorld| {
        let entities = &mut world.level_mut_by_id(&end.into()).entities;
        let id = spawn_ender_pearl(
            entities,
            position(0.5, 70.5, 0.5),
            ProjectileShot::new(Vec3d::default()),
            plain_stack("minecraft:ender_pearl", 1).unwrap(),
            Some(owner),
        )
        .unwrap();
        let entity = entities.get_mut(id).unwrap();
        entity.set_no_gravity(true);
        entity.uuid()
    };
    assert!(!world.levels[PLAYER].cold.seen_credits);
    let blocked = spawn(&mut world);
    for _ in 0..5 {
        world.tick(Vec::new());
    }
    assert!(world
        .live_entity_id_for_uuid_for_test(end, blocked)
        .is_some());
    assert!(world
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, blocked)
        .is_none());

    world.levels[PLAYER].cold.seen_credits = true;
    let allowed = spawn(&mut world);
    for _ in 0..5 {
        world.tick(Vec::new());
        if world
            .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, allowed)
            .is_some()
        {
            break;
        }
    }
    assert!(world
        .live_entity_id_for_uuid_for_test(end, allowed)
        .is_none());
    assert!(world
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, allowed)
        .is_some());
}

#[test]
fn ender_pearl_used_item_hits_block_teleports_and_damages_player() {
    let mut world = fixture();
    wall(&mut world);
    let id = throw(&mut world);
    let motion = position(0.25, 0.1, -0.125);
    let rotation = PlayerRotation {
        y_rot: 37.0,
        x_rot: -11.0,
    };
    let player = &mut world.levels[PLAYER];
    player.set_delta_movement(motion);
    player.set_rotation(rotation);
    player.block_contact.fall_distance = 9.0;
    player.cold.fall_impulse = FallImpulseContext {
        reset_grace_time: 40,
        impact_position: Some(position(0.5, 80.0, 0.5)),
    };
    let mut events = Vec::new();
    let mut destination = None;
    for _ in 0..12 {
        let old = world
            .level_by_id(&source())
            .entities
            .get(id)
            .unwrap()
            .position();
        world.tick_level_entities(&source(), true, &mut events);
        world.apply_level_entity_lifecycle_by_key(&source(), &mut events);
        if world.level_by_id(&source()).entities.get(id).is_none() {
            destination = Some(old);
            break;
        }
    }
    let destination = destination.expect("the used pearl hits the wall");
    let player = &world.levels[PLAYER];
    assert_eq!(player.position(), destination);
    assert!(
        destination.x < 5.0,
        "the tick's old position precedes the collision face"
    );
    assert_eq!(player.rotation(), rotation);
    assert_eq!(player.delta_movement(), motion);
    assert_eq!(player.damage.health, 15.0);
    assert_eq!(player.block_contact.fall_distance, 0.0);
    assert_eq!(player.cold.fall_impulse, FallImpulseContext::default());
    let update = teleport_packet(&events);
    assert_eq!(update.position, destination);
    assert_eq!(update.relatives_mask, PRESERVE_MOTION_AND_ROTATION);
    let pearl_damage = world
        .cold
        .damage_types
        .iter()
        .find(|damage| damage.resource_id == "minecraft:ender_pearl")
        .unwrap()
        .registry_id;
    assert!(events.iter().any(|event| matches!(
        event.packet_for(PLAYER),
        Some(ClientboundPlayPacket::DamageEvent(damage))
            if damage.source_type_id == pearl_damage
    )));
    assert!(events.iter().any(|event| matches!(
        event.sent_packet(),
        Some(ClientboundPlayPacket::Sound(sound))
            if sound.source == SoundSource::Players && sound.position == destination
                && sound.sound == SoundEventHolder::Reference { registry_id: vanilla_sound_event_registry_id("minecraft:entity.player.teleport").unwrap() }
    )));
    let particles = events.iter().filter(|event| matches!(
        event.packet_for(OBSERVER),
        Some(ClientboundPlayPacket::LevelParticles(particles))
            if particles.particle.particle_type_id == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_PORTAL_ID
    )).count();
    assert_eq!(particles, 32);
}

fn zombie(world: &mut ServerWorld, at: Vec3d) -> (i32, uuid::Uuid) {
    let store = &mut world.level_mut_by_id(&source()).entities;
    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
    data.set_no_ai(true);
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        at,
        EntityData::GeneralLiving(data),
    );
    let entity = store.get_mut(id).unwrap();
    entity.set_no_gravity(true);
    (id, entity.uuid())
}

#[test]
fn ender_pearl_entity_impact_applies_zero_damage_hurt_and_teleports_stored_owner() {
    let mut world = fixture();
    let (owner_id, owner) = zombie(&mut world, position(0.5, 70.0, 5.5));
    let (target, _) = zombie(&mut world, position(4.0, 71.0, 0.5));
    let motion = position(0.0, 0.0, 0.125);
    let entity = world
        .level_mut_by_id(&source())
        .entities
        .get_mut(owner_id)
        .unwrap();
    entity.set_rot(63.0, -17.0);
    entity.set_delta_movement(motion);
    entity.set_fall_distance(8.0);
    entity.living_mut().unwrap().fall_impulse.apply_grace(40);
    let id = pearl(&mut world, Some(owner));
    let (destination, tick) = impact(&mut world, id);
    let store = &world.level_by_id(&source()).entities;
    let entity = store.get(owner_id).unwrap();
    assert_eq!(entity.position(), destination);
    assert_eq!(entity.living().unwrap().health(), 20.0);
    assert_eq!(entity.fall_distance(), 0.0);
    assert_eq!(
        entity.living().unwrap().fall_impulse,
        FallImpulseContext::default()
    );
    assert_eq!((entity.y_rot(), entity.x_rot()), (63.0, -17.0));
    assert_eq!(store.get(target).unwrap().living().unwrap().health(), 20.0);
    assert!(tick.events.iter().any(|event| matches!(
        event.sent_packet(),
        Some(ClientboundPlayPacket::DamageEvent(damage))
            if damage.entity_id == target
                && damage.source_direct_id == id
                && damage.source_cause_id == owner_id
    )));
}

#[test]
fn ender_pearl_returns_owner_from_another_level() {
    let mut world = fixture();
    wall(&mut world);
    let id = throw(&mut world);
    let target: LevelId = Java26BuiltinDimension::Nether.into();
    world.teleport_player_id(
        PLAYER,
        TeleportTransition {
            new_level: target,
            position: position(10.0, 80.0, 10.0),
            y_rot: 12.0,
            x_rot: 4.0,
            relatives_mask: 0,
        },
        &mut Vec::new(),
    );
    let (destination, tick) = impact(&mut world, id);
    assert_eq!(world.levels[PLAYER].dimension, source());
    assert_eq!(world.levels[PLAYER].position(), destination);
    assert_eq!(world.levels[PLAYER].damage.health, 15.0);
    assert_eq!(teleport_packet(&tick.events).position, destination);
}

#[test]
fn ender_pearl_in_flight_obeys_owner_death_rule_and_preserves_credits() {
    for (vanish, credits) in [(true, false), (false, false), (true, true)] {
        let mut world = fixture();
        assert!(world.admin.set_game_rule(
            "minecraft:ender_pearls_vanish_on_death",
            bbb_protocol::game_rules::GameRuleValue::Boolean(vanish),
        ));
        let id = throw(&mut world);
        let _ = world.tick(Vec::new());
        let old = world
            .level_by_id(&source())
            .entities
            .get(id)
            .expect("the thrown pearl is still in flight")
            .position();
        if credits {
            world.teleport_player_id(
                PLAYER,
                TeleportTransition {
                    new_level: Java26BuiltinDimension::End.into(),
                    position: position(10.0, 80.0, 10.0),
                    y_rot: 0.0,
                    x_rot: 0.0,
                    relatives_mask: 0,
                },
                &mut Vec::new(),
            );
            world.record_player_portal_contact(
                PLAYER,
                crate::entity::portal::PortalKind::End,
                BlockPos {
                    x: 10,
                    y: 80,
                    z: 10,
                },
                &mut Vec::new(),
            );
            assert!(world.levels[PLAYER].cold.won_game);
            assert!(world.levels[PLAYER].avatar_removed);
        } else {
            assert!(world.hurt_connected_player(
                PLAYER,
                "minecraft:generic_kill",
                40.0,
                None,
                None,
                None,
                &mut Vec::new(),
            ));
            assert!(world.levels[PLAYER].damage.is_dead_or_dying());
        }
        let tick = world.tick(Vec::new());
        let remaining = world.level_by_id(&source()).entities.get(id);
        if vanish && !credits {
            assert!(remaining.is_none());
            assert!(tick.events.iter().any(|event| matches!(
                event.packet_for(OBSERVER),
                Some(ClientboundPlayPacket::RemoveEntities(remove))
                    if remove.entity_ids.contains(&id)
            )));
        } else {
            assert_ne!(remaining.expect("the pearl keeps flying").position(), old);
        }
    }
}

#[test]
fn ender_pearl_returns_stored_owner_by_uuid_after_level_transfer() {
    let mut world = fixture();
    wall(&mut world);
    let (owner_id, owner) = zombie(&mut world, position(0.5, 70.0, 5.5));
    let target: LevelId = Java26BuiltinDimension::Nether.into();
    let transferred = world
        .teleport_entity_subtree(
            source(),
            target,
            owner_id,
            StoredEntityTransition {
                position: position(10.5, 80.0, 10.5),
                delta_movement: position(0.1, 0.2, 0.3),
                y_rot: 23.0,
                x_rot: -8.0,
                relatives_mask: 0,
            },
            false,
            &mut Vec::new(),
        )
        .unwrap()
        .unwrap();
    let id = pearl(&mut world, Some(owner));
    let (destination, _) = impact(&mut world, id);
    assert!(world
        .level_by_id(&target)
        .entities
        .get(transferred)
        .is_none());
    let returned = world
        .level_by_id(&source())
        .entities
        .entity_id_for_uuid(owner)
        .unwrap();
    let entity = world.level_by_id(&source()).entities.get(returned).unwrap();
    assert_eq!(entity.position(), destination);
    assert_eq!(entity.living().unwrap().health(), 20.0);
    assert_eq!((entity.y_rot(), entity.x_rot()), (23.0, -8.0));
    assert_eq!(entity.delta_movement(), position(0.1, 0.2, 0.3));
}

#[test]
fn ender_pearl_discards_for_missing_dead_sleeping_and_disconnected_owners() {
    let mut world = fixture();
    wall(&mut world);
    let id = pearl(&mut world, Some(uuid::Uuid::from_u128(9000)));
    impact(&mut world, id);
    let bed = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:red_bed")
            .unwrap()
            .id,
    )
    .unwrap();
    world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 0, y: 70, z: 5 },
        bed,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    for sleeping in [false, true] {
        let (owner_id, owner) = zombie(&mut world, position(0.5, 70.0, 5.5));
        let entity = world
            .level_mut_by_id(&source())
            .entities
            .get_mut(owner_id)
            .unwrap();
        if sleeping {
            entity.set_sleeping_pos(Some(BlockPos { x: 0, y: 70, z: 5 }));
        } else {
            entity.living_mut().unwrap().set_health(0.0);
        }
        let _ = world.tick(Vec::new());
        let position = world
            .level_by_id(&source())
            .entities
            .get(owner_id)
            .unwrap()
            .position();
        let id = pearl(&mut world, Some(owner));
        impact(&mut world, id);
        assert_eq!(
            world
                .level_by_id(&source())
                .entities
                .get(owner_id)
                .unwrap()
                .position(),
            position
        );
    }
    let owner = world.levels[PLAYER].profile.uuid;
    world.remove_player(PLAYER, &mut Vec::new());
    let id = pearl(&mut world, Some(owner));
    let (_, tick) = impact(&mut world, id);
    assert!(!tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: PLAYER,
            packet: ClientboundPlayPacket::PlayerPosition(_)
        }
    )));
}
