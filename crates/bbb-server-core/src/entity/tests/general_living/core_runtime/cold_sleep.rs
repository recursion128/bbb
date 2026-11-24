use super::*;

#[test]
fn sleep_runtime_cold_first_tick_snaps_before_body_and_eye_fluid_queries() {
    for ty in [
        VANILLA_ENTITY_TYPE_COW_ID,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    ] {
        let mut store = EntityStore::new(17, EntityIdCounter::new());
        let id = if ty == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
            crate::entity::spawn_armor_stand(&mut store, position()).unwrap()
        } else {
            store.spawn_general_living(ty, position()).unwrap()
        };
        let bed = BlockPos { x: 4, y: 70, z: 8 };
        let entity = store.get_mut(id).unwrap();
        entity.set_sleeping_pos(Some(bed));
        entity.set_no_gravity(true);
        entity.set_air_supply(-7);
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            data.set_no_ai(true);
        }
        let chunk = write_entity_chunk(&store, (0, 0)).unwrap();
        let mut loaded = EntityStore::new(19, EntityIdCounter::new());
        let ids = read_entity_chunk(&mut loaded, &chunk).unwrap();
        let id = ids[0];
        assert_eq!(loaded.get(id).unwrap().position(), position());
        let mut level = {
            let mut native = NativeLevel::new();
            native.set(bed, state_id("minecraft:red_bed"));
            native.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:water"));
            native.set(BlockPos { x: 0, y: 65, z: 0 }, state_id("minecraft:water"));
            native
        };
        level.with_level(|level| tick_entities(&mut loaded, level, true));
        let entity = loaded.get(id).unwrap();
        assert_eq!(
            entity.position(),
            Vec3d {
                x: 4.5,
                y: 70.6875,
                z: 8.5
            }
        );
        assert!(!entity.is_eye_in_fluid(crate::entity::FluidTag::Water));
        assert_eq!(entity.air_supply(), -3);
    }
}
