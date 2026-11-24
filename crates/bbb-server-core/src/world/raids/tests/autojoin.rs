//! An unaffiliated raider joins a nearby raid before any wave can recruit it.

use super::scene::{command, members, pre_wave, tick, DIMENSION};
use super::*;
use crate::entity::raider::RaiderRaid;
use crate::entity::EntityData;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITCH_ID;
use bbb_server_types::geometry::Vec3Ops;

#[test]
fn an_unaffiliated_no_ai_raider_joins_a_nearby_pre_wave_raid_once() {
    let (mut world, sender, receiver, raid_id, hero_health) = pre_wave();
    world.tick_rate_mut().set_frozen(true);
    command(&sender, &receiver, &mut world, "gamemode creative hero");
    let center = raid_manager(&world).row(raid_id).unwrap().center;
    let ids = world.stored_entity_ids_for_test(DIMENSION);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!(
            "summon minecraft:witch {} {} {} {{CanJoinRaid:1b,NoAI:1b}}",
            f64::from(center.x) + 32.5,
            center.y,
            f64::from(center.z) + 0.5,
        ),
    );
    let peer = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .filter(|id| !ids.contains(id))
        .filter_map(|id| world.stored_entity_for_test(DIMENSION, id))
        .find(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_WITCH_ID)
        .expect("the console summons a fresh real witch");
    let (peer_id, peer_uuid, peer_health) =
        (peer.id(), peer.uuid(), peer.living().unwrap().health());
    let EntityData::GeneralLiving(data) = peer.data() else {
        unreachable!()
    };
    assert!(data.is_no_ai());
    let retained = data.raider_retained().unwrap();
    assert!(retained.can_join_raid());
    assert_eq!(retained.raid(), RaiderRaid::None);
    assert!(
        peer.position().distance_to_sqr(Vec3d {
            x: f64::from(center.x),
            y: f64::from(center.y),
            z: f64::from(center.z),
        }) < 96.0 * 96.0
    );
    assert_eq!(raid_manager(&world).row(raid_id).unwrap().groups_spawned, 0);
    assert_eq!(raid_manager(&world).row(raid_id).unwrap().total_health, 0.0);
    assert_eq!(raid_manager(&world).total_raiders_alive(raid_id), 0);
    assert!(!world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .filter_map(|id| world.stored_entity_for_test(DIMENSION, id))
        .any(crate::entity::raider::holds_current_raid));
    world.tick_rate_mut().set_frozen(false);
    let mut joined = false;
    for _ in 0..60 {
        tick(&sender, &receiver, &mut world);
        let body = world.stored_entity_for_test(DIMENSION, peer_id).unwrap();
        assert_eq!(body.uuid(), peer_uuid);
        let EntityData::GeneralLiving(data) = body.data() else {
            unreachable!()
        };
        if data.raider_retained().unwrap().raid() == RaiderRaid::Current(raid_id) {
            joined = true;
            break;
        }
        assert_eq!(raid_manager(&world).total_raiders_alive(raid_id), 0);
    }
    assert!(
        joined,
        "the actual unaffiliated NoAI witch joins without a recruiting wave"
    );
    for _ in 0..40 {
        tick(&sender, &receiver, &mut world);
    }
    let row = raid_manager(&world).row(raid_id).unwrap();
    assert_eq!(row.groups_spawned, 0);
    assert_eq!(row.total_health, peer_health);
    assert_eq!(raid_manager(&world).total_raiders_alive(raid_id), 1);
    let actual_members = members(&world, raid_id, 0);
    assert_eq!(actual_members.len(), 1);
    let body = actual_members[0];
    assert_eq!((body.id(), body.uuid()), (peer_id, peer_uuid));
    let EntityData::GeneralLiving(data) = body.data() else {
        unreachable!()
    };
    assert!(data.is_no_ai());
    let retained = data.raider_retained().unwrap();
    assert_eq!(retained.raid(), RaiderRaid::Current(raid_id));
    assert_eq!(retained.wave(), 0);
    assert_eq!(
        raid_manager(&world).runtime[&raid_id].raiders[&0]
            .keys()
            .collect::<Vec<_>>(),
        [&peer_uuid]
    );
    assert!(
        (raid_manager(&world).runtime[&raid_id].boss.progress()
            - body.living().unwrap().health() / row.total_health)
            .abs()
            < 1.0e-5
    );
    let saved = world
        .save_request(1_786_000_000_300, 13, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(saved_raid_ids(&saved)[&peer_uuid], Some(raid_id));
    let saved_wave = saved.entity_writes.iter().find_map(|write| {
        let data = bbb_server_persistence::Java26EntityChunkData::parse(
            write.document.clone(),
            write.chunk_x,
            write.chunk_z,
        )
        .unwrap();
        let record = data.entities().find(|record| record.uuid() == peer_uuid);
        record.and_then(|record| {
            record.raw().iter().find_map(|entry| match entry.value {
                NbtSummaryValue::Int(value) if entry.name == "Wave" => Some(value),
                _ => None,
            })
        })
    });
    assert_eq!(saved_wave, Some(0));
    assert_eq!(world.player_health_for_test(HERO), hero_health);
}
