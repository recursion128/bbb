//! A natural raid wave approaches its village and recruits an existing nearby body.

use super::scene::{command, members, natural_wave, tick, DIMENSION};
use super::*;
use crate::entity::raider::RaiderRaid;
use crate::entity::EntityData;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITCH_ID;
use bbb_server_types::geometry::Vec3Ops;

#[test]
fn a_natural_raid_wave_approaches_its_center_and_recruits_a_nearby_witch_once() {
    let (mut world, sender, receiver, raid_id, wave, hero_health) = natural_wave();
    command(&sender, &receiver, &mut world, "gamemode creative hero");
    let center = raid_manager(&world).row(raid_id).unwrap().center;
    let center = Vec3d {
        x: f64::from(center.x) + 0.5,
        y: f64::from(center.y),
        z: f64::from(center.z) + 0.5,
    };
    let mut observations: BTreeMap<i32, (Uuid, Vec3d, bool)> = members(&world, raid_id, wave)
        .into_iter()
        .map(|body| (body.id(), (body.uuid(), body.position(), false)))
        .collect();
    assert!(!observations.is_empty());
    let mut recruiter = None;
    for _ in 0..500 {
        tick(&sender, &receiver, &mut world);
        for body in members(&world, raid_id, wave) {
            let Some((uuid, initial, speed)) = observations.get_mut(&body.id()) else {
                continue;
            };
            assert_eq!(body.uuid(), *uuid);
            let at = body.position();
            let outside_village = world.is_close_to_village(
                &overworld(),
                [
                    at.x.floor() as i32,
                    at.y.floor() as i32,
                    at.z.floor() as i32,
                ],
                1,
            ) == Some(false);
            let dry_ground = body.on_ground() && !body.is_in_water() && !body.is_in_lava();
            let living = body.living().unwrap();
            *speed |= outside_village
                && dry_ground
                && (f64::from(living.speed())
                    - living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
                .abs()
                    < 1.0e-5;
            if *speed
                && outside_village
                && dry_ground
                && initial.distance_to_sqr(center).sqrt() - at.distance_to_sqr(center).sqrt() > 3.0
            {
                recruiter = Some((body.uuid(), at));
                break;
            }
        }
        if recruiter.is_some() {
            break;
        }
    }
    let (recruiter_uuid, at) = recruiter.expect(
        "an unmodified natural raider walks on dry ground at speed 1.0 and approaches its actual raid center",
    );
    world.set_player_position_for_test(
        HERO,
        Vec3d {
            x: at.x + 12.0,
            ..at
        },
    );
    world.tick_rate_mut().set_frozen(true);
    let original_members: BTreeSet<_> = members(&world, raid_id, wave)
        .into_iter()
        .map(|body| body.uuid())
        .collect();
    let original_total = raid_manager(&world).row(raid_id).unwrap().total_health;
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {recruiter_uuid} 2 minecraft:generic"),
    );
    let ids = world.stored_entity_ids_for_test(DIMENSION);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!(
            "summon minecraft:witch {} {} {} {{CanJoinRaid:1b}}",
            at.x + 2.0,
            at.y,
            at.z,
        ),
    );
    let peer = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .filter(|id| !ids.contains(id))
        .filter_map(|id| world.stored_entity_for_test(DIMENSION, id))
        .find(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_WITCH_ID)
        .expect("the console summons a fresh real witch");
    let (peer_id, peer_uuid, peer_position, peer_health) = (
        peer.id(),
        peer.uuid(),
        peer.position(),
        peer.living().unwrap().health(),
    );
    let EntityData::GeneralLiving(data) = peer.data() else {
        unreachable!()
    };
    let retained = data.raider_retained().unwrap();
    assert!(retained.can_join_raid());
    assert_eq!(retained.raid(), RaiderRaid::None);
    assert!(peer_position.distance_to_sqr(at) < 16.0 * 16.0);
    assert!(!raid_manager(&world).runtime[&raid_id].raiders[&wave].contains_key(&peer_uuid));
    world.tick_rate_mut().set_frozen(false);
    let mut boss_progress = None;
    let mut recruited = false;
    for _ in 0..100 {
        let output = tick(&sender, &receiver, &mut world);
        for operation in boss_operations(&output.events) {
            if let BossEventOperation::UpdateProgress { progress } = operation {
                boss_progress = Some(*progress);
            }
        }
        let body = world.stored_entity_for_test(DIMENSION, peer_id).unwrap();
        assert_eq!(body.uuid(), peer_uuid);
        let EntityData::GeneralLiving(data) = body.data() else {
            unreachable!()
        };
        if data.raider_retained().unwrap().raid() == RaiderRaid::Current(raid_id) {
            assert!(body.position().distance_to_sqr(peer_position) < 8.0 * 8.0);
            recruited = true;
            break;
        }
    }
    assert!(
        recruited,
        "the moving wave recruits the actual nearby witch"
    );
    let expected_total = original_total + peer_health;
    let health: f32 = members(&world, raid_id, wave)
        .iter()
        .map(|body| body.living().unwrap().health())
        .sum();
    let expected_progress = health / expected_total;
    assert!(
        (raid_manager(&world).runtime[&raid_id].boss.progress() - expected_progress).abs() < 1.0e-5
    );
    assert!(boss_progress.is_some_and(|progress| (progress - expected_progress).abs() < 1.0e-5));
    for _ in 0..60 {
        tick(&sender, &receiver, &mut world);
        assert_eq!(
            raid_manager(&world).row(raid_id).unwrap().total_health,
            expected_total
        );
    }
    let row = raid_manager(&world).row(raid_id).unwrap();
    assert_eq!(row.groups_spawned, wave);
    let actual_members = members(&world, raid_id, wave);
    assert_eq!(actual_members.len(), original_members.len() + 1);
    assert_eq!(
        raid_manager(&world).total_raiders_alive(raid_id),
        original_members.len() + 1,
    );
    assert!(actual_members
        .iter()
        .all(|body| body.uuid() == peer_uuid || original_members.contains(&body.uuid())));
    let body = world.stored_entity_for_test(DIMENSION, peer_id).unwrap();
    assert_eq!(body.uuid(), peer_uuid);
    let EntityData::GeneralLiving(data) = body.data() else {
        unreachable!()
    };
    let retained = data.raider_retained().unwrap();
    assert_eq!(retained.raid(), RaiderRaid::Current(raid_id));
    assert_eq!(retained.wave(), row.groups_spawned);
    assert!(raid_manager(&world).runtime[&raid_id].raiders[&wave].contains_key(&peer_uuid));
    let saved = world
        .save_request(1_786_000_000_200, 12, RegionCompression::Zlib)
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
    assert_eq!(saved_wave, Some(wave));
    assert_eq!(world.player_health_for_test(HERO), hero_health);
}
