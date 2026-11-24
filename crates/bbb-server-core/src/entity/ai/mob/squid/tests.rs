use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SQUID_ID;
use bbb_protocol::packets::Vec3d;
use uuid::Uuid;

use crate::entity::ai::mob::MobAiRuntime;
use crate::entity::living::ActiveMobEffect;
use crate::entity::{Entity, EntityData, GeneralLivingData};

fn entity(entity_type_id: i32) -> Entity {
    Entity::new(
        37,
        Uuid::from_u128(37),
        entity_type_id,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::GeneralLiving(GeneralLivingData::new(entity_type_id).unwrap()),
    )
}

#[test]
fn squid_autonomous_runtime_water_dry_and_levitation_movement() {
    let mut squid = entity(VANILLA_ENTITY_TYPE_SQUID_ID);
    let mut runtime = MobAiRuntime::for_entity(
        VANILLA_ENTITY_TYPE_SQUID_ID,
        crate::random::GameRandom::with_seed(0),
    );
    let propulsion = Vec3d {
        x: 0.2,
        y: 0.1,
        z: -0.2,
    };
    runtime.squid_runtime_mut().unwrap().set_test_state(
        propulsion,
        std::f32::consts::PI * 0.8,
        0.0,
    );
    assert_eq!(runtime.tick_squid(&mut squid, Some(true)), Some(false));
    assert_eq!(squid.delta_movement(), propulsion);
    assert_eq!(squid.y_rot(), squid.living().unwrap().y_body_rot);

    squid.set_delta_movement(Vec3d {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    });
    runtime
        .squid_runtime_mut()
        .unwrap()
        .set_test_state(Vec3d::default(), 0.0, 0.0);
    assert_eq!(runtime.tick_squid(&mut squid, Some(false)), Some(false));
    assert_eq!(
        squid.delta_movement(),
        Vec3d {
            x: 0.0,
            y: (1.0 - 0.08) * f64::from(0.98_f32),
            z: 0.0,
        }
    );

    squid
        .living_mut()
        .unwrap()
        .add_active_effect(ActiveMobEffect::new("minecraft:levitation", 20, 1).unwrap());
    squid.set_delta_movement(Vec3d {
        x: 0.0,
        y: -4.0,
        z: 0.0,
    });
    assert_eq!(runtime.tick_squid(&mut squid, Some(false)), Some(false));
    assert_eq!(squid.delta_movement().y, 0.1 * f64::from(0.98_f32));
}
