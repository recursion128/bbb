//! A real raid member crosses its village toward the only available home.

use super::scene::{command, pre_wave, tick, DIMENSION};
use super::*;
use crate::chunk::poi::PoiOccupancy;
use crate::entity::raider::RaiderRaid;
use crate::entity::EntityData;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID;
use bbb_server_types::geometry::Vec3Ops;

#[test]
fn a_real_raid_member_walks_through_its_village_toward_an_actual_home() {
    let (mut world, sender, receiver, raid_id, hero_health) = pre_wave();
    world.tick_rate_mut().set_frozen(true);
    command(&sender, &receiver, &mut world, "gamemode creative hero");
    let center = raid_manager(&world).row(raid_id).unwrap().center;
    let home = BlockPos {
        x: center.x + 10,
        ..center
    };
    for (x, part) in [(home.x - 1, "foot"), (home.x, "head")] {
        command(
            &sender,
            &receiver,
            &mut world,
            &format!(
                "setblock {x} {} {} minecraft:red_bed[part={part},facing=east]",
                home.y, home.z,
            ),
        );
    }
    let mut homes = Vec::new();
    world
        .level_by_id(&DIMENSION.into())
        .chunks
        .poi()
        .visit_records_in_range(
            [center.x, center.y, center.z],
            48,
            PoiOccupancy::Any,
            |record| {
                if record.type_id == crate::entity::villager::HOME_POI {
                    homes.push(record.pos);
                }
            },
        );
    assert_eq!(homes, vec![[home.x, home.y, home.z]]);
    let ids = world.stored_entity_ids_for_test(DIMENSION);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!(
            "summon minecraft:pillager {} {} {} {{CanJoinRaid:1b}}",
            f64::from(center.x) - 4.5,
            center.y,
            f64::from(center.z) + 0.5,
        ),
    );
    let body = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .filter(|id| !ids.contains(id))
        .filter_map(|id| world.stored_entity_for_test(DIMENSION, id))
        .find(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_PILLAGER_ID)
        .expect("the console summons a fresh ordinary pillager");
    let (id, uuid, health) = (body.id(), body.uuid(), body.living().unwrap().health());
    let EntityData::GeneralLiving(data) = body.data() else {
        unreachable!()
    };
    assert!(!data.is_no_ai());
    assert!(data.raider_retained().unwrap().can_join_raid());
    assert_eq!(data.raider_retained().unwrap().raid(), RaiderRaid::None);
    let destination = Vec3d {
        x: f64::from(home.x) + 0.5,
        y: f64::from(home.y) + 0.5,
        z: f64::from(home.z) + 0.5,
    };
    let initial_distance = body.position().distance_to_sqr(destination).sqrt();
    assert_eq!(raid_manager(&world).total_raiders_alive(raid_id), 0);
    world.tick_rate_mut().set_frozen(false);
    let (mut joined, mut speed, mut approached, mut arrived) = (false, false, false, false);
    let mut final_distance = initial_distance;
    for _ in 0..200 {
        tick(&sender, &receiver, &mut world);
        assert_eq!(raid_manager(&world).row(raid_id).unwrap().groups_spawned, 0);
        let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
        assert_eq!(body.uuid(), uuid);
        let at = body.position();
        assert_eq!(
            world.is_close_to_village(
                &overworld(),
                [
                    at.x.floor() as i32,
                    at.y.floor() as i32,
                    at.z.floor() as i32
                ],
                1,
            ),
            Some(true),
            "the real body stays inside the village where raid-center approach cannot start"
        );
        let EntityData::GeneralLiving(data) = body.data() else {
            unreachable!()
        };
        if data.raider_retained().unwrap().raid() != RaiderRaid::Current(raid_id) {
            continue;
        }
        joined = true;
        let living = body.living().unwrap();
        let dry_ground = body.on_ground() && !body.is_in_water() && !body.is_in_lava();
        speed |= dry_ground
            && (f64::from(living.speed())
                - f64::from(1.05f32) * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
            .abs()
                < 1.0e-5;
        final_distance = at.distance_to_sqr(destination).sqrt();
        approached |= initial_distance - final_distance > 5.0;
        arrived |= final_distance < 2.5;
        if speed && approached && arrived {
            break;
        }
    }
    assert!(joined, "the real spatial autojoin binds this pillager");
    assert!(
        speed && approached && arrived,
        "the actual member walks on dry ground at speed 1.05 toward its home: initial={initial_distance}, final={final_distance}, speed={speed}, approached={approached}, arrived={arrived}"
    );
    let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
    assert_eq!(body.uuid(), uuid);
    assert_eq!(body.living().unwrap().health(), health);
    assert_eq!(world.player_health_for_test(HERO), hero_health);
}
