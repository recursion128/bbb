use super::*;
use crate::entity::{Entity, EntityData};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID;
use bbb_protocol::entity_synched_data::{VANILLA_MOB_FLAGS_DATA_ID, VANILLA_MOB_FLAG_AGGRESSIVE};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_VINDICATOR_ID,
};
use bbb_protocol::packets::EntityDataValueKind;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const HERO: ConnectionId = ConnectionId(1);
const HERO_UUID: Uuid = Uuid::from_u128(1);

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let tick = console_command(sender, receiver, world, 100, text);
    let Ok(AdminResult::Command(report)) = &admin_outcome(&tick).result else {
        panic!("{text}: command.execute must answer with a command report: {tick:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
    tick
}

fn summon(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
    entity_type: i32,
) -> (i32, Uuid) {
    let before = world.stored_entity_ids_for_test(DIM);
    command(sender, receiver, world, text);
    let added: Vec<_> = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect();
    assert_eq!(added.len(), 1, "one summoned entity: {text}");
    let entity = world.stored_entity_for_test(DIM, added[0]).unwrap();
    assert_eq!(entity.entity_type_id(), entity_type);
    (added[0], entity.uuid())
}

fn body(world: &ServerWorld, id: i32, uuid: Uuid) -> &Entity {
    let entity = world.stored_entity_for_test(DIM, id).unwrap();
    assert_eq!(entity.uuid(), uuid);
    entity
}

fn aggressive(entity: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        panic!("a living raider")
    };
    data.mob_flags().unwrap() & VANILLA_MOB_FLAG_AGGRESSIVE != 0
}

fn assert_patrolling(entity: &Entity) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        panic!("a living raider")
    };
    let raider = data.raider_retained().unwrap();
    assert!(raider.patrolling());
    assert!(!raider.can_join_raid());
    assert!(!crate::entity::raider::holds_current_raid(entity));
}

fn published_aggressive(tick: &TickResult, id: i32) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
            WorldEvent::Send {
                connection_id: HERO,
                packet: ClientboundPlayPacket::SetEntityData(data),
            } if data.id == id && data.values.iter().any(|value|
                value.data_id == VANILLA_MOB_FLAGS_DATA_ID
                    && value.serializer_id == VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID
                    && matches!(value.value, EntityDataValueKind::Byte(flags)
                        if flags & VANILLA_MOB_FLAG_AGGRESSIVE != 0))
        )
    })
}

fn published_head(tick: &TickResult, id: i32) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
            WorldEvent::Send {
                connection_id: HERO,
                packet: ClientboundPlayPacket::RotateHead(head),
            } if head.id == id
        )
    })
}

