use super::*;
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::ArmorStandData;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID;
use bbb_protocol::packets::{Direction, Vec3d};

#[test]
fn a_piston_push_moves_an_armor_stand_with_the_piston_displacement_limit() {
    let mut level = NativeLevel::new();
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::ArmorStand(ArmorStandData::new().unwrap()),
    );
    let mut entity = store.detach(id).unwrap();
    level.with_level(|ctx| {
        crate::entity::movement::move_entity_by_piston(
            &mut entity,
            &mut store,
            ctx,
            Vec3d {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Direction::East,
        )
    });
    assert_eq!(
        entity.position(),
        Vec3d {
            x: 1.01,
            y: 64.0,
            z: 0.5
        }
    );
    store.attach(entity);
}
