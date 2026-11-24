//! A real raid survivor retrieves its fallen captain's canonical banner.

use super::scene::{command, members, natural_wave, tick, DIMENSION};
use super::*;
use crate::entity::general_living::spawn_finalize_equipment::is_ominous_banner;
use crate::entity::EntityData;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::geometry::Vec3Ops;

#[test]
fn a_natural_wave_survivor_walks_to_its_captains_dropped_banner_and_becomes_leader() {
    let (mut world, sender, receiver, raid_id, wave, health) = natural_wave();
    let captain_uuid = raid_manager(&world)
        .leader(raid_id, wave)
        .expect("a natural captain");
    let captain_id = world
        .level_by_id(&DIMENSION.into())
        .entities
        .entity_id_for_uuid(captain_uuid)
        .unwrap();
    let mut separated = false;
    let mut separation = Vec::new();
    for _ in 0..1600 {
        let captain = world.stored_entity_for_test(DIMENSION, captain_id).unwrap();
        let at = captain.position();
        separation = members(&world, raid_id, wave)
            .into_iter()
            .filter(|body| body.id() != captain_id)
            .map(|body| (body.id(), body.position().distance_to_sqr(at).sqrt()))
            .collect();
        if !separation.is_empty()
            && separation.iter().all(|(_, d)| *d > 4.5)
            && separation.iter().any(|(_, d)| *d < 16.0)
            && captain.on_ground()
        {
            separated = true;
            break;
        }
        world.set_player_position_for_test(
            HERO,
            Vec3d {
                x: at.x + 24.0,
                y: at.y,
                z: at.z,
            },
        );
        tick(&sender, &receiver, &mut world);
    }
    assert!(
        separated,
        "the real wave must naturally separate beyond immediate pickup: {separation:?}"
    );
    let death_position = world
        .stored_entity_for_test(DIMENSION, captain_id)
        .unwrap()
        .position();
    let EntityData::GeneralLiving(captain_data) = world
        .stored_entity_for_test(DIMENSION, captain_id)
        .unwrap()
        .data()
    else {
        unreachable!()
    };
    let captain_raid = captain_data.raider_retained().unwrap().raid();
    world.set_player_position_for_test(
        HERO,
        Vec3d {
            x: death_position.x + 24.0,
            y: death_position.y,
            z: death_position.z,
        },
    );
    tick(&sender, &receiver, &mut world);
    let killed = command(
        &sender,
        &receiver,
        &mut world,
        &format!("kill {captain_uuid}"),
    );
    assert!(!world
        .stored_entity_for_test(DIMENSION, captain_id)
        .unwrap()
        .is_alive_with_health());
    let resolver = world.item_stack_resolver_for_test();
    let (banner_id, banner_uuid, banner_stack, banner_position) = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .find_map(|id| {
            let body = world.stored_entity_for_test(DIMENSION, id)?;
            let EntityData::Item(item) = body.data() else {
                return None;
            };
            is_ominous_banner(item.item(), &resolver)
                .then(|| (id, body.uuid(), item.item().clone(), body.position()))
        })
        .expect("normal captain death drops the exact canonical ominous banner");
    let mut observations: BTreeMap<i32, (f64, bool, bool)> = members(&world, raid_id, wave)
        .into_iter()
        .map(|body| {
            let distance = body.position().distance_to_sqr(banner_position).sqrt();
            assert!(
                distance > 2.0,
                "the actual banner starts beyond pickup reach: {distance}"
            );
            (body.id(), (distance, false, false))
        })
        .collect();
    assert!(!killed.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::TakeItemEntity(take), .. } if take.item_id == banner_id)));
    let mut collector = None;
    let mut equipped = BTreeSet::new();
    for _ in 0..600 {
        let output = tick(&sender, &receiver, &mut world);
        if world
            .stored_entity_for_test(DIMENSION, banner_id)
            .filter(|body| body.uuid() == banner_uuid)
            .is_some()
        {
            for body in members(&world, raid_id, wave) {
                let Some((initial, speed, progress)) = observations.get_mut(&body.id()) else {
                    continue;
                };
                let living = body.living().unwrap();
                let dry_ground = body.on_ground() && !body.is_in_water() && !body.is_in_lava();
                *speed |= dry_ground
                    && (f64::from(living.speed())
                        - 1.15 * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
                    .abs()
                        < 1.0e-5;
                *progress |= dry_ground
                    && *initial - body.position().distance_to_sqr(banner_position).sqrt() > 1.0;
            }
        }
        for event in &output.events {
            if let WorldEvent::Send {
                connection_id: HERO,
                packet,
                ..
            } = event
            {
                match packet {
                    ClientboundPlayPacket::TakeItemEntity(take) if take.item_id == banner_id => {
                        assert_eq!(take.amount, 1);
                        collector = Some(take.player_id);
                    }
                    ClientboundPlayPacket::SetEquipment(update)
                        if update.slots.iter().any(|slot| {
                            slot.slot == EquipmentSlot::Head && &slot.item == banner_stack.summary()
                        }) =>
                    {
                        equipped.insert(update.entity_id);
                    }
                    _ => {}
                }
            }
        }
        if collector.is_some_and(|id| equipped.contains(&id))
            && world.stored_entity_for_test(DIMENSION, banner_id).is_none()
        {
            break;
        }
    }
    let collector = collector.expect("the real tracked item publishes its full TakeItem pickup");
    assert!(equipped.contains(&collector));
    let (initial, speed, progress) = observations[&collector];
    assert!(speed && progress, "the actual collector walks at 1.15 before pickup: initial={initial}, speed={speed}, progress={progress}");
    assert!(world.stored_entity_for_test(DIMENSION, banner_id).is_none());
    let body = world.stored_entity_for_test(DIMENSION, collector).unwrap();
    assert!(is_ominous_banner(
        body.living()
            .unwrap()
            .item_by_slot(EquipmentSlot::Head)
            .unwrap(),
        &resolver
    ));
    assert!(crate::entity::patrol_spawn_state(body).unwrap().1);
    let EntityData::GeneralLiving(data) = body.data() else {
        unreachable!()
    };
    let retained = data.raider_retained().unwrap();
    assert_eq!(retained.raid(), captain_raid);
    assert_eq!(retained.wave(), wave);
    assert_eq!(
        data.mob_retained()
            .unwrap()
            .drop_chance(EquipmentSlot::Head),
        2.0
    );
    assert_eq!(
        raid_manager(&world).leader(raid_id, wave),
        Some(body.uuid())
    );
    assert_eq!(world.player_health_for_test(HERO), health);
}
