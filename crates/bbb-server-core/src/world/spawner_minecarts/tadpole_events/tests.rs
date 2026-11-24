use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TADPOLE_ID;
use bbb_protocol::RichTextComponent;
use std::sync::OnceLock;

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn world() -> (ServerWorld, LevelId) {
    static CONFIG: OnceLock<crate::world::ServerWorldConfig> = OnceLock::new();
    let mut world =
        ServerWorld::new(CONFIG.get_or_init(crate::tests::datapack_config).clone()).unwrap();
    let dimension = world.require_level_id(&OVERWORLD.into());
    world.load_entity_chunk_for_test(OVERWORLD, (0, 0)).unwrap();
    world
        .levels
        .get_mut(&dimension)
        .unwrap()
        .chunks
        .load_column(0, 0);
    (world, dimension)
}

#[test]
fn a_tadpole_grows_into_a_persistent_frog() {
    let (mut world, dimension) = world();
    let entities = &mut world.levels.get_mut(&dimension).unwrap().entities;
    let tadpole = entities
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_TADPOLE_ID,
            Vec3d {
                x: 0.5,
                y: 65.0,
                z: 0.5,
            },
        )
        .unwrap();
    entities
        .get_mut(tadpole)
        .unwrap()
        .set_custom_name(Some(RichTextComponent::plain("Taddy")));

    assert_eq!(world.grow_tadpole(&dimension, tadpole), Some(()));
    let entities = &world.levels.get(&dimension).unwrap().entities;
    assert!(entities
        .get(tadpole)
        .is_none_or(|entity| entity.removal_reason().is_some()));
    let frog = entities
        .ids()
        .into_iter()
        .filter_map(|id| entities.get(id))
        .find(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_FROG_ID)
        .expect("the tadpole became a frog");
    assert_eq!(frog.custom_name(), Some(&RichTextComponent::plain("Taddy")));
    let NbtSummaryValue::Compound(record) = entities.command_data(frog.id()).unwrap() else {
        panic!("an entity record is a compound")
    };
    assert!(record.iter().any(
        |entry| entry.name == "PersistenceRequired" && entry.value == NbtSummaryValue::Byte(1)
    ));
    assert!(record.iter().any(|entry| entry.name == "variant"));
}