fn separation(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

#[test]
fn patrolling_vindicator_holds_ground_then_releases_its_hidden_no_ai_companion() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    config.keep_alive_interval_ticks = 10_000;
    let mut world = spawn_pinned_world(config);
    world.tick_rate_mut().set_frozen(true);
    join_loaded(&sender, &receiver, &mut world, 1, "hero", 1);
    for text in [
        "gamerule spawn_mobs false",
        "fill -4 64 -4 24 64 8 minecraft:stone",
        "fill -4 65 -4 24 68 8 minecraft:stone",
        "fill -4 65 -4 24 68 8 minecraft:air",
        "fill -2 65 2 12 67 2 minecraft:stone",
        "tp hero 16.5 65 0.5",
        "effect give hero minecraft:resistance infinite 4 true",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let (source, source_uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:vindicator 0.5 65 0.5 {Patrolling:1b,CanJoinRaid:0b,PersistenceRequired:1b}",
        VANILLA_ENTITY_TYPE_VINDICATOR_ID,
    );
    let (ally, ally_uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:pillager 0.5 65 4.5 {NoAI:1b,CanJoinRaid:0b,PersistenceRequired:1b}",
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
    );
    for text in [
        format!("attribute {source_uuid} minecraft:follow_range base set 32"),
        format!("effect give {ally_uuid} minecraft:invisibility 60 0 true"),
        "gamemode survival hero".into(),
    ] {
        command(&sender, &receiver, &mut world, &text);
    }
    assert_eq!(
        body(&world, source, source_uuid)
            .living()
            .unwrap()
            .attributes()
            .base_value(VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID),
        Some(32.0)
    );
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    assert_eq!(world.publication().players[0].uuid, HERO_UUID);
    assert!(world
        .player_tracking_viewers_for_test(DIM, source)
        .contains(&HERO));
    assert!(world
        .player_tracking_viewers_for_test(DIM, ally)
        .contains(&HERO));
    let health = [source, ally].map(|id| {
        world
            .stored_entity_for_test(DIM, id)
            .unwrap()
            .living()
            .unwrap()
            .health()
    });
    let hero_health = world.player_health_for_test(HERO);
    assert_eq!(world.mob_target_for_test(DIM, ally), None);
    world.tick_rate_mut().set_frozen(false);
    let mut head_published = false;
    for _ in 0..300 {
        let tick = world.tick(receiver.drain());
        head_published |= published_head(&tick, source);
        if world.mob_target_for_test(DIM, source) == Some(HERO_UUID)
            && world.mob_target_for_test(DIM, ally) == Some(HERO_UUID)
        {
            break;
        }
    }
    for id in [source, ally] {
        assert_eq!(world.mob_target_for_test(DIM, id), Some(HERO_UUID));
    }
    let waiting_at = body(&world, source, source_uuid).position();
    for _ in 0..30 {
        let tick = world.tick(receiver.drain());
        head_published |= published_head(&tick, source);
        let entity = body(&world, source, source_uuid);
        assert_patrolling(entity);
        assert!(!aggressive(entity));
        assert!(!aggressive(body(&world, ally, ally_uuid)));
        assert!(separation(entity.position(), waiting_at) < 0.5);
        assert!(separation(entity.position(), world.publication().players[0].position) > 10.0);
        for id in [source, ally] {
            assert_eq!(world.mob_target_for_test(DIM, id), Some(HERO_UUID));
        }
    }
    let source_body = body(&world, source, source_uuid);
    let at = source_body.position();
    let hero_at = world.publication().players[0].position;
    let expected_yaw = (-(hero_at.x - at.x)).atan2(hero_at.z - at.z).to_degrees();
    let yaw_error =
        (f64::from(source_body.y_head_rot()) - expected_yaw + 180.0).rem_euclid(360.0) - 180.0;
    assert!(
        yaw_error.abs() < 15.0,
        "head faces the distant hero: {yaw_error}"
    );
    assert!(head_published, "the hero receives the real head rotation");

    let tick = command(
        &sender,
        &receiver,
        &mut world,
        &format!("tp hero {} {} {}", at.x + 8.0, at.y, at.z),
    );
    let mut published = [
        published_aggressive(&tick, source),
        published_aggressive(&tick, ally),
    ];
    for _ in 0..100 {
        let tick = world.tick(receiver.drain());
        for (seen, id) in published.iter_mut().zip([source, ally]) {
            *seen |= published_aggressive(&tick, id);
        }
        assert_patrolling(body(&world, source, source_uuid));
        body(&world, ally, ally_uuid);
        for id in [source, ally] {
            assert_eq!(world.mob_target_for_test(DIM, id), Some(HERO_UUID));
        }
        if published.into_iter().all(|seen| seen)
            && separation(body(&world, source, source_uuid).position(), at) > 1.0
        {
            break;
        }
    }
    assert!(
        published.into_iter().all(|seen| seen),
        "both raiders publish aggression to the hero"
    );
    assert!(aggressive(body(&world, source, source_uuid)));
    assert!(aggressive(body(&world, ally, ally_uuid)));
    assert!(
        separation(body(&world, source, source_uuid).position(), at) > 1.0,
        "the released vindicator pursues the hero"
    );
    for ((id, uuid), expected) in [(source, source_uuid), (ally, ally_uuid)]
        .into_iter()
        .zip(health)
    {
        assert_eq!(body(&world, id, uuid).living().unwrap().health(), expected);
    }
    assert_eq!(world.player_health_for_test(HERO), hero_health);
    assert_eq!(world.publication().players[0].uuid, HERO_UUID);
}
