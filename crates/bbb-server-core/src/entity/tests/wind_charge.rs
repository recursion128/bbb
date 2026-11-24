use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::world::WorldEvent;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID,
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
};

use crate::entity::hurting_projectile::wind_charge::{self, WindChargeData};
use crate::entity::hurting_projectile::HurtingProjectileState;
use crate::entity::item::{plain_stack, ItemEntityData};
use crate::entity::projectile_shoot::ProjectileShot;

#[test]
fn spawned_wind_charge_carries_its_projectile_defaults() {
    let mut store = EntityStore::new(110, EntityIdCounter::new());
    let owner = Uuid::from_u128(3110);
    let position = Vec3d {
        x: 2.5,
        y: 70.25,
        z: -4.5,
    };
    let shot = ProjectileShot {
        movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.5,
        },
        source_movement: Vec3d {
            x: 0.25,
            y: 0.0,
            z: -0.5,
        },
    };
    let id = wind_charge::spawn_wind_charge(&mut store, position, shot, Some(owner))
        .expect("the generated wind-charge type accepts a fresh UUID");

    let live = store.get(id).expect("the wind charge is filed").clone();
    assert_eq!(live.entity_type_id(), VANILLA_ENTITY_TYPE_WIND_CHARGE_ID);
    assert_eq!(live.position(), position);
    assert_eq!(
        live.delta_movement(),
        Vec3d {
            x: 0.25,
            y: 0.0,
            z: 1.0
        }
    );
    assert_eq!((live.y_rot(), live.x_rot()), (0.0, 0.0));
    assert!(live.needs_sync());
    let EntityData::WindCharge(data) = live.data() else {
        panic!("spawned entity is not a wind charge")
    };
    assert_eq!(data.projectile().owner(), Some(owner));
    assert_eq!(data.projectile().acceleration_power(), 0.0);
    assert_eq!(data.no_deflect_ticks(), 5);
}

#[test]
fn spawned_breeze_wind_charge_carries_its_projectile_defaults() {
    let mut store = EntityStore::new(110, EntityIdCounter::new());
    let owner = Uuid::from_u128(5110);
    let position = Vec3d {
        x: 3.5,
        y: 71.0,
        z: 1.25,
    };
    let shot = ProjectileShot {
        movement: Vec3d {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        source_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    };
    let id = wind_charge::spawn_breeze_wind_charge(&mut store, position, shot, owner)
        .expect("the generated breeze wind-charge type accepts a fresh UUID");

    let live = store
        .get(id)
        .expect("the breeze wind charge is filed")
        .clone();
    assert_eq!(
        live.entity_type_id(),
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID
    );
    assert_eq!(live.position(), position);
    let EntityData::BreezeWindCharge(data) = live.data() else {
        panic!("spawned entity is not a breeze wind charge")
    };
    assert_eq!(data.projectile().owner(), Some(owner));
    assert_eq!(data.projectile().acceleration_power(), 0.0);
    assert_eq!(data.no_deflect_ticks(), 0);
}

#[test]
fn wind_charge_block_hit_explodes_discards_and_raises_land() {
    for entity_type in [
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID,
    ] {
        let mut native = NativeLevel::new();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "observer", 1);
        crate::tests::place_player(
            &sender,
            &receiver,
            &mut native.world,
            1,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 4.5,
            },
            None,
        );
        let target = pos(2, 68, 0);
        native.set(target, state_id("minecraft:stone"));
        let button = pos(1, 68, 1);
        native.set(pos(1, 67, 1), state_id("minecraft:stone"));
        native.set(
            button,
            state_with(
                "minecraft:stone_button",
                &[("face", "floor"), ("facing", "north"), ("powered", "false")],
            ),
        );
        let charge = native
            .world
            .spawn_stored_entity_for_test(
                DIMENSION,
                entity_type,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 0.5,
                },
                if entity_type == VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID {
                    EntityData::BreezeWindCharge(WindChargeData::new(
                        HurtingProjectileState::new(None, 0.0),
                        0,
                    ))
                } else {
                    EntityData::WindCharge(WindChargeData::new(
                        HurtingProjectileState::new(None, 0.0),
                        0,
                    ))
                },
            )
            .0;
        native
            .world
            .stored_entity_mut_for_test(DIMENSION, charge)
            .unwrap()
            .set_delta_movement(Vec3d {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            });
        let output = native.world.tick(receiver.drain());
        assert!(native
            .world
            .stored_entity_for_test(DIMENSION, charge)
            .is_none());
        assert!(output.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Explode(_),
                ..
            }
        )));
        assert!(native
            .world
            .positioned_game_events_for_test(DIMENSION)
            .iter()
            .any(|event| event.recorded.game_event
                == crate::level_event::VanillaGameEvent::PROJECTILE_LAND
                && event.recorded.pos == target));
        assert_eq!(
            native.world.block_state(DIMENSION, button),
            Some(state_with(
                "minecraft:stone_button",
                &[("face", "floor"), ("facing", "north"), ("powered", "true")]
            ))
        );
    }
}

