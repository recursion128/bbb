//! A natural raid publishes celebration after its real village is lost.

use super::scene::{command, members, natural_wave, tick, DIMENSION};
use super::*;
use crate::chunk::poi::PoiOccupancy;
use crate::entity::raider::RaiderRaid;
use crate::entity::EntityData;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::VANILLA_RAIDER_CELEBRATING_DATA_ID;
use bbb_protocol::packets::EntityDataValueKind;

#[test]
fn a_natural_wave_member_publishes_celebration_after_the_actual_village_is_lost() {
    let (mut world, sender, receiver, raid_id, wave, hero_health) = natural_wave();
    command(&sender, &receiver, &mut world, "gamemode creative hero");
    let center = raid_manager(&world).row(raid_id).unwrap().center;
    let mut bells = Vec::new();
    world
        .level_by_id(&DIMENSION.into())
        .chunks
        .poi()
        .visit_records_in_range(
            [center.x, center.y, center.z],
            64,
            PoiOccupancy::IsOccupied,
            |record| {
                if record.type_id == crate::entity::villager::MEETING_POI {
                    bells.push(record.pos);
                }
            },
        );
    let [bell] = bells.as_slice() else {
        panic!("the scene has exactly one real occupied bell: {bells:?}");
    };
    let bell = *bell;
    let body = members(&world, raid_id, wave)
        .into_iter()
        .find(|body| {
            world
                .level_by_id(&DIMENSION.into())
                .entities
                .mob_target(body.id())
                .is_none()
        })
        .expect("the creative hero leaves an actual living wave member without a target");
    let (id, uuid, health) = (body.id(), body.uuid(), body.living().unwrap().health());
    let EntityData::GeneralLiving(data) = body.data() else {
        unreachable!()
    };
    assert_eq!(
        data.raider_retained().unwrap().raid(),
        RaiderRaid::Current(raid_id)
    );
    assert_eq!(data.raider_retained().unwrap().wave(), wave);
    for _ in 0..40 {
        let at = world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .position();
        world.set_player_position_for_test(
            HERO,
            Vec3d {
                x: at.x + 8.0,
                ..at
            },
        );
        tick(&sender, &receiver, &mut world);
    }
    let row = raid_manager(&world).row(raid_id).unwrap();
    assert!(row.groups_spawned > 0);
    assert_eq!(row.status, Java26RaidStatus::Ongoing);
    let removed = command(
        &sender,
        &receiver,
        &mut world,
        &format!("setblock {} {} {} minecraft:air", bell[0], bell[1], bell[2]),
    );
    let celebrates = |output: &crate::world::TickResult| {
        output.events.iter().any(|event| {
            matches!(event, WorldEvent::Send {
                connection_id: HERO,
                packet: ClientboundPlayPacket::SetEntityData(update),
            } if update.id == id && update.values.iter().any(|value| {
                value.data_id == VANILLA_RAIDER_CELEBRATING_DATA_ID
                    && value.serializer_id == VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
                    && value.value == EntityDataValueKind::Boolean(true)
            }))
        })
    };
    let mut lost = raid_manager(&world).row(raid_id).unwrap().status == Java26RaidStatus::Loss;
    let mut published = lost && celebrates(&removed);
    for _ in 0..100 {
        let at = world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .position();
        world.set_player_position_for_test(
            HERO,
            Vec3d {
                x: at.x + 8.0,
                ..at
            },
        );
        let output = tick(&sender, &receiver, &mut world);
        lost |= raid_manager(&world).row(raid_id).unwrap().status == Java26RaidStatus::Loss;
        published |= lost && celebrates(&output);
        if published {
            break;
        }
    }
    assert!(
        lost,
        "removing the real village loses an already spawned raid"
    );
    assert!(
        published,
        "the actual tracked living member publishes BOOLEAN celebrating=true"
    );
    let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
    assert_eq!(body.uuid(), uuid);
    assert!(body.is_alive_with_health());
    assert_eq!(body.living().unwrap().health(), health);
    let EntityData::GeneralLiving(data) = body.data() else {
        unreachable!()
    };
    assert_eq!(
        data.raider_retained().unwrap().raid(),
        RaiderRaid::Current(raid_id)
    );
    assert_eq!(data.raider_retained().unwrap().wave(), wave);
    assert_eq!(world.player_health_for_test(HERO), hero_health);
}
