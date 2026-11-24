use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::world::WorldEvent;

use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::store::EntityIdCounter;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_FIREBALL_ID};

use crate::entity::hurting_projectile::fireball::{self, LargeFireballData};
use crate::entity::hurting_projectile::HurtingProjectileState;
use crate::entity::item::plain_stack;

#[test]
fn spawned_large_fireball_carries_its_projectile_defaults() {
    let mut store = EntityStore::new(110, EntityIdCounter::new());
    let owner = Uuid::from_u128(4110);
    let position = Vec3d {
        x: 1.5,
        y: 64.0,
        z: -3.25,
    };
    let shot = ProjectileShot {
        movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.5,
        },
        source_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    };
    let id = fireball::spawn_large_fireball(&mut store, position, shot, owner, 1)
        .expect("the generated fireball type accepts a fresh UUID");

    let live = store.get(id).expect("the fireball is filed").clone();
    assert_eq!(live.entity_type_id(), VANILLA_ENTITY_TYPE_FIREBALL_ID);
    assert_eq!(live.position(), position);
    let EntityData::LargeFireball(data) = live.data() else {
        panic!("spawned entity is not a large fireball")
    };
    assert_eq!(data.projectile().owner(), Some(owner));
    assert_eq!(data.explosion_power(), 1);
    assert_eq!(
        data.projectile().acceleration_power(),
        HurtingProjectileState::INITIAL_ACCELERATION_POWER
    );
}

#[test]
fn fireball_entity_hit_deflects_explodes_and_discards() {
    for kind in [VANILLA_ENTITY_TYPE_FIREBALL_ID, VANILLA_ENTITY_TYPE_COW_ID] {
        let dangling_owner = Uuid::from_u128(0xdad1);
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
        let target = native
            .world
            .spawn_stored_entity_for_test(
                DIMENSION,
                kind,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 2.0,
                },
                if kind == VANILLA_ENTITY_TYPE_FIREBALL_ID {
                    EntityData::LargeFireball(LargeFireballData::new(
                        HurtingProjectileState::new(None, 0.0),
                        plain_stack("minecraft:fire_charge", 1).unwrap(),
                        1,
                    ))
                } else {
                    let mut data = crate::entity::GeneralLivingData::new(kind).unwrap();
                    data.set_no_ai(true);
                    EntityData::GeneralLiving(data)
                },
            )
            .0;
        let before_health = native
            .world
            .stored_entity_for_test(DIMENSION, target)
            .unwrap()
            .living()
            .map(|living| living.health());
        let source = native
            .world
            .spawn_stored_entity_for_test(
                DIMENSION,
                VANILLA_ENTITY_TYPE_FIREBALL_ID,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 0.5,
                },
                EntityData::LargeFireball(LargeFireballData::new(
                    HurtingProjectileState::new(Some(dangling_owner), 0.1),
                    plain_stack("minecraft:fire_charge", 1).unwrap(),
                    // Saved zero blast power isolates the direct hit on the cow.
                    if kind == VANILLA_ENTITY_TYPE_COW_ID {
                        0
                    } else {
                        1
                    },
                )),
            )
            .0;
        native
            .world
            .stored_entity_mut_for_test(DIMENSION, source)
            .unwrap()
            .set_delta_movement(Vec3d {
                x: 0.0,
                y: 0.0,
                z: 2.0,
            });
        let output = native.world.tick(receiver.drain());
        assert!(native
            .world
            .stored_entity_for_test(DIMENSION, source)
            .is_none());
        let hit = native
            .world
            .stored_entity_for_test(DIMENSION, target)
            .unwrap();
        if let EntityData::LargeFireball(data) = hit.data() {
            assert_eq!(data.projectile().owner(), None);
            assert_eq!(
                data.projectile().acceleration_power(),
                HurtingProjectileState::INITIAL_ACCELERATION_POWER
            );
        } else {
            let living = hit.living().unwrap();
            assert_eq!(living.health(), before_health.unwrap() - 6.0);
            assert_eq!(
                living.last_damage_type.as_deref(),
                Some("minecraft:unattributed_fireball")
            );
            assert_eq!(living.last_damage_source_entity(), None);
            assert_eq!(living.last_hurt_by_player, None);
        }
        assert!(output.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Explode(_),
                ..
            }
        )));
    }
}