#[test]
fn wind_charge_entity_hit_hurts_without_crediting_a_nonliving_owner() {
    for kind in [
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        VANILLA_ENTITY_TYPE_COW_ID,
    ] {
        let mut native = NativeLevel::new();
        let owner = native
            .world
            .spawn_stored_entity_for_test(
                DIMENSION,
                VANILLA_ENTITY_TYPE_ITEM_ID,
                Vec3d {
                    x: 10.0,
                    y: 68.0,
                    z: 10.0,
                },
                EntityData::Item(ItemEntityData::new(
                    plain_stack("minecraft:stone", 1).unwrap(),
                    0.0,
                )),
            )
            .1;
        let mut stand_store = EntityStore::new(112, EntityIdCounter::new());
        let stand = crate::entity::spawn_armor_stand(
            &mut stand_store,
            Vec3d {
                x: 0.5,
                y: 68.0,
                z: 2.0,
            },
        )
        .unwrap();
        let target = native
            .world
            .spawn_stored_entity_for_test(
                DIMENSION,
                kind,
                stand_store.get(stand).unwrap().position(),
                if kind == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
                    stand_store.get(stand).unwrap().data().clone()
                } else {
                    let mut data = crate::entity::GeneralLivingData::new(kind).unwrap();
                    data.set_no_ai(true);
                    EntityData::GeneralLiving(data)
                },
            )
            .0;
        let before = native
            .world
            .stored_entity_for_test(DIMENSION, target)
            .unwrap()
            .living()
            .unwrap()
            .health();
        let charge = native
            .world
            .spawn_stored_entity_for_test(
                DIMENSION,
                VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 0.5,
                },
                EntityData::WindCharge(WindChargeData::new(
                    HurtingProjectileState::new(Some(owner), 0.0),
                    0,
                )),
            )
            .0;
        native
            .world
            .stored_entity_mut_for_test(DIMENSION, charge)
            .unwrap()
            .set_delta_movement(Vec3d {
                x: 0.0,
                y: 0.0,
                z: 2.0,
            });
        native.world.tick(Vec::new());
        assert!(native
            .world
            .stored_entity_for_test(DIMENSION, charge)
            .is_none());
        if kind == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
            assert!(native
                .world
                .stored_entity_for_test(DIMENSION, target)
                .is_none());
            assert!(native
                .world
                .stored_items_for_test(DIMENSION)
                .iter()
                .any(|item| item.2 == "minecraft:armor_stand"));
        } else {
            let living = native
                .world
                .stored_entity_for_test(DIMENSION, target)
                .unwrap()
                .living()
                .unwrap();
            assert!(living.health() < before);
            assert_eq!(living.last_damage_source_entity(), None);
        }
    }
}

#[test]
fn a_breeze_wind_charge_does_not_hurt_another_breeze() {
    let mut native = NativeLevel::new();
    let breeze = |native: &mut NativeLevel, z: f64| {
        let mut data = crate::entity::GeneralLivingData::new(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BREEZE_ID,
        )
        .unwrap();
        data.set_no_ai(true);
        native.world.spawn_stored_entity_for_test(
            DIMENSION,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BREEZE_ID,
            Vec3d { x: 0.5, y: 68.0, z },
            EntityData::GeneralLiving(data),
        )
    };
    let (_, owner) = breeze(&mut native, -6.0);
    let (target, _) = breeze(&mut native, 2.0);
    let health = |native: &NativeLevel| {
        native
            .world
            .stored_entity_for_test(DIMENSION, target)
            .unwrap()
            .living()
            .unwrap()
            .health()
    };
    let before = health(&native);
    let charge = native
        .world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID,
            Vec3d {
                x: 0.5,
                y: 68.5,
                z: 0.5,
            },
            EntityData::BreezeWindCharge(WindChargeData::new(
                HurtingProjectileState::new(Some(owner), 0.0),
                0,
            )),
        )
        .0;
    native
        .world
        .stored_entity_mut_for_test(DIMENSION, charge)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 2.0,
        });
    native.world.tick(Vec::new());
    assert!(native
        .world
        .stored_entity_for_test(DIMENSION, charge)
        .is_none());
    assert_eq!(health(&native), before);
}
