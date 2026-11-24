use super::*;

#[test]
fn sheep_growth_eating_installs_adult_shape_above_the_real_floor() {
    let mut native = NativeLevel::new();
    let pos = BlockPos { x: 8, y: 64, z: 8 };
    native.set(BlockPos { y: 63, ..pos }, state("minecraft:dirt"));
    native.set(pos, state("minecraft:short_grass"));
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            Vec3d {
                x: 8.5,
                y: 64.0,
                z: 8.5,
            },
        )
        .unwrap();
    let mut entity = store.detach(id).unwrap();
    assert!(crate::entity::general_living::set_spawner_age(
        &mut entity,
        -100
    ));
    entity.first_tick = false;
    entity.set_no_gravity(true);
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        data.sheep_retained_mut().unwrap().set_sheared(true);
    }
    native.with_level(|level| {
        assert!(apply_grazing(
            &mut entity,
            &mut store,
            level,
            GrazingAction::Eat
        ));
    });
    assert_eq!(entity.bb_height(), 1.3_f32);
    assert_eq!(
        entity.position(),
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 8.5
        }
    );
    assert!(entity.bounding_box().min_y >= 64.0);
    assert_eq!(native.world.block_state(DIMENSION, pos), Some(air()));
}
