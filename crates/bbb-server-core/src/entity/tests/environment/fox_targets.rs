use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_FOX_ID,
};

fn fixture(snow: bool) -> (NativeLevel, i32, i32, i32) {
    let mut native = NativeLevel::new();
    let ids = [
        VANILLA_ENTITY_TYPE_FOX_ID,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        VANILLA_ENTITY_TYPE_COD_ID,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, kind)| {
        let pos = Vec3d {
            x: 0.5 + index as f64 * 2.0,
            y: 64.0,
            z: 0.5,
        };
        let (id, _) = native.world.spawn_stored_entity_for_test(
            DIMENSION,
            kind,
            pos,
            EntityData::GeneralLiving(crate::entity::GeneralLivingData::new(kind).unwrap()),
        );
        let body = native
            .world
            .stored_entity_mut_for_test(DIMENSION, id)
            .unwrap();
        body.set_no_gravity(true);
        if kind != VANILLA_ENTITY_TYPE_FOX_ID {
            let EntityData::GeneralLiving(data) = &mut body.data else {
                panic!()
            };
            data.set_no_ai(true);
        }
        id
    })
    .collect::<Vec<_>>();
    let fox = ids[0];
    let body = native
        .world
        .stored_entity_mut_for_test(DIMENSION, fox)
        .unwrap();
    crate::entity::fox::set_spawn_variant(body, snow);
    assert!(crate::entity::configure_fox_target_goals(body));
    (native, fox, ids[1], ids[2])
}
fn seek(native: &mut NativeLevel, fox: i32, prey: i32) {
    let uuid = native
        .world
        .stored_entity_for_test(DIMENSION, prey)
        .unwrap()
        .uuid();
    for _ in 0..100 {
        native.world.tick(Vec::new());
        if native.world.mob_target_for_test(DIMENSION, fox) == Some(uuid) {
            return;
        }
    }
    panic!("fox selects the live stored prey");
}

#[test]
fn fox_saved_type_selects_prey_after_reload() {
    for snow in [false, true] {
        let (mut native, fox, chicken, cod) = fixture(snow);
        seek(&mut native, fox, if snow { cod } else { chicken });
        let NbtSummaryValue::Compound(mut fields) = native
            .world
            .stored_entity_command_data_for_test(DIMENSION, fox)
            .unwrap()
        else {
            panic!()
        };
        fields
            .iter_mut()
            .find(|field| field.name == "Type")
            .unwrap()
            .value = NbtSummaryValue::String(if snow { "red" } else { "snow" }.into());
        native
            .world
            .load_stored_entity_record_for_test(DIMENSION, fox, &fields);
        let fox_uuid = native
            .world
            .stored_entity_for_test(DIMENSION, fox)
            .unwrap()
            .uuid();
        let prey_uuid = native
            .world
            .stored_entity_for_test(DIMENSION, if snow { chicken } else { cod })
            .unwrap()
            .uuid();
        let root = std::env::temp_dir().join(format!("bbb-fox-reload-{}", Uuid::new_v4()));
        let storage = crate::tests::created_fixture_save(&root, "Fox reload");
        let candidate = storage.load_candidate().unwrap();
        let request = native
            .world
            .save_request(0, 0, bbb_server_persistence::RegionCompression::Zlib)
            .unwrap();
        storage.save_snapshot(&candidate, &request).unwrap();
        let reloaded = storage.load_candidate().unwrap();
        native.world = crate::world::ServerWorld::restore(
            crate::world::ServerWorldConfig {
                datapacks: Some(crate::registry_sync::builtin_datapack_startup_shared()),
                ..Default::default()
            },
            &reloaded,
        )
        .unwrap();
        drop(storage);
        std::fs::remove_dir_all(root).unwrap();
        let ids = native.world.stored_entity_ids_for_test(DIMENSION);
        let fox = *ids
            .iter()
            .find(|id| {
                native
                    .world
                    .stored_entity_for_test(DIMENSION, **id)
                    .unwrap()
                    .uuid()
                    == fox_uuid
            })
            .unwrap();
        let prey = *ids
            .iter()
            .find(|id| {
                native
                    .world
                    .stored_entity_for_test(DIMENSION, **id)
                    .unwrap()
                    .uuid()
                    == prey_uuid
            })
            .unwrap();
        seek(&mut native, fox, prey);
    }
}
