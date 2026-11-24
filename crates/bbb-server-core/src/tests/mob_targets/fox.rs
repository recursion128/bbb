use super::*;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_FOX_ID,
    VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
};

fn load_type(world: &mut ServerWorld, fox: i32, snow: bool) {
    let Some(NbtSummaryValue::Compound(mut fields)) =
        world.stored_entity_command_data_for_test(DIMENSION, fox)
    else {
        panic!()
    };
    fields
        .iter_mut()
        .find(|field| field.name == "Type")
        .unwrap()
        .value = NbtSummaryValue::String(if snow { "snow" } else { "red" }.into());
    world.load_stored_entity_record_for_test(DIMENSION, fox, &fields);
}

#[test]
fn live_world_fox_loaded_prey_priority_survives_repeated_type_changes() {
    for snow in [false, true] {
        let mut world =
            spawn_pinned_restore(datapack_config(), &restore_candidate(0, None, Vec::new()));
        install_floor(&mut world);
        let (fox, _) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_FOX_ID,
            Vec3d {
                x: 4.5,
                y: 65.0,
                z: 4.5,
            },
        );
        spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
            Vec3d {
                x: 12.5,
                y: 65.0,
                z: 4.5,
            },
        );
        load_type(&mut world, fox, snow);
        for _ in 0..20 {
            world.tick(Vec::new());
            assert_eq!(target_of(&world, fox), None);
        }
        let (_, chicken) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            Vec3d {
                x: 8.5,
                y: 65.0,
                z: 4.5,
            },
        );
        let (_, cod) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_COD_ID,
            Vec3d {
                x: 4.5,
                y: 65.0,
                z: 8.5,
            },
        );
        let preferred = if snow { cod } else { chicken };
        tick_until(&mut world, 100, |world| {
            target_of(world, fox) == Some(preferred)
        });
        for _ in 0..3 {
            load_type(&mut world, fox, !snow);
            assert_eq!(target_of(&world, fox), Some(preferred));
            world.tick(Vec::new());
            assert_eq!(target_of(&world, fox), Some(preferred));
        }
    }
}
